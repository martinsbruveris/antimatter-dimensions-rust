<script setup>
// The templates pane (vendored from AutomatorDocsTemplateList.vue): buttons
// opening the template prompt, plus — in block mode — the custom template
// blocks created this session, draggable into the editor where they unpack
// into their individual blocks.
import { computed, ref } from "vue";
import draggable from "vuedraggable";

import { useGameStore } from "../../../stores/game";
import { AUTOMATOR_TEMPLATES } from "../../../data/automatorTemplates";
import { hydrateBlock, newBlockId } from "../../../data/automatorBlocks";
import {
  blockLines,
  blockTemplates,
  saveBlocksToScript,
} from "../../../util/blockAutomator";

const emit = defineEmits(["show-template"]);

const game = useGameStore();
const isBlock = computed(
  () => game.snapshot.automator.editor_type === "block",
);
const selectedTemplateId = ref(-1);

const pasteText = computed(() =>
  isBlock.value
    ? `create a special block you can drag into your Automator where you would like it to be placed. It will then
      automatically fill in all of the individual blocks needed for the template`
    : `copy the template as text onto your clipboard. You can directly paste the template text into your Automator
      wherever you would like it`,
);

// The dragged clone is a placeholder (no cmd — parseLines skips it); the
// end handler replaces it with the template's hydrated blocks.
function clone(template) {
  return { ...template, id: newBlockId() };
}

function unpackTemplateBlocks(event) {
  const template = blockTemplates.value[selectedTemplateId.value];
  const beforeBlocks = blockLines.value.slice(0, event.newIndex);
  const afterBlocks = blockLines.value
    .slice(event.newIndex)
    .filter((b) => b.cmd);
  const blocksToAdd = template.blocks.map(hydrateBlock);
  blockLines.value = [...beforeBlocks, ...blocksToAdd, ...afterBlocks];
  saveBlocksToScript(game, game.snapshot.automator.editor_script);
}
</script>

<template>
  <div>
    These templates will let you do some more common things within the Automator. They may be slightly slower than
    manually-written scripts, but don't require you to have any previous programming experience to use. Clicking any
    of these buttons will open up a prompt with some input fields, which will generate a template you can place into
    your Automator.
    <button
      v-for="template in AUTOMATOR_TEMPLATES"
      :key="template.name"
      class="o-primary-btn c-automator-docs-template--button l-automator__button"
      @click="emit('show-template', template)"
    >
      Template: {{ template.name }}
    </button>
    Since you are currently in the {{ isBlock ? "Block" : "Text" }} editor, this panel will {{ pasteText }}.
    <br>
    <br>
    <draggable
      v-if="isBlock"
      :key="blockTemplates.length"
      class="template-container"
      :list="blockTemplates"
      :group="{ name: 'code-blocks', pull: 'clone', put: false }"
      :sort="false"
      :clone="clone"
      item-key="name"
      @end="unpackTemplateBlocks"
    >
      <template #item="{ element, index }">
        <div
          class="o-automator-command o-automator-block-list draggable-blocks"
          @dragstart="selectedTemplateId = index"
          @pointerdown="selectedTemplateId = index"
        >
          {{ element.name }}
        </div>
      </template>
    </draggable>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDocsTemplateList.vue scoped style. */
.c-automator-docs-template--button {
  margin: 0.4rem;
  border-radius: var(--var-border-radius, 0.4rem);
  border-width: var(--var-border-width, 0.2rem);
  cursor: pointer;
}

.template-container {
  display: flex;
  flex-direction: column;
}

.o-automator-block-list {
  display: flex;
  width: 8.7rem;
  text-align: center;
  height: 5.5rem;
  font-size: 1.2rem;
  justify-content: center;
  align-items: center;
}
</style>
