import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import "./app-shell.css";
import initFormat from "./wasm/ad_format.js";

// Load the `ad-format` WASM module before mounting so the synchronous
// `formatDecimal` helper (util/format.js) is ready for the first render.
initFormat().then(() => {
  createApp(App).use(createPinia()).mount("#app");

  // Dismiss the startup splash (#loading, defined in index.html) once the app
  // has mounted. The original game hides it 500ms after window.onload; our load
  // is near-instant, so we hold for 1s to keep the splash visible, then fade
  // out via CSS instead of a hard cut. The element is removed after the
  // transition so it never intercepts pointer events.
  const splash = document.getElementById("loading");
  if (splash) {
    setTimeout(() => {
      splash.classList.add("is-hidden");
      splash.addEventListener("transitionend", () => splash.remove(), { once: true });
    }, 1000);
  }
});
