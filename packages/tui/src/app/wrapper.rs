use render::{area::Size, chunk::Chunk, Draw, DrawErr, Update};

use crate::{event::Event, layout::Container, widget::Widget};

use super::runtime::App;

/// Wrapper to adapt App to DrawUpdate trait for use with event_loop
pub struct AppWrapper<State, F, V, M> {
    pub(super) app: App<State>,
    update_fn: F,
    view_fn: V,
    messages: Vec<M>,
    needs_redraw: bool,
    // Performance optimization: cache widget tree to avoid rebuilding every frame
    cached_container: Option<Box<Container<M>>>,
    state_changed: bool,
    // Inline mode configuration
    pub(crate) inline_mode: bool,
    pub(crate) inline_max_height: u16,
    pub(crate) terminal_width: u16,
}

impl<State, F, V, M> AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    pub fn new(app: App<State>, update_fn: F, view_fn: V) -> Self {
        Self {
            app,
            update_fn,
            view_fn,
            messages: Vec::new(),
            needs_redraw: true,
            cached_container: None,
            state_changed: true, // Start with state_changed=true to build initial tree
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
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    fn draw(&mut self, mut chunk: Chunk) -> std::result::Result<Size, DrawErr> {
        // Always rebuild widget tree to support animations
        // Animations update based on time, so we need to call view() every frame
        let container = (self.view_fn)(&self.app.state);

        // Cache the container
        self.cached_container = Some(Box::new(container));

        // Reset state_changed flag after rebuilding
        self.state_changed = false;

        // Get reference to cached container for rendering
        let container = self
            .cached_container
            .as_mut()
            .expect("Container should be cached after rebuild");

        // Get the chunk area for size
        let size = chunk.area().size();

        // Render the widget tree directly to chunk
        container.render(&mut chunk);

        Ok(size)
    }
}

impl<State, F, V, M> Update for AppWrapper<State, F, V, M>
where
    F: Fn(&mut State, M),
    V: Fn(&State) -> Container<M>,
    M: Clone + std::fmt::Debug,
{
    fn on_events(
        &mut self,
        events: &[crossterm::event::Event],
    ) -> std::result::Result<(), DrawErr> {
        // Ensure we have a container (should be cached from draw, but check anyway)
        if self.cached_container.is_none() {
            let container = (self.view_fn)(&self.app.state);
            self.cached_container = Some(Box::new(container));
        }

        for event in events {
            // Handle Resize events - force rebuild of widget tree
            if let Event::Resize(_, _) = event {
                self.state_changed = true;
                self.needs_redraw = true;
                // Clear cache to force rebuild with new size
                self.cached_container = None;
                continue;
            }

            // Route event to widgets and collect messages using cached container
            let container = self
                .cached_container
                .as_mut()
                .expect("Container should be cached");
            let (_result, messages) = container.handle_event_with_messages(event);

            // Store messages for processing in update()
            self.messages.extend(messages);
            self.needs_redraw = true;
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
            // Rebuild container now so required_size() will see updated tree
            // This is crucial for inline mode where height is calculated from container constraints
            let container = (self.view_fn)(&self.app.state);
            self.cached_container = Some(Box::new(container));
        }

        // Always return true to ensure continuous rendering for animations
        Ok(true)
    }

    fn required_size(&self, current_size: Size) -> Option<Size> {
        if !self.inline_mode {
            return None;
        }

        // Use cached container to calculate required height
        // At this point, cached_container should reflect current state
        // (built during previous draw or initial setup)
        if let Some(container) = &self.cached_container {
            let constraints = container.constraints();
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
