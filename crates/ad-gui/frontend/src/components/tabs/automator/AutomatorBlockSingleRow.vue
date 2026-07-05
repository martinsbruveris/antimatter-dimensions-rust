<script setup>
// One block row (vendored from AutomatorBlockSingleRow.vue): the command
// label + its inputs + delete, and — for IF/WHILE/UNTIL — the nested
// drop zone.
import { computed } from "vue";
import draggable from "vuedraggable";

import { automatorErrors } from "../../../util/automatorEditor";
import { lineNumberOfBlock } from "../../../util/blockAutomator";
import AutomatorBlockSingleInput from "./AutomatorBlockSingleInput.vue";

const props = defineProps({
  block: { type: Object, required: true },
  // Reparse + save after any structural or input change.
  changed: { type: Function, required: true },
  deleteBlock: { type: Function, required: true },
  // The active (running) block id for highlighting.
  activeBlockId: { type: Number, default: -1 },
});

const lineNumber = computed(() => lineNumberOfBlock(props.block.id));
const hasError = computed(() =>
  automatorErrors.value.some((e) => e.line === lineNumber.value),
);
const highlightClass = computed(() => ({
  "c-automator-block-row-active": props.block.id === props.activeBlockId,
  "c-automator-block-row-error": hasError.value,
}));

const firstTargetValue = computed(() => {
  const value = props.block.targets
    ? props.block[props.block.targets[0]]
    : "";
  return value ? `${value}` : "";
});

function deleteFromNest(id) {
  const idx = props.block.nest.findIndex((x) => x.id === id);
  if (idx !== -1) props.block.nest.splice(idx, 1);
  props.changed();
}
</script>

<template>
  <div class="c-automator-block-row--container">
    <div
      class="c-automator-block-row"
      :class="highlightClass"
    >
      <AutomatorBlockSingleInput
        :constant="block.alias ? block.alias : block.cmd"
        :block="block"
        :changed="changed"
      />
      <AutomatorBlockSingleInput
        v-if="block.canWait"
        :block="block"
        block-target="nowait"
        :initial-selection="block.nowait ? 'NOWAIT' : ''"
        :changed="changed"
      />
      <AutomatorBlockSingleInput
        v-if="block.canRespec"
        :block="block"
        block-target="respec"
        :initial-selection="block.respec ? 'RESPEC' : ''"
        :changed="changed"
      />
      <AutomatorBlockSingleInput
        v-if="block.allowedPatterns"
        :block="block"
        :block-target="block.targets[0]"
        :patterns="block.allowedPatterns"
        :initial-selection="firstTargetValue"
        :changed="changed"
        :recursive="true"
        :has-error="hasError"
      />
      <div
        class="o-automator-block-delete"
        @click="deleteBlock(block.id)"
      >
        X
      </div>
    </div>
    <draggable
      v-if="block.nested"
      :list="block.nest"
      class="l-automator-nested-block"
      group="code-blocks"
      item-key="id"
      @change="changed"
    >
      <template #item="{ element }">
        <AutomatorBlockSingleRow
          :block="element"
          :changed="changed"
          :delete-block="deleteFromNest"
          :active-block-id="activeBlockId"
        />
      </template>
    </draggable>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorBlockSingleRow.vue scoped style. */
.c-automator-block-row--container {
  margin: -0.002rem;
  padding: 0.002rem;
}

.l-automator-nested-block {
  width: fit-content;
  min-width: 30rem;
  min-height: 3.65rem;
  border: 0.1rem dotted #55ff55;
  margin: -0.1rem 0 -0.1rem 3rem;
  padding: 0 0.5rem;
}
</style>
