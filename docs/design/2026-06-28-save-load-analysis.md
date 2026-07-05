# Save / Load: Analysis

Status: in progress (§9 phases 1–5 done; phase 6 **done** — on-disk
persistence, 3 save slots, autosave, 8 backup slots/slot, backup-bundle
file import/export, time-since-save, and the Ctrl/Cmd+S shortcut are all
wired). Scope: design how `ad-gui` persists a game and, crucially, how it
interoperates with **external Antimatter Dimensions saves** — both directions —
while only a slice of the game is implemented.

See §9 for the phase checklist, §10 for the now-resolved open decisions,
§11 for the original game's full persistence architecture (localStorage,
save slots, backup slots, autosave, clipboard/file, cloud saves), and §12
for our Tauri persistence design.

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

### 2.4 Other payloads sharing the same codec

`GameSaveSerializer` (the §2.1 `AAB` pipeline) is a *universal* string codec —
the same encode/decode is reused for several different JSON payloads. Everything
we read so far is a single `player` object, but the game stores and exports a few
other shapes through the identical codec. Source:
`../antimatter-dimensions/src/core/storage/storage.js`.

| Producer | Decoded JSON shape | Notes |
|----------|--------------------|-------|
| Options → "Export as File" (`exportAsFile`) / clipboard `export` / cloud / a single backup *slot* (`saveToBackup`) | a single `player` | What `decode_save`/`encode_save` handle today; our fixtures are these. |
| **Backup → "Export as File" (`exportBackupsAsFile`) / "Import from File" (`importBackupsFromFile`)** | **a *bundle* of players** | See below. **Not** a single player — needs a dispatch layer. |
| Browser `localStorage` root (`GameStorage.load`/`save`) | `{ saves: { 0,1,2 }, current }` wrapper (or, legacy, a bare `player`) | Only relevant if we ever ingest a *live* localStorage dump; out of scope for file/clipboard import. |

**The backup-bundle file** (`exportBackupsAsFile`, storage.js:373) is the same
`AntimatterDimensionsSavefileFormatAAB…EndOfSavefile` string, but the payload is
a map of populated backup slots to full `player` objects, plus a reserved `time`
key of timing metadata:

```json
{
  "1": { …full player… },        // keyed by AutoBackupSlots id (1–8):
  "2": { …full player… },        //   online 1/2/3/4, offline 5/6/7, reserve 8
  "5": { …full player… },        // only populated slots are present
  "8": { …full player… },
  "time": { "1": { "backupTimer": …, "date": … }, … }
}
```

`importBackupsFromFile` (storage.js:387) reverses it: skip `"time"`, treat every
other key as a slot id whose value is a full `player`.

**Implication (deferred — documented 2026-06-30, not yet implemented).** To
support the Backup menu's *Export/Import as File*, the byte codec is unchanged,
but we need a **dispatch layer above `decode_save`** that, after the pipeline,
inspects the top-level JSON: a single player (`version`/`antimatter` at top) maps
via `from_save_dto` as today; a bundle (numeric slot keys + `time`) yields the
contained players (each is a full `player` → `from_save_dto`), carrying slot id +
backup time. Proposed shape:

```rust
pub enum ImportedSave {
    Single(GameState),
    Backups(Vec<BackupSlotSave>), // { id, backup_timer, state }
}
pub fn decode_save_file(text: &str) -> Result<ImportedSave, SaveError>;
```

Writing a bundle (export) additionally needs a multi-slot/backup concept of our
own to bundle, so it is tied to how `ad-gui` manages saves. **Open (deferred to
the implementation phase):** scope (import-only vs symmetric) and how a bundle's
multiple saves are surfaced on import (return all for the caller to choose vs
auto-pick the newest by `backupTimer`).

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
schema for *only the fields we read*. `serde` on a struct already ignores unknown
keys by default, which is exactly the "ignore unimplemented mechanics" behavior
we want for req. (1): a late-game save deserializes fine; we read the handful of
fields we understand and drop the rest.

**Strict on what we model (decided 2026-06-30).** The fields we *do* declare are
**required** — no `#[serde(default)]` — so a missing modelled field fails the
load (serde "missing field") rather than being silently replaced by a default.
The goal is to be *alerted* that the external format changed rather than quietly
diverge. This is orthogonal to ignoring unknown keys (which still happens). The
same strictness applies in `from_save_dto` validation (below).

