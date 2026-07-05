<script setup>
// The Reality autobuyer row (RealityAutobuyerBox in the original): shown once
// Reality Upgrade 25 is bought. Always has the mode dropdown; the two input
// boxes swap targets by mode (RM + glyph level, or time + glyph level — the
// original's "hasAlternateInputs" layout, minus the Effarig relic-shard mode).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerInput from "./AutobuyerInput.vue";
import AutobuyerModeDropdown from "./AutobuyerModeDropdown.vue";
import AutobuyerToggleFooter from "./AutobuyerToggleFooter.vue";

const game = useGameStore();
const entry = computed(() => game.snapshot.autobuyers.reality);

const MODES = [
  { id: "rm", label: "Reality Machines" },
  { id: "glyph", label: "Glyph level" },
  { id: "either", label: "RM OR Level" },
  { id: "both", label: "RM AND Level" },
  { id: "time", label: "Real-time seconds" },
];

// Time mode repurposes the first input (the container fits two boxes).
const hasAlternateInputs = computed(() => entry.value.mode === "time");
</script>

<template>
  <div
    v-if="entry.is_unlocked"
    class="c-autobuyer-box-row"
  >
    <div class="l-autobuyer-box__header">
      Automatic Reality
    </div>
    <div class="c-autobuyer-box-row__intervalSlot">
      <AutobuyerModeDropdown
        :modes="MODES"
        :mode="entry.mode"
        @select="game.setRealityAutobuyerMode($event)"
      />
    </div>
    <div class="c-autobuyer-box-row__toggleSlot">
      <div v-if="hasAlternateInputs">
        Target Time (seconds):
      </div>
      <div v-else>
        Target Reality Machines:
      </div>
      <AutobuyerInput
        :key="`first-${hasAlternateInputs}`"
        :value="hasAlternateInputs ? entry.time : entry.rm"
        :type="hasAlternateInputs ? 'float' : 'decimal'"
        @commit="game.setRealityAutobuyerValue(hasAlternateInputs ? 'time' : 'rm', $event)"
      />
    </div>
    <div class="c-autobuyer-box-row__checkboxSlot">
      <div>Target Glyph level:</div>
      <AutobuyerInput
        :value="entry.glyph"
        type="int"
        @commit="game.setRealityAutobuyerValue('glyph', $event)"
      />
    </div>
    <AutobuyerToggleFooter
      :is-active="entry.is_active"
      @toggle="game.toggleAutobuyer('reality')"
    />
  </div>
</template>
