// Engagement combat in the client (docs/ENGAGEMENTS.md; docs/HUMAN.md, "Engagement combat"): what
// the map, the panels and the screens need besides the battle screens themselves (battle.js and
// its modules, loaded on first use): whether an attack is a battle, a unit's battle numbers,
// the view's battles (`view.battles`) as the map's markers, opening a battle's report or the plan
// of an attack, the combat flash from a battle's summary, the toast of a battle's message with
// its links, and the battles shown as they come: every battle you fight opens the battle screens
// (your attacks, and the attacks on you during the other players' turns, one after the other).
//
// A battle's summary (the view's `battles`, the turn report's `engagements`, the `engage` and
// `move` replies' `battle`): battle_log.js's header lists it.

import * as api from "./api.js";
import { S, dname, info, tname, emit } from "./state.js";
import { h, toast, toastNode } from "./dom.js";
import { render as markup } from "./markup.js";
import { plotIndex } from "./tiles.js";

/** Whether an attack of `units` (ids, the selection: its head decides, as the rules' group test
 *  does) on plot (x, y) is fought as a battle (the rules' `engage_applies`): the units are land
 *  units (one carried on a ship too: an amphibious attack) and the plot is land. Ships against
 *  ships and air units keep the duel. */
export function battleTerrain(units, x, y) {
  if (!S.map) return false;
  const head = S.units?.get(units?.[0]);
  if (!head || tname("DomainType", info("UnitType", head.type)?.domain) !== "DOMAIN_LAND") return false;
  const i = plotIndex(S.map, x, y);
  return i >= 0 && tname("PlotType", S.map.plot_type?.[i]) !== "PLOT_OCEAN";
}

let battleModule = null;
/** battle.js, loaded on first use. */
const battle = () => (battleModule ??= import("./battle.js"));

/** Opens the plan of an attack on plot `to` (`{x, y}`) by `by` (`{army}` or `{units}`). Resolves
 *  when the overlay closes: whether an attack was made. */
export async function openPlan(by, to) {
  const m = await battle();
  return new Promise((resolve) => {
    let attacked = false;
    const src = m.apiSource({ ...by, to });
    const attack = src.attack;
    src.attack = async (r) => {
      const out = await attack(r);
      if (out?.log) attacked = true;
      return out;
    };
    m.openBattlePlan(src, { onClose: () => resolve(attacked) });
  });
}

// ---------------------------------------------------------------------------------------------
// The viewer's battle preferences: per browser, a convenience (losing them brings the defaults
// back). `watch`: the player's own attacks are watched ("watch") or open on their report
// ("report"; the plan's Watch / Report switch); `defended`: the battles fought against the player
// during the other players' turns are watched ("watch"), open on their report ("report"), or only
// listed in the turn report ("list"); `speed`: the replay's speed. Options shows the first two.

const PREFS_KEY = "ffh-battle-prefs-v2";

export const prefs = {
  data: null,
  load() {
    if (this.data) return this.data;
    this.data = {};
    try {
      this.data = JSON.parse(globalThis.localStorage?.getItem(PREFS_KEY) ?? "{}") ?? {};
    } catch {
      /* blocked or broken: the defaults */
    }
    return this.data;
  },
  get(k, def) {
    return this.load()[k] ?? def;
  },
  set(k, v) {
    this.load()[k] = v;
    try {
      globalThis.localStorage?.setItem(PREFS_KEY, JSON.stringify(this.data));
    } catch {
      /* blocked: kept for this page */
    }
  },
};

/** How the battles fought against the player open: "watch", "report" or "list" (not opened). */
export function defendedMode() {
  const v = prefs.get("defended", "watch");
  return ["watch", "report", "list"].includes(v) ? v : "watch";
}

/** Opens battle report `id`: watched in the viewer's watch mode, or (`report`) its report. */
export async function openReport(id, { report = false } = {}) {
  const m = await battle();
  await m.openBattleReport(id, { report });
}

