#!/usr/bin/env node
// Screenshots of raids as the player sees them (docs/RAIDS.md, section 8; docs/HUMAN.md, "Raids";
// CLAUDE.md, "Show UI changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 on the
// HD-2D map (`?renderer=hd2d&force3d=1`; SwiftShader here unless --gpu).
//
//   node tools/raids-shots.mjs --bin <built>/ffh-play [--out docs/shots/raids]
//                              [--assets crates/ffh-play/assets] [--gpu] [--only order,overrun,contested]
//
// - `order` (`stage raid-order`, the human's army on a cottage of an enemy city's countryside):
//   raid-order.png (the unit panel's `Raid <city>` with its tooltip: the take and its
//   calculation), pillage-refused.png (the pillage order disabled on the fields, its reason in the
//   card's description), then the order taken: raid-army-line.png (the panel's army line,
//   `Raiding <city>.`, and the raider's supply), raid-stop.png (Stop raiding with its tooltip) and
//   armies-raiding.png (the Armies screen's row).
// - `overrun` (`stage raid-overrun`, the human's city raided, its raid step run): raid-plate.png
//   (the HD-2D plate `<city> · Overrun` ringed in amber), city-raid.png (the city screen's Raid
//   section with its calculation's tooltip open), city-sally.png (the Sally selector) and
//   city-panel-raid.png (the compact city panel's line).
// - `contested` (`stage raid-contested`): raid-plate-contested.png.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value", gpu: "flag" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "docs", "shots", "raids"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1100, height: 880 };
const TIP = [".tooltip:not([hidden])", ".tip-side:not([hidden])"];

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

/** A crop around the union of what `sels` match (with `pad` CSS px), at most `maxW` wide. */
async function crop(page, sels, name, { pad = 8, maxW = 560 } = {}) {
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

/** A crop of `w` x `h` CSS px centred on the viewport's point (cx, cy). */
async function cropAt(page, cx, cy, w, h, name) {
  const x = Math.max(0, Math.round(cx - w / 2));
  const y = Math.max(0, Math.round(cy - h / 2));
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: Math.min(w, VIEW.width - x), height: Math.min(h, VIEW.height - y) }, timeout: 300000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip (or the card's description) of `sel` as a pointer over it. */
async function hover(page, sel, { tip = true } = {}) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e.scrollIntoView({ block: "center" });
    const r = e.getBoundingClientRect();
    const at = { clientX: r.left + 10, clientY: r.top > innerHeight / 2 ? r.top + 4 : r.bottom - 6 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
    e.dispatchEvent(new MouseEvent("mousemove", { ...at, bubbles: true }));
  }, sel);
  if (tip) await until(() => page.evaluate(() => [...document.querySelectorAll(".tooltip, .tip-side")].some((e) => !e.hidden)), `the tooltip of ${sel}`, 15000);
  await sleep(600);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false }));
    e?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
  }, sel);
  await page.mouse.move(4, 4);
  await sleep(300);
}

