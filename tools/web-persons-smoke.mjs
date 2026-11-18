#!/usr/bin/env node
// The browser smoke test of Persons of Note (docs/PERSONS.md, "The second build", rule 13; "UI",
// parts A, B, D and J; docs/HUMAN.md, "Testing the browser client"), in the style of
// tools/web-smoke.mjs: `ffh-play stage persons` served with `ffh-play serve --load`, the page in
// headless Chromium on the HD-2D map (with `--shots`; else the map the page chooses). It checks the
// Court button's count; opens the Court with Shift+F9 (the header with the Crown's limit and the
// realm's default tribute, the lines that need attention, the Crown and the four Houses); opens a
// House's tribute list, sets another rate and sees the House's tribute part, total and state
// become what the rate's row said; opens the grant list of the contested Crown city (the claimants
// first, the card's lines: the city moving to the House's realm, the maintenance, the Crown back
// within its limit, the tribute), grants it to the claimant House in two steps (the confirmation
// line names the city and the House and says it cannot be undone), sees the toast and the Court
// refreshed (the city is the House's); opens a House's Free confirmation and backs out; opens the
// Armies screen's Requisition from list (a House city that takes levies, one that refuses, in the
// rules' words); and on a phone (390 x 844) opens the Court, a House's tribute list and a grant
// list without a sideways scroll. Any console error, page error or failed request fails it.
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

    // The Court button's count: a Furious House, a Furious commander.
    const badge = await until(() => page.evaluate(() => document.querySelector("#btn-court .court-badge")?.textContent ?? null), "the Court button's count", 20000 * SLOW);
    check(Number(badge) >= 2, `the Court's count is at least 2 (got ${badge})`);
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
        persons: m.querySelectorAll(".person-card").length,
        chips: [...m.querySelectorAll(".court-keywords .kw-chip")].map((c) => c.textContent),
        marks: [...m.querySelectorAll(".house-card .phead > .pstate")].map((s) => s.textContent),
      };
    });
    check(court.header[0]?.startsWith("Government: Aristocracy."), `the header: ${court.header}`);
    check(/^The Crown holds 4 cities, 1 over its limit of 3: \+[\d.]+ gold a turn\./.test(court.crown) && court.crownCaution, `the Crown's limit, in the caution colour: ${court.crown}`);
    check(court.tributeDefault === "20", `the realm's default tribute: ${court.tributeDefault}`);
    const kinds = court.attention.map((a) => a.kind);
    check(kinds.indexOf("furious") === 0 && kinds.indexOf("levy") > kinds.lastIndexOf("furious") && kinds.indexOf("crown_over") > kinds.indexOf("levy") && kinds.indexOf("grantable") > kinds.indexOf("crown_over"), `the needs-attention lines' order: ${kinds}`);
    check(court.attention.some((a) => /Furious\. Will break away with .* in 8 turns: war\./.test(a.text)), `the Furious House's line: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.attention.some((a) => a.text.startsWith(`${houseV.label} is Annoyed. It takes no levy.`)), `the House that takes no levy: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.attention.some((a) => a.kind === "crown_over" && a.text.includes(`${taken.name} is over the Crown's limit of 3`)), `the city over the Crown's limit: ${court.attention.map((a) => a.text).join(" | ")}`);
    check(court.houses === 4 && court.crownCities === 4 && court.over.length === 1 && court.persons >= 2, `the cards: ${JSON.stringify(court)}`);
    check(court.marks.some((t) => t.includes("Furious") && t.includes("✖") && t.includes("8 turns")), `the Furious mark with its turns: ${court.marks}`);
    check(["House", "Crown", "Tribute", "Levy", "Claimant"].every((k) => court.chips.includes(k)) && !court.chips.includes("Furious"), `the keyword chips: ${court.chips}`);
    steps.push(`the Court: ${court.header.join(" ")} ${court.attention.length} lines need attention (${kinds.join(", ")}); the Crown (${court.crownCities} cities), ${court.houses} Houses, ${court.persons} persons without a House`);
    await crop(page, ".modal[data-tag=court] .court-header, .modal[data-tag=court] .court-attention", "court-attention.png");
    await crop(page, ".modal[data-tag=court]", "court.png", { pad: 2 });
    // The rest of the Court, a screen at a time: the realm's cards, then the persons.
    if (SHOTS) {
      const scrollTo = (sel) => page.evaluate((sel) => document.querySelector(sel)?.scrollIntoView({ block: "start" }), sel);
      await scrollTo(".modal[data-tag=court] .pcards");
      await crop(page, ".modal[data-tag=court]", "court-realm.png", { pad: 2 });
      await scrollTo(".modal[data-tag=court] .house-card:nth-child(4)");
      await crop(page, ".modal[data-tag=court]", "court-realm-2.png", { pad: 2 });
      await scrollTo(".modal[data-tag=court] .person-card");
      await crop(page, ".modal[data-tag=court]", "court-persons.png", { pad: 2 });
      await page.evaluate(() => (document.querySelector(".modal[data-tag=court] .modal-body").scrollTop = 0));
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
    const rates = await page.evaluate((sel) =>
      [...document.querySelectorAll(`${sel} button.trate`)].map((b) => ({ pct: Number(b.dataset.pct), current: b.getAttribute("aria-pressed") === "true", part: b.querySelector(".trate-part").textContent, state: b.querySelector(".pstate").dataset.state, est: b.querySelector(".trate-est").textContent })),
    card);
    const now = rates.find((r) => r.current);
    check(now?.pct === 0 && rates.filter((r) => r.current).length === 1, `the current rate (House A pays 0%): ${JSON.stringify(rates)}`);
    check(rates.every((r) => /^Pays \d+ gold, \d+ research\.$/.test(r.est) && /^Tribute: [+-]?\d+ · Loyalty: [+-]?\d+$/.test(r.part)), `the rows' lines: ${JSON.stringify(rates)}`);
    const before = await cardState(page, houseA.id);
    await page.evaluate((sel) => document.querySelector(`${sel} details.ptribute`).scrollIntoView({ block: "center" }), card);
    await crop(page, `${card} details.ptribute`, "court-house-tribute.png", { pad: 6 });
    const pick = rates.find((r) => r.pct === 30);
    const [, partWant, totalWant] = /^Tribute: ([+-]?\d+) · Loyalty: ([+-]?\d+)$/.exec(pick.part);
    await tap(page, `${card} button.trate[data-pct="30"]`);
    await until(async () => (await cardState(page, houseA.id))?.line.startsWith("Tribute 30%"), "the tribute set to 30%", 10000 * SLOW);
    const after = await cardState(page, houseA.id);
    check(after.parts.includes(`Tribute 30%: ${partWant}`) && !after.parts.includes("Tribute 0%: +1"), `the tribute part changed as the row said (${partWant}): ${JSON.stringify(after)}`);
    check(after.total === `Loyalty: ${totalWant}.` && after.state === pick.state && after.open, `the total and state the row said (${totalWant}, ${pick.state}), the list kept open: ${JSON.stringify(after)}`);
    steps.push(`${houseA.label}'s tribute 0% -> 30%: parts ${before.parts.find((p) => p.startsWith("Tribute"))} -> ${after.parts.find((p) => p.startsWith("Tribute"))}, ${before.total} -> ${after.total}`);
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
    check(/^Moves to .*'s realm: its production and commerce are the House's\.$/.test(l[0]) && /^Your maintenance: /.test(l[1]) && l.includes("The Crown after the grant: 3 cities, within its limit of 3.") && l.some((x) => /^Tribute from it at \d+%\.$/.test(x)), `the first card's lines: ${JSON.stringify(l)}`);
    check(grant.first.effects.some((e) => /granted to .*: -\d \(30 turns\)\./.test(e)), `the first card's snubs: ${JSON.stringify(grant.first.effects)}`);
    check(grant.kinds.includes("great_person") && grant.kinds.filter((x) => x === "person").length >= 1, `the founders (a veteran, a great person; no hero): ${grant.kinds}`);
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
    check(freeLine === `Free ${houseA.label}? It is no longer our vassal, at peace with us. This cannot be undone.`, `the Free confirmation: ${freeLine}`);
    await page.evaluate((sel) => document.querySelector(`${sel} .pfoot`).scrollIntoView({ block: "center" }), card);
    await crop(page, `${card} .pfoot`, "court-free.png", { pad: 4 });
    await tap(page, `${card} .pconfirm button.small:not(.danger)`);
    await until(() => page.evaluate((sel) => !!document.querySelector(`${sel} button.prelease`), card), "back from the Free confirmation", 5000 * SLOW);
    steps.push(`${houseA.label}'s Free confirmation ("${freeLine}"), then Back`);
    await page.keyboard.press("Escape");

    // The Armies screen: Requisition from lists the Houses' cities with their levy status.
    await until(async () => (await topTag(page)) === null, "the Court closed", 5000 * SLOW);
    await page.evaluate(() => void window.ffh.openScreen("armies"));
    const armySel = `#modals .army-card[data-army="${st.armies.loyal.id}"]`;
    await until(() => page.evaluate((sel) => !!document.querySelector(`${sel} details.requisition-pick`), armySel), "the Armies screen", 20000 * SLOW);
    await page.evaluate((sel) => {
      const d = document.querySelector(`${sel} details.requisition-pick`);
      d.open = true;
      d.scrollIntoView({ block: "start" });
    }, armySel);
    const levy = await page.evaluate((sel) => [...document.querySelectorAll(`${sel} .requisition-city`)].map((r) => ({ city: Number(r.dataset.city), house: r.querySelector(".requisition-house")?.dataset.house ?? null, levy: r.querySelector(".requisition-house")?.dataset.levy ?? null, text: r.querySelector(".levy-text")?.textContent ?? null })), armySel);
    const takes = levy.find((x) => x.levy === "yes");
    const refuses = levy.find((x) => x.levy === "no" && x.house === String(houseV.id));
    check(takes?.text === "Takes levies. Each levy: -1 (20 turns)." && refuses?.text === `${houseV.label} is Annoyed. It takes no levy.`, `the levy lines: ${JSON.stringify(levy)}`);
    check(levy.findIndex((x) => x.house) > levy.findIndex((x) => !x.house), `the Crown's cities first: ${JSON.stringify(levy)}`);
    steps.push(`Requisition from: ${levy.filter((x) => !x.house).length} Crown cities, ${levy.filter((x) => x.house).length} Houses' cities ("${takes.text}" / "${refuses.text}")`);
    await crop(page, `${armySel} .requisition-cities`, "armies-levy.png", { pad: 4 });
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
