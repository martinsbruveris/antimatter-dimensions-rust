# Save / Load: Analysis

Status: in progress (phase 1 of §9 done). Scope: design how `ad-gui` persists a
game and, crucially, how it interoperates with **external Antimatter Dimensions
saves** — both directions — while only a slice of the game is implemented.

See §9 for the phase checklist and §10 for the now-resolved open decisions.

## 1. Goals & constraints

From the requirements:

1. **Load external saves**, ignoring every mechanic we have not implemented. A
   late-game save should load and leave us with a sensible early-game state
   (antimatter, dimensions, tickspeed, boosts, galaxies, options, …).
2. **Our saves load in the original game.** A save we write must be a *complete,
   valid* `player` object the real game accepts, even though we only populate the
   fields we model. The original game fills the rest from its defaults.
3. **No round-trip preservation.** Loading a late-game save in our engine and
   re-saving is allowed to lose everything past the implemented frontier. We do
   not try to carry unknown fields through.

Requirement (3) is the simplifying decision. It means we do **not** need to
preserve-and-reemit the parts of the save we don't understand. But requirement
(2) means a save we *write* still has to be structurally complete — the original
game's loader is not tolerant of a half-populated `player`. The two are
reconciled by **templating**: we overlay our fields onto a baked-in copy of the
original's default `player` object (see §5).

## 2. The original save format

Source: `../antimatter-dimensions/src/core/storage/serializer.js` and
`player.js`.

### 2.1 Encoding pipeline

A save string is the `player` object `JSON.stringify`'d and then pushed through
`GameSaveSerializer.encodeText(json, "savefile")`. Steps, **in encode order**:

1. UTF-8 encode to bytes (`TextEncoder`).
2. **Deflate** with `pako.deflate` — zlib format (zlib header + Adler-32), *not*
   raw deflate, *not* gzip.
3. Bytes → Latin-1 string (`String.fromCharCode` per byte).
4. `btoa` → standard base64.
5. Character-safe cleanup, applied in this order:
   - strip trailing `=`
   - `0` → `0a`
   - `+` → `0b`
   - `/` → `0c`
6. Append ending marker `EndOfSavefile` (only for version `>= AAB`).
7. Prepend `AntimatterDimensionsSavefileFormat` + 3-char version `AAB`.

Decoding reverses this. `decodeText` checks the leading magic string; if absent,
it assumes a pre-Reality save and just `atob`s. We only need to support the
current `AAB` format (and should *emit* `AAB`).

The JSON converter special-cases two things:

- `value === Infinity` → the string `"Infinity"` (and back on decode).
- `Set` → array of its keys (e.g. `infinityUpgrades`, `eternityUpgrades`). We
  don't model any Set-valued field yet, so this only matters when we *template*
  a full save: the defaults contain empty Sets that must serialize as `[]`.

### 2.2 How `Decimal` is stored

break_infinity.js defines `toJSON = toString`, so every `Decimal` is a **JSON
string**, not an object:

- exponent in `(-7, 21)` → a plain JS number string, e.g. `"10"`, `"1000"`.
- otherwise → `"<mantissa>e<+|->",<exponent>`, e.g. `"1e+100"`, `"1.5e+100"`,
  `"5e-8"`. Reload via `new Decimal(str)` (`fromString`).

**This is the single biggest interop gotcha for us.** Our `break_infinity`
`Decimal` currently derives `serde::Serialize/Deserialize` on its `{ m, e }`
fields, so the default serde representation is `{"m":1.0,"e":100}` — completely
incompatible with the save. The save layer must (de)serialize `Decimal` **as a
string** matching JS `toString`/`fromString`. Our `Display` impl already follows
the same branching (plain for `-7 < e < 21`, else `m e±exp`) and `FromStr`
parses `"1e+100"`-style strings, so the building blocks exist — we just need to
route the save layer through them rather than the derived field serde. We are
not aiming for byte-identical mantissa precision (req. 3), only for strings the
original `fromString` accepts and that we can parse back.

### 2.3 The `player` schema (relevant subset)

The default object is `window.player` in `player.js`; `Player.defaultStart` is a
deep clone of it. Fields that map to what we currently model:

