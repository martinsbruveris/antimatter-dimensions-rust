//! Perks (Feature 6.3): a 35-node tree of permanent QoL/progression unlocks,
//! 1 Perk Point each; a perk is buyable when adjacent to an owned perk.
//!
//! Mirrors `src/core/perks.js` and `secret-formula/reality/perks.js`
//! (catalogue + connection graph). Effects apply at their engine sites (each
//! site names its perk); the perks whose target system doesn't exist yet
//! (EC auto-completion, TT/dilation/ID/replicanti autobuyer improvements)
//! are tracked but deferred — see `design-docs/2026-07-05-reality.md`.

use break_infinity::Decimal;

use crate::state::GameState;

/// Perk families (`PERK_FAMILY`), for node coloring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerkFamily {
    Antimatter,
    Infinity,
    Eternity,
    Dilation,
    Reality,
    Automation,
    Achievement,
}

/// One perk (id, label, family; descriptions live frontend-side).
#[derive(Debug, Clone, Copy)]
pub struct PerkDef {
    pub id: u8,
    pub label: &'static str,
    pub family: PerkFamily,
    /// Automator Points granted while bought (`automatorPoints` in the
    /// original's perk config; 0 = none).
    pub automator_points: u32,
}

use PerkFamily::*;

/// The full catalogue, in the original's declaration order.
pub const PERKS: &[PerkDef] = &[
    PerkDef {
        id: 0,
        label: "START",
        family: Reality,
        automator_points: 0,
    },
    PerkDef {
        id: 10,
        label: "SAM",
        family: Antimatter,
        automator_points: 0,
    },
    PerkDef {
        id: 12,
        label: "SIP1",
        family: Infinity,
        automator_points: 0,
    },
    PerkDef {
        id: 13,
        label: "SIP2",
        family: Infinity,
        automator_points: 0,
    },
    PerkDef {
        id: 14,
        label: "SEP1",
        family: Eternity,
        automator_points: 5,
    },
    PerkDef {
        id: 15,
        label: "SEP2",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 16,
        label: "SEP3",
        family: Eternity,
        automator_points: 10,
    },
    PerkDef {
        id: 17,
        label: "STP",
        family: Dilation,
        automator_points: 5,
    },
    PerkDef {
        id: 30,
        label: "ANR",
        family: Antimatter,
        automator_points: 0,
    },
    PerkDef {
        id: 31,
        label: "PASS",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 40,
        label: "EU1",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 41,
        label: "EU2",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 42,
        label: "DU1",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 43,
        label: "DU2",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 44,
        label: "ATT",
        family: Dilation,
        automator_points: 5,
    },
    PerkDef {
        id: 45,
        label: "ATD",
        family: Dilation,
        automator_points: 5,
    },
    PerkDef {
        id: 46,
        label: "REAL",
        family: Reality,
        automator_points: 10,
    },
    PerkDef {
        id: 51,
        label: "IDR",
        family: Infinity,
        automator_points: 0,
    },
    PerkDef {
        id: 52,
        label: "TGR",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 53,
        label: "DILR",
        family: Dilation,
        automator_points: 5,
    },
    PerkDef {
        id: 54,
        label: "EC1R",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 55,
        label: "EC2R",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 56,
        label: "EC3R",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 57,
        label: "EC5R",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 60,
        label: "PEC1",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 61,
        label: "PEC2",
        family: Automation,
        automator_points: 0,
    },
    PerkDef {
        id: 62,
        label: "PEC3",
        family: Automation,
        automator_points: 10,
    },
    PerkDef {
        id: 70,
        label: "ACT",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 71,
        label: "IDL",
        family: Eternity,
        automator_points: 0,
    },
    PerkDef {
        id: 72,
        label: "ECR",
        family: Eternity,
        automator_points: 10,
    },
    PerkDef {
        id: 73,
        label: "ECB",
        family: Eternity,
        automator_points: 15,
    },
    PerkDef {
        id: 80,
        label: "TP1",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 81,
        label: "TP2",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 82,
        label: "TP3",
        family: Dilation,
        automator_points: 0,
    },
    PerkDef {
        id: 83,
        label: "TP4",
        family: Dilation,
        automator_points: 10,
    },
    PerkDef {
        id: 100,
        label: "DAU",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 101,
        label: "IDAS",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 102,
        label: "REPAS",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 103,
        label: "DAS",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 104,
        label: "TTS",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 105,
        label: "TTF",
        family: Automation,
        automator_points: 0,
    },
    PerkDef {
        id: 106,
        label: "TTM",
        family: Automation,
        automator_points: 10,
    },
    PerkDef {
        id: 107,
        label: "DAB",
        family: Automation,
        automator_points: 5,
    },
    PerkDef {
        id: 201,
        label: "ACH1",
        family: Achievement,
        automator_points: 5,
    },
    PerkDef {
        id: 202,
        label: "ACH2",
        family: Achievement,
        automator_points: 0,
    },
    PerkDef {
        id: 203,
        label: "ACH3",
        family: Achievement,
        automator_points: 0,
    },
    PerkDef {
        id: 204,
        label: "ACH4",
        family: Achievement,
        automator_points: 0,
    },
    PerkDef {
        id: 205,
        label: "ACHNR",
        family: Achievement,
        automator_points: 10,
    },
];

