//! The Automator's execution engine (Stage C): the stack machine, tick
//! integration, and every command's runtime behavior. Mirrors
//! `automator-backend.js` (`AutomatorBackend.update/step/nextCommand`, the
//! stack) and the `compile`d run functions in `automator-commands.js`.
//!
//! The runtime stack is a path of indices (`runtime.indices[d]` = current
//! command inside the block at depth `d`), kept in lockstep with the
//! persistent `state.stack` (line numbers + per-command scratch state). On
//! load the indices are rebuilt by matching the saved line numbers against
//! the recompiled script (`initializeFromSave`); a mismatch — the script was
//! edited — restarts the run from the top.

use break_infinity::Decimal;

use crate::state::GameState;

use super::program::{
    AutoSetting, CompiledCommand, Instruction, PauseText, PrestigeLayer, UntilCondition,
    WaitCondition,
};
use super::{AutomatorEvent, AutomatorMode, CommandStateData, StackEntryData};

/// What a command's execution tells the stepper (`AUTOMATOR_COMMAND_STATUS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandStatus {
    NextInstruction,
    NextTickSameInstruction,
    NextTickNextInstruction,
    /// Re-run the (new) current command within this step — used when a block
    /// is entered.
    SameInstruction,
    /// A no-op; advance and keep going without consuming the step.
    SkipInstruction,
    Halt,
    Restart,
}

/// At most this many commands run per engine tick (`MAX_COMMANDS_PER_UPDATE`).
const MAX_COMMANDS_PER_UPDATE: u32 = 100;

/// The `$prestigeLevel` ordering used by `wait <event>` / `until <event>`.
fn prestige_level(layer: PrestigeLayer) -> u8 {
    match layer {
        PrestigeLayer::Infinity => 1,
        PrestigeLayer::Eternity => 2,
        PrestigeLayer::Reality => 3,
    }
}

/// The command at `indices` within `program` (walking nested blocks).
fn command_at<'a>(
    program: &'a [CompiledCommand],
    indices: &[usize],
) -> Option<&'a CompiledCommand> {
    let mut block = program;
    let mut found = None;
    for &i in indices {
        let cmd = block.get(i)?;
        found = Some(cmd);
        block = cmd.op.block().unwrap_or(&[]);
    }
    found
}

/// The block containing the command at depth `depth` (0 = the top level).
fn block_at<'a>(
    program: &'a [CompiledCommand],
    indices: &[usize],
    depth: usize,
) -> Option<&'a [CompiledCommand]> {
    let mut block = program;
    for &i in indices.iter().take(depth) {
        block = block.get(i)?.op.block()?;
    }
    Some(block)
}

/// Simple engine-side number formatting for event-log text: `format(x,
/// places, places_under_1000)` approximated with plain scientific notation
/// above 1000 (the original uses the player's display notation — a cosmetic
/// deviation noted in the design doc).
fn fmt(value: &Decimal, places: usize, places_under_1000: usize) -> String {
    let abs = value.abs();
    if abs < Decimal::from_float(1000.0) {
        format!("{:.*}", places_under_1000, value.to_f64())
    } else {
        format!("{:.*}e{}", places, value.mantissa(), value.exponent())
    }
}

/// A short human duration (`TimeSpan.toStringShort` approximation).
fn fmt_duration_short(ms: f64) -> String {
    let secs = ms / 1000.0;
    if secs < 60.0 {
        format!("{secs:.2} seconds")
    } else if secs < 3600.0 {
        format!("{:.2} minutes", secs / 60.0)
    } else {
        format!("{:.2} hours", secs / 3600.0)
    }
}

impl GameState {
    // --- Public state/controls (`AutomatorBackend`) ----------------------------

    /// Milliseconds per command (`currentInterval`): each Reality makes the
    /// Automator 0.6% faster, down to 1 ms.
    pub fn automator_current_interval(&self) -> f64 {
        (0.994f64.powf(self.reality.realities as f64) * 500.0).max(1.0)
    }

    /// "On" = the stack is non-empty (a paused Automator is still on).
    pub fn automator_is_on(&self) -> bool {
        !self.automator.state.stack.is_empty()
    }

    pub fn automator_is_running(&self) -> bool {
        self.automator_is_on() && self.automator.state.mode == AutomatorMode::Run
    }

    /// The 1-based line of the current command (-1 → None when off).
    pub fn automator_current_line(&self) -> Option<u32> {
        self.automator.state.stack.last().map(|e| e.line_number)
    }

    /// Start (or restart) a script (`AutomatorBackend.start`). `script_id` of
    /// None reruns the current top-level script. Recompiles; a script with
    /// errors does not start. Only works once the Automator is unlocked.
    pub fn automator_start(&mut self, script_id: Option<u32>) -> bool {
        self.automator_start_mode(script_id, AutomatorMode::Run, true)
    }

    fn automator_start_mode(
        &mut self,
        script_id: Option<u32>,
        mode: AutomatorMode,
        recompile: bool,
    ) -> bool {
        if !self.automator_unlocked() {
            return false;
        }
        self.automator.runtime.has_just_completed = false;
        let id = script_id.unwrap_or(self.automator.state.top_level_script);
        self.automator.state.top_level_script = id;
        self.automator.exec_timer = 0.0;
        let Some(script) = self.automator.scripts.get(&id) else {
            return false;
        };
        if recompile || self.automator.runtime.program.is_none() {
            let compiled = self.compile_automator_script(&script.content.clone());
            self.automator.runtime.program = compiled.commands;
        }
        let Some(program) = &self.automator.runtime.program else {
            return false;
        };
        // Reset the stack onto the first command (empty scripts never start).
        let Some(first) = program.first() else {
            return false;
        };
        let first_line = first.line;
        self.automator.runtime.indices = vec![0];
        self.automator.state.stack = vec![StackEntryData {
            line_number: first_line,
            command_state: None,
        }];
        self.automator.state.mode = mode;
        self.automator.runtime.is_waiting = false;
        // A fresh start needs no post-load resume.
        self.automator.runtime.initialized = true;
        if self.options.automator_events.clear_on_restart {
            self.automator_clear_event_log();
        }
        true
    }

    /// Stop: clear the stack and pause (`AutomatorBackend.stop`).
    pub fn automator_stop(&mut self) {
        self.automator.state.stack.clear();
        self.automator.runtime.indices.clear();
        self.automator.state.mode = AutomatorMode::Pause;
        self.automator.runtime.has_just_completed = true;
    }

    /// Pause in place (the stack survives).
    pub fn automator_pause(&mut self) {
        self.automator.state.mode = AutomatorMode::Pause;
    }

    /// Resume a paused run (the play button's "on but paused" branch:
    /// `AutomatorBackend.mode = RUN`).
    pub fn automator_resume(&mut self) {
        if self.automator_is_on() {
            self.automator.state.mode = AutomatorMode::Run;
        }
    }

    /// The play button (`AutomatorControls.play`): pause when running, resume
    /// when paused, else start the given script.
    pub fn automator_play(&mut self, script_id: u32) {
        if self.automator_is_running() {
            self.automator_pause();
        } else if self.automator_is_on() {
            self.automator_resume();
        } else {
            self.automator_start(Some(script_id));
        }
    }

