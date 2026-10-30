// Armies (this game's own rules, docs/ARMIES.md): the Armies screen (F5) and the city screen's
// recruitment controls.
//
// Land combat units serve in armies (named, moving as one) or in a city's garrison. A trained
// unit does not appear in its city: it travels off the map to its destination (the city's
// garrison, another city's, or an army) and joins it there. A reinforcement advances only while
// its army is in supply (within the supply range of your network of cities); after a few turns
// out of supply, or when the army is gone, it goes to the nearest garrison instead. The `armies`
// query lists the armies and garrisons with the units on their way; the buttons send
// `army_rename`, `army_target`, `army_upgrade`, `army_disband` and `recruit_dest`, which the
// server checks.

import * as api from "./api.js";
import { S, dname, turns } from "./state.js";
import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";

/** "Axeman from Bannor · 3 turns" for a unit on its way. */
function transitLine(t, { to = false } = {}) {
  return h(
    "li",
    h("span", dname("UnitType", t.unit)),
    t.from ? h("span.dim", ` from ${t.from}`) : null,
    to ? h("span.dim", ` to ${t.to}`) : null,
    t.turns_left === null ? h("span.warn", " · no way there yet") : h("span.dim", ` · ${t.turns_left ? turns(t.turns_left) : "arrives"}`),
    t.stalled ? h("span.warn", { title: "Its army is out of supply: no progress" }, ` · waiting ${turns(t.stalled)}`) : null,
  );
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

/** The Armies screen (F5): every army (name, units, place, supply, target size, reinforcements on
 *  their way) and every garrison with the units on their way to it. */
export async function openArmies({ select, centerOn } = {}) {
  const m = openModal({ title: "Armies", body: [h("p.dim", "…")], tag: "armies", wide: true, buttons: [{ label: "Close", value: null }] });
  const load = async () => {
    let r;
    try {
      r = await api.query("armies");
    } catch (e) {
      m.setBody(h("p.warn", e.detail ?? e.message));
      return;
    }
    if (m.closed) return;
    if (!r.on) {
      m.setBody(h("p.dim", "This game has no armies (the Armies option is off)."));
      return;
    }
    const act = async (cmd) => {
      if (await run(cmd)) load();
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
          a.in_supply ? h("span.badge", { title: `Within ${r.supply_range} plots of your network of cities` }, "in supply") : h("span.badge.warn", { title: `More than ${r.supply_range} plots from your network of cities: its reinforcements wait, and after ${turns(r.stall_turns)} go to the nearest garrison` }, "out of supply"),
          a.disband_to ? h("span.dim", ` · disbanded: marching to ${a.disband_to}`) : null,
        ),
        h("div.dim", composition(a.units) || "—"),
        h(
          "div.row",
          h(
            "label",
            { title: "The size the army is meant to have (0: none); the AI sets its own armies' to what its war needs" },
            "Target size ",
            h("input.army-target", {
              type: "number",
              min: 0,
              max: 99,
              value: a.target_size,
              onchange: (ev) => act({ c: "army_target", army: a.id, size: Math.max(0, Math.min(99, Number(ev.target.value) || 0)) }),
            }),
          ),
          a.target_size ? h("span.dim", ` ${n + a.transit.length}/${a.target_size} with the reinforcements`) : null,
          h(
            "button.small",
            { disabled: !a.upgrades, onclick: () => act({ c: "army_upgrade", army: a.id }), title: a.upgrades ? "Upgrade every unit that can, the most experienced first, while the gold lasts" : "No unit of the army has an upgrade" },
            a.upgrades ? `Upgrade all: ${a.upgrades} (${a.upgrade_price} gold)` : "Upgrade all",
          ),
          a.disband_to ? null : h("button.small.danger", { onclick: () => act({ c: "army_disband", army: a.id }), title: "Its units march to the nearest of your cities and join the garrison there" }, "Disband"),
        ),
        a.transit.length ? h("div", h("b", `On their way (${a.transit.length})`), h("ul.army-transit", a.transit.map((t) => transitLine(t)))) : null,
      );
    });
    const garrisons = r.garrisons.map((gr) =>
      h(
        "li",
        h("b", gr.name),
        h("span.dim", ` · ${gr.units.length ? composition(gr.units) : "no garrison"}`),
        gr.transit.length ? h("ul.army-transit", gr.transit.map((t) => transitLine(t))) : null,
      ),
    );
    m.setBody(
      h(
        "p.dim",
        "Land combat units serve in armies, which move as one, or in a city's garrison. A unit a city trains travels off the map to the destination its order names and joins it there; it advances only while its army is in supply. Heroes, mages, recon units and summons move on their own.",
      ),
      h("h4", `Armies (${r.armies.length})`),
      armies.length ? h("div.army-list", armies) : h("p.dim", "None: units outside your cities form armies; select some and choose Form a new army."),
      h("h4", "Garrisons"),
      garrisons.length ? h("ul.garrisons", garrisons) : h("p.dim", "No cities."),
    );
  };
  await load();
  return m.done;
}

