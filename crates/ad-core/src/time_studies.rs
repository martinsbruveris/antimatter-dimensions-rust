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

// --- Study import strings + presets (`TimeStudyTree` / `timestudy.presets`) ---

/// One Time Study preset slot (`player.timestudy.presets[i]`): a display name
/// (≤ 4 ASCII chars) and a study import string.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StudyPreset {
    pub name: String,
    pub studies: String,
}

/// Maximum preset name length (`nicknameBlur` slices to 4).
pub const STUDY_PRESET_NAME_MAX: usize = 4;

/// The path-name shorthands accepted in import strings (`TimeStudyTree.sets`;
/// the Ra-gated `triad` set is out of frontier).
const STUDY_SETS: [(&str, &[u16]); 8] = [
    ("antimatter", &[71, 81, 91, 101]),
    ("infinity", &[72, 82, 92, 102]),
    ("time", &[73, 83, 93, 103]),
    ("active", &[121, 131, 141]),
    ("passive", &[122, 132, 142]),
    ("idle", &[123, 133, 143]),
    ("light", &[221, 223, 225, 227, 231, 233]),
    ("dark", &[222, 224, 226, 228, 232, 234]),
];

/// A parsed study import string (`TimeStudyTree.parseStudyImport`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedStudyImport {
    /// Valid study ids, in the listed order (set names and ranges expanded).
    pub studies: Vec<u16>,
    /// The EC id from a trailing `|N` (0 = none).
    pub ec: u8,
    /// Whether the string ends with `!` (start the EC on commit).
    pub start_ec: bool,
    /// Tokens that were well-formed but named nonexistent studies/ECs.
    pub invalid: Vec<String>,
}

/// `TimeStudyTree.truncateInput`: lowercase, expand set names into id lists,
/// strip spaces, collapse duplicate commas and a comma before `|`, drop a
/// trailing `,`/`|`.
fn truncate_study_import(input: &str) -> String {
    let mut s = input.to_lowercase();
    for (name, ids) in STUDY_SETS {
        if s.contains(name) {
            let list = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            s = s.replace(name, &list);
        }
    }
    s.retain(|c| c != ' ');
    while s.contains(",,") {
        s = s.replace(",,", ",");
    }
    s = s.replace(",|", "|");
    while s.ends_with(',') || s.ends_with('|') {
        s.pop();
    }
    s
}

/// `TimeStudyTree.isValidImportString`: pure format check (existence is
/// checked separately so error messages can distinguish the two). Equivalent
/// to the original's `^,?((\d{2,3}(-\d{2,3})?)\b,?)*(\|\d{1,2}!?)?$` after
/// set-name removal.
pub fn is_valid_study_import(input: &str) -> bool {
    if input.trim().is_empty() {
        return false;
    }
    let mut s: String = input.chars().filter(|&c| c != ' ').collect();
    s = s.to_lowercase();
    for (name, _) in STUDY_SETS {
        while let Some(pos) = s.find(name) {
            let mut removed: String = s[..pos].to_string();
            let rest = &s[pos + name.len()..];
            removed.push_str(rest.strip_prefix(',').unwrap_or(rest));
            s = removed;
        }
    }
    let (studies_part, ec_part) = match s.split_once('|') {
        Some((a, b)) => (a, Some(b)),
        None => (s.as_str(), None),
    };
    let studies_part = studies_part.strip_prefix(',').unwrap_or(studies_part);
    let entry_ok = |entry: &str| -> bool {
        let ok_num = |n: &str| {
            n.len() >= 2 && n.len() <= 3 && n.bytes().all(|b| b.is_ascii_digit())
        };
        match entry.split_once('-') {
            Some((a, b)) => ok_num(a) && ok_num(b),
            None => ok_num(entry),
        }
    };
    if !studies_part.is_empty() && !studies_part.split(',').all(entry_ok) {
        return false;
    }
    match ec_part {
        None => true,
        Some(ec) => {
            let ec = ec.strip_suffix('!').unwrap_or(ec);
            !ec.is_empty() && ec.len() <= 2 && ec.bytes().all(|b| b.is_ascii_digit())
        }
    }
}

