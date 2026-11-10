// Governors (this game's own rules, docs/GOVERNORS.md): the city screen's Governance section, the
// Governors screen (F9) and the two steps of an empire order.
//
// The government civic decides who runs the cities: under Despotism or God King every city is
// ruled directly; under any other government every city, the capital included, is governed. A
// governed city has a role (proposed from its building tiers, or chosen by the player when it
// suits the city well enough) and a governor, who builds by it. The player sets up to two empire
// priorities (roles that weigh more in the proposals) and places empire orders in governed cities:
// a unit (for an army too), a wonder, a palace or a ritual, ahead of the governor's choice. The
// `governors` query has the screen; `empire_items` and `empire_cities` the order's two steps; the
// buttons send `govern_role`, `govern_priorities`, `empire_order` and `empire_order_remove`,
// which the server checks. Cultural channels are the player's alone (culture.js).

import * as api from "./api.js";
import { S, dname, info, turns, orderName, emit } from "./state.js";
import { h, toast, errorText } from "./dom.js";
import { plain } from "./markup.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";

// ---- the rules, in the help display (help.js; the Governors screen's (i) opens it) ------------

/** The numbers of the rules as the last `governors` query gave them (null before one). */
let rules = null;

addHelp({
  id: "governors",
  group: "Game options",
  title: "Governors",
  body: () => [
    h("p", "The government decides who runs the cities. Under Despotism and God King every city is ruled directly. Under any other government every city, the capital included, is governed. A change of government hands every city over or takes every city back at once."),
    h("h4", "Governed cities"),
    h(
      "ul",
      h("li", "A governor chooses what the city builds and which yields its citizens favour, by the city's role: Growth, Research, Commerce, Culture, Garrison or Production."),
      h("li", "The proposed role is the one the city's building tiers suit best: a role's suitability is the mean effect of the city's tiers for its four key buildings."),
      h("li", `You may choose another role for a city if its suitability is ${rules ? `${rules.min}%` : "high enough"} or more. Choosing the proposed role again lets the role follow the proposal.`),
      h("li", `Priorities: up to ${rules ? rules.max : 2} roles weigh ${rules ? `+${rules.weight}%` : "more"} in every city's proposal.`),
      h("li", "A governor has a name and at most one trait: Cautious (one more defender) or Ambitious (the costliest building first)."),
    ),
    h("h4", "Empire orders"),
    h(
      "ul",
      h("li", "In a governed city you may order a unit (for an army too), a wonder, a palace or a ritual. It goes ahead of the governor's choice, which keeps its progress and resumes after it."),
      h("li", "An army's requisition goes first, as in any city: you pick the cities on the Armies screen [F5]."),
    ),
  ],
});

/** The class that colours a building tier (`tier-0` Lesser .. `tier-4` Legendary; none for
 *  Standard), as the city screen shows tiers. */
function tierSel(t) {
  return t === null || t === undefined || t === 1 ? "" : `.tier-${t}`;
}

/** "Steward (Cautious)" with the trait's rule as the title. */
function governorNode(gv) {
  if (!gv) return h("span.dim", "No governor");
  if (gv.trait === null || gv.trait === undefined) return h("span.governor-name", gv.name);
  const help = info("GovernorTraitType", gv.trait)?.help;
  return h("span.governor-name", { title: help ? plain(help) : "" }, `${gv.name} (${dname("GovernorTraitType", gv.trait)})`);
}

/** "Research (proposed)" or "Research (overruled)" for a governed city. */
function roleLabel(c) {
  if (c.role === null || c.role === undefined) return "-";
  return `${dname("CityRoleType", c.role)} (${c.role_overridden ? "overruled" : "proposed"})`;
}

/** "Government: Aristocracy. Every city is governed." */
function modeLine(government, direct) {
  return `Government: ${dname("CivicType", government)}. Every city is ${direct ? "ruled directly" : "governed"}.`;
}

/** The city screen's Governance section (null for another's city). `reload` re-reads the screen. */
export function governanceView(c) {
  const k = c.governance;
  if (!k) return null;
  if (!k.governed) {
    return h("div.section.governance", h("h4", "Governance"), h("p", `Ruled directly. Government: ${dname("CivicType", k.government)}.`));
  }
  return h(
    "div.section.governance",
    h("h4", "Governance"),
    h("p", "Governed by ", governorNode(k.governor), `. Role: ${roleLabel(k)}${k.suitability !== null && k.suitability !== undefined ? `, suitability ${k.suitability}%` : ""}.`),
  );
}

