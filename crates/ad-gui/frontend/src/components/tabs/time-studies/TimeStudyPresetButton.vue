<script setup>
// One Time Study preset slot button, a port of the original
// TimeStudySaveLoadButton.vue: click loads, shift-click saves, hovering opens
// the context menu (rename input / Edit / Export / Save / Load with "Respec
// and Load" hover option / Delete). Edit and Delete are delegated to the tab
// (a modal). The scoped styles are vendored from the original component.
import { computed, ref, watch } from "vue";

import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";
import HoverMenu from "./HoverMenu.vue";

const props = defineProps({
  // 1-based slot number, matching the original's `saveslot` prop.
  saveslot: { type: Number, required: true },
});
const emit = defineEmits(["edit", "delete"]);

const game = useGameStore();
const ui = useUiStore();
const menu = ref(null);
const closeMenu = () => menu.value?.close();

const preset = computed(
  () => game.snapshot.time_studies.presets[props.saveslot - 1],
);
const displayName = computed(() =>
  preset.value.name === "" ? `${props.saveslot}` : preset.value.name,
);
const canEternity = computed(() => game.snapshot.time_studies.can_eternity);

// The rename input holds local state while focused; the snapshot is replaced
// every tick, so binding it directly would clobber in-progress typing.
const nameEdit = ref("");
const nameFocused = ref(false);
watch(
  () => preset.value.name,
  (name) => {
    if (!nameFocused.value) nameEdit.value = name;
  },
  { immediate: true },
);

function presetName() {
  return preset.value.name ? `Study preset "${preset.value.name}"` : "Study preset";
}

async function save() {
  closeMenu();
  await game.studyPresetSave(props.saveslot - 1);
  ui.notify(`${presetName()} saved in slot ${props.saveslot}`, "eternity");
}

async function load() {
  closeMenu();
  if (preset.value.studies) {
    await game.studyPresetLoad(props.saveslot - 1);
    ui.notify(`${presetName()} loaded from slot ${props.saveslot}`, "eternity");
  } else {
    ui.notify("This Time Study list currently contains no Time Studies.", "error");
  }
}

async function respecAndLoad() {
  if (!canEternity.value) return;
  closeMenu();
  await game.studyPresetLoad(props.saveslot - 1, true);
  ui.notify(`${presetName()} loaded from slot ${props.saveslot}`, "eternity");
}

async function rename(event) {
  nameFocused.value = false;
  const newName = event.target.value.slice(0, 4).trim();
  await game.studyPresetRename(props.saveslot - 1, newName);
  nameEdit.value = preset.value.name;
}

async function handleExport() {
  closeMenu();
  await navigator.clipboard.writeText(preset.value.studies);
  ui.notify(
    `${presetName()} exported from slot ${props.saveslot} to your clipboard`,
    "eternity",
  );
}

function edit() {
  closeMenu();
  emit("edit", props.saveslot);
}

function deletePreset() {
  closeMenu();
  if (preset.value.studies) emit("delete", props.saveslot);
  else ui.notify("This Time Study list currently contains no Time Studies.", "error");
}
</script>

<template>
  <HoverMenu
    ref="menu"
    class="l-tt-save-load-btn__wrapper"
  >
    <template #object>
      <button
        class="l-tt-save-load-btn c-tt-buy-button c-tt-buy-button--unlocked"
        @click.shift.exact="save"
        @click.exact="load"
      >
        {{ displayName }}
      </button>
    </template>
    <template #menu>
      <div class="l-tt-save-load-btn__menu c-tt-save-load-btn__menu">
        <span title="Set a custom name (up to 4 ASCII characters)">
          <input
            v-model="nameEdit"
            type="text"
            size="4"
            maxlength="4"
            class="l-tt-save-load-btn__menu-rename c-tt-save-load-btn__menu-rename"
            @focus="nameFocused = true"
            @blur="rename"
          >
        </span>
        <div
          class="l-tt-save-load-btn__menu-item c-tt-save-load-btn__menu-item"
          @click="edit"
        >
          Edit
        </div>
        <div
          class="l-tt-save-load-btn__menu-item c-tt-save-load-btn__menu-item"
          @click="handleExport"
        >
          Export
        </div>
        <div
          class="l-tt-save-load-btn__menu-item c-tt-save-load-btn__menu-item"
          @click="save"
        >
          Save
        </div>
        <div class="l-tt-save-load-btn__menu-item">
          <div
            class="c-tt-save-load-btn__menu-item"
            @click="load"
          >
            Load
          </div>
          <div class="c-tt-save-load-btn__menu-item__hover-options">
            <div
              :class="{
                'c-tt-save-load-btn__menu-item__hover-option': true,
                'c-tt-save-load-btn__menu-item__hover-option--disabled': !canEternity,
              }"
              @click="respecAndLoad"
            >
              Respec and Load
            </div>
          </div>
        </div>
        <div
          class="l-tt-save-load-btn__menu-item c-tt-save-load-btn__menu-item"
          @click="deletePreset"
        >
          Delete
        </div>
      </div>
    </template>
  </HoverMenu>
