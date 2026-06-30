<script setup>
// First-infinity "You are about to Infinity" explanation. Mirrors the original
// BigCrunchModal.vue on the first infinity: explanatory copy and **no** disable
// checkbox (there is nothing to disable yet). Confirm performs the Big Crunch.
import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

function confirm() {
  game.bigCrunch();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="You are about to Infinity"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    Upon Infinity, all Dimensions, Dimension Boosts, and Antimatter Galaxies are
    reset. In return, you gain an Infinity Point (IP). This allows you to buy
    multiple upgrades that you can find in the Infinity tab. You will also gain
    one Infinity, which is the stat shown in the Statistics tab.
  </ConfirmModal>
</template>
