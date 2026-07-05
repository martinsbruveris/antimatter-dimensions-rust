<script setup>
// The "Offline Progress Simulation" progress bar, shown while an offline
// catch-up of >= 10 s is replayed in chunks. Faithfully mirrors the original
// ModalProgressBar.vue (../antimatter-dimensions/src/components/modals): a
// label, an explanatory paragraph, a "Ticks: current/max" line, a live
// "Remaining" estimate, and the black/blue bar. We deliberately omit the
// original's "Speed up" / "SKIP" buttons — the Rust engine replays even a large
// tick budget near-instantly, so there is nothing to skip. See
// docs/design/2026-06-30-offline-progress.md.
import { computed } from "vue";

import { useUiStore } from "../stores/ui";
import { timeDisplayShort } from "../util/format";

const ui = useUiStore();

const progress = computed(() => ui.offlineProgress ?? { current: 0, max: 1, startTime: 0 });

// Integer tick counts, grouped with thousands separators (the original uses
// formatInt; these values stay well within a plain locale-formatted integer).
function formatInt(n) {
  return Math.round(n).toLocaleString();
}

const fillStyle = computed(() => {
  const p = progress.value;
  const pct = p.max > 0 ? (p.current / p.max) * 100 : 0;
  return { width: `${pct}%` };
});

// Estimated remaining wall-time: elapsed so far scaled by the fraction of ticks
// still to go (matches the original's remainingTime computation).
const remainingTime = computed(() => {
  const p = progress.value;
  if (p.current <= 0) return timeDisplayShort(0);
  const elapsed = Date.now() - p.startTime;
  const ms = (elapsed * (p.max - p.current)) / p.current;
  return timeDisplayShort(ms);
});
</script>

<template>
  <div class="l-modal-overlay c-modal-overlay progress-bar-modal">
    <div class="c-modal">
      <div class="modal-progress-bar">
        <div class="modal-progress-bar__label">
          Offline Progress Simulation
        </div>
        <div>
          The game is being run at a lower accuracy in order to quickly calculate
          the resources you gained while you were away. See the How To Play entry
          on "Offline Progress" for technical details.
        </div>
        <div class="modal-progress-bar__margin">
          <div>
            Ticks: {{ formatInt(progress.current) }}/{{ formatInt(progress.max) }}
          </div>
          <div>
            Remaining: {{ remainingTime }}
          </div>
          <div class="modal-progress-bar__hbox">
            <div class="modal-progress-bar__bg">
              <div
                class="modal-progress-bar__fg"
                :style="fillStyle"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Ported verbatim from the original ModalProgressBar.vue scoped styles. */
.progress-bar-modal {
  z-index: 8;
}

.c-modal {
  position: fixed;
  /* stylelint-disable-next-line unit-allowed-list */
  top: 50vh;
  /* stylelint-disable-next-line unit-allowed-list */
  left: 50vw;
  transform: translate(-50%, -50%);
}

.modal-progress-bar {
  display: flex;
  flex-direction: column;
  width: 40rem;
  z-index: 3;
  justify-content: space-between;
  align-items: center;
}

.modal-progress-bar__hbox {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
}

.modal-progress-bar__bg {
  width: 20rem;
  height: 2rem;
  background: black;
  margin-right: 1rem;
  margin-left: 1rem;
}

.modal-progress-bar__fg {
  height: 100%;
  background: blue;
}

.modal-progress-bar__label {
  font-size: large;
  padding-bottom: 0.5rem;
}

.modal-progress-bar__margin {
  margin: 1rem 0;
}
</style>
