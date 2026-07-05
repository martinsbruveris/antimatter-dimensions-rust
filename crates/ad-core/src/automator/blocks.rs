//! The Blockifier: parsed script text → block-editor structures (the
//! `blockify` hooks in `automator-commands.js` + `blockifyTextAutomator`).
//! The engine ships only the per-block *values*; the frontend merges in the
//! palette configuration (targets, patterns) and assigns UI ids.

use super::lexer::StudyPathKind;
use super::parser::{
    self, AutoArgAst, CmpValueAst, CommandAst, ComparisonAst, ParsedCommand,
    PauseArgAst, PresetRefAst, StoreGameTimeAction, StudiesArgAst, StudyListEntryAst,
    UntilCondAst, WaitCondAst,
};
use super::program::{AutomatorCurrency, PrestigeLayer};
use crate::state::GameState;

/// One block-editor block (`blockify` output). Field names serialize in the
/// camelCase the original block components use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct BlockData {
    /// The palette key ("AUTO", "IF", "STUDIES PURCHASE", …).
    pub cmd: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub single_selection_input: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub single_text_input: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub generic_input1: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub comp_operator: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub generic_input2: Option<String>,
    pub nowait: bool,
    pub respec: bool,
    /// The inner block for IF / WHILE / UNTIL.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub nest: Option<Vec<BlockData>>,
}

/// The blockified script plus whether any lines failed to convert
/// (`blockifyTextAutomator`'s validated-vs-visited count — commands that
/// don't even parse have no block form and are lost on conversion).
#[derive(Debug, Clone)]
pub struct BlockifyResult {
    pub blocks: Vec<BlockData>,
    /// Lines that could not be converted into blocks (parse failures).
    pub lost_lines: usize,
}

impl GameState {
    /// Convert script text into block-editor structures. Commands that fail
    /// *validation* (but parse) still blockify, like the original.
    pub fn automator_blockify(&self, text: &str) -> BlockifyResult {
        let lexed = super::lexer::lex(text);
        let parsed = parser::parse(&lexed);
        BlockifyResult {
            blocks: blockify_commands(&parsed.commands),
            lost_lines: parsed.errors.len(),
        }
    }

    /// Whether some lines would be irreversibly lost converting to block mode
    /// (`BlockAutomator.hasUnparsableCommands`).
    pub fn automator_has_unparsable_commands(&self, text: &str) -> bool {
        self.automator_blockify(text).lost_lines > 0
    }

    /// Switch the editor flavor (`type`); conversion of content happens in
    /// the frontend (block→text saves the regenerated text first). Like the
    /// original's `changeModes`, switching stops a running script.
    pub fn automator_set_editor_type(&mut self, block: bool) {
        self.automator.editor_type = if block {
            super::AutomatorEditorType::Block
        } else {
            super::AutomatorEditorType::Text
        };
        self.automator_stop();
    }
}

fn blockify_commands(commands: &[ParsedCommand]) -> Vec<BlockData> {
    commands.iter().filter_map(blockify_command).collect()
}

fn block(cmd: &str) -> BlockData {
    BlockData {
        cmd: cmd.to_string(),
        ..Default::default()
    }
}

/// `standardizeAutomatorValues`: currencies display as their canonical
/// uppercase form; constants/numbers pass through unchanged.
fn standardize(value: &CmpValueAst) -> String {
    match value {
        CmpValueAst::Currency(c, _) => currency_canonical(*c),
        CmpValueAst::Number(image) => image.clone(),
        CmpValueAst::Const(name) => name.clone(),
    }
}

/// The canonical uppercase form of a currency (`$autocomplete.toUpperCase()`).
fn currency_canonical(c: AutomatorCurrency) -> String {
    use AutomatorCurrency::*;
    match c {
        Am => "AM".into(),
        Ip => "IP".into(),
        Ep => "EP".into(),
        Rm => "RM".into(),
        Dt => "DT".into(),
        Tp => "TP".into(),
        Rg => "RG".into(),
        Rep => "REP".into(),
        Tt => "TT".into(),
        TotalTt => "TOTAL TT".into(),
        SpentTt => "SPENT TT".into(),
        Infinities => "INFINITIES".into(),
        BankedInfinities => "BANKED INFINITIES".into(),
        Eternities => "ETERNITIES".into(),
        Realities => "REALITIES".into(),
        PendingIp => "PENDING IP".into(),
        PendingEp => "PENDING EP".into(),
        PendingTp => "PENDING TP".into(),
        PendingRm => "PENDING RM".into(),
        PendingGlyphLevel => "PENDING GLYPH LEVEL".into(),
        TotalCompletions => "TOTAL COMPLETIONS".into(),
        PendingCompletions => "PENDING COMPLETIONS".into(),
        EcCompletions(n) => format!("EC{n} COMPLETIONS"),
        FilterScore => "FILTER SCORE".into(),
        SpaceTheorems => "SPACE THEOREMS".into(),
        TotalSpaceTheorems => "TOTAL SPACE THEOREMS".into(),
    }
}

