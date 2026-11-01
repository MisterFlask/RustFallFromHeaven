// Crusades and allied loans (docs/CRUSADE.md, "Human play"), drawn for the default HD-2D map
// and its floating interface (isoui.js), which the classic interface shares:
//
// - the call to arms: a deferred popup (`pending.crusade_calls`) with the target's leader,
//   pledge or refuse;
// - lending from the map: the selection's card offers "Lend to the crusade against ..." (a
//   `crusade` action of the `group` query), the way to lend units standing where the Host needs
//   them; the lend dialog picks them from a list instead (the Crusades screen, the call);
// - the Crusades advisor (the rail's crossed swords, shown only in a game with crusades, marked
//   while a call waits): each crusade with its Host's banner colour, your part in it, and "Show"
//   buttons that close the screen and bring the HD-2D camera to the Host's army or the target;
// - the loan proposal from Foreign affairs, before the other leader as the trade screen is.
//
// Strengths are the rules' (combat strength x100, scaled by health), shown as combat strength: a
// healthy warrior is 2.

import * as api from "./api.js";
import { S, dname, teamName, tidx, me } from "./state.js";
import { h, toast } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { run } from "./commands.js";
import { playerPortrait, PORTRAIT } from "./portraits.js";
import { isoSkin, unlockIcon } from "./isoui.js";
import { playerColor } from "./tiles.js";
import { leaderHead, speech } from "./diplo.js";

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
        isoSkin()
          ? "Pledged forces count once you lend them, before the deadline: choose them from a list now, or later select units on the map and give them the order Lend to the crusade."
          : "Pledged forces count once you lend the units, before the deadline.",
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
          h(
            "p.page-text",
            `${callerName(call.source)} calls you to commit forces against ${t}. Lent units serve the crusade's Host until it ends (turn ${call.end}) and earn you a share of the spoils once they kill or die; lending means war with ${t}.`,
          ),
          h(
            "p",
            `Your obligation: strength ${strengthLabel(call.obligation)} by turn ${call.deadline} (you can lend ${strengthLabel(call.lendable)}). Refusing, or delivering nothing by then, costs your standing with the caller; delivering less costs part of it.`,
          ),
        ),
      ),
      h("label.field", h("span", "Pledge strength"), input),
      h(
        "div.options",
        h("button.option", { "data-pledge": "yes", onclick: () => send(Math.max(1, Math.round(Number(input.value) * 100))) }, "Pledge"),
        h("button.option", { "data-pledge": "refuse", onclick: () => send(null) }, "Refuse"),
      ),
    ],
    buttons: [{ label: "Later", value: "later", title: "No answer by the deadline counts as a refusal" }],
    tag: "crusade",
    className: "event crusade",
  });
  const r = await m.done;
  return result ?? r;
}

/** The lend dialog for crusade `id`: the units we can lend, with checkboxes (in the HD-2D
 *  interface each with its map figure). */
export async function openLend(id) {
  const m = openModal({ title: "Lend units", body: [h("p.dim", "…")], tag: "crusade-lend", wide: true, buttons: [{ label: "Close", value: null }] });
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
    h("p", hostSwatch(c.host), `The Host fights ${teamName(c.target)} until turn ${c.end}. Lent units cannot be recalled; they come home when it ends.`),
    units.length
      ? h(
          "div.checks.lend-units",
          units.map((u, i) => h("label.check", { "data-unit": u.unit }, boxes[i], figure(u), ` ${dname("UnitType", u.type)} · strength ${strengthLabel(u.strength)} · at (${u.x}, ${u.y})`)),
        )
      : h("p.dim", "None of your units can be lent (combat land units only, not summoned, not aboard a ship or carrying units)."),
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
  const m = openModal({ title: "Crusades", body: [h("p.dim", "…")], tag: "crusades", wide: true, buttons: [{ label: "Close", value: null }] });
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
      rows.length ? h("p.dim", isoSkin() ? "To lend units where they stand, select them on the map: their card offers Lend to the crusade." : "To lend units where they stand, select them: their actions offer Lend to the crusade.") : null,
      past.length ? h("details", h("summary", "Your past crusades"), h("ul", past)) : null,
    );
  };
  load();
  return m.done;
}

