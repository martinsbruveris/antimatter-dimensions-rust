<script setup>
// The Statistics page — a port of the original StatisticsTab.vue reading the
// snapshot's `statistics` view. Omitted relative to the original (systems the
// engine does not model): news-ticker stats, Secret Achievements, paperclips,
// full-game completions, the Doomed real-time line, and the Content Summary
// button. See docs/design/2026-07-10-statistics-tab.md.
import { computed, ref, watch } from "vue";

import { useGameStore } from "../../stores/game";
import {
  formatDecimal,
  formatDateTime,
  formatTime,
  timeDisplayShort,
} from "../../util/format";
import { estimateMatterScale } from "../../util/matterScale";

const game = useGameStore();
const s = computed(() => game.snapshot);
const stats = computed(() => s.value?.statistics);

// The "no fastest Infinity/Eternity yet" sentinel (the original compares
// `bestInfinity.time < 999999999999`).
const NO_BEST_MS = 999999999999;

const infinityUnlocked = computed(() => Boolean(s.value?.infinity_unlocked));
const eternityUnlocked = computed(() => Boolean(s.value?.eternity_unlocked));
const realityUnlocked = computed(() => Boolean(s.value?.reality?.unlocked));
const isDoomed = computed(() => Boolean(stats.value?.is_doomed));

// log10-safe comparisons on raw { m, e } numbers.
const gtZero = (num) => Boolean(num) && num.m > 0;
const gtOneBillion = (num) => gtZero(num) && Math.log10(num.m) + num.e > 9;

// The original `formatDecimalAmount`: full notation above 1e9, else the
// floored integer (run through the formatter for thousand separators, like
// the original's `formatInt`).
function formatDecimalAmount(num) {
  if (!num) return "0";
  if (gtOneBillion(num)) return formatDecimal(num, 3, 0);
  const floored = Math.floor(num.m * Math.pow(10, num.e));
  if (floored === 0) return "0";
  const e = Math.floor(Math.log10(floored));
  return formatDecimal({ m: floored / Math.pow(10, e), e }, 2, 0);
}

// pluralize("Infinity", n) — "y" → "ies" like the original's default rule.
function pluralize(word, num) {
  const isOne = num.m === 1 && num.e === 0;
  if (isOne) return word;
  return word.endsWith("y") ? `${word.slice(0, -1)}ies` : `${word}s`;
}

const infinityCountString = computed(() => {
  const count = stats.value?.infinities;
  return gtZero(count)
    ? `${formatDecimalAmount(count)} ${pluralize("Infinity", count)}`
    : "no Infinities";
});

const eternityCountString = computed(() => {
  const count = stats.value?.eternities;
  return gtZero(count)
    ? `${formatDecimalAmount(count)} ${pluralize("Eternity", count)}`
    : "no Eternities";
});

const realityCountString = computed(() => {
  const count = stats.value?.realities ?? 0;
  return `${count} ${count === 1 ? "Reality" : "Realities"}`;
});

// "Your save was created on X (Y ago)" — hidden when the timestamp is unknown
// (a save the backend never stamped). `game.nowMs` is refreshed every frame.
const createdTime = computed(() => stats.value?.game_created_time_ms ?? 0);
const startDate = computed(() => formatDateTime(createdTime.value));
const saveAge = computed(() => formatTime(game.nowMs - createdTime.value));

// Best Glyph rarity (the original `formatRarity`): whole percent unless the
// tenths digit is non-zero.
const bestRarity = computed(() => {
  const value = stats.value?.best_glyph_rarity ?? 0;
  const places = value.toFixed(1).endsWith(".0") ? 0 : 1;
  return `${value.toFixed(places)}%`;
});

// The matter-scale comparison recomputes at most once per second (the
// original's jitter guard).
const matterScale = ref([]);
let lastMatterTime = 0;
watch(
  () => s.value?.antimatter,
  (antimatter) => {
    if (!antimatter) return;
    if (Date.now() - lastMatterTime > 1000) {
      matterScale.value = estimateMatterScale(antimatter);
      lastMatterTime = Date.now();
    }
  },
  { immediate: true },
);

const realityTitleClass = computed(() => ({
  "c-stats-tab-title": true,
  "c-stats-tab-reality": !isDoomed.value,
  "c-stats-tab-doomed": isDoomed.value,
}));
</script>

