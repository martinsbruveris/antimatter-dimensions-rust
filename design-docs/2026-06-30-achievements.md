# Normal Achievements: state, unlocks, effects & tab

Date: 2026-06-30

**Status: implemented** (rows 1–2, minus the News achievement 22; all five
phases landed). Later rows and secret achievements remain deferred.

## Goal

Model the game's **normal achievements** as persistent, effect-bearing state, and
render the Achievements tab from real unlock data. This doc covers the first
milestone:

1. The visual achievement tab, driven by engine state.
2. Persisting achievement state in save files.
3. Achieving the first two rows of achievements (ids 11–18 and 21–28) from their
   conditions — **except** the News achievement (id 22).
4. Achievement **rewards** (per-achievement effects + the global achievement
   power multiplier).

Secret achievements are out of scope. This work is the
"general achievement model" flagged as a prerequisite in
`2026-06-30-ui-reveal-and-tutorial.md` (§"Note: a general achievement model" and
Open Question 1): it subsumes that doc's one-off `bought_8th_dimension` stopgap —
sacrifice visibility becomes "achievement 18 unlocked."

## Scope

Pre-Infinity only, matching the rest of the engine. All of rows 1–2 (minus 22)
are reachable pre-Infinity: id 24's threshold (1e80) sits below the Big Crunch
threshold (1e308); id 21 needs the crunch, which exists. None of the in-scope
conditions reference Eternity, Reality, Pelle, challenges, or Continuum except in
flavour text. Later rows (and secret achievements) are explicitly deferred.

---

## How the original implements achievements

### Data model

Each achievement is a config object in
`secret-formula/achievements/normal-achievements.js`: `id`, `name`,
`description`, `checkEvent`, `checkRequirement()`, and optionally `reward` (a
display string) plus `effect` (the actual number). The `id` encodes grid
position:

```
row = floor(id / 10)      column = id % 10
```

so 11–18 is row 1, 21–28 is row 2.

### State & persistence

Unlock state is a bitmask array `player.achievementBits` — **17 ints, one per
row** (confirmed in `save/default_player.json`). Secret achievements use a
separate `secretAchievementBits` array.

```js
isUnlocked = (player.achievementBits[row - 1] & (1 << (column - 1))) !== 0
```

So achievement 18 → row 1, column 8 → `achievementBits[0]` bit `1 << 7` (128).
Bits persist forever, including across a Big Crunch.

### Unlock mechanism (`achievements/normal-achievement.js`)

Two paths:

- **Direct unlock.** Row 1 ("buy an Nth dimension") is unlocked imperatively:
  `onBuyDimension(tier)` calls `Achievement(10 + tier).unlock()` with no condition
  evaluated.
- **Event + requirement.** Others register on a `checkEvent` (`BIG_CRUNCH_BEFORE`,
  `GALAXY_RESET_AFTER`, `GAME_TICK_AFTER`, …); when it fires, `tryUnlock()` runs
  `checkRequirement()` and unlocks if true (and not already unlocked).

The event-bus part is the only piece that **does not map onto our codebase — and
doesn't need to.** We have a centralized `apply_action` seam (`action.rs`) plus
`tick.rs`. The original's "event" is just "we reached this point in this action,"
so we call the relevant checks inline at those points. No EventHub.

### Effects — two kinds

1. **Per-achievement effects**, applied via `timesEffectsOf(Achievement(N), …)`
   in `dimensions/antimatter-dimension.js`. In our scope only three carry an
   effect:

   | id | Effect | Where consumed |
   |----|--------|----------------|
   | 21 | Start with 100 antimatter | `currency.js` `startingValue` |
   | 23 | 8th AD ×1.1 | tier-8 multiplier |
   | 28 | 1st AD ×1.1 | tier-1 multiplier |

   Rows 1 (11–18) and ids 24, 25, 26, 27 have **no** per-achievement effect.

2. **Global achievement power** (`normal-achievement.js`, the `_power` lazy),
   multiplied into **every** Antimatter Dimension (`antimatter-dimension.js:10`):

   ```js
   power = 1.25 ^ (completed rows) * 1.03 ^ (total unlocked achievements)
   ```

   (The full formula raises this to a glyph/Ra exponent that is **1 pre-Reality**,
   so it collapses to the above for us.) This is the dominant reward and it is
   **implicit** — even row 1, which has no per-achievement reward, contributes
   `1.03×` per unlock and `1.25×` once the row is complete.

