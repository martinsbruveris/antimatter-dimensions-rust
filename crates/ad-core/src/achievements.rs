//! Normal achievements: persistent unlock state, effects, and the global
//! achievement-power multiplier.
//!
//! Unlock state is a bitmask array mirroring the original `player.achievementBits`
//! (18 rows, one bitmask each). An achievement `id` encodes its grid position as
//! `row = id / 10`, `column = id % 10`, and unlock is bit `column - 1` of
//! `achievement_bits[row - 1]` — identical to the original's
//! `achievementBits[row-1] & (1 << (column-1))`.
//!
//! Row 18 (ids 181-188) is the Pelle achievements. We model no Pelle mechanic
//! and never *unlock* those bits ourselves, but the array is sized to hold them
//! so a save from the original game — including a Doomed one whose
//! `achievementBits` has grown to length 18 — round-trips losslessly. They count
//! toward the global power exactly as the original's `Achievements.all` does
//! (the Doomed multiplier-disable is a separate mechanic we don't model).
//!
//! Unlocks are driven inline from the relevant action methods (buying a
//! dimension, a galaxy, a boost, crunching, and once per tick) rather than via an
//! event bus — our centralized action methods *are* the events. Effects are read
//! back in [`GameState::dimension_multiplier`] (per-dimension boosts + the global
//! power) and [`GameState::starting_antimatter`] (achievement 21). See
//! `docs/design/2026-06-30-achievements.md`.

use break_infinity::Decimal;

use crate::data::constants::INITIAL_ANTIMATTER;
use crate::state::GameState;

/// Number of achievement rows; mirrors the full length of
/// `player.achievementBits` once row 18 (the Pelle achievements) exists. A fresh
/// or pre-Pelle original save has only 17 rows; the save loader zero-fills the
/// 18th. See [`crate::save`].
pub const ACHIEVEMENT_ROW_COUNT: usize = 18;

/// Number of columns (achievements) per row.
pub const ACHIEVEMENTS_PER_ROW: u16 = 8;

/// The achievements the engine can currently award (an inline unlock hook
/// exists at the relevant seam). The Reality study's "all pre-Reality
/// achievements" requirement is checked against this set until achievement
/// coverage reaches rows 1–13 (see `docs/design/2026-07-05-reality.md`).
///
/// Excludes achievements the engine cannot earn naturally, which are only ever
/// set via Reality auto-achievement or the ACHNR reality upgrade: 22 (News),
/// 35 (6-hour offline), 61 (autobuyer bulk has no in-engine upgrade), 62/111
/// (`bestRunIPPM` / the recent-infinities ring is unmodelled), 65/74 (the
/// Normal-Challenge best-times sum is unmodelled), 117 (no ≥750 bulk
/// Dimension-Boost purchase path), 156 (`noPurchasedTT` unmodelled), 165 (the
/// per-factor glyph-level weights are unmodelled), and 172 (`noTriads`
/// unmodelled). Their *effects* are still wired where they have a consumption
/// site, so an auto-achieved unlock behaves correctly. Row 18 (Pelle) is never
/// awarded by design.
pub const IMPLEMENTED_ACHIEVEMENTS: &[u16] = &[
    11, 12, 13, 14, 15, 16, 17, 18, // row 1
    21, 23, 24, 25, 26, 27, 28, // row 2 (22 = News, deferred)
    31, 32, 33, 34, 36, 37, 38, // row 3 (35 = offline, deferred)
    41, 42, 43, 44, 45, 46, 47, 48, // row 4
    51, 52, 53, 54, 55, 56, 57, 58, // row 5
    63, 64, 66, 67, 68, // row 6 (61/62/65 deferred)
    71, 72, 73, 75, 76, 77, 78, // row 7 (74 deferred)
    81, 82, 83, 84, 85, 86, 87, 88, // row 8
    91, 92, 93, 94, 95, 96, 97, 98, // row 9
    101, 102, 103, 104, 105, 106, 107, 108, // row 10
    112, 113, 114, 115, 116, 118, // row 11 (111/117 deferred)
    121, 122, 123, 124, 125, 126, 127, 128, // row 12
    131, 132, 133, 134, 135, 136, 137, 138, // row 13
    141, 142, 143, 144, 145, 146, 147, 148, // row 14
    151, 152, 153, 154, 155, 157, 158, // row 15 (156 deferred)
    161, 162, 163, 164, 166, 167, 168, // row 16 (165 deferred)
    171, 173, 174, 175, 176, 177, 178, // row 17 (172 deferred)
];

impl GameState {
    /// `(row_index, column_bitmask)` for an achievement id, where
    /// `id = row * 10 + column` with `row ∈ 1..=18` and `column ∈ 1..=8`.
    fn achievement_index(id: u16) -> (usize, u32) {
        let row = (id / 10) as usize;
        let column = id % 10;
        (row - 1, 1 << (column - 1))
    }

    /// Whether achievement `id` is unlocked. Mirrors
    /// `player.achievementBits[row-1] & (1 << (column-1))`.
    pub fn achievement_unlocked(&self, id: u16) -> bool {
        let (row, mask) = Self::achievement_index(id);
        self.achievement_bits[row] & mask != 0
    }

    /// Unlock achievement `id`. Idempotent; achievements never re-lock,
    /// including across a Big Crunch. Call sites guard their own condition with
    /// a plain `if`, then call this — there is no separate `try_unlock` because,
    /// unlike the original's `tryUnlock`, we dispatch no events, so the set is
    /// already a no-op when the bit is held.
    pub(crate) fn unlock_achievement(&mut self, id: u16) {
        let (row, mask) = Self::achievement_index(id);
        if self.achievement_bits[row] & mask != 0 {
            return;
        }
        self.achievement_bits[row] |= mask;
        // Achievements 85/93 multiply IP gain ×4; the Big Crunch autobuyer's
        // "Dynamic amount" threshold scales along (`bumpAmount(4)`).
        if id == 85 || id == 93 {
            self.bump_big_crunch_amount(break_infinity::Decimal::from_float(4.0));
        }
    }

    /// Sorted list of unlocked achievement ids — the presentation-layer view of
    /// the bitmask (the snapshot exposes this rather than the raw bits).
    pub fn unlocked_achievement_ids(&self) -> Vec<u16> {
        let mut ids = Vec::new();
        for row in 1..=ACHIEVEMENT_ROW_COUNT as u16 {
            for column in 1..=ACHIEVEMENTS_PER_ROW {
                let id = row * 10 + column;
                if self.achievement_unlocked(id) {
                    ids.push(id);
                }
            }
        }
        ids
    }

    /// The global achievement-power multiplier, applied to every Antimatter
    /// Dimension: `1.25^(completed rows) × 1.03^(total unlocked)`. Mirrors
    /// `Achievements.power` pre-Reality, where the glyph/Ra exponent is 1.
    pub fn achievement_power(&self) -> Decimal {
        let mut completed_rows = 0u32;
        let mut total_unlocked = 0u32;
        for &row_bits in &self.achievement_bits {
            let row_count = (row_bits & 0xFF).count_ones();
            total_unlocked += row_count;
            if row_count == ACHIEVEMENTS_PER_ROW as u32 {
                completed_rows += 1;
            }
        }
        let power =
            1.25f64.powi(completed_rows as i32) * 1.03f64.powi(total_unlocked as i32);
        // Ra's `achievementPower` unlock (V pet level 25) raises the whole
        // multiplier `^1.5`; the exponent is 1 until then.
        Decimal::from_float(power.powf(self.ra_achievement_power_exponent()))
    }

    /// Antimatter to reset to after a dimension boost, galaxy, or Big Crunch.
    /// Mirrors `Currency.antimatter.startingValue = Effects.max(10, Perk.startAM,
    /// Achievement(21) = 100, Achievement(37) = 5000, Achievement(54) = 5e5,
    /// Achievement(55) = 5e10, Achievement(78) = 5e25)`.
    pub fn starting_antimatter(&self) -> Decimal {
        // The SAM perk (`Perk.startAM`): start every reset with 5e130 — larger
        // than every achievement term, so it dominates the `Effects.max`.
        if self.perk_bought(10) {
            return Decimal::new(5.0, 130);
        }
        let mut value = Decimal::from_float(INITIAL_ANTIMATTER);
        if self.achievement_unlocked(21) {
            value = value.max(&Decimal::from_float(100.0));
        }
        if self.achievement_unlocked(37) {
            value = value.max(&Decimal::from_float(5000.0));
        }
        if self.achievement_unlocked(54) {
            value = value.max(&Decimal::new(5.0, 5));
        }
        if self.achievement_unlocked(55) {
            value = value.max(&Decimal::new(5.0, 10));
        }
        if self.achievement_unlocked(78) {
            value = value.max(&Decimal::new(5.0, 25));
        }
        value
    }
}

