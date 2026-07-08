---
date: 2026-07-08
feature: fidelity
design_docs:
  - ../design/2026-07-06-fidelity-testing.md
---

# Fidelity — model the run-flags & reality peaks the exhaustive allowlist exposed

## Summary
Modelled the near-universal `player`-tree fields that the newly-exhaustive
allowlist (see `2026-07-07-fidelity-celestial-allowlist.md`) turned red on every
save: `postC4Tier`, the AD-purchase / AM-gain / sacrifice / galaxy
`requirementChecks` flags, and `records.thisReality.{maxAM,maxIP}`. This lifts the
suite from 0/312 to 32/312 (all early fixtures 0–7 recovered).

## What shipped
- **`postC4Tier`** — was a round-trip gap: `dimensions.rs` already set it on
  purchase, but the DTO hardcoded `1` and encode never wrote it. Now read from the
  save (default `0`, matching the original) and written back.
- **`requirementChecks` run-flags** — modelled and mirrored to their exact JS
  mutation sites:
  - `eternity.onlyAD8`/`onlyAD1`/`noAD1` — flipped in `on_buy_dimension`
    (tier ≠ 8 / ≠ 1 / == 1); `noAD1` also cleared each tick when AD1 has stock,
    mirroring `antimatter-dimension.js`.
  - `reality.noAM` — cleared in `tick` on any antimatter gain (`currency.js` add).
  - `infinity.noAD8` — cleared on an AD8 purchase.
  - `infinity.noSacrifice` — cleared in `sacrifice()`, restored in `galaxy_reset()`
    (per-Galaxy, not per-Infinity, per `galaxy.js`).
  - `infinity.maxAll` — a permanent latch set only by the manual Max All (never in
    replay), so round-tripped unchanged.
- **`records.thisReality.maxAM` / `maxIP`** — added to `ThisReality`, round-tripped,
  and peak-tracked in `tick` alongside the existing `thisInfinity`/`thisEternity`
  peaks (the original updates all three in the AM/IP setters).
- Plumbing: new `InfinityChecksDTO`, extended `EternityChecksDTO`/`RealityChecksDTO`/
  `ThisRealityDTO`/`PlayerDTO`, the `From<PlayerDTO>` conversions, and the encode
  writers.

## Decisions & why
- **Round-trip + mirror the mutation sites, not "change the default".** The fresh
  defaults are correct for a fresh save; the divergence came from the DTO not
  *reading* these fields (decode drops them, encode re-emits the template default).
  So each field is read/held/written, and any field a tick can change is updated at
  the same point the original does.
- **Scope: the near-universal gaps first.** These flip in essentially every save,
  so modelling them recovers whole fixtures at once. Verified against `ad-fidelity`
  after each batch (fixture 0 → all of 0–7).

## Deviations from the design doc
- None. Extends the code to match `player.js`; the design doc is unchanged.

## Surprises & gotchas
- `cargo test -p ad-core` fails to *compile* on default features because the `save`
  module (and its tests) are `#[cfg(feature = "serde")]`; run
  `cargo test -p ad-core --features serde` (29/29 pass).
- The remaining red cells split cleanly: the pre-existing **economy** divergence
  (`dimensions.antimatter`, downstream `antimatter`/`thisInfinity.maxAM`/…) caps the
  recoverable ceiling at the original 40, and the only *introduced* blocker left is
  the rate records below.

## Follow-ups
- **Deferred (explicitly): the IP/infinity rate records** — `thisEternity.
  bestInfinitiesPerMs`, `thisEternity.bestIPMsWithoutMaxAll`,
  `bestInfinity.bestIPminEternity`. They gate certain Infinity-Upgrade rewards, so
  they need *full* modelling, not just round-trip: reset to 0 on Eternity/Reality/EC,
  and the crunch-time `clampMin` updates in `bigCrunchUpdateStatistics`
  (`bestIPminEternity = clampMin(thisInfinity.bestIPmin)`; `bestInfinitiesPerMs`
  from `gainedInfinities()/clampMin(33, thisInfinity.realTime)`;
  `bestIPMsWithoutMaxAll` from `IP/clampMin(33, realTime)` when `!maxAll`). These
  are the last blocker for late fixtures 49/70.

## Tests
- `cargo test -p ad-core --features serde`: 29/29 pass.
- `cargo run -p ad-fidelity`: **32/312** (was 0 after the allowlist widening; was 40
  before it). Fixtures 0–7 fully green; 49/70 fail only on the deferred rate records.
- `cargo fmt` / `cargo clippy` clean (pre-existing unused-serde-helper warnings
  aside).
