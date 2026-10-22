//! Collapsible container widget.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::event::{Event, KeyCode};
use crate::focus::FocusConfig;
use crate::layout::Constraints;
use crate::style::{Padding, Style};
use crate::widget::{EventCtx, EventPhase, IntoWidget, RenderCtx, Widget};

use super::panel::{container_constraints, render_children_vertical};

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

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    pub fn child(mut self, widget: impl IntoWidget<M>) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    pub fn children<I>(mut self, widgets: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoWidget<M>,
    {
        self.children
            .extend(widgets.into_iter().map(IntoWidget::into_widget));
        self
    }

    pub fn on_toggle<F>(mut self, handler: F) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl<M> Widget<M> for Collapsible<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
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
        let header = truncate_to_width(&format!("{marker} {}", self.title), area.width() as usize);
        let _ = chunk.set_string(0, 0, &header, header_style);

        if !self.expanded || area.height() <= 1 {
            return;
        }

        let content = area
            .shrink_saturating(1, 0, self.padding.left, self.padding.right)
            .shrink_saturating(self.padding.top, self.padding.bottom, 0, 0);
        render_children_vertical(chunk, ctx, &self.children, content, self.gap);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let Event::Key(key_event) = event else {
            return;
        };

        if matches!(key_event.code, KeyCode::Enter | KeyCode::Char(' ')) {
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

fn truncate_to_width(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out
}

pub fn collapsible<M>(title: impl Into<String>) -> Collapsible<M> {
    Collapsible::new(title)
}
