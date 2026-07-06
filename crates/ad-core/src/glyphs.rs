//! Glyphs (Feature 6.2): equippable items granted on each Reality. Each glyph
//! has a type, level (from the reality's records), rarity/strength (seeded
//! RNG), and 1–4 effects; up to `3 + RU9 + RU24` can be equipped at once.
//!
//! Mirrors `src/core/glyphs/glyph-core.js` (inventory/equip),
//! `glyph-generator.js` (the seeded xorshift32 RNG, strength/effect rolls, and
//! the early-reality uniformity code), `glyph-purge-handler.js` (sacrifice),
//! `secret-formula/reality/glyph-effects.js` (effect formulas + combiners) and
//! `glyph-sacrifices.js`. The frontier covers the 5 basic types plus the
//! companion glyph; Effarig/reality/cursed glyphs, cosmetics, alteration, the
//! glyph filter, and alchemy are celestial content. See
//! `docs/design/2026-07-05-reality.md`.

use break_infinity::Decimal;

use crate::reality::GlyphLevel;
use crate::state::GameState;

/// Inventory capacity (`Glyphs.totalSlots`), 12 rows of 10.
pub const GLYPH_TOTAL_SLOTS: u32 = 120;

/// The strength cap (100% rarity).
const MAX_STRENGTH: f64 = 3.5;

/// Sacrifice totals above this stop improving effects
/// (`GlyphSacrificeHandler.maxSacrificeForEffects`).
const MAX_SACRIFICE_FOR_EFFECTS: f64 = 1e100;

/// `GlyphRNG.SECOND_GAUSSIAN_DEFAULT_VALUE` — "no cached deviate".
const SECOND_GAUSSIAN_DEFAULT: f64 = 1e6;

/// The five basic glyph types in `BASIC_GLYPH_TYPES` order, plus the one-off
/// companion. The order is load-bearing: the uniformity code and
/// `GlyphTypes.random` both index this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum GlyphType {
    Power,
    Infinity,
    Replication,
    Time,
    Dilation,
    Companion,
}

/// `BASIC_GLYPH_TYPES` (also the sacrifice-array index order).
pub const BASIC_GLYPH_TYPES: [GlyphType; 5] = [
    GlyphType::Power,
    GlyphType::Infinity,
    GlyphType::Replication,
    GlyphType::Time,
    GlyphType::Dilation,
];

impl GlyphType {
    /// The save-string id (`glyph.type`).
    pub fn save_id(self) -> &'static str {
        match self {
            GlyphType::Power => "power",
            GlyphType::Infinity => "infinity",
            GlyphType::Replication => "replication",
            GlyphType::Time => "time",
            GlyphType::Dilation => "dilation",
            GlyphType::Companion => "companion",
        }
    }

    pub fn from_save_id(id: &str) -> Option<Self> {
        Some(match id {
            "power" => GlyphType::Power,
            "infinity" => GlyphType::Infinity,
            "replication" => GlyphType::Replication,
            "time" => GlyphType::Time,
            "dilation" => GlyphType::Dilation,
            "companion" => GlyphType::Companion,
            _ => return None,
        })
    }

    /// Index into [`BASIC_GLYPH_TYPES`] / the sacrifice array.
    pub fn basic_index(self) -> Option<usize> {
        BASIC_GLYPH_TYPES.iter().position(|&t| t == self)
    }

    /// The generated-effect bitmask bits this type can roll, in the
    /// declaration order of `glyph-effects.js` (ascending per type).
    fn effect_bits(self) -> &'static [u8] {
        match self {
            GlyphType::Time => &[0, 1, 2, 3],
            GlyphType::Dilation => &[4, 5, 6, 7],
            GlyphType::Replication => &[8, 9, 10, 11],
            GlyphType::Infinity => &[12, 13, 14, 15],
            GlyphType::Power => &[16, 17, 18, 19],
            GlyphType::Companion => &[],
        }
    }

    /// The always-present primary effect bit (`primaryEffect`), if any.
    fn primary_effect_bit(self) -> Option<u8> {
        match self {
            GlyphType::Time => Some(0),
            GlyphType::Infinity => Some(12),
            GlyphType::Power => Some(16),
            _ => None,
        }
    }
}

/// `rarityToStrength` / `strengthToRarity`.
pub fn rarity_to_strength(rarity: f64) -> f64 {
    rarity * 2.5 / 100.0 + 1.0
}
pub fn strength_to_rarity(strength: f64) -> f64 {
    (strength - 1.0) * 100.0 / 2.5
}

/// A single glyph (`player.reality.glyphs.active[]/inventory[]` entries).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Glyph {
    /// Unique id (monotonic; used by the UI for drag/deletion).
    pub id: u32,
    /// Slot index: inventory 0–119 (first `10 × protected_rows` protected),
    /// active 0–4.
    pub idx: u32,
    /// Glyph type.
    pub kind: GlyphType,
    /// Strength (1.0–3.5; rarity = `(strength − 1) × 40`%). The companion
    /// glyph encodes the first reality's EP here.
    pub strength: f64,
    /// Effective level (instability-softcapped at creation).
    pub level: u32,
    /// Pre-instability level (display only).
    pub raw_level: u32,
    /// Effect bitmask. For the 5 basic types these are the generated-effect
    /// bit indices (0–19); the companion glyph uses the non-generated bits
    /// 8/9 (its effects are display-only).
    pub effects: u32,
}

/// `player.reality.glyphs` (modelled subset: no undo stack, sets, filter, or
/// cosmetics).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlyphState {
    /// Equipped glyphs (`idx` = active slot).
    pub active: Vec<Glyph>,
    /// Inventory glyphs (`idx` = inventory slot).
    pub inventory: Vec<Glyph>,
    /// Cumulative sacrifice value per basic type
    /// ([`BASIC_GLYPH_TYPES`] order).
    pub sac: [f64; 5],
    /// Protected inventory rows (top `protected_rows × 10` slots; new glyphs
    /// never land there). Default 2.
    pub protected_rows: u32,
}

impl GlyphState {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            inventory: Vec::new(),
            sac: [0.0; 5],
            protected_rows: 2,
        }
    }
}

impl Default for GlyphState {
    fn default() -> Self {
        Self::new()
    }
}

// --- The seeded RNG (glyph-generator.js / math.js) -----------------------------

/// JS `ToInt32` semantics for the save's `f64` seed.
fn js_to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    let m = (x.trunc() as i64).rem_euclid(1 << 32);
    if m >= 1 << 31 {
        (m - (1 << 32)) as i32
    } else {
        m as i32
    }
}

/// `xorshift32Update` with JS int32 semantics.
fn xorshift32_update(mut state: i32) -> i32 {
    state ^= state << 13;
    state ^= ((state as u32) >> 17) as i32;
    state ^= state << 5;
    state
}

/// The seeded glyph RNG (`GlyphRNG`): a 32-bit xorshift stream with a cached
/// Marsaglia-polar spare deviate. Constructed from the save's
/// `reality.seed`/`secondGaussian` and written back by
/// [`GlyphRng::finalize`] — the "real" RNG. Preview rolls simply drop it.
pub struct GlyphRng {
    state: i32,
    second_gaussian: f64,
}

impl GlyphRng {
    fn new(seed: f64, second_gaussian: f64) -> Self {
        Self {
            state: js_to_int32(seed),
            second_gaussian,
        }
    }

    /// Uniform in [0, 1): `state × 2^-32 + 0.5` on the signed state.
    fn uniform(&mut self) -> f64 {
        self.state = xorshift32_update(self.state);
        self.state as f64 * 2.328_306_436_538_696_3e-10 + 0.5
    }

