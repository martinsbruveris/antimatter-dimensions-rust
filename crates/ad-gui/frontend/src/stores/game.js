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
    // "Until 10" fills the current group (capped by affordability);
    // "Buy 1" buys a single dimension.
    buyDim(tier) {
      return invoke(this.buyUntil10 ? "buy_until_10" : "buy_dimension", { tier });
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
