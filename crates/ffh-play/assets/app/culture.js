// The civilization's cultural channel (this game's own rules: docs/CULTURE.md, "UI"; docs/PERSONS.md,
// "The fourth build", rule 10): the Culture screen (F8, the `culture` query) and the city screen's
// Culture section (the city query's `cultural`).
//
// A civilization has at most one channel: Court a House of another realm, Guard one of its own
// Houses, or Spread Religion. The screen shows, from top to bottom: the head (the channel and its
// inflow), your channel's full preview with Stop, the chooser (the three actions, their targets
// with one line each, a picked target's full preview and Set), the channels aimed at you (each
// courtship of your Houses, with a Guard button and its preview), those between others, and what
// other leaders remember. Every number is the server's (the rules' read API: the view sends the
// parts as fields); this module formats and lays them out, and decides nothing. Amounts of
// culture come in hundredths (shown with one decimal; favour and requirements as whole numbers,
// as the rules' lines write them), chances in hundredths of a percent.

import * as api from "./api.js";
import { S, dname, emit } from "./state.js";
import { h, fill, errorText } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { infoTip } from "./infotips.js";
import { addHelp } from "./help.js";
import { keywordChips, remindersNode } from "./keywords.js";
import { withCalc } from "./calc.js";

// ---------------------------------------------------------------------------------------------
// The help topic: where things are; the rules are the keywords' reminders (from the data).

addHelp({
  id: "culture",
  group: "Game options",
  title: "Culture",
  body: () => {
    const kws = (S.data?.tables?.KeywordType ?? []).filter((k) => ["KEYWORD_CHANNEL", "KEYWORD_COURT", "KEYWORD_GUARD"].includes(k.type) && k.reminder);
    return [
      h("p", "The Culture screen (F8, the menu's Culture) holds your civilization's channel: what it does, Stop, the choice of another, the channels aimed at you, those between others, and what other leaders remember of yours."),
      h("p", "A city's screen says what the city diverts to the channel."),
      remindersNode(kws.map((k) => ({ key: k.type, name: k.name, text: k.reminder }))),
    ];
  },
});

// ---------------------------------------------------------------------------------------------
// Numbers and words (pure; assets/test/culture.test.mjs).

const MINUS = "−";

/** Hundredths with one decimal: 640 -> "6.4", 1200 -> "12.0", -320 -> "−3.2". */
export function tenths(n) {
  const t = Math.round((n ?? 0) / 10);
  const a = Math.abs(t);
  return `${t < 0 ? MINUS : ""}${Math.floor(a / 10)}.${a % 10}`;
}

/** Hundredths as a signed amount a turn: 330 -> "+3.3", with `minus` "−3.3"; 0 -> "0.0". */
export function signed(n, minus = false) {
  if (!n) return "0.0";
  return `${minus ? MINUS : "+"}${tenths(Math.abs(n))}`;
}

/** Hundredths as the whole number the rules' lines write (favour, where it settles): 8099 -> "80". */
export function whole(n) {
  return String(Math.floor((n ?? 0) / 100));
}

/** A chance in hundredths of a percent as the rules write it: 1250 -> "13%", 260 -> "2.6%". */
export function chancePct(bp) {
  const n = Math.max(0, bp ?? 0);
  if (n >= 1000 || n % 100 === 0) return `${Math.floor((n + 50) / 100)}%`;
  return `${Math.floor(n / 100)}.${Math.floor((n % 100) / 10)}%`;
}

/** "1 plot", "7 plots". */
export function plots(n) {
  return `${n} plot${n === 1 ? "" : "s"}`;
}

/** A reach `{distance, effect}`: "7 plots: 51%" (null: "none"). */
export function reachText(r) {
  return r ? `${plots(r.distance)}: ${r.effect}%` : "none";
}

/** The head's two lines from the `culture` query: the channel (`Channel: Court House Aldric,
 *  since turn 140.` / `Channel: none. Your cities keep all their culture.`) and the inflow
 *  (`Inflow: 6.4 culture a turn.` / `A channel would take 6.4 culture a turn.`; how it is made, by
 *  city, is the line's tooltip: the share a channel takes is the keyword Channel's), or why no channel
 *  can be set. */
export function headLines(r) {
  const ch = r.channel;
  const first = ch ? `Channel: ${ch.name}, since turn ${ch.since_turn}.` : "Channel: none. Your cities keep all their culture.";
  if (r.refusal && !ch) return [first, r.refusal];
  const what = `${tenths(r.inflow)} culture a turn`;
  return [first, ch ? `Inflow: ${what}.` : `A channel would take ${what}.`];
}

