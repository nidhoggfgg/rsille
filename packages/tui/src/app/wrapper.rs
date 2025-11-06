use render::{area::Size, chunk::Chunk, Draw, DrawErr, Update};

use crate::{
    event::{Event, KeyCode},
    layout::Container,
    widget::Widget,
};

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
        }
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
        let mut container = (self.view_fn)(&self.app.state);

        // Initialize focus management
        self.app.apply_focus(&mut container);

        // Cache the container
        self.cached_container = Some(Box::new(container));

        // Reset state_changed flag after rebuilding
        self.state_changed = false;

        // Get reference to cached container for rendering
        let container = self
            .cached_container
            .as_mut()
            .expect("Container should be cached after rebuild");

        // Get the chunk area
        let area = chunk.area();
        let size = area.size();

        // Create an Area from the chunk size for rendering (starting at 0,0 since chunk handles positioning)
        use render::area::Area;
        let render_area = Area::new((0, 0).into(), size);

        // Render the widget tree directly to chunk
        container.render(&mut chunk, render_area);

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
            let mut container = (self.view_fn)(&self.app.state);
            self.app.apply_focus(&mut container);
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

            // Handle Tab navigation
            if let Event::Key(ref key_ev) = event {
                if key_ev.code == KeyCode::Tab {
                    // Tab navigation modifies focus, need to rebuild on next draw
                    let container = self
                        .cached_container
                        .as_mut()
                        .expect("Container should be cached");
                    let shift = key_ev.modifiers.contains(crate::event::KeyModifiers::SHIFT);
                    self.app.handle_tab(container, shift);
                    self.needs_redraw = true;
                    // Note: We don't set state_changed because focus is not part of state
                    // But we'll need to reapply focus on next rebuild
                    continue;
                }
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
        }

        // Always return true to ensure continuous rendering for animations
        Ok(true)
    }
}
