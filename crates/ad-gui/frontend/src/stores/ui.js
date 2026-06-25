import { defineStore } from "pinia";

import { TABS } from "../config/tabs";

// UI-only navigation state (which tab/subtab is open). Kept separate from
// the `game` store, which mirrors the Rust snapshot.
export const useUiStore = defineStore("ui", {
  state: () => ({
    currentTabKey: "dimensions",
    // Remembers the last-open subtab per tab: { [tabKey]: subtabKey }.
    currentSubtabKey: {},
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
  },
});
