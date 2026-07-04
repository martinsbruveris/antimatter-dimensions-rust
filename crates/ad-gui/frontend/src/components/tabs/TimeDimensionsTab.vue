<script setup>
// Time Dimensions tab. Mirrors the original ModernTimeDimensionsTab.vue /
// ModernTimeDimensionRow.vue: Max-all, the Time-Shard / free-Tickspeed-upgrade
// readout, 8 dimension rows (name + ×mult, amount + rate, Buy / Buy Max), and
// the cost-jump footnote. Tiers 5–8 stay locked until Dilation (Phase 5).
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { formatDecimal, formatMultiplier } from "../../util/format";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);
const td = computed(() => s.value?.time_dimensions);

const ORDINALS = ["1st", "2nd", "3rd", "4th", "5th", "6th", "7th", "8th"];

// Rows are shown when unlocked (5–8 hide until Dilation, like the original
// pre-Reality: `showRow = isUnlocked || requirementReached`).
const rows = computed(() =>
  (td.value?.dimensions ?? [])
    .filter((d) => d.is_unlocked)
    .map((d) => ({
      ...d,
      name: `${ORDINALS[d.tier]} Time Dimension`,
      costText: `Cost: ${formatDecimal(d.cost, 2)} EP`,
    }))
);

const showPercentage = computed(
  () => Boolean(s.value?.options?.show_hint_text?.show_percentage) || ui.shiftDown
);

function ratePercent(d) {
  return d.rate_percent > 0 ? ` (+${d.rate_percent.toFixed(2)}%/s)` : "";
}
</script>

<template>
  <div
    v-if="td"
    class="l-time-dim-tab l-centered-vertical-tab"
  >
    <div class="c-subtab-option-container">
      <button
        class="o-primary-btn o-primary-btn--subtab-option"
        @click="game.maxAllTimeDimensions()"
      >
        Max all
      </button>
    </div>
    <div>
      <p>
        You have gained
        <span class="c-time-dim-description__accent">{{ td.total_tick_gained }}</span>
        Tickspeed upgrades from
        <span class="c-time-dim-description__accent">{{ formatDecimal(td.time_shards, 2, 1) }}</span>
        Time Shards.
      </p>
      <p>
        Next Tickspeed upgrade at
        <span class="c-time-dim-description__accent">{{ formatDecimal(td.next_shards, 2, 1) }}</span>,
        increasing by
        <span class="c-time-dim-description__accent">×{{ td.mult_to_next.toFixed(2) }}</span>
        per Tickspeed upgrade gained.
      </p>
    </div>
    <div>
      The amount each additional upgrade requires will start increasing above
      {{ td.softcap.toLocaleString("en-US") }} Tickspeed upgrades.
    </div>
    <div>
      You are getting {{ formatDecimal(td.shards_per_second, 2, 0) }} Time Shards per second.
    </div>
    <div class="l-dimensions-container">
      <div
        v-for="d in rows"
        :key="d.tier"
        class="c-dimension-row l-dimension-row-time-dim l-dimension-single-row"
      >
        <div class="l-dimension-text-container">
          <div class="l-wide-box">
            <span class="c-dim-row__large">{{ d.name }}</span>
            <span class="c-dim-row__small">{{ formatMultiplier(d.multiplier) }}</span>
          </div>
          <div class="l-wide-box">
            <span class="c-dim-row__large">{{ formatDecimal(d.amount, 2) }}</span>
            <span
              v-if="showPercentage && d.rate_percent > 0"
              class="c-dim-row__small"
            >{{ ratePercent(d) }}</span>
          </div>
        </div>
        <div class="l-dim-row-multi-button-container">
          <button
            class="o-primary-btn o-primary-btn--new o-primary-btn--buy-td o-primary-btn--buy-dim"
            :class="{ 'o-primary-btn--disabled': !d.available_for_purchase }"
            @click="game.buyTimeDimension(d.tier)"
          >
            {{ d.costText }}
          </button>
          <button
            class="o-primary-btn o-primary-btn--buy-td-auto"
            :class="{ 'o-primary-btn--disabled': !d.available_for_purchase }"
            @click="game.buyMaxTimeDimension(d.tier)"
          >
            Buy Max
          </button>
        </div>
      </div>
    </div>
    <div>
      Time Dimension costs jump at {{ formatDecimal({ m: 1.79769, e: 308 }, 2, 2) }} and
      {{ formatDecimal({ m: 1, e: 1300 }) }} Eternity Points,
      <br>
      and costs increase much faster after {{ formatDecimal({ m: 1, e: 6000 }) }} Eternity Points.
      <br>
      Any 8th Time Dimensions purchased above {{ formatDecimal({ m: 1, e: 8 }) }} will
      not further increase the multiplier.
    </div>
  </div>
</template>

<style scoped>
/* From the original's scoped styles (GenericDimensionRowText / tab layout). */
.l-time-dim-tab > div {
  margin-top: 0.8rem;
}

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

.c-dim-row__large {
  text-align: left;
  margin-right: 1rem;
}

.c-dim-row__small {
  font-size: 1.2rem;
  margin-right: 1rem;
}
</style>
