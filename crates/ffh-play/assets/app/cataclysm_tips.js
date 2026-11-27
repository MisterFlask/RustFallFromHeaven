// Cataclysm tooltips: separates "what this does / live state" (the main box, with game nouns in
// amber `.kw` markup) from "what those game nouns do" (separate `.reminder` cards via
// `keywords.js` `remindersNode` and `tips.js` `.tip-side`, matching docs/PLAY_DESIGN.md §4.8).
//
// The server sends the canonical keyword dictionary in `S.view.cataclysm.keywords` (built from
// `cataclysm-core` constants) with transitive `with` expansion; this module scans `[KW=CAT_...]`
// marks plus explicit unit/action keys and renders every referenced reminder card once, in order
// of first mention.

import { h } from "./dom.js";
import { render as markup, tokenize } from "./markup.js";
import { remindersNode } from "./keywords.js";

const ROW_NAMES = ["Front", "Mid", "Rear"];

const ACTION_KW = {
  Strike: "CAT_STRIKE",
  Charge: "CAT_CHARGE",
  Stab: "CAT_STAB",
  Shoot: "CAT_SHOOT",
  Spell: "CAT_SPELL",
  Bombard: "CAT_BOMBARD",
  Heal: "CAT_HEAL",
};

const PATTERN_KW = {
  Volley: "CAT_VOLLEY",
  Pierce: "CAT_PIERCE",
  Sweep: "CAT_SWEEP",
  Cleave: "CAT_CLEAVE",
  Trample: "CAT_TRAMPLE",
};

export function actionKeywordKey(kind) {
  return ACTION_KW[kind] ?? null;
}

export function patternKeywordKey(pattern) {
  return PATTERN_KW[pattern] ?? null;
}

/** Collects all `CAT_*` keyword keys from a unit class or squad member (traits + 3 rows' actions/patterns). */
export function unitActionKeywords(u) {
  if (!u) return [];
  const keys = [];
  if (u.is_hero) keys.push("CAT_HERO");
  if (u.wounds) keys.push("CAT_WOUNDS");
  if (u.large) keys.push("CAT_LARGE");
  if (u.anti_flank) keys.push("CAT_ANTI_FLANK");
  if (u.armor_piercing) keys.push("CAT_ARMOR_PIERCING");
  if ((u.damage_vs_walls ?? 0) > 0) keys.push("CAT_SENTINELS");
  for (const a of u.actions ?? []) {
    if (!a || (a.n ?? 0) <= 0) continue;
    const ak = actionKeywordKey(a.kind);
    if (ak) keys.push(ak);
    const pk = patternKeywordKey(a.pattern);
    if (pk) keys.push(pk);
  }
  return keys;
}

/** Resolves all referenced Cataclysm reminders from markup `texts` and explicit `extraKeys` against
 *  server dictionary `dict` (`S.view.cataclysm.keywords`), expanding each keyword's transitive
 *  `with` list in order of first mention with no duplicates. */
export function extractCatReminders(texts, extraKeys = [], dict = null) {
  if (!dict) return [];
  const rawKeys = [];
  for (const t of texts ?? []) {
    if (typeof t !== "string" || !t) continue;
    for (const tok of tokenize(t)) {
      if (tok.t === "kw" && tok.key) rawKeys.push(tok.key);
    }
  }
  for (const k of extraKeys ?? []) {
    if (k) rawKeys.push(k);
  }
  const seen = new Set();
  const out = [];
  const push = (k) => {
    if (seen.has(k)) return;
    const entry = dict[k];
    if (!entry) return;
    seen.add(k);
    out.push(entry);
  };
  for (const k of rawKeys) {
    const entry = dict[k];
    if (!entry) continue;
    const list = entry.with?.length ? entry.with : [k];
    for (const w of list) push(w);
  }
  return out;
}

/** Why `cls` cannot currently be reinforced into squad `sq` (null if allowed). Mirrors
 *  `cmd_reinforce_squad` in `crates/cataclysm-core/src/game.rs`. */
export function reinforceWhy(data, sq, cls) {
  if (!sq || !cls) return "Choose a unit class.";
  if (sq.in_home_borders === false) return sq.reinforce_border_why ?? "The squad must stand inside your borders to reinforce.";
  const free = (sq.place_cap ?? 0) - (sq.places_used ?? 0);
  const need = cls.large ? 2 : 1;
  if (need > free) return `Needs ${need} free place${need === 1 ? "" : "s"} (${sq.places_used}/${sq.place_cap}).`;
  if (cls.large && sq.can_fit_2cell === false) return "The squad's 3x3 grid has no 2 adjacent free cells for a Large unit.";
  if (!cls.large && sq.can_fit_1cell === false) return "The squad's 3x3 grid has no free cell for this unit.";
  const gold = data?.currencies?.gold;
  if (typeof gold === "number" && typeof cls.cost_gold === "number" && gold < cls.cost_gold) {
    return `Need ${cls.cost_gold} Gold (have ${gold}).`;
  }
  return null;
}

