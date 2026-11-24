// Armies (this game's own rules, docs/ARMIES.md): the Armies screen (F5) and the city screen's
// garrison line.
//
// Land combat units serve in armies (named, moving as one) or in a city's garrison. A unit a city
// trains appears in its city, in its garrison, as any unit. The `armies` query lists the armies
// and the garrisons; the buttons send `army_rename`, `army_upgrade` and `army_disband`, which the
// server checks.
//
// Each army's supply (docs/SUPPLY.md) is stated in the view's bare words (`supply.text`).
//
// Fleets (docs/NAVAL.md, section 7): warships and ships with cargo room serve in fleets, the
// armies' container at sea (the query's `fleets`). A fleet has no supply line; it has
// a standing order (`station`: none, Patrol of a sea of `seas`, Escort of one of
// `escort_targets`), sent by `army_station`, and its ships' wear.
//
// Raids (docs/RAIDS.md, section 8): a land army holding the Raid order says so (`station`:
// `kind: "raid"`, `text`), with Stop raiding (`army_station` with no station).
//
// Waypoints (docs/ARMIES.md): a city's waypoint (the city query's `waypoint`), an army, a fleet or
// a plot its new units walk to under an ordinary go-to; set with `city_waypoint` from the city
// screen (`waypointView`) or from an army's card (`Waypoint of: …`, its Cities… checklist), which
// also lists the units on their way to it (`waypoint_units`).

import * as api from "./api.js";
import { S, dname, cityAt, me, emit } from "./state.js";
import { h, errorText, toast } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";
import { squadBoard } from "./squads.js";
import { caravanStationValue, caravanStation } from "./trade_view.js";

// ---- the rules, in the help display (help.js; the Armies screen's (i) opens it) ---------------

addHelp({
  id: "armies",
  group: "Game options",
  title: "Armies and garrisons",
  body: () => [
    h("p", "Land combat units serve in armies, which move as one, or in a city's garrison. Heroes, mages, recon units and summons move on their own."),
    h("h4", "Forming armies"),
    h(
      "ul",
      h("li", "Units outside your cities form armies: select some and choose Form a new army. An army can merge into another."),
      h("li", "In a city, an army's units can join its garrison; a garrison unit that leaves the city forms an army again. A disbanded army marches to the nearest of your cities and joins its garrison there."),
    ),
    h("h4", "Supply lines"),
    h(
      "ul",
      h("li", "Within your borders an army needs no supply line. Outside them, each army needs one: the cheapest path over land to one of your cities, or to one of your ships that can carry land units, within its budget. A road plot costs least, then your own territory; other land costs more. Enemy units, enemy cities, land next to a manned enemy fort and closed borders block it."),
      h("li", "An invested city is not counted as your territory: an army in it needs a supply line, and it moves out of the city only to attack until the siege lifts."),
      h("li", "The map draws your armies' lines, and the lines of other armies you see over the plots you have revealed. With an army selected, the plot under the pointer says whether it would be supplied there."),
      h("li", "Cut at the start of your turn: Hunger (no healing), after a few turns Starvation (units lose HP each turn down to a floor, and deal less damage in battles). A restored line ends both at your next turn."),
    ),
    h("h4", "Fleets"),
    h(
      "ul",
      h("li", "Warships and ships with cargo room serve in fleets, which move as one. A ship at sea forms a fleet of its own; a ship in port in no fleet is simply a ship. A fleet has no supply line: range and wear instead."),
      h("li", "Standing order (the Armies screen): Patrol of a sea in your range: the fleet sails to the nearest plot of the sea and its warships take the Patrol order there. Escort of an army or a fleet: when it ends a move on water, the fleet moves to its plot if it can reach it this turn. A move order ends the standing order."),
      h("li", "A disbanded fleet sails to the nearest of your ports; there its ships are in no fleet."),
    ),
    h("h4", "Waypoints"),
    h(
      "ul",
      h("li", "A city's waypoint is one of your armies, a fleet (a coastal city only) or a plot. Set it on the city screen, or with Cities… on an army's card [F5]."),
      ...WAYPOINT_RULES.map((t) => h("li", t)),
    ),
  ],
});