/// Undirected adjacency (from `perkConnections`' groups). A perk is
/// purchasable when it's `firstPerk` or shares an edge with an owned perk.
pub const PERK_CONNECTIONS: &[(u8, u8)] = &[
    (0, 201),
    (0, 10),
    (0, 40),
    (0, 57),
    (10, 30),
    (10, 12),
    (30, 14),
    (12, 13),
    (12, 14),
    (12, 101),
    (13, 51),
    (13, 102),
    (14, 15),
    (14, 17),
    (15, 16),
    (17, 80),
    (40, 41),
    (41, 100),
    (42, 43),
    (43, 44),
    (44, 103),
    (44, 45),
    (45, 46),
    (52, 100),
    (52, 80),
    (54, 55),
    (54, 56),
    (54, 72),
    (54, 31),
    (55, 70),
    (56, 71),
    (57, 70),
    (57, 71),
    (57, 31),
    (60, 61),
    (61, 62),
    (70, 104),
    (71, 60),
    (72, 73),
    (80, 81),
    (81, 82),
    (82, 83),
    (100, 42),
    (100, 53),
    (100, 107),
    (104, 105),
    (105, 106),
    (201, 202),
    (202, 203),
    (203, 204),
    (204, 205),
];

impl GameState {
    /// Whether perk `id` exists in the catalogue.
    fn perk_exists(id: u8) -> bool {
        PERKS.iter().any(|p| p.id == id)
    }

    /// Whether perk `id` can be bought now (`PerkState.canBeBought`): not
    /// owned, 1 PP available, and reachable (firstPerk or adjacent to an
    /// owned perk).
    pub fn can_buy_perk(&self, id: u8) -> bool {
        if !Self::perk_exists(id) || self.perk_bought(id) {
            return false;
        }
        if self.reality.perk_points < 1.0 {
            return false;
        }
        id == 0
            || PERK_CONNECTIONS.iter().any(|&(a, b)| {
                (a == id && self.perk_bought(b)) || (b == id && self.perk_bought(a))
            })
    }

    /// Buy perk `id`. Returns whether it happened.
    pub fn buy_perk(&mut self, id: u8) -> bool {
        if !self.can_buy_perk(id) {
            return false;
        }
        self.reality.perk_points -= 1.0;
        self.reality.perks.insert(id);
        self.on_perk_purchased(id);
        true
    }

