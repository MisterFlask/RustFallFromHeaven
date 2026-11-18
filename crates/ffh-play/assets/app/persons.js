// Persons of Note in the browser client (docs/PERSONS.md, "UI"): the one mark of a loyalty state
// (every screen draws it with stateMark, never restyled), a House's shield, the Court screen (part
// A: Shift+F9, the menu's Court), a city's grant list (part B) and the help topic (part J).
//
// The server makes every line about the rules (`court`, `grant {city}`: view/persons.rs); this
// module lays them out, puts the buttons where the lines say, and sends `grant_city`, `name_heir`,
// `release_house` (and an empire order for a Works demand), which the server checks. A grant, and
// a House's release, take two steps: the choice, then a line that names what is given and says it
// cannot be undone. Nothing here blocks the turn: what needs attention is a line at the top of the
// Court and the count on its menu button (the view's `court_attention`).

import * as api from "./api.js";
import { S, emit, me, info } from "./state.js";
import { h, fill, toast, errorText } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { unitIcon } from "./look.js";
import { isoSkin, unitFigure } from "./isoui.js";
import { infoTip } from "./infotips.js";
import { keywordChips, remindersNode } from "./keywords.js";
import { render as markup } from "./markup.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateClass, stateGlyph, stateKey, stateWord, turnsText } from "./persons_mark.js";

const SVG = "http://www.w3.org/2000/svg";

// ---- the state mark and the shield (principle 2: one function draws each) ---------------------

/** The reminder boxes of loyalty state `key` (the data's KeywordType entry `KEYWORD_<STATE>` and
 *  those it implies), or null before the data arrives. */
function stateReminders(key) {
  const table = S.data?.tables?.KeywordType ?? [];
  const entry = table.find((k) => k.type === `KEYWORD_${key.toUpperCase()}`);
  if (!entry?.reminder) return null;
  const keys = entry.with ?? [entry.type];
  const list = keys.map((t) => table.find((k) => k.type === t)).filter((k) => k?.reminder).map((k) => ({ key: k.type, name: k.name, text: k.reminder }));
  return remindersNode(list);
}

/** The mark of a state: `span.pstate.ps-<state>` with the glyph and the word, and for a running
 *  countdown the turns left (`Disloyal ✖ 8 turns`). `state` as the host sends it (`Restless`) or
 *  a key of persons_mark.js STATES; `turns` null or undefined: none. Its tooltip is the state's
 *  reminder text (keywords.jsonc). */
export function stateMark(state, turns) {
  const key = stateKey(state);
  const hasTurns = turns !== null && turns !== undefined;
  return h(
    `span.pstate.${stateClass(key)}`,
    { "data-state": key, tip: () => stateReminders(key) },
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
    h("p", "A civilization has a court: named persons who hold its cities and command its armies. Each wants things, and gives less when crossed."),
    h("h4", "The court"),
    h(
      "ul",
      h("li", "A hero enters the court when you have it. Your best veteran in an army enters at level 5 (one level more for each veteran already in it). A great person can found a House."),
      h("li", "A person has one post: the head of a House, the commander of an army, or none."),
      h("li", "An army's commander is the person with it of the highest level, unless you name another (the Armies screen)."),
    ),
    h("h4", "Houses and the Crown"),
    h(
      "ul",
      h("li", "Every city starts as the Crown's. You grant a city to a House, to a person without a House (who founds one), or to a great person (who is consumed and draws the founder). A grant is for good. The capital stays the Crown's."),
      h("li", "Under a government of direct rule (Despotism, God King) there are no Houses. Adopting one while Houses stand makes them all secede at once."),
      h("li", "When a head leaves, a single person without a post succeeds; with several, you name the heir."),
      h("li", "A House can be released as your vassal civilization."),
    ),
    h("h4", "Demands and loyalty"),
    h(
      "ul",
      h("li", "Each person makes one demand at a time, with a term: a post, a House, a city granted or taken, a war, a state religion, a wonder in the seat. The Court's demand line has a button that goes where it is met."),
      h("li", "Met: +3 for 60 turns. Refused at its term, or made impossible by your act: -3 for 60 turns. A grant: +1 for 30 turns to the House that gets it, a loss to every other claimant."),
      h("li", "Every act that moves loyalty shows the change before it is done: the grant list, civics, religion, war and peace, razing a city."),
      h("li", "The four states are Devoted, Loyal, Restless and Disloyal; their marks show their rules."),
    ),
    h("h4", "Keys"),
    h("ul", h("li", "Shift+F9 opens the Court. Its button's count is what needs attention: an heir to name, a Disloyal holder, a demand with 10 turns or fewer.")),
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
    const fig = isoSkin() ? unitFigure(p.unit_type, owner, size) : unitIcon(p.unit_type, ownerLook(), { title: p.name });
    if (fig) return h("span.pportrait.unit", fig);
  }
  const initials = String(p.name ?? p.title ?? "?").split(/\s+/).map((w) => w[0] ?? "").join("").slice(0, 2);
  return h("span.pportrait.none", { "aria-hidden": "true", style: { width: `${size}px`, height: `${size}px` } }, initials);
}