/// Achievement unlock conditions ported from the original's `checkEvent` /
/// `checkRequirement` pairs. The original registers each on an event bus; we
/// have no bus, so we call the relevant `check_*_achievements` at the equivalent
/// action seam (once per tick, at a crunch/galaxy/eternity/reality reset, etc.),
/// matching *when* the original's event fires. Each condition is guarded by a
/// plain `if` and calls the idempotent [`unlock_achievement`](GameState::unlock_achievement).
impl GameState {
    /// The all-tier achievement multiplier applied to every Antimatter
    /// Dimension — the achievement terms of the original's
    /// `antimatterDimensionCommonMultiplier`. The global achievement *power*
    /// (`achievement_power`) and the per-tier terms (28/31/23/34/43/64/68/71)
    /// are applied separately in [`dimension_multiplier`](GameState::dimension_multiplier).
    pub(crate) fn achievement_ad_common_mult(&self) -> Decimal {
        let mut mult = Decimal::ONE;
        // 48: complete all Normal Challenges — all Dimensions ×1.1.
        if self.achievement_unlocked(48) {
            mult *= Decimal::from_float(1.1);
        }
        // 56: boost in the first 3 minutes of an Infinity (max(6/(min+3), 1)).
        if self.achievement_unlocked(56) {
            let minutes = self.records.this_infinity.time_ms / 60_000.0;
            if minutes < 3.0 {
                mult *= Decimal::from_float((6.0 / (minutes + 3.0)).max(1.0));
            }
        }
        // 65: like 56 but only inside a challenge (max(4/(min+1), 1)).
        if self.achievement_unlocked(65) && self.is_in_any_challenge() {
            let minutes = self.records.this_infinity.time_ms / 60_000.0;
            if minutes < 3.0 {
                mult *= Decimal::from_float((4.0 / (minutes + 1.0)).max(1.0));
            }
        }
        // 72: every AD multiplier over Number.MAX_VALUE — all ×1.1.
        if self.achievement_unlocked(72) {
            mult *= Decimal::from_float(1.1);
        }
        // 73: multiplier based on current antimatter (`AM^0.00002 + 1`).
        if self.achievement_unlocked(73) {
            mult *= self.antimatter.pow(&Decimal::from_float(0.00002)) + Decimal::ONE;
        }
        // 74: all ADs ×1.4, but only inside a challenge.
        if self.achievement_unlocked(74) && self.is_in_any_challenge() {
            mult *= Decimal::from_float(1.4);
        }
        // 76: tiny multiplier based on time played (`max((days/2)^0.05, 1)`).
        if self.achievement_unlocked(76) {
            let days = self.records.total_time_played_ms / 86_400_000.0;
            mult *= Decimal::from_float((days / 2.0).powf(0.05).max(1.0));
        }
        // 84: multiplier based on current (unspent) antimatter (`AM^0.00002 + 1`).
        if self.achievement_unlocked(84) {
            mult *= self.antimatter.pow(&Decimal::from_float(0.00002)) + Decimal::ONE;
        }
        // 91: strong boost in the first 5 seconds of an Infinity
        // (`max((5 - sec) × 60, 1)`).
        if self.achievement_unlocked(91) {
            let seconds = self.records.this_infinity.time_ms / 1000.0;
            if seconds < 5.0 {
                mult *= Decimal::from_float(((5.0 - seconds) * 60.0).max(1.0));
            }
        }
        // 92: strong boost in the first 60 seconds of an Infinity
        // (`max((1 - min) × 100, 1)`).
        if self.achievement_unlocked(92) {
            let minutes = self.records.this_infinity.time_ms / 60_000.0;
            if minutes < 1.0 {
                mult *= Decimal::from_float(((1.0 - minutes) * 100.0).max(1.0));
            }
        }
        mult
    }

    /// `Player.isInAnyChallenge`: any Antimatter Challenge or Eternity Challenge
    /// is running.
    fn is_in_any_challenge(&self) -> bool {
        self.in_any_antimatter_challenge() || self.any_ec_running()
    }

    /// Whether every AD autobuyer's "Buys max" bulk is at the cap (achievement
    /// 61's `hasMaxedBulk` over the zero-indexed AD autobuyers).
    fn all_ad_autobuyers_bulk_maxed(&self) -> bool {
        self.autobuyers
            .dimensions
            .iter()
            .all(|ab| ab.bulk >= crate::autobuyers::AD_AUTOBUYER_BULK_CAP)
    }

    /// Whether `id` is the sole running normal challenge — the original's
    /// `NormalChallenge(id).isOnlyActiveChallenge` (`player.challenge.normal
    /// .current === id`), which — unlike [`challenge_running`](GameState::challenge_running)
    /// — is **not** satisfied by Infinity Challenge 1 running the challenge.
    fn is_only_active_normal_challenge(&self, id: u8) -> bool {
        self.challenge.current == id
    }

    /// Number of completed Normal Challenges (`NormalChallenges.all.countWhere`).
    fn completed_normal_challenge_count(&self) -> u32 {
        (1..=crate::NORMAL_CHALLENGE_COUNT)
            .filter(|&id| self.challenge_completed(id))
            .count() as u32
    }

