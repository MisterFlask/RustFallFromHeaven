// Sending commands (design §4.5) the way the Civ4 interface does: one at a time, with the
// confirmations the exe asks for (declare war to move, delete, global and war spells, which
// transport to load into, ending the turn with things open) answered in dialogs and the
// command resent with the answer.

import * as api from "./api.js";
import { S, emit, on, dname, info, teamName, gateLines, loadUnitCommand, tradeItemLabel, observing, OBSERVER_COMMANDS } from "./state.js";
import { ask } from "./modals.js";
import { toast, busy, errorText } from "./dom.js";
import { showBattles, battleShown, battlesMuted, battlesPending } from "./engagements.js";

/** The messages of a command that fought a battle (the plan's attack, or a move that attacked:
 *  the battle screens open for it): shown when the screens close. The battle's own message is the
 *  screens' to show; the others (a city captured, a defensive strike) would tell how the battle
 *  ends before its replay does. */
let heldLines = [];
on("battle-closed", () => {
  // More battles to show: their end comes later.
  if (battlesPending()) return;
  const lines = heldLines;
  heldLines = [];
  for (const line of lines) toast(line.text, "log", 7000);
});

let inFlight = false;
/** The automatic (`quiet`) command on its way, if any: AutoMoves, a spectated turn, grouping. */
let automatic = null;

/** Whether a command is being sent (another one waits: the exe sends one at a time). */
export function sending() {
  return inFlight;
}

/** Sends command `c` (`{c: "move", ...}`), asks the confirmations, and returns the reply of
 *  the command that was applied, or null (refused, cancelled, or another command in flight).
 *  Messages the command produced are shown; `applied` is emitted with the reply. A command of
 *  the player's own while an automatic one is on its way (P6: the AutoMoves the group cycle
 *  sends when it comes round) waits for it rather than being dropped; a second click while the
 *  player's own command is on its way is still dropped (one request per click). */
export async function run(c, { quiet = false } = {}) {
  // An observed game is the AI's: only turns are played (the server refuses the rest too).
  if (observing() && !OBSERVER_COMMANDS.includes(c.c)) {
    if (!quiet) toast("Observing: the AI plays every civilization (nothing can be ordered).", "error");
    return null;
  }
  if (inFlight && automatic && !quiet) await automatic.catch(() => {});
  if (inFlight) return null;
  inFlight = true;
  const sent = busy(() => loop(c, quiet));
  if (quiet) automatic = sent;
  try {
    return await sent;
  } finally {
    inFlight = false;
    if (automatic === sent) automatic = null;
  }
}

async function loop(c, quiet) {
  let next = c;
  for (let round = 0; round < 4 && next; round++) {
    let reply;
    emit("sending", next);
    try {
      reply = await api.cmd(next.c, strip(next));
    } catch (e) {
      if (!quiet) toast(refusal(e), "error");
      emit("refused", next, e);
      return null;
    }
    const confirm = reply.r?.confirm;
    if (!confirm) {
      const battleOpen = document.body.classList.contains("battle-open");
      const isBattle = (line) => line.kind === "engagement" && line.report !== undefined;
      const hold = battleOpen || ((reply.log ?? []).some(isBattle) && !battlesMuted());
      for (const line of reply.log ?? []) {
        // A battle (engagement combat) opens the battle screens, unless they show it already (the
        // plan's attack); the other messages wait for the screens to close.
        if (isBattle(line)) {
          if (battleOpen) battleShown(line.report);
          else showBattles([line.report]);
        } else if (hold) heldLines.push(line);
        else toast(line.text, "log", 7000);
      }
      emit("applied", next, reply);
      return reply;
    }
    next = await answerConfirm(next, confirm);
  }
  return null;
}

function strip(c) {
  const { c: _name, ...rest } = c;
  return rest;
}

/** A refusal for people (dom.js `errorText`: a plain sentence; the technical detail goes to
 *  the console). */
export function refusal(e) {
  return errorText(e);
}

