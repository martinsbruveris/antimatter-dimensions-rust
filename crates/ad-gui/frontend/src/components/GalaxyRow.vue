<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { GALAXY_BUTTON_TEXT } from "../util/dimensionText";
import { TUTORIAL_STATE, hasTutorial } from "../util/tutorial";

const game = useGameStore();
const s = computed(() => game.snapshot);

// Tutorial highlight for the Antimatter Galaxy button (the GALAXY step).
const hasTut = computed(() => hasTutorial(s.value, TUTORIAL_STATE.GALAXY));
</script>

<template>
  <div class="reset-container galaxy">
    <h4>Antimatter Galaxies ({{ s.galaxies }})</h4>
    <span>Requires: {{ s.galaxy_requirement }} 8th Antimatter D</span>
    <button
      class="o-primary-btn o-primary-btn--new o-primary-btn--dimension-reset"
      :class="{
        'o-primary-btn--disabled': !s.can_buy_galaxy,
        'tutorial--glow': hasTut && s.can_buy_galaxy,
      }"
      @click="game.requestGalaxy()"
    >
      {{ GALAXY_BUTTON_TEXT }}
      <div
        v-if="hasTut"
        class="fas fa-circle-exclamation l-notification-icon"
      />
    </button>
  </div>
</template>
