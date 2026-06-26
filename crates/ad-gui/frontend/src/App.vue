<script setup>
import { onMounted, onUnmounted } from "vue";

import { useGameStore } from "./stores/game";
import { useUiStore } from "./stores/ui";
import { handleShortcut } from "./util/shortcuts";
import Sidebar from "./components/Sidebar.vue";
import GameHeader from "./components/GameHeader.vue";
import InfoButtons from "./components/InfoButtons.vue";
import HotkeysModal from "./components/HotkeysModal.vue";

const game = useGameStore();
const ui = useUiStore();

let raf = null;
let last = performance.now();

function loop() {
  const now = performance.now();
  game.tick((now - last) * ui.speedMultiplier);
  last = now;
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
          v-for="s in [1, 10, 60]"
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
    <div class="tab-container">
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
</template>

<style scoped>
.top-right-controls {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  display: flex;
  align-items: center;
  gap: 0.8rem;
  z-index: 10;
}

.speed-controls {
  display: flex;
  gap: 0.3rem;
}

.speed-btn {
  padding: 0.2rem 0.6rem;
  font-size: 0.8rem;
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
