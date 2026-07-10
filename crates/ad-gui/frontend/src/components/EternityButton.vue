<script setup>
// The header Eternity button. Mirrors the original
// ui-modes/prestige-header/EternityButton.vue for the in-frontier display
// types: CANNOT_ETERNITY (goal not reached), FIRST_TIME (pre-first-eternity
// flavour text), and NORMAL (gained EP + EP/min rates below the 1e40 rate
// threshold, red↔green gain coloring above it). The challenge / dilation /
// new-content types arrive with their features.
import { computed, ref } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";
import { numLog10 as log10, scaleNum } from "../util/num";

const game = useGameStore();
const s = computed(() => game.snapshot);
const hover = ref(false);

// Show EP/min below this threshold, color the EP number above it.
const RATE_THRESHOLD_LOG10 = 40;

// isVisible: canEternity, the autoUnlockID milestone (25 eternities), or an
// unlocked 8th Infinity Dimension.
const isVisible = computed(() => {
  if (!s.value) return false;
  return (
    s.value.can_eternity ||
    log10(s.value.eternities) >= Math.log10(25) ||
    s.value.infinity_dimensions.dimensions[7].is_unlocked
  );
});

const canEternity = computed(() => Boolean(s.value?.can_eternity));
const firstTime = computed(() => !s.value?.eternity_unlocked);
const inEC = computed(() =>
  (s.value?.eternity_challenges ?? []).some((c) => c.is_running)
);
const inDilation = computed(() => Boolean(s.value?.dilation?.active));

const showEPRate = computed(
  () => log10(s.value.best_ep_min) <= RATE_THRESHOLD_LOG10
);

// Current EP/min (gained EP over this eternity's real minutes).
const currentEPRate = computed(() => {
  const minutes = Math.max(s.value.this_eternity_real_time_ms / 60000, 0.0005);
  return scaleNum(s.value.gained_eternity_points, 1 / minutes);
});

const epIsOne = computed(
  () =>
    s.value.gained_eternity_points.m === 1 && s.value.gained_eternity_points.e === 0
);

// Red↔green gain coloring above the rate threshold (original amountStyle).
const amountStyle = computed(() => {
  const headerColored = s.value.options?.header_text_colored ?? true;
  if (!headerColored || log10(s.value.eternity_points) < RATE_THRESHOLD_LOG10)
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
  const ratio = log10(s.value.gained_eternity_points) / log10(s.value.eternity_points);
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
</script>

<template>
  <button
    v-if="isVisible"
    class="o-prestige-button"
    :class="{
      'o-eternity-button': !inDilation,
      'o-eternity-button--dilation': inDilation,
      'o-eternity-button--unavailable': !canEternity,
    }"
    @click="game.requestEternity()"
    @mouseover="hover = true"
    @mouseleave="hover = false"
  >
    <!-- Cannot Eternity -->
    <template v-if="!canEternity">
      Reach {{ formatDecimal(s.eternity_goal, 2, 2) }}
      <br>
      Infinity Points
    </template>

    <!-- First time -->
    <template v-else-if="firstTime">
      Other times await.. I need to become Eternal
    </template>

    <!-- Eternity Challenge running -->
    <template v-else-if="inEC">
      Other challenges await... I need to become Eternal
    </template>

    <!-- Dilated run -->
    <template v-else-if="inDilation">
      Eternity for
      <span>{{ formatDecimal(s.dilation.tachyon_gain, 2, 1) }}</span>
      {{ s.dilation.tachyon_gain.m === 1 && s.dilation.tachyon_gain.e === 0
        ? "Tachyon Particle" : "Tachyon Particles" }}
    </template>

    <!-- Normal -->
    <template v-else>
      Eternity for
      <span :style="amountStyle">{{ formatDecimal(s.gained_eternity_points, 2) }}</span>
      <span v-if="showEPRate"> EP</span>
      <span v-else> Eternity {{ epIsOne ? "Point" : "Points" }}</span>
      <br>
      <template v-if="showEPRate">
        Current: {{ formatDecimal(currentEPRate, 2, 2) }} EP/min
        <br>
        Peak: {{ formatDecimal(s.best_ep_min, 2, 2) }} EP/min
        <br>
        at {{ formatDecimal(s.best_ep_min_val, 2, 2) }} EP
      </template>
    </template>
  </button>
</template>
