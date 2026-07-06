<script setup>
// The Challenges → Normal Challenges subtab. Mirrors NormalChallengesTab.vue +
// ChallengeBox.vue + ChallengeTabHeader.vue: a grid of 12 challenge boxes plus an
// Exit/Restart header while a challenge runs. Layout + descriptions live in
// data/normalChallenges.js; the engine owns run/complete/unlock state. The
// show-all toggle and the start-confirmation modal are deferred.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { formatDecimal } from "../../util/format";
import { NORMAL_CHALLENGES } from "../../data/normalChallenges";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

// The corner "C1"-style IDs obey the "Challenge IDs" Info-Display option;
// holding Shift always shows them (original HintText type="challenges").
const showIds = computed(
  () => Boolean(s.value?.options?.show_hint_text?.challenges) || ui.shiftDown,
);

// Live challenge state (is_unlocked/is_running/is_completed) keyed by id.
const viewById = computed(
  () => new Map((s.value?.challenges ?? []).map((c) => [c.id, c])),
);

const anyRunning = computed(() =>
  (s.value?.challenges ?? []).some((c) => c.is_running),
);
const runningId = computed(
  () => (s.value?.challenges ?? []).find((c) => c.is_running)?.id ?? 0,
);

// Per-box derived state (mirrors ChallengeBox.vue's buttonClassObject/buttonText).
function box(meta) {
  const view = viewById.value.get(meta.id) ?? {};
  const isRunning = Boolean(view.is_running);
  const isCompleted = Boolean(view.is_completed);
  const isUnlocked = Boolean(view.is_unlocked);
  // C1 shows "Running" until completed while not in another challenge.
  const inC1 = meta.id === 1 && !isCompleted && !anyRunning.value;
  const locked = !(isCompleted || isRunning || inC1 || isUnlocked);
  const unenterable = !isUnlocked || isRunning || meta.id === 1;

  const classes = {
    "o-challenge-btn": true,
    "o-challenge-btn--running": isRunning || inC1,
    "o-challenge-btn--completed": isCompleted && isUnlocked,
    "o-challenge-btn--unlocked": !isCompleted && isUnlocked,
    "o-challenge-btn--locked": locked,
    "o-challenge-btn--unenterable": unenterable,
  };

  let text;
  if (isRunning || inC1) text = "Running";
  else if (isCompleted) text = "Completed";
  else if (isUnlocked) text = "Start";
  else {
    const inf = formatDecimal(s.value?.infinities, 0);
    text = `Locked (${inf}/${meta.lockedAt})`;
  }

  // A locked challenge hides its description behind the unlock requirement
  // (original NormalChallengeBox.descriptionDisplayConfig).
  const description = isUnlocked
    ? meta.description
    : `Infinity ${meta.lockedAt} times to unlock.`;

  return { meta, isUnlocked, isRunning, unenterable, classes, text, description };
}

const boxes = computed(() => NORMAL_CHALLENGES.map(box));

function onClick(b) {
  // Only enterable, non-running, non-C1 boxes start a challenge.
  if (b.unenterable) return;
  game.startChallenge(b.meta.id);
}

function exitChallenge() {
  game.exitChallenge();
}
function restartChallenge() {
  const id = runningId.value;
  game.exitChallenge();
  if (id >= 2) game.startChallenge(id);
}

// "Automatically retry challenges" (original retryChallenge): crunching inside an
// antimatter challenge re-enters it instead of exiting.
const retryChallenge = computed(() =>
  Boolean(s.value?.options?.retry_challenge),
);
function toggleRetryChallenge() {
  game.setRetryChallenge(!retryChallenge.value);
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
          class="o-primary-btn o-primary-btn--subtab-option"
          @click="toggleRetryChallenge"
        >
          Automatically retry challenges: {{ retryChallenge ? "ON" : "OFF" }}
        </button>
        <button
          v-if="anyRunning"
          class="o-primary-btn o-primary-btn--subtab-option"
          @click="restartChallenge"
        >
          Restart Challenge
        </button>
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
      Some Normal Challenges have requirements to be able to run that challenge.
    </div>
    <div>
      If you have an active Big Crunch Autobuyer, it will attempt to Crunch
      as soon as possible when reaching Infinite antimatter.
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
            C{{ b.meta.id }}
          </div>
          <span>{{ b.description }}</span>
          <div class="l-challenge-box__fill" />
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
