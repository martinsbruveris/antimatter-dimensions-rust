<script setup>
// A hover tooltip matching the original game's `v-tooltip` directive: the
// vendored `general-tooltip` styling (Typewriter font, dark box, arrow) placed
// above the trigger (v-tooltip's default "top" placement). We don't pull in the
// v-tooltip/popper library — positioning is a small bit of CSS instead — but the
// box/arrow styling is the game's own (public/stylesheets/tooltips.css).
defineProps({
  // Tooltip text shown on hover.
  text: { type: String, required: true },
});
</script>

<template>
  <div class="l-tooltip-host">
    <slot />
    <div
      class="general-tooltip"
      x-placement="top"
      role="tooltip"
    >
      <div class="tooltip-arrow" />
      <div class="tooltip-inner">{{ text }}</div>
    </div>
  </div>
</template>

<style scoped>
/* Shrink-wrap the trigger and anchor the absolutely-positioned tooltip; using
   inline-flex keeps the host behaving like the button it wraps inside the
   surrounding flex rows. */
.l-tooltip-host {
  position: relative;
  display: inline-flex;
}

/* Centre the tooltip above the trigger (matching v-tooltip's "top" placement);
   the vendored `[x-placement^="top"]` rules style the downward arrow. Hidden
   until the host is hovered. */
.l-tooltip-host > .general-tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  visibility: hidden;
  opacity: 0;
  transition: opacity 0.3s, visibility 0.3s;
}

.l-tooltip-host:hover > .general-tooltip {
  visibility: visible;
  opacity: 1;
}
</style>
