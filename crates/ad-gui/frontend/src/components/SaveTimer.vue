<script setup>
// Bottom-left "Time since last save" timer, mirroring the original game's
// SaveTimer.vue (../antimatter-dimensions/src/components/SaveTimer.vue), which is
// mounted as a fixed overlay by GameUiComponentFixed.vue. It shows the elapsed
// time since the last local save in the original's short (HH:MM:SS) format,
// clicking it saves the game, and it is gated by the `showTimeSinceSave` option.
// Cloud-save state is omitted (no cloud saves in our build).
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { timeDisplayShort } from "../util/format";

const game = useGameStore();

// Gated by the player option (original `player.options.showTimeSinceSave`).
const show = computed(() => game.snapshot?.options?.show_time_since_save);
// Store's per-tick clock keeps this reactive without a separate timer.
const timeString = computed(() => timeDisplayShort(game.msSinceSave));

function save() {
  game.saveGame();
}
</script>

<template>
  <div
    v-if="show"
    class="o-save-timer"
    @click="save"
  >
    <span>Time since last save: {{ timeString }}</span>
  </div>
</template>

<style scoped>
/* Replicated verbatim from the original SaveTimer.vue's <style scoped> block
   (these classes are not part of the vendored global CSS). The cloud-save and
   theme-specific (t-s2/t-s3) rules are dropped. */
.o-save-timer {
  white-space: nowrap;
  position: absolute;
  bottom: 0;
  left: 0;
  z-index: 5;
  text-align: left;
  color: var(--color-text);
  background-color: var(--color-base);
  border-top: 0.1rem solid var(--color-accent);
  border-right: 0.1rem solid var(--color-accent);
  padding: 0 0.5rem;
  pointer-events: auto;
  -webkit-user-select: none;
  user-select: none;
  cursor: pointer;
}
</style>
