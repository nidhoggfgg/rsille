//! Dialog widget - overlay dialog component for confirmations, alerts, and custom content

use super::*;
use crate::event::{Event, EventResult, KeyCode};
use crate::focus::WidgetRegistry;
use crate::layout::{col, row, Constraints, JustifyContent, Layout};
use crate::style::{BorderStyle, Padding, Style, ThemeManager};
use crate::widget::{button, label, ButtonVariant, IntoWidget};
use crate::widget_id::WidgetId;
use render::area::{Area, Position, Size};
use std::sync::{Arc, Mutex};

/// Dialog size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSize {
    /// Small dialog (30% width, min 30 cols)
    Small,
    /// Medium dialog (50% width, min 50 cols)
    Medium,
    /// Large dialog (70% width, min 70 cols)
    Large,
    /// Custom size with explicit dimensions
    Custom { width: u16, height: u16 },
}

impl Default for DialogSize {
    fn default() -> Self {
        Self::Medium
    }
}

/// Standard dialog messages for pre-configured types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogMessage {
    /// User clicked the primary action (OK, Confirm)
    Confirmed,
    /// User clicked cancel or pressed Escape
    Cancelled,
    /// Dialog was closed by any means
    Closed,
}

/// Dialog widget for popups and confirmations
///
/// Provides a centered overlay dialog with keyboard navigation,
/// and automatic focus trapping. Dialog renders on top of background content.
///
/// # Features
/// - Centered positioning with customizable size
/// - Renders over existing content (not replacing it)
/// - Focus trap (Tab/Shift+Tab stay within dialog)
/// - **ESC key always closes the dialog** (consumed to prevent app exit)
/// - Pre-configured types (Alert, Confirm) or custom content
/// - Theme integration
///
/// # ESC Key Behavior
/// - ESC always closes the dialog and triggers `on_close` callback
/// - ESC is consumed and **never** reaches background widgets
/// - This prevents accidental app termination when dialog is open
///
/// # Examples
///
/// ## Alert Dialog
/// ```ignore
/// use tui::widget::{alert, DialogMessage};
///
/// let main_view = col().child(label("Main content"));
/// let alert_dialog = alert(main_view, "Operation Successful", "The file has been saved.");
/// ```
///
/// ## Confirm Dialog
/// ```ignore
/// use tui::widget::{confirm, DialogMessage};
///
/// let main_view = col().child(label("Main content"));
/// let confirm_dialog = confirm(
///     main_view,
///     "Delete File?",
///     "This action cannot be undone. Are you sure?"
/// );
/// ```
///
/// ## Using Dialog with background content
/// ```ignore
/// use tui::prelude::*;
///
/// fn view(state: &State) -> Box<dyn Layout<Message>> {
///     let main_content = col()
///         .gap(1)
///         .child(label("Main Page Content"));
///
///     if state.show_dialog {
///         Box::new(alert(main_content, "Title", "Message"))
///     } else {
///         Box::new(main_content)
///     }
/// }
/// ```
pub struct Dialog<M = ()> {
    /// Background content that dialog overlays on top of
    background: Box<dyn Widget<M>>,
    /// Dialog content
    content: Box<dyn Widget<M>>,
    size: DialogSize,
    focused: bool,
    on_close: Option<Arc<dyn Fn() -> M + Send + Sync>>,
    /// Cached dialog area for focus trapping
    cached_dialog_area: Mutex<Option<Area>>,
}

impl<M> std::fmt::Debug for Dialog<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("size", &self.size)
            .field("focused", &self.focused)
            .finish()
    }
}

impl<M> Dialog<M> {
    /// Create a new dialog that overlays on top of background content
    ///
    /// For most cases, prefer using `alert()` or `confirm()` convenience functions
    /// which handle the background automatically.
    ///
    /// # Examples
    /// ```ignore
    /// use tui::prelude::*;
    ///
    /// let main_view = col()
    ///     .gap(1)
    ///     .child(label("Main content"));
    ///
    /// let dialog_content = col()
    ///     .gap(2)
    ///     .padding(Padding::new(2, 2, 2, 2))
    ///     .border(BorderStyle::Rounded)
    ///     .children(vec![
    ///         label("Dialog Title").into_widget(),
    ///         label("Dialog content").into_widget(),
    ///     ]);
    ///
    /// let with_dialog = Dialog::new(main_view, dialog_content);
    /// ```
    pub fn new(background: impl Widget<M> + 'static, content: impl Widget<M> + 'static) -> Self {
        Self {
            background: Box::new(background),
            content: Box::new(content),
            size: DialogSize::default(),
            focused: false,
            on_close: None,
            cached_dialog_area: Mutex::new(None),
        }
    }

