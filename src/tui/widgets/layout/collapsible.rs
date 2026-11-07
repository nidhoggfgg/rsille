//! Collapsible container widget.

use crate::render::area::Area;
use crate::tui::layout::Layout;

use crate::render::text::TextWidth;

use crate::tui::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::Constraints;
use crate::tui::style::{Padding, Style};
use crate::tui::widget::{EventCtx, EventPhase, IntoWidget, MeasureCtx, RenderCtx, Widget};

use super::panel::{container_constraints, layout_children_vertical};

/// A titled section whose expanded state is controlled by the application.
///
/// Feed [`Self::on_toggle`] values back through [`Self::expanded`]. Collapsed
/// children are unmounted unless [`Self::retain_state`] is enabled.
pub struct Collapsible<M = ()> {
    title: String,
    expanded: bool,
    retain_state: bool,
    disabled: bool,
    children: Vec<Box<dyn Widget<M>>>,
    padding: Padding,
    gap: u16,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_toggle: Option<Box<dyn Fn(bool) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Collapsible<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Collapsible")
            .field("title", &self.title)
            .field("expanded", &self.expanded)
            .field("disabled", &self.disabled)
            .field("children", &self.children.len())
            .field("padding", &self.padding)
            .field("gap", &self.gap)
            .field("on_toggle", &self.on_toggle.is_some())
            .finish()
    }
}

impl<M> Collapsible<M> {
    /// Creates an empty, collapsed section with no padding or gap.
    ///
    /// Hidden child state is not retained by default.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            expanded: false,
            retain_state: false,
            disabled: false,
            children: Vec::new(),
            padding: Padding::ZERO,
            gap: 0,
            custom_style: None,
            custom_focus_style: None,
            on_toggle: None,
            widget_key: None,
        }
    }

    /// Sets the stable widget key used to identify this widget across view rebuilds.
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    /// Sets the application-controlled expansion state.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Keep described children mounted while collapsed. They are excluded from
    /// layout, focus, input and animation updates. Default: unmount on collapse.
    /// Retained resources are bounded by the children in this description and
    /// released when this container or the children are removed.
    pub fn retain_state(mut self, retain: bool) -> Self {
        self.retain_state = retain;
        self
    }

    /// Sets whether this widget ignores input and is excluded from keyboard focus.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets interior spacing in terminal cells.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the number of terminal rows between consecutive children.
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Sets the custom rendering style.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Sets the style used to highlight the active item or control while focused.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Appends a child after the existing children.
    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    /// Appends children in iteration order, preserving existing children.
    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(IntoWidget::into_widget));
        self
    }

    /// Sets the callback receiving the proposed expansion state.
    ///
    /// Feed the value back through [`Self::expanded`] on the next view rebuild.
    pub fn on_toggle<F>(mut self, handler: F) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl<M> Widget<M> for Collapsible<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        ctx.record_bounds(area);

        let theme = ctx.theme();
        let header_style = if self.disabled {
            theme.styles.interactive_disabled
        } else if ctx.is_focused() {
            self.custom_focus_style
                .unwrap_or(theme.styles.interactive_focused)
        } else {
            self.custom_style.unwrap_or(theme.styles.interactive)
        }
        .to_render_style();
        let marker = if self.expanded { "v" } else { ">" };
        let header = truncate_to_width(
            &format!("{marker} {}", self.title),
            area.width() as usize,
            chunk.text_metrics(),
        );
        let _ = chunk.set_string(0, 0, &header, header_style);

        if !self.expanded || area.height() <= 1 {
            return;
        }

        ctx.render_children(chunk, self);
    }

    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        if !self.expanded || area.height() <= 1 {
            return Layout::default();
        }
        let content = area
            .shrink_saturating(1, 0, self.padding.left, self.padding.right)
            .shrink_saturating(self.padding.top, self.padding.bottom, 0, 0);
        layout_children_vertical(&self.children, content, content, self.gap, ctx)
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if matches!(event, Event::Key(key) if key.kind != crate::tui::event::KeyEventKind::Press) {
            return;
        }
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let activate = match event {
            Event::Key(key) => matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')),
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                matches!(ctx.local_mouse_position(event), Some((_, 0)))
            }
            _ => false,
        };
        if activate {
            ctx.set_handled();
            if let Some(handler) = self.on_toggle.as_ref() {
                ctx.emit(handler(!self.expanded));
            }
        }
    }

    fn constraints(&self) -> Constraints {
        let width = self.title.width() as u16 + 2;
        if !self.expanded {
            return Constraints {
                min_width: width,
                max_width: None,
                min_height: 1,
                max_height: Some(1),
                flex: None,
            };
        }

        let content = container_constraints(&self.children, self.padding, false, self.gap);
        Constraints {
            min_width: width.max(content.min_width),
            max_width: None,
            min_height: 1 + content.min_height,
            max_height: None,
            flex: Some(1.0),
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled {
            FocusConfig::None
        } else {
            FocusConfig::Leaf
        }
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        if self.expanded {
            &self.children
        } else {
            &[]
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }

    fn retained_children(&self) -> &[Box<dyn Widget<M>>] {
        if !self.expanded && self.retain_state {
            &self.children
        } else {
            &[]
        }
    }
}

fn truncate_to_width(
    text: &str,
    max_width: usize,
    metrics: crate::render::text::TextMetrics,
) -> String {
    metrics.clip(text, 0, max_width)
}

/// Creates a collapsed [`Collapsible`] section with the supplied title.
pub fn collapsible<M>(title: impl Into<String>) -> Collapsible<M> {
    Collapsible::new(title)
}
