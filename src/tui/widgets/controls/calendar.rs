//! Calendar widget for date navigation and selection.

use crate::tui::event::{Event, KeyCode};
use crate::tui::focus::FocusConfig;
use crate::tui::layout::border_renderer;
use crate::tui::layout::Constraints;
use crate::tui::style::{BorderStyle, Style};
use crate::tui::widget::{EventCtx, EventPhase, RenderCtx, Widget};

/// A simple date value used by the calendar widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl CalendarDate {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }

        let max_day = days_in_month(year, month);
        if !(1..=max_day).contains(&day) {
            return None;
        }

        Some(Self { year, month, day })
    }
}

/// Persistent calendar state stored in the widget store.
#[derive(Debug, Clone, Default)]
pub struct CalendarState {
    pub visible_year: Option<i32>,
    pub visible_month: Option<u32>,
    pub selected_date: Option<CalendarDate>,
}

/// Focusable calendar widget with date navigation.
pub struct Calendar<M = ()> {
    initial_date: CalendarDate,
    border: Option<BorderStyle>,
    disabled: bool,
    custom_style: Option<Style>,
    custom_focus_style: Option<Style>,
    on_change: Option<Box<dyn Fn(CalendarDate) -> M>>,
    on_submit: Option<Box<dyn Fn(CalendarDate) -> M>>,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Calendar<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Calendar")
            .field("initial_date", &self.initial_date)
            .field("border", &self.border)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.is_some())
            .field("on_submit", &self.on_submit.is_some())
            .finish()
    }
}

impl<M> Calendar<M> {
    pub fn new(initial_date: CalendarDate) -> Self {
        Self {
            initial_date,
            border: Some(BorderStyle::Single),
            disabled: false,
            custom_style: None,
            custom_focus_style: None,
            on_change: None,
            on_submit: None,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = Some(border);
        self
    }

    pub fn borderless(mut self) -> Self {
        self.border = None;
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
        F: Fn(CalendarDate) -> M + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_submit<F>(mut self, handler: F) -> Self
    where
        F: Fn(CalendarDate) -> M + 'static,
    {
        self.on_submit = Some(Box::new(handler));
        self
    }

    fn current_selection(&self, state: Option<&CalendarState>) -> CalendarDate {
        state
            .and_then(|state| state.selected_date)
            .unwrap_or(self.initial_date)
    }

    fn current_month(&self, state: Option<&CalendarState>) -> (i32, u32) {
        let selected = self.current_selection(state);
        (
            state
                .and_then(|state| state.visible_year)
                .unwrap_or(selected.year),
            state
                .and_then(|state| state.visible_month)
                .unwrap_or(selected.month),
        )
    }
}

impl<M: 'static> Widget<M> for Calendar<M> {
    fn render(&self, chunk: &mut crate::render::chunk::Chunk, ctx: &RenderCtx) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let is_focused = ctx.is_focused();
        let theme = ctx.theme();
        let base_style = if self.disabled {
            theme.styles.interactive_disabled
        } else {
            theme.styles.surface_elevated
        };
        let base_render_style = self
            .custom_style
            .as_ref()
            .map(|style| style.merge(base_style))
            .unwrap_or(base_style)
            .to_render_style();
        let selected_style = if is_focused {
            self.custom_focus_style
                .unwrap_or(theme.styles.selected_focused)
        } else {
            theme.styles.selected
        }
        .to_render_style();
        let header_style = theme.styles.surface_header.to_render_style();
        let muted_style = theme.styles.text_muted.to_render_style();
        let border_style = if is_focused {
            theme.styles.border_focused.to_render_style()
        } else {
            theme.styles.border.to_render_style()
        };

        let _ = chunk.fill(0, 0, area.width(), area.height(), ' ', base_render_style);

        let (content_x, content_y, content_width, content_height) =
            if let Some(border) = self.border {
                if area.width() < 2 || area.height() < 2 {
                    return;
                }
                border_renderer::render_border(chunk, border, border_style);
                (1u16, 1u16, area.width() - 2, area.height() - 2)
            } else {
                (0u16, 0u16, area.width(), area.height())
            };

        if content_width < 20 || content_height < 8 {
            return;
        }

        let state = ctx.state::<CalendarState>();
        let selected = self.current_selection(state);
        let (year, month) = self.current_month(state);

        let header = format!("{} {}", month_name(month), year);
        let header_x = content_x + content_width.saturating_sub(header.len() as u16) / 2;
        let _ = chunk.set_string(header_x, content_y, &header, header_style);

        let weekdays = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        for (index, day) in weekdays.iter().enumerate() {
            let x = content_x + index as u16 * 3;
            let _ = chunk.set_string(x, content_y + 1, day, muted_style);
        }

        let first_weekday = weekday(year, month, 1) as i32;
        let mut current = add_days(
            CalendarDate {
                year,
                month,
                day: 1,
            },
            -first_weekday,
        );

        for row in 0..6u16 {
            for col in 0..7u16 {
                let x = content_x + col * 3;
                let y = content_y + 2 + row;
                let in_month = current.month == month && current.year == year;
                let is_selected = current == selected;
                let style = if is_selected {
                    selected_style
                } else if in_month {
                    base_render_style
                } else {
                    muted_style
                };
                let cell = format!("{:>2}", current.day);
                let _ = chunk.set_string(x, y, &cell, style);
                current = add_days(current, 1);
            }
        }
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<M>) {
        if crate::tui::event::is_key_release(event) {
            return;
        }
        if ctx.phase() != EventPhase::Target || self.disabled {
            return;
        }

        let Event::Key(key_event) = event else {
            return;
        };

        let mut emit_change = None;
        let mut emit_submit = None;
        let mut handled = true;

        {
            let state = ctx.state_mut::<CalendarState>();
            let mut selected = state.selected_date.unwrap_or(self.initial_date);

            match key_event.code {
                KeyCode::Left => selected = add_days(selected, -1),
                KeyCode::Right => selected = add_days(selected, 1),
                KeyCode::Up => selected = add_days(selected, -7),
                KeyCode::Down => selected = add_days(selected, 7),
                KeyCode::Home => {
                    selected.day = 1;
                }
                KeyCode::End => {
                    selected.day = days_in_month(selected.year, selected.month);
                }
                KeyCode::PageUp => selected = shift_month(selected, -1),
                KeyCode::PageDown => selected = shift_month(selected, 1),
                KeyCode::Enter => {
                    if let Some(ref handler) = self.on_submit {
                        emit_submit = Some(handler(selected));
                    }
                }
                _ => {
                    handled = false;
                }
            }

            if handled {
                state.selected_date = Some(selected);
                state.visible_year = Some(selected.year);
                state.visible_month = Some(selected.month);
                if emit_submit.is_none() {
                    if let Some(ref handler) = self.on_change {
                        emit_change = Some(handler(selected));
                    }
                }
            }
        }

        if handled {
            ctx.set_handled();
        }
        if let Some(message) = emit_change {
            ctx.emit(message);
        }
        if let Some(message) = emit_submit {
            ctx.emit(message);
        }
    }

