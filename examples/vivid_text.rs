use std::{thread, time::Duration};

use rsille::{
    render, term,
    tui::{composite::Reactive, widgets::Text},
};

fn main() {
    let s = r#"Hello! 你好！こんにちは！안녕하세요! Bonjour! 😊
这是一段混合了中文、English、日本語、한국어和Français的文本。
测试符号：!@#$%^&*()_+{}[];:'",.<>/?|~`
数字：1234567890 🔢
Emoji 序列：🚀🎉💻❤️😂🐱‍👤
会被截断的内容：12345678901234567890098765432112345678900987654321"#;

    let text = Text::new("");
    let mut ss = String::new();
    let mut reactive = Reactive::new(text);
    let sender = reactive.watch(String::new(), |text, s| {
        text.replace(s.to_string());
    });

    let size = term::terminal_size().unwrap();
    let center = if size.0 > 60 && size.1 > 10 {
        (size.0 / 2 - 30, size.1 / 2 - 5)
    } else {
        (0, 0)
    };

    let handle = thread::spawn(move || {
        for c in s.chars() {
            ss.push(c);
            sender.send(ss.clone()).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    });
    let el_handle = thread::spawn(move || {
        let el = render::Builder::new()
            .enable_all()
            .size((60, 10))
            .pos(center)
            .clear(false)
            .frame_limit(30)
            .build_event_loop(reactive);
        el.run();
    });

    el_handle.join().unwrap();
    handle.join().unwrap();
}
