//! Serde DTOs mirroring the original `player` save schema, for the **read path**.
//!
//! These structs deliberately match the *save's* JSON layout (nested,
//! camelCased) rather than our internal [`GameState`]. We only declare the
//! fields we actually model; serde ignores every other key in the save, which is
//! exactly the "ignore unimplemented mechanics" behaviour we want — a late-game
//! save deserializes fine and we read just the early-game slice we understand.
//!
//! The fields we *do* model are **required**: there are no serde defaults, so a
//! missing one fails deserialization rather than being silently replaced. We
//! would rather be alerted that the external format changed than quietly diverge.
//! (Ignoring *unknown* keys is a separate behaviour that still applies — that's
//! how we drop mechanics we don't model.) Every `Decimal` is a JSON string, read
//! through [`break_infinity::serde_string`].
//!
//! [`PlayerDTO`] is the untrusted external shape; [`GameState::from_save_dto`]
//! is where we map it in, rebuild derived state (tickspeed cost, autobuyer
//! intervals/timers) from our own constructors, and validate the modelled
//! values — erroring on anything out of range or the wrong shape — so a
//! malformed save is rejected rather than silently coerced. The one intentional
//! leniency is an unmodelled notation name (we implement only a subset of the
//! game's notations), which is ignored, keeping our default.

use break_infinity::Decimal;
use serde::Deserialize;

use crate::achievements::ACHIEVEMENT_ROW_COUNT;
use crate::autobuyers::{
    AutoRealityMode, AutobuyerMode, AutobuyerState, EternityAutobuyer,
    PrestigeAutobuyerMode, PrestigeGoalSettings, RealityAutobuyer,
};
use crate::break_infinity_upgrades::BreakInfinityUpgrade;
use crate::challenges::NormalChallengeState;
use crate::dilation::DilationState;
use crate::infinity_challenges::InfinityChallengeState;
use crate::infinity_dimensions::{InfinityDimension, INFINITY_DIMENSION_COUNT};
use crate::infinity_upgrades::InfinityUpgrade;
use crate::options::{
    AwayProgress, Confirmations, Options, ShowHintText, MAX_AUTOSAVE_INTERVAL_MS,
    MAX_NOTATION_DIGITS, MAX_UPDATE_RATE_MS, MIN_AUTOSAVE_INTERVAL_MS,
    MIN_NOTATION_DIGITS, MIN_UPDATE_RATE_MS, TAB_COUNT,
};
use crate::records::{
    BestEternity, BestInfinity, RecentEternity, Records, ThisEternity, ThisInfinity,
};
use crate::replicanti::ReplicantiState;
use crate::state::{DimensionTier, GameState, TickspeedState};
use crate::time_dimensions::{time_dimension_cost, TimeDimension, TIME_DIMENSION_COUNT};

use super::SaveError;

/// The original `AUTOBUYER_MODE` values we accept; any other value in a save is
/// rejected as malformed (see [`autobuyer_mode_from_raw`]).
const AUTOBUYER_MODE_BUY_SINGLE: i64 = 1;
const AUTOBUYER_MODE_BUY_10: i64 = 10;

/// The fixed number of antimatter dimension tiers (and their autobuyers).
const DIMENSION_COUNT: usize = 8;

/// Top-level `player` object (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub antimatter: Decimal,
    pub dimensions: DimensionsDTO,
    #[serde(with = "break_infinity::serde_string")]
    pub sacrificed: Decimal,
    pub dimension_boosts: u32,
    pub galaxies: u32,
    pub total_tick_bought: u64,
    /// `player.chall9TickspeedCostBumps` — NC9 tickspeed cost bumps (a number).
    pub chall9_tickspeed_cost_bumps: u64,
    /// `player.break` — break-infinity flag. `break` is a Rust keyword, hence the
    /// rename. Part of the Infinity-unlocked test (§4.3).
    #[serde(rename = "break")]
    pub break_unlocked: bool,
    #[serde(with = "break_infinity::serde_string")]
    pub infinities: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub infinity_points: Decimal,
    /// `player.eternityPoints` — the Eternity prestige currency.
    #[serde(with = "break_infinity::serde_string")]
    pub eternity_points: Decimal,
    /// `player.eternities` — number of Eternities performed (a Decimal).
    #[serde(with = "break_infinity::serde_string")]
    pub eternities: Decimal,
    /// `player.timeShards` — produced by the 1st Time Dimension.
    #[serde(with = "break_infinity::serde_string")]
    pub time_shards: Decimal,
    /// `player.totalTickGained` — free Tickspeed upgrades from Time Shards.
    pub total_tick_gained: u64,
    /// `player.timestudy` — Time Theorems + bought studies.
    pub timestudy: TimestudyDTO,
    /// `player.respec` — respec the study tree on the next Eternity.
    pub respec: bool,
    /// `player.infinitiesBanked` — banked Infinities (TS191).
    #[serde(with = "break_infinity::serde_string")]
    pub infinities_banked: Decimal,
    /// `player.eternityChalls` — per-EC completion counts (`"eterc<N>"` keys).
    pub eternity_challs: std::collections::HashMap<String, u8>,
    /// EC8's per-run purchase budgets.
    pub eterc8ids: i32,
    pub eterc8repl: i32,
    /// `player.dilation` — Time Dilation state.
    pub dilation: DilationDTO,
    /// `player.eternityUpgrades` — owned Eternity Upgrades (a Set of ids 1–6).
    pub eternity_upgrades: Vec<u8>,
    /// `player.epmultUpgrades` — rebuyable ×5 EP-multiplier purchases.
    pub epmult_upgrades: u32,
    /// `player.infinityPower` — produced by the Infinity Dimensions.
    #[serde(with = "break_infinity::serde_string")]
    pub infinity_power: Decimal,
    /// `player.infinityUpgrades` — owned Infinity Upgrades **and** the one-time
    /// Break Infinity Upgrades (they share this string set), by original id.
    /// Unmodelled ids are ignored on load.
    pub infinity_upgrades: Vec<String>,
    /// `player.infinityRebuyables` — purchase counts of the 3 rebuyable Break
    /// Infinity Upgrades.
    pub infinity_rebuyables: Vec<u32>,
    /// `player.partInfinityPoint` — the `ipGen` fractional IP accumulator.
    pub part_infinity_point: f64,
    /// `player.achievementBits` — one bitmask int per achievement row.
    pub achievement_bits: Vec<u32>,
    /// `player.tutorialState` — current tutorial-highlight step.
    pub tutorial_state: u8,
    /// `player.tutorialActive` — whether the current step's highlight shows.
    pub tutorial_active: bool,
    pub records: RecordsDTO,
    pub auto: AutoDTO,
    pub options: OptionsDTO,
    /// `player.challenge` — only the normal-challenge run state is modelled.
    pub challenge: ChallengeDTO,
    /// `player.chall8TotalSacrifice` — NC8's running sacrifice product (a Decimal
    /// string in the save). See `sacrifice.rs`.
    #[serde(with = "break_infinity::serde_string")]
    pub chall8_total_sacrifice: Decimal,
    /// `player.chall2Pow` — NC2's production factor (a plain number in the save).
    pub chall2_pow: f64,
    /// `player.chall3Pow` — NC3's 1st-dimension multiplier (a Decimal string).
    #[serde(with = "break_infinity::serde_string")]
    pub chall3_pow: Decimal,
    /// `player.matter` — normal matter for NC11 (a Decimal string).
    #[serde(with = "break_infinity::serde_string")]
    pub matter: Decimal,
    /// `player.replicanti` — the Replicanti state (Feature 3.2).
    pub replicanti: ReplicantiDTO,
    /// `player.tabNotifications` — tabs currently showing the `!` badge, as
    /// concatenated `parentKey + subtabKey` strings (a JS `Set` serialized as an
    /// array). Keys we don't render are carried through untouched.
    pub tab_notifications: Vec<String>,
    /// `player.triggeredTabNotificationBits` — which tab notifications have ever
    /// fired. Bits beyond our modelled ids round-trip verbatim.
    pub triggered_tab_notification_bits: u32,
    /// `player.realities` — Realities performed (a plain number at the root).
    pub realities: f64,
    /// `player.blackHole` — the two Black Holes.
    pub black_hole: Vec<BlackHoleDTO>,
    /// `player.blackHolePause` / `blackHolePauseTime`.
    pub black_hole_pause: bool,
    pub black_hole_pause_time: f64,
    /// `player.reality` — the Reality-layer state (modelled subset).
    pub reality: RealityDTO,
    /// `player.requirementChecks` — the "avoided X" run flags (modelled subset).
    pub requirement_checks: RequirementChecksDTO,
}

/// `player.reality` (modelled subset). The glyph inventory lives under
/// `reality.glyphs` (Feature 6.2); the automator subtree is ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealityDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub reality_machines: Decimal,
    #[serde(rename = "maxRM", with = "break_infinity::serde_string")]
    pub max_rm: Decimal,
    /// Unspent Perk Points (a plain number).
    pub perk_points: f64,
    /// Bought perk ids (a Set serialized as an array).
    pub perks: Vec<u8>,
    /// The live glyph RNG seed (a plain number; a 32-bit xorshift state once
    /// rolling, the raw initial seed right after the first Reality).
    pub seed: f64,
    pub initial_seed: f64,
    /// The cached Marsaglia spare deviate (`1e6` = none).
    pub second_gaussian: f64,
    /// Rebuyable Reality Upgrade counts, keyed by id string ("1".."5").
    pub rebuyables: std::collections::HashMap<String, u32>,
    pub upgrade_bits: u32,
    pub upg_reqs: u32,
    pub req_lock: ReqLockDTO,
    pub respec: bool,
    pub ach_timer: f64,
    pub auto_achieve: bool,
    pub gained_auto_achievements: bool,
    pub glyphs: GlyphsDTO,
    /// `player.reality.automator`: scripts, constants, editor + run state
    /// (Feature 6.6 Stage B), and the force-unlock flag.
    pub automator: RealityAutomatorDTO,
}

