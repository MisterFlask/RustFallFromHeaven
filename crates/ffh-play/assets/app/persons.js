// Houses in the browser client (docs/PERSONS.md, "The third build", on "The second build", rule
// 13; "UI"): the one mark of a loyalty state (every screen draws it with stateMark, never
// restyled), a House's shield, the Court screen (the realm: Shift+F9, the menu's Court), a city's
// grant list, a House's card on its own (`openHouse`: a House of another realm with its Court
// button) and the help topic. (The module keeps the name of the system it began as.)
//
// The server makes every line about the rules (`court`, `grant {city}`, `house {house}`:
// view/persons.rs); this module lays them out, puts the buttons where the lines say, and sends
// `grant_city` (to a House, or `found` for a realm's first House), `house_tribute`,
// `tribute_default`, `free_house` and `culture_channel` (Court or Guard: docs/PERSONS.md, "The
// fourth build", rule 10), which the server checks. A grant, and freeing a House, take two steps: the choice, then a line that
// names what is given and says it cannot be undone. A tribute is chosen from a
// list that states, for every rate, what the House would pay this turn and its loyalty part and
// total at that rate. Nothing here blocks the turn: what needs attention is a line at the top of
// the Court and the count on its menu button (the view's `court_attention`).

import * as api from "./api.js";
import { S, emit } from "./state.js";
import { h, fill, toast, errorText } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { keywordChips, remindersNode } from "./keywords.js";
import { render as markup } from "./markup.js";
import { withCalc } from "./calc.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateClass, stateGlyph, stateKey, stateWord, turnsText } from "./persons_mark.js";

const SVG = "http://www.w3.org/2000/svg";

// ---- the state mark and the shield (principle 2: one function draws each) ---------------------

/** The tooltip of loyalty state `key`: what that state does for a House, the attitude's own help
 *  in the data (attitudes.jsonc, `ATTITUDE_<STATE>`); null before the data
 *  arrives. The states are the game's attitude words, not keywords. */
function stateTip(key) {
  const table = S.data?.tables?.AttitudeType ?? [];
  const entry = table.find((k) => k.type === `ATTITUDE_${key.toUpperCase()}`);
  if (!entry?.help) return null;
  return remindersNode([{ key: entry.type, name: entry.name ?? stateWord(key), text: entry.help }]);
}

/** The mark of a state: `span.pstate.ps-<state>` with the glyph and the word, and a
 *  count of turns after it when one is given (`Furious ✖ 8 turns`; no host sends one since the
 *  third build removed the countdown). `state` as the host sends it (`Annoyed`) or
 *  a key of persons_mark.js STATES; `turns` null or undefined: none. Its tooltip says what the
 *  state does (the attitude's help in the data); `note`: a line of the row's own about the state
 *  (`Annoyed after 2 levies.`) at the tooltip's head, where a crowded row puts it rather than in
 *  another sentence. */
export function stateMark(state, turns, { note = null } = {}) {
  const key = stateKey(state);
  const hasTurns = turns !== null && turns !== undefined;
  return h(
    `span.pstate.${stateClass(key)}`,
    { "data-state": key, "data-note": note ?? null, tip: () => [note ? h("div.tip-title", note) : null, stateTip(key)] },
    h("span.pstate-glyph", { "aria-hidden": "true" }, stateGlyph(key)),
    h("span.pstate-word", stateWord(key)),
    hasTurns ? h("span.pstate-turns", turnsText(turns)) : null,
  );
}

