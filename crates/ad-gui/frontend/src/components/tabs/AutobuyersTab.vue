<script setup>
import AutobuyerToggles from "./autobuyers/AutobuyerToggles.vue";
import DimensionAutobuyerBox from "./autobuyers/DimensionAutobuyerBox.vue";
import TickspeedAutobuyerBox from "./autobuyers/TickspeedAutobuyerBox.vue";

// Pre-Infinity Autobuyers tab. Only the antimatter dimension and tickspeed
// autobuyers exist this early; the dimboost/galaxy/crunch autobuyers and
// interval upgrades arrive after the first Infinity.
</script>

<template>
  <div class="l-autobuyers-tab">
    <AutobuyerToggles />
    <div>
      Challenges for upgrading autobuyers are unlocked by reaching Infinity.
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
  </div>
</template>
