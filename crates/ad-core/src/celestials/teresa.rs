//! Teresa (Feature 7.1) — the first, simplest celestial. Pour Reality Machines
//! into Teresa to raise a global RM-gain multiplier and cross unlock
//! thresholds; run Teresa's Reality (IP/EP gain `^0.55`) to raise a
//! glyph-sacrifice multiplier; spend Perk Points in the Perk Shop.
//!
//! See `docs/design/2026-07-06-celestials.md` §1. Original:
//! `celestials/teresa.js` + `secret-formula/celestials/{teresa,perk-shop}.js`.

use crate::state::GameState;
use break_infinity::Decimal;

/// `Teresa.pouredAmountCap` — the RM pool caps at `1e24`.
pub const POURED_AMOUNT_CAP: f64 = 1e24;

/// The Teresa unlocks (`GameDatabase.celestials.teresa.unlocks`), each an
/// automatic threshold on `pouredAmount` (not a manual purchase). `id` is the
/// save bit; `price` is the poured-RM threshold.
#[derive(Debug, Clone, Copy)]
pub struct TeresaUnlock {
    pub id: u8,
    pub price: f64,
}

pub const TERESA_UNLOCK_RUN: TeresaUnlock = TeresaUnlock { id: 0, price: 1e14 };
pub const TERESA_UNLOCK_EP_GEN: TeresaUnlock = TeresaUnlock { id: 1, price: 1e18 };
pub const TERESA_UNLOCK_SHOP: TeresaUnlock = TeresaUnlock { id: 2, price: 1e21 };
pub const TERESA_UNLOCK_EFFARIG: TeresaUnlock = TeresaUnlock { id: 3, price: 1e24 };
pub const TERESA_UNLOCK_UNDO: TeresaUnlock = TeresaUnlock { id: 4, price: 1e10 };
pub const TERESA_UNLOCK_START_EU: TeresaUnlock = TeresaUnlock { id: 5, price: 1e6 };

pub const TERESA_UNLOCKS: [TeresaUnlock; 6] = [
    TERESA_UNLOCK_RUN,
    TERESA_UNLOCK_EP_GEN,
    TERESA_UNLOCK_SHOP,
    TERESA_UNLOCK_EFFARIG,
    TERESA_UNLOCK_UNDO,
    TERESA_UNLOCK_START_EU,
];

/// A Perk-Shop rebuyable (`GameDatabase.celestials.perkShop`): cost
/// `initial · 2^bought`, effect `2^bought` (or `1.05^bought` for glyphLevel),
/// capped when `cost == cost_cap`. The non-Ra caps are used (Ra's
/// `perkShopIncrease` is out of frontier).
#[derive(Debug, Clone, Copy)]
pub struct PerkShopEntry {
    pub id: usize,
    pub initial_cost: f64,
    pub cost_cap: f64,
    pub increment: f64,
}

pub const PERK_SHOP_GLYPH_LEVEL: PerkShopEntry = PerkShopEntry {
    id: 0,
    initial_cost: 1.0,
    cost_cap: 2048.0,
    increment: 2.0,
};
pub const PERK_SHOP_RM_MULT: PerkShopEntry = PerkShopEntry {
    id: 1,
    initial_cost: 1.0,
    cost_cap: 2048.0,
    increment: 2.0,
};
pub const PERK_SHOP_BULK_DILATION: PerkShopEntry = PerkShopEntry {
    id: 2,
    initial_cost: 100.0,
    cost_cap: 1600.0,
    increment: 2.0,
};
pub const PERK_SHOP_AUTO_SPEED: PerkShopEntry = PerkShopEntry {
    id: 3,
    initial_cost: 1000.0,
    cost_cap: 4000.0,
    increment: 2.0,
};

/// The four modelled Perk-Shop rebuyables (music-glyph entries 4/5 are cut —
/// music glyphs are unmodelled).
pub const PERK_SHOP_ENTRIES: [PerkShopEntry; 4] = [
    PERK_SHOP_GLYPH_LEVEL,
    PERK_SHOP_RM_MULT,
    PERK_SHOP_BULK_DILATION,
    PERK_SHOP_AUTO_SPEED,
];

