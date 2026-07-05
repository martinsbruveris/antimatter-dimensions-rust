<script setup>
// The left pane: controls bar + the text editor (vendored from
// AutomatorEditor.vue; the block editor is Stage E).
import AutomatorControls from "./AutomatorControls.vue";
import AutomatorTextEditor from "./AutomatorTextEditor.vue";
</script>

<template>
  <div class="l-automator-pane">
    <AutomatorControls />
    <AutomatorTextEditor />
  </div>
</template>
