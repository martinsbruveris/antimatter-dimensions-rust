//! Automator script import/export (`automator-backend.js`'s serialization
//! half): the length-prefixed data format, the encoded text forms
//! (`GameSaveSerializer.encodeText` with the automator marker strings), and
//! the import application. "Full data" exports bundle the study presets and
//! constants a script references.

use crate::save::codec::{decode_text_with_markers, encode_text_with_markers};
use crate::state::GameState;

const SCRIPT_PREFIX: &str = "AntimatterDimensionsAutomatorScriptFormat";
const SCRIPT_SUFFIX: &str = "EndOfAutomatorScript";
const DATA_PREFIX: &str = "AntimatterDimensionsAutomatorDataFormat";
const DATA_SUFFIX: &str = "EndOfAutomatorData";

/// Concatenate data segments with 5-digit zero-padded lengths prepended
/// (`serializeAutomatorData`): `["blob", "11,21"]` → `"00004blob0000511,21"`.
/// Unambiguous regardless of segment contents (comments can contain almost
/// anything).
pub fn serialize_automator_data(segments: &[&str]) -> String {
    let mut out = String::new();
    for segment in segments {
        out.push_str(&format!("{:05}", segment.len()));
        out.push_str(segment);
    }
    out
}

/// Inverse of [`serialize_automator_data`]; None for malformed input.
pub fn deserialize_automator_data(data: &str) -> Option<Vec<String>> {
    if data.is_empty() {
        return None;
    }
    let mut segments = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let len: usize = rest.get(..5)?.parse().ok()?;
        rest = &rest[5..];
        let segment = rest.get(..len)?;
        segments.push(segment.to_string());
        rest = &rest[len..];
    }
    Some(segments)
}

/// A parsed single-script import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScriptImport {
    pub name: String,
    pub content: String,
}

/// A parsed full-data import (`parseFullScriptData`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFullScriptImport {
    pub name: String,
    pub content: String,
    /// Bundled presets: (0-based slot, name, studies).
    pub presets: Vec<(usize, String, String)>,
    /// Bundled constants: (name, value).
    pub constants: Vec<(String, String)>,
}

impl GameState {
    /// The 0-based preset slots a script references (`getUsedPresets`):
    /// `studies [nowait] load id <1-6>` and `load name <existing preset>`.
    pub fn automator_used_presets(&self, id: u32) -> Vec<usize> {
        let Some(script) = self.automator.scripts.get(&id) else {
            return Vec::new();
        };
        let mut found = std::collections::BTreeSet::new();
        for line in script.content.lines() {
            let lower = line.to_ascii_lowercase();
            if let Some(pos) = find_load_argument(&lower, "id") {
                let arg = line[pos..].trim();
                if let Some(digit) = arg.chars().next().and_then(|c| c.to_digit(10)) {
                    if (1..=6).contains(&digit) {
                        found.insert(digit as usize - 1);
                    }
                }
            }
            if let Some(pos) = find_load_argument(&lower, "name") {
                let name = line[pos..].trim();
                let name = name.split_whitespace().next().unwrap_or("");
                if let Some(slot) =
                    self.study_presets.iter().position(|p| p.name == name)
                {
                    found.insert(slot);
                }
            }
        }
        found.into_iter().collect()
    }

