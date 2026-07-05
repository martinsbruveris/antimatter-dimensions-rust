<script setup>
// The event-log page (vendored from AutomatorEventLog.vue): sorting +
// timestamp-style controls (persisted in options.automatorEvents) and the
// entries, polled from the engine while the page is open.
import { computed, onMounted, onUnmounted, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import { timeDisplayShort } from "../../../util/format";
import { AutomatorTextUI } from "../../../util/automatorEditor";

const TIMESTAMP_MODE = {
  DISABLED: 0,
  THIS_REALITY: 1,
  RELATIVE_NOW: 2,
  RELATIVE_PREV: 3,
  DATE_TIME: 4,
};

const game = useGameStore();
const log = ref({ now_play_time_ms: 0, events: [] });
// Wall-clock instant matching the engine's play-time clock at fetch, for the
// DATE_TIME style (the engine has no wall clock).
const fetchedAt = ref(Date.now());
let pollTimer = null;

const options = computed(() => game.snapshot.automator.event_options);

async function refresh() {
  log.value = await game.getAutomatorEvents();
  fetchedAt.value = Date.now();
}

onMounted(() => {
  refresh();
  pollTimer = setInterval(refresh, 500);
});
onUnmounted(() => clearInterval(pollTimer));

const events = computed(() => {
  const sorted = [...log.value.events].sort((a, b) =>
    a.play_time_ms === b.play_time_ms
      ? a.this_reality_ms === b.this_reality_ms
        ? a.line - b.line
        : a.this_reality_ms - b.this_reality_ms
      : a.play_time_ms - b.play_time_ms,
  );
  return options.value.newest_first ? sorted.reverse() : sorted;
});

function setOption(option, value) {
  game.setAutomatorEventOption(option, value ? 1 : 0);
}

function setTimestampMode(key) {
  game.setAutomatorEventOption("timestampType", TIMESTAMP_MODE[key]);
}

function selectedStyle(selected) {
  return { "background-color": selected ? "var(--color-reality)" : "" };
}

function timestamp(entry) {
  const mode = options.value.timestamp_type;
  switch (mode) {
    case TIMESTAMP_MODE.DISABLED:
      return "";
    case TIMESTAMP_MODE.THIS_REALITY:
      return `, ${timeDisplayShort(entry.this_reality_ms)} (real-time) in Reality`;
    case TIMESTAMP_MODE.RELATIVE_NOW:
      return `, ${timeDisplayShort(log.value.now_play_time_ms - entry.play_time_ms)} ago`;
    case TIMESTAMP_MODE.RELATIVE_PREV:
      if (entry.timegap_ms === entry.play_time_ms) return ", first logged event";
      return `, ${timeDisplayShort(entry.timegap_ms)} after previous event`;
    case TIMESTAMP_MODE.DATE_TIME: {
      const wall =
        fetchedAt.value - (log.value.now_play_time_ms - entry.play_time_ms);
      return `, ${new Date(wall).toLocaleString()}`;
    }
    default:
      return "";
  }
}

function scrollToLine(line) {
  AutomatorTextUI.scrollToLine(line);
  AutomatorTextUI.updateHighlightedLine(line, "event");
}

async function clearLog() {
  await game.automatorClearLog();
  refresh();
}
</script>

<template>
  <div class="c-automator-docs-page">
    <div>
      This panel keeps a running event log of all the commands which the automator has recently executed, with a little
      extra info on some of the commands. It may be useful to help you find problems if you find your automator is
      getting stuck in certain spots.
      <br>
      <br>
      While your settings are kept within your savefile, the actual events are not and will disappear on refresh.
      <br>
      <br>
      <b>Entry Sorting:</b>
      <button
        title="Oldest results first"
        :style="selectedStyle(!options.newest_first)"
        class="c-automator-docs--button fas fa-angle-down"
        @click="setOption('newestFirst', false)"
      />
      <button
        title="Newest results first"
        :style="selectedStyle(options.newest_first)"
        class="c-automator-docs--button fas fa-angle-up"
        @click="setOption('newestFirst', true)"
      />
      <button
        :title="`Clear all entries (Max. ${options.max_entries})`"
        class="c-automator-docs--button fas fa-trash"
        @click="clearLog"
      />
      <button
        title="Clear event log every Reality"
        :style="selectedStyle(options.clear_on_reality)"
        class="c-automator-docs--button fas fa-eraser"
        @click="setOption('clearOnReality', !options.clear_on_reality)"
      />
      <button
        title="Clear event log on script restart"
        :style="selectedStyle(options.clear_on_restart)"
        class="c-automator-docs--button fas fa-backspace"
        @click="setOption('clearOnRestart', !options.clear_on_restart)"
      />
    </div>
    <div>
      <b>Timestamp style:</b>
      <button
        title="No timestamps"
        :style="selectedStyle(options.timestamp_type === TIMESTAMP_MODE.DISABLED)"
        class="c-automator-docs--button fas fa-ban"
        @click="setTimestampMode('DISABLED')"
      />
      <button
        title="Current time this Reality"
        :style="selectedStyle(options.timestamp_type === TIMESTAMP_MODE.THIS_REALITY)"
        class="c-automator-docs--button fas fa-stopwatch"
        @click="setTimestampMode('THIS_REALITY')"
      />
      <button
        title="Time elapsed since event"
        :style="selectedStyle(options.timestamp_type === TIMESTAMP_MODE.RELATIVE_NOW)"
        class="c-automator-docs--button fas fa-clock"
        @click="setTimestampMode('RELATIVE_NOW')"
      />
      <button
        title="Time since last event"
        :style="selectedStyle(options.timestamp_type === TIMESTAMP_MODE.RELATIVE_PREV)"
        class="c-automator-docs--button fas fa-arrow-left"
        @click="setTimestampMode('RELATIVE_PREV')"
      />
      <button
        title="Date and time"
        :style="selectedStyle(options.timestamp_type === TIMESTAMP_MODE.DATE_TIME)"
        class="c-automator-docs--button fas fa-user-clock"
        @click="setTimestampMode('DATE_TIME')"
      />
    </div>
    <span
      v-for="(event, id) in events"
      :key="id"
    >
      <b>Line {{ event.line }}{{ timestamp(event) }}:</b>
      <button
        title="Jump to line"
        class="c-automator-docs--button fas fa-arrow-circle-right"
        @click="scrollToLine(event.line)"
      />
      <div class="c-automator-docs-page__indented">
        <i>{{ event.message }}</i>
      </div>
    </span>
  </div>
</template>
