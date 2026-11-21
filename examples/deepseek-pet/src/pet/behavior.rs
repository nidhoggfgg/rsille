use crate::{
    animation::{self, Clip},
    food::{Foraging, Treat},
    mouse::Pursuit,
    Feature,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) enum Mood {
    Swim,
    Watch,
    Grab,
    Hop,
    Swoop,
    Bubbles,
    Wave,
    Startled,
    Puzzled,
    Snack,
    EatFish,
    Nap,
    #[default]
    Idle,
}

#[derive(Default)]
pub(super) struct Life {
    anchored: bool,
    pub(super) mood: Mood,
    pub(super) age: u16,
    fatigue: u16,
    playing: bool,
    pub(super) foraging: Option<Foraging>,
    pub(super) treat: Option<Treat>,
    pub(super) cooldown: u16,
    bell_cooldown: u16,
    pub(super) choice: u32,
    pub(super) leftward: bool,
    pub(super) x: u16,
    pub(super) altitude: u16,
    pub(super) pointer: Option<Pursuit>,
    last: Option<u64>,
}

impl Life {
    pub(super) fn new(choice: u32, features: &BTreeSet<Feature>) -> Self {
        Self {
            choice,
            pointer: (features.contains(&Feature::Roam) && features.contains(&Feature::Mouse))
                .then(|| Pursuit::new(choice)),
            foraging: features.contains(&Feature::Forage).then(Foraging::default),
            anchored: !features.contains(&Feature::Roam),
            ..Self::default()
        }
    }

    pub(super) fn react(&mut self, mood: Mood) {
        self.altitude = self.altitude.saturating_add(self.lift());
        self.playing = false;
        self.mood = mood;
        self.age = 0;
        if let Some(pointer) = &mut self.pointer {
            pointer.target = None;
        }
    }

    pub(super) fn play(&mut self, mood: Mood) {
        self.react(mood);
        self.playing = true;
    }

    pub(super) fn pause_at(&mut self, tick: u64) {
        self.last = Some(tick);
    }

    pub(super) fn update(
        &mut self,
        tick: u64,
        travel: u16,
        rise: u16,
        width: u16,
        height: u16,
    ) -> Option<(u16, u16)> {
        let mut eaten = None;
        self.altitude = self.altitude.min(rise);
        let visible =
            |&(c, r): &(u16, u16)| c > 0 && r > 0 && c <= travel + width && r <= rise + height;
        self.treat = self.treat.filter(|t| visible(&t.cell));
        if let Some(foraging) = &mut self.foraging {
            foraging.target = foraging.target.filter(visible);
        }
        if let Some(pointer) = &mut self.pointer {
            pointer.target = pointer.target.filter(visible);
        }
        if self.anchored || self.last.is_none() {
            self.x = travel;
        }
        self.x = self.x.min(travel);
        if self.anchored {
            self.altitude = 0;
        }
        let reachable = |&(col, row): &(u16, u16)| !self.anchored || (col > travel && row > rise);
        // A suspended session resumes near its last pose, without unbounded catch-up.
        let elapsed = tick
            .saturating_sub(self.last.replace(tick).unwrap_or(tick))
            .min(128);
        for _ in 0..elapsed {
            self.age += 1;
            self.cooldown = self.cooldown.saturating_sub(1);
            self.bell_cooldown = self.bell_cooldown.saturating_sub(1);
            let meal = self
                .foraging
                .as_mut()
                .and_then(Foraging::tick)
                .filter(reachable);
            let pointer = self.pointer.as_mut().and_then(Pursuit::tick);
            let reacting =
                self.playing || matches!(self.mood, Mood::Wave | Mood::Startled | Mood::Puzzled);
            let treat = self.treat.filter(|t| reachable(&t.cell));
            let target = treat.map(|t| t.cell).or(meal).or(pointer);
            if let Some((c, r)) = target.filter(|_| !reacting) {
                // Aim pointer interactions at the hands, slightly below the sprite center.
                let hand_target = treat.is_some() || meal.is_none();
                let offset_y = if hand_target {
                    (height / 2 + (height / 8).max(1)).min(height - 1)
                } else {
                    height / 2
                };
                let (x, altitude) = if self.anchored {
                    (travel, 0)
                } else {
                    (
                        (c - 1).saturating_sub(width / 2).min(travel),
                        rise - (r - 1).saturating_sub(offset_y).min(rise),
                    )
                };
                if self.x != x {
                    self.leftward = x < self.x;
                }
                self.x = approach(self.x, x);
                self.altitude = approach(self.altitude, altitude);
                if (self.x, self.altitude) != (x, altitude) {
                    self.mood = Mood::Swim;
                    self.age %= animation::SWIM.duration;
                    continue;
                }
                if let Some(pointer) = &mut self.pointer {
                    pointer.target = None;
                }
                if treat.is_some() {
                    self.treat = None;
                    if let Some(foraging) = &mut self.foraging {
                        foraging.fill_up();
                    }
                    self.playing = true;
                    self.mood = Mood::EatFish;
                } else if let Some(foraging) = self.foraging.as_mut().filter(|_| meal.is_some()) {
                    eaten = foraging.eat();
                    self.mood = Mood::Snack;
                } else {
                    self.playing = true;
                    self.mood = Mood::Grab;
                }
                self.age = 0;
                continue;
            }
            if !reacting && !matches!(self.mood, Mood::Snack | Mood::Idle) {
                self.altitude = self.altitude.saturating_sub(1);
            }
            if !self.anchored && matches!(self.mood, Mood::Swim | Mood::Swoop) {
                self.fatigue = self.fatigue.saturating_add(1);
                if self.mood == Mood::Swoop || self.age % 2 == 0 {
                    if self.x == 0 || self.x == travel {
                        self.leftward = self.x != 0;
                    }
                    self.x = if self.leftward {
                        self.x.saturating_sub(1)
                    } else {
                        self.x.saturating_add(1).min(travel)
                    };
                }
            }
            let duration = match self.mood {
                Mood::Swim => 48 + (self.choice % 49) as u16,
                Mood::Idle => 70 + (self.choice % 81) as u16,
                Mood::Swoop => 40,
                _ => self.clip().duration,
            };
            if self.age >= duration {
                self.playing = false;
                if self.mood == Mood::Nap {
                    self.fatigue = 0;
                }
                if matches!(self.mood, Mood::Snack | Mood::EatFish) {
                    if let Some(foraging) = &mut self.foraging {
                        foraging.hunger = 0;
                    }
                }
                self.choice = self.choice.wrapping_mul(1664525).wrapping_add(1013904223);
                self.mood = if self.fatigue >= 160 {
                    Mood::Nap
                } else if !self.anchored && self.mood == Mood::Idle {
                    let actions = [
                        Mood::Swim,
                        Mood::Watch,
                        Mood::Swoop,
                        Mood::Hop,
                        Mood::Bubbles,
                    ];
                    actions[(self.choice >> 16) as usize % actions.len()]
                } else {
                    Mood::Idle
                };
                self.age = 0;
            }
        }
        eaten
    }

