<script setup>
// Sidebar resource picker for the Visual options tab. Like the notation
// picker, a simplified port of the original's ExpandingControlBox +
// SelectSidebarDropdown: a header button expanding an inline list of "Latest
// Resource" plus every unlocked resource. Selecting one writes the engine's
// `sidebarResourceID` option; SidebarCurrency.vue renders the choice (and the
// sidebar box itself also cycles it on click).
import { computed, ref } from "vue";

import { useGameStore } from "../../stores/game";
import { SIDEBAR_RESOURCES } from "../../config/sidebarResources";

const game = useGameStore();
const open = ref(false);

const available = computed(() =>
  SIDEBAR_RESOURCES.filter((r) => r.isAvailable(game.snapshot))
);

const label = computed(() => {
  const id = game.snapshot?.options?.sidebar_resource_id ?? 0;
  const name =
    id === 0
      ? "Latest Resource"
      : (available.value.find((r) => r.id === id)?.optionName ?? "Latest Resource");
  return `Sidebar (Modern UI): ${name}`;
});

function select(id) {
  game.setSidebarResource(id);
  open.value = false;
}
</script>

<template>
  <div class="l-expanding-control-box l-options-grid__button c-options-grid__notations">
    <div
      class="l-expanding-control-box__container"
      @mouseleave="open = false"
    >
      <div
        class="o-primary-btn o-primary-btn--option l-options-grid__notations-header"
        @click="open = !open"
      >
        {{ label }}
        <span
          class="c-indicator-arrow"
          :class="{ 'c-indicator-arrow--flipped': open }"
        >▼</span>
      </div>
      <div v-show="open">
        <div class="l-select-theme">
          <div class="l-select-theme__inner">
            <div
              key="Default"
              class="o-primary-btn l-select-theme__item c-select-theme__item"
              @click="select(0)"
            >
              Latest Resource
            </div>
            <div
              v-for="res in available"
              :key="res.id"
              class="o-primary-btn l-select-theme__item c-select-theme__item"
              @click="select(res.id)"
            >
              {{ res.optionName }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original ExpandingControlBox.vue scoped style (these
   layout classes are not in the global vendored CSS); same block as
   SelectNotationDropdown.vue. */
.l-expanding-control-box {
  position: relative;
  z-index: 3;
  height: 5.5rem;
}

.l-expanding-control-box__container {
  display: block;
  width: 100%;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.o-primary-btn--option {
  cursor: pointer;
}

.c-indicator-arrow {
  margin-left: 0.6rem;
  transition: transform 0.25s ease-out;
}

.c-indicator-arrow--flipped {
  transform: rotate(-180deg);
}
</style>
