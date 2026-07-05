<script setup>
// Threshold input for the prestige autobuyers, a port of the original
// AutobuyerInput.vue. The box shows the engine's value (formatted in
// scientific notation regardless of the chosen display notation, like the
// original's `Notation.scientific.format(value, 2, 2)`) until focused; while
// editing, local validity is checked with the original's format patterns and
// the raw string is committed to the engine on change.
import { computed, ref, watch } from "vue";

import { format as wasmFormat } from "../../../wasm/ad_format.js";

const props = defineProps({
  // Current engine value: { m, e } for "decimal", a plain number otherwise.
  value: { type: [Object, Number], required: true },
  // "decimal" | "float" | "int" (the original's AutobuyerInputFunctions keys).
  type: { type: String, required: true },
});

// The parent commits the string to the engine (and may reject it).
const emit = defineEmits(["commit"]);

const isFocused = ref(false);
const isValid = ref(true);
const displayValue = ref("");

function formatValue() {
  if (props.type === "decimal") {
    // Scientific, 2 places, matching the original's input formatting.
    return wasmFormat(props.value.m, props.value.e, "Scientific", 2, 2, 5, 9, false);
  }
  return `${props.value}`;
}

function syncFromEngine() {
  if (isFocused.value) return;
  displayValue.value = formatValue();
  isValid.value = true;
}

watch(() => props.value, syncFromEngine, { immediate: true, deep: true });

// The original's tryParse patterns (commas stripped): plain / scientific /
// logarithm ("e30") / mixed scientific ("2.33e41.2") for decimals.
function checkValid(input) {
  const s = input.replaceAll(",", "");
  if (s.length === 0) return false;
  switch (props.type) {
    case "decimal":
      return (
        /^e\d*[.]?\d+$/u.test(s) ||
        /^\d*[.]?\d+(e\d*[.]?\d+)?$/u.test(s)
      );
    case "float":
      return !isNaN(parseFloat(s));
    case "int":
      return /^\d+$/u.test(s);
    default:
      return false;
  }
}

const validityClass = computed(() => (isValid.value ? undefined : "o-autobuyer-input--invalid"));
const inputType = computed(() => (props.type === "int" ? "number" : "text"));

function handleInput(event) {
  displayValue.value = event.target.value;
  isValid.value = checkValid(displayValue.value);
}

async function handleChange(event) {
  if (isValid.value) {
    emit("commit", displayValue.value.replaceAll(",", ""));
  }
  isFocused.value = false;
  isValid.value = true;
  syncFromEngine();
  event.target.blur();
}
</script>

<template>
  <input
    :value="displayValue"
    :class="validityClass"
    :type="inputType"
    class="o-autobuyer-input"
    @change="handleChange"
    @focus="isFocused = true"
    @input="handleInput"
  >
</template>

<style scoped>
.o-autobuyer-input--invalid {
  background-color: var(--color-bad);
}
</style>
