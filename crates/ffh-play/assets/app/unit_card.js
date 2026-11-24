// The Unit Card (the owner's requests, 2026-11-21 and 2026-11-24: a unit's tooltip as a miniature
// character screen, and one standard card for every unit and unit type, easier to read than a wall
// of text). One renderer for both: a unit on the map (unit strip, unit list, the panel's battle
// line: units.js) and a unit type (every `infoTip("UnitType", ..)`: the build list, the tech tree,
// the portraits, an upgrade order). Top to bottom: the header (portrait, name, one line; a unit's
// HP and XP bars), the nine battle numbers as tiles, the attack table (what it does from each row
// of a battle, how often, how sure, how hard), the type's sections (the server's lines, two
// columns on a wide window), its abilities and upgrades, its promotions, its state as chips, the
// caller's extra lines, the strategy text and what a click does.
//
// Mechanical text, bare (CLAUDE.md, "Rules text and tooltips"). Keywords are words in the keyword
// style; their reminders go to the tooltip's side boxes (keywords.js `remindersNode`, tips.js),
// never into the card. The numbers come from engagements.js (`engageStats`, `cardRows`,
// `rowPlan`, `battleHp`, `squadLine`, `keywordReminders`): the client does no rules arithmetic.

import { h } from "./dom.js";
import { S, info, dname, moves, me } from "./state.js";
import { render as markup } from "./markup.js";
import { promotionIcon } from "./look.js";
import { unitFigure } from "./isoui.js";
import { remindersNode } from "./keywords.js";
import { engageStats, battleHp, rowPlan, cardRows, squadLine, keywordReminders } from "./engagements.js";
import { setUnitTypeCard } from "./infotips.js";

/** A bar with its numbers: `label`, the fill (`num` of `den`), "num/den" after it. */
function bar(kind, label, num, den, text = `${num}/${den}`) {
  const f = den > 0 ? Math.max(0, Math.min(1, num / den)) : 0;
  return h(
    `div.uc-bar.${kind}`,
    h("span.uc-bar-k", label),
    h("span.uc-bar-track", h("span.uc-bar-fill", { style: { width: `${Math.round(100 * f)}%` } })),
    h("span.uc-bar-v", text),
  );
}

const has = (v) => v !== undefined && v !== null;

/** A ship's strength as the sub line says it: "Strength 4", "Strength 4/3". */
function strengthOf(e) {
  if (!e?.strength && !e?.strength_defense) return null;
  return e.strength_defense && e.strength_defense !== e.strength ? `Strength ${e.strength}/${e.strength_defense}` : `Strength ${e.strength}`;
}

/** The card's header: the portrait, the name and its line; a unit's HP and XP bars. `unit`: a unit
 *  on the map (else a type: `t` its `UnitType` entry). */
function header({ unit, type, t, name, figure, owner }) {
  const combat = has(t?.unit_combat) && t.unit_combat >= 0 ? dname("UnitCombatType", t.unit_combat) : null;
  let sub;
  let bars = [];
  if (unit) {
    sub = [
      has(unit.level) ? `Level ${unit.level}` : null,
      has(unit.moves) ? `${moves(unit.moves)}/${moves(unit.max_moves)} moves` : null,
      combat,
      has(unit.strength) ? `strength ${unit.strength}` : null,
    ];
    const bhp = battleHp(unit);
    const pct = Math.round((100 * (unit.hp ?? 0)) / Math.max(1, unit.max_hp ?? 100));
    const low = ((unit.hp ?? 0) / Math.max(1, unit.max_hp ?? 100)) * 4 < 1 ? ".low" : "";
    bars = [
      bhp ? bar(`hp${low}`, "HP", bhp.hp, bhp.max) : bar(`hp${low}`, "HP", pct, 100, `${pct}%`),
      has(unit.xp) && unit.xp_needed ? bar("xp", "XP", unit.xp, unit.xp_needed) : null,
    ];
  } else {
    const m = t?.moves;
    // A type without a battle sheet (a ship, an air unit): its old numbers.
    const atk = !t?.battle && !strengthOf(t) && (t?.attack || t?.defense) ? `ATK ${t.attack} · DEF ${t.defense}` : null;
    sub = [has(t?.tier) ? `Tier ${t.tier}` : null, combat, has(m) ? `${m} move${m === 1 ? "" : "s"}` : null, strengthOf(t), atk];
  }
  sub = sub.filter(Boolean);
  return h(
    "div.uc-head",
    h("div.uc-portrait", figure ?? unitFigure(type, owner, 56) ?? h("span.uc-abbr", name.slice(0, 2))),
    h("div.uc-id", h("div.uc-name", name), sub.length ? h("div.uc-sub", sub.join(" · ")) : null, bars),
  );
}

