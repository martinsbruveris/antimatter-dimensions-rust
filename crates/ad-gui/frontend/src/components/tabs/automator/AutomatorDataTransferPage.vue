<script setup>
// The extended data-transfer pane (vendored from
// AutomatorDataTransferPage.vue): per-script full-data export entries.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutomatorDataTransferSingleEntry from "./AutomatorDataTransferSingleEntry.vue";

const game = useGameStore();
const scripts = computed(() => game.snapshot.automator.scripts);
</script>

<template>
  <div class="l-panel-padding">
    This page lets you import and export scripts with additional data attached; the encoded text will also include data
    for any Time Study presets or constants used within the script. This will allow you to more easily transfer working
    scripts between different save files, but you may have to overwrite existing data in the process due to limited
    space for study presets and constants. Data exported from this page is also imported in the same way that single
    script data is imported.
    <br>
    <br>
    Note: Any mentions of constant names or full study-buying commands within comments will also be counted as being
    "used" within a script. This is intentional, as the comment is assumed to be indicative of what the script itself
    is attempting to do with presets or constants.
    <br>
    <br>
    <div
      v-for="script in scripts"
      :key="script.id"
    >
      <AutomatorDataTransferSingleEntry
        class="l-entry-margin"
        :script="script"
      />
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDataTransferPage.vue scoped style. */
.l-panel-padding {
  padding: 0.5rem 2rem 1rem 0;
}

.l-entry-margin {
  margin-bottom: 1rem;
}
</style>