`isEffectActive = isUnlocked && !isDisabled`; the only `isDisabled` source is
Pelle, out of scope, so for us **`isEffectActive == isUnlocked`.**

### Tab (`tabs/normal-achievements/NormalAchievement.vue`)

A 104×104px sprite sheet (`public/images/normal achievements.png`) positioned by
`-(col-1)*104px -(row-1)*104px`. Locked tiles get `o-achievement--locked` (grey,
no sprite); a `fa-star` badge marks reward achievements; the tooltip shows
name / description / reward.

---

## What our codebase had (starting point, before this work)

This section describes the pre-implementation state the design started from.

- **No achievement state at all.** `GameState` had no achievement field;
  `save/encode.rs` overlaid only modelled fields onto a fresh-start template, so
  `achievementBits` round-tripped as all-zeros. Any achievement-gated visibility or
  effect was lost.
- **The tab was scaffolded but inert.** `frontend/components/tabs/
  NormalAchievementsTab.vue` + `frontend/data/achievements.js` existed and rendered
  every tile `o-achievement--locked`, with no sprite and no engine link. The
  extracted data was lossy (id 22 description blank, id 32 description the literal
  `"Achievement32"`) — re-extracted cleanly when wiring real data.
- **Clean inline unlock points** in `dimensions.rs` (`buy_one_dimension`),
  `galaxy.rs`, `dim boost`, `crunch.rs` (`big_crunch`), and `tick.rs`.
- **`dimension_multiplier(tier)`** (`dimensions.rs`, 0-indexed tier) is the single
  place per-dimension multipliers and the global power factor in.
- **`big_crunch`** (`crunch.rs`) resets `antimatter = INITIAL_ANTIMATTER` — the
  hook point for achievement 21's starting-antimatter effect.
- **`ObservedState`** (`observed.rs`) is the snapshot; it already exposes
  `sacrifice_unlocked` and `unlocked_dimensions`.

---

## Design

### 1. State (`ad-core`)

Add to `GameState`:

```rust
/// Mirrors `player.achievementBits`: 17 rows, one bitmask each.
/// `is_unlocked(id) = bits[id/10 - 1] & (1 << (id%10 - 1)) != 0`.
pub achievement_bits: [u32; 17],
```

with helpers (a dedicated `achievements.rs` module):

```rust
pub fn achievement_unlocked(&self, id: u16) -> bool;
fn unlock_achievement(&mut self, id: u16);                 // idempotent set
```

There is deliberately **no** `try_unlock(id, cond)` wrapper. The original's
`tryUnlock` guards an event dispatch (`if !isUnlocked && checkRequirement`); we
dispatch no events and `unlock_achievement` is idempotent, so a wrapper would
collapse to `if cond { unlock }` while splitting one logical condition across a
guard and an argument. Call sites just write the `if` directly.

`u32` per row is deliberate (a row is 8 bits, but `achievementBits` ints are
unbounded in JS and this keeps the bit math obvious). Default is all-zeros via
`Default`/`GameState::new`.

### 2. Unlock conditions (inline at the seam, no event bus)

Port each `checkEvent`/`checkRequirement` to the equivalent action point. Ids in
scope (22 excluded):

| id | Condition | Hook |
|----|-----------|------|
| 11–18 | buy an Nth AD | end of `buy_one_dimension(tier)`: `unlock(11 + tier)` |
| 21 | go Infinite | `big_crunch`, **before** the reset: `unlock(21)` |
| 23 | exactly 99 eighth ADs | after buying the 8th AD: `if amt == 99 { unlock(23) }` |
| 24 | antimatter ≥ 1e80 | end of `tick()`: `if exponent ≥ 80 { unlock(24) }` |
| 25 | ≥ 10 Dimension Boosts | after dim boost: `if purchased ≥ 10 { unlock(25) }` |
| 26 | buy an Antimatter Galaxy | `buy_galaxy`, before reset: `unlock(26)` |
| 27 | ≥ 2 Antimatter Galaxies | after `buy_galaxy`: `if galaxies ≥ 2 { unlock(27) }` |
| 28 | buy a 1st AD while holding ≥ 1e150 | end of tier-1 buy: `if exp ≥ 150 { unlock(28) }` |

