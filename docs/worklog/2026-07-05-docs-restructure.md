---
date: 2026-07-05
topic: documentation restructure
---

# Documentation restructure — `docs/` tree, slim `AGENTS.md`, status front-matter

## Summary
Reorganised the project's documentation around a `docs/` tree and a clear
temporal model (living reference vs. historical design/worklog). Split the
oversized `AGENTS.md` into a router plus living architecture docs, moved the
design docs under `docs/`, added status front-matter to every design/analysis
doc, and repointed all stale references. No design doc drove this — the structure
was agreed in conversation this session.

## What shipped
- **`docs/` tree created:** `README.md` (index + conventions), `ARCHITECTURE.md`
  (living crate overview, dependency graph, principles), `PORTING.md` (fidelity
  method + referencing the original game), `worklog/` (README + template).
- **`AGENTS.md` slimmed 409 → 189 lines** to a router: overview, documentation
  map, build/test, code style, number system, add-a-system checklist, a rewritten
  "Updating Documentation" section (three-bucket model + cross-linking), testing.
- **`CLAUDE.md` added** — a shim that imports `@AGENTS.md` so Claude Code loads it.
- **Per-crate file maps:** the ~170-line "Key Source Files" block moved to
  `crates/ad-core/ARCHITECTURE.md`; `crates/ad-sim/ARCHITECTURE.md` added.
- **Design docs moved** (by the user) into `docs/design/` (34 RFCs) and
  `docs/analysis/` (6 studies of the original JS); `docs/research/` retained.
- **Status front-matter** added to all 40 design/analysis docs, seeded from the
  code-level port audit (`docs/design/2026-07-05-port-audit.md`) and reviewed:
  18 Implemented, 10 Reference, 8 Partial, 3 Proposed, 1 Rejected.
- **References repointed:** ~40 files had `design-docs/2026-*` pointers rewritten
  to `docs/design/2026-*` — Rust doc-comments across `ad-core`/`ad-format`,
  the Vue/JS frontend, `ad-gui/AGENTS.md`, `ad-gui/src/main.rs`.
- **`docs/README.md` index** rebuilt as complete Design + Analysis tables with a
  Status column.

## Decisions & why
- **Three doc tiers by *when* they're written, not by topic.** Living reference
  (`ARCHITECTURE.md` files — kept current) vs. design RFCs (before coding,
  historical) vs. worklog (after coding, append-only). Key correction to the
  original framing: we do **not** try to keep 40 dated design docs "up to date" —
  that's a treadmill. The living truth is the `ARCHITECTURE.md` files; design docs
  are historical and only ever get `status`/checkbox edits.
- **Per-crate `ARCHITECTURE.md` for locality.** The file map lives next to the
  code it describes, so it's more likely to be updated, and it keeps the huge
  block out of the always-loaded `AGENTS.md`.
- **`status` front-matter as the cheap "is this still current?" signal** instead
  of rewriting historical docs. Added a `Reference` value for analysis/review/
  exploration docs that have no single feature to "complete."
- **Append-only worklog** (new files, never edit existing) — deliberately
  immutable so parallel work never produces merge conflicts here.
- **Hand-maintained index** (not generated) — the codebase is small enough;
  revisit if it drifts.
- **`AGENTS.md` stays a router.** Number System and the add-a-system checklist
  are kept in it (small, high-touch); the porting *methodology* moved to
  `PORTING.md`.

## Surprises & gotchas
- **`AGENTS.md` was not in the agent's auto-loaded context this session** — only
  `MEMORY.md` was; the file had to be opened manually. That's the evidence that
  motivated the `CLAUDE.md` → `@AGENTS.md` shim. Still needs confirming next
  session (see follow-ups).
- **macOS default bash is 3.2** (no associative arrays) — the front-matter script
  used a portable `apply()` function + `sed` with a `#` delimiter to avoid
  clashing with the `/` in paths.
- **One `design-docs/` reference left on purpose:** the repo-tree diagram in
  `docs/design/2026-06-19-architecture.md` is a historical snapshot of the old
  layout, not a cross-reference, so the global rewrite (scoped to
  `design-docs/2026-*`) correctly skipped it.
- **The port audit (dated today) was the authoritative status source** — far
  better than guessing per doc. User confirmed the uncertain ones:
  `config-driven-engine` + `break-eternity-representation` = Proposed, the two
  test plans = Proposed/Partial, `break-infinity` = Implemented.

## Follow-ups
- **Verify the `CLAUDE.md` `@AGENTS.md` import actually loads** in the next Claude
  Code session (the whole restructure assumes the agent reads `AGENTS.md`).
- Backfill `ARCHITECTURE.md` for the other crates (`break_infinity`, `ad-format`,
  `ad-fidelity`, `ad-python`) lazily, as they're touched.
- If the hand-maintained index starts to drift, switch to generating it from the
  front-matter (currently the deliberate simple option).
- Revisit statuses as work continues (Partial → Implemented, etc.). Per the port
  audit, achievements are the biggest "looks done, isn't" gap (rows 4–18 unwired)
  — not a docs task, but the highest-leverage correctness item it surfaced.

## Tests
- `cargo check --workspace` — green. The source edits were comment-only
  (doc-comment path strings); no behavioural code changed.