function kwSpan(key, word) {
  return key ? h("span.kw", { "data-keyword": key }, word) : word;
}

/** Compact stat strip + 3-row attack table for a Cataclysm unit class or squad member. */
export function unitProfileNodes(u, { currentRow = null, hpLabel = null } = {}) {
  if (!u) return [];
  const out = [];

  // Meta & trait tags
  const meta = [];
  if (u.tier !== undefined && u.tier !== null) meta.push(`Tier ${u.tier}`);
  if (u.large) meta.push([kwSpan("CAT_LARGE", "Large"), " (2 places)"]);
  else if (currentRow === null) meta.push("1 place");
  if (u.moves) meta.push(`${u.moves} moves`);
  if (u.is_hero) meta.push(kwSpan("CAT_HERO", "Hero"));
  if (u.anti_flank) meta.push(kwSpan("CAT_ANTI_FLANK", "Anti-Flank"));
  if (u.armor_piercing) meta.push(kwSpan("CAT_ARMOR_PIERCING", "Armor-Piercing"));
  if ((u.damage_vs_walls ?? 0) > 0) meta.push([`+${u.damage_vs_walls}% vs `, kwSpan("CAT_SENTINELS", "Sentinels")]);

  if (meta.length) {
    const joined = [];
    meta.forEach((m, i) => {
      if (i > 0) joined.push(" · ");
      if (Array.isArray(m)) joined.push(...m);
      else joined.push(m);
    });
    out.push(h("div.cat-tip-meta", ...joined));
  }

  if (u.wounds) {
    out.push(h("div.warn", kwSpan("CAT_WOUNDS", "Wounds"), ` ${u.wounds}`));
  }

  // Stat tiles
  const stats = [
    ["HP", hpLabel ?? (u.hp !== undefined ? String(u.hp) : `${u.hp_pct ?? 100}%`)],
    ["ATK", String(u.atk ?? 0)],
    ["DEF", String(u.def ?? 0)],
    (u.magic ?? 0) > 0 ? ["MAG", String(u.magic)] : null,
    u.mdef !== undefined ? ["MDEF", String(u.mdef)] : null,
    u.armor !== undefined ? ["ARM", String(u.armor)] : null,
    u.speed !== undefined ? ["SPD", String(u.speed)] : null,
    u.crit !== undefined ? ["CRIT", `${u.crit}%`] : null,
  ].filter(Boolean);

  if (stats.length) {
    out.push(
      h(
        "div.cat-tip-stats",
        ...stats.map(([k, v]) => h("div.cat-tip-stat", h("span.cat-tip-stat-k", k), h("span.cat-tip-stat-v", v))),
      ),
    );
  }

  // 3-row battle actions table
  const actions = u.actions ?? [];
  if (actions.length) {
    const bestRow = u.best_row;
    const rows = actions.map((a, r) => {
      const isBest = bestRow === r;
      const isHere = currentRow === r;
      const cls = `.cat-tip-row${isBest ? ".best" : ""}${isHere ? ".here" : ""}${!a || a.n <= 0 ? ".none" : ""}`;
      const actNodes = [];
      if (a && a.n > 0) {
        actNodes.push(kwSpan(actionKeywordKey(a.kind), a.kind), ` ×${a.n}`);
        if (a.pattern && a.pattern !== "Single") {
          actNodes.push(" · ", kwSpan(patternKeywordKey(a.pattern), a.pattern));
        }
      } else {
        actNodes.push(h("span.dim", "No action"));
      }
      return h(
        `div${cls}`,
        h(
          "span.cat-tip-row-name",
          ROW_NAMES[r] ?? `Row ${r + 1}`,
          isBest ? h("span.cat-tip-badge", "best") : null,
          isHere ? h("span.cat-tip-here", "here") : null,
        ),
        h("span.cat-tip-row-act", ...actNodes),
      );
    });
    out.push(h("div.cat-tip-rows", ...rows));
  }

  return out;
}

/** Builds a Cataclysm tooltip: main charcoal box with `[KW=...]` markup + separate side reminder
 *  cards for all referenced game nouns (and their transitive `with` cards). */
