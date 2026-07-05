<script setup>
// The "Current Script" dropdown (simplified ExpandingControlBox +
// AutomatorScriptDropdownEntryList): pick a script to edit, or create a new
// one (which opens the rename input via the parent).
import { computed, ref } from "vue";

import { useGameStore } from "../../../stores/game";

const MAX_SCRIPTS = 20;

const emit = defineEmits(["rename"]);
const game = useGameStore();
const open = ref(false);

const auto = computed(() => game.snapshot.automator);
const scripts = computed(() => auto.value.scripts);
const currentName = computed(
  () =>
    scripts.value.find((s) => s.id === auto.value.editor_script)?.name ?? "",
);
const highlightRunning = computed(() => auto.value.is_on);
const canMakeNewScript = computed(() => scripts.value.length < MAX_SCRIPTS);

function dropdownLabel(script) {
  const labels = [];
  if (script.id === auto.value.editor_script) labels.push("viewing");
  if (script.id === auto.value.top_level_script) {
    if (auto.value.is_running) labels.push("running");
    else if (auto.value.is_on) labels.push("paused");
  }
  if (labels.length === 0) return script.name;
  const status = labels.join(", ");
  return `${script.name} (${status.charAt(0).toUpperCase()}${status.slice(1)})`;
}

function labelClassObject(id) {
  return {
    "c-automator-docs-script-select": true,
    "l-active-script":
      id === auto.value.top_level_script && highlightRunning.value,
    "l-selected-script":
      id === auto.value.editor_script &&
      (id !== auto.value.top_level_script || !highlightRunning.value),
  };
}

async function changeScript(id) {
  await game.automatorSelectScript(id);
  open.value = false;
}

async function createNewScript() {
  const id = await game.automatorNewScript();
  open.value = false;
  if (id !== null) emit("rename");
}
</script>

<template>
  <div class="l-expanding-control-box l-automator__scripts-dropdown">
    <div
      class="l-expanding-control-box__container"
      @mouseleave="open = false"
    >
      <div
        class="c-automator-docs-script-select"
        @click="open = !open"
      >
        ▼ Current Script: {{ currentName }}
      </div>
      <div v-show="open">
        <div
          v-for="script in scripts"
          :key="script.id"
          class="l-script-option c-script-option-hover-effect"
          :class="labelClassObject(script.id)"
          @click="changeScript(script.id)"
        >
          {{ dropdownLabel(script) }}
        </div>
        <div
          v-if="canMakeNewScript"
          class="l-create-script c-automator-docs-script-select c-script-option-hover-effect"
          @click="createNewScript"
        >
          <i>Create a new script (You have {{ scripts.length }} / {{ MAX_SCRIPTS }})</i>
        </div>
        <div
          v-else
          class="l-create-script c-automator-docs-script-select l-max-scripts"
        >
          <i>You can only have {{ MAX_SCRIPTS }} scripts!</i>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Expanding-box layout (replicated, per the SelectNotationDropdown
   precedent) + the original AutomatorScriptDropdownEntryList styles. */
.l-expanding-control-box {
  position: relative;
  z-index: 4;
  height: 2.4rem;
  width: 100%;
  margin: 0.4rem;
  user-select: none;
}

.l-expanding-control-box__container {
  display: block;
  width: 100%;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.l-script-option {
  border-radius: 0;
  border-bottom: 0;
}

.c-script-option-hover-effect:hover {
  filter: brightness(70%);
  background-color: var(--color-automator-active-line-background);
}

.l-create-script {
  border-radius: 0 0 var(--var-border-radius, 0.5rem) var(--var-border-radius, 0.5rem);
}

.l-active-script {
  background-color: var(--color-automator-controls-active);
}

.l-selected-script {
  background-color: var(--color-automator-active-line-outline);
}

.l-max-scripts {
  background-color: var(--color-automator-error-background);
  cursor: auto;
}
</style>