    /// GAME_TICK_AFTER conditions, checked once per tick (from [`tick`](GameState::tick)).
    /// `dt_ms` is the game-time step, used by the marathon timers (the original's
    /// `AchievementTimers`, which advance on `Time.deltaTime`).
    pub(crate) fn check_tick_achievements(&mut self, dt_ms: f64) {
        // 24: reach 1e80 antimatter.
        if self.antimatter.exponent() >= 80 {
            self.unlock_achievement(24);
        }
        // 31: any Antimatter Dimension multiplier over 1e31.
        if !self.achievement_unlocked(31)
            && (0..8).any(|t| self.dimension_multiplier(t).exponent() >= 31)
        {
            self.unlock_achievement(31);
        }
        // 42: antimatter/s exceeds current antimatter, above 1e63.
        if self.antimatter.exponent() >= 63
            && self.antimatter_per_second() > self.antimatter
        {
            self.unlock_achievement(42);
        }
        // 43: the 8 AD multipliers are strictly increasing by tier.
        if !self.achievement_unlocked(43) {
            let mults: Vec<Decimal> =
                (0..8).map(|t| self.dimension_multiplier(t)).collect();
            if mults.windows(2).all(|w| w[0] < w[1]) {
                self.unlock_achievement(43);
            }
        }
        // 44: antimatter/s exceeds antimatter for 30 consecutive (game) seconds
        // (`AchievementTimers.marathon1`).
        if !self.achievement_unlocked(44) {
            if self.antimatter_per_second() > self.antimatter {
                self.ach_marathon1_ms += dt_ms;
                if self.ach_marathon1_ms >= 30_000.0 {
                    self.unlock_achievement(44);
                }
            } else {
                self.ach_marathon1_ms = 0.0;
            }
        }
        // 45: over 1e29 ticks/second (tickspeed interval ≤ 1e-26 ms).
        if self.current_tickspeed_ms().exponent() <= -26 {
            self.unlock_achievement(45);
        }
        // 46: reach 1e12 of the 7th Antimatter Dimension.
        if self.dimensions[6].amount.exponent() >= 12 {
            self.unlock_achievement(46);
        }
        // 61: all AD autobuyers have maxed bulk. The reward (unlimited bulk)
        // affects production; the original fires this on Reality / save-convert,
        // but the engine has no bulk-upgrade action, so a guarded per-tick check
        // catches a loaded save that already has maxed bulk.
        if !self.achievement_unlocked(61) && self.all_ad_autobuyers_bulk_maxed() {
            self.unlock_achievement(61);
        }
        // 63: begin generating Infinity Power.
        if self.infinity_power > Decimal::ONE {
            self.unlock_achievement(63);
        }
        // 66: over 1e58 ticks/second (tickspeed interval ≤ 1e-55 ms).
        if self.current_tickspeed_ms().exponent() <= -55 {
            self.unlock_achievement(66);
        }
        // 72: every AD multiplier at least Number.MAX_VALUE.
        if !self.achievement_unlocked(72)
            && (0..8).all(|t| self.dimension_multiplier(t) >= Decimal::NUMBER_MAX_VALUE)
        {
            self.unlock_achievement(72);
        }
        // 73: reach 9.9999e9999 antimatter.
        if self.antimatter >= Decimal::new(9.9999, 9999) {
            self.unlock_achievement(73);
        }
        // 75: unlock the 4th Infinity Dimension.
        if self.infinity_dimensions[3].is_unlocked {
            self.unlock_achievement(75);
        }
        // 76: play for 8 days (game time).
        if self.records.total_time_played_ms >= 8.0 * 86_400_000.0 {
            self.unlock_achievement(76);
        }
        // 77: reach 1e6 Infinity Power.
        if self.infinity_power.exponent() >= 6 {
            self.unlock_achievement(77);
        }
        // 84: reach 1e35000 antimatter.
        if self.antimatter.exponent() >= 35_000 {
            self.unlock_achievement(84);
        }
        // 86: tickspeed 1000× faster per upgrade (`multiplier.recip() ≥ 1000`).
        if !self.achievement_unlocked(86)
            && 1.0 / self.tickspeed_purchase_multiplier() >= 1000.0
        {
            self.unlock_achievement(86);
        }
        // 87: Infinity 2e6 times.
        if self.infinities > Decimal::new(2.0, 6) {
            self.unlock_achievement(87);
        }
        // 94: reach 1e260 Infinity Power.
        if self.infinity_power.exponent() >= 260 {
            self.unlock_achievement(94);
        }
        // 98: unlock the 8th Infinity Dimension.
        if self.infinity_dimensions[7].is_unlocked {
            self.unlock_achievement(98);
        }
        // 102: reach every Eternity milestone.
        if !self.achievement_unlocked(102)
            && crate::eternity_milestones::ETERNITY_MILESTONES
                .iter()
                .all(|m| self.eternity_milestone_reached(m.eternities))
        {
            self.unlock_achievement(102);
        }
        // 103: reach 1e1000 Infinity Points.
        if self.infinity_points.exponent() >= 1000 {
            self.unlock_achievement(103);
        }
        // 105: 308 Tickspeed upgrades from Time Dimensions (free tick gained).
        if self.total_tick_gained >= 308 {
            self.unlock_achievement(105);
        }
        // 121: reach 1e30008 Infinity Points.
        if self.infinity_points.exponent() >= 30008 {
            self.unlock_achievement(121);
        }
        // 124: Infinity Power/s exceeds Infinity Power for 60 consecutive (game)
        // seconds, outside EC7 (`AchievementTimers.marathon2`).
        if !self.achievement_unlocked(124) {
            if !self.ec_running(7)
                && self.id_production_per_second(0) > self.infinity_power
            {
                self.ach_marathon2_ms += dt_ms;
                if self.ach_marathon2_ms >= 60_000.0 {
                    self.unlock_achievement(124);
                }
            } else {
                self.ach_marathon2_ms = 0.0;
            }
        }
        // 125: 1e90 IP with no Infinities or 1st ADs this Eternity.
        if self.infinity_points.exponent() >= 90
            && self.requirement_checks.eternity_no_ad1
            && self.infinities == Decimal::ZERO
        {
            self.unlock_achievement(125);
        }
        // 126: 180× more Replicanti Galaxies than Antimatter Galaxies.
        if self.galaxies > 0
            && u64::from(self.replicanti.galaxies + self.extra_replicanti_galaxies())
                >= 180 * self.galaxies as u64
        {
            self.unlock_achievement(126);
        }
        // 127: reach Number.MAX_VALUE Eternity Points.
        if self.eternity_points >= Decimal::NUMBER_MAX_VALUE {
            self.unlock_achievement(127);
        }
        // 128: reach 1e22000 Infinity Points without any Time Studies.
        if self.infinity_points.exponent() >= 22000 && self.studies.is_empty() {
            self.unlock_achievement(128);
        }
        // 133: reach 1e200000 IP without buying any Infinity Dimensions or the
        // ×2 IP multiplier (the latter is unmodelled, hence always satisfied).
        if self.infinity_points.exponent() >= 200_000
            && self.infinity_dimensions.iter().all(|d| d.purchases() == 0)
        {
            self.unlock_achievement(133);
        }
        // 134: reach 1e18000 Replicanti.
        if self.replicanti.amount.exponent() >= 18000 {
            self.unlock_achievement(134);
        }
        // 135: over 1e8296262 ticks/second.
        if self.current_tickspeed_ms().exponent() <= -8_296_262 {
            self.unlock_achievement(135);
        }
        // 137: 1e260000 antimatter within 1 minute (game time) while Dilated.
        if self.dilation.active
            && self.antimatter.exponent() >= 260_000
            && self.records.this_eternity.time_ms <= 60_000.0
        {
            self.unlock_achievement(137);
        }
        // 138: 1e26000 IP with no Time Studies while Dilated.
        if self.dilation.active
            && self.studies.is_empty()
            && self.infinity_points.exponent() >= 26000
        {
            self.unlock_achievement(138);
        }
        // 142: unlock the Automator (the original fires on Reality/perk/black-hole
        // events; a guarded per-tick check covers all of them).
        if !self.achievement_unlocked(142) && self.automator_unlocked() {
            self.unlock_achievement(142);
        }
        // 152: have 100 Glyphs in the inventory (a GLYPHS_CHANGED achievement,
        // checked per tick here).
        if self.reality.glyphs.inventory.len() >= 100 {
            self.unlock_achievement(152);
        }
        // 155: play for 13.7 billion years (game time).
        if self.records.total_time_played_ms > 13.7e9 * 365.25 * 86_400_000.0 {
            self.unlock_achievement(155);
        }
        // 157: have a Glyph with 4 effects (a GLYPHS_CHANGED achievement).
        if !self.achievement_unlocked(157)
            && self
                .reality
                .glyphs
                .active
                .iter()
                .chain(self.reality.glyphs.inventory.iter())
                .any(|g| g.effects.count_ones() >= 4)
        {
            self.unlock_achievement(157);
        }
        // 161: 1e100000000 antimatter while Dilated.
        if self.dilation.active && self.antimatter.exponent() >= 100_000_000 {
            self.unlock_achievement(161);
        }
        // 162: have every Time Study at once (58 of them).
        if self.studies.len() >= 58 {
            self.unlock_achievement(162);
        }
        // 163: all Eternity Challenges completed 5× within 1 second this Reality.
        if self.records.this_reality.time_ms <= 1000.0
            && (1..=crate::eternity_challenges::ETERNITY_CHALLENGE_COUNT as u8)
                .map(|id| self.eternity_challenge_completions(id))
                .min()
                .unwrap_or(0)
                >= 5
        {
            self.unlock_achievement(163);
        }
        // 164: reach Number.MAX_VALUE total Infinities.
        if self.infinities_total() >= Decimal::NUMBER_MAX_VALUE {
            self.unlock_achievement(164);
        }
        // 167: reach Number.MAX_VALUE Reality Machines.
        if self.reality.machines >= Decimal::NUMBER_MAX_VALUE {
            self.unlock_achievement(167);
        }
        // 168: 50 total Ra Celestial Memory levels.
        if (0..crate::celestials::ra::PET_COUNT)
            .map(|p| self.ra_pet_level(p))
            .sum::<u32>()
            >= 50
        {
            self.unlock_achievement(168);
        }
        // 171: sacrifice every (basic) Glyph type at least once.
        if self.reality.glyphs.sac.iter().all(|&s| s > 0.0) {
            self.unlock_achievement(171);
        }
        // 173: reach 9.99999e999 Reality Machines.
        if self.reality.machines >= Decimal::new(9.99999, 999) {
            self.unlock_achievement(173);
        }
        // 52: max the interval for the AD and Tickspeed autobuyers. The original
        // fires this on REALITY_RESET_AFTER / REALITY_UPGRADE_TEN_BOUGHT, but a
        // Reality clears autobuyer intervals, so it is only truly reachable while
        // a run is in progress. Checked per tick here (it carries no production
        // effect, so the unlock timing has no numeric consequence).
        if !self.achievement_unlocked(52) && self.ad_and_tickspeed_autobuyers_maxed() {
            self.unlock_achievement(52);
        }
        // 53: max the intervals for all upgradeable normal autobuyers.
        if !self.achievement_unlocked(53) && self.all_upgradeable_autobuyers_maxed() {
            self.unlock_achievement(53);
        }
    }

