use std::fmt;
use std::time::{Duration, Instant};

use crate::tui::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::tui::widget::WidgetId;

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
    pub sequences: Vec<Vec<KeyEvent>>,
    pub enabled: bool,
    pub scope: Option<WidgetId>,
}

pub struct CommandEntry<M> {
    descriptor: CommandDescriptor,
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
        self.descriptor.enabled = enabled;
        self
    }

    /// Restrict the command to focus within this identity subtree. Deeper scopes win.
    pub fn scope(&mut self, scope: WidgetId) -> &mut Self {
        self.descriptor.scope = Some(scope);
        self
    }

    pub fn sequence(&mut self, keys: impl IntoIterator<Item = KeyEvent>) -> &mut Self {
        let keys: Vec<_> = keys.into_iter().collect();
        assert!(!keys.is_empty(), "a command sequence cannot be empty");
        self.descriptor.sequences.push(keys);
        self
    }

    pub fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }
}

/// Result of a shortcut/sequence lookup. A prefix consumes input until it
/// completes, mismatches, changes focus, or expires before the next key.
pub enum CommandMatch<M> {
    Ignored,
    Pending,
    Message(M),
}

pub struct CommandRouter<M> {
    commands: Vec<CommandEntry<M>>,
    pending: Vec<KeyEvent>,
    pending_focus: Option<WidgetId>,
    deadline: Option<Instant>,
    sequence_timeout: Duration,
}

impl<M> Default for CommandRouter<M> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            pending: Vec::new(),
            pending_focus: None,
            deadline: None,
            sequence_timeout: Duration::from_secs(1),
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
        let id = id.into();
        assert!(
            !self.commands.iter().any(|entry| entry.descriptor.id == id),
            "duplicate command id: {id}"
        );
        self.commands.push(CommandEntry {
            descriptor: CommandDescriptor {
                id,
                title: title.into(),
                description: None,
                hotkeys: Vec::new(),
                sequences: Vec::new(),
                enabled: true,
                scope: None,
            },
            handler: Box::new(handler),
        });
        self.commands.last_mut().expect("command was just inserted")
    }

    pub fn dispatch(&self, id: &str) -> Option<M> {
        self.commands
            .iter()
            .find(|command| command.descriptor.enabled && command.descriptor.id == id)
            .map(|command| (command.handler)())
    }

    /// Resolve a global single-key shortcut, including user rebindings.
    /// Use `handle_key` for focus scopes and multi-key sequences.
    pub fn dispatch_hotkey(&self, key_event: &KeyEvent) -> Option<M> {
        if key_event.kind == KeyEventKind::Release {
            return None;
        }
        self.commands
            .iter()
            .find(|entry| {
                let d = &entry.descriptor;
                d.enabled
                    && d.scope.is_none()
                    && d.hotkeys
                        .iter()
                        .map(std::slice::from_ref)
                        .chain(d.sequences.iter().map(Vec::as_slice))
                        .any(|sequence| {
                            sequence.len() == 1 && hotkey_matches(&sequence[0], key_event)
                        })
            })
            .map(|entry| (entry.handler)())
    }

    pub fn cancel_pending(&mut self) {
        self.pending.clear();
        self.deadline = None;
        self.pending_focus = None;
    }

    pub(crate) fn sync_focus(&mut self, focused: Option<&WidgetId>) {
        if self.pending_focus.as_ref() != focused {
            self.cancel_pending();
        }
    }

    /// Replace all default shortcuts for a command; an empty list unbinds it.
    pub fn rebind(&mut self, id: &str, sequences: Vec<Vec<KeyEvent>>) -> bool {
        if sequences.iter().any(Vec::is_empty) {
            return false;
        }
        let Some(entry) = self
            .commands
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
        else {
            return false;
        };
        entry.descriptor.hotkeys.clear();
        entry.descriptor.sequences = sequences;
        self.pending.clear();
        true
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Some(entry) = self
            .commands
            .iter_mut()
            .find(|entry| entry.descriptor.id == id)
        else {
            return false;
        };
        entry.enabled(enabled);
        self.pending.clear();
        true
    }

    pub fn sequence_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.sequence_timeout = timeout;
        self
    }

    /// Palette entries and shortcut routing share availability and metadata.
    pub fn palette_items(
        &self,
        focused: Option<&WidgetId>,
    ) -> Vec<crate::tui::widgets::CommandItem> {
        self.commands
            .iter()
            .map(|entry| {
                let d = &entry.descriptor;
                let available = d.enabled
                    && d.scope
                        .as_ref()
                        .is_none_or(|scope| focused.is_some_and(|id| id.starts_with(scope)));
                let mut item =
                    crate::tui::widgets::CommandItem::new(&d.id, &d.title).disabled(!available);
                if let Some(description) = &d.description {
                    item = item.keyword(description);
                }
                item
            })
            .collect()
    }

    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        focused: Option<&WidgetId>,
        now: Instant,
    ) -> CommandMatch<M> {
        self.handle_key_in(key, focused, None, now)
    }

    /// A modal boundary excludes global commands and commands outside that subtree.
    pub fn handle_key_in(
        &mut self,
        key: &KeyEvent,
        focused: Option<&WidgetId>,
        modal: Option<&WidgetId>,
        now: Instant,
    ) -> CommandMatch<M> {
        if key.kind == KeyEventKind::Release {
            return CommandMatch::Ignored;
        }
        if self.deadline.is_some_and(|deadline| now >= deadline)
            || self.pending_focus.as_ref() != focused
        {
            self.pending.clear();
        }
        self.pending_focus = focused.cloned();
        self.pending.push(*key);
        // On a mismatch the final key can start a new shortcut.
        for attempt in 0..2 {
            let mut candidates: Vec<_> = self
                .commands
                .iter()
                .filter(|entry| {
                    let d = &entry.descriptor;
                    d.enabled
                        && d.scope
                            .as_ref()
                            .is_none_or(|scope| focused.is_some_and(|id| id.starts_with(scope)))
                        && modal.is_none_or(|modal| {
                            d.scope
                                .as_ref()
                                .is_some_and(|scope| scope.starts_with(modal))
                        })
                })
                .collect();
            candidates.sort_by_key(|entry| {
                std::cmp::Reverse(
                    entry
                        .descriptor
                        .scope
                        .as_ref()
                        .map_or(0, |scope| scope.len() + 1),
                )
            });
            for entry in candidates {
                let d = &entry.descriptor;
                let shortcuts = d
                    .hotkeys
                    .iter()
                    .map(std::slice::from_ref)
                    .chain(d.sequences.iter().map(Vec::as_slice));
                let mut prefix = false;
                for sequence in shortcuts {
                    if sequence.len() >= self.pending.len()
                        && sequence
                            .iter()
                            .zip(&self.pending)
                            .all(|(a, b)| hotkey_matches(a, b))
                    {
                        if sequence.len() == self.pending.len() {
                            self.pending.clear();
                            self.deadline = None;
                            return CommandMatch::Message((entry.handler)());
                        }
                        prefix = true;
                    }
                }
                if prefix {
                    self.deadline = Some(now + self.sequence_timeout);
                    return CommandMatch::Pending;
                }
            }
            if attempt == 0 && self.pending.len() > 1 {
                self.pending.clear();
                self.pending.push(*key);
            } else {
                break;
            }
        }
        self.pending.clear();
        self.deadline = None;
        CommandMatch::Ignored
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
