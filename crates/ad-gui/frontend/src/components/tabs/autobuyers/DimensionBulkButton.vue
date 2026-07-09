<script setup>
// The AD autobuyer's bulk-upgrade button, mirroring the original
// `DimensionBulkButton.vue`: once the interval is maxed, IP purchases double
// the "Buys max" bulk ("×N ➜ ×2N bulk buy / Cost") up to the 512 cap; with
// Achievement 61 (unlimited bulk) nothing renders.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";

const props = defineProps({
  entry: { type: Object, required: true },
  tier: { type: Number, required: true },
});

const game = useGameStore();

const BULK_CAP = 512;

const bulkDisplay = computed(() => {
  if (props.entry.has_maxed_bulk) {
    return `×${props.entry.bulk} bulk buy (capped)`;
  }
  const newBulk = Math.min(props.entry.bulk * 2, BULK_CAP);
  return `×${props.entry.bulk} ➜ ×${newBulk} bulk buy`;
});

const btnClass = computed(() => ({
  "o-autobuyer-btn": true,
  "o-autobuyer-btn--unavailable":
    !props.entry.can_afford_bulk_upgrade && !props.entry.has_maxed_bulk,
  "o-non-clickable": props.entry.has_maxed_bulk,
}));

function onClick() {
  if (!props.entry.can_afford_bulk_upgrade) return;
  game.upgradeAdAutobuyerBulk(props.tier);
}
</script>

<template>
  <button
    v-if="entry.has_maxed_interval && !entry.has_unlimited_bulk"
    :class="btnClass"
    @click="onClick"
  >
    <span>{{ bulkDisplay }}</span>
    <template v-if="!entry.has_maxed_bulk">
      <br>
      <span>Cost: {{ formatDecimal(entry.upgrade_cost, 2) }} IP</span>
    </template>
  </button>
</template>

<style scoped>
/* From the original DimensionBulkButton.vue's scoped style. */
.o-non-clickable {
  cursor: auto;
}
</style>