/** The bare line of a diagnosis; a city one names the city as a link (`go`). */
function diagnosisLine(x, go) {
  const city = () => h("button.link", { onclick: () => go(x.city), title: "Open the city" }, x.name);
  switch (x.kind) {
    case "role_unfilled":
      return h("li", `No city has role ${dname("CityRoleType", x.role)}.`);
    case "poor_fit":
      return h("li", city(), `: role ${dname("CityRoleType", x.role)} at suitability ${x.suitability}%.`);
    case "slow_order":
      return h("li", city(), `: ${x.order !== null && x.order !== undefined ? orderName(x.order, x.type) : "order"} takes ${turns(x.turns)}.`);
    default:
      return null;
  }
}

/** A queue order of a role card: "Warrior · 4 turns · to the 1st Army" with a remove button for an
 *  empire order. */
function orderLine(c, o, act) {
  return h(
    "li",
    { "data-index": o.index },
    h("span", orderName(o.order, o.type)),
    h("span.dim", ` · ${turns(o.turns)}`),
    o.requisition ? h("span.dim", ` · requisition${o.dest_name ? ` for ${o.dest_name}` : ""}`) : o.dest_name ? h("span.dim", ` · to ${o.dest_name}`) : null,
    o.empire ? h("button.small.empire-remove", { title: "Take the empire order off (what was put into it stays)", "aria-label": "Remove", onclick: () => act({ c: "empire_order_remove", city: c.city, index: o.index }) }, "✕") : null,
  );
}

/** The role card of governed city `c` (the `governors` query's). */
function roleCard(c, r, act, go) {
  const k = c.card;
  const keys = k.key_buildings.map((b) =>
    h(
      "li",
      { title: b.none ? "Your civilization cannot build it." : `Core effect: ${b.effect}%` },
      h("span", `${dname("BuildingClassType", b.class)}: `),
      b.none ? h("span.dim", "cannot build") : h(`span${tierSel(b.tier)}`, dname("BuildingTierType", b.tier)),
    ),
  );
  const current = k.roles.find((x) => x.role === c.role);
  const others = k.roles
    .filter((x) => x.role !== c.role)
    .map((x) =>
      h(
        "button.small.role-switch",
        {
          "data-role": x.role,
          disabled: !x.allowed,
          title: x.allowed ? `Switch to ${dname("CityRoleType", x.role)}${x.role === c.proposal ? " (the proposal)" : ""}. Now: ${dname("CityRoleType", c.role)} ${current?.suitability ?? "-"}%.` : (x.why ?? ""),
          onclick: () => act({ c: "govern_role", city: c.city, role: x.role }),
        },
        `${dname("CityRoleType", x.role)} ${x.suitability}%`,
      ),
    );
  const plan = k.plan.length ? `Plan: ${k.plan.map((b) => dname("BuildingType", b)).join(", ")}` : "Plan: none.";
  return h(
    "div.role-card",
    { "data-city": c.city },
    h(
      "div.role-card-head",
      h("button.link", { onclick: () => go(c.city), title: "Open the city" }, h("b", c.name)),
      c.capital ? h("span.dim", " (capital)") : null,
      h("span", " · "),
      governorNode(c.governor),
    ),
    h("div.role-line", h("span", "Role: "), h("b", dname("CityRoleType", c.role)), h("span.dim", ` (${c.role_overridden ? "overruled" : "proposed"}), suitability ${c.suitability}%`)),
    h("ul.role-keys", keys),
    h("div.role-others", h("span.dim", "Other roles: "), others),
    h("div.role-plan.dim", plan),
    k.orders.length ? h("ul.role-orders", k.orders.map((o) => orderLine(c, o, act))) : null,
    k.governor_order ? h("div.dim", `Governor builds: ${orderName(k.governor_order.order, k.governor_order.type)} · ${turns(k.governor_order.turns)}`) : null,
  );
}

