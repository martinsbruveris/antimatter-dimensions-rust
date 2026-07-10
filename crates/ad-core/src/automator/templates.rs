//! Script templates (`script-templates.js` `ScriptTemplate` + the warnings
//! from `secret-formula/script-templates.js`): five parameterized generators
//! that emit ready-to-paste Automator script text plus advisory warnings.
//! Template *metadata* (names, prompts, param types) is frontend display
//! data; the generation and every game-state check live here.

use break_infinity::Decimal;

use crate::state::GameState;
use crate::time_studies::{parse_study_import, time_study_def, TsRequirement};

/// Inputs for every template (unused fields are ignored per template; the
/// frontend fills only the prompts its template defines).
#[derive(Debug, Clone)]
pub struct TemplateParams {
    /// The study tree: an import string, or a preset reference typed as
    /// `NAME <name>` / `ID <slot>` (the modal's preset buttons produce these).
    pub tree_studies: String,
    /// false = keep buying studies (block on the line), true = continue.
    pub tree_nowait: bool,
    pub final_ep: Decimal,
    pub crunches_per_eternity: u32,
    pub eternities: Decimal,
    pub infinities: Decimal,
    pub is_banked: bool,
    pub ec: u8,
    pub completions: u32,
    /// "mult" (times highest) or "time" (seconds between).
    pub auto_inf_mode: String,
    pub auto_inf_value: Decimal,
    pub auto_eter_mode: String,
    pub auto_eter_value: Decimal,
}

impl Default for TemplateParams {
    fn default() -> Self {
        Self {
            tree_studies: String::new(),
            tree_nowait: false,
            final_ep: Decimal::ZERO,
            crunches_per_eternity: 1,
            eternities: Decimal::ZERO,
            infinities: Decimal::ZERO,
            is_banked: false,
            ec: 1,
            completions: 1,
            auto_inf_mode: "mult".into(),
            auto_inf_value: Decimal::ONE,
            auto_eter_mode: "mult".into(),
            auto_eter_value: Decimal::ONE,
        }
    }
}

/// A generated template: the script text and any advisory warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedTemplate {
    pub script: String,
    pub warnings: Vec<String>,
}

/// The template's own number formatting (`ScriptTemplate.format`) —
/// deliberately notation-independent so the output always parses:
/// ≤ 1000 → two decimals, else `m.mm`e`exp`.
fn tfmt(num: &Decimal) -> String {
    if *num <= Decimal::from_float(1000.0) {
        format!("{:.2}", num.to_f64())
    } else {
        format!("{:.2}e{}", num.mantissa(), num.exponent())
    }
}

/// Structural simulation of buying `ids` in order from an empty tree
/// (`TimeStudyTree.attemptBuyArray` minus TT costs; special EC-completion
/// requirements are treated as satisfied). Returns the purchasable subset.
fn simulate_tree_purchase(ids: &[u16], has_ts201: bool) -> Vec<u16> {
    let mut bought: Vec<u16> = Vec::new();
    for &id in ids {
        let Some(def) = time_study_def(id) else {
            continue;
        };
        if bought.contains(&id) {
            continue;
        }
        let have = |list: &[u16]| list.iter().all(|s| bought.contains(s));
        let ok = match def.requirement {
            TsRequirement::AtLeastOne(list) => {
                list.is_empty() || list.iter().any(|s| bought.contains(s))
            }
            TsRequirement::All(list) => have(list),
            TsRequirement::DimensionPath(list) => {
                let paths = [71u16, 72, 73]
                    .iter()
                    .filter(|p| bought.contains(p))
                    .count();
                let allowed = if has_ts201 { 2 } else { 1 };
                have(list) && (paths < allowed || bought.contains(&id))
            }
        };
        let set_free = !def.requires_st.iter().any(|s| bought.contains(s));
        if ok && set_free {
            bought.push(id);
        }
    }
    bought
}

/// The tree data a template needs (`storeTreeData`).
struct TreeData {
    /// The script line buying the tree.
    line: String,
    /// Studies the tree names (for reachability checks).
    selected: Vec<u16>,
    /// The structurally purchasable subset from an empty tree.
    purchased: Vec<u16>,
    warnings: Vec<String>,
}

