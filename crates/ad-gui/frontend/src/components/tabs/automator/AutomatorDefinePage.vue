<script setup>
// The constants ("define") panel (vendored from AutomatorDefinePage.vue +
// AutomatorDefineSingleEntry.vue): a list of name 🠈 value rows, edited in
// place, with an empty trailing row for adding a new constant. The engine
// enforces the limits and reserved-word rules; commits happen on blur.
import { computed, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import { AutomatorTextUI } from "../../../util/automatorEditor";

const MAX_CONSTANTS = 30;
const MAX_NAME = 20;
const MAX_VALUE = 250;

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);

// Rows under edit: keyed by original name ("" = the new-constant row); local
// state while focused so the per-tick snapshot doesn't clobber typing.
const editing = ref(null); // { oldName, name, value } | null

const rows = computed(() => {
  const stored = auto.value.constants.map((c) => ({
    oldName: c.name,
    name: c.name,
    value: c.value,
  }));
  if (stored.length < MAX_CONSTANTS) {
    stored.push({ oldName: "", name: "", value: "" });
  }
  if (editing.value !== null) {
    const i = stored.findIndex((r) => r.oldName === editing.value.oldName);
    if (i !== -1) stored[i] = editing.value;
  }
  return stored;
});

const hasConstants = computed(() => auto.value.constants.length > 0);

function beginEdit(row) {
  if (editing.value?.oldName !== row.oldName) {
    editing.value = { ...row };
  }
}

async function commitEdit() {
  const row = editing.value;
  editing.value = null;
  if (!row) return;
  const name = row.name.trim();
  const value = row.value ?? "";
  if (!row.oldName && !name) return;
  if (!name) {
    // Cleared name = delete (the original's handleFocus branch).
    await game.automatorDeleteConstant(row.oldName);
  } else if (!row.oldName) {
    await game.automatorSetConstant(name, value);
  } else if (row.oldName === name) {
    await game.automatorSetConstant(name, value);
  } else {
    await game.automatorRenameConstant(row.oldName, name);
    await game.automatorSetConstant(name, value);
  }
  // Scripts referencing the constant revalidate immediately.
  AutomatorTextUI.refreshErrors();
}

async function deleteConstant(row) {
  editing.value = null;
  if (row.oldName) {
    await game.automatorDeleteConstant(row.oldName);
    AutomatorTextUI.refreshErrors();
  }
}
</script>

<template>
  <div class="l-panel-padding">
    This panel allows you to define case-sensitive constant values which can be used in place of numbers or Time Study
    import strings. These definitions are shared across all of your scripts and are limited to a maximum of
    {{ MAX_CONSTANTS }} defined constants. Additionally, constant names and values are limited to lengths of
    {{ MAX_NAME }} and {{ MAX_VALUE }} characters respectively. Changes made to constants will not apply
    until any currently running scripts are restarted.
    <br>
    <br>
    As a usage example, defining
    <b>first 🠈 11,21,22,31,32,33</b>
    allows you to use
    <b>studies purchase first</b>
    in order to purchase all of the studies in the first three rows.
    <br>
    <br>
    <span v-if="!hasConstants"><i>You have no defined constants.</i></span>
    <div class="l-definition-container">
      <div
        v-for="row in rows"
        :key="row.oldName"
        class="l-single-definition-container"
      >
        <input
          :value="row.name"
          class="c-define-textbox c-alias"
          :class="{ 'l-limit-textbox': row.name.length === MAX_NAME }"
          placeholder="New constant..."
          :maxlength="MAX_NAME"
          @focusin="beginEdit(row)"
          @input="beginEdit(row); editing.name = $event.target.value"
          @focusout="commitEdit"
        >
        <span
          v-if="row.name"
          class="o-arrow-padding"
        >
          🠈
        </span>
        <input
          v-if="row.name"
          :value="row.value"
          class="c-define-textbox c-value"
          :class="{ 'l-limit-textbox': row.value && row.value.length === MAX_VALUE }"
          placeholder="Value for constant..."
          :maxlength="MAX_VALUE"
          @focusin="beginEdit(row)"
          @input="beginEdit(row); editing.value = $event.target.value"
          @focusout="commitEdit"
        >
        <button
          v-if="row.oldName"
          title="Delete this constant"
          class="c-delete-button fas fa-eraser"
          @click="deleteConstant(row)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDefinePage.vue /
   AutomatorDefineSingleEntry.vue scoped styles. */
.l-panel-padding {
  padding: 0.5rem 2rem 0 0;
}

.l-definition-container {
  display: flex;
  flex-direction: column;
  border: solid 0.1rem var(--color-automator-docs-font);
  border-radius: var(--var-border-radius, 0.5rem);
  padding: 0.5rem;
  margin-top: 1rem;
}

.l-single-definition-container {
  display: flex;
  flex-direction: row;
  padding: 0.5rem;
}

.o-arrow-padding {
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 0 1rem;
}

.c-define-textbox {
  display: inline-block;
  font-family: Typewriter, serif;
  font-size: 1.1rem;
  background: var(--color-blockmator-block-background);
  border: 0.1rem solid var(--color-blockmator-block-border);
  border-radius: var(--var-border-radius, 0.5rem);
  padding: 0.5rem;
  color: #00ac00;
}

.l-limit-textbox {
  border-style: dotted;
  border-color: var(--color-automator-error-outline);
}

.c-alias {
  min-width: 14.5rem;
}

.c-value {
  width: 100%;
}

.c-delete-button {
  display: flex;
  justify-content: center;
  align-items: center;
  border: var(--var-border-width, 0.2rem) solid var(--color-automator-controls-border);
  border-radius: var(--var-border-radius, 0.3rem);
  margin: 0.1rem -0.4rem 0.1rem 0.6rem;
  cursor: pointer;
  color: var(--color-automator-docs-font);
  background-color: var(--color-automator-controls-inactive);
}

.c-delete-button:hover {
  background-color: var(--color-automator-error-background);
}
</style>
