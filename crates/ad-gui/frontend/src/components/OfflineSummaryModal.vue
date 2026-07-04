<script setup>
// Catch-up summary shown after Offline mode replays >= 10 s of accumulated
// game-time. Mirrors the original AwayProgressModal.vue + AwayProgressEntry
// (../antimatter-dimensions/src/components/modals): a "While you were away
// for {time}:" header followed by one line per resource that increased, in
// the "increased from {before} to {after}" wording. Which resources may show
// is the Away Progress option set (Visual tab modal); clicking a line
// strikes it through and turns its option off for future summaries.
import { computed, reactive } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { formatDecimal, formatTime } from "../util/format";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();

const summary = computed(() => ui.offlineSummary);

// The modelled away-progress resources (original AwayProgressTypes subset):
// option key (camelCase, for set_away_progress), the snapshot option field,
// value getter, display name, and the per-resource colour class.
const ENTRIES = [
  {
    kind: "antimatter",
    option: "antimatter",
    name: "Antimatter",
    value: (s) => s.antimatter,
    cssClass: "c-modal-away-progress__antimatter",
  },
  {
    kind: "dimensionBoosts",
    option: "dimension_boosts",
    name: "Dimension Boosts",
    value: (s) => s.dim_boosts,
    cssClass: "c-modal-away-progress__dimension-boosts",
  },
  {
    kind: "antimatterGalaxies",
    option: "antimatter_galaxies",
    name: "Antimatter Galaxies",
    value: (s) => s.galaxies,
    cssClass: "c-modal-away-progress__antimatter-galaxies",
  },
  {
    kind: "infinities",
    option: "infinities",
    name: "Infinities",
    value: (s) => s.infinities,
    cssClass: "c-modal-away-progress__infinities",
  },
  {
    kind: "infinityPoints",
    option: "infinity_points",
    name: "Infinity Points",
    value: (s) => s.infinity_points,
    cssClass: "c-modal-away-progress__infinity-points",
  },
  {
    kind: "replicanti",
    option: "replicanti",
    name: "Replicanti",
    value: (s) => s.replicanti.amount,
    cssClass: "c-modal-away-progress__replicanti",
  },
  {
    kind: "replicantiGalaxies",
    option: "replicanti_galaxies",
    name: "Replicanti Galaxies",
    value: (s) => s.replicanti.galaxies,
    cssClass: "c-modal-away-progress__replicanti-galaxies",
  },
];

// A resource value is either a plain number (boost/galaxy counts) or a
// snapshot Num ({ m, e }); these helpers convert between the two. Exponents
// past float range only ever get compared, so Infinity is fine there.
function asFloat(v) {
  if (typeof v === "number") return v;
  if (v.e > 300) return Infinity;
  return v.m * 10 ** v.e;
}

function asNum(v) {
  if (typeof v !== "number") return v;
  if (v === 0) return { m: 0, e: 0 };
  const e = Math.floor(Math.log10(Math.abs(v)));
  return { m: v / 10 ** e, e };
}

// The original's formatPseudo: integers below 1e9 for readability, full
// 2-decimal notation formatting above.
function formatPseudo(v) {
  const n = asFloat(v);
  if (n < 1e9) return formatDecimal(asNum(Math.floor(n)), 0, 0);
  return formatDecimal(asNum(v), 2, 2);
}

// Lines struck through by a click this modal-opening (the option itself is
// flipped too, but the line stays visible so the click can be undone).
const removed = reactive(new Set());

// Rows to render: the resource increased (visibly — same formatted string
// counts as unchanged, like the original) and its option was on when the
// replay ran. Option state is read from the *before* snapshot so a mid-modal
// toggle doesn't yank rows; new state applies to the next summary.
const rows = computed(() => {
  const s = summary.value;
  if (!s) return [];
  const options = s.before.options?.away_progress ?? {};
  return ENTRIES.flatMap((entry) => {
    if (!(options[entry.option] ?? true)) return [];
    const before = entry.value(s.before);
    const after = entry.value(s.after);
    const beforeText = formatPseudo(before);
    const afterText = formatPseudo(after);
    if (beforeText === afterText || !(asFloat(after) > asFloat(before))) {
      return [];
    }
    return [{ ...entry, beforeText, afterText }];
  });
});

// Click a line to hide that resource from future summaries (and back).
function toggleEntry(row) {
  const nowRemoved = !removed.has(row.kind);
  if (nowRemoved) removed.add(row.kind);
  else removed.delete(row.kind);
  game.setAwayProgress(row.kind, !nowRemoved);
}

const headerText = computed(() => {
  const s = summary.value;
  if (!s) return "";
  const time = formatTime(s.seconds * 1000);
  return rows.value.length > 0
    ? `While you were away for ${time}: `
    : `While you were away for ${time}... Nothing happened.`;
});
</script>

<template>
  <Modal
    compact
    fit-content
    @close="$emit('close')"
  >
    <div
      v-if="summary"
      class="c-modal-away-progress"
    >
      <div class="c-modal-away-progress__header">
        {{ headerText }}
      </div>
      <div
        v-if="rows.length > 0"
        class="c-modal-away-progress__resources"
      >
        <div
          v-for="row in rows"
          :key="row.kind"
          :class="removed.has(row.kind)
            ? 'c-modal-away-progress__disabled'
            : row.cssClass"
          @click="toggleEntry(row)"
        >
          <b>{{ row.name }}</b>
          increased from
          {{ row.beforeText }} to {{ row.afterText }}
        </div>
      </div>
      <span v-if="rows.length > 0">Note: Click an entry to hide it in the future.</span>
    </div>
  </Modal>
</template>

<style scoped>
/* The original keeps the resource rows in a component-scoped block (not the
   vendored stylesheet); reproduce its underlined-row look. The vendored
   `.c-modal` centers text, but our Modal shell's `.c-modal-text` resets it to
   left — restore the original's centering here. */
.c-modal-away-progress {
  text-align: center;
}

.c-modal-away-progress__resources div {
  min-width: 35rem;
  border-bottom: 0.1rem solid var(--color-text, #cccccc);
  margin-bottom: 0.2rem;
  padding-bottom: 0.2rem;
  cursor: pointer;
}

.c-modal-away-progress__resources div:last-child {
  border: none;
}

/* Per-resource coloring from the original AwayProgressEntry's scoped styles
   (antimatter-family red + the dark-theme glow, infinity purple, replicanti
   blue, and the struck-through disabled state). */
.c-modal-away-progress__dimension-boosts,
.c-modal-away-progress__antimatter-galaxies,
.c-modal-away-progress__antimatter {
  color: var(--color-antimatter);
}

.t-dark .c-modal-away-progress__antimatter {
  animation: a-game-header__antimatter--glow 25s infinite;
}

.c-modal-away-progress__infinities,
.c-modal-away-progress__infinity-points {
  color: var(--color-infinity);
}

.c-modal-away-progress__replicanti-galaxies,
.c-modal-away-progress__replicanti {
  color: #03a9f4;
}

.c-modal-away-progress__disabled b,
.c-modal-away-progress__disabled {
  font-style: italic;
  color: #303030;
  text-shadow: 0 0 0.3rem #303030;
  text-decoration: line-through;
  animation: none;
}
</style>
