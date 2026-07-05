# Feature 6.6: Automator

**Status: Stages A + B implemented** (see §12–§13); Stages C–E pending. This doc
records the original's mechanics, the frontier cuts, the Rust design, and —
the headline question — how to decompose the feature. **Answer up front: the
Automator should not be ported in one go.** It decomposes cleanly into five
stages (A–E below), each independently testable and committable, and the first
stage is not Automator code at all but deferred engine debt the Automator
depends on.

Original sources: `src/core/automator/*` (lexer, parser, compiler,
automator-backend, automator-commands, automator-points, script-templates,
automator-codemirror), `src/core/secret-formula/reality/automator.js` (docs
data), `src/components/tabs/automator/*.vue` (23 components),
`src/components/modals/*Automator*.vue` (4 modals), `player.js`
(`player.reality.automator`, `Player.automatorUnlocked`), `hotkeys.js`.

Size of the original: ~4,450 lines of core JS + ~3,950 lines of Vue in the
tab + ~640 lines of docs data. This is the largest single feature in Phase 6.

---

## 1. What the Automator is

A scripting language plus an in-game IDE. Scripts automate the
Eternity-and-below grind: buy study trees, set prestige autobuyers, wait for
conditions, run ECs, dilate, prestige, loop. The player edits scripts in a
text editor (CodeMirror) or a drag-and-drop block editor — both edit the same
underlying text — and runs them at a game-state-dependent speed.

## 2. Unlock: Automator Points

The Automator unlocks at **100 AP** (`AutomatorPoints.pointsForAutomator`),
or `player.reality.automator.forceUnlock`. AP sources:

- **Perks** with an `automatorPoints` prop (5/10/15 each; ~16 perks).
- **Reality Upgrades**: RU10 (15), RU11 (5), RU13 (10), RU14 (5), RU20 (10),
  RU25 (100 — buying RU25 alone unlocks the Automator).
- **Other** (`otherAutomatorPoints`): +2 per Reality up to 50 Realities
  (max 100), +10 for unlocking Black Hole 1.

While locked, the Automator tab shows the `AutomatorPointsList` page: total
AP, progress bar, and three panels enumerating every source (perks, upgrades,
other) with bought/unbought styling.

Our port's `perks.rs` / `reality_upgrades.rs` configs do **not** carry AP
values yet — adding them is part of Stage A.

## 3. Scripts, constants, limits

`player.reality.automator`:

```
state: {
  mode,                  // PAUSE=1, RUN=2, SINGLE_STEP=3
  topLevelScript,        // id of the running script
  editorScript,          // id of the script open in the editor
  repeat, forceRestart, followExecution,   // toggles on the controls bar
  stack: [ { lineNumber, commandState } ]  // persisted execution stack
},
scripts: { id: { id, name, content } },    // content is always TEXT
constants: { name: "value" },              // defined via UI panel, not script
constantSortOrder: [ name ],
execTimer,             // ms accumulated toward the next command
type,                  // TEXT=0, BLOCK=1 (current editor mode)
forceUnlock,
currentInfoPane,       // which docs pane is open
```

plus `player.options.automatorEvents` (newestFirst, timestampType,
maxEntries, clearOnReality, clearOnRestart).

Limits (enforced at save time; edits beyond them are not persisted):
20 scripts, 10,000 chars/script, 60,000 chars total, 15-char script names,
30 constants, 20-char constant names, 250-char constant values.

Script deletion leaves id gaps (ids are never re-indexed); new scripts fill
the first gap. If the last script is deleted a fresh "New Script" is created.

## 4. The language

### 4.1 Tokens

Case-insensitive keywords; some are multi-word (`black hole`, `store game
time`, `x highest`, `pending ip`, `total tt`, `banked infinities`, …).
Comments: `#` or `//` to end of line. Numbers: decimal with optional
exponent (`1.23e45`); parsed into `Decimal`. Strings (for `notify`): double
or single quoted. Durations: number + unit (`ms`, `s|sec|seconds`,
`m|min|minutes`, `h|hours`).

**Currencies** usable in comparisons (each has a getter; some have an
`$unlocked` gate): `am, ip, ep, rm, dt, tp, rg, rep(licanti), tt, total tt,
spent tt, infinities, banked infinities, eternities, realities, pending
ip/ep/tp/rm, pending glyph level, total completions, pending completions,
ec1..ec12 completions, filter score, space theorems, total space theorems`.

