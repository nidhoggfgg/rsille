use std::io::stdout;

use rsille::{render, tui::widgets::Text};

fn main() {
    let s = r#"Hello! 你好！こんにちは！안녕하세요! Bonjour! 😊
这是一段混合了中文、English、日本語、한국어和Français的文本。
测试符号：!@#$%^&*()_+{}[];:'",.<>/?|~`
数字：1234567890 🔢
Emoji 序列：🚀🎉💻❤️😂🐱‍👤
会被截断的内容：12345678901234567890098765432112345678900987654321"#;
    let text = Text::new(&s.to_string());

    let render = render::Builder::new()
        .size((60, 10))
        .build_render(text, stdout());
    render.render().unwrap();
}
