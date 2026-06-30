# Offline Progress & Manual Offline Mode

Date: 2026-06-30

## Goal

Two related things:

1. Understand how the original game simulates **offline progress** (time that
   passed while the game was closed) and assess how cleanly it maps onto our
   engine.
2. Add an **Offline mode button** under the speed-control buttons that *pauses*
   the live game and runs the elapsed wall-clock time as one offline batch when
   resumed. This is essentially an in-app way to exercise and observe the
   offline mechanism on demand.

This doc records the analysis and a proposed implementation. It also answers
the three time-handling questions raised: how game speed should scale offline
time, whether the save-file timestamp is a problem, and whether we use absolute
timestamps anywhere else.

## How the original game does offline

The relevant code is `src/game.js` (`gameLoop`, `simulateTime`) and
`src/core/storage/storage.js` (the load path).

### Entry point (load)

`GameStorage.loadPlayerObject` (storage.js ~line 493):

```
const rawDiff = Date.now() - player.lastUpdate;
const simulateOffline = this.offlineEnabled ?? player.options.offlineProgress;
if (simulateOffline && diff > 10000) {
  simulateTime(diff / 1000, false, diff < 50 * 1000);
}
```

So offline progress is just *the wall-clock gap since the save's `lastUpdate`
timestamp*, replayed through the normal tick loop. Under 10 s is ignored; the
`fast` flag (under 50 s) caps the work at 50 ticks.

### `simulateTime(seconds, real, fast)` (game.js ~line 895)

The mechanism, stripped of black-hole / celestial concerns we don't model:

- Base resolution is a **50 ms tick** → `ticks = floor(seconds * 20)`.
- Tick count is capped by `maxOfflineTicks`:
  `min(player.options.offlineTicks /*default 1000*/, floor(simulatedMs / 33), 1e6)`.
  Two invariants: a tick is never shorter than 33 ms (so you can't exploit tick
  microstructure), and there are at most 1e6 ticks.
- The total *game* time to simulate is `getGameSpeedupFactor() * seconds` (game
  speed multiplies real time).
- It then runs that many `gameLoop(1000 * diff)` calls, where each `diff` is the
  remaining real time divided by the remaining tick count, so the elapsed time
  is spread evenly across the capped tick budget.

The crucial point: **offline accuracy comes from running many real-sized-ish
ticks, not one big step.** Per-tick discrete effects (above all autobuyers,
which fire at most once per `gameLoop`) need many tick boundaries to behave
correctly. A single huge tick would let autobuyers fire only once.

## What our codebase already has

We are in remarkably good shape — the engine already exposes exactly the right
primitives and already keeps the wall clock at arm's length.

- `GameState::tick(dt_ms)` — one tick (runs autobuyers, then production).
- `GameState::ticks(dt_ms, repeats)` — N discrete real-sized ticks. Its doc
  comment already articulates the "many small ticks beats one big step" rule;
  this is the same rule offline depends on.
- `GameState::simulate(total_ms, tick_size_ms)` — runs `total_ms / tick_size_ms`
  steps of `tick_size_ms`. **This is the offline primitive**; offline is just
  `simulate` with a tick size derived from a capped tick budget.

The live loop (`App.vue`) already ticks via
`tick_and_get_state(dt_ms, repeats)` where `repeats = speedMultiplier`, i.e. one
real frame of `dt_ms` becomes `speedMultiplier` ticks. Game speed already means
"do more ticks," exactly as the user framed it.

### The save timestamp — not a problem

