#!/usr/bin/env node
// Screenshots of trade as the player sees it (docs/TRADE.md, section 9; docs/HUMAN.md, "Trade";
// CLAUDE.md, "Show UI changes in a screenshot"): phone-readable crops at deviceScaleFactor 2 on the
// HD-2D map (`?renderer=hd2d&force3d=1`; SwiftShader here unless --gpu).
//
//   node tools/trade-shots.mjs --bin <built>/ffh-play [--out docs/shots/trade]
//                              [--assets crates/ffh-play/assets] [--gpu] [--only ready,routes,closed,raid,help,checks]
//
// - `ready` (`stage trade-ready`, a Caravan in the capital): establish-action.png (the unit panel's
//   Establish route with its tooltip), establish-dialog.png (the dialog: the limits line, one row
//   per destination, the first row focused), establish-preview.png (the map behind it: the focused
//   row's path drawn as the preview line).
// - `routes` (`stage trade-routes`): trade-screen.png (the Trade screen: an open route with a lost
//   caravan, the domestic route with both on the road, the rival's route into the capital),
//   trade-map.png (the Trade layer: the routes' lines), caravan-tooltip.png (the plot tooltip of
//   the rival's caravan: its route and plunder), caravan-panel.png (the unit panel of a caravan of
//   the human's), city-routes.png (the city screen's Routes section).
//   Cargo and escorts: cargo-dialog.png (a foreign row's selectors: Fur +150%, Gems +50%, the pay
//   with them), cargo-only-copy.png (Dye on the second caravan: the only-copy warning),
//   caravan-ship.png (a caravan at sea drawn as a ship), caravan-land.png (the rival's caravan on
//   land: the Settler's figure) [ready]; refit-action.png and refit-dialog.png (a Caravan in the
//   port), escort-line.png (the escorting army's line), escort-actions.png (Stop escorting and
//   Escort with the pace rule), armies-escort.png (the Armies screen's Escort selector) [routes].
// - `checks` (`stage trade-routes` on `?renderer=flat` and `?renderer=iso`): check-flat-layer.png,
//   check-iso-layer.png (the Trade layer on the other two maps).
// - `closed` (`stage trade-closed`): trade-screen-closed.png (a route closed with its reason).
// - `raid` (`stage trade-raid`, at war): attack-preview.png (the attack preview on a caravan: the
//   plunder line).
// - `help` (`stage trade-ready`): help-civic.png (Foreign Trade's tooltip on the civics screen:
//   `+1 route.`, `+1 route from a coastal city.`, the Route keyword's reminder).

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { ROOT, parseArgs, findBin, tempDir, startServer, stage, until, skipped, chromiumLaunch } from "./web-lib.mjs";

const args = parseArgs(process.argv.slice(2), { bin: "value", out: "value", assets: "value", only: "value", gpu: "flag" });
const ONLY = args.only ? new Set(args.only.split(",")) : null;
const want = (part) => !ONLY || ONLY.has(part);
const OUT = path.resolve(args.out ?? path.join(ROOT, "docs", "shots", "trade"));
const ASSETS = path.resolve(args.assets ?? path.join(ROOT, "crates", "ffh-play", "assets"));
const VIEW = { width: 1100, height: 880 };
const TIP = [".tooltip:not([hidden])", ".tip-side:not([hidden])"];

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
const frame = () => (args.gpu ? 2500 : 25000);

/** A crop around the union of what `sels` match (with `pad` CSS px), at most `maxW` wide. */
async function crop(page, sels, name, { pad = 8, maxW = 560, maxH = 900 } = {}) {
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
  const w = Math.min(maxW, Math.ceil(r.x1 + pad) - x, r.w - x);
  const h = Math.min(maxH, Math.ceil(r.y1 + pad) - y, r.h - y);
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: w, height: h }, timeout: 300000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** A crop of `w` x `h` CSS px centred on the viewport's point (cx, cy). */
async function cropAt(page, cx, cy, w, h, name) {
  const x = Math.max(0, Math.round(cx - w / 2));
  const y = Math.max(0, Math.round(cy - h / 2));
  await page.screenshot({ path: path.join(OUT, name), clip: { x, y, width: Math.min(w, VIEW.width - x), height: Math.min(h, VIEW.height - y) }, timeout: 300000 });
  console.log(`${name}: ${w} x ${h} CSS px`);
}

