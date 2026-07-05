<script setup>
// The Automator tab (vendored from AutomatorTab.vue): the interval line and
// character counters over a two-pane split — editor left, docs right. Locked,
// it shows the AP progress page. The original's draggable SplitPane is a
// fixed 50/50 split here (a noted Stage D deviation); full-screen mode is
// deferred with it.
import { computed, ref } from "vue";

import Modal from "../Modal.vue";
import { useGameStore } from "../../stores/game";
import { AutomatorTextUI } from "../../util/automatorEditor";
import AutomatorDocs from "./automator/AutomatorDocs.vue";
import AutomatorEditor from "./automator/AutomatorEditor.vue";
import AutomatorPointsList from "./automator/AutomatorPointsList.vue";

const MAX_SCRIPT_CHARS = 10000;
const MAX_TOTAL_CHARS = 60000;

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);
const deleteScriptId = ref(null);

const intervalText = computed(() => {
  const interval = auto.value.interval_ms;
  if (interval === 1) {
    return "The Automator is running at max speed (1000 commands per real-time second).";
  }
  const perSecond = (1000 / interval).toFixed(2);
  return `The Automator is running ${perSecond} commands per real-time second. ` +
    "Each Reality makes it run 0.6% faster, up to a maximum of 1000 per second.";
});

// The live editor buffer refines the stored current-script count.
const currentChars = computed(() => {
  const live = AutomatorTextUI.editor?.getDoc().getValue().length;
  return live ?? auto.value.current_script_chars;
});
const totalChars = computed(
  () =>
    auto.value.total_script_chars -
    auto.value.current_script_chars +
    currentChars.value,
);
const withinLimit = computed(
  () =>
    currentChars.value <= MAX_SCRIPT_CHARS && totalChars.value <= MAX_TOTAL_CHARS,
);

const deleteScriptName = computed(
  () =>
    auto.value.scripts.find((s) => s.id === deleteScriptId.value)?.name ?? "",
);

async function confirmDelete() {
  await game.automatorDeleteScript(deleteScriptId.value);
  deleteScriptId.value = null;
}
</script>

<template>
  <div class="c-automator-tab l-automator-tab">
    <div v-if="auto.unlocked">
      <div>
        {{ intervalText }}
      </div>
      <span :class="{ 'c-overlimit': currentChars > MAX_SCRIPT_CHARS }">
        This script: {{ currentChars }} / {{ MAX_SCRIPT_CHARS }}
      </span>
      |
      <span :class="{ 'c-overlimit': totalChars > MAX_TOTAL_CHARS }">
        Across all scripts: {{ totalChars }} / {{ MAX_TOTAL_CHARS }}
      </span>
      <br>
      <span
        v-if="!withinLimit"
        class="c-overlimit"
      >
        (Your changes will not be saved due to being over a character limit!)
      </span>
      <div class="c-automator-split-pane">
        <div class="l-automator-split">
          <AutomatorEditor />
          <AutomatorDocs @delete-script="deleteScriptId = $event" />
        </div>
      </div>
      <Modal
        v-if="deleteScriptId !== null"
        title="Delete Automator Script"
        compact
        centered
        @close="deleteScriptId = null"
      >
        <div>
          Are you sure you want to delete the script "{{ deleteScriptName }}"?
          This cannot be undone!
        </div>
        <div class="l-modal-buttons">
          <button
            class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
            @click="deleteScriptId = null"
          >
            Cancel
          </button>
          <button
            class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
            @click="confirmDelete"
          >
            Confirm
          </button>
        </div>
      </Modal>
    </div>
    <AutomatorPointsList v-else />
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorTab.vue scoped style; the SplitPane
   library is replaced by a fixed 50/50 flex split. */
.c-overlimit {
  font-weight: bold;
  color: var(--color-bad);
}

.c-automator-tab {
  width: 80%;
  min-width: 100rem;
}

.l-automator-tab {
  position: relative;
  align-self: center;
  margin-top: 0.5rem;
}

.c-automator-split-pane {
  width: 100%;
  height: 57rem;
  background-color: #bbbbbb;
}

.s-base--dark .c-automator-split-pane {
  width: 100%;
  background-color: #474747;
}

.l-automator-split {
  display: flex;
  flex-direction: row;
  width: 100%;
  height: 100%;
}

.l-automator-split > * {
  width: 50%;
  height: 100%;
}
</style>