/** The command to send after the player answered `confirm` (null: cancelled). */
async function answerConfirm(c, confirm) {
  switch (confirm.kind) {
    case "declare_war": {
      // Irreversible: Cancel has the focus (Enter, also the unit-cycle key, does not declare).
      const choices = [{ label: "Declare war", value: "war", primary: true, danger: true }];
      if (confirm.move_anyway) choices.push({ label: "Move without declaring", value: "move" });
      choices.push({ label: "Cancel", value: null });
      const v = await ask("Declare war?", `This move means war with ${teamName(confirm.team)}.`, choices);
      if (!v) return null;
      // An attack (`engage`, engagement combat) is resent with the answer.
      if (c.c === "engage") return v === "war" ? { ...c, declare_war: true } : null;
      // The exe answers the DECLAREWARMOVE popup with a move (design §4.5).
      const move = c.c === "move" ? c : { c: "move", units: c.units, x: c.data1, y: c.data2, append: c.append ?? false };
      return { ...move, declare_war: v === "war" };
    }
    case "command": {
      const name = dname("CommandType", confirm.command);
      const v = await ask(`${name}?`, `Are you sure? (${name})`, [
        { label: name, value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      return v ? { ...c, confirmed: true } : null;
    }
    case "cast_global": {
      const name = dname("SpellType", confirm.spell);
      // A card's world spell (docs/CARDS.md) has its own once-a-game flag.
      const once = info("SpellType", confirm.spell)?.once;
      const v = await ask(`Cast ${name}?`, once ? "This is a card spell: it can be cast once a game." : "This is a world spell: it can be cast once per game.", [
        { label: `Cast ${name}`, value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      return v ? { ...c, confirmed: true } : null;
    }
    case "cast_war": {
      const name = dname("SpellType", confirm.spell);
      const v = await ask(`Cast ${name}?`, `Casting ${name} here declares war on ${teamName(confirm.team)}.`, [
        { label: "Cast and declare war", value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      return v ? { ...c, confirmed: true } : null;
    }
    case "load_unit": {
      const choices = confirm.transports.map((t) => {
        const u = S.units.get(t);
        return { label: u ? `${dname("UnitType", u.type)} (${t})` : `Unit ${t}`, value: t };
      });
      choices.push({ label: "Cancel", value: null });
      const t = await ask("Load into which transport?", "Several transports have room.", choices);
      if (t === null || t === undefined) return null;
      return loadUnitCommand(c.units, t);
    }
    case "end_turn":
      return (await askGate(confirm)) ? { ...c, force: true } : null;
    case "lend_war": {
      const v = await ask("Declare war?", `Lending to this crusade means war with ${teamName(confirm.team)}.`, [
        { label: "Lend and declare war", value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      return v ? { ...c, declare_war: true } : null;
    }
    case "loan_counter": {
      // The other side's terms, as a `propose_loan` to resend (docs/CRUSADE.md).
      const k = confirm.cmd;
      const items = (v) => (v?.length ? v.map(tradeItemLabel).join(", ") : "nothing");
      const who = S.players.get(k.to)?.name ?? `Player ${k.to}`;
      const text = [
        `${who} would agree to: ${k.lend ? "you lend" : "they lend"} strength ${k.strength / 100} for ${k.duration} turns, to ${k.mission === "defend" ? "defend the borrower against" : "attack"} ${teamName(k.target)}.`,
        `You give: ${items(k.give)}. You get: ${items(k.get)}.`,
      ];
      const v = await ask("Counter-offer", text, [
        { label: "Accept", value: true, primary: true },
        { label: "Decline", value: null },
      ]);
      return v ? { ...k } : null;
    }
    default:
      toast(`Unknown confirmation ${confirm.kind}`, "error");
      return null;
  }
}

/** The command of a `group` query action, with the plot picked for a `target` action. */
export function actionCommand(action, x, y) {
  const c = { ...action.cmd };
  if (action.kind !== "target") return c;
  if (c.c === "move") return { ...c, x, y };
  return { ...c, data1: x, data2: y };
}

/** The cities with nothing to build (`pending.idle_cities`), unless observing. */
function idleCities() {
  return observing() ? [] : (S.view?.pending?.idle_cities ?? []);
}

/** The end-turn gate (`confirm end_turn`, with `idle_cities` added by the client: the server's
 *  gate does not count them). Resolves true when the player ends the turn anyway; the other
 *  buttons open what is still open. */
async function askGate(confirm) {
  const idle = confirm.idle_cities ?? idleCities();
  const lines = gateLines(confirm).map((l) => `${l[0].toUpperCase()}${l.slice(1)}.`);
  if (idle.length) lines.push(idle.length === 1 ? "1 city has nothing to build." : `${idle.length} cities have nothing to build.`);
  const choices = [];
  if (confirm.events || confirm.votes || confirm.elections || confirm.research || confirm.crusade_calls) choices.push({ label: "Answer them", value: "popups", primary: true });
  if (idle.length) {
    const city = S.cities.get(idle[0]);
    choices.push({ label: "Choose production", value: "city", primary: !choices.length, title: city ? `Open ${city.name}'s build list` : "Open the city's build list" });
  }
  if (confirm.ready_units) choices.push({ label: "Select a unit", value: "units", primary: !choices.length });
  choices.push({ label: "End turn anyway", value: "force" });
  const v = await ask("End the turn?", lines.map((l) => `• ${l}`), choices, { tag: "gate" });
  if (v === "popups") emit("reopen-popups");
  if (v === "city") emit("open-idle-city", idle[0]);
  if (v === "units") emit("select-ready");
  return v === "force";
}

/** End turn (`force` skips the gate). The server's gate does not count the cities with nothing
 *  to build (the build popup's "Later" sets one aside): when there are some, the client asks the
 *  whole gate itself, from the view's pending lists (the same ones the server counts), and
 *  "End turn anyway" sends the forced end. */
export async function endTurn(force = false) {
  if (force) return run({ c: "end_turn", force: true });
  const idle = idleCities();
  const p = S.view?.pending;
  if (!idle.length || !p || inFlight) return run({ c: "end_turn" });
  const confirm = {
    kind: "end_turn",
    events: p.events?.length ?? 0,
    votes: p.votes?.length ?? 0,
    elections: p.elections?.length ?? 0,
    research: !!p.research_needed,
    ready_units: p.ready_groups?.length ?? 0,
    crusade_calls: p.crusade_calls?.length ?? 0,
    idle_cities: idle,
  };
  return (await askGate(confirm)) ? run({ c: "end_turn", force: true }) : null;
}

/** For the end-turn button's label: what is still open this turn (the cities with nothing to
 *  build included). */
export function openCount() {
  const p = S.view?.pending;
  if (!p) return 0;
  return (p.events?.length ?? 0) + (p.votes?.length ?? 0) + (p.crusade_calls?.length ?? 0) + (p.research_needed ? 1 : 0) + idleCities().length;
}
