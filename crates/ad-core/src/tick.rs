use break_infinity::Decimal;

use crate::data::constants::BIG_CRUNCH_THRESHOLD;
use crate::GameState;

/// Native offline tick resolution in ms (the original simulates offline at a
/// 50 ms base tick). See `docs/design/2026-06-30-offline-progress.md`.
const OFFLINE_BASE_TICK_MS: f64 = 50.0;

#[allow(clippy::needless_range_loop)]
impl GameState {
    /// Advance the game by `dt_ms` milliseconds.
    /// Production chain: AD[n+1] produces AD[n], AD1 produces antimatter.
    /// All production is scaled by the dimension's multiplier and tickspeed effect.
    pub fn tick(&mut self, dt_ms: f64) {
        // Game-speed factor (`getGameSpeedupFactor`): EC12 runs the game 1000×
        // slower. Production, currencies, and game-time records use the scaled
        // interval; autobuyer timers and real-time records stay on real time.
        let real_dt_ms = dt_ms;

        // Storing real time skips the whole game loop (`realTimeMechanics`):
        // only Ra memories/momentum and the Dark Matter Dimensions run, the
        // real-time records advance, the interval is banked at 70% efficiency,
        // and the autobuyers still tick. Black-hole phases, production, and
        // game-time records all freeze.
        if self.is_storing_real_time() {
            self.ra_memory_tick(real_dt_ms, false);
            self.ra_tick(real_dt_ms);
            self.dmd_tick(real_dt_ms);
            self.records.real_time_played_ms += real_dt_ms;
            self.records.this_infinity.real_time_ms += real_dt_ms;
            self.records.this_eternity.real_time_ms += real_dt_ms;
            self.records.this_reality.real_time_ms += real_dt_ms;
            self.enslaved_store_real_time(real_dt_ms);
            self.tick_autobuyers(real_dt_ms);
            return;
        }

        // The Ra auto-release: every 5th tick, discharge 1% of the stored game
        // time (keeping 99% banked) as a burst folded in below.
        if self.celestials.enslaved.is_auto_releasing && !self.black_holes_are_negative()
        {
            self.celestials.enslaved.auto_release_tick += 1;
            if self.celestials.enslaved.auto_release_tick >= 5 {
                self.celestials.enslaved.auto_release_tick = 0;
                self.enslaved_use_stored_time(true);
            }
        }

        // Black-hole phases advance on real time, before the speed factor is
        // read (`BlackHoles.updatePhases`).
        self.tick_black_holes(real_dt_ms);
        // Enslaved game-time storage/release wraps the speed factor: while
        // storing, the Black-Hole boost is banked and the game runs at 1×; a
        // release injects its burst as raw game time.
        let dt_ms = self.enslaved_apply_time_flow(real_dt_ms, self.game_speed_factor());

        // Ra's real-time mechanics (`realTimeMechanics`): pet Memories accrue
        // from real time (chunk generation is off while storing real time), and
        // the peak game-speed / momentum-time accumulators advance.
        self.ra_memory_tick(real_dt_ms, !self.celestials.enslaved.is_storing_real);
        self.ra_tick(real_dt_ms);

        // Lai'tela's real-time mechanics: Dark Matter Dimensions produce DM/DE,
        // the entropy/destabilization run advances, and Imaginary Machines
        // approach their cap.
        self.dmd_tick(real_dt_ms);
        self.laitela_reality_tick(real_dt_ms);
        self.tick_imaginary_machines(real_dt_ms);

        // Pelle (while doomed): Reality Shards, rift fills, the Galaxy
        // Generator, and the game-end check.
        self.pelle_tick(real_dt_ms);

        // Run autobuyers before production (real time — autobuyer intervals are
        // wall-clock in the original).
        self.tick_autobuyers(real_dt_ms);

        // Passive Infinity-Point generation from the `ipGen` Infinity Upgrade
        // (mirrors the original's `preProductionGenerateIP`).
        self.generate_passive_ip(dt_ms);
        // TS181: 1% of the pending crunch IP per second.
        self.generate_ts181_ip(dt_ms);
        // Teresa's `epGen` unlock: passive EP from the peak EP/min.
        self.generate_teresa_ep(dt_ms);
        // Charged `ipGen`: Reality Machines each real-time second proportional
        // to the pending Reality's RM gain (`applyAutoprestige`):
        // `gainedRM × teresaLevel² × continuousTTBoost.autoPrestige`.
        if self
            .infinity_upgrade_charged(crate::infinity_upgrades::InfinityUpgrade::IpGen)
        {
            let level = self.ra_pet_level(crate::celestials::ra::PET_TERESA) as f64;
            self.reality.machines += self.gained_reality_machines()
                * Decimal::from_float(level * level * self.ra_tt_boost_auto_prestige())
                * Decimal::from_float(real_dt_ms / 1000.0);
        }
        // Chaos rift milestone 2: 10% of the Eternity EP gain per (real)
        // second (`applyAutoprestige`'s Pelle branch).
        if self.pelle_rift_milestone(crate::celestials::pelle::RIFT_CHAOS, 2) {
            self.eternity_points += self.gained_eternity_points()
                * Decimal::from_float(0.1)
                * Decimal::from_float(real_dt_ms / 1000.0);
        }

        // Advance the time records *before* production, matching the original game
        // loop (`records.thisInfinity.time += diff`, etc. run before the Dimension
        // ticks). Dimension production reads these — e.g. the time-based Antimatter
        // Dimension achievement multipliers (56/76/91/92) and Infinity Challenge 8's
        // decay — so they must see *this* tick's elapsed time, not last tick's.
        // Pre-Infinity the game-speed multiplier is 1, so game and real time both
        // advance by `dt_ms`. Runs during offline replay too, since that loops `tick`.
        self.records.total_time_played_ms += dt_ms;
        self.records.real_time_played_ms += real_dt_ms;
        self.records.this_infinity.time_ms += dt_ms;
        self.records.this_infinity.real_time_ms += real_dt_ms;
        self.records.this_eternity.time_ms += dt_ms;
        self.records.this_eternity.real_time_ms += real_dt_ms;
        self.records.this_reality.time_ms += dt_ms;
        self.records.this_reality.real_time_ms += real_dt_ms;

        // Advance the per-run challenge accumulators (`updateNormalAndInfinity
        // Challenges`), matching the original game loop: it runs *after* the
        // autobuyers and the records-time increment, but before production. Order
        // matters for NC2 — a tickspeed/dimension purchase (autobuyer, above)
        // zeroes `chall2Pow`, and this then regrows it by one tick's worth; running
        // it before the autobuyers would let the purchase wipe the growth.
        self.update_challenges(dt_ms);

        // The original produces Time → Infinity → Antimatter Dimensions in that
        // order each tick (`game.js`), so Antimatter Dimension production reads the
        // Infinity Power (and free Tickspeed) generated *this* tick, not last tick.
        // Time Dimensions produce Time Shards → free Tickspeed upgrades.
        self.tick_time_dimensions(dt_ms);
        // Infinity Dimensions produce Infinity Power (feeds the AD multiplier below).
        self.tick_infinity_dimensions(dt_ms);

        // Production flows from higher dimensions to lower and, from the 1st
        // dimension, into antimatter (`AntimatterDimensions.tick`). Two subtleties
        // must match the original exactly:
        //
        //  * Dimension→dimension production runs at 1/10 the rate of the
        //    1st-dimension→antimatter production: the original passes `diff / 10`
        //    to `produceDimensions` but the full `diff` to `produceCurrency`, and
        //    `productionForDiff` scales linearly by that interval.
        //  * The chain is applied top-down, mutating each tier's amount before the
        //    tier below reads it (`for (tier = max; tier >= 1; --tier)`), so a
        //    dimension produces from its amount *including* this tick's gain from
        //    the tier above (and AD1 feeds antimatter from its just-bumped amount).
        let dt = Decimal::from_float(dt_ms / 1000.0);
        let dt_dim = Decimal::from_float(dt_ms / 10.0 / 1000.0);

        // Normal Challenge 12 shifts the chain up by one: the 1st *and* 2nd
        // dimensions make antimatter, and higher dimensions feed 2 tiers below
        // (AD3→AD1, AD4→AD2, …). Locked dimensions and EC3-silenced tiers produce
        // 0 via `dimension_production_per_second`, so the loop needs no extra guard.
        // `AntimatterDimensions.tick`'s `hasBigCrunchGoal`: pre-break, or inside any
        // antimatter challenge, the run targets a Big Crunch goal (`1e308` for
        // Normal Challenges, the IC's own goal for Infinity Challenges). Post-break
        // and outside a challenge, antimatter grows without bound.
        let goal = self.infinity_goal();
        let has_big_crunch_goal =
            !self.broke_infinity || self.in_any_antimatter_challenge();

        // The original's `AntimatterDimensions.tick` returns early once the goal is
        // met (`hasBigCrunchGoal && antimatter.gte(infinityGoal)`): pre-break the
        // dimensions are hidden behind the Big Crunch button, so production — and
        // thus every dimension amount and antimatter itself — freezes until the
        // player crunches. `peak_am` tracks the post-production, pre-cap antimatter
        // for the `maxAM` records below (mirroring the antimatter setter, which
        // records `maxAM` before the goal cap).
        let mut peak_am = self.antimatter;
        if !(has_big_crunch_goal && self.antimatter >= goal) {
            let offset = if self.challenge_running(12) { 2 } else { 1 };
            for producer in (offset..8).rev() {
                let produced = self.dimension_production_per_second(producer) * dt_dim;
                self.dimensions[producer - offset].amount += produced;
            }

            // Any 1st Antimatter Dimension stock breaks the "no AD1" run flag
            // (`antimatter-dimension.js` checks this after dimension production).
            if self.dimensions[0].amount > Decimal::ZERO {
                self.requirement_checks.eternity_no_ad1 = false;
            }

            // The 1st dimension (and the 2nd under NC12) makes antimatter at the
            // full interval, reading its amount after the chain above has fed into
            // it. `total_antimatter` (monotonic, survives crunches) counts all
            // antimatter produced, before the Big Crunch cap.
            let mut am_gain = self.dimension_production_per_second(0) * dt;
            if self.challenge_running(12) {
                am_gain += self.dimension_production_per_second(1) * dt;
            }
            self.antimatter += am_gain;
            self.total_antimatter += am_gain;
            // Any antimatter gain breaks the "no antimatter this reality" flag
            // (`Currency.antimatter.add`, currency.js).
            if am_gain > Decimal::ZERO {
                self.requirement_checks.reality_no_am = false;
            }

            // The antimatter setter records `maxAM` from this post-gain value —
            // before the cap below — so an overshoot on the reaching tick counts
            // (maxAM ends slightly above the goal pre-break).
            peak_am = self.antimatter;

            // `Currency.antimatter.dropTo(Player.infinityGoal)`: production may
            // overshoot the goal on the final tick before a crunch; cap it back.
            if has_big_crunch_goal && self.antimatter > goal {
                self.antimatter = goal;
            }
        }

        // Convert this tick's Time Shards into free Tickspeed upgrades
        // (`totalTickGained`). The original does this *after* `AntimatterDimensions
        // .tick` (game.js), so an upgrade earned this tick only speeds up Antimatter
        // Dimension production from the next tick — not this one.
        self.update_free_tickspeed();

        // Replicanti grow (multiplying Infinity Dimensions on the next tick, matching
        // the original's `replicantiLoop` running after the dimension ticks).
        self.tick_replicanti(dt_ms);
        // REPLICANTI_TICK_AFTER achievements (95, 106).
        self.check_replicanti_after_achievements();

        // Eternity-Milestone per-tick effects: auto-complete Infinity
        // Challenges (autoIC, 7 eternities) and auto-unlock Infinity
        // Dimensions (autoUnlockID, 25).
        self.try_complete_infinity_challenges();
        self.try_auto_unlock_infinity_dimensions();

        // EC12's time limit is checked every tick (`EternityChallenge(12)
        // .tryFail()` in the game loop).
        self.ec_try_fail(12);

        // Dilated Time generation, Tachyon Galaxies, and the ttGenerator
        // upgrade's TT stream.
        self.tick_dilation(dt_ms);

        // `passivePrestigeGen`: continuous Eternities (RU14) and Infinities
        // (`infinitiedGen` / RU11 / Effarig's Eternity unlock), with the
        // `partEternitied` / `partInfinitied` carries.
        self.passive_prestige_gen(dt_ms);

        // Track the peak Infinity Points this eternity (the original updates
        // `thisEternity.maxIP` in the `Currency.infinityPoints` setter; the
        // in-tick IP source is the passive `ipGen` upgrade).
        self.records.this_eternity.max_ip =
            self.records.this_eternity.max_ip.max(&self.infinity_points);
        self.records.this_reality.max_ip =
            self.records.this_reality.max_ip.max(&self.infinity_points);
        // Track the peak antimatter this infinity, mirroring the antimatter
        // setter's `thisInfinity.maxAM = maxAM.max(value)`. `peak_am` is the
        // post-production value *before* the goal cap, so a pre-crunch overshoot is
        // counted. The same setter also updates the this-reality peak.
        self.records.this_infinity.max_am =
            self.records.this_infinity.max_am.max(&peak_am);
        self.records.this_reality.max_am =
            self.records.this_reality.max_am.max(&peak_am);
        // Peak antimatter this eternity (persists across crunches; gates Infinity
        // Challenge unlocks).
        let prev_peak = self.records.this_eternity.max_am;
        self.records.this_eternity.max_am =
            self.records.this_eternity.max_am.max(&peak_am);

        // Reality-record peaks (the original tracks these in the EP / DT /
        // replicanti currency setters): peak EP, replicanti, and DT this
        // reality feed the RM formula and glyph level.
        self.records.this_reality.max_ep =
            self.records.this_reality.max_ep.max(&self.eternity_points);
        self.records.this_reality.max_replicanti = self
            .records
            .this_reality
            .max_replicanti
            .max(&self.replicanti.amount);
        self.records.this_reality.max_dt = self
            .records
            .this_reality
            .max_dt
            .max(&self.dilation.dilated_time);

        // Auto-achievements regrant over real time after the first Reality.
        self.tick_auto_achievements(real_dt_ms);

        // Perk automation: EU auto-grants + the dilation/TD/Reality-study
        // auto-unlock perks.
        self.tick_perk_effects();

        // EC auto-completion (the PEC perks; real time).
        self.ec_auto_complete_tick(real_dt_ms);

        // Reality Upgrades: per-tick requirement checks (RU11/14/20/21/22).
        // (Their continuous generation lives in `passive_prestige_gen`.)
        self.check_reality_upgrade_reqs_on_tick();
        // Imaginary Upgrades: the deep requirement latches (13/14/22/23/24).
        self.check_imaginary_upgrade_reqs_on_tick();

        // Alchemy `uncountability`: passive Realities and Perk Points per
        // (real) second; whole Realities bank via the fractional carry.
        let uncountability = self.alchemy_uncountability();
        if uncountability > 0.0 {
            let gain = uncountability * real_dt_ms / 1000.0;
            self.reality.perk_points += gain;
            self.reality.realities_frac += gain;
            let whole = self.reality.realities_frac.floor();
            self.reality.realities += whole as u32;
            self.reality.realities_frac -= whole;
        }

        // The Automator executes on real time, after production and
        // automation like the original game loop (`AutomatorBackend
        // .update(realDiff)`).
        self.automator_update(real_dt_ms);

        // `updatePrestigeRates`: peak IP/min / EP/min for the header buttons.
        self.update_prestige_rates();

        // Tab-notification checks the original runs from the antimatter setter:
        // a newly crossed IC unlock threshold and an affordable autobuyer unlock.
        self.notify_ic_unlock(prev_peak);
        self.notify_new_autobuyer();

        // Normal-achievement GAME_TICK_AFTER checks (24, 31, 42–46, and the
        // later-batch tick achievements). `dt_ms` is the game-time step (drives
        // the marathon timers).
        self.check_tick_achievements(dt_ms);

        // V's per-tick unlock checks (`V.checkForUnlocks`): the ST-gated
        // rewards and, while running, the V-achievement completions.
        self.v_check_for_unlocks();

        // Advance the tutorial highlight if the next step's condition now holds
        // (mirrors the original's game-loop-driven `tutorialLoop`).
        self.tutorial_loop();
    }

