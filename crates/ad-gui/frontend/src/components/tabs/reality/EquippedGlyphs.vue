<script setup>
// The equipped-glyph circle + the respec and undo buttons (EquippedGlyphs.vue;
// the protected-slot-target / cosmetic buttons are out of frontier).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import GlyphComponent from "../../GlyphComponent.vue";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);

const GLYPH_SIZE = 5;
const slotCount = computed(() => reality.value?.active_slot_count ?? 3);
const arrangementRadius = computed(() => slotCount.value + 1);

const slots = computed(() => {
  const list = Array(slotCount.value).fill(null);
  for (const glyph of reality.value?.active_glyphs ?? []) {
    if (glyph.idx < list.length) list[glyph.idx] = glyph;
  }
  return list;
});

function positionStyle(idx) {
  const angle = (2 * Math.PI * idx) / slotCount.value;
  const dx = -GLYPH_SIZE / 2 + arrangementRadius.value * Math.sin(angle);
  const dy = -GLYPH_SIZE / 2 + arrangementRadius.value * Math.cos(angle);
  return {
    position: "absolute",
    left: `calc(50% + ${dx}rem)`,
    top: `calc(50% + ${dy}rem)`,
    "z-index": 1,
  };
}

const respec = computed(() => Boolean(reality.value?.respec));
const undoVisible = computed(() => Boolean(reality.value?.undo_unlocked));
const undoAvailable = computed(() => Boolean(reality.value?.can_undo));
const respecStyle = computed(() =>
  respec.value
    ? {
        color: "var(--color-reality-light)",
        "background-color": "var(--color-reality)",
        "border-color": "#094e0b",
        cursor: "pointer",
      }
    : { cursor: "pointer" }
);
</script>

<template>
  <div class="l-equipped-glyphs">
    <div class="l-equipped-glyphs__slots">
      <div
        v-for="(glyph, idx) in slots"
        :key="idx"
        class="l-glyph-set-preview"
        :style="positionStyle(idx)"
      >
        <GlyphComponent
          v-if="glyph"
          :glyph="glyph"
          :circular="true"
          class="c-equipped-glyph"
        />
        <div
          v-else
          class="l-equipped-glyphs__empty c-equipped-glyphs__empty"
        />
      </div>
    </div>
    <div class="l-equipped-glyphs__buttons">
      <button
        class="c-reality-upgrade-btn l-glyph-equip-button-short"
        :style="respecStyle"
        :title="respec
          ? 'Respec is active and will place your currently-equipped Glyphs into your inventory after Reality.'
          : 'Your currently-equipped Glyphs will stay equipped on Reality.'"
        @click="game.setGlyphRespec(!respec)"
      >
        Unequip Glyphs on Reality
      </button>
      <button
        v-if="undoVisible"
        class="c-reality-upgrade-btn l-glyph-equip-button-short"
        :class="{ 'c-reality-upgrade-btn--unavailable': !undoAvailable }"
        title="Unequips the last equipped Glyph and rewinds this Reality's
          progress to the point it was equipped."
        @click="undoAvailable && game.undoGlyph()"
      >
        <span>Rewind to <b>undo</b> the last equipped Glyph</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.l-glyph-equip-button-short {
  width: 100%;
  height: 2.5rem;
  margin: 0.25rem 0.5rem;
}
</style>
