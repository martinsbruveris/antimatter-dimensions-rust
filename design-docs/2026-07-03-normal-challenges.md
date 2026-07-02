# Normal Challenges — Feature 2.5

The 12 Normal Challenges: constrained pre-Infinity runs that each modify the
production rules. Completing one (by reaching Infinity under its restriction)
unlocks the next and grants a permanent reward — an autobuyer. This is the true
next feature after Infinity Upgrades (2.2): Break Infinity (2.3) is gated behind
completing **Normal Challenge 12** (see the ordering note in
[`2026-06-23-feature-decomposition.md`](./2026-06-23-feature-decomposition.md) §2.3).

Original source: `core/normal-challenges.js` (state machine + the matter/chall-pow
tick), `secret-formula/challenges/normal-challenges.js` (data), and the
`NormalChallenge(N).isRunning` sites scattered through
`dimensions/antimatter-dimension.js`, `tickspeed.js`, `dimboost.js`, `galaxy.js`,
`sacrifice.js`.

---

## 1. State machine

Original `player.challenge.normal`:
- `current` — active challenge id (0 = none).
- `completedBits` — bitmask, bit `1 << id`.
- `bestTimes[id-2]` — fastest completion time per challenge (records; deferred).

Rust (`GameState`):
```rust
pub challenge: NormalChallengeState,   // { current: u8, completed: u16 }
```
- `current == 0` → not in a challenge; `1..=12` → running that one.
- `completed` bit `1 << id`.

Transitions (mirror `NormalChallengeState`):
- **start(id)**: a forced Big Crunch (`bigCrunchReset(true, true)` — reset **with**
  IP gain), then `current = id`. Challenge 1 can't be "started" (it is the base
  game); starting the already-active one is a no-op.
- **complete**: on any Big Crunch *while in a challenge*, set the completed bit and
  fire the reward (autobuyer unlock). NC1 auto-completes on the first-ever Infinity
  even outside a challenge (`handleChallengeCompletion`).
- **exit**: `current = 0`, forced Big Crunch **without** IP gain
  (`bigCrunchReset(true, false)`).
- **goal**: `Number.MAX_VALUE` (= our `BIG_CRUNCH_THRESHOLD`); reaching Infinity is
  the win condition (`this_infinity.max_am >= goal`, already tracked in `Records`).
- **isUnlocked**: `infinities >= lockedAt`. Challenges 1–9 unlock at 0 infinities
  (i.e. as soon as the Challenges tab is open); **C10 at 16**, C11/C12 at their own
  thresholds (verify in code during implementation).

The **Challenges tab** itself unlocks after the first Infinity
(`Tab.challenges.isUnlocked` — confirm the exact gate; likely
`PlayerProgress.infinityUnlocked()`).

Reset semantics: enter/exit/complete all route through the existing Big-Crunch
reset (`big_crunch`), which already resets pre-Infinity progress and awards IP. The
forced variant (`forced = true`) must reset even below the threshold — our
`big_crunch` currently early-returns unless `can_big_crunch`; a `forced` path is
needed (reset without the threshold check, IP awarded only when actually at goal).

---

## 2. The 12 modifiers

Each is a rule bend active while `current == id`. The clean Rust shape is a
computed `ActiveModifiers` (architecture doc §5.5) folded from `current`, read at
each rule site — *not* scattered `if challenge == N` across the engine.

| NC | Restriction | Key formula / site | Reward |
|----|-------------|--------------------|--------|
| 1 | none (tutorial) | — | 1st AD autobuyer |
| 2 | buying an AD/tickspeed halts **all** AD production; recovers over 3 min | `chall2Pow`: set 0 on buy, `+= diff/100/1800` capped at 1; multiplies AD production | 2nd AD autobuyer |
| 3 | 1st AD heavily weakened, but an uncapped exp. multiplier that resets on boost/galaxy | `chall3Pow *= 1.00038^(diff/100)`; applied to AD1; reset on boost/galaxy | 3rd AD autobuyer |
| 4 | buying an AD erases all lower-tier ADs | on buy tier t: `resetAmountUpToTier(t-1)` | 4th AD autobuyer |
| 5 | tickspeed purchase mult starts at 1.080 (vs 1.1245) | pre-3-gal `baseMultiplier = 1/1.08`; 3+ `baseMultiplier = 0.83` | 5th AD autobuyer |
| 6 | upgrading an AD costs the AD **2 tiers below** instead of antimatter; AD prices modified | `_c6BaseCost` / `_c6BaseCostMultiplier`; spend `AD(t-2).amount` | 6th AD autobuyer |
| 7 | buy-10 mult reduced to ×1, +0.2 per Dim Boost up to ×2, unaffected by upgrades | `buyTenMultiplier = min(2, 1 + totalBoosts/5)` | 7th AD autobuyer |
| 8 | Dim Boosts give no mult; Galaxies disabled; Sacrifice resets AM + all ADs but far stronger | `DimBoost.power = 1`; galaxy `canBeBought=false`; sacrifice `base=1` variant | 8th AD autobuyer |
| 9 | buying tickspeed or 10 ADs bumps everything of equal cost to its next step | `multiplySameCosts()` on the buy | Tickspeed autobuyer |
| 10 | only 6 ADs; Dim Boost/Galaxy costs modified | dims cap 6; galaxy `baseCost 99, costMult 90`; boost tier-6 scaling | Dim Boost autobuyer |
| 11 | "matter" rises with ≥1 2nd AD; if matter > antimatter, Dim Boost without bonus (annihilation) | `matter *= (1.03 + boosts/200 + galaxies/100)^(diff/20)`; soft reset when matter > AM | Galaxy autobuyer |
| 12 | each AD produces **2 tiers lower**; 1st/2nd make AM; 2/4/6 stronger | production retarget; sacrifice uses tier 6 | Big Crunch autobuyer |

