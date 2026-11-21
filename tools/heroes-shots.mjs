#!/usr/bin/env node
// Screenshots of the dead the rules remember (ffh-core's unit/fallen.rs; CLAUDE.md, "Show UI
// changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 on the HD-2D map of the
// "has fallen" alert, the Wounded hero's line and its calculation, the Hero and Wounded
// promotions' text, Resurrection's choice of the dead, and the Armies screen's sections of heroes
// recovering and of the dead within Resurrection's reach.
//
//   node tools/heroes-shots.mjs --bin <built>/ffh-play [--out DIR] [--assets crates/ffh-play/assets]
//
// It stages `ffh-play stage heroes`, serves it with `ffh-play serve --load` and drives the page
// with Playwright's Chromium (as tools/world-rituals-shots.mjs), the map on HD-2D in software
// (`?renderer=hd2d&force3d=1`).

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "target", "heroes-shots"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1280, height: 800 };
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

/** A crop around the union of the elements `sels` match (with `pad` CSS px), saved as `name`. */
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
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: w, height: h }, timeout: 120000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` (as a pointer over it). */
async function hover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    const r = e.getBoundingClientRect();
    const at = { clientX: r.left + 8, clientY: r.top > innerHeight / 2 ? r.top + 4 : r.bottom - 6 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
    e.dispatchEvent(new FocusEvent("focus"));
  }, sel);
  await until(() => page.evaluate(() => !!document.querySelector(".tooltip:not([hidden])")), `the tooltip of ${sel}`, 10000);
  await sleep(400);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false }));
    e?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
    e?.dispatchEvent(new FocusEvent("blur"));
  }, sel);
  await page.mouse.move(640, 790);
  await sleep(300);
}

/** Selects unit `id` and waits for its panel's details. */
async function selectUnit(page, id) {
  await page.evaluate((id) => window.ffh.units.select([id], { center: true }), id);
  await until(() => page.evaluate(() => !!document.querySelector("#unit-panel button.action")), "the unit panel's actions", 30000);
  await sleep(1500);
}

async function main() {
  const pw = await loadPlaywright();
  if (!pw) return skipped("heroes-shots", "playwright not found");
  const bin = findBin(args.bin);
  if (!bin) return skipped("heroes-shots", "no ffh-play binary");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("heroes-shots");
  const st = await stage(bin, "heroes", path.join(saves, "staged-heroes"));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    const page = await browser.newPage({ viewport: VIEW, deviceScaleFactor: 2 });
    page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
    await page.goto(`${server.url}/?renderer=hd2d&force3d=1`);
    await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.status?.phase === "human")), "the staged game", 180000);
    await sleep(4000);
    for (let i = 0; i < 4 && (await page.evaluate(() => !!document.querySelector(".modal"))); i++) {
      await page.keyboard.press("Escape");
      await sleep(300);
    }
    // (1) The alert of this turn's fallen hero, if the page shows it as a toast; the message log
    // has it either way.
    const fallenToast = await page.evaluate(() => {
      const ts = [...document.querySelectorAll("#toasts .toast")];
      const t = ts.find((t) => /has fallen/.test(t.textContent));
      for (const o of ts) if (o !== t) o.remove();
      return !!t;
    });
    if (fallenToast) await crop(page, ["#toasts .toast"], "fallen-alert.png", { pad: 6 });
    else console.log("fallen-alert.png: no toast (the message is in the log)");
    await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));

    // (2) The Wounded hero: its panel line, then the line with its calculation.
    if (want("unit")) {
    await selectUnit(page, st.wounded);
    await until(() => page.evaluate(() => !!document.querySelector(".wounds-line")), "the Wounds line", 20000);
    await crop(page, ["#unit-panel"], "wounded-panel.png", { pad: 4, maxW: 700 });
    await hover(page, ".wounds-line");
    await crop(page, [".wounds-line", ".tooltip:not([hidden])"], "wounds-calc.png", { pad: 8, maxW: 600 });
    await unhover(page, ".wounds-line");

    await crop(page, ["#unit-panel .seals"], "wounded-seals.png", { pad: 6, maxW: 600 });

    // (3) The Hero and the Wounded promotions' text (their seals' tooltips).
    const seals = await page.evaluate(() => {
      const S = window.ffh.S;
      const idx = (t) => S.data.tables.PromotionType.findIndex((e) => e.type === t);
      return { hero: idx("PROMOTION_HERO"), wounded: idx("PROMOTION_WOUNDED") };
    });
    for (const [name, i] of [["hero", seals.hero], ["wounded", seals.wounded]]) {
      const sel = `.seal[data-promotion="${i}"]`;
      if (!(await page.evaluate((s) => !!document.querySelector(s), sel))) {
        console.log(`promotion-${name}.png: no seal`);
        continue;
      }
      await hover(page, sel);
      await crop(page, [".tooltip:not([hidden])"], `promotion-${name}.png`, { pad: 4, maxW: 600 });
      await unhover(page, sel);
    }

    }

    // (4) Resurrection: the Adept's choice of the dead.
    if (want("resurrection")) {
    await selectUnit(page, st.caster);
    const ok = await page.evaluate(async (caster) => {
      const S = window.ffh.S;
      const spell = S.data.tables.SpellType.findIndex((e) => e.type === "SPELL_RESSURECTION");
      const r = await window.ffh.api.query("group", { units: [caster] });
      const a = (r.actions ?? []).find((a) => a.kind === "spell" && a.spell === spell);
      if (!a) return "no Resurrection action";
      window.ffh.units.castTargeted(a);
      return null;
    }, st.caster);
    if (ok) throw new Error(ok);
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="cast-target"]')), "Resurrection's choice", 20000);
    await sleep(500);
    await crop(page, ['.modal[data-tag="cast-target"]'], "resurrection-choice.png", { pad: 4, maxW: 760 });
    await page.keyboard.press("Escape");
    await sleep(500);

    }

    // (5) The Armies screen's sections.
    // (Not awaited: the screen's promise settles when it closes.)
    await page.evaluate(() => { window.ffh.openScreen("armies"); });
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="armies"] .fallen-list')), "the Armies screen's dead", 20000);
    await sleep(500);
    await page.evaluate(() => document.querySelector('.modal[data-tag="armies"] .fallen-list').scrollIntoView({ block: "center" }));
    await sleep(300);
    await crop(page, ['.modal[data-tag="armies"] .fallen-list', '.modal[data-tag="armies"] h4:has(+ .fallen-list)'], "armies-fallen.png", { pad: 10, maxW: 760 });
    await page.close();
  } finally {
    await browser.close();
    await server.stop();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
