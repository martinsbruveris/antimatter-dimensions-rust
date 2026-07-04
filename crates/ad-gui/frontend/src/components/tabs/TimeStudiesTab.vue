<script setup>
// The Time Studies tree. Mirrors the original TimeStudiesTab.vue +
// time-study-tree-layout.js (NORMAL layout) + TimeStudyButton.vue: the TT
// header with the three buy buttons and respec toggle, absolutely-positioned
// study buttons (vendored time-studies.css path colors), and the SVG
// connection lines. EC nodes render as locked placeholders until Feature 4.5.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";
import {
  TIME_STUDY_DESCRIPTIONS,
  TREE_ROWS,
  TREE_CONNECTIONS,
  studyPath,
} from "../../data/timeStudies";

const game = useGameStore();
const s = computed(() => game.snapshot);
const ts = computed(() => s.value?.time_studies);

// --- Layout (time-study-tree-layout.js, scaling = 1; rem units) -------------
const ROW_HEIGHT = 10;
const ROW_SPACING = 4;
const NORMAL = { itemWidth: 18, spacing: 3 };
const WIDE = { itemWidth: 12, spacing: 0.6 };

function rowLayout(row) {
  return row.wide ? WIDE : NORMAL;
}

function rowWidth(row) {
  const l = rowLayout(row);
  return row.items.length * l.itemWidth + (row.items.length - 1) * l.spacing;
}

const treeWidth = computed(() => Math.max(...TREE_ROWS.map(rowWidth)));
const treeHeight = computed(
  () => TREE_ROWS.length * ROW_HEIGHT + (TREE_ROWS.length - 1) * ROW_SPACING
);

// Node positions keyed by id (number or "ECn"): center + box, in rem.
const nodePositions = computed(() => {
  const positions = new Map();
  TREE_ROWS.forEach((row, rowIndex) => {
    const l = rowLayout(row);
    const left0 = (treeWidth.value - rowWidth(row)) / 2;
    row.items.forEach((item, column) => {
      if (item === null) return;
      const left = left0 + column * (l.itemWidth + l.spacing);
      const top = rowIndex * (ROW_HEIGHT + ROW_SPACING);
      positions.set(item, {
        left,
        top,
        width: l.itemWidth,
        height: ROW_HEIGHT,
        cx: left + l.itemWidth / 2,
        cy: top + ROW_HEIGHT / 2,
        small: Boolean(row.wide),
      });
    });
  });
  return positions;
});

const studyById = computed(() => {
  const map = new Map();
  for (const study of ts.value?.studies ?? []) map.set(study.id, study);
  return map;
});

// Renderable nodes: normal studies + EC placeholders.
const nodes = computed(() => {
  const list = [];
  for (const [item, pos] of nodePositions.value) {
    if (typeof item === "number") {
      const study = studyById.value.get(item);
      if (!study) continue;
      const path = studyPath(item);
      const state = study.is_bought
        ? "bought"
        : study.can_buy
          ? "available"
          : "unavailable";
      list.push({
        key: `TS${item}`,
        id: item,
        pos,
        classes: [
          "o-time-study",
          "l-time-study",
          pos.small ? "o-time-study--small" : "",
          `o-time-study--${state}`,
          `o-time-study-${path}--${state}`,
        ],
        description: TIME_STUDY_DESCRIPTIONS[item] ?? "",
        cost: study.cost,
        clickable: study.can_buy,
      });
    } else {
      // EC study slot: a locked placeholder until Feature 4.5.
      const ecId = Number(item.slice(2));
      list.push({
        key: item,
        id: null,
        pos,
        classes: [
          "o-time-study",
          "l-time-study",
          "o-time-study--unavailable",
          "o-time-study-eternity-challenge--unavailable",
        ],
        description: `Eternity Challenge ${ecId}`,
        cost: null,
        clickable: false,
      });
    }
  }
  return list;
});

// Connection lines between node centers (1rem = 10px in the SVG).
const lines = computed(() => {
  const out = [];
  for (const [from, to] of TREE_CONNECTIONS) {
    const a = nodePositions.value.get(from);
    const b = nodePositions.value.get(to);
    if (!a || !b) continue;
    const fromBought =
      typeof from === "number" ? Boolean(studyById.value.get(from)?.is_bought) : false;
    const toPath = typeof to === "number" ? studyPath(to) : "eternity-challenge";
    const pathClass =
      typeof to === "number" && toPath === "normal"
        ? typeof from === "number"
          ? `o-time-study-connection--${studyPath(from)}`
          : ""
        : `o-time-study-connection--${toPath}`;
    out.push({
      key: `${from}-${to}`,
      x1: a.cx * 10,
      y1: a.cy * 10,
      x2: b.cx * 10,
      y2: b.cy * 10,
      classes: [
        "o-time-study-connection",
        fromBought ? "o-time-study-connection--bought" : "",
        pathClass === "o-time-study-connection--normal" ? "" : pathClass,
      ],
    });
  }
  return out;
});

