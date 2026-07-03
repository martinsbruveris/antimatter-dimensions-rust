<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerBox from "./AutobuyerBox.vue";
import IntervalUpgradeButton from "./IntervalUpgradeButton.vue";

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
      <IntervalUpgradeButton
        :entry="entry"
        target="tickspeed"
      />
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
