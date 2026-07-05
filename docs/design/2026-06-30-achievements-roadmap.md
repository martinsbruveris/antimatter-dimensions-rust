---
status: Partial
---

# Achievements: feature correlation & secret-achievement analysis

Date: 2026-06-30

**Status: analysis only.** This doc does not propose code. It answers two
questions to sequence future achievement work:

1. **Normal achievements** — for each, the unlock condition and which feature
   (per `2026-06-23-feature-decomposition.md`) must exist before it can be
   implemented, so each achievement can be scheduled to land with its feature.
2. **Secret achievements** — a summary of their conditions and the implementation
   challenges they pose.

It builds on `2026-06-30-achievements.md`, which implemented the substrate
(`achievement_bits`, unlock/effect plumbing, the tab) and rows 1–2 (minus 22).
Conditions/effects below were re-read from the original source at
`antimatter-dimensions/src/core/secret-formula/achievements/`
(`normal-achievements.js`, `secret-achievements.js`).

---

## Part 1 — Normal achievements vs. features

### Method

Each achievement's `checkRequirement` references some game state. The achievement
becomes implementable once the feature that *owns that state* exists. The
substrate (bitmask, unlock seam, the global `1.25^rows × 1.03^count` power, and
the per-achievement effect hook) is already built, so adding any later
achievement is "evaluate one condition at the right seam + optionally read one
effect number" — the cost is in the **feature it reads**, not in the achievement
machinery.

I group achievements into **waves** keyed to the decomposition's features. A wave
is "earliest point this is implementable," not a hard ordering.