    /// BIG_CRUNCH_BEFORE conditions, checked at a rewarded crunch before the
    /// reset (so pre-reset galaxies / dimensions / this-infinity timing apply).
    pub(crate) fn check_crunch_before_achievements(&mut self) {
        // 21: "To infinity!" — go Infinite.
        self.unlock_achievement(21);
        // 34: Infinity without any 8th Antimatter Dimensions.
        if self.dimensions[7].amount <= Decimal::ZERO {
            self.unlock_achievement(34);
        }
        // 36: Infinity with exactly 1 Antimatter Galaxy.
        if self.galaxies == 1 {
            self.unlock_achievement(36);
        }
        // 37: Infinity in under 2 hours (real time).
        if self.records.this_infinity.real_time_ms <= 2.0 * 3_600_000.0 {
            self.unlock_achievement(37);
        }
        // 54: Infinity in 10 minutes or less (real time).
        if self.records.this_infinity.real_time_ms <= 10.0 * 60_000.0 {
            self.unlock_achievement(54);
        }
        // 55: Infinity in 1 minute or less (real time).
        if self.records.this_infinity.real_time_ms <= 60_000.0 {
            self.unlock_achievement(55);
        }
        // 78: Infinity in under 250 ms (real time).
        if self.records.this_infinity.real_time_ms <= 250.0 {
            self.unlock_achievement(78);
        }
        let secs = self.records.this_infinity.real_time_ms / 1000.0;
        // 56: complete the NC2 (AD Autobuyer) challenge in ≤ 3 minutes.
        if self.is_only_active_normal_challenge(2) && secs <= 180.0 {
            self.unlock_achievement(56);
        }
        // 57: complete the NC8 (AD8 Autobuyer) challenge in ≤ 3 minutes.
        if self.is_only_active_normal_challenge(8) && secs <= 180.0 {
            self.unlock_achievement(57);
        }
        // 58: complete the NC9 (Tickspeed Autobuyer) challenge in ≤ 3 minutes.
        if self.is_only_active_normal_challenge(9) && secs <= 180.0 {
            self.unlock_achievement(58);
        }
        // 68: complete the NC3 challenge in ≤ 10 seconds.
        if self.is_only_active_normal_challenge(3) && secs <= 10.0 {
            self.unlock_achievement(68);
        }
        // 64: Infinity in a Normal Challenge with no Boosts or Galaxies.
        if self.galaxies == 0 && self.dim_boosts == 0 && self.any_challenge_running() {
            self.unlock_achievement(64);
        }
        // 71: Infinity with a single 1st AD, no Boosts/Galaxies, in NC2.
        if self.is_only_active_normal_challenge(2)
            && self.dimensions[0].amount == Decimal::ONE
            && self.dim_boosts == 0
            && self.galaxies == 0
        {
            self.unlock_achievement(71);
        }
        // 81: beat Infinity Challenge 5 in ≤ 15 seconds.
        if self.infinity_challenge_running(5) && secs <= 15.0 {
            self.unlock_achievement(81);
        }
        // The pending crunch's IP exponent, for the "Big Crunch for Xe IP"
        // achievements (85/91/92/93).
        let gained_ip_exp = self.gained_infinity_points().exponent();
        // 85: Big Crunch for 1e150 IP.
        if gained_ip_exp >= 150 {
            self.unlock_achievement(85);
        }
        // 91: Big Crunch for 1e200 IP in ≤ 2 seconds.
        if gained_ip_exp >= 200 && secs <= 2.0 {
            self.unlock_achievement(91);
        }
        // 92: Big Crunch for 1e250 IP in ≤ 20 seconds.
        if gained_ip_exp >= 250 && secs <= 20.0 {
            self.unlock_achievement(92);
        }
        // 93: Big Crunch for 1e300 IP.
        if gained_ip_exp >= 300 {
            self.unlock_achievement(93);
        }
    }

    /// BIG_CRUNCH_AFTER conditions, checked at the end of a rewarded crunch.
    pub(crate) fn check_crunch_after_achievements(&mut self) {
        // 33: reach Infinity 10 times.
        if self.infinities >= Decimal::from_float(10.0) {
            self.unlock_achievement(33);
        }
        // 97: sum of Infinity Challenge best times under 6.66 seconds. Uncompleted
        // ICs hold the `f64::MAX` sentinel, so the sum stays huge until all eight
        // are completed quickly.
        let ic_sum: f64 = self.ic_best_times_ms.iter().sum();
        if ic_sum < 6_660.0 {
            self.unlock_achievement(97);
        }
        // 112: sum of Infinity Challenge best times below 750 ms.
        if ic_sum < 750.0 {
            self.unlock_achievement(112);
        }
        self.check_challenge_completion_achievements();
    }

    /// SACRIFICE_RESET_BEFORE conditions, checked before a performed sacrifice.
    pub(crate) fn check_sacrifice_before_achievements(&mut self) {
        // 88: a single Dimensional Sacrifice worth ≥ Number.MAX_VALUE.
        if self.next_sacrifice_boost() >= Decimal::NUMBER_MAX_VALUE {
            self.unlock_achievement(88);
        }
    }

    /// REPLICANTI_TICK_AFTER conditions, checked after Replicanti grow.
    pub(crate) fn check_replicanti_after_achievements(&mut self) {
        // 95: reach Number.MAX_VALUE Replicanti (or hold a Replicanti Galaxy)
        // within 1 hour (real time) of the Infinity. Reward (keep Replicanti +
        // 1 RG on Infinity) is read in `big_crunch_reset`.
        if !self.achievement_unlocked(95)
            && self.records.this_infinity.real_time_ms <= 3_600_000.0
            && (self.replicanti.amount >= Decimal::NUMBER_MAX_VALUE
                || self.replicanti.galaxies > 0)
        {
            self.unlock_achievement(95);
        }
        // 106: 10 Replicanti Galaxies within 15 seconds (game time) of the
        // Infinity.
        if self.replicanti.galaxies + self.extra_replicanti_galaxies() >= 10
            && self.records.this_infinity.time_ms <= 15_000.0
        {
            self.unlock_achievement(106);
        }
    }

    /// ETERNITY_RESET_BEFORE conditions, checked at a rewarded Eternity before
    /// the reset (pre-reset run flags / this-eternity timing apply).
    pub(crate) fn check_eternity_before_achievements(&mut self) {
        // 96: "Time is relative" — go Eternal.
        self.unlock_achievement(96);
        // 101: Eternity without buying Antimatter Dimensions 1–7.
        if self.requirement_checks.eternity_only_ad8 {
            self.unlock_achievement(101);
        }
        // 104: Eternity in under 30 seconds (game time).
        if self.records.this_eternity.time_ms <= 30_000.0 {
            self.unlock_achievement(104);
        }
        // 107: Eternity with fewer than 10 Infinities.
        if self.infinities < Decimal::from_float(10.0) {
            self.unlock_achievement(107);
        }
        // 108: Eternity with exactly 9 Replicanti.
        if self.replicanti.amount.round() == Decimal::from_float(9.0) {
            self.unlock_achievement(108);
        }
        // 113: Eternity in under 250 ms (game time). Reward (×2 Eternities) is
        // read in `gained_eternities`, computed after this check.
        if self.records.this_eternity.time_ms <= 250.0 {
            self.unlock_achievement(113);
        }
        // 116: Eternity with only 1 Infinity.
        if self.infinities <= Decimal::ONE {
            self.unlock_achievement(116);
        }
        // 122: Eternity without buying Antimatter Dimensions 2–8.
        if self.requirement_checks.eternity_only_ad1 {
            self.unlock_achievement(122);
        }
    }

    /// ETERNITY_RESET_AFTER conditions.
    pub(crate) fn check_eternity_after_achievements(&mut self) {
        // 123: complete 50 unique Eternity Challenge tiers.
        let ec_completions: u32 =
            self.eternity_challenges.iter().map(|&c| c as u32).sum();
        if ec_completions >= 50 {
            self.unlock_achievement(123);
        }
        // 131: 2e9 Banked Infinities.
        if self.infinities_banked > Decimal::new(2.0, 9) {
            self.unlock_achievement(131);
        }
        // 143: each of the last 10 Eternities gave ≥ Number.MAX_VALUE× the EP of
        // the previous one (a filled ring, newest first).
        let recent = &self.records.recent_eternities;
        let filled = recent.iter().all(|r| r.time_ms != f64::MAX);
        if filled
            && recent
                .windows(2)
                .all(|w| w[0].ep >= w[1].ep * Decimal::NUMBER_MAX_VALUE)
        {
            self.unlock_achievement(143);
        }
    }

    /// REALITY_RESET_BEFORE conditions (read the pre-reset run state).
    pub(crate) fn check_reality_before_achievements(&mut self) {
        // 141: make a new Reality.
        self.unlock_achievement(141);
        // 148: Reality with one of each basic Glyph type equipped.
        if crate::glyphs::BASIC_GLYPH_TYPES
            .iter()
            .all(|&t| self.reality.glyphs.active.iter().any(|g| g.kind == t))
        {
            self.unlock_achievement(148);
        }
        // 153: Reality without producing antimatter.
        if self.requirement_checks.reality_no_am {
            self.unlock_achievement(153);
        }
        // 154: Reality in under 5 seconds (game time).
        if self.records.this_reality.time_ms <= 5_000.0 {
            self.unlock_achievement(154);
        }
        // 166: a Glyph with level exactly 6969.
        if self.gained_glyph_level().actual_level == 6969 {
            self.unlock_achievement(166);
        }
        // 165 (level-5000 Glyph with equal Effarig weights) is deferred: the
        // per-factor glyph-level weights are not modelled.
    }

