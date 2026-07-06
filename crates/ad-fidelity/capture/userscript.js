// ==UserScript==
// @name         AD Fidelity Capture
// @namespace    ad-fidelity
// @version      0.1
// @description  Speed controls + time-based savefile capture for the AD fidelity harness
// @match        http://localhost:8080/*
// @match        https://ivark.github.io/*
// @match        https://*.antimatterdimensions.com/*
// @grant        none
// @run-at       document-idle
// ==/UserScript==

// Injected into the *original* Antimatter Dimensions game (run it locally, e.g.
// `npm run serve` in ../antimatter-dimensions, then load this via Tampermonkey/
// Violentmonkey). Adds a small panel with:
//   - speed buttons (1×/5×/25×/100×/1000×): run the game loop faster, in the
//     game's normal `updateRate`-sized steps (no giant single ticks);
//   - time-based capture: exports the savefile on a *game-time* cadence and
//     POSTs it to the local save-server (save-server.js).
// Event-driven capture is deferred to phase 2 (see the design doc).

(function () {
  "use strict";

  // ---- config ----
  const SERVER_URL = "http://localhost:8899";
  const CAPTURE_GAME_MS = 60_000; // capture every 60 s of *game* time
  const POLL_MS = 500; // how often (real time) to check the game-time cadence
  const SPEEDS = [1, 5, 25, 100, 1000];

  // ---- state ----
  let speed = 1;
  let fastTimer = null;
  let capturing = false;
  let nextCaptureAt = null;
  let captureCount = 0;

  function ready() {
    return (
      window.player &&
      window.GameStorage &&
      typeof window.GameStorage.exportModifiedSave === "function" &&
      typeof window.gameLoop === "function" &&
      window.GameIntervals &&
      window.GameIntervals.gameLoop
    );
  }

  // Speed control. The game's own gameLoop interval calls a module-local
  // `gameLoop` (not `window.gameLoop`), so we cannot wrap it — instead we stop
  // that interval and drive `window.gameLoop(step)` m times per real tick. At
  // 1× we simply restore the game's own loop.
  function setSpeed(m) {
    speed = m;
    if (fastTimer) {
      clearInterval(fastTimer);
      fastTimer = null;
    }
    const gi = window.GameIntervals;
    if (m <= 1) {
      if (gi.gameLoop && !gi.gameLoop.isStarted) gi.gameLoop.start();
    } else {
      if (gi.gameLoop && gi.gameLoop.isStarted) gi.gameLoop.stop();
      const step = (window.player.options && window.player.options.updateRate) || 33;
      fastTimer = setInterval(() => {
        for (let i = 0; i < m; i++) window.gameLoop(step);
      }, step);
    }
    render();
  }

  function capture(tag) {
    let save;
    try {
      save = window.GameStorage.exportModifiedSave();
    } catch (e) {
      console.warn("[ad-fidelity] export failed", e);
      return;
    }
    const body = JSON.stringify({
      tag: tag || "timed",
      wall: Date.now(),
      save,
      meta: { gameTime: window.player.records.totalTimePlayed, speed },
    });
    fetch(SERVER_URL + "/save", {
      method: "POST",
      keepalive: true,
      headers: { "Content-Type": "application/json" },
      body,
    })
      .then(() => {
        captureCount++;
        render();
      })
      .catch((e) => console.warn("[ad-fidelity] POST failed", e));
  }

  // Game-time cadence: poll in real time, capture whenever game time has
  // advanced past the next multiple of CAPTURE_GAME_MS. Robust to speed changes
  // (fast-forward simply crosses more thresholds per poll).
  setInterval(() => {
    if (!capturing || !ready()) return;
    const gt = window.player.records.totalTimePlayed;
    if (nextCaptureAt === null) nextCaptureAt = gt; // capture immediately on enable
    if (gt >= nextCaptureAt) {
      capture("timed");
      do {
        nextCaptureAt += CAPTURE_GAME_MS;
      } while (gt >= nextCaptureAt);
    }
  }, POLL_MS);

  // ---- UI ----
  let panel;
  function render() {
    if (!panel) return;
    panel.querySelector("#adf-status").textContent =
      `speed ${speed}× · capture ${capturing ? "ON" : "off"} · ${captureCount} saved`;
    panel.querySelectorAll("[data-speed]").forEach((b) => {
      b.style.fontWeight = Number(b.dataset.speed) === speed ? "bold" : "normal";
    });
    const t = panel.querySelector("#adf-toggle");
    if (t) t.textContent = capturing ? "Stop capture" : "Start capture";
  }

  function buildPanel() {
    panel = document.createElement("div");
    panel.style.cssText =
      "position:fixed;bottom:8px;right:8px;z-index:99999;background:#111;color:#ddd;" +
      "font:12px monospace;padding:8px;border:1px solid #444;border-radius:6px;opacity:.9";
    const speeds = SPEEDS.map(
      (s) => `<button data-speed="${s}" style="margin:0 2px">${s}×</button>`
    ).join("");
    panel.innerHTML =
      `<div style="margin-bottom:4px">AD Fidelity Capture</div>` +
      `<div style="margin-bottom:4px">${speeds}</div>` +
      `<div style="margin-bottom:4px">` +
      `<button id="adf-toggle">Start capture</button> ` +
      `<button id="adf-now">Save now</button></div>` +
      `<div id="adf-status"></div>`;
    document.body.appendChild(panel);
    panel
      .querySelectorAll("[data-speed]")
      .forEach((b) =>
        b.addEventListener("click", () => setSpeed(Number(b.dataset.speed)))
      );
    panel.querySelector("#adf-toggle").addEventListener("click", () => {
      capturing = !capturing;
      nextCaptureAt = null;
      render();
    });
    panel.querySelector("#adf-now").addEventListener("click", () => capture("manual"));
    render();
  }

  const boot = setInterval(() => {
    if (!ready() || !document.body) return;
    clearInterval(boot);
    buildPanel();
    console.log("[ad-fidelity] capture userscript ready");
  }, 500);
})();
