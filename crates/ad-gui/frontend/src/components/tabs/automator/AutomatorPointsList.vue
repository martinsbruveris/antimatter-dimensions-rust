<script setup>
// The locked Automator tab: AP progress + every source (vendored from
// AutomatorPointsList.vue). The engine ships ids/AP/bought; labels and copy
// come from the frontend data tables.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { PERKS } from "../../../data/perks";
import { REALITY_UPGRADES } from "../../../data/realityUpgrades";
import {
  PERK_AP_DESCRIPTIONS,
  UPGRADE_AP_DESCRIPTIONS,
  OTHER_AP_SOURCES,
} from "../../../data/automatorPoints";

const game = useGameStore();
const points = computed(() => game.snapshot.automator.points);
const intervalMs = computed(() => game.snapshot.automator.interval_ms);

const perkLabel = (id) => PERKS.find((p) => p.id === id)?.label ?? `#${id}`;
const upgradeName = (id) =>
  REALITY_UPGRADES.find((u) => u.id === id)?.name ?? `Upgrade ${id}`;

function textColor(bought) {
  return { color: bought ? "var(--color-good)" : "var(--color-bad)" };
}

const commandsPerSecond = computed(() =>
  (1000 / intervalMs.value).toFixed(2),
);
</script>

<template>
  <div v-if="points">
    <div class="l-header">
      You have {{ points.total }} / {{ points.threshold }}
      Automator Points towards unlocking the Automator.
      <br>
      You gain Automator Points from the following sources:
    </div>
    <div class="l-automator-points-list-container">
      <div class="l-automator-points-list-side-col c-automator-points-list-col">
        <span class="c-automator-points-list-symbol fas fa-project-diagram" />
        <span class="c-automator-points-list-ap--large">{{ points.from_perks }} AP</span>
        <span class="l-large-text">
          Perks
        </span>
        <div
          v-for="perk in points.perks"
          :key="perk.id"
          class="c-automator-points-list-single-entry"
          :style="textColor(perk.bought)"
        >
          <span class="c-automator-points-list-perk-label">{{ perkLabel(perk.id) }}</span>
          - {{ PERK_AP_DESCRIPTIONS[perk.id] }}
          <span class="c-automator-points-list-ap">{{ perk.ap }} AP</span>
        </div>
      </div>
      <div class="l-automator-points-list-center-col">
        <div
          v-for="source in points.other"
          :key="source.name"
          class="c-automator-points-list-cell"
        >
          <span class="c-automator-points-list-ap--large">{{ source.ap }} AP</span>
          <span class="l-large-text">
            {{ source.name }}
          </span>
          <br>
          <br>
          <span :style="textColor(source.ap > 0)">
            {{ OTHER_AP_SOURCES[source.name]?.description }}
          </span>
          <span
            v-if="OTHER_AP_SOURCES[source.name]?.isIcon"
            class="c-automator-points-list-symbol"
            :class="OTHER_AP_SOURCES[source.name].symbol"
          />
          <span
            v-else
            class="c-automator-points-list-symbol"
          >{{ OTHER_AP_SOURCES[source.name]?.symbol }}</span>
        </div>
      </div>
      <div class="l-automator-points-list-side-col c-automator-points-list-col">
        <span class="c-automator-points-list-symbol fas fa-arrow-up" />
        <span class="c-automator-points-list-ap--large">{{ points.from_upgrades }} AP</span>
        <span class="l-large-text">
          Reality Upgrades
        </span>
        <div
          v-for="upgrade in points.upgrades"
          :key="upgrade.id"
          class="c-automator-points-list-single-entry l-upgrade-list"
          :style="textColor(upgrade.bought)"
        >
          <b>{{ upgradeName(upgrade.id) }}</b>
          <span class="c-automator-points-list-ap">{{ upgrade.ap }} AP</span>
          <br>
          {{ UPGRADE_AP_DESCRIPTIONS[upgrade.id] }}
        </div>
      </div>
    </div>
    <br>
    <div>
      The Automator allows (amongst other things) buying full Time Study Trees, entering Eternity Challenges,
      or starting Dilation.
      <br>
      It can also force prestige events on certain conditions independently from your Autobuyers or modify
      some of your Autobuyer settings.
      <br>
      The speed of the Automator gradually increases as you get more Realities. If unlocked right now,
      it would run {{ commandsPerSecond }} commands per real-time second.
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorPointsList.vue scoped style. */
.l-automator-points-list-container {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  margin-top: 1rem;
  -webkit-user-select: none;
  user-select: none;
}

.c-automator-points-list-col {
  position: relative;
  text-align: left;
  border: var(--var-border-width, 0.15rem) solid var(--color-text);
  border-radius: var(--var-border-radius, 0.5rem);
  padding: 1rem;
}

.l-automator-points-list-side-col {
  display: flex;
  flex-direction: column;
  width: 35%;
  justify-content: space-between;
}

.l-automator-points-list-center-col {
  display: flex;
  flex-direction: column;
  width: 25%;
  justify-content: space-between;
}

.c-automator-points-list-cell {
  overflow: hidden;
  width: 100%;
  height: 48%;
  position: relative;
  text-align: left;
  border: var(--var-border-width, 0.15rem) solid var(--color-text);
  border-radius: var(--var-border-radius, 0.5rem);
  padding: 1rem;
}

.c-automator-points-list-symbol {
  display: flex;
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  justify-content: center;
  align-items: center;
  font-size: 15rem;
  opacity: 0.2;
  text-shadow: 0 0 2rem;
  pointer-events: none;
}

.c-automator-points-list-perk-label {
  display: inline-block;
  width: 3rem;
  max-width: 3rem;
  font-weight: bold;
}

.c-automator-points-list-single-entry {
  position: relative;
}

.c-automator-points-list-ap {
  position: absolute;
  right: 0;
  opacity: 0.8;
}

.c-automator-points-list-ap--large {
  position: absolute;
  right: 1rem;
  font-size: 1.8rem;
  opacity: 0.6;
}

.l-header {
  font-size: 2rem;
}

.l-large-text {
  font-size: 1.8rem;
}

.l-upgrade-list {
  font-size: 1.3rem;
}
</style>