/// `player.reality.automator`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealityAutomatorDTO {
    pub force_unlock: bool,
    /// Scripts keyed by id string (`{ "1": { id, name, content } }`).
    pub scripts: std::collections::HashMap<String, AutomatorScriptDTO>,
    /// Constants; values are strings in practice, but we accept any scalar
    /// (imported data may carry numbers) and stringify.
    pub constants: std::collections::HashMap<String, serde_json::Value>,
    pub constant_sort_order: Vec<String>,
    /// `AUTOMATOR_TYPE`: 0 text, 1 block.
    #[serde(rename = "type")]
    pub editor_type: i64,
    /// `AutomatorPanels` (0–7).
    pub current_info_pane: i64,
    pub exec_timer: f64,
    pub state: AutomatorStateDTO,
}

/// One `player.reality.automator.scripts[id]` (the duplicated `id` prop
/// inside the object is ignored; the map key is authoritative).
#[derive(Debug, Clone, Deserialize)]
pub struct AutomatorScriptDTO {
    pub name: String,
    pub content: String,
}

/// `player.reality.automator.state`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomatorStateDTO {
    /// `AUTOMATOR_MODE` (1 pause / 2 run / 3 single-step). Fresh saves lack
    /// the key entirely (the original initializes it to an undefined enum
    /// member), which reads as paused.
    #[serde(default)]
    pub mode: Option<i64>,
    pub top_level_script: i64,
    pub editor_script: i64,
    pub repeat: bool,
    pub force_restart: bool,
    pub follow_execution: bool,
    /// Stack entries (`{ lineNumber, commandState }`); parsed leniently — the
    /// original resets the run when the stack can't be matched to the script
    /// anyway, so an unrecognized shape clears the stack rather than failing
    /// the load.
    pub stack: Vec<serde_json::Value>,
}

/// One entry of `player.blackHole[]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackHoleDTO {
    pub unlocked: bool,
    pub active: bool,
    pub phase: f64,
    pub activations: f64,
    pub interval_upgrades: f64,
    pub power_upgrades: f64,
    pub duration_upgrades: f64,
}

/// `player.reality.glyphs` (modelled subset: no undo/sets/filter/cosmetics).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphsDTO {
    pub active: Vec<GlyphDTO>,
    pub inventory: Vec<GlyphDTO>,
    /// Cumulative sacrifice per type, keyed by type name.
    pub sac: std::collections::HashMap<String, f64>,
    pub protected_rows: u32,
}

/// One glyph. Types we don't model (effarig/reality/cursed) are skipped on
/// load rather than failing it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphDTO {
    /// May be absent on freshly generated originals (`id: undefined`).
    #[serde(default)]
    pub id: Option<f64>,
    #[serde(default)]
    pub idx: Option<f64>,
    #[serde(rename = "type")]
    pub kind: String,
    pub strength: f64,
    pub level: f64,
    /// Absent from very old saves; recomputed as `level` then.
    #[serde(default)]
    pub raw_level: Option<f64>,
    pub effects: u32,
}

/// `player.reality.reqLock` — player-armed requirement locks.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqLockDTO {
    pub reality: u32,
}

/// `player.requirementChecks` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementChecksDTO {
    pub eternity: EternityChecksDTO,
    pub reality: RealityChecksDTO,
}

/// `player.requirementChecks.eternity` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EternityChecksDTO {
    #[serde(rename = "noRG")]
    pub no_rg: bool,
}

/// `player.requirementChecks.reality` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealityChecksDTO {
    pub no_infinities: bool,
    pub no_eternities: bool,
    pub max_glyphs: i32,
}

/// `player.replicanti` (modelled subset). The sub-interval `timer` is transient and
/// absent from real saves, so it is not read (defaults to 0). `galCost` is present
/// in the save but derived on our side (`replicanti_galaxy_cost`), so it is omitted
/// here and ignored on load.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicantiDTO {
    /// Whether Replicanti are unlocked (`unl`).
    pub unl: bool,
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    /// Reproduction chance per interval (a plain number).
    pub chance: f64,
    #[serde(with = "break_infinity::serde_string")]
    pub chance_cost: Decimal,
    /// Reproduction interval in ms (a plain number).
    pub interval: f64,
    #[serde(with = "break_infinity::serde_string")]
    pub interval_cost: Decimal,
    /// Max Replicanti Galaxies (`boughtGalaxyCap`).
    pub bought_galaxy_cap: u32,
    /// Replicanti Galaxies made.
    pub galaxies: u32,
}

/// `player.dilation` (modelled subset). `rebuyables` is an id-keyed object map
/// (ids 1–3 in frontier; the Pelle ids 11–13 are ignored/written as 0).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DilationDTO {
    /// Dilation studies bought (ids 1–6; 6 is out of frontier but carried).
    pub studies: Vec<u8>,
    /// Whether a dilated Eternity is running.
    pub active: bool,
    #[serde(with = "break_infinity::serde_string")]
    pub tachyon_particles: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub dilated_time: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub next_threshold: Decimal,
    /// Plain numbers in the save (total may be fractional past 1000 TGs).
    pub base_tachyon_galaxies: f64,
    pub total_tachyon_galaxies: f64,
    /// One-time upgrade ids (a Set serialized as an array).
    pub upgrades: Vec<u8>,
    /// Rebuyable purchase counts, keyed by id string.
    pub rebuyables: std::collections::HashMap<String, u32>,
    /// Save key `lastEP` (capital EP).
    #[serde(rename = "lastEP", with = "break_infinity::serde_string")]
    pub last_ep: Decimal,
}

/// `player.challenge` — the `normal`, `infinity`, and `eternity` run states.
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeDTO {
    pub normal: NormalChallengeDTO,
    pub infinity: InfinityChallengeDTO,
    pub eternity: EternityChallengeDTO,
}

/// `player.challenge.eternity` (modelled subset; `requirementBits` is ignored
/// until a consumer exists).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EternityChallengeDTO {
    /// Active challenge id (`0` = none).
    pub current: u8,
    /// The EC whose unlock study is held (`0` = none).
    pub unlocked: u8,
    /// Which EC studies' secondary requirements were previously met.
    pub requirement_bits: u16,
}

/// `player.timestudy` (modelled subset; preferred paths are frontend-free for
/// now and ignored).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestudyDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub theorem: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub max_theorem: Decimal,
    pub am_bought: u32,
    pub ip_bought: u32,
    pub ep_bought: u32,
    pub studies: Vec<u16>,
    /// The six preset slots (`{name, studies}` each).
    pub presets: Vec<StudyPresetDTO>,
}

/// One `player.timestudy.presets[]` slot.
#[derive(Debug, Clone, Deserialize)]
pub struct StudyPresetDTO {
    pub name: String,
    pub studies: String,
}

/// `player.challenge.infinity` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfinityChallengeDTO {
    /// Active challenge id (`0` = none).
    pub current: u8,
    /// Completed-challenge bitmask (bit `1 << id`).
    pub completed_bits: u16,
    /// Fastest completion times (ms), `Number.MAX_VALUE` = never.
    pub best_times: Vec<f64>,
}

/// `player.challenge.normal` (modelled subset). `bestTimes` is ignored until a
/// records consumer exists.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalChallengeDTO {
    /// Active challenge id (`0` = none).
    pub current: u8,
    /// Completed-challenge bitmask (bit `1 << id`).
    pub completed_bits: u16,
}

/// `player.dimensions` — the `antimatter`, `infinity`, and `time` arrays.
#[derive(Debug, Clone, Deserialize)]
pub struct DimensionsDTO {
    pub antimatter: Vec<DimensionDTO>,
    pub infinity: Vec<InfinityDimensionDTO>,
    pub time: Vec<TimeDimensionDTO>,
}

/// One entry of `player.dimensions.time[]`. The stored `cost` is derived from
/// `bought` on load (like tickspeed), so it is ignored here.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeDimensionDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    pub bought: u64,
}

/// One entry of `player.dimensions.infinity[]` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfinityDimensionDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub cost: Decimal,
    /// `10 × purchases`.
    pub base_amount: u64,
    pub is_unlocked: bool,
}

/// One entry of `player.dimensions.antimatter[]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    pub bought: u64,
    /// `costBumps` — extra cost-scaling steps (nonzero only under NC9). See
    /// [`DimensionTier::cost_bumps`].
    pub cost_bumps: u64,
}

/// `player.records` — the modelled slice: all-time antimatter, total time
/// played, and the current/fastest infinity timing.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordsDTO {
    #[serde(with = "break_infinity::serde_string")]
    pub total_antimatter: Decimal,
    /// `player.records.totalTimePlayed` — game time (ms), monotonic.
    pub total_time_played: f64,
    /// `player.records.realTimePlayed` — real time (ms), monotonic.
    pub real_time_played: f64,
    /// `records.timePlayedAtBHUnlock` (`Number.MAX_VALUE` = not yet).
    #[serde(rename = "timePlayedAtBHUnlock")]
    pub time_played_at_bh_unlock: f64,
    pub this_infinity: ThisInfinityDTO,
    pub best_infinity: BestInfinityDTO,
    pub this_eternity: ThisEternityDTO,
    pub best_eternity: BestEternityDTO,
    /// `records.recentEternities` — 10 mixed-type tuples
    /// `[time, realTime, EP, eternities, challenge, TT]`; parsed leniently
    /// (unrecognized shapes fall back to the placeholder entry).
    pub recent_eternities: Vec<serde_json::Value>,
    pub this_reality: ThisRealityDTO,
    pub best_reality: BestRealityDTO,
    /// `records.recentRealities` — 10 mixed-type tuples
    /// `[time, realTime, RM, realityCount, challenge, projIM]`; parsed
    /// leniently like the eternity ring.
    pub recent_realities: Vec<serde_json::Value>,
}

