//! Small, deterministic voxel world. All game rules live outside the renderer.
use std::sync::Arc;

pub const SIZE: i32 = 64;
pub const HEIGHT: i32 = 32;
pub type Cell = [i32; 3];
pub type Vec3 = [f64; 3];
pub const REACH: f64 = 6.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Block {
    #[default]
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Wood,
    Leaves,
    Planks,
    Glass,
    Water,
    Bedrock,
    Wool,
    Coal,
}

impl Block {
    pub fn name(self) -> &'static str {
        match self {
            Self::Air => "Air",
            Self::Grass => "Grass",
            Self::Dirt => "Dirt",
            Self::Stone => "Stone",
            Self::Sand => "Sand",
            Self::Wood => "Oak log",
            Self::Leaves => "Oak leaves",
            Self::Planks => "Oak planks",
            Self::Glass => "Glass",
            Self::Water => "Water",
            Self::Bedrock => "Bedrock",
            Self::Wool => "Wool",
            Self::Coal => "Coal",
        }
    }

    pub fn solid(self) -> bool {
        !matches!(self, Self::Air | Self::Water)
    }
}

pub const HOTBAR: [Block; 8] = [
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Wood,
    Block::Planks,
    Block::Leaves,
    Block::Sand,
    Block::Glass,
];

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub cell: Cell,
    pub normal: Cell,
    pub point: Vec3,
    pub distance: f64,
    pub block: Block,
}

/// Legacy terminals send repeat events, not reliable key releases. Keep a small,
/// bounded amount of unfinished input, and consume it over successive frames.
#[derive(Clone, Default)]
pub(crate) struct KeyboardMotion {
    walk: [f64; 2],
    turn: [f64; 2],
    rise: f64,
}

impl KeyboardMotion {
    fn add_pair(pending: &mut [f64; 2], input: [f64; 2], limit: f64) {
        for i in 0..2 {
            pending[i] += input[i];
        }
        let length = pending[0].hypot(pending[1]);
        if length > limit {
            for value in pending {
                *value *= limit / length;
            }
        }
    }

    fn take(&mut self, delta: f64) -> ([f64; 2], [f64; 2], f64) {
        // Exact exponential integration: the response depends on elapsed time,
        // not frame count. About 94% of a tap completes within 150 ms.
        let fraction = -(-delta / 0.055).exp_m1();
        let drain = |pending: &mut f64| {
            let amount = *pending * fraction;
            *pending -= amount;
            amount
        };
        let walk = self.walk.each_mut().map(drain);
        let turn = self.turn.each_mut().map(drain);
        (walk, turn, drain(&mut self.rise))
    }
}

#[derive(Clone)]
pub struct World {
    pub(crate) blocks: Arc<Vec<Block>>,
    pub(crate) motion: KeyboardMotion,
    /// Player's feet; eye height is added when casting rays.
    pub player: Vec3,
    pub yaw: f64,
    pub pitch: f64,
    pub velocity: f64,
    pub flying: bool,
    pub selected: usize,
    pub time: f64,
    pub night: bool,
    pub paused: bool,
    pub help: bool,
    pub notice: String,
    pub notice_time: f64,
    pub swing: f64,
    pub mined: u32,
    pub placed: u32,
}

pub fn terrain(x: i32, z: i32) -> i32 {
    (5.5 + (x as f64 * 0.13).sin() * 1.5
        + (z as f64 * 0.16).cos() * 1.3
        + ((x + z) as f64 * 0.24).sin() * 0.6)
        .floor() as i32
}

