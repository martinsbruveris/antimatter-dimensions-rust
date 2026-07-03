<script setup>
// Mirrors HeaderChallengeDisplay.vue: the first line of the information-header,
// visible from the first Infinity — "You are currently in the Antimatter
// Universe (no active challenges)" or the running challenge's name (clickable,
// navigates to its tab) plus an Exit Challenge button. Only the NC/IC parts of
// the original's restriction list exist here; the retry-challenge option and
// the exit-confirmation modal are not built yet, so the button always reads
// "Exit Challenge" and exits directly.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { NORMAL_CHALLENGES } from "../data/normalChallenges";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

const runningNormal = computed(
  () => (s.value?.challenges ?? []).find((c) => c.is_running)?.id ?? 0
);
const runningInfinity = computed(
  () => (s.value?.infinity_challenges ?? []).find((c) => c.is_running)?.id ?? 0
);

// Outermost restriction first, like the original's `parts` order (IC, then NC).
const activeChallengeNames = computed(() => {
  const names = [];
  if (runningInfinity.value > 0) {
    names.push(`Infinity Challenge ${runningInfinity.value}`);
  }
  if (runningNormal.value > 0) {
    const name = NORMAL_CHALLENGES.find((c) => c.id === runningNormal.value)?.name;
    names.push(`${name} Challenge`);
  }
  return names;
});

const isVisible = computed(
  () => Boolean(s.value?.infinity_unlocked) || activeChallengeNames.value.length > 0
);

const challengeDisplay = computed(() =>
  activeChallengeNames.value.length === 0
    ? "the Antimatter Universe (no active challenges)"
    : activeChallengeNames.value.join(" + ")
);

const showExit = computed(() => activeChallengeNames.value.length !== 0);

// Bring the player to the tab of the innermost active challenge.
function textClicked() {
  if (runningNormal.value > 0) ui.setSubtab("challenges", "normal");
  else if (runningInfinity.value > 0) ui.setSubtab("challenges", "infinity");
}
</script>

<template>
  <div
    v-if="isVisible"
    class="l-game-header__challenge-text"
  >
    <span
      :class="{
        'l-challenge-display': true,
        'l-challenge-display--clickable': showExit,
      }"
      @click="textClicked"
    >
      You are currently in {{ challengeDisplay }}
    </span>
    <span class="l-padding-line" />
    <button
      v-if="showExit"
      class="o-primary-btn"
      @click="game.exitChallenge()"
    >
      Exit Challenge
    </button>
  </div>
</template>

<style scoped>
/* From the original component's scoped styles. */
.l-game-header__challenge-text {
  display: flex;
  height: 2rem;
  top: 50%;
  justify-content: center;
  align-items: center;
  font-size: 1.2rem;
  font-weight: bold;
  color: var(--color-text);
  margin: 0.5rem;
}

.l-challenge-display {
  padding: 0.5rem;
  cursor: default;
}

.l-challenge-display--clickable {
  cursor: pointer;
  user-select: none;
}

.l-challenge-display--clickable:hover {
  text-decoration: underline;
}

.l-padding-line {
  padding: 0.3rem;
}
</style>
