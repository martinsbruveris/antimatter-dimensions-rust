<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

// The per-autobuyer interval-upgrade button. Reduces the interval one step for
// Infinity Points once the autobuyer's Normal Challenge is completed
// (`can_be_upgraded`), down to the 100 ms floor (`has_maxed_interval`). Before
// the challenge it shows the unlock hint; when maxed it is inert.
const props = defineProps({
  entry: { type: Object, required: true },
  // The string autobuyer handle passed to `upgrade_autobuyer_interval`
  // ("ad0".."ad7", "tickspeed", "dimBoost", "galaxy", "bigCrunch").
  target: { type: String, required: true },
});

const game = useGameStore();

const text = computed(() => {
  if (props.entry.has_maxed_interval) return "Interval minimized";
  if (!props.entry.can_be_upgraded) {
    return `Complete Normal Challenge ${props.entry.unlock_challenge} to upgrade interval`;
  }
  return `Reduce interval: ${formatDecimal(props.entry.upgrade_cost, 0)} IP`;
});

const btnClass = computed(() => ({
  "o-autobuyer-btn": true,
  "l-autobuyer-box__button": true,
  "o-autobuyer-btn--unavailable": !props.entry.can_afford_upgrade,
}));

function onClick() {
  if (!props.entry.can_afford_upgrade) return;
  game.upgradeAutobuyerInterval(props.target);
}
</script>

<template>
  <button
    :class="btnClass"
    @click="onClick"
  >
    {{ text }}
  </button>
</template>
