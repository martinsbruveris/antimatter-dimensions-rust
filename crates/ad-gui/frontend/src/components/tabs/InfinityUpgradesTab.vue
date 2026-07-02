<script setup>
// The Infinity → Infinity Upgrades subtab. For now it renders only the
// Infinity-Points header (the original's `InfinityPointsHeader`, shown as the
// infinity tab's `before` chrome). The upgrade grid — InfinityUpgradeButton
// columns + IP-multiplier button — lands with Feature 2.2; the vendored
// `.l-infinity-upgrades-tab` container is kept so it slots straight in.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";

const game = useGameStore();
const s = computed(() => game.snapshot);

// Singular only for exactly one Infinity Point (mirrors the original's
// `pluralize("Infinity Point", infinityPoints)`).
const ipWord = computed(() => {
  const ip = s.value?.infinity_points;
  const isOne = ip && ip.m === 1 && ip.e === 0;
  return isOne ? "Infinity Point" : "Infinity Points";
});
</script>

<template>
  <div
    v-if="s"
    class="l-infinity-upgrades-tab"
  >
    <div class="c-infinity-tab__header">
      You have
      <span class="c-infinity-tab__infinity-points">{{ formatDecimal(s.infinity_points, 2) }}</span>
      {{ ipWord }}.
    </div>
  </div>
</template>
