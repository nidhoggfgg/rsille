//! Prelude module for convenient imports
//!
//! Import everything you need with `use tui::prelude::*;`
//!
//! # Available Functions
//!
//! When you import the prelude, you get access to these convenience functions:
//! - `col()` - Create empty vertical containers
//! - `row()` - Create empty horizontal containers
//! - `grid()` - Create grid containers
//! - `label()` - Create label widgets
//! - `button()` - Create button widgets
//! - `checkbox()` - Create checkbox widgets
//! - `checkbox_group()` - Create checkbox group widgets
//! - `radio_group()` - Create radio group widgets
//! - `text_input()` - Create text input widgets
//! - `spacer()` - Create spacer widgets
//! - `divider()` - Create divider widgets (horizontal/vertical separators)
//! - `interactive()` - Wrap widgets with mouse event handling
//!
//! # Available Macros (Legacy)
//!
//! The old macro-based API is still available:
//! - `col![]` - Create vertical containers (macro)
//! - `row![]` - Create horizontal containers (macro)
//! - `label!()` - Create label widgets (macro)
//! - `button!()` - Create button widgets (macro)
//! - `checkbox!()` - Create checkbox widgets (macro)
//! - `text_input!()` - Create text input widgets (macro)
//! - `spacer!()` - Create spacer widgets (macro)
//!
//! # New Declarative UI Macro (`ui!`)
//!
//! The `ui!` macro provides a more concise syntax for building widget trees,
//! reducing type annotation noise in the editor.
//!
//! ## Why use `ui!` macro?
//!
//! The builder pattern (`col().child().child()`) causes editors to display
//! verbose type annotations like `Container<Message>` → `Container<Message>` → ...
//! at every step. The `ui!` macro hides this complexity while providing the
//! same functionality.
//!
//! ## Basic Usage
//!
//! ```rust
//! use tui::prelude::*;
//! use tui::ui; // Import the macro
//!
//! #[derive(Clone)]
//! enum Message {
//!     ButtonClick,
//! }
//!
//! fn view(state: &State) -> Container<Message> {
//!     ui! {
//!         col [gap: 1, padding: (2, 2, 1, 1)] {
//!             label("Title") [fg: Color::Cyan, bold: true],
//!             button("Click me") [on_click: || Message::ButtonClick],
//!         }
//!     }
//! }
//! ```
//!
//! ## Comparison: Builder Pattern vs. `ui!` Macro
//!
//! ### Builder Pattern (Traditional)
//!
//! ```rust
//! # use tui::prelude::*;
//! # #[derive(Clone)] enum Message { Click }
//! # fn example() -> Container<Message> {
//! col()
//!     .padding(Padding::new(2, 2, 1, 1))
//!     .gap(1)
//!     .child(
//!         label("Title")
//!             .fg(Color::Cyan)
//!             .bold()
//!     )
//!     .child(
//!         grid()
//!             .columns("1fr 1fr")
//!             .gap(1)
//!             .child(label("Cell A"))
//!             .child(label("Cell B"))
//!     )
//! # }
//! ```
//!
//! ### `ui!` Macro (Cleaner)
//!
//! ```rust
//! # use tui::prelude::*;
//! # use tui::ui;
//! # #[derive(Clone)] enum Message { Click }
//! # fn example() -> Container<Message> {
//! ui! {
//!     col [padding: (2, 2, 1, 1), gap: 1] {
//!         label("Title") [fg: Color::Cyan, bold: true],
//!         grid [columns: "1fr 1fr", gap: 1] {
//!             label("Cell A"),
//!             label("Cell B"),
//!         },
//!     }
//! }
//! # }
//! ```
//!
//! ## Supported Syntax
//!
//! ### Containers
//!
//! - `col { ... }` - Vertical container
//! - `col [gap: 1, padding: (2, 2, 1, 1)] { ... }` - Container with attributes
//! - `row { ... }` - Horizontal container
//! - `grid [columns: "1fr 1fr", rows: "auto"] { ... }` - Grid container
//!
//! ### Widgets
//!
//! - `label("text")` - Simple label
//! - `label("text") [fg: Color::Yellow, bold: true]` - Styled label
//! - `button("text") [on_click: || Message::Click]` - Button with handler
//! - `checkbox("label", true)` - Checkbox
//! - `text_input(value)` - Text input
//!
//! ### Container Attributes
//!
//! - `gap: u16` - Gap between children
//! - `padding: (top, right, bottom, left)` - Padding as tuple
//! - `padding: Padding::new(...)` - Padding object
//! - `border: BorderStyle::Rounded` - Border style
//! - `align_items: AlignItems::Center` - Cross-axis alignment
//! - `justify_content: JustifyContent::SpaceBetween` - Main-axis alignment
//!
//! ### Grid Attributes
//!
//! - `columns: "1fr 1fr 1fr"` - Column template
//! - `rows: "auto auto"` - Row template
//! - `gap: 1` - Gap between cells
//! - `gap_row: 1`, `gap_column: 1` - Specific gap values
//!
//! ### Widget Attributes
//!
//! - `fg: Color` - Foreground color
//! - `bg: Color` - Background color
//! - `bold: true` - Bold text
//! - `italic: true` - Italic text
//! - `underline: true` - Underlined text
//!
//! ### Button Attributes
//!
//! - `on_click: || Message::Click` - Click handler
//! - `variant: ButtonVariant::Primary` - Button variant
//!
//! ## Important Note
//!
//! Children within containers **must be comma-separated**:
//!
//! ```rust
//! # use tui::prelude::*;
//! # use tui::ui;
//! # #[derive(Clone)] enum Message {}
//! # fn example() -> Container<Message> {
//! ui! {
//!     col {
//!         label("First"),   // <-- comma required
//!         label("Second"),  // <-- comma required
//!         label("Third"),   // <-- trailing comma optional
//!     }
//! }
//! # }
//! ```
//!
//! ## Examples
//!
//! See `examples/grid_ui_macro.rs` for a complete example comparing both styles.
//!
//! ## Backward Compatibility
//!
//! The builder pattern is still fully supported and works exactly as before.
//! You can mix both styles in the same project, or gradually migrate to the
//! `ui!` macro as you prefer.

