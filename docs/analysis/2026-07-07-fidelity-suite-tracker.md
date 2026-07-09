# Fidelity suite tracker

Progress of the `ad-fidelity` save-replay suite (design:
[`../design/2026-07-06-fidelity-testing.md`](../design/2026-07-06-fidelity-testing.md)),
measured as the number of passing **grid cells** across the commit history.

A *cell* is one (fixture × horizon) comparison. The current suite uses 4 horizons 
(1, 10, 100, 1000 ticks) and the number of fixtures will grow over time. A cell
passes when every allowlisted `player` field matches the JS oracle within the
log-space tolerance. As of 2026-07-09 the default is **`1e-4`**: raised from
`1e-6` to absorb the accumulated-rounding / catastrophic-cancellation drift
between Rust `f64` and V8 (a `libm`/fdlibm `pow` swap flips zero cells, so the
residue is `Decimal` add/normalize + multiplication-order noise, not `pow`), so
the suite tracks *structural* divergences rather than floating-point noise.

## Progress

Commits that don't change the count are omitted.

| Date | Commit | What changed | Passing cells |
| ---------- | --------- | ------------------------------------------------------------- | ------------- |
| 2026-07-07 | `923c2ea` | Introduced the save-replay harness (capture → JS oracle → Rust diff) — baseline | 20 |
| 2026-07-07 | `fb05716` | Fixed AD production (dimension→dimension feed ran 10× too fast) and the "Buys max" group/bulk buying | 35 (+15) |
| 2026-07-07 | `21d2f2a` | Restored the autobuyer timer phase from the save's `lastTick` on load (was reset to 0) and aligned `advance` with the JS pre-increment interval check | 38 (+3) |
| 2026-07-07 | `8dcda3e` | Gated each interval autobuyer's timer reset on its `canTick` readiness, so phase accrues while waiting to afford instead of restarting each interval | 39 (+1) |
| 2026-07-07 | `650d255` | Rounded the dimension amount after a "Buys max" group purchase (mirroring `buyUntilTen`), dropping lingering fractional production stock | 40 (+1) |
| 2026-07-07 | `1bff354` | Widening coverage, verifying ~60 more fields per cell. Fixed some small issues involving non-modelled fields. Obtaining new baseline. | 32 (−8) |
| 2026-07-09 | (batch 1) | Wired normal achievements 31–54 (conditions + effects): sacrifice exponent (32), starting antimatter (37/54), tickspeed base (36/45), AD multipliers (31/34/43/48) | 34 (+2) |
| 2026-07-09 | (pending) | Froze AD production once the Big Crunch goal is reached (pre-break / antimatter challenge) and recorded `maxAM` from the pre-cap overshoot, mirroring `AntimatterDimensions.tick`'s early return. (Also fixed achievement 28 firing on the bulk "buy max" AD1 path — count-neutral.) | 36 (+2) |
| 2026-07-09 | (pending) | Modelled the best-rate records `thisEternity.bestInfinitiesPerMs` and `bestInfinity.bestIPminEternity` (decode/encode + the `bigCrunchUpdateStatistics` update + Eternity/Reality/EC resets); previously dropped on decode | 93 (+57) |
| 2026-07-09 | (pending) | Modelled Normal Challenge best times (`challenge.normal.bestTimes`, 11 entries): decode/encode + the crunch `updateChallengeTime` step; previously dropped to the `f64::MAX` sentinel | 95 (+2) |
| 2026-07-09 | (pending) | Modelled `thisEternity.bestIPMsWithoutMaxAll` (decode/encode + the guarded `bigCrunchUpdateStatistics` update); previously dropped on decode | 130 (+35) |
| 2026-07-09 | (pending) | Accrued autobuyer timers while globally disabled so the derived `lastTick` stays fixed (was drifting +1 tick/frame), mirroring the original's `timeSinceLastTick = realTimePlayed - lastTick` | 160 (+30) |
| 2026-07-09 | (pending) | Modelled the Dim Boost / Galaxy autobuyer limit config (`limitDimBoosts`/`maxDimBoosts`/`limitUntilGalaxies`/`galaxies`, `limitGalaxies`/`maxGalaxies`/`buyMax`): decode/encode + readiness gating (stops the over-boost in `00061`) | 187 (+27) |
| 2026-07-09 | (pending) | Decoded/encoded the Tickspeed autobuyer's `BUY_MAX` mode (100, distinct from AD `BUY_10`=10); was never decoded and mis-encoded | 193 (+6) |
| 2026-07-09 | (pending) | Modelled the `ipMult` ×2 Infinity Upgrade (`IPMultPurchases`): decode/encode, `2^purchases` in `total_ip_mult`, and the Eternity/Reality resets | 195 (+2) |
| 2026-07-09 | (pending) | Applied the autobuyer `resetTick` (`lastTick`→0) on each prestige event and reset `postC4Tier` in `reset_challenge_stuff` — the missing prestige-reset behaviour (found via a dense `00059` trace) | 203 (+8) |
| 2026-07-09 | (pending) | Added a second fixture batch (Infinity→early-Eternity), raised default tolerance to 1e-4, and modelled the super-exponential `ExponentialCostScaling` for AD/Tickspeed costs past `Number.MAX_VALUE` (`00086`: 28 diverged fields → 1) | 303/1148 |
| 2026-07-09 | (pending) | Preserved `ipOffline`; aligned num-field epsilon to 1e-4; modelled the early-Eternity per-tick accumulators `partInfinitied` (passive `infinitiedGen`), `reality.maxID1` (bool→Decimal peak), and the `ic2Count` if/else timer | 350/1148 |
| 2026-07-09 | (pending) | Preserved `player.ic2Count`; modelled the Eternity best-rate records `thisReality.bestEternitiesPerMs` and `bestEternity.bestEPminReality` (decode/encode + the Eternity update + Reality reset) | 420/1148 |
| 2026-07-09 | (pending) | Produced Time → Infinity → Antimatter Dimensions each tick (game.js order) so AD production reads *this* tick's Infinity Power (was stale) | 424/1148 |
| 2026-07-09 | (pending) | Implemented the `slowestChallengeMult` Break Infinity Upgrade (`clampMin(50 / worstChallengeMinutes, 1)`, cap 3e4) from the decoded Normal Challenge best times; was stubbed as deferred, missing a ~24× AD multiplier on post-break saves (`00119`) | 574/1148 |
| 2026-07-09 | (pending) | Implemented the Dimension Boost autobuyer's `autobuyMaxDimboosts` buy-max branch (gate `canUnlockNewDimension \|\| galaxyCondition`, `maxBuyDimBoosts`, `softReset(bulk)`) + the `infinityLimit`/`infinityGoal` boost guards; Rust was boosting (and collapsing antimatter) when JS's buy-max branch would not (`00204`/`00206`) | 580/1148 |
| 2026-07-09 | (pending) | Modelled the AD autobuyer group toggle (`auto.antimatterDims.isActive`) + `collapseDisplay` (all tiers maxed/unlocked + Achievement 61); Rust ran the tier autobuyers when JS's collapsed group toggle disabled them (`00184`/`00185`) | 584/1148 |
| 2026-07-09 | (pending) | Ran the Galaxy autobuyer before the Dim Boost one (original `singleComplex` order) so a galaxy pre-empts a boost at a shared threshold, and made `resetTickOn` buy-max-aware (`INFINITY`, not `ANTIMATTER_GALAXY`) — `00129`/`00154` | 602/1148 |

## Method

Run fidelity suite against fixture set via

```sh
cargo run -q -p ad-fidelity -- crates/ad-fidelity/saves/fixtures
```

Append new count-changing commits to the table as fidelity work continues.
