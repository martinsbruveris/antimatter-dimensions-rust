<script setup>
// The Challenges → Infinity Challenges subtab: a grid of 8 challenge boxes plus an
// Exit header while one runs. Mirrors the Normal Challenges subtab; live
// unlocked/running/completed state comes from the engine snapshot, layout +
// descriptions from data/infinityChallenges.js.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { INFINITY_CHALLENGES } from "../../data/infinityChallenges";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

// The corner "IC1"-style IDs obey the "Challenge IDs" Info-Display option;
// holding Shift always shows them (original HintText type="challenges").
const showIds = computed(
  () => Boolean(s.value?.options?.show_hint_text?.challenges) || ui.shiftDown,
);

const viewById = computed(
  () => new Map((s.value?.infinity_challenges ?? []).map((c) => [c.id, c])),
);
const anyRunning = computed(() =>
  (s.value?.infinity_challenges ?? []).some((c) => c.is_running),
);

function box(meta) {
  const view = viewById.value.get(meta.id) ?? {};
  const isRunning = Boolean(view.is_running);
  const isCompleted = Boolean(view.is_completed);
  const isUnlocked = Boolean(view.is_unlocked);
  const unenterable = !isUnlocked || isRunning;

  const classes = {
    "o-challenge-btn": true,
    "o-challenge-btn--running": isRunning,
    "o-challenge-btn--completed": isCompleted && isUnlocked,
    "o-challenge-btn--unlocked": !isCompleted && isUnlocked,
    "o-challenge-btn--locked": !(isCompleted || isRunning || isUnlocked),
    "o-challenge-btn--unenterable": unenterable,
  };

  let text;
  if (isRunning) text = "Running";
  else if (isCompleted) text = "Completed";
  else if (isUnlocked) text = "Start";
  else text = `Reach ${meta.unlockAM} antimatter to unlock`;

  return { meta, unenterable, classes, text };
}

const boxes = computed(() => INFINITY_CHALLENGES.map(box));

function onClick(b) {
  if (b.unenterable) return;
  game.startInfinityChallenge(b.meta.id);
}

function exitChallenge() {
  game.exitChallenge();
}
</script>

<template>
  <div
    v-if="s"
    class="l-challenges-tab"
  >
    <div class="l-challenges-tab__header">
      <div class="c-subtab-option-container">
        <button
          v-if="anyRunning"
          class="o-primary-btn o-primary-btn--subtab-option"
          @click="exitChallenge"
        >
          Exit Challenge
        </button>
      </div>
    </div>
    <div>
      Infinity Challenges are unlocked by reaching enough antimatter, and starting
      one breaks Infinity.
    </div>

    <div class="l-challenge-grid">
      <div
        v-for="b in boxes"
        :key="b.meta.id"
        class="l-challenge-grid__cell"
      >
        <div class="c-challenge-box l-challenge-box c-challenge-box--normal">
          <div
            v-show="showIds"
            class="o-hint-text l-hint-text l-hint-text--challenge"
          >
            IC{{ b.meta.id }}
          </div>
          <span>{{ b.meta.description }}</span>
          <div class="l-challenge-box__fill" />
          <div class="o-hint-text">
            Goal: {{ b.meta.goal }} antimatter
          </div>
          <button
            :class="b.classes"
            @click="onClick(b)"
          >
            {{ b.text }}
          </button>
          <span>Reward: {{ b.meta.reward }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
