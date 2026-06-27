<script setup>
// Gameplay options subtab. The grid layout mirrors the original
// `OptionsGameplayTab.vue`, but only the Hotkeys enable/disable toggle is
// implemented for now; every other slot is an invisible placeholder so the
// implemented control keeps its original position. Other controls land
// iteratively (see design-docs/2026-06-27-options-tabs.md).
import { useGameStore } from "../../stores/game";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";

const game = useGameStore();

// Bind straight to the snapshot: a toggle changes once per click, so the
// one-tick refresh lag is imperceptible (unlike a dragged slider).
function setHotkeys(value) {
  game.setHotkeys(value);
}
</script>

<template>
  <div class="l-options-tab">
    <div class="l-options-grid">
      <div class="l-options-grid__row">
        <div class="l-options-grid__button l-options-grid__button--hidden" />
        <PrimaryToggleButton
          class="o-primary-btn--option l-options-grid__button"
          label="Hotkeys:"
          on="Enabled"
          off="Disabled"
          :model-value="game.snapshot.options.hotkeys"
          @update:model-value="setHotkeys"
        />
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
