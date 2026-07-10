//! Enslaved — The Nameless Ones (Feature 7.3) — game-time storage/release and a
//! restrictive Reality. See `docs/design/2026-07-06-celestials.md` §3. Original:
//! `celestials/enslaved.js`.
//!
//! **Scope.** Game-time storage (bank the Black-Hole boost) + release (a single
//! burst), the two unlocks (softcap, run — with the level-5000/rarity-100 glyph
//! gate), and the full set of run restrictions (glyph-level minimum, always-
//! dilated AD, 8th-AD/ID/TD purchase limits, uncapped-Replicanti lock, disabled
//! Black Hole, TP/DT nerfs, the discharge nerf, EC1 goal 1000) are ported, as
//! are **real-time storage + Reality amplification** (`boostReality`), the Ra
//! auto-release / offline auto-store, and **Tesseracts** (the ID-cap currency
//! plus its milestone/IU23 hooks). The hints/progress/`feelEternity`/
//! secret-study flavor stays deferred (see the design doc).

use break_infinity::Decimal;

use crate::state::GameState;

/// `Enslaved.glyphLevelMin` — inside the run glyph levels are boosted to at
/// least this (`getAdjustedGlyphLevel`).
pub const GLYPH_LEVEL_MIN: u32 = 5000;

/// Unlock ids (`ENSLAVED_UNLOCKS`) — bought with stored game time.
pub const ENSLAVED_UNLOCK_SOFTCAP: u8 = 0;
pub const ENSLAVED_UNLOCK_RUN: u8 = 1;

/// Milliseconds per year (`TimeSpan.fromYears`: `value × 31536e6`).
const MS_PER_YEAR: f64 = 31_536e6;
/// Softcap unlock price: `TimeSpan.fromYears(1e35)`.
pub const SOFTCAP_UNLOCK_PRICE_MS: f64 = 1e35 * MS_PER_YEAR;
/// Run unlock price: `TimeSpan.fromYears(1e40)`.
pub const RUN_UNLOCK_PRICE_MS: f64 = 1e40 * MS_PER_YEAR;

/// `Enslaved.timeCap` — the stored-game-time cap.
pub const TIME_CAP_MS: f64 = 1e300;
/// `storedRealTimeEfficiency` — the fraction of real time banked.
pub const STORED_REAL_EFFICIENCY: f64 = 0.7;
/// The base stored-real-time cap: 8 hours (Ra's `improvedStoredTime` adds
/// 1 hour per Nameless level).
pub const STORED_REAL_BASE_CAP_MS: f64 = 1000.0 * 3600.0 * 8.0;
/// `Enslaved.tachyonNerf` — the TP/DT nerf exponent inside the run.
pub const TACHYON_NERF: f64 = 0.3;

/// `Tesseracts.BASE_COSTS`: hardcoded log10-cost bases (the IP cost is
/// `10^(1e7 × base)`) — 2, 4, 6 then successive ×2, ×4, ×6, …
pub const TESSERACT_BASE_COSTS: [f64; 12] = [
    2.0,
    4.0,
    6.0,
    12.0,
    48.0,
    288.0,
    2304.0,
    23040.0,
    276480.0,
    3870720.0,
    61931520.0,
    1114767360.0,
];

