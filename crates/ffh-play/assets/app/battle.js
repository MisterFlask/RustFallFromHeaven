// The battle screens (the battle UI design, DESIGN.md; phase6_plan 9.3 and 15): the plan before an
// attack (battle_plan.js), the battle watched from its log (battle_watch.js) and the report
// (battle_report.js), over one stage (battle_stage.js) in an overlay above the dimmed map. This
// module owns the overlay, the frame loop, the keys while it is open, the teaching ledger, and the
// sources the plan asks for previews and attacks.
//
// A source: `{preview({force?: [unit ids], stance?}) -> EngagePreview, alternatives({force, stance})
// -> [Alternative], attack({force, stance}) -> {log}}` (battle_log.js lists the shapes). For now the
// screens open only from the dev entry, `?battle=<fixture>` (main.js), over a fixture of
// battle_fixture.js (docs/HUMAN.md, "The battle screens"); `apiSource` is the protocol's, for when
// the engine's queries land (phase6_plan step 9).

import * as api from "./api.js";
import { h, toast } from "./dom.js";
import { installFrames } from "./isoui.js";
import { installIsoFont } from "./isofont.js";
import { Ledger } from "./battle_teach.js";
import { compositionKey } from "./battle_log.js";
import { Stage } from "./battle_stage.js";
import * as planScreen from "./battle_plan.js";
import * as watchScreen from "./battle_watch.js";
import * as reportScreen from "./battle_report.js";

const B = { root: null, frame: null, ov: null, ui: null, stage: null, screen: null, raf: 0, last: 0, ledger: null, source: null, title: null, onClose: null };

/** Whether the battle overlay is open. */
export function battleOpen() {
  return !!B.root;
}

/** What the screens share: the stage and its layers, the ledger, the source, and the way between
 *  the screens. */
const ctx = {
  get stage() {
    return B.stage;
  },
  get ov() {
    return B.ov;
  },
  get ui() {
    return B.ui;
  },
  get ledger() {
    return B.ledger;
  },
  get source() {
    return B.source;
  },
  reduced: () => globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  toPlan: () => show(planScreen),
  toWatch: (log, opts) => show(watchScreen, log, opts),
  toReport: (log, opts) => show(reportScreen, log, opts),
  close: () => closeBattle(),
};

function show(screen, ...args) {
  B.screen?.leave?.(ctx);
  B.ov.replaceChildren();
  B.ui.replaceChildren();
  B.screen = screen;
  B.root.dataset.screen = screen.NAME;
  return screen.enter(ctx, ...args);
}

function mount() {
  if (B.root) return;
  installFrames();
  installIsoFont();
  B.ledger ??= new Ledger();
  const canvas = h("canvas.bt-stage", { "aria-hidden": "true" });
  B.ov = h("div.bt-ov");
  B.ui = h("div.bt-ui");
  B.frame = h("div.bt-frame", canvas, B.ov, B.ui);
  B.root = h("div#battle", { role: "dialog", "aria-modal": "true", "aria-label": "Battle" }, B.frame);
  document.body.append(B.root);
  document.body.classList.add("battle-open");
  B.stage = new Stage(canvas);
  document.addEventListener("keydown", onKey, true);
  B.last = performance.now();
  B.raf = requestAnimationFrame(loop);
}

/** Closes the overlay (the map comes back). */
export function closeBattle() {
  if (!B.root) return;
  B.screen?.leave?.(ctx);
  B.screen = null;
  cancelAnimationFrame(B.raf);
  document.removeEventListener("keydown", onKey, true);
  B.root.remove();
  B.root = null;
  document.body.classList.remove("battle-open");
  const done = B.onClose;
  B.onClose = null;
  done?.();
}

function loop(now) {
  if (!B.root) return;
  const dt = Math.min(100, now - B.last);
  B.last = now;
  try {
    B.screen?.frame?.(ctx, dt, now);
  } catch (e) {
    console.error("battle frame:", e);
  }
  B.raf = requestAnimationFrame(loop);
}

/** The keys while the overlay is open: the screen's, and none reach the map. */
function onKey(ev) {
  if (!B.root) return;
  if (B.screen?.key?.(ctx, ev)) ev.preventDefault();
  ev.stopPropagation();
}

/** Opens the plan of an attack over `source` (it asks for the first preview). `onClose` runs when
 *  the overlay goes. */
export async function openBattlePlan(source, { onClose } = {}) {
  B.source = source;
  B.onClose = onClose ?? null;
  mount();
  try {
    await show(planScreen);
  } catch (e) {
    closeBattle();
    toast(`The battle plan cannot open: ${e.message ?? e}`, "error");
  }
}

/** Opens a battle's log: watched in the viewer's watch mode, or straight to its report. */
export function openBattleLog(log, { report = false, onClose } = {}) {
  B.onClose = onClose ?? null;
  mount();
  return report || B.ledger.watchMode() === "result" ? show(reportScreen, log) : show(watchScreen, log);
}

// ---------------------------------------------------------------------------------------------
// Sources

