<script setup>
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";

// The two global controls at the top of the Autobuyers tab: the global
// pause/resume switch and the "Enable/Disable all" button (which only affects
// already-unlocked autobuyers).
const game = useGameStore();
const ab = computed(() => game.snapshot.autobuyers);

const unlockedEntries = computed(() => {
  const list = ab.value.dimensions.filter((d) => d.is_bought);
  if (ab.value.tickspeed.is_bought) list.push(ab.value.tickspeed);
  return list;
});

const allDisabled = computed(
  () =>
    unlockedEntries.value.length > 0 &&
    unlockedEntries.value.every((e) => !e.is_active)
);
</script>

<template>
  <div class="c-subtab-option-container">
    <button
      class="o-primary-btn o-primary-btn--subtab-option"
      @click="game.toggleAutobuyers()"
    >
      {{ ab.enabled ? "Pause autobuyers" : "Resume autobuyers" }}
    </button>
    <button
      class="o-primary-btn o-primary-btn--subtab-option"
      @click="game.setAllAutobuyersActive(allDisabled)"
    >
      {{ allDisabled ? "Enable" : "Disable" }} all autobuyers
    </button>
  </div>
</template>
