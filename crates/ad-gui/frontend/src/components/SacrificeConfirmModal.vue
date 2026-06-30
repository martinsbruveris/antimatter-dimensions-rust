<script setup>
// "Dimensional Sacrifice" confirmation. Mirrors the original SacrificeModal.vue
// (pre-Achievement-118 branch); Confirm performs the sacrifice. Shows the
// current and post-sacrifice 8th-dimension multipliers.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { formatMultiplier } from "../util/format";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();
const s = computed(() => game.snapshot);

// Multiply two snapshot numbers ({m, e}), renormalising the mantissa into
// [1, 10) so the formatter gets a well-formed value.
function mul(a, b) {
  let m = a.m * b.m;
  let e = a.e + b.e;
  if (m !== 0) {
    while (Math.abs(m) >= 10) {
      m /= 10;
      e += 1;
    }
    while (Math.abs(m) < 1) {
      m *= 10;
      e -= 1;
    }
  }
  return { m, e };
}

// Current total boost, and the boost after sacrificing (nextBoost × total),
// matching the original's multiplierText.
const multiplierText = computed(() => {
  const current = s.value.sacrifice_multiplier;
  const next = mul(s.value.next_sacrifice_boost, s.value.sacrifice_multiplier);
  return `Multiplier is currently ×${formatMultiplier(current)} and will increase to ×${formatMultiplier(next)} on Dimensional Sacrifice.`;
});

function confirm() {
  game.sacrifice();
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    title="Dimensional Sacrifice"
    option="sacrifice"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    Dimensional Sacrifice will remove all of your 1st through 7th Antimatter
    Dimensions (with the cost and multiplier unchanged), for a boost to the 8th
    Antimatter Dimension based on the total amount of 1st Antimatter Dimensions
    sacrificed. It will take time to regain production.
    <br><br>
    {{ multiplierText }}
  </ConfirmModal>
</template>