    /// REALITY_RESET_AFTER conditions.
    pub(crate) fn check_reality_after_achievements(&mut self) {
        // 175: all Alchemy Resources at the flat cap (25000).
        let cap = crate::celestials::alchemy::ALCHEMY_RESOURCE_CAP;
        if (0..crate::celestials::alchemy::ALCHEMY_COUNT)
            .all(|id| self.alchemy_amount(id) >= cap)
        {
            self.unlock_achievement(175);
        }
    }

    /// SINGULARITY_RESET_BEFORE conditions.
    pub(crate) fn check_singularity_before_achievements(&mut self) {
        // 174: get a Singularity.
        self.unlock_achievement(174);
    }

    /// SINGULARITY_RESET_AFTER conditions.
    pub(crate) fn check_singularity_after_achievements(&mut self) {
        // 177: complete every Singularity Milestone at least once.
        if (0..crate::celestials::singularity::MILESTONE_COUNT)
            .all(|id| self.singularity_milestone_completions(id) > 0)
        {
            self.unlock_achievement(177);
        }
    }

    /// The Dark-Matter-Dimension annihilation achievement (176), unlocked the
    /// first time an annihilation happens.
    pub(crate) fn check_annihilation_achievements(&mut self) {
        // 176: annihilate your Dark Matter Dimensions.
        self.unlock_achievement(176);
    }

    /// The static glyph-level adder from achievements 148 (number of distinct
    /// equipped basic Glyph types) and 166 (+69) — the original's
    /// `Effects.sum(Achievement(148), Achievement(166))` in `getGlyphLevelInputs`.
    pub(crate) fn achievement_glyph_level_bonus(&self) -> u32 {
        let mut bonus = 0u32;
        if self.achievement_unlocked(148) {
            let active = self.active_glyphs_without_companion();
            bonus += crate::glyphs::BASIC_GLYPH_TYPES
                .iter()
                .filter(|&&t| active.iter().any(|g| g.kind == t))
                .count() as u32;
        }
        if self.achievement_unlocked(166) {
            bonus += 69;
        }
        bonus
    }

    /// PERK_BOUGHT conditions.
    pub(crate) fn check_perk_bought_achievements(&mut self) {
        // 146: have all Perks bought.
        if self.reality.perks.len() >= crate::perks::PERKS.len() {
            self.unlock_achievement(146);
        }
    }

    /// REALITY_UPGRADE_BOUGHT conditions.
    pub(crate) fn check_reality_upgrade_bought_achievements(&mut self) {
        // 147: have all (one-time) Reality Upgrades bought.
        if (6..=25).all(|id| self.reality_upgrade_bought(id)) {
            self.unlock_achievement(147);
        }
    }

    /// BLACK_HOLE_UNLOCKED conditions.
    pub(crate) fn check_black_hole_unlocked_achievements(&mut self) {
        // 144: unlock the Black Hole.
        self.unlock_achievement(144);
    }

    /// BLACK_HOLE_UPGRADE_BOUGHT conditions.
    pub(crate) fn check_black_hole_upgrade_achievements(&mut self) {
        // 145: either Black Hole interval smaller than its duration.
        if (0..2).any(|i| self.black_hole_interval(i) < self.black_hole_duration(i)) {
            self.unlock_achievement(145);
        }
        // 158: make both Black Holes permanent.
        if (0..2).all(|i| self.black_hole_is_permanent(i)) {
            self.unlock_achievement(158);
        }
    }

    /// CHALLENGE_FAILED conditions (an Eternity Challenge's restriction breaks).
    pub(crate) fn check_challenge_failed_achievements(&mut self) {
        // 114: fail an Eternity Challenge.
        self.unlock_achievement(114);
    }

    /// GALAXY_RESET_BEFORE conditions (checked before the galaxy's reset).
    pub(crate) fn check_galaxy_before_achievements(&mut self) {
        // 26: buy an Antimatter Galaxy.
        self.unlock_achievement(26);
        // 38: buy an Antimatter Galaxy without Dimensional Sacrificing (since the
        // last Galaxy).
        if self.requirement_checks.infinity_no_sacrifice {
            self.unlock_achievement(38);
        }
    }

    /// GALAXY_RESET_AFTER conditions (checked after the galaxy count increments).
    pub(crate) fn check_galaxy_after_achievements(&mut self) {
        // 27: have 2 Antimatter Galaxies.
        if self.galaxies >= 2 {
            self.unlock_achievement(27);
        }
        // 83: have 50 Antimatter Galaxies.
        if self.galaxies >= 50 {
            self.unlock_achievement(83);
        }
        // 132: 569 Antimatter Galaxies without gaining a Replicanti Galaxy this
        // Eternity.
        if self.galaxies >= 569 && self.requirement_checks.eternity_no_rg {
            self.unlock_achievement(132);
        }
        // 151: 800 Antimatter Galaxies without buying an 8th AD this Infinity.
        if self.galaxies >= 800 && self.requirement_checks.infinity_no_ad8 {
            self.unlock_achievement(151);
        }
        // 178: 100000 Antimatter Galaxies.
        if self.galaxies >= 100_000 {
            self.unlock_achievement(178);
        }
    }

    /// SACRIFICE_RESET_AFTER conditions (checked after a performed sacrifice).
    pub(crate) fn check_sacrifice_after_achievements(&mut self) {
        // 32: total Dimensional Sacrifice multiplier over 600, outside Challenge 8.
        if !self.is_only_active_normal_challenge(8)
            && self.sacrifice_multiplier() >= Decimal::from_float(600.0)
        {
            self.unlock_achievement(32);
        }
    }

    /// INFINITY_CHALLENGE_COMPLETED conditions (checked when an IC is completed).
    pub(crate) fn check_infinity_challenge_completed_achievements(&mut self) {
        // 67: complete an Infinity Challenge.
        if (1..=crate::INFINITY_CHALLENGE_COUNT)
            .any(|id| self.infinity_challenge_completed(id))
        {
            self.unlock_achievement(67);
        }
        // 82: complete all 8 Infinity Challenges.
        if (1..=crate::INFINITY_CHALLENGE_COUNT)
            .all(|id| self.infinity_challenge_completed(id))
        {
            self.unlock_achievement(82);
        }
    }

    /// 47/48 check (Normal Challenge completion counts). The original also
    /// registers these on REALITY_RESET_AFTER, but a Reality clears challenge
    /// completions, so the meaningful seam is the crunch that banks the
    /// completion.
    fn check_challenge_completion_achievements(&mut self) {
        let completed = self.completed_normal_challenge_count();
        // 47: complete 3 Normal Challenges.
        if completed >= 3 {
            self.unlock_achievement(47);
        }
        // 48: complete all 12 Normal Challenges.
        if completed >= crate::NORMAL_CHALLENGE_COUNT as u32 {
            self.unlock_achievement(48);
        }
    }

    /// Whether every AD autobuyer and the Tickspeed autobuyer are unlocked and at
    /// their minimum interval (achievement 52).
    fn ad_and_tickspeed_autobuyers_maxed(&self) -> bool {
        use crate::AutobuyerTarget;
        let mut targets: Vec<AutobuyerTarget> =
            (0..8).map(AutobuyerTarget::AdTier).collect();
        targets.push(AutobuyerTarget::Tickspeed);
        targets.iter().all(|&t| {
            self.autobuyer_is_unlocked(t) && self.autobuyer_has_maxed_interval(t)
        })
    }

