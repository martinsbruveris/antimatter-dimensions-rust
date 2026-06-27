<script setup>
// Visual options subtab. The grid layout mirrors the original
// `OptionsVisualTab.vue` (rows of three controls), but only the Update Rate
// slider is implemented for now; every other slot is an invisible placeholder
// so the implemented control keeps its original position. The Classic-UI
// toggle is intentionally dropped (Modern UI only). Other controls land
// iteratively (see design-docs/2026-06-27-options-tabs.md).
import { ref } from "vue";

import { useGameStore } from "../../stores/game";
import OptionsSlider from "../options/OptionsSlider.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";

const game = useGameStore();

// Local copy for a responsive slider: the snapshot only refreshes each tick,
// so driving the dot straight off it would stutter while dragging. We own the
// displayed value and push every change to the engine.
const updateRate = ref(game.snapshot.options.update_rate);

function setUpdateRate(value) {
  updateRate.value = value;
  game.setUpdateRate(value);
}
</script>

<template>
  <div class="l-options-tab">
    <div class="l-options-grid">
      <div class="l-options-grid__row">
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div
          class="o-primary-btn o-primary-btn--option o-primary-btn--slider l-options-grid__button"
        >
          <b>Update rate: {{ updateRate }} ms</b>
          <OptionsSlider
            class="o-primary-btn--slider__slider"
            :min="33"
            :max="200"
            :interval="1"
            :model-value="updateRate"
            @update:model-value="setUpdateRate"
          />
        </div>
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <div class="l-options-grid__row">
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <div class="l-options-grid__row">
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <div class="l-options-grid__row">
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <OpenHotkeysButton />
    </div>
  </div>
</template>
