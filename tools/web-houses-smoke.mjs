#!/usr/bin/env node
// Browser smoke of this game's Houses (docs/HOUSES.md, rule 15; docs/HUMAN.md, "Houses"): on the
// staged realm (`ffh-play stage houses`), served with `ffh-play serve --load` and driven with
// Playwright's Chromium on the HD-2D map in software (`?renderer=hd2d&force3d=1`):
// - the House alerts of the turn (the forming messages) as toasts;
// - the plates' shields drawn on the map, with their tooltip boxes;
// - the Houses screen (Shift+F9): four cards, the Furious House's perk off, the city in no House;
// - a city from a card's chip: the city screen's House block;
// - the diplomacy screen's Houses of the rival, and the war question with its Houses preview;
// - ending the turn: the pick dialog of the waiting city's House (cards side by side), a card
//   chosen, the House formed;
// - `houses-foreign`: the foreign pick's last card is the outsider's.
// No console error, no page error.
//
//   node tools/web-houses-smoke.mjs --bin <built>/ffh-play [--assets crates/ffh-play/assets]

import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value" });
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

const steps = [];
const t00 = Date.now();
const log = (s) => console.log(`  [${((Date.now() - t00) / 1000).toFixed(0)} s] ${s}`);
const problems = [];

function check(ok, what) {
  if (!ok) throw new Error(what);
}

/** Opens the staged game `kind` in a new page of `browser`; returns `{page, server, st}`. */
async function open(browser, bin, kind, query = "renderer=hd2d&force3d=1") {
  const saves = tempDir(`houses-smoke-${kind}`);
  const st = await stage(bin, kind, path.join(saves, `staged-${kind}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  const page = await browser.newPage({ viewport: VIEW });
  page.on("pageerror", (e) => problems.push(`${kind}: page error: ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error") problems.push(`${kind}: console error: ${m.text()}`);
  });
  await page.goto(`${server.url}/?${query}`);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.status?.phase === "human")), `the staged game ${kind}`, 180000);
  await sleep(3000);
  return { page, server, st };
}

async function closeModals(page) {
  for (let i = 0; i < 6 && (await page.evaluate(() => !!document.querySelector("#modals .modal:not(.decision)"))); i++) {
    await page.keyboard.press("Escape");
    await sleep(300);
  }
}

