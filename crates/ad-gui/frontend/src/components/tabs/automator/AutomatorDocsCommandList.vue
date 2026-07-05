<script setup>
// The command reference (vendored from AutomatorDocsCommandList.vue +
// AutomatorDocsManPage.vue, with the man pages from the vendored docs data).
import { computed, ref } from "vue";

import { useGameStore } from "../../../stores/game";
import {
  AUTOMATOR_CATEGORY_NAMES,
  AUTOMATOR_COMMAND_DOCS,
} from "../../../data/automatorDocs";

const game = useGameStore();
const selectedCommand = ref(-1);

// `unlock` gates: "blackHole" needs BH1; "enslaved" is celestial (never at
// our frontier).
const unlockedCommands = computed(() =>
  AUTOMATOR_COMMAND_DOCS.filter((c) => {
    if (c.unlock === null) return true;
    if (c.unlock === "blackHole") {
      return game.snapshot.reality.black_holes.unlocked;
    }
    return false;
  }),
);

function commandsInCategory(category) {
  return unlockedCommands.value.filter((c) => c.category === category);
}

const command = computed(() =>
  AUTOMATOR_COMMAND_DOCS.find((c) => c.id === selectedCommand.value),
);
</script>

<template>
  <div>
    <div v-if="command">
      <button
        class="c-automator-docs--button l-return-button fas fa-arrow-left"
        @click="selectedCommand = -1"
      />
      Return to the Command List
      <div class="c-automator-docs-page">
        <b>NAME</b>
        <div
          class="c-automator-docs-page__indented"
          v-html="command.keyword"
        />
        <b>SYNTAX</b>
        <div
          class="c-automator-docs-page__indented"
          v-html="command.syntax"
        />
        <template v-if="command.description">
          <b>DESCRIPTION</b>
          <div
            class="c-automator-docs-page__indented"
            v-html="command.description"
          />
        </template>
        <template
          v-for="section in command.sections ?? []"
          :key="section.name"
        >
          <b>{{ section.name }}</b>
          <div
            v-for="item in section.items"
            :key="item.header"
            class="c-automator-docs-page__indented"
          >
            <div v-html="item.header" />
            <div
              class="c-automator-docs-page__indented"
              v-html="item.description"
            />
          </div>
        </template>
        <template v-if="command.examples">
          <b>USAGE EXAMPLES</b>
          <div
            v-for="example in command.examples"
            :key="example"
            class="c-automator-docs-page__indented"
            v-html="example"
          />
        </template>
      </div>
    </div>
    <div
      v-else
      class="c-automator-docs-page"
    >
      Click on an underlined command to see more details on syntax, usage, and functionality.
      <br>
      <br>
      <span>Command List:</span>
      <br>
      <div
        v-for="(category, i) in AUTOMATOR_CATEGORY_NAMES"
        :key="i"
      >
        {{ category }} ({{ commandsInCategory(i).length }} commands)
        <div
          v-for="entry in commandsInCategory(i)"
          :key="entry.id"
          class="c-automator-docs-page__link l-command-group"
          @click="selectedCommand = entry.id"
        >
          <span>{{ entry.keyword }}</span>
        </div>
      </div>
      <br>
      <span>
        Note: In the SYNTAX note on each command, <u>underlined</u> inputs are <i>required</i> inputs which you must
        fill and inputs in [square brackets] are optional (if used, they should be input <i>without</i> the brackets).
        Any other parts should be typed in as they appear. Unless otherwise stated, all of the inputs are
        case-insensitive. Some commands may have more than one valid format, which will appear on separate lines.
      </span>
    </div>
  </div>
</template>

<style scoped>
/* Vendored from the original AutomatorDocsCommandList.vue scoped style. */
.l-command-group {
  display: flex;
  flex-direction: column;
  padding-left: 1rem;
}

.l-return-button {
  width: 4rem;
  height: 2.6rem;
  font-size: 1.8rem;
  margin-left: 2rem;
}
</style>
