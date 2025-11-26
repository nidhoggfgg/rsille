//! Declarative UI macro for simplified widget tree construction
//!
//! This module provides the `ui!` macro that offers a more concise syntax
//! for building UI trees, reducing type annotation noise in the editor.
//!
//! # Examples
//!
//! ```rust
//! use tui::prelude::*;
//!
//! fn view() -> impl Layout<Message> {
//!     ui! {
//!         col [gap: 1, padding: (2, 2, 1, 1)] {
//!             label("Title") [fg: Cyan, bold: true],
//!             grid [columns: "1fr 1fr", gap: 1] {
//!                 label("Cell A"),
//!                 label("Cell B"),
//!             },
//!         }
//!     }
//! }
//! ```

/// Declarative UI macro for building widget trees with less type annotation noise
///
/// # Syntax
///
/// Children within containers must be comma-separated.
///
/// ## Containers
///
/// - `col { ... }` - Vertical flex layout
/// - `col [attr: value, ...] { ... }` - Flex layout with attributes
/// - `row { ... }` - Horizontal flex layout
/// - `grid [columns: "...", rows: "..."] { ... }` - Grid layout
///
/// ## Widgets
///
/// - `label("text")` - Simple label
/// - `label("text") [fg: Color, bg: Color, bold: true]` - Styled label
/// - `button("text") [on_click: || Message::Click]` - Button with handler
///
/// ## Attributes
///
/// ### Flex layout attributes:
/// - `gap: u16` - Gap between children
/// - `padding: (u16, u16, u16, u16)` - Padding (top, right, bottom, left)
/// - `border: BorderStyle` - Border style
/// - `align_items: AlignItems` - Cross-axis alignment
/// - `justify_content: JustifyContent` - Main-axis alignment
///
/// ### Grid attributes:
/// - `columns: "..."` - Column template
/// - `rows: "..."` - Row template
/// - `gap: u16` - Gap between cells
///
/// ### Widget attributes:
/// - `fg: Color` - Foreground color
/// - `bg: Color` - Background color
/// - `bold: true` - Bold text
/// - `italic: true` - Italic text
/// - `underline: true` - Underlined text
/// - `on_click: || Message` - Click handler (button)
///
#[macro_export]
macro_rules! ui {
    // ==================== Entry Point ====================
    // Only match non-@ tokens to avoid interfering with internal rules
    (col $($rest:tt)*) => {
        ui!(@element col $($rest)*)
    };

    (row $($rest:tt)*) => {
        ui!(@element row $($rest)*)
    };

    (grid $($rest:tt)*) => {
        ui!(@element grid $($rest)*)
    };

    (label($($args:tt)*)) => {
        ui!(@element label($($args)*))
    };

    (button($($args:tt)*)) => {
        ui!(@element button($($args)*))
    };

    (checkbox($($args:tt)*)) => {
        ui!(@element checkbox($($args)*))
    };

    (text_input($($args:tt)*)) => {
        ui!(@element text_input($($args)*))
    };

    // ==================== Container: col ====================
    (@element col [$($attrs:tt)*] { $($children:tt)* }) => {{
        let layout = $crate::layout::col();
        let layout = ui!(@apply_layout_attrs layout, $($attrs)*);
        ui!(@add_children layout; $($children)*)
    }};

    (@element col { $($children:tt)* }) => {{
        let layout = $crate::layout::col();
        ui!(@add_children layout; $($children)*)
    }};

    // ==================== Container: row ====================
    (@element row [$($attrs:tt)*] { $($children:tt)* }) => {{
        let layout = $crate::layout::row();
        let layout = ui!(@apply_layout_attrs layout, $($attrs)*);
        ui!(@add_children layout; $($children)*)
    }};

    (@element row { $($children:tt)* }) => {{
        let layout = $crate::layout::row();
        ui!(@add_children layout; $($children)*)
    }};

    // ==================== Container: grid ====================
    (@element grid [$($attrs:tt)*] { $($children:tt)* }) => {{
        let grid = $crate::layout::grid();
        let grid = ui!(@apply_grid_attrs grid, $($attrs)*);
        ui!(@add_children grid; $($children)*)
    }};

    (@element grid { $($children:tt)* }) => {{
        let grid = $crate::layout::grid();
        ui!(@add_children grid; $($children)*)
    }};

    // ==================== Widget: label ====================
    (@element label($text:expr) [$($attrs:tt)*]) => {{
        let widget = $crate::widget::label($text);
        ui!(@apply_widget_attrs widget, $($attrs)*)
    }};

    (@element label($text:expr)) => {
        $crate::widget::label($text)
    };

    // ==================== Widget: button ====================
    (@element button($text:expr) [$($attrs:tt)*]) => {{
        let widget = $crate::widget::button($text);
        ui!(@apply_button_attrs widget, $($attrs)*)
    }};

    (@element button($text:expr)) => {
        $crate::widget::button($text)
    };

    // ==================== Widget: checkbox ====================
    (@element checkbox($label:expr, $checked:expr) [$($attrs:tt)*]) => {{
        let widget = $crate::widget::checkbox($label, $checked);
        ui!(@apply_checkbox_attrs widget, $($attrs)*)
    }};

    (@element checkbox($label:expr, $checked:expr)) => {
        $crate::widget::checkbox($label, $checked)
    };

    // ==================== Widget: text_input ====================
    (@element text_input($value:expr) [$($attrs:tt)*]) => {{
        let widget = $crate::widget::text_input($value);
        ui!(@apply_text_input_attrs widget, $($attrs)*)
    }};

    (@element text_input($value:expr)) => {
        $crate::widget::text_input($value)
    };

    // ==================== Add Children (Comma-separated) ====================
    // Base case: no children
    (@add_children $layout:expr;) => {
        $layout
    };

    // Trailing comma case
    (@add_children $layout:expr; ,) => {
        $layout
    };

    // ===== Container elements =====

    // col with attributes and children, followed by comma
    (@add_children $layout:expr; col [$($attrs:tt)*] { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element col [$($attrs)*] { $($children)* }));
            $($rest)*
        )
    };

    // col without attributes, followed by comma
    (@add_children $layout:expr; col { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element col { $($children)* }));
            $($rest)*
        )
    };

    // row with attributes and children, followed by comma
    (@add_children $layout:expr; row [$($attrs:tt)*] { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element row [$($attrs)*] { $($children)* }));
            $($rest)*
        )
    };

    // row without attributes, followed by comma
    (@add_children $layout:expr; row { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element row { $($children)* }));
            $($rest)*
        )
    };

    // grid with attributes and children, followed by comma
    (@add_children $layout:expr; grid [$($attrs:tt)*] { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element grid [$($attrs)*] { $($children)* }));
            $($rest)*
        )
    };

    // grid without attributes, followed by comma
    (@add_children $layout:expr; grid { $($children:tt)* }, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element grid { $($children)* }));
            $($rest)*
        )
    };

    // ===== Widget elements =====

    // label with attributes, followed by comma
    (@add_children $layout:expr; label($text:expr) [$($attrs:tt)*], $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element label($text) [$($attrs)*]));
            $($rest)*
        )
    };

    // label without attributes, followed by comma
    (@add_children $layout:expr; label($text:expr), $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element label($text)));
            $($rest)*
        )
    };

    // button with attributes, followed by comma
    (@add_children $layout:expr; button($text:expr) [$($attrs:tt)*], $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element button($text) [$($attrs)*]));
            $($rest)*
        )
    };

    // button without attributes, followed by comma
    (@add_children $layout:expr; button($text:expr), $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element button($text)));
            $($rest)*
        )
    };

    // checkbox with attributes, followed by comma
    (@add_children $layout:expr; checkbox($label:expr, $checked:expr) [$($attrs:tt)*], $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element checkbox($label, $checked) [$($attrs)*]));
            $($rest)*
        )
    };

    // checkbox without attributes, followed by comma
    (@add_children $layout:expr; checkbox($label:expr, $checked:expr), $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element checkbox($label, $checked)));
            $($rest)*
        )
    };

    // text_input with attributes, followed by comma
    (@add_children $layout:expr; text_input($value:expr) [$($attrs:tt)*], $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element text_input($value) [$($attrs)*]));
            $($rest)*
        )
    };

    // text_input without attributes, followed by comma
    (@add_children $layout:expr; text_input($value:expr), $($rest:tt)*) => {
        ui!(@add_children
            $layout.child(ui!(@element text_input($value)));
            $($rest)*
        )
    };

    // Fallback: any expression (for things like keyboard_controller().on(...)), followed by comma
    (@add_children $layout:expr; $expr:expr, $($rest:tt)*) => {
        ui!(@add_children
            $layout.child($expr);
            $($rest)*
        )
    };

    // ===== Last child without trailing comma =====

    // col with attributes (last child)
    (@add_children $layout:expr; col [$($attrs:tt)*] { $($children:tt)* }) => {
        $layout.child(ui!(@element col [$($attrs)*] { $($children)* }))
    };

    // col without attributes (last child)
    (@add_children $layout:expr; col { $($children:tt)* }) => {
        $layout.child(ui!(@element col { $($children)* }))
    };

    // row with attributes (last child)
    (@add_children $layout:expr; row [$($attrs:tt)*] { $($children:tt)* }) => {
        $layout.child(ui!(@element row [$($attrs)*] { $($children)* }))
    };

    // row without attributes (last child)
    (@add_children $layout:expr; row { $($children:tt)* }) => {
        $layout.child(ui!(@element row { $($children)* }))
    };

    // grid with attributes (last child)
    (@add_children $layout:expr; grid [$($attrs:tt)*] { $($children:tt)* }) => {
        $layout.child(ui!(@element grid [$($attrs)*] { $($children)* }))
    };

    // grid without attributes (last child)
    (@add_children $layout:expr; grid { $($children:tt)* }) => {
        $layout.child(ui!(@element grid { $($children)* }))
    };

    // label with attributes (last child)
    (@add_children $layout:expr; label($text:expr) [$($attrs:tt)*]) => {
        $layout.child(ui!(@element label($text) [$($attrs)*]))
    };

    // label without attributes (last child)
    (@add_children $layout:expr; label($text:expr)) => {
        $layout.child(ui!(@element label($text)))
    };

    // button with attributes (last child)
    (@add_children $layout:expr; button($text:expr) [$($attrs:tt)*]) => {
        $layout.child(ui!(@element button($text) [$($attrs)*]))
    };

    // button without attributes (last child)
    (@add_children $layout:expr; button($text:expr)) => {
        $layout.child(ui!(@element button($text)))
    };

    // checkbox with attributes (last child)
    (@add_children $layout:expr; checkbox($label:expr, $checked:expr) [$($attrs:tt)*]) => {
        $layout.child(ui!(@element checkbox($label, $checked) [$($attrs)*]))
    };

    // checkbox without attributes (last child)
    (@add_children $layout:expr; checkbox($label:expr, $checked:expr)) => {
        $layout.child(ui!(@element checkbox($label, $checked)))
    };

    // text_input with attributes (last child)
    (@add_children $layout:expr; text_input($value:expr) [$($attrs:tt)*]) => {
        $layout.child(ui!(@element text_input($value) [$($attrs)*]))
    };

    // text_input without attributes (last child)
    (@add_children $layout:expr; text_input($value:expr)) => {
        $layout.child(ui!(@element text_input($value)))
    };

    // Fallback: any expression (last child)
    (@add_children $layout:expr; $expr:expr) => {
        $layout.child($expr)
    };

    // ==================== Container Attributes ====================
    // gap
    (@apply_layout_attrs $layout:expr, gap: $gap:expr) => {
        $layout.gap($gap)
    };
    (@apply_layout_attrs $layout:expr, gap: $gap:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.gap($gap), $($rest)*)
    };

    // padding - tuple syntax
    (@apply_layout_attrs $layout:expr, padding: ($top:expr, $right:expr, $bottom:expr, $left:expr)) => {
        $layout.padding($crate::style::Padding::new($top, $right, $bottom, $left))
    };
    (@apply_layout_attrs $layout:expr, padding: ($top:expr, $right:expr, $bottom:expr, $left:expr), $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.padding($crate::style::Padding::new($top, $right, $bottom, $left)), $($rest)*)
    };

    // padding - Padding value
    (@apply_layout_attrs $layout:expr, padding: $padding:expr) => {
        $layout.padding($padding)
    };
    (@apply_layout_attrs $layout:expr, padding: $padding:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.padding($padding), $($rest)*)
    };

    // border
    (@apply_layout_attrs $layout:expr, border: $border:expr) => {
        $layout.border($border)
    };
    (@apply_layout_attrs $layout:expr, border: $border:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.border($border), $($rest)*)
    };

    // align_items
    (@apply_layout_attrs $layout:expr, align_items: $align:expr) => {
        $layout.align_items($align)
    };
    (@apply_layout_attrs $layout:expr, align_items: $align:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.align_items($align), $($rest)*)
    };

    // justify_content
    (@apply_layout_attrs $layout:expr, justify_content: $justify:expr) => {
        $layout.justify_content($justify)
    };
    (@apply_layout_attrs $layout:expr, justify_content: $justify:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.justify_content($justify), $($rest)*)
    };

    // style
    (@apply_layout_attrs $layout:expr, style: $style:expr) => {
        $layout.style($style)
    };
    (@apply_layout_attrs $layout:expr, style: $style:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.style($style), $($rest)*)
    };

    // overflow
    (@apply_layout_attrs $layout:expr, overflow: $overflow:expr) => {
        $layout.overflow($overflow)
    };
    (@apply_layout_attrs $layout:expr, overflow: $overflow:expr, $($rest:tt)*) => {
        ui!(@apply_layout_attrs $layout.overflow($overflow), $($rest)*)
    };

    // No more attributes
    (@apply_layout_attrs $layout:expr,) => {
        $layout
    };

    // ==================== Grid Attributes ====================
    // columns
    (@apply_grid_attrs $grid:expr, columns: $cols:expr) => {
        $grid.columns($cols)
    };
    (@apply_grid_attrs $grid:expr, columns: $cols:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.columns($cols), $($rest)*)
    };

    // rows
    (@apply_grid_attrs $grid:expr, rows: $rows:expr) => {
        $grid.rows($rows)
    };
    (@apply_grid_attrs $grid:expr, rows: $rows:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.rows($rows), $($rest)*)
    };

    // gap
    (@apply_grid_attrs $grid:expr, gap: $gap:expr) => {
        $grid.gap($gap)
    };
    (@apply_grid_attrs $grid:expr, gap: $gap:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.gap($gap), $($rest)*)
    };

    // gap_row
    (@apply_grid_attrs $grid:expr, gap_row: $gap:expr) => {
        $grid.gap_row($gap)
    };
    (@apply_grid_attrs $grid:expr, gap_row: $gap:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.gap_row($gap), $($rest)*)
    };

    // gap_column
    (@apply_grid_attrs $grid:expr, gap_column: $gap:expr) => {
        $grid.gap_column($gap)
    };
    (@apply_grid_attrs $grid:expr, gap_column: $gap:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.gap_column($gap), $($rest)*)
    };

    // padding - tuple syntax
    (@apply_grid_attrs $grid:expr, padding: ($top:expr, $right:expr, $bottom:expr, $left:expr)) => {
        $grid.padding($crate::style::Padding::new($top, $right, $bottom, $left))
    };
    (@apply_grid_attrs $grid:expr, padding: ($top:expr, $right:expr, $bottom:expr, $left:expr), $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.padding($crate::style::Padding::new($top, $right, $bottom, $left)), $($rest)*)
    };

    // padding - Padding value
    (@apply_grid_attrs $grid:expr, padding: $padding:expr) => {
        $grid.padding($padding)
    };
    (@apply_grid_attrs $grid:expr, padding: $padding:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.padding($padding), $($rest)*)
    };

    // border
    (@apply_grid_attrs $grid:expr, border: $border:expr) => {
        $grid.border($border)
    };
    (@apply_grid_attrs $grid:expr, border: $border:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.border($border), $($rest)*)
    };

    // style
    (@apply_grid_attrs $grid:expr, style: $style:expr) => {
        $grid.style($style)
    };
    (@apply_grid_attrs $grid:expr, style: $style:expr, $($rest:tt)*) => {
        ui!(@apply_grid_attrs $grid.style($style), $($rest)*)
    };

    // No more attributes
    (@apply_grid_attrs $grid:expr,) => {
        $grid
    };

    // ==================== Widget Attributes (Label) ====================
    // fg (foreground color)
    (@apply_widget_attrs $widget:expr, fg: $color:expr) => {
        $widget.fg($color)
    };
    (@apply_widget_attrs $widget:expr, fg: $color:expr, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget.fg($color), $($rest)*)
    };

    // bg (background color)
    (@apply_widget_attrs $widget:expr, bg: $color:expr) => {
        $widget.bg($color)
    };
    (@apply_widget_attrs $widget:expr, bg: $color:expr, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget.bg($color), $($rest)*)
    };

    // bold
    (@apply_widget_attrs $widget:expr, bold: true) => {
        $widget.bold()
    };
    (@apply_widget_attrs $widget:expr, bold: true, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget.bold(), $($rest)*)
    };
    (@apply_widget_attrs $widget:expr, bold: false) => {
        $widget
    };
    (@apply_widget_attrs $widget:expr, bold: false, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget, $($rest)*)
    };

    // italic
    (@apply_widget_attrs $widget:expr, italic: true) => {
        $widget.italic()
    };
    (@apply_widget_attrs $widget:expr, italic: true, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget.italic(), $($rest)*)
    };
    (@apply_widget_attrs $widget:expr, italic: false) => {
        $widget
    };
    (@apply_widget_attrs $widget:expr, italic: false, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget, $($rest)*)
    };

    // underline
    (@apply_widget_attrs $widget:expr, underline: true) => {
        $widget.underline()
    };
    (@apply_widget_attrs $widget:expr, underline: true, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget.underline(), $($rest)*)
    };
    (@apply_widget_attrs $widget:expr, underline: false) => {
        $widget
    };
    (@apply_widget_attrs $widget:expr, underline: false, $($rest:tt)*) => {
        ui!(@apply_widget_attrs $widget, $($rest)*)
    };

    // No more attributes
    (@apply_widget_attrs $widget:expr,) => {
        $widget
    };

    // ==================== Button Attributes ====================
    // on_click handler
    (@apply_button_attrs $button:expr, on_click: $handler:expr) => {
        $button.on_click($handler)
    };
    (@apply_button_attrs $button:expr, on_click: $handler:expr, $($rest:tt)*) => {
        ui!(@apply_button_attrs $button.on_click($handler), $($rest)*)
    };

    // variant
    (@apply_button_attrs $button:expr, variant: $variant:expr) => {
        $button.variant($variant)
    };
    (@apply_button_attrs $button:expr, variant: $variant:expr, $($rest:tt)*) => {
        ui!(@apply_button_attrs $button.variant($variant), $($rest)*)
    };

    // fg, bg (reuse widget attrs)
    (@apply_button_attrs $button:expr, fg: $color:expr, $($rest:tt)*) => {
        ui!(@apply_button_attrs $button.fg($color), $($rest)*)
    };
    (@apply_button_attrs $button:expr, bg: $color:expr, $($rest:tt)*) => {
        ui!(@apply_button_attrs $button.bg($color), $($rest)*)
    };

    // No more attributes
    (@apply_button_attrs $button:expr,) => {
        $button
    };

    // ==================== Checkbox Attributes ====================
    // on_change handler
    (@apply_checkbox_attrs $checkbox:expr, on_change: $handler:expr) => {
        $checkbox.on_change($handler)
    };
    (@apply_checkbox_attrs $checkbox:expr, on_change: $handler:expr, $($rest:tt)*) => {
        ui!(@apply_checkbox_attrs $checkbox.on_change($handler), $($rest)*)
    };

    // No more attributes
    (@apply_checkbox_attrs $checkbox:expr,) => {
        $checkbox
    };

    // ==================== TextInput Attributes ====================
    // on_change handler
    (@apply_text_input_attrs $input:expr, on_change: $handler:expr) => {
        $input.on_change($handler)
    };
    (@apply_text_input_attrs $input:expr, on_change: $handler:expr, $($rest:tt)*) => {
        ui!(@apply_text_input_attrs $input.on_change($handler), $($rest)*)
    };

    // placeholder
    (@apply_text_input_attrs $input:expr, placeholder: $placeholder:expr) => {
        $input.placeholder($placeholder)
    };
    (@apply_text_input_attrs $input:expr, placeholder: $placeholder:expr, $($rest:tt)*) => {
        ui!(@apply_text_input_attrs $input.placeholder($placeholder), $($rest)*)
    };

    // variant
    (@apply_text_input_attrs $input:expr, variant: $variant:expr) => {
        $input.variant($variant)
    };
    (@apply_text_input_attrs $input:expr, variant: $variant:expr, $($rest:tt)*) => {
        ui!(@apply_text_input_attrs $input.variant($variant), $($rest)*)
    };

    // No more attributes
    (@apply_text_input_attrs $input:expr,) => {
        $input
    };
}