/** A trait's line with the trait's own help and which of its parts a city carries as its tooltip. */
function traitLine(text, t) {
  if (!text) return null;
  if (!t) return h("div.pline", text);
  return h("div.pline.ptrait", { tip: infoTip("TraitType", t.trait, { extra: h("div.tip-hint", t.applies) }) }, text);
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
    case "heir":
      return pickHeir(act.house);
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

/** The opened part of a card: loyalty parts and total, then the other lines, the record, and the
 *  backstory as flavour, apart. */
function moreBlock(x, extra = []) {
  return h(
    "details.pmore",
    h("summary", "Loyalty and record"),
    h("ul.pparts", x.parts.length ? x.parts.map((p) => h("li", p)) : h("li.dim", "No modifiers.")),
    h("div.ptotal", x.total),
    ...extra,
    x.record?.length ? [h("div.psub", "Record"), h("ul.precord", x.record.map((r) => h("li", r)))] : null,
    x.backstory ? h("div.pbackstory", markup(x.backstory)) : null,
  );
}

/** A person's head row: portrait, name, title, state mark (`mark` false: a head's, whose state is
 *  their House's, marked once on the House). */
function personHead(p, size = PORTRAIT.small, mark = true) {
  return h(
    "div.phead",
    portrait(p, size),
    h("div.pname", h("b", p.name), p.title ? h("div.ptitle.dim", p.title) : null),
    mark ? stateMark(p.state, p.countdown_turns) : null,
  );
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
    moreBlock(p, [h("div.pline", p.ambition), traitLine(p.trait_line, p.trait), p.rival ? h("div.pline", p.rival) : null]),
  );
}

/** A House's card (part A.3). */
function houseCard(x, m, reload) {
  const chips = x.cities.map((c) =>
    h(`button.small.city-chip${c.seat ? ".seat" : ""}`, { "data-city": c.city, title: "Open the city", onclick: () => (m.close(null), emit("open-city", c.city)) }, c.name),
  );
  const head = x.head
    ? h("div.phouse-head", personHead(x.head, PORTRAIT.small, false), h("div.pline", x.head.where))
    : h("div.phouse-head", h("div.pline.warn", x.head_line), x.needs_heir ? actButton({ label: "Name an heir", act: { kind: "heir", house: x.id } }, m) : null);
  const foot = h("div.pfoot");
  const release = () =>
    fill(foot,
      h("button.small.prelease", { disabled: !x.release.ok, title: x.release.refusal ?? "Release the House as your vassal civilization", onclick: confirm }, "Release as a vassal"),
      x.release.refusal ? h("span.dim", ` ${x.release.refusal}`) : null,
    );
  const confirm = () =>
    fill(foot,
      h("div.pconfirm", h("p", x.release.confirm), h("button.small.danger", { onclick: async () => (await run({ c: "release_house", house: x.id })) && reload() }, "Release"), h("button.small", { onclick: release }, "Back")),
    );
  release();
  return h(
    `div.pcard.house-card${x.state === "Disloyal" ? ".alarm" : ""}`,
    { "data-house": x.id },
    h("div.phead", houseShield(x.color, 22), h("div.pname", h("b", x.label)), stateMark(x.state, x.countdown_turns)),
    x.outcome ? h("div.pline.warn", x.outcome) : null,
    x.held ? h("div.pline", x.held) : null,
    head,
    x.seat ? h("div.pline", x.seat) : null,
    h("div.pchips", chips),
    h("div.pline", x.orders),
    traitLine(x.trait_line, x.trait),
    h("div.pline", x.maintenance),
    x.head ? demandLine(x.head.demand, m) : null,
    foot,
    moreBlock({ parts: x.parts, total: x.total, record: x.head?.record, backstory: x.head?.backstory }, [h("div.pline.dim", x.founded), x.head ? h("div.pline", x.head.ambition) : null, x.head?.rival ? h("div.pline", x.head.rival) : null]),
  );
}

/** The Crown's card: its cities, each with [Grant] or the reason it cannot be granted. */
function crownCard(r, m) {
  const rows = r.crown.map((c) =>
    h(
      "li.crown-city",
      { "data-city": c.city },
      h("button.link", { title: "Open the city", onclick: () => (m.close(null), emit("open-city", c.city)) }, c.name),
      c.grantable ? h("button.small.pgrant", { "data-city": c.city, onclick: () => openGrant(c.city) }, "Grant") : h("span.dim.prefusal", ` ${c.refusal ?? ""}`),
    ),
  );
  return h("div.pcard.crown-card", { "data-crown": "1" }, h("div.phead", h("span.crown-glyph", { "aria-hidden": "true" }, "♛"), h("div.pname", h("b", "The Crown"))), r.grants ? h("div.pline.dim", r.grants) : null, h("ul.crown-cities", rows));
}

