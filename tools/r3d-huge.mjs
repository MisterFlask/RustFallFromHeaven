#!/usr/bin/env node
// The largest case the HD-2D map draws, as a pass/fail check (CLAUDE.md, "Shader changes draw a
// Huge map fully revealed before they are done"): starts `ffh-play serve` on a new Huge game,
// opens the client with `?renderer=hd2d` in headless Chromium zoomed out, switches debug reveal on
// (the whole map and every sea, unit and city come at once) and requires, within the limit: the
// revealed map arrived in the page, the page answers, and one complete frame of it is drawn.
// Fails (exit 1) on a page that does not answer in time, a crashed tab, a page error or a
// console error.
//
//   node tools/r3d-huge.mjs [--bin FFH_PLAY] [--limit SECONDS] [--gpu] [--w 512 --h 320]
//        [--world WORLDSIZE_HUGE] [--opponents 8] [--seed 5] [--layer sea]
//
// --limit: 30 by default under SwiftShader (software WebGL, headless Chromium's default here), 10
// with --gpu (the machine's GPU; see chromiumLaunch in web-lib.mjs for FFH_CHROMIUM_CHANNEL).
// --layer draws a map layer as well (`sea`: the Sea layer in the world).

import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, chromiumLaunch, webglRenderer, skipped } from "./web-lib.mjs";

const NAME = "r3d-huge";
const args = parseArgs(process.argv.slice(2), { gpu: "flag", bin: "v", limit: "v", w: "v", h: "v", world: "v", opponents: "v", seed: "v", layer: "v" });
const limitMs = Number(args.limit ?? (args.gpu ? 10 : 30)) * 1000;

const require = createRequire(import.meta.url);
let pw;
try {
  pw = require("playwright");
} catch {
  try {
    pw = require(require.resolve("playwright", { paths: ["/opt/node22/lib/node_modules"] }));
  } catch {
    process.exit(skipped(NAME, "the playwright package was not found"));
  }
}

const server = await startServer({
  bin: findBin(args.bin),
  saves: tempDir(NAME),
  extra: ["--new", "--seed", args.seed ?? "5", "--world", args.world ?? "WORLDSIZE_HUGE", "--opponents", args.opponents ?? "8", "--assets", path.join(ROOT, "crates", "ffh-play", "assets")],
});
let browser = null;
const finish = async (code) => {
  await Promise.race([browser?.close().catch(() => {}), new Promise((r) => setTimeout(r, 3000))]);
  await Promise.race([server.stop(), new Promise((r) => setTimeout(r, 3000))]);
  process.exit(code);
};
const fail = (why) => {
  console.log(`${NAME}: FAILED: ${why}`);
  return finish(1);
};
/** `p`, or a rejection after `ms` (a page whose main thread is stuck never settles an evaluate). */
const within = (p, ms, what) => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error(`${what}: no answer within ${Math.round(ms / 1000)} s`)), Math.max(1, ms)))]);

try {
  browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
} catch (e) {
  await server.stop();
  process.exit(skipped(NAME, `no browser: ${String(e.message ?? e).split("\n")[0]}`));
}
const page = await browser.newPage({ viewport: { width: Number(args.w ?? 512), height: Number(args.h ?? 320) } });
const errors = [];
let crashed = false;
page.on("pageerror", (e) => errors.push(`page error: ${e.message}`));
page.on("console", (m) => (m.type() === "error" ? errors.push(`console error: ${m.text().slice(0, 300)}`) : null));
page.on("crash", () => (crashed = true));

try {
  // force3d=1 bypasses the GPU guard (docs/R3D.md §1): SwiftShader on purpose.
  await page.goto(`${server.url}/?renderer=hd2d&force3d=1`);
  await page.waitForFunction(() => window.ffh?.ready && window.ffh.S.map && window.ffh.mapView.r3d?.cam, null, { timeout: 300000 });
  console.log(`WebGL renderer: ${await webglRenderer(page)}`);
  const size = await page.evaluate(
    ({ layer }) => {
      const { S, mapView: mv } = window.ffh;
      if (layer) window.ffh.setLayer(layer);
      // Zoomed out, at the middle of the map: the most plots, names and cards in one frame.
      mv.cam = { ...mv.cam, ts: 40 };
      mv.r3d.ts = 40;
      mv.centerOn(Math.floor(S.map.w / 2), Math.floor(S.map.h / 2), { animate: false });
      return { w: S.map.w, h: S.map.h };
    },
    { layer: args.layer ?? null },
  );
  console.log(`map ${size.w}x${size.h}, limit ${limitMs / 1000} s`);

  const t0 = Date.now();
  const left = () => limitMs - (Date.now() - t0);
  // As a player does it: the reveal is asked of the server by the page, which then fetches the
  // view and the map and brings the scene up to date in its own frame loop.
  page.evaluate(async () => (await import("/app/api.js")).session("debug", { reveal: true })).catch(() => {});
  for (;;) {
    const state = await within(
      page.evaluate(() => {
        const { S } = window.ffh;
        const vis = S.map?.vis;
        let hidden = 0;
        if (vis) for (let i = 0; i < vis.length; i++) if (vis[i] === 0) hidden++;
        return { reveal: !!S.status?.reveal && !!S.map?.reveal, hidden, seas: S.view?.seas?.length ?? 0 };
      }),
      left(),
      "the page after the reveal",
    );
    if (state.reveal && state.hidden === 0) {
      console.log(`revealed map in the page after ${Date.now() - t0} ms (${state.seas} seas)`);
      break;
    }
    if (left() <= 0) throw new Error(`the revealed map did not arrive within ${limitMs / 1000} s (${state.hidden} plots hidden)`);
    await new Promise((r) => setTimeout(r, 100));
  }
  // One complete frame of the revealed map: synced, drawn and finished by the GPU (readPixels).
  const frame = await within(
    page.evaluate(async () => {
      const r3d = window.ffh.mapView.r3d;
      const gl = r3d.gl;
      const t = performance.now();
      r3d.sync();
      r3d.render(1);
      await r3d.whenReady?.();
      r3d.render(1);
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(4));
      return { ms: Math.round(performance.now() - t), lost: gl.isContextLost() };
    }),
    left(),
    "the frame of the revealed map",
  );
  const total = Date.now() - t0;
  console.log(`frame of the revealed map: ${frame.ms} ms; reveal to frame: ${total} ms`);
  if (frame.lost) throw new Error("the WebGL context was lost");
  if (crashed) throw new Error("the tab crashed");
  if (errors.length) throw new Error(errors.slice(0, 5).join("\n  "));
  console.log(`${NAME}: ok (${total} ms of ${limitMs} ms)`);
  await finish(0);
} catch (e) {
  await fail(`${crashed ? "the tab crashed; " : ""}${e.message ?? e}${errors.length ? `\n  ${errors.slice(0, 5).join("\n  ")}` : ""}`);
}