async function main() {
  const pw = await loadPlaywright();
  if (!pw) return skipped("web-houses-smoke", "playwright not found");
  const bin = findBin(args.bin);
  if (!bin) return skipped("web-houses-smoke", "no ffh-play binary");
  const t0 = Date.now();
  const browser = await pw.chromium.launch(chromiumLaunch(false));
  try {
    // ---- houses --------------------------------------------------------------------------------
    const { page, server, st } = await open(browser, bin, "houses");
    try {
      const alerts = await page.evaluate(() => [...document.querySelectorAll("#toasts .house-alert")].map((t) => t.textContent));
      check(alerts.some((t) => / forms: /.test(t)), `no House alert of the forming: ${JSON.stringify(alerts)}`);
      log(`${alerts.length} House alerts`), steps.push(`${alerts.length} House alerts`);

      // The plates' shields (HD-2D): a box per own city in a House, once a frame is drawn.
      await until(() => page.evaluate(() => (window.ffh.mapView?.r3d?.plateHits ?? []).some((b) => b?.kind === "house")), "a House shield on a plate", 60000);
      const shields = await page.evaluate(() => (window.ffh.mapView.r3d.plateHits ?? []).filter((b) => b?.kind === "house").length);
      log(`${shields} House shields on the plates`), steps.push(`${shields} House shields on the plates`);

      // The rest on the flat map (software GL draws a frame in seconds and holds the page meanwhile);
      // its plates carry the shields too.
      await page.evaluate(() => window.ffh.setRenderer("flat"));
      await until(() => page.evaluate(() => !window.ffh.mapView.r3d && (window.ffh.mapView.plateHits ?? []).some((b) => b?.kind === "house")), "the flat map's shields", 60000);
      log("the flat map's plates carry the shields"), steps.push("the flat map's plates carry the shields");

      // The Houses screen (Shift+F9).
      await closeModals(page);
      await page.evaluate(() => document.dispatchEvent(new KeyboardEvent("keydown", { key: "F9", shiftKey: true, bubbles: true, cancelable: true })));
      await until(() => page.evaluate(() => document.querySelectorAll('.modal[data-tag="houses"] .hcard').length === 4), "the Houses screen's four cards", 30000);
      const screen = await page.evaluate(() => document.querySelector('.modal[data-tag="houses"]').textContent);
      check(screen.includes("Perk off: the House is Furious."), "the Furious House's perk is not shown off");
      check(screen.includes("Outsider: -3 happiness in this House's cities. No sympathy."), "the outsider's line is missing");
      check(screen.includes("No House yet: it joins or forms one at your next turn."), "the city in no House is missing");
      log("the Houses screen: four cards, the perk off, the outsider, the city in no House"), steps.push("the Houses screen: four cards, the perk off, the outsider, the city in no House");

      // A city from the seat's chip: the city screen's House block.
      await page.evaluate((s) => document.querySelector(s).click(), `.modal[data-tag="houses"] .hcard[data-house="${st.houses.pleased.id}"] .city-chip.seat`);
      await until(() => page.evaluate(() => !!document.querySelector(".city-house-line")), "the city screen's House line", 30000);
      const line = await page.evaluate(() => document.querySelector(".city-house-line").textContent);
      check(line.startsWith(`${st.houses.pleased.label} (`) && line.includes(": Pleased, +1 happiness. Perk: "), `the House line: ${line}`);
      log(`the city screen: ${line}`), steps.push(`the city screen: ${line}`);
      await closeModals(page);

      // Diplomacy: the rival's Houses, the war question's preview.
      await page.evaluate(() => {
        window.ffh.openScreen("diplomacy");
      });
      await until(() => page.evaluate((p) => !!document.querySelector(`tr[data-player="${p}"] .diplo-house`), st.rival), "the rival's Houses on the diplomacy screen", 30000);
      await page.evaluate((s) => document.querySelector(s).click(), `tr[data-player="${st.rival}"] button[data-war]`);
      await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="ask"] .house-preview')), "the war question's House preview", 20000);
      const pv = await page.evaluate(() => document.querySelector('.modal[data-tag="ask"] .house-preview').textContent);
      check(pv.includes(`${st.houses.pleased.label}: Pleased to `), `the war preview: ${pv}`);
      log(`the war question: ${pv}`), steps.push(`the war question: ${pv}`);
      await page.keyboard.press("Escape");
      await sleep(300);
      await closeModals(page);

      // The end of the turn: the pick.
      await page.evaluate(() => {
        window.ffh.api.cmd("end_turn", { force: true }).catch(() => {});
      });
      await until(() => page.evaluate(() => document.querySelectorAll(".modal.decision.house-pick .hcard").length >= 2), "the pick dialog's cards", 120000);
      const pick = await page.evaluate(() => {
        const m = document.querySelector(".modal.decision.house-pick");
        return { cards: m.querySelectorAll(".hcard").length, advised: m.querySelectorAll(".hcard.advised").length, head: m.querySelector(".hpick-forms")?.textContent ?? "" };
      });
      check(pick.advised === 1, `the advisor's card: ${pick.advised}`);
      check(/^House .* forms with /.test(pick.head), `the pick's head: ${pick.head}`);
      log(`the pick: ${pick.cards} cards, ${pick.head}`), steps.push(`the pick: ${pick.cards} cards, ${pick.head}`);
      const before = await page.evaluate(() => window.ffh.api.query("houses").then((r) => r.houses.length));
      await page.evaluate((s) => document.querySelector(s).click(), ".modal.decision.house-pick .hcard[data-card=\"0\"] .hchoose");
      await until(() => page.evaluate(() => window.ffh.S.status?.phase === "human" && !document.querySelector(".modal.decision")), "the turn after the pick", 120000);
      const after = await page.evaluate(() => window.ffh.api.query("houses").then((r) => r.houses.length));
      check(after === before + 1, `Houses before and after the pick: ${before}, ${after}`);
      log(`the first card chosen: ${after} Houses`), steps.push(`the first card chosen: ${after} Houses`);
      await page.close();
    } finally {
      await server.stop();
    }

    // ---- houses-foreign ------------------------------------------------------------------------
    const f = await open(browser, bin, "houses-foreign", "renderer=flat");
    try {
      await f.page.evaluate(() => {
        window.ffh.api.cmd("end_turn", { force: true }).catch(() => {});
      });
      await until(() => f.page.evaluate(() => document.querySelectorAll(".modal.decision.house-pick .hcard").length >= 2), "the foreign pick's cards", 120000);
      const last = await f.page.evaluate(() => {
        const cards = [...document.querySelectorAll(".modal.decision.house-pick .hcard")];
        return { outsider: cards[cards.length - 1].classList.contains("outsider"), text: cards[cards.length - 1].textContent };
      });
      check(last.outsider && last.text.includes("Outsider: -3 happiness in this House's cities. No sympathy."), `the outsider's card: ${last.text}`);
      log("the foreign pick: the outsider's card last"), steps.push("the foreign pick: the outsider's card last");
      await f.page.evaluate((s) => document.querySelector(s).click(), ".modal.decision.house-pick .modal-buttons button");
      await until(() => f.page.evaluate(() => window.ffh.S.status?.phase === "human" && !document.querySelector(".modal.decision")), "the turn after the foreign pick", 120000);
      await f.page.close();
    } finally {
      await f.server.stop();
    }
  } catch (e) {
    problems.push(e.message);
  } finally {
    await browser.close();
  }

  console.log(`web-houses-smoke: ${steps.length} steps in ${((Date.now() - t0) / 1000).toFixed(1)} s`);
  if (problems.length) {
    for (const p of problems) console.error(`FAIL: ${p}`);
    process.exit(1);
  }
  console.log("web-houses-smoke: ok");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
