<script setup>
import { onMounted, onUnmounted } from "vue";

import { useGameStore } from "./stores/game";
import { useUiStore } from "./stores/ui";
import { handleShortcut } from "./util/shortcuts";
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
import NotificationContainer from "./components/NotificationContainer.vue";

const game = useGameStore();
const ui = useUiStore();

let raf = null;
let last = performance.now();

// Default cadence until the first snapshot arrives (original `updateRate: 33`).
const DEFAULT_UPDATE_RATE = 33;

function loop() {
  const now = performance.now();
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

  <!-- Saving-tab popups (visual only; save/load not wired up yet). -->
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

  <!-- Transient top-right toast popups (e.g. autobuyer pause/resume). -->
  <NotificationContainer />
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

.speed-controls {
  display: flex;
  gap: 0.3rem;
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
