//! Ra (Feature 7.5) — the Celestial of pets & memories, plus Glyph Alchemy
//! (`alchemy.rs`). Four Celestial *Memories* gain memory chunks from real time
//! inside Ra's Reality and level up; Ra's 28 levels unlock rewards that mostly
//! re-activate effects earlier Celestials deferred. See
//! `docs/design/2026-07-07-ra.md`. Original: `celestials/ra/ra.js` +
//! `secret-formula/celestials/ra.js`.
//!
//! **Scope.** The pet/memory/level/upgrade system, all 28 unlocks (state +
//! effect readers), Remembrance, the theorem-boost + achievement unlocks,
//! momentum, peak-game-speed tracking, and Ra's Reality are ported. The charged
//! Infinity-Upgrade *state* (count gate + charge/discharge) is modelled; the
//! per-upgrade charged *effect variants* are deferred (documented). Glyph
//! Alteration thresholds live in `alteration` below. Deferred QoL/automation
//! unlocks (`allGamespeedGlyphs`, `autoPulseTime`, `blackHolePowerAutobuyers`,
//! `instantEC…`, `autoUnlockDilation`) store their bit but are neutral.

use crate::state::GameState;
use break_infinity::Decimal;

// --- Pet indices ------------------------------------------------------------
pub const PET_TERESA: usize = 0;
pub const PET_EFFARIG: usize = 1;
pub const PET_ENSLAVED: usize = 2;
pub const PET_V: usize = 3;
pub const PET_COUNT: usize = 4;

pub const RA_LEVEL_CAP: u32 = 25;

// --- Unlock ids (`secret-formula/celestials/ra.js`) -------------------------
pub const RA_UNLOCK_AUTO_TP: u8 = 0;
pub const RA_UNLOCK_CHARGED_INFINITY: u8 = 1;
pub const RA_UNLOCK_TERESA_XP: u8 = 2;
pub const RA_UNLOCK_ALTERED_GLYPHS: u8 = 3;
pub const RA_UNLOCK_EFFARIG_MEMORIES: u8 = 4;
pub const RA_UNLOCK_PERK_SHOP_INCREASE: u8 = 5;
pub const RA_UNLOCK_DILATION_STARTING_TP: u8 = 6;
pub const RA_UNLOCK_EXTRA_GLYPH_CHOICES: u8 = 7;
pub const RA_UNLOCK_GLYPH_ALCHEMY: u8 = 8;
pub const RA_UNLOCK_EFFARIG_XP: u8 = 9;
pub const RA_UNLOCK_GLYPH_EFFECT_COUNT: u8 = 10;
pub const RA_UNLOCK_ENSLAVED_MEMORIES: u8 = 11;
pub const RA_UNLOCK_RELIC_SHARD_LEVEL: u8 = 12;
pub const RA_UNLOCK_MAX_RARITY: u8 = 13;
pub const RA_UNLOCK_BH_POWER_AUTOBUYERS: u8 = 14;
pub const RA_UNLOCK_IMPROVED_STORED_TIME: u8 = 15;
pub const RA_UNLOCK_ENSLAVED_XP: u8 = 16;
pub const RA_UNLOCK_AUTO_PULSE: u8 = 17;
pub const RA_UNLOCK_V_MEMORIES: u8 = 18;
pub const RA_UNLOCK_PEAK_GAMESPEED_DT: u8 = 19;
pub const RA_UNLOCK_ALL_GAMESPEED_GLYPHS: u8 = 20;
pub const RA_UNLOCK_INSTANT_EC: u8 = 21;
pub const RA_UNLOCK_AUTO_UNLOCK_DILATION: u8 = 22;
pub const RA_UNLOCK_V_XP: u8 = 23;
pub const RA_UNLOCK_HARD_V: u8 = 24;
pub const RA_UNLOCK_CONTINUOUS_TT_BOOST: u8 = 25;
pub const RA_UNLOCK_ACHIEVEMENT_TT_MULT: u8 = 26;
pub const RA_UNLOCK_ACHIEVEMENT_POWER: u8 = 27;

pub const RA_UNLOCK_COUNT: u8 = 28;

