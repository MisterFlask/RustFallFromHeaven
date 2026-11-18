#!/usr/bin/env node
// The browser smoke test of Persons of Note (docs/PERSONS.md, "UI", parts A, B and J; docs/HUMAN.md,
// "Testing the browser client"), in the style of tools/web-smoke.mjs: `ffh-play stage persons`
// served with `ffh-play serve --load`, the page in headless Chromium on the HD-2D map
// (with `--shots`; else the map the page chooses). It checks the Court button's count, opens the Court with Shift+F9
// (the header, the needs-attention lines, the Crown and the four Houses), opens a House's card
// (its loyalty parts and record), opens the grant list of the contested Crown city (the claimants
// first, each with its preview lines), grants in two steps (the confirmation line names the city
// and the House and says it cannot be undone), sees the toast and the Court refreshed (the city is
// the House's), names an heir, and on a phone (390 x 844) opens the Court and the grant list without
// a sideways scroll. Any console error, page error or failed request fails it.
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
    const houseA = st.houses.A;
    const houseD = st.houses.D;
    const taken = st.taken;

    // The Court button's count: an heir to name, a Disloyal House, a Disloyal commander.
    const badge = await until(() => page.evaluate(() => document.querySelector("#btn-court .court-badge")?.textContent ?? null), "the Court button's count", 20000 * SLOW);
    check(Number(badge) >= 3, `the Court's count is at least 3 (got ${badge})`);
    steps.push(`the Court button's count: ${badge}`);
    await crop(page, "#btn-court", "court-button.png", { pad: 10 });

    // Shift+F9 opens the Court.
    await page.keyboard.press("Shift+F9");
    await until(async () => (await topTag(page)) === "court" && page.evaluate(() => !!document.querySelector(".modal[data-tag=court] .pcards")), "the Court (Shift+F9)", 20000 * SLOW);
    const court = await page.evaluate(() => {
      const m = document.querySelector(".modal[data-tag=court]");
      return {
        header: [...m.querySelectorAll(".court-header > div")].map((d) => d.textContent),
        attention: [...m.querySelectorAll(".pattn")].map((d) => d.textContent),
        houses: m.querySelectorAll(".house-card").length,
        crown: m.querySelectorAll(".crown-card .crown-city").length,
        persons: m.querySelectorAll(".person-card").length,
        chips: [...m.querySelectorAll(".court-header .kw-chip")].map((c) => c.textContent),
        marks: [...m.querySelectorAll(".house-card .phead .pstate")].map((s) => s.textContent),
      };
    });
    check(court.header[0]?.startsWith("Government: Aristocracy."), `the header: ${court.header}`);
    check(court.attention.some((t) => /has no head/.test(t)) && court.attention.some((t) => /Disloyal\. Will secede/.test(t)) && court.attention.some((t) => /can be granted/.test(t)), `the needs-attention lines: ${court.attention.join(" | ")}`);
    check(court.houses === 4 && court.crown === 4 && court.persons >= 2, `the cards: ${JSON.stringify(court)}`);
    check(court.marks.some((t) => t.includes("Disloyal") && t.includes("✖") && t.includes("8 turns")), `the Disloyal mark with its turns: ${court.marks}`);
    check(court.chips.includes("Claimant") && court.chips.includes("Restless"), `the keyword chips: ${court.chips}`);
    steps.push(`the Court: ${court.header.join(" ")} ${court.attention.length} lines need attention; the Crown (${court.crown} cities), ${court.houses} Houses, ${court.persons} persons without a House`);
    await crop(page, ".modal[data-tag=court] .court-header, .modal[data-tag=court] .court-attention", "court-attention.png");
    await crop(page, ".modal[data-tag=court]", "court.png", { pad: 2 });

    // A House's card opened: its loyalty parts, total and record.
    const card = `.modal[data-tag=court] .house-card[data-house="${houseA.id}"]`;
    await page.evaluate((sel) => {
      const c = document.querySelector(sel);
      c.querySelector("details.pmore").open = true;
      c.scrollIntoView({ block: "start" });
    }, card);
    await sleep(300);
    const opened = await page.evaluate((sel) => {
      const c = document.querySelector(sel);
      return { parts: [...c.querySelectorAll("ul.pparts li")].map((l) => l.textContent), total: c.querySelector(".ptotal")?.textContent, record: c.querySelectorAll("ul.precord li").length, demand: c.querySelector(".pdemand")?.textContent ?? "" };
    }, card);
    check(opened.parts.length > 0 && /^Loyalty: [+-]?\d+\.$/.test(opened.total ?? "") && opened.record > 0, `House A's opened card: ${JSON.stringify(opened)}`);
    steps.push(`${houseA.label}'s card opened: ${opened.parts.length} loyalty parts, ${opened.total} ${opened.record} record lines; its demand: ${opened.demand}`);
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
        first: { title: cards[0].querySelector(".pname").textContent, lines: [...cards[0].querySelectorAll("ul.plines li")].map((l) => l.textContent), effects: [...cards[0].querySelectorAll(".peffect")].map((e) => e.textContent) },
      };
    });
    check(grant.title === `Grant ${taken.name}` && /A grant is for good\./.test(grant.head) && /Held by the Crown: .*city-count maintenance/.test(grant.head), `the grant list's head: ${grant.title} / ${grant.head}`);
    const k = grant.claimants.indexOf(false);
    check(grant.claimants[0] && (k < 0 || grant.claimants.slice(k).every((x) => !x)), `claimants first: ${grant.claimants}`);
    check(grant.first.lines.some((l) => l.startsWith("Maintenance:")) && grant.first.lines.some((l) => l.startsWith(`Trait in ${taken.name}:`)) && grant.first.effects.some((e) => /granted to House .*: -\d \(30 turns\)\./.test(e)), `the first card's preview: ${JSON.stringify(grant.first)}`);
    steps.push(`the grant list of ${taken.name}: ${grant.claimants.filter(Boolean).length} claimants first; ${grant.first.title}: ${grant.first.lines.join(" ")} ${grant.first.effects.length} subjects in its preview`);
    await crop(page, ".modal[data-tag=grant]", "grant-list.png", { pad: 2 });
    await crop(page, ".modal[data-tag=grant] .grant-card:first-child", "grant-claimant.png", { pad: 4 });

    // Two steps: the button, then the confirmation line, then Grant.
    const first = ".modal[data-tag=grant] .grant-card:first-child";
    const target = await page.evaluate((sel) => document.querySelector(sel).querySelector(".pname").textContent, first);
    await tap(page, `${first} button.pgrant-to`);
    const confirm = await until(() => page.evaluate((sel) => document.querySelector(`${sel} .pconfirm p`)?.textContent ?? null, first), "the confirmation line", 5000 * SLOW);
    check(confirm === `Grant ${taken.name} to ${target}? This cannot be undone.`, `the confirmation: ${confirm}`);
    await crop(page, `${first}`, "grant-confirm.png", { pad: 4 });
    await tap(page, `${first} button.pgrant-yes`);
    await until(async () => (await topTag(page)) === "court", "the grant list closed", 20000 * SLOW);
    const toast = await until(() => page.evaluate((t) => [...document.querySelectorAll("#toasts .toast")].map((x) => x.textContent).find((x) => x.includes(t)) ?? null, `${taken.name} granted to ${target}.`), "the grant's toast", 10000 * SLOW);
    await until(() => page.evaluate((id) => !document.querySelector(`.modal[data-tag=court] .crown-card [data-city="${id}"]`) && !!document.querySelector(`.modal[data-tag=court] .house-card .city-chip[data-city="${id}"]`), taken.id), "the Court refreshed: the city is the House's", 10000 * SLOW);
    steps.push(`granted in two steps ("${confirm}"); the toast "${toast}"; the Court refreshed`);

    // Naming an heir from the needs-attention line.
    await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
    await tap(page, `.modal[data-tag=court] .pattn[data-kind=heir] button`);
    await until(async () => (await topTag(page)) === "court-heir", "the heir's choice", 10000 * SLOW);
    await crop(page, ".modal[data-tag=court-heir]", "court-heir.png", { pad: 2 });
    await tap(page, ".modal[data-tag=court-heir] .heir-card:first-child button.pact");
    await until(() => page.evaluate((id) => !!document.querySelector(`.modal[data-tag=court] .house-card[data-house="${id}"] .phouse-head .pname`), houseD.id), "the heir named", 10000 * SLOW);
    steps.push(`an heir named for ${houseD.label} from the needs-attention line`);
    await page.keyboard.press("Escape");
    await page.close();

    // On a phone: the Court and a grant list fit the width.
    const phone = await open(browser, server.url, PHONE, { isMobile: true, hasTouch: true });
    await phone.evaluate(() => void window.ffh.openScreen("court"));
    await until(() => phone.evaluate(() => !!document.querySelector(".modal[data-tag=court] .pcards")), "the Court on a phone", 20000 * SLOW);
    let f = await fits(phone);
    check(f.page && f.body && !f.over.length, `the Court fits a phone: ${JSON.stringify(f)}`);
    await crop(phone, ".modal[data-tag=court]", "phone-court.png", { pad: 0 });
    const crownCity = st.crown.find((c) => c.id !== taken.id);
    await phone.evaluate((id) => void window.ffh.persons.openGrant(id), crownCity.id);
    await until(() => phone.evaluate(() => !!document.querySelector(".modal[data-tag=grant] .grant-card")), "a grant list on a phone", 20000 * SLOW);
    f = await fits(phone);
    check(f.page && f.body && !f.over.length, `the grant list fits a phone: ${JSON.stringify(f)}`);
    await crop(phone, ".modal[data-tag=grant]", "phone-grant.png", { pad: 0 });
    steps.push(`on a phone (${PHONE.width} x ${PHONE.height}): the Court and the grant list of ${crownCity.name} without a sideways scroll`);
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
