// The battle screens (formation battles, phase 6 revision 4; docs/HUMAN.md, "The battle screens"):
// the plan before an attack (battle_plan.js), the battle watched from its log (battle_watch.js) and
// the report (battle_report.js), over one battlefield (battle_stage.js) in an overlay above the
// dimmed map. This module owns the overlay, the frame loop, the keys while it is open, the
// viewer's preferences, and the sources the plan asks for its preview and the attack.
//
// A source: `{preview() -> EngagePreview, attack({declare_war}) -> {log} | null}` (battle_log.js
// lists the shapes; null: the attack was not made). Each side fights in the formation its tactic
// gives (docs/TACTICS.md): the plan shows it, nobody arranges it. `apiSource` is the protocol's (`engage_preview`, the
// `engage` command, `battle_log`): the plan opens when the player orders an army or a stack onto an
// enemy plot (units.js), a battle's log from its message, the turn report or its marker on the map
// (`openBattleReport`). The dev entry, `?battle=<fixture>` (main.js), opens a fixture of
// battle_fixture.js instead (made by test/make_battle_fixtures.mjs from the prototype).

import * as api from "./api.js";
import { run } from "./commands.js";
import { emit } from "./state.js";
import { h, toast } from "./dom.js";
import { installFrames } from "./isoui.js";
import { installIsoFont } from "./isofont.js";
import { battleToast, prefs } from "./engagements.js";
import { Stage, btn } from "./battle_stage.js";
import * as planScreen from "./battle_plan.js";
import * as watchScreen from "./battle_watch.js";
import * as reportScreen from "./battle_report.js";

const B = { root: null, frame: null, ov: null, ui: null, stage: null, screen: null, raf: 0, last: 0, source: null, onClose: null, queue: null };

/** Whether the battle overlay is open. */
export function battleOpen() {
  return !!B.root;
}

// The viewer's preferences (whether an attack is watched, how the battles fought against the
// player open, the replay's speed): engagements.js keeps them (Options shows them too).

export { prefs };

/** What the screens share: the stage and its layers, the preferences, the source, and the way
 *  between the screens. */
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
  get source() {
    return B.source;
  },
  get queue() {
    return B.queue;
  },
  prefs,
  reduced: () => globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  toPlan: () => show(planScreen),
  toWatch: (log) => show(watchScreen, log),
  toReport: (log) => show(reportScreen, log),
  close: () => closeBattle(),
  /** The battles queued at turn start, when more than one: "Battle 2 of 5" and "Skip the rest"
   *  (this one closes, the rest count as shown and stay in the turn report), or null. */
  queueRow() {
    const q = B.queue;
    if (!q) return null;
    return h(
      "div.bt-row.bt-queue",
      h("span#bt-queue-at", `Battle ${q.at} of ${q.of}`),
      btn("Skip the rest", {
        cls: "sm",
        key: "skip-rest",
        title: "Close this battle and the rest queued; they stay in the turn report",
        onclick: () => {
          q.skip?.();
          closeBattle();
        },
      }),
    );
  },
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
  B.queue = null;
  document.body.classList.remove("battle-open");
  const done = B.onClose;
  B.onClose = null;
  done?.();
  // The popups the battle held back (a captured city's production, ...) may show now.
  emit("battle-closed");
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

/** Whether `ev` is one of the game's own global hotkeys (main.js `onKey`: the F-keys of the
 *  screens and the capital, Ctrl+S, Ctrl+L, Ctrl+1/2/3, Tab, Backspace): stopped while the overlay
 *  is open, and the browser's default too (F5 would reload the page, Ctrl+S save it). */
function gameHotkey(ev) {
  const ctrl = ev.ctrlKey || ev.metaKey;
  if (/^F([1-9]|10)$/.test(ev.key) && !ctrl && !ev.altKey) return true;
  if (ctrl && !ev.altKey && ["s", "l", "1", "2", "3"].includes(ev.key.toLowerCase())) return true;
  return (ev.key === "Tab" || ev.key === "Backspace") && !ctrl && !ev.altKey;
}

/** The keys while the overlay is open: the screen's, and none reach the map; the game's own
 *  hotkeys do nothing (nor their browser defaults), the browser's other keys keep theirs; a dialog
 *  over it (a confirmation the attack asks) takes its own. */
