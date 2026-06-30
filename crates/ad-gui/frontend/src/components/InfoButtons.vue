<script setup>
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";

import { useGameStore } from "../stores/game";
import { useUiStore } from "../stores/ui";
import { emphasizeH2P } from "../util/tutorial";
import Modal from "./Modal.vue";
import H2PModal from "./H2PModal.vue";
import CreditsDisplay from "./CreditsDisplay.vue";

// Modal open/close state lives in the ui store so the keyboard shortcuts
// (e.g. H for "How to play") drive the same popups as these buttons.
const ui = useUiStore();
const game = useGameStore();

// Pulse the How-To-Play link until the first Dimension Boost (the original's
// Tutorial.emphasizeH2P): a gold glow overlay + "Click for info" tooltip.
// Suppressed while `h2pEmphasisShown` is set (currently always, so it doesn't
// overlay the dev controls — see the ui store flag).
const emphasizeHelp = computed(
  () => !ui.h2pEmphasisShown && emphasizeH2P(game.snapshot)
);

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
    <div class="h2p-wrapper">
      <div
        class="o-questionmark"
        title="How to play"
        @click="show('help')"
      >
        ?
      </div>
      <!-- Tutorial emphasis: pulsing gold overlay + tooltip on the "?" link
           until the first Dimension Boost (mirrors HowToPlay.vue). -->
      <div
        v-if="emphasizeHelp"
        class="h2p-tutorial--glow"
      />
      <div
        v-if="emphasizeHelp"
        class="h2p-tooltip"
      >
        Click for info
      </div>
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

/* Anchor for the H2P tutorial overlay/tooltip, so they position over the
   "?" button. */
.h2p-wrapper {
  position: relative;
}

/* Pulsing gold overlay covering the "?" button while the tutorial emphasises
   it (mirrors HowToPlay.vue's .h2p-tutorial--glow; reuses the vendored
   a-opacity keyframe). */
.h2p-tutorial--glow {
  position: absolute;
  top: 0;
  left: 0;
  width: 2.2rem;
  height: 2.2rem;
  background: gold;
  animation: a-opacity 3s infinite;
  pointer-events: none;
  z-index: 2;
}

/* "Click for info" tooltip to the left of the button. */
.h2p-tooltip {
  position: absolute;
  top: 0;
  right: 100%;
  width: fit-content;
  white-space: nowrap;
  color: #fff;
  background: #000;
  border: 0.1rem solid var(--color-text, #cccccc);
  border-radius: 0.5rem;
  transform: translate(-0.7rem, -0.4rem);
  padding: 0.2rem 0.4rem;
  z-index: 3;
}

.h2p-tooltip::after {
  content: "";
  position: absolute;
  top: 0.6rem;
  left: 100%;
  border-top: 0.5rem solid transparent;
  border-left: 0.5rem solid var(--color-text, #cccccc);
  border-bottom: 0.5rem solid transparent;
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