/** Shows the tooltip of `sel` as a pointer over it. */
async function hover(page, sel, { tip = true } = {}) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e.scrollIntoView({ block: "center" });
    const r = e.getBoundingClientRect();
    const at = { clientX: r.left + 10, clientY: r.top > innerHeight / 2 ? r.top + 4 : r.bottom - 6 };
    e.dispatchEvent(new MouseEvent("mouseenter", { ...at, bubbles: false }));
    e.dispatchEvent(new MouseEvent("mouseover", { ...at, bubbles: true }));
    e.dispatchEvent(new MouseEvent("mousemove", { ...at, bubbles: true }));
  }, sel);
  if (tip) await until(() => page.evaluate(() => [...document.querySelectorAll(".tooltip, .tip-side")].some((e) => !e.hidden)), `the tooltip of ${sel}`, 15000);
  await sleep(600);
}

async function unhover(page, sel) {
  await page.evaluate((sel) => {
    const e = document.querySelector(sel);
    e?.dispatchEvent(new MouseEvent("mouseleave", { bubbles: false }));
    e?.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
  }, sel);
  await page.mouse.move(4, 4);
  await sleep(300);
}

/** Closes what the load opened and drops the toasts. */
async function settle(page) {
  for (let i = 0; i < 6; i++) {
    await page.keyboard.press("Escape");
    await sleep(250);
  }
  await dropToasts(page);
}

const dropToasts = (page) => page.evaluate(() => document.querySelectorAll("#toasts .toast").forEach((t) => t.remove()));

/** Serves staged game `kind` on the HD-2D map and runs `f(page, st)`. */
async function onStage(browser, bin, saves, kind, f, query = "?renderer=hd2d&force3d=1") {
  const st = await stage(bin, kind, path.join(saves, `staged-${kind}`));
  const server = await startServer({ bin, saves, extra: ["--load", st.save, "--assets", ASSETS] });
  try {
    const page = await browser.newPage({ viewport: VIEW, deviceScaleFactor: 2 });
    page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
    page.on("console", (m) => m.type() === "error" && console.error(`console: ${m.text()}`));
    try {
      await page.goto(`${server.url}/${query}`);
      await until(() => page.evaluate(() => !!(window.ffh?.ready && window.ffh.S.view && window.ffh.S.map && window.ffh.S.status?.phase === "human")), "the staged game", 240000, 250);
      await sleep(3000);
      await settle(page);
      await f(page, st);
    } finally {
      await page.close();
    }
  } finally {
    await server.stop();
  }
}

/** Selects `units` (centred) and waits for the panel. */
async function selectUnits(page, units, sel = "#unit-panel button.action") {
  await page.evaluate((ids) => window.ffh.units.select(ids, { center: true }), units);
  await until(() => page.evaluate((s) => !!document.querySelector(s), sel), `the unit panel (${sel})`, 30000);
  await sleep(1500);
}

async function centre(page, x, y, wait) {
  await page.evaluate(([x, y]) => window.ffh.mapView.centerOn(x, y, { animate: false }), [x, y]);
  await sleep(wait);
}

/** The viewport point of plot (x, y)'s centre on the HD-2D map (null when it is not drawn). */
function plotPoint(page, x, y) {
  return page.evaluate(([x, y]) => {
    const r3d = window.ffh.mapView.r3d;
    if (!r3d?.project || !r3d.cam) return null;
    const p = r3d.project(r3d.cam, [r3d.nearX(x) + 0.5, r3d.world.plotHeight(x, y), -(y + 0.5)]);
    if (!p) return null;
    const rect = document.querySelector("#mapwrap canvas.map").getBoundingClientRect();
    return { x: rect.left + p.x, y: rect.top + p.y };
  }, [x, y]);
}

/** The pointer over plot (x, y) of the HD-2D map (centred first), as a player hovers it. */
async function pointAt(page, x, y) {
  await centre(page, x, y, frame());
  const p = await plotPoint(page, x, y);
  if (!p) throw new Error(`plot (${x}, ${y}) is not drawn`);
  await page.mouse.move(p.x - 30, p.y - 30);
  await page.mouse.move(p.x, p.y, { steps: 4 });
  await sleep(1200);
}