    /// Set the dialog size
    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Set callback for when dialog is closed
    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.on_close = Some(Arc::new(handler));
        self
    }

    /// Get the dialog dimensions based on size preset
    fn get_dimensions(&self, available_width: u16, available_height: u16) -> (u16, u16) {
        match self.size {
            DialogSize::Small => {
                let width = (available_width * 3 / 10).max(30).min(available_width);
                let height = (available_height * 3 / 10).max(10).min(available_height);
                (width, height)
            }
            DialogSize::Medium => {
                let width = (available_width / 2).max(50).min(available_width);
                let height = (available_height / 2).max(15).min(available_height);
                (width, height)
            }
            DialogSize::Large => {
                let width = (available_width * 7 / 10).max(70).min(available_width);
                let height = (available_height * 7 / 10).max(20).min(available_height);
                (width, height)
            }
            DialogSize::Custom { width, height } => {
                (width.min(available_width), height.min(available_height))
            }
        }
    }
}

impl<M: Clone + Send + Sync + 'static> Widget<M> for Dialog<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        let area_size = area.size();
        let area_pos = area.pos();

        // First, render the background content
        self.background.render(chunk);

        // Calculate modal dimensions and center position
        let (modal_width, modal_height) = self.get_dimensions(area_size.width, area_size.height);
        let modal_x = (area_size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area_size.height.saturating_sub(modal_height)) / 2;

        // Create modal area and cache it for event handling
        let modal_area = Area::new(
            Position {
                x: area_pos.x + modal_x,
                y: area_pos.y + modal_y,
            },
            Size {
                width: modal_width,
                height: modal_height,
            },
        );
        *self.cached_dialog_area.lock().unwrap() = Some(modal_area);

        // Render modal content in a sub-region
        // The modal content will naturally overlay on top of the background
        let modal_chunk = chunk.shrink(
            modal_y,
            area_size.height.saturating_sub(modal_y + modal_height),
            modal_x,
            area_size.width.saturating_sub(modal_x + modal_width),
        );

        if let Ok(mut modal_chunk) = modal_chunk {
            self.content.render(&mut modal_chunk);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        // Always handle ESC key to close dialog - this prevents ESC from bubbling
        // up to the app level and causing program exit
        if let Event::Key(key_event) = event {
            if key_event.code == KeyCode::Esc {
                // Dialog always closes on ESC, trigger close callback
                let mut messages = vec![];
                if let Some(ref on_close) = self.on_close {
                    messages.push(on_close());
                }
                return EventResult::Consumed(messages);
            }
        }

        // Route other events to dialog content
        match self.content.handle_event(event) {
            EventResult::Consumed(messages) => EventResult::Consumed(messages),
            EventResult::Ignored => {
                // IMPORTANT: Consume all unhandled events to implement focus trap
                // This prevents any event from reaching background widgets
                EventResult::Consumed(vec![])
            }
        }
    }

    fn constraints(&self) -> Constraints {
        // Dialog always fills available space (acts as full-screen overlay)
        Constraints::fill()
    }

    fn focusable(&self) -> bool {
        true
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        // Propagate focus to content
        self.content.set_focused(focused);
    }

    fn build_focus_chain_recursive(
        &self,
        current_path: &mut Vec<usize>,
        chain: &mut Vec<crate::widget_id::WidgetId>,
        registry: &mut crate::focus::WidgetRegistry,
    ) {
        // Build focus chain for dialog content only (not background)
        current_path.push(0);
        self.content
            .build_focus_chain_recursive(current_path, chain, registry);
        current_path.pop();
    }

    fn update_focus_states_recursive(
        &mut self,
        current_path: &[usize],
        focus_id: Option<crate::widget_id::WidgetId>,
    ) {
        // Update focus states for dialog content only
        self.content
            .update_focus_states_recursive(current_path, focus_id);
    }
}

// Implement Layout trait for Dialog to make it usable as a root-level view
impl<M: Clone + Send + Sync + 'static> Layout<M> for Dialog<M> {
    fn update_focus_states(&mut self, focus_id: Option<WidgetId>, _registry: &WidgetRegistry) {
        // Update focus states recursively through content
        self.content.update_focus_states_recursive(&[], focus_id);
    }

    fn handle_event_with_focus(
        &mut self,
        event: &Event,
        _focus_id: Option<WidgetId>,
        _registry: &WidgetRegistry,
    ) -> (EventResult<M>, Vec<M>) {
        // Handle events through normal Widget trait
        let result = self.handle_event(event);
        let messages = match &result {
            EventResult::Consumed(msgs) => msgs.clone(),
            EventResult::Ignored => vec![],
        };
        // Return the actual result so EventRouter knows if event was consumed
        (result, messages)
    }
}