| Our `GameState`                | `player` path                              | Type / notes |
|--------------------------------|--------------------------------------------|--------------|
| `antimatter`                   | `antimatter`                               | Decimal string |
| `total_antimatter`             | `records.totalAntimatter`                  | Decimal string |
| `dimensions[t].amount`         | `dimensions.antimatter[t].amount`          | Decimal string |
| `dimensions[t].bought`         | `dimensions.antimatter[t].bought`          | int (single-buy count) |
| — (not modelled)               | `dimensions.antimatter[t].costBumps`       | always 0 pre-challenges; ignore |
| `tickspeed.bought`             | `totalTickBought`                          | int |
| — (derived)                    | `tickspeed.cost`                           | recompute on load; not in save |
| `dim_boosts`                   | `dimensionBoosts`                          | int |
| `galaxies`                     | `galaxies`                                 | int |
| `sacrificed`                   | `sacrificed`                               | Decimal string |
| `infinity_unlocked`            | `break` (and/or `infinities > 0`)          | see §4.3 |
| `options.hotkeys`              | `options.hotkeys`                          | bool |
| `options.update_rate`          | `options.updateRate`                       | int |
| `options.notation`             | `options.notation`                         | string name |
| `options.notation_digits_*`    | `options.notationDigits.{comma,notation}`  | int |
| `autobuyers.enabled`           | `auto.autobuyersOn`                        | bool |
| `autobuyers.dimensions[t].*`   | `auto.antimatterDims.all[t].*`             | see §4.4 |
| `autobuyers.tickspeed.*`       | `auto.tickspeed.*`                         | see §4.4 |

Notes on the trickier mappings:

- **Tickspeed bought.** The original has no stored `bought` count on a tickspeed
  object; it derives the purchased count from `player.totalTickBought` (+
  `chall9TickspeedCostBumps`, which is 0 for us). Cost is computed, never
  stored. So: load `tickspeed.bought ← totalTickBought`, then recompute
  `tickspeed.cost` from our cost formula. There is also `totalTickGained` (free
  tickspeed); we don't model it and can leave it at 0.
- **`costBumps` / `chall9TickspeedCostBumps`.** Only ever incremented inside
  Normal Challenge 9 / Infinity Challenge 5, neither implemented. Always 0 in
  saves we read at our frontier and in saves we write. Safe to drop.
- **AD `bought` semantics match.** Our `bought` is the raw single-purchase count
  and our cost uses `bought / 10`; the original uses
  `floor(bought/10) + costBumps`. Identical with `costBumps = 0`. Direct map.

## 3. The version / migration question

`migrations.patch()` runs every patch whose key `v` satisfies
`player.version < v < maxVersion`. The default `player.version` is **25**, and
the highest migration patch key is also **25**. Therefore a save we emit with
`version: 25` triggers **no** migrations in the original game. Because we build
our save by overlaying onto the baked `defaultStart` template (§5), we inherit
`version: 25` for free and stay migration-free, provided our overlaid data uses
the current schema (it does). The serializer's own version marker is `AAB`.

Risk to validate by test: if the live game's `defaultStart.version` has since
advanced past 25, our template must be refreshed from that build, or the game
will run migrations against current-format data. Pin the template to a known
game version and re-vendor it deliberately.

## 4. Loading an external save (decode direction)

### 4.1 Pipeline

Reverse §2.1:

1. Strip the `AntimatterDimensionsSavefileFormat` prefix; read the 3-char
   version. If the magic string is absent, treat as legacy (`atob` only) — or
   simply reject, since legacy pre-Reality saves are out of scope.
2. Strip the `EndOfSavefile` suffix (version `>= AAB`).
3. Reverse cleanup: `0b`→`+`, `0c`→`/`, `0a`→`0` (order matters — decode `0b`/`0c`
   before `0a`). Re-pad base64 with `=` to a multiple of 4.
4. base64 decode.
5. zlib `inflate`.
6. UTF-8 → JSON string; parse.

Rust crates: `flate2` (zlib `ZlibDecoder`/`ZlibEncoder`, backed by miniz_oxide —
matches pako) and `base64`. Both are small, well-established.

### 4.2 Parsing strategy: partial DTO, not serde-on-GameState

Do **not** put `#[derive(Deserialize)]` straight on `GameState`. Our internal
layout deliberately differs from the `player` schema (e.g. flat `dimensions:
[DimensionTier; 8]` vs nested `dimensions.antimatter[]`, derived tickspeed cost,
different option field names/casing). Instead add a dedicated **save module**
(proposed `ad-core::save`) with serde DTO structs that mirror the `player`
schema for *only the fields we read*, each annotated `#[serde(default)]` so any
missing or extra field is ignored. `serde` on a struct already ignores unknown
keys by default, which is exactly the "ignore unimplemented mechanics" behavior
we want for req. (1). A late-game save deserializes fine; we read the handful of
fields we understand and drop the rest.

