<script setup>
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import AutobuyerToggles from "./autobuyers/AutobuyerToggles.vue";
import DimensionAutobuyerBox from "./autobuyers/DimensionAutobuyerBox.vue";
import PrestigeAutobuyerBox from "./autobuyers/PrestigeAutobuyerBox.vue";
import TickspeedAutobuyerBox from "./autobuyers/TickspeedAutobuyerBox.vue";

// Pre-Infinity Autobuyers tab. The antimatter dimension and tickspeed autobuyers
// unlock with antimatter; the Dim Boost / Galaxy / Big Crunch autobuyers unlock
// by completing Normal Challenges 10/11/12. Interval upgrades (Infinity Points)
// become available once each autobuyer's challenge is completed.
const game = useGameStore();
const auto = computed(() => game.snapshot.autobuyers);
</script>

<template>
  <div class="l-autobuyers-tab">
    <AutobuyerToggles />
    <div>
      Complete Normal Challenges to unlock the prestige autobuyers and to upgrade
      autobuyer intervals with Infinity Points.
    </div>
    <b>Autobuyers with no displayed bulk have unlimited bulk by default.</b>
    <b>
      Antimatter Dimension Autobuyers can have their bulk upgraded once interval
      is below 100 ms.
    </b>
    <TickspeedAutobuyerBox />
    <DimensionAutobuyerBox
      v-for="tier in 8"
      :key="tier"
      :tier="tier - 1"
    />
    <PrestigeAutobuyerBox
      :entry="auto.dim_boost"
      target="dimBoost"
    />
    <PrestigeAutobuyerBox
      :entry="auto.galaxy"
      target="galaxy"
    />
    <PrestigeAutobuyerBox
      :entry="auto.big_crunch"
      target="bigCrunch"
    />
  </div>
</template>