/// `player.records.thisReality` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThisRealityDTO {
    pub time: f64,
    pub real_time: f64,
    #[serde(rename = "maxEP", with = "break_infinity::serde_string")]
    pub max_ep: Decimal,
    #[serde(with = "break_infinity::serde_string")]
    pub max_replicanti: Decimal,
    #[serde(rename = "maxDT", with = "break_infinity::serde_string")]
    pub max_dt: Decimal,
}

/// `player.records.bestReality` (modelled subset; the glyph-loadout snapshot
/// fields are ignored).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestRealityDTO {
    pub time: f64,
    pub real_time: f64,
    #[serde(rename = "RMmin", with = "break_infinity::serde_string")]
    pub rm_min: Decimal,
    pub glyph_level: u32,
    #[serde(rename = "bestEP", with = "break_infinity::serde_string")]
    pub best_ep: Decimal,
    pub glyph_strength: f64,
}

/// `player.records.thisEternity` (modelled subset): timing plus the peak
/// antimatter (gates IC/ID unlocks) and peak IP (drives the Eternity goal and
/// the EP formula) this eternity.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThisEternityDTO {
    /// Game time in this eternity (ms).
    pub time: f64,
    /// Real time in this eternity (ms).
    pub real_time: f64,
    /// Peak antimatter this eternity. Save key `maxAM` (capital AM).
    #[serde(rename = "maxAM", with = "break_infinity::serde_string")]
    pub max_am: Decimal,
    /// Peak Infinity Points this eternity. Save key `maxIP` (capital IP).
    #[serde(rename = "maxIP", with = "break_infinity::serde_string")]
    pub max_ip: Decimal,
    /// Peak EP/min this eternity (`bestEPmin`).
    #[serde(rename = "bestEPmin", with = "break_infinity::serde_string")]
    pub best_ep_min: Decimal,
    /// The Eternity EP gain when the peak rate was set (`bestEPminVal`).
    #[serde(rename = "bestEPminVal", with = "break_infinity::serde_string")]
    pub best_ep_min_val: Decimal,
}

/// `player.records.bestEternity` (modelled subset). Times are
/// `Number.MAX_VALUE` when no eternity has been performed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestEternityDTO {
    /// Fastest eternity by game time (ms).
    pub time: f64,
    /// Fastest eternity by real time (ms).
    pub real_time: f64,
}

/// `player.records.thisInfinity` (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThisInfinityDTO {
    /// Game time in this infinity (ms).
    pub time: f64,
    /// Real time in this infinity (ms).
    pub real_time: f64,
    /// Peak antimatter this infinity. The save key is `maxAM` (capital AM),
    /// which `camelCase` would render as `maxAm`, so it is renamed explicitly.
    #[serde(rename = "maxAM", with = "break_infinity::serde_string")]
    pub max_am: Decimal,
    /// Peak IP/min this infinity (`bestIPmin`).
    #[serde(rename = "bestIPmin", with = "break_infinity::serde_string")]
    pub best_ip_min: Decimal,
    /// The crunch IP gain when the peak rate was set (`bestIPminVal`).
    #[serde(rename = "bestIPminVal", with = "break_infinity::serde_string")]
    pub best_ip_min_val: Decimal,
}

/// `player.records.bestInfinity` (modelled subset). Times are `Number.MAX_VALUE`
/// when no infinity has been performed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestInfinityDTO {
    /// Fastest infinity by game time (ms).
    pub time: f64,
    /// Fastest infinity by real time (ms).
    pub real_time: f64,
}

/// `player.auto` — autobuyer state (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDTO {
    pub autobuyers_on: bool,
    pub antimatter_dims: AntimatterDimsDTO,
    pub tickspeed: AutobuyerDTO,
    /// `player.auto.dimBoost` (NC10 autobuyer). Only the interval-upgrade state +
    /// active flag are modelled; the limit config is ignored (inert pre-break).
    pub dim_boost: PrestigeAutobuyerDTO,
    /// `player.auto.galaxy` (NC11 autobuyer).
    pub galaxy: PrestigeAutobuyerDTO,
    /// `player.auto.bigCrunch` (NC12 autobuyer): interval-upgrade state plus
    /// the post-break goal settings (mode/amount/time/xHighest).
    pub big_crunch: BigCrunchAutobuyerDTO,
    /// `player.auto.eternity` (100-Eternities milestone autobuyer).
    pub eternity: EternityAutobuyerDTO,
    /// `player.auto.reality` (Reality Upgrade 25 autobuyer). The Effarig
    /// `shard` target is out of frontier and ignored.
    pub reality: RealityAutobuyerDTO,
}

/// `player.auto.bigCrunch`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigCrunchAutobuyerDTO {
    pub is_active: bool,
    pub interval: f64,
    pub cost: f64,
    /// `AUTO_CRUNCH_MODE`: 0 amount, 1 time, 2 X highest.
    pub mode: i64,
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    pub increase_with_mult: bool,
    pub time: f64,
    #[serde(with = "break_infinity::serde_string")]
    pub x_highest: Decimal,
}

/// `player.auto.eternity`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EternityAutobuyerDTO {
    pub is_active: bool,
    /// `AUTO_ETERNITY_MODE`: 0 amount, 1 time, 2 X highest.
    pub mode: i64,
    #[serde(with = "break_infinity::serde_string")]
    pub amount: Decimal,
    pub increase_with_mult: bool,
    pub time: f64,
    #[serde(with = "break_infinity::serde_string")]
    pub x_highest: Decimal,
}

/// `player.auto.reality`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealityAutobuyerDTO {
    pub is_active: bool,
    /// `AUTO_REALITY_MODE`: 0 RM, 1 glyph, 2 either, 3 both, 4 time,
    /// 5 relic shards (out of frontier — loads as RM).
    pub mode: i64,
    #[serde(with = "break_infinity::serde_string")]
    pub rm: Decimal,
    pub glyph: f64,
    pub time: f64,
}

/// A Dim Boost / Galaxy / Big Crunch autobuyer entry. These have no antimatter
/// "slow version" (`isBought`) or single/max `mode`, so we read only the
/// active flag and interval-upgrade state; the rest of each object (limit config,
/// crunch mode) is ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrestigeAutobuyerDTO {
    pub is_active: bool,
    pub interval: f64,
    pub cost: f64,
}

/// `player.auto.antimatterDims` — the `all` array holds the 8 tier autobuyers.
#[derive(Debug, Clone, Deserialize)]
pub struct AntimatterDimsDTO {
    pub all: Vec<AutobuyerDTO>,
}

/// A single autobuyer entry (`auto.antimatterDims.all[t]` or `auto.tickspeed`).
///
/// We read `isActive`/`isBought`/`mode` plus the interval-upgrade state
/// (`interval`/`cost`), which round-trips now that interval upgrades are modelled
/// (Feature 2.6). `bulk` is still ignored (its upgrades are Break-Infinity-era);
/// `lastTick` is transient timer phase (an absolute timestamp in the original; an
/// elapsed-time accumulator for us) reset to 0 on load.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutobuyerDTO {
    pub is_active: bool,
    pub is_bought: bool,
    /// Original `AUTOBUYER_MODE` (`1` = single, `10` = buy-10/max).
    pub mode: i64,
    /// Current tick interval in ms (reduced by interval upgrades).
    pub interval: f64,
    /// IP cost of the next interval upgrade (a plain number).
    pub cost: f64,
}

/// `player.options` — UI/UX preferences (modelled subset).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsDTO {
    pub hotkeys: bool,
    pub update_rate: u32,
    pub notation: String,
    pub notation_digits: NotationDigitsDTO,
    pub offline_ticks: u32,
    /// `player.options.autosaveInterval` — autosave cadence in milliseconds.
    pub autosave_interval: u32,
    /// `player.options.showTimeSinceSave` — header time-since-save indicator.
    pub show_time_since_save: bool,
    /// `player.options.saveFileName` — custom per-save export/display name.
    pub save_file_name: String,
    /// `player.options.confirmations` (modelled subset).
    pub confirmations: ConfirmationsDTO,
    /// `player.options.animations` (modelled subset).
    pub animations: AnimationsDTO,
    /// `player.options.showHintText` (modelled subset).
    pub show_hint_text: ShowHintTextDTO,
    /// `player.options.awayProgress` (modelled subset).
    pub away_progress: AwayProgressDTO,
    /// `player.options.headerTextColored` — relative prestige-gain coloring.
    pub header_text_colored: bool,
    /// `player.options.sidebarResourceID` (note the capitalized `ID` in the
    /// original key) — the Modern-UI sidebar resource (0 = latest).
    #[serde(rename = "sidebarResourceID")]
    pub sidebar_resource_id: u32,
    /// `player.options.hiddenTabBits` — hidden top-level tabs bitmask.
    pub hidden_tab_bits: u32,
    /// `player.options.hiddenSubtabBits` — 11 per-tab hidden-subtab bitmasks.
    pub hidden_subtab_bits: Vec<u32>,
    /// `player.options.automatorEvents` — the Automator event-log settings.
    pub automator_events: AutomatorEventsDTO,
}

/// `player.options.automatorEvents`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomatorEventsDTO {
    pub newest_first: bool,
    pub timestamp_type: i64,
    pub max_entries: u32,
    pub clear_on_reality: bool,
    pub clear_on_restart: bool,
}

/// `player.options.animations` — the animation toggles we model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationsDTO {
    pub big_crunch: bool,
}

/// `player.options.showHintText` — the info-display hints we model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowHintTextDTO {
    pub show_percentage: bool,
    pub achievements: bool,
    pub achievement_unlock_states: bool,
    pub challenges: bool,
}

/// `player.options.awayProgress` — the away-progress toggles we model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwayProgressDTO {
    pub antimatter: bool,
    pub dimension_boosts: bool,
    pub antimatter_galaxies: bool,
    pub infinities: bool,
    pub infinity_points: bool,
    pub replicanti: bool,
    pub replicanti_galaxies: bool,
}