    pub(super) fn notice_command(&mut self, phase: crate::dialogue::Phase) {
        use crate::dialogue::Phase;
        self.react(match phase {
            Phase::Editing => Mood::Watch,
            Phase::Running => Mood::Swim,
            Phase::Done => Mood::Wave,
            Phase::Failed => Mood::Startled,
            Phase::Missing => Mood::Puzzled,
            Phase::Interrupted => Mood::Idle,
        });
    }

    pub(super) fn notice_bell(&mut self) {
        if self.bell_cooldown == 0 && self.mood != Mood::Wave {
            self.react(Mood::Startled);
            self.bell_cooldown = 60;
        }
    }

    /// Notice a mouse position in one-based terminal cells.
    pub(super) fn notice_mouse(&mut self, col: u16, row: u16) {
        if self.treat.is_some() {
            return;
        }
        if let Some(pointer) = &mut self.pointer {
            let receptive = matches!(self.mood, Mood::Swim | Mood::Idle | Mood::Watch);
            pointer.motion((col, row), receptive, &mut self.cooldown);
        }
    }

    /// Occasionally look up. Key repeats cannot restart the gesture.
    pub(super) fn notice_input(&mut self) {
        if self.cooldown == 0 && matches!(self.mood, Mood::Idle | Mood::Swim) {
            self.mood = Mood::Watch;
            self.age = 0;
            self.cooldown = 120;
        }
    }

