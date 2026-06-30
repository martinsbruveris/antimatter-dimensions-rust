<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { formatDecimal } from "../util/format";
import { TUTORIAL_STATE, hasTutorial } from "../util/tutorial";
import GeneralTooltip from "./GeneralTooltip.vue";

const game = useGameStore();
const s = computed(() => game.snapshot);

// Tutorial highlight for the Tickspeed buy button (the TICKSPEED step).
const hasTut = computed(() => hasTutorial(s.value, TUTORIAL_STATE.TICKSPEED));

// Mirrors the original `upgradeCount` (pre-Infinity, no free upgrades):
// `quantifyInt("Purchased Upgrade", totalTickBought)`.
const tickspeedTooltip = computed(() => {
  const n = s.value.tickspeed_bought;
  return `${n.toLocaleString("en-US")} Purchased Upgrade${n === 1 ? "" : "s"}`;
});
</script>

<template>
  <div
    class="l-tickspeed-container"
    :class="{ 'l-tickspeed-container--hidden': !s.tickspeed_unlocked }"
  >
    <div class="tickspeed-buttons">
      <GeneralTooltip :text="tickspeedTooltip">
        <button
          class="o-primary-btn tickspeed-btn"
          :class="{
            'o-primary-btn--disabled': !s.can_buy_tickspeed,
            'tutorial--glow': hasTut && s.can_buy_tickspeed,
          }"
          @click="game.buyTickspeed()"
        >
          Tickspeed Cost: {{ formatDecimal(s.tickspeed_cost, 0) }}
          <div
            v-if="hasTut"
            class="fas fa-circle-exclamation l-notification-icon"
          />
        </button>
      </GeneralTooltip>
      <button
        class="o-primary-btn tickspeed-max-btn"
        :class="{ 'o-primary-btn--disabled': !s.can_buy_tickspeed }"
        @click="game.buyMaxTickspeed()"
      >
        Buy Max
      </button>
    </div>
  </div>
</template>

<style scoped>
/* From TickspeedRow.vue scoped style (not in the global vendored CSS). */
.o-primary-btn {
  position: relative;
  vertical-align: middle;
}

.tickspeed-btn {
  position: relative;
  width: 30rem;
  height: 2.5rem;
  padding: 0.25rem;
}

.l-tickspeed-container {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding-top: 0.5rem;
}

/* Hidden before the first 2nd-dimension purchase. `visibility: hidden` (not
   `display: none`) so the row still reserves its space and the layout doesn't
   jump when tickspeed appears — matching the original TickspeedRow.vue. */
.l-tickspeed-container--hidden {
  visibility: hidden;
}

.tickspeed-max-btn {
  margin-left: 0.5rem;
  width: 10rem;
  height: 2.5rem;
  padding: 0.25rem;
}
</style>