function onKey(ev) {
  if (!B.root || document.querySelector("#modals .backdrop:not(.under)")) return;
  if (B.screen?.key?.(ctx, ev) || gameHotkey(ev)) ev.preventDefault();
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

/** Opens a battle's log: watched, or straight to its report (`report`, or the viewer's choice).
 *  `queue`: its place among the battles queued at turn start (`{at, of, skip}`), or null. */
export function openBattleLog(log, { report = false, onClose, queue = null } = {}) {
  B.onClose = onClose ?? null;
  B.queue = queue;
  mount();
  return report ? show(reportScreen, log) : show(watchScreen, log);
}

// ---------------------------------------------------------------------------------------------
// Sources

/** The protocol's source for an attack of army `army` (or the selected `units`, or the units on
 *  plot `from`: `{x, y}`) on plot `to` (`{x, y}`): the plan's preview and the attack (the `engage`
 *  command through commands.js, which asks to declare war when the attack would; then the
 *  battle's log). */
export function apiSource({ army, units, from, to }) {
  const where = army !== undefined && army !== null ? { army } : units ? { units } : { from };
  const req = (r) => {
    const out = { ...where, to };
    // The plan said "Attacking declares war" and the player attacked: the confirmation.
    if (r?.declare_war) out.declare_war = true;
    return out;
  };
  return {
    preview: () => api.query("engage_preview", req({})),
    attack: async (r) => {
      const reply = await run({ c: "engage", ...req(r) });
      const id = reply?.r?.report;
      if (id === null || id === undefined) {
        if (reply) toast("No battle was fought.", "info");
        return null;
      }
      try {
        return { log: await api.query("battle_log", { report: id }) };
      } catch (e) {
        // Fought, but its log cannot be read: back to the map, with the battle's message (and its
        // links) that the screens were to show (commands.js left it to them).
        console.warn("battle: the battle's log:", e);
        closeBattle();
        for (const line of reply.log ?? []) if (line.kind === "engagement" && line.report === id) battleToast(line);
        return null;
      }
    },
  };
}

/** Opens battle report `id` (`report`: straight to the report, else watched): its log from the
 *  engine. A report kept without its log (older ones) says so. */
export async function openBattleReport(id, { report = false } = {}) {
  let log;
  try {
    log = await api.query("battle_log", { report: id });
  } catch (e) {
    toast(e?.code === "gone" ? `The record of battle #${id} is no longer kept (only the summary of older battles is).` : `Battle #${id}: ${e.detail ?? e.message ?? e}`, "info", 6000);
    return;
  }
  await openBattleLog(log, { report });
}

/** A source over a fixture (battle_fixture.js): its preview (the rules' auto formation and its
 *  forecast) and the auto formation's battle log. Plays the engine's part: it is not the
 *  client's. */
export function fixtureSource(fx) {
  const base = fx.preview;
  return {
    fixture: fx,
    async preview() {
      // The engine answers in a moment: so does the fixture.
      await new Promise((r) => setTimeout(r, 40));
      return structuredClone(base);
    },
    async attack() {
      return { log: structuredClone(fx.logs[0].log) };
    },
  };
}

/** The dev entry (`?battle=<name>`, main.js): the plan over fixture `name` of battle_fixture.js
 *  (loaded only then). */
export async function openDevBattle(name) {
  const { FIXTURES } = await import("./battle_fixture.js");
  const first = Object.keys(FIXTURES)[0];
  const fx = FIXTURES[name] ?? FIXTURES[first];
  if (!FIXTURES[name]) toast(`No battle fixture "${name}": showing "${first}" (${Object.keys(FIXTURES).join(", ")}).`, "info", 6000);
  await openBattlePlan(fixtureSource(fx));
}

/** Hooks for the screenshot and smoke scripts (`window.ffh.battle`). */
export const dev = {
  open: openDevBattle,
  openReport: openBattleReport,
  openLog: openBattleLog,
  close: closeBattle,
  get screen() {
    return B.screen?.NAME ?? null;
  },
  get source() {
    return B.source;
  },
  prefs,
  plan: planScreen.dev,
  watch: watchScreen.dev,
  report: reportScreen.dev,
};
if (globalThis.window?.ffh) globalThis.window.ffh.battle = dev;
