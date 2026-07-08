---
date: 2026-07-07
feature: fidelity
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity — make the comparison allowlist exhaustive over engine-relevant fields

## Summary
Rewrote `ad-fidelity`'s comparison allowlist (`src/allowlist.rs`) to be exhaustive
over **every engine-relevant persisted field** — the Phase-7 Celestials + Imaginary
Upgrades and the previously-omitted core/records/requirement/autobuyer fields —
*including* fields `ad-core` does not model yet, so the suite showcases those gaps
rather than hiding them. Backed by a full field-by-field census of the player tree.

## What shipped
- New **Celestials** section: Teresa/Effarig relic economy + unlock/run state;
  Enslaved storage (`stored`, toggles, `unlocks`, gates `hasSecretStudy`/
  `feltEternity`/`progressBits`); V unlock/run/`goalReductionSteps`/`STSpent`/
  `runRecords`; Ra pets (level, Memories, Chunks, upgrades), `charged`/`disCharge`/
  `peakGamespeed`/`petWithRemembrance`, alchemy, `highestRefinementValue`; Lai'tela
  dark-matter economy + Dark Matter Dimensions (+ `upgrades`); Pelle remnants/
  reality-shards/records/upgrades/rebuyables/rifts/Galaxy Generator; `isGameEnd`.
- `reality.imaginaryUpgradeBits`/`imaginaryRebuyables` added to the Reality section,
  plus the reality gaps (`imaginaryMachines`, `iMCap`, `imaginaryUpgReqs`,
  `createdRealityGlyph`, `unlockedEC`, `partEternitied`, `partSimulatedReality`).
- Core-economy gaps (`ic2Count`, `partInfinitied`, `postC4Tier`, `IPMultPurchases`),
  Infinity gaps (`infinity.upgradeBits`, `challenge.normal.{current,completedBits}`),
  `blackHoleNegative`, the full `requirementChecks.*` run-flag/peak family, the
  record peers/rate-records (`thisReality.{maxAM,maxIP}`, `best*PerMs`, `bestRSmin*`,
  `bestInfinity.bestIPmin*`, `fullGameCompletions`, …), and the unmodelled autobuyer
  subsystems (`auto.{infinityDims,timeDims,replicantiUpgrades,dilationUpgrades,
  blackHolePower,realityUpgrades,imaginaryUpgrades,…}` `isActive` toggles).
- A `leak(String) -> &'static str` helper so per-Ra-pet and per-Pelle-rift paths
  (object-keyed, no array wildcard) compose in a loop; the list is built once.
- README + module doc rewritten to the "showcase the gaps" policy and the flat-list
  consequence.

## Decisions & why
- **Include test is "engine-relevant at full fidelity", not "already modelled".**
  An earlier draft of this change scoped the allowlist to *only* the fields
  `encode.rs` overlays from state, to avoid false divergences. That was reversed
  mid-session after clarifying the harness's purpose: unmodelled fields are
  *temporary gaps the harness should showcase*, not hide. The encoder overlays
  modelled fields onto a fresh-start template, so an unmodelled field is emitted
  as its fresh default — a rule over it diverges as `Rust = default` vs
  `JS = real`, which is exactly the signal we want. A full field-by-field census
  of the player tree drove the final list (every leaf classified include/exclude
  with a reason).
- **Flat list, no `Modelled`/`Gap` kind (user's call).** Once the gaps were added,
  the grid went 40/312 → **0/312**: a few gaps (`postC4Tier`, the
  `requirementChecks` run-flags, `records.thisReality.maxAM`) are non-default in
  essentially every real save, so every cell carries ≥1 known-gap divergence. The
  alternative — tagging each rule `Modelled`/`Gap` so the pass count tracks only
  modelled rows — was considered and declined in favour of maximum gap visibility.
  The module doc records how to reintroduce the split if the noise ever wins.
- **Resources vs. clocks.** Real-time/game-time *accumulators that act as clocks*
  are skipped as bookkeeping (design §5): Teresa `timePoured`, Enslaved
  `storedReal`, Ra `momentumTime`, Lai'tela `thisCompletion`/`fastestCompletion`
  and the DMD `timeSinceLastUpdate`, and V `runRecords`. Resources that merely
  *accrue* on a real-time schedule (Ra Memories, Dark Matter/Energy, Relic Shards)
  are mechanics and are compared like the rest of the economy. Enslaved `stored`
  is banked *game* time — a spendable resource — so it is compared; `storedReal`
  (banked *real* time) is not.
- Array/map length parity verified against both `default_player.json` and the JS
  `player.js` defaults so structural `Exact` compares hold: imaginary rebuyables
  10↔10, `ra.alchemy` 21, `laitela.dimensions` 4, Pelle rifts/rebuyables keys.

## Deviations from the design doc
- The design doc's §5 allowlist table predates the Celestials and does not
  enumerate them; this session extends the *code* allowlist beyond that table.
  The design doc is historical and was left unchanged (status still Accepted).

## Surprises & gotchas
- The template-overlay encode model shapes what a divergence *means*, not what may
  be listed: a modelled field that diverges is a formula bug; an unmodelled
  (fresh-default) field that diverges is a missing-model gap. The roundtrip
  identity guard (design §6) separates the two, so listing gap fields is safe and
  informative rather than misleading.

## Follow-ups
- The suite is all-red until the listed gaps are modelled. Priority gaps (drive the
  most cells red): `postC4Tier`, the `requirementChecks.*` run-flags, and
  `records.thisReality.maxAM` — modelling these would recover most of the grid.
- No *celestial* coverage is exercised yet: all 78 captured fixtures are early-game,
  so the celestial rules sit at defaults. Real coverage needs late-game captures
  (Ra/Lai'tela/Pelle unlocked).
- If gap-noise ever masks a real regression, add a per-rule `kind` (`Modelled`/
  `Gap`) and count only `Modelled` toward pass/fail (see module doc).

## Tests
- `cargo test -p ad-fidelity`: 6/6 pass (incl. `roundtrip_identity_holds_over_
  allowlist`, confirming the expanded list preserves decode→encode identity on the
  fresh save it uses).
- `cargo run -p ad-fidelity`: **0/312** (was 40/312). The drop is deliberate
  coverage widening — every remaining divergence checked via `--verbose` is a
  legitimate missing-model gap (e.g. `postC4Tier` JS=1/Rust=0), not a comparison-mode
  false positive. Recorded in the fidelity-suite tracker.
- `cargo fmt` / `cargo clippy -p ad-fidelity`: clean.
