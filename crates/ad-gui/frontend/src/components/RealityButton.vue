<script setup>
// The header Reality button (RealityButton.vue). Clicking opens the Reality
// (glyph choice) modal; the tooltip lists the other gained resources.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { formatDecimal } from "../util/format";

const game = useGameStore();
const ui = useUiStore();
const reality = computed(() => game.snapshot?.reality);

const canReality = computed(() => Boolean(reality.value?.is_available));

const machinesGained = computed(() => {
  const rm = reality.value?.gained_rm;
  if (!rm || (rm.m === 0)) return "No Machines gained";
  return `Machines gained: ${formatDecimal(rm, 2)}`;
});

const machineStats = computed(() => {
  const r = reality.value;
  if (!r) return "";
  const rm = r.gained_rm;
  const value = rm.m * Math.pow(10, Math.min(rm.e, 300));
  if (!r.unlocked && r.next_machine_ep.e > 8000) return "(Capped this Reality!)";
  if (value > 0 && value < 100) {
    return `(Next at ${formatDecimal(r.next_machine_ep, 2)} EP)`;
  }
  const minutes = Math.max(r.reality_time_minutes, 0.0005);
  return `(${formatDecimal(
    { m: rm.m / minutes, e: rm.e },
    2,
    2
  )} RM/min)`;
});

const glyphLevelText = computed(() => {
  const r = reality.value;
  if (!r) return "";
  if (r.glyph_level >= 10000) return `Glyph level: ${r.glyph_level}`;
  const frac = Math.min(r.glyph_level_exact - Math.floor(r.glyph_level_exact), 0.999);
  const decimals = r.glyph_level > 1000 ? 0 : 1;
  return `Glyph level: ${r.glyph_level} (${(frac * 100).toFixed(decimals)}% to next)`;
});

function handleClick() {
  if (!canReality.value) return;
  ui.showModal("reality");
}
</script>

<template>
  <div class="l-reality-button">
    <button
      class="c-reality-button infotooltip"
      :class="canReality ? 'c-reality-button--unlocked' : 'c-reality-button--locked'"
      @click="handleClick"
    >
      <div class="l-reality-button__contents">
        <template v-if="canReality">
          <div class="c-reality-button__header">
            Make a new Reality
          </div>
          <div>{{ machinesGained }} {{ machineStats }}</div>
          <div>{{ glyphLevelText }}</div>
        </template>
        <template v-else-if="reality?.has_reality_study">
          <div>Get {{ formatDecimal({ m: 1, e: 4000 }, 0) }} Eternity Points to unlock a new Reality</div>
        </template>
        <template v-else>
          <div>Purchase the study in the Eternity tab to unlock a new Reality</div>
        </template>
        <div
          v-if="canReality"
          class="infotooltiptext"
        >
          <div>Other resources gained:</div>
          <div>1 Perk Point</div>
        </div>
      </div>
    </button>
  </div>
</template>
