<script setup>
// The Eternity → Time Dilation subtab. Mirrors the original
// TimeDilationTab.vue / DilationButton.vue / DilationUpgradeButton.vue
// (vendored o-dilation-btn / o-dilation-upgrade / c-dilation-tab styles):
// TP readout, the dilate button, DT income, the Tachyon-Galaxy threshold
// line, and the upgrade grid — 3 rebuyables, 2×3 one-time upgrades, and the
// TT generator.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";

const game = useGameStore();
const s = computed(() => game.snapshot);
const d = computed(() => s.value?.dilation);

const DESCRIPTIONS = {
  1: "Double Dilated Time gain",
  2: "Reset Dilated Time and Tachyon Galaxies, but lower their threshold",
  3: "Triple the amount of Tachyon Particles gained",
  4: "Gain twice as many Tachyon Galaxies, up to 500 base Galaxies",
  5: "Time Dimensions are affected by Replicanti multiplier ^0.1, reduced effect above ×1e9000",
  6: "Antimatter Dimension multiplier based on Dilated Time, unaffected by Time Dilation",
  7: "Gain a multiplier to Infinity Points based on Dilated Time",
  8: "You can buy all three Time Study paths from the Dimension Split",
  9: "Reduce the Dilation penalty (^0.79 after reduction)",
  10: "Generate Time Theorems based on Tachyon Particles",
};

const upgradeById = computed(
  () => new Map((d.value?.upgrades ?? []).map((u) => [u.id, u]))
);

// Grid rows: rebuyables, then the 2×3 one-time upgrades, then the TT generator.
const rebuyableRow = computed(() =>
  [1, 2, 3].map((id) => upgradeById.value.get(id)).filter(Boolean)
);
const upgradeRows = computed(() => [
  [4, 5, 6].map((id) => upgradeById.value.get(id)).filter(Boolean),
  [7, 8, 9].map((id) => upgradeById.value.get(id)).filter(Boolean),
  [10].map((id) => upgradeById.value.get(id)).filter(Boolean),
]);

function classObject(u) {
  return {
    "o-dilation-upgrade": true,
    "o-dilation-upgrade--rebuyable": u.is_rebuyable,
    "o-dilation-upgrade--available": !u.is_bought && !u.is_capped && u.can_buy,
    "o-dilation-upgrade--unavailable": !u.is_bought && !u.is_capped && !u.can_buy,
    "o-dilation-upgrade--bought": u.is_bought,
    "o-dilation-upgrade--capped": u.is_capped,
  };
}

function costText(u) {
  if (u.is_bought) return "Bought";
  if (u.is_capped) return "Capped";
  return `Cost: ${formatDecimal(u.cost, 2)} Dilated Time`;
}

function buy(u) {
  if (u.can_buy) game.buyDilationUpgrade(u.id);
}

const tpIsOne = computed(
  () => d.value?.tachyon_particles.m === 1 && d.value?.tachyon_particles.e === 0
);
</script>

<template>
  <div
    v-if="d"
    class="l-dilation-tab"
  >
    <span>
      You have
      <span class="c-dilation-tab__tachyons">{{ formatDecimal(d.tachyon_particles, 2, 1) }}</span>
      {{ tpIsOne ? "Tachyon Particle" : "Tachyon Particles" }}.
    </span>
    <button
      class="o-dilation-btn o-dilation-btn--unlocked"
      @click="game.requestDilation()"
    >
      <span v-if="!d.active">Dilate time.</span>
      <span v-else>
        Disable Dilation.
        <br>
        Gain {{ formatDecimal(d.tachyon_gain, 2, 1) }} Tachyon Particles on Eternity.
      </span>
    </button>
    <span>
      You have
      <span class="c-dilation-tab__dilated-time">{{ formatDecimal(d.dilated_time, 2, 1) }}</span>
      Dilated Time.
      <span class="c-dilation-tab__dilated-time-income">+{{ formatDecimal(d.dt_per_second, 2, 1) }}/s</span>
    </span>
    <span>
      Next
      <span v-if="d.tachyon_galaxy_gain > 1">{{ d.tachyon_galaxy_gain }}</span>
      {{ d.tachyon_galaxy_gain === 1 ? "Tachyon Galaxy" : "Tachyon Galaxies" }} at
      <span class="c-dilation-tab__galaxy-threshold">{{ formatDecimal(d.next_threshold, 2, 1) }}</span>
      Dilated Time, gained total of
      <span class="c-dilation-tab__galaxies">{{ Math.floor(d.total_tachyon_galaxies) }}</span>
      {{ d.total_tachyon_galaxies === 1 ? "Tachyon Galaxy" : "Tachyon Galaxies" }}
      ({{ d.base_tachyon_galaxies }} Base)
    </span>
    <div class="l-dilation-upgrades-grid">
      <div class="l-dilation-upgrades-grid__row">
        <button
          v-for="u in rebuyableRow"
          :key="u.id"
          :class="classObject(u)"
          class="l-dilation-upgrades-grid__cell"
          @click="buy(u)"
        >
          {{ DESCRIPTIONS[u.id] }}
          <br>
          <span v-if="u.is_rebuyable">Bought: {{ u.count }}</span>
          <br>
          {{ costText(u) }}
        </button>
      </div>
      <div
        v-for="(row, i) in upgradeRows"
        :key="'row' + i"
        class="l-dilation-upgrades-grid__row"
      >
        <button
          v-for="u in row"
          :key="u.id"
          :class="classObject(u)"
          class="l-dilation-upgrades-grid__cell"
          @click="buy(u)"
        >
          {{ DESCRIPTIONS[u.id] }}
          <br>
          {{ costText(u) }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* From the original TimeDilationTab.vue's scoped styles. */
.l-dilation-tab {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.l-dilation-tab > * {
  margin-top: 0.8rem;
}

.l-dilation-upgrades-grid {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.l-dilation-upgrades-grid__row {
  display: flex;
  flex-direction: row;
}

.l-dilation-upgrades-grid__cell {
  margin: 0.5rem 0.6rem;
}
</style>
