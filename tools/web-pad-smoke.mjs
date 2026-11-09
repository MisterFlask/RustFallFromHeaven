#!/usr/bin/env node
// The controller smoke test of the browser client (docs/HUMAN.md, "A controller on a phone"):
// starts `ffh-play serve` with a new game and drives it with an emulated standard gamepad (the
// page's `navigator.getGamepads` answers a pad the test presses). On a computer (a fine pointer)
// the pad changes nothing. On an emulated phone (touch, a coarse pointer, the isometric map): the
// pad's first press starts the controller's mode and does nothing else; RB selects a unit; the
// D-pad moves the map's cursor (the plot's hover: its tooltip says "A to go"), A moves the unit
// there; Y gives the focus to the selection's card and B gives it back to the map; Back opens the
// menu with the focus on its buttons, the D-pad moves it and B closes the menu; Start ends the
// turn, the end-turn check's dialog taking the focus and A on "End turn anyway" answering it; a
// finger on the screen ends the mode. It fails on any console error, page error, failed request or
// HTTP error, and saves screenshots for review.
//
//   node tools/web-pad-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]
//
// The browser: the `playwright` package and its Chromium, found as tools/web-touch-smoke.mjs finds
// them; without it the test is SKIPPED (exit 0, or 1 when $CI is set). Screenshots go to --out
// (default target/web-pad-smoke, not committed). Exit 1 on a failure, 2 on bad arguments.

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped } from "./web-lib.mjs";

let args;
try {
  args = parseArgs(process.argv.slice(2), { bin: "v", out: "v", seed: "v", assets: "v", headed: "flag" });
  if (args.seed !== undefined && !/^\d+$/.test(args.seed)) throw new Error(`--seed needs a number, not ${args.seed}`);
} catch (e) {
  console.error(`web-pad-smoke: ${e.message}`);
  console.error("usage: node tools/web-pad-smoke.mjs [--bin PATH] [--out DIR] [--seed 3] [--assets DIR] [--headed]");
  process.exit(2);
}
const OUT = path.resolve(args.out ?? path.join(ROOT, "target", "web-pad-smoke"));
const SEED = Number(args.seed ?? 3);
const ASSETS = args.assets ? ["--assets", path.resolve(args.assets)] : [];

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

/** The emulated pad, installed before the page's scripts: `window.__pad` is the pad (standard
 *  mapping), `__padConnect()` plugs it in (the browser's `gamepadconnected`). */
function installPad() {
  const pad = {
    id: "Test controller (STANDARD GAMEPAD)",
    index: 0,
    connected: true,
    mapping: "standard",
    timestamp: 0,
    axes: [0, 0, 0, 0],
    buttons: Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 })),
  };
  let plugged = false;
  window.__pad = pad;
  Navigator.prototype.getGamepads = function () {
    return plugged ? [pad, null, null, null] : [null, null, null, null];
  };
  window.__padConnect = () => {
    plugged = true;
    window.dispatchEvent(new Event("gamepadconnected"));
  };
}

const B = { a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, lt: 6, rt: 7, back: 8, start: 9, up: 12, down: 13, left: 14, right: 15 };

