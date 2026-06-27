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
    // Advance the engine by `repeats` discrete ticks of `dtMs` each (the dev
    // game-speed control passes the multiplier as `repeats`), returning a
    // single snapshot. Looping in Rust avoids one IPC round-trip per tick.
    async tick(dtMs, repeats = 1) {
      this.snapshot = await invoke("tick_and_get_state", { dtMs, repeats });
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
    // First Big Crunch (Infinity): resets all pre-Infinity progress in the
    // engine. Available once `snapshot.can_big_crunch` is true.
    bigCrunch() {
      return invoke("big_crunch");
    },
    // --- Autobuyers ---
    // Unlock an AD autobuyer's slow version (no antimatter cost; only succeeds
    // once the requirement is met).
    unlockAdAutobuyer(tier) {
      return invoke("unlock_ad_autobuyer", { tier });
    },
    toggleAdAutobuyer(tier) {
      return invoke("toggle_ad_autobuyer", { tier });
    },
    toggleAdAutobuyerMode(tier) {
      return invoke("toggle_ad_autobuyer_mode", { tier });
    },
    unlockTickspeedAutobuyer() {
      return invoke("unlock_tickspeed_autobuyer");
    },
    toggleTickspeedAutobuyer() {
      return invoke("toggle_tickspeed_autobuyer");
    },
    // Global pause/resume (the `a` hotkey and the toggles bar).
    toggleAutobuyers() {
      return invoke("toggle_autobuyers");
    },
    // "Enable/Disable all autobuyers": sets the active flag on every unlocked
    // autobuyer.
    setAllAutobuyersActive(active) {
      return invoke("set_all_autobuyers_active", { active });
    },
    // --- Options ---
    // Enable/disable keyboard shortcuts (original `player.options.hotkeys`).
    setHotkeys(enabled) {
      return invoke("set_hotkeys", { enabled });
    },
    // Game-loop cadence in ms (original `player.options.updateRate`); the
    // engine clamps to the 33–200 slider range.
    setUpdateRate(rate) {
      return invoke("set_update_rate", { rate });
    },
  },
});