const key = (dest) => JSON.stringify(dest ?? null);

/** The name of the city's default destination (its own garrison when none is set). */
function defaultName(c) {
  const rc = c.recruit;
  return rc.options.find((o) => key(o.dest) === key(rc.default))?.name ?? "this city's garrison";
}

/** A production order's destination controls (an order for a land combat unit: the queue's
 *  `dest`), or null: where the unit goes, and with an army "Reinforce to N" (the order repeats
 *  until the army, with the units on their way, counts N). */
export function recruitControls(c, i, o, reload) {
  if (!c.recruit || o.dest === undefined) return null;
  const send = async (dest, reinforce) => {
    if (await run({ c: "recruit_dest", city: c.id, index: i, dest, reinforce_to: reinforce })) reload();
  };
  const select = h(
    "select.recruit-dest",
    {
      "aria-label": "Destination",
      title: "Where the unit goes when trained (it travels off the map and joins it)",
      onchange: (ev) => {
        const dest = JSON.parse(ev.target.value);
        send(dest, dest?.army !== undefined ? o.reinforce_to : 0);
      },
    },
    h("option", { value: "null", selected: o.dest === null }, `→ default (${defaultName(c)})`),
    c.recruit.options.map((opt) => h("option", { value: key(opt.dest), selected: o.dest !== null && key(opt.dest) === key(o.dest) }, `→ ${opt.name}`)),
  );
  const army = o.dest?.army !== undefined;
  const reinforce = army
    ? h(
        "label.small",
        { title: "Repeat the order until the army, with its units on their way, counts this many (0: once)" },
        " reinforce to ",
        h("input.recruit-reinforce", {
          type: "number",
          min: 0,
          max: 99,
          value: o.reinforce_to,
          onchange: (ev) => send(o.dest, Math.max(0, Math.min(99, Number(ev.target.value) || 0))),
        }),
      )
    : null;
  return h("span.recruit", select, reinforce);
}

/** The city's default destination for the land combat units it trains, and its garrison. */
export function recruitView(c, reload) {
  const rc = c.recruit;
  if (!rc) return null;
  const set = async (dest) => {
    if (await run({ c: "recruit_dest", city: c.id, index: -1, dest, reinforce_to: 0 })) reload();
  };
  return h(
    "div.recruit-default",
    h("span", "Units trained here go to "),
    h(
      "select.recruit-default-dest",
      { "aria-label": "Default destination", title: "The default destination of the land combat units this city trains", onchange: (ev) => set(JSON.parse(ev.target.value)) },
      rc.options.map((opt) => h("option", { value: key(opt.dest), selected: key(opt.dest) === key(rc.default ?? rc.options[0].dest) }, opt.name)),
    ),
    h("span.dim", ` · garrison: ${rc.garrison.length ? composition(rc.garrison) : "none"}`),
  );
}