/** The nine battle numbers, one row on a wide card; a number the unit does not use greyed "—". */
function statTiles(u) {
  const stats = engageStats(u);
  if (!stats) return null;
  return h(
    "div.uc-stats",
    stats.map((s) => h(`div.uc-stat${s.off ? ".off" : ""}`, { "data-stat": s.key }, h("span.uc-stat-k", s.label), h("span.uc-stat-v", s.value))),
  );
}

/** The attack table (engagements.js `cardRows`): a line a row, the best row gilt, the row a squad
 *  places it in outlined; the warning and the notes under it. */
function attackTable(m) {
  const head = (text, cls = "") => h(`div.uc-th${cls}`, text);
  const refs = m.refs.length ? m.refs : ["", "", ""];
  const cells = [
    // Two header lines: what the damage is against, then the column names and the references.
    h("div.uc-th", { style: { gridColumn: "1 / span 5" } }),
    h("div.uc-th.uc-th-dmg", { style: { gridColumn: `span ${refs.length}` } }, `Damage ${m.dmgHead}`),
    head("Row"),
    head("Attack"),
    head("Per round", ".uc-n"),
    head("Hit", ".uc-n"),
    head("Crit", ".uc-n"),
    refs.map((r) => head(r, ".uc-n.uc-ref")),
  ];
  for (const r of m.rows) {
    const cls = `${r.none ? ".none" : ""}${r.best ? ".best" : ""}${r.here ? ".here" : ""}`;
    const td = (content, extra = "") => h(`div.uc-td${extra}${cls}`, content);
    cells.push(
      td([r.name, r.best ? h("span.uc-mark.uc-best", "best") : null, r.here ? h("span.uc-mark.uc-row-here", "stands here") : null], ".uc-rn"),
      td([r.attack, ...r.tags.flatMap((t) => [h("span.uc-sep", " · "), h("span.kw", { "data-keyword": t.key }, t.word)])], ".uc-atk"),
      td(r.per, ".uc-n"),
      td(r.hit, ".uc-n"),
      td(r.crit, ".uc-n"),
      r.dmg ? r.dmg.map((v) => td(v, ".uc-n.uc-dmg")) : h(`div.uc-td.uc-dmg.uc-say${cls}`, { style: { gridColumn: `span ${refs.length}` } }, r.damage),
    );
  }
  return [
    h("div.uc-label", m.label),
    h("div.uc-table", { style: { gridTemplateColumns: `auto minmax(0, 1fr) repeat(3, auto) repeat(${refs.length}, auto)` } }, cells),
    m.warn ? h("div.uc-warn", m.warn) : null,
    m.notes.length ? h("div.uc-notes", m.notes.join(" ")) : null,
  ];
}

/** Front, Middle, Rear as three boxes from the view's `engage_actions` (an older server, without
 *  `engage_rows`): the best row gilt, the row it stands in outlined, a line when it does less. */
function rowBoxes(u) {
  const plan = rowPlan(u);
  if (!plan) return null;
  const boxes = plan.rows.map((r, i) =>
    h(
      `div.uc-row${i === plan.best ? ".best" : ""}${i === plan.here ? ".here" : ""}`,
      h("div.uc-row-k", r.name, i === plan.best ? h("span.uc-best", " · best") : null),
      h(`div.uc-row-v${r.n ? "" : ".none"}`, r.actions),
      i === plan.here ? h("div.uc-row-here", "stands here") : null,
    ),
  );
  return [h("div.uc-label", "Attacks by row"), h("div.uc-rows", boxes), plan.warn ? h("div.uc-warn", plan.warn) : null];
}

/** The battle part: the attack table, else (no `engage_rows`) the three row boxes. */
function battlePart(u) {
  const m = cardRows(u, S.data?.defines ?? {});
  return m ? attackTable(m) : rowBoxes(u);
}

/** The type's sections (`[{key, title, lines}]`), each its label and its lines (markup), then
 *  the blocks `more` (abilities, upgrades, promotions); `keys` limits the sections (a unit on the
 *  map: its battle section only), `titles` renames some (by key). Two columns on a wide window, a
 *  block never split (the browser balances them: the lists fill the shorter column). */
