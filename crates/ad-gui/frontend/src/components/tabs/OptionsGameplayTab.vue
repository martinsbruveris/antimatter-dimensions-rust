<script setup>
// Gameplay options subtab. The grid layout mirrors the original
// `OptionsGameplayTab.vue`; the Hotkeys toggle and the Offline-ticks slider are
// implemented, the remaining slots are invisible placeholders so the live
// controls keep their original positions. More land iteratively (see
// design-docs/2026-06-27-options-tabs.md).
import { ref } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal } from "../../util/format";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";
import OptionsSlider from "../options/OptionsSlider.vue";

const game = useGameStore();

// Bind straight to the snapshot: a toggle changes once per click, so the
// one-tick refresh lag is imperceptible (unlike a dragged slider).
function setHotkeys(value) {
  game.setHotkeys(value);
}

// Offline-ticks slider. Values follow the original's per-decade spacing
// `(1 + v%9) × 10^floor(v/9)`, but over slider indices 36..=63 → 10K, 20K, …,
// 100K, …, 1M, …, 10M (the original runs 22..=54 → 500..1M). See
// design-docs/2026-06-30-offline-progress.md.
const SLIDER_MIN = 36;
const SLIDER_MAX = 63;

function ticksFromSlider(v) {
  return (1 + (v % 9)) * 10 ** Math.floor(v / 9);
}

function sliderFromTicks(ticks) {
  const exponent = Math.floor(Math.log10(ticks));
  const mantissa = ticks / 10 ** exponent - 1;
  const raw = Math.round(9 * exponent + mantissa);
  return Math.min(SLIDER_MAX, Math.max(SLIDER_MIN, raw));
}

// Local slider index for immediate, jitter-free dragging; the engine is the
// source of truth but its snapshot lags a tick behind a drag.
const sliderIndex = ref(sliderFromTicks(game.snapshot.options.offline_ticks));

function onSlide(value) {
  sliderIndex.value = value;
  game.setOfflineTicks(ticksFromSlider(value));
}

// The original labels this with `formatInt(offlineTicks)`; render the live
// slider value notation-aware (so e.g. Standard shows "100,000").
function formatInt(n) {
  if (n === 0) return "0";
  const e = Math.floor(Math.log10(n));
  return formatDecimal({ m: n / 10 ** e, e }, 0, 0);
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
        <div
          class="o-primary-btn o-primary-btn--option o-primary-btn--slider l-options-grid__button"
        >
          <b>Offline ticks: {{ formatInt(ticksFromSlider(sliderIndex)) }}</b>
          <OptionsSlider
            class="o-primary-btn--slider__slider"
            :min="SLIDER_MIN"
            :max="SLIDER_MAX"
            :interval="1"
            :model-value="sliderIndex"
            @update:model-value="onSlide"
          />
        </div>
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <OpenHotkeysButton />
    </div>
  </div>
</template>
