---
status: Implemented
---

# Tab notification badges (yellow `!` on tabs)

2026-07-04

The original game directs the player's attention to newly relevant content with a
pulsing yellow exclamation mark on tab and subtab buttons: after the first Infinity
the Challenges tab lights up, unlocking an Infinity Challenge badges the IC subtab,
an affordable autobuyer badges Automation, and so on. This doc describes how the
original implements the system and the design for porting it.

## 1. How the original works

### 1.1 State — two persisted `player` fields

- `player.tabNotifications` — a JS `Set` of strings, serialized to the save as an
  array (the serializer converts Sets to arrays). Each element is the
  concatenation `parentTabKey + subtabKey`, e.g. `"challengesnormal"`,
  `"infinityreplicanti"`. A key being present means: that subtab (and therefore
  its parent tab) currently shows the `!`.
- `player.triggeredTabNotificationBits` — a bitmask over notification-definition
  ids recording which notifications have *ever fired*, so each fires only once
  per save (unless explicitly re-armed via `clearTrigger`).

### 1.2 Definitions

`src/core/secret-formula/tab-notifications.js` defines 17 notifications, each:

- `id` — the bit in `triggeredTabNotificationBits`;
- `tabsToHighLight` — a list of `{ parent, tab }` targets;
- `condition()` — must hold for the trigger to fire;
- `events` (optional) — EventHub events on which `tryTrigger` runs automatically.
  Definitions without `events` are triggered by explicit `tryTrigger()` calls at
  the relevant mechanic's code site.

### 1.3 Trigger / clear semantics (`src/core/tab-notifications.js`)

`tryTrigger()`: if `condition()` holds and the id's bit is not yet set, add every
target key to the set **except the tab+subtab the player is currently viewing**,
set the bit, and force-unhide the target tabs (players can hide tabs; a
notification overrides that).

`clearTrigger()`: clear the bit and remove the target keys — used to re-arm
repeatable notifications (`newAutobuyer`, `ICUnlock`).

Clearing on view: `TabState.show()` (`src/core/tabs.js`) deletes the
`tabKey + subtabKey` of the newly shown subtab from the set. That is the only
acknowledgement path.

### 1.4 Rendering

`SubtabState.hasNotification` = set membership of `parent.key + key`;
`TabState.hasNotification` = any subtab has one. `ModernTabButton.vue` renders
`<div class="fas fa-circle-exclamation l-notification-icon" />` inside the tab
button and on each subtab icon. `l-notification-icon` (vendored `styles.css`)
positions it top-right, colors it `--color-notification`, and pulses it with the
`a-notification-glow` animation. We already use this exact class for the tutorial
`!`, so no new CSS is needed.

### 1.5 Notifications within our current feature frontier

| id | name | trigger | condition | targets |
|----|------|---------|-----------|---------|
| 0 | `firstInfinity` | `BIG_CRUNCH_BEFORE` event | infinity not yet unlocked (first crunch) | infinity/upgrades, challenges/normal, statistics/multipliers |
| 1 | `breakInfinity` | explicit, from the Big Crunch autobuyer's `upgradeInterval` | its interval is maxed (100 ms floor) | infinity/break |
| 3 | `ICUnlock` | explicit: from `breakInfinity()` (game.js) and from the antimatter setter when AM crosses a locked IC's `unlockAM` (`notifyICUnlock`, clear-then-try so it re-fires per IC); cleared in `handleChallengeCompletion` on an IC's first completion | true (pre-Eternity) | challenges/infinity |
| 4 | `replicanti` | `BIG_CRUNCH_AFTER` event | IP ≥ 1e140 | infinity/replicanti |
| 12 | `newAutobuyer` | explicit, from the antimatter setter whenever AM ≥ the cheapest not-yet-unlocked AD/tickspeed autobuyer requirement (`GameCache.cheapestAntimatterAutobuyer`); always clear-then-try, so leaving the Automation tab with an affordable unlock immediately re-badges it; cleared on autobuyer unlock and NC completion | true (pre-Pelle) | automation/autobuyers |

`IDUnlock` (id 2) is defined in the database but has **no trigger site** in the
current original codebase — dead config. We skip it (its bit still round-trips
through the save untouched).

The remaining definitions (ids 5–11, 13–16) are Eternity-and-later; they get added
alongside those features.

### 1.6 Interactions worth noting

- `firstInfinity` fires *before* the first crunch's tab switch, while the player
  is on the Dimensions tab, so all three targets get keys; the automatic
  navigation to infinity/upgrades then immediately clears that one, leaving
  Challenges and Statistics badged. Our first-crunch flow (store `bigCrunch`
  navigating to the Infinity tab) reproduces this ordering for free.
- `newAutobuyer`'s clear-then-try means its bit never gates it; the *displayed*
  badge is governed purely by set membership and the current-tab exclusion.

## 2. Port design

Engine-owned state and triggers (like achievements and the tutorial); the
frontend renders the snapshot and acknowledges views. Wire format stays
byte-compatible: our `config/tabs.js` tab/subtab keys already match the
original's, so the concatenated keys round-trip verbatim.

