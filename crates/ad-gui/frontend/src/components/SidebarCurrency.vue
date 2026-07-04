<script setup>
// The sidebar's currency box. Mirrors the original ModernSidebarCurrency.vue:
// shows the resource picked by the `sidebarResourceID` option (0 = the latest
// unlocked resource, drawn with the thicker "default" border), and cycles
// through the unlocked resources on click (shift-click cycles backwards).
// The resource DB lives in config/sidebarResources.js; the option is also
// settable from the Visual tab's Sidebar dropdown.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { SIDEBAR_RESOURCES } from "../config/sidebarResources";

const game = useGameStore();

const available = computed(() =>
  SIDEBAR_RESOURCES.filter((r) => r.isAvailable(game.snapshot))
);

const selectedId = computed(
  () => game.snapshot?.options?.sidebar_resource_id ?? 0
);

// Id 0 means "latest": the highest-id unlocked resource (the DB is id-sorted).
// An id we can't render — locked again after an import, or past our frontier —
// falls back to the latest too.
const resource = computed(() => {
  const list = available.value;
  const chosen =
    selectedId.value === 0
      ? undefined
      : list.find((r) => r.id === selectedId.value);
  return chosen ?? list[list.length - 1] ?? SIDEBAR_RESOURCES[0];
});

const displayValue = computed(() =>
  game.snapshot ? resource.value.formatValue(resource.value.value(game.snapshot)) : "0"
);

// Shrink long values to fit, mirroring ModernSidebarCurrency.styleObject.
const scaleStyle = computed(() => {
  const len = displayValue.value.length;
  return { transform: `scale(${len < 10 ? 1 : 10 / len})` };
});

// Cycle through "latest" (0) and the unlocked resources, mirroring the
// original's cycleResource (which skips unavailable entries).
function cycleResource(dir) {
  const ids = [0, ...available.value.map((r) => r.id)];
  const idx = Math.max(0, ids.indexOf(selectedId.value));
  game.setSidebarResource(ids[(idx + ids.length + dir) % ids.length]);
}
</script>

<template>
  <div
    class="c-sidebar-resource"
    :class="{ 'c-sidebar-resource-default': selectedId === 0 }"
    @click.exact="cycleResource(1)"
    @click.shift.exact="cycleResource(-1)"
  >
    <h2
      :class="resource.formatClass"
      :style="scaleStyle"
    >
      {{ displayValue }}
    </h2>
    <div class="c-sidebar-resource__information">
      <span class="c-sidebar-resource__name">{{ resource.optionName }}</span>
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

.o-sidebar-currency--infinity {
  color: var(--color-infinity);
}

.o-sidebar-currency--replicanti {
  /* Taken from glyph-types.js (the original's scoped rule). */
  color: #03a9f4;
}
</style>
