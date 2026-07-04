<script setup>
// "Modify Visible Tabs" popup (Visual options tab / the Tab key). Mirrors the
// original HiddenTabsModal + HiddenTabGroup + HiddenSubtabsButton
// (../antimatter-dimensions/src/components/modals/options/hidden-tabs): one
// row per unlocked tab holding its unlocked subtabs as toggle buttons plus a
// row-level indicator that toggles the whole tab. Hidden state lives in the
// engine's options (`hidden_tab_bits`/`hidden_subtab_bits`, original bit ids
// via config/tabs.js `hideId`); the ui store's visibleTabs/visibleSubtabs
// getters honour it. Guards mirrored from the original: the Options tab and
// the currently open tab/subtab cannot be hidden.
import { computed } from "vue";

import { TABS } from "../config/tabs";
import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();

// Every unlocked tab is listed — including hidden ones (so they can be
// unhidden) and the never-hidable Options tab.
const unlockedTabs = computed(() =>
  TABS.filter((t) => !t.condition || t.condition(game.snapshot))
);

function unlockedSubtabs(tab) {
  return tab.subtabs.filter((st) => !st.condition || st.condition(game.snapshot));
}

function isCurrentTab(tab) {
  return tab.key === ui.currentTabKey;
}

function isCurrentSubtab(tab, subtab) {
  return isCurrentTab(tab) && ui.currentSubtab.key === subtab.key;
}

// A tab that can never be hidden, or is the one currently open (the original's
// alwaysVisible): its row is grey and clicking it does nothing.
function tabAlwaysVisible(tab) {
  return tab.hidable === false || isCurrentTab(tab);
}

function rowClass(tab) {
  const hidden = ui.tabIsHidden(tab);
  return {
    "c-hide-modal-all-subtab-container": true,
    "l-hide-modal-subtab-container": true,
    "l-hide-modal-tab-container": true,
    "c-hidden-tabs-background__visible": !hidden,
    "c-hidden-tabs-background__hidden": hidden,
    "c-hidden-tabs-background__always-visible": tabAlwaysVisible(tab),
  };
}

function rowIndicatorClass(tab) {
  return {
    "c-indicator-icon": true,
    fas: true,
    "fa-check": !ui.tabIsHidden(tab),
    "fa-times": ui.tabIsHidden(tab),
    "fa-exclamation": tabAlwaysVisible(tab),
  };
}

function rowIndicatorTooltip(tab) {
  if (ui.tabIsHidden(tab)) return "Click to unhide tab";
  if (!tabAlwaysVisible(tab)) return "Click to hide tab";
  return "This tab cannot be hidden";
}

function subtabClass(tab, subtab) {
  const hidden = ui.subtabIsHidden(subtab);
  return {
    "c-hide-modal-tab-button": true,
    "c-hide-modal-button--active": !hidden,
    "c-hide-modal-button--inactive": hidden,
    "c-hide-modal-button--always-visible":
      subtab.hidable === false || isCurrentSubtab(tab, subtab),
    [`c-hide-modal-tab-button--${tab.key}`]: !isCurrentSubtab(tab, subtab),
  };
}

function subtabTooltip(tab, subtab) {
  if (subtab.hidable === false) return "Options tabs cannot be hidden";
  if (isCurrentSubtab(tab, subtab)) return "You cannot hide the tab you are on";
  return "";
}

function toggleSubtab(tab, subtab) {
  if (subtab.hidable === false || isCurrentSubtab(tab, subtab)) return;
  game.toggleSubtabVisibility(subtab.hideId[0], subtab.hideId[1]);
}

// Row toggle (the original HiddenTabGroup.toggleVisibility): unhiding a tab
// whose unlocked subtabs are all hidden also unhides every subtab, so the tab
// actually reappears.
function toggleTab(tab) {
  if (tabAlwaysVisible(tab)) return;
  const subtabs = unlockedSubtabs(tab);
  if (ui.tabIsHidden(tab) && subtabs.every((st) => ui.subtabIsHidden(st))) {
    for (const st of subtabs) {
      game.toggleSubtabVisibility(st.hideId[0], st.hideId[1]);
    }
    game.unhideTab(tab.hideId);
  } else {
    game.toggleTabVisibility(tab.hideId);
  }
}

function showAllTabs() {
  game.showAllTabs();
}
</script>

<template>
  <Modal
    title="Modify Visible Tabs"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="l-wrapper c-modal--short">
      Click a button to toggle showing a tab on/off.
      <br>
      Some tabs cannot be hidden, and you cannot hide your current tab.
      <br>
      Unhiding a tab in which all subtabs are hidden will also unhide all subtabs,
      and hiding all subtabs will also hide the tab.
      <br>
      <button
        class="o-primary-btn"
        @click="showAllTabs"
      >
        Show all tabs
      </button>
      <div
        v-for="tab in unlockedTabs"
        :key="tab.key"
        :class="rowClass(tab)"
        @click.self="toggleTab(tab)"
      >
        <div
          v-for="subtab in unlockedSubtabs(tab)"
          :key="subtab.key"
          :class="subtabClass(tab, subtab)"
          :title="subtabTooltip(tab, subtab)"
          @click="toggleSubtab(tab, subtab)"
        >
          <div class="l-hide-modal-button">
            <div
              class="l-hide-modal-button__subtab-icon"
              v-html="subtab.symbol"
            />
            <div class="l-hide-modal-button__subtab-name">
              {{ subtab.name }}
            </div>
          </div>
        </div>
        <div
          :class="rowIndicatorClass(tab)"
          :title="rowIndicatorTooltip(tab)"
          @click="toggleTab(tab)"
        />
      </div>
    </div>
  </Modal>
</template>

<style scoped>
/* Width from the original HiddenTabsModal's scoped .l-wrapper. */
.l-wrapper {
  width: 62rem;
  text-align: center;
}

/* The rules below live in the original HiddenTabGroup / HiddenSubtabsButton
   <style scoped> blocks (not the vendored global CSS). */
.c-indicator-icon {
  width: 2rem;
  position: absolute;
  top: 0;
  right: 0;
  color: black;
  text-shadow: none;
  padding: 0.2rem;
}

.c-hidden-tabs-background__visible {
  background-color: var(--color-good);
}

.c-hidden-tabs-background__hidden {
  background-color: var(--color-gh-purple);
}

.c-hidden-tabs-background__always-visible {
  background-color: var(--color-disabled);
  cursor: default;
}

.l-hide-modal-button {
  display: flex;
  flex-flow: row;
  align-items: center;
}

.l-hide-modal-button__subtab-icon {
  font-size: 1.5rem;
  width: 2rem;
  margin: 0.2rem;
}

.l-hide-modal-button__subtab-name {
  width: 8.2rem;
}
</style>
