<script setup>
import { computed, onMounted, onUnmounted, ref } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { formatMultiplier } from "../../util/format";
import { randomCrossWords } from "../../util/word-shift";
import { NORMAL_ACHIEVEMENTS } from "../../data/achievements";

const game = useGameStore();
const ui = useUiStore();

// Group achievements into rows of 8 by id (id = row*10 + column), as the
// original does.
const byRow = {};
for (const ach of NORMAL_ACHIEVEMENTS) {
  const row = Math.floor(ach.id / 10);
  (byRow[row] ??= []).push(ach);
}
const ROWS = Object.keys(byRow)
  .map(Number)
  .sort((a, b) => a - b)
  .map((r) => byRow[r].sort((a, b) => a.id - b.id));

// Unlock state comes from the engine snapshot (`unlocked_achievements` is a
// sorted list of ids). A Set keeps the per-tile lookup O(1).
const unlocked = computed(
  () => new Set(game.snapshot?.unlocked_achievements ?? [])
);

// Info-Display options: the tile-corner achievement IDs obey the
// "Achievement IDs" toggle (holding Shift always shows them, mirroring the
// original HintText); the ✓/✗ unlock-state indicators obey their own toggle
// (no Shift override, as in the original).
const showIds = computed(
  () =>
    Boolean(game.snapshot?.options?.show_hint_text?.achievements) ||
    ui.shiftDown
);
const showUnlockState = computed(() =>
  Boolean(game.snapshot?.options?.show_hint_text?.achievement_unlock_states)
);

// Global achievement-power multiplier (1.25^rows × 1.03^count), shown as the
// boost the tab grants to all Antimatter Dimensions.
const powerText = computed(() =>
  game.snapshot ? formatMultiplier(game.snapshot.achievement_power) : "×1.00"
);

// "Hide completed rows" toggle (matches the original's subtab option). A row
// counts as completed once every achievement in it is unlocked. The flag lives
// in the ui store so it persists across tab switches.
function isRowCompleted(row) {
  return row.every((ach) => unlocked.value.has(ach.id));
}

const visibleRows = computed(() =>
  ROWS.filter(
    (row) => !(ui.hideCompletedAchievementRows && isRowCompleted(row))
  )
);

// Sprite cell offset: the sheet is a grid of 104px cells, indexed by the
// achievement's (column-1, row-1) — matching the original NormalAchievement.vue.
function spriteStyle(id) {
  const row = Math.floor(id / 10);
  const column = id % 10;
  return {
    backgroundPosition: `-${(column - 1) * 104}px -${(row - 1) * 104}px`,
  };
}

// --- Obscured last row (Pelle achievements, ids 181-188) ---------------------
// The original renders these as a shimmering, unreadable mystery until the
// player is Doomed. We have no Pelle, so row 18 is permanently obscured: it
// shows the "?" tile (o-achievement--hidden) and garbled id/name/description,
// and reveals no reward. The engine never unlocks these ids anyway.
function isObscured(ach) {
  return Math.floor(ach.id / 10) === 18;
}

// Deterministic, same-length, space-preserving scramble (seeded by char code +
// index), reproducing the original's makeGarbledTemplate. This is the stable
// skeleton the time-varying shimmer is layered on; spaces are kept so the text
// can't word-wrap any worse than the real string. Computed once per tile.
function makeGarbledTemplate(input) {
  const text = `${input}`;
  let garbled = "";
  for (let i = 0; i < text.length; i++) {
    if (text[i] === " ") {
      garbled += " ";
    } else {
      const n = text[i].charCodeAt(0);
      garbled += String.fromCharCode(33 + ((n * n + i * i) % 93));
    }
  }
  return garbled;
}

const garbleTemplates = new Map();
for (const ach of NORMAL_ACHIEVEMENTS) {
  if (isObscured(ach)) {
    garbleTemplates.set(ach.id, {
      id: makeGarbledTemplate(ach.id),
      name: makeGarbledTemplate(ach.name),
      description: makeGarbledTemplate(ach.description),
    });
  }
}

// Bumped on an interval to drive the shimmer (randomCrossWords is time-keyed,
// and re-rolls its random symbols on each call). 250ms matches the original's
// roughly-every-few-frames re-render without being costly for 8 tiles.
const garbleTick = ref(0);
let garbleTimer;
onMounted(() => {
  garbleTimer = setInterval(() => {
    garbleTick.value += 1;
  }, 250);
});
onUnmounted(() => clearInterval(garbleTimer));

