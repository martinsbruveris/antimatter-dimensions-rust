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
    // "notation" | "importSave" | "hardReset" | "loadGame" | "backup"
    // (null = none). Centralised here so both InfoButtons and the keyboard
    // shortcuts (?, H) drive the same state; only one modal is open at once.
    openModal: null,
    // Dev-only: multiplier applied to wall-clock dt before ticking.
    speedMultiplier: 1,
    // Offline mode: while on, the live loop stops ticking the engine and instead
    // accumulates speed-scaled game-time (`accumulatedGameMs`), replayed as one
    // offline batch when switched off. See
    // design-docs/2026-06-30-offline-progress.md.
    offlineMode: false,
    accumulatedGameMs: 0,
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
  }),
  getters: {
    // Tabs currently visible, honouring each tab's optional unlock condition
    // (evaluated against the latest game snapshot). Hidden tabs are skipped
    // by the sidebar and by arrow-key navigation.
    visibleTabs() {
      const game = useGameStore();
      return TABS.filter((t) => !t.condition || t.condition(game.snapshot));
    },
    currentTab(state) {
      return (
        this.visibleTabs.find((t) => t.key === state.currentTabKey) ?? TABS[0]
      );
    },
    currentSubtab(state) {
      const tab = this.currentTab;
      const key = state.currentSubtabKey[tab.key] ?? tab.subtabs[0].key;
      return tab.subtabs.find((st) => st.key === key) ?? tab.subtabs[0];
    },
    currentComponent() {
      return this.currentSubtab.component;
    },
  },
  actions: {
    setTab(tabKey) {
      this.currentTabKey = tabKey;
    },
    setSubtab(tabKey, subtabKey) {
      this.currentTabKey = tabKey;
      this.currentSubtabKey[tabKey] = subtabKey;
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
      const subtabs = tab.subtabs;
      const idx = subtabs.findIndex((st) => st.key === this.currentSubtab.key);
      const next = (idx + delta + subtabs.length) % subtabs.length;
      this.setSubtab(tab.key, subtabs[next].key);
    },
    showModal(name) {
      this.openModal = name;
    },
    closeModal() {
      this.openModal = null;
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
    // game-time as one offline batch and, above the threshold, opens the
    // catch-up summary with before/after snapshots.
    async toggleOfflineMode() {
      if (!this.offlineMode) {
        this.accumulatedGameMs = 0;
        this.offlineMode = true;
        return;
      }

      const game = useGameStore();
      const gameMs = this.accumulatedGameMs;
      const before = game.snapshot;
      const offlineTicks =
        game.snapshot?.options?.offline_ticks ?? DEFAULT_OFFLINE_TICKS;
      this.offlineMode = false;
      this.accumulatedGameMs = 0;

      await game.simulateOffline(gameMs, offlineTicks);

      if (gameMs >= OFFLINE_SUMMARY_THRESHOLD_MS) {
        this.offlineSummary = {
          seconds: gameMs / 1000,
          before,
          after: game.snapshot,
        };
        this.openModal = "offlineSummary";
      }
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
