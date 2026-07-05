//! The Automator (Feature 6.6). Stage B: the language core — lexer, parser,
//! validator/compiler — plus script & constant storage with the original's
//! limits. Execution (the stack machine) is Stage C. See
//! `design-docs/2026-07-05-automator.md`.
//!
//! Mirrors `src/core/automator/{lexer,parser,compiler,automator-commands}.js`
//! and the storage half of `automator-backend.js`.

pub mod compile;
pub mod exec;
pub mod lexer;
pub mod parser;
pub mod program;

use std::collections::{BTreeMap, HashMap};

use break_infinity::Decimal;

use crate::state::GameState;

pub use compile::CompileResult;
pub use program::{
    AutoSetting, AutomatorCurrency, CmpValue, Comparison, CompiledCommand, Instruction,
    PrestigeLayer,
};

// --- Limits (AutomatorData.MAX_ALLOWED_*) -------------------------------------

pub const MAX_SCRIPT_CHARS: usize = 10_000;
pub const MAX_TOTAL_CHARS: usize = 60_000;
pub const MAX_SCRIPT_NAME_LENGTH: usize = 15;
pub const MAX_SCRIPT_COUNT: usize = 20;
pub const MAX_CONSTANT_NAME_LENGTH: usize = 20;
pub const MAX_CONSTANT_VALUE_LENGTH: usize = 250;
pub const MAX_CONSTANT_COUNT: usize = 30;

/// A compilation/validation error shown in the editor's error panel:
/// the offending line, what is wrong, and a suggested fix (`tip`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomatorError {
    /// 1-based script line.
    pub line: u32,
    pub info: String,
    pub tip: String,
}

/// One stored script (`player.reality.automator.scripts[id]`; the id is the
/// map key).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutomatorScript {
    pub name: String,
    pub content: String,
}

/// The editor flavor (`AUTOMATOR_TYPE`: TEXT = 0, BLOCK = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AutomatorEditorType {
    #[default]
    Text,
    Block,
}

/// Execution modes (`AUTOMATOR_MODE`: PAUSE = 1, RUN = 2, SINGLE_STEP = 3).
/// Stage B stores this passively; Stage C gives it behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AutomatorMode {
    #[default]
    Pause,
    Run,
    SingleStep,
}

/// Per-command scratch state persisted on the execution stack
/// (`commandState`): a `pause`'s elapsed timer, a `wait <event>`/`until
/// <event>`'s seen-prestige level, or an entered `if`'s exit bookkeeping.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommandStateData {
    Pause {
        time_ms: f64,
    },
    PrestigeLevel {
        level: u8,
    },
    IfEntered {
        advance_on_pop: bool,
        if_end_line: u32,
    },
}

/// One persisted stack entry (`state.stack[i]`): the running command's line
/// plus its scratch state. On load, Stage C re-matches these line numbers
/// against the compiled script to rebuild the transient indices.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StackEntryData {
    pub line_number: u32,
    pub command_state: Option<CommandStateData>,
}

/// The persisted run state (`player.reality.automator.state`). Stage B only
/// stores it; the fields become live with the Stage C executor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutomatorStateData {
    pub mode: AutomatorMode,
    /// The id of the running script.
    pub top_level_script: u32,
    /// The id of the script open in the editor.
    pub editor_script: u32,
    /// Restart the script from the top when it completes.
    pub repeat: bool,
    /// Restart the script on any Reality.
    pub force_restart: bool,
    /// Auto-scroll the editor to the executing line.
    pub follow_execution: bool,
    pub stack: Vec<StackEntryData>,
}

impl Default for AutomatorStateData {
    fn default() -> Self {
        Self {
            mode: AutomatorMode::Pause,
            top_level_script: 1,
            editor_script: 1,
            repeat: true,
            force_restart: true,
            follow_execution: true,
            stack: Vec::new(),
        }
    }
}

