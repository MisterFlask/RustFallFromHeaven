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
//        [--world WORLDSIZE_HUGE] [--opponents 8] [--seed 5] [--layer sea] [--query doom=100]
//        [--load SAVE [--player P]]
//
// --limit: 30 by default under SwiftShader (software WebGL, headless Chromium's default here), 10
// with --gpu (the machine's GPU; see chromiumLaunch in web-lib.mjs for FFH_CHROMIUM_CHANNEL).
// --layer draws a map layer as well (`sea`: the Sea layer in the world). --query adds to the page's
// URL (`doom=100`: the map as at Armageddon counter 100, docs/HD2D.md §6).
// --load plays a saved game instead of a new one (a Huge game played on, whose cities have Houses:
// `ffh-play soak run --size WORLDSIZE_HUGE --opponents 8 --turns 150 --saves DIR` leaves its
// autosaves in DIR/auto); with --player P the viewer takes over player P (`--take-over`), so the
// cities of a civilization of the AI's carry its Houses' shields (`--player` needs an AI-only
// save: `ffh-play observe --setup '{"seed":5,"world":"WORLDSIZE_HUGE","opponents":8}' --to-turn 150
// --saves DIR`). After the frame it times the HUD's pass (hud.js `drawHud`, the city plates) as
// served, and 300 plates with Houses' shields against the same without (a measurement, not a
// limit).

import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, chromiumLaunch, webglRenderer, skipped } from "./web-lib.mjs";

const NAME = "r3d-huge";
const args = parseArgs(process.argv.slice(2), { gpu: "flag", bin: "v", limit: "v", w: "v", h: "v", world: "v", opponents: "v", seed: "v", layer: "v", query: "v", load: "v", player: "v" });
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
  extra: [
    ...(args.load
      ? args.player !== undefined
        ? ["--take-over", path.resolve(args.load), "--player", String(args.player)]
        : ["--load", path.resolve(args.load)]
      : ["--new", "--seed", args.seed ?? "5", "--world", args.world ?? "WORLDSIZE_HUGE", "--opponents", args.opponents ?? "8"]),
    "--assets",
    path.join(ROOT, "crates", "ffh-play", "assets"),
  ],
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
  await page.goto(`${server.url}/?renderer=hd2d&force3d=1${args.query ? `&${args.query}` : ""}`);
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
  // The plates' cost with Houses' shields (docs/HOUSES.md, rule 15: an own city's shield on its
  // plate): the HUD's pass (hud.js `drawHud`, every plate in view) timed with the Houses as
  // served, then with 300 plates (the cities in view, copied round to 300 entries at their plots)
  // all carrying a shield (a third of them Annoyed or Furious, with the state's mark) against the
  // same 300 without, alternated over three rounds of 50 passes.
  const plates = await within(
    page.evaluate(async () => {
      const { S, mapView: mv } = window.ffh;
      const r3d = mv.r3d;
      const { drawHud } = await import("/app/r3d/hud.js");
      const time = (n) => {
        drawHud(r3d, r3d.cam);
        const t = performance.now();
        for (let i = 0; i < n; i++) drawHud(r3d, r3d.cam);
        return (performance.now() - t) / n;
      };
      const shields = () => (r3d.plateHits ?? []).filter((b) => b?.kind === "house").length;
      const own = (r3d.cities ?? []).filter((c) => (S.cities.get(c.city.id) ?? c.city).house).length;
      const asServed = time(50);
      const servedDrawn = shields();
      const real = r3d.cities ?? [];
      if (!real.length) return { cities: 0, own, asServed, servedDrawn };
      // The cities whose plates are in view (a shield on each, one pass), copied round to 300.
      const probe = real.map((c, i) => ({ ...c, city: { ...(S.cities.get(c.city.id) ?? c.city), id: -1 - i, house: { id: 0, label: "House", state: "Pleased" } } }));
      r3d.cities = probe;
      drawHud(r3d, r3d.cam);
      const seen = new Set((r3d.plateHits ?? []).filter((b) => b?.kind === "house").map((b) => b.city));
      const inView = real.filter((_, i) => seen.has(-1 - i));
      const pool = inView.length ? inView : real;
      const fake = [];
      for (let i = 0; i < 300; i++) {
        const c = pool[i % pool.length];
        // Ids no view city has, so the HUD reads the entry's own city (`S.cities.get` misses).
        fake.push({ ...c, city: { ...(S.cities.get(c.city.id) ?? c.city), id: -1 - i, house: null } });
      }
      const states = ["Pleased", "Annoyed", "Furious"];
      const set = (on) => fake.forEach((c, i) => (c.city.house = on ? { id: i % 8, label: "House", state: states[i % 3] } : null));
      r3d.cities = fake;
      const rounds = [];
      try {
        for (let k = 0; k < 3; k++) {
          set(true);
          const on = time(50);
          const drawn = shields();
          set(false);
          rounds.push({ on, off: time(50), drawn });
        }
      } finally {
        r3d.cities = real;
      }
      return { cities: real.length, own, asServed, servedDrawn, rounds };
    }),
    180000,
    "the plates' timing",
  );
  const f = (x) => x.toFixed(2);
  console.log(`plates: ${plates.cities} cities on the map, ${plates.own} with a House's shield (${plates.servedDrawn} drawn in view); the HUD pass ${f(plates.asServed)} ms as served`);
  for (const r of plates.rounds ?? []) console.log(`plates: 300 plates, ${r.drawn} shields drawn in view: ${f(r.on)} ms with shields, ${f(r.off)} ms without (+${f(r.on - r.off)} ms)`);
  await finish(0);
} catch (e) {
  await fail(`${crashed ? "the tab crashed; " : ""}${e.message ?? e}${errors.length ? `\n  ${errors.slice(0, 5).join("\n  ")}` : ""}`);
}
