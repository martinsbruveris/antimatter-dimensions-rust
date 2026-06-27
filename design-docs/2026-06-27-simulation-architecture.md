# Simulation Architecture: Driving the Game Without Polluting It

## 1. Problem Statement

A core goal of the project is **offline analysis of game dynamics**, culminating
in a *full end-to-end simulation* of a playthrough — from a fresh game to (much)
later, across prestige layers.

This is hard for a specific reason: the game **unlocks mechanics progressively**
and **automates manual tasks over time**. Early on, a player buys dimensions,
boosts, and galaxies by hand; later, autobuyers take over; much later, the
Automator (a scripting language) takes over higher-level decisions. To simulate
the whole arc we need a way to perform actions "manually" *before* the in-game
automation exists, and to hand off to in-game automation once it does.

Three constraints shape the design:

1. **We don't want to script every keystroke.** Specifying "buy 3 boosts, then a
   galaxy, then 5 boosts" by hand for an entire run is unworkable. We need a
   *middle ground* — compressed, declarative-or-scripted intent. We already have
   a taste of this with the `boost`/`boost:n` plan steps and `StrategyConfig`.
2. **Clean separation of game engine vs simulation engine.** Simulation must not
   be able to (accidentally) change game logic. A bug in the "manual player"
   layer must never alter what the game *would* do.
3. **There is a built-in analogue: the Automator.** The game already ships a
   tick-driven scripting interpreter that does almost exactly what we want.
   Should we reuse it, reimplement it, or build something separate?

This document surveys the architecture options, with reference to the original
JS source, and gives a recommendation.

## 2. The Central Question, Restated

The user's question — *"Do I reimplement the Automator interpreter at the
simulation level in addition to the game level, or can I reuse code?"* — conflates
two genuinely different things. Untangling them is most of the design work:

- **In-game automation** (Autobuyers, and later the Automator) is a **game
  mechanic**. It is part of the *rules*, unlocks at defined thresholds, runs at
  defined speeds, and *must be behaviourally faithful* to the original. It
  belongs in `ad-core`.
- **Simulation driving** (the "HumanAutobuyer", entering challenges manually
  before automation is available, choosing a strategy to study) is **not a game
  rule**. It is an external *agent* deciding which legal actions to take. It must
  *never* change game behaviour — it only chooses among actions the game already
  permits.

Once separated, the answer to the reuse question becomes clear (see §7), and the
"HumanAutobuyer" finds a natural home that is explicitly *not* the autobuyer
system.

## 3. What We Already Have (the seam is already here)

The engine already exposes a clean **action API** on `GameState` — a uniform
vocabulary of "things a player can do", each paired with a `can_*` predicate:

```
buy_dimension(tier) / buy_max_dimension / buy_until_10_dimension
buy_tickspeed / buy_max_tickspeed
buy_galaxy        / can_buy_galaxy
buy_dim_boost     / can_dim_boost
sacrifice         / can_sacrifice
big_crunch        / can_big_crunch
unlock_ad_autobuyer(tier), toggle_*  (autobuyer configuration)
```

Three distinct callers already drive the game *exclusively through this API*:

1. The **GUI** (`ad-gui`) — user clicks map to these methods.
2. The **Autobuyers** (`ad-core/autobuyers.rs`) — `tick_autobuyers` calls
   `buy_dimension` / `buy_until_10_dimension` / `buy_max_tickspeed` on a timer.
3. The **Simulator** (`ad-core/simulator.rs`) — `execute_strategy` calls
   `buy_max_dimension` / `buy_galaxy` / `sacrifice` / etc. driven by a
   `StrategyConfig`.

**This action API is the seam.** It is the one place where "intent" becomes
"state mutation". Every architecture option below is really a question of *who
produces actions* and *how that producer is expressed and isolated*. The seam
already exists; we mostly need to formalise and defend it.

A second relevant asset is `ObservedState` (`observed.rs`): a read-only snapshot
of `GameState` already used for traces and the GUI. This is the natural **input**
to any decision-maker — it lets a controller read the game without holding a
mutable borrow and without reaching into engine internals.

## 4. How the Original Automator Works (and why it matters)

Reference: `../antimatter-dimensions/src/core/automator/` —
`automator-backend.js`, `automator-commands.js`, and the command table in
`src/core/secret-formula/reality/automator.js`.