// ---- `trade-ready`: the action, the dialog, its preview ----
async function shotsReady(browser, bin, saves) {
  await onStage(browser, bin, saves, "trade-ready", async (page, st) => {
    const action = '#unit-panel button.action[data-kind="trade"]';
    await selectUnits(page, [st.caravan], action);
    await hover(page, action);
    await crop(page, [action, ...TIP], "establish-action.png", { maxW: 700 });
    await unhover(page, action);
    await page.click(action);
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="establish"] .trade-offer')), "the Establish dialog", 30000);
    // The dialog stands at the foot of the screen: the map above it shows the focused row's path.
    await centre(page, st.cx + 6, st.cy - 4, 500);
    await page.focus('.modal[data-tag="establish"] .trade-offer');
    await sleep(frame());
    await crop(page, ['.modal[data-tag="establish"]'], "establish-dialog.png", { maxW: 760 });
    const top = await page.evaluate(() => document.querySelector('.modal[data-tag="establish"]').getBoundingClientRect().top);
    await page.screenshot({ path: path.join(OUT, "establish-preview.png"), clip: { x: 60, y: Math.max(0, top - 460), width: 900, height: 460 + 110 }, timeout: 300000 });
    console.log("establish-preview.png: 900 x 570 CSS px");
    // The foreign row: its cargo selectors (Fur preselected: +150%), Gems on the second (+50%).
    await page.click(`.modal[data-tag="establish"] .trade-offer-box[data-city="${st.far}"] .trade-offer`);
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="establish"] .cargo-panel')), "the cargo selectors", 10000);
    const pickBonus = (slot, name) =>
      page.evaluate(([slot, name]) => {
        const sel = document.querySelector(`.modal[data-tag="establish"] select.cargo-select[data-slot="${slot}"]`);
        const o = [...sel.options].find((x) => x.textContent.startsWith(name));
        sel.value = o.value;
        sel.dispatchEvent(new Event("change", { bubbles: true }));
      }, [slot, name]);
    await pickBonus(1, "Gems");
    await sleep(400);
    await crop(page, ['.modal[data-tag="establish"]'], "cargo-dialog.png", { maxW: 760, maxH: 1000 });
    // An only copy: its warning.
    await pickBonus(1, "Dye");
    await sleep(400);
    await crop(page, ['.modal[data-tag="establish"] .cargo-panel'], "cargo-only-copy.png", { maxW: 760 });
    await page.keyboard.press("Escape");
    await sleep(500);
    // A caravan on water: drawn as a ship.
    const ship = await page.evaluate(() => [...window.ffh.S.units.values()].find((u) => u.art !== undefined && u.caravan));
    if (ship) {
      await centre(page, ship.x, ship.y, frame());
      const p = await plotPoint(page, ship.x, ship.y);
      await cropAt(page, p?.x ?? VIEW.width / 2, (p?.y ?? VIEW.height / 2) - 30, 360, 240, "caravan-ship.png");
    } else console.log("caravan-ship.png: no caravan on water in the scene");
    // The rival's caravan on land: the Settler's figure.
    const tu = await page.evaluate((id) => window.ffh.S.units.get(id), st.theirs_unit);
    if (tu) {
      await centre(page, tu.x, tu.y, frame());
      const p = await plotPoint(page, tu.x, tu.y);
      await cropAt(page, p?.x ?? VIEW.width / 2, (p?.y ?? VIEW.height / 2) - 30, 360, 240, "caravan-land.png");
    }
    const drawn = await page.evaluate(() => window.ffh.mapView.r3d?.tradeDrawn ?? window.ffh.mapView.tradeDrawn ?? null);
    console.log(`establish-preview: lines drawn ${JSON.stringify(drawn)}`);
  });
}

