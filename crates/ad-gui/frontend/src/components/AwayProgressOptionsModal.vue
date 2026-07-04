<script setup>
// Away Progress Options popup, opened from the Visual options tab. Mirrors the
// original AwayProgressOptionsModal.vue (../antimatter-dimensions/src/
// components/modals/options): one toggle per resource that may appear in the
// "While you were away" summary shown after an offline catch-up. Only the
// in-frontier resources are listed (the original has ~24); each appears once
// its mechanic is unlocked, matching the original's per-type isUnlocked().
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import Modal from "./Modal.vue";
import ModalOptionsToggleButton from "./options/ModalOptionsToggleButton.vue";

defineEmits(["close"]);

const game = useGameStore();

// The modelled away-progress resources: engine option key (camelCase, what
// set_away_progress expects), snapshot field, display label, and unlock gate.
const ENTRIES = [
  { kind: "antimatter", field: "antimatter", label: "Antimatter:" },
  { kind: "dimensionBoosts", field: "dimension_boosts", label: "Dimension Boosts:" },
  {
    kind: "antimatterGalaxies",
    field: "antimatter_galaxies",
    label: "Antimatter Galaxies:",
  },
  {
    kind: "infinities",
    field: "infinities",
    label: "Infinities:",
    condition: (s) => Boolean(s?.infinity_unlocked),
  },
  {
    kind: "infinityPoints",
    field: "infinity_points",
    label: "Infinity Points:",
    condition: (s) => Boolean(s?.infinity_unlocked),
  },
  {
    kind: "replicanti",
    field: "replicanti",
    label: "Replicanti:",
    condition: (s) => Boolean(s?.replicanti?.unlocked),
  },
  {
    kind: "replicantiGalaxies",
    field: "replicanti_galaxies",
    label: "Replicanti Galaxies:",
    condition: (s) => Boolean(s?.replicanti?.unlocked),
  },
];

const visibleEntries = computed(() =>
  ENTRIES.filter((e) => !e.condition || e.condition(game.snapshot))
);

const options = computed(() => game.snapshot?.options?.away_progress ?? {});
</script>

<template>
  <Modal
    title="Away Progress Options"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="l-wrapper l-modal-options">
      <div class="c-modal-options__button-container">
        <ModalOptionsToggleButton
          v-for="entry in visibleEntries"
          :key="entry.kind"
          :text="entry.label"
          :model-value="Boolean(options[entry.field])"
          @update:model-value="game.setAwayProgress(entry.kind, $event)"
        />
      </div>
      Note: Selected resources will only show if they've increased.
    </div>
  </Modal>
</template>

<style scoped>
/* The original sizes this modal wider than the standard options modal
   (its scoped .l-wrapper { width: 75rem }); our 3-per-row toggle grid fits
   comfortably in the c-modal-options__large width instead. */
.l-wrapper {
  width: 55rem;
}
</style>
