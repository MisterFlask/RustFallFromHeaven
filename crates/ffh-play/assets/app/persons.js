// Persons of Note in the browser client (docs/PERSONS.md, "The second build", rule 13; "UI"): the
// one mark of a loyalty state (every screen draws it with stateMark, never restyled), a House's
// shield, the Court screen (the realm: Shift+F9, the menu's Court), a city's grant list and the
// help topic.
//
// The server makes every line about the rules (`court`, `grant {city}`: view/persons.rs); this
// module lays them out, puts the buttons where the lines say, and sends `grant_city`,
// `house_tribute`, `tribute_default` and `free_house` (and an empire order for a first build's
// Works demand), which the server checks. A grant, and freeing a House, take two steps: the choice,
// then a line that names what is given and says it cannot be undone. A tribute is chosen from a
// list that states, for every rate, what the House would pay this turn and its loyalty part and
// total at that rate. Nothing here blocks the turn: what needs attention is a line at the top of
// the Court and the count on its menu button (the view's `court_attention`).

import * as api from "./api.js";
import { S, emit, me } from "./state.js";
import { h, fill, toast, errorText } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { keywordChips, remindersNode } from "./keywords.js";
import { render as markup } from "./markup.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateClass, stateGlyph, stateKey, stateWord, turnsText } from "./persons_mark.js";

// The unit figures (look.js, isoui.js) are loaded in the page only: they read the generated art,
// which the node tests of the modules that import this one (armies.js, governors.js, ...) do not
// have. Until they have loaded, a hero shows its initials.
let unitIcon = null;
let isoSkin = () => false;
let unitFigure = null;
if (typeof window !== "undefined" && typeof document !== "undefined") {
  import("./look.js").then((m) => (unitIcon = m.unitIcon)).catch(() => {});
  import("./isoui.js")
    .then((m) => {
      isoSkin = m.isoSkin;
      unitFigure = m.unitFigure;
    })
    .catch(() => {});
}

const SVG = "http://www.w3.org/2000/svg";

// ---- the state mark and the shield (principle 2: one function draws each) ---------------------

/** The tooltip of loyalty state `key`: what that state does for a House and for a commander, the
 *  attitude's own help in the data (attitudes.jsonc, `ATTITUDE_<STATE>`); null before the data
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
 *  state does (the attitude's help in the data). */
export function stateMark(state, turns) {
  const key = stateKey(state);
  const hasTurns = turns !== null && turns !== undefined;
  return h(
    `span.pstate.${stateClass(key)}`,
    { "data-state": key, tip: () => stateTip(key) },
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
  title: "Persons of Note",
  body: () => [
    h("p", "A civilization has a court: named persons who lead its Houses and command its armies. Its Houses are its vassal civilizations; the Crown is what it holds itself."),
    h("h4", "The Crown and its Houses"),
    h(
      "ul",
      h("li", "Every city starts as the Crown's. The capital stays the Crown's."),
      h("li", "Under a governing civic the Crown holds a few cities at the normal maintenance (3; the Kuriotates: the capital alone, settlements not counted). Each city beyond pays triple city-count maintenance. The Court's header says how many and what they cost."),
      h("li", "A grant moves a Crown city to a House, for good: the city, its production and its commerce are the House's. Granting it to a veteran or a great person founds a new House; its founder leaves the map and leads it. A hero cannot head a House."),
      h("li", "A House is a civilization of its own and your vassal. Every vassal is one, whether it capitulated or sought protection."),
      h("li", "A House holding more cities than the realm allows (3, more as the realm grows) loses loyalty for each city over."),
    ),
    h("h4", "Tribute and levies"),
    h(
      "ul",
      h("li", "Each House pays a tribute of its gold and research, 0 to 50%; the higher it is, the lower its loyalty. Choose it on the House's card: the list shows what each rate pays and does to its loyalty."),
      h("li", "Your armies requisition in your Houses' cities too (the Armies screen): each levy costs the House loyalty for a while. An Annoyed or Furious House takes no levy."),
    ),
    h("h4", "Loyalty"),
    h(
      "ul",
      h("li", "A House's and a person's loyalty is a sum of parts; its state is the attitude scale: Friendly, Pleased, Cautious, Annoyed, Furious. A state's mark says what it does."),
      h("li", "Each person makes one demand at a time, with a term. Met: +3 for 60 turns. Refused at its term, or made impossible by your act: -3 for 60 turns. The demand line's button goes where it is met."),
      h("li", "Every act that moves loyalty shows the change before it is done: the grant list, the tribute list, civics, religion, war and peace, razing a city."),
      h("li", "A Furious House breaks away at the end of a 10-turn countdown, at war with you; a city of it where your army stands stays yours. A Furious commander's sworn units secede."),
    ),
    h("h4", "Breaking away, freeing, side deals"),
    h(
      "ul",
      h("li", "Adopting a government of direct rule (Despotism, God King) makes every House break away at once, each at war with you."),
      h("li", "You can free a House: it is no longer your vassal, at peace with you."),
      h("li", "Others can offer your Cautious or worse Houses gold to secede (the diplomacy screen); you can do the same to theirs. A secession is a war on the old liege."),
    ),
    h("h4", "Keys"),
    h("ul", h("li", "Shift+F9 opens the Court. Its button's count is what needs attention: a Furious House or commander, a demand with 10 turns or fewer.")),
  ],
});