impl GameState {
    /// Generate the named template. None for an unknown template name.
    pub fn automator_template(
        &self,
        name: &str,
        params: &TemplateParams,
    ) -> Option<GeneratedTemplate> {
        let mut t = match name {
            "Climb EP" => self.template_climb_ep(params),
            "Grind Eternities" => self.template_grind_eternities(params),
            "Grind Infinities" => self.template_grind_infinities(params),
            "Complete Eternity Challenge" => self.template_do_ec(params),
            "Unlock Dilation" => self.template_unlock_dilation(params),
            _ => return None,
        };
        t.warnings.extend(self.template_metadata_warnings(name));
        Some(t)
    }

    /// The game-state advisory warnings from the template metadata
    /// (`secret-formula/script-templates.js` `warnings()` closures).
    fn template_metadata_warnings(&self, name: &str) -> Vec<String> {
        let mut list = Vec::new();
        let ru = |id: u8| self.reality_upgrade_bought(id);
        let perk = |id: u8| self.perk_applies(id);
        match name {
            "Climb EP" => {
                if !ru(10) {
                    list.push(
                        "This script will be unable to properly set Autobuyer modes \
                         without at least 100 Eternities. Consider getting Reality \
                         Upgrade \"The Boundless Flow\" before using this at the \
                         start of a Reality."
                            .into(),
                    );
                }
                if !ru(13) {
                    list.push(
                        "This template may perform poorly without Reality Upgrade \
                         \"The Eternal Flow\""
                            .into(),
                    );
                }
                if !perk(104) {
                    list.push(
                        "This template may perform poorly without Perk \"TTS\" \
                         unless you can generate Time Theorems without purchsing them"
                            .into(),
                    );
                }
            }
            "Grind Eternities" if ru(14) => {
                list.push(
                    "You probably do not need to use this due to Reality \
                     Upgrade \"The Paradoxical Forever\""
                        .into(),
                );
            }
            "Grind Infinities" => {
                if !perk(205) {
                    list.push(
                        "You will not start this Reality with Achievement \"No \
                         ethical consumption\" - grinding Infinities may be less \
                         useful than expected since they cannot be Banked until \
                         later"
                            .into(),
                    );
                }
                if ru(11) {
                    list.push(
                        "You probably do not need to use this due to Reality \
                         Upgrade \"The Knowing Existence\""
                            .into(),
                    );
                }
            }
            "Complete Eternity Challenge" => {
                if !perk(72) {
                    list.push(
                        "Eternity Challenges may not be reliably unlockable due to \
                         secondary resource requirements, consider unlocking Perk \
                         \"ECR\" before using this template"
                            .into(),
                    );
                }
                if !perk(73) {
                    list.push(
                        "Using this template without bulk completions of Eternity \
                         Challenges may lead to long scripts which are slower and \
                         difficult to modify. If you use this template, consider \
                         returning to simplify your scripts after unlocking Perk \
                         \"ECB\""
                            .into(),
                    );
                }
            }
            "Unlock Dilation" => {
                if !ru(13) {
                    list.push(
                        "This template may perform poorly without Reality Upgrade \
                         \"The Eternal Flow\""
                            .into(),
                    );
                }
                if !perk(104) {
                    list.push(
                        "This template may perform poorly without Perk \"TTS\" \
                         unless you can generate Time Theorems without purchsing them"
                            .into(),
                    );
                }
            }
            _ => {}
        }
        list
    }