// ---------------------------------------------------------------------------------------------
// Battles shown as they come (the owner's request, 2026-11-03): every battle the player fights
// opens the battle screens: an attack made without the plan (a move, Shift+right-click) when its
// reply comes, watched or straight to its report as the player chose on the plan (its Watch /
// Report switch), and the battles fought against the player during the other players' turns when
// the player's turn comes, oldest first, one after the other, as Options → Battles says (watched,
// on their report, or not opened: only listed in the turn report). The deferred popups wait for
// them.

/** Battle reports this page has shown (or queued), by id: none shows twice. */
let shownBattles = new Set();
let battleQueue = [];
let pumping = false;
/** The battles taken from the queue since it was last empty (the screens' "Battle 2 of 5"). */
let batch = 0;
/** Whether battles are not shown as they come (main.js: while the AI plays the player's slot,
 *  spectating or observing, which would stop at every battle). */
let muted = () => false;

/** Sets when battles are not shown as they come (`f()` true: they are only listed). */
export function muteBattlesWhen(f) {
  muted = f;
}

/** Whether battles are not shown as they come now. */
export function battlesMuted() {
  return !!muted();
}

/** Another game (new or loaded): its battles are all new to this page. */
export function resetShownBattles() {
  shownBattles = new Set();
  battleQueue = [];
  batch = 0;
}

/** The summary of battle `id` (the turn report's or the view's), or null. */
function summaryOf(id) {
  return (S.report?.engagements ?? []).find((b) => b?.id === id) ?? battleById(id);
}

/** Whether battle `id` was fought against the player (its summary says the player defended). */
function defended(id) {
  return (summaryOf(id)?.viewer_side ?? 0) === 1;
}

/** "Skip the rest": the battles waiting count as shown (they stay in the turn report). */
export function skipQueuedBattles() {
  for (const id of battleQueue) shownBattles.add(id);
  battleQueue = [];
}

/** Battle report `id` is on the battle screens already (the plan's own attack). */
export function battleShown(id) {
  if (id !== null && id !== undefined) shownBattles.add(id);
}

/** Of the battle summaries `list` (the turn report's `engagements`, newest first), the ids to show
 *  now: those with a log (older ones keep only their summary), not shown yet, oldest first. */
export function battlesToShow(list, shown = shownBattles) {
  return (list ?? [])
    .filter((b) => b?.has_log !== false && b?.id !== null && b?.id !== undefined && !shown.has(b.id))
    .map((b) => b.id)
    .sort((a, b) => a - b);
}

/** Shows battle reports `ids` on the battle screens, after those already waiting (while muted,
 *  they only count as shown). */
export function showBattles(ids) {
  for (const id of ids ?? []) {
    if (id === null || id === undefined || shownBattles.has(id) || battleQueue.includes(id)) continue;
    // Muted, or a battle fought against the player that Options keeps to the turn report.
    if (battlesMuted() || (defended(id) && defendedMode() === "list")) shownBattles.add(id);
    else battleQueue.push(id);
  }
  pumpBattles();
}

/** Whether battles are on their way to the screens (the popups wait for them). */
export function battlesPending() {
  return pumping || battleQueue.length > 0;
}

/** Opens the waiting battles one at a time: each when the screens are free (another battle there,
 *  the plan's attack, keeps it waiting: `battle-closed` resumes). */