// ---- small parts -----------------------------------------------------------------------------

/** The owner's look for a unit's picture (the map's art), as the unit panel uses. */
function ownerLook() {
  const p = me()?.player;
  return p === undefined ? null : window.ffh?.mapView?.art?.owners?.get(p) ?? null;
}

/** A person's picture (principle 9): the leader's portrait as the diplomacy screen draws it, else
 *  their unit's figure as the unit panel draws it, else their initials. */
function portrait(p, size = PORTRAIT.small) {
  if (p.leader !== null && p.leader !== undefined) {
    const img = leaderPortrait(p.leader, { size, title: p.name });
    if (img) return h("span.pportrait", img);
  }
  if (p.unit_type !== null && p.unit_type !== undefined) {
    const owner = me()?.player ?? null;
    const fig = isoSkin() && unitFigure ? unitFigure(p.unit_type, owner, size) : unitIcon ? unitIcon(p.unit_type, ownerLook(), { title: p.name }) : null;
    if (fig) return h("span.pportrait.unit", fig);
  }
  const initials = String(p.name ?? p.title ?? "?").split(/\s+/).map((w) => w[0] ?? "").join("").slice(0, 2);
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
    case "unit":
      m?.close(null);
      closeTag("court");
      emit("center", act.x, act.y);
      if (act.unit !== null && act.unit !== undefined) window.ffh?.units?.select?.([act.unit]);
      return;
    case "diplomacy":
      return emit("open-screen", "diplomacy");
    case "religion":
      return emit("open-screen", "religion");
    case "order":
      return orderWonder(act);
    case "card":
      return focusCard(act.house !== undefined ? { house: act.house } : { person: act.person });
    case "crown":
      return focusCard({ crown: true });
  }
}

/** A button for a payload's `{label, act, refusal?}`. */
function actButton(b, m, cls = "") {
  if (!b) return null;
  return h(`button.small.pact${cls}`, { disabled: !!b.refusal, title: b.refusal ?? null, "data-act": b.act?.kind ?? "", onclick: () => doAct(b.act, m) }, b.label);
}

/** The demand line (`Demands: Torrin granted to House Ashledger. 34 turns.`), its note and its
 *  button; between demands `No demand. Next in 14 turns.` */
function demandLine(d, m) {
  if (!d) return null;
  if (!d.kind) return h("div.pdemand.dim", d.text);
  return h(
    "div.pdemand",
    { "data-kind": d.kind },
    h("span.pdemand-text", d.text, " ", h(`span.pdemand-turns${d.urgent ? ".urgent" : ""}`, d.turns)),
    d.note ? h("span.pdemand-note.dim", ` ${d.note}`) : null,
    d.button ? actButton(d.button, m) : null,
  );
}

/** The loyalty parts, one a line, and the total. */
function partsBlock(x) {
  return [h("ul.pparts", x.parts.length ? x.parts.map((p) => h("li", p)) : h("li.dim", "No modifiers.")), h("div.ptotal", x.total)];
}

/** The opened part of a card: `extra` lines, the record, and the backstory as flavour, apart.
 *  `parts`: the loyalty parts and total are in it too (a person's card; a House's shows them on
 *  the card). */
