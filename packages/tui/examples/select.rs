// Select widget example
//
// This example demonstrates the Select (dropdown) widget with various options

use tui::prelude::*;
use tui::widget::{select, Select, SelectEvent, SelectItem};

#[derive(Clone, Debug, PartialEq)]
enum Fruit {
    Apple,
    Banana,
    Orange,
    Grape,
    Strawberry,
    Mango,
    Pineapple,
    Watermelon,
    Kiwi,
    Peach,
}

impl Fruit {
    fn label(&self) -> String {
        match self {
            Fruit::Apple => "🍎 Apple".to_string(),
            Fruit::Banana => "🍌 Banana".to_string(),
            Fruit::Orange => "🍊 Orange".to_string(),
            Fruit::Grape => "🍇 Grape".to_string(),
            Fruit::Strawberry => "🍓 Strawberry".to_string(),
            Fruit::Mango => "🥭 Mango".to_string(),
            Fruit::Pineapple => "🍍 Pineapple".to_string(),
            Fruit::Watermelon => "🍉 Watermelon".to_string(),
            Fruit::Kiwi => "🥝 Kiwi".to_string(),
            Fruit::Peach => "🍑 Peach".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
enum Country {
    UnitedStates,
    China,
    Japan,
    Germany,
    UnitedKingdom,
}

impl Country {
    fn label(&self) -> &str {
        match self {
            Country::UnitedStates => "🇺🇸 United States",
            Country::China => "🇨🇳 China",
            Country::Japan => "🇯🇵 Japan",
            Country::Germany => "🇩🇪 Germany",
            Country::UnitedKingdom => "🇬🇧 United Kingdom",
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    FruitSelected(SelectEvent<Fruit>),
    CountrySelected(SelectEvent<Country>),
    SizeSelected(SelectEvent<String>),
    ColorSelected(SelectEvent<String>),
}

struct State {
    selected_fruit: Option<Fruit>,
    selected_country: Option<Country>,
    selected_size: Option<String>,
    selected_color: Option<String>,
}

fn update(state: &mut State, msg: Message) {
    match msg {
        Message::FruitSelected(event) => {
            state.selected_fruit = Some(event.value);
        }
        Message::CountrySelected(event) => {
            state.selected_country = Some(event.value);
        }
        Message::SizeSelected(event) => {
            state.selected_size = Some(event.value);
        }
        Message::ColorSelected(event) => {
            state.selected_color = Some(event.value);
        }
    }
}

fn view(state: &State) -> impl Layout<Message> {
    // Create fruit select items
    let fruit_items = vec![
        SelectItem::new(Fruit::Apple, Fruit::Apple.label()),
        SelectItem::new(Fruit::Banana, Fruit::Banana.label()),
        SelectItem::new(Fruit::Orange, Fruit::Orange.label()),
        SelectItem::new(Fruit::Grape, Fruit::Grape.label()),
        SelectItem::new(Fruit::Strawberry, Fruit::Strawberry.label()),
        SelectItem::new(Fruit::Mango, Fruit::Mango.label()),
        SelectItem::new(Fruit::Pineapple, Fruit::Pineapple.label()),
        SelectItem::new(Fruit::Watermelon, Fruit::Watermelon.label()),
        SelectItem::new(Fruit::Kiwi, Fruit::Kiwi.label()),
        SelectItem::new(Fruit::Peach, Fruit::Peach.label()),
    ];

    // Create country select items
    let country_items = vec![
        SelectItem::new(Country::UnitedStates, Country::UnitedStates.label()),
        SelectItem::new(Country::China, Country::China.label()),
        SelectItem::new(Country::Japan, Country::Japan.label()),
        SelectItem::new(Country::Germany, Country::Germany.label()),
        SelectItem::new(Country::UnitedKingdom, Country::UnitedKingdom.label()),
    ];

    col()
        .gap(1)
        .padding(Padding::new(1, 2, 1, 2)) // top, right, bottom, left
        // Title
        .child(label("Select Widget Example").style(Style::default().bold()))
        .child(spacer().height(1))
        // Fruit selector (Overlay mode - default)
        .child(label("Select your favorite fruit (Overlay Mode):"))
        .child(
            Select::new(fruit_items.clone())
                .placeholder("Choose a fruit...")
                .dropdown_height(6)
                .overlay_mode(true) // Explicit overlay mode (default)
                .on_select(|event| Message::FruitSelected(event)),
        )
        .child(label(if let Some(ref fruit) = state.selected_fruit {
            format!("Selected: {}", fruit.label())
        } else {
            "No fruit selected yet".to_string()
        }))
        .child(spacer().height(1))
        // Country selector (Overlay mode)
        .child(label("Select your country (Overlay Mode):"))
        .child(
            Select::new(country_items)
                .placeholder("Choose a country...")
                .on_select(|event| Message::CountrySelected(event)),
        )
        .child(label(if let Some(ref country) = state.selected_country {
            format!("Selected: {}", country.label())
        } else {
            "No country selected yet".to_string()
        }))
        .child(spacer().height(1))
        // Size selector (Inline mode for comparison)
        .child(label("Select a size (Inline Mode):"))
        .child(
            select()
                .item("xs".to_string(), "Extra Small")
                .item("s".to_string(), "Small")
                .item("m".to_string(), "Medium")
                .item("l".to_string(), "Large")
                .item("xl".to_string(), "Extra Large")
                .item_disabled("xxl".to_string(), "XXL (Out of stock)")
                .placeholder("Choose size...")
                .overlay_mode(false) // Inline mode: dropdown pushes content down
                .on_select(|event| Message::SizeSelected(event)),
        )
        .child(label(if let Some(ref size) = state.selected_size {
            format!("Selected size: {}", size)
        } else {
            "No size selected yet".to_string()
        }))
        .child(spacer().height(1))
        // Borderless select (NEW feature)
        .child(label("Select a color (Borderless Mode):"))
        .child(
            select()
                .item("red".to_string(), "Red")
                .item("blue".to_string(), "Blue")
                .item("green".to_string(), "Green")
                .item("yellow".to_string(), "Yellow")
                .item("purple".to_string(), "Purple")
                .placeholder("Choose color...")
                .borderless(true) // NEW: Borderless variant
                .on_select(|event| Message::ColorSelected(event)),
        )
        .child(label(if let Some(ref color) = state.selected_color {
            format!("Selected color: {}", color)
        } else {
            "No color selected yet".to_string()
        }))
        .child(spacer().height(1))
        .child(label("--- Instructions ---"))
        .child(label("Overlay mode: Dropdown floats above (like web UI)"))
        .child(label("Inline mode: Dropdown pushes content down"))
        .child(label("Borderless mode: Minimal styling without borders"))
        .child(spacer().height(1))
        .child(label("Use Tab to navigate between selects"))
        .child(label("Use Enter/Space to open dropdown"))
        .child(label("Use Arrow keys to navigate options"))
        .child(label("Use Mouse Click to select (NEW feature)"))
        .child(label("Use Escape to close dropdown"))
        .child(label("Press Ctrl+C to exit"))
}

fn main() -> tui::Result<()> {
    let state = State {
        selected_fruit: None,
        selected_country: None,
        selected_size: None,
        selected_color: None,
    };

    App::new(state).run(update, view)?;

    Ok(())
}
