import { ref } from "vue";

// Mirrors the original GenericDimensionRowText: dimension row text
// stacks vertically ("narrow") below this width, and sits on one row
// ("wide") above it. One shared listener feeds all rows.
const NARROW_BELOW = 1573;

export const isSmall = ref(window.innerWidth < NARROW_BELOW);

window.addEventListener("resize", () => {
  isSmall.value = window.innerWidth < NARROW_BELOW;
});
