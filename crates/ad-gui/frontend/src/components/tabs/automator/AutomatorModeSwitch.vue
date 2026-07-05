<script setup>
// The block/text editor slider toggle (vendored from
// AutomatorModeSwitch.vue). Switching with errors, unparsable lines, or a
// running script asks for confirmation first (unless that confirmation is
// disabled).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { automatorErrors } from "../../../util/automatorEditor";
import {
  pendingModeSwitch,
  performModeSwitch,
} from "../../../util/blockAutomator";

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);
const isText = computed(() => auto.value.editor_type === "text");

const tooltip = computed(() =>
  isText.value ? "Switch to the block editor" : "Switch to the text editor",
);

async function toggle() {
  const confirmations = game.snapshot.options.confirmations;
  let hasTextErrors = false;
  let lostBlocks = 0;
  if (isText.value) {
    // Nothing is ever lost converting block → text, so only text mode checks.
    const blockified = await game.automatorBlockify(auto.value.editor_script);
    lostBlocks = blockified.lost_lines;
    hasTextErrors = lostBlocks > 0 || automatorErrors.value.length !== 0;
  }
  if (
    confirmations.switch_automator_mode &&
    (hasTextErrors || auto.value.is_running)
  ) {
    pendingModeSwitch.value = {
      lostBlocks,
      errorCount: automatorErrors.value.length,
    };
    return;
  }
  await performModeSwitch(game);
}
</script>

<template>
  <button
    :title="tooltip"
    :class="{
      'c-slider-toggle-button': true,
      'c-slider-toggle-button--right': isText,
    }"
    @click="toggle"
  >
    <i class="fas fa-cubes" />
    <i class="fas fa-code" />
  </button>
</template>

<style scoped>
/* Vendored from the original AutomatorModeSwitch.vue scoped style. */
.c-slider-toggle-button {
  display: flex;
  overflow: hidden;
  position: relative;
  align-items: center;
  color: var(--color-automator-docs-font);
  background-color: #626262;
  border: var(--var-border-width, 0.2rem) solid #767676;
  border-radius: var(--var-border-radius, 0.3rem);
  margin: 0.3rem 0.4rem 0.3rem 0.5rem;
  padding: 0.3rem 0;
  cursor: pointer;
}

.c-slider-toggle-button .fas {
  width: 3rem;
  position: relative;
  z-index: 1;
}

.c-slider-toggle-button::before {
  content: "";
  width: 3rem;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  z-index: 0;
  background-color: var(--color-automator-controls-inactive);
  border-radius: var(--var-border-radius, 0.3rem);
  transition: 0.3s ease all;
}

.c-slider-toggle-button--right::before {
  left: 3rem;
  background-color: var(--color-automator-controls-inactive);
}
</style>
