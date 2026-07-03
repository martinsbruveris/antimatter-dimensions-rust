<script setup>
// The Infinity → Break Infinity subtab: the IP header plus the 9 one-time and 3
// rebuyable Break Infinity Upgrade buttons. Owned state / affordability / cost /
// counts come from the engine snapshot (`game.break_infinity`); descriptions come
// from data/breakInfinityUpgrades.js. Reuses the vendored infinity-upgrade button
// styling (accent colour 2, matching the BREAK INFINITY button).
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";
import {
  BREAK_INFINITY_UPGRADES,
  BREAK_INFINITY_REBUYABLES,
} from "../../data/breakInfinityUpgrades";

const game = useGameStore();
const s = computed(() => game.snapshot);
const bi = computed(() => s.value?.break_infinity);

const byId = computed(
  () => new Map((bi.value?.upgrades ?? []).map((u) => [u.id, u])),
);
const rebuyById = computed(
  () => new Map((bi.value?.rebuyables ?? []).map((r) => [r.id, r])),
);

const upgrades = computed(() =>
  BREAK_INFINITY_UPGRADES.map((meta) => ({
    meta,
    view: byId.value.get(meta.id) ?? null,
  })),
);
const rebuyables = computed(() =>
  BREAK_INFINITY_REBUYABLES.map((meta) => ({
    meta,
    view: rebuyById.value.get(meta.id) ?? null,
  })),
);

function upgradeClass(view) {
  return {
    "o-infinity-upgrade-btn": true,
    "o-infinity-upgrade-btn--color-2": true,
    "o-infinity-upgrade-btn--bought": Boolean(view?.is_bought),
    "o-infinity-upgrade-btn--available":
      view && !view.is_bought && view.can_be_bought,
    "o-infinity-upgrade-btn--unavailable":
      view && !view.is_bought && !view.can_be_bought,
  };
}

function rebuyClass(view) {
  const capped = view && view.count >= view.max;
  return {
    "o-infinity-upgrade-btn": true,
    "o-infinity-upgrade-btn--color-2": true,
    "o-infinity-upgrade-btn--bought": Boolean(capped),
    "o-infinity-upgrade-btn--available": view && !capped && view.can_be_bought,
    "o-infinity-upgrade-btn--unavailable": view && !capped && !view.can_be_bought,
  };
}

function buyUpgrade(cell) {
  if (cell.view?.can_be_bought) game.buyBreakInfinityUpgrade(cell.meta.id);
}
function buyRebuyable(cell) {
  if (cell.view?.can_be_bought) game.buyBreakInfinityRebuyable(cell.meta.id);
}
</script>

<template>
  <div
    v-if="bi"
    class="l-break-infinity-tab"
  >
    <div class="c-infinity-tab__header">
      You have
      <span class="c-infinity-tab__infinity-points">{{
        formatDecimal(s.infinity_points, 2)
      }}</span>
      Infinity Points.
    </div>

    <div class="l-break-infinity-grid">
      <button
        v-for="cell in upgrades"
        :key="cell.meta.id"
        :class="upgradeClass(cell.view)"
        @click="buyUpgrade(cell)"
      >
        <span>{{ cell.meta.description }}</span>
        <template v-if="cell.meta.deferred">
          <br><i>(no effect yet)</i>
        </template>
        <template v-if="cell.view && !cell.view.is_bought">
          <br>Cost: {{ formatDecimal(cell.view.cost, 2) }} IP
        </template>
      </button>

      <button
        v-for="cell in rebuyables"
        :key="`r${cell.meta.id}`"
        :class="rebuyClass(cell.view)"
        @click="buyRebuyable(cell)"
      >
        <span>{{ cell.meta.description }}</span>
        <template v-if="cell.meta.deferred">
          <br><i>(no effect yet)</i>
        </template>
        <template v-if="cell.view">
          <br>{{ cell.view.count }} / {{ cell.view.max }}
          <template v-if="cell.view.count < cell.view.max">
            <br>Cost: {{ formatDecimal(cell.view.cost, 2) }} IP
          </template>
        </template>
      </button>
    </div>
  </div>
</template>

<style scoped>
.l-break-infinity-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 1rem;
}

.l-break-infinity-grid .o-infinity-upgrade-btn {
  width: 16rem;
  min-height: 6rem;
}
</style>
