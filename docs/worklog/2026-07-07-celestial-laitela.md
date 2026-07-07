---
date: 2026-07-07
feature: 7.6
design_docs:
  - ../design/2026-07-07-laitela.md
---

# Celestial 6 — Lai'tela + Imaginary Upgrades

## Summary
Ported Lai'tela (Feature 7.6) end-to-end and, as its unlock gate, the Imaginary
Machines currency + the 25 Imaginary Upgrades (Feature 6.4's deferred half). The
Dark Matter Dimension economy, Singularities + the 30 milestones, Continuum, and
the entropy destabilization run all land, with engine + save/load + a Lai'tela
subtab.

## What shipped
- **`imaginary_upgrades.rs`:** Imaginary Machines (approach `baseIMCap` over real
  time), the 10 rebuyables (cost/effect) + 15 one-time upgrades (requirement-gated
  purchase; 15–18 seed the DMDs). Effects wired: iU8 (ID ×1e100000/buy), iU10
  (singularity gain), iU21 (annihilation mult), iU15 (Lai'tela + Continuum), iU19
  (annihilation).
- **`laitela.rs`:** `LaitelaState` — 4 Dark Matter Dimensions with real-time DM/DE
  production (`dmd_tick`), interval/powerDM/powerDE upgrades, ascension, and
  annihilation (`darkMatterMultGain`); Continuum (`ad_continuum_value` /
  `tickspeed_continuum_value` folded into the buy-10 seams; manual buys stay
  discrete otherwise); the entropy run (`laitela_reality_tick` → destabilization,
  difficulty tier, `maxAllowedDimension` disabling top AD/ID/TD tiers,
  `realityReward`).
- **`singularity.rs`:** condense DE → Singularities (cap `200·10^capIncreases`,
  the ±cap knob), the 30-milestone catalogue with `completions` (nerf-softcap +
  limit) and effect readers. DMD-internal milestones feed the DMD formulas;
  `gamespeedFromSingularities` and `glyphLevelFromSingularities` wired globally.
- **Save/load:** `LaitelaDTO` + the imaginary bits/rebuyables on the Reality DTO
  round-trip; three new run-scoped requirement checks (`reality_had_id1`,
  `reality_max_studies`, `reality_no_continuum`) tracked at the ID1/study/
  continuum sites and reset on Reality.
- **GUI:** a Lai'tela subtab (DM/DE/singularity header + condense + cap knob, the
  4 DMD panels, annihilation + continuum toggle + run + entropy/difficulty, the
  Imaginary Upgrade grid, and the milestone list).

## Decisions & why
- **Built Imaginary Upgrades now.** Lai'tela gates on iU15, so the iM currency +
  25 upgrades had to land here (Feature 6.4 had deferred them). The iM balance is
  re-earned from its cap each session rather than round-tripping the original's
  `1e10000×iM`-in-maxRM encoding (deep-endgame; documented).
- **Continuum via the linear branch.** `getContinuumValue`'s super-exponential
  branch needs the cost-scaling precalc internals; the linear branch (`1 +
  (log10(money) − log10(base))/log10(mult)`) is exact in the frontier region and
  is what's ported. Documented as an approximation.
- **`unpredictability`/autobuyer/tesseract milestones stored, effects deferred.**
  Their targets are unbuilt or inert; the milestone `completions` still compute.

## Deviations from the design doc
- The deep imaginary-upgrade requirements (11–14, 22–24) never auto-satisfy (they
  need records we don't track); their bit + cost are modelled.
- iM save encoding not round-tripped (see above); the bits + rebuyables are.

## Tests
- New `ad-core` unit tests across `laitela` (DMD production, annihilation,
  destabilization, continuum), `singularity` (condense, milestone completions,
  unique one-shot), and `imaginary_upgrades` (rebuyable scaling, iU15 unlock +
  DMD seed), plus the Lai'tela fields in the save round-trip. `cargo test -p
  ad-core --features serde` = 450 lib pass; `cargo clippy` clean; `cargo check -p
  ad-gui` + frontend `npm run build` green.

## Follow-ups
- Continuum super-exponential branch; the DMD/ascension/annihilation/condense
  autobuyers; the deep imaginary-upgrade requirements; tesseract/`boundless`/
  `multiversal` effects. Imaginary Upgrade 25 (unlock Pelle) and the Ra
  `disabledByPelle` guards land with Pelle (7.7).