    /// `PerkState.onPurchased`: the immediate side effects.
    fn on_perk_purchased(&mut self, id: u8) {
        match id {
            // START-family currency bumps (`bumpCurrency`).
            10 => self.antimatter = self.antimatter.max(&Decimal::new(5.0, 130)),
            12 => {
                self.infinity_points = self.infinity_points.max(&Decimal::new(5.0, 15))
            }
            13 => {
                self.infinity_points = self.infinity_points.max(&Decimal::new(5.0, 130))
            }
            14 => {
                self.eternity_points =
                    self.eternity_points.max(&Decimal::from_float(10.0))
            }
            15 => {
                self.eternity_points =
                    self.eternity_points.max(&Decimal::from_float(5000.0))
            }
            16 => self.eternity_points = self.eternity_points.max(&Decimal::new(5.0, 9)),
            // EU1: applies immediately once you have Eternities (the
            // eternities check lives inside `apply_eu1`).
            40 => self.apply_eu1(),
            // ACHNR: instantly unlock the pre-Reality achievements (counts as
            // auto-granted if any were missing).
            205 => {
                if !self.pre_reality_achievements_complete() {
                    self.reality.gained_auto_achievements = true;
                }
                for row in 1..=crate::reality::PRE_REALITY_ACHIEVEMENT_ROWS as u16 {
                    for column in 1..=crate::achievements::ACHIEVEMENTS_PER_ROW {
                        self.unlock_achievement(row * 10 + column);
                    }
                }
            }
            _ => {}
        }
    }

    /// The starting Infinity Points after an Eternity/Reality
    /// (`Currency.infinityPoints.startingValue`; Achievement 104 deferred).
    pub(crate) fn starting_ip(&self) -> Decimal {
        let mut value = Decimal::ZERO;
        if self.perk_bought(12) {
            value = value.max(&Decimal::new(5.0, 15));
        }
        if self.perk_bought(13) {
            value = value.max(&Decimal::new(5.0, 130));
        }
        value
    }

    /// The starting Eternity Points after a Reality
    /// (`Currency.eternityPoints.startingValue`).
    pub(crate) fn starting_ep(&self) -> Decimal {
        let mut value = Decimal::ZERO;
        if self.perk_bought(14) {
            value = value.max(&Decimal::from_float(10.0));
        }
        if self.perk_bought(15) {
            value = value.max(&Decimal::from_float(5000.0));
        }
        if self.perk_bought(16) {
            value = value.max(&Decimal::new(5.0, 9));
        }
        value
    }

    /// `applyEU1`: the EU1 perk grants the first Eternity Upgrade row for
    /// free once you have Eternities.
    pub(crate) fn apply_eu1(&mut self) {
        if !self.perk_bought(40) || self.eternities == Decimal::ZERO {
            return;
        }
        for upgrade in crate::ALL_ETERNITY_UPGRADES.iter().take(3) {
            self.eternity_upgrades |= upgrade.bit();
        }
    }

    /// `applyEU2`: with the EU2 perk the second row auto-purchases at 1e10×
    /// less than list price.
    pub(crate) fn apply_eu2(&mut self) {
        if !self.perk_bought(41) {
            return;
        }
        for upgrade in crate::ALL_ETERNITY_UPGRADES.iter().skip(3) {
            if self.eternity_upgrade_bought(*upgrade) {
                continue;
            }
            if self.eternity_points >= upgrade.cost() / Decimal::new_unchecked(1.0, 10) {
                self.eternity_upgrades |= upgrade.bit();
            }
        }
    }

