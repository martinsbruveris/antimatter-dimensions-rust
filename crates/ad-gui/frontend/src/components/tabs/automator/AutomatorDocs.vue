<script setup>
// The right-hand docs pane (vendored from AutomatorDocs.vue): pane buttons on
// top, the export/import + script dropdown / rename / delete row, and the
// selected page. Full-screen mode is deferred with the fixed split (Stage D
// deviation).
import { computed, nextTick, ref, watch } from "vue";

import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";
import { automatorErrors } from "../../../util/automatorEditor";
import AutomatorButton from "./AutomatorButton.vue";
import AutomatorBlocksPage from "./AutomatorBlocksPage.vue";
import AutomatorDataTransferPage from "./AutomatorDataTransferPage.vue";
import AutomatorDefinePage from "./AutomatorDefinePage.vue";
import AutomatorDocsCommandList from "./AutomatorDocsCommandList.vue";
import AutomatorDocsIntroPage from "./AutomatorDocsIntroPage.vue";
import AutomatorDocsTemplateList from "./AutomatorDocsTemplateList.vue";
import AutomatorErrorPage from "./AutomatorErrorPage.vue";
import AutomatorEventLog from "./AutomatorEventLog.vue";
import AutomatorScriptDropdown from "./AutomatorScriptDropdown.vue";
import AutomatorScriptTemplateModal from "./AutomatorScriptTemplateModal.vue";
import ImportAutomatorDataModal from "./ImportAutomatorDataModal.vue";

// `AutomatorPanels` (saved as `currentInfoPane`).
const PANELS = {
  INTRO_PAGE: 0,
  COMMANDS: 1,
  ERRORS: 2,
  EVENTS: 3,
  DATA_TRANSFER: 4,
  CONSTANTS: 5,
  TEMPLATES: 6,
  BLOCKS: 7,
};

const MAX_SCRIPT_COUNT = 20;

const emit = defineEmits(["delete-script"]);

const game = useGameStore();
const ui = useUiStore();
const auto = computed(() => game.snapshot.automator);
const pane = computed(() => auto.value.current_info_pane);
const errorCount = computed(() => automatorErrors.value.length);
const isBlock = computed(() => auto.value.editor_type === "block");

const editingName = ref(false);
const renameInput = ref(null);
const isNameTooLong = ref(false);
const shownTemplate = ref(null);
const importModalOpen = ref(false);

const canMakeNewScript = computed(
  () => auto.value.scripts.length < MAX_SCRIPT_COUNT,
);
const importTooltip = computed(() =>
  canMakeNewScript.value
    ? "Import single automator script or data"
    : "You have too many scripts to import another!",
);

function setPane(id) {
  game.automatorSetInfoPane(id);
}

// `openMatchingAutomatorTypeDocs` + `fixAutomatorTypeDocs`: the mode-specific
// reference pane follows the editor type.
watch(isBlock, (block) => {
  setPane(block ? PANELS.BLOCKS : PANELS.COMMANDS);
});
if (
  (pane.value === PANELS.COMMANDS && isBlock.value) ||
  (pane.value === PANELS.BLOCKS && !isBlock.value)
) {
  setPane(isBlock.value ? PANELS.BLOCKS : PANELS.COMMANDS);
}

async function exportScript() {
  const toExport = await game.automatorExportScript(auto.value.editor_script);
  if (toExport) {
    await navigator.clipboard.writeText(toExport);
    ui.notify("Exported current Automator script to your clipboard", "automator", 3000);
  } else {
    ui.notify("Could not export blank Automator script!", "error");
  }
}

function importScript() {
  if (!canMakeNewScript.value) return;
  importModalOpen.value = true;
}

function activePanelClass(id) {
  return { "c-automator__button--active": pane.value === id };
}

const currentScriptName = computed(
  () =>
    auto.value.scripts.find((s) => s.id === auto.value.editor_script)?.name ??
    "",
);

function rename() {
  editingName.value = true;
  nextTick(() => {
    renameInput.value.value = currentScriptName.value;
    renameInput.value.focus();
  });
}

async function nameEdited() {
  const trimmed = renameInput.value.value.trim();
  if (trimmed.length > 15) {
    isNameTooLong.value = true;
    return;
  }
  isNameTooLong.value = false;
  if (trimmed.length > 0) {
    await game.automatorRenameScript(auto.value.editor_script, trimmed);
  }
  editingName.value = false;
}
</script>

