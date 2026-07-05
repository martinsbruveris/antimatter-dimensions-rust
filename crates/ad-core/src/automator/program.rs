//! The compiled Automator program: the instruction set the executor (Stage C)
//! will run, plus the currency table with live getters over [`GameState`].

use break_infinity::Decimal;

use crate::state::GameState;

use super::lexer::CmpOp;

/// The three prestige layers referenced by `auto`, the prestige commands, and
/// `wait`/`until` events. Levels are ordered: something waiting for an
/// Infinity is also satisfied by an Eternity or a Reality (`$prestigeLevel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrestigeLayer {
    Infinity,
    Eternity,
    Reality,
}

impl PrestigeLayer {
    pub fn name(self) -> &'static str {
        match self {
            PrestigeLayer::Infinity => "infinity",
            PrestigeLayer::Eternity => "eternity",
            PrestigeLayer::Reality => "reality",
        }
    }

    /// The prestige currency's display name (`$prestigeCurrency`).
    pub fn currency_name(self) -> &'static str {
        match self {
            PrestigeLayer::Infinity => "IP",
            PrestigeLayer::Eternity => "EP",
            PrestigeLayer::Reality => "RM",
        }
    }
}

/// The currencies usable in comparisons (`AutomatorCurrency` category in the
/// original lexer). Each has a live getter; a few are permanently locked at
/// our frontier (`$unlocked` — comparisons using them evaluate to false).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomatorCurrency {
    Am,
    Ip,
    Ep,
    Rm,
    Dt,
    Tp,
    Rg,
    Rep,
    Tt,
    TotalTt,
    SpentTt,
    Infinities,
    BankedInfinities,
    Eternities,
    Realities,
    PendingIp,
    PendingEp,
    PendingTp,
    PendingRm,
    PendingGlyphLevel,
    TotalCompletions,
    PendingCompletions,
    EcCompletions(u8),
    /// Effarig's glyph filter — celestial content, locked at our frontier.
    FilterScore,
    /// V's Space Theorems — celestial content, locked at our frontier.
    SpaceTheorems,
    TotalSpaceTheorems,
}

impl AutomatorCurrency {
    /// Whether the currency is usable (`$unlocked`); a comparison touching a
    /// locked currency compiles to constant-false, like the original.
    pub fn unlocked(self, _game: &GameState) -> bool {
        !matches!(
            self,
            AutomatorCurrency::FilterScore
                | AutomatorCurrency::SpaceTheorems
                | AutomatorCurrency::TotalSpaceTheorems
        )
    }

    /// The live value (`$getter`).
    pub fn value(self, game: &GameState) -> Decimal {
        use AutomatorCurrency::*;
        match self {
            Am => game.antimatter,
            Ip => game.infinity_points,
            Ep => game.eternity_points,
            Rm => game.reality.machines,
            Dt => game.dilation.dilated_time,
            Tp => game.dilation.tachyon_particles,
            // `Replicanti.galaxies.total` = bought + extra (not tachyon).
            Rg => Decimal::from(
                (game.replicanti.galaxies + game.extra_replicanti_galaxies()) as u64,
            ),
            Rep => game.replicanti.amount,
            Tt => game.time_theorems,
            // `total TT` = unspent + everything invested in studies
            // (`calculateTimeStudiesCost`: normal + EC + dilation studies).
            TotalTt => {
                game.time_theorems + Decimal::from_float(game.invested_study_tt())
            }
            // `spent TT` = the current tree's TT cost (normal + EC study).
            SpentTt => Decimal::from_float(game.tree_spent_tt()),
            Infinities => game.infinities,
            BankedInfinities => game.infinities_banked,
            Eternities => game.eternities,
            Realities => Decimal::from(game.reality.realities as u64),
            PendingIp => {
                if game.can_big_crunch() {
                    game.gained_infinity_points()
                } else {
                    Decimal::ZERO
                }
            }
            PendingEp => {
                if game.can_eternity() {
                    game.gained_eternity_points()
                } else {
                    Decimal::ZERO
                }
            }
            PendingTp => {
                if game.dilation.active {
                    game.tachyon_gain()
                } else {
                    Decimal::ZERO
                }
            }
            PendingRm => {
                if game.is_reality_available() {
                    game.gained_reality_machines()
                } else {
                    Decimal::ZERO
                }
            }
            PendingGlyphLevel => {
                if game.is_reality_available() {
                    Decimal::from(game.gained_glyph_level().actual_level as u64)
                } else {
                    Decimal::ZERO
                }
            }
            TotalCompletions => Decimal::from(
                game.eternity_challenges
                    .iter()
                    .map(|&c| c as u64)
                    .sum::<u64>(),
            ),
            // Outside an EC this pretends to be huge so any check for
            // sufficient completions passes (nonblocking).
            PendingCompletions => {
                if game.any_ec_running() {
                    Decimal::from(
                        game.ec_pending_total_completions(
                            game.eternity_challenge_current,
                        ) as u64,
                    )
                } else {
                    Decimal::from_float(f64::MAX)
                }
            }
            EcCompletions(id) => {
                Decimal::from(game.eternity_challenge_completions(id) as u64)
            }
            // Locked at the frontier; the getter is the nonblocking extreme.
            FilterScore => Decimal::from_float(-f64::MAX),
            SpaceTheorems | TotalSpaceTheorems => Decimal::ZERO,
        }
    }
}