/** Propose an allied loan to player `p` (`players` query row), before its leader (as the trade
 *  screen): lend or borrow a strength against a team at war with the borrower. The leader
 *  answers each proposal; the form stays open for another. */
export async function openLoan(p) {
  let q;
  try {
    q = await api.query("crusades");
  } catch (e) {
    return toast(e.detail ?? e.message, "error");
  }
  const myTeam = me()?.team;
  const enemiesOf = (team) => q.wars.flatMap(([a, b]) => (a === team ? [b] : b === team ? [a] : [])).filter((t) => t !== myTeam && t !== p.team);
  const role = h("select", { id: "loan-role" }, h("option", { value: "borrow" }, `Borrow from ${p.name}`), h("option", { value: "lend" }, `Lend to ${p.name}`));
  const target = h("select", { id: "loan-target" });
  const fillTargets = () => {
    const teams = enemiesOf(role.value === "borrow" ? myTeam : p.team);
    target.replaceChildren(...(teams.length ? teams.map((t) => h("option", { value: String(t) }, teamName(t))) : [h("option", { value: "" }, "(the borrower is at war with no one)")]));
  };
  role.addEventListener("change", fillTargets);
  fillTargets();
  const mission = h("select", { id: "loan-mission" }, h("option", { value: "attack" }, "Attack the enemy"), h("option", { value: "defend" }, "Defend the borrower"));
  const strength = h("input", { type: "number", id: "loan-strength", min: 0.5, step: 0.5, value: "4" });
  const duration = h("input", { type: "number", id: "loan-duration", min: 1, max: q.rules.loan_max_turns, value: String(Math.min(10, q.rules.loan_max_turns)) });
  const pay = h("input", { type: "number", id: "loan-gold", min: 0, value: "0" });
  const status = h("p.dim");
  const field = (label, el) => h("label.field", h("span", label), el);
  const gold = (n) => (n > 0 ? [{ item: tidx("TradeableItem", "TRADE_GOLD"), data: n }] : []);
  const leader = leaderHead(p.id, null);
  speech(p.id).then((r) => !m.closed && leader.say(r?.text));
  const m = openModal({
    title: `Loan with ${p.name}`,
    body: [
      leader.el,
      h(
        "p.dim",
        `Lent units serve a general dedicated to the borrower's war until the loan ends; lending means war with the enemy. The borrower pays ${q.rules.loan_upkeep_gold} gold per lent unit per turn. When you lend, your advisor picks the units.`,
      ),
      h("div.fields", field("Loan", role), field("Against", target), field("Mission", mission), field("Strength", strength), field(`Turns (up to ${q.rules.loan_max_turns})`, duration), field("Gold paid with it", pay)),
      status,
    ],
    buttons: [
      {
        label: "Propose",
        primary: true,
        value: "propose",
        onClick: async () => {
          if (!target.value) {
            status.textContent = "Choose an enemy of the borrower.";
            status.className = "error";
            return;
          }
          const lend = role.value === "lend";
          const g = Math.max(0, Math.round(Number(pay.value)));
          const cmd = {
            c: "propose_loan",
            to: p.id,
            lend,
            target: Number(target.value),
            mission: mission.value,
            strength: Math.max(1, Math.round(Number(strength.value) * 100)),
            duration: Math.max(1, Math.round(Number(duration.value))),
            // The gold goes from the borrower to the lender.
            give: lend ? [] : gold(g),
            get: lend ? gold(g) : [],
            units: [],
          };
          const reply = await run(cmd);
          if (!reply) return;
          // The leader answers (Civ4's lines for a deal accepted or refused).
          const accepted = !!reply.r?.accepted;
          status.textContent = accepted ? `${p.name} agreed to the loan.` : `${p.name} refused the loan.`;
          status.className = accepted ? "good" : "warn";
          status.dataset.answer = accepted ? "agreed" : "refused";
          const r = await speech(p.id, accepted ? "AI_DIPLOCOMMENT_ACCEPT" : "AI_DIPLOCOMMENT_NO_DEAL");
          if (!m.closed) leader.say(r?.text);
        },
      },
      { label: "Close", value: null },
    ],
    tag: "loan",
    className: "loan",
  });
  return m.done;
}
