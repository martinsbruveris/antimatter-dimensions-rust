<script setup>
// The Challenges → Eternity Challenges subtab: 12 challenge boxes with
// completion counts (×5), scaled goals, and Start/Running/Completed state,
// plus an Exit header while one runs. Mirrors the original
// EternityChallengesTab; live state comes from the engine snapshot, layout +
// descriptions from data/eternityChallenges.js.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { useUiStore } from "../../stores/ui";
import { formatDecimal } from "../../util/format";
import { ETERNITY_CHALLENGES } from "../../data/eternityChallenges";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

const showIds = computed(
  () => Boolean(s.value?.options?.show_hint_text?.challenges) || ui.shiftDown,
);

const viewById = computed(
  () => new Map((s.value?.eternity_challenges ?? []).map((c) => [c.id, c])),
);
const anyRunning = computed(() =>
  (s.value?.eternity_challenges ?? []).some((c) => c.is_running),
);
const totalCompletions = computed(() =>
  (s.value?.eternity_challenges ?? []).reduce((sum, c) => sum + c.completions, 0),
);

function box(meta) {
  const view = viewById.value.get(meta.id) ?? {};
  const isRunning = Boolean(view.is_running);
  const isUnlocked = Boolean(view.is_unlocked);
  const completions = view.completions ?? 0;
  const fullyCompleted = completions >= 5;
  const unenterable = !isUnlocked || isRunning;

  const classes = {
    "o-challenge-btn": true,
    "o-challenge-btn--running": isRunning,
    "o-challenge-btn--completed": fullyCompleted && isUnlocked,
    "o-challenge-btn--unlocked": !fullyCompleted && isUnlocked,
    "o-challenge-btn--locked": !(isRunning || isUnlocked),
    "o-challenge-btn--unenterable": unenterable,
  };

  let text;
  if (isRunning) text = "Running";
  else if (isUnlocked) text = fullyCompleted ? "Completed" : "Start";
  else text = "Locked (unlock via Time Studies)";

  return {
    meta,
    view,
    completions,
    unenterable,
    classes,
    text,
    restriction: meta.restriction ? meta.restriction(completions) : null,
  };
}

const boxes = computed(() => ETERNITY_CHALLENGES.map(box));

function onClick(b) {
  if (b.unenterable) return;
  game.startEternityChallenge(b.meta.id);
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
          @click="game.exitEternityChallenge()"
        >
          Exit Challenge
        </button>
      </div>
    </div>
    <div>
      Eternity Challenges are unlocked by purchasing their study in the Time
      Studies tree. Complete one by reaching its Infinity Point goal and
      performing an Eternity; each can be completed up to 5 times with growing
      goals. Total completions: {{ totalCompletions }}/60.
    </div>

    <div class="l-challenge-grid">
      <div
        v-for="b in boxes"
        :key="b.meta.id"
        class="l-challenge-grid__cell"
      >
        <div class="c-challenge-box l-challenge-box c-challenge-box--eternity">
          <div
            v-show="showIds"
            class="o-hint-text l-hint-text l-hint-text--challenge"
          >
            EC{{ b.meta.id }}
          </div>
          <span>{{ b.meta.description }}</span>
          <div class="l-challenge-box__fill" />
          <div class="o-hint-text">
            Goal: {{ formatDecimal(b.view.goal, 2) }} IP
            <template v-if="b.restriction">
              {{ b.restriction }}
            </template>
            <br>
            Completed {{ b.completions }}/5 {{ b.completions === 1 ? "time" : "times" }}
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
