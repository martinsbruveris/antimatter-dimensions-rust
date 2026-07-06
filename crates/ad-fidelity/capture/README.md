# Capture rig

The capture side of the save-replay fidelity harness (design:
[`docs/design/2026-07-06-fidelity-testing.md`](../../../docs/design/2026-07-06-fidelity-testing.md)
§4). It records real savefiles from a manual playthrough of the **original** JS
game, to be replayed later by the [oracle](../oracle) and the Rust harness.

Two pieces:

- **`userscript.js`** — injected into the running original game. Adds a small
  panel with **speed buttons** (fast-forward the manual playthrough) and
  **time-based capture** (export the savefile on a *game-time* cadence and POST
  it to the local server). Event-driven capture is deferred to phase 2.
- **`save-server.js`** — a tiny local HTTP server that receives the POSTed saves
  and writes them, sequenced, to a directory (plus an `index.jsonl` of metadata).

```
 original game (localhost:8080)          this crate
 ┌───────────────────────────┐          ┌──────────────────────┐
 │ userscript.js panel        │  POST    │ save-server.js        │
 │  • speed buttons           │ ───────▶ │  → captures/NNNNN-*.txt│
 │  • time-based capture      │  /save   │  → captures/index.jsonl│
 └───────────────────────────┘          └──────────────────────┘
```

## Prerequisites

- **Node.js 18+** (for the save server).
- **The original game** checked out at `../../../../antimatter-dimensions`
  (sibling of this workspace) with `npm install` run.
- A **userscript manager** in your browser: Tampermonkey or Violentmonkey
  (Chrome/Firefox/Edge).

## Usage

### 1. Serve the original game

```bash
cd ../../../../antimatter-dimensions   # the original JS game
npm install                            # once
npm run serve                          # dev server at http://localhost:8080
```

Leave it running. (Any host/port works — see *Serving at a different URL* below.)

### 2. Start the save server

```bash
cd crates/ad-fidelity/capture
npm run server                         # writes to ./captures, listens on :8899
# or, explicitly:  node save-server.js <port> <outDir>
# or, via env:     PORT=8899 CAPTURE_DIR=captures node save-server.js
```

It prints the output directory and the next sequence number, and logs each
capture. `captures/` is created if missing and is git-ignored.

### 3. Install the userscript

1. Open your userscript manager → **Create a new script**.
2. Replace the template with the entire contents of
   [`userscript.js`](userscript.js) and save.
3. Confirm it is **enabled** and its `@match` covers your game URL (it ships with
   `http://localhost:8080/*`).

### 4. Play and capture

Open <http://localhost:8080>. A panel appears bottom-right:

```
AD Fidelity Capture
[1×] [5×] [25×] [100×] [1000×]
[Start capture]  [Save now]
speed 1× · capture off · 0 saved
```

- **Speed buttons** — fast-forward the game (see *How speed works* below). The
  active speed is shown in bold.
- **Start / Stop capture** — toggles the time-based cadence. On start, it
  captures immediately, then every `CAPTURE_GAME_MS` of **game** time.
- **Save now** — capture a single savefile on demand (tagged `manual`).
- **Status line** — current speed, whether capture is on, and the running count.

Play the game normally, pick a speed your machine can sustain, and click
**Start capture**. Savefiles land in `captures/` as `NNNNN-HHH-MM-SS-timed.txt`,
where `HHH-MM-SS` is the game time elapsed at capture; on-demand saves are
`NNNNN-HHH-MM-SS-manual.txt`. Metadata for every capture is appended to
`captures/index.jsonl`.

## Configuration

### Userscript (`userscript.js`)

Edit the constants at the top of the file:

| Constant | Default | Meaning |
|----------|---------|---------|
| `SERVER_URL` | `http://localhost:8899` | Where captures are POSTed. Must match the save server. |
| `CAPTURE_GAME_MS` | `60_000` | Game-time between captures (ms). Lower = denser sampling. |
| `POLL_MS` | `500` | How often (real time) the cadence is checked. |
| `SPEEDS` | `[1, 5, 25, 100, 1000]` | Speed-button multipliers. |

The `@match` lines in the `// ==UserScript==` header control which pages the
script runs on. It ships matching `http://localhost:8080/*`,
`https://ivark.github.io/*`, and `https://*.antimatterdimensions.com/*`.

### Save server (`save-server.js`)

| Setting | Env var | Positional arg | Default |
|---------|---------|----------------|---------|
| Port | `PORT` | 1st | `8899` |
| Output dir | `CAPTURE_DIR` | 2nd | `./captures` |

Examples: `PORT=9000 node save-server.js`, or `node save-server.js 9000 /tmp/caps`.

### Serving the game at a different URL

If you serve the game somewhere other than `http://localhost:8080` (a different
port, or a built `dist/` served statically):

1. Add a matching `@match` line to the userscript header (e.g.
   `// @match http://localhost:5000/*`).
2. Leave `SERVER_URL` pointing at the save server — the server sends
   `Access-Control-Allow-Origin: *`, so cross-origin POSTs work from any game
   origin.

## Output format

- `captures/NNNNN-HHH-MM-SS-<tag>.txt` — the raw savefile string, exactly as the
  game's export produces it (importable back into the game, and decodable by the
  Rust save codec). `NNNNN` is the capture sequence; `HHH-MM-SS` is the game time
  elapsed at capture (hours-minutes-seconds).
- `captures/index.jsonl` — one JSON object per capture:

  ```json
  {"file":"00007-001-00-00-timed.txt","tag":"timed","wall":1751799600000,
   "bytes":7043,"meta":{"gameTime":3600000,"speed":25}}
  ```

  `wall` is the real-time epoch (ms), `bytes` the save length, `meta.gameTime` is
  `records.totalTimePlayed` at capture, `meta.speed` the active multiplier.

## Feeding captures into the oracle

The [oracle](../oracle) reads `*.txt` savefiles from a directory (`SAVES_DIR`,
default the repo `saves/`). To turn a capture run into fixtures, pick a
representative subset (the curated set) and point the oracle at it:

```bash
cd ../oracle
SAVES_DIR=../capture/captures npm run generate   # or copy chosen files into saves/
```

Use `index.jsonl` (game time / magnitudes) to choose saves spread across the
game's progression rather than clustered where you spent real time.

## Troubleshooting

- **No panel appears** — the userscript waits for the game globals; if `init()`
  never ran (unsupported-browser check), the panel won't show. Check the console
  for `[ad-fidelity] capture userscript ready`, and that `@match` covers the URL.
- **Captures don't appear / `POST failed` in the console** — the save server
  isn't running, or `SERVER_URL` is wrong. Verify with
  `curl http://localhost:8899/`.
- **Browser lags at high speed** — 1000× runs ~1000 game ticks per real tick;
  drop to a speed your machine sustains. Speed affects *which* states you sample,
  not their validity.
- **Server rejects with `missing 'save' string`** — the POST body wasn't the
  expected JSON; only happens if something other than the userscript posts.

## Notes

- Speed control stops the game's own `gameLoop` interval and drives
  `window.gameLoop(updateRate)` *m* times per real tick — i.e. more ticks of the
  normal step size, not one giant tick. This keeps tick granularity faithful.
- The cadence is measured in **game time** (`records.totalTimePlayed`), so
  coverage stays even regardless of the chosen speed.
- `captures/` is git-ignored; the curated subset used by tests is selected from a
  capture run separately.
