<script setup>
import { openUrl } from "@tauri-apps/plugin-opener";

import { useUiStore } from "../stores/ui";
import Modal from "./Modal.vue";
import H2PModal from "./H2PModal.vue";
import CreditsDisplay from "./CreditsDisplay.vue";

// Modal open/close state lives in the ui store so the keyboard shortcuts
// (e.g. H for "How to play") drive the same popups as these buttons.
const ui = useUiStore();

// Open a URL in the system default browser. Uses the Tauri opener plugin
// when running in the app; falls back to window.open in a plain browser
// (e.g. `npm run dev`), where the plugin's IPC is unavailable.
async function openLink(url) {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener");
  }
}

// External links shown in the "About the game" modal as icons with a
// tooltip above each one.
const links = [
  {
    icon: "fab fa-github",
    label: "GitHub repository",
    href: "https://github.com/IvarK/AntimatterDimensionsSourceCode",
  },
  {
    icon: "fab fa-reddit-alien",
    label: "r/AntimatterDimensions",
    href: "https://www.reddit.com/r/AntimatterDimensions/",
  },
  {
    icon: "fab fa-discord",
    label: "Antimatter Dimensions Discord Server",
    href: "https://discord.gg/ST9NaXa",
  },
  {
    icon: "fab fa-google-play",
    label: "Antimatter Dimensions on Google Play",
    href: "https://play.google.com/store/apps/details?id=kajfosz.antimatterdimensions",
  },
  {
    icon: "fab fa-steam",
    label: "Antimatter Dimensions on Steam",
    href: "https://store.steampowered.com/app/1399720/Antimatter_Dimensions/",
  },
];

function show(name) {
  ui.showModal(name);
}

function close() {
  ui.closeModal();
}
</script>

<template>
  <div class="l-info-buttons">
    <div
      class="o-questionmark"
      title="How to play"
      @click="show('help')"
    >
      ?
    </div>
    <div
      class="o-questionmark"
      title="Information"
      @click="show('info')"
    >
      i
    </div>
  </div>

  <H2PModal v-if="ui.openModal === 'help'" />

  <Modal
    v-if="ui.openModal === 'info'"
    title="About the game"
    @close="close"
  >
    <p>
      Antimatter Dimensions is an Idle Incremental game created by Finnish
      developer Hevipelle. Originating as a solo project in 2016, it was
      expanded upon by a large team of developers and testers from then on. The
      game has unfolding gameplay and multiple prestige layers. 
    </p>
    <p>
      The "How to Play" button contains useful information about progressing.
    </p>
    <p>This is a Rust reimplementation of the game.</p>
    <div class="l-info-links">
      <span
        v-for="link in links"
        :key="link.href"
        class="info-link-wrapper"
      >
        <a
          class="info-link"
          :href="link.href"
          @click.prevent="openLink(link.href)"
        >
          <i :class="link.icon" />
        </a>
        <span
          class="c-tooltip-content c-tooltip-content--dark c-tooltip--top info-tooltip"
        >{{ link.label }}</span>
        <span
          class="c-tooltip-arrow c-tooltip-arrow--dark c-tooltip--top info-tooltip"
        />
      </span>
      <!-- Credits opens a modal rather than an external link, matching the
           original's InformationModalButton with show-modal="credits". -->
      <span class="info-link-wrapper">
        <a
          class="info-link"
          href="#"
          @click.prevent="show('credits')"
        >
          <i class="fas fa-users" />
        </a>
        <span
          class="c-tooltip-content c-tooltip-content--dark c-tooltip--top info-tooltip"
        >Credits</span>
        <span
          class="c-tooltip-arrow c-tooltip-arrow--dark c-tooltip--top info-tooltip"
        />
      </span>
    </div>
  </Modal>

  <Modal
    v-if="ui.openModal === 'credits'"
    title="Antimatter Dimensions"
    title-class="c-game-header__antimatter"
    @close="close"
  >
    <CreditsDisplay />
  </Modal>
</template>

<style scoped>
.l-info-buttons {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* Slightly larger than the vendored default, matching the JS version's
   red-bordered corner buttons. */
.o-questionmark {
  width: 2.2rem;
  height: 2.2rem;
  font-size: 1.5rem;
  cursor: pointer;
  color: var(--color-text, #cccccc);
  border-color: var(--color-bad, #e02e2e);
  background: transparent;
}

.o-questionmark:hover {
  color: #000;
  background: var(--color-bad, #e02e2e);
}

/* Row of link icons in the "About the game" modal, spread evenly across
   the full width like the original's .l-socials. */
.l-info-links {
  display: flex;
  flex-direction: row;
  justify-content: space-evenly;
  align-items: center;
  margin-top: 3rem;
}

.info-link-wrapper {
  position: relative;
  display: inline-flex;
}

/* Compound selector to win over the modal's generic `a` colour rule
   (including the global `a:hover`), so the icons stay white like the
   original. Icon size matches the original's .l-socials (7.5rem). */
.l-info-links .info-link,
.l-info-links .info-link-wrapper:hover .info-link {
  font-size: 7.5rem;
  color: #ffffff;
}

/* Tooltip anchored above the icon, shown on hover. */
.info-tooltip.c-tooltip-content {
  left: 50%;
  bottom: calc(100% + 0.8rem);
  width: max-content;
  max-width: 20rem;
  transform: translateX(-50%);
}

.info-tooltip.c-tooltip-arrow {
  left: 50%;
  bottom: calc(100% + 0.25rem);
  transform: translateX(-50%);
}

.info-link-wrapper:hover .info-tooltip {
  visibility: visible;
  opacity: 1;
}
</style>