    /// Standard normal via the Marsaglia polar method with a cached spare.
    fn normal(&mut self) -> f64 {
        if self.second_gaussian != SECOND_GAUSSIAN_DEFAULT {
            let spare = self.second_gaussian;
            self.second_gaussian = SECOND_GAUSSIAN_DEFAULT;
            return spare;
        }
        loop {
            let u = self.uniform() * 2.0 - 1.0;
            let v = self.uniform() * 2.0 - 1.0;
            let s = u * u + v * v;
            if s < 1.0 && s != 0.0 {
                let scale = (-2.0 * s.ln() / s).sqrt();
                self.second_gaussian = v * scale;
                return u * scale;
            }
        }
    }
}

/// Lehmer-code decode (`permutationIndex`): the permutation of `0..len` with
/// lexicographic index `lex_index % len!`.
pub(crate) fn permutation_index(len: usize, lex_index: i64) -> Vec<usize> {
    let mut num_perm: i64 = 1;
    for n in 1..=len as i64 {
        num_perm *= n;
    }
    let mut index = lex_index.rem_euclid(num_perm);
    let mut rem_order = num_perm / len as i64;
    let mut ordered: Vec<usize> = (0..len).collect();
    let mut perm = Vec::with_capacity(len);
    while !ordered.is_empty() {
        let div = (index / rem_order) as usize;
        let rem = index % rem_order;
        perm.push(ordered.remove(div));
        index = rem;
        if !ordered.is_empty() {
            rem_order /= ordered.len() as i64;
        }
    }
    perm
}

/// `orderedEffectList` restricted to the generated bits (0–19): the display /
/// removal order used by the uniformity code.
const ORDERED_GENERATED_BITS: [u8; 20] = [
    16, 12, 9, 0, 7, 17, 18, 19, 6, 15, 14, 3, 4, 10, 8, 2, 5, 13, 11, 1,
];

impl GameState {
    // --- Generation --------------------------------------------------------------

    /// `GlyphGenerator.strengthMultiplier`: ×1.3 with Reality Upgrade 16.
    fn glyph_strength_multiplier(&self) -> f64 {
        if self.reality_upgrade_bought(16) {
            1.3
        } else {
            1.0
        }
    }

    /// `gaussianBellCurve`: a polynomial approximation of
    /// `max(normal()+1, 1)^0.65`.
    fn gaussian_bell_curve(rng: &mut GlyphRng) -> f64 {
        let x = (rng.normal().abs() + 1.0).sqrt();
        -0.111_749_606_737
            + x * (0.900_603_878_243_551
                + x * (0.229_108_274_476_697 + x * -0.017_962_545_983_249))
    }

    /// `GlyphGenerator.randomStrength`. The relic-shard uniform is drawn even
    /// though its boost is celestial-only, to keep the stream aligned.
    fn random_strength(&self, rng: &mut GlyphRng) -> f64 {
        let mut result =
            Self::gaussian_bell_curve(rng) * self.glyph_strength_multiplier();
        let _relic_shard_factor = rng.uniform();
        // increasedRarity: Effarig / Achievement 146 / effarig sacrifice — all
        // out of frontier (0).
        result = (result * 400.0).ceil() / 400.0;
        result.min(MAX_STRENGTH)
    }

    /// `GlyphGenerator.randomNumberOfEffects`. Draws two uniforms up-front to
    /// keep the stream aligned; RU17 grants a 50% chance of one extra.
    fn random_number_of_effects(
        &self,
        strength: f64,
        level: u32,
        rng: &mut GlyphRng,
    ) -> u32 {
        let random1 = rng.uniform();
        let random2 = rng.uniform();
        let mut num = (4.0f64).min(
            (random1.powf(1.0 - (level as f64 * strength).sqrt() / 100.0) * 1.5 + 1.0)
                .floor(),
        ) as u32;
        if self.reality_upgrade_bought(17) && random2 < 0.5 {
            num = (num + 1).min(4);
        }
        num
    }

    /// `GlyphGenerator.generateEffects`: weight each possible effect with a
    /// uniform (padding the draw count to 7), force the primary effect, and
    /// keep the top `count`.
    fn generate_effects(kind: GlyphType, count: u32, rng: &mut GlyphRng) -> u32 {
        let bits = kind.effect_bits();
        let mut weighted: Vec<(u8, f64)> =
            bits.iter().map(|&b| (b, rng.uniform())).collect();
        for _ in 0..(7 - bits.len()) {
            rng.uniform();
        }
        if let Some(primary) = kind.primary_effect_bit() {
            for entry in weighted.iter_mut() {
                if entry.0 == primary {
                    entry.1 = 2.0;
                }
            }
        }
        // Stable sort by weight descending (JS sorts the ascending-bit key
        // list), then take the top `count`.
        weighted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        weighted
            .iter()
            .take(count as usize)
            .fold(0u32, |mask, &(bit, _)| mask | (1 << bit))
    }

    /// `GlyphGenerator.randomType`: uniform over the basic types, excluding
    /// the most-repeated types so far.
    fn random_type(rng: &mut GlyphRng, types_so_far: &[GlyphType]) -> GlyphType {
        let count = |t: GlyphType| types_so_far.iter().filter(|&&x| x == t).count();
        let max_count = BASIC_GLYPH_TYPES.iter().map(|&t| count(t)).max().unwrap();
        let candidates: Vec<GlyphType> = if types_so_far.is_empty() {
            BASIC_GLYPH_TYPES.to_vec()
        } else {
            BASIC_GLYPH_TYPES
                .iter()
                .copied()
                .filter(|&t| count(t) != max_count)
                .collect()
        };
        // `GlyphTypes.random` over the non-blacklisted types; an empty list
        // can't happen for 4 draws over 5 types, but fall back to all.
        let pool = if candidates.is_empty() {
            BASIC_GLYPH_TYPES.to_vec()
        } else {
            candidates
        };
        pool[(rng.uniform() * pool.len() as f64) as usize]
    }

    /// `GlyphGenerator.randomGlyph` (type always supplied on our paths).
    fn random_glyph(
        &self,
        level: GlyphLevel,
        rng: &mut GlyphRng,
        kind: GlyphType,
    ) -> Glyph {
        let strength = self.random_strength(rng);
        let num_effects =
            self.random_number_of_effects(strength, level.actual_level, rng);
        let effects = Self::generate_effects(kind, num_effects, rng);
        Glyph {
            id: 0,
            idx: 0,
            kind,
            strength,
            level: level.actual_level.max(1),
            raw_level: level.raw_level,
            effects,
        }
    }

