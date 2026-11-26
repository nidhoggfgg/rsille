# rsille-overlay

English · [中文](README_cn.md)

Draw over a live shell instead of replacing it.

`rsille-overlay` runs your shell (or any program) in a PTY and passes its
output through unchanged. Between output batches, at points where no escape
sequence, image upload or synchronized update is open, it lets an overlay
draw on top. Before the next output that could disturb it, the overlay hides
again and the covered text is put back.

Shell activity is reported as events, so an overlay can react to what you do:

| Event | When |
| --- | --- |
| `Prompt` | a new prompt is shown |
| `Edit(line)` | the line being edited changed (zsh) |
| `Exec(line)` | a command is about to run |
| `Done(status)` | that command finished |
| `Read` | a running command reads a line through the shell (fish) |

Events come from bash, fish and zsh hooks installed only for the session.
The user's own startup files still run first. bash reads the command line
from its history, so a line history skips (for example with
`HISTCONTROL=ignorespace`) is reported as the previous entry. Other shells
work in a session but report no events.

The session clears the screen when it starts and after each resize. It cannot
read what was on the terminal before, and terminals reflow text on resize
differently from the session's copy, so clearing is what keeps restored text
correct. Use `alt_screen(true)` to keep your scrollback untouched.

## A button over your shell

`WidgetOverlay` hosts a regular rsille-tui app in a small box. Mouse clicks
inside the box reach its widgets; the keyboard stays with the shell.

```rust,no_run
use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};
use rsille_tui::prelude::*;

let app = App::new(String::new())
    .update(|last: &mut String, msg: Option<String>, _ctx| *last = msg.unwrap_or_default())
    .view(|last: &String| {
        row::<Option<String>>()
            .child(label(last.clone()).width(20))
            .child(button("×").on_click(|| None))
    })
    .build();
let overlay = WidgetOverlay::new(app, (24, 1))
    .anchor(Anchor::TopRight)
    .on_shell(|event| match event {
        ShellEvent::Exec(line) => Some(Some(line.to_owned())),
        _ => None,
    });
Session::new().mouse(true).run(overlay)?;
# Ok::<(), std::io::Error>(())
```

Run the examples from bash, fish or zsh:

```sh
cargo run -p rsille-overlay --example status_badge     # last command's status
cargo run -p rsille-overlay --example click_counter    # two boxes over your shell
cargo run -p rsille-overlay --example click_counter -- top
```

Press **Ctrl+]** to leave a session at any time, or `exit` the shell.

## Custom overlays

Implement `Overlay` to write the bytes yourself, including Kitty or Sixel
images. The trait has two required methods, `draw` and `hide`. Everything
else (pointer hits, input, output, bell, shell events, resize) is optional.
`Stack` combines several overlays; presses go to the topmost one that claims
them.

```rust,no_run
use rsille_overlay::{Context, Overlay, Session};
use rsille_render::area::Area;
use std::io::{self, Write};

struct Clock(Option<Area>);

impl Overlay for Clock {
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        let col = cx.cols.saturating_sub(10);
        write!(out, "\x1b[1;{}H\x1b[7m {:>6.1}s \x1b[0m", col + 1, cx.elapsed().as_secs_f32())?;
        self.0 = Some(Area::new((col, 0).into(), (9, 1).into()));
        Ok(())
    }
    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        if let Some(area) = self.0.take() {
            cx.erase(area, out); // Puts back the shell's text under the clock.
        }
    }
}

Session::new().run(Clock(None))?;
# Ok::<(), std::io::Error>(())
```

## Graphics and the mouse

| | Text (default) | Sixel | Kitty |
| --- | --- | --- | --- |
| Enable | — | `.graphics(RasterBackend::Sixel)` | `.graphics(RasterBackend::Kitty)` |
| On a fullscreen child (vim, top) | hidden | hidden | stays visible |
| Mouse with `.mouse(true)` | overlay gets it while the child does not track the mouse | same as text | shared: claimed presses go to the overlay, the rest to the child in its own mouse mode |

`rsille pet` from rsille-playground is built on this crate.

Sessions require Unix PTYs.
