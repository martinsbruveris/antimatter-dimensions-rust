<script setup>
// The editor pane's controls bar (vendored from AutomatorControls.vue):
// rewind / play-pause / stop / step, the repeat / force-restart / follow
// toggles, the mode switch, and the status line. The undo/redo pair is served
// by CodeMirror's native history (mod+z / mod+y in the editor), so unlike the
// original's cross-editor undo buffer it only appears in text mode.
import { computed } from "vue";

import { useGameStore } from "../../../stores/game";
import { AutomatorTextUI, automatorErrors } from "../../../util/automatorEditor";
import AutomatorButton from "./AutomatorButton.vue";
import AutomatorModeSwitch from "./AutomatorModeSwitch.vue";

const game = useGameStore();
const auto = computed(() => game.snapshot.automator);

const isRunning = computed(() => auto.value.is_running);
const isPaused = computed(() => auto.value.is_on && !auto.value.is_running);
const hasErrors = computed(() => automatorErrors.value.length !== 0);

const playButtonClass = computed(() => ({
  "c-automator__button--active": isRunning.value,
  "fa-play": !isRunning.value && !isPaused.value,
  "fa-pause": isRunning.value,
  "fa-eject": isPaused.value,
}));

const playTooltip = computed(() => {
  if (isPaused.value) return "Resume Automator execution";
  if (!isRunning.value) return "Start Automator";
  return "Pause Automator execution";
});

const statusName = computed(() => {
  if (isPaused.value || isRunning.value) return auto.value.running_script_name;
  return (
    auto.value.scripts.find((s) => s.id === auto.value.editor_script)?.name ?? ""
  );
});

const editingDifferentScript = computed(
  () =>
    (isRunning.value || isPaused.value) &&
    auto.value.editor_script !== auto.value.top_level_script,
);

const duplicateName = computed(
  () =>
    auto.value.scripts.filter((s) => s.name === statusName.value).length > 1,
);

const statusText = computed(() => {
  // Pad the line number to the script's digit count to prevent jitter.
  const scriptLines = AutomatorTextUI.editor?.getDoc().lineCount() ?? 1;
  const digits = Math.max(Math.ceil(Math.log10(scriptLines + 1)), 1);
  const lineNum = `0000${auto.value.current_line}`.slice(-digits);

  if (isPaused.value) return `Paused: "${statusName.value}" (Resumes on Line ${lineNum})`;
  if (isRunning.value) return `Running: "${statusName.value}" (Line ${lineNum})`;
  if (hasErrors.value) return `Stopped: "${statusName.value}" has errors (Cannot run)`;
  return `Stopped: Will start running "${statusName.value}"`;
});

function play() {
  if (hasErrors.value && !auto.value.is_on) return;
  game.automatorPlay(auto.value.editor_script);
}
</script>

<template>
  <div class="c-automator__controls l-automator__controls">
    <div class="c-automator-control-row l-automator-button-row">
      <div class="c-button-group">
        <AutomatorButton
          title="Rewind Automator to the first command"
          class="fa-fast-backward"
          @click="game.automatorRewind()"
        />
        <AutomatorButton
          :title="playTooltip"
          :class="playButtonClass"
          @click="play"
        />
        <AutomatorButton
          title="Stop Automator and reset position"
          class="fa-stop"
          @click="game.automatorStop()"
        />
        <AutomatorButton
          title="Step forward one line"
          class="fa-step-forward"
          @click="game.automatorStep(auto.editor_script)"
        />
        <AutomatorButton
          title="Restart script automatically when it reaches the end"
          class="fa-sync-alt"
          :class="{ 'c-automator__button--active': auto.repeat }"
          @click="game.automatorToggleSetting('repeat')"
        />
        <AutomatorButton
          title="Automatically restart the active script when finishing or restarting a Reality"
          class="fa-reply"
          :class="{ 'c-automator__button--active': auto.force_restart }"
          @click="game.automatorToggleSetting('forceRestart')"
        />
        <AutomatorButton
          title="Scroll Automator to follow current line"
          class="fa-indent"
          :class="{ 'c-automator__button--active': auto.follow_execution }"
          @click="game.automatorToggleSetting('followExecution')"
        />
      </div>
      <div class="c-button-group">
        <template v-if="auto.editor_type === 'text'">
          <AutomatorButton
            title="Undo (also Ctrl+Z in the editor)"
            class="fa-arrow-rotate-left"
            @click="AutomatorTextUI.editor?.undo()"
          />
          <AutomatorButton
            title="Redo (also Ctrl+Y in the editor)"
            class="fa-arrow-rotate-right"
            @click="AutomatorTextUI.editor?.redo()"
          />
        </template>
        <AutomatorModeSwitch />
      </div>
    </div>
    <div class="l-automator-button-row">
      <span
        v-if="duplicateName"
        title="More than one script has this name!"
        class="fas fa-exclamation-triangle c-automator__status-text c-automator__status-text--error"
      />
      <span
        v-if="editingDifferentScript"
        title="The automator is running a different script than the editor is showing"
        class="fas fa-circle-exclamation c-automator__status-text c-automator__status-text--warning"
      />
      <span
        v-if="auto.just_completed && !auto.is_on"
        title="The automator completed running the previous script"
        class="fas fa-circle-check c-automator__status-text"
      />
      <span
        class="c-automator__status-text"
        :class="{ 'c-automator__status-text--error': hasErrors && !(isRunning || isPaused) }"
      >
        {{ statusText }}
      </span>
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorControls.vue scoped style. */
.c-automator-control-row {
  justify-content: space-between;
}

.c-button-group {
  display: flex;
  flex-direction: row;
  align-items: center;
}

.c-automator__status-text {
  font-size: 1.3rem;
  font-weight: bold;
  color: var(--color-reality);
  padding: 0 0.5rem;
}

.c-automator__status-text--warning {
  color: var(--color-good-paused);
}

.c-automator__status-text--error {
  color: var(--color-bad);
}

.c-automator__button--active {
  background-color: var(--color-automator-controls-active);
  border-color: var(--color-reality-light);
}

:deep(.c-automator__button.fa-eject::before) {
  transform: rotate(90deg);
}
</style>
