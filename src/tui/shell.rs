use std::fmt;
use std::time::{Duration, Instant};

use crate::tui::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub id: String,
    pub key_event: KeyEvent,
    pub label: String,
    pub description: Option<String>,
}

impl Hotkey {
    pub fn new(id: impl Into<String>, key_event: KeyEvent, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key_event,
            label: label.into(),
            description: None,
        }
    }

    pub fn simple(id: impl Into<String>, key_code: KeyCode, label: impl Into<String>) -> Self {
        Self::new(id, KeyEvent::new(key_code, KeyModifiers::empty()), label)
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

struct HotkeyBinding<M> {
    hotkey: Hotkey,
    enabled: bool,
    handler: Box<dyn Fn() -> M>,
}

pub struct HotkeyRegistry<M> {
    bindings: Vec<HotkeyBinding<M>>,
}

impl<M> Default for HotkeyRegistry<M> {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

impl<M> HotkeyRegistry<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind<F>(&mut self, hotkey: Hotkey, handler: F) -> &mut Self
    where
        F: Fn() -> M + 'static,
    {
        self.bindings.push(HotkeyBinding {
            hotkey,
            enabled: true,
            handler: Box::new(handler),
        });
        self
    }

    pub fn bind_key<F>(&mut self, key_code: KeyCode, handler: F) -> &mut Self
    where
        F: Fn() -> M + 'static,
    {
        let id = format!("key::{key_code:?}");
        let label = hotkey_label(&KeyEvent::new(key_code, KeyModifiers::empty()));
        self.bind(Hotkey::simple(id, key_code, label), handler)
    }

    pub fn resolve(&self, key_event: &KeyEvent) -> Option<M> {
        self.bindings
            .iter()
            .find(|binding| binding.enabled && hotkey_matches(&binding.hotkey.key_event, key_event))
            .map(|binding| (binding.handler)())
    }

    pub fn bindings(&self) -> Vec<Hotkey> {
        self.bindings
            .iter()
            .map(|binding| binding.hotkey.clone())
            .collect()
    }
}

impl<M> fmt::Debug for HotkeyRegistry<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HotkeyRegistry")
            .field("bindings", &self.bindings())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hotkeys: Vec<KeyEvent>,
}

pub struct CommandEntry<M> {
    descriptor: CommandDescriptor,
    enabled: bool,
    handler: Box<dyn Fn() -> M>,
}

impl<M> CommandEntry<M> {
    pub fn description(&mut self, description: impl Into<String>) -> &mut Self {
        self.descriptor.description = Some(description.into());
        self
    }

    pub fn hotkey(&mut self, key_event: KeyEvent) -> &mut Self {
        self.descriptor.hotkeys.push(key_event);
        self
    }

    pub fn enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    pub fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }
}

pub struct CommandRouter<M> {
    commands: Vec<CommandEntry<M>>,
}

impl<M> Default for CommandRouter<M> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<M> CommandRouter<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        handler: F,
    ) -> &mut CommandEntry<M>
    where
        F: Fn() -> M + 'static,
    {
        self.commands.push(CommandEntry {
            descriptor: CommandDescriptor {
                id: id.into(),
                title: title.into(),
                description: None,
                hotkeys: Vec::new(),
            },
            enabled: true,
            handler: Box::new(handler),
        });
        self.commands.last_mut().expect("command was just inserted")
    }

    pub fn dispatch(&self, id: &str) -> Option<M> {
        self.commands
            .iter()
            .find(|command| command.enabled && command.descriptor.id == id)
            .map(|command| (command.handler)())
    }

    pub fn dispatch_hotkey(&self, key_event: &KeyEvent) -> Option<M> {
        self.commands
            .iter()
            .find(|command| {
                command.enabled
                    && command
                        .descriptor
                        .hotkeys
                        .iter()
                        .any(|candidate| hotkey_matches(candidate, key_event))
            })
            .map(|command| (command.handler)())
    }

    pub fn commands(&self) -> Vec<CommandDescriptor> {
        self.commands
            .iter()
            .map(|command| command.descriptor.clone())
            .collect()
    }
}

impl<M> fmt::Debug for CommandRouter<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandRouter")
            .field("commands", &self.commands())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ModalManager<T> {
    stack: Vec<T>,
}

impl<T> Default for ModalManager<T> {
    fn default() -> Self {
        Self { stack: Vec::new() }
    }
}

impl<T> ModalManager<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, modal: T) {
        self.stack.push(modal);
    }

    pub fn replace_top(&mut self, modal: T) -> Option<T> {
        if let Some(top) = self.stack.last_mut() {
            Some(std::mem::replace(top, modal))
        } else {
            self.stack.push(modal);
            None
        }
    }

    pub fn close_top(&mut self) -> Option<T> {
        self.stack.pop()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn active(&self) -> Option<&T> {
        self.stack.last()
    }

    pub fn active_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.stack.iter()
    }
}

