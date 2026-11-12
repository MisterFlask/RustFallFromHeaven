#!/usr/bin/env node
// Screenshots of keywords and reminder text, the rituals' rules and the pending world spells
// (docs/PLAY_DESIGN.md 4.8, docs/HUMAN.md "Keywords and reminder text" and "World rituals";
// CLAUDE.md, "Show UI changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 on the
// HD-2D map (`?renderer=hd2d&force3d=1`; SwiftShader here: minutes, not seconds).
//
//   node tools/keywords-shots.mjs --bin <built>/ffh-play [--out docs/shots/keywords]
//                                 [--assets crates/ffh-play/assets] [--only hurry,cull,notarget,pending,wellspring]
//
// - `hurry` (`stage rituals-hurry`): production-world-ritual.png (Ritual: Warcry's entry, its
//   World Ritual chip's boxes open), production-ritual.png (Rites of Oghma's, its Ritual chip),
//   hurry-ritual.png (gold disabled with its reason, population enabled), help-project.png (the
//   Warcry entry's tooltip: rules text and reminder boxes).
// - `cull` (`stage ritual-civ:CIVILIZATION_DOVIELLO`, The Cull pending with a valid target):
//   tooltip-reminders-wide.png and tooltip-reminders-narrow.png (the world spell's tooltip, its
//   reminder boxes beside it, then below it at 420 px), end-turn-blocked.png.
// - `notarget` (`stage ritual-notarget:CIVILIZATION_DOVIELLO`): pending-no-target.png.
// - `pending` (`stage rituals-pending`): rituals-pending.png (the full list: `Cast pending`).
// - `wellspring` (`stage ritual-civ:CIVILIZATION_AMURITES`): wellspring-prompt.png (the
//   placement opened by itself, no Later).

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "docs", "shots", "keywords"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1000, height: 720 };
const NARROW = { width: 420, height: 860 };
const FRAME = 25000;

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

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function dropToasts(page, keep = null) {
  return page.evaluate((keep) => {
    for (const t of document.querySelectorAll("#toasts .toast")) if (!keep || !t.matches(keep)) t.remove();
  }, keep);
}

/** A page on a served staged save drawn by the HD-2D renderer in software. */
async function open3d(browser, url, viewport = VIEW) {
  const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  await page.goto(url);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 240000, 250);
  await sleep(3000);
  return page;
}

/** Closes the dialogs and the pick a load opened (research, a placement). */
async function settle(page) {
  for (let i = 0; i < 6; i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
  await dropToasts(page);
}

/** A crop around the union of what `sels` match (with `pad` CSS px), at most `maxW` wide. */
async function crop(page, sels, name, { pad = 8, maxW = 520 } = {}) {
  const r = await page.evaluate((sels) => {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const s of sels) {
      for (const e of document.querySelectorAll(s)) {
        if (e.hidden) continue;
        const b = e.getBoundingClientRect();
        if (!b.width || !b.height) continue;
        x0 = Math.min(x0, b.left), y0 = Math.min(y0, b.top), x1 = Math.max(x1, b.right), y1 = Math.max(y1, b.bottom);
      }
    }
    return x0 === Infinity ? null : { x0, y0, x1, y1, w: innerWidth, h: innerHeight };
  }, sels);
  if (!r) throw new Error(`${name}: nothing matches ${sels.join(", ")}`);
  const x = Math.max(0, Math.floor(r.x0 - pad));
  const y = Math.max(0, Math.floor(r.y0 - pad));
  const w = Math.min(maxW, Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(Math.ceil(r.y1 + pad) - y, r.h - y);
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: w, height: h }, timeout: 300000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` as a pointer over it (at its left edge, `dy` below its top). */
async function hover(page, sel, { dx = 8, dy = null } = {}) {
  const at = await page.evaluate(([sel, dx, dy]) => {
    const e = document.querySelector(sel);
    e.scrollIntoView({ block: "center" });
    const r = e.getBoundingClientRect();
    return { x: r.left + dx, y: dy === null ? r.top + r.height / 2 : r.top + dy };
  }, [sel, dx, dy]);
  await page.evaluate(([sel, x, y]) => {
    const e = document.querySelector(sel);
    e.dispatchEvent(new MouseEvent("mouseenter", { clientX: x, clientY: y, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mousemove", { clientX: x, clientY: y, bubbles: true }));
  }, [sel, at.x, at.y]);
  await until(() => page.evaluate(() => !![...document.querySelectorAll(".tooltip.richtip, .tip-side")].some((e) => !e.hidden)), `the tooltip of ${sel}`, 15000);
  await sleep(600);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => document.querySelector(sel)?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false })), sel);
  await sleep(200);
}

const TIP = [".tooltip.richtip:not([hidden])", ".tip-side:not([hidden])"];

/** Serves staged game `kind` and runs `f(page, st, server)`. */
async function onStage(browser, bin, saves, kind, f, viewport = VIEW) {
  const st = await stage(bin, kind, path.join(saves, `staged-${kind.replace(/[^a-z0-9-]/gi, "_")}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
    const page = await open3d(browser, `${server.url}/?renderer=hd2d&force3d=1`, viewport);
    try {
      await f(page, st, server);
    } finally {
      await page.close();
    }
  } finally {
    await server.stop();
  }
}