### 2.1 ad-core: `tab_notifications.rs`

- `TabNotificationId` enum with the original ids: `FirstInfinity = 0`,
  `BreakInfinity = 1`, `IcUnlock = 3`, `Replicanti = 4`, `NewAutobuyer = 12`.
  Each id knows its `condition(&GameState)` and `target_keys()` (the
  concatenated strings).
- On `GameState`:
  - `tab_notifications: BTreeSet<String>` ↔ `player.tabNotifications`
    (BTreeSet for deterministic serialization order);
  - `triggered_tab_notification_bits: u32` ↔
    `player.triggeredTabNotificationBits`. Loaded/saved verbatim, so bits for
    unmodelled notifications survive a round-trip.
- Methods mirroring the JS:
  - `try_trigger_tab_notification(id)` — condition + bit check, insert all
    target keys, set the bit. No current-tab exclusion (engine doesn't know the
    open tab; see §2.4) and no unhide step (we don't model hidden tabs).
  - `clear_tab_notification_trigger(id)` — clear bit, remove target keys.
  - `tab_notification_seen(key)` — remove one key (the `show()` acknowledgement;
    called by the frontend over IPC).

### 2.2 Trigger hook sites (all in existing engine paths)

- `crunch.rs::big_crunch_reset` — at goal, **before** setting
  `infinity_unlocked`: `FirstInfinity` (= `BIG_CRUNCH_BEFORE`). After the reset
  and IP award: `Replicanti` (= `BIG_CRUNCH_AFTER`, condition IP ≥ 1e140).
- `autobuyers.rs::upgrade_autobuyer_interval` — after a successful Big Crunch
  interval upgrade: `BreakInfinity` (condition: interval now maxed).
- `crunch.rs::break_infinity` — `IcUnlock` (mirrors game.js).
- `tick.rs::tick` — where `records.max_am_this_eternity` advances: if the update
  newly unlocks an IC (prev peak < `unlockAM` ≤ new peak), clear-then-try
  `IcUnlock` (mirrors `notifyICUnlock` in the antimatter setter).
- `challenges.rs::handle_challenge_completion` — when an IC is running and not
  yet completed, clear `IcUnlock` (so the next IC unlock re-badges);
  NC completion also clears `NewAutobuyer` (its unlock set changed).
- `NewAutobuyer`: checked in `tick` — when `total_antimatter` meets the cheapest
  requirement among AD/tickspeed autobuyers that are neither bought nor
  challenge-unlocked, clear-then-try (the original checks on every antimatter
  set; once per tick is equivalent). Also cleared in
  `unlock_ad_autobuyer` / `unlock_tickspeed_autobuyer`.

Divergence note: the original's `cheapestAntimatterAutobuyer` compares against
*current* antimatter; our unlock gate is all-time `total_antimatter`
(matching our autobuyer unlock model), so the check uses that — the badge and
the purchasable unlock appear together, which is the intent.

### 2.3 Save codec

- `PlayerDTO`: `tab_notifications: Vec<String>` + a `u32` for
  `triggeredTabNotificationBits`; mapped verbatim in `from_save_dto`.
- `encode.rs::overlay`: write both fields (the template already carries
  `"tabNotifications": []` and `"triggeredTabNotificationBits": 0`).
- Keys we don't render (e.g. `statisticsmultipliers`, or keys written by a real
  late-game save) are preserved untouched — they simply match no sidebar entry.

### 2.4 GUI

- `GameView.tab_notifications: Vec<String>` (sorted, from the BTreeSet).
- New command `tab_notification_seen(key)` → engine method; store action
  `tabNotificationSeen`.
- `stores/ui.js`: on `setTab`/`setSubtab` (and arrow-key navigation, which routes
  through them), dispatch `tabNotificationSeen(newTabKey + newSubtabKey)`.
- `Sidebar.vue`: per-subtab badge when `snapshot.tab_notifications` contains
  `tab.key + subtab.key` **and** that subtab is not the currently open one;
  per-tab badge when any of its subtabs has a badge. The current-subtab
  exclusion replaces the original's exclusion-at-trigger-time: the engine may
  add (or re-add, for `newAutobuyer`) a key for the open tab, but it is never
  displayed there, and navigation acknowledges it — observable behavior matches
  the original, including `newAutobuyer` re-badging when you leave Automation
  with an affordable unlock pending.

### 2.5 Out of scope for now

- Eternity+ notifications (ids 5–11, 13–16) — added with their features.
- Force-unhiding hidden tabs (we don't model tab hiding).
- The "You have unlocked Infinity Challenge N" toast that accompanies
  `ICUnlock` — a frontend diff of `infinity_challenges[].is_unlocked`, like the
  achievement toast; a natural follow-up.

## 3. Testing

- Engine unit tests per trigger: first crunch badges the three targets and only
  once; second crunch doesn't re-badge; replicanti badge appears only at
  IP ≥ 1e140; maxing the Big Crunch interval badges Break Infinity; IC unlock
  crossing badges (and re-badges after a completion clear); autobuyer
  affordability re-arms after `seen`.
- Save round-trip test: keys + bits survive encode → decode, including keys and
  bits we don't model.
