// Crusades and allied loans (docs/CRUSADE.md, "Human play"), drawn for the default HD-2D map
// and its floating interface (isoui.js), which the classic interface shares:
//
// - the call to arms: a deferred popup (`pending.crusade_calls`) with the target's leader,
//   pledge or refuse;
// - lending to a crusade: the lend dialog, from the call or the Crusades screen, picks the units;
// - the Crusades advisor (the rail's crossed swords, shown only in a game with crusades, marked
//   while a call waits): each crusade with its Host's banner colour, your part in it, and "Show"
//   buttons that close the screen and bring the HD-2D camera to the Host's army or the target;
// - allied loans: troops are an item of the trade screen (diplo.js), priced by the rest of the
//   deal; adding them opens the troops screen, where you pick the units you lend.
//
// Strengths are the rules' (combat strength x100, scaled by health), shown as combat strength: a
// healthy warrior is 2.

import * as api from "./api.js";
import { S, dname, teamName } from "./state.js";
import { h, toast } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { addHelp } from "./help.js";
import { run } from "./commands.js";
import { playerPortrait, PORTRAIT } from "./portraits.js";
import { isoSkin, unlockIcon } from "./isoui.js";
import { playerColor } from "./tiles.js";

/** The map hooks (main.js `init`): `centerOn(x, y)` brings the camera to a plot. */
const hooks = { centerOn: null };

export function init({ centerOn } = {}) {
  hooks.centerOn = centerOn ?? null;
}

/** A strength for people (the rules count x100). */
export function strengthLabel(s) {
  const v = s / 100;
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
}

/** Who called crusade `kind` (`crusades` query `kind` object, or a pending call's `source`). */
function callerName(source) {
  return source !== null && source !== undefined ? dname("VoteSourceType", source) : "An ally";
}

/** The leader of team `t` among the players met (the first alive), or null. */
function teamLeader(t) {
  const players = [...S.players.values()].filter((p) => p.team === t);
  return players.find((p) => p.alive) ?? players[0] ?? null;
}

/** A Host's banner colour, as the map flies it. */
function hostSwatch(host) {
  return h("span.swatch.host-swatch", { style: { background: playerColor(host) }, title: "The Host's colour on the map" });
}

/** Closes dialog `m` and brings the map to `at` (`[x, y]`). */
function show(m, at) {
  if (!at) return;
  m.close(null);
  hooks.centerOn?.(at[0], at[1]);
}

// ---- the rules, in the help display (help.js; each dialog's (i) opens its topic) --------------

const GROUP = "Crusades (a rules variant)";
const li = (...parts) => h("li", ...parts);

addHelp({
  id: "crusades",
  group: GROUP,
  title: "Crusades and the call to arms",
  body: () => [
    h("p", "With the new game's Crusades box on, a council's war resolution does not drag its members into the war. It raises a crusade: an army of its own, the Host, made of the units its members lend, which fights the target until the crusade ends."),
    h("h4", "The call"),
    h(
      "ul",
      li("Each member able to fight the war is called, with an obligation: a share of its army, smaller the farther it is from the target."),
      li("You answer with a pledge or a refusal. A pledge is a promise: only the units you lend count, and you have until the deadline to lend them (the Crusades advisor, or the lend dialog after you pledge)."),
      li("No answer by the deadline counts as a refusal."),
    ),
    h("h4", "At the deadline"),
    h(
      "ul",
      li("What you lent is weighed against your obligation."),
      li("A refusal, or nothing lent, defies the council: unhappiness in your cities for some 20 turns (at normal speed), and you lose your loyal membership until you next comply with one of its resolutions."),
      li("Lending less than asked costs the shortfall's share of that unhappiness. Meeting the obligation keeps you a loyal member."),
    ),
  ],
});

