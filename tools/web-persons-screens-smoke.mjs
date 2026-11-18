#!/usr/bin/env node
// The browser smoke test of Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts
// C to I; docs/HUMAN.md): stages `ffh-play stage persons` and `persons-direct`, serves each, loads
// the page on the HD-2D map (`?force3d=1`: SwiftShader here, slow) and reads, as a player sees
// them: the House marks on the Houses' cities and the Houses drawn in their lieges' colours, a
// Crown city's Governance line on the city screen, the Armies screen's commander lines, the unit
// panel's person line and a great person's Found a House, a person's alert, the plates' marks'
// tooltips (the Furious countdown's pill, a House capital's star), the war declaration's dialog
// (the Houses it brings in, its preview), the diplomacy list's Houses (yours first with their
// state, the neighbour's under its liege with its state and price) and the neighbour's House's
// trade screen (a Secede offer refused, Become your vassal's war, then a Secede offer accepted
// with its cost with the liege: the House breaks away and its alert shows), and in
// `persons-direct` the civics screen's warning for God King. Any
// console error or page error fails it. (The second build, docs/PERSONS.md: a House's city is its
// own player's, so it has no Governance section on the liege's city screen; the stage's Houses
// and armies are named by role; the states are the attitude scale.)
//
// With `--shots DIR` it also saves the screenshots the owner reviews (CLAUDE.md, "Show UI changes
// in a screenshot": crops at deviceScaleFactor 2, the screens at a 390 px wide page): the map's
// plates and two of their marks' tooltips, the Governance sections, the commander lines, the unit
// panels, the alerts, the diplomacy list (on a desktop page and a phone's) and its Houses, the
// House's trade screen refused, with Become your vassal and accepted, the civics warning and the
// war declaration's dialog.
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

/** A query to the served game (`{k: "q", q, ...}`): its reply's `r`. */
async function query(url, q) {
  const r = await fetch(`${url}/api`, { method: "POST", headers: { "X-FFH": "1", "Content-Type": "application/json" }, body: JSON.stringify({ k: "q", ...q }) });
  if (!r.ok) throw new Error(`${q.q}: HTTP ${r.status} ${await r.text()}`);
  const j = await r.json();
  if (!j.ok) throw new Error(`${q.q}: ${JSON.stringify(j)}`);
  return j.r;
}

/** Waits for the trade screen's answer to the deal as it stands (`accepted` or `refused`) and
 *  returns the summary's text. */
async function tradeAnswer(page, want) {
  await until(() => page.evaluate((want) => document.querySelector(`#modals .trade-summary [data-answer="${want}"]`) !== null, want), `the trade screen's answer: ${want}`, 30000, 100);
  return page.evaluate(() => document.querySelector("#modals .trade-summary").innerText.replace(/\s+/g, " ").trim());
}

/** The trade screen's gold amount set to `n` (the item in the deal). */
async function setGold(page, n) {
  await page.evaluate((n) => {
    const i = document.querySelector('#modals .trade-item[data-item="TRADE_GOLD"] input.amount');
    i.value = String(n);
    i.dispatchEvent(new Event("input", { bubbles: true }));
  }, n);
}

/** A crop of the trade screen's tables, its `Trades through` line and the answer, scrolled into
 *  view (the leader's head above them takes most of a phone's height). */
async function tradeCrop(page, name) {
  await page.evaluate(() => document.querySelector("#modals .modal:last-child .trade-summary")?.scrollIntoView({ block: "end" }));
  await sleep(300);
  await crop(page, ["#modals .modal:last-child .trade", "#modals .modal:last-child .trades-through", "#modals .modal:last-child .trade-summary"], name);
}

/** The diplomacy screen's Houses (docs/PERSONS.md, "The second build", rules 7, 9 and 13): the
 *  human's Houses listed first and the neighbour's House under its liege (`House X, vassal of the
 *  Y`); the House's trade screen: Secede and Become your vassal, `Trades through the Y.`, a Secede
 *  offer below its price refused (the rules' reason, proposed and refused) and one at its price
 *  accepted (the preview, the war it means, then proposed: the House breaks away and the human is
 *  told). */
