//! Table widget variants example
//!
//! Demonstrates different table variants and border styles:
//! - Simple: Default table with column dividers
//! - Bordered: Table with full borders
//! - Compact: Table without horizontal dividers
//! - Striped: Table with alternating row backgrounds
//! - Static: Display-only table with no selection
//!
//! Controls:
//! - V: Cycle through table variants
//! - B: Cycle through border styles (for Bordered variant)
//! - Q/Esc: Quit
//!
//! Run with: cargo run --example table_variants

use tui::prelude::*;

#[derive(Clone, Debug)]
struct Product {
    id: u32,
    name: String,
    category: String,
    price: f64,
    stock: u32,
}

impl Product {
    fn new(id: u32, name: &str, category: &str, price: f64, stock: u32) -> Self {
        Self {
            id,
            name: name.to_string(),
            category: category.to_string(),
            price,
            stock,
        }
    }
}

#[derive(Debug)]
struct State {
    variant_index: usize,
    border_index: usize,
}

#[derive(Clone, Debug)]
enum Message {
    CycleVariant,
    CycleBorder,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::CycleVariant => {
            state.variant_index = (state.variant_index + 1) % 5;
        }
        Message::CycleBorder => {
            state.border_index = (state.border_index + 1) % 4;
        }
        Message::Quit => {
            std::process::exit(0);
        }
    }
}

fn get_products() -> Vec<Product> {
    vec![
        Product::new(1, "Laptop", "Electronics", 999.99, 15),
        Product::new(2, "Mouse", "Accessories", 29.99, 50),
        Product::new(3, "Keyboard", "Accessories", 79.99, 30),
        Product::new(4, "Monitor", "Electronics", 299.99, 20),
        Product::new(5, "Desk Chair", "Furniture", 199.99, 10),
        Product::new(6, "USB Cable", "Accessories", 9.99, 100),
        Product::new(7, "Headphones", "Audio", 149.99, 25),
        Product::new(8, "Webcam", "Electronics", 89.99, 40),
    ]
}

fn get_columns() -> Vec<Column<Product>> {
    vec![
        Column::new("ID", |p: &Product| p.id.to_string()).width(ColumnWidth::Fixed(5)),
        Column::new("Product", |p: &Product| p.name.clone()).width(ColumnWidth::Flex(3)),
        Column::new("Category", |p: &Product| p.category.clone()).width(ColumnWidth::Flex(2)),
        Column::new("Price", |p: &Product| format!("${:.2}", p.price))
            .width(ColumnWidth::Fixed(10)),
        Column::new("Stock", |p: &Product| p.stock.to_string()).width(ColumnWidth::Fixed(7)),
    ]
}

fn get_variant(index: usize) -> (TableVariant, &'static str) {
    match index {
        0 => (TableVariant::Simple, "Simple"),
        1 => (TableVariant::Bordered, "Bordered"),
        2 => (TableVariant::Compact, "Compact"),
        3 => (TableVariant::Striped, "Striped"),
        4 => (TableVariant::Simple, "Static (display-only)"),
        _ => (TableVariant::Simple, "Simple"),
    }
}

fn get_border(index: usize) -> (BorderStyle, &'static str) {
    match index {
        0 => (BorderStyle::Single, "Single"),
        1 => (BorderStyle::Double, "Double"),
        2 => (BorderStyle::Rounded, "Rounded"),
        3 => (BorderStyle::Thick, "Thick"),
        _ => (BorderStyle::Single, "Single"),
    }
}

fn view(state: &State) -> impl Layout<Message> {
    let (variant, variant_name) = get_variant(state.variant_index);
    let (border, border_name) = get_border(state.border_index);
    let is_static = state.variant_index == 4;

    // Create the appropriate table based on variant
    let table_widget = if is_static {
        // Static table - no selection
        Table::<Product, Message>::static_table(get_columns())
            .rows(get_products())
            .viewport_height(10)
            .variant(variant)
            .show_header(true)
    } else {
        // Interactive table
        let mut table = table(get_columns())
            .rows(get_products())
            .selection_mode(SelectionMode::Single)
            .viewport_height(10)
            .variant(variant)
            .show_header(true);

        // Apply border style for bordered variant
        if variant == TableVariant::Bordered {
            table = table.border(border);
        }

        table
    };

    col()
        .padding(Padding::new(2, 2, 1, 1))
        .gap(1)
        // Title
        .child(label("Table Variants Demo").fg(Color::Cyan).bold())
        .child(spacer().height(1))
        // Info
        .child(label(format!("Current Variant: {}", variant_name)).fg(Color::Yellow))
        .child(if variant == TableVariant::Bordered {
            label(format!("Border Style: {}", border_name)).fg(Color::Green)
        } else {
            label("")
        })
        .child(spacer().height(1))
        // Table
        .child(table_widget)
        .child(spacer().height(1))
        // Instructions
        .child(
            col()
                .gap(0)
                .child(label("Controls:").fg(Color::Indexed(8)))
                .child(label("  V: Cycle table variant").fg(Color::Indexed(8)))
                .child(
                    label("  B: Cycle border style (Bordered variant only)").fg(Color::Indexed(8)),
                )
                .child(label("  Q or Esc: Quit").fg(Color::Indexed(8))),
        )
        .child(spacer().height(1))
        // Variant descriptions
        .child(
            col()
                .gap(0)
                .child(label("Variants:").fg(Color::Cyan))
                .child(label("  • Simple: Default with column dividers").fg(Color::Indexed(8)))
                .child(label("  • Bordered: Full table borders").fg(Color::Indexed(8)))
                .child(label("  • Compact: No horizontal dividers").fg(Color::Indexed(8)))
                .child(label("  • Striped: Alternating row backgrounds").fg(Color::Indexed(8)))
                .child(
                    label("  • Static: Display-only, no selection/scrollbar").fg(Color::Indexed(8)),
                ),
        )
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        variant_index: 0,
        border_index: 0,
    });

    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .on_key(KeyCode::Char('Q'), || Message::Quit)
        .on_key(KeyCode::Char('v'), || Message::CycleVariant)
        .on_key(KeyCode::Char('V'), || Message::CycleVariant)
        .on_key(KeyCode::Char('b'), || Message::CycleBorder)
        .on_key(KeyCode::Char('B'), || Message::CycleBorder)
        .on_key(KeyCode::Esc, || Message::Quit)
        .run_inline(update, view)?;
    Ok(())
}
