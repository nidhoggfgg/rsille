//! TreeView widget - hierarchical data display component
//!
//! A modern tree component inspired by contemporary UI frameworks,
//! adapted for terminal interfaces with support for various tree data structures.

use super::*;
use crate::event::{Event, KeyCode, MouseButton, MouseEventKind};
use crate::style::{Style, ThemeManager};
use crate::widget::common::SelectableNavigation;
use std::sync::Arc;

/// Trait for tree node data that can be displayed in a TreeView
///
/// Implement this trait to use custom data structures with TreeView.
pub trait TreeNode: Clone + Send + Sync {
    /// Get the display label for this node
    fn label(&self) -> String;

    /// Get the children of this node
    fn children(&self) -> Vec<Self>;

    /// Check if this node has children
    fn has_children(&self) -> bool {
        !self.children().is_empty()
    }

    /// Check if this node is disabled (cannot be selected)
    fn is_disabled(&self) -> bool {
        false
    }

    /// Get the icon for this node (optional)
    fn icon(&self) -> Option<&str> {
        None
    }
}

/// Simple tree node implementation for basic use cases
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleTreeNode<T: Clone> {
    /// The actual value/data of this node
    pub value: T,
    /// Display label for this node
    pub label: String,
    /// Child nodes
    pub children: Vec<SimpleTreeNode<T>>,
    /// Whether this node is disabled
    pub disabled: bool,
    /// Optional icon
    pub icon: Option<String>,
}

impl<T: Clone> SimpleTreeNode<T> {
    /// Create a new leaf node (no children)
    pub fn leaf(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            children: Vec::new(),
            disabled: false,
            icon: None,
        }
    }

    /// Create a new branch node (with children)
    pub fn branch(value: T, label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            value,
            label: label.into(),
            children,
            disabled: false,
            icon: None,
        }
    }

    /// Mark this node as disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set an icon for this node
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl<T: Clone + Send + Sync> TreeNode for SimpleTreeNode<T> {
    fn label(&self) -> String {
        self.label.clone()
    }

    fn children(&self) -> Vec<Self> {
        self.children.clone()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn is_disabled(&self) -> bool {
        self.disabled
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }
}

/// Flattened tree node for internal rendering
#[derive(Debug, Clone)]
struct FlatNode<T: TreeNode> {
    /// The tree node data
    node: T,
    /// Depth level in the tree (0 = root)
    depth: usize,
    /// Whether this node is expanded
    expanded: bool,
    /// Path indices from root to this node
    path: Vec<usize>,
}

/// Selection event information
#[derive(Debug, Clone)]
pub struct TreeSelectionEvent<T: TreeNode> {
    /// The selected node
    pub node: T,
    /// Path indices from root to the selected node
    pub path: Vec<usize>,
    /// Current set of all expanded paths (for state persistence)
    pub expanded_paths: std::collections::HashSet<Vec<usize>>,
    /// Currently selected node index
    pub selected_index: Option<usize>,
}

/// Expand/collapse event information
#[derive(Debug, Clone)]
pub struct TreeExpandEvent<T: TreeNode> {
    /// The node that was expanded or collapsed
    pub node: T,
    /// Path indices from root to the node
    pub path: Vec<usize>,
    /// Whether the node is now expanded (true) or collapsed (false)
    pub expanded: bool,
    /// Current set of all expanded paths (for state persistence)
    pub expanded_paths: std::collections::HashSet<Vec<usize>>,
    /// Currently selected node index
    pub selected_index: Option<usize>,
}

/// Interactive hierarchical tree widget
///
/// A modern tree component with support for:
/// - Keyboard navigation (arrows for navigation, Enter to expand/collapse)
/// - Mouse click selection and expand/collapse
/// - Expandable/collapsible nodes
/// - Custom icons for node types
/// - Selection support
/// - Lazy loading support (via TreeNode trait)
/// - Custom styling
///
/// # Examples
/// ```
/// use tui::widget::{TreeView, SimpleTreeNode};
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     NodeSelected(String),
/// }
///
/// let root = SimpleTreeNode::branch(
///     "root",
///     "Root",
///     vec![
///         SimpleTreeNode::leaf("child1", "Child 1"),
///         SimpleTreeNode::branch(
///             "child2",
///             "Child 2",
///             vec![
///                 SimpleTreeNode::leaf("grandchild1", "Grandchild 1"),
///             ]
///         ),
///     ]
/// );
///
/// let tree = TreeView::new(vec![root])
///     .on_select(|event| Message::NodeSelected(event.node.label()));
/// ```
#[derive(Clone)]
pub struct TreeView<T: TreeNode, M = ()> {
    /// Root nodes of the tree
    roots: Vec<T>,
    /// Flattened visible nodes (only expanded branches are included)
    flat_nodes: Vec<FlatNode<T>>,
    /// Currently selected flat node index
    selected_index: Option<usize>,
    /// Set of expanded node paths (stored as path strings for persistence)
    expanded_paths: std::collections::HashSet<Vec<usize>>,
    /// Navigation helper
    navigation: SelectableNavigation,
    /// Empty state message
    empty_message: String,
    /// Whether to show expand/collapse icons
    show_icons: bool,
    /// Whether to show scrollbar
    show_scrollbar: bool,
    /// Whether the tree is focused
    focused: bool,
    /// Custom style
    custom_style: Option<Style>,
    /// Custom focus style
    custom_focus_style: Option<Style>,
    /// Custom selected style
    custom_selected_style: Option<Style>,
    /// Selection change handler
    on_select: Option<Arc<dyn Fn(TreeSelectionEvent<T>) -> M + Send + Sync>>,
    /// Expand/collapse change handler
    on_expand: Option<Arc<dyn Fn(TreeExpandEvent<T>) -> M + Send + Sync>>,
}

impl<T: TreeNode, M> std::fmt::Debug for TreeView<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeView")
            .field("roots", &self.roots.len())
            .field("flat_nodes", &self.flat_nodes.len())
            .field("selected_index", &self.selected_index)
            .field("empty_message", &self.empty_message)
            .field("show_icons", &self.show_icons)
            .field("show_scrollbar", &self.show_scrollbar)
            .field("focused", &self.focused)
            .field("on_select", &self.on_select.is_some())
            .finish()
    }
}

