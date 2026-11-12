#!/usr/bin/env node
// Screenshots of the world rituals' interface (docs/HUMAN.md, "World rituals"; CLAUDE.md, "Show UI
// changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 of the rituals element,
// the full list, a "began" alert, the world spell's button in each state, the production dialog's
// ritual entry, its disabled reason and the abandon question; The Tithe's rival naming (the
// button, the list of rivals, the question, the production entry before and after); Sea Ice on
// the HD-2D and the isometric map; and the text of each civilization's new ritual effect as the
// game shows it (`text-*.png`: the top bar's world action, a unit's action or a production entry,
// with its tooltip). Regenerate them after a change to any of these:
//
//   node tools/world-rituals-shots.mjs --bin <built>/ffh-play [--out docs/shots/world-rituals]
//                                      [--assets crates/ffh-play/assets]
//                                      [--only rituals,charged,tithe,ice,ice3d,civs,wellspring3d]
//
// It stages `ffh-play stage rituals`, `rituals-charged`, `tithe`, `ice` and
// `ritual-civ:CIVILIZATION_X`, serves each with `ffh-play serve --load`, and drives the page with
// Playwright's Chromium (as tools/web-smoke.mjs). The map is the isometric one (headless Chromium
// without WebGL in software falls back to it), the look a player gets without a GPU fast enough for
// the HD-2D map; the interface is the same. `ice3d` and `wellspring3d` (Wellspring's plot pick
// and mana list, the world spell's caster line) draw the HD-2D map in software (SwiftShader,
// `?renderer=hd2d&force3d=1`): minutes, not seconds.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
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
async function crop(page, sels, name, { pad = 8, maxW = 1000, timeout = 30000 } = {}) {
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
  await page.screenshot({ path: file, clip: { x, y, width: w, height: h }, timeout });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` (as a pointer over it). */
async function hover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    const r = e.getBoundingClientRect();
    // At the button's lower edge (the tooltip opens under it, the button in view); in the lower
    // half of the page at its upper edge (the tooltip opens above it).
    const at = { clientX: r.left + 8, clientY: r.top > innerHeight / 2 ? r.top + 4 : r.bottom - 6 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
  }, sel);
  await until(() => page.evaluate(() => !!document.querySelector(".tooltip.richtip:not([hidden])")), `the tooltip of ${sel}`, 10000);
  await sleep(300);
  // Placed again once its text is laid out (it flips above the pointer when it does not fit under).
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    const r = e.getBoundingClientRect();
    e.dispatchEvent(new MouseEvent("mousemove", { clientX: r.left + 8, clientY: r.top > innerHeight / 2 ? r.top + 4 : r.bottom - 6, bubbles: true }));
  }, sel);
  await sleep(200);
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

/** The index of `type` in data table `kind` (as the page knows it). */
function typeIndex(page, kind, type) {
  return page.evaluate(([k, t]) => window.ffh.S.data.tables[k].findIndex((e) => e.type === t), [kind, type]);
}

/** The top bar's world action for spell `type`: its selector. */
async function worldActionSel(page, type) {
  const i = await typeIndex(page, "SpellType", type);
  if (i < 0) throw new Error(`no spell ${type}`);
  const sel = `#world-actions button.world-action[data-spell="${i}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the world action ${type}`, 20000);
  return sel;
}

/** Element `sel` and its tooltip (`only`: the tooltip alone), saved as `name`. */
async function tipShot(page, sel, name, { only = false, ...opts } = {}) {
  await hover(page, sel);
  await crop(page, only ? [".tooltip.richtip:not([hidden])"] : [sel, ".tooltip.richtip:not([hidden])"], name, { maxW: 560, ...opts });
  await unhover(page, sel);
}

/** The text of an effect: the tooltip of `sel`, alone. */
const textShot = (page, sel, name) => tipShot(page, sel, name, { only: true, pad: 2 });

/** Runs `f` with the page at a phone-like width (the dialogs then fit a narrow crop). */
async function narrow(page, f) {
  await page.setViewportSize({ width: 500, height: VIEW.height });
  await sleep(500);
  try {
    await f();
  } finally {
    await page.setViewportSize(VIEW);
    await sleep(500);
  }
}

