use rustc_hash::{FxHashMap, FxHashSet};

use crate::tui::widget::{Widget, WidgetId, WidgetKey, WidgetPath};

/// How a widget participates in keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FocusConfig {
    /// The widget does not participate in the focus system.
    #[default]
    None,
    /// A leaf focus target that participates in the global Tab order.
    Leaf,
    /// A composite focus target that owns internal navigation state.
    Composite,
    /// A focus scope that can trap Tab and restore previously focused children.
    Scope(FocusScope),
}

/// Policy for entering a focus scope.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScopeEntry {
    /// Focus the first focusable descendant.
    #[default]
    First,
    /// Restore the last focused descendant when available.
    LastFocused,
    /// Prefer a specific child path under the scope root.
    Child(WidgetKey),
}

/// Focus behavior for scope widgets such as dialogs or grouped forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusScope {
    /// Whether Tab should cycle inside this scope.
    pub trap_tab: bool,
    /// Whether the scope should remember the last focused descendant.
    pub restore_focus: bool,
    /// Which child should receive focus when entering the scope.
    pub entry: ScopeEntry,
}

impl Default for FocusScope {
    fn default() -> Self {
        Self {
            trap_tab: false,
            restore_focus: true,
            entry: ScopeEntry::First,
        }
    }
}

impl FocusScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trap_tab(mut self, trap_tab: bool) -> Self {
        self.trap_tab = trap_tab;
        self
    }

    pub fn restore_focus(mut self, restore_focus: bool) -> Self {
        self.restore_focus = restore_focus;
        self
    }

    pub fn entry(mut self, entry: ScopeEntry) -> Self {
        self.entry = entry;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget {
    pub id: WidgetId,
    pub path: WidgetPath,
}

#[derive(Debug, Clone)]
struct ScopeNode {
    id: WidgetId,
    path: WidgetPath,
    scope: FocusScope,
}

/// Snapshot of the focus-relevant parts of the current widget tree.
#[derive(Debug, Default, Clone)]
pub struct FocusAnalysis {
    live_paths: FxHashSet<WidgetPath>,
    live_ids: FxHashSet<WidgetId>,
    id_by_path: FxHashMap<WidgetPath, WidgetId>,
    path_by_id: FxHashMap<WidgetId, WidgetPath>,
    focus_targets: Vec<FocusTarget>,
    focus_target_ids: FxHashSet<WidgetId>,
    scopes_by_path: FxHashMap<WidgetPath, ScopeNode>,
    scopes_by_id: FxHashMap<WidgetId, ScopeNode>,
    modals: Vec<FocusTarget>,
}

impl FocusAnalysis {
    pub fn analyze<M>(root: &dyn Widget<M>) -> Self {
        let mut analysis = Self::default();
        let mut path = WidgetPath::root();
        let id = WidgetId::root();
        let stable_scope_id = WidgetId::root();
        Self::walk(root, &mut path, id, stable_scope_id, &mut analysis);
        analysis
    }

    fn walk<M>(
        widget: &dyn Widget<M>,
        path: &mut WidgetPath,
        id: WidgetId,
        stable_scope_id: WidgetId,
        analysis: &mut Self,
    ) {
        analysis.live_paths.insert(path.clone());
        analysis.live_ids.insert(id.clone());
        analysis.id_by_path.insert(path.clone(), id.clone());
        if let Some(previous) = analysis.path_by_id.insert(id.clone(), path.clone()) {
            panic!("Duplicate widget key/identity {id:?}: paths {previous:?} and {path:?}. Keys must be unique within the nearest keyed ancestor.");
        }

        if widget.is_modal() {
            analysis.modals.push(FocusTarget {
                id: id.clone(),
                path: path.clone(),
            });
        }
        match widget.focus_config() {
            FocusConfig::Leaf | FocusConfig::Composite => {
                analysis.focus_target_ids.insert(id.clone());
                analysis.focus_targets.push(FocusTarget {
                    id: id.clone(),
                    path: path.clone(),
                });
            }
            FocusConfig::Scope(scope) => {
                let node = ScopeNode {
                    id: id.clone(),
                    path: path.clone(),
                    scope,
                };
                analysis.scopes_by_path.insert(path.clone(), node.clone());
                analysis.scopes_by_id.insert(id.clone(), node);
            }
            FocusConfig::None => {}
        }

        for (index, child) in widget.children().iter().enumerate() {
            let key = WidgetKey::for_child(index, child.as_ref());
            let (child_id, child_stable_scope_id) =
                WidgetId::for_child(&id, &stable_scope_id, &key);
            path.push(key);
            Self::walk(
                child.as_ref(),
                path,
                child_id,
                child_stable_scope_id,
                analysis,
            );
            path.pop();
        }
    }

    pub fn live_paths(&self) -> &FxHashSet<WidgetPath> {
        &self.live_paths
    }

    pub fn live_ids(&self) -> &FxHashSet<WidgetId> {
        &self.live_ids
    }

    pub fn focus_targets(&self) -> &[FocusTarget] {
        &self.focus_targets
    }

    pub fn is_live(&self, path: &WidgetPath) -> bool {
        self.live_paths.contains(path)
    }

    pub fn id_for_path(&self, path: &WidgetPath) -> Option<&WidgetId> {
        self.id_by_path.get(path)
    }

    pub fn path_for_id(&self, id: &WidgetId) -> Option<&WidgetPath> {
        self.path_by_id.get(id)
    }

    pub fn is_focus_target_id(&self, id: &WidgetId) -> bool {
        self.focus_target_ids.contains(id)
    }

    pub fn scope(&self, path: &WidgetPath) -> Option<&FocusScope> {
        self.scopes_by_path.get(path).map(|node| &node.scope)
    }

    fn scope_node(&self, path: &WidgetPath) -> Option<&ScopeNode> {
        self.scopes_by_path.get(path)
    }

    fn scope_node_by_id(&self, id: &WidgetId) -> Option<&ScopeNode> {
        self.scopes_by_id.get(id)
    }

    pub fn descendant_targets<'a>(
        &'a self,
        scope_path: &'a WidgetPath,
    ) -> impl Iterator<Item = &'a FocusTarget> + 'a {
        self.focus_targets
            .iter()
            .filter(move |target| target.path.starts_with(scope_path))
    }

    pub fn first_descendant_target(&self, path: &WidgetPath) -> Option<&FocusTarget> {
        self.focus_targets
            .iter()
            .find(|target| target.path.starts_with(path))
    }
}