The DTO uses a `Decimal`-as-string newtype/helper (a `deserialize_with` that
calls `Decimal::from_str`) so the string format from §2.2 is honored.

`GameState::from_save_dto(dto) -> GameState`:

- copy mapped scalar/Decimal fields;
- rebuild derived state (tickspeed cost, autobuyer timers/intervals) from our own
  constructors so it's internally consistent;
- clamp / validate (e.g. options through the existing `set_*` clamps), so a
  hostile or out-of-range external value can't put us in an invalid state;
- everything we don't model is simply never read.

### 4.3 `infinity_unlocked`

The original gates Infinity UI on `PlayerProgress.infinityUnlocked()`, which is
true once `player.break` is set or any infinity has happened. For our flag, set
`infinity_unlocked = save.break || infinities > 0 || infinityPoints > 0`. Since
we reset everything past our frontier, a late-game save loads as "infinity
unlocked, fresh early-game run," which is the correct behavior for our slice.

### 4.4 Autobuyers

Map `is_active`, `isBought`, `mode` (`AUTOBUYER_MODE.BUY_10`/`BUY_SINGLE`) and
the global `auto.autobuyersOn`. Intervals are derived from upgrades in the
original; we currently use fixed intervals, so rebuild interval/timer from our
constructors rather than trusting the saved interval. The tickspeed autobuyer's
`mode` is locked to single pre-Infinity for us regardless of the saved value.

## 5. Writing a save loadable by the original (encode direction)

This is where req. (2) bites: the produced `player` must be *complete*. Approach:

1. **Vendor `Player.defaultStart` as a baked template** — a JSON file checked in
   under the save module (generated once from the pinned game build, documented
   as such). It already contains every field, every empty `Set`-as-`[]`, the
   correct `version: 25`, the full `options`/`auto`/`records` trees, etc.
2. Parse the template into a `serde_json::Value` (or our own owned JSON value).
3. **Overlay** our modelled fields onto it at the right paths (the inverse of the
   §2.3 table): write Decimals as strings via `Decimal::Display`, ints as
   numbers, set `records.totalAntimatter`, `dimensions.antimatter[t]`,
   `totalTickBought`, `dimensionBoosts`, `galaxies`, `sacrificed`,
   `break`/`infinities` to reflect `infinity_unlocked`, the `options.*` and
   `auto.*` subtrees, and a fresh `lastUpdate`/`records.*Time` (use a caller-
   supplied timestamp; the engine stays deterministic — no `SystemTime` inside
   `ad-core`).
4. `serde_json` serialize with the `Infinity → "Infinity"` and `Set → []`
   conventions already satisfied by the template (we never introduce a raw
   `Infinity`; `Number.MAX_VALUE` best-times in the template stay as their
   numeric literals).
5. Run the §2.1 encode pipeline.

Overlaying onto a template (rather than hand-building the object) is what lets us
satisfy "complete and valid" without modelling the whole game, and it tracks the
original's own `deepmergeAll([{}, player])` philosophy. The cost is we must keep
the template in sync with the targeted game version (§3 risk).

An alternative — emit only our fields and rely on the original's load-time
`deepmergeAll(defaultStart, save)` to fill gaps — is tempting but fragile: the
original merges onto defaults, yet several systems read fields expecting arrays
of a fixed length or interdependent values, and a partial object can desync
`GameCache`. Templating is safer and only marginally larger. Recommend
templating.

## 6. Where it plugs into `ad-gui`

State today is a `Mutex<GameState>` managed by Tauri (`main.rs`), with no
persistence. Proposed additions:

- **Engine layer (`ad-core::save`)**, behind the existing `serde` feature:
  `encode_save(&GameState, now_ms) -> String` and
  `decode_save(&str) -> Result<GameState, SaveError>`. Pure, deterministic, no
  IO, no wall clock (timestamp passed in). This keeps `ad-core` IO-free per the
  architecture principles and makes the codec unit-testable.
- **Tauri commands**: `export_save() -> String`, `import_save(text: String) ->
  Result<GameView, String>`, plus `save_to_disk` / `load_from_disk` and an
  autosave tick. The webview gets export/import text-box modals mirroring the
  original's; `import_save` swaps the `Mutex<GameState>` and returns a fresh
  `GameView`.
- **Persistence target**: a JSON-or-encoded blob in the Tauri app data dir
  (and/or `localStorage`-equivalent). Decide whether our *own* on-disk save uses
  the AD-compatible string (simplest — one format, automatically importable into
  the real game) or a plain internal JSON. Recommend reusing the AD-compatible
  string so there is exactly one serialization path to test.