The DTO uses a `Decimal`-as-string newtype/helper (a `deserialize_with` that
calls `Decimal::from_str`) so the string format from §2.2 is honored.

`GameState::from_save_dto(dto) -> Result<GameState, SaveError>`:

- copy mapped scalar/Decimal fields;
- rebuild derived state (tickspeed cost, autobuyer timers/intervals) from our own
  constructors so it's internally consistent;
- **validate strictly, erroring (not clamping/guessing)** on anything off:
  out-of-range numeric options (`SaveError::OptionOutOfRange`), an unexpected
  fixed-array length (`UnexpectedArrayLength`), or an unrecognized autobuyer mode
  (`InvalidAutobuyerMode`);
- **one intentional leniency:** an unmodelled `options.notation` name (we
  implement only a subset of the game's notations, and the game default
  "Mixed scientific" is one we don't model) is ignored, keeping our default —
  erroring there would reject almost every real save;
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
- Out-of-range or malicious external values: validated on import and **rejected
  with an error** (not clamped/guessed), so an unexpected value or format change
  is surfaced rather than silently absorbed. The lone exception is an unmodelled
  notation name, which is ignored (see §4.2).
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
2. **[DONE]** Encode/decode *pipeline* (steps 1–7) with fixture tests, JSON ⇄ string only.
3. **[DONE]** Read path: DTO + `from_save_dto`, decode an external save into `GameState`.
4. **[DONE]** Write path: vendored template + overlay + `encode_save`.
5. **[DONE]** Tauri commands + webview import/export modals.
6. **[DONE]** Autosave, on-disk persistence, save slots + backups, "time since
   last save", keyboard shortcuts (§12).

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

- **Phase 2 — done.** Added the `ad-core::save` module (gated behind the crate's
  `serde` feature, per §6), with a `codec` submodule porting
  `serializer.js`/`GameSaveSerializer` for the `AAB` format:
  `encode_pipeline(json: &str) -> String` and
  `decode_pipeline(save: &str) -> Result<String, SaveError>`. Pure/deterministic,
  no IO, JSON-string ⇄ save-string only. The `Infinity`/`Set`/`Decimal`-string
  conventions are deliberately left to the DTO layer (phases 3–4). Uses `flate2`
  (zlib) and `base64` as optional deps pulled in by the `serde` feature.
  `SaveError` covers `UnrecognizedFormat` (no magic prefix → legacy/garbage,
  rejected per §10), base64, inflate, and UTF-8 failures.
  - Two real saves serve as fixtures (decoded & asserted in tests), under
    `crates/ad-core/tests/fixtures/`: `ad_initial_save.txt` (fresh start) and
    `ad_sample_save.txt` (mid-game, pre-Big-Crunch).
  - Tests (6): decode each real fixture and assert mapped fields; JSON round-trip;
    encoded-string well-formedness (prefix/suffix present, no raw `+`/`/`/`=` in
    the body); both real fixtures survive decode→re-encode→decode; rejection of
    non-AD / legacy strings.
  - Cross-checked the **write** direction out-of-band: the original game's own
    `pako.inflate` successfully decompresses our `encode_pipeline` output (zlib
    bytes differ from pako's, but the stream is standard zlib so the real game
    accepts it). The full §8.3 paste-into-game acceptance waits on phase 4's
    template-based complete save.
  - Files: `crates/ad-core/src/save/mod.rs` (module + `SaveError`),
    `crates/ad-core/src/save/codec.rs` (pipeline + tests),
    `crates/ad-core/src/lib.rs` (gated `pub mod save`),
    `crates/ad-core/Cargo.toml` (`flate2`/`base64` optional deps via `serde`
    feature; `serde_json` dev-dep).

- **Phase 3 — done** (read path; made **strict on load** 2026-06-30, see §4.2):
  - `save/dto.rs` — `Deserialize`-only serde DTOs mirroring the `player` schema
    for the modelled subset only (`PlayerDTO` + `DimensionsDTO`/`DimensionDTO`,
    `RecordsDTO`, `AutoDTO`/`AntimatterDimsDTO`/`AutobuyerDTO`,
    `OptionsDTO`/`NotationDigitsDTO`), `#[serde(rename_all="camelCase")]`.
    **No serde defaults:** the fields we model are required, so a missing one is a
    load error (surfacing a format change) rather than a silent default; unknown
    keys are still ignored. Every `Decimal` is read as a string via
    `break_infinity::serde_string`; `break` via `#[serde(rename="break")]`.
  - `GameState::from_save_dto(&PlayerDTO) -> Result<GameState, SaveError>` —
    copies the mapped fields; rebuilds derived state from our own constructors
    rather than trusting the save (`TickspeedState::with_bought(totalTickBought)`
    recomputes the unsaved tickspeed cost; autobuyer intervals/timers come from
    `AutobuyerState::new()`, only the `isActive`/`isBought`/`mode` flags overlaid);
    computes `infinity_unlocked = break || infinities > 0 || infinityPoints > 0`
    (§4.3); locks the tickspeed autobuyer to single-buy regardless of saved mode
    (§4.4). **Strict validation, erroring rather than guessing:** autobuyer mode
    must be `1`→`BuySingle` / `10`→`BuyMax` (else `InvalidAutobuyerMode`); the
    dimension and dimension-autobuyer arrays must be exactly length 8 (else
    `UnexpectedArrayLength`); numeric options must be in range (else
    `OptionOutOfRange`). **Sole leniency:** an unmodelled `options.notation` (incl.
    the game default "Mixed scientific") is ignored, keeping our default.
  - `save::decode_save(&str) -> Result<GameState, SaveError>` ties it together
    (`decode_pipeline` → `serde_json` → `from_save_dto`). `SaveError` gained
    `Json`, `InvalidAutobuyerMode(i64)`, `OptionOutOfRange { field, value, min,
    max }`, and `UnexpectedArrayLength { field, expected, found }`.
  - `TickspeedState::with_bought` added in `state.rs`.
  - `serde_json` moved from a dev-dependency to an optional dependency pulled in
    by the `serde` feature (now needed at runtime by `decode_save`).
  - Tests (11): decode both real fixtures and assert mapped fields; `infinity_unlocked`
    from each of `break`/`infinities`/`infinityPoints`; unknown-field tolerance;
    **missing modelled field errors**; **wrong array length errors**; in-range
    options applied vs **out-of-range options error**; unmodelled notation kept
    lenient; autobuyer mode/flag mapping and **invalid mode errors**. Targeted
    tests mutate the real fixture JSON (now that partial objects no longer parse).
  - Files: `crates/ad-core/src/save/dto.rs` (new),
    `crates/ad-core/src/save/mod.rs` (`decode_save`, the `SaveError` variants),
    `crates/ad-core/src/state.rs` (`TickspeedState::with_bought`),
    `crates/ad-core/Cargo.toml` (`serde_json` optional dep).

- **Phase 4 — done.** Added the write path in `ad-core::save`:
  - `save/default_player.json` — the vendored `defaultStart` template: a complete,
    valid fresh-start `player` (67 keys, `version: 25`, all empty `Set`s as `[]`,
    full `options`/`auto`/`records` trees) decoded from
    `tests/fixtures/ad_initial_save.txt`. Regenerated manually per §10 (decode a
    fresh save, overwrite the file).
  - `save/encode.rs` — `encode_save(&GameState, now_ms: i64) -> String`. Parses a
    fresh copy of the template into a `serde_json::Value`, overlays only our
    modelled fields **in place** (never removing keys, so the object stays
    complete): `antimatter`, `records.totalAntimatter`, `sacrificed`,
    `dimensionBoosts`, `galaxies`, `totalTickBought`, `break` (the §4.3-inverse of
    `infinity_unlocked`), each `dimensions.antimatter[t].{amount,bought}`, the
    `options.*` subtree, and the `auto.*` flags/modes — then runs `encode_pipeline`.
    Decimals are written as JSON strings (`Decimal::Display`); `AutobuyerMode` maps
    back to the numeric `AUTOBUYER_MODE` (`BuyMax`→10, `BuySingle`→1); autobuyer
    intervals/timers are left at the template's derived values (§4.4); `lastUpdate`
    is stamped with the caller-supplied `now_ms` so import computes ~0 offline
    progress (engine stays clock-free).
  - Tests (4): template completeness/validity; produced save decodes back to a
    complete `player` with our fields + timestamp; full `decode → encode → decode`
    round-trip reproduces every modelled field for both fixtures; mutated-state
    changes survive the round-trip (incl. `break` reflecting `infinity_unlocked`).
  - **Acceptance (§8.3) cross-checked out-of-band:** emitted a real `encode_save`
    output from Rust and decoded it with the **original game's own** `decodeText` +
    `JSON.parse` — accepted, `version: 25` (no migrations), all 67 keys present,
    overlaid fields intact, `lastUpdate` stamped. The write path is wire-compatible
    with the real game (a manual paste-into-game remains the ultimate confirmation).
  - Files: `crates/ad-core/src/save/default_player.json` (new template),
    `crates/ad-core/src/save/encode.rs` (new), `crates/ad-core/src/save/mod.rs`
    (`pub use encode::encode_save`).

- **Phase 5 — UI shell built; core save/load buttons wired.** The **Saving**
  options subtab and its modals exist in `ad-gui` as faithful, vendored-CSS
  replicas of the original (top half only; all Cloud-save UI deliberately
  omitted, as is the post-Reality Speedrun row):
  `components/tabs/OptionsSavingTab.vue` (the button grid: Export/Import save,
  RESET THE GAME, Save game, Choose save, autosave-interval slider,
  Export/Import save as file, "Display time since save" toggle, Open Backup
  Menu, Save file name input) plus four modals — `ImportSaveModal.vue`,
  `HardResetModal.vue`, `LoadGameModal.vue`, `BackupWindowModal.vue` — opened
  via `ui.openModal` ids `importSave`/`hardReset`/`loadGame`/`backup`. The tab
  is wired into `config/tabs.js`; modals render from `App.vue`. Modal widths
  use `Modal.vue`'s `fit-content` to match the originals.
  - **Wired (2026-06-30):** five Tauri commands backed by the engine codec:
    - `export_save` — encodes the `Mutex<GameState>` via `encode_save`, returns
      the AD save string. The frontend copies it to the clipboard via
      `navigator.clipboard.writeText()` and shows a toast.
    - `import_save(text)` — `decode_save` on the input, swaps the
      `Mutex<GameState>`, returns a fresh `GameView`. The `ImportSaveModal`
      sends the pasted text, shows errors inline, closes + toasts on success.
    - `export_save_to_file(save_file_name)` — encodes, then shows a native
      "Save As" dialog via `tauri-plugin-dialog` (`blocking_save_file`), writes
      the `.txt`. The "Save file name" input on the tab feeds the default
      filename.
    - `import_save_from_file` — shows a native "Open" dialog
      (`blocking_pick_file`), reads the file, decodes, swaps state. Replaces
      the old `<input type="file">` + `.c-file-import` CSS hack, eliminating
      the WebKit overflow issue entirely.
    - `hard_reset` — replaces the `Mutex<GameState>` with `GameState::new()`,
      returns a fresh `GameView`. The `HardResetModal` calls it on confirmation
      (the "Shrek is love, Shrek is life" phrase gate is unchanged).
    - Added `tauri-plugin-dialog` dependency (Cargo.toml, registered in
      Builder, `dialog:default` permission in `capabilities/default.json`).
    - Store actions: `exportSave`, `importSave`, `exportSaveToFile`,
      `importSaveFromFile`, `hardReset` in `stores/game.js`.
  - **Still to wire:** Save game button, Choose save / save slots, backup
    slots, autosave, on-disk persistence, "time since last save", the `S`
    keyboard shortcut (save).
  - **WebKit note (now partially resolved):** the `.c-file-import` overflow
    hack is no longer used for the main "Import save from file" button (replaced
    by the native dialog), but `BackupWindowModal.vue` still uses the
    `<input type="file">` + `overflow: hidden` pattern for its per-slot import.

- **Phase 6 — done (§12 local persistence).** On-disk save/load, save slots,
  automatic backups, and the remaining Saving-tab wiring.
  - **Engine (`ad-core::save`, still pure).** Added the multi-player *bundle*
    codec in `save/bundle.rs`: the localStorage-root shape `{ current, saves }`
    (`RootSave` + `encode_root`/`decode_root`, §11.1, tolerating a legacy bare
    `player` in slot 0) and the backup-bundle file shape (§2.4) via
    `ImportedSave`/`BackupSlotSave` + `decode_save_file` (dispatches single vs
    bundle on the top-level shape) + `encode_backup_bundle`. Both reuse the
    existing byte pipeline; the shared building blocks `to_player_value`
    (overlay → `Value`) and `from_player_value` (`Value` → `GameState`) were
    factored out of `encode`/`decode`. `RootSave` carries each slot's
    `lastUpdate` for offline-gap detection. `SaveError` gained
    `MissingSavesWrapper`.
  - **Engine-owned Saving options.** `autosave_interval` (ms, 10–60 s slider,
    range-checked on load) and `show_time_since_save` were added to `Options`
    (DTO + overlay + setter), so they round-trip through saves like the other
    options.
  - **GUI backend (`ad-gui/src/persistence.rs`).** `SaveManager` owns the
    app-data dir (§12.1), the active slot, and the slot cache. It does all the
    IO and wall-clock work `ad-core` deliberately avoids: `saves.dat` (encoded
    root, all 3 slots), `backups/{slot}/{1..8}.dat` (encoded single players),
    atomic temp-file+rename writes, backup ages from file mtime (no separate
    `times.dat`). On startup (`.setup`) it loads the root into the live
    `Mutex<GameState>` and fires the longest applicable **offline** backup from
    the load gap; **online** backups (slots 1–4) and **autosave** are driven on
    the wall clock by the `App.vue` rAF loop; the **reserve** slot (8) is written
    before any backup load. New Tauri commands: `save_game`, `switch_save_slot`,
    `get_save_slots`, `trigger_backup`, `get_backups`, `load_backup`,
    `export_backups_to_file`, `import_backups_from_file`, plus
    `set_autosave_interval`/`set_show_time_since_save`; `import_save*` and
    `hard_reset` now persist the root immediately (mirroring the original's
    save-after-import).
  - **Frontend.** `LoadGameModal` and `BackupWindowModal` are wired (fetch
    summaries on open, Load switches slot / loads backup); the backup modal's
    Export/Import-as-file now use native dialogs (dropping the last
    `<input type=file>`/`.c-file-import` WebKit hack). The Saving tab's *Save
    game* button, autosave slider, and *Display time since save* toggle drive the
    engine; a bottom-left `SaveTimer.vue` (faithful replica of the original's
    fixed `o-save-timer`, click-to-save, `timeDisplayShort`/HH:MM:SS format, gated
    by `show_time_since_save`) shows the elapsed time; `Ctrl/Cmd+S` saves
    (original `mod+s`, a `bind` so it ignores the Hotkeys option).
  - **Deferred still:** offline *progress* on real startup (only the offline
    *backup* fires; the game itself doesn't replay the gap yet — the Offline-mode
    dev control remains the only replay path), and the Backup modal's "Load with
    offline progress disabled" toggle is inert for the same reason.

## 10. Open decisions (RESOLVED)

All three resolved by the user (2026-06-29):

- **On-disk format:** reuse the AD-compatible encoded string for our own saves —
  one codec to maintain, freely importable into the real game. No separate
  internal JSON format.
- **Template source:** AD is not under active development and the save format is
  stable, so regenerating the `defaultStart` template is kept a **manual
  process**. An initial real save captured from the browser game is checked in
  at `crates/ad-core/tests/fixtures/ad_initial_save.txt` for use as a decode
  fixture / template source.
- **Legacy saves:** **reject** pre-`AAB` (pre-Reality) saves. Only the latest
  format (`AAB`) needs to be supported, in both directions. The `atob`-only
  legacy decode path is out of scope.

## 11. Original game persistence architecture

Source: `../antimatter-dimensions/src/core/storage/storage.js`,
`intervals.js`, `cloud-saving.js`, `player.js`.

This section documents how the browser game persists saves —
`localStorage`, save slots, automatic backups, clipboard/file
export, and (briefly) cloud saves — as context for our Tauri
analogue in §12.

### 11.1 localStorage: the primary store

All local persistence goes through the browser `localStorage` API.
The root key is `"dimensionSave"` (`"dimensionTestSave"` in dev
mode), keyed via `GameStorage.localStorageKey`.

The root value is a `GameSaveSerializer`-encoded (§2.1 pipeline)
JSON object with this shape:

```json
{
  "current": 0,
  "saves": {
    "0": { …full player… },
    "1": { …full player… },
    "2": null
  }
}
```

`current` is the active slot index (0–2). `saves` holds up to **3
save slots**, each either a full `player` object or `null`/`undefined`
(empty slot). The game encodes the entire root into a single
`localStorage` entry on every save.

There is also a legacy format (a bare `player` with no `saves`
wrapper); on first load, `loadRoot()` migrates it into the new
format.

### 11.2 Save slots (3 slots)

The player can switch between 3 save slots via **Options → Saving →
Choose save** (`LoadGameModal`). Switching:

1. Saves the current slot first (`this.save(true)`).
2. Loads the target slot's `player` (or `Player.defaultStart` if
   empty).