async function run(pw, bin) {
  const problems = [];
  const saves = tempDir("pad-saves");
  const server = await startServer({ bin, saves, extra: ["--new", "--seed", String(SEED), ...ASSETS] });
  const browser = await pw.chromium.launch({ headless: !args.headed });
  try {
    const check = (ok, what) => {
      if (!ok) throw new Error(what);
    };
    const open = async (name, opts) => {
      const ctx = await browser.newContext(opts);
      await ctx.addInitScript(installPad);
      const page = await ctx.newPage();
      page.on("console", (m) => m.type() === "error" && problems.push(`${name} console: ${m.text()}`));
      page.on("pageerror", (e) => problems.push(`${name} page error: ${e.message}`));
      page.on("requestfailed", (r) => r.failure()?.errorText !== "net::ERR_ABORTED" && problems.push(`${name} request failed: ${r.url()} ${r.failure()?.errorText ?? ""}`));
      page.on("response", (r) => r.status() >= 400 && problems.push(`${name} HTTP ${r.status()}: ${r.url()}`));
      const ev = (f, a) => page.evaluate(f, a);
      const wait = (f, a, what, ms = 15000) => until(() => ev(f, a), what, ms);
      /** A press of button `b`: down for a few frames, then up. */
      const press = async (b) => {
        await ev((i) => Object.assign(window.__pad.buttons[i], { pressed: true, value: 1 }), b);
        await page.waitForTimeout(90);
        await ev((i) => Object.assign(window.__pad.buttons[i], { pressed: false, value: 0 }), b);
        await page.waitForTimeout(90);
      };
      await page.goto(`${server.url}/?renderer=iso`);
      await wait(() => window.ffh?.ready && window.ffh.S.view && window.ffh.mapView.iso, null, "the game and the isometric map", 30000);
      await page.waitForTimeout(600);
      return { ctx, page, ev, wait, press, step: (m) => console.log(`web-pad-smoke [${name}]: ${m}`), shot: (n) => page.screenshot({ path: path.join(OUT, `${name}-${n}.png`) }) };
    };

    // A computer: the pad changes nothing.
    {
      const t = await open("computer", { viewport: { width: 1280, height: 800 } });
      check(!(await t.ev(() => matchMedia("(pointer: coarse)").matches)), "the computer has a coarse pointer");
      const sel0 = await t.ev(() => JSON.stringify(window.ffh.S.sel));
      await t.ev(() => window.__padConnect());
      for (const b of [B.a, B.rb, B.y, B.right, B.start]) await t.press(b);
      check(!(await t.ev(() => document.body.classList.contains("pad-mode"))), "a pad on a computer started the controller's mode");
      check(sel0 === (await t.ev(() => JSON.stringify(window.ffh.S.sel))), "a pad on a computer changed the selection");
      check(!(await t.ev(() => !!document.querySelector(".pad-cursor:not([hidden]), .pad-focus"))), "a pad on a computer shows a cursor or a focus");
      t.step("the pad does nothing");
      await t.ctx.close();
    }

    // A phone with a controller.
    const t = await open("phone", { viewport: { width: 844, height: 390 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
    const { ev, wait, press, step, shot, page } = t;
    check(await ev(() => matchMedia("(pointer: coarse)").matches), "the emulated phone has no coarse pointer");
    check(!(await ev(() => document.body.classList.contains("pad-mode"))), "the controller's mode is on before a pad");
    const sel0 = await ev(() => JSON.stringify(window.ffh.S.sel));
    await ev(() => window.__padConnect());
    await press(B.a);
    await wait(() => document.body.classList.contains("pad-mode"), null, "the controller's mode after the first press");
    check(sel0 === (await ev(() => JSON.stringify(window.ffh.S.sel))), "the press that started the mode acted");
    await wait(() => { const c = document.querySelector(".pad-cursor"); return c && !c.hidden; }, null, "the map's cursor");
    step("the first press starts the mode");

    // RB: a unit that can move (the cycle's next); its plot gets the cursor.
    let unit = null;
    for (let i = 0; i < 6 && !unit; i++) {
      await press(B.rb);
      unit = await ev(() => {
        const S = window.ffh.S;
        const id = S.sel?.units?.[0];
        const u = id !== undefined ? S.units.get(id) : null;
        const type = u ? (S.data.tables.UnitType[u.type]?.type ?? "") : "";
        return u && !/SETTLER|WORKER/.test(type) ? { id: u.id, x: u.x, y: u.y, type } : null;
      });
    }
    check(unit, "RB selects no unit that can move (besides the settler)");
    await wait((u) => { const h = window.ffh.mapView.overlay.hover; return h?.x === u.x && h?.y === u.y; }, unit, "the cursor on the selected unit");
    step(`RB selects the ${unit.type}`);

    // A free land plot two steps away, on screen; the D-pad walks the cursor there.
    const target = await ev(([x0, y0]) => {
      const S = window.ffh.S;
      const m = S.map;
      const kinds = S.data.tables.PlotType.map((k) => k.type);
      const mv = window.ffh.mapView;
      const r = mv.canvas.getBoundingClientRect();
      for (const [dx, dy] of [[2, 0], [0, 2], [-2, 0], [0, -2], [1, 1], [-1, -1], [1, -1], [-1, 1]]) {
        const x = x0 + dx;
        const y = y0 + dy;
        if (y < 0 || y >= m.h || (!m.wrap_x && (x < 0 || x >= m.w))) continue;
        const i = y * m.w + (((x % m.w) + m.w) % m.w);
        if (m.vis[i] !== 2 || !["PLOT_LAND", "PLOT_HILLS"].includes(kinds[m.plot_type[i]])) continue;
        if (S.view.units.some((u) => u.x === x && u.y === y)) continue;
        const s = mv.screenOf(x, y);
        if (!document.elementFromPoint(r.left + s.x, r.top + s.y)?.matches("canvas.map")) continue;
        return { x, y };
      }
      return null;
    }, [unit.x, unit.y]);
    check(target, "no free land plot near the unit on screen");
    for (let i = 0; i < 8; i++) {
      const d = await ev((tg) => {
        const mv = window.ffh.mapView;
        const h = mv.overlay.hover;
        if (!h || (h.x === tg.x && h.y === tg.y)) return null;
        const a = mv.screenOf(h.x, h.y);
        const b = mv.screenOf(tg.x, tg.y);
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        // The screen axis with more to go.
        return Math.abs(dx) >= Math.abs(dy) ? (dx > 0 ? "right" : "left") : dy > 0 ? "down" : "up";
      }, target);
      if (!d) break;
      await press(B[d]);
    }
    check(await ev((tg) => { const h = window.ffh.mapView.overlay.hover; return h?.x === tg.x && h?.y === tg.y; }, target), "the D-pad does not bring the cursor to the plot");
    await wait(() => { const tip = document.querySelector("#mapwrap .tooltip"); return !tip.hidden && /A to go/.test(tip.textContent); }, null, "the move's preview at the cursor");
    await shot("01-preview");
    await press(B.a);
    await wait(([id, x, y]) => { const u = window.ffh.S.units.get(id); return u && (u.x !== x || u.y !== y); }, [unit.id, unit.x, unit.y], "the move after A");
    step("D-pad to the plot, A: moved");

    // Y: the card's orders take the focus; B: the map again.
    await wait(() => !!window.ffh.S.sel, null, "a selection after the move", 10000);
    await press(B.y);
    await wait(() => { const f = document.querySelector(".pad-focus"); return !!f && !f.closest("#modals, #title-screen"); }, null, "Y: the interface's focus");
    check(await ev(() => document.querySelector(".pad-cursor")?.hidden), "the cursor shows while the interface has the focus");
    await shot("02-interface");
    await press(B.b);
    await wait(() => !document.querySelector(".pad-focus") && !document.querySelector(".pad-cursor").hidden, null, "B: the map's cursor again");
    step("Y, B");

    // Back: the menu with the focus on its buttons; the D-pad moves it; B closes the menu.
    await press(B.back);
    await wait(() => document.body.classList.contains("menu-open") && !!document.querySelector("#menu .pad-focus"), null, "Back: the menu with the focus");
    const f0 = await ev(() => document.querySelector(".pad-focus").id);
    for (const d of [B.right, B.down]) {
      await press(d);
      if ((await ev(() => document.querySelector(".pad-focus")?.id)) !== f0) break;
    }
    check((await ev(() => document.querySelector(".pad-focus")?.id)) !== f0, "the D-pad does not move the menu's focus");
    await shot("03-menu");
    await press(B.b);
    await wait(() => !document.body.classList.contains("menu-open"), null, "B closes the menu");
    step("Back, D-pad, B");

    // Start: End turn; its check's dialog takes the focus, A on "End turn anyway" answers it.
    const turn = await ev(() => window.ffh.S.status.turn);
    await press(B.start);
    await page.waitForTimeout(500);
    if (await ev(() => !!document.querySelector('#modals .modal-buttons button[data-value="force"]'))) {
      await wait(() => !!document.querySelector("#modals .modal .pad-focus"), null, "the end-turn dialog's focus");
      await shot("04-dialog");
      for (let i = 0; i < 6 && !(await ev(() => document.querySelector(".pad-focus")?.dataset.value === "force")); i++) {
        const dir = await ev(() => {
          const a = document.querySelector(".pad-focus").getBoundingClientRect();
          const b = document.querySelector('#modals .modal-buttons button[data-value="force"]').getBoundingClientRect();
          const dx = b.left + b.width / 2 - (a.left + a.width / 2);
          const dy = b.top + b.height / 2 - (a.top + a.height / 2);
          return Math.abs(dy) > Math.abs(dx) ? (dy > 0 ? "down" : "up") : dx > 0 ? "right" : "left";
        });
        await press(B[dir]);
      }
      check(await ev(() => document.querySelector(".pad-focus")?.dataset.value === "force"), "the D-pad does not reach End turn anyway");
      await press(B.a);
      step("the end-turn dialog");
    }
    await wait((n) => { const s = window.ffh.S.status; return s.turn > n && s.phase === "human"; }, turn, `turn ${turn + 1}`, 60000);
    await shot("05-next-turn");
    step(`Start ended turn ${turn}`);

    // A finger ends the mode.
    const mid = await ev(() => { const mv = window.ffh.mapView; const r = mv.canvas.getBoundingClientRect(); const k = mv.clearRect(); return { x: r.left + (k.x0 + k.x1) / 2, y: r.top + (k.y0 + k.y1) / 2 }; });
    await page.waitForTimeout(500);
    await page.touchscreen.tap(mid.x, mid.y);
    await wait(() => !document.body.classList.contains("pad-mode"), null, "a tap ends the controller's mode");
    check(await ev(() => !window.ffh.mapView.pad), "the map keeps the controller's flag after a tap");
    step("a tap ends the mode");
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
  process.exit(skipped("web-pad-smoke", "no playwright package"));
}
let bin;
try {
  bin = findBin(args.bin);
} catch (e) {
  console.error(`web-pad-smoke: ${e.message}`);
  process.exit(1);
}
fs.mkdirSync(OUT, { recursive: true });
const problems = await run(pw, bin);
for (const p of problems) console.error(`web-pad-smoke: FAIL ${p}`);
console.log(problems.length ? "web-pad-smoke: FAILED" : `web-pad-smoke: ok (screenshots in ${path.relative(process.cwd(), OUT)})`);
process.exit(problems.length ? 1 : 0);