- **Autosave + "time since last save"** (already on the todo list) and the
  save/load keyboard shortcuts.

## 7. Edge cases & risks

- Decimal-as-string vs the existing `{m,e}` derive — must override in the save
  layer (§2.2). The most likely source of silent breakage.
- base64 cleanup order and re-padding `=` on decode.
- zlib vs raw-deflate vs gzip — must be **zlib** (`flate2::*Zlib*`).
- `Infinity` / `Number.MAX_VALUE` sentinels in the template (best-times arrays):
  leave untouched; never emit a literal JS `Infinity` ourselves.
- Sets serialized as arrays: only via the template's empty `[]`s for now.
- Template/version drift (§3): pin and document the source game build.
- Out-of-range or malicious external values: validate/clamp on import.
- Tickspeed cost & autobuyer intervals are derived — recompute on load, don't
  trust saved values.

## 8. Testing

1. **Codec round-trip (ours):** `GameState` → `encode_save` → `decode_save` →
   compare modelled fields.
2. **Decode a real external save fixture:** drop a genuine `AAB` save string into
   `ad-core/tests/fixtures/`, decode it, assert the mapped early-game fields.
   This proves the pipeline matches pako/btoa exactly.
3. **Encode → original game (manual / fidelity):** produce a save, paste it into
   the real game's import, confirm it loads without migration warnings and shows
   the expected antimatter/dimensions. This is the acceptance test for req. (2).
4. **Pipeline unit tests:** each §2.1 step in isolation against known vectors
   (especially the base64 cleanup and zlib).
5. **Property test:** random early-game `GameState`s survive round-trip within
   the modelled fields.

## 9. Suggested phasing

1. **[DONE]** `Decimal` string (de)serialization helpers + tests.
2. Encode/decode *pipeline* (steps 1–7) with fixture tests, JSON ⇄ string only.
3. Read path: DTO + `from_save_dto`, decode an external save into `GameState`.
4. Write path: vendored template + overlay + `encode_save`.
5. Tauri commands + webview import/export modals.
6. Autosave, on-disk persistence, "time since last save", keyboard shortcuts.

Phases 1–4 live entirely in `ad-core`/`break_infinity` and are testable headless
before any UI work.

### Progress log

- **Phase 1 — done.** Added `break_infinity::serde_string`, an opt-in serde
  helper module (gated behind the crate's existing `serde` feature) that
  (de)serializes a `Decimal` as a JSON **string** via its `Display`/`FromStr`
  impls, matching break_infinity.js `toString`/`fromString`. Provides
  `serialize`/`deserialize` (for `#[serde(with = "break_infinity::serde_string")]`)
  and a `::option` submodule for `Option<Decimal>` (string-or-null). The type's
  derived `{m,e}` serde is intentionally left untouched for our own internal
  serialization (`ad-core`/`ad-gui`); only the save DTO routes through the new
  helpers. Verified the type-level `Infinity` sentinel matches the original
  exactly (both emit `"Infinity"` at `e >= 9e15 = EXP_LIMIT`); the game's
  *formatting* Infinity-threshold is a separate display concern and never enters
  serialization. Tests cover plain/scientific/sentinel forms, round-trips,
  rejection of non-string input, and the `option` helper.
  - Files: `crates/break_infinity/src/serde_string.rs` (new),
    `crates/break_infinity/src/lib.rs` (module registration),
    `crates/break_infinity/Cargo.toml` (serde_json/serde dev-deps).
  - Run the tests: `cargo test -p break_infinity --features serde` (they are
    feature-gated, so a plain `cargo test` skips them; the whole-workspace
    command is `cargo test --workspace --all-features`).

- **Next up — phase 2:** the encode/decode pipeline (§2.1 / §4.1), JSON ⇄ save
  string only, with the real external save in `ad_initial_save.txt` as a decode
  fixture.

## 10. Open decisions (RESOLVED)

All three resolved by the user (2026-06-29):

- **On-disk format:** reuse the AD-compatible encoded string for our own saves —
  one codec to maintain, freely importable into the real game. No separate
  internal JSON format.
- **Template source:** AD is not under active development and the save format is
  stable, so regenerating the `defaultStart` template is kept a **manual
  process**. An initial real save captured from the browser game is checked in
  at `ad_initial_save.txt` (repo root) for use as a decode fixture / template
  source.
- **Legacy saves:** **reject** pre-`AAB` (pre-Reality) saves. Only the latest
  format (`AAB`) needs to be supported, in both directions. The `atob`-only
  legacy decode path is out of scope.