Frontier note: `filter score` (Effarig), `space theorems` / `total space
theorems` (V) are celestial-gated. Their `$unlocked` returns false at our
frontier, which makes any comparison using them compile to constant-false.
We keep the tokens and that exact behavior.

### 4.2 Commands (the real list)

The feature-decomposition doc's command sketch was approximate; the actual
command set (from `automator-commands.js`) is:

| Command | Notes |
|---|---|
| `auto infinity/eternity/reality <on\|off\|duration\|N x highest\|N ip/ep/rm>` | configures + toggles prestige autobuyers; validation gates on autobuyer unlocks (see §7) |
| `infinity/eternity/reality [nowait] [respec]` | prestige; waits for availability unless `nowait`; `respec` invalid on infinity |
| `studies [nowait] purchase <list\|constant>` | study list: ids, ranges `11-62`, path names (`antimatter, infinity, time, active, passive, idle, light, dark`), `\|ecN` suffix, `!` to auto-start the EC |
| `studies [nowait] load id <1-6>` / `load name <preset>` | loads a Time Study preset |
| `studies respec` | sets the respec flag |
| `unlock [nowait] dilation` / `unlock [nowait] ec<N>` | |
| `start dilation` / `start ec<N>` | |
| `if <cmp> { … }` | one-shot conditional block |
| `while <cmp> { … }` / `until <cmp\|prestige-event> { … }` | loops; `until <event>` runs the block until that prestige (or a higher layer) occurs |
| `wait <cmp>` / `wait <prestige-event>` / `wait black hole <off\|bh1\|bh2>` | |
| `pause <duration\|constant>` | |
| `notify "text"` | toast + event log |
| `black hole on/off` | |
| `store game time on/off/use` | **Enslaved-gated → out of frontier**; validation error kept |
| `stop` | halts execution |
| `blob` | hidden easter-egg no-op (kept — it's 5 lines) |
| comments | no-op instructions |

Comparisons are `value <op> value` with `<, >, <=, >=` only (`==` is
rejected with a specific error). Values: currency, number literal, or
constant name. There is **no** `define` command and no `buy <upgrade>`
command (the decomposition doc guessed wrong); constants are created in the
"define" UI panel.

### 4.3 Compilation pipeline

The original runs (chevrotain-based): lexer → CST parser with error
recovery → **Validator** (per-command `validate` hooks; produces errors
`{startLine, startOffset, endOffset, info, tip}`) → **Compiler** (per-command
`compile` hooks; array of `{run, blockCommands?, lineNumber}`) → optionally
**Blockifier** (CST → block-editor structures).

Validation is **game-state-dependent**: it checks autobuyer/milestone
unlocks (e.g. "Eternity autobuyer is not unlocked"), preset existence by
name, EC ids 1–12, study ids against the study database, constant formats
(number / study string / duration). So the same script can be valid on one
save and invalid on another. Errors are deduplicated to one per line and
carry a human "tip" (fix suggestion); we reproduce the messages verbatim
where practical.

A script is recompiled on every edit (for the error panel) and on `start`.
Only the *top-level running script* needs compiled form retained; the editor
needs errors for the *editor script*.

## 5. Execution semantics

- **Modes**: PAUSE / RUN / SINGLE_STEP. "On" = stack non-empty (a paused
  automator is still on).
- **Speed**: one command per `max(0.994^realities × 500, 1)` ms
  (`AutomatorBackend.currentInterval`). Each game tick adds `diff` to
  `execTimer` and executes `floor(execTimer / interval)` commands, capped at
  **100 per update**.
- **Command statuses** drive the stepper: `NEXT_INSTRUCTION`,
  `NEXT_TICK_SAME_INSTRUCTION` (waits), `NEXT_TICK_NEXT_INSTRUCTION`,
  `SAME_INSTRUCTION` (entering a block pushes the block and re-runs),
  `SKIP_INSTRUCTION` (no-ops; a run of 100 consecutive no-ops halts with an
  error toast), `HALT` (stop), `RESTART`.
- **Stack**: entries hold `{commands, commandIndex}` transient +
  `{lineNumber, commandState}` persisted. `commandState` is per-command
  scratch: `{timeMs}` for `pause`, `{prestigeLevel}` for `wait event` /
  `until event`, `{advanceOnPop, ifEndLine}` for `if`. Popping past the end
  of the top-level script either restarts (repeat toggle) or stops.
- **Prestige events**: on big crunch / eternity / reality reset, the backend
  bumps `commandState.prestigeLevel` on the current stack top (levels:
  infinity 1 < eternity 2 < reality 3), which is what `wait eternity` etc.
  poll. A reality with `forceRestart` on restarts the script from the top
  (also triggered outside the automator by any reality).
- **Event log**: every command execution appends `{message, line,
  thisReality, timestamp, timegap}`; capped at
  `options.automatorEvents.maxEntries`; optionally cleared on
  restart/reality. Not saved. Wait commands log once (an `isWaiting` flag),
  not per attempt.
- **Resume across save/load**: the persisted stack stores line numbers only.
  `initializeFromSave` re-walks the compiled script, matching each stack
  level's `lineNumber` against the block's commands (descending through
  `blockCommands`); any mismatch (script edited since save) resets the run.
  Editing the running script stops the automator.

## 6. UI inventory

Tab layout (`AutomatorTab.vue`): speed line + character counters, then a
44/50 vertical **SplitPane**: left = editor pane, right = docs pane. A
fullscreen toggle expands the split pane. While locked, the AP list page.

**Editor pane** (`AutomatorEditor.vue` + children):
- Controls bar (`AutomatorControls`): rewind/play-pause/step/stop buttons,
  running-script name, repeat / force-restart / follow-execution toggles,
  mode-switch button (text↔block, with confirmation modal when the script
  has errors), script dropdown (rename inline, create, delete with modal).
- **Text editor** (`AutomatorTextEditor`): CodeMirror 5 with a custom simple
  mode (keyword/currency highlighting), autocomplete hints, error gutter
  markers, and line highlights: active (running) line, event line, error
  line. Follow-execution auto-scrolls. Undo/redo buffer (30 entries, min 10
  chars between snapshots) with mod+z / mod+y hotkeys.
- **Block editor** (`AutomatorBlockEditor` + `AutomatorBlockSingleRow` +
  `AutomatorBlockSingleInput`): vuedraggable-based; nested blocks for
  if/while/until; per-block inputs (dropdowns, text fields) with the same
  validation; block↔text conversion via the Blockifier (text→blocks) and
  `BlockAutomator.parseLines` (blocks→text, plain string building). Line
  numbers differ between modes (block mode gives `}` no line), handled by
  `translateLineNumber`.

**Docs pane** (`AutomatorDocs` + pages), pane chosen by
`currentInfoPane` (saved): intro page, command reference (man pages from
`secret-formula/reality/automator.js`: 20 entries, 5 categories), block list
(block mode), script templates, define (constants) panel, AP breakdown,
error page (errors for current script, with tips), event log page (with
timestamp display options), data-transfer page (per-script export/import of
script + referenced presets/constants).

**Modals**: delete-script confirmation, switch-editor-mode confirmation,
script-template parameter form (per-template inputs + warnings), import
automator data (with "ignore presets/constants" checkboxes).

**Templates** (`script-templates.js`): five parameterized generators —
Climb EP, Grind Eternities, Grind Infinities, Complete EC, Unlock Dilation.
Each takes a study tree (preset or import string), autobuyer settings and a
target, emits script text + warnings (e.g. "tree can't reach the EC").
Templates need study-tree analysis (`TimeStudyTree`) and autobuyer settings.

**Import/export**: script text and "full data" (script + referenced presets
+ constants) serialized as length-prefixed concatenated strings
(`serializeAutomatorData`), then base64-encoded via `GameSaveSerializer`
`encodeText` with type prefixes (`automator script` / `automator data`).

**Hotkeys**: `u` start/pause, `shift+u` restart, `mod+z`/`mod+y` undo/redo.

## 7. Engine dependencies missing in the port (the gating discovery)

The Automator's commands configure systems that earlier phases deferred.
From the eternity/reality design docs and `autobuyers.rs` (which today only
models the pre-Infinity autobuyers with interval upgrades):

1. **Time Study presets** — `player.timestudy.presets` (6 × `{name,
   studies}`) and the save/load buttons on the Time Studies tab are not
   ported. Needed by `studies load`, templates, and full-data export.
2. **Big Crunch autobuyer advanced modes** — `AUTO_CRUNCH_MODE`
   AMOUNT/TIME/X_HIGHEST, gated by the `bigCrunchModes` eternity milestone
   (5 eternities). Needed by `auto infinity <setting>`.
3. **Eternity autobuyer** — unlocked by the `autobuyerEternity` milestone
   (100 eternities); AMOUNT/TIME/X_HIGHEST modes gated by RU13. Needed by
   `auto eternity`, and `eternity`-command validation gates on it.
4. **Reality autobuyer** — unlocked by RU25; the automator sets
   `AUTO_REALITY_MODE.RM` only (GLYPH/EITHER/BOTH/TIME modes exist in the
   autobuyer UI). Needed by `auto reality`, and `reality`-command validation
   gates on RU25.
5. **AP values** on perk / reality-upgrade configs + the AP total (§2).

All five are *independently* part of a faithful port (they are original
pre-Automator features we deferred), so they form a prerequisite stage
rather than bloating the Automator stages.

## 8. Frontier decisions

- **`store game time`**: Enslaved (celestial). Keep grammar + the original
  validation error ("You do not yet know how to store game time"); no
  runtime implementation.
- **`filter score`, `space theorems`, `total space theorems`**: tokens kept,
  permanently locked at our frontier → comparisons using them compile to
  constant-false, exactly as the original behaves pre-unlock.
- **Triad studies** (Ra): study-id validation rejects them naturally (not in
  our study database).
- **`blob`**: kept (hidden no-op).
- **Speedrun references, Pelle checks** inside command bodies: dropped.
- **Hotkeys**: port `u` / `shift+u`; undo/redo bindings come with the text
  editor. (We already have a hotkey mechanism? If not: deferred, note in UI
  stage.)
- **Black-hole commands**: in frontier (6.5 is ported): `black hole on/off`,
  `wait black hole …`.

## 9. Rust design

### 9.1 Engine (`ad-core/src/automator/`)

New module with submodules; all state hangs off `GameState` like other
features:

- `tokens.rs` — token kinds, the currency table (name → getter over
  `GameState`, `unlocked` predicate), keyword table with multi-word
  patterns. Hand-written scanner, case-insensitive, span-tracking
  (line/offset) for error reporting.
- `parser.rs` — hand-written line-oriented recursive descent. The grammar is
  one command per line with `{`/`}` blocks, which makes per-line error
  recovery natural (skip to EOL, keep parsing; brace matching for blocks).
  Produces a `Vec<ParsedLine>` AST.
- `validate.rs` — AST → `Vec<AutomatorError { start_line, info, tip }>`,
  including the game-state-dependent checks (§4.3), constant-format checks,
  one-error-per-line dedup. Error strings mirror the original.
- `program.rs` — compiled form:

  ```rust
  enum Instruction {
      Auto { layer: PrestigeLayer, setting: AutoSetting },
      BlackHole { on: bool },
      Notify(String),
      NoOp,                                  // comment / blob
      Pause(DurationSource),                 // literal ms or constant name
      Prestige { layer, nowait: bool, respec: bool },
      StartDilation, StartEc(u8),
      StudiesBuy { list: StudyListSource, nowait: bool },
      StudiesLoad { preset: PresetRef, nowait: bool },
      StudiesRespec,
      UnlockDilation { nowait: bool }, UnlockEc { ec: u8, nowait: bool },
      If { cmp: Comparison, block: Vec<Instruction> },
      While { cmp: Comparison, block: Vec<Instruction> },
      Until { cond: UntilCond, block: Vec<Instruction> },   // cmp or prestige event
      Wait(WaitCond),                        // cmp / prestige event / black hole
      Stop,
  }
  struct Comparison { left: CmpValue, op: CmpOp, right: CmpValue }
  enum CmpValue { Currency(AutomatorCurrency), Const(String), Literal(Decimal) }
  ```

  Constants resolve at *runtime* (as in the original), so editing a constant
  affects a running script. Each instruction carries its source
  `line_number` for the stack, highlighting and the event log.
- `exec.rs` — the stack machine: `AutomatorMode`, `CommandStatus`, frames
  `{ block_path: Vec<usize>, command_index, line_number, command_state }`
  (`block_path` replaces JS object references into nested blocks),
  `CommandState` enum (`PauseTimer{ms}`, `PrestigeLevel(u8)`,
  `IfEntered{end_line}`), `update(diff)` with the interval formula and the
  100-command / 100-no-op caps, prestige-event notification hook called from
  the existing crunch/eternity/reality reset paths, event log ring buffer.
- `data.rs` — script/constant CRUD with the §3 limits, AP totals,
  import/export serialization (length-prefixed format + the existing save
  codec's text encoding), used-preset/used-constant scans for full-data
  export.

`blockify` (compiled-for-UI block structures) is a small extra visitor over
the AST producing serde-serializable block descriptions for the block
editor; block→text stays in the frontend (it is plain string assembly, as in
the original).

**Why hand-written rather than `nom`/`pest`:** the grammar is line-oriented
and tiny (22 commands), but needs (a) case-insensitive multi-word keywords
with longest-match against identifiers, (b) per-line error *recovery* with
original-fidelity messages and tips, (c) game-state-dependent validation
interleaved with parsing results. Parser-generator ergonomics fight all
three; a hand-written scanner + recursive descent is ~the same code size and
keeps error text under exact control, with no new dependency.

### 9.2 Save/load

DTOs mirror §3 exactly (scripts as a string-keyed map with id gaps,
constants + sort order, state incl. stack `{lineNumber, commandState}`,
execTimer, type, forceUnlock, currentInfoPane; options.automatorEvents).
Resume uses the original's line-number re-matching; mismatch resets the
stack. Round-trip tests against original saves with running automators.

### 9.3 UI (`ad-gui`)

Same architecture as the rest of the port: vendored markup/CSS from the
original components, Pinia snapshot for per-frame state, Tauri commands for
actions and for request/response data:

- Snapshot additions (`GameView.automator`): unlocked, AP total + threshold,
  mode, isOn, running script id/name, current line (translated), repeat /
  forceRestart / followExecution, interval, char counts, event-log length.
- IPC queries (not in the per-tick snapshot): `automator_errors(script)`,
  `automator_blockify(script)`, `automator_event_log`, `automator_docs` (or
  ship docs data as static frontend JSON — it is pure text; **decision:
  static frontend data**, like other vendored copy), AP breakdown,
  import/export encode/decode, template generation.
- Actions: script CRUD/rename/save-content, constants CRUD, mode/type
  switches, play/pause/step/stop/restart, toggles, preset save/load (Stage
  A), `forceUnlock` (dev).
- **Text editor**: CodeMirror 5 (npm `codemirror@5`), porting
  `automator-codemirror.js` (mode + hints; keyword lists duplicated as
  static frontend tables) and the line-highlight/gutter logic.
- **Block editor**: `vuedraggable` (Vue-3-compatible `vuedraggable@next`),
  porting the three block components + conversion glue.

## 10. Staging plan (the answer to "one go or stages?")

Five stages, each a coherent commit series with tests. Estimated engine code
is the bulk; UI stages reuse vendored markup as usual.

- **Stage A — prerequisites (no automator code).** Study presets (engine +
  save + Time Studies tab UI), Big Crunch autobuyer modes, Eternity
  autobuyer, Reality autobuyer (modes + gating + autobuyer-tab UI rows), AP
  values on perks/reality upgrades + `AutomatorPoints` totals. Each item is
  deferred debt from Features 2.6/4.x/6.4 and lands with its own tests.
  *Value: standalone; the game is more faithful even if the Automator
  stopped here.*
- **Stage B — language core (engine).** Tokens, parser, validator, compiled
  `Instruction` program, script/constant storage + limits + CRUD, save/load
  of scripts and constants (no execution). Tests: golden scripts → expected
  instruction trees; error fixtures → original messages; save round-trip.
- **Stage C — execution engine.** Stack machine, tick integration, all
  frontier command implementations, prestige-event notifications, event log,
  stack persistence + resume. Tests: multi-tick engine tests driving small
  scripts (waits, loops, prestige commands) against the real engine;
  resume-after-reload tests.
- **Stage D — UI: tab, text editor, docs.** Locked AP page, split-pane tab,
  controls bar, script dropdown + modals, CodeMirror text editor with
  highlights/errors/autocomplete, docs pane (intro, command reference,
  define panel, AP list, error page, event log). Undo/redo + hotkeys.
- **Stage E — UI: block editor, templates, data transfer.** Block editor +
  drag/drop + block↔text conversion + mode switch modal, script templates
  (engine-side generation, modal UI), script/full-data import/export +
  data-transfer page.

Dependencies: A → B → C → D → E is strictly ordered except A∥B (parser needs
presets only for *validation* of `studies load name`; we still do A first so
validation lands complete). D needs C for live state but the editor/docs
parts only need B — if a checkpoint is wanted, D's editor could land after B
with the controls bar stubbed; not planned, just an option.

## 11. Open questions (proceeding with best guesses)

1. **Event-log formatting** uses the player's notation via `format()` in the
   original. Engine-side we format with `ad-format` using current notation
   settings at log time (matches original behavior of baking text at event
   time). *Proceeding with: engine formats messages as strings.*
2. **Docs data location**: static JSON in the frontend (vendored copy of the
   man pages) vs engine. The `isUnlocked` gates on a few entries reference
   game state (e.g. black hole unlocked); those flags come from the
   snapshot. *Proceeding with: static frontend data + snapshot flags.*
3. **CodeMirror 5 vs 6**: original uses 5; its API (`simple mode`, line
   classes, hints) is what the vendored code expects. *Proceeding with 5.*
4. **`translateLineNumber` / block-mode line numbering** lives in the
   frontend in the original (needs block layout). We mirror that: engine
   reports raw text line numbers; frontend translates in block mode.
5. The decomposition doc's 6.6 command list is inaccurate (`define`, `buy`
   don't exist). Corrected here; decomposition doc updated to point at this
   doc.

## 12. Stage A implementation notes (2026-07-05)

Everything in §10 Stage A is implemented (engine + save + UI + tests):

- **Study presets** — `time_studies.rs`: `StudyPreset` (6 slots on
  `GameState.study_presets`), the full import-string machinery
  (`is_valid_study_import` / `parse_study_import` with set names, ranges,
  `|EC` and `!` — mirroring `TimeStudyTree`), `study_tree_export_string`,
  save/load/respec-and-load/rename/edit. UI: `TimeStudyPresetButton.vue` +
  `HoverMenu.vue` (Vue 3 port with local menu state) in the Time Studies tab,
  with edit/delete modals. The preset row sits on its own centered row below
  the TT buy buttons (our simplified header; the original embeds it in the
  ttshop bar).
- **Big Crunch autobuyer modes** — `PrestigeAutobuyerMode` +
  `PrestigeGoalSettings` on `AutobuyerState.big_crunch_settings`;
  `will_auto_crunch()` gates the tick post-break (pre-break/in-challenge it
  always crunches at the goal, as before). Mode reset on Eternity/Reality
  without the milestone; `bump_big_crunch_amount` fires from Achievements
  85/93 (the `ipMult` rebuyable is still an unported Break-Infinity feature,
  so it can't bump yet).
- **Eternity autobuyer** — `EternityAutobuyer` (no interval; checked per
  tick), `will_auto_eternity()` incl. the in-EC behavior
  (`ec_pending_total_completions`, the ECB-perk bulk wait); deactivates on
  reset without the 100-Eternities milestone; `bump_eternity_amount(×5)` from
  `buy_ep_mult`.
- **Reality autobuyer** — `RealityAutobuyer` with RM/Glyph/Either/Both/Time
  modes (`RELIC_SHARD` is Effarig content: a save carrying mode 5 loads as
  RM, the `shard` field is ignored). Fires `auto_reality()`
  (`processAutoGlyph` semantics: `glyph_list(choiceCount)[0]`, sacrifice on
  full inventory).
- **AP** — `automator_points.rs`: perk values on `PerkDef.automator_points`
  (21 perks, sum 150), the six upgrade grants, +2/reality (cap 50), +10 BH;
  `automator_unlocked()` at 100 AP or `reality.automator_force_unlock`
  (round-trips `player.reality.automator.forceUnlock`).
- **Save** — `player.auto.bigCrunch` goal fields, `auto.eternity`,
  `auto.reality`, `timestudy.presets` all round-trip (strict mode parsing,
  like the rest of the DTO layer).
- **UI** — `BigCrunchAutobuyerBox` / `EternityAutobuyerBox` /
  `RealityAutobuyerBox` with `AutobuyerModeDropdown` (simplified
  ExpandingControlBox, per the SelectNotationDropdown precedent) and
  `AutobuyerInput` (scientific-formatted display; the original's
  plain/scientific/log/mixed input grammar, validated locally and parsed
  engine-side in `parse_decimal_input`).

Known deviations, each judged sub-interval or out-of-frontier:

- The original's `resetTick` (autobuyer timer-phase reset on prestige events)
  isn't modeled — our timers are elapsed-time accumulators; the effect is at
  most one interval (100 ms when maxed) of extra latency after a prestige.
- `EC pending completions` ignores `maxValidCompletions` (the EC4/EC12
  restriction cap), consistent with the existing `complete_running_ec`.
- The original suppresses the reality autobuyer while the glyph-choice modal
  is open (`GlyphSelection.active`); our engine has no modal state. The modal
  confirm no-ops afterwards (reality no longer available), so the effect is
  the modal closing on its own.

## 13. Stage B implementation notes (2026-07-05)

The language core lives in `crates/ad-core/src/automator/` (no UI yet —
Stage D):

- **`lexer.rs`** — hand-written line-oriented scanner. Words are scanned as
  maximal identifier chunks and classified case-insensitively, which
  reproduces chevrotain's `longer_alt: Identifier` semantics for free
  ("ecological" ≠ `ec`). Multi-word tokens are matched longest-phrase-first
  ("pending glyph level", "total space theorems", "ec5 completions", "store
  game time", "x highest"). The special originals are kept: `name` swallows
  its argument at lex time, `id` takes one optional digit, `blob` needs two
  trailing spaces, `-` always lexes as Dash (so number literals are
  unsigned, as in the original's token order), a leading `0` ends its
  integer part. Unexpected characters become recoverable lex errors.
- **`parser.rs`** — recursive descent per line; `{` blocks recurse until a
  lone `}` line. Per-line error recovery with the original's message shapes:
  "Unrecognized command \"X\"" for identifier-initial lines, "Unexpected
  input X"/"Remove X" for extra tokens, "Unexpected end of command" for
  truncated ones, `checkBlock`'s messages for brace mismatches.
- **`compile.rs`** — validation + compilation in one pass over the AST;
  instructions are produced only when there are zero errors, and errors are
  deduped one-per-line and sorted (`modifyErrorMessages`). All the
  game-state-dependent checks are ported with original text: autobuyer
  unlock gates for `auto`/`eternity`/`reality` (wired to Stage A's
  autobuyers), EC ids 1–12, study-id existence, preset id/name resolution
  (baked to a slot at compile time, `$presetIndex`), constant format checks
  per var type (number/studies/duration), equality-comparison rejection, and
  the always-failing `store game time` (Enslaved is out of frontier).
  Constant *comparisons* resolve at runtime (live `constants` lookup);
  constant pause durations and study strings bake at compile time — both as
  in the original.
- **`program.rs`** — `CompiledCommand { line, op: Instruction }` with nested
  blocks, `Comparison::evaluate(&GameState)` (locked currencies →
  constant-false) and the full currency table: `am ip ep rm dt tp rg rep tt
  total/spent tt, (banked) infinities, eternities, realities, pending
  ip/ep/tp/rm/glyph level/completions, total completions, ecN completions`,
  plus the frontier-locked `filter score` and `space theorems`. `total tt` /
  `spent tt` needed two new engine helpers (`invested_study_tt`,
  `tree_spent_tt`).
- **`mod.rs`** — storage: `AutomatorData` on `GameState` (scripts keyed by
  id with gap-filling creation, constants + `constantSortOrder`, editor
  type, docs pane, exec timer, and the passive run state incl. the typed
  stack entries). CRUD with the original's limits (20 scripts / 10k / 60k
  chars, 15-char names, 30 constants / 20-char names / 250-char values) and
  reserved-word constant-name validation (`forbiddenConstantPatterns`).
- **Save** — the full `player.reality.automator` subtree round-trips:
  scripts (id-keyed with the duplicated `id` prop), constants (non-string
  scalars stringified), sort order, `type`, `currentInfoPane`, `execTimer`,
  and `state` including the stack (`{lineNumber, commandState}` with the
  three commandState shapes). A fresh save's *missing* `mode` key (the
  original's `AUTOMATOR_MODE.STOP` is `undefined`) loads as paused; an
  unrecognized stack shape clears the stack rather than failing the load
  (the original resets unresumable runs anyway).

Deviations, all judged cosmetic: error text for chevrotain's internal
recovery messages is approximated by the post-`modifyErrorMessages` forms
(the panel-visible strings match for the common cases); errors carry line
numbers only (the original also tracks offsets, but its editor decorations
are line-based).
