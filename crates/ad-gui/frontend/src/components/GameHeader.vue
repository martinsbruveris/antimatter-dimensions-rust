<script setup>
// Mirrors HeaderPrestigeGroup.vue: a 14rem header row with absolutely
// positioned blocks — the antimatter text centered (HeaderCenterContainer)
// and, once Infinity is unlocked, the Infinity-Points readout at the right
// quarter (HeaderInfinityContainer). The eternity block (left quarter) and
// the post-break header crunch button are later features.
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

// JS: `showContainer = player.break || PlayerProgress.infinityUnlocked()`.
const showInfinity = computed(() =>
  Boolean(s.value?.broke_infinity || s.value?.infinity_unlocked)
);

// JS floors the displayed IP (`Currency.infinityPoints.value.floor()`).
const ipIsOne = computed(
  () => s.value?.infinity_points.m === 1 && s.value?.infinity_points.e === 0
);
</script>

<template>
  <div
    v-if="s"
    class="c-prestige-info-blocks"
  >
    <div class="c-prestige-button-container l-game-header__center">
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
    <div
      v-if="showInfinity"
      class="c-prestige-button-container l-game-header__infinity"
    >
      <div class="c-infinity-points">
        You have
        <span class="c-game-header__ip-amount">{{ formatDecimal(s.infinity_points, 2) }}</span>
        {{ ipIsOne ? "Infinity Point" : "Infinity Points" }}.
      </div>
    </div>
  </div>
</template>

<style scoped>
/* From HeaderPrestigeGroup.vue's scoped styles. The absolute blocks anchor to
   the vendored `.tab-container` (position: relative), as in the original. */
.c-prestige-info-blocks {
  display: flex;
  flex-direction: row;
  height: 14rem;
  width: 100%;
  color: var(--color-text);
}

.l-game-header__center {
  position: absolute;
  right: calc(50% - 25rem);
  width: 50rem;
}

.l-game-header__infinity {
  position: absolute;
  right: calc(25% - 22rem);
  width: 22rem;
}

/* From HeaderInfinityContainer.vue's scoped styles. */
.c-infinity-points {
  font-size: 1.2rem;
  padding-bottom: 0.5rem;
}
</style>
