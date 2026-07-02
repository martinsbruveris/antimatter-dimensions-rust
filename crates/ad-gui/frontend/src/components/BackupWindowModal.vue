<script setup>
// "Automatic Backup Saves" popup, opened from the Saving options tab. Mirrors
// the original game's BackupWindowModal.vue + BackupEntry.vue
// (../antimatter-dimensions/src/components/modals/options) — the explanatory
// text, the "load without offline" toggle, the grid of backup-slot entries and
// the import/export file buttons. Wired to the engine's per-save-slot backup
// files: it fetches per-slot summaries on open, loads a backup on click, and
// exports/imports the whole backup set as a file (§2.4 bundle). Renders inside
// our shared Modal.vue wrapper.
import { onMounted, onUnmounted, ref } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { formatDecimal, formatTime } from "../util/format";
import Modal from "./Modal.vue";

defineEmits(["close"]);

const game = useGameStore();
const ui = useUiStore();

const ignoreOffline = ref(false);

// The eight automatic backup slots, mirroring core/storage `AutoBackupSlots`.
// `desc` is the original BackupEntry `slotType` string for each slot.
const BACKUP_DESCRIPTIONS = {
  1: "Saves every 1 minute online",
  2: "Saves every 5 minutes online",
  3: "Saves every 20 minutes online",
  4: "Saves every 1 hour online",
  5: "Saves after 10 minutes offline",
  6: "Saves after 1 hour offline",
  7: "Saves after 5 hours offline",
  8: "Pre-loading save",
};

// Per-slot summaries from the engine:
// { id, exists, antimatter: {m,e}, last_backup_ms }.
const slots = ref([]);

async function refresh() {
  slots.value = await game.getBackups();
}

// The elapsed "Last saved … ago" ticks off the store clock every frame, but the
// backups themselves only change when one is written (an online backup fires, or
// a load reserves slot 8). Re-poll while the modal is open so a new backup's
// timestamp (reset to ~0) and antimatter are picked up; 1 s is plenty responsive
// for the minute-plus online intervals and avoids re-decoding the files per frame.
let pollId = null;

onMounted(() => {
  refresh();
  pollId = setInterval(refresh, 1000);
});

onUnmounted(() => {
  if (pollId) clearInterval(pollId);
});

function description(id) {
  return BACKUP_DESCRIPTIONS[id];
}

// "Last saved: X ago", mirroring the original BackupEntry. Reading the store's
// per-frame `nowMs` clock inside the render makes it a reactive dependency, so
// the elapsed time ticks in real time (the backup's own timestamp is fixed).
function lastSaved(slot) {
  if (!slot.exists) return "Slot not currently in use";
  const elapsed = Math.max(0, game.nowMs - slot.last_backup_ms);
  return `Last saved: ${formatTime(elapsed)} ago`;
}

async function loadBackup(slot) {
  if (!slot.exists) return;
  await game.loadBackup(slot.id);
  ui.notify("Game loaded");
  ui.closeModal();
}

async function exportBackups() {
  try {
    await game.exportBackupsToFile();
    ui.notify("Backups exported to file");
  } catch (e) {
    if (e !== "Cancelled") ui.notify(`Export failed: ${e}`, "error");
  }
}

async function importBackups() {
  try {
    const count = await game.importBackupsFromFile();
    ui.notify(`Imported ${count} backup${count === 1 ? "" : "s"}`);
    await refresh();
  } catch (e) {
    if (e !== "Cancelled") ui.notify(`Import failed: ${e}`, "error");
  }
}
</script>

<template>
  <Modal
    title="Automatic Backup Saves"
    compact
    fit-content
    @close="$emit('close')"
  >
    <div class="c-info c-modal--short">
      The game makes automatic backups based on time you have spent online or offline.
      Timers for online backups only run when the game is open, and offline backups only save to the slot
      with the longest applicable timer.
      Additionally, your current save is saved into the last slot any time a backup from here is loaded.
      <div
        class="c-modal__confirmation-toggle"
        @click="ignoreOffline = !ignoreOffline"
      >
        <div
          class="c-modal__confirmation-toggle__checkbox"
          :class="{ 'c-modal__confirmation-toggle__checkbox--active': ignoreOffline }"
        >
          <span
            v-if="ignoreOffline"
            class="fas fa-check"
          />
        </div>
        <span class="c-modal__confirmation-toggle__text">
          Load with offline progress disabled
        </span>
      </div>
      <div class="c-entry-container">
        <div
          v-for="slot in slots"
          :key="slot.id"
          class="l-backup-entry c-bordered-entry"
        >
          <h3>Slot #{{ slot.id }}:</h3>
          <span v-if="slot.exists">Antimatter: {{ formatDecimal(slot.antimatter, 2, 1) }}</span>
          <span v-else>(Empty)</span>
          <span>{{ description(slot.id) }}</span>
          <span class="c-fixed-height">
            {{ lastSaved(slot) }}
          </span>
          <button
            class="o-primary-btn o-primary-btn--width-medium"
            :class="{ 'o-primary-btn--disabled': !slot.exists }"
            @click="loadBackup(slot)"
          >
            Load
          </button>
        </div>
      </div>
      These backups are still stored in the same place as your game save and can still be lost if you do anything
      external to the game which would delete your save itself, such as clearing your browser cookies. You can
      import/export all backups at once as files, using these buttons:
      <div class="c-backup-file-ops">
        <button
          class="o-primary-btn o-btn-file-ops"
          @click="exportBackups"
        >
          Export as file
        </button>
        <button
          class="o-primary-btn o-btn-file-ops"
          @click="importBackups"
        >
          Import from file
        </button>
      </div>
      Each of your three save slots has its own separate set of backups.
    </div>
  </Modal>
</template>

<style scoped>
/* Replicated from the original BackupWindowModal.vue / BackupEntry.vue
   <style scoped> blocks (these classes are not part of the vendored global
   CSS). */
.c-info {
  width: 60rem;
  padding-right: 1rem;
  overflow-x: hidden;
  /* The original game's base `.c-modal` centres its text; our shared Modal.vue
     overrides that to left-align for the long-form Info/Hotkeys/Notation
     modals, so restore centring here to match the original backup modal. */
  text-align: center;
}

.c-backup-file-ops {
  margin: 0.5rem;
}

.o-btn-file-ops {
  margin: 0 0.5rem;
}

.c-entry-container {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
}

.l-backup-entry {
  width: calc(50% - 0.6rem);
  height: calc(25% - 0.6rem);
}

.c-bordered-entry {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0.5rem 0.3rem;
  margin: 0.3rem;
  font-size: 1.1rem;
  border: var(--var-border-width, 0.2rem) solid;
  border-radius: var(--var-border-radius, 0.4rem);
}

.c-fixed-height {
  height: 4rem;
}
</style>
