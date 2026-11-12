// World spells the player places with its ritual's charge, no unit casting them (docs/CARDS.md,
// "World rituals"; docs/HUMAN.md): Wellspring. The placement is a plot pick on the map (units.js
// `openPlacement`) and then this menu of the plot's mana types (the `place_menu` query, every
// word of it the server's). Each row is compact: the mana's icon, name and count (`1 → 2`), its
// effect chips, the sphere's spells as chips grouped by tier (I, II, III), and at most one line on
// what this placement changes; every sentence of what owning it gives is in a chip's tooltip. A
// row places the node; a tap on a chip shows its tooltip and places nothing. Cancel goes back to
// the plot pick; Escape postpones (the charge stays held).

import { tname } from "./state.js";
import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { render as markup } from "./markup.js";
import { iconEl, typeIcon, uiIcon } from "./look.js";
import { showTip } from "./tips.js";
import { chipArt, chipGlyph, spellLabel, spellTipParts, tierTipParts } from "./ritual_rows.js";

/** A tap (or click) on a chip shows its tooltip (no hover on a touch screen) and does not reach
 *  the row, which would place the node; Enter or Space on a focused chip does the same. */
const tipOnly = {
  onclick: (ev) => {
    ev.stopPropagation();
    showTip(ev.currentTarget);
  },
  onkeydown: (ev) => {
    if (ev.key !== "Enter" && ev.key !== " ") return;
    ev.preventDefault();
    ev.stopPropagation();
    showTip(ev.currentTarget);
  },
};

/** A tier mark (`I`): its tooltip the promotion's name, requirement and free rule. */
function tierMark(p) {
  const t = tierTipParts(p);
  return h(
    "button.place-tier",
    { type: "button", "data-promotion": p.promotion, "aria-label": [p.name, ...t.lines].join(" "), tip: () => [h("div.tip-title", t.title), ...t.lines.map((l) => h("div.tip-hint", l))], ...tipOnly },
    p.tier,
  );
}

/** A spell of a tier, as a chip with its name: its tooltip the name, `Needs Fire I.` and its help. */
function spellChip(s) {
  const t = spellTipParts(s);
  return h(
    "button.place-spell",
    {
      type: "button",
      "data-spell": s.spell,
      "aria-label": spellLabel(s),
      tip: () => [h("div.tip-title", t.title), ...t.lines.map((l) => h("div.tip-hint", l)), t.help ? h("div.tip-effects", markup(t.help)) : null],
      ...tipOnly,
    },
    s.name,
  );
}

/** An effect chip (`chips` of a row: health, happiness, affinity, buildings, info): an icon and
 *  its label, its tooltip the full sentences. */
function effectChip(c) {
  const art = chipArt(c.kind);
  const icon =
    (art ? iconEl(`icon8/${art}`) : c.kind === "buildings" && c.building !== undefined ? typeIcon("building", c.building, 8) : null) ??
    h("span.place-glyph", chipGlyph(c.kind));
  return h(
    `button.place-chip.place-chip-${c.kind}`,
    { type: "button", "aria-label": c.aria, tip: () => (c.tip ?? []).map((l, i) => h(i === 0 && c.kind !== "info" ? "div.tip-title" : "div.tip-hint", l)), ...tipOnly },
    icon,
    c.label ? h("span.place-chip-n", c.label) : null,
    c.new ? h("span.place-new", { "aria-hidden": "true" }) : null,
  );
}

/** One mana type of the menu: the row that places it. */
function row(r, choose) {
  const icon = uiIcon("mana", tname("BonusType", r.bonus) ?? "", 16) ?? h("span.place-mana-dot", (r.name ?? "?").slice(0, 1));
  const pick = () => choose(r);
  const count = r.count ? h("span.place-count", { tip: r.count_tip, "aria-label": r.count_tip }, r.count) : null;
  return h(
    "div.place-row",
    {
      role: "button",
      tabindex: 0,
      "data-bonus": r.bonus,
      "aria-label": `Place ${r.name}. ${r.count_tip ?? ""}${r.now ? ` ${r.now.text}` : ""}`.trim(),
      onclick: pick,
      onkeydown: (ev) => ev.target === ev.currentTarget && (ev.key === "Enter" || ev.key === " ") && (ev.preventDefault(), pick()),
    },
    h("div.place-head", h("span.place-icon", icon), h("span.place-name", r.name), count, h("span.place-chips", (r.chips ?? []).map(effectChip))),
    r.promotions?.length ? h("div.place-tiers", r.promotions.map((p) => h("span.place-tier-group", tierMark(p), (p.spells ?? []).map(spellChip)))) : null,
    r.now ? h(`div.place-now${r.now.warn ? ".warn" : ""}`, r.now.text) : null,
  );
}

/** Opens the menu of `menu` (a `place_menu` reply). Resolves with the row chosen, "back" (Cancel:
 *  back to the plot pick) or null (Escape: later). */
export function choosePlacement(menu) {
  let m = null;
  const choose = (r) => m.close(r);
  m = openModal({
    title: menu.title,
    tag: "place-menu",
    wide: true,
    className: "place-menu",
    body: [menu.intro ? h("div.place-intro.dim", menu.intro) : null, h("div.place-rows", (menu.rows ?? []).map((r) => row(r, choose)))],
    buttons: [{ label: "Cancel", value: "back" }],
  });
  return m.done;
}
