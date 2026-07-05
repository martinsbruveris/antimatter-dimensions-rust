<script setup>
// The right-hand docs pane (vendored from AutomatorDocs.vue): pane buttons on
// top, the script dropdown / rename / delete row, and the selected page.
// Stage E adds the data-transfer, templates, and blocks panes plus the
// import/export buttons; their pane ids are reserved.
import { computed, nextTick, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import { automatorErrors } from "../../../util/automatorEditor";
import AutomatorButton from "./AutomatorButton.vue";
import AutomatorDefinePage from "./AutomatorDefinePage.vue";
import AutomatorDocsCommandList from "./AutomatorDocsCommandList.vue";
import AutomatorDocsIntroPage from "./AutomatorDocsIntroPage.vue";
import AutomatorErrorPage from "./AutomatorErrorPage.vue";
import AutomatorEventLog from "./AutomatorEventLog.vue";
import AutomatorScriptDropdown from "./AutomatorScriptDropdown.vue";

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

const emit = defineEmits(["delete-script"]);

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);
const pane = computed(() => auto.value.current_info_pane);
const errorCount = computed(() => automatorErrors.value.length);

const editingName = ref(false);
const renameInput = ref(null);
const isNameTooLong = ref(false);

function setPane(id) {
  game.automatorSetInfoPane(id);
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
      </div>
      <div class="l-automator-button-row">
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
      <AutomatorDefinePage v-else-if="pane === PANELS.CONSTANTS" />
      <!-- Data transfer / templates / blocks panes are Stage E. -->
      <AutomatorDocsIntroPage v-else />
    </div>
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
