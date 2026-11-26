//! The pet as an overlay on a live shell session.
use crate::deepseek_pet::{
    commands::TypedPreview,
    dialogue::Catalog,
    food::Food,
    pet::{KittySprites, Pet, FRAMES_PER_TICK},
    Feature,
};
use rsille::render::raster_output::InlineRasterSession;
use rsille_overlay::{
    Attach, Context, Graphics, MouseKind, Overlay, Pointer, Session, Shell, ShellEvent,
};
use std::{
    io,
    time::{Duration, Instant},
};

const FRAME_RATE: u64 = 10 * FRAMES_PER_TICK;

/// Exit status of a direct command, otherwise 0.
pub fn run(options: crate::deepseek_pet::Options) -> io::Result<i32> {
    let direct = !options.command.is_empty();
    if options
        .command
        .first()
        .is_some_and(|program| program.is_empty())
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "程序路径为空"));
    }
    let features = options.enabled_features();
    let command_events = !direct
        && (features.contains(&Feature::CommandReactions) || features.contains(&Feature::Speech));
    let forage = features.contains(&Feature::Forage);
    let capture_mouse = features.contains(&Feature::Mouse);
    let catalog = if command_events || options.commands.is_some() {
        Catalog::load(options.commands.as_deref())?
    } else {
        Default::default()
    };
    let raster = options
        .backend
        .negotiate_inline(None, Duration::from_millis(500))
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("无法确认终端图形协议或字符格像素尺寸，未启动子进程：{e}"),
            )
        })?;
    let kitty = if matches!(raster, InlineRasterSession::Kitty(_)) {
        Some(KittySprites::new()?)
    } else {
        None
    };
    let mut pet = Pet::new(&catalog, features);
    pet.kitty = kitty.as_ref();
    let mut overlay = PetOverlay::new(pet, &catalog, forage);
    let mut session = Session::new()
        .name("rsille pet")
        .raster_session(raster)
        .alt_screen(options.alt_screen)
        .restore_text(!options.no_restore)
        .mouse(capture_mouse)
        .shell_events(command_events);
    if direct {
        session = session.command(options.command);
    }
    session.run(&mut overlay)
}

/// Follows a command from editing to its exit status.
#[derive(Default)]
struct CommandTracker {
    /// The rule shown for the line being edited (zsh reports edits).
    preview: Option<usize>,
    /// The rule of the running simple command; 0 for other simple commands.
    active: Option<usize>,
    /// bash and fish report no edits, so their typed bytes are previewed instead.
    typed: Option<TypedPreview>,
}

pub struct PetOverlay<'a> {
    pet: Pet<'a>,
    catalog: &'a Catalog,
    food: Option<Food>,
    food_deadline: Option<Instant>,
    commands: CommandTracker,
    epoch: Instant,
    kitty: bool,
    /// Animate the next frame promptly, e.g. while dragging.
    soon: bool,
}

impl<'a> PetOverlay<'a> {
    pub fn new(pet: Pet<'a>, catalog: &'a Catalog, forage: bool) -> Self {
        Self {
            kitty: pet.kitty.is_some(),
            pet,
            catalog,
            food: forage.then(|| Food::new(Some((1, 1)))),
            food_deadline: None,
            commands: CommandTracker::default(),
            epoch: Instant::now(),
            soon: false,
        }
    }

    fn frame(&self, now: Instant) -> u64 {
        (now.saturating_duration_since(self.epoch).as_nanos() * u128::from(FRAME_RATE)
            / 1_000_000_000) as u64
    }

    fn deadline(&self, frame: u64) -> Instant {
        self.epoch
            + Duration::from_secs(frame / FRAME_RATE)
            + Duration::from_nanos(((frame % FRAME_RATE) * 1_000_000_000).div_ceil(FRAME_RATE))
    }

    /// Run `f` with the session's text copy lent to the pet.
    fn with_screen<R>(&mut self, cx: &mut Context<'_>, f: impl FnOnce(&mut Pet<'a>) -> R) -> R {
        self.pet.screen = cx.screen.take();
        let result = f(&mut self.pet);
        *cx.screen = self.pet.screen.take();
        result
    }

    fn notice(&mut self, phase: u8, rule: usize) {
        self.pet.notice_command(phase, rule);
    }
}