export async function pumpBattles() {
  if (pumping) return;
  pumping = true;
  let handled = false;
  try {
    while (battleQueue.length) {
      if (document.body.classList.contains("battle-open")) break;
      const id = battleQueue.shift();
      if (shownBattles.has(id)) continue;
      shownBattles.add(id);
      handled = true;
      batch++;
      const at = batch;
      let log;
      try {
        log = await api.query("battle_log", { report: id });
      } catch (e) {
        console.warn(`battle #${id}: its log:`, e?.detail ?? e?.message ?? e);
        const b = summaryOf(id);
        const place = b ? (b.place ?? `(${b.x}, ${b.y})`) : `#${id}`;
        toast(`Battle at ${place} could not be loaded. It is listed in the turn report.`, "error");
        continue;
      }
      // The player defended: Options → Battles (the summary may have been missing when it was queued).
      const against = (log?.viewer_side ?? 0) === 1;
      const mode = against ? defendedMode() : prefs.get("watch", "watch");
      if (mode === "list") continue;
      const of = at + battleQueue.length;
      const queue = of > 1 ? { at, of, skip: skipQueuedBattles } : null;
      const m = await battle();
      await new Promise((resolve) => {
        Promise.resolve(m.openBattleLog(log, { report: mode === "report", queue, onClose: resolve })).catch((e) => {
          console.warn(`battle #${id}: the screens:`, e);
          m.closeBattle();
          resolve();
        });
      });
    }
  } finally {
    pumping = false;
    if (!battleQueue.length) batch = 0;
  }
  // The popups and the messages held back for the battles may show now (also when a battle's log
  // could not be read and its screens never opened).
  if (handled && !battleQueue.length) emit("battle-closed");
}

// ---------------------------------------------------------------------------------------------
// Battles of the view

/** How a battle went for the viewer: "won" (its attack took the plot), "repelled" (its attack did
 *  not: the defenders stay until annihilated), "lost" (its plot was taken), "held" (it held). */
export function outcomeFor(b) {
  const me = b.viewer_side ?? 0;
  if (me === 0) return b.plot_taken ? "won" : "repelled";
  return b.plot_taken ? "lost" : "held";
}

/** One line about battle `b` for people: "Prespur: won, the city taken; 0 lost, 3 of theirs". */
export function battleLine(b) {
  const me = b.viewer_side ?? 0;
  const place = b.place ?? `(${b.x}, ${b.y})`;
  const out = { won: b.city_captured !== null && b.city_captured !== undefined ? "won, the city taken" : "won, the plot taken", repelled: "they hold", lost: "lost", held: "held" }[outcomeFor(b)];
  const mine = b.sides?.[me]?.lost ?? 0;
  const theirs = b.sides?.[1 - me]?.lost ?? 0;
  return `${place}: ${out}; ${mine} lost, ${theirs} of theirs`;
}

/** The map's battle markers: one per plot of the view's battles (this game turn's and the last),
 *  the newest there: `{x, y, id, kind, title}`. */
export function battleMarkers() {
  const seen = new Set();
  const out = [];
  for (const b of S.view?.battles ?? []) {
    const k = `${b.x},${b.y}`;
    if (seen.has(k)) continue;
    seen.add(k);
    out.push({ x: b.x, y: b.y, id: b.id, kind: outcomeFor(b), title: `Battle #${b.id}, ${battleLine(b)}`, has_log: b.has_log });
  }
  return out;
}

/** A battle's summary in the view, or null. */
export function battleById(id) {
  return (S.view?.battles ?? []).find((b) => b.id === id) ?? null;
}

/** The combat flash of battle `b` on map view `mapView`: the defended plot ("death" when a defender
 *  fell, else "hit"), and the attacking plot when an attacker fell and no defender did. */
export function flashBattle(mapView, b) {
  if (!mapView || !b) return;
  const att = b.sides?.[0]?.lost ?? 0;
  const def = b.sides?.[1]?.lost ?? 0;
  if (def > 0) mapView.flashTile(b.x, b.y, "death");
  else if (att > 0 && b.from) mapView.flashTile(b.from.x, b.from.y, "death");
  else mapView.flashTile(b.x, b.y, "hit");
}

/** "Watch" and "Report" links for battle report `id` (a message, the turn report's list). */
export function battleLinks(id, { hasLog = true } = {}) {
  if (!hasLog) return h("span.dim.small", { title: "Older battles keep only their summary" }, " (summary only)");
  const go = (report) => (ev) => {
    ev.stopPropagation();
    openReport(id, { report });
  };
  return h(
    "span.battle-links",
    " ",
    h("button.link.small", { "data-battle": id, "data-open": "watch", title: `Watch battle #${id} again`, onclick: go(false) }, "Watch"),
    " · ",
    h("button.link.small", { "data-battle": id, "data-open": "report", title: `The report of battle #${id}`, onclick: go(true) }, "Report"),
  );
}

