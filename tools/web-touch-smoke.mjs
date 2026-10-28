#!/usr/bin/env node
// The phone smoke test of the browser client (docs/HUMAN.md, "Phones and touch screens"): starts
// `ffh-play serve` with a new game and plays it on an emulated phone (touch only, a coarse
// pointer, device-pixel ratio 2) with the isometric map and its interface (the default, and the
// one look phones must support), upright (390x844) and on its side (844x390): the page fits the
// screen (no sideways scroll, End turn and the map in view, the bottom panel folded), the menu
// button opens the screens and a tap on the map only closes it, a tap selects a unit, a first tap
// on a plot previews the move (the tooltip says "tap again") and a second tap moves there, a
// two-finger pinch zooms in and out, the settler founds a city from its button, the research and
// production popups offer their tap modes and "Add to queue" appends while the popup stays, the
// city screen opens from the Cities list as a whole-screen dialog with the plot grid, the tech
// tree opens from the menu, a tab opens the bottom panel, and End turn ends the turn (the gate
// answered with "End turn anyway"). It fails on any console error, page error, failed request or
// HTTP error, and saves screenshots for review.
//
//   node tools/web-touch-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]
//        [--assets DIR]   (serve the client files from DIR: crates/ffh-play/assets, live edits)
//
// The browser: the `playwright` package (resolved normally, then in node's global modules and
// $NODE_PATH, as tools/web-smoke.mjs does) and its Chromium; without it the test is SKIPPED
// (exit 0, or 1 when $CI is set). Screenshots go to --out (default target/web-touch-smoke, not
// committed). Exit 1 on a failure, 2 on bad arguments.

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped } from "./web-lib.mjs";

let args;
try {
  args = parseArgs(process.argv.slice(2), { bin: "v", out: "v", seed: "v", assets: "v", headed: "flag" });
  if (args.seed !== undefined && !/^\d+$/.test(args.seed)) throw new Error(`--seed needs a number, not ${args.seed}`);
} catch (e) {
  console.error(`web-touch-smoke: ${e.message}`);
  console.error("usage: node tools/web-touch-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]");
  process.exit(2);
}
const OUT = path.resolve(args.out ?? path.join(ROOT, "target", "web-touch-smoke"));
const SEED = Number(args.seed ?? 3);
const ASSETS = args.assets ? ["--assets", path.resolve(args.assets)] : [];
const PHONES = [
  { name: "upright", width: 390, height: 844 },
  { name: "side", width: 844, height: 390 },
];

async function loadPlaywright() {
  try {
    return await import("playwright");
  } catch {
    /* not installed locally */
  }
  const require = createRequire(import.meta.url);
  const roots = [path.join(path.dirname(process.execPath), "..", "lib", "node_modules"), ...(process.env.NODE_PATH ?? "").split(path.delimiter).filter(Boolean)];
  for (const r of roots) {
    try {
      return require(require.resolve("playwright", { paths: [r] }));
    } catch {
      /* next */
    }
  }
  return null;
}

