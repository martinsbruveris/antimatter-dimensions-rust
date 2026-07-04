<script setup>
// Visual options subtab. The grid layout mirrors the original
// `OptionsVisualTab.vue` (rows of three controls): row 1 the Update Rate
// slider; row 2 the Notation picker + Exponent Notation Options button; row 3
// the Animation / Info Display / Away Progress modal buttons; row 4 the
// Modify Visible Tabs button, the prestige-gain coloring toggle, and the
// Sidebar resource picker. Remaining slots (Classic-UI toggle — intentionally
// dropped, Modern UI only; News; Theme) are invisible placeholders so the
// implemented controls keep their original positions (see
// design-docs/2026-06-27-options-tabs.md).
import { ref } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import OptionsSlider from "../options/OptionsSlider.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import SelectNotationDropdown from "../options/SelectNotationDropdown.vue";
import SelectSidebarDropdown from "../options/SelectSidebarDropdown.vue";

const game = useGameStore();
const ui = useUiStore();

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
        <SelectNotationDropdown />
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('notation')"
        >
          Open Exponent Notation Options
        </button>
      </div>
      <div class="l-options-grid__row">
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('animationOptions')"
        >
          Open Animation Options
        </button>
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('infoDisplayOptions')"
        >
          Open Info Display Options
        </button>
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('awayProgressOptions')"
        >
          Open Away Progress Options
        </button>
      </div>
      <div class="l-options-grid__row">
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('hiddenTabs')"
        >
          Modify Visible Tabs
        </button>
        <PrimaryToggleButton
          class="o-primary-btn--option l-options-grid__button"
          label="Relative prestige gain text coloring:"
          :model-value="game.snapshot.options.header_text_colored"
          @update:model-value="game.setHeaderTextColored($event)"
        />
        <SelectSidebarDropdown />
      </div>
      <OpenHotkeysButton />
    </div>
  </div>
</template>
