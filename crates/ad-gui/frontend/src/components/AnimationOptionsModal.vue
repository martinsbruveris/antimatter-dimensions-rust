<script setup>
// Animation Options popup, opened from the Visual options tab. Mirrors the
// original AnimationOptionsModal.vue (../antimatter-dimensions/src/components/
// modals/options): a container of per-animation toggles, each shown once its
// prestige layer is reached. Only the Big Crunch animation is in-frontier
// (gated on Infinity being unlocked); the eternity/dilation/reality toggles
// arrive with those layers. The animation itself is a follow-up — this stores
// the flag the crunch animation will honour.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import Modal from "./Modal.vue";
import ModalOptionsToggleButton from "./options/ModalOptionsToggleButton.vue";

defineEmits(["close"]);

const game = useGameStore();

const infinityUnlocked = computed(() =>
  Boolean(game.snapshot?.infinity_unlocked)
);
const bigCrunch = computed(
  () => game.snapshot?.options?.animations?.big_crunch ?? true
);
</script>

<template>
  <Modal
    title="Animation Options"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="c-modal-options__large l-modal-options">
      <div class="c-modal-options__button-container">
        <ModalOptionsToggleButton
          v-if="infinityUnlocked"
          text="Big Crunch:"
          :model-value="bigCrunch"
          @update:model-value="game.setAnimation('bigCrunch', $event)"
        />
        <div v-else>
          No toggleable animations are available yet.
        </div>
      </div>
    </div>
  </Modal>
</template>