/// `player.celestials.enslaved`.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EnslavedState {
    /// Whether game-time storage is active (`isStoring`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_storing: bool,
    /// Stored game time in ms (`stored`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub stored: f64,
    /// Whether real-time storage is active (`isStoringReal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_storing_real: bool,
    /// Stored real time in ms (`storedReal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub stored_real: f64,
    /// Whether offline time is banked into `storedReal` (`autoStoreReal`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub auto_store_real: bool,
    /// Whether the Ra auto-release is on (`isAutoReleasing`): every 5th tick
    /// discharges 1% of the stored game time.
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_auto_releasing: bool,
    /// The auto-release 5-tick counter (`autoReleaseTick`). Runtime-only.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub auto_release_tick: u8,
    /// Whether the next Reality is amplified by stored real time
    /// (`Enslaved.boostReality`). A module flag in the original — deliberately
    /// **not** part of the save; cleared on load like a reload clears it there.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub boost_reality: bool,
    /// Unlock bits (ids 0/1, packed from the original's `unlocks` array).
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    /// Whether Enslaved's Reality is running (`run`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Whether Enslaved's Reality has been completed (`completed`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub completed: bool,
    /// Tesseracts bought (`tesseracts`): the ID-purchase-cap currency, bought
    /// with (unspent) Infinity Points once the run is completed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tesseracts: u32,
    /// A pending game-time release burst (ms), consumed by the next tick.
    /// Runtime-only (not part of the save).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub release_ms: f64,
}

