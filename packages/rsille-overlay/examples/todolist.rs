//! A fixed session checklist. Click a task to check it off, then clear completed tasks.
use rsille_overlay::{Anchor, Session, WidgetOverlay};
use rsille_tui::prelude::*;

struct Task {
    name: &'static str,
    done: bool,
}

enum Msg {
    Check(usize, bool),
    ClearDone,
}

fn view(tasks: &[Task]) -> impl Widget<Msg> {
    let done = tasks.iter().filter(|task| task.done).count();
    let mut panel = panel()
        .title(format!("Tasks  {done}/{}", tasks.len()))
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1));
    if tasks.is_empty() {
        panel = panel.child(label("All clear!").fg(Color::Green));
    }
    for (i, task) in tasks.iter().enumerate() {
        panel = panel.child(
            checkbox(task.name)
                .checked(task.done)
                .on_change(move |v| Msg::Check(i, v)),
        );
    }
    panel.child(
        button("Clear done")
            .disabled(done == 0)
            .on_click(|| Msg::ClearDone),
    )
}

fn main() -> std::io::Result<()> {
    let tasks: Vec<_> = ["Review the diff", "Run the tests", "Ship the patch"]
        .into_iter()
        .map(|name| Task { name, done: false })
        .collect();
    let height = tasks.len() as u16 + 3;
    let app = App::new(tasks)
        .update(|tasks, msg, _| match msg {
            Msg::Check(i, done) => tasks[i].done = done,
            Msg::ClearDone => tasks.retain(|task| !task.done),
        })
        .view(|tasks| view(tasks))
        .build();
    Session::new()
        .mouse(true)
        .run(WidgetOverlay::new(app, (32, height)).anchor(Anchor::TopRight))?;
    Ok(())
}