addHelp({
  id: "crusades.lending",
  group: GROUP,
  title: "Lending units and the spoils",
  body: () => [
    h(
      "ul",
      li("Lending means war with the crusade's target (you are asked to confirm the declaration)."),
      li("Lent units join the Host where they stand. They cannot be recalled; when the crusade ends the survivors come home with their experience and promotions. A lent hero who falls escapes home instead of dying."),
      li("Only combat land units can be lent: not summoned units, not units aboard a ship or carrying others."),
      li("While lent, a unit's upkeep is the Host's, not yours."),
    ),
    h("h4", "Credit and the spoils"),
    h(
      "ul",
      li("A lent unit earns its strength (at handover) as your credit when it first kills an enemy of the crusade, or when it dies. Units that never fight earn nothing."),
      li("When the crusade ends, its spoils (the cities the Host took and the gold it gathered) are shared by credit: each lender drafts cities up to its share, or takes it as gold."),
    ),
  ],
});

addHelp({
  id: "crusades.host",
  group: GROUP,
  title: "The Host and the Crusades advisor",
  body: () => [
    h(
      "ul",
      li("The Host is an AI player of its own, flying its own colour (the swatch beside a crusade's name). No civilization controls it."),
      li("It musters at the friendly city nearest its objective, marches, besieges and takes cities, and falls back to heal; its state is shown under each crusade."),
      li("It heals in its lenders' and the victim's lands, and fights only the crusade's war."),
    ),
    h("h4", "The Crusades advisor"),
    h(
      "ul",
      li("Delivered: the strength you lent, and how many of your units still serve."),
      li("Credit: the strength of your lent units that have fought, by which the spoils are shared."),
      li("Show the Host / Show the target: close the screen and bring the camera to the Host's largest stack you can see, or to the target's city nearest your capital."),
    ),
  ],
});

addHelp({
  id: "loans",
  group: GROUP,
  title: "Allied loans of troops",
  body: () => [
    h("p", "A loan is a small crusade made by agreement: one civilization lends units to another's war. Troops are an item of the trade screen, on either side of the deal."),
    h(
      "ul",
      li("The units serve a general dedicated to the borrower's war, which neither side controls, until the loan ends. They cannot be recalled, and come home then."),
      li("Lending means war with the enemy the loan is against."),
      li("The price is the rest of the deal: gold, gold per turn, technologies, resources. Anything paid every turn runs as long as the loan; cancelling it ends the loan."),
      li("A loan also ends early when the borrower makes peace with the enemy, the enemy is gone, or lender and borrower go to war."),
      li("The general's captures go to the borrower. When an AI asks you to lend, your advisor picks the units."),
    ),
  ],
});

/** A crusade call (`pending.crusade_calls[]`): pledge a strength or refuse. Resolves "chosen",
 *  "later" or null. */
export async function openCall(call) {
  let result = null;
  const suggested = Math.min(call.obligation, call.lendable) || call.obligation || 100;
  const input = h("input", { type: "number", id: "crusade-pledge", min: 0.5, step: 0.5, value: strengthLabel(suggested) });
  const send = async (strength) => {
    const reply = await run({ c: "crusade_answer", crusade: call.crusade, strength });
    if (!reply) return;
    result = "chosen";
    m.close("chosen");
    if (strength !== null) {
      const now = await ask(
        "Lend units now?",
        `Choose the units now, or later from the Crusades advisor (before turn ${call.deadline}).`,
        [
          { label: "Choose units", value: true, primary: true },
          { label: "Later", value: null },
        ],
      );
      if (now) openLend(call.crusade);
    }
  };
  const t = teamName(call.target);
  const enemy = teamLeader(call.target);
  const portrait = enemy ? playerPortrait(enemy.id, { size: PORTRAIT.medium }) : null;
  const m = openModal({
    title: `Crusade against ${t}`,
    body: [
      h(
        "div.crusade-call",
        portrait ? h("figure.crusade-enemy", portrait, h("figcaption.dim", enemy.name)) : null,
        h(
          "div.crusade-call-text",
          h("p.page-text", `${callerName(call.source)} calls you to send strength ${strengthLabel(call.obligation)} against ${t} by turn ${call.deadline}, to serve until turn ${call.end}.`),
          h("p.dim", `You can lend up to ${strengthLabel(call.lendable)}.`),
        ),
      ),
      h("label.field", h("span", "Pledge strength"), input),
      h(
        "div.options",
        h("button.option", { "data-pledge": "yes", onclick: () => send(Math.max(1, Math.round(Number(input.value) * 100))) }, "Pledge"),
        h("button.option", { "data-pledge": "refuse", title: "Defies the council: unhappiness in your cities, and your loyal membership lost", onclick: () => send(null) }, "Refuse"),
      ),
    ],
    buttons: [{ label: "Later", value: "later", title: "No answer by the deadline counts as a refusal" }],
    tag: "crusade",
    className: "event crusade",
    help: "crusades",
  });
  const r = await m.done;
  return result ?? r;
}

