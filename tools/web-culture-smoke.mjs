#!/usr/bin/env node
// The browser smoke test of the Culture screen and the city screen's Culture section
// (docs/PERSONS.md, "The fourth build", rule 10, and "as built, part 2A"; docs/CULTURE.md, "UI";
// docs/HUMAN.md): stages `ffh-play stage culture` (the `persons` stage, its hand_set printed, with
// a religion in every city of the human: Spread Religion then has a refused target), serves it,
// loads the page on the HD-2D map (`?force3d=1`: SwiftShader here, slow) at a phone's width and
// reads, against the `culture` query's numbers:
// - the head (the channel, the inflow and the half of it);
// - your Court channel's preview: the requirement's parts and its formula, the turns to the split,
//   the split's cities (`It joins whole.` for a House of one city);
// - the chooser: the three actions, each with its targets or its line of why none; Spread
//   Religion's refused target disabled with the rules' refusal;
// - aimed at you: the neighbour's courtship of your House past two thirds, marked as needing
//   attention, and the Guard button's preview;
// - the Guard button: it sets the guard, the head and the preview change, the courter's prediction
//   moves to where the guard's preview said; Stop; a Court set again from the chooser;
// - the city screen's Culture line and its link to the Culture screen; the keywords' boxes;
// - the calculation tooltips (docs/PLAY_DESIGN.md 4.9): a tap on the head's inflow, your Court's
//   gain, a courtship aimed at you and the Guard button's preview shows the lines the view sends.
// Any console error or page error fails it.
//
// With `--shots DIR` it also saves the screenshots the owner reviews (CLAUDE.md, "Show UI changes
// in a screenshot": crops at deviceScaleFactor 2 on a 390 px wide page, the HD-2D map): the head
// and your Court channel with its requirement, the chooser with a refused target, aimed at you
// with the Guard preview, a Guard channel, the city screen's Culture section, the keyword boxes,
// and the calculation tooltips (`calc-*.png`).
//
//   node tools/web-culture-smoke.mjs [--bin PATH] [--assets DIR] [--shots DIR] [--gpu]
//
// The browser: Playwright's Chromium (as tools/web-smoke.mjs); without it the test is SKIPPED
// (exit 0, or 1 when $CI is set). Exit 1 on a failure.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch, calcLines, shownCalc, hideTip } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value", shots: "value", gpu: "flag" });
const SHOTS = args.shots ? path.resolve(args.shots) : null;
const ASSETS = args.assets ? ["--assets", path.resolve(args.assets)] : [];
const PHONE = { width: 390, height: 844 };
const M = "#modals .modal[data-tag=culture]";

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

// ---- the numbers as the screen writes them (docs/CULTURE.md, "UI") --------------------------

/** Hundredths with one decimal. */
function t(n) {
  const v = Math.round((n ?? 0) / 10);
  const a = Math.abs(v);
  return `${v < 0 ? "−" : ""}${Math.floor(a / 10)}.${a % 10}`;
}
const whole = (n) => String(Math.floor((n ?? 0) / 100));
const split = (turns) => (turns === null || turns === undefined ? null : turns <= 1 ? "a split next turn" : `a split in ~${turns} turns`);
const prospect = (turns, settles, gain) => split(turns) ?? (gain > 0 ? `settling near ${whole(settles)}` : "fading");
const turnsText = (turns) => (turns === null || turns === undefined ? "Never at these inflows." : turns <= 1 ? "Next turn." : `In ~${turns} turns.`);
const stepText = (n) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : "0");
const intText = (n) => (n < 0 ? `−${-n}` : String(n));
const text = (s) => (s ?? "").replace(/\s+/g, " ").trim();

/** The guard button's preview from the view's `guard` (`house_guard_with`). */
function guardLine(gv) {
  const loss = gv.loss ? `−${t(gv.loss)}` : "0.0";
  return [`Guard: ${loss} a turn to each courtship.`, ...gv.courtships.map((c) => `The ${c.courter_name}: ${prospect(c.turns_without, c.settles_without, c.gain)} without it, ${prospect(c.turns_with, c.settles_with, c.gain)} with it.`)].join(" ");
}

// ---- the page ----------------------------------------------------------------------------------