/** The protocol's source for an attack of army `army` (or the units on `from`) on plot `to`
 *  (phase6_plan 9.2; not wired until the engine answers these). */
export function apiSource({ army, from, to }) {
  const where = army !== undefined ? { army } : { from };
  return {
    preview: (req) => api.query("engage_preview", { ...where, to, ...req }),
    alternatives: async (req) => (await api.query("engage_preview", { ...where, to, ...req, alternatives: true })).alternatives ?? [],
    attack: async (req) => {
      const r = await api.cmd("engage", { ...where, to, ...req });
      const id = r?.report ?? r?.result?.report;
      return { log: await api.query("battle_log", { id }) };
    },
  };
}

const sortedKey = (ids) => ids.slice().sort((a, b) => a - b).join(",");

/** A source over a fixture (`assets/test/fixtures/battle_*.json`): the previews of every force it
 *  holds, the stances' picks, and its battle logs (an attack with a force it has no log for plays
 *  its first log, and says so). Plays the engine's part: it is not the client's. */
export function fixtureSource(fx) {
  const F = fx.preview.fighters;
  const me = 0;
  const pool = F.filter((f) => f.side === me).map((f) => f.unit);
  const bOf = (id) => F.find((f) => f.unit === id).b;
  const pickOf = (stance) => fx.stance_picks[stance] ?? fx.stance_picks[fx.stance];
  const winOf = (ids) => fx.forecasts[sortedKey(ids)]?.win_permille ?? 0;
  const alternatives = ({ force, stance }) => {
    stance ??= fx.stance;
    const out = [];
    const add = (kind, st, f) => {
      const fc = fx.forecasts[sortedKey(f)];
      if (fc) out.push({ kind, stance: st, force: f, win_permille: fc.win_permille, losses_if_win_x10: fc.losses_if_win_x10 });
    };
    add("StancePick", stance, pickOf(stance));
    // the two best single swaps that change the force's make-up (an Archer L1 for another is none)
    const swaps = [];
    for (const r of pool.filter((x) => !force.includes(x))) for (const c of force) swaps.push(force.map((x) => (x === c ? r : x)));
    swaps.sort((a, b) => winOf(b) - winOf(a));
    const kinds = new Set([compositionKey(F, force)]);
    for (const f of swaps) {
      const k = compositionKey(F, f);
      if (kinds.has(k) || kinds.size > 2) continue;
      kinds.add(k);
      add("Swap", stance, f);
    }
    for (const st of Object.keys(fx.stance_picks)) if (st !== stance) add("OtherStance", st, pickOf(st));
    return out;
  };
  return {
    fixture: fx,
    async preview({ force, stance } = {}) {
      stance ??= fx.stance;
      const auto = pickOf(stance);
      const ids = force ?? auto;
      const fc = fx.forecasts[sortedKey(ids)];
      if (!fc) throw new Error(`the fixture has no forecast for the force ${ids.join(", ")}`);
      const fighters = F.map((f) => ({ ...f, slot: ids.includes(f.unit) ? ids.indexOf(f.unit) : null }));
      return {
        ...fx.preview,
        fighters,
        force: ids.map(bOf),
        auto_pick: auto.map(bOf),
        reserve: pool.filter((x) => !ids.includes(x)).map(bOf),
        stance,
        forecast: fc,
      };
    },
    async alternatives(req) {
      // The engine answers this in a second call; the fixture's is at hand, so wait a moment as it would.
      await new Promise((r) => setTimeout(r, 30));
      return alternatives(req);
    },
    async attack({ force }) {
      const hit = fx.logs.find((l) => sortedKey(l.force) === sortedKey(force));
      if (!hit) toast("The fixture has no battle for this force: it plays its first one.", "info", 6000);
      return { log: structuredClone((hit ?? fx.logs[0]).log) };
    },
  };
}

/** The dev entry (`?battle=<name>`, main.js): the plan over fixture `name` of battle_fixture.js
 *  (loaded only then). `viewer` ("new", "some", "veteran") sets the teaching ledger first. */
export async function openDevBattle(name, { viewer } = {}) {
  const { FIXTURES } = await import("./battle_fixture.js");
  const fx = FIXTURES[name] ?? FIXTURES[Object.keys(FIXTURES)[0]];
  if (!FIXTURES[name]) toast(`No battle fixture "${name}": showing "${Object.keys(FIXTURES)[0]}".`, "info", 6000);
  B.ledger ??= new Ledger();
  if (viewer) B.ledger.setViewer(viewer);
  await openBattlePlan(fixtureSource(fx));
}

/** Hooks for the screenshot and smoke scripts (`window.ffh.battle`). */
export const dev = {
  open: openDevBattle,
  close: closeBattle,
  get screen() {
    return B.screen?.NAME ?? null;
  },
  get ledger() {
    return B.ledger;
  },
  plan: planScreen.dev,
  watch: watchScreen.dev,
  report: reportScreen.dev,
};
if (globalThis.window?.ffh) globalThis.window.ffh.battle = dev;