/// `player.options.notationDigits`.
#[derive(Debug, Clone, Deserialize)]
pub struct NotationDigitsDTO {
    pub comma: u32,
    pub notation: u32,
}

/// `player.options.confirmations` — the four prestige confirmations we model.
/// All required: a missing one fails the load, surfacing a format change rather
/// than silently defaulting.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmationsDTO {
    pub dimension_boost: bool,
    pub antimatter_galaxy: bool,
    pub sacrifice: bool,
    pub big_crunch: bool,
    pub eternity: bool,
    pub dilation: bool,
    pub switch_automator_mode: bool,
}

impl GameState {
    /// Maps a decoded save DTO into a fresh [`GameState`].
    ///
    /// The DTO's modelled fields are required, so a missing one has already
    /// failed deserialization before we get here. This method additionally
    /// validates the shape/values and returns a [`SaveError`] rather than
    /// silently guessing when something is off:
    /// - [`UnexpectedArrayLength`](SaveError::UnexpectedArrayLength) if the
    ///   dimension or dimension-autobuyer arrays aren't exactly 8 long;
    /// - [`InvalidAutobuyerMode`](SaveError::InvalidAutobuyerMode) for an
    ///   unrecognized autobuyer mode;
    /// - [`OptionOutOfRange`](SaveError::OptionOutOfRange) for a numeric option
    ///   outside its accepted range.
    ///
    /// Everything past our frontier is left at its constructor default (a
    /// late-game save loads as "fresh early-game run, Infinity unlocked"). The
    /// derived tickspeed cost is recomputed from our formula and autobuyer
    /// intervals/timers come from our constructors. The single intentional
    /// leniency is an unmodelled notation name, which is ignored (we model only
    /// a subset), keeping our default.
    pub fn from_save_dto(dto: &PlayerDTO) -> Result<GameState, SaveError> {
        // The 8 antimatter dimensions are a fixed-length array; a different
        // length signals an unexpected save format.
        let dims = &dto.dimensions.antimatter;
        if dims.len() != DIMENSION_COUNT {
            return Err(SaveError::UnexpectedArrayLength {
                field: "dimensions.antimatter",
                expected: DIMENSION_COUNT,
                found: dims.len(),
            });
        }
        let dimensions = std::array::from_fn(|tier| DimensionTier {
            amount: dims[tier].amount,
            bought: dims[tier].bought,
            cost_bumps: dims[tier].cost_bumps,
        });

        // The 8 time dimensions are likewise fixed-length; their cost is
        // derived from the purchase count on load.
        let time_dims = &dto.dimensions.time;
        if time_dims.len() != TIME_DIMENSION_COUNT {
            return Err(SaveError::UnexpectedArrayLength {
                field: "dimensions.time",
                expected: TIME_DIMENSION_COUNT,
                found: time_dims.len(),
            });
        }
        let time_dimensions = std::array::from_fn(|tier| TimeDimension {
            amount: time_dims[tier].amount,
            bought: time_dims[tier].bought,
            cost: time_dimension_cost(tier, time_dims[tier].bought),
        });

        // The 8 infinity dimensions are likewise fixed-length.
        let inf_dims = &dto.dimensions.infinity;
        if inf_dims.len() != INFINITY_DIMENSION_COUNT {
            return Err(SaveError::UnexpectedArrayLength {
                field: "dimensions.infinity",
                expected: INFINITY_DIMENSION_COUNT,
                found: inf_dims.len(),
            });
        }
        let infinity_dimensions = std::array::from_fn(|tier| InfinityDimension {
            amount: inf_dims[tier].amount,
            base_amount: inf_dims[tier].base_amount,
            cost: inf_dims[tier].cost,
            is_unlocked: inf_dims[tier].is_unlocked,
        });

        // Replicanti (Feature 3.2). The bought-galaxy-cap cost (`galCost`) is
        // derived from `boughtGalaxyCap`, and the sub-interval `timer` is transient
        // (starts at 0), so neither is round-tripped.
        let replicanti = ReplicantiState {
            unlocked: dto.replicanti.unl,
            amount: dto.replicanti.amount,
            timer_ms: 0.0,
            chance: dto.replicanti.chance,
            chance_cost: dto.replicanti.chance_cost,
            interval_ms: dto.replicanti.interval,
            interval_cost: dto.replicanti.interval_cost,
            galaxies: dto.replicanti.galaxies,
            galaxy_cap: dto.replicanti.bought_galaxy_cap,
        };

        // `player.break` is the Break-Infinity flag. Infinity-*unlocked* (has
        // reached Infinity) is derived: broke Infinity, or any infinity / IP was
        // ever gained. We reset the pre-Infinity *mechanics* past the frontier, but
        // Infinity Points, the infinities count, and the time/infinity records are
        // within our frontier now, so they carry over verbatim.
        let broke_infinity = dto.break_unlocked;
        let infinity_unlocked = broke_infinity
            || dto.infinities > Decimal::ZERO
            || dto.infinity_points > Decimal::ZERO;
        // Eternity-unlocked is likewise derived (`PlayerProgress.eternityUnlocked`
        // ⇔ eternities gained or EP held).
        let eternity_unlocked =
            dto.eternities > Decimal::ZERO || dto.eternity_points > Decimal::ZERO;

        // Infinity Upgrades + one-time Break Infinity Upgrades share the string
        // set: set the bit in whichever bitmask a modelled id belongs to; unknown
        // ids are ignored so a later-game save still loads.
        let mut infinity_upgrades = 0u32;
        let mut break_infinity_upgrades = 0u32;
        for id in &dto.infinity_upgrades {
            if let Some(upgrade) = InfinityUpgrade::from_save_id(id) {
                infinity_upgrades |= upgrade.bit();
            } else if let Some(upgrade) = BreakInfinityUpgrade::from_save_id(id) {
                break_infinity_upgrades |= upgrade.bit();
            }
        }

        // The 3 rebuyable Break Infinity Upgrade counts. A different length is an
        // unexpected save shape.
        let rebuyables = &dto.infinity_rebuyables;
        if rebuyables.len() != 3 {
            return Err(SaveError::UnexpectedArrayLength {
                field: "infinityRebuyables",
                expected: 3,
                found: rebuyables.len(),
            });
        }
        let infinity_rebuyables = [rebuyables[0], rebuyables[1], rebuyables[2]];

        // Parse the recent-eternities tuples: `[time, realTime, EP-string,
        // eternities-string, ...]`. Decimals may be strings or plain numbers.
        let parse_decimal = |v: &serde_json::Value| -> Option<Decimal> {
            match v {
                serde_json::Value::String(s) => s.parse().ok(),
                serde_json::Value::Number(n) => n.as_f64().map(Decimal::from_float),
                _ => None,
            }
        };
        let mut recent_eternities = Vec::with_capacity(10);
        for entry in dto.records.recent_eternities.iter().take(10) {
            let parsed = entry.as_array().and_then(|t| {
                Some(RecentEternity {
                    time_ms: t.first()?.as_f64()?,
                    real_time_ms: t.get(1)?.as_f64()?,
                    ep: parse_decimal(t.get(2)?)?,
                    eternities: parse_decimal(t.get(3)?)?,
                })
            });
            recent_eternities.push(parsed.unwrap_or_else(RecentEternity::placeholder));
        }
        while recent_eternities.len() < 10 {
            recent_eternities.push(RecentEternity::placeholder());
        }

        // Per-EC completion counts from the `eternityChalls` map.
        let mut eternity_challenges = [0u8; 12];
        for (key, count) in &dto.eternity_challs {
            if let Some(id) = key
                .strip_prefix("eterc")
                .and_then(|n| n.parse::<usize>().ok())
            {
                if (1..=12).contains(&id) {
                    eternity_challenges[id - 1] =
                        (*count).min(crate::eternity_challenges::EC_MAX_COMPLETIONS);
                }
            }
        }

        // Parse the recent-realities tuples: `[time, realTime, RM-string,
        // realityCount, ...]`.
        let mut recent_realities = Vec::with_capacity(10);
        for entry in dto.records.recent_realities.iter().take(10) {
            let parsed = entry.as_array().and_then(|t| {
                Some(crate::records::RecentReality {
                    time_ms: t.first()?.as_f64()?,
                    real_time_ms: t.get(1)?.as_f64()?,
                    rm: parse_decimal(t.get(2)?)?,
                    reality_count: t.get(3)?.as_f64()?,
                })
            });
            recent_realities
                .push(parsed.unwrap_or_else(crate::records::RecentReality::placeholder));
        }
        while recent_realities.len() < 10 {
            recent_realities.push(crate::records::RecentReality::placeholder());
        }

        let records = Records {
            total_time_played_ms: dto.records.total_time_played,
            real_time_played_ms: dto.records.real_time_played,
            time_played_at_bh_unlock_ms: dto.records.time_played_at_bh_unlock,
            this_infinity: ThisInfinity {
                time_ms: dto.records.this_infinity.time,
                real_time_ms: dto.records.this_infinity.real_time,
                max_am: dto.records.this_infinity.max_am,
                // Transient IC8 decay timer: start it at the current time on load so
                // production isn't spuriously decayed before the next purchase.
                last_buy_time_ms: dto.records.this_infinity.time,
                best_ip_min: dto.records.this_infinity.best_ip_min,
                best_ip_min_val: dto.records.this_infinity.best_ip_min_val,
            },
            best_infinity: BestInfinity {
                time_ms: dto.records.best_infinity.time,
                real_time_ms: dto.records.best_infinity.real_time,
            },
            this_eternity: ThisEternity {
                time_ms: dto.records.this_eternity.time,
                real_time_ms: dto.records.this_eternity.real_time,
                max_am: dto.records.this_eternity.max_am,
                max_ip: dto.records.this_eternity.max_ip,
                best_ep_min: dto.records.this_eternity.best_ep_min,
                best_ep_min_val: dto.records.this_eternity.best_ep_min_val,
            },
            best_eternity: BestEternity {
                time_ms: dto.records.best_eternity.time,
                real_time_ms: dto.records.best_eternity.real_time,
            },
            recent_eternities,
            this_reality: crate::records::ThisReality {
                time_ms: dto.records.this_reality.time,
                real_time_ms: dto.records.this_reality.real_time,
                max_ep: dto.records.this_reality.max_ep,
                max_replicanti: dto.records.this_reality.max_replicanti,
                max_dt: dto.records.this_reality.max_dt,
            },
            best_reality: crate::records::BestReality {
                time_ms: dto.records.best_reality.time,
                real_time_ms: dto.records.best_reality.real_time,
                rm_min: dto.records.best_reality.rm_min,
                glyph_level: dto.records.best_reality.glyph_level,
                best_ep: dto.records.best_reality.best_ep,
                glyph_strength: dto.records.best_reality.glyph_strength,
            },
            recent_realities,
        };

        // Glyphs: unknown (celestial) types are skipped; ids/idx are
        // normalized to integers.
        let parse_glyphs = |list: &[GlyphDTO]| -> Vec<crate::glyphs::Glyph> {
            list.iter()
                .filter_map(|g| {
                    let kind = crate::glyphs::GlyphType::from_save_id(&g.kind)?;
                    Some(crate::glyphs::Glyph {
                        id: g.id.unwrap_or(0.0).max(0.0) as u32,
                        idx: g.idx.unwrap_or(0.0).max(0.0) as u32,
                        kind,
                        strength: g.strength,
                        level: g.level.max(1.0) as u32,
                        raw_level: g.raw_level.unwrap_or(g.level).max(0.0) as u32,
                        effects: g.effects,
                    })
                })
                .collect()
        };
        let glyphs = crate::glyphs::GlyphState {
            active: parse_glyphs(&dto.reality.glyphs.active),
            inventory: parse_glyphs(&dto.reality.glyphs.inventory),
            sac: {
                let mut sac = [0.0f64; 5];
                for (i, kind) in crate::glyphs::BASIC_GLYPH_TYPES.iter().enumerate() {
                    if let Some(v) = dto.reality.glyphs.sac.get(kind.save_id()) {
                        sac[i] = *v;
                    }
                }
                sac
            },
            protected_rows: dto.reality.glyphs.protected_rows,
        };

        // Reality-layer state (`player.reality` + the root `realities`).
        let reality = {
            let mut rebuyables = [0u32; 5];
            for (key, count) in &dto.reality.rebuyables {
                if let Ok(id) = key.parse::<usize>() {
                    if (1..=5).contains(&id) {
                        rebuyables[id - 1] = *count;
                    }
                }
            }
            crate::reality::RealityState {
                machines: dto.reality.reality_machines,
                max_rm: dto.reality.max_rm,
                realities: dto.realities.max(0.0) as u32,
                perk_points: dto.reality.perk_points,
                perks: dto.reality.perks.iter().copied().collect(),
                seed: dto.reality.seed,
                initial_seed: dto.reality.initial_seed,
                second_gaussian: dto.reality.second_gaussian,
                upgrade_bits: dto.reality.upgrade_bits,
                upg_reqs: dto.reality.upg_reqs,
                req_lock: dto.reality.req_lock.reality,
                rebuyables,
                respec: dto.reality.respec,
                ach_timer: dto.reality.ach_timer,
                auto_achieve: dto.reality.auto_achieve,
                gained_auto_achievements: dto.reality.gained_auto_achievements,
                glyphs,
                automator_force_unlock: dto.reality.automator.force_unlock,
            }
        };
        let requirement_checks = crate::reality::RequirementChecks {
            eternity_no_rg: dto.requirement_checks.eternity.no_rg,
            reality_no_infinities: dto.requirement_checks.reality.no_infinities,
            reality_no_eternities: dto.requirement_checks.reality.no_eternities,
            reality_max_glyphs: dto.requirement_checks.reality.max_glyphs,
        };

        // Achievement bitmask. The original's `achievementBits` is 17 rows in a
        // fresh or pre-Pelle save and grows to 18 the moment a row-18 (Pelle)
        // achievement is touched. Accept either length and zero-fill the missing
        // Pelle row, so we can load *any* original save — including a Doomed one
        // — while ignoring the Pelle mechanics we don't model. Any other length
        // signals an unexpected save format.
        let bits_len = dto.achievement_bits.len();
        if bits_len != ACHIEVEMENT_ROW_COUNT && bits_len != ACHIEVEMENT_ROW_COUNT - 1 {
            return Err(SaveError::UnexpectedArrayLength {
                field: "achievementBits",
                expected: ACHIEVEMENT_ROW_COUNT,
                found: bits_len,
            });
        }
        let mut achievement_bits = [0u32; ACHIEVEMENT_ROW_COUNT];
        achievement_bits[..bits_len].copy_from_slice(&dto.achievement_bits);

        // The per-tier autobuyer array is fixed-length for the same reason.
        let ad_autobuyers = &dto.auto.antimatter_dims.all;
        if ad_autobuyers.len() != DIMENSION_COUNT {
            return Err(SaveError::UnexpectedArrayLength {
                field: "auto.antimatterDims.all",
                expected: DIMENSION_COUNT,
                found: ad_autobuyers.len(),
            });
        }
        // Rebuild autobuyers from defaults (fixed intervals, zeroed timers) and
        // overlay only the saved active/bought/mode flags (§4.4).
        let mut autobuyers = AutobuyerState::new();
        autobuyers.enabled = dto.auto.autobuyers_on;
        for (tier, ab) in autobuyers.dimensions.iter_mut().enumerate() {
            let src = &ad_autobuyers[tier];
            ab.is_active = src.is_active;
            ab.is_bought = src.is_bought;
            ab.mode = autobuyer_mode_from_raw(src.mode)?;
            // Interval-upgrade state round-trips (Feature 2.6).
            ab.interval_ms = src.interval;
            ab.cost = src.cost;
        }
        // The tickspeed autobuyer's mode is locked to single pre-Infinity for us,
        // so only its active/bought flags (and interval-upgrade state) are taken.
        autobuyers.tickspeed.is_active = dto.auto.tickspeed.is_active;
        autobuyers.tickspeed.is_bought = dto.auto.tickspeed.is_bought;
        autobuyers.tickspeed.interval_ms = dto.auto.tickspeed.interval;
        autobuyers.tickspeed.cost = dto.auto.tickspeed.cost;
        // The prestige autobuyers (Dim Boost / Galaxy / Big Crunch): active flag +
        // interval-upgrade state. They unlock by challenge, not `is_bought`.
        for (ab, is_active, interval, cost) in [
            (
                &mut autobuyers.dim_boost,
                dto.auto.dim_boost.is_active,
                dto.auto.dim_boost.interval,
                dto.auto.dim_boost.cost,
            ),
            (
                &mut autobuyers.galaxy,
                dto.auto.galaxy.is_active,
                dto.auto.galaxy.interval,
                dto.auto.galaxy.cost,
            ),
            (
                &mut autobuyers.big_crunch,
                dto.auto.big_crunch.is_active,
                dto.auto.big_crunch.interval,
                dto.auto.big_crunch.cost,
            ),
        ] {
            ab.is_active = is_active;
            ab.interval_ms = interval;
            ab.cost = cost;
        }
        // Big Crunch goal settings + the Eternity / Reality autobuyers.
        autobuyers.big_crunch_settings = PrestigeGoalSettings {
            mode: prestige_goal_mode_from_raw(dto.auto.big_crunch.mode)?,
            amount: dto.auto.big_crunch.amount,
            increase_with_mult: dto.auto.big_crunch.increase_with_mult,
            time: dto.auto.big_crunch.time,
            x_highest: dto.auto.big_crunch.x_highest,
        };
        autobuyers.eternity = EternityAutobuyer {
            is_active: dto.auto.eternity.is_active,
            settings: PrestigeGoalSettings {
                mode: prestige_goal_mode_from_raw(dto.auto.eternity.mode)?,
                amount: dto.auto.eternity.amount,
                increase_with_mult: dto.auto.eternity.increase_with_mult,
                time: dto.auto.eternity.time,
                x_highest: dto.auto.eternity.x_highest,
            },
        };
        autobuyers.reality = RealityAutobuyer {
            is_active: dto.auto.reality.is_active,
            mode: match dto.auto.reality.mode {
                0 => AutoRealityMode::Rm,
                1 => AutoRealityMode::Glyph,
                2 => AutoRealityMode::Either,
                3 => AutoRealityMode::Both,
                4 => AutoRealityMode::Time,
                // RELIC_SHARD (5) is Effarig content — past the frontier it
                // resets to the RM default rather than erroring.
                5 => AutoRealityMode::Rm,
                other => return Err(SaveError::InvalidAutobuyerMode(other)),
            },
            rm: dto.auto.reality.rm,
            glyph: dto.auto.reality.glyph.max(0.0) as u32,
            time: dto.auto.reality.time,
        };

        // Options: numeric values must be in range — we reject rather than clamp.
        // Notation is the one intentional exception: a name we don't model (the
        // game default "Mixed scientific" included) is ignored, keeping our
        // default, since we implement only a subset of notations.
        let mut options = Options::new();
        options.hotkeys = dto.options.hotkeys;
        options.set_update_rate(check_range(
            "options.updateRate",
            dto.options.update_rate,
            MIN_UPDATE_RATE_MS,
            MAX_UPDATE_RATE_MS,
        )?);
        options.set_notation(&dto.options.notation);
        let comma = check_range(
            "options.notationDigits.comma",
            dto.options.notation_digits.comma,
            MIN_NOTATION_DIGITS,
            MAX_NOTATION_DIGITS,
        )?;
        let notation = check_range(
            "options.notationDigits.notation",
            dto.options.notation_digits.notation,
            MIN_NOTATION_DIGITS,
            MAX_NOTATION_DIGITS,
        )?;
        options.set_notation_digits(comma, notation);
        // Offline ticks are intentionally *not* range-checked: our slider range
        // diverges from the original's, so we accept any positive value from the
        // save as-is (§ offline-progress design).
        options.set_offline_ticks(dto.options.offline_ticks);
        // Autosave interval matches the original's slider range, so it *is*
        // range-checked (reject rather than clamp), like updateRate.
        options.set_autosave_interval(check_range(
            "options.autosaveInterval",
            dto.options.autosave_interval,
            MIN_AUTOSAVE_INTERVAL_MS,
            MAX_AUTOSAVE_INTERVAL_MS,
        )?);
        options.show_time_since_save = dto.options.show_time_since_save;
        options.set_save_file_name(&dto.options.save_file_name);
        // Confirmation toggles are plain booleans (no range checks); read the
        // modelled subset straight through.
        options.confirmations = Confirmations {
            dimension_boost: dto.options.confirmations.dimension_boost,
            antimatter_galaxy: dto.options.confirmations.antimatter_galaxy,
            sacrifice: dto.options.confirmations.sacrifice,
            big_crunch: dto.options.confirmations.big_crunch,
            eternity: dto.options.confirmations.eternity,
            dilation: dto.options.confirmations.dilation,
            switch_automator_mode: dto.options.confirmations.switch_automator_mode,
        };
        options.animations.big_crunch = dto.options.animations.big_crunch;
        options.show_hint_text = ShowHintText {
            show_percentage: dto.options.show_hint_text.show_percentage,
            achievements: dto.options.show_hint_text.achievements,
            achievement_unlock_states: dto
                .options
                .show_hint_text
                .achievement_unlock_states,
            challenges: dto.options.show_hint_text.challenges,
        };
        options.away_progress = AwayProgress {
            antimatter: dto.options.away_progress.antimatter,
            dimension_boosts: dto.options.away_progress.dimension_boosts,
            antimatter_galaxies: dto.options.away_progress.antimatter_galaxies,
            infinities: dto.options.away_progress.infinities,
            infinity_points: dto.options.away_progress.infinity_points,
            replicanti: dto.options.away_progress.replicanti,
            replicanti_galaxies: dto.options.away_progress.replicanti_galaxies,
        };
        options.header_text_colored = dto.options.header_text_colored;
        options.set_sidebar_resource(dto.options.sidebar_resource_id);
        // Hidden-tab state: the original always writes 11 per-tab bitmask
        // entries; a different length signals an unexpected save format.
        if dto.options.hidden_subtab_bits.len() != TAB_COUNT {
            return Err(SaveError::UnexpectedArrayLength {
                field: "options.hiddenSubtabBits",
                expected: TAB_COUNT,
                found: dto.options.hidden_subtab_bits.len(),
            });
        }
        options.hidden_tab_bits = dto.options.hidden_tab_bits;
        options.hidden_subtab_bits =
            std::array::from_fn(|i| dto.options.hidden_subtab_bits[i]);
        options.automator_events = crate::options::AutomatorEventsOptions {
            newest_first: dto.options.automator_events.newest_first,
            timestamp_type: dto.options.automator_events.timestamp_type.clamp(0, 4)
                as u8,
            max_entries: dto.options.automator_events.max_entries.max(1),
            clear_on_reality: dto.options.automator_events.clear_on_reality,
            clear_on_restart: dto.options.automator_events.clear_on_restart,
        };

        Ok(GameState {
            antimatter: dto.antimatter,
            total_antimatter: dto.records.total_antimatter,
            dimensions,
            tickspeed: TickspeedState::with_bought_and_bumps(
                dto.total_tick_bought,
                dto.chall9_tickspeed_cost_bumps,
            ),
            dim_boosts: dto.dimension_boosts,
            galaxies: dto.galaxies,
            sacrificed: dto.sacrificed,
            infinity_points: dto.infinity_points,
            infinities: dto.infinities,
            eternity_points: dto.eternity_points,
            eternities: dto.eternities,
            eternity_unlocked,
            time_dimensions,
            time_shards: dto.time_shards,
            total_tick_gained: dto.total_tick_gained,
            time_theorems: dto.timestudy.theorem,
            max_theorem: dto.timestudy.max_theorem,
            tt_am_bought: dto.timestudy.am_bought,
            tt_ip_bought: dto.timestudy.ip_bought,
            tt_ep_bought: dto.timestudy.ep_bought,
            studies: dto.timestudy.studies.clone(),
            automator: automator_from_dto(&dto.reality.automator),
            study_presets: {
                let mut presets: [crate::time_studies::StudyPreset; 6] =
                    Default::default();
                for (slot, src) in dto.timestudy.presets.iter().take(6).enumerate() {
                    presets[slot] = crate::time_studies::StudyPreset {
                        name: src.name.clone(),
                        studies: src.studies.clone(),
                    };
                }
                presets
            },
            respec: dto.respec,
            infinities_banked: dto.infinities_banked,
            eternity_challenge_unlocked: dto.challenge.eternity.unlocked,
            eternity_challenges,
            eternity_challenge_current: dto.challenge.eternity.current,
            ec_requirement_bits: dto.challenge.eternity.requirement_bits,
            eterc8_ids: dto.eterc8ids,
            eterc8_repl: dto.eterc8repl,
            reality,
            requirement_checks,
            black_holes: {
                let mut state = crate::black_holes::BlackHolesState::new();
                for (i, hole) in dto.black_hole.iter().take(2).enumerate() {
                    state.holes[i] = crate::black_holes::BlackHole {
                        unlocked: hole.unlocked,
                        active: hole.active,
                        phase: hole.phase,
                        activations: hole.activations.max(0.0) as u32,
                        interval_upgrades: hole.interval_upgrades.max(0.0) as u32,
                        power_upgrades: hole.power_upgrades.max(0.0) as u32,
                        duration_upgrades: hole.duration_upgrades.max(0.0) as u32,
                    };
                }
                state.paused = dto.black_hole_pause;
                state.pause_time_ms = dto.black_hole_pause_time;
                state
            },
            dilation: {
                let mut rebuyables = [0u32; 3];
                for (key, count) in &dto.dilation.rebuyables {
                    if let Ok(id) = key.parse::<usize>() {
                        if (1..=3).contains(&id) {
                            rebuyables[id - 1] = *count;
                        }
                    }
                }
                let mut upgrades = 0u32;
                for id in &dto.dilation.upgrades {
                    if (4..=10).contains(id) {
                        upgrades |= 1 << id;
                    }
                }
                DilationState {
                    studies: dto.dilation.studies.clone(),
                    active: dto.dilation.active,
                    tachyon_particles: dto.dilation.tachyon_particles,
                    dilated_time: dto.dilation.dilated_time,
                    next_threshold: dto.dilation.next_threshold,
                    base_tachyon_galaxies: dto.dilation.base_tachyon_galaxies.max(0.0)
                        as u32,
                    total_tachyon_galaxies: dto.dilation.total_tachyon_galaxies,
                    upgrades,
                    rebuyables,
                    last_ep: dto.dilation.last_ep,
                }
            },
            eternity_upgrades: {
                let mut bits = 0u32;
                for id in &dto.eternity_upgrades {
                    if let Some(upgrade) = crate::EternityUpgrade::from_id(*id) {
                        bits |= upgrade.bit();
                    }
                }
                bits
            },
            epmult_upgrades: dto.epmult_upgrades,
            ic_best_times_ms: {
                let mut times = [f64::MAX; 8];
                for (i, t) in
                    dto.challenge.infinity.best_times.iter().take(8).enumerate()
                {
                    times[i] = *t;
                }
                times
            },
            infinity_upgrades,
            part_infinity_point: dto.part_infinity_point,
            challenge: NormalChallengeState {
                current: dto.challenge.normal.current,
                completed: dto.challenge.normal.completed_bits,
            },
            infinity_challenge: InfinityChallengeState {
                current: dto.challenge.infinity.current,
                completed: dto.challenge.infinity.completed_bits,
            },
            // Transient per-run challenge counters (re-established on the next
            // purchase / tick); defaulted rather than round-tripped.
            post_c4_tier: 1,
            ic2_count: 0.0,
            chall8_total_sacrifice: dto.chall8_total_sacrifice,
            chall2_pow: dto.chall2_pow,
            chall3_pow: dto.chall3_pow,
            matter: dto.matter,
            infinity_unlocked,
            broke_infinity,
            break_infinity_upgrades,
            infinity_rebuyables,
            infinity_dimensions,
            infinity_power: dto.infinity_power,
            replicanti,
            records,
            achievement_bits,
            tab_notifications: dto.tab_notifications.iter().cloned().collect(),
            triggered_tab_notification_bits: dto.triggered_tab_notification_bits,
            tutorial_state: dto.tutorial_state,
            tutorial_active: dto.tutorial_active,
            autobuyers,
            options,
        })
    }
}

