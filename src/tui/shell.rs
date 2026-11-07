//! Application helpers for shortcuts, commands, modal stacks, notices, and routes.
//!
//! [`HotkeyRegistry`] resolves individual keys; [`CommandRouter`] also supports
//! scoped commands and multi-key sequences. Install a router on an application
//! to share command metadata with a palette and route keyboard input consistently.
//! [`ModalManager`], [`NotificationCenter`], and [`Navigator`] store application
//! state; the application chooses how to render their contents.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::{
//!     event::{KeyCode, KeyEvent, KeyModifiers},
//!     shell::{CommandRouter, Navigator},
//! };
//!
//! let mut commands = CommandRouter::new();
//! commands.register("settings", "Open settings", || "settings")
//!     .hotkey(KeyEvent::new(KeyCode::Char(','), KeyModifiers::CONTROL));
//! let mut routes = Navigator::new("home");
//! routes.push(commands.dispatch("settings").unwrap());
//! assert_eq!(routes.current(), &"settings");
//! assert_eq!(routes.go_back(), Some("settings"));
//! assert_eq!(routes.current(), &"home");
//! ```

use std::fmt;
use std::time::{Duration, Instant};

use crate::tui::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::tui::widget::WidgetId;

/// Single-key shortcut metadata, including exact modifiers and help text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    /// Application-defined binding identifier.
    pub id: String,
    /// Key code and exact modifier combination used for matching.
    pub key_event: KeyEvent,
    /// Short display label for help or status UI.
    pub label: String,
    /// Optional longer explanation of the binding.
    pub description: Option<String>,
}

impl Hotkey {
    /// Creates shortcut metadata with an explicit key event and no description.
    pub fn new(id: impl Into<String>, key_event: KeyEvent, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key_event,
            label: label.into(),
            description: None,
        }
    }

    /// Creates a shortcut for a key code with no modifiers.
    pub fn simple(id: impl Into<String>, key_code: KeyCode, label: impl Into<String>) -> Self {
        Self::new(id, KeyEvent::new(key_code, KeyModifiers::empty()), label)
    }

    /// Sets the optional help description.
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

/// Ordered single-key bindings that produce application messages.
///
/// The first matching enabled binding wins. Matching compares key code and
/// modifiers, accepts presses and repeats, and ignores releases.
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
    /// Creates an empty binding registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an enabled binding; earlier matches take precedence.
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

    /// Appends a binding without modifiers, generating its ID and display label.
    pub fn bind_key<F>(&mut self, key_code: KeyCode, handler: F) -> &mut Self
    where
        F: Fn() -> M + 'static,
    {
        let id = format!("key::{key_code:?}");
        let label = hotkey_label(&KeyEvent::new(key_code, KeyModifiers::empty()));
        self.bind(Hotkey::simple(id, key_code, label), handler)
    }

    /// Invokes the first matching enabled binding and returns its message.
    ///
    /// Press and repeat events match by code and modifiers; releases never match.
    pub fn resolve(&self, key_event: &KeyEvent) -> Option<M> {
        self.bindings
            .iter()
            .find(|binding| binding.enabled && hotkey_matches(&binding.hotkey.key_event, key_event))
            .map(|binding| (binding.handler)())
    }

    /// Returns cloned metadata in registration order.
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

/// Display metadata, shortcuts, availability, and focus scope of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescriptor {
    /// Unique command identifier within the router.
    pub id: String,
    /// User-visible command title.
    pub title: String,
    /// Optional help text, also used as a command-palette search keyword.
    pub description: Option<String>,
    /// Single-key shortcuts in registration order.
    pub hotkeys: Vec<KeyEvent>,
    /// Nonempty key sequences in registration order.
    pub sequences: Vec<Vec<KeyEvent>>,
    /// Whether the command can be dispatched.
    pub enabled: bool,
    /// Optional widget identity subtree in which keyboard routing is available.
    pub scope: Option<WidgetId>,
}

/// A registered command with mutable metadata and a message-producing handler.
pub struct CommandEntry<M> {
    descriptor: CommandDescriptor,
    handler: Box<dyn Fn() -> M>,
}

