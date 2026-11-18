// Persons of Note in the existing screens (docs/PERSONS.md, "UI", parts C to I): the House tag of
// the lists, the preview lines under an act, the commander's line, the person's line of the unit
// panel, the House shield on the map's name plates and the alerts from the log. The one mark of a
// state is persons.js `stateMark` (never restyled here); the Court and the grant list are
// persons.js `openCourt` / `openGrant`.

import { S, observing } from "./state.js";
import { h, toastNode } from "./dom.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { stateMark, houseShield, openCourt } from "./persons.js";
import { SHIELD, SHIELD_EDGE, houseColour, shieldMark, stateKey } from "./persons_mark.js";

// The unit figures (look.js) are loaded in the page only: look.js reads the generated art, which
// the node tests of the modules that import this one do not have.
let unitIcon = null;
if (typeof window !== "undefined" && typeof document !== "undefined") {
  import("./look.js").then((m) => (unitIcon = m.unitIcon)).catch(() => {});
}

/** A House in a list or a line: its shield and `House Arne`, and the state mark when it is
 *  Restless or Disloyal (with the countdown's turns). `house` as the host sends it (`{id, label,
 *  colour, state, turns}`); null: `Crown`. `open`: a click opens the Court at the House. */
export function houseTag(house, { open = false, mark = true } = {}) {
  if (!house) return h("span.house-tag.crown", "Crown");
  const k = stateKey(house.state);
  const body = [houseShield(house.colour, 14), h("span.house-name", house.label), mark && (k === "restless" || k === "disloyal") ? [" ", stateMark(house.state, house.turns)] : null];
  if (!open) return h("span.house-tag", { "data-house": house.id }, body);
  return h("button.link.house-tag", { "data-house": house.id, title: "Open the Court at this House", onclick: (ev) => (ev.stopPropagation(), openCourt({ house: house.id })) }, body);
}

/** The preview lines of an act (`[{subject, text, state_after, line}]` from the host), one per
 *  line: `House Arne: Demand refused: -3 (60 turns). Becomes Restless ◆.`, the state after in
 *  the one mark. Null when there are none (nothing is shown). `title`: a line above them. */
export function previewList(lines, { title = null } = {}) {
  if (!lines?.length) return null;
  return h(
    "div.persons-preview",
    title ? h("div.persons-preview-title", title) : null,
    lines.map((l) => {
      const text = `${l.subject}: ${l.text}${/\.$/.test(l.text) ? "" : "."}`;
      return h("div.persons-preview-line", h("span", text), l.state_after ? [" Becomes ", stateMark(l.state_after), "."] : null);
    }),
  );
}

/** The preview lines as plain sentences (for a question's text). */
export function previewText(lines) {
  return (lines ?? []).map((l) => l.line).join(" ");
}

/** `Hrolf (level 7 Axeman)` for a person with a unit. */
function personWithUnit(p, unitName) {
  const lvl = p.level !== null && p.level !== undefined ? `level ${p.level}` : null;
  const type = p.unit_type !== null && p.unit_type !== undefined ? unitName(p.unit_type) : null;
  // (A hero's unit type is its name: `Donal Lugh (level 1)`.)
  const what = [lvl, type && type !== p.name ? type : null].filter(Boolean).join(" ") || null;
  return what ? `${p.name} (${what})` : p.name;
}

/** An army's commander line (part D): `Commander: Hrolf (level 7 Axeman). Loyal ●.` with the
 *  small portrait, or `No commander.`; Restless and Disloyal add `Takes no order to attack.`, a
 *  Disloyal countdown its stated outcome. With more than one person with the army, `[Change]`
 *  (`change(person)` sends the choice). `a`: the army as the host sends it; null when the army
 *  has no `commander` field (a civilization without persons). */
export function commanderLine(a, { unitName, change = null, compact = false } = {}) {
  if (!("commander" in a)) return null;
  const c = a.commander;
  const others = (a.persons_with ?? []).filter((p) => !c || p.person !== c.person);
  const pick = change && others.length
    ? h(
        "select.commander-change",
        {
          "aria-label": "Change the commander",
          title: "Name another person with the army as its commander",
          onchange: (ev) => {
            const v = Number(ev.target.value);
            if (v >= 0) change(v);
          },
        },
        h("option", { value: "-1", selected: true }, "Change…"),
        others.map((p) => h("option", { value: String(p.person) }, personWithUnit(p, unitName))),
      )
    : null;
  if (!c) return h("div.commander-line", { "data-commander": "" }, h("span.dim", "No commander."), pick ? [" ", pick] : null);
  // A person with a leader: the portrait; a hero: its unit's figure (principle 9).
  const face = c.leader !== null && c.leader !== undefined ? leaderPortrait(c.leader, { size: PORTRAIT.small, title: c.name }) : c.unit_type !== null && c.unit_type !== undefined && unitIcon ? h("span.commander-figure", unitIcon(c.unit_type, null)) : null;
  const k = stateKey(c.state);
  return h(
    "div.commander-line",
    { "data-commander": c.person, "data-state": k },
    face,
    h(
      "div.commander-text",
      h(
        "div",
        h("span", "Commander: "),
        h("button.link", { title: "Open the Court at this person", onclick: (ev) => (ev.stopPropagation(), openCourt({ person: c.person })) }, personWithUnit(c, unitName)),
        ". ",
        stateMark(c.state, c.turns),
        pick ? [" ", pick] : null,
      ),
      c.withholds_attack ? h(stateKey(c.state) === "disloyal" ? "div.warn" : "div.caution", "Takes no order to attack.") : null,
      !compact && c.outcome ? h("div.warn", c.outcome) : null,
    ),
  );
}

