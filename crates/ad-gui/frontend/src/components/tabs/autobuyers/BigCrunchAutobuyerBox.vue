<script setup>
// The Big Crunch autobuyer row, mirroring the original BigCrunchAutobuyerBox:
// pre-break it is the plain interval-upgrade row; post-break the interval
// slot shows the goal-mode selector (dropdown once the `bigCrunchModes`
// milestone is reached) and the toggle slot the threshold input; amount mode
// adds the "Dynamic amount" checkbox. Locked display (NC12 not complete)
// matches our PrestigeAutobuyerBox.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerInput from "./AutobuyerInput.vue";
import AutobuyerModeDropdown from "./AutobuyerModeDropdown.vue";
import AutobuyerToggleFooter from "./AutobuyerToggleFooter.vue";
import IntervalUpgradeButton from "./IntervalUpgradeButton.vue";

const game = useGameStore();
const entry = computed(() => game.snapshot.autobuyers.big_crunch);
const settings = computed(() => game.snapshot.autobuyers.big_crunch_settings);
const postBreak = computed(() => game.snapshot.broke_infinity);

const MODES = [
  { id: "amount", label: "Big Crunch at X IP" },
  { id: "time", label: "Seconds between Crunches" },
  { id: "xHighest", label: "X times highest IP" },
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
      Automatic Big Crunch
      <div
        v-if="!postBreak"
        class="c-autobuyer-box__small-text"
      >
        Current interval: {{ entry.interval_seconds }} seconds
      </div>
    </div>
    <div class="c-autobuyer-box-row__intervalSlot">
      <IntervalUpgradeButton
        v-if="!entry.has_maxed_interval"
        :entry="entry"
        target="bigCrunch"
      />
      <template v-else-if="postBreak">
        <AutobuyerModeDropdown
          v-if="settings.has_modes"
          :modes="MODES"
          :mode="settings.mode"
          @select="game.setPrestigeAutobuyerMode('bigCrunch', $event)"
        />
        <span v-else>{{ modeLabel(settings.mode) }}:</span>
      </template>
    </div>
    <div class="c-autobuyer-box-row__toggleSlot">
      <AutobuyerInput
        v-if="postBreak"
        :key="settings.mode"
        :value="modeProps.value"
        :type="modeProps.type"
        @commit="game.setPrestigeAutobuyerValue('bigCrunch', $event)"
      />
    </div>
    <div class="c-autobuyer-box-row__checkboxSlot">
      <label
        v-if="postBreak && settings.mode === 'amount'"
        class="o-autobuyer-toggle-checkbox o-clickable"
      >
        <input
          :checked="settings.increase_with_mult"
          type="checkbox"
          class="o-clickable"
          @change="game.toggleAutobuyerDynamicAmount('bigCrunch')"
        >
        Dynamic amount
      </label>
    </div>
    <AutobuyerToggleFooter
      :is-active="entry.is_active"
      @toggle="game.toggleAutobuyer('bigCrunch')"
    />
  </div>
  <div
    v-else
    class="c-autobuyer-buy-box o-primary-btn o-primary-btn--disabled"
  >
    Automatic Big Crunch
    <br>
    Complete Normal Challenge {{ entry.unlock_challenge }} to unlock
  </div>
</template>

<style scoped>
.o-clickable {
  cursor: pointer;
}
</style>
