import { defineStore } from "pinia";

import { TABS } from "../config/tabs";

// UI-only navigation state (which tab/subtab is open). Kept separate from
// the `game` store, which mirrors the Rust snapshot.
export const useUiStore = defineStore("ui", {
  state: () => ({
    currentTabKey: "dimensions",
    // Remembers the last-open subtab per tab: { [tabKey]: subtabKey }.
    currentSubtabKey: {},
    // Which popup is open, if any: "help" | "info" | "credits" | "hotkeys"
    // (null = none). Centralised here so both InfoButtons and the keyboard
    // shortcuts (?, H) drive the same state; only one modal is open at once.
    openModal: null,
    // Dev-only: multiplier applied to wall-clock dt before ticking.
    speedMultiplier: 1,
  }),
  getters: {
    currentTab(state) {
      return TABS.find((t) => t.key === state.currentTabKey) ?? TABS[0];
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
      const idx = TABS.findIndex((t) => t.key === this.currentTabKey);
      const next = (idx + delta + TABS.length) % TABS.length;
      this.setTab(TABS[next].key);
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
  },
});
