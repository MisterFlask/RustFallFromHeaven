#!/usr/bin/env node
// Screenshots of the world rituals' interface (docs/HUMAN.md, "World rituals"; CLAUDE.md, "Show UI
// changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 of the rituals element,
// the full list, a "began" alert, the world spell's button in each state, and the production
// dialog's ritual entry, its disabled reason and the abandon question. Regenerate them after a
// change to any of these:
//
//   node tools/world-rituals-shots.mjs --bin <built>/ffh-play [--out docs/shots/world-rituals]
//                                      [--assets crates/ffh-play/assets]
//
// It stages `ffh-play stage rituals` and `rituals-charged`, serves each with `ffh-play serve
// --load`, and drives the page with Playwright's Chromium (as tools/web-smoke.mjs). The map is the
// isometric one (headless Chromium draws WebGL in software, so the page falls back to it), the
// look a player gets without a GPU fast enough for the HD-2D map; the interface is the same.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value" });
const OUT = path.resolve(args.out ?? path.join(ROOT, "docs", "shots", "world-rituals"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1280, height: 800 };
const PHONE = { width: 390, height: 844 };

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

const q = (s) => JSON.stringify(s);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** A page on a served staged save, at the player's turn, with no toast but the ritual alerts. */
async function open(browser, url, opts = {}) {
  const page = await browser.newPage({ viewport: VIEW, deviceScaleFactor: 2, ...opts });
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  await page.goto(url);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.status?.phase === "human")), "the staged game", 60000);
  await until(() => page.evaluate(() => !!document.querySelector("#rituals:not([hidden])")), "the rituals element", 20000);
  await sleep(1500);
  await dropToasts(page, true);
  return page;
}

/** Removes the toasts (`keepRituals`: but the ritual alerts). */
function dropToasts(page, keepRituals = false) {
  return page.evaluate((keep) => {
    for (const t of document.querySelectorAll("#toasts .toast")) if (!keep || !t.classList.contains("ritual-alert")) t.remove();
  }, keepRituals);
}

/** A crop around the union of the elements `sels` match (with `pad` CSS px), at most `maxW` CSS px
 *  wide, saved as `name`. */
