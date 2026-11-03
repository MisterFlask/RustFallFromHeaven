// The rich tooltips of the game's types (a unit, a building, a promotion, a civic, ...): the
// name, a unit's numbers, what it does (the data's `effects`: the server's help texts, ports of
// C++ `CvGameTextMgr`; else its XML help), its strategy text and what a click does.

import { h } from "./dom.js";
import { render as markup } from "./markup.js";
import { S, info, dname } from "./state.js";

/** A unit type's numbers as markup: strength (and defence when it differs), moves, its class of
 *  combat (`UnitType` entry `e` of the data). */
export function unitStats(e) {
  if (!e) return "";
  const parts = [];
  if (e.combat || e.combat_defense) parts.push(e.combat_defense && e.combat_defense !== e.combat ? `${e.combat}/${e.combat_defense}[ICON_STRENGTH]` : `${e.combat}[ICON_STRENGTH]`);
  parts.push(`${e.moves}[ICON_MOVES]`);
  if (e.unit_combat !== null && e.unit_combat !== undefined) parts.push(dname("UnitCombatType", e.unit_combat));
  return parts.join("  ");
}

/** What entry `i` of `kind` does, as markup: its generated `effects`, else its XML help; "". */
export function effectsOf(kind, i) {
  const e = info(kind, i);
  const base = e?.effects || e?.help || "";
  // Engagement combat (docs/ENGAGEMENTS.md): what a unit's or promotion's first strikes,
  // withdrawal, collateral, Blitz and combat limit mean in a battle (the data's
  // `effects_engagement`).
  const battle = e?.effects_engagement ?? null;
  return battle ? (base ? `${base}[NEWLINE]${battle}` : battle) : base;
}

/** The rich tooltip (tips.js) of entry `i` of table `kind`: a function making the content when
 *  the tip shows. `hint`: what a click does; `extra`: nodes after the effects (a city's numbers,
 *  a reason it cannot be had). */
export function infoTip(kind, i, { hint = null, extra = null, title = null } = {}) {
  return () => {
    const e = info(kind, i);
    if (!e) return hint ? h("div.tip-hint", hint) : null;
    const does = effectsOf(kind, i);
    return [
      h("div.tip-title", title ?? dname(kind, i)),
      // A unit's help starts with its combat line (C++ setBasicUnitHelp); without it, the numbers.
      kind === "UnitType" && !e.effects ? h("div.tip-stats", markup(unitStats(e))) : null,
      does ? h("div.tip-effects", markup(does)) : null,
      extra,
      e.strategy ? h("div.tip-strategy", markup(e.strategy)) : null,
      hint ? h("div.tip-hint", hint) : null,
    ];
  };
}