Salient facts, because they shape what "reuse" would cost:

- **It is a real interpreter.** Scripts are lexed/parsed/compiled by a Chevrotain
  grammar into an array of command objects, each with a `run(stackEntry)`
  method. There is a **stack VM** (`AutomatorStackEntry`, `stack`) to support
  nested blocks (`if`, `while`, `until`, loops).
- **It is tick-driven with an interval.** `AutomatorBackend.update(diff)`
  accumulates real time into `execTimer`, then executes up to
  `floor(execTimer / currentInterval)` commands per update (capped at
  `MAX_COMMANDS_PER_UPDATE = 100`). `currentInterval` itself scales with realities
  (`0.994^realities * 500 ms`, min 1 ms). So the Automator is *not* infinitely
  fast — it has a deliberate per-command cadence.
- **Commands yield across ticks.** `run()` returns a status
  (`NEXT_INSTRUCTION`, `NEXT_TICK_SAME_INSTRUCTION`, `HALT`, `RESTART`, …) that the
  backend uses to advance the instruction pointer or stop. `wait`/loop commands
  use this to span many ticks.
- **Commands call the same action API the UI does.** `prestige`, `auto`,
  `studies buy`, `unlock`, `start ec`, etc. ultimately invoke ordinary game
  methods (`Autobuyer.bigCrunch`, prestige resets, study purchases). The
  interpreter is glue; the verbs are game actions.
- **It is a *late* feature.** The Automator unlocks in Reality. For the entire
  pre-Reality arc (which includes everything currently implemented), it does not
  exist in the game at all.

Two consequences:

1. The Automator, *as a game mechanic*, is a `ad-core` responsibility we will have
   to implement anyway — faithfully — when we reach the Reality layer. Its
   execution model (tick-driven, interval-throttled, yielding commands over a
   stack VM) is exactly an in-engine controller.
2. Its grammar/VM is **heavyweight** and **only relevant late**. Using it as the
   mechanism for *early-game manual play* would mean shipping a parser and stack
   VM to express "buy 4 boosts then a galaxy" — a large mismatch in cost.

## 5. Design Axes

Every option is a point in three axes:

- **Who produces actions?** A declarative config interpreted by a fixed loop; a
  Rust trait object (`Controller`); or a script interpreted by a DSL VM.
- **Where does it live?** Inside `ad-core` (and how isolated), or in a separate
  crate that depends on `ad-core` one-way.
- **How expressive is the "intent" layer?** From flat config → reactive rule engine
  over a closed vocabulary → full imperative scripting language (the Automator). See
  §6½ for why the middle rung suffices until the Automator.

## 6. Options

### Option A — Grow the declarative `StrategyConfig` (status quo, extended)

Keep the current model: a config struct (`StrategyConfig`) is interpreted by a
fixed loop in `simulator.rs`. Extend it with more knobs and richer
`PrestigeMode::Plan` steps (challenges, study presets, "until X" conditions) as
new mechanics land.

- **Pros:** Already built and tested. No FFI of callbacks (Python passes config,
  gets results). Trivially serialisable, trivially sweepable in parameter studies.
  Deterministic by construction.
- **Cons:** Expressiveness ceiling. Config can express "a strategy", not "a
  sequence of conditional manual actions". Every new kind of decision needs a new
  field and new interpreter branches. `simulator.rs` becomes a god-loop. It does
  not generalise to "play the whole game across all layers".
- **Verdict:** Correct for the *current* narrow question (time-to-crunch parameter
  sweeps). Insufficient as the spine of a full end-to-end simulation.

### Option B — A `Controller` trait + a stable `Action` IR (recommended spine)

Introduce two things:

1. An **`Action` enum** — a serialisable, exhaustive vocabulary of legal player
   actions (`BuyDimension(tier)`, `BuyMaxTickspeed`, `Galaxy`, `DimBoost`,
   `Sacrifice`, `Crunch`, `EnterChallenge(id)`, …). One `apply_action(&mut self,
   Action)` dispatcher on `GameState` routes each to the existing methods. This
   formalises the seam from §3.
