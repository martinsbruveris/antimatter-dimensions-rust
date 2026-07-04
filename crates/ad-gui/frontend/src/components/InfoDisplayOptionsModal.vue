<script setup>
// Info Display Options popup, opened from the Visual options tab. Mirrors the
// original InfoDisplayOptionsModal.vue (../antimatter-dimensions/src/
// components/modals/options): toggles for the overlay hints — the % gain on
// dimension rows, achievement IDs and unlock-state indicators, and challenge
// IDs (shown once Infinity is unlocked). Later hints (Time Study IDs, glyph
// dots, …) arrive with their layers. Holding Shift always shows the hint
// text regardless of these options (ui.shiftDown).
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import Modal from "./Modal.vue";
import ModalOptionsToggleButton from "./options/ModalOptionsToggleButton.vue";

defineEmits(["close"]);

const game = useGameStore();

const hints = computed(() => game.snapshot?.options?.show_hint_text ?? {});
const infinityUnlocked = computed(() =>
  Boolean(game.snapshot?.infinity_unlocked)
);
</script>

<template>
  <Modal
    title="Info Display Options"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="c-modal-options__large l-modal-options">
      <div class="c-modal-options__button-container">
        <ModalOptionsToggleButton
          text="Show % gain:"
          :model-value="Boolean(hints.show_percentage)"
          @update:model-value="game.setHintText('showPercentage', $event)"
        />
        <ModalOptionsToggleButton
          text="Achievement IDs:"
          :model-value="Boolean(hints.achievements)"
          @update:model-value="game.setHintText('achievements', $event)"
        />
        <ModalOptionsToggleButton
          text="Achievement unlock state indicators:"
          :model-value="Boolean(hints.achievement_unlock_states)"
          @update:model-value="game.setHintText('achievementUnlockStates', $event)"
        />
        <ModalOptionsToggleButton
          v-if="infinityUnlocked"
          text="Challenge IDs:"
          :model-value="Boolean(hints.challenges)"
          @update:model-value="game.setHintText('challenges', $event)"
        />
      </div>
      Note: All types of additional info above will always display when holding shift.
    </div>
  </Modal>
</template>
