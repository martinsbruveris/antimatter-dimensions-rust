import { defineStore } from "pinia";

import { TABS } from "../config/tabs";
import { useGameStore } from "./game";

// Fallback offline replay resolution, used only before the first snapshot
// arrives. Normally we read the player's `offline_ticks` option from the
// snapshot (set via the Gameplay tab slider).
const DEFAULT_OFFLINE_TICKS = 100000;

// Below this much accumulated offline game-time we apply it silently, with no
// catch-up summary (mirrors the original's 10 s away-modal threshold).
const OFFLINE_SUMMARY_THRESHOLD_MS = 10000;

// UI-only navigation state (which tab/subtab is open). Kept separate from
// the `game` store, which mirrors the Rust snapshot.
export const useUiStore = defineStore("ui", {
  state: () => ({
    currentTabKey: "dimensions",
    // Remembers the last-open subtab per tab: { [tabKey]: subtabKey }.
    currentSubtabKey: {},
    // Which popup is open, if any: "help" | "info" | "credits" | "hotkeys" |
    // "notation" | "importSave" | "hardReset" | "loadGame" | "backup" |
    // "animationOptions" | "infoDisplayOptions" | "awayProgressOptions" |
    // "hiddenTabs" (null = none). Centralised here so both InfoButtons and the
    // keyboard shortcuts (?, H) drive the same state; only one modal is open
    // at once.
    openModal: null,
    // Optional payload for the open modal (e.g. the glyph a sacrifice
    // confirmation targets). Cleared with the modal.
    modalPayload: null,
    // Whether Shift is currently held (original ui.view.shiftDown): overrides
    // the Info-Display hint options so hint text always shows while held.
    shiftDown: false,
    // Whether the How-To-Play tutorial emphasis (emphasizeH2P — the pulsing
    // gold "?" highlight) has already been shown. Pre-set to true so it does
    // NOT appear right now: it would overlay the always-visible dev speed/
    // offline/pause controls in the top-right. Once those controls become a
    // toggleable option, initialise this from whether they are hidden so the
    // emphasis returns when they are off.
    h2pEmphasisShown: true,
    // Dev-only: multiplier applied to wall-clock dt before ticking.
    speedMultiplier: 1,
    // Offline mode: while on, the live loop stops ticking the engine and instead
    // accumulates speed-scaled game-time (`accumulatedGameMs`), replayed as one
    // offline batch when switched off. See
    // docs/design/2026-06-30-offline-progress.md.
    offlineMode: false,
    accumulatedGameMs: 0,
    // Live progress of a running offline catch-up: { current, max, startTime }
    // (ticks done / total, and when the replay began). Drives the
    // OfflineProgressModal bar; null when no replay is in flight.
    offlineProgress: null,
    // True while an offline catch-up is replaying in chunks. The App.vue loop
    // suspends live ticking while set, so the replay isn't raced by the live
    // engine (relevant when a catch-up fires mid-session on import/backup load).
    offlineReplayActive: false,
    // Absolute pause (dev): freezes everything — live ticks AND offline
    // accumulation. Takes priority over offline mode.
    absolutePause: false,
    // Catch-up summary shown after an offline replay of >= 10 s:
    // { seconds, before, after } snapshots. Drives the offlineSummary modal.
    offlineSummary: null,
    // Transient toast notifications shown top-right (the blue "info" popups the
    // original triggers e.g. when toggling autobuyers via the keyboard). Each:
    // { id, text, typeClass, entering, leaving }. Mirrors core/notify.js.
    notifications: [],
    // Monotonic id source for notifications.
    nextNotificationId: 0,
    // Achievements tab: hide rows whose achievements are all unlocked. Lives
    // here (not in the tab component) so it survives the tab unmounting on
    // navigation, mirroring the original's persisted player option.
    hideCompletedAchievementRows: false,
  }),
  getters: {
    // Whether the player has hidden `subtab` via the Modify Visible Tabs modal
    // (original SubtabState.isHidden): its bit in the engine's
    // `hidden_subtab_bits` is set and it is hidable. The bit positions are the
    // *original game's* tab/subtab ids (`hideId` in config/tabs.js), so the
    // state round-trips through real saves.
    subtabIsHidden() {
      const game = useGameStore();
      return (subtab) => {
        if (subtab.hidable === false) return false;
        const [tabId, subtabId] = subtab.hideId;
        const bits = game.snapshot?.options?.hidden_subtab_bits?.[tabId] ?? 0;
        return (bits & (1 << subtabId)) !== 0;
      };
    },
    // Whether the player has hidden `tab` (original TabState.isHidden): its
    // own bit is set, or every subtab of it is unavailable — and it is hidable.
    tabIsHidden() {
      const game = useGameStore();
      return (tab) => {
        if (tab.hidable === false) return false;
        const bits = game.snapshot?.options?.hidden_tab_bits ?? 0;
        const hasVisibleSubtab = this.visibleSubtabs(tab).length > 0;
        return (bits & (1 << tab.hideId)) !== 0 || !hasVisibleSubtab;
      };
    },
    // Tabs currently visible, honouring each tab's optional unlock condition
    // (evaluated against the latest game snapshot) and the hidden-tab option
    // bits. Hidden tabs are skipped by the sidebar and by arrow-key
    // navigation; like the original (TabState.isAvailable), the tab currently
    // open stays visible even if hidden, so the view never yanks away.
    visibleTabs(state) {
      const game = useGameStore();
      return TABS.filter(
        (t) =>
          (!t.condition || t.condition(game.snapshot)) &&
          (t.key === state.currentTabKey || !this.tabIsHidden(t)),
      );
    },
    // A subtab may carry its own `condition(snapshot)` (e.g. Break Infinity, shown
    // only after breaking) — filter those the same way as tabs, then drop
    // player-hidden subtabs (except the one currently open).
    visibleSubtabs(state) {
      const game = useGameStore();
      return (tab) =>
        tab.subtabs.filter(
          (st) =>
            (!st.condition || st.condition(game.snapshot)) &&
            ((tab.key === state.currentTabKey &&
              st.key === state.currentSubtabKey[tab.key]) ||
              !this.subtabIsHidden(st)),
        );
    },
    currentTab(state) {
      return (
        this.visibleTabs.find((t) => t.key === state.currentTabKey) ?? TABS[0]
      );
    },
    currentSubtab(state) {
      const tab = this.currentTab;
      const subtabs = this.visibleSubtabs(tab);
      // Fall back to the tab's own first subtab if every visible one is hidden.
      const fallback = subtabs[0] ?? tab.subtabs[0];
      const key = state.currentSubtabKey[tab.key] ?? fallback.key;
      return subtabs.find((st) => st.key === key) ?? fallback;
    },
    currentComponent() {
      return this.currentSubtab.component;
    },
    // Whether `subtab` of `tab` shows the yellow `!` notification badge:
    // present in the snapshot's badge keys and not the currently open subtab
    // (the open subtab's badge is acknowledged, never displayed — the original
    // never adds a key for the tab being viewed).
    subtabHasNotification() {
      const game = useGameStore();
      return (tab, subtab) =>
        (game.snapshot?.tab_notifications ?? []).includes(tab.key + subtab.key) &&
        !(tab.key === this.currentTabKey && subtab.key === this.currentSubtab.key);
    },
    // A tab is badged when any of its visible subtabs is.
    tabHasNotification() {
      return (tab) =>
        this.visibleSubtabs(tab).some((st) => this.subtabHasNotification(tab, st));
    },
  },
  actions: {
    setTab(tabKey) {
      this.currentTabKey = tabKey;
      this.ackTabNotification();
    },
    setSubtab(tabKey, subtabKey) {
      this.currentTabKey = tabKey;
      this.currentSubtabKey[tabKey] = subtabKey;
      this.ackTabNotification();
    },
    // Acknowledge the open subtab's notification badge, if it carries one:
    // called on navigation (mirroring the original TabState.show's
    // `tabNotifications.delete`) and after every tick, which covers a
    // notification firing while its tab is already open — the original avoids
    // that case by excluding the current tab at trigger time; our engine
    // doesn't know the open tab, so the frontend acknowledges instead.
    ackTabNotification() {
      const game = useGameStore();
      const key = this.currentTabKey + this.currentSubtab.key;
      if (game.snapshot?.tab_notifications?.includes(key)) {
        game.tabNotificationSeen(key);
      }
    },
    // Cycle through tabs (delta -1/+1) with wraparound, mirroring the
    // original's Up/Down arrow tab movement.
    moveTab(delta) {
      const tabs = this.visibleTabs;
      const idx = tabs.findIndex((t) => t.key === this.currentTabKey);
      const next = (idx + delta + tabs.length) % tabs.length;
      this.setTab(tabs[next].key);
    },
    // Cycle through the current tab's subtabs (delta -1/+1) with wraparound,
    // mirroring the original's Left/Right arrow subtab movement.
    moveSubtab(delta) {
      const tab = this.currentTab;
      const subtabs = this.visibleSubtabs(tab);
      // Every subtab can be hidden (e.g. by an imported save's hidden-tab
      // bits) — nothing to cycle through then.
      if (subtabs.length === 0) return;
      const idx = subtabs.findIndex((st) => st.key === this.currentSubtab.key);
      const next = (idx + delta + subtabs.length) % subtabs.length;
      this.setSubtab(tab.key, subtabs[next].key);
    },
    showModal(name, payload = null) {
      this.openModal = name;
      this.modalPayload = payload;
    },
    closeModal() {
      this.openModal = null;
      this.modalPayload = null;
    },
    // Open `name`, or close it if it's already the open modal (matches the
    // original's ?/H toggle behaviour).
    toggleModal(name) {
      this.openModal = this.openModal === name ? null : name;
    },
    setSpeed(multiplier) {
      this.speedMultiplier = multiplier;
    },
    // Toggle the absolute (dev) pause. Freezes the live loop and offline
    // accumulation alike; the App.vue loop checks this first.
    toggleAbsolutePause() {
      this.absolutePause = !this.absolutePause;
    },
    // Accumulate speed-scaled game-time while offline mode is engaged. Called
    // each frame by the App.vue loop.
    accumulateOffline(gameMs) {
      this.accumulatedGameMs += gameMs;
    },
    // Toggle Offline mode. Turning it on resets the accumulator and freezes the
    // live loop (handled in App.vue). Turning it off replays the accumulated
    // game-time as an offline catch-up (progress modal + summary).
    async toggleOfflineMode() {
      if (!this.offlineMode) {
        this.accumulatedGameMs = 0;
        this.offlineMode = true;
        return;
      }

      const game = useGameStore();
      const gameMs = this.accumulatedGameMs;
      const offlineTicks =
        game.snapshot?.options?.offline_ticks ?? DEFAULT_OFFLINE_TICKS;
      this.offlineMode = false;
      this.accumulatedGameMs = 0;

      await this.runOfflineReplay(gameMs, offlineTicks);
    },
    // Replay `gameMs` of offline game-time at `offlineTicks` resolution — the one
    // path shared by startup, save/backup load, and the dev Offline button.
    //
    // Below the 10 s threshold it applies silently in a single engine call. Above
    // it, the replay is split into 100 batches so the OfflineProgressModal bar
    // fills visibly (near-instant for short away-times, meaningful for large tick
    // budgets), then the "While you were away…" summary is shown. The 10 s
    // summary threshold is a deliberate divergence from the original's 600 s
    // AwayProgressModal gate — see docs/design/2026-06-30-offline-progress.md.
    async runOfflineReplay(gameMs, offlineTicks) {
      if (!(gameMs > 0)) return;
      const game = useGameStore();
      const before = game.snapshot;

      if (gameMs < OFFLINE_SUMMARY_THRESHOLD_MS) {
        await game.simulateOffline(gameMs, offlineTicks);
        return;
      }

      const plan = await game.offlinePlan(gameMs, offlineTicks);
      const total = plan.ticks;
      if (total <= 0) return;

      this.offlineReplayActive = true;
      this.offlineProgress = { current: 0, max: total, startTime: Date.now() };
      this.openModal = "offlineProgress";

      // Distribute `total` ticks across 100 chunks (remainder front-loaded), so
      // the summed replay is exactly `total` ticks — identical to one batch.
      const CHUNKS = 100;
      const base = Math.floor(total / CHUNKS);
      const extra = total % CHUNKS;
      let done = 0;
      for (let c = 0; c < CHUNKS; c++) {
        const n = base + (c < extra ? 1 : 0);
        if (n > 0) {
          await game.offlineChunk(plan.tick_size_ms, n);
          done += n;
          this.offlineProgress.current = done;
        }
        // Yield to the browser so the progress bar repaints between chunks.
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }

      // Reseed achievements silently: offline gains are summarised by the modal,
      // not fired as a per-achievement toast storm.
      game.notifyNewAchievements(true);

      this.offlineReplayActive = false;
      this.offlineProgress = null;
      if (this.openModal === "offlineProgress") this.openModal = null;

      this.offlineSummary = {
        seconds: gameMs / 1000,
        before,
        after: game.snapshot,
      };
      this.openModal = "offlineSummary";
    },
    // Show a transient toast, mirroring core/notify.js: it slides in (enter
    // animation), stays for `duration` ms, then slides out (leave animation)
    // and is removed. `type` selects the colour (o-notification--<type>);
    // "info" is the blue popup. Clicking it dismisses early.
    notify(text, type = "info", duration = 2000) {
      const id = this.nextNotificationId++;
      this.notifications.push({
        id,
        text,
        typeClass: `o-notification--${type}`,
        entering: true,
        leaving: false,
      });
      // Drop the enter class once the slide-in finishes (matches notify.js).
      setTimeout(() => {
        const n = this.notifications.find((x) => x.id === id);
        if (n) n.entering = false;
      }, 500);
      setTimeout(() => this.dismissNotification(id), duration);
    },
    // Begin the leave animation for a notification, then remove it once the
    // 0.25s slide-out has elapsed. Idempotent (a second call is a no-op).
    dismissNotification(id) {
      const n = this.notifications.find((x) => x.id === id);
      if (!n || n.leaving) return;
      n.entering = false;
      n.leaving = true;
      setTimeout(() => {
        this.notifications = this.notifications.filter((x) => x.id !== id);
      }, 500);
    },
  },
});
