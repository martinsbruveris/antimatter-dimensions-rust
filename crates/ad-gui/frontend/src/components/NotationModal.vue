<script setup>
// Exponent Notation Settings popup, opened from the Visual options tab. Mirrors
// the original NotationModal.vue (../antimatter-dimensions/src/components/modals/
// options) — same text, two digit-threshold sliders, and live sample preview.
//
// The two thresholds control how a large *exponent* is rendered: below the comma
// threshold it shows plain, below the notation threshold it gets commas, and
// above it the exponent itself is formatted in notation. The sample preview uses
// the in-flight slider values so it updates while dragging (matching the
// original, which writes straight to the notations library's live Settings).
import { ref } from "vue";

import { useGameStore } from "../stores/game";
import { formatExponentSample } from "../util/format";
import Modal from "./Modal.vue";
import OptionsSlider from "./options/OptionsSlider.vue";

defineEmits(["close"]);

const game = useGameStore();

const commaDigits = ref(game.snapshot.options.notation_digits_comma);
const notationDigits = ref(game.snapshot.options.notation_digits_notation);

// The original's sample set: 10^(prefix of "123456789012345") for 4..15 digits,
// spanning plain, comma-grouped, and in-notation exponents. Each is a pure power
// of ten, so mantissa = 1 and only the exponent's formatting varies.
const SAMPLE_PREFIX = "123456789012345";
const sampleNums = [];
for (let digits = 4; digits < 16; digits++) {
  sampleNums.push({ m: 1, e: Number(SAMPLE_PREFIX.substring(0, digits)) });
}

// Persist the pair after every adjustment; the engine clamps and enforces the
// invariant, but we mirror the original's interactive behaviour locally so the
// sliders and preview stay consistent mid-drag.
function persist() {
  game.setNotationDigits(commaDigits.value, notationDigits.value);
}

// The notation threshold must stay >= the comma threshold (original invariant):
// raising commas past notation drags notation up; lowering notation past commas
// drags commas down.
function adjustSliderComma(value) {
  commaDigits.value = value;
  if (value > notationDigits.value) notationDigits.value = value;
  persist();
}
function adjustSliderNotation(value) {
  notationDigits.value = value;
  if (value < commaDigits.value) commaDigits.value = value;
  persist();
}
</script>

<template>
  <Modal
    title="Exponent Notation Settings"
    compact
    @close="$emit('close')"
  >
    You can adjust what your numbers look like when very large. With small values, the exponent will
    be directly displayed with no additional formatting. Larger values will have commas inserted into the exponent
    for clarity, and the largest values will apply notation formatting to the exponent in order to shorten it. You can
    adjust the two thresholds between these regions below:
    <br>
    <br>
    <div class="c-single-slider">
      <b class="o-digit-text">Minimum for commas in exponent: {{ commaDigits }} digits</b>
      <OptionsSlider
        class="o-slider"
        :min="3"
        :max="15"
        :interval="1"
        width="25rem"
        :model-value="commaDigits"
        @update:model-value="adjustSliderComma"
      />
    </div>
    <div class="c-single-slider">
      <b class="o-digit-text">Minimum for notation in exponent: {{ notationDigits }} digits</b>
      <OptionsSlider
        class="o-slider"
        :min="3"
        :max="15"
        :interval="1"
        width="25rem"
        :model-value="notationDigits"
        @update:model-value="adjustSliderNotation"
      />
    </div>
    <br>
    Sample numbers for exponent formatting:
    <div class="c-sample-numbers">
      <span
        v-for="(num, id) in sampleNums"
        :key="id"
        class="o-single-number"
      >
        {{ formatExponentSample(num, commaDigits, notationDigits) }}
      </span>
    </div>
    <br>
    Note: The interface is generally optimized for Scientific notation with settings of 5
    and 9 digits. Some text may look odd or overflow out of boxes if you
    differ significantly from these values. Additionally, these settings might not cause any visual changes
    when using certain notations.
  </Modal>
</template>

<style scoped>
/* Replicated from the original NotationModal.vue's <style scoped> block
   (these classes are not part of the vendored global CSS). */
.c-single-slider {
  display: flex;
  flex-direction: row;
  justify-content: space-around;
}

.o-digit-text {
  width: 40rem;
}

.o-slider {
  width: 25rem;
}

.c-sample-numbers {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  font-size: 1.5rem;
}

.o-single-number {
  width: 33%;
}
</style>
