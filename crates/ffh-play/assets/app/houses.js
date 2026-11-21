// This game's Houses in the browser client (docs/HOUSES.md, rule 15; docs/HUMAN.md, "Houses"):
// the pick dialog (the `house_leader` decision's designed form: the candidates as cards side by
// side, stacked on a narrow screen), the Houses screen (the menu's Houses, Shift+F9), a House's
// card, the city screen's House block, a House's shield on the map's plates, the previews under
// an act, the Houses of another civilization on the diplomacy screen, the alerts and the help.
//
// The server makes every line about the rules (`houses` query, the decision's `offer`, the city's
// `happiness.house` and `house_reason`, the players' `houses` and `*_house_preview`:
// view/houses.rs); this module lays them out. The pure parts are in houses_lines.js.

import * as api from "./api.js";
import { S, emit, observing } from "./state.js";
import { h, errorText, toastNode } from "./dom.js";
import { openModal, closeTag, registerDecisionView } from "./modals.js";
import { addHelp } from "./help.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { keywordChips, remindersNode } from "./keywords.js";
import { calcTip } from "./calc.js";
import { tip, showTip } from "./tips.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateClass, stateName, cardSections, previewLines, alertEntries, alertHouse } from "./houses_lines.js";

const SVG = "http://www.w3.org/2000/svg";

// ---- small parts -------------------------------------------------------------------------------

