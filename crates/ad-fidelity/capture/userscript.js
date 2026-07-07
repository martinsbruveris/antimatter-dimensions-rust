// ==UserScript==
// @name         AD Fidelity Capture
// @namespace    ad-fidelity
// @version      0.1
// @description  Speed controls + real-time savefile capture for the AD fidelity harness
// @match        http://localhost:8080/*
// @match        https://ivark.github.io/*
// @match        https://*.antimatterdimensions.com/*
// @grant        none
// @run-at       document-idle
// ==/UserScript==

// Injected into the *original* Antimatter Dimensions game (run it locally, e.g.
// `npm run serve` in ../antimatter-dimensions, then load this via Tampermonkey/
// Violentmonkey). Adds a small panel with:
//   - speed buttons (1×/10×/100×/1000×): run the game loop faster, in the
//     game's normal `updateRate`-sized steps (no giant single ticks);
//   - time-based capture: exports the savefile on a *real-time* cadence and
//     POSTs it to the local save-server (save-server.js).
// Event-driven capture is deferred to phase 2 (see the design doc).

(function () {
  "use strict";

  // ---- config ----
  const SERVER_URL = "http://localhost:8899";
  const CAPTURE_REAL_MS = 60_000; // capture every 60 s of *real* time
  const POLL_MS = 500; // how often (real time) to check the capture cadence
  const SPEEDS = [1, 10, 100, 1000];

  // ---- state ----
  let speed = 1;
  let fastTimer = null;
  let capturing = false;
  let nextCaptureWall = null; // wall-clock ms of the next scheduled capture
  let lastCaptureWall = null; // wall-clock ms of the last successful capture
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
        lastCaptureWall = Date.now();
        render();
      })
      .catch((e) => console.warn("[ad-fidelity] POST failed", e));
  }

  // Real-time cadence: poll and capture whenever wall-clock time has advanced
  // past the next multiple of CAPTURE_REAL_MS. Independent of game speed, so a
  // fast-forwarded run is sampled on the same real-time schedule.
  setInterval(() => {
    if (!capturing || !ready()) return;
    const now = Date.now();
    if (nextCaptureWall === null) nextCaptureWall = now; // capture immediately on enable
    if (now >= nextCaptureWall) {
      capture("timed");
      do {
        nextCaptureWall += CAPTURE_REAL_MS;
      } while (now >= nextCaptureWall);
    }
  }, POLL_MS);

  // Keep the "since save" counter ticking even between captures.
  setInterval(renderSince, 1000);

  // ---- UI ----
  let panel;

  // Real-time duration (ms) -> "M:SS" or "H:MM:SS".
  function formatDuration(ms) {
    const totalSec = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    const mm = String(m).padStart(2, "0");
    const ss = String(s).padStart(2, "0");
    return h > 0 ? `${h}:${mm}:${ss}` : `${m}:${ss}`;
  }

  // Live "time since last successful save" counter, next to the buttons.
  function renderSince() {
    if (!panel) return;
    const el = panel.querySelector("#adf-since");
    if (!el) return;
    el.textContent =
      lastCaptureWall === null
        ? "since save —"
        : `since save ${formatDuration(Date.now() - lastCaptureWall)}`;
  }

  function render() {
    if (!panel) return;
    
    panel.querySelector("#adf-status").textContent =
      `speed ${speed}× · capture ${capturing ? "ON" : "off"} · ${captureCount} saved`;
    panel.querySelectorAll("[data-speed]").forEach((b) => {
      b.style.fontWeight = Number(b.dataset.speed) === speed ? "bold" : "normal";
    });
    const t = panel.querySelector("#adf-toggle");
    if (t) t.textContent = capturing ? "Stop capture" : "Start capture";
    renderSince();
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
      `<span id="adf-since" style="margin-right:4px">since save —</span>` +
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
      nextCaptureWall = null;
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
