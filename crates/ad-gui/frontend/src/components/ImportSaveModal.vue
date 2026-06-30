<script setup>
// "Import save" popup, opened from the Saving options tab. Mirrors the original
// game's ImportSaveModal.vue (../antimatter-dimensions/src/components/modals) —
// same title, input box and Cancel button. Save parsing/validation is not wired
// up yet, so only the empty entry state is shown (the original reveals save
// details and an Import button once a valid save string is typed). Renders
// inside our shared Modal.vue wrapper.
import { ref } from "vue";

import Modal from "./Modal.vue";

defineEmits(["close"]);

const input = ref("");
</script>

<template>
  <Modal
    title="Input your save"
    compact
    fit-content
    @close="$emit('close')"
  >
    <input
      v-model="input"
      type="text"
      class="c-modal-input c-modal-import__input"
    >
    <div class="c-modal-import__save-info" />
    <div class="l-modal-buttons">
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="$emit('close')"
      >
        Cancel
      </button>
    </div>
  </Modal>
</template>