/// One side of a comparison. The `image` (text as typed) feeds event-log and
/// error text (`parseConditionalIntoText`).
#[derive(Debug, Clone, PartialEq)]
pub enum CmpValue {
    Currency(AutomatorCurrency, String),
    /// A named constant, resolved against `automator.constants` at *runtime*
    /// (editing a constant affects a running script).
    Const(String),
    Literal(Decimal, String),
}

impl CmpValue {
    fn value(&self, game: &GameState) -> Decimal {
        match self {
            CmpValue::Currency(c, _) => c.value(game),
            // A constant deleted or corrupted after compilation reads as 0
            // (the original coerces `undefined` through `Decimal`).
            CmpValue::Const(name) => game
                .automator
                .constants
                .get(name)
                .and_then(|v| super::parse_decimal_literal(v))
                .unwrap_or(Decimal::ZERO),
            CmpValue::Literal(v, _) => *v,
        }
    }

    /// The display text (`parseConditionalIntoText`'s getters: currency image,
    /// constant name, or the literal formatted).
    pub fn display(&self) -> String {
        match self {
            CmpValue::Currency(_, image) => image.clone(),
            CmpValue::Const(name) => name.clone(),
            CmpValue::Literal(_, image) => image.clone(),
        }
    }
}

/// A compiled comparison (`comparison` rule).
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub left: CmpValue,
    pub op: CmpOp,
    pub right: CmpValue,
}

impl Comparison {
    /// Evaluate against the live game state. Locked currencies force `false`.
    pub fn evaluate(&self, game: &GameState) -> bool {
        for side in [&self.left, &self.right] {
            if let CmpValue::Currency(c, _) = side {
                if !c.unlocked(game) {
                    return false;
                }
            }
        }
        let a = self.left.value(game);
        let b = self.right.value(game);
        match self.op {
            CmpOp::Gt => a > b,
            CmpOp::Lt => a < b,
            CmpOp::Gte => a >= b,
            CmpOp::Lte => a <= b,
            // Rejected in validation; kept total for safety.
            CmpOp::Eq => a == b,
        }
    }

    /// `left op right` as typed, for the event log.
    pub fn display(&self) -> String {
        format!(
            "{} {} {}",
            self.left.display(),
            self.op.symbol(),
            self.right.display()
        )
    }
}

/// An `auto <prestige>` setting.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoSetting {
    On,
    Off,
    /// Seconds-between mode; the payload is milliseconds (the original passes
    /// `duration / 1000` seconds to the autobuyer).
    DurationMs(f64),
    XHighest(Decimal),
    Amount(Decimal),
}

/// How a `pause` displays its duration in the event log: the literal as typed
/// ("10 s") or a formatted timespan for constants.
#[derive(Debug, Clone, PartialEq)]
pub enum PauseText {
    Literal(String),
    ConstantMs(f64),
}

/// The `wait` command's condition.
#[derive(Debug, Clone, PartialEq)]
pub enum WaitCondition {
    Comparison(Comparison),
    Prestige(PrestigeLayer),
    /// `wait black hole <off|bh1|bh2>`; `hole` = 0 for `off`.
    BlackHole {
        off: bool,
        hole: u8,
    },
}

/// The `until` header: a comparison or a prestige event.
#[derive(Debug, Clone, PartialEq)]
pub enum UntilCondition {
    Comparison(Comparison),
    Prestige(PrestigeLayer),
}

/// One compiled command with its source line (for the execution stack,
/// highlighting, and the event log).
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledCommand {
    /// 1-based source line.
    pub line: u32,
    pub op: Instruction,
}

/// The instruction set (one per original command; `comment`/`blob` are
/// no-ops). Block instructions carry their compiled inner block plus the
/// lines of their braces for event-log messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Auto {
        layer: PrestigeLayer,
        setting: AutoSetting,
    },
    BlackHole {
        on: bool,
    },
    NoOp,
    Notify {
        /// The string literal as typed, quotes included (the original toasts
        /// and logs the raw image).
        text: String,
    },
    Pause {
        ms: f64,
        text: PauseText,
    },
    Prestige {
        layer: PrestigeLayer,
        nowait: bool,
        respec: bool,
    },
    StartDilation,
    StartEc {
        ec: u8,
    },
    StudiesBuy {
        studies: Vec<u16>,
        ec: u8,
        start_ec: bool,
        nowait: bool,
        /// The list as typed (or the constant's name) for the block editor
        /// and logs.
        display: String,
    },
    StudiesLoad {
        /// 0-based preset slot (resolved at compile time, like the original's
        /// `$presetIndex`).
        slot: usize,
        nowait: bool,
        display: String,
    },
    StudiesRespec,
    UnlockDilation {
        nowait: bool,
    },
    UnlockEc {
        ec: u8,
        nowait: bool,
    },
    If {
        cmp: Comparison,
        block: Vec<CompiledCommand>,
        /// The `}` line (event log: "skipping to line N").
        end_line: u32,
    },
    While {
        cmp: Comparison,
        block: Vec<CompiledCommand>,
        end_line: u32,
    },
    Until {
        cond: UntilCondition,
        block: Vec<CompiledCommand>,
        end_line: u32,
    },
    Wait {
        cond: WaitCondition,
    },
    Stop,
}

impl Instruction {
    /// The nested block, for stack reconstruction on load (`blockCommands`).
    pub fn block(&self) -> Option<&[CompiledCommand]> {
        match self {
            Instruction::If { block, .. }
            | Instruction::While { block, .. }
            | Instruction::Until { block, .. } => Some(block),
            _ => None,
        }
    }
}
