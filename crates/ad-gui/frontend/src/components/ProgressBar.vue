<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";

const game = useGameStore();
const pct = computed(() =>
  (game.snapshot.infinity_progress * 100).toFixed(2)
);
</script>

<template>
  <div class="c-progress-bar">
    <div
      class="c-progress-bar__fill"
      :style="{ width: pct + '%' }"
    >
      <span class="c-progress-bar__percents">{{ pct }}%</span>
    </div>
  </div>
</template>

<style scoped>
/* The vendored .c-progress-bar__fill has `transition-duration: 0.1s`, which
   animates the width. Because we push a fresh `pct` every animation frame,
   the eased width perpetually trails ~0.1s behind the instantly-updated
   percentage label. We update continuously, so disable the smoothing and
   let the fill track the label exactly. */
.c-progress-bar__fill {
  transition-duration: 0s;
}
</style>
