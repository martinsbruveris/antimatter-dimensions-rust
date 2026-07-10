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
| 2026-07-09 | (pending) | Big Crunch now resets `thisInfinity` selectively (keeps `bestIPminVal`, zeroes `bestIPmin` only at the goal) and runs `resetRequirements("infinity")` (clears `maxAll`/`noSacrifice`/`noAD8`) — `00124` | 606/1148 |
| 2026-07-09 | (pending) | Applied the Infinity Challenge 8 completion reward to AD 2–7 (`(AD1×AD8)^0.02`) and made Infinity Dimension production compound within a tick (was snapshotting all rates up front); together they cut `00241`'s rebuild divergence from ~743 orders to ~1e-4/tier | 607/1148 |
| 2026-07-09 | (pending) | Advanced the records time (`thisInfinity.time` etc.) before Dimension production instead of at the end of the tick, matching the game loop; time-based AD achievement multipliers (56/76/91/92) and IC8 decay now read this tick's time (fixed the general ~2e-4/tier AD-chain drift) | 706/1148 |
| 2026-07-09 | (pending) | Capped pre-break Antimatter Dimension production at 1e315 (`cappedProductionInNormalChallenges`); bounds the Big-Crunch goal overshoot (`maxAM`/`totalAntimatter`) that was running away on repeatedly-crunching wall fixtures (`00079`/`00081`–`00085`) | 715/1148 |
| 2026-07-09 | (pending) | Removed the 0.01 clamp on the Tickspeed per-purchase multiplier for ≥3 galaxies (the original only floors the <3 branch); deep-galaxy tickspeeds were e1888 too slow, driving the residual per-tier AD-chain drift on steady-state fixtures (`00232`) | 812/1148 |
| 2026-07-09 | (pending) | Fixed the Dim Boost autobuyer `lastTick`: a Galaxy reset (which precedes Dim Boost's own `advance`) now targets the pre-increment realTimePlayed so `lastTick` lands on 0 not −dt (`00118`); and a buy-max Dim Boost emits `DIMENSION_BOOST` not `INFINITY`, so it no longer zeroes the Galaxy autobuyer's phase (`00126`/`00136`) | 839/1148 |
| 2026-07-09 | (pending) | IC3 now neutralises only the Tickspeed *per-purchase* multiplier (moved into `tickspeed_purchase_multiplier`), not the whole production factor; the base tickspeed's Achievement effects (36/45/66/83) survive, so `tickspeed_effect = 1/startingTickspeedMult` (≈1.062 for `00146`) instead of 1 — fixed the residual ~1.8e-4/tier AD drift on IC3 fixtures (`00146`/`00147`) | 844/1148 |
| 2026-07-09 | (pending) | Modelled the `recentInfinities` ring + `bestRunIPPM` and added the missing second passive-IP term (`BreakInfinityUpgrade.ipGen`: `bestRunIPPM · infinityRebuyables[2]/20 · diff/60000` per tick); `infinityPoints` was the most common diverged field (~245). Ring updated on crunch, cleared on Eternity/EC/Reality (`00267`) | 1090/1148 |
| 2026-07-09 | (pending) | Moved `updateNormalAndInfinityChallenges` after the autobuyer pass + records-time increment (was before) so an NC2 purchase zeroes `chall2Pow` *before* its one-tick regrowth (`00133`); and widened the matter-growth gate to `NC11 \|\| IC6` (annihilation stays NC11-only) so IC6 grows matter (`00203`) | 1094/1148 |
| 2026-07-09 | (pending) | Made `buy_until_10_dimension` an atomic group buy (pay `costUntil10` up front, one `on_buy_dimension`) instead of looping single buys; under NC4+NC6 (both active in IC1) the per-buy NC4 erase was wiping AD4's NC6 currency (AD2) after the first buy, stalling the group at 1 vs the original's 10 (`00133`) | 1099/1148 |
| 2026-07-09 | (pending) | Ported `buyMaxTickSpeed`'s closed form: the analytic `getMaxBought` (charging only the top purchase) replaces the repeated-single-buy loop, with the original's NC9 purchase-by-purchase branch; `buyTickSpeed` now carries the full `isAvailableForPurchase` guard | 1118/1148 |
| 2026-07-10 | (pending) | Wired the achievements tail (Feature 2.4): conditions 35/61/62/65/74/111/117/156/165/172 + row 18, effects 126/133/138/156/168/171/175/183/187, the `noPurchasedTT`/`noTriads` requirement flags, and the AD-autobuyer `upgradeBulk` purchase path | 1121/1148 |
| 2026-07-10 | (pending) | A third fixture batch landed (82 new late-game saves; the grid is now 369×4). The original 287-fixture range still passes 1121/1148 after the 6.2–6.7 feature work; the new fixtures are fresh late-game coverage to chase | 1172/1476 |
| 2026-07-10 | (pending) | Galaxy autobuyer now resets its phase (`lastTick`) on every ready tick even at the `maxGalaxies` cap — its `canTick` never tested the limit (only `requestGalaxyReset` caps the purchase); also added the missing `Galaxy.canBeBought` past-the-Infinity-goal gate to `can_buy_galaxy` (`00072`–`00077`) | 1186/1476 |
| 2026-07-10 | (pending) | Added the missing `Currency.antimatter.lt(Player.infinityLimit)` gate to `can_sacrifice`; inside IC2 (auto-sacrifice, goal `1e10500`) production freezes at the goal, so Rust was sacrificing (zeroing frozen dims) past the point the original stops (`00144`) | 1187/1476 |
| 2026-07-10 | (pending) | Replicanti sub-interval timer now rolls over via `total − whole·interval` instead of `(ticks − whole)·interval`; the f64 `(total/interval)·interval` round-trip drifted the timer below the integer and missed an interval boundary by one tick, and the Replicanti→RG→tickspeed path spread that into a ~1e-3 per-tier AD-chain drift (`00222`) | 1188/1476 |
| 2026-07-10 | (pending) | Free Tickspeed grant (`totalTickGained` from Time Shards) now runs after AD production, not inside `tick_time_dimensions` (before it); the original updates it after `AntimatterDimensions.tick`, so a tickspeed upgrade earned this tick only speeds AD from the next — Rust applied it a tick early, a ~0.05%/step AD-chain drift (`00244`) | 1198/1476 |

## Method

Run fidelity suite against fixture set via

```sh
cargo run -q -p ad-fidelity -- crates/ad-fidelity/saves/fixtures
```

Append new count-changing commits to the table as fidelity work continues.