    /// `storeTreeData`: resolve the tree input (preset reference or import
    /// string) into the buy line + parsed studies + warnings.
    fn tree_data(&self, params: &TemplateParams) -> TreeData {
        let nowait = if params.tree_nowait { " nowait" } else { "" };
        let input = params.tree_studies.trim();
        let mut warnings = Vec::new();

        // Preset reference: `NAME <x>` / `ID <n>` (the modal's buttons).
        let preset = if let Some(name) = input.strip_prefix("NAME ") {
            self.study_presets
                .iter()
                .enumerate()
                .find(|(_, p)| p.name == name.trim())
                .map(|(i, p)| (format!("name {}", p.name), i, p.studies.clone()))
        } else if let Some(id) = input.strip_prefix("ID ") {
            id.trim().parse::<usize>().ok().and_then(|slot| {
                self.study_presets
                    .get(slot.wrapping_sub(1))
                    .map(|p| (format!("id {slot}"), slot - 1, p.studies.clone()))
            })
        } else {
            None
        };

        let (line, studies_string) = match preset {
            Some((preset_ref, _, studies)) => {
                (format!("studies{nowait} load {preset_ref}"), studies)
            }
            None => (
                format!("studies{nowait} purchase {input}"),
                input.to_string(),
            ),
        };

        let parsed = parse_study_import(&studies_string);
        if !parsed.invalid.is_empty() {
            warnings.push("Tree contains invalid Study IDs".into());
        }
        let purchased =
            simulate_tree_purchase(&parsed.studies, parsed.studies.contains(&201));
        if purchased.len() < parsed.studies.len() {
            warnings.push(
                "Tree structure results in some unbought studies when imported \
                 with an empty tree"
                    .into(),
            );
            if !params.tree_nowait {
                warnings.push(
                    "Automator may possibly get stuck with \"Keep buying Studies\" \
                     setting"
                        .into(),
                );
            }
        }
        TreeData {
            line,
            selected: parsed.studies,
            purchased,
            warnings,
        }
    }

    /// `parseAutobuyerProp`: mode+value → the `auto <prestige>` suffix.
    fn autobuyer_suffix(mode: &str, value: &Decimal) -> String {
        if mode == "time" {
            format!("{} seconds", tfmt(value))
        } else {
            format!("{} x highest", tfmt(value))
        }
    }

    fn template_climb_ep(&self, p: &TemplateParams) -> GeneratedTemplate {
        let tree = self.tree_data(p);
        let mut lines = vec![
            "// Template: Climb EP".to_string(),
            format!(
                "notify \"Running Template Climb EP (to {})\"",
                tfmt(&p.final_ep)
            ),
            format!(
                "auto infinity {}",
                Self::autobuyer_suffix(&p.auto_inf_mode, &p.auto_inf_value)
            ),
            format!(
                "auto eternity {}",
                Self::autobuyer_suffix(&p.auto_eter_mode, &p.auto_eter_value)
            ),
            format!("while ep < {} {{", tfmt(&p.final_ep)),
            format!(" {}", tree.line),
            " studies respec".to_string(),
            " wait eternity".to_string(),
            "}".to_string(),
        ];
        // (The original inserts the tree buy inside the loop, as above.)
        let _ = &mut lines;
        GeneratedTemplate {
            script: lines.join("\n"),
            warnings: tree.warnings,
        }
    }

    fn template_grind_eternities(&self, p: &TemplateParams) -> GeneratedTemplate {
        let tree = self.tree_data(p);
        // Crunch threshold: split the IP gap from the starting value to
        // Infinity across N crunches, with a ×5 safety factor. The starting
        // IP is clamped to 1 — the original divides by a raw 0 without the
        // START perks and prints garbage; clamping keeps the output parseable.
        let gap = Decimal::from_float(f64::MAX) / self.starting_ip().max(&Decimal::ONE)
            * Decimal::from_float(5.0);
        let per_crunch = gap.pow(&Decimal::from_float(
            1.0 / p.crunches_per_eternity.max(1) as f64,
        ));
        let lines = [
            "// Template: Grind Eternities".to_string(),
            format!(
                "notify \"Running Template Grind Eternities (to {})\"",
                tfmt(&p.eternities)
            ),
            tree.line.clone(),
            "auto eternity 0 ep".to_string(),
            format!("auto infinity {} x highest", tfmt(&per_crunch)),
            format!("wait eternities > {}", tfmt(&p.eternities)),
            "auto eternity off".to_string(),
        ];
        GeneratedTemplate {
            script: lines.join("\n"),
            warnings: tree.warnings,
        }
    }

