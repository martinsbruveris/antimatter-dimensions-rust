<script setup>
// The Infinity → Replicanti subtab: unlock button (pre-unlock), then the amount +
// its Infinity-Dimension multiplier, the three IP upgrades (chance / interval / max
// galaxies), and the Replicanti Galaxy button. State comes from the engine snapshot
// (`game.replicanti`). Pre-Eternity scope: no auto-galaxy toggle, no cap-raising
// rewards, no gain estimate.
import { computed } from "vue";

import { useGameStore } from "../../stores/game";
import { formatDecimal, formatMultiplier } from "../../util/format";

const game = useGameStore();
const r = computed(() => game.snapshot?.replicanti);

// Chance is stored as a fraction; Replicanti chance is always a whole percent.
const chancePercent = computed(() => Math.round((r.value?.chance ?? 0) * 100));

// Mirror the original's interval formatting: seconds above 1000 ms, else ms.
function formatInterval(ms) {
  if (ms > 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(2)}ms`;
}
const intervalText = computed(() => formatInterval(r.value?.interval_ms ?? 0));
const nextIntervalText = computed(() =>
  formatInterval(Math.max((r.value?.interval_ms ?? 0) * 0.9, 50)),
);
</script>

<template>
  <div
    v-if="r"
    class="l-replicanti-tab"
  >
    <!-- Locked: the unlock button. -->
    <button
      v-if="!r.unlocked"
      class="o-primary-btn o-primary-btn--replicanti-unlock"
      :class="{ 'o-primary-btn--disabled': !r.can_unlock }"
      @click="r.can_unlock && game.unlockReplicanti()"
    >
      Unlock Replicanti
      <br>
      Cost: {{ formatDecimal(r.unlock_cost) }} IP
    </button>

    <template v-else>
      <p class="c-replicanti-description">
        You have
        <span class="c-replicanti-description__accent">{{
          formatDecimal(r.amount, 2)
        }}</span>
        Replicanti, translated to a
        <span class="c-replicanti-description__accent">{{
          formatMultiplier(r.mult)
        }}</span>
        multiplier on all Infinity Dimensions.
      </p>

      <div class="l-replicanti-upgrade-row">
        <!-- Chance -->
        <button
          class="o-primary-btn c-replicanti-upgrade"
          :class="{ 'o-primary-btn--disabled': !r.can_buy_chance }"
          @click="r.can_buy_chance && game.buyReplicantiChance()"
        >
          Replicate chance: {{ chancePercent }}%
          <br>
          <template v-if="r.chance_capped">Capped</template>
          <template v-else>+1% Costs: {{ formatDecimal(r.chance_cost) }} IP</template>
        </button>

        <!-- Interval -->
        <button
          class="o-primary-btn c-replicanti-upgrade"
          :class="{ 'o-primary-btn--disabled': !r.can_buy_interval }"
          @click="r.can_buy_interval && game.buyReplicantiInterval()"
        >
          Interval: {{ intervalText }}
          <br>
          <template v-if="r.interval_capped">Capped</template>
          <template v-else>
            ➜ {{ nextIntervalText }} Costs: {{ formatDecimal(r.interval_cost) }} IP
          </template>
        </button>

        <!-- Max galaxies -->
        <button
          class="o-primary-btn c-replicanti-upgrade"
          :class="{ 'o-primary-btn--disabled': !r.can_buy_galaxy_cap }"
          @click="r.can_buy_galaxy_cap && game.buyReplicantiGalaxyCap()"
        >
          Max Replicanti Galaxies: {{ r.galaxy_cap }}
          <br>
          +1 Costs: {{ formatDecimal(r.galaxy_cost) }} IP
        </button>
      </div>

      <p class="c-replicanti-note">
        The Max Replicanti Galaxy upgrade can be purchased endlessly, but costs
        increase more rapidly above 100 Replicanti Galaxies and even more so above
        1000 Replicanti Galaxies.
      </p>

      <button
        v-if="r.can_see_galaxy_button"
        class="o-primary-btn l-replicanti-tab__galaxy"
        :class="{ 'o-primary-btn--disabled': !r.can_buy_galaxy }"
        @click="r.can_buy_galaxy && game.buyReplicantiGalaxy()"
      >
        Reset Replicanti amount for a Replicanti Galaxy
        <br>
        Currently: {{ r.galaxies }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.l-replicanti-tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding-top: 1rem;
}

.c-replicanti-description {
  max-width: 44rem;
  text-align: center;
}

.c-replicanti-description__accent {
  color: var(--color-accent, #5151ec);
  font-weight: bold;
}

.l-replicanti-upgrade-row {
  display: flex;
  gap: 0.6rem;
  justify-content: center;
  flex-wrap: wrap;
}

.c-replicanti-upgrade {
  min-width: 16rem;
}

.c-replicanti-note {
  max-width: 44rem;
  text-align: center;
  opacity: 0.85;
}

.l-replicanti-tab__galaxy {
  min-width: 22rem;
}
</style>