/// Parse a study import string into ids + EC (`parseStudyImport`). Invalid
/// study/EC ids are collected rather than failing the parse.
pub fn parse_study_import(input: &str) -> ParsedStudyImport {
    let mut out = ParsedStudyImport {
        start_ec: input.trim_end().ends_with('!'),
        ..Default::default()
    };
    let truncated = truncate_study_import(input);
    let studies_part = truncated.split('|').next().unwrap_or("");
    for entry in studies_part.split(',').filter(|e| !e.is_empty()) {
        match entry.split_once('-') {
            // A range (`studyRangeToArray`): both endpoints must be existing
            // studies, then every existing id in between is included; a bad
            // range contributes nothing (and records nothing) like the
            // original.
            Some((first, last)) => {
                let (Ok(a), Ok(b)) = (first.parse::<u16>(), last.parse::<u16>()) else {
                    out.invalid.push(entry.to_string());
                    continue;
                };
                if time_study_def(a).is_none() || time_study_def(b).is_none() {
                    continue;
                }
                out.studies
                    .extend((a..=b).filter(|&id| time_study_def(id).is_some()));
            }
            None => match entry.parse::<u16>() {
                Ok(id) if time_study_def(id).is_some() => out.studies.push(id),
                _ => out.invalid.push(entry.to_string()),
            },
        }
    }
    if let Some(ec_str) = truncated.split('|').nth(1) {
        let ec_str = ec_str.strip_suffix('!').unwrap_or(ec_str);
        match ec_str.parse::<u8>() {
            // 0 is allowed (saved presets contain it by default).
            Ok(ec) if ec <= 12 => out.ec = ec,
            _ => out.invalid.push(format!("EC{ec_str}")),
        }
    }
    out
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
        // The TTF perk (105): purchases no longer spend the currency.
        if !self.perk_bought(105) {
            self.antimatter -= cost;
        }
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
        if !self.perk_bought(105) {
            self.infinity_points -= cost;
        }
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
        if !self.perk_bought(105) {
            self.eternity_points -= cost;
        }
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
            // The EC5R perk waives TS62's EC5 requirement.
            62 => self.perk_bought(57) || self.eternity_challenge_completions(5) > 0,
            // The dimension paths are excluded by holding the EC11/EC12 study
            // (their `forbiddenStudies` counterpart).
            71 => self.eternity_challenge_unlocked != 12,
            72 => {
                self.eternity_challenge_unlocked != 11
                    && self.eternity_challenge_unlocked != 12
            }
            73 => self.eternity_challenge_unlocked != 11,
            // 181 needs EC1–3 completed at least once (each waivable by the
            // EC1R/EC2R/EC3R perks 54/55/56).
            181 => [(1u8, 54u8), (2, 55), (3, 56)].iter().all(|&(ec, perk)| {
                self.perk_bought(perk) || self.eternity_challenge_completions(ec) > 0
            }),
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

    // --- Study presets ---------------------------------------------------------

    /// The current tree as an import string (`TimeStudyTree.exportString`):
    /// bought study ids joined by commas, `|<held EC id>` (0 when none), and a
    /// trailing `!` while an EC runs.
    pub fn study_tree_export_string(&self) -> String {
        let ids = self
            .studies
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let bang = if self.eternity_challenge_current != 0 {
            "!"
        } else {
            ""
        };
        format!("{ids}|{}{bang}", self.eternity_challenge_unlocked)
    }

    /// Buy a parsed study list into the current tree, in order (the effect of
    /// `commitToGameState` on a combined current+import tree): each study is
    /// attempted once; failures are skipped. A trailing EC study is bought and
    /// — with `start_ec` — started.
    pub(crate) fn commit_study_import(&mut self, parsed: &ParsedStudyImport) {
        for &id in &parsed.studies {
            self.buy_time_study(id);
        }
        if parsed.ec != 0 {
            self.buy_ec_study(parsed.ec);
            if parsed.start_ec && self.eternity_challenge_unlocked == parsed.ec {
                self.start_eternity_challenge(parsed.ec);
            }
        }
    }

    /// Load preset `slot` (0-indexed) into the current tree
    /// (`TimeStudySaveLoadButton.load`). Returns false for an empty preset.
    pub fn load_study_preset(&mut self, slot: usize) -> bool {
        let Some(preset) = self.study_presets.get(slot) else {
            return false;
        };
        if preset.studies.is_empty() {
            return false;
        }
        let parsed = parse_study_import(&preset.studies.clone());
        self.commit_study_import(&parsed);
        true
    }

    /// "Respec and Load": flag a respec, Eternity, then buy the preset tree
    /// from scratch (`respecAndLoad`). No-op unless an Eternity is available.
    pub fn respec_and_load_study_preset(&mut self, slot: usize) -> bool {
        if !self.can_eternity() {
            return false;
        }
        self.respec = true;
        if !self.eternity() {
            return false;
        }
        self.load_study_preset(slot)
    }

    /// Save the current tree into preset `slot` (`TimeStudySaveLoadButton
    /// .save`).
    pub fn save_study_preset(&mut self, slot: usize) -> bool {
        if slot >= self.study_presets.len() {
            return false;
        }
        self.study_presets[slot].studies = self.study_tree_export_string();
        true
    }

    /// Rename preset `slot`: at most [`STUDY_PRESET_NAME_MAX`] ASCII
    /// (≤ `\u{ff}`) characters, unique among presets; empty clears the name
    /// (the button then shows the slot number). Mirrors `nicknameBlur`.
    pub fn set_study_preset_name(&mut self, slot: usize, name: &str) -> bool {
        if slot >= self.study_presets.len() {
            return false;
        }
        let name: String = name.chars().take(STUDY_PRESET_NAME_MAX).collect();
        let name = name.trim().to_string();
        if name.chars().any(|c| c > '\u{ff}') {
            return false;
        }
        if !name.is_empty() && self.study_presets.iter().any(|p| p.name == name) {
            return false;
        }
        self.study_presets[slot].name = name;
        true
    }

    /// Overwrite preset `slot`'s study string (the Edit modal). The string
    /// must be well-formed, or empty (the Delete action).
    pub fn set_study_preset_studies(&mut self, slot: usize, studies: &str) -> bool {
        if slot >= self.study_presets.len() {
            return false;
        }
        let studies = studies.trim();
        if !studies.is_empty() && !is_valid_study_import(studies) {
            return false;
        }
        self.study_presets[slot].studies = studies.to_string();
        true
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
        // The ACT perk (studyActiveEP): Active path multipliers maximized.
        if self.perk_bought(70) {
            return 50.0;
        }
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
    fn import_string_validation() {
        assert!(is_valid_study_import("11,21,22"));
        assert!(is_valid_study_import("11-62"));
        assert!(is_valid_study_import("11,21|4"));
        assert!(is_valid_study_import("11,21|4!"));
        assert!(is_valid_study_import("antimatter,infinity"));
        assert!(is_valid_study_import("11, 21, 22"));
        // Trailing separators are tolerated by truncation in the original's
        // regex via the optional groups.
        assert!(!is_valid_study_import(""));
        assert!(!is_valid_study_import("  "));
        assert!(!is_valid_study_import("abc"));
        assert!(!is_valid_study_import("1"));
        assert!(!is_valid_study_import("11;21"));
        assert!(!is_valid_study_import("11|123"));
    }

    #[test]
    fn import_parsing_expands_ranges_sets_and_ec() {
        let parsed = parse_study_import("11,21-33,antimatter|4!");
        assert!(parsed.studies.starts_with(&[11, 21, 22, 31, 32, 33]));
        // The antimatter set: 71, 81, 91, 101.
        assert!(parsed.studies.ends_with(&[71, 81, 91, 101]));
        assert_eq!(parsed.ec, 4);
        assert!(parsed.start_ec);
        assert!(parsed.invalid.is_empty());

        // Nonexistent single ids are recorded; bad ranges contribute nothing.
        let parsed = parse_study_import("11,99,12-14");
        assert_eq!(parsed.studies, vec![11]);
        assert_eq!(parsed.invalid, vec!["99"]);
        assert_eq!(parsed.ec, 0);
        assert!(!parsed.start_ec);
    }

    #[test]
    fn export_string_matches_original_format() {
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        game.buy_time_study(21);
        assert_eq!(game.study_tree_export_string(), "11,21|0");
        game.eternity_challenge_unlocked = 4;
        assert_eq!(game.study_tree_export_string(), "11,21|4");
    }

    #[test]
    fn preset_save_and_load_round_trip() {
        let mut game = game_with_tt(100.0);
        game.buy_time_study(11);
        game.buy_time_study(21);
        assert!(game.save_study_preset(0));
        assert_eq!(game.study_presets[0].studies, "11,21|0");

        // Respec, then load the preset back.
        game.respec_time_studies_now();
        assert!(game.studies.is_empty());
        assert!(game.load_study_preset(0));
        assert_eq!(game.studies, vec![11, 21]);

        // An empty slot refuses to load.
        assert!(!game.load_study_preset(1));
    }

    #[test]
    fn preset_load_skips_unaffordable_studies() {
        let mut game = game_with_tt(4.0); // enough for 11 (1) + 21 (3) only
        game.study_presets[0].studies = "11,21,22".into();
        assert!(game.load_study_preset(0));
        assert_eq!(game.studies, vec![11, 21]);
    }

    #[test]
    fn preset_names_are_validated() {
        let mut game = GameState::new();
        assert!(game.set_study_preset_name(0, "ANTI"));
        assert_eq!(game.study_presets[0].name, "ANTI");
        // Over-long names are truncated to 4 chars, like the original input.
        assert!(game.set_study_preset_name(1, "TOOLONG"));
        assert_eq!(game.study_presets[1].name, "TOOL");
        // Duplicates are rejected.
        assert!(!game.set_study_preset_name(2, "ANTI"));
        // Non-ASCII (beyond U+00FF) is rejected.
        assert!(!game.set_study_preset_name(2, "日本"));
        // Clearing is allowed.
        assert!(game.set_study_preset_name(0, ""));
    }

    #[test]
    fn preset_studies_edit_validates_format() {
        let mut game = GameState::new();
        assert!(game.set_study_preset_studies(0, "11,21|4"));
        assert!(!game.set_study_preset_studies(0, "garbage"));
        assert_eq!(game.study_presets[0].studies, "11,21|4");
        // Deleting (empty string) is allowed.
        assert!(game.set_study_preset_studies(0, ""));
        assert_eq!(game.study_presets[0].studies, "");
    }

    #[test]
    fn respec_and_load_eternities_first() {
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        game.buy_time_study(22);
        game.study_presets[0].studies = "11,21".into();
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.respec_and_load_study_preset(0));
        // The old tree (incl. 22) was respecced away; the preset bought fresh.
        assert_eq!(game.studies, vec![11, 21]);

        // Without an available Eternity nothing happens.
        let mut game = game_with_tt(10.0);
        game.buy_time_study(11);
        assert!(!game.respec_and_load_study_preset(0));
        assert_eq!(game.studies, vec![11]);
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