    /// Whether every upgradeable normal autobuyer (dimensions, tickspeed,
    /// dimension boost, galaxy, big crunch) is unlocked and interval-maxed
    /// (achievement 53).
    fn all_upgradeable_autobuyers_maxed(&self) -> bool {
        use crate::AutobuyerTarget;
        let mut targets: Vec<AutobuyerTarget> =
            (0..8).map(AutobuyerTarget::AdTier).collect();
        targets.extend([
            AutobuyerTarget::Tickspeed,
            AutobuyerTarget::DimBoost,
            AutobuyerTarget::Galaxy,
            AutobuyerTarget::BigCrunch,
        ]);
        targets.iter().all(|&t| {
            self.autobuyer_is_unlocked(t) && self.autobuyer_has_maxed_interval(t)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_maps_to_row_and_column_bit() {
        // 18 → row 1, column 8 → bits[0] bit 1<<7 (the original's achievement 18).
        let mut game = GameState::new();
        assert!(!game.achievement_unlocked(18));
        game.unlock_achievement(18);
        assert_eq!(game.achievement_bits[0], 1 << 7);
        assert!(game.achievement_unlocked(18));

        // 21 → row 2, column 1 → bits[1] bit 1<<0.
        game.unlock_achievement(21);
        assert_eq!(game.achievement_bits[1], 1 << 0);
        assert!(game.achievement_unlocked(21));
    }

    #[test]
    fn unlock_is_idempotent() {
        let mut game = GameState::new();
        game.unlock_achievement(11);
        game.unlock_achievement(11);
        assert_eq!(game.achievement_bits[0], 1);
    }

    #[test]
    fn unlocked_ids_are_sorted() {
        let mut game = GameState::new();
        game.unlock_achievement(28);
        game.unlock_achievement(11);
        game.unlock_achievement(21);
        assert_eq!(game.unlocked_achievement_ids(), vec![11, 21, 28]);
    }

    #[test]
    fn power_counts_unlocks_and_completed_rows() {
        let mut game = GameState::new();
        assert_eq!(game.achievement_power(), Decimal::from_float(1.0));

        // Two unlocks, no complete row: 1.03^2.
        game.unlock_achievement(11);
        game.unlock_achievement(12);
        let expected = Decimal::from_float(1.03f64.powi(2));
        assert_eq!(game.achievement_power(), expected);

        // Complete row 1 (ids 11..=18): 1.25^1 × 1.03^8.
        for id in 11..=18 {
            game.unlock_achievement(id);
        }
        let expected = Decimal::from_float(1.25f64.powi(1) * 1.03f64.powi(8));
        assert_eq!(game.achievement_power(), expected);
    }

    #[test]
    fn starting_antimatter_tracks_achievement_21() {
        let mut game = GameState::new();
        assert_eq!(
            game.starting_antimatter(),
            Decimal::from_float(INITIAL_ANTIMATTER)
        );
        game.unlock_achievement(21);
        assert_eq!(game.starting_antimatter(), Decimal::from_float(100.0));
    }

    #[test]
    fn buying_each_dimension_unlocks_its_achievement() {
        let mut game = GameState::new();
        game.dim_boosts = 4; // unlock all 8 tiers
        game.antimatter = Decimal::new(1.0, 120); // enough to buy up the chain
        for tier in 0..8 {
            assert!(!game.achievement_unlocked(11 + tier as u16));
            assert!(game.buy_dimension(tier));
            assert!(game.achievement_unlocked(11 + tier as u16));
        }
    }

    #[test]
    fn achievement_28_unlocks_buying_ad1_over_1e150() {
        let mut game = GameState::new();
        game.antimatter = Decimal::new(1.0, 200);
        game.dimensions[0].amount = Decimal::new(1.0, 150); // exactly 1e150
        assert!(game.buy_dimension(0));
        assert!(game.achievement_unlocked(28));
    }

    #[test]
    fn achievement_24_unlocks_at_1e80_antimatter() {
        let mut game = GameState::new();
        // Strong AD1 so a tick pushes antimatter past 1e80.
        game.dimensions[0].amount = Decimal::new(1.0, 85);
        assert!(!game.achievement_unlocked(24));
        game.tick(1000.0);
        assert!(game.achievement_unlocked(24));
    }

    #[test]
    fn achievement_18_persists_through_crunch() {
        let mut game = GameState::new();
        game.dim_boosts = 4; // unlock the 8th dimension
        game.antimatter = Decimal::new(1.0, 70);
        // Buying an 8th dimension requires owning a 7th (the purchasability
        // chain); seed it directly so we exercise just the achievement unlock.
        game.dimensions[6].amount = Decimal::ONE;
        assert!(game.buy_dimension(7)); // buy an 8th dimension → achievement 18
        assert!(game.achievement_unlocked(18));

        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        // Bits survive the reset (unlike dim_boosts/galaxies).
        assert!(game.achievement_unlocked(18));
    }

    /// Crunch to the goal with a 3-hour infinity: only 21 fires among the
    /// starting-antimatter achievements, so it stays 100.
    fn crunch_slow(game: &mut GameState) {
        game.records.this_infinity.real_time_ms = 3.0 * 3_600_000.0;
        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
    }

    #[test]
    fn achievement_33_unlocks_at_ten_infinities() {
        let mut game = GameState::new();
        game.infinities = Decimal::from_float(9.0); // +1 on this crunch → 10
        crunch_slow(&mut game);
        assert!(game.achievement_unlocked(33));
    }

    #[test]
    fn achievement_34_needs_no_eighth_dimension_at_crunch() {
        let mut game = GameState::new();
        game.dimensions[7].amount = Decimal::ONE;
        crunch_slow(&mut game);
        assert!(!game.achievement_unlocked(34));

        let mut game = GameState::new();
        // No 8th dimension held → "you didn't need it anyway".
        crunch_slow(&mut game);
        assert!(game.achievement_unlocked(34));
    }

    #[test]
    fn achievement_36_needs_exactly_one_galaxy_at_crunch() {
        let mut game = GameState::new();
        game.galaxies = 1;
        crunch_slow(&mut game);
        assert!(game.achievement_unlocked(36));
    }

    #[test]
    fn fast_crunch_unlocks_speed_achievements_and_raises_starting_antimatter() {
        let mut game = GameState::new();
        // Zero real time trips every "fast Infinity" achievement: 37 (≤2 h),
        // 54 (≤10 min), 55 (≤1 min), 78 (≤250 ms).
        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(game.achievement_unlocked(37));
        assert!(game.achievement_unlocked(54));
        assert!(game.achievement_unlocked(55));
        assert!(game.achievement_unlocked(78));
        // startingValue = max(100, 5000, 5e5, 5e10, 5e25) = 5e25.
        assert_eq!(game.antimatter, Decimal::new(5.0, 25));
    }

    #[test]
    fn achievement_38_needs_galaxy_without_sacrifice() {
        let mut game = GameState::new();
        game.requirement_checks.infinity_no_sacrifice = true;
        game.check_galaxy_before_achievements();
        assert!(game.achievement_unlocked(38));

        let mut game = GameState::new();
        game.requirement_checks.infinity_no_sacrifice = false;
        game.check_galaxy_before_achievements();
        assert!(!game.achievement_unlocked(38));
    }

    #[test]
    fn achievement_46_unlocks_at_1e12_seventh_dimension() {
        let mut game = GameState::new();
        game.dimensions[6].amount = Decimal::new(1.0, 12);
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(46));
    }

    #[test]
    fn achievement_44_marathon_needs_thirty_seconds() {
        let mut game = GameState::new();
        // AD1 stock huge, antimatter tiny → antimatter/s exceeds antimatter.
        game.antimatter = Decimal::ONE;
        game.dimensions[0].amount = Decimal::new(1.0, 10);
        game.dimensions[0].bought = 1;
        game.check_tick_achievements(15_000.0);
        assert!(!game.achievement_unlocked(44));
        game.check_tick_achievements(15_000.0); // 30 s total
        assert!(game.achievement_unlocked(44));
    }

    #[test]
    fn achievement_44_marathon_resets_when_condition_breaks() {
        let mut game = GameState::new();
        game.antimatter = Decimal::ONE;
        game.dimensions[0].amount = Decimal::new(1.0, 10);
        game.dimensions[0].bought = 1;
        game.check_tick_achievements(20_000.0);
        // Condition breaks: antimatter now dwarfs production.
        game.antimatter = Decimal::new(1.0, 40);
        game.check_tick_achievements(20_000.0);
        assert_eq!(game.ach_marathon1_ms, 0.0);
        assert!(!game.achievement_unlocked(44));
    }

    #[test]
    fn achievement_48_all_challenges_unlocks_and_boosts_dimensions() {
        let mut game = GameState::new();
        for id in 1..=crate::NORMAL_CHALLENGE_COUNT {
            game.complete_challenge(id);
        }
        game.check_crunch_after_achievements();
        assert!(game.achievement_unlocked(47));
        assert!(game.achievement_unlocked(48));
        // 48 contributes ×1.1 to the all-tier common multiplier.
        assert_eq!(game.achievement_ad_common_mult(), Decimal::from_float(1.1));
    }

    #[test]
    fn achievement_32_strengthens_sacrifice_and_unlocks() {
        let mut game = GameState::new();
        // A large sacrificed total so totalBoost = (log10/10)^2 = 30^2 = 900 ≥ 600.
        game.sacrificed = Decimal::new(1.0, 300);
        let before = game.sacrifice_multiplier();
        game.check_sacrifice_after_achievements();
        assert!(game.achievement_unlocked(32));
        // Exponent rose from 2 to 2.2, so the boost strengthens.
        let after = game.sacrifice_multiplier();
        assert!(after > before);
    }

    #[test]
    fn achievement_51_unlocks_on_break_infinity() {
        use crate::AutobuyerTarget;
        let mut game = GameState::new();
        game.complete_challenge(12);
        game.infinity_points = Decimal::from_float(1e9);
        for _ in 0..50 {
            game.upgrade_autobuyer_interval(AutobuyerTarget::BigCrunch);
        }
        assert!(game.break_infinity());
        assert!(game.achievement_unlocked(51));
    }

    #[test]
    fn achievement_31_boosts_first_dimension() {
        // Isolate 31's ×1.05 from the global achievement-power bump each unlock
        // gives: compare against a game with an equal-count, no-effect unlock (11).
        let mut baseline = GameState::new();
        baseline.unlock_achievement(11); // buys-a-1st-dim: no multiplier effect
        let mut game = GameState::new();
        game.unlock_achievement(31);
        assert_eq!(
            game.dimension_multiplier(0),
            baseline.dimension_multiplier(0) * Decimal::from_float(1.05)
        );
    }

    // ---- Batch 2 (ids 55–78) ----

    /// Compare a per-tier effect against a same-unlock-count baseline (achievement
    /// 11 has no multiplier effect), cancelling the ×1.03 achievement-power bump.
    fn tier_mult_with_only(id: u16, tier: usize) -> (Decimal, Decimal) {
        let mut baseline = GameState::new();
        baseline.unlock_achievement(11);
        let mut game = GameState::new();
        game.unlock_achievement(id);
        (
            game.dimension_multiplier(tier),
            baseline.dimension_multiplier(tier),
        )
    }

    #[test]
    fn achievements_64_68_71_boost_dimensions() {
        // 68: AD1 ×1.5, 71: AD1 ×3.
        let (g, b) = tier_mult_with_only(68, 0);
        assert_eq!(g, b * Decimal::from_float(1.5));
        let (g, b) = tier_mult_with_only(71, 0);
        assert_eq!(g, b * Decimal::from_float(3.0));
        // 64: AD1–4 ×1.25, AD5 unaffected.
        let (g, b) = tier_mult_with_only(64, 3);
        assert_eq!(g, b * Decimal::from_float(1.25));
        let (g, b) = tier_mult_with_only(64, 4);
        assert_eq!(g, b);
    }

    #[test]
    fn achievement_58_boosts_buy_ten_multiplier() {
        let mut game = GameState::new();
        let before = game.buy_ten_multiplier();
        game.unlock_achievement(58);
        assert_eq!(
            game.buy_ten_multiplier(),
            before * Decimal::from_float(1.01)
        );
    }

    #[test]
    fn achievement_66_speeds_tickspeed() {
        let mut game = GameState::new();
        game.dimensions[1].bought = 5; // some tickspeed context
        let before = game.tickspeed_effect();
        game.unlock_achievement(66);
        assert!(game.tickspeed_effect() > before);
    }

    #[test]
    fn achievement_75_extends_achievement_bonus_to_infinity_dimensions() {
        let mut game = GameState::new();
        assert_eq!(game.id_common_multiplier(), Decimal::ONE);
        game.unlock_achievement(75);
        // Only 75 unlocked → achievement_power = 1.03^1, applied to ID mult.
        assert_eq!(game.id_common_multiplier(), Decimal::from_float(1.03));
    }

    #[test]
    fn achievement_72_boosts_all_dimensions() {
        let mut game = GameState::new();
        game.unlock_achievement(72);
        assert_eq!(game.achievement_ad_common_mult(), Decimal::from_float(1.1));
    }

    #[test]
    fn achievement_73_scales_with_antimatter() {
        let mut game = GameState::new();
        game.antimatter = Decimal::new(1.0, 5);
        game.unlock_achievement(73);
        // AM^0.00002 + 1 > 2 for any AM > 1.
        assert!(game.achievement_ad_common_mult() > Decimal::from_float(2.0));
    }

    #[test]
    fn achievements_63_and_77_track_infinity_power() {
        let mut game = GameState::new();
        game.infinity_power = Decimal::from_float(2.0);
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(63));
        assert!(!game.achievement_unlocked(77));

        game.infinity_power = Decimal::new(1.0, 6);
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(77));
    }