    /// `GlyphGenerator.uniformGlyphs`: the first 20 post-first realities
    /// spread types and effects deterministically (driven by the initial
    /// seed) so early RNG can't repeatedly starve a type.
    fn uniform_glyphs(
        &self,
        level: GlyphLevel,
        rng: &mut GlyphRng,
        reality_count: u32,
    ) -> Vec<Glyph> {
        let group_num = (reality_count as i64 - 1) / 5;
        let group_index = (reality_count as i64 - 1) % 5;
        let init_seed = self.reality.initial_seed as i64;

        let type_perm =
            permutation_index(5, (31 + init_seed % 7) * group_num + init_seed % 1123);

        let mut type_perm_index = [0usize; 5];
        for &perm_type in type_perm.iter().take(group_index as usize) {
            for (t, slot) in type_perm_index.iter_mut().enumerate() {
                if t != perm_type {
                    *slot += 1;
                }
            }
        }

        // The lowest effect-bit of each type, in BASIC_GLYPH_TYPES order.
        let start_id = [16u8, 12, 8, 0, 4];
        let mut types_this_reality: Vec<usize> = (0..5).collect();
        types_this_reality.remove(type_perm[group_index as usize]);

        let mut uniform_effects = Vec::with_capacity(4);
        for &ty in types_this_reality.iter() {
            let effect_perm = permutation_index(
                4,
                5 * ty as i64 + (7 + init_seed % 5) * group_num + init_seed % 11,
            );
            uniform_effects.push(start_id[ty] + effect_perm[type_perm_index[ty]] as u8);
        }

        let max_effects = if self.reality_upgrade_bought(17) {
            3
        } else {
            2
        };
        let mut glyphs = Vec::with_capacity(4);
        for i in 0..4usize {
            let ty = types_this_reality[i];
            let mut glyph = self.random_glyph(level, rng, BASIC_GLYPH_TYPES[ty]);
            let new_mask = if (init_seed + reality_count as i64 + i as i64) % 2 == 0 {
                1u32 << uniform_effects[i]
            } else {
                glyph.effects | (1u32 << uniform_effects[i])
            };
            if new_mask.count_ones() > max_effects {
                // Deterministically drop one removable (non-primary) effect
                // of the glyph's *original* mask.
                let replaceable: Vec<u8> = ORDERED_GENERATED_BITS
                    .iter()
                    .copied()
                    .filter(|&b| glyph.effects & (1 << b) != 0)
                    .filter(|&b| b != 0 && b != 12 && b != 16)
                    .collect();
                if replaceable.is_empty() {
                    glyph.effects = new_mask;
                } else {
                    let to_remove = replaceable[((init_seed + reality_count as i64)
                        .abs()
                        % replaceable.len() as i64)
                        as usize];
                    glyph.effects = new_mask & !(1u32 << to_remove);
                }
            } else {
                glyph.effects = new_mask;
            }

            // Re-add the primary power effect (the mask reset drops it half
            // the time).
            if let Some(primary) = glyph.kind.primary_effect_bit() {
                glyph.effects |= 1 << primary;
            }
            glyphs.push(glyph);
        }
        glyphs
    }

    /// `GlyphSelection.glyphUncommonGuarantee`: if no choice is uncommon
    /// (strength ≥ 1.5), one random choice gets an uncommon re-roll.
    fn glyph_uncommon_guarantee(&self, glyphs: &mut [Glyph], rng: &mut GlyphRng) {
        const STRENGTH_THRESHOLD: f64 = 1.5;
        let random = rng.uniform();
        let mut new_strength;
        loop {
            new_strength = self.random_strength(rng);
            if new_strength >= STRENGTH_THRESHOLD {
                break;
            }
        }
        if glyphs.iter().any(|g| g.strength >= STRENGTH_THRESHOLD) {
            return;
        }
        let index = (random * glyphs.len() as f64) as usize;
        glyphs[index].strength = new_strength;
    }

    /// `GlyphSelection.glyphList`: generate the choice list (always rolling at
    /// least 4 to keep the RNG stream stable, then truncating).
    fn glyph_list(
        &self,
        count_in: usize,
        level: GlyphLevel,
        rng: &mut GlyphRng,
    ) -> Vec<Glyph> {
        let count = count_in.max(4);
        // Uniformity is active for the first 20 (post-first) realities.
        let mut glyphs = if self.reality.realities <= 20 {
            self.uniform_glyphs(level, rng, self.reality.realities)
        } else {
            let mut types: Vec<GlyphType> = Vec::with_capacity(count);
            for _ in 0..count {
                types.push(Self::random_type(rng, &types));
            }
            types
                .iter()
                .map(|&t| self.random_glyph(level, rng, t))
                .collect()
        };
        self.glyph_uncommon_guarantee(&mut glyphs, rng);
        glyphs.truncate(count_in);
        glyphs
    }

    /// How many glyph choices a Reality offers (`GlyphSelection.choiceCount`):
    /// 4 with the START perk, 1 before it.
    pub fn glyph_choice_count(&self) -> usize {
        if self.perk_bought(0) {
            4
        } else {
            1
        }
    }

    /// The glyph choices the next Reality will offer, without advancing the
    /// RNG (`GlyphSelection.upcomingGlyphs`). Before the START perk this is
    /// the single deterministic pick; on the very first Reality it is the
    /// fixed starting glyph.
    pub fn upcoming_glyphs(&self) -> Vec<Glyph> {
        if self.reality.realities == 0 {
            return vec![self.starting_glyph()];
        }
        let mut rng = GlyphRng::new(self.reality.seed, self.reality.second_gaussian);
        let level = self.gained_glyph_level();
        if self.perk_bought(0) {
            self.glyph_list(self.glyph_choice_count(), level, &mut rng)
        } else {
            let group = self.glyph_list(4, level, &mut rng);
            vec![group[self.glyph_index_without_start()].clone()]
        }
    }

    /// `GlyphSelection.indexWithoutSTART`: the deterministic pick used before
    /// the START perk exists.
    fn glyph_index_without_start(&self) -> usize {
        let lex =
            self.reality.realities as i64 * ((self.reality.initial_seed as i64) % 5 + 3);
        permutation_index(4, lex)[0]
    }

    /// The fixed first-reality glyph (`GlyphGenerator.startingGlyph`).
    fn starting_glyph(&self) -> Glyph {
        let level = self.gained_glyph_level();
        Glyph {
            id: 0,
            idx: 0,
            kind: GlyphType::Power,
            strength: 1.5,
            level: level.actual_level.max(1),
            raw_level: level.raw_level,
            effects: 1 << 16, // powerpow
        }
    }

    /// The companion glyph (`GlyphGenerator.companionGlyph`): its rarity
    /// encodes the pre-Reality EP.
    fn companion_glyph(&self, eternity_points: Decimal) -> Glyph {
        let strength = rarity_to_strength(eternity_points.pos_log10() / 1e6);
        Glyph {
            id: 0,
            idx: 0,
            kind: GlyphType::Companion,
            strength,
            level: 1,
            raw_level: 1,
            effects: (1 << 8) | (1 << 9), // companiondescription | companionEP
        }
    }

    // --- The Reality glyph grant ---------------------------------------------------

    /// Perform a Reality picking glyph choice `choice` (only meaningful with
    /// the START perk; `None` = first choice). `sacrifice_choice` sends the
    /// picked glyph straight to sacrifice/deletion instead of the inventory.
    pub fn reality_with_glyph_choice(
        &mut self,
        choice: Option<usize>,
        sacrifice_choice: bool,
    ) -> bool {
        if !self.is_reality_available() {
            return false;
        }
        if self.reality.realities == 0 {
            self.reality.seed = self.reality.initial_seed;
        }
        self.grant_glyphs_on_reality(choice, sacrifice_choice);
        self.finish_process_reality();
        true
    }

    /// An automatic Reality (`autoReality()` → `processAutoGlyph`): used by
    /// the Reality autobuyer (and later the Automator's `reality` command).
    /// Generates `choiceCount` glyphs and keeps the first — with the START
    /// perk this matches the manual no-modal path; without it the original's
    /// auto path generates a single glyph (unlike the manual 4-then-pick), and
    /// we mirror that. The Effarig glyph-filter branch is out of frontier.
    pub fn auto_reality(&mut self) -> bool {
        if !self.is_reality_available() {
            return false;
        }
        if self.reality.realities == 0 {
            // Defensive: the autobuyer needs RU25, so a zeroth reality can't
            // normally happen here; fall back to the standard first-reality
            // grant (starting + companion glyphs).
            return self.reality_with_glyph_choice(None, false);
        }

        let mut rng = GlyphRng::new(self.reality.seed, self.reality.second_gaussian);
        let level = self.gained_glyph_level();
        let glyphs = self.glyph_list(self.glyph_choice_count(), level, &mut rng);
        self.reality.seed = rng.state as f64;
        self.reality.second_gaussian = rng.second_gaussian;

        let picked = glyphs[0].clone();
        if self.glyph_free_inventory_space() == 0 {
            self.sacrifice_or_delete(&picked);
        } else {
            self.add_glyph_to_inventory(picked);
        }
        self.finish_process_reality();
        true
    }

