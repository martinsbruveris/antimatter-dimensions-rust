<script setup>
// The template-generation prompt (vendored from the original
// AutomatorScriptTemplate.vue modal). Inputs are validated as typed; once
// everything is valid the engine generates the script + warnings. In text
// mode the result is copied to the clipboard; in block mode it becomes a
// custom template block for the templates pane. A deviation from the
// original: the game-state warnings only appear once the inputs are valid
// (the engine returns them together with the generated script).
import { computed, reactive, ref, watch } from "vue";

import Modal from "../../Modal.vue";
import { useGameStore } from "../../../stores/game";
import { useUiStore } from "../../../stores/ui";
import { templateParamType } from "../../../data/automatorTemplates";
import { blockTemplates } from "../../../util/blockAutomator";

const props = defineProps({
  // One entry of AUTOMATOR_TEMPLATES: { name, description, inputs }.
  template: { type: Object, required: true },
});
const emit = defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();

const isBlock = computed(
  () => game.snapshot.automator.editor_type === "block",
);
const presets = computed(() => game.snapshot.time_studies.presets);

// Input values keyed by the original input names; bools start false, text
// starts empty (and counts as invalid until filled).
const templateInputs = reactive({});
const buttonTextStrings = reactive({});
// Per-input validity (tree validation is async, so it's tracked as state).
const inputValid = reactive({});
// The matched preset reference for the treeStudies box, "" for none.
const currentPreset = ref("");

for (const input of props.template.inputs) {
  const boolProp = templateParamType(input.type).boolDisplay;
  if (boolProp) {
    templateInputs[input.name] = false;
    buttonTextStrings[input.name] = boolProp[1];
    inputValid[input.name] = true;
  } else {
    templateInputs[input.name] = "";
    inputValid[input.name] = false;
  }
}

const invalidInputCount = computed(
  () => props.template.inputs.filter((i) => !inputValid[i.name]).length,
);

// The generated result (engine call), refreshed whenever inputs change.
const templateScript = ref(null);

// `TemplateParamsIn` keys (serde camelCase; note finalEP → finalEp).
const PARAM_KEYS = { finalEP: "finalEp" };

async function refreshTemplate() {
  if (invalidInputCount.value !== 0) {
    templateScript.value = null;
    return;
  }
  const params = {};
  for (const input of props.template.inputs) {
    const typeObj = templateParamType(input.type);
    const value = templateInputs[input.name];
    params[PARAM_KEYS[input.name] ?? input.name] = typeObj.map
      ? typeObj.map(value)
      : value;
  }
  templateScript.value = await game.automatorTemplate(
    props.template.name,
    params,
  );
}

async function validateInput(input) {
  const typeObj = templateParamType(input.type);
  const value = templateInputs[input.name];
  if (input.type === "tree") {
    // A preset reference ("NAME xxxx" / "ID n") or a full import string.
    const preset = value.match(/^(NAME (.{1,4})|ID (\d))$/u);
    if (preset) {
      const byName = presets.value.some((p) => p.name === preset[2]);
      const byId = Number(preset[3]) > 0 && Number(preset[3]) < 7;
      inputValid[input.name] = byName || byId;
      currentPreset.value = inputValid[input.name] ? value : "";
    } else {
      currentPreset.value = "";
      inputValid[input.name] = await game.studyTreeIsValid(value);
    }
  } else if (typeObj.isValidString) {
    inputValid[input.name] = typeObj.isValidString(value);
  } else {
    inputValid[input.name] = true;
  }
}

async function updateTemplateProps() {
  await Promise.all(props.template.inputs.map(validateInput));
  await refreshTemplate();
}

watch(
  () => props.template,
  () => updateTemplateProps(),
);

function validityClass(input) {
  if (input.name === "treeStudies" && currentPreset.value !== "") {
    return "c-automator-template-textbox--preset";
  }
  return inputValid[input.name]
    ? undefined
    : "c-automator-template-textbox--invalid";
}

function loadPreset(name, id) {
  templateInputs.treeStudies = name ? `NAME ${name}` : `ID ${id}`;
  updateTemplateProps();
}

async function loadCurrent() {
  templateInputs.treeStudies = await game.studyTreeExport();
  updateTemplateProps();
}

function updateButton(input) {
  templateInputs[input.name] = !templateInputs[input.name];
  const boolProp = templateParamType(input.type).boolDisplay;
  buttonTextStrings[input.name] =
    boolProp[templateInputs[input.name] ? 0 : 1];
  updateTemplateProps();
}

const validWarnings = computed(() => templateScript.value?.warnings ?? []);

async function copyAndClose() {
  if (!templateScript.value) return;
  if (isBlock.value) {
    const blockified = await game.automatorBlockifyText(
      templateScript.value.script,
    );
    blockTemplates.value.push({
      name: `Template: ${props.template.name}`,
      blocks: blockified.blocks,
    });
    ui.notify("Custom template block created");
  } else {
    await navigator.clipboard.writeText(templateScript.value.script);
    ui.notify("Template copied to clipboard");
  }
  emit("close");
}
</script>

<template>
  <Modal
    class="c-automator-template-container"
    :title="`${template.name} Template`"
    centered
    @close="emit('close')"
  >
    <div class="c-automator-template-description">
      {{ template.description }}
    </div>
    <div class="c-automator-template-inputs">
      <b>Required Information:</b>
      <br>
      Use a preset Study Tree:
      <button
        v-for="(preset, presetNumber) in presets"
        :key="presetNumber"
        class="o-primary-btn o-load-preset-button-margin"
        @click="loadPreset(preset.name, presetNumber + 1)"
      >
        {{ preset.name ? preset.name : presetNumber + 1 }}
      </button>
      <button
        class="o-primary-btn o-load-preset-button-margin"
        @click="loadCurrent"
      >
        <i>Current Tree</i>
      </button>
      <div
        v-for="input in template.inputs"
        :key="input.name"
        class="c-automator-template-entry"
      >
        {{ input.prompt }}:
        <span v-if="templateParamType(input.type).boolDisplay">
          <button
            class="o-primary-btn"
            @click="updateButton(input)"
          >
            {{ buttonTextStrings[input.name] }}
          </button>
        </span>
        <span v-else>
          <input
            v-model="templateInputs[input.name]"
            type="text"
            class="c-automator-template-textbox"
            :class="validityClass(input)"
            @input="updateTemplateProps"
          >
        </span>
      </div>
    </div>
    <div class="c-automator-template-warnings">
      <b>Possible things to consider:</b>
      <div v-if="validWarnings.length !== 0">
        <div
          v-for="warning in validWarnings"
          :key="warning"
          class="c-automator-template-entry"
        >
          {{ warning }}
        </div>
      </div>
      <div v-else>
        (If something seems wrong with the template inputs, it will show up here)
      </div>
      <br>
      <br>
    </div>
    <button
      v-if="invalidInputCount === 0 && templateScript"
      class="o-primary-btn"
      @click="copyAndClose"
    >
      {{ isBlock ? "Create custom template block" : "Copy this template to your clipboard" }} and close this modal
    </button>
    <button
      v-else
      class="o-primary-btn o-primary-btn--disabled"
    >
      Cannot generate template (You have {{ invalidInputCount }} invalid
      {{ invalidInputCount === 1 ? "input" : "inputs" }})
    </button>
  </Modal>
</template>

<style scoped>
/* Vendored from the original AutomatorScriptTemplate.vue scoped style. */
.o-load-preset-button-margin {
  margin-right: 0.3rem;
}
</style>