/// Manages keyboard focus navigation across focusable widgets and scopes.
#[derive(Debug, Default)]
pub struct FocusManager {
    analysis: FocusAnalysis,
    focused_id: Option<WidgetId>,
    scope_memory: FxHashMap<WidgetId, WidgetId>,
    modal_history: Vec<(WidgetId, Option<WidgetId>)>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// A replaced component must not inherit focus or scope restoration memory.
    pub(crate) fn forget(&mut self, id: &WidgetId) {
        if self.focused_id.as_ref() == Some(id) {
            self.focused_id = None;
        }
        self.scope_memory
            .retain(|scope, target| scope != id && target != id);
    }

    pub fn rebuild<M>(&mut self, root: &dyn Widget<M>) {
        let mut old_focus = self.focused_id.clone();
        self.analysis = FocusAnalysis::analyze(root);
        while self
            .modal_history
            .last()
            .is_some_and(|(id, _)| !self.analysis.modals.iter().any(|modal| &modal.id == id))
        {
            old_focus = self.modal_history.pop().unwrap().1;
        }
        for modal in &self.analysis.modals {
            if !self.modal_history.iter().any(|(id, _)| id == &modal.id) {
                self.modal_history
                    .push((modal.id.clone(), old_focus.clone()));
                old_focus = self
                    .analysis
                    .first_descendant_target(&modal.path)
                    .map(|target| target.id.clone());
            }
        }
        self.prune_scope_memory();

        self.focused_id = old_focus
            .as_ref()
            .filter(|id| self.analysis.is_focus_target_id(id))
            .cloned()
            .or_else(|| {
                self.analysis
                    .focus_targets()
                    .first()
                    .map(|target| target.id.clone())
            });

        if let Some(modal) = self.modal_path().cloned() {
            if !self
                .current_path()
                .is_some_and(|path| path.starts_with(&modal))
            {
                self.focused_id = self
                    .analysis
                    .first_descendant_target(&modal)
                    .map(|target| target.id.clone());
            }
        }
        if let Some(focused) = self.focused_id.clone() {
            self.remember_focus(&focused);
        }
    }

