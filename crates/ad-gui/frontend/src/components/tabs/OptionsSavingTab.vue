<script setup>
// Saving options subtab. The grid layout mirrors the original
// `OptionsSavingTab.vue` (../antimatter-dimensions/src/components/tabs/
// options-saving), reproducing the top half only — everything related to Cloud
// saves (and the post-Reality Speedrun row) is omitted.
import { ref } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import OptionsSlider from "../options/OptionsSlider.vue";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";

const game = useGameStore();
const ui = useUiStore();

// Local-only placeholders until saving options are engine-owned.
const autosaveInterval = ref(30);
const showTimeSinceSave = ref(false);
const saveFileName = ref("");

// Mirror the original SaveFileName input filter: strip anything that is not
// alphanumeric, space or hyphen.
function handleNameChange(event) {
  const newName = event.target.value.trim().replace(/[^a-zA-Z0-9 -]/gu, "");
  saveFileName.value = newName;
  event.target.value = newName;
}

async function exportToClipboard() {
  const saveStr = await game.exportSave();
  await navigator.clipboard.writeText(saveStr);
  ui.notify("Save exported to clipboard");
}

async function exportToFile() {
  try {
    await game.exportSaveToFile(saveFileName.value);
    ui.notify("Save exported to file");
  } catch (e) {
    if (e !== "Cancelled") {
      ui.notify(`Export failed: ${e}`, "error");
    }
  }
}

async function importFromFile() {
  try {
    await game.importSaveFromFile();
    ui.notify("Save loaded from file");
  } catch (e) {
    if (e !== "Cancelled") {
      ui.notify(`Import failed: ${e}`, "error");
    }
  }
}
</script>

<template>
  <div class="l-options-tab">
    <div class="l-options-grid">
      <div class="l-options-grid__row">
        <button
          class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button"
          @click="exportToClipboard"
        >
          Export save
        </button>
        <button
          class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button"
          @click="ui.showModal('importSave')"
        >
          Import save
        </button>
        <button
          class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button"
          @click="ui.showModal('hardReset')"
        >
          RESET THE GAME
        </button>
      </div>
      <div class="l-options-grid__row">
        <button class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button">
          Save game
        </button>
        <button
          class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button"
          @click="ui.showModal('loadGame')"
        >
          Choose save
        </button>
        <div class="o-primary-btn o-primary-btn--option o-primary-btn--slider l-options-grid__button">
          <b>Autosave interval: {{ autosaveInterval }}s</b>
          <OptionsSlider
            class="o-primary-btn--slider__slider"
            :min="10"
            :max="60"
            :interval="1"
            :model-value="autosaveInterval"
            @update:model-value="autosaveInterval = $event"
          />
        </div>
      </div>
      <div class="l-options-grid__row">
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="exportToFile"
        >
          Export save as file
        </button>
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="importFromFile"
        >
          Import save from file
        </button>
        <PrimaryToggleButton
          class="o-primary-btn--option l-options-grid__button"
          label="Display time since save:"
          :model-value="showTimeSinceSave"
          @update:model-value="showTimeSinceSave = $event"
        />
      </div>
      <div class="l-options-grid__row">
        <button
          class="o-primary-btn o-primary-btn--option l-options-grid__button"
          @click="ui.showModal('backup')"
        >
          Open Automatic Save Backup Menu
        </button>
        <div class="o-primary-btn o-primary-btn--option o-primary-btn--input l-options-grid__button">
          <b>Save file name:</b>
          <span ach-tooltip="Set a custom name (up to 16 alphanumeric characters, including space and hyphen)">
            <input
              class="c-custom-save-name__input"
              type="text"
              maxlength="16"
              placeholder="Custom save name"
              :value="saveFileName"
              @change="handleNameChange"
            >
          </span>
        </div>
        <div class="l-options-grid__button l-options-grid__button--hidden" />
      </div>
      <OpenHotkeysButton />
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original SaveFileName.vue's <style scoped> block (this
   class is not part of the vendored global CSS). */
.c-custom-save-name__input {
  font-family: Typewriter;
  font-size: 1.3rem;
  font-weight: bold;
  text-align: center;
  border: 0.1rem solid black;
  border-radius: var(--var-border-radius, 0.3rem);
}
</style>