3. Reloads backup times and fires offline backup checks.

### 11.3 Automatic backups (8 slots per save slot)

Each of the 3 save slots has its own independent set of **8 backup
slots**, stored in separate `localStorage` keys:

- Data: `backupSave-{saveSlot}-{backupSlot}` (each is a
  `GameSaveSerializer`-encoded single `player`).
- Timers: `backupTimes-{saveSlot}` (an encoded map of timing
  metadata: `{ backupTimer, date }` per backup slot).

The 8 backup slots are typed:

| Id | Type | Interval |
|----|------|----------|
| 1 | ONLINE | 1 minute |
| 2 | ONLINE | 5 minutes |
| 3 | ONLINE | 20 minutes |
| 4 | ONLINE | 1 hour |
| 5 | OFFLINE | 10 minutes |
| 6 | OFFLINE | 1 hour |
| 7 | OFFLINE | 5 hours |
| 8 | RESERVE | (manual only) |

**ONLINE** backups are checked every second by
`GameIntervals.checkEverySecond` (a `setInterval(…, 1000)` timer).
`tryOnlineBackups()` scans all ONLINE slots, and for each one where
`player.backupTimer - lastBackupTimes[id].backupTimer >=
interval × 1000 - 800` (the 800 ms grace prevents timer drift from
the save I/O itself), it writes the current `player` into that
backup slot's `localStorage` key.