impl<T: TreeNode, M> TreeView<T, M> {
    /// Create a new tree view with root nodes
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{TreeView, SimpleTreeNode};
    ///
    /// let roots = vec![
    ///     SimpleTreeNode::leaf("node1", "Node 1"),
    ///     SimpleTreeNode::leaf("node2", "Node 2"),
    /// ];
    ///
    /// let tree = TreeView::<_, ()>::new(roots);
    /// ```
    pub fn new(roots: Vec<T>) -> Self {
        let mut tree = Self {
            roots: roots.clone(),
            flat_nodes: Vec::new(),
            selected_index: None,
            expanded_paths: std::collections::HashSet::new(),
            navigation: SelectableNavigation::with_initial_focus(0, 10, None),
            empty_message: "No items".to_string(),
            show_icons: true,
            show_scrollbar: true,
            focused: false,
            custom_style: None,
            custom_focus_style: None,
            custom_selected_style: None,
            on_select: None,
            on_expand: None,
        };

        // Build initial flat structure
        tree.rebuild_flat_nodes();

        // Auto-focus first non-disabled node
        if !tree.flat_nodes.is_empty() {
            tree.selected_index = tree
                .flat_nodes
                .iter()
                .position(|node| !node.node.is_disabled());
            tree.navigation = SelectableNavigation::with_initial_focus(
                tree.flat_nodes.len(),
                10,
                tree.selected_index,
            );
        }

        tree
    }

    /// Create an empty tree view
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Set the empty state message
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    /// Set whether to show expand/collapse icons
    pub fn show_icons(mut self, show: bool) -> Self {
        self.show_icons = show;
        self
    }

    /// Set default expanded state - expand all nodes
    pub fn default_expanded(mut self, expanded: bool) -> Self {
        if expanded {
            // Collect all paths that have children
            let mut paths = std::collections::HashSet::new();
            for (i, root) in self.roots.iter().enumerate() {
                Self::collect_parent_paths_recursive_static(root, vec![i], &mut paths);
            }
            self.expanded_paths = paths;
            // Rebuild to reflect the expanded state
            self.rebuild_flat_nodes();
        }
        self
    }

