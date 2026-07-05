<script setup>
// "Start this Reality over" confirmation (modals/ResetRealityModal.vue): a
// forced, reward-free Reality reset.
import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

function confirm() {
  game.resetReality();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="Reset this Reality"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    Reset this Reality? You will gain no rewards — no Reality Machines, no
    Glyph, and no Perk Point — and everything below Reality will start over.
  </ConfirmModal>
</template>
