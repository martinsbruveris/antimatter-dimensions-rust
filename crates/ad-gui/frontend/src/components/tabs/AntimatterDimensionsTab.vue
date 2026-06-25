<script setup>
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import TickspeedRow from "../TickspeedRow.vue";
import DimensionRow from "../DimensionRow.vue";
import DimBoostRow from "../DimBoostRow.vue";
import GalaxyRow from "../GalaxyRow.vue";
import ProgressBar from "../ProgressBar.vue";

const game = useGameStore();
const s = computed(() => game.snapshot);

const multiplierText = computed(() => {
  let t = `Buy 10 Dimension purchase multiplier: ×${s.value.buy_ten_multiplier}`;
  if (s.value.sacrifice_unlocked) {
    t += ` | Dimensional Sacrifice multiplier: ×${s.value.sacrifice_multiplier}`;
  }
  return t;
});

const sacrificeTooltip = computed(() =>
  s.value.can_sacrifice
    ? `Boosts 8th Antimatter Dimension by ×${s.value.next_sacrifice_boost}`
    : ""
);
</script>

<template>
  <div class="l-antimatter-dim-tab">
    <div class="modes-container">
      <button
        class="o-primary-btn l-button-container"
        @click="game.toggleBuyMode()"
      >
        {{ game.buyUntil10 ? "Until 10" : "Buy 1" }}
      </button>
      <button
        v-show="s.sacrifice_unlocked"
        class="o-primary-btn o-primary-btn--sacrifice"
        :class="{ 'o-primary-btn--disabled': !s.can_sacrifice }"
        :title="sacrificeTooltip"
        @click="game.sacrifice()"
      >
        <span v-if="s.can_sacrifice">
          Dimensional Sacrifice (×{{ s.next_sacrifice_boost }})
        </span>
        <span v-else>
          Dimensional Sacrifice Disabled ({{ s.sacrifice_disabled_condition }})
        </span>
      </button>
      <button
        class="o-primary-btn l-button-container"
        @click="game.maxAll()"
      >
        Max All (M)
      </button>
    </div>
    <span>{{ multiplierText }}</span>
    <TickspeedRow />
    <div class="l-dimensions-container">
      <DimensionRow
        v-for="tier in 8"
        :key="tier"
        :tier="tier - 1"
      />
    </div>
    <div class="resets-container">
      <DimBoostRow />
      <GalaxyRow />
    </div>
    <ProgressBar />
  </div>
</template>

<style scoped>
/* From ModernAntimatterDimensionsTab.vue scoped style (not in the
   global vendored CSS). */
.l-button-container {
  width: 100px;
  height: 30px;
  padding: 0;
}
</style>
