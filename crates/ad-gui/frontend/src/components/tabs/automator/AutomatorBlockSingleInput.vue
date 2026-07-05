<script setup>
// One input in a block row (vendored behavior from
// AutomatorBlockSingleInput.vue): a fixed label, a boolean toggle
// (NOWAIT/RESPEC), or a dropdown that can flip into a text box (options
// starting with `*`), chaining recursively along the block's allowed
// patterns.
import { computed, ref, watch } from "vue";

const props = defineProps({
  // Fixed label (the command name); when set, nothing is editable.
  constant: { type: String, default: "" },
  block: { type: Object, required: true },
  blockTarget: { type: String, default: "" },
  // Commit changes upward (triggers reparse + save).
  changed: { type: Function, required: true },
  initialSelection: { type: String, default: "" },
  patterns: { type: Array, default: () => [] },
  recursive: { type: Boolean, default: false },
  currentPath: { type: String, default: "" },
  hasError: { type: Boolean, default: false },
});

const isBoolTarget = computed(
  () => props.blockTarget === "nowait" || props.blockTarget === "respec",
);

// Build the dropdown options for this node of the pattern path.
const pathRef = {};
const dropdownOptions = ref([]);
const dropdownSelection = ref("");
const isTextInput = ref(false);
const textContents = ref("");

function initialize() {
  dropdownOptions.value = [];
  if (props.constant) return;
  if (isBoolTarget.value) {
    dropdownOptions.value = [props.blockTarget.toUpperCase()];
    dropdownSelection.value = props.block[props.blockTarget]
      ? props.blockTarget.toUpperCase()
      : "";
    return;
  }
  if (props.recursive) {
    const nodes = props.patterns
      .filter(
        (s) =>
          s.startsWith(props.currentPath) && s.length > props.currentPath.length,
      )
      .map((s) => s.charAt(props.currentPath.length));
    for (const node of nodes) {
      if (pathRef[node]) continue;
      const entries = props.block[node] ?? [];
      pathRef[node] = entries;
      dropdownOptions.value.push(...entries);
    }
  }
  if (dropdownOptions.value.includes(props.initialSelection)) {
    dropdownSelection.value = props.initialSelection;
  } else if (props.initialSelection) {
    isTextInput.value = true;
    textContents.value = props.initialSelection;
  }
  // Text-only fields (single `*` option) start as text boxes.
  if (
    dropdownOptions.value.length === 1 &&
    dropdownOptions.value[0].startsWith("*")
  ) {
    isTextInput.value = true;
    textContents.value = props.initialSelection;
  }
}
initialize();

watch(dropdownSelection, (v) => {
  if (v.startsWith("*")) {
    isTextInput.value = true;
    textContents.value = "";
  }
});

const displayedConstant = computed(() => {
  if (props.constant) {
    return props.constant === "BLOB" ? "" : props.constant;
  }
  return dropdownOptions.value.length === 1 &&
    !isBoolTarget.value &&
    !isTextInput.value
    ? dropdownOptions.value[0]
    : "";
});

// Which pattern node the current selection sits on, and whether more inputs
// follow it.
const currentNode = computed(() => {
  let node = " ";
  for (const key of Object.keys(pathRef)) {
    const isValidText =
      pathRef[key].some((o) => o.startsWith("*")) && isTextInput.value;
    if (pathRef[key].includes(dropdownSelection.value) || isValidText) {
      node = key;
    }
  }
  return node;
});

const nextNodeCount = computed(() => {
  const fullPath = props.currentPath + currentNode.value;
  return props.patterns.filter(
    (p) => p.length > fullPath.length && p.startsWith(fullPath),
  ).length;
});

const unknownNext = computed(
  () =>
    nextNodeCount.value > 1 ||
    (dropdownSelection.value === "" && !isTextInput.value),
);

