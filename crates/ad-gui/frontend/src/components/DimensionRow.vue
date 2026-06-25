<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { DIM_NAMES } from "../util/dimensionText";
import { isSmall } from "../util/responsive";

const props = defineProps({ tier: { type: Number, required: true } });

const game = useGameStore();
const s = computed(() => game.snapshot);
const dim = computed(() => s.value.dimensions[props.tier]);
const unlocked = computed(() => props.tier < s.value.unlocked_dimensions);

// Stack name/multiplier (and amount/rate) vertically on narrow windows,
// matching the original GenericDimensionRowText.
const boxClass = computed(() => (isSmall.value ? "l-narrow-box" : "l-wide-box"));

const name = computed(() => `${DIM_NAMES[props.tier]} Antimatter Dimension`);

// "Until 10" buys as many as fill the group; "Buy 1" caps at one.
const howMany = computed(() =>
  game.buyUntil10 ? dim.value.how_many_can_buy : Math.min(dim.value.how_many_can_buy, 1)
);
const costText = computed(() => {
  const cost = game.buyUntil10 ? dim.value.until_10_cost : dim.value.single_cost;
  return `Cost: ${cost} AM`;
});
const hasLongText = computed(() => costText.value.length > 20);
const showRate = computed(() => props.tier < 7 && dim.value.rate_percent > 0.01);

function buy() {
  if (unlocked.value) game.buyDim(props.tier);
}
</script>

<template>
  <div
    class="c-dimension-row l-dimension-row-antimatter-dim c-antimatter-dim-row l-dimension-single-row"
    :class="{ 'c-dim-row--not-reached': !unlocked }"
  >
    <div class="l-dimension-text-container">
      <div :class="boxClass">
        <span class="c-dim-row__large">{{ name }}</span>
        <span
          v-if="unlocked"
          class="c-dim-row__small"
        >×{{ dim.multiplier }}</span>
      </div>
      <div :class="boxClass">
        <span
          v-if="unlocked"
          class="c-dim-row__large"
        >{{ dim.amount }}</span>
        <span
          v-if="unlocked && showRate"
          class="c-dim-row__small"
        >(+{{ dim.rate_percent.toFixed(2) }}%/s)</span>
      </div>
    </div>
    <div class="l-dim-row-multi-button-container">
      <button
        class="o-primary-btn o-primary-btn--new"
        :class="{ 'o-primary-btn--disabled': !unlocked || !dim.can_buy }"
        :title="unlocked ? `Purchased ${dim.bought} times` : ''"
        @click="buy"
      >
        <div class="button-content l-modern-buy-ad-text">
          <div>{{ unlocked ? `Buy ${howMany}` : "Locked" }}</div>
          <div
            v-if="unlocked"
            :class="{ 'l-dim-row-small-text': hasLongText }"
          >
            {{ costText }}
          </div>
        </div>
        <div
          v-if="unlocked"
          class="fill"
        >
          <div
            class="fill-purchased"
            :style="{ width: dim.bought_mod_10 * 10 + '%' }"
          />
          <div
            class="fill-possible"
            :style="{ width: howMany * 10 + '%' }"
          />
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original GenericDimensionRowText.vue and
   ModernAntimatterDimensionRow.vue scoped styles (these classes are
   not in the global vendored CSS). */
.l-dimension-text-container {
  display: flex;
  height: 3.5rem;
  align-content: center;
  grid-column: 1 / 5;
}

.l-wide-box {
  display: flex;
  text-align: left;
  width: 100%;
  flex-direction: row;
  justify-content: flex-start;
  align-items: center;
}

.l-narrow-box {
  display: flex;
  text-align: left;
  width: 100%;
  flex-direction: column;
  justify-content: center;
  align-items: flex-start;
}

.c-dim-row__large {
  text-align: left;
  margin-right: 1rem;
}

.c-dim-row__small {
  font-size: 1.2rem;
  margin-right: 1rem;
}

.l-modern-buy-ad-text {
  display: flex;
  flex-direction: column;
}
</style>