pub use crate::app::App;
pub use crate::error::{WidgetError, WidgetResult};
pub use crate::event::{
    Event, EventResult, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use crate::layout::{
    // fns
    col,
    grid,
    row,

    // types
    AlignItems,
    Constraints,
    Direction,
    Flex,
    Grid,
    GridLine,
    GridPlacement,
    GridTrack,
    JustifyContent,
    Layout,
};
pub use crate::style::{BorderStyle, Color, CssError, Padding, Style, TextModifiers, ThemeManager};
pub use crate::widget::{
    // fns
    alert,
    button,
    checkbox,
    checkbox_group,
    code_block,
    confirm,
    dialog,
    divider,
    enhanced,
    label,
    list,
    progress_bar,
    radio_group,
    spacer,
    table,
    text_input,
    textarea,
    tree_view,

    // types
    Button,
    ButtonVariant,
    Checkbox,
    CheckboxDirection,
    CheckboxGroup,
    CodeBlock,
    Column,
    ColumnWidth,
    Dialog,
    DialogMessage,
    DialogSize,
    Divider,
    DividerDirection,
    DividerTextPosition,
    DividerVariant,
    Enhanced,
    IntoWidget,
    Label,
    LabelPosition,
    LineMarker,
    List,
    ListItem,
    ProgressBar,
    ProgressMode,
    RadioDirection,
    RadioGroup,
    SelectionEvent,
    SelectionMode,
    SimpleTreeNode,
    SimpleWidget,
    Spacer,
    Table,
    TableSelectionEvent,
    TableVariant,
    TextInput,
    TextInputVariant,
    Textarea,
    TextareaVariant,
    TreeExpandEvent,
    TreeNode,
    TreeSelectionEvent,
    TreeView,
    Widget,
};

// The ui! macro is automatically available due to #[macro_export]
// Import it with: use tui::ui;