/** Ends the turn (past the end-turn checks) and waits for the next one. */
async function endTurn(page) {
  const turn = await page.evaluate(() => window.ffh.S.view.turn);
  await page.evaluate(() => document.getElementById("end-turn").click());
  await sleep(800);
  const force = '.modal[data-tag="gate"] button[data-value="force"]';
  if (await page.evaluate((s) => !!document.querySelector(s), force)) await page.evaluate((s) => document.querySelector(s).click(), force);
  await until(() => page.evaluate((t) => window.ffh.S.view?.turn > t && window.ffh.S.status?.phase === "human", turn), "the next turn", 120000, 250);
  await sleep(1500);
  for (let i = 0; i < 6 && (await page.evaluate(() => !!document.querySelector(".modal"))); i++) {
    await page.keyboard.press("Escape");
    await sleep(300);
  }
  await dropToasts(page);
}

/** The production entry of project `type` in the open city screen: its selector, scrolled into
 *  view. */
async function projectEntrySel(page, type) {
  const i = await typeIndex(page, "ProjectType", type);
  const sel = `.modal.city-screen button.build.ritual-build[data-type="${i}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the production entry ${type}`, 20000);
  await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "center" }), sel);
  await sleep(300);
  return sel;
}

/** Closes the city screen. */
async function closeCity(page) {
  await page.keyboard.press("Escape");
  await until(() => page.evaluate(() => !document.querySelector(".modal.city-screen")), "the city screen closed", 10000);
  await sleep(200);
}

