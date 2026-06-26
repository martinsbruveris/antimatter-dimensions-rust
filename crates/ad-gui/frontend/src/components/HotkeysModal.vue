<script setup>
// Hotkey List popup, opened with the "?" key. Mirrors the original game's
// HotkeysModal.vue (../antimatter-dimensions/src/components/modals/options)
// — same two-column layout, markup and class names — trimmed to the
// mechanics this reimplementation has. Renders inside our shared Modal.vue
// wrapper (close button, overlay, pinned title).
import Modal from "./Modal.vue";
import { shortcuts } from "../data/shortcuts.js";

defineEmits(["close"]);

// Turn a key token into its displayed label, matching the original's
// format(): "mod" becomes the cross-platform CTRL/⌘, everything else is
// uppercased ("t" -> "T", "?" -> "?").
function format(key) {
  return key === "mod" ? "CTRL/⌘" : key.toUpperCase();
}
</script>

<template>
  <Modal
    title="Hotkey List"
    fit-content
    @close="$emit('close')"
  >
    <span class="c-modal-hotkeys l-modal-hotkeys">
      <div class="l-modal-hotkeys__column">
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Buy 1 Dimension</span>
          <kbd>SHIFT</kbd><kbd>1</kbd>-<kbd>SHIFT</kbd><kbd>8</kbd>
        </div>
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Buy 10 Dimensions</span>
          <kbd>1</kbd>-<kbd>8</kbd>
        </div>
        <div
          v-for="shortcut in shortcuts"
          :key="shortcut.name"
          class="l-modal-hotkeys-row"
        >
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">{{ shortcut.name }}</span>
          <kbd
            v-for="(key, i) in shortcut.keys"
            :key="i"
          >
            {{ format(key) }}
          </kbd>
        </div>
      </div>
      <div class="l-modal-hotkeys__column l-modal-hotkeys__column--right">
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Modifier Key</span>
          <kbd>SHIFT</kbd>
        </div>
        <span class="c-modal-hotkeys__shift-description">
          Shift is a modifier key that shows additional information on certain things
          and adjusts the function of certain buttons.
        </span>
        <br>
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Autobuyer Controls</span>
          <kbd>ALT</kbd>
        </div>
        <span class="c-modal-hotkeys__shift-description">
          Alt is a modifier key that, when pressed in conjunction with any key that has a corresponding autobuyer,
          will toggle said autobuyer.
          <br>
          When pressing both Alt and Shift, you can toggle buying singles or buying max for the Antimatter Dimension
          and Tickspeed Autobuyers instead.
        </span>
        <br>
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Tab Movement</span>
          <div>
            <kbd>←</kbd><kbd>↓</kbd><kbd>↑</kbd><kbd>→</kbd>
          </div>
        </div>
        <span class="c-modal-hotkeys__shift-description">
          Using the Arrow Keys will cycle you through the game's pages.
          The Up and Down arrows cycle you through tabs,
          and the Left and Right arrows cycle you through that tab's subtabs.
        </span>
        <br>
        <div class="l-modal-hotkeys-row">
          <span class="c-modal-hotkeys-row__name l-modal-hotkeys-row__name">Numpad Support</span>
        </div>
        <span class="c-modal-hotkeys__shift-description">
          Due to technical reasons, pressing a numpad key will purchase 10 of a Dimension if possible, but pressing
          a numpad key with <kbd>SHIFT</kbd> will not buy a single Dimension. It may instead, depending on your device,
          cause the page to scroll or change game tabs. <kbd>ALT</kbd> will still work as expected.
        </span>
      </div>
    </span>
  </Modal>
</template>

<style scoped>
/* Scoped layout copied from the original HotkeysModal.vue's <style scoped>
   block, which is not part of the vendored global CSS. */
.l-modal-hotkeys__column {
  display: flex;
  flex-direction: column;
  width: 28rem;
}

.l-modal-hotkeys__column--right {
  margin-left: 1rem;
}

.c-modal-hotkeys {
  font-size: 1.25rem;
}

.l-modal-hotkeys {
  display: flex;
  flex-direction: row;
}

.l-modal-hotkeys-row {
  display: flex;
  flex-direction: row;
  line-height: 1.6rem;
  padding-bottom: 0.3rem;
}

.c-modal-hotkeys-row__name {
  text-align: left;
}

.l-modal-hotkeys-row__name {
  flex: 1 1 auto;
}

.c-modal-hotkeys__shift-description {
  text-align: left;
  font-size: 1rem;
}
</style>