impl Default for World {
    fn default() -> Self {
        let mut world = Self {
            blocks: Arc::new(vec![Block::Air; (SIZE * SIZE * HEIGHT) as usize]),
            motion: KeyboardMotion::default(),
            player: [24.5, 9.0, 10.5],
            yaw: 0.50,
            pitch: -0.06,
            velocity: 0.0,
            flying: false,
            selected: 0,
            time: 0.0,
            night: false,
            paused: false,
            help: false,
            notice: "A little world. Make it yours.   [?] controls".into(),
            notice_time: 6.0,
            swing: 0.0,
            mined: 0,
            placed: 0,
        };
        // Keep one mutable borrow during generation; frame snapshots share the storage.
        let blocks = Arc::make_mut(&mut world.blocks);
        let mut put = |x, y, z, block| {
            if let Some(i) = index([x, y, z]) {
                blocks[i] = block;
            }
        };
        for z in 0..SIZE {
            for x in 0..SIZE {
                let lake = ((x - 33) as f64 / 8.0).powi(2) + ((z - 29) as f64 / 10.0).powi(2);
                let mut top = terrain(x, z);
                if lake < 1.0 {
                    top = 2;
                } else if lake < 1.5 {
                    top = top.max(4);
                }
                if (39..=49).contains(&x) && (24..=35).contains(&z) {
                    top = 5;
                }
                for y in 0..=top {
                    let block = if y == 0 {
                        Block::Bedrock
                    } else if y < top - 2 {
                        Block::Stone
                    } else if lake < 1.35 {
                        Block::Sand
                    } else if y == top {
                        Block::Grass
                    } else {
                        Block::Dirt
                    };
                    put(x, y, z, block);
                }
                if lake < 1.0 {
                    for y in top + 1..=4 {
                        put(x, y, z, Block::Water);
                    }
                }
            }
        }
        // A small oak cabin: open doorway, windows, stepped roof and a chimney.
        for z in 26..=32 {
            for x in 40..=47 {
                put(x, 6, z, Block::Planks);
                for y in 7..=10 {
                    if x == 40 || x == 47 || z == 26 || z == 32 {
                        let door = z == 26 && (43..=44).contains(&x) && y <= 8;
                        let window = (y == 8 || y == 9)
                            && ((z == 26 && (x == 41 || x == 46)) || (x == 40 && z == 29));
                        put(
                            x,
                            y,
                            z,
                            if door {
                                Block::Air
                            } else if window {
                                Block::Glass
                            } else if (x == 40 || x == 47) && (z == 26 || z == 32) {
                                Block::Wood
                            } else {
                                Block::Planks
                            },
                        );
                    }
                }
            }
        }
        for level in 0..=3 {
            for x in 39..=48 {
                for z in 25 + level..=33 - level {
                    put(x, 11 + level, z, Block::Wood);
                }
            }
        }
        for y in 11..=16 {
            put(46, y, 30, Block::Stone);
        }
        // Trees have chunky canopies and visible trunks, like the original voxel game.
        for (x, z) in [
            (25, 22),
            (16, 28),
            (13, 39),
            (22, 44),
            (38, 43),
            (49, 19),
            (52, 39),
            (8, 18),
            (10, 51),
            (47, 52),
            (32, 52),
            (55, 50),
        ] {
            let ground = terrain(x, z);
            for y in ground + 1..=ground + 5 {
                world.set([x, y, z], Block::Wood);
            }
            for dy in 3_i32..=6 {
                let radius: i32 = if dy == 6 { 1 } else { 2 };
                for dx in -radius..=radius {
                    for dz in -radius..=radius {
                        if dx.abs() == 2 && dz.abs() == 2 {
                            continue;
                        }
                        let cell = [x + dx, ground + dy, z + dz];
                        if world.get(cell) == Block::Air {
                            world.set(cell, Block::Leaves);
                        }
                    }
                }
            }
        }
        // A blocky sheep on the near shore (a tiny static voxel sculpture).
        let (x, z) = (31, 18);
        let y = (1..HEIGHT - 3)
            .rev()
            .find(|&y| world.get([x, y - 1, z]).solid())
            .unwrap_or(1);
        for dx in 0..=1 {
            for dz in 0..=2 {
                world.set([x + dx, y + 1, z + dz], Block::Wool);
                if dz != 1 {
                    world.set([x + dx, y, z + dz], Block::Coal);
                }
            }
        }
        world.set([x, y + 1, z - 1], Block::Coal);
        world.set([x + 1, y + 1, z - 1], Block::Wool);
        world.player[1] = (terrain(24, 10) + 1) as f64;
        world
    }
}

fn index([x, y, z]: Cell) -> Option<usize> {
    if (0..SIZE).contains(&x) && (0..HEIGHT).contains(&y) && (0..SIZE).contains(&z) {
        Some((y * SIZE * SIZE + z * SIZE + x) as usize)
    } else {
        None
    }
}

impl World {
    pub fn get(&self, cell: Cell) -> Block {
        index(cell).map_or(Block::Air, |i| self.blocks[i])
    }

    pub fn set(&mut self, cell: Cell, block: Block) {
        if let Some(i) = index(cell) {
            Arc::make_mut(&mut self.blocks)[i] = block;
        }
    }

    pub fn eye(&self) -> Vec3 {
        [self.player[0], self.player[1] + 1.62, self.player[2]]
    }

    pub fn direction(&self) -> Vec3 {
        [
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ]
    }

