# A Config-Driven Game Engine for Antimatter Dimensions — Design Exploration

*Blue-sky design study. No implementation is planned.* The current codebase hard-wires
each mechanic in Rust (see `crates/ad-core/src/{dimensions,tick,action,sacrifice,galaxy,
crunch}.rs`), and for the pre-Infinity slice that is implemented today that is the right
call: the logic is small, readable, and directly testable against the original. This
document asks a different question:

> If we wanted to cover the *whole* game — every phase in
> [`2026-06-23-feature-decomposition.md`](./2026-06-23-feature-decomposition.md) **plus** the
> major mods (Endgame, Redemption, Vis; see
> [`2026-06-28-endgame-1.0-analysis.md`](./2026-06-28-endgame-1.0-analysis.md),
> [`2026-06-25-redemption-analysis.md`](./2026-06-25-redemption-analysis.md),
> [`2026-06-24-vis-analysis.md`](./2026-06-24-vis-analysis.md)) — could the features be
> expressed as *instances* of a small, config-driven engine rather than ~37 hand-wired
> subsystems? And would that make game-design and design-space exploration easier?

The short answer: **yes, to a well-defined point, and the original game is the proof.**
But "config-driven" is a spectrum, and the interesting design work is choosing *where on
that spectrum to stop* — because the two motivating goals (faithful replication of the
original, and free-form design-space exploration) pull in opposite directions.

---

## 1. Reframing the question

Two things get conflated under "config-driven," and separating them is the single most
useful move in this whole analysis:

- **Parameter exploration.** Change *numbers* in existing formulas: dimension base costs,
  cost multipliers, buy-10 multiplier, galaxy thresholds, prestige-gain exponents,
  softcap breakpoints, autobuyer intervals. The *shape* of the game is fixed; you are
  tuning a curve.
- **Mechanic invention.** Change *structure*: add a production chain that feeds two tiers
  down instead of one, a new prestige layer, a challenge that rewrites the production
  rule, a currency with a novel gain law.

Nearly all *tuning* and *balance* work in an incremental game is the first kind. Nearly
all *design* work is the second. A config engine can make the first kind essentially free
(sweep a data table, no recompile) while the second kind will always bottom out in code or
in a scripting language. Any honest recommendation has to say which one it is optimizing.

The second framing tension is **fidelity vs. exploration**:

- The original game *is* the spec. Reproducing it means matching exact formulas, softcaps,
  breakpoints, and — critically — **order of operations** in the multiplier chain.
  Reordering two multipliers changes the result. This is why the project already carries a
  fidelity test suite ([`2026-06-23-fidelity-test-plan.md`](./2026-06-23-fidelity-test-plan.md)).
- Exploration wants to *break* the spec freely and see what happens.

A good engine keeps these as two configs over one structure: a "canonical" config that is
validated bit-for-bit against recorded original traces, and arbitrary "experiment" configs
derived from it. The fidelity suite becomes the regression guard for the engine itself.

---

## 2. The evidence: Antimatter Dimensions is already an engine

The strongest argument that a config-driven engine is *feasible* is that the original
authors already built one — informally, in JavaScript — and **three independent mods
stress-tested it without adding a single new paradigm.** This is not a claim from one
analysis; the Endgame, Redemption, and Vis analyses each reach it separately, in a section
each titled verbatim **"No New Architectural Paradigms."**

From the Endgame v1.0 analysis (§5, "Architecture Assessment"): a mod that adds **~22,000
core JS lines, ~16 new currencies, two new dimension families, and a dozen mechanics**
reuses, verbatim, the same handful of base classes:

- `DimensionState` — every dimension family (Antimatter, Infinity, Time, Celestial, Divine,
  Dark Matter): 8 tiers, `tier[k+1]` produces `tier[k]`, `tier[0]` produces a currency,
  `{amount, bought, cost}`, buy-10 multiplier, exponential cost scaling.
- `BitPurchasableMechanicState` — one-shot upgrades stored as bits (infinity, break-infinity,
  reality, dilation, duality upgrades).
- `RebuyableMechanicState` — geometric-cost repeatable upgrades (IP/EP multipliers,
  tickspeed cost, DT gain).
