#!/usr/bin/env node
// The orientation smoke test of the browser client (docs/HUMAN.md, "Upright, on its side,
// locked"): starts `ffh-play serve` with a new game and plays on an emulated phone (touch, a coarse
// pointer, the isometric map). Turned with the phone: the layout follows, upright (the card across
// the screen) and on its side (the card a column at the right), back and forth in one page. Locked
// in Options (headless Chromium grants no lock, so the page turns itself, as on an iPhone): locked
// to landscape while upright, the content is turned and laid out on its side (the map wider than
// high, the card a column), a tap on the screen where a unit is drawn selects it, the buttons take
// taps (Close, End turn), and turning the phone on its side drops the turn; locked to portrait
// while on its side, the content is turned and laid out upright; the lock outlasts a reload; "Turns
// with the phone" ends it. On a computer the choice is not offered and a stored lock changes
// nothing. It fails on any console error, page error, failed request or HTTP error, and saves
// screenshots (of the screen as it is: a turned page shows sideways).
//
//   node tools/web-orient-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]
//
// The browser: the `playwright` package and its Chromium, found as tools/web-touch-smoke.mjs finds
// them; without it the test is SKIPPED (exit 0, or 1 when $CI is set). Screenshots go to --out
// (default target/web-orient-smoke, not committed). Exit 1 on a failure, 2 on bad arguments.

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped } from "./web-lib.mjs";

let args;
try {
  args = parseArgs(process.argv.slice(2), { bin: "v", out: "v", seed: "v", assets: "v", headed: "flag" });
  if (args.seed !== undefined && !/^\d+$/.test(args.seed)) throw new Error(`--seed needs a number, not ${args.seed}`);
} catch (e) {
  console.error(`web-orient-smoke: ${e.message}`);
  console.error("usage: node tools/web-orient-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]");
  process.exit(2);
}
const OUT = path.resolve(args.out ?? path.join(ROOT, "target", "web-orient-smoke"));
const SEED = Number(args.seed ?? 3);
const ASSETS = args.assets ? ["--assets", path.resolve(args.assets)] : [];
const UPRIGHT = { width: 390, height: 844 };
const SIDE = { width: 844, height: 390 };

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