    /// The constants a script references (`getUsedConstants`): any
    /// whitespace-delimited occurrence of a defined constant's name, except
    /// directly after `name` (which would be a preset reference).
    pub fn automator_used_constants(&self, id: u32) -> Vec<String> {
        let Some(script) = self.automator.scripts.get(&id) else {
            return Vec::new();
        };
        let mut found = std::collections::BTreeSet::new();
        for line in script.content.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            for (i, word) in words.iter().enumerate() {
                if !self.automator.constants.contains_key(*word) {
                    continue;
                }
                let after_name = i > 0 && words[i - 1].eq_ignore_ascii_case("name");
                if !after_name {
                    found.insert(word.to_string());
                }
            }
        }
        found.into_iter().collect()
    }

    /// Export a script's text (`exportCurrentScriptContents`): None for a
    /// blank script.
    pub fn automator_export_script(&self, id: u32) -> Option<String> {
        let script = self.automator.scripts.get(&id)?;
        let trimmed = script.content.trim();
        if trimmed.is_empty() {
            return None;
        }
        let serialized = serialize_automator_data(&[&script.name, trimmed]);
        Some(encode_text_with_markers(
            &serialized,
            SCRIPT_PREFIX,
            SCRIPT_SUFFIX,
        ))
    }

    /// Export a script with the presets/constants it references
    /// (`exportFullScriptData`).
    pub fn automator_export_full_data(&self, id: u32) -> Option<String> {
        let script = self.automator.scripts.get(&id)?;
        let trimmed = script.content.trim();
        if trimmed.is_empty() {
            return None;
        }
        let presets = self
            .automator_used_presets(id)
            .into_iter()
            .map(|slot| {
                let preset = &self.study_presets[slot];
                format!("{}:{}:{}", slot, preset.name, preset.studies)
            })
            .collect::<Vec<_>>()
            .join("*");
        let constants = self
            .automator_used_constants(id)
            .into_iter()
            .map(|name| format!("{}:{}", name, self.automator.constants[&name]))
            .collect::<Vec<_>>()
            .join("*");
        let serialized =
            serialize_automator_data(&[&script.name, &presets, &constants, trimmed]);
        Some(encode_text_with_markers(
            &serialized,
            DATA_PREFIX,
            DATA_SUFFIX,
        ))
    }

    /// Import a script (accepting either the single-script or the full-data
    /// format; the latter's presets/constants can be skipped). Returns the
    /// new script's id, opened in the editor.
    pub fn automator_import(
        &mut self,
        raw: &str,
        ignore_presets: bool,
        ignore_constants: bool,
    ) -> Option<u32> {
        if let Some(parsed) = parse_script_contents(raw) {
            let id = self.automator_create_script(&parsed.name, &parsed.content)?;
            self.automator_select_editor_script(id);
            return Some(id);
        }
        let parsed = parse_full_script_data(raw)?;
        let id = self.automator_create_script(&parsed.name, &parsed.content)?;
        self.automator_select_editor_script(id);
        if !ignore_presets {
            for (slot, name, studies) in &parsed.presets {
                if let Some(preset) = self.study_presets.get_mut(*slot) {
                    preset.name = name.clone();
                    preset.studies = studies.clone();
                }
            }
        }
        if !ignore_constants {
            for (name, value) in &parsed.constants {
                self.automator_set_constant(name, value);
            }
        }
        Some(id)
    }
}

/// The character position right after `studies [nowait] load <kind>` on a
/// lowercased line, or None when the line isn't that command.
fn find_load_argument(lower_line: &str, kind: &str) -> Option<usize> {
    let mut rest = lower_line.trim_start();
    let mut offset = lower_line.len() - rest.len();
    for expected in ["studies", "nowait", "load", kind] {
        rest = rest.trim_start();
        offset = lower_line.len() - rest.len();
        if let Some(after) = rest.strip_prefix(expected) {
            // Word boundary.
            if after.chars().next().is_some_and(|c| !c.is_whitespace()) {
                return None;
            }
            offset += expected.len();
            rest = after;
        } else if expected == "nowait" {
            continue; // optional
        } else {
            return None;
        }
    }
    Some(offset)
}

/// Decode + parse a single-script export (`parseScriptContents`).
pub fn parse_script_contents(raw: &str) -> Option<ParsedScriptImport> {
    let decoded =
        decode_text_with_markers(raw.trim(), SCRIPT_PREFIX, SCRIPT_SUFFIX).ok()?;
    let parts = deserialize_automator_data(&decoded)?;
    let [name, content] = parts.as_slice() else {
        return None;
    };
    Some(ParsedScriptImport {
        name: name.clone(),
        content: content.clone(),
    })
}