    /// Topmost modal subtree, if any.
    pub fn modal_path(&self) -> Option<&WidgetPath> {
        self.analysis.modals.last().map(|target| &target.path)
    }

    pub(crate) fn analysis_id(&self, path: &WidgetPath) -> Option<&WidgetId> {
        self.analysis.id_for_path(path)
    }

    pub fn current_id(&self) -> Option<&WidgetId> {
        self.focused_id.as_ref()
    }

    pub fn current_path(&self) -> Option<&WidgetPath> {
        self.focused_id
            .as_ref()
            .and_then(|id| self.analysis.path_for_id(id))
    }

    pub fn live_paths(&self) -> &FxHashSet<WidgetPath> {
        self.analysis.live_paths()
    }

    pub fn live_ids(&self) -> &FxHashSet<WidgetId> {
        self.analysis.live_ids()
    }

    pub fn request_focus(&mut self, path: &WidgetPath) -> bool {
        let Some(next) = self.resolve_requested_path(path) else {
            return false;
        };

        if self
            .modal_path()
            .is_some_and(|modal| !next.path.starts_with(modal))
        {
            return false;
        }
        self.set_focus(Some(next.id))
    }

    pub fn clear(&mut self) -> bool {
        if self.modal_path().is_some() {
            return false;
        }
        self.set_focus(None)
    }

    fn constrain_to_modal(&self, scope: Option<WidgetPath>) -> Option<WidgetPath> {
        match self.modal_path() {
            Some(modal) => Some(
                scope
                    .filter(|scope| scope.starts_with(modal))
                    .unwrap_or_else(|| modal.clone()),
            ),
            None => scope,
        }
    }

    pub fn next(&mut self) {
        if let Some(scope) = self.constrain_to_modal(self.active_trapping_scope()) {
            self.move_within_scope(&scope, true);
        } else {
            self.move_global(true);
        }
    }

    pub fn prev(&mut self) {
        if let Some(scope) = self.constrain_to_modal(self.active_trapping_scope()) {
            self.move_within_scope(&scope, false);
        } else {
            self.move_global(false);
        }
    }

    pub fn next_in_scope(&mut self, scope: Option<&WidgetPath>) {
        if let Some(scope_path) = self.constrain_to_modal(self.resolve_scope_path(scope)) {
            self.move_within_scope(&scope_path, true);
        } else {
            self.move_global(true);
        }
    }

    pub fn prev_in_scope(&mut self, scope: Option<&WidgetPath>) {
        if let Some(scope_path) = self.constrain_to_modal(self.resolve_scope_path(scope)) {
            self.move_within_scope(&scope_path, false);
        } else {
            self.move_global(false);
        }
    }

    pub fn is_focus_within(&self, path: &WidgetPath) -> bool {
        self.current_path()
            .map(|focused| focused.starts_with(path))
            .unwrap_or(false)
    }

    fn resolve_requested_path(&self, path: &WidgetPath) -> Option<FocusTarget> {
        let id = self.analysis.id_for_path(path)?;
        if self.analysis.is_focus_target_id(id) {
            return Some(FocusTarget {
                id: id.clone(),
                path: path.clone(),
            });
        }

        if self.analysis.scope(path).is_some() {
            return self.resolve_scope_entry(path);
        }

        if self.analysis.is_live(path) {
            return self.analysis.first_descendant_target(path).cloned();
        }

        None
    }

