<script setup>
// "Choose save" popup, opened from the Saving options tab. Mirrors the original
// game's LoadGameModal.vue + LoadGameEntry.vue
// (../antimatter-dimensions/src/components/modals) — the "Save Selection" header
// and the three save-slot records with their Load buttons. Save data is not
// wired up yet, so the three slots show the empty/new-save state (antimatter 10,
// slot #1 selected). Renders inside our shared Modal.vue wrapper.
import Modal from "./Modal.vue";

defineEmits(["close"]);

// Static placeholder slots until save slots are wired up. The original defaults
// an empty slot's antimatter to 10 and marks the current slot "(selected)".
const saves = [
  { id: 0, selected: true },
  { id: 1, selected: false },
  { id: 2, selected: false },
];
</script>

<template>
  <Modal
    title="Save Selection"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div
      v-for="save in saves"
      :key="save.id"
      class="l-modal-options__save-record c-entry-border"
    >
      <h3>Save #{{ save.id + 1 }}:<span v-if="save.selected"> (selected)</span></h3>
      <span>Antimatter: 10</span>
      <button class="o-primary-btn o-primary-btn--width-medium">
        Load
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
