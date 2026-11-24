#!/usr/bin/env node
// Screenshots of the unit card (docs/HUMAN.md, "Units"; CLAUDE.md, "Show UI changes in a
// screenshot"): phone-readable crops at deviceScaleFactor 2 on the HD-2D map
// (`?renderer=hd2d&force3d=1`; in software here unless `--gpu`), and a check that every unit
// type's card fits the window (a hover tooltip cannot scroll).
//
//   node tools/unit-card-shots.mjs --bin <built>/ffh-play [--out DIR] [--assets crates/ffh-play/assets]
//                                  [--only build,types,narrow,fit,unit,caster] [--types UNIT_A,UNIT_B]
//                                  [--fit-window 420x860] [--gpu]
//
// - `build` (`stage ritual-civ:CIVILIZATION_DOVIELLO`): build-beastman.png, the Beastman's entry of
//   the capital's production list with its card and the card's reminder boxes.
// - `types` (the same game): type-<name>.png, the card of a spread of unit types (an archer, a
//   caster, a charger, a large unit, a siege engine, a hero, a ship, a unit that does not fight),
//   or of the types `--types` names.
// - `narrow` (the same game at 420 px): build-beastman-narrow.png, the reminder boxes below the card.
// - `fit` (the same game at 1366 x 768, or `--fit-window WxH`): every unit type's card is measured;
//   the five tallest are listed, and any that does not fit the window fails the run.
// - `unit` (`stage engagement`): unit-<name>.png, the card of an army's units standing in a squad.
// - `caster` (`stage heroes`): unit-caster.png and unit-hero.png, a caster's and a hero's card.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value", types: "value", "fit-window": "value", gpu: "flag" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "target", "unit-card-shots"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1280, height: 800 };
const [FIT_W, FIT_H] = (args["fit-window"] ?? "1366x768").split("x").map(Number);
const LAPTOP = { width: FIT_W, height: FIT_H };
const NARROW = { width: 420, height: 860 };
const TIP = [".tooltip.richtip:not([hidden])", ".tip-side:not([hidden])"];
/** The types of the `types` part: the first of each list the data has. */
const TYPES = [
  ["UNIT_ARCHER"],
  ["UNIT_MAGE", "UNIT_ADEPT"],
  ["UNIT_KNIGHT", "UNIT_HORSEMAN"],
  ["UNIT_WAR_ELEPHANT"],
  ["UNIT_CATAPULT"],
  ["UNIT_DONAL"],
  ["UNIT_GALLEY"],
  ["UNIT_SETTLER"],
];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

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

/** A page on a served staged save drawn by the HD-2D renderer. */
async function open3d(browser, url, viewport) {
  const page = await browser.newPage({ viewport, deviceScaleFactor: 2 });
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  await page.goto(url);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 240000, 250);
  await sleep(3000);
  return page;
}

