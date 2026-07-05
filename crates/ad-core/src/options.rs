//! Player options (UI/UX preferences).
//!
//! These mirror the original game's `player.options` object. The engine itself
//! is indifferent to almost all of them — they configure the frontend — but
//! they live in `GameState` so that a save file produced from a fresh game is
//! valid and so that options round-trip unchanged when a save is loaded, the
//! engine is run, and the save is written out again. Defaults match the
//! original game (`src/core/player.js`).
//!
//! Only the subset that is currently surfaced in the UI is modelled. More
//! fields are added as the corresponding options tabs are implemented.

/// Default game-loop cadence in milliseconds (original `updateRate: 33`).
pub const DEFAULT_UPDATE_RATE_MS: u32 = 33;
/// Slider bounds for the update rate, matching the original (33–200 ms).
pub const MIN_UPDATE_RATE_MS: u32 = 33;
pub const MAX_UPDATE_RATE_MS: u32 = 200;

/// The notation names the frontend can render (subset of the original's ~22).
/// These are the display names; the `ad-format` WASM matches them case-insensitively.
pub const NOTATIONS: [&str; 4] = ["Scientific", "Engineering", "Standard", "Letters"];
/// Default notation. The original defaults to "Mixed scientific" (not yet ported);
/// until then we default to "Standard".
pub const DEFAULT_NOTATION: &str = "Standard";

/// Slider bounds for the exponent-notation digit thresholds, matching the
/// original's Exponent Notation modal (3–15 digits).
pub const MIN_NOTATION_DIGITS: u32 = 3;
pub const MAX_NOTATION_DIGITS: u32 = 15;
/// Defaults for the two thresholds (original `notationDigits: { comma: 5, notation: 9 }`):
/// the exponent gets commas at 10^comma and switches to in-notation at 10^notation.
pub const DEFAULT_NOTATION_DIGITS_COMMA: u32 = 5;
pub const DEFAULT_NOTATION_DIGITS_NOTATION: u32 = 9;

/// Offline tick budget: across how many discrete ticks is an offline interval being
/// replayed (the resolution dial). Default matches the original (`offlineTicks:
/// 1e5`). Our slider range diverges from the original's (500..1e6): we run
/// 10K..=10M, exploiting the faster engine. See
/// `design-docs/2026-06-30-offline-progress.md`.
pub const DEFAULT_OFFLINE_TICKS: u32 = 100_000;
pub const MIN_OFFLINE_TICKS: u32 = 10_000;
pub const MAX_OFFLINE_TICKS: u32 = 10_000_000;

/// Autosave cadence in milliseconds (original `autosaveInterval`). The frontend
/// autosave loop writes the on-disk root this often; the Saving-tab slider runs
/// 10..=60 s in 1 s steps (matching the original), so the stored value is in
/// `[MIN_AUTOSAVE_INTERVAL_MS, MAX_AUTOSAVE_INTERVAL_MS]`.
pub const DEFAULT_AUTOSAVE_INTERVAL_MS: u32 = 30_000;
pub const MIN_AUTOSAVE_INTERVAL_MS: u32 = 10_000;
pub const MAX_AUTOSAVE_INTERVAL_MS: u32 = 60_000;

/// Maximum length of the custom save-file name (original input `maxlength="16"`).
pub const MAX_SAVE_FILE_NAME_LEN: usize = 16;

/// Number of top-level tabs in the original game (`hiddenSubtabBits` is an
/// 11-entry array indexed by the original tab id). We keep the original's tab
/// ids and array shape so hidden-tab state round-trips through the save
/// unchanged, even for tabs we don't render yet.
pub const TAB_COUNT: usize = 11;

/// Per-action confirmation toggles, mirroring the subset of
/// `player.options.confirmations` we model. Each is "show the explanatory modal
/// before performing this action"; all default `true`. The modal's "Don't show
/// this message again" checkbox flips the corresponding flag off.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Confirmations {
    pub dimension_boost: bool,
    pub antimatter_galaxy: bool,
    pub sacrifice: bool,
    pub big_crunch: bool,
    /// Eternity confirmation (original `confirmations.eternity`).
    #[cfg_attr(feature = "serde", serde(default = "confirmation_default"))]
    pub eternity: bool,
    /// Dilation enter/exit confirmation (original `confirmations.dilation`).
    #[cfg_attr(feature = "serde", serde(default = "confirmation_default"))]
    pub dilation: bool,
}

