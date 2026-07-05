//! Eternity Milestones (Feature 4.2): permanent unlocks earned by total
//! Eternities performed. Pure derived state — `isReached ⇔ eternities >=
//! threshold` — so nothing is saved beyond the eternities count itself.
//!
//! Mirrors `secret-formula/eternity/eternity-milestones.js` (thresholds/ids)
//! and the effect sites spread through the original core. The reset-time keeps
//! (2 keepAutobuyers / 4 keepInfinityUpgrades / 8 keepBreakUpgrades /
//! 10 unlockReplicanti) are applied in `eternity.rs::eternity_reset`; the
//! per-tick effects (7 autoIC, 25 autoUnlockID) hook into `tick`; 30
//! unlockAllND and 40 replicantiNoReset are read at their sites (dimension
//! availability / tickspeed unlock / Replicanti-Galaxy purchase). Milestones
//! 5 (`bigCrunchModes`) and 100 (`autobuyerEternity`) are wired in
//! `autobuyers.rs` (Automator Stage A). The milestones that unlock autobuyer
//! types we haven't built yet (1, 3, 9, 11–18, 50, 60, 80) and the offline
//! generators (6, 200, 1000) display as reached but have no engine effect
//! until those systems exist. See `design-docs/2026-07-04-eternity.md` §2.

use crate::state::GameState;

/// One milestone: its original config key and the eternities required. Display
/// strings live frontend-side (as for achievements).
#[derive(Debug, Clone, Copy)]
pub struct EternityMilestone {
    /// Original id (`GameDatabase.eternity.milestones` key).
    pub id: &'static str,
    /// Eternities required for the milestone to be reached.
    pub eternities: u64,
}

/// The full catalogue, in the original's threshold order.
pub const ETERNITY_MILESTONES: [EternityMilestone; 27] = [
    EternityMilestone {
        id: "autobuyerIPMult",
        eternities: 1,
    },
    EternityMilestone {
        id: "keepAutobuyers",
        eternities: 2,
    },
    EternityMilestone {
        id: "autobuyerReplicantiGalaxy",
        eternities: 3,
    },
    EternityMilestone {
        id: "keepInfinityUpgrades",
        eternities: 4,
    },
    EternityMilestone {
        id: "bigCrunchModes",
        eternities: 5,
    },
    EternityMilestone {
        id: "autoEP",
        eternities: 6,
    },
    EternityMilestone {
        id: "autoIC",
        eternities: 7,
    },
    EternityMilestone {
        id: "keepBreakUpgrades",
        eternities: 8,
    },
    EternityMilestone {
        id: "autobuyMaxGalaxies",
        eternities: 9,
    },
    EternityMilestone {
        id: "unlockReplicanti",
        eternities: 10,
    },
    EternityMilestone {
        id: "autobuyerID1",
        eternities: 11,
    },
    EternityMilestone {
        id: "autobuyerID2",
        eternities: 12,
    },
    EternityMilestone {
        id: "autobuyerID3",
        eternities: 13,
    },
    EternityMilestone {
        id: "autobuyerID4",
        eternities: 14,
    },
    EternityMilestone {
        id: "autobuyerID5",
        eternities: 15,
    },
    EternityMilestone {
        id: "autobuyerID6",
        eternities: 16,
    },
    EternityMilestone {
        id: "autobuyerID7",
        eternities: 17,
    },
    EternityMilestone {
        id: "autobuyerID8",
        eternities: 18,
    },
    EternityMilestone {
        id: "autoUnlockID",
        eternities: 25,
    },
    EternityMilestone {
        id: "unlockAllND",
        eternities: 30,
    },
    EternityMilestone {
        id: "replicantiNoReset",
        eternities: 40,
    },
    EternityMilestone {
        id: "autobuyerReplicantiChance",
        eternities: 50,
    },
    EternityMilestone {
        id: "autobuyerReplicantiInterval",
        eternities: 60,
    },
    EternityMilestone {
        id: "autobuyerReplicantiMaxGalaxies",
        eternities: 80,
    },
    EternityMilestone {
        id: "autobuyerEternity",
        eternities: 100,
    },
    EternityMilestone {
        id: "autoEternities",
        eternities: 200,
    },
    EternityMilestone {
        id: "autoInfinities",
        eternities: 1000,
    },
];