    fn resolve_scope_entry(&self, scope_path: &WidgetPath) -> Option<FocusTarget> {
        let scope_node = self.analysis.scope_node(scope_path)?;
        let scope = &scope_node.scope;

        if matches!(scope.entry, ScopeEntry::LastFocused) {
            if let Some(remembered) = self.scope_memory.get(&scope_node.id) {
                if let Some(target) = self.target_by_id_within(remembered, scope_path) {
                    return Some(target);
                }
            }
        }

        if let ScopeEntry::Child(ref key) = scope.entry {
            let requested = scope_path.child(key.clone());
            if let Some(target) = self.resolve_requested_path(&requested) {
                if target.path.starts_with(scope_path) {
                    return Some(target);
                }
            }
        }

        if matches!(scope.entry, ScopeEntry::First) {
            if let Some(target) = self.analysis.first_descendant_target(scope_path) {
                return Some(target.clone());
            }
        }

        if scope.restore_focus {
            if let Some(remembered) = self.scope_memory.get(&scope_node.id) {
                if let Some(target) = self.target_by_id_within(remembered, scope_path) {
                    return Some(target);
                }
            }
        }

        self.analysis.first_descendant_target(scope_path).cloned()
    }

    fn target_by_id_within(&self, id: &WidgetId, scope_path: &WidgetPath) -> Option<FocusTarget> {
        if !self.analysis.is_focus_target_id(id) {
            return None;
        }
        let path = self.analysis.path_for_id(id)?;
        path.starts_with(scope_path).then(|| FocusTarget {
            id: id.clone(),
            path: path.clone(),
        })
    }

    fn set_focus(&mut self, next: Option<WidgetId>) -> bool {
        if self.focused_id == next {
            return false;
        }
        self.focused_id = next;
        if let Some(focused) = self.focused_id.clone() {
            self.remember_focus(&focused);
        }
        true
    }

    fn move_global(&mut self, forward: bool) {
        let targets = self.analysis.focus_targets();
        if targets.is_empty() {
            return;
        }

        let next_index = match self
            .focused_id
            .as_ref()
            .and_then(|id| targets.iter().position(|target| &target.id == id))
        {
            Some(index) if forward => (index + 1) % targets.len(),
            Some(0) => targets.len() - 1,
            Some(index) => index - 1,
            None if forward => 0,
            None => targets.len() - 1,
        };

        self.set_focus(Some(targets[next_index].id.clone()));
    }

    fn move_within_scope(&mut self, scope_path: &WidgetPath, forward: bool) {
        let targets: Vec<&FocusTarget> = self.analysis.descendant_targets(scope_path).collect();
        if targets.is_empty() {
            return;
        }

        let next_index = match self
            .focused_id
            .as_ref()
            .and_then(|id| targets.iter().position(|target| &target.id == id))
        {
            Some(index) if forward => (index + 1) % targets.len(),
            Some(0) => targets.len() - 1,
            Some(index) => index - 1,
            None if forward => 0,
            None => targets.len() - 1,
        };

        self.set_focus(Some(targets[next_index].id.clone()));
    }

    fn resolve_scope_path(&self, requested: Option<&WidgetPath>) -> Option<WidgetPath> {
        if let Some(path) = requested {
            return self.analysis.scope(path).map(|_| path.clone());
        }

        let focused = self.current_path()?;
        self.ancestor_scope_paths(focused).pop()
    }

    fn active_trapping_scope(&self) -> Option<WidgetPath> {
        let focused = self.current_path()?;
        self.ancestor_scope_paths(focused)
            .into_iter()
            .rev()
            .find(|scope_path| {
                self.analysis
                    .scope(scope_path)
                    .map(|scope| scope.trap_tab)
                    .unwrap_or(false)
            })
    }

    fn ancestor_scope_paths(&self, path: &WidgetPath) -> Vec<WidgetPath> {
        let mut scopes = Vec::new();
        for ancestor in path.ancestors_inclusive() {
            if self.analysis.scope(&ancestor).is_some() {
                scopes.push(ancestor);
            }
        }
        scopes
    }

