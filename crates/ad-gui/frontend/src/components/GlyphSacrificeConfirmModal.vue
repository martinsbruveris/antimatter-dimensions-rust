<script setup>
// Glyph sacrifice/delete confirmation (modals/DeleteGlyphModal /
// glyph-management sacrifice modal). The target glyph rides in
// `ui.modalPayload`.
import { computed } from "vue";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import ConfirmModal from "./ConfirmModal.vue";

const game = useGameStore();
const ui = useUiStore();

const payload = computed(() => ui.modalPayload ?? {});
const glyph = computed(() => payload.value.glyph);
const canSacrifice = computed(() => Boolean(payload.value.canSacrifice));

const title = computed(() =>
  canSacrifice.value ? "Glyph Sacrifice" : "Deleting a Glyph"
);
const message = computed(() => {
  if (!glyph.value) return "";
  if (glyph.value.kind === "companion") {
    return "Are you sure you want to delete your Companion Glyph? It loves you very much.";
  }
  return canSacrifice.value
    ? `Do you really want to sacrifice this Glyph? Your total power of sacrificed
       ${glyph.value.kind} Glyphs will increase by
       ${glyph.value.sacrifice_value.toExponential(2)}.`
    : "Do you really want to delete this Glyph? You gain nothing in return.";
});

function confirm() {
  if (glyph.value) game.sacrificeGlyph(glyph.value.id);
  ui.closeModal();
}
</script>

<template>
  <ConfirmModal
    :title="title"
    @confirm="confirm"
    @close="ui.closeModal()"
  >
    {{ message }}
  </ConfirmModal>
</template>
