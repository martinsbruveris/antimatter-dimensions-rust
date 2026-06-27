<script setup>
// A minimal, visually faithful reimplementation of the original
// `SliderComponent` (a vue-slider-component port) for the single-handle,
// horizontal, no-tooltip case the options tabs use. It renders the same DOM /
// class structure so the vendored `ad-slider-component.css` styles it
// identically (grey track, blue fill, white draggable dot), and supports
// click-to-set and drag. Values snap to `interval` and clamp to [min, max].
import { computed, onBeforeUnmount, ref } from "vue";

const props = defineProps({
  min: { type: Number, default: 0 },
  max: { type: Number, default: 100 },
  interval: { type: Number, default: 1 },
  modelValue: { type: Number, required: true },
  width: { type: String, default: "100%" },
});
const emit = defineEmits(["update:modelValue"]);

// Matches the original dot size (dotSize: "16px"); the usable travel of the
// dot centre is the track width minus this, so the dot never overflows.
const DOT_SIZE = 16;

const bg = ref(null);

const ratio = computed(() => {
  const span = props.max - props.min;
  if (span <= 0) return 0;
  return Math.min(1, Math.max(0, (props.modelValue - props.min) / span));
});

const dotStyle = computed(() => ({
  position: "absolute",
  top: "50%",
  width: `${DOT_SIZE}px`,
  height: `${DOT_SIZE}px`,
  left: `calc(${ratio.value} * (100% - ${DOT_SIZE}px))`,
  transform: "translateY(-50%)",
}));

const processStyle = computed(() => ({
  position: "absolute",
  top: "0",
  left: "0",
  height: "100%",
  width: `calc(${ratio.value} * (100% - ${DOT_SIZE}px) + ${DOT_SIZE / 2}px)`,
}));

function valueFromClientX(clientX) {
  const el = bg.value;
  if (!el) return props.modelValue;
  const rect = el.getBoundingClientRect();
  const usable = rect.width - DOT_SIZE;
  const x = clientX - rect.left - DOT_SIZE / 2;
  const r = usable > 0 ? Math.min(1, Math.max(0, x / usable)) : 0;
  const raw = props.min + r * (props.max - props.min);
  const snapped = Math.round(raw / props.interval) * props.interval;
  return Math.min(props.max, Math.max(props.min, snapped));
}

function emitFromClientX(clientX) {
  const v = valueFromClientX(clientX);
  if (v !== props.modelValue) emit("update:modelValue", v);
}

function onMouseMove(e) {
  emitFromClientX(e.clientX);
}
function onMouseUp() {
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", onMouseUp);
}
function onMouseDown(e) {
  emitFromClientX(e.clientX);
  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
  e.preventDefault();
}

onBeforeUnmount(() => {
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", onMouseUp);
});
</script>

<template>
  <div class="l-ad-slider l-ad-slider--horizontal">
    <div
      class="l-ad-slider__wrap"
      :style="{ width }"
    >
      <div
        ref="bg"
        class="l-ad-slider__bg c-ad-slider__bg"
        style="height: 6px; position: relative; cursor: pointer"
        @mousedown="onMouseDown"
      >
        <div
          class="l-ad-slider__process c-ad-slider__process"
          :style="processStyle"
        />
        <div
          class="l-ad-slider__dot c-ad-slider__dot"
          :style="dotStyle"
        >
          <div class="l-ad-slider__dot-handle c-ad-slider__dot-handle" />
        </div>
      </div>
    </div>
  </div>
</template>