/// serde default for newer confirmation toggles (on, like the originals).
#[cfg(feature = "serde")]
fn confirmation_default() -> bool {
    true
}

impl Confirmations {
    pub fn new() -> Self {
        Self {
            dimension_boost: true,
            antimatter_galaxy: true,
            sacrifice: true,
            big_crunch: true,
            eternity: true,
            dilation: true,
        }
    }
}

impl Default for Confirmations {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation toggles (original `player.options.animations`, modelled subset).
/// Only the Big Crunch animation is in-frontier; the rest are added as the
/// matching prestige layers land.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Animations {
    pub big_crunch: bool,
}

impl Animations {
    pub fn new() -> Self {
        Self { big_crunch: true }
    }
}

impl Default for Animations {
    fn default() -> Self {
        Self::new()
    }
}

/// Info-display hint toggles (original `player.options.showHintText`, modelled
/// subset). Each shows a piece of overlay info; holding Shift shows them all
/// regardless (a frontend behaviour).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShowHintText {
    /// "Show % gain" — the per-second growth rate on dimension rows.
    pub show_percentage: bool,
    /// Achievement IDs on the Achievements-tab tiles.
    pub achievements: bool,
    /// Achievement unlock-state indicators (the ✓/✗ corner icons).
    pub achievement_unlock_states: bool,
    /// Challenge IDs on the challenge boxes.
    pub challenges: bool,
}

impl ShowHintText {
    pub fn new() -> Self {
        Self {
            show_percentage: true,
            achievements: true,
            achievement_unlock_states: true,
            challenges: true,
        }
    }
}

impl Default for ShowHintText {
    fn default() -> Self {
        Self::new()
    }
}

/// Away-progress display toggles (original `player.options.awayProgress`,
/// modelled subset): which resources may appear in the "While you were away"
/// summary. A resource still only shows if it actually increased.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AwayProgress {
    pub antimatter: bool,
    pub dimension_boosts: bool,
    pub antimatter_galaxies: bool,
    pub infinities: bool,
    pub infinity_points: bool,
    pub replicanti: bool,
    pub replicanti_galaxies: bool,
}

impl AwayProgress {
    pub fn new() -> Self {
        Self {
            antimatter: true,
            dimension_boosts: true,
            antimatter_galaxies: true,
            infinities: true,
            infinity_points: true,
            replicanti: true,
            replicanti_galaxies: true,
        }
    }
}

