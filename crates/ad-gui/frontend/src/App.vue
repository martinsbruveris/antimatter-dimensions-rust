<script setup>
import { onMounted, onUnmounted } from "vue";

import { useGameStore } from "./stores/game";
import { useUiStore } from "./stores/ui";
import Sidebar from "./components/Sidebar.vue";
import GameHeader from "./components/GameHeader.vue";

const game = useGameStore();
const ui = useUiStore();

let raf = null;
let last = performance.now();

function loop() {
  const now = performance.now();
  game.tick(now - last);
  last = now;
  raf = requestAnimationFrame(loop);
}

function onKeydown(e) {
  if (e.key === "m" || e.key === "M") game.maxAll();
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
</template>