`encode_save(state, now_ms)` writes `player.lastUpdate = now_ms`
(`save/encode.rs`). `now_ms()` lives in the GUI (`main.rs`) via `SystemTime`, so
`ad-core` stays clock-free (AGENTS.md principle #3).

Two facts make the timestamp harmless for us:

1. **We never read it back.** The read path (`save/dto.rs`) does not model
   `lastUpdate` at all, and `GameState` has no timestamp field. A save we import
   produces **zero** offline progress in our app today — we just resume.
2. **We stamp it with "now" specifically so the *original* game also computes
   ~0 offline progress** when our save is imported there. It exists purely for
   cross-compatibility, not for our own logic.

So the timestamp is write-only, compatibility-only. It does not constrain how we
implement offline.

### Are we using absolute timestamps anywhere else?

No. The only `SystemTime` call in the whole workspace is `now_ms()` in
`crates/ad-gui/src/main.rs`, used solely for the save stamp above. The engine is
fully deterministic and clock-free. All in-engine time is **relative** (`dt_ms`,
autobuyer `timer_ms`/`interval_ms`). This means we are free to compute an
offline duration however we like in the GUI layer and feed it to the engine as a
plain `dt`.

### Does the *original* game use timestamps for statistics (e.g. play time)?

This is the natural worry: "total time played" sounds like it would be
`now - startTimestamp`. It isn't. The original tracks play time as **duration
accumulators incremented from per-tick deltas**, not as differences of absolute
timestamps. In `gameLoop` (game.js ~line 404/525-527):

```
player.records.realTimePlayed  += realDiff;   // real ms this tick
player.records.realTimeDoomed   += realDiff;
player.records.totalTimePlayed += diff;        // game ms (realDiff * speedFactor)
player.records.thisInfinity.time += diff;      // per-prestige timers, same pattern
```

Every play-time statistic (`totalTimePlayed`, `realTimePlayed`, the per-prestige
`thisInfinity/thisEternity/thisReality.time`, etc.) is `0` at start and grows by
the tick delta. They are stored in the save as **elapsed milliseconds**, not
timestamps. This is exactly our relative-`dt` model — they would port as plain
`f64`/`Duration` fields bumped in the tick loop, with **no clock dependency**.

Note the two-speed split that *directly concerns offline mode*:
`realTimePlayed += realDiff` (wall time) vs `totalTimePlayed += diff` (game
time = `realDiff * speedFactor`). When we eventually add play-time stats, offline
mode must feed real elapsed ms into the "real" accumulators and speed-scaled ms
into the "game" accumulators — the same split this doc applies to production.

The genuinely absolute wall-clock timestamps in the original are few and **none
drive gameplay or the core statistics**:

- `player.lastUpdate` — the offline-diff save stamp (discussed above).
- `player.records.gameCreatedTime = Date.now()` — used only for a cosmetic "this
  save has existed for X" news-ticker line (`Date.now() - gameCreatedTime`).
- `GameStorage.lastSaveTime` / `lastCloudSave` — autosave/cloud cadence
  (transient, not in the player save) — the convenience-cadence category.
- RNG / speedrun seeds (`initialSeed`, `musicSeed`, `speedrun.startDate`) seeded
  from `Date.now()` — entropy, not time tracking.

Bottom line: there is no statistics feature whose value depends on reading the
wall clock at display time. Porting play-time stats later needs only relative
accumulators, and they interact with offline mode exactly as production does.

## Game speed and offline time

The user's mental model is correct and already matches the live loop:

- Game speed = pretend wall time passes faster = do more ticks. The game logic
  is unchanged.
- Therefore **offline time must be scaled by the speed multiplier**: one second
  of real away-time at `Nx` advances `N` seconds of game time.
- The speed multiplier is **integrated over changes during the pause**, not
  sampled once at resume (the user's choice). If the player sits in offline mode
  and slides the speed up and down, the accumulated game time reflects each
  segment at the speed in force at the time:

  ```
  game_ms = Σ over frames  (real_frame_ms × speed_at_that_frame)
  ```

  This is what makes the live offline-time display meaningful — the counter
  visibly climbs faster as speed is raised — and it lets the user "dial in" how
  much offline time to bank by watching it.
- The lone exception the user named — **save frequency** — is indeed a pure
  convenience cadence and should *not* be touched by the speed multiplier or by
  offline mode. It is not game state.

Mechanically, the GUI accumulates a single running total while offline mode is
engaged, adding to it each animation frame and **deferring all production** (the
engine is not ticked, so the snapshot freezes):

- `accumulated_game_ms += real_frame_ms × speedMultiplier` — the integrated game
  time. This is what the readout displays *and* what gets produced; the tick
  budget is derived from it (see "Tick budget" below), so no separate real-time
  total is needed. When offline mode is switched off, this whole interval is
  replayed as one capped-tick batch, then reset to 0.

Note this differs subtly from the original's *load-time* offline, which takes a
single real diff and multiplies by the then-current speed. Integrating frame by
frame is strictly more general and degenerates to the same thing when speed is
constant.

## Tick budget — a behaviour knob, not just a perf knob

This is the subtle part the analysis surfaced, and it changes the design.

### The original's actual policy

The default is `player.options.offlineTicks = 1e5` (**100,000**, not 1000 — an
earlier draft of this doc was wrong). The in-game slider
(`OptionsGameplayTab.vue`) ranges **500 → 1,000,000**. On top of the chosen
value, `GameStorage.maxOfflineTicks` clamps:

```
ticks_wanted = floor(real_seconds * 20)                  // 50 ms base resolution
max_ticks    = min(offlineTicks, floor(real_ms / 33), 1e6)
ticks        = min(ticks_wanted, max_ticks)              // never increased
```

So: 50 ms base tick, never shorter than 33 ms of *real* time, capped at the
player's `offlineTicks` and a hard 1e6. The elapsed real time is then spread
across `ticks` `gameLoop` calls, each of which applies the game-speed factor
internally. **The budget is a function of real time and the player option — game
speed does not buy more offline ticks**, it only makes each tick span more game
time.

### Why the cap exists, and why it matters less for us

In JS the 1e5 cap is largely a **performance** ceiling: processing 100,000 ticks
takes ~10 s, which is why the original shows a progress modal with a "Speed
up / SKIP" escape hatch. Our Rust engine is far faster, so the *performance*
reason to cap is mostly gone — we could tick at true 50 ms game-time resolution
for very long intervals without a visible stall.

But the tick count is **also a behaviour knob**, and that part does not go away:
autobuyers (and any once-per-tick effect) fire at most once per tick, so more
ticks → more autobuyer fires → more production. Two saves with identical state
and identical away-time produce *different* results under different tick budgets.
The budget is therefore a genuine game-rules parameter, not a free performance
dial we can silently max out. If we just "use our speed to run way more ticks,"
we **diverge from the original's numbers**.

### Decision: one slider, extended range

There is **one** offline mechanism, with a single player-facing knob —
`offline_ticks` — and no separate "modes." The divergence the faster engine
buys is simply a **wider slider range**, not a different code path:

- **Default 100,000** reproduces the original's behaviour and numbers.
- The slider's **maximum is extended well beyond the original's 1e6** (target
  ~10M–100M, precise bounds TBC) because our engine processes that many ticks in
  a fraction of the ~10 s the JS game needs. A player who cranks it up gets finer
  offline resolution — closer to "as if I'd been online" — without the wait. A
  player who leaves it at 100,000 gets exactly the original behaviour, just
  computed instantly.

So `offline_ticks` is the sole control over **how accumulated offline time is
translated into game progress**. The tick budget is `min(game_ms / 50 ms,
offline_ticks)`:

- While the accumulated game-time fits inside `offline_ticks × 50 ms`, replay
  runs at full native 50 ms resolution (offline ≈ online).
- Past that, the count saturates at `offline_ticks` and each tick stretches —
  the same coarsening the original does at its cap, just at whatever ceiling the
  player chose.

This cleanly separates the two concerns the user identified:

- **Game speed** governs *how much* offline game-time accumulates while paused
  (`Σ real_frame_ms × speed`).
- **`offline_ticks`** governs *how finely* that accumulated time is replayed.

At speed 1× this reproduces the original's load-time offline exactly (the budget
formula coincides); at higher speed you accumulate more game-time, and
`offline_ticks` decides whether that replays finely (high slider) or coarsely
(left at 100,000). The ability to match the original is therefore always
preserved — it is just the low end of the same slider.

## Caveats in our codebase

1. **Autobuyers fire at most once per tick.** `Autobuyer::advance` (autobuyers.rs)
   adds `dt_ms` to its timer, fires *once* if it crossed the interval, then
   clamps any remainder to avoid unbounded carry. This matches the original
   (one fire per `gameLoop`), and is exactly why the tick budget is a behaviour
   knob (see "Tick budget" above): too few ticks over a long away-time →
   autobuyers fire too rarely → under-production. Honour the chosen budget
   (default 100,000 ticks, 50 ms base) rather than collapsing to one big
   `tick(game_ms)`, which would fire each autobuyer exactly once.

2. **No `lastUpdate` in `GameState`.** Because we don't persist a timestamp, we
   can't (and needn't) reconstruct away-time across an app restart from the save
   alone. For the *manual* Offline-mode button this is irrelevant — the GUI
   measures the pause duration directly. If we later want
   *offline-progress-on-load*, the GUI would need to record load wall-time and
   diff against the save's `lastUpdate` (which would then have to be modelled on
   the read path). Out of scope here; noted for the future.

