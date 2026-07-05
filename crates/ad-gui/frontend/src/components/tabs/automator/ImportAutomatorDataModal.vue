<script setup>
// The script-import modal (vendored from ImportAutomatorDataModal.vue):
// paste an exported string, preview its contents (name, size, attached
// presets/constants with overwrite warnings), choose whether to import the
// extra data, and create the script at the end of the list.
import { computed, onMounted, ref, watch } from "vue";

import Modal from "../../Modal.vue";
import { useGameStore } from "../../../stores/game";

const emit = defineEmits(["close"]);

const game = useGameStore();

const input = ref("");
const inputEl = ref(null);
const preview = ref(null);
const ignorePresets = ref(false);
const ignoreConstants = ref(false);

const MAX_CONSTANT_COUNT = 30;

const isValid = computed(() => preview.value !== null);
const hasExtraData = computed(() => preview.value?.is_full_data ?? false);
const importedPresets = computed(() => preview.value?.presets ?? []);
const importedConstants = computed(() => preview.value?.constants ?? []);
const hasPresets = computed(() => importedPresets.value.length !== 0);
const hasConstants = computed(() => importedConstants.value.length !== 0);
const lineCount = computed(
  () => preview.value?.content.split("\n").length ?? 0,
);

const currentPresets = computed(() => game.snapshot.time_studies.presets);
const currentConstants = computed(() => game.snapshot.automator.constants);

// Number of non-empty presets whose contents differ from the import.
const overwrittenPresetCount = computed(() => {
  let mismatched = 0;
  for (const [slot, name, studies] of importedPresets.value) {
    const existing = currentPresets.value[slot - 1];
    const isEmpty = existing.name === "" && existing.studies === "";
    if (!isEmpty && (existing.name !== name || existing.studies !== studies)) {
      mismatched++;
    }
  }
  return mismatched;
});

const willOverwriteConstant = computed(() => {
  if (!hasExtraData.value) return false;
  const byName = new Map(
    currentConstants.value.map((c) => [c.name, c.value]),
  );
  return importedConstants.value.some(
    ([key, value]) => byName.has(key) && byName.get(key) !== value,
  );
});

const constantCountAfterImport = computed(() => {
  const all = new Set(currentConstants.value.map((c) => c.name));
  if (hasExtraData.value) {
    for (const [key] of importedConstants.value) all.add(key);
  }
  return all.size;
});

const extraConstants = computed(
  () => constantCountAfterImport.value - MAX_CONSTANT_COUNT,
);

const isImportingExtraData = computed(() => {
  const hasNewConstants =
    willOverwriteConstant.value ||
    constantCountAfterImport.value > currentConstants.value.length;
  const isImportingPresets = hasPresets.value && !ignorePresets.value;
  const isImportingConstants =
    hasConstants.value && !ignoreConstants.value && hasNewConstants;
  return (
    isValid.value &&
    hasExtraData.value &&
    (isImportingPresets || isImportingConstants)
  );
});

const presetButtonText = computed(() =>
  ignorePresets.value ? "Will Ignore Presets" : "Will Import Presets",
);
const constantButtonText = computed(() =>
  ignoreConstants.value ? "Will Ignore Constants" : "Will Import Constants",
);

watch(input, async (raw) => {
  preview.value = raw ? await game.automatorImportPreview(raw) : null;
});

onMounted(() => inputEl.value.select());

async function importSave() {
  if (!isValid.value) return;
  await game.automatorImport(
    input.value,
    ignorePresets.value,
    ignoreConstants.value,
  );
  emit("close");
}
</script>

<template>
  <Modal
    title="Import Automator Script Data"
    compact
    centered
    @close="emit('close')"
  >
    <div class="c-modal-message__text">
      This will create a new Automator script at the end of your list.
      <span v-if="isImportingExtraData">This will also import additional data related to the script.</span>
    </div>
    <input
      ref="inputEl"
      v-model="input"
      type="text"
      class="c-modal-input c-modal-import__input"
      @keyup.enter="importSave"
      @keyup.esc="emit('close')"
    >
    <div
      v-if="isValid"
      class="c-modal-message__text"
    >
      Script name: {{ preview.name }}
      <br>
      Line count: {{ lineCount }}
      <div v-if="hasPresets">
        <br>
        Study Presets:
        <span
          v-for="[slot, name] in importedPresets"
          :key="slot"
          class="c-import-data-name"
        >
          <span v-if="name">"{{ name }}" (slot {{ slot }})</span>
          <span v-else>Preset slot #{{ slot }}</span>
        </span>
        <div
          v-if="!ignorePresets && overwrittenPresetCount > 0"
          class="l-has-errors"
        >
          {{ overwrittenPresetCount }} of your existing presets
          will be overwritten by imported presets!
        </div>
        <br>
        <button
          class="o-primary-btn"
          @click="ignorePresets = !ignorePresets"
        >
          {{ presetButtonText }}
        </button>
      </div>
      <div v-if="hasConstants">
        <br>
        Constants:
        <span
          v-for="[key] in importedConstants"
          :key="key"
          class="c-import-data-name"
        >
          "{{ key }}"
        </span>
        <div
          v-if="!ignoreConstants && (willOverwriteConstant || extraConstants > 0)"
          class="l-has-errors"
        >
          <span v-if="willOverwriteConstant">Some of your existing constants will be overwritten!</span>
          <br v-if="willOverwriteConstant && extraConstants > 0">
          <span v-if="extraConstants > 0">
            {{ extraConstants }} {{ extraConstants === 1 ? "constant" : "constants" }} will not be imported due to the
            {{ MAX_CONSTANT_COUNT }} constant limit.
          </span>
        </div>
        <br>
        <button
          class="o-primary-btn"
          @click="ignoreConstants = !ignoreConstants"
        >
          {{ constantButtonText }}
        </button>
      </div>
      <br>
      <div
        v-if="preview.has_errors"
        class="l-has-errors"
      >
        This script has errors which need to be fixed before it can be run!
      </div>
      <div v-if="preview.has_errors && isImportingExtraData">
        <i>Some errors may be fixed with the additional data being imported.</i>
      </div>
    </div>
    <div
      v-else-if="input.length !== 0"
      class="c-modal-message__text"
    >
      Invalid Automator data string
    </div>
    <div class="l-modal-buttons">
      <button
        v-if="!isValid"
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn"
        @click="emit('close')"
      >
        Cancel
      </button>
      <button
        v-else
        class="o-primary-btn o-primary-btn--width-medium c-modal-message__okay-btn c-modal__confirm-btn"
        @click="importSave"
      >
        Import
      </button>
    </div>
  </Modal>
</template>

<style scoped>
/* Vendored from the original ImportAutomatorDataModal.vue scoped style. */
.l-has-errors {
  color: red;
}

.c-import-data-name {
  padding: 0 1rem;
}
</style>
