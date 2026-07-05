<script setup>
// Mode selector for the prestige autobuyers: a simplified port of the
// original's ExpandingControlBox + AutobuyerDropdownEntry (same approach as
// SelectNotationDropdown) — a "▼ Current Setting: ▼" header expanding an
// inline option list.
import { ref } from "vue";

const props = defineProps({
  // [{ id, label }] in display order.
  modes: { type: Array, required: true },
  // Currently selected mode id.
  mode: { type: String, required: true },
});

const emit = defineEmits(["select"]);
const open = ref(false);

function currentLabel() {
  return props.modes.find((m) => m.id === props.mode)?.label ?? "";
}

function select(id) {
  emit("select", id);
  open.value = false;
}
</script>

<template>
  <div class="l-expanding-control-box">
    <div
      class="l-expanding-control-box__container"
      @mouseleave="open = false"
    >
      <div
        class="o-primary-btn c-autobuyer-box__mode-select c-autobuyer-box__mode-select-header"
        @click="open = !open"
      >
        ▼ Current Setting: ▼
        <br>
        {{ currentLabel() }}
      </div>
      <div v-show="open">
        <div
          v-for="option in modes"
          :key="option.id"
          class="o-primary-btn c-autobuyer-box__mode-select l-autobuyer-choice"
          @click="select(option.id)"
        >
          {{ option.label }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Replicated from the original ExpandingControlBox.vue scoped style (these
   layout classes are not in the global vendored CSS). */
.l-expanding-control-box {
  position: relative;
  z-index: 3;
  height: 4.5rem;
  width: 100%;
}

.l-expanding-control-box__container {
  display: block;
  width: 100%;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.c-autobuyer-box__mode-select-header {
  cursor: pointer;
  width: 100%;
}

/* Replicated from AutobuyerDropdownEntry.vue's scoped style. */
.l-autobuyer-choice {
  display: block;
  width: 100%;
  border-radius: 0;
  border-top: 0;
  box-shadow: none;
}

.l-autobuyer-choice:hover {
  background-color: var(--color-good);
}

.l-autobuyer-choice:last-child {
  border-radius: 0 0 var(--var-border-radius, 0.5rem) var(--var-border-radius, 0.5rem);
}
</style>