/** A query to the served game: its reply's `r`. */
async function query(url, q) {
  const r = await fetch(`${url}/api`, { method: "POST", headers: { "X-FFH": "1", "Content-Type": "application/json" }, body: JSON.stringify({ k: "q", ...q }) });
  if (!r.ok) throw new Error(`${q.q}: HTTP ${r.status} ${await r.text()}`);
  const j = await r.json();
  if (!j.ok) throw new Error(`${q.q}: ${JSON.stringify(j)}`);
  return j.r;
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

/** Closes every modal and drops the toasts, the tooltips and the selection (its unit panel would
 *  show through the dialogs on the HD-2D map). */
async function clear(page) {
  await page.evaluate(() => {
    window.ffh.units?.clearSelection?.();
    for (const b of document.querySelectorAll("#modals .modal")) b.querySelector(".modal-buttons button:last-child")?.click();
    for (const t of document.querySelectorAll("#toasts .toast")) t.remove();
    document.querySelectorAll(".tooltip, .tip-side").forEach((e) => (e.hidden = true));
  });
  await page.keyboard.press("Escape");
  await sleep(300);
}

/** A crop around the union of what `sels` match (`pad` CSS px round it), saved as `name`. */
async function crop(page, sels, name, { pad = 6, maxH = 2400 } = {}) {
  if (!SHOTS) return;
  // A command may select the next unit: its panel would show through the dialog.
  await page.evaluate(() => window.ffh.units?.clearSelection?.());
  await sleep(300);
  const r = await page.evaluate((sels) => {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const s of sels) {
      for (const e of document.querySelectorAll(s)) {
        if (e.hidden) continue;
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

/** Scrolls the element `sel` matches to the top of its scrolling box. */
async function scrollTo(page, sel, block = "start") {
  await page.evaluate(([sel, block]) => document.querySelector(sel)?.scrollIntoView({ block }), [sel, block]);
  await sleep(400);
}

/** Opens the Culture screen (F8) and waits for its head. */
async function openCulture(page) {
  await page.evaluate(() => void window.ffh.openScreen("culture"));
  await until(() => page.evaluate((M) => !!document.querySelector(`${M} .culture-head`), M), "the Culture screen", 60000, 200);
  await sleep(300);
}

/** The head's two lines. */
function headLines(page) {
  return page.evaluate((M) => [...document.querySelectorAll(`${M} .culture-head p`)].map((p) => p.textContent.replace(/\s+/g, " ").trim()), M);
}

/** Waits until the head's first line is `want`. */
async function headIs(page, want) {
  await until(async () => (await headLines(page))[0] === want, `the head: ${want}`, 60000, 200);
}

/** The `[label, value]` rows of the table `sel` matches. */
function rows(page, sel) {
  return page.evaluate((sel) => [...document.querySelectorAll(`${sel} tr`)].map((tr) => [...tr.children].map((c) => c.textContent.replace(/\s+/g, " ").trim())), sel);
}

/** Clicks the button `sel` matches. */
async function click(page, sel) {
  const ok = await page.evaluate((sel) => {
    const b = document.querySelector(sel);
    if (!b) return false;
    b.click();
    return true;
  }, sel);
  if (!ok) throw new Error(`nothing to click: ${sel}`);
  await sleep(300);
}

// ---- the checks --------------------------------------------------------------------------------

/** Taps the element `sel` matches and returns the calculation lines its tooltip shows (null: no
 *  tooltip); with `shot`, saves the element and its tooltip as one crop; then hides the tooltip. */
async function tipOf(page, sel, shot = null) {
  await click(page, sel);
  await sleep(250);
  const lines = await shownCalc(page);
  if (shot && lines) await crop(page, [sel, ".richtip"], shot, { pad: 8 });
  await hideTip(page);
  return lines;
}

/** The calculation tooltips: the head's inflow by city, your Court's gain (its inflow, reach and
 *  product), the first courtship aimed at you and its Guard preview, each as the view sends it. */
async function calcTips(page, r) {
  const inflow = await tipOf(page, `${M} .culture-head p.culture-inflow`, "calc-inflow.png");
  check(JSON.stringify(inflow) === JSON.stringify(calcLines(r.inflow_calc)) && inflow.at(-1).startsWith("Inflow: "), `the inflow's tooltip: ${JSON.stringify(inflow)}`);
  const cv = r.preview?.court;
  if (cv) {
    const gainCell = await page.evaluate((M) => {
      const tr = [...document.querySelectorAll(`${M} .culture-ours table.culture-facts tr`)].find((t) => t.querySelector("th")?.textContent === "Gain");
      tr?.querySelector("td")?.setAttribute("data-test", "gain");
      return !!tr;
    }, M);
    check(gainCell, "your Court's Gain row");
    await scrollTo(page, `${M} .culture-ours`);
    const gain = await tipOf(page, `${M} td[data-test=gain]`, "calc-gain.png");
    const want = calcLines(cv.calc, ["inflow", "reach", "gain"]);
    check(JSON.stringify(gain) === JSON.stringify(want) && /^Gain: .+ × \d+% = .+ a turn\.$/.test(gain.at(-1)), `the gain's tooltip: ${JSON.stringify(gain)}`);
  }
  const x = r.aimed[0];
  if (x) {
    await scrollTo(page, `${M} .culture-aimed`);
    const line = await tipOf(page, `${M} .culture-aimed-house[data-house="${x.house}"] li.courtship .courtship-line`, "calc-courtship.png");
    check(JSON.stringify(line) === JSON.stringify(calcLines(x.courtships[0].calc)) && line.some((l) => l.startsWith("Next liege turn: ")), `a courtship's tooltip: ${JSON.stringify(line)}`);
    if (!x.guarded && !x.guard_refusal) {
      const guard = await tipOf(page, `${M} .culture-aimed-house[data-house="${x.house}"] p.culture-guard-preview`, "calc-guard.png");
      check(JSON.stringify(guard) === JSON.stringify(calcLines(x.guard.calc)) && /^Guard: .+ × \d+% = .+ a turn from each courtship\.$/.test(guard.at(-1)), `the Guard preview's tooltip: ${JSON.stringify(guard)}`);
    }
  }
}

/** The head and your channel's preview against the view's numbers; a Court preview's
 *  requirement, turns and cities. */
async function headAndChannel(page, r) {
  const lines = await headLines(page);
  const half = r.percent === 50 ? "half of" : `${r.percent}% of`;
  if (r.channel) {
    check(lines[0] === `Channel: ${r.channel.name}, since turn ${r.channel.since_turn}.`, `the head's channel: ${lines[0]}`);
    check(lines[1] === `Inflow: ${t(r.inflow)} culture a turn (${half} ${t(r.base)}).`, `the head's inflow: ${lines[1]}`);
  } else {
    check(lines[0] === "Channel: none. Your cities keep all their culture.", `the head without a channel: ${lines[0]}`);
    check(lines[1] === `A channel would take ${t(r.inflow)} culture a turn (${half} ${t(r.base)}).`, `the head's inflow: ${lines[1]}`);
  }
  const cv = r.preview?.court;
  if (!cv) return;
  const q = cv.requirement;
  // Each row read as `label: value` (the formula's row is one cell across the table).
  const req = (await rows(page, `${M} .culture-ours table.culture-req`)).map((cells) => cells.join(": "));
  const want = [
    `Loyalty of ${cv.label} to the ${cv.liege_name}: ${intText(q.loyalty)}`,
    `Attitude of its leader toward you: ${q.attitude_name} (${stepText(q.attitude)})`,
    `World culture unit: ${q.unit}`,
    `Requirement: ${q.formula}`,
  ];
  check(JSON.stringify(req) === JSON.stringify(want), `the requirement's parts: ${JSON.stringify(req)}`);
  check(q.formula.endsWith(`= ${q.total}`), `the formula's total: ${q.formula}`);
  const facts = Object.fromEntries(await rows(page, `${M} .culture-ours table.culture-facts`));
  check(facts.Split === turnsText(cv.turns), `the turns to the split: ${facts.Split}`);
  check(facts["Settles near"] === whole(cv.settles), `where the favour settles: ${facts["Settles near"]}`);
  check(facts.Reach === `${cv.reach.distance} plot${cv.reach.distance === 1 ? "" : "s"}: ${cv.reach.effect}%`, `the reach: ${facts.Reach}`);
  const favour = await page.evaluate((M) => document.querySelector(`${M} .culture-ours .favour-label`)?.textContent, M);
  check(favour === `Favour ${whole(cv.favour)} of ${q.total}`, `the favour bar's label: ${favour}`);
  const cities = await page.evaluate((M) => [...document.querySelectorAll(`${M} .culture-ours .culture-split, ${M} .culture-ours .culture-held`)].map((p) => p.textContent.replace(/\s+/g, " ").trim()), M);
  if (cv.split?.whole) check(cities[0] === "It joins whole.", `a House of one city: ${cities.join(" | ")}`);
  else if (cv.split) check(cities[0] === `Would go: ${cv.split.going.length ? cv.split.going.map((c) => c.name).join(", ") : "none"}.`, `the split's cities: ${cities.join(" | ")}`);
}

/** The chooser: the three actions, each with its targets (a refused one disabled with the rules'
 *  refusal, an allowed one with its line) or its one line of why none. Leaves Spread Religion
 *  picked. */
async function chooser(page, r) {
  const shown = await page.evaluate((M) => [...document.querySelectorAll(`${M} .culture-actions button[data-action]`)].map((b) => b.textContent.trim()), M);
  check(shown.length === r.actions.length && r.actions.every((a) => shown.includes(a.name)), `the actions: ${shown.join(", ")}`);
  const seen = [];
  for (const a of r.actions) {
    await click(page, `${M} .culture-actions button[data-action="${a.action}"]`);
    await until(() => page.evaluate(([M, a]) => !!document.querySelector(`${M} .culture-actions button[data-action="${a}"].on`), [M, a.action]), `${a.name} picked`, 10000, 100);
    const got = await page.evaluate((M) => ({
      none: document.querySelector(`${M} p.culture-none`)?.textContent ?? null,
      targets: [...document.querySelectorAll(`${M} button.culture-target`)].map((b) => ({ target: Number(b.dataset.target), disabled: b.disabled, line: b.querySelector(".target-line")?.textContent ?? "" })),
    }), M);
    // Spread Religion lists the targets of the first religion (the one picked).
    const options = a.options.filter((o) => (o.religion ?? null) === (a.religions[0] ?? null));
    if (a.none || !options.length) {
      check(got.none === a.none && !got.targets.length, `${a.name}: its line of why none: ${got.none}`);
    } else {
      const ok = options.every((o) => {
        const b = got.targets.find((x) => x.target === o.target.player);
        if (!b) return false;
        return o.refusal ? b.disabled && b.line === o.refusal : !b.disabled && (o.current ? b.line === `Your channel. ${o.line}` : b.line === o.line);
      });
      check(ok && got.targets.length === options.length, `${a.name}: ${options.length} targets, each with its line or its refusal (${got.targets.filter((x) => x.disabled).length} refused)`);
    }
    seen.push(`${a.name} ${got.none ? "none" : got.targets.length}`);
  }
  const spread = r.actions.find((a) => a.effect === "spread_religion");
  const refused = spread.options.find((o) => o.refusal && (o.religion ?? null) === (spread.religions[0] ?? null));
  check(!!refused, `a refused target in the stage: ${refused?.label}: ${refused?.refusal}`);
  // Leave Spread Religion picked for the screenshot, with an allowed target's preview.
  await click(page, `${M} .culture-actions button[data-effect="spread_religion"]`);
  const allowed = spread.options.find((o) => !o.refusal && (o.religion ?? null) === (spread.religions[0] ?? null));
  if (allowed) {
    await click(page, `${M} button.culture-target[data-target="${allowed.target.player}"]`);
    await until(() => page.evaluate((M) => !!document.querySelector(`${M} .culture-pick .culture-spread`), M), "the picked target's preview", 10000, 100);
    const set = await page.evaluate((M) => document.querySelector(`${M} .culture-pick button.culture-set`)?.textContent, M);
    check(set === (r.channel ? `Set (replaces ${r.channel.name})` : "Set"), `the Set button: ${set}`);
    const chance = await page.evaluate((M) => document.querySelector(`${M} .culture-pick .culture-chance`)?.textContent, M);
    check(chance === allowed.preview.line, `the spread's chance: ${chance}`);
  }
  return seen.join(", ");
}

/** Aimed at you: the courtship of House `pleased` by its line, marked as needing attention, with
 *  the Guard button and its preview. Returns that House's entry of the view. */
async function aimedAtYou(page, r, st) {
  const pleased = st.houses.pleased;
  const x = r.aimed.find((a) => a.house === pleased.id);
  check(!!x, `${pleased.label} is among the courted Houses`);
  if (!x) return null;
  const sel = `${M} .culture-aimed-house[data-house="${x.house}"]`;
  const got = await page.evaluate((sel) => ({
    lines: [...document.querySelectorAll(`${sel} li.courtship`)].map((l) => ({ text: l.textContent.replace(/\s+/g, " ").trim(), attention: l.classList.contains("attention"), courter: Number(l.dataset.courter) })),
    button: document.querySelector(`${sel} button.culture-guard`)?.textContent ?? null,
    preview: document.querySelector(`${sel} .culture-guard-preview`)?.textContent ?? null,
  }), sel);
  const cv = x.courtships.find((c) => c.courter === st.neighbour.player);
  const line = got.lines.find((l) => l.courter === st.neighbour.player);
  check(!!cv && cv.attention && line?.attention && line.text === `! ${cv.line}`, `the neighbour's courtship past two thirds, marked: ${line?.text}`);
  check(got.button === (r.channel ? `Guard ${x.label} (replaces ${r.channel.name})` : `Guard ${x.label}`), `the Guard button: ${got.button}`);
  check(got.preview === guardLine(x.guard), `the Guard button's preview: ${got.preview}`);
  return x;
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("web-culture-smoke", "no playwright");
  if (SHOTS) fs.mkdirSync(SHOTS, { recursive: true });
  const saves = tempDir("culture");
  const st = await stage(bin, "culture", saves);
  console.log(`stage culture: turn ${st.turn}; set by hand: ${st.hand_set.join("; ")}`);
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, ...ASSETS] });
  try {
    await research(server.url);
    const page = await open(browser, server.url, PHONE);
    await clear(page);
    let r = await query(server.url, { q: "culture" });
    check(r.channel?.effect === "court" || st.hand_set.some((l) => l.startsWith("the human's Court of the neighbour's House refused")), `your channel: ${r.channel?.name ?? "none (its refusal is listed)"}`);

    // The head and your Court channel with its requirement.
    await openCulture(page);
    await headAndChannel(page, r);
    await scrollTo(page, `${M} .culture-head`);
    await crop(page, [`${M} .culture-head`, `${M} .culture-ours`], "culture-head-court.png");

    // The calculation tooltips.
    await calcTips(page, r);
    await scrollTo(page, `${M} .culture-head`);

    // The chooser and its refusals.
    const seen = await chooser(page, r);
    await scrollTo(page, `${M} .culture-choose`);
    await crop(page, [`${M} .culture-actions`, `${M} .culture-religions`, `${M} button.culture-target.refused`, `${M} button.culture-target.on`, `${M} .culture-pick`], "culture-chooser-refused.png");

    // Aimed at you, with the Guard preview.
    const x = await aimedAtYou(page, r, st);
    await scrollTo(page, `${M} .culture-aimed`);
    await crop(page, [`${M} .culture-aimed`], "culture-aimed-guard.png");

    // The Guard button: the guard is set, the head and the preview change, the courter's
    // prediction moves to where the guard's preview said.
    if (x) {
      const before = x.courtships.find((c) => c.courter === st.neighbour.player);
      const row = x.guard.courtships.find((c) => c.courter === st.neighbour.player);
      await click(page, `${M} .culture-aimed-house[data-house="${x.house}"] button.culture-guard`);
      await headIs(page, `Channel: Guard ${x.label}, since turn ${st.turn}.`);
      const r2 = await query(server.url, { q: "culture" });
      check(r2.channel?.effect === "guard" && !!r2.preview?.guard, `the Guard channel: ${r2.channel?.name}`);
      const x2 = r2.aimed.find((a) => a.house === x.house);
      const after = x2?.courtships.find((c) => c.courter === st.neighbour.player);
      check(x2?.guarded && after?.guard > 0 && after.settles === row.settles_with && (after.turns ?? null) === (row.turns_with ?? null), `the courter's prediction moved: settles near ${whole(before.settles)} -> ${whole(after?.settles)} (the preview said ${whole(row.settles_with)})`);
      const guarded = await page.evaluate((M) => document.querySelector(`${M} .culture-guarded-line`)?.textContent ?? null, M);
      check(guarded === `Your channel guards it. Guard: ${r2.preview.guard.loss ? `−${t(r2.preview.guard.loss)}` : "0.0"} a turn to each courtship.`, `the House's line under a guard: ${guarded}`);
      const cells = await page.evaluate((M) => [...document.querySelectorAll(`${M} .culture-ours table.culture-guarded tbody tr`)].map((tr) => [...tr.children].map((c) => c.textContent)), M);
      const gr = r2.preview.guard.courtships.find((c) => c.courter === st.neighbour.player);
      check(cells.some((c) => c[0] === gr.courter_name && c[1] === `${whole(gr.favour)} of ${gr.requirement}`), `the Guard preview's row: ${JSON.stringify(cells)}`);
      await scrollTo(page, `${M} .culture-head`);
      await crop(page, [`${M} .culture-head`, `${M} .culture-ours`], "culture-guard-channel.png");

      // Stop.
      await click(page, `${M} .culture-ours .culture-stop`);
      await headIs(page, "Channel: none. Your cities keep all their culture.");
      const r3 = await query(server.url, { q: "culture" });
      check(r3.channel === null && !(await page.evaluate((M) => !!document.querySelector(`${M} .culture-ours`), M)), "Stop: no channel");

      // A Court set from the chooser again (the Set button), when the stage had one.
      const court = r3.actions.find((a) => a.effect === "court");
      const target = court?.options.find((o) => !o.refusal);
      if (target) {
        await click(page, `${M} .culture-actions button[data-effect="court"]`);
        await click(page, `${M} button.culture-target[data-target="${target.target.player}"]`);
        const set = await page.evaluate((M) => document.querySelector(`${M} .culture-pick button.culture-set`)?.textContent, M);
        check(set === "Set", `the Set button without a channel: ${set}`);
        await click(page, `${M} .culture-pick button.culture-set`);
        await headIs(page, `Channel: ${target.preview.order.name}, since turn ${st.turn}.`);
      }
    }
    await clear(page);

    // The city screen's Culture section: its line, its link to the Culture screen.
    const city = await query(server.url, { q: "city", city: st.capital.id });
    const k = city.cultural;
    await page.evaluate((id) => window.ffh.emit("open-city", id), st.capital.id);
    await until(() => page.evaluate(() => !!document.querySelector("#modals .section.culture .culture-line")), "the city's Culture section", 60000, 200);
    const line = await page.evaluate(() => document.querySelector("#modals .section.culture .culture-line").textContent);
    const want = !k.channel ? `Culture: ${t(k.rate)} a turn; no channel.` : k.disorder ? `Culture: ${t(k.rate)} a turn; nothing diverted in disorder.` : `Culture: ${t(k.rate)} a turn; ${t(k.diverted)} of it to ${k.channel.name}.`;
    check(line === want, `the city's Culture line: ${line}`);
    const chip = await page.evaluate(() => [...document.querySelectorAll("#modals .section.culture .kw-chip")].map((c) => c.dataset.keyword));
    check(chip.join() === "KEYWORD_CHANNEL", `the city section's keyword: ${chip.join()}`);
    await scrollTo(page, "#modals .section.culture", "center");
    await crop(page, ["#modals .section.culture"], "city-culture.png");
    await click(page, "#modals .section.culture button.culture-open");
    await until(() => page.evaluate((M) => !!document.querySelector(`${M} .culture-head`), M), "the link opens the Culture screen", 60000, 200);
    check(true, "the city section's link opens the Culture screen");

    // The keywords' boxes: a chip's tap shows its reminder boxes.
    const keys = await page.evaluate((M) => [...document.querySelectorAll(`${M} .culture-keywords .kw-chip`)].map((c) => c.dataset.keyword), M);
    check(keys.join() === "KEYWORD_CHANNEL,KEYWORD_COURT,KEYWORD_GUARD", `the screen's keywords: ${keys.join(", ")}`);
    await scrollTo(page, `${M} .culture-keywords`, "end");
    await click(page, `${M} .culture-keywords .kw-chip[data-keyword="KEYWORD_GUARD"]`);
    await until(() => page.evaluate(() => [...document.querySelectorAll(".tip-side, .tooltip.richtip")].some((e) => !e.hidden && e.querySelector(".reminder"))), "the Guard chip's reminder boxes", 15000, 100);
    const boxes = await page.evaluate(() => [...document.querySelectorAll(".tip-side:not([hidden]) .reminder, .tooltip.richtip:not([hidden]) .reminder")].map((b) => b.dataset.keyword));
    check(["KEYWORD_GUARD", "KEYWORD_CHANNEL", "KEYWORD_COURT"].every((kw) => boxes.includes(kw)), `Guard's boxes: ${boxes.join(", ")}`);
    await sleep(500);
    await crop(page, [".tip-side:not([hidden])", ".tooltip.richtip:not([hidden])"], "culture-keywords.png");
    await clear(page);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    check(overflow <= 0, `no horizontal overflow at 390 px (${overflow})`);
    console.log(`chooser: ${seen}`);
    await page.close();
  } finally {
    await server.stop();
    await browser.close();
  }
  for (const e of errors) console.error(e);
  check(errors.length === 0, `no console or page error (${errors.length})`);
  if (failed) process.exit(1);
  console.log("web-culture-smoke: ok");
}

main().catch((e) => {
  console.error(`web-culture-smoke: ${e.stack ?? e}`);
  process.exit(1);
});
