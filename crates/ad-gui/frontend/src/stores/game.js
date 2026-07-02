import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

import { useUiStore } from "./ui";
import { NORMAL_ACHIEVEMENTS } from "../data/achievements";

// id → display name, for the unlock toast.
const ACHIEVEMENT_NAMES = new Map(NORMAL_ACHIEVEMENTS.map((a) => [a.id, a.name]));

// The Rust engine is authoritative. This store holds the latest
// per-tick snapshot for display and dispatches actions over Tauri IPC.
// `buyUntil10` is UI-only state (never part of the engine's GameState).
export const useGameStore = defineStore("game", {
  state: () => ({
    snapshot: null,
    buyUntil10: true,
    // Ids of achievements unlocked as of the last snapshot we diffed, so a
    // freshly-unlocked one fires exactly one toast. `null` until first seeded.
    seenAchievements: null,
    // Wall-clock (ms) of the last successful save and a "now" clock refreshed
    // every animation frame by App.vue's loop, for the bottom-left save timer
    // (SaveTimer.vue). Keeping `nowMs` reactive lets `msSinceSave` re-render
    // without a separate interval, and it advances even while paused/offline.
    lastSaveTime: Date.now(),
    nowMs: Date.now(),
  }),
  getters: {
    // Milliseconds elapsed since the last save (>= 0).
    msSinceSave(state) {
      return Math.max(0, state.nowMs - state.lastSaveTime);
    },
  },
  actions: {
    // Fire a success toast (mirroring the original's
    // `GameUI.notify.success`) for each achievement newly present in the
    // current snapshot. Seeds silently the first time and whenever the state
    // is replaced wholesale (load/import/reset), so those don't spam toasts.
    notifyNewAchievements(seedOnly = false) {
      const ids = this.snapshot?.unlocked_achievements ?? [];
      if (!seedOnly && this.seenAchievements !== null) {
        const prev = new Set(this.seenAchievements);
        const ui = useUiStore();
        for (const id of ids) {
          if (!prev.has(id)) {
            const name = ACHIEVEMENT_NAMES.get(id) ?? `Achievement ${id}`;
            ui.notify(`Achievement: ${name}`, "success", 3000);
          }
        }
      }
      this.seenAchievements = ids;
    },
    // Advance the engine by `repeats` discrete ticks of `dtMs` each (the dev
    // game-speed control passes the multiplier as `repeats`), returning a
    // single snapshot. Looping in Rust avoids one IPC round-trip per tick.
    async tick(dtMs, repeats = 1) {
      this.snapshot = await invoke("tick_and_get_state", { dtMs, repeats });
      this.notifyNewAchievements();
    },
    // Replay `gameMs` of offline game-time (already speed-scaled) at the
    // resolution set by `offlineTicks`, all at once. Used for sub-threshold
    // catch-ups (no progress modal). Reseeds achievements silently.
    async simulateOffline(gameMs, offlineTicks) {
      this.snapshot = await invoke("simulate_offline", { gameMs, offlineTicks });
      this.notifyNewAchievements(true);
    },
    // The engine's offline replay plan for `gameMs`: { ticks, tick_size_ms }.
    // The UI store uses it to chunk the catch-up behind the progress bar.
    offlinePlan(gameMs, offlineTicks) {
      return invoke("offline_plan", { gameMs, offlineTicks });
    },
    // One offline replay batch: advance `repeats` discrete ticks of `tickSizeMs`
    // and update the snapshot. Achievement toasts are suppressed here (the
    // caller reseeds once at the end); the offline modal summarises the gains.
    async offlineChunk(tickSizeMs, repeats) {
      this.snapshot = await invoke("tick_and_get_state", {
        dtMs: tickSizeMs,
        repeats,
      });
    },
    // The current engine snapshot without advancing time (startup seed).
    getState() {
      return invoke("get_state");
    },
    // Consumes the startup offline gap (ms) detected at load, once.
    takePendingOffline() {
      return invoke("take_pending_offline");
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
    // Big Crunch (Infinity): resets all pre-Infinity progress in the engine and
    // awards Infinity Points. On the *first* crunch, navigate to the new Infinity
    // tab (mirrors the original's `Tab.infinity.upgrades.show()` in
    // `bigCrunchTabChange`); the tab becomes visible on the next snapshot.
    async bigCrunch() {
      const wasFirstInfinity = !this.snapshot?.infinity_unlocked;
      await invoke("big_crunch");
      if (wasFirstInfinity) {
        useUiStore().setSubtab("infinity", "upgrades");
      }
    },
    // Buy an Infinity Upgrade by its original save id (e.g. "timeMult").
    buyInfinityUpgrade(id) {
      return invoke("buy_infinity_upgrade", { id });
    },
    // --- Confirmation-gated requests ---
    // Each click handler routes through one of these: if the matching
    // confirmation option is on, open the explanatory modal (whose Confirm
    // button performs the action); otherwise perform it directly. Mirrors the
    // original's `manualRequest*` / `sacrificeBtnClick` indirection.
    requestDimBoost() {
      if (!this.snapshot?.can_dim_boost) return;
      const ui = useUiStore();
      if (this.snapshot?.options?.confirmations?.dimension_boost) {
        ui.showModal("dimboostConfirm");
      } else {
        this.buyDimBoost();
      }
    },
    requestGalaxy() {
      if (!this.snapshot?.can_buy_galaxy) return;
      const ui = useUiStore();
      if (this.snapshot?.options?.confirmations?.antimatter_galaxy) {
        ui.showModal("galaxyConfirm");
      } else {
        this.buyGalaxy();
      }
    },
    requestSacrifice() {
      if (!this.snapshot?.can_sacrifice) return;
      const ui = useUiStore();
      if (this.snapshot?.options?.confirmations?.sacrifice) {
        ui.showModal("sacrificeConfirm");
      } else {
        this.sacrifice();
      }
    },
    // Big Crunch request. Mirrors the original `manualBigCrunchResetRequest`:
    // the modal shows only when the bigCrunch confirmation is on AND it is the
    // first infinity (or, once Break Infinity lands, `player.break`). So pre-break
    // the first crunch pops the explanatory modal and every later crunch goes
    // through directly. The post-break "IP/infinities gained" modal + disable
    // checkbox arrive with Feature 2.3.
    requestBigCrunch() {
      if (!this.snapshot?.can_big_crunch) return;
      const ui = useUiStore();
      const firstInfinity = !this.snapshot?.infinity_unlocked;
      if (this.snapshot?.options?.confirmations?.big_crunch && firstInfinity) {
        ui.showModal("bigCrunchConfirm");
      } else {
        this.bigCrunch();
      }
    },
    // Flip a confirmation toggle (original `player.options.confirmations.*`);
    // `kind` is the camelCase action name the engine expects.
    setConfirmation(kind, enabled) {
      return invoke("set_confirmation", { kind, enabled });
    },
    // Hard reset: wipes the current save slot back to a fresh state (persisted).
    async hardReset() {
      this.snapshot = await invoke("hard_reset");
      this.lastSaveTime = Date.now();
      this.notifyNewAchievements(true);
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
    // Number-formatting notation (original `player.options.notation`); the
    // engine ignores names outside its known set.
    setNotation(notation) {
      return invoke("set_notation", { notation });
    },
    // Offline replay resolution (original `player.options.offlineTicks`); the
    // engine accepts any positive value (we diverge from the original's range).
    setOfflineTicks(ticks) {
      return invoke("set_offline_ticks", { ticks });
    },
    // Exponent Notation digit thresholds (original
    // `player.options.notationDigits`); the engine clamps to [3, 15] and keeps
    // the notation threshold >= the comma threshold.
    setNotationDigits(comma, notation) {
      return invoke("set_notation_digits", { comma, notation });
    },
    // --- Save / Load ---
    // Returns the current game state as an AD-compatible save string.
    exportSave() {
      return invoke("export_save");
    },
    // Installs a freshly loaded/imported state ({ view, offline_ms }) and replays
    // the offline gap the save carried (from its lastUpdate). Shared by the
    // paste/file import and backup-load paths.
    async applyLoadResult(res) {
      this.snapshot = res.view;
      this.lastSaveTime = Date.now();
      this.notifyNewAchievements(true);
      const ui = useUiStore();
      await ui.runOfflineReplay(
        res.offline_ms,
        this.snapshot?.options?.offline_ticks ?? 100000,
      );
    },
    // Imports a save from a text string. Replaces the running game state
    // (persisted immediately by the backend), then catches up offline progress.
    async importSave(text) {
      await this.applyLoadResult(await invoke("import_save", { text }));
    },
    // Exports the save to a user-chosen file via native Save As dialog. The
    // backend uses the engine-owned `saveFileName` option as the default name.
    exportSaveToFile() {
      return invoke("export_save_to_file");
    },
    // Imports a save from a user-chosen file via native Open dialog.
    async importSaveFromFile() {
      await this.applyLoadResult(await invoke("import_save_from_file"));
    },
    // --- On-disk persistence (save slots + backups) ---
    // Writes the current game to disk (manual "Save game", autosave, Cmd/Ctrl+S).
    async saveGame() {
      await invoke("save_game");
      this.lastSaveTime = Date.now();
    },
    // Switches the active save slot (persists current, loads target).
    async switchSaveSlot(index) {
      this.snapshot = await invoke("switch_save_slot", { index });
      this.lastSaveTime = Date.now();
      this.notifyNewAchievements(true);
    },
    // Per-slot summaries for the "Choose save" modal.
    getSaveSlots() {
      return invoke("get_save_slots");
    },
    // Writes the current game into one automatic backup slot (online timers +
    // the manual reserve slot).
    triggerBackup(slot) {
      return invoke("trigger_backup", { slot });
    },
    // Per-backup-slot summaries for the Backup menu.
    getBackups() {
      return invoke("get_backups");
    },
    // Loads a backup slot into the running game (reserves the current save
    // first), then catches up the offline gap the backup carried.
    async loadBackup(slot) {
      await this.applyLoadResult(await invoke("load_backup", { slot }));
    },
    // Exports all populated backups of the current slot as one file.
    exportBackupsToFile() {
      return invoke("export_backups_to_file");
    },
    // Imports a backup-bundle file into the current slot's backup slots.
    importBackupsFromFile() {
      return invoke("import_backups_from_file");
    },
    // --- Saving options ---
    // Autosave cadence in ms (original `autosaveInterval`; engine clamps 10–60 s).
    setAutosaveInterval(interval) {
      return invoke("set_autosave_interval", { interval });
    },
    // Header "time since save" indicator (original `showTimeSinceSave`).
    setShowTimeSinceSave(enabled) {
      return invoke("set_show_time_since_save", { enabled });
    },
    // Custom save-file name (original `saveFileName`); the engine sanitizes it.
    setSaveFileName(name) {
      return invoke("set_save_file_name", { name });
    },
  },
});
