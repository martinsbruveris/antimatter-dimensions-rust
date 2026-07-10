<script setup>
// Past Prestige Runs — the last-10 Infinity/Eternity/Reality tables (a port
// of the original PastPrestigeRunsTab.vue). The "Showing X" button cycles the
// resource pair through the engine-owned `statTabResources` option.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import PastPrestigeRunsContainer from "./statistics/PastPrestigeRunsContainer.vue";

const game = useGameStore();
const s = computed(() => game.snapshot);
const stats = computed(() => s.value?.statistics);

const resourceType = computed(() => s.value?.options?.stat_tab_resources ?? 0);
// Real-time columns appear once Reality is unlocked (the Statistics page
// gates its real-time lines the same way).
const hasRealTime = computed(() => Boolean(s.value?.reality?.unlocked));

// Newest-first layer order like the original (Reality → Eternity → Infinity),
// each gated on its unlock.
const layers = computed(() => [
  {
    key: "reality",
    name: "Reality",
    plural: "Realities",
    currency: "RM",
    condition: Boolean(s.value?.reality?.unlocked),
    runs: stats.value?.recent_realities ?? [],
    shown: Boolean(stats.value?.shown_runs?.reality),
  },
  {
    key: "eternity",
    name: "Eternity",
    plural: "Eternities",
    currency: "EP",
    condition: Boolean(s.value?.eternity_unlocked),
    runs: stats.value?.recent_eternities ?? [],
    shown: Boolean(stats.value?.shown_runs?.eternity),
  },
  {
    key: "infinity",
    name: "Infinity",
    plural: "Infinities",
    currency: "IP",
    condition: Boolean(s.value?.infinity_unlocked),
    runs: stats.value?.recent_infinities ?? [],
    shown: Boolean(stats.value?.shown_runs?.infinity),
  },
]);

const resourceText = computed(() => {
  switch (resourceType.value) {
    case 0: return "total resource gain";
    case 1: return "resource gain rate";
    case 2: return "prestige currency";
    default: return "prestige count";
  }
});

function cycleButton() {
  game.setStatTabResources((resourceType.value + 1) % 4);
}
</script>

<template>
  <div
    v-if="stats"
    class="c-stats-tab"
  >
    <div class="c-subtab-option-container">
      <button
        class="o-primary-btn o-primary-btn--subtab-option"
        @click="cycleButton()"
      >
        Showing {{ resourceText }}
      </button>
    </div>
    <template
      v-for="layer in layers"
      :key="layer.name"
    >
      <PastPrestigeRunsContainer
        v-if="layer.condition"
        :layer="layer"
        :runs="layer.runs"
        :shown="layer.shown"
        :resource-type="resourceType"
        :has-real-time="hasRealTime"
      />
    </template>
  </div>
</template>
