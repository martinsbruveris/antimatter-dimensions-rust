<script setup>
// One script's row on the data-transfer pane (vendored from
// AutomatorDataTransferSingleEntry.vue): the full-data export button plus
// collapsible lists of the presets/constants the script references.
import { onMounted, ref, watch } from "vue";

import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";

const props = defineProps({
  // { id, name }
  script: { type: Object, required: true },
});

const game = useGameStore();
const ui = useUiStore();

// [slot (1-based), name, studies] / [name, value] tuples from the engine.
const presets = ref([]);
const constants = ref([]);
const hidePresets = ref(true);
const hideConstants = ref(true);

async function load() {
  const info = await game.automatorScriptDataInfo(props.script.id);
  presets.value = info.presets;
  constants.value = info.constants;
}
onMounted(load);
watch(() => props.script.id, load);

function iconClass(state) {
  return state ? "far fa-plus-square" : "far fa-minus-square";
}

async function exportData() {
  const toExport = await game.automatorExportFull(props.script.id);
  if (toExport) {
    await navigator.clipboard.writeText(toExport);
    ui.notify(
      `Exported all data associated with "${props.script.name}" to your clipboard`,
      "automator",
      6000,
    );
  } else {
    ui.notify("Could not export data from blank Automator script!", "error");
  }
}
</script>

<template>
  <div class="l-entry-padding">
    <button
      title="Export Full Script Data"
      class="l-button-margin fas fa-file-export"
      @click="exportData"
    />
    <b>Script name: {{ script.name }}</b>
    <br>
    <span v-if="presets.length !== 0">
      <span
        :class="iconClass(hidePresets)"
        @click="hidePresets = !hidePresets"
      />
      References {{ presets.length }} recognized study
      {{ presets.length === 1 ? "preset" : "presets" }}
      <span v-if="!hidePresets">
        <div
          v-for="[slot, name, studies] in presets"
          :key="slot"
        >
          <span v-if="name">"{{ name }}" (slot {{ slot }}):</span>
          <span v-else>Preset slot {{ slot }}:</span>
          <br>
          <div class="l-value-padding">
            <span v-if="studies">{{ studies }}</span>
            <i v-else>Empty Study Preset</i>
          </div>
        </div>
      </span>
    </span>
    <span v-else>
      Does not reference any study presets.
    </span>
    <br>
    <span v-if="constants.length !== 0">
      <span
        :class="iconClass(hideConstants)"
        @click="hideConstants = !hideConstants"
      />
      References {{ constants.length }} defined
      {{ constants.length === 1 ? "constant" : "constants" }}
      <span v-if="!hideConstants">
        <div
          v-for="[name, value] in constants"
          :key="name"
        >
          "{{ name }}":
          <br>
          <div class="l-value-padding">
            {{ value }}
          </div>
        </div>
      </span>
    </span>
    <span v-else>
      Does not reference any defined constants.
    </span>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDataTransferSingleEntry.vue style. */
.l-entry-padding {
  border: solid 0.1rem var(--color-automator-docs-font);
  border-radius: var(--var-border-radius, 0.5rem);
  overflow-wrap: break-word;
  padding: 1rem 1.5rem;
}

.l-value-padding {
  padding-left: 1.5rem;
}

.l-button-margin {
  margin-right: 1rem;
}
</style>