fn comparison_fields(b: &mut BlockData, cmp: &ComparisonAst) {
    b.generic_input1 = Some(standardize(&cmp.left));
    b.comp_operator = Some(cmp.op.symbol().to_string());
    b.generic_input2 = Some(standardize(&cmp.right));
}

fn study_list_image(arg: &StudiesArgAst) -> String {
    match arg {
        StudiesArgAst::Const(name) => name.clone(),
        StudiesArgAst::List(list) => {
            // Rebuild from entries (close to the original's raw image).
            let mut out = String::new();
            for (i, entry) in list.entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match entry {
                    StudyListEntryAst::Id(id) => out.push_str(id),
                    StudyListEntryAst::Range(a, z) => out.push_str(&format!("{a}-{z}")),
                    StudyListEntryAst::Path(p) => out.push_str(path_name(*p)),
                    StudyListEntryAst::InfinityPath => out.push_str("infinity"),
                }
            }
            if let Some(ec) = &list.ec {
                out.push_str(&format!("|{ec}"));
                if list.start_ec {
                    out.push('!');
                }
            }
            out
        }
    }
}

fn path_name(p: StudyPathKind) -> &'static str {
    match p {
        StudyPathKind::Idle => "idle",
        StudyPathKind::Passive => "passive",
        StudyPathKind::Active => "active",
        StudyPathKind::Antimatter => "antimatter",
        StudyPathKind::Time => "time",
        StudyPathKind::Light => "light",
        StudyPathKind::Dark => "dark",
    }
}

fn layer_name(layer: PrestigeLayer) -> String {
    layer.name().to_ascii_uppercase()
}