/** Closes the dialogs a load opened and drops the toasts. */
async function settle(page) {
  for (let i = 0; i < 6; i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
  await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
}

/** The box round what `sels` match, in CSS px (null: nothing shown). */
function boxOf(page, sels) {
  return page.evaluate((sels) => {
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
}

/** A crop round what `sels` match (with `pad` CSS px), saved as `name`. */
async function crop(page, sels, name, { pad = 8 } = {}) {
  const r = await boxOf(page, sels);
  if (!r) throw new Error(`${name}: nothing matches ${sels.join(", ")}`);
  const x = Math.max(0, Math.floor(r.x0 - pad));
  const y = Math.max(0, Math.floor(r.y0 - pad));
  const w = Math.min(Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(Math.ceil(r.y1 + pad) - y, r.h - y);
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: w, height: h }, timeout: 300000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` as a pointer over it. */
async function hover(page, sel, { dx = 8, dy = null, x = null, y = null } = {}) {
  await page.evaluate(([sel, dx, dy, px, py]) => {
    const e = document.querySelector(sel);
    e.scrollIntoView({ block: "center" });
    const r = e.getBoundingClientRect();
    const x = px ?? r.left + dx;
    const y = py ?? (dy === null ? r.top + r.height / 2 : r.top + dy);
    e.dispatchEvent(new MouseEvent("mouseenter", { clientX: x, clientY: y, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mousemove", { clientX: x, clientY: y, bubbles: true }));
  }, [sel, dx, dy, x, y]);
  await until(() => page.evaluate(() => !![...document.querySelectorAll(".tooltip.richtip, .tip-side")].some((e) => !e.hidden)), `the tooltip of ${sel}`, 15000);
  await sleep(500);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => document.querySelector(sel)?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false })), sel);
  await sleep(150);
}

/** A stand-in element with unit type `i`'s tooltip (`infoTip`), at the window's top left: its
 *  selector. The page's own modules make the tip, so it is the card every screen shows. */
async function typeProbe(page, i) {
  await page.evaluate(async (i) => {
    const { infoTip } = await import("/app/infotips.js");
    const { tip } = await import("/app/tips.js");
    document.getElementById("uc-probe")?.remove();
    const el = document.createElement("span");
    el.id = "uc-probe";
    el.style.cssText = "position:fixed;left:4px;top:4px;width:8px;height:8px;z-index:1";
    document.body.appendChild(el);
    tip(el, infoTip("UnitType", i, { hint: "Click: build now. Shift: add to the queue. Ctrl: put first." }));
  }, i);
  return "#uc-probe";
}

/** Serves staged game `kind` and runs `f(page, st)`. */
async function onStage(browser, bin, saves, kind, f, viewport = VIEW) {
  const st = await stage(bin, kind, path.join(saves, `staged-${kind.replace(/[^a-z0-9-]/gi, "_")}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
    const page = await open3d(browser, `${server.url}/?renderer=hd2d&force3d=1`, viewport);
    try {
      await settle(page);
      await f(page, st);
    } finally {
      await page.close();
    }
  } finally {
    await server.stop();
  }
}

const typeIndex = (page, name) => page.evaluate((t) => window.ffh.S.data.tables.UnitType.findIndex((e) => e.type === t), name);

/** Opens city `id`'s screen and returns the selector of unit type `name`'s production entry. */
async function buildEntry(page, id, name) {
  await page.evaluate((id) => window.ffh.emit("open-city", id), id);
  await until(() => page.evaluate(() => !!document.querySelector(".modal.city-screen button.build")), "the city screen's production list", 30000);
  await sleep(1200);
  const i = await typeIndex(page, name);
  const sel = `.modal.city-screen button.build[data-order="0"][data-type="${i}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the entry of ${name}`, 20000);
  return sel;
}

// ---- The build list's card (the owner's example), then the type cards ----
async function shotsBuild(browser, bin, saves) {
  await onStage(browser, bin, saves, "ritual-civ:CIVILIZATION_DOVIELLO", async (page, st) => {
    if (want("build")) {
      const sel = await buildEntry(page, st.capital, "UNIT_BEASTMAN");
      await hover(page, sel, { dx: 40 });
      await crop(page, [sel, ...TIP], "build-beastman.png");
      await unhover(page, sel);
      await page.keyboard.press("Escape");
      await sleep(400);
    }
    if (want("types")) {
      for (const names of args.types ? args.types.split(",").map((n) => [n]) : TYPES) {
        let name = null;
        let i = -1;
        for (const n of names) {
          i = await typeIndex(page, n);
          if (i >= 0) {
            name = n;
            break;
          }
        }
        if (i < 0) {
          console.log(`types: none of ${names.join(", ")} in the data`);
          continue;
        }
        const sel = await typeProbe(page, i);
        await hover(page, sel, { x: 12, y: 12 });
        await crop(page, TIP, `type-${name.replace(/^UNIT_/, "").toLowerCase()}.png`);
        await unhover(page, sel);
      }
    }
  });
}

// ---- The same entry on a phone's width: the reminder boxes below the card ----
async function shotsNarrow(browser, bin, saves) {
  await onStage(
    browser,
    bin,
    saves,
    "ritual-civ:CIVILIZATION_DOVIELLO",
    async (page) => {
      const sel = await typeProbe(page, await typeIndex(page, "UNIT_BEASTMAN"));
      await hover(page, sel, { x: 12, y: 12 });
      await crop(page, TIP, "build-beastman-narrow.png", { pad: 4 });
      await unhover(page, sel);
    },
    NARROW,
  );
}

// ---- Every unit type's card fits the window ----
async function checkFit(browser, bin, saves) {
  let bad = 0;
  await onStage(
    browser,
    bin,
    saves,
    "ritual-civ:CIVILIZATION_DOVIELLO",
    async (page) => {
      const n = await page.evaluate(() => window.ffh.S.data.tables.UnitType.length);
      const sizes = [];
      for (let i = 0; i < n; i++) {
        const sel = await typeProbe(page, i);
        await page.evaluate((sel) => {
          const e = document.querySelector(sel);
          e.dispatchEvent(new MouseEvent("mouseenter", { clientX: 12, clientY: 12, bubbles: false }));
          e.dispatchEvent(new MouseEvent("mousemove", { clientX: 12, clientY: 12, bubbles: true }));
        }, sel);
        const r = await boxOf(page, TIP);
        const name = await page.evaluate((i) => window.ffh.S.data.tables.UnitType[i].type, i);
        if (r) sizes.push({ name, w: Math.round(r.x1 - r.x0), h: Math.round(r.y1 - r.y0), out: r.x0 < 0 || r.y0 < 0 || r.x1 > r.w || r.y1 > r.h });
        await page.evaluate((sel) => document.querySelector(sel)?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false })), sel);
      }
      sizes.sort((a, b) => b.h - a.h);
      console.log(`fit: ${sizes.length} cards at ${LAPTOP.width} x ${LAPTOP.height}; the tallest:`);
      for (const s of sizes.slice(0, 5)) console.log(`  ${s.name}: ${s.w} x ${s.h}`);
      const outs = sizes.filter((s) => s.out);
      bad = outs.length;
      for (const s of outs) console.log(`  DOES NOT FIT: ${s.name}: ${s.w} x ${s.h}`);
      if (sizes.length) {
        const tall = sizes[0].name;
        const sel = await typeProbe(page, await typeIndex(page, tall));
        await hover(page, sel, { x: 12, y: 12 });
        await crop(page, TIP, "tallest.png", { pad: 4 });
        await unhover(page, sel);
      }
    },
    LAPTOP,
  );
  return bad;
}

