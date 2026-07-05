<script setup>
// Hover/right-click context menu, a Vue 3 port of the original tt-shop
// HoverMenu.vue. Visibility is local state (we have no global context-menu
// registry); the show/hide timers mirror the original's 250 ms / 500 ms
// delays. `close` is exposed so the owner can dismiss the menu after a
// menu-item action (the original mutates the global context-menu id).
import { onUnmounted, ref } from "vue";

const visible = ref(false);
let showTimer = null;
let hideTimer = null;

function clearTimers() {
  if (showTimer) clearTimeout(showTimer);
  if (hideTimer) clearTimeout(hideTimer);
  showTimer = null;
  hideTimer = null;
}

function startShowTimer() {
  if (hideTimer) clearTimeout(hideTimer);
  hideTimer = null;
  if (visible.value || showTimer) return;
  showTimer = setTimeout(() => {
    showTimer = null;
    visible.value = true;
  }, 250);
}

function startHideTimer() {
  if (showTimer) clearTimeout(showTimer);
  showTimer = null;
  if (!visible.value || hideTimer) return;
  hideTimer = setTimeout(() => {
    hideTimer = null;
    visible.value = false;
  }, 500);
}

function toggle() {
  clearTimers();
  visible.value = !visible.value;
}

function close() {
  clearTimers();
  visible.value = false;
}

defineExpose({ close });
onUnmounted(clearTimers);
</script>

<template>
  <div
    class="hover-menu__wrapper"
    @mouseenter="startShowTimer"
    @mouseleave="startHideTimer"
    @touchstart="startShowTimer"
    @contextmenu.prevent="toggle"
  >
    <slot name="object" />
    <slot
      v-if="visible"
      name="menu"
    />
  </div>
</template>

<style scoped>
.hover-menu__wrapper {
  position: relative;
}
</style>
