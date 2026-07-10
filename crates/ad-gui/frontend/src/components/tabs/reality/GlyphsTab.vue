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
import GlyphFilterPanel from "./GlyphFilterPanel.vue";
import GlyphInventory from "./GlyphInventory.vue";
import SacrificedGlyphs from "./SacrificedGlyphs.vue";

const game = useGameStore();
const ui = useUiStore();
const reality = computed(() => game.snapshot?.reality);

const showInstability = computed(() => (reality.value?.best_glyph_level ?? 0) > 800);

// The Reality-amplify button (RealityAmplifyButton.vue): visible once
// Enslaved is unlocked; arms Enslaved.boostReality for the next Reality.
const enslaved = computed(() => game.snapshot?.celestials?.enslaved);
const amplifyTooltip = computed(() => {
  if (!enslaved.value?.can_amplify && !enslaved.value?.boost_reality) {
    return "Store more real time or complete the Reality faster to amplify";
  }
  return null;
});
function toggleAmplify() {
  game.toggleBoostReality();
}
const sacrificeDisplayed = ref(false);

// Effarig's glyph-level weight adjuster (4 factors summing 100). Adjusting
// one weight proportionally rebalances the other three, like the original's
// slider group.
const WEIGHT_NAMES = ["EP", "Replicanti", "DT", "Eternities"];
function setWeight(index, valueIn) {
  const weights = [...(reality.value?.glyph_weights ?? [25, 25, 25, 25])];
  const value = Math.min(100, Math.max(0, Number(valueIn)));
  const othersTotal = 100 - value;
  const oldOthers = weights.reduce((a, w, i) => (i === index ? a : a + w), 0);
  for (let i = 0; i < 4; i++) {
    if (i === index) weights[i] = value;
    else weights[i] = oldOthers > 0 ? (weights[i] / oldOthers) * othersTotal : othersTotal / 3;
  }
  game.setGlyphWeights(weights);
}
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
        <button
          v-if="enslaved?.unlocked"
          class="l-reality-amplify-button"
          :class="{
            'l-reality-amplify-button--clickable': enslaved.can_amplify,
            'o-enslaved-mechanic-button--storing-time': enslaved.boost_reality,
          }"
          :ach-tooltip="amplifyTooltip"
          @click="toggleAmplify"
        >
          <div v-if="enslaved.can_amplify || enslaved.boost_reality">
            <span v-if="enslaved.boost_reality">Will be amplified:</span>
            <span v-else>Amplify this Reality:</span>
            <br>
            All rewards ×{{ Math.floor(enslaved.reality_boost_ratio) }}
          </div>
          <div v-else>
            Not enough stored real time to amplify.
          </div>
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
        <div
          v-if="reality.weights_unlocked"
          class="c-glyph-weights-panel"
        >
          <b>Glyph level factor weights</b>
          <label
            v-for="(name, i) in WEIGHT_NAMES"
            :key="name"
          >
            {{ name }}:
            <input
              type="number"
              min="0"
              max="100"
              :value="Math.round(reality.glyph_weights[i])"
              @change="setWeight(i, $event.target.value)"
            >
          </label>
        </div>
        <GlyphFilterPanel v-if="reality.filter_unlocked" />
        <GlyphInventory />
      </div>
    </div>
  </div>
</template>

<style scoped>
.c-glyph-weights-panel {
  display: flex;
  gap: 1rem;
  justify-content: center;
  align-items: center;
  margin: 0.4rem;
  font-size: 1.2rem;
}

.c-glyph-weights-panel input {
  width: 5rem;
}
</style>
