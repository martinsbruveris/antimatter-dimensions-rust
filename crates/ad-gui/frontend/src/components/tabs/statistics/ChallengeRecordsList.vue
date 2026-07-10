<script setup>
// One best-times list (Normal or Infinity Challenges) — a port of the
// original ChallengeRecordsList.vue. `times` are ms with `Number.MAX_VALUE`
// as the "never completed" sentinel (the engine's `f64::MAX`).
import { computed } from "vue";

import { timeDisplayShort } from "../../../util/format";

const props = defineProps({
  name: { type: String, required: true },
  start: { type: Number, required: true },
  times: { type: Array, required: true },
});

// Any never-completed sentinel pushes the sum to/above MAX_VALUE.
const timeSum = computed(() => props.times.reduce((a, b) => a + b, 0));
const completedAllChallenges = computed(() => timeSum.value < Number.MAX_VALUE);

function completionString(time) {
  return time < Number.MAX_VALUE
    ? `record time: ${timeDisplayShort(time)}`
    : "has not yet been completed";
}
</script>

<template>
  <div>
    <br>
    <div
      v-for="(time, i) in times"
      :key="i"
    >
      <span>{{ name }} {{ start + i }} {{ completionString(time) }}</span>
    </div>
    <br>
    <div v-if="completedAllChallenges">
      Sum of {{ name }} record times: {{ timeDisplayShort(timeSum) }}
    </div>
    <div v-else>
      You have not completed all {{ name }}s yet.
    </div>
  </div>
</template>