async function diplomacyHouses(page, url, st) {
  const nh = st.houses.neighbour;
  const liege = st.neighbour.player;
  await page.evaluate(() => void window.ffh.openScreen("diplomacy"));
  await until(() => page.evaluate(() => document.querySelectorAll("#modals table.diplo tr.house-row").length > 0), "the Houses in the diplomacy list", 60000, 200);
  await sleep(500);
  const rows = await page.evaluate(() => [...document.querySelectorAll("#modals table.diplo tr[data-player]")].map((r) => ({ id: Number(r.dataset.player), house: r.classList.contains("house-row"), who: r.querySelector(".leader-cell .dim")?.textContent ?? "" })));
  const own = ["friendly", "pleased", "annoyed", "furious"].map((r) => st.houses[r].player);
  const iL = rows.findIndex((r) => r.id === liege);
  const iN = rows.findIndex((r) => r.id === nh.player);
  check(iL >= 0 && iN === iL + 1 && rows[iN].house && rows[iN].who.trim().startsWith(`${nh.label}, vassal of the `), `the neighbour's House under its liege: ${rows[iN]?.who}`);
  check(own.every((p) => rows.slice(0, own.length).some((r) => r.id === p && /, your vassal( · |$)/.test(r.who.trim()))), `your Houses first: ${rows.slice(0, own.length).map((r) => r.who.trim()).join(" | ")}`);
  const marks = await page.evaluate(() => [...document.querySelectorAll("#modals table.diplo tr.house-row td:nth-child(3) .pstate")].map((e) => e.dataset.state));
  check(marks.length === own.length, `your Houses' attitude marks (${marks.join(", ")})`);
  // Each House's line states its state where a phone sees it (the Attitude column is cut there):
  // yours toward you, with the Furious countdown; the neighbour's toward its liege, with its price.
  const lines = await page.evaluate(() => Object.fromEntries([...document.querySelectorAll("#modals table.diplo tr.house-row")].map((r) => [r.dataset.player, { text: r.querySelector(".house-who")?.textContent ?? "", state: r.querySelector(".house-who .pstate")?.dataset.state ?? null, price: r.querySelector(".house-who .house-price")?.dataset.price ?? null }])));
  check(own.every((p) => lines[p]?.state === st.houses[["friendly", "pleased", "annoyed", "furious"][own.indexOf(p)]].state.toLowerCase()), `your Houses' lines carry their state: ${own.map((p) => lines[p]?.text).join(" | ")}`);
  check(/ · ✖ ?Furious ?8 turns$/.test(lines[st.houses.furious.player]?.text.replace(/\s+/g, " ").trim() ?? ""), `the Furious House's line: ${lines[st.houses.furious.player]?.text}`);
  const pl = await query(url, { q: "players" });
  const offer = pl.players.find((p) => p.id === nh.player)?.house_offer;
  const nline = lines[nh.player];
  check(!!offer && nline?.state === offer.state.toLowerCase() && (offer.price === null ? nline.price === null : nline.price === String(offer.price) && nline.text.includes(`Secede: ${offer.price} gold`)), `the neighbour's House's line: ${nline?.text}`);
  const ownPeace = await page.evaluate((ids) => ids.map((id) => document.querySelector(`#modals tr[data-player="${id}"] td:nth-child(2)`)?.textContent.trim()), own);
  check(ownPeace.every((t) => t === ""), `your Houses' relations say no "peace": ${JSON.stringify(ownPeace)}`);
  await page.evaluate((ids) => document.querySelector(`#modals tr[data-player="${ids[0]}"]`)?.scrollIntoView({ block: "start" }), [liege]);
  await sleep(300);
  await crop(page, [`#modals tr[data-player="${liege}"]`, `#modals tr[data-player="${nh.player}"]`], "diplomacy-house-under-liege.png");
  await page.evaluate(() => document.querySelector("#modals table.diplo")?.scrollIntoView({ block: "start" }));
  await sleep(300);
  await crop(page, own.map((p) => `#modals tr[data-player="${p}"]`), "diplomacy-your-houses.png");

  // The House's trade screen.
  const t = await query(url, { q: "trade", with: nh.player });
  const price = t.house?.price;
  check(Number.isInteger(price) && price > 1, `the House's price: ${price}`);
  await page.evaluate((id) => document.querySelector(`#modals button[data-trade="${id}"]`).click(), nh.player);
  await until(() => page.evaluate(() => !!document.querySelector('#modals .trade-item[data-item="TRADE_SECEDE"]')), "the House's trade table", 60000, 200);
  await sleep(500);
  const items = await page.evaluate(() => [...document.querySelectorAll("#modals .trade-side")].map((s) => [...s.querySelectorAll(".trade-item")].map((e) => e.dataset.item)));
  check(items[1]?.join(",") === "TRADE_SECEDE,TRADE_HOUSE_VASSAL" && items[0]?.every((x) => x === "TRADE_GOLD" || x === "TRADE_GOLD_PER_TURN"), `the House's trade table: ${JSON.stringify(items)}`);
  const through = await page.evaluate(() => document.querySelector("#modals .trades-through")?.textContent ?? "");
  check(/^Trades through the \S.*\./.test(through), `what is its liege's: ${through}`);
  // Secede for 1 gold: refused, the rules' reason (its price).
  await page.evaluate(() => {
    document.querySelector('#modals .trade-item[data-item="TRADE_SECEDE"] button').click();
    document.querySelector('#modals .trade-item[data-item="TRADE_GOLD"] button').click();
  });
  await setGold(page, 1);
  const refused = await tradeAnswer(page, "refused");
  check(refused.includes(`Reason: ${nh.label} asks ${price} gold.`) && refused.includes(`${nh.label} will break away from the `) && refused.includes(": war between them."), `refused: ${refused}`);
  check(!/attitude toward you/.test(refused), `a refused offer states no cost: ${refused}`);
  await tradeCrop(page, "trade-house-refused.png");
  // Become your vassal (when the rules let the human take it): the war it means and who joins it.
  const vassalOk = await page.evaluate(() => {
    const b = document.querySelector('#modals .trade-item[data-item="TRADE_HOUSE_VASSAL"] button');
    if (!b || b.disabled) return false;
    b.click();
    return true;
  });
  if (vassalOk) {
    await setGold(page, price);
    const v = await tradeAnswer(page, "accepted");
    const yours = ["friendly", "pleased", "annoyed", "furious"].map((r) => st.houses[r].label);
    check(/You become its liege: war with the \S.*\./.test(v) && yours.every((l) => v.includes(l)) && v.includes("Your Houses join: "), `Become your vassal: ${v}`);
    check(/If accepted: .*The \S.*'s attitude toward you -\d+ \(lasting\)\./.test(v), `Become your vassal's cost with the liege: ${v}`);
    await tradeCrop(page, "trade-house-vassal.png");
    await page.evaluate(() => document.querySelector('#modals .trade-item[data-item="TRADE_SECEDE"] button').click());
    await setGold(page, 1);
    await tradeAnswer(page, "refused");
  } else check(true, "Become your vassal: not open to the human here");
  await page.evaluate(() => [...document.querySelectorAll("#modals .modal:last-child .modal-buttons button")].find((b) => b.textContent === "Propose").click());
  await until(() => page.evaluate(() => !!document.querySelector('#modals .modal:last-child [data-answer="declined"]')), "the offer refused", 30000, 100);
  check(true, "a Secede offer below the price proposed and refused");
  // At the price: accepted, and what it means (war between the House and its liege).
  await setGold(page, price);
  const accepted = await tradeAnswer(page, "accepted");
  check(accepted.includes(`${nh.label} will break away from the `), `accepted: ${accepted}`);
  const grudge = await page.evaluate(() => document.querySelector("#modals .trade-summary .house-grudge")?.textContent ?? "");
  check(/^The \S.*'s attitude toward you -\d+ \(lasting\)\.$/.test(grudge), `an accepted offer's cost with the liege: ${grudge}`);
  await tradeCrop(page, "trade-house-accepted.png");
  await page.evaluate(() => [...document.querySelectorAll("#modals .modal:last-child .modal-buttons button")].find((b) => b.textContent === "Propose").click());
  await until(() => page.evaluate(() => !!document.querySelector('#modals .modal:last-child [data-answer="accepted"]')), "the offer accepted", 30000, 100);
  // The House broke away: it is no House now, and its break-away is told (an alert).
  const after = await query(url, { q: "players" });
  const np = after.players.find((p) => p.id === nh.player);
  check(np && np.house === null && np.color_player === nh.player, `the House broke away: ${JSON.stringify({ house: np?.house, color_player: np?.color_player })}`);
  await until(() => page.evaluate(() => [...document.querySelectorAll("#toasts .person-alert.secession")].some((e) => / breaks away from the .+: war\.$/.test(e.textContent))), "the break-away's alert", 60000, 200);
  check(true, "the break-away's alert");
  if (SHOTS) {
    // (Held for the screenshot, as the state's alert.)
    await page.evaluate(() => {
      const t = [...document.querySelectorAll("#toasts .person-alert.secession")].find((e) => / breaks away from the /.test(e.textContent));
      if (t) t.parentElement.replaceChild(Object.assign(t.cloneNode(true), { id: "shot-alert" }), t);
    });
    await crop(page, ["#shot-alert"], "alert-break-away.png");
    await page.evaluate(() => document.getElementById("shot-alert")?.remove());
  }
}

/** The plates' marks explain themselves (docs/PERSONS.md, "UI", part G): the pointer on the Furious
 *  House's countdown pill gives `House X: Furious.` and the rules' outcome; on a House capital's
 *  star, `Capital of House X (your vassal).`; with the House's reminder. The boxes are the ones the
 *  map recorded as it drew (plates.js). With --shots, a crop of each. */
async function plateTips(page, st) {
  const f = st.houses.furious;
  const ids = f.cities.map((c) => c.id);
  const hitsOf = () => page.evaluate((ids) => {
    const mv = window.ffh.mapView;
    const hits = mv.r3d?.plateHits ?? mv.iso?.plateHits ?? mv.plateHits ?? [];
    const rc = mv.canvas.getBoundingClientRect();
    return hits.filter((x) => ids.includes(x.city)).map((x) => ({ ...x, cx: rc.left + (x.x0 + x.x1) / 2, cy: rc.top + (x.y0 + x.y1) / 2 }));
  }, ids);
  const hits = await until(async () => {
    const h = await hitsOf();
    return h.some((x) => x.kind === "pill") ? h : null;
  }, "the Furious House's plate drawn with its countdown", 240000, 500);
  const tipText = () => page.evaluate(() => {
    const t = document.querySelector("#mapwrap .tooltip");
    return t && !t.hidden ? t.innerText.replace(/\s+/g, " ").trim() : "";
  });
  const point = async (hit, want, name) => {
    await page.mouse.move(5, 5);
    await sleep(200);
    await page.mouse.move(hit.cx, hit.cy);
    const text = await until(async () => {
      const t = await tipText();
      return want.test(t) ? t : null;
    }, `the ${hit.kind}'s tooltip`, 30000, 100).catch(async () => tipText());
    check(want.test(text), `the ${hit.kind}'s tooltip: ${text}`);
    if (SHOTS) {
      const box = await page.evaluate((hit) => {
        const t = document.querySelector("#mapwrap .tooltip").getBoundingClientRect();
        return { x0: Math.min(t.left, hit.cx - 60), y0: Math.min(t.top, hit.cy - 30), x1: Math.max(t.right, hit.cx + 60), y1: Math.max(t.bottom, hit.cy + 30) };
      }, hit);
      const x = Math.max(0, Math.floor(box.x0 - 6));
      const y = Math.max(0, Math.floor(box.y0 - 6));
      await page.screenshot({ path: path.join(SHOTS, name), clip: { x, y, width: Math.min(DESK.width - x, Math.ceil(box.x1 - box.x0 + 12)), height: Math.min(DESK.height - y, Math.ceil(box.y1 - box.y0 + 12)) }, timeout: 300000 });
      console.log(`shot ${name}`);
    }
  };
  const pill = hits.find((x) => x.kind === "pill");
  const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  await point(pill, new RegExp(`^${esc(f.label)}: Furious\\. ${esc(f.label)} will (break away|end) .*8 turns`), "plate-tip-countdown.png");
  const house = await page.evaluate(() => (window.ffh.S.data.tables.KeywordType ?? []).find((k) => k.type === "KEYWORD_HOUSE")?.name ?? "House");
  check((await tipText()).includes(house.toUpperCase()) || (await page.evaluate(() => !!document.querySelector('#mapwrap .tooltip .reminder[data-keyword="KEYWORD_HOUSE"]'))), "the countdown's tooltip carries the House's reminder");
  const star = (await hitsOf()).find((x) => x.kind === "star");
  if (star) await point(star, new RegExp(`^Capital of ${esc(f.label)} \\(your vassal\\)\\.`), "plate-tip-capital.png");
  else check(true, "no House capital among the Furious House's cities in view");
  await page.mouse.move(5, 5);
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
      // G. The map: the plates of the Houses' cities, centred between the Houses' seats.
      const seat = (role) => st.houses[role].seat;
      const at = await page.evaluate((ids) => ids.map((id) => window.ffh.S.cities.get(id)).filter(Boolean).map((c) => [c.x, c.y]), [seat("friendly").id, seat("pleased").id, seat("furious").id, st.crown[0].id]);
      const houseMarks = await page.evaluate(() => [...window.ffh.S.cities.values()].filter((c) => c.house).length);
      const ownHouseCities = ["friendly", "pleased", "annoyed", "furious"].reduce((n, r) => n + st.houses[r].cities.length, 0);
      check(houseMarks >= ownHouseCities, `cities with a House mark in the view (${houseMarks}; the Houses' own: ${ownHouseCities})`);
      // A House is drawn in its liege's colour (rule 1): yours in yours, the neighbour's in its.
      const colours = await page.evaluate(async ([own, nh, liege]) => {
        const T = await import("./app/tiles.js");
        const me = window.ffh.S.view.me.player;
        return { own: own.map((p) => T.playerColor(p) === T.playerColor(me)), neighbour: T.playerColor(nh) === T.playerColor(liege) && T.playerColor(nh) !== T.playerColor(me) };
      }, [["friendly", "pleased", "annoyed", "furious"].map((r) => st.houses[r].player), st.houses.neighbour.player, st.neighbour.player]);
      check(colours.own.every(Boolean) && colours.neighbour, `Houses in their lieges' colours: ${JSON.stringify(colours)}`);
      if (!args["no-map"]) {
        await clear(page);
        const cx = Math.round(at.reduce((s, p) => s + p[0], 0) / at.length);
        const cy = Math.round(at.reduce((s, p) => s + p[1], 0) / at.length);
        await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), [cx, cy]);
        await plateTips(page, st);
      }
      if (SHOTS && !args["no-map"]) {
        await clear(page);
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
      // The diplomacy list on a desktop page (the Houses' lines and the Attitude column).
      await clear(page);
      if (SHOTS) {
        await page.evaluate(() => void window.ffh.openScreen("diplomacy"));
        await until(() => page.evaluate(() => document.querySelectorAll("#modals table.diplo tr.house-row").length > 0), "the diplomacy list", 60000, 200);
        await sleep(500);
        await crop(page, ["#modals table.diplo"], "diplomacy-list-desktop.png");
        await clear(page);
      }
      // H. One alert: a change of state.
      await clear(page);
      const shownAlert = await page.evaluate(async () => {
        const m = await import("./app/persons_lines.js");
        const e = (window.ffh.S.messages ?? []).filter((x) => x.kind === "person" && / is Furious\./.test(x.text)).pop();
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
      // C. A Crown city's Governance line (a House's city is its own player's: no such section).
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
      check(lines.some((t) => t.startsWith("Commander: ")) && lines.some((t) => /Furious/.test(t) && /attack/.test(t)), `commander lines: ${lines.join(" | ")}`);
      for (const [role, name] of [["loyal", "armies-commander-loyal.png"], ["furious", "armies-commander-furious.png"]]) {
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
      check(/^Found a House in .+\. The great person is consumed; a person of note leads the House\./.test(fh), `Found a House: ${fh}`);
      await crop(phone, ["#unit-panel"], "unit-great-person.png", { maxH: 700 });
      await phone.evaluate(() => window.ffh.units.clearSelection());
      await clear(phone);
      // I. The war declaration's preview (the Svartalfar: a commander demands this war).
      await phone.evaluate(() => void window.ffh.openScreen("diplomacy"));
      await until(() => phone.evaluate(() => !!document.querySelector("#modals button[data-war]")), "the Declare war button", 60000, 200);
      await phone.evaluate(() => document.querySelector("#modals button[data-war]").click());
      // The Houses the war brings in for certain (the vassal rules): the neighbour's, at war with
      // you as well, and yours, joining; defensive pacts only may.
      await until(() => phone.evaluate(() => !!document.querySelector("#modals .modal:last-child .war-parties")), "the war dialog's parties", 30000, 100);
      const parties = await phone.evaluate(() => [...document.querySelectorAll("#modals .modal:last-child .war-parties")].map((e) => e.textContent));
      const yours = ["friendly", "pleased", "annoyed", "furious"].map((r) => st.houses[r].label);
      check(parties[0] === `At war with you as well: ${st.houses.neighbour.label}.`, `war dialog: ${parties[0]}`);
      check(parties[1]?.startsWith("Your Houses join: ") && yours.every((l) => parties[1].includes(l)), `war dialog: ${parties[1]}`);
      const pactLine = await phone.evaluate(() => [...document.querySelectorAll("#modals .modal:last-child p")].some((e) => e.textContent === "Their defensive pact partners may join them."));
      check(pactLine, "war dialog: defensive pacts may");
      // (The second build: the stage's persons need not demand this war; a preview shown has lines.)
      await sleep(3000);
      const war = await phone.evaluate(() => document.querySelector("#modals .modal .persons-preview")?.textContent ?? null);
      check(war === null || war.trim().length > 0, `war preview: ${war}`);
      await crop(phone, ["#modals .modal:last-child"], "war-dialog.png");
      await clear(phone);
      await diplomacyHouses(phone, server.url, st);
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
      const perHouse = await phone.evaluate(() => [...document.querySelectorAll("#modals .persons-warning ul.persons-warning-houses li")].map((l) => l.textContent));
      check(/^Adopting .+: \d+ Houses break away, taking \d+ cities, each at war with you\./.test(warn), `civics warning: ${warn}`);
      check(perHouse.length >= 2 && perHouse.some((l) => / returns? to you \(an army of yours stands in /.test(l)), `civics warning, a line per House: ${perHouse.join(" | ")}`);
      check(button.some((b) => /^Adopt .+: \d+ Houses? breaks? away$/.test(b)), `the revolution's button: ${button.join(", ")}`);
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
