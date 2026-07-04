<script setup>
// Toggle button styled for the options modals, mirroring the original
// `ModalOptionsToggleButton.vue`: a PrimaryToggleButton in the fixed
// `o-primary-btn--modal-option` box, coloured green (on) / purple (off).
import { computed } from "vue";

import PrimaryToggleButton from "./PrimaryToggleButton.vue";

const props = defineProps({
  text: { type: String, required: true },
  modelValue: { type: Boolean, required: true },
});
const emit = defineEmits(["update:modelValue"]);

const styleObject = computed(() => ({
  "background-color": props.modelValue
    ? "var(--color-good)"
    : "var(--color-gh-purple)",
}));
</script>

<template>
  <PrimaryToggleButton
    class="o-primary-btn--modal-option"
    :style="styleObject"
    :label="text"
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