async function run(pw, bin) {
  const problems = [];
  const saves = tempDir("orient-saves");
  const server = await startServer({ bin, saves, extra: ["--new", "--seed", String(SEED), ...ASSETS] });
  const browser = await pw.chromium.launch({ headless: !args.headed });
  const check = (ok, what) => {
    if (!ok) throw new Error(what);
  };
  const watch = (name, page) => {
    page.on("console", (m) => m.type() === "error" && problems.push(`${name} console: ${m.text()}`));
    page.on("pageerror", (e) => problems.push(`${name} page error: ${e.message}`));
    page.on("requestfailed", (r) => r.failure()?.errorText !== "net::ERR_ABORTED" && problems.push(`${name} request failed: ${r.url()} ${r.failure()?.errorText ?? ""}`));
    page.on("response", (r) => r.status() >= 400 && problems.push(`${name} HTTP ${r.status()}: ${r.url()}`));
  };
  const ready = (page) => until(() => page.evaluate(() => window.ffh?.ready && window.ffh.S.view && window.ffh.mapView.iso), "the game and the isometric map", 30000);
  try {
    // A computer: no choice, and a stored lock changes nothing.
    {
      const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      await ctx.addInitScript(() => localStorage.setItem("ffh-orientation", "portrait"));
      const page = await ctx.newPage();
      watch("computer", page);
      await page.goto(`${server.url}/?renderer=iso`);
      await ready(page);
      const st = await page.evaluate(() => ({ rot: window.ffh.orient().rot, turned: document.documentElement.classList.contains("turned") }));
      check(st.rot === 0 && !st.turned, "a stored lock turned the page on a computer");
      await page.evaluate(() => window.ffh.tools.openOptions());
      await until(() => page.evaluate(() => !!document.querySelector(".modal[data-tag=options]")), "the Options dialog");
      check(!(await page.evaluate(() => !!document.querySelector("input[name=option-orientation]"))), "Options offers the orientation on a computer");
      console.log("web-orient-smoke [computer]: no choice, no turn");
      await ctx.close();
    }

    const ctx = await browser.newContext({ viewport: UPRIGHT, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
    const page = await ctx.newPage();
    watch("phone", page);
    const ev = (f, a) => page.evaluate(f, a);
    const wait = (f, a, what, ms = 15000) => until(() => ev(f, a), what, ms);
    const shot = (n) => page.screenshot({ path: path.join(OUT, `${n}.png`) });
    const step = (m) => console.log(`web-orient-smoke [phone]: ${m}`);
    await page.goto(`${server.url}/?renderer=iso`);
    await ready(page);
    await page.waitForTimeout(600);
    // A unit selected, so the card shows.
    await ev(() => {
      const S = window.ffh.S;
      const u = S.view.units.find((x) => x.owner === S.view.me.player);
      window.ffh.units.select([u.id]);
    });
    await wait(() => !!window.ffh.S.sel && document.querySelector("#side").getBoundingClientRect().width > 0, null, "the card");
    /** The layout as the page sees it: its viewport, the map's size, the card's place. */
    const layout = () =>
      ev(() => {
        const side = document.querySelector("#side").getBoundingClientRect();
        const end = document.querySelector("#end-turn").getBoundingClientRect();
        const mv = window.ffh.mapView;
        return {
          w: innerWidth,
          h: innerHeight,
          rot: window.ffh.orient().rot,
          map: { w: mv.vw, h: mv.vh },
          // The card across the screen (upright) or a column at its right (on its side).
          card: side.width >= innerWidth * 0.8 ? "across" : side.right >= innerWidth - 8 && side.width < innerWidth * 0.6 ? "column" : `${Math.round(side.left)}..${Math.round(side.right)} of ${innerWidth}`,
          end: end.width > 0 && end.left >= -1 && end.top >= -1 && end.right <= innerWidth + 1 && end.bottom <= innerHeight + 1,
        };
      });
    const expect = async (what, want) => {
      await page.waitForTimeout(400);
      const l = await layout();
      const ok = l.rot === want.rot && l.card === want.card && l.end && (want.wide ? l.map.w > l.map.h : l.map.h > l.map.w) && l.w === want.w && l.h === want.h;
      check(ok, `${what}: ${JSON.stringify(l)}`);
      return l;
    };

    // Turned with the phone: the layout follows.
    await expect("upright", { rot: 0, card: "across", wide: false, w: UPRIGHT.width, h: UPRIGHT.height });
    await page.setViewportSize(SIDE);
    await expect("turned on its side", { rot: 0, card: "column", wide: true, w: SIDE.width, h: SIDE.height });
    await page.setViewportSize(UPRIGHT);
    await expect("upright again", { rot: 0, card: "across", wide: false, w: UPRIGHT.width, h: UPRIGHT.height });
    await shot("01-upright");
    step("the layout follows the phone");

    // Locked to landscape while upright: the page turns itself.
    const choose = async (val) => {
      await ev(() => window.ffh.tools.openOptions());
      await wait(() => !!document.querySelector(".modal[data-tag=options]"), null, "the Options dialog");
      await page.tap(`label:has(#option-${val}-orientation)`);
      await wait((v) => document.querySelector(`#option-${v}-orientation`)?.checked, val, `${val} chosen`);
      await page.waitForTimeout(300);
      // Close: a tap through the turn.
      await page.tap('.modal[data-tag=options] .modal-buttons button');
      await wait(() => !document.querySelector(".modal[data-tag=options]"), null, "Options closed by a tap");
    };
    await choose("landscape");
    const l1 = await expect("locked to landscape, upright", { rot: await ev(() => window.ffh.orient().rot || 1), card: "column", wide: true, w: UPRIGHT.height, h: UPRIGHT.width });
    check(await ev(() => document.documentElement.classList.contains("turned")), "the page is not turned");
    await shot("02-landscape-locked-upright");
    step(`locked to landscape while upright: turned ${l1.rot} (the browser's lock: ${(await ev(() => window.ffh.orient().locked)) ? "granted" : "none"})`);

    // A tap where a unit is drawn selects it (the pointer through the turn).
    await ev(() => window.ffh.units.clearSelection());
    const unit = await ev(() => {
      const S = window.ffh.S;
      const own = S.view.units.filter((u) => u.owner === S.view.me.player);
      return own[0] ? { id: own[0].id, x: own[0].x, y: own[0].y } : null;
    });
    await ev((u) => window.ffh.mapView.centerOn(u.x, u.y, { animate: false }), unit);
    await page.waitForTimeout(300);
    const tapPlot = async (x, y) => {
      const p = await ev(([x, y]) => {
        const mv = window.ffh.mapView;
        const r = mv.canvas.getBoundingClientRect();
        const s = mv.screenOf(x, y);
        return window.ffh.orient().toScreen(r.left + s.x, r.top + s.y);
      }, [x, y]);
      await page.touchscreen.tap(p.x, p.y);
    };
    for (let i = 0; i < 3 && !(await ev((id) => window.ffh.S.sel?.units.includes(id), unit.id)); i++) {
      await tapPlot(unit.x, unit.y);
      await page.waitForTimeout(300);
    }
    check(await ev((id) => window.ffh.S.sel?.units.includes(id), unit.id), "a tap on a unit through the turn does not select it");
    step("a tap through the turn selects a unit");

    // End turn takes a tap through the turn.
    const turn = await ev(() => window.ffh.S.status.turn);
    await page.tap("#end-turn");
    await page.waitForTimeout(500);
    if (await ev(() => !!document.querySelector('#modals .modal-buttons button[data-value="force"]'))) await page.tap('#modals .modal-buttons button[data-value="force"]');
    await wait((t) => { const s = window.ffh.S.status; return s.turn > t && s.phase === "human"; }, turn, `turn ${turn + 1}`, 60000);
    step(`End turn by a tap through the turn: turn ${turn + 1}`);

    // The phone on its side: the lock is met, no turn. (Headless Chromium enters fullscreen for the
    // lock; a window in fullscreen cannot be resized: out of it first.)
    await ev(() => document.fullscreenElement && document.exitFullscreen());
    await page.waitForTimeout(300);
    await page.setViewportSize(SIDE);
    await expect("locked to landscape, on its side", { rot: 0, card: "column", wide: true, w: SIDE.width, h: SIDE.height });
    check(!(await ev(() => document.documentElement.classList.contains("turned"))), "the page stays turned on its side");
    step("on its side: no turn");

    // Locked to portrait while on its side: turned, upright layout (the card across the screen).
    await choose("portrait");
    await ev(() => {
      const S = window.ffh.S;
      const u = S.view.units.find((x) => x.owner === S.view.me.player);
      window.ffh.units.select([u.id]);
    });
    const l2 = await expect("locked to portrait, on its side", { rot: await ev(() => window.ffh.orient().rot || 1), card: "across", wide: false, w: SIDE.height, h: SIDE.width });
    await shot("03-portrait-locked-side");
    step(`locked to portrait while on its side: turned ${l2.rot}`);

    // The lock outlasts a reload.
    await page.reload();
    await ready(page);
    await page.waitForTimeout(600);
    const l3 = await layout();
    check(l3.rot !== 0 && l3.map.h > l3.map.w && l3.w === SIDE.height, `after a reload: ${JSON.stringify(l3)}`);
    step("the lock outlasts a reload");

    // Turns with the phone: no turn.
    await choose("auto");
    const l4 = await layout();
    check(l4.rot === 0 && l4.map.w > l4.map.h && l4.w === SIDE.width, `turns with the phone again: ${JSON.stringify(l4)}`);
    check(!(await ev(() => document.documentElement.classList.contains("turned") || !!document.body.style.transform)), "the turn's styles stay");
    await shot("04-auto");
    step("turns with the phone again");
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
  process.exit(skipped("web-orient-smoke", "no playwright package"));
}
let bin;
try {
  bin = findBin(args.bin);
} catch (e) {
  console.error(`web-orient-smoke: ${e.message}`);
  process.exit(1);
}
fs.mkdirSync(OUT, { recursive: true });
const problems = await run(pw, bin);
for (const p of problems) console.error(`web-orient-smoke: FAIL ${p}`);
console.log(problems.length ? "web-orient-smoke: FAILED" : `web-orient-smoke: ok (screenshots in ${path.relative(process.cwd(), OUT)})`);
process.exit(problems.length ? 1 : 0);
