<script setup>
// "How to Play" popup, opened with the "?" corner button or the H key.
// Mirrors the original game's H2PModal.vue
// (../antimatter-dimensions/src/components/modals/H2PModal.vue) — same
// two-pane layout (a searchable tab list on the left, the selected entry's
// body on the right) and the same vendored class names. Renders inside our
// shared Modal.vue wrapper (overlay, close button, pinned title).
import { ref, computed, onMounted } from "vue";

import { useUiStore } from "../stores/ui";
import { useGameStore } from "../stores/game";
import { h2pTabs } from "../data/h2p.js";
import Modal from "./Modal.vue";

const ui = useUiStore();
const game = useGameStore();

const searchValue = ref("");

// Unlock flags derived from the engine snapshot, mirroring the original's
// progress predicates. Passed to each entry's `isUnlocked`/`info`. Falsy
// before the first snapshot arrives (the modal only opens mid-game).
const flags = computed(() => ({
  tickspeedUnlocked: game.snapshot?.tickspeed_unlocked ?? false,
  sacrificeUnlocked: game.snapshot?.sacrifice_unlocked ?? false,
  infinityUnlocked: game.snapshot?.infinity_unlocked ?? false,
}));

// Entries unlocked at the current progress, mirroring the original filtering
// the tab list by `tab.isUnlocked()`.
const unlockedTabs = computed(() =>
  h2pTabs.filter((entry) => entry.isUnlocked(flags.value)),
);

// Of those, the ones whose name or tags contain the search text — the
// original's relevance-ranked search trimmed to a simple substring filter.
const matchingTabs = computed(() => {
  const query = searchValue.value.trim().toLowerCase();
  if (!query) return unlockedTabs.value;
  return unlockedTabs.value.filter(
    (entry) =>
      entry.name.toLowerCase().includes(query) ||
      entry.tags.some((tag) => tag.includes(query)),
  );
});

// Select the (unlocked) entry whose `tab` matches the currently open
// tab/subtab, like the original opening on the most relevant page;
// otherwise fall back to the first unlocked entry ("This Modal").
const tabKey = ui.currentTabKey;
const subtabKey = ui.currentSubtab.key;
const open = unlockedTabs.value;
const initial =
  open.find((e) => e.tab === `${tabKey}/${subtabKey}` || e.tab === tabKey) ??
  open[0];
const activeTab = ref(initial);

// Resolve an entry's body: a plain string, or a function of the unlock flags
// (the original's `info()`), used where the body grows with progress.
const activeBody = computed(() => {
  const info = activeTab.value.info;
  return typeof info === "function" ? info(flags.value) : info;
});

const bodyEl = ref(null);

function setActiveTab(entry) {
  activeTab.value = entry;
  if (bodyEl.value) bodyEl.value.scrollTop = 0;
}

const searchInput = ref(null);
onMounted(() => searchInput.value?.select());
</script>

<template>
  <Modal
    title="How To Play"
    fit-content
    @close="ui.closeModal()"
  >
    <div class="l-h2p-container">
      <div class="l-h2p-search-tab">
        <input
          ref="searchInput"
          v-model="searchValue"
          placeholder="Type to search..."
          class="c-h2p-search-bar"
          @keyup.esc="ui.closeModal()"
        >
        <div class="l-h2p-tab-list">
          <div
            v-for="entry in matchingTabs"
            :key="entry.name"
            class="o-h2p-tab-button"
            :class="{ 'o-h2p-tab-button--selected': entry === activeTab }"
            @click="setActiveTab(entry)"
          >
            {{ entry.name }}
          </div>
        </div>
      </div>
      <div class="l-h2p-info">
        <div class="c-h2p-body--title">
          {{ activeTab.name }}
        </div>
        <!-- v-html: bodies are in-repo HTML (see data/h2p.js), the same way
             the original renders activeTab.info(). -->
        <div
          ref="bodyEl"
          class="l-h2p-body c-h2p-body"
          v-html="activeBody"
        />
      </div>
    </div>
  </Modal>
</template>

<style scoped>
/* Size the two-pane layout to fill the modal, matching the original's
   .l-h2p-modal/.l-h2p-container (which take up most of the viewport). The
   tab list and body each scroll internally via the vendored global classes. */
.l-h2p-container {
  display: flex;
  flex-direction: row;
  width: calc(100vw - 30vh);
  max-width: 90vw;
  height: 60vh;
  margin: 0;
}

.l-h2p-search-tab {
  flex: 0 0 16rem;
}

.l-h2p-info {
  margin-left: 1rem;
}
</style>
