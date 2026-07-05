<script setup>
// The CodeMirror-based script editor (vendored behavior from
// AutomatorTextEditor.vue): adopts the session-long editor singleton, swaps
// per-script documents, and follows the running line with the active-line
// highlight.
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { useGameStore } from "../../../stores/game";
import { AutomatorTextUI } from "../../../util/automatorEditor";

const game = useGameStore();
const container = ref(null);
const auto = computed(() => game.snapshot.automator);
const editorScript = computed(() => auto.value.editor_script);

async function openCurrentScript() {
  const id = editorScript.value;
  const content = await game.getAutomatorScript(id);
  AutomatorTextUI.openScript(id, content);
}

watch(editorScript, async (_, oldId) => {
  AutomatorTextUI.updateHighlightedLine(-1, "active");
  await openCurrentScript();
  // A deleted script's stale document is dropped.
  if (!auto.value.scripts.some((s) => s.id === oldId)) {
    AutomatorTextUI.dropDocument(oldId);
  }
});

// Track the executing line each frame: highlight it when the editor shows the
// running script, and optionally scroll to follow.
watch(
  () => [auto.value.current_line, auto.value.is_on, auto.value.top_level_script],
  ([line, isOn, runningId]) => {
    if (!AutomatorTextUI.editor) return;
    if (isOn && runningId === editorScript.value) {
      AutomatorTextUI.updateHighlightedLine(line, "active");
      if (auto.value.follow_execution && auto.value.is_running) {
        AutomatorTextUI.scrollToLine(line);
      }
    } else {
      AutomatorTextUI.updateHighlightedLine(-1, "active");
    }
  },
);

onMounted(async () => {
  AutomatorTextUI.initialize();
  container.value.appendChild(AutomatorTextUI.container);
  await openCurrentScript();
  AutomatorTextUI.editor.refresh();
});

onBeforeUnmount(() => {
  AutomatorTextUI.clearAllHighlights();
  if (AutomatorTextUI.container?.parentNode === container.value) {
    container.value.removeChild(AutomatorTextUI.container);
  }
});
</script>

<template>
  <div
    ref="container"
    class="c-automator-editor l-automator-editor l-automator-pane__content"
  />
</template>