    /// The glyph half of `processManualReality`.
    pub(crate) fn grant_glyphs_on_reality(
        &mut self,
        choice: Option<usize>,
        sacrifice_choice: bool,
    ) {
        if self.reality.realities == 0 {
            // First reality: the fixed starting glyph + the companion.
            let starting = self.starting_glyph();
            self.add_glyph_to_inventory(starting);
            let companion = self.companion_glyph(self.eternity_points);
            self.add_glyph_to_inventory(companion);
            return;
        }

        let mut rng = GlyphRng::new(self.reality.seed, self.reality.second_gaussian);
        let level = self.gained_glyph_level();
        let (glyphs, index) = if self.perk_bought(0) {
            let glyphs = self.glyph_list(self.glyph_choice_count(), level, &mut rng);
            let index = choice.unwrap_or(0).min(glyphs.len() - 1);
            (glyphs, index)
        } else {
            let glyphs = self.glyph_list(4, level, &mut rng);
            (glyphs, self.glyph_index_without_start())
        };
        // Finalize the RNG (`rng.finalize()`): write the stream back out.
        self.reality.seed = rng.state as f64;
        self.reality.second_gaussian = rng.second_gaussian;

        let picked = glyphs[index].clone();
        if sacrifice_choice || self.glyph_free_inventory_space() == 0 {
            self.sacrifice_or_delete(&picked);
        } else {
            self.add_glyph_to_inventory(picked);
        }
    }

    // --- Inventory ----------------------------------------------------------------

    /// Highest glyph id in use (`GlyphGenerator.maxID` analogue).
    fn max_glyph_id(&self) -> u32 {
        self.reality
            .glyphs
            .active
            .iter()
            .chain(self.reality.glyphs.inventory.iter())
            .map(|g| g.id)
            .max()
            .unwrap_or(0)
    }

    /// First free unprotected inventory slot, if any.
    fn find_free_inventory_slot(&self) -> Option<u32> {
        let protected = self.reality.glyphs.protected_rows * 10;
        (protected..GLYPH_TOTAL_SLOTS)
            .find(|&slot| !self.reality.glyphs.inventory.iter().any(|g| g.idx == slot))
    }

    /// Free unprotected inventory slots (`GameCache.glyphInventorySpace`).
    pub fn glyph_free_inventory_space(&self) -> u32 {
        let protected = self.reality.glyphs.protected_rows * 10;
        let used = self
            .reality
            .glyphs
            .inventory
            .iter()
            .filter(|g| g.idx >= protected)
            .count() as u32;
        GLYPH_TOTAL_SLOTS - protected - used
    }

    /// Add a glyph to the first free unprotected slot (assigning a fresh id).
    pub(crate) fn add_glyph_to_inventory(&mut self, mut glyph: Glyph) {
        let Some(slot) = self.find_free_inventory_slot() else {
            return;
        };
        glyph.id = self.max_glyph_id() + 1;
        glyph.idx = slot;
        self.records.best_reality.glyph_strength =
            self.records.best_reality.glyph_strength.max(glyph.strength);
        self.reality.glyphs.inventory.push(glyph);
    }

    /// Number of equipped glyph slots (`Glyphs.activeSlotCount`):
    /// `3 + RU9 + RU24`.
    pub fn glyph_active_slot_count(&self) -> usize {
        3 + usize::from(self.reality_upgrade_bought(9))
            + usize::from(self.reality_upgrade_bought(24))
    }

    /// Equipped glyphs excluding the companion.
    pub(crate) fn active_glyphs_without_companion(&self) -> Vec<&Glyph> {
        self.reality
            .glyphs
            .active
            .iter()
            .filter(|g| g.kind != GlyphType::Companion)
            .collect()
    }

    /// Equip inventory glyph `id` into `target_slot`. Equipping is one-way
    /// mid-reality (unequip happens via the Reality respec). Fails into a
    /// no-op if the slot is taken or out of range.
    pub fn equip_glyph(&mut self, id: u32, target_slot: u32) -> bool {
        if target_slot as usize >= self.glyph_active_slot_count() {
            return false;
        }
        if self
            .reality
            .glyphs
            .active
            .iter()
            .any(|g| g.idx == target_slot)
        {
            return false;
        }
        let Some(pos) = self
            .reality
            .glyphs
            .inventory
            .iter()
            .position(|g| g.id == id)
        else {
            return false;
        };
        let mut glyph = self.reality.glyphs.inventory.remove(pos);
        glyph.idx = target_slot;
        self.reality.glyphs.active.push(glyph);
        // `Glyphs.updateMaxGlyphCount`.
        let count = self.active_glyphs_without_companion().len() as i32;
        self.requirement_checks.reality_max_glyphs =
            self.requirement_checks.reality_max_glyphs.max(count);
        true
    }

    /// Unequip every equipped glyph into free inventory slots
    /// (`Glyphs.unequipAll`, the respec path).
    pub(crate) fn unequip_all_glyphs_impl(&mut self) {
        while let Some(glyph) = self.reality.glyphs.active.pop() {
            let Some(slot) = self.find_free_inventory_slot() else {
                // No space: put it back and stop (the original leaves it
                // equipped).
                self.reality.glyphs.active.push(glyph);
                break;
            };
            let mut glyph = glyph;
            glyph.idx = slot;
            self.reality.glyphs.inventory.push(glyph);
        }
    }

    /// Move an inventory glyph to a specific empty inventory slot (UI
    /// drag-and-drop / protected-row management).
    pub fn move_glyph_to_slot(&mut self, id: u32, target_slot: u32) -> bool {
        if target_slot >= GLYPH_TOTAL_SLOTS {
            return false;
        }
        if self
            .reality
            .glyphs
            .inventory
            .iter()
            .any(|g| g.idx == target_slot)
        {
            return false;
        }
        let Some(glyph) = self
            .reality
            .glyphs
            .inventory
            .iter_mut()
            .find(|g| g.id == id)
        else {
            return false;
        };
        glyph.idx = target_slot;
        true
    }

    // --- Sacrifice (glyph-purge-handler.js) ----------------------------------------

    /// Whether sacrifice is unlocked (`GlyphSacrificeHandler.canSacrifice`,
    /// Reality Upgrade 19).
    pub fn can_sacrifice_glyphs(&self) -> bool {
        self.reality_upgrade_bought(19)
    }

    /// The sacrifice value of a glyph (`glyphSacrificeGain`); 0 before RU19.
    pub fn glyph_sacrifice_gain(&self, glyph: &Glyph) -> f64 {
        if !self.can_sacrifice_glyphs() || glyph.kind == GlyphType::Companion {
            return 0.0;
        }
        let level = glyph.level as f64;
        let pre10k = (level.min(10_000.0) + 10.0).powf(2.5);
        let post10k = 1.0 + (level - 10_000.0).max(0.0) / 100.0;
        // Teresa's `runRewardMultiplier` (glyph-sacrifice bonus from the best
        // Teresa run); the Ra rarity/shard power is out of frontier.
        pre10k * post10k * glyph.strength * self.teresa_run_reward_multiplier()
    }

