<script setup>
// The "Press ? to open the hotkey list." link shown at the bottom of both
// options tabs, mirroring the original `OpenModalHotkeysButton.vue`. Clicking
// it opens the same hotkeys modal as the `?` shortcut.
import { useUiStore } from "../../stores/ui";

const ui = useUiStore();
</script>

<template>
  <p
    class="c-options-tab__hotkeys-link"
    @click="ui.showModal('hotkeys')"
  >
    Press <kbd>?</kbd> to open the hotkey list.
  </p>
</template>