    /// Select which script the editor shows (`state.editorScript`).
    pub fn automator_select_editor_script(&mut self, id: u32) -> bool {
        if !self.automator.scripts.contains_key(&id) {
            return false;
        }
        self.automator.state.editor_script = id;
        true
    }

    /// Restart the running script from the top (`AutomatorBackend.restart`).
    pub fn automator_restart(&mut self) {
        self.automator_start_mode(None, AutomatorMode::Run, true);
    }

    /// The single-step button: run one command on the next update. Starts
    /// `script_id` (default: the last-run script) paused-at-step when the
    /// Automator is off (`AutomatorControls.step`).
    pub fn automator_step_once(&mut self, script_id: Option<u32>) -> bool {
        if self.automator_is_on() {
            self.automator.state.mode = AutomatorMode::SingleStep;
            true
        } else {
            self.automator_start_mode(script_id, AutomatorMode::SingleStep, true)
        }
    }

    pub fn automator_toggle_repeat(&mut self) {
        self.automator.state.repeat = !self.automator.state.repeat;
    }

    pub fn automator_toggle_force_restart(&mut self) {
        self.automator.state.force_restart = !self.automator.state.force_restart;
    }

    pub fn automator_toggle_follow_execution(&mut self) {
        self.automator.state.follow_execution = !self.automator.state.follow_execution;
    }

    pub fn automator_clear_event_log(&mut self) {
        self.automator.runtime.events.clear();
        self.automator.runtime.last_event_ms = 0.0;
    }

    // --- Tick integration -------------------------------------------------------

    /// Advance the Automator by one engine tick of `real_dt_ms`
    /// (`AutomatorBackend.update(diff)` — real time, unaffected by game
    /// speed).
    pub(crate) fn automator_update(&mut self, real_dt_ms: f64) {
        self.automator_ensure_initialized();
        if !self.automator_is_on() {
            return;
        }
        match self.automator.state.mode {
            AutomatorMode::Pause => return,
            AutomatorMode::SingleStep => {
                self.automator_single_step();
                self.automator.state.mode = AutomatorMode::Pause;
                return;
            }
            AutomatorMode::Run => {}
        }

        self.automator.runtime.tick_dt_ms = real_dt_ms;
        self.automator.exec_timer += real_dt_ms;
        let interval = self.automator_current_interval();
        let commands_this_update = ((self.automator.exec_timer / interval).floor()
            as u32)
            .min(MAX_COMMANDS_PER_UPDATE);
        self.automator.exec_timer -= commands_this_update as f64 * interval;

        for _ in 0..commands_this_update {
            if !self.automator_is_running() || !self.automator_step() {
                break;
            }
        }
    }

