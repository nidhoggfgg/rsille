//! Interactive inline showcase launcher shown when `rsille` runs with no arguments in a TTY.

pub(crate) mod fx;
pub(crate) mod scenes;
pub(crate) mod state;
pub(crate) mod ui;

use std::{
    error::Error,
    io::{self, Write},
    time::Instant,
};

use crossterm::{
    cursor::{MoveToColumn, MoveToPreviousLine},
    event::Event,
    style::{Print, ResetColor},
    terminal::{Clear, ClearType},
};
use rsille::render::{
    area::Size, buffer::Buffer, chunk::Chunk, Builder, Draw, DrawErr, InlineMouseMode, Update,
};

use self::{
    scenes::SceneEngine,
    state::{HitRegions, LaunchTarget, LauncherOutcome, LauncherState},
    ui::LauncherRenderer,
};

const INLINE_MAX_HEIGHT: u16 = 24;
const TARGET_INLINE_HEIGHT: u16 = 22;
const FRAME_RATE_FPS: u16 = 30;

/// Interactive inline launcher application driving the 30 FPS showcase loop.
pub(crate) struct Launcher {
    state: LauncherState,
    scenes: SceneEngine,
    renderer: LauncherRenderer,
    regions: HitRegions,
    last_tick: Option<Instant>,
    last_rendered_height: u16,
}

