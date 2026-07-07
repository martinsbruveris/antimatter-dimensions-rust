#!/usr/bin/env node
"use strict";

// Node oracle for the save-replay fidelity harness.
//
// Boots the *real* Antimatter Dimensions game in headless Chromium (Playwright),
// deterministically ticks each input save forward, and writes reference
// "expected after N ticks" fixtures that the Rust replay harness diffs against.
// See docs/design/2026-07-06-fidelity-testing.md §6 (architecture) and §10
// (runtime + determinism controls).
//
//   GAME_URL=http://localhost:8080 node generate-replay-fixtures.js
//
// Prerequisite: the original game must be reachable at GAME_URL (e.g. run
// `npm run serve` in ../../../../antimatter-dimensions, or serve a built dist).

const fs = require("fs");
const path = require("path");
const { chromium } = require("playwright");

const GAME_URL = process.env.GAME_URL || "http://localhost:8080";
const SAVES_DIR = path.resolve(
  process.env.SAVES_DIR || path.join(__dirname, "..", "saves", "captures")
);
const OUT_DIR = path.resolve(
  process.env.OUT_DIR || path.join(__dirname, "..", "saves", "fixtures")
);
const TICK_MS = Number(process.env.TICK_MS || 50); // §10: 50 ms granularity (parameter)
const HORIZONS = (process.env.HORIZONS || "1,10,100,1000").split(",").map(Number);

// A normal desktop Chrome UA so the game's browserCheck() passes and init()
// runs (headless Chrome's default "HeadlessChrome" UA would fail the regex).
const USER_AGENT =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 " +
  "(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

// Injected before any game script runs: a deterministic clock + PRNG so the
// oracle is reproducible. Both are resettable per save (below). Replicanti
// samplers are mocked after load, inside replayInPage.
function determinismInit() {
  const BASE = 1704067200000; // 2024-01-01T00:00:00Z, fixed
  let now = BASE;
  // eslint-disable-next-line no-global-assign
  Date.now = () => now;
  window.__adClock = {
    advance: (ms) => {
      now += ms;
    },
    reset: () => {
      now = BASE;
    },
    now: () => now,
  };

  // mulberry32 — deterministic Math.random so incidental draws are reproducible.
  const SEED = 0x9e3779b9 >>> 0;
  let s = SEED;
  window.__adReseed = () => {
    s = SEED;
  };
  // eslint-disable-next-line no-global-assign
  Math.random = () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// Runs in the page for one save: reset determinism, import (no offline), stop
// the loop, mock the replicanti samplers, then tick to each horizon and export.
function replayInPage(args) {
  const { save, horizons, tickMs } = args;
  const GS = window.GameStorage;

  window.__adClock.reset();
  window.__adReseed();

  GS.offlineEnabled = false; // disable offline catch-up on import
  GS.import(save);
  if (window.GameIntervals) window.GameIntervals.stop(); // import restarts it

  // Replicanti: compare in expectation mode — mock the samplers to their means
  // (design §7). Inert for pre-replicanti saves, but correct when it matters.
  window.poissonDistribution = (mu) => mu;
  window.binomialDistribution = (n, p) =>
    typeof n === "number" ? n * p : n.times(p);

  window.player.lastUpdate = window.__adClock.now();

  const sorted = [...horizons].sort((a, b) => a - b);
  const out = {};
  let ticked = 0;
  for (const h of sorted) {
    while (ticked < h) {
      window.__adClock.advance(tickMs);
      window.gameLoop(tickMs);
      ticked++;
    }
    out[String(h)] = GS.exportModifiedSave();
  }
  return out;
}

async function main() {
  if (!fs.existsSync(SAVES_DIR)) {
    console.error(`saves dir not found: ${SAVES_DIR}`);
    process.exit(1);
  }
  const saveFiles = fs
    .readdirSync(SAVES_DIR)
    .filter((f) => /\.txt$/.test(f))
    .sort();
  if (saveFiles.length === 0) {
    console.error(`no .txt saves in ${SAVES_DIR}`);
    process.exit(1);
  }
  fs.mkdirSync(OUT_DIR, { recursive: true });

  console.log(`launching headless Chromium; game at ${GAME_URL}`);
  const browser = await chromium.launch();
  const context = await browser.newContext({ userAgent: USER_AGENT });
  await context.addInitScript(determinismInit);
  const page = await context.newPage();
  page.on("pageerror", (e) => console.warn("[page error]", e.message));

  await page.goto(GAME_URL, { waitUntil: "load", timeout: 60000 });
  // Wait for the game to boot (merge-globals + init()).
  await page.waitForFunction(
    () =>
      window.player &&
      window.GameStorage &&
      typeof window.GameStorage.import === "function" &&
      typeof window.GameStorage.exportModifiedSave === "function" &&
      typeof window.gameLoop === "function",
    { timeout: 60000 }
  );
  await page.evaluate(() => {
    if (window.GameIntervals) window.GameIntervals.stop();
  });

  for (const file of saveFiles) {
    const input = fs.readFileSync(path.join(SAVES_DIR, file), "utf8").trim();
    process.stdout.write(`  ${file} … `);
    const expected = await page.evaluate(replayInPage, {
      save: input,
      horizons: HORIZONS,
      tickMs: TICK_MS,
    });
    const fixture = {
      meta: {
        sourceSave: file,
        tickMs: TICK_MS,
        horizons: HORIZONS,
        generatedAt: new Date().toISOString(),
        notes:
          "offline disabled; deterministic clock + Math.random; " +
          "replicanti samplers mocked to their means (design §7)",
      },
      input,
      expected,
    };
    const outName = file.replace(/\.txt$/, ".json");
    fs.writeFileSync(
      path.join(OUT_DIR, outName),
      JSON.stringify(fixture, null, 2) + "\n"
    );
    console.log(`ok -> ${outName}`);
  }

  await browser.close();
  console.log(`done. fixtures in ${OUT_DIR}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