/** A House's shield in its colour (`House::color`), `size` CSS px high: an inline SVG. */
export function houseShield(colourIndex, size = 16) {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 ${SHIELD.w} ${SHIELD.h}`);
  svg.setAttribute("width", String(Math.round((size * SHIELD.w) / SHIELD.h)));
  svg.setAttribute("height", String(size));
  svg.setAttribute("class", "house-shield");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", SHIELD.path);
  path.setAttribute("fill", houseColour(colourIndex));
  path.setAttribute("stroke", SHIELD_EDGE);
  path.setAttribute("stroke-width", "1.2");
  path.setAttribute("stroke-linejoin", "round");
  svg.appendChild(path);
  return svg;
}

// ---- the help topic (part J) ------------------------------------------------------------------

addHelp({
  id: "persons",
  group: "Game options",
  title: "Houses",
  body: () => [
    h("p", "A civilization's Houses are its vassal civilizations; the Crown is what it holds itself. Each House has a leader of its own, makes requests, pays tribute and can break away."),
    h("h4", "The Crown and its Houses"),
    h(
      "ul",
      h("li", "Every city starts as the Crown's. The capital stays the Crown's."),
      h("li", "Under a governing civic the Crown holds 3 cities at the normal maintenance. Each city beyond pays triple city-count maintenance. The Court says how many and what they cost."),
      h("li", "A House with a city limit (the Kuriotates) holds only settlements: a full city cannot be granted to it, and a city it founds or takes is a settlement."),
      h("li", "A grant moves a Crown city to a House, for good: the city, its production and its commerce are the House's. A realm without a House founds one with its first grant: its leader is drawn from the free minor leaders of your civilization (else of other civilizations of your alignment)."),
      h("li", "A House is a civilization of its own and your vassal. Every vassal is one, whether it capitulated or sought protection."),
    ),
    h("h4", "Tribute and levies"),
    h(
      "ul",
      h("li", "Each House pays a tribute of its gold, 0 to 50%; the higher it is, the lower its loyalty. The rate in force moves one step of 10% toward the rate you set every 5 turns, up or down, and the payment and the loyalty follow the rate in force. Set it on the House's card: the list shows what each rate pays and does to its loyalty once in force, and when."),
      h("li", "Your armies requisition in your Houses' cities too (the Armies screen): each levy costs the House loyalty for a while. An Annoyed or Furious House takes no levy."),
    ),
    h("h4", "Loyalty"),
    h(
      "ul",
      h("li", "A House's loyalty is a sum of parts; its state is the attitude scale: Friendly, Pleased, Cautious, Annoyed, Furious. A state's mark says what it does."),
      h("li", "Each House makes one request at a time, with a term. Met: +3 for 60 turns. Refused at its term, or made impossible by your act: -3 for 60 turns. The request line's button goes where it is met."),
      h("li", "Every act that moves loyalty shows the change before it is done: the grant list, the tribute list, civics, religion, war and peace, razing a city."),
      h("li", "A Furious House issues an ultimatum at your next turn: one demand you can meet at once. Accepted, a contract binds you for 20 turns and the House stays at least Annoyed. Refused, the House breaks away, at war with you; a city of it where your army's units outnumber the House's land combat units stays yours (a tie goes to the House)."),
      h("li", "A House holding 6 cities or more splits with a chance of 5% a turn: a new House, its cadet, takes the half of its cities farthest from its capital."),
    ),
    h("h4", "Breaking away, freeing, courting"),
    h(
      "ul",
      h("li", "Adopting a government of direct rule (Despotism, God King) makes every House break away at once, each at war with you."),
      h("li", "You can free a House: it is no longer your vassal, at peace with you."),
      h("li", "A civilization courts a rival's House with its culture: its channel (the Culture screen, F8) set to Court that House. When the House's favour toward it reaches the requirement, half of the House's cities, the nearest, join the courter as a new House of its realm, in peace. A House's loyalty to its liege and its leader's attitude toward the courter set the requirement."),
      h("li", "A liege guards one of its Houses with its own channel, set to Guard: every courtship of the House loses that culture each turn."),
      h("li", "A House's card shows every courtship of it, its guard and the button that sets your channel: Guard on yours, Court on a House of another realm (the diplomacy screen opens it). A courtship of yours past two thirds of its requirement needs attention."),
      h("li", "A House takes gifts of gold and resources (its trade table) and gives nothing. A gift raises its leader's attitude toward you, and with it lowers your requirement."),
      h("li", "A realm shares its knowledge: a tech you or one of your Houses learns, all of you learn. Techs are yours to trade, not your Houses'."),
    ),
    h("h4", "Keys"),
    h("ul", h("li", "Shift+F9 opens the Court. Its button's count is what needs attention: a Furious House, a courtship of yours past two thirds, a request with 10 turns or fewer.")),
  ],
});

// ---- small parts -----------------------------------------------------------------------------

/** A House leader's picture (principle 9): the leader's portrait as the diplomacy screen draws
 *  it, else the initials of the name. */
function portrait(p, size = PORTRAIT.small) {
  if (p.leader !== null && p.leader !== undefined) {
    const img = leaderPortrait(p.leader, { size, title: p.name });
    if (img) return h("span.pportrait", img);
  }
  const initials = String(p.name ?? "?").split(/\s+/).map((w) => w[0] ?? "").join("").slice(0, 2);
  return h("span.pportrait.none", { "aria-hidden": "true", style: { width: `${size}px`, height: `${size}px` } }, initials);
}

/** Runs one of the payload's acts (a button's `act`). `m`: the dialog it was pressed in. */
async function doAct(act, m = null) {
  if (!act) return;
  switch (act.kind) {
    case "grant":
      return openGrant(act.city, act.prefer ?? null);
    case "show":
      m?.close(null);
      closeTag("court");
      emit("center", act.x, act.y);
      return;
    case "diplomacy":
      return emit("open-screen", "diplomacy");
    case "religion":
      return emit("open-screen", "religion");
    case "card":
      return focusCard({ house: act.house });
    case "crown":
      return focusCard({ crown: true });
    case "culture":
      // The Culture screen (F8), over the Court: closed, the Court is where it was.
      return emit("open-screen", "culture");
  }
}

/** A button for a payload's `{label, act, refusal?}`. */
function actButton(b, m, cls = "") {
  if (!b) return null;
  return h(`button.small.pact${cls}`, { disabled: !!b.refusal, title: b.refusal ?? null, "data-act": b.act?.kind ?? "", onclick: () => doAct(b.act, m) }, b.label);
}

/** What a request's term lapsing does (`At its term: -3 (60 turns), becomes ✖ Furious.`): the
 *  host's `lapse` and the state after in the one mark, when it changes. */
function lapseText(d) {
  if (!d?.lapse) return null;
  return h("span.pdemand-lapse", " ", d.lapse, d.lapse_state ? [", becomes ", stateMark(d.lapse_state), d.lapse_then ? `: ${d.lapse_then}.` : "."] : ".");
}

/** The request line (`Request: Torrin granted to House Ashledger. 34 turns. At its term: -3 (60
 *  turns).`), its note and its button; between requests `No request. Next in 14 turns.` */
function requestLine(d, m) {
  if (!d) return null;
  if (!d.kind) return h("div.pdemand.dim", d.text);
  return h(
    "div.pdemand",
    { "data-kind": d.kind },
    h("span.pdemand-text", d.text, " ", h(`span.pdemand-turns${d.urgent ? ".urgent" : ""}`, d.turns), lapseText(d)),
    d.note ? h("span.pdemand-note.dim", ` ${d.note}`) : null,
    d.button ? actButton(d.button, m) : null,
  );
}

/** The loyalty parts, one a line, and the total. */
function partsBlock(x) {
  return [h("ul.pparts", x.parts.length ? x.parts.map((p) => h("li", p)) : h("li.dim", "No modifiers.")), h("div.ptotal", x.total)];
}

/** A House's courtships, a line each (`Courted by the Sheaim: favour 80 of 188, a split in ~12
 *  turns (Ventry, Corona).`; yours `Your courtship of House Alyndra: ...`), one past two thirds of
 *  its requirement marked as needing attention; and its guard (`Guarded: −6 a turn to each
 *  courtship.`). */
function courtshipLines(x) {
  return [
    ...(x.courtships ?? []).map((c) =>
      h(
        `div.pline.pcourted${c.attention ? ".attention" : ""}${c.own ? ".own" : ""}`,
        { "data-courter": c.courter, "data-attention": c.attention ? "1" : "0" },
        c.attention ? h("span.pcourted-mark", { title: "Needs attention: past two thirds of the requirement", "aria-label": "Needs attention" }, "!") : null,
        withCalc(h("span.pcourted-line", c.line), c.calc),
      ),
    ),
    x.guard ? withCalc(h("div.pline.pguard", x.guard), x.guard_calc) : null,
  ];
}

/** A button that sets your channel (`{kind, label, line, refusal, cmd}`: Court or Guard;
 *  docs/PERSONS.md, "The fourth build", rule 10): the button, which sends `culture_channel` (the
 *  server checks it), and beside it what the order would do (the rules' preview) or why it cannot
 *  be set. `done`: called once it is set. */
export function channelButton(b, done) {
  if (!b) return null;
  const set = async () => {
    if (await run(b.cmd)) done?.();
  };
  return h(
    "div.pchannel",
    { "data-kind": b.kind },
    h(`button.small.pchannel-set`, { disabled: !!b.refusal, title: b.refusal ?? null, "data-channel": b.kind, onclick: set }, b.label),
    b.refusal ? h("span.pchannel-refusal.dim", b.refusal) : b.line ? withCalc(h("span.pchannel-line", b.line), b.calc) : null,
  );
}

/** The opened part of a House's card: `extra` lines, the record, and its leader's backstory as
 *  flavour, apart. */
function moreBlock(x, extra = [], { summary = "Record" } = {}) {
  return h(
    "details.pmore",
    h("summary", summary),
    ...extra,
    x.record?.length ? [h("div.psub", "Record"), h("ul.precord", x.record.map((r) => h("li", r)))] : null,
    x.backstory ? h("div.pbackstory", markup(x.backstory)) : null,
  );
}

// ---- a House's card (rule 13) ---------------------------------------------------------------

/** The House cards whose tribute list is open (by House id): kept open when the Court is read
 *  again after a change. */
const tributeOpen = new Set();

/** A House's tribute: its line (`Tribute 20%: 12 gold, 8 research last turn.`; while the rate in
 *  force moves, `Tribute 30%, now 10%: reaches 30% in 10 turns. Paid last turn: ...`) and the list
 *  of rates (0 to 50%), each once in force (rule 4): what the House would pay, when it would be in
 *  force were it set now (`In force in 15 turns.`), its tribute part and its loyalty then; a tap on
 *  a rate sets it (`house_tribute`) and the Court is read again. The rate set is marked (`set`);
 *  a tap on it does nothing. A rate a contract's bound refuses is barred, its reason as its title
 *  (docs/PERSONS.md, "The third build", rule 6). */
function tributeBlock(x, reload) {
  const t = x.tribute;
  const set = async (pct) => {
    if (await run({ c: "house_tribute", house: x.id, pct })) reload();
  };
  const rows = t.rates.map((r) =>
    h(
      `button.trate${r.current ? ".current" : ""}${r.changes ? ".changes" : ""}${r.barred ? ".barred" : ""}`,
      {
        type: "button",
        "data-pct": r.pct,
        "aria-pressed": r.current ? "true" : "false",
        "aria-disabled": r.barred ? "true" : null,
        title: r.current ? "The rate set" : r.barred ?? `Set the tribute to ${r.pct}%`,
        onclick: () => (r.current || r.barred ? null : set(r.pct)),
      },
      h("span.trate-pct", `${r.pct}%`, r.current ? h("span.trate-now", "set") : null),
      h("span.trate-est", r.estimate),
      h("span.trate-part", h("span.trate-when", r.when), " ", [r.part_text, r.loyalty_text].join(" · ")),
      h("span.trate-state", stateMark(r.state)),
      r.barred ? h("span.trate-barred", r.barred) : null,
    ),
  );
  const box = h(
    "details.ptribute",
    { open: tributeOpen.has(x.id), ontoggle: (ev) => (ev.target.open ? tributeOpen.add(x.id) : tributeOpen.delete(x.id)) },
    h("summary", h("span.ptribute-line", t.line), h("span.ptribute-change", "Change")),
    h("div.trates-head.dim", "At each rate, once in force:"),
    h("div.trates", { role: "group", "aria-label": `${x.label}: tribute` }, rows),
  );
  return box;
}

/** A House's card: who leads it (portrait, House, leader and title, since when), its attitude and,
 *  Furious, its ultimatum's outcome; its contract and its split line (docs/PERSONS.md, "The third
 *  build", rules 4 to 6), the request, its loyalty parts, its tribute, its levy, its cities, the
 *  courtships of it and its guard with the Guard button (the fourth build, rule 10), Free; opened:
 *  the record, the leader's ambition and backstory. */
function houseCard(x, m, reload) {
  const chips = x.cities.map((c) =>
    h(
      `button.small.city-chip${c.seat ? ".seat" : ""}${c.held ? ".held" : ""}`,
      {
        "data-city": c.city,
        // A House's city is its own player's: the map shows it (your city screen does not open on it).
        // The contest for it when an army of yours stands in it (`Corona returns to you: ...`).
        title: c.hold ? `${c.hold} Show it on the map` : "Show it on the map",
        onclick: () => (m.close(null), emit("center", c.x, c.y), emit("flash", c.x, c.y)),
      },
      c.name,
    ),
  );
  const foot = h("div.pfoot");
  const free = () =>
    fill(foot,
      h("button.small.prelease", { disabled: !x.free.ok, title: x.free.refusal ?? "Free the House: it is no longer your vassal", onclick: confirm }, "Free"),
      x.free.refusal ? h("span.dim", ` ${x.free.refusal}`) : null,
    );
  const confirm = () =>
    fill(foot,
      h("div.pconfirm", h("p", x.free.confirm), h("button.small.danger", { onclick: async () => (await run({ c: "free_house", house: x.id })) && reload() }, "Free"), h("button.small", { onclick: free }, "Back")),
    );
  free();
  const furious = stateKey(x.state) === "furious";
  return h(
    `div.pcard.house-card${furious ? ".alarm" : ""}`,
    { "data-house": x.id },
    h(
      "div.phead",
      portrait({ leader: x.leader, name: x.leader_name || x.label }),
      h(
        "div.pname",
        h("div.phouse-name", houseShield(x.color, 18), h("b", x.label)),
        x.leader_name ? h("div.pleader", x.leader_name, x.title ? h("span.ptitle.dim", `, ${x.title}`) : null) : null,
        h("div.psince.dim", x.since),
      ),
      stateMark(x.state),
    ),
    x.outcome ? h("div.pline.warn", x.outcome) : null,
    x.contract ? h("div.pline.pcontract", x.contract) : null,
    x.split ? h("div.pline.psplit", x.split) : null,
    // The request is the line the player acts on: right under who leads the House.
    requestLine(x.request, m),
    h("div.pparts-block", partsBlock(x)),
    tributeBlock(x, reload),
    h(`div.pline.plevy${x.levy.takes ? "" : furious ? ".warn" : ".caution"}`, x.levy.text),
    h("div.pchips", chips),
    // Who courts it and its guard (docs/PERSONS.md, "The fourth build", rule 10), with the Guard
    // button while it is courted.
    ...courtshipLines(x),
    channelButton(x.channel, reload),
    foot,
    moreBlock({ record: x.record, backstory: x.backstory }, [x.ambition ? h("div.pline", x.ambition) : null]),
  );
}

/** The card of a House of another realm (the `house` query's): who leads it, whose vassal it is
 *  and its state toward its liege, the cities your map shows, every courtship of it and its guard,
 *  your requirement against its leader's attitude, and the Court button (with your channel running
 *  elsewhere, what it replaces), or your channel when it courts the House already. */
function foreignCard(x, m, reload) {
  const chips = x.cities.map((c) =>
    h(
      `button.small.city-chip${c.seat ? ".seat" : ""}`,
      { "data-city": c.city, title: "Show it on the map", onclick: () => (m.close(null), emit("center", c.x, c.y), emit("flash", c.x, c.y)) },
      c.name,
    ),
  );
  return h(
    "div.pcard.house-card.foreign",
    { "data-house": x.id, "data-player": x.player },
    h(
      "div.phead",
      portrait({ leader: x.leader, name: x.leader_name || x.label }),
      h(
        "div.pname",
        h("div.phouse-name", houseShield(x.color, 18), h("b", x.label)),
        x.leader_name ? h("div.pleader", x.leader_name, x.title ? h("span.ptitle.dim", `, ${x.title}`) : null) : null,
        h("div.psince.dim", `${x.vassal} ${x.since}`),
      ),
      stateMark(x.state, null, { note: `Toward the ${x.liege_name}.` }),
    ),
    h("div.pchips", chips),
    ...courtshipLines(x),
    withCalc(h("div.pline.prequirement", x.requirement), x.requirement_calc),
    x.courting ? h("div.pline.pcourting", x.courting) : channelButton(x.channel, reload),
  );
}

/** The Crown's card: its cities, each with [Grant] or the reason it cannot be granted, those over
 *  the limit marked (the limit is the header's line); with no House, the founding line (a grant
 *  founds one, and where its leader is drawn from; or why none can be founded). */
function crownCard(r, m) {
  const rows = r.crown.map((c) =>
    h(
      `li.crown-city${c.over ? ".over" : ""}`,
      { "data-city": c.city },
      h("button.link", { title: "Open the city", onclick: () => (m.close(null), emit("open-city", c.city)) }, c.name),
      c.note ? h(`span.crown-note${c.over ? ".caution" : ".dim"}`, c.note) : null,
      c.grantable ? h("button.small.pgrant", { "data-city": c.city, onclick: () => openGrant(c.city) }, "Grant") : h("span.dim.prefusal", ` ${c.refusal ?? ""}`),
    ),
  );
  return h(
    "div.pcard.crown-card",
    { "data-crown": "1" },
    h("div.phead", h("span.crown-glyph", { "aria-hidden": "true" }, "♛"), h("div.pname", h("b", "The Crown"))),
    r.grants ? h("div.pline.dim", r.grants) : null,
    r.founding ? h(`div.pline.pfounding${r.founding.ok ? "" : ".caution"}`, { "data-founding": r.founding.ok ? "1" : "0" }, r.founding.text) : null,
    h("ul.crown-cities", rows),
  );
}

/** The realm's default tribute (the one a new House takes) and its selector (`tribute_default`). */
function defaultTribute(r, reload) {
  if (r.tribute_default === null || r.tribute_default === undefined) return null;
  const sel = h(
    "select.tribute-default",
    {
      "aria-label": "Tribute of a new House",
      onchange: async (ev) => {
        const pct = Number(ev.target.value);
        if (await run({ c: "tribute_default", pct })) reload();
        else ev.target.value = String(r.tribute_default);
      },
    },
    r.tribute_steps.map((p) => h("option", { value: String(p), selected: p === r.tribute_default }, `${p}%`)),
  );
  return h("label.ptribute-default", "Tribute of a new House: ", sel);
}

// ---- the Court screen (part A) ----------------------------------------------------------------

/** The Court's dialog while open, and how to read it again (a grant elsewhere refreshes it). */
let court = null;

/** Scrolls the open Court to a card (`{house}`, `{crown}`) and marks it a moment. */
function focusCard(focus) {
  const el = court?.m?.el;
  if (!el || !focus) return;
  const sel = focus.crown ? ".crown-card" : `.house-card[data-house="${focus.house}"]`;
  const card = el.querySelector(sel);
  if (!card) return;
  card.scrollIntoView({ block: "start", behavior: "auto" });
  card.classList.add("pfocus");
  setTimeout(() => card.classList.remove("pfocus"), 1600);
}

/** Opens the Court screen (part A); `focus`: `{house}` (an id) to open at that card, or nothing.
 *  Resolves when it closes. */
export async function openCourt(focus = null) {
  const m = openModal({ title: "Court", tag: "court", wide: true, className: "court", body: [h("p.dim", "…")], buttons: [{ label: "Close", value: null }], help: "persons" });
  const load = async (at = null) => {
    let r;
    try {
      r = await api.query("court");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    setBadge(r.badge);
    const top = m.el.querySelector(".modal-body")?.scrollTop ?? 0;
    const header = h(
      "div.court-header",
      r.header.map((l) => h("div", l)),
      r.crown_line ? h(`div.court-crown${r.crown_line.over ? ".caution" : ""}`, r.crown_line.text) : null,
      defaultTribute(r, () => load()),
    );
    // The keywords' chips close the screen: what needs attention comes first (a phone showed two
    // rows of chips above it).
    const chips = h("div.court-keywords", keywordChips(r.reminders));
    if (r.empty.length) {
      m.setBody(header, h("div.court-empty", r.empty.map((l) => h("p", l))), chips);
      return;
    }
    const attention = r.attention.length
      ? h(
          "div.court-attention",
          { role: "list", "aria-label": "Needs attention" },
          r.attention.map((a) =>
            h(
              `div.pattn${a.urgent || a.kind === "furious" ? ".urgent" : ""}`,
              { role: "listitem", "data-kind": a.kind, "data-house": a.house ?? null },
              withCalc(h("span", a.text), a.calc),
              lapseText(a),
              a.note ? h("span.dim", ` ${a.note}`) : null,
              actButton(a.button, m),
            ),
          ),
        )
      : null;
    m.setBody(
      header,
      attention,
      h("h4", "The realm"),
      h("div.pcards", crownCard(r, m), r.houses.map((x) => houseCard(x, m, () => load()))),
      chips,
    );
    const body = m.el.querySelector(".modal-body");
    if (at) focusCard(at);
    else if (body) body.scrollTop = top;
  };
  court = { m, load };
  await load(focus);
  const done = await m.done;
  if (court?.m === m) court = null;
  return done;
}

// ---- a House's card on its own (the fourth build, rule 10) -------------------------------------

/** Opens House `id`'s card (the `house` query): one of yours opens the Court at its card; a House
 *  of another realm its card in a dialog of its own, read again once your channel is set.
 *  Resolves when it closes. */
export async function openHouse(id) {
  let r;
  try {
    r = await api.query("house", { house: id });
  } catch (e) {
    toast(errorText(e), "error");
    return null;
  }
  if (r.own) return openCourt({ house: id });
  const m = openModal({ title: r.house.label, tag: "house", className: "court house-dialog", body: [], buttons: [{ label: "Close", value: null }], help: "persons" });
  const draw = (x) => m.setBody(h("div.pcards.one", foreignCard(x.house, m, reload)), h("div.court-keywords", keywordChips(x.reminders)));
  const reload = async () => {
    try {
      const again = await api.query("house", { house: id });
      if (!m.closed) draw(again);
    } catch {
      /* keep the card as it was */
    }
  };
  draw(r);
  return m.done;
}

/** The Court again, if it is open (after a grant, a House freed). */
function reloadCourt() {
  if (court && !court.m.closed) court.load();
}

// ---- the count on the Court button (principle 6) ---------------------------------------------

function setBadge(n) {
  const btn = document.getElementById("btn-court");
  if (!btn) return;
  let b = btn.querySelector(".court-badge");
  if (!n) {
    b?.remove();
    btn.removeAttribute("data-attention");
    return;
  }
  if (!b) {
    b = h("span.court-badge", { "aria-hidden": "true" });
    btn.appendChild(b);
  }
  b.textContent = String(n);
  btn.dataset.attention = String(n);
  btn.setAttribute("aria-label", `Court: ${n} need${n === 1 ? "s" : ""} attention`);
}

/** The view changed: the badge follows its `court_attention`. */
export function viewChanged() {
  setBadge(S.view?.court_attention ?? 0);
}

// ---- the grant list (part B) ------------------------------------------------------------------

/** Whether candidate `c` is the one `prefer` names (`{house}`). */
function prefers(c, prefer) {
  return !!prefer && prefer.house !== undefined && c.to.house === prefer.house;
}

/** `Becomes ▲ Friendly.`: the one state mark, as every preview draws it (persons_lines.js). */
function becomesLine(state) {
  return h("div.pbecomes", "Becomes ", stateMark(state), ".");
}

/** One subject's lines of a candidate's preview: who (with their state now and why they claim),
 *  the lines, and the state after when it changes. */
function effectBlock(e) {
  return h(
    "div.peffect",
    h("div.peffect-who", h("b", e.subject), e.tag ? h("span.dim", ` (${e.tag})`) : null, e.state ? [" ", stateMark(e.state)] : null),
    h("ul", e.lines.map((l) => h("li", l))),
    e.state_after ? becomesLine(e.state_after) : null,
  );
}

/** A candidate's card: who (a House, or the founding of one), the claim, what the grant does, its
 *  loyalty changes, and its button, which turns into the confirmation line. */
function candidateCard(r, c, m) {
  const foot = h("div.pfoot");
  // A House: its leader's portrait, its name with its shield; a founding: the new House's name.
  const who = portrait({ leader: c.leader, name: c.kind === "found" ? c.house : c.title });
  const shield = c.kind === "house" && c.color !== null && c.color !== undefined ? houseShield(c.color, 18) : null;
  const choose = () =>
    fill(foot,
      c.refusal ? h("p.warn", c.refusal) : null,
      h("button.pact.pgrant-to", { disabled: !!c.refusal, title: c.refusal ?? null, onclick: confirm }, c.button),
    );
  const confirm = () => {
    fill(foot,
      h(
        "div.pconfirm",
        { role: "alertdialog", "aria-label": c.confirm },
        h("p", c.confirm),
        // What goes with the city: its garrison, and the army units that stay yours.
        (c.garrison ?? []).map((l) => h("p.pconfirm-garrison", l)),
        h("button.primary.danger.pgrant-yes", { onclick: () => send() }, "Grant"),
        h("button.pgrant-back", { onclick: choose }, "Back"),
      ),
    );
    foot.querySelector(".pgrant-back")?.focus({ preventScroll: true });
  };
  const send = async () => {
    const reply = await run({ c: "grant_city", city: r.city, ...c.to });
    if (!reply) return;
    m.close(true);
    reloadCourt();
  };
  choose();
  const own = c.effects.find((e) => e.self);
  const claim = c.claimant ? keywordChips((r.reminders ?? []).filter((k) => k.key === "KEYWORD_CLAIMANT")) : null;
  const ownLines = own?.lines ?? [];
  return h(
    "div.pcard.grant-card",
    { "data-kind": c.kind, "data-claimant": c.claimant ? "1" : "0" },
    h("div.phead", who, h("div.pname", h("div.phouse-name", shield, h("b", c.title))), claim, c.state ? stateMark(c.state) : null),
    h("ul.plines", c.lines.map((l) => h("li", l))),
    // The grantee's own loyalty lines: its header already names it and its state.
    ownLines.length || own?.state_after ? h("div.peffect.self", h("ul", ownLines.map((l) => h("li", l))), own?.state_after ? becomesLine(own.state_after) : null) : null,
    c.effects.filter((e) => !e.self).map(effectBlock),
    foot,
  );
}

/** Opens the grant list of city `cityId` (part B); `prefer`: `{house}` (an id) to put that
 *  candidate first, or nothing. A city that cannot be granted opens nothing: its reason
 *  is told. Resolves with true when a grant was made. */
export async function openGrant(cityId, prefer = null) {
  let r;
  try {
    r = await api.query("grant", { city: cityId });
  } catch (e) {
    toast(errorText(e), "error");
    return null;
  }
  if (r.refusal) {
    toast(r.refusal, "error");
    return null;
  }
  const m = openModal({ title: r.title, tag: "grant", className: "grant", body: [], buttons: [{ label: "Close", value: null }], help: "persons" });
  const list = [...r.candidates];
  const first = list.findIndex((c) => prefers(c, prefer));
  if (first > 0) list.unshift(...list.splice(first, 1));
  m.setBody(
    h("div.grant-head", h("p", h("b", r.note)), h("p.dim", r.baseline), r.crown_line ? h(`p${r.crown_line.over ? ".caution" : ".dim"}`, r.crown_line.text) : null),
    list.length ? h("div.pcards.one.grant-list", list.map((c) => candidateCard(r, c, m))) : h("p.dim", r.none ?? ""),
    h("div.court-keywords", keywordChips(r.reminders)),
  );
  return m.done;
}