`ActiveModifiers` fields the engine needs (closed vocabulary, grows with the
challenges): `production_halt` (NC2 factor), `ad1_pow` (NC3), `erase_lower_on_buy`
(NC4), `tickspeed_base_override` (NC5), `ad_cost_in_lower_dim` (NC6),
`buy10_override` (NC7), `dimboost_no_mult` + `galaxies_disabled` +
`sacrifice_variant` (NC8), `same_cost_bump` (NC9), `max_dimensions`/`cost_overrides`
(NC10), `matter_mechanic` (NC11), `production_shift`/`am_from_low_tiers` (NC12).

`chall2Pow`, `chall3Pow`, and `matter` are new `GameState` fields (mirroring
`player.chall2Pow` / `chall3Pow` / `Currency.matter`), advanced in `tick` while the
relevant challenge runs, and reset by the challenge reset path.

---

## 3. Rewards ↔ autobuyers (ties into Feature 2.6)

Completing challenge N unlocks an autobuyer: NC1–8 → the 1st–8th AD autobuyers,
NC9 → Tickspeed, NC10 → Dim Boost, NC11 → Galaxy, NC12 → Big Crunch. **Nuance:** in
the original a challenge completion makes the autobuyer *upgradeable* (the
`canBeUpgraded` flag — e.g. the Big Crunch autobuyer's interval can only be reduced,
and thus reach 0.1 s to unlock Break Infinity, after NC12). Our current AD/tickspeed
autobuyers are the antimatter-bought "slow" versions with fixed intervals; the
Dim-Boost/Galaxy/Big-Crunch autobuyers don't exist yet.

So the reward wiring is staged with Feature 2.6:
- **Now (2.5):** completing a challenge sets its completed bit and marks the
  corresponding autobuyer *unlocked* (for the AD/tickspeed ones that already exist,
  this is a second unlock path alongside the antimatter purchase). The
  Dim-Boost/Galaxy/Big-Crunch autobuyers are created as unlockable-but-fixed for now.
- **Later (2.6):** the interval-upgrade system + `canBeUpgraded` (challenge-gated),
  which is what Break Infinity (2.3) ultimately needs.

---

## 4. Save / load

`player.challenge.normal.{current, completedBits}` → `challenge.current` /
`challenge.completed`. `player.chall2Pow` / `chall3Pow` (numbers) and
`Currency.matter` (Decimal string) round-trip. All present in the template.

---

## 5. UI

The **Challenges** tab (new top-level tab, conditional on Infinity unlocked) → the
Normal Challenges subtab: a grid of 12 tiles (vendored `challenges` CSS). Each tile
shows the challenge name, description, best time (deferred), and a Start/Running
state; the currently-running challenge is highlighted, completed ones marked. A
`start_challenge(id)` / `exit_challenge` command pair + snapshot
`challenges[]` (`{ id, name, description, is_running, is_completed, is_unlocked }`).
The confirmation modal (`startNormalChallenge`) mirrors the prestige-confirm shell.

---

## 6. Incremental plan

1. **Infra + NC1 vertical slice**: `challenge` state, forced-reset path,
   enter/complete/exit, the `ActiveModifiers` scaffold (empty), NC1 (no modifier)
   end-to-end, save/load, the Challenges tab + tile grid, reward = mark autobuyer
   unlocked. Commit.
2. **Per-challenge modifiers**, a few at a time, each behind the fidelity harness:
   the "simple multiplier/threshold" ones first (NC5 tickspeed base, NC7 buy-10,
   NC8 no-boost/no-galaxy, NC10 six-dims), then the "tick-state" ones (NC2 halt,
   NC3 pow, NC11 matter), then the "production-rewrite" ones (NC4 erase, NC6
   cost-in-lower-dim, NC9 same-cost-bump, NC12 production-shift). Commit per batch.
3. **Records** (best challenge times) and the reward autobuyers' *upgradeable*
   behaviour land with Feature 2.6.

---

## 7. Open questions (proceeding with best-guess defaults)

- Exact Challenges-tab unlock gate and C11/C12 `lockedAt` thresholds — read from the
  data during implementation (best guess: tab = infinity-unlocked; C10=16, C11/C12
  higher).
- Whether to model `bestTimes` now — deferred until the Statistics/records consumer
  exists (consistent with the 2.1 best-IP/min deferral).
- The reward autobuyers that don't exist yet (Dim Boost / Galaxy / Big Crunch) —
  created as unlockable stubs in 2.5, fully wired in 2.6.

*Document generated: 2026-07-03.*
