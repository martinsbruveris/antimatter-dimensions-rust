<script setup>
// Teresa subtab (Feature 7.1), a faithful rebuild of the original TeresaTab.vue
// without the glyph-set-preview record and the SVG celestial-quote history:
// the pour-RM bar + button, the threshold-unlock markers, the run button, and
// the Perk Shop. Reads `snapshot.celestials.teresa`; pours via a hold loop.
import { computed, onUnmounted, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal, formatMultiplier } from "../../../util/format";
import { floatToNum, numLog10 } from "../../../util/num";
import {
  PERK_SHOP_DESCRIPTIONS,
  TERESA_RUN_DESCRIPTION,
  TERESA_UNLOCK_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const teresa = computed(() => game.snapshot?.celestials?.teresa);

const isRunning = computed(() => Boolean(teresa.value?.is_running));
const runUnlocked = computed(() => Boolean(teresa.value?.run_unlocked));
const shopUnlocked = computed(() => Boolean(teresa.value?.shop_unlocked));
const showRunReward = computed(() => (teresa.value?.run_reward_multiplier ?? 1) > 1);
const hasEpGen = computed(() =>
  Boolean(teresa.value?.unlocks?.find((u) => u.id === 1)?.unlocked));

// Format a plain float as a `×N` multiplier via the WASM formatter.
function xFloat(f) {
  if (!isFinite(f)) return "×Infinite";
  return formatMultiplier(floatToNum(f));
}

const fillPercent = computed(() => `${((teresa.value?.fill ?? 0) * 100).toFixed(2)}%`);
const possibleFillPercent = computed(
  () => `${((teresa.value?.possible_fill ?? 0) * 100).toFixed(2)}%`);
const isCapped = computed(() => (teresa.value?.fill ?? 0) >= 1);

// Marker height for a threshold unlock: `log10(price) / 24`, matching the bar.
function markerHeight(unlock) {
  const pos = Math.max(0, numLog10(unlock.price)) / 24;
  return `calc(${(100 * pos).toFixed(2)}% - 0.1rem)`;
}
function unlockTitle(unlock) {
  return `${formatDecimal(unlock.price, 2, 2)}: ${TERESA_UNLOCK_DESCRIPTIONS[unlock.id]}`;
}

// --- Pouring (hold-to-pour rAF loop) ---
const pouring = ref(false);
let lastTime = 0;
let rafId = 0;
function pourFrame(now) {
  if (!pouring.value) return;
  const diffMs = now - lastTime;
  lastTime = now;
  if (diffMs > 0) game.teresaPourRm(diffMs);
  rafId = requestAnimationFrame(pourFrame);
}
function startPour() {
  if (pouring.value || isCapped.value) return;
  pouring.value = true;
  lastTime = performance.now();
  rafId = requestAnimationFrame(pourFrame);
}
function stopPour() {
  if (!pouring.value) return;
  pouring.value = false;
  cancelAnimationFrame(rafId);
  game.teresaStopPouring();
}
onUnmounted(stopPour);

function startRun() {
  game.startCelestialReality("teresa");
}
function buyPerkShop(id) {
  game.buyPerkShop(id);
}
function perkDescription(id) {
  return PERK_SHOP_DESCRIPTIONS[id] ?? "";
}
</script>

<template>
  <div v-if="teresa" class="l-teresa-celestial-tab">
    <div>You have {{ formatDecimal(teresa.reality_machines, 2, 2) }} Reality Machines.</div>
    <div class="l-mechanics-container">
      <div v-if="runUnlocked" class="l-teresa-mechanic-container">
        <div class="c-teresa-unlock c-teresa-run-button">
          <span>Start Teresa's Reality.</span>
          <div
            class="c-teresa-run-button__icon c-celestial-run-button--clickable"
            :class="{ 'c-teresa-run-button__icon--running': isRunning }"
            @click="startRun()"
          >
            Ϟ
          </div>
          {{ TERESA_RUN_DESCRIPTION }}
          <br><br>
          <div>
            This Reality can be repeated for a stronger reward based on the antimatter gained within it.
            <br><br>
            <span v-if="showRunReward">
              Teresa Reality reward: Glyph Sacrifice power
              {{ xFloat(teresa.run_reward_multiplier) }}
            </span>
            <span v-else>You have not completed Teresa's Reality yet.</span>
          </div>
        </div>
        <div v-if="hasEpGen" class="c-teresa-unlock">
          Every second, you gain 1.00% of your peaked Eternity Points per minute this Reality.
        </div>
      </div>

      <div class="l-rm-container l-teresa-mechanic-container">
        <button
          class="o-teresa-shop-button c-teresa-pour"
          :class="{
            'o-teresa-shop-button--available': !isCapped,
            'o-teresa-shop-button--capped': isCapped,
          }"
          @mousedown="startPour()"
          @touchstart.prevent="startPour()"
          @mouseup="stopPour()"
          @touchend="stopPour()"
          @mouseleave="stopPour()"
        >
          {{ isCapped ? "Filled" : "Pour RM" }}
        </button>
        <div class="c-rm-store">
          <div
            class="c-rm-store-inner c-rm-store-inner--light"
            :style="{ height: possibleFillPercent }"
          />
          <div class="c-rm-store-inner" :style="{ height: fillPercent }">
            <div class="c-rm-store-label">
              {{ xFloat(teresa.rm_multiplier) }} RM gain
              <br>
              {{ formatDecimal(teresa.poured_amount, 2, 2) }} / {{ formatDecimal({ m: 1, e: 24 }, 2, 2) }}
            </div>
          </div>
          <div
            v-for="unlock in teresa.unlocks"
            :key="unlock.id"
            class="c-teresa-milestone-line"
            :class="{ 'c-teresa-milestone-line--unlocked': unlock.unlocked }"
            :style="{ bottom: markerHeight(unlock) }"
            :title="unlockTitle(unlock)"
          />
        </div>
      </div>

      <div v-if="shopUnlocked" class="c-teresa-shop">
        <span class="o-teresa-pp">
          You have {{ formatDecimal(floatToNum(teresa.perk_points), 2, 0) }} Perk Points.
        </span>
        <div
          v-for="upg in teresa.perk_shop"
          :key="upg.id"
          class="l-spoon-btn-group"
        >
          <button
            class="o-teresa-shop-button"
            :class="{
              'o-teresa-shop-button--available': upg.can_buy,
              'o-teresa-shop-button--capped': upg.capped,
            }"
            @click="buyPerkShop(upg.id)"
          >
            {{ perkDescription(upg.id) }}
            <br>
            Currently: {{ formatMultiplier(upg.effect) }}
            <br>
            <span v-if="!upg.capped">
              Cost: {{ formatDecimal(upg.cost, 2, 0) }} Perk Points
            </span>
          </button>
        </div>
      </div>
      <div v-else class="l-rm-container-labels l-teresa-mechanic-container" />
    </div>
  </div>
</template>

<style scoped>
.c-teresa-milestone-line {
  position: absolute;
  right: 0;
}
</style>
