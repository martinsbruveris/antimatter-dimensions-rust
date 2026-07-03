<script setup>
// Catch-up summary shown after Offline mode replays >= 10 s of accumulated
// game-time. Mirrors the original AwayProgressModal.vue
// (../antimatter-dimensions/src/components/modals): a "While you were away
// for {time}:" header followed by one resource line per gain (pre-Infinity,
// just Antimatter) in the same "increased from {before} to {after}" wording.
import { computed } from "vue";

import { useUiStore } from "../stores/ui";
import { formatDecimal, formatTime } from "../util/format";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const ui = useUiStore();

const summary = computed(() => ui.offlineSummary);

// A snapshot number ({ m, e }) formatted like the original `format(n, 2, 2)`.
function fmt(num) {
  return formatDecimal(num, 2, 2);
}

// Whether antimatter actually changed; drives the "Nothing happened" wording
// and whether the resource line shows (matches the original's somethingHappened).
const antimatterChanged = computed(() => {
  const s = summary.value;
  if (!s) return false;
  const a = s.before.antimatter;
  const b = s.after.antimatter;
  return a.m !== b.m || a.e !== b.e;
});

const headerText = computed(() => {
  const s = summary.value;
  if (!s) return "";
  const time = formatTime(s.seconds * 1000);
  return antimatterChanged.value
    ? `While you were away for ${time}: `
    : `While you were away for ${time}... Nothing happened.`;
});
</script>

<template>
  <Modal
    compact
    fit-content
    @close="$emit('close')"
  >
    <div
      v-if="summary"
      class="c-modal-away-progress"
    >
      <div class="c-modal-away-progress__header">
        {{ headerText }}
      </div>
      <div
        v-if="antimatterChanged"
        class="c-modal-away-progress__resources"
      >
        <div class="c-modal-away-progress__antimatter">
          <b>Antimatter</b>
          increased from
          {{ fmt(summary.before.antimatter) }} to {{ fmt(summary.after.antimatter) }}
        </div>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
/* The original keeps the resource rows in a component-scoped block (not the
   vendored stylesheet); reproduce its underlined-row look. The vendored
   `.c-modal` centers text, but our Modal shell's `.c-modal-text` resets it to
   left — restore the original's centering here. */
.c-modal-away-progress {
  text-align: center;
}

.c-modal-away-progress__resources div {
  min-width: 35rem;
  border-bottom: 0.1rem solid var(--color-text, #cccccc);
  margin-bottom: 0.2rem;
  padding-bottom: 0.2rem;
}

.c-modal-away-progress__resources div:last-child {
  border: none;
}

/* Per-resource coloring from the original AwayProgressEntry's scoped styles
   (antimatter red + the dark-theme glow). */
.c-modal-away-progress__antimatter {
  color: var(--color-antimatter);
}

.t-dark .c-modal-away-progress__antimatter {
  animation: a-game-header__antimatter--glow 25s infinite;
}
</style>
