<script setup>
// Labelled on/off toggle button, mirroring the original
// `components/PrimaryToggleButton.vue`: it shows "<label> <on|off>" and flips
// the bound value on click. Sizing/placement classes (e.g.
// `o-primary-btn--option l-options-grid__button`) are passed by the parent and
// fall through onto the root button.
import { computed } from "vue";

const props = defineProps({
  label: { type: String, default: "" },
  on: { type: String, default: "ON" },
  off: { type: String, default: "OFF" },
  modelValue: { type: Boolean, required: true },
});
const emit = defineEmits(["update:modelValue"]);

const displayText = computed(() =>
  `${props.label} ${props.modelValue ? props.on : props.off}`.trim()
);
</script>

<template>
  <button
    class="o-primary-btn"
    @click="emit('update:modelValue', !modelValue)"
  >
    {{ displayText }}
  </button>
</template>