impl EnslavedState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether unlock `id` is owned.
    pub fn unlock_bought(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// `Enslaved.isUnlocked`: available once Effarig's Eternity stage is done.
    pub fn enslaved_unlocked(&self) -> bool {
        self.celestials
            .effarig
            .unlock_bought(crate::celestials::effarig::EFFARIG_UNLOCK_ETERNITY)
    }

    /// Whether Enslaved's Reality is unlocked (the `RUN` unlock, id 1).
    pub fn enslaved_run_unlocked(&self) -> bool {
        self.celestials.enslaved.unlock_bought(ENSLAVED_UNLOCK_RUN)
    }

    /// Whether the free-tickspeed-softcap unlock is owned.
    pub fn enslaved_softcap_unlocked(&self) -> bool {
        self.celestials
            .enslaved
            .unlock_bought(ENSLAVED_UNLOCK_SOFTCAP)
    }

    /// The `RUN` unlock's secondary requirement: a best-reality glyph of level
    /// ≥ 5000 and rarity ≥ 100.
    pub fn enslaved_run_requirement_met(&self) -> bool {
        self.records.best_reality.glyph_level >= GLYPH_LEVEL_MIN
            && crate::glyphs::strength_to_rarity(
                self.records.best_reality.glyph_strength,
            ) >= 100.0
    }

    /// The price (stored game ms) of an unlock.
    pub fn enslaved_unlock_price(id: u8) -> f64 {
        match id {
            ENSLAVED_UNLOCK_SOFTCAP => SOFTCAP_UNLOCK_PRICE_MS,
            _ => RUN_UNLOCK_PRICE_MS,
        }
    }

    /// `Enslaved.canBuy`: enough stored time, the secondary requirement met, and
    /// not already owned.
    pub fn can_buy_enslaved_unlock(&self, id: u8) -> bool {
        if self.celestials.enslaved.unlock_bought(id) {
            return false;
        }
        let secondary = id != ENSLAVED_UNLOCK_RUN || self.enslaved_run_requirement_met();
        self.celestials.enslaved.stored >= Self::enslaved_unlock_price(id) && secondary
    }

    /// `Enslaved.buyUnlock`: spend stored time on an unlock. Returns success.
    pub fn buy_enslaved_unlock(&mut self, id: u8) -> bool {
        if !self.can_buy_enslaved_unlock(id) {
            return false;
        }
        self.celestials.enslaved.stored -= Self::enslaved_unlock_price(id);
        self.celestials.enslaved.unlock_bits |= 1u32 << id;
        true
    }

    // --- Time storage / release -------------------------------------------------

    /// `Enslaved.canModifyGameTimeStorage`: unlocked, not paused, not in EC12,
    /// and not inside Enslaved's own Reality (Pelle/Lai'tela are out of frontier).
    pub fn can_modify_game_time_storage(&self) -> bool {
        self.enslaved_unlocked()
            && !self.black_holes.paused
            && !self.ec_running(12)
            && !self.celestials.enslaved.run
    }

    /// `Enslaved.isStoringGameTime`.
    pub fn is_storing_game_time(&self) -> bool {
        self.can_modify_game_time_storage() && self.celestials.enslaved.is_storing
    }

    /// `Enslaved.toggleStoreBlackHole`.
    pub fn toggle_store_game_time(&mut self) {
        if !self.can_modify_game_time_storage() {
            return;
        }
        self.celestials.enslaved.is_storing = !self.celestials.enslaved.is_storing;
        self.celestials.enslaved.is_storing_real = false;
    }

    /// `storedTimeInsideEnslaved(x)`: the discharge nerf applied while inside the
    /// run — `1e3 · 10^(log10(x/1e3)^0.55)` for `x > 1e3`.
    pub fn stored_time_inside_enslaved(stored: f64) -> f64 {
        if stored <= 1e3 {
            stored
        } else {
            1e3 * 10f64.powf((stored / 1e3).log10().powf(0.55))
        }
    }

    /// `Enslaved.canRelease(auto)`: not while storing real time, in EC12, in
    /// Lai'tela's Reality, Doomed, or (for the auto release) inside Enslaved's
    /// own Reality.
    pub fn enslaved_can_release(&self, auto: bool) -> bool {
        !(self.is_storing_real_time()
            || self.ec_running(12)
            || self.celestials.laitela.run
            || (self.celestials.enslaved.run && auto)
            || self.is_doomed())
    }

    /// `Enslaved.useStoredTime(autoRelease)`: release stored game time as a
    /// burst consumed by the next tick — all of it manually, 1% on the Ra
    /// auto-release (which keeps 99% banked). Returns whether it happened.
    pub fn enslaved_use_stored_time(&mut self, auto: bool) -> bool {
        if !self.enslaved_can_release(auto) || self.celestials.enslaved.stored <= 0.0 {
            return false;
        }
        // (The original also refuses under IU24's requirement *lock* inside
        // Ra's Reality; the deliberate lock system (`isLockingMechanics`) is
        // out of frontier.)
        // A discharge resets the slowest-inversion tracker (IU24's gate).
        self.requirement_checks.reality_slowest_bh = 1.0;
        let mut release = self.celestials.enslaved.stored;
        if self.celestials.enslaved.run {
            release = Self::stored_time_inside_enslaved(release);
        }
        if auto {
            release *= 0.01;
        }
        self.celestials.enslaved.release_ms = release.min(TIME_CAP_MS);
        // `peakGamespeed` assumes a 50 ms update rate for consistency.
        self.celestials.ra.peak_gamespeed =
            self.celestials.ra.peak_gamespeed.max(release / 50.0);
        self.celestials.enslaved.stored *= if auto { 0.99 } else { 0.0 };
        true
    }

    /// The manual discharge (`useStoredTime(false)`).
    pub fn enslaved_release_stored_time(&mut self) -> bool {
        self.enslaved_use_stored_time(false)
    }

    // --- Real-time storage --------------------------------------------------------

    /// `Enslaved.canModifyRealTimeStorage`: unlocked and not Doomed.
    pub fn can_modify_real_time_storage(&self) -> bool {
        self.enslaved_unlocked() && !self.is_doomed()
    }

    /// `Enslaved.storedRealTimeCap`: 8 hours plus Ra's `improvedStoredTime`
    /// bonus (1 hour per Nameless level).
    pub fn stored_real_time_cap(&self) -> f64 {
        let added = if self
            .ra_unlock_active(crate::celestials::ra::RA_UNLOCK_IMPROVED_STORED_TIME)
        {
            1000.0
                * 3600.0
                * self.ra_pet_level(crate::celestials::ra::PET_ENSLAVED) as f64
        } else {
            0.0
        };
        STORED_REAL_BASE_CAP_MS + added
    }

    /// `Enslaved.isStoringRealTime`.
    pub fn is_storing_real_time(&self) -> bool {
        self.can_modify_real_time_storage() && self.celestials.enslaved.is_storing_real
    }

    /// `Enslaved.toggleStoreReal`. Refused only when the storage can't be
    /// modified *and* the store is already full (the original's
    /// `!canModify && !isStoredRealTimeCapped` early return —
    /// `isStoredRealTimeCapped` is true while under the cap).
    pub fn toggle_store_real(&mut self) {
        let under_cap =
            self.celestials.enslaved.stored_real < self.stored_real_time_cap();
        if !self.can_modify_real_time_storage() && !under_cap {
            return;
        }
        self.celestials.enslaved.is_storing_real =
            !self.celestials.enslaved.is_storing_real;
        self.celestials.enslaved.is_storing = false;
    }

    /// `Enslaved.toggleAutoStoreReal` (bank offline time into `storedReal`).
    pub fn toggle_auto_store_real(&mut self) {
        if !self.can_modify_real_time_storage() {
            return;
        }
        self.celestials.enslaved.auto_store_real =
            !self.celestials.enslaved.auto_store_real;
    }

    /// `Enslaved.storeRealTime`: bank a live interval at 70% efficiency; at the
    /// cap the toggle switches itself off.
    pub(crate) fn enslaved_store_real_time(&mut self, real_dt_ms: f64) {
        if self.is_doomed() {
            return;
        }
        let cap = self.stored_real_time_cap();
        let e = &mut self.celestials.enslaved;
        e.stored_real += real_dt_ms * STORED_REAL_EFFICIENCY;
        if e.stored_real > cap {
            e.is_storing_real = false;
            e.stored_real = cap;
        }
    }

    /// `Enslaved.autoStoreRealTime(diffMs)`: bank as much of an *offline* gap
    /// as fits under the cap (at 70% efficiency) and return the remainder to
    /// be simulated normally.
    pub(crate) fn enslaved_auto_store_real_time(&mut self, diff_ms: f64) -> f64 {
        let cap = self.stored_real_time_cap();
        let max_gain = cap - self.celestials.enslaved.stored_real;
        let used = diff_ms.min((max_gain / STORED_REAL_EFFICIENCY).max(0.0));
        self.celestials.enslaved.stored_real += used * STORED_REAL_EFFICIENCY;
        diff_ms - used
    }

    // --- Amplified Realities --------------------------------------------------------

    /// `Enslaved.realityBoostRatio`: how many Realities the stored real time
    /// would simulate — `max(1, floor(storedReal / max(1000, thisReality
    /// realTime)))`.
    pub fn reality_boost_ratio(&self) -> f64 {
        (self.celestials.enslaved.stored_real
            / self.records.this_reality.real_time_ms.max(1000.0))
        .floor()
        .max(1.0)
    }

    /// `Enslaved.canAmplify`: a ratio above 1, not Doomed, and not inside a
    /// celestial Reality.
    pub fn can_amplify_reality(&self) -> bool {
        self.reality_boost_ratio() > 1.0
            && !self.is_doomed()
            && !self.is_in_celestial_reality()
    }

    /// Toggle the amplify-next-Reality flag (the Enslaved tab's button). Can
    /// always be switched off; switching on requires `canAmplify`.
    pub fn toggle_boost_reality(&mut self) {
        if self.celestials.enslaved.boost_reality {
            self.celestials.enslaved.boost_reality = false;
        } else if self.can_amplify_reality() {
            self.celestials.enslaved.boost_reality = true;
        }
    }

    /// Game-time storage step, run at the top of [`tick`](Self::tick). Given the
    /// real interval and the full speed factor, banks the Black-Hole boost into
    /// `stored` and returns the effective speed (1 while storing), then folds in
    /// any pending release burst as raw game time. Returns the game-time
    /// interval (ms) to advance by.
    pub(crate) fn enslaved_apply_time_flow(
        &mut self,
        real_dt_ms: f64,
        speed: f64,
    ) -> f64 {
        let mut speed = speed;
        if self.is_storing_game_time() {
            // Bank the difference between the boosted speed and 1× (the game
            // runs at 1× while storing), amplified by Ra's `improvedStoredTime`
            // (`20^Nameless-level`).
            let gain = real_dt_ms * (speed - 1.0) * self.ra_stored_time_amplification();
            if gain > 0.0 {
                self.celestials.enslaved.stored =
                    (self.celestials.enslaved.stored + gain).min(TIME_CAP_MS);
            }
            speed = 1.0;
        }
        let mut game_dt = real_dt_ms * speed;
        // A release injects its burst as raw game time (a single big tick).
        if self.celestials.enslaved.release_ms > 0.0 {
            game_dt += self.celestials.enslaved.release_ms;
            self.celestials.enslaved.release_ms = 0.0;
        }
        game_dt
    }

    /// The completion hook: mark Enslaved's Reality completed (`completeRun`).
    pub(crate) fn enslaved_complete_run(&mut self) {
        self.celestials.enslaved.completed = true;
    }

    // --- Tesseracts ---------------------------------------------------------------

    /// `Tesseracts.costs(index)`: `10^(1e7 × BASE_COSTS[index])`; past the
    /// hardcoded table the cost is unreachable.
    pub fn tesseract_cost(index: u32) -> Decimal {
        match TESSERACT_BASE_COSTS.get(index as usize) {
            Some(&base) => Decimal::pow10(1e7 * base),
            None => Decimal::MAX_VALUE,
        }
    }

    /// The IP cost of the next Tesseract (`nextCost`).
    pub fn next_tesseract_cost(&self) -> Decimal {
        Self::tesseract_cost(self.celestials.enslaved.tesseracts)
    }

    /// `Tesseracts.effectiveCount`: bought Tesseracts scaled by the
    /// `tesseractMultFromSingularities` milestone (`bought × effect`, the
    /// original's `bought + extra`).
    pub fn tesseract_effective_count(&self) -> f64 {
        let mult = self.singularity_milestone_effect_or(
            crate::celestials::singularity::TESSERACT_MULT_FROM_SINGULARITIES,
            1.0,
        );
        self.celestials.enslaved.tesseracts as f64 * mult
    }

    /// `Tesseracts.capIncrease(count)`: the extra Infinity-Dimension purchases —
    /// `250e3 × 2^(count × milestoneMult)` (0 below one effective Tesseract),
    /// times `boundless + 1`.
    pub fn tesseract_cap_increase_at(&self, count: f64) -> f64 {
        let mult = self.singularity_milestone_effect_or(
            crate::celestials::singularity::TESSERACT_MULT_FROM_SINGULARITIES,
            1.0,
        );
        let total = count * mult;
        let base = if total < 1.0 {
            0.0
        } else {
            250e3 * 2f64.powf(total)
        };
        base * (self.alchemy_boundless() + 1.0)
    }

    /// The current cap increase from bought Tesseracts.
    pub fn tesseract_cap_increase(&self) -> f64 {
        self.tesseract_cap_increase_at(self.celestials.enslaved.tesseracts as f64)
    }

    /// `Tesseracts.canBuyTesseract`: Enslaved's Reality completed and enough
    /// Infinity Points (a threshold — buying does **not** spend them).
    pub fn can_buy_tesseract(&self) -> bool {
        self.celestials.enslaved.completed
            && self.infinity_points >= self.next_tesseract_cost()
    }

    /// `Tesseracts.buyTesseract`. Returns whether one was bought.
    pub fn buy_tesseract(&mut self) -> bool {
        if !self.can_buy_tesseract() {
            return false;
        }
        self.celestials.enslaved.tesseracts += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use break_infinity::Decimal;

    fn enslaved_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // Effarig eternity stage done → Enslaved unlocked.
        game.celestials.effarig.unlock_bits |=
            1 << crate::celestials::effarig::EFFARIG_UNLOCK_ETERNITY;
        game
    }

    #[test]
    fn storing_game_time_banks_the_boost() {
        let mut game = enslaved_game();
        game.celestials.enslaved.is_storing = true;
        // 1000 ms of real time at 5× → banks 4000 ms, game advances 1000 ms.
        let game_dt = game.enslaved_apply_time_flow(1000.0, 5.0);
        assert_eq!(game_dt, 1000.0);
        assert_eq!(game.celestials.enslaved.stored, 4000.0);
    }

    #[test]
    fn release_injects_a_burst() {
        let mut game = enslaved_game();
        game.celestials.enslaved.stored = 1e6;
        assert!(game.enslaved_release_stored_time());
        assert_eq!(game.celestials.enslaved.stored, 0.0);
        // The next flow step injects the burst on top of the normal interval.
        let game_dt = game.enslaved_apply_time_flow(50.0, 1.0);
        assert_eq!(game_dt, 50.0 + 1e6);
    }

    #[test]
    fn run_unlock_needs_the_glyph_requirement() {
        let mut game = enslaved_game();
        game.celestials.enslaved.stored = RUN_UNLOCK_PRICE_MS * 2.0;
        // Without a level-5000 / rarity-100 glyph record, the run stays locked.
        assert!(!game.can_buy_enslaved_unlock(ENSLAVED_UNLOCK_RUN));
        game.records.best_reality.glyph_level = 5000;
        game.records.best_reality.glyph_strength =
            crate::glyphs::rarity_to_strength(100.0);
        assert!(game.can_buy_enslaved_unlock(ENSLAVED_UNLOCK_RUN));
        assert!(game.buy_enslaved_unlock(ENSLAVED_UNLOCK_RUN));
        assert!(game.enslaved_run_unlocked());
    }

    #[test]
    fn discharge_nerf_inside_run_compresses() {
        // 1e6 stored → far less after the ^0.55 compression.
        let nerfed = GameState::stored_time_inside_enslaved(1e6);
        assert!(nerfed < 1e6);
        assert!(nerfed > 1e3);
        // Small amounts pass through unchanged.
        assert_eq!(GameState::stored_time_inside_enslaved(500.0), 500.0);
    }

    #[test]
    fn glyph_level_minimum_applies_in_run() {
        let mut game = enslaved_game();
        game.celestials.enslaved.run = true;
        let glyph = crate::glyphs::Glyph {
            id: 1,
            idx: 0,
            kind: crate::glyphs::GlyphType::Power,
            strength: 1.0,
            level: 100,
            raw_level: 100,
            effects: 0,
        };
        // Below the 5000 minimum, the effective level is raised.
        assert_eq!(game.adjusted_glyph_level(&glyph), 5000.0);
        let _ = Decimal::ONE; // silence unused import in some cfgs
    }

    /// Tesseracts: bought against an IP *threshold* (not spent), gated on a
    /// completed run; the cap increase is `250e3 × 2^count`.
    #[test]
    fn tesseracts_buy_and_raise_the_id_cap() {
        let mut game = GameState::new();
        game.infinity_points = Decimal::pow10(2e7);
        // Not completed yet: no purchase.
        assert!(!game.can_buy_tesseract());
        game.celestials.enslaved.completed = true;

        // First cost is 10^(1e7×2).
        assert_eq!(game.next_tesseract_cost(), Decimal::pow10(2e7));
        assert!(game.buy_tesseract());
        // IP is a threshold, not spent.
        assert_eq!(game.infinity_points, Decimal::pow10(2e7));
        assert_eq!(game.celestials.enslaved.tesseracts, 1);
        // Next cost jumps to 10^(1e7×4); unaffordable now.
        assert!(!game.can_buy_tesseract());

        // Cap increase: 250e3 × 2^1 = 500e3 (no milestone/alchemy scaling).
        assert_eq!(game.tesseract_cap_increase(), 500e3);
        // The ID purchase cap reflects it: 2e6 + 5e5.
        game.infinity_dimensions[0].base_amount = 2_499_999 * 10;
        assert!(!game.id_is_capped(0));
        game.infinity_dimensions[0].base_amount = 2_500_000 * 10;
        assert!(game.id_is_capped(0));
    }

    /// Storing real time banks 70% of the interval, freezes game time and
    /// production, and switches itself off at the cap.
    #[test]
    fn storing_real_time_banks_and_freezes_the_game() {
        let mut game = enslaved_game();
        game.celestials.enslaved.is_storing_real = true;
        assert!(game.is_storing_real_time());

        let am_before = game.antimatter;
        let game_time_before = game.records.total_time_played_ms;
        game.tick(1000.0);
        // 700 ms banked, real time advanced, game time/production frozen.
        assert_eq!(game.celestials.enslaved.stored_real, 700.0);
        assert_eq!(game.records.real_time_played_ms, 1000.0);
        assert_eq!(game.records.total_time_played_ms, game_time_before);
        assert_eq!(game.antimatter, am_before);

        // At the cap (8 h base) the toggle switches itself off.
        game.celestials.enslaved.stored_real = game.stored_real_time_cap() - 100.0;
        game.tick(1000.0);
        assert_eq!(
            game.celestials.enslaved.stored_real,
            game.stored_real_time_cap()
        );
        assert!(!game.celestials.enslaved.is_storing_real);
    }

    /// The offline auto-store banks what fits under the cap and returns the
    /// remainder to simulate.
    #[test]
    fn auto_store_real_time_banks_offline_gaps() {
        let mut game = enslaved_game();
        // Room for 700 ms of banked time = 1000 ms of gap.
        game.celestials.enslaved.stored_real = game.stored_real_time_cap() - 700.0;
        let remainder = game.enslaved_auto_store_real_time(5000.0);
        assert_eq!(remainder, 4000.0);
        assert_eq!(
            game.celestials.enslaved.stored_real,
            game.stored_real_time_cap()
        );
    }

    /// The Ra auto-release discharges 1% of the stored time every 5th tick,
    /// keeping 99% banked.
    #[test]
    fn auto_release_discharges_every_fifth_tick() {
        let mut game = enslaved_game();
        game.celestials.enslaved.stored = 1e6;
        game.celestials.enslaved.is_auto_releasing = true;
        for _ in 0..4 {
            game.tick(50.0);
        }
        assert_eq!(game.celestials.enslaved.stored, 1e6);
        game.tick(50.0);
        assert_eq!(game.celestials.enslaved.stored, 1e6 * 0.99);
    }

    /// An amplified Reality multiplies the rewards by the boost ratio and
    /// consumes the stored real time.
    #[test]
    fn amplified_reality_multiplies_rewards() {
        let mut game = crate::reality::tests::game_at_reality_goal();
        game.celestials.effarig.unlock_bits |=
            1 << crate::celestials::effarig::EFFARIG_UNLOCK_ETERNITY;
        // 10 s of run time, 50 s stored → ratio 5.
        game.records.this_reality.real_time_ms = 10_000.0;
        game.celestials.enslaved.stored_real = 50_000.0;
        assert_eq!(game.reality_boost_ratio(), 5.0);
        assert!(game.can_amplify_reality());
        game.toggle_boost_reality();

        assert!(game.reality());
        // ×5 Realities and Perk Points; stored real time consumed; flag off.
        assert_eq!(game.reality.realities, 5);
        assert_eq!(game.reality.perk_points, 5.0);
        assert_eq!(game.celestials.enslaved.stored_real, 0.0);
        assert!(!game.celestials.enslaved.boost_reality);
    }

    /// IU23 multiplies the imaginary free Dim Boosts by
    /// `floor(0.25 × effectiveCount²)`.
    #[test]
    fn iu23_scales_imaginary_boosts_by_tesseracts() {
        let mut game = GameState::new();
        game.reality.imaginary_upgrade_bits |= 1 << 12;
        game.reality.imaginary_rebuyables[0] = 2;
        assert_eq!(game.imaginary_dim_boosts(), 4e4);

        game.reality.imaginary_upgrade_bits |= 1 << 23;
        game.celestials.enslaved.tesseracts = 4;
        // floor(0.25 × 16) = 4 → ×4.
        assert_eq!(game.imaginary_dim_boosts(), 16e4);
    }
}