/** Selects `ids` and returns the selector of unit `id`'s chip in the unit strip. */
async function chipOf(page, ids, id) {
  await page.evaluate((ids) => window.ffh.units.select(ids, { center: true }), ids);
  const sel = `.unit-chip[data-unit="${id}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `unit ${id}'s chip`, 20000);
  await sleep(1200);
  return sel;
}

const unitTypeName = (page, id) =>
  page.evaluate((id) => {
    const u = window.ffh.S.units.get(id);
    return window.ffh.S.data.tables.UnitType[u.type].type.replace(/^UNIT_/, "").toLowerCase();
  }, id);

// ---- Units of an army standing in a squad ----
async function shotsUnit(browser, bin, saves) {
  await onStage(browser, bin, saves, "engagement", async (page, st) => {
    const seen = new Set();
    for (const id of st.units) {
      const name = await unitTypeName(page, id);
      if (seen.has(name)) continue;
      seen.add(name);
      const sel = await chipOf(page, st.units, id);
      await hover(page, sel, { dy: 4 });
      await crop(page, TIP, `unit-${name}.png`);
      await unhover(page, sel);
    }
  });
}

// ---- A caster and a wounded hero ----
async function shotsCaster(browser, bin, saves) {
  await onStage(browser, bin, saves, "heroes", async (page, st) => {
    for (const [id, name] of [
      [st.caster, "unit-caster.png"],
      [st.wounded, "unit-hero.png"],
    ]) {
      const sel = await chipOf(page, [id], id);
      await hover(page, sel, { dy: 4 });
      await crop(page, TIP, name);
      await unhover(page, sel);
    }
  });
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("unit-card-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("unit-card-shots");
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  let bad = 0;
  try {
    if (want("build") || want("types")) await shotsBuild(browser, bin, saves);
    if (want("narrow")) await shotsNarrow(browser, bin, saves);
    if (want("fit")) bad = await checkFit(browser, bin, saves);
    if (want("unit")) await shotsUnit(browser, bin, saves);
    if (want("caster")) await shotsCaster(browser, bin, saves);
  } finally {
    await browser.close();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  console.log(`unit-card-shots: written to ${path.relative(process.cwd(), OUT) || OUT}`);
  if (bad) {
    console.error(`unit-card-shots: FAILED: ${bad} card(s) do not fit ${LAPTOP.width} x ${LAPTOP.height}`);
    return 1;
  }
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`unit-card-shots: FAILED: ${e.stack ?? e}`);
    process.exit(1);
  },
);
