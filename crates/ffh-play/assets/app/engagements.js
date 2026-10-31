// Engagement combat in the client (docs/ENGAGEMENTS.md; docs/HUMAN.md, "Engagement combat"): what
// the map, the panels and the screens need besides the battle screens themselves (battle.js and
// its modules, loaded on first use): whether the game has engagements, a unit's battle numbers,
// the view's battles (`view.battles`) as the map's markers, opening a battle's report or the plan
// of an attack, the combat flash from a battle's summary, and the toast of a battle's message with
// its links.
//
// A battle's summary (the view's `battles`, the turn report's `engagements`, the `engage` and
// `move` replies' `battle`): battle_log.js's header lists it.

import { S, dname, info, tname } from "./state.js";
import { h, toastNode } from "./dom.js";
import { render as markup } from "./markup.js";
import { plotIndex } from "./tiles.js";

/** Whether this game has engagement combat (the view says so). */
export const engagementsOn = () => !!S.view?.engagements;

/** Whether an attack of `units` (ids, the selection: its head decides, as the rules' group test
 *  does) on plot (x, y) is fought as a battle (the rules' `engage_applies`): the game has
 *  engagement combat, the units are land units (one carried on a ship too: an amphibious attack)
 *  and the plot is land. Ships against ships and air units keep the duel. */
export function battleTerrain(units, x, y) {
  if (!engagementsOn() || !S.map) return false;
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

/** Opens battle report `id`: watched in the viewer's watch mode, or (`report`) its report. */
export async function openReport(id, { report = false } = {}) {
  const m = await battle();
  await m.openBattleReport(id, { report });
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

/** A role's display name ("Frontline") for a unit's `engage_role`. */
export function roleName(i) {
  return i === null || i === undefined ? "" : dname("CombatRoleType", i);
}

const ACT = { Strike: "strike", Charge: "charge", Stab: "stab", Shoot: "shot", Spell: "spell", Bombard: "bombard", Heal: "heal" };
const ACTS = { Strike: "strikes", Charge: "charges", Stab: "stabs", Shoot: "shots", Spell: "spells", Bombard: "bombards", Heal: "heals" };

/** A unit's actions in its front, middle and rear rows (the view's `engage_actions`):
 *  "2 strikes / 1 strike / 1 strike" ("—" where it has none), or "". */
export function actionsLine(acts) {
  if (!Array.isArray(acts) || acts.length !== 3) return "";
  return acts
    .map((a) => {
      const k = typeof a?.kind === "string" ? a.kind[0].toUpperCase() + a.kind.slice(1).toLowerCase() : "";
      return a?.n && ACT[k] ? `${a.n} ${a.n > 1 ? ACTS[k] : ACT[k]}` : "—";
    })
    .join(" / ");
}

/** A unit's battle numbers (the view's `engage_*`): "A 4 · Dv 5 · HP 55/120 · speed 4 · large"
 *  (`hp` / `max_hp` are the health percent; its battle HP scales it to `engage_max_hp`), then its
 *  actions by row when the view sends them. */
export function engageNumbers(u) {
  const x100 = (v) => (v % 100 === 0 ? String(v / 100) : (v / 100).toFixed(1));
  const hp = Math.round(((u.hp ?? 0) / Math.max(1, u.max_hp ?? 100)) * u.engage_max_hp);
  const parts = [`A ${x100(u.engage_attack)}`, `Dv ${x100(u.engage_defense)}`, `HP ${hp}/${u.engage_max_hp}`];
  if (u.engage_speed !== undefined && u.engage_speed !== null) parts.push(`speed ${u.engage_speed}`);
  if (u.engage_large) parts.push("large");
  const acts = actionsLine(u.engage_actions);
  return `${parts.join(" · ")}${acts ? ` · front / middle / rear: ${acts}` : ""}`;
}