impl GameState {
    /// `tryCompleteInfinityChallenges` (autoIC milestone, 7 eternities):
    /// complete every unlocked-but-incomplete Infinity Challenge. Runs each
    /// tick, so an IC completes the moment its antimatter threshold is crossed.
    pub(crate) fn try_complete_infinity_challenges(&mut self) {
        if !self.eternity_milestone_reached(7) {
            return;
        }
        for id in 1..=crate::INFINITY_CHALLENGE_COUNT {
            if self.infinity_challenge_unlocked(id)
                && !self.infinity_challenge_completed(id)
            {
                self.complete_infinity_challenge(id);
            }
        }
    }

    /// `InfinityDimensions.tryAutoUnlock` (autoUnlockID milestone, 25
    /// eternities): unlock Infinity Dimensions as soon as they are reachable.
    pub(crate) fn try_auto_unlock_infinity_dimensions(&mut self) {
        if !self.eternity_milestone_reached(25)
            || self.infinity_dimensions[crate::INFINITY_DIMENSION_COUNT - 1].is_unlocked
        {
            return;
        }
        for tier in 0..crate::INFINITY_DIMENSION_COUNT {
            if self.infinity_dimensions[tier].is_unlocked {
                continue;
            }
            // If this one cannot be unlocked, the rest can't either.
            if !self.unlock_infinity_dimension(tier) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use break_infinity::Decimal;

    #[test]
    fn milestones_reached_by_eternity_count() {
        let mut game = GameState::new();
        assert!(!game.eternity_milestone_reached(1));
        game.eternities = Decimal::from_float(7.0);
        assert!(game.eternity_milestone_reached(7));
        assert!(!game.eternity_milestone_reached(8));
    }

    #[test]
    fn auto_ic_completes_unlocked_challenges() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(7.0);
        game.records.this_eternity.max_am = Decimal::new(1.0, 2500); // IC1 (1e2000)
        game.tick(50.0);
        assert!(game.infinity_challenge_completed(1));
        assert!(!game.infinity_challenge_completed(2)); // 1e5000 not reached
    }

    #[test]
    fn auto_ic_requires_milestone() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(6.0);
        game.records.this_eternity.max_am = Decimal::new(1.0, 2500);
        game.tick(50.0);
        assert!(!game.infinity_challenge_completed(1));
    }

    #[test]
    fn auto_unlock_id_unlocks_reachable_tiers() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(25.0);
        game.infinity_points = Decimal::new(1.0, 12); // ID1's 1e8-IP requirement
        game.records.this_eternity.max_am = Decimal::new(1.0, 2000); // ID1+ID2
        game.tick(50.0);
        assert!(game.infinity_dimensions[0].is_unlocked);
        assert!(game.infinity_dimensions[1].is_unlocked);
        assert!(!game.infinity_dimensions[2].is_unlocked); // needs 1e2400
    }

    #[test]
    fn unlock_all_nd_opens_every_tier_and_tickspeed() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(30.0);
        assert_eq!(game.unlocked_dimensions(), 8);
        // Purchasable without owning the tier below.
        assert!(game.dim_available_for_purchase(7));
        // Tickspeed no longer needs an AD2 purchase.
        assert!(game.tickspeed_unlocked());
    }

    #[test]
    fn replicanti_no_reset_keeps_progress_on_rg() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(40.0);
        game.replicanti.unlocked = true;
        game.replicanti.amount = crate::REPLICANTI_CAP;
        game.replicanti.galaxy_cap = 1;
        game.dim_boosts = 6;
        game.antimatter = Decimal::new(1.0, 50);

        assert!(game.buy_replicanti_galaxy());
        assert_eq!(game.replicanti.galaxies, 1);
        assert_eq!(game.replicanti.amount, Decimal::ONE);
        // Dimension Boosts (and the boost-style reset) are skipped.
        assert_eq!(game.dim_boosts, 6);
        assert_eq!(game.antimatter, Decimal::new(1.0, 50));
    }
}
