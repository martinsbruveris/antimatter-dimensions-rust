<script setup>
// Challenge records — the NC/IC best-times lists (a port of the original
// ChallengeRecordsTab.vue). NC times start at challenge 2 (NC1 has no
// restriction and keeps no time); IC times start at 1. The IC list shows once
// an Infinity Challenge has been completed or Eternity is unlocked (original
// `PlayerProgress.infinityChallengeCompleted() || eternityUnlocked()`).
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import ChallengeRecordsList from "./statistics/ChallengeRecordsList.vue";

const game = useGameStore();
const s = computed(() => game.snapshot);
const stats = computed(() => s.value?.statistics);

const infinityChallengesUnlocked = computed(
  () =>
    Boolean(s.value?.eternity_unlocked) ||
    (s.value?.infinity_challenges ?? []).some((ic) => ic.is_completed),
);
</script>

<template>
  <div
    v-if="stats"
    class="l-challenge-records-tab c-stats-tab"
  >
    <ChallengeRecordsList
      :start="2"
      :times="stats.nc_best_times_ms"
      name="Normal Challenge"
    />
    <ChallengeRecordsList
      v-if="infinityChallengesUnlocked"
      :start="1"
      :times="stats.ic_best_times_ms"
      name="Infinity Challenge"
      class="l-challenge-records-tab__infinity_challenges"
    />
  </div>
</template>