2. A **`Controller` trait** living *outside* `ad-core`:
   ```rust
   trait Controller {
       fn act(&mut self, obs: &ObservedState, now: SimTime) -> ActionBatch;
   }
   ```
   The simulation loop becomes: each tick, ask the active controller for actions,
   `apply_action` each, then run engine production. Controllers are pluggable:
   `HumanController` (throttled manual play), `StrategyController` (wraps today's
   `StrategyConfig`), `RuleEngineController` (Option C), `ScriptController` (the real
   Automator, Option D), `NullController` (let in-game autobuyers do everything).

- **Pros:** This *is* the clean separation the user wants, and the compiler
  enforces it: `ad-core` defines `Action`/`apply_action` but knows nothing about
  controllers; the simulation crate depends on `ad-core` one-way. A simulation
  bug cannot change game logic because controllers can only emit `Action`s the
  engine already validates. The same `Action` IR can later be the recording
  format ("replay"), the autobuyer output, and the Automator output — unifying
  three callers. The per-tick `act()` shape is exactly the Automator's own
  execution model, so the real Automator slots in as one controller later.
- **Cons:** A trait object on the action path (mitigated: actions are produced in
  batches per tick, not per-operation, so `dyn` cost is negligible vs production
  math). Slightly more indirection than calling methods directly. The IR must be
  kept in sync as mechanics grow (but this is a feature — it's the audited list of
  what a player can do).
- **Verdict:** The recommended backbone. It subsumes Option A (config becomes one
  controller) and hosts Options C/D as further controllers.

### Option C — A reactive rule engine (the pre-Automator manual era)