impl<M> CommandEntry<M> {
    /// Sets help text also exposed as a command-palette keyword.
    pub fn description(&mut self, description: impl Into<String>) -> &mut Self {
        self.descriptor.description = Some(description.into());
        self
    }

    /// Appends a single-key shortcut.
    pub fn hotkey(&mut self, key_event: KeyEvent) -> &mut Self {
        self.descriptor.hotkeys.push(key_event);
        self
    }

    /// Sets whether this command is available for dispatch.
    pub fn enabled(&mut self, enabled: bool) -> &mut Self {
        self.descriptor.enabled = enabled;
        self
    }

    /// Restrict the command to focus within this identity subtree. Deeper scopes win.
    pub fn scope(&mut self, scope: WidgetId) -> &mut Self {
        self.descriptor.scope = Some(scope);
        self
    }

    /// Appends a multi-key shortcut in input order.
    ///
    /// # Panics
    ///
    /// Panics if `keys` is empty.
    pub fn sequence(&mut self, keys: impl IntoIterator<Item = KeyEvent>) -> &mut Self {
        let keys: Vec<_> = keys.into_iter().collect();
        assert!(!keys.is_empty(), "a command sequence cannot be empty");
        self.descriptor.sequences.push(keys);
        self
    }

    /// Returns this command's current metadata.
    pub fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }
}

/// Result of a shortcut/sequence lookup. A prefix consumes input until it
/// completes, mismatches, changes focus, or expires before the next key.
pub enum CommandMatch<M> {
    /// No shortcut or sequence prefix matched; input may continue to other handlers.
    Ignored,
    /// A sequence prefix matched and consumed the key, awaiting another key.
    Pending,
    /// A command matched and produced an application message.
    Message(M),
}

/// Routes command IDs, shortcuts, and key sequences to application messages.
///
/// Scoped commands take precedence over less-specific scopes during keyboard
/// routing. Prefixes are held until completion, mismatch, focus change, or
/// expiry. The default interval between sequence keys is one second.
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
    /// Creates an empty router with a one-second sequence timeout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an enabled global command and returns its configurable entry.
    ///
    /// # Panics
    ///
    /// Panics if another command already has the same ID.
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

    /// Invokes an enabled command by ID, returning `None` when absent or disabled.
    ///
    /// This direct dispatch does not check focus scopes or modal boundaries.
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

    /// Clears the current sequence prefix, deadline, and remembered focus.
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

    /// Updates availability and clears the pending prefix.
    ///
    /// Returns `false` if the ID is unknown; otherwise returns `true`.
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

    /// Sets the allowed interval between sequence keys.
    ///
    /// Expiry is checked on subsequent input rather than by a background timer.
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

    /// Routes a key using the focused identity, without a modal boundary.
    ///
    /// Presses and repeats can match; releases are ignored. Deeper eligible scopes
    /// win, with registration order breaking ties. A mismatched sequence retries
    /// its final key as the start of a new shortcut.
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

    /// Returns cloned command metadata in registration order.
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

/// Application-owned stack of modal descriptions.
///
/// The last item is active. This helper stores state; the view must render the
/// corresponding modal widgets and focus boundaries.
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
    /// Creates an empty modal stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a modal and makes it active.
    pub fn open(&mut self, modal: T) {
        self.stack.push(modal);
    }

    /// Replaces and returns the active modal, or pushes one and returns `None` when empty.
    pub fn replace_top(&mut self, modal: T) -> Option<T> {
        if let Some(top) = self.stack.last_mut() {
            Some(std::mem::replace(top, modal))
        } else {
            self.stack.push(modal);
            None
        }
    }

    /// Removes and returns the active modal, revealing the previous one.
    pub fn close_top(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Removes every modal.
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Returns the top modal, or `None` when empty.
    pub fn active(&self) -> Option<&T> {
        self.stack.last()
    }

    /// Returns mutable access to the top modal, or `None` when empty.
    pub fn active_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    /// Returns the number of open modal descriptions.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Returns whether the modal stack is nonempty.
    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Iterates from the oldest modal to the active one; reverse iteration starts at the top.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.stack.iter()
    }
}