    /// Advance the per-run challenge accumulators (`updateNormalAndInfinityChallenges`):
    /// NC11 matter growth (and its annihilation soft reset), NC3's exponential
    /// 1st-dimension multiplier, and NC2's linear production recovery. Called at
    /// the top of [`tick`](Self::tick), before autobuyers and production. A no-op
    /// unless the corresponding challenge is running.
    fn update_challenges(&mut self, dt_ms: f64) {
        // NC11 (or IC6): normal matter rises once a 2nd Antimatter Dimension
        // exists; under NC11 only, if it overtakes antimatter (and you cannot yet
        // Crunch) it annihilates.
        if self.challenge_running(11) || self.infinity_challenge_running(6) {
            if self.dimensions[1].amount != Decimal::ZERO {
                // `Currency.matter.bumpTo(1)` — never let it drop below 1 here.
                self.matter = self.matter.max(&Decimal::ONE);
                // Caps are the values reached at ~1e308 IP.
                let capped_base = 1.03
                    + (self.dim_boosts.min(400) as f64) / 200.0
                    + (self.galaxies.min(100) as f64) / 100.0;
                let growth = Decimal::from_float(capped_base)
                    .pow(&Decimal::from_float(dt_ms / 20.0));
                // The `Currency.matter` setter clamps to Number.MAX_VALUE.
                self.matter = (self.matter * growth).min(&BIG_CRUNCH_THRESHOLD);
            }
            if self.matter > self.antimatter
                && self.challenge_running(11)
                && !self.can_big_crunch()
            {
                // Annihilation (NC11 only, not IC6): a Dimension-Boost-style soft
                // reset that grants no boost (`softReset(0, true, true)` — forced,
                // so the ANR perk does not soften it), keeping boosts and galaxies.
                self.dim_boost_reset_forced();
            }
        }

        // NC3: the 1st dimension's exponential multiplier grows ×1.00038 per
        // 100 ms, uncapped up to Number.MAX_VALUE.
        if self.challenge_running(3) {
            let growth =
                Decimal::from_float(1.000_38).pow(&Decimal::from_float(dt_ms / 100.0));
            self.chall3_pow = (self.chall3_pow * growth).min(&BIG_CRUNCH_THRESHOLD);
        }

        // NC2: production recovers linearly to full (1) over 3 minutes since the
        // last AD/tickspeed purchase (which resets it to 0).
        if self.challenge_running(2) {
            self.chall2_pow = (self.chall2_pow + dt_ms / 100.0 / 1800.0).min(1.0);
        }

        // IC2: a Dimensional Sacrifice fires automatically every 400 ms (once an
        // 8th dimension exists; `sacrifice` no-ops otherwise). The original is an
        // `if/else`: on the tick the counter reaches 400 it sacrifices and takes
        // the modulo *without* also adding this tick's delta, and it accrues real
        // time (`Date.now() - lastUpdate`, clamped to `[1, 6h]`; = `dt_ms` at
        // game-speed 1) rather than the game-time step.
        if self.infinity_challenge_running(2) {
            if self.ic2_count >= 400.0 {
                self.sacrifice();
                self.ic2_count %= 400.0;
            } else {
                self.ic2_count += dt_ms.clamp(1.0, 21_600_000.0);
            }
        }
    }