    fn constraints(&self) -> Constraints {
        let border_size = if self.border.is_some() { 2 } else { 0 };

        Constraints {
            min_width: 22 + border_size,
            max_width: Some(22 + border_size),
            min_height: 10,
            max_height: Some(10),
            flex: None,
        }
    }

    fn focus_config(&self) -> FocusConfig {
        if self.disabled {
            FocusConfig::None
        } else {
            FocusConfig::Composite
        }
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

impl<M> Default for Calendar<M> {
    fn default() -> Self {
        Self::new(CalendarDate {
            year: 2026,
            month: 1,
            day: 1,
        })
    }
}

/// Create a new calendar widget.
pub fn calendar<M>(initial_date: CalendarDate) -> Calendar<M> {
    Calendar::new(initial_date)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn shift_month(date: CalendarDate, delta: i32) -> CalendarDate {
    let total_months = date.year * 12 + (date.month as i32 - 1) + delta;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) as u32 + 1;
    let day = date.day.min(days_in_month(year, month));
    CalendarDate { year, month, day }
}

fn add_days(mut date: CalendarDate, delta: i32) -> CalendarDate {
    if delta > 0 {
        for _ in 0..delta {
            if date.day < days_in_month(date.year, date.month) {
                date.day += 1;
            } else {
                date.day = 1;
                if date.month == 12 {
                    date.month = 1;
                    date.year += 1;
                } else {
                    date.month += 1;
                }
            }
        }
    } else if delta < 0 {
        for _ in 0..(-delta) {
            if date.day > 1 {
                date.day -= 1;
            } else if date.month == 1 {
                date.year -= 1;
                date.month = 12;
                date.day = days_in_month(date.year, date.month);
            } else {
                date.month -= 1;
                date.day = days_in_month(date.year, date.month);
            }
        }
    }

    date
}

fn weekday(year: i32, month: u32, day: u32) -> u32 {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = year;
    if month < 3 {
        year -= 1;
    }

    ((year + year / 4 - year / 100 + year / 400 + offsets[month as usize - 1] + day as i32) % 7)
        as u32
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Month",
    }
}
