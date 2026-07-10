<script setup>
// The auto-glyph filter panel (a compact port of the original
// AutoGlyphScreen / GlyphFilterPanel): the selection-mode list, the rejected-
// glyph handling dropdown, and the mode-specific per-type settings. Shown in
// the Glyphs tab once Effarig's glyph-filter unlock is bought.
import { computed, ref, watch } from "vue";

import { GLYPH_EFFECTS, GLYPH_TYPES } from "../../../data/glyphs";
import { useGameStore } from "../../../stores/game";

const game = useGameStore();
const reality = computed(() => game.snapshot?.reality);
const filter = computed(() => reality.value?.filter);

const MODE_NAMES = [
  "Lowest Total Glyph Sacrifice",
  "Number of Effects",
  "Rarity Threshold",
  "Specified Effect",
  "Effect Score",
  "Lowest Alchemy Resource",
  "Refinement Value",
];

const TRASH_NAMES = ["Always sacrifice", "Always refine", "Refine to cap, then sacrifice"];

// Local editable copies (pushed to the engine on change).
const select = ref(0);
const trash = ref(0);
const simple = ref(0);
watch(
  filter,
  (f) => {
    if (!f) return;
    select.value = f.select;
    trash.value = f.trash;
    simple.value = f.simple;
  },
  { immediate: true },
);

function pushModes() {
  game.setGlyphFilterModes(Number(select.value), Number(trash.value), Number(simple.value));
}

function typeLabel(cfg) {
  return GLYPH_TYPES[cfg.kind]?.adjective ?? cfg.kind;
}

function effectName(cfg, i) {
  return GLYPH_EFFECTS[cfg.bit_offset + i]?.id ?? `effect ${i}`;
}

function setRarity(cfg, value) {
  game.setGlyphFilterType(cfg.kind, { ...cfg, rarity: Number(value) });
}
function setScore(cfg, value) {
  game.setGlyphFilterType(cfg.kind, { ...cfg, score: Number(value) });
}
function setEffectCount(cfg, value) {
  game.setGlyphFilterType(cfg.kind, { ...cfg, effect_count: Number(value) });
}
function toggleSpecifiedEffect(cfg, i) {
  const bit = 1 << (cfg.bit_offset + i);
  game.setGlyphFilterType(cfg.kind, {
    ...cfg,
    specified_mask: cfg.specified_mask ^ bit,
  });
}
function setEffectScore(cfg, i, value) {
  const scores = [...cfg.effect_scores];
  scores[i] = Number(value);
  game.setGlyphFilterType(cfg.kind, { ...cfg, effect_scores: scores });
}
</script>

<template>
  <div
    v-if="filter"
    class="c-glyph-filter-panel"
  >
    <div class="c-glyph-filter-panel__header">
      <b>Glyph Filter</b> — new Glyphs from automated Realities are picked and
      kept by these rules.
    </div>
    <div class="c-glyph-filter-panel__modes">
      <label>
        Selection mode:
        <select
          v-model="select"
          @change="pushModes"
        >
          <option
            v-for="(name, i) in MODE_NAMES"
            :key="i"
            :value="i"
          >
            {{ name }}
          </option>
        </select>
      </label>
      <label>
        Rejected Glyphs:
        <select
          v-model="trash"
          @change="pushModes"
        >
          <option
            v-for="(name, i) in TRASH_NAMES"
            :key="i"
            :value="i"
          >
            {{ name }}
          </option>
        </select>
      </label>
      <label v-if="Number(select) === 1">
        Minimum effect count:
        <input
          v-model="simple"
          type="number"
          min="0"
          max="7"
          @change="pushModes"
        >
      </label>
    </div>
    <div
      v-if="[2, 3, 4].includes(Number(select))"
      class="c-glyph-filter-panel__types"
    >
      <div
        v-for="cfg in filter.types"
        :key="cfg.kind"
        class="c-glyph-filter-panel__type"
      >
        <div
          class="c-glyph-filter-panel__type-name"
          :style="{ color: GLYPH_TYPES[cfg.kind]?.color }"
        >
          {{ typeLabel(cfg) }}
        </div>
        <label v-if="[2, 3].includes(Number(select))">
          Rarity ≥
          <input
            :value="cfg.rarity"
            type="number"
            min="0"
            max="100"
            @change="setRarity(cfg, $event.target.value)"
          >%
        </label>
        <label v-if="Number(select) === 3">
          Effects ≥
          <input
            :value="cfg.effect_count"
            type="number"
            min="0"
            max="7"
            @change="setEffectCount(cfg, $event.target.value)"
          >
        </label>
        <div v-if="Number(select) === 3">
          <label
            v-for="(score, i) in cfg.effect_scores"
            :key="i"
            class="c-glyph-filter-panel__effect"
          >
            <input
              type="checkbox"
              :checked="(cfg.specified_mask & (1 << (cfg.bit_offset + i))) !== 0"
              @change="toggleSpecifiedEffect(cfg, i)"
            >
            {{ effectName(cfg, i) }}
          </label>
        </div>
        <template v-if="Number(select) === 4">
          <label>
            Score ≥
            <input
              :value="cfg.score"
              type="number"
              @change="setScore(cfg, $event.target.value)"
            >
          </label>
          <label
            v-for="(score, i) in cfg.effect_scores"
            :key="i"
            class="c-glyph-filter-panel__effect"
          >
            {{ effectName(cfg, i) }}:
            <input
              :value="score"
              type="number"
              @change="setEffectScore(cfg, i, $event.target.value)"
            >
          </label>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.c-glyph-filter-panel {
  border: 0.1rem solid var(--color-reality-dark, #094e0b);
  border-radius: var(--var-border-radius, 0.5rem);
  margin: 0.5rem;
  padding: 0.5rem;
  font-size: 1.2rem;
}

.c-glyph-filter-panel__modes {
  display: flex;
  gap: 1rem;
  justify-content: center;
  margin: 0.5rem 0;
  flex-wrap: wrap;
}

.c-glyph-filter-panel__types {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.6rem;
}

.c-glyph-filter-panel__type {
  border: 0.1rem solid currentcolor;
  border-radius: var(--var-border-radius, 0.4rem);
  padding: 0.4rem;
  min-width: 12rem;
}

.c-glyph-filter-panel__type-name {
  font-weight: bold;
}

.c-glyph-filter-panel__effect {
  display: block;
  text-align: left;
}

.c-glyph-filter-panel input[type="number"] {
  width: 6rem;
}
</style>