    fn remember_focus(&mut self, focused_id: &WidgetId) {
        let Some(focused_path) = self.analysis.path_for_id(focused_id) else {
            return;
        };

        for ancestor in focused_path.ancestors_inclusive() {
            if let Some(scope) = self.analysis.scope_node(&ancestor) {
                self.scope_memory
                    .insert(scope.id.clone(), focused_id.clone());
            }
        }
    }

    fn prune_scope_memory(&mut self) {
        let analysis = &self.analysis;
        self.scope_memory.retain(|scope_id, focused_id| {
            let Some(scope) = analysis.scope_node_by_id(scope_id) else {
                return false;
            };
            let Some(focused_path) = analysis.path_for_id(focused_id) else {
                return false;
            };
            analysis.is_focus_target_id(focused_id) && focused_path.starts_with(&scope.path)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{FocusConfig, FocusManager, FocusScope, ScopeEntry};
    use crate::tui::layout::Constraints;
    use crate::tui::widget::{Widget, WidgetKey, WidgetPath};

    struct TestWidget {
        key: Option<&'static str>,
        focus: FocusConfig,
        children: Vec<Box<dyn Widget<()>>>,
    }

    impl TestWidget {
        fn node(key: Option<&'static str>, focus: FocusConfig) -> Self {
            Self {
                key,
                focus,
                children: Vec::new(),
            }
        }

        fn with_children(mut self, children: Vec<Box<dyn Widget<()>>>) -> Self {
            self.children = children;
            self
        }
    }

    impl Widget<()> for TestWidget {
        fn render(
            &self,
            _chunk: &mut crate::render::chunk::Chunk,
            _ctx: &crate::tui::widget::RenderCtx,
        ) {
        }

        fn constraints(&self) -> Constraints {
            Constraints::fixed(1, 1)
        }

        fn focus_config(&self) -> FocusConfig {
            self.focus.clone()
        }

        fn children(&self) -> &[Box<dyn Widget<()>>] {
            &self.children
        }

        fn key(&self) -> Option<&str> {
            self.key
        }
    }

    fn path(parts: &[&str]) -> WidgetPath {
        let mut path = WidgetPath::root();
        for part in parts {
            path.push(WidgetKey::Named((*part).to_owned()));
        }
        path
    }

    #[test]
    fn scope_request_uses_explicit_entry() {
        let tree = TestWidget::node(None, FocusConfig::None).with_children(vec![
            Box::new(
                TestWidget::node(
                    Some("dialog"),
                    FocusConfig::Scope(
                        FocusScope::new()
                            .entry(ScopeEntry::Child(WidgetKey::Named("second".into()))),
                    ),
                )
                .with_children(vec![
                    Box::new(TestWidget::node(Some("first"), FocusConfig::Leaf)),
                    Box::new(TestWidget::node(Some("second"), FocusConfig::Leaf)),
                ]),
            ),
            Box::new(TestWidget::node(Some("outside"), FocusConfig::Leaf)),
        ]);

        let mut focus = FocusManager::new();
        focus.rebuild(&tree);

        assert!(focus.request_focus(&path(&["outside"])));
        assert!(focus.request_focus(&path(&["dialog"])));
        assert_eq!(
            focus.current_path().cloned(),
            Some(path(&["dialog", "second"]))
        );
    }

    #[test]
    fn requesting_unfocusable_path_preserves_current_focus() {
        let tree = TestWidget::node(None, FocusConfig::None).with_children(vec![
            Box::new(TestWidget::node(Some("first"), FocusConfig::Leaf)),
            Box::new(TestWidget::node(Some("disabled"), FocusConfig::None)),
            Box::new(TestWidget::node(Some("second"), FocusConfig::Leaf)),
        ]);

        let mut focus = FocusManager::new();
        focus.rebuild(&tree);
        assert_eq!(focus.current_path().cloned(), Some(path(&["first"])));

        assert!(focus.request_focus(&path(&["second"])));
        assert_eq!(focus.current_path().cloned(), Some(path(&["second"])));

        assert!(!focus.request_focus(&path(&["disabled"])));
        assert_eq!(focus.current_path().cloned(), Some(path(&["second"])));
    }

    #[test]
    fn last_focused_entry_restores_previous_descendant() {
        let tree = TestWidget::node(None, FocusConfig::None).with_children(vec![
            Box::new(
                TestWidget::node(
                    Some("dialog"),
                    FocusConfig::Scope(
                        FocusScope::new()
                            .restore_focus(true)
                            .entry(ScopeEntry::LastFocused),
                    ),
                )
                .with_children(vec![
                    Box::new(TestWidget::node(Some("first"), FocusConfig::Leaf)),
                    Box::new(TestWidget::node(Some("second"), FocusConfig::Leaf)),
                ]),
            ),
            Box::new(TestWidget::node(Some("outside"), FocusConfig::Leaf)),
        ]);

        let mut focus = FocusManager::new();
        focus.rebuild(&tree);

        assert!(focus.request_focus(&path(&["dialog", "second"])));
        assert!(focus.request_focus(&path(&["outside"])));
        assert!(focus.request_focus(&path(&["dialog"])));
        assert_eq!(
            focus.current_path().cloned(),
            Some(path(&["dialog", "second"]))
        );
    }

    #[test]
    fn trapping_scope_cycles_within_scope() {
        let tree = TestWidget::node(None, FocusConfig::None).with_children(vec![
            Box::new(
                TestWidget::node(
                    Some("scope"),
                    FocusConfig::Scope(FocusScope::new().trap_tab(true)),
                )
                .with_children(vec![
                    Box::new(TestWidget::node(Some("a"), FocusConfig::Leaf)),
                    Box::new(TestWidget::node(Some("b"), FocusConfig::Leaf)),
                ]),
            ),
            Box::new(TestWidget::node(Some("outside"), FocusConfig::Leaf)),
        ]);

        let mut focus = FocusManager::new();
        focus.rebuild(&tree);
        assert!(focus.request_focus(&path(&["outside"])));
        assert!(focus.request_focus(&path(&["scope", "a"])));

        focus.next();
        assert_eq!(focus.current_path().cloned(), Some(path(&["scope", "b"])));

        focus.next();
        assert_eq!(focus.current_path().cloned(), Some(path(&["scope", "a"])));

        assert!(focus.request_focus(&path(&["outside"])));
        focus.next();
        assert_eq!(focus.current_path().cloned(), Some(path(&["scope", "a"])));
    }

    #[test]
    fn keyed_focus_survives_inserted_unkeyed_wrapper_before_it() {
        let first_tree = TestWidget::node(None, FocusConfig::None).with_children(vec![Box::new(
            TestWidget::node(None, FocusConfig::None).with_children(vec![Box::new(
                TestWidget::node(Some("target"), FocusConfig::Leaf),
            )]),
        )]);

        let second_tree = TestWidget::node(None, FocusConfig::None).with_children(vec![
            Box::new(TestWidget::node(None, FocusConfig::None)),
            Box::new(
                TestWidget::node(None, FocusConfig::None).with_children(vec![Box::new(
                    TestWidget::node(Some("target"), FocusConfig::Leaf),
                )]),
            ),
        ]);

        let mut first_path = WidgetPath::root();
        first_path.push(WidgetKey::Index(0));
        first_path.push(WidgetKey::Named("target".into()));

        let mut second_path = WidgetPath::root();
        second_path.push(WidgetKey::Index(1));
        second_path.push(WidgetKey::Named("target".into()));

        let mut focus = FocusManager::new();
        focus.rebuild(&first_tree);
        assert_eq!(focus.current_path().cloned(), Some(first_path));

        focus.rebuild(&second_tree);
        assert_eq!(focus.current_path().cloned(), Some(second_path));
    }
}
