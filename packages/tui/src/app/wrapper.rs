use render::{area::Size, chunk::Chunk, Draw, DrawErr, Update};

use crate::{
    event::Event,
    focus::FocusManager,
    layout::{Layout, OverlayManager},
};

use super::{
    event_router::EventRouter, hover_processor::HoverEventProcessor, layout_cache::LayoutCache,
    runtime::App,
};

/// Wrapper to adapt App to DrawUpdate trait for use with event_loop
pub struct AppWrapper<State, F, V, M> {
    pub(super) app: App<State, M>,
    update_fn: F,
    view_fn: V,
    messages: Vec<M>,
    needs_redraw: bool,

    // Separated subsystems
    event_router: EventRouter<M>,
    layout_cache: LayoutCache<M>,
    focus_manager: FocusManager,

    // Inline mode configuration
    pub(crate) inline_mode: bool,
    pub(crate) inline_max_height: u16,
    pub(crate) terminal_width: u16,
}

impl<State, F, V, M> AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug + 'static,
{
    pub fn new(app: App<State, M>, update_fn: F, view_fn: V) -> Self {
        // Initialize event router with global key handlers and quit behavior from app
        let mut event_router = EventRouter::with_quit_behavior(app.quit_behavior.clone());
        for (key, handler) in &app.global_key_handlers {
            event_router.add_global_handler(*key, {
                let handler = handler.clone();
                move || handler()
            });
        }

        Self {
            app,
            update_fn,
            view_fn,
            messages: Vec::new(),
            needs_redraw: true,
            event_router,
            layout_cache: LayoutCache::new(),
            focus_manager: FocusManager::new(),
            inline_mode: false,
            inline_max_height: 50,
            terminal_width: 80,
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
        HoverEventProcessor::begin_frame();

        // Clear any previous overlays
        OverlayManager::global().clear();

        // Check if we need to rebuild layout
        let needs_rebuild = self.layout_cache.needs_rebuild();

        // Get or rebuild layout (rebuilds if state changed or cache is empty)
        if needs_rebuild {
            let new_layout = (self.view_fn)(&self.app.state);

            // Build focus chain from widget tree
            let mut path = Vec::new();
            let mut chain = Vec::new();
            let mut registry = crate::focus::WidgetRegistry::new();
            new_layout.build_focus_chain_recursive(&mut path, &mut chain, &mut registry);
            self.focus_manager.set_focus_chain(chain, registry);

            // Set the new layout in cache
            self.layout_cache.set(new_layout);

            // Update focus states in the newly cached layout
            let focus_id = self.focus_manager.focus_id();
            let registry = &self.focus_manager.registry;
            if let Some(layout) = self.layout_cache.get_mut() {
                layout.update_focus_states(focus_id, registry);
            }
        }

        // Get the chunk area for size
        let size = chunk.area().size();

        // Get layout for rendering
        let layout = self
            .layout_cache
            .get_mut()
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
        HoverEventProcessor::end_frame();

        Ok(size)
    }
}

impl<State, F, V, M> Update for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Box<dyn Layout<M>>,
    M: Clone + std::fmt::Debug + 'static,
{
    fn on_events(
        &mut self,
        events: &[crossterm::event::Event],
    ) -> std::result::Result<(), DrawErr> {
        // Ensure we have a layout (should be cached from draw, but check anyway)
        if self.layout_cache.get().is_none() {
            let layout = (self.view_fn)(&self.app.state);

            // Build focus chain from new layout
            let mut path = Vec::new();
            let mut chain = Vec::new();
            let mut registry = crate::focus::WidgetRegistry::new();
            layout.build_focus_chain_recursive(&mut path, &mut chain, &mut registry);
            self.focus_manager.set_focus_chain(chain, registry);

            // Set the layout in cache
            self.layout_cache.set(layout);
        }

        // Begin hover event batch - clears pending events from last batch
        HoverEventProcessor::begin_event_batch();

        for event in events {
            // Handle Resize events - force rebuild of widget tree
            if let Event::Resize(width, height) = event {
                self.layout_cache.clear();
                self.needs_redraw = true;
                // Update hover manager's spatial grid
                HoverEventProcessor::set_terminal_size(*width, *height);
                continue;
            }

            // Update HoverManager on mouse movement and check for changes
            if !HoverEventProcessor::process_mouse_event(event) {
                // Mouse moved but no hover state changed, skip routing
                continue;
            }

            // Route event using EventRouter
            let layout = self
                .layout_cache
                .get_mut()
                .expect("Layout should be cached");

            let route_result =
                self.event_router
                    .route_event(event, layout, &mut self.focus_manager);

            // Check if application should quit
            if route_result.should_quit {
                std::process::exit(0);
            }

            // Store messages for processing in update()
            self.messages.extend(route_result.messages);

            // Update redraw flag
            if route_result.needs_redraw {
                self.needs_redraw = true;
            }

            // Update focus states if focus changed
            if route_result.focus_changed {
                self.update_focus_states();
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
            self.layout_cache.invalidate();

            // Rebuild layout now so required_size() will see updated tree
            // This is crucial for inline mode where height is calculated from layout constraints
            let layout = (self.view_fn)(&self.app.state);

            // Rebuild focus chain from new layout
            self.rebuild_focus_chain(&layout);

            // Set the new layout in cache
            self.layout_cache.set(layout);

            // Update focus states after rebuild
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
        if let Some(layout) = self.layout_cache.get() {
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
    M: Clone + std::fmt::Debug + 'static,
{
    /// Rebuild focus chain from given widget tree
    fn rebuild_focus_chain(&mut self, layout: &Box<dyn Layout<M>>) {
        let mut path = Vec::new();
        let mut chain = Vec::new();
        let mut registry = crate::focus::WidgetRegistry::new();
        layout.build_focus_chain_recursive(&mut path, &mut chain, &mut registry);
        self.focus_manager.set_focus_chain(chain, registry);
    }

    /// Update focus states in widget tree based on current focus
    fn update_focus_states(&mut self) {
        if let Some(layout) = self.layout_cache.get_mut() {
            let focus_id = self.focus_manager.focus_id();
            let registry = &self.focus_manager.registry;
            layout.update_focus_states(focus_id, registry);
        }
    }
}
