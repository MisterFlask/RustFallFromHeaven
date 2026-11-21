#!/usr/bin/env node
// Screenshots of this game's Houses (docs/HOUSES.md, rule 15; CLAUDE.md, "Show UI changes in a
// screenshot"): phone-readable crops at deviceScaleFactor 2 on the HD-2D map of the pick dialog
// (a home House, a foreign House with the outsider's card, the same at a phone's width), the
// Houses screen, a card's opinion tooltip with its calculation, the city screen's House block,
// the war question with its Houses preview, the plates' shields on the map, a House alert and the
// diplomacy row of a rival's Houses.
//
//   node tools/houses-shots.mjs --bin <built>/ffh-play [--out DIR] [--assets crates/ffh-play/assets] [--only a,b]
//
// It stages `ffh-play stage houses` and `houses-foreign`, serves each with `ffh-play serve --load`
// and drives the page with Playwright's Chromium (as tools/heroes-shots.mjs), the map on HD-2D in
// software (`?renderer=hd2d&force3d=1`) for the alert and the plates; the dialogs over the flat map
// (their crops show none of it). Parts (`--only`): home, map, screen, city, diplo, foreign.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "shots", "houses"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1280, height: 800 };
const PHONE = { width: 390, height: 844 };
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
        if (!b.width || !b.height || e.hidden) continue;
        x0 = Math.min(x0, b.left);
        y0 = Math.min(y0, b.top);
        x1 = Math.max(x1, b.right);
        y1 = Math.max(y1, b.bottom);
      }
    }
    return x0 === Infinity ? null : { x0, y0, x1, y1, w: innerWidth, h: innerHeight };
  }, sels);
  if (!r) throw new Error(`${name}: nothing matches ${sels.join(", ")}`);
  await clip(page, r, name, { pad, maxW });
}

