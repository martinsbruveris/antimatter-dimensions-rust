<script setup>
// "About to purchase an Antimatter Galaxy" confirmation. Mirrors the original
// AntimatterGalaxyModal.vue (pre-Infinity branch); Confirm buys the galaxy.
import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

function confirm() {
  game.buyGalaxy();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="You are about to purchase an Antimatter Galaxy"
    option="antimatterGalaxy"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    Are you sure you want to purchase an Antimatter Galaxy? This will reset your
    Antimatter Dimensions, Tickspeed and Dimension Boosts. However, you will
    receive a small boost to Tickspeed Upgrades.
  </ConfirmModal>
</template>
