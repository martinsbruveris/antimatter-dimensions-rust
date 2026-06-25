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
