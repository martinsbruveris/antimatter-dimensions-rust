//! Automator Points (the Automator's unlock currency — `automator-points.js`
//! and `secret-formula/reality/automator.js: otherAutomatorPoints`). The
//! Automator unlocks at 100 AP; points come from perks with an
//! `automator_points` value, six Reality Upgrades, Reality count, and the
//! Black Hole unlock. Stage A of Feature 6.6 — see
//! `docs/design/2026-07-05-automator.md`.

use crate::perks::PERKS;
use crate::state::GameState;

/// AP needed to unlock the Automator (`AutomatorPoints.pointsForAutomator`).
pub const AUTOMATOR_UNLOCK_POINTS: u32 = 100;

/// Reality Upgrades granting AP while bought (`automatorPoints` in
/// `secret-formula/reality/reality-upgrades.js`).
pub const UPGRADE_AUTOMATOR_POINTS: [(u8, u32); 6] =
    [(10, 15), (11, 5), (13, 10), (14, 5), (20, 10), (25, 100)];

/// AP per Reality, and the Reality count the bonus caps at ("Reality Count":
/// +2 per Reality, up to 50 Realities).
pub const AP_PER_REALITY: u32 = 2;
pub const AP_REALITY_CAP: u32 = 50;

/// AP for having unlocked Black Hole 1.
pub const AP_BLACK_HOLE: u32 = 10;

impl GameState {
    /// AP from bought perks (`AutomatorPoints.pointsFromPerks`).
    pub fn automator_points_from_perks(&self) -> u32 {
        PERKS
            .iter()
            .filter(|p| p.automator_points > 0 && self.perk_applies(p.id))
            .map(|p| p.automator_points)
            .sum()
    }

    /// AP from bought Reality Upgrades (`pointsFromUpgrades`).
    pub fn automator_points_from_upgrades(&self) -> u32 {
        UPGRADE_AUTOMATOR_POINTS
            .iter()
            .filter(|&&(id, _)| self.reality_upgrade_bought(id))
            .map(|&(_, points)| points)
            .sum()
    }

    /// AP from the "other" sources (`pointsFromOther`): Reality count and the
    /// Black Hole unlock.
    pub fn automator_points_from_other(&self) -> u32 {
        let realities = self.reality.realities.min(AP_REALITY_CAP);
        let black_hole = if self.black_holes.holes[0].unlocked {
            AP_BLACK_HOLE
        } else {
            0
        };
        AP_PER_REALITY * realities + black_hole
    }

    /// Total AP (`AutomatorPoints.totalPoints`).
    pub fn automator_points(&self) -> u32 {
        self.automator_points_from_perks()
            + self.automator_points_from_upgrades()
            + self.automator_points_from_other()
    }

    /// Whether the Automator is unlocked (`Player.automatorUnlocked`): 100 AP,
    /// or the dev/imported `forceUnlock` flag.
    pub fn automator_unlocked(&self) -> bool {
        self.automator_points() >= AUTOMATOR_UNLOCK_POINTS
            || self.reality.automator_force_unlock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ap_from_perks_counts_bought_only() {
        let mut game = GameState::new();
        assert_eq!(game.automator_points_from_perks(), 0);
        // SEP1 (14) grants 5 AP, ECB (73) grants 15.
        game.reality.perks.insert(14);
        game.reality.perks.insert(73);
        // START (0) grants none.
        game.reality.perks.insert(0);
        assert_eq!(game.automator_points_from_perks(), 20);
    }

    #[test]
    fn ap_from_upgrades() {
        let mut game = GameState::new();
        game.reality.upgrade_bits |= 1 << 10; // RU10: 15 AP
        assert_eq!(game.automator_points_from_upgrades(), 15);
        game.reality.upgrade_bits |= 1 << 25; // RU25: 100 AP
        assert_eq!(game.automator_points_from_upgrades(), 115);
    }

    #[test]
    fn ap_from_realities_caps_at_50() {
        let mut game = GameState::new();
        game.reality.realities = 3;
        assert_eq!(game.automator_points_from_other(), 6);
        game.reality.realities = 200;
        assert_eq!(game.automator_points_from_other(), 100);
        game.black_holes.holes[0].unlocked = true;
        assert_eq!(game.automator_points_from_other(), 110);
    }

    #[test]
    fn unlock_at_100_ap_or_force() {
        let mut game = GameState::new();
        assert!(!game.automator_unlocked());
        // RU25 alone unlocks the Automator (100 AP).
        game.reality.upgrade_bits |= 1 << 25;
        assert!(game.automator_unlocked());

        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        assert!(game.automator_unlocked());
    }
}
