import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

// The Rust engine is authoritative. This store holds the latest
// per-tick snapshot for display and dispatches actions over Tauri IPC.
// `buyUntil10` is UI-only state (never part of the engine's GameState).
export const useGameStore = defineStore("game", {
  state: () => ({
    snapshot: null,
    buyUntil10: true,
  }),
  actions: {
    async tick(dtMs) {
      this.snapshot = await invoke("tick_and_get_state", { dtMs });
    },
    toggleBuyMode() {
      this.buyUntil10 = !this.buyUntil10;
    },
    // "Until 10" fills the current group (capped by affordability).
    buyDimMany(tier) {
      return invoke("buy_until_10", { tier });
    },
    // Buys a single dimension.
    buyDimSingle(tier) {
      return invoke("buy_dimension", { tier });
    },
    // Click handler: follows the buy-mode toggle. Keyboard shortcuts call
    // buyDimMany / buyDimSingle directly (1-8 vs Shift+1-8), independent of
    // the toggle, matching the original.
    buyDim(tier) {
      return this.buyUntil10 ? this.buyDimMany(tier) : this.buyDimSingle(tier);
    },
    buyTickspeed() {
      return invoke("buy_tickspeed");
    },
    buyMaxTickspeed() {
      return invoke("buy_max_tickspeed");
    },
    buyDimBoost() {
      return invoke("buy_dim_boost");
    },
    buyGalaxy() {
      return invoke("buy_galaxy");
    },
    sacrifice() {
      return invoke("sacrifice");
    },
    maxAll() {
      return invoke("max_all");
    },
  },
});