    /// Amanatides-Woo traversal. Zero direction components never step that axis.
    pub fn ray(&self, origin: Vec3, direction: Vec3, reach: f64, water: bool) -> Option<Hit> {
        let mut cell = origin.map(|v| v.floor() as i32);
        let step = direction.map(|v| if v > 0.0 { 1 } else { -1 });
        let delta = direction.map(|v| {
            if v.abs() < 1e-12 {
                f64::INFINITY
            } else {
                1.0 / v.abs()
            }
        });
        let mut next: Vec3 = std::array::from_fn(|i| {
            if direction[i].abs() < 1e-12 {
                f64::INFINITY
            } else {
                ((cell[i] + i32::from(step[i] > 0)) as f64 - origin[i]) / direction[i]
            }
        });
        let (mut distance, mut normal) = (0.0, [0; 3]);
        for _ in 0..256 {
            if distance > reach || index(cell).is_none() {
                return None;
            }
            let block = self.get(cell);
            if block.solid() || (water && block == Block::Water) {
                return Some(Hit {
                    cell,
                    normal,
                    distance,
                    block,
                    point: std::array::from_fn(|i| origin[i] + direction[i] * distance),
                });
            }
            let axis = if next[0] < next[1] && next[0] < next[2] {
                0
            } else if next[1] < next[2] {
                1
            } else {
                2
            };
            distance = next[axis];
            cell[axis] += step[axis];
            next[axis] += delta[axis];
            normal = [0; 3];
            normal[axis] = -step[axis];
        }
        None
    }

    pub fn target(&self) -> Option<Hit> {
        self.ray(self.eye(), self.direction(), REACH, false)
    }

    pub fn active(&self) -> bool {
        !self.paused && !self.help
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.notice = text.into();
        self.notice_time = 2.5;
    }

    pub fn mine(&mut self) {
        if !self.active() {
            return;
        }
        self.swing = 1.0;
        if let Some(hit) = self.target() {
            if hit.block == Block::Bedrock {
                self.say("Bedrock is unbreakable.");
                return;
            }
            self.set(hit.cell, Block::Air);
            self.mined += 1;
            self.say(format!("Mined {}", hit.block.name()));
        } else {
            self.say("Aim at a block within 6 blocks.");
        }
    }

    pub fn place(&mut self) {
        if !self.active() {
            return;
        }
        if let Some(hit) = self.target() {
            let cell = std::array::from_fn(|i| hit.cell[i] + hit.normal[i]);
            if index(cell).is_none() {
                self.say("Edge of this little world.");
                return;
            }
            if self.get(cell).solid() || self.overlaps(cell) {
                self.say("Leave some room for yourself.");
                return;
            }
            let block = HOTBAR[self.selected];
            self.set(cell, block);
            self.placed += 1;
            self.swing = 1.0;
            self.say(format!("Placed {}", block.name()));
        } else {
            self.say("Aim at a nearby face to place a block.");
        }
    }

    fn overlaps(&self, cell: Cell) -> bool {
        self.player[0] + 0.27 > cell[0] as f64
            && self.player[0] - 0.27 < cell[0] as f64 + 1.0
            && self.player[1] + 1.8 > cell[1] as f64
            && self.player[1] < cell[1] as f64 + 1.0
            && self.player[2] + 0.27 > cell[2] as f64
            && self.player[2] - 0.27 < cell[2] as f64 + 1.0
    }