- `SetPurchasableMechanicState` / tree structures — perks, time studies.
- `GameMechanicState` + `secret-formula/*` data tables — the base for everything, with all
  numeric/formula config living in a data directory separate from logic.

That is a **mechanic vocabulary of roughly eight primitives**. Enumerated against our full
scope (feature-decomposition phases 1–7 + Endgame):

| Primitive | Instances across the whole game |
|---|---|
| **Production chain** (N-tier dimensions) | AM, Infinity, Time, Celestial, Divine, Dark Matter dims — plus replicanti as a degenerate 1-"tier" self-producer |
| **Currency** | ~30+: antimatter, IP, EP, RM, iM, TP, DT, relic shards, singularities, alchemy resources, dual machines, divine matter/energy, null matter… |
| **One-shot purchasable** | infinity/break-infinity/reality/dilation/duality upgrades, imaginary upgrades, milestones-as-unlocks |
| **Rebuyable purchasable** | IP/EP mult, tickspeed cost, dim cost, DT gain, galaxy-threshold buys |
| **Tree purchasable** | time studies (~220), perks (~100), automator commands as a graph |
| **Effect / modifier** | every achievement, upgrade, glyph, time study, milestone, challenge reward that contributes a ×/^/+ to some target |
| **Prestige layer** | Infinity, Eternity, Reality, Celestial-Infinity/Eternity, Endgame, Alpha stages |
| **Rule-modifying run** (challenge/reality) | 12 NC, 8 IC, 12 EC, dilation, 7 celestial realities, Pelle |
| **Milestone / threshold unlock** | eternity milestones, singularity milestones, V-achievements, divinity milestones, accelerators |
| **Autobuyer** | ~30 across all mechanics: {interval, mode, condition, target action} |

The feature-decomposition doc's own "Cross-Cutting Concerns" section already names the
engine's would-be subsystems without calling them that: *Multiplier Pipeline, Challenge
Modifiers, Prestige Reset Chain, Records, Cost Scaling, Save/Load.* The engine is latent in
the analysis; the question is only whether to make it explicit in Rust.

### 2.1 Three mods, one verdict

The single-mod evidence would be suggestive; the *three-mod* evidence is decisive. Endgame,
Redemption, and Vis were built by different authors, extend the game in different directions
(vertical prestige-stacking vs. horizontal in-layer complexity vs. multiple parallel
celestials), and are **mutually exclusive** — yet all three land as instances of the same
vocabulary:

| Aspect | Endgame | Redemption | Vis |
|---|---|---|---|
| Core lines added | +22k / +35% (v1.0) | +11.3k / +18% | +8.5k / +14% |
| New prestige layer | Endgame | Mending | Meta |
| New celestials | Alpha | Kohler, Destroyer* | Glitch, Cante, Null |
| New dimension families | Celestial (8T), Divine (8T) | Multiversal (8T), Matter (4T) | Chaos (12T) |
| New currencies | ~16 | 8 | 9 |
| Nested reset sublayers | — | Corrupted Mends | Plynia, Null Parallax/Corruption, Cante Reforge/Purge |
| Analysis verdict | "no new paradigms" | "no new paradigms" | "no new paradigms" |

\*Destroyer is a placeholder shell. Every row above is *more instances of the ten
primitives*, not a new primitive: new prestige layers are `PrestigeLayer` config (including
the nested sublayers — Plynia, Parallax, Corruption, Reforge/Purge are simply
`PrestigeLayer`s scoped inside a celestial); Corruption is a parameterized `Rule-modifying
run` plus a scoring `Formula`; Glitch challenges are `Rule-modifying run` × `Milestone`
hybrids; the 25×-per-category Mending/Warp/Kohler/Meta upgrades are `Purchasable` rows.

Two independent details in these analyses *reinforce* recommendations made later in this
document, by negative example:

- **Scattered conditionals confirm the effect-bus case (§3, Approach 3).** Both the
  Redemption and Vis analyses flag, as a *concern*, effects wired in via
  `MendingUpgrade(N).isBought` / `GlitchSpeedUpgrade(N).isBought` sprinkled across the
  AD/IP/EP/Pelle code. That is precisely the coupling a data-row effect bus dissolves — the
  mods are field evidence that the hand-wired multiplier chain does not scale.
