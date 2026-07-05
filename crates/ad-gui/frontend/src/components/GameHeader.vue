<script setup>
// Mirrors HeaderPrestigeGroup.vue: a 14rem header row with absolutely
// positioned blocks — the Eternity block at the left quarter
// (HeaderEternityContainer), the antimatter text centered
// (HeaderCenterContainer), and the Infinity-Points readout + post-break
// crunch button at the right quarter (HeaderInfinityContainer).
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";
import EternityButton from "./EternityButton.vue";
import HeaderBigCrunchButton from "./HeaderBigCrunchButton.vue";
import RealityButton from "./RealityButton.vue";

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

// JS (HeaderEternityContainer): `showContainer = player.break ||
// PlayerProgress.eternityUnlocked()`; the EP readout needs eternityUnlocked.
const showEternity = computed(() =>
  Boolean(s.value?.broke_infinity || s.value?.eternity_unlocked)
);
const showEP = computed(() => Boolean(s.value?.eternity_unlocked));

// JS floors the displayed IP (`Currency.infinityPoints.value.floor()`).
const ipIsOne = computed(
  () => s.value?.infinity_points.m === 1 && s.value?.infinity_points.e === 0
);
const epIsOne = computed(
  () => s.value?.eternity_points.m === 1 && s.value?.eternity_points.e === 0
);

// JS (HeaderCenterContainer): everything but antimatter is replaced by the
// Reality button + RM readout once the Reality study is bought.
const hasRealityButton = computed(() =>
  Boolean(s.value?.reality?.unlocked || s.value?.reality?.has_reality_study)
);
const rmIsOne = computed(
  () => s.value?.reality?.machines.m === 1 && s.value?.reality?.machines.e === 0
);
</script>

<template>
  <div
    v-if="s"
    class="c-prestige-info-blocks"
  >
    <div
      v-if="showEternity"
      class="c-prestige-button-container l-game-header__eternity"
    >
      <div
        v-if="showEP"
        class="c-eternity-points"
      >
        You have
        <span class="c-game-header__ep-amount">{{ formatDecimal(s.eternity_points, 2) }}</span>
        {{ epIsOne ? "Eternity Point" : "Eternity Points" }}.
      </div>
      <EternityButton />
    </div>
    <div class="c-prestige-button-container l-game-header__center">
      <span>You have
        <span class="c-game-header__antimatter">{{ formatDecimal(s.antimatter, 2, 1) }}</span>
        antimatter.</span>
      <div
        v-if="hasRealityButton"
        class="c-reality-container"
      >
        <div class="c-reality-currency">
          You have
          <b class="c-reality-tab__reality-machines">{{ formatDecimal(s.reality.machines, 2) }}</b>
          {{ rmIsOne ? "Reality Machine" : "Reality Machines" }}.
        </div>
        <RealityButton />
      </div>
      <template v-else>
        <div>
          You are getting {{ formatDecimal(s.antimatter_per_sec) }} antimatter per second.
        </div>
        <div>
          ADs produce ×{{ perUpgrade }} faster per Tickspeed upgrade
          <br>
          Total Tickspeed: {{ formatDecimal(s.tickspeed_effect, 2, 3) }} / sec
        </div>
      </template>
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
      <HeaderBigCrunchButton />
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

.l-game-header__eternity {
  position: absolute;
  left: calc(25% - 22rem);
  width: 22rem;
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

/* From HeaderInfinityContainer.vue / HeaderEternityContainer.vue. */
.c-infinity-points,
.c-eternity-points {
  font-size: 1.2rem;
  padding-bottom: 0.5rem;
}

/* From HeaderCenterContainer.vue / RealityCurrencyHeader.vue. */
.c-reality-container {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: center;
}

.c-reality-currency {
  font-size: 1.2rem;
  margin-bottom: 1rem;
}
</style>
