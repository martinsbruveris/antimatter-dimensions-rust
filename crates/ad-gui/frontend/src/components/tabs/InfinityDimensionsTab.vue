<script setup>
// The Infinity → Infinity Dimensions subtab: the Infinity-Power header plus 8
// dimension rows (amount, multiplier, unlock/buy, buy-max). State comes from the
// engine snapshot (`game.infinity_dimensions`).
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal, formatMultiplier } from "../../util/format";
import { floatToNum } from "../../util/num";

const game = useGameStore();
const s = computed(() => game.snapshot);
const id = computed(() => s.value?.infinity_dimensions);

const ORDINALS = ["1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"];

const dims = computed(() =>
  (id.value?.dimensions ?? []).map((d) => {
    let buttonText;
    if (!d.is_unlocked) buttonText = d.can_unlock ? "Unlock" : "Locked";
    else if (d.is_capped) buttonText = "Capped";
    else buttonText = `Cost: ${formatDecimal(d.cost, 2)} IP`;
    const enabled = (!d.is_unlocked && d.can_unlock) || d.can_be_bought;
    return {
      ...d,
      name: `${ORDINALS[d.tier]} Infinity Dimension`,
      buttonText,
      enabled,
    };
  }),
);

function buy(d) {
  if (d.enabled) game.buyInfinityDimension(d.tier);
}
function buyMax(d) {
  if (d.is_unlocked) game.buyMaxInfinityDimension(d.tier);
}

// Tesseracts (once Enslaved's Reality is completed): "N" or "N + extra".
const tesseractCountString = computed(() => {
  const extra = id.value?.extra_tesseracts ?? 0;
  const bought = id.value?.tesseracts ?? 0;
  return extra > 0 ? `${bought} + ${extra.toFixed(2)}` : `${bought}`;
});
</script>

<template>
  <div
    v-if="id"
    class="l-infinity-dims-tab"
  >
    <div class="c-infinity-tab__header">
      You have
      <span class="c-infinity-tab__infinity-points">{{
        formatDecimal(id.power, 2)
      }}</span>
      Infinity Power, giving a {{ formatMultiplier(id.power_mult) }} multiplier to
      all Antimatter Dimensions.
    </div>

    <button
      class="o-primary-btn l-infinity-dims-tab__max"
      @click="game.buyMaxAllInfinityDimensions()"
    >
      Max all
    </button>

    <div
      v-if="id.tesseract_unlocked"
      class="l-infinity-dim-tab__tesseract-container"
    >
      <button
        class="c-infinity-dim-tab__tesseract-button"
        :class="{
          'c-infinity-dim-tab__tesseract-button--disabled': !id.can_buy_tesseract,
        }"
        @click="game.buyTesseract()"
      >
        <p>Buy a Tesseract ({{ tesseractCountString }})</p>
        <p>Increase dimension caps by {{ formatDecimal(floatToNum(id.next_dim_cap_increase), 2) }}</p>
        <p><b>Costs: {{ formatDecimal(id.tesseract_cost) }} IP</b></p>
      </button>
    </div>

    <div class="l-infinity-dims-grid">
      <div
        v-for="d in dims"
        :key="d.tier"
        class="c-id-row"
      >
        <div class="c-id-row__label">
          {{ d.name }}:
          <span v-if="d.is_unlocked">
            {{ formatDecimal(d.amount, 2) }} ({{ formatMultiplier(d.multiplier) }})
          </span>
          <span v-else>locked</span>
        </div>
        <button
          class="o-primary-btn"
          :class="{ 'o-primary-btn--disabled': !d.enabled }"
          @click="buy(d)"
        >
          {{ d.buttonText }}
        </button>
        <button
          v-if="d.is_unlocked && !d.is_capped"
          class="o-primary-btn"
          @click="buyMax(d)"
        >
          Buy max
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.l-infinity-dims-grid {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-top: 1rem;
}

.c-id-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  justify-content: center;
}

.c-id-row__label {
  min-width: 24rem;
  text-align: right;
}

.l-infinity-dims-tab__max {
  margin-top: 0.6rem;
}
</style>