<template>
  <div
    v-if="stats"
    class="c-stats-tab"
  >
    <div>
      <div class="c-stats-tab-title c-stats-tab-general">
        General
      </div>
      <div class="c-stats-tab-general">
        <div>You have made a total of {{ formatDecimal(stats.total_antimatter, 2, 1) }} antimatter.</div>
        <div>You have played for {{ formatTime(stats.real_time_played_ms) }}. (real time)</div>
        <div v-if="realityUnlocked">
          Your existence has spanned {{ formatTime(stats.total_time_played_ms) }} of time. (game time)
        </div>
        <div v-if="createdTime > 0">
          Your save was created on {{ startDate }} ({{ saveAge }} ago)
        </div>
      </div>
      <div>
        <br>
        <div class="c-matter-scale-container c-stats-tab-general">
          <div
            v-for="(line, i) in matterScale"
            :key="i"
          >
            {{ line }}
          </div>
          <br v-if="matterScale.length < 2">
          <br v-if="matterScale.length < 3">
        </div>
      </div>
      <br>
    </div>
    <div
      v-if="infinityUnlocked"
      class="c-stats-tab-subheader c-stats-tab-general"
    >
      <div class="c-stats-tab-title c-stats-tab-infinity">
        Infinity
      </div>
      <div>
        You have {{ infinityCountString }}<span v-if="eternityUnlocked"> this Eternity</span>.
      </div>
      <div v-if="gtZero(stats.infinities_banked)">
        You have {{ formatDecimalAmount(stats.infinities_banked) }}
        {{ pluralize("Banked Infinity", stats.infinities_banked) }}.
      </div>
      <div v-if="stats.best_infinity_time_ms < NO_BEST_MS">
        Your fastest Infinity was {{ timeDisplayShort(stats.best_infinity_time_ms) }}.
      </div>
      <div v-else>
        You have no fastest Infinity<span v-if="eternityUnlocked"> this Eternity</span>.
      </div>
      <div>
        You have spent {{ timeDisplayShort(stats.this_infinity_time_ms) }} in this Infinity.
        <span v-if="realityUnlocked">
          ({{ timeDisplayShort(stats.this_infinity_real_time_ms) }} real time)
        </span>
      </div>
      <div>
        Your best Infinity Points per minute
        <span v-if="gtZero(stats.eternities)">this Eternity </span>
        is {{ formatDecimal(stats.best_ip_min, 2, 2) }}.
      </div>
      <br>
    </div>
    <div
      v-if="eternityUnlocked"
      class="c-stats-tab-subheader c-stats-tab-general"
    >
      <div class="c-stats-tab-title c-stats-tab-eternity">
        Eternity
      </div>
      <div>
        You have {{ eternityCountString }}<span v-if="realityUnlocked"> this Reality</span>.
      </div>
      <div v-if="gtZero(stats.projected_banked)">
        You will gain {{ formatDecimalAmount(stats.projected_banked) }}
        {{ pluralize("Banked Infinity", stats.projected_banked) }} on Eternity
        ({{ formatDecimalAmount(stats.banked_rate_per_min) }} per minute).
      </div>
      <div v-else-if="gtZero(stats.infinities_banked)">
        You will gain no Banked Infinities on Eternity.
      </div>
      <div v-if="stats.best_eternity_time_ms < NO_BEST_MS">
        Your fastest Eternity was {{ timeDisplayShort(stats.best_eternity_time_ms) }}.
      </div>
      <div v-else>
        You have no fastest Eternity<span v-if="realityUnlocked"> this Reality</span>.
      </div>
      <div>
        You have spent {{ timeDisplayShort(stats.this_eternity_time_ms) }} in this Eternity.
        <span v-if="realityUnlocked">
          ({{ timeDisplayShort(stats.this_eternity_real_time_ms) }} real time)
        </span>
      </div>
      <div>
        Your best Eternity Points per minute
        <span v-if="realityUnlocked">this Reality </span>
        is {{ formatDecimal(stats.best_ep_min, 2, 2) }}.
      </div>
      <br>
    </div>
    <div
      v-if="realityUnlocked"
      class="c-stats-tab-subheader c-stats-tab-general"
    >
      <div :class="realityTitleClass">
        {{ isDoomed ? "Doomed Reality" : "Reality" }}
      </div>
      <div>You have {{ realityCountString }}.</div>
      <div>Your fastest game-time Reality was {{ timeDisplayShort(stats.best_reality_time_ms) }}.</div>
      <div>Your fastest real-time Reality was {{ timeDisplayShort(stats.best_reality_real_time_ms) }}.</div>
      <div :class="{ 'c-stats-tab-doomed': isDoomed }">
        You have spent {{ timeDisplayShort(stats.this_reality_time_ms) }}
        in this {{ isDoomed ? "Armageddon" : "Reality" }}.
        ({{ timeDisplayShort(stats.this_reality_real_time_ms) }} real time)
      </div>
      <div>
        Your best Reality Machines per minute is {{ formatDecimal(stats.best_rm_min, 2, 2) }}.
      </div>
      <div>Your best Glyph rarity is {{ bestRarity }}.</div>
      <br>
    </div>
  </div>
</template>

<style scoped>
.c-matter-scale-container {
  height: 5rem;
}

.c-stats-tab-general {
  color: var(--color-text);
}

.c-stats-tab-title {
  font-size: 2rem;
  font-weight: bold;
}

.c-stats-tab-subheader {
  height: 15rem;
}

.c-stats-tab-infinity {
  color: var(--color-infinity);
}

.c-stats-tab-eternity {
  color: var(--color-eternity);
}

.c-stats-tab-reality {
  color: var(--color-reality);
}

.c-stats-tab-doomed {
  color: var(--color-pelle--base);
}
</style>