/** Plays one phone's run against a fresh server; returns the problems seen. */
async function run(pw, bin, phone) {
  const problems = [];
  const saves = tempDir("touch-saves");
  const server = await startServer({ bin, saves, extra: ["--new", "--seed", String(SEED), ...ASSETS] });
  const browser = await pw.chromium.launch({ headless: !args.headed });
  try {
    const ctx = await browser.newContext({ viewport: { width: phone.width, height: phone.height }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
    const page = await ctx.newPage();
    page.on("console", (m) => m.type() === "error" && problems.push(`console: ${m.text()}`));
    page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
    page.on("requestfailed", (r) => r.failure()?.errorText !== "net::ERR_ABORTED" && problems.push(`request failed: ${r.url()} ${r.failure()?.errorText ?? ""}`));
    page.on("response", (r) => r.status() >= 400 && problems.push(`HTTP ${r.status()}: ${r.url()}`));
    const ev = (f, a) => page.evaluate(f, a);
    const wait = (f, a, what, ms = 15000) => until(() => ev(f, a), what, ms);
    const shot = (n) => page.screenshot({ path: path.join(OUT, `${phone.name}-${n}.png`) });
    const step = (m) => console.log(`web-touch-smoke [${phone.name}]: ${m}`);
    const check = (ok, what) => {
      if (!ok) throw new Error(what);
    };

    await page.goto(server.url);
    await wait(() => window.ffh?.ready && window.ffh.S.view && window.ffh.mapView.iso, null, "the game and the isometric map", 30000);
    await page.waitForTimeout(800);

    // The page fits the phone.
    const fit = await ev(() => {
      const r = (s) => document.querySelector(s).getBoundingClientRect();
      const end = r("#end-turn");
      return {
        sideways: document.documentElement.scrollWidth - innerWidth,
        end: end.right <= innerWidth && end.bottom <= innerHeight && end.width > 0,
        // The map's part no panel covers (MapView.clearRect: the isometric interface floats over it).
        map: (() => { const c = window.ffh.mapView.clearRect(); return Math.min(r("#mapwrap").height, c.y1 - c.y0) / innerHeight; })(),
        folded: document.body.classList.contains("bottom-closed"),
        coarse: matchMedia("(pointer: coarse)").matches,
      };
    });
    check(fit.coarse, "the emulated phone has no coarse pointer");
    check(fit.sideways <= 0, `the page scrolls sideways by ${fit.sideways} px`);
    check(fit.end, "End turn is not on screen");
    check(fit.folded, "the bottom panel does not start folded");
    check(fit.map >= 0.3, `the map has ${Math.round(fit.map * 100)}% of the screen's height`);
    await shot("01-start");
    step(`fits: map ${Math.round(fit.map * 100)}% of the height`);

    // The menu: open, every screen's button in view; a tap on the map only closes it.
    await page.tap("#btn-menu");
    check(await ev(() => [...document.querySelectorAll("#menu button")].every((b) => { const r = b.getBoundingClientRect(); return r.width > 0 && r.bottom <= innerHeight; })), "a menu button is hidden");
    await shot("02-menu");
    const sel0 = await ev(() => JSON.stringify(window.ffh.S.sel));
    // A point of the map no panel and not the sheet covers.
    const mid = await ev(() => {
      const r = window.ffh.mapView.canvas.getBoundingClientRect();
      for (let y = r.bottom - 40; y > r.top; y -= 20) {
        for (const x of [r.left + r.width / 2, r.left + r.width / 4, r.left + (3 * r.width) / 4]) if (document.elementFromPoint(x, y)?.matches("canvas.map")) return { x, y };
      }
      return null;
    });
    check(mid, "the open menu leaves no map to tap");
    await page.touchscreen.tap(mid.x, mid.y);
    await page.waitForTimeout(300);
    check(!(await ev(() => document.body.classList.contains("menu-open"))), "a tap on the map left the menu open");
    check(sel0 === (await ev(() => JSON.stringify(window.ffh.S.sel))), "the tap that closed the menu changed the selection");
    check(!(await ev(() => window.ffh.units.currentMode())) && (await ev(() => document.querySelector("#mapwrap .tooltip").hidden)), "the tap that closed the menu acted on the map");
    step("menu");

    // A unit that can move (not the settler), centred; a tap selects it.
    const unit = await ev(() => {
      const S = window.ffh.S;
      const own = S.view.units.filter((u) => u.owner === S.view.me.player).map((u) => ({ id: u.id, x: u.x, y: u.y, type: S.data.tables.UnitType[u.type]?.type ?? "" }));
      return own.find((u) => !/SETTLER|WORKER/.test(u.type)) ?? null;
    });
    check(unit, "no unit besides the settler");
    await ev((u) => window.ffh.mapView.centerOn(u.x, u.y, { animate: false }), unit);
    await page.waitForTimeout(250);
    const at = (x, y) => ev(([x, y]) => { const mv = window.ffh.mapView; const r = mv.canvas.getBoundingClientRect(); const s = mv.screenOf(x, y); return { x: r.left + s.x, y: r.top + s.y }; }, [x, y]);
    const tapPlot = async (x, y) => {
      const p = await at(x, y);
      await page.touchscreen.tap(p.x, p.y);
    };
    for (let i = 0; i < 3 && !(await ev((id) => window.ffh.S.sel?.units.includes(id), unit.id)); i++) {
      await tapPlot(unit.x, unit.y);
      await page.waitForTimeout(250);
    }
    check(await ev((id) => window.ffh.S.sel?.units.includes(id), unit.id), `a tap does not select the ${unit.type}`);
    step(`a tap selects the ${unit.type}`);

    // Tap a free land plot two steps away: a preview; tap it again: the move.
    const target = await ev(([x0, y0]) => {
      const S = window.ffh.S;
      const m = S.map;
      const kinds = S.data.tables.PlotType.map((t) => t.type);
      const r = window.ffh.mapView.canvas.getBoundingClientRect();
      for (const [dx, dy] of [[2, 0], [0, 2], [-2, 0], [0, -2], [1, 1], [-1, -1], [1, -1], [-1, 1]]) {
        const x = x0 + dx;
        const y = y0 + dy;
        if (y < 0 || y >= m.h || (!m.wrap_x && (x < 0 || x >= m.w))) continue;
        const i = y * m.w + (((x % m.w) + m.w) % m.w);
        if (m.vis[i] !== 2 || !["PLOT_LAND", "PLOT_HILLS"].includes(kinds[m.plot_type[i]])) continue;
        if (S.view.units.some((u) => u.x === x && u.y === y)) continue;
        // On the map in view, not under a panel.
        const s = window.ffh.mapView.screenOf(x, y);
        if (!document.elementFromPoint(r.left + s.x, r.top + s.y)?.matches("canvas.map")) continue;
        return { x, y };
      }
      return null;
    }, [unit.x, unit.y]);
    check(target, "no free land plot near the unit on screen");
    await tapPlot(target.x, target.y);
    await wait(() => { const t = document.querySelector("#mapwrap .tooltip"); return !t.hidden && /tap again to go/.test(t.textContent); }, null, "the move's preview after one tap");
    check(await ev(([id, x, y]) => { const u = window.ffh.S.units.get(id); return u.x === x && u.y === y; }, [unit.id, unit.x, unit.y]), "one tap moved the unit");
    await shot("03-preview");
    await tapPlot(target.x, target.y);
    await wait(([id, x, y]) => { const u = window.ffh.S.units.get(id); return u && (u.x !== x || u.y !== y); }, [unit.id, unit.x, unit.y], "the move after the second tap");
    check(await ev(() => document.querySelector("#mapwrap .tooltip").hidden), "the preview stays after the move");
    step("tap, tap again: moved");

    // A pinch out zooms in; a pinch in zooms out.
    const cdp = await ctx.newCDPSession(page);
    const pinch = async (from, to) => {
      // About the middle of the map's clear part (MapView.clearRect: no panel under a finger).
      const c = await ev(() => { const mv = window.ffh.mapView; const r = mv.canvas.getBoundingClientRect(); const k = mv.clearRect(); return { x: r.left + (k.x0 + k.x1) / 2, y: r.top + (k.y0 + k.y1) / 2 }; });
      const points = (d) => [{ x: c.x - d, y: c.y, id: 1 }, { x: c.x + d, y: c.y, id: 2 }];
      await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: points(from) });
      for (let i = 1; i <= 8; i++) await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: points(from + ((to - from) * i) / 8) });
      await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
      await page.waitForTimeout(150);
      return ev(() => window.ffh.mapView.cam.ts);
    };
    const ts0 = await ev(() => window.ffh.mapView.cam.ts);
    const ts1 = await pinch(40, 100);
    const ts2 = await pinch(100, 30);
    check(ts1 > ts0 && ts2 < ts1, `the pinch zooms ${ts0} -> ${ts1} -> ${ts2}`);
    step(`pinch: ${ts0} -> ${ts1} -> ${ts2}`);

    // The settler founds a city: the research popup, then the production popup, with tap modes.
    const settler = await ev(() => window.ffh.S.view.units.find((u) => u.owner === window.ffh.S.view.me.player && window.ffh.S.data.tables.UnitType[u.type]?.type === "UNIT_SETTLER")?.id ?? null);
    check(settler !== null, "no settler");
    await ev((id) => window.ffh.units.select([id], { center: true }), settler);
    await wait(() => !!document.querySelector('#unit-panel button.action[data-type="MISSION_FOUND"]:not([disabled])'), null, "the Found button");
    await page.tap('#unit-panel button.action[data-type="MISSION_FOUND"]');
    await wait(() => !!document.querySelector(".modal button.build[data-tech]"), null, "the research popup");
    check(await ev(() => getComputedStyle(document.querySelector(".modal .tap-mode")).display !== "none"), "the research popup has no tap modes");
    await shot("04-research");
    await page.tap(".modal button.build[data-tech]");
    await wait(() => !!document.querySelector(".modal.city-screen button.build:not([disabled])"), null, "the production popup");
    await page.tap('.modal.city-screen .tap-mode button[data-tap="queue"]');
    await wait(() => document.querySelector('.modal.city-screen .tap-mode button[data-tap="queue"]')?.classList.contains("on"), null, "Add to queue on");
    const queued = () => ev(() => document.querySelectorAll(".modal.city-screen ol.queue li:not(.dim)").length);
    const n0 = await queued();
    await page.tap(".modal.city-screen button.build:not([disabled])");
    await wait((n) => document.querySelectorAll(".modal.city-screen ol.queue li:not(.dim)").length === n + 1, n0, "the queued build");
    await shot("05-city");
    await page.tap(".modal.city-screen .modal-head .close");
    step("founded a city, queued a build");

    // The city screen from the Cities list: the whole-screen dialog with the plot grid (not the
    // desktop's panels around the city on the map), a plot worked or freed with a tap.
    await page.tap("#btn-menu");
    await page.tap("#btn-cities");
    await wait(() => !!document.querySelector(".modal tr.clickable"), null, "the cities list");
    await page.tap(".modal tr.clickable");
    await wait(() => !!document.querySelector(".modal.city-screen .cgrid button.cplot:not([disabled])"), null, "the city screen's plot grid");
    check(!(await ev(() => !!document.querySelector(".modal.city-overlay"))), "the city screen is the overlay");
    check(await ev(() => { const m = document.querySelector(".modal.city-screen").getBoundingClientRect(); return m.left >= 0 && m.right <= innerWidth; }), "the city screen is wider than the screen");
    await shot("06-city-screen");
    await page.tap(".modal.city-screen .modal-head .close");
    step("the city screen");

    // The tech tree from the menu; a tab opens the bottom panel.
    await page.tap("#btn-menu");
    await page.tap("#btn-techs");
    await wait(() => !!document.querySelector(".modal.techtree .tech"), null, "the tech tree");
    await shot("07-techtree");
    await page.tap(".modal.techtree .modal-head .close");
    await page.tap('#bottom-tabs button[data-tab="report"]');
    check(!(await ev(() => document.body.classList.contains("bottom-closed"))), "a tab does not open the bottom panel");
    await shot("08-bottom");
    await page.tap("#bottom-toggle");

    // End the turn (the gate: end anyway); the next turn comes back.
    const turn = await ev(() => window.ffh.S.status.turn);
    await page.tap("#end-turn");
    await page.waitForTimeout(400);
    if (await ev(() => !!document.querySelector('#modals .modal-buttons button[data-value="force"]'))) await page.tap('#modals .modal-buttons button[data-value="force"]');
    await wait((t) => { const s = window.ffh.S.status; return s.turn > t && s.phase === "human"; }, turn, `turn ${turn + 1}`, 60000);
    await shot("09-next-turn");
    step(`ended turn ${turn}`);
  } catch (e) {
    problems.push(e.message);
  } finally {
    await browser.close();
    server.stop();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  return problems;
}

const pw = await loadPlaywright();
if (!pw) {
  process.exit(skipped("web-touch-smoke", "no playwright package"));
}
let bin;
try {
  bin = findBin(args.bin);
} catch (e) {
  console.error(`web-touch-smoke: ${e.message}`);
  process.exit(1);
}
fs.mkdirSync(OUT, { recursive: true });
let failed = false;
for (const phone of PHONES) {
  const problems = await run(pw, bin, phone);
  for (const p of problems) console.error(`web-touch-smoke [${phone.name}]: FAIL ${p}`);
  failed ||= problems.length > 0;
}
console.log(failed ? "web-touch-smoke: FAILED" : `web-touch-smoke: ok (screenshots in ${path.relative(process.cwd(), OUT)})`);
process.exit(failed ? 1 : 0);