    /// Recursively collect paths of nodes that have children (static version)
    fn collect_parent_paths_recursive_static(
        node: &T,
        path: Vec<usize>,
        paths: &mut std::collections::HashSet<Vec<usize>>,
    ) {
        if node.has_children() {
            paths.insert(path.clone());
            let children = node.children();
            for (i, child) in children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                Self::collect_parent_paths_recursive_static(&child, child_path, paths);
            }
        }
    }

    /// Set whether to show the scrollbar
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set the viewport height (number of visible items)
    pub fn viewport_height(mut self, height: u16) -> Self {
        self.navigation.set_viewport_size(height as usize);
        self
    }

    /// Set the selection change handler
    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(TreeSelectionEvent<T>) -> M + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(handler));
        self
    }

    /// Set the expand/collapse change handler
    ///
    /// This handler is called whenever a node is expanded or collapsed.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::{TreeView, SimpleTreeNode};
    ///
    /// #[derive(Clone)]
    /// enum Message {
    ///     NodeExpanded(String, bool),
    /// }
    ///
    /// let tree = TreeView::new(vec![])
    ///     .on_expand(|event| Message::NodeExpanded(event.node.label(), event.expanded));
    /// ```
    pub fn on_expand<F>(mut self, handler: F) -> Self
    where
        F: Fn(TreeExpandEvent<T>) -> M + Send + Sync + 'static,
    {
        self.on_expand = Some(Arc::new(handler));
        self
    }

    /// Set the expanded paths from external state
    ///
    /// This allows persisting the expand/collapse state across renders.
    ///
    /// # Examples
    /// ```
    /// use tui::widget::TreeView;
    /// use std::collections::HashSet;
    ///
    /// let expanded_paths: HashSet<Vec<usize>> = HashSet::new();
    /// let tree = TreeView::new(vec![])
    ///     .expanded_paths(expanded_paths);
    /// ```
    pub fn expanded_paths(mut self, paths: std::collections::HashSet<Vec<usize>>) -> Self {
        self.expanded_paths = paths;
        self.rebuild_flat_nodes();
        self
    }

    /// Set the selected node index from external state
    ///
    /// This allows persisting the selection state across renders.
    pub fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self.navigation.set_focused_index(index);
        self
    }

    /// Set a custom style
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Set a custom focus style
    pub fn focus_style(mut self, style: Style) -> Self {
        self.custom_focus_style = Some(style);
        self
    }

    /// Set a custom selected item style
    pub fn selected_style(mut self, style: Style) -> Self {
        self.custom_selected_style = Some(style);
        self
    }

    /// Rebuild the flattened node list based on expanded state
    fn rebuild_flat_nodes(&mut self) {
        self.flat_nodes.clear();
        let roots = self.roots.clone();
        for (i, root) in roots.iter().enumerate() {
            self.flatten_node(root.clone(), 0, vec![i]);
        }
        // Update navigation total items
        self.navigation = SelectableNavigation::with_initial_focus(
            self.flat_nodes.len(),
            self.navigation.viewport_size(),
            self.selected_index,
        );
    }

    /// Recursively flatten a node and its expanded children
    fn flatten_node(&mut self, node: T, depth: usize, path: Vec<usize>) {
        let has_children = node.has_children();
        let is_expanded = self.is_path_expanded(&path);

        self.flat_nodes.push(FlatNode {
            node: node.clone(),
            depth,
            expanded: is_expanded,
            path: path.clone(),
        });

        // If expanded, add children
        if is_expanded && has_children {
            let children = node.children();
            for (i, child) in children.into_iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                self.flatten_node(child, depth + 1, child_path);
            }
        }
    }

    /// Check if a path is expanded
    fn is_path_expanded(&self, path: &[usize]) -> bool {
        self.expanded_paths.contains(path)
    }

    /// Toggle expand/collapse of the currently selected node
    fn toggle_expand(&mut self) -> Vec<M> {
        if let Some(idx) = self.selected_index {
            if idx < self.flat_nodes.len() {
                let has_children = self.flat_nodes[idx].node.has_children();
                if has_children {
                    let path = self.flat_nodes[idx].path.clone();
                    let node = self.flat_nodes[idx].node.clone();

                    // Determine the new expanded state
                    let was_expanded = self.expanded_paths.contains(&path);
                    let is_now_expanded = !was_expanded;

                    // Toggle expanded state in the set
                    if was_expanded {
                        self.expanded_paths.remove(&path);
                    } else {
                        self.expanded_paths.insert(path.clone());
                    }

                    // Rebuild flat structure
                    self.rebuild_flat_nodes();

                    // Find the node with the same path and restore selection
                    self.selected_index = self
                        .flat_nodes
                        .iter()
                        .position(|n| n.path == path)
                        .or_else(|| {
                            // If not found, keep the selection valid
                            if self.flat_nodes.is_empty() {
                                None
                            } else {
                                Some(idx.min(self.flat_nodes.len() - 1))
                            }
                        });
                    // Update navigation focus
                    self.navigation.set_focused_index(self.selected_index);

                    // Emit expand event
                    if let Some(ref handler) = self.on_expand {
                        let event = TreeExpandEvent {
                            node,
                            path,
                            expanded: is_now_expanded,
                            expanded_paths: self.expanded_paths.clone(),
                            selected_index: self.selected_index,
                        };
                        let message = handler(event);
                        return vec![message];
                    }
                }
            }
        }
        vec![]
    }

    /// Move selection to parent node
    fn select_parent(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.flat_nodes.len() {
                let current_path = &self.flat_nodes[idx].path;
                if current_path.len() > 1 {
                    // Find parent by removing last element from path
                    let parent_path: Vec<usize> = current_path[..current_path.len() - 1].to_vec();
                    if let Some(parent_idx) =
                        self.flat_nodes.iter().position(|n| n.path == parent_path)
                    {
                        self.selected_index = Some(parent_idx);
                        self.navigation.set_focused_index(Some(parent_idx));
                        self.navigation.ensure_visible(self.flat_nodes.len());
                    }
                }
            }
        }
    }

    /// Move selection to first child (if expanded)
    fn select_first_child(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.flat_nodes.len() {
                let current = &self.flat_nodes[idx];
                if current.expanded && current.node.has_children() {
                    // Next item in flat list is the first child
                    if idx + 1 < self.flat_nodes.len() {
                        self.selected_index = Some(idx + 1);
                        self.navigation.set_focused_index(Some(idx + 1));
                        self.navigation.ensure_visible(self.flat_nodes.len());
                    }
                }
            }
        }
    }

    /// Emit selection change event
    fn emit_selection(&self) -> Vec<M> {
        if let Some(ref handler) = self.on_select {
            if let Some(idx) = self.selected_index {
                if let Some(flat_node) = self.flat_nodes.get(idx) {
                    let event = TreeSelectionEvent {
                        node: flat_node.node.clone(),
                        path: flat_node.path.clone(),
                        expanded_paths: self.expanded_paths.clone(),
                        selected_index: self.selected_index,
                    };
                    let message = handler(event);
                    return vec![message];
                }
            }
        }
        vec![]
    }

    /// Get expand/collapse icon for a node
    fn get_expand_icon(&self, flat_node: &FlatNode<T>) -> &str {
        if !flat_node.node.has_children() {
            "  "
        } else if flat_node.expanded {
            "▼ "
        } else {
            "▶ "
        }
    }

    /// Calculate indent string based on depth
    fn get_indent(&self, depth: usize) -> String {
        "  ".repeat(depth)
    }
}

