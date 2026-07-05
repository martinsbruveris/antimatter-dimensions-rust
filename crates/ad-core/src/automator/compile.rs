//! Validation + compilation: AST → `Vec<CompiledCommand>`, mirroring the
//! original's `Validator` and `Compiler` visitors (`compiler.js` +
//! `automator-commands.js` `validate`/`compile` hooks). Validation is
//! game-state-dependent — the same script can be valid on one save and not
//! another. Compilation happens only when there are no errors, like the
//! original.

use crate::autobuyers::AutobuyerTarget;
use crate::state::GameState;
use crate::time_studies::time_study_def;

use super::lexer::{lex, CmpOp};
use super::parser::{
    self, AutoArgAst, CmpValueAst, CommandAst, ComparisonAst, ParsedCommand,
    PauseArgAst, PresetRefAst, StudiesArgAst, StudyListAst, StudyListEntryAst,
    UntilCondAst, WaitCondAst,
};
use super::program::{
    AutoSetting, AutomatorCurrency, CmpValue, Comparison, CompiledCommand, Instruction,
    PauseText, PrestigeLayer, UntilCondition, WaitCondition,
};
use super::{is_valid_number_string, parse_decimal_literal, AutomatorError};

/// The result of compiling a script: the errors (empty = valid), and the
/// compiled program when valid.
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub errors: Vec<AutomatorError>,
    pub commands: Option<Vec<CompiledCommand>>,
}

impl GameState {
    /// Compile an Automator script against the current game state
    /// (`compile(text)` in the original — validation includes unlock checks,
    /// preset lookups and constant formats, so it needs `self`).
    pub fn compile_automator_script(&self, text: &str) -> CompileResult {
        let lexed = lex(text);
        let parsed = parser::parse(&lexed);
        let mut ctx = Compiler {
            game: self,
            errors: parsed.errors,
        };
        let compiled = ctx.compile_block(&parsed.commands);

        // One error per line, sorted (`modifyErrorMessages`).
        let mut errors = ctx.errors;
        errors.sort_by_key(|e| e.line);
        errors.dedup_by_key(|e| e.line);

        CompileResult {
            commands: if errors.is_empty() {
                Some(compiled)
            } else {
                None
            },
            errors,
        }
    }

    /// Whether a script has any compilation errors (`hasCompilationErrors`).
    pub fn automator_script_has_errors(&self, text: &str) -> bool {
        !self.compile_automator_script(text).errors.is_empty()
    }
}

struct Compiler<'a> {
    game: &'a GameState,
    errors: Vec<AutomatorError>,
}

