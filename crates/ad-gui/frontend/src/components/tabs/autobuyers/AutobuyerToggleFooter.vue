<script setup>
// The on/off footer of an autobuyer row (check/pause/cross icon + checkbox),
// shared by the prestige autobuyer boxes. Same display logic as
// PrestigeAutobuyerBox's inline footer.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";

const props = defineProps({
  isActive: { type: Boolean, required: true },
});
const emit = defineEmits(["toggle"]);

const game = useGameStore();
const globalOn = computed(() => game.snapshot.autobuyers.enabled);

const toggleIconClass = computed(() => {
  if (!globalOn.value) {
    return props.isActive ? "fas fa-pause" : "fas fa-times";
  }
  return props.isActive ? "fas fa-check" : "fas fa-times";
});

const stateClass = computed(() => {
  if (!globalOn.value) {
    return {
      "o-autobuyer-toggle-checkbox__label": true,
      "o-autobuyer-toggle-checkbox__label--active-paused": props.isActive,
      "o-autobuyer-toggle-checkbox__label--deactive-paused": !props.isActive,
      "o-autobuyer-toggle-checkbox__label--disabled": true,
    };
  }
  return {
    "o-autobuyer-toggle-checkbox__label": true,
    "o-autobuyer-toggle-checkbox__label--active": props.isActive,
    "o-autobuyer-toggle-checkbox__label--disabled": false,
  };
});
</script>

<template>
  <div
    class="l-autobuyer-box__footer"
    @click="emit('toggle')"
  >
    <label :class="stateClass">
      <span :class="toggleIconClass" />
    </label>
    <input
      :checked="isActive && globalOn"
      :disabled="!globalOn"
      type="checkbox"
    >
  </div>
</template>
