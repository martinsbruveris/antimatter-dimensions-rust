---
date: 2026-07-07
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity harness — Rust comparison side (stage 3)

## Summary
Built the Rust replay/comparison stage of the save-replay fidelity harness: a
library + `ad-fidelity` CLI that loads oracle fixtures, replays each input save
through `ad-core` to the same horizons, and diffs the persisted `player` tree
field-by-field with per-field tolerance. This is the third and final stage of the
harness (capture and oracle shipped 2026-07-06).

## What shipped
- `compare.rs` — the tolerant diff walker. A `FieldRule { path, mode }` table is
  resolved against two `serde_json::Value` player trees; a dotted path with a
  `[]` array-wildcard drives lockstep descent. Comparison modes (design §8):
  `Exact`, `Decimal` (log-space), `Number` (relative), `IdSet`
  (order-insensitive), `Glyphs` (object-array matched by slot `idx`).
- `allowlist.rs` — the include-only allowlist over the save tree (design §5):
  AM economy, Infinity/Eternity/Replicanti/Dilation, unlock-gating records,
  autobuyer settings, achievements/requirement checks, and partial
  Reality/black-hole coverage. ~120 rules.
- `fixture.rs` — fixture loading (`{meta, input, expected{horizon→save}}`) and
  the Rust replay: `decode_save` → `ticks(tick_ms, horizon)` → `to_player_value`.
- `run.rs` — orchestrates a (fixtures × horizons) grid of `Outcome`s
  (Pass/Fail/Error/Missing), with tally + exit-code helpers.
- `report.rs` — the default pass/fail table (rows = fixtures, cols = horizons)
  and the `--verbose` per-field listing.
- `main.rs` — the clap CLI: `[DIR] --tests --ticks --tick-ms --epsilon
  --roundtrip --verbose`, exiting non-zero on any failure (CI-ready).
- `tests/replay_smoke.rs` — end-to-end plumbing tests against the real
  `saves/01_pre_big_crunch.txt`, no Node required.

## Decisions & why
- **Compare raw `Value` trees, not `PlayerDTO`.** Design §6 floats `PlayerDTO` as
  the canonical intermediate, but the allowlist and comparison modes are all
  expressed in JS/save keys, and comparing the serialized form directly also
  exercises Rust's write path (the design's stated goal, guarded by the
  round-trip check below). It's less code and stays faithful to "compare the
  persisted save". The Rust `actual` side is `to_player_value` (overlay onto the
  default template); since the allowlist is include-only, the template defaults
  for unmodelled keys never produce noise.
- **`Exact` compares numbers by value, recursively.** The real save stores
  integral values as JSON ints (`0`, `1`), while Rust writes several as floats
  (`0.0`, `1.0` — `reality.seed`, `perkPoints`, `glyphs.sac`, …). That's a
  serialization-representation quirk, never a fidelity divergence, so `Exact`
  treats `0 == 0.0` (integers compared as integers to keep >2^53 bitmask
  precision). This was surfaced by the round-trip test, not guessed.
- **`--roundtrip` / horizon 0 = the design §6 identity guard.** Comparing Rust's
  decode→encode of the *input* against the input itself isolates an encode/decode
  bug from a tick bug, cheaply, in the same grid.
- **Tolerance kept general but simple.** A single log epsilon to start (design
  §10), but `Tolerance` already carries a per-tick-linear term so horizon-scaled
  tolerance can be switched on once the oracle gives us data.
- **Tick granularity defaults to each fixture's `meta.tickMs`.** A mismatched
  tick size diverges the engines by construction, so the replay reads the
  oracle's own value unless `--tick-ms` overrides it.

## Deviations from the design doc
- Comparison happens at the `Value`-tree layer rather than by deserializing both
  sides into `PlayerDTO` (design §6 wording). Same boundary (the persisted
  player tree), simpler mechanism; noted above. Design doc left as `Accepted` —
  the architecture it describes is unchanged, only the intermediate
  representation differs.
- Glyph tolerant-match is implemented but exercised only trivially (the one
  available save is pre-Infinity, so glyph arrays are empty). Treated as
  best-effort until a glyph-bearing save exists.

## Surprises & gotchas
- serde_json `Value` equality distinguishes `0` (int) from `0.0` (float); a naive
  `Exact` fails on a dozen legitimately-equal fields. See the `Exact`-by-value
  decision.
- The round-trip test doubles as a real regression guard on `ad-core`'s write
  path: it passes today, meaning decode→encode preserves every allowlisted
  modelled field for a real save.

## Follow-ups
- **No fixtures exist yet** — the oracle hasn't been run end-to-end (needs the
  game served + `playwright install`). The CLI degrades gracefully ("no fixtures
  found") until then; the smoke tests fabricate fixtures to exercise the plumbing.
- **Event-count mode** (design §7/§8) is not implemented; near-threshold discrete
  events (an off-by-one galaxy/crunch) will currently show as a wall of magnitude
  diffs. Comparing per-window deltas of galaxies/infinities/eternities would be
  more robust.
- **Fail-loud on unknown fields** (design §5) — the allowlist is include-only and
  silently ignores new save keys; a coverage check could flag them.
- Horizon-scaled tolerance constants remain to be fixed empirically.

## Tests
- `cargo test -p ad-fidelity` — 10 pass (7 unit in `compare`, 3 end-to-end in
  `replay_smoke`, including the full-allowlist round-trip identity over the real
  save). `cargo clippy -p ad-fidelity --all-targets` clean; `cargo fmt` applied.
