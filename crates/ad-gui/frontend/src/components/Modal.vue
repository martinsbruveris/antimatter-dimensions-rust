<script setup>
defineProps({
  title: { type: String, default: "" },
  // Extra class(es) for the title (e.g. `c-game-header__antimatter` to make
  // the credits title red, matching the original).
  titleClass: { type: String, default: "" },
  // Size the modal to its content instead of the wide default Information-
  // modal width. Used by the Hotkey List modal, which is only as wide as
  // its two columns (matching the original).
  fitContent: { type: Boolean, default: false },
});

const emit = defineEmits(["close"]);
</script>

<template>
  <!-- Overlay closes the modal when the backdrop (not the modal body) is
       clicked; mirrors the original game's modal behaviour. -->
  <div
    class="l-modal-overlay"
    @click.self="emit('close')"
  >
    <div class="l-modal">
      <div
        class="c-modal c-modal-text"
        :class="{ 'c-modal-text--fit': fitContent }"
      >
        <div
          class="c-modal__close-btn"
          @click="emit('close')"
        >
          <i class="fas fa-times" />
        </div>
        <!-- Title sits in a non-scrolling header so it stays pinned (sticky)
             while the body below scrolls, matching the original. -->
        <div
          v-if="title"
          class="c-modal__title"
          :class="titleClass"
        >
          {{ title }}
        </div>
        <div class="l-modal-text__content">
          <slot />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Pin to the viewport so the popup is centred in the whole window
   regardless of the scrolled/positioned game-container it renders inside.
   Transparent backdrop: still captures backdrop clicks to close, but does
   not grey out the main screen. */
.l-modal-overlay {
  position: fixed;
  background-color: transparent;
  animation: none;
}

.l-modal {
  position: fixed;
}

/* Width matches the original Information modal: a wide panel that scales
   with the viewport (calc(100vw - 50vh)). */
.c-modal-text {
  width: calc(100vw - 50vh);
  max-width: 95vw;
  font-size: 2rem;
  text-align: left;
}

/* Shrink-to-fit variant: only as wide as the content needs (e.g. the
   Hotkey List's two columns), defined after the rule above so it wins. */
.c-modal-text--fit {
  width: fit-content;
}

/* Title spans the full width, stays centred, and is larger than the
   body text, matching the original. Colour comes from the base modal
   (white) or from `titleClass` (e.g. red `c-game-header__antimatter`). */
.c-modal-text .c-modal__title {
  display: block;
  width: 100%;
  font-size: 2.6rem;
  text-align: center;
}

/* Slightly larger close button with a green border, matching the original. */
.c-modal__close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2.6rem;
  height: 2.6rem;
  font-size: 1.6rem;
  cursor: pointer;
  border: 0.1rem solid var(--color-good, #1bbb36);
  border-radius: var(--var-border-radius, 0.3rem);
}

.c-modal__close-btn:hover {
  color: #000;
  background: var(--color-good, #1bbb36);
}

/* The body scrolls (long content like credits) while the title above stays
   pinned. Short modals never reach the max-height, so no scrollbar shows. */
.l-modal-text__content {
  margin-top: 1.5rem;
  max-height: 60vh;
  overflow-y: auto;
  line-height: 1.6;
}

.l-modal-text__content :deep(p) {
  margin: 1.5rem 0;
}

.l-modal-text__content :deep(a) {
  color: var(--color-accent, #5f9948);
}

.l-modal-text__content :deep(h3) {
  margin: 1.2rem 0 0.4rem;
  font-size: 1.5rem;
}

.l-modal-text__content :deep(ul) {
  margin: 0.3rem 0;
  padding-left: 2rem;
}
</style>
