<script setup>
// The block editor (vendored from AutomatorBlockEditor.vue): a line-number
// gutter next to the draggable block list. Blocks load from the stored
// script text via the engine blockifier; every change regenerates the text
// and saves it.
import { computed, onMounted, watch } from "vue";
import draggable from "vuedraggable";

import { useGameStore } from "../../../stores/game";
import {
  blockIdAtLine,
  blockLines,
  loadBlocksFromScript,
  saveBlocksToScript,
} from "../../../util/blockAutomator";
import { numberOfLinesInBlock } from "../../../data/automatorBlocks";
import AutomatorBlockSingleRow from "./AutomatorBlockSingleRow.vue";

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);
const editorScript = computed(() => auto.value.editor_script);

const numberOfLines = computed(() =>
  blockLines.value.reduce((a, b) => a + numberOfLinesInBlock(b), 0),
);

// The running command's block, for the active-row highlight.
const activeBlockId = computed(() => {
  if (!auto.value.is_on || auto.value.top_level_script !== editorScript.value) {
    return -1;
  }
  return blockIdAtLine(auto.value.current_line) ?? -1;
});

async function load() {
  await loadBlocksFromScript(game, editorScript.value);
}

function changed() {
  saveBlocksToScript(game, editorScript.value);
}

function deleteBlock(id) {
  const idx = blockLines.value.findIndex((x) => x.id === id);
  if (idx !== -1) blockLines.value.splice(idx, 1);
  changed();
}

onMounted(load);
watch(editorScript, load);
</script>

<template>
  <div class="c-automator-block-editor--container l-automator-pane__content">
    <div class="c-automator-block-editor--gutter">
      <div
        v-for="i in numberOfLines"
        :key="i"
        class="c-automator-block-line-number"
        :style="{ top: `${(i - 1) * 3.45}rem` }"
      >
        {{ i }}
      </div>
    </div>
    <div class="c-automator-block-editor">
      <draggable
        :list="blockLines"
        group="code-blocks"
        class="c-automator-blocks"
        ghost-class="c-automator-block-row-ghost"
        item-key="id"
        @change="changed"
      >
        <template #item="{ element }">
          <AutomatorBlockSingleRow
            :block="element"
            :changed="changed"
            :delete-block="deleteBlock"
            :active-block-id="activeBlockId"
          />
        </template>
      </draggable>
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorBlockEditor.vue scoped style. */
.c-automator-block-editor {
  display: flex;
  overflow-y: auto;
  tab-size: 1.5rem;
  width: 100%;
  background-color: var(--color-blockmator-editor-background);
  box-sizing: content-box;
}

.c-automator-block-editor--container {
  display: flex;
  overflow-y: hidden;
  height: 100%;
  position: relative;
  box-sizing: border-box;
}

.c-automator-blocks {
  width: 100%;
  height: max-content;
  padding: 0.3rem 0.6rem 5rem;
}

.c-automator-block-editor--gutter {
  height: max-content;
  min-height: 100%;
  position: relative;
  background-color: var(--color-automator-controls-background);
  border-right: 0.1rem solid #505050;
  padding: 0.3rem 1rem 20rem;
}

.c-automator-block-line-number {
  display: flex;
  height: 3.45rem;
  justify-content: flex-end;
  align-items: center;
  font-size: 1.4rem;
  color: var(--color-automator-docs-font);
}
</style>
