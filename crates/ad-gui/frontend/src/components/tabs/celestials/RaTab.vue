<script setup>
// Ra subtab (Feature 7.5). The four Celestial-Memory pet panels (level, memory
// / chunk readouts, the two upgrade buttons, milestone icons, Remembrance
// toggle), the run button, the charged-Infinity-Upgrade counter, and the Glyph
// Alchemy resource grid. Reads `snapshot.celestials.ra`. CSS vendored from the
// original `celestial-ra` styles; the animated navigation hub is cut.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";
import {
  RA_PETS,
  RA_UNLOCK_DESCRIPTIONS,
  ALCHEMY_RESOURCES,
} from "../../../data/celestials";

const game = useGameStore();
const canCreateRealityGlyph = computed(
  () => game.snapshot?.reality?.can_create_reality_glyph ?? false,
);
const realityGlyphLevel = computed(
  () => game.snapshot?.reality?.reality_glyph_level ?? 0,
);
const ra = computed(() => game.snapshot?.celestials?.ra);

const isRunning = computed(() => Boolean(ra.value?.is_running));

// Memory / chunk / resource counts ship as { m, e } Num objects.
function fmt(x, places = 2) {
  return formatDecimal(x, places, places);
}
// log10 of a { m, e } Num (robust across huge exponents).
function numLog(x) {
  return Math.log10(Math.max(x?.m ?? 0, 1e-300)) + (x?.e ?? 0);
}
function numLt(a, b) {
  return numLog(a) < numLog(b);
}

function petMeta(id) {
  return RA_PETS[id] ?? {};
}
function unlocksForPet(petId) {
  return (ra.value?.unlocks ?? []).filter((u) => u.pet === petId);
}
function memoryFraction(pet) {
  // required_memories is clamped to 1e300 at the cap; treat that as "full".
  if (!pet.required_memories || pet.required_memories.e >= 300) return 0;
  return Math.min(1, 10 ** (numLog(pet.memories) - numLog(pet.required_memories)));
}
function canLevel(pet) {
  return pet.level < 25 && !numLt(pet.memories, pet.required_memories);
}
function canMemory(pet) {
  return !pet.memory_upgrade_capped && !numLt(pet.memories, pet.memory_upgrade_cost);
}
function canChunk(pet) {
  return !pet.chunk_upgrade_capped && !numLt(pet.memories, pet.chunk_upgrade_cost);
}

function startRun() {
  game.startCelestialReality("ra");
}
function levelUp(id) {
  game.raLevelUp(id);
}
function buyMemory(id) {
  game.raBuyMemoryUpgrade(id);
}
function buyChunk(id) {
  game.raBuyChunkUpgrade(id);
}
function setRemembrance(id) {
  game.raSetRemembrance(id);
}
function toggleReaction(id) {
  game.alchemyToggleReaction(id);
}
</script>

