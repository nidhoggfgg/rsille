use std::{thread, time::Duration};

use rsille::{
    render,
    tui::{composite::Reactive, widgets::Text},
};

fn main() {
    let s = r#"Hello! 你好！こんにちは！안녕하세요! Bonjour! 😊
这是一段混合了中文、English、日本語、한국어和Français的文本。
测试符号：!@#$%^&*()_+{}[];:'",.<>/?|~`
数字：1234567890 🔢
Emoji 序列：🚀🎉💻❤️😂🐱‍👤
会被截断的内容：12345678901234567890098765432112345678900987654321"#;
    let chars = s.chars();
    let mut text = Reactive::new(Text::new(""));
    let bind_str = text.watch(String::new(), |t, ss| {
        t.replace(ss.to_string());
    });

    let handler = thread::spawn(move || {
        let mut now = String::new();
        for c in chars {
            now.push(c);
            _ = bind_str.send(now.clone());
            thread::sleep(Duration::from_millis(100));
        }
    });

    let render = render::Builder::new()
        .size((60, 10))
        .frame_limit(30)
        .enable_all()
        .build_eventloop(text);
    render.run();
    handler.join().unwrap();
}
