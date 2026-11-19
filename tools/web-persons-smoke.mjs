#!/usr/bin/env node
// The browser smoke test of the Houses (docs/PERSONS.md, "The third build" on "The second build",
// rule 13; "UI", parts A, B, D and J; docs/HUMAN.md, "Testing the browser client"), in the style of
// tools/web-smoke.mjs: `ffh-play stage persons` served with `ffh-play serve --load`, the page in
// headless Chromium on the HD-2D map (with `--shots`; else the map the page chooses). It checks the
// Court button's count; opens the Court with Shift+F9 (the header with the Crown's limit and the
// realm's default tribute, the lines that need attention, the Crown and the five Houses, each with
// its leader and its request; no person anywhere; the courted House's card with the courtship
// and its gold-only tribute line, the third build's rules 7 and 10); opens a
// House's tribute list, reads each rate once in force and when (a step every 5 turns), sets another
// rate and sees the card say the rate set and the rate in force while its parts, total and state
// stay the rate in force's; opens the grant list of the contested Crown city (the claimants
// first, the card's lines: the city moving to the House's realm, the maintenance, the Crown back
// within its limit, the tribute), grants it to the claimant House in two steps (the confirmation
// line names the city and the House and says it cannot be undone), sees the toast and the Court
// refreshed (the city is the House's); opens a House's Free confirmation and backs out; opens the
// Armies screen's Requisition from list (a House city that takes levies, one that refuses, in the
// rules' words; the army has no commander line); on a phone (390 x 844) opens the Court, a House's
// tribute list and a grant list without a sideways scroll; the third build's part 2A (a House under
// contract: its line, its floor among its parts, the rates its bound refuses barred in its tribute
// list; a grant that brings a House to the split's threshold says so, and the card of a House that
// may split says so); and last, with the human's Houses freed,
// the Court's founding line and the grant list's founding card (`A new House: its leader is drawn
// from the Bannor's minor leaders.`), founded in two steps. Any console error, page error or failed
// request fails it.
//
//   node tools/web-persons-smoke.mjs --bin <built>/ffh-play [--assets crates/ffh-play/assets]
//                                    [--shots DIR] [--gpu]
//
// With `--shots DIR` it also saves the screenshots of the UI (CLAUDE.md, "Show UI changes in a
// screenshot"): crops at deviceScaleFactor 2, desktop and phone, with the page on the HD-2D map
// (`?renderer=hd2d&force3d=1`: under SwiftShader minutes, not seconds). Without it the page draws
// the map it chooses (without a GPU, the isometric one): the dialogs are the same.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value", shots: "value", gpu: "flag" });
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const SHOTS = args.shots ? path.resolve(args.shots) : null;
const VIEW = { width: 1280, height: 800 };
const PHONE = { width: 390, height: 844 };
/** The waits are longer while the HD-2D map draws in software (`--shots`). */
const SLOW = SHOTS ? 8 : 1;
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

const errors = [];
/** The steps passed, told as they pass. */
const steps = { push: (x) => console.log(`  ok: ${x}`) };
function check(ok, what) {
  if (!ok) throw new Error(`check failed: ${what}`);
}

/** A page on the served staged save at the human's turn, its popups answered or set aside. */
async function open(browser, url, viewport, opts = {}) {
  const page = await browser.newPage({ viewport, deviceScaleFactor: 2, ...opts });
  page.on("console", (m) => m.type() === "error" && errors.push(`console: ${m.text()}`));
  page.on("pageerror", (e) => errors.push(`page error: ${e.message}`));
  page.on("requestfailed", (r) => errors.push(`request failed: ${r.url()} ${r.failure()?.errorText ?? ""}`));
  page.on("response", (r) => r.status() >= 400 && errors.push(`HTTP ${r.status()}: ${r.url()}`));
  // The screenshots are taken on the HD-2D map (CLAUDE.md; SwiftShader here: a frame takes
  // seconds); the checks alone run on the page's own choice (without a GPU: the isometric map).
  await page.goto(SHOTS ? `${url}/?renderer=hd2d&force3d=1` : url);
  await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 240000, 250);
  console.log(`  the page is up (${viewport.width} x ${viewport.height})`);
  await clearPopups(page);
  return page;
}

/** Answers the event popups (their first option) and sets the others aside (Escape) until none
 *  is open for a moment. */
async function clearPopups(page) {
  let quiet = 0;
  for (let i = 0; i < 80 && quiet < 6; i++) {
    const open = await page.evaluate(() => {
      const m = [...document.querySelectorAll("#modals .modal")].pop();
      if (!m) return false;
      const opt = m.querySelector("button.option:not([disabled])");
      if (opt) opt.click();
      return true;
    });
    if (open) {
      quiet = 0;
      await page.keyboard.press("Escape");
    } else quiet++;
    await sleep(250);
  }
  await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
}

/** A crop around the elements `sel` matches (`pad` CSS px), saved as `name` when shots are on. */
async function crop(page, sel, name, { pad = 6 } = {}) {
  if (!SHOTS) return;
  const r = await page.evaluate((sel) => {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const e of document.querySelectorAll(sel)) {
      const b = e.getBoundingClientRect();
      if (!b.width || !b.height) continue;
      x0 = Math.min(x0, b.left);
      y0 = Math.min(y0, b.top);
      x1 = Math.max(x1, b.right);
      y1 = Math.max(y1, b.bottom);
    }
    return x0 === Infinity ? null : { x0, y0, x1, y1, w: innerWidth, h: innerHeight };
  }, sel);
  check(r, `${name}: ${sel} on screen`);
  const x = Math.max(0, Math.floor(r.x0 - pad));
  const y = Math.max(0, Math.floor(r.y0 - pad));
  const w = Math.min(Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(Math.ceil(r.y1 + pad) - y, r.h - y);
  await page.screenshot({ path: path.join(SHOTS, name), clip: { x, y, width: w, height: h }, timeout: 300000 });
  console.log(`  shot ${name}: ${w} x ${h} CSS px`);
}