    #[test]
    fn achievement_67_unlocks_on_infinity_challenge_completion() {
        let mut game = GameState::new();
        game.complete_infinity_challenge(3);
        assert!(game.achievement_unlocked(67));
    }

    // ---- Batch 3 (ids 81–104) ----

    #[test]
    fn achievements_85_93_multiply_ip_gain() {
        let mut game = GameState::new();
        assert_eq!(game.total_ip_mult(), Decimal::ONE);
        game.unlock_achievement(85);
        assert_eq!(game.total_ip_mult(), Decimal::from_float(4.0));
        game.unlock_achievement(93);
        assert_eq!(game.total_ip_mult(), Decimal::from_float(16.0));
    }

    #[test]
    fn achievement_87_raises_infinities_gain_after_five_seconds() {
        let mut game = GameState::new();
        game.unlock_achievement(87);
        // Only counts once the Infinity is longer than 5 seconds.
        game.records.this_infinity.time_ms = 4000.0;
        assert_eq!(game.gained_infinities(), Decimal::ONE);
        game.records.this_infinity.time_ms = 6000.0;
        assert_eq!(game.gained_infinities(), Decimal::from_float(250.0));
    }

    #[test]
    fn achievement_94_doubles_first_infinity_dimension() {
        let mut game = GameState::new();
        let before = game.id_multiplier(0);
        game.unlock_achievement(94);
        assert_eq!(game.id_multiplier(0), before * Decimal::from_float(2.0));
    }

    #[test]
    fn achievement_83_unlocks_at_50_galaxies_and_speeds_tickspeed() {
        let mut game = GameState::new();
        game.galaxies = 50;
        game.check_galaxy_after_achievements();
        assert!(game.achievement_unlocked(83));
        // The 0.95^galaxies base-tickspeed factor speeds production up.
        let mut without = GameState::new();
        without.galaxies = 50;
        let mut with = without.clone();
        with.unlock_achievement(83);
        assert!(with.tickspeed_effect() > without.tickspeed_effect());
    }

    #[test]
    fn achievement_88_needs_a_number_max_value_sacrifice() {
        let mut game = GameState::new();
        // IC2 completed drops the log10 from the sacrifice formula, so a very
        // large AD1 can push nextBoost past Number.MAX_VALUE.
        game.complete_infinity_challenge(2);
        game.dimensions[0].amount = Decimal::new(1.0, 40_000);
        game.sacrificed = Decimal::ONE;
        game.check_sacrifice_before_achievements();
        assert!(game.achievement_unlocked(88));
    }

    #[test]
    fn achievement_95_keeps_replicanti_on_infinity() {
        let mut game = GameState::new();
        game.replicanti.unlocked = true;
        game.replicanti.galaxies = 1;
        game.records.this_infinity.real_time_ms = 0.0;
        game.check_replicanti_after_achievements();
        assert!(game.achievement_unlocked(95));
    }