// =============================================================================
// Convenience constructors
// =============================================================================

/// Create an alert dialog with a message and single "OK" button
///
/// The alert dialog overlays on top of the provided background content.
///
/// # Examples
/// ```ignore
/// use tui::widget::alert;
///
/// let main_view = col().child(label("Main content"));
/// let with_alert = alert(main_view, "Success", "Your changes have been saved.");
/// ```
pub fn alert<M>(
    background: impl Widget<M> + 'static,
    title: impl Into<String>,
    message: impl Into<String>,
) -> Dialog<M>
where
    M: From<DialogMessage> + Clone + Send + Sync + 'static,
{
    let title_text = title.into();
    let message_text = message.into();

    let dialog_content = col()
        .gap(2)
        .padding(Padding {
            top: 2,
            bottom: 2,
            left: 2,
            right: 2,
        })
        .border(BorderStyle::Rounded)
        .style(ThemeManager::global().with_theme(|theme| {
            Style::default()
                .bg(theme.colors.surface)
                .fg(theme.colors.text)
        }))
        .children(vec![
            label(title_text)
                .style(
                    ThemeManager::global()
                        .with_theme(|theme| Style::default().fg(theme.colors.text).bold()),
                )
                .into_widget(),
            label(message_text).into_widget(),
            row()
                .gap(2)
                .justify_content(JustifyContent::Center)
                .children(vec![button("OK")
                    .variant(ButtonVariant::Primary)
                    .on_click(|| M::from(DialogMessage::Confirmed))
                    .into_widget()])
                .into_widget(),
        ]);

    Dialog::new(background, dialog_content)
        .size(DialogSize::Small)
        .on_close(|| M::from(DialogMessage::Closed))
}

/// Create a confirm dialog with "OK" and "Cancel" buttons
///
/// The confirm dialog overlays on top of the provided background content.
///
/// # Examples
/// ```ignore
/// use tui::widget::confirm;
///
/// let main_view = col().child(label("Main content"));
/// let with_confirm = confirm(
///     main_view,
///     "Delete File?",
///     "This action cannot be undone."
/// );
/// ```
pub fn confirm<M>(
    background: impl Widget<M> + 'static,
    title: impl Into<String>,
    message: impl Into<String>,
) -> Dialog<M>
where
    M: From<DialogMessage> + Clone + Send + Sync + 'static,
{
    let title_text = title.into();
    let message_text = message.into();

    let dialog_content = col()
        .gap(2)
        .padding(Padding {
            top: 2,
            bottom: 2,
            left: 2,
            right: 2,
        })
        .border(BorderStyle::Rounded)
        .style(ThemeManager::global().with_theme(|theme| {
            Style::default()
                .bg(theme.colors.surface)
                .fg(theme.colors.text)
        }))
        .children(vec![
            label(title_text)
                .style(
                    ThemeManager::global()
                        .with_theme(|theme| Style::default().fg(theme.colors.text).bold()),
                )
                .into_widget(),
            label(message_text).into_widget(),
            row()
                .gap(2)
                .justify_content(JustifyContent::Center)
                .children(vec![
                    button("OK")
                        .variant(ButtonVariant::Primary)
                        .on_click(|| M::from(DialogMessage::Confirmed))
                        .into_widget(),
                    button("Cancel")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| M::from(DialogMessage::Cancelled))
                        .into_widget(),
                ])
                .into_widget(),
        ]);

    Dialog::new(background, dialog_content)
        .size(DialogSize::Medium)
        .on_close(|| M::from(DialogMessage::Closed))
}

/// Create a custom dialog with flexible content
///
/// This is an alias for `Dialog::new()` for consistency with other constructors.
///
/// # Examples
/// ```ignore
/// use tui::prelude::*;
///
/// let main_view = col().child(label("Main content"));
/// let dialog_content = col()
///     .gap(2)
///     .padding(Padding::new(2, 2, 2, 2))
///     .border(BorderStyle::Rounded)
///     .children(vec![
///         label("Custom Dialog").into_widget(),
///         label("Your content here").into_widget(),
///     ]);
///
/// let custom = dialog(main_view, dialog_content);
/// ```
pub fn dialog<M>(
    background: impl Widget<M> + 'static,
    content: impl Widget<M> + 'static,
) -> Dialog<M> {
    Dialog::new(background, content)
}
