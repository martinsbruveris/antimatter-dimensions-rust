<script setup>
import { onMounted, onUnmounted } from "vue";

import { useGameStore } from "./stores/game";
import { useUiStore } from "./stores/ui";
import { handleShortcut } from "./util/shortcuts";
import { formatTime } from "./util/format";
import Sidebar from "./components/Sidebar.vue";
import GameHeader from "./components/GameHeader.vue";
import InfoButtons from "./components/InfoButtons.vue";
import HotkeysModal from "./components/HotkeysModal.vue";
import NotationModal from "./components/NotationModal.vue";
import ImportSaveModal from "./components/ImportSaveModal.vue";
import HardResetModal from "./components/HardResetModal.vue";
import LoadGameModal from "./components/LoadGameModal.vue";
import BackupWindowModal from "./components/BackupWindowModal.vue";
import BigCrunchScreen from "./components/BigCrunchScreen.vue";
import OfflineSummaryModal from "./components/OfflineSummaryModal.vue";
import NotificationContainer from "./components/NotificationContainer.vue";
import SaveTimer from "./components/SaveTimer.vue";
import DimensionBoostConfirmModal from "./components/DimensionBoostConfirmModal.vue";
import AntimatterGalaxyConfirmModal from "./components/AntimatterGalaxyConfirmModal.vue";
import SacrificeConfirmModal from "./components/SacrificeConfirmModal.vue";
import BigCrunchConfirmModal from "./components/BigCrunchConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

let raf = null;
let last = performance.now();

// Default cadence until the first snapshot arrives (original `updateRate: 33`).
const DEFAULT_UPDATE_RATE = 33;

// Default autosave cadence until the first snapshot arrives (30 s).
const DEFAULT_AUTOSAVE_INTERVAL = 30000;

// Online automatic-backup slots and their wall-clock intervals (ms), mirroring
// the original's ONLINE `AutoBackupSlots` (§11.3). Checked while the app runs.
const ONLINE_BACKUP_SLOTS = [
  { id: 1, interval: 60000 },
  { id: 2, interval: 300000 },
  { id: 3, interval: 1200000 },
  { id: 4, interval: 3600000 },
];

// Wall-clock timestamps of the last autosave / last online backup per slot.
// Seeded to "now" so nothing fires the instant the app opens.
let lastAutosave = Date.now();
const lastBackup = { 1: Date.now(), 2: Date.now(), 3: Date.now(), 4: Date.now() };

// Autosave and online backups are wall-clock driven and independent of the
// engine tick: fire each when its interval has elapsed. Only runs once the first
// snapshot exists (so we never save before the on-disk state has loaded).
function maybePersist(nowWall) {
  if (!game.snapshot) return;

  const interval =
    game.snapshot.options?.autosave_interval ?? DEFAULT_AUTOSAVE_INTERVAL;
  if (nowWall - lastAutosave >= interval) {
    lastAutosave = nowWall;
    game.saveGame();
  }

  for (const slot of ONLINE_BACKUP_SLOTS) {
    if (nowWall - lastBackup[slot.id] >= slot.interval) {
      lastBackup[slot.id] = nowWall;
      game.triggerBackup(slot.id);
    }
  }
}

function loop() {
  const now = performance.now();

  // Keep the store's wall clock current every frame so the save timer advances
  // in real time even while the engine is paused or in offline mode.
  game.nowMs = Date.now();

  // Autosave + online backups run on the wall clock, independent of the engine
  // tick / dev pause / offline mode (the original checks them while the app is
  // open regardless of game state).
  maybePersist(Date.now());

  // Absolute pause (dev) freezes everything: no live ticks and no offline
  // accumulation. Consume the elapsed wall time so unpausing doesn't jump.
  if (ui.absolutePause) {
    last = now;
    raf = requestAnimationFrame(loop);
    return;
  }

  // Offline mode: don't tick the engine. Accumulate speed-scaled game-time each
  // frame (the integration), to be replayed as one batch when switched off.
  if (ui.offlineMode) {
    ui.accumulateOffline((now - last) * ui.speedMultiplier);
    last = now;
    raf = requestAnimationFrame(loop);
    return;
  }

  // Mirror the original game loop, which runs every `updateRate` ms rather
  // than every animation frame: only tick once that much wall-clock time has
  // elapsed, then process the whole elapsed interval. A larger update rate
  // therefore means coarser, less frequent updates.
  const updateRate = game.snapshot?.options?.update_rate ?? DEFAULT_UPDATE_RATE;
  if (now - last >= updateRate) {
    // The speed multiplier runs the engine as N discrete ticks of the real
    // elapsed dt (looped in Rust), not a single dt * N step.
    game.tick(now - last, ui.speedMultiplier);
    last = now;
  }
  raf = requestAnimationFrame(loop);
}

function onKeydown(e) {
  handleShortcut(e, game, ui);
}

