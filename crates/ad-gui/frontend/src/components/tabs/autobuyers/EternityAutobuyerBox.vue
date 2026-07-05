<script setup>
// The Eternity autobuyer row (EternityAutobuyerBox in the original): shown
// once the 100-Eternities milestone is reached, with the mode dropdown gated
// on Reality Upgrade 13. Like the original, no interval line (this autobuyer
// checks its condition every tick).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerInput from "./AutobuyerInput.vue";
import AutobuyerModeDropdown from "./AutobuyerModeDropdown.vue";
import AutobuyerToggleFooter from "./AutobuyerToggleFooter.vue";

const game = useGameStore();
const entry = computed(() => game.snapshot.autobuyers.eternity);
const settings = computed(() => entry.value.settings);

const MODES = [
  { id: "amount", label: "Eternity at X EP" },
  { id: "time", label: "Seconds between Eternities" },
  { id: "xHighest", label: "X times highest EP" },
];

const modeProps = computed(() => {
  switch (settings.value.mode) {
    case "time":
      return { value: settings.value.time, type: "float" };
    case "xHighest":
      return { value: settings.value.x_highest, type: "decimal" };
    default:
      return { value: settings.value.amount, type: "decimal" };
  }
});

function modeLabel(id) {
  return MODES.find((m) => m.id === id)?.label ?? "";
}
</script>

<template>
  <div
    v-if="entry.is_unlocked"
    class="c-autobuyer-box-row"
  >
    <div class="l-autobuyer-box__header">
      Automatic Eternity
    </div>
    <div class="c-autobuyer-box-row__intervalSlot">
      <AutobuyerModeDropdown
        v-if="settings.has_modes"
        :modes="MODES"
        :mode="settings.mode"
        @select="game.setPrestigeAutobuyerMode('eternity', $event)"
      />
      <span v-else>{{ modeLabel(settings.mode) }}:</span>
    </div>
    <div class="c-autobuyer-box-row__toggleSlot">
      <AutobuyerInput
        :key="settings.mode"
        :value="modeProps.value"
        :type="modeProps.type"
        @commit="game.setPrestigeAutobuyerValue('eternity', $event)"
      />
    </div>
    <div class="c-autobuyer-box-row__checkboxSlot">
      <label
        v-if="settings.mode === 'amount'"
        class="o-autobuyer-toggle-checkbox o-clickable"
      >
        <input
          :checked="settings.increase_with_mult"
          type="checkbox"
          class="o-clickable"
          @change="game.toggleAutobuyerDynamicAmount('eternity')"
        >
        Dynamic amount
      </label>
    </div>
    <AutobuyerToggleFooter
      :is-active="entry.is_active"
      @toggle="game.toggleAutobuyer('eternity')"
    />
  </div>
</template>

<style scoped>
.o-clickable {
  cursor: pointer;
}
</style>
