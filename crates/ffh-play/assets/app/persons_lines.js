// Houses in the existing screens (docs/PERSONS.md, "UI", parts C to I): the House tag of the lists,
// the preview lines under an act, the House shield on the map's name plates and the alerts from the
// log. The one mark of a state is persons.js `stateMark` (never restyled here); the Court and the
// grant list are persons.js `openCourt` / `openGrant`.

import { S, observing } from "./state.js";
import { h, toastNode } from "./dom.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { stateMark, houseShield, openCourt } from "./persons.js";
import { SHIELD, SHIELD_EDGE, houseColour, stateColour, stateGlyph, stateKey } from "./persons_mark.js";

/** A House in a list or a line: its shield and `House Arne`, and the state mark when it is
 *  Annoyed or Furious. `house` as the host sends it (`{id, label, colour, state}`); null: `Crown`.
 *  `open`: a click opens the Court at the House. */
export function houseTag(house, { open = false, mark = true } = {}) {
  if (!house) return h("span.house-tag.crown", "Crown");
  const k = stateKey(house.state);
  const body = [houseShield(house.colour, 14), h("span.house-name", house.label), mark && house.state && (k === "annoyed" || k === "furious") ? [" ", stateMark(house.state)] : null];
  if (!open) return h("span.house-tag", { "data-house": house.id }, body);
  return h("button.link.house-tag", { "data-house": house.id, title: "Open the Court at this House", onclick: (ev) => (ev.stopPropagation(), openCourt({ house: house.id })) }, body);
}

/** The preview lines of an act (`[{subject, text, state_after, line}]` from the host), one per
 *  line: `House Arne: Request refused: -3 (60 turns). Becomes ◆ Annoyed.`, the state after in
 *  the one mark. Null when there are none (nothing is shown). `title`: a line above them. */
export function previewList(lines, { title = null } = {}) {
  if (!lines?.length) return null;
  return h(
    "div.persons-preview",
    title ? h("div.persons-preview-title", title) : null,
    lines.map((l) => {
      const text = `${l.subject}: ${l.text}${/\.$/.test(l.text) ? "" : "."}`;
      // Furious is an ultimatum (docs/PERSONS.md, "The third build", rule 5).
      const then = l.state_after && stateKey(l.state_after) === "furious" ? ": an ultimatum." : ".";
      return h("div.persons-preview-line", h("span", text), l.state_after ? [" Becomes ", stateMark(l.state_after), then] : null);
    }),
  );
}

/** The preview lines as plain sentences (for a question's text). */
export function previewText(lines) {
  return (lines ?? []).map((l) => l.line).join(" ");
}

// ---- the map's name plates (part G) ------------------------------------------------------------

let shieldPath = null;

/** What a plate's shield carries for the House's liege (docs/PERSONS.md, rule 13: "its attitude
 *  mark for the liege"): the one mark of its state, `{glyph, colour}` (▲ △ ● ◆ ✖ in the states'
 *  colours); null when the state is not sent (a House not ours: its shield only). */
export function plateMark(state) {
  if (state === null || state === undefined) return null;
  const k = stateKey(state);
  return { glyph: stateGlyph(k), colour: stateColour(k) };
}

/** The plate's House shield (the view's `city.house`: `{colour, state}`) drawn on canvas `g`,
 *  `ht` px high, its centre at (`cx`, `cy`): the House's colour with a dark edge; for its liege,
 *  the state's mark on a dark disc on it ({@link plateMark}). Returns the width it takes (0
 *  without a House). */
export function drawPlateShield(g, house, cx, cy, ht) {
  if (!house) return 0;
  const s = ht / SHIELD.h;
  const w = SHIELD.w * s;
  if (!shieldPath && typeof Path2D !== "undefined") shieldPath = new Path2D(SHIELD.path);
  g.save();
  g.translate(cx - w / 2, cy - ht / 2);
  g.scale(s, s);
  g.fillStyle = houseColour(house.colour);
  g.strokeStyle = SHIELD_EDGE;
  g.lineWidth = 1.6 / s;
  g.lineJoin = "round";
  if (shieldPath) {
    g.fill(shieldPath);
    g.stroke(shieldPath);
  }
  g.restore();
  const mk = plateMark(house.state);
  if (mk) {
    const r = ht * 0.3;
    const my = cy - ht * 0.06;
    g.save();
    g.beginPath();
    g.arc(cx, my, r, 0, Math.PI * 2);
    g.fillStyle = "rgba(18,14,12,0.9)";
    g.fill();
    g.fillStyle = mk.colour;
    g.font = `700 ${Math.max(7, Math.round(r * 1.5))}px system-ui, 'DejaVu Sans', sans-serif`;
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(mk.glyph, cx, my + 0.5);
    g.restore();
  }
  return w;
}

