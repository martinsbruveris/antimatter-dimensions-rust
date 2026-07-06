<script setup>
import { computed } from "vue";

import { useGameStore } from "../stores/game";

// The "BREAK INFINITY" button. Mirrors BreakInfinityButton.vue: it is always
// shown, styled `--unavailable` (disabled look) until the Big Crunch autobuyer's
// interval is maxed (`break_infinity_unlockable`), and reads "INFINITY IS BROKEN"
// (unclickable) after breaking. (Enslaved's "FEEL ETERNITY" state isn't modelled.)
const game = useGameStore();
const s = computed(() => game.snapshot);

const isBroken = computed(() => Boolean(s.value?.broke_infinity));
const isUnlocked = computed(() => Boolean(s.value?.break_infinity_unlockable));

const classObject = computed(() => ({
  "o-infinity-upgrade-btn": true,
  "o-infinity-upgrade-btn--color-2": true,
  "o-infinity-upgrade-btn--available": isUnlocked.value,
  "o-infinity-upgrade-btn--unavailable": !isUnlocked.value,
  "o-infinity-upgrade-btn--unclickable": isBroken.value,
}));

const text = computed(() =>
  isBroken.value ? "INFINITY IS BROKEN" : "BREAK INFINITY",
);

function clicked() {
  if (!isBroken.value && isUnlocked.value) game.breakInfinity();
}
</script>

<template>
  <button
    :class="classObject"
    @click="clicked"
  >
    {{ text }}
  </button>
</template>