- **Duplication confirms the parameterize-don't-copy case.** Redemption re-implements the
  Black Hole as a parallel 687-line `ExpoBlackHoleState` rather than reusing the base one;
  its own analysis says a Rust port "could parameterize the existing Black Hole struct
  rather than duplicating it." The config-engine thesis is stated by the analysis itself.
- **Mutual exclusivity is an argument *for* config bundles.** Vis and Endgame edit the same
  base files incompatibly, so in JS they can only exist as forks. A config engine expresses
  each mod as an alternative **config bundle** swapped in at load time — turning a
  fork-management problem into a data-selection one. This benefit is invisible from any
  single game and only appears once you look across mods.

### 2.2 The one real refinement: production *topology* must be config, not assumed

The mods do force exactly *one* honest amendment — not a new primitive, but a missing
*parameter* on the production-chain primitive. The base game (and today's `tick()`) bakes in
a single shape: `tier[k+1]` produces `tier[k]`, and `tier[0]` produces a currency. Vis
breaks that assumption twice:

- **Cante Replicators** — each tier produces *more of itself*, not the tier below
  (self-replication).
- **Null Cycles** — a 16-tier **ring**: the highest unlocked tier loops back to tier 1 *and*
  emits currency.

The Vis analysis calls this out directly: "the dimension production trait/enum needs to
support these variants." The fix is not another primitive — it is promoting production
**topology** to a config axis on the existing `DimensionFamily`:

```rust
enum ProductionTopology {
    DownwardChain,     // tier[k+1] -> tier[k] -> currency   (AM/ID/TD/Celestial/Divine/…)
    SelfReplicating,   // tier[k]   -> tier[k]               (Cante Replicators; replicanti)
    Ring,              // top tier  -> tier[0] + currency    (Null Cycles)
}
```

This is a textbook instance of the §3 "under-fit and you bolt on conditionals" risk: had the
engine hard-coded the downward chain, Cante and Null would each need a special case. Making
topology data absorbs both — and, as a bonus, subsumes **replicanti** (a degenerate
self-replicator the base game already special-cases). Cost scaling gets the same treatment:
the mods add `HyperExponentialCostScaling` and hybrid scaling, i.e. more `CostScaling` enum
variants — instances of a primitive, not new primitives.

**Conclusion of §2:** across the base game and three independent, mutually-exclusive mods
(~+42k JS lines combined), the primitive set is small (≈10), stable, and crisply
identified — the count and kind of primitives never grew. The *only* structural adjustment
the mods demand is a topology parameter on one primitive. That is exactly the precondition
under which a config-driven engine pays off rather than becoming a speculative framework.

---

## 3. The design spectrum

Five points on the "how config-driven" axis, from today's code to a full DSL. Each is
described by *what it is*, *how an AD feature maps onto it*, and *trade-offs*. They are
cumulative — later options subsume earlier ones.

### Approach 0 — Hard-wired logic (status quo)

Each mechanic is bespoke Rust. `dimension_multiplier` (dimensions.rs:112) is the archetype:
buy-10, dim-boost, sacrifice, and achievement terms are inlined `if` blocks in a fixed
order.

- **Pros:** minimal ceremony; the code reads like the mechanic; trivial to match the
  original line-by-line; fastest possible hot loop; no abstraction to get wrong.
- **Cons:** every new effect edits a growing central function; the ~200 effect sources of
  the full game turn `dimension_multiplier`, IP/EP-gain, and game-speed into sprawling
  conditionals (the Endgame doc flags exactly this — milestone checks "threaded through far
  more systems"); parameters are baked into code, so exploration means editing and
  recompiling; adding Celestial/Divine dims means copy-pasting the AM dimension logic.
- **Break-even:** fine through Phase 1. Starts hurting in Phase 2 (achievements + challenges
  + infinity upgrades all pile effects onto the same targets), painful by Phase 4+.

### Approach 1 — Typed data tables + enum-dispatch formulas

The architecture doc (§4, §5.3) already proposes this. Config is **const/static Rust data**
(`ANTIMATTER_DIMENSIONS: [DimensionCostConfig; 8]`); formulas that can't be pure data are an
enum `FormulaId` / `EffectFormula` resolved to a pure Rust `fn(&GameState) -> Decimal`.

```rust
enum EffectFormula {
    Constant(Decimal),
    TimeInEternity { max_minutes: f64, multiplier: f64 },
    Computed(FormulaId),          // dispatches to a named pure fn
}
```

- **Pros:** all *numbers* become data → parameter sweeps are free and type-checked;
  formulas stay in Rust so fidelity and performance are unaffected; no `dyn`; the compiler
  still forces exhaustiveness.
- **Cons:** structure is still code — a new *kind* of formula is a new enum variant + fn +
  recompile; nothing yet unifies "an achievement and an upgrade are both just effects";
  config lives in the binary (can't hot-reload or author from Python without a rebuild).
- **This is the cheapest step up from status quo and is almost pure upside.** Even if we go
  no further, moving every literal in `data/constants.rs` behind a `GameConfig` struct that
  the engine reads (instead of `const`) unlocks the bulk of design-space exploration.

### Approach 2 — A generic mechanic framework (the "features as instances" answer)

Model the ≈8 primitives from §2 as a small set of Rust structs, each parameterized by data
plus a few formula hooks. A **feature** becomes a *bundle of instances*, not a module of
code. Sketch:

```rust
struct DimensionFamily {
    tiers: u8,
    produces: ProduceTarget,          // AM, InfinityPower, TimeShards, DarkMatter…
    cost: CostScaling,                 // Exponential { base, mult, softcap } | Linear{…}
    buy10_mult: Formula,
    per_tier: [TierData; MAX_TIERS],
}

struct Purchasable {
    id: PurchasableId,
    cost: Cost,                        // fixed | geometric(rebuyable)
    currency: CurrencyId,
    requires: Condition,               // prerequisite upgrades / thresholds
    effects: Vec<EffectId>,            // what it contributes when owned
}

struct PrestigeLayer {
    trigger: Condition,                // e.g. IP >= 1e308
    gain: Formula,                     // EP = 5^(log10(IP)/308 - 0.7) * mult
    resets: ResetSpec,                 // which state classes clear
    keeps: Vec<(KeepFlag, Condition)>, // milestone/perk-gated "keep" rules
}

struct RuleModifier {                  // a challenge / celestial reality
    overrides: ProductionRule,         // e.g. tickspeed affects only tier 8
    disables: DisableSet,              // sacrifice off, galaxies off…
    goal: Formula, reward: EffectId,
}
```

The engine holds `Vec`s (or typed arenas) of these and iterates generically: production
walks every `DimensionFamily`; purchase resolves a `PurchasableId`; a prestige runs a
`PrestigeLayer`; `active_modifiers()` folds the running `RuleModifier`s. This is a direct
Rust transcription of the JS class hierarchy.

- **Pros:** new content is *data* — Celestial and Divine dimensions are two more
  `DimensionFamily` rows, not two new modules (the Endgame doc's headline recommendation:
  "a port that parameterizes the crunch/eternity machinery will absorb [the Celestial
  Dimension Expansion] for free"); one code path per primitive means one place to get the
  formula right and one place to test it; the effect/challenge/prestige subsystems match the
  cross-cutting concerns 1:1.
- **Cons:** real up-front design cost; the abstractions must be *exactly* expressive enough
  for the original's special cases (sacrifice only on tier 8; tickspeed-affects-only-tier-8
  in NC9/EC7; continuum replacing buy-10 in Lai'tela) — under-fit and you bolt on
  conditionals anyway, over-fit and you've built a framework nobody needs; generic iteration
  is slightly slower than inlined code (mitigable, see §5); the hand-enumerated `Action` enum
  (action.rs) must become data-driven too (`BuyPurchasable(PurchasableId)`), losing some
  compile-time exhaustiveness.
- **Break-even:** this is the design that "pays for itself" if and only if we commit to
  Phases 3–7. For a pre-Infinity-only game it is over-engineering.

### Approach 3 — Declarative effect graph / dataflow

Go beyond "instances" to a **dependency graph**. Every currency, dimension multiplier,
game-speed, and threshold is a *node* with declared inputs and a pure recompute function;
effects are *edges*. The engine topologically evaluates dirty nodes each tick (or on
demand). This is the "spreadsheet model" of the game.

The multiplier pipeline is the natural fit. Instead of `dimension_multiplier` reaching into
state, model a global **effect bus**:

```rust
struct Effect {
    source: Condition,        // active when: achievement N unlocked / upgrade owned / in EC7…
    target: Target,           // ADMult(tier) | ADMultAll | IPGain | GameSpeed | GalaxyThreshold…
    op: Op,                   // Mul | Pow | Add | Max | Set
    value: Formula,
    combine: CombineGroup,    // e.g. same-type glyph effects combine additively then apply
}
```

`ad_mult(tier)` becomes: gather every `Effect` whose `target` matches and whose `source` is
active, group by `combine`, fold in the canonical order. **Now an achievement, an infinity
upgrade, a glyph, a time study, and a challenge reward are *the same thing* — rows in the
effect table with different `source` conditions.** That is the literal "every feature is a
special case of one mechanism" the question asks about.

- **Pros:** maximally uniform; adding an effect never edits a production function; the graph
  is *introspectable* — you can dump "what feeds AD tier-3 multiplier right now," which is
  gold for both debugging fidelity mismatches and for design-space exploration UIs; caching
  falls out naturally (dirty-tracking per node; the architecture doc's tick-generation cache
  in §5.6 is a coarse version of this).
- **Cons:** the ordering/combination rules are the whole game's difficulty, and a graph
  doesn't remove them — it just moves them into `CombineGroup` semantics that must exactly
  match the original (glyphs combine per-type; some effects are `max`, some multiply, some
  are softcapped *after* summation); effects that read *arbitrary* state ("mult based on
  current AM," "based on slowest challenge time," "based on time in this infinity") make the
  graph dense — every such node depends on half the state; heavier machinery, easy to
  over-build.

### Approach 4 — Embedded expression DSL / scripting

Push *formulas themselves* into config: a small embedded expression language, or a scripting
runtime (Rhai/Rune/Lua), so `gain = 5^(log10(IP)/308 - 0.7)*mult` is authored as data and
evaluated at runtime — no recompile to change *structure*, not just parameters.

- **Pros:** the only option where genuinely *new mechanics* can be authored without touching
  Rust; ideal for a design sandbox where a human (or Python) writes a candidate formula and
  runs a sweep; config becomes fully external/hot-reloadable.
- **Cons:** **fidelity risk** — the original's formulas rely on JS float semantics and on a
  bignum library; re-expressing hundreds of them in a DSL is a fresh transcription surface
  for bugs, and the fidelity suite would have to cover the DSL, not just the engine;
  **performance** — interpreting expressions inside a loop that runs 10⁵–10⁶ ticks per
  simulation (offline replay caps at 100k ticks; Python experiments sweep far more) is
  1–2 orders of magnitude slower than compiled Rust unless every formula is cached to a
  scalar per tick; **number type** — the DSL must be generic over `Decimal`/break-eternity,
  or you lose tetration-scale range that Endgame needs (Penteract costs `pow10(pow10(105+…))`).
- **Verdict:** the right tool for the **Automator** (it *is* a scripting language the game
  exposes to players — see [`2026-06-27-simulation-architecture.md`](./2026-06-27-simulation-architecture.md) §6,
  Option D) and arguably for a designer-facing experiment sandbox, but the *wrong* substrate
  for the canonical, fidelity-locked game rules.

### Approach 5 — Full ECS

Entities = mechanic instances, components = `{Amount, Cost, Effect, …}`, systems =
production/purchase/prestige. Mentioned for completeness. It buys the same uniformity as
Approach 2/3 but imports an ECS's scheduling/query machinery that a single-threaded,
deterministic, fully-known-at-design-time simulation doesn't need. **Not recommended** —
the entity set is static and small; a `Vec<DimensionFamily>` is simpler than an archetype
store and easier to make deterministic for fidelity tests.

---

## 4. How each subsystem maps to a config engine

Grounding the spectrum in AD's actual cross-cutting concerns (feature-decomposition §
"Cross-Cutting Concerns"):

- **Production chains.** The most regular thing in the game. A single generic
  `tick_dimension_family(&mut family, dt)` replaces per-family loops. The current
  `dimension_production_per_second` (dimensions.rs:156) already has the exact shape
  (`amount × multiplier × tickspeed_effect`); generalizing it over a `produces: Target`
  and a per-family multiplier source is mechanical. Two config axes are required for the
  mods (§2.2): the `ProductionTopology` (downward-chain / self-replicating / ring — Vis's
  Cante Replicators and Null Cycles), and a `PurchaseMode { Discrete10, Continuum }` for
  Lai'tela's **continuum** (fractional purchasing that replaces buy-10). Both are parameters
  on the family struct; neither is a new mechanic. Skip either and the mods degrade into
  special cases bolted onto the generic path — the failure mode §3 warns about.

- **Effect / multiplier pipeline (the crux).** This is where Approach 3's effect bus earns
  its keep even if the rest stays Approach 2. It is *also* the highest-fidelity-risk
  subsystem because of ordering and combination rules. Recommendation: model effects as data
  rows, but keep the *fold order* and *combine semantics* as explicit, tested code — not as
  emergent graph behavior.

- **Challenges / rule-modifying runs.** Exactly the architecture doc's `ActiveModifiers`
  (§5.5): one field per rule a challenge can bend, folded from whichever runs are active.
  Config-driven form: each challenge is a `RuleModifier` row. The list of *bendable rules*
  (max tier, cost override, tickspeed target, disabled mechanics) is a closed, code-defined
  vocabulary — you can't data-drive a rule the engine doesn't know how to bend, and the
  original bends a lot of them, so this vocabulary grows with the phases.

- **Prestige / reset chains.** A `PrestigeLayer` with a `ResetSpec` (which state classes
  clear) and milestone/perk-gated `keeps`. The "keep" matrix is enormous by Reality
  (eternity milestones alone in feature-decomposition §4.2 list ~20 keep rules). Data-driving
  the keep rules — `(what_to_keep, gating_condition)` pairs — is a clear win over nested
  `if` ladders.

- **Cost scaling.** Already flagged as a reusable utility: `Exponential { base, mult,
  softcap }`, `Linear { base, incr }`, `FreeTickspeed`. This is straightforwardly a small
  enum + data; no reason *not* to do it even in the status-quo design.

- **Unlock / visibility gating.** `dim_is_shown` / `dim_available_for_purchase`
  (state.rs:179–196) are hand-coded conditions today. A `Condition` DSL (a small enum:
  `And/Or/Not/ThresholdReached/UpgradeOwned/MilestoneReached/…`) unifies visibility,
  purchasability, effect-activation, and prestige triggers under one evaluable type. This
  `Condition` type is the connective tissue of Approaches 2–3 and is worth prototyping first.

- **Autobuyers.** Already the most "config-like" subsystem in the current code
  (autobuyers.rs: `{interval, mode, is_active, target}`). Generalizes cleanly to
  `Autobuyer { target: Action, interval, mode, condition }` over the data-driven action set.

- **Actions & save.** Two consequences to flag. (1) The stable `Action` IR (action.rs) —
  which the simulation architecture doc leans on as the drive seam — must shift from a
  hand-written enum to data-parameterized variants (`BuyPurchasable(id)`,
  `Prestige(layer)`), trading some exhaustiveness for open extensibility. (2) Save/load
  becomes easier in one way (state is uniform collections keyed by id) and harder in another
  (id stability across config versions becomes a migration concern — the Endgame mod already
  ships its own `endgame-migrations.js`, per the analysis doc).

---

## 5. Cross-cutting constraints (the things that actually decide it)

1. **Fidelity is the dominant constraint.** The engine's canonical config must reproduce the
   original bit-for-bit, which means the abstractions cannot be *cleaner* than the original's
   own special cases. Every place the original does something irregular (sacrifice only on
   AD8; tickspeed retargeted in NC9/IC7/EC7; continuum in Lai'tela; the dilation compression
   `10^(sign·|log10 x|^0.75)`) is a place the generic model must have an escape hatch. The
   asset here is the existing fidelity harness: a config engine is *validated the same way
   the hand-written code is*, so generalization doesn't cost test coverage — it reuses it.

2. **Performance / the hot loop.** Design-space exploration *is* batch simulation
   (`ad-python`, `ad-fidelity`), and offline replay already runs up to 100k discrete ticks
   (tick.rs). The production inner loop must stay allocation-free and branch-light. This is
   the argument for keeping *structure* compiled (Approaches 1–2) and against a per-tick DSL
   (Approach 4). The reconciliation: effects and multipliers are **recomputed only when
   inputs change**, not every tick — the tick-generation cache (architecture §5.6) or a
   dirty-node graph (Approach 3) collapses the effect bus to a handful of cached scalars that
   the hot loop reads. Config-driven *setup*, compiled *steady state*.

3. **Number type.** Endgame needs tetration-scale range (break-eternity;
   [`2026-06-21-break-eternity-representation.md`](./2026-06-21-break-eternity-representation.md)).
   A generic engine should be generic over the numeric type (or commit to the widest), which
   is a constraint on every `Formula`/`Cost`/`Effect` signature. Cheaper to decide once,
   up front, than to retrofit.

4. **Effect combination semantics are not free.** "Everything is an effect" (Approach 3) is
   elegant but the *combination rules* are the hard part and are irreducibly part of the
   spec: glyphs of a type combine additively-then-apply; some effects `max` rather than
   multiply; softcaps apply after summation. The uniform representation must carry a
   `combine`/`order` descriptor precise enough to reproduce these — otherwise uniformity is
   an illusion that fails fidelity.

5. **Config location.** Compile-time `const` (fast, type-checked, needs rebuild) vs. external
   files (RON/TOML, hot-reload, designer/Python-authorable, runtime-validated) vs. embedded
   DSL. For *parameter* exploration, external data loaded into a `GameConfig` is the sweet
   spot and dovetails with the existing Python experiment API
   ([`2026-06-24-experiment-architecture.md`](./2026-06-24-experiment-architecture.md)) — a
   sweep is `for cfg in configs: simulate(cfg)`. For *structural* exploration you need the
   DSL, with the fidelity/perf costs above.

---

## 6. Recommendation

Not "build a framework," and not "stay hard-wired forever" — a **staged hybrid**, adopted
only as the scope that justifies it actually lands:

1. **Now / cheap / almost pure upside — do regardless (Approach 1 core):**
   - Hoist every literal out of `data/constants.rs` into a `GameConfig` value the engine
     *reads* (not `const`), so the whole game is a function of config. This alone unlocks the
     bulk of design-space *parameter* exploration and plugs straight into `ad-python`.
   - Introduce the two reusable utilities the feature doc already calls for: a `CostScaling`
     enum and a `Formula`/`EffectFormula` enum-dispatch. Low risk, immediately useful.
   - Prototype the `Condition` enum (§4, unlock/visibility). It is the connective tissue for
     everything later and is worth having even in the current code.

2. **At the Phase 2→3 boundary — introduce the primitives (Approach 2), incrementally:**
   - When the second dimension family (Infinity Dimensions) and the effect count crosses
     ~a few dozen sources, promote `DimensionFamily`, `Purchasable`, `PrestigeLayer`,
     `RuleModifier` to real types and port existing mechanics onto them *one at a time*,
     each guarded by the fidelity suite. The Endgame doc's core finding — parameterize the
     crunch/eternity machinery and the Celestial Expansion is nearly free — is the payoff
     thesis.

3. **For the multiplier pipeline specifically — a data-row effect bus (Approach 3, scoped):**
   - Represent effects as `{source, target, op, value, combine}` rows so achievements,
     upgrades, glyphs, time studies, and challenge rewards unify. **But** keep the fold order
     and combine semantics as explicit tested code, and cache the result to per-target
     scalars so the hot loop never walks the table. This is the concrete form of "features as
     special cases of one mechanism."

4. **Structure stays compiled; only parameters and effect-tables are data (avoid Approach 4
   for the engine).** Reserve the embedded DSL for (a) the **Automator**, which is a
   player-facing scripting feature anyway, and (b) an optional designer sandbox where
   fidelity is explicitly *not* required.

5. **Do not build an ECS (Approach 5).** The entity set is static, small, and deterministic;
   plain typed collections are simpler and better for reproducible fidelity tests.

**What to explicitly *not* do:** generalize ahead of the second instance. The moment to
introduce `DimensionFamily` is when there are two dimension families, not one; the moment to
introduce the effect bus is when a production function has a dozen effect terms, not three.
Premature generalization here would trade the current code's biggest virtue — it reads like
the mechanic and matches the original line-by-line — for a framework validated against a
single instance.

---

## 7. What this does and doesn't buy for design-space exploration

**Buys (with the staged hybrid above):**
- Free parameter sweeps and A/B experiments over costs, multipliers, thresholds, exponents,
  softcap breakpoints — the 80% case — directly from `ad-python`, no recompile.
- Adding a *new instance* of an existing primitive (another dimension family, another
  rebuyable upgrade, another challenge) as data.
- Introspection: "what currently feeds this multiplier / this threshold," which serves both
  fidelity debugging and a future design UI.

**Doesn't buy (without the DSL, which we deliberately decline for the engine):**
- Inventing a mechanic whose *shape* isn't already an enumerated variant — a production
  topology beyond {downward, self-replicating, ring}, a cost law beyond the known scalings,
  a new combination rule. Choosing *among* enumerated variants is config; adding a variant
  bottoms out in Rust.
- Any exploration that changes the *combination semantics* of the effect pipeline, since
  those are (correctly) pinned to code to protect fidelity.

This matches the reality of the game: the three largest "design-space explorations" ever
performed on Antimatter Dimensions — the Endgame, Redemption, and Vis mods, ~+42k JS lines
between them by different authors — added **zero new paradigms** (§2.1). An engine sized to
the ten primitives, with production topology as a config axis (§2.2), would have absorbed
almost all of that as configuration. That three independent expansions all fit the same mold
is the single best piece of evidence that the config-driven direction is sound *and* that
its ceiling (structure in code, parameters in data) is set in the right place.

---

## 8. Bottom line

- Antimatter Dimensions plus its three major mods is expressible as instances of **≈10
  primitives**; the original proves it by construction, and Endgame, Redemption, and Vis
  independently prove the primitive set is stable under ~+42k lines of expansion by different
  authors (§2.1). The only structural amendment the mods force is a **production-topology
  config axis** on one primitive (§2.2) — not a new primitive.
- "Config-driven" is a spectrum. The valuable, low-risk band is **parameters + effect tables
  as data, structure and combination semantics as compiled Rust**, with a scoped effect bus
  unifying the multiplier pipeline. A full formula-DSL belongs to the Automator and to an
  explicitly non-fidelity sandbox, not to the canonical engine.
- The dominant constraint is **fidelity**: the abstractions can be no cleaner than the
  original's special cases, and the existing fidelity + Python-experiment infrastructure is
  the exact harness that makes generalizing safe and makes exploration cheap.
- For the game as it stands today (pre-Infinity, hard-wired), **none of this is worth doing
  yet** — the recommendation is a *conditional* one, triggered at the Phase 2→3 boundary and
  adopted one mechanic at a time behind the fidelity suite. The highest-value move available
  *right now* is the smallest one: make the game a pure function of a `GameConfig` so that
  every number is sweepable.

---

*Document generated: 2026-07-01. Blue-sky design study; no implementation planned. Builds on
[`2026-06-19-architecture.md`](./2026-06-19-architecture.md) (§4 enum-dispatch, §5.3
multiplier pipeline, §5.5 modifiers), [`2026-06-23-feature-decomposition.md`](./2026-06-23-feature-decomposition.md)
(scope + cross-cutting concerns), the three mod analyses that each independently reach the
"no new paradigms" finding — [`2026-06-28-endgame-1.0-analysis.md`](./2026-06-28-endgame-1.0-analysis.md),
[`2026-06-25-redemption-analysis.md`](./2026-06-25-redemption-analysis.md),
[`2026-06-24-vis-analysis.md`](./2026-06-24-vis-analysis.md) (the last the source of the
production-topology refinement) — and [`2026-06-27-simulation-architecture.md`](./2026-06-27-simulation-architecture.md)
(Action IR / Automator).*