    /// Advance the game by `repeats` discrete steps of `dt_ms` each.
    ///
    /// Used by the dev game-speed control: running N real-sized ticks is more
    /// faithful than a single `dt_ms * N` step, which would lump discrete
    /// per-tick effects (e.g. autobuyers) into one and lose precision.
    pub fn ticks(&mut self, dt_ms: f64, repeats: u32) {
        for _ in 0..repeats {
            self.tick(dt_ms);
        }
    }

    /// Run the simulation for `total_ms` of real time, using `tick_size_ms` per step.
    pub fn simulate(&mut self, total_ms: f64, tick_size_ms: f64) {
        let steps = (total_ms / tick_size_ms) as u64;
        for _ in 0..steps {
            self.tick(tick_size_ms);
        }
    }

    /// Replay `game_ms` of (already speed-scaled) game time as offline progress.
    ///
    /// The interval is spread across `min(game_ms / 50, offline_ticks)` discrete
    /// ticks rather than one big step, so per-tick effects (autobuyers, which
    /// fire at most once per tick) behave. Below `offline_ticks × 50 ms` the
    /// replay runs at the native 50 ms resolution; beyond it the tick count
    /// saturates at `offline_ticks` and each tick stretches.
    ///
    /// `offline_ticks` is the player's resolution setting (default 100_000,
    /// reproducing the original game's offline tick budget). A non-positive
    /// `game_ms` is a no-op. See
    /// `docs/design/2026-06-30-offline-progress.md`.
    pub fn simulate_offline(&mut self, game_ms: f64, offline_ticks: u32) {
        // `Enslaved.autoStoreRealTime`: with the toggle on, offline time is
        // banked into the real-time store (70% efficiency, up to the cap)
        // before the remainder is simulated.
        let game_ms = if self.can_modify_real_time_storage()
            && self.celestials.enslaved.auto_store_real
        {
            self.enslaved_auto_store_real_time(game_ms)
        } else {
            game_ms
        };
        let (ticks, tick_size) = offline_plan(game_ms, offline_ticks);
        if ticks == 0 {
            return;
        }
        self.offline_currency_gain(game_ms);
        self.ticks(tick_size, ticks);
    }

