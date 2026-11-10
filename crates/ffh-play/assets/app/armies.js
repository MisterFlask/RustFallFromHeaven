// Armies (this game's own rules, docs/ARMIES.md): the Armies screen (F5) and the city screen's
// recruitment controls.
//
// Land combat units serve in armies (named, moving as one) or in a city's garrison. An army
// requisitions soldiers from the cities you pick for it while it is short of its target size:
// each picked city trains one unit for it at a time, first in its queue, ahead of what it was
// building (which keeps its progress). A unit a city trains for an army or another city's garrison
// appears in its city as any unit and marches there on its own: you see it on the map but cannot
// order it until it arrives. Toward an army it advances only while the army is in supply; after a
// few turns without progress it goes to the nearest garrison instead. The `armies` query lists
// the armies (with the cities each may requisition from) and the garrisons with the recruits
// marching to them; the buttons send `army_rename`, `army_target`, `army_requisition`,
// `army_requisition_set`, `army_upgrade`, `army_disband`, `army_tactic`, `garrison_tactic` and
// `recruit_dest`, which the server checks.
//
// Each army and each garrison has a battle tactic (docs/TACTICS.md): how its units are arranged
// in every battle it fights. A change takes effect at the start of your next turn. Each army's
// supply (docs/SUPPLY.md) is stated in the view's bare words (`supply.text`).

import * as api from "./api.js";
import { S, dname, turns, info, cityAt, me } from "./state.js";
import { plain } from "./markup.js";
import { h, errorText } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";

// ---- the rules, in the help display (help.js; the Armies screen's (i) opens it) ---------------

/** The turns a recruit waits for its army before it goes to a garrison, as the last `armies`
 *  query said (null before one). */
let stallTurns = null;

addHelp({
  id: "armies",
  group: "Game options",
  title: "Armies and garrisons",
  body: () => [
    h("p", "Land combat units serve in armies, which move as one, or in a city's garrison. Heroes, mages, recon units and summons move on their own."),
    h("h4", "Forming armies"),
    h(
      "ul",
      h("li", "Units outside your cities form armies: select some and choose Form a new army. An army can merge into another; the units on their way to it follow."),
      h("li", "In a city, an army's units can join its garrison; a garrison unit that leaves the city forms an army again. A disbanded army marches to the nearest of your cities and joins its garrison there."),
    ),
    h("h4", "Requisition"),
    h(
      "ul",
      h("li", "An army requisitions soldiers from the cities you pick for it (the Armies screen [F5]) while it is short of its target size; without a target size it requisitions nothing."),
      h("li", "Each picked city trains one unit for it at a time, first in its queue (placed as your turn ends), in a governed city too: what the city was building waits, with its progress kept."),
      h("li", "Releasing a city withdraws its order for the army."),
    ),
    h("h4", "Recruits on the march"),
    h(
      "ul",
      h("li", "A city sends the land combat units it trains to its default destination (its own garrison unless you choose another city's or an army), or to the destination of each order in its queue."),
      h("li", "A unit trained for an army or another city's garrison marches there on its own: it shows on the map, but takes no orders until it arrives."),
      h("li", `Toward an army it advances only while the army is supplied; after ${stallTurns ? turns(stallTurns) : "a few turns"} without progress it goes to the nearest garrison instead.`),
    ),
    h("h4", "Supply lines"),
    h(
      "ul",
      h("li", "Each army needs a supply line: the cheapest path over land to one of your cities, or to one of your ships that can carry land units, within its budget. A road plot costs least, then your own territory; other land costs more. Enemy units, enemy cities, land next to a manned enemy fort and closed borders block it."),
      h("li", "The map draws your armies' lines, and the lines of other armies you see over the plots you have revealed. With an army selected, the plot under the pointer says whether it would be supplied there."),
      h("li", "Cut at the start of your turn: Hunger (no healing, no recruits), after a few turns Starvation (units lose HP each turn down to a floor, and deal less damage in battles). A restored line ends both at your next turn."),
    ),
    h("h4", "Battle tactics"),
    h(
      "ul",
      h("li", "Each army and each city's garrison has a battle tactic (the Armies screen, the city screen): which of its units fight when the slots are few and where they stand. Every battle it fights, attacking or defending, is arranged by it; the battle plan shows the result."),
      h("li", "A change takes effect at the start of your next turn. A side's tactic in a battle is that of the army (or garrison) most of its units belong to."),
    ),
  ],
});

/** How far a marching recruit is from its destination: "3 turns away", "arrives", "waiting 2
 *  turns" (no progress: its army out of supply, the way blocked) or "no way there now". `m` is a
 *  unit's `march` (or a recruit of the Armies screen). */
