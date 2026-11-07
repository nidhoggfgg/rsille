//! Fixed-height, controlled collection virtualization with bounded row lifetimes.

use crate::render::{area::Area, chunk::Chunk};
use crate::tui::{
    event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    focus::FocusConfig,
    layout::{ChildLayout, Constraints, Layout},
    widget::{
        EventCtx, EventPhase, IntoWidget, Lifecycle, MeasureCtx, RenderCtx, StateCtx, Widget,
        WidgetExt,
    },
};
use std::sync::Arc;

/// A row can be addressed before its payload has been loaded. Retry controls may
/// be returned by the row builder for `Failed`, using normal widget messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowData<T> {
    /// A loaded row payload.
    Ready(T),
    /// A row whose payload has not arrived yet.
    Loading,
    /// A row that failed to load, with an error message for display or retry UI.
    Failed(String),
}

/// Random access to a collection. Keys must be unique and remain stable across
/// sorting, filtering and insertion. Implement `index_of` using an index for large
/// collections; the widget never enumerates the complete source.
pub trait DataSource {
    /// The loaded payload type for each row.
    type Item;
    /// Returns the number of addressable rows, including rows still loading.
    fn len(&self) -> usize;
    /// Returns whether the source has no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the stable, unique key for a row index.
    ///
    /// The widget calls this only for indices less than [`Self::len`].
    fn key(&self, index: usize) -> String;
    /// Resolves a stable key to its current index, or `None` if it no longer exists.
    fn index_of(&self, key: &str) -> Option<usize>;
    /// Returns the payload or loading/failure status for a valid row index.
    ///
    /// The widget requests only rows within the viewport and overscan margin.
    fn get(&self, index: usize) -> RowData<Self::Item>;
}

/// Application-owned selection and first-row anchor. If a key disappears,
/// selection clears and the anchor falls back to its clamped previous index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtualListState {
    /// Stable key of the selected row, if any.
    pub selected: Option<String>,
    /// Stable key of the first visible row, used to preserve position after reordering.
    pub anchor: Option<String>,
    /// Zero-based first visible row index; fallback when the anchor disappears.
    pub offset: usize,
}
impl VirtualListState {
    /// Reconciles selection and the first-row anchor with an updated source.
    ///
    /// `rows` is the viewport capacity in items, treated as at least one. A missing
    /// selection is cleared; a missing anchor falls back to the clamped offset.
    pub fn reconcile<S: DataSource + ?Sized>(&mut self, source: &S, rows: usize) {
        if self
            .selected
            .as_deref()
            .is_some_and(|key| source.index_of(key).is_none())
        {
            self.selected = None;
        }
        self.offset = self
            .anchor
            .as_deref()
            .and_then(|key| source.index_of(key))
            .unwrap_or_else(|| self.offset.min(source.len().saturating_sub(rows.max(1))));
        self.anchor = (self.offset < source.len()).then(|| source.key(self.offset));
    }
    /// Selects an index and scrolls just enough to keep it visible.
    ///
    /// `rows` is the viewport capacity in items, treated as at least one. An index
    /// outside the source leaves the state unchanged.
    pub fn select<S: DataSource + ?Sized>(&mut self, source: &S, index: usize, rows: usize) {
        if index >= source.len() {
            return;
        }
        self.selected = Some(source.key(index));
        self.offset = crate::tui::layout::ensure_item_visible(self.offset, index, rows.max(1))
            .min(source.len().saturating_sub(rows.max(1)));
        self.anchor = Some(source.key(self.offset));
    }
}

/// A fixed terminal-cell viewport and row height. Overscan rows are retained,
/// receiving lifecycle synchronization but no focus, events or animation updates.
#[derive(Debug, Clone, Copy)]
pub struct VirtualListConfig {
    /// Viewport height in terminal rows; defaults to 10 and is clamped to at least 1.
    pub height: u16,
    /// Fixed item height in terminal rows; defaults to 1 and is clamped to at least 1.
    pub row_height: u16,
    /// Number of retained rows on each side of the viewport; defaults to 2.
    pub overscan: usize,
}
impl Default for VirtualListConfig {
    fn default() -> Self {
        Self {
            height: 10,
            row_height: 1,
            overscan: 2,
        }
    }
}