/** Opens city `id`'s screen. */
async function openCity(page, id) {
  await page.evaluate((id) => window.ffh.emit("open-city", id), id);
  await until(() => page.evaluate(() => !!document.querySelector(".modal.city-screen button.build.ritual-build")), "the city screen's rituals", 30000);
  await sleep(1500);
}

/** The production entry of project `type`: its selector, scrolled into view. */
async function entrySel(page, type) {
  const i = await page.evaluate((t) => window.ffh.S.data.tables.ProjectType.findIndex((e) => e.type === t), type);
  const sel = `.modal.city-screen button.build.ritual-build[data-type="${i}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the entry ${type}`, 20000);
  await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "center" }), sel);
  await sleep(500);
  return sel;
}

// ---- `rituals-hurry`: the production dialog's rituals, their chips, the hurry row ----
async function shotsHurry(browser, bin, saves) {
  await onStage(browser, bin, saves, "rituals-hurry", async (page, st) => {
    await settle(page);
    await openCity(page, st.capital);
    const warcry = await entrySel(page, "PROJECT_RITUAL_WARCRY");
    await hover(page, `${warcry} .kw-chip[data-keyword="KEYWORD_WORLD_RITUAL"]`);
    await crop(page, [warcry, ...TIP], "production-world-ritual.png", { maxW: 760 });
    await unhover(page, `${warcry} .kw-chip[data-keyword="KEYWORD_WORLD_RITUAL"]`);
    const oghma = await entrySel(page, "PROJECT_RITES_OF_OGHMA");
    await hover(page, `${oghma} .kw-chip[data-keyword="KEYWORD_RITUAL"]`);
    await crop(page, [oghma, ...TIP], "production-ritual.png", { maxW: 760 });
    await unhover(page, `${oghma} .kw-chip[data-keyword="KEYWORD_RITUAL"]`);
    // The Warcry entry's tooltip: the project's help (rules text) and its reminder boxes.
    await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "end" }), warcry);
    await hover(page, warcry, { dx: 40, dy: 6 });
    await crop(page, [...TIP], "help-project.png", { maxW: 1000 });
    await unhover(page, warcry);
    // The hurry row: gold disabled with its reason, population enabled.
    const row = ".modal.city-screen button[data-hurry]";
    await page.evaluate((s) => document.querySelector(s).closest(".section").scrollIntoView({ block: "center" }), row);
    await sleep(600);
    await page.evaluate(() => document.querySelectorAll(".tooltip, .tip-side").forEach((t) => (t.hidden = true)));
    await page.mouse.move(2, 2);
    await crop(page, [`${row}`, ".modal.city-screen .hurry-note"], "hurry-ritual.png", { pad: 10 });
  });
}