3. **Antimatter is capped each tick** at `BIG_CRUNCH_THRESHOLD` (tick.rs). Fine
   for offline — a long away-time simply parks at the cap, as it should
   pre-Infinity.

4. **Precision degrades past the budget.** Once away-time exceeds what the budget
   covers at 50 ms (≈ 83 min at the 100,000 default), tick *count* stops growing
   and tick *size* grows instead — the full duration is still simulated, just at
   coarser resolution. Continuous production (dimensions feeding lower dimensions)
   is ~unaffected (it is linear in `dt`); per-tick effects (autobuyers) thin out.
   This is the same deliberate trade-off the original makes; raising the
   `offline_ticks` slider pushes the degradation threshold out.

5. **Determinism preserved.** All of the above stays inside the engine's
   IO-free, clock-free boundary. The engine receives only a pre-integrated
   `game_ms`; the wall clock (per-frame `performance.now()` deltas) is read
   entirely in the GUI — same category as `now_ms()`.

## Proposed design: two controls

Two independent toggles in the top-right controls, under the speed-control row:
an **Offline-mode** button (with a live accumulated-time readout) and an
**absolute Pause** button (dev). They compose; the table below is the full
behaviour matrix.

| State | Live ticks | Offline accumulation | Snapshot |
|-------|-----------|----------------------|----------|
| Normal | yes (`tick(dt, speed)`) | — | updates live |
| Offline mode | no | yes (`+= dt × speed`) | frozen until resume |
| Absolute pause | no | no | frozen |
| Offline + pause | no | no (frozen) | frozen |

