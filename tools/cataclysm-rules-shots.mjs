#!/usr/bin/env node
// Screenshots of how the Cataclysm screens state the pacing pass's rules (docs/CATACLYSM.md, "Rules
// changes: the pacing pass"): phone-readable crops at deviceScaleFactor 2 on the HD-2D map of
// - diplomacy at war, with our standing peace offer (the peace rule, the offer's turns left);
// - the Rituals tab after Vent the Seal was cast (the recast time, "Ready in N turns");
// - the Research tab (the tier costs, a tech's Strain for its first researcher, the Lore rule);
// - the new-game screen (the starting factions) and the Worldseal screen (King Cinder's Host's
//   entry, Great Furnace's ward, the Breach, the Furnace victory).
//
//   node tools/cataclysm-rules-shots.mjs --bin <built>/ffh-play --out DIR [--gpu]

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", assets: "value", gpu: "flag", out: "value" });
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const OUT = path.resolve(args.out ?? "cataclysm-rules-shots");
const VIEW = { width: 900, height: 1700 };
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

async function closeModals(page) {
  for (let i = 0; i < 6 && (await page.evaluate(() => !!document.querySelector("#modals .modal"))); i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
}

/** Clicks the first enabled button of the top modal whose text matches `re`; false when none does. */
const press = (page, re) =>
  page.evaluate((src) => {
    const m = [...document.querySelectorAll("#modals .modal")].pop();
    const b = m && [...m.querySelectorAll("button")].find((x) => !x.disabled && new RegExp(src).test(x.innerText.trim()));
    b?.click();
    return !!b;
  }, re.source);

async function open(page, name) {
  await closeModals(page);
  await page.evaluate((n) => window.ffh.openScreen(n), name);
  await until(() => page.evaluate(() => !!document.querySelector("#modals .modal")), `the ${name} screen`, 15000);
  await sleep(400);
}

/** A crop of the top modal, at most `maxH` CSS px high. */
async function shot(page, name, maxH = 1000) {
  const r = await page.evaluate(() => {
    const b = [...document.querySelectorAll("#modals .modal")].pop().getBoundingClientRect();
    return { x: b.left, y: b.top, w: b.width, h: b.height };
  });
  const clip = { x: Math.max(0, r.x), y: Math.max(0, r.y), width: Math.min(r.w, VIEW.width - Math.max(0, r.x)), height: Math.min(r.h, maxH, VIEW.height - Math.max(0, r.y)) };
  await page.screenshot({ path: path.join(OUT, `${name}.png`), clip, timeout: 120000 });
  console.log(`${name}.png: ${Math.round(clip.width)} x ${Math.round(clip.height)} CSS px`);
}

async function main() {
  const pw = await loadPlaywright();
  if (!pw) return skipped("cataclysm-rules-shots", "playwright not found");
  const bin = findBin(args.bin);
  if (!bin) return skipped("cataclysm-rules-shots", "no ffh-play binary");
  fs.mkdirSync(OUT, { recursive: true });
  const server = await startServer({ bin, saves: tempDir("cataclysm-shots"), extra: ["--no-game", "--assets", ASSETS] });
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  try {
    const page = await browser.newPage({ viewport: VIEW, deviceScaleFactor: 2 });
    page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
    await page.goto(`${server.url}/?renderer=hd2d${args.gpu ? "" : "&force3d=1"}`);
    await until(() => page.evaluate(() => !!document.getElementById("title-cataclysm")), "the title", 120000);
    await page.click("#title-cataclysm");
    // The Tappers' College starts with the Mana for a ritual.
    await until(() => page.evaluate(() => !!document.querySelector('.cat-fac-card[data-faction="amurites"]')), "the New Cataclysm screen", 30000);
    await page.click('.cat-fac-card[data-faction="amurites"]');
    // The new-game screen: the starting factions (King Cinder's Host only enters mid-game).
    await shot(page, "new-game");
    await press(page, /Start/);
    await until(() => page.evaluate(() => window.ffh.S.view?.mode === "cataclysm" && window.ffh.S.status?.phase === "human"), "the Cataclysm game", 120000);
    await sleep(2500);

    // The Worldseal: how King Cinder's Host enters and how the Furnace wins.
    await open(page, "finance");
    await shot(page, "worldseal", 1600);

    // Research: the tier costs and the Strain of a tech nobody has researched.
    await open(page, "techs");
    await shot(page, "research");

    // Rituals: cast Vent the Seal, then the tab shows its recast time.
    await open(page, "civics");
    const cast = await page.evaluate(() => {
      const m = [...document.querySelectorAll("#modals .modal")].pop();
      const row = [...m.querySelectorAll("*")].filter((e) => /Vent the Seal/.test(e.innerText ?? "") && e.querySelector("button")).pop();
      const b = row && [...row.querySelectorAll("button")].find((x) => /Cast/.test(x.innerText) && !x.disabled);
      b?.click();
      return !!b;
    });
    if (!cast) console.error("Vent the Seal could not be cast");
    await sleep(800);
    await press(page, /^(Cast|Confirm|Yes|OK)/);
    await sleep(800);
    await open(page, "civics");
    await shot(page, "rituals");

    // Diplomacy: war on the first rival, then our peace offer.
    await open(page, "diplomacy");
    await press(page, /^Declare war/);
    await sleep(600);
    await press(page, /Declare war|Confirm|Yes/);
    await sleep(800);
    await open(page, "diplomacy");
    await press(page, /^Offer peace/);
    await sleep(800);
    await open(page, "diplomacy");
    await shot(page, "diplomacy", 760);
  } finally {
    await browser.close();
    await server.stop?.();
  }
}

main().then(
  () => process.exit(0),
  (e) => {
    console.error(`cataclysm-rules-shots: FAILED\n  ${e.message}`);
    process.exit(1);
  },
);