export function catTip({ title = null, right = null, lines = [], extra = [], hint = null, why = null, keywords = [], dict = null } = {}) {
  const textList = (Array.isArray(lines) ? lines : [lines]).filter((l) => l !== null && l !== undefined && l !== "");
  const lineTexts = textList.map((l) => (typeof l === "string" ? l : typeof l === "object" && typeof l.text === "string" ? l.text : ""));
  const allTexts = [...lineTexts, ...(typeof hint === "string" ? [hint] : []), ...(typeof why === "string" ? [why] : [])];
  const reminders = extractCatReminders(allTexts, keywords, dict);
  const nodes = [];
  if (title) {
    nodes.push(right ? h("div.tip-title.cat-tip-head", h("span", title), h("span.cat-tip-right", right)) : h("div.tip-title", title));
  }
  for (const l of textList) {
    if (l instanceof Node) nodes.push(l);
    else if (typeof l === "object" && l.text) nodes.push(h(`div${l.cls ?? ""}`, markup(l.text)));
    else nodes.push(h("div", markup(String(l))));
  }
  for (const e of [extra].flat(Infinity)) {
    if (e) nodes.push(e);
  }
  if (why) nodes.push(h("div.warn", markup(String(why))));
  if (hint) nodes.push(h("div.tip-hint", markup(String(hint))));
  const remNode = remindersNode(reminders);
  if (remNode) nodes.push(remNode);
  return nodes.length ? nodes : null;
}

/** Tooltip for Reinforce (both the unit-class dropdown and the Reinforce button, enabled or disabled). */
export function reinforceTip(data, sq, cls) {
  const dict = data?.keywords ?? null;
  const why = reinforceWhy(data, sq, cls);
  if (!cls) {
    return catTip({
      title: "Reinforce",
      lines: [data?.order_tips?.reinforce ?? "Adds a unit to an open cell of this squad's [KW=CAT_FORMATION]Formation[\\KW] for Gold."],
      why,
      dict,
    });
  }
  const upkeep = cls.upkeep ?? 2;
  const actionLine = cls.large
    ? `Adds a ${cls.name} to 2 side-by-side cells of this squad's [KW=CAT_FORMATION]Formation[\\KW] (instant; uses no movement; inside your borders only). Upkeep: ${upkeep} Gold/t.`
    : `Adds a ${cls.name} to an open cell of this squad's [KW=CAT_FORMATION]Formation[\\KW] (instant; uses no movement; inside your borders only). Upkeep: ${upkeep} Gold/t.`;
  const hintParts = [];
  if (cls.reinforce_discount && cls.base_cost_gold !== undefined && cls.base_cost_gold !== cls.cost_gold) {
    hintParts.push(`${cls.reinforce_discount.replace(/\.$/, "")} (base ${cls.base_cost_gold} → ${cls.cost_gold} Gold).`);
  }
  if (cls.upgrades_to) hintParts.push(`Upgrades to ${cls.upgrades_to}.`);
  return catTip({
    title: `Reinforce · ${cls.name}`,
    right: `${cls.cost_gold} Gold`,
    lines: [actionLine],
    extra: unitProfileNodes(cls),
    hint: hintParts.length ? hintParts.join(" ") : null,
    why,
    keywords: ["CAT_FORMATION", ...unitActionKeywords(cls)],
    dict,
  });
}

/** Tooltip for a unit class in the City Raise Squad composer. */
export function unitClassTip(data, cls) {
  if (!cls) return null;
  const hint = cls.upgrades_to ? `Upgrades to ${cls.upgrades_to}.` : null;
  return catTip({
    title: cls.name,
    right: `${cls.cost_industry} Industry`,
    extra: unitProfileNodes(cls),
    hint,
    keywords: unitActionKeywords(cls),
    dict: data?.keywords ?? null,
  });
}

/** Tooltip for upgrading an existing squad member to its next class. */
export function upgradeTip(data, sq, member, targetCls) {
  const dict = data?.keywords ?? null;
  const gold = data?.currencies?.gold;
  const why =
    sq.in_home_borders === false
      ? "The squad must stand inside your borders to upgrade."
      : typeof gold === "number" && typeof member.upgrade_cost === "number" && gold < member.upgrade_cost
        ? `Need ${member.upgrade_cost} Gold (have ${gold}).`
        : null;
  return catTip({
    title: `Upgrade · ${member.class} → ${member.upgrades_to}`,
    right: `${member.upgrade_cost} Gold`,
    lines: [`Upgrades ${member.name} in place inside this squad's [KW=CAT_FORMATION]Formation[\\KW] (instant; uses no movement; inside your borders only).`],
    extra: targetCls ? unitProfileNodes(targetCls, { currentRow: member.row }) : [],
    why,
    keywords: ["CAT_FORMATION", ...(targetCls ? unitActionKeywords(targetCls) : [])],
    dict,
  });
}

/** Tooltip for an occupied 3x3 formation cell in the Squad panel. */
export function memberTip(data, occ, r, selMember) {
  const hint = selMember
    ? selMember.id === occ.id
      ? "Click: keep it here."
      : `Click: swap with ${selMember.name}.`
    : "Click, then click another cell: move it there.";
  const hpLabel = occ.max_hp ? `${occ.hp_pct}% (${occ.max_hp})` : `${occ.hp_pct}%`;
  return catTip({
    title: occ.name,
    right: `Lv ${occ.level} ${occ.class}`,
    extra: unitProfileNodes(occ, { currentRow: r, hpLabel }),
    hint,
    keywords: unitActionKeywords(occ),
    dict: data?.keywords ?? null,
  });
}
