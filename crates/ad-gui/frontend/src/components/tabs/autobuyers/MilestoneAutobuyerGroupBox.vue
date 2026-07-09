<script setup>
// A milestone-autobuyer group row (the Infinity Dimension and Replicanti-
// upgrade autobuyers), mirroring the original `MultipleAutobuyersBox.vue`: a
// group on/off toggle on the left, the group title, and one toggle per
// unlocked entry. Interval labels are omitted (all fire at the fixed 1 s).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import AutobuyerToggleFooter from "./AutobuyerToggleFooter.vue";

const props = defineProps({
  // The MilestoneAutobuyerGroupView snapshot slice.
  group: { type: Object, required: true },
  // "infinityDims" / "replicantiUpgrades" (the engine toggle handle).
  kind: { type: String, required: true },
  // Group title, e.g. "Infinity Dimension".
  name: { type: String, required: true },
});

const game = useGameStore();
const unlockedEntries = computed(() =>
  props.group.entries
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => entry.is_unlocked),
);
</script>

<template>
  <span
    v-if="group.any_unlocked"
    class="c-autobuyer-box-row"
  >
    <AutobuyerToggleFooter
      :is-active="group.group_active"
      @toggle="game.toggleMilestoneAutobuyer(kind)"
    />
    <div class="l-autobuyer-box__title">
      {{ name }}<br>Autobuyers
    </div>
    <div class="l-autobuyer-box__autobuyers">
      <span
        v-for="{ entry, index } in unlockedEntries"
        :key="index"
        class="c-autobuyer-box-slot"
      >
        <AutobuyerToggleFooter
          :is-active="entry.is_active && group.group_active"
          @toggle="game.toggleMilestoneAutobuyer(kind, index)"
        />
        {{ entry.name }}
      </span>
    </div>
  </span>
</template>
