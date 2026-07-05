<script setup>
// The 12×10 glyph inventory grid (GlyphInventory.vue). Double-click equips
// into the first free active slot; shift-click sacrifices (with the
// confirmation unless force-clicked with Ctrl+Shift).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";
import GlyphComponent from "../../GlyphComponent.vue";

const game = useGameStore();
const ui = useUiStore();
const reality = computed(() => game.snapshot?.reality);

const ROWS = 12;
const COLS = 10;

const byIndex = computed(() => {
  const map = new Map();
  for (const glyph of reality.value?.inventory_glyphs ?? []) {
    map.set(glyph.idx, glyph);
  }
  return map;
});
const protectedSlots = computed(() => (reality.value?.protected_rows ?? 2) * 10);

function slotClass(index) {
  return index < protectedSlots.value
    ? "c-glyph-inventory__protected-slot"
    : "c-glyph-inventory__slot";
}

function equip(id) {
  game.equipGlyph(id);
}

function requestSacrifice(id) {
  const glyph = (reality.value?.inventory_glyphs ?? []).find((g) => g.id === id);
  if (!glyph) return;
  ui.showModal("glyphSacrifice", {
    glyph,
    canSacrifice: reality.value?.can_sacrifice,
  });
}
</script>

<template>
  <div class="l-glyph-inventory">
    Double-click to equip Glyphs. Shift-click to {{ reality?.can_sacrifice ? "sacrifice" : "delete" }}.
    <div
      v-for="row in ROWS"
      :key="row"
      class="l-glyph-inventory__row"
    >
      <div
        v-for="col in COLS"
        :key="col"
        class="l-glyph-inventory__slot"
        :class="slotClass((row - 1) * COLS + (col - 1))"
      >
        <GlyphComponent
          v-if="byIndex.get((row - 1) * COLS + (col - 1))"
          :glyph="byIndex.get((row - 1) * COLS + (col - 1))"
          :show-sacrifice="reality?.can_sacrifice"
          :tooltip-above="row > 6"
          @shiftClicked="requestSacrifice"
          @ctrlShiftClicked="game.sacrificeGlyph"
          @dblclick="equip(byIndex.get((row - 1) * COLS + (col - 1)).id)"
        />
      </div>
    </div>
  </div>
</template>
