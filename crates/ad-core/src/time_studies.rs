//! Time Studies (Feature 4.4): the tree of ~60 pre-dilation studies bought with
//! **Time Theorems** (TT), themselves bought with AM / IP / EP.
//!
//! Mirrors `src/core/time-theorems.js`, `src/core/time-studies/*`, and the
//! study catalogue in `secret-formula/eternity/time-studies/normal-time-studies
//! .js`. Structural rules ported: prerequisite chains (`AT_LEAST_ONE` / `ALL`),
//! the **Dimension split** (71/72/73 — one path, two with TS201), the mutually
//! exclusive **Pace** columns (121/131/141 vs 122/132/142 vs 123/133/143) and
//! **Light/Dark** pairs (221–234) — pre-Space-Theorems only one side of each
//! set is purchasable (`requiresST` blocks the other). Triad studies (301+)
//! are Ra content and out of frontier.
//!
//! Study *effects* are applied at the original's sites across the engine
//! (dimension/ID/TD multipliers, IP/EP formulas, replicanti, galaxy costs,
//! sacrifice, free tickspeed); each site names its study. See
//! `design-docs/2026-07-04-eternity.md` §4.

use break_infinity::Decimal;

use crate::state::GameState;

/// Requirement kinds (`TS_REQUIREMENT_TYPE`).
#[derive(Debug, Clone, Copy)]
pub enum TsRequirement {
    /// Any one of the listed studies.
    AtLeastOne(&'static [u16]),
    /// All of the listed studies (plus any special condition, see
    /// [`GameState::ts_special_requirement`]).
    All(&'static [u16]),
    /// All of the listed studies **and** a free Dimension-split path slot.
    DimensionPath(&'static [u16]),
}

/// One study: id, TT cost, structural requirement, and the studies whose
/// ownership locks this one behind Space Theorems (pre-Ra: locks it outright).
#[derive(Debug, Clone, Copy)]
pub struct TimeStudyDef {
    pub id: u16,
    pub cost: f64,
    pub requirement: TsRequirement,
    pub requires_st: &'static [u16],
}

const fn ts(
    id: u16,
    cost: f64,
    requirement: TsRequirement,
    requires_st: &'static [u16],
) -> TimeStudyDef {
    TimeStudyDef {
        id,
        cost,
        requirement,
        requires_st,
    }
}

use TsRequirement::{All, AtLeastOne, DimensionPath};

/// The pre-dilation study catalogue (triads excluded).
pub const TIME_STUDIES: [TimeStudyDef; 58] = [
    ts(11, 1.0, All(&[]), &[]),
    ts(21, 3.0, AtLeastOne(&[11]), &[]),
    ts(22, 2.0, AtLeastOne(&[11]), &[]),
    ts(31, 3.0, AtLeastOne(&[21]), &[]),
    ts(32, 2.0, AtLeastOne(&[22]), &[]),
    ts(33, 2.0, AtLeastOne(&[22]), &[]),
    ts(41, 4.0, AtLeastOne(&[31]), &[]),
    ts(42, 6.0, AtLeastOne(&[32]), &[]),
    ts(51, 3.0, AtLeastOne(&[41, 42]), &[]),
    ts(61, 3.0, AtLeastOne(&[51]), &[]),
    // 62 additionally needs an EC5 completion (special requirement).
    ts(62, 3.0, All(&[42]), &[]),
    // The Dimension split; 71/72/73 also carry EC-study exclusions (special).
    ts(71, 4.0, DimensionPath(&[61]), &[]),
    ts(72, 6.0, DimensionPath(&[61]), &[]),
    ts(73, 5.0, DimensionPath(&[61]), &[]),
    ts(81, 4.0, AtLeastOne(&[71]), &[]),
    ts(82, 6.0, AtLeastOne(&[72]), &[]),
    ts(83, 5.0, AtLeastOne(&[73]), &[]),
    ts(91, 4.0, AtLeastOne(&[81]), &[]),
    ts(92, 5.0, AtLeastOne(&[82]), &[]),
    ts(93, 7.0, AtLeastOne(&[83]), &[]),
    ts(101, 4.0, AtLeastOne(&[91]), &[]),
    ts(102, 6.0, AtLeastOne(&[92]), &[]),
    ts(103, 6.0, AtLeastOne(&[93]), &[]),
    ts(111, 12.0, AtLeastOne(&[101, 102, 103]), &[]),
    // The Pace split (Active / Passive / Idle).
    ts(121, 9.0, AtLeastOne(&[111]), &[122, 123]),
    ts(122, 9.0, AtLeastOne(&[111]), &[121, 123]),
    ts(123, 9.0, AtLeastOne(&[111]), &[121, 122]),
    ts(131, 5.0, AtLeastOne(&[121]), &[132, 133]),
    ts(132, 5.0, AtLeastOne(&[122]), &[131, 133]),
    ts(133, 5.0, AtLeastOne(&[123]), &[131, 132]),
    ts(141, 4.0, AtLeastOne(&[131]), &[142, 143]),
    ts(142, 4.0, AtLeastOne(&[132]), &[141, 143]),
    ts(143, 4.0, AtLeastOne(&[133]), &[141, 142]),
    ts(151, 8.0, AtLeastOne(&[141, 142, 143]), &[]),
    ts(161, 7.0, AtLeastOne(&[151]), &[]),
    ts(162, 7.0, AtLeastOne(&[151]), &[]),
    ts(171, 15.0, AtLeastOne(&[161, 162]), &[]),
    // 181 additionally needs EC1–3 completions; 191–193 an EC10 completion.
    ts(181, 200.0, All(&[171]), &[]),
    ts(191, 400.0, All(&[181]), &[]),
    ts(192, 730.0, All(&[181]), &[]),
    ts(193, 300.0, All(&[181]), &[]),
    ts(201, 900.0, AtLeastOne(&[192]), &[]),
    ts(211, 120.0, AtLeastOne(&[191]), &[]),
    ts(212, 150.0, AtLeastOne(&[191]), &[]),
    ts(213, 200.0, AtLeastOne(&[193]), &[]),
    ts(214, 120.0, AtLeastOne(&[193]), &[]),
    // Light/Dark pairs.
    ts(221, 900.0, AtLeastOne(&[211]), &[222]),
    ts(222, 900.0, AtLeastOne(&[211]), &[221]),
    ts(223, 900.0, AtLeastOne(&[212]), &[224]),
    ts(224, 900.0, AtLeastOne(&[212]), &[223]),
    ts(225, 900.0, AtLeastOne(&[213]), &[226]),
    ts(226, 900.0, AtLeastOne(&[213]), &[225]),
    ts(227, 900.0, AtLeastOne(&[214]), &[228]),
    ts(228, 900.0, AtLeastOne(&[214]), &[227]),
    ts(231, 500.0, AtLeastOne(&[221, 222]), &[232]),
    ts(232, 500.0, AtLeastOne(&[223, 224]), &[231]),
    ts(233, 500.0, AtLeastOne(&[225, 226]), &[234]),
    ts(234, 500.0, AtLeastOne(&[227, 228]), &[233]),
];

/// Look up a study definition by id.
pub fn time_study_def(id: u16) -> Option<&'static TimeStudyDef> {
    TIME_STUDIES.iter().find(|d| d.id == id)
}

/// Time-Theorem purchase costs: AM `1e20000 × 1e20000^n`, IP `1 × 1e100^n`,
/// EP `1 × 2^n` (`TimeTheoremPurchaseType`).
const TT_AM_COST_EXP: f64 = 20_000.0;
const TT_IP_COST_EXP: f64 = 100.0;
const TT_EP_COST_MULT: f64 = 2.0;

impl GameState {
    // --- Time Theorems -------------------------------------------------------

    /// Whether TT can be purchased at all (`TimeTheorems.checkForBuying`,
    /// pre-Reality): at least one 1st Time Dimension bought.
    pub fn can_buy_time_theorems(&self) -> bool {
        self.time_dimensions[0].bought > 0
    }

    /// Cost of the next AM-bought Time Theorem.
    pub fn tt_cost_am(&self) -> Decimal {
        Decimal::pow10(TT_AM_COST_EXP * (self.tt_am_bought as f64 + 1.0))
    }

    /// Cost of the next IP-bought Time Theorem.
    pub fn tt_cost_ip(&self) -> Decimal {
        Decimal::pow10(TT_IP_COST_EXP * self.tt_ip_bought as f64)
    }

    /// Cost of the next EP-bought Time Theorem.
    pub fn tt_cost_ep(&self) -> Decimal {
        Decimal::from_float(TT_EP_COST_MULT)
            .pow(&Decimal::from(self.tt_ep_bought as u64))
    }

    /// Grant `count` Time Theorems (updates the all-time max).
    fn add_time_theorems(&mut self, count: f64) {
        self.time_theorems += Decimal::from_float(count);
        self.max_theorem = self.max_theorem.max(&self.time_theorems);
    }

    /// Buy one Time Theorem with antimatter. Returns whether it happened.
    pub fn buy_tt_with_am(&mut self) -> bool {
        let cost = self.tt_cost_am();
        if !self.can_buy_time_theorems() || self.antimatter < cost {
            return false;
        }
        self.antimatter -= cost;
        self.tt_am_bought += 1;
        self.add_time_theorems(1.0);
        true
    }

    /// Buy one Time Theorem with Infinity Points.
    pub fn buy_tt_with_ip(&mut self) -> bool {
        let cost = self.tt_cost_ip();
        if !self.can_buy_time_theorems() || self.infinity_points < cost {
            return false;
        }
        self.infinity_points -= cost;
        self.tt_ip_bought += 1;
        self.add_time_theorems(1.0);
        true
    }

    /// Buy one Time Theorem with Eternity Points.
    pub fn buy_tt_with_ep(&mut self) -> bool {
        let cost = self.tt_cost_ep();
        if !self.can_buy_time_theorems() || self.eternity_points < cost {
            return false;
        }
        self.eternity_points -= cost;
        self.tt_ep_bought += 1;
        self.add_time_theorems(1.0);
        true
    }

    /// Buy as many Time Theorems as affordable across all three currencies
    /// (`TimeTheorems.buyMax`, via repeated singles — the counts stay small
    /// because each cost curve is steeply geometric). Returns the number bought.
    pub fn buy_max_time_theorems(&mut self) -> u64 {
        let mut count = 0;
        while self.buy_tt_with_am() {
            count += 1;
        }
        while self.buy_tt_with_ip() {
            count += 1;
        }
        while self.buy_tt_with_ep() {
            count += 1;
        }
        count
    }

    /// Total TT ever purchased (`TimeTheorems.totalPurchased`), gating dilation
    /// later.
    pub fn tt_total_purchased(&self) -> u32 {
        self.tt_am_bought + self.tt_ip_bought + self.tt_ep_bought
    }

    // --- Studies -------------------------------------------------------------

    /// Whether study `id` is bought.
    pub fn time_study_bought(&self, id: u16) -> bool {
        self.studies.contains(&id)
    }

    /// Number of Dimension-split paths entered (studies 71/72/73 bought).
    fn dim_path_count(&self) -> usize {
        [71u16, 72, 73]
            .iter()
            .filter(|&&id| self.time_study_bought(id))
            .count()
    }

    /// Allowed Dimension-split paths: 1; 2 with TS201; all 3 with the
    /// `timeStudySplit` Dilation Upgrade.
    fn allowed_dim_path_count(&self) -> usize {
        if self.dilation_upgrade_bought(8) {
            3
        } else if self.time_study_bought(201) {
            2
        } else {
            1
        }
    }

    /// The non-structural conditions bundled into some studies' requirements
    /// (the closures in `normal-time-studies.js`).
    fn ts_special_requirement(&self, id: u16) -> bool {
        match id {
            // EC5 completion unlocks 62.
            62 => self.eternity_challenge_completions(5) > 0,
            // The dimension paths are excluded by holding the EC11/EC12 study
            // (their `forbiddenStudies` counterpart).
            71 => self.eternity_challenge_unlocked != 12,
            72 => {
                self.eternity_challenge_unlocked != 11
                    && self.eternity_challenge_unlocked != 12
            }
            73 => self.eternity_challenge_unlocked != 11,
            // 181 needs EC1–3 completed at least once.
            181 => (1..=3).all(|ec| self.eternity_challenge_completions(ec) > 0),
            // 191/192/193 need an EC10 completion.
            191..=193 => self.eternity_challenge_completions(10) > 0,
            _ => true,
        }
    }

    /// Whether study `id` can be bought right now: exists, not bought, enough
    /// TT, structural requirement met, special condition met, and not locked by
    /// an owned mutually-exclusive study (`requiresST` pre-Ra).
    pub fn can_buy_time_study(&self, id: u16) -> bool {
        let Some(def) = time_study_def(id) else {
            return false;
        };
        if self.time_study_bought(id)
            || self.time_theorems < Decimal::from_float(def.cost)
        {
            return false;
        }
        let check_all = |ids: &[u16]| ids.iter().all(|&r| self.time_study_bought(r));
        let structural = match def.requirement {
            AtLeastOne(ids) => ids.iter().any(|&r| self.time_study_bought(r)),
            All(ids) => check_all(ids),
            DimensionPath(ids) => {
                check_all(ids) && self.dim_path_count() < self.allowed_dim_path_count()
            }
        };
        let set_free = !def.requires_st.iter().any(|&s| self.time_study_bought(s));
        structural && self.ts_special_requirement(id) && set_free
    }

    /// Buy study `id`. Returns whether it happened.
    pub fn buy_time_study(&mut self, id: u16) -> bool {
        if !self.can_buy_time_study(id) {
            return false;
        }
        let def = time_study_def(id).expect("checked in can_buy");
        self.time_theorems -= Decimal::from_float(def.cost);
        self.studies.push(id);
        true
    }

    /// Respec: refund every bought study (and the held EC study slot) and clear
    /// the tree (`respecTimeStudies`).
    pub fn respec_time_studies_now(&mut self) {
        let mut refund = 0.0;
        for &id in &self.studies {
            if let Some(def) = time_study_def(id) {
                refund += def.cost;
            }
        }
        self.studies.clear();
        self.add_time_theorems(refund);
        if self.eternity_challenge_unlocked != 0 {
            let cost = crate::eternity_challenges::ec_study_cost(
                self.eternity_challenge_unlocked,
            );
            self.add_time_theorems(cost);
            self.eternity_challenge_unlocked = 0;
        }
    }

    /// Toggle the "respec on next Eternity" flag (`player.respec`).
    pub fn set_respec(&mut self, respec: bool) {
        self.respec = respec;
    }

    // --- Effect helpers used by the formula sites ----------------------------

    /// `Currency.infinitiesTotal`: infinities this eternity plus banked
    /// infinities (TS191).
    pub fn infinities_total(&self) -> Decimal {
        self.infinities + self.infinities_banked
    }

    /// The "this infinity time" decaying/growing multiplier used by TS141/143:
    /// `15^(ln(t·10+1) × min((t·10+1)^0.125, 500))` for `t` in seconds.
    pub(crate) fn this_infinity_mult(seconds: f64) -> Decimal {
        let scaled = seconds * 10.0 + 1.0;
        let capped = scaled.powf(0.125).min(500.0);
        Decimal::from_float(15.0).pow(&Decimal::from_float(scaled.ln() * capped))
    }

    /// TS121's Active-path EP multiplier: `clamp(250 / avg real seconds per
    /// eternity (last 10), 1, 50)`.
    pub(crate) fn ts121_effect(&self) -> f64 {
        let avg_secs = self
            .records
            .recent_eternities
            .iter()
            .map(|r| r.real_time_ms)
            .sum::<f64>()
            / (1000.0 * self.records.recent_eternities.len() as f64);
        (250.0 / avg_secs).clamp(1.0, 50.0)
    }

    /// TS11's Time-Dimension-1 multiplier: the reciprocal of
    /// `(tick/1000)^0.005 × 0.95 + (tick/1000)^0.0003 × 0.05`, capped at 1e2500.
    pub(crate) fn ts11_effect(&self) -> Decimal {
        let tickspeed = self.current_tickspeed_ms() / Decimal::from_float(1000.0);
        let first =
            tickspeed.pow(&Decimal::from_float(0.005)) * Decimal::from_float(0.95);
        let second =
            tickspeed.pow(&Decimal::from_float(0.0003)) * Decimal::from_float(0.05);
        (Decimal::ONE / (first + second)).min(&Decimal::new_unchecked(1.0, 2500))
    }

    /// The passive-IP time study (TS181): gain 1% of the pending crunch IP per
    /// second. Applied each tick (game.js `Currency.infinityPoints.add(
    /// TimeStudy(181).effectOrDefault(0))`).
    pub(crate) fn generate_ts181_ip(&mut self, dt_ms: f64) {
        if !self.time_study_bought(181) {
            return;
        }
        let gain =
            self.gained_infinity_points() * Decimal::from_float(dt_ms / 1000.0 / 100.0);
        self.infinity_points += gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with_tt(tt: f64) -> GameState {
        let mut game = GameState::new();
        game.time_theorems = Decimal::from_float(tt);
        game
    }

    #[test]
    fn tt_costs_follow_original_curves() {
        let mut game = GameState::new();
        assert_eq!(game.tt_cost_am(), Decimal::new(1.0, 20000));
        assert_eq!(game.tt_cost_ip(), Decimal::ONE);
        assert_eq!(game.tt_cost_ep(), Decimal::ONE);
        game.tt_am_bought = 1;
        game.tt_ip_bought = 2;
        game.tt_ep_bought = 3;
        assert_eq!(game.tt_cost_am(), Decimal::new(1.0, 40000));
        assert_eq!(game.tt_cost_ip(), Decimal::new(1.0, 200));
        assert_eq!(game.tt_cost_ep(), Decimal::from_float(8.0));
    }

    #[test]
    fn tt_purchase_requires_a_time_dimension() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::new(1.0, 10);
        assert!(!game.buy_tt_with_ip());
        game.time_dimensions[0].bought = 1;
        assert!(game.buy_tt_with_ip());
        assert_eq!(game.time_theorems, Decimal::ONE);
        assert_eq!(game.max_theorem, Decimal::ONE);
    }

    #[test]
    fn first_study_needs_no_requirements() {
        let mut game = game_with_tt(10.0);
        assert!(game.can_buy_time_study(11));
        assert!(game.buy_time_study(11));
        assert_eq!(game.time_theorems, Decimal::from_float(9.0));
        assert!(game.time_study_bought(11));
        // Cannot re-buy.
        assert!(!game.buy_time_study(11));
    }

    #[test]
    fn requirements_gate_purchases() {
        let mut game = game_with_tt(100.0);
        // 21 needs 11.
        assert!(!game.can_buy_time_study(21));
        game.buy_time_study(11);
        assert!(game.buy_time_study(21));
        // 51 needs 41 or 42.
        assert!(!game.can_buy_time_study(51));
        game.buy_time_study(31); // needs 21 ✓
        game.buy_time_study(41); // needs 31 ✓
        assert!(game.buy_time_study(51));
    }

    #[test]
    fn dimension_split_allows_one_path() {
        let mut game = game_with_tt(1000.0);
        for id in [11, 22, 32, 42, 51, 61] {
            // 51 needs 41/42; buy 42's chain (22→32→42).
            if game.can_buy_time_study(id) {
                game.buy_time_study(id);
            }
        }
        assert!(game.time_study_bought(61));
        assert!(game.buy_time_study(71));
        // A second path is locked without TS201.
        assert!(!game.can_buy_time_study(72));
        assert!(!game.can_buy_time_study(73));
        // But continuing the chosen path works.
        assert!(game.buy_time_study(81));
    }

    #[test]
    fn pace_split_is_mutually_exclusive() {
        let mut game = game_with_tt(10_000.0);
        game.studies = vec![11, 22, 32, 42, 51, 61, 71, 81, 91, 101, 111];
        assert!(game.buy_time_study(121));
        // The other pace columns are locked by ownership of 121.
        assert!(!game.can_buy_time_study(122));
        assert!(!game.can_buy_time_study(123));
        assert!(game.buy_time_study(131));
        assert!(!game.can_buy_time_study(132));
        assert!(game.buy_time_study(141));
    }

    #[test]
    fn light_dark_pairs_are_exclusive() {
        let mut game = game_with_tt(100_000.0);
        game.studies = vec![
            11, 22, 32, 42, 51, 61, 71, 81, 91, 101, 111, 121, 131, 141, 151, 161, 171,
            181, 191, 211,
        ];
        // EC-gated specials: fake an EC10 completion for 191's descendants.
        game.eternity_challenges[9] = 1;
        assert!(game.buy_time_study(221));
        assert!(!game.can_buy_time_study(222));
        assert!(game.buy_time_study(231));
        assert!(!game.can_buy_time_study(232));
    }

    #[test]
    fn ec_gated_studies_need_completions() {
        let mut game = game_with_tt(10_000.0);
        game.studies = vec![
            11, 22, 32, 42, 51, 61, 71, 81, 91, 101, 111, 121, 131, 141, 151, 161, 171,
        ];
        // 181 needs EC1–3 completions.
        assert!(!game.can_buy_time_study(181));
        game.eternity_challenges[0] = 1;
        game.eternity_challenges[1] = 1;
        game.eternity_challenges[2] = 1;
        assert!(game.buy_time_study(181));
        // 191 needs EC10.
        assert!(!game.can_buy_time_study(191));
        game.eternity_challenges[9] = 1;
        assert!(game.buy_time_study(191));
    }

    #[test]
    fn ts201_allows_second_dimension_path() {
        let mut game = game_with_tt(100_000.0);
        game.studies = vec![
            11, 22, 32, 42, 51, 61, 72, 82, 92, 102, 111, 123, 133, 143, 151, 161, 171,
            181, 192,
        ];
        game.eternity_challenges[9] = 1; // EC10 for 192's special (already owned)
        assert!(!game.can_buy_time_study(71));
        assert!(game.buy_time_study(201));
        assert!(game.buy_time_study(71));
        // Not a third.
        assert!(!game.can_buy_time_study(73));
    }

    #[test]
    fn respec_refunds_everything() {
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        game.buy_time_study(21);
        game.buy_time_study(22);
        assert_eq!(game.time_theorems, Decimal::from_float(4.0));
        game.respec_time_studies_now();
        assert!(game.studies.is_empty());
        assert_eq!(game.time_theorems, Decimal::from_float(10.0));
    }

    #[test]
    fn respec_on_eternity_when_flagged() {
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        game.respec = true;
        assert!(game.eternity());
        assert!(game.studies.is_empty());
        assert!(!game.respec);
        assert_eq!(game.time_theorems, Decimal::from_float(10.0));
    }

    #[test]
    fn studies_persist_across_eternity_without_respec() {
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());
        assert!(game.time_study_bought(11));
    }
}