pub type NotificationId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification<T> {
    pub id: NotificationId,
    pub level: NotificationLevel,
    pub payload: T,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct NotificationCenter<T> {
    next_id: NotificationId,
    items: Vec<Notification<T>>,
}

impl<T> Default for NotificationCenter<T> {
    fn default() -> Self {
        Self {
            next_id: 0,
            items: Vec::new(),
        }
    }
}

impl<T> NotificationCenter<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, level: NotificationLevel, payload: T) -> NotificationId {
        self.push_for(level, payload, None)
    }

    pub fn push_timed(
        &mut self,
        level: NotificationLevel,
        payload: T,
        ttl: Duration,
    ) -> NotificationId {
        self.push_for(level, payload, Some(ttl))
    }

    fn push_for(
        &mut self,
        level: NotificationLevel,
        payload: T,
        ttl: Option<Duration>,
    ) -> NotificationId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let created_at = Instant::now();
        self.items.push(Notification {
            id,
            level,
            payload,
            created_at,
            expires_at: ttl.map(|ttl| created_at + ttl),
        });
        id
    }

    pub fn dismiss(&mut self, id: NotificationId) -> Option<Notification<T>> {
        let index = self.items.iter().position(|item| item.id == id)?;
        Some(self.items.remove(index))
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn prune_expired(&mut self, now: Instant) {
        self.items.retain(|item| {
            item.expires_at
                .map(|deadline| deadline > now)
                .unwrap_or(true)
        });
    }

    pub fn items(&self) -> &[Notification<T>] {
        &self.items
    }

    pub fn latest(&self) -> Option<&Notification<T>> {
        self.items.last()
    }
}

#[derive(Debug, Clone)]
pub struct Navigator<Route> {
    stack: Vec<Route>,
}

impl<Route> Navigator<Route> {
    pub fn new(initial: Route) -> Self {
        Self {
            stack: vec![initial],
        }
    }

    pub fn current(&self) -> &Route {
        self.stack
            .last()
            .expect("Navigator always keeps at least one route")
    }

    pub fn current_mut(&mut self) -> &mut Route {
        self.stack
            .last_mut()
            .expect("Navigator always keeps at least one route")
    }

    pub fn push(&mut self, route: Route) {
        self.stack.push(route);
    }

    pub fn replace(&mut self, route: Route) {
        if let Some(current) = self.stack.last_mut() {
            *current = route;
        } else {
            self.stack.push(route);
        }
    }

    pub fn reset(&mut self, route: Route) {
        self.stack.clear();
        self.stack.push(route);
    }

    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn go_back(&mut self) -> Option<Route> {
        if self.can_go_back() {
            self.stack.pop()
        } else {
            None
        }
    }

    pub fn stack(&self) -> &[Route] {
        &self.stack
    }
}

pub(crate) fn hotkey_matches(expected: &KeyEvent, actual: &KeyEvent) -> bool {
    actual.kind != KeyEventKind::Release
        && expected.code == actual.code
        && expected.modifiers == actual.modifiers
}

pub(crate) fn hotkey_label(key_event: &KeyEvent) -> String {
    let mut parts = Vec::new();
    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_owned());
    }
    if key_event.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_owned());
    }
    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_owned());
    }
    parts.push(format!("{:?}", key_event.code));
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::{
        hotkey_matches, CommandRouter, ModalManager, Navigator, NotificationCenter,
        NotificationLevel,
    };
    use crate::tui::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::{Duration, Instant};

    #[test]
    fn command_router_dispatches_by_id_and_hotkey() {
        let mut router = CommandRouter::new();
        router
            .register("save", "Save", || 1)
            .description("Persist current document")
            .hotkey(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

        assert_eq!(router.dispatch("save"), Some(1));
        assert_eq!(
            router.dispatch_hotkey(&KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(1)
        );
    }

    #[test]
    fn modal_manager_behaves_like_stack() {
        let mut manager = ModalManager::new();
        manager.open("first");
        manager.open("second");

        assert_eq!(manager.active(), Some(&"second"));
        assert_eq!(manager.close_top(), Some("second"));
        assert_eq!(manager.active(), Some(&"first"));
    }

    #[test]
    fn notifications_can_expire() {
        let mut center = NotificationCenter::new();
        center.push_timed(NotificationLevel::Info, "hello", Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(2));
        center.prune_expired(Instant::now());
        assert!(center.items().is_empty());
    }

    #[test]
    fn navigator_tracks_history() {
        let mut navigator = Navigator::new("home");
        navigator.push("settings");

        assert_eq!(navigator.current(), &"settings");
        assert!(navigator.can_go_back());
        assert_eq!(navigator.go_back(), Some("settings"));
        assert_eq!(navigator.current(), &"home");
    }

    #[test]
    fn hotkeys_ignore_kind_and_state_bits() {
        let expected = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let actual = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(hotkey_matches(&expected, &actual));
    }
}
