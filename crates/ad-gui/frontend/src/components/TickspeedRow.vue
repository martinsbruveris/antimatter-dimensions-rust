<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";

const game = useGameStore();
const s = computed(() => game.snapshot);
</script>

<template>
  <div class="l-tickspeed-container">
    <div class="tickspeed-buttons">
      <button
        class="o-primary-btn tickspeed-btn"
        :class="{ 'o-primary-btn--disabled': !s.can_buy_tickspeed }"
        @click="game.buyTickspeed()"
      >
        Tickspeed Cost: {{ s.tickspeed_cost }}
      </button>
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

.tickspeed-max-btn {
  margin-left: 0.5rem;
  width: 10rem;
  height: 2.5rem;
  padding: 0.25rem;
}
</style>