/** Serves staged game `kind` and runs `f(page, st)` on a page on it (`viewport`, `url` a query). */
async function onStage(browser, bin, saves, kind, f, { viewport = VIEW, query = "", ready = open } = {}) {
  const st = await stage(bin, kind, path.join(saves, `staged-${kind.replace(/[^a-z0-9-]/gi, "_")}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
    const page = await ready(browser, `${server.url}/${query}`, { viewport });
    try {
      // A popup on load (the research choice of a staged duel) is put off.
      for (let i = 0; i < 4 && (await page.evaluate(() => !!document.querySelector(".modal"))); i++) {
        await page.keyboard.press("Escape");
        await sleep(300);
      }
      await f(page, st);
    } finally {
      await page.close();
    }
  } finally {
    await server.stop();
  }
}

// ---- `stage rituals`: rituals in progress (yours too), a charge held, one unmet ----
async function shotsRituals(browser, bin, saves) {
  const st = await stage(bin, "rituals", path.join(saves, "staged-rituals"));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
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
    // phone crop above shows it at a phone's width.
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
    await closeCity(page);
    // Abandoned: no charge, nothing queued.
    await until(() => page.evaluate(() => /No charge\. Build /.test(window.ffh.S.view.me.world_actions.find((a) => a.ritual !== null && a.ritual !== undefined)?.why ?? "")), "the ritual abandoned", 20000);
    await sleep(300);
    await dropToasts(page);
    await buttonShot(page, "worldspell-button-no-charge.png");
    await page.close();
  } finally {
    await server.stop();
  }
}

// ---- `stage rituals-charged`: the human holds its charge (in its capital) ----
async function shotsCharged(browser, bin, saves) {
  await onStage(browser, bin, saves, "rituals-charged", async (page, ch) => {
    await dropToasts(page);
    await buttonShot(page, "worldspell-button-charged.png");
    await openCity(page, ch.capital);
    await ritualEntry(page);
    await crop(page, [".modal.city-screen button.build.ritual-build"], "production-ritual-disabled.png", { pad: 10 });
  });
}

// ---- `stage tithe`: the Calabim name a rival for Ritual: The Tithe ----
async function shotsTithe(browser, bin, saves) {
  await onStage(browser, bin, saves, "tithe", async (page, st) => {
    await dropToasts(page);
    const btn = await worldActionSel(page, "SPELL_NAME_THE_TITHE");
    // (a) The top bar's action and its rules.
    await tipShot(page, btn, "tithe-button.png");
    await textShot(page, btn, "text-name-the-tithe.png");
    // (d) The production entry before a rival is named: listed, disabled, the reason.
    await openCity(page, st.capital);
    let entry = await projectEntrySel(page, "PROJECT_RITUAL_THE_TITHE");
    if (!(await page.evaluate((s) => document.querySelector(s).disabled, entry))) throw new Error("Ritual: The Tithe is enabled before a rival is named");
    await crop(page, [entry], "tithe-production-unnamed.png", { pad: 10 });
    // Its rules, as the entry's tooltip.
    await textShot(page, entry, "text-ritual-the-tithe.png");
    await closeCity(page);
    // (b) The rivals to choose from.
    const pick = async () => {
      await page.evaluate((s) => document.querySelector(s).click(), btn);
      await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="cast-target"]')), "the list of rivals", 10000);
      await sleep(300);
    };
    const choose = async (re) => {
      const ok = await page.evaluate((src) => {
        const b = [...document.querySelectorAll('.modal[data-tag="cast-target"] button.option')].find((e) => new RegExp(src).test(e.textContent));
        b?.click();
        return !!b;
      }, re.source);
      if (!ok) throw new Error(`no rival matches ${re}`);
      await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="cast-confirm"]')), "the question", 10000);
      const focused = await page.evaluate(() => document.activeElement?.textContent ?? "");
      if (!/Cancel/.test(focused)) throw new Error(`the question has "${focused}" focused, not Cancel`);
      await sleep(300);
    };
    const answer = async (value) => {
      await page.evaluate((v) => document.querySelector(`.modal[data-tag="cast-confirm"] button${v ? "[data-danger]" : ":not([data-danger]):not(.close)"}`).click(), value);
      await until(() => page.evaluate(() => !document.querySelector('.modal[data-tag="cast-confirm"]')), "the question answered", 10000);
    };
    // At a phone's width: the dialogs fit a narrow crop.
    await narrow(page, async () => {
      await pick();
      await crop(page, ['.modal[data-tag="cast-target"]'], "tithe-pick.png", { pad: 4 });
      // (c) The question, for a rival at war (no declaration) and at peace (the war it declares).
      await choose(/at war/);
      await crop(page, ['.modal[data-tag="cast-confirm"]'], "tithe-confirm-at-war.png", { pad: 4 });
      await answer(false);
      await pick();
      await choose(/at peace/);
      await crop(page, ['.modal[data-tag="cast-confirm"]'], "tithe-confirm.png", { pad: 4 });
      await answer(true);
    });
    await until(() => page.evaluate(() => !document.querySelector('.modal[data-tag="cast-target"]')), "the list closed", 10000);
    // Named: a modal the declaration raised (none expected) is closed.
    await sleep(1500);
    for (let i = 0; i < 4 && (await page.evaluate(() => !!document.querySelector(".modal"))); i++) {
      await page.keyboard.press("Escape");
      await sleep(300);
    }
    await dropToasts(page);
    // (e) The production entry once named: enabled, the rival named; then queued.
    await openCity(page, st.capital);
    entry = await projectEntrySel(page, "PROJECT_RITUAL_THE_TITHE");
    await until(() => page.evaluate((s) => !document.querySelector(s).disabled, entry), "Ritual: The Tithe enabled", 10000);
    await crop(page, [entry], "tithe-production-named.png", { pad: 10 });
    await page.evaluate((s) => document.querySelector(s).click(), entry);
    await until(() => page.evaluate(() => !!window.ffh.S.view?.rituals?.find((r) => r.mine && r.state !== "idle")), "the ritual queued", 20000);
    await sleep(500);
    if (await page.evaluate(() => !!document.querySelector(".modal.city-screen"))) await closeCity(page);
    // A turn later: production in it.
    await endTurn(page);
    await until(() => page.evaluate(() => !!window.ffh.S.view?.rituals?.find((r) => r.mine && r.state === "in_progress")), "the ritual in progress", 20000);
    await page.click("#rituals .ritual-all");
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="rituals"] .rituals-list')), "the rituals list", 10000);
    await sleep(300);
    await crop(page, ['.modal[data-tag="rituals"] .modal-head', '.modal[data-tag="rituals"] li.ritual-entry.mine'], "tithe-rituals-panel.png", { pad: 4 });
    await page.keyboard.press("Escape");
    await sleep(300);
    await crop(page, ["#rituals .ritual-head", "#rituals .ritual-row.mine"], "tithe-rituals-hud.png", { pad: 10 });
  });
}

/** Selects unit `id` and waits for its action `type` (a spell) in the unit panel: its selector. */
async function unitAction(page, id, type) {
  // Taller: the unit panel's spells and a tooltip above them fit on the page.
  await page.setViewportSize({ width: VIEW.width, height: 1100 });
  await page.evaluate((id) => window.ffh.units.select([id]), id);
  const sel = `#unit-panel button.action[data-type="${type}"]`;
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the action ${type}`, 20000);
  await sleep(500);
  // The card's lists scroll: the action in view.
  await page.evaluate((s) => document.querySelector(s).scrollIntoView({ block: "center" }), sel);
  await sleep(300);
  return sel;
}

// ---- `stage ice` on the isometric map: Sea Ice, Icebreak, Frozen Passage ----
async function shotsIce(browser, bin, saves) {
  await onStage(browser, bin, saves, "ice", async (page, st) => {
    await dropToasts(page);
    await textShot(page, await worldActionSel(page, "SPELL_FROZEN_PASSAGE"), "text-frozen-passage.png");
    const sel = await unitAction(page, st.ship, "SPELL_ICEBREAK");
    await textShot(page, sel, "text-icebreak.png");
    // The patch, the walker on it, and the plot's tooltip.
    await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), st.target);
    await sleep(1500);
    // The plot below the target's (the walker's label stays clear of the pointer), in page px.
    const at = await page.evaluate(([x, y]) => {
      const mv = window.ffh.mapView;
      const p = mv.screenOf(x, y - 1);
      const r = mv.wrap.getBoundingClientRect();
      return { x: r.left + p.x, y: r.top + p.y };
    }, st.target);
    await page.evaluate(() => window.ffh.units.select([]));
    await page.mouse.move(at.x, at.y);
    await until(() => page.evaluate(() => { const t = window.ffh.mapView.tip; return !t.hidden && /Sea Ice/.test(t.textContent); }), "the plot tooltip", 15000);
    await sleep(500);
    // The patch around the pointer, and the whole tooltip.
    const tip = await page.evaluate(() => window.ffh.mapView.tip.getBoundingClientRect().toJSON());
    const x0 = Math.max(0, Math.round(Math.min(at.x - 200, tip.left - 8)));
    const y0 = Math.max(0, Math.round(Math.min(at.y - 180, tip.top - 8)));
    const x1 = Math.round(Math.max(at.x + 120, tip.right + 8));
    const y1 = Math.round(Math.max(at.y + 60, tip.bottom + 8));
    const clip = { x: x0, y: y0, width: Math.min(x1, VIEW.width) - x0, height: Math.min(y1, VIEW.height) - y0 };
    await page.screenshot({ path: path.join(OUT, "sea-ice-iso.png"), clip });
    console.log(`sea-ice-iso.png: ${clip.width} x ${clip.height} CSS px`);
  });
}

/** A page on a served staged save drawn by the HD-2D renderer in software (slow). */
async function open3d(browser, url, opts = {}) {
  const page = await browser.newPage({ ...opts, viewport: { width: 1000, height: 720 }, deviceScaleFactor: 2 });
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  await page.goto(url);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 180000, 250);
  for (let i = 0; i < 5; i++) {
    await page.keyboard.press("Escape");
    await sleep(200);
  }
  await dropToasts(page);
  return page;
}

// ---- `stage ice` on the HD-2D map (SwiftShader: 10-30 s a frame) ----
async function shotsIce3d(bin, saves, pw) {
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    await onStage(
      browser,
      bin,
      saves,
      "ice",
      async (page, st) => {
        const [tx, ty] = st.target;
        const shipAt = await page.evaluate((id) => {
          const u = (window.ffh.S.view.units ?? []).find((u) => u.id === id);
          return u ? [u.x, u.y] : null;
        }, st.ship);
        const mid = shipAt ? [Math.round((tx + shipAt[0]) / 2), Math.round((ty + shipAt[1]) / 2)] : [tx, ty];
        for (let i = 0; i < 2; i++) {
          await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), mid);
          await sleep(30000);
        }
        // The westmost ice plot on screen, in the middle band, for the tooltip.
        const s = await page.evaluate(() => {
          const m = window.ffh.S.map;
          let best = null;
          for (let i = 0; i < m.w * m.h; i++) {
            if (m.sea_ice[i] !== 1) continue;
            const p = window.ffh.mapView.screenOf(i % m.w, Math.floor(i / m.w));
            if (p && p.x >= 175 && p.y > 200 && p.y < 450 && (!best || p.x < best.x)) best = p;
          }
          return best;
        });
        if (!s) throw new Error("no Sea Ice plot on screen");
        await page.mouse.move(s.x, s.y);
        await until(() => page.evaluate(() => /Sea Ice/.test(document.querySelector(".tooltip:not([hidden])")?.textContent ?? "")), "the plot tooltip", 120000, 500);
        await sleep(20000);
        await page.screenshot({ path: path.join(OUT, "sea-ice-map.png"), clip: { x: 140, y: 150, width: 500, height: 400 }, timeout: 300000 });
        console.log("sea-ice-map.png: 500 x 400 CSS px");
        // The ship selected: Icebreak among its actions.
        await page.mouse.move(300, 690);
        await page.evaluate((id) => window.ffh.units.select([id]), st.ship);
        await until(() => page.evaluate(() => !!document.querySelector('#unit-panel button.action[data-type="SPELL_ICEBREAK"]')), "Icebreak offered", 120000, 500);
        await sleep(30000);
        await page.screenshot({ path: path.join(OUT, "sea-ice-icebreak.png"), clip: { x: 110, y: 190, width: 480, height: 530 }, timeout: 300000 });
        console.log("sea-ice-icebreak.png: 480 x 530 CSS px");
      },
      { query: "?renderer=hd2d&force3d=1", ready: open3d },
    );
  } finally {
    await browser.close();
  }
}

// ---- `stage ritual-civ:CIVILIZATION_AMURITES` on the HD-2D map: Wellspring picked by plot, then
// by mana; the world spell's caster line (SwiftShader: 10-30 s a frame) ----
async function shotsWellspring3d(bin, saves, pw) {
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    await onStage(
      browser,
      bin,
      saves,
      "ritual-civ:CIVILIZATION_AMURITES",
      async (page, st) => {
        const cap = await page.evaluate((id) => {
          const c = (window.ffh.S.view.cities ?? []).find((c) => c.id === id);
          return c ? [c.x, c.y] : null;
        }, st.capital);
        const btn = await worldActionSel(page, "SPELL_WELLSPRING");
        for (let i = 0; i < 2; i++) {
          await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), cap);
          await sleep(30000);
        }
        // The tooltip: "Cast by your Adept in <capital>; then pick its target (...)".
        await hover(page, btn);
        await sleep(15000);
        await crop(page, [btn, ".tooltip.richtip:not([hidden])"], "worldspell-caster-line.png", { maxW: 520, pad: 6, timeout: 300000 });
        await unhover(page, btn);
        // Step a: the valid plots outlined on the map, the prompt in its toast (kept on the page
        // past its 6 s: a frame takes longer than that here).
        await page.evaluate(() => {
          const box = document.getElementById("toasts");
          window.__keepPrompt = new MutationObserver((ms) => {
            for (const m of ms) for (const n of m.removedNodes) if (/pick a plot/.test(n.textContent ?? "") && !n.isConnected) n.classList.remove("leaving"), box.appendChild(n);
          });
          window.__keepPrompt.observe(box, { childList: true });
        });
        await page.evaluate((s) => document.querySelector(s).click(), btn);
        await until(() => page.evaluate(() => window.ffh.units.currentMode?.()?.cast === true), "the plot pick", 20000);
        await sleep(40000);
        const box = await page.evaluate(() => {
          const a = window.ffh.units.currentMode().action;
          const seen = new Set();
          let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
          for (const t of a.targets) {
            const k = t.at.join(",");
            if (seen.has(k)) continue;
            seen.add(k);
            const p = window.ffh.mapView.screenOf(t.at[0], t.at[1]);
            if (!p || p.x < 0 || p.y < 0 || p.x > innerWidth || p.y > innerHeight) continue;
            x0 = Math.min(x0, p.x), y0 = Math.min(y0, p.y), x1 = Math.max(x1, p.x), y1 = Math.max(y1, p.y);
          }
          const toast = [...document.querySelectorAll("#toasts .toast")].find((e) => /pick a plot/.test(e.textContent))?.getBoundingClientRect().toJSON() ?? null;
          return { x0, y0, x1, y1, toast, plots: seen.size };
        });
        console.log(`wellspring: ${box.plots} plots; on screen ${JSON.stringify(box)}`);
        // The plots and the prompt's toast above them (at the top of the map, where it shows).
        const t = box.toast ?? { left: box.x0, right: box.x1, top: box.y0 - 60 };
        const mx = (Math.min(t.left, box.x0) + Math.max(t.right, box.x1)) / 2;
        const clip = { x: Math.max(0, Math.round(mx - 250)), y: Math.max(0, Math.round(Math.min(t.top, box.y0 - 60) - 10)), width: 500, height: 430 };
        await page.mouse.move(5, 715);
        await sleep(20000);
        await page.screenshot({ path: path.join(OUT, "wellspring-pick-plot.png"), clip, timeout: 300000 });
        await page.evaluate(() => window.__keepPrompt.disconnect());
        console.log(`wellspring-pick-plot.png: 500 x 430 CSS px (prompt ${box.toast ? "shown" : "MISSING"})`);
        // Step b: a plot picked (the one nearest the capital), its manas listed.
        const plot = await page.evaluate(([cx, cy]) => {
          const a = window.ffh.units.currentMode().action;
          return a.targets.map((t) => t.at).sort((p, q) => Math.max(Math.abs(p[0] - cx), Math.abs(p[1] - cy)) - Math.max(Math.abs(q[0] - cx), Math.abs(q[1] - cy)))[0];
        }, cap);
        await page.setViewportSize({ width: 500, height: 900 });
        await page.evaluate(([x, y]) => window.ffh.units.click(x, y, {}), plot);
        await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="cast-target"]')), "the Wellspring manas", 20000);
        await sleep(15000);
        // The resize's notice (the HD-2D map drawn at half resolution here) is not this dialog's.
        await dropToasts(page);
        await sleep(15000);
        await crop(page, ['.modal[data-tag="cast-target"]'], "wellspring-pick-mana.png", { pad: 4, maxW: 500, timeout: 300000 });
        await page.keyboard.press("Escape");
      },
      { query: "?renderer=hd2d&force3d=1", ready: open3d },
    );
  } finally {
    await browser.close();
  }
}

// ---- `stage ritual-civ:CIVILIZATION_X`: each new effect's text as the game shows it ----
async function shotsCivs(browser, bin, saves) {
  const civ = (name, f) => onStage(browser, bin, saves, `ritual-civ:CIVILIZATION_${name}`, async (page, st) => (await dropToasts(page), f(page, st)));
  await civ("LANUN", async (page) => {
    await textShot(page, await worldActionSel(page, "SPELL_RAGING_SEAS"), "text-raging-seas.png");
    await textShot(page, await worldActionSel(page, "SPELL_STEER_THE_STORM"), "text-steer-the-storm.png");
  });
  await civ("KHAZAD", async (page, st) => {
    await textShot(page, await worldActionSel(page, "SPELL_THE_DELVE"), "text-the-delve.png");
    await textShot(page, await unitAction(page, st.unit, "SPELL_ENTER_THE_DELVE"), "text-enter-the-delve.png");
  });
  await civ("DOVIELLO", async (page) => {
    await textShot(page, await worldActionSel(page, "SPELL_THE_CULL"), "text-the-cull.png");
  });
  await civ("AMURITES", async (page) => {
    const btn = await worldActionSel(page, "SPELL_WELLSPRING");
    await textShot(page, btn, "text-wellspring.png");
    // The plot pick, then the manas: on the HD-2D map (`wellspring3d`).
  });
  await civ("LUCHUIRP", async (page, st) => {
    await openCity(page, st.capital);
    await textShot(page, await projectEntrySel(page, "PROJECT_RITUAL_UPGRADE"), "text-ritual-upgrade.png");
  });
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("world-rituals-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("rituals-shots");
  const browser = await pw.chromium.launch({ headless: true });
  try {
    if (want("rituals")) await shotsRituals(browser, bin, saves);
    if (want("charged")) await shotsCharged(browser, bin, saves);
    if (want("tithe")) await shotsTithe(browser, bin, saves);
    if (want("ice")) await shotsIce(browser, bin, saves);
    if (want("civs")) await shotsCivs(browser, bin, saves);
    if (want("ice3d")) await shotsIce3d(bin, saves, pw);
    if (want("wellspring3d")) await shotsWellspring3d(bin, saves, pw);
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
