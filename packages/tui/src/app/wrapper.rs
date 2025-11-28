use render::{area::Size, chunk::Chunk, Draw, DrawErr, Update};

use crate::{
    event::{Event, KeyCode, KeyModifiers},
    focus::FocusManager,
    layout::{Layout, OverlayManager},
};

use super::runtime::App;

/// Wrapper to adapt App to DrawUpdate trait for use with event_loop
pub struct AppWrapper<State, F, V, M> {
    pub(super) app: App<State, M>,
    update_fn: F,
    view_fn: V,
    messages: Vec<M>,
    needs_redraw: bool,
    // Performance optimization: cache widget tree to avoid rebuilding every frame
    cached_layout: Option<Box<dyn Layout<M>>>,
    state_changed: bool,
    // Inline mode configuration
    pub(crate) inline_mode: bool,
    pub(crate) inline_max_height: u16,
    pub(crate) terminal_width: u16,
    // Focus management
    focus_manager: FocusManager,
}

impl<State, F, V, M> AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug,
{
    pub fn new(app: App<State, M>, update_fn: F, view_fn: V) -> Self {
        Self {
            app,
            update_fn,
            view_fn,
            messages: Vec::new(),
            needs_redraw: true,
            cached_layout: None,
            state_changed: true, // Start with state_changed=true to build initial tree
            inline_mode: false,
            inline_max_height: 50,
            terminal_width: 80,
            focus_manager: FocusManager::new(),
        }
    }

    pub fn with_inline_config(
        mut self,
        inline_mode: bool,
        inline_max_height: u16,
        terminal_width: u16,
    ) -> Self {
        self.inline_mode = inline_mode;
        self.inline_max_height = inline_max_height;
        self.terminal_width = terminal_width;
        self
    }
}

impl<State, F, V, M> Draw for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug,
{
    fn draw(&mut self, mut chunk: Chunk) -> std::result::Result<Size, DrawErr> {
        // Begin hover tracking frame
        crate::hover::HoverManager::global().begin_frame();

        // Clear any previous overlays
        OverlayManager::global().clear();

        // Rebuild widget tree only when state has changed
        // This preserves internal widget state (like Interactive's pressed state)
        // between events while still rebuilding when app state changes
        if self.state_changed || self.cached_layout.is_none() {
            let layout = (self.view_fn)(&self.app.state);
            self.cached_layout = Some(layout);
            self.state_changed = false;

            // Build focus chain from widget tree
            self.rebuild_focus_chain();

            // Update focus states in widget tree
            self.update_focus_states();
        }

        // Get the chunk area for size
        let size = chunk.area().size();

        // Get reference to cached container for rendering
        let layout = self
            .cached_layout
            .as_mut()
            .expect("Layout should be cached after rebuild");

        // Render the widget tree directly to chunk
        layout.render(&mut chunk);

        // Render all overlays on top
        let overlays = OverlayManager::global().take_overlays();
        for overlay in overlays {
            // Try to create a sub-chunk for the overlay area
            if let Ok(mut overlay_chunk) = chunk.from_area(overlay.area) {
                (overlay.renderer)(&mut overlay_chunk);
            }
        }

        // End hover tracking frame - calculate hover states
        crate::hover::HoverManager::global().end_frame();

        Ok(size)
    }
}

