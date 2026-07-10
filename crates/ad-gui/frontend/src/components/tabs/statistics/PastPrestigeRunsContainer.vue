<script setup>
// One prestige layer's "Last 10 runs" table — a port of the original
// PastPrestigeRunsContainer.vue. Simplifications (engine rings store only
// `[time, realTime, currency, count]`): no Challenge column and no per-layer
// extra columns (TT / Glyph Level / Relic Shards), and the Real Time column
// shows once Reality is unlocked (instead of `seenAlteredSpeed`). See
// docs/design/2026-07-10-statistics-tab.md.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal, timeDisplayShort } from "../../../util/format";

const props = defineProps({
  // { key, name, plural, currency } — key is the engine layer id
  // ("infinity" / "eternity" / "reality").
  layer: { type: Object, required: true },
  // RecentRunView[]: { time_ms, real_time_ms, currency: Num, count: Num }.
  runs: { type: Array, required: true },
  shown: { type: Boolean, required: true },
  resourceType: { type: Number, required: true },
  hasRealTime: { type: Boolean, required: true },
});

const game = useGameStore();

// ----- { m, e } helpers -------------------------------------------------
const numLog10 = (num) => (num.m === 0 ? -Infinity : Math.log10(num.m) + num.e);

function normalizeNum(m, e) {
  if (m === 0 || !Number.isFinite(m)) return { m: 0, e: 0 };
  const shift = Math.floor(Math.log10(Math.abs(m)));
  return { m: m / Math.pow(10, shift), e: e + shift };
}

// Scale a Num by an f64 factor (used for the per-minute/per-hour rates).
const scaleNum = (num, factor) => normalizeNum(num.m * factor, num.e);

// Arithmetic mean of Nums: sum mantissas relative to the largest exponent
// (terms > ~15 orders below it underflow to 0 — negligible in a mean).
function averageNums(nums) {
  const maxE = Math.max(...nums.map((n) => n.e));
  const sum = nums.reduce((acc, n) => acc + n.m * Math.pow(10, n.e - maxE), 0);
  return normalizeNum(sum / nums.length, maxE);
}

// ----- the 11 table rows (10 runs + average) ------------------------------
// The original's `averageRun`: average the valid runs; with none, reuse the
// placeholder so the average row renders the "cannot calculate" text.
const averageRun = computed(() => {
  const valid = props.runs.filter((r) => r.time_ms !== Number.MAX_VALUE);
  if (valid.length === 0) return props.runs[0];
  return {
    time_ms: valid.reduce((a, r) => a + r.time_ms, 0) / valid.length,
    real_time_ms: valid.reduce((a, r) => a + r.real_time_ms, 0) / valid.length,
    currency: averageNums(valid.map((r) => r.currency)),
    count: averageNums(valid.map((r) => r.count)),
  };
});

const tableRuns = computed(() => [...props.runs, averageRun.value]);

// ----- resource-pair selection (the "Showing X" cycle) --------------------
// RECENT_PRESTIGE_RESOURCE: 0 ABSOLUTE_GAIN, 1 RATE, 2 CURRENCY, 3 COUNT.
const selectedResources = computed(() => {
  switch (props.resourceType) {
    case 0: return [0, 2];
    case 1: return [1, 3];
    case 2: return [0, 1];
    default: return [2, 3];
  }
});

const resourceTitles = computed(() => {
  const names = [
    props.layer.currency,
    `${props.layer.currency} Rate`,
    props.layer.plural,
    `${props.layer.name} Rate`,
  ];
  return selectedResources.value.map((i) => names[i]);
});

// ----- cell text ----------------------------------------------------------
function pluralizeCount(count) {
  const isOne = count.m === 1 && count.e === 0;
  if (isOne) return props.layer.name;
  return props.layer.plural;
}

// `ratePerMinute` + the original's under-1 "per hour" fallback.
function rateText(run, amount) {
  const perMin = scaleNum(amount, 60000 / Math.max(run.real_time_ms, 1));
  return numLog10(perMin) < 0
    ? `${formatDecimal(scaleNum(perMin, 60), 2, 2)} per hour`
    : `${formatDecimal(perMin, 2, 2)} per min`;
}

function resourceText(run, index) {
  switch (index) {
    case 0: return `${formatDecimal(run.currency, 2)} ${props.layer.currency}`;
    case 1: return rateText(run, run.currency);
    case 2: return `${formatDecimal(run.count, 2)} ${pluralizeCount(run.count)}`;
    default: return rateText(run, run.count);
  }
}

function infoArray(run, index) {
  let name;
  if (index === 0) name = "Last";
  else if (index === 10) name = "Average";
  else name = `${index + 1} ago`;

  const cells = [name, timeDisplayShort(run.time_ms)];
  if (props.hasRealTime) cells.push(timeDisplayShort(run.real_time_ms));
  cells.push(resourceText(run, selectedResources.value[0]));
  cells.push(resourceText(run, selectedResources.value[1]));
  return cells;
}

const infoCol = computed(() => {
  const cells = ["Run", props.hasRealTime ? "Game Time" : "Time in Run"];
  if (props.hasRealTime) cells.push("Real Time");
  cells.push(...resourceTitles.value);
  return cells;
});

const dropDownIconClass = computed(() =>
  props.shown ? "far fa-minus-square" : "far fa-plus-square",
);

function toggleShown() {
  game.toggleShownRuns(props.layer.key);
}

// The original's fixed column widths, minus the challenge/extra columns.
function cellStyle(col, isHeader) {
  let width;
  switch (col) {
    case 0:
      width = "7rem";
      break;
    case 3:
    case 4:
      width = props.layer.name === "Reality" ? "15rem" : "20rem";
      break;
    default:
      width = "13rem";
  }
  return {
    width,
    border: "0.05rem solid #999999",
    margin: "-0.05rem",
    padding: "0.2rem 0",
    "border-bottom-width": isHeader ? "0.3rem" : "0.1rem",
    "font-weight": isHeader ? "bold" : null,
    color: "var(--color-text)",
  };
}
</script>

<template>
  <div>
    <div
      class="c-past-runs-header"
      @click="toggleShown"
    >
      <span class="o-run-drop-down-icon">
        <i :class="dropDownIconClass" />
      </span>
      <span>
        <h3>Last 10 {{ layer.plural }}:</h3>
      </span>
    </div>
    <div v-show="shown">
      <div class="c-row-container">
        <span
          v-for="(entry, col) in infoCol"
          :key="col"
          :style="cellStyle(col, true)"
        >
          {{ entry }}
        </span>
      </div>
      <div
        v-for="(run, index) in tableRuns"
        :key="index"
      >
        <span
          v-if="run.time_ms === Number.MAX_VALUE"
          class="c-empty-row"
        >
          <i v-if="index === 10">
            An average cannot be calculated with no {{ layer.plural }}.
          </i>
          <i v-else>
            You have not done {{ index + 1 }}
            {{ index === 0 ? layer.name : layer.plural }} yet.
          </i>
        </span>
        <span
          v-else
          class="c-row-container"
        >
          <span
            v-for="(entry, col) in infoArray(run, index)"
            :key="10 * index + col"
            :style="cellStyle(col, false)"
          >
            {{ entry }}
          </span>
        </span>
      </div>
      <br>
    </div>
  </div>
</template>

<style scoped>
.c-row-container {
  display: flex;
  flex-direction: row;
  width: 100%;
}

.c-empty-row {
  display: block;
  border: 0.05rem solid #999999;
  color: var(--color-text);
  width: 100%;
  padding: 0.2rem 0;
  margin: -0.1rem;
}
</style>