// ---- `trade-routes`: the screen, the map, the caravans, the city ----
async function shotsRoutes(browser, bin, saves) {
  await onStage(browser, bin, saves, "trade-routes", async (page, st) => {
    await page.evaluate(() => window.ffh.openScreen("trade"));
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="trade-screen"] .trade-route')), "the Trade screen", 20000);
    await sleep(800);
    await crop(page, ['.modal[data-tag="trade-screen"]'], "trade-screen.png", { maxW: 900, maxH: 1200 });
    await page.keyboard.press("Escape");
    await sleep(500);
    // The Trade layer on the map: the routes' lines.
    await page.evaluate(() => window.ffh.setLayer("trade"));
    await centre(page, Math.round((st.cx + st.fx) / 2), st.cy - 1, frame());
    await dropToasts(page);
    await cropAt(page, VIEW.width / 2 - 140, VIEW.height / 2, 760, 420, "trade-map.png");
    console.log(`trade-map: lines drawn ${JSON.stringify(await page.evaluate(() => window.ffh.mapView.r3d?.tradeDrawn ?? null))}`);
    // The rival's caravan: the plot tooltip.
    const tu = await page.evaluate((id) => window.ffh.S.units.get(id), st.theirs_unit);
    await pointAt(page, tu.x, tu.y);
    await until(() => page.evaluate(() => !!document.querySelector(".tooltip:not([hidden]) .tip-caravan, .tip-side:not([hidden]) .tip-caravan")), "the caravan's tooltip line", 20000);
    await sleep(800);
    await crop(page, TIP, "caravan-tooltip.png", { maxW: 560 });
    await page.mouse.move(4, 4);
    await page.evaluate(() => window.ffh.units.hoverAt(null, null));
    await page.evaluate(() => document.querySelectorAll(".tooltip").forEach((t) => (t.hidden = true)));
    // A caravan of the human's: the unit panel.
    const mine = await page.evaluate(() => window.ffh.S.view.trade.routes.flatMap((r) => r.caravans).find((c) => c.unit !== null && c.state === "road")?.unit);
    await selectUnits(page, [mine], "#unit-panel .trade-lines");
    await crop(page, ["#unit-panel"], "caravan-panel.png", { maxW: 760, maxH: 560 });
    await page.evaluate(() => window.ffh.units.clearSelection?.());
    // Refit: the Caravan in the port, its action and dialog.
    const refit = '#unit-panel button.action[data-kind="trade"][data-type="trade_refit"]';
    await selectUnits(page, [st.refit_caravan], refit);
    await hover(page, refit);
    await crop(page, [refit, ...TIP], "refit-action.png", { maxW: 700 });
    await unhover(page, refit);
    await page.click(refit);
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="refit"] .cargo-picker')), "the Refit dialog", 20000);
    await sleep(600);
    await crop(page, ['.modal[data-tag="refit"]'], "refit-dialog.png", { maxW: 760, maxH: 1000 });
    await page.keyboard.press("Escape");
    await sleep(500);
    // Escort: the army's unit panel (its line and the Escort actions), then the Armies screen.
    await selectUnits(page, st.escort_units, '#unit-panel button.action[data-type="escort_caravan"]');
    await page.mouse.move(4, 4);
    await sleep(400);
    await crop(page, ["#unit-panel .army-line"], "escort-line.png", { maxW: 760 });
    const esc = '#unit-panel button.action[data-type="escort_caravan"]';
    await hover(page, esc);
    await crop(page, ['#unit-panel button.action[data-type="stop_escort"]', esc, ...TIP], "escort-actions.png", { maxW: 760, maxH: 700 });
    await unhover(page, esc);
    await page.evaluate(() => {
      window.ffh.openScreen("armies");
    });
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="armies"] .escort-station')), "the Armies screen's escort", 20000);
    await sleep(600);
    await page.evaluate(() => {
      const c = document.querySelector('.modal[data-tag="armies"] .escort-station').closest(".army-card");
      c.id = "escort-card";
      c.scrollIntoView({ block: "center" });
    });
    await sleep(300);
    await crop(page, ["#escort-card .army-title", "#escort-card .escort-station"], "armies-escort.png", { maxW: 760 });
    await page.keyboard.press("Escape");
    await sleep(500);
    // The city screen's Routes section (the port: a route from it and one into it).
    await page.evaluate((c) => window.ffh.emit("open-city", c), st.port);
    await until(() => page.evaluate(() => !!document.querySelector(".modal.city-screen section.city-routes")), "the city screen's Routes", 30000);
    await sleep(1200);
    await page.evaluate(() => document.querySelector(".modal.city-screen section.city-routes").scrollIntoView({ block: "center" }));
    await sleep(400);
    await crop(page, [".modal.city-screen section.city-routes"], "city-routes.png", { maxW: 600 });
  });
}