Notes:
- 23 is an **exact-equality** check — only re-test it when the 8th-dimension
  amount changes (i.e. on buying the 8th dimension). **Do not** put it on the tick
  loop. The original guards it the same way.
- 21 and 26 fire *before* their reset (matching `*_BEFORE` events), so the
  pre-reset state (e.g. "you did infinity / bought a galaxy") is what unlocks them.

### 3. Effects

- **Global power** in `dimension_multiplier`:

  ```rust
  mult *= self.achievement_power();   // 1.25^completed_rows * 1.03^unlocked_count
  ```

  `achievement_power()` counts complete rows and total unlocked from
  `achievement_bits`. Pre-Reality the exponent is 1, so no glyph/Ra terms.

- **Per-achievement effects** in `dimension_multiplier`:
  - tier 0 (1st AD): `if achievement_unlocked(28) { mult *= 1.1 }`
  - tier 7 (8th AD): `if achievement_unlocked(23) { mult *= 1.1 }`

- **Achievement 21** in `big_crunch`: set the post-reset antimatter to
  `max(INITIAL_ANTIMATTER, 100)` when 21 is unlocked (mirrors `startingValue`'s
  `Effects.max(10, Achievement(21), …)`).

### 4. Persistence (`save/`)

- `encode.rs`: write `achievement_bits` verbatim into the `achievementBits`
  array (replaces today's all-zeros). This automatically carries achievement 18,
  so a save we write keeps the sacrifice button visible.
- `dto.rs`: read `achievementBits` into `achievement_bits`. Like the other
  modelled fields it is **required** (no serde default — a missing one fails the
  load, surfacing a format change) and its length is validated to be exactly 17
  (`SaveError::UnexpectedArrayLength`, mirroring the dimensions array). Every real
  AD save always has it, so this is strict by design. (`GameState`'s *own* serde
  derive does carry `#[serde(default)]` so an internally-serialized state without
  the field still loads as all-zeros — that is the internal-state path, not the
  save-file path.)

### 5. Snapshot (`observed.rs`)

The raw `[u32; 17]` bitmask stays in `GameState` (it must serialize that way), but
the presentation layer gets a semantically meaningful shape instead of the bits.
Expose a sorted list of unlocked ids plus the derived power, built from the
bitmask:

```rust
pub unlocked_achievements: Vec<u16>,   // sorted; frontend builds a Set
pub achievement_power: Decimal,        // 1.25^rows * 1.03^count, for the tab's boost display
```

Both fields are added to **`ObservedState`** (the `ad-sim`/Python snapshot) and to
the GUI's own **`GameView`** in `ad-gui/src/main.rs`, where `achievement_power` is
serialized through the frontend's `Num { m, e }` number shape like every other
displayed `Decimal`.

The tab renders every tile from its own `data/achievements.js` and marks a tile
unlocked iff its id is in `unlocked_achievements` — so unmodelled (later-row)
achievements simply stay locked. The list is also what Phase 5's toast diffs.

The substrate also enables retiring the UI-reveal stopgap: `sacrifice_unlocked`'s
*visibility* term can read `achievement_unlocked(18)` instead of a bespoke
`bought_8th_dimension` bool (the *enable* gate `dim_boosts >= 5` stays). Bits
persist across a crunch, which is what fixes the "sacrifice button vanishes after
a boost-resetting crunch" bug. **This rewire is deferred to the UI-reveal feature
that owns the visible/enable split — this milestone leaves sacrifice gating
untouched (see Phase 2).**

### 6. Tab (`ad-gui` frontend)

- Vendor `normal achievements.png` into `frontend/public/images/` (the dir is
  currently empty).
- Port `NormalAchievement.vue`'s sprite positioning (`background-position:
  -(col-1)*104px -(row-1)*104px`), `o-achievement--locked/--unlocked` classes,
  the `fa-star` reward badge, and the tooltip (name / description / reward).
- Drive each tile's unlocked state from the snapshot `unlocked_achievements` list
  (build a `Set` once per update); show the boost from `achievement_power`.
- Re-extract clean `name`/`description`/`reward` strings into the frontend
  `data/achievements.js` (decided: strings stay frontend-side — see Open
  Question 1); fix the lossy entries (id 22 blank, id 32 = `"Achievement32"`).

## Persistence summary

| Field | Save path | Default |
|-------|-----------|---------|
| `achievement_bits: [u32; 17]` | `player.achievementBits` | all-zeros |

One field, round-tripped verbatim, defaulted so old saves and fresh games are
valid.

## Implementation plan

Ordered foundation-first; the substrate underpins everything that reads it.

### Phase 1 — substrate + persistence

- `state.rs` / `achievements.rs`: `achievement_bits`, `achievement_unlocked`,
  `unlock_achievement`.
- `save/encode.rs` + `save/dto.rs`: round-trip `achievementBits`. Test the
  round-trip and the "bits survive a crunch" property.

### Phase 2 — unlock conditions

- Hook ids 11–18, 21, 23–28 at the seam points in the table above. Tests: each
  condition unlocks at its boundary and not before; 23's exact-99 check; 18
  persists across a crunch.
- Point `sacrifice_unlocked` visibility at `achievement_unlocked(18)`; drop the
  `bought_8th_dimension` plan from the UI-reveal doc. **(Substrate landed —
  `achievement_unlocked(18)` works and persists across a crunch — but the actual
  rewire of the sacrifice button's visible/enable split is left to the UI-reveal
  feature, which owns that split. This milestone does not change sacrifice
  gating.)**

### Phase 3 — effects

- `dimension_multiplier`: global `achievement_power()` first (it is the dominant
  reward), then the 23 / 28 ×1.1 terms.
- `big_crunch`: achievement-21 starting antimatter.
- **Fidelity:** these change AD output, so add a fidelity scenario and note the
  pre/post difference is intentional.

### Phase 4 — tab

- Vendor the sprite; wire `NormalAchievementsTab.vue` to the snapshot
  `unlocked_achievements` list; add star badges + reward tooltips; re-extract the
  display strings.

### Phase 5 — unlock notification toast

Mirror the original's `GameUI.notify.success(\`Achievement: ${name}\`)`. The
infrastructure already exists — `ui.notify(text, type, duration)` in the `ui`
store + `NotificationContainer.vue`, with an `o-notification--success` style — so
this is **frontend-only** and needs no engine changes.

- In the `game` store, diff `unlocked_achievements` against the previous
  snapshot's list on each update; for each newly-added id, look up its name in
  `data/achievements.js` and call `ui.notify(\`Achievement: ${name}\`, 'success')`.
- **Seed, don't spam:** on the first snapshot after load/import, record the
  unlocked set **without** notifying, so importing a save full of achievements
  doesn't fire a wall of toasts. Notify only on transitions after that.

## Other considerations

1. **The global power multiplier changes production now.** Once achievements
   unlock, an existing save's antimatter rate shifts by `1.25^rows × 1.03^count`.
   This is correct (it matches the original) but is a fidelity-relevant change —
   cover it with a test and call it out.
2. **`isEffectActive` simplifies to `isUnlocked`** for us (only Pelle disables,
   out of scope). Don't model disabling.
3. **Deferred, none blocking:** the `hideCompletedAchievementRows` option,
   Reality auto-achieve, and secret achievements. (Unlock toast notifications are
   **in scope** — Phase 5.)
4. **Replaces the UI-reveal stopgap** — do not implement `bought_8th_dimension`;
   read `achievement_unlocked(18)`.

## Open questions

1. **Display strings: engine or frontend?** *(Decided: frontend.)*
   Names/descriptions/reward-text stay in the frontend `data/achievements.js`,
   next to the component that renders them — least plumbing, and the snapshot
   carries only unlock state. The *effect numbers* still live in the engine (they
   drive production); the frontend strings are display-only and may duplicate a
   reward value as text. The cost is a second copy of the names to keep honest;
   the existing `data/achievements.js` already holds them, so re-extract those
   cleanly (id 22 blank, id 32 = `"Achievement32"`) rather than relying on the
   lossy current entries. (`ad-core/src/data/` remains an option if Python ever
   needs the strings, but nothing requires it now.)
2. **Snapshot shape** — *(Decided: list.)* The raw `[u32; 17]` bitmask stays in
   `GameState` for serialization, but the snapshot exposes a semantic
   `unlocked_achievements: Vec<u16>` (+ derived `achievement_power`) rather than
   the bits, so the presentation layer never touches bit math.
3. **Unlock notifications** — *(Decided: implement, Phase 5.)* Frontend-only,
   reusing the existing `ui.notify` toast; diff the `unlocked_achievements` list
   between snapshots and seed on first load to avoid spamming on import.
