---
status: Implemented
---

# Progressive UI Reveal, First-Time Confirmations & Tutorial Highlights

Date: 2026-06-30

**Status: implemented (2026-06-30).** All three features landed. Resolved open
questions: tutorial advance stays tick-driven; the snapshot exposes raw
`tutorial_state` / `tutorial_active`; `dim_available_for_purchase` was fixed
(can no longer buy a dimension before owning the one below it); sacrifice
visibility now reads `achievement_unlocked(18)` (no `bought_8th_dimension`
field — the persistence table below is superseded on that row); and
`emphasizeH2P` (the pulsing How-To-Play link) is included.

Two post-doc implementation notes:

- **`emphasizeH2P` is currently suppressed.** It overlays the always-visible dev
  speed/offline/pause controls, so the frontend gates it behind a UI flag
  (`ui.h2pEmphasisShown`, pre-set so a new game never shows it). When those dev
  controls become a toggleable option, drive the flag from their visibility so
  the emphasis returns only when they are hidden.
- **New save fields are required, not defaulted.** Saves are short-lived during
  active development, so the read path (`save/dto.rs`) treats `tutorialState`,
  `tutorialActive`, and `options.confirmations` (and its four flags) as required
  — a missing one fails the load (surfacing a format change) rather than
  silently defaulting. Tutorial Feature 1 still adds no persisted state of its
  own (sacrifice visibility rides on the existing `achievementBits`).

## Goal

Three early-game presentation features from the original game, all cosmetic but
all backed by **persisted save state**. They share a theme: easing a new player
into mechanics by hiding what isn't relevant yet, explaining actions the first
time, and drawing the eye to what just became available.

1. **Progressive reveal.** At the start, most UI is hidden and appears as it
   becomes reachable: Antimatter Dimension rows that aren't purchasable yet, the
   Tickspeed row, and the Dimensional Sacrifice button.
2. **First-time / disable-able confirmation popups.** Boost, Galaxy, Sacrifice
   and Big Crunch show an explanatory modal before acting, each with a "Don't
   show this message again" checkbox that flips a persisted option.
3. **Tutorial highlight.** A pulsing gold glow plus a yellow exclamation mark
   (`!`) draws attention to the one element the player should interact with next,
   advancing through a small state machine.

This doc records how the original implements each, what our codebase already has,
and a phased plan. The three are independent and can land in any order; they are
grouped because they all touch the same early-game components and all add small
amounts of persisted state.

## Scope

Pre-Infinity only, matching the rest of the engine today. Every original
condition below is quoted in full, but the parts that reference Eternity,
Reality, Pelle, Continuum, challenges, etc. are simply absent for us and collapse
to their pre-Infinity value (noted inline). Big Crunch is in scope only as the
*first* infinity (the explanatory modal); see Feature 2.

---

## Feature 1 — Progressive reveal

### How the original does it

There is **no dedicated "reveal" state** — visibility is derived live from game
state, recomputed each frame in each component's `update()`.

**Antimatter Dimension rows** (`ClassicAntimatterDimensionRow.vue` /
`ModernAntimatterDimensionRow.vue`):

```js
showRow = this.isShown || this.isUnlocked || this.amount.gt(0);
// isUnlocked = dimension.isAvailableForPurchase
// isShown    = (DimBoost.totalBoosts > 0 && DimBoost.totalBoosts + 3 >= tier)
//              || PlayerProgress.infinityUnlocked();   // pre-Infinity: false
```

with `isAvailableForPurchase` (`antimatter-dimension.js`):

```js
get isAvailableForPurchase() {
  if (this.tier > DimBoost.totalBoosts + 4) return false;          // unlock band
  const hasPrevTier = this.tier === 1
    || AntimatterDimension(this.tier - 1).totalAmount.gt(0);       // own previous
  if (!hasPrevTier) return false;
  return this.tier < 7 || !NormalChallenge(10).isRunning;          // pre-Inf: true
}
```

The row is hidden with `v-show="showRow"` (i.e. `display:none`, it collapses).
Net early-game behaviour:

- **Very start (0 boosts):** only the **1st** AD row is visible. The 2nd row
  appears once you own ≥1 of the 1st (`hasPrevTier`), the 3rd once you own the
  2nd, and so on — rows unfold one at a time as you buy.
