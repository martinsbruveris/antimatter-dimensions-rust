<script setup>
// Eternity Milestones grid. Mirrors the original EternityMilestonesTab.vue /
// EternityMilestoneButton.vue: a 3-column grid of milestone cells in
// threshold order, reached cells green. The engine ships threshold + reached
// state per milestone; reward text lives in data/eternityMilestones.js.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";
import { ETERNITY_MILESTONE_REWARDS } from "../../data/eternityMilestones";

const game = useGameStore();
const s = computed(() => game.snapshot);

const milestones = computed(() => s.value?.eternity_milestones ?? []);
const rows = computed(() => Math.ceil(milestones.value.length / 3));

function milestone(row, column) {
  return milestones.value[(row - 1) * 3 + column - 1];
}

function reward(m) {
  return ETERNITY_MILESTONE_REWARDS[m.id] ?? m.id;
}

function rewardClassObject(m) {
  const text = reward(m);
  return {
    "o-eternity-milestone__reward": true,
    "o-eternity-milestone__reward--locked": !m.is_reached,
    "o-eternity-milestone__reward--reached": m.is_reached,
    "o-eternity-milestone__reward--small-font": text.length > 80,
  };
}

const eternityCount = computed(() => {
  const e = s.value?.eternities ?? { m: 0, e: 0 };
  // Floor the displayed count like the original.
  return formatDecimal({ m: e.m, e: e.e }, 3, 0);
});
const isOne = computed(
  () => s.value?.eternities.m === 1 && s.value?.eternities.e === 0
);
</script>

<template>
  <div
    v-if="s"
    class="l-eternity-milestone-grid"
  >
    <div>You have {{ eternityCount }} {{ isOne ? "Eternity" : "Eternities" }}.</div>
    <div>
      Offline generation milestones are only active under certain conditions,
      mouse-over to see these conditions.
    </div>
    <div
      v-for="row in rows"
      :key="row"
      class="l-eternity-milestone-grid__row"
    >
      <div
        v-for="column in 3"
        :key="row * 3 + column"
        class="l-eternity-milestone l-eternity-milestone-grid__cell"
      >
        <template v-if="milestone(row, column)">
          <span class="o-eternity-milestone__goal">
            {{ milestone(row, column).eternities }}
            {{ milestone(row, column).eternities === 1 ? "Eternity" : "Eternities" }}:
          </span>
          <button :class="rewardClassObject(milestone(row, column))">
            <span>{{ reward(milestone(row, column)) }}</span>
          </button>
        </template>
      </div>
    </div>
  </div>
</template>