/** Closes what the load opened and drops the toasts. */
async function settle(page) {
  for (let i = 0; i < 6; i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
  await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
}

/** Serves staged game `kind` on the HD-2D map and runs `f(page, st)`. */
async function onStage(browser, bin, saves, kind, f) {
  const st = await stage(bin, kind, path.join(saves, `staged-${kind}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
    const page = await browser.newPage({ viewport: VIEW, deviceScaleFactor: 2 });
    page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
    page.on("console", (m) => m.type() === "error" && console.error(`console: ${m.text()}`));
    try {
      await page.goto(`${server.url}/?renderer=hd2d&force3d=1`);
      await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 240000, 250);
      await sleep(3000);
      await settle(page);
      await f(page, st);
    } finally {
      await page.close();
    }
  } finally {
    await server.stop();
  }
}

/** Selects `units` (centred) and waits for the panel's actions. */
async function selectUnits(page, units) {
  await page.evaluate((ids) => window.ffh.units.select(ids, { center: true }), units);
  await until(() => page.evaluate(() => !!document.querySelector("#unit-panel button.action")), "the unit panel's actions", 30000);
  await sleep(1500);
}

/** Centres the map on (x, y) and waits for frames. */
async function centre(page, x, y, wait) {
  await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), [x, y]);
  await sleep(wait);
}

const frame = () => (args.gpu ? 2500 : 25000);

// ---- `raid-order`: the order, the pillage refused, the order taken ----
async function shotsOrder(browser, bin, saves) {
  await onStage(browser, bin, saves, "raid-order", async (page, st) => {
    await selectUnits(page, st.units);
    const raid = '#unit-panel button.action[data-kind="army"][data-type="raid"]';
    await until(() => page.evaluate((s) => !!document.querySelector(s), raid), "the Raid action", 20000);
    await hover(page, raid);
    await crop(page, [raid, ...TIP], "raid-order.png", { maxW: 700 });
    await unhover(page, raid);
    // The pillage order on the fields: disabled, its reason in the card's description box (a tile)
    // or its tooltip (a listed button on a touch screen).
    const pillage = '#unit-panel button.action[data-kind="mission"][data-type="MISSION_PILLAGE"]';
    await until(() => page.evaluate((s) => !!document.querySelector(s), pillage), "the pillage order", 20000);
    await hover(page, pillage, { tip: false });
    await crop(page, [pillage, "#unit-panel .iso-desc-one.shown"], "pillage-refused.png", { maxW: 700 });
    await unhover(page, pillage);
    // Taken: a real click.
    await page.click(raid);
    await until(() => page.evaluate(() => !!document.querySelector("#unit-panel .raid-station-line")), "the army's Raid line", 30000);
    await sleep(1200);
    await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
    await crop(page, ["#unit-panel .army-line"], "raid-army-line.png", { maxW: 700 });
    const stop = '#unit-panel button.action[data-type="stop_raid"]';
    await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "center" }), stop);
    await hover(page, stop);
    await crop(page, [stop, ...TIP], "raid-stop.png", { maxW: 700 });
    await unhover(page, stop);
    // The Armies screen's row (not awaited: the screen's promise settles when it closes).
    await page.evaluate(() => {
      window.ffh.openScreen("armies");
    });
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="armies"] .raid-station')), "the Armies screen's Raid line", 20000);
    await sleep(600);
    await page.evaluate(() => document.querySelector('.modal[data-tag="armies"] .raid-station').scrollIntoView({ block: "center" }));
    await sleep(300);
    const card = await page.evaluate(() => {
      const c = document.querySelector('.modal[data-tag="armies"] .raid-station').closest(".army-card");
      c.id = "raid-card";
      return !!c;
    });
    if (!card) throw new Error("no army card");
    await crop(page, ["#raid-card .army-title", "#raid-card .raid-station", "#raid-card .army-supply"], "armies-raiding.png", { maxW: 760 });
  });
}

/** The plate of city (x, y) on the HD-2D map: centred, the frame waited for, a crop around it. */
async function plate(page, st, name) {
  await centre(page, st.cx, st.cy, frame());
  // The capital's star on the plate, where the HD-2D map drew it (r3d/hud.js's `plateHits`).
  const star = await page.evaluate((c) => window.ffh.mapView.r3d?.plateHits?.find((x) => x.kind === "star" && x.city === c) ?? null, st.city);
  if (star) await cropAt(page, star.x0 + 80, star.y0 + 10, 380, 220, name);
  else await cropAt(page, VIEW.width / 2, VIEW.height / 2, 420, 260, name);
}

// ---- `raid-overrun`: the plate, the city screen, the compact panel ----
async function shotsOverrun(browser, bin, saves) {
  await onStage(browser, bin, saves, "raid-overrun", async (page, st) => {
    await plate(page, st, "raid-plate.png");
    // The compact city panel (the plot's city selected on the HD-2D map).
    await page.evaluate((c) => window.ffh.emit("open-city", c), st.city);
    await until(() => page.evaluate(() => !!document.querySelector(".modal.city-screen section.raid-status")), "the city screen's Raid section", 30000);
    await sleep(1500);
    const take = ".modal.city-screen section.raid-status .raid-take";
    await hover(page, take);
    await crop(page, [".modal.city-screen section.raid-status", ...TIP], "city-raid.png", { maxW: 760 });
    await unhover(page, take);
    const sally = ".modal.city-screen section.sally";
    await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "center" }), sally);
    await sleep(400);
    await crop(page, [sally], "city-sally.png", { maxW: 700 });
    await page.keyboard.press("Escape");
    await sleep(800);
    const panel = await page.evaluate(() => !!document.querySelector("#iso-city:not([hidden]) p.raid-status"));
    if (panel) await crop(page, ["#iso-city h3.iso-title", "#iso-city p.raid-status"], "city-panel-raid.png", { maxW: 500 });
    else console.log("city-panel-raid.png: no compact city panel shown");
  });
}

// ---- `raid-contested`: the plate ----
async function shotsContested(browser, bin, saves) {
  await onStage(browser, bin, saves, "raid-contested", async (page, st) => {
    await plate(page, st, "raid-plate-contested.png");
  });
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("raids-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("raids-shots");
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  try {
    if (want("order")) await shotsOrder(browser, bin, saves);
    if (want("overrun")) await shotsOverrun(browser, bin, saves);
    if (want("contested")) await shotsContested(browser, bin, saves);
  } finally {
    await browser.close();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  console.log(`raids-shots: written to ${path.relative(process.cwd(), OUT) || OUT}`);
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`raids-shots: FAILED: ${e.stack ?? e}`);
    process.exit(1);
  },
);