function sectionsPart(sections, keys = null, more = [], titles = {}) {
  const list = (sections ?? []).filter((s) => s?.lines?.length && (!keys || keys.includes(s.key)));
  const blocks = [...list.map((s) => h("div.uc-sec", { "data-section": s.key }, h("div.uc-label", titles[s.key] ?? s.title), s.lines.map((l) => h("div.uc-line", markup(l))))), ...more.filter(Boolean)];
  if (!blocks.length) return null;
  return h(`div.uc-sections${blocks.length > 1 ? ".multi" : ""}`, blocks);
}

/** A labelled block of names as chips (`key` its data-section). */
function namedChips(key, label, names) {
  if (!names.length) return null;
  return h("div.uc-sec.uc-list", { "data-section": key }, h("div.uc-label", label), h("div.uc-chips-line", names.map((n) => h("span.uc-chip", n))));
}

/** A labelled block of promotions (a unit's own under "Promotions", a type's free ones under
 *  "Starts with"): each its seal and its name. */
function promotionBlock(key, label, list) {
  if (!list?.length) return null;
  return h(
    "div.uc-sec.uc-list",
    { "data-section": key },
    h("div.uc-label", label),
    h("div.uc-chips-line", list.map((p) => h("span.uc-chip.uc-promo-tag", promotionIcon(p, { u: 1, title: dname("PromotionType", p) }), dname("PromotionType", p)))),
  );
}

/** The card of unit `u` (a unit of the view, with the `group` query's details when it has them):
 *  `chips` its state (army, status, ... as badges from units.js), `hint` what a click does,
 *  `figure` its portrait (else its map figure). Returns `[card, reminder boxes]` (tips.js moves the
 *  boxes beside the card): its battle keywords (`engage_keywords`) and its own `reminders`. */
export function unitCard(u, { chips = [], hint = null, figure = null } = {}) {
  const t = info("UnitType", u.type);
  const reminders = Array.isArray(u.engage_keywords) ? keywordReminders(u.engage_keywords, u.reminders) : (u.reminders ?? []);
  const kwLarge = reminders.some((r) => /LARGE$/.test(r.key ?? ""));
  const squad = squadLine(u);
  const allChips = [squad ? h("span.badge.squad", squad) : null, ...chips, u.engage_large && !kwLarge ? h("span.badge", "Large") : null].filter(Boolean);
  return [
    h(
      "div.unit-card",
      header({ unit: u, type: u.type, t, name: u.name ?? dname("UnitType", u.type), figure, owner: u.owner }),
      statTiles(u),
      battlePart(u),
      // The battle lines are its type's (the tiles and the table above are the unit's own, its
      // promotions counted): the label says so, and its promotions stand beside them.
      sectionsPart(t?.sections, ["battle"], [promotionBlock("promotions", "Promotions", u.promotions)], { battle: "Its type in battle" }),
      allChips.length ? h("div.uc-chips", allChips) : null,
      hint ? h("div.uc-hint", hint) : null,
    ),
    remindersNode(reminders),
  ];
}

/** The card of unit type `i` (every unit type's tooltip: infotips.js `infoTip`): `title` in place
 *  of its name, `extra` nodes after its chips (the build list's House lines), `hint` what a click
 *  does, `effects` the markup shown when the data has no `sections` (an older server). Returns
 *  `[card, reminder boxes]`; null for an unknown type. */
export function unitTypeCard(i, { hint = null, extra = null, title = null, effects = "" } = {}) {
  const t = info("UnitType", i);
  if (!t) return null;
  const sheet = t.battle ? { type: i, ...t.battle } : null;
  const name = title ?? dname("UnitType", i);
  const more = [
    namedChips("abilities", "Abilities", (t.abilities ?? []).map((s) => dname("SpellType", s))),
    namedChips("upgrades_to", "Upgrades to", (t.upgrades_to ?? []).map((u) => dname("UnitType", u))),
    promotionBlock("starts_with", "Starts with", t.free_promotions),
  ];
  // An older server: its `effects` as one block.
  const old = !t.sections && effects ? h("div.uc-sec", h("div.uc-line", markup(effects))) : null;
  const sections = sectionsPart(t.sections, null, [old, ...more]);
  return [
    h(
      "div.unit-card.type",
      header({ unit: null, type: i, t, name, figure: null, owner: me()?.player ?? null }),
      sheet ? statTiles(sheet) : null,
      sheet ? battlePart(sheet) : null,
      sections,
      [extra].flat(Infinity).some(Boolean) ? h("div.uc-extra", extra) : null,
      t.strategy ? h("div.uc-strategy", markup(t.strategy)) : null,
      hint ? h("div.uc-hint", hint) : null,
    ),
    remindersNode(t.reminders),
  ];
}

// Every unit type's tooltip (infotips.js `infoTip`) is this card.
setUnitTypeCard(unitTypeCard);
