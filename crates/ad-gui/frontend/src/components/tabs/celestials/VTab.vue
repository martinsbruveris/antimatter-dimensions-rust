<script setup>
// V subtab (Feature 7.4). Before V is unlocked it shows the six main-unlock
// progress bars + the "unlock V" button; after, the run button, the Space
// Theorem count, the 9 V-achievement tiles with tier progress, and the ST-gated
// reward list. Reads `snapshot.celestials.v`. The original's hexagonal layout is
// simplified to a grid; the Perk-Point goal reduction spends Perk Points.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";
import {
  V_ACHIEVEMENTS,
  V_MAIN_UNLOCK_LABELS,
  V_REWARD_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const v = computed(() => game.snapshot?.celestials?.v);

const isVUnlocked = computed(() => Boolean(v.value?.is_v_unlocked));
const isRunning = computed(() => Boolean(v.value?.is_running));
const SYMBOL = "⌬";

function achName(id) {
  return V_ACHIEVEMENTS[id]?.name ?? "";
}
function rewardDescription(id) {
  return V_REWARD_DESCRIPTIONS[id] ?? "";
}

// Format an achievement's raw measured value per its type.
function formatAchValue(id, value) {
  const type = V_ACHIEVEMENTS[id]?.type;
  switch (type) {
    case "negcount":
      return `${Math.round(-value)}`;
    case "pow10":
      return formatDecimal({ m: 1, e: value }, 2);
    case "bh":
      return `1 / ${formatDecimal({ m: 1, e: value }, 2)}`;
    default:
      return `${Math.round(value)}`;
  }
}

function unlockV() {
  game.vUnlockCelestial();
}
function startRun() {
  game.startCelestialReality("v");
}
</script>

<template>
  <div v-if="v" class="l-v-celestial-tab">
    <div class="c-v-info-text">
      You have {{ v.space_theorems }} Space Theorems ({{ v.available_st }} available).
    </div>

    <!-- Pre-unlock: the six main conditions -->
    <template v-if="!isVUnlocked">
      <div class="c-v-info-text">
        Meet all of the following requirements simultaneously to unlock V,
        The Celestial Of Achievements:
      </div>
      <div class="l-v-unlock-conditions">
        <div
          v-for="(label, i) in V_MAIN_UNLOCK_LABELS"
          :key="i"
          class="c-v-unlock"
        >
          <div class="c-v-unlock__label">{{ label }}</div>
          <div class="c-v-unlock-bar">
            <div
              class="c-v-unlock-bar__fill"
              :style="{ width: `${Math.min(100, v.main_progress[i] * 100).toFixed(1)}%` }"
            />
          </div>
        </div>
      </div>
      <button
        class="c-v-unlock-button"
        :class="{ 'c-v-unlock-button--enabled': v.can_unlock }"
        :disabled="!v.can_unlock"
        @click="unlockV"
      >
        Unlock V
      </button>
    </template>

    <!-- Post-unlock: run + achievements + rewards -->
    <template v-else>
      <div class="l-v-run-row">
        <div class="c-v-info-text">Enter V's Reality.</div>
        <div
          class="c-v-run-button c-celestial-run-button--clickable"
          :class="{ 'c-v-run-button--running': isRunning }"
          @click="startRun"
        >
          {{ SYMBOL }}
        </div>
        <div class="c-v-info-text">
          All Dimension multipliers, EP, IP, and DT/sec are square-rooted;
          the Replicanti interval is squared.
        </div>
      </div>

      <div class="l-v-achievement-grid">
        <div
          v-for="ach in v.achievements"
          :key="ach.id"
          class="c-v-achievement"
          :class="{
            'c-v-achievement--complete': ach.completions >= ach.tiers,
            'c-v-achievement--hard': ach.is_hard,
          }"
        >
          <div class="c-v-achievement__name">{{ achName(ach.id) }}</div>
          <div class="c-v-achievement__tier">
            Tier {{ ach.completions }} / {{ ach.tiers }}
          </div>
          <div class="c-v-achievement__progress">
            Current: {{ formatAchValue(ach.id, ach.current_value) }}
            <br>
            Next: {{ ach.completions < ach.tiers
              ? formatAchValue(ach.id, ach.next_goal) : "—" }}
          </div>
          <button
            v-if="v.shard_reduction_unlocked && ach.can_reduce"
            class="c-v-reduce-btn"
            :disabled="v.perk_points < ach.reduction_cost"
            :title="`Spend ${Math.round(ach.reduction_cost)} Perk Points to reduce this goal`"
            @click="game.vReduceGoal(ach.id)"
          >
            Reduce goal ({{ Math.round(ach.reduction_cost) }} PP)
          </button>
        </div>
      </div>

      <div class="l-v-unlocks-container">
        <div
          v-for="reward in v.rewards"
          :key="reward.id"
          class="c-v-reward"
          :class="{ 'c-v-reward--unlocked': reward.unlocked }"
        >
          <b>{{ reward.st_required }} Space Theorems:</b>
          {{ rewardDescription(reward.id) }}
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.l-v-unlock-conditions {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin: 1rem auto;
  width: 40rem;
}
.c-v-unlock__label {
  margin-bottom: 0.2rem;
}
.c-v-unlock-bar {
  height: 1.2rem;
  background: #333;
  border-radius: 0.3rem;
  overflow: hidden;
}
.c-v-unlock-bar__fill {
  height: 100%;
  background: var(--color-v--base, #ead584);
  transition: width 0.2s;
}
.c-v-unlock-button {
  margin: 1rem auto;
  padding: 0.5rem 1.5rem;
  cursor: default;
  opacity: 0.6;
}
.c-v-unlock-button--enabled {
  cursor: pointer;
  opacity: 1;
  background: var(--color-v--base, #ead584);
  color: black;
}
.l-v-run-row {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
  margin: 1rem 0;
}
.l-v-achievement-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.8rem;
  margin: 1rem auto;
  max-width: 70rem;
}
.c-v-achievement {
  border: 0.2rem solid var(--color-v--base, #ead584);
  border-radius: 0.5rem;
  padding: 0.6rem;
}
.c-v-achievement--complete {
  background: rgba(234, 213, 132, 0.2);
}
.c-v-achievement--hard {
  opacity: 0.75;
  border-style: dashed;
}
.c-v-achievement__name {
  font-weight: bold;
}
.c-v-achievement__tier {
  margin: 0.3rem 0;
}
.c-v-achievement__progress {
  font-size: 0.9rem;
}
.l-v-unlocks-container {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin: 1rem auto;
  max-width: 60rem;
}
.c-v-reward {
  opacity: 0.6;
}
.c-v-reward--unlocked {
  opacity: 1;
}
.c-v-reduce-btn {
  margin-top: 0.3rem;
  font-size: 1rem;
  background: transparent;
  color: inherit;
  border: 0.1rem solid currentcolor;
  border-radius: 0.3rem;
  cursor: pointer;
}

.c-v-reduce-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