fn blockify_command(cmd: &ParsedCommand) -> Option<BlockData> {
    let b = match &cmd.kind {
        CommandAst::Auto { layer, arg } => {
            let mut b = block("AUTO");
            b.single_selection_input = Some(layer_name(*layer));
            b.single_text_input = Some(match arg {
                AutoArgAst::On => "ON".into(),
                AutoArgAst::Off => "OFF".into(),
                AutoArgAst::Duration { num, unit, .. } => format!("{num} {unit}"),
                AutoArgAst::XHighest(num) => format!("{num} x highest"),
                AutoArgAst::Amount {
                    num,
                    currency_image,
                    ..
                } => format!("{num} {}", currency_image.to_ascii_uppercase()),
            });
            b
        }
        CommandAst::BlackHole { on } => {
            let mut b = block("BLACK HOLE");
            b.single_selection_input = Some(if *on { "ON" } else { "OFF" }.into());
            b
        }
        CommandAst::Blob => block("BLOB"),
        CommandAst::Comment { text } => {
            let mut b = block("COMMENT");
            b.single_text_input = Some(text.clone());
            b
        }
        CommandAst::If {
            cmp, block: inner, ..
        } => {
            let mut b = block("IF");
            comparison_fields(&mut b, cmp);
            b.nest = Some(blockify_commands(inner));
            b
        }
        CommandAst::While {
            cmp, block: inner, ..
        } => {
            let mut b = block("WHILE");
            comparison_fields(&mut b, cmp);
            b.nest = Some(blockify_commands(inner));
            b
        }
        CommandAst::Until {
            cond, block: inner, ..
        } => {
            let mut b = block("UNTIL");
            match cond {
                UntilCondAst::Cmp(cmp) => comparison_fields(&mut b, cmp),
                UntilCondAst::Prestige(layer) => {
                    b.generic_input1 = Some(layer_name(*layer))
                }
            }
            b.nest = Some(blockify_commands(inner));
            b
        }
        CommandAst::Notify { text } => {
            let mut b = block("NOTIFY");
            b.single_text_input = Some(text.clone());
            b
        }
        CommandAst::Pause { arg } => {
            let mut b = block("PAUSE");
            b.single_text_input = Some(match arg {
                PauseArgAst::Duration { num, unit, .. } => format!("{num} {unit}"),
                PauseArgAst::Const(name) => name.clone(),
            });
            b
        }
        CommandAst::Prestige {
            layer,
            nowait,
            respec,
        } => {
            let mut b = block(&layer_name(*layer));
            b.nowait = *nowait;
            b.respec = *respec;
            b
        }
        CommandAst::StartDilation => {
            let mut b = block("START");
            b.single_selection_input = Some("DILATION".into());
            b
        }
        CommandAst::StartEc { ec } => {
            let mut b = block("START");
            b.single_selection_input = Some("EC".into());
            b.single_text_input = Some(ec.to_string());
            b
        }
        CommandAst::StoreGameTime { action } => {
            let mut b = block("STORE GAME TIME");
            b.single_selection_input = Some(
                match action {
                    StoreGameTimeAction::On => "ON",
                    StoreGameTimeAction::Off => "OFF",
                    StoreGameTimeAction::Use => "USE",
                }
                .into(),
            );
            b
        }
        CommandAst::StudiesBuy { arg, nowait } => {
            let mut b = block("STUDIES PURCHASE");
            b.single_text_input = Some(study_list_image(arg));
            b.nowait = *nowait;
            b
        }
        CommandAst::StudiesLoad { preset, nowait } => {
            let mut b = block("STUDIES LOAD");
            match preset {
                PresetRefAst::Id(id) => {
                    b.single_selection_input = Some("ID".into());
                    b.single_text_input =
                        Some(id.map(|d| d.to_string()).unwrap_or_default());
                }
                PresetRefAst::Name(name) => {
                    b.single_selection_input = Some("NAME".into());
                    b.single_text_input = Some(name.clone());
                }
            }
            b.nowait = *nowait;
            b
        }
        CommandAst::StudiesRespec => block("STUDIES RESPEC"),
        CommandAst::UnlockDilation { nowait } => {
            let mut b = block("UNLOCK");
            b.single_selection_input = Some("DILATION".into());
            b.nowait = *nowait;
            b
        }
        CommandAst::UnlockEc { ec, nowait } => {
            let mut b = block("UNLOCK");
            b.single_selection_input = Some("EC".into());
            b.single_text_input = Some(ec.to_string());
            b.nowait = *nowait;
            b
        }
        CommandAst::Wait { cond } => {
            let mut b = block("WAIT");
            match cond {
                WaitCondAst::Cmp(cmp) => comparison_fields(&mut b, cmp),
                WaitCondAst::Prestige(layer) => {
                    b.generic_input1 = Some(layer_name(*layer))
                }
                WaitCondAst::BlackHole { off, hole } => {
                    b.generic_input1 = Some("BLACK HOLE".into());
                    b.comp_operator = Some(if *off {
                        "OFF".into()
                    } else {
                        format!("BH{hole}")
                    });
                }
            }
            b
        }
        CommandAst::Stop => block("STOP"),
    };
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blockify(text: &str) -> Vec<BlockData> {
        GameState::new().automator_blockify(text).blocks
    }

    #[test]
    fn simple_commands_blockify() {
        let blocks = blockify("studies respec\neternity nowait respec\nstop");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].cmd, "STUDIES RESPEC");
        assert_eq!(blocks[1].cmd, "ETERNITY");
        assert!(blocks[1].nowait);
        assert!(blocks[1].respec);
        assert_eq!(blocks[2].cmd, "STOP");
    }

    #[test]
    fn comparisons_standardize_currencies() {
        let blocks = blockify("while banked infinities < 5e10 {\nnotify \"x\"\n}");
        let b = &blocks[0];
        assert_eq!(b.cmd, "WHILE");
        assert_eq!(b.generic_input1.as_deref(), Some("BANKED INFINITIES"));
        assert_eq!(b.comp_operator.as_deref(), Some("<"));
        assert_eq!(b.generic_input2.as_deref(), Some("5e10"));
        let nest = b.nest.as_ref().unwrap();
        assert_eq!(nest[0].cmd, "NOTIFY");
        assert_eq!(nest[0].single_text_input.as_deref(), Some("\"x\""));
    }

    #[test]
    fn studies_and_auto_blockify() {
        let blocks = blockify(
            "studies nowait purchase 11,21-33,antimatter|4!\nauto infinity 30 s\n\
             studies load id 2\nwait black hole bh1",
        );
        assert_eq!(blocks[0].cmd, "STUDIES PURCHASE");
        assert!(blocks[0].nowait);
        assert_eq!(
            blocks[0].single_text_input.as_deref(),
            Some("11,21-33,antimatter|4!")
        );
        assert_eq!(blocks[1].cmd, "AUTO");
        assert_eq!(
            blocks[1].single_selection_input.as_deref(),
            Some("INFINITY")
        );
        assert_eq!(blocks[1].single_text_input.as_deref(), Some("30 s"));
        assert_eq!(blocks[2].cmd, "STUDIES LOAD");
        assert_eq!(blocks[2].single_selection_input.as_deref(), Some("ID"));
        assert_eq!(blocks[2].single_text_input.as_deref(), Some("2"));
        assert_eq!(blocks[3].cmd, "WAIT");
        assert_eq!(blocks[3].generic_input1.as_deref(), Some("BLACK HOLE"));
        assert_eq!(blocks[3].comp_operator.as_deref(), Some("BH1"));
    }

    #[test]
    fn unparsable_lines_are_counted_as_lost() {
        let game = GameState::new();
        let result = game.automator_blockify("florble\npause 10s");
        assert_eq!(result.lost_lines, 1);
        assert_eq!(result.blocks.len(), 1);
        assert!(game.automator_has_unparsable_commands("florble"));
        assert!(!game.automator_has_unparsable_commands("pause 10s"));
    }
}
