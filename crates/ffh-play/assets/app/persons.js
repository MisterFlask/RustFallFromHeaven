// Houses in the browser client (docs/PERSONS.md, "The third build", on "The second build", rule
// 13; "UI"): the one mark of a loyalty state (every screen draws it with stateMark, never
// restyled), a House's shield, the Court screen (the realm: Shift+F9, the menu's Court), a city's
// grant list and the help topic. (The module keeps the name of the system it began as.)
//
// The server makes every line about the rules (`court`, `grant {city}`: view/persons.rs); this
// module lays them out, puts the buttons where the lines say, and sends `grant_city` (to a House,
// or `found` for a realm's first House), `house_tribute`, `tribute_default` and `free_house`, which
// the server checks. A grant, and freeing a House, take two steps: the choice, then a line that
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

/** The mark of a state: `span.pstate.ps-<state>` with the glyph and the word, and for a running
 *  countdown the turns left (`Furious ✖ 8 turns`). `state` as the host sends it (`Annoyed`) or
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
      h("li", "Under a governing civic the Crown holds a few cities at the normal maintenance (3; the Kuriotates: the capital alone, settlements not counted). Each city beyond pays triple city-count maintenance. The Court says how many and what they cost."),
      h("li", "A grant moves a Crown city to a House, for good: the city, its production and its commerce are the House's. A realm without a House founds one with its first grant: its leader is drawn from the free minor leaders of your civilization (else of other civilizations of your alignment)."),
      h("li", "A House is a civilization of its own and your vassal. Every vassal is one, whether it capitulated or sought protection."),
    ),
    h("h4", "Tribute and levies"),
    h(
      "ul",
      h("li", "Each House pays a tribute of its gold and research, 0 to 50%; the higher it is, the lower its loyalty. The rate in force moves one step of 10% toward the rate you set every 5 turns, up or down, and the payment and the loyalty follow the rate in force. Set it on the House's card: the list shows what each rate pays and does to its loyalty once in force, and when."),
      h("li", "Your armies requisition in your Houses' cities too (the Armies screen): each levy costs the House loyalty for a while. An Annoyed or Furious House takes no levy."),
    ),
    h("h4", "Loyalty"),
    h(
      "ul",
      h("li", "A House's loyalty is a sum of parts; its state is the attitude scale: Friendly, Pleased, Cautious, Annoyed, Furious. A state's mark says what it does."),
      h("li", "Each House makes one request at a time, with a term. Met: +3 for 60 turns. Refused at its term, or made impossible by your act: -3 for 60 turns. The request line's button goes where it is met."),
      h("li", "Every act that moves loyalty shows the change before it is done: the grant list, the tribute list, civics, religion, war and peace, razing a city."),
      h("li", "A Furious House breaks away at the end of a 10-turn countdown, at war with you; a city of it where your army's units outnumber the House's land combat units stays yours (a tie goes to the House)."),
    ),
    h("h4", "Breaking away, freeing, side deals"),
    h(
      "ul",
      h("li", "Adopting a government of direct rule (Despotism, God King) makes every House break away at once, each at war with you."),
      h("li", "You can free a House: it is no longer your vassal, at peace with you."),
      h("li", "Others can offer your Cautious or worse Houses gold to secede (the diplomacy screen); you can do the same to theirs. A secession is a war on the old liege."),
    ),
    h("h4", "Keys"),
    h("ul", h("li", "Shift+F9 opens the Court. Its button's count is what needs attention: a Furious House, a request with 10 turns or fewer.")),
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
  return h("span.pdemand-lapse", " ", d.lapse, d.lapse_state ? [", becomes ", stateMark(d.lapse_state), "."] : ".");
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
 *  a tap on it does nothing. */
function tributeBlock(x, reload) {
  const t = x.tribute;
  const set = async (pct) => {
    if (await run({ c: "house_tribute", house: x.id, pct })) reload();
  };
  const rows = t.rates.map((r) =>
    h(
      `button.trate${r.current ? ".current" : ""}${r.changes ? ".changes" : ""}`,
      {
        type: "button",
        "data-pct": r.pct,
        "aria-pressed": r.current ? "true" : "false",
        title: r.current ? "The rate set" : `Set the tribute to ${r.pct}%`,
        onclick: () => (r.current ? null : set(r.pct)),
      },
      h("span.trate-pct", `${r.pct}%`, r.current ? h("span.trate-now", "set") : null),
      h("span.trate-est", r.estimate),
      h("span.trate-part", h("span.trate-when", r.when), " ", [r.part_text, r.loyalty_text].join(" · ")),
      h("span.trate-state", stateMark(r.state)),
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

/** A House's card: who leads it (portrait, House, leader and title, since when), its attitude and
 *  countdown, the request, its loyalty parts, its tribute, its levy, its cities, Free; opened: the
 *  record, the refusal of any offer, the leader's ambition and backstory. */
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
      stateMark(x.state, x.countdown_turns),
    ),
    x.outcome ? h("div.pline.warn", x.outcome) : null,
    // The request is the line the player acts on: right under who leads the House.
    requestLine(x.request, m),
    h("div.pparts-block", partsBlock(x)),
    tributeBlock(x, reload),
    h(`div.pline.plevy${x.levy.takes ? "" : furious ? ".warn" : ".caution"}`, x.levy.text),
    h("div.pchips", chips),
    // Its price while it would accept an offer (Cautious or worse); a refusal is under Record.
    x.price ? h("div.pline.pprice", x.price) : null,
    foot,
    moreBlock({ record: x.record, backstory: x.backstory }, [x.price_refusal ? h("div.pline", x.price_refusal) : null, x.ambition ? h("div.pline", x.ambition) : null]),
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
              { role: "listitem", "data-kind": a.kind },
              h("span", a.text),
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
