---
status: Implemented
feature: "3.2"
---

# Replicanti — Feature 3.2

Self-replicating entities unlocked with Infinity Points. They multiply all Infinity
Dimensions and can be converted into **Replicanti Galaxies** (which act like
Antimatter Galaxies for the tickspeed formula). Replicanti persist across a Big
Crunch (reset only on Eternity, later).

Original: `core/replicanti.js`. **Pre-Eternity the mechanic simplifies sharply**:
`isUncapped` (TS192/Pelle) is false, so Replicanti are always capped at
`Number.MAX_VALUE` and the over-cap interval scaling never runs; the speed
multiplier and all `extra` terms (time studies / achievements / glyphs) are ×0/×1.

---

## 1. State (`player.replicanti`)

```rust
pub struct ReplicantiState {
    unlocked: bool,
    amount: Decimal,        // capped at 1.8e308
    timer_ms: f64,          // sub-interval accumulator
    chance: f64,            // 0.01, cap 1.0    (upgrade 1)
    chance_cost: Decimal,   // 1e150, ×1e15/buy
    interval_ms: f64,       // 1000, cap 50     (upgrade 2)
    interval_cost: Decimal, // 1e140, ×1e10/buy
    galaxies: u32,          // Replicanti Galaxies made
    galaxy_cap: u32,        // max RGs (upgrade 3; `boughtGalaxyCap`)
    // gal_cost is derived from galaxy_cap (below), not stored.
}
pub replicanti: ReplicantiState,   // on GameState
```

**Unlock:** costs `1e140` IP (`Replicanti.unlock`); sets `unlocked`, `amount = 1`,
`timer = 0`.

---

## 2. Growth (`replicantiLoop`, simplified)

Interval = `interval_ms` (speed mult = 1; no ×10 without TS133; no over-cap scaling
since we stay capped). Each tick, a replicanti reproduces with probability `chance`,
so over `diff` ms:

```
ticks   = (diff + timer) / interval
timer   = ticks < 100 ? frac(ticks)·interval : 0
amount  = min(amount · (1 + chance)^floor(ticks),  REPLICANTI_CAP)   // 1.8e308
```

Use `Decimal::pow` for `(1+chance)^ticks` (overflows f64 for fast intervals; the cap
then clamps). This continuous approximation is the JS "fast gain" path; the
binomial/Poisson randomness at tiny amounts is dropped (a faithful aggregate).

---

## 3. Replicanti Galaxies

- **canBuyMore**: `amount ≥ 1.8e308 && galaxies < galaxy_cap`.
- **buy** (`replicantiGalaxy`, manual — the autobuyer is Eternity-gated): `timer = 0`,
  `amount = 1`, `galaxies += 1`, then `addReplicantiGalaxies`: `dim_boosts = 0` and a
  `dim_boost_reset` (soft reset of dims/tickspeed/AM/sacrifice, keeping antimatter
  galaxies). So an RG is an antimatter-galaxy-like reset that also grants a free
  galaxy for tickspeed.
- **tickspeed**: RGs count as galaxies in `effectiveBaseGalaxies`. Add
  `effective_galaxies() = galaxies + replicanti.galaxies` and use it **only** in the
  tickspeed purchase multiplier (not the galaxy/boost *requirements*).

---

## 4. Multiplier to Infinity Dimensions

`replicantiMult = log2(max(amount, 1))^2`, clamped to ≥ 1 (`replicantiMult()`,
dropping the TS/glyph terms). Folded into `id_common_multiplier` when
`unlocked && amount > 1`.

---

## 5. Upgrades (bought with IP)

| # | field | effect | cost | ×/buy | cap |
|---|-------|--------|------|-------|-----|
| 1 | chance | `+0.01` | `chance_cost` (1e150) | ×1e15 | 1.0 |
| 2 | interval | `×0.9 ms` | `interval_cost` (1e140) | ×1e10 | 50 ms |
| 3 | galaxy_cap | `+1` max RG | derived (below) | — | — |

Galaxy-cap cost (derived from `galaxy_cap = count`, ignoring the ≥100 "distant" /
≥1000 "remote" terms that are far past our frontier):
`gal_cost = 10^(170 + 25·count + 5·count·(count−1)/2)`.

---

## 6. Reset / save

- **Big Crunch**: Replicanti are **not** reset (only Eternity resets them, later).
- **Save**: `player.replicanti.{unl, amount, timer, chance, chanceCost, interval,
  intervalCost, galaxies, boughtGalaxyCap}` → the state; `galCost` is derived on
  load. All present in the template.

---

## 7. UI

A **Replicanti** tab (top-level, shown once `unlocked` or IP ≥ 1e140): the unlock
button, amount + its ID multiplier, chance/interval readouts, the 3 upgrade
buttons, and the Replicanti Galaxy button (count / max). Vendored `replicanti`
styling. Commands + snapshot.

---

## 8. Incremental plan

1. **Engine**: state, unlock, the growth loop, RG buy + tickspeed integration, the
   3 upgrades, the ID multiplier, save/load. Tests. Commit.
2. **UI**: the Replicanti tab. Commit.

## 9. Open questions

- `scaleFactor`/over-cap growth: unreachable pre-Eternity (always capped); omitted.
- The Replicanti-Galaxy / upgrade autobuyers are Eternity-milestone gated; deferred.

*Document generated: 2026-07-03.*
