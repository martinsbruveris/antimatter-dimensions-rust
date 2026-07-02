<script setup>
// "Choose save" popup, opened from the Saving options tab. Mirrors the original
// game's LoadGameModal.vue + LoadGameEntry.vue
// (../antimatter-dimensions/src/components/modals) — the "Save Selection" header
// and the three save-slot records with their Load buttons. Wired to the engine's
// on-disk save slots: it fetches per-slot summaries on open and switches the
// active slot on Load. Renders inside our shared Modal.vue wrapper.
import { onMounted, ref } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { formatDecimal } from "../util/format";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();

// Per-slot summaries: { id, exists, antimatter: {m,e}, is_current }.
const slots = ref([]);

async function refresh() {
  slots.value = await game.getSaveSlots();
}

onMounted(refresh);

async function loadSlot(slot) {
  if (slot.is_current) return;
  await game.switchSaveSlot(slot.id);
  ui.notify("Game loaded");
  ui.closeModal();
}
</script>

<template>
  <Modal
    title="Save Selection"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div
      v-for="slot in slots"
      :key="slot.id"
      class="l-modal-options__save-record c-entry-border"
    >
      <h3>Save #{{ slot.id + 1 }}:<span v-if="slot.is_current"> (selected)</span></h3>
      <span v-if="slot.save_file_name">File name: {{ slot.save_file_name }}</span>
      <span v-if="slot.exists">Antimatter: {{ formatDecimal(slot.antimatter, 2, 1) }}</span>
      <span v-else>Empty save</span>
      <button
        class="o-primary-btn o-primary-btn--width-medium"
        :class="{ 'o-primary-btn--disabled': slot.is_current }"
        @click="loadSlot(slot)"
      >
        {{ slot.is_current ? "Selected" : "Load" }}
      </button>
    </div>
  </Modal>
</template>

<style scoped>
/* Replicated from the original LoadGameModal.vue's <style scoped> block (these
   classes are not part of the vendored global CSS). */
.c-entry-border {
  width: 28rem;
  padding-bottom: 1rem;
  border-bottom: 0.1rem solid var(--color-text);
}

.c-entry-border:last-child {
  padding-bottom: 0;
  border-bottom: none;
}
</style>
