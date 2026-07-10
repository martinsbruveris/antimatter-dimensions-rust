<script setup>
// The post-break header Big Crunch button. Mirrors the original
// ui-modes/prestige-header/BigCrunchButton.vue (minus the Tesseract branch,
// a celestial feature): visible once Infinity is broken; shows the goal when
// below it, the challenge line inside an antimatter challenge, and otherwise
// the crunch IP gain with IP/min rates below the 5e11 rate threshold.
import { computed, ref } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";
import { numLog10 as log10, scaleNum } from "../util/num";

const game = useGameStore();
const s = computed(() => game.snapshot);
const hover = ref(false);

// Show IP/min below this threshold, color the IP number above it.
const RATE_THRESHOLD_LOG10 = Math.log10(5e11);

const isVisible = computed(() => Boolean(s.value?.broke_infinity));
const canCrunch = computed(() => Boolean(s.value?.can_big_crunch));
const inChallenge = computed(
  () =>
    (s.value?.challenges ?? []).some((c) => c.is_running) ||
    (s.value?.infinity_challenges ?? []).some((c) => c.is_running)
);

const showIPRate = computed(
  () => log10(s.value.best_ip_min) <= RATE_THRESHOLD_LOG10
);

const currentIPRate = computed(() => {
  const minutes = Math.max(s.value.this_infinity_real_time_ms / 60000, 0.0005);
  return scaleNum(s.value.gained_infinity_points, 1 / minutes);
});

const ipIsOne = computed(
  () =>
    s.value.gained_infinity_points.m === 1 && s.value.gained_infinity_points.e === 0
);

// Antimatter goal shown while below it (the current infinity goal — an IC's
// own goal while one runs, else 1.8e308). Shipped by the engine.
const infinityGoal = computed(() => s.value.infinity_goal);

const amountStyle = computed(() => {
  const headerColored = s.value.options?.header_text_colored ?? true;
  if (!headerColored || log10(s.value.infinity_points) < RATE_THRESHOLD_LOG10)
    return { "transition-duration": "0s" };
  if (hover.value) return { color: "black", "transition-duration": "0.2s" };

  const textHexCode = getComputedStyle(document.body)
    .getPropertyValue("--color-text")
    .split("#")[1];
  const stepRGB = [
    [255, 0, 0],
    [
      parseInt(textHexCode.substring(0, 2), 16),
      parseInt(textHexCode.substring(2, 4), 16),
      parseInt(textHexCode.substring(4), 16),
    ],
    [0, 255, 0],
  ];
  const ratio =
    log10(s.value.gained_infinity_points) / log10(s.value.infinity_points);
  const interFn = (index) => {
    if (ratio < 0.9) return stepRGB[0][index];
    if (ratio < 1) {
      const r = 10 * (ratio - 0.9);
      return Math.round(stepRGB[0][index] * (1 - r) + stepRGB[1][index] * r);
    }
    if (ratio < 1.1) {
      const r = 10 * (ratio - 1);
      return Math.round(stepRGB[1][index] * (1 - r) + stepRGB[2][index] * r);
    }
    return stepRGB[2][index];
  };
  return {
    color: `rgb(${interFn(0)},${interFn(1)},${interFn(2)})`,
    "transition-duration": "0.2s",
  };
});

function crunch() {
  if (!canCrunch.value) return;
  game.requestBigCrunch();
}
</script>

<template>
  <button
    v-if="isVisible"
    class="o-prestige-button o-infinity-button"
    :class="{ 'o-infinity-button--unavailable': !canCrunch }"
    @click="crunch"
    @mouseover="hover = true"
    @mouseleave="hover = false"
  >
    <!-- Cannot Crunch -->
    <template v-if="!canCrunch">
      Reach {{ formatDecimal(infinityGoal, 2, 2) }}
      <br>
      antimatter
    </template>

    <!-- Can Crunch in challenge -->
    <template v-else-if="inChallenge">
      Big Crunch to
      <br>
      complete the challenge
    </template>

    <!-- Can Crunch -->
    <template v-else>
      <div v-if="!showIPRate" />
      <b>
        Big Crunch for
        <span :style="amountStyle">{{ formatDecimal(s.gained_infinity_points, 2) }}</span>
        <span v-if="showIPRate"> IP</span>
        <span v-else> Infinity {{ ipIsOne ? "Point" : "Points" }}</span>
      </b>
      <template v-if="showIPRate">
        <br>
        Current: {{ formatDecimal(currentIPRate, 2) }} IP/min
        <br>
        Peak: {{ formatDecimal(s.best_ip_min, 2) }} IP/min
        <br>
        at {{ formatDecimal(s.best_ip_min_val, 2) }} IP
      </template>
      <div v-else />
    </template>
  </button>
</template>