// ---- alerts (part H) ---------------------------------------------------------------------------

/** The log entries to alert among `entries` (newest last): a House's message (`kind: "house"`, or
 *  `mark: "house"` for one the rules leave untagged, a city held or lost at a break-away) and the
 *  news of a secession (`kind: "secession"`, told to everyone who met the liege). `fresh`: the
 *  game's log read for the first time: this turn's only. */
export function houseAlertEntries(entries, { fresh, turn }) {
  return (entries ?? []).filter((e) => {
    if (fresh && e.t !== turn) return false;
    return e.kind === "secession" || e.kind === "house" || e.mark === "house";
  });
}

/** The Houses' alerts shown at once (the newest); the rest are counted in one more toast. */
const ALERTS_SHOWN = 3;

/** Shows the Houses' news among new log `entries` as toasts: the House leader's portrait where the
 *  entry names one of the human's Houses; a click opens the Court at the House (a secession's
 *  news: a click dismisses it). Not while observing. */
export function houseAlerts(entries, { fresh = false } = {}) {
  if (observing()) return;
  const mine = new Map((S.view?.houses ?? []).map((x) => [x.id, x]));
  const all = houseAlertEntries(entries, { fresh, turn: S.view?.turn ?? S.status?.turn });
  // A turn with many (a game's first turns, a load): the newest few, and how many more the log has.
  const shown = all.slice(-ALERTS_SHOWN);
  if (all.length > shown.length) {
    const more = all.length - shown.length;
    toastNode(h("div.toast.log.house-alert.more", { role: "status", title: "Click to dismiss" }, h("span.house-alert-mark", { "aria-hidden": "true" }, "✦"), h("span.house-alert-text", `Houses: ${more} more ${more === 1 ? "message" : "messages"} in the log.`)), 12000);
  }
  // The human's Houses among the players (the players' `house` whose liege is the human), for the
  // news of a House the entry does not tag (its label in the text says which).
  const me_ = S.view?.me?.player;
  const houses = [...(S.players?.values() ?? [])].map((p) => p.house).filter((x) => x && x.liege === me_);
  for (const e of shown) {
    const focus = alertFocus(e, houses);
    const brief = focus ? mine.get(focus.house) : null;
    const face = brief && brief.leader !== null && brief.leader !== undefined ? leaderPortrait(brief.leader, { size: PORTRAIT.small, title: brief.leader_name ?? brief.label }) : null;
    const court = e.kind !== "secession";
    const el = h(
      `div.toast.log.house-alert${e.kind === "secession" ? ".secession" : ""}`,
      { role: "status", title: court ? "Click: open the Court" : "Click to dismiss", "data-house": focus?.house ?? "" },
      face ?? h("span.house-alert-mark", { "aria-hidden": "true" }, e.kind === "secession" ? "✖" : "✦"),
      h("span.house-alert-text", alertBody(e.text, !!focus)),
    );
    if (court) el.addEventListener("click", () => openCourt(focus));
    toastNode(el, 12000);
  }
}

/** Where a tap on a House alert opens the Court (part H): at the House the entry tags (`house`),
 *  else at the House whose label the text names, else at the top. */
export function alertFocus(e, houses = []) {
  if (e.house !== undefined && e.house !== null) return { house: e.house };
  const text = e.text ?? "";
  // The longest label the text names (`House Arne` before a vassal civilization's `Arne`).
  const named = houses.filter((x) => x.label && text.includes(x.label)).sort((a, b) => b.label.length - a.label.length)[0];
  return named ? { house: named.id } : null;
}

/** An alert's text (part H). A change of state (`House Arne is Annoyed.`, `House Arne is
 *  Furious.`): the one mark of the new state over the rest of the text, the
 *  first sentence dropped (the mark says it), or with nothing after it the subject and the mark
 *  (`House Arne ◆ Annoyed`); a House turned Annoyed or Furious adds that it takes no levy (`House
 *  Arne takes no levy.`): the rules' withholding (rule 5). Any other text as it is. `house`: the
 *  alert is about a House. */
export function alertBody(text, house = false) {
  const changed = /^([^.]*) is (Friendly|Pleased|Cautious|Annoyed|Furious)\.\s*(.*)$/.exec(text ?? "");
  if (!changed) return text;
  const [, who, state, rest] = changed;
  const withholds = state === "Annoyed" || state === "Furious";
  const then = withholds && house ? `${who} takes no levy.` : null;
  const head = rest ? [stateMark(state)] : [who, " ", stateMark(state)];
  const tail = [rest || null, then].filter(Boolean);
  return tail.length ? [...head, ...tail.flatMap((t) => [h("br"), t])] : head;
}
