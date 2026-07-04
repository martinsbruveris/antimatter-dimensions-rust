<script setup>
// Enter/exit Time Dilation confirmation. Mirrors the original
// modals/DilationModal.vue / prestige ExitDilation flow: entering explains the
// dilated run; exiting shows the pending Tachyon Particle gain. The disable
// checkbox flips `confirmations.dilation`.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";
import { formatDecimal } from "../util/format";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);
const active = computed(() => Boolean(s.value?.dilation?.active));

function confirm() {
  game.toggleDilation();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    :title="active ? 'Exit Time Dilation' : 'You are about to Dilate time'"
    option="dilation"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    <template v-if="!active">
      Dilating time will start a modified Eternity, where all of your Antimatter,
      Infinity, and Time Dimension multipliers, as well as tickspeed, will be
      exponentially compressed. If you can reach {{ formatDecimal({ m: 1.79769, e: 308 }, 2, 2) }}
      Infinity Points to complete this dilated Eternity, you will be rewarded with
      Tachyon Particles based on your antimatter reached.
    </template>
    <template v-else>
      Exiting Time Dilation performs an Eternity
      <template v-if="s.can_eternity">
        and rewards {{ formatDecimal(s.dilation.tachyon_gain, 2, 1) }} Tachyon Particles.
      </template>
      <template v-else>
        . You have not reached the Eternity goal, so you will gain no Tachyon
        Particles.
      </template>
    </template>
  </ConfirmModal>
</template>
