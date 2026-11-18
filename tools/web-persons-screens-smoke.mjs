#!/usr/bin/env node
// The browser smoke test of Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts
// C to I; docs/HUMAN.md): stages `ffh-play stage persons` and `persons-direct`, serves each, loads
// the page on the HD-2D map (`?force3d=1`: SwiftShader here, slow) and reads, as a player sees
// them: a House city's Governance line on the city screen, a Crown city's, the Armies screen's
// commander lines, the unit panel's person line and a great person's Found a House, a person's
// alert, and in `persons-direct` the civics screen's warning for God King. Any console error or
// page error fails it.
//
// With `--shots DIR` it also saves the screenshots the owner reviews (CLAUDE.md, "Show UI changes
// in a screenshot": crops at deviceScaleFactor 2, the screens at a 390 px wide page): the map's
// plates, the Governance sections, the commander lines, the unit panels, an alert, the civics
// warning and the war declaration's preview.
//
//   node tools/web-persons-screens-smoke.mjs [--bin PATH] [--assets DIR] [--shots DIR] [--gpu]
//
// The browser: Playwright's Chromium (as tools/web-smoke.mjs); without it the test is SKIPPED
// (exit 0, or 1 when $CI is set). Exit 1 on a failure.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value", shots: "value", gpu: "flag", "no-map": "flag" });
const SHOTS = args.shots ? path.resolve(args.shots) : null;
const ASSETS = args.assets ? ["--assets", path.resolve(args.assets)] : [];
const DESK = { width: 1280, height: 800 };
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

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const errors = [];
let failed = false;

function check(cond, what) {
  if (cond) {
    console.log(`ok: ${what}`);
  } else {
    failed = true;
    console.error(`FAIL: ${what}`);
  }
}

/** Chooses a research for the human (the staged game asks for one: its popup would cover the
 *  screens). */
async function research(url) {
  const r = await fetch(`${url}/api`, { method: "POST", headers: { "X-FFH": "1", "Content-Type": "application/json" }, body: JSON.stringify({ k: "cmd", c: "research", tech: "TECH_ANCIENT_CHANTS" }) });
  if (!r.ok) throw new Error(`research: HTTP ${r.status} ${await r.text()}`);
}