// ---- `trade-closed`: the screen with a closed route ----
async function shotsClosed(browser, bin, saves) {
  await onStage(browser, bin, saves, "trade-closed", async (page) => {
    await page.evaluate(() => window.ffh.openScreen("trade"));
    await until(() => page.evaluate(() => !!document.querySelector('.modal[data-tag="trade-screen"] .trade-route.closed')), "the closed route", 20000);
    await sleep(800);
    await crop(page, ['.modal[data-tag="trade-screen"] .trade-route.closed'], "trade-screen-closed.png", { maxW: 820 });
  });
}

// ---- `trade-raid`: the attack preview on a caravan ----
async function shotsRaid(browser, bin, saves) {
  await onStage(browser, bin, saves, "trade-raid", async (page, st) => {
    await selectUnits(page, [st.warrior]);
    await pointAt(page, st.tx, st.ty);
    await until(() => page.evaluate(() => !!document.querySelector(".tooltip:not([hidden]) .tip-plunder, .tip-side:not([hidden]) .tip-plunder")), "the attack preview's plunder", 30000);
    await sleep(800);
    await crop(page, TIP, "attack-preview.png", { maxW: 600 });
  });
}

// ---- `help`: a reworded key in a civic's tooltip ----
async function shotsHelp(browser, bin, saves) {
  await onStage(browser, bin, saves, "trade-ready", async (page) => {
    const idx = await page.evaluate(() => {
      const t = window.ffh.S.data.tables.CivicType;
      return Object.keys(t).find((k) => t[k].type === "CIVIC_FOREIGN_TRADE");
    });
    // (Not awaited: the screen's promise settles when it closes.)
    await page.evaluate(() => {
      window.ffh.openScreen("civics");
    });
    const sel = `button.civic[data-civic="${idx}"]`;
    await until(() => page.evaluate((s) => !!document.querySelector(s), sel), "the civics screen", 30000);
    await sleep(600);
    await hover(page, sel);
    await crop(page, TIP, "help-civic.png", { maxW: 640 });
  });
}

// ---- `checks`: the Trade layer on the flat and the isometric maps ----
async function shotsChecks(browser, bin, saves) {
  for (const r of ["flat", "iso"]) {
    await onStage(
      browser,
      bin,
      saves,
      "trade-routes",
      async (page, st) => {
        await page.evaluate(() => window.ffh.setLayer("trade"));
        await centre(page, Math.round((st.cx + st.fx) / 2), st.cy, 2500);
        await dropToasts(page);
        await cropAt(page, VIEW.width / 2 - 150, VIEW.height / 2, 780, 440, `check-${r}-layer.png`);
        const drawn = await page.evaluate(() => window.ffh.mapView.tradeDrawn ?? window.ffh.mapView.iso?.tradeDrawn ?? null);
        console.log(`check-${r}-layer: ${JSON.stringify(drawn)}`);
      },
      `?renderer=${r}`,
    );
  }
}

async function main() {
  const bin = findBin(args.bin);
  const pw = await loadPlaywright();
  if (!pw) return skipped("trade-shots", "no playwright package");
  fs.mkdirSync(OUT, { recursive: true });
  const saves = tempDir("trade-shots");
  const browser = await pw.chromium.launch(chromiumLaunch(!!args.gpu));
  try {
    if (want("ready")) await shotsReady(browser, bin, saves);
    if (want("routes")) await shotsRoutes(browser, bin, saves);
    if (want("closed")) await shotsClosed(browser, bin, saves);
    if (want("raid")) await shotsRaid(browser, bin, saves);
    if (want("help")) await shotsHelp(browser, bin, saves);
    if (want("checks")) await shotsChecks(browser, bin, saves);
  } finally {
    await browser.close();
    fs.rmSync(saves, { recursive: true, force: true });
  }
  console.log(`trade-shots: written to ${path.relative(process.cwd(), OUT) || OUT}`);
  return 0;
}

main().then(
  (code) => process.exit(code ?? 0),
  (e) => {
    console.error(`trade-shots: FAILED: ${e.stack ?? e}`);
    process.exit(1);
  },
);