/// Maps the original numeric `AUTOBUYER_MODE` to ours: `1` (BUY_SINGLE) →
/// `BuySingle`, `10` (BUY_10) → `BuyMax`. Any other value indicates a malformed
/// or unsupported save and is rejected with [`SaveError::InvalidAutobuyerMode`].
fn autobuyer_mode_from_raw(mode: i64) -> Result<AutobuyerMode, SaveError> {
    match mode {
        AUTOBUYER_MODE_BUY_SINGLE => Ok(AutobuyerMode::BuySingle),
        AUTOBUYER_MODE_BUY_10 => Ok(AutobuyerMode::BuyMax),
        other => Err(SaveError::InvalidAutobuyerMode(other)),
    }
}

/// Maps the original numeric `AUTO_CRUNCH_MODE` / `AUTO_ETERNITY_MODE`
/// (0 amount / 1 time / 2 X highest) to [`PrestigeAutobuyerMode`].
fn prestige_goal_mode_from_raw(mode: i64) -> Result<PrestigeAutobuyerMode, SaveError> {
    match mode {
        0 => Ok(PrestigeAutobuyerMode::Amount),
        1 => Ok(PrestigeAutobuyerMode::Time),
        2 => Ok(PrestigeAutobuyerMode::XHighest),
        other => Err(SaveError::InvalidAutobuyerMode(other)),
    }
}

