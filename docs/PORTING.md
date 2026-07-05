# Porting Guide

This project is a port of the original JavaScript Antimatter Dimensions. This
guide covers the fidelity standard we hold ports to and how to reference the
original game. For the mechanical "where do I add code" checklist, see
**Adding a Game System** in [`../AGENTS.md`](../AGENTS.md).

## Fidelity standard

When porting a system, aim for **behavioral fidelity** (same gameplay results)
rather than structural fidelity (same code organization). Idiomatic Rust that
reproduces the original's observable behavior beats a line-by-line translation.

Fidelity is verified with log-space relative tolerance (default 1e-10), since
floating-point arithmetic differs slightly between JS and Rust. See `ad-fidelity`.

## Porting roadmap (phases)

The project follows a phased approach (originally recorded in
[`design/2026-06-19-architecture.md`](design/2026-06-19-architecture.md) §9):

1. **Foundation:** `break_infinity` + basic `GameState`
2. **Core:** antimatter dimensions, tickspeed, dim boosts, galaxies, sacrifice
3. **First prestige:** infinity, infinity dimensions, normal challenges
4. **Second prestige:** eternity, time dimensions, time studies
5. **Mid-game:** replicanti, dilation, eternity challenges
6. **Reality:** glyphs, perks, celestials

## Referencing the original game

The original JS source is at `../antimatter-dimensions/src/core/`. Key
directories:

- `src/core/dimensions/` — Dimension classes
- `src/core/secret-formula/` — Game data/constants/configurations
- `src/core/game-mechanics/` — Base classes (Effect, Purchasable, etc.)
- `src/core/celestials/` — Endgame celestial mechanics
- `src/game.js` — Main game loop + prestige formulas

## UI fidelity

The UI should match the original game **exactly** — same layout, sizing, colors,
fonts, and styling. The frontend vendors the original game's stylesheets verbatim
(see `crates/ad-gui/frontend/public/stylesheets/`), so for any UI implementation,
consult the original game code to see how those stylesheets are applied: which
classes a component uses, the exact CSS values (widths, font-sizes, spacing), and
which CSS variables (e.g. `--color-accent`, `--color-good`) it references. The
original Vue components live in `../antimatter-dimensions/src/components/`. Prefer
reusing the vendored classes and variables over inventing new styles, and copy
concrete values from the original rather than guessing.

## Number formatting / notations

`src/core/format.js` holds only the thin wrappers (`format`, `formatInt`,
`formatX`, …). The actual notation strategies (Scientific, Engineering, Standard,
Letters, …) live in the external `@antimatter-dimensions/notations` package, not
in `src/core/`. Its source is the bundled dist (no `.ts` sources shipped):
`../antimatter-dimensions/node_modules/@antimatter-dimensions/notations/dist/ad-notations.esm.js`
(run `npm install` in `../antimatter-dimensions` if `node_modules` is absent).
