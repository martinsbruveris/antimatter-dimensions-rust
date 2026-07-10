<script setup>
// The Nameless Ones subtab (Feature 7.3), a faithful rebuild of EnslavedTab.vue:
// game-time storage (charge/discharge + the Ra auto-release toggle), real-time
// storage (store/auto-store, feeding Reality amplification — the amplify button
// itself sits on the Glyphs tab like the original), the two stored-time
// unlocks, and the run button. Hints and the charging sliders are out of
// frontier.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { timeDisplayShort } from "../../../util/format";
import {
  ENSLAVED_RUN_DESCRIPTION,
  ENSLAVED_UNLOCK_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const enslaved = computed(() => game.snapshot?.celestials?.enslaved);

const runUnlocked = computed(() => Boolean(enslaved.value?.run_unlocked));
const isRunning = computed(() => Boolean(enslaved.value?.is_running));
const SYMBOL = "\uf0c1";

function unlockDescription(id) {
  return ENSLAVED_UNLOCK_DESCRIPTIONS[id] ?? "";
}
function toggleStore() {
  game.toggleStoreGameTime();
}
function discharge() {
  game.enslavedRelease();
}
function buyUnlock(id) {
  game.buyEnslavedUnlock(id);
}
function startRun() {
  game.startCelestialReality("enslaved");
}
function toggleStoreReal() {
  game.toggleStoreRealTime();
}
function toggleAutoStoreReal() {
  game.toggleAutoStoreReal();
}
function toggleAutoRelease() {
  game.toggleAutoRelease();
}
</script>

<template>
  <div v-if="enslaved" class="l-enslaved-celestial-tab">
    <div class="l-enslaved-celestial-tab--inner">
      <div class="l-enslaved-run-container">
        <div v-if="runUnlocked" class="c-enslaved-run-button">
          <div class="c-enslaved-run-button__title">The Nameless Ones' Reality</div>
          <div v-if="enslaved.completed"><b>(Completed)</b></div>
          <div
            class="c-enslaved-run-button__icon c-celestial-run-button--clickable"
            :class="{ 'c-enslaved-run-button__icon--running': isRunning }"
            @click="startRun"
          >
            <div class="c-enslaved-run-button__icon__sigil">{{ SYMBOL }}</div>
          </div>
          <div
            v-for="line in ENSLAVED_RUN_DESCRIPTION"
            :key="line"
            class="c-enslaved-run-description-line"
          >
            {{ line }}
          </div>
        </div>
      </div>

      <div class="l-enslaved-upgrades-column">
        <div class="l-enslaved-top-container">
          <div class="l-enslaved-top-container__half">
            While charging, game speed multipliers are disabled, and the lost speed is converted
            into stored game time. Discharging the Black Hole allows you to skip forward in time.
            Stored game time is also used to unlock certain upgrades.
            <button
              class="o-enslaved-mechanic-button"
              :class="{ 'o-enslaved-mechanic-button--storing-time': enslaved.is_storing_game_time }"
              :disabled="!enslaved.can_modify_game_time_storage"
              @click="toggleStore"
            >
              <div class="o-enslaved-stored-time">{{ timeDisplayShort(enslaved.stored) }}</div>
              <div>{{ enslaved.is_storing_game_time ? "Charging Black Hole" : "Charge Black Hole" }}</div>
            </button>
            <button
              class="o-enslaved-mechanic-button"
              :disabled="!enslaved.can_release"
              @click="discharge"
            >
              <span>Discharge Black Hole</span>
            </button>
            <button
              v-if="enslaved.auto_release_available"
              class="o-enslaved-mechanic-button"
              :class="{ 'o-enslaved-mechanic-button--storing-time': enslaved.is_auto_releasing }"
              @click="toggleAutoRelease"
            >
              {{ enslaved.is_auto_releasing ? "Auto-releasing stored time" : "Auto-release stored time" }}
            </button>
          </div>
          <div class="l-enslaved-top-container__half">
            Storing real time completely halts all production, setting game speed to 0.
            You can use stored real time to "amplify" a Reality, simulating repeated
            runs of it (see the Glyphs tab). Amplified Realities give all the rewards
            that normal Realities do.
            <button
              class="o-enslaved-mechanic-button"
              :class="{ 'o-enslaved-mechanic-button--storing-time': enslaved.is_storing_real_time }"
              :disabled="!enslaved.can_modify_real_time_storage"
              @click="toggleStoreReal"
            >
              <div class="o-enslaved-stored-time">{{ timeDisplayShort(enslaved.stored_real) }}</div>
              <div>{{ enslaved.is_storing_real_time ? "Storing real time" : "Store real time" }}</div>
            </button>
            <button
              class="o-enslaved-mechanic-button"
              :class="{ 'o-enslaved-mechanic-button--storing-time': enslaved.auto_store_real }"
              :disabled="!enslaved.can_modify_real_time_storage"
              @click="toggleAutoStoreReal"
            >
              {{ enslaved.auto_store_real ? "Offline time stored" : "Offline time used for production" }}
            </button>
            <div>Efficiency: 70%</div>
            <div>Maximum stored real time: {{ timeDisplayShort(enslaved.stored_real_cap) }}</div>
          </div>
        </div>

        <div class="l-enslaved-shop-container">
          <button
            v-for="unlock in enslaved.unlocks"
            :key="unlock.id"
            class="o-enslaved-shop-button"
            :class="{
              'o-enslaved-shop-button--bought': unlock.owned,
              'o-enslaved-shop-button--available': unlock.can_buy,
            }"
            @click="buyUnlock(unlock.id)"
          >
            {{ unlockDescription(unlock.id) }}
            <div v-if="!unlock.owned">Costs: {{ timeDisplayShort(unlock.price_ms) }}</div>
            <div v-else><b>(Unlocked)</b></div>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.o-enslaved-mechanic-button:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