/** The armies whose squad board is open on the Armies screen (kept when the screen is re-read). */
const squadsOpen = new Set();

/** Squads (docs/ENGAGEMENTS.md, "Squads"): army `a`'s squads as a board under a fold, or null;
 *  `reload` re-reads the screen after a change. */
function squadsView(a, reload) {
  const list = a.squads ?? [];
  if (!list.length) return null;
  const d = h(
    "details.army-squads",
    { "data-army": a.id, ontoggle: (ev) => (ev.target.open ? squadsOpen.add(a.id) : squadsOpen.delete(a.id)) },
    h("summary", `Squads: ${list.length}`),
    squadBoard(list, { unitOf: (id) => S.units.get(id) ?? null, changed: reload }),
  );
  if (squadsOpen.has(a.id)) d.open = true;
  return d;
}

// ---- waypoints --------------------------------------------------------------------------------

/** The waypoint rules, bare, for the help and the city screen's tooltip. */
export const WAYPOINT_RULES = [
  "A unit the city trains walks to the waypoint under a go-to order. To an army or a fleet: the units that can serve in it. To a plot: units with an attack strength, land units to land and ships to water.",
  "On its way a unit is in no army and no garrison. Any order you give it ends its waypoint.",
  "On arrival it joins the army; on a plot, your oldest army there, or the garrison in your city, else it stands alone. No way there: it stays and waits for orders.",
  "A governed city: only the units of your empire orders follow the waypoint.",
];

/** The city screen's line for waypoint `wp` (the city query's `waypoint`): "Waypoint: none",
 *  "Waypoint: the 2nd Army", "Waypoint: plot 14, 9". */
export function waypointText(wp) {
  return `Waypoint: ${wp?.name ?? "none"}`;
}

/** The value of waypoint `wp` in the city screen's selector: "none", "army:3", "plot:14,9". */
export function waypointValue(wp) {
  if (!wp || wp.kind === "none") return "none";
  return wp.kind === "army" ? `army:${wp.army}` : `plot:${wp.x},${wp.y}`;
}

/** `city_waypoint`'s `to` for a selector value (`waypointValue`'s form). */
export function waypointTo(value) {
  if (!value || value === "none") return null;
  const [k, rest] = value.split(":");
  if (k === "army") return { army: Number(rest) };
  const [x, y] = rest.split(",").map(Number);
  return { plot: [x, y] };
}

/** "Waypoint of: Vallus, Torrolerial" or "Waypoint of: no city" for army `a`. */
export function waypointOfText(a) {
  const names = (a.waypoint_cities ?? []).map((c) => c.name);
  return `Waypoint of: ${names.length ? names.join(", ") : "no city"}`;
}

/** The city screen's waypoint line (`c.waypoint`; null for another's city): the waypoint, a
 *  selector (None, each army, each fleet for a coastal city, the plot if it is one) and Plot…,
 *  which closes the screen (`close`) into a pick of a plot on the map (Esc cancels). `reload`
 *  re-reads the screen. */
export function waypointView(c, reload, close) {
  const wp = c.waypoint;
  if (!wp) return null;
  const set = async (to) => {
    if (await run({ c: "city_waypoint", city: c.id, to })) reload();
  };
  const cur = waypointValue(wp);
  const opts = [h("option", { value: "none", selected: cur === "none" }, "None")];
  for (const a of wp.choices ?? []) {
    const v = `army:${a.army}`;
    opts.push(h("option", { value: v, selected: cur === v }, a.fleet ? `${a.name} (fleet)` : a.name));
  }
  if (!opts.some((o) => o.value === cur)) opts.push(h("option", { value: cur, selected: true }, wp.name ?? cur));
  const pickPlot = () => {
    close?.();
    emit("center", c.x, c.y);
    emit("map-pick", {
      label: `the waypoint of ${c.name}`,
      plots: [],
      pick: async (x, y) => {
        if (await run({ c: "city_waypoint", city: c.id, to: { plot: [x, y] } })) toast(`${c.name}: waypoint plot ${x}, ${y}.`);
      },
    });
    toast(`Waypoint of ${c.name}: click a plot. Esc cancels.`, "info", 6000);
  };
  return h(
    "div.city-waypoint",
    { tip: WAYPOINT_RULES.join(" ") },
    h("span.waypoint-line", waypointText(wp)),
    " ",
    h("select.city-waypoint-pick", { "aria-label": "Waypoint", onchange: (ev) => set(waypointTo(ev.target.value)) }, opts),
    " ",
    h("button.small.waypoint-plot", { onclick: pickPlot, title: "Pick a plot on the map" }, "Plot…"),
    wp.governed ? h("div.dim.waypoint-governed", "Governed city: only the units of your empire orders follow the waypoint.") : null,
  );
}

