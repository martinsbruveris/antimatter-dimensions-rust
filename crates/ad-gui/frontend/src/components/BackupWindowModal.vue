<script setup>
// "Automatic Backup Saves" popup, opened from the Saving options tab. Mirrors
// the original game's BackupWindowModal.vue + BackupEntry.vue
// (../antimatter-dimensions/src/components/modals/options) — the explanatory
// text, the "load without offline" toggle, the grid of backup-slot entries and
// the import/export file buttons. Backups are not wired up yet, so every slot
// shows the empty state. Renders inside our shared Modal.vue wrapper.
import { ref } from "vue";

import Modal from "./Modal.vue";

defineEmits(["close"]);

const ignoreOffline = ref(false);

// The eight automatic backup slots, mirroring core/storage `AutoBackupSlots`.
// `desc` is the original BackupEntry `slotType` string for each slot.
const backupSlots = [
  { id: 1, desc: "Saves every 1 minute online" },
  { id: 2, desc: "Saves every 5 minutes online" },
  { id: 3, desc: "Saves every 20 minutes online" },
  { id: 4, desc: "Saves every 1 hour online" },
  { id: 5, desc: "Saves after 10 minutes offline" },
  { id: 6, desc: "Saves after 1 hour offline" },
  { id: 7, desc: "Saves after 5 hours offline" },
  { id: 8, desc: "Pre-loading save" },
];
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
          v-for="slot in backupSlots"
          :key="slot.id"
          class="l-backup-entry c-bordered-entry"
        >
          <h3>Slot #{{ slot.id }}:</h3>
          <span>(Empty)</span>
          <span>{{ slot.desc }}</span>
          <span class="c-fixed-height">Slot not currently in use</span>
          <button class="o-primary-btn o-primary-btn--width-medium o-primary-btn--disabled">
            Load
          </button>
        </div>
      </div>
      These backups are still stored in the same place as your game save and can still be lost if you do anything
      external to the game which would delete your save itself, such as clearing your browser cookies. You can
      import/export all backups at once as files, using these buttons:
      <div class="c-backup-file-ops">
        <button class="o-primary-btn o-btn-file-ops">
          Export as file
        </button>
        <button class="o-primary-btn o-btn-file-ops c-file-import-button">
          <input
            class="c-file-import"
            type="file"
            accept=".txt"
          >
          <label for="file">Import from file</label>
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

/* The "Import from file" button uses the vendored `.c-file-import` hack, whose
   invisible `::before` (font-size: 100rem) overflows hugely. Inside the modal's
   scrollable content that overflow becomes scrollable empty space far past the
   end. Clip it to the button (the input still fills its own button, so import
   stays clickable). Same fix as the Saving tab's file-import button. */
.c-file-import-button {
  overflow: hidden;
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
