#!/usr/bin/env node
"use strict";

// Local save-capture server for the fidelity capture rig.
//
// Receives savefiles POSTed by the in-browser userscript (userscript.js) and
// writes them, sequenced, to a directory. Pairs with the time-based capture
// described in docs/design/2026-07-06-fidelity-testing.md §4 ("Capture rig").
//
//   node save-server.js [port] [outDir]
//   PORT=8899 CAPTURE_DIR=../saves/captures node save-server.js
//
// POST /save  body: {"tag": "...", "wall": <ms>, "save": "<savefile>", "meta": {...}}
//   -> writes NNNNN-HHHH-MM-SS-<tag>.txt containing the savefile, and appends a
//      metadata line to index.jsonl. The sequence continues across restarts.

const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = Number(process.env.PORT || process.argv[2] || 8899);
const OUT_DIR = path.resolve(
  process.env.CAPTURE_DIR ||
    process.argv[3] ||
    path.join(__dirname, "..", "saves", "captures")
);

fs.mkdirSync(OUT_DIR, { recursive: true });

// Continue the sequence across restarts: next index = (max existing) + 1.
function initialSeq() {
  let max = 0;
  for (const f of fs.readdirSync(OUT_DIR)) {
    const m = /^(\d+)-/.exec(f);
    if (m) max = Math.max(max, Number(m[1]));
  }
  return max + 1;
}
let seq = initialSeq();

function sanitizeTag(tag) {
  return String(tag || "capture")
    .replace(/[^a-zA-Z0-9._-]/g, "-")
    .slice(0, 60);
}

// Game time elapsed (ms) -> "HHHH-MM-SS" (hours zero-padded to 4 digits; wider
// if a run exceeds 9999 h).
function formatGameTime(ms) {
  const totalSec = Math.floor((Number(ms) || 0) / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  return (
    `${String(h).padStart(4, "0")}-` +
    `${String(m).padStart(2, "0")}-` +
    `${String(s).padStart(2, "0")}`
  );
}

function writeCapture({ tag, save, wall, meta }) {
  const idx = String(seq++).padStart(5, "0");
  const hms = formatGameTime(meta && meta.gameTime);
  const name = `${idx}-${hms}-${sanitizeTag(tag)}.txt`;
  fs.writeFileSync(path.join(OUT_DIR, name), save);
  const record = {
    file: name,
    tag: sanitizeTag(tag),
    wall: wall || Date.now(),
    bytes: save.length,
    ...(meta ? { meta } : {}),
  };
  fs.appendFileSync(
    path.join(OUT_DIR, "index.jsonl"),
    JSON.stringify(record) + "\n"
  );
  return name;
}

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "POST, GET, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
};

const server = http.createServer((req, res) => {
  if (req.method === "OPTIONS") {
    res.writeHead(204, CORS);
    return res.end();
  }

  if (req.method === "GET" && req.url === "/") {
    res.writeHead(200, { "Content-Type": "text/plain", ...CORS });
    return res.end(
      `ad-fidelity capture server\nout: ${OUT_DIR}\nnext seq: ${seq}\n`
    );
  }

  if (req.method === "POST" && (req.url === "/save" || req.url === "/")) {
    let body = "";
    req.on("data", (c) => {
      body += c;
      if (body.length > 50 * 1024 * 1024) req.destroy(); // 50 MB guard
    });
    req.on("end", () => {
      try {
        const payload = JSON.parse(body);
        if (!payload.save || typeof payload.save !== "string") {
          throw new Error("missing 'save' string");
        }
        const name = writeCapture(payload);
        console.log(
          `[capture] ${name} (${payload.save.length} bytes, tag=${payload.tag || "?"})`
        );
        res.writeHead(200, { "Content-Type": "application/json", ...CORS });
        res.end(JSON.stringify({ ok: true, file: name }));
      } catch (e) {
        res.writeHead(400, { "Content-Type": "application/json", ...CORS });
        res.end(JSON.stringify({ ok: false, error: String(e.message || e) }));
      }
    });
    return;
  }

  res.writeHead(404, CORS);
  res.end();
});

server.listen(PORT, () => {
  console.log(`ad-fidelity capture server listening on http://localhost:${PORT}`);
  console.log(`writing captures to ${OUT_DIR} (next #${seq})`);
});