/// `(pet, required_level)` for each unlock id (index = id).
pub const RA_UNLOCK_REQS: [(usize, u32); RA_UNLOCK_COUNT as usize] = [
    (PET_TERESA, 1),   // 0 autoTP
    (PET_TERESA, 2),   // 1 chargedInfinityUpgrades
    (PET_TERESA, 5),   // 2 teresaXP
    (PET_TERESA, 10),  // 3 alteredGlyphs
    (PET_TERESA, 8),   // 4 effarigUnlock
    (PET_TERESA, 15),  // 5 perkShopIncrease
    (PET_TERESA, 25),  // 6 unlockDilationStartingTP
    (PET_EFFARIG, 1),  // 7 extraGlyphChoices
    (PET_EFFARIG, 2),  // 8 unlockGlyphAlchemy
    (PET_EFFARIG, 5),  // 9 effarigXP
    (PET_EFFARIG, 10), // 10 glyphEffectCount
    (PET_EFFARIG, 8),  // 11 enslavedUnlock
    (PET_EFFARIG, 15), // 12 relicShardGlyphLevelBoost
    (PET_EFFARIG, 25), // 13 maxGlyphRarity
    (PET_ENSLAVED, 1), // 14 blackHolePowerAutobuyers
    (PET_ENSLAVED, 2), // 15 improvedStoredTime
    (PET_ENSLAVED, 5), // 16 enslavedXP
    (PET_ENSLAVED, 10), // 17 autoPulseTime
    (PET_ENSLAVED, 8), // 18 vUnlock
    (PET_ENSLAVED, 15), // 19 peakGamespeedDT
    (PET_ENSLAVED, 25), // 20 allGamespeedGlyphs
    (PET_V, 1),        // 21 instantEC
    (PET_V, 2),        // 22 autoUnlockDilation
    (PET_V, 5),        // 23 vXP
    (PET_V, 6),        // 24 unlockHardV
    (PET_V, 10),       // 25 continuousTTBoost
    (PET_V, 15),       // 26 achievementTTMult
    (PET_V, 25),       // 27 achievementPower
];

/// One Celestial Memory (`player.celestials.ra.pets[i]`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RaPet {
    #[cfg_attr(feature = "serde", serde(default = "one_u32"))]
    pub level: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub memories: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub memory_chunks: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub memory_upgrades: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub chunk_upgrades: u32,
}

fn one_u32() -> u32 {
    1
}

impl Default for RaPet {
    fn default() -> Self {
        Self {
            level: 1,
            memories: 0.0,
            memory_chunks: 0.0,
            memory_upgrades: 0,
            chunk_upgrades: 0,
        }
    }
}

/// `player.celestials.ra`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RaState {
    #[cfg_attr(feature = "serde", serde(default))]
    pub pets: [RaPet; PET_COUNT],
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlock_bits: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub run: bool,
    /// Bitset of charged Infinity-Upgrade save-ids.
    #[cfg_attr(feature = "serde", serde(default))]
    pub charged: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub dis_charge: bool,
    #[cfg_attr(feature = "serde", serde(default = "one_f64"))]
    pub peak_gamespeed: f64,
    /// Pet index with Remembrance active (−1 = none).
    #[cfg_attr(feature = "serde", serde(default = "neg_one"))]
    pub pet_with_remembrance: i8,
    /// Real ms since Alchemy `momentum` unlocked (`momentumTime`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub momentum_time: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub alchemy: [super::alchemy::AlchemyResource; super::alchemy::ALCHEMY_COUNT],
    /// Per base resource cap ratchet (power/infinity/time/replication/dilation/
    /// effarig).
    #[cfg_attr(feature = "serde", serde(default))]
    pub highest_refinement_value: [f64; 6],
}

fn one_f64() -> f64 {
    1.0
}
fn neg_one() -> i8 {
    -1
}

impl Default for RaState {
    fn default() -> Self {
        Self::new()
    }
}

impl RaState {
    pub fn new() -> Self {
        Self {
            pets: Default::default(),
            unlock_bits: 0,
            run: false,
            charged: 0,
            dis_charge: false,
            peak_gamespeed: 1.0,
            pet_with_remembrance: -1,
            momentum_time: 0.0,
            alchemy: [super::alchemy::AlchemyResource::default(); super::alchemy::ALCHEMY_COUNT],
            highest_refinement_value: [0.0; 6],
        }
    }

    pub fn has_unlock(&self, id: u8) -> bool {
        self.unlock_bits & (1u32 << id) != 0
    }
}

impl GameState {
    // --- Availability -----------------------------------------------------------

