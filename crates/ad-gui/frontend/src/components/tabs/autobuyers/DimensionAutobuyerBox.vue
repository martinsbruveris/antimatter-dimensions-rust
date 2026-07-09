<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerBox from "./AutobuyerBox.vue";
import DimensionBulkButton from "./DimensionBulkButton.vue";
import IntervalUpgradeButton from "./IntervalUpgradeButton.vue";

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
      <IntervalUpgradeButton
        :entry="entry"
        :target="`ad${tier}`"
      />
      <DimensionBulkButton
        :entry="entry"
        :tier="tier"
      />
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