/// Pre-computed styles for efficient rendering
struct TreeStyles {
    normal: render::style::Style,
    focused: render::style::Style,
    selected: render::style::Style,
    disabled: render::style::Style,
    scrollbar_track: render::style::Style,
    scrollbar_thumb: render::style::Style,
}

impl<T: TreeNode, M> TreeView<T, M> {
    /// Get pre-computed styles for all states
    fn get_tree_styles(&self) -> TreeStyles {
        ThemeManager::global().with_theme(|theme| {
            let normal = self
                .custom_style
                .unwrap_or(Style::default().fg(theme.colors.text))
                .to_render_style();

            let base_focused = Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.focus_background);
            let focused = self
                .custom_focus_style
                .as_ref()
                .map(|s| s.merge(base_focused))
                .unwrap_or(base_focused)
                .to_render_style();

            let base_selected = Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.primary);
            let selected = self
                .custom_selected_style
                .as_ref()
                .map(|s| s.merge(base_selected))
                .unwrap_or(base_selected)
                .to_render_style();

            let disabled = Style::default()
                .fg(theme.colors.text_muted)
                .to_render_style();

            let scrollbar_track = Style::default().fg(theme.colors.border).to_render_style();
            let scrollbar_thumb = Style::default().fg(theme.colors.info).to_render_style();

            TreeStyles {
                normal,
                focused,
                selected,
                disabled,
                scrollbar_track,
                scrollbar_thumb,
            }
        })
    }

    /// Select style for a node
    #[inline]
    fn select_node_style(
        &self,
        styles: &TreeStyles,
        is_disabled: bool,
        is_selected: bool,
    ) -> render::style::Style {
        if is_disabled {
            styles.disabled
        } else if is_selected {
            styles.selected
        } else if is_selected && self.focused {
            styles.focused
        } else {
            styles.normal
        }
    }
}

