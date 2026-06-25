<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { GALAXY_BUTTON_TEXT } from "../util/dimensionText";

const game = useGameStore();
const s = computed(() => game.snapshot);
</script>

<template>
  <div class="reset-container galaxy">
    <h4>Antimatter Galaxies ({{ s.galaxies }})</h4>
    <span>Requires: {{ s.galaxy_requirement }} 8th Antimatter D</span>
    <button
      class="o-primary-btn o-primary-btn--new o-primary-btn--dimension-reset"
      :class="{ 'o-primary-btn--disabled': !s.can_buy_galaxy }"
      @click="game.buyGalaxy()"
    >
      {{ GALAXY_BUTTON_TEXT }}
    </button>
  </div>
</template>
