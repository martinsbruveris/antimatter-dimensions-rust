<script setup>
// The Reality → Glyphs page (GlyphsTab.vue): reset-reality button + glyph
// level instability notice on the left, equipped glyphs / effects panel /
// inventory on the right. The celestial widgets (peek, amplify, filter
// sidebar, level factors) are out of frontier.
import { computed, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";
import CurrentGlyphEffects from "./CurrentGlyphEffects.vue";
import EquippedGlyphs from "./EquippedGlyphs.vue";
import GlyphInventory from "./GlyphInventory.vue";
import SacrificedGlyphs from "./SacrificedGlyphs.vue";

const game = useGameStore();
const ui = useUiStore();
const reality = computed(() => game.snapshot?.reality);

const showInstability = computed(() => (reality.value?.best_glyph_level ?? 0) > 800);
const sacrificeDisplayed = ref(false);
</script>

<template>
  <div v-if="reality">
    <div class="l-glyphs-tab">
      <div class="l-reality-button-column">
        <button
          v-if="reality.unlocked"
          class="c-reset-reality-button l-reset-reality-button"
          @click="ui.showModal('resetReality')"
        >
          Start this Reality over
        </button>
        <br>
        <div v-if="showInstability">
          <br>
          Glyphs are becoming unstable.
          <br>
          Glyph levels higher than 1,000 are harder to reach.
          <br>
          This effect is even stronger above level 4,000.
        </div>
      </div>
      <div class="l-player-glyphs-column">
        <div class="l-equipped-glyphs-and-effects-container">
          <EquippedGlyphs />
          <div class="l-glyph-info-wrapper">
            <div
              v-if="reality.can_sacrifice"
              class="c-glyph-info-options"
            >
              <button
                class="l-glyph-info-button c-glyph-info-button"
                :class="sacrificeDisplayed
                  ? 'c-glyph-info-button--inactive'
                  : 'c-glyph-info-button--active'"
                @click="sacrificeDisplayed = false"
              >
                Current Glyph effects
              </button>
              <button
                class="l-glyph-info-button c-glyph-info-button"
                :class="sacrificeDisplayed
                  ? 'c-glyph-info-button--active'
                  : 'c-glyph-info-button--inactive'"
                @click="sacrificeDisplayed = true"
              >
                Glyph Sacrifice totals
              </button>
            </div>
            <SacrificedGlyphs v-if="reality.can_sacrifice && sacrificeDisplayed" />
            <CurrentGlyphEffects
              v-else
              :class="{ 'c-current-glyph-effects-with-top-border': !reality.can_sacrifice }"
            />
          </div>
        </div>
        <GlyphInventory />
      </div>
    </div>
  </div>
</template>
