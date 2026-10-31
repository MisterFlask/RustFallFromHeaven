// Engagement combat in the client (docs/ENGAGEMENTS.md; docs/HUMAN.md, "Engagement combat"): what
// the map, the panels and the screens need besides the battle screens themselves (battle.js and
// its modules, loaded on first use): whether the game has engagements, the stances and their help
// and a selector for them, the view's battles (`view.battles`) as the map's markers, opening a
// battle's report or the plan of an attack, the combat flash from a battle's summary, and the
// toast of a battle's message with its links.
//
// A battle's summary (the view's `battles`, the turn report's `engagements`, the `engage` and
// `move` replies' `battle`): battle_log.js's header lists it.

import { S, dname, info } from "./state.js";
import { h, toastNode } from "./dom.js";
import { render as markup, plain } from "./markup.js";

/** Whether this game has engagement combat (the view says so). */
export const engagementsOn = () => !!S.view?.engagements;

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
// Stances

/** The stances, in the data's order: `[{i, type, name, help}]`. */
export function stances() {
  return (S.data?.tables?.StanceType ?? []).map((e, i) => ({ i, type: e.type, name: dname("StanceType", i), help: e.help ?? "" }));
}

/** Stance `i`'s help as plain text (the selector's tooltip). */
export function stanceHelp(i) {
  const help = info("StanceType", i)?.help ?? "";
  return plain(help);
}

/** A stance selector: `current` (an index) selected, `onChange(i)` on a change; the stance's help
 *  under it (`help`: false leaves it out) and as the tooltip of each option. `what`: what the
 *  stance is of ("army", "garrison"), for the label. */
export function stanceSelect(current, onChange, { what = "army", help = true, disabled = false } = {}) {
  const list = stances();
  const gloss = h("div.stance-help.dim.small", list[current]?.help ? markup(list[current].help) : "");
  const sel = h(
    "select.stance-select",
    {
      "aria-label": `${what === "garrison" ? "Garrison" : "Army"} stance`,
      title: `The ${what}'s stance: which of its units fight when it defends${what === "army" ? " and which it proposes when it attacks" : ""} (engagement combat)`,
      disabled,
      "data-stance": current,
      onchange: (ev) => {
        const i = Number(ev.target.value);
        gloss.replaceChildren(...(list[i]?.help ? [markup(list[i].help)] : []));
        onChange(i);
      },
    },
    list.map((s) => h("option", { value: s.i, selected: s.i === current, title: stanceHelp(s.i) }, s.name)),
  );
  return h(`div.stance-row`, h("label", h("span", "Stance "), sel), help ? gloss : null);
}

// ---------------------------------------------------------------------------------------------
// Battles of the view

/** How a battle went for the viewer: "won" (its attack took the plot), "lost" (it was taken), "held"
 *  (it held, or its attack was repelled with the enemy losing more), "repelled", "draw". */
export function outcomeFor(b) {
  const me = b.viewer_side ?? 0;
  if (b.result === "Draw" || b.result === "Mutual") return "draw";
  const attackerWon = b.result === "AttackerWon";
  if (me === 0) return attackerWon ? "won" : "repelled";
  return attackerWon ? "lost" : "held";
}

/** One line about battle `b` for people: "Prespur: won, the city taken; 0 lost, 3 of theirs". */
export function battleLine(b) {
  const me = b.viewer_side ?? 0;
  const place = b.place ?? `(${b.x}, ${b.y})`;
  const out = { won: b.city_captured !== null && b.city_captured !== undefined ? "won, the city taken" : "won", repelled: "repelled", lost: "lost", held: "held", draw: "a draw" }[outcomeFor(b)];
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
