#!/usr/bin/env node
// Screenshots of the battle screens (docs/HUMAN.md, "The battle screens") over the dev fixtures
// (`?battle=<fixture>`: no game is played, no rules are involved): the plan, the plan with a unit's
// tooltip, the battle on a blow's contact frame, on a critical or killing blow, at its end, and the
// report. The screens wear the isometric skin whatever the renderer, so the map behind them is the
// isometric one (no WebGL: a frame in a moment).
//
//   node tools/battle-shots.mjs --bin <built>/ffh-play [--out DIR] [--assets crates/ffh-play/assets]
//                               [--fixtures field,city] [--only plan,tip,blow,crit,end,report]
//                               [--scale 1] [--view 1280x800] [--page] [--renderer hd2d] [--gpu]
//
// `--page`: the whole viewport (the map behind the overlay), not the overlay's frame alone;
// `--renderer hd2d`: over the HD-2D map (`force3d=1`: in software here unless `--gpu`).

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", fixtures: "value", only: "value", scale: "value", view: "value", page: "flag", renderer: "value", gpu: "flag" });
const OUT = path.resolve(args.out ?? path.join(ROOT, "docs", "shots", "battle"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const FIXTURES = (args.fixtures ?? "field,city").split(",");
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const SCALE = Number(args.scale ?? 1);
const [VW, VH] = (args.view ?? "1280x800").split("x").map(Number);
const RENDERER = args.renderer ?? "iso";

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

const pw = await loadPlaywright();
if (!pw) process.exit(skipped("battle-shots", "the playwright package was not found"));
const bin = findBin(args.bin);
fs.mkdirSync(OUT, { recursive: true });
const server = await startServer({ bin, saves: tempDir("battle-shots"), extra: ["--assets", ASSETS, "--seed", "3", "--opponents", "2", "--world", "WORLDSIZE_DUEL"] });
const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
let failed = false;
try {
  for (const fx of FIXTURES) {
    const page = await browser.newPage({ viewport: { width: VW, height: VH }, deviceScaleFactor: SCALE });
    page.on("pageerror", (e) => ((failed = true), console.error(`page error: ${e.message}`)));
    page.on("console", (m) => m.type() === "error" && console.error(`console: ${m.text()}`));
    await page.goto(`${server.url}/?renderer=${RENDERER}${RENDERER === "hd2d" ? "&force3d=1" : ""}&battle=${fx}`);
    const ready = (expr, what) => until(() => page.evaluate(expr), what, 60000, 100);
    const shot = async (name) => {
      await sleep(150);
      const file = path.join(OUT, `${fx}-${name}.png`);
      if (args.page) await page.screenshot({ path: file });
      else await page.locator("#battle .bt-frame").screenshot({ path: file });
      console.log(file);
    };
    await ready(`!!window.ffh?.battle && window.ffh.battle.screen === "plan" && window.ffh.battle.plan.ready`, "the plan");
    await page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));
    await page.evaluate(() => document.fonts.ready.then(() => true));
    if (want("plan")) await shot("plan");
    if (want("tip")) {
      const at = await page.evaluate(() => {
        const p = window.ffh.battle.plan;
        const f = p.preview.fighters.find((g) => g.side === 0 && g.lane !== null && !!p.point(g.b));
        return f ? p.point(f.b) : null;
      });
      if (at) {
        await page.mouse.move(at.x, at.y);
        await page.mouse.move(at.x + 1, at.y + 1);
        await shot("tip");
        await page.mouse.move(2, 2);
      }
    }
    await page.evaluate(() => (window.ffh.battle.prefs.set("watch", "watch"), window.ffh.battle.plan.attack()));
    await ready(`window.ffh.battle.screen === "watch" && window.ffh.battle.watch.state().ready`, "the battle");
    const find = (src) => page.evaluate(`window.ffh.battle.watch.find(${src})`);
    const freeze = (i, ms) => page.evaluate(([i, ms]) => window.ffh.battle.watch.freeze(i, ms), [i, ms]);
    if (want("blow")) {
      const i = await find(`(e) => (e.hits ?? []).some((x) => !x.miss && !x.killed && !x.crit)`);
      if (i >= 0) {
        await freeze(i, 120);
        await shot("blow");
      }
    }
    if (want("crit")) {
      const i = await find(`(e) => (e.hits ?? []).some((x) => x.killed || x.crit)`);
      if (i >= 0) {
        await freeze(i, 200);
        await shot("crit");
      }
    }
    if (want("end")) {
      const n = await page.evaluate(() => window.ffh.battle.watch.state().n);
      // paused on the last state: the fallen lie, the log is whole, no banner yet
      await page.evaluate((n) => window.ffh.battle.watch.seek(n), n);
      await shot("end");
    }
    await page.evaluate(() => window.ffh.battle.watch.skip());
    await ready(`window.ffh.battle.screen === "report" && window.ffh.battle.report.ready`, "the report");
    if (want("report")) await shot("report");
    await page.close();
  }
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  await browser.close();
  await server.stop();
}
process.exit(failed ? 1 : 0);