/** The armies whose Cities… checklist is open on the Armies screen (kept when it is re-read). */
const waypointsOpen = new Set();

/** Army or fleet `a`'s waypoint lines on the Armies screen: `Waypoint of: …` with Cities… (a
 *  checklist of `garrisons`, the screen's cities: a tick makes this army the city's waypoint, an
 *  untick clears it; a fleet only for a coastal city) and `On the way: n` with the units (a click
 *  selects one: `goUnit`). `act` sends a command and re-reads the screen. */
function waypointCard(a, garrisons, act, goUnit) {
  const fleet = a.domain === "sea";
  const rows = garrisons.map((gr) => {
    const wp = gr.waypoint ?? { kind: "none" };
    const mine = wp.kind === "army" && wp.army === a.id;
    const off = fleet && !gr.port;
    return h(
      "label.waypoint-city",
      { "data-city": gr.city, title: off ? "Not a coastal city" : null },
      h("input", { type: "checkbox", checked: mine, disabled: off || !!a.disband_to, onchange: (ev) => act({ c: "city_waypoint", city: gr.city, to: ev.target.checked ? { army: a.id } : null }) }),
      h("span", ` ${gr.name}`),
      !mine && wp.kind !== "none" ? h("span.dim", ` (now: ${wp.name})`) : null,
    );
  });
  const pick = h(
    "details.waypoint-pick",
    { "data-army": a.id, ontoggle: (ev) => (ev.target.open ? waypointsOpen.add(a.id) : waypointsOpen.delete(a.id)) },
    h("summary", { title: "Tick a city: its new units walk to this army" }, "Cities…"),
    h("div.waypoint-cities", rows),
  );
  if (waypointsOpen.has(a.id)) pick.open = true;
  const units = (a.waypoint_units ?? []).map((id) => S.units.get(id)).filter(Boolean);
  return [
    h("div.waypoint-of", { tip: WAYPOINT_RULES.join(" ") }, h("span", waypointOfText(a)), " ", pick),
    units.length
      ? h(
          "div.waypoint-units",
          h("span", `On the way: ${units.length}`),
          h("ul.waypoint-unit-list", units.map((u) => h("li", h("button.link.small", { onclick: () => goUnit(u), title: "Select it on the map" }, `${dname("UnitType", u.type)} at (${u.x}, ${u.y})`)))),
        )
      : null,
  ];
}

/** "2 Axeman, 1 Warrior" for a list of unit ids (the view's units). */
function composition(ids) {
  const n = new Map();
  for (const id of ids) {
    const u = S.units.get(id);
    if (!u) continue;
    n.set(u.type, (n.get(u.type) ?? 0) + 1);
  }
  return [...n].sort((a, b) => b[1] - a[1]).map(([t, k]) => `${k} ${dname("UnitType", t)}`).join(", ");
}

/** A fleet's standing order selector (docs/NAVAL.md, 7.2): none, Patrol with each sea of `seas`,
 *  Escort with each army or other fleet of `targets`; `set` sends the chosen order (null, `{patrol}`
 *  or `{escort}`). */