    #[test]
    fn achievement_98_and_102_from_tick() {
        let mut game = GameState::new();
        game.infinity_dimensions[7].is_unlocked = true;
        game.eternities = Decimal::from_float(1000.0); // past every milestone (max 1000)
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(98));
        assert!(game.achievement_unlocked(102));
    }

    #[test]
    fn achievement_97_from_fast_infinity_challenges() {
        let mut game = GameState::new();
        // All eight IC best times tiny → sum well under 6.66 s.
        game.ic_best_times_ms = [100.0; 8];
        game.check_crunch_after_achievements();
        assert!(game.achievement_unlocked(97));
    }

    #[test]
    fn eternity_unlocks_96_101_104_and_104_sets_starting_ip() {
        let mut game = GameState::new();
        game.infinity_unlocked = true;
        game.infinity_points = crate::ETERNITY_GOAL;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        game.requirement_checks.eternity_only_ad8 = true;
        // Fast (game time 0) eternity → 104.
        assert!(game.eternity());
        assert!(game.achievement_unlocked(96));
        assert!(game.achievement_unlocked(101));
        assert!(game.achievement_unlocked(104));
        // 104 → start Eternities with 5e25 IP.
        assert_eq!(game.infinity_points, Decimal::new(5.0, 25));
    }

    #[test]
    fn achievement_82_needs_all_infinity_challenges() {
        let mut game = GameState::new();
        for id in 1..=crate::INFINITY_CHALLENGE_COUNT {
            game.complete_infinity_challenge(id);
        }
        assert!(game.achievement_unlocked(82));
    }

    // ---- Batch 4 (ids 105–128) ----

    #[test]
    fn achievement_128_scales_time_dimensions_by_study_count() {
        let mut game = GameState::new();
        game.studies = vec![11, 21, 31]; // 3 studies (none with a TD effect)
        assert_eq!(game.td_common_multiplier(), Decimal::ONE);
        game.unlock_achievement(128);
        assert_eq!(game.td_common_multiplier(), Decimal::from_float(3.0));
    }

    #[test]
    fn achievement_111_keeps_antimatter_on_dimension_boost() {
        let mut game = GameState::new();
        game.dimensions[3].amount = Decimal::from_float(20.0); // first-boost req
        game.antimatter = Decimal::new(1.0, 50);
        game.unlock_achievement(111);
        assert!(game.buy_dim_boost());
        // Antimatter is kept (not reset to the starting value).
        assert_eq!(game.antimatter, Decimal::new(1.0, 50));
    }

    #[test]
    fn achievement_113_doubles_eternities() {
        let mut game = GameState::new();
        let before = game.gained_eternities();
        game.unlock_achievement(113);
        assert_eq!(game.gained_eternities(), before * Decimal::from_float(2.0));
    }

    #[test]
    fn achievement_117_strengthens_dimension_boosts() {
        let mut game = GameState::new();
        let before = game.dim_boost_power();
        game.unlock_achievement(117);
        assert_eq!(game.dim_boost_power(), before * Decimal::from_float(1.01));
    }

    #[test]
    fn achievement_118_keeps_dimensions_on_sacrifice() {
        let mut game = GameState::new();
        game.dim_boosts = 5;
        game.dimensions[7].amount = Decimal::ONE;
        game.dimensions[0].amount = Decimal::new(1.0, 20);
        game.unlock_achievement(118);
        assert!(game.sacrifice());
        // AD1 is not wiped by the sacrifice.
        assert_eq!(game.dimensions[0].amount, Decimal::new(1.0, 20));
    }

    #[test]
    fn achievement_116_multiplies_ip_by_total_infinities() {
        let mut game = GameState::new();
        game.infinities = Decimal::new(1.0, 10);
        assert_eq!(game.total_ip_mult(), Decimal::ONE);
        game.unlock_achievement(116);
        assert!(game.total_ip_mult() > Decimal::ONE);
    }

    #[test]
    fn eternity_before_conditions_107_108() {
        let mut game = GameState::new();
        game.infinities = Decimal::from_float(5.0); // < 10, > 1
        game.replicanti.amount = Decimal::from_float(9.0);
        game.check_eternity_before_achievements();
        assert!(game.achievement_unlocked(107));
        assert!(game.achievement_unlocked(108));
        assert!(!game.achievement_unlocked(116)); // needs infinities ≤ 1
    }

    #[test]
    fn achievement_123_needs_50_ec_completions() {
        let mut game = GameState::new();
        game.eternity_challenges = [5; 12]; // 60 total
        game.check_eternity_after_achievements();
        assert!(game.achievement_unlocked(123));
    }

    #[test]
    fn achievement_114_on_challenge_failed() {
        let mut game = GameState::new();
        game.check_challenge_failed_achievements();
        assert!(game.achievement_unlocked(114));
    }

    #[test]
    fn achievement_124_marathon_needs_sixty_seconds() {
        let mut game = GameState::new();
        game.infinity_dimensions[0].is_unlocked = true;
        game.infinity_dimensions[0].amount = Decimal::new(1.0, 10);
        game.infinity_power = Decimal::ZERO;
        // Production/s exceeds Infinity Power; needs 60 game-seconds.
        game.check_tick_achievements(30_000.0);
        assert!(!game.achievement_unlocked(124));
        game.check_tick_achievements(30_000.0);
        assert!(game.achievement_unlocked(124));
    }

    // ---- Batch 5 (ids 131–154) ----

    #[test]
    fn achievement_131_doubles_infinities() {
        let mut game = GameState::new();
        let before = game.gained_infinities();
        game.unlock_achievement(131);
        assert_eq!(game.gained_infinities(), before * Decimal::from_float(2.0));
    }

    #[test]
    fn achievement_141_multiplies_ip_and_buy_ten() {
        let mut game = GameState::new();
        let ip_before = game.total_ip_mult();
        let buy10_before = game.buy_ten_multiplier();
        game.unlock_achievement(141);
        assert_eq!(game.total_ip_mult(), ip_before * Decimal::from_float(4.0));
        // Buy-10 base rises by +0.1 (2.0 → 2.1).
        assert_eq!(
            game.buy_ten_multiplier(),
            buy10_before + Decimal::from_float(0.1)
        );
    }

    #[test]
    fn achievement_142_strengthens_dimension_boosts() {
        let mut game = GameState::new();
        let before = game.dim_boost_power();
        game.unlock_achievement(142);
        assert_eq!(game.dim_boost_power(), before * Decimal::from_float(1.5));
    }

    #[test]
    fn achievement_143_keeps_boosts_through_galaxy() {
        let mut game = GameState::new();
        game.dim_boosts = 4;
        game.dimensions[7].amount = Decimal::new(1.0, 6); // clears galaxy req
        game.unlock_achievement(143);
        assert!(game.buy_galaxy());
        assert_eq!(game.galaxies, 1);
        // Dimension Boosts survive the galaxy.
        assert_eq!(game.dim_boosts, 4);
    }

    #[test]
    fn achievement_145_shortens_black_hole_interval() {
        let mut game = GameState::new();
        let before = game.black_hole_interval(0);
        game.unlock_achievement(145);
        assert!(game.black_hole_interval(0) < before);
    }

    #[test]
    fn achievement_132_multiplies_tachyon_gain() {
        let mut game = GameState::new();
        game.galaxies = 100;
        let before = game.tachyon_gain_multiplier();
        game.unlock_achievement(132);
        assert!(game.tachyon_gain_multiplier() > before);
    }

    #[test]
    fn achievement_137_doubles_dilated_time_while_dilated() {
        let mut game = GameState::new();
        game.dilation.tachyon_particles = Decimal::ONE;
        game.dilation.active = true;
        let before = game.dilation_gain_per_second();
        game.unlock_achievement(137);
        assert_eq!(
            game.dilation_gain_per_second(),
            before * Decimal::from_float(2.0)
        );
    }

    #[test]
    fn reality_before_conditions_141_153_154() {
        let mut game = GameState::new();
        game.requirement_checks.reality_no_am = true;
        game.records.this_reality.time_ms = 0.0;
        game.check_reality_before_achievements();
        assert!(game.achievement_unlocked(141));
        assert!(game.achievement_unlocked(153));
        assert!(game.achievement_unlocked(154));
    }

    #[test]
    fn achievement_144_on_black_hole_unlock() {
        let mut game = GameState::new();
        game.check_black_hole_unlocked_achievements();
        assert!(game.achievement_unlocked(144));
    }

    #[test]
    fn achievement_147_needs_all_reality_upgrades() {
        let mut game = GameState::new();
        for id in 6..=25 {
            game.reality.upgrade_bits |= 1u32 << id;
        }
        game.check_reality_upgrade_bought_achievements();
        assert!(game.achievement_unlocked(147));
    }

    // ---- Batch 6 (ids 155–178) ----

    #[test]
    fn achievement_164_multiplies_infinities() {
        let mut game = GameState::new();
        let before = game.gained_infinities();
        game.unlock_achievement(164);
        assert_eq!(
            game.gained_infinities(),
            before * Decimal::from_float(1024.0)
        );
    }

    #[test]
    fn achievement_178_strengthens_galaxies() {
        let mut game = GameState::new();
        let before = game.galaxy_strength_effect();
        game.unlock_achievement(178);
        assert!((game.galaxy_strength_effect() - before * 1.01).abs() < 1e-12);
    }

    #[test]
    fn achievements_155_158_boost_black_holes() {
        let mut game = GameState::new();
        let dur_before = game.black_hole_duration(0);
        let pow_before = game.black_hole_power(0);
        game.unlock_achievement(155);
        game.unlock_achievement(158);
        assert!((game.black_hole_duration(0) - dur_before * 1.1).abs() < 1e-9);
        assert!((game.black_hole_power(0) - pow_before * 1.1).abs() < 1e-9);
    }

    #[test]
    fn achievement_166_adds_69_glyph_levels() {
        let mut game = GameState::new();
        assert_eq!(game.achievement_glyph_level_bonus(), 0);
        game.unlock_achievement(166);
        assert_eq!(game.achievement_glyph_level_bonus(), 69);
    }

    #[test]
    fn achievement_162_needs_all_time_studies() {
        let mut game = GameState::new();
        game.studies = (1..=58).collect();
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(162));
    }

    #[test]
    fn achievement_171_needs_every_glyph_type_sacrificed() {
        let mut game = GameState::new();
        game.reality.glyphs.sac = [1.0; 5];
        game.check_tick_achievements(50.0);
        assert!(game.achievement_unlocked(171));
    }

    #[test]
    fn achievement_176_on_annihilation() {
        let mut game = GameState::new();
        game.check_annihilation_achievements();
        assert!(game.achievement_unlocked(176));
    }

    #[test]
    fn singularity_achievements_174_177() {
        let mut game = GameState::new();
        game.check_singularity_before_achievements();
        assert!(game.achievement_unlocked(174));
        // Enough Singularities that every milestone has ≥ 1 completion.
        game.celestials.laitela.singularities = 1e300;
        game.check_singularity_after_achievements();
        assert!(game.achievement_unlocked(177));
    }
}
