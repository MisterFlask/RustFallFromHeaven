#!/usr/bin/env node
// Browser smoke of the Cataclysm mode (docs/CATACLYSM.md, "Screens"): a new Cataclysm game started
// from the title screen and driven with Playwright's Chromium on the HD-2D map:
// - the title's New Cataclysm entry, the nine factions' cards and their details, Start;
// - the top bar's chips and the turn report (no "undefined" anywhere);
// - every screen the menu and the F keys open (cities, research, rituals, creeds, the Court, the
//   Houses, diplomacy, the Worldseal): each opens a screen of its own name;
// - the capital's city screen, and a rival's city showing none of its production;
// - the Help: the Cataclysm topics, none of the classic game's;
// - the map tooltip over the capital (this mode's, not the classic game's);
// - research chosen and turns ended with Enter;
// - an emulated phone: the Strain chip first in the top bar, and a squad moved with two taps on a
//   tile (the map sends no right-click from a finger).
// No console error, no page error, no failed request, no "undefined" / "NaN" / "[object" in any
// screen's text.
//
//   node tools/web-cataclysm-smoke.mjs --bin <built>/ffh-play [--assets crates/ffh-play/assets]
//        [--turns 4] [--gpu] [--out DIR]   (screenshots of the screens into DIR)

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value", turns: "value", gpu: "flag", out: "value" });
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const TURNS = Number(args.turns ?? 4);
const OUT = args.out ? path.resolve(args.out) : null;
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

const t00 = Date.now();
const log = (s) => console.log(`  [${((Date.now() - t00) / 1000).toFixed(0)} s] ${s}`);
const problems = [];

function check(ok, what) {
  if (!ok) throw new Error(what);
}