/// Numeric notification identity allocated by a notification center.
pub type NotificationId = u64;

/// Semantic severity or outcome attached to a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Informational update.
    Info,
    /// Successful completion.
    Success,
    /// Condition requiring attention.
    Warning,
    /// Failed operation or error.
    Error,
}

/// Notification payload with identity, severity, and optional expiry.
#[derive(Debug, Clone)]
pub struct Notification<T> {
    /// Identifier assigned by the owning center.
    pub id: NotificationId,
    /// Severity or result classification.
    pub level: NotificationLevel,
    /// Application-defined notification content.
    pub payload: T,
    /// Wall-clock instant when the notification was added.
    pub created_at: Instant,
    /// Optional deadline used by explicit expiry pruning.
    pub expires_at: Option<Instant>,
}

/// Application-owned notification list with explicit expiry pruning.
///
/// Timed entries remain visible until [`Self::prune_expired`] is called. This
/// helper does not install timers, render widgets, or send notifications itself.
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
    /// Creates an empty center whose first allocated ID is zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a persistent notification and returns its ID.
    pub fn push(&mut self, level: NotificationLevel, payload: T) -> NotificationId {
        self.push_for(level, payload, None)
    }

    /// Adds a notification expiring `ttl` after the current wall-clock instant.
    ///
    /// Call [`Self::prune_expired`] to remove expired entries.
    ///
    /// # Panics
    ///
    /// Panics if adding `ttl` exceeds the representable range of [`Instant`].
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

    /// Removes and returns a notification by ID, or `None` if absent.
    pub fn dismiss(&mut self, id: NotificationId) -> Option<Notification<T>> {
        let index = self.items.iter().position(|item| item.id == id)?;
        Some(self.items.remove(index))
    }

    /// Removes all notifications without resetting the ID counter.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Removes entries whose expiry is at or before `now`, retaining persistent entries.
    pub fn prune_expired(&mut self, now: Instant) {
        self.items.retain(|item| {
            item.expires_at
                .map(|deadline| deadline > now)
                .unwrap_or(true)
        });
    }

    /// Returns stored notifications in insertion order, including unpruned expired entries.
    pub fn items(&self) -> &[Notification<T>] {
        &self.items
    }

    /// Returns the most recently added remaining notification, even if expired but unpruned.
    pub fn latest(&self) -> Option<&Notification<T>> {
        self.items.last()
    }
}

/// Application-owned route history that always retains at least one route.
///
/// The view renders the current route; pushing or replacing routes only changes
/// this stored history.
#[derive(Debug, Clone)]
pub struct Navigator<Route> {
    stack: Vec<Route>,
}

impl<Route> Navigator<Route> {
    /// Creates a history containing only `initial`.
    pub fn new(initial: Route) -> Self {
        Self {
            stack: vec![initial],
        }
    }

    /// Returns the current route.
    pub fn current(&self) -> &Route {
        self.stack
            .last()
            .expect("Navigator always keeps at least one route")
    }

    /// Returns mutable access to the current route.
    pub fn current_mut(&mut self) -> &mut Route {
        self.stack
            .last_mut()
            .expect("Navigator always keeps at least one route")
    }

    /// Appends a route and makes it current.
    pub fn push(&mut self, route: Route) {
        self.stack.push(route);
    }

    /// Replaces the current route without adding a history entry.
    pub fn replace(&mut self, route: Route) {
        if let Some(current) = self.stack.last_mut() {
            *current = route;
        } else {
            self.stack.push(route);
        }
    }

    /// Replaces all history with one route.
    pub fn reset(&mut self, route: Route) {
        self.stack.clear();
        self.stack.push(route);
    }

    /// Returns whether the history has more than one entry.
    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Removes and returns the current route, or returns `None` at the initial route.
    pub fn go_back(&mut self) -> Option<Route> {
        if self.can_go_back() {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Returns history from the oldest route to the current route.
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