function buy(node) {
  if (node.clickable) game.buyTimeStudy(node.id);
}
</script>

<template>
  <div
    v-if="ts"
    class="l-time-studies-tab"
  >
    <div class="c-subtab-option-container">
      <button
        class="o-primary-btn o-primary-btn--subtab-option"
        :class="{ 'o-primary-btn--respec-active': ts.respec }"
        @click="game.setRespec(!ts.respec)"
      >
        Respec Time Studies on next Eternity
      </button>
    </div>
    <div class="c-time-study-tab__theorems">
      You have
      <span class="c-time-study-tab__theorems-amount">{{ formatDecimal(ts.theorems, 2) }}</span>
      Time {{ ts.theorems.m === 1 && ts.theorems.e === 0 ? "Theorem" : "Theorems" }}.
    </div>
    <div class="l-tt-buy-row">
      <button
        class="o-primary-btn"
        :class="{ 'o-primary-btn--disabled': !ts.can_afford_am }"
        @click="game.buyTimeTheorem('am')"
      >
        Buy 1 TT for {{ formatDecimal(ts.am_cost, 2) }} AM
      </button>
      <button
        class="o-primary-btn"
        :class="{ 'o-primary-btn--disabled': !ts.can_afford_ip }"
        @click="game.buyTimeTheorem('ip')"
      >
        Buy 1 TT for {{ formatDecimal(ts.ip_cost, 2) }} IP
      </button>
      <button
        class="o-primary-btn"
        :class="{ 'o-primary-btn--disabled': !ts.can_afford_ep }"
        @click="game.buyTimeTheorem('ep')"
      >
        Buy 1 TT for {{ formatDecimal(ts.ep_cost, 2) }} EP
      </button>
      <button
        class="o-primary-btn"
        @click="game.buyMaxTimeTheorems()"
      >
        Buy max
      </button>
    </div>
    <div
      v-if="!ts.can_buy_tt"
      class="c-tt-locked-note"
    >
      You need to buy at least 1 Time Dimension before you can purchase Time Theorems.
    </div>
    <div
      class="l-time-study-tree"
      :style="{ width: `${treeWidth}rem`, height: `${treeHeight}rem` }"
    >
      <button
        v-for="node in nodes"
        :key="node.key"
        :class="node.classes"
        :style="{ top: `${node.pos.top}rem`, left: `${node.pos.left}rem` }"
        @click="buy(node)"
      >
        {{ node.description }}
        <template v-if="node.cost !== null">
          <br>
          Cost: {{ node.cost }} Time {{ node.cost === 1 ? "Theorem" : "Theorems" }}
        </template>
        <template v-else>
          <br>
          (Feature 4.5)
        </template>
      </button>
      <svg
        class="l-time-study-connection"
        :width="treeWidth * 10"
        :height="treeHeight * 10"
      >
        <line
          v-for="line in lines"
          :key="line.key"
          :x1="line.x1"
          :y1="line.y1"
          :x2="line.x2"
          :y2="line.y2"
          :class="line.classes"
        />
      </svg>
    </div>
  </div>
</template>

<style scoped>
/* Tab shell: the tree is centered and scrolls with the page (original
   .l-time-studies-tab is a centered flex column). */
.l-time-studies-tab {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.c-time-study-tab__theorems {
  margin: 0.8rem 0;
  font-size: 1.5rem;
}

.c-time-study-tab__theorems-amount {
  font-size: 1.8rem;
  font-weight: bold;
}

.l-tt-buy-row {
  display: flex;
  gap: 0.6rem;
  margin-bottom: 1rem;
}

.c-tt-locked-note {
  margin-bottom: 0.8rem;
  color: var(--color-bad);
}

/* Node/line positioning comes from the vendored time-studies.css
   (.l-time-study-tree / .l-time-study / .l-time-study-connection); only the
   tab-local margin is ours. */
.l-time-study-tree {
  margin: 0 auto 2rem;
}

.l-time-study-connection {
  pointer-events: none;
}
</style>