/** The Governors screen (F9): the mode line; with governed cities the priorities, the diagnoses,
 *  the Empire order action and a role card per city. */
export async function openGovernors({ openCity } = {}) {
  const m = openModal({ title: "Governors", body: [h("p.dim", "…")], tag: "governors", wide: true, buttons: [{ label: "Close", value: null }], help: "governors" });
  const load = async () => {
    let r;
    try {
      r = await api.query("governors");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    rules = { min: r.min_suitability, max: r.max_priorities, weight: r.priority_weight };
    const act = async (cmd) => {
      await run(cmd);
      load();
    };
    const go = (id) => {
      m.close(null);
      openCity?.(id);
    };
    const mode = h("p.governors-mode", modeLine(r.government, r.direct_rule));
    if (r.direct_rule) {
      const rows = r.cities.map((c) => h("tr", { "data-city": c.city }, h("td", h("button.link", { onclick: () => go(c.city), title: "Open the city" }, c.name), c.capital ? h("span.dim", " (capital)") : null), h("td.num", `${c.production}`)));
      m.setBody(mode, h("table.governors", h("thead", h("tr", h("th", "City"), h("th", { title: "Production" }, "Prod."))), h("tbody", rows)));
      return;
    }
    const roles = (S.data?.tables?.CityRoleType ?? []).map((_, i) => i);
    const on = new Set(r.priorities);
    const toggles = roles.map((role) =>
      h(
        "label.toggle.priority",
        { "data-role": role },
        h("input", {
          type: "checkbox",
          checked: on.has(role),
          disabled: !on.has(role) && on.size >= r.max_priorities,
          onchange: (ev) => {
            const next = r.priorities.filter((x) => x !== role);
            if (ev.target.checked) next.push(role);
            act({ c: "govern_priorities", roles: next });
          },
        }),
        ` ${dname("CityRoleType", role)}`,
      ),
    );
    const diagnoses = r.diagnoses.length ? h("ul.diagnoses", r.diagnoses.map((x) => diagnosisLine(x, go))) : h("p.dim", "None.");
    const cards = r.cities.filter((c) => c.governed && c.card).map((c) => roleCard(c, r, act, go));
    m.setBody(
      mode,
      h("h4", "Priorities"),
      h("div.priorities", toggles),
      h("p.dim", `Priority: +${r.priority_weight}% weight in role proposals. At most ${r.max_priorities}.`),
      h("h4", "Diagnoses"),
      diagnoses,
      h(
        "div.row",
        h(
          "button.small.empire-order",
          {
            title: "Order a unit, a wonder, a palace or a ritual in a governed city, ahead of its governor's choice",
            onclick: async () => {
              const item = await pickEmpireItem({});
              if (!item) return;
              await orderEmpire({ order: item.order, data: item.type, beforeMap: () => m.close(null) });
              if (!m.closed) load();
            },
          },
          "Empire order",
        ),
      ),
      h("h4", `Cities (${cards.length})`),
      h("div.role-cards", cards),
    );
  };
  await load();
  return m.done;
}

// ---- empire orders: the two steps -------------------------------------------------------------

const GROUPS = [
  [0, "Units"],
  [1, "Wonders and palaces"],
  [2, "Rituals"],
];

/** Step 1: what to order. With `army` (an army id), the units that serve in armies, those no
 *  governed city can train shown unavailable. Resolves with `{order, type}` or null. */
export async function pickEmpireItem({ army = null, title = null } = {}) {
  let r;
  try {
    r = await api.query("empire_items", army !== null ? { army } : {});
  } catch (e) {
    toast(errorText(e), "error");
    return null;
  }
  if (!r.governed) {
    toast("No governed city.", "error");
    return null;
  }
  const m = openModal({ title: title ?? "Empire order", body: [], tag: "empire-item", wide: true, buttons: [{ label: "Cancel", value: null }] });
  const lists = GROUPS.map(([order, label]) => {
    const items = r.items.filter((x) => x.order === order);
    if (!items.length) return null;
    return h(
      "div.section",
      h("h4", label),
      h(
        "div.build-grid",
        items.map((x) =>
          h(
            "button.build.empire-item",
            {
              "data-order": x.order,
              "data-type": x.type,
              disabled: !x.cities,
              title: x.cities ? `${x.cities} governed cit${x.cities === 1 ? "y" : "ies"} can build it.` : "No governed city can build it.",
              onclick: () => m.close({ order: x.order, type: x.type }),
            },
            h("span", orderName(x.order, x.type)),
            h("span.dim", x.cities ? `${x.cities} cit${x.cities === 1 ? "y" : "ies"}` : "unavailable"),
          ),
        ),
      ),
    );
  }).filter(Boolean);
  m.setBody(lists.length ? lists : h("p.dim", "Nothing to order."));
  return m.done;
}

/** "Warrior · 6 turns" for what a city builds now and would be delayed. */
function displaced(x) {
  if (!x.displaces) return h("span.dim", "builds nothing now");
  return h("span", `${orderName(x.displaces.order, x.displaces.type)} +${turns(x.own_turns)}`);
}

/** Step 2: where. Every governed city able to build the item, the suggestion pre-selected (Enter
 *  or Order confirms it: one click); "On the map" outlines the cities on the map, where a click on
 *  one orders it there (`beforeMap` first closes the screen that asked). Sends `empire_order`
 *  (with `army` as the destination). Resolves with whether the order was placed (null when it
 *  went to the map). */
export async function orderEmpire({ order, data, army = null, beforeMap = null }) {
  let r;
  try {
    r = await api.query("empire_cities", { order, data, ...(army !== null ? { army } : {}) });
  } catch (e) {
    toast(errorText(e), "error");
    return false;
  }
  if (!r.cities.length) {
    toast("No governed city can build it.", "error");
    return false;
  }
  const what = orderName(order, data);
  const send = (c) => run({ c: "empire_order", city: c.city, order, data, dest: r.dest });
  let picked = r.cities.find((c) => c.suggested) ?? r.cities[0];
  const m = openModal({ title: `${what}: which city?`, body: [], tag: "empire-city", wide: true, buttons: [] });
  const render = () => {
    const rows = r.cities.map((c) =>
      h(
        "tr.empire-city",
        { "data-city": c.city, class: c.city === picked.city ? "picked" : "", onclick: () => ((picked = c), render()) },
        h("td", h("input", { type: "radio", name: "empire-city", checked: c.city === picked.city, "aria-label": c.name })),
        h("td", c.name, c.suggested ? h("span.dim", " (suggested)") : null),
        h("td.num", turns(c.turns)),
        h("td", c.ahead ? h("span.dim", `${c.ahead} ahead (${turns(c.ahead_turns)})`) : h("span.dim", "-")),
        h("td", displaced(c)),
        army !== null ? h("td.num", c.distance === null || c.distance === undefined ? "-" : `${c.distance}`) : null,
        h("td", c.cost_percent !== undefined ? h(`span${tierSel(c.tier)}`, `Cost: ${c.cost_percent}%`) : null),
      ),
    );
    m.setBody(
      h(
        "table.governors.empire-cities",
        h("thead", h("tr", h("th", ""), h("th", "City"), h("th", "Turns"), h("th", "Ahead"), h("th", "Delays"), army !== null ? h("th", { title: "Plots to the army" }, "Distance") : null, h("th", ""))),
        h("tbody", rows),
      ),
    );
    m.setButtons([
      { label: `Order in ${picked.name}`, primary: true, onClick: async () => (await send(picked)) && m.close(true) },
      { label: "On the map", title: "Outline the cities on the map; a click on one orders it there (Esc cancels)", onClick: () => toMap() },
      { label: "Cancel", value: false },
    ]);
    m.el.querySelector(".modal-buttons button.primary")?.focus({ preventScroll: true });
  };
  const toMap = () => {
    m.close(null);
    beforeMap?.();
    emit("center", picked.x, picked.y);
    emit("map-pick", {
      label: `empire order of ${what}, an outlined city`,
      plots: r.cities.map((c) => [c.x, c.y]),
      pick: async (x, y) => {
        const c = r.cities.find((k) => k.x === x && k.y === y);
        if (!c) return toast("Not one of the outlined cities.", "error");
        if (await send(c)) toast(`${what}: ordered in ${c.name}.`);
      },
    });
    toast(`${what}: click an outlined city. Esc cancels.`, "info", 6000);
  };
  render();
  return m.done;
}
