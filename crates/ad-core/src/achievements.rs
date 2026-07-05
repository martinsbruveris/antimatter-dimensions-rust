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
pub const IMPLEMENTED_ACHIEVEMENTS: &[u16] =
    &[11, 12, 18, 21, 23, 24, 25, 26, 27, 28, 136];

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
        Decimal::from_float(power)
    }

    /// Antimatter to reset to after a dimension boost, galaxy, or Big Crunch.
    /// Mirrors `Currency.antimatter.startingValue` = `Effects.max(10,
    /// Achievement(21) = 100, …)`; pre-Infinity only achievement 21 applies.
    pub fn starting_antimatter(&self) -> Decimal {
        // The SAM perk (`Perk.startAM`): start every reset with 5e130.
        if self.perk_bought(10) {
            return Decimal::new(5.0, 130);
        }
        if self.achievement_unlocked(21) {
            Decimal::from_float(100.0)
        } else {
            Decimal::from_float(INITIAL_ANTIMATTER)
        }
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
}
