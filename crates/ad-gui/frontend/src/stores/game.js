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
      // Automator `notify` commands queue toasts engine-side; the tick
      // response drains them (`GameUI.notify.automator` is the purple toast).
      const ui = useUiStore();
      for (const text of this.snapshot?.automator?.notifications ?? []) {
        ui.notify(text, "automator", 3000);
      }
      // A badge that landed on the subtab the player is looking at is
      // acknowledged immediately (never displayed).
      ui.ackTabNotification();
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
    // awards Infinity Points. Tab change mirrors the original's
    // `bigCrunchTabChange`: it runs *after* Infinities are awarded, so its
    // first-infinity branch (Infinity tab) never fires — the effective behavior
    // is that an "early game" crunch (best infinity > 60 s, pre-break) or a
    // crunch that finishes a challenge lands on the Antimatter Dimensions tab.
    async bigCrunch() {
      const wasInChallenge =
        (this.snapshot?.challenges ?? []).some((c) => c.is_running) ||
        (this.snapshot?.infinity_challenges ?? []).some((c) => c.is_running);
      await invoke("big_crunch");
      this.snapshot = await this.getState();
      const earlyGame =
        this.snapshot.best_infinity_time_ms > 60000 &&
        !this.snapshot.broke_infinity;
      if (earlyGame || wasInChallenge) {
        useUiStore().setSubtab("dimensions", "antimatter");
      }
    },
    // Eternity (second prestige): resets the whole Infinity layer in the engine
    // and awards Eternity Points. On the first eternity, navigate to the Time
    // Dimensions tab (original: `Tab.dimensions.time.show()` when eternities
    // was 0 and few were gained).
    async eternity() {
      const firstEternity = !this.snapshot?.eternity_unlocked;
      await invoke("eternity");
      this.snapshot = await this.getState();
      if (firstEternity) {
        useUiStore().setSubtab("eternity", "timedims");
      }
    },
    // --- Time Dilation ---
    async toggleDilation() {
      await invoke("toggle_dilation");
      this.snapshot = await this.getState();
    },
    // Dilation request (original `startDilatedEternityRequest`): pops the
    // confirmation modal when the dilation confirmation is on.
    requestDilation() {
      if (!this.snapshot?.dilation?.unlocked) return;
      const ui = useUiStore();
      if (this.snapshot?.options?.confirmations?.dilation) {
        ui.showModal("dilationConfirm");
      } else {
        this.toggleDilation();
      }
    },
    buyDilationStudy(id) {
      return invoke("buy_dilation_study", { id });
    },
    // --- Reality ---
    // Perform a Reality with glyph choice `choice` (undefined = the default
    // pick); `sacrifice` sends the picked glyph straight to sacrifice.
    async doReality(choice, sacrifice = false) {
      await invoke("do_reality", { choice: choice ?? null, sacrifice });
      this.snapshot = await this.getState();
    },
    async resetReality() {
      await invoke("reset_reality");
      this.snapshot = await this.getState();
    },
    // Equip inventory glyph `id` (first free slot unless one is given).
    async equipGlyph(id, slot = null) {
      await invoke("equip_glyph", { id, slot });
      this.snapshot = await this.getState();
    },
    async sacrificeGlyph(id) {
      await invoke("sacrifice_glyph", { id });
      this.snapshot = await this.getState();
    },
    moveGlyph(id, slot) {
      return invoke("move_glyph", { id, slot });
    },
    async setGlyphRespec(respec) {
      await invoke("set_glyph_respec", { respec });
      this.snapshot = await this.getState();
    },
    async buyPerk(id) {
      await invoke("buy_perk", { id });
      this.snapshot = await this.getState();
    },
    buyRealityRebuyable(id) {
      return invoke("buy_reality_rebuyable", { id });
    },
    async buyRealityUpgrade(id) {
      await invoke("buy_reality_upgrade", { id });
      this.snapshot = await this.getState();
    },
    async unlockBlackHole() {
      await invoke("unlock_black_hole");
      this.snapshot = await this.getState();
    },
    buyBlackHoleUpgrade(hole, kind) {
      return invoke("buy_black_hole_upgrade", { hole, kind });
    },
    async toggleBlackHolePause() {
      await invoke("toggle_black_hole_pause");
      this.snapshot = await this.getState();
    },
    buyDilationUpgrade(id) {
      return invoke("buy_dilation_upgrade", { id });
    },
    // --- Eternity Upgrades ---
    buyEternityUpgrade(id) {
      return invoke("buy_eternity_upgrade", { id });
    },
    buyEpMult() {
      return invoke("buy_ep_mult");
    },
    buyMaxEpMult() {
      return invoke("buy_max_ep_mult");
    },
    // --- Eternity Challenges ---
    buyEcStudy(id) {
      return invoke("buy_ec_study", { id });
    },
    startEternityChallenge(id) {
      return invoke("start_eternity_challenge", { id });
    },
    exitEternityChallenge() {
      return invoke("exit_eternity_challenge");
    },
    // --- Time Studies ---
    buyTimeStudy(id) {
      return invoke("buy_time_study", { id });
    },
    buyTimeTheorem(currency) {
      return invoke("buy_time_theorem", { currency });
    },
    buyMaxTimeTheorems() {
      return invoke("buy_max_time_theorems");
    },
    setRespec(respec) {
      return invoke("set_respec", { respec });
    },
    // --- Time Dimensions ---
    buyTimeDimension(tier) {
      return invoke("buy_time_dimension", { tier });
    },
    buyMaxTimeDimension(tier) {
      return invoke("buy_max_time_dimension", { tier });
    },
    maxAllTimeDimensions() {
      return invoke("max_all_time_dimensions");
    },
    // Eternity request (original `eternityResetRequest` → askEternityConfirmation):
    // pops the confirmation modal when the eternity confirmation is on.
    requestEternity() {
      if (!this.snapshot?.can_eternity) return;
      const ui = useUiStore();
      if (this.snapshot?.options?.confirmations?.eternity) {
        ui.showModal("eternityConfirm");
      } else {
        this.eternity();
      }
    },
    // Break Infinity: lift the 1e308 cap so antimatter can grow further and the
    // IP formula scales. Offered once the Big Crunch autobuyer's interval is maxed.
    breakInfinity() {
      return invoke("break_infinity");
    },
    // Buy a one-time Break Infinity Upgrade by save id, or a rebuyable by index.
    buyBreakInfinityUpgrade(id) {
      return invoke("buy_break_infinity_upgrade", { id });
    },
    buyBreakInfinityRebuyable(id) {
      return invoke("buy_break_infinity_rebuyable", { id });
    },
    // Buy one purchase (10 IDs) of an Infinity Dimension tier (or unlock it), the
    // whole tier at once, or all tiers.
    buyInfinityDimension(tier) {
      return invoke("buy_infinity_dimension", { tier });
    },
    buyMaxInfinityDimension(tier) {
      return invoke("buy_max_infinity_dimension", { tier });
    },
    buyMaxAllInfinityDimensions() {
      return invoke("buy_max_all_infinity_dimensions");
    },
    // Acknowledge a tab-notification badge (`tabKey + subtabKey`); called when
    // the player opens that tab. The next snapshot drops the key.
    tabNotificationSeen(key) {
      return invoke("tab_notification_seen", { key });
    },
    // Replicanti: unlock (1e140 IP), the 3 IP upgrades, and a Replicanti Galaxy.
    unlockReplicanti() {
      return invoke("unlock_replicanti");
    },
    buyReplicantiChance() {
      return invoke("buy_replicanti_chance");
    },
    buyReplicantiInterval() {
      return invoke("buy_replicanti_interval");
    },
    buyReplicantiGalaxyCap() {
      return invoke("buy_replicanti_galaxy_cap");
    },
    buyReplicantiGalaxy() {
      return invoke("buy_replicanti_galaxy");
    },
    // Buy an Infinity Upgrade by its original save id (e.g. "timeMult").
    buyInfinityUpgrade(id) {
      return invoke("buy_infinity_upgrade", { id });
    },
    // Start Normal Challenge `id` (a forced Big Crunch, then enter). Navigates to
    // the Antimatter Dimensions tab like the original's `start()`.
    startChallenge(id) {
      useUiStore().setSubtab("dimensions", "antimatter");
      return invoke("start_challenge", { id });
    },
    // Exit the current challenge (Normal or Infinity).
    exitChallenge() {
      return invoke("exit_challenge");
    },
    // Toggle "Automatically retry challenges" (original `retryChallenge`): when
    // on, crunching inside an antimatter challenge re-enters it.
    setRetryChallenge(enabled) {
      return invoke("set_retry_challenge", { enabled });
    },
    // Start Infinity Challenge `id` (a forced Big Crunch that also breaks Infinity,
    // then enter). Navigates to the Antimatter Dimensions tab.
    startInfinityChallenge(id) {
      useUiStore().setSubtab("dimensions", "antimatter");
      return invoke("start_infinity_challenge", { id });
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
    // Reduce an autobuyer's interval one step, spending Infinity Points. `target`
    // is the string handle ("ad0".."ad7", "tickspeed", "dimBoost", "galaxy",
    // "bigCrunch"). Only works once the autobuyer's challenge is completed.
    upgradeAutobuyerInterval(target) {
      return invoke("upgrade_autobuyer_interval", { target });
    },
    // Toggle a prestige autobuyer (Dim Boost / Galaxy / Big Crunch / Eternity /
    // Reality) on/off.
    toggleAutobuyer(target) {
      return invoke("toggle_autobuyer", { target });
    },
    // Big Crunch / Eternity autobuyer goal mode ("amount"/"time"/"xHighest").
    setPrestigeAutobuyerMode(target, mode) {
      return invoke("set_prestige_autobuyer_mode", { target, mode });
    },
    // Value input for the current goal mode; resolves false for bad input.
    setPrestigeAutobuyerValue(target, value) {
      return invoke("set_prestige_autobuyer_value", { target, value });
    },
    // The "Dynamic amount" checkbox (amount scales with prestige multipliers).
    toggleAutobuyerDynamicAmount(target) {
      return invoke("toggle_autobuyer_dynamic_amount", { target });
    },
    // Reality autobuyer mode ("rm"/"glyph"/"either"/"both"/"time").
    setRealityAutobuyerMode(mode) {
      return invoke("set_reality_autobuyer_mode", { mode });
    },
    // Reality autobuyer targets ("rm" decimal / "glyph" int / "time" float).
    setRealityAutobuyerValue(property, value) {
      return invoke("set_reality_autobuyer_value", { property, value });
    },
    // --- Automator ---
    // Play button: pause when running, resume when paused, else start the
    // editor's script.
    automatorPlay(scriptId) {
      return invoke("automator_play", { scriptId });
    },
    automatorStop() {
      return invoke("automator_stop");
    },
    // Rewind: restart the running script from the top.
    automatorRewind() {
      return invoke("automator_rewind");
    },
    // Single-step one command (starts the editor's script when off).
    automatorStep(scriptId) {
      return invoke("automator_step", { scriptId });
    },
    // "repeat" / "forceRestart" / "followExecution".
    automatorToggleSetting(setting) {
      return invoke("automator_toggle_setting", { setting });
    },
    automatorSelectScript(id) {
      return invoke("automator_select_script", { id });
    },
    // Resolves to the new script's id (null at the 20-script cap).
    automatorNewScript() {
      return invoke("automator_new_script");
    },
    automatorRenameScript(id, name) {
      return invoke("automator_rename_script", { id, name });
    },
    automatorDeleteScript(id) {
      return invoke("automator_delete_script", { id });
    },
    // Resolves to the stored script content.
    getAutomatorScript(id) {
      return invoke("get_automator_script", { id });
    },
    // Persist editor content; resolves to { saved, errors } (compile errors
    // of the typed content either way).
    saveAutomatorScript(id, content) {
      return invoke("save_automator_script", { id, content });
    },
    getAutomatorErrors(id) {
      return invoke("get_automator_errors", { id });
    },
    automatorSetConstant(name, value) {
      return invoke("automator_set_constant", { name, value });
    },
    automatorRenameConstant(oldName, newName) {
      return invoke("automator_rename_constant", { oldName, newName });
    },
    automatorDeleteConstant(name) {
      return invoke("automator_delete_constant", { name });
    },
    automatorSetInfoPane(pane) {
      return invoke("automator_set_info_pane", { pane });
    },
    // Resolves to { now_play_time_ms, events }.
    getAutomatorEvents() {
      return invoke("get_automator_events");
    },
    automatorClearLog() {
      return invoke("automator_clear_log");
    },
    // "newestFirst"/"clearOnReality"/"clearOnRestart" (0/1) and
    // "timestampType" (0-4).
    setAutomatorEventOption(option, value) {
      return invoke("set_automator_event_option", { option, value });
    },
    // Resolves to { blocks, lost_lines } for the block editor.
    automatorBlockify(id) {
      return invoke("automator_blockify", { id });
    },
    automatorBlockifyText(content) {
      return invoke("automator_blockify_text", { content });
    },
    automatorSetEditorType(block) {
      return invoke("automator_set_editor_type", { block });
    },
    // Resolves to { script, warnings } or null on invalid params.
    automatorTemplate(name, params) {
      return invoke("automator_template", { name, params });
    },
    // Resolve to the encoded text blob for the clipboard.
    automatorExportScript(id) {
      return invoke("automator_export_script", { id });
    },
    automatorExportFull(id) {
      return invoke("automator_export_full", { id });
    },
    // Resolves to { name, content, presets, constants, is_full_data } or null.
    automatorImportPreview(raw) {
      return invoke("automator_import_preview", { raw });
    },
    // Resolves to the new script's id, or null if the data is invalid.
    automatorImport(raw, ignorePresets, ignoreConstants) {
      return invoke("automator_import", { raw, ignorePresets, ignoreConstants });
    },
    // Resolves to { presets, constants } used by a stored script.
    automatorScriptDataInfo(id) {
      return invoke("automator_script_data_info", { id });
    },
    // --- Time Study presets ---
    studyPresetSave(slot) {
      return invoke("study_preset_save", { slot });
    },
    // The current tree as an import string (template modal "Current Tree").
    studyTreeExport() {
      return invoke("study_tree_export");
    },
    studyTreeIsValid(text) {
      return invoke("study_tree_is_valid", { text });
    },
    // Load a preset into the current tree; `respec` = "Respec and Load".
    studyPresetLoad(slot, respec = false) {
      return invoke("study_preset_load", { slot, respec });
    },
    // Rename a preset (≤ 4 ASCII chars, unique); resolves false when rejected.
    studyPresetRename(slot, name) {
      return invoke("study_preset_rename", { slot, name });
    },
    // Overwrite a preset's study string; resolves false for a malformed string.
    studyPresetEdit(slot, studies) {
      return invoke("study_preset_edit", { slot, studies });
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
    // Flip an animation toggle (original `player.options.animations.*`);
    // `kind` is the camelCase name ("bigCrunch").
    setAnimation(kind, enabled) {
      return invoke("set_animation", { kind, enabled });
    },
    // Flip an info-display hint toggle (original
    // `player.options.showHintText.*`), e.g. "showPercentage".
    setHintText(kind, enabled) {
      return invoke("set_hint_text", { kind, enabled });
    },
    // Flip an away-progress display toggle (original
    // `player.options.awayProgress.*`), e.g. "infinityPoints".
    setAwayProgress(kind, enabled) {
      return invoke("set_away_progress", { kind, enabled });
    },
    // Relative prestige-gain text coloring (original `headerTextColored`).
    setHeaderTextColored(enabled) {
      return invoke("set_header_text_colored", { enabled });
    },
    // Sidebar resource picker (original `sidebarResourceID`; 0 = latest).
    setSidebarResource(id) {
      return invoke("set_sidebar_resource", { id });
    },
    // Hidden-tab bits (original `hiddenTabBits`/`hiddenSubtabBits`, original
    // tab/subtab ids). The "cannot hide current/non-hidable" guards live in
    // the modal; the engine just flips bits.
    toggleTabVisibility(tabId) {
      return invoke("toggle_tab_visibility", { tabId });
    },
    unhideTab(tabId) {
      return invoke("unhide_tab", { tabId });
    },
    toggleSubtabVisibility(tabId, subtabId) {
      return invoke("toggle_subtab_visibility", { tabId, subtabId });
    },
    showAllTabs() {
      return invoke("show_all_tabs");
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