function moreBlock(x, extra = [], { parts = true, summary = "Loyalty and record" } = {}) {
  return h(
    "details.pmore",
    h("summary", summary),
    parts ? partsBlock(x) : null,
    ...extra,
    x.record?.length ? [h("div.psub", "Record"), h("ul.precord", x.record.map((r) => h("li", r)))] : null,
    x.backstory ? h("div.pbackstory", markup(x.backstory)) : null,
  );
}

/** A person's head row: portrait, name, title, state mark. */
function personHead(p, size = PORTRAIT.small) {
  return h("div.phead", portrait(p, size), h("div.pname", h("b", p.name), p.title ? h("div.ptitle.dim", p.title) : null), stateMark(p.state, p.countdown_turns));
}

/** A person's card (part A.4): a commander or a person with no post. */
function personCard(p, m) {
  const show = p.x !== null && p.x !== undefined ? h("button.small.plocate", { title: "Show the unit on the map", "aria-label": "Show the unit", onclick: () => doAct({ kind: "unit", x: p.x, y: p.y, unit: p.unit }, m) }, "⌖") : null;
  return h(
    "div.pcard.person-card",
    { "data-person": p.id },
    personHead(p),
    h("div.pline", p.where, " ", show),
    h("div.pline", p.post),
    p.outcome ? h("div.pline.warn", p.outcome) : null,
    demandLine(p.demand, m),
    moreBlock(p, [h("div.pline", p.ambition), p.rival ? h("div.pline", p.rival) : null]),
  );
}

// ---- a House's card (rule 13) ---------------------------------------------------------------

/** The House cards whose tribute list is open (by House id): kept open when the Court is read
 *  again after a change. */
const tributeOpen = new Set();

/** A House's tribute: its line (`Tribute 20%: 12 gold, 8 research last turn.`) and the list of
 *  rates (0 to 50%), each with what the House would pay this turn and its tribute part and
 *  loyalty at that rate (rule 4); a tap on a rate sets it (`house_tribute`) and the Court is read
 *  again. The current rate is marked and cannot be chosen. */
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
        disabled: r.current,
        title: r.current ? "The House's tribute now" : `Set the tribute to ${r.pct}%`,
        onclick: () => set(r.pct),
      },
      h("span.trate-pct", `${r.pct}%`, r.current ? h("span.trate-now", "now") : null),
      h("span.trate-est", r.estimate),
      h("span.trate-part", `${r.part_text} · ${r.loyalty_text}`),
      h("span.trate-state", stateMark(r.state)),
    ),
  );
  const box = h(
    "details.ptribute",
    { open: tributeOpen.has(x.id), ontoggle: (ev) => (ev.target.open ? tributeOpen.add(x.id) : tributeOpen.delete(x.id)) },
    h("summary", h("span.ptribute-line", t.line), h("span.ptribute-change", "Change")),
    h("div.trates-head.dim", "At each rate, this turn:"),
    h("div.trates", { role: "group", "aria-label": `${x.label}: tribute` }, rows),
  );
  return box;
}

/** A House's card: who leads it (portrait, House, leader, since when), its attitude and
 *  countdown, the demand, its loyalty parts, its tribute, its levy, its cities and size, Free;
 *  opened: the record, the price to secede, the leader's backstory. */
function houseCard(x, m, reload) {
  const chips = x.cities.map((c) =>
    h(
      `button.small.city-chip${c.seat ? ".seat" : ""}${c.held ? ".held" : ""}`,
      {
        "data-city": c.city,
        // A House's city is its own player's: the map shows it (your city screen does not open on it).
        title: c.held ? "An army of yours stands in it. Show it on the map" : "Show it on the map",
        onclick: () => (m.close(null), emit("center", c.x, c.y), emit("flash", c.x, c.y)),
      },
      c.name,
    ),
  );
  const head = x.head;
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
      portrait({ leader: x.leader, name: head?.name ?? x.label }),
      h(
        "div.pname",
        h("div.phouse-name", houseShield(x.color, 18), h("b", x.label)),
        head ? h("div.pleader", head.name, head.title ? h("span.ptitle.dim", `, ${head.title}`) : null) : null,
        h("div.psince.dim", x.since),
      ),
      stateMark(x.state, x.countdown_turns),
    ),
    x.outcome ? h("div.pline.warn", x.outcome) : null,
    // The demand is the line the player acts on: right under who leads the House.
    head ? demandLine(head.demand, m) : null,
    h("div.pparts-block", partsBlock(x)),
    tributeBlock(x, reload),
    h(`div.pline.plevy${x.levy.takes ? "" : furious ? ".warn" : ".caution"}`, x.levy.text),
    h("div.pchips", chips),
    h(`div.pline.psize${x.size.over ? ".caution" : ".dim"}`, x.size.text),
    foot,
    moreBlock({ record: head?.record, backstory: head?.backstory }, [h("div.pline", x.price), head ? h("div.pline", head.ambition) : null, head?.rival ? h("div.pline", head.rival) : null], { parts: false, summary: "Record" }),
  );
}

