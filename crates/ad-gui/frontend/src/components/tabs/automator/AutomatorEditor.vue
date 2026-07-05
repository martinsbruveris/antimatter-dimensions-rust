<script setup>
// The left pane: controls bar + the text or block editor (vendored from
// AutomatorEditor.vue). Switching to a script whose text has unparsable
// commands while in block mode falls back to the text editor, keeping the
// stored content intact (the original's game-load/script-switch guard).
import { computed, onMounted, watch } from "vue";

import { useGameStore } from "../../../stores/game";
import { blockSwitchMessage } from "../../../util/blockAutomator";
import AutomatorBlockEditor from "./AutomatorBlockEditor.vue";
import AutomatorControls from "./AutomatorControls.vue";
import AutomatorTextEditor from "./AutomatorTextEditor.vue";

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);
const isTextAutomator = computed(() => auto.value.editor_type === "text");

async function checkUnparsable() {
  if (isTextAutomator.value) return;
  const blockified = await game.automatorBlockify(auto.value.editor_script);
  if (blockified.lost_lines > 0) {
    blockSwitchMessage.value =
      "Some incomplete blocks were unrecognizable - defaulting to text editor.";
    // Switch the flavor only — never save the lossy block conversion.
    await game.automatorSetEditorType(false);
  }
}

onMounted(checkUnparsable);
watch(() => auto.value.editor_script, checkUnparsable);
</script>

<template>
  <div class="l-automator-pane">
    <AutomatorControls />
    <AutomatorTextEditor v-if="isTextAutomator" />
    <AutomatorBlockEditor v-if="!isTextAutomator" />
  </div>
</template>
