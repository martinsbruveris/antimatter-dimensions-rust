<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";

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
      <span class="c-game-header__antimatter">{{ s.antimatter }}</span>
      antimatter.</span>
    <div>
      You are getting {{ s.antimatter_per_sec }} antimatter per second.
      <br>
      ADs produce ×{{ perUpgrade }} faster per Tickspeed upgrade
      <br>
      Total Tickspeed: {{ s.tickspeed_effect }} / sec
    </div>
  </div>
</template>