/// All persistent Automator data (`player.reality.automator` minus
/// `forceUnlock`, which lives on [`crate::reality::RealityState`]).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AutomatorData {
    /// Scripts by id. Deletion leaves gaps; new scripts fill the first gap.
    pub scripts: BTreeMap<u32, AutomatorScript>,
    /// Named constants (values are stored as the typed strings).
    pub constants: HashMap<String, String>,
    /// Constant display order (`constantSortOrder`).
    pub constant_sort_order: Vec<String>,
    /// Current editor flavor (`type`).
    pub editor_type: AutomatorEditorType,
    /// Which docs pane is open (`currentInfoPane`, `AutomatorPanels` 0–7).
    pub current_info_pane: u8,
    /// Accumulated execution time toward the next command (`execTimer`).
    pub exec_timer: f64,
    pub state: AutomatorStateData,
    /// Transient execution scaffolding (compiled program, frame indices, the
    /// event log) — rebuilt after load, never saved.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub runtime: AutomatorRuntime,
}

/// One event-log entry (`AutomatorData.eventLog[i]`). The original stamps
/// wall-clock times; the engine has no wall clock, so entries carry the
/// engine's play-time clock instead (the Stage D log formats either).
#[derive(Debug, Clone, PartialEq)]
pub struct AutomatorEvent {
    pub message: String,
    /// 1-based script line the command sat on.
    pub line: u32,
    /// `Time.thisRealityRealTime` at log time (ms).
    pub this_reality_ms: f64,
    /// `records.realTimePlayed` at log time (replaces `Date.now()`).
    pub play_time_ms: f64,
    /// Time since the previous entry (ms).
    pub timegap_ms: f64,
}

/// The transient (never saved) execution scaffolding: the compiled top-level
/// script, the runtime half of the stack, and the log/wait bookkeeping the
/// original keeps on the non-persistent `AutomatorData` object.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutomatorRuntime {
    /// The compiled running script (`AutomatorScript._compiled` of the
    /// top-level script). None = not compiled yet or compile failed.
    pub program: Option<Vec<CompiledCommand>>,
    /// The runtime stack: `indices[d]` is the current command index inside
    /// the block at depth `d` (the persistent halves — line numbers and
    /// command state — live in `state.stack`, kept in lockstep).
    pub indices: Vec<usize>,
    /// Whether the post-load resume (`initializeFromSave`) has run.
    pub initialized: bool,
    /// Guards the end-of-script no-op handling (`hasJustCompleted`).
    pub has_just_completed: bool,
    /// A `wait` is in progress (dedupes its log entries; `isWaiting`).
    pub is_waiting: bool,
    /// Play-time when the current wait started (`waitStart`).
    pub wait_start_ms: f64,
    /// The real duration of the current engine tick, feeding `pause` timers
    /// (`Time.unscaledDeltaTime`).
    pub tick_dt_ms: f64,
    /// The event log (transient, like the original's `eventLog`).
    pub events: Vec<AutomatorEvent>,
    /// Play-time of the last log entry (`lastEvent`).
    pub last_event_ms: f64,
    /// Toasts queued by `notify` commands; the frontend drains these.
    pub pending_notifications: Vec<String>,
    /// The last prestige's gain per layer (IP/EP/RM), for
    /// `findLastPrestigeRecord` log text.
    pub last_prestige_gain: [Decimal; 3],
    /// EC completions banked by the last Eternity (`lastECCompletionCount`).
    pub last_ec_completions: u8,
}

impl AutomatorData {
    pub fn new() -> Self {
        let mut scripts = BTreeMap::new();
        // A fresh save holds one empty "New Script" (`_createDefaultScript`).
        scripts.insert(
            1,
            AutomatorScript {
                name: "New Script".to_string(),
                content: String::new(),
            },
        );
        Self {
            scripts,
            constants: HashMap::new(),
            constant_sort_order: Vec::new(),
            editor_type: AutomatorEditorType::Text,
            current_info_pane: 0,
            exec_timer: 0.0,
            state: AutomatorStateData::default(),
            runtime: AutomatorRuntime::default(),
        }
    }

    /// Total characters across all scripts (`totalScriptCharacters`).
    pub fn total_script_chars(&self) -> usize {
        self.scripts.values().map(|s| s.content.len()).sum()
    }

    /// The first free script id (ids start at 1; deletion leaves gaps which
    /// new scripts fill — `AutomatorScript.create`'s missing-index scan).
    fn first_free_id(&self) -> u32 {
        let mut id = 1;
        while self.scripts.contains_key(&id) {
            id += 1;
        }
        id
    }
}

