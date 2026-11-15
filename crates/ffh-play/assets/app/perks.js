// Titles, icons and full rules share one presentation in offers and selected city perks.
import { h } from "./dom.js";
import { iconEl } from "./look.js";
import { isoIcon, isoSkin } from "./isoui.js";

const FLAT_ICONS = {
  xp: "experience", construct: "bclass_wonder", found: "civic_membership",
  greatpeople: "greatpeople", research: "research", culture: "culture",
  religion: "_religion", seapatrol: "promo_naval", heal: "healthy", road: "moves",
  food: "food", hurry: "civic_labor", strength: "strength", gold: "gold",
  population: "_specialist", mana: "mana", help: "_seal",
};

export function perkView(perk) {
  if (!perk) return null;
  const sphere = perk.icon.startsWith("sphere_");
  const icon = (isoSkin() ? isoIcon(perk.icon, { px: 32 }) : null)
    ?? iconEl(sphere ? `icon8/${perk.icon}` : `icon16/${FLAT_ICONS[perk.icon] ?? "mana"}`, { u: sphere ? 4 : 2 });
  return h("span.perk",
    h("span.perk-heading", h("span.perk-icon", { "aria-hidden": "true" }, icon), h("strong", perk.title)),
    h("span.perk-rule", perk.text),
  );
}
