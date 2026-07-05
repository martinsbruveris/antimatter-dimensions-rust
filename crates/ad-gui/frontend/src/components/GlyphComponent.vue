<script setup>
// A single glyph square (border = type color, symbol tinted by rarity, one
// dot per effect) with the hover tooltip. Faithful reduction of the
// original's GlyphComponent.vue: the gradient rarity borders, drag-and-drop,
// and cosmetics are omitted; interaction is click / shift-click (emitted).
import { computed, ref } from "vue";

import {
  COMPANION_TEXT,
  GLYPH_EFFECTS,
  GLYPH_TYPES,
  rarityOf,
  strengthToRarityPercent,
} from "../data/glyphs";

const props = defineProps({
  glyph: { type: Object, required: true },
  size: { type: String, default: "5rem" },
  circular: { type: Boolean, default: false },
  showSacrifice: { type: Boolean, default: false },
  tooltipAbove: { type: Boolean, default: false },
});
const emit = defineEmits(["clicked", "shiftClicked", "ctrlShiftClicked"]);

const showTooltip = ref(false);

const typeInfo = computed(() => GLYPH_TYPES[props.glyph.kind] ?? GLYPH_TYPES.power);
const isCompanion = computed(() => props.glyph.kind === "companion");
const rarity = computed(() => rarityOf(props.glyph.strength));
const rarityPercent = computed(() =>
  strengthToRarityPercent(props.glyph.strength).toFixed(1)
);

const borderColor = computed(() => typeInfo.value.color);
const symbolColor = computed(() =>
  isCompanion.value ? typeInfo.value.color : rarity.value.color
);

const outerStyle = computed(() => ({
  width: props.size,
  height: props.size,
  "background-color": borderColor.value,
  "box-shadow": `0 0 1rem 0.2rem ${borderColor.value}`,
  "border-radius": props.circular ? "50%" : "0",
}));
const innerStyle = computed(() => ({
  width: `calc(${props.size} - 0.2rem)`,
  height: `calc(${props.size} - 0.2rem)`,
  "font-size": `calc(${props.size} * 0.5)`,
  color: symbolColor.value,
  "text-shadow": `-0.04em 0.04em 0.08em ${symbolColor.value}`,
  "border-radius": props.circular ? "50%" : "0",
  "padding-bottom": "0.3rem",
}));

// One dot per effect, placed clockwise from the bottom left (the original's
// effectIconPos, 90° apart starting at half-step).
const effectDots = computed(() => {
  if (isCompanion.value) return [];
  const minBit = { time: 0, dilation: 4, replication: 8, infinity: 12, power: 16 }[
    props.glyph.kind
  ];
  const dots = [];
  const scale = 0.28 * parseFloat(props.size);
  let mask = props.glyph.effects >> minBit;
  for (let id = 0; mask > 0; id++, mask >>= 1) {
    if ((mask & 1) === 0) continue;
    const angle = (Math.PI / 2) * (id + 0.5);
    dots.push({
      dx: -scale * Math.sin(angle) - 0.045,
      dy: scale * (Math.cos(angle) + 0.15) - 0.045,
    });
  }
  return dots;
});

const tooltipHeader = computed(() =>
  isCompanion.value
    ? "Companion Glyph"
    : `${rarity.value.name} glyph of ${props.glyph.kind}`
);
const effectLines = computed(() => {
  if (isCompanion.value) return COMPANION_TEXT;
  return props.glyph.effect_values.map((ev) =>
    GLYPH_EFFECTS[ev.bit] ? GLYPH_EFFECTS[ev.bit].single(ev.value) : ""
  );
});

function onClick(event) {
  if (event.shiftKey && (event.ctrlKey || event.metaKey)) {
    emit("ctrlShiftClicked", props.glyph.id);
  } else if (event.shiftKey) {
    emit("shiftClicked", props.glyph.id);
  } else {
    emit("clicked", props.glyph.id);
  }
}
</script>

<template>
  <div
    :style="outerStyle"
    class="l-glyph-component"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
    @click="onClick"
  >
    <div
      :style="innerStyle"
      class="l-glyph-component c-glyph-component"
    >
      {{ typeInfo.symbol }}
      <div
        v-for="(dot, i) in effectDots"
        :key="i"
        class="o-glyph-effect-dot"
        :style="{
          background: symbolColor,
          transform: `translate(${dot.dx}rem, ${dot.dy}rem)`,
        }"
      />
    </div>
    <div
      v-if="showTooltip"
      class="c-glyph-tooltip"
      :class="tooltipAbove ? 'c-glyph-tooltip--above' : 'c-glyph-tooltip--below'"
    >
      <div
        class="c-glyph-tooltip__header"
        :style="{ color: symbolColor }"
      >
        {{ tooltipHeader }}
      </div>
      <div class="c-glyph-tooltip__info">
        <span v-if="!isCompanion">Level: {{ glyph.level }} | Rarity: {{ rarityPercent }}%</span>
      </div>
      <div
        v-for="(line, i) in effectLines"
        :key="i"
        class="c-glyph-tooltip__effect"
      >
        {{ line }}
      </div>
      <div
        v-if="showSacrifice && !isCompanion"
        class="c-glyph-tooltip__sacrifice"
      >
        Can be sacrificed for {{ glyph.sacrifice_value.toExponential(2) }} power
      </div>
    </div>
  </div>
</template>

<style scoped>
.l-glyph-component {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  user-select: none;
}

.o-glyph-effect-dot {
  position: absolute;
  width: 0.3rem;
  height: 0.3rem;
  border-radius: 50%;
  opacity: 0.8;
}

/* Simplified fixed-position tooltip (the original tracks the mouse). */
.c-glyph-tooltip {
  position: absolute;
  left: 50%;
  z-index: 7;
  width: 22rem;
  transform: translateX(-50%);
  text-align: center;
  font-family: Typewriter, serif;
  font-size: 1.2rem;
  background: rgba(0, 0, 0, 0.9);
  color: white;
  border: 0.1rem solid var(--color-text);
  border-radius: var(--var-border-radius, 0.5rem);
  padding: 0.5rem;
  pointer-events: none;
}

.c-glyph-tooltip--below {
  top: calc(100% + 0.6rem);
}

.c-glyph-tooltip--above {
  bottom: calc(100% + 0.6rem);
}

.c-glyph-tooltip__header {
  font-weight: bold;
}

.c-glyph-tooltip__info {
  margin-bottom: 0.3rem;
}

.c-glyph-tooltip__effect {
  color: #76ee76;
}

.c-glyph-tooltip__sacrifice {
  margin-top: 0.3rem;
  color: #ff6666;
}
</style>
