//! Simple Tabs Test - Minimal example to debug rendering
//!
//! Run with: cargo run --example tabs_simple

use tui::prelude::*;

#[derive(Debug)]
struct State {
    tab_index: usize,
    variant_index: usize,
    variants: Vec<(&'static str, TabVariant)>,
}

#[derive(Clone, Debug)]
enum Message {
    TabChanged(usize),
    SwitchVariant,
    Quit,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::TabChanged(idx) => {
            state.tab_index = idx;
        }
        Message::SwitchVariant => {
            state.variant_index = (state.variant_index + 1) % state.variants.len();
        }
        Message::Quit => std::process::exit(0),
    }
}

fn view(state: &State) -> impl Layout<Message> {
    let (variant_name, variant) = state.variants[state.variant_index];

    col()
        .padding(Padding::new(2, 2, 2, 2))
        .gap(1)
        .child(
            label(format!("Simple Tabs Test - Variant: {}", variant_name))
                .fg(Color::Cyan)
                .bold(),
        )
        .child(label(format!("Current Tab: {}", state.tab_index)).fg(Color::Green))
        .child(
            tabs()
                .variant(variant)
                .orientation(TabOrientation::Horizontal)
                .active(state.tab_index)
                .tab(TabItem::new(
                    "Tab 1",
                    col()
                        .padding(Padding::new(1, 1, 1, 1))
                        .child(label("Content of Tab 1").fg(Color::White)),
                ))
                .tab(TabItem::new(
                    "Tab 2",
                    col()
                        .padding(Padding::new(1, 1, 1, 1))
                        .child(label("Content of Tab 2").fg(Color::Yellow)),
                ))
                .tab(TabItem::new(
                    "Tab 3",
                    col()
                        .padding(Padding::new(1, 1, 1, 1))
                        .child(label("Content of Tab 3").fg(Color::Cyan)),
                ))
                .on_change(|event| Message::TabChanged(event.index)),
        )
        .child(spacer().height(1))
        .child(
            label("Left/Right: Switch tabs | 'v': Switch variant | 'q': Quit")
                .fg(Color::Indexed(8)),
        )
}

fn main() -> WidgetResult<()> {
    let app = App::new(State {
        tab_index: 0,
        variant_index: 0,
        variants: vec![
            ("Line", TabVariant::Line),
            ("Solid", TabVariant::Solid),
            ("Pills", TabVariant::Pills),
            ("Enclosed", TabVariant::Enclosed),
            ("Minimal", TabVariant::Minimal),
        ],
    });

    app.on_key(KeyCode::Char('q'), || Message::Quit)
        .on_key(KeyCode::Char('v'), || Message::SwitchVariant)
        .run_inline(update, view)?;
    Ok(())
}