/** The lend dialog for crusade `id`: the units we can lend, with checkboxes (in the HD-2D
 *  interface each with its map figure). */
export async function openLend(id) {
  const m = openModal({ title: "Lend units", body: [h("p.dim", "…")], tag: "crusade-lend", wide: true, buttons: [{ label: "Close", value: null }], help: "crusades.lending" });
  let q;
  try {
    q = await api.query("crusades");
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  const c = q.crusades.find((x) => x.id === id);
  if (!c) {
    m.setBody(h("p.warn", "This crusade has ended."));
    return m.done;
  }
  const pledged = typeof c.call?.answer === "number" ? c.call.answer : 0;
  const units = [...q.lendable].sort((a, b) => b.strength - a.strength || a.unit - b.unit);
  const boxes = units.map((u) => h("input", { type: "checkbox", value: String(u.unit), "data-strength": String(u.strength) }));
  const total = h("p.dim");
  const update = () => {
    const s = boxes.filter((b) => b.checked).reduce((a, b) => a + Number(b.dataset.strength), 0);
    const owed = Math.max(0, pledged - c.delivered);
    total.textContent = `Selected: strength ${strengthLabel(s)}${pledged ? ` (still owed: ${strengthLabel(owed)})` : ""}.`;
  };
  for (const b of boxes) b.addEventListener("change", update);
  const lend = h(
    "button.primary",
    {
      id: "crusade-lend",
      onclick: async () => {
        const chosen = boxes.filter((b) => b.checked).map((b) => Number(b.value));
        if (!chosen.length) return toast("Choose the units to lend", "error");
        const reply = await run({ c: "crusade_lend", crusade: id, units: chosen, declare_war: false });
        if (reply) {
          toast(`${chosen.length} unit${chosen.length === 1 ? "" : "s"} lent`, "info");
          m.close("lent");
        }
      },
    },
    "Lend",
  );
  update();
  const figure = (u) => (isoSkin() ? unlockIcon("units", u.type) : null);
  m.setBody(
    h("p", hostSwatch(c.host), `The Host fights ${teamName(c.target)} until turn ${c.end}.`),
    units.length
      ? h(
          "div.checks.lend-units",
          units.map((u, i) => h("label.check", { "data-unit": u.unit }, boxes[i], figure(u), ` ${dname("UnitType", u.type)} · strength ${strengthLabel(u.strength)} · at (${u.x}, ${u.y})`)),
        )
      : h("p.dim", "None of your units can be lent now."),
    total,
    units.length ? lend : null,
  );
  return m.done;
}

function kindLabel(k) {
  if (k.kind === "council") return dname("VoteSourceType", k.source);
  if (k.kind === "loan") return `Loan general of ${S.players.get(k.borrower)?.name ?? `player ${k.borrower}`}`;
  return "Crusade";
}

/** The Crusades screen (the rail's advisor): running crusades and loan generals, our part in
 *  each, our past crusades. "Show" closes it and brings the map to the Host's army or the
 *  target. */
export async function openCrusades() {
  const m = openModal({
    title: "Crusades",
    body: [h("p.dim", "…")],
    tag: "crusades",
    wide: true,
    buttons: [{ label: "Close", value: null }],
    help: "crusades.host",
  });
  const load = async () => {
    let q;
    try {
      q = await api.query("crusades");
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    if (!q.enabled) return m.setBody(h("p.dim", "Crusades are off in this game (start a new game with Crusades on)."));
    const rows = q.crusades.map((c) => {
      const call = c.call;
      const answer = !call ? "not called" : call.answer === null ? (call.settled ? "no answer" : "waiting for your answer") : call.answer === "refuse" ? "refused" : `pledged ${strengthLabel(call.answer)}`;
      const actions = [];
      if (call && call.answer === null && !call.settled) {
        actions.push(
          h(
            "button.small",
            {
              "data-answer": c.id,
              onclick: () =>
                openCall({ crusade: c.id, source: c.kind.kind === "council" ? c.kind.source : null, target: c.target, obligation: call.obligation, lendable: q.military_strength, deadline: c.deadline, end: c.end }).then(load),
            },
            "Answer…",
          ),
        );
      }
      if (c.can_lend === true) actions.push(h("button.small", { "data-lend": c.id, onclick: () => openLend(c.id).then(load) }, "Lend units…"));
      if (c.host_at) actions.push(h("button.small", { "data-show-host": c.id, title: `The Host's largest stack in view, at (${c.host_at[0]}, ${c.host_at[1]})`, onclick: () => show(m, c.host_at) }, "Show the Host"));
      if (c.target_at) actions.push(h("button.small", { "data-show-target": c.id, title: `The target's city nearest your capital, at (${c.target_at[0]}, ${c.target_at[1]})`, onclick: () => show(m, c.target_at) }, "Show the target"));
      const enemy = teamLeader(c.target);
      return h(
        "tr",
        { "data-crusade": c.id },
        h(
          "td",
          h(
            "div.leader-cell",
            enemy ? playerPortrait(enemy.id, { size: PORTRAIT.small }) : null,
            h("div.crusade-name", h("b", hostSwatch(c.host), kindLabel(c.kind)), h("div.dim", `against ${teamName(c.target)} until turn ${c.end} · ${c.state} · ${c.host_units} in the Host`)),
          ),
          actions.length ? h("div.actions.crusade-actions", actions) : null,
        ),
        h("td", call ? [h("div", `obligation ${strengthLabel(call.obligation)} by turn ${c.deadline}`), h("div.dim", answer)] : h("span.dim", answer)),
        h("td.num", h("div", `${strengthLabel(c.delivered)} · ${c.serving} serving`), h("div.dim", `credit ${strengthLabel(c.credit)}`)),
      );
    });
    const past = q.reports.map((r) =>
      h("li", `${kindLabel(r.kind)} against ${teamName(r.target)}, turns ${r.start}–${r.end} (${r.reason}): delivered ${strengthLabel(r.delivered)}, credit ${strengthLabel(r.credit)}, spoils ${r.spoils} gold-equivalent${r.cities ? `, ${r.cities} cities` : ""}${r.refused ? " (you refused)" : ""}`),
    );
    m.setBody(
      rows.length
        ? h("table.scores.crusades", h("tr", h("th", "Crusade"), h("th", "Your call"), h("th", "Delivered")), rows)
        : h("p.dim", "No crusade is running."),
      past.length ? h("details", h("summary", "Your past crusades"), h("ul", past)) : null,
    );
  };
  load();
  return m.done;
}

/** A loan of troops as the trade screen shows it in the deal. */
export function troopsLabel(k) {
  const who = k.lend ? `${k.units.length} of your units` : "their troops";
  return `Troops: ${who}, strength ${strengthLabel(k.strength)}, ${k.mission === "defend" ? "defending the borrower against" : "attacking"} ${teamName(k.target)} for ${k.duration} turns`;
}

/** The troops screen, opened from the trade screen's Troops item: the terms of a loan of troops
 *  in the deal -- the enemy it is against, the mission, the turns, and (lending) the units you
 *  send, picked here, or (borrowing) the strength you ask for. `way` is the `trade` query's
 *  `troops.lend` or `troops.borrow`; `current`, the terms already in the deal. Resolves with the
 *  terms `{lend, target, mission, duration, strength, units}`, or null. */
export async function openTroops({ lend, other, way, maxTurns, current = null }) {
  const name = S.players.get(other)?.name ?? `player ${other}`;
  const target = h("select", { id: "troops-target" }, way.targets.map((t) => h("option", { value: String(t), selected: current?.target === t }, teamName(t))));
  const mission = h(
    "select",
    { id: "troops-mission" },
    h("option", { value: "attack", selected: current?.mission !== "defend" }, "Attack the enemy"),
    h("option", { value: "defend", selected: current?.mission === "defend" }, "Defend the borrower"),
  );
  const duration = h("input", { type: "number", id: "troops-turns", min: 1, max: maxTurns, value: String(current?.duration ?? Math.min(10, maxTurns)) });
  const err = h("p.error");
  const field = (label, el) => h("label.field", h("span", label), el);
  let units = [];
  let boxes = [];
  let strength = null;
  const total = h("p.dim");
  if (lend) {
    units = [...way.units].sort((a, b) => b.strength - a.strength || a.unit - b.unit);
    boxes = units.map((u) => h("input", { type: "checkbox", value: String(u.unit), "data-strength": String(u.strength), checked: !!current?.units?.includes(u.unit) }));
    const update = () => {
      const s = boxes.filter((b) => b.checked).reduce((a, b) => a + Number(b.dataset.strength), 0);
      total.textContent = `Selected: strength ${strengthLabel(s)}.`;
    };
    for (const b of boxes) b.addEventListener("change", update);
    update();
  } else {
    strength = h("input", { type: "number", id: "troops-strength", min: 0.5, step: 0.5, value: current ? strengthLabel(current.strength) : "4" });
  }
  const figure = (u) => (isoSkin() ? unlockIcon("units", u.type) : null);
  const m = openModal({
    title: lend ? `Lend troops to ${name}` : `Ask ${name} for troops`,
    help: "loans",
    body: [
      lend ? null : h("p.dim", `${name} picks the units it sends.`),
      h("div.fields", field("Against", target), field("Mission", mission), field(`Turns (up to ${maxTurns})`, duration), lend ? null : field("Strength", strength)),
      lend
        ? units.length
          ? h(
              "div.checks.lend-units",
              units.map((u, i) => h("label.check", { "data-unit": u.unit }, boxes[i], figure(u), ` ${dname("UnitType", u.type)} · strength ${strengthLabel(u.strength)} · at (${u.x}, ${u.y})`)),
            )
          : h("p.dim", "None of your units can be lent now.")
        : null,
      lend ? total : null,
      err,
    ],
    buttons: [
      {
        label: "Put in the deal",
        primary: true,
        value: "ok",
        onClick: async (dialog) => {
          const turns = Math.max(1, Math.min(maxTurns, Math.round(Number(duration.value) || 1)));
          let terms;
          if (lend) {
            const chosen = boxes.filter((b) => b.checked);
            if (!chosen.length) {
              err.textContent = "Choose the units to lend.";
              return;
            }
            terms = { lend, target: Number(target.value), mission: mission.value, duration: turns, units: chosen.map((b) => Number(b.value)), strength: chosen.reduce((a, b) => a + Number(b.dataset.strength), 0) };
          } else {
            terms = { lend, target: Number(target.value), mission: mission.value, duration: turns, units: [], strength: Math.max(1, Math.round(Number(strength.value) * 100)) };
          }
          dialog.close(terms);
        },
      },
      { label: "Cancel", value: null },
    ],
    tag: "troops",
    className: "troops",
    wide: lend,
  });
  return m.done;
}