impl<T: TreeNode + 'static, M: Send + Sync> Widget<M> for TreeView<T, M> {
    fn render(&self, chunk: &mut render::chunk::Chunk) {
        let area = chunk.area();
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        let width = area.width();
        let height = area.height();

        // Handle empty state
        if self.flat_nodes.is_empty() {
            let style = ThemeManager::global()
                .with_theme(|theme| theme.styles.text_muted.to_render_style());

            use unicode_width::UnicodeWidthStr;
            let msg_width = self.empty_message.width() as u16;
            let x = if width > msg_width {
                (width - msg_width) / 2
            } else {
                0
            };
            let y = height / 2;

            let _ = chunk.set_string(x, y, &self.empty_message, style);
            return;
        }

        // Pre-compute all styles
        let tree_styles = self.get_tree_styles();

        // Calculate visible range
        let viewport_size = height.min(self.navigation.viewport_size() as u16) as usize;
        let visible_start = self.navigation.scroll_offset();
        let visible_end = (visible_start + viewport_size).min(self.flat_nodes.len());

        // Determine scrollbar width
        let scrollbar_width = if self.show_scrollbar && self.flat_nodes.len() > viewport_size {
            2
        } else {
            0
        };
        let content_width = width.saturating_sub(scrollbar_width);

        // Render visible nodes
        let mut y = 0u16;
        for (flat_idx, flat_node) in self
            .flat_nodes
            .iter()
            .enumerate()
            .skip(visible_start)
            .take(visible_end - visible_start)
        {
            if y >= height {
                break;
            }

            let is_selected = Some(flat_idx) == self.selected_index;
            let is_disabled = flat_node.node.is_disabled();

            let render_style = self.select_node_style(&tree_styles, is_disabled, is_selected);

            // Fill background for selected items
            if is_selected {
                let _ = chunk.fill(0, y, content_width, 1, ' ', render_style);
            }

            // Render indent
            let indent = self.get_indent(flat_node.depth);
            let mut x = 0u16;
            let _ = chunk.set_string(x, y, &indent, render_style);
            x += indent.len() as u16;

            // Render expand/collapse icon
            if self.show_icons {
                let expand_icon = self.get_expand_icon(flat_node);
                let _ = chunk.set_string(x, y, expand_icon, render_style);
                x += expand_icon.len() as u16;
            }

            // Render node icon if present
            if let Some(icon) = flat_node.node.icon() {
                let _ = chunk.set_string(x, y, icon, render_style);
                x += icon.len() as u16;
                let _ = chunk.set_string(x, y, " ", render_style);
                x += 1;
            }

            // Render label
            use unicode_width::UnicodeWidthStr;
            let label = flat_node.node.label();
            let available_width = content_width.saturating_sub(x);

            if label.width() as u16 > available_width {
                let mut truncated = String::with_capacity(available_width as usize);
                let mut current_width = 0;
                for ch in label.chars() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if current_width + ch_width + 1 > available_width {
                        truncated.push('…');
                        break;
                    }
                    truncated.push(ch);
                    current_width += ch_width;
                }
                let _ = chunk.set_string(x, y, &truncated, render_style);
            } else {
                let _ = chunk.set_string(x, y, &label, render_style);
            }

            y += 1;
        }

        // Render scrollbar if needed
        if self.show_scrollbar && self.flat_nodes.len() > viewport_size {
            self.render_scrollbar(chunk, content_width, height, viewport_size, &tree_styles);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult<M> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Down => {
                    self.navigation.focus_next(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                KeyCode::Up => {
                    self.navigation.focus_previous(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                KeyCode::Right => {
                    // Expand if collapsed, or move to first child if expanded
                    let mut messages = vec![];
                    if let Some(idx) = self.selected_index {
                        if idx < self.flat_nodes.len() {
                            let has_children = self.flat_nodes[idx].node.has_children();
                            let is_expanded = self.flat_nodes[idx].expanded;
                            if has_children {
                                if !is_expanded {
                                    // Expand
                                    messages = self.toggle_expand();
                                } else {
                                    // Move to first child
                                    self.select_first_child();
                                }
                            }
                        }
                    }
                    EventResult::Consumed(messages)
                }
                KeyCode::Left => {
                    // Collapse if expanded, or move to parent if collapsed
                    let mut messages = vec![];
                    if let Some(idx) = self.selected_index {
                        if idx < self.flat_nodes.len() {
                            let has_children = self.flat_nodes[idx].node.has_children();
                            let is_expanded = self.flat_nodes[idx].expanded;
                            if has_children && is_expanded {
                                // Collapse
                                messages = self.toggle_expand();
                            } else {
                                // Move to parent
                                self.select_parent();
                            }
                        }
                    }
                    EventResult::Consumed(messages)
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Toggle expand/collapse and emit selection
                    let mut messages = self.toggle_expand();
                    messages.extend(self.emit_selection());
                    EventResult::Consumed(messages)
                }
                KeyCode::Home => {
                    self.navigation.focus_first(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                KeyCode::End => {
                    self.navigation.focus_last(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageDown => {
                    self.navigation.page_down(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                KeyCode::PageUp => {
                    self.navigation.page_up(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                _ => EventResult::Ignored,
            },
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let messages = self.emit_selection();
                    EventResult::Consumed(messages)
                }
                MouseEventKind::ScrollDown => {
                    self.navigation.focus_next(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                MouseEventKind::ScrollUp => {
                    self.navigation.focus_previous(
                        |idx| self.flat_nodes[idx].node.is_disabled(),
                        self.flat_nodes.len(),
                    );
                    self.selected_index = self.navigation.focused_index();
                    EventResult::Consumed(vec![])
                }
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }

    fn constraints(&self) -> Constraints {
        use unicode_width::UnicodeWidthStr;

        // Calculate max width needed
        let max_width = self
            .flat_nodes
            .iter()
            .map(|flat_node| {
                let indent_width = flat_node.depth * 2;
                let expand_icon_width = if self.show_icons { 2 } else { 0 };
                let icon_width = flat_node
                    .node
                    .icon()
                    .map(|icon| icon.width() + 1)
                    .unwrap_or(0);
                let label_width = flat_node.node.label().width();
                (indent_width + expand_icon_width + icon_width + label_width) as u16
            })
            .max()
            .unwrap_or(self.empty_message.width() as u16);

        let content_height = if self.flat_nodes.is_empty() {
            1
        } else {
            self.flat_nodes.len()
        };

        let needs_scrollbar =
            self.show_scrollbar && content_height > self.navigation.viewport_size();
        let scrollbar_width = if needs_scrollbar { 2 } else { 0 };

        let total_width = max_width + scrollbar_width;
        let display_height = content_height.min(self.navigation.viewport_size()) as u16;

        Constraints {
            min_width: total_width.max(20),
            max_width: Some(total_width.max(20)),
            min_height: display_height.max(3),
            max_height: Some(display_height.max(3)),
            flex: None,
        }
    }

    fn focusable(&self) -> bool {
        self.flat_nodes.iter().any(|n| !n.node.is_disabled())
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;

        if focused && self.selected_index.is_none() && !self.flat_nodes.is_empty() {
            self.navigation.focus_first(
                |idx| self.flat_nodes[idx].node.is_disabled(),
                self.flat_nodes.len(),
            );
            self.selected_index = self.navigation.focused_index();
        }
    }
}

impl<T: TreeNode, M> TreeView<T, M> {
    /// Render scrollbar in the rightmost column
    fn render_scrollbar(
        &self,
        chunk: &mut render::chunk::Chunk,
        x_offset: u16,
        height: u16,
        viewport_size: usize,
        styles: &TreeStyles,
    ) {
        let total_items = self.flat_nodes.len();

        let scrollbar_height = height as usize;
        let thumb_size = ((viewport_size as f64 / total_items as f64) * scrollbar_height as f64)
            .max(1.0)
            .round() as usize;

        let scroll_ratio =
            self.navigation.scroll_offset() as f64 / (total_items - viewport_size).max(1) as f64;
        let thumb_position =
            (scroll_ratio * (scrollbar_height - thumb_size) as f64).round() as usize;

        // Draw scrollbar track
        for y in 0..height {
            let _ = chunk.set_char(x_offset, y, '│', styles.scrollbar_track);
        }

        // Draw scrollbar thumb
        for offset in 0..thumb_size {
            let y = (thumb_position + offset).min(scrollbar_height - 1);
            let _ = chunk.set_char(x_offset, y as u16, '█', styles.scrollbar_thumb);
        }
    }
}

/// Create a new tree view widget (convenience function)
///
/// # Examples
/// ```
/// use tui::prelude::*;
///
/// #[derive(Clone)]
/// enum Message { Selected(String) }
///
/// let tree = tree_view()
///     .on_select(|event| Message::Selected(event.node.label()));
/// ```
pub fn tree_view<T: TreeNode, M>() -> TreeView<T, M> {
    TreeView::empty()
}