const targetIndex = computed(() =>
  props.block.targets ? props.block.targets.indexOf(props.blockTarget) : -1,
);
const nextInputKey = computed(
  () => props.block.targets?.[targetIndex.value + 1],
);
const nextInputValue = computed(() => {
  const value = nextInputKey.value ? props.block[nextInputKey.value] : "";
  return value ? `${value}` : "";
});

const hasLongTextInput = computed(
  () => props.block.cmd === "NOTIFY" || props.block.cmd === "COMMENT",
);

function commit() {
  if (props.blockTarget) {
    let newValue;
    if (isBoolTarget.value) newValue = dropdownSelection.value !== "";
    else if (isTextInput.value) newValue = textContents.value;
    else newValue = dropdownSelection.value;
    props.block[props.blockTarget] = newValue;
    // A structure change can orphan later inputs on the line; clear them.
    if (nextNodeCount.value === 0 && !isBoolTarget.value && props.block.targets) {
      for (
        let toClear = targetIndex.value + 1;
        toClear < props.block.targets.length;
        toClear++
      ) {
        props.block[props.block.targets[toClear]] = undefined;
      }
    }
  }
  props.changed();
}

function revertToDropdown() {
  isTextInput.value = false;
  dropdownSelection.value = "";
  textContents.value = "";
  commit();
}

const textInputClass = computed(() => ({
  "o-automator-block-input": true,
  "o-long-text-input": hasLongTextInput.value,
  "l-error-textbox": props.hasError,
  "c-automator-input-required": !props.hasError,
}));

const dropdownClass = computed(() => ({
  "o-automator-block-input": true,
  "c-automator-input-required": !isBoolTarget.value,
  "c-automator-input-optional": isBoolTarget.value,
  "l-error-textbox":
    props.hasError && !isBoolTarget.value && dropdownSelection.value === "",
}));
</script>

<template>
  <div class="c-automator-single-block">
    <div
      v-if="displayedConstant"
      class="c-automator-single-block o-automator-command c-automator-constant-block"
      :class="{ 'l-blob': constant === 'BLOB' }"
    >
      {{ displayedConstant }}
    </div>
    <div
      v-else-if="isTextInput"
      class="c-automator-text-input-container"
    >
      <input
        v-model="textContents"
        :class="textInputClass"
        @keyup="commit"
      >
      <div
        v-if="dropdownOptions.length > 1"
        class="c-automator-close-text-input fa-solid fa-circle-xmark"
        @click="revertToDropdown"
      />
    </div>
    <select
      v-else
      v-model="dropdownSelection"
      :class="dropdownClass"
      @change="commit"
    >
      <option
        v-for="target in ['', ...dropdownOptions]"
        :key="target"
        :value="target"
      >
        {{ target }}
      </option>
    </select>
    <AutomatorBlockSingleInput
      v-if="recursive && nextNodeCount > 0 && nextInputKey"
      :key="currentNode"
      :constant="unknownNext ? '...' : ''"
      :block="block"
      :block-target="nextInputKey"
      :patterns="patterns"
      :initial-selection="nextInputValue"
      :changed="changed"
      :recursive="true"
      :current-path="currentPath + currentNode"
      :has-error="hasError"
    />
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorBlockSingleInput.vue scoped style. */
.c-automator-single-block {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  height: 2.8rem;
  white-space: nowrap;
}

.c-automator-constant-block {
  background: var(--color-blockmator-block-command);
  color: var(--color-blockmator-editor-background);
}

.c-automator-text-input-container {
  position: relative;
}

.o-long-text-input {
  width: 30rem;
}

.c-automator-close-text-input {
  position: absolute;
  color: var(--color-automator-error-outline);
  font-size: 1.5rem;
  z-index: 1;
  right: 0.8rem;
  top: 0.6rem;
}

.l-error-textbox {
  background: var(--color-automator-error-background);
  color: yellow;
}

.l-blob {
  font-size: 1.8rem;
  background: black;
  color: #fc2;
}
</style>
