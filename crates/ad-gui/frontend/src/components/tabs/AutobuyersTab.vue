<script setup>
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import OpenHotkeysButton from "../options/OpenHotkeysButton.vue";
import AutobuyerToggles from "./autobuyers/AutobuyerToggles.vue";
import AutobuyerToggleFooter from "./autobuyers/AutobuyerToggleFooter.vue";
import BigCrunchAutobuyerBox from "./autobuyers/BigCrunchAutobuyerBox.vue";
import DimensionAutobuyerBox from "./autobuyers/DimensionAutobuyerBox.vue";
import EternityAutobuyerBox from "./autobuyers/EternityAutobuyerBox.vue";
import MilestoneAutobuyerGroupBox from "./autobuyers/MilestoneAutobuyerGroupBox.vue";
import PrestigeAutobuyerBox from "./autobuyers/PrestigeAutobuyerBox.vue";
import RealityAutobuyerBox from "./autobuyers/RealityAutobuyerBox.vue";
import TickspeedAutobuyerBox from "./autobuyers/TickspeedAutobuyerBox.vue";

// Autobuyers tab. The antimatter dimension and tickspeed autobuyers unlock with
// antimatter; the Dim Boost / Galaxy / Big Crunch autobuyers unlock by
// completing Normal Challenges 10/11/12 (interval upgrades with Infinity
// Points once each challenge is completed). The Eternity / Reality autobuyers
// (100-Eternities milestone / Reality Upgrade 25) appear once unlocked, in the
// original's prestige order: Crunch, Eternity, Reality.
const game = useGameStore();
const auto = computed(() => game.snapshot.autobuyers);
</script>

<template>
  <div class="l-autobuyers-tab">
    <AutobuyerToggles />
    <OpenHotkeysButton />
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
    <BigCrunchAutobuyerBox />
    <EternityAutobuyerBox />
    <RealityAutobuyerBox />
    <MilestoneAutobuyerGroupBox
      :group="auto.infinity_dims"
      kind="infinityDims"
      name="Infinity Dimension"
    />
    <MilestoneAutobuyerGroupBox
      :group="auto.replicanti_upgrades"
      kind="replicantiUpgrades"
      name="Replicanti Upgrade"
    />
    <span
      v-if="auto.replicanti_galaxy_unlocked"
      class="c-autobuyer-box-row"
    >
      <AutobuyerToggleFooter
        :is-active="auto.replicanti_galaxy_active"
        @toggle="game.toggleMilestoneAutobuyer('replicantiGalaxy')"
      />
      <div class="l-autobuyer-box__title">
        Replicanti Galaxy<br>Autobuyer
      </div>
    </span>
  </div>
</template>