async function clip(page, r, name, { pad = 8, maxW = 1000 } = {}) {
  const vw = r.w ?? page.viewportSize().width;
  const vh = r.h ?? page.viewportSize().height;
  const x = Math.max(0, Math.floor(r.x0 - pad));
  const y = Math.max(0, Math.floor(r.y0 - pad));
  const w = Math.min(maxW, Math.ceil(r.x1 + pad) - x, vw - x);
  const h = Math.min(Math.ceil(r.y1 + pad) - y, vh - y);
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: w, height: h }, timeout: 180000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` (as a pointer over it). */
async function hover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    const r = e.getBoundingClientRect();
    const at = { clientX: r.left + 12, clientY: r.bottom - 4 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
  }, sel);
  await until(() => page.evaluate(() => [...document.querySelectorAll(".richtip, .tooltip")].some((b) => !b.hidden)), `the tooltip of ${sel}`, 10000);
  await sleep(400);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false }));
    e?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
    document.querySelectorAll(".richtip, .tip-side").forEach((b) => (b.hidden = true));
  }, sel);
  await sleep(200);
}

async function closeModals(page) {
  for (let i = 0; i < 6 && (await page.evaluate(() => !!document.querySelector("#modals .modal:not(.decision)"))); i++) {
    await page.keyboard.press("Escape");
    await sleep(300);
  }
}

async function openGame(browser, server, view = VIEW, query = "renderer=hd2d&force3d=1") {
  const page = await browser.newPage({ viewport: view, deviceScaleFactor: 2 });
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  page.on("console", (m) => m.type() === "error" && console.error(`console error: ${m.text()}`));
  await page.goto(`${server.url}/?${query}`);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && (window.ffh.S.status?.phase === "human" || window.ffh.S.status?.phase === "decision"))), "the staged game", 180000);
  await sleep(3000);
  return page;
}

/** Ends the turn and waits for the pick dialog. */
async function toPick(page) {
  await page.evaluate(() => {
        window.ffh.api.cmd("end_turn", { force: true }).catch(() => {});
      });
  await until(() => page.evaluate(() => document.querySelectorAll(".modal.decision.house-pick .hcard").length >= 2), "the pick dialog", 180000);
  await sleep(1500);
}

async function main() {
  const pw = await loadPlaywright();
  if (!pw) return skipped("houses-shots", "playwright not found");
  const bin = findBin(args.bin);
  if (!bin) return skipped("houses-shots", "no ffh-play binary");
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    if (want("home") || want("map") || want("screen") || want("city") || want("diplo")) {
      const saves = tempDir("houses-shots");
      const st = await stage(bin, "houses", path.join(saves, "staged-houses"));
      const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
      try {
        const page = await openGame(browser, server);
        // (9) A House alert: the forming messages of the turn, the newest first kept.
        const alert = await page.evaluate(() => {
          const ts = [...document.querySelectorAll("#toasts .toast")];
          const keep = ts.filter((t) => t.classList.contains("house-alert") && !t.classList.contains("more")).pop();
          for (const t of ts) if (t !== keep) t.remove();
          return !!keep;
        });
        if (alert) await crop(page, ["#toasts .house-alert"], "09-house-alert.png", { pad: 6, maxW: 560 });
        else console.log("09-house-alert.png: no toast");
        await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));

        // (8) The plates' shields: the map centred on the capital, zoomed in.
        if (want("map")) {
          await page.evaluate((c) => {
            const city = window.ffh.S.cities.get(c);
            window.ffh.mapView.centerOn(city.x, city.y);
          }, st.capital.id);
          for (let i = 0; i < 2; i++) {
            await page.keyboard.press("q");
            await sleep(300);
          }
          await until(() => page.evaluate(() => (window.ffh.mapView?.r3d?.plateHits ?? []).filter((b) => b?.kind === "house").length >= 3), "the shields on the plates", 90000);
          await sleep(6000);
          const r = await page.evaluate(() => {
            const hits = (window.ffh.mapView.r3d.plateHits ?? []).filter((b) => b?.kind === "house");
            const rc = window.ffh.mapView.canvas.getBoundingClientRect();
            let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
            for (const b of hits) {
              x0 = Math.min(x0, b.x0); y0 = Math.min(y0, b.y0); x1 = Math.max(x1, b.x1); y1 = Math.max(y1, b.y1);
            }
            return { x0: rc.left + x0 - 160, y0: rc.top + y0 - 90, x1: rc.left + x1 + 40, y1: rc.top + y1 + 40, w: innerWidth, h: innerHeight };
          });
          await clip(page, r, "08-plates.png", { pad: 0, maxW: 760 });
          // Its tooltip: a pointer over the capital's shield.
          const at = await page.evaluate((c) => {
            const b = (window.ffh.mapView.r3d.plateHits ?? []).find((b) => b?.kind === "house" && b.city === c);
            const rc = window.ffh.mapView.canvas.getBoundingClientRect();
            return b ? { x: rc.left + (b.x0 + b.x1) / 2, y: rc.top + (b.y0 + b.y1) / 2 } : null;
          }, st.capital.id);
          if (at) {
            await page.mouse.move(at.x, at.y);
            await sleep(800);
            await crop(page, [".tooltip:not([hidden])", ".tip-side:not([hidden])"], "08b-plate-tip.png", { pad: 6, maxW: 600 }).catch((e) => console.log(`08b-plate-tip.png: ${e.message}`));
            await page.mouse.move(640, 790);
          }
        }

        // The dialogs over the flat map: their crops show none of it, and software GL holds the page
        // for seconds a frame on HD-2D.
        await page.evaluate(() => window.ffh.setRenderer("flat"));
        await sleep(1500);
        // (4) The Houses screen, (5) a card's opinion tooltip.
        if (want("screen")) {
          await closeModals(page);
          await page.evaluate(() => document.dispatchEvent(new KeyboardEvent("keydown", { key: "F9", shiftKey: true, bubbles: true, cancelable: true })));
          await until(() => page.evaluate(() => document.querySelectorAll('.modal[data-tag="houses"] .hcard').length === 4), "the Houses screen", 30000);
          await sleep(800);
          await crop(page, ['.modal[data-tag="houses"]'], "04-houses-screen.png", { pad: 0, maxW: 1000 });
          const sel = `.modal[data-tag="houses"] .hcard[data-house="${st.houses.pleased.id}"] .hopinion`;
          await hover(page, sel);
          await crop(page, [`.modal[data-tag="houses"] .hcard[data-house="${st.houses.pleased.id}"]`, ".richtip:not([hidden])", ".tip-side:not([hidden])"], "05-opinion-tooltip.png", { pad: 6, maxW: 900 });
          await unhover(page, sel);
          await closeModals(page);
        }

        // (6) The city screen's House block and the mood.
        if (want("city")) {
          await page.evaluate((c) => window.ffh.emit("open-city", c), st.houses.annoyed.cities[0].id);
          await until(() => page.evaluate(() => !!document.querySelector(".section.city-house")), "the city's House block", 30000);
          await sleep(800);
          await page.evaluate(() => document.querySelector(".section.city-house").scrollIntoView({ block: "center" }));
          await sleep(300);
          await crop(page, [".section.city-house", ".section.governance"], "06-city-house.png", { pad: 8, maxW: 600 });
          await hover(page, ".section.mood .mood-row");
          await crop(page, [".section.mood", ".richtip:not([hidden])"], "06b-city-mood.png", { pad: 6, maxW: 700 });
          await unhover(page, ".section.mood .mood-row");
          await closeModals(page);
        }

        // (10) The diplomacy row of the rival's Houses, (7) the war question with its preview.
        if (want("diplo")) {
          await page.evaluate(() => {
            window.ffh.openScreen("diplomacy");
          });
          await until(() => page.evaluate((p) => !!document.querySelector(`tr[data-player="${p}"] .diplo-house`), st.rival), "the rival's Houses", 30000);
          await sleep(600);
          const sel = `tr[data-player="${st.rival}"] .diplo-house`;
          await hover(page, sel);
          await crop(page, [`tr[data-player="${st.rival}"] td:first-child`, ".richtip:not([hidden])", ".tip-side:not([hidden])"], "10-diplomacy-houses.png", { pad: 6, maxW: 900 });
          await unhover(page, sel);
          await page.evaluate((s) => document.querySelector(s).click(), `tr[data-player="${st.rival}"] button[data-war]`);
          await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="ask"] .house-preview')), "the war question", 20000);
          await sleep(500);
          await crop(page, ['.modal[data-tag="ask"]'], "07-war-question.png", { pad: 0, maxW: 700 });
          await page.keyboard.press("Escape");
          await sleep(300);
          await closeModals(page);
        }

        // (1) The pick of a home House.
        if (want("home")) {
          await toPick(page);
          await crop(page, [".modal.decision.house-pick"], "01-pick-home.png", { pad: 0, maxW: 1280 });
        }
        await page.close();
      } finally {
        await server.stop();
      }
    }

    // (2) The pick of a foreign House with the outsider's card, (3) the same on a phone.
    if (want("foreign")) {
      const saves = tempDir("houses-shots-foreign");
      const st = await stage(bin, "houses-foreign", path.join(saves, "staged-houses-foreign"));
      const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
      try {
        const page = await openGame(browser, server, VIEW, "renderer=flat");
        await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
        await toPick(page);
        await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
        await crop(page, [".modal.decision.house-pick"], "02-pick-foreign.png", { pad: 0, maxW: 1280 });
        const phone = await openGame(browser, server, PHONE, "renderer=flat");
        await until(() => phone.evaluate(() => document.querySelectorAll(".modal.decision.house-pick .hcard").length >= 2), "the pick on the phone", 120000);
        await sleep(1500);
        await phone.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
        await crop(phone, [".modal.decision.house-pick"], "03-pick-phone.png", { pad: 0, maxW: 390 });
        // The last card (the outsider's) scrolled into view.
        await phone.evaluate(() => {
          const cards = document.querySelectorAll(".modal.decision.house-pick .hcard");
          cards[cards.length - 1].scrollIntoView({ block: "center" });
        });
        await sleep(500);
        await crop(phone, [".modal.decision.house-pick .hcard.outsider"], "03b-pick-phone-outsider.png", { pad: 6, maxW: 390 });
        await phone.close();
        await page.close();
      } finally {
        await server.stop();
      }
    }
  } finally {
    await browser.close();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