onMounted(() => {
  last = performance.now();
  raf = requestAnimationFrame(loop);
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  if (raf) cancelAnimationFrame(raf);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <Sidebar />
  <div class="game-container">
    <div class="top-right-controls">
      <div class="control-stack">
        <div class="speed-controls">
          <button
            v-for="s in [1, 10, 100, 1000]"
            :key="s"
            :class="['speed-btn', { active: ui.speedMultiplier === s }]"
            @click="ui.setSpeed(s)"
          >
            {{ s }}x
          </button>
        </div>
        <!-- Offline mode + absolute pause (dev). Sit under the speed row,
             right-aligned with it; the live readout (left of the buttons) shows
             accumulated offline game-time. -->
        <div class="offline-controls">
          <span
            v-if="ui.offlineMode"
            class="offline-readout"
          >
            {{ formatTime(ui.accumulatedGameMs) }}
          </span>
          <button
            :class="['speed-btn', { active: ui.offlineMode }]"
            @click="ui.toggleOfflineMode()"
          >
            Offline
          </button>
          <button
            :class="['speed-btn', { active: ui.absolutePause }]"
            @click="ui.toggleAbsolutePause()"
          >
            Pause
          </button>
        </div>
      </div>
      <!-- Help (?) and info (i) buttons, matching the JS version's
           top-right placement. -->
      <InfoButtons />
    </div>
    <!-- Once antimatter reaches the Big Crunch threshold the whole game view
         is replaced by the Big Crunch screen, matching ModernUi.vue's
         `tab-container` being hidden while the crunch button shows. -->
    <BigCrunchScreen v-if="game.snapshot && game.snapshot.can_big_crunch" />
    <div
      v-else
      class="tab-container"
    >
      <GameHeader />
      <!-- Matches ModernUi.vue: an (empty pre-infinity) information-header
           whose green border-bottom is the separator under the header. -->
      <div class="information-header" />
      <!-- The active page; swaps based on the selected tab/subtab. -->
      <component
        :is="ui.currentComponent"
        v-if="ui.currentComponent && game.snapshot"
      />
      <div
        v-else-if="!ui.currentComponent"
        class="c-coming-soon"
      >
        This page isn't implemented yet.
      </div>
    </div>
  </div>

  <HotkeysModal
    v-if="ui.openModal === 'hotkeys'"
    @close="ui.closeModal()"
  />

  <NotationModal
    v-if="ui.openModal === 'notation'"
    @close="ui.closeModal()"
  />

  <!-- Saving-tab popups (wired to the engine's save/load + backup commands). -->
  <ImportSaveModal
    v-if="ui.openModal === 'importSave'"
    @close="ui.closeModal()"
  />

  <HardResetModal
    v-if="ui.openModal === 'hardReset'"
    @close="ui.closeModal()"
  />

  <LoadGameModal
    v-if="ui.openModal === 'loadGame'"
    @close="ui.closeModal()"
  />

  <BackupWindowModal
    v-if="ui.openModal === 'backup'"
    @close="ui.closeModal()"
  />

  <!-- Catch-up summary after an Offline-mode replay of >= 10 s. -->
  <OfflineSummaryModal
    v-if="ui.openModal === 'offlineSummary'"
    @close="ui.closeModal()"
  />

  <!-- Prestige confirmation popups (shown when the matching confirmation
       option is on; each Confirm button performs the engine action). -->
  <DimensionBoostConfirmModal v-if="ui.openModal === 'dimboostConfirm'" />
  <AntimatterGalaxyConfirmModal v-if="ui.openModal === 'galaxyConfirm'" />
  <SacrificeConfirmModal v-if="ui.openModal === 'sacrificeConfirm'" />
  <BigCrunchConfirmModal v-if="ui.openModal === 'bigCrunchConfirm'" />

  <!-- Transient top-right toast popups (e.g. autobuyer pause/resume). -->
  <NotificationContainer />

  <!-- Bottom-left "time since last save" timer (fixed overlay; click to save),
       gated by the Display-time-since-save option. -->
  <SaveTimer />
</template>

<style scoped>
.top-right-controls {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  display: flex;
  /* Top-align so the speed row lines up with the "?" button (the top of the
     stacked ?/i column), not the middle of the column. */
  align-items: flex-start;
  gap: 0.8rem;
  z-index: 10;
}

.control-stack {
  display: flex;
  flex-direction: column;
  /* Right-align both rows so the offline row's right edge (the Pause button)
     lines up with the speed row's right edge (the 1000x button). The readout
     grows leftward without shifting the buttons. */
  align-items: flex-end;
  gap: 0.3rem;
}

.speed-controls {
  display: flex;
  gap: 0.3rem;
}

.offline-controls {
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.offline-readout {
  font-size: 0.9rem;
  color: var(--color-text, #cccccc);
  white-space: nowrap;
}

.speed-btn {
  display: inline-flex;
  align-items: center;
  /* Match the "?" button height (2.2rem). */
  height: 2.2rem;
  padding: 0 0.7rem;
  font-size: 1rem;
  cursor: pointer;
  border: 1px solid var(--color-accent, #5f9948);
  border-radius: 3px;
  background: transparent;
  color: var(--color-text, #cccccc);
}

.speed-btn.active {
  background: var(--color-accent, #5f9948);
  color: #000;
}
</style>
