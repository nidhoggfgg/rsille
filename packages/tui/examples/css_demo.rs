//! CSS Styling Demo
//!
//! A clean demonstration of CSS styling capabilities without visual clutter.
//!
//! Run with: cargo run --example css_demo

use tui::prelude::*;

#[derive(Debug)]
struct StyleDemo;

#[derive(Clone, Debug)]
enum Message {}

fn update(_state: &mut StyleDemo, _msg: Message) {}

fn view(_state: &StyleDemo) -> Container<Message> {
    Container::vertical(vec![
        // Title
        Label::new("═══════════════════════════════════════════════════════")
            .style(Style::css("color: cyan; font-weight: bold").unwrap())
            .into(),
        Label::new("        CSS Styling Demonstration")
            .style(Style::css("color: cyan; font-weight: bold").unwrap())
            .into(),
        Label::new("═══════════════════════════════════════════════════════")
            .style(Style::css("color: cyan; font-weight: bold").unwrap())
            .into(),

        Label::new("").into(),

        // Named Colors
        Label::new("Named Colors:")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Label::new("  red | green | blue | cyan | magenta | yellow | white")
            .style(Style::css("color: white").unwrap())
            .into(),

        Container::horizontal(vec![
            Label::new("  Red   ").style(Style::css("color: red").unwrap()).into(),
            Label::new("  Green ").style(Style::css("color: green").unwrap()).into(),
            Label::new("  Blue  ").style(Style::css("color: blue").unwrap()).into(),
            Label::new("  Cyan  ").style(Style::css("color: cyan").unwrap()).into(),
            Label::new("  Magenta ").style(Style::css("color: magenta").unwrap()).into(),
            Label::new("  Yellow").style(Style::css("color: yellow").unwrap()).into(),
        ])
        .gap(1)
        .into(),

        Label::new("").into(),

        // Hex Colors
        Label::new("Hex Colors (#RRGGBB or #RGB):")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Container::horizontal(vec![
            Label::new("  #ff5733 ").style(Style::css("color: #ff5733").unwrap()).into(),
            Label::new("  #33ff57 ").style(Style::css("color: #33ff57").unwrap()).into(),
            Label::new("  #3357ff ").style(Style::css("color: #3357ff").unwrap()).into(),
            Label::new("  #f0f ").style(Style::css("color: #f0f").unwrap()).into(),
            Label::new("  #0ff ").style(Style::css("color: #0ff").unwrap()).into(),
        ])
        .gap(1)
        .into(),

        Label::new("").into(),

        // RGB Colors
        Label::new("RGB Colors (rgb(r,g,b)):")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Container::horizontal(vec![
            Label::new("  rgb(255,100,50)  ").style(Style::css("color: rgb(255,100,50)").unwrap()).into(),
            Label::new("  rgb(100,255,100) ").style(Style::css("color: rgb(100,255,100)").unwrap()).into(),
            Label::new("  rgb(100,150,255) ").style(Style::css("color: rgb(100,150,255)").unwrap()).into(),
        ])
        .gap(1)
        .into(),

        Label::new("").into(),

        // Text Decorations
        Label::new("Text Decorations:")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Container::horizontal(vec![
            Label::new("  Normal  ").style(Style::css("color: white").unwrap()).into(),
            Label::new("  Bold  ").style(Style::css("color: green; font-weight: bold").unwrap()).into(),
            Label::new("  Italic  ").style(Style::css("color: cyan; font-style: italic").unwrap()).into(),
            Label::new("  Underline  ").style(Style::css("color: magenta; text-decoration: underline").unwrap()).into(),
        ])
        .gap(2)
        .into(),

        Container::horizontal(vec![
            Label::new("  Bold+Italic  ").style(Style::css("color: yellow; font-weight: bold; font-style: italic").unwrap()).into(),
            Label::new("  Bold+Underline  ").style(Style::css("color: red; font-weight: bold; text-decoration: underline").unwrap()).into(),
            Label::new("  All Combined  ").style(Style::css("color: #00ffff; font-weight: bold; font-style: italic; text-decoration: underline").unwrap()).into(),
        ])
        .gap(2)
        .into(),

        Label::new("").into(),

        // Background Colors (only on labels, not containers)
        Label::new("Background Colors:")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Container::horizontal(vec![
            Label::new("  White on Red  ").style(Style::css("color: white; background-color: red").unwrap()).into(),
            Label::new("  Black on Cyan  ").style(Style::css("color: black; background-color: cyan").unwrap()).into(),
            Label::new("  White on Blue  ").style(Style::css("color: white; background-color: blue").unwrap()).into(),
        ])
        .gap(2)
        .into(),

        Container::horizontal(vec![
            Label::new("  Dark BG  ").style(Style::css("color: white; background-color: #1e1e1e").unwrap()).into(),
            Label::new("  Purple BG  ").style(Style::css("color: white; background-color: #663399").unwrap()).into(),
            Label::new("  Orange BG  ").style(Style::css("color: black; background-color: #ff9933").unwrap()).into(),
        ])
        .gap(2)
        .into(),

        Label::new("").into(),

        // Border Examples with single container
        Label::new("Border Styles (on containers):")
            .style(Style::css("color: yellow; font-weight: bold").unwrap())
            .into(),

        Label::new("").into(),

        Container::horizontal(vec![
            Container::vertical(vec![
                Label::new(" Single ").style(Style::css("color: cyan").unwrap()).into(),
            ])
            .style(Style::css("border: single; padding: 1").unwrap())
            .into(),

            Container::vertical(vec![
                Label::new(" Rounded ").style(Style::css("color: green").unwrap()).into(),
            ])
            .style(Style::css("border: rounded; padding: 1").unwrap())
            .into(),

            Container::vertical(vec![
                Label::new(" Double ").style(Style::css("color: magenta").unwrap()).into(),
            ])
            .style(Style::css("border: double; padding: 1").unwrap())
            .into(),

            Container::vertical(vec![
                Label::new(" Thick ").style(Style::css("color: yellow").unwrap()).into(),
            ])
            .style(Style::css("border: thick; padding: 1").unwrap())
            .into(),
        ])
        .gap(3)
        .into(),

        Label::new("").into(),
        Label::new("").into(),

        // Footer
        Label::new("Press Esc or 'q' to quit")
            .style(Style::css("color: #666666; font-style: italic").unwrap())
            .into(),
    ])
    .gap(0)
    .padding(Padding::uniform(2))
}

fn main() -> Result<()> {
    let app = App::new(StyleDemo);
    app.run(update, view)?;
    Ok(())
}