<template>
  <div v-if="ra" class="l-ra-celestial-tab">
    <div class="c-ra-info">
      Ra, Celestial of the Memories. Enter Ra's Reality and gain Memories from
      real time; level up the Celestial Memories to unlock rewards.
      <span v-if="ra.total_charges > 0">
        Charged Infinity Upgrades: {{ ra.charges_used }} / {{ ra.total_charges }}
        (charge bought upgrades on the Infinity Upgrades tab).
        <button class="c-ra-discharge-btn" @click="game.toggleDischarge()">
          {{ ra.discharge_armed
            ? "Will discharge on Reality" : "Discharge on next Reality" }}
        </button>
      </span>
    </div>

    <div class="l-ra-run-row">
      <button
        class="c-ra-run-button"
        :class="{ 'c-ra-run-button--active': isRunning }"
        :disabled="!ra.can_start_run && !isRunning"
        @click="startRun"
      >
        <div class="c-ra-run-button__icon"><i class="fas fa-sun" /></div>
        <div>{{ isRunning ? "Ra's Reality is active" : "Enter Ra's Reality" }}</div>
        <div class="c-ra-run-button__desc">
          Total Memory levels: {{ ra.total_pet_level }}
        </div>
      </button>
    </div>

    <!-- Pet panels -->
    <div class="l-ra-pets">
      <div
        v-for="pet in ra.pets"
        :key="pet.id"
        v-show="pet.unlocked"
        class="c-ra-pet-header"
        :style="{ color: petMeta(pet.id).color }"
      >
        <div class="c-ra-pet-title">
          {{ petMeta(pet.id).name }} Level {{ pet.level }}/25
        </div>
        <div class="c-ra-pet-memories">
          {{ fmt(pet.memories) }} Memories
          <span v-if="pet.level < 25">
            (need {{ fmt(pet.required_memories) }} to level)
          </span>
        </div>
        <div class="c-ra-exp-bar" v-if="pet.level < 25">
          <div
            class="c-ra-exp-bar-inner"
            :style="{ width: `${100 * memoryFraction(pet)}%`,
                      background: petMeta(pet.id).color }"
          />
        </div>
        <div class="l-ra-pet-actions">
          <button
            class="c-ra-pet-btn"
            :disabled="!canLevel(pet)"
            @click="levelUp(pet.id)"
          >
            Level Up
          </button>
          <button
            class="c-ra-pet-btn"
            :disabled="!canMemory(pet)"
            :ach-tooltip="`+30% Memories · cost ${fmt(pet.memory_upgrade_cost)}`"
            @click="buyMemory(pet.id)"
          >
            <span class="fas fa-brain" /> Recollection
          </button>
          <button
            class="c-ra-pet-btn"
            :disabled="!canChunk(pet)"
            :ach-tooltip="`+50% Memory Chunks · cost ${fmt(pet.chunk_upgrade_cost)}`"
            @click="buyChunk(pet.id)"
          >
            <span class="fas fa-dice-d6" /> Fragmentation
          </button>
          <button
            v-if="ra.remembrance_unlocked"
            class="c-ra-pet-btn c-ra-pet-btn--remembrance"
            :class="{ 'c-ra-pet-btn--active': pet.has_remembrance }"
            @click="setRemembrance(pet.id)"
          >
            Remembrance
          </button>
        </div>
        <div class="c-ra-chunks">{{ fmt(pet.memory_chunks) }} Memory Chunks</div>
        <!-- Milestone icons -->
        <div class="l-ra-pet-milestones">
          <div
            v-for="u in unlocksForPet(pet.id)"
            :key="u.id"
            class="c-ra-upgrade-icon"
            :class="{ 'c-ra-upgrade-icon--inactive': !u.unlocked }"
            :ach-tooltip="`Lvl ${u.level}: ${RA_UNLOCK_DESCRIPTIONS[u.id]}`"
          >
            <i class="fas fa-star" v-if="u.unlocked" />
            <i class="fas fa-lock" v-else />
          </div>
        </div>
      </div>
    </div>

    <!-- Glyph Alchemy -->
    <div v-if="ra.alchemy_unlocked" class="l-ra-alchemy">
      <div class="c-ra-info">Glyph Alchemy — refine Glyphs to fill resources.</div>
      <div class="l-alchemy-grid">
        <div
          v-for="res in ra.alchemy"
          :key="res.id"
          class="c-alchemy-resource"
          :class="{ 'c-alchemy-resource--locked': !res.unlocked }"
          :ach-tooltip="ALCHEMY_RESOURCES[res.id].effect"
        >
          <div class="c-alchemy-symbol">{{ ALCHEMY_RESOURCES[res.id].symbol }}</div>
          <div class="c-alchemy-name">{{ ALCHEMY_RESOURCES[res.id].name }}</div>
          <div v-if="res.unlocked" class="c-alchemy-amount">
            {{ fmt(res.amount, 1) }} / {{ fmt(res.cap, 1) }}
          </div>
          <div v-else class="c-alchemy-lock">Effarig Lvl {{ res.unlocked_at }}</div>
          <button
            v-if="res.unlocked && !res.is_base"
            class="c-alchemy-reaction"
            :class="{ 'c-alchemy-reaction--on': res.reaction_active }"
            @click="toggleReaction(res.id)"
          >
            {{ res.reaction_active ? "Reaction ON" : "Reaction OFF" }}
          </button>
          <button
            v-if="res.id === 20 && res.unlocked"
            class="c-alchemy-reaction"
            :disabled="!canCreateRealityGlyph"
            :title="canCreateRealityGlyph
              ? `Create a level ${realityGlyphLevel} Reality Glyph (consumes all
                Reality resource)`
              : 'Requires Reality resource and inventory space'"
            @click="game.createRealityGlyph()"
          >
            Create Reality Glyph (lvl {{ realityGlyphLevel }})
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.l-ra-celestial-tab {
  padding: 1rem 0 3rem;
}

.c-ra-info {
  max-width: 60rem;
  margin: 1rem auto;
  text-align: center;
  font-size: 1.3rem;
}

.l-ra-run-row {
  display: flex;
  justify-content: center;
}