impl Default for AutomatorData {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    // --- Script CRUD (`AutomatorBackend` / `AutomatorData`) --------------------

    /// Create a script with the given name and content, returning its id
    /// (`AutomatorScript.create`). Fails (None) at the 20-script cap.
    pub fn automator_create_script(&mut self, name: &str, content: &str) -> Option<u32> {
        if self.automator.scripts.len() >= MAX_SCRIPT_COUNT {
            return None;
        }
        let id = self.automator.first_free_id();
        let name: String = name.chars().take(MAX_SCRIPT_NAME_LENGTH).collect();
        self.automator.scripts.insert(
            id,
            AutomatorScript {
                name,
                content: content.to_string(),
            },
        );
        Some(id)
    }

    /// Create a fresh script with a unique default name
    /// (`AutomatorBackend.newScript`: "New Script", then "New Script (2)", …).
    pub fn automator_new_script(&mut self) -> Option<u32> {
        let names: Vec<&str> = self
            .automator
            .scripts
            .values()
            .map(|s| s.name.as_str())
            .collect();
        let name = if !names.contains(&"New Script") {
            "New Script".to_string()
        } else {
            let mut n = 2;
            while names.contains(&format!("New Script ({n})").as_str()) {
                n += 1;
            }
            format!("New Script ({n})")
        };
        self.automator_create_script(&name, "")
    }

    /// Overwrite a script's content (`AutomatorBackend.saveScript`). Rejects
    /// (returns false) past the character limits — the original refuses to
    /// persist over-limit edits.
    pub fn automator_save_script(&mut self, id: u32, content: &str) -> bool {
        let Some(script) = self.automator.scripts.get(&id) else {
            return false;
        };
        let others = self.automator.total_script_chars() - script.content.len();
        if content.len() > MAX_SCRIPT_CHARS || others + content.len() > MAX_TOTAL_CHARS {
            return false;
        }
        self.automator
            .scripts
            .get_mut(&id)
            .expect("checked")
            .content = content.to_string();
        // Editing the running script stops it and drops the stale compiled
        // form (`saveScript`: `if (id === state.topLevelScript) this.stop()`).
        if id == self.automator.state.top_level_script {
            self.automator_stop();
            self.automator.runtime.program = None;
        }
        true
    }

    /// Rename a script (truncated to 15 chars, like the editor input).
    pub fn automator_rename_script(&mut self, id: u32, name: &str) -> bool {
        let Some(script) = self.automator.scripts.get_mut(&id) else {
            return false;
        };
        script.name = name.chars().take(MAX_SCRIPT_NAME_LENGTH).collect();
        true
    }

    /// Delete a script (`AutomatorBackend.deleteScript`): ids are never
    /// re-indexed; deleting the last script recreates the default one; a
    /// dangling running/editor script id moves to the first remaining.
    pub fn automator_delete_script(&mut self, id: u32) -> bool {
        if self.automator.scripts.remove(&id).is_none() {
            return false;
        }
        // Deleting the running script stops it (`deleteScript`).
        if id == self.automator.state.top_level_script {
            self.automator_stop();
            self.automator.runtime.program = None;
        }
        if self.automator.scripts.is_empty() {
            self.automator.scripts.insert(
                1,
                AutomatorScript {
                    name: "New Script".to_string(),
                    content: String::new(),
                },
            );
        }
        let first = *self.automator.scripts.keys().next().expect("non-empty");
        if !self
            .automator
            .scripts
            .contains_key(&self.automator.state.top_level_script)
        {
            self.automator.state.top_level_script = first;
        }
        if !self
            .automator
            .scripts
            .contains_key(&self.automator.state.editor_script)
        {
            self.automator.state.editor_script = first;
        }
        true
    }

    // --- Constant CRUD ----------------------------------------------------------

    /// Add or overwrite a constant (`modifyConstant`/`addConstant`): the name
    /// must be a valid, non-reserved identifier; lengths are capped; new
    /// constants fail at the 30-constant cap.
    pub fn automator_set_constant(&mut self, name: &str, value: &str) -> bool {
        if !is_valid_constant_name(name)
            || name.len() > MAX_CONSTANT_NAME_LENGTH
            || value.len() > MAX_CONSTANT_VALUE_LENGTH
        {
            return false;
        }
        let auto = &mut self.automator;
        if !auto.constants.contains_key(name) {
            if auto.constants.len() >= MAX_CONSTANT_COUNT {
                return false;
            }
            auto.constant_sort_order.push(name.to_string());
        }
        auto.constants.insert(name.to_string(), value.to_string());
        true
    }