    /// Sacrifice (with RU19) or simply delete an inventory glyph by id.
    pub fn sacrifice_glyph(&mut self, id: u32) -> bool {
        let Some(pos) = self
            .reality
            .glyphs
            .inventory
            .iter()
            .position(|g| g.id == id)
        else {
            return false;
        };
        let glyph = self.reality.glyphs.inventory[pos].clone();
        if glyph.kind == GlyphType::Companion {
            // The companion can be deleted (with heartbreak) but never adds
            // sacrifice value.
            self.reality.glyphs.inventory.remove(pos);
            return true;
        }
        if let (true, Some(index)) =
            (self.can_sacrifice_glyphs(), glyph.kind.basic_index())
        {
            self.reality.glyphs.sac[index] += self.glyph_sacrifice_gain(&glyph);
        }
        self.reality.glyphs.inventory.remove(pos);
        true
    }

    /// `AutoGlyphProcessor.getRidOfGlyph` for a not-yet-added glyph (the
    /// reality pick when sacrificing or out of space).
    fn sacrifice_or_delete(&mut self, glyph: &Glyph) {
        if self.can_sacrifice_glyphs() {
            if let Some(index) = glyph.kind.basic_index() {
                self.reality.glyphs.sac[index] += self.glyph_sacrifice_gain(glyph);
            }
        }
    }

    /// The capped sacrifice total for a basic type's effect.
    fn sac_capped(&self, kind: GlyphType) -> f64 {
        let index = kind.basic_index().expect("basic type");
        self.reality.glyphs.sac[index].min(MAX_SACRIFICE_FOR_EFFECTS)
    }

    /// Power sacrifice: Distant Galaxy scaling starts this many later.
    pub fn glyph_sac_power_effect(&self) -> u32 {
        let sac = self.sac_capped(GlyphType::Power);
        let base = (sac + 1.0).log10() / MAX_SACRIFICE_FOR_EFFECTS.log10();
        (750.0 * base.powf(1.2)).floor() as u32
    }

    /// Infinity sacrifice: multiplier on the 8th Infinity Dimension's
    /// per-purchase multiplier.
    pub fn glyph_sac_infinity_effect(&self) -> f64 {
        let sac = self.sac_capped(GlyphType::Infinity);
        1.0 + (1.0 + sac.powf(0.2) / 100.0).log10()
    }

    /// Time sacrifice: multiplier on the 8th Time Dimension's per-purchase
    /// multiplier.
    pub fn glyph_sac_time_effect(&self) -> f64 {
        let sac = self.sac_capped(GlyphType::Time);
        (1.0 + sac.powf(0.2) / 100.0).powi(2)
    }

    /// Replication sacrifice: Replicanti Galaxy scaling starts this many
    /// later.
    pub fn glyph_sac_replication_effect(&self) -> u32 {
        let sac = self.sac_capped(GlyphType::Replication);
        let base = (sac + 1.0).log10() / MAX_SACRIFICE_FOR_EFFECTS.log10();
        (1500.0 * base.powf(1.2)).floor() as u32
    }

    /// Dilation sacrifice: Tachyon Particle gain multiplier.
    pub fn glyph_sac_dilation_effect(&self) -> f64 {
        let sac = self.sac_capped(GlyphType::Dilation);
        let exponent =
            0.32 * ((sac + 1.0).log10() / MAX_SACRIFICE_FOR_EFFECTS.log10()).powf(0.1);
        sac.max(1.0).powf(exponent)
    }

    // --- Effects (glyph-effects.js) --------------------------------------------------

    /// `getAdjustedGlyphLevel`: a glyph's *effective* level for effect
    /// computation. Inside a celestial run the level is clamped — Enslaved
    /// raises it to a 5000 minimum, Effarig caps it at the current stage's cap.
    /// (Pelle's cap + the Reality-glyph level boost are out of frontier.)
    pub(crate) fn adjusted_glyph_level(&self, glyph: &Glyph) -> f64 {
        let level = glyph.level as f64;
        if self.celestials.enslaved.run {
            return level.max(crate::celestials::enslaved::GLYPH_LEVEL_MIN as f64);
        }
        if self.celestials.effarig.run {
            return level.min(self.effarig_glyph_level_cap() as f64);
        }
        level
    }

    /// Effect values of `bit` across the equipped basic-type glyphs.
    fn glyph_effect_values(&self, bit: u8) -> Vec<(f64, f64)> {
        self.reality
            .glyphs
            .active
            .iter()
            .filter(|g| g.kind != GlyphType::Companion)
            .filter(|g| g.effects & (1 << bit) != 0)
            .map(|g| (self.adjusted_glyph_level(g), g.strength))
            .collect()
    }

    /// `GlyphCombiner.addExponents`: Σx + (1 − n) (neutral 1).
    fn combine_add_exponents(values: &[f64]) -> f64 {
        values.iter().sum::<f64>() + 1.0 - values.len() as f64
    }

    /// timepow (bit 0): TD multipliers `^x`.
    pub fn glyph_effect_timepow(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(0)
            .iter()
            .map(|&(l, s)| 1.01 + l.powf(0.32) * s.powf(0.45) / 75.0)
            .collect();
        Self::combine_add_exponents(&v)
    }

    /// timespeed (bit 1): game speed `×x`.
    pub fn glyph_effect_timespeed(&self) -> f64 {
        self.glyph_effect_values(1)
            .iter()
            .map(|&(l, s)| 1.0 + l.powf(0.3) * s.powf(0.65) / 20.0)
            .product()
    }

    /// timeetermult (bit 2): Eternity gain `×x`.
    pub fn glyph_effect_timeetermult(&self) -> f64 {
        self.glyph_effect_values(2)
            .iter()
            .map(|&(l, s)| ((s + 3.0) * l).powf(0.9))
            .product()
    }

    /// timeEP (bit 3): EP gain `×x`.
    pub fn glyph_effect_time_ep(&self) -> f64 {
        self.glyph_effect_values(3)
            .iter()
            .map(|&(l, s)| (l * s).powi(3) * 100.0)
            .product()
    }

    /// dilationDT (bit 4): DT gain `×x` (Decimal).
    pub fn glyph_effect_dilation_dt(&self) -> Decimal {
        self.glyph_effect_values(4)
            .iter()
            .map(|&(l, s)| {
                Decimal::from_float(l * s).pow(&Decimal::from_float(1.5))
                    * Decimal::from_float(2.0)
            })
            .fold(Decimal::ONE, |acc, x| acc * x)
    }

    /// dilationgalaxyThreshold (bit 5): TG threshold `×x`, softcapped below
    /// 0.4.
    pub fn glyph_effect_dilation_galaxy_threshold(&self) -> f64 {
        let prod: f64 = self
            .glyph_effect_values(5)
            .iter()
            .map(|&(l, s)| 1.0 - l.powf(0.17) * s.powf(0.35) / 100.0)
            .product();
        if prod < 0.4 {
            0.4 - (0.4 - prod).powf(1.7)
        } else {
            prod
        }
    }

    /// dilationTTgen (bit 6): TT per second.
    pub fn glyph_effect_dilation_ttgen(&self) -> f64 {
        self.glyph_effect_values(6)
            .iter()
            .map(|&(l, s)| (l * s).sqrt() / 10_000.0)
            .sum()
    }

    /// dilationpow (bit 7): AD multipliers `^x` while dilated.
    pub fn glyph_effect_dilationpow(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(7)
            .iter()
            .map(|&(l, s)| 1.1 + l.powf(0.7) * s.powf(0.7) / 25.0)
            .collect();
        Self::combine_add_exponents(&v)
    }

    /// replicationspeed (bit 8): replication speed `×x` (Decimal).
    pub fn glyph_effect_replicationspeed(&self) -> Decimal {
        self.glyph_effect_values(8)
            .iter()
            .map(|&(l, s)| Decimal::from_float(l * s * 3.0))
            .fold(Decimal::ONE, |acc, x| acc * x)
    }