    /// The lump-sum currency awards of the original's `simulateTime` (fired once
    /// per offline catch-up, before the tick replay): the Eternity-milestone
    /// offline generators — `autoEternities` (200) banks Eternities at half the
    /// best rate, else `autoInfinities` (1000) banks Infinities, else `autoEP`
    /// (6) grants 25% of the best EP/min — plus the `ipOffline` Infinity
    /// Upgrade's 50%-of-best-IP/min award. The GUI's chunked replay calls this
    /// once before its first chunk; [`simulate_offline`](Self::simulate_offline)
    /// calls it itself.
    ///
    /// The original gates these on `player.options.offlineProgress` (not
    /// modelled — the 8.8 gap) and scales the milestone rewards by the game
    /// speed; our single `away_ms` is the replayed game-time interval, which
    /// carries that scaling for the tick replay already.
    pub fn offline_currency_gain(&mut self, away_ms: f64) {
        // 35: be offline for over 6 hours (the original checks `Date.now() -
        // lastUpdate` on the first tick back; the replayed away interval is the
        // engine's view of that gap).
        if away_ms >= 21_600_000.0 {
            self.unlock_achievement(35);
        }
        if away_ms <= 0.0 {
            return;
        }
        // `getEternitiedMilestoneReward` / `getInfinitiedMilestoneReward` /
        // `getOfflineEPGain`, applied in the original's if/else-if order.
        let eternitied = if self.auto_eternities_available() {
            (self.records.this_reality.best_eternities_per_ms
                * Decimal::from_float(away_ms / 2.0))
            .floor()
        } else {
            Decimal::ZERO
        };
        let infinitied = if self.auto_infinities_available() {
            (self.records.this_eternity.best_infinities_per_ms
                * Decimal::from_float(away_ms / 2.0))
            .floor()
        } else {
            Decimal::ZERO
        };
        if eternitied > Decimal::ZERO {
            self.eternities += eternitied;
        } else if infinitied > Decimal::ZERO {
            self.infinities += infinitied;
        } else if self.eternity_milestone_reached(6) {
            // `autoEP`: 25% of the best EP/min (bestEPminReality) while away.
            self.eternity_points += self.records.best_eternity.best_ep_min_reality
                * Decimal::from_float(away_ms / 60_000.0 / 4.0);
        }
        if self.ip_offline_bought {
            self.infinity_points +=
                self.records.this_eternity.best_ip_ms_without_max_all
                    * Decimal::from_float(away_ms / 2.0);
        }
    }
}

