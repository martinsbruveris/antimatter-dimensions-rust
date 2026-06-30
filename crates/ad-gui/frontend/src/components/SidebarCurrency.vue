<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";

const game = useGameStore();

// Sidebar antimatter uses `format(x, 2, 1)` in the original
// (secret-formula/sidebar-resources.js).
const displayValue = computed(() =>
  game.snapshot ? formatDecimal(game.snapshot.antimatter, 2, 1) : "0"
);

// Shrink long values to fit, mirroring ModernSidebarCurrency.styleObject.
const scaleStyle = computed(() => {
  const len = displayValue.value.length;
  return { transform: `scale(${len < 10 ? 1 : 10 / len})` };
});
</script>

<template>
  <div class="c-sidebar-resource c-sidebar-resource-default">
    <h2
      class="o-sidebar-currency--antimatter"
      :style="scaleStyle"
    >
      {{ displayValue }}
    </h2>
    <div class="c-sidebar-resource__information">
      <span class="c-sidebar-resource__name">Antimatter</span>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from ModernSidebarCurrency.vue scoped style (not in the
   global vendored CSS). t-normal antimatter colour = accent (red). */
.c-sidebar-resource {
  display: flex;
  flex-direction: column;
  width: var(--sidebar-width);
  height: 7rem;
  justify-content: center;
  align-items: center;
  background-color: var(--color-base);
  border-right: 0.1rem solid var(--color-accent);
  border-bottom: 0.1rem solid var(--color-accent);
  padding: 1rem;
  user-select: none;
}

.c-sidebar-resource-default {
  border-width: 0.3rem;
}

.c-sidebar-resource h2 {
  z-index: 1;
  font-size: 1.9rem;
  margin: 0;
}

.c-sidebar-resource__information {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-start;
  font-size: 1.5rem;
  color: var(--color-text);
}

.c-sidebar-resource__name {
  font-size: 1.2rem;
}

.o-sidebar-currency--antimatter {
  color: var(--color-accent);
}
</style>
