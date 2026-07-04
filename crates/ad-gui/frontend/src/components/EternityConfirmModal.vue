<script setup>
// "You are about to Eternity" confirmation. Mirrors the original
// modals/prestige/EternityModal.vue (via its ResetModal shell): the reset
// explanation, the gained EP/Eternities line, and the disable checkbox
// (confirmations.eternity). The EC-completion variant arrives with Feature 4.5.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";
import { formatDecimal } from "../util/format";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

const message = computed(() =>
  s.value.eternity_unlocked
    ? `Eternity will reset everything except Achievements, Challenge records, and
      anything under the General header on the Statistics tab.`
    : `Eternity will reset everything except Achievements, Challenge records, and
      anything under the General header on the Statistics tab. You will also gain
      an Eternity Point and unlock various upgrades.`
);

const epIsOne = computed(
  () =>
    s.value.gained_eternity_points.m === 1 && s.value.gained_eternity_points.e === 0
);

function confirm() {
  game.eternity();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="You are about to Eternity"
    option="eternity"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    {{ message }}
    <br>
    <br>
    You will gain 1 Eternity and
    {{ formatDecimal(s.gained_eternity_points, 2) }} Eternity
    {{ epIsOne ? "Point" : "Points" }} on Eternity.
  </ConfirmModal>
</template>