    /// The post-load resume (`initializeFromSave`, run lazily on the first
    /// update): recompile the running script and re-match the saved stack's
    /// line numbers; a mismatch restarts the run from the top, a script that
    /// no longer compiles stops it.
    fn automator_ensure_initialized(&mut self) {
        if self.automator.runtime.initialized {
            return;
        }
        self.automator.runtime.initialized = true;
        if self.automator.state.stack.is_empty() {
            return;
        }
        let id = self.automator.state.top_level_script;
        let Some(script) = self.automator.scripts.get(&id) else {
            self.automator_stop();
            return;
        };
        let compiled = self.compile_automator_script(&script.content.clone());
        let Some(program) = compiled.commands else {
            // The script no longer compiles: the stack clears (the original's
            // `this.stack.clear()` branch), mode stays.
            self.automator.state.stack.clear();
            self.automator.runtime.indices.clear();
            return;
        };

        // Match each saved frame's line number within its block.
        let mut indices = Vec::new();
        let mut block: &[CompiledCommand] = &program;
        let mut matched = true;
        for (depth, entry) in self.automator.state.stack.iter().enumerate() {
            let Some(found) = block.iter().position(|c| c.line == entry.line_number)
            else {
                matched = false;
                break;
            };
            indices.push(found);
            if depth != self.automator.state.stack.len() - 1 {
                match block[found].op.block() {
                    Some(inner) => block = inner,
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
        }

        if matched {
            self.automator.runtime.indices = indices;
        } else {
            // Could not match the stack to the script — restart from the top
            // (`if (!stack.initializeFromSave(commands)) this.reset(commands)`).
            let first_line = program[0].line;
            self.automator.runtime.indices = vec![0];
            self.automator.state.stack = vec![StackEntryData {
                line_number: first_line,
                command_state: None,
            }];
        }
        self.automator.runtime.program = Some(program);
    }

    // --- The stepper ------------------------------------------------------------

    /// Run commands until one yields the tick (`AutomatorBackend.step`).
    /// Returns whether the update loop may spend another command.
    fn automator_step(&mut self) -> bool {
        if self.automator.state.stack.is_empty() {
            return false;
        }
        for _ in 0..100 {
            if self.automator.runtime.has_just_completed {
                break;
            }
            match self.automator_run_current_command() {
                CommandStatus::SameInstruction => return true,
                CommandStatus::NextInstruction => return self.automator_next_command(),
                CommandStatus::NextTickSameInstruction => return false,
                CommandStatus::NextTickNextInstruction => {
                    self.automator_next_command();
                    return false;
                }
                CommandStatus::SkipInstruction => {
                    self.automator_next_command();
                }
                CommandStatus::Halt => {
                    self.automator_stop();
                    return false;
                }
                CommandStatus::Restart => {
                    self.automator_restart();
                    return false;
                }
            }
            if self.automator.state.stack.is_empty() {
                self.automator.runtime.has_just_completed = true;
            }
        }
        // 100 consecutive no-ops: halt so a comment-only loop can't hang the
        // game (unless we only looped through a completed script's tail).
        if !self.automator.runtime.has_just_completed {
            let line = self.automator_current_line().unwrap_or(0);
            self.log_event(
                "Automator halted due to excessive no-op commands".to_string(),
                line,
            );
            self.automator
                .runtime
                .pending_notifications
                .push("Automator halted - too many consecutive no-ops detected".into());
        }
        self.automator_stop();
        false
    }

    /// One command in SINGLE_STEP mode (`AutomatorBackend.singleStep`): always
    /// advances a line except for still-waiting commands and HALT/RESTART.
    fn automator_single_step(&mut self) {
        if self.automator.state.stack.is_empty() {
            return;
        }
        match self.automator_run_current_command() {
            CommandStatus::NextTickSameInstruction => {}
            CommandStatus::Halt => self.automator_stop(),
            CommandStatus::Restart => self.automator_restart(),
            _ => {
                self.automator_next_command();
            }
        }
    }

    /// Execute the current command (`runCurrentCommand`). The program is
    /// temporarily taken out of the runtime so command handlers can borrow it
    /// while mutating the game; a restart from within (a Reality with
    /// force-restart) installs a fresh program, which wins.
    fn automator_run_current_command(&mut self) -> CommandStatus {
        let Some(program) = self.automator.runtime.program.take() else {
            return CommandStatus::Halt;
        };
        let status = match command_at(&program, &self.automator.runtime.indices) {
            Some(cmd) => {
                let cmd = cmd.clone();
                self.run_instruction(&cmd)
            }
            None => CommandStatus::Halt,
        };
        if self.automator.runtime.program.is_none() {
            self.automator.runtime.program = Some(program);
        }
        status
    }

    /// Advance to the next command (`nextCommand`), popping finished blocks.
    fn automator_next_command(&mut self) -> bool {
        loop {
            let depth = self.automator.runtime.indices.len();
            if depth == 0 {
                return false;
            }
            let Some(program) = self.automator.runtime.program.as_ref() else {
                return false;
            };
            let indices = &self.automator.runtime.indices;
            let Some(block) = block_at(program, indices, depth - 1) else {
                return false;
            };
            let idx = indices[depth - 1];
            if idx + 1 >= block.len() {
                // Block finished: pop the frame.
                self.automator.runtime.indices.pop();
                self.automator.state.stack.pop();
                if self.automator.state.stack.is_empty() {
                    if self.automator.state.repeat {
                        self.automator_start_mode(None, AutomatorMode::Run, false);
                        return false;
                    }
                    self.automator_stop();
                    return true;
                }
                // An entered `if` advances past itself when its block pops.
                if let Some(CommandStateData::IfEntered {
                    advance_on_pop: true,
                    if_end_line,
                }) = self
                    .automator
                    .state
                    .stack
                    .last()
                    .and_then(|e| e.command_state.clone())
                {
                    self.log_event("Exiting IF block".to_string(), if_end_line);
                    continue;
                }
                return true;
            }
            // Advance within the block.
            let next_line = block[idx + 1].line;
            self.automator.runtime.indices[depth - 1] = idx + 1;
            let entry = self
                .automator
                .state
                .stack
                .last_mut()
                .expect("stack tracks indices");
            entry.command_state = None;
            entry.line_number = next_line;
            return true;
        }
    }

    /// Push a block onto the stack (`AutomatorBackend.push`); empty blocks
    /// are not pushed.
    fn automator_push_block(&mut self, block: &[CompiledCommand]) -> bool {
        let Some(first) = block.first() else {
            return false;
        };
        self.automator.runtime.indices.push(0);
        self.automator.state.stack.push(StackEntryData {
            line_number: first.line,
            command_state: None,
        });
        true
    }

    // --- Event log + prestige notifications --------------------------------------

    /// Append an event-log entry (`AutomatorData.logCommandEvent`).
    pub(crate) fn log_event(&mut self, message: String, line: u32) {
        let now = self.records.real_time_played_ms;
        let timegap = now - self.automator.runtime.last_event_ms;
        self.automator.runtime.last_event_ms = now;
        self.automator.runtime.events.push(AutomatorEvent {
            message,
            line,
            this_reality_ms: self.records.this_reality.real_time_ms,
            play_time_ms: now,
            timegap_ms: timegap,
        });
        let max = self.options.automator_events.max_entries.max(1) as usize;
        while self.automator.runtime.events.len() > max {
            self.automator.runtime.events.remove(0);
        }
    }

    /// A prestige happened (`prestigeNotify`, fired from the crunch /
    /// eternity / reality resets): record the gain for log text, and bump the
    /// *top* stack frame's seen-prestige level if it tracks one (only
    /// `wait <event>` / `until <event>` states do; this top-only behavior is
    /// a faithful quirk).
    pub(crate) fn automator_notify_prestige(
        &mut self,
        layer: PrestigeLayer,
        gained: Decimal,
    ) {
        let runtime = &mut self.automator.runtime;
        runtime.last_prestige_gain[layer as usize] = gained;
        if self.automator.state.stack.is_empty() {
            return;
        }
        if let Some(entry) = self.automator.state.stack.last_mut() {
            if let Some(CommandStateData::PrestigeLevel { level }) =
                &mut entry.command_state
            {
                *level = (*level).max(prestige_level(layer));
            }
        }
    }

    /// `findLastPrestigeRecord`: what the last prestige of `layer` granted.
    fn last_prestige_record(&self, layer: PrestigeLayer) -> String {
        let gain = &self.automator.runtime.last_prestige_gain[layer as usize];
        match layer {
            PrestigeLayer::Infinity => format!("{} IP", fmt(gain, 2, 0)),
            PrestigeLayer::Eternity => {
                let completions = self.automator.runtime.last_ec_completions;
                if completions == 0 {
                    format!("{} EP", fmt(gain, 2, 0))
                } else {
                    format!("{} EP, {completions} completions", fmt(gain, 2, 0))
                }
            }
            PrestigeLayer::Reality => format!("{} RM", fmt(gain, 2, 0)),
        }
    }

    // --- Command behaviors (`automator-commands.js` `compile` closures) ---------

    fn run_instruction(&mut self, cmd: &CompiledCommand) -> CommandStatus {
        let line = cmd.line;
        match &cmd.op {
            Instruction::NoOp => CommandStatus::SkipInstruction,
            Instruction::Stop => {
                self.log_event(
                    "Automator execution stopped with STOP command".to_string(),
                    line,
                );
                CommandStatus::Halt
            }
            Instruction::Notify { text } => {
                self.automator
                    .runtime
                    .pending_notifications
                    .push(format!("Automator: {text}"));
                self.log_event(format!("NOTIFY call: {text}"), line);
                CommandStatus::NextInstruction
            }
            Instruction::Auto { layer, setting } => self.run_auto(line, *layer, setting),
            Instruction::BlackHole { on } => {
                if *on == self.black_holes.paused {
                    self.toggle_black_hole_pause();
                }
                let message = if self.black_holes.holes[0].unlocked {
                    format!("Black Holes toggled {}", if *on { "ON" } else { "OFF" })
                } else {
                    "Black Hole command ignored because BH is not unlocked".to_string()
                };
                self.log_event(message, line);
                CommandStatus::NextInstruction
            }
            Instruction::Pause { ms, text } => self.run_pause(line, *ms, text),
            Instruction::Prestige {
                layer,
                nowait,
                respec,
            } => self.run_prestige(line, *layer, *nowait, *respec),
            Instruction::StartDilation => {
                if self.dilation.active {
                    self.log_event(
                        "Start Dilation encountered but ignored due to already being \
                         dilated"
                            .to_string(),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                if self.start_dilated_eternity() {
                    self.log_event("Dilation entered".to_string(), line);
                    CommandStatus::NextTickNextInstruction
                } else {
                    CommandStatus::NextTickSameInstruction
                }
            }
            Instruction::StartEc { ec } => {
                let ec = *ec;
                if self.ec_running(ec) {
                    self.log_event(
                        "Start EC encountered but ignored due to already being in the \
                         specified EC"
                            .to_string(),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                if self.eternity_challenge_unlocked != ec && !self.buy_ec_study(ec) {
                    return CommandStatus::NextTickSameInstruction;
                }
                if self.start_eternity_challenge(ec) {
                    self.log_event(format!("Eternity Challenge {ec} started"), line);
                    CommandStatus::NextTickNextInstruction
                } else {
                    CommandStatus::NextTickSameInstruction
                }
            }
            Instruction::StudiesBuy {
                studies,
                ec,
                start_ec,
                nowait,
                ..
            } => self.run_studies_buy(line, studies, *ec, *start_ec, *nowait),
            Instruction::StudiesLoad {
                slot,
                nowait,
                display,
            } => self.run_studies_load(line, *slot, *nowait, display),
            Instruction::StudiesRespec => {
                self.respec = true;
                self.log_event("Turned study respec ON".to_string(), line);
                CommandStatus::NextInstruction
            }
            Instruction::UnlockDilation { nowait } => {
                if self.dilation_unlocked() {
                    self.log_event(
                        "Skipped dilation unlock due to being already unlocked"
                            .to_string(),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                if self.buy_dilation_study(1) {
                    self.log_event("Unlocked Dilation".to_string(), line);
                    return CommandStatus::NextInstruction;
                }
                if *nowait {
                    self.log_event(
                        "Skipped dilation unlock due to lack of requirements (NOWAIT)"
                            .to_string(),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                CommandStatus::NextTickSameInstruction
            }
            Instruction::UnlockEc { ec, nowait } => {
                let ec = *ec;
                if self.eternity_challenge_unlocked == ec {
                    self.log_event(
                        format!("Skipped EC {ec} unlock due to being already unlocked"),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                // Note the original checks NOWAIT *before* attempting the
                // purchase, so `unlock nowait ecN` never buys — faithful.
                if *nowait {
                    self.log_event(
                        format!("EC {ec} unlock failed and skipped (NOWAIT)"),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                if self.buy_ec_study(ec) {
                    self.log_event(format!("EC {ec} unlocked"), line);
                    return CommandStatus::NextInstruction;
                }
                CommandStatus::NextTickSameInstruction
            }
            Instruction::If {
                cmp,
                block,
                end_line,
            } => {
                // A non-null command state means the block already ran.
                let entry = self.automator.state.stack.last().expect("running");
                if entry.command_state.is_some() {
                    return CommandStatus::NextInstruction;
                }
                let state = CommandStateData::IfEntered {
                    advance_on_pop: true,
                    if_end_line: *end_line,
                };
                self.automator
                    .state
                    .stack
                    .last_mut()
                    .expect("running")
                    .command_state = Some(state);
                if !cmp.evaluate(self) {
                    self.log_event(
                        format!(
                            "Checked {} (false), skipping to line {}",
                            cmp.display(),
                            end_line + 1
                        ),
                        line,
                    );
                    return CommandStatus::NextInstruction;
                }
                self.log_event(
                    format!("Checked {} (true), entering IF block", cmp.display()),
                    line,
                );
                self.automator_push_block(block);
                CommandStatus::SameInstruction
            }
            Instruction::While {
                cmp,
                block,
                end_line,
            } => self.run_condition_loop(line, cmp, block, *end_line, false),
            Instruction::Until {
                cond,
                block,
                end_line,
            } => match cond {
                UntilCondition::Comparison(cmp) => {
                    self.run_condition_loop(line, cmp, block, *end_line, true)
                }
                UntilCondition::Prestige(layer) => {
                    let layer = *layer;
                    let entry = self.automator.state.stack.last_mut().expect("running");
                    if entry.command_state.is_none() {
                        entry.command_state =
                            Some(CommandStateData::PrestigeLevel { level: 0 });
                    }
                    let seen = match &entry.command_state {
                        Some(CommandStateData::PrestigeLevel { level }) => *level,
                        _ => 0,
                    };
                    let name = capitalized(layer.name());
                    if seen >= prestige_level(layer) {
                        self.log_event(
                            format!("{name} prestige has occurred, exiting until loop"),
                            line,
                        );
                        return CommandStatus::NextInstruction;
                    }
                    let first_line = block.first().map(|c| c.line).unwrap_or(line + 1);
                    self.log_event(
                        format!(
                            "{name} prestige has not occurred yet, moving to line \
                             {first_line} (start of until loop)"
                        ),
                        line,
                    );
                    self.automator_push_block(block);
                    CommandStatus::SameInstruction
                }
            },
            Instruction::Wait { cond } => self.run_wait(line, cond),
        }
    }

    /// `while` / `until <comparison>` (`compileConditionLoop`). For `until`
    /// the comparison is inverted; the logged boolean is the literal result.
    fn run_condition_loop(
        &mut self,
        line: u32,
        cmp: &super::program::Comparison,
        block: &[CompiledCommand],
        end_line: u32,
        is_until: bool,
    ) -> CommandStatus {
        let result = cmp.evaluate(self);
        let keep_looping = if is_until { !result } else { result };
        let loop_str = if is_until { "UNTIL" } else { "WHILE" };
        if !keep_looping {
            self.log_event(
                format!(
                    "Checked {} ({result}), exiting loop at line {end_line} (end of \
                     {loop_str} loop)",
                    cmp.display()
                ),
                line,
            );
            return CommandStatus::NextTickNextInstruction;
        }
        self.log_event(
            format!(
                "Checked {} ({result}), moving to line {line} (start of {loop_str} \
                 loop)",
                cmp.display()
            ),
            line,
        );
        self.automator_push_block(block);
        CommandStatus::SameInstruction
    }

    fn run_auto(
        &mut self,
        line: u32,
        layer: PrestigeLayer,
        setting: &AutoSetting,
    ) -> CommandStatus {
        use crate::autobuyers::{AutoRealityMode, PrestigeAutobuyerMode};
        let on = !matches!(setting, AutoSetting::Off);
        let mut setting_str = String::new();
        match layer {
            PrestigeLayer::Infinity | PrestigeLayer::Eternity => {
                match setting {
                    AutoSetting::DurationMs(ms) => {
                        let secs = ms / 1000.0;
                        let s = if secs > 1000.0 {
                            format!("{secs:.0}")
                        } else {
                            format!("{secs} seconds")
                        };
                        setting_str = s.clone();
                        let settings = if layer == PrestigeLayer::Infinity {
                            &mut self.autobuyers.big_crunch_settings
                        } else {
                            &mut self.autobuyers.eternity.settings
                        };
                        settings.mode = PrestigeAutobuyerMode::Time;
                        settings.time = secs;
                    }
                    AutoSetting::XHighest(x) => {
                        setting_str = format!("{} times highest", fmt(x, 2, 2));
                        let settings = if layer == PrestigeLayer::Infinity {
                            &mut self.autobuyers.big_crunch_settings
                        } else {
                            &mut self.autobuyers.eternity.settings
                        };
                        settings.mode = PrestigeAutobuyerMode::XHighest;
                        settings.x_highest = *x;
                    }
                    AutoSetting::Amount(a) => {
                        setting_str = format!("{} {}", a, layer.currency_name());
                        let settings = if layer == PrestigeLayer::Infinity {
                            &mut self.autobuyers.big_crunch_settings
                        } else {
                            &mut self.autobuyers.eternity.settings
                        };
                        settings.mode = PrestigeAutobuyerMode::Amount;
                        settings.amount = *a;
                    }
                    AutoSetting::On | AutoSetting::Off => {}
                }
                if layer == PrestigeLayer::Infinity {
                    self.autobuyers.big_crunch.is_active = on;
                } else {
                    self.autobuyers.eternity.is_active = on;
                }
            }
            PrestigeLayer::Reality => {
                if let AutoSetting::Amount(a) = setting {
                    self.autobuyers.reality.mode = AutoRealityMode::Rm;
                    self.autobuyers.reality.rm = *a;
                    setting_str = format!("{} RM", fmt(a, 2, 0));
                }
                self.autobuyers.reality.is_active = on;
            }
        }
        let settings_suffix = if on && !setting_str.is_empty() {
            format!(" (Setting: {setting_str})")
        } else {
            String::new()
        };
        self.log_event(
            format!(
                "Automatic {} turned {}{settings_suffix}",
                layer.name(),
                if on { "ON" } else { "OFF" }
            ),
            line,
        );
        CommandStatus::NextInstruction
    }

    fn run_pause(&mut self, line: u32, ms: f64, text: &PauseText) -> CommandStatus {
        let time_string = match text {
            PauseText::Literal(s) => s.clone(),
            PauseText::ConstantMs(ms) => fmt_duration_short(*ms),
        };
        // First execution initializes the timer; later ones advance it by the
        // real tick duration (at least one command interval).
        let advance = self
            .automator
            .runtime
            .tick_dt_ms
            .max(self.automator_current_interval());
        let entry = self.automator.state.stack.last_mut().expect("running");
        let elapsed = match &mut entry.command_state {
            Some(CommandStateData::Pause { time_ms }) => {
                *time_ms += advance;
                *time_ms
            }
            _ => {
                entry.command_state = Some(CommandStateData::Pause { time_ms: 0.0 });
                self.log_event(format!("Pause started (waiting {time_string})"), line);
                0.0
            }
        };
        if elapsed >= ms {
            self.log_event(format!("Pause finished (waited {time_string})"), line);
            CommandStatus::NextInstruction
        } else {
            CommandStatus::NextTickSameInstruction
        }
    }

    fn run_prestige(
        &mut self,
        line: u32,
        layer: PrestigeLayer,
        nowait: bool,
        respec: bool,
    ) -> CommandStatus {
        let available = match layer {
            PrestigeLayer::Infinity => self.can_big_crunch(),
            PrestigeLayer::Eternity => self.can_eternity(),
            PrestigeLayer::Reality => self.is_reality_available(),
        };
        if !available {
            if !nowait {
                return CommandStatus::NextTickSameInstruction;
            }
            self.log_event(
                format!("{} attempted, but skipped due to NOWAIT", layer.name()),
                line,
            );
            return CommandStatus::NextInstruction;
        }
        if respec {
            match layer {
                PrestigeLayer::Eternity => self.respec = true,
                PrestigeLayer::Reality => self.reality.respec = true,
                PrestigeLayer::Infinity => {}
            }
        }
        match layer {
            PrestigeLayer::Infinity => {
                self.big_crunch();
            }
            PrestigeLayer::Eternity => {
                self.eternity();
            }
            PrestigeLayer::Reality => {
                self.auto_reality();
            }
        }
        let name = layer.name().to_ascii_uppercase();
        let record = self.last_prestige_record(layer);
        self.log_event(format!("{name} triggered ({record})"), line);
        // A Reality with force-restart already restarted the run inside the
        // reset; report RESTART so the stepper stops advancing lines.
        if layer == PrestigeLayer::Reality && self.automator.state.force_restart {
            CommandStatus::Restart
        } else {
            CommandStatus::NextTickNextInstruction
        }
    }

    fn run_studies_buy(
        &mut self,
        line: u32,
        studies: &[u16],
        ec: u8,
        start_ec: bool,
        nowait: bool,
    ) -> CommandStatus {
        if nowait {
            for &id in studies {
                self.buy_time_study(id);
            }
            if ec == 0 || self.eternity_challenge_unlocked == ec {
                return CommandStatus::NextInstruction;
            }
            self.buy_ec_study(ec);
            return CommandStatus::NextInstruction;
        }

        let mut pre_purchased = 0usize;
        let mut purchased = 0usize;
        let mut final_purchased_ts: Option<u16> = None;
        for &id in studies {
            if self.time_study_bought(id) {
                pre_purchased += 1;
            } else if self.buy_time_study(id) {
                purchased += 1;
            } else if final_purchased_ts.is_none() {
                final_purchased_ts = Some(id);
            }
        }
        if pre_purchased + purchased < studies.len() {
            if pre_purchased + purchased == 0 {
                self.log_event(
                    "Could not purchase any of the specified Time Studies".to_string(),
                    line,
                );
            }
            if purchased > 0 {
                if let Some(stopped_at) = final_purchased_ts {
                    self.log_event(
                        format!(
                            "Purchased {} and stopped at Time Study {stopped_at}, \
                             waiting to attempt to purchase more Time Studies",
                            quantify_int("Time Study", purchased)
                        ),
                        line,
                    );
                }
            }
            return CommandStatus::NextTickSameInstruction;
        }
        let has_ec = ec != 0 && self.eternity_challenge_unlocked == ec;
        if ec == 0 || (has_ec && !start_ec) {
            self.log_event("Purchased all specified Time Studies".to_string(), line);
            return CommandStatus::NextInstruction;
        }
        let unlocked_ec = self.buy_ec_study(ec);
        if has_ec || unlocked_ec {
            if start_ec {
                self.start_eternity_challenge(ec);
                if self.ec_running(ec) {
                    self.log_event(
                        format!(
                            "Purchased all specified Time Studies, then unlocked and \
                             started running Eternity Challenge {ec}"
                        ),
                        line,
                    );
                } else {
                    self.log_event(
                        format!(
                            "Purchased all specified Time Studies and unlocked \
                             Eternity Challenge {ec}, but failed to start it"
                        ),
                        line,
                    );
                }
            } else {
                self.log_event(
                    format!(
                        "Purchased all specified Time Studies and unlocked Eternity \
                         Challenge {ec}"
                    ),
                    line,
                );
            }
            return CommandStatus::NextInstruction;
        }
        CommandStatus::NextTickSameInstruction
    }

    fn run_studies_load(
        &mut self,
        line: u32,
        slot: usize,
        nowait: bool,
        display: &str,
    ) -> CommandStatus {
        // The preset's *current* content is read at runtime, like the
        // original's `new TimeStudyTree(player.timestudy.presets[i].studies)`.
        let preset = self
            .study_presets
            .get(slot)
            .map(|p| p.studies.clone())
            .unwrap_or_default();
        let parsed = crate::time_studies::parse_study_import(&preset);
        let before = self.studies.len();
        self.commit_study_import(&parsed);
        let after = self.studies.len();
        let missing = parsed
            .studies
            .iter()
            .filter(|&&id| !self.time_study_bought(id))
            .count();

        if missing == 0 {
            self.log_event(format!("Fully loaded study preset {display}"), line);
        } else if after > before {
            self.log_event(
                format!(
                    "Partially loaded study preset {display} (missing {})",
                    quantify_int("study", missing)
                ),
                line,
            );
        }
        if nowait || missing == 0 {
            CommandStatus::NextInstruction
        } else {
            CommandStatus::NextTickSameInstruction
        }
    }

    fn run_wait(&mut self, line: u32, cond: &WaitCondition) -> CommandStatus {
        let now = self.records.real_time_played_ms;
        match cond {
            WaitCondition::Comparison(cmp) => {
                if cmp.evaluate(self) {
                    if self.automator.runtime.is_waiting {
                        let waited = fmt_duration_short(
                            now - self.automator.runtime.wait_start_ms,
                        );
                        self.log_event(
                            format!(
                                "Continuing after WAIT ({} is true, after {waited})",
                                cmp.display()
                            ),
                            line,
                        );
                    } else {
                        self.log_event(
                            format!("WAIT skipped ({} is already true)", cmp.display()),
                            line,
                        );
                    }
                    self.automator.runtime.is_waiting = false;
                    return CommandStatus::NextInstruction;
                }
                if !self.automator.runtime.is_waiting {
                    self.log_event(format!("Started WAIT for {}", cmp.display()), line);
                    self.automator.runtime.wait_start_ms = now;
                }
                self.automator.runtime.is_waiting = true;
                CommandStatus::NextTickSameInstruction
            }
            WaitCondition::Prestige(layer) => {
                let layer = *layer;
                let entry = self.automator.state.stack.last_mut().expect("running");
                if entry.command_state.is_none() {
                    entry.command_state =
                        Some(CommandStateData::PrestigeLevel { level: 0 });
                }
                let seen = match &entry.command_state {
                    Some(CommandStateData::PrestigeLevel { level }) => *level,
                    _ => 0,
                };
                let name = layer.name().to_ascii_uppercase();
                if seen >= prestige_level(layer) {
                    let waited =
                        fmt_duration_short(now - self.automator.runtime.wait_start_ms);
                    let record = self.last_prestige_record(layer);
                    self.log_event(
                        format!(
                            "Continuing after WAIT ({name} occurred for {record}, \
                             after {waited})"
                        ),
                        line,
                    );
                    self.automator.runtime.is_waiting = false;
                    return CommandStatus::NextInstruction;
                }
                if !self.automator.runtime.is_waiting {
                    self.log_event(format!("Started WAIT for {name}"), line);
                    self.automator.runtime.wait_start_ms = now;
                }
                self.automator.runtime.is_waiting = true;
                CommandStatus::NextTickSameInstruction
            }
            WaitCondition::BlackHole { off, hole } => {
                let condition = if *off {
                    !self.black_hole_is_active(0)
                } else {
                    self.black_hole_is_active((*hole as usize).saturating_sub(1))
                };
                let bh_str = if *off {
                    "inactive Black Holes".to_string()
                } else {
                    format!("active Black Hole {hole}")
                };
                if condition {
                    let waited =
                        fmt_duration_short(now - self.automator.runtime.wait_start_ms);
                    self.log_event(
                        format!("Continuing after WAIT (waited {waited} for {bh_str})"),
                        line,
                    );
                    self.automator.runtime.is_waiting = false;
                    return CommandStatus::NextInstruction;
                }
                if !self.automator.runtime.is_waiting {
                    self.log_event(format!("Started WAIT for {bh_str}"), line);
                    self.automator.runtime.wait_start_ms = now;
                }
                self.automator.runtime.is_waiting = true;
                CommandStatus::NextTickSameInstruction
            }
        }
    }
}

/// "Infinity" from "infinity".
fn capitalized(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `quantifyInt("Time Study", 3)` → "3 Time Studies".
fn quantify_int(name: &str, count: usize) -> String {
    if count == 1 {
        format!("{count} {name}")
    } else if let Some(stem) = name.strip_suffix('y') {
        format!("{count} {stem}ies")
    } else {
        format!("{count} {name}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automator::AutomatorMode;

    /// A game with the Automator force-unlocked and `script` installed and
    /// started (repeat off unless a test re-enables it).
    fn game_running(script: &str) -> GameState {
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        game.automator.state.repeat = false;
        assert!(game.automator_save_script(1, script));
        assert!(game.automator_start(Some(1)), "script failed to start");
        game
    }

    /// Run one engine-side automator update of `n` command intervals.
    fn run_commands(game: &mut GameState, n: u32) {
        let interval = game.automator_current_interval();
        game.automator_update(interval * n as f64);
    }

    fn log_messages(game: &GameState) -> Vec<String> {
        game.automator
            .runtime
            .events
            .iter()
            .map(|e| e.message.clone())
            .collect()
    }

    #[test]
    fn start_requires_unlock_and_valid_script() {
        let mut game = GameState::new();
        game.automator_save_script(1, "stop");
        assert!(!game.automator_start(Some(1))); // locked
        game.reality.automator_force_unlock = true;
        assert!(game.automator_start(Some(1)));
        assert!(game.automator_is_running());

        // A script with errors doesn't start.
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        game.automator.state.stack.clear();
        game.automator_save_script(1, "florble");
        assert!(!game.automator_start(Some(1)));
        assert!(!game.automator_is_on());
    }

    #[test]
    fn runs_at_one_command_per_interval() {
        let mut game = game_running("notify \"a\"\nnotify \"b\"\nnotify \"c\"\nstop");
        // Half an interval: nothing runs yet.
        game.automator_update(250.0);
        assert!(game.automator.runtime.pending_notifications.is_empty());
        // One interval: exactly one command.
        game.automator_update(250.0);
        assert_eq!(game.automator.runtime.pending_notifications.len(), 1);
        assert_eq!(
            game.automator.runtime.pending_notifications[0],
            "Automator: \"a\""
        );
        // Two more intervals: two more commands.
        game.automator_update(1000.0);
        assert_eq!(game.automator.runtime.pending_notifications.len(), 3);
        // The stop command halts and clears the stack.
        run_commands(&mut game, 1);
        assert!(!game.automator_is_on());
        assert_eq!(game.automator.state.mode, AutomatorMode::Pause);
        assert!(log_messages(&game)
            .contains(&"Automator execution stopped with STOP command".to_string()));
    }

    #[test]
    fn realities_speed_up_the_interval() {
        let mut game = GameState::new();
        assert_eq!(game.automator_current_interval(), 500.0);
        game.reality.realities = 200;
        let interval = game.automator_current_interval();
        assert!((interval - 500.0 * 0.994f64.powi(200)).abs() < 1e-9);
        game.reality.realities = 2_000_000;
        assert_eq!(game.automator_current_interval(), 1.0);
    }

    #[test]
    fn pause_waits_for_its_duration() {
        let mut game = game_running("pause 10s\nnotify \"done\"");
        // First run starts the timer.
        run_commands(&mut game, 1);
        assert_eq!(game.automator_current_line(), Some(1));
        assert!(log_messages(&game).contains(&"Pause started (waiting 10 s)".into()));
        // 5 s: still paused.
        game.automator_update(5000.0);
        assert_eq!(game.automator_current_line(), Some(1));
        assert!(game.automator.runtime.pending_notifications.is_empty());
        // Another 6 s: the pause finishes and the notify runs.
        game.automator_update(6000.0);
        game.automator_update(500.0);
        assert!(!game.automator.runtime.pending_notifications.is_empty());
        assert!(log_messages(&game).contains(&"Pause finished (waited 10 s)".into()));
    }

    #[test]
    fn if_block_enters_and_skips() {
        let mut game = game_running(
            "if am > 5 {\nnotify \"in\"\n}\nif am > 1e10 {\nnotify \"no\"\n}\nstop",
        );
        for _ in 0..10 {
            run_commands(&mut game, 1);
        }
        let notes = &game.automator.runtime.pending_notifications;
        assert_eq!(notes, &vec!["Automator: \"in\"".to_string()]);
        let logs = log_messages(&game);
        assert!(logs
            .iter()
            .any(|m| m.contains("Checked am > 5 (true), entering IF block")));
        assert!(logs.iter().any(|m| m == "Exiting IF block"));
        assert!(logs
            .iter()
            .any(|m| m.contains("Checked am > 1e10 (false), skipping to line")));
        assert!(!game.automator_is_on());
    }

    #[test]
    fn while_loop_iterates_until_condition_flips() {
        let mut game = game_running("while am < 1e5 {\nnotify \"x\"\n}\nnotify \"out\"");
        // A few passes: the loop keeps producing notifications.
        for _ in 0..6 {
            run_commands(&mut game, 1);
        }
        let in_loop = game.automator.runtime.pending_notifications.len();
        assert!(in_loop >= 2, "loop should have iterated, got {in_loop}");
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .iter()
            .all(|n| n == "Automator: \"x\""));

        // Flip the condition: the loop exits and the tail runs.
        game.antimatter = Decimal::from_float(1e6);
        for _ in 0..4 {
            run_commands(&mut game, 1);
        }
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"out\"".to_string()));
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.contains("exiting loop at line 3 (end of WHILE loop)")));
    }

    #[test]
    fn wait_blocks_until_comparison_holds() {
        let mut game = game_running("wait am >= 1e4\nnotify \"past\"");
        run_commands(&mut game, 1);
        assert!(log_messages(&game).contains(&"Started WAIT for am >= 1e4".into()));
        run_commands(&mut game, 5);
        assert_eq!(game.automator_current_line(), Some(1));

        game.antimatter = Decimal::from_float(2e4);
        game.records.real_time_played_ms += 3000.0;
        run_commands(&mut game, 2);
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"past\"".to_string()));
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.contains("Continuing after WAIT (am >= 1e4 is true, after")));
    }

    #[test]
    fn wait_event_continues_after_prestige() {
        let mut game = game_running("wait eternity\nnotify \"done\"");
        run_commands(&mut game, 1);
        assert!(log_messages(&game).contains(&"Started WAIT for ETERNITY".into()));
        run_commands(&mut game, 1);
        assert_eq!(game.automator_current_line(), Some(1));

        // An eternity elsewhere (manual or autobuyer) notifies the waiting
        // command through the prestige hook.
        game.eternity_unlocked = true;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());
        run_commands(&mut game, 2);
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"done\"".to_string()));
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.contains("Continuing after WAIT (ETERNITY occurred for")));
    }

    #[test]
    fn prestige_command_waits_then_fires() {
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        game.automator.state.repeat = false;
        // Eternity autobuyer milestone so the command validates.
        game.eternities = Decimal::from_float(100.0);
        game.automator_save_script(1, "eternity\nnotify \"after\"");
        assert!(game.automator_start(Some(1)));

        // Not at the goal: the command waits.
        run_commands(&mut game, 3);
        assert_eq!(game.automator_current_line(), Some(1));
        let before = game.eternities;

        // At the goal it eternities and moves on.
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        run_commands(&mut game, 1);
        assert!(game.eternities > before);
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.starts_with("ETERNITY triggered (")));
        run_commands(&mut game, 2);
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"after\"".to_string()));
    }

    #[test]
    fn prestige_nowait_skips_when_unavailable() {
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        game.automator.state.repeat = false;
        game.eternities = Decimal::from_float(100.0);
        game.automator_save_script(1, "eternity nowait\nnotify \"skipped\"");
        assert!(game.automator_start(Some(1)));
        run_commands(&mut game, 3);
        assert!(log_messages(&game)
            .contains(&"eternity attempted, but skipped due to NOWAIT".into()));
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"skipped\"".to_string()));
    }

    #[test]
    fn studies_purchase_waits_for_affordability() {
        let mut game = game_running("studies purchase 11,21\nnotify \"done\"");
        game.time_theorems = Decimal::from_float(1.0); // enough for 11 only
        run_commands(&mut game, 1);
        assert!(game.time_study_bought(11));
        assert!(!game.time_study_bought(21));
        assert_eq!(game.automator_current_line(), Some(1));
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.contains("Purchased 1 Time Study and stopped at Time Study 21")));

        game.time_theorems = Decimal::from_float(10.0);
        run_commands(&mut game, 1);
        assert!(game.time_study_bought(21));
        assert!(
            log_messages(&game).contains(&"Purchased all specified Time Studies".into())
        );
    }

    #[test]
    fn studies_load_and_respec_commands() {
        let mut game = game_running("studies respec\nstudies nowait load id 1\nstop");
        game.study_presets[0].studies = "11,21".into();
        game.time_theorems = Decimal::from_float(10.0);
        for _ in 0..5 {
            run_commands(&mut game, 1);
        }
        assert!(game.respec);
        assert_eq!(game.studies, vec![11, 21]);
        let logs = log_messages(&game);
        assert!(logs.contains(&"Turned study respec ON".into()));
        assert!(logs.contains(&"Fully loaded study preset id 1".into()));
    }

    #[test]
    fn auto_command_configures_autobuyers() {
        let mut game = GameState::new();
        game.reality.automator_force_unlock = true;
        game.automator.state.repeat = false;
        game.complete_challenge(12);
        game.eternities = Decimal::from_float(200.0);
        game.reality.upgrade_bits |= (1 << 13) | (1 << 25);
        game.autobuyers.reality.is_active = true;
        game.automator_save_script(
            1,
            "auto infinity 30s\nauto eternity 0 ep\nauto reality off\nstop",
        );
        assert!(game.automator_start(Some(1)));

        for _ in 0..5 {
            run_commands(&mut game, 1);
        }
        use crate::autobuyers::PrestigeAutobuyerMode;
        assert_eq!(
            game.autobuyers.big_crunch_settings.mode,
            PrestigeAutobuyerMode::Time
        );
        assert_eq!(game.autobuyers.big_crunch_settings.time, 30.0);
        assert!(game.autobuyers.big_crunch.is_active);
        assert_eq!(
            game.autobuyers.eternity.settings.mode,
            PrestigeAutobuyerMode::Amount
        );
        assert_eq!(game.autobuyers.eternity.settings.amount, Decimal::ZERO);
        assert!(game.autobuyers.eternity.is_active);
        assert!(!game.autobuyers.reality.is_active);
        let logs = log_messages(&game);
        assert!(
            logs.contains(&"Automatic infinity turned ON (Setting: 30 seconds)".into())
        );
        assert!(logs.contains(&"Automatic eternity turned ON (Setting: 0 EP)".into()));
        assert!(logs.contains(&"Automatic reality turned OFF".into()));
    }

    #[test]
    fn repeat_restarts_the_script() {
        let mut game = game_running("notify \"pass\"");
        game.automator.state.repeat = true;
        game.options.automator_events.clear_on_restart = false;
        // Each pass ends the update (the restart resets the exec timer, like
        // the original's `start`), so drive several updates.
        for _ in 0..5 {
            run_commands(&mut game, 1);
        }
        assert!(game.automator_is_running());
        assert!(game.automator.runtime.pending_notifications.len() >= 2);
    }

    #[test]
    fn completion_without_repeat_stops() {
        let mut game = game_running("notify \"only\"");
        run_commands(&mut game, 3);
        assert!(!game.automator_is_on());
        assert_eq!(game.automator.runtime.pending_notifications.len(), 1);
    }

    #[test]
    fn noop_only_script_with_repeat_trips_the_guard() {
        let mut game = game_running("# just a comment");
        game.automator.state.repeat = true;
        game.options.automator_events.clear_on_restart = false;
        run_commands(&mut game, 1);
        assert!(!game.automator_is_on());
        assert!(game.automator.runtime.pending_notifications.contains(
            &"Automator halted - too many consecutive no-ops detected".into()
        ));

        // Without repeat the same script completes silently.
        let mut game = game_running("# just a comment");
        run_commands(&mut game, 1);
        assert!(!game.automator_is_on());
        assert!(game.automator.runtime.pending_notifications.is_empty());
    }

    #[test]
    fn single_step_runs_one_command() {
        let mut game = game_running("notify \"a\"\nnotify \"b\"");
        game.automator_pause();
        assert!(game.automator_step_once(None));
        run_commands(&mut game, 3); // only one command runs regardless
        assert_eq!(game.automator.runtime.pending_notifications.len(), 1);
        assert_eq!(game.automator.state.mode, AutomatorMode::Pause);
        assert_eq!(game.automator_current_line(), Some(2));
    }

    #[test]
    fn resume_from_save_matches_lines() {
        let mut game = game_running("notify \"a\"\npause 100s\nnotify \"b\"");
        run_commands(&mut game, 2); // notify, then the pause starts
        assert_eq!(game.automator_current_line(), Some(2));

        // Round-trip through the save codec: the stack (line 2 + pause timer)
        // persists; the transient program does not.
        let saved = crate::save::encode_save(&game, 1_700_000_000_000);
        let mut reloaded = crate::save::decode_save(&saved).unwrap();
        assert_eq!(reloaded.automator_current_line(), Some(2));
        assert!(!reloaded.automator.runtime.initialized);

        // The first update recompiles and resumes in place.
        reloaded.automator_update(500.0);
        assert_eq!(reloaded.automator_current_line(), Some(2));
        assert!(reloaded.automator_is_running());
        assert!(reloaded.automator.runtime.program.is_some());
    }

    #[test]
    fn resume_restarts_when_script_was_edited() {
        let mut game = game_running("notify \"a\"\npause 100s\nnotify \"b\"");
        run_commands(&mut game, 2);
        assert_eq!(game.automator_current_line(), Some(2));

        let saved = crate::save::encode_save(&game, 1_700_000_000_000);
        let mut reloaded = crate::save::decode_save(&saved).unwrap();
        // Edit the script behind the runner's back (bypassing the stop that
        // automator_save_script performs). The new script has no line 2 at
        // all, so the saved stack can't be matched.
        reloaded.automator.scripts.get_mut(&1).unwrap().content = "stop".to_string();
        reloaded.automator_update(0.0);
        // The run restarted from the top (`initializeFromSave` mismatch).
        assert_eq!(reloaded.automator_current_line(), Some(1));
    }

    #[test]
    fn reality_with_force_restart_restarts_script() {
        let mut game = crate::reality::tests::game_at_reality_goal();
        game.reality.automator_force_unlock = true;
        game.automator.state.repeat = false;
        game.automator.state.force_restart = true;
        game.automator_save_script(1, "pause 100s\nnotify \"tail\"");
        assert!(game.automator_start(Some(1)));
        run_commands(&mut game, 1);
        assert_eq!(game.automator_current_line(), Some(1));
        let pause_state = game.automator.state.stack[0].command_state.clone();
        assert!(pause_state.is_some());

        // A (manual) Reality restarts the running script from the top.
        assert!(game.reality_with_glyph_choice(None, false));
        assert!(game.automator_is_running());
        assert_eq!(game.automator_current_line(), Some(1));
        assert!(game.automator.state.stack[0].command_state.is_none());
    }

    #[test]
    fn until_event_loops_until_prestige_at_header() {
        let mut game = game_running("until eternity {\nnotify \"pass\"\n}\nstop");
        game.options.automator_events.clear_on_restart = false;
        run_commands(&mut game, 4);
        let passes = game.automator.runtime.pending_notifications.len();
        assert!(passes >= 1);
        assert!(game.automator_is_on());

        // Prestige while the loop header is the top frame (its command state
        // holds the seen-prestige level): the loop exits.
        game.eternity_unlocked = true;
        game.records.this_eternity.max_ip = crate::ETERNITY_GOAL;
        assert!(game.eternity());
        for _ in 0..6 {
            run_commands(&mut game, 1);
        }
        assert!(!game.automator_is_on());
        assert!(log_messages(&game)
            .iter()
            .any(|m| m.contains("Eternity prestige has occurred, exiting until loop")));
    }

    #[test]
    fn black_hole_command_toggles_pause() {
        let mut game = game_running("black hole off\nstop");
        game.black_holes.holes[0].unlocked = true;
        assert!(game.automator_start(Some(1)));
        assert!(!game.black_holes.paused);
        run_commands(&mut game, 2);
        assert!(game.black_holes.paused);
        assert!(log_messages(&game).contains(&"Black Holes toggled OFF".into()));
    }

    #[test]
    fn runs_inside_the_game_tick() {
        // End to end through the real game loop: the Automator executes on
        // real time from `tick()`.
        let mut game = game_running("notify \"ticked\"\nstop");
        for _ in 0..30 {
            game.tick(50.0); // 1.5 s real time, 3 command intervals
        }
        assert!(game
            .automator
            .runtime
            .pending_notifications
            .contains(&"Automator: \"ticked\"".to_string()));
        assert!(!game.automator_is_on());
    }

    #[test]
    fn event_log_caps_at_max_entries() {
        let mut game = game_running("notify \"x\"");
        game.automator.state.repeat = true;
        game.options.automator_events.clear_on_restart = false;
        game.options.automator_events.max_entries = 5;
        for _ in 0..20 {
            run_commands(&mut game, 1);
        }
        assert_eq!(game.automator.runtime.events.len(), 5);
    }
}
