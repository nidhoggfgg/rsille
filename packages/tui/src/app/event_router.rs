//! Event routing system
//!
//! Routes events to appropriate handlers (global, focus navigation, widget handlers)

use crate::{
    event::{Event, KeyCode, KeyModifiers},
    focus::FocusManager,
    layout::Layout,
};
use rustc_hash::FxHashMap;

use super::runtime::QuitBehavior;

/// Event router for handling event distribution
///
/// Routes events to:
/// - Built-in quit key handler
/// - Global key handlers
/// - Focus navigation (Tab/Shift+Tab)
/// - Widget event handlers
pub struct EventRouter<M> {
    /// Global key handlers registered by the application
    global_key_handlers: FxHashMap<KeyCode, Box<dyn Fn() -> M + Send + Sync>>,
    /// Quit key behavior configuration
    quit_behavior: QuitBehavior,
}

impl<M> EventRouter<M>
where
    M: Clone + std::fmt::Debug,
{
    /// Create a new event router
    pub fn new() -> Self {
        Self {
            global_key_handlers: FxHashMap::default(),
            quit_behavior: QuitBehavior::default(),
        }
    }

    /// Create a new event router with quit behavior configuration
    pub fn with_quit_behavior(quit_behavior: QuitBehavior) -> Self {
        Self {
            global_key_handlers: FxHashMap::default(),
            quit_behavior,
        }
    }

    /// Add a global key handler
    pub fn add_global_handler<F>(&mut self, key: KeyCode, handler: F)
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        self.global_key_handlers.insert(key, Box::new(handler));
    }

    /// Route a single event
    ///
    /// Event handling order (with bubbling support):
    /// 1. Route to widgets first - they can consume events to prevent bubbling
    /// 2. If widget consumed the event, stop here (no bubbling)
    /// 3. If widget ignored the event, handle Tab/Shift+Tab for focus navigation
    /// 4. Check built-in quit key handler
    /// 5. If still not handled, check global key handlers
    ///
    /// Returns (should_continue, messages, needs_redraw, focus_changed)
    pub fn route_event(
        &self,
        event: &Event,
        layout: &mut Box<dyn Layout<M>>,
        focus_manager: &mut FocusManager,
    ) -> RouteResult<M> {
        // STEP 1: Route event to widgets first - they have priority
        let focus_id = focus_manager.focus_id();
        let registry = &focus_manager.registry;
        let (result, mut messages) = layout.handle_event_with_focus(event, focus_id, registry);

        // If widget consumed the event, stop here (event was handled, no bubbling)
        if result.is_consumed() {
            let needs_redraw = !messages.is_empty();
            return RouteResult {
                messages,
                needs_redraw,
                focus_changed: false,
                should_quit: false,
            };
        }

        // STEP 2: Widget ignored the event, now handle Tab/Shift+Tab for focus navigation
        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Tab if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    // Focus previous widget
                    focus_manager.focus_prev();
                    return RouteResult {
                        messages: Vec::new(),
                        needs_redraw: true,
                        focus_changed: true,
                        should_quit: false,
                    };
                }
                KeyCode::Tab => {
                    // Focus next widget
                    focus_manager.focus_next();
                    return RouteResult {
                        messages: Vec::new(),
                        needs_redraw: true,
                        focus_changed: true,
                        should_quit: false,
                    };
                }
                _ => {}
            }

            // STEP 3: Check built-in quit key handler
            let should_quit = match &self.quit_behavior {
                QuitBehavior::Default => {
                    // Default is Esc without any modifiers
                    key_event.code == KeyCode::Esc && key_event.modifiers.is_empty()
                }
                QuitBehavior::CustomKey(quit_key) => {
                    // Custom key without modifiers
                    key_event.code == *quit_key && key_event.modifiers.is_empty()
                }
                QuitBehavior::CustomKeyEvent(quit_event) => {
                    // Custom key with modifiers - match both key and modifiers
                    key_event.code == quit_event.code && key_event.modifiers == quit_event.modifiers
                }
                QuitBehavior::Disabled => false,
            };

            if should_quit {
                return RouteResult {
                    messages: Vec::new(),
                    needs_redraw: false,
                    focus_changed: false,
                    should_quit: true,
                };
            }

            // STEP 4: Check global key handlers (app-level shortcuts)
            if let Some(handler) = self.global_key_handlers.get(&key_event.code) {
                let message = handler();
                messages.push(message);
                return RouteResult {
                    messages,
                    needs_redraw: true,
                    focus_changed: false,
                    should_quit: false,
                };
            }
        }

        // Event was not handled by anyone
        let needs_redraw = !messages.is_empty();

        RouteResult {
            messages,
            needs_redraw,
            focus_changed: false,
            should_quit: false,
        }
    }
}

impl<M> Default for EventRouter<M>
where
    M: Clone + std::fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Result of routing an event
#[derive(Debug)]
pub struct RouteResult<M> {
    /// Messages generated by event handling
    pub messages: Vec<M>,
    /// Whether a redraw is needed
    pub needs_redraw: bool,
    /// Whether focus changed
    pub focus_changed: bool,
    /// Whether the application should quit
    pub should_quit: bool,
}
