<script setup>
// Effarig subtab (Feature 7.2), a faithful rebuild of EffarigTab.vue without the
// amplification/rarity-boost/cursed-glyph extras: the Relic-Shard shop with the
// unlock buttons, the run button + description, and the three stage rewards.
// Reads `snapshot.celestials.effarig`.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";
import {
  EFFARIG_RUN_DESCRIPTION,
  EFFARIG_STAGE_DESCRIPTIONS,
  EFFARIG_STAGE_LABELS,
  EFFARIG_UNLOCK_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const effarig = computed(() => game.snapshot?.celestials?.effarig);

const runUnlocked = computed(() => Boolean(effarig.value?.run_unlocked));
const isRunning = computed(() => Boolean(effarig.value?.is_running));
const SYMBOL = "Ϙ";

// The shop shows adjuster/glyphFilter/setSaves always, and the run unlock only
// until it's bought (the original hides it once owned).
const shopUnlocks = computed(() =>
  (effarig.value?.shop_unlocks ?? []).filter((u) => u.id !== 3 || !u.unlocked));

function unlockDescription(id) {
  return EFFARIG_UNLOCK_DESCRIPTIONS[id] ?? "";
}
function stageLabel(id) {
  return EFFARIG_STAGE_LABELS[id] ?? "";
}
function stageDescriptions(id) {
  return EFFARIG_STAGE_DESCRIPTIONS[id] ?? [];
}

function buyUnlock(id) {
  game.effarigBuyUnlock(id);
}
function startRun() {
  game.startCelestialReality("effarig");
}
</script>

<template>
  <div v-if="effarig" class="l-teresa-celestial-tab">
    <div class="l-effarig-shop-and-run">
      <div class="l-effarig-shop">
        <div class="c-effarig-relics">
          You have {{ formatDecimal(effarig.relic_shards, 2, 0) }} Relic Shards.
        </div>
        <div class="c-effarig-relic-description">
          You will gain {{ formatDecimal(effarig.shards_gained, 2) }} Relic Shards next Reality.
        </div>
        <div class="c-effarig-relic-description">
          More Eternity Points slightly increases Relic Shards gained.
          More distinct Glyph effects significantly increases Relic Shards gained.
        </div>
        <button
          v-for="unlock in shopUnlocks"
          :key="unlock.id"
          class="c-effarig-shop-button"
          :class="{
            'c-effarig-shop-button--available': unlock.can_buy,
            'c-effarig-shop-button--bought': unlock.unlocked,
          }"
          @click="buyUnlock(unlock.id)"
        >
          {{ unlockDescription(unlock.id) }}
          <br>
          <span v-if="unlock.unlocked">(Unlocked)</span>
          <span v-else>Cost: {{ formatDecimal(unlock.cost, 2, 0) }} Relic Shards</span>
        </button>
      </div>

      <div v-if="runUnlocked" class="l-effarig-run">
        <div class="c-effarig-run-description">Enter Effarig's Reality.</div>
        <div
          class="l-effarig-run-button c-effarig-run-button c-celestial-run-button--clickable"
          :class="isRunning ? 'c-effarig-run-button--running' : 'c-effarig-run-button--not-running'"
          @click="startRun"
        >
          <div
            :class="isRunning
              ? 'c-effarig-run-button__inner--running'
              : 'c-effarig-run-button__inner--not-running'"
          >
            {{ SYMBOL }}
          </div>
        </div>
        <div class="c-effarig-run-description" style="white-space: pre-line">
          {{ EFFARIG_RUN_DESCRIPTION }}
        </div>
        <div
          v-for="stage in effarig.stage_unlocks"
          :key="stage.id"
          class="l-effarig-tab__reward"
        >
          <div class="c-effarig-tab__reward-label">{{ stageLabel(stage.id) }}:</div>
          <div v-if="stage.unlocked" class="l-effarig-tab__reward-descriptions">
            <div
              v-for="(desc, i) in stageDescriptions(stage.id)"
              :key="i"
              class="c-effarig-tab__reward-description"
            >
              <span class="c-effarig-tab__reward-symbol">{{ SYMBOL }}</span>
              <span>{{ desc }}</span>
            </div>
          </div>
          <span v-else class="c-effarig-tab__reward-symbol">?</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.c-effarig-relic-description {
  width: 46rem;
}
</style>