    fn template_grind_infinities(&self, p: &TemplateParams) -> GeneratedTemplate {
        let tree = self.tree_data(p);
        let mut warnings = tree.warnings;
        let mut lines = vec![
            "// Template: Grind Infinities".to_string(),
            format!(
                "notify \"Running Template Grind Infinities (to {})\"",
                tfmt(&p.infinities)
            ),
            tree.line.clone(),
            "auto eternity off".to_string(),
            "auto infinity 5s".to_string(),
        ];
        if p.is_banked {
            let has_191 = tree.purchased.contains(&191);
            if !has_191 {
                warnings.push(
                    "TS191 is not reachable from an empty tree; banking anything \
                     in this template will require Achievement \"No ethical \
                     consumption\""
                        .into(),
                );
            }
            let bank_rate = if has_191 { 0.1 } else { 0.05 };
            lines.push(
                "// Note: This template attempts to get all the Banked Infinities \
                 within a single Eternity"
                    .into(),
            );
            lines.push(format!(
                "wait infinities > {}",
                tfmt(&(p.infinities / Decimal::from_float(bank_rate)))
            ));
            lines.push("eternity".into());
        } else {
            lines.push(format!("wait infinities > {}", tfmt(&p.infinities)));
        }
        GeneratedTemplate {
            script: lines.join("\n"),
            warnings,
        }
    }

    fn template_do_ec(&self, p: &TemplateParams) -> GeneratedTemplate {
        let tree = self.tree_data(p);
        let mut warnings = tree.warnings;
        let mut lines = vec![
            "// Template: Complete Eternity Challenge".to_string(),
            format!(
                "notify \"Running Template Complete Eternity Challenge (EC{})\"",
                p.ec
            ),
            // Force an Eternity to buy the tree fresh.
            "eternity respec".to_string(),
            tree.line.clone(),
        ];

        let tree_ec = parse_study_import(p.tree_studies.trim()).ec;
        if tree_ec == 0 {
            lines.push(format!("unlock ec {}", p.ec));
            // Reachability: the EC's unlock study needs one of its
            // prerequisite studies in the tree.
            let reachable = crate::eternity_challenges::ec_study_prerequisites(p.ec)
                .iter()
                .any(|s| tree.purchased.contains(s));
            if !reachable {
                warnings.push("Specified Study Tree cannot reach specified EC".into());
            }
        } else if tree_ec != p.ec {
            warnings
                .push("Specified Study Tree already has a different EC unlocked".into());
        }

        lines.push(format!(
            "auto infinity {}",
            Self::autobuyer_suffix(&p.auto_inf_mode, &p.auto_inf_value)
        ));
        lines.push("auto eternity off".to_string());
        if !(1..=12).contains(&p.ec) {
            warnings.push("Specified template EC does not exist".into());
        }
        lines.push(format!("start ec {}", p.ec));
        if p.completions > 5 {
            warnings.push("ECs cannot be completed more than 5 times".into());
        }
        lines.push(format!("wait pending completions >= {}", p.completions));
        lines.push("eternity".to_string());
        GeneratedTemplate {
            script: lines.join("\n"),
            warnings,
        }
    }

