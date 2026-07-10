<script setup>
// Lai'tela subtab (Feature 7.6). The Dark Matter / Dark Energy / Singularity
// header, the 4 Dark Matter Dimension panels, the Continuum toggle, the
// annihilation + run controls, the Imaginary Upgrade grid, and the Singularity
// Milestone list. Reads `snapshot.celestials.laitela`. CSS is themed to match
// the other celestial tabs (the milestone auto-sort UI is simplified).
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { formatDecimal } from "../../../util/format";
import {
  MILESTONE_DESCRIPTIONS,
  IMAGINARY_UPGRADE_DESCRIPTIONS,
  IMAGINARY_REBUYABLE_DESCRIPTIONS,
} from "../../../data/celestials";

const game = useGameStore();
const l = computed(() => game.snapshot?.celestials?.laitela);

function fmt(x, places = 2) {
  return formatDecimal(x, places, places);
}
function ms(x) {
  return `${(x / 1000).toFixed(3)}s`;
}
const dimName = ["1st", "2nd", "3rd", "4th"];

function startRun() {
  game.startCelestialReality("laitela");
}
</script>

<template>
  <div v-if="l" class="l-laitela-tab">
    <div class="c-laitela-header">
      <div>Dark Matter: {{ fmt(l.dark_matter) }} (peak {{ fmt(l.max_dark_matter) }})</div>
      <div>Dark Energy: {{ fmt(l.dark_energy) }} / {{ fmt(l.singularity_cap) }}</div>
      <div>Singularities: {{ fmt(l.singularities) }}</div>
      <div>Dark Matter Multiplier: {{ fmt({ m: l.dark_matter_mult, e: 0 }) }}</div>
    </div>

    <div class="l-laitela-controls">
      <button class="c-laitela-btn" :disabled="!l.can_condense" @click="game.laitelaCondense()">
        Condense {{ fmt(l.singularities_gained) }} Singularities
      </button>
      <div class="c-laitela-cap">
        Cap increases: {{ l.singularity_cap_increases }}
        <button class="c-laitela-mini" @click="game.laitelaChangeSingularityCap(false)">−</button>
        <button class="c-laitela-mini" @click="game.laitelaChangeSingularityCap(true)">+</button>
      </div>
      <button
        v-if="l.annihilation_unlocked"
        class="c-laitela-btn"
        :disabled="!l.can_annihilate"
        @click="game.laitelaAnnihilate()"
      >
        Annihilate (+{{ fmt({ m: l.dark_matter_mult_gain, e: 0 }) }} mult)
      </button>
      <button
        v-if="l.continuum_unlocked"
        class="c-laitela-btn"
        :class="{ 'c-laitela-btn--on': l.continuum_active }"
        @click="game.laitelaSetContinuum(!l.continuum_active)"
      >
        Continuum: {{ l.continuum_active ? "ON" : "OFF" }}
      </button>
      <button class="c-laitela-btn" @click="game.laitelaMaxAllDmd()">Max all DMD</button>
    </div>

    <!-- Singularity-milestone autobuyers -->
    <div
      v-if="l.auto_dmd_unlocked || l.auto_ascension_unlocked ||
        l.auto_annihilation_unlocked || l.auto_singularity_unlocked"
      class="l-laitela-controls"
    >
      <button
        v-if="l.auto_dmd_unlocked"
        class="c-laitela-btn"
        :class="{ 'c-laitela-btn--on': l.auto_dmd_active }"
        @click="game.toggleDmdAutobuyer()"
      >
        Auto DMD: {{ l.auto_dmd_active ? "ON" : "OFF" }}
      </button>
      <button
        v-if="l.auto_ascension_unlocked"
        class="c-laitela-btn"
        :class="{ 'c-laitela-btn--on': l.auto_ascension_active }"
        @click="game.toggleAscensionAutobuyer()"
      >
        Auto Ascend: {{ l.auto_ascension_active ? "ON" : "OFF" }}
      </button>
      <button
        v-if="l.auto_annihilation_unlocked"
        class="c-laitela-btn"
        :class="{ 'c-laitela-btn--on': l.auto_annihilation_active }"
        @click="game.toggleAnnihilationAutobuyer()"
      >
        Auto Annihilate: {{ l.auto_annihilation_active ? "ON" : "OFF" }}
      </button>
      <label v-if="l.auto_annihilation_unlocked" class="c-laitela-cap">
        at mult ≥
        <input
          class="c-laitela-input"
          type="number"
          step="0.01"
          min="0.01"
          :value="l.annihilation_multiplier"
          @change="game.setAnnihilationMultiplier(Number($event.target.value))"
        >
      </label>
      <button
        v-if="l.auto_singularity_unlocked"
        class="c-laitela-btn"
        :class="{ 'c-laitela-btn--on': l.auto_singularity_active }"
        @click="game.toggleSingularityAutobuyer()"
      >
        Auto Condense: {{ l.auto_singularity_active ? "ON" : "OFF" }}
      </button>
    </div>

    <!-- Run -->
    <div class="l-laitela-run">
      <button
        class="c-laitela-run-btn"
        :class="{ 'c-laitela-run-btn--active': l.is_running }"
        :disabled="!l.can_start_run && !l.is_running"
        @click="startRun"
      >
        {{ l.is_running ? "In Lai'tela's Reality" : "Enter Lai'tela's Reality" }}
      </button>
      <div class="c-laitela-run-info">
        Difficulty tier {{ l.difficulty_tier }} · dimensions ≤ {{ l.max_allowed_dimension }}
        · reward ×{{ fmt({ m: l.reality_reward, e: 0 }) }}
        <span v-if="l.is_running"> · entropy {{ (l.entropy * 100).toFixed(1) }}%</span>
      </div>
    </div>

    <!-- Dark Matter Dimensions -->
    <div class="l-dmd-grid">
      <div
        v-for="d in l.dimensions"
        :key="d.tier"
        v-show="d.unlocked"
        class="c-dmd"
      >
        <div class="c-dmd-title">{{ dimName[d.tier] }} Dark Matter Dimension</div>
        <div>Amount: {{ fmt(d.amount) }} · every {{ ms(d.interval_ms) }}</div>
        <div>DM ×{{ fmt(d.power_dm) }} · DE +{{ fmt(d.power_de) }}</div>
        <div class="c-dmd-buttons">
          <button class="c-laitela-mini" @click="game.dmdBuyUpgrade(d.tier, 0)">
            Interval ({{ fmt(d.interval_cost) }})
          </button>
          <button class="c-laitela-mini" @click="game.dmdBuyUpgrade(d.tier, 1)">
            Power DM ({{ fmt(d.power_dm_cost) }})
          </button>
          <button class="c-laitela-mini" @click="game.dmdBuyUpgrade(d.tier, 2)">
            Power DE ({{ fmt(d.power_de_cost) }})
          </button>
          <button
            class="c-laitela-mini"
            :disabled="!d.can_ascend"
            @click="game.dmdAscend(d.tier)"
          >
            Ascend ({{ d.ascensions }})
          </button>
        </div>
      </div>
    </div>

    <!-- Imaginary Upgrades -->
    <div class="l-laitela-section-title">
      Imaginary Machines: {{ fmt(l.imaginary_machines) }} / {{ fmt(l.im_cap) }}
    </div>
    <div class="l-im-grid">
      <button
        v-for="u in l.imaginary_rebuyables"
        :key="`r${u.id}`"
        class="c-im-upgrade c-im-upgrade--rebuyable"
        :ach-tooltip="IMAGINARY_REBUYABLE_DESCRIPTIONS[u.id]"
        @click="game.buyImaginaryRebuyable(u.id)"
      >
        <div>{{ IMAGINARY_REBUYABLE_DESCRIPTIONS[u.id] }} ({{ u.count }})</div>
        <div class="c-im-cost">{{ fmt(u.cost) }} iM</div>
      </button>
      <button
        v-for="u in l.imaginary_upgrades"
        :key="`u${u.id}`"
        class="c-im-upgrade"
        :class="{ 'c-im-upgrade--bought': u.bought, 'c-im-upgrade--available': u.available }"
        :disabled="u.bought || !u.available"
        :ach-tooltip="IMAGINARY_UPGRADE_DESCRIPTIONS[u.id]"
        @click="game.buyImaginaryUpgrade(u.id)"
      >
        <div>{{ IMAGINARY_UPGRADE_DESCRIPTIONS[u.id] }}</div>
        <div class="c-im-cost">{{ u.bought ? "Bought" : `${fmt(u.cost)} iM` }}</div>
      </button>
    </div>

    <!-- Singularity Milestones -->
    <div class="l-laitela-section-title">Singularity Milestones</div>
    <div class="l-milestone-grid">
      <div
        v-for="mst in l.milestones"
        :key="mst.id"
        class="c-milestone"
        :class="{ 'c-milestone--locked': !mst.unlocked }"
        :ach-tooltip="`${MILESTONE_DESCRIPTIONS[mst.id]} (from ${fmt({ m: mst.start, e: 0 })} Singularities)`"
      >
        <div class="c-milestone-desc">{{ MILESTONE_DESCRIPTIONS[mst.id] }}</div>
        <div class="c-milestone-comp">
          {{ mst.unlocked ? `×${mst.completions}` : `${fmt({ m: mst.start, e: 0 })} Ω` }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.l-laitela-tab {
  padding: 1rem 0 3rem;
  color: #b3ccff;
}

.c-laitela-header {
  display: flex;
  flex-wrap: wrap;
  gap: 1.5rem;
  justify-content: center;
  font-size: 1.3rem;
  margin: 1rem 0;
}

.l-laitela-controls,
.l-laitela-run {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  justify-content: center;
  align-items: center;
  margin: 0.8rem 0;
}

.c-laitela-btn {
  color: white;
  background: #16003b;
  border: 0.1rem solid #9575cd;
  border-radius: 0.5rem;
  padding: 0.5rem 1rem;
  font-size: 1.05rem;
  cursor: pointer;
}
.c-laitela-btn:disabled {
  opacity: 0.45;
  cursor: default;
}
.c-laitela-btn--on {
  background: #9575cd;
  color: black;
}

.c-laitela-cap {
  font-size: 1.05rem;
}
.c-laitela-mini {
  color: white;
  background: #222222;
  border: 0.1rem solid #111111;
  border-radius: 0.4rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.95rem;
  cursor: pointer;
}
.c-laitela-mini:disabled {
  opacity: 0.4;
  cursor: default;
}

.c-laitela-run-btn {
  color: #b3ccff;
  background: black;
  border: 0.2rem solid #b3ccff;
  border-radius: 1rem;
  padding: 1rem 2rem;
  font-size: 1.3rem;
  cursor: pointer;
}
.c-laitela-run-btn--active {
  box-shadow: 0 0 1rem #b3ccff;
}
.c-laitela-run-info {
  width: 100%;
  text-align: center;
  font-size: 1.1rem;
}

.l-dmd-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.8rem;
  margin: 1rem 0;
}
.c-dmd {
  width: 26rem;
  background: linear-gradient(#101024, #1c1c3a);
  border: 0.1rem solid #3a3a6a;
  border-radius: 0.8rem;
  padding: 0.8rem;
  text-align: center;
  color: #ccd;
}
.c-dmd-title {
  font-size: 1.2rem;
  font-weight: bold;
  margin-bottom: 0.4rem;
}
.c-dmd-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
  justify-content: center;
  margin-top: 0.5rem;
}