export function stationControl(f, seas, targets, set) {
  const st = f.station ?? { kind: "none" };
  const cur = caravanStationValue(st) ?? (st.kind === "patrol" ? `patrol:${st.sea}` : st.kind === "escort" ? (st.army !== null && st.army !== undefined ? `escort:${st.army}` : `group:${st.group}`) : "none");
  const opts = [h("option", { value: "none", selected: cur === "none" }, "No standing order")];
  for (const s of seas) opts.push(h("option", { value: `patrol:${s.id}`, selected: cur === `patrol:${s.id}` }, `Patrol: ${s.name}`));
  for (const t of targets) {
    if (t.army === f.id) continue;
    opts.push(h("option", { value: `escort:${t.army}`, selected: cur === `escort:${t.army}` }, `Escort: ${t.name}`));
  }
  // Trade (docs/TRADE.md 4.4): the caravans it could escort.
  for (const c of f.escort_caravans ?? []) {
    const v = `caravan:${c.route}:${c.slot}`;
    opts.push(h("option", { value: v, selected: cur === v }, `Escort: ${c.name}`));
  }
  // An order the lists do not hold (a sea now out of range, a ship group): kept as it is.
  if (!opts.some((o) => o.value === cur)) opts.push(h("option", { value: cur, selected: true }, st.text ?? cur));
  const pick = (v) => {
    if (v === "none") return set(null);
    const caravan = caravanStation(v);
    if (caravan) return set(caravan);
    const [k, id] = v.split(":");
    if (k === "patrol") return set({ patrol: Number(id) });
    if (k === "escort") return set({ escort: Number(id) });
    return set({ escort_group: Number(id) });
  };
  return h(
    "span.station",
    { title: (f.escort_caravans ?? []).length ? ESCORT_LINE : null },
    h("span", "Standing order "),
    h("select.fleet-station", { "aria-label": "Standing order", title: "Patrol: the fleet sails to the sea and patrols it. Escort: it follows the army or fleet when that ends a move on water and it can reach it this turn. A move order ends it.", onchange: (ev) => pick(ev.target.value) }, opts),
  );
}

/** The pace rule of an Escort of a caravan (docs/TRADE.md 4.4), said where the order is offered. */
const ESCORT_LINE = "A caravan standing with its escort moves only as far as the escort can.";

/** A land army's Escort of a caravan (docs/TRADE.md 4.4): a selector of the caravans it could
 *  escort (`a.escort_caravans`), its order's line and why it waits; null when it has none to
 *  escort and escorts none. */
export function escortControl(a, set) {
  const list = a.escort_caravans ?? [];
  const cur = caravanStationValue(a.station);
  if (!list.length && !cur) return null;
  const opts = [h("option", { value: "none", selected: !cur }, "No escort")];
  for (const c of list) {
    const v = `caravan:${c.route}:${c.slot}`;
    opts.push(h("option", { value: v, selected: cur === v }, c.name));
  }
  if (cur && !opts.some((o) => o.value === cur)) opts.push(h("option", { value: cur, selected: true }, a.station.text));
  return h(
    "div.row.escort-station",
    h(
      "label",
      { title: ESCORT_LINE },
      "Escort ",
      h("select.army-escort", { "aria-label": "Escort a caravan", onchange: (ev) => set(ev.target.value === "none" ? null : caravanStation(ev.target.value)) }, opts),
    ),
    cur ? h("span.dim", ` ${a.station.text}`) : null,
    a.station?.wait ? h("span.warn", ` ${a.station.wait}`) : null,
    h("div.small.dim", ESCORT_LINE),
  );
}

/** The Armies screen (F5): every army (name, units, place, supply), every fleet (its standing
 *  order and wear in place of supply) and every garrison. */
