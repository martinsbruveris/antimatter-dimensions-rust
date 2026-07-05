<script setup>
// "You are about to Reality" — the glyph selection modal
// (modals/prestige/RealityModal.vue). Shows the upcoming glyph choice(s);
// confirming performs the Reality with the selected glyph (Sacrifice sends
// the pick straight to sacrifice instead).
import { computed, ref } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";
import GlyphComponent from "./GlyphComponent.vue";
import { formatDecimal } from "../util/format";

const game = useGameStore();
const ui = useUiStore();
const reality = computed(() => game.snapshot?.reality);

const selected = ref(undefined);

const firstReality = computed(() => reality.value?.realities === 0);
const hasChoice = computed(() => (reality.value?.choice_count ?? 1) > 1);
const glyphs = computed(() => reality.value?.upcoming_glyphs ?? []);

const firstRealityText =
  "Reality will reset everything except Challenge records and anything under " +
  "the General header on the Statistics tab. The first 13 rows of Achievements " +
  "are also reset, but you will automatically get one Achievement back every " +
  "30 minutes. You will also gain Reality Machines based on your Eternity " +
  "Points, a Glyph with a level based on your Eternity Points, Replicanti, and " +
  "Dilated Time, a Perk Point to spend on quality of life upgrades, and unlock " +
  "various upgrades.";

const gainedText = computed(
  () =>
    `You will gain 1 Reality, 1 Perk Point, and ` +
    `${formatDecimal(reality.value.gained_rm, 2)} Reality Machines`
);

const levelStats = computed(() => {
  const r = reality.value;
  const diff = Math.abs(r.best_glyph_level - r.glyph_level);
  const comparison =
    r.glyph_level === r.best_glyph_level
      ? "equal to"
      : `${diff} ${diff === 1 ? "level" : "levels"} ${
          r.glyph_level > r.best_glyph_level ? "higher" : "lower"
        } than`;
  return `You will get a level ${r.glyph_level} Glyph on Reality, which is ${comparison} your best.`;
});

const warnText = computed(() => {
  if (!hasChoice.value && !firstReality.value) {
    return (
      "You currently only have a single option for new Glyphs every Reality. " +
      "You can unlock the ability to choose from multiple Glyphs by canceling " +
      "out of this modal and purchasing the START Perk."
    );
  }
  return selected.value === undefined && !firstReality.value
    ? "You must select a Glyph in order to continue."
    : null;
});

const canConfirm = computed(
  () => firstReality.value || selected.value !== undefined
);

function confirm(sacrifice) {
  if (!canConfirm.value) return;
  game.doReality(selected.value, sacrifice);
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="You are about to Reality"
    @confirm="confirm(false)"
    @close="ui.closeModal()"
  >
    <div
      v-if="firstReality"
      class="c-modal-message__text"
    >
      {{ firstRealityText }}
    </div>
    <div class="c-modal-message__text">
      {{ gainedText }}
    </div>
    <div
      v-if="!firstReality"
      class="l-glyph-selection__row"
    >
      <GlyphComponent
        v-for="(glyph, index) in glyphs"
        :key="index"
        class="l-modal-glyph-selection__glyph"
        :class="{ 'l-modal-glyph-selection__glyph--selected': selected === index }"
        :glyph="glyph"
        :show-sacrifice="reality.can_sacrifice"
        @clicked="selected = index"
      />
    </div>
    <div v-if="!firstReality">
      {{ levelStats }}
      <br>
      <b
        v-if="warnText"
        class="o-warning"
      >
        {{ warnText }}
      </b>
    </div>
    <div
      v-if="reality.can_sacrifice && canConfirm && !firstReality"
      class="l-modal-buttons"
    >
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="confirm(true)"
      >
        Sacrifice
      </button>
    </div>
  </ConfirmModal>
</template>

<style scoped>
.l-glyph-selection__row {
  display: flex;
  flex-direction: row;
  justify-content: center;
  gap: 1.5rem;
  margin: 1.5rem 0;
}

.l-modal-glyph-selection__glyph {
  cursor: pointer;
}

.l-modal-glyph-selection__glyph--selected {
  outline: 0.3rem solid var(--color-reality-light, #0b600e);
  outline-offset: 0.3rem;
}

.o-warning {
  color: var(--color-infinity);
}
</style>
