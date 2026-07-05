<script setup>
// The Reality Upgrades tab (RealityUpgradesTab.vue): the rebuyable Amplifier
// row + four rows of one-time upgrades, using the vendored
// c-reality-upgrade-btn styles.
import { computed } from "vue";

import {
  REALITY_REBUYABLES,
  REALITY_UPGRADES,
} from "../../../data/realityUpgrades";
import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);

const rebuyables = computed(() => {
  const state = reality.value?.rebuyables ?? [];
  return REALITY_REBUYABLES.map((def, i) => ({ ...def, ...state[i] }));
});
const upgrades = computed(() => {
  const state = reality.value?.upgrades ?? [];
  return REALITY_UPGRADES.map((def, i) => ({ ...def, ...state[i] }));
});
const upgradeRows = computed(() => {
  const rows = [];
  for (let r = 0; r < 4; r++) rows.push(upgrades.value.slice(r * 5, r * 5 + 5));
  return rows;
});

function classFor(upgrade) {
  return {
    "c-reality-upgrade-btn": true,
    "c-reality-upgrade-btn--bought": upgrade.is_bought,
    "c-reality-upgrade-btn--unavailable": !upgrade.is_bought && !upgrade.can_buy,
  };
}
</script>

<template>
  <div
    v-if="reality"
    class="l-reality-upgrade-grid"
  >
    <div class="c-reality-upgrade-infotext">
      The first row of upgrades can be purchased endlessly for increasing costs
      and the rest are single-purchase.
      <br>
      Single-purchase upgrades also have requirements which, once completed,
      permanently unlock the ability to purchase the upgrades at any point.
      <br>
      Every completed row of purchased upgrades increases your Glyph level by 1.
    </div>
    <div class="l-reality-upgrade-grid__row">
      <button
        v-for="upgrade in rebuyables"
        :key="upgrade.id"
        class="c-reality-upgrade-btn"
        :class="{ 'c-reality-upgrade-btn--unavailable': !upgrade.can_buy }"
        @click="game.buyRealityRebuyable(upgrade.id)"
      >
        <b>{{ upgrade.name }}</b>
        <div>{{ upgrade.description }}</div>
        <div>Purchased {{ upgrade.count }} {{ upgrade.count === 1 ? "time" : "times" }}</div>
        <div>Cost: {{ formatDecimal(upgrade.cost, 2, 0) }} RM</div>
      </button>
    </div>
    <div
      v-for="(row, r) in upgradeRows"
      :key="r"
      class="l-reality-upgrade-grid__row"
    >
      <button
        v-for="upgrade in row"
        :key="upgrade.id"
        :class="classFor(upgrade)"
        @click="game.buyRealityUpgrade(upgrade.id)"
      >
        <b>{{ upgrade.name }}</b>
        <template v-if="upgrade.is_bought">
          <div>{{ upgrade.description }}</div>
        </template>
        <template v-else-if="upgrade.req_met">
          <div>{{ upgrade.description }}</div>
          <div>Cost: {{ formatDecimal(upgrade.cost, 2, 0) }} RM</div>
        </template>
        <template v-else>
          <div class="o-requirement">
            Requires: {{ upgrade.requirement }}
          </div>
          <div>{{ upgrade.description }}</div>
        </template>
      </button>
    </div>
  </div>
</template>

<style scoped>
.l-reality-upgrade-grid {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.c-reality-upgrade-infotext {
  color: var(--color-text);
  margin: 0 0 1.5rem;
}

.l-reality-upgrade-grid__row {
  display: flex;
  flex-direction: row;
}

.c-reality-upgrade-btn {
  width: 20rem;
  min-height: 13rem;
  font-family: Typewriter, serif;
  font-size: 1rem;
  margin: 0.5rem;
    padding: 0.5rem;
}

.o-requirement {
  color: var(--color-reality-light, #58a642);
  font-weight: bold;
}
</style>