<template>
  <div class="l-automator-pane">
    <div class="c-automator__controls l-automator__controls">
      <div class="l-automator-button-row">
        <AutomatorButton
          title="Automator Introduction"
          class="fa-circle-info"
          :class="activePanelClass(PANELS.INTRO_PAGE)"
          @click="setPane(PANELS.INTRO_PAGE)"
        />
        <AutomatorButton
          title="Scripting Information"
          class="fa-list"
          :class="activePanelClass(PANELS.COMMANDS)"
          @click="setPane(PANELS.COMMANDS)"
        />
        <AutomatorButton
          :title="`Your script has ${errorCount} error(s)`"
          :style="{ 'background-color': errorCount === 0 ? '' : 'red' }"
          class="fa-exclamation-triangle"
          :class="activePanelClass(PANELS.ERRORS)"
          @click="setPane(PANELS.ERRORS)"
        />
        <AutomatorButton
          title="Extended Data Transfer"
          class="fa-window-restore"
          :class="activePanelClass(PANELS.DATA_TRANSFER)"
          @click="setPane(PANELS.DATA_TRANSFER)"
        />
        <AutomatorButton
          title="View recently executed commands"
          class="fa-eye"
          :class="activePanelClass(PANELS.EVENTS)"
          @click="setPane(PANELS.EVENTS)"
        />
        <AutomatorButton
          title="Modify defined constants"
          class="fa-book"
          :class="activePanelClass(PANELS.CONSTANTS)"
          @click="setPane(PANELS.CONSTANTS)"
        />
        <AutomatorButton
          title="Template Creator List"
          class="fa-file-code"
          :class="activePanelClass(PANELS.TEMPLATES)"
          @click="setPane(PANELS.TEMPLATES)"
        />
        <AutomatorButton
          v-if="isBlock"
          title="Command menu for Block editor mode"
          class="fa-cubes"
          :class="activePanelClass(PANELS.BLOCKS)"
          @click="setPane(PANELS.BLOCKS)"
        />
      </div>
      <div class="l-automator-button-row">
        <AutomatorButton
          title="Export single automator script"
          class="fa-file-export"
          @click="exportScript"
        />
        <AutomatorButton
          :title="importTooltip"
          class="fa-file-import"
          :class="{ 'c-automator__status-text--error': !canMakeNewScript }"
          @click="importScript"
        />
        <div class="l-automator__script-names">
          <template v-if="!editingName">
            <AutomatorScriptDropdown @rename="rename" />
            <AutomatorButton
              title="Rename script"
              class="far fa-edit"
              @click="rename"
            />
          </template>
          <input
            v-else
            ref="renameInput"
            :title="isNameTooLong ? 'Names cannot be longer than 15 characters!' : undefined"
            class="l-automator__rename-input c-automator__rename-input"
            :class="{ 'c-long-name-box': isNameTooLong }"
            @blur="nameEdited"
            @keyup.enter="renameInput.blur()"
          >
        </div>
        <AutomatorButton
          title="Delete this script"
          class="fas fa-trash"
          @click="emit('delete-script', auto.editor_script)"
        />
      </div>
    </div>
    <div class="c-automator-docs l-automator-pane__content">
      <AutomatorDocsIntroPage v-if="pane === PANELS.INTRO_PAGE" />
      <AutomatorDocsCommandList v-else-if="pane === PANELS.COMMANDS" />
      <AutomatorErrorPage v-else-if="pane === PANELS.ERRORS" />
      <AutomatorEventLog v-else-if="pane === PANELS.EVENTS" />
      <AutomatorDataTransferPage v-else-if="pane === PANELS.DATA_TRANSFER" />
      <AutomatorDefinePage v-else-if="pane === PANELS.CONSTANTS" />
      <AutomatorDocsTemplateList
        v-else-if="pane === PANELS.TEMPLATES"
        @show-template="shownTemplate = $event"
      />
      <AutomatorBlocksPage v-else-if="pane === PANELS.BLOCKS" />
      <AutomatorDocsIntroPage v-else />
    </div>
    <AutomatorScriptTemplateModal
      v-if="shownTemplate"
      :template="shownTemplate"
      @close="shownTemplate = null"
    />
    <ImportAutomatorDataModal
      v-if="importModalOpen"
      @close="importModalOpen = false"
    />
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDocs.vue scoped style. */
.l-automator__script-names {
  flex-grow: 1;
  display: flex;
  flex-direction: row;
  justify-content: space-evenly;
  align-items: center;
}

.l-automator__rename-input {
  width: 100%;
  height: calc(2rem + 1rem / 3 - var(--var-border-width, 0rem) * 2);
  border: var(--var-border-width, 0.2rem) solid var(--color-reality-light);
  border-radius: var(--var-border-radius, 0.3rem);
  margin: 0.4rem;
  padding: 0.4rem;
}

.c-automator__rename-input {
  font-family: Typewriter;
  font-size: 1.2rem;
  color: var(--color-automator-docs-font);
  background-color: var(--color-automator-controls-active);
}

.c-automator__button--active {
  background-color: var(--color-automator-controls-active);
  border-color: var(--color-reality-light);
}

.c-long-name-box {
  background-color: var(--color-automator-error-background);
  border-color: var(--color-automator-error-outline);
}
</style>