**OFFLINE** backups fire once on game load. `backupOfflineSlots()`
compares `Date.now()` against `lastUpdateOnLoad` (the `lastUpdate`
from the loaded save). If the offline gap exceeds a slot's interval,
the **longest matching** offline slot is written (only one offline
slot is saved per load, the longest applicable).

The **RESERVE** slot (id 8) is written manually via
`saveToReserveSlot()`, triggered before risky operations (e.g.
before applying a cloud save).

`player.backupTimer` is an in-game-time counter (incremented in the
game loop) that serves as the logical clock for online backup
intervals; it is not wall-clock time. This prevents paused or
offline time from skewing the intervals.

### 11.4 Autosave

`GameIntervals.save` is a `setInterval` timer calling
`GameStorage.save()`. Its interval is
`player.options.autosaveInterval` (default 30 000 ms = 30 s),
adjustable via the UI slider from **10 s to 60 s** in 1 s steps.
The timer accounts for drift: on restart, its effective interval
subtracts the time already elapsed since `lastSaveTime`.

`GameStorage.save()` encodes the entire root (`{ current, saves }`)
and writes it to the single `localStorage` key. It respects a
`canSave()` guard that blocks saving during glyph selection, offline
progress simulation, and certain endgame states.

### 11.5 Export/import: clipboard and file