</template>

<style scoped>
/* Vendored from the original TimeStudySaveLoadButton.vue scoped style. */
.l-tt-save-load-btn__wrapper {
  position: relative;
  margin: 0.3rem;
}

.l-tt-save-load-btn {
  min-width: 2rem;
}

.l-tt-save-load-btn__menu {
  position: absolute;
  top: -0.5rem;
  left: 50%;
  padding: 0.5rem 0;
  transform: translate(-50%, -100%);
  z-index: 5;
}

.c-tt-save-load-btn__menu {
  text-align: left;
  font-family: Typewriter;
  font-size: 1.4rem;
  font-weight: bold;
  color: white;
  background: black;
  border-radius: var(--var-border-radius, 0.5rem);
}

.l-tt-save-load-btn__menu::after {
  content: "";
  position: absolute;
  top: 100%;
  left: 50%;
  border-color: black transparent transparent;
  border-style: solid;
  border-width: var(--var-border-width, 0.5rem);
  margin-left: -0.5rem;
}

.l-tt-save-load-btn__menu-rename {
  margin: 0.3rem 0.5rem 0.5rem 0.7rem;
}

.c-tt-save-load-btn__menu-rename {
  text-align: left;
  font-family: Typewriter;
  font-size: 1.4rem;
  font-weight: bold;
  border: none;
  border-radius: var(--var-border-radius, 0.3rem);
  padding: 0.2rem;
}

.l-tt-save-load-btn__menu-item {
  position: relative;
  cursor: pointer;
}

.c-tt-save-load-btn__menu-item {
  text-align: left;
  padding: 0.25rem 1rem;
}

.c-tt-save-load-btn__menu-item:hover {
  color: black;
  background: white;
}

.c-tt-save-load-btn__menu-item__hover-options {
  visibility: hidden;
  width: fit-content;
  position: absolute;
  top: 0;
  left: 100%;
  opacity: 0;
  color: white;
  background: black;
  border: 0.1rem solid black;
  border-radius: var(--var-border-width, 0.5rem);
  transform: translateX(0.5rem);
  transition: visibility 0.2s, opacity 0.2s;
  transition-delay: 0.5s;
  cursor: pointer;
}

.c-tt-save-load-btn__menu-item__hover-option {
  white-space: nowrap;
  padding: 0.25rem 1rem;
}

.c-tt-save-load-btn__menu-item__hover-options::after {
  content: "";
  position: absolute;
  /* A single menu item is 26px tall, minus 5px from the border */
  top: 0.8rem;
  right: 100%;
  border-top: 0.5rem solid transparent;
  border-right: 0.5rem solid black;
  border-bottom: 0.5rem solid transparent;
}

.c-tt-save-load-btn__menu-item:hover,
.c-tt-save-load-btn__menu-item__hover-option:hover {
  color: black;
  background: white;
}

.l-tt-save-load-btn__menu-item:hover .c-tt-save-load-btn__menu-item__hover-options {
  visibility: visible;
  opacity: 1;
  transition-delay: 0s;
}

.c-tt-save-load-btn__menu-item__hover-option--disabled {
  opacity: 0.7;
  cursor: default;
}

.c-tt-save-load-btn__menu-item__hover-option--disabled:hover {
  color: white;
  background: transparent;
}
</style>