/// Map `player.reality.automator` into engine state. Ids/scripts and
/// constants load strictly by shape; the run-state stack is parsed leniently
/// (an unrecognized `commandState` clears the stack — the original resets a
/// run it can't resume anyway).
fn automator_from_dto(dto: &RealityAutomatorDTO) -> crate::automator::AutomatorData {
    use crate::automator::{
        AutomatorEditorType, AutomatorMode, AutomatorScript, AutomatorStateData,
        StackEntryData,
    };

    let mut scripts = std::collections::BTreeMap::new();
    for (key, script) in &dto.scripts {
        if let Ok(id) = key.parse::<u32>() {
            scripts.insert(
                id,
                AutomatorScript {
                    name: script.name.clone(),
                    content: script.content.clone(),
                },
            );
        }
    }
    if scripts.is_empty() {
        scripts.insert(
            1,
            AutomatorScript {
                name: "New Script".to_string(),
                content: String::new(),
            },
        );
    }

    let constants: std::collections::HashMap<String, String> = dto
        .constants
        .iter()
        .map(|(k, v)| {
            let value = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (k.clone(), value)
        })
        .collect();
    // Keep only ordering entries that name real constants, then append any
    // constants the order list missed (defensive; the original keeps the two
    // in lockstep).
    let mut constant_sort_order: Vec<String> = dto
        .constant_sort_order
        .iter()
        .filter(|n| constants.contains_key(*n))
        .cloned()
        .collect();
    for name in constants.keys() {
        if !constant_sort_order.contains(name) {
            constant_sort_order.push(name.clone());
        }
    }

    let stack: Option<Vec<StackEntryData>> = dto
        .state
        .stack
        .iter()
        .map(|entry| {
            let line_number = entry.get("lineNumber")?.as_f64()? as u32;
            let command_state = match entry.get("commandState") {
                None | Some(serde_json::Value::Null) => None,
                Some(state) => Some(parse_command_state(state)?),
            };
            Some(StackEntryData {
                line_number,
                command_state,
            })
        })
        .collect();

    let first_script = *scripts.keys().next().expect("non-empty");
    let clamp_script = |id: i64| {
        let id = id.max(0) as u32;
        if scripts.contains_key(&id) {
            id
        } else {
            first_script
        }
    };

    crate::automator::AutomatorData {
        runtime: Default::default(),
        state: AutomatorStateData {
            mode: match dto.state.mode {
                Some(2) => AutomatorMode::Run,
                Some(3) => AutomatorMode::SingleStep,
                // 1, unknown values, and the fresh save's missing key are
                // all paused.
                _ => AutomatorMode::Pause,
            },
            top_level_script: clamp_script(dto.state.top_level_script),
            editor_script: clamp_script(dto.state.editor_script),
            repeat: dto.state.repeat,
            force_restart: dto.state.force_restart,
            follow_execution: dto.state.follow_execution,
            stack: stack.unwrap_or_default(),
        },
        scripts,
        constants,
        constant_sort_order,
        editor_type: if dto.editor_type == 1 {
            AutomatorEditorType::Block
        } else {
            AutomatorEditorType::Text
        },
        current_info_pane: dto.current_info_pane.clamp(0, 7) as u8,
        exec_timer: dto.exec_timer,
    }
}