- The `isShown` lookahead (`totalBoosts + 3 >= tier`) only kicks in **after the
  first Dimension Boost**, revealing the next couple of rows slightly *before*
  they're purchasable so the player can see what a boost will unlock.
- A row that is shown but not yet purchasable (e.g. revealed by `isShown`) gets
  the dimmed `c-dim-row--not-reached` class.

**Tickspeed row** (`TickspeedRow.vue`):

```js
isVisible = Tickspeed.isUnlocked || EternityChallenge(9).isRunning; // pre-Inf: isUnlocked
// Tickspeed.isUnlocked  <=>  AntimatterDimension(2).bought > 0
```

Hidden via `visibility: hidden` (`.l-tickspeed-container--hidden`) — note this
**reserves the space** (unlike the dimension rows' `display:none`), so the layout
doesn't jump when tickspeed appears after the first 2nd-dimension purchase.

**Sacrifice button** (`AntimatterDimensionsTab.vue`, gated on
`Sacrifice.isVisible`). Crucially, **visibility is not boost count** — it is a
persisted **achievement**:

```js
static get isVisible() {                                      // sacrifice.js
  return Achievement(18).isUnlocked || PlayerProgress.realityUnlocked();
}
static get canSacrifice() {                                   // the *enable* gate
  return DimBoost.purchasedBoosts > 4 && this.nextBoost.gt(1)
    && AntimatterDimension(8).totalAmount.gt(0) && /* …challenge guards… */;
}
```

Achievement 18 ("90 degrees to infinity") unlocks the first time you **buy an 8th
Antimatter Dimension** and, like all achievements, stays unlocked forever
(including across a Big Crunch). So the button **appears** as soon as an 8th AD is
bought (which needs 4 boosts) — initially **disabled** with the
`disabledCondition` text "Requires 5 Dimension Boosts" — and **enables** at the
5th boost. `purchasedBoosts > 4` is the *enable* threshold, not the *reveal* one.

Achievements persist in `player.achievementBits` (17 ints, one bitmask per row;
`isUnlocked = achievementBits[row-1] & (1 << (column-1))`, with `row =
floor(id/10)`, `column = id%10`). Achievement 18 → row 1, column 8 →
**`achievementBits[0]` bit `1<<7` (128)**.

### What our codebase already has

- `GameState::unlocked_dimensions()` returns `4 + min(dim_boosts, 4)` and
  `is_dimension_unlocked(tier) = tier < unlocked_dimensions()`. **This diverges
  from the original**: it reports tiers 1–4 unlocked from the very start, ignoring
  the `hasPrevTier` rule. So today `DimensionRow.vue` renders all 8 rows always
  (`v-for="tier in 8"`), showing "Locked" for not-yet-unlocked tiers rather than
  hiding them.
- `GameState::tickspeed_unlocked()` already mirrors `dimensions[1].bought > 0`.
- `GameState::sacrifice_unlocked()` returns `dim_boosts >= 5`, and the tab gates
  the button with `v-show="s.sacrifice_unlocked"`. **This is wrong**: it models the
  original's *enable* gate (`canSacrifice`), not its *visibility* gate
  (`Achievement(18).isUnlocked`). Consequences: (a) the button appears one boost
  too late (at 5 boosts instead of when the 8th AD is first bought, at 4); (b) it
  would vanish after a Big Crunch resets boosts below 5, whereas the original
  keeps it; (c) the achievement state is **not persisted** — see below.
- **We do not model achievements at all.** `encode.rs` overlays only modelled
  fields onto a fresh-start template, so `achievementBits` is exported as
  all-zeros. A save we write therefore has achievement 18 locked, and the
  original game **hides the sacrifice button** even for a player who has
  sacrificed. (Observed in testing — this is what surfaced the bug.) This is a
  general save-fidelity gap: any achievement-gated visibility/effect is lost on
  round-trip. **(No longer true as of 2026-06-30 — achievements are now modelled
  and round-tripped; see `docs/design/2026-06-30-achievements.md`. Achievement 18
  unlocks and persists, so the remaining step for this feature is to point
  sacrifice visibility at `achievement_unlocked(18)`.)**
- The snapshot (`ObservedDimensionState`) exposes per-tier `unlocked`; the tab
  snapshot exposes `unlocked_dimensions`, `sacrifice_unlocked`.

### Design

Move the visibility predicates into the engine (they are pure functions of game
state, deterministic, and one of them — purchasability — already gates buying),
and expose two booleans per dimension plus the tickspeed flag in the snapshot.

1. **Fix the unlock predicate to match the original** (this is both a reveal fix
   and a real purchase-gating fix — today the engine lets you buy the 2nd
   dimension before owning the 1st). Add to `state.rs`:

   ```rust
   /// `AntimatterDimension.isAvailableForPurchase`, pre-Infinity.
   pub fn dim_available_for_purchase(&self, tier: usize) -> bool {
       if tier > (self.dim_boosts as usize) + 4 { return false; } // 1-indexed tier
       tier == 1 || self.dimensions[tier - 2].amount.gt(&Decimal::ZERO)
   }

   /// `showRow` minus the per-component `amount > 0` term (the view can add it).
   pub fn dim_is_shown(&self, tier: usize) -> bool {
       let lookahead = self.dim_boosts > 0 && (self.dim_boosts as usize) + 3 >= tier;
       lookahead || self.dim_available_for_purchase(tier)
   }
   ```

   Route the existing purchase guard through `dim_available_for_purchase` so the
   engine and UI agree (the original buys through the same predicate).

2. **Snapshot.** Replace/augment `ObservedDimensionState::unlocked` with
   `available_for_purchase` and `shown` (the latter `dim_is_shown(tier) ||
   amount > 0`). Keep a single `unlocked`-style flag for the dimmed
   `not-reached` styling = "shown but not purchasable".

3. **`DimensionRow.vue`.** Wrap the row in `v-show="dim.shown"`; drive
   `c-dim-row--not-reached` off `!dim.available_for_purchase`; gate the buy
   button on `available_for_purchase`. This replaces today's always-8-rows +
   "Locked" placeholder with the original's unfolding behaviour.

4. **Tickspeed.** Add `tickspeed_unlocked` to the tab snapshot; in
   `TickspeedRow.vue` mirror the original's `visibility:hidden` (reserve space)
   rather than `v-if`, using the vendored `.l-tickspeed-container--hidden` rule.

5. **Sacrifice.** Split the conflated flag in two:
   - `sacrifice_visible` — "has an 8th AD ever been bought" (i.e. achievement 18).
     Track it as persistent engine state: a `bought_8th_dimension: bool` on
     `GameState`, set `true` whenever `dimensions[7].bought` first goes positive
     (and never cleared, including across a crunch). Drive the button's `v-show`
     off this.
   - `can_sacrifice` — the existing `dim_boosts >= 5 && …` enable check, used for
     the disabled state + the "Requires 5 Dimension Boosts" text (already mostly
     wired in the tab).
   - **Persist it** through the save round-trip: in `encode.rs`, when
     `bought_8th_dimension`, set `player.achievementBits[0] |= 1 << 7`; in
     `dto.rs`, read `bought_8th_dimension` back from that same bit (defaulting
     `false`). This is the minimal faithful fix; a fuller achievement model
     (below) would subsume it.

Feature 1 thus adds **one** persisted field (`bought_8th_dimension`), backed by
achievement bit 18. Everything else (AD rows, tickspeed) is derived from existing
fields.

### Note: a general achievement model

> **Update (2026-06-30): this has since been built** — see
> `docs/design/2026-06-30-achievements.md`. `GameState` now has
> `achievement_bits: [u32; 17]`, round-tripped verbatim through `achievementBits`,
> with `achievement_unlocked(id)` and inline unlocks (including 18 on buying an
> 8th AD). So **drop `bought_8th_dimension`**: Feature 1's sacrifice *visibility*
> should read `self.achievement_unlocked(18)` directly. The achievement milestone
> deliberately left sacrifice gating untouched, so wiring that visibility term is
> the small remaining step that belongs to this feature. The note below is the
> original pre-implementation reasoning, kept for context.

`bought_8th_dimension` is really "achievement 18 unlocked." Several pre-Infinity
achievements gate UI or carry production effects, and all of them currently
vanish on save round-trip for the same reason. Rather than accreting one-off
bits, it is worth modelling achievements minimally — e.g. a `achievement_bits:
[u32; 17]` (or a `HashSet<u16>`) on `GameState`, unlocked by the relevant game
events, round-tripped verbatim through `achievementBits`. The sacrifice fix would
then read `achievement(18)` instead of a bespoke bool. This is larger than the
three features here and is flagged as a **prerequisite/sibling** rather than
folded in; if deferred, the one-off `bought_8th_dimension` is the stopgap.

---

## Feature 2 — First-time / disable-able confirmation modals

### How the original does it

Each manual action checks a per-action boolean in
`player.options.confirmations.*` (all default `true`) and, if set, shows a modal
instead of acting; the modal's "Confirm" button performs the action and a
`ModalConfirmationCheck` checkbox can flip the option off. The flow (e.g.
`dimboost.js`):

```js
export function manualRequestDimensionBoost(bulk) {
  if (!DimBoost.canBeBought) return;
  if (player.options.confirmations.dimensionBoost) {
    Modal.dimensionBoost.show({ bulk });   // modal's Confirm calls requestDimensionBoost
    return;
  }
  requestDimensionBoost(bulk);             // the actual reset
}
```

Same shape for `manualRequestGalaxyReset` (`confirmations.antimatterGalaxy`) and
`sacrificeBtnClick` (`confirmations.sacrifice`). So these are **always-on
confirmations the player can disable**, not strictly first-time — but the *effect*
the user described ("explain it, with a tickbox to hide it") is exactly the
`ModalConfirmationCheck` checkbox, which sets e.g.
`ConfirmationTypes.dimensionBoost.option = false`.

`ModalConfirmationCheck.vue` renders the checkbox + "Don't show this message
again"; toggling it writes the option. `ModalWrapperChoice.vue` is the shared
shell (header slot, body slot, optional `<ModalConfirmationCheck :option>`,
Cancel/Confirm buttons).

**Big Crunch is special** (`big-crunch.js`):

```js
// Show the modal on the first ever infinity (to explain) AND post-break.
if (player.options.confirmations.bigCrunch
    && (!PlayerProgress.infinityUnlocked() || player.break)) {
  Modal.bigCrunch.show();
} else { bigCrunchResetRequest(); }
```

On the **first** infinity, `BigCrunchModal` passes `confirm-option: undefined`
(the explanatory text **cannot** be dismissed via the checkbox — there is nothing
to disable yet), and after confirming it queues a one-off message about
animations. Post-break it passes `confirm-option: 'bigCrunch'` (disable-able).
Pre-Infinity, our case is always the first-infinity explanatory branch.

Modal bodies (examples):
- **DimensionBoost:** "This will reset your Antimatter and Antimatter Dimensions.
  Are you sure?" (`option="dimensionBoost"`).
- **AntimatterGalaxy:** analogous reset warning (`option="antimatterGalaxy"`).
- **Sacrifice:** `SacrificeModal.vue` (existing).
- **BigCrunch:** "Upon Infinity, all Dimensions, Dimension Boosts, and Antimatter
  Galaxies are reset. In return, you gain an Infinity Point (IP)…".

Defaults live in `player.options.confirmations` (`player.js` ~846): the four we
care about — `sacrifice`, `bigCrunch`, `antimatterGalaxy`, `dimensionBoost` — all
`true`.

### What our codebase already has

- A working modal system: `ui.js` (`openModal`, `showModal`, `closeModal`,
  `toggleModal`) and a generic `Modal.vue` shell, with several modals
  (`H2PModal`, `NotationModal`, `OfflineSummaryModal`, …) already wired.
- `Options` (`options.rs`) persists a modelled subset of `player.options`,
  round-tripped through `save/dto.rs` + `save/encode.rs`, surfaced via
  `OptionsView` in `main.rs` and the `game` store. **No `confirmations` yet.**
- Manual actions (`buyDim`, `sacrifice`, boost, galaxy, crunch) are Tauri
  commands invoked directly from the components — there is **no
  "request → maybe-confirm → perform" indirection yet**; the confirm gate is new.

### Design

1. **Engine option.** Add a `Confirmations` sub-struct to `Options`:

   ```rust
   pub struct Confirmations {
       pub dimension_boost: bool,
       pub antimatter_galaxy: bool,
       pub sacrifice: bool,
       pub big_crunch: bool,
   }   // all default true
   ```

   Persist under `player.options.confirmations.{dimensionBoost, antimatterGalaxy,
   sacrifice, bigCrunch}` in `dto.rs`/`encode.rs`; surface in `OptionsView`; add
   a `set_confirmation(kind, enabled)` command + a `game`-store action.

2. **Confirm gate lives in the frontend.** The decision "show modal vs. act" is a
   pure UI concern and the engine action is unchanged, so unlike Feature 1 this
   stays in JS. In each handler (the tab's `sacrifice()`, boost/galaxy/crunch
   click handlers), branch on `snapshot.options.confirmations.<kind>`: if set,
   `ui.showModal('<kind>Confirm')`; else invoke the existing command directly.
   The modal's Confirm button invokes the same command.

3. **Components.** Port `ModalWrapperChoice` + `ModalConfirmationCheck` (small,
   self-contained) and add four bodies (`DimensionBoostModal`,
   `AntimatterGalaxyModal`, `SacrificeModal`, `BigCrunchModal`) reusing the
   vendored `c-modal__confirmation-toggle*` and `c-modal-message*` classes. The
   checkbox calls `setConfirmation(kind, false)`.

4. **Big Crunch nuance.** Pre-Infinity, always show `BigCrunchModal` with the
   first-infinity explanatory copy and **no** disable checkbox
   (`confirm-option: undefined`), matching the original. The `confirmations.big_crunch`
   flag is still persisted now (cheap, and it's what governs the post-break case
   we'll add later); it simply isn't surfaced as a checkbox on the first infinity.

---

## Feature 3 — Tutorial highlight (glow + exclamation)

### How the original does it

A 6-step state machine in `core/tutorial.js` over two persisted fields,
`player.tutorialState` (int, default 0) and `player.tutorialActive` (bool,
default true):

```js
TUTORIAL_STATE = { DIM1:0, DIM2:1, TICKSPEED:2, DIMBOOST:3, GALAXY:4, AUTOMATOR:5 };
// Advance to state N when state N's condition is true:
//  DIM2:      Currency.antimatter.gte(100)
//  TICKSPEED: AntimatterDimension(2).bought > 0
//  DIMBOOST:  AntimatterDimension(4).amount.gte(20)
//  GALAXY:    AntimatterDimension(8).amount.gte(80)
//  AUTOMATOR: Player.automatorUnlocked            // out of scope for us
```

- `Tutorial.isActive(state)` = `fullGameCompletions === 0 && view.tutorialState
  === state && view.tutorialActive`. (`fullGameCompletions` is always 0 for us
  pre-Infinity, so it's a no-op.)
- `tutorialLoop()` (called from the game loop) checks the **next** state's
  condition and `moveOn()`s if met — this is how DIM1→DIM2→TICKSPEED advance
  purely from progress.
- `turnOffEffect(fromState)` clears `tutorialActive` and is called explicitly
  from the **boost** and **galaxy** actions (`requestDimensionBoost`,
  `requestGalaxyReset`) — those two advance by *doing the action*, not by a
  passive condition; it then re-runs `tutorialLoop()` to chain into the next
  state immediately (handles e.g. buying dim 2 + tickspeed in one tick).

Each highlighted component reads `Tutorial.isActive(MY_STATE)` into `hasTutorial`
and renders two things on its primary button:

```html
<div :class="{ 'tutorial--glow': isAffordable && hasTutorial }">…</div>
<div v-if="hasTutorial" class="fas fa-circle-exclamation l-notification-icon" />
```

So: the **gold glow** (`tutorial--glow`, a pulsing `::after` overlay) appears only
when the action is *affordable*; the **exclamation icon**
(`l-notification-icon`, FontAwesome `fa-circle-exclamation`, coloured by
`--color-notification` = yellow, with a glow keyframe) appears whenever the
element is the current tutorial target. Mapping of states → components:

| State | Component | Glow when |
|-------|-----------|-----------|
| DIM1 | 1st AD row buy button | affordable |
| DIM2 | 2nd AD row buy button | affordable |
| TICKSPEED | Tickspeed buy button | affordable |
| DIMBOOST | Dimension Boost button | buyable |
| GALAXY | Antimatter Galaxy button | buyable |

`emphasizeH2P()` additionally pulses the How-To-Play link until the first boost;
minor, can be a later add.

### What our codebase already has

- Both CSS rules are **already vendored** in
  `frontend/public/stylesheets/styles.css`: `.tutorial--glow` (+ `a-opacity`
  keyframe) at ~9114 and `.l-notification-icon` (+ `a-notification-glow`) at
  ~5760, with `--color-notification` resolving to yellow in the active themes.
  So no CSS work — just apply the classes.
- FontAwesome is loaded (used elsewhere).
- **No tutorial state** in `GameState` and none in the save round-trip.

### Design

The state machine is small, deterministic, reads game state, and is mutated by
engine actions (boost/galaxy) — so it belongs in the **engine**, consistent with
how the original places it in `player.*` and how we already keep `Options` in
`GameState`. The frontend only reads the resulting `(tutorial_state,
tutorial_active)` and renders glow + icon.

1. **Engine.** Add `tutorial_state: u8` (default 0) and `tutorial_active: bool`
   (default true) to `GameState`. Port `tutorial.js`:
   - `tutorial_loop(&mut self)` — called at the end of `tick()`; advances when
     the next state's condition holds. Conditions read existing state
     (`antimatter`, `dimensions[1].bought`, `dimensions[3].amount`,
     `dimensions[7].amount`).
   - `tutorial_turn_off(&mut self, from: u8)` — called from the boost and galaxy
     actions (in `dimboost`/`galaxy` modules), then re-runs `tutorial_loop`.
   - `tutorial_active_at(&self, state: u8) -> bool` — the `isActive` helper for
     the snapshot (drop the `fullGameCompletions` term; always 0).
   Stop the machine at the GALAXY state for now (AUTOMATOR is out of scope) — the
   `next condition` for AUTOMATOR simply never fires.

2. **Snapshot.** Add `tutorial_state` + `tutorial_active` (or pre-computed
   per-target `has_tutorial` booleans) to the relevant view structs. Simplest:
   expose the two raw fields once at the top level and let each component compare.

3. **Components.** In `DimensionRow.vue` (tiers 1 & 2), `TickspeedRow.vue`,
   `DimBoostRow.vue`, `GalaxyRow.vue`: compute `hasTutorial` from the snapshot,
   add the `tutorial--glow` class when `hasTutorial && affordable`, and render the
   `fa-circle-exclamation l-notification-icon` div when `hasTutorial`. Mirror the
   original's markup exactly (the glow goes on an inner wrapper so the `::after`
   overlay sizes to the button).

4. **Persistence.** Save `player.tutorialState` / `player.tutorialActive` in
   `dto.rs` + `encode.rs` (defaults 0 / true if absent, so existing saves and
   fresh games behave). These live at the `player` root, **not** under `options`.

---

## Persistence summary

New persisted state, all defaulting so old saves and fresh games are valid:

| Field | Save path | Default | Feature |
|-------|-----------|---------|---------|
| `bought_8th_dimension` | `player.achievementBits[0]` bit `1<<7` | false | 1 |
| `tutorial_state` | `player.tutorialState` | 0 | 3 |
| `tutorial_active` | `player.tutorialActive` | true | 3 |
| `confirmations.dimension_boost` | `player.options.confirmations.dimensionBoost` | true | 2 |
| `confirmations.antimatter_galaxy` | `player.options.confirmations.antimatterGalaxy` | true | 2 |
| `confirmations.sacrifice` | `player.options.confirmations.sacrifice` | true | 2 |
| `confirmations.big_crunch` | `player.options.confirmations.bigCrunch` | true | 2 |

Feature 1 adds **no** persisted state. All six round-trip through
`dto.rs`/`encode.rs` and (for confirmations) `OptionsView`, exactly like the
existing option fields.

## Implementation plan

Ordered cheapest-first; the three features are independent.

### Phase 1 — Feature 1 (reveal), no save changes

- `state.rs`: `dim_available_for_purchase`, `dim_is_shown`; route the purchase
  guard through the former (fixes the 2nd-dim-before-1st divergence). Tests: only
  1st row shown at start; nth row available iff (n-1)th owned; `isShown`
  lookahead after first boost.
- Snapshot: per-tier `available_for_purchase` + `shown`; tab `tickspeed_unlocked`.
- `DimensionRow.vue`: `v-show="shown"`, `not-reached` off `!available_for_purchase`.
- `TickspeedRow.vue`: `visibility:hidden` when not unlocked.
- Sacrifice: add `bought_8th_dimension` to `GameState` (set when `dimensions[7].bought`
  first > 0, never cleared); round-trip it via `achievementBits[0]` bit `1<<7` in
  `encode.rs`/`dto.rs`; drive the button `v-show` off it, keeping `can_sacrifice`
  for the enabled state. Test the round-trip and the "stays visible after a
  boost-resetting crunch" property. (Or land the general achievement model first.)

### Phase 2 — Feature 3 (tutorial highlight)

- `state.rs`/`tick.rs`: `tutorial_state`, `tutorial_active`, `tutorial_loop`
  (end of tick), `tutorial_turn_off` (from boost/galaxy actions),
  `tutorial_active_at`. Tests: DIM1→DIM2 at 100 AM; TICKSPEED at dim2 bought;
  turn-off-on-boost then chain; persistence round-trip.
- `dto.rs`/`encode.rs`: `player.tutorialState` / `tutorialActive` (defaulted).
- Snapshot + the five components: glow + `l-notification-icon` (CSS already
  vendored).

### Phase 3 — Feature 2 (confirmations)

- `options.rs`: `Confirmations` sub-struct (+ setter); `dto.rs`/`encode.rs`
  under `player.options.confirmations`; `OptionsView` + `set_confirmation`
  command + store action.
- Port `ModalWrapperChoice` + `ModalConfirmationCheck`; add the four modal
  bodies; branch the boost/galaxy/sacrifice/crunch handlers on the flag.
- Big Crunch: first-infinity explanatory copy, no disable checkbox.

## Open questions

1. **Achievements: minimal substrate first, or stopgap?** *(Resolved: built the
   substrate — option (a).)* The full achievement model (substrate + tab +
   rewards) landed; see `docs/design/2026-06-30-achievements.md`. So Feature 1
   drops `bought_8th_dimension` and reads `achievement_unlocked(18)` directly.
   Original framing kept below for context. Feature 1's sacrifice *visibility*
   needs achievement 18, which is persisted state we don't model. Two paths: (a)
   build a minimal **achievement-bit substrate** first — `achievement_bits` on
   `GameState`, event-driven `unlock(id)`, `achievementBits` round-tripped in
   `encode.rs`/`dto.rs` — and have sacrifice read `achievement(18)`; or (b) ship a
   one-off `bought_8th_dimension` bool now and swap later. (a) also closes the
   broader gap that *all* achievement-gated state is currently lost on save
   round-trip. Recommend (a); it is small (no tab/rewards) and is the correct
   foundation. The achievements **tab + rewards/effects** are deferred regardless.
   Features 2 and 3 are unaffected either way.
2. **Tutorial advance timing.** `tutorial_loop` runs at end of `tick()`, but buys
   are Tauri commands that return a snapshot immediately. So e.g. after buying the
   2nd dimension, the TICKSPEED glow appears on the *next* tick (~≤ update-rate ms
   later), not in the snapshot the buy command returns. This matches the original
   (its `tutorialLoop` is also game-loop-driven) and the lag is imperceptible —
   but if we want it instant, call `tutorial_loop` at the end of the relevant
   action handlers too. Recommend leaving it tick-driven.
3. **Snapshot shape for the tutorial flag** — expose raw `(tutorial_state,
   tutorial_active)` once at the top level (components compare), or pre-compute a
   `has_tutorial` per target? Raw is less code and matches the original's
   `Tutorial.isActive(MY_STATE)` call site; leaning raw.
4. **Should fixing `dim_available_for_purchase` (Phase 1) be split out** as its
   own engine-fidelity change, independent of the UI reveal? It changes buy
   behaviour (can no longer buy dim 2 before dim 1), so it's arguably a bugfix in
   its own right. Recommend keeping it in Phase 1 but calling it out in the commit.
5. **`emphasizeH2P`** (pulsing the How-To-Play link until first boost) — include
   in Phase 2 or defer? It's one extra boolean; minor either way.
