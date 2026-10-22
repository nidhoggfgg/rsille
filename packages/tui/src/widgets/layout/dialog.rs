//! Dialog container widget.

use render::area::Area;

use crate::event::Event;
use crate::focus::{FocusConfig, FocusScope, ScopeEntry};
use crate::layout::border_renderer;
use crate::layout::Constraints;
use crate::style::{BorderStyle, Padding, Style};
use crate::widget::{EventCtx, IntoWidget, RenderCtx, Widget};

use super::panel::{container_constraints, render_children_vertical};

/// Modal-style surface intended for use inside `overlay()`.
pub struct Dialog<M = ()> {
    children: Vec<Box<dyn Widget<M>>>,
    title: Option<String>,
    border: BorderStyle,
    padding: Padding,
    gap: u16,
    width: Option<u16>,
    height: Option<u16>,
    style: Style,
    focus_scope: FocusScope,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Dialog<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("children", &self.children.len())
            .field("title", &self.title)
            .field("border", &self.border)
            .field("padding", &self.padding)
            .field("gap", &self.gap)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("style", &self.style)
            .finish()
    }
}

impl<M> Dialog<M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            title: None,
            border: BorderStyle::Double,
            padding: Padding::uniform(1),
            gap: 1,
            width: None,
            height: None,
            style: Style::default(),
            focus_scope: FocusScope::new()
                .trap_tab(true)
                .entry(ScopeEntry::LastFocused),
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
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

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = scope;
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
}

impl<M> Default for Dialog<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Widget<M> for Dialog<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() < 2 || area.height() < 2 {
            return;
        }

        ctx.record_bounds(area);

        let surface_style = self.style.merge(ctx.theme().styles.surface_modal);
        let render_style = surface_style.to_render_style();
        let border_style = ctx
            .theme()
            .styles
            .border_focused
            .merge(surface_style)
            .to_render_style();
        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', render_style);
        border_renderer::render_border(chunk, self.border, border_style);

        if let Some(title) = self.title.as_ref() {
            let available = area.width().saturating_sub(4) as usize;
            let display = truncate_to_width(title, available, chunk.text_metrics());
            if !display.is_empty() {
                let _ = chunk.set_string(2, 0, &display, border_style);
            }
        }

        let inner = Area::new(
            (area.x() + 1, area.y() + 1).into(),
            (area.width() - 2, area.height() - 2).into(),
        )
        .shrink_saturating(
            self.padding.top,
            self.padding.bottom,
            self.padding.left,
            self.padding.right,
        );
        render_children_vertical(chunk, ctx, &self.children, inner, self.gap);
    }

    fn handle_event(&self, _event: &Event, _ctx: &mut EventCtx<M>) {}

    fn constraints(&self) -> Constraints {
        let mut constraints = container_constraints(&self.children, self.padding, true, self.gap);
        constraints.min_width = self.width.unwrap_or(constraints.min_width.max(32));
        constraints.min_height = self.height.unwrap_or(constraints.min_height.max(7));
        constraints.max_width = self.width;
        constraints.max_height = self.height;
        constraints.flex = None;
        constraints
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        &self.children
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Scope(self.focus_scope.clone())
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn truncate_to_width(text: &str, max_width: usize, metrics: render::text::TextMetrics) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn dialog<M>() -> Dialog<M> {
    Dialog::new()
}
