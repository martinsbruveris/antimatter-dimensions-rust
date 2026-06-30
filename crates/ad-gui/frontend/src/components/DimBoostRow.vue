<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { DIM_NAMES, dimBoostText } from "../util/dimensionText";
import { TUTORIAL_STATE, hasTutorial } from "../util/tutorial";

const game = useGameStore();
const s = computed(() => game.snapshot);

const dimName = computed(() => DIM_NAMES[s.value.dim_boost_req_tier]);
const buttonText = computed(() =>
  dimBoostText(s.value.dim_boosts, s.value.dim_boost_power)
);

// Tutorial highlight for the Dimension Boost button (the DIMBOOST step). The
// glow shows when the boost is buyable (it has no "affordability" beyond that).
const hasTut = computed(() => hasTutorial(s.value, TUTORIAL_STATE.DIMBOOST));
</script>

<template>
  <div class="reset-container dimboost">
    <h4>Dimension Boost ({{ s.dim_boosts }})</h4>
    <span>Requires: {{ s.dim_boost_req_amount }} {{ dimName }} Antimatter D</span>
    <button
      class="o-primary-btn o-primary-btn--new o-primary-btn--dimension-reset"
      :class="{
        'o-primary-btn--disabled': !s.can_dim_boost,
        'tutorial--glow': hasTut && s.can_dim_boost,
      }"
      @click="game.requestDimBoost()"
    >
      {{ buttonText }}
      <div
        v-if="hasTut"
        class="fas fa-circle-exclamation l-notification-icon"
      />
    </button>
  </div>
</template>
