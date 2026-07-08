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
/// Excludes achievement 22 (News — unmodelled) and 35 (6-hour offline — no
/// wall-clock model), which the engine cannot earn naturally and are only ever
/// set via Reality auto-achievement or the ACHNR reality upgrade.
pub const IMPLEMENTED_ACHIEVEMENTS: &[u16] = &[
    11, 12, 13, 14, 15, 16, 17, 18, // row 1
    21, 23, 24, 25, 26, 27, 28, // row 2 (22 = News, deferred)
    31, 32, 33, 34, 36, 37, 38, // row 3 (35 = offline, deferred)
    41, 42, 43, 44, 45, 46, 47, 48, // row 4
    51, 52, 53, 54,  // row 5 (55–58 pending)
    136, // dilate time
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
        mult
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
    }

    /// BIG_CRUNCH_AFTER conditions, checked at the end of a rewarded crunch.
    pub(crate) fn check_crunch_after_achievements(&mut self) {
        // 33: reach Infinity 10 times.
        if self.infinities >= Decimal::from_float(10.0) {
            self.unlock_achievement(33);
        }
        self.check_challenge_completion_achievements();
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
    fn fast_crunch_unlocks_37_54_and_raises_starting_antimatter() {
        let mut game = GameState::new();
        // Zero real time: under 10 minutes and under 2 hours → 37 and 54.
        game.antimatter = crate::data::constants::BIG_CRUNCH_THRESHOLD;
        assert!(game.big_crunch());
        assert!(game.achievement_unlocked(37));
        assert!(game.achievement_unlocked(54));
        // startingValue = max(100, 5000, 5e5) = 5e5.
        assert_eq!(game.antimatter, Decimal::new(5.0, 5));
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
}