    /// `Ra.isUnlocked` = `V.spaceTheorems >= 36` (the `raUnlock` VUnlock).
    pub fn ra_is_unlocked(&self) -> bool {
        self.v_space_theorems() >= 36
    }

    pub fn ra_is_running(&self) -> bool {
        self.celestials.ra.run
    }

    /// Whether Ra's Reality can be entered (Ra unlocked).
    pub fn ra_run_unlocked(&self) -> bool {
        self.ra_is_unlocked()
    }

    /// Whether unlock `id`'s bit is set (`RaUnlockState.isUnlocked`).
    pub fn ra_has_unlock(&self, id: u8) -> bool {
        self.celestials.ra.has_unlock(id)
    }

    /// `RaUnlockState.isEffectActive` — the bit is set and Pelle isn't disabling
    /// it (Pelle unbuilt, so identical to `ra_has_unlock`).
    pub fn ra_unlock_active(&self, id: u8) -> bool {
        self.ra_has_unlock(id)
    }

    // --- Pets -------------------------------------------------------------------

    /// Whether a pet's Memory is unlocked (teresa: always; others: their unlock).
    pub fn ra_pet_unlocked(&self, pet: usize) -> bool {
        match pet {
            PET_TERESA => true,
            PET_EFFARIG => self.ra_has_unlock(RA_UNLOCK_EFFARIG_MEMORIES),
            PET_ENSLAVED => self.ra_has_unlock(RA_UNLOCK_ENSLAVED_MEMORIES),
            PET_V => self.ra_has_unlock(RA_UNLOCK_V_MEMORIES),
            _ => false,
        }
    }

    /// `pet.level` — the stored level, or 0 if the pet isn't unlocked.
    pub fn ra_pet_level(&self, pet: usize) -> u32 {
        if self.ra_pet_unlocked(pet) {
            self.celestials.ra.pets[pet].level
        } else {
            0
        }
    }

    /// `Ra.totalPetLevel`.
    pub fn ra_total_pet_level(&self) -> u32 {
        (0..PET_COUNT).map(|p| self.ra_pet_level(p)).sum()
    }

    /// `Ra.requiredMemoriesForLevel(level)` — memories on `level` to reach the
    /// next (∞ at the cap).
    pub fn ra_required_memories_for_level(&self, level: u32) -> f64 {
        if level >= RA_LEVEL_CAP {
            return f64::INFINITY;
        }
        let l = level as f64;
        let adjusted = l + l * l / 10.0;
        let post15 = 1.5f64.powf((level as i64 - 15).max(0) as f64);
        (adjusted.powf(5.52) * post15 * 1e6).floor()
    }

    // --- Memory production ------------------------------------------------------

    /// `Ra.theoremBoostFactor` = min(10, max(0, log10(TT) − 350)/50).
    pub(crate) fn ra_theorem_boost_factor(&self) -> f64 {
        (((self.time_theorems.pos_log10() - 350.0).max(0.0)) / 50.0).min(10.0)
    }

