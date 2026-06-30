<script setup>
// Saving options subtab. The grid layout mirrors the original
// `OptionsSavingTab.vue` (../antimatter-dimensions/src/components/tabs/
// options-saving), reproducing the top half only — everything related to Cloud
// saves (and the post-Reality Speedrun row) is omitted. Nothing is wired up yet:
// the buttons are visual only, except the ones that open a modal (Import save,
// RESET THE GAME, Choose save, backup menu), which drive `ui.openModal` so the
// modals can be seen. The autosave-interval slider and save-file-name input keep
// local-only state. Real save/load actions land later.
import { ref } from "vue";

import { useUiStore } from "../../stores/ui";
import OptionsSlider from "../options/OptionsSlider.vue";
import PrimaryToggleButton from "../options/PrimaryToggleButton.vue";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";

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
</script>

<template>
  <div class="l-options-tab">
    <div class="l-options-grid">
      <div class="l-options-grid__row">
        <button class="o-primary-btn o-primary-btn--option o-primary-btn--option_font-x-large l-options-grid__button">
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
        <button class="o-primary-btn o-primary-btn--option l-options-grid__button">
          Export save as file
        </button>
        <button class="o-primary-btn o-primary-btn--option l-options-grid__button c-file-import-button">
          <input
            class="c-file-import"
            type="file"
            accept=".txt"
          >
          <label for="file">Import save from file</label>
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
/* The vendored `.c-file-import` hack balloons an invisible `::before`
   (font-size: 100rem; padding: 10rem 20rem) so the whole button opens the file
   dialog. WebKit (the Tauri macOS webview) paints that overflow outside the
   button — covering the "Choose save" button in the row above and stealing its
   clicks — whereas Chrome clips it. Clip it to the button: the file input still
   fills its own button (width: 100%; height: 5.5rem), so import stays clickable. */
.c-file-import-button {
  overflow: hidden;
}

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