**Clipboard export** (`GameStorage.export()`): encodes the current
slot's `player` via `GameSaveSerializer.serialize(player)`, copies
the string to the system clipboard via `document.execCommand("copy")`
(a hidden `<textarea>`).

**Clipboard import** (`GameStorage.import(saveData)`): deserializes
the pasted string, validates (§6 `checkPlayerObject`), and loads it
into the current slot. Then fires an immediate save so the import
persists.

**File export** (`GameStorage.exportAsFile()`): same as clipboard
export, but writes to a downloaded `.txt` file. The filename
includes the slot number, an optional user-chosen `saveFileName`,
an incrementing `exportedFileCount`, and the date:
`AD Save, Slot 1 - myname, #3 (2026-7-1).txt`.

**File import** (`GameStorage.importAsFile()`): reads a user-
selected file via `FileReader`, then calls `import()`.

**Backup file export/import** (`exportBackupsAsFile` /
`importBackupsFromFile`): bundles all populated backup slots for the
current save slot into a single encoded file (see §2.4 for the
bundle format). Import writes each slot back to its `localStorage`
key.

### 11.6 "Time since last save" indicator

When `player.options.showTimeSinceSave` is enabled (default `true`),
the bottom-left of the game header shows the elapsed time since
the last autosave. This is a display-only concern driven by
`Date.now() - GameStorage.lastSaveTime`, rendered in the game's
time-span format.