    fn clip(&self) -> &'static Clip {
        match self.mood {
            Mood::Swim | Mood::Swoop => &animation::SWIM,
            Mood::Bubbles => &animation::BUBBLES,
            Mood::Wave => &animation::WAVE,
            Mood::Puzzled if self.leftward => &animation::PUZZLED_LEFT,
            Mood::Puzzled => &animation::PUZZLED,
            Mood::Startled => &animation::STARTLED,
            Mood::Hop => &animation::HOP,
            Mood::Watch => &animation::WATCH,
            Mood::Grab => &animation::GRAB,
            Mood::Snack | Mood::EatFish => &animation::SNACK,
            Mood::Nap => &animation::NAP,
            Mood::Idle if self.choice & 1 != 0 => &animation::IDLE_FIN,
            Mood::Idle => &animation::IDLE,
        }
    }

    pub(super) fn frame(&self) -> usize {
        let age = if self.mood == Mood::Idle {
            self.age + (self.choice % 21) as u16
        } else {
            self.age
        };
        self.clip().frame(age)
    }

    pub(super) fn lift(&self) -> u16 {
        if self.anchored {
            return 0;
        }
        // Keep the motion in step with the clip even when its frame timings change.
        let arc = |heights: &[u16]| {
            let phase = usize::from(self.age) * heights.len() / usize::from(self.clip().duration);
            heights[phase.min(heights.len() - 1)]
        };
        match self.mood {
            Mood::Swim => self.age / 6 % 2,
            Mood::Startled => arc(&[0, 3, 5, 4, 2, 0]),
            Mood::Hop => arc(&[0, 0, 1, 3, 3, 1, 0, 0]),
            Mood::Swoop => {
                let phase = f32::from(self.age) / 39.0;
                let amplitude = 5 + self.choice % 9;
                ((phase * std::f32::consts::PI).sin() * amplitude as f32).round() as u16
            }
            _ => 0,
        }
    }
}

fn approach(value: u16, goal: u16) -> u16 {
    if value < goal {
        value + 1
    } else if value > goal {
        value - 1
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::SIDE;

    #[test]
    fn idle_gestures_leave_the_whale_in_place() {
        for (choice, duration) in [(0, 70), (1, 71), (80, 150)] {
            let mut life = Life {
                choice,
                altitude: 5,
                ..Life::default()
            };
            let mut still = 0;
            let mut blinked = false;
            let mut moved_fin = false;
            for tick in 0..duration {
                life.update(tick, 60, 10, 1, 1);
                assert_eq!((life.mood, life.x, life.altitude), (Mood::Idle, 60, 5));
                assert_eq!(life.lift(), 0);
                match life.frame() {
                    f if f == animation::IDLE.frame(0) => still += 1,
                    f if f == animation::IDLE.frame(32) => blinked = true,
                    f if f == animation::IDLE_FIN.frame(60) => moved_fin = true,
                    other => panic!("unexpected idle frame {other}"),
                }
            }
            assert!(still > duration * 9 / 10 && blinked);
            assert_eq!(moved_fin, choice & 1 != 0);
            life.update(duration, 60, 10, 1, 1);
            assert_ne!(life.mood, Mood::Idle);
        }
    }

    #[test]
    fn undisturbed_whale_blows_rising_bubbles() {
        let mut life = Life::default();
        for tick in 0..2000 {
            life.update(tick, 60, 10, 1, 1);
            if life.mood == Mood::Bubbles {
                let has_bubble = |frame, x: usize, y: usize| {
                    let px: usize = (2 * x + 1) * SIDE / (2 * 96);
                    let py: usize = (2 * y + 1) * SIDE / (2 * 48);
                    let i = (py * SIDE + px) * 4;
                    animation::pixels(frame)[i..i + 4] == [146, 255, 255, 255]
                };
                assert!(has_bubble(life.frame(), 82, 25));
                life.update(tick + 10, 60, 10, 1, 1);
                assert!(has_bubble(life.frame(), 87, 10));
                life.update(tick + 20, 60, 10, 1, 1);
                assert_ne!(life.mood, Mood::Bubbles);
                return;
            }
        }
        panic!("the whale never blew bubbles");
    }

    #[test]
    fn fed_pet_uses_every_action_and_stays_in_bounds() {
        let mut life = Life::new(0, &[Feature::Forage, Feature::Roam].into());
        let actions = [
            Mood::Idle,
            Mood::Snack,
            Mood::Swim,
            Mood::Hop,
            Mood::Watch,
            Mood::Nap,
        ];
        let mut seen = [false; 6];
        for tick in 0..12000 {
            let travel = (tick / 31 % 8) as u16;
            if tick % 600 == 0 {
                life.foraging.as_mut().unwrap().target = Some((1, 1));
            }
            life.update(tick, travel, 0, 1, 1);
            if let Some(i) = actions.iter().position(|&mood| mood == life.mood) {
                seen[i] = true;
            }
            assert!(life.x <= travel);
        }
        assert_eq!(seen, [true; 6]);
    }

    #[test]
    fn swoop_travels_while_rising_and_falling() {
        let mut life = Life {
            mood: Mood::Swoop,
            ..Life::default()
        };
        life.update(0, 100, 20, 1, 1);
        life.update(20, 100, 20, 1, 1);
        assert_eq!(life.x, 80);
        assert!(life.lift() >= 5);
        life.update(39, 100, 20, 1, 1);
        assert_eq!((life.x, life.lift()), (61, 0));
    }
}