export function marchState(m) {
  if (m.stalled) return { text: `waiting ${turns(m.stalled)}`, warn: true, title: "No progress: its army is out of supply, or the way is blocked. After a few turns it goes to the nearest garrison instead." };
  if (m.turns_left === null || m.turns_left === undefined) return { text: "no way there now", warn: true, title: "No path by land now (it waits, then goes to the nearest garrison)" };
  return { text: m.turns_left ? `${turns(m.turns_left)} away` : "arrives", warn: false, title: null };
}

/** "Marching to the 1st Army (3 turns)" for a unit's `march`. */
export function marchLine(m) {
  const when = m.stalled ? `waiting ${turns(m.stalled)}` : m.turns_left === null || m.turns_left === undefined ? "no way there now" : m.turns_left ? turns(m.turns_left) : "arrives";
  return `Marching to ${m.to} (${when})`;
}

/** A recruit marching (the Armies screen's `recruits`): "Axeman from Ithralia at (12, 5) · 3 turns
 *  away"; a click on the place selects it on the map. `to`: also say where it goes. */
function recruitLine(t, { to = false, go } = {}) {
  const st = marchState(t);
  return h(
    "li",
    { "data-unit": t.id },
    h("span", dname("UnitType", t.unit)),
    t.from ? h("span.dim", ` from ${t.from}`) : null,
    to ? h("span.dim", ` to ${t.to}`) : null,
    " ",
    go ? h("button.link.small", { onclick: () => go(t), title: "Select it on the map (it takes no orders until it arrives)" }, `at (${t.x}, ${t.y})`) : h("span.dim", `at (${t.x}, ${t.y})`),
    h(st.warn ? "span.warn" : "span.dim", { title: st.title }, ` · ${st.text}`),
    t.diverted ? h("span.dim", { title: "Sent to the nearest garrison instead of its first destination" }, " · diverted") : null,
  );
}

/** The battle tactics of the data (docs/TACTICS.md): `[{i, name, help}]` (help without markup). */
function tactics() {
  return (S.data?.tables?.BattleTacticType ?? []).map((t, i) => ({ i, name: t.name, help: t.help ? plain(t.help) : "" }));
}

/** A tactic selector (four options, each with its help as its tooltip) for `now` (the tactic in
 *  force) and `next` (the one from the next turn; a pending change says "from next turn"); `set`
 *  sends the choice. `cls`: the select's class. */
