//! Static table example
//!
//! Demonstrates using tables for static data display (no interaction).
//! Static tables cannot be focused or selected, making them perfect for
//! showing configuration, status information, or readonly data.
//!
//! Run with: cargo run --example table_static

use tui::prelude::*;

#[derive(Clone)]
struct SystemInfo {
    key: String,
    value: String,
}

impl SystemInfo {
    fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

fn main() -> WidgetResult<()> {
    let system_data = vec![
        SystemInfo::new("OS", "Linux"),
        SystemInfo::new("Kernel", "6.6.87"),
        SystemInfo::new("CPU", "Intel Core i7-9700K"),
        SystemInfo::new("Memory", "32 GB DDR4"),
        SystemInfo::new("Disk", "1 TB NVMe SSD"),
    ];

    let network_data = vec![
        SystemInfo::new("Interface", "eth0"),
        SystemInfo::new("IP Address", "192.168.1.100"),
        SystemInfo::new("Gateway", "192.168.1.1"),
        SystemInfo::new("DNS", "8.8.8.8"),
    ];

    let columns = vec![
        Column::new("Property", |info: &SystemInfo| info.key.clone()).width(ColumnWidth::Flex(1)),
        Column::new("Value", |info: &SystemInfo| info.value.clone()).width(ColumnWidth::Flex(2)),
    ];

    let app = App::new(());
    app.on_key(KeyCode::Char('q'), || ())
        .run_inline(
        |_state, _msg| {},
        move |_state| {
            col()
                .padding(Padding::new(2, 2, 1, 1))
                .gap(1)
                // Title
                .child(label("System Information").fg(Color::Cyan).bold())
                .child(
                    label("Static tables cannot be focused or selected with keyboard")
                        .fg(Color::Yellow),
                )
                .child(spacer().height(1))
                // System info table (bordered with rounded corners)
                .child(label("System Details:").fg(Color::Green))
                .child(
                    Table::<SystemInfo, ()>::static_table(columns.clone())
                        .rows(system_data.clone())
                        .variant(TableVariant::Bordered)
                        .border(BorderStyle::Rounded)
                        .show_header(true),
                )
                .child(spacer().height(1))
                // Network info table (striped variant)
                .child(label("Network Configuration:").fg(Color::Green))
                .child(
                    Table::<SystemInfo, ()>::static_table(columns.clone())
                        .rows(network_data.clone())
                        .variant(TableVariant::Striped)
                        .show_header(true),
                )
                .child(spacer().height(1))
                .child(label("Press Ctrl+C or Q to exit").fg(Color::Indexed(8)))
        },
    )?;
    Ok(())
}
