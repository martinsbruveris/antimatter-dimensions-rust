<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import IntervalUpgradeButton from "./IntervalUpgradeButton.vue";

// The Dim Boost / Galaxy / Big Crunch autobuyers. Unlike the AD/Tickspeed boxes
// these have no antimatter "slow version": they show a full row once their
// Normal Challenge is completed (`is_unlocked`), and a locked hint before that.
const props = defineProps({
  entry: { type: Object, required: true },
  // The string autobuyer handle ("dimBoost" / "galaxy" / "bigCrunch").
  target: { type: String, required: true },
});

const game = useGameStore();
const globalOn = computed(() => game.snapshot.autobuyers.enabled);

const toggleIconClass = computed(() => {
  if (!globalOn.value) {
    return props.entry.is_active ? "fas fa-pause" : "fas fa-times";
  }
  return props.entry.is_active ? "fas fa-check" : "fas fa-times";
});

const stateClass = computed(() => {
  if (!globalOn.value) {
    return {
      "o-autobuyer-toggle-checkbox__label": true,
      "o-autobuyer-toggle-checkbox__label--active-paused": props.entry.is_active,
      "o-autobuyer-toggle-checkbox__label--deactive-paused": !props.entry.is_active,
      "o-autobuyer-toggle-checkbox__label--disabled": true,
    };
  }
  return {
    "o-autobuyer-toggle-checkbox__label": true,
    "o-autobuyer-toggle-checkbox__label--active": props.entry.is_active,
    "o-autobuyer-toggle-checkbox__label--disabled": false,
  };
});
</script>

<template>
  <div
    v-if="entry.is_unlocked"
    class="c-autobuyer-box-row"
  >
    <div class="l-autobuyer-box__header">
      {{ entry.name }}
      <div class="c-autobuyer-box__small-text">
        Current interval: {{ entry.interval_seconds }} seconds
      </div>
    </div>
    <div class="c-autobuyer-box-row__intervalSlot">
      <IntervalUpgradeButton
        :entry="entry"
        :target="target"
      />
    </div>
    <div class="c-autobuyer-box-row__toggleSlot" />
    <div class="c-autobuyer-box-row__checkboxSlot" />
    <div
      class="l-autobuyer-box__footer"
      @click="game.toggleAutobuyer(target)"
    >
      <label :class="stateClass">
        <span :class="toggleIconClass" />
      </label>
      <input
        :checked="entry.is_active && globalOn"
        :disabled="!globalOn"
        type="checkbox"
      >
    </div>
  </div>
  <div
    v-else
    class="c-autobuyer-buy-box o-primary-btn o-primary-btn--disabled"
  >
    {{ entry.name }}
    <br>
    Complete Normal Challenge {{ entry.unlock_challenge }} to unlock
  </div>
</template>