impl Default for AwayProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Options {
    /// Whether keyboard shortcuts are active (original `hotkeys`).
    pub hotkeys: bool,
    /// Game-loop cadence in milliseconds (original `updateRate`). The frontend
    /// only ticks the engine once this much wall-clock time has elapsed, so a
    /// larger value means coarser, less frequent updates.
    pub update_rate: u32,
    /// Active number-formatting notation (original `notation`). Display name from
    /// [`NOTATIONS`]; the frontend hands it to the `ad-format` WASM formatter.
    pub notation: String,
    /// Exponent digit count at/above which the exponent is comma-grouped
    /// (original `notationDigits.comma`); the threshold is 10^this.
    pub notation_digits_comma: u32,
    /// Exponent digit count at/above which the exponent is itself rendered in
    /// notation (original `notationDigits.notation`); the threshold is 10^this.
    /// Always `>= notation_digits_comma`.
    pub notation_digits_notation: u32,
    /// Offline replay resolution (original `offlineTicks`): the maximum number of
    /// discrete ticks an offline interval is spread across. Higher = finer.
    pub offline_ticks: u32,
    /// Autosave cadence in milliseconds (original `autosaveInterval`). Drives the
    /// frontend autosave loop and the Saving-tab slider.
    pub autosave_interval: u32,
    /// Whether the header shows the elapsed time since the last save (original
    /// `showTimeSinceSave`).
    pub show_time_since_save: bool,
    /// Custom save-file name (original `saveFileName`). Stored per save, so each
    /// save slot carries its own; shown per slot in the "Choose save" modal and
    /// used as the default filename when exporting the save to a file.
    pub save_file_name: String,
    /// Per-action confirmation toggles (original `confirmations`).
    pub confirmations: Confirmations,
    /// Animation toggles (original `animations`, modelled subset).
    pub animations: Animations,
    /// Info-display hint toggles (original `showHintText`, modelled subset).
    pub show_hint_text: ShowHintText,
    /// Away-progress display toggles (original `awayProgress`, modelled subset).
    pub away_progress: AwayProgress,
    /// Whether the header's prestige-gain number is colored relative to the
    /// current amount (original `headerTextColored`). The consumer is the
    /// post-break header crunch button, which isn't built yet.
    pub header_text_colored: bool,
    /// Which resource the Modern-UI sidebar shows (original `sidebarResourceID`):
    /// `0` = the latest unlocked resource, otherwise the original's resource id
    /// (2 = Antimatter, 3 = Infinity Points, 4 = Replicanti, …). Ids past our
    /// frontier are preserved for save round-trips; the frontend falls back to
    /// the latest resource when it can't render one.
    pub sidebar_resource_id: u32,
    /// Bitmask of hidden top-level tabs (original `hiddenTabBits`), indexed by
    /// the original tab ids (0 = Dimensions, … 10 = Shop).
    pub hidden_tab_bits: u32,
    /// Per-tab bitmasks of hidden subtabs (original `hiddenSubtabBits`), indexed
    /// by the original tab id, then the original subtab id within that tab.
    pub hidden_subtab_bits: [u32; TAB_COUNT],
    /// Automator event-log settings (original `automatorEvents`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub automator_events: AutomatorEventsOptions,
}

/// `player.options.automatorEvents`: how the Automator's event log displays
/// and retains entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutomatorEventsOptions {
    /// Show the newest entries at the top.
    pub newest_first: bool,
    /// Which timestamp the log shows (0 none / 1 in-reality time / 2 real
    /// time / 3 since-last-event gap — consumed by the Stage D UI).
    pub timestamp_type: u8,
    /// Ring-buffer size for retained entries.
    pub max_entries: u32,
    /// Clear the log on a Reality reset.
    pub clear_on_reality: bool,
    /// Clear the log when a script (re)starts.
    pub clear_on_restart: bool,
}

impl AutomatorEventsOptions {
    pub fn new() -> Self {
        Self {
            newest_first: false,
            timestamp_type: 0,
            max_entries: 200,
            clear_on_reality: true,
            clear_on_restart: true,
        }
    }
}