impl Overlay for PetOverlay<'_> {
    fn attach(&mut self, attach: &Attach) -> io::Result<()> {
        self.epoch = attach.epoch;
        self.kitty = attach.graphics == Graphics::Kitty;
        self.commands.typed =
            matches!(attach.shell, Some(Shell::Bash | Shell::Fish)).then(TypedPreview::default);
        Ok(())
    }

    fn cleanup(&self, out: &mut Vec<u8>) {
        if let Some(kitty) = self.pet.kitty {
            for image in [&kitty.whale, &kitty.fish, &kitty.panel] {
                image.delete(out).expect("Vec write");
            }
        }
    }

    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        if self.food_deadline.is_some_and(|d| cx.now >= d) {
            self.pet.forget_food();
            self.food_deadline = None;
        }
        self.soon = false;
        let frame = self.frame(cx.now);
        let (cols, rows, cell) = (cx.cols, cx.rows, cx.cell);
        self.with_screen(cx, |pet| {
            pet.draw_frame(frame, cols, rows, cell.width, cell.height, out)
        })
    }

    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        let (cols, rows) = (cx.cols, cx.rows);
        self.with_screen(cx, |pet| pet.hide(cols, rows, out));
    }

    fn invalidate(&mut self) {
        self.pet.invalidate();
    }

    fn wake_at(&mut self, now: Instant) -> Option<Instant> {
        let frame = self.frame(now);
        let next = (self.soon || self.pet.moving(frame)).then(|| self.deadline(frame + 1));
        [next, self.pet.fish_refresh_at(now)]
            .into_iter()
            .flatten()
            .min()
    }

    fn hit(&self, p: &Pointer) -> bool {
        self.pet.captures_press(p.kind, (p.col, p.row), p.pixel)
    }

    fn pointer(&mut self, p: Pointer) {
        self.soon |= matches!(p.kind, MouseKind::LeftDrag | MouseKind::LeftRelease);
        self.pet.mouse_event(p.kind, p.col, p.row);
    }

    fn pointer_passed(&mut self, p: Pointer) {
        self.soon |= matches!(p.kind, MouseKind::LeftDrag | MouseKind::LeftRelease);
        self.pet.dismiss_outside(p.kind);
    }

    fn pointer_reset(&mut self) {
        self.pet.cancel_mouse_interaction();
    }

    fn input(&mut self, bytes: &[u8]) {
        self.pet.notice_input();
        if let Some(typed) = &mut self.commands.typed {
            for &byte in bytes {
                if let Some(rule) = typed.input(self.catalog, byte) {
                    self.pet.notice_command(b'e', rule);
                }
            }
        }
    }

    fn output(&mut self, bytes: &[u8], cx: &mut Context<'_>) {
        let Some(change) = self
            .food
            .as_mut()
            .and_then(|food| food.feed(bytes, cx.cols, cx.rows))
        else {
            return;
        };
        match change.filter(|_| !cx.alternate || self.kitty) {
            Some((col, row)) => {
                self.pet.notice_food(col, row);
                self.food_deadline = Some(cx.now + Duration::from_secs(3));
            }
            None => {
                self.pet.forget_food();
                self.food_deadline = None;
            }
        }
    }

    fn bell(&mut self) {
        self.pet.notice_bell();
    }

    fn shell(&mut self, event: ShellEvent<'_>) {
        match event {
            ShellEvent::Prompt => {
                if let Some(typed) = &mut self.commands.typed {
                    typed.prompt();
                }
                if self.commands.preview.take().is_some() {
                    self.notice(b'e', 0);
                }
            }
            ShellEvent::Edit(line) => {
                let rule = self.catalog.editing_line(line);
                if rule != self.commands.preview {
                    self.commands.preview = rule;
                    self.notice(b'e', rule.unwrap_or(0));
                }
            }
            ShellEvent::Exec(line) => {
                // A typed preview ends here; zsh already reported the final edit.
                if self.commands.typed.as_mut().is_some_and(|t| t.end()) {
                    self.notice(b'e', 0);
                }
                let rule = self.catalog.running(line);
                self.commands.active = rule;
                match rule {
                    Some(rule) if rule != 0 => self.notice(b'r', rule),
                    _ if self.commands.preview.is_some() => self.notice(b'e', 0),
                    _ => {}
                }
                self.commands.preview = None;
            }
            ShellEvent::Done(status) => match self.commands.active.take() {
                Some(_) if status == 127 => self.notice(b'n', 0),
                Some(rule) if rule != 0 => {
                    let phase = match status {
                        0 => b'+',
                        130 => b'i',
                        _ => b'-',
                    };
                    self.notice(phase, rule);
                }
                _ => {}
            },
            ShellEvent::Read => {
                if self.commands.typed.as_mut().is_some_and(|t| t.end()) {
                    self.notice(b'e', 0);
                }
            }
        }
    }

    fn resized(&mut self, _cols: u16, _rows: u16) {
        self.pet.forget_food();
        self.food_deadline = None;
        self.pet.invalidate();
    }
}