    pub fn free(&self, feet: Vec3) -> bool {
        if feet[0] < 0.3
            || feet[2] < 0.3
            || feet[0] > SIZE as f64 - 0.3
            || feet[2] > SIZE as f64 - 0.3
            || feet[1] < 1.0
            || feet[1] > HEIGHT as f64 - 2.0
        {
            return false;
        }
        for x in [
            (feet[0] - 0.27).floor() as i32,
            (feet[0] + 0.27).floor() as i32,
        ] {
            for z in [
                (feet[2] - 0.27).floor() as i32,
                (feet[2] + 0.27).floor() as i32,
            ] {
                for y in feet[1].floor() as i32..=(feet[1] + 1.79).floor() as i32 {
                    if self.get([x, y, z]).solid() {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn walk(&mut self, forward: f64, side: f64) {
        if !self.active() {
            return;
        }
        KeyboardMotion::add_pair(&mut self.motion.walk, [forward * 0.36, side * 0.36], 0.72);
    }

    /// Keyboard turns are smoothed separately from direct mouse look.
    pub fn turn(&mut self, yaw: f64, pitch: f64) {
        if self.active() {
            KeyboardMotion::add_pair(&mut self.motion.turn, [yaw, pitch], 0.18);
        }
    }

    pub fn stop_input(&mut self) {
        self.motion = KeyboardMotion::default();
    }

    fn move_horizontal(&mut self, forward: f64, side: f64) {
        let dx = self.yaw.sin() * forward + self.yaw.cos() * side;
        let dz = self.yaw.cos() * forward - self.yaw.sin() * side;
        // Resolve axes independently so walking into a wall slides along it.
        for (axis, delta) in [(0, dx), (2, dz)] {
            let mut next = self.player;
            next[axis] += delta;
            if self.free(next) {
                self.player = next;
            } else if !self.flying && self.grounded() {
                next[1] += 1.01;
                if self.free(next) {
                    self.player = next;
                }
            }
        }
    }

    fn grounded(&self) -> bool {
        let mut below = self.player;
        below[1] -= 0.06;
        !self.free(below)
    }

    pub fn jump(&mut self) {
        if !self.active() {
            return;
        }
        if self.flying {
            self.vertical(0.5);
        } else if self.grounded() {
            self.velocity = 7.0;
        }
    }

    pub fn vertical(&mut self, amount: f64) {
        if !self.active() || !self.flying {
            return;
        }
        self.motion.rise = (self.motion.rise + amount).clamp(-1.0, 1.0);
    }

    fn move_vertical(&mut self, amount: f64) {
        let mut next = self.player;
        next[1] += amount;
        if self.free(next) {
            self.player = next;
        }
    }

    pub fn look(&mut self, yaw: f64, pitch: f64) {
        if self.active() {
            self.motion.turn = [0.0; 2];
            self.yaw += yaw;
            self.pitch = (self.pitch + pitch).clamp(-1.45, 1.45);
        }
    }

    pub fn advance(&mut self, delta: f64) {
        if !self.active() {
            return;
        }
        let delta = delta.clamp(0.0, 0.08);
        self.time += delta;
        self.notice_time = (self.notice_time - delta).max(0.0);
        self.swing = (self.swing - delta * 4.0).max(0.0);
        let (walk, turn, rise) = self.motion.take(delta);
        self.yaw += turn[0];
        self.pitch = (self.pitch + turn[1]).clamp(-1.45, 1.45);
        // Consume input even when movement is blocked, so walls cannot store up
        // a later lurch. Substeps also keep rapid/batched repeats collision safe.
        for _ in 0..4 {
            self.move_horizontal(walk[0] / 4.0, walk[1] / 4.0);
            if self.flying {
                self.move_vertical(rise / 4.0);
            } else {
                let dt = delta / 4.0;
                self.velocity = (self.velocity - 20.0 * dt).max(-18.0);
                let mut next = self.player;
                next[1] += self.velocity * dt;
                if self.free(next) {
                    self.player = next;
                } else {
                    self.velocity = 0.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn airborne() -> World {
        World {
            player: [28.5, 20.0, 17.5],
            yaw: 0.0,
            pitch: 0.0,
            flying: true,
            ..World::default()
        }
    }

    #[test]
    fn keyboard_taps_are_smooth_and_follow_elapsed_time() {
        for fps in [30, 60, 120] {
            let mut w = airborne();
            let start = w.player;
            w.walk(1.0, 0.0);
            w.turn(0.0, 0.075);
            w.vertical(0.5);
            assert_eq!(w.player, start);
            assert_eq!(w.pitch, 0.0);
            let mut previous = start;
            for _ in 0..fps / 2 {
                w.advance(1.0 / fps as f64);
                let step = w.player[2] - previous[2];
                assert!(step > 0.0 && step < 0.18, "a tap must span multiple frames");
                previous = w.player;
            }
            assert!((w.player[2] - start[2] - 0.36).abs() < 0.0001);
            assert!((w.player[1] - start[1] - 0.5).abs() < 0.0001);
            assert!((w.pitch - 0.075).abs() < 0.0001);
        }
    }

    #[test]
    fn repeated_input_moves_between_events_and_cannot_build_a_long_backlog() {
        let mut w = airborne();
        // Simulate 30 keyboard repeats per second, rendered at 60 FPS.
        for frame in 0..30 {
            if frame % 2 == 0 {
                w.walk(1.0, 0.0);
                w.turn(0.09, 0.0);
            }
            let before = w.player;
            let yaw = w.yaw;
            w.advance(1.0 / 60.0);
            let distance = (w.player[0] - before[0]).hypot(w.player[2] - before[2]);
            assert!(distance > 0.0 && distance < 0.23);
            assert!(w.yaw > yaw && w.yaw - yaw < 0.06);
        }
        // An input burst must stop after a short, bounded tail.
        w.stop_input();
        let start = w.player;
        for _ in 0..200 {
            w.walk(1.0, 0.0);
            w.turn(0.09, 0.0);
        }
        let yaw = w.yaw;
        for _ in 0..60 {
            w.advance(1.0 / 60.0);
        }
        assert!((w.player[0] - start[0]).hypot(w.player[2] - start[2]) <= 0.721);
        assert!(w.yaw - yaw <= 0.181);
        let stopped = w.player;
        w.advance(1.0 / 60.0);
        assert!((w.player[2] - stopped[2]).abs() < 0.000001);
    }

    #[test]
    fn mouse_look_cancels_unfinished_keyboard_turns() {
        let mut w = airborne();
        w.turn(0.09, 0.075);
        w.look(0.14, -0.13);
        assert_eq!(w.yaw, 0.14);
        assert_eq!(w.pitch, -0.13);
        w.advance(1.0 / 60.0);
        assert_eq!(w.yaw, 0.14);
        assert_eq!(w.pitch, -0.13);
    }

    #[test]
    fn ray_faces_and_reach() {
        let mut w = World::default();
        w.set([28, 20, 20], Block::Stone);
        let hit = w
            .ray([28.5, 20.5, 17.5], [0.0, 0.0, 1.0], 6.0, false)
            .unwrap();
        assert_eq!(hit.cell, [28, 20, 20]);
        assert_eq!(hit.normal, [0, 0, -1]);
        assert_eq!(hit.distance, 2.5);
        assert!(w
            .ray([28.5, 20.5, 17.5], [0.0, 0.0, 1.0], 2.0, false)
            .is_none());
        let reverse = w
            .ray([28.5, 20.5, 23.5], [0.0, 0.0, -1.0], 6.0, false)
            .unwrap();
        assert_eq!(reverse.normal, [0, 0, 1]);
    }

    #[test]
    fn editing_uses_adjacent_face_and_keeps_snapshots() {
        let mut w = World {
            player: [28.5, 18.88, 17.5],
            yaw: 0.0,
            pitch: 0.0,
            ..World::default()
        };
        w.set([28, 20, 20], Block::Stone);
        let snapshot = w.clone();
        w.selected = 3;
        w.place();
        assert_eq!(w.get([28, 20, 19]), Block::Wood);
        assert_eq!(snapshot.get([28, 20, 19]), Block::Air);
        w.mine();
        assert_eq!(w.get([28, 20, 19]), Block::Air);
        assert_eq!(w.get([28, 20, 20]), Block::Stone);
        w.set([28, 20, 20], Block::Bedrock);
        w.mine();
        assert_eq!(w.get([28, 20, 20]), Block::Bedrock);
    }

    #[test]
    fn physics_floor_jump_and_pause() {
        let mut w = World::default();
        let start = w.player;
        assert!(w.free(start));
        for _ in 0..120 {
            w.advance(1.0 / 60.0);
        }
        assert!((w.player[1] - start[1]).abs() < 0.08);
        w.jump();
        for _ in 0..10 {
            w.advance(1.0 / 60.0);
        }
        assert!(w.player[1] > start[1] + 0.6);
        for _ in 0..120 {
            w.advance(1.0 / 60.0);
        }
        assert!((w.player[1] - start[1]).abs() < 0.08);
        w.help = true;
        let before = w.player;
        let time = w.time;
        w.advance(0.08);
        w.walk(1.0, 0.0);
        w.mine();
        assert_eq!(w.player, before);
        assert_eq!(w.time, time);
        assert_eq!(w.mined, 0);
    }

    #[test]
    fn placement_cannot_trap_player_and_walls_block_movement() {
        let mut w = World {
            player: [28.5, 19.0, 18.5],
            yaw: 0.0,
            pitch: 0.0,
            flying: true,
            ..World::default()
        };
        w.set([28, 20, 19], Block::Stone);
        w.place();
        assert_eq!(w.get([28, 20, 18]), Block::Air);
        for _ in 0..6 {
            w.walk(1.0, 0.0);
            w.advance(1.0 / 30.0);
        }
        assert!(w.player[2] > 18.5 && w.player[2] < 18.74);
    }
}