export function tacticControl(now, next, set, cls, label = "Tactic") {
  const list = tactics();
  const help = list[next]?.help ?? "";
  return h(
    "span.tactic",
    h("span", `${label} `),
    h(
      `select.${cls}`,
      { "aria-label": label, title: help, onchange: (ev) => set(Number(ev.target.value)) },
      list.map((t) => h("option", { value: String(t.i), selected: t.i === next, title: t.help }, t.name)),
    ),
    next !== now ? h("span.dim.tactic-pending", { title: "A change of tactic takes effect at the start of your next turn" }, ` from next turn (now ${info("BattleTacticType", now)?.name ?? "-"})`) : null,
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

/** "Axeman · 4 turns to train · 3 turns' march" for a city the army may requisition from. */
function candidateLine(c) {
  if (c.unit === null || c.unit === undefined) return h("span.dim", " · trains no unit for the army now");
  const march = c.march_turns === null || c.march_turns === undefined ? h("span.warn", { title: "No path by land to the army now" }, " · no way by land") : h("span.dim", ` · ${c.march_turns ? `${turns(c.march_turns)}' march` : "at the army"}`);
  return h("span", h("span.dim", ` · ${dname("UnitType", c.unit)}, ${turns(c.production_turns)} to train`), march);
}

/** An army's requisition on the Armies screen: the cities it requisitions from, its orders in
 *  progress, and every city it may requisition from with a checkbox (`act` sends a command and
 *  re-reads the screen). */
function requisitionView(a, act) {
  const rq = a.requisition;
  if (a.disband_to) return null;
  const picked = rq.candidates.filter((c) => c.picked);
  const names = picked.map((c) => c.name);
  const head = h(
    "div.requisition-head",
    h("b", "Requisition"),
    names.length ? h("span", ` from ${names.join(", ")}`) : h("span.dim", " from no city"),
    !a.target_size && names.length ? h("span.warn", { title: "An army requisitions only while it is short of its target size" }, " · no target size: it requisitions nothing") : null,
    a.target_size && names.length ? (rq.shortfall > 0 ? h("span.dim", ` · ${rq.shortfall} more wanted`) : h("span.dim", " · nothing wanted now")) : null,
  );
  const orders = rq.orders.length
    ? h(
        "ul.requisition-orders",
        rq.orders.map((o) => h("li", { "data-city": o.city }, h("span", `${dname("UnitType", o.unit)} in ${o.name}`), h("span.dim", ` · ${turns(o.turns)}`), o.index ? h("span.dim", { title: "Behind another army's requisition in that city" }, ` · ${o.index + 1}. in the queue`) : null)),
      )
    : null;
  const toggle = (c, on) => act({ c: "army_requisition", army: a.id, city: c.city, on });
  const rows = rq.candidates.map((c) =>
    h(
      "label.requisition-city",
      { "data-city": c.city, title: c.picked ? "Release the city from the army's requisition (its order for the army is withdrawn)" : "Requisition from this city: while the army is short, the city trains a unit for it first" },
      h("input", { type: "checkbox", checked: c.picked, disabled: !rq.open, onchange: (ev) => toggle(c, ev.target.checked) }),
      h("span", ` ${c.name}`),
      c.governed ? h("span.dim", " (governed)") : null,
      candidateLine(c),
      c.order ? h("span.badge", { title: "Training for the army now" }, `${turns(c.order.turns)}`) : null,
    ),
  );
  const pick = h(
    "details.requisition-pick",
    { open: !names.length && !!a.target_size },
    h("summary", { title: "While the army is short of its target size, each city you pick trains one unit for it at a time, first in its queue (placed as your turn ends): what the city was building waits, with its progress kept. The unit marches to the army." }, "Requisition from…"),
    h("div.requisition-cities", rows),
    picked.length ? h("button.small", { onclick: () => act({ c: "army_requisition_set", army: a.id, cities: [] }), title: "Release every city (their orders for the army are withdrawn)" }, "Release all") : null,
  );
  return h("div.requisition", head, orders, pick);
}

/** The Armies screen (F5): every army (name, units, place, supply, target size, requisition,
 *  recruits marching to it) and every garrison with the recruits marching to it. */
export async function openArmies({ select, centerOn } = {}) {
  const m = openModal({ title: "Armies", body: [h("p.dim", "…")], tag: "armies", wide: true, buttons: [{ label: "Close", value: null }], help: "armies" });
  // The open requisition pickers (by army) stay open when the screen is re-read.
  const opened = new Set();
  const load = async () => {
    let r;
    try {
      r = await api.query("armies");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    for (const d of document.querySelectorAll("#modals .army-card details.requisition-pick")) {
      const id = Number(d.closest(".army-card")?.dataset.army);
      if (d.open) opened.add(id);
      else opened.delete(id);
    }
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
      const v = await ask("Disband the army?", `Disband the ${a.name}? Its ${one ? "unit" : `${n} units`} ${where}; its requisition ends.`, [
        { label: "Disband", value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      if (v) act({ c: "army_disband", army: a.id });
    };
    const goUnit = (t) => {
      m.close(null);
      centerOn?.(t.x, t.y);
      select?.([t.id]);
    };
    const armies = r.armies.map((a) => {
      const n = a.units.length;
      const go = () => {
        if (a.x === null) return;
        m.close(null);
        centerOn?.(a.x, a.y);
        select?.(a.units);
      };
      // (As the shortfall counts them: the requisition's orders and the cities' own next units
      // bound for the army.)
      const coming = a.marching.length + a.requisition.orders.length + (a.requisition.head_orders ?? 0);
      const req = requisitionView(a, act);
      if (req && opened.has(a.id)) req.querySelector("details.requisition-pick")?.setAttribute("open", "");
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
        h(`div.army-supply${a.supply?.state === "supplied" && a.supply?.line ? "" : ".warn"}`, { title: `Supply line: the cheapest path over land to one of your cities or ships, of cost ${r.supply_budget} at most. Cut: Hunger, then Starvation.` }, a.supply?.text ?? ""),
        h("div.row", tacticControl(a.tactic, a.tactic_next, (t) => act({ c: "army_tactic", army: a.id, tactic: t }), "army-tactic")),
        h(
          "div.row",
          h(
            "label",
            { title: "The size the army is meant to have (0: none). While it is short, it requisitions from the cities you pick. The AI sets its own armies' to what its war needs." },
            "Target size ",
            h("input.army-target", {
              type: "number",
              min: 0,
              max: 99,
              value: a.target_size,
              onchange: (ev) => act({ c: "army_target", army: a.id, size: Math.max(0, Math.min(99, Number(ev.target.value) || 0)) }),
            }),
          ),
          a.target_size ? h("span.dim", { title: "Its units, the recruits marching to it and the units in training for it (its requisition's, and the units cities build next for it)" }, ` ${n + coming}/${a.target_size} with ${coming ? `${coming} coming` : "none coming"}`) : null,
          h(
            "button.small",
            { disabled: !a.upgrades, onclick: () => act({ c: "army_upgrade", army: a.id }), title: a.upgrades ? "Upgrade every unit that can, the most experienced first, while the gold lasts" : "No unit of the army has an upgrade" },
            a.upgrades ? `Upgrade all: ${a.upgrades} (${a.upgrade_price} gold)` : "Upgrade all",
          ),
          a.disband_to ? null : h("button.small.danger", { onclick: () => disband(a), title: "Its units march to the nearest of your cities and join the garrison there; its requisition ends" }, "Disband"),
        ),
        req,
        a.recruits.length ? h("div", h("b", `Marching to it (${a.recruits.length})`), h("ul.army-transit", a.recruits.map((t) => recruitLine(t, { go: goUnit })))) : null,
      );
    });
    const garrisons = r.garrisons.map((gr) =>
      h(
        "li",
        { "data-city": gr.city },
        h("b", gr.name),
        h("span.dim", ` · ${gr.units.length ? composition(gr.units) : "no garrison"}`),
        gr.recruits.length ? h("ul.army-transit", gr.recruits.map((t) => recruitLine(t, { go: goUnit }))) : null,
      ),
    );
    stallTurns = r.stall_turns ?? null;
    m.setBody(
      h("h4", `Armies (${r.armies.length})`),
      armies.length ? h("div.army-list", armies) : h("p.dim", "None."),
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

/** "Requisitioned by the 1st Army" for a requisition order of the queue (it cannot be removed by
 *  hand: the city is released on the Armies screen). */
export function requisitionTag(o) {
  return h(
    "span.requisition-tag",
    { title: "An army's requisition: it goes first, what the city was building waits with its progress kept. Release the city from the army's requisition on the Armies screen [F5] to take it off." },
    ` Requisitioned by ${o.dest_name}`,
  );
}

/** A production order's destination control (an order for a land combat unit: the queue's
 *  `dest`), or null: where the unit goes when trained (bound elsewhere, it marches there). A
 *  requisition order goes to its army: its tag instead. */
export function recruitControls(c, i, o, reload) {
  if (!c.recruit || o.dest === undefined) return null;
  if (o.requisition) return requisitionTag(o);
  const send = async (dest) => {
    if (await run({ c: "recruit_dest", city: c.id, index: i, dest })) reload();
  };
  const select = h(
    "select.recruit-dest",
    {
      "aria-label": "Destination",
      title: "Where the unit goes when trained (another city's garrison or an army: it marches there)",
      onchange: (ev) => send(JSON.parse(ev.target.value)),
    },
    h("option", { value: "null", selected: o.dest === null }, `→ default (${defaultName(c)})`),
    c.recruit.options.map((opt) => h("option", { value: key(opt.dest), selected: o.dest !== null && key(opt.dest) === key(o.dest) }, `→ ${opt.name}`)),
  );
  return h("span.recruit", select);
}

/** The armies that requisition from the city ("The 1st Army requisitions from this city"), or
 *  null. */
export function requisitionNote(c) {
  const by = c.recruit?.requisitioned_by ?? [];
  if (!by.length) return null;
  const names = by.map((a) => `the ${a.name}${a.target_size ? (a.shortfall > 0 ? ` (${a.shortfall} more wanted)` : " (no more wanted now)") : " (no target size)"}`);
  return h("div.requisition-note.dim", { title: "While an army is short of its target size, the city trains a unit for it first. Release the city on the Armies screen [F5]." }, `Requisitioned by ${names.join(", ")}.`);
}

/** The city's default destination for the land combat units it trains, and its garrison. */
export function recruitView(c, reload) {
  const rc = c.recruit;
  if (!rc) return null;
  const set = async (dest) => {
    if (await run({ c: "recruit_dest", city: c.id, index: -1, dest })) reload();
  };
  return h(
    "div.recruit-default",
    h("span", "Units trained here go to "),
    h(
      "select.recruit-default-dest",
      { "aria-label": "Default destination", title: "The default destination of the land combat units this city trains (bound elsewhere, they march there)", onchange: (ev) => set(JSON.parse(ev.target.value)) },
      rc.options.map((opt) => h("option", { value: key(opt.dest), selected: key(opt.dest) === key(rc.default ?? rc.options[0].dest) }, opt.name)),
    ),
    h("span.dim", ` · garrison: ${rc.garrison.length ? composition(rc.garrison) : "none"}`),
    // The garrison's battle tactic (docs/TACTICS.md).
    h("div.garrison-tactic-row", tacticControl(rc.garrison_tactic, rc.garrison_tactic_next, async (t) => {
      if (await run({ c: "garrison_tactic", city: c.id, tactic: t })) reload();
    }, "garrison-tactic", "Garrison tactic")),
    requisitionNote(c),
  );
}