/* Vendored from `.c-ra-run-button` / `__icon`. */
.c-ra-run-button {
  display: flex;
  flex-direction: column;
  width: 26rem;
  justify-content: center;
  align-items: center;
  color: #9575cd;
  background: black;
  border: 0.2rem solid #9575cd;
  border-radius: 1.5rem;
  margin: 1rem;
  padding: 1.5rem;
  cursor: pointer;
}

.c-ra-run-button:disabled {
  opacity: 0.5;
  cursor: default;
}

.c-ra-run-button--active {
  box-shadow: 0 0 1rem #9575cd;
}

.c-ra-run-button__icon {
  display: flex;
  width: 7rem;
  height: 7rem;
  justify-content: center;
  align-items: center;
  font-size: 3rem;
  color: #9575cd;
  background-color: black;
  border: 0.4rem solid #9575cd;
  border-radius: 50%;
  box-shadow: 0 0 0.7rem #9575cd, 0 0 0.7rem #9575cd inset;
  margin: 1rem 0;
}

.c-ra-run-button__desc {
  margin-top: 0.5rem;
  font-size: 1.1rem;
}

.l-ra-pets {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
}

/* Vendored from `.c-ra-pet-header`. */
.c-ra-pet-header {
  width: 32rem;
  background: linear-gradient(#2f2f2f, #464646);
  border-radius: 1.5rem;
  margin: 1rem;
  padding: 1rem;
  text-align: center;
}

.c-ra-pet-title {
  font-size: 2rem;
  font-weight: bold;
}

.c-ra-pet-memories {
  margin: 0.5rem 0;
  font-size: 1.1rem;
  color: #dddddd;
}

/* Vendored from `.c-ra-exp-bar`. */
.c-ra-exp-bar {
  width: 100%;
  height: 1.6rem;
  border: 0.2rem solid black;
  border-radius: 0.4rem;
  overflow: hidden;
  margin-bottom: 0.6rem;
}

.c-ra-exp-bar-inner {
  height: 100%;
}

.l-ra-pet-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.4rem;
  margin: 0.6rem 0;
}

.c-ra-pet-btn {
  color: white;
  background: #222222;
  border: 0.1rem solid #111111;
  border-radius: 0.5rem;
  padding: 0.4rem 0.8rem;
  font-size: 1.05rem;
  cursor: pointer;
}

.c-ra-pet-btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.c-ra-pet-btn--active {
  background: #9575cd;
  color: black;
}

.c-ra-chunks {
  font-size: 1rem;
  color: #cccccc;
}

.l-ra-pet-milestones {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  margin-top: 0.6rem;
}

/* Vendored from `.c-ra-upgrade-icon`. */
.c-ra-upgrade-icon {
  display: flex;
  width: 3rem;
  height: 3rem;
  justify-content: center;
  align-items: center;
  font-size: 1.4rem;
  color: #ffd700;
  background: #222222;
  border: 0.1rem solid #111111;
  border-radius: 50%;
  box-shadow: 0.1rem 0.1rem 0.1rem rgba(0, 0, 0, 70%);
  margin: 0.2rem;
}

.c-ra-upgrade-icon--inactive {
  color: #555555;
}

.l-ra-alchemy {
  margin-top: 2rem;
}

.l-alchemy-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.6rem;
}

.c-alchemy-resource {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 12rem;
  background: linear-gradient(#2f2f2f, #464646);
  border: 0.1rem solid black;
  border-radius: 0.8rem;
  padding: 0.6rem;
}

.c-alchemy-resource--locked {
  opacity: 0.45;
}

.c-alchemy-symbol {
  font-size: 2rem;
  font-weight: bold;
}

.c-alchemy-name {
  font-size: 1.1rem;
  font-weight: bold;
}

.c-alchemy-amount {
  font-size: 1rem;
  color: #cccccc;
}

.c-alchemy-lock {
  font-size: 0.95rem;
  color: #999999;
}

.c-alchemy-reaction {
  margin-top: 0.4rem;
  color: white;
  background: #333333;
  border: 0.1rem solid #111111;
  border-radius: 0.4rem;
  padding: 0.2rem 0.5rem;
  font-size: 0.9rem;
  cursor: pointer;
}

.c-alchemy-reaction--on {
  background: #5cb85c;
  color: black;
}
.c-ra-discharge-btn {
  margin-left: 0.5rem;
  background: transparent;
  color: inherit;
  border: 0.1rem solid currentcolor;
  border-radius: 0.3rem;
  cursor: pointer;
}
</style>