    /// `continuousTTBoost.memoryChunks` / `.memories` = 1 + b/50 (1 if locked).
    fn ra_continuous_tt_memory_factor(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_CONTINUOUS_TT_BOOST) {
            1.0 + self.ra_theorem_boost_factor() / 50.0
        } else {
            1.0
        }
    }

    /// A pet's XP-unlock `memoryProductionMultiplier`.
    fn ra_memory_production_multiplier(&self, pet: usize) -> f64 {
        match pet {
            PET_TERESA if self.ra_unlock_active(RA_UNLOCK_TERESA_XP) => {
                1.0 + (self.reality.machines.pos_log10() / 100.0).sqrt()
            }
            PET_EFFARIG if self.ra_unlock_active(RA_UNLOCK_EFFARIG_XP) => {
                1.0 + self.records.best_reality.glyph_level as f64 / 7000.0
            }
            PET_ENSLAVED if self.ra_unlock_active(RA_UNLOCK_ENSLAVED_XP) => {
                1.0 + (self.records.total_time_played_ms.max(1.0)).log10() / 200.0
            }
            PET_V if self.ra_unlock_active(RA_UNLOCK_V_XP) => {
                1.0 + self.ra_total_pet_level() as f64 / 50.0
            }
            _ => 1.0,
        }
    }

    /// `Ra.productionPerMemoryChunk`.
    fn ra_production_per_memory_chunk(&self) -> f64 {
        // Achievement 168 is not in our wired set → ×1.
        let mut res = self.ra_continuous_tt_memory_factor();
        for pet in 0..PET_COUNT {
            if self.ra_pet_unlocked(pet) {
                res *= self.ra_memory_production_multiplier(pet);
            }
        }
        res
    }

    /// A pet's raw memory-chunk/second (`rawMemoryChunksPerSecond`).
    fn ra_raw_memory_chunks_per_second(&self, pet: usize) -> f64 {
        match pet {
            PET_TERESA => 4.0 * (self.eternity_points.pos_log10() / 1e4).powi(3),
            PET_EFFARIG => 4.0 * self.effarig_shards_gained().powf(0.1),
            PET_ENSLAVED => 4.0 * (self.time_shards.pos_log10() / 3e5).powi(2),
            PET_V => 4.0 * (self.infinity_power.pos_log10() / 1e7).powf(1.5),
            _ => 0.0,
        }
    }

    /// A pet's `memoryChunksPerSecond` (0 unless the pet is unlocked and Ra runs).
    fn ra_memory_chunks_per_second(&self, pet: usize) -> f64 {
        if !self.ra_pet_unlocked(pet) || !self.ra_is_running() {
            return 0.0;
        }
        let chunk_mult = 1.5f64.powi(self.celestials.ra.pets[pet].chunk_upgrades as i32);
        // GlyphSacrifice.reality (Reality-glyph sacrifice) is 1 in frontier.
        let mut res = self.ra_raw_memory_chunks_per_second(pet)
            * chunk_mult
            * self.ra_continuous_tt_memory_factor();
        // Remembrance.
        if self.celestials.ra.pet_with_remembrance == pet as i8 {
            res *= 5.0;
        } else if self.celestials.ra.pet_with_remembrance >= 0 {
            res *= 0.5;
        }
        res
    }

    /// `Ra.memoryTick(realDiff, generateChunks)` — accrue chunks + memories for
    /// every pet. `generate_chunks` is false while Enslaved stores real time.
    pub(crate) fn ra_memory_tick(&mut self, real_diff_ms: f64, generate_chunks: bool) {
        if !self.ra_is_unlocked() {
            return;
        }
        let seconds = real_diff_ms / 1000.0;
        let per_chunk = self.ra_production_per_memory_chunk();
        for pet in 0..PET_COUNT {
            let mcps = self.ra_memory_chunks_per_second(pet);
            let new_chunks = if generate_chunks { seconds * mcps } else { 0.0 };
            let mem_upgrade_mult =
                1.3f64.powi(self.celestials.ra.pets[pet].memory_upgrades as i32);
            let p = &self.celestials.ra.pets[pet];
            let new_memories =
                seconds * (p.memory_chunks + new_chunks / 2.0) * per_chunk * mem_upgrade_mult;
            let p = &mut self.celestials.ra.pets[pet];
            p.memory_chunks += new_chunks;
            p.memories += new_memories;
        }
    }

    // --- Upgrades / leveling ----------------------------------------------------

    pub fn ra_memory_upgrade_cost(&self, pet: usize) -> f64 {
        1000.0 * 5f64.powi(self.celestials.ra.pets[pet].memory_upgrades as i32)
    }
    pub fn ra_chunk_upgrade_cost(&self, pet: usize) -> f64 {
        5000.0 * 25f64.powi(self.celestials.ra.pets[pet].chunk_upgrades as i32)
    }
    fn ra_upgrade_cap(&self) -> f64 {
        0.5 * self.ra_required_memories_for_level(RA_LEVEL_CAP - 1)
    }
    pub fn ra_memory_upgrade_capped(&self, pet: usize) -> bool {
        self.ra_memory_upgrade_cost(pet) >= self.ra_upgrade_cap()
    }
    pub fn ra_chunk_upgrade_capped(&self, pet: usize) -> bool {
        self.ra_chunk_upgrade_cost(pet) >= self.ra_upgrade_cap()
    }

    pub fn ra_purchase_memory_upgrade(&mut self, pet: usize) -> bool {
        let cost = self.ra_memory_upgrade_cost(pet);
        if self.celestials.ra.pets[pet].memories < cost || self.ra_memory_upgrade_capped(pet) {
            return false;
        }
        self.celestials.ra.pets[pet].memories -= cost;
        self.celestials.ra.pets[pet].memory_upgrades += 1;
        true
    }

    pub fn ra_purchase_chunk_upgrade(&mut self, pet: usize) -> bool {
        let cost = self.ra_chunk_upgrade_cost(pet);
        if self.celestials.ra.pets[pet].memories < cost || self.ra_chunk_upgrade_capped(pet) {
            return false;
        }
        self.celestials.ra.pets[pet].memories -= cost;
        self.celestials.ra.pets[pet].chunk_upgrades += 1;
        true
    }

    /// `RaPetState.levelUp` — spend `requiredMemories`, level up, re-check
    /// unlocks. Returns whether it happened.
    pub fn ra_level_up(&mut self, pet: usize) -> bool {
        if !self.ra_pet_unlocked(pet) {
            return false;
        }
        let level = self.celestials.ra.pets[pet].level;
        let required = self.ra_required_memories_for_level(level);
        if self.celestials.ra.pets[pet].memories < required {
            return false;
        }
        self.celestials.ra.pets[pet].memories -= required;
        self.celestials.ra.pets[pet].level += 1;
        self.ra_check_for_unlocks();
        true
    }

    /// Level a pet up as many times as its memories allow (UI "level up" button).
    pub fn ra_level_up_max(&mut self, pet: usize) -> u32 {
        let mut count = 0;
        while self.ra_level_up(pet) {
            count += 1;
        }
        count
    }

    /// `Ra.checkForUnlocks` — gated on `VUnlocks.raUnlock`; flip every unlock
    /// whose pet has reached its level.
    pub(crate) fn ra_check_for_unlocks(&mut self) {
        if !self.ra_is_unlocked() {
            return;
        }
        for id in 0..RA_UNLOCK_COUNT {
            let (pet, level) = RA_UNLOCK_REQS[id as usize];
            if self.ra_pet_level(pet) >= level {
                self.celestials.ra.unlock_bits |= 1u32 << id;
            }
        }
    }

    // --- Remembrance ------------------------------------------------------------

    /// `Ra.remembrance.isUnlocked` — total pet level ≥ 20.
    pub fn ra_remembrance_unlocked(&self) -> bool {
        self.ra_total_pet_level() >= 20
    }

    /// Cycle Remembrance onto `pet` (or off if already set). No-op if locked.
    pub fn ra_set_remembrance(&mut self, pet: i8) -> bool {
        if !self.ra_remembrance_unlocked() {
            return false;
        }
        self.celestials.ra.pet_with_remembrance =
            if self.celestials.ra.pet_with_remembrance == pet { -1 } else { pet };
        true
    }

    // --- Charged Infinity Upgrades ----------------------------------------------

    /// `Ra.totalCharges` = min(12, ⌊teresa.level/2⌋) when unlocked.
    pub fn ra_total_charges(&self) -> u32 {
        if self.ra_unlock_active(RA_UNLOCK_CHARGED_INFINITY) {
            (self.ra_pet_level(PET_TERESA) / 2).min(12)
        } else {
            0
        }
    }

    pub fn ra_charges_used(&self) -> u32 {
        self.celestials.ra.charged.count_ones()
    }

    pub fn ra_charges_left(&self) -> u32 {
        self.ra_total_charges().saturating_sub(self.ra_charges_used())
    }

    /// Whether Infinity-Upgrade save-id `id` (0–15) is charged.
    pub fn ra_is_charged(&self, id: u8) -> bool {
        self.celestials.ra.charged & (1u16 << id) != 0
    }

    // --- Ra unlock effect readers (wired at engine sites) -----------------------

    /// `continuousTTBoost` exponent for a given base multiplier (10^(k·b) when
    /// unlocked, else 1). `k` per the data table.
    fn ra_continuous_tt_pow(&self, k: f64) -> Decimal {
        if self.ra_unlock_active(RA_UNLOCK_CONTINUOUS_TT_BOOST) {
            Decimal::pow10(k * self.ra_theorem_boost_factor())
        } else {
            Decimal::ONE
        }
    }

    /// `continuousTTBoost.infinity` (10^(15b)) — multiplies infinity generation.
    pub(crate) fn ra_tt_boost_infinities(&self) -> Decimal {
        self.ra_continuous_tt_pow(15.0)
    }
    /// `continuousTTBoost.eternity` (10^(2b)).
    pub(crate) fn ra_tt_boost_eternities(&self) -> Decimal {
        self.ra_continuous_tt_pow(2.0)
    }
    /// `continuousTTBoost.replicanti` (10^(20b)).
    pub(crate) fn ra_tt_boost_replicanti(&self) -> Decimal {
        self.ra_continuous_tt_pow(20.0)
    }
    /// `continuousTTBoost.dilatedTime` (10^(3b)).
    pub(crate) fn ra_tt_boost_dilated_time(&self) -> Decimal {
        self.ra_continuous_tt_pow(3.0)
    }
    /// `continuousTTBoost.ttGen` (10^(5b)).
    pub(crate) fn ra_tt_boost_tt_gen(&self) -> Decimal {
        self.ra_continuous_tt_pow(5.0)
    }
    /// `continuousTTBoost.autoPrestige` (1 + 2.4b). Deferred: auto-prestige
    /// speed is an automation hook not wired in frontier.
    #[allow(dead_code)]
    pub(crate) fn ra_tt_boost_auto_prestige(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_CONTINUOUS_TT_BOOST) {
            1.0 + 2.4 * self.ra_theorem_boost_factor()
        } else {
            1.0
        }
    }

    /// `achievementTTMult` — `Achievements.power` applied to TT generation
    /// (1 if not unlocked).
    pub(crate) fn ra_achievement_tt_mult(&self) -> Decimal {
        if self.ra_unlock_active(RA_UNLOCK_ACHIEVEMENT_TT_MULT) {
            self.achievement_power()
        } else {
            Decimal::ONE
        }
    }

    /// `achievementPower` — the extra `^1.5` on the achievement multiplier
    /// (1.0 = no change when not unlocked).
    pub(crate) fn ra_achievement_power_exponent(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_ACHIEVEMENT_POWER) {
            1.5
        } else {
            1.0
        }
    }

    /// Whether V's hard achievements + Triad studies are unlocked (`unlockHardV`).
    pub fn ra_hard_v_unlocked(&self) -> bool {
        self.ra_unlock_active(RA_UNLOCK_HARD_V)
    }

    /// `unlockHardV.effect` — a Triad study every 6 V levels.
    pub fn ra_triad_study_count(&self) -> u32 {
        if self.ra_hard_v_unlocked() {
            self.ra_pet_level(PET_V) / 6
        } else {
            0
        }
    }

    /// `peakGamespeedDT` — DT boost from peak game speed, `max((log10(peak)−90)³,
    /// 1)` (1 if not unlocked).
    pub(crate) fn ra_peak_gamespeed_dt(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_PEAK_GAMESPEED_DT) {
            (self.celestials.ra.peak_gamespeed.log10() - 90.0).powi(3).max(1.0)
        } else {
            1.0
        }
    }

    /// `improvedStoredTime.gameTimeAmplification` — `20^min(level,25)` (1 if
    /// not unlocked). Amplifies Enslaved stored game time. Deferred: the
    /// Enslaved amplification hook is out of frontier.
    #[allow(dead_code)]
    pub(crate) fn ra_stored_time_amplification(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_IMPROVED_STORED_TIME) {
            20f64.powi(self.ra_pet_level(PET_ENSLAVED).min(RA_LEVEL_CAP) as i32)
        } else {
            1.0
        }
    }

    /// `unlockDilationStartingTP` — TP gained as if reaching `√(total AM)` in
    /// Dilation (None if not unlocked). Deferred: the retroactive TP-gain path
    /// (`getTachyonGain` outside Dilation) is out of frontier.
    #[allow(dead_code)]
    pub(crate) fn ra_dilation_starting_tp_am(&self) -> Option<Decimal> {
        if self.ra_unlock_active(RA_UNLOCK_DILATION_STARTING_TP) {
            Some(self.total_antimatter.sqrt())
        } else {
            None
        }
    }

    /// `relicShardGlyphLevelBoost` — `100·log10(max(shardsGained,1))²` glyph-level
    /// bonus (0 if not unlocked).
    pub(crate) fn ra_relic_shard_glyph_level(&self) -> f64 {
        if self.ra_unlock_active(RA_UNLOCK_RELIC_SHARD_LEVEL) {
            100.0 * self.effarig_shards_gained().max(1.0).log10().powi(2)
        } else {
            0.0
        }
    }

    /// `Ra.momentumValue` — `min(1 + 0.005·hours, momentumCap)` (1 if momentum
    /// locked). Achievement 175 (× the growth rate) is unbuilt → ×1.
    pub(crate) fn ra_momentum_value(&self) -> f64 {
        if !self.alchemy_momentum_unlocked() {
            return 1.0;
        }
        let hours = self.celestials.ra.momentum_time / (1000.0 * 3600.0);
        (1.0 + 0.005 * hours).min(self.alchemy_momentum_cap())
    }

    // --- Per-tick / per-reality hooks -------------------------------------------

    /// Track the per-Reality peak game-speed factor and accrue momentum time.
    /// Called from `tick` on real time.
    pub(crate) fn ra_tick(&mut self, real_diff_ms: f64) {
        if !self.ra_is_unlocked() {
            return;
        }
        let speed = self.game_speed_factor();
        if speed > self.celestials.ra.peak_gamespeed {
            self.celestials.ra.peak_gamespeed = speed;
        }
        if self.alchemy_momentum_unlocked() {
            self.celestials.ra.momentum_time += real_diff_ms;
        }
    }

    /// `Ra.reset`-adjacent per-Reality bookkeeping: reset the peak game speed
    /// (the original resets `peakGamespeed` to 1 each Reality).
    pub(crate) fn ra_on_reality_reset(&mut self) {
        self.celestials.ra.peak_gamespeed = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ra_game() -> GameState {
        let mut game = GameState::new();
        game.reality.realities = 1;
        // Give Ra: 36 ST via V run-unlocks (6 easy done + hard don't matter).
        for i in 0..6 {
            game.celestials.v.run_unlocks[i] = 6;
        }
        // 6 easy achievements × 6 = 36 ST exactly.
        assert!(game.v_space_theorems() >= 36);
        game
    }

    #[test]
    fn ra_unlocks_at_36_space_theorems() {
        let game = ra_game();
        assert!(game.ra_is_unlocked());
        let mut locked = GameState::new();
        assert!(!locked.ra_is_unlocked());
    }

    #[test]
    fn leveling_teresa_unlocks_effarig_memory() {
        let mut game = ra_game();
        // Give teresa enough memories to reach level 8.
        game.celestials.ra.pets[PET_TERESA].memories = 1e30;
        let gained = game.ra_level_up_max(PET_TERESA);
        assert!(gained >= 7);
        assert!(game.ra_has_unlock(RA_UNLOCK_EFFARIG_MEMORIES));
        assert!(game.ra_pet_unlocked(PET_EFFARIG));
    }

    #[test]
    fn required_memories_grows_and_caps() {
        let game = ra_game();
        assert!(game.ra_required_memories_for_level(1) > 0.0);
        assert!(
            game.ra_required_memories_for_level(20)
                > game.ra_required_memories_for_level(10)
        );
        assert!(game.ra_required_memories_for_level(RA_LEVEL_CAP).is_infinite());
    }

    #[test]
    fn total_charges_scale_with_teresa_level() {
        let mut game = ra_game();
        game.celestials.ra.unlock_bits |= 1 << RA_UNLOCK_CHARGED_INFINITY;
        game.celestials.ra.pets[PET_TERESA].level = 10;
        assert_eq!(game.ra_total_charges(), 5);
        game.celestials.ra.pets[PET_TERESA].level = 25;
        assert_eq!(game.ra_total_charges(), 12); // capped
    }

    #[test]
    fn memory_tick_accrues_chunks_and_memories() {
        let mut game = ra_game();
        game.celestials.ra.run = true;
        game.eternity_points = Decimal::new(1.0, 40000); // huge EP → chunks
        game.ra_memory_tick(1000.0, true);
        assert!(game.celestials.ra.pets[PET_TERESA].memory_chunks > 0.0);
        // A second tick now also makes memories (from the accrued chunks).
        game.ra_memory_tick(1000.0, true);
        assert!(game.celestials.ra.pets[PET_TERESA].memories > 0.0);
    }

    #[test]
    fn remembrance_requires_total_level_20() {
        let mut game = ra_game();
        assert!(!game.ra_remembrance_unlocked());
        game.celestials.ra.unlock_bits |= 1 << RA_UNLOCK_EFFARIG_MEMORIES;
        game.celestials.ra.pets[PET_TERESA].level = 12;
        game.celestials.ra.pets[PET_EFFARIG].level = 8;
        assert!(game.ra_remembrance_unlocked());
        assert!(game.ra_set_remembrance(PET_TERESA as i8));
        assert_eq!(game.celestials.ra.pet_with_remembrance, PET_TERESA as i8);
    }
}
