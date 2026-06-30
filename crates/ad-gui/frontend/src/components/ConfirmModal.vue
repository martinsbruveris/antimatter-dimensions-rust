<script setup>
// Shared shell for the prestige confirmation modals (Dimension Boost, Galaxy,
// Sacrifice, Big Crunch). Mirrors the original's ModalWrapperChoice: a header,
// an explanatory body (slot), an optional "Don't show again" checkbox, and
// Cancel / Confirm buttons. Built on our generic Modal.vue rather than the
// original's EventHub-based wrapper.
import Modal from "./Modal.vue";
import ModalConfirmationCheck from "./ModalConfirmationCheck.vue";

defineProps({
  title: { type: String, required: true },
  // Confirmation kind (camelCase) for the disable checkbox, or null to omit it
  // (the first-infinity Big Crunch modal has nothing to disable yet).
  option: { type: String, default: null },
});

const emit = defineEmits(["confirm", "close"]);
</script>

<template>
  <Modal
    :title="title"
    compact
    centered
    @close="emit('close')"
  >
    <div class="c-modal-message__text">
      <slot />
    </div>
    <ModalConfirmationCheck
      v-if="option"
      :option="option"
    />
    <div class="l-modal-buttons">
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="emit('close')"
      >
        Cancel
      </button>
      <button
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn c-modal__confirm-btn"
        @click="emit('confirm')"
      >
        Confirm
      </button>
    </div>
  </Modal>
</template>