const TOP = "#modals .backdrop:last-child";

/** Clicks the element `sel` once it is there (the DOM's own click: under SwiftShader a pointer
 *  click waits for frames that take seconds). */
async function tap(page, sel) {
  await until(() => page.evaluate((sel) => !!document.querySelector(sel), sel), sel, 20000 * SLOW);
  await page.evaluate((sel) => document.querySelector(sel).click(), sel);
}

/** The top dialog's tag. */
const topTag = (page) => page.evaluate(() => [...document.querySelectorAll("#modals .modal")].pop()?.dataset.tag ?? null);

/** Whether the page and the top dialog fit the width (no sideways scroll). */
const fits = (page) =>
  page.evaluate(() => {
    const m = [...document.querySelectorAll("#modals .modal")].pop();
    const body = m?.querySelector(".modal-body");
    const over = [...(m?.querySelectorAll(".pcard *") ?? [])].filter((e) => e.getBoundingClientRect().right > innerWidth + 1).map((e) => e.className || e.tagName);
    return { page: document.documentElement.scrollWidth <= innerWidth + 1, body: !body || body.scrollWidth <= body.clientWidth + 1, over: over.slice(0, 5) };
  });


/** The Court's card of House `id`. */
const houseSel = (id) => `.modal[data-tag=court] .house-card[data-house="${id}"]`;

/** House card `id`'s tribute line, loyalty parts, total and state as the page shows them. */
const cardState = (page, id) =>
  page.evaluate((sel) => {
    const c = document.querySelector(sel);
    if (!c) return null;
    return {
      line: c.querySelector(".ptribute-line")?.textContent ?? "",
      parts: [...c.querySelectorAll(".pparts-block ul.pparts li")].map((l) => l.textContent),
      total: c.querySelector(".pparts-block .ptotal")?.textContent ?? "",
      state: c.querySelector(".phead > .pstate")?.dataset.state ?? "",
      open: !!c.querySelector("details.ptribute")?.open,
    };
  }, houseSel(id));

/** The third build, part 2A (docs/PERSONS.md, rules 4 and 6): House `annoyed` under a contract
 *  (its line `Contract until turn N: no tribute above 10%.`, its floor among its parts, the rates
 *  above 10% barred in its tribute list with the contract's reason); the grant list of a Crown
 *  city states that House `large` (5 cities) would hold 6 (`... will hold 6 cities: 5% a turn to
 *  split.`); granted, its card says `Holds 6 cities: 5% a turn to split.` */
async function contractAndSplit(browser, url, st) {
  const page = await open(browser, url, VIEW);
  const houseV = st.houses.annoyed;
  const houseL = st.houses.large;
  await page.evaluate(() => void window.ffh.openScreen("court"));
  await until(() => page.evaluate(() => !!document.querySelector(".modal[data-tag=court] .pcards")), "the Court", 20000 * SLOW);
  const vsel = houseSel(houseV.id);
  const contract = await page.evaluate((sel) => document.querySelector(`${sel} > .pcontract`)?.textContent ?? null, vsel);
  check(/^Contract until turn \d+: no tribute above 10%\.$/.test(contract ?? ""), `the contract's line: ${contract}`);
  const vstate = await cardState(page, houseV.id);
  check(vstate.parts.some((p) => /^Contract until turn \d+: at least Annoyed( \(\+\d+\))?\.$/.test(p)) && vstate.state === "annoyed", `the contract's floor among the parts: ${JSON.stringify(vstate)}`);
  await page.evaluate((sel) => {
    const c = document.querySelector(sel);
    c.scrollIntoView({ block: "start" });
    c.querySelector("details.ptribute > summary").click();
  }, vsel);
  await until(() => page.evaluate((sel) => document.querySelectorAll(`${sel} details.ptribute[open] button.trate`).length === 6, vsel), "its tribute list", 5000 * SLOW);
  const rows = await page.evaluate((sel) => [...document.querySelectorAll(`${sel} button.trate`)].map((b) => ({ pct: Number(b.dataset.pct), barred: b.classList.contains("barred"), why: b.querySelector(".trate-barred")?.textContent ?? null })), vsel);
  const why = `Your contract with ${houseV.label} forbids it until turn `;
  check(rows.every((r) => r.barred === r.pct > 10 && (r.barred ? r.why?.startsWith(why) : r.why === null)), `the rates the contract bars: ${JSON.stringify(rows)}`);
  steps.push(`${houseV.label}: "${contract}"; rates above 10% barred ("${rows.find((r) => r.barred)?.why}")`);
  await crop(page, vsel, "court-house-contract.png", { pad: 4 });
  await page.evaluate((sel) => document.querySelector(`${sel} details.ptribute`).scrollIntoView({ block: "center" }), vsel);
  await crop(page, `${vsel} details.ptribute`, "court-house-tribute-barred.png", { pad: 6 });
  // A tap on a barred rate sets nothing.
  await page.evaluate((sel) => document.querySelector(`${sel} button.trate[data-pct="40"]`).click(), vsel);
  await sleep(500);
  check((await query(page, "court")).houses.find((x) => x.id === houseV.id)?.tribute?.pct === 10, "a barred rate is not set");
  // The grant list of a Crown city: the split line on House `large`'s card.
  await page.keyboard.press("Escape");
  const crownZ = st.crown[2];
  await page.evaluate((id) => void window.ffh.persons.openGrant(id), crownZ.id);
  await until(() => page.evaluate(() => !!document.querySelector(".modal[data-tag=grant] .grant-card")), "the grant list", 20000 * SLOW);
  const n = await page.evaluate((label) => [...document.querySelectorAll(".modal[data-tag=grant] .grant-card")].findIndex((c) => c.querySelector(".pname").textContent === label), houseL.label);
  check(n >= 0, `House large's card in the grant list of ${crownZ.name}`);
  const lsel = `.modal[data-tag=grant] .grant-card:nth-child(${n + 1})`;
  const lines = await page.evaluate((sel) => [...document.querySelectorAll(`${sel} ul.plines li`)].map((l) => l.textContent), lsel);
  check(lines.includes(`${houseL.label} will hold 6 cities: 5% a turn to split.`), `the grant's split line: ${JSON.stringify(lines)}`);
  steps.push(`the grant list of ${crownZ.name}: "${houseL.label} will hold 6 cities: 5% a turn to split."`);
  await page.evaluate((sel) => document.querySelector(sel).scrollIntoView({ block: "start" }), lsel);
  await crop(page, lsel, "grant-split-line.png", { pad: 4 });
  // Granted: the card says it may split.
  await tap(page, `${lsel} button.pgrant-to`);
  await until(() => page.evaluate((sel) => !!document.querySelector(`${sel} button.pgrant-yes`), lsel), "the confirmation", 5000 * SLOW);
  await tap(page, `${lsel} button.pgrant-yes`);
  await until(async () => (await topTag(page)) !== "grant", "the grant list closed", 20000 * SLOW);
  await page.evaluate(() => document.querySelectorAll("#modals .modal").forEach(() => {}));
  if ((await topTag(page)) !== "court") {
    await page.evaluate(() => void window.ffh.openScreen("court"));
  }
  const lcard = houseSel(houseL.id);
  const split = await until(() => page.evaluate((sel) => document.querySelector(`${sel} > .psplit`)?.textContent ?? null, lcard), "the split line on House large's card", 20000 * SLOW);
  check(split === "Holds 6 cities: 5% a turn to split.", `the card's split line: ${split}`);
  steps.push(`${houseL.label}'s card: "${split}"`);
  await page.evaluate((sel) => document.querySelector(sel).scrollIntoView({ block: "start" }), lcard);
  await crop(page, lcard, "court-house-split.png", { pad: 4 });
  await page.close();
}