    fn template_unlock_dilation(&self, p: &TemplateParams) -> GeneratedTemplate {
        let tree = self.tree_data(p);
        let mut warnings = tree.warnings;
        if ![231u16, 232, 233, 234]
            .iter()
            .any(|s| tree.selected.contains(s) && tree.purchased.contains(s))
        {
            warnings.push("Specified Study Tree cannot reach Dilation".into());
        }
        let lines = vec![
            "// Template: Unlock Dilation".to_string(),
            "notify \"Running Template Unlock Dilation\"".to_string(),
            "auto infinity off".to_string(),
            format!(
                "auto eternity {}",
                Self::autobuyer_suffix(&p.auto_eter_mode, &p.auto_eter_value)
            ),
            // A plain JS number in the original formats as a rounded integer
            // (`ScriptTemplate.format`'s typeof-number branch).
            format!(
                "while total tt < {} {{",
                crate::dilation::DILATION_TT_REQUIREMENT.round()
            ),
            format!(" {}", tree.line),
            " studies respec".to_string(),
            " wait eternity".to_string(),
            "}".to_string(),
            "unlock dilation".to_string(),
        ];
        GeneratedTemplate {
            script: lines.join("\n"),
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climb_ep_generates_valid_script() {
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        // Unlock everything the generated commands validate against.
        game.complete_challenge(12);
        game.eternities = Decimal::from_float(200.0);
        game.reality.upgrade_bits |= (1 << 10) | (1 << 13) | (1 << 25);

        let params = TemplateParams {
            tree_studies: "11,21,22,31,32,33".into(),
            tree_nowait: true,
            final_ep: Decimal::new(1.0, 10),
            auto_inf_mode: "mult".into(),
            auto_inf_value: Decimal::from_float(5.0),
            auto_eter_mode: "time".into(),
            auto_eter_value: Decimal::from_float(30.0),
            ..Default::default()
        };
        game.reality.perks.insert(104); // TTS, quiets the advisory warning
        let t = game.automator_template("Climb EP", &params).unwrap();
        assert!(t.script.contains("auto infinity 5.00 x highest"));
        assert!(t.script.contains("auto eternity 30.00 seconds"));
        assert!(t.script.contains("while ep < 1.00e10 {"));
        assert!(t
            .script
            .contains("studies nowait purchase 11,21,22,31,32,33"));
        assert!(
            t.warnings.is_empty(),
            "unexpected warnings: {:?}",
            t.warnings
        );

        // The generated script actually compiles.
        let result = game.compile_automator_script(&t.script);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn tree_warnings_fire() {
        let game = GameState::new();
        let params = TemplateParams {
            // 33 requires 22 (absent) — structurally unbuyable.
            tree_studies: "11,33".into(),
            tree_nowait: false,
            ..Default::default()
        };
        let t = game.automator_template("Climb EP", &params).unwrap();
        assert!(t.warnings.iter().any(|w| w.contains("unbought studies")));
        assert!(t.warnings.iter().any(|w| w.contains("get stuck")));
        // Metadata warnings fire too (no RU10/13, no TTS perk).
        assert!(t.warnings.iter().any(|w| w.contains("The Boundless Flow")));
    }

    #[test]
    fn grind_eternities_splits_the_ip_gap() {
        let game = GameState::new();
        let params = TemplateParams {
            tree_studies: "11".into(),
            crunches_per_eternity: 2,
            eternities: Decimal::from_float(100.0),
            ..Default::default()
        };
        let t = game
            .automator_template("Grind Eternities", &params)
            .unwrap();
        assert!(t.script.contains("auto eternity 0 ep"));
        assert!(t.script.contains("wait eternities > 100.00"));
        // sqrt(5 × MAX/10) ≈ 1.34e154.
        assert!(t.script.contains("e154 x highest"));
    }

    #[test]
    fn grind_infinities_banked_variant() {
        let game = GameState::new();
        let params = TemplateParams {
            tree_studies: "11".into(),
            infinities: Decimal::from_float(1e6),
            is_banked: true,
            ..Default::default()
        };
        let t = game
            .automator_template("Grind Infinities", &params)
            .unwrap();
        // Without TS191 the bank rate is 5% → ×20 the target.
        assert!(t.script.contains("wait infinities > 2.00e7"));
        assert!(t.warnings.iter().any(|w| w.contains("TS191")));
    }

    #[test]
    fn do_ec_checks_reachability() {
        let game = GameState::new();
        let params = TemplateParams {
            tree_studies: "11,21".into(), // nowhere near 171+
            ec: 5,
            completions: 3,
            ..Default::default()
        };
        let t = game
            .automator_template("Complete Eternity Challenge", &params)
            .unwrap();
        assert!(t.script.contains("unlock ec 5"));
        assert!(t.script.contains("wait pending completions >= 3"));
        assert!(t
            .warnings
            .iter()
            .any(|w| w.contains("cannot reach specified EC")));
    }

    #[test]
    fn unlock_dilation_checks_bottom_row() {
        let game = GameState::new();
        let params = TemplateParams {
            tree_studies: "11,21,22".into(),
            auto_eter_mode: "time".into(),
            auto_eter_value: Decimal::from_float(10.0),
            ..Default::default()
        };
        let t = game.automator_template("Unlock Dilation", &params).unwrap();
        assert!(t.script.contains("while total tt < 12900 {"));
        assert!(t.script.contains("unlock dilation"));
        assert!(t
            .warnings
            .iter()
            .any(|w| w.contains("cannot reach Dilation")));
    }
}
