<script setup>
// The Perks tab. The original renders a physics-driven vis-network graph;
// we render the same tree as a static SVG using the original's "Default
// Untangled" fixed layout positions and family colors. Click a reachable
// perk to buy it (1 Perk Point each).
import { computed, ref } from "vue";

import { PERKS, PERK_COLORS, PERK_EDGES } from "../../../data/perks";
import { useGameStore } from "../../../stores/game";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);

const bought = computed(() => new Set(reality.value?.perks_bought ?? []));
const buyable = computed(() => new Set(reality.value?.perks_buyable ?? []));

// Fit the layout into the viewBox with some padding.
const PAD = 40;
const xs = PERKS.map((p) => p.x);
const ys = PERKS.map((p) => p.y);
const minX = Math.min(...xs) - PAD;
const minY = Math.min(...ys) - PAD;
const width = Math.max(...xs) - minX + PAD;
const height = Math.max(...ys) - minY + PAD;

const byId = new Map(PERKS.map((p) => [p.id, p]));

const hovered = ref(null);

function nodeFill(perk) {
  const colors = PERK_COLORS[perk.family];
  if (bought.value.has(perk.id)) return colors.primary;
  if (buyable.value.has(perk.id)) return colors.secondary;
  return "#656565";
}

function edgeStroke([a, b]) {
  if (bought.value.has(a) && bought.value.has(b)) return "#0b600e";
  return "#444444";
}

function clickPerk(perk) {
  if (!buyable.value.has(perk.id)) return;
  game.buyPerk(perk.id);
}

const ppText = computed(() => {
  const pp = reality.value?.perk_points ?? 0;
  return `You have ${pp} Perk ${pp === 1 ? "Point" : "Points"}.`;
});
</script>

<template>
  <div
    v-if="reality"
    class="l-perks-tab"
  >
    <div class="c-perk-points">
      {{ ppText }}
    </div>
    <div class="c-perk-hover-text">
      <template v-if="hovered">
        <b>{{ hovered.label }}</b>: {{ hovered.description }}
        <span v-if="!bought.has(hovered.id) && !buyable.has(hovered.id)">
          (Requires an adjacent perk.)
        </span>
      </template>
      <template v-else>
        Hover over a perk for details; click a highlighted perk to buy it.
      </template>
    </div>
    <svg
      class="c-perk-network"
      :viewBox="`${minX} ${minY} ${width} ${height}`"
    >
      <line
        v-for="(edge, i) in PERK_EDGES"
        :key="'e' + i"
        :x1="byId.get(edge[0]).x"
        :y1="byId.get(edge[0]).y"
        :x2="byId.get(edge[1]).x"
        :y2="byId.get(edge[1]).y"
        :stroke="edgeStroke(edge)"
        stroke-width="2"
      />
      <g
        v-for="perk in PERKS"
        :key="perk.id"
        :transform="`translate(${perk.x}, ${perk.y})`"
        :style="{ cursor: buyable.has(perk.id) ? 'pointer' : 'default' }"
        @mouseenter="hovered = perk"
        @mouseleave="hovered = null"
        @click="clickPerk(perk)"
      >
        <circle
          r="11"
          :fill="nodeFill(perk)"
          :stroke="bought.has(perk.id) ? '#094e0b' : '#111111'"
          stroke-width="2"
        />
        <text
          y="22"
          text-anchor="middle"
          class="c-perk-label"
        >{{ perk.label }}</text>
      </g>
    </svg>
  </div>
</template>

<style scoped>
.l-perks-tab {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.c-perk-points {
  font-size: 1.5rem;
  margin: 0.5rem;
  color: var(--color-text);
}

.c-perk-hover-text {
  min-height: 3.6rem;
  max-width: 90rem;
  font-size: 1.2rem;
  margin-bottom: 0.5rem;
  color: var(--color-text);
}

.c-perk-network {
  width: 95%;
  max-height: 60rem;
  background: var(--color-base);
  border: 0.1rem solid var(--color-text);
  border-radius: var(--var-border-radius, 0.5rem);
}

.c-perk-label {
  font-size: 0.9rem;
  fill: var(--color-text);
  user-select: none;
}
</style>
