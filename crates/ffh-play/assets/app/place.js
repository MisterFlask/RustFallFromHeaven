// World spells the player places with its ritual's charge, no unit casting them (docs/CARDS.md,
// "World rituals"; docs/HUMAN.md): Wellspring. The placement is a plot pick on the map (units.js
// `openPlacement`) and then this menu of the plot's mana types (the `place_menu` query, every
// word of it the server's): each row a mana with its icon, what owning it gives (`passive`) and
// the sphere promotions it opens as icons whose tooltips list their spells. A row places the
// node; Cancel goes back to the plot pick; Escape postpones (the charge stays held).

import { tname } from "./state.js";
import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { render as markup } from "./markup.js";
import { promotionIcon, uiIcon } from "./look.js";
import { showTip } from "./tips.js";
import { passiveText, promoTipParts } from "./ritual_rows.js";

/** A sphere promotion's tooltip: its name, requirement and free rule, then each spell it opens
 *  with the spell's help. */
function promoTip(p) {
  const t = promoTipParts(p);
  return [
    h("div.tip-title", t.title),
    ...t.lines.map((l) => h("div.tip-hint", l)),
    t.none ? h("div.dim", t.none) : null,
    ...t.spells.map((s) => h("div.place-tip-spell", h("b", s.name), s.help ? h("div.tip-effects", markup(s.help)) : null)),
  ];
}

/** A promotion's icon (its seal), or a chip of its tier (`I`) where it has no art. */
function promoChip(p) {
  const icon = promotionIcon(p.type, { u: 2, title: null });
  return h(
    "button.place-promo",
    {
      type: "button",
      "data-promotion": p.promotion,
      "aria-label": `${p.name}: spells`,
      tip: () => promoTip(p),
      // A tap shows the tooltip (no hover on a touch screen) and places nothing.
      onclick: (ev) => {
        ev.stopPropagation();
        showTip(ev.currentTarget);
      },
    },
    icon ?? h("span.place-tier", p.tier),
  );
}

/** One mana type of the menu: the row that places it. */
function row(r, choose) {
  const icon = uiIcon("mana", tname("BonusType", r.bonus) ?? "", 16) ?? h("span.place-mana-dot", (r.name ?? "?").slice(0, 1));
  const pick = () => choose(r);
  return h(
    "div.place-row",
    { role: "button", tabindex: 0, "data-bonus": r.bonus, "aria-label": `Place ${r.name}`, onclick: pick, onkeydown: (ev) => (ev.key === "Enter" || ev.key === " ") && (ev.preventDefault(), pick()) },
    h("div.place-head", h("span.place-icon", icon), h("span.place-name", r.name), h("span.place-owned.dim", r.owned ?? "")),
    h("div.place-passive", h("span.place-label", "Passive:"), passiveText(r).length === 1 && !r.passive?.length ? " none." : h("ul.place-lines", passiveText(r).map((l) => h("li", l)))),
    r.promotions?.length ? h("div.place-promos", h("span.place-label", "Spell promotions: "), r.promotions.map(promoChip)) : null,
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
    body: [h("div.place-rows", (menu.rows ?? []).map((r) => row(r, choose)))],
    buttons: [{ label: "Cancel", value: "back" }],
  });
  return m.done;
}
