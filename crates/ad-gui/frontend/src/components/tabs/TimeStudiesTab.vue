<script setup>
// The Time Studies tree. Mirrors the original TimeStudiesTab.vue +
// time-study-tree-layout.js (NORMAL layout) + TimeStudyButton.vue: the TT
// header with the three buy buttons and respec toggle, absolutely-positioned
// study buttons (vendored time-studies.css path colors), and the SVG
// connection lines. EC nodes render as locked placeholders until Feature 4.5.
import { computed, ref } from "vue";

import Modal from "../Modal.vue";
import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { formatDecimal } from "../../util/format";
import TimeStudyPresetButton from "./time-studies/TimeStudyPresetButton.vue";
import {
  TIME_STUDY_DESCRIPTIONS,
  DILATION_STUDY_DESCRIPTIONS,
  TREE_ROWS,
  TREE_CONNECTIONS,
  studyPath,
} from "../../data/timeStudies";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);
const ts = computed(() => s.value?.time_studies);

// --- Study presets (edit / delete modals; the button row lives in the TT
// header). Mirrors the original's StudyStringModal in both modes. -----------
const editSlot = ref(null); // 1-based slot being edited, null = closed
const editText = ref("");
const editInvalid = ref(false);
const deleteSlot = ref(null); // 1-based slot pending delete confirmation

function openEdit(saveslot) {
  editSlot.value = saveslot;
  editText.value = ts.value.presets[saveslot - 1].studies;
  editInvalid.value = false;
}

async function confirmEdit() {
  const ok = await game.studyPresetEdit(editSlot.value - 1, editText.value);
  if (ok) {
    editSlot.value = null;
  } else {
    editInvalid.value = true;
  }
}

async function confirmDelete() {
  await game.studyPresetEdit(deleteSlot.value - 1, "");
  ui.notify(`Study preset in slot ${deleteSlot.value} deleted`, "eternity");
  deleteSlot.value = null;
}

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

const ecById = computed(() => {
  const map = new Map();
  for (const ec of s.value?.eternity_challenges ?? []) map.set(ec.id, ec);
  return map;
});

const dilationStudyById = computed(() => {
  const map = new Map();
  for (const ds of s.value?.dilation?.studies ?? []) map.set(ds.id, ds);
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
    } else if (item.startsWith("D")) {
      // Dilation study (D1 = unlock dilation, D2–D5 = TD5–8).
      const dsId = Number(item.slice(1));
      const ds = dilationStudyById.value.get(dsId) ?? {};
      const state = ds.is_bought
        ? "bought"
        : ds.can_buy
          ? "available"
          : "unavailable";
      list.push({
        key: item,
        id: null,
        dsId,
        pos,
        classes: [
          "o-time-study",
          "l-time-study",
          `o-time-study--${state}`,
          `o-time-study-dilation--${state}`,
        ],
        description: DILATION_STUDY_DESCRIPTIONS[dsId] ?? "",
        cost: ds.cost ?? null,
        clickable: Boolean(ds.can_buy),
        dilation: true,
      });
    } else {
      // EC study slot: buy the unlock study, then click again to start the
      // challenge (the original's double-click flow, simplified).
      const ecId = Number(item.slice(2));
      const ec = ecById.value.get(ecId) ?? {};
      const state = ec.is_running
        ? "bought"
        : ec.is_unlocked
          ? "bought"
          : ec.can_unlock
            ? "available"
            : "unavailable";
      list.push({
        key: item,
        id: null,
        ecId,
        pos,
        classes: [
          "o-time-study",
          "l-time-study",
          `o-time-study--${state}`,
          `o-time-study-eternity-challenge--${state}`,
          ec.is_running ? "o-time-study-eternity-challenge--running" : "",
        ],
        description: `Eternity Challenge ${ecId}`,
        cost: ec.study_cost ?? null,
        completions: ec.completions ?? 0,
        clickable: Boolean(ec.can_unlock || (ec.is_unlocked && !ec.is_running)),
        ec,
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
    const toPath =
      typeof to === "number"
        ? studyPath(to)
        : String(to).startsWith("D")
          ? "dilation"
          : "eternity-challenge";
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
  if (!node.clickable) return;
  if (node.id !== null) {
    game.buyTimeStudy(node.id);
    return;
  }
  if (node.dilation) {
    game.buyDilationStudy(node.dsId);
    return;
  }
  // EC node: buy the study first, then a further click starts the challenge.
  if (node.ec.can_unlock) game.buyEcStudy(node.ecId);
  else if (node.ec.is_unlocked && !node.ec.is_running)
    game.startEternityChallenge(node.ecId);
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
    <div class="l-tree-load-button-row">
      <span class="c-ttshop__save-load-text">Load:</span>
      <TimeStudyPresetButton
        v-for="saveslot in 6"
        :key="saveslot"
        :saveslot="saveslot"
        @edit="openEdit"
        @delete="deleteSlot = $event"
      />
    </div>
    <Modal
      v-if="editSlot !== null"
      title="Edit Time Study Preset"
      compact
      centered
      @close="editSlot = null"
    >
      <div>
        Your Time Study Preset in slot {{ editSlot }}:
      </div>
      <textarea
        v-model="editText"
        class="c-modal-input c-study-string-modal__input"
        :class="{ 'c-study-string-modal__input--invalid': editInvalid }"
        rows="3"
        @input="editInvalid = false"
      />
      <div
        v-if="editInvalid"
        class="c-study-string-modal__error"
      >
        Not a valid Time Study string
      </div>
      <div class="l-modal-buttons">
        <button
          class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
          @click="editSlot = null"
        >
          Cancel
        </button>
        <button
          class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
          @click="confirmEdit"
        >
          Confirm
        </button>
      </div>
    </Modal>
    <Modal
      v-if="deleteSlot !== null"
      title="Delete Time Study Preset"
      compact
      centered
      @close="deleteSlot = null"
    >
      <div>
        Are you sure you want to delete the Time Study preset in slot
        {{ deleteSlot }}? This cannot be undone!
      </div>
      <div class="l-modal-buttons">
        <button
          class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
          @click="deleteSlot = null"
        >
          Cancel
        </button>
        <button
          class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
          @click="confirmDelete"
        >
          Confirm
        </button>
      </div>
    </Modal>
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
        <template v-if="node.id === null">
          <br>
          Completed {{ node.completions }}/5
          <template v-if="node.ec.is_unlocked && !node.ec.is_running">
            <br>
            Click to start
          </template>
        </template>
        <template v-if="node.cost !== null">
          <br>
          Cost: {{ node.cost }} Time {{ node.cost === 1 ? "Theorem" : "Theorems" }}
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

/* Preset button row (the original places these in the TT shop bar's
   l-tree-load-button-wrapper; our simplified header puts them on their own
   centered row below the buy buttons). */
.l-tree-load-button-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  margin: 0.3rem 0;
}

.c-ttshop__save-load-text {
  font-size: 1.4rem;
  font-weight: bold;
  margin-right: 0.5rem;
}

/* The preset-edit modal input (the original StudyStringModal's input). */
.c-study-string-modal__input {
  width: 40rem;
  max-width: 100%;
  margin-top: 1rem;
  padding: 0.5rem;
}

.c-study-string-modal__input--invalid {
  border: 0.1rem solid var(--color-bad);
}

.c-study-string-modal__error {
  color: var(--color-bad);
  font-weight: bold;
  margin-top: 0.5rem;
}
</style>