    /// Rename a constant, keeping its sort position (`renameConstant`).
    pub fn automator_rename_constant(&mut self, old: &str, new: &str) -> bool {
        if !is_valid_constant_name(new)
            || new.len() > MAX_CONSTANT_NAME_LENGTH
            || self.automator.constants.contains_key(new)
        {
            return false;
        }
        let Some(value) = self.automator.constants.remove(old) else {
            return false;
        };
        self.automator.constants.insert(new.to_string(), value);
        if let Some(slot) = self
            .automator
            .constant_sort_order
            .iter_mut()
            .find(|n| *n == old)
        {
            *slot = new.to_string();
        }
        true
    }

    /// Delete a constant (`deleteConstant`).
    pub fn automator_delete_constant(&mut self, name: &str) -> bool {
        if self.automator.constants.remove(name).is_none() {
            return false;
        }
        self.automator.constant_sort_order.retain(|n| n != name);
        true
    }
}

/// The original's `NumberLiteral` format (`AUTOMATOR_VAR_TYPES.NUMBER`
/// validation regex): `-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?`, full-match.
pub fn is_valid_number_string(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    // (0|[1-9]\d*)
    if bytes[0] == b'0' {
        i = 1;
    } else if bytes[0].is_ascii_digit() {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return false;
    }
    // (\.\d+)?
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    // ([eE][+-]?\d+)?
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    i == bytes.len()
}

/// Parse a number literal (the lexer's `NumberLiteral` / a numeric constant)
/// into a `Decimal`, handling exponents past f64 range like the original's
/// `new Decimal("1e400")`.
pub fn parse_decimal_literal(s: &str) -> Option<Decimal> {
    let s = s.trim();
    let (sign, body) = match s.strip_prefix('-') {
        Some(rest) => (-1.0, rest),
        None => (1.0, s),
    };
    if body.is_empty() {
        return None;
    }
    let (mantissa_str, exp) = match body.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i64>().ok()?),
        None => (body, 0),
    };
    let mantissa = mantissa_str.parse::<f64>().ok()?;
    if !mantissa.is_finite() {
        return None;
    }
    Some(Decimal::new(sign * mantissa, exp))
}

/// Words reserved by the language — a constant may not shadow any token word
/// (the original's `forbiddenConstantPatterns`, phrases split on whitespace).
const RESERVED_WORDS: &[&str] = &[
    "am",
    "ip",
    "ep",
    "rm",
    "dt",
    "tp",
    "rg",
    "rep",
    "replicanti",
    "tt",
    "infinities",
    "banked",
    "eternities",
    "realities",
    "pending",
    "glyph",
    "level",
    "completions",
    "total",
    "spent",
    "filter",
    "score",
    "space",
    "theorems",
    "theorem",
    "auto",
    "buy",
    "blob",
    "if",
    "load",
    "notify",
    "nowait",
    "off",
    "on",
    "pause",
    "name",
    "id",
    "purchase",
    "respec",
    "restart",
    "start",
    "stop",
    "studies",
    "unlock",
    "until",
    "use",
    "wait",
    "while",
    "black",
    "hole",
    "store",
    "stored",
    "game",
    "time",
    "dilation",
    "ec",
    "x",
    "highest",
    "infinity",
    "eternity",
    "reality",
    "idle",
    "passive",
    "active",
    "antimatter",
    "light",
    "dark",
    "ms",
    "s",
    "sec",
    "second",
    "seconds",
    "m",
    "min",
    "minute",
    "minutes",
    "h",
    "hour",
    "hours",
    "bh1",
    "bh2",
];