impl<State, F, V, M> Update for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug,
{
    fn on_events(
        &mut self,
        events: &[crossterm::event::Event],
    ) -> std::result::Result<(), DrawErr> {
        // Ensure we have a layout (should be cached from draw, but check anyway)
        if self.cached_layout.is_none() {
            let layout = (self.view_fn)(&self.app.state);
            self.cached_layout = Some(layout);
            self.rebuild_focus_chain();
        }

        // Begin hover event batch - clears pending events from last batch
        crate::hover::HoverManager::global().begin_event_batch();

        // Track if we need to route MouseMoved events
        let mut has_hover_changes = false;

        for event in events {
            // Handle Resize events - force rebuild of widget tree
            if let Event::Resize(_, _) = event {
                self.state_changed = true;
                self.needs_redraw = true;
                // Clear cache to force rebuild with new size
                self.cached_layout = None;
                continue;
            }

            // Update HoverManager on mouse movement and check for changes
            if let Event::Mouse(mouse_event) = event {
                use crate::event::MouseEventKind;
                if matches!(
                    mouse_event.kind,
                    MouseEventKind::Moved | MouseEventKind::Drag(_)
                ) {
                    has_hover_changes = crate::hover::HoverManager::global()
                        .update_mouse_position(mouse_event.column, mouse_event.row);

                    // Skip routing MouseMoved if no hover state changed (optimization)
                    if !has_hover_changes && matches!(mouse_event.kind, MouseEventKind::Moved) {
                        continue;
                    }
                }
            }

            // Intercept Tab/Shift+Tab for focus navigation
            if let Event::Key(key_event) = event {
                match key_event.code {
                    KeyCode::Tab if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                        // Focus previous widget
                        self.focus_manager.focus_prev();
                        self.update_focus_states();
                        self.needs_redraw = true;
                        continue; // Consume event
                    }
                    KeyCode::Tab => {
                        // Focus next widget
                        self.focus_manager.focus_next();
                        self.update_focus_states();
                        self.needs_redraw = true;
                        continue; // Consume event
                    }
                    _ => {}
                }

                // Check global key handlers before routing to widgets
                if let Some(handler) = self.app.global_key_handlers.get(&key_event.code) {
                    let message = handler();
                    self.messages.push(message);
                    self.needs_redraw = true;
                    continue; // Consume event
                }
            }

            // Route event to widgets with focus information
            let layout = self
                .cached_layout
                .as_mut()
                .expect("Layout should be cached");

            let focus_path = self.focus_manager.focus_path();
            let (_result, messages) = layout.handle_event_with_focus(event, &[], focus_path);

            // Store messages for processing in update()
            let has_messages = !messages.is_empty();
            self.messages.extend(messages);

            // If we got messages, mark that we need to redraw
            if has_messages {
                self.needs_redraw = true;
            }
        }

        Ok(())
    }

    fn update(&mut self) -> std::result::Result<bool, DrawErr> {
        // Process all collected messages
        let had_messages = !self.messages.is_empty();
        for msg in self.messages.drain(..) {
            (self.update_fn)(&mut self.app.state, msg);
        }

        // If we processed any messages, state has changed
        if had_messages {
            self.state_changed = true;
            // Rebuild layout now so required_size() will see updated tree
            // This is crucial for inline mode where height is calculated from layout constraints
            let layout = (self.view_fn)(&self.app.state);
            self.cached_layout = Some(layout);
            // Rebuild focus chain after state change
            self.rebuild_focus_chain();
            self.update_focus_states();
        }

        // Always return true to ensure continuous rendering for animations
        Ok(true)
    }

    fn required_size(&self, current_size: Size) -> Option<Size> {
        if !self.inline_mode {
            return None;
        }

        // Use cached layout to calculate required height
        // At this point, cached_layout should reflect current state
        // (built during previous draw or initial setup)
        if let Some(layout) = &self.cached_layout {
            let constraints = layout.constraints();
            let required_height = constraints.min_height;
            let height = required_height.min(self.inline_max_height);

            // Only return new size if height changed
            if height != current_size.height {
                return Some(Size {
                    width: current_size.width,
                    height,
                });
            }
        }

        None
    }
}

// Private helper methods for focus management
impl<State, F, V, M> AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug,
{
    /// Rebuild focus chain from current widget tree
    fn rebuild_focus_chain(&mut self) {
        if let Some(layout) = &self.cached_layout {
            let mut chain = Vec::new();
            let mut path = Vec::new();
            layout.build_focus_chain(&mut path, &mut chain);
            self.focus_manager.set_focus_chain(chain);
        }
    }

    /// Update focus states in widget tree based on current focus
    fn update_focus_states(&mut self) {
        if let Some(layout) = &mut self.cached_layout {
            let focus_path = self.focus_manager.focus_path();
            layout.update_focus_states(&[], focus_path);
        }
    }
}