    /// replicationpow (bit 9): replicanti multiplier `^x`.
    pub fn glyph_effect_replicationpow(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(9)
            .iter()
            .map(|&(l, s)| 1.1 + l.sqrt() * s / 25.0)
            .collect();
        Self::combine_add_exponents(&v)
    }

    /// replicationdtgain (bit 10): DT multiplier per 1e10000 replicanti (the
    /// special relative-stacking combiner).
    pub fn glyph_effect_replicationdtgain(&self) -> f64 {
        let values: Vec<f64> = self
            .glyph_effect_values(10)
            .iter()
            .map(|&(l, s)| 0.0003 * l.powf(0.3) * s.powf(0.65))
            .collect();
        if values.is_empty() {
            0.0
        } else {
            values.iter().product::<f64>() * 0.0001f64.powi(1 - values.len() as i32)
        }
    }

    /// replicationglyphlevel (bit 11): replicanti exponent for glyph level,
    /// with the diminishing-stack softcap.
    pub fn glyph_effect_replicationglyphlevel_impl(&self) -> f64 {
        let values: Vec<f64> = self
            .glyph_effect_values(11)
            .iter()
            .map(|&(l, s)| (l.powf(0.25) * s.powf(0.4)).sqrt() / 50.0)
            .collect();
        let mut sum: f64 = values.iter().sum();
        if values.len() > 2 {
            sum *= 6.0 / (values.len() as f64 + 4.0);
        }
        if sum > 0.1 {
            0.1 + 0.2 * (sum - 0.1)
        } else {
            sum
        }
    }

    /// infinitypow (bit 12): ID multipliers `^x`.
    pub fn glyph_effect_infinitypow(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(12)
            .iter()
            .map(|&(l, s)| 1.007 + l.powf(0.21) * s.powf(0.4) / 75.0)
            .collect();
        Self::combine_add_exponents(&v)
    }

    /// infinityrate (bit 13): infinity-power conversion `+x` on the `^7`.
    pub fn glyph_effect_infinityrate(&self) -> f64 {
        self.glyph_effect_values(13)
            .iter()
            .map(|&(l, s)| l.powf(0.2) * s.powf(0.4) * 0.04)
            .sum()
    }

    /// infinityIP (bit 14): IP gain `×x`.
    pub fn glyph_effect_infinity_ip(&self) -> f64 {
        self.glyph_effect_values(14)
            .iter()
            .map(|&(l, s)| (l * (s + 1.0)).powi(6) * 10_000.0)
            .product()
    }

    /// infinityinfmult (bit 15): Infinity gain `×x` (Decimal).
    pub fn glyph_effect_infinityinfmult(&self) -> Decimal {
        self.glyph_effect_values(15)
            .iter()
            .map(|&(l, s)| {
                Decimal::from_float(l * s).pow(&Decimal::from_float(1.5))
                    * Decimal::from_float(2.0)
            })
            .fold(Decimal::ONE, |acc, x| acc * x)
    }

    /// powerpow (bit 16): AD multipliers `^x`.
    pub fn glyph_effect_powerpow(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(16)
            .iter()
            .map(|&(l, s)| 1.015 + l.powf(0.2) * s.powf(0.4) / 75.0)
            .collect();
        Self::combine_add_exponents(&v)
    }

    /// powermult (bit 17): AD multipliers `×x` (Decimal; computed in log
    /// space — `(10·l·s)^(10·l·s)` overflows f64 fast).
    pub fn glyph_effect_powermult(&self) -> Decimal {
        self.glyph_effect_values(17)
            .iter()
            .map(|&(l, s)| {
                let base = l * s * 10.0;
                if base <= 0.0 {
                    Decimal::ONE
                } else {
                    Decimal::pow10(base * base.log10())
                }
            })
            .fold(Decimal::ONE, |acc, x| acc * x)
    }

    /// powerdimboost (bit 18): Dimension Boost power `×x`.
    pub fn glyph_effect_powerdimboost(&self) -> f64 {
        self.glyph_effect_values(18)
            .iter()
            .map(|&(l, s)| (l * s).sqrt())
            .product()
    }

    /// A single glyph's value for effect `bit`, as a Decimal (presentation:
    /// glyph tooltips / the choice modal). Mirrors the per-effect formulas
    /// used by the combiners above; `powermult` is computed in log space.
    pub fn glyph_single_effect_value(glyph: &Glyph, bit: u8) -> Decimal {
        let l = glyph.level as f64;
        let s = glyph.strength;
        let v = |x: f64| Decimal::from_float(x);
        match bit {
            0 => v(1.01 + l.powf(0.32) * s.powf(0.45) / 75.0),
            1 => v(1.0 + l.powf(0.3) * s.powf(0.65) / 20.0),
            2 => v(((s + 3.0) * l).powf(0.9)),
            3 => v((l * s).powi(3) * 100.0),
            4 => v((l * s).powf(1.5) * 2.0),
            5 => v(1.0 - l.powf(0.17) * s.powf(0.35) / 100.0),
            6 => v((l * s).sqrt() / 10_000.0),
            7 => v(1.1 + l.powf(0.7) * s.powf(0.7) / 25.0),
            8 => v(l * s * 3.0),
            9 => v(1.1 + l.sqrt() * s / 25.0),
            10 => v(0.0003 * l.powf(0.3) * s.powf(0.65)),
            11 => v((l.powf(0.25) * s.powf(0.4)).sqrt() / 50.0),
            12 => v(1.007 + l.powf(0.21) * s.powf(0.4) / 75.0),
            13 => v(l.powf(0.2) * s.powf(0.4) * 0.04),
            14 => v((l * (s + 1.0)).powi(6) * 10_000.0),
            15 => v((l * s).powf(1.5) * 2.0),
            16 => v(1.015 + l.powf(0.2) * s.powf(0.4) / 75.0),
            17 => {
                let base = l * s * 10.0;
                if base <= 0.0 {
                    Decimal::ONE
                } else {
                    Decimal::pow10(base * base.log10())
                }
            }
            18 => v((l * s).sqrt()),
            19 => v(1.0 + l * s / 12.0),
            _ => Decimal::ZERO,
        }
    }

