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
  if (view?.is_charged) return "o-infinity-upgrade-btn--charged";
  if (view?.is_bought && view?.can_charge) {
    return "o-infinity-upgrade-btn--chargeable";
  }
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
  if (cell.view?.can_be_bought) {
    game.buyInfinityUpgrade(cell.meta.id);
  } else if (cell.view?.can_charge) {
    // The original's purchase() falls through to charging (Ra's Teresa
    // unlock); a charged upgrade swaps to its charged variant.
    game.chargeInfinityUpgrade(cell.meta.id);
  } else if (cell.view?.is_charged) {
    game.dischargeInfinityUpgrade(cell.meta.id);
  }
}

// The Achievement-41 bottom row (ipMult rebuyable + ipOffline).
const bottom = computed(() => s.value?.infinity_upgrades_bottom_row);

const ipMultClass = computed(() => ({
  "o-infinity-upgrade-btn--bought": bottom.value?.ip_mult_capped,
  "o-infinity-upgrade-btn--available":
    !bottom.value?.ip_mult_capped && bottom.value?.ip_mult_can_be_bought,
  "o-infinity-upgrade-btn--unavailable":
    !bottom.value?.ip_mult_capped && !bottom.value?.ip_mult_can_be_bought,
}));

const ipOfflineClass = computed(() => ({
  "o-infinity-upgrade-btn--bought": bottom.value?.ip_offline_bought,
  "o-infinity-upgrade-btn--available":
    !bottom.value?.ip_offline_bought && bottom.value?.ip_offline_can_be_bought,
  "o-infinity-upgrade-btn--unavailable":
    !bottom.value?.ip_offline_bought && !bottom.value?.ip_offline_can_be_bought,
}));
</script>

<template>
  <div
    v-if="s"
    class="l-infinity-upgrades-tab"
  >
    <br>
    Within each column, the upgrades must be purchased from top to bottom.
    <br>

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

    <div
      v-if="bottom && bottom.unlocked"
      class="l-infinity-upgrades-bottom-row"
    >
      <div class="l-spoon-btn-group l-infinity-upgrades-tab__mult-btn">
        <button
          class="o-infinity-upgrade-btn o-infinity-upgrade-btn--multiplier"
          :class="ipMultClass"
          @click="game.buyIpMult()"
        >
          <span>Multiply Infinity Points from all sources by ×2</span>
          <br>
          ×{{ formatDecimal(bottom.ip_mult_effect, 2, 2) }}
          <template v-if="bottom.ip_mult_capped">
            <br>
            <span>(Capped at
              {{ formatDecimal({ m: 1, e: 6000000 }) }} Infinity Points)</span>
          </template>
          <template v-else>
            <br>
            Cost: {{ formatDecimal(bottom.ip_mult_cost, 2) }} IP
          </template>
        </button>
        <button
          class="o-primary-btn l--spoon-btn-group__little-spoon
            o-primary-btn--small-spoon"
          @click="game.buyMaxIpMult()"
        >
          Max Infinity Point mult
        </button>
        <button
          v-if="bottom.autobuyer_unlocked"
          class="o-primary-btn l--spoon-btn-group__little-spoon
            o-primary-btn--small-spoon"
          @click="game.setIpMultAutobuyer(!bottom.autobuyer_active)"
        >
          Autobuy IP mult {{ bottom.autobuyer_active ? "ON" : "OFF" }}
        </button>
      </div>
      <button
        class="o-infinity-upgrade-btn l-infinity-upgrade-grid__cell
          o-infinity-upgrade-btn--color-2"
        :class="ipOfflineClass"
        @click="game.buyIpOffline()"
      >
        <span>Only while offline, gain 50% of your best IP/min
          without using Max All</span>
        <br>
        {{ formatDecimal(bottom.ip_offline_effect_per_min, 2, 2) }} IP/min
        <template v-if="!bottom.ip_offline_bought">
          <br>
          Cost: {{ formatDecimal(bottom.ip_offline_cost, 2) }} IP
        </template>
      </button>
    </div>
    <div v-if="s.eternity_unlocked && bottom && bottom.unlocked">
      The Infinity Point multiplier becomes more expensive
      <br>
      above {{ formatDecimal({ m: 1, e: 3000000 }) }} Infinity Points,
      and cannot be purchased past
      {{ formatDecimal({ m: 1, e: 6000000 }) }} Infinity Points.
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

.l-infinity-upgrades-bottom-row .l-infinity-upgrade-grid__cell,
.l-infinity-upgrades-bottom-row .l-infinity-upgrades-tab__mult-btn {
  margin: 0.5rem 1.1rem;
}
.o-infinity-upgrade-btn--charged {
  border-color: var(--color-ra--base, #9575cd);
  box-shadow: inset 0 0 0.6rem var(--color-ra--base, #9575cd);
}

.o-infinity-upgrade-btn--chargeable {
  border-style: dashed;
  border-color: var(--color-ra--base, #9575cd);
}
</style>
