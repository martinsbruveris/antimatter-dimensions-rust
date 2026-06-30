<script setup>
// "Import save" popup, opened from the Saving options tab. Mirrors the original
// game's ImportSaveModal.vue (../antimatter-dimensions/src/components/modals).
// The user pastes an AD save string into the text input; clicking Import
// decodes it via the Rust engine and replaces the running game state.
import { ref } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();
const input = ref("");
const error = ref("");
const importing = ref(false);

async function doImport() {
  if (!input.value.trim()) return;
  importing.value = true;
  error.value = "";
  try {
    await game.importSave(input.value);
    ui.notify("Save imported successfully");
    ui.closeModal();
  } catch (e) {
    error.value = typeof e === "string" ? e : e.message || "Import failed";
  } finally {
    importing.value = false;
  }
}
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
      placeholder="Paste your save here"
      @keyup.enter="doImport"
    >
    <div
      v-if="error"
      class="c-modal-import__error"
    >
      {{ error }}
    </div>
    <div class="l-modal-buttons">
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        :disabled="!input.trim() || importing"
        @click="doImport"
      >
        Import
      </button>
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="$emit('close')"
      >
        Cancel
      </button>
    </div>
  </Modal>
</template>

<style scoped>
.c-modal-import__error {
  color: var(--color-bad, #e74c3c);
  margin-top: 0.5rem;
  font-size: 1.2rem;
}
</style>
