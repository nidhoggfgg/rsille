//! Held-key input is used only when the backend can report key releases.
use std::collections::HashSet;

use rsille::tui::prelude::*;

#[derive(Clone)]
pub(crate) struct GameInput {
    pub enhanced: bool,
    pub focused: bool,
    down: HashSet<KeyCode>,
    held: HashSet<KeyCode>,
}

impl Default for GameInput {
    fn default() -> Self {
        Self {
            enhanced: false,
            focused: true,
            down: HashSet::new(),
            held: HashSet::new(),
        }
    }
}

pub(crate) fn normalize(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => code,
    }
}

pub(crate) fn is_motion(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Char('w' | 'a' | 's' | 'd' | ' ' | 'c')
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
    )
}

impl GameInput {
    /// Whether this event may activate an action. Releases always clean up,
    /// including when modifiers changed or gameplay is inactive.
    pub fn key(&mut self, key: KeyEvent, active: bool) -> bool {
        let code = normalize(key.code);
        if key.kind == KeyEventKind::Release {
            self.down.remove(&code);
            self.held.remove(&code);
            return false;
        }
        if self.enhanced {
            if key.kind != KeyEventKind::Press || !self.down.insert(code) {
                return false;
            }
        } else if key.kind == KeyEventKind::Repeat && !is_motion(code) {
            return false;
        }
        if key.modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        ) {
            return false;
        }
        if self.enhanced && active && is_motion(code) {
            self.held.insert(code);
        }
        true
    }

    pub fn clear(&mut self) {
        self.held.clear();
        // Keep physical downs until Release, so repeats (also reported as
        // Press by Windows) cannot restart movement after a state transition.
    }

    pub fn clear_turn(&mut self) {
        self.held.retain(|code| {
            !matches!(
                code,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            )
        });
    }

    /// World units and radians integrated over elapsed seconds, independent of
    /// keyboard repeat settings. Opposing keys cancel before normalization.
    pub fn motion(&self, delta: f64) -> ([f64; 2], [f64; 2], f64) {
        let axis = |positive, negative| {
            f64::from(self.held.contains(&positive)) - f64::from(self.held.contains(&negative))
        };
        let forward = axis(KeyCode::Char('w'), KeyCode::Char('s'));
        let side = axis(KeyCode::Char('d'), KeyCode::Char('a'));
        let length = forward.hypot(side).max(1.0);
        let walk = [forward / length * 6.0 * delta, side / length * 6.0 * delta];
        let turn = [
            axis(KeyCode::Right, KeyCode::Left) * 2.7 * delta,
            axis(KeyCode::Up, KeyCode::Down) * 2.25 * delta,
        ];
        let rise = axis(KeyCode::Char(' '), KeyCode::Char('c')) * 6.0 * delta;
        (walk, turn, rise)
    }
}