/** When a split comes, as the rules' lines say it: "a split next turn", "a split in ~6 turns";
 *  null for never. */
export function splitWhen(turns) {
  if (turns === null || turns === undefined) return null;
  return turns <= 1 ? "a split next turn" : `a split in ~${turns} turns`;
}

/** What becomes of a favour: the split's turns, else where it settles under a gain, else
 *  "fading". */
export function prospect(turns, settles, gain) {
  return splitWhen(turns) ?? (gain > 0 ? `settling near ${whole(settles)}` : "fading");
}

/** The turns to the split as a sentence: "Next turn.", "In ~31 turns.", "Never at these
 *  inflows.". */
export function turnsText(turns) {
  if (turns === null || turns === undefined) return "Never at these inflows.";
  return turns <= 1 ? "Next turn." : `In ~${turns} turns.`;
}

/** "+1", "−2", "0". */
function step(n) {
  return n > 0 ? `+${n}` : n < 0 ? `${MINUS}${-n}` : "0";
}

/** "3", "−2". */
function int(n) {
  return n < 0 ? `${MINUS}${-n}` : String(n);
}

/** Whom a courtship's attitude is toward: "you" (`me` the courter), else "the Sheaim". */
function toward(cv, me) {
  return cv.courter === me ? "you" : `the ${cv.courter_name}`;
}

/** The requirement of courtship `cv` as its table: `[label, value]` rows (`Loyalty of House Aldric
 *  to the Bannor: 3`, `Attitude of its leader toward you: Pleased (+1)`, `World culture unit: 24`,
 *  `Requirement: (3 + 6 − 2 × 1) × 24 = 168`). `me`: the human's player. */
export function requirementRows(cv, me) {
  const q = cv.requirement;
  return [
    [`Loyalty of ${cv.label} to the ${cv.liege_name}`, int(q.loyalty)],
    [`Attitude of its leader toward ${toward(cv, me)}`, `${q.attitude_name} (${step(q.attitude)})`],
    ["World culture unit", String(q.unit)],
    ["Requirement", q.formula],
  ];
}

/** Courtship `cv`'s numbers as `[label, value]` rows: the reach, the gain and the guard a turn,
 *  where the favour settles, the turns to the split. */
export function courtRows(cv) {
  return [
    ["Reach", reachText(cv.reach)],
    ["Gain", `${signed(cv.gain)} a turn`],
    ["Guard", cv.guard ? `${signed(cv.guard, true)} a turn` : "none"],
    ["Settles near", whole(cv.settles)],
    ["Split", turnsText(cv.turns)],
  ];
}

/** "Favour 80 of 188". */
export function favourText(favour, total) {
  return `Favour ${whole(favour)} of ${total}`;
}

/** Where a favour bar's fill ends and its mark at two thirds sits, percent of the bar (the bar is
 *  the requirement; a favour past it fills it). */
export function favourBar(favour, total) {
  const need = Math.max(1, 100 * (total ?? 0));
  return { fill: Math.min(100, Math.max(0, (100 * (favour ?? 0)) / need)), mark: 200 / 3 };
}

/** The guard of `gv` a turn: "Guard: −3.2 a turn to each courtship." */
export function guardLossLine(gv) {
  return `Guard: ${signed(gv.loss, true)} a turn to each courtship.`;
}

/** The Guard button's preview (`house_guard_with` at your inflow now): `Guard: −3.2 a turn to each
 *  courtship. The Sheaim: a split in ~6 turns without it, settling near 40 with it.` */
export function guardButtonLine(gv) {
  const parts = gv.courtships.map((c) => `The ${c.courter_name}: ${prospect(c.turns_without, c.settles_without, c.gain)} without it, ${prospect(c.turns_with, c.settles_with, c.gain)} with it.`);
  return [guardLossLine(gv), ...parts].join(" ");
}

/** A guard table's cell: where the favour settles and the split ("near 40; no split", "near 210;
 *  a split in ~12 turns"). */
export function guardCell(turns, settles, gain) {
  const split = splitWhen(turns) ?? "no split";
  return gain > 0 ? `near ${whole(settles)}; ${split}` : `fading; ${split}`;
}

