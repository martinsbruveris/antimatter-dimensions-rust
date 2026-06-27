<script setup>
// Top-right toast notifications. Mirrors the original's #notification-container
// (GameUiComponentFixed.vue) + core/notify.js: the `ui` store holds the live
// list and drives the enter/leave animation flags; this just renders them with
// the vendored o-notification / a-notification CSS. Clicking dismisses early.
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div
    id="notification-container"
    class="l-notification-container"
  >
    <div
      v-for="n in ui.notifications"
      :key="n.id"
      class="o-notification"
      :class="[
        n.typeClass,
        {
          'a-notification--enter': n.entering,
          'a-notification--leave': n.leaving,
        },
      ]"
      @click="ui.dismissNotification(n.id)"
    >
      {{ n.text }}
    </div>
  </div>
</template>
