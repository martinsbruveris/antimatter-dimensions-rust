<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerBox from "./AutobuyerBox.vue";

const props = defineProps({
  tier: { type: Number, required: true },
});

const game = useGameStore();
const entry = computed(() => game.snapshot.autobuyers.dimensions[props.tier]);
const modeDisplay = computed(() =>
  entry.value.mode === "single" ? "Buys singles" : "Buys max"
);
</script>

<template>
  <AutobuyerBox
    :entry="entry"
    show-bulk
    @unlock="game.unlockAdAutobuyer(tier)"
    @toggle="game.toggleAdAutobuyer(tier)"
  >
    <template #intervalSlot>
      <!-- Interval upgrades cost Infinity Points; locked until a challenge is
           completed (pre-Infinity this button is always disabled). -->
      <button
        class="o-autobuyer-btn l-autobuyer-box__button o-autobuyer-btn--unavailable"
      >
        Complete the challenge to upgrade interval
      </button>
    </template>
    <template #toggleSlot>
      <button
        class="o-autobuyer-btn"
        @click="game.toggleAdAutobuyerMode(tier)"
      >
        {{ modeDisplay }}
      </button>
    </template>
  </AutobuyerBox>
</template>
