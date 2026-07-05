<script setup>
// The block palette pane (vendored from AutomatorBlocks.vue): drag these
// into the block editor on the left.
import { computed } from "vue";
import draggable from "vuedraggable";

import { useGameStore } from "../../../stores/game";
import { newBlockId, PALETTE_BLOCKS } from "../../../data/automatorBlocks";

const game = useGameStore();

const blocks = computed(() =>
  PALETTE_BLOCKS.filter((b) => {
    if (!b.unlock) return true;
    if (b.unlock === "blackHole") {
      return game.snapshot.reality.black_holes.unlocked;
    }
    if (b.unlock === "reality25") {
      return game.snapshot.reality.upgrades.some(
        (u) => u.id === 25 && u.is_bought,
      );
    }
    return false; // "enslaved" — celestial, never at our frontier
  }),
);

function clone(block) {
  const b = { ...block, id: newBlockId() };
  if (block.nested) b.nest = [];
  return b;
}
</script>

<template>
  <div class="o-drag-cancel-region">
    <p>
      Drag and drop these blocks to the area on the left! The blocks have names matching the commands in the reference
      page, but may change appearance after being placed to describe what they do in a more natural-sounding manner.
      If a block changes in this way, the alternate text will be shown as a tooltip when going to drag it over.
    </p>
    <br>
    <p>
      Inputs with a <span class="c-automator-input-optional">brown</span> color are optional, while inputs with a
      <span class="c-automator-input-required">teal</span> color are required.
      <span class="c-automator-block-row-error">Red</span> inputs are causing errors and must be changed before the
      script can be run. For more details, check the Scripting Information pane.
    </p>
    <p>
      Options in dropdown menus which start with a * will be replaced with a text box. This can be turned back into a
      dropdown by clicking the <i class="fa-solid fa-circle-xmark" /> on the right side of the text box.
    </p>
    <draggable
      class="block-container"
      :list="blocks"
      :group="{ name: 'code-blocks', pull: 'clone', put: false }"
      :sort="false"
      :clone="clone"
      item-key="cmd"
    >
      <template #item="{ element }">
        <div
          :title="element.alias"
          class="o-automator-command o-automator-block-list draggable-blocks"
        >
          {{ element.cmd }}
        </div>
      </template>
    </draggable>
    <p>
      Note: Blocks and their contents count towards the character limits as if the command was typed in text mode.
    </p>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorBlocks.vue scoped style. */
.block-container {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  margin: 1rem 0;
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

.o-drag-cancel-region {
  width: 100%;
  height: 100%;
}
</style>
