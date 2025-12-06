//! TreeView Component Example
//!
//! Demonstrates:
//! - Hierarchical data display with expand/collapse
//! - Keyboard navigation (Up/Down/Left/Right arrows)
//! - Custom TreeNode implementation
//! - SimpleTreeNode for quick tree building
//! - Icons and visual indicators
//! - Selection handling
//! - **IMPORTANT**: Proper state management to preserve expand/collapse state
//!
//! Controls:
//! - Up/Down: Navigate through nodes
//! - Left: Collapse or move to parent
//! - Right: Expand or move to first child
//! - Enter/Space: Toggle expand/collapse and select node
//! - Home/End: Jump to first/last node
//! - PageUp/PageDown: Jump by page
//! - Q/Esc: Quit
//!
//! Run with: cargo run --example tree_view

use tui::prelude::*;

// Example 1: File system tree using custom TreeNode
#[derive(Clone, Debug)]
struct FileNode {
    name: String,
    is_dir: bool,
    children: Vec<FileNode>,
}

impl FileNode {
    fn dir(name: &str, children: Vec<FileNode>) -> Self {
        Self {
            name: name.to_string(),
            is_dir: true,
            children,
        }
    }

    fn file(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_dir: false,
            children: Vec::new(),
        }
    }
}

impl TreeNode for FileNode {
    fn label(&self) -> String {
        self.name.clone()
    }

    fn children(&self) -> Vec<Self> {
        self.children.clone()
    }

    fn icon(&self) -> Option<&str> {
        if self.is_dir {
            Some("📁")
        } else {
            Some("📄")
        }
    }
}

/// Application state
#[derive(Debug)]
struct State {
    selected_info: String,
    // Store tree data (not TreeView instances)
    file_tree_data: Vec<FileNode>,
    animal_tree_data: Vec<SimpleTreeNode<&'static str>>,
    json_tree_data: Vec<SimpleTreeNode<&'static str>>,
    // Store expand/collapse state externally
    file_expanded: std::collections::HashSet<Vec<usize>>,
    animal_expanded: std::collections::HashSet<Vec<usize>>,
    json_expanded: std::collections::HashSet<Vec<usize>>,
    // Store selection state externally
    file_selected: Option<usize>,
    animal_selected: Option<usize>,
    json_selected: Option<usize>,
}

