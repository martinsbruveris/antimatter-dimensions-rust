<script setup>
// The Glyph Sacrifice totals panel (SacrificedGlyphs.vue + TypeSacrifice;
// the celestial alteration block is out of frontier).
import { computed } from "vue";

import {
  BASIC_TYPE_ORDER,
  GLYPH_TYPES,
  SACRIFICE_DESCRIPTIONS,
} from "../../../data/glyphs";
import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);

const rows = computed(() => {
  const totals = reality.value?.sac_totals ?? [];
  const effects = reality.value?.sac_effects ?? [];
  return BASIC_TYPE_ORDER.map((type, i) => ({
    type,
    symbol: GLYPH_TYPES[type].symbol,
    color: GLYPH_TYPES[type].color,
    total: totals[i] ?? 0,
    description: SACRIFICE_DESCRIPTIONS[i](effects[i] ?? 0),
  })).filter((row) => row.total > 0);
});

function formatTotal(total) {
  const e = total > 0 ? Math.floor(Math.log10(total)) : 0;
  return formatDecimal({ m: total / Math.pow(10, e), e }, 2, 2);
}
</script>

<template>
  <div class="c-current-glyph-effects l-current-glyph-effects">
    <div class="l-sacrificed-glyphs__help">
      <div>Shift-click Glyphs in the inventory to Sacrifice them.</div>
    </div>
    <br>
    <div class="c-sacrificed-glyphs__header">
      Glyph Sacrifice Boosts:
    </div>
    <div v-if="rows.length">
      <div
        v-for="row in rows"
        :key="row.type"
        class="l-sacrificed-glyphs__type"
      >
        <span
          class="l-sacrificed-glyphs__type-symbol"
          :style="{ color: row.color }"
        >{{ row.symbol }}</span>
        <span class="l-sacrificed-glyphs__type-amount">
          {{ formatTotal(row.total) }}:
        </span>
        {{ row.description }}
      </div>
    </div>
    <div v-else>
      You haven't Sacrificed any Glyphs yet!
    </div>
  </div>
</template>

<style scoped>
.l-sacrificed-glyphs__type {
  margin: 0.4rem 0;
}

.l-sacrificed-glyphs__type-symbol {
  font-size: 1.6rem;
  margin-right: 0.4rem;
}
</style>
