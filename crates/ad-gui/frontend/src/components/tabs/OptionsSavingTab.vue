<script setup>
// Saving options subtab. The grid layout mirrors the original
// `OptionsSavingTab.vue` (../antimatter-dimensions/src/components/tabs/
// options-saving), reproducing the top half only — everything related to Cloud
// saves (and the post-Reality Speedrun row) is omitted.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import OptionsSlider from "../options/OptionsSlider.vue";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";

const game = useGameStore();
const ui = useUiStore();

// Autosave interval / time-since-save are engine-owned options (round-tripped
// through the save); read them from the snapshot, write via the store actions.
// The slider is in seconds; the option is stored in milliseconds.
const autosaveInterval = computed(() =>
  Math.round((game.snapshot?.options?.autosave_interval ?? 30000) / 1000)
);
const showTimeSinceSave = computed(
  () => game.snapshot?.options?.show_time_since_save ?? true
);
// Custom save-file name is engine-owned (stored per save slot), so read it from
// the snapshot rather than a local ref.
const saveFileName = computed(() => game.snapshot?.options?.save_file_name ?? "");

async function saveGame() {
  await game.saveGame();
  ui.notify("Game saved");
}

// Mirror the original SaveFileName input handler: strip anything that is not
// alphanumeric, space or hyphen for immediate feedback, then persist to the
// engine (which sanitizes again and stores it on the current save slot).
function handleNameChange(event) {
  const newName = event.target.value.trim().replace(/[^a-zA-Z0-9 -]/gu, "");
  event.target.value = newName;
  game.setSaveFileName(newName);
}

async function exportToClipboard() {
  const saveStr = await game.exportSave();
  await navigator.clipboard.writeText(saveStr);
  ui.notify("Save exported to clipboard");
}

async function exportToFile() {
  try {
    await game.exportSaveToFile();
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
    ui.notify("Game loaded");
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
        <button
          class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button"
          @click="saveGame"
        >
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
            @update:model-value="game.setAutosaveInterval($event * 1000)"
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
          @update:model-value="game.setShowTimeSinceSave($event)"
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
