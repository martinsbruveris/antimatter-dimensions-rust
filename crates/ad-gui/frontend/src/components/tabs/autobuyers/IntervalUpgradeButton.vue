<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

// The per-autobuyer interval-upgrade button, mirroring the original
// `AutobuyerIntervalButton.vue`. Once the autobuyer's Normal Challenge is
// completed (`can_be_upgraded`) it reduces the interval one step for Infinity
// Points ("40% smaller interval / Cost: N IP") down to the 100 ms floor
// (`has_maxed_interval`); before the challenge it shows the locked hint. Once
// the interval is minimized nothing is rendered.
const props = defineProps({
  entry: { type: Object, required: true },
  // The string autobuyer handle passed to `upgrade_autobuyer_interval`
  // ("ad0".."ad7", "tickspeed", "dimBoost", "galaxy", "bigCrunch").
  target: { type: String, required: true },
});

const game = useGameStore();

// Original: `Cost: {{ format(cost, 2) }} IP`.
const cost = computed(() => formatDecimal(props.entry.upgrade_cost, 2));

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
    v-if="!entry.has_maxed_interval && entry.can_be_upgraded"
    :class="btnClass"
    @click="onClick"
  >
    40% smaller interval
    <br>
    Cost: {{ cost }} IP
  </button>
  <button
    v-else-if="!entry.has_maxed_interval"
    class="o-autobuyer-btn l-autobuyer-box__button o-autobuyer-btn--unavailable"
  >
    Complete the challenge to upgrade interval
  </button>
</template>
