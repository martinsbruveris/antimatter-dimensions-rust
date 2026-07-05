<script setup>
// The "Currently active Glyph effects" panel (CurrentGlyphEffects.vue),
// driven by the engine's combined effect totals.
import { computed } from "vue";

import { GLYPH_EFFECTS } from "../../../data/glyphs";
import { useGameStore } from "../../../stores/game";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);

const effects = computed(() =>
  (reality.value?.active_effects ?? []).map((e) => ({
    bit: e.bit,
    capped: e.capped,
    text: GLYPH_EFFECTS[e.bit] ? GLYPH_EFFECTS[e.bit].total(e.value) : "",
  }))
);
const anyCapped = computed(() => effects.value.some((e) => e.capped));
</script>

<template>
  <div class="c-current-glyph-effects l-current-glyph-effects">
    <div class="c-current-glyph-effects__header">
      Currently active Glyph effects:
    </div>
    <div
      v-if="anyCapped"
      class="l-current-glyph-effects__capped-header"
    >
      <span class="c-current-glyph-effects__effect--capped">Italic</span>
      effects have been slightly reduced due to a softcap
    </div>
    <br>
    <div v-if="effects.length === 0">
      None (equip Glyphs to get their effects)
    </div>
    <div
      v-for="effect in effects"
      :key="effect.bit"
      class="c-current-glyph-effects__effect"
      :class="{ 'c-current-glyph-effects__effect--capped': effect.capped }"
    >
      {{ effect.text }}
    </div>
  </div>
</template>