/// Whether a constant name is usable: an identifier that doesn't shadow a
/// language word (case-insensitively) and isn't `ec<N>`-shaped.
pub fn is_valid_constant_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    if RESERVED_WORDS.contains(&lower.as_str()) {
        return false;
    }
    // `ec<digits>` is the ECLiteral token.
    if let Some(digits) = lower.strip_prefix("ec") {
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_ids_fill_gaps() {
        let mut game = GameState::new();
        assert_eq!(game.automator.scripts.len(), 1); // the default script
        let a = game.automator_create_script("A", "").unwrap();
        let b = game.automator_create_script("B", "").unwrap();
        assert_eq!((a, b), (2, 3));
        game.automator_delete_script(2);
        // The gap at 2 is refilled.
        assert_eq!(game.automator_create_script("C", "").unwrap(), 2);
    }

    #[test]
    fn new_script_names_are_unique() {
        let mut game = GameState::new();
        // The default script is already named "New Script".
        let id = game.automator_new_script().unwrap();
        assert_eq!(game.automator.scripts[&id].name, "New Script (2)");
        let id = game.automator_new_script().unwrap();
        assert_eq!(game.automator.scripts[&id].name, "New Script (3)");
    }

    #[test]
    fn deleting_last_script_recreates_default() {
        let mut game = GameState::new();
        game.automator.state.top_level_script = 1;
        assert!(game.automator_delete_script(1));
        assert_eq!(game.automator.scripts.len(), 1);
        assert_eq!(game.automator.scripts[&1].name, "New Script");
        assert_eq!(game.automator.state.top_level_script, 1);
    }

    #[test]
    fn save_script_respects_limits() {
        let mut game = GameState::new();
        assert!(game.automator_save_script(1, "eternity"));
        assert_eq!(game.automator.scripts[&1].content, "eternity");
        let too_long = "x".repeat(MAX_SCRIPT_CHARS + 1);
        assert!(!game.automator_save_script(1, &too_long));
        assert_eq!(game.automator.scripts[&1].content, "eternity");
    }

    #[test]
    fn script_count_cap() {
        let mut game = GameState::new();
        for _ in 0..(MAX_SCRIPT_COUNT - 1) {
            assert!(game.automator_new_script().is_some());
        }
        assert!(game.automator_new_script().is_none());
    }

    #[test]
    fn constant_crud_and_limits() {
        let mut game = GameState::new();
        assert!(game.automator_set_constant("first", "11,21|0"));
        assert!(game.automator_set_constant("myval", "1e100"));
        assert_eq!(game.automator.constant_sort_order, vec!["first", "myval"]);

        // Overwrite keeps the sort position.
        assert!(game.automator_set_constant("first", "42"));
        assert_eq!(game.automator.constant_sort_order, vec!["first", "myval"]);
        assert_eq!(game.automator.constants["first"], "42");

        // Rename keeps position; deleting removes from the order.
        assert!(game.automator_rename_constant("first", "renamed"));
        assert_eq!(game.automator.constant_sort_order[0], "renamed");
        assert!(game.automator_delete_constant("renamed"));
        assert_eq!(game.automator.constant_sort_order, vec!["myval"]);

        // Reserved names and bad identifiers are rejected.
        assert!(!game.automator_set_constant("wait", "1"));
        assert!(!game.automator_set_constant("EC10", "1"));
        assert!(!game.automator_set_constant("1abc", "1"));
        assert!(!game.automator_set_constant("has space", "1"));
        assert!(game.automator_set_constant("ecX", "1"));
    }

    #[test]
    fn number_string_validation() {
        assert!(is_valid_number_string("0"));
        assert!(is_valid_number_string("123"));
        assert!(is_valid_number_string("-1.5"));
        assert!(is_valid_number_string("1e400"));
        assert!(is_valid_number_string("1.5e-4"));
        assert!(!is_valid_number_string("01"));
        assert!(!is_valid_number_string("11,21"));
        assert!(!is_valid_number_string("1.5e"));
        assert!(!is_valid_number_string(""));
        assert!(!is_valid_number_string("abc"));
    }

    #[test]
    fn decimal_literal_parsing() {
        assert_eq!(
            parse_decimal_literal("123").unwrap(),
            Decimal::from_float(123.0)
        );
        assert_eq!(
            parse_decimal_literal("2.5e30").unwrap(),
            Decimal::new(2.5, 30)
        );
        // Past f64 range.
        assert_eq!(
            parse_decimal_literal("1e400").unwrap(),
            Decimal::new(1.0, 400)
        );
        assert!(parse_decimal_literal("abc").is_none());
    }
}