/** The toast of a game-log message naming a battle (`kind: "engagement"`), with its links. */
export function battleToast(line, ms = 9000) {
  const b = battleById(line.report);
  return toastNode(h("div.toast.log.battle", { role: "status", title: "Click to dismiss" }, markup(line.text), battleLinks(line.report, { hasLog: b?.has_log ?? true })), ms);
}

const ACT = { Strike: "strike", Charge: "charge", Stab: "stab", Shoot: "shot", Spell: "spell", Bombard: "bombard", Heal: "heal" };
const ACTS = { Strike: "strikes", Charge: "charges", Stab: "stabs", Shoot: "shots", Spell: "spells", Bombard: "bombards", Heal: "heals" };
const ROW_NAMES = ["Front", "Middle", "Rear"];

/** One row's actions ("2 strikes"; "—" without any). */
function rowActions(a) {
  const k = typeof a?.kind === "string" ? a.kind[0].toUpperCase() + a.kind.slice(1).toLowerCase() : "";
  return a?.n && ACT[k] ? `${a.n} ${a.n > 1 ? ACTS[k] : ACT[k]}` : "—";
}

/** The row a unit prefers and what it does there, from the view's `engage_row_order` and
 *  `engage_actions` ("Front: 2 strikes"), or "" (no engagements, or a sea or air unit). Each unit
 *  has its own rows and rank preference (no roles). */
export function preferredRow(u) {
  const order = u?.engage_row_order;
  const acts = u?.engage_actions;
  if (!Array.isArray(order) || !order.length || !Array.isArray(acts) || acts.length !== 3) return "";
  const r = order[0];
  return `${ROW_NAMES[r] ?? "-"}: ${rowActions(acts[r])}`;
}

/** A unit's actions in its front, middle and rear rows (the view's `engage_actions`):
 *  "2 strikes / 1 strike / 1 strike" ("—" where it has none), or "". */
export function actionsLine(acts) {
  if (!Array.isArray(acts) || acts.length !== 3) return "";
  return acts.map(rowActions).join(" / ");
}

/** A unit's battle numbers (the view's `engage_*`; docs/UNIT_STATS.md): "ATK 40 · DEF 40 · HP 55/120
 *  · MAG 30 · MDEF 15 · ACC 90 · EVA 0 · CRIT 10 · speed 4 · large" (`hp` / `max_hp` are the health
 *  percent; its battle HP scales it to `engage_max_hp`; MAG only for a caster), then its actions by
 *  row when the view sends them. */
export function engageNumbers(u) {
  const parts = [engageBrief(u)];
  const st = (v) => Math.round((v ?? 0) / 10);
  if (u.engage_magic >= 10) parts.push(`MAG ${st(u.engage_magic)}`);
  if (u.engage_magic_defense !== undefined && u.engage_magic_defense !== null) parts.push(`MDEF ${st(u.engage_magic_defense)}`);
  for (const [k, label] of [["engage_accuracy", "ACC"], ["engage_evasion", "EVA"], ["engage_crit", "CRIT"]]) {
    if (u[k] !== undefined && u[k] !== null) parts.push(`${label} ${u[k]}`);
  }
  if (u.engage_speed !== undefined && u.engage_speed !== null) parts.push(`speed ${u.engage_speed}`);
  if (u.engage_large) parts.push("large");
  const acts = actionsLine(u.engage_actions);
  return `${parts.join(" · ")}${acts ? ` · front / middle / rear: ${acts}` : ""}`;
}

/** The first of a unit's battle numbers: "ATK 40 · DEF 40 · HP 55/120" (the card's line; the rest is
 *  in its tooltip). */
export function engageBrief(u) {
  const st = (v) => Math.round((v ?? 0) / 10);
  const hp = Math.round(((u.hp ?? 0) / Math.max(1, u.max_hp ?? 100)) * u.engage_max_hp);
  return `ATK ${st(u.engage_attack)} · DEF ${st(u.engage_defense)} · HP ${hp}/${u.engage_max_hp}`;
}
