use std::fmt;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rsille::render::area::{Area, Size};

use super::scenes::{ExperienceId, PreviewInteraction};

pub const AUTO_TOUR_INTERVAL_SECS: f64 = 6.5;
pub const TRANSITION_DURATION_SECS: f64 = 0.65;
pub const WIDE_LAYOUT_MIN_WIDTH: u16 = 76;

const INITIAL_PULSE_TIME: f64 = -100.0;

/// Function plotter presets matching the launcher's live preview variants.
const FUNCTION_PLOTTER_PRESETS: [&str; 4] = [
    "surface(sin(x)*cos(y), -pi, pi, -pi, pi)",
    "surface((x^2-y^2)/3, -2, 2, -2, 2)",
    "param3((2+cos(3*t))*cos(2*t), (2+cos(3*t))*sin(2*t), sin(3*t), 0, 2*pi)",
    "polar(1+cos(theta), 0, 2*pi); polar(sin(3*theta), 0, 2*pi)",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperienceSpec {
    pub id: ExperienceId,
    pub icon: &'static str,
    pub title: &'static str,
    pub compact_title: &'static str,
    pub subtitle: &'static str,
    pub badge: &'static str,
}

impl ExperienceSpec {
    pub fn for_id(id: ExperienceId) -> Self {
        match id {
            ExperienceId::Jellyfish => Self {
                id,
                icon: "◎",
                title: "Jellyfish",
                compact_title: "Jellyfish",
                subtitle: "Bioluminescent deep-sea drift",
                badge: "HalfBlock",
            },
            ExperienceId::FunctionPlotter => Self {
                id,
                icon: "ƒ",
                title: "Function Plotter",
                compact_title: "Plotter",
                subtitle: "3D surfaces, curves & REPL",
                badge: "Kitty/Sixel",
            },
            ExperienceId::AiBabies => Self {
                id,
                icon: "✦",
                title: "AI Babies",
                compact_title: "AI Babies",
                subtitle: "Animated pixel trio studio",
                badge: "Quadrant",
            },
            ExperienceId::DeepseekPet => Self {
                id,
                icon: "◆",
                title: "DeepSeek Pet",
                compact_title: "Pet",
                subtitle: "Interactive shell whale pet",
                badge: if cfg!(unix) {
                    "Unix PTY"
                } else if cfg!(windows) {
                    "Win ConPTY"
                } else {
                    "Unsupported"
                },
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchTarget {
    pub experience: ExperienceId,
    pub args: Vec<String>,
    pub display_command: String,
}

impl LaunchTarget {
    pub fn for_experience(exp: ExperienceId, variant: usize) -> Self {
        let count = exp.variant_count().max(1);
        let variant = variant % count;
        let args: Vec<String> = match exp {
            ExperienceId::Jellyfish => vec!["jellyfish".to_owned()],
            ExperienceId::FunctionPlotter => {
                if let Some(&expr) = FUNCTION_PLOTTER_PRESETS.get(variant) {
                    vec!["draw".to_owned(), "--expr".to_owned(), expr.to_owned()]
                } else {
                    vec!["draw".to_owned()]
                }
            }
            ExperienceId::AiBabies => match variant {
                1 => vec![
                    "ai-babies".to_owned(),
                    "--animate".to_owned(),
                    "--character".to_owned(),
                    "chatgpt".to_owned(),
                ],
                2 => vec![
                    "ai-babies".to_owned(),
                    "--animate".to_owned(),
                    "--character".to_owned(),
                    "deepseek".to_owned(),
                ],
                3 => vec![
                    "ai-babies".to_owned(),
                    "--animate".to_owned(),
                    "--character".to_owned(),
                    "claude".to_owned(),
                ],
                _ => vec!["ai-babies".to_owned(), "--animate".to_owned()],
            },
            ExperienceId::DeepseekPet => vec!["pet".to_owned()],
        };

        let display_command = format_display_command(&args);
        Self {
            experience: exp,
            args,
            display_command,
        }
    }
}

fn format_display_command(args: &[String]) -> String {
    let mut out = String::from("rsille");
    for arg in args {
        out.push(' ');
        if arg.contains(' ') || arg.contains(';') || arg.contains('(') || arg.contains('*') {
            out.push('\'');
            out.push_str(arg);
            out.push('\'');
        } else {
            out.push_str(arg);
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LauncherOutcome {
    Quit,
    Launch(LaunchTarget),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionState {
    pub from: ExperienceId,
    pub to: ExperienceId,
    pub progress: f32,
}

/// Interactive hit-test regions computed from the exact layout geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitRegions {
    pub size: Size,
    pub cards: [Area; 4],
    pub stage: Area,
    pub tour_pill: Area,
}

impl HitRegions {
    pub fn card_at(&self, col: u16, row: u16) -> Option<ExperienceId> {
        ExperienceId::ALL
            .into_iter()
            .find(|&exp| area_contains(self.cards[exp.index()], col, row))
    }

    pub fn stage_contains(&self, col: u16, row: u16) -> bool {
        area_contains(self.stage, col, row)
    }

    pub fn tour_pill_contains(&self, col: u16, row: u16) -> bool {
        area_contains(self.tour_pill, col, row)
    }
}

pub fn area_contains(area: Area, col: u16, row: u16) -> bool {
    if area.width() == 0 || area.height() == 0 {
        return false;
    }
    let x0 = area.x();
    let y0 = area.y();
    let x1 = x0.saturating_add(area.width());
    let y1 = y0.saturating_add(area.height());
    col >= x0 && col < x1 && row >= y0 && row < y1
}

/// Pure, deterministic state machine for the inline interactive launcher.
#[derive(Clone, Debug)]
pub struct LauncherState {
    pub selected: ExperienceId,
    pub time: f64,
    pub auto_tour: bool,
    pub tour_elapsed: f64,
    pub interactions: [PreviewInteraction; 4],
    pub transition: Option<TransitionState>,
    pub hovered_card: Option<ExperienceId>,
    pub hovered_stage: bool,
    pub outcome: Option<LauncherOutcome>,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherState {
    pub fn new() -> Self {
        let initial = PreviewInteraction {
            variant: 0,
            action_pulse: INITIAL_PULSE_TIME,
            action_count: 0,
        };
        Self {
            selected: ExperienceId::Jellyfish,
            time: 0.0,
            auto_tour: true,
            tour_elapsed: 0.0,
            interactions: [initial; 4],
            transition: None,
            hovered_card: None,
            hovered_stage: false,
            outcome: None,
        }
    }

    pub fn interaction(&self, exp: ExperienceId) -> PreviewInteraction {
        self.interactions[exp.index()]
    }

    pub fn active_interaction(&self) -> PreviewInteraction {
        self.interaction(self.selected)
    }

    pub fn current_launch_target(&self) -> LaunchTarget {
        let variant = self.active_interaction().variant;
        LaunchTarget::for_experience(self.selected, variant)
    }

    pub fn tour_progress(&self) -> f32 {
        if !self.auto_tour {
            return 0.0;
        }
        (self.tour_elapsed / AUTO_TOUR_INTERVAL_SECS).clamp(0.0, 1.0) as f32
    }

    /// Returns a subtle 0.0..=1.0 pulse intensity right after user interaction.
    pub fn active_pulse_strength(&self) -> f32 {
        let dt = self.time - self.active_interaction().action_pulse;
        if !(0.0..=1.4).contains(&dt) {
            0.0
        } else {
            let t = 1.0 - (dt / 1.4);
            (t * t) as f32
        }
    }

    pub fn should_quit(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn advance(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(0.1);
        self.time += dt;

        if let Some(mut tr) = self.transition {
            tr.progress += (dt / TRANSITION_DURATION_SECS) as f32;
            if tr.progress >= 1.0 {
                self.transition = None;
            } else {
                self.transition = Some(tr);
            }
        }

        if self.auto_tour && self.outcome.is_none() {
            self.tour_elapsed += dt;
            if self.tour_elapsed >= AUTO_TOUR_INTERVAL_SECS {
                self.tour_elapsed %= AUTO_TOUR_INTERVAL_SECS;
                let next = ExperienceId::from_index((self.selected.index() + 1) % 4);
                self.switch_to(next);
            }
        }
    }

    pub fn select(&mut self, target: ExperienceId, manual: bool) {
        if manual {
            self.auto_tour = false;
            self.tour_elapsed = 0.0;
        }
        if target != self.selected {
            self.switch_to(target);
        }
    }

    pub fn select_next(&mut self) {
        let next = ExperienceId::from_index((self.selected.index() + 1) % 4);
        self.select(next, true);
    }

    pub fn select_prev(&mut self) {
        let prev = ExperienceId::from_index((self.selected.index() + 3) % 4);
        self.select(prev, true);
    }

    pub fn cycle_variant(&mut self, delta: isize) {
        let idx = self.selected.index();
        let count = self.selected.variant_count().max(1);
        let current = self.interactions[idx].variant % count;
        let next = (current as isize + delta).rem_euclid(count as isize) as usize;
        self.interactions[idx].variant = next;
        if self.auto_tour {
            self.tour_elapsed = 0.0;
        }
    }

    pub fn trigger_interaction(&mut self) {
        let idx = self.selected.index();
        let count = self.selected.variant_count().max(1);
        let entry = &mut self.interactions[idx];
        entry.action_pulse = self.time;
        entry.action_count = entry.action_count.saturating_add(1);
        if count > 1 {
            entry.variant = (entry.variant + 1) % count;
        }
        if self.auto_tour {
            self.tour_elapsed = 0.0;
        }
    }

    pub fn toggle_auto_tour(&mut self) {
        self.auto_tour = !self.auto_tour;
        self.tour_elapsed = 0.0;
    }

    pub fn request_launch(&mut self) {
        self.outcome = Some(LauncherOutcome::Launch(self.current_launch_target()));
    }

    pub fn request_quit(&mut self) {
        self.outcome = Some(LauncherOutcome::Quit);
    }

    pub fn clear_hover(&mut self) {
        self.hovered_card = None;
        self.hovered_stage = false;
    }

    // --- Deterministic helpers for tests & visual QA ---

    #[allow(dead_code)]
    pub fn select_immediate(&mut self, exp: ExperienceId) {
        self.selected = exp;
        self.transition = None;
    }

    #[allow(dead_code)]
    pub fn set_transition(&mut self, from: ExperienceId, to: ExperienceId, progress: f32) {
        self.selected = to;
        self.transition = Some(TransitionState {
            from,
            to,
            progress: progress.clamp(0.0, 1.0),
        });
    }

    #[allow(dead_code)]
    pub fn set_time(&mut self, time: f64) {
        self.time = time;
    }

    #[allow(dead_code)]
    pub fn set_auto_tour(&mut self, enabled: bool) {
        self.auto_tour = enabled;
    }

    #[allow(dead_code)]
    pub fn set_tour_elapsed(&mut self, elapsed: f64) {
        self.tour_elapsed = elapsed.clamp(0.0, AUTO_TOUR_INTERVAL_SECS);
    }

    #[allow(dead_code)]
    pub fn set_variant(&mut self, exp: ExperienceId, variant: usize) {
        let count = exp.variant_count().max(1);
        self.interactions[exp.index()].variant = variant % count;
    }

    pub fn handle_event(&mut self, event: &Event, regions: &HitRegions) -> bool {
        match event {
            Event::Key(key) => self.handle_key(*key),
            Event::Mouse(mouse) => self.handle_mouse(*mouse, regions),
            _ => false,
        }
    }

    fn switch_to(&mut self, next: ExperienceId) {
        let from = self.selected;
        self.selected = next;
        self.transition = Some(TransitionState {
            from,
            to: next,
            progress: 0.0,
        });
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }
        let is_press = key.kind == KeyEventKind::Press;

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            self.request_quit();
            return true;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') if is_press => {
                self.request_quit();
                true
            }
            KeyCode::Enter if is_press => {
                self.request_launch();
                true
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.select_next();
                true
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.select_prev();
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.cycle_variant(1);
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.cycle_variant(-1);
                true
            }
            KeyCode::Char(' ') if is_press => {
                self.trigger_interaction();
                true
            }
            KeyCode::Char('a') | KeyCode::Char('A') if is_press => {
                self.toggle_auto_tour();
                true
            }
            KeyCode::Char(d @ '1'..='4') if is_press => {
                let idx = (d as u8 - b'1') as usize;
                self.select(ExperienceId::from_index(idx), true);
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, regions: &HitRegions) -> bool {
        match mouse.kind {
            MouseEventKind::Moved => {
                let card = regions.card_at(mouse.column, mouse.row);
                let stage = regions.stage_contains(mouse.column, mouse.row);
                let changed = self.hovered_card != card || self.hovered_stage != stage;
                self.hovered_card = card;
                self.hovered_stage = stage;
                changed
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if regions.tour_pill_contains(mouse.column, mouse.row) {
                    self.toggle_auto_tour();
                    return true;
                }
                if let Some(card) = regions.card_at(mouse.column, mouse.row) {
                    if card == self.selected {
                        self.request_launch();
                    } else {
                        self.select(card, true);
                    }
                    return true;
                }
                if regions.stage_contains(mouse.column, mouse.row) {
                    self.trigger_interaction();
                    return true;
                }
                false
            }
            MouseEventKind::ScrollDown => {
                self.select_next();
                true
            }
            MouseEventKind::ScrollUp => {
                self.select_prev();
                true
            }
            _ => false,
        }
    }
}

impl fmt::Display for LaunchTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn sample_regions() -> HitRegions {
        HitRegions {
            size: (110, 22).into(),
            cards: [
                Area::new((1, 4).into(), (34, 3).into()),
                Area::new((1, 8).into(), (34, 3).into()),
                Area::new((1, 12).into(), (34, 3).into()),
                Area::new((1, 16).into(), (34, 3).into()),
            ],
            stage: Area::new((37, 3).into(), (72, 18).into()),
            tour_pill: Area::new((90, 0).into(), (18, 1).into()),
        }
    }

    #[test]
    fn starts_in_auto_tour_with_clean_pulse_and_advances_worlds() {
        let mut state = LauncherState::new();
        assert_eq!(state.selected, ExperienceId::Jellyfish);
        assert!(state.auto_tour);
        assert_eq!(state.active_pulse_strength(), 0.0);
        assert!(state.transition.is_none());

        // Advance smoothly up to just before AUTO_TOUR_INTERVAL_SECS.
        for _ in 0..64 {
            state.advance(0.1);
        }
        assert_eq!(state.selected, ExperienceId::Jellyfish);
        assert!(state.tour_progress() > 0.9);

        // Two 0.06s steps cleanly cross the 6.5s threshold without f64 0.1 drift.
        state.advance(0.06);
        state.advance(0.06);
        assert_eq!(state.selected, ExperienceId::FunctionPlotter);
        let tr = state
            .transition
            .expect("auto-tour switch starts transition");
        assert_eq!(tr.from, ExperienceId::Jellyfish);
        assert_eq!(tr.to, ExperienceId::FunctionPlotter);

        // Complete transition over TRANSITION_DURATION_SECS.
        for _ in 0..8 {
            state.advance(0.1);
        }
        assert!(state.transition.is_none());
    }

    #[test]
    fn keyboard_navigation_wraps_pauses_tour_and_toggles_back() {
        let mut state = LauncherState::new();
        let regions = sample_regions();

        assert!(state.handle_event(&press(KeyCode::Up), &regions));
        assert_eq!(state.selected, ExperienceId::DeepseekPet);
        assert!(!state.auto_tour);

        assert!(state.handle_event(&press(KeyCode::Tab), &regions));
        assert_eq!(state.selected, ExperienceId::Jellyfish);

        assert!(state.handle_event(&press(KeyCode::Char('3')), &regions));
        assert_eq!(state.selected, ExperienceId::AiBabies);

        assert!(state.handle_event(&press(KeyCode::Char('a')), &regions));
        assert!(state.auto_tour);
        assert_eq!(state.tour_elapsed, 0.0);
    }

    #[test]
    fn space_triggers_pulse_and_cycles_variants_while_arrows_step() {
        let mut state = LauncherState::new();
        let regions = sample_regions();
        state.set_time(2.5);
        state.select_immediate(ExperienceId::AiBabies);

        assert!(state.handle_event(&press(KeyCode::Char(' ')), &regions));
        let info = state.active_interaction();
        assert_eq!(info.action_count, 1);
        assert_eq!(info.action_pulse, 2.5);
        assert!(state.active_pulse_strength() > 0.99);
        assert_eq!(
            info.variant,
            1 % ExperienceId::AiBabies.variant_count().max(1)
        );

        let after_space = info.variant;
        state.handle_event(&press(KeyCode::Left), &regions);
        assert_eq!(state.active_interaction().variant, 0);
        state.handle_event(&press(KeyCode::Right), &regions);
        assert_eq!(state.active_interaction().variant, after_space);
    }

    #[test]
    fn release_events_are_ignored_and_quit_keys_work() {
        let mut state = LauncherState::new();
        let regions = sample_regions();

        let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert!(!state.handle_event(&Event::Key(release), &regions));
        assert!(!state.should_quit());

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(state.handle_event(&Event::Key(ctrl_c), &regions));
        assert_eq!(state.outcome, Some(LauncherOutcome::Quit));
    }

    #[test]
    fn mouse_click_selects_inactive_launches_active_and_interacts_on_stage() {
        let mut state = LauncherState::new();
        let regions = sample_regions();
        state.set_time(1.0);

        let click = |col, row| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };

        // Click card 3 (AI Babies): selects it, does not launch yet.
        assert!(state.handle_event(&click(10, 13), &regions));
        assert_eq!(state.selected, ExperienceId::AiBabies);
        assert!(!state.auto_tour);
        assert!(state.outcome.is_none());

        // Click live stage: triggers interaction & cycles variant to ChatGPT (1).
        assert!(state.handle_event(&click(55, 10), &regions));
        assert_eq!(state.active_interaction().action_count, 1);

        // Click active card 3 again: launches AI Babies!
        assert!(state.handle_event(&click(10, 13), &regions));
        let Some(LauncherOutcome::Launch(target)) = &state.outcome else {
            panic!("expected launch outcome");
        };
        assert_eq!(target.experience, ExperienceId::AiBabies);
        assert_eq!(target.args[0], "ai-babies");
        assert_eq!(target.args[1], "--animate");

        // Wheel navigation works cleanly.
        let mut wheel_state = LauncherState::new();
        let wheel_down = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        });
        assert!(wheel_state.handle_event(&wheel_down, &regions));
        assert_eq!(wheel_state.selected, ExperienceId::FunctionPlotter);
        assert!(!wheel_state.auto_tour);

        // Hover tracking, pointer leave, and clicking the header Auto-Tour pill.
        let move_card = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 17,
            modifiers: KeyModifiers::NONE,
        });
        assert!(wheel_state.handle_event(&move_card, &regions));
        assert_eq!(wheel_state.hovered_card, Some(ExperienceId::DeepseekPet));
        wheel_state.clear_hover();
        assert_eq!(wheel_state.hovered_card, None);

        assert!(wheel_state.handle_event(&click(95, 0), &regions));
        assert!(wheel_state.auto_tour);
    }

    #[test]
    fn launch_targets_match_experience_and_variant() {
        assert_eq!(
            LaunchTarget::for_experience(ExperienceId::Jellyfish, 0).args,
            vec!["jellyfish"]
        );
        assert_eq!(
            LaunchTarget::for_experience(ExperienceId::DeepseekPet, 0).args,
            vec!["pet"]
        );
        assert_eq!(
            LaunchTarget::for_experience(ExperienceId::AiBabies, 0).args,
            vec!["ai-babies", "--animate"]
        );
        assert_eq!(
            LaunchTarget::for_experience(ExperienceId::AiBabies, 2).args,
            vec!["ai-babies", "--animate", "--character", "deepseek"]
        );
        let draw = LaunchTarget::for_experience(ExperienceId::FunctionPlotter, 0);
        assert_eq!(draw.args[0], "draw");
        assert_eq!(draw.args[1], "--expr");
        assert!(draw.display_command.starts_with("rsille draw --expr '"));
    }
}
