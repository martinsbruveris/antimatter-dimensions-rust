<script setup>
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatMultiplier } from "../../util/format";
import { NORMAL_ACHIEVEMENTS } from "../../data/achievements";

const game = useGameStore();

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

// Global achievement-power multiplier (1.25^rows × 1.03^count), shown as the
// boost the tab grants to all Antimatter Dimensions.
const powerText = computed(() =>
  game.snapshot ? formatMultiplier(game.snapshot.achievement_power) : "×1.00"
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
</script>

<template>
  <div class="l-achievements-tab">
    <div class="c-achievements-tab__header">
      Achievements with a <i class="fas fa-star" /> icon also give an additional reward.
    </div>
    <div class="c-achievements-tab__header">
      Multiplier to all Antimatter Dimensions from Achievements: {{ powerText }}
    </div>
    <div class="l-achievement-grid">
      <div
        v-for="(row, i) in ROWS"
        :key="i"
        class="l-achievement-grid__row"
      >
        <div
          v-for="ach in row"
          :key="ach.id"
          class="l-achievement-grid__cell o-achievement o-achievement--normal"
          :class="
            unlocked.has(ach.id)
              ? 'o-achievement--unlocked'
              : 'o-achievement--locked'
          "
          :style="spriteStyle(ach.id)"
        >
          <div class="o-achievement__tooltip">
            <div class="o-achievement__tooltip__name">{{ ach.name }} ({{ ach.id }})</div>
            <div class="o-achievement__tooltip__description">{{ ach.description }}</div>
            <div
              v-if="ach.reward"
              class="o-achievement__tooltip__reward"
            >
              Reward: {{ ach.reward }}
            </div>
          </div>
          <div
            v-if="ach.reward"
            class="o-achievement__reward"
          >
            <i class="fas fa-star" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* These two rules live in the original NormalAchievement.vue's scoped styles
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
</style>