// Garble one field of an obscured achievement, re-inserting the template's
// spaces (randomCrossWords preserves length, so indices line up). Reads
// garbleTick so the template re-evaluates each tick, producing the shimmer.
function garble(id, field) {
  void garbleTick.value;
  const template = garbleTemplates.get(id)?.[field] ?? "";
  const raw = randomCrossWords(template);
  let out = "";
  for (let i = 0; i < raw.length; i++) {
    out += template[i] === " " ? " " : raw[i];
  }
  return out;
}

// Per-tile class: obscured tiles use the gray "?" sprite; the rest keep the
// normal sheet plus their unlocked/locked colour.
function cellClass(ach) {
  if (isObscured(ach)) return "o-achievement--hidden";
  return [
    "o-achievement--normal",
    unlocked.value.has(ach.id)
      ? "o-achievement--unlocked"
      : "o-achievement--locked",
  ];
}
</script>

<template>
  <div class="l-achievements-tab">
    <div class="c-subtab-option-container">
      <button
        class="o-primary-btn o-primary-btn--subtab-option"
        @click="ui.hideCompletedAchievementRows = !ui.hideCompletedAchievementRows"
      >
        Hide completed rows: {{ ui.hideCompletedAchievementRows ? "ON" : "OFF" }}
      </button>
    </div>
    <div class="c-achievements-tab__header c-achievements-tab__header--multipliers">
      Achievements provide a multiplier to
      <div>Antimatter Dimensions: ×{{ powerText }}</div>
    </div>
    <div class="c-achievements-tab__header">
      Achievements with a <i class="fas fa-star" /> icon also give an additional reward.
    </div>
    <div class="l-achievement-grid">
      <div
        v-for="row in visibleRows"
        :key="row[0].id"
        class="l-achievement-grid__row"
        :class="{ 'c-achievement-grid__row--completed': isRowCompleted(row) }"
      >
        <div
          v-for="ach in row"
          :key="ach.id"
          class="l-achievement-grid__cell o-achievement"
          :class="cellClass(ach)"
          :style="spriteStyle(ach.id)"
        >
          <div
            v-show="showIds"
            class="o-hint-text l-hint-text l-hint-text--achievement"
          >
            {{ isObscured(ach) ? garble(ach.id, "id") : ach.id }}
          </div>
          <div class="o-achievement__tooltip">
            <div class="o-achievement__tooltip__name">
              {{ isObscured(ach) ? garble(ach.id, "name") : ach.name }}
              ({{ isObscured(ach) ? garble(ach.id, "id") : ach.id }})
            </div>
            <div class="o-achievement__tooltip__description">
              {{ isObscured(ach) ? garble(ach.id, "description") : ach.description }}
            </div>
            <div
              v-if="ach.reward && !isObscured(ach)"
              class="o-achievement__tooltip__reward"
            >
              Reward: {{ ach.reward }}
            </div>
          </div>
          <div
            v-if="ach.reward && !isObscured(ach)"
            class="o-achievement__reward"
            :class="{ 'o-achievement__reward--locked': !unlocked.has(ach.id) }"
          >
            <i class="fas fa-star" />
          </div>
          <div
            v-if="showUnlockState"
            class="o-achievement__indicator"
            :class="{ 'o-achievement__indicator--locked': !unlocked.has(ach.id) }"
          >
            <i :class="unlocked.has(ach.id) ? 'fas fa-check' : 'fas fa-times'" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* These rules live in the original NormalAchievement.vue's scoped styles
   (not the vendored global sheet), so they are reproduced here verbatim. */
.o-achievement--locked {
  background-color: #a3a3a3;
  border-color: var(--color-bad);
}

.o-achievement__reward {
  width: 1.5rem;
  height: 1.5rem;
  position: absolute;
  left: 0;
  bottom: 0;
  font-size: 1rem;
  color: black;
  background: #5ac467;
  border-top: var(--var-border-width, 0.2rem) solid #127a20;
  border-right: var(--var-border-width, 0.2rem) solid #127a20;
  border-top-right-radius: var(--var-border-radius, 0.8rem);
  border-bottom-left-radius: var(--var-border-radius, 0.6rem);
}

.o-achievement__reward--locked {
  background: #a3a3a3;
  border-color: var(--color-bad);
}
</style>
