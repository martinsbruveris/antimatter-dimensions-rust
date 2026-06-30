<script setup>
// The first Big Crunch screen. Mirrors the Modern-UI branch of the original
// game's BigCrunchButton.vue (src/components/ui-modes/BigCrunchButton.vue):
// the collapse message + the "Big Crunch" button. Shown in place of the whole
// game view once antimatter reaches the Big Crunch threshold.
import { useGameStore } from "../stores/game";

const game = useGameStore();

function crunch() {
  game.requestBigCrunch();
}
</script>

<template>
  <div class="l-big-crunch">
    <h3 class="l-big-crunch__message">
      The world has collapsed due to excess antimatter.
    </h3>
    <button
      class="btn-big-crunch"
      @click="crunch"
    >
      Big Crunch
    </button>
  </div>
</template>

<style scoped>
/* The button itself uses the vendored `.btn-big-crunch`. In the original the
   crunch screen sits at the very top of the content area, horizontally centred
   — the collapse text lines up with the "?" help button on the right. So we
   top-align with no extra top offset (the game-container's top padding already
   matches the "?" button's vertical position). */
.l-big-crunch {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

/* Plain h3 with no explicit font-size in the original (so it stays small) and
   no extra top margin, so it aligns with the "?" button. */
.l-big-crunch__message {
  margin-top: 0;
}
</style>