/// Decode + parse a full-data export (`parseFullScriptData`).
pub fn parse_full_script_data(raw: &str) -> Option<ParsedFullScriptImport> {
    let decoded = decode_text_with_markers(raw.trim(), DATA_PREFIX, DATA_SUFFIX).ok()?;
    let parts = deserialize_automator_data(&decoded)?;
    let [name, preset_data, constant_data, content] = parts.as_slice() else {
        return None;
    };

    let mut presets = Vec::new();
    if !preset_data.is_empty() {
        for preset in preset_data.split('*') {
            let mut props = preset.splitn(3, ':');
            let slot = props.next()?.parse::<usize>().ok()?;
            let name = props.next()?.to_string();
            let studies = props.next()?.to_string();
            if slot < 6 {
                presets.push((slot, name, studies));
            }
        }
    }
    presets.sort_by_key(|(slot, ..)| *slot);

    let mut constants = Vec::new();
    for constant in constant_data.split('*') {
        if constant.is_empty() {
            continue;
        }
        let (key, value) = constant.split_once(':')?;
        constants.push((key.to_string(), value.to_string()));
    }

    Some(ParsedFullScriptImport {
        name: name.clone(),
        content: content.clone(),
        presets,
        constants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_prefixed_serialization_round_trips() {
        let segments = ["My Script", "pause 10s\n# has:colons*and*stars"];
        let serialized = serialize_automator_data(&segments);
        assert!(serialized.starts_with("00009My Script00032"));
        let parsed = deserialize_automator_data(&serialized).unwrap();
        assert_eq!(parsed, segments);
        assert!(deserialize_automator_data("").is_none());
        assert!(deserialize_automator_data("0000").is_none());
        assert!(deserialize_automator_data("00010ab").is_none());
    }

    #[test]
    fn script_export_import_round_trips() {
        let mut game = GameState::new();
        game.automator_rename_script(1, "Main");
        game.automator_save_script(1, "pause 10s\neternity");
        let exported = game.automator_export_script(1).unwrap();
        assert!(exported.starts_with(SCRIPT_PREFIX));
        assert!(exported.ends_with(SCRIPT_SUFFIX));

        let parsed = parse_script_contents(&exported).unwrap();
        assert_eq!(parsed.name, "Main");
        assert_eq!(parsed.content, "pause 10s\neternity");

        let mut other = GameState::new();
        let id = other.automator_import(&exported, false, false).unwrap();
        assert_eq!(other.automator.scripts[&id].name, "Main");
        assert_eq!(other.automator.scripts[&id].content, "pause 10s\neternity");
        assert_eq!(other.automator.state.editor_script, id);

        // Blank scripts refuse to export.
        assert!(game.automator_export_script(99).is_none());
        let mut blank = GameState::new();
        blank.automator_save_script(1, "   ");
        assert!(blank.automator_export_script(1).is_none());
    }

    #[test]
    fn used_presets_and_constants_are_detected() {
        let mut game = GameState::new();
        game.study_presets[1].name = "ANTI".into();
        game.study_presets[1].studies = "11,21".into();
        game.automator_set_constant("goal", "1e300");
        game.automator_set_constant("tree", "11,21,22|0");
        game.automator_save_script(
            1,
            "studies load id 3\nstudies nowait load name ANTI\nwait ip > goal\n\
             studies load name goal",
        );
        assert_eq!(game.automator_used_presets(1), vec![1, 2]);
        // `goal` after `name` is a preset reference, but it also appears as a
        // comparison operand, so it still counts.
        assert_eq!(game.automator_used_constants(1), vec!["goal"]);
    }

    #[test]
    fn full_data_export_import_round_trips() {
        let mut game = GameState::new();
        game.study_presets[0].name = "PUSH".into();
        game.study_presets[0].studies = "11,21,22|0".into();
        game.automator_set_constant("goal", "1e300");
        game.automator_rename_script(1, "Full");
        game.automator_save_script(1, "studies load name PUSH\nwait ep > goal");
        let exported = game.automator_export_full_data(1).unwrap();
        assert!(exported.starts_with(DATA_PREFIX));

        let parsed = parse_full_script_data(&exported).unwrap();
        assert_eq!(parsed.name, "Full");
        assert_eq!(
            parsed.presets,
            vec![(0, "PUSH".to_string(), "11,21,22|0".to_string())]
        );
        assert_eq!(
            parsed.constants,
            vec![("goal".to_string(), "1e300".to_string())]
        );

        // Import into a fresh save applies the bundled data.
        let mut other = GameState::new();
        let id = other.automator_import(&exported, false, false).unwrap();
        assert_eq!(other.automator.scripts[&id].name, "Full");
        assert_eq!(other.study_presets[0].name, "PUSH");
        assert_eq!(other.automator.constants["goal"], "1e300");

        // Or skips it when asked.
        let mut skipping = GameState::new();
        skipping.automator_import(&exported, true, true).unwrap();
        assert_eq!(skipping.study_presets[0].name, "");
        assert!(skipping.automator.constants.is_empty());
    }

    #[test]
    fn import_rejects_garbage() {
        let mut game = GameState::new();
        assert!(game
            .automator_import("not a script", false, false)
            .is_none());
        assert!(parse_script_contents("garbage").is_none());
    }
}
