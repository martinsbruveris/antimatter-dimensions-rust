<script setup>
import { NORMAL_ACHIEVEMENTS } from "../../data/achievements";

// Group achievements into rows of 8 by id (id = row*10 + column), as the
// original does. All shown locked (grey) for now — no unlock state from
// the engine yet.
const byRow = {};
for (const ach of NORMAL_ACHIEVEMENTS) {
  const row = Math.floor(ach.id / 10);
  (byRow[row] ??= []).push(ach);
}
const ROWS = Object.keys(byRow)
  .map(Number)
  .sort((a, b) => a - b)
  .map((r) => byRow[r].sort((a, b) => a.id - b.id));
</script>

<template>
  <div class="l-achievements-tab">
    <div class="c-achievements-tab__header">
      Achievements with a <i class="fas fa-star" /> icon also give an additional reward.
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
          class="l-achievement-grid__cell o-achievement o-achievement--locked"
        >
          <span class="c-ach-id">{{ ach.id }}</span>
          <div class="o-achievement__tooltip">
            <div class="o-achievement__tooltip__name">{{ ach.name }} ({{ ach.id }})</div>
            <div class="o-achievement__tooltip__description">{{ ach.description }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* The achievement id, centered in the tile (the original shows it as
   small hint text over a sprite; locked tiles have no sprite, so we just
   center the id). */
.c-ach-id {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 1.6rem;
  color: black;
}
</style>