/** The chooser's Set button: "Set", or "Set (replaces Court House Aldric)" while a channel runs. */
export function setLabel(channel) {
  return channel ? `Set (replaces ${channel.name})` : "Set";
}

/** The city screen's line from its `cultural`: `Culture: 12.8 a turn; 6.4 of it to Court House
 *  Aldric.`, `...; nothing diverted in disorder.`, `...; no channel.` */
export function cityCultureLine(k) {
  const head = `Culture: ${tenths(k.rate)} a turn`;
  if (!k.channel) return `${head}; no channel.`;
  if (k.disorder) return `${head}; nothing diverted in disorder.`;
  return `${head}; ${tenths(k.diverted)} of it to ${k.channel.name}.`;
}

/** A channel between others: `Sheaim: Court House Bertram (Kuriotates), since turn 130.` */
export function otherLine(ch) {
  const liege = ch.target?.liege_name ? ` (${ch.target.liege_name})` : "";
  return `${ch.caster_name}: ${ch.name}${liege}, since turn ${ch.since_turn}.`;
}

/** A memory's cells: "2 of 8". */
export function outOf(n, max) {
  return `${n} of ${max}`;
}

// ---------------------------------------------------------------------------------------------
// Pieces.

/** The human's player. */
function mePlayer() {
  return S.view?.me?.player ?? null;
}

/** A city link (`{id, name, own, known, x, y}`): yours opens its screen, one you have seen shows
 *  it on the map, another is its name. `m`: the dialog, closed first. */
function cityLink(m, c) {
  if (c.own) return h("button.link", { "data-city": c.id, title: "Open the city", onclick: () => (m.close(null), emit("open-city", c.id)) }, c.name);
  if (!c.known) return h("span", c.name);
  return h("button.link", { "data-city": c.id, title: "Show it on the map", onclick: () => (m.close(null), emit("center", c.x, c.y), emit("flash", c.x, c.y)) }, c.name);
}

/** Cities as links separated by commas. */
function cityLinks(m, cities) {
  const out = [];
  cities.forEach((c, i) => {
    if (i) out.push(", ");
    out.push(cityLink(m, c));
  });
  return out;
}

/** `[label, value]` rows as a two-column table (`cls` its class). */
/** A table of `[label, value]` rows; `calcOf(label)`: the `[calc, parts]` of a value's tooltip
 *  (calc.js), or null. */
function rowsTable(cls, rows, calcOf = () => null) {
  return h(`table.${cls}`, h("tbody", rows.map(([k, v]) => h("tr", h("th", { scope: "row" }, k), withCalc(h("td", v), ...(calcOf(k) ?? [null]))))));
}

/** The parts of a courtship's calculation each of its rows' tooltips shows (`courtRows`). */
const ROW_PARTS = { Reach: ["reach"], Gain: ["inflow", "reach", "gain"], Guard: ["guard"], "Settles near": ["turn"], Split: ["turn"] };

/** The requirement's table: its parts as rows, the formula last across both columns (`Requirement:
 *  (3 + 6 − 2 × 1) × 24 = 168`: it stays whole on a phone). */
function requirementTable(rows) {
  const parts = rows.slice(0, -1);
  const [label, formula] = rows[rows.length - 1];
  return h("table.culture-req", h("tbody", parts.map(([k, v]) => h("tr", h("th", { scope: "row" }, k), h("td", v))), h("tr.total", h("td", { colSpan: 2 }, h("span.req-label", `${label}: `), h("span.req-formula", formula)))));
}

/** A favour bar: the favour against the requirement, with a mark at two thirds. */
function favourBarView(favour, total, attention) {
  const b = favourBar(favour, total);
  return h(
    "div.favour",
    { class: attention ? "attention" : "" },
    h("div.favour-label", favourText(favour, total)),
    h(
      "div.favour-bar",
      { role: "img", "aria-label": favourText(favour, total) },
      h("span.favour-fill", { style: { width: `${b.fill.toFixed(2)}%` } }),
      h("span.favour-mark", { style: { left: `${b.mark.toFixed(2)}%` } }),
    ),
  );
}

/** A Court preview: the favour bar, the requirement's table, the courtship's numbers, the cities
 *  that would go and those the hold contest keeps. */
