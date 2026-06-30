<script setup>
// "Don't show this message again" checkbox inside a confirmation modal.
// Ported from the original ModalConfirmationCheck.vue: toggling it flips the
// corresponding `player.options.confirmations.<option>` flag in the engine.
// The modal only opens while the confirmation is on, so the checkbox always
// starts unchecked (setting = true = "keep showing").
import { ref } from "vue";

import { useGameStore } from "../stores/game";

const props = defineProps({
  // The engine confirmation kind (camelCase), e.g. "dimensionBoost".
  option: { type: String, required: true },
});

const game = useGameStore();
// `setting` mirrors the original: true = confirmation stays on (box inactive),
// false = disabled (box shows a check).
const setting = ref(true);

function toggle() {
  setting.value = !setting.value;
  game.setConfirmation(props.option, setting.value);
}
</script>

<template>
  <div
    class="c-modal__confirmation-toggle"
    @click="toggle"
  >
    <div
      class="c-modal__confirmation-toggle__checkbox"
      :class="{ 'c-modal__confirmation-toggle__checkbox--active': !setting }"
    >
      <span
        v-if="!setting"
        class="fas fa-check"
      />
      <div class="c-modal__confirmation-toggle__tooltip">
        {{ setting ? "Disable" : "Reenable" }} this confirmation
      </div>
    </div>
    <span class="c-modal__confirmation-toggle__text">
      Don't show this message again
    </span>
  </div>
</template>