| Wave | Gating feature(s) | Decomp ref | Status of feature |
|------|-------------------|-----------|-------------------|
| **A** | Pre-Infinity state + tick + sacrifice + Big Crunch reset | 1.1–1.7, 2.1 (reset only) | ✅ exists |
| **B** | Infinity *records*: infinities count, this-infinity timing, best IP/min, recent-infinity log, achievement timers | 2.1 (fleshed out) | ⚠️ only `total_antimatter` today |
| **C** | Infinity Upgrades / Break Infinity / real IP formula | 2.2, 2.3 | ❌ |
| **D** | Normal Challenges + Autobuyers | 2.5, 2.6 | ❌ |
| **E** | Infinity Challenges | 2.7 | ❌ |
| **F** | Infinity Dimensions + Replicanti | 3.1, 3.2 | ❌ |
| **G** | Eternity (prestige, milestones, time dims, time studies, ECs) | 4.x | ❌ |
| **H** | Time Dilation | 5.x | ❌ |
| **I** | Reality (prestige, glyphs, perks, RU, black holes, automator) | 6.x | ❌ |
| **J** | Celestials (V, Ra, Lai'tela/singularity/dark matter, Pelle) | 7.x | ❌ |
| **X** | Cross-cutting infra not in the decomposition: **news ticker**, **time-played tracking**, **offline duration** | — | offline ✅; news/time-played ❌ |

### Cross-cutting infrastructure several achievements share

These are worth building once rather than per-achievement; many conditions reduce
to "read one of these":

- **Infinities counter** (`Currency.infinities`): 33, 87, 107, 116, 164, secret 26-adjacent.
- **Run timing & records** (`Time.thisInfinityRealTime`, `bestInfinity`,
  `bestRunIPPM`, `Time.challengeSum`, `recentInfinities[]`): 37, 54, 55, 56, 57,
  58, 62, 65, 68, 74, 78, 91, 92, 111, secret 32.
- **Per-run requirement flags** (`player.requirementChecks.{infinity,eternity,
  reality,permanent}`): 38, 64, 101, 122, 125, 132, 133, 151, 153, 156, 172,
  several secrets. These are booleans set when a "disqualifying" purchase happens
  during a run and cleared on the relevant reset — a small but distinct subsystem.
- **Achievement timers** (`AchievementTimers.*.check(cond, seconds)`): "hold a
  condition for N consecutive seconds": 44, 124, secret 16, secret 44.

### Row-by-row mapping

Row 1–2 status reflects `2026-06-30-achievements.md`. "Effect" notes only
achievements that carry a production effect (the rest contribute solely to the
global power).

**Row 1 (11–18) — Wave A — ✅ implemented.** Buy a 1st…8th Antimatter Dimension.
No per-achievement effects.

**Row 2 (21–28) — Wave A — ✅ implemented except 22.**

| id | Condition | Wave | Effect |
|----|-----------|------|--------|
| 21 | Big Crunch | A ✅ | start with 100 AM |
| 22 | See 50 distinct news tickers | **X** (news ticker) | — |
| 23 | exactly 99 8th ADs | A ✅ | 8th AD ×1.1 |
| 24 | AM ≥ 1e80 | A ✅ | — |
| 25 | 10 Dimension Boosts | A ✅ | — |
| 26 | Buy an Antimatter Galaxy | A ✅ | — |
| 27 | ≥ 2 Antimatter Galaxies | A ✅ | — |
| 28 | Buy a 1st AD with ≥ 1e150 of them | A ✅ | 1st AD ×1.1 |

**Row 3 (31–38).**

| id | Condition | Wave | Note |
|----|-----------|------|------|
| 31 | Any AD multiplier ≥ 1e31 | **A** | tick check on `dimension_multiplier`; reward 1st AD ×1.05 |
| 32 | Sacrifice total boost ≥ 600 (outside Challenge 8) | **A** | sacrifice exists; the "outside C8" clause is trivially true pre-challenges. Reward changes the **sacrifice exponent** (stacks with 57, 88) |
| 33 | Reach Infinity ≥ 10 times | **B** | needs infinities counter |
| 34 | Infinity having never bought an 8th AD this run | **B** | crunch-before + per-run "bought AD8" flag; reward dims 1–7 ×1.02 |
| 35 | Offline ≥ 6 hours | **X** (offline) | offline mode exists; needs `lastUpdate` delta surfaced |
| 36 | Infinity with exactly 1 galaxy | **A** | crunch-before, `galaxies == 1`; reward starting tickspeed ×1/1.02 |
| 37 | Infinity in < 2 hours | **B** | this-infinity real time; reward start 5000 AM |
| 38 | Buy a galaxy without sacrificing this infinity | **B** | per-run `noSacrifice` flag |

**Row 4 (41–48).**

| id | Condition | Wave |
|----|-----------|------|
| 41 | 16 Infinity Upgrades bought | **C** |
| 42 | AM/sec > current AM, above 1e63 | **A** (production per second) |
| 43 | AD multipliers strictly ascending (8th highest … 1st lowest) | **A** (tick) |
| 44 | AM/sec > AM for 30 consecutive seconds | **B** (achievement timer) |
| 45 | Tickspeed < 1e-26 s (≥1e29 ticks/s) | **A** (tickspeed) |
| 46 | 7th AD amount ≥ 1e12 (i.e. all of 1–7) | **A** |
| 47 | Complete 3 Normal Challenges | **D** |
| 48 | Complete all 12 Normal Challenges | **D** |

**Row 5 (51–58).**

| id | Condition | Wave |
|----|-----------|------|
| 51 | Break Infinity | **C** |
| 52 | Max AD + tickspeed autobuyer intervals | **D** |
| 53 | Max all normal autobuyer intervals | **D** |
| 54 | Infinity ≤ 10 min | **B** |
| 55 | Infinity ≤ 1 min | **B** |
| 56 | Complete NC2 ≤ 3 min | **D** |
| 57 | Complete NC8 ≤ 3 min | **D** (reward: sacrifice exponent, stacks w/ 32, 88) |
| 58 | Complete NC9 ≤ 3 min | **D** (reward: buy-10 mult +0.01) |

**Row 6 (61–68).**

| id | Condition | Wave |
|----|-----------|------|
| 61 | All AD autobuyer bulk amounts maxed | **D** |
| 62 | Best IP/min ≥ 1e8 | **C** (real IP formula + records) |
| 63 | Begin generating Infinity Power | **F** (Infinity Dimensions) |
| 64 | Infinity in a Normal Challenge w/ 0 boosts & 0 galaxies | **D** |
| 65 | Sum of Normal Challenge times < 3 min | **D** |
| 66 | Tickspeed < 1e-55 s | **A** (mechanic; realistically needs many galaxies) |
| 67 | Complete an Infinity Challenge | **E** |
| 68 | Complete NC3 ≤ 10 s | **D** |

**Row 7 (71–78).**

| id | Condition | Wave |
|----|-----------|------|
| 71 | NC2 with a single AD1, 0 boosts, 0 galaxies | **D** |
| 72 | All AD multipliers ≥ 1.8e308 | **C** (needs Break Infinity to reach) |
| 73 | AM ≥ 9.9999e9999 | **C** (Break Infinity) |
| 74 | Sum of best Normal Challenge times < 5 s | **D** |
| 75 | Unlock 4th Infinity Dimension | **F** |
| 76 | Play for 8 days | **X** (time-played) |
| 77 | Infinity Power ≥ 1e6 | **F** |
| 78 | Infinity in < 250 ms | **B** |

**Row 8 (81–88).**

| id | Condition | Wave |
|----|-----------|------|
| 81 | Beat IC5 ≤ 15 s | **E** |
| 82 | Complete all 8 Infinity Challenges | **E** |
| 83 | 50 Antimatter Galaxies | **C** (needs distant-galaxy scaling + Break to reach) |
| 84 | AM ≥ 1e35000 | **C** |
| 85 | Big Crunch for ≥ 1e150 IP | **C** (post-break IP) |
| 86 | Tickspeed ≥ 1000× faster per upgrade | **A** (mechanic) |
| 87 | 2,000,000 Infinities | **B** (counter; realistically mass-infinity, Wave G) |
| 88 | Single sacrifice ≥ 1.8e308 | **C** (needs Break) — reward: sacrifice exponent (32/57/88) |

**Row 9 (91–98).**

| id | Condition | Wave |
|----|-----------|------|
| 91 | BC for 1e200 IP ≤ 2 s | **C** |
| 92 | BC for 1e250 IP ≤ 20 s | **C** |
| 93 | BC for 1e300 IP | **C** |
| 94 | Infinity Power ≥ 1e260 | **F** |
| 95 | 1.8e308 Replicanti in ≤ 1 h | **F** (Replicanti) |
| 96 | Go Eternal | **G** |
| 97 | Sum of IC times < 6.66 s | **E** |
| 98 | Unlock 8th Infinity Dimension | **F** |

**Row 10 (101–108) — mostly Eternity.**

| id | Condition | Wave |
|----|-----------|------|
| 101 | Eternity without buying ADs 1–7 | **G** |
| 102 | All Eternity milestones | **G** |
| 103 | IP ≥ 1e1000 | **C** |
| 104 | Eternity < 30 s | **G** |
| 105 | 308 tickspeed upgrades from Time Dimensions | **G** |
| 106 | 10 Replicanti Galaxies in ≤ 15 s | **F** |
| 107 | Eternity with < 10 Infinities | **G** |
| 108 | Eternity with exactly 9 Replicanti | **G** |

**Row 11 (111–118).** 111 record-scaling (B/G); 112 IC times (E); 113 Eternity
<250ms (G); 114 fail an EC (G); 115 IC inside EC (G); 116 Eternity w/ 1 infinity
(G); 117 bulk-buy 750 dim boosts (**C/D** — needs max-dimboost autobuy/bulk); 118
total sacrifice 1e9000 (**C**).

**Row 12 (121–128).** 121 IP 1e30008 (C); 122 Eternity w/o ADs 2–8 (G); 123 50 EC
tiers (G); 124 Infinity-Power marathon 60 s (F + timer); 125 1e90 IP, no
infinities/no AD1 (G); 126 180× more RG than AG (F/G); 127 1.8e308 EP (G); 128 IP
1e22000 w/o Time Studies (G). → **Wave G** dominant, a few C/F.

**Row 13 (131–138).** 131 2e9 banked infinities (G); 132 569 galaxies, no RG (G);
133 1e200000 IP, no IDs/no IP-mult (G); 134 1e18000 Replicanti (F); 135 tickspeed
< 1e-8296262 s (G/H, mechanic only); 136 Dilate time (**H**); 137 1e260000 AM in
≤1 min dilated (H); 138 1e26000 IP, no TS, dilated (H). → **Wave H** dominant.

**Row 14 (141–148) — Reality, Wave I.** 141 make a Reality; 142 unlock automator;
143 eternity record-scaling; 144 unlock Black Hole; 145 BH interval < duration;
146 all Perks; 147 all Reality Upgrades; 148 Reality with each basic Glyph type.

**Row 15 (151–158) — Wave I/J.** 151 800 galaxies w/o AD8 this infinity (reward
unlocks **V** → J); 152 100 Glyphs in inventory (I); 153 Reality w/o producing AM
(I); 154 Reality < 5 s (I); 155 play 13.7 **billion** years (**X**, effectively a
time-warp/never-natural condition); 156 Reality w/o buying Time Theorems (I); 157
Glyph with 4 effects (I); 158 both Black Holes permanent (I).

**Row 16 (161–168) — Wave H/I/J.** 161 1e1e8 AM dilated (H); 162 every Time Study
at once (G/I); 163 all ECs ×5 in <1 s Reality (I); 164 1.8e308 Infinities (B/G);
165 level-5000 balanced Glyph (I); 166 Glyph level exactly 6969 (I); 167 1.8e308
RM (I); 168 50 total Ra memory levels (**J**, Ra).

**Row 17 (171–178) — Wave J (Ra / Lai'tela).** 171 sacrifice every Glyph type
(I/J); 172 Reality 1.8e308 RM, no charged upgrades/glyphs/triads (J); 173
9.99999e999 RM (I); 174 get a Singularity (J); 175 cap all Alchemy resources (J);
176 annihilate Dark Matter Dimensions (J); 177 all Singularity milestones (J);
178 100000 galaxies (J, remote scaling).

**Row 18 (181–188) — Wave J (Pelle), and a substrate caveat.** 181 Doom your
Reality; 182 regain all AD autobuyers (Pelle upgrades); 183 IC5 while Doomed; 184
third Pelle Strike; 185 fourth Pelle Strike; 186 buy Time Study 181 while Doomed;
187 unlock Dilation while Doomed; 188 Beat the game.

> ✅ **`achievement_bits` is now `[u32; 18]` (resolved 2026-06-30).** The original
> `player.achievementBits` defaults to `Array.repeat(0, 17)`, yet row 18 writes
> `achievementBits[17]` (e.g. `dev-migrations.js:1256`), so JS silently grows the
> array to length 18. We widened `ACHIEVEMENT_ROW_COUNT` to 18 so ids 181–188 map
> in-bounds, and `save/dto.rs` now accepts an `achievementBits` of length **17 or
> 18**, zero-filling the missing Pelle row. This lets us load *any* original save —
> including a Doomed one — even though we model no Pelle mechanic and never unlock
> those bits ourselves. Row 18 counts toward the global power exactly as the
> original's `Achievements.all` does; the Doomed multiplier-disable
> (`Pelle.isDisabled`) is a separate mechanic we don't model. When Pelle (Wave J)
> actually lands, only the *unlock conditions* and that disable remain to wire up —
> the storage/save substrate is already in place.

### Summary of when to implement

- **Now (Wave A), low-hanging fruit beyond rows 1–2:** 31, 32, 36, 42, 43, 45,
  46, 66, 86. All readable from current pre-Infinity state at an existing seam
  (tick / sacrifice / crunch-before). 32, 57, 88 share the sacrifice-exponent
  reward — implement 32's effect path once and the later two slot in.
- **With a fleshed-out Infinity layer (Wave B):** the run-timing/records/counter
  cluster — 33, 34, 37, 38, 44, 54, 55, 62, 78, 87, 111. Build the
  infinities-counter + this-infinity-timer + records infra once; ~a dozen
  achievements fall out.
- **Per feature thereafter:** add each achievement *with* the feature it reads
  (challenges → D/E, IDs/Replicanti → F, Eternity → G, etc.). The substrate cost
  is ~zero; schedule them as a checklist item in each feature's own doc.
- **Cross-cutting (Wave X):** 22 (news ticker) and 76/155 (time-played) need
  small standalone subsystems not in the decomposition; 35 rides the existing
  offline feature.

---

## Part 2 — Secret achievements

Source: `secret-achievements.js` (24 achievements, ids 11–18, 21–28, 31–38,
41–48). They use a **separate** `secretAchievementBits` array and — crucially —
**none carry a production effect or reward**. They are pure completion/cosmetic.
That makes the *engine* side trivial (a second bitmask + an idempotent
`unlock_secret(id)` action + save round-trip, with **no** multiplier plumbing).
The difficulty is entirely in **what triggers them**: most are UI/interaction
events that have no representation in a headless engine condition.

### Conditions, grouped by trigger type

**(a) Pure frontend interaction — no engine state at all.** These can only be
unlocked by the frontend calling an "unlock secret N" action in response to a UI
event. The engine cannot evaluate them.

| id | Name | Trigger |
|----|------|---------|
| 11 | The first one's always free | Click the achievement itself |
| 13 | It pays to have respect | "Pay respects" (press F) |
| 14 | So do I | Type something naughty into an input |
| 15 | Do a barrel roll! | Trigger the barrel-roll animation |
| 17 | 30 Lives | Enter the Konami code |
| 23 | Stop right there criminal scum! | Open the dev console (browser detection) |
| 25 | Shhh… It's a secret | Discover a secret theme |
| 28 | Nice. | "Don't act like you don't know what you did" (type 69) |
| 31 | …download some more RAM | Set update rate to 200 ms (options) |
| 33 | A sound financial decision | Click the "buy STD coins" button |
| 37 | You followed the instructions | Follow a scripted UI sequence |
| 38 | Knife's edge | Close the Hard Reset modal after typing the confirmation |
| 41 | That dimension doesn't exist | Click the (joke) 9th-dimension buy button |
| 47 | ALT+ | Hide every possible tab |

**(b) Frontend interaction with a timer / counter, still frontend-owned.**

| id | Name | Trigger |
|----|------|---------|
| 12 | Just in case | Save 100 times **without refreshing** (session counter) |
| 24 | Real news | Click a news-ticker message that *does* something (news ticker + clickable tickers) |
| 36 | While you were away… | See *nothing* happen on an offline return (offline feature) |
| 44 | Are you statisfied now? | Statistics tab open for 15 real-time min (tab-open timer) |
| 45 | This dragging is dragging on | Drag the Perk tree for ~60 s (UI drag + persisted counter) |

**(c) Engine/state-backed, but gated on a not-yet-built feature.**

| id | Name | Condition | Gated on |
|----|------|-----------|----------|
| 16 | Do you enjoy pain? | A "painful" notation active 10 min after an Eternity | notation system + Eternity (G) + ach. timer |
| 18 | Do you feel lucky? | 1/100000 chance **per second** | RNG in tick |
| 21 | Go study in real life instead | Purchase the secret Time Study | Time Studies (G) |
| 22 | Deep fried | Buy 1e5 galaxies total while in **emoji** notation | notation + a persisted `emojiGalaxies` counter |
| 26 | You're a failure | Fail ECs 10× **without refreshing** | Eternity Challenges (G) + session counter |
| 27 | …matter dimensions…? | Get "Infinite" matter (≥1.8e308) | `matter` mechanic (Challenge 3) |
| 32 | ≤ 0.001 | Best infinity *or* eternity time ≤ 1 ms | run-time records (B / G) |
| 34 | You do know how these work | Respec an empty Time Study tree | Time Studies (G) |
| 35 | Should we tell them about buy max | Buy single Tickspeed 1e5 times | persisted `singleTickspeed` counter |
| 42 | SHAME ON ME | Try to use EC12 to speed up time | Eternity Challenges (G) |
| 43 | A cacophonous chorus | All equipped Glyphs are Music Glyphs | Glyphs (I) + music-glyph variant |
| 46 | For a rainy day | Store a day of real time | Enslaved time storage (J) |
| 48 | Stack overflow | More Automator errors than lines | Automator (I) |

### Implementation challenges

1. **Wrong shape for the condition-at-a-seam model.** The normal-achievement
   design works because each condition reads engine state at an action seam. The
   majority of secret achievements (group **a**, plus b) are triggered by *UI
   events the engine never sees* — clicks, key sequences, modal interactions,
   theme/console/devtools state. They require an explicit **frontend → engine**
   `unlock_secret(id)` action (or a frontend-only secret bitmask synced into the
   save). This is a new direction of data flow vs. the engine-evaluates-everything
   pattern used so far.

2. **No effects = simpler engine, but new persisted state.** Because secret
   achievements have no rewards, there's **no** effect/multiplier plumbing —
   unlike normal achievements, nothing reads them back into production. The only
   engine work is a `secret_achievement_bits` field + save round-trip (mirroring
   the existing `achievement_bits` work) and an idempotent unlock entry point. The
   tab/notification UI mirrors the normal one.

3. **Session-scoped vs. permanent counters — two distinct lifetimes.**
   - *Permanent* lifetime counters that survive **all** resets:
     `singleTickspeed` (35), `emojiGalaxies` (22), `perkTreeDragging` (45). These
     live in `requirementChecks.permanent` and need new persisted fields.
   - *Session* counters that must **reset on refresh** and never persist: save
     count (12), EC-fail count (26 — the original literally uses a JS closure so
     the count is per page-load). Persisting these would wrongly unlock on reload.
     The engine must keep them out of the save (or the frontend owns them).

4. **Notation coupling (16, 22).** These read the *active number notation*
   ("painful" set; "emoji"). That couples a secret achievement to the
   number-formatting/notation layer (`ad-format`) and to whichever side owns the
   notation selection — likely frontend, reinforcing challenge 1.

5. **Randomness vs. determinism (18).** A 1/100000-per-second roll introduces
   non-determinism into `tick`. The fidelity/replay tests assume deterministic
   ticks; a random unlock would make runs non-reproducible. Needs either a seeded
   RNG threaded through the sim or an explicit "exclude from fidelity" carve-out.

6. **Browser/meta conditions may be un-portable (23, 25, 33).** Console detection,
   secret themes, and the "STD coins" microtransaction joke are tied to the
   original's web shell. They may be intentionally **stubbed or skipped** rather
   than ported, depending on whether the GUI reproduces those surfaces.

7. **Most engine-backed ones are far-future anyway (group c).** 16/21/26/34/42
   (Eternity/Time Studies/ECs), 43 (Glyphs), 46 (Enslaved), 48 (Automator) can't
   land before their features exist — so secret achievements are naturally a
   *late* effort. The early-reachable engine-backed ones are just **18** (RNG),
   **27** (matter, with Challenge 3), **32** (run-time records), **35** (tickspeed
   counter), and **22** (galaxy counter + notation).

### Recommendation

Treat secret achievements as a **single late milestone, frontend-led**: build the
`secret_achievement_bits` substrate (state + save + tab + toast, ~a copy of the
normal-achievement substrate **minus all effect plumbing**) once, expose an
`unlock_secret(id)` action, then wire triggers opportunistically — frontend UI
events for group (a)/(b), and engine checks for group (c) as each gating feature
lands. Decide explicitly whether the browser-meta trio (23, 25, 33) and the
13.7-billion-year-style novelty conditions are in or out of scope before starting.
The RNG one (18) needs a determinism decision up front.
