//! Radio group widget.

use render::text::TextWidth;

use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::focus::FocusConfig;
use crate::layout::Constraints;
use crate::style::Style;
use crate::widget::{EventCtx, EventPhase, RenderCtx, Widget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl RadioOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<&str> for RadioOption {
    fn from(value: &str) -> Self {
        Self::new(value, value)
    }
}

impl From<String> for RadioOption {
    fn from(value: String) -> Self {
        Self::new(value.clone(), value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RadioGroupState {
    pub active_value: Option<String>,
}

pub struct RadioGroup<M = ()> {
    options: Vec<RadioOption>,
    selected_value: Option<String>,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(String) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for RadioGroup<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroup")
            .field("options", &self.options)
            .field("selected_value", &self.selected_value)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.is_some())
            .finish()
    }
}

impl<M> RadioGroup<M> {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
            selected_value: None,
            disabled: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn option(mut self, option: impl Into<RadioOption>) -> Self {
        self.options.push(option.into());
        self
    }

    pub fn options<I>(mut self, options: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<RadioOption>,
    {
        self.options.extend(options.into_iter().map(Into::into));
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected_value = Some(value.into());
        self
    }

    pub fn selected_opt(mut self, value: Option<String>) -> Self {
        self.selected_value = value;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> M + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    fn has_enabled_options(&self) -> bool {
        self.options.iter().any(|option| !option.disabled)
    }

    fn first_enabled_index(&self) -> Option<usize> {
        self.options.iter().position(|option| !option.disabled)
    }

    fn last_enabled_index(&self) -> Option<usize> {
        self.options.iter().rposition(|option| !option.disabled)
    }

    fn index_for_value(&self, value: &str) -> Option<usize> {
        self.options
            .iter()
            .position(|option| option.value == value && !option.disabled)
    }

    fn active_index_from_state(&self, state: &RadioGroupState) -> Option<usize> {
        state
            .active_value
            .as_deref()
            .and_then(|value| self.index_for_value(value))
            .or_else(|| {
                self.selected_value
                    .as_deref()
                    .and_then(|value| self.index_for_value(value))
            })
            .or_else(|| self.first_enabled_index())
    }

    fn next_enabled_index(&self, current: usize) -> Option<usize> {
        self.options
            .iter()
            .enumerate()
            .skip(current.saturating_add(1))
            .find(|(_, option)| !option.disabled)
            .map(|(index, _)| index)
    }

    fn prev_enabled_index(&self, current: usize) -> Option<usize> {
        self.options
            .iter()
            .enumerate()
            .take(current)
            .rev()
            .find(|(_, option)| !option.disabled)
            .map(|(index, _)| index)
    }
}

impl<M> Default for RadioGroup<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Widget<M> for RadioGroup<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let theme = ctx.theme();
        let row_style = self
            .custom_style
            .map(|style| style.merge(theme.styles.interactive))
            .unwrap_or(theme.styles.interactive)
            .to_render_style();
        let active_style = if ctx.is_focused() {
            self.custom_focus_style
                .unwrap_or(theme.styles.interactive_focused)
        } else {
            theme.styles.interactive
        }
        .to_render_style();
        let disabled_style = theme.styles.interactive_disabled.to_render_style();
        let state = ctx.state_or_default::<RadioGroupState>();
        let active_index = self.active_index_from_state(state);

        for (row, option) in self.options.iter().enumerate().take(area.height() as usize) {
            let is_selected = self.selected_value.as_deref() == Some(option.value.as_str());
            let is_active = active_index == Some(row);
            let style = if self.disabled || option.disabled {
                disabled_style
            } else if is_active {
                active_style
            } else {
                row_style
            };
            let mark = if is_selected { "*" } else { " " };
            let prefix = if is_active { ">" } else { " " };
            let text = format!("{prefix} ({mark}) {}", option.label);
            let display = truncate_to_width(&text, area.width() as usize, chunk.text_metrics());
            let _ = chunk.set_string(0, row as u16, &display, style);
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if ctx.phase() != EventPhase::Target || self.disabled || !self.has_enabled_options() {
            return;
        }

        let clicked_index = match event {
            Event::Mouse(mouse_event)
                if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                ctx.local_mouse_position(event).map(|(_, row)| row as usize)
            }
            _ => None,
        };
        let mut next_value = None;
        {
            let state = ctx.state_mut::<RadioGroupState>();
            let Some(mut active_index) = self.active_index_from_state(state) else {
                return;
            };

            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Up | KeyCode::Left => {
                        if let Some(index) = self.prev_enabled_index(active_index) {
                            active_index = index;
                        }
                    }
                    KeyCode::Down | KeyCode::Right => {
                        if let Some(index) = self.next_enabled_index(active_index) {
                            active_index = index;
                        }
                    }
                    KeyCode::Home => {
                        if let Some(index) = self.first_enabled_index() {
                            active_index = index;
                        }
                    }
                    KeyCode::End => {
                        if let Some(index) = self.last_enabled_index() {
                            active_index = index;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {}
                    _ => return,
                },
                Event::Mouse(mouse_event)
                    if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) =>
                {
                    let Some(index) = clicked_index else {
                        return;
                    };
                    if index >= self.options.len() || self.options[index].disabled {
                        return;
                    }
                    active_index = index;
                }
                _ => return,
            }

            let value = self.options[active_index].value.clone();
            state.active_value = Some(value.clone());
            if self.selected_value.as_deref() != Some(value.as_str()) {
                next_value = Some(value);
            }
        }

        ctx.set_handled();
        if let (Some(value), Some(handler)) = (next_value, self.on_change.as_ref()) {
            ctx.emit(handler(value));
        }
    }

    fn constraints(&self) -> Constraints {
        let widest = self
            .options
            .iter()
            .map(|option| option.label.width() as u16)
            .max()
            .unwrap_or(8);
        Constraints {
            min_width: widest + 7,
            max_width: None,
            min_height: self.options.len().max(1) as u16,
            max_height: Some(self.options.len().max(1) as u16),
            flex: None,
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled || !self.has_enabled_options() {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn truncate_to_width(text: &str, max_width: usize, metrics: render::text::TextMetrics) -> String {
    metrics.clip(text, 0, max_width)
}

pub fn radio_group<M>() -> RadioGroup<M> {
    RadioGroup::new()
}
