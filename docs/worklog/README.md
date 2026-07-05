# Worklog

An **append-only** record of work sessions, written **after** coding. Each entry
captures what a session actually did — the narrative, decisions, and surprises
that git history and the (forward-looking) design docs don't carry.

## Rules

- **One file per work session or feature stage.** Never combine unrelated
  sessions into one file.
- **Append-only.** Add new files; **do not edit existing entries.** An entry is a
  snapshot of what was known and done at that time. (This immutability is
  deliberate: append-only new files mean parallel work never produces merge
  conflicts in the worklog.)
- **File name:** `YYYY-MM-DD-<slug>.md`, where the date is the day of the work.
  Include the feature id in the slug when it applies, e.g.
  `2026-07-05-feature-6.6e-automator-block-editor.md`.
- **Cross-link.** Link the design doc(s) the session implemented, and call out
  any **deviations** from them — that delta is the most valuable thing here,
  because it's where a design doc silently stopped matching the code.
- Record the *why* and the narrative, not the diff. Don't restate what `git log`
  already shows.

## Template

```markdown
---
date: 2026-07-05
feature: 6.6e
design_docs:
  - ../design/2026-07-05-automator.md
---

# <Feature / stage> — <short title>

## Summary
One or two sentences: what shipped this session.

## What shipped
- Concrete changes (modules added, behavior now working).

## Decisions & why
- Notable choices made during implementation and their rationale.

## Deviations from the design doc
- Where the implementation diverged from the linked design doc(s), and why.
  (If a design doc no longer reflects reality, update its `status` — do not
  rewrite its body.)

## Surprises & gotchas
- Anything non-obvious discovered along the way (original-game quirks, subtle
  bugs, fidelity mismatches and how they were resolved).

## Follow-ups
- Deferred pieces, known gaps, TODOs surfaced by this work.

## Tests
- What was added/run and the result.
```