/// The replay plan for `game_ms` of offline time at `offline_ticks` resolution:
/// the discrete tick count and the per-tick size in ms. Returns `(0, 0.0)` when
/// there is nothing to replay (`game_ms <= 0`).
///
/// The GUI uses this to drive a chunked, progress-bar-backed replay (the offline
/// catch-up modal): it splits `ticks` into batches and runs `tick_size`-sized
/// ticks itself, so the budget policy stays in the engine while the pacing/UI
/// lives above it. [`GameState::simulate_offline`] is the all-at-once path over
/// the same plan. See `docs/design/2026-06-30-offline-progress.md`.
pub fn offline_plan(game_ms: f64, offline_ticks: u32) -> (u32, f64) {
    if game_ms <= 0.0 {
        return (0, 0.0);
    }
    let ticks = offline_tick_count(game_ms, offline_ticks);
    (ticks, game_ms / ticks as f64) // tick_size >= 50 ms once saturated
}

/// The discrete tick budget for replaying `game_ms` of offline time:
/// `min(game_ms / 50, offline_ticks)`, never below 1. Below
/// `offline_ticks × 50 ms` this is the native 50 ms count; past it the budget
/// saturates at `offline_ticks` (so each tick covers more than 50 ms).
fn offline_tick_count(game_ms: f64, offline_ticks: u32) -> u32 {
    let want = (game_ms / OFFLINE_BASE_TICK_MS).floor() as u32;
    want.clamp(1, offline_ticks.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutobuyerMode, GameState};

    #[test]
    fn dimension_production_uses_this_ticks_advanced_time() {
        use break_infinity::Decimal;
        // Achievement 56 boosts Antimatter Dimensions in the first 3 minutes of an
        // Infinity by `6/(min+3)`. Because the records time advances *before*
        // production, a tick's antimatter gain must use the post-increment time.
        let mut game = GameState::new();
        game.broke_infinity = true; // no Big Crunch goal freeze
        game.autobuyers.enabled = false; // no purchases perturbing the amounts
        game.unlock_achievement(56);
        game.records.this_infinity.time_ms = 60_000.0;
        game.dimensions[0].bought = 10;
        game.dimensions[0].amount = Decimal::from_float(1e10);

        // Expected: production reads the time *after* this tick's +50 ms.
        let mut reference = game.clone();
        reference.records.this_infinity.time_ms = 60_050.0;
        let expected_gain =
            reference.dimension_production_per_second(0) * Decimal::from_float(0.05);

        let before = game.antimatter;
        game.tick(50.0);
        let actual_gain = game.antimatter - before;

        let ratio = actual_gain.to_f64() / expected_gain.to_f64();
        assert!((ratio - 1.0).abs() < 1e-9, "ratio={ratio}");
    }

    #[test]
    fn matter_grows_under_ic6_but_does_not_annihilate() {
        use break_infinity::Decimal;
        // `updateNormalAndInfinityChallenges` grows `matter` under NC11 *or* IC6.
        let mut game = GameState::new();
        game.infinity_challenge.current = 6;
        game.dimensions[1].amount = Decimal::from_float(1e3); // a 2nd dimension exists
        game.antimatter = Decimal::ONE; // tiny → matter will exceed it
        game.matter = Decimal::ZERO;

        game.update_challenges(50.0);

        // Matter is bumped to 1 then grows (capped_base 1.03 over 50/20 ticks).
        assert!(
            game.matter > Decimal::ONE,
            "matter did not grow: {}",
            game.matter
        );
        // Annihilation is NC11-only, so under IC6 the 2nd dimension survives even
        // though matter now exceeds antimatter.
        assert_eq!(game.dimensions[1].amount, Decimal::from_float(1e3));
    }

    #[test]
    fn offline_tick_count_native_resolution_below_cap() {
        // 50 s of game time wants 1000 ticks at 50 ms; a generous budget leaves
        // that untouched.
        assert_eq!(offline_tick_count(50_000.0, 100_000), 1000);
    }

    #[test]
    fn offline_tick_count_saturates_at_budget() {
        // A long interval saturates at `offline_ticks`; each tick then spans far
        // more than 50 ms.
        let ticks = offline_tick_count(50_000_000.0, 1000);
        assert_eq!(ticks, 1000);
        let tick_size = 50_000_000.0 / ticks as f64;
        assert!(tick_size >= OFFLINE_BASE_TICK_MS);
        assert_eq!(tick_size, 50_000.0);
    }

    #[test]
    fn offline_tick_count_floor_is_one() {
        // Sub-tick intervals and a zero budget both clamp up to a single tick.
        assert_eq!(offline_tick_count(10.0, 1000), 1);
        assert_eq!(offline_tick_count(1_000_000.0, 0), 1);
    }

    #[test]
    fn offline_plan_splits_game_time_across_the_budget() {
        // Nothing to replay for a non-positive interval.
        assert_eq!(offline_plan(0.0, 1000), (0, 0.0));
        assert_eq!(offline_plan(-5.0, 1000), (0, 0.0));

        // Below the budget: native 50 ms resolution, tick_size == 50 ms and
        // ticks × tick_size reconstructs the full interval.
        let (ticks, tick_size) = offline_plan(50_000.0, 100_000);
        assert_eq!(ticks, 1000);
        assert_eq!(tick_size, 50.0);
        assert_eq!(ticks as f64 * tick_size, 50_000.0);

        // Past the budget: the count saturates and each tick stretches, but the
        // product still covers the whole interval.
        let (ticks, tick_size) = offline_plan(50_000_000.0, 1000);
        assert_eq!(ticks, 1000);
        assert_eq!(tick_size, 50_000.0);
        assert!(tick_size >= OFFLINE_BASE_TICK_MS);
    }

    #[test]
    fn offline_plan_chunked_matches_all_at_once() {
        // Driving the plan in batches (as the GUI progress modal does) is
        // identical to one `simulate_offline` call, since both loop `ticks`.
        let mut base = GameState::new();
        base.dimensions[1].amount = Decimal::new(1.0, 3);

        let mut all_at_once = base.clone();
        all_at_once.simulate_offline(50_000.0, 100_000);

        let (ticks, tick_size) = offline_plan(50_000.0, 100_000);
        let mut chunked = base;
        let (base_ticks, extra) = (ticks / 100, ticks % 100);
        for c in 0..100 {
            let n = base_ticks + if c < extra { 1 } else { 0 };
            chunked.ticks(tick_size, n);
        }

        assert_eq!(chunked.antimatter, all_at_once.antimatter);
    }

    #[test]
    fn simulate_offline_non_positive_is_noop() {
        let mut game = GameState::new();
        game.dimensions[1].amount = Decimal::new(1.0, 1);
        let before = game.antimatter;

        game.simulate_offline(0.0, 1000);
        game.simulate_offline(-5_000.0, 1000);

        assert_eq!(game.antimatter, before);
    }

    #[test]
    fn simulate_offline_matches_simulate_at_native_resolution() {
        // When the budget doesn't bind, simulate_offline is exactly the native
        // 50 ms tick loop (`simulate` with a 50 ms step over the same total).
        let mut base = GameState::new();
        base.dimensions[0].amount = Decimal::new(1.0, 1);
        base.dimensions[1].amount = Decimal::new(1.0, 1);

        let mut via_offline = base.clone();
        via_offline.simulate_offline(50_000.0, 100_000);

        let mut via_simulate = base;
        via_simulate.simulate(50_000.0, OFFLINE_BASE_TICK_MS);

        assert_eq!(via_offline.antimatter, via_simulate.antimatter);
        for tier in 0..8 {
            assert_eq!(
                via_offline.dimensions[tier].amount,
                via_simulate.dimensions[tier].amount
            );
        }
    }

    #[test]
    fn offline_ticks_is_a_behaviour_knob_for_autobuyers() {
        // The tick budget governs how often once-per-tick effects fire: more
        // ticks over the same game time → more autobuyer purchases. With ample
        // antimatter, the 1st-dimension autobuyer buys more under a large budget
        // (fine resolution) than a tiny one (coarse).
        let mut base = GameState::new();
        base.antimatter = Decimal::new(1.0, 100);
        base.autobuyers.enabled = true;
        base.autobuyers.dimensions[0].is_bought = true;
        base.autobuyers.dimensions[0].is_active = true;
        base.autobuyers.dimensions[0].mode = AutobuyerMode::BuySingle;

        // 50 s of game time. Large budget → 50 ms ticks (≈100 autobuyer fires at
        // the 500 ms interval); tiny budget → 5 s ticks (one fire each, 10 total).
        let mut fine = base.clone();
        fine.simulate_offline(50_000.0, 100_000);

        let mut coarse = base;
        coarse.simulate_offline(50_000.0, 10);

        assert!(
            fine.dimensions[0].bought > coarse.dimensions[0].bought,
            "fine={} coarse={}",
            fine.dimensions[0].bought,
            coarse.dimensions[0].bought
        );
    }

    #[test]
    fn tick_caps_antimatter_at_big_crunch_threshold() {
        let cap = BIG_CRUNCH_THRESHOLD;
        let mut game = GameState::new();

        // Start just below the cap with strong production so a tick would
        // otherwise push antimatter well past it.
        game.antimatter = cap * Decimal::from_float(0.9);
        game.dimensions[0].amount = Decimal::new(1.0, 400);

        game.tick(1000.0);

        assert!(
            game.antimatter <= cap,
            "antimatter {:?} exceeded the cap {:?}",
            game.antimatter,
            cap
        );
        assert_eq!(game.antimatter, cap);
    }
}