/** A House's shield in its colour (by its id), `size` CSS px high: an inline SVG. */
export function houseShield(id, size = 16) {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 ${SHIELD.w} ${SHIELD.h}`);
  svg.setAttribute("width", String(Math.round((size * SHIELD.w) / SHIELD.h)));
  svg.setAttribute("height", String(size));
  svg.setAttribute("class", "house-shield");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", SHIELD.path);
  path.setAttribute("fill", houseColour(id));
  path.setAttribute("stroke", SHIELD_EDGE);
  path.setAttribute("stroke-width", "1.2");
  path.setAttribute("stroke-linejoin", "round");
  svg.appendChild(path);
  return svg;
}

/** The House keywords' reminders as the data has them (`[{key, name, text}]`), for a tooltip
 *  that is not sent its own (a plate's shield). */
function dataReminders(keys) {
  const table = S.data?.tables?.KeywordType ?? [];
  return keys.map((k) => table.find((x) => x.type === k)).filter((k) => k?.reminder).map((k) => ({ key: k.type, name: k.name, text: k.reminder }));
}

/** Of `reminders` (the server's), the keywords a card's lines name: House always, Sympathy and
 *  Outsider where the card has one. */
function cardReminders(x, reminders) {
  const want = new Set(["KEYWORD_HOUSE", x.outsider ? "KEYWORD_OUTSIDER" : null, !x.outsider && x.sympathy !== null && x.sympathy !== undefined ? "KEYWORD_SYMPATHY" : null].filter(Boolean));
  return (reminders ?? []).filter((r) => want.has(r.key));
}

/** A state as a word in its colour (`hs-pleased`). */
export function stateWord(state) {
  return h(`span.hstate.${stateClass(state)}`, stateName(state));
}

/** Element `el` with House calculation `calc` and keyword `reminders` as its tooltip (hover,
 *  focus or a tap), marked as a calculated number. */
function withHouseCalc(el, calc, reminders) {
  if (!calc?.length) return el;
  el.classList.add("calc");
  el.tabIndex = 0;
  tip(el, () => [calcTip(calc), remindersNode(reminders)]);
  el.addEventListener("click", (ev) => (ev.stopPropagation(), showTip(el)));
  return el;
}

/** The leader's portrait (`PORTRAIT.medium`), else the initials of the name. */
function portrait(leader, name, size = PORTRAIT.medium) {
  const img = leader !== null && leader !== undefined ? leaderPortrait(leader, { size, title: name }) : null;
  if (img) return h("span.hportrait", img);
  const initials = String(name ?? "?").split(/\s+/).map((w) => w[0] ?? "").join("").slice(0, 2);
  return h("span.hportrait.none", { "aria-hidden": "true", style: { width: `${size}px`, height: `${size}px` } }, initials);
}

// ---- a card (the Houses screen's and the pick's) -------------------------------------------------

/** A House's or a candidate's card. `x`: a House of the `houses` query (`mode: "house"`) or a
 *  card of a pick's offer (`mode: "pick"`). `reminders`: the keywords' (the server's). `open`: a
 *  city chip's action (the Houses screen); `pick(i)`: the pick's choice. */
export function houseCard(x, { mode = "house", reminders = [], open = null, pick = null } = {}) {
  const sec = cardSections(x);
  const name = mode === "house" ? x.leader_name : x.name;
  const off = mode === "house" && x.perk && x.perk.on === false;
  const rem = cardReminders(x, reminders);
  const head = h(
    "div.hhead",
    portrait(x.leader, name),
    h(
      "div.hname",
      mode === "house" ? h("div.hlabel", houseShield(x.id, 18), h("b", x.label)) : null,
      h(mode === "house" ? "div.hleader" : "div.hleader.big", name),
      x.epithet ? h("div.hepithet", x.epithet) : null,
      x.advisor ? h("span.badge.hadvice", { title: "The advisor's pick" }, "advisor") : null,
    ),
  );
  const perk = h(
    `div.hperk${off ? ".off" : ""}`,
    sec.perk ? h("div.hperk-name", sec.perk) : null,
    sec.perkLines.length ? h("ul.hperk-lines", sec.perkLines.map((l) => h("li", l))) : null,
    sec.perkOff && mode === "house" ? h("div.hperk-off.warn", sec.perkOff) : null,
  );
  const opinion = sec.opinion ? withHouseCalc(h(`div.hopinion.${stateClass(x.state)}`, sec.opinion), x.calc, rem) : null;
  const cities =
    mode === "house" && x.cities?.length
      ? h(
          "div.hcities",
          x.cities.map((c) =>
            h(
              `button.small.city-chip${c.seat ? ".seat" : ""}`,
              { "data-city": c.id, title: c.seat ? "The seat. Open the city" : "Open the city", onclick: (ev) => (ev.stopPropagation(), open?.(c)) },
              c.name,
            ),
          ),
        )
      : null;
  const choose =
    mode === "pick"
      ? h("button.primary.hchoose", { "data-pick": x.index, onclick: (ev) => (ev.stopPropagation(), pick?.(x.index)), "aria-label": `Choose ${name}` }, "Choose")
      : null;
  return h(
    `div.hcard${x.outsider ? ".outsider" : ""}${x.advisor ? ".advised" : ""}${off ? ".alarm" : ""}`,
    { "data-state": stateName(x.state), "data-house": mode === "house" ? x.id : null, "data-card": mode === "pick" ? x.index : null },
    head,
    h("div.hlines", sec.leaning.map((l) => h(`div.hline${l.startsWith("Outsider:") ? ".houtsider" : ""}`, l))),
    perk,
    opinion,
    sec.happiness ? h("div.hhappy", sec.happiness) : null,
    cities,
    choose,
  );
}

// ---- the pick (rule 5) -------------------------------------------------------------------------

/** The pick dialog's body (the `house_leader` decision with its `offer`): the head (the House
 *  and its cities, the culture there), the cards side by side, the keywords. */
function pickView(d, send) {
  const o = d.offer;
  const cards = (o.cards ?? []).map((c) => houseCard(c, { mode: "pick", reminders: o.reminders, pick: (i) => send(i) }));
  return {
    wide: true,
    className: "house-pick",
    body: [
      h("div.hpick-head", (o.head ?? []).map((l, i) => h(i ? "p.hpick-culture" : "p.hpick-forms", l))),
      h(`div.hcards.pick.n${cards.length}`, cards),
      o.reminders?.length ? h("div.decision-keywords", keywordChips(o.reminders.filter((r) => r.key !== "KEYWORD_OUTSIDER" || o.foreign))) : null,
    ],
  };
}

registerDecisionView("house_leader", pickView);

// ---- the Houses screen -------------------------------------------------------------------------

/** The Houses screen: each House a card (its leader, lines, state with its calculation, perk,
 *  cities), then the cities in no House with the reason. `house`: the House to bring into view. */
export async function openHouses({ house = null } = {}) {
  const m = openModal({ title: "Houses", body: [h("p.dim", "…")], tag: "houses", wide: true, buttons: [{ label: "Close", value: null }], help: "houses" });
  let r;
  try {
    r = await api.query("houses");
  } catch (e) {
    m.setBody(h("p.warn", errorText(e)));
    return m.done;
  }
  if (m.closed) return m.done;
  const open = (c) => {
    m.close(null);
    emit("center", c.x, c.y);
    emit("open-city", c.id);
  };
  const cards = r.houses.map((x) => houseCard(x, { mode: "house", reminders: r.reminders, open }));
  const unhoused = r.unhoused.length
    ? h(
        "div.hunhoused",
        h("h4", "Cities in no House"),
        h(
          "ul",
          r.unhoused.map((c) => h("li", { "data-city": c.id }, h("button.link", { onclick: () => open(c), title: "Open the city" }, c.name), ": ", c.reason)),
        ),
      )
    : null;
  const empty = !r.houses.length && !r.unhoused.length ? h("p.dim", r.has_houses ? "No city." : "Your civilization has no Houses.") : null;
  m.setBody(h("div.hcards", cards), unhoused, empty, r.reminders?.length ? h("div.decision-keywords", keywordChips(r.reminders)) : null);
  if (house !== null && house !== undefined) {
    const el = m.el.querySelector(`.hcard[data-house="${house}"]`);
    if (el) {
      el.classList.add("hfocus");
      el.scrollIntoView({ block: "nearest" });
    }
  }
  return m.done;
}

// ---- the city screen ---------------------------------------------------------------------------

/** The city screen's House block: the Governance line (`House Aldric (Tovin Cinderwright):
 *  Pleased, +1 happiness. Perk: Horsemasters I.`) with the House's calculation, a link to the
 *  Houses screen at the House, the happiness lines the House gives; for a city in no House the
 *  reason. Null for a city that is not the human's or has neither. */
export function houseCityView(c) {
  const x = c.happiness?.house;
  const reason = c.house_reason;
  if (!x && !reason) return null;
  const go = () => openHouses({ house: x?.id ?? null });
  if (!x) return h("div.section.city-house", h("h4", "House"), h("p.city-house-reason", reason, " ", h("button.link.small", { onclick: go, title: "Open the Houses screen [Shift+F9]" }, "Houses")));
  const rem = dataReminders(["KEYWORD_HOUSE"]);
  const parts = (c.happiness.happy_parts ?? []).concat((c.happiness.unhappy_parts ?? []).map(([l, n]) => [l, -n])).filter(([l]) => /^House .* is (Friendly|Pleased|Cautious|Annoyed|Furious)$/.test(l) || l === "Ruled by an outsider");
  return h(
    "div.section.city-house",
    { "data-house": x.id },
    h("h4", "House"),
    withHouseCalc(h("p.city-house-line", x.line), x.calc, rem),
    parts.length ? h("ul.city-house-parts", parts.map(([l, n]) => withHouseCalc(h("li", `${l}: ${n > 0 ? "+" : ""}${n}`), x.calc, rem))) : null,
    h("button.link.small.city-house-open", { onclick: go, title: "Open the Houses screen at this House [Shift+F9]" }, "Houses screen"),
  );
}

// ---- previews (rule 12) --------------------------------------------------------------------------

/** The Houses' preview of an act (`[{lines}]` from the server) as a block of lines under a
 *  question; null when nothing changes. `title`: a line above them. */
export function previewNode(list, { title = null } = {}) {
  const lines = previewLines(list);
  if (!lines.length) return null;
  return h("div.house-preview", title ? h("div.house-preview-title", title) : null, lines.map((l) => h(`div.house-preview-line${/^Its perk goes off/.test(l) ? ".warn" : ""}`, l)));
}

// ---- diplomacy ----------------------------------------------------------------------------------

/** A civilization's Houses on its diplomacy row (the players query's `houses`): each House a line
 *  (`House Aethel (Ilya Vane): Pleased`) with its lines as a tooltip; null for none. */
export function diploHouses(p) {
  const list = p.houses ?? [];
  if (!list.length) return null;
  const rem = dataReminders(["KEYWORD_HOUSE"]);
  return h(
    "div.diplo-houses",
    list.map((x) =>
      h(
        "div.diplo-house",
        {
          "data-house": x.id,
          tabIndex: 0,
          tip: () => [h("div.tip-title", `${x.label} (${x.leader_name})`), ...(x.lines ?? []).map((l) => h("div", l)), x.perk_on === false ? h("div.warn", "Perk off: the House is Furious.") : null, h("div", `State: ${stateName(x.state)}`), remindersNode(rem)],
        },
        h("span.diplo-house-name", `${x.label} (${x.leader_name})`),
        ": ",
        stateWord(x.state),
      ),
    ),
  );
}

/** The Houses of a civilization as a section of its leader's page (`p` the players query's
 *  row): one card a House with its leader's portrait and lines; null for none. */
export function diploHouseCards(p) {
  const list = p.houses ?? [];
  if (!list.length) return null;
  return h(
    "div.diplo-house-cards",
    h("h4", "Houses"),
    list.map((x) =>
      h(
        "div.diplo-house-card",
        { "data-house": x.id },
        portrait(x.leader, x.leader_name, PORTRAIT.small),
        h("div.dhc-body", h("div", h("b", x.label), ` (${x.leader_name})`, ": ", stateWord(x.state)), (x.lines ?? []).map((l) => h("div.dhc-line", l)), x.perk_on === false ? h("div.dhc-line.warn", "Perk off: the House is Furious.") : null),
      ),
    ),
  );
}

// ---- alerts (rule 11) ----------------------------------------------------------------------------

/** The Houses' alerts shown at once (the newest); the rest are counted in one more toast. */
const ALERTS_SHOWN = 3;

/** Shows the Houses' news among new log `entries` as toasts, each with its House leader's
 *  portrait; a tap opens the Houses screen at the House. Not while observing. `fresh`: the game's
 *  log read for the first time (this turn's only). */
export function houseAlerts(entries, { fresh = false } = {}) {
  if (observing()) return;
  const all = alertEntries(entries, { fresh, turn: S.view?.turn ?? S.status?.turn ?? null });
  const shown = all.slice(-ALERTS_SHOWN);
  if (all.length > shown.length) {
    const more = all.length - shown.length;
    const el = h("div.toast.log.house-alert.more", { role: "status", title: "Click: open the Houses screen" }, h("span.house-alert-mark", { "aria-hidden": "true" }, "⛉"), h("span.house-alert-text", `Houses: ${more} more ${more === 1 ? "message" : "messages"} in the log.`));
    el.addEventListener("click", () => openHouses());
    toastNode(el, 12000);
  }
  for (const e of shown) {
    const b = alertHouse(e, S.view?.houses);
    const face = b ? leaderPortrait(b.leader, { size: PORTRAIT.small, title: b.leader_name }) : null;
    const el = h(
      "div.toast.log.house-alert",
      { role: "status", title: "Click: open the Houses screen at this House", "data-house": e.house ?? "" },
      face ? h("span.house-alert-face", face) : h("span.house-alert-mark", { "aria-hidden": "true" }, "⛉"),
      b ? h("span.house-alert-shield", houseShield(b.id, 18)) : null,
      h("span.house-alert-text", e.text),
    );
    el.addEventListener("click", () => openHouses({ house: e.house ?? null }));
    toastNode(el, 12000);
  }
}

/** Closes the Houses screen (a new game). */
export function closeHouses() {
  closeTag("houses");
}

// ---- help ----------------------------------------------------------------------------------------

addHelp({
  id: "houses",
  group: "Game options",
  title: "Houses",
  body: () => {
    const rem = dataReminders(["KEYWORD_HOUSE", "KEYWORD_SYMPATHY", "KEYWORD_OUTSIDER"]);
    return [
      h("p", "Your cities are grouped into Houses. A House's cities stay yours in every respect."),
      ...rem.map((r) => [h("h4", r.name), h("p", r.text)]),
      h("h4", "Forming"),
      h(
        "ul",
        h("li", "A House forms with a leader you pick among the cards offered: each card shows the leader's alignment, religion, sympathy and perk, and what its opinion and happiness would be now."),
        h("li", "A House of another civilization's cities is offered one more card: the outsider."),
        h("li", "A perk applies in the House's cities. Each card of a pick has another kind of perk; its tier is set by the turn the House forms."),
      ),
      h("h4", "Screens"),
      h(
        "ul",
        h("li", "Shift+F9 or the menu's Houses: every House with its state, calculation, perk and cities, and your cities in no House with the reason."),
        h("li", "The city screen: the city's House, its happiness lines and the calculation."),
        h("li", "The map: a House's shield on its cities' plates."),
        h("li", "Declaring war, making peace, open borders, defensive pacts and a state religion: each question lists the Houses whose state it would change."),
      ),
    ];
  },
});