/** Text that shows a value the client did not have. */
const BROKEN = /undefined|NaN|\[object|null\b/;

async function closeModals(page) {
  for (let i = 0; i < 6 && (await page.evaluate(() => !!document.querySelector("#modals .modal"))); i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
}

/** The top modal's `{title, tag, text}`, or null. */
const modal = (page) =>
  page.evaluate(() => {
    const m = [...document.querySelectorAll("#modals .modal")].pop();
    return m ? { title: m.querySelector("h2")?.textContent ?? "", tag: m.dataset.tag ?? "", text: m.innerText } : null;
  });

async function shot(page, name) {
  if (!OUT) return;
  await page.screenshot({ path: path.join(OUT, `${name}.png`), timeout: 120000 });
}

/** Opens a screen with `open()`, checks it, and returns its modal. */
async function screen(page, what, open) {
  await open();
  await until(() => page.evaluate(() => !!document.querySelector("#modals .modal")), `the ${what} screen`, 15000);
  await sleep(300);
  const m = await modal(page);
  check(m.title.trim().length > 0, `${what}: the screen has no title`);
  const bad = BROKEN.exec(m.text);
  check(!bad, `${what}: "${bad?.[0]}" in the screen's text: ${m.text.slice(Math.max(0, (bad?.index ?? 0) - 60), (bad?.index ?? 0) + 40).replace(/\n/g, " / ")}`);
  await shot(page, `screen-${what}`);
  return m;
}

async function main() {
  const pw = await loadPlaywright();
  if (!pw) return skipped("web-cataclysm-smoke", "playwright not found");
  const bin = findBin(args.bin);
  if (!bin) return skipped("web-cataclysm-smoke", "no ffh-play binary");
  if (OUT) fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("cataclysm-smoke");
  const server = await startServer({ bin, saves, extra: ["--no-game", "--assets", ASSETS] });
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  try {
    const page = await browser.newPage({ viewport: VIEW });
    page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
    page.on("console", (m) => {
      if (m.type() === "error") problems.push(`console error: ${m.text()}`);
    });
    page.on("requestfailed", (r) => problems.push(`request failed: ${r.url()}`));
    page.on("response", (r) => {
      if (r.status() >= 400) problems.push(`HTTP ${r.status()}: ${r.url()}`);
    });
    await page.goto(`${server.url}/?renderer=hd2d${args.gpu ? "" : "&force3d=1"}`);
    await until(() => page.evaluate(() => !!window.ffh?.ready), "the page", 120000);

    // The title's entry and the new-game screen.
    await until(() => page.evaluate(() => !!document.getElementById("title-cataclysm")), "the title's New Cataclysm entry", 30000);
    await page.click("#title-cataclysm");
    await until(() => page.evaluate(() => document.querySelectorAll('.modal[data-tag="cataclysm-new"] .cat-fac-card').length > 0), "the New Cataclysm screen", 30000);
    const factions = await page.evaluate(() => [...document.querySelectorAll(".cat-fac-card")].map((b) => b.dataset.faction));
    check(factions.length === 9, `nine factions, got ${factions.length}`);
    for (const f of factions) {
      await page.click(`.cat-fac-card[data-faction="${f}"]`);
      const text = await page.evaluate(() => document.querySelector(".cat-ng-detail").innerText);
      const bad = BROKEN.exec(text);
      check(!bad, `faction ${f}: "${bad?.[0]}" in its detail`);
      check(/Victory/.test(text), `faction ${f}: its detail does not say how it wins`);
    }
    await shot(page, "new-game");
    log(`the New Cataclysm screen: ${factions.length} factions`);
    await page.click(`.cat-fac-card[data-faction="${factions[0]}"]`);
    await page.evaluate(() => [...document.querySelectorAll('.modal[data-tag="cataclysm-new"] .modal-buttons button')].find((b) => /Start/.test(b.innerText)).click());
    await until(() => page.evaluate(() => window.ffh.S.view?.mode === "cataclysm" && window.ffh.S.status?.phase === "human"), "the Cataclysm game", 120000);
    await sleep(2500);

    // The top bar and the turn report.
    const top = await page.evaluate(() => ({ text: document.getElementById("top").innerText, chips: document.querySelectorAll("#empire .cat-top-chip").length }));
    check(top.chips >= 6, `the top bar's chips: ${top.chips}`);
    check(!BROKEN.test(top.text), `the top bar: ${top.text.replace(/\n+/g, " | ")}`);
    await page.click('#bottom-tabs button[data-tab="report"]').catch(() => {});
    const report = await page.evaluate(() => document.getElementById("bottom-body")?.innerText ?? "");
    check(!BROKEN.test(report), `the turn report: ${report.slice(0, 200)}`);
    await shot(page, "hud");
    log(`the top bar: ${top.text.replace(/\n+/g, " | ").slice(0, 160)}`);

    // Every screen, by the client's own names (the F keys' and the menu's).
    const titles = {};
    for (const name of ["cities", "techs", "civics", "religion", "governors", "houses", "diplomacy", "finance"]) {
      titles[name] = (await screen(page, name, () => page.evaluate((n) => window.ffh.openScreen(n), name))).title;
      await closeModals(page);
    }
    check(new Set(Object.values(titles)).size === 8, `the screens' titles: ${JSON.stringify(titles)}`);
    log(`screens: ${Object.values(titles).join(" · ")}`);

    // The menu: each visible advisor button opens a screen, no two the same.
    const buttons = await page.evaluate(() =>
      ["btn-research", "btn-techs", "btn-cities", "btn-civics", "btn-religion", "btn-armies", "btn-governors", "btn-houses", "btn-diplo", "btn-crusades", "btn-seas", "btn-trade"].filter((id) => {
        const b = document.getElementById(id);
        return b && !b.hidden && b.getBoundingClientRect().width > 0;
      }),
    );
    const opened = {};
    for (const id of buttons) {
      opened[id] = (await screen(page, id, () => page.click(`#${id}`))).title;
      await closeModals(page);
    }
    check(new Set(Object.values(opened)).size === buttons.length, `two menu buttons open the same screen: ${JSON.stringify(opened)}`);
    log(`menu: ${buttons.length} advisor buttons, each its own screen`);

    // The capital's city screen; a rival's city shows none of its production.
    const city = await screen(page, "city", () => page.keyboard.press("F10"));
    check(/Production/.test(city.text), "the city screen names its production");
    check(!/guard \+ guard/.test(city.text), "the city screen lists class keys");
    await closeModals(page);
    const rival = await page.evaluate(() => {
      const d = window.ffh.S.view.cataclysm;
      const c = d.cities.find((c) => c.owner !== window.ffh.S.view.me.player);
      if (!c) return null;
      window.ffh.cataclysm.handleDblClick(c.x, c.y);
      return c.name;
    });
    if (rival) {
      await sleep(400);
      const m = await modal(page);
      check(m && !/Production|Work Crew|Raise squad/i.test(m.text), `a rival's city (${rival}) shows its production`);
      await closeModals(page);
    }
    log(`the city screen${rival ? `; ${rival} shows no production` : ""}`);

    // The Help: this mode's topics only.
    const help = await screen(page, "help", () => page.keyboard.press("?"));
    check(/Squads and supply/.test(help.text) && /The Worldseal/.test(help.text), "the Help lists the Cataclysm topics");
    check(!/settler|Armageddon|commerce rates/i.test(help.text), "the Help shows the classic game's text");
    const topics = await page.evaluate(() => [...document.querySelectorAll(".help-topic")].map((b) => b.dataset.topic));
    for (const t of topics) {
      await page.click(`.help-topic[data-topic="${t}"]`);
      const text = await page.evaluate(() => document.querySelector(".help-article").innerText);
      check(text.length > 80, `help topic ${t} is empty`);
      check(!BROKEN.test(text), `help topic ${t}: ${text.slice(0, 160)}`);
    }
    await closeModals(page);
    log(`the Help: ${topics.length} topics`);

    // The map tooltip over the capital.
    const at = await page.evaluate(() => {
      const d = window.ffh.S.view.cataclysm;
      const c = d.cities.find((c) => c.owner === window.ffh.S.view.me.player);
      window.ffh.mapView.centerOn(c.x, c.y);
      const r = window.ffh.mapView.canvas.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2, name: c.name };
    });
    await sleep(args.gpu ? 1500 : 8000);
    await page.mouse.move(at.x - 4, at.y - 4);
    await page.mouse.move(at.x, at.y);
    const tip = await until(
      () => page.evaluate(() => [...document.querySelectorAll(".tooltip, .richtip")].filter((b) => !b.hidden && b.getBoundingClientRect().width).map((b) => b.innerText).join(" | ")),
      "the map tooltip",
      20000,
    );
    await sleep(600);
    const tip2 = await page.evaluate(() => [...document.querySelectorAll(".tooltip, .richtip")].filter((b) => !b.hidden && b.getBoundingClientRect().width).map((b) => b.innerText).join(" | "));
    check(!BROKEN.test(tip2 || tip), `the map tooltip: ${tip2 || tip}`);
    check(!/commerce|Agriculture|Granary/.test(tip2 || tip), `the map tooltip is the classic game's: ${tip2 || tip}`);
    await shot(page, "map-tooltip");
    await page.mouse.move(at.x, VIEW.height - 6);
    log(`the map tooltip: ${(tip2 || tip).replace(/\n+/g, " / ").slice(0, 160)}`);

    // Research chosen, turns ended with Enter.
    await page.evaluate(async () => {
      const d = window.ffh.S.view.cataclysm;
      if (d.research.current) return;
      const t = d.research.techs.find((t) => t.can_research && !t.known);
      if (t) window.ffh.emit("applied", await window.ffh.api.cmd("cat_research", { tech: t.id }));
    });
    for (let i = 0; i < TURNS; i++) {
      const turn = await page.evaluate(() => window.ffh.S.status.turn);
      await page.keyboard.press("Enter");
      await until(() => page.evaluate((t) => window.ffh.S.status?.turn > t && window.ffh.S.status?.phase === "human", turn), `turn ${turn + 1}`, 60000);
      // A battle fought against the player opens its screen: leave it.
      if (await page.evaluate(() => document.body.classList.contains("battle-open"))) await page.keyboard.press("Escape");
      await closeModals(page);
      const r = await page.evaluate(() => document.getElementById("bottom-body")?.innerText ?? "");
      check(!BROKEN.test(r), `the turn report of turn ${turn + 1}: ${r.slice(0, 200)}`);
    }
    log(`${TURNS} turns ended`);

    // A phone: the Worldstrain chip leads the chips, and two taps on a tile move the selected squad.
    const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
    phone.on("pageerror", (e) => problems.push(`phone: page error: ${e.message}`));
    phone.on("console", (m) => {
      if (m.type() === "error") problems.push(`phone: console error: ${m.text()}`);
    });
    await phone.goto(`${server.url}/?renderer=hd2d${args.gpu ? "" : "&force3d=1"}`);
    await until(() => phone.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view?.cataclysm && window.ffh.S.status?.phase === "human")), "the game on the phone", 120000);
    await sleep(args.gpu ? 2500 : 10000);
    const first = await phone.evaluate(() => {
      const chips = [...document.querySelectorAll("#empire .cat-top-chip")].map((c) => ({ text: c.innerText, left: c.getBoundingClientRect().left }));
      chips.sort((a, b) => a.left - b.left);
      return chips[0]?.text ?? "";
    });
    check(/Worldstrain|Blowout/.test(first), `the phone's first chip is "${first}"`);
    const sq = await phone.evaluate(() => {
      const d = window.ffh.S.view.cataclysm;
      const s = d.squads.find((s) => s.owner === window.ffh.S.view.me.player && s.moves_left > 0);
      if (!s) return null;
      window.ffh.cataclysm.clearSelection();
      window.ffh.cataclysm.handleClick(s.x, s.y);
      window.ffh.mapView.centerOn(s.x, s.y);
      return { id: s.id, x: s.x, y: s.y };
    });
    check(sq, "a squad with moves left");
    await sleep(args.gpu ? 2500 : 10000);
    let moved = false;
    for (const [dx, dy] of [[1, 0], [0, 1], [-1, 0], [0, -1], [1, 1], [-1, -1], [1, -1], [-1, 1]]) {
      // Where that tile is on the screen: the map's own picking, scanned over the part of the
      // canvas that no panel covers.
      const at = await phone.evaluate(([x, y]) => {
        const mv = window.ffh.mapView;
        const rc = mv.canvas.getBoundingClientRect();
        for (let py = Math.max(rc.top, 220); py < Math.min(rc.bottom, 430); py += 10) {
          for (let px = rc.left + 10; px < rc.right - 10; px += 10) {
            const q = mv.plotAt({ clientX: px, clientY: py });
            if (q && q.x === x && q.y === y) return { x: px + 3, y: py + 3 };
          }
        }
        return null;
      }, [sq.x + dx, sq.y + dy]);
      if (!at) continue;
      await phone.touchscreen.tap(at.x, at.y);
      await sleep(600);
      await phone.touchscreen.tap(at.x, at.y);
      moved = await until(
        () => phone.evaluate((s) => window.ffh.S.view.cataclysm.squads.some((q) => q.id === s.id && (q.x !== s.x || q.y !== s.y)), sq),
        "the squad's move",
        args.gpu ? 4000 : 12000,
      ).catch(() => false);
      if (moved) break;
      await phone.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
    }
    check(moved, "two taps on a tile next to the selected squad did not move it");
    await phone.close();
    log("the phone: Worldstrain first, a squad moved with two taps");
  } finally {
    await browser.close();
    await server.stop();
  }
  if (problems.length) {
    console.error(`web-cataclysm-smoke: FAILED\n  ${[...new Set(problems)].join("\n  ")}`);
    process.exit(1);
  }
  console.log("web-cataclysm-smoke: ok");
}

main().catch((e) => {
  console.error(`web-cataclysm-smoke: FAILED: ${e.message}`);
  process.exit(1);
});
