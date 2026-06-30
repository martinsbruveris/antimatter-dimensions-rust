<script setup>
// "RESET THE GAME" confirmation popup, opened from the Saving options tab.
// Mirrors the original game's HardResetModal.vue
// (../antimatter-dimensions/src/components/modals/prestige) — same title, danger
// text, confirmation-phrase input and the phrase-gated HARD RESET button. The
// reveal of the confirm button when the phrase matches is pure presentation; no
// reset is actually wired up (the button just closes the modal). Renders inside
// our shared Modal.vue wrapper.
import { computed, ref } from "vue";

import Modal from "./Modal.vue";

defineEmits(["close"]);

const input = ref("");
const willHardReset = computed(() => input.value === "Shrek is love, Shrek is life");
</script>

<template>
  <Modal
    title="HARD RESET"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="c-modal-message__text">
      Please confirm your desire to hard reset this save slot.
      <span class="c-modal-hard-reset-danger">Deleting your save will not unlock anything secret.</span>
      Type in "Shrek is love, Shrek is life" to confirm.
      <div class="c-modal-hard-reset-danger">
        THIS WILL WIPE YOUR SAVE.
      </div>
    </div>
    <input
      v-model="input"
      type="text"
      class="c-modal-input c-modal-hard-reset__input"
    >
    <div class="c-modal-hard-reset-info">
      <div
        v-if="willHardReset"
        class="c-modal-hard-reset-danger"
      >
        Phrase confirmed - continuing will irreversibly delete your save!
      </div>
      <div v-else>
        Type in the correct phrase to hard reset.
      </div>
    </div>
    <div class="l-modal-buttons">
      <button
        v-if="!willHardReset"
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="$emit('close')"
      >
        Cancel
      </button>
      <button
        v-else
        class="o-primary-btn o-primary-btn--width-medium c-modal__confirm-btn c-modal-hard-reset-btn"
        @click="$emit('close')"
      >
        HARD RESET
      </button>
    </div>
  </Modal>
</template>
