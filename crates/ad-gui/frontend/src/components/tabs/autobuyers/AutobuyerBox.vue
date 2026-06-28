<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

// Shared autobuyer entry: a full row once unlocked, or a purchase box before
// then. Mirrors the original AutobuyerBox.vue. The interval-upgrade and mode
// controls are passed in via the `intervalSlot` / `toggleSlot` slots.
const props = defineProps({
  entry: { type: Object, required: true },
  // Antimatter Dimension autobuyers display a "Current bulk" line; tickspeed
  // does not.
  showBulk: { type: Boolean, default: false },
});
const emit = defineEmits(["unlock", "toggle"]);

const game = useGameStore();
const globalOn = computed(() => game.snapshot.autobuyers.enabled);

const buyBoxClass = computed(() => ({
  "c-autobuyer-buy-box": true,
  "o-primary-btn": true,
  "o-primary-btn--enabled": props.entry.can_unlock,
  "o-primary-btn--disabled": !props.entry.can_unlock,
}));

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
    v-if="entry.is_bought"
    class="c-autobuyer-box-row"
  >
    <div class="l-autobuyer-box__header">
      {{ entry.name }}
      <div class="c-autobuyer-box__small-text">
        Current interval: {{ entry.interval_seconds }} seconds
        <span v-if="showBulk">
          <br>
          Current bulk: ×1.00
        </span>
      </div>
    </div>
    <div class="c-autobuyer-box-row__intervalSlot">
      <slot name="intervalSlot" />
    </div>
    <div class="c-autobuyer-box-row__toggleSlot">
      <slot name="toggleSlot" />
    </div>
    <div class="c-autobuyer-box-row__checkboxSlot" />
    <div
      class="l-autobuyer-box__footer"
      @click="emit('toggle')"
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
    :class="buyBoxClass"
    @click="emit('unlock')"
  >
    {{ entry.name }}
    <br>
    Requirement: {{ formatDecimal(entry.requirement) }} Total Antimatter
  </div>
</template>
