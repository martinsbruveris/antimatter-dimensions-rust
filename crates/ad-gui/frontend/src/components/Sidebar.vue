<script setup>
import { useUiStore } from "../stores/ui";
import SidebarCurrency from "./SidebarCurrency.vue";

const ui = useUiStore();
</script>

<template>
  <div class="c-modern-sidebar">
    <SidebarCurrency />
    <div
      v-for="tab in ui.visibleTabs"
      :key="tab.key"
      class="o-tab-btn o-tab-btn--modern-tabs o-tab-btn--subtabs"
      :class="[tab.uiClass, { 'o-tab-btn--active': ui.currentTabKey === tab.key }]"
    >
      <div
        class="l-tab-btn-inner"
        @click="ui.setTab(tab.key)"
      >
        {{ tab.name }}
        <div
          v-if="ui.tabHasNotification(tab)"
          class="fas fa-circle-exclamation l-notification-icon"
        />
      </div>
      <div class="subtabs">
        <div
          v-for="subtab in ui.visibleSubtabs(tab)"
          :key="subtab.key"
          class="o-tab-btn o-tab-btn--subtab"
          :class="[
            tab.uiClass,
            {
              'o-subtab-btn--active':
                ui.currentTabKey === tab.key && ui.currentSubtab.key === subtab.key,
            },
          ]"
          @click="ui.setSubtab(tab.key, subtab.key)"
        >
          <!-- eslint-disable-next-line vue/no-v-html -->
          <span v-html="subtab.symbol" />
          <div
            v-if="ui.subtabHasNotification(tab, subtab)"
            class="fas fa-circle-exclamation l-notification-icon"
          />
          <div class="o-subtab__tooltip">{{ subtab.name }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from ModernTabButton.vue scoped style (not in the global
   vendored CSS): the active-tab accent bar, active-subtab underline, and
   rounded ends on the subtab flyout. */
.o-tab-btn::before {
  content: "";
  width: 0;
  height: 100%;
  position: absolute;
  right: 0;
  left: 0;
  background-color: var(--color-accent);
  transition: width 0.15s;
}

.o-tab-btn--active::before {
  width: 0.5rem;
}

/* Prestige tabs tint the active-tab accent bar to match their text/symbol
   colour (original ModernTabButton.vue scoped style). */
.o-tab-btn--infinity::before {
  background-color: var(--color-infinity);
}

.o-tab-btn--eternity::before {
  background-color: var(--color-eternity);
}

.o-tab-btn--reality::before {
  background-color: var(--color-reality);
}

.o-tab-btn--celestial::before {
  background-color: var(--color-celestials);
}

.o-subtab-btn--active {
  border-bottom-width: 0.5rem;
}

.o-tab-btn--subtab:first-child {
  border-top-left-radius: var(--var-border-radius, 0.5rem);
  border-bottom-left-radius: var(--var-border-radius, 0.5rem);
  transition: border-radius 0s;
}

.o-tab-btn--subtab:last-child {
  border-top-right-radius: var(--var-border-radius, 0.5rem);
  border-bottom-right-radius: var(--var-border-radius, 0.5rem);
  transition: border-radius 0s;
}
</style>