// ---- `ritual-civ:CIVILIZATION_DOVIELLO`: The Cull pending, a valid target ----
async function shotsCull(browser, bin, saves) {
  for (const [viewport, name] of [
    [VIEW, "tooltip-reminders-wide.png"],
    [NARROW, "tooltip-reminders-narrow.png"],
  ]) {
    await onStage(
      browser,
      bin,
      saves,
      "ritual-civ:CIVILIZATION_DOVIELLO",
      async (page) => {
        await settle(page);
        await sleep(FRAME);
        const sel = "#world-actions button.world-action";
        await until(() => page.evaluate((s) => !!document.querySelector(s), sel), "the world action", 20000);
        await hover(page, sel, { dy: 4 });
        await crop(page, [sel, ...TIP], name, { maxW: viewport.width });
        await unhover(page, sel);
        if (viewport !== VIEW) return;
        // End turn: refused with its reason; the pick opens.
        await page.evaluate(() => document.getElementById("end-turn").click());
        await sleep(1500);
        // The refusal's reason (an error toast at the top, 10 s), then the button's line.
        await crop(page, ["#toasts .toast.error"], "end-turn-refused.png", { pad: 6 });
        await sleep(FRAME);
        await crop(page, ["#end-turn"], "end-turn-blocked.png", { pad: 10 });
      },
      viewport,
    );
  }
}

// ---- `ritual-notarget:CIVILIZATION_DOVIELLO`: pending, no valid target ----
async function shotsNoTarget(browser, bin, saves) {
  await onStage(browser, bin, saves, "ritual-notarget:CIVILIZATION_DOVIELLO", async (page) => {
    await settle(page);
    await sleep(FRAME);
    const sel = "#world-actions button.world-action";
    await until(() => page.evaluate((s) => !!document.querySelector(s), sel), "the world action", 20000);
    await hover(page, sel, { dy: 4 });
    await crop(page, [sel, ...TIP, "#end-turn"], "pending-no-target.png", { maxW: 1000 });
  });
}

// ---- `rituals-pending`: the full list shows `Cast pending` ----
async function shotsPending(browser, bin, saves) {
  await onStage(browser, bin, saves, "rituals-pending", async (page) => {
    await settle(page);
    await page.click("#rituals .ritual-all");
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="rituals"] .rituals-list')), "the rituals list", 10000);
    await sleep(800);
    await crop(page, ['.modal[data-tag="rituals"]'], "rituals-pending.png", { pad: 4, maxW: 680 });
  });
}

// ---- `ritual-civ:CIVILIZATION_AMURITES`: Wellspring's placement opened by itself ----
async function shotsWellspring(browser, bin, saves) {
  await onStage(browser, bin, saves, "ritual-civ:CIVILIZATION_AMURITES", async (page) => {
    // The research choice of the load closed; the placement opens by itself after it.
    for (let i = 0; i < 4 && (await page.evaluate(() => !!document.querySelector(".modal"))); i++) {
      await page.keyboard.press("Escape");
      await sleep(400);
    }
    // (Opened by itself once a turn; an Escape above may have closed it: End turn's way opens it.)
    await sleep(3000);
    if (!(await page.evaluate(() => window.ffh.units.currentMode?.()?.place === true))) {
      await page.evaluate(() => window.ffh.emit("open-pending-cast", window.ffh.S.view.me.world_actions.find((a) => a.pending)?.spell));
    }
    await until(() => page.evaluate(() => window.ffh.units.currentMode?.()?.place === true), "the placement open", 60000, 500);
    await dropToasts(page, ".place-prompt");
    await sleep(FRAME);
    await crop(page, ["#toasts .place-prompt", "#end-turn"], "wellspring-prompt.png", { pad: 10, maxW: 1000 });
  });
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("keywords-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("keywords-shots");
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    if (want("hurry")) await shotsHurry(browser, bin, saves);
    if (want("cull")) await shotsCull(browser, bin, saves);
    if (want("notarget")) await shotsNoTarget(browser, bin, saves);
    if (want("pending")) await shotsPending(browser, bin, saves);
    if (want("wellspring")) await shotsWellspring(browser, bin, saves);
  } finally {
    await browser.close();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  console.log(`keywords-shots: written to ${path.relative(process.cwd(), OUT) || OUT}`);
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`keywords-shots: FAILED: ${e.stack ?? e}`);
    process.exit(1);
  },
);
