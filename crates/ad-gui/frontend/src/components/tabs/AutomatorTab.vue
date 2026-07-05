<script setup>
// The Automator tab (vendored from AutomatorTab.vue): the interval line and
// character counters over a two-pane split — editor left, docs right. Locked,
// it shows the AP progress page. The original's draggable SplitPane is a
// fixed 50/50 split here (a noted Stage D deviation); full-screen mode is
// deferred with it.
import { computed, ref } from "vue";

import ConfirmModal from "../ConfirmModal.vue";
import Modal from "../Modal.vue";
import { useGameStore } from "../../stores/game";
import { AutomatorTextUI } from "../../util/automatorEditor";
import {
  blockSwitchMessage,
  pendingModeSwitch,
  performModeSwitch,
} from "../../util/blockAutomator";
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

// The live editor buffer refines the stored current-script count (text mode
// only; in block mode every change saves immediately, so the stored count is
// current and the CodeMirror document may be stale).
const currentChars = computed(() => {
  const live =
    auto.value.editor_type === "text"
      ? AutomatorTextUI.editor?.getDoc().getValue().length
      : undefined;
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

// The mode the switch confirmation would change to.
const otherMode = computed(() =>
  auto.value.editor_type === "text" ? "Block" : "Text",
);

async function confirmModeSwitch() {
  pendingModeSwitch.value = null;
  await performModeSwitch(game);
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
      <!-- Vendored from SwitchAutomatorEditorModal.vue. -->
      <ConfirmModal
        v-if="pendingModeSwitch"
        :title="`Change Automator to ${otherMode} editor`"
        option="switchAutomatorMode"
        @close="pendingModeSwitch = null"
        @confirm="confirmModeSwitch"
      >
        This will stop your current script if it is running!
        <div v-if="pendingModeSwitch.errorCount">
          <br>
          Your script has some errors which may not get converted properly to {{ otherMode }} mode. Continuing on will
          make the Automator attempt to parse these lines anyway, although some information may get lost or not be
          converted properly.
        </div>
        <!-- Note: this can only ever appear on text-to-block -->
        <b v-if="pendingModeSwitch.lostBlocks">
          <br>
          Warning: Your script also currently has some lines which cannot interpreted as particular commands. These
          lines will end up being deleted since there is no block they can be converted into.
          If an error occurs at the start of a loop or IF, this may end up deleting large portions of your script!
          <span class="l-lost-text">
            Changing editor modes right now will cause
            {{ pendingModeSwitch.lostBlocks }} {{ pendingModeSwitch.lostBlocks === 1 ? "line" : "lines" }} of code to
            be irreversibly lost!
          </span>
        </b>
        <br>
        <span class="l-lost-text">
          Hiding this confirmation is not recommended, as it may cause parts of scripts to be immediately and
          irreversibly lost if your script has errors when attempting to switch modes.
        </span>
        <br>
        <br>
        Are you sure you want to change to the {{ otherMode }} editor?
      </ConfirmModal>
      <Modal
        v-if="blockSwitchMessage"
        title="Automator"
        compact
        centered
        @close="blockSwitchMessage = null"
      >
        <div class="c-modal-message__text">
          {{ blockSwitchMessage }}
        </div>
        <div class="l-modal-buttons">
          <button
            class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
            @click="blockSwitchMessage = null"
          >
            Okay
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

.l-lost-text {
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