async function crop(page, sels, name, { pad = 8, maxW = 1000 } = {}) {
  const r = await page.evaluate((sels) => {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const s of sels) {
      for (const e of document.querySelectorAll(s)) {
        const b = e.getBoundingClientRect();
        if (!b.width || !b.height) continue;
        x0 = Math.min(x0, b.left);
        y0 = Math.min(y0, b.top);
        x1 = Math.max(x1, b.right);
        y1 = Math.max(y1, b.bottom);
      }
    }
    return x0 === Infinity ? null : { x0, y0, x1, y1, w: innerWidth, h: innerHeight };
  }, sels);
  if (!r) throw new Error(`${name}: nothing matches ${sels.join(", ")}`);
  const x = Math.max(0, Math.floor(r.x0 - pad));
  const y = Math.max(0, Math.floor(r.y0 - pad));
  const w = Math.min(maxW, Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(Math.ceil(r.y1 + pad) - y, r.h - y);
  const file = path.join(OUT, name);
  await page.screenshot({ path: file, clip: { x, y, width: w, height: h } });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` (as a pointer over it). */
async function hover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    const r = e.getBoundingClientRect();
    // At the button's lower edge: the tooltip opens under it, the button in view.
    const at = { clientX: r.left + 8, clientY: r.bottom - 6 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
  }, sel);
  await until(() => page.evaluate(() => !!document.querySelector(".tooltip.richtip:not([hidden])")), `the tooltip of ${sel}`, 10000);
  await sleep(300);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false }));
    e?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
  }, sel);
  await page.mouse.move(640, 790);
  await sleep(200);
}

/** The world-spell button and its tooltip. */
async function buttonShot(page, name) {
  const sel = "#world-actions button.world-action";
  await hover(page, sel);
  await crop(page, [sel, ".tooltip.richtip:not([hidden])"], name);
  await unhover(page, sel);
}

/** Opens city `id`'s screen and waits for its build lists. */
async function openCity(page, id) {
  await page.evaluate((id) => window.ffh.emit("open-city", id), id);
  await until(() => page.evaluate(() => !!document.querySelector(".modal.city-screen button.build.ritual-build")), "the city screen's ritual entry", 20000);
  await sleep(800);
}

/** The production column's ritual entry, scrolled into view, with the production line above. */
async function ritualEntry(page) {
  await page.evaluate(() => document.querySelector(".modal.city-screen button.build.ritual-build").scrollIntoView({ block: "center" }));
  await sleep(300);
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("world-rituals-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("rituals-shots");
  const browser = await pw.chromium.launch({ headless: true });
  try {
    // ---- `stage rituals`: rituals in progress (yours too), a charge held, one unmet ----
    const st = await stage(bin, "rituals", path.join(saves, "staged-rituals"));
    let server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
    // The rituals element on a phone: a row of its own under the bar, scrolling sideways.
    let page = await open(browser, server.url, { viewport: PHONE, isMobile: true, hasTouch: true });
    await dropToasts(page);
    await crop(page, ["#top"], "rituals-hud-phone.png", { pad: 0 });
    await page.close();
    page = await open(browser, server.url);
    // The "began" alerts of this turn, shown on load (the first one: another player's).
    await until(() => page.evaluate(() => document.querySelectorAll("#toasts .toast.ritual-alert").length >= 2), "the ritual alerts", 10000);
    await page.evaluate(() => {
      const all = [...document.querySelectorAll("#toasts .toast.ritual-alert")];
      // Another player's start (the human's own is first in the log).
      all.forEach((t, i) => i !== 1 && t.remove());
    });
    await crop(page, ["#toasts .toast.ritual-alert"], "ritual-alert.png", { pad: 10 });
    await dropToasts(page);
    // The strip of four rituals is wider than the other crops (about 1150 CSS px at 1280): the
    // phone crop below shows it at a phone's width.
    await crop(page, ["#rituals .ritual-head", "#rituals .ritual-row"], "rituals-hud.png", { pad: 10, maxW: 1240 });
    await page.click("#rituals .ritual-all");
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="rituals"] .rituals-list')), "the rituals list", 10000);
    await sleep(300);
    await crop(page, ['.modal[data-tag="rituals"]'], "rituals-panel.png", { pad: 4 });
    await page.keyboard.press("Escape");
    await sleep(300);
    await buttonShot(page, "worldspell-button-in-progress.png");
    // The capital's production: the ritual's entry and its rules line.
    await openCity(page, st.capital);
    await ritualEntry(page);
    await crop(page, [".modal.city-screen .builds:has(button.ritual-build)"], "production-ritual-entry.png", { pad: 6 });
    // Another build would take the ritual off the head: the question, Cancel focused.
    await page.evaluate(() => document.querySelector(".modal.city-screen .builds button.build[data-order='0']:not([disabled])").click());
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="abandon-ritual"]')), "the abandon question", 10000);
    const focused = await page.evaluate(() => document.activeElement?.textContent ?? "");
    if (!/Cancel/.test(focused)) throw new Error(`the abandon question has "${focused}" focused, not Cancel`);
    await sleep(300);
    await crop(page, ['.modal[data-tag="abandon-ritual"]'], "production-abandon-confirm.png", { pad: 4 });
    await page.evaluate(() => document.querySelector('.modal[data-tag="abandon-ritual"] button[data-danger]').click());
    await until(() => page.evaluate(() => !document.querySelector('.modal[data-tag="abandon-ritual"]')), "the question answered", 10000);
    // The warrior went first; the ritual, its production lost, stays behind it: taken off too.
    const second = '.modal.city-screen ol.queue button[data-index="1"]';
    await until(() => page.evaluate((s) => !!document.querySelector(s), second), "the ritual behind the warrior", 10000);
    await page.evaluate((s) => document.querySelector(s).click(), second);
    await until(() => page.evaluate(() => !document.querySelector('.modal.city-screen ol.queue button[data-index="1"]')), "the ritual off the queue", 10000);
    await sleep(500);
    await page.keyboard.press("Escape");
    await until(() => page.evaluate(() => !document.querySelector(".modal.city-screen")), "the city screen closed", 10000);
    // Abandoned: no charge, nothing queued.
    await until(() => page.evaluate(() => /No charge\. Build /.test(window.ffh.S.view.me.world_actions.find((a) => a.ritual !== null && a.ritual !== undefined)?.why ?? "")), "the ritual abandoned", 20000);
    await sleep(300);
    await dropToasts(page);
    await buttonShot(page, "worldspell-button-no-charge.png");
    await page.close();
    await server.stop();

    // ---- `stage rituals-charged`: the human holds its charge ----
    const ch = await stage(bin, "rituals-charged", path.join(saves, "staged-rituals-charged"));
    server = await startServer({ bin, saves, extra: ["--load", ch.save, "--assets", ASSETS] });
    page = await open(browser, server.url);
    await dropToasts(page);
    await buttonShot(page, "worldspell-button-charged.png");
    await openCity(page, ch.second);
    await ritualEntry(page);
    await crop(page, [".modal.city-screen button.build.ritual-build"], "production-ritual-disabled.png", { pad: 10 });
    await page.close();
    await server.stop();
  } finally {
    await browser.close();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  console.log(`world-rituals-shots: written to ${path.relative(process.cwd(), OUT) || OUT}`);
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`world-rituals-shots: FAILED: ${e.stack ?? e}`);
    process.exit(1);
  },
);
