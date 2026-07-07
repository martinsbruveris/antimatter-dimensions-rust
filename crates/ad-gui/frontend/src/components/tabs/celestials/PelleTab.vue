<script setup>
// Pelle subtab (Feature 7.7). Pre-doom: the Doom button. Doomed: the Remnants /
// Reality Shards header, the Armageddon button, the 5 rift bars (fill % + toggle
// + milestone dots), the Pelle Upgrade grid, and the Galaxy Generator readout.
// Reads `snapshot.celestials.pelle`. The zalgo/credits end sequence is cut; the
// game-end shows a plain progress bar.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";
import {
  PELLE_RIFTS,
  PELLE_REBUYABLE_DESCRIPTIONS,
  PELLE_UPGRADE_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const p = computed(() => game.snapshot?.celestials?.pelle);

function fmt(x, places = 2) {
  return formatDecimal(x, places, places);
}
function n(x) {
  return fmt({ m: x, e: 0 });
}
</script>

<template>
  <div v-if="p" class="l-pelle-tab">
    <div v-if="p.is_game_end" class="c-pelle-end">The End.</div>

    <!-- Pre-doom -->
    <template v-else-if="!p.doomed">
      <div class="c-pelle-intro">
        Pelle, Celestial of Antimatter. Dooming your Reality is permanent and
        disables most mechanics — but opens the final path.
      </div>
      <div class="l-pelle-center">
        <button class="c-pelle-doom" @click="game.doomReality()">Doom your Reality</button>
      </div>
    </template>

    <!-- Doomed -->
    <template v-else>
      <div class="c-pelle-header">
        <div>Remnants: {{ n(p.remnants) }}</div>
        <div>Reality Shards: {{ fmt(p.reality_shards) }} (+{{ fmt(p.reality_shard_per_second) }}/s)</div>
      </div>
      <div class="l-pelle-center">
        <button
          class="c-pelle-doom"
          :disabled="!p.can_armageddon"
          @click="game.pelleArmageddon()"
        >
          Armageddon (+{{ n(p.remnants_gain) }} Remnants)
        </button>
      </div>

      <!-- Game-end progress -->
      <div class="c-pelle-endbar">
        <div class="c-pelle-endbar-inner" :style="{ width: `${p.game_end_progress * 100}%` }" />
      </div>

      <!-- Rifts -->
      <div class="l-pelle-section">Rifts</div>
      <div class="l-rift-grid">
        <div
          v-for="rift in p.rifts"
          :key="rift.id"
          class="c-rift"
          :class="{ 'c-rift--locked': !rift.unlocked, 'c-rift--active': rift.active }"
        >
          <div class="c-rift-name">
            {{ PELLE_RIFTS[rift.id].name }}
            <span class="c-rift-effect">{{ PELLE_RIFTS[rift.id].effect }}</span>
          </div>
          <div class="c-rift-bar">
            <div class="c-rift-bar-inner" :style="{ width: `${rift.percentage * 100}%` }" />
          </div>
          <div class="c-rift-milestones">
            <span
              v-for="(mst, i) in rift.milestones"
              :key="i"
              class="c-rift-dot"
              :class="{ 'c-rift-dot--on': mst }"
            />
          </div>
          <button
            v-if="rift.unlocked"
            class="c-pelle-mini"
            @click="game.pelleToggleRift(rift.id)"
          >
            {{ rift.active ? "Draining" : "Drain" }}
          </button>
          <div v-else class="c-rift-locked-text">Locked — reach the Strike</div>
        </div>
      </div>

      <!-- Galaxy Generator -->
      <div v-if="p.galaxy_generator_unlocked" class="c-pelle-gg">
        Galaxy Generator: {{ n(p.galaxy_generator_galaxies) }} galaxies
        <button class="c-pelle-mini" @click="game.pelleStartSacrifice()">Sacrifice rift</button>
      </div>

      <!-- Upgrades -->
      <div class="l-pelle-section">Reality Shard Upgrades</div>
      <div class="l-pelle-upg-grid">
        <button
          v-for="u in p.rebuyables"
          :key="`r${u.id}`"
          class="c-pelle-upg c-pelle-upg--rebuyable"
          :disabled="u.count >= u.cap"
          :ach-tooltip="PELLE_REBUYABLE_DESCRIPTIONS[u.id]"
          @click="game.buyPelleRebuyable(u.id)"
        >
          <div>{{ PELLE_REBUYABLE_DESCRIPTIONS[u.id] }} ({{ u.count }}/{{ u.cap }})</div>
          <div class="c-pelle-cost">{{ u.count >= u.cap ? "Capped" : `${fmt(u.cost)} RS` }}</div>
        </button>
        <button
          v-for="u in p.upgrades"
          :key="`u${u.id}`"
          class="c-pelle-upg"
          :class="{ 'c-pelle-upg--bought': u.bought }"
          :disabled="u.bought"
          :ach-tooltip="PELLE_UPGRADE_DESCRIPTIONS[u.id]"
          @click="game.buyPelleUpgrade(u.id)"
        >
          <div>{{ PELLE_UPGRADE_DESCRIPTIONS[u.id] }}</div>
          <div class="c-pelle-cost">{{ u.bought ? "Bought" : `${fmt(u.cost)} RS` }}</div>
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.l-pelle-tab {
  padding: 1rem 0 3rem;
  color: #d0c0e0;
}
.c-pelle-end {
  text-align: center;
  font-size: 4rem;
  margin: 6rem 0;
  color: #b39ddb;
}
.c-pelle-intro,
.c-pelle-header {
  max-width: 55rem;
  margin: 1rem auto;
  text-align: center;
  font-size: 1.3rem;
}
.c-pelle-header {
  display: flex;
  gap: 2rem;
  justify-content: center;
}
.l-pelle-center {
  display: flex;
  justify-content: center;
  margin: 1rem 0;
}
.c-pelle-doom {
  color: #b39ddb;
  background: #10001b;
  border: 0.2rem solid #b39ddb;
  border-radius: 0.8rem;
  padding: 0.8rem 2rem;
  font-size: 1.3rem;
  cursor: pointer;
}
.c-pelle-doom:disabled {
  opacity: 0.45;
  cursor: default;
}
.c-pelle-endbar {
  width: 40rem;
  max-width: 90%;
  height: 0.6rem;
  margin: 0.5rem auto 1.5rem;
  background: #1a0030;
  border-radius: 0.3rem;
  overflow: hidden;
}
.c-pelle-endbar-inner {
  height: 100%;
  background: #b39ddb;
}
.l-pelle-section {
  text-align: center;
  font-size: 1.4rem;
  font-weight: bold;
  margin: 1.5rem 0 0.6rem;
}
.l-rift-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.6rem;
}
.c-rift {
  width: 16rem;
  background: linear-gradient(#150025, #24003b);
  border: 0.1rem solid #4a2a7a;
  border-radius: 0.6rem;
  padding: 0.6rem;
  text-align: center;
}
.c-rift--locked {
  opacity: 0.4;
}
.c-rift--active {
  border-color: #b39ddb;
  box-shadow: 0 0 0.5rem #b39ddb;
}
.c-rift-name {
  font-weight: bold;
}
.c-rift-effect {
  display: block;
  font-size: 0.85rem;
  color: #9a8;
}
.c-rift-bar {
  height: 0.8rem;
  margin: 0.4rem 0;
  background: #10001b;
  border-radius: 0.3rem;
  overflow: hidden;
}
.c-rift-bar-inner {
  height: 100%;
  background: #b39ddb;
}
.c-rift-milestones {
  display: flex;
  justify-content: center;
  gap: 0.4rem;
  margin-bottom: 0.4rem;
}
.c-rift-dot {
  width: 0.7rem;
  height: 0.7rem;
  border-radius: 50%;
  background: #333;
}
.c-rift-dot--on {
  background: #b39ddb;
}
.c-rift-locked-text {
  font-size: 0.85rem;
  color: #887;
}
.c-pelle-gg {
  text-align: center;
  font-size: 1.2rem;
  margin: 1rem 0;
}
.c-pelle-mini {
  color: white;
  background: #333;
  border: 0.1rem solid #111;
  border-radius: 0.4rem;
  padding: 0.25rem 0.6rem;
  font-size: 0.9rem;
  cursor: pointer;
}
.l-pelle-upg-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.5rem;
}
.c-pelle-upg {
  width: 15rem;
  color: #d0c0e0;
  background: #14002b;
  border: 0.1rem solid #4a2a7a;
  border-radius: 0.6rem;
  padding: 0.5rem;
  font-size: 0.9rem;
  cursor: pointer;
}
.c-pelle-upg--rebuyable {
  border-color: #7a2a5a;
}
.c-pelle-upg--bought {
  background: #2a0040;
  opacity: 0.8;
}
.c-pelle-cost {
  font-size: 0.8rem;
  color: #99a;
}
</style>