The manual arc before the Automator is **reactive, not imperative**: it is a set of
standing orders ("whenever you can afford a galaxy, buy one"; "when total antimatter
≥ 1e40, unlock this autobuyer"), not a sequence of statements with control flow.
This mirrors the real game, whose autobuyers *are* reactive rules and whose
imperative scripting (the Automator) only unlocks at Reality. So Option C is a
**rule engine**, not a mini-language. See §6½ for the full justification that this
suffices for everything pre-Automator.

Three primitives, all closed-vocabulary enums — no lexer, no grammar, no variables,
no arithmetic over game values:

```rust
// 1. Reactive rule: a standing order, optionally throttled / one-shot.
struct Rule {
    trigger: Trigger,               // closed predicate vocabulary
    action: ActionSpec,             // -> emits Action(s) via the §6/B IR
    interval: Option<SimDuration>,  // "human speed"; None = act every tick
    once: bool,                     // e.g. unlock an autobuyer exactly once
}

// Closed predicate vocabulary — enums, never free-form expressions:
enum Trigger {
    Always,
    Affordable(ActionId),           // can_buy_galaxy, can_dim_boost, ...
    AtLeast(Metric, Decimal),       // total_antimatter >= 1e40
    Unlocked(Feature),
    Completed(Challenge),
    And(Vec<Trigger>), Or(Vec<Trigger>), Not(Box<Trigger>),  // light combinators
}

// 2. Ordered plan: a list + cursor (infinity-upgrade order, challenge order).
struct Plan<T> { items: Vec<T>, cursor: usize }

// 3. Phase schedule: which rules/plans are active, and when to advance.
struct Phase { enter_when: Trigger, rules: Vec<Rule>, plans: /* ... */ }
```

A `RuleEngineController` (one Option-B controller) runs this each tick: evaluate the
active phase's rules against `ObservedState`, emit `Action`s, advance any plan
cursor whose completion condition fired, and advance the phase when its `enter_when`
holds. Today's `Vec<PrestigeStep>` + `PlanCursor` is already the `Plan<T>` primitive;
`StrategyConfig` is already a fixed-purpose rule set. This generalises both.

- **Pros:** Hits the "middle ground" — far more than flat config, with no parser and
  no language. Pure Rust enums: trivially constructible from Python or Rust,
  serialisable, deterministic. Faithful to how the game's own automation works
  (reactive standing orders). Sequencing needs (challenges, upgrade order) are
  handled by *cursors over data*, and phase changes by the *schedule* — neither
  requires imperative control flow.
- **Cons:** Not Turing-expressive: no user variables, computed values, `wait N`, or
  reusable subroutines. (That is the point — see §6½ — but it means there is a hard
  ceiling, and the temptation to breach it by adding expressions must be resisted.)
- **Verdict:** Adopt as the pre-Automator driver. The one discipline: keep `Trigger`
  and `ActionSpec` **closed enums** — no string expressions, no variables, no
  arithmetic over game values. The day you genuinely need those you have reached the
  Automator and should build/reuse that (Option D), not grow a shadow language here.

### Option D — Implement the Automator as a real game mechanic, then drive it

When the project reaches the Reality layer, the Automator must be implemented in
`ad-core` *anyway*, faithfully (grammar → compiled commands → stack VM →
interval-throttled tick execution). At that point the simulation reuses it
directly: feed it a script string, run ticks, done. No second interpreter.

- **Pros:** Zero reimplementation — the interpreter exists once, in the engine,
  as the genuine feature. Maximum fidelity (it *is* the game's Automator).
  Late-game runs can use real player scripts verbatim.
- **Cons:** Only available once Reality is implemented — useless for the early
  arc. Heavyweight (Chevrotain-equivalent parser + stack VM in Rust). Overkill for
  "buy 4 boosts". Pulls a large feature forward if attempted early just for
  simulation (don't).
- **Verdict:** This is the *answer to the reuse question*: implement the Automator
  **once**, in `ad-core`, as the real mechanic, and let the simulation invoke it
  via a `ScriptController` that wraps `AutomatorBackend`. Do **not** build a
  parallel Automator at the simulation level. Until that mechanic lands, use
  Option C for the manual era.

### Non-option — Reimplement the Automator interpreter at the simulation level

Explicitly rejected. It would mean two interpreters (one in `ad-core` as the game
feature, one in the sim layer), guaranteeing drift, doubling fidelity-test
surface, and front-loading a Reality-tier feature for early-game needs it doesn't
serve. The only legitimate "controller at sim level" is the *reactive rule engine* of
Option C, which is deliberately not an interpreter at all.

## 6½. Do We Need a Scripting Language Before the Automator?

**No.** Everything between a fresh game and the Automator's unlock (Reality) is
expressible with the Option-C rule engine. The reason is that pre-Automator play is
*reactive*, not *imperative*, and the decisions draw from a *closed, finite*
vocabulary of conditions and actions.

### The test that decides it

The real question is not "rules vs. a language" but whether the control logic needs
an **open expression language** or only a **closed predicate/action vocabulary**:

- **Closed vocabulary** — a finite, enumerated set of conditions (`can_big_crunch`,
  `total_antimatter ≥ 1e40`, `challenge C7 completed`) and actions (`Crunch`,
  `BuyMaxDimension(t)`, `EnterChallenge(id)`), combined with AND/OR/comparison but
  never extended with new terms. This is **configuration data**.
- **Open language** — arbitrary arithmetic, *user variables*, computed values
  ("set the autobuyer threshold to 1.8× current EP/min"), `wait N`, reusable
  parameterised subroutines, arbitrary nested control flow. This is a **programming
  language**.

A scripting language is justified only when the second is genuinely required. The
pre-Automator arc lives entirely in the first — and not by accident: the game's own
autobuyers are reactive rules, and the Automator (imperative scripting) unlocks only
at Reality. The architecture should mirror that progression, which also tells us the
language is not needed until the feature that *is* the language arrives.

### A representative pre-Automator playthrough, classified

Every step maps to one of three primitives — none of which is control flow:

| Step | Primitive |
|------|-----------|
| Human autobuyers (dim/tick/sacrifice/boost/galaxy) | **Reactive rule** + interval |
| Progressively unlock autobuyers at thresholds | **One-shot rule** (`am ≥ req`) |
| Keep human + real autobuyer both active a while | nothing — don't disable |
| Manual Big Crunch | **Reactive rule** `can_big_crunch → Crunch` + interval |
| Disable human dim/tick autobuyers; keep the rest | **Phase transition** (swap rules) |
| Repeated manual Big Crunch | same reactive Crunch rule |
| Buy infinity upgrades in a predetermined order | **Ordered plan** (`Plan<Upgrade>`) |
| Enter normal challenges at thresholds | **Ordered plan** (`Plan<Challenge>`) + rules |
| Update autobuyer settings over time | **Reactive threshold rules** |

The two steps that *look* like they need sequencing do not. "Predetermined order" is
**data** — an ordered list consumed greedily — not control flow. A challenge run is:
enter the challenge, let the *same reactive rules* play to `can_big_crunch`, crunch
(which completes it), advance the cursor. That is exactly the `PlanCursor` already in
`simulator.rs`, with per-challenge rule tweaks expressed as *config* (which rules are
active), not as branches.

### Where the boundary actually is

What a rule engine cannot cleanly express — and what the Automator exists to add:

- **User variables and computed values** (`x = ep/min; auto eternity x*1.8`).
- **`wait N` / `store game time` / restore** — stateful multi-step procedures with
  memory beyond a cursor.
- **Arbitrary nested loops with reusable bodies / subroutines.**

None appear in the pre-Automator arc. The closest is the human throttle interval, and
that is per-rule timer state, not a language feature. So the boundary is clean:
reactive rules carry the whole pre-Automator arc; the Automator is precisely the
point where the genuine interpreter is built/reused (Option D).

### The one slippery slope

`And`/`Or`/`Not` in `Trigger` flirt with "language", and that is fine **only while the
leaf terms stay a fixed enum**. A closed set of metrics combined with booleans is a
predicate; it becomes a language the moment variables or arithmetic over game values
are added. Hold that line and the rule engine cannot accidentally grow into a second
Automator.

## 7. Recommendation

A layered design combining B (spine), C (manual era), and D (late-game reuse):

```
┌──────────────────────────────────────────────────────────────┐
│  ad-sim  (NEW crate; depends on ad-core one-way)              │
│  • trait Controller { act(&ObservedState, SimTime) -> Batch } │
│  • HumanController       — throttled manual play (interval)    │
│  • StrategyController    — wraps today's StrategyConfig        │
│  • RuleEngineController  — Option C reactive rule engine       │
│  • ScriptController      — wraps ad-core Automator (Option D)  │
│  • run_simulation(controllers, schedule, stop) -> Result      │
├──────────────────────────────────────────────────────────────┤
│  ad-core  (game engine — never depends on ad-sim)            │
│  • enum Action  +  GameState::apply_action(Action)            │
│  • Autobuyers   (game mechanic; emits Actions internally)     │
│  • Automator    (game mechanic, when Reality lands)           │
│  • tick(): production + in-game automation                    │
└──────────────────────────────────────────────────────────────┘
```

Concrete decisions:

1. **Formalise the seam: add `Action` + `apply_action` to `ad-core`.** Refactor
   autobuyers and the existing simulator to route through it. This is low-risk and
   immediately pays off (one audited action vocabulary, replayable, the FFI-stable
   boundary for Python).
2. **Move the simulation driver out of `ad-core` into a new `ad-sim` crate.**
   `strategy.rs` + `simulator.rs` migrate; the dependency is one-way
   (`ad-sim → ad-core`). This makes "simulation cannot change game logic" a
   *compile-time* guarantee, not a guideline — the strongest possible separation,
   and the direct answer to constraint 2. (If a full crate split is premature, the
   interim is an `ad-core::sim` module gated behind a `sim` cargo feature, relying
   on the `Action` seam for discipline; but the crate split is the goal.)
3. **The `Controller` trait is the unifying abstraction.** Its per-tick `act`
   shape matches both the autobuyer timer model and the Automator's
   interval-throttled VM, so every "action producer" — manual, strategy, plan,
   real Automator — is one implementation.
4. **The "HumanAutobuyer" is a `HumanController`, *not* an autobuyer.** Model
   human slowness as a controller-side interval (act every N ms of game time)
   rather than an instantaneous decision. Crucially it lives in `ad-sim`, so it
   can never be mistaken for, or accidentally alter, the real autobuyer mechanic.
   The hand-off the user described — "turn the human off after Big Crunch once
   autobuyers are unlocked" — becomes a *schedule*: controllers are
   enabled/disabled at defined milestones (see below).
5. **For "manual but not keystroke-by-keystroke", use the reactive rule engine
   (Option C), not a language.** Express the whole pre-Automator arc as standing
   rules over a closed predicate/action vocabulary, plus ordered plans (cursors) for
   sequencing and a phase schedule for hand-offs (§6½). Keep `Trigger`/`ActionSpec`
   closed enums. Reserve a real text language for when the Automator mechanic
   (Option D) exists, then reuse it.

### Composition / hand-off ("schedule")

The full-run driver needs to switch who is in control as the game evolves. Model
this as a **schedule** of `(condition, controller-set)` — e.g.:

- *Fresh game → first Big Crunch:* `HumanController` active; in-game autobuyers
  off (they aren't unlocked yet anyway).
- *After autobuyers unlock:* disable `HumanController`; let `ad-core` autobuyers
  run inside `tick()` (controller = `Null`). The engine drives itself faithfully.
- *After Reality:* a `ScriptController` wrapping the real Automator can take over
  high-level decisions.

Conditions are evaluated against `ObservedState`, keeping the schedule declarative
and deterministic.

## 8. Why This Keeps Game and Simulation Cleanly Separated

- **One-way dependency.** `ad-core` has no `use ad_sim::*`. The compiler forbids
  game logic from referencing simulation — accidental coupling is impossible.
- **Actions are the only mutation path.** Controllers cannot poke fields; they
  emit `Action`s the engine validates (`can_*` checks already exist). A buggy
  controller can at most choose a *legal* action at the wrong time — it cannot
  invent an illegal state transition.
- **In-game automation stays in the engine.** Autobuyers and the Automator are
  game rules and live in `ad-core`; they are *not* `Controller`s. The simulation
  can switch them on/off (as a real player can) but cannot redefine them.
- **Determinism preserved.** Controllers read `ObservedState` and a `SimTime`
  argument — no `SystemTime`, no unseeded RNG — matching the engine's existing
  determinism guarantee. Reproducible runs and fidelity tests still hold.
- **Replay & audit.** Because every run is a stream of `Action`s, runs can be
  recorded and replayed against the engine alone, decoupled from whichever
  controller produced them — useful for regression and fidelity testing.

## 9. Risks & Caveats

- **`Action` IR churn.** The enum must track every mechanic. Mitigation: it is the
  intended single source of truth for "what a player can do"; growing it is
  normal, and an exhaustive `match` in `apply_action` makes omissions compile
  errors.
- **Rule-engine scope creep (Option C).** The closed predicate vocabulary can drift
  toward a language if variables or arithmetic over game values are added.
  Mitigation: hard rule — `Trigger`/`ActionSpec` stay closed enums and *no text
  parsing in `ad-sim`*; if you need expressions, you need the Automator (Option D),
  so build that instead.
- **Double interpreter temptation.** The pressure to "just script the early game
  with Automator syntax" will recur. Mitigation: accept that the early arc uses
  the reactive rule engine and the late arc reuses the real Automator; never two
  interpreters.
- **Throttle fidelity for `HumanController`.** "Human speed" is a modelling choice,
  not a game rule. Keep it a tunable interval and document that runs using it are
  *behavioural studies*, not fidelity baselines.
- **Crate-split cost.** Moving `simulator.rs`/`strategy.rs` to `ad-sim` touches
  `ad-python` (which currently expects them in `ad-core`). Low effort but a
  coordinated change; do it alongside introducing `apply_action`.
- **Performance.** `dyn Controller` dispatch and `Vec<Action>` allocation per tick
  are negligible next to per-tick Decimal production, *provided* controllers act
  in batches (and idle controllers can short-circuit on an interval). Avoid
  per-operation virtual calls; keep `apply_action` a static dispatch.

## 10. Suggested Phasing

Status legend: ✅ done · ▶ next · ☐ pending.

1. ✅ **Introduce `Action` + `GameState::apply_action` in `ad-core`.** Adds the
   audited seam. No behaviour change. (See §10.1 for what shipped.)
2. ▶ **Create `ad-sim`; define `Controller` + `run_simulation`.** Move
   `strategy.rs`/`simulator.rs` over as `StrategyController`. Update `ad-python`
   imports. Now separation is compiler-enforced.
3. ☐ **Add the `RuleEngineController` (rules + plans) and the phase schedule.** The
   `HumanController` is then just a rule set with throttle intervals. Enables the
   first true end-to-end "manual until autobuyers, then engine-driven" run through
   Big Crunch.
4. ☐ **Grow the rule/plan vocabulary (Option C)** as new mechanics (challenges,
   infinity upgrades, studies) land — adding `Trigger`/`ActionSpec`/`Plan` variants
   and expanding `Action` in step. No control-flow syntax; keep the vocabulary
   closed.
5. ☐ **When Reality lands, implement the Automator in `ad-core`** as the real
   mechanic and add a thin `ScriptController` wrapper in `ad-sim`. No second
   interpreter is ever written.

### 10.1 Implementation status (as of 2026-06-27)

**Milestone 0 (phase 1) — done.** The `Action` IR seam shipped in `ad-core`:

- New `crates/ad-core/src/action.rs`:
  - `enum Action` — the closed action vocabulary for current pre-Infinity
    mechanics: `BuyDimension`/`BuyMaxDimension`/`BuyUntil10Dimension(tier)`,
    `BuyTickspeed`/`BuyMaxTickspeed`, `Galaxy`, `DimBoost`, `Sacrifice`, `Crunch`,
    `UnlockAdAutobuyer(tier)`, `UnlockTickspeedAutobuyer`, `SetAutobuyers(bool)`.
    Derives `serde` behind the existing `serde` feature.
  - `struct ActionOutcome { applied: bool, count: u64 }` (`count` carries bulk-buy
    quantity).
  - `GameState::apply_action(&mut self, Action) -> ActionOutcome` — one exhaustive
    `match` routing to existing methods (`buy_*`, `buy_galaxy`, `buy_dim_boost`,
    `sacrifice`, `big_crunch`, `unlock_*`). Illegal actions are no-ops returning
    `applied == false`; the per-mechanic `can_*` guards already enforce legality.
  - 6 unit tests (routing, unaffordable no-op, bulk count, crunch, requirement-gated
    unlock, global toggle).
- `ad-core/src/lib.rs` exports `Action`, `ActionOutcome`; `AGENTS.md` lists
  `action.rs` under *Key Source Files (ad-core)*.
- Verified: `cargo test -p ad-core` green; `cargo build -p ad-core --features serde`
  compiles; `cargo clippy -p ad-core` clean. `apply_action` has no callers yet — the
  wiring happens in Milestone 1.

**Resume at Milestone 1 (phase 2).** Agreed decisions for that step:

- **Fully replace** `ad_core::simulator` with `ad-sim` — no compatibility shim in
  `ad-core`. `ad-python` is the only consumer and is updated in the same change.
- Move `strategy.rs` + `simulator.rs` (and their tests, incl. any in
  `ad-core/tests/`) into a new `crates/ad-sim`; drop the corresponding re-exports
  from `ad-core/src/lib.rs`; repoint `ad-python` from `ad_core::{simulator,strategy}`
  to `ad_sim::*` (Python API surface unchanged).
- Define `trait Controller { fn on_start(&mut self, &mut GameState) {} fn act(&mut
  self, &ObservedState, game_time_ms: f64) -> Vec<Action>; }`. `run_simulation`
  drives `on_start` → per tick `act` → `apply_action` each → `game.tick(dt)` →
  reuse existing `StateTrace`/`StopCondition`/`StopReason`. Re-express today's logic
  as `StrategyController`; keep `simulate(config)` as a thin wrapper for API parity.
- `on_start` resolves the autobuyer-coexistence divergence: `StrategyController`
  disables in-game autobuyers (as today); the future `RuleEngineController` leaves
  them on so human + unlocked autobuyers coexist.

**Working-tree note (2026-06-27):** an unrelated in-progress change (a new
`ad-core` `Options` module: `options.rs`, `state.rs`, and several `ad-gui` files)
was already uncommitted when Milestone 0 was written and is independent of this
work. It leaves `state.rs` with `cargo fmt` drift to be cleaned up as part of *that*
change, not the simulation work.

## 11. Direct Answers to the Original Questions

- *"Is it reimplementing an interpreter at simulation level in addition to game
  level?"* — **No.** Implement the Automator once, in `ad-core`, as the real game
  feature (when Reality lands), and let the simulation invoke it. The pre-Automator
  era needs no language at all — a **reactive rule engine** over a closed vocabulary
  covers it (§6½).
- *"Can I reuse code and functionality?"* — **Yes, via two shared assets:** the
  `Action` IR (one vocabulary used by the GUI, autobuyers, the simulator, and
  eventually the Automator) and, later, the engine's own Automator (reused
  wholesale by a `ScriptController`).
- *"Can I add functionality to the game engine while keeping game and simulation
  cleanly separable?"* — **Yes.** Add only the `Action`/`apply_action` seam to
  `ad-core`; put everything that *decides* in a one-way-dependent `ad-sim` crate.
  The separation then holds by compilation, not convention.