/** A query (`{k: "q", q}`) from the page's API, its reply. */
function query(page, q) {
  return page.evaluate((q) => window.ffh.api.query(q), q);
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("web-persons-smoke", "the playwright package is not installed");
  if (SHOTS) fs.mkdirSync(SHOTS, { recursive: true });
  const saves = tempDir("persons-smoke");
  const st = await stage(bin, "persons", path.join(saves, "staged"));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  let browser;
  try {
    browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  } catch (e) {
    await server.stop();
    return skipped("web-persons-smoke", `no browser: ${e.message.split("\n")[0]}`);
  }
  try {
    const page = await open(browser, server.url, VIEW);
    const houseA = st.houses.friendly;
    const houseP = st.houses.pleased;
    const houseV = st.houses.annoyed;
    const taken = st.taken;

    // The Court button's count: a Furious House.
    const badge = await until(() => page.evaluate(() => document.querySelector("#btn-court .court-badge")?.textContent ?? null), "the Court button's count", 20000 * SLOW);
    check(Number(badge) >= 1, `the Court's count is at least 1 (got ${badge})`);
    steps.push(`the Court button's count: ${badge}`);
    await crop(page, "#btn-court", "court-button.png", { pad: 10 });

    // Shift+F9 opens the Court.
    await page.keyboard.press("Shift+F9");
    await until(async () => (await topTag(page)) === "court" && page.evaluate(() => !!document.querySelector(".modal[data-tag=court] .pcards")), "the Court (Shift+F9)", 20000 * SLOW);
    const court = await page.evaluate(() => {
      const m = document.querySelector(".modal[data-tag=court]");
      return {
        header: [...m.querySelectorAll(".court-header > div")].map((d) => d.textContent),
        crown: m.querySelector(".court-header .court-crown")?.textContent ?? "",
        crownCaution: !!m.querySelector(".court-header .court-crown.caution"),
        tributeDefault: m.querySelector(".court-header select.tribute-default")?.value ?? null,
        attention: [...m.querySelectorAll(".pattn")].map((d) => ({ kind: d.dataset.kind, text: d.textContent })),
        houses: m.querySelectorAll(".house-card").length,
        crownCities: m.querySelectorAll(".crown-card .crown-city").length,
        over: [...m.querySelectorAll(".crown-card .crown-city.over")].map((c) => c.textContent),
        persons: m.querySelectorAll(".person-card, .commander-line, .person-line").length,
        leaders: [...m.querySelectorAll(".house-card .pleader")].map((e) => e.textContent),
        requests: [...m.querySelectorAll(".house-card .pdemand")].map((e) => e.textContent),
        chips: [...m.querySelectorAll(".court-keywords .kw-chip")].map((c) => c.textContent),
        marks: [...m.querySelectorAll(".house-card .phead > .pstate")].map((s) => s.textContent),
      };
    });
    check(court.header[0]?.startsWith("Government: Aristocracy."), `the header: ${court.header}`);
    check(/^The Crown holds 4 cities, 1 over its limit of 3\.$/.test(court.crown) && court.crownCaution, `the Crown's limit, in the caution colour, without its cost: ${court.crown}`);
    check(court.tributeDefault === "20", `the realm's default tribute: ${court.tributeDefault}`);
    const kinds = court.attention.map((a) => a.kind);
    check(kinds.indexOf("furious") === 0 && kinds.indexOf("levy") > kinds.lastIndexOf("furious") && kinds.indexOf("crown_over") > kinds.indexOf("levy") && kinds.indexOf("grantable") > kinds.indexOf("crown_over"), `the needs-attention lines' order: ${kinds}`);
    check(court.attention.some((a) => /Furious\. Issues an ultimatum at your next turn\. Refused, it breaks away, taking .*: war\./.test(a.text)), `the Furious House's line: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.attention.some((a) => a.text.startsWith(`${houseV.label} is Annoyed. It takes no levy.`)), `the House that takes no levy: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.attention.some((a) => a.kind === "crown_over" && a.text.includes(`${taken.name} is over the Crown's limit of 3: maintenance +`)), `the city over the Crown's limit, its cost signed: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.houses === 5 && court.crownCities === 4 && court.over.length === 1 && court.persons === 0, `the cards (no person): ${JSON.stringify(court)}`);
    check(court.leaders.length === 5 && court.leaders.every((t) => t.length > 0), `every House card names its leader: ${JSON.stringify(court.leaders)}`);
    check(court.requests.length === 5 && court.requests.every((t) => t.startsWith("Request: ") || t.startsWith("No request.")) && court.requests.some((t) => t.startsWith("Request: ")), `the requests: ${JSON.stringify(court.requests)}`);
    check(court.marks.some((t) => t.includes("Furious") && t.includes("✖")) && !court.marks.some((t) => /turns?$/.test(t)), `the Furious mark, no countdown: ${court.marks}`);
    check(["House", "Crown", "Request", "Ultimatum", "Tribute", "Levy", "Claimant"].every((k) => court.chips.includes(k)) && !court.chips.includes("Furious") && !court.chips.includes("Sworn"), `the keyword chips: ${court.chips}`);
    steps.push(`the Court: ${court.header.join(" ")} ${court.attention.length} lines need attention (${kinds.join(", ")}); the Crown (${court.crownCities} cities), ${court.houses} Houses led by ${court.leaders.join("; ")}`);
    await crop(page, ".modal[data-tag=court] .court-header, .modal[data-tag=court] .court-attention", "court-attention.png");
    await crop(page, ".modal[data-tag=court]", "court.png", { pad: 2 });
    // The rest of the Court, a screen at a time: the realm's cards.
    if (SHOTS) {
      const scrollTo = (sel) => page.evaluate((sel) => document.querySelector(sel)?.scrollIntoView({ block: "start" }), sel);
      await scrollTo(".modal[data-tag=court] .pcards");
      await crop(page, ".modal[data-tag=court]", "court-realm.png", { pad: 2 });
      await scrollTo(".modal[data-tag=court] .house-card:nth-child(4)");
      await crop(page, ".modal[data-tag=court]", "court-realm-2.png", { pad: 2 });
      await page.evaluate(() => (document.querySelector(".modal[data-tag=court] .modal-body").scrollTop = 0));
    }

    // A request's term says what its lapse does; a House that would accept an offer shows its
    // price on the card.
    const cards = await page.evaluate((v) => {
      const m = document.querySelector(".modal[data-tag=court]");
      const lapse = [...m.querySelectorAll(".pdemand-lapse")].map((e) => e.textContent);
      const price = (id) => m.querySelector(`.house-card[data-house="${id}"] > .pprice`)?.textContent ?? null;
      return { lapse, priceV: price(v.annoyed), priceA: price(v.friendly) };
    }, { annoyed: houseV.id, friendly: houseA.id });
    check(cards.lapse.length > 0 && cards.lapse.every((t) => /^ At its term: -3 \(60 turns\)(, becomes .+?(: an ultimatum)?)?\.$/.test(t)), `the requests' lapse: ${JSON.stringify(cards.lapse)}`);
    check(/^Price to secede: \d+ gold\.$/.test(cards.priceV ?? "") && cards.priceA === null, `the price in the card (Annoyed: ${cards.priceV}; Friendly: ${cards.priceA})`);
    steps.push(`request terms: "${cards.lapse[0].trim()}"; ${houseV.label}: "${cards.priceV}"`);
    // The Furious House's outcome: the contest for each city where an army of yours stands (rule
    // 7: held when your army's units outnumber the House's; its seat contested and lost).
    const houseC = st.houses.furious;
    const outcome = await page.evaluate((sel) => document.querySelector(`${sel} > .pline.warn`)?.textContent ?? null, houseSel(houseC.id));
    check(/ returns to you: your army's \d+ units? outnumbers? its \d+\./.test(outcome ?? "") && / goes with it: its \d+ units? against your army's \d+\./.test(outcome ?? ""), `the Furious House's outcome, held and contested: ${outcome}`);
    steps.push(`${houseC.label}'s outcome: "${outcome}"`);
    if (SHOTS) {
      const shoot = async (sel, name) => {
        await page.evaluate((sel) => document.querySelector(sel)?.scrollIntoView({ block: "start" }), sel);
        await sleep(300);
        await crop(page, sel, name, { pad: 4 });
      };
      await shoot(houseSel(houseV.id), "court-house-price.png");
      await shoot(houseSel(houseC.id), "court-house-furious.png");
      const requestCard = await page.evaluate(() => {
        const c = document.querySelector(".modal[data-tag=court] .house-card .pdemand-lapse")?.closest(".house-card");
        return c ? `.modal[data-tag=court] .house-card[data-house="${c.dataset.house}"]` : null;
      });
      if (requestCard) await shoot(requestCard, "court-house-request.png");
      await page.evaluate(() => (document.querySelector(".modal[data-tag=court] .modal-body").scrollTop = 0));
    }

    // ---- The third build, part 2B (rules 7 and 10): the neighbour courts House `pleased` (a
    // resource and gold, open); its card says so, and its tribute line is gold only.
    {
      const courted = await page.evaluate((sel) => {
        const c = document.querySelector(sel);
        return { lines: [...c.querySelectorAll(".pcourted")].map((e) => e.textContent), tribute: c.querySelector(".ptribute-line")?.textContent ?? "" };
      }, houseSel(houseP.id));
      check(courted.lines.length === 1 && /^Courted by the \S.* \(gifts: \d+ gold, [^)]+\)\. Offers possible from turn \d+\.$/.test(courted.lines[0]), `the courted House's card: ${JSON.stringify(courted.lines)}`);
      check(!/research/.test(courted.tribute) && /^Tribute \d+%(: \d+ gold last turn\.|\. Not paid yet\.)$/.test(courted.tribute), `its tribute line, gold only: ${courted.tribute}`);
      const others = await page.evaluate((id) => [...document.querySelectorAll(`.modal[data-tag=court] .house-card:not([data-house="${id}"]) .pcourted`)].length, houseP.id);
      check(others === 0, `no other House of yours is courted (${others})`);
      steps.push(`${houseP.label}'s card: "${courted.lines[0]}"; tribute "${courted.tribute}"`);
      if (SHOTS) {
        await page.evaluate((sel) => document.querySelector(sel)?.scrollIntoView({ block: "start" }), houseSel(houseP.id));
        await sleep(300);
        await crop(page, houseSel(houseP.id), "court-house-courted.png", { pad: 4 });
        await page.evaluate(() => (document.querySelector(".modal[data-tag=court] .modal-body").scrollTop = 0));
      }
    }

    // A state mark's tooltip: the attitude's help from the data.
    const tipText = await page.evaluate(async (sel) => {
      const mark = document.querySelector(`${sel} .phead > .pstate`);
      mark.dispatchEvent(new MouseEvent("mouseenter", { bubbles: false, clientX: 200, clientY: 200 }));
      await new Promise((r) => setTimeout(r, 300));
      const t = document.querySelector(".tip-side .reminder-text")?.textContent ?? null;
      mark.dispatchEvent(new MouseEvent("mouseleave", { bubbles: true }));
      return t;
    }, houseSel(houseA.id));
    check(/^Loyalty \+3 or more\. A House takes levies/.test(tipText ?? ""), `the Friendly mark's tooltip (the attitude's help): ${tipText}`);
    steps.push(`the Friendly mark's tooltip: "${tipText}"`);
    await page.mouse.move(2, 2);

    // A House's tribute list: open it, read the rates, set another; the card says what the row did.
    const card = houseSel(houseA.id);
    await page.evaluate((sel) => {
      const c = document.querySelector(sel);
      c.scrollIntoView({ block: "start" });
      c.querySelector("details.ptribute > summary").click();
    }, card);
    await until(() => page.evaluate((sel) => document.querySelectorAll(`${sel} details.ptribute[open] button.trate`).length === 6, card), "the tribute list (6 rates)", 5000 * SLOW);
    const readRates = () =>
      page.evaluate((sel) =>
        [...document.querySelectorAll(`${sel} button.trate`)].map((b) => ({ pct: Number(b.dataset.pct), current: b.getAttribute("aria-pressed") === "true", now: !!b.querySelector(".trate-now"), part: b.querySelector(".trate-part").textContent, when: b.querySelector(".trate-when")?.textContent ?? "", state: b.querySelector(".pstate").dataset.state, est: b.querySelector(".trate-est").textContent })),
      card);
    const rates = await readRates();
    const now = rates.find((r) => r.current);
    check(now?.pct === 0 && rates.filter((r) => r.current).length === 1 && now.now, `the rate set (House A pays 0%), marked now: ${JSON.stringify(rates)}`);
    // Each row, once in force (rule 4): when (`In force in 15 turns.`: a step of 10% every 5 turns
    // from the rate in force), its part and the total then.
    const ROW = /^(In force\.|In force in (\d+) turns?\.) Tribute: ([+-]?\d+) · Loyalty: ([+-]?\d+)$/;
    check(rates.every((r) => /^Pays \d+ gold\.$/.test(r.est) && ROW.test(r.part)), `the rows' lines: ${JSON.stringify(rates)}`);
    check(rates.every((r) => Number(ROW.exec(r.part)?.[2] ?? 0) === (Math.abs(r.pct - now.pct) / 10) * 5), `each row's turns until in force: ${JSON.stringify(rates)}`);
    const before = await cardState(page, houseA.id);
    check(/^Tribute 0%: \d+ gold last turn\.$|^Tribute 0%\. Not paid yet\.$/.test(before.line), `the line of a rate at rest: ${before.line}`);
    await page.evaluate((sel) => document.querySelector(`${sel} details.ptribute`).scrollIntoView({ block: "center" }), card);
    await crop(page, `${card} details.ptribute`, "court-house-tribute.png", { pad: 6 });
    const pick = rates.find((r) => r.pct === 30);
    const [, whenWant, turnsWant, partWant, totalWant] = ROW.exec(pick.part);
    await tap(page, `${card} button.trate[data-pct="30"]`);
    await until(async () => (await cardState(page, houseA.id))?.line.startsWith("Tribute 30%"), "the tribute set to 30%", 10000 * SLOW);
    // Set: the line says both rates and when; the loyalty is still the rate in force's.
    const after = await cardState(page, houseA.id);
    check(after.line.startsWith(`Tribute 30%, now 0%: reaches 30% in ${turnsWant} turns. `), `the line of a moving rate (${whenWant}): ${after.line}`);
    check(after.parts.includes("Tribute 0%: +1") && !after.parts.some((p) => p.startsWith("Tribute 30%")), `the part is the rate in force's: ${JSON.stringify(after)}`);
    check(after.total === before.total && after.state === before.state && after.open, `the total and state unchanged until the rate is in force, the list kept open: ${JSON.stringify(after)}`);
    const moved = await readRates();
    const set30 = moved.find((r) => r.pct === 30);
    const at0 = moved.find((r) => r.pct === 0);
    check(set30.current && set30.now && set30.when === whenWant && set30.part === pick.part && moved.filter((r) => r.current).length === 1, `the rate set marked now, its row as before: ${JSON.stringify(moved)}`);
    check(at0.when === "In force." && !at0.current, `the rate in force's row: ${JSON.stringify(at0)}`);
    check(!after.parts.some((p) => /raised/i.test(p)), `no raise modifier: ${JSON.stringify(after.parts)}`);
    steps.push(`${houseA.label}'s tribute set 0% -> 30%: "${after.line}"; the row: "${set30.part}" (${partWant} once in force, total ${totalWant}); parts still ${after.parts.filter((p) => p.startsWith("Tribute ")).join(", ")}, ${after.total}`);
    await page.evaluate((sel) => document.querySelector(`${sel} details.ptribute`).scrollIntoView({ block: "center" }), card);
    await crop(page, `${card} details.ptribute`, "court-house-tribute-moving.png", { pad: 6 });
    await page.evaluate((sel) => document.querySelector(sel).scrollIntoView({ block: "start" }), card);
    await crop(page, card, "court-house-card.png", { pad: 4 });

    // The grant list of the contested Crown city, from the Crown's card.
    await tap(page, `.modal[data-tag=court] .crown-card button.pgrant[data-city="${taken.id}"]`);
    await until(async () => (await topTag(page)) === "grant" && page.evaluate(() => !!document.querySelector(".modal[data-tag=grant] .grant-card")), "the grant list", 20000 * SLOW);
    const grant = await page.evaluate(() => {
      const m = document.querySelector(".modal[data-tag=grant]");
      const cards = [...m.querySelectorAll(".grant-card")];
      return {
        title: m.querySelector(".modal-head h2").textContent,
        head: m.querySelector(".grant-head").textContent,
        claimants: cards.map((c) => c.dataset.claimant === "1"),
        kinds: cards.map((c) => c.dataset.kind),
        first: { title: cards[0].querySelector(".pname").textContent, lines: [...cards[0].querySelectorAll("ul.plines li")].map((l) => l.textContent), effects: [...cards[0].querySelectorAll(".peffect:not(.self)")].map((e) => e.textContent) },
      };
    });
    check(grant.title === `Grant ${taken.name}` && /A grant is for good\./.test(grant.head) && /Held by the Crown: its maintenance/.test(grant.head) && /The Crown holds 4 cities, 1 over its limit of 3/.test(grant.head), `the grant list's head: ${grant.title} / ${grant.head}`);
    const k = grant.claimants.indexOf(false);
    check(grant.claimants[0] && (k < 0 || grant.claimants.slice(k).every((x) => !x)), `claimants first: ${grant.claimants}`);
    const l = grant.first.lines;
    check(/^Moves to .*'s realm: its production and commerce are the House's\.$/.test(l[0]) && /^(Its garrison goes to .+: .+|No garrison goes with it)\.$/.test(l[1]) && l.some((x) => /^Your maintenance: -[\d.]+ gold a turn \([\d.]+ of it the Crown's limit\)\.$/.test(x)) && l.includes("The Crown after the grant: 3 cities, within its limit of 3.") && l.some((x) => /^Tribute from it at \d+%\.$/.test(x)), `the first card's lines: ${JSON.stringify(l)}`);
    check(grant.first.effects.some((e) => /granted to .*: -\d \(30 turns\)\./.test(e)), `the first card's snubs: ${JSON.stringify(grant.first.effects)}`);
    check(grant.kinds.length === 5 && grant.kinds.every((x) => x === "house"), `the realm's five Houses, no founding while one stands: ${grant.kinds}`);
    steps.push(`the grant list of ${taken.name}: ${grant.claimants.filter(Boolean).length} claimants first; ${grant.first.title}: ${l.join(" ")} ${grant.first.effects.length} other subjects in its preview`);
    await crop(page, ".modal[data-tag=grant]", "grant-list.png", { pad: 2 });
    await crop(page, ".modal[data-tag=grant] .grant-head, .modal[data-tag=grant] .grant-card:first-child", "grant-first-card.png", { pad: 4 });

    // Two steps, to the claimant House: the button, then the confirmation line, then Grant.
    const nth = await page.evaluate((label) => [...document.querySelectorAll(".modal[data-tag=grant] .grant-card")].findIndex((c) => c.querySelector(".pname").textContent === label), houseP.label);
    check(nth >= 0, `the claimant House ${houseP.label} has a card`);
    const sel = `.modal[data-tag=grant] .grant-card:nth-child(${nth + 1})`;
    const target = await page.evaluate((sel) => document.querySelector(sel).querySelector(".pname").textContent, sel);
    await page.evaluate((sel) => document.querySelector(sel).scrollIntoView({ block: "start" }), sel);
    await tap(page, `${sel} button.pgrant-to`);
    const confirm = await until(() => page.evaluate((sel) => document.querySelector(`${sel} .pconfirm p`)?.textContent ?? null, sel), "the confirmation line", 5000 * SLOW);
    check(confirm === `Grant ${taken.name} to ${target}? This cannot be undone.`, `the confirmation: ${confirm}`);
    const garrison = await page.evaluate((sel) => [...document.querySelectorAll(`${sel} .pconfirm p.pconfirm-garrison`)].map((p) => p.textContent), sel);
    check(garrison.length > 0 && /^(Its garrison goes to .+|No garrison goes with it)\.$/.test(garrison[0]), `the confirmation's garrison: ${JSON.stringify(garrison)}`);
    await crop(page, sel, "grant-confirm.png", { pad: 4 });
    await tap(page, `${sel} button.pgrant-yes`);
    await until(async () => (await topTag(page)) === "court", "the grant list closed", 20000 * SLOW);
    const toast = await until(() => page.evaluate((t) => [...document.querySelectorAll("#toasts .toast")].map((x) => x.textContent).find((x) => x.includes(t)) ?? null, `${taken.name} granted to ${target}.`), "the grant's toast", 10000 * SLOW);
    // The city changes hands (a new id): the House's card names it; the Crown is within its limit.
    await until(
      () =>
        page.evaluate(
          ([id, name, house]) =>
            !document.querySelector(`.modal[data-tag=court] .crown-card [data-city="${id}"]`) &&
            [...document.querySelectorAll(`.modal[data-tag=court] .house-card[data-house="${house}"] .city-chip`)].some((c) => c.textContent === name) &&
            /^The Crown holds 3 cities; its limit is 3\./.test(document.querySelector(".modal[data-tag=court] .court-crown")?.textContent ?? ""),
          [taken.id, taken.name, houseP.id],
        ),
      "the Court refreshed: the city is the House's, the Crown within its limit",
      10000 * SLOW,
    );
    steps.push(`granted in two steps ("${confirm}"); the toast "${toast}"; the Court refreshed`);

    // Freeing a House: the confirmation line, then Back (nothing changes).
    await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
    await tap(page, `${card} button.prelease`);
    const freeLine = await until(() => page.evaluate((sel) => document.querySelector(`${sel} .pconfirm p`)?.textContent ?? null, card), "the Free confirmation", 5000 * SLOW);
    check(freeLine === `Free ${houseA.label}? It is no longer your vassal, at peace with you. This cannot be undone.`, `the Free confirmation: ${freeLine}`);
    await page.evaluate((sel) => document.querySelector(`${sel} .pfoot`).scrollIntoView({ block: "center" }), card);
    await crop(page, `${card} .pfoot`, "court-free.png", { pad: 4 });
    await tap(page, `${card} .pconfirm button.small:not(.danger)`);
    await until(() => page.evaluate((sel) => !!document.querySelector(`${sel} button.prelease`), card), "back from the Free confirmation", 5000 * SLOW);
    steps.push(`${houseA.label}'s Free confirmation ("${freeLine}"), then Back`);
    await page.keyboard.press("Escape");

    // The Armies screen: Requisition from lists the Houses' cities with their levy status.
    await until(async () => (await topTag(page)) === null, "the Court closed", 5000 * SLOW);
    await page.evaluate(() => void window.ffh.openScreen("armies"));
    const armySel = `#modals .army-card[data-army="${st.armies.main.id}"]`;
    await until(() => page.evaluate((sel) => !!document.querySelector(`${sel} details.requisition-pick`), armySel), "the Armies screen", 20000 * SLOW);
    check(await page.evaluate(() => !document.querySelector("#modals .commander-line, #modals .army-card select.commander-change")), "no commander on the Armies screen");
    await crop(page, armySel, "armies.png", { pad: 4 });
    await page.evaluate((sel) => {
      const d = document.querySelector(`${sel} details.requisition-pick`);
      d.open = true;
      d.scrollIntoView({ block: "start" });
    }, armySel);
    const levy = await page.evaluate((sel) => [...document.querySelectorAll(`${sel} .requisition-city`)].map((r) => ({ city: Number(r.dataset.city), house: r.querySelector(".requisition-house")?.dataset.house ?? null, levy: r.querySelector(".requisition-house")?.dataset.levy ?? null, text: r.querySelector(".levy-text")?.textContent ?? null, mark: r.querySelector(".requisition-house .pstate")?.dataset.state ?? null, note: r.querySelector(".requisition-house .pstate")?.dataset.note ?? null })), armySel);
    const takes = levy.find((x) => x.levy === "yes");
    const refuses = levy.find((x) => x.levy === "no" && x.house === String(houseV.id));
    check(takes?.text === "Takes levies. Each levy: -1 (20 turns)." && /^Annoyed after \d+ lev(y|ies)\.$/.test(takes?.note ?? "") && takes?.mark, `the levy line that takes, its mark's note: ${JSON.stringify(takes)}`);
    // (Annoyed, or Furious when the grant above refused its request for that city.)
    const word = { annoyed: "Annoyed", furious: "Furious" }[refuses?.mark];
    check(refuses?.text === "Takes no levy." && word && refuses?.note === `${houseV.label} is ${word}. It takes no levy.`, `the levy line that refuses: ${JSON.stringify(refuses)}`);
    check(levy.findIndex((x) => x.house) > levy.findIndex((x) => !x.house), `the Crown's cities first: ${JSON.stringify(levy)}`);
    steps.push(`Requisition from: ${levy.filter((x) => !x.house).length} Crown cities, ${levy.filter((x) => x.house).length} Houses' cities ("${takes.text}", the mark's tip "${takes.note}" / "${refuses.text}")`);
    await crop(page, `${armySel} .requisition-cities`, "armies-levy.png", { pad: 4 });
    // The mark's tooltip: the levies before it takes none, over what the state does.
    const tipSel = `${armySel} .requisition-city[data-city="${takes.city}"] .requisition-house .pstate`;
    const tip = await page.evaluate(async (sel) => {
      const mark = document.querySelector(sel);
      const r = mark.getBoundingClientRect();
      mark.dispatchEvent(new MouseEvent("mouseenter", { bubbles: false, clientX: r.left + 4, clientY: r.bottom + 2 }));
      await new Promise((res) => setTimeout(res, 300));
      return document.querySelector(".tooltip.richtip:not([hidden]) .tip-title")?.textContent ?? null;
    }, tipSel);
    check(tip === takes.note, `the levy mark's tooltip: ${tip}`);
    await crop(page, `${tipSel}, .tooltip.richtip:not([hidden]), .tip-side:not([hidden])`, "armies-levy-tip.png", { pad: 6 });
    await page.evaluate((sel) => document.querySelector(sel).dispatchEvent(new MouseEvent("mouseleave", { bubbles: true })), tipSel);
    // A picked city whose House takes no levy: the requisition's head says it is passed over.
    await page.evaluate(([sel, city]) => document.querySelector(`${sel} .requisition-city[data-city="${city}"] input`).click(), [armySel, refuses.city]);
    const head = await until(() => page.evaluate((sel) => document.querySelector(`${sel} .requisition-passed`)?.closest(".requisition-head")?.textContent ?? null, armySel), "the passed-over city in the requisition's head", 10000 * SLOW);
    check(head.includes(`: passed over (${houseV.label} takes no levy).`), `the requisition's head: ${head}`);
    steps.push(`the requisition's head: "${head}"`);
    await page.evaluate((sel) => document.querySelector(`${sel} .requisition-head`).scrollIntoView({ block: "center" }), armySel);
    await crop(page, `${armySel} .requisition-head`, "armies-passed.png", { pad: 4 });
    await page.keyboard.press("Escape");
    await page.close();

    // On a phone: the Court, a House's tribute list and a grant list fit the width.
    const phone = await open(browser, server.url, PHONE, { isMobile: true, hasTouch: true });
    await phone.evaluate(() => void window.ffh.openScreen("court"));
    await until(() => phone.evaluate(() => !!document.querySelector(".modal[data-tag=court] .pcards")), "the Court on a phone", 20000 * SLOW);
    let f = await fits(phone);
    check(f.page && f.body && !f.over.length, `the Court fits a phone: ${JSON.stringify(f)}`);
    await crop(phone, ".modal[data-tag=court]", "phone-court.png", { pad: 0 });
    const pcard = houseSel(houseV.id);
    await phone.evaluate((sel) => {
      const c = document.querySelector(sel);
      c.querySelector("details.ptribute").open = true;
      c.scrollIntoView({ block: "start" });
    }, pcard);
    await sleep(400);
    f = await fits(phone);
    check(f.page && f.body && !f.over.length, `a House's tribute list fits a phone: ${JSON.stringify(f)}`);
    await crop(phone, pcard, "phone-house-tribute.png", { pad: 2 });
    const crownCity = st.crown.find((c) => c.id !== taken.id);
    await phone.evaluate((id) => void window.ffh.persons.openGrant(id), crownCity.id);
    await until(() => phone.evaluate(() => !!document.querySelector(".modal[data-tag=grant] .grant-card")), "a grant list on a phone", 20000 * SLOW);
    f = await fits(phone);
    check(f.page && f.body && !f.over.length, `the grant list fits a phone: ${JSON.stringify(f)}`);
    await crop(phone, ".modal[data-tag=grant]", "phone-grant.png", { pad: 0 });
    steps.push(`on a phone (${PHONE.width} x ${PHONE.height}): the Court, ${houseV.label}'s tribute list and the grant list of ${crownCity.name} without a sideways scroll`);
    await phone.close();

    // ---- the third build, part 2A (docs/PERSONS.md, rules 4 and 6) ----
    await contractAndSplit(browser, server.url, st);

    // A realm without a House (the human's five freed): the Court's founding line, the grant
    // list's founding card, founded in two steps (rule 3).
    const fpage = await open(browser, server.url, VIEW);
    const ids = [houseA.id, houseV.id, st.houses.furious.id, houseP.id, st.houses.large.id];
    await fpage.evaluate(async (ids) => {
      for (const id of ids) await window.ffh.api.cmd("free_house", { house: id });
    }, ids);
    await fpage.evaluate(() => void window.ffh.openScreen("court"));
    const founding = await until(() => fpage.evaluate(() => document.querySelector(".modal[data-tag=court] .crown-card .pfounding")?.textContent ?? null), "the Court's founding line", 20000 * SLOW);
    check(founding === "No House yet. A grant of a Crown city founds one. A new House: its leader is drawn from the Bannor's minor leaders.", `the founding line: ${founding}`);
    check(await fpage.evaluate(() => document.querySelectorAll(".modal[data-tag=court] .house-card").length === 0), "no House card");
    await crop(fpage, ".modal[data-tag=court] .crown-card", "court-founding.png", { pad: 4 });
    await fpage.keyboard.press("Escape");
    await fpage.evaluate((id) => void window.ffh.persons.openGrant(id), crownCity.id);
    await until(() => fpage.evaluate(() => !!document.querySelector(".modal[data-tag=grant] .grant-card")), "the founding grant list", 20000 * SLOW);
    const fcard = await fpage.evaluate(() => {
      const cards = [...document.querySelectorAll(".modal[data-tag=grant] .grant-card")];
      const c = cards[0];
      return { n: cards.length, kind: c.dataset.kind, title: c.querySelector(".pname").textContent, lines: [...c.querySelectorAll("ul.plines li")].map((l) => l.textContent), button: c.querySelector("button.pgrant-to")?.textContent ?? "" };
    });
    check(fcard.n === 1 && fcard.kind === "found" && fcard.title === "A new House", `the founding card: ${JSON.stringify(fcard)}`);
    check(fcard.lines[0] === "A new House: its leader is drawn from the Bannor's minor leaders." && /^Found House .+$/.test(fcard.button), `the founding card's lines: ${JSON.stringify(fcard)}`);
    await crop(fpage, ".modal[data-tag=grant]", "grant-founding.png", { pad: 2 });
    await tap(fpage, ".modal[data-tag=grant] .grant-card button.pgrant-to");
    const fconfirm = await until(() => fpage.evaluate(() => document.querySelector(".modal[data-tag=grant] .pconfirm p")?.textContent ?? null), "the founding's confirmation", 5000 * SLOW);
    check(/^Grant .+ to found House .+\? Its leader is drawn now\. This cannot be undone\.$/.test(fconfirm), `the founding's confirmation: ${fconfirm}`);
    await crop(fpage, ".modal[data-tag=grant] .grant-card", "grant-founding-confirm.png", { pad: 4 });
    await tap(fpage, ".modal[data-tag=grant] button.pgrant-yes");
    await until(async () => (await topTag(fpage)) === null, "the grant list closed", 20000 * SLOW);
    await fpage.evaluate(() => void window.ffh.openScreen("court"));
    const founded = await until(() => fpage.evaluate(() => document.querySelector(".modal[data-tag=court] .house-card .pleader")?.textContent ?? null), "the founded House's card", 20000 * SLOW);
    check(await fpage.evaluate(() => !document.querySelector(".modal[data-tag=court] .pfounding")), "no founding line with a House");
    steps.push(`the founding: "${founding}"; the card "${fcard.button}"; "${fconfirm}"; led by ${founded}`);
    await fpage.close();
  } finally {
    await browser.close();
    await server.stop();
  }
  if (errors.length) {
    for (const e of errors) console.error(`  ${e}`);
    throw new Error(`${errors.length} console or page errors`);
  }
  console.log("web-persons-smoke: OK");
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`web-persons-smoke: FAILED: ${e.stack ?? e}`);
    for (const x of errors) console.error(`  ${x}`);
    process.exit(1);
  },
);