/** A page on the served game at the player's turn; console and page errors are collected. */
async function open(browser, url, viewport) {
  // Reduced motion: the HD-2D map draws only when something changes (SwiftShader takes seconds a
  // frame; an animated map would starve the page's scripts).
  const page = await browser.newPage({ viewport, deviceScaleFactor: SHOTS ? 2 : 1, reducedMotion: "reduce" });
  page.on("pageerror", (e) => errors.push(`page error: ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(`console error: ${m.text()}`);
  });
  page.on("requestfailed", (r) => errors.push(`request failed: ${r.url()}`));
  await page.goto(`${url}/?force3d=1`);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.status?.phase === "human")), "the staged game", 180000, 250);
  await sleep(1000);
  return page;
}

/** Closes every modal and drops the toasts. */
async function clear(page) {
  await page.evaluate(() => {
    for (const b of document.querySelectorAll("#modals .modal")) b.querySelector(".modal-buttons button:last-child")?.click();
    for (const t of document.querySelectorAll("#toasts .toast")) t.remove();
  });
  await page.keyboard.press("Escape");
  await sleep(300);
}

/** A crop around the union of what `sels` match (`pad` CSS px round it), saved as `name`. */
async function crop(page, sels, name, { pad = 6, maxH = 2000 } = {}) {
  if (!SHOTS) return;
  const r = await page.evaluate((sels) => {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const s of sels) {
      for (const e of document.querySelectorAll(s)) {
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
  const w = Math.min(Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(maxH, Math.ceil(r.y1 + pad) - y, r.h - y);
  await page.screenshot({ path: path.join(SHOTS, name), clip: { x, y, width: w, height: h }, timeout: 300000 });
  console.log(`shot ${name}: ${w} x ${h} CSS px`);
}

/** Opens city `id`'s screen and waits for its Governance section. */
async function openCity(page, id) {
  await page.evaluate((id) => window.ffh.emit("open-city", id), id);
  await until(() => page.evaluate(() => !!document.querySelector("#modals .section.governance .holder-line")), `the Governance line of city ${id}`, 60000, 200);
  await page.evaluate(() => document.querySelector("#modals .section.governance")?.scrollIntoView({ block: "center" }));
  await sleep(500);
  return page.evaluate(() => document.querySelector("#modals .section.governance .holder-line").textContent);
}

/** Selects unit `id` and waits for its panel's line `sel`. */
async function selectUnit(page, id, sel) {
  await page.evaluate((id) => window.ffh.units.select([id], { center: true }), id);
  await until(() => page.evaluate((sel) => !!document.querySelector(sel), sel), `the unit panel's ${sel}`, 60000, 200);
  await sleep(500);
  return page.evaluate((sel) => document.querySelector(sel).textContent, sel);
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("web-persons-screens-smoke", "no playwright");
  if (SHOTS) fs.mkdirSync(SHOTS, { recursive: true });
  const saves = tempDir("persons-screens");
  const st = await stage(bin, "persons", saves);
  const sd = await stage(bin, "persons-direct", saves);
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  try {
    // ---- persons: the screens on a desktop page, the map's plates --------------------------
    let server = await startServer({ bin, saves, extra: ["--load", st.save, ...ASSETS] });
    try {
      await research(server.url);
      const page = await open(browser, server.url, DESK);
      // H. The persons' alerts of the loaded turn are up (the newest few and a count).
      const alerts = await page.evaluate(() => document.querySelectorAll("#toasts .person-alert").length);
      check(alerts > 0, `persons' alerts on the load (${alerts})`);
      // G. The map: the plates of the Houses' cities, centred between House C's seat and House B's.
      const seat = (role) => st.houses[role].seat;
      const at = await page.evaluate((ids) => ids.map((id) => window.ffh.S.cities.get(id)).map((c) => [c.x, c.y]), [seat("A").id, seat("B").id, seat("C").id, st.crown[0].id]);
      const houseMarks = await page.evaluate(() => [...window.ffh.S.cities.values()].filter((c) => c.house).length);
      check(houseMarks >= 8, `own cities with a House mark in the view (${houseMarks})`);
      if (SHOTS && !args["no-map"]) {
        await clear(page);
        const cx = Math.round(at.reduce((s, p) => s + p[0], 0) / at.length);
        const cy = Math.round(at.reduce((s, p) => s + p[1], 0) / at.length);
        await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), [cx, cy]);
        await sleep(args.gpu ? 3000 : 30000);
        const box = await page.evaluate((pts) => {
          const ps = pts.map(([x, y]) => window.ffh.mapView.screenOf(x, y)).filter(Boolean);
          return { x0: Math.min(...ps.map((p) => p.x)), y0: Math.min(...ps.map((p) => p.y)), x1: Math.max(...ps.map((p) => p.x)), y1: Math.max(...ps.map((p) => p.y)) };
        }, at);
        const x = Math.max(0, Math.floor(box.x0 - 110));
        const y = Math.max(0, Math.floor(box.y0 - 60));
        const w = Math.min(DESK.width - x, Math.ceil(box.x1 - box.x0 + 220));
        const h = Math.min(DESK.height - y, Math.ceil(box.y1 - box.y0 + 130));
        await page.screenshot({ path: path.join(SHOTS, "map-plates.png"), clip: { x, y, width: w, height: h }, timeout: 300000 });
        console.log(`shot map-plates.png: ${w} x ${h} CSS px`);
      }
      // H. One alert: a change of state.
      await clear(page);
      const shownAlert = await page.evaluate(async () => {
        const m = await import("./app/persons_lines.js");
        const e = (window.ffh.S.messages ?? []).filter((x) => x.kind === "person" && / is Disloyal\./.test(x.text)).pop();
        if (!e) return null;
        m.personAlerts([e]);
        return e.text;
      });
      check(!!shownAlert, `a state change's alert: ${shownAlert}`);
      await sleep(500);
      // (Held for the screenshot: a toast leaves after 12 s, a SwiftShader frame can take longer.)
      await page.evaluate(() => {
        const t = document.querySelector("#toasts .person-alert");
        if (t) t.parentElement.replaceChild(Object.assign(t.cloneNode(true), { id: "shot-alert" }), t);
      });
      await crop(page, ["#shot-alert"], "alert-state.png");
      await page.evaluate(() => document.getElementById("shot-alert")?.remove());
      await clear(page);
      await page.close();

      // The screens on a phone-wide page.
      const phone = await open(browser, server.url, PHONE);
      await clear(phone);
      // C. A House city's Governance line, and a Crown city's.
      const a = await openCity(phone, seat("A").id);
      check(a.includes(st.houses.A.label) && a.includes("Head:") && /Devoted/.test(a), `House city's Governance line: ${a}`);
      await crop(phone, ["#modals .section.governance"], "city-governance-house.png");
      await clear(phone);
      const b = await openCity(phone, seat("B").id);
      check(/Restless/.test(b) && b.includes("It takes no orders."), `Restless House city's Governance line: ${b}`);
      await crop(phone, ["#modals .section.governance"], "city-governance-restless.png");
      await clear(phone);
      const c = await openCity(phone, st.crown[1].id);
      check(c.includes("Held by the Crown."), `Crown city's Governance line: ${c}`);
      await crop(phone, ["#modals .section.governance"], "city-governance-crown.png");
      const overflow = await phone.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      check(overflow <= 0, `no horizontal overflow at 390 px (${overflow})`);
      await clear(phone);
      // D. The Armies screen's commander lines.
      await phone.evaluate(() => void window.ffh.openScreen("armies"));
      await until(() => phone.evaluate(() => document.querySelectorAll("#modals .army-card .commander-line").length > 0), "the commander lines", 60000, 200);
      await sleep(500);
      const lines = await phone.evaluate(() => [...document.querySelectorAll("#modals .army-card .commander-line")].map((e) => e.textContent));
      check(lines.some((t) => t.startsWith("Commander: ") && /Loyal/.test(t)) && lines.some((t) => /Disloyal/.test(t) && /attack/.test(t)), `commander lines: ${lines.join(" | ")}`);
      for (const [role, name] of [["loyal", "armies-commander-loyal.png"], ["disloyal", "armies-commander-disloyal.png"]]) {
        const id = st.armies[role].id;
        await phone.evaluate((id) => document.querySelector(`#modals .army-card[data-army="${id}"]`)?.scrollIntoView({ block: "start" }), id);
        await sleep(400);
        await crop(phone, [`#modals .army-card[data-army="${id}"] .army-title`, `#modals .army-card[data-army="${id}"] .commander-line`], name);
      }
      await clear(phone);
      // F. The unit panel: a commander's unit, the great person's Found a House.
      const pl = await selectUnit(phone, st.persons.commander_loyal.unit, ".person-line");
      check(/^Person of note: .+, commander of the /.test(pl), `person line: ${pl}`);
      await crop(phone, ["#unit-panel"], "unit-person.png", { maxH: 700 });
      const fh = await selectUnit(phone, st.great_person, ".found-house-line");
      check(fh.startsWith("Found a House"), `Found a House: ${fh}`);
      await crop(phone, ["#unit-panel"], "unit-great-person.png", { maxH: 700 });
      await phone.evaluate(() => window.ffh.units.clearSelection());
      await clear(phone);
      // I. The war declaration's preview (the Svartalfar: a commander demands this war).
      await phone.evaluate(() => void window.ffh.openScreen("diplomacy"));
      await until(() => phone.evaluate(() => !!document.querySelector("#modals button[data-war]")), "the Declare war button", 60000, 200);
      await phone.evaluate(() => document.querySelector("#modals button[data-war]").click());
      await until(() => phone.evaluate(() => !!document.querySelector("#modals .modal .persons-preview")), "the war's preview lines", 30000, 200);
      const war = await phone.evaluate(() => document.querySelector("#modals .modal .persons-preview").textContent);
      check(/Demand met/.test(war), `war preview: ${war}`);
      await crop(phone, ["#modals .modal:last-child"], "war-preview.png");
      await clear(phone);
      await phone.close();
    } finally {
      await server.stop();
    }

    // ---- persons-direct: the civics screen's warning ---------------------------------------
    server = await startServer({ bin, saves, extra: ["--load", sd.save, ...ASSETS] });
    try {
      await research(server.url);
      const phone = await open(browser, server.url, PHONE);
      await clear(phone);
      await phone.evaluate(() => void window.ffh.openScreen("civics"));
      await until(() => phone.evaluate(() => !!document.querySelector("#modals button.civic")), "the civics screen", 60000, 200);
      const gk = await phone.evaluate(() => {
        const i = (window.ffh.S.data.tables.CivicType ?? []).findIndex((c) => c.type === "CIVIC_GOD_KING");
        document.querySelector(`#modals button.civic[data-civic="${i}"]`)?.click();
        return i;
      });
      check(gk >= 0, "God King picked");
      await until(() => phone.evaluate(() => !!document.querySelector("#modals .persons-warning")), "the civics warning", 30000, 200);
      const warn = await phone.evaluate(() => document.querySelector("#modals .persons-warning").textContent);
      const button = await phone.evaluate(() => [...document.querySelectorAll("#modals .modal-buttons button")].map((b) => b.textContent));
      check(/^Adopting .+: \d+ Houses secede with /.test(warn), `civics warning: ${warn}`);
      check(button.includes("Adopt and face the secession"), `the revolution's button: ${button.join(", ")}`);
      await phone.evaluate(() => document.querySelector("#modals .persons-warning").scrollIntoView({ block: "center" }));
      await sleep(400);
      await crop(phone, ["#modals .civic-summary", "#modals .modal-buttons"], "civics-warning.png");
      await phone.close();
    } finally {
      await server.stop();
    }
  } finally {
    await browser.close();
  }
  for (const e of errors) console.error(e);
  check(errors.length === 0, `no console or page error (${errors.length})`);
  if (failed) process.exit(1);
  console.log("web-persons-screens-smoke: ok");
}

main().catch((e) => {
  console.error(`web-persons-screens-smoke: ${e.stack ?? e}`);
  process.exit(1);
});