    /// powerbuy10 (bit 19): buy-10 multiplier `×x`.
    pub fn glyph_effect_powerbuy10(&self) -> f64 {
        let v: Vec<f64> = self
            .glyph_effect_values(19)
            .iter()
            .map(|&(l, s)| 1.0 + l * s / 12.0)
            .collect();
        Self::combine_add_exponents(&v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with_reality() -> GameState {
        let mut game = GameState::new();
        game.eternity_unlocked = true;
        game.eternity_points = Decimal::new(1.0, 4000);
        game.records.this_reality.max_ep = Decimal::new(1.0, 4000);
        game.dilation.studies = vec![1, 2, 3, 4, 5, 6];
        game
    }

    fn glyph(kind: GlyphType, level: u32, strength: f64, effects: u32) -> Glyph {
        Glyph {
            id: 1,
            idx: 0,
            kind,
            strength,
            level,
            raw_level: level,
            effects,
        }
    }

    #[test]
    fn xorshift_matches_js_semantics() {
        // Reference values computed with the JS algorithm.
        assert_eq!(xorshift32_update(1), 270_369);
        assert_eq!(xorshift32_update(270_369), 67_634_689);
        // Negative (int32) states keep working.
        let s = xorshift32_update(-12345);
        assert_eq!(s, xorshift32_update(js_to_int32(-12345.0)));
    }

    #[test]
    fn rng_stream_matches_js_reference() {
        // Reference values generated with the original's JS code (GlyphRNG +
        // gaussianBellCurve + randomStrength) under node.
        let mut rng = GlyphRng::new(4_294_967_291.0, SECOND_GAUSSIAN_DEFAULT);
        let expected = [
            0.500_295_675_126_835_7,
            0.421_980_260_638_520_12,
            0.654_451_923_212_036_49,
            0.714_812_924_619_764_09,
            0.239_391_246_112_063_53,
        ];
        for &want in &expected {
            assert_eq!(rng.uniform(), want);
        }
        assert_eq!(rng.state, -1_119_306_075);

        let mut rng = GlyphRng::new(987_654_321.0, SECOND_GAUSSIAN_DEFAULT);
        assert_eq!(rng.normal(), 0.037_929_019_647_045_87);
        assert_eq!(rng.normal(), 0.317_373_212_411_152_46);

        let mut rng = GlyphRng::new(123_456_789.0, SECOND_GAUSSIAN_DEFAULT);
        let bell = GameState::gaussian_bell_curve(&mut rng);
        assert!((bell - 1.711_963_456_036_710_5).abs() < 1e-15);
        assert_eq!(rng.state, -474_866_960);
        assert!((rng.second_gaussian - -0.488_814_257_796_925_17).abs() < 1e-15);

        let game = GameState::new();
        let mut rng = GlyphRng::new(55_555.0, SECOND_GAUSSIAN_DEFAULT);
        assert_eq!(game.random_strength(&mut rng), 1.4675);
        assert_eq!(rng.state, -1_628_335_088);
    }

    #[test]
    fn uniform_is_in_unit_interval_and_deterministic() {
        let mut a = GlyphRng::new(12345.0, SECOND_GAUSSIAN_DEFAULT);
        let mut b = GlyphRng::new(12345.0, SECOND_GAUSSIAN_DEFAULT);
        for _ in 0..100 {
            let x = a.uniform();
            assert!((0.0..1.0).contains(&x));
            assert_eq!(x, b.uniform());
        }
    }

    #[test]
    fn permutation_index_decodes_lehmer_codes() {
        assert_eq!(permutation_index(4, 0), vec![0, 1, 2, 3]);
        assert_eq!(permutation_index(4, 23), vec![3, 2, 1, 0]);
        // 4! wraps.
        assert_eq!(permutation_index(4, 24), vec![0, 1, 2, 3]);
        assert_eq!(permutation_index(5, 1), vec![0, 1, 2, 4, 3]);
    }

    #[test]
    fn first_reality_grants_starting_and_companion_glyphs() {
        let mut game = game_with_reality();
        assert!(game.reality_with_glyph_choice(None, false));
        let inv = &game.reality.glyphs.inventory;
        assert_eq!(inv.len(), 2);
        assert_eq!(inv[0].kind, GlyphType::Power);
        assert_eq!(inv[0].strength, 1.5);
        assert_eq!(inv[0].effects, 1 << 16);
        assert_eq!(inv[1].kind, GlyphType::Companion);
        // Companion rarity encodes the 1e4000 EP.
        assert!((inv[1].strength - rarity_to_strength(4000.0 / 1e6)).abs() < 1e-12);
        // Seed locked in.
        assert_eq!(game.reality.seed, game.reality.initial_seed);
    }

    #[test]
    fn later_realities_grant_generated_glyphs_deterministically() {
        let mut game = game_with_reality();
        game.reality.realities = 1;
        game.reality.seed = game.reality.initial_seed;

        let preview = game.upcoming_glyphs();
        assert_eq!(preview.len(), 1);

        let mut game2 = game.clone();
        assert!(game.reality_with_glyph_choice(None, false));
        assert_eq!(game.reality.glyphs.inventory.len(), 1);
        let granted = game.reality.glyphs.inventory[0].clone();
        // The preview (which does not advance the RNG) matches the grant.
        assert_eq!(preview[0].kind, granted.kind);
        assert_eq!(preview[0].strength, granted.strength);
        assert_eq!(preview[0].effects, granted.effects);
        // The RNG advanced.
        assert_ne!(game.reality.seed, game.reality.initial_seed);

        // Replaying the same state gives the same glyph (determinism).
        assert!(game2.reality_with_glyph_choice(None, false));
        assert_eq!(game2.reality.glyphs.inventory[0], granted);
    }

    #[test]
    fn uniform_glyph_lists_match_js_reference() {
        // Full glyph-choice lists generated by the original's uniformity code
        // (glyphList → uniformGlyphs → randomGlyph) under node, at glyph
        // level 42 with no Reality Upgrades.
        struct Case {
            seed: f64,
            init_seed: f64,
            realities: u32,
            end_seed: i32,
            glyphs: [(GlyphType, f64, u32); 4],
        }
        let cases = [
            Case {
                seed: 4_294_967_291.0,
                init_seed: 4_294_967_291.0,
                realities: 1,
                end_seed: 1_734_709_005,
                glyphs: [
                    (GlyphType::Power, 1.0075, 196_608),
                    (GlyphType::Infinity, 2.3525, 20_480),
                    (GlyphType::Time, 1.225, 1),
                    (GlyphType::Dilation, 1.455, 48),
                ],
            },
            Case {
                seed: 12_345_678.0,
                init_seed: 987_654_321_012.0,
                realities: 7,
                end_seed: -119_418_386,
                glyphs: [
                    (GlyphType::Power, 1.8775, 589_824),
                    (GlyphType::Infinity, 1.775, 12_288),
                    (GlyphType::Replication, 1.52, 3_072),
                    (GlyphType::Dilation, 1.37, 64),
                ],
            },
            Case {
                seed: -99_887_766.0,
                init_seed: 555_555_555.0,
                realities: 20,
                end_seed: -744_087_444,
                glyphs: [
                    (GlyphType::Power, 1.52, 589_824),
                    (GlyphType::Infinity, 1.085, 20_480),
                    (GlyphType::Replication, 1.03, 3_072),
                    (GlyphType::Dilation, 1.63, 32),
                ],
            },
        ];
        for case in cases {
            let mut game = GameState::new();
            game.reality.realities = case.realities;
            game.reality.initial_seed = case.init_seed;
            let mut rng = GlyphRng::new(case.seed, SECOND_GAUSSIAN_DEFAULT);
            let level = GlyphLevel {
                raw_level: 42,
                actual_level: 42,
            };
            let glyphs = game.glyph_list(4, level, &mut rng);
            assert_eq!(rng.state, case.end_seed, "seed {}", case.seed);
            for (glyph, want) in glyphs.iter().zip(case.glyphs.iter()) {
                assert_eq!(glyph.kind, want.0);
                assert_eq!(glyph.strength, want.1);
                assert_eq!(glyph.effects, want.2);
            }
        }
    }

    #[test]
    fn random_glyph_list_matches_js_reference_past_uniformity() {
        // The non-uniform path (realities > 20), JS-referenced at glyph
        // level 9001, seed 777777.
        let mut game = GameState::new();
        game.reality.realities = 21;
        let mut rng = GlyphRng::new(777_777.0, SECOND_GAUSSIAN_DEFAULT);
        let level = GlyphLevel {
            raw_level: 9001,
            actual_level: 9001,
        };
        let glyphs = game.glyph_list(4, level, &mut rng);
        assert_eq!(rng.state, -1_891_797_454);
        let want = [
            (GlyphType::Replication, 1.255, 2_304u32),
            (GlyphType::Time, 1.885, 9),
            (GlyphType::Dilation, 2.1, 48),
            (GlyphType::Power, 1.0325, 327_680),
        ];
        for (glyph, want) in glyphs.iter().zip(want.iter()) {
            assert_eq!(glyph.kind, want.0);
            assert_eq!(glyph.strength, want.1);
            assert_eq!(glyph.effects, want.2);
        }
        game.reality.realities = 21;
    }

    #[test]
    fn uniformity_spreads_types_across_a_group() {
        let mut game = game_with_reality();
        game.reality.realities = 1;
        game.reality.seed = game.reality.initial_seed;
        let mut rng = GlyphRng::new(game.reality.seed, SECOND_GAUSSIAN_DEFAULT);
        let glyphs = game.glyph_list(4, game.gained_glyph_level(), &mut rng);
        // Four glyphs of four *different* basic types.
        let mut kinds: Vec<GlyphType> = glyphs.iter().map(|g| g.kind).collect();
        kinds.dedup();
        assert_eq!(kinds.len(), 4);
        // Every glyph has at least one effect and ≤ 2 (uniformity cap
        // without RU17), not counting a forced primary re-add.
        for g in &glyphs {
            assert!(g.effects.count_ones() >= 1);
            assert!(g.effects.count_ones() <= 3);
        }
    }

    #[test]
    fn start_perk_offers_four_choices() {
        let mut game = game_with_reality();
        game.reality.realities = 5;
        game.reality.seed = 987_654_321.0;
        game.reality.perks.insert(0);
        assert_eq!(game.glyph_choice_count(), 4);
        let choices = game.upcoming_glyphs();
        assert_eq!(choices.len(), 4);
        // Choosing index 2 grants exactly that glyph.
        let want = choices[2].clone();
        assert!(game.reality_with_glyph_choice(Some(2), false));
        let got = &game.reality.glyphs.inventory[0];
        assert_eq!(got.kind, want.kind);
        assert_eq!(got.effects, want.effects);
        assert_eq!(got.strength, want.strength);
    }

    #[test]
    fn uncommon_guarantee_boosts_a_choice() {
        // Over many seeds, every choice list must contain an uncommon glyph.
        let mut game = game_with_reality();
        game.reality.realities = 25; // uniformity off
        for seed in 1..50 {
            let mut rng = GlyphRng::new(seed as f64, SECOND_GAUSSIAN_DEFAULT);
            let glyphs = game.glyph_list(4, game.gained_glyph_level(), &mut rng);
            assert!(
                glyphs.iter().any(|g| g.strength >= 1.5),
                "no uncommon glyph for seed {seed}"
            );
        }
        game.reality.realities = 25;
    }

    #[test]
    fn equip_and_respec_round_trip() {
        let mut game = game_with_reality();
        game.add_glyph_to_inventory(glyph(GlyphType::Power, 10, 2.0, 1 << 16));
        let id = game.reality.glyphs.inventory[0].id;

        assert!(game.equip_glyph(id, 0));
        assert!(game.reality.glyphs.inventory.is_empty());
        assert_eq!(game.reality.glyphs.active.len(), 1);
        assert_eq!(game.requirement_checks.reality_max_glyphs, 1);
        // Slot occupied → second equip into the same slot fails.
        game.add_glyph_to_inventory(glyph(GlyphType::Time, 5, 2.0, 1));
        let id2 = game.reality.glyphs.inventory[0].id;
        assert!(!game.equip_glyph(id2, 0));
        // Slots beyond the cap (3 without RU9/24) fail.
        assert!(!game.equip_glyph(id2, 3));
        assert!(game.equip_glyph(id2, 1));

        // Respec at the next reality returns them to the inventory.
        game.reality.realities = 1;
        game.reality.seed = game.reality.initial_seed;
        game.reality.respec = true;
        assert!(game.reality_with_glyph_choice(None, false));
        assert!(game.reality.glyphs.active.is_empty());
        assert!(!game.reality.respec);
        // 2 unequipped + 1 new grant.
        assert_eq!(game.reality.glyphs.inventory.len(), 3);
    }

    #[test]
    fn glyph_effects_apply_when_equipped() {
        let mut game = GameState::new();
        // A power glyph with powerpow + powermult + powerdimboost + powerbuy10.
        game.reality.glyphs.active.push(glyph(
            GlyphType::Power,
            100,
            2.0,
            (1 << 16) | (1 << 17) | (1 << 18) | (1 << 19),
        ));
        let pow = game.glyph_effect_powerpow();
        assert!(
            (pow - (1.015 + 100f64.powf(0.2) * 2f64.powf(0.4) / 75.0)).abs() < 1e-12
        );
        // powermult = 2000^2000 = 10^(2000·log10(2000)).
        let mult = game.glyph_effect_powermult();
        assert!((mult.log10() - 2000.0 * 2000f64.log10()).abs() < 1e-6);
        assert!((game.glyph_effect_powerdimboost() - 200f64.sqrt()).abs() < 1e-12);
        assert!(
            (game.glyph_effect_powerbuy10() - (1.0 + 100.0 * 2.0 / 12.0)).abs() < 1e-12
        );
        // Unequipped effects stay neutral.
        assert_eq!(game.glyph_effect_timespeed(), 1.0);
        assert_eq!(game.glyph_effect_infinityrate(), 0.0);
        assert_eq!(game.glyph_effect_dilation_dt(), Decimal::ONE);
    }

    #[test]
    fn add_exponents_combines_relative_to_one() {
        let mut game = GameState::new();
        game.reality
            .glyphs
            .active
            .push(glyph(GlyphType::Power, 100, 2.0, 1 << 16));
        game.reality
            .glyphs
            .active
            .push(glyph(GlyphType::Power, 50, 1.5, 1 << 16));
        let single1 = 1.015 + 100f64.powf(0.2) * 2f64.powf(0.4) / 75.0;
        let single2 = 1.015 + 50f64.powf(0.2) * 1.5f64.powf(0.4) / 75.0;
        assert!(
            (game.glyph_effect_powerpow() - (single1 + single2 - 1.0)).abs() < 1e-12
        );
    }

    #[test]
    fn companion_bits_do_not_leak_into_replication_effects() {
        let mut game = GameState::new();
        game.reality.glyphs.active.push(glyph(
            GlyphType::Companion,
            1,
            2.0,
            (1 << 8) | (1 << 9),
        ));
        assert_eq!(game.glyph_effect_replicationspeed(), Decimal::ONE);
        assert_eq!(game.glyph_effect_replicationpow(), 1.0);
    }

    #[test]
    fn sacrifice_needs_ru19_and_accumulates() {
        let mut game = GameState::new();
        game.add_glyph_to_inventory(glyph(GlyphType::Power, 100, 2.0, 1 << 16));
        let id = game.reality.glyphs.inventory[0].id;
        // Without RU19 the glyph is deleted with no gain.
        assert!(game.sacrifice_glyph(id));
        assert_eq!(game.reality.glyphs.sac[0], 0.0);

        game.reality.upgrade_bits |= 1 << 19;
        game.add_glyph_to_inventory(glyph(GlyphType::Power, 100, 2.0, 1 << 16));
        let id = game.reality.glyphs.inventory[0].id;
        let expected = (110f64).powf(2.5) * 2.0;
        assert!(game.sacrifice_glyph(id));
        assert!((game.reality.glyphs.sac[0] - expected).abs() < 1e-9);

        // Sacrifice effects move off their neutral values.
        assert!(game.glyph_sac_power_effect() > 0);
        assert!(game.glyph_sac_infinity_effect() == 1.0);
        game.reality.glyphs.sac[1] = 1e60;
        assert!(game.glyph_sac_infinity_effect() > 1.0);
        game.reality.glyphs.sac[4] = 1e60;
        assert!(game.glyph_sac_dilation_effect() > 1.0);
    }

    #[test]
    fn inventory_respects_protected_rows_and_capacity() {
        let mut game = GameState::new();
        // Default 2 protected rows → first free slot is 20.
        game.add_glyph_to_inventory(glyph(GlyphType::Power, 1, 1.5, 1 << 16));
        assert_eq!(game.reality.glyphs.inventory[0].idx, 20);
        assert_eq!(game.glyph_free_inventory_space(), 99);
    }
}
