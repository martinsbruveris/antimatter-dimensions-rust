<script setup>
// The error panel (vendored from AutomatorErrorPage.vue): the editor script's
// compile errors with tips and jump-to-line buttons.
import { computed } from "vue";

import { AutomatorTextUI, automatorErrors } from "../../../util/automatorEditor";

const errors = computed(() => automatorErrors.value);

function scrollToLine(line) {
  AutomatorTextUI.scrollToLine(line);
  AutomatorTextUI.updateHighlightedLine(line, "error");
}
</script>

<template>
  <div class="c-automator-docs-page">
    <div v-if="errors.length === 0">
      No script errors found!
    </div>
    <div v-else>
      <b>Your script has the following {{ errors.length === 1 ? "error" : `${errors.length} errors` }}:</b>
      <br>
      <span
        v-for="(error, i) in errors"
        :key="i"
      >
        <b>On line {{ error.line }}:</b>
        <button
          title="Jump to line"
          class="c-automator-docs--button fas fa-arrow-circle-right"
          @click="scrollToLine(error.line)"
        />
        <div class="c-automator-docs-page__indented">
          {{ error.info }}
        </div>
        <div class="c-automator-docs-page__indented">
          <i>Suggested fix: {{ error.tip }}</i>
        </div>
      </span>
      <i>
        Note: Sometimes errors may cause the automator to be unable to scan the rest of the script properly.
        This may result in some errors "disappearing" due to other errors occurring in earlier lines, or
        errors in a command which has an inner block (eg. commands like IF or WHILE) causing errors to appear
        on correctly-formatted later commands.
        Additionally, some of the suggested fixes may be potentially misleading due to the cause of
        the error being unclear.
      </i>
    </div>
  </div>
</template>
