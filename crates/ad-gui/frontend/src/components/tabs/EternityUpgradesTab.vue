<script setup>
// The Eternity → Eternity Upgrades subtab. Mirrors the original
// EternityUpgradesTab.vue: a 2×3 grid of one-time upgrades (vendored
// o-eternity-upgrade tiles with live effect readouts) plus the rebuyable ×5
// EP-multiplier box and the cost-jump footnote.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal, formatMultiplier } from "../../util/format";

const game = useGameStore();
const s = computed(() => game.snapshot);
const eu = computed(() => s.value?.eternity_upgrades);

// Description + effect wording per original id.
const DESCRIPTIONS = {
  1: "Infinity Dimension multiplier based on unspent Eternity Points (x+1)",
  2: "Infinity Dimension multiplier based on Eternities ((x/200)^log4(2x), softcap at 100,000 Eternities)",
  3: "Infinity Dimension multiplier based on sum of Infinity Challenge times",
  4: "Your Achievement bonus affects Time Dimensions",
  5: "Time Dimensions are multiplied by your unspent Time Theorems",
  6: "Time Dimensions are multiplied by days played",
};

const rows = computed(() => {
  const upgrades = eu.value?.upgrades ?? [];
  return [upgrades.slice(0, 3), upgrades.slice(3, 6)];
});

function classObject(u) {
  return {
    "o-eternity-upgrade": true,
    "o-eternity-upgrade--bought": u.is_bought,
    "o-eternity-upgrade--available": !u.is_bought && u.can_buy,
    "o-eternity-upgrade--unavailable": !u.is_bought && !u.can_buy,
  };
}

function buy(u) {
  if (!u.is_bought && u.can_buy) game.buyEternityUpgrade(u.id);
}
</script>

<template>
  <div
    v-if="eu"
    class="l-eternity-upgrades-grid"
  >
    <div
      v-for="(row, i) in rows"
      :key="i"
      class="l-eternity-upgrades-grid__row"
    >
      <button
        v-for="u in row"
        :key="u.id"
        :class="classObject(u)"
        class="l-eternity-upgrades-grid__cell"
        @click="buy(u)"
      >
        {{ DESCRIPTIONS[u.id] }}
        <br>
        Currently: {{ formatMultiplier(u.effect) }}
        <br>
        Cost: {{ formatDecimal(u.cost, 2) }} EP
      </button>
    </div>
    <div class="l-spoon-btn-group l-margin-top">
      <button
        :class="{
          'o-eternity-upgrade': true,
          'l-eternity-upgrades-grid__cell': true,
          'o-eternity-upgrade--available': eu.can_buy_ep_mult,
          'o-eternity-upgrade--unavailable': !eu.can_buy_ep_mult,
        }"
        @click="game.buyEpMult()"
      >
        You gain ×5 more Eternity Points
        <br>
        Currently: {{ formatMultiplier(eu.ep_mult_effect) }}
        <br>
        Cost: {{ formatDecimal(eu.ep_mult_cost, 2) }} EP
      </button>
      <button
        class="l--spoon-btn-group__little-spoon o-primary-btn--small-spoon"
        @click="game.buyMaxEpMult()"
      >
        Max EP mult purchase
      </button>
    </div>
    <div>
      The cost for the ×5 multiplier jumps at {{ formatDecimal({ m: 1, e: 100 }) }},
      {{ formatDecimal({ m: 1.79769, e: 308 }, 2) }}, and
      {{ formatDecimal({ m: 1, e: 1300 }) }} Eternity Points.
      <br>
      The cost increases super-exponentially after
      {{ formatDecimal({ m: 1, e: 4000 }) }} Eternity Points.
    </div>
  </div>
</template>

<style scoped>
/* From the original EternityUpgradesTab.vue's scoped styles. */
.l-eternity-upgrades-grid {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 1rem;
}

.l-eternity-upgrades-grid__row {
  display: flex;
  flex-direction: row;
}

.l-eternity-upgrades-grid__cell {
  margin: 0.5rem 0.8rem;
}

.l-margin-top {
  margin-top: 1rem;
}
</style>
