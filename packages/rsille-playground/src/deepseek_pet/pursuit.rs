//! Pointer chasing for a roaming pet.

/// Tracks the latest pointer position once curiosity starts a pursuit.
#[derive(Default)]
pub struct Pursuit {
    pub target: Option<(u16, u16)>,
    pub curiosity: u32,
}

impl Pursuit {
    pub fn motion(&mut self, cell: (u16, u16), receptive: bool, cooldown: &mut u16) {
        if self.target.is_some() {
            self.target = Some(cell);
        } else if receptive && *cooldown == 0 {
            self.curiosity = self
                .curiosity
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            *cooldown = 8;
            // A quarter of eligible pointer movements start a chase.
            if self.curiosity >> 30 == 0 {
                *cooldown = 48;
                self.target = Some(cell);
            }
        }
    }

    pub fn new(curiosity: u32) -> Self {
        Self {
            curiosity,
            ..Self::default()
        }
    }
}
