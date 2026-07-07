---
date: 2026-07-07
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity harness — trace debugging UI + shared id convention

## Summary
Added a debugging workflow for narrowing 1000-tick divergences: a dense per-tick
trace fixture from the oracle, an `ad-fidelity trace` subcommand that reports the
first tick to diverge (and the fields that broke), and a shared short-id
convention for naming saves/fixtures across both the Rust crate and the JS oracle.

## What shipped
- **Shared id resolver** (`src/resolve.rs`, mirrored in the oracle's
  `resolveId`): given `id`, `dir`, `ext`, resolve in the order
  path → `dir/id` → glob `0*<id>-*.<ext>`, accepting only existing files. The
  glob must be unambiguous — 0 matches is `NotFound`, ≥2 is `Ambiguous` listing
  candidates. Matching strips leading zeros from the filename's leading number
  segment (so `1` matches `00001-…` but not `00010-…`; all-zero → `0`).
- **`ad-fidelity trace <ID>`** (`src/trace.rs`, `run_trace` in `main.rs`): loads a
  dense fixture, scans its horizons ascending, and reports the first divergent
  tick + fields. `--tick X` dumps the full field diff at exactly `X`. Inherits
  `--epsilon`/`tickMs`; reuses the grid's allowlist + `Tolerance`, so divergence
  means the same thing in both. Exit codes match the grid (1 divergence, 2 error,
  0 clean).
- **Oracle `--save` / `--trace`** (`generate-replay-fixtures.js`): `--save <id>`
  restricts the run to one resolved capture; `--trace <name>` writes a single
  dense fixture (ticks `1..1000`, same schema) to `saves/traces/<name>` and
  requires `--save`. Standard `npm run generate` behavior is unchanged.
- **Launcher `crates/ad-fidelity/package.json`** (`"generate": "node oracle/…"`)
  so `npm run generate` and `cargo run -p ad-fidelity` both launch from the crate
  root. Node resolves Playwright from `oracle/node_modules` regardless of cwd, so
  the oracle's install/deps stay put and the launcher carries no dependencies.
- Docs: oracle README + crate README (usage, the `--` forwarding gotcha for both
  npm and cargo, the id convention, the launcher, `git bisect run` note).

## Decisions & why
- **`trace` subcommand, not a `--debug` flag.** A flag that silently changed the
  positional's meaning (fixtures *dir* → single *file*) was too magic; a clap
  subcommand keeps grid vs. trace honestly separate. Grid mode stays the
  no-subcommand default via `args_conflicts_with_subcommands`.
- **Precedence path → `dir/id` → glob**, per discussion: a real/relative path is
  the least surprising thing to honor first.
- **Symmetric bare names in `saves/traces/`.** Both sides anchor there
  (`--trace t.json` ⇄ `trace t.json`), so a trace round-trips by name. The whole
  `saves/` tree is already git-ignored, so no new ignore rule.
- **Naive per-horizon replay for now.** `trace` re-decodes+re-ticks from scratch
  at each horizon (O(n²)); the single-pass snapshot version is deferred until we
  see whether it's actually too slow.
- **Non-zero exit on divergence** (consistent with grid), specifically so
  `git bisect run` and CI key off it; `2` is kept distinct for load/resolve
  errors.

## Deviations from the design doc
- None substantive. This is tooling/UX around the existing save-replay harness
  (design `2026-07-06-fidelity-testing.md` §6/§10); the fixture schema, tolerance
  model, and allowlist are unchanged. The dense trace is just a normal fixture
  with every horizon populated.

## Surprises & gotchas
- npm eats a bare `--save` (it's a real npm flag) and cargo rejects a bare
  `trace`/`--debug`; both need args after `--`. Documented prominently.
- Node resolves `require("playwright")` from the *script's* directory, not the
  cwd — which is what makes the crate-root launcher (`node oracle/…`) work without
  a second `node_modules`.
- The whole pipeline ran end-to-end for the first time this session (the game
  happened to be served): the oracle wrote a real 1000-tick trace of capture 1,
  and `ad-fidelity trace` scanned it clean — no divergence over 1000 ticks. The
  naive O(n²) scan returned instantly, so no optimization is warranted yet.

## Follow-ups
- Single-pass trace (replay once, snapshot per tick) if O(n²) proves slow.
- Optional: report each field's *own* first-divergence tick (deferred — it would
  surface independent bugs together, which we'd fix one at a time anyway).

## Tests
- `cargo test -p ad-fidelity` — 15 unit (7 new `resolve` cases) + 6 integration
  (3 new: self-consistent trace → no divergence; a corrupted horizon → first
  divergence at exactly that tick with an `antimatter` diff; missing-horizon
  `compare_at` errors). All pass; clippy clean.
- JS: `node --check` on the oracle script; `resolveId` verified against the real
  captures dir (`1`→`00001`, `33`→`00033`, `999`→not found); `--trace` without
  `--save` exits 2.
- End-to-end: `npm run generate -- --save 1 --trace t.json` from the crate root
  (via the launcher) produced a 1000-horizon fixture; `ad-fidelity trace t.json`
  reported no divergence over 1000 ticks (exit 0). Test artifact cleaned up.