export async function openArmies({ select, centerOn } = {}) {
  const m = openModal({ title: "Armies", body: [h("p.dim", "…")], tag: "armies", wide: true, buttons: [{ label: "Close", value: null }], help: "armies" });
  const load = async () => {
    let r;
    try {
      r = await api.query("armies");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    const act = async (cmd) => {
      await run(cmd);
      // Re-read either way: a refused command leaves the controls showing the army's state.
      load();
    };
    // Disbanding is not undone: ask first (Cancel has the focus). Its units join the garrison of
    // the city they stand in, else march to the nearest of your cities (army_disband).
    const disband = async (a) => {
      const n = a.units.length;
      const city = a.x !== null ? cityAt(a.x, a.y) : null;
      const one = n === 1;
      const where = city && city.owner === me()?.player ? `${one ? "joins" : "join"} the garrison of ${city.name}` : `${one ? "marches" : "march"} to the nearest of your cities and ${one ? "joins" : "join"} its garrison`;
      const fleet = a.domain === "sea";
      const v = await ask(fleet ? "Disband the fleet?" : "Disband the army?", fleet ? `Disband the ${a.name}? Its ${one ? "ship" : `${n} ships`} ${city && city.owner === me()?.player ? "stay in port" : "sail to the nearest of your ports"}, in no fleet.` : `Disband the ${a.name}? Its ${one ? "unit" : `${n} units`} ${where}.`, [
        { label: "Disband", value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      if (v) act({ c: "army_disband", army: a.id });
    };
    // A unit on its way: selected on the map.
    const goUnit = (u) => {
      m.close(null);
      centerOn?.(u.x, u.y);
      select?.([u.id]);
    };
    const armies = r.armies.map((a) => {
      const n = a.units.length;
      const go = () => {
        if (a.x === null) return;
        m.close(null);
        centerOn?.(a.x, a.y);
        select?.(a.units);
      };
      return h(
        "div.army-card",
        { "data-army": a.id },
        h(
          "div.army-title",
          h("input.army-name", {
            value: a.name,
            maxlength: 40,
            "aria-label": "Name",
            title: "Rename the army (Enter)",
            onchange: (ev) => {
              const name = ev.target.value.trim();
              if (name && name !== a.name) act({ c: "army_rename", army: a.id, name });
            },
          }),
          h("span", ` ${n} unit${n === 1 ? "" : "s"}`),
          a.x !== null ? h("button.link.small", { onclick: go, title: "Select the army on the map" }, `at (${a.x}, ${a.y})`) : null,
          a.disband_to ? h("span.dim", ` · disbanded: marching to ${a.disband_to}`) : null,
        ),
        h("div.dim", composition(a.units) || "—"),
        // Supply (docs/SUPPLY.md, section 5): the state in bare words.
        h(`div.army-supply${a.supply?.state === "supplied" && (a.supply?.line || a.supply?.raiding || a.supply?.home) ? "" : ".warn"}`, { title: `Supply line (outside your borders): the cheapest path over land to one of your cities or ships, of cost ${r.supply_budget} at most. Cut: Hunger, then Starvation.` }, a.supply?.text ?? ""),
        // Raids (docs/RAIDS.md, section 8): its Raid order, and Stop raiding.
        a.station?.kind === "raid"
          ? h("div.row.raid-station", h("span.warn", a.station.text), " ", h("button.small.stop-raid", { onclick: () => act({ c: "army_station", army: a.id, station: null }), title: "End the Raid order. A move order also ends it." }, "Stop raiding"))
          : null,
        // Trade (docs/TRADE.md 4.4): Escort of a caravan, where the Raid order's line is.
        a.station?.kind === "raid" ? null : escortControl(a, (station) => act({ c: "army_station", army: a.id, station })),
        h(
          "div.row",
          h(
            "button.small",
            { disabled: !a.upgrades, onclick: () => act({ c: "army_upgrade", army: a.id }), title: a.upgrades ? "Upgrade every unit that can, the most experienced first, while the gold lasts" : "No unit of the army has an upgrade" },
            a.upgrades ? `Upgrade all: ${a.upgrades} (${a.upgrade_price} gold)` : "Upgrade all",
          ),
          a.disband_to ? null : h("button.small.danger", { onclick: () => disband(a), title: "Its units march to the nearest of your cities and join the garrison there" }, "Disband"),
        ),
        ...waypointCard(a, r.garrisons, act, goUnit),
        squadsView(a, load),
      );
    });
    // Fleets (docs/NAVAL.md, section 7).
    const fleets = (r.fleets ?? []).map((f) => {
      const n = f.units.length;
      const go = () => {
        if (f.x === null) return;
        m.close(null);
        centerOn?.(f.x, f.y);
        select?.(f.units);
      };
      return h(
        "div.army-card.fleet-card",
        { "data-army": f.id },
        h(
          "div.army-title",
          h("input.army-name", {
            value: f.name,
            maxlength: 40,
            "aria-label": "Name",
            title: "Rename the fleet (Enter)",
            onchange: (ev) => {
              const name = ev.target.value.trim();
              if (name && name !== f.name) act({ c: "army_rename", army: f.id, name });
            },
          }),
          h("span", ` ${n} ship${n === 1 ? "" : "s"}`),
          f.x !== null ? h("button.link.small", { onclick: go, title: "Select the fleet on the map" }, `at (${f.x}, ${f.y})`) : null,
          f.sea ? h("span.dim", ` · ${f.sea}`) : null,
          f.disband_to ? h("span.dim", ` · disbanded: sailing to ${f.disband_to}`) : null,
        ),
        h("div.dim", composition(f.units) || "—"),
        h(`div.fleet-wear${f.wear ? ".warn" : ".dim"}`, f.wear_text ?? ""),
        f.disband_to ? h("div.dim", f.station?.text ?? "") : h("div.row", stationControl(f, r.seas ?? [], r.escort_targets ?? [], (station) => act({ c: "army_station", army: f.id, station }))),
        // Trade (docs/TRADE.md 4.4): an Escort of a caravan says what it escorts and why it waits.
        f.station?.kind === "escort" && f.station.route !== undefined && f.station.route !== null ? h("div.dim.escort-station-line", f.station.text, f.station.wait ? h("span.warn", ` ${f.station.wait}`) : null) : null,
        h(
          "div.row",
          h(
            "button.small",
            { disabled: !f.upgrades, onclick: () => act({ c: "army_upgrade", army: f.id }), title: f.upgrades ? "Upgrade every ship that can, the most experienced first, while the gold lasts" : "No ship of the fleet has an upgrade" },
            f.upgrades ? `Upgrade all: ${f.upgrades} (${f.upgrade_price} gold)` : "Upgrade all",
          ),
          f.disband_to ? null : h("button.small.danger", { onclick: () => disband(f), title: "Its ships sail to the nearest of your ports, in no fleet" }, "Disband"),
        ),
        ...waypointCard(f, r.garrisons, act, goUnit),
      );
    });
    const garrisons = r.garrisons.map((gr) =>
      h(
        "li",
        { "data-city": gr.city },
        h("b", gr.name),
        h("span.dim", ` · ${gr.units.length ? composition(gr.units) : "no garrison"}`),
      ),
    );
    m.setBody(
      h("h4", `Armies (${r.armies.length})`),
      armies.length ? h("div.army-list", armies) : h("p.dim", "None."),
      h("h4", `Fleets (${fleets.length})`),
      fleets.length ? h("div.army-list.fleet-list", fleets) : h("p.dim", "None."),
      h("h4", "Garrisons"),
      garrisons.length ? h("ul.garrisons", garrisons) : h("p.dim", "No cities."),
      ...fallenSections(r),
    );
  };
  await load();
  return m.done;
}

/** The dead the rules remember: heroes on their way back to the capital (ffh-core's
 *  unit/fallen.rs), then the other dead within Resurrection's reach. Shown only when there are some. */
function fallenSections(r) {
  const out = [];
  const back = r.recovering ?? [];
  if (back.length) {
    out.push(
      h("h4", `Heroes recovering (${back.length})`),
      h(
        "ul.fallen-list.recovering",
        back.map((x) =>
          h(
            "li",
            h("b", x.name),
            h("span.dim", ` · level ${x.level}`),
            ` · ${x.city ? `returns to ${x.city} in ${x.turns === 1 ? "1 turn" : `${x.turns} turns`}` : "returns when you have a city"}`,
            h("span.warn", ` · Wounds after: ${x.wounds}`),
          ),
        ),
      ),
    );
  }
  const dead = r.fallen ?? [];
  if (dead.length) {
    out.push(
      h("h4", { title: "A Life III caster's Resurrection brings one back, with its promotions and experience." }, `Fallen, within Resurrection's reach (${dead.length})`),
      h(
        "ul.fallen-list",
        dead.map((x) => h("li", h("b", x.name), h("span.dim", ` · level ${x.level} · died turn ${x.died} · ${x.turns === 1 ? "1 turn" : `${x.turns} turns`} left`))),
      ),
    );
  }
  return out;
}

/** The city screen's garrison line: the land combat units in the city's garrison
 *  (`c.army_garrison`), or null without the field. */
export function garrisonView(c) {
  const ids = c.army_garrison;
  if (!ids) return null;
  return h("div.army-garrison.dim", `Garrison: ${ids.length ? composition(ids) : "none"}`);
}