/// `player.celestials.teresa`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TeresaState {
    /// RM poured into Teresa (`pouredAmount`, capped at `1e24`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub poured_amount: f64,
    /// Time spent pouring (`timePoured`), drives the pour rate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub time_poured: f64,
    /// Unlock bits (`unlockBits`), one per [`TeresaUnlock`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    /// Whether Teresa's Reality is running (`run`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Best antimatter reached in a Teresa run (`bestRunAM`), drives the
    /// glyph-sacrifice reward.
    #[cfg_attr(
        feature = "serde",
        serde(default = "crate::state::default_decimal_one")
    )]
    pub best_run_am: Decimal,
    /// Perk-Shop purchase counts, ids 0–4 (`perkShop`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub perk_shop: [u32; 5],
}

impl Default for TeresaState {
    fn default() -> Self {
        Self::new()
    }
}

impl TeresaState {
    pub fn new() -> Self {
        Self {
            poured_amount: 0.0,
            time_poured: 0.0,
            unlock_bits: 0,
            run: false,
            best_run_am: Decimal::ONE,
            perk_shop: [0; 5],
        }
    }

    /// Whether unlock `id` is owned.
    pub fn unlock_bought(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    // --- Pouring ----------------------------------------------------------------

    /// `Teresa.pourRM(diff)`: pour RM into the pool while below the cap. `diff`
    /// is real time in ms; the original's `timePoured` (and hence the pour rate)
    /// is in **seconds**, so we convert.
    pub fn teresa_pour_rm(&mut self, diff_ms: f64) {
        if self.celestials.teresa.poured_amount >= POURED_AMOUNT_CAP {
            return;
        }
        self.celestials.teresa.time_poured += diff_ms / 1000.0;
        let rm = self.reality.machines.to_f64();
        let poured = self.celestials.teresa.poured_amount;
        let t = self.celestials.teresa.time_poured;
        // `rmPoured = min((poured + 1e6) · 0.01 · timePoured², RM)`.
        let rm_poured = ((poured + 1e6) * 0.01 * t * t).min(rm);
        let added = rm_poured.min(POURED_AMOUNT_CAP - poured);
        self.celestials.teresa.poured_amount += added;
        // The original subtracts the *uncapped* `rmPoured` (RM can be wasted
        // near the cap); Currency clamps at 0.
        self.reality.machines =
            (self.reality.machines - Decimal::from_float(rm_poured)).max(&Decimal::ZERO);
        self.teresa_check_unlocks();
    }

    /// Reset the pour-rate timer (the original zeroes `Teresa.timePoured` when
    /// the pour button is released, so each hold ramps up from scratch).
    pub fn teresa_stop_pouring(&mut self) {
        self.celestials.teresa.time_poured = 0.0;
    }

    /// `Teresa.checkForUnlocks()`: auto-unlock every threshold the pool now
    /// meets, applying each `onUnlock` side effect once.
    pub(crate) fn teresa_check_unlocks(&mut self) {
        for unlock in TERESA_UNLOCKS {
            if !self.celestials.teresa.unlock_bought(unlock.id)
                && self.celestials.teresa.poured_amount >= unlock.price
            {
                self.celestials.teresa.unlock_bits |= 1u32 << unlock.id;
                self.teresa_on_unlock(unlock.id);
            }
        }
    }

    fn teresa_on_unlock(&mut self, id: u8) {
        // startEU immediately grants all 6 Eternity Upgrades (and re-grants on
        // each Reality reset via `apply_teresa_start_eu`).
        if id == TERESA_UNLOCK_START_EU.id {
            self.apply_teresa_start_eu();
        }
    }

    /// Grant all 6 one-time Eternity Upgrades if `startEU` is unlocked
    /// (`REALITY_RESET_AFTER` hook — bits 1..=6).
    pub(crate) fn apply_teresa_start_eu(&mut self) {
        if self
            .celestials
            .teresa
            .unlock_bought(TERESA_UNLOCK_START_EU.id)
        {
            for id in 1..=6u32 {
                self.eternity_upgrades |= 1u32 << id;
            }
        }
    }

    // --- Multipliers / rewards --------------------------------------------------

    /// `Teresa.rmMultiplier`: the RM-gain multiplier from the poured pool,
    /// `max(250 · (poured/1e24)^0.1, 1)`.
    pub fn teresa_rm_multiplier(&self) -> f64 {
        (250.0 * (self.celestials.teresa.poured_amount / POURED_AMOUNT_CAP).powf(0.1))
            .max(1.0)
    }

    /// `MachineHandler.realityMachineMultiplier`: Teresa's pool multiplier times
    /// the Perk-Shop `rmMult` (celestial/Ra sources are out of frontier → 1).
    pub(crate) fn reality_machine_multiplier(&self) -> f64 {
        self.teresa_rm_multiplier() * self.perk_shop_effect(PERK_SHOP_RM_MULT).max(1.0)
    }

    /// `Teresa.rewardMultiplier(am)`: the glyph-sacrifice multiplier a run with
    /// `am` antimatter would grant, `max((log10(am+1)/1.5e8)^12, 1)`.
    pub fn teresa_reward_multiplier(&self, antimatter: Decimal) -> f64 {
        let base = (antimatter + Decimal::ONE).log10() / 1.5e8;
        base.powi(12).max(1.0)
    }

    /// `Teresa.runRewardMultiplier`: the live glyph-sacrifice multiplier from
    /// the best Teresa run.
    pub fn teresa_run_reward_multiplier(&self) -> f64 {
        self.teresa_reward_multiplier(self.celestials.teresa.best_run_am)
    }

    /// Progress of the pour bar (`Teresa.fill`): `log10(poured)/24`, clamped.
    pub fn teresa_fill(&self) -> f64 {
        (self.celestials.teresa.poured_amount.log10() / 24.0).clamp(0.0, 1.0)
    }

    // --- Unlock accessors -------------------------------------------------------

    /// Whether Teresa's Reality is unlocked (the `run` threshold).
    pub fn teresa_run_unlocked(&self) -> bool {
        self.celestials.teresa.unlock_bought(TERESA_UNLOCK_RUN.id)
    }

    /// Whether the Perk Shop is unlocked.
    pub fn teresa_shop_unlocked(&self) -> bool {
        self.celestials.teresa.unlock_bought(TERESA_UNLOCK_SHOP.id)
    }

    /// Whether Teresa's `effarig` threshold — which unlocks Effarig — is met.
    pub fn teresa_effarig_unlocked(&self) -> bool {
        self.celestials
            .teresa
            .unlock_bought(TERESA_UNLOCK_EFFARIG.id)
    }

    // --- Run completion ---------------------------------------------------------

    /// The Teresa completion hook from `giveRealityRewards`: record the best
    /// antimatter reached (which raises the glyph-sacrifice reward).
    pub(crate) fn teresa_complete_run(&mut self) {
        if self.antimatter > self.celestials.teresa.best_run_am {
            self.celestials.teresa.best_run_am = self.antimatter;
        }
    }

    // --- epGen ------------------------------------------------------------------

    /// `applyAutoprestige` EP term: with the `epGen` unlock, generate EP at
    /// `bestEPmin · 0.01 · gameSpeed · dt/1000`. `dt_ms` is the game-time
    /// interval (already speed-scaled by the tick loop, matching the original's
    /// `getGameSpeedupFactor() · diff`).
    pub(crate) fn generate_teresa_ep(&mut self, dt_ms: f64) {
        if !self
            .celestials
            .teresa
            .unlock_bought(TERESA_UNLOCK_EP_GEN.id)
        {
            return;
        }
        let gain = self.records.this_eternity.best_ep_min
            * Decimal::from_float(0.01 * dt_ms / 1000.0);
        self.eternity_points += gain;
    }

    // --- Perk Shop --------------------------------------------------------------

    /// Purchase count of a Perk-Shop entry.
    pub fn perk_shop_bought(&self, entry: PerkShopEntry) -> u32 {
        self.celestials.teresa.perk_shop[entry.id]
    }

    /// The current cost of a Perk-Shop entry (`initial · 2^bought`).
    pub fn perk_shop_cost(&self, entry: PerkShopEntry) -> f64 {
        entry.initial_cost * entry.increment.powi(self.perk_shop_bought(entry) as i32)
    }

    /// The `autoSpeed` Perk-Shop effect: ×2 per purchase (divides the
    /// milestone-autobuyer intervals).
    pub fn perk_shop_auto_speed_effect(&self) -> f64 {
        2f64.powi(self.perk_shop_bought(PERK_SHOP_AUTO_SPEED) as i32)
    }

    /// Whether a Perk-Shop entry is capped (`cost == cost_cap`).
    pub fn perk_shop_capped(&self, entry: PerkShopEntry) -> bool {
        self.perk_shop_cost(entry) >= entry.cost_cap
    }

    /// The effect value of a Perk-Shop entry. `glyphLevel` is `1.05^bought`;
    /// the rest are `2^bought`.
    pub fn perk_shop_effect(&self, entry: PerkShopEntry) -> f64 {
        let bought = self.perk_shop_bought(entry) as i32;
        if entry.id == PERK_SHOP_GLYPH_LEVEL.id {
            1.05f64.powi(bought)
        } else {
            2.0f64.powi(bought)
        }
    }

    /// Whether a Perk-Shop entry can be bought now (unlocked, uncapped, and
    /// affordable in Perk Points).
    pub fn perk_shop_can_buy(&self, entry: PerkShopEntry) -> bool {
        self.teresa_shop_unlocked()
            && !self.perk_shop_capped(entry)
            && self.reality.perk_points >= self.perk_shop_cost(entry)
    }

    /// Buy one level of a Perk-Shop entry. Returns whether it happened.
    pub fn buy_perk_shop(&mut self, entry: PerkShopEntry) -> bool {
        if !self.perk_shop_can_buy(entry) {
            return false;
        }
        self.reality.perk_points -= self.perk_shop_cost(entry);
        self.celestials.teresa.perk_shop[entry.id] += 1;
        // Perk-Shop id 1 (`rmMult`) bumps the Reality autobuyer amount in the
        // original; that autobuyer knob is out of frontier.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn realitied_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        game
    }

    #[test]
    fn pouring_raises_pool_and_crosses_thresholds() {
        let mut game = realitied_game();
        game.reality.machines = Decimal::from_float(1e12);
        // Pour for a long time; the pool should rise and cross the startEU/undo
        // thresholds (1e6 / 1e10).
        for _ in 0..100 {
            game.teresa_pour_rm(1000.0);
        }
        assert!(game.celestials.teresa.poured_amount > 1e6);
        assert!(game
            .celestials
            .teresa
            .unlock_bought(TERESA_UNLOCK_START_EU.id));
        // startEU granted all 6 Eternity Upgrades.
        for id in 1..=6u32 {
            assert!(game.eternity_upgrades & (1u32 << id) != 0);
        }
    }

    #[test]
    fn rm_multiplier_at_cap_is_250() {
        let mut game = realitied_game();
        game.celestials.teresa.poured_amount = POURED_AMOUNT_CAP;
        assert!((game.teresa_rm_multiplier() - 250.0).abs() < 1e-6);
    }

    #[test]
    fn reward_multiplier_grows_with_antimatter() {
        let game = realitied_game();
        // Below the 1.5e8-log threshold the reward is clamped to 1.
        assert_eq!(
            game.teresa_reward_multiplier(Decimal::from_float(1e100)),
            1.0
        );
        // A huge antimatter run beats 1.
        let big = Decimal::new(1.0, 300_000_000);
        assert!(game.teresa_reward_multiplier(big) > 1.0);
    }

    #[test]
    fn perk_shop_caps_and_effects() {
        let mut game = realitied_game();
        game.celestials.teresa.unlock_bits |= 1u32 << TERESA_UNLOCK_SHOP.id;
        game.reality.perk_points = 1e9;
        // rmMult: 11 buys reach cost 2048 = cap.
        for _ in 0..20 {
            game.buy_perk_shop(PERK_SHOP_RM_MULT);
        }
        assert_eq!(game.perk_shop_bought(PERK_SHOP_RM_MULT), 11);
        assert!((game.perk_shop_effect(PERK_SHOP_RM_MULT) - 2048.0).abs() < 1e-6);
        assert!(game.perk_shop_capped(PERK_SHOP_RM_MULT));
    }
}
