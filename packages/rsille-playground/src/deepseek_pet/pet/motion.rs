//! Sample cell-based behavior at pixel resolution between animation ticks.
use super::{Mood, FRAMES_PER_TICK};

#[derive(Default)]
struct Axis {
    from: u32,
    to: u32,
    start: u64,
    frames: u64,
}

impl Axis {
    fn sample(&self, frame: u64) -> u32 {
        if self.frames == 0 {
            return self.to;
        }
        let step = (frame.saturating_sub(self.start) + 1).min(self.frames);
        ((u64::from(self.from) * (self.frames - step)
            + u64::from(self.to) * step
            + self.frames / 2)
            / self.frames) as u32
    }

    fn aim(&mut self, frame: u64, start: u64, to: u32, frames: u64) -> u32 {
        if frames == 0 || self.to != to {
            self.from = self.sample(frame.saturating_sub(1));
            self.to = to;
            self.start = start;
            self.frames = frames;
        }
        self.sample(frame)
    }

    fn moving(&self, frame: u64) -> bool {
        self.sample(frame) != self.to
    }
}

pub(super) enum Movement {
    Step,
    SlowSwim,
    Drag,
    Paused,
}

#[derive(Default)]
pub(super) struct Motion {
    x: Axis,
    y: Axis,
    last_frame: u64,
    // Pose and pixel lift applied to the last displayed base position.
    lift: Option<(Mood, u32)>,
    viewport: Option<(u16, u16, u16, u16)>,
}

impl Motion {
    pub(super) fn position(
        &mut self,
        frame: u64,
        target: (u16, u16),
        viewport: (u16, u16, u16, u16),
        movement: Movement,
        mut lift: Option<(Mood, u32)>,
    ) -> (u32, u32) {
        let resized = self.viewport.replace(viewport) != Some(viewport);
        let (_, _, cw, ch) = viewport;
        let (mut x, mut y) = (
            u32::from(target.0) * u32::from(cw),
            u32::from(target.1) * u32::from(ch),
        );
        let last_y = self
            .y
            .sample(self.last_frame)
            .saturating_sub(self.lift.map_or(0, |(_, h)| h));
        let mut start = frame - frame % FRAMES_PER_TICK;
        let (horizontal, vertical) = if resized {
            (0, 0)
        } else {
            match movement {
                Movement::Paused => {
                    x = self.x.sample(self.last_frame);
                    y = last_y;
                    lift = None;
                    (0, 0)
                }
                Movement::Drag => {
                    // Pointer reports can arrive between behavior ticks.
                    start = frame;
                    (FRAMES_PER_TICK, FRAMES_PER_TICK)
                }
                Movement::SlowSwim => (2 * FRAMES_PER_TICK, FRAMES_PER_TICK),
                Movement::Step => (FRAMES_PER_TICK, FRAMES_PER_TICK),
            }
        };
        let height = lift.map_or(0, |(_, h)| h);
        let changed_curve = self.lift.map(|(mood, _)| mood) != lift.map(|(mood, _)| mood);
        let mut vertical_start = start;
        if !resized
            && vertical != 0
            && changed_curve
            && frame.saturating_sub(self.last_frame) <= FRAMES_PER_TICK
        {
            // A click or command can change the pose mid-flight. Begin its base
            // transition at the visible height, including the outgoing pose's lift.
            self.y = Axis {
                to: last_y + height,
                ..Axis::default()
            };
            vertical_start = frame;
        }
        self.lift = lift;
        self.last_frame = frame;
        (
            self.x.aim(frame, start, x, horizontal),
            self.y
                .aim(frame, vertical_start, y, vertical)
                .saturating_sub(height),
        )
    }

    pub(super) fn moving(&self, frame: u64) -> bool {
        self.lift.is_some() || self.x.moving(frame) || self.y.moving(frame)
    }
}