/** The unit panel's person line (part F): `Person of note: Hrolf, commander of the 2nd Army. Loyal ●.`
 *  (a click opens the Court at the person); null for a unit that is no person. */
export function personLine(u) {
  const p = u?.person;
  if (!p) return null;
  return h(
    "div.person-line",
    { "data-person": p.person },
    h("button.link", { title: "Open the Court at this person", onclick: () => openCourt({ person: p.person }) }, "Person of note"),
    `: ${p.name}, ${p.post}. `,
    stateMark(p.state, p.turns),
  );
}

// ---- the map's name plates (part G) ------------------------------------------------------------

let shieldPath = null;

/** The plate's House shield (the view's `city.house`: `{colour, state, turns}`) drawn on canvas
 *  `g`, `ht` px high, its centre at (`cx`, `cy`): the House's colour with a dark edge; Restless:
 *  the amber ◆ on a dark disc on it; Disloyal: the red ✖ there and the turns left in a pill
 *  under its point (not at its right, where the plate's size disc is). Returns the width it
 *  takes (0 without a House). */
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
  const mk = shieldMark(house.state, house.turns);
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
    if (mk.count !== null && mk.count !== undefined) {
      const txt = String(mk.count);
      const fs = Math.max(8, Math.round(ht * 0.5));
      g.font = `700 ${fs}px system-ui, 'DejaVu Sans', sans-serif`;
      const tw = g.measureText(txt).width;
      const pw = tw + fs * 0.6;
      const ph = fs + 2;
      const px = cx - pw / 2;
      const py = cy + ht * 0.5 - ph * 0.4;
      g.beginPath();
      const rr = ph / 2;
      g.moveTo(px + rr, py);
      g.arcTo(px + pw, py, px + pw, py + ph, rr);
      g.arcTo(px + pw, py + ph, px, py + ph, rr);
      g.arcTo(px, py + ph, px, py, rr);
      g.arcTo(px, py, px + pw, py, rr);
      g.closePath();
      g.fillStyle = "#5a1010";
      g.fill();
      g.lineWidth = 1;
      g.strokeStyle = mk.colour;
      g.stroke();
      g.fillStyle = "#fff0e8";
      g.fillText(txt, px + pw / 2, py + ph / 2 + 0.5);
    }
    g.restore();
  }
  return w;
}

// ---- alerts (part H) ---------------------------------------------------------------------------

/** The log entries to alert among `entries` (newest last): a person's or a House's message
 *  (`kind: "person"`, or `mark: "person"` for a House without a head) and the news of a
 *  secession (`kind: "secession"`, told to everyone who met the owner); not a commander's
 *  routine change of command. `fresh`: the game's log read for the first time: this turn's only. */
export function personAlertEntries(entries, { fresh, turn }) {
  return (entries ?? []).filter((e) => {
    if (fresh && e.t !== turn) return false;
    if (e.kind === "secession") return true;
    if (e.kind !== "person" && e.mark !== "person") return false;
    return !/ (no longer )?commands the /.test(e.text ?? "");
  });
}

/** The persons' alerts shown at once (the newest); the rest are counted in one more toast. */
const ALERTS_SHOWN = 3;

/** Shows the persons' news among new log `entries` as toasts: the subject's portrait where the
 *  entry names a person with a leader; a click opens the Court at the person (a secession's
 *  news: a click dismisses it). Not while observing. */
export function personAlerts(entries, { fresh = false } = {}) {
  if (observing()) return;
  const cast = new Map((S.view?.persons ?? []).map((p) => [p.id, p]));
  const all = personAlertEntries(entries, { fresh, turn: S.view?.turn ?? S.status?.turn });
  // A turn with many (a game's first turns, a load): the newest few, and how many more the log has.
  const shown = all.slice(-ALERTS_SHOWN);
  if (all.length > shown.length) {
    const more = all.length - shown.length;
    toastNode(h("div.toast.log.person-alert.more", { role: "status", title: "Click to dismiss" }, h("span.person-alert-mark", { "aria-hidden": "true" }, "✦"), h("span.person-alert-text", `Persons of note: ${more} more ${more === 1 ? "message" : "messages"} in the log.`)), 12000);
  }
  for (const e of shown) {
    const p = e.person !== undefined && e.person !== null ? cast.get(e.person) : null;
    const face = p && p.leader !== null && p.leader !== undefined ? leaderPortrait(p.leader, { size: PORTRAIT.small, title: p.name }) : null;
    const court = e.kind !== "secession";
    // A change of state (`House Arne is Restless.`): the one mark of the new state over the text.
    const changed = e.kind === "person" ? /^([^.]*) is (Devoted|Loyal|Restless|Disloyal)\.\s*(.*)$/.exec(e.text ?? "") : null;
    const el = h(
      `div.toast.log.person-alert${e.kind === "secession" ? ".secession" : ""}`,
      { role: "status", title: court ? "Click: open the Court" : "Click to dismiss", "data-person": e.person ?? "" },
      face ?? h("span.person-alert-mark", { "aria-hidden": "true" }, e.kind === "secession" ? "✖" : "✦"),
      // `Sera Vell is Disloyal. Sera Vell will seize Vallus ...`: the mark says the state, so the
      // first sentence goes; with nothing after it, the subject and the mark (`House Arne ◆ Restless`).
      h("span.person-alert-text", changed ? (changed[3] ? [stateMark(changed[2]), h("br"), changed[3]] : [changed[1], " ", stateMark(changed[2])]) : e.text),
    );
    if (court) el.addEventListener("click", () => openCourt(e.person !== undefined && e.person !== null ? { person: e.person } : null));
    toastNode(el, 12000);
  }
}
