<script setup>
// The Black Hole tab (BlackHoleTab.vue, without the canvas animation and the
// celestial inversion/auto-pause extras): unlock button, per-hole status
// line, the three upgrade buttons per hole, and the pause toggle.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);
const blackHoles = computed(() => reality.value?.black_holes);

const UPGRADES = [
  { kind: 0, name: "Interval", description: "Reduce the time between activations by 20%" },
  { kind: 1, name: "Power", description: "Make the Black Hole 35% more powerful" },
  { kind: 2, name: "Duration", description: "Extend the Black Hole duration by 30%" },
];

function formatSeconds(seconds) {
  if (seconds >= 86400) return `${(seconds / 86400).toFixed(2)} days`;
  if (seconds >= 3600) return `${(seconds / 3600).toFixed(2)} hours`;
  if (seconds >= 60) return `${(seconds / 60).toFixed(2)} minutes`;
  return `${seconds.toFixed(1)} seconds`;
}

function holeStatus(hole) {
  if (blackHoles.value.paused) return "Paused";
  if (hole.is_permanent) return "Permanent";
  if (hole.charged) {
    return `Active (${formatSeconds(Math.max(hole.duration - hole.phase, 0))} left)`;
  }
  return `Inactive (${formatSeconds(Math.max(hole.interval - hole.phase, 0))} until active)`;
}

function holeDescription(hole, index) {
  return (
    `Black Hole ${index + 1}: every ${formatSeconds(hole.interval)}, ` +
    `the game runs ×${hole.power.toFixed(1)} faster for ${formatSeconds(hole.duration)}.`
  );
}
</script>

<template>
  <div
    v-if="blackHoles"
    class="l-black-hole-tab"
  >
    <template v-if="!blackHoles.unlocked">
      <button
        class="c-reality-upgrade-btn l-black-hole-unlock"
        :class="{ 'c-reality-upgrade-btn--unavailable': !blackHoles.can_unlock }"
        @click="game.unlockBlackHole()"
      >
        <b>Unlock the Black Hole</b>
        <div>Cost: 100 RM</div>
      </button>
      <div class="c-black-hole-description">
        The Black Hole periodically makes the whole game run significantly faster.
      </div>
    </template>
    <template v-else>
      <button
        class="o-primary-btn l-black-hole-pause"
        @click="game.toggleBlackHolePause()"
      >
        {{ blackHoles.paused ? "Unpause" : "Pause" }} Black Hole
      </button>
      <div
        v-for="(hole, index) in blackHoles.holes"
        :key="index"
      >
        <div
          v-if="hole.unlocked"
          class="c-black-hole-status"
        >
          <div class="c-black-hole-description">
            <b>{{ holeDescription(hole, index) }}</b>
            <div>
              Status: {{ holeStatus(hole) }} —
              activated {{ hole.activations }} {{ hole.activations === 1 ? "time" : "times" }}
            </div>
          </div>
          <div class="l-black-hole-upgrade-row">
            <button
              v-for="upgrade in UPGRADES"
              :key="upgrade.kind"
              class="c-reality-upgrade-btn c-black-hole-upgrade"
              :class="{
                'c-reality-upgrade-btn--unavailable': !hole.can_buy_upgrades[upgrade.kind],
              }"
              @click="game.buyBlackHoleUpgrade(index, upgrade.kind)"
            >
              <b>Black Hole {{ index + 1 }} {{ upgrade.name }}</b>
              <div>{{ upgrade.description }}</div>
              <div>Cost: {{ formatDecimal(hole.upgrade_costs[upgrade.kind], 2, 0) }} RM</div>
            </button>
          </div>
        </div>
      </div>
      <div
        v-if="!blackHoles.holes[1].unlocked"
        class="c-black-hole-description"
      >
        A second Black Hole is unlocked by a Reality Upgrade.
      </div>
    </template>
  </div>
</template>

<style scoped>
.l-black-hole-tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: var(--color-text);
}

.l-black-hole-unlock {
  width: 25rem;
  min-height: 6rem;
  font-size: 1.2rem;
  margin: 1rem;
}

.l-black-hole-pause {
  width: 20rem;
  margin: 1rem;
}

.c-black-hole-description {
  font-size: 1.3rem;
  margin: 1rem;
}

.l-black-hole-upgrade-row {
  display: flex;
  flex-direction: row;
  justify-content: center;
}

.c-black-hole-upgrade {
  width: 20rem;
  min-height: 9rem;
  font-family: Typewriter, serif;
  font-size: 1rem;
  margin: 0.5rem;
  padding: 0.5rem;
}
</style>