### Offline-mode button (with live readout)

A toggle. When **on**:

- The live loop stops ticking the engine — production is **deferred**, the
  snapshot freezes.
- Each animation frame, the GUI adds `real_frame_ms × speedMultiplier` to a
  running `accumulated_game_ms` (the integration from the game-speed section).
- A readout **next to the button** shows `accumulated_game_ms` formatted as a
  duration, updating every frame. Because the increment scales with speed, the
  counter visibly speeds up / slows down as the speed buttons are pressed —
  giving the user a live sense of time passing and direct control over how much
  offline time to bank.

When switched **off**:

- The whole `accumulated_game_ms` interval is replayed as one capped-tick offline
  batch (`simulate_offline`), the snapshot updates, `accumulated_game_ms` resets
  to 0, and the live loop resumes.

This is an honest in-app reproduction of "I was away for a while," except the
away-time is accrued interactively and at a speed the user steers in real time.

### Absolute Pause button (dev)

A second toggle that **freezes everything**: no live ticks *and* no offline
accumulation. While engaged, the frame loop consumes elapsed wall time without
acting on it (so unpausing doesn't produce a catch-up jump). It overrides offline
mode — if both are on, `accumulated_game_ms` holds steady until pause is lifted.
Purpose is dev inspection: stop the world, examine state, resume.

### Engine side (`ad-core`)

Add a small, explicit offline entry point so the tick-budget policy lives in the
engine and is unit-testable, rather than being reconstructed in JS:

The budget is `min(game_ms / 50 ms, offline_ticks)` — one formula, with
`offline_ticks` (the player slider value) as the only knob:

```rust
/// Native offline tick resolution (the original's 50 ms base).
const OFFLINE_BASE_TICK_MS: f64 = 50.0;

impl GameState {
    /// Replay `game_ms` of (speed-integrated) game time as offline progress,
    /// spread across at most `offline_ticks` discrete ticks. Below
    /// `offline_ticks × 50 ms` of game time this runs at native 50 ms
    /// resolution; beyond it the count saturates and ticks stretch. Many ticks,
    /// not one big step, so per-tick effects (autobuyers) behave.
    ///
    /// `offline_ticks` is the player setting (default 100_000, reproducing the
    /// original; the slider's extended max trades the engine's speed for finer
    /// resolution). At speed 1× this matches the original's load-time offline.
    pub fn simulate_offline(&mut self, game_ms: f64, offline_ticks: u32) {
        if game_ms <= 0.0 {
            return;
        }
        let want = (game_ms / OFFLINE_BASE_TICK_MS).floor() as u32;
        let ticks = want.clamp(1, offline_ticks);
        let tick_size = game_ms / ticks as f64; // >= 50 ms
        self.ticks(tick_size, ticks);
    }
}
```

The engine takes only `game_ms` — the GUI does the speed integration, and the
budget needs no separate `real_ms` (the 50 ms base is in game time). (We reuse
`ticks`; it states the budget directly.)

### GUI side (`ad-gui`)

- `main.rs`: a `simulate_offline` Tauri command taking `game_ms` and the
  `offline_ticks` setting, calling `game.simulate_offline(game_ms, offline_ticks)`
  and returning a fresh `GameView`. Integration (speed scaling) happens in the JS
  frame loop because that is where per-frame speed is known; the engine owns the
  tick-budget formula. Contract: "GUI passes integrated game-time, engine replays
  it at the chosen resolution."
- `stores/ui.js`: add `offlineMode` and `absolutePause` booleans, an
  `accumulatedGameMs` number, and `toggleOfflineMode()` / `toggleAbsolutePause()`
  actions. `accumulatedGameMs` is reactive so the readout re-renders each frame.
  `offline_ticks` is a player option (read from the snapshot, like the other
  options) once the slider exists; until then a constant default of 100,000.
- `App.vue` `loop()`, in priority order each frame (always advance `last` so no
  state produces a catch-up jump):
  1. `absolutePause` → `last = now; return` (freeze everything).
  2. `offlineMode` → `accumulatedGameMs += (now - last) * speedMultiplier;
     last = now; return` (accumulate, defer production).
  3. otherwise → the existing live `game.tick(now - last, speedMultiplier)`.
  On switching offline mode **off**, invoke `simulate_offline` with
  `accumulatedGameMs` and the current `offline_ticks`, then reset the accumulator
  to 0. The readout displays `accumulatedGameMs`.
- Buttons + readout: a sibling block of `.speed-controls` in
  `.top-right-controls`. Two buttons styled like `.speed-btn` (with `active`
  states) and a duration label bound to `accumulatedGameMs` next to the
  offline-mode button. Reuse the vendored button styling; format the duration via
  the existing time/format helpers.

The save cadence (when we add periodic autosave) must be driven by real time
independent of `offlineMode`, `absolutePause`, and `speedMultiplier`, per the
convenience-vs-state distinction above — i.e. the backup timer keeps running even
under absolute pause (matching the original, which advances `backupTimer` from
`realDiff` regardless of game speed).

## Effort estimate

Low. The engine primitive is ~15 lines reusing `ticks`, plus one Tauri command
and a small amount of Vue state/UI. No new persistence, no clock in `ad-core`,
no changes to the save format. The main correctness concern — autobuyer
once-per-tick behaviour — is already handled by routing through a capped tick
budget rather than a single step.

## Resolved decisions

- **Speed during the pause:** integrated frame by frame, not sampled at resume.
  Game speed governs *how much* offline game-time accumulates.
- **Live readout:** show accumulated game-time next to the offline button,
  updating each frame.
- **Absolute Pause:** a separate dev toggle that freezes live ticks *and* offline
  accumulation.
- **Tick budget:** a single knob, the `offline_ticks` player slider. Budget =
  `min(game_ms / 50 ms, offline_ticks)`. Default 100,000 reproduces the original;
  the slider's max is extended (~10M–100M, TBC) so players can spend the faster
  engine on finer resolution. No separate modes — matching the original is the
  low end of the same slider. `offline_ticks` is the sole control over how
  accumulated offline time translates into progress.

## Sequencing note

The full feature has a natural ordering:

1. Engine `simulate_offline(game_ms, offline_ticks)` + the Offline-mode and
   Absolute-pause buttons with a hardcoded `offline_ticks = 100_000`. This alone
   delivers the interactive, speed-integrated offline behaviour.
2. Model `offline_ticks` as a player option: add it to `Options`
   (`ad-core/src/options.rs`), the save DTO read/write paths
   (`save/dto.rs`, `save/encode.rs` — the original stores it at
   `player.options.offlineTicks`), and the snapshot, then build the slider in the
   Gameplay options tab. The original's slider math
   (`(1 + v%9) × 10^floor(v/9)`) is worth reusing, extended to the new max.

Splitting it this way keeps the first PR small and avoids touching the save
format until the mechanic itself is proven.

## Open questions

1. Precise extended slider bounds and step spacing (target max ~10M–100M). Needs
   a quick benchmark of how long the engine takes to run, say, 100M ticks so the
   max stays interactive.
2. When the original stores `offlineTicks` beyond our extended range (or vice
   versa) on import, do we clamp or accept? (Lean: accept any positive value;
   it's just a resolution dial.)
3. Should offline mode have a minimum threshold (the original ignores < 10 s of
   load-time offline)? For a manual button, probably not — apply whatever
   accumulated.
4. Do we want a visible "X time away → Y gained" summary like the original's
   offline modal, or just silently apply and update the snapshot? (Recommend
   silent for v1; add a toast later.)