    /// Per-tick perk automation: EU auto-grants and the auto-unlock perks
    /// for dilation upgrades / Time Dimensions / the Reality study.
    pub(crate) fn tick_perk_effects(&mut self) {
        if self.reality.perks.is_empty() {
            return;
        }
        self.apply_eu1();
        self.apply_eu2();
        // ATT (44): auto-purchase the TT-generation Dilation Upgrade.
        if self.perk_bought(44) && self.can_buy_dilation_upgrade(10) {
            self.buy_dilation_upgrade(10);
        }
        // ATD (45): auto-unlock TD5–8 once affordable.
        if self.perk_bought(45) {
            for id in 2..=5u8 {
                if self.can_buy_dilation_study(id) {
                    self.buy_dilation_study(id);
                }
            }
        }
        // REAL (46): auto-unlock the Reality study.
        if self.perk_bought(46) && self.can_buy_dilation_study(6) {
            self.buy_dilation_study(6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with_pp(pp: f64) -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game.reality.perk_points = pp;
        game
    }

    #[test]
    fn first_perk_is_the_only_root() {
        let mut game = game_with_pp(2.0);
        assert!(!game.can_buy_perk(10)); // not adjacent to anything owned
        assert!(game.buy_perk(0));
        assert_eq!(game.reality.perk_points, 1.0);
        assert!(game.can_buy_perk(10)); // START—SAM edge
        assert!(!game.can_buy_perk(12)); // needs SAM first
        assert!(game.buy_perk(10));
        assert!(!game.can_buy_perk(12)); // out of PP
    }

    #[test]
    fn start_perks_bump_currencies_and_persist_on_reset() {
        let mut game = game_with_pp(10.0);
        game.buy_perk(0);
        game.buy_perk(10);
        assert!(game.antimatter >= Decimal::new(5.0, 130));
        game.buy_perk(12);
        assert_eq!(game.infinity_points, Decimal::new(5.0, 15));

        // Eternity starts with the perk IP.
        game.infinity_points = crate::ETERNITY_GOAL;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());
        assert_eq!(game.infinity_points, Decimal::new(5.0, 15));
        assert_eq!(game.records.this_eternity.max_ip, Decimal::new(5.0, 15));
    }

    #[test]
    fn eu1_and_eu2_auto_grant() {
        let mut game = game_with_pp(10.0);
        game.buy_perk(0);
        game.buy_perk(40);
        // No eternities yet → nothing.
        assert_eq!(game.eternity_upgrades, 0);
        game.eternities = Decimal::ONE;
        game.tick_perk_effects();
        assert_eq!(game.eternity_upgrades & 0b111, 0b111);

        game.buy_perk(41);
        // Second row costs 1e16/1e40/1e50 EP; EU2 buys at 1e10× less.
        game.eternity_points = Decimal::new(1.0, 30);
        game.tick_perk_effects();
        assert_eq!(game.eternity_upgrades & 0b111111, 0b011111);
        game.eternity_points = Decimal::new(1.0, 45);
        game.tick_perk_effects();
        assert_eq!(game.eternity_upgrades & 0b111111, 0b111111);
    }

    #[test]
    fn achnr_unlocks_pre_reality_achievement_rows() {
        let mut game = game_with_pp(10.0);
        for id in [0u8, 201, 202, 203, 204] {
            assert!(game.buy_perk(id), "chain to ACHNR broke at {id}");
        }
        assert!(game.buy_perk(205));
        assert!(game.pre_reality_achievements_complete());
        // Reality no longer locks them.
        game.eternity_points = Decimal::new(1.0, 4000);
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        game.dilation.studies = vec![1, 2, 3, 4, 5, 6];
        assert!(game.reality());
        assert!(game.pre_reality_achievements_complete());
    }

    #[test]
    fn achievement_perks_shorten_the_auto_achievement_period() {
        let mut game = game_with_pp(10.0);
        assert_eq!(game.achievement_period_ms(), 30.0 * 60_000.0);
        game.buy_perk(0);
        game.buy_perk(201);
        assert_eq!(game.achievement_period_ms(), 20.0 * 60_000.0);
        game.buy_perk(202);
        game.buy_perk(203);
        game.buy_perk(204);
        assert_eq!(game.achievement_period_ms(), 2.0 * 60_000.0);
    }

    #[test]
    fn perk_points_round_trip_through_reality() {
        // Each Reality grants 1 PP.
        let mut game = GameState::new();
        game.eternity_points = Decimal::new(1.0, 4000);
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        game.dilation.studies = vec![1, 2, 3, 4, 5, 6];
        assert!(game.reality());
        assert_eq!(game.reality.perk_points, 1.0);
        assert!(game.buy_perk(0));
        assert_eq!(game.reality.perk_points, 0.0);
        // Perks persist across a Reality.
        assert!(game.perk_bought(0));
    }
}