.l-laitela-section-title {
  text-align: center;
  font-size: 1.4rem;
  font-weight: bold;
  margin: 1.5rem 0 0.6rem;
}

.l-im-grid,
.l-milestone-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.5rem;
}
.c-im-upgrade {
  width: 14rem;
  color: #ccd;
  background: #14002b;
  border: 0.1rem solid #4a2a7a;
  border-radius: 0.6rem;
  padding: 0.5rem;
  font-size: 0.95rem;
  cursor: pointer;
}
.c-im-upgrade--rebuyable {
  border-color: #2a7a4a;
}
.c-im-upgrade--available {
  border-color: #b3ccff;
}
.c-im-upgrade--bought {
  background: #003b1a;
  opacity: 0.8;
}
.c-im-cost {
  font-size: 0.85rem;
  color: #99a;
}

.c-milestone {
  width: 15rem;
  color: #ccd;
  background: linear-gradient(#101024, #1c1c3a);
  border: 0.1rem solid #3a3a6a;
  border-radius: 0.5rem;
  padding: 0.4rem;
  font-size: 0.9rem;
  text-align: center;
}
.c-milestone--locked {
  opacity: 0.4;
}
.c-milestone-comp {
  font-weight: bold;
}
.c-laitela-input {
  width: 5rem;
  background: transparent;
  color: inherit;
  border: 0.1rem solid currentcolor;
  border-radius: 0.3rem;
  padding: 0.1rem 0.3rem;
}
</style>
