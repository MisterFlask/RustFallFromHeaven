// The rich tooltips of the game's types (a unit, a building, a promotion, a civic, ...): the
// name, a unit's numbers, what it does (the data's `effects`: the server's help texts, ports of
// C++ `CvGameTextMgr`; else its XML help), its strategy text and what a click does. Also the
// tooltips of a number by its parts (a city's mood and health, the gold rate, an attitude, the
// combat modifiers): one part a line, the numbers in a column (`partsList`).

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

/** A number with its sign, a true minus for a loss: "+3", "−2", "0". */
function signedNum(n) {
  return n > 0 ? `+${n}` : n < 0 ? `−${-n}` : "0";
}

/** A number by its parts, as a tooltip's list: one part a line, its number right-aligned in a
 *  column before its label. Each group of `groups` is a head (`icon` node, `head`, `total` at the
 *  right) over its `rows` `[label, n]` (n 0 or null: a line without a number); a part counts for
 *  the reader when `n * sense` > 0 (green) and against when < 0 (red), `sense` 0 leaves it plain.
 *  `unit` follows each number ("%"); `sign` false shows counts without a "+". Groups without rows
 *  are left out; null when none has any. */
export function partsList(groups, { unit = "", sign = true } = {}) {
  const cells = [];
  for (const g of groups) {
    if (!g?.rows?.length) continue;
    if (g.head) cells.push(h("div.pl-head", g.icon ?? null, h("span", g.head), g.total !== null && g.total !== undefined ? h("span.pl-total", String(g.total)) : null));
    for (const [label, n] of g.rows) {
      const s = (n ?? 0) * (g.sense ?? 1);
      cells.push(h(`span.pl-n${s > 0 ? ".good" : s < 0 ? ".warn" : ""}`, n ? `${sign ? signedNum(n) : n}${unit}` : ""), h("span.pl-label", label));
    }
  }
  return cells.length ? h(`div.parts-list${groups.some((g) => g?.head) ? ".grouped" : ""}`, cells) : null;
}

/** What room city `c`'s mood (`happy`: the `city` query's `happiness`) or health leaves before
 *  trouble, in words: how many more citizens it takes, or what the excess costs. */
export function roomText(c, happy) {
  const room = happy ? c.happiness.happy - c.happiness.unhappy : c.health.good - c.health.bad;
  const s = Math.abs(room) > 1 ? "s" : "";
  if (happy) return room > 0 ? `Room for ${room} more citizen${s} before unrest` : room === 0 ? "At the limit: the next citizen will be unhappy and angry" : `${-room} angry citizen${s}: they do not work`;
  return room > 0 ? `Room for ${room} more citizen${s} before sickness costs food` : room === 0 ? "At the limit: the next citizen costs a food a turn" : `Sickness: −${-room} food a turn`;
}

/** The tooltip of city `c`'s mood (`happy`) or health (the `city` query): its happy and unhappy
 *  (healthy and unhealthy) citizens by their causes, each list summing to its count, then the
 *  room left and `notes`; `icon(name)` makes an interface icon ("happy", "unhealthy", ...). */
export function moodTip(c, happy, icon, notes = []) {
  const groups = happy
    ? [
        { head: "Happy", icon: icon("happy"), total: c.happiness.happy, rows: c.happiness.happy_parts },
        { head: "Unhappy", icon: icon("unhappy"), total: c.happiness.unhappy, rows: c.happiness.unhappy_parts, sense: -1 },
      ]
    : [
        { head: "Healthy", icon: icon("health"), total: c.health.good, rows: c.health.good_parts },
        { head: "Unhealthy", icon: icon("unhealthy"), total: c.health.bad, rows: c.health.bad_parts, sense: -1 },
      ];
  return [
    h("div.tip-title", happy ? "Mood" : "Health"),
    partsList(groups) ?? h("div.dim", "Nothing either way."),
    h("div.tip-hint", roomText(c, happy)),
    notes.filter(Boolean).map((n) => h("div.tip-hint", n)),
  ];
}
