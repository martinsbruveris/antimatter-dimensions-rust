<script setup>
// The Infinity → Infinity Upgrades subtab: the Infinity-Points header (the
// original's `InfinityPointsHeader`, shown as the tab's `before` chrome) plus the
// 4×4 upgrade grid. Mirrors the original InfinityUpgradesTab.vue +
// InfinityUpgradeButton.vue. Owned-state / affordability / cost / effect values
// come from the engine snapshot; the layout + descriptions come from
// data/infinityUpgrades.js. Charged (Ra) upgrades are not modelled.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";
import { INFINITY_UPGRADE_COLUMNS } from "../../data/infinityUpgrades";

const game = useGameStore();
const s = computed(() => game.snapshot);

// Singular only for exactly one Infinity Point (mirrors the original's
// `pluralize("Infinity Point", infinityPoints)`).
const ipWord = computed(() => {
  const ip = s.value?.infinity_points;
  const isOne = ip && ip.m === 1 && ip.e === 0;
  return isOne ? "Infinity Point" : "Infinity Points";
});

// Snapshot upgrade views keyed by id.
const byId = computed(
  () => new Map((s.value?.infinity_upgrades ?? []).map((u) => [u.id, u])),
);

// Colour a column-background segment: lit for an owned cell, transparent otherwise.
function segColor(view) {
  return view?.is_bought ? "var(--color-infinity)" : "transparent";
}

// The grid: metadata columns joined with live snapshot state + the per-column
// background gradient (lit bands for owned cells, matching the original).
const columns = computed(() =>
  INFINITY_UPGRADE_COLUMNS.map((col, colIndex) => {
    const cells = col.map((meta) => ({ meta, view: byId.value.get(meta.id) ?? null }));
    const c = cells.map((cell) => segColor(cell.view));
    const bgStyle = {
      background:
        `linear-gradient(to bottom, ${c[0]} 15%, ${c[1]} 35% 40%, ` +
        `${c[2]} 60% 65%, ${c[3]} 85% 100%)`,
    };
    // Columns 1..3 get the accent-colour variants (column 0 is the default).
    const colorClass =
      colIndex > 0 ? `o-infinity-upgrade-btn--color-${colIndex + 1}` : null;
    return { colIndex, cells, bgStyle, colorClass };
  }),
);

function stateClass(view) {
  if (view?.is_bought) return "o-infinity-upgrade-btn--bought";
  if (view?.can_be_bought) return "o-infinity-upgrade-btn--available";
  return "o-infinity-upgrade-btn--unavailable";
}

// The effect line under the description, or null when the tile has none.
function effectLine(cell) {
  const effect = cell.meta.effect;
  if (!effect) return null;
  if (effect.kind === "text") return effect.text;
  if (effect.kind === "mult") {
    return `×${formatDecimal(cell.view?.effect, effect.places, effect.under)}`;
  }
  return null;
}

function buy(cell) {
  if (cell.view?.can_be_bought) game.buyInfinityUpgrade(cell.meta.id);
}
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

    Within each column, the upgrades must be purchased from top to bottom.

    <div class="l-infinity-upgrade-grid l-infinity-upgrades-tab__grid">
      <div
        v-for="col in columns"
        :key="col.colIndex"
        class="c-infinity-upgrade-grid__column"
      >
        <button
          v-for="cell in col.cells"
          :key="cell.meta.id"
          class="o-infinity-upgrade-btn l-infinity-upgrade-grid__cell"
          :class="[col.colorClass, stateClass(cell.view)]"
          @click="buy(cell)"
        >
          <span>{{ cell.meta.description }}</span>
          <template v-if="effectLine(cell)">
            <br>
            {{ effectLine(cell) }}
          </template>
          <template v-if="cell.view && !cell.view.is_bought">
            <br>
            Cost: {{ formatDecimal(cell.view.cost, 2) }} IP
          </template>
        </button>
        <div
          class="c-infinity-upgrade-grid__column--background"
          :style="col.bgStyle"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original InfinityUpgradesTab.vue scoped style (not in the
   global vendored CSS): the per-column fl* container + the absolutely-positioned
   lit-band background behind each column. */
.c-infinity-upgrade-grid__column {
  display: flex;
  overflow: hidden;
  flex-direction: column;
  position: relative;
  border-radius: var(--var-border-radius, 0.3rem);
  margin: 0 0.3rem;
}

.c-infinity-upgrade-grid__column--background {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  z-index: -1;
  opacity: 0.7;
}

.s-base--dark .c-infinity-upgrade-grid__column--background {
  opacity: 0.5;
}
</style>