impl Compiler<'_> {
    fn error(&mut self, line: u32, info: impl Into<String>, tip: impl Into<String>) {
        self.errors.push(AutomatorError {
            line,
            info: info.into(),
            tip: tip.into(),
        });
    }

    fn compile_block(&mut self, commands: &[ParsedCommand]) -> Vec<CompiledCommand> {
        let mut out = Vec::new();
        for cmd in commands {
            if let Some(op) = self.compile_command(cmd) {
                out.push(CompiledCommand { line: cmd.line, op });
            }
        }
        out
    }

    fn compile_command(&mut self, cmd: &ParsedCommand) -> Option<Instruction> {
        let line = cmd.line;
        match &cmd.kind {
            CommandAst::Comment { .. } | CommandAst::Blob => Some(Instruction::NoOp),
            CommandAst::Auto { layer, arg } => self.compile_auto(line, *layer, arg),
            CommandAst::BlackHole { on } => Some(Instruction::BlackHole { on: *on }),
            CommandAst::Notify { text } => {
                Some(Instruction::Notify { text: text.clone() })
            }
            CommandAst::Pause { arg } => self.compile_pause(line, arg),
            CommandAst::Prestige {
                layer,
                nowait,
                respec,
            } => {
                match layer {
                    PrestigeLayer::Eternity => {
                        if !self.game.eternity_autobuyer_unlocked() {
                            self.error(
                                line,
                                "Eternity autobuyer is not unlocked",
                                "Reach 100 Eternities to use this command",
                            );
                        }
                    }
                    PrestigeLayer::Reality => {
                        if !self.game.reality_upgrade_bought(25) {
                            self.error(
                                line,
                                "Reality autobuyer is not unlocked",
                                "Purchase the Reality Upgrade which unlocks the \
                                 Reality autobuyer",
                            );
                        }
                    }
                    PrestigeLayer::Infinity => {
                        if *respec {
                            self.error(
                                line,
                                "There's no 'respec' for infinity",
                                "Remove 'respec' from the command",
                            );
                        }
                    }
                }
                Some(Instruction::Prestige {
                    layer: *layer,
                    nowait: *nowait,
                    respec: *respec,
                })
            }
            CommandAst::StartDilation => Some(Instruction::StartDilation),
            CommandAst::StartEc { ec } => {
                let ec = self.check_ec_id(line, *ec)?;
                Some(Instruction::StartEc { ec })
            }
            CommandAst::StoreGameTime { action: _ } => {
                // Enslaved is celestial content — permanently locked at our
                // frontier, so this always fails validation like a
                // pre-Enslaved original save.
                self.error(
                    line,
                    "You do not yet know how to store game time",
                    "Unlock the ability to store game time",
                );
                None
            }
            CommandAst::StudiesBuy { arg, nowait } => {
                self.compile_studies_buy(line, arg, *nowait)
            }
            CommandAst::StudiesLoad { preset, nowait } => {
                self.compile_studies_load(line, preset, *nowait)
            }
            CommandAst::StudiesRespec => Some(Instruction::StudiesRespec),
            CommandAst::UnlockDilation { nowait } => {
                Some(Instruction::UnlockDilation { nowait: *nowait })
            }
            CommandAst::UnlockEc { ec, nowait } => {
                let ec = self.check_ec_id(line, *ec)?;
                Some(Instruction::UnlockEc {
                    ec,
                    nowait: *nowait,
                })
            }
            CommandAst::If {
                cmp,
                block,
                end_line,
            } => {
                let cmp = self.compile_comparison(line, cmp);
                let block = self.compile_block(block);
                Some(Instruction::If {
                    cmp: cmp?,
                    block,
                    end_line: *end_line,
                })
            }
            CommandAst::While {
                cmp,
                block,
                end_line,
            } => {
                let cmp = self.compile_comparison(line, cmp);
                let block = self.compile_block(block);
                Some(Instruction::While {
                    cmp: cmp?,
                    block,
                    end_line: *end_line,
                })
            }
            CommandAst::Until {
                cond,
                block,
                end_line,
            } => {
                let cond = match cond {
                    UntilCondAst::Cmp(cmp) => {
                        let cmp = self.compile_comparison(line, cmp);
                        Some(UntilCondition::Comparison(cmp?))
                    }
                    UntilCondAst::Prestige(layer) => {
                        Some(UntilCondition::Prestige(*layer))
                    }
                };
                let block = self.compile_block(block);
                Some(Instruction::Until {
                    cond: cond?,
                    block,
                    end_line: *end_line,
                })
            }
            CommandAst::Wait { cond } => {
                let cond = match cond {
                    WaitCondAst::Cmp(cmp) => {
                        WaitCondition::Comparison(self.compile_comparison(line, cmp)?)
                    }
                    WaitCondAst::Prestige(layer) => WaitCondition::Prestige(*layer),
                    WaitCondAst::BlackHole { off, hole } => WaitCondition::BlackHole {
                        off: *off,
                        hole: *hole,
                    },
                };
                Some(Instruction::Wait { cond })
            }
            CommandAst::Stop => Some(Instruction::Stop),
        }
    }

    /// `auto <prestige> <arg>` validation (`AutomatorCommands` "auto").
    fn compile_auto(
        &mut self,
        line: u32,
        layer: PrestigeLayer,
        arg: &AutoArgAst,
    ) -> Option<Instruction> {
        // A fixed-amount setting must use the layer's own prestige currency.
        if let AutoArgAst::Amount {
            currency,
            currency_image,
            ..
        } = arg
        {
            let expected = match layer {
                PrestigeLayer::Infinity => AutomatorCurrency::Ip,
                PrestigeLayer::Eternity => AutomatorCurrency::Ep,
                PrestigeLayer::Reality => AutomatorCurrency::Rm,
            };
            if *currency != expected {
                self.error(
                    line,
                    format!(
                        "AutomatorCurrency doesn't match prestige ({} vs {})",
                        layer.currency_name(),
                        currency_image.to_ascii_uppercase()
                    ),
                    format!(
                        "Use {} for the specified prestige resource",
                        layer.currency_name()
                    ),
                );
                return None;
            }
        }

        let advanced =
            matches!(arg, AutoArgAst::Duration { .. } | AutoArgAst::XHighest(_));
        match layer {
            PrestigeLayer::Infinity => {
                if !self
                    .game
                    .autobuyer_can_be_upgraded(AutobuyerTarget::BigCrunch)
                {
                    self.error(
                        line,
                        "Infinity autobuyer is not unlocked",
                        "Complete the Big Crunch Autobuyer challenge to use this \
                         command",
                    );
                    return None;
                }
                if advanced && !self.game.big_crunch_autobuyer_has_modes() {
                    self.error(
                        line,
                        "Advanced Infinity autobuyer settings are not unlocked",
                        "Reach 5 Eternities to use this command",
                    );
                    return None;
                }
            }
            PrestigeLayer::Eternity => {
                if !self.game.eternity_autobuyer_unlocked() {
                    self.error(
                        line,
                        "Eternity autobuyer is not unlocked",
                        "Reach 100 Eternities to use this command",
                    );
                    return None;
                }
                if advanced && !self.game.eternity_autobuyer_has_modes() {
                    self.error(
                        line,
                        "Advanced Eternity autobuyer settings are not unlocked",
                        "Purchase the Reality Upgrade which unlocks advanced \
                         Eternity autobuyer settings",
                    );
                    return None;
                }
            }
            PrestigeLayer::Reality => {
                if !self.game.reality_upgrade_bought(25) {
                    self.error(
                        line,
                        "Reality autobuyer is not unlocked",
                        "Purchase the Reality Upgrade which unlocks the Reality \
                         autobuyer",
                    );
                    return None;
                }
                if advanced {
                    self.error(
                        line,
                        "Auto Reality cannot be set to a duration or x highest",
                        "Use RM for Auto Reality",
                    );
                    return None;
                }
            }
        }

        let setting = match arg {
            AutoArgAst::On => AutoSetting::On,
            AutoArgAst::Off => AutoSetting::Off,
            AutoArgAst::Duration { ms, .. } => {
                if ms.is_nan() {
                    self.error(
                        line,
                        "Error parsing duration",
                        "Provide a properly-formatted number for time",
                    );
                    return None;
                }
                AutoSetting::DurationMs(*ms)
            }
            AutoArgAst::XHighest(num) => {
                AutoSetting::XHighest(parse_decimal_literal(num)?)
            }
            AutoArgAst::Amount { num, .. } => {
                AutoSetting::Amount(parse_decimal_literal(num)?)
            }
        };
        Some(Instruction::Auto { layer, setting })
    }

    /// `pause` validation: a literal duration or a duration-format constant
    /// (resolved here, at compile time, like the original).
    fn compile_pause(&mut self, line: u32, arg: &PauseArgAst) -> Option<Instruction> {
        match arg {
            PauseArgAst::Duration { ms, num, unit } => {
                if ms.is_nan() {
                    self.error(
                        line,
                        "Error parsing duration",
                        "Provide a properly-formatted number for time",
                    );
                    return None;
                }
                Some(Instruction::Pause {
                    ms: *ms,
                    text: PauseText::Literal(format!("{num} {unit}")),
                })
            }
            PauseArgAst::Const(name) => {
                let seconds = self
                    .game
                    .automator
                    .constants
                    .get(name)
                    .and_then(|v| v.trim().parse::<f64>().ok());
                let Some(seconds) = seconds else {
                    self.error(
                        line,
                        format!("Constant {name} is not a valid time duration constant"),
                        format!(
                            "Ensure that {name} is a number of seconds less than \
                             1.80e305"
                        ),
                    );
                    return None;
                };
                let ms = (seconds * 1000.0).trunc();
                Some(Instruction::Pause {
                    ms,
                    text: PauseText::ConstantMs(ms),
                })
            }
        }
    }

    /// EC id 1–12 (`eternityChallenge` rule).
    fn check_ec_id(&mut self, line: u32, ec: u32) -> Option<u8> {
        if !(1..=12).contains(&ec) {
            self.error(
                line,
                format!("Invalid Eternity Challenge ID {ec}"),
                format!(
                    "Eternity Challenge {ec} does not exist, use an integer between \
                     1 and 12"
                ),
            );
            return None;
        }
        Some(ec as u8)
    }

    /// `studies [nowait] purchase <list|constant>`.
    fn compile_studies_buy(
        &mut self,
        line: u32,
        arg: &StudiesArgAst,
        nowait: bool,
    ) -> Option<Instruction> {
        match arg {
            StudiesArgAst::Const(name) => {
                let value = self.game.automator.constants.get(name);
                let valid = value
                    .map(|v| crate::time_studies::is_valid_study_import(v))
                    .unwrap_or(false);
                if !valid {
                    self.error(
                        line,
                        format!("Constant {name} is not a valid Time Study constant"),
                        format!(
                            "Ensure that {name} is a properly-formatted Time Study \
                             string"
                        ),
                    );
                    return None;
                }
                // Resolved at compile time (`varInfo.value`), like the
                // original — later edits to the constant need a recompile.
                let parsed =
                    crate::time_studies::parse_study_import(value.expect("checked"));
                Some(Instruction::StudiesBuy {
                    studies: parsed.studies,
                    ec: parsed.ec,
                    start_ec: parsed.start_ec,
                    nowait,
                    display: name.clone(),
                })
            }
            StudiesArgAst::List(list) => {
                let (studies, ec, start_ec) = self.validate_study_list(line, list)?;
                Some(Instruction::StudiesBuy {
                    studies,
                    ec,
                    start_ec,
                    nowait,
                    display: list.image.clone(),
                })
            }
        }
    }

    /// The inline study list (`studyList` rule): ids, ranges, path names,
    /// `|EC[!]`.
    fn validate_study_list(
        &mut self,
        line: u32,
        list: &StudyListAst,
    ) -> Option<(Vec<u16>, u8, bool)> {
        let before = self.errors.len();
        let mut studies = Vec::new();
        for entry in &list.entries {
            match entry {
                StudyListEntryAst::Id(image) => {
                    if let Some(id) = self.check_study_number(line, image) {
                        studies.push(id);
                    }
                }
                StudyListEntryAst::Range(first, last) => {
                    let first = self.check_study_number(line, first);
                    let last = self.check_study_number(line, last);
                    if let (Some(a), Some(b)) = (first, last) {
                        studies
                            .extend((a..=b).filter(|&id| time_study_def(id).is_some()));
                    }
                }
                StudyListEntryAst::Path(path) => studies.extend(path.studies()),
                StudyListEntryAst::InfinityPath => {
                    studies.extend(super::lexer::StudyPathKind::INFINITY_PATH)
                }
            }
        }

        let mut ec = 0u8;
        if let Some(ec_image) = &list.ec {
            let n = ec_image.parse::<f64>().unwrap_or(f64::NAN);
            // 0 is allowed (saved presets contain it by default).
            if n.fract() != 0.0 || !(0.0..=12.0).contains(&n) {
                self.error(
                    line,
                    format!("Invalid Eternity Challenge ID {ec_image}"),
                    format!(
                        "Eternity Challenge {ec_image} does not exist, use an \
                         integer between 1 and 12"
                    ),
                );
            } else {
                ec = n as u8;
            }
        }

        if self.errors.len() > before {
            return None;
        }
        Some((studies, ec, list.start_ec))
    }

    /// `checkTimeStudyNumber`: the id must name an existing study (triads are
    /// Ra content and absent from our catalogue, so they fail naturally).
    fn check_study_number(&mut self, line: u32, image: &str) -> Option<u16> {
        let n = image.parse::<f64>().unwrap_or(f64::NAN);
        let id = if n.fract() == 0.0 && (0.0..=u16::MAX as f64).contains(&n) {
            n as u16
        } else {
            0
        };
        if id == 0 || time_study_def(id).is_none() {
            self.error(
                line,
                format!("Invalid Time Study identifier {image}"),
                "Make sure you copied or typed in your time study IDs correctly",
            );
            return None;
        }
        Some(id)
    }

    /// `studies [nowait] load (id N | name X)`: the preset is resolved to a
    /// slot at compile time (`$presetIndex`).
    fn compile_studies_load(
        &mut self,
        line: u32,
        preset: &PresetRefAst,
        nowait: bool,
    ) -> Option<Instruction> {
        match preset {
            PresetRefAst::Id(None) => {
                self.error(
                    line,
                    "Missing preset id",
                    "Provide the id of a saved study preset slot from the Time \
                     Studies page",
                );
                None
            }
            PresetRefAst::Id(Some(id)) => {
                if !(1..=6).contains(id) {
                    self.error(
                        line,
                        format!("Could not find a preset with an id of {id}"),
                        "Type in a valid id (1 - 6) for your study preset",
                    );
                    return None;
                }
                Some(Instruction::StudiesLoad {
                    slot: (*id - 1) as usize,
                    nowait,
                    display: format!("id {id}"),
                })
            }
            PresetRefAst::Name(name) => {
                if name.is_empty() {
                    self.error(
                        line,
                        "Missing preset name",
                        "Provide the name of a saved study preset from the Time \
                         Studies page",
                    );
                    return None;
                }
                let slot = self.game.study_presets.iter().position(|p| p.name == *name);
                let Some(slot) = slot else {
                    self.error(
                        line,
                        format!(
                            "Could not find preset named {name} (Note: Names are \
                             case-sensitive)"
                        ),
                        "Check to make sure you typed in the correct name for your \
                         study preset",
                    );
                    return None;
                };
                Some(Instruction::StudiesLoad {
                    slot,
                    nowait,
                    display: format!("name {name}"),
                })
            }
        }
    }

    /// Comparison validation: equality rejected; constants must exist and be
    /// number-formatted.
    fn compile_comparison(
        &mut self,
        line: u32,
        cmp: &ComparisonAst,
    ) -> Option<Comparison> {
        if cmp.op == CmpOp::Eq {
            self.error(
                line,
                "Please use an inequality comparison (>, <, >=, <=)",
                "Comparisons cannot be done with equality, only with inequality \
                 operators",
            );
            return None;
        }
        let left = self.compile_cmp_value(line, &cmp.left);
        let right = self.compile_cmp_value(line, &cmp.right);
        Some(Comparison {
            left: left?,
            op: cmp.op,
            right: right?,
        })
    }

    fn compile_cmp_value(&mut self, line: u32, v: &CmpValueAst) -> Option<CmpValue> {
        match v {
            CmpValueAst::Currency(c, image) => {
                Some(CmpValue::Currency(*c, image.clone()))
            }
            CmpValueAst::Number(image) => {
                let value = parse_decimal_literal(image)?;
                Some(CmpValue::Literal(value, image.clone()))
            }
            CmpValueAst::Const(name) => {
                match self.game.automator.constants.get(name) {
                    None => {
                        self.error(
                            line,
                            format!("Variable {name} has not been defined"),
                            format!(
                                "Use the definition panel to define {name} in order \
                                 to reference it, or check for typos"
                            ),
                        );
                        None
                    }
                    Some(value) if !is_valid_number_string(value) => {
                        self.error(
                            line,
                            format!("Constant {name} cannot be used for comparison"),
                            format!(
                                "Ensure that {name} contains a properly-formatted \
                                 number and not a Time Study string"
                            ),
                        );
                        None
                    }
                    // Resolved at runtime (the getter reads the live constant).
                    Some(_) => Some(CmpValue::Const(name.clone())),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automator::program::*;
    use break_infinity::Decimal;

    /// A state where everything the test scripts reference is unlocked:
    /// crunch autobuyer + modes, eternity autobuyer + modes, reality
    /// autobuyer, presets, constants.
    fn game_all_unlocked() -> GameState {
        let mut game = GameState::new();
        game.complete_challenge(12); // Infinity autobuyer
        game.eternities = Decimal::from_float(200.0); // crunch modes + eternity autobuyer
        game.reality.upgrade_bits |= (1 << 13) | (1 << 25); // adv. eternity modes + reality
        game
    }

    fn compile_ok(game: &GameState, script: &str) -> Vec<CompiledCommand> {
        let result = game.compile_automator_script(script);
        assert!(
            result.errors.is_empty(),
            "unexpected errors: {:?}",
            result.errors
        );
        result.commands.unwrap()
    }

    fn first_error(game: &GameState, script: &str) -> AutomatorError {
        let result = game.compile_automator_script(script);
        assert!(result.commands.is_none());
        result.errors.into_iter().next().expect("expected an error")
    }

    #[test]
    fn golden_script_compiles_to_expected_tree() {
        let mut game = game_all_unlocked();
        game.study_presets[1].name = "ANTI".into();
        game.study_presets[1].studies = "11,21,22".into();

        let script = "\
// climb to 1e10 EP
auto infinity 30s
auto eternity 0 ep
while ep < 1e10 {
 studies nowait purchase 11,21-33,antimatter|4!
 studies load name ANTI
 pause 10s
 eternity respec
}
auto eternity off
notify \"done\"
stop";
        let commands = compile_ok(&game, script);
        assert_eq!(commands.len(), 7);
        assert_eq!(commands[0].op, Instruction::NoOp);
        assert_eq!(
            commands[1].op,
            Instruction::Auto {
                layer: PrestigeLayer::Infinity,
                setting: AutoSetting::DurationMs(30_000.0),
            }
        );
        assert_eq!(commands[1].line, 2);
        assert_eq!(
            commands[2].op,
            Instruction::Auto {
                layer: PrestigeLayer::Eternity,
                setting: AutoSetting::Amount(Decimal::ZERO),
            }
        );

        let Instruction::While {
            cmp,
            block,
            end_line,
        } = &commands[3].op
        else {
            panic!("expected while, got {:?}", commands[3].op);
        };
        assert_eq!(cmp.display(), "ep < 1e10");
        assert_eq!(*end_line, 9);
        assert_eq!(block.len(), 4);
        assert_eq!(
            block[0].op,
            Instruction::StudiesBuy {
                studies: vec![11, 21, 22, 31, 32, 33, 71, 81, 91, 101],
                ec: 4,
                start_ec: true,
                nowait: true,
                display: "11,21-33,antimatter|4!".to_string(),
            }
        );
        assert_eq!(
            block[1].op,
            Instruction::StudiesLoad {
                slot: 1,
                nowait: false,
                display: "name ANTI".to_string(),
            }
        );
        assert_eq!(
            block[2].op,
            Instruction::Pause {
                ms: 10_000.0,
                text: PauseText::Literal("10 s".to_string()),
            }
        );
        assert_eq!(
            block[3].op,
            Instruction::Prestige {
                layer: PrestigeLayer::Eternity,
                nowait: false,
                respec: true,
            }
        );

        assert_eq!(
            commands[5].op,
            Instruction::Notify {
                text: "\"done\"".to_string(),
            }
        );
        assert_eq!(commands[6].op, Instruction::Stop);
    }

    #[test]
    fn wait_until_if_and_ec_commands() {
        let game = game_all_unlocked();
        let script = "\
unlock ec10
start ec10
wait pending completions >= 5
if ec10 completions < 5 {
 wait eternity
}
until reality {
 wait black hole bh1
}
start dilation
unlock nowait dilation";
        let commands = compile_ok(&game, script);
        assert_eq!(
            commands[0].op,
            Instruction::UnlockEc {
                ec: 10,
                nowait: false
            }
        );
        assert_eq!(commands[1].op, Instruction::StartEc { ec: 10 });
        let Instruction::Wait {
            cond: WaitCondition::Comparison(cmp),
        } = &commands[2].op
        else {
            panic!("expected wait comparison");
        };
        assert_eq!(cmp.display(), "pending completions >= 5");
        let Instruction::If { cmp, block, .. } = &commands[3].op else {
            panic!("expected if");
        };
        assert_eq!(cmp.display(), "ec10 completions < 5");
        assert_eq!(
            block[0].op,
            Instruction::Wait {
                cond: WaitCondition::Prestige(PrestigeLayer::Eternity),
            }
        );
        let Instruction::Until {
            cond: UntilCondition::Prestige(PrestigeLayer::Reality),
            block,
            ..
        } = &commands[4].op
        else {
            panic!("expected until reality");
        };
        assert_eq!(
            block[0].op,
            Instruction::Wait {
                cond: WaitCondition::BlackHole {
                    off: false,
                    hole: 1,
                },
            }
        );
        assert_eq!(commands[5].op, Instruction::StartDilation);
        assert_eq!(commands[6].op, Instruction::UnlockDilation { nowait: true });
    }

    #[test]
    fn constants_in_comparisons_and_studies() {
        let mut game = game_all_unlocked();
        game.automator_set_constant("goal", "1e300");
        game.automator_set_constant("tree", "11,21,22|0");
        game.automator_set_constant("delay", "5");

        let commands =
            compile_ok(&game, "wait ip > goal\nstudies purchase tree\npause delay");
        let Instruction::Wait {
            cond: WaitCondition::Comparison(cmp),
        } = &commands[0].op
        else {
            panic!("expected wait");
        };
        assert_eq!(cmp.right, CmpValue::Const("goal".to_string()));
        assert_eq!(
            commands[1].op,
            Instruction::StudiesBuy {
                studies: vec![11, 21, 22],
                ec: 0,
                start_ec: false,
                nowait: false,
                display: "tree".to_string(),
            }
        );
        // Constant pause durations are baked at compile time, in seconds.
        assert_eq!(
            commands[2].op,
            Instruction::Pause {
                ms: 5000.0,
                text: PauseText::ConstantMs(5000.0),
            }
        );
    }

    #[test]
    fn validation_gates_on_unlocks() {
        let game = GameState::new();
        let err = first_error(&game, "auto infinity 1e100 ip");
        assert_eq!(err.info, "Infinity autobuyer is not unlocked");
        assert_eq!(
            err.tip,
            "Complete the Big Crunch Autobuyer challenge to use this command"
        );

        let mut game = GameState::new();
        game.complete_challenge(12);
        let err = first_error(&game, "auto infinity 30s");
        assert_eq!(
            err.info,
            "Advanced Infinity autobuyer settings are not unlocked"
        );
        assert_eq!(err.tip, "Reach 5 Eternities to use this command");

        let err = first_error(&game, "eternity");
        assert_eq!(err.info, "Eternity autobuyer is not unlocked");
        assert_eq!(err.tip, "Reach 100 Eternities to use this command");

        let err = first_error(&game, "reality");
        assert_eq!(err.info, "Reality autobuyer is not unlocked");

        let mut game = game_all_unlocked();
        game.reality.upgrade_bits &= !(1 << 13);
        let err = first_error(&game, "auto eternity 5 x highest");
        assert_eq!(
            err.info,
            "Advanced Eternity autobuyer settings are not unlocked"
        );

        let game = game_all_unlocked();
        let err = first_error(&game, "auto reality 30s");
        assert_eq!(
            err.info,
            "Auto Reality cannot be set to a duration or x highest"
        );
        assert_eq!(err.tip, "Use RM for Auto Reality");
    }

    #[test]
    fn currency_must_match_prestige() {
        let game = game_all_unlocked();
        let err = first_error(&game, "auto infinity 1e10 ep");
        assert_eq!(
            err.info,
            "AutomatorCurrency doesn't match prestige (IP vs EP)"
        );
        assert_eq!(err.tip, "Use IP for the specified prestige resource");
    }

    #[test]
    fn store_game_time_is_locked_at_frontier() {
        let game = game_all_unlocked();
        let err = first_error(&game, "store game time on");
        assert_eq!(err.info, "You do not yet know how to store game time");
    }

    #[test]
    fn equality_comparisons_are_rejected() {
        let game = GameState::new();
        let err = first_error(&game, "wait ip == 5");
        assert_eq!(
            err.info,
            "Please use an inequality comparison (>, <, >=, <=)"
        );
        let err = first_error(&game, "wait ip = 5");
        assert_eq!(
            err.info,
            "Please use an inequality comparison (>, <, >=, <=)"
        );
    }

    #[test]
    fn undefined_and_malformed_constants() {
        let mut game = GameState::new();
        let err = first_error(&game, "wait ip > goal");
        assert_eq!(err.info, "Variable goal has not been defined");

        game.automator_set_constant("tree", "11,21,22");
        let err = first_error(&game, "wait ip > tree");
        assert_eq!(err.info, "Constant tree cannot be used for comparison");

        let err = first_error(&game, "pause tree");
        assert_eq!(
            err.info,
            "Constant tree is not a valid time duration constant"
        );

        game.automator_set_constant("num", "1e100");
        let err = first_error(&game, "studies purchase num");
        assert_eq!(err.info, "Constant num is not a valid Time Study constant");

        // A short numeric constant like "42" *is* a valid study-string format
        // in the original (it just names no real study and buys nothing).
        game.automator_set_constant("fake", "42");
        compile_ok(&game, "studies purchase fake");
    }

    #[test]
    fn bad_ids_and_presets() {
        let game = game_all_unlocked();
        let err = first_error(&game, "unlock ec13");
        assert_eq!(err.info, "Invalid Eternity Challenge ID 13");
        assert_eq!(
            err.tip,
            "Eternity Challenge 13 does not exist, use an integer between 1 and 12"
        );

        let err = first_error(&game, "studies purchase 11,99");
        assert_eq!(err.info, "Invalid Time Study identifier 99");

        let err = first_error(&game, "studies load id 7");
        assert_eq!(err.info, "Could not find a preset with an id of 7");

        let err = first_error(&game, "studies load id");
        assert_eq!(err.info, "Missing preset id");

        let err = first_error(&game, "studies load name NOPE");
        assert_eq!(
            err.info,
            "Could not find preset named NOPE (Note: Names are case-sensitive)"
        );
    }

    #[test]
    fn parse_errors_recover_per_line() {
        let game = GameState::new();
        let result = game
            .compile_automator_script("florble\npause 10s\nwait ip >\nnotify \"ok\"");
        assert!(result.commands.is_none());
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0].line, 1);
        assert_eq!(result.errors[0].info, "Unrecognized command \"florble\"");
        assert_eq!(
            result.errors[0].tip,
            "Check to make sure you have typed in the command name correctly"
        );
        assert_eq!(result.errors[1].line, 3);
        assert_eq!(result.errors[1].info, "Missing value for comparison");
    }

    #[test]
    fn block_brace_errors() {
        let game = GameState::new();
        let err = first_error(&game, "while ep < 5 {\npause 10s");
        assert_eq!(err.line, 1);
        assert_eq!(err.info, "Missing closing }");

        let err = first_error(&game, "pause 10s\n}");
        assert_eq!(err.line, 2);
        assert_eq!(err.tip, "Remove }");
    }

    #[test]
    fn one_error_per_line() {
        let game = GameState::new();
        // Undefined constant in a comparison produces two raw errors on the
        // same line; only one surfaces (`modifyErrorMessages`).
        let result = game.compile_automator_script("wait bad1 > bad2");
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn missing_time_unit_and_extra_tokens() {
        let game = GameState::new();
        let err = first_error(&game, "pause 10");
        assert_eq!(err.info, "Missing time unit");

        let err = first_error(&game, "stop now");
        assert_eq!(err.info, "Unexpected input now");
        assert_eq!(err.tip, "Remove now");
    }

    #[test]
    fn comparison_evaluates_against_game_state() {
        let mut game = GameState::new();
        game.antimatter = Decimal::from_float(1000.0);
        let commands = compile_ok(&game, "wait am >= 1e3");
        let Instruction::Wait {
            cond: WaitCondition::Comparison(cmp),
        } = &commands[0].op
        else {
            panic!("expected wait");
        };
        assert!(cmp.evaluate(&game));
        game.antimatter = Decimal::from_float(999.0);
        assert!(!cmp.evaluate(&game));
    }

    #[test]
    fn locked_currencies_compare_false() {
        let game = GameState::new();
        let commands =
            compile_ok(&game, "wait filter score > 5\nwait space theorems < 5");
        for cmd in &commands {
            let Instruction::Wait {
                cond: WaitCondition::Comparison(cmp),
            } = &cmd.op
            else {
                panic!("expected wait");
            };
            assert!(!cmp.evaluate(&game));
        }
    }

    #[test]
    fn constant_comparison_reads_live_value() {
        let mut game = GameState::new();
        game.automator_set_constant("goal", "100");
        game.antimatter = Decimal::from_float(500.0);
        let commands = compile_ok(&game, "wait am > goal");
        let Instruction::Wait {
            cond: WaitCondition::Comparison(cmp),
        } = &commands[0].op
        else {
            panic!("expected wait");
        };
        assert!(cmp.evaluate(&game));
        // Constants resolve at runtime: editing one affects the compiled
        // comparison without a recompile.
        game.automator_set_constant("goal", "1000");
        assert!(!cmp.evaluate(&game));
    }

    #[test]
    fn comments_and_blob_are_noops() {
        let game = GameState::new();
        let commands = compile_ok(&game, "# hi\n// there\nblob  ");
        assert!(commands.iter().all(|c| c.op == Instruction::NoOp));
    }

    #[test]
    fn until_with_comparison() {
        let game = GameState::new();
        let commands = compile_ok(&game, "until ep > 1e8 {\npause 10s\n}");
        let Instruction::Until {
            cond: UntilCondition::Comparison(cmp),
            ..
        } = &commands[0].op
        else {
            panic!("expected until comparison");
        };
        assert_eq!(cmp.display(), "ep > 1e8");
    }
}