/// One persisted `commandState` (`{timeMs}` / `{prestigeLevel}` /
/// `{advanceOnPop, ifEndLine}`).
fn parse_command_state(
    state: &serde_json::Value,
) -> Option<crate::automator::CommandStateData> {
    use crate::automator::CommandStateData;
    if let Some(time_ms) = state.get("timeMs").and_then(|v| v.as_f64()) {
        return Some(CommandStateData::Pause { time_ms });
    }
    if let Some(level) = state.get("prestigeLevel").and_then(|v| v.as_f64()) {
        return Some(CommandStateData::PrestigeLevel { level: level as u8 });
    }
    if let Some(advance_on_pop) = state.get("advanceOnPop").and_then(|v| v.as_bool()) {
        let if_end_line = state.get("ifEndLine").and_then(|v| v.as_f64())? as u32;
        return Some(CommandStateData::IfEntered {
            advance_on_pop,
            if_end_line,
        });
    }
    None
}

/// Validates that a modelled numeric option lies within its accepted range,
/// returning [`SaveError::OptionOutOfRange`] (rather than clamping) if not.
fn check_range(
    field: &'static str,
    value: u32,
    min: u32,
    max: u32,
) -> Result<u32, SaveError> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(SaveError::OptionOutOfRange {
            field,
            value,
            min,
            max,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::json;

    use super::*;
    use crate::options::{
        DEFAULT_NOTATION, DEFAULT_NOTATION_DIGITS_COMMA,
        DEFAULT_NOTATION_DIGITS_NOTATION, DEFAULT_UPDATE_RATE_MS,
    };
    use crate::save::{decode_pipeline, decode_save};

    const INITIAL_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_initial_save.txt"
    ));
    const SAMPLE_SAVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ad_sample_save.txt"
    ));

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// A complete, valid `player` JSON value (the fresh-start fixture) that tests
    /// mutate to exercise individual fields. Starting from a real save keeps the
    /// now-required fields present.
    fn base_player() -> serde_json::Value {
        let json = decode_pipeline(INITIAL_SAVE.trim()).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// Parses a `player` JSON value through the DTO and maps it to a `GameState`,
    /// mirroring `decode_save` minus the byte pipeline.
    fn load(player: serde_json::Value) -> Result<GameState, SaveError> {
        let dto: PlayerDTO = serde_json::from_value(player)?;
        GameState::from_save_dto(&dto)
    }

    #[test]
    fn decodes_initial_save() {
        let state = decode_save(INITIAL_SAVE.trim()).unwrap();

        assert_eq!(state.antimatter, dec("10"));
        assert_eq!(state.total_antimatter, dec("10"));
        assert_eq!(state.dim_boosts, 0);
        assert_eq!(state.galaxies, 0);
        assert_eq!(state.tickspeed.bought, 0);
        assert_eq!(state.sacrificed, Decimal::ZERO);
        assert!(!state.infinity_unlocked);
        // Fresh-start Infinity currency is zero; best infinity is the "none yet"
        // sentinel (Number.MAX_VALUE == f64::MAX).
        assert_eq!(state.infinity_points, Decimal::ZERO);
        assert_eq!(state.infinities, Decimal::ZERO);
        assert_eq!(state.records.best_infinity.time_ms, f64::MAX);
        assert!(state.dimensions.iter().all(|d| d.bought == 0));

        // Autobuyers: globally on, none unlocked yet, dims default to buy-max.
        assert!(state.autobuyers.enabled);
        assert!(!state.autobuyers.dimensions[0].is_bought);
        assert!(state.autobuyers.dimensions[0].is_active);
        assert_eq!(state.autobuyers.dimensions[0].mode, AutobuyerMode::BuyMax);

        // Options: defaults, and the save's "Mixed scientific" (which we don't
        // model) is ignored, leaving our default notation.
        assert!(state.options.hotkeys);
        assert_eq!(state.options.update_rate, DEFAULT_UPDATE_RATE_MS);
        assert_eq!(state.options.notation, DEFAULT_NOTATION);
        assert_eq!(
            state.options.notation_digits_comma,
            DEFAULT_NOTATION_DIGITS_COMMA
        );
        assert_eq!(
            state.options.notation_digits_notation,
            DEFAULT_NOTATION_DIGITS_NOTATION
        );
    }

    #[test]
    fn decodes_sample_save() {
        let state = decode_save(SAMPLE_SAVE.trim()).unwrap();

        assert_eq!(state.antimatter, dec("16613773273375400000"));
        assert_eq!(state.total_antimatter, dec("3.3579029107185e+134"));
        assert_eq!(state.galaxies, 1);
        assert_eq!(state.dim_boosts, 0);
        assert_eq!(state.sacrificed, Decimal::ZERO);
        assert!(!state.infinity_unlocked);

        // Tickspeed: only the purchased count is stored; cost is recomputed.
        assert_eq!(state.tickspeed.bought, 12);
        assert_eq!(state.tickspeed.cost, TickspeedState::with_bought(12).cost);

        // Dimension purchase counts and the first tier's fractional amount.
        let bought: Vec<u64> = state.dimensions.iter().map(|d| d.bought).collect();
        assert_eq!(bought, vec![50, 30, 20, 20, 0, 0, 0, 0]);
        assert_eq!(state.dimensions[0].amount, dec("43777257640570.91"));

        assert_eq!(state.options.notation, DEFAULT_NOTATION);
        assert_eq!(state.options.update_rate, DEFAULT_UPDATE_RATE_MS);
    }

    #[test]
    fn loads_infinity_points_infinities_and_records() {
        let mut player = base_player();
        player["infinityPoints"] = json!("1.5e3");
        player["infinities"] = json!("42");
        player["records"]["totalTimePlayed"] = json!(600_000.0);
        player["records"]["thisInfinity"]["time"] = json!(12_345.0);
        player["records"]["thisInfinity"]["maxAM"] = json!("1e100");
        player["records"]["bestInfinity"]["time"] = json!(30_000.0);

        let state = load(player).unwrap();
        assert_eq!(state.infinity_points, dec("1.5e3"));
        assert_eq!(state.infinities, dec("42"));
        assert_eq!(state.records.total_time_played_ms, 600_000.0);
        assert_eq!(state.records.this_infinity.time_ms, 12_345.0);
        assert_eq!(state.records.this_infinity.max_am, dec("1e100"));
        assert_eq!(state.records.best_infinity.time_ms, 30_000.0);
        // Any IP/infinities gained implies Infinity is unlocked.
        assert!(state.infinity_unlocked);
    }

    #[test]
    fn loads_infinity_upgrades_and_part_ip() {
        let mut player = base_player();
        player["infinityUpgrades"] = json!(["timeMult", "18Mult", "dimMult"]);
        player["partInfinityPoint"] = json!(0.75);
        // An unmodelled id must not fail the load (forward-compat).
        player["infinityUpgrades"]
            .as_array_mut()
            .unwrap()
            .push(json!("someFutureUpgrade"));

        let state = load(player).unwrap();
        assert!(state.infinity_upgrade_bought(InfinityUpgrade::TotalTimeMult));
        assert!(state.infinity_upgrade_bought(InfinityUpgrade::Dim18Mult));
        assert!(state.infinity_upgrade_bought(InfinityUpgrade::Buy10Mult));
        assert!(!state.infinity_upgrade_bought(InfinityUpgrade::Dim27Mult));
        assert_eq!(state.part_infinity_point, 0.75);
    }

    #[test]
    fn infinity_unlocked_from_break_or_progress() {
        let mut by_break = base_player();
        by_break["break"] = json!(true);
        assert!(load(by_break).unwrap().infinity_unlocked);

        let mut by_infinities = base_player();
        by_infinities["infinities"] = json!("1");
        assert!(load(by_infinities).unwrap().infinity_unlocked);

        let mut by_ip = base_player();
        by_ip["infinityPoints"] = json!("5.5e10");
        assert!(load(by_ip).unwrap().infinity_unlocked);

        // The fresh-start base has break=false, infinities="0", IP="0".
        assert!(!load(base_player()).unwrap().infinity_unlocked);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Unmodelled mechanics in the save must not prevent a load.
        let mut player = base_player();
        player["celestials"] = json!({ "teresa": { "pouredAmount": 1 } });
        player["someBrandNewField"] = json!(42);

        let state = load(player).unwrap();
        assert_eq!(state.antimatter, dec("10"));
    }

    #[test]
    fn missing_modelled_field_errors() {
        // Dropping a field we model must fail the load (surfacing a format change)
        // rather than silently defaulting.
        let mut player = base_player();
        player.as_object_mut().unwrap().remove("antimatter");
        assert!(matches!(load(player), Err(SaveError::Json(_))));

        let mut player = base_player();
        player["records"]
            .as_object_mut()
            .unwrap()
            .remove("totalAntimatter");
        assert!(matches!(load(player), Err(SaveError::Json(_))));

        // Newer modelled fields are required too (no serde defaults): a save
        // missing them is rejected rather than silently defaulted.
        let mut player = base_player();
        player.as_object_mut().unwrap().remove("tutorialActive");
        assert!(matches!(load(player), Err(SaveError::Json(_))));

        let mut player = base_player();
        player["options"]
            .as_object_mut()
            .unwrap()
            .remove("confirmations");
        assert!(matches!(load(player), Err(SaveError::Json(_))));

        let mut player = base_player();
        player["options"]["confirmations"]
            .as_object_mut()
            .unwrap()
            .remove("sacrifice");
        assert!(matches!(load(player), Err(SaveError::Json(_))));
    }

    #[test]
    fn wrong_array_length_errors() {
        // A dimensions array that isn't exactly 8 long is an unexpected shape.
        let mut player = base_player();
        player["dimensions"]["antimatter"]
            .as_array_mut()
            .unwrap()
            .truncate(7);
        assert!(matches!(
            load(player),
            Err(SaveError::UnexpectedArrayLength {
                field: "dimensions.antimatter",
                expected: 8,
                found: 7,
            })
        ));

        // Likewise for the per-tier autobuyer array.
        let mut player = base_player();
        player["auto"]["antimatterDims"]["all"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(matches!(
            load(player),
            Err(SaveError::UnexpectedArrayLength {
                field: "auto.antimatterDims.all",
                ..
            })
        ));
    }

    #[test]
    fn achievement_bits_accepts_17_or_18_rows() {
        // A fresh/pre-Pelle original save has 17 rows; it loads and the missing
        // Pelle row is zero-filled.
        let mut player = base_player();
        player["achievementBits"] = json!(vec![0u32; 17]);
        let state = load(player).unwrap();
        assert_eq!(state.achievement_bits, [0u32; ACHIEVEMENT_ROW_COUNT]);

        // A Doomed (Pelle) save has grown `achievementBits` to 18 rows. We load
        // it even though we model no Pelle mechanic; row-18 bits round-trip.
        let mut bits = vec![0u32; 18];
        bits[0] = 1 << 7; // achievement 18 (row 1, col 8)
        bits[17] = 0b1010_1010; // some row-18 (Pelle) achievements, ids 182/184/186/188
        let mut player = base_player();
        player["achievementBits"] = json!(bits);
        let state = load(player).unwrap();
        assert!(state.achievement_unlocked(18));
        assert!(state.achievement_unlocked(182));
        assert!(state.achievement_unlocked(188));
        assert!(!state.achievement_unlocked(181));
        assert_eq!(state.achievement_bits[17], 0b1010_1010);
    }

    #[test]
    fn achievement_bits_wrong_length_errors() {
        // Anything other than 17 or 18 rows is still an unexpected shape.
        let mut player = base_player();
        player["achievementBits"] = json!(vec![0u32; 16]);
        assert!(matches!(
            load(player),
            Err(SaveError::UnexpectedArrayLength {
                field: "achievementBits",
                expected: ACHIEVEMENT_ROW_COUNT,
                found: 16,
            })
        ));
    }

    #[test]
    fn valid_in_range_options_are_applied() {
        let mut player = base_player();
        player["options"]["hotkeys"] = json!(false);
        player["options"]["updateRate"] = json!(100);
        player["options"]["notation"] = json!("Engineering");
        player["options"]["notationDigits"] = json!({ "comma": 4, "notation": 12 });
        // A value inside our extended slider range.
        player["options"]["offlineTicks"] = json!(5_000_000);

        let state = load(player).unwrap();
        assert!(!state.options.hotkeys);
        assert_eq!(state.options.update_rate, 100);
        assert_eq!(state.options.notation, "Engineering");
        assert_eq!(state.options.notation_digits_comma, 4);
        assert_eq!(state.options.notation_digits_notation, 12);
        assert_eq!(state.options.offline_ticks, 5_000_000);
    }

    #[test]
    fn offline_ticks_outside_slider_range_is_accepted() {
        // Unlike the other numeric options, offlineTicks is not range-checked:
        // our slider range diverges from the original's, so an imported value
        // (here the original's 1e6 max, below our 10M but a fine example) is
        // taken as-is rather than rejected.
        let mut player = base_player();
        player["options"]["offlineTicks"] = json!(500);
        assert_eq!(load(player).unwrap().options.offline_ticks, 500);
    }

    #[test]
    fn out_of_range_options_error() {
        let mut player = base_player();
        player["options"]["updateRate"] = json!(99999);
        assert!(matches!(
            load(player),
            Err(SaveError::OptionOutOfRange {
                field: "options.updateRate",
                ..
            })
        ));

        let mut player = base_player();
        player["options"]["notationDigits"]["notation"] = json!(99);
        assert!(matches!(
            load(player),
            Err(SaveError::OptionOutOfRange { .. })
        ));
    }

    #[test]
    fn unsupported_notation_is_kept_lenient() {
        // We model only a subset of notations, so an unknown name (here, and the
        // game default "Mixed scientific") is ignored rather than failing the
        // load — the one intentional leniency.
        let mut player = base_player();
        player["options"]["notation"] = json!("Totally Made Up Notation");
        assert_eq!(load(player).unwrap().options.notation, DEFAULT_NOTATION);
    }

    #[test]
    fn autobuyer_mode_and_flags_mapped() {
        let mut player = base_player();
        player["auto"]["autobuyersOn"] = json!(false);
        let tier0 = &mut player["auto"]["antimatterDims"]["all"][0];
        tier0["isActive"] = json!(false);
        tier0["isBought"] = json!(true);
        tier0["mode"] = json!(1);
        player["auto"]["tickspeed"]["isBought"] = json!(true);
        player["auto"]["tickspeed"]["mode"] = json!(10);

        let state = load(player).unwrap();

        assert!(!state.autobuyers.enabled);
        // Tier 0 overlaid from the save: single-buy, bought, inactive.
        assert_eq!(
            state.autobuyers.dimensions[0].mode,
            AutobuyerMode::BuySingle
        );
        assert!(state.autobuyers.dimensions[0].is_bought);
        assert!(!state.autobuyers.dimensions[0].is_active);
        // Tier 1 keeps the base save's values (buy-max, not yet unlocked).
        assert_eq!(state.autobuyers.dimensions[1].mode, AutobuyerMode::BuyMax);
        assert!(!state.autobuyers.dimensions[1].is_bought);
        assert!(state.autobuyers.dimensions[1].is_active);
        // Tickspeed flags taken from save, but mode stays locked to single.
        assert!(state.autobuyers.tickspeed.is_bought);
        assert_eq!(state.autobuyers.tickspeed.mode, AutobuyerMode::BuySingle);
    }

    #[test]
    fn rejects_invalid_autobuyer_mode() {
        // A dimension autobuyer `mode` that is neither 1 nor 10 is malformed.
        let mut player = base_player();
        player["auto"]["antimatterDims"]["all"][0]["mode"] = json!(7);
        match load(player) {
            Err(SaveError::InvalidAutobuyerMode(7)) => {}
            other => panic!("expected InvalidAutobuyerMode(7), got {other:?}"),
        }
    }
}
