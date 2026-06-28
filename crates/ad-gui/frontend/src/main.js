import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import "./app-shell.css";
import initFormat from "./wasm/ad_format.js";

// Load the `ad-format` WASM module before mounting so the synchronous
// `formatDecimal` helper (util/format.js) is ready for the first render.
initFormat().then(() => {
  createApp(App).use(createPinia()).mount("#app");
});