#[derive(Default)]
struct Interaction {
    value: VirtualListState,
    controlled: Option<VirtualListState>,
}

/// A controlled list that builds only visible rows and an overscan margin.
///
/// Keep [`VirtualListState`] in application state and update it from
/// [`Self::on_change`] messages before rebuilding the view. Source keys identify
/// rows across sorting and insertion; rows outside the retained window are unmounted.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use rsille::tui::widgets::{
///     label, DataSource, RowData, VirtualList, VirtualListConfig, VirtualListState,
/// };
///
/// struct Rows(Vec<String>);
/// impl DataSource for Rows {
///     type Item = String;
///     fn len(&self) -> usize { self.0.len() }
///     // The values in this example are unique and also serve as stable keys.
///     fn key(&self, index: usize) -> String { self.0[index].clone() }
///     fn index_of(&self, key: &str) -> Option<usize> {
///         self.0.iter().position(|value| value == key)
///     }
///     fn get(&self, index: usize) -> RowData<String> {
///         RowData::Ready(self.0[index].clone())
///     }
/// }
///
/// let source = Arc::new(Rows(vec!["Alpha".into(), "Beta".into(), "Gamma".into()]));
/// let state = VirtualListState::default();
/// let config = VirtualListConfig { height: 2, row_height: 1, overscan: 1 };
/// let view = VirtualList::new(source, &state, config, |_, data, selected| {
///     let text = match data {
///         RowData::Ready(text) => text,
///         RowData::Loading => "Loading...".into(),
///         RowData::Failed(error) => error,
///     };
///     let row = label::<VirtualListState>(text);
///     if selected { row.bold() } else { row }
/// }).on_change(|next| next);
/// assert_eq!(view.window(), 0..2);
/// // Store emitted VirtualListState messages before constructing the next view.
/// ```
pub struct VirtualList<M, S: DataSource + ?Sized> {
    source: Arc<S>,
    state: VirtualListState,
    config: VirtualListConfig,
    children: Vec<Box<dyn Widget<M>>>,
    retained: Vec<Box<dyn Widget<M>>>,
    on_change: Option<Box<dyn Fn(VirtualListState) -> M>>,
    key: Option<String>,
}
impl<M: 'static, S: DataSource + ?Sized + 'static> VirtualList<M, S> {
    /// Builds only the viewport and overscan. Store emitted state in your update
    /// function so the next description constructs the newly visible window.
    ///
    /// The row builder receives the source index, payload status, and whether the
    /// row is selected. It also runs for overscan rows. Heights are normalized to
    /// at least one terminal row before the visible range is computed.
    pub fn new<W: IntoWidget<M> + 'static>(
        source: Arc<S>,
        state: &VirtualListState,
        mut config: VirtualListConfig,
        mut row: impl FnMut(usize, RowData<S::Item>, bool) -> W,
    ) -> Self {
        config.row_height = config.row_height.max(1);
        config.height = config.height.max(1);
        let rows = usize::from(config.height).div_ceil(usize::from(config.row_height));
        let mut state = state.clone();
        state.reconcile(source.as_ref(), rows);
        let end = state.offset.saturating_add(rows).min(source.len());
        let mut children = Vec::new();
        let mut retained = Vec::new();
        for i in state.offset.saturating_sub(config.overscan)
            ..end.saturating_add(config.overscan).min(source.len())
        {
            let key = source.key(i);
            let selected = state.selected.as_deref() == Some(&key);
            let widget = row(i, source.get(i), selected).into_widget().scope(key);
            if i >= state.offset && i < end {
                children.push(Box::new(widget) as Box<dyn Widget<M>>);
            } else {
                retained.push(Box::new(widget) as Box<dyn Widget<M>>);
            }
        }
        Self {
            source,
            state,
            config,
            children,
            retained,
            on_change: None,
            key: None,
        }
    }
    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
    /// Sets the callback receiving proposed selection and scroll state.
    ///
    /// Store this state in the application before rebuilding the list. Setting a
    /// callback also enables focus and interaction.
    pub fn on_change(mut self, handler: impl Fn(VirtualListState) -> M + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
    /// Returns the half-open range of source indices built for visible rows.
    ///
    /// The range excludes retained overscan rows.
    pub fn window(&self) -> std::ops::Range<usize> {
        self.state.offset..self.state.offset + self.children.len()
    }
}
impl<M: 'static, S: DataSource + ?Sized + 'static> Widget<M> for VirtualList<M, S> {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() == Lifecycle::Unmount {
            return;
        }
        let state = ctx.state_mut::<Interaction>();
        if state.controlled.as_ref() != Some(&self.state) {
            state.value = self.state.clone();
            state.controlled = Some(self.state.clone());
        }
    }
    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 1,
            max_width: None,
            min_height: self.config.height,
            max_height: Some(self.config.height),
            flex: None,
        }
    }
    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }
    fn retained_children(&self) -> &[Box<dyn Widget<M>>] {
        &self.retained
    }
    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    fn focus_config(&self) -> FocusConfig {
        if self.on_change.is_some() {
            FocusConfig::Composite
        } else {
            FocusConfig::None
        }
    }
    fn layout(&self, area: Area, _: &MeasureCtx) -> Layout {
        Layout::new(self.children.iter().enumerate().filter_map(|(i, _)| {
            let y = i * usize::from(self.config.row_height);
            (y < usize::from(area.height())).then(|| {
                ChildLayout::new(
                    i,
                    Area::new(
                        (area.x(), area.y().saturating_add(y as u16)).into(),
                        (
                            area.width(),
                            self.config.row_height.min(area.height() - y as u16),
                        )
                            .into(),
                    ),
                )
            })
        }))
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if self.on_change.is_none() || ctx.phase() == EventPhase::Capture || ctx.was_handled() {
            return;
        }
        if matches!(event, Event::Key(key) if key.kind == KeyEventKind::Release) {
            return;
        }
        let rows = ctx
            .bounds()
            .map(|a| usize::from(a.height()).div_ceil(usize::from(self.config.row_height)))
            .unwrap_or(1)
            .max(1);
        let clicked = ctx
            .local_mouse_position(event)
            .map(|(_, y)| usize::from(y) / usize::from(self.config.row_height));
        let source = self.source.as_ref();
        let value = &mut ctx.state_mut::<Interaction>().value;
        let current = value.selected.as_deref().and_then(|k| source.index_of(k));
        let index = match event {
            Event::Key(key) => match key.code {
                KeyCode::Up => Some(current.unwrap_or(value.offset).saturating_sub(1)),
                KeyCode::Down => Some(current.map_or(value.offset, |i| i.saturating_add(1))),
                KeyCode::Home => Some(0),
                KeyCode::End => Some(source.len().saturating_sub(1)),
                KeyCode::PageUp => Some(current.unwrap_or(value.offset).saturating_sub(rows)),
                KeyCode::PageDown => Some(current.unwrap_or(value.offset).saturating_add(rows)),
                _ => return,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    clicked.map(|row| self.state.offset.saturating_add(row))
                }
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    value.offset = if mouse.kind == MouseEventKind::ScrollDown {
                        value.offset.saturating_add(3)
                    } else {
                        value.offset.saturating_sub(3)
                    }
                    .min(source.len().saturating_sub(rows));
                    value.anchor = (!source.is_empty()).then(|| source.key(value.offset));
                    None
                }
                _ => return,
            },
            _ => return,
        };
        if let Some(index) = index.filter(|_| !source.is_empty()) {
            value.select(source, index.min(source.len() - 1), rows);
        }
        let next = value.clone();
        if let Some(handler) = &self.on_change {
            ctx.emit(handler(next));
        }
        ctx.set_handled();
    }
}