/** The Crown's card: its cities, each with [Grant] or the reason it cannot be granted, those over
 *  the limit marked (the limit is the header's line). */
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

/** Scrolls the open Court to a card (`{house}`, `{person}`, `{crown}`), opens its details and
 *  marks it a moment. */
function focusCard(focus) {
  const el = court?.m?.el;
  if (!el || !focus) return;
  const sel = focus.crown ? ".crown-card" : focus.house !== undefined ? `[data-house="${focus.house}"]` : `[data-person="${focus.person}"]`;
  const card = el.querySelector(sel);
  if (!card) return;
  const more = card.querySelector("details.pmore");
  if (more && focus.person !== undefined) more.open = true;
  card.scrollIntoView({ block: "start", behavior: "auto" });
  card.classList.add("pfocus");
  setTimeout(() => card.classList.remove("pfocus"), 1600);
}

/** Opens the Court screen (part A); `focus`: `{house}` or `{person}` (ids) to open at that card,
 *  or nothing. Resolves when it closes. */
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
      r.realm ? h("div.court-realm", r.realm) : null,
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
      r.persons.length ? [h("h4", "Commanders and persons without a post"), h("div.pcards", r.persons.map((p) => personCard(p, m)))] : null,
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

// ---- a Works demand's wonder ordered in the seat ----------------------------------------------

async function orderWonder(act) {
  const m = openModal({
    title: `Order ${act.what}`,
    tag: "court-order",
    body: [h("p", act.line ?? `${act.what}: ${act.name}`), act.refusal ? h("p.warn", act.refusal) : null],
    buttons: [
      {
        label: `Order in ${act.name}`,
        primary: true,
        disabled: !!act.refusal,
        onClick: async () => {
          if (await run({ c: "empire_order", city: act.city, order: act.order, data: act.data, dest: null })) {
            m.close(true);
            reloadCourt();
          }
        },
      },
      { label: "Cancel", value: null },
    ],
  });
  return m.done;
}

// ---- the grant list (part B) ------------------------------------------------------------------

/** Whether candidate `c` is the one `prefer` names. */
function prefers(c, prefer) {
  if (!prefer) return false;
  return (prefer.house !== undefined && c.to.house === prefer.house) || (prefer.person !== undefined && (c.to.person === prefer.person || c.person === prefer.person)) || (prefer.unit !== undefined && c.to.unit === prefer.unit);
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

/** A candidate's card: who, the claim, what the grant does, its loyalty changes, and its button,
 *  which turns into the confirmation line. */
function candidateCard(r, c, m) {
  const foot = h("div.pfoot");
  const who = c.kind === "house" && c.color !== null && c.color !== undefined ? houseShield(c.color, 22) : portrait({ leader: c.leader, unit_type: c.unit_type, name: c.title });
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
    h("div.phead", who, h("div.pname", h("b", c.title)), claim, c.state ? stateMark(c.state) : null),
    h("ul.plines", c.lines.map((l) => h("li", l))),
    // The grantee's own loyalty lines: its header already names it and its state.
    ownLines.length || own?.state_after ? h("div.peffect.self", h("ul", ownLines.map((l) => h("li", l))), own?.state_after ? becomesLine(own.state_after) : null) : null,
    c.effects.filter((e) => !e.self).map(effectBlock),
    foot,
  );
}

/** Opens the grant list of city `cityId` (part B); `prefer`: `{house}`, `{person}` or `{unit}` (ids)
 *  to put that candidate first, or nothing. A city that cannot be granted opens nothing: its reason
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
