<script setup>
// "About to do a Dimension Boost Reset" confirmation. Mirrors the original
// DimensionBoostModal.vue; Confirm performs the boost.
import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

function confirm() {
  game.buyDimBoost();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="You are about to do a Dimension Boost Reset"
    option="dimensionBoost"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    This will reset your Antimatter and Antimatter Dimensions. Are you sure you
    want to do this?
  </ConfirmModal>
</template>