impl Default for Launcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher {
    pub(crate) fn new() -> Self {
        Self {
            state: LauncherState::new(),
            scenes: SceneEngine::new(),
            renderer: LauncherRenderer::new(),
            regions: HitRegions::default(),
            last_tick: None,
            last_rendered_height: 0,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn state(&self) -> &LauncherState {
        &self.state
    }

    #[allow(dead_code)]
    pub(crate) fn state_mut(&mut self) -> &mut LauncherState {
        &mut self.state
    }

    #[allow(dead_code)]
    pub(crate) fn hit_regions(&self) -> HitRegions {
        self.regions
    }

    #[allow(dead_code)]
    pub(crate) fn advance(&mut self, dt: f64) {
        self.state.advance(dt);
    }

    #[allow(dead_code)]
    pub(crate) fn handle_event(&mut self, event: &Event) -> bool {
        self.state.handle_event(event, &self.regions)
    }

    pub(crate) fn outcome(&self) -> Option<&LauncherOutcome> {
        self.state.outcome.as_ref()
    }

    pub(crate) fn last_rendered_height(&self) -> u16 {
        self.last_rendered_height
    }

    #[allow(dead_code)]
    pub(crate) fn render_frame(&mut self, size: Size) -> Result<Buffer, DrawErr> {
        render_frame_for_test(self, size)
    }
}

/// Renders a single deterministic launcher frame into an offscreen `Buffer` for tests and QA.
#[allow(dead_code)]
pub(crate) fn render_frame_for_test(
    launcher: &mut Launcher,
    size: Size,
) -> Result<Buffer, DrawErr> {
    let mut buffer = Buffer::new(size);
    let chunk = Chunk::new(&mut buffer, size.into())?;
    launcher.draw(chunk)?;
    Ok(buffer)
}

impl Update for Launcher {
    fn on_events(&mut self, events: &[Event]) -> Result<(), DrawErr> {
        for event in events {
            self.state.handle_event(event, &self.regions);
            if self.state.should_quit() {
                break;
            }
        }
        Ok(())
    }

    fn on_pointer_leave(&mut self) {
        self.state.clear_hover();
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        let now = Instant::now();
        if let Some(last) = self.last_tick {
            let dt = now.duration_since(last).as_secs_f64();
            self.state.advance(dt);
        }
        self.last_tick = Some(now);
        Ok(true)
    }

    fn should_quit(&self) -> bool {
        self.state.should_quit()
    }

    fn required_size(&self, current_size: Size) -> Option<Size> {
        let desired_h = current_size.height.min(TARGET_INLINE_HEIGHT);
        Some(Size::from((current_size.width, desired_h)))
    }
}

impl Draw for Launcher {
    fn draw(&mut self, chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        self.last_rendered_height = size.height;
        let layout = self.renderer.draw(&self.state, &mut self.scenes, chunk)?;
        self.regions = layout.regions;
        Ok(size)
    }
}

/// Runs the inline interactive launcher and cleanly hands off to any selected experience.
pub fn run() -> Result<(), Box<dyn Error>> {
    crate::require_terminal()?;
    let (width, height) = crossterm::terminal::size()?;
    let buf_h = height.clamp(1, INLINE_MAX_HEIGHT);
    let used_h = height.clamp(1, TARGET_INLINE_HEIGHT);

    let mut builder = Builder::new();
    builder
        .size((width, buf_h))
        .inline_mode(true)
        .inline_max_height(INLINE_MAX_HEIGHT)
        .inline_mouse_mode(InlineMouseMode::Local)
        .enable_raw_mode()
        .enable_hide_cursor()
        .enable_mouse_capture()
        .disable_exit_code()
        .frame_limit(FRAME_RATE_FPS);

    let mut event_loop = builder.build_event_loop(Launcher::new());
    event_loop.set_initial_used_height(used_h);
    let launcher = event_loop.run_returning()?;

    match launcher.outcome() {
        Some(LauncherOutcome::Launch(target)) => {
            clear_inline_region(launcher.last_rendered_height())?;
            handoff_launch(target)
        }
        Some(LauncherOutcome::Quit) | None => {
            settle_inline_exit()?;
            Ok(())
        }
    }
}

fn clear_inline_region(rendered_rows: u16) -> io::Result<()> {
    let mut out = io::stdout();
    let rows = crossterm::terminal::size()
        .map(|(_, h)| rendered_rows.min(h))
        .unwrap_or(rendered_rows);
    write_clear_inline_region(&mut out, rows)?;
    out.flush()
}

fn write_clear_inline_region(out: &mut impl Write, rows: u16) -> io::Result<()> {
    crossterm::queue!(out, ResetColor, Print("\r"), MoveToColumn(0))?;
    for row in 0..rows {
        crossterm::queue!(out, Clear(ClearType::CurrentLine))?;
        if row + 1 < rows {
            crossterm::queue!(out, MoveToPreviousLine(1))?;
        }
    }
    crossterm::queue!(out, MoveToColumn(0))?;
    Ok(())
}

fn settle_inline_exit() -> io::Result<()> {
    let mut out = io::stdout();
    crossterm::queue!(out, ResetColor, Print("\r\n"))?;
    out.flush()
}

fn handoff_launch(target: &LaunchTarget) -> Result<(), Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).args(&target.args).exec();
        Err(Box::new(err))
    }
    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&exe)
            .args(&target.args)
            .status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use scenes::ExperienceId;

    #[test]
    fn render_frame_for_test_populates_buffer_and_hit_regions() {
        let mut launcher = Launcher::new();
        launcher.advance(0.5);
        let buffer = render_frame_for_test(&mut launcher, (110, 24).into()).unwrap();
        assert_eq!(buffer.size(), Size::from((110, 24)));
        assert_eq!(launcher.last_rendered_height(), 24);

        // Clicking inside card 2 (FunctionPlotter) selects it using the computed hit regions.
        let card2 = launcher.hit_regions().cards[ExperienceId::FunctionPlotter.index()];
        assert!(card2.width() > 0 && card2.height() > 0);
        let click = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: card2.x() + 2,
            row: card2.y() + 1,
            modifiers: KeyModifiers::NONE,
        });
        assert!(launcher.handle_event(&click));
        assert_eq!(launcher.state().selected, ExperienceId::FunctionPlotter);
        assert!(!launcher.state().auto_tour);

        // Enter produces a Launch outcome targeting `draw`.
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(launcher.handle_event(&enter));
        assert!(launcher.should_quit());
        let Some(LauncherOutcome::Launch(target)) = launcher.outcome() else {
            panic!("expected launch outcome");
        };
        assert_eq!(target.experience, ExperienceId::FunctionPlotter);
        assert_eq!(target.args[0], "draw");
    }

    #[test]
    fn clear_inline_region_emits_reset_and_erases_exact_row_count() {
        let mut bytes = Vec::new();
        write_clear_inline_region(&mut bytes, 4).unwrap();
        let seq = String::from_utf8(bytes).unwrap();
        assert!(seq.starts_with("\x1b[0m\r"));
        assert_eq!(seq.matches("\x1b[2K").count(), 4);
        assert_eq!(seq.matches("\x1b[1F").count(), 3);

        let mut empty_bytes = Vec::new();
        write_clear_inline_region(&mut empty_bytes, 0).unwrap();
        let empty_seq = String::from_utf8(empty_bytes).unwrap();
        assert!(empty_seq.starts_with("\x1b[0m\r"));
        assert_eq!(empty_seq.matches("\x1b[2K").count(), 0);
    }
}
