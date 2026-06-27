<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerBox from "./AutobuyerBox.vue";

const game = useGameStore();
const entry = computed(() => game.snapshot.autobuyers.tickspeed);
</script>

<template>
  <AutobuyerBox
    :entry="entry"
    @unlock="game.unlockTickspeedAutobuyer()"
    @toggle="game.toggleTickspeedAutobuyer()"
  >
    <template #intervalSlot>
      <button
        class="o-autobuyer-btn l-autobuyer-box__button o-autobuyer-btn--unavailable"
      >
        Complete the challenge to upgrade interval
      </button>
    </template>
    <template #toggleSlot>
      <!-- Pre-Infinity the tickspeed autobuyer is locked to "Buys singles". -->
      <button
        class="o-autobuyer-btn o-autobuyer-btn--unavailable"
      >
        Complete the challenge to change mode
      </button>
    </template>
  </AutobuyerBox>
</template>