impl Default for AutomatorEventsOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Self {
        Self {
            hotkeys: true,
            update_rate: DEFAULT_UPDATE_RATE_MS,
            notation: DEFAULT_NOTATION.to_string(),
            notation_digits_comma: DEFAULT_NOTATION_DIGITS_COMMA,
            notation_digits_notation: DEFAULT_NOTATION_DIGITS_NOTATION,
            offline_ticks: DEFAULT_OFFLINE_TICKS,
            autosave_interval: DEFAULT_AUTOSAVE_INTERVAL_MS,
            show_time_since_save: true,
            save_file_name: String::new(),
            confirmations: Confirmations::new(),
            animations: Animations::new(),
            show_hint_text: ShowHintText::new(),
            away_progress: AwayProgress::new(),
            header_text_colored: false,
            sidebar_resource_id: 0,
            hidden_tab_bits: 0,
            hidden_subtab_bits: [0; TAB_COUNT],
            automator_events: AutomatorEventsOptions::new(),
        }
    }

    /// Set a single confirmation toggle by its original camelCase name
    /// (`dimensionBoost`, `antimatterGalaxy`, `sacrifice`, `bigCrunch`). An
    /// unknown name is ignored.
    pub fn set_confirmation(&mut self, kind: &str, enabled: bool) {
        match kind {
            "dimensionBoost" => self.confirmations.dimension_boost = enabled,
            "antimatterGalaxy" => self.confirmations.antimatter_galaxy = enabled,
            "sacrifice" => self.confirmations.sacrifice = enabled,
            "bigCrunch" => self.confirmations.big_crunch = enabled,
            "eternity" => self.confirmations.eternity = enabled,
            "dilation" => self.confirmations.dilation = enabled,
            _ => {}
        }
    }

    /// Set the offline tick budget. Any positive value is accepted **as-is** —
    /// we deliberately do not clamp to the slider's 10K..=10M range, so a value
    /// from an imported save (including the original's out-of-range values) is
    /// preserved. A zero falls back to 1 (the budget is always at least one tick).
    pub fn set_offline_ticks(&mut self, ticks: u32) {
        self.offline_ticks = ticks.max(1);
    }

    /// Set the update rate, clamped to the original game's slider range.
    pub fn set_update_rate(&mut self, rate: u32) {
        self.update_rate = rate.clamp(MIN_UPDATE_RATE_MS, MAX_UPDATE_RATE_MS);
    }

    /// Set the autosave interval, clamped to the Saving-tab slider range
    /// (10..=60 s).
    pub fn set_autosave_interval(&mut self, interval_ms: u32) {
        self.autosave_interval =
            interval_ms.clamp(MIN_AUTOSAVE_INTERVAL_MS, MAX_AUTOSAVE_INTERVAL_MS);
    }

    /// Set the custom save-file name, sanitized like the original's
    /// `SaveFileName` input: trimmed, restricted to alphanumerics, spaces and
    /// hyphens (`[^a-zA-Z0-9 -]` stripped), and capped at
    /// [`MAX_SAVE_FILE_NAME_LEN`] characters (the input's `maxlength`).
    pub fn set_save_file_name(&mut self, name: &str) {
        self.save_file_name = name
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
            .take(MAX_SAVE_FILE_NAME_LEN)
            .collect();
    }

    /// Set a single animation toggle by its original camelCase name
    /// (`bigCrunch`). An unknown name is ignored.
    pub fn set_animation(&mut self, kind: &str, enabled: bool) {
        if kind == "bigCrunch" {
            self.animations.big_crunch = enabled;
        }
    }

    /// Set a single info-display hint toggle by its original camelCase name
    /// (`showPercentage`, `achievements`, `achievementUnlockStates`,
    /// `challenges`). An unknown name is ignored.
    pub fn set_hint_text(&mut self, kind: &str, enabled: bool) {
        match kind {
            "showPercentage" => self.show_hint_text.show_percentage = enabled,
            "achievements" => self.show_hint_text.achievements = enabled,
            "achievementUnlockStates" => {
                self.show_hint_text.achievement_unlock_states = enabled
            }
            "challenges" => self.show_hint_text.challenges = enabled,
            _ => {}
        }
    }

    /// Set a single away-progress display toggle by its original camelCase name
    /// (`antimatter`, `dimensionBoosts`, `antimatterGalaxies`, `infinities`,
    /// `infinityPoints`, `replicanti`, `replicantiGalaxies`). An unknown name is
    /// ignored.
    pub fn set_away_progress(&mut self, kind: &str, enabled: bool) {
        match kind {
            "antimatter" => self.away_progress.antimatter = enabled,
            "dimensionBoosts" => self.away_progress.dimension_boosts = enabled,
            "antimatterGalaxies" => self.away_progress.antimatter_galaxies = enabled,
            "infinities" => self.away_progress.infinities = enabled,
            "infinityPoints" => self.away_progress.infinity_points = enabled,
            "replicanti" => self.away_progress.replicanti = enabled,
            "replicantiGalaxies" => self.away_progress.replicanti_galaxies = enabled,
            _ => {}
        }
    }

    /// Set the sidebar resource id. Any value is accepted **as-is**: ids past our
    /// frontier come from imported saves and must round-trip unchanged; the
    /// frontend falls back to the latest resource when it can't render one.
    pub fn set_sidebar_resource(&mut self, id: u32) {
        self.sidebar_resource_id = id;
    }

    /// Toggle a top-level tab's hidden bit (original `Tab.toggleVisibility`).
    /// The "cannot hide the current tab / a non-hidable tab" rules live in the
    /// frontend, which knows the open tab; an out-of-range id is ignored.
    pub fn toggle_tab_visibility(&mut self, tab_id: u32) {
        if (tab_id as usize) < TAB_COUNT {
            self.hidden_tab_bits ^= 1 << tab_id;
        }
    }

    /// Clear a top-level tab's hidden bit (original `Tab.unhideTab`).
    pub fn unhide_tab(&mut self, tab_id: u32) {
        if (tab_id as usize) < TAB_COUNT {
            self.hidden_tab_bits &= !(1 << tab_id);
        }
    }

    /// Toggle a subtab's hidden bit (original `SubtabState.toggleVisibility`).
    /// Ids are the original tab/subtab ids; out-of-range ids are ignored.
    pub fn toggle_subtab_visibility(&mut self, tab_id: u32, subtab_id: u32) {
        if (tab_id as usize) < TAB_COUNT && subtab_id < u32::BITS {
            self.hidden_subtab_bits[tab_id as usize] ^= 1 << subtab_id;
        }
    }

    /// Unhide every tab and subtab (the modal's "Show all tabs" button).
    pub fn show_all_tabs(&mut self) {
        self.hidden_tab_bits = 0;
        self.hidden_subtab_bits = [0; TAB_COUNT];
    }

    /// Set the notation, ignoring any name not in [`NOTATIONS`].
    pub fn set_notation(&mut self, notation: &str) {
        if NOTATIONS.contains(&notation) {
            self.notation = notation.to_string();
        }
    }

    /// Set the exponent-notation digit thresholds. Each is clamped to the
    /// [3, 15] slider range, and the notation threshold is kept `>=` the comma
    /// threshold (original NotationModal invariant).
    pub fn set_notation_digits(&mut self, comma: u32, notation: u32) {
        let comma = comma.clamp(MIN_NOTATION_DIGITS, MAX_NOTATION_DIGITS);
        let notation = notation.clamp(MIN_NOTATION_DIGITS, MAX_NOTATION_DIGITS);
        self.notation_digits_comma = comma;
        self.notation_digits_notation = notation.max(comma);
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_digits_clamp_to_range() {
        let mut o = Options::new();
        o.set_notation_digits(0, 99);
        assert_eq!(o.notation_digits_comma, MIN_NOTATION_DIGITS);
        assert_eq!(o.notation_digits_notation, MAX_NOTATION_DIGITS);
    }

    #[test]
    fn notation_threshold_stays_at_least_comma() {
        let mut o = Options::new();
        // A notation threshold below the comma threshold is raised to match.
        o.set_notation_digits(10, 4);
        assert_eq!(o.notation_digits_comma, 10);
        assert_eq!(o.notation_digits_notation, 10);
    }

    #[test]
    fn confirmations_default_on_and_toggle_by_name() {
        let mut o = Options::new();
        assert!(o.confirmations.dimension_boost);
        assert!(o.confirmations.antimatter_galaxy);
        assert!(o.confirmations.sacrifice);
        assert!(o.confirmations.big_crunch);

        o.set_confirmation("dimensionBoost", false);
        assert!(!o.confirmations.dimension_boost);
        // Other toggles are untouched, and an unknown name is a no-op.
        assert!(o.confirmations.antimatter_galaxy);
        o.set_confirmation("nope", false);
        assert!(o.confirmations.antimatter_galaxy);
    }

    #[test]
    fn autosave_interval_clamps_to_slider_range() {
        let mut o = Options::new();
        assert_eq!(o.autosave_interval, DEFAULT_AUTOSAVE_INTERVAL_MS);
        assert!(o.show_time_since_save);

        o.set_autosave_interval(5_000);
        assert_eq!(o.autosave_interval, MIN_AUTOSAVE_INTERVAL_MS);
        o.set_autosave_interval(999_999);
        assert_eq!(o.autosave_interval, MAX_AUTOSAVE_INTERVAL_MS);
        o.set_autosave_interval(45_000);
        assert_eq!(o.autosave_interval, 45_000);
    }

    #[test]
    fn save_file_name_is_sanitized_and_capped() {
        let mut o = Options::new();
        assert_eq!(o.save_file_name, "");

        // Strips disallowed characters, keeps spaces/hyphens, trims ends.
        o.set_save_file_name("  My Save!@# - 2  ");
        assert_eq!(o.save_file_name, "My Save - 2");

        // Capped at 16 characters (the input's maxlength).
        o.set_save_file_name("abcdefghijklmnopqrstuvwxyz");
        assert_eq!(o.save_file_name, "abcdefghijklmnop");
        assert_eq!(o.save_file_name.len(), MAX_SAVE_FILE_NAME_LEN);
    }

    #[test]
    fn animation_and_hint_toggles_default_on_and_set_by_name() {
        let mut o = Options::new();
        assert!(o.animations.big_crunch);
        assert!(o.show_hint_text.show_percentage);
        assert!(o.show_hint_text.achievement_unlock_states);

        o.set_animation("bigCrunch", false);
        assert!(!o.animations.big_crunch);
        o.set_hint_text("achievementUnlockStates", false);
        assert!(!o.show_hint_text.achievement_unlock_states);
        // Other toggles untouched; unknown names are no-ops.
        assert!(o.show_hint_text.achievements);
        o.set_hint_text("nope", false);
        o.set_animation("nope", false);
        assert!(o.show_hint_text.show_percentage);
        assert!(o.show_hint_text.challenges);
    }

    #[test]
    fn away_progress_toggles_default_on_and_set_by_name() {
        let mut o = Options::new();
        assert!(o.away_progress.antimatter);
        assert!(o.away_progress.replicanti_galaxies);

        o.set_away_progress("dimensionBoosts", false);
        o.set_away_progress("infinityPoints", false);
        assert!(!o.away_progress.dimension_boosts);
        assert!(!o.away_progress.infinity_points);
        assert!(o.away_progress.antimatter_galaxies);
        o.set_away_progress("nope", false);
        assert!(o.away_progress.infinities);
    }

    #[test]
    fn hidden_tab_bits_toggle_unhide_and_show_all() {
        let mut o = Options::new();
        assert_eq!(o.hidden_tab_bits, 0);
        assert_eq!(o.hidden_subtab_bits, [0; TAB_COUNT]);

        o.toggle_tab_visibility(5);
        o.toggle_tab_visibility(2);
        assert_eq!(o.hidden_tab_bits, (1 << 5) | (1 << 2));
        // Toggling again clears the bit; unhide is idempotent.
        o.toggle_tab_visibility(5);
        assert_eq!(o.hidden_tab_bits, 1 << 2);
        o.unhide_tab(2);
        o.unhide_tab(2);
        assert_eq!(o.hidden_tab_bits, 0);

        o.toggle_subtab_visibility(6, 1);
        o.toggle_subtab_visibility(0, 1);
        assert_eq!(o.hidden_subtab_bits[6], 1 << 1);
        assert_eq!(o.hidden_subtab_bits[0], 1 << 1);

        // Out-of-range ids are ignored.
        o.toggle_tab_visibility(11);
        o.toggle_subtab_visibility(11, 0);
        o.toggle_subtab_visibility(0, 32);
        assert_eq!(o.hidden_tab_bits, 0);
        assert_eq!(o.hidden_subtab_bits[0], 1 << 1);

        o.toggle_tab_visibility(3);
        o.show_all_tabs();
        assert_eq!(o.hidden_tab_bits, 0);
        assert_eq!(o.hidden_subtab_bits, [0; TAB_COUNT]);
    }

    #[test]
    fn sidebar_resource_accepts_any_id() {
        let mut o = Options::new();
        assert_eq!(o.sidebar_resource_id, 0);
        o.set_sidebar_resource(3);
        assert_eq!(o.sidebar_resource_id, 3);
        // Past-frontier ids from imported saves are preserved.
        o.set_sidebar_resource(14);
        assert_eq!(o.sidebar_resource_id, 14);
    }

    #[test]
    fn offline_ticks_accepts_any_positive_value() {
        let mut o = Options::new();
        assert_eq!(o.offline_ticks, DEFAULT_OFFLINE_TICKS);

        // Values outside our slider range are kept (we diverge from the original
        // and accept imported values as-is).
        o.set_offline_ticks(500);
        assert_eq!(o.offline_ticks, 500);
        o.set_offline_ticks(50_000_000);
        assert_eq!(o.offline_ticks, 50_000_000);

        // Zero falls back to a single tick.
        o.set_offline_ticks(0);
        assert_eq!(o.offline_ticks, 1);
    }
}