function courtView(m, cv) {
  const sp = cv.split;
  const cities = !sp
    ? null
    : sp.whole
      ? h("p.culture-split", "It joins whole.")
      : [
          h("p.culture-split", "Would go: ", sp.going.length ? cityLinks(m, sp.going) : "none", "."),
          sp.held.length ? h("p.culture-held", "Held: ", cityLinks(m, sp.held), ".") : null,
        ];
  const calcOf = (k) => (ROW_PARTS[k] ? [cv.calc, ROW_PARTS[k]] : null);
  return h("div.culture-court", { "data-house": cv.house }, favourBarView(cv.favour, cv.requirement.total, cv.attention), requirementTable(requirementRows(cv, mePlayer())), rowsTable("culture-facts", courtRows(cv), calcOf), cities);
}

/** A guard cell's two parts, each kept whole when the cell wraps ("near 40;" "no split"). */
function cellParts(text) {
  const i = text.indexOf("; ");
  if (i < 0) return text;
  return [h("span.nowrap", text.slice(0, i + 1)), " ", h("span.nowrap", text.slice(i + 2))];
}

/** A Guard preview: what it takes from each courtship, and each courter's favour, settling and
 *  split with the guard and without it. */
function guardView(gv) {
  const head = [withCalc(h("p.culture-loss", guardLossLine(gv)), gv.calc), rowsTable("culture-facts", [["Reach", reachText(gv.reach)]])];
  if (!gv.courtships.length) return h("div.culture-guard", head, h("p.dim", `No civilization courts ${gv.label}.`));
  return h(
    "div.culture-guard",
    { "data-house": gv.house },
    head,
    h(
      "table.culture-guarded",
      h("thead", h("tr", h("th", "Courter"), h("th", "Favour"), h("th", "With guard"), h("th", "Without"))),
      h(
        "tbody",
        gv.courtships.map((c) => h("tr", { "data-courter": c.courter }, h("td", c.courter_name), h("td", `${whole(c.favour)} of ${c.requirement}`), h("td.with", cellParts(guardCell(c.turns_with, c.settles_with, c.gain))), h("td.without", cellParts(guardCell(c.turns_without, c.settles_without, c.gain))))),
      ),
    ),
  );
}

/** A Spread Religion preview: the chance a turn and its cities in reach (city, distance, share). */
function spreadView(m, sp, line) {
  const rows = [...sp.cities].sort((a, b) => b.amount - a.amount || a.distance - b.distance);
  return h(
    "div.culture-spread",
    withCalc(h("p.culture-chance", line), sp.calc),
    rows.length
      ? h(
          "table.culture-reach",
          h("thead", h("tr", h("th", "City"), h("th", "Distance"), h("th", "Share"))),
          h("tbody", rows.map((x) => h("tr", { "data-city": x.city.id }, h("td", cityLink(m, x.city)), h("td", reachText(x)), h("td", x.has ? "has it" : x.can ? chancePct(x.amount) : "cannot take it")))),
        )
      : h("p.dim", "No city of theirs in reach."),
  );
}

/** An order's full preview (`culture_channel_preview`), by its effect. */
function previewView(m, pv) {
  if (pv.court) return courtView(m, pv.court);
  if (pv.guard) return guardView(pv.guard);
  if (pv.spread) return spreadView(m, pv.spread, pv.line);
  return pv.line ? h("p", pv.line) : null;
}

// ---------------------------------------------------------------------------------------------
// The city screen.

/** The city screen's Culture section: the city's culture and what it diverts to the channel, a
 *  link that opens the Culture screen, and the keyword Channel. `reload` re-reads the city screen
 *  (after the Culture screen closes). Null without `cultural`. */
export function cultureView(c, reload) {
  const k = c.cultural;
  if (!k) return null;
  const open = async () => {
    await openCulture();
    await reload?.();
  };
  return h(
    "div.section.culture",
    h("h4", "Culture"),
    withCalc(h("p.culture-line", cityCultureLine(k)), k.calc ? [k.calc] : null),
    h("div.culture-links", h("button.link.culture-open", { title: "Your civilization's channel [F8]", onclick: open }, "Culture screen"), keywordChips(k.reminders)),
  );
}

// ---------------------------------------------------------------------------------------------
// The Culture screen (F8).

/** What the chooser has picked (kept while the page is open): `{action, religion, target}`
 *  (`target`: the target's player, as the command takes it). */
let pick = { action: null, religion: null, target: null };

/** The Culture screen: the head, your channel, the chooser, what is aimed at you, the channels
 *  between others, what others remember, the keywords. */
