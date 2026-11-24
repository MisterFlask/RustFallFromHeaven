// The unit card (the owner's request, 2026-11-21: the unit tooltip as a miniature character
// screen): one unit's portrait, name, level, moves and strength, its battle HP and experience as
// bars, its eight battle numbers as tiles, what it does from each row of a battle, its
// promotions, its state as chips, and what a click does. Drawn as the tooltip of a unit in the
// unit strip, the unit list and the panel's battle line (units.js).
//
// Mechanical text, bare (CLAUDE.md, "Rules text and tooltips"). Keywords are chips; their reminders
// go to the tooltip's side boxes (keywords.js `remindersNode`, tips.js), never into the card. The
// numbers come from engagements.js (`engageStats`, `battleHp`, `rowPlan`, `squadLine`): the client
// does no rules arithmetic of its own.

import { h } from "./dom.js";
import { dname, moves } from "./state.js";
import { promotionIcon } from "./look.js";
import { isoSkin, unitFigure } from "./isoui.js";
import { remindersNode } from "./keywords.js";
import { engageStats, battleHp, rowPlan, squadLine } from "./engagements.js";

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

/** The card's header: the portrait, the name, `Level N · M/M moves · strength S`, the HP and XP
 *  bars. */
function header(u, figure) {
  const name = u.name ?? dname("UnitType", u.type);
  const sub = [
    u.level !== undefined && u.level !== null ? `Level ${u.level}` : null,
    u.moves !== undefined && u.moves !== null ? `${moves(u.moves)}/${moves(u.max_moves)} moves` : null,
    u.strength !== undefined && u.strength !== null ? `strength ${u.strength}` : null,
  ].filter(Boolean);
  const bhp = battleHp(u);
  const pct = Math.round((100 * (u.hp ?? 0)) / Math.max(1, u.max_hp ?? 100));
  const frac = (u.hp ?? 0) / Math.max(1, u.max_hp ?? 100);
  const hp = bhp ? bar(`hp${frac * 4 < 1 ? ".low" : ""}`, "HP", bhp.hp, bhp.max) : bar(`hp${frac * 4 < 1 ? ".low" : ""}`, "HP", pct, 100, `${pct}%`);
  const xp = u.xp !== undefined && u.xp !== null && u.xp_needed ? bar("xp", "XP", u.xp, u.xp_needed) : null;
  return h(
    "div.uc-head",
    h("div.uc-portrait", figure ?? unitFigure(u.type, u.owner, 56) ?? h("span.uc-abbr", name.slice(0, 2))),
    h("div.uc-id", h("div.uc-name", name), sub.length ? h("div.uc-sub", sub.join(" · ")) : null, hp, xp),
  );
}

/** The eight battle numbers, two rows of four; a number the unit does not use greyed with "—". */
function statGrid(u) {
  const stats = engageStats(u);
  if (!stats) return null;
  return h(
    "div.uc-stats",
    stats.map((s) => h(`div.uc-stat${s.off ? ".off" : ""}`, { "data-stat": s.key }, h("span.uc-stat-k", s.label), h("span.uc-stat-v", s.value))),
  );
}

/** Front, Middle, Rear: the actions from each row; the best row gilt, the row it stands in (in a
 *  squad) outlined, and a line when that row does less than the best. */
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
  return [h("div.uc-label", "In battle"), h("div.uc-rows", boxes), plan.warn ? h("div.uc-warn", plan.warn) : null];
}

/** The unit's promotions as the unit panel draws them (their seals), or their names without art. */
function promotionRow(u) {
  const list = u.promotions ?? [];
  if (!list.length) return null;
  return h(
    "div.uc-promos",
    list.map((p) => promotionIcon(p, { u: isoSkin() ? 2 : 1, title: dname("PromotionType", p) }) ?? h("span.uc-promo-name", dname("PromotionType", p))),
  );
}

/** The card of unit `u` (a unit of the view, with the `group` query's details when it has them):
 *  `chips` its state (army, status, ... as badges from units.js), `hint` what a click does,
 *  `figure` its portrait (else its map figure). Its keywords' reminders (`u.reminders`) go beside
 *  it (tips.js side boxes). */
export function unitCard(u, { chips = [], hint = null, figure = null } = {}) {
  const reminders = u.reminders ?? [];
  const kwLarge = reminders.some((r) => /LARGE$/.test(r.key ?? ""));
  const squad = squadLine(u);
  const allChips = [
    squad ? h("span.badge.squad", squad) : null,
    ...chips,
    u.engage_large && !kwLarge ? h("span.badge", "Large") : null,
    ...reminders.map((r) => h("span.kw-chip", { "data-keyword": r.key }, r.name)),
  ].filter(Boolean);
  return [
    h(
      "div.unit-card",
      header(u, figure),
      statGrid(u),
      rowBoxes(u),
      promotionRow(u),
      allChips.length ? h("div.uc-chips", allChips) : null,
      hint ? h("div.uc-hint", hint) : null,
    ),
    remindersNode(reminders),
  ];
}