/// Messages from UI interactions
#[derive(Clone, Debug)]
enum Message {
    FileSelected(TreeSelectionEvent<FileNode>),
    AnimalSelected(TreeSelectionEvent<SimpleTreeNode<&'static str>>),
    JsonSelected(TreeSelectionEvent<SimpleTreeNode<&'static str>>),
    FileExpanded(TreeExpandEvent<FileNode>),
    AnimalExpanded(TreeExpandEvent<SimpleTreeNode<&'static str>>),
    JsonExpanded(TreeExpandEvent<SimpleTreeNode<&'static str>>),
    Quit,
}

/// Update function - handles messages
fn update(state: &mut State, msg: Message) {
    match msg {
        Message::FileSelected(event) => {
            state.selected_info = format!("File: {}", event.node.label());
            // Persist state
            state.file_expanded = event.expanded_paths;
            state.file_selected = event.selected_index;
        }
        Message::AnimalSelected(event) => {
            state.selected_info = format!("Animal/Plant: {}", event.node.label());
            // Persist state
            state.animal_expanded = event.expanded_paths;
            state.animal_selected = event.selected_index;
        }
        Message::JsonSelected(event) => {
            state.selected_info = format!("JSON key: {}", event.node.label());
            // Persist state
            state.json_expanded = event.expanded_paths;
            state.json_selected = event.selected_index;
        }
        Message::FileExpanded(event) => {
            let action = if event.expanded {
                "Expanded"
            } else {
                "Collapsed"
            };
            state.selected_info = format!("{}: {}", action, event.node.label());
            // Persist state
            state.file_expanded = event.expanded_paths;
            state.file_selected = event.selected_index;
        }
        Message::AnimalExpanded(event) => {
            let action = if event.expanded {
                "Expanded"
            } else {
                "Collapsed"
            };
            state.selected_info = format!("{}: {}", action, event.node.label());
            // Persist state
            state.animal_expanded = event.expanded_paths;
            state.animal_selected = event.selected_index;
        }
        Message::JsonExpanded(event) => {
            let action = if event.expanded {
                "Expanded"
            } else {
                "Collapsed"
            };
            state.selected_info = format!("{}: {}", action, event.node.label());
            // Persist state
            state.json_expanded = event.expanded_paths;
            state.json_selected = event.selected_index;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

/// View function - builds the UI
fn view(state: &State) -> impl Layout<Message> {
    row()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(2)
        // File system column
        .child(
            col()
                .gap(1)
                .child(label("File System").fg(Color::Cyan).bold())
                .child(
                    TreeView::new(state.file_tree_data.clone())
                        .viewport_height(15)
                        .expanded_paths(state.file_expanded.clone())
                        .selected_index(state.file_selected)
                        .on_select(|event| Message::FileSelected(event))
                        .on_expand(|event| Message::FileExpanded(event)),
                ),
        )
        // Animals/Plants column
        .child(
            col()
                .gap(1)
                .child(label("Animals & Plants").fg(Color::Cyan).bold())
                .child(
                    TreeView::new(state.animal_tree_data.clone())
                        .viewport_height(15)
                        .show_icons(false) // Hide expand icons to use emoji icons
                        .expanded_paths(state.animal_expanded.clone())
                        .selected_index(state.animal_selected)
                        .on_select(|event| Message::AnimalSelected(event))
                        .on_expand(|event| Message::AnimalExpanded(event)),
                ),
        )
        // JSON column
        .child(
            col()
                .gap(1)
                .child(label("JSON Structure").fg(Color::Cyan).bold())
                .child(
                    TreeView::new(state.json_tree_data.clone())
                        .viewport_height(15)
                        .expanded_paths(state.json_expanded.clone())
                        .selected_index(state.json_selected)
                        .on_select(|event| Message::JsonSelected(event))
                        .on_expand(|event| Message::JsonExpanded(event)),
                ),
        )
}

fn main() -> WidgetResult<()> {
    use std::collections::HashSet;

    // File system tree
    let file_tree_data = vec![
        FileNode::dir(
            "src",
            vec![
                FileNode::file("main.rs"),
                FileNode::file("lib.rs"),
                FileNode::dir(
                    "widget",
                    vec![
                        FileNode::file("mod.rs"),
                        FileNode::file("button.rs"),
                        FileNode::file("list.rs"),
                        FileNode::file("tree.rs"),
                    ],
                ),
            ],
        ),
        FileNode::dir("examples", vec![FileNode::file("tree_view.rs")]),
        FileNode::file("Cargo.toml"),
        FileNode::file("README.md"),
    ];

    // Simple tree with SimpleTreeNode
    let simple_tree_data = vec![
        SimpleTreeNode::branch(
            "animals",
            "Animals",
            vec![
                SimpleTreeNode::branch(
                    "mammals",
                    "🐾 Mammals",
                    vec![
                        SimpleTreeNode::leaf("dog", "🐕 Dog"),
                        SimpleTreeNode::leaf("cat", "🐈 Cat"),
                        SimpleTreeNode::leaf("elephant", "🐘 Elephant"),
                    ],
                ),
                SimpleTreeNode::branch(
                    "birds",
                    "🦅 Birds",
                    vec![
                        SimpleTreeNode::leaf("eagle", "🦅 Eagle"),
                        SimpleTreeNode::leaf("parrot", "🦜 Parrot"),
                    ],
                ),
            ],
        ),
        SimpleTreeNode::branch(
            "plants",
            "Plants",
            vec![
                SimpleTreeNode::leaf("rose", "🌹 Rose"),
                SimpleTreeNode::leaf("tulip", "🌷 Tulip"),
            ],
        ),
    ];

    // JSON-like structure tree
    let json_tree_data = vec![SimpleTreeNode::branch(
        "config",
        "config.json",
        vec![
            SimpleTreeNode::leaf("name", "name: \"MyApp\""),
            SimpleTreeNode::leaf("version", "version: \"1.0.0\""),
            SimpleTreeNode::branch(
                "settings",
                "settings: {...}",
                vec![
                    SimpleTreeNode::leaf("theme", "theme: \"dark\""),
                    SimpleTreeNode::leaf("language", "language: \"en\""),
                ],
            ),
        ],
    )];

    // Initialize with all nodes expanded
    let mut file_expanded = HashSet::new();
    file_expanded.insert(vec![0]); // src
    file_expanded.insert(vec![0, 2]); // src/widget
    file_expanded.insert(vec![1]); // examples

    let mut animal_expanded = HashSet::new();
    animal_expanded.insert(vec![0]); // animals
    animal_expanded.insert(vec![0, 0]); // mammals
    animal_expanded.insert(vec![0, 1]); // birds
    animal_expanded.insert(vec![1]); // plants

    let mut json_expanded = HashSet::new();
    json_expanded.insert(vec![0]); // config
    json_expanded.insert(vec![0, 2]); // settings

    let app = App::new(State {
        selected_info: "No selection".to_string(),
        file_tree_data,
        animal_tree_data: simple_tree_data,
        json_tree_data,
        file_expanded,
        animal_expanded,
        json_expanded,
        file_selected: Some(0),
        animal_selected: Some(0),
        json_selected: Some(0),
    });

    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .on_key(KeyCode::Esc, || Message::Quit)
        .run_inline(update, view)?;

    Ok(())
}
