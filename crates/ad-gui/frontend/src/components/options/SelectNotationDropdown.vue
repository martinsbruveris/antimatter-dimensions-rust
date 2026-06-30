<script setup>
// Notation picker for the Visual options tab. A simplified port of the
// original's ExpandingControlBox + SelectNotationDropdown: a header button that
// expands an inline list. Reuses the vendored `l-select-notation` /
// `c-select-notation__item` styling; the expanding-box layout classes (not in
// the global CSS) are replicated in the scoped block below.
//
// Only the notations `ad-format` implements are listed (no dead entries) — see
// design-docs/2026-06-27-options-tabs.md §8.
import { ref } from "vue";

import { useGameStore } from "../../stores/game";

const NOTATIONS = [
  "Scientific",
  "Engineering",
  "Standard",
  "Letters",
  "Mixed scientific",
  "Mixed engineering",
  "Logarithm",
  "Infinity",
];

const game = useGameStore();
const open = ref(false);

function select(name) {
  game.setNotation(name);
  open.value = false;
}
</script>

<template>
  <div class="l-expanding-control-box l-options-grid__button c-options-grid__notations">
    <div
      class="l-expanding-control-box__container"
      @mouseleave="open = false"
    >
      <div
        class="o-primary-btn o-primary-btn--option l-options-grid__notations-header"
        @click="open = !open"
      >
        Notation: {{ game.snapshot.options.notation }}
        <span
          class="c-indicator-arrow"
          :class="{ 'c-indicator-arrow--flipped': open }"
        >▼</span>
      </div>
      <div v-show="open">
        <div class="l-select-notation">
          <div class="l-select-notation__inner">
            <div
              v-for="notation in NOTATIONS"
              :key="notation"
              class="o-primary-btn l-select-notation__item c-select-notation__item"
              @click="select(notation)"
            >
              {{ notation }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original ExpandingControlBox.vue scoped style (these
   layout classes are not in the global vendored CSS). The root reserves the
   grid cell; the container floats the header + dropdown on top. */
.l-expanding-control-box {
  position: relative;
  z-index: 3;
  height: 5.5rem;
}

.l-expanding-control-box__container {
  display: block;
  width: 100%;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.o-primary-btn--option {
  cursor: pointer;
}

.c-indicator-arrow {
  margin-left: 0.6rem;
  transition: transform 0.25s ease-out;
}

.c-indicator-arrow--flipped {
  transform: rotate(-180deg);
}
</style>
