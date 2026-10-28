//! Turn-based combat. Damage resolves once at impact; drawing never mutates combat.
use crate::math::smooth;

pub const PLAYER_HP: u16 = 120;
pub const ENEMY_HP: u16 = 160;
pub const ACTION_SECONDS: f64 = 1.8;
pub const IMPACT_SECONDS: f64 = 0.65;
pub const MAX_PP: [u8; 4] = [3, 20, 8, 5];
pub const SKILLS: [Move; 4] = [
    Move::Thunderbolt,
    Move::QuickAttack,
    Move::IronTail,
    Move::ThunderWave,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Thunderbolt,
    QuickAttack,
    IronTail,
    ThunderWave,
    HydroPump,
    Tackle,
    Withdraw,
    Paralyzed,
}

impl Move {
    pub fn player(self) -> bool {
        matches!(
            self,
            Self::Thunderbolt | Self::QuickAttack | Self::IronTail | Self::ThunderWave
        )
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Thunderbolt => "THUNDERBOLT",
            Self::QuickAttack => "QUICK ATTACK",
            Self::IronTail => "IRON TAIL",
            Self::ThunderWave => "THUNDER WAVE",
            Self::HydroPump => "HYDRO PUMP",
            Self::Tackle => "TACKLE",
            Self::Withdraw => "WITHDRAW",
            Self::Paralyzed => "PARALYZED",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Thunderbolt => {
                "ELECTRIC / 48 damage vs Water. Only 3 PP: save it for an opening!"
            }
            Self::QuickAttack => "NORMAL / 23 damage. A reliable dash attack with plenty of PP.",
            Self::IronTail => "STEEL / 34 damage. Breaks through Withdraw's guard.",
            Self::ThunderWave => {
                "STATUS / Paralyzes Blastoise: every second counterattack is skipped."
            }
            _ => "",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Choose,
    Action { attack: Move, age: f64, hit: bool },
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Battle {
    pub time: f64,
    pub player_hp: u16,
    pub enemy_hp: u16,
    pub pp: [u8; 4],
    pub phase: Phase,
    pub turn: u32,
    pub paralyzed: bool,
    pub guard: bool,
    pub message: &'static str,
    pub last_damage: u16,
}

impl Default for Battle {
    fn default() -> Self {
        Self {
            time: 0.0,
            player_hp: PLAYER_HP,
            enemy_hp: ENEMY_HP,
            pp: MAX_PP,
            phase: Phase::Choose,
            turn: 1,
            paralyzed: false,
            guard: false,
            message: "Choose a move for Pikachu.",
            last_damage: 0,
        }
    }
}

impl Battle {
    pub fn choose(&mut self, index: usize) -> bool {
        if self.phase != Phase::Choose || index >= SKILLS.len() {
            return false;
        }
        if self.pp[index] == 0 {
            self.message = "No PP left! Choose another move.";
            return false;
        }
        if SKILLS[index] == Move::ThunderWave && self.paralyzed {
            self.message = "Blastoise is already paralyzed. Choose an attack.";
            return false;
        }
        self.pp[index] -= 1;
        self.start(SKILLS[index]);
        true
    }

    fn start(&mut self, attack: Move) {
        self.phase = Phase::Action {
            attack,
            age: 0.0,
            hit: false,
        };
        self.last_damage = 0;
        self.message = match attack {
            Move::Thunderbolt => "Pikachu used Thunderbolt!",
            Move::QuickAttack => "Pikachu used Quick Attack!",
            Move::IronTail => "Pikachu used Iron Tail!",
            Move::ThunderWave => "Pikachu used Thunder Wave!",
            Move::HydroPump => "Blastoise used Hydro Pump!",
            Move::Tackle => "Blastoise used Tackle!",
            Move::Withdraw => "Blastoise used Withdraw!",
            Move::Paralyzed => "Blastoise is paralyzed! It can't move!",
        };
    }

    pub fn advance(&mut self, seconds: f64) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        self.time += seconds;
        let mut remaining = seconds;
        // At most two actions follow a choice. Consume excess time across their boundary.
        while remaining > 0.0 {
            let Phase::Action { attack, age, hit } = self.phase else {
                break;
            };
            let step = remaining.min(ACTION_SECONDS - age);
            let next = age + step;
            remaining -= step;
            let impact = !hit && next >= IMPACT_SECONDS;
            if impact {
                self.resolve(attack);
            }
            self.phase = Phase::Action {
                attack,
                age: next,
                hit: hit || impact,
            };
            if next < ACTION_SECONDS {
                break;
            }
            if self.enemy_hp == 0 {
                self.phase = Phase::Won;
                self.message = "Blastoise fainted. YOU WIN! Press R for a rematch.";
            } else if self.player_hp == 0 {
                self.phase = Phase::Lost;
                self.message = "Pikachu fainted. YOU LOST. Press R to try again.";
            } else if attack.player() {
                let reply = if self.paralyzed && self.turn.is_multiple_of(2) {
                    Move::Paralyzed
                } else {
                    match (self.turn - 1) % 4 {
                        0 | 3 => Move::HydroPump,
                        1 => Move::Tackle,
                        _ => Move::Withdraw,
                    }
                };
                self.start(reply);
            } else {
                self.turn += 1;
                self.phase = Phase::Choose;
                self.message = "Your turn. Choose Pikachu's next move.";
                self.last_damage = 0;
            }
        }
    }

    fn resolve(&mut self, attack: Move) {
        let mut damage = match attack {
            Move::Thunderbolt => 48,
            Move::QuickAttack => 23,
            Move::IronTail => 34,
            Move::HydroPump => 28,
            Move::Tackle => 22,
            Move::ThunderWave => {
                self.paralyzed = true;
                self.message = "Blastoise was paralyzed! Every second reply will fail.";
                return;
            }
            Move::Withdraw => {
                self.guard = true;
                self.message =
                    "Blastoise raised its guard. The next hit is halved; Iron Tail breaks it.";
                return;
            }
            Move::Paralyzed => return,
        };
        if attack.player() {
            if self.guard && attack != Move::IronTail {
                damage /= 2;
                self.message = "Blastoise's guard softened the hit!";
            } else if self.guard {
                self.message = "Iron Tail broke through Blastoise's guard!";
            } else if attack == Move::Thunderbolt {
                self.message = "Thunderbolt is super effective!";
            }
            self.guard = false;
            self.last_damage = damage.min(self.enemy_hp);
            self.enemy_hp = self.enemy_hp.saturating_sub(damage);
        } else {
            self.last_damage = damage.min(self.player_hp);
            self.player_hp = self.player_hp.saturating_sub(damage);
        }
    }

    pub fn attack(self) -> Option<(Move, f32)> {
        match self.phase {
            Phase::Action { attack, age, .. } => Some((attack, (age / ACTION_SECONDS) as f32)),
            _ => None,
        }
    }
    pub fn strength(self, attack: Move) -> f32 {
        match self.attack() {
            Some((current, p)) if current == attack => smooth(p / 0.18) * smooth((1.0 - p) / 0.24),
            _ => 0.0,
        }
    }
}