export async function openCulture() {
  const m = openModal({ title: "Culture", body: [h("p.dim", "…")], tag: "culture", wide: true, buttons: [{ label: "Close", value: null }], help: "culture" });
  const load = async () => {
    let r;
    try {
      r = await api.query("culture");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    const keep = m.el.querySelector(".modal-body")?.scrollTop ?? 0;
    m.setBody(...screenParts(m, r, load));
    const body = m.el.querySelector(".modal-body");
    if (body && keep) body.scrollTop = keep;
  };
  await load();
  return m.done;
}

/** Sends `culture_channel` (`order` null: stop) and re-reads the screen either way (a refused
 *  command leaves it showing the state). */
async function setChannel(order, reload) {
  const cmd = order ? { c: "culture_channel", action: order.action, target: order.target.player, religion: order.religion ?? null } : { c: "culture_channel", action: null, target: null, religion: null };
  await run(cmd);
  await reload();
}

function screenParts(m, r, reload) {
  const [line1, line2] = headLines(r);
  const head = h("div.culture-head", h("p.culture-channel-line", line1), withCalc(h("p.culture-inflow", { class: r.refusal && !r.channel ? "dim" : "" }, line2), r.refusal && !r.channel ? null : r.inflow_calc));
  const ours = r.channel && r.preview
    ? [
        h("h4", "Your channel"),
        h(
          "div.culture-ours",
          { "data-effect": r.channel.effect },
          h("div.culture-ours-head", h("span", { tip: infoTip("CulturalActionType", r.channel.action) }, h("b", r.channel.name)), h("button.small.culture-stop", { title: "Stop the channel: your cities keep all their culture", onclick: () => setChannel(null, reload) }, "Stop")),
          previewView(m, r.preview),
        ),
      ]
    : null;
  const box = h("div.culture-choose");
  const drawChooser = () => fill(box, ...chooserParts(m, r, reload, drawChooser));
  drawChooser();
  return [head, ours, h("h4", "Choose a channel"), box, h("h4", "Aimed at you"), aimedView(m, r, reload), h("h4", "Between others"), othersView(r), h("h4", "What others remember"), memoriesView(r), h("div.culture-keywords", keywordChips(r.reminders))];
}

/** The chooser: the three actions (Spread Religion with its religions), the targets of the picked
 *  one (each with its line, a refused one disabled with its refusal), the picked target's full
 *  preview and Set; one bare line when the action has no target. */
function chooserParts(m, r, reload, draw) {
  const actions = r.actions;
  if (!actions.length) return [h("p.dim", "No cultural action.")];
  if (!actions.some((a) => a.action === pick.action)) {
    pick = { action: r.channel?.action ?? (actions.find((a) => a.options.some((o) => !o.refusal)) ?? actions[0]).action, religion: null, target: null };
  }
  const act = actions.find((a) => a.action === pick.action);
  const spread = act.effect === "spread_religion";
  if (spread && !act.religions.includes(pick.religion)) {
    pick.religion = r.channel?.action === act.action && act.religions.includes(r.channel.religion) ? r.channel.religion : (act.religions[0] ?? null);
  }
  if (!spread) pick.religion = null;
  const choose = (patch) => {
    pick = { ...pick, ...patch };
    draw();
  };
  const actionRow = h(
    "div.culture-actions",
    { role: "group", "aria-label": "Action" },
    actions.map((a) => h("button.small", { class: a.action === pick.action ? "on" : "", "aria-pressed": String(a.action === pick.action), "data-action": a.action, "data-effect": a.effect, tip: infoTip("CulturalActionType", a.action), onclick: () => choose({ action: a.action, religion: null, target: null }) }, a.name)),
  );
  const religionRow =
    spread && act.religions.length
      ? h(
          "div.culture-religions",
          { role: "group", "aria-label": "Religion" },
          act.religions.map((rel) => h("button.small", { class: rel === pick.religion ? "on" : "", "aria-pressed": String(rel === pick.religion), "data-religion": rel, onclick: () => choose({ religion: rel, target: null }) }, dname("ReligionType", rel))),
        )
      : null;
  const options = act.options.filter((o) => (o.religion ?? null) === (pick.religion ?? null));
  if (act.none || !options.length) return [actionRow, religionRow, h("p.culture-none", act.none ?? "No target.")];
  // The picked target's full preview and Set open right under it (a phone shows them together).
  const sel = options.find((o) => o.target.player === pick.target && !o.refusal) ?? null;
  const picked = sel
    ? h(
        "div.culture-pick",
        { "data-target": sel.target.player },
        sel.preview ? previewView(m, sel.preview) : null,
        sel.current
          ? h("p.dim.culture-current", "This is your channel.")
          : h("div.culture-commit", h("button.primary.culture-set", { onclick: () => ((pick = { ...pick, target: null }), setChannel(sel.preview.order, reload)) }, setLabel(r.channel))),
      )
    : null;
  const targets = h(
    "div.culture-targets",
    { role: "radiogroup", "aria-label": "Target" },
    options.map((o) => [
      withCalc(h(
        "button.culture-target",
        {
          class: [o.target.player === pick.target ? "on" : "", o.refusal ? "refused" : "", o.current ? "current" : ""].filter(Boolean).join(" "),
          role: "radio",
          "aria-checked": String(o.target.player === pick.target),
          "data-target": o.target.player,
          disabled: !!o.refusal,
          onclick: () => choose({ target: o.target.player === pick.target ? null : o.target.player }),
        },
        h("span.target-name", o.label),
        h("span.target-line", { class: o.refusal ? "warn" : "dim" }, o.current ? `Your channel. ${o.line}` : (o.refusal ?? o.line)),
      ), o.refusal ? null : (o.preview?.court?.calc ?? o.preview?.guard?.calc ?? o.preview?.spread?.calc ?? null)),
      o === sel ? picked : null,
    ]),
  );
  return [actionRow, religionRow, targets];
}

/** Aimed at you: each courtship of each of your Houses by its line (past two thirds marked as
 *  needing attention), the Guard button with its preview (or that your channel guards it), then
 *  the Spread Religion channels aimed at your civilization. */
function aimedView(m, r, reload) {
  const houses = r.aimed.map((x) => {
    const lines = h(
      "ul.culture-courtships",
      x.courtships.map((cv) => h("li.courtship", { class: cv.attention ? "attention" : "", "data-courter": cv.courter }, cv.attention ? h("span.attention-mark", { "aria-label": "Needs attention", title: "Past two thirds of the requirement" }, "! ") : null, withCalc(h("span.courtship-line", cv.line), cv.calc))),
    );
    const guard = x.guarded
      ? withCalc(h("p.culture-guarded-line", `Your channel guards it. ${guardLossLine(x.guard)}`), x.guard.calc)
      : x.guard_order
        ? h(
            "div.culture-guard-offer",
            h("button.small.culture-guard", { "data-house": x.house, disabled: !!x.guard_refusal, title: x.guard_refusal ?? null, onclick: () => setChannel(x.guard_order, reload) }, r.channel ? `Guard ${x.label} (replaces ${r.channel.name})` : `Guard ${x.label}`),
            withCalc(h("p.culture-guard-preview", { class: x.guard_refusal ? "warn" : "" }, x.guard_refusal ?? guardButtonLine(x.guard)), x.guard_refusal ? null : x.guard.calc),
          )
        : null;
    return h("div.culture-aimed-house", { "data-house": x.house }, h("div.culture-aimed-head", h("b", x.label)), lines, guard);
  });
  const spreads = r.aimed_spread.length ? h("ul.culture-aimed-spread", r.aimed_spread.map((ch) => withCalc(h("li", { class: ch.hostile ? "hostile" : "" }, `${ch.caster_name}: ${ch.name}, since turn ${ch.since_turn}. ${ch.line}`), ch.calc))) : null;
  if (!houses.length && !spreads) return h("p.dim.culture-aimed", "No channel is aimed at you.");
  return h("div.culture-aimed", houses, spreads);
}

function othersView(r) {
  return r.others.length ? h("ul.culture-others", r.others.map((ch) => h("li", { class: ch.hostile ? "hostile" : "" }, otherLine(ch)))) : h("p.dim", "None seen.");
}

function memoriesView(r) {
  if (!r.memories.length) return h("p.dim", "Nothing.");
  const max = r.memory_max;
  return h(
    "table.culture-memories",
    h("thead", h("tr", h("th", "Leader"), h("th", "Hostility"), h("th", "Friendship"), h("th", "Courted their Houses"))),
    h("tbody", r.memories.map((x) => h("tr", { "data-player": x.player }, h("td", `${x.civ} (${x.leader})`), h("td", outOf(x.hostility, max)), h("td", outOf(x.friendship, max)), h("td", outOf(x.courted, max))))),
  );
}