// ---- the Court screen (part A) ----------------------------------------------------------------

/** The Court's dialog while open, and how to read it again (a grant elsewhere refreshes it). */
let court = null;

/** Scrolls the open Court to a card (`{house}`, `{person}`, `{crown}`), opens its record and marks
 *  it a moment. */
function focusCard(focus) {
  const el = court?.m?.el;
  if (!el || !focus) return;
  const sel = focus.crown ? ".crown-card" : focus.house !== undefined ? `[data-house="${focus.house}"]` : `[data-person="${focus.person}"]`;
  const card = el.querySelector(sel);
  if (!card) return;
  const more = card.querySelector("details.pmore");
  if (more && !focus.crown) more.open = true;
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
    const header = h("div.court-header", r.header.map((l) => h("div", l)), keywordChips(r.reminders));
    if (r.empty.length) {
      m.setBody(header, h("div.court-empty", r.empty.map((l) => h("p", l))));
      return;
    }
    const attention = r.attention.length
      ? h(
          "div.court-attention",
          { role: "list", "aria-label": "Needs attention" },
          r.attention.map((a) =>
            h(
              `div.pattn${a.urgent || a.kind === "disloyal" ? ".urgent" : ""}`,
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
      h("h4", "Houses"),
      h("div.pcards", crownCard(r, m), r.houses.map((x) => houseCard(x, m, load))),
      r.persons.length ? [h("h4", "Persons without a House"), h("div.pcards", r.persons.map((p) => personCard(p, m)))] : null,
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

/** The Court again, if it is open (after a grant, an heir, a release). */
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

// ---- naming an heir --------------------------------------------------------------------------

/** The heir's choice for House `houseId`: its candidates as cards, one click names one. */
async function pickHeir(houseId) {
  let r;
  try {
    r = await api.query("court");
  } catch (e) {
    toast(errorText(e), "error");
    return null;
  }
  const x = r.houses.find((k) => k.id === houseId);
  if (!x) return null;
  const m = openModal({ title: `Name an heir: ${x.label}`, tag: "court-heir", body: [], buttons: [{ label: "Cancel", value: null }] });
  m.setBody(
    h("p.dim", x.head_line),
    h(
      "div.pcards.one",
      x.heir_candidates.map((p) =>
        h(
          "div.pcard.heir-card",
          { "data-person": p.id },
          personHead(p),
          h("div.pline", p.where),
          traitLine(p.trait_line, p.trait),
          h("div.pline", p.ambition),
          h(
            "button.pact.primary",
            {
              onclick: async () => {
                if (await run({ c: "name_heir", house: x.id, person: p.id })) {
                  m.close(true);
                  reloadCourt();
                }
              },
            },
            `Name ${p.name}`,
          ),
        ),
      ),
    ),
  );
  return m.done;
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

/** One subject's lines of a candidate's preview: who (with their state now and why they claim),
 *  the lines, and the state after when it changes. */
function effectBlock(e) {
  return h(
    "div.peffect",
    h("div.peffect-who", h("b", e.subject), e.tag ? h("span.dim", ` (${e.tag})`) : null, e.state ? [" ", stateMark(e.state)] : null),
    h("ul", e.lines.map((l) => h("li", l))),
    e.becomes ? h(`div.pbecomes.${stateClass(e.state_after)}`, e.becomes, " ", h("span.pstate-glyph", { "aria-hidden": "true" }, stateGlyph(e.state_after))) : null,
  );
}

/** A candidate's card: who, the claim, what the grant does, and its button, which turns into the
 *  confirmation line. */
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
  const claim = c.claimant ? keywordChips((r.reminders ?? []).filter((k) => k.key === "KEYWORD_CLAIMANT")) : null;
  return h(
    "div.pcard.grant-card",
    { "data-kind": c.kind, "data-claimant": c.claimant ? "1" : "0" },
    h("div.phead", who, h("div.pname", h("b", c.title)), claim, c.state ? stateMark(c.state) : null),
    h("ul.plines", c.lines.map((l) => h("li", l))),
    c.effects.map(effectBlock),
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
    h("div.grant-head", h("p", h("b", r.note)), h("p.dim", r.baseline), keywordChips(r.reminders)),
    list.length ? h("div.pcards.one.grant-list", list.map((c) => candidateCard(r, c, m))) : h("p.dim", r.none ?? ""),
  );
  return m.done;
}
