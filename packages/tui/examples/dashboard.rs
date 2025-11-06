//! Dashboard Example
//!
//! A comprehensive example demonstrating multiple widgets together:
//! buttons, checkboxes, labels, progress indicators, and layouts.
//! This simulates a system monitoring dashboard.
//!
//! Run with: cargo run --example dashboard

use tui::prelude::*;

/// Application state
#[derive(Debug)]
struct DashboardState {
    cpu_usage: f32,
    memory_usage: f32,
    disk_usage: f32,
    services: Vec<Service>,
    notifications_enabled: bool,
    auto_refresh: bool,
    uptime_seconds: u32,
}

#[derive(Debug, Clone)]
struct Service {
    name: String,
    status: ServiceStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum ServiceStatus {
    Running,
    Stopped,
    Error,
}

/// Messages that can update the state
#[derive(Clone, Debug)]
enum Message {
    ToggleNotifications,
    ToggleAutoRefresh,
    IncreaseCpu,
    DecreaseCpu,
    Refresh,
}

/// Update function - handles state changes
fn update(state: &mut DashboardState, msg: Message) {
    match msg {
        Message::ToggleNotifications => {
            state.notifications_enabled = !state.notifications_enabled;
        }
        Message::ToggleAutoRefresh => {
            state.auto_refresh = !state.auto_refresh;
        }
        Message::IncreaseCpu => {
            state.cpu_usage = (state.cpu_usage + 0.1).min(1.0);
        }
        Message::DecreaseCpu => {
            state.cpu_usage = (state.cpu_usage - 0.1).max(0.0);
        }
        Message::Refresh => {
            state.uptime_seconds += 1;
            // Simulate some changes
            state.memory_usage = ((state.memory_usage + 0.01) % 1.0).max(0.3);
            state.disk_usage = ((state.disk_usage + 0.005) % 1.0).max(0.5);
        }
    }
}

/// Helper to create a progress bar
fn progress_bar(label: &str, value: f32, color: Color) -> AnyWidget<Message> {
    let percentage = (value * 100.0) as u32;
    let filled = (value * 20.0) as usize;
    let bar = format!(
        "{:10} [{}{}] {:3}%",
        label,
        "█".repeat(filled),
        "░".repeat(20 - filled),
        percentage
    );

    let bar_color = if value > 0.8 {
        Color::Red
    } else if value > 0.6 {
        Color::Yellow
    } else {
        color
    };

    Label::new(bar)
        .style(Style::default().fg(bar_color))
        .into()
}

/// Helper to create service status widget
fn service_widget(service: &Service, _index: usize) -> AnyWidget<Message> {
    let (status_icon, status_color) = match service.status {
        ServiceStatus::Running => ("●", Color::Green),
        ServiceStatus::Stopped => ("○", Color::Indexed(8)),
        ServiceStatus::Error => ("✖", Color::Red),
    };

    Label::new(format!("  {} {}", status_icon, service.name))
        .style(Style::default().fg(status_color))
        .into()
}

/// Format uptime
fn format_uptime(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

/// View function - renders the UI from current state
fn view(state: &DashboardState) -> Container<Message> {
    // Header section
    let header = Container::vertical(vec![
        Label::new("📊 System Dashboard")
            .style(Style::default().fg(Color::Cyan).bold())
            .into(),
        Label::new(format!("Uptime: {}", format_uptime(state.uptime_seconds)))
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0);

    // Resource monitoring section
    let resources = Container::vertical(vec![
        Label::new("Resource Usage:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        progress_bar("CPU:", state.cpu_usage, Color::Cyan),
        progress_bar("Memory:", state.memory_usage, Color::Green),
        progress_bar("Disk:", state.disk_usage, Color::Blue),
    ])
    .gap(0);

    // Services section
    let mut service_widgets = vec![
        Label::new("Services:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
    ];

    for (i, service) in state.services.iter().enumerate() {
        service_widgets.push(service_widget(service, i));
    }

    let services = Container::vertical(service_widgets).gap(0);

    // Settings section
    let settings = Container::vertical(vec![
        Label::new("Settings:")
            .style(Style::default().fg(Color::Yellow).bold())
            .into(),
        Checkbox::new(
            "Enable notifications",
            state.notifications_enabled,
        )
        .on_toggle(|| Message::ToggleNotifications)
        .into(),
        Checkbox::new("Auto-refresh", state.auto_refresh)
            .on_toggle(|| Message::ToggleAutoRefresh)
            .into(),
    ])
    .gap(0);

    // Main layout
    Container::vertical(vec![
        header.into(),
        Label::new("")
            .into(),
        resources.into(),
        Label::new("")
            .into(),
        services.into(),
        Label::new("")
            .into(),
        settings.into(),
        Label::new("")
            .into(),
        // Keyboard controls
        KeyboardController::new()
            .on_key(KeyCode::Char('r'), || Message::Refresh)
            .on_up(|| Message::IncreaseCpu)
            .on_down(|| Message::DecreaseCpu)
            .into(),
        // Instructions
        Label::new("Controls:")
            .style(Style::default().fg(Color::Yellow))
            .into(),
        Label::new("  r: refresh | Up/Down: adjust CPU | Tab: navigate | Space: toggle")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
        Label::new("  Esc/q: quit")
            .style(Style::default().fg(Color::Indexed(8)))
            .into(),
    ])
    .gap(0)
    .padding(Padding::new(2, 2, 1, 1))
}

fn main() -> Result<()> {
    let app = App::new(DashboardState {
        cpu_usage: 0.45,
        memory_usage: 0.67,
        disk_usage: 0.82,
        services: vec![
            Service {
                name: "Web Server".to_string(),
                status: ServiceStatus::Running,
            },
            Service {
                name: "Database".to_string(),
                status: ServiceStatus::Running,
            },
            Service {
                name: "Cache".to_string(),
                status: ServiceStatus::Running,
            },
            Service {
                name: "Queue Worker".to_string(),
                status: ServiceStatus::Stopped,
            },
            Service {
                name: "Backup Service".to_string(),
                status: ServiceStatus::Error,
            },
        ],
        notifications_enabled: true,
        auto_refresh: false,
        uptime_seconds: 3661,
    });
    app.run(update, view)?;
    Ok(())
}
