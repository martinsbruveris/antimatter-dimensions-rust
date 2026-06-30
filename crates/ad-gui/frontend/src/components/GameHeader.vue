<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";

const game = useGameStore();
const s = computed(() => game.snapshot);

// Effect per Tickspeed upgrade = reciprocal of the per-upgrade
// purchase multiplier (mirrors HeaderTickspeedInfo's perUpgrade).
const perUpgrade = computed(() =>
  s.value ? (1.0 / s.value.tickspeed_purchase_multiplier).toFixed(3) : "0"
);
</script>

<template>
  <div
    v-if="s"
    class="c-prestige-button-container"
  >
    <span>You have
      <span class="c-game-header__antimatter">{{ formatDecimal(s.antimatter, 2, 1) }}</span>
      antimatter.</span>
    <div>
      You are getting {{ formatDecimal(s.antimatter_per_sec) }} antimatter per second.
    </div>
    <div>
      ADs produce ×{{ perUpgrade }} faster per Tickspeed upgrade
      <br>
      Total Tickspeed: {{ formatDecimal(s.tickspeed_effect, 2, 3) }} / sec
    </div>
  </div>
</template>