### 11.7 Cloud saves (Firebase — not replicated)

The original offers optional **cloud saves** via **Firebase Realtime
Database** with Google OAuth authentication. The Firebase project
(`antimatter-dimensions-a00f2`) is hosted by the game developer;
its API keys are embedded in the production JS bundle (Firebase API
keys are public client-side identifiers — access is controlled by
server-side security rules scoping each user to their own
`users/{uid}/` path).

Cloud saves:
- Store only the **current slot** (not all 3, not backups) at
  `users/{uid}/web/{slotNumber}`.
- Auto-sync every 10 minutes (`GameIntervals.checkCloudSave`,
  600 s), with conflict detection (SHA-512/256 hash comparison +
  `ProgressChecker` for which save is farther/older) and a modal
  for the player to resolve conflicts.
- Use the same `GameSaveSerializer` codec as local saves.

**We do not plan to replicate cloud saving.** The Firebase
integration, Google OAuth, and conflict-resolution modals are out
of scope for our standalone version. The UI omits the entire Cloud
save section of the Saving tab.

## 12. Our Tauri persistence design

### 12.1 Storage location

Instead of `localStorage`, we use the filesystem via Tauri's
`path::app_data_dir()`, which resolves to the OS-appropriate
application data directory:

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/com.antimatter-dimensions.rust/` |
| Linux | `~/.config/com.antimatter-dimensions.rust/` (or `$XDG_CONFIG_HOME/…`) |
| Windows | `C:\Users\{user}\AppData\Roaming\com.antimatter-dimensions.rust\` |

The bundle identifier comes from `tauri.conf.json` (`"identifier":
"com.antimatter-dimensions.rust"`).

### 12.2 File layout (proposed)

```
{app_data_dir}/
├── saves.dat          # Root save: encoded { current, saves }
├── backups/
│   ├── 0/             # Backups for save slot 0
│   │   ├── 1.dat      # Backup slot 1 (online, 1 min)
│   │   ├── 2.dat      # …
│   │   ├── …
│   │   ├── 8.dat      # Backup slot 8 (reserve)
│   │   └── times.dat  # Backup timing metadata
│   ├── 1/             # Backups for save slot 1
│   │   └── …
│   └── 2/             # Backups for save slot 2
│       └── …
```

Each `.dat` file contains a `GameSaveSerializer`-encoded string
(the same `AAB` format), matching our resolved decision (§10) to
use the AD-compatible codec everywhere. This means any `.dat` can
be pasted into the original game's import box.

Alternatively, a simpler flat layout using the same naming as the
original's `localStorage` keys (`backupSave-0-1.dat`, etc.) would
also work.

### 12.3 Features to replicate

All local save functionalities from §11 should be faithfully
replicated:

1. **3 save slots** with a "Choose save" modal to switch between
   them (§11.2).
2. **Autosave** on a configurable interval (10–60 s slider, default
   30 s), writing the root `{ current, saves }` to `saves.dat`
   (§11.4).
3. **8 backup slots per save slot** with the same ONLINE/OFFLINE/
   RESERVE types and intervals (§11.3). ONLINE backups checked
   every second; OFFLINE backups on app start; RESERVE on demand.
4. **Clipboard export/import** (§11.5) — the webview uses
   `navigator.clipboard` (already wired in phase 5).
5. **File export/import** (§11.5) — Tauri native file dialogs
   (already wired in phase 5 via `tauri-plugin-dialog`).
6. **Backup file export/import** (§11.5, §2.4) — the bundle format.
7. **"Time since last save"** display (§11.6).
8. **Manual "Save game" button** and `S` keyboard shortcut.
9. **Hard reset** (already wired in phase 5).

### 12.4 What we omit

- Cloud saves / Firebase / Google OAuth (§11.7).
- The Cloud-related UI (save/load cloud buttons, conflict modals,
  `hideGoogleName`, `forceCloudOverwrite`, `syncSaveIntervals`).
- Secret import easter eggs (`tryImportSecret`).
- Speedrun-specific save modifications (`exportModifiedSave`
  segmented flag).
- The `canSave()` guards for glyph selection and endgame states
  (not yet relevant at our implementation frontier).
