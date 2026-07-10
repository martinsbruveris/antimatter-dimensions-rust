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
//! availability / tickspeed unlock / Replicanti-Galaxy purchase). The
//! milestone autobuyers (1 IP-mult, 3 RG, 5 bigCrunchModes, 9 buy-max
//! Galaxies, 11–18 ID, 50/60/80 Replicanti upgrades, 100 autobuyerEternity)
//! are wired in `autobuyers.rs`; the offline generators (6 autoEP, 200
//! autoEternities, 1000 autoInfinities — `auto_eternities_available` /
//! `auto_infinities_available` below) fire from `offline_currency_gain`
//! (tick.rs). See `docs/design/2026-07-04-eternity.md` §2.

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

    /// `Autobuyer.eternity.autoEternitiesAvailable` (the `autoEternities`
    /// milestone, 200 Eternities): passive offline Eternity generation is on
    /// when the Eternity autobuyer idles at an Amount goal of 0, outside any
    /// challenge and not Dilated.
    pub(crate) fn auto_eternities_available(&self) -> bool {
        use crate::autobuyers::PrestigeAutobuyerMode;
        self.eternity_milestone_reached(200)
            && !self.in_any_antimatter_challenge()
            && !self.any_ec_running()
            && !self.dilation.active
            && self.autobuyers.enabled
            && self.autobuyers.eternity.is_active
            && self.autobuyers.eternity.settings.mode == PrestigeAutobuyerMode::Amount
            && self.autobuyers.eternity.settings.amount == break_infinity::Decimal::ZERO
    }

    /// `Autobuyer.bigCrunch.autoInfinitiesAvailable` (the `autoInfinities`
    /// milestone, 1000 Eternities): passive offline Infinity generation is on
    /// when the Big Crunch autobuyer runs a ≤5 s Time goal, the Eternity
    /// autobuyer is off, and no challenge blocks it.
    pub(crate) fn auto_infinities_available(&self) -> bool {
        use crate::autobuyers::PrestigeAutobuyerMode;
        self.eternity_milestone_reached(1000)
            && !self.ec_running(4)
            && !self.ec_running(12)
            && !self.in_any_antimatter_challenge()
            && self.autobuyers.enabled
            && self.autobuyers.big_crunch.is_active
            && !self.autobuyers.eternity.is_active
            && self.autobuyers.big_crunch_settings.mode == PrestigeAutobuyerMode::Time
            && self.autobuyers.big_crunch_settings.time <= 5.0
            && !self.auto_eternities_available()
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
    fn id_autobuyer_buys_max_on_its_interval() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(11.0); // autobuyerID1
        game.infinity_points = Decimal::new(1.0, 10);
        game.infinity_dimensions[0].is_unlocked = true;
        game.autobuyers.infinity_dims[0].is_active = true;
        // Carry a full interval so the first tick fires.
        game.autobuyers.infinity_dims[0].timer_ms = 1_000.0;
        game.tick(50.0);
        assert!(game.infinity_dimensions[0].base_amount > 0);

        // Below the milestone nothing fires.
        let mut locked = GameState::new();
        locked.eternities = Decimal::from_float(10.0);
        locked.infinity_points = Decimal::new(1.0, 10);
        locked.infinity_dimensions[0].is_unlocked = true;
        locked.autobuyers.infinity_dims[0].is_active = true;
        locked.autobuyers.infinity_dims[0].timer_ms = 1_000.0;
        locked.tick(50.0);
        assert_eq!(locked.infinity_dimensions[0].base_amount, 0);
    }

    #[test]
    fn replicanti_galaxy_autobuyer_fires_each_tick() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(3.0); // autobuyerReplicantiGalaxy
        game.replicanti.unlocked = true;
        game.replicanti.amount = crate::REPLICANTI_CAP;
        game.replicanti.galaxy_cap = 1;
        game.autobuyers.replicanti_galaxies_active = true;
        game.tick(50.0);
        assert_eq!(game.replicanti.galaxies, 1);
    }

    #[test]
    fn replicanti_upgrade_autobuyer_buys_to_the_max() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(50.0); // chance autobuyer
        game.replicanti.unlocked = true;
        // Enough for exactly two chance upgrades (1e150 then 1e165).
        game.infinity_points = Decimal::new(2.0, 165);
        game.autobuyers.replicanti_upgrades[0].is_active = true;
        game.autobuyers.replicanti_upgrades[0].timer_ms = 1_000.0;
        game.tick(50.0);
        let chance = (game.replicanti.chance * 100.0).round();
        assert_eq!(chance, 3.0, "chance={}", game.replicanti.chance);
    }

    #[test]
    fn offline_generators_follow_the_milestone_priority() {
        // autoEternities (200): banks eternities at half the best rate.
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(200.0);
        game.autobuyers.eternity.is_active = true;
        // Amount mode with goal 0 (the availability condition).
        game.autobuyers.eternity.settings.amount = Decimal::ZERO;
        game.records.this_reality.best_eternities_per_ms = Decimal::from_float(0.01);
        game.offline_currency_gain(10_000.0);
        // 0.01/ms × 10000 ms / 2 = 50.
        assert_eq!(game.eternities, Decimal::from_float(250.0));

        // autoEP (6): with no generator available, EP accrues at 25% of the
        // best rate.
        let mut ep_game = GameState::new();
        ep_game.eternities = Decimal::from_float(6.0);
        ep_game.records.best_eternity.best_ep_min_reality = Decimal::from_float(8.0);
        ep_game.offline_currency_gain(120_000.0); // 2 minutes
        assert_eq!(ep_game.eternity_points, Decimal::from_float(4.0));
    }

    #[test]
    fn galaxy_autobuyer_buys_max_with_the_milestone() {
        let mut game = GameState::new();
        game.eternities = Decimal::from_float(9.0); // autobuyMaxGalaxies
        game.dim_boosts = 4;
        game.dimensions[7].bought = 1;
        // Enough 8th dimensions for several galaxies (req 80, 140, 200 …).
        game.dimensions[7].amount = Decimal::from_float(220.0);
        assert!(game.max_buy_galaxies(u64::MAX));
        assert_eq!(game.galaxies, 3);

        // The limit clamps the bulk.
        let mut capped = GameState::new();
        capped.eternities = Decimal::from_float(9.0);
        capped.dim_boosts = 4;
        capped.dimensions[7].bought = 1;
        capped.dimensions[7].amount = Decimal::from_float(220.0);
        assert!(capped.max_buy_galaxies(2));
        assert_eq!(capped.galaxies, 2);
    }

    #[test]
    fn td_and_ep_mult_autobuyers_run_with_ru13() {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.upgrade_bits |= 1 << 13;
        game.eternity_unlocked = true;
        game.eternity_points = Decimal::new(1.0, 20);
        game.autobuyers.time_dims[0].is_active = true;
        game.autobuyers.time_dims[0].timer_ms = 1_000.0;
        game.autobuyers.ep_mult_buyer_active = true;
        game.tick(50.0);
        // The epMult buyer bought rebuyables and the TD autobuyer bought TDs.
        assert!(game.epmult_upgrades > 0);
        assert!(game.time_dimensions[0].bought > 0);

        // Without RU13 neither runs.
        let mut locked = GameState::new();
        locked.eternity_unlocked = true;
        locked.eternity_points = Decimal::new(1.0, 20);
        locked.autobuyers.time_dims[0].is_active = true;
        locked.autobuyers.time_dims[0].timer_ms = 1_000.0;
        locked.autobuyers.ep_mult_buyer_active = true;
        locked.tick(50.0);
        assert_eq!(locked.epmult_upgrades, 0);
        assert_eq!(locked.time_dimensions[0].bought, 0);
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
