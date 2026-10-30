// Cultural channels (this game's own rules, docs/CULTURE.md): the city screen's Culture section,
// the Cultural pressure panel (F8), and the words and colours of the map's revolt marks.
//
// A city may channel half its culture (rounded down) into one standing order aimed at a
// civilization, until you change it: Foment Revolt (revolt pressure on that civilization's cities
// in reach) or Spread Religion (one roll a turn that one of the city's religions spreads to one of
// their cities). The effect on each city falls with the square of the distance. The city query's
// `cultural` has the channel with what it does now, every channel the city could set with its
// preview (`options`) and the revolt pressure on the city; the buttons send `culture_channel`,
// which the server checks. The `culture` query is the panel. Pressures and amounts are in
// hundredths of culture, spread chances in hundredths of a percent.

import * as api from "./api.js";
import { S, dname, turns, playerName, me, emit, isBarbarian } from "./state.js";
import { h, fill } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { infoTip } from "./infotips.js";
import { playerColor } from "./tiles.js";

// ---------------------------------------------------------------------------------------------
// Numbers and words (pure; assets/test/culture.test.mjs).

/** The revolt levels (`culture::REVOLT_*`) as the screens and the map's marks name them. */
export const REVOLT_LEVELS = ["calm", "unrest", "in revolt", "close to changing hands"];

/** Hundredths of culture for people: 306 -> "3.1", 4210 -> "42", 0 -> "0". */
export function pts(n) {
  const v = (n ?? 0) / 100;
  if (Number.isInteger(v)) return String(v);
  return Math.abs(v) >= 9.95 ? String(Math.round(v)) : v.toFixed(1);
}

/** A chance in hundredths of a percent as the rules' messages write it: 1250 -> "13%",
 *  260 -> "2.6%", 34 -> "0.3%". */
export function chancePct(bp) {
  const n = Math.max(0, bp ?? 0);
  if (n >= 1000 || n % 100 === 0) return `${Math.floor((n + 50) / 100)}%`;
  return `${Math.floor(n / 100)}.${Math.floor((n % 100) / 10)}%`;
}

/** How far a civilization's revolt pressure `p` (hundredths) goes with `rest` from the others: 0
 *  nothing yet, 1 unrest, 2 revolt, 3 close to changing hands, 4 changes hands. Unrest and the
 *  revolt come of the city's total (`p + rest`), the last two of the civilization's own pressure
 *  (`capital`: never changes hands). */
export function pressureStage(p, th, capital = false, rest = 0) {
  if (p >= th.flip && th.flipping && !capital) return 4;
  if (p >= th.near_flip) return 3;
  if (p + rest >= th.disorder) return 2;
  if (p + rest >= th.unhappy_step) return 1;
  return 0;
}

/** The unhappiness revolt pressure `p` brings: a point per step, at most the maximum. */
export function unrestOf(p, th) {
  return Math.min(th.unhappy_max, Math.floor(Math.max(0, p) / Math.max(1, th.unhappy_step)));
}

/** What a civilization's pressure `p` (with `rest` from the others) brings about in a city: "no
 *  effect", "unrest (−2 happy)", "revolt (disorder)", ..., "the city joins <who>". */
export function outcome(p, th, { capital = false, who = "you", rest = 0 } = {}) {
  switch (pressureStage(p, th, capital, rest)) {
    case 4:
      return `the city joins ${who}`;
    case 3:
      return p >= th.flip ? (capital ? "revolt; a capital never changes hands" : "revolt; city flips are off") : "revolt, close to changing hands";
    case 2:
      return "revolt (disorder)";
    case 1:
      return `unrest (−${unrestOf(p + rest, th)} happy)`;
    default:
      return "no effect";
  }
}

// The predictions run the rules' turns against a city's resistance `r`, as the server sends it
// with each Foment Revolt reach entry and each city's `pressure`: `resist` (what it resists now:
// half its culture and its defenders), `resist_revolt` (its defenders' part: all it resists in
// revolt, a city in disorder making no culture), `rest` (the other civilizations' pressure on it,
// taken as constant) and `in_revolt` (in revolt now: its full resistance cannot be read, so it
// resists `resist_revolt` both ways). A city whose total reached the revolt after its turn is in
// revolt through its next turn; it leaves it in the turn the total falls below, and its culture
// resists again from the turn after.

/** What city `r` resists a turn out of revolt, or in revolt (`revolt`). */
function resistOf(r, revolt) {
  const garrison = r.resist_revolt ?? r.resist;
  return revolt || r.in_revolt ? garrison : r.resist;
}

/** One turn of a civilization's revolt pressure `p` in a city (hundredths), as the rules run it:
 *  the inflow in, then the city's resistance (its defenders' alone while `revolt`) and the fade
 *  (`fade_percent`, at least a culture) out. Returns the pressure after it and whether the city
 *  is in revolt in its next turn (the total, with `r.rest`, at the revolt). */
function stepOf(p, inflow, revolt, r, th) {
  const q = p + inflow;
  const next = q > 0 ? Math.max(0, q - resistOf(r, revolt) - Math.max(Math.floor((q * th.fade_percent) / 100), 100)) : 0;
  return [next, next + (r.rest ?? 0) >= th.disorder];
}

/** Where a civilization's pressure, now `now`, settles under `inflow` a turn against city `r`
 *  (hundredths), where the city's turn weighs it, as the server's `settles`: about `(inflow −
 *  resistance) / fade − inflow`, with the resistance of a city in revolt once the total reaches
 *  the revolt (often past the flip). */
export function settleOf(now, inflow, r, th) {
  let p = Math.max(0, now ?? 0);
  let revolt = !!r.in_revolt;
  for (let i = 0; i < 2000; i++) {
    const [next, rv] = stepOf(p, inflow, revolt, r, th);
    if (next === p && rv === revolt) break;
    p = next;
    revolt = rv;
  }
  return p;
}

/** Turns until a civilization's pressure, now `now`, reaches `target` (hundredths) under `inflow`
 *  a turn against city `r`, as the rules run a turn (the inflow, then the city resists and the
 *  pressure fades by `fade_percent`, at least a culture); 0 when it is there, null when it never
 *  gets there (within `limit` turns). */
export function turnsTo(now, inflow, r, target, th, limit = 200) {
  let p = Math.max(0, now ?? 0);
  let revolt = !!r.in_revolt;
  if (p >= target) return 0;
  if (inflow <= Math.min(resistOf(r, false), resistOf(r, true))) return null;
  for (let t = 1; t <= limit; t++) {
    [p, revolt] = stepOf(p, inflow, revolt, r, th);
    if (p >= target) return t;
  }
  return null;
}

/** Turns until pressure `now` with no inflow falls below `level` (hundredths): it loses the
 *  city's resistance (`r`) and the fade each turn. */
export function turnsBelow(now, r, level, th, limit = 400) {
  let p = Math.max(0, now ?? 0);
  let revolt = !!r.in_revolt;
  for (let t = 0; t <= limit; t++) {
    if (p < level) return t;
    [p, revolt] = stepOf(p, 0, revolt, r, th);
  }
  return null;
}

/** "in ~6 turns" / "next turn" for a turn count (null: nothing). */
function soon(n) {
  if (n === null || n === undefined) return null;
  return n <= 1 ? "next turn" : `in ~${n} turns`;
}

/** The next stage a civilization's pressure brings from `now` under `inflow` against city `r`
 *  (with the others' `r.rest`), with when: "revolt in ~6 turns", or null when it is already as
 *  far as it will settle. */
export function nextStage(now, inflow, r, th, capital = false) {
  const rest = r.rest ?? 0;
  const settle = settleOf(now, inflow, r, th);
  const cur = pressureStage(now, th, capital, rest);
  const end = pressureStage(settle, th, capital, rest);
  if (end <= cur) return null;
  // Where this civilization's pressure brings each stage: unrest and the revolt with the others'.
  const marks = [th.unhappy_step - rest, th.disorder - rest, th.near_flip, th.flip];
  const names = ["unrest", "revolt", "close to changing hands", "changes hands"];
  const n = turnsTo(now, inflow, r, marks[cur], th);
  return n === null ? null : `${names[cur]} ${soon(n)}`;
}

/** A Foment Revolt reach entry with what the caster's channels bring there: `inflow` (this
 *  channel's and the caster's other cities'), where it settles, and its stage (0 while this
 *  channel brings nothing: the others' pressure alone is not its effect). */
export function reachRow(x, th) {
  const inflow = x.amount + (x.others ?? 0);
  const settle = settleOf(x.now ?? 0, inflow, x, th);
  return { x, inflow, settle, stage: settle > 0 ? pressureStage(settle, th, x.capital, x.rest ?? 0) : 0 };
}

/** The chooser's one line for a Foment Revolt option: how many cities it reaches and the most it
 *  brings in one of them (the highest stage, then the highest pressure; a capital never changes
 *  hands). */
export function revoltSummary(o, th) {
  const reach = o.reach ?? [];
  const n = reach.length + (o.unseen ?? 0);
  let best = null;
  for (const row of reach.map((x) => reachRow(x, th))) {
    if (!best || row.stage > best.stage || (row.stage === best.stage && row.settle > best.settle)) best = row;
  }
  const what = !best ? "" : best.settle > 0 ? ` · up to ${outcome(best.settle, th, { capital: best.x.capital, rest: best.x.rest ?? 0 })}` : " · too weak for an effect";
  return `${n} cit${n === 1 ? "y" : "ies"} in reach${what}`;
}

/** "5 resisted", "5 resisted (2 in revolt)", "2 resisted (in revolt)": what city `r` resists. */
function resistText(r) {
  const garrison = resistOf(r, true);
  if (r.in_revolt) return `${pts(garrison)} resisted (in revolt)`;
  return garrison < r.resist ? `${pts(r.resist)} resisted (${pts(garrison)} in revolt)` : `${pts(r.resist)} resisted`;
}

// ---------------------------------------------------------------------------------------------
// Names.

/** A civilization by its name ("Sheaim", "the Barbarians", "your own civilization"). */
export function civName(p) {
  const m = me();
  if (m && p === m.player) return "your own civilization";
  const pl = S.players.get(p);
  if (!pl) return playerName(p);
  if (pl.barbarian) return "the Barbarians";
  return pl.civ !== null && pl.civ !== undefined ? dname("CivilizationType", pl.civ) : pl.name;
}

/** "Sheaim (Os-Gabella)": the civilization and its leader. */
function civLabel(p) {
  const m = me();
  const pl = S.players.get(p);
  if ((m && p === m.player) || !pl || pl.barbarian) return civName(p);
  return `${civName(p)} (${pl.name})`;
}

/** Whom a city joins: "you", "the Sheaim", "the Barbarians". */
function joinWho(p) {
  const m = me();
  if (m && p === m.player) return "you";
  const name = civName(p);
  return name.startsWith("the ") ? name : `the ${name}`;
}

/** Whose pressure it is: "Yours", "Sheaim", "the Barbarians". */
function casterName(p) {
  const m = me();
  return m && p === m.player ? "Yours" : civName(p);
}

function colorOf(p) {
  return playerColor(p, isBarbarian(p));
}

/** An action's kind: "foment_revolt" or "spread_religion". */
function kindOf(action) {
  return S.data?.tables?.CulturalActionType?.[action]?.effect ?? null;
}

/** "Foment Revolt", "Spread Religion (Esus)". */
function channelName(o) {
  const name = dname("CulturalActionType", o.action);
  return o.religion !== null && o.religion !== undefined ? `${name} (${dname("ReligionType", o.religion)})` : name;
}

/** "7 plots (51%)" / "4 plots": a reached city's distance and the distance effect. */
function distText(x) {
  return `${x.distance} plot${x.distance === 1 ? "" : "s"}${x.effect < 100 ? ` (${x.effect}%)` : ""}`;
}

const samePick = (a, b) => !!a && !!b && a.action === b.action && a.target === b.target && (a.religion ?? null) === (b.religion ?? null);

// ---------------------------------------------------------------------------------------------
// Pieces.

/** A revolt pressure scale (div.revolt-bar): the pressure in each caster's colour (`parts`:
 *  `[{v, color, label}]`) against the thresholds (each point of unrest, the revolt line, the line
 *  two thirds of the way to changing hands, the line where one civilization's own pressure takes
 *  the city); `mark`: where it settles. `labels`: the threshold names under the scale. */
export function pressureBar(parts, th, { mark = null, labels = false, title = null } = {}) {
  const total = parts.reduce((s, p) => s + Math.max(0, p.v), 0);
  const top = Math.max(th.flip * 1.08, total * 1.04, (mark ?? 0) * 1.04);
  const at = (v) => `${Math.min(100, (100 * v) / top).toFixed(2)}%`;
  const kids = [];
  let x = 0;
  for (const p of parts) {
    if (p.v <= 0) continue;
    kids.push(h("span.revolt-fill", { style: { left: at(x), width: at(p.v), background: p.color }, title: p.label ? `${p.label}: ${pts(p.v)}` : null }));
    x += p.v;
  }
  for (let k = 1; k <= th.unhappy_max; k++) {
    const v = k * th.unhappy_step;
    if (v < th.disorder) kids.push(h("span.revolt-tick", { style: { left: at(v) } }));
  }
  kids.push(h("span.revolt-tick.revolt", { style: { left: at(th.disorder) } }));
  kids.push(h("span.revolt-tick.near", { style: { left: at(th.near_flip) } }));
  if (th.flipping) kids.push(h("span.revolt-tick.flip", { style: { left: at(th.flip) } }));
  if (mark !== null && mark !== undefined) kids.push(h("span.revolt-mark", { style: { left: at(mark) }, title: `Settles near ${pts(mark)}` }));
  const bar = h("div.revolt-bar", { role: "img", "aria-label": `Revolt pressure ${pts(total)}`, title }, kids);
  if (!labels) return bar;
  return h(
    "div.revolt-scale",
    bar,
    h(
      "div.revolt-labels",
      h("span", { style: { left: at(th.unhappy_step) } }, "unrest"),
      h("span", { style: { left: at(th.disorder) } }, "revolt"),
      th.flipping ? h("span", { style: { left: at(th.flip) } }, "joins") : null,
    ),
  );
}

/** The cities a Foment Revolt channel reaches: each with its distance, what it adds a turn
 *  against the city's resistance, a small scale (the caster's pressure there now, where it
 *  settles), what that brings about and when. `who`: whom a city would join. */
function revoltReach(o, th, { who = "you", caster = me()?.player ?? 0 } = {}) {
  const rows = (o.reach ?? []).map((x) => reachRow(x, th)).sort((a, b) => b.settle - a.settle || a.x.distance - b.x.distance);
  const items = rows.map(({ x, inflow, settle, stage }) => {
    const next = nextStage(x.now ?? 0, inflow, x, th, x.capital);
    const owner = x.owner !== o.target ? ` (${civName(x.owner)})` : "";
    return h(
      "li",
      { class: `stage-${stage}`, "data-city": x.city },
      h("div.reach-head", h("b", x.name), owner ? h("span.dim", owner) : null, x.capital ? h("span.dim", { title: "A capital never changes hands" }, " ★") : null, h("span.dim", ` · ${distText(x)}`)),
      pressureBar([{ v: x.now ?? 0, color: colorOf(caster), label: "Now" }], th, { mark: settle, title: `Now ${pts(x.now ?? 0)}; settles near ${pts(settle)}` }),
      h(
        "div.reach-line",
        x.now ? h("span", { title: "The pressure there now (this civilization's)" }, `now ${pts(x.now)}; `) : null,
        `+${pts(x.amount)} a turn`,
        x.others ? h("span.dim", ` (+${pts(x.others)} from other cities)`) : null,
        h("span", { title: "Half its culture and its defenders; in revolt (disorder) it makes no culture, and only its defenders resist" }, `, ${resistText(x)}`),
        settle > 0 ? ` → near ${pts(settle)}: ` : ": ",
        h("span.stage", settle > 0 ? outcome(settle, th, { capital: x.capital, who, rest: x.rest ?? 0 }) : "no effect"),
        next ? h("span.dim", ` · ${next}`) : null,
      ),
    );
  });
  if (o.unseen) items.push(h("li.dim", `and ${o.unseen} cit${o.unseen === 1 ? "y" : "ies"} you have not seen`));
  return h("ul.culture-reach", items.length ? items : h("li.dim", "No city of theirs in reach."));
}

/** The cities a Spread Religion channel reaches: the chance a turn and each city's part (or why
 *  it cannot take the religion). */
function spreadReach(o, th) {
  const rows = [...(o.reach ?? [])].sort((a, b) => b.amount - a.amount || a.distance - b.distance);
  const items = rows.map((x) => {
    const why = x.amount > 0 ? null : x.has ? "has it" : "cannot take it now";
    return h(
      "li",
      { class: x.amount > 0 ? "" : "dim", "data-city": x.city },
      h("b", x.name),
      x.owner !== o.target ? h("span.dim", ` (${civName(x.owner)})`) : null,
      h("span.dim", ` · ${distText(x)}`),
      why ? h("span.dim", ` · ${why}`) : h("span", { title: x.religions ? `Its ${x.religions} religion${x.religions === 1 ? "" : "s"} divide the chance by ${x.religions + 1}` : null }, ` · ${chancePct(x.amount)}${x.religions ? ` (÷${x.religions + 1})` : ""}`),
    );
  });
  if (o.unseen) items.push(h("li.dim", `and ${o.unseen} cit${o.unseen === 1 ? "y" : "ies"} you have not seen (counted in the chance)`));
  const capped = (o.reach ?? []).reduce((s, x) => s + x.amount, 0) > th.spread_max;
  return [
    h(
      "p.culture-chance",
      h("b", `${chancePct(o.chance ?? 0)} a turn`),
      h("span.dim", ` that ${dname("ReligionType", o.religion)} spreads to one of these cities${capped ? ` (at most ${chancePct(th.spread_max)})` : ""}; the city is drawn by its share.`),
    ),
    h("ul.culture-reach.spread", items.length ? items : h("li.dim", "No city of theirs in reach.")),
  ];
}

/** What a channel or option does: its reach, by kind. */
function effectView(o, th, opts) {
  return kindOf(o.action) === "spread_religion" ? spreadReach(o, th) : revoltReach(o, th, opts);
}

/** The revolt pressure on a city (the `pressure` of the city query or of the panel's list): a
 *  scale by caster against the thresholds, the level and its unhappiness, each caster with what
 *  its channels add a turn and where that settles, the city's resistance and how to raise it. */
function pressureView(p, th, { rate = null, name = null, compact = false } = {}) {
  const parts = p.casters.filter((x) => x.pressure > 0).map((x) => ({ v: x.pressure, color: colorOf(x.caster), label: casterName(x.caster) }));
  if (p.unknown) parts.push({ v: p.unknown, color: "#8a8a8a", label: "civilizations you have not met" });
  const resist = [
    rate !== null ? `${th.resist_percent}% of its ${rate} culture` : `${th.resist_percent}% of its culture`,
    `${pts(th.garrison)} for each defender (${p.defenders})`,
  ].join(" and ");
  const garrison = p.resist_revolt ?? p.resist;
  const inRevolt = p.in_revolt
    ? ` In revolt it makes no culture: only its defenders resist (${pts(garrison)}) until the total falls below ${pts(th.disorder)}.`
    : garrison < p.resist
      ? ` In revolt (disorder) it would make no culture: only its defenders would resist (${pts(garrison)}).`
      : "";
  const hint = `It resists ${pts(p.resist)} of each civilization's pressure a turn: ${resist}; the pressure also fades ${th.fade_percent}% a turn.${inRevolt}`;
  const calm = !p.total && !p.level && !p.casters.length;
  if (calm) return h("div.culture-under", h("p.dim", `No revolt pressure${name ? ` on ${name}` : ""}. ${hint}`));
  const stage = Math.max(p.level, pressureStage(p.total, th, p.capital));
  const lead = [...p.casters].sort((a, b) => b.pressure - a.pressure)[0];
  const casters = p.casters.map((x) => {
    // The city's resistance against this civilization: the others' pressure is its `rest`.
    const r = { resist: p.resist, resist_revolt: p.resist_revolt, in_revolt: p.in_revolt, rest: x.rest ?? Math.max(0, p.total - x.pressure) };
    const next = nextStage(x.pressure, x.inflow, r, th, p.capital);
    const falling = !x.inflow && x.pressure > 0 ? turnsBelow(x.pressure, r, th.unhappy_step, th) : null;
    return h(
      "li",
      h("span.swatch", { style: { background: colorOf(x.caster) } }),
      h("b", casterName(x.caster)),
      ` ${pts(x.pressure)}`,
      x.inflow
        ? h("span.dim", ` · +${pts(x.inflow)} a turn → near ${pts(x.settles)}: ${x.settles > 0 ? outcome(x.settles, th, { capital: p.capital, who: joinWho(x.caster), rest: r.rest }) : "no effect"}${next ? `; ${next}` : ""}`)
        : h("span.dim", ` · no channel feeds it: fading${falling !== null && falling > 0 ? `, gone ${soon(falling)}` : ""}`),
    );
  });
  if (p.unknown) casters.push(h("li.dim", `${pts(p.unknown)} from civilizations you have not met`));
  return h(
    "div.culture-under",
    { class: `stage-${stage}` },
    h(
      "div.under-head",
      h("b", `${name ? `${name}: ` : ""}revolt pressure ${pts(p.total)}`),
      h("span.stage", ` · ${REVOLT_LEVELS[Math.min(3, p.level)] ?? "calm"}`),
      p.unhappiness ? h("span.warn", ` · −${p.unhappiness} happy`) : null,
    ),
    pressureBar(parts, th, { labels: !compact, title: `Unrest: a point of unhappiness per ${pts(th.unhappy_step)} (at most ${th.unhappy_max}); revolt (disorder) at ${pts(th.disorder)}; a civilization whose own pressure reaches ${pts(th.flip)} takes the city` }),
    h("ul.culture-casters", casters),
    h(
      "p.dim.culture-hint",
      compact
        ? `It resists ${pts(p.resist)} a turn (${p.defenders} defender${p.defenders === 1 ? "" : "s"} at ${pts(th.garrison)} each, ${p.in_revolt ? "and no culture while in revolt" : `and half its own culture${garrison < p.resist ? `; ${pts(garrison)} in revolt` : ""}`}).`
        : hint,
    ),
    p.capital ? h("p.dim", "A capital never changes hands, whatever the pressure.") : lead && lead.pressure >= th.near_flip ? h("p.warn", `Close to joining ${joinWho(lead.caster)}: station defenders, raise its culture, or make them stop.`) : null,
  );
}

// ---------------------------------------------------------------------------------------------
// The city screen.

/** What the player has picked in a city's channel chooser (kept while the page is open), by city. */
const picks = new Map();

/** The city screen's Culture section: the rate and what the channel takes; the current channel
 *  and what it does now (Stop); the chooser (the action, the religion, the civilization) with the
 *  preview of what it would do before it is set; the revolt pressure on the city. `reload` re-reads
 *  the screen. Null when the game has no cultural actions. */
export function cultureView(c, reload) {
  const k = c.cultural;
  if (!k) return null;
  const el = h("div.section.culture");
  const draw = () => fill(el, ...sectionParts(c, k, reload, draw, el));
  draw();
  return el;
}

/** Runs `reload` (the city screen's, which rebuilds it) keeping the scroll of the dialog's
 *  scrolling parts, so the Culture section stays in view after a change. */
async function reloadInPlace(el, reload) {
  const box = el.closest(".modal");
  const parts = (b) => (b ? [...b.querySelectorAll(".modal-body, .city-col")] : []);
  const kept = parts(box).map((x) => x.scrollTop);
  await reload();
  parts(box).forEach((x, i) => {
    if (kept[i]) x.scrollTop = kept[i];
  });
}

function sectionParts(c, k, reload, draw, el) {
  const th = k.thresholds;
  const ch = k.channel;
  // Re-read either way: a refused command leaves the screen showing the city's state.
  const set = async (cmd) => {
    await run({ c: "culture_channel", city: c.id, ...cmd });
    await reloadInPlace(el, reload);
  };
  const rateLine = h(
    "p.culture-rate",
    `${k.rate} culture a turn`,
    ch
      ? h("span", `: ${k.amount} channelled (${th.channel_percent}%, rounded down), ${Math.max(0, k.rate - k.amount)} to the city`)
      : h("span.dim", ": all of it grows the city's culture and borders"),
  );
  const current = ch
    ? h(
        "div.culture-channel",
        { class: ch.hostile ? "hostile" : "" },
        h(
          "div.channel-head",
          h("button.small.culture-stop", { title: "Stop channelling: all the city's culture goes to the city again", onclick: () => set({ action: null, target: null, religion: null }) }, "Stop"),
          h("span", { tip: infoTip("CulturalActionType", ch.action) }, h("b", channelName(ch))),
          h("span", ` → ${civLabel(ch.target)}`),
        ),
        h(
          "div.dim",
          ch.hostile ? h("span.warn", { title: "Their leader remembers it (not an act of war)" }, "Hostile. ") : null,
          `Since turn ${ch.since_turn}. ${ch.amount ? `${ch.amount} culture a turn goes into it.` : "The city makes too little culture: it channels nothing this turn."}`,
        ),
        effectView(ch, th),
      )
    : null;
  return [h("h4", "Culture"), rateLine, current, chooser(c, k, set, draw), h("h5", "Revolt pressure on this city"), pressureView(k.pressure, th, { rate: k.rate })];
}

/** The channel chooser: the action, the religion (a spread), the civilization; the preview of
 *  the picked option and the button that sets it (or why it cannot). */
function chooser(c, k, set, draw) {
  const th = k.thresholds;
  const actions = (S.data?.tables?.CulturalActionType ?? []).map((_, i) => i);
  const byAction = (a) => k.options.filter((o) => o.action === a);
  const ch = k.channel;
  const pick = { ...(picks.get(c.id) ?? {}) };
  if (pick.action === undefined || !actions.includes(pick.action)) {
    pick.action = ch?.action ?? actions.find((a) => byAction(a).some((o) => o.ok)) ?? actions.find((a) => byAction(a).length) ?? actions[0];
  }
  const opts = byAction(pick.action);
  const spread = kindOf(pick.action) === "spread_religion";
  const religions = [...new Set(opts.map((o) => o.religion).filter((r) => r !== null && r !== undefined))];
  if (spread && !religions.includes(pick.religion)) pick.religion = ch && ch.action === pick.action && religions.includes(ch.religion) ? ch.religion : religions[0];
  if (!spread) pick.religion = null;
  const targets = opts.filter((o) => (o.religion ?? null) === (pick.religion ?? null));
  if (!targets.some((o) => o.target === pick.target)) {
    pick.target = (ch && targets.find((o) => samePick(o, ch)) ? ch.target : null) ?? targets.find((o) => o.ok)?.target ?? targets[0]?.target ?? null;
  }
  picks.set(c.id, pick);
  const choose = (patch) => {
    picks.set(c.id, { ...pick, ...patch });
    draw();
  };
  const sel = targets.find((o) => o.target === pick.target) ?? null;
  const actionRow = h(
    "div.culture-actions",
    { role: "group", "aria-label": "Channel" },
    actions.map((a) =>
      h(
        "button.small",
        { class: a === pick.action ? "on" : "", "aria-pressed": String(a === pick.action), "data-action": a, tip: infoTip("CulturalActionType", a), onclick: () => choose({ action: a, target: undefined, religion: undefined }) },
        dname("CulturalActionType", a),
      ),
    ),
  );
  // Every religion of the city that reaches someone (shown also when there is one: it names it).
  const religionRow =
    spread && religions.length
      ? h(
          "div.culture-religions",
          { role: "group", "aria-label": "Religion" },
          religions.map((r) => h("button.small", { class: r === pick.religion ? "on" : "", "aria-pressed": String(r === pick.religion), "data-religion": r, onclick: () => choose({ religion: r, target: undefined }) }, dname("ReligionType", r))),
        )
      : null;
  const summary = (o) => {
    if (!o.ok) return o.why;
    if (kindOf(o.action) === "spread_religion") return `${chancePct(o.chance ?? 0)} a turn`;
    return revoltSummary(o, th);
  };
  const targetList = targets.length
    ? h(
        "div.culture-targets",
        { role: "radiogroup", "aria-label": "Civilization" },
        targets.map((o) =>
          h(
            "button.culture-target",
            { class: [o.target === pick.target ? "on" : "", o.ok ? "" : "refused", ch && samePick(o, ch) ? "current" : ""].filter(Boolean).join(" "), role: "radio", "aria-checked": String(o.target === pick.target), "data-target": o.target, onclick: () => choose({ target: o.target }) },
            h("span.swatch", { style: { background: colorOf(o.target) } }),
            h("span.target-name", civLabel(o.target)),
            h(
              "span.target-sum",
              ch && samePick(o, ch) ? h("span.good", "channelling now · ") : null,
              o.hostile ? h("span.warn", { title: "Their leader remembers it (not an act of war)" }, "hostile · ") : null,
              h("span", { class: o.ok ? "dim" : "warn" }, summary(o)),
            ),
          ),
        ),
      )
    : h("p.dim.culture-none", emptyText(c, k, pick.action, spread));
  let action = null;
  if (sel) {
    const now = ch && samePick(sel, ch);
    const label = now ? "Channelling this now" : `${ch ? "Switch to" : "Channel"} ${channelName(sel)} → ${civName(sel.target)}`;
    action = h(
      "div.culture-commit",
      h(
        "button.culture-set",
        {
          class: now ? "" : "primary",
          disabled: !sel.ok || now,
          title: !sel.ok ? `Not possible: ${sel.why}` : now ? "This is the city's channel" : ch ? `Replaces ${channelName(ch)} → ${civName(ch.target)}; changing costs nothing` : `The city channels half its culture into it every turn until you stop it`,
          onclick: () => {
            // Set: the chooser folds away under the new channel.
            pick.open = undefined;
            set({ action: sel.action, target: sel.target, religion: sel.religion ?? null });
          },
        },
        label,
      ),
      sel.ok ? null : h("span.warn", ` ${sel.why}.`),
    );
  }
  const least = Math.ceil(100 / Math.max(1, th.channel_percent));
  const lowCulture = k.amount <= 0 ? h("p.warn.culture-low", `At ${k.rate} culture a turn the city would channel nothing (${th.channel_percent}%, rounded down): a channel does nothing until it makes ${least} or more.`) : null;
  // Open while the city channels nothing, folded under a channel; a click on the title is
  // remembered (the screen redraws on every choice).
  return h(
    "details.culture-choose",
    { open: pick.open ?? !ch },
    h("summary", { onclick: (ev) => (pick.open = !ev.currentTarget.parentElement.open) }, ch ? "Change the channel" : "Channel culture"),
    h(
      "p.dim.culture-hint",
      { title: `The effect on each of their cities falls with the square of the distance beyond ${th.full_range} plots.` },
      `Half its culture, every turn, against one civilization until you change it: in full within ${th.full_range} plots, a quarter at ${2 * th.full_range}, nothing beyond ${th.range}.`,
    ),
    actionRow,
    religionRow,
    lowCulture,
    targetList,
    sel && sel.ok && !(ch && samePick(sel, ch)) ? h("div.culture-preview", h("div.preview-head", "What it would do:"), effectView(sel, th)) : null,
    action,
  );
}

/** Why the chooser has no civilization for `action`. */
function emptyText(c, k, action, spread) {
  if (spread) {
    if (!(c.religions ?? []).length) return "This city has no religion to spread.";
    return `No city of a civilization you have met within ${k.thresholds.range} plots could take this city's religions.`;
  }
  return `No city of another civilization you have met is within ${k.thresholds.range} plots.`;
}

// ---------------------------------------------------------------------------------------------
// The Cultural pressure panel (F8).

/** A link that opens one of our cities, or centres the map on a foreign one. */
function cityLink(m, id, name, { own = false, x = null, y = null } = {}) {
  if (own) return h("button.link", { "data-city": id, title: "Open the city", onclick: () => (m.close(null), emit("open-city", id)) }, name);
  if (x === null || x === undefined) return h("b", name);
  return h("button.link", { "data-city": id, title: "Show it on the map", onclick: () => (m.close(null), emit("center", x, y), emit("flash", x, y)) }, name);
}

/** The Cultural pressure panel (F8): our cities under revolt pressure, the channels aimed at us,
 *  our channels with what they do, the channels we see between others, and what other leaders
 *  remember of our cultural actions. */
export async function openCulture() {
  const m = openModal({ title: "Cultural pressure", body: [h("p.dim", "…")], tag: "culture", wide: true, buttons: [{ label: "Close", value: null }] });
  const load = async () => {
    let r;
    try {
      r = await api.query("culture");
    } catch (e) {
      m.setBody(h("p.warn", e.detail ?? e.message));
      return;
    }
    if (m.closed) return;
    if (!r.on) {
      m.setBody(h("p.dim", "This game has no cultural actions (the Cultural actions option is off)."));
      return;
    }
    m.setBody(...panelParts(m, r, load));
  };
  await load();
  return m.done;
}

function panelParts(m, r, reload) {
  const th = r.thresholds;
  const stop = async (city) => {
    if (await run({ c: "culture_channel", city, action: null, target: null, religion: null })) reload();
  };
  const under = r.pressure.length
    ? h(
        "div.culture-list",
        [...r.pressure].sort((a, b) => b.total - a.total).map((p) => h("div.culture-card", { "data-city": p.city }, h("div.card-head", cityLink(m, p.city, p.name, { own: true })), pressureView(p, th, { compact: true }))),
      )
    : h("p.dim", "None of your cities is under revolt pressure.");
  const aimed = r.aimed.length
    ? h(
        "div.culture-list",
        r.aimed.map((ch) =>
          h(
            "div.culture-card",
            { class: ch.hostile ? "hostile" : "" },
            h(
              "div.card-head",
              cityLink(m, ch.city, ch.city_name, { x: ch.x, y: ch.y }),
              h("span.dim", ` (${civName(ch.caster)}) `),
              h("span", { tip: infoTip("CulturalActionType", ch.action) }, h("b", channelName(ch))),
              h("span.dim", ` against you since turn ${ch.since_turn}`),
              ch.hostile ? h("span.warn", " · hostile") : null,
            ),
            ch.reach ? effectView(ch, th, { who: joinWho(ch.caster), caster: ch.caster }) : null,
          ),
        ),
      )
    : h("p.dim", "No civilization channels its culture against you.");
  const ours = r.ours.length
    ? h(
        "div.culture-list",
        r.ours.map((ch) =>
          h(
            "div.culture-card",
            { class: ch.hostile ? "hostile" : "", "data-city": ch.city },
            h(
              "div.card-head",
              h("button.small.culture-stop", { title: "Stop channelling", onclick: () => stop(ch.city) }, "Stop"),
              cityLink(m, ch.city, ch.city_name, { own: true }),
              ": ",
              h("span", { tip: infoTip("CulturalActionType", ch.action) }, h("b", channelName(ch))),
              h("span", ` → ${civLabel(ch.target)}`),
              h("span.dim", ` · ${ch.amount} culture a turn since turn ${ch.since_turn}`),
              ch.hostile ? h("span.warn", " · hostile") : null,
            ),
            effectView(ch, th),
          ),
        ),
      )
    : h("p.dim", "None: set one from a city's screen (its Culture section).");
  const others = r.others.length
    ? h(
        "ul.culture-others",
        r.others.map((ch) =>
          h(
            "li",
            { class: ch.hostile ? "hostile" : "" },
            cityLink(m, ch.city, ch.city_name, { x: ch.x, y: ch.y }),
            h("span.dim", ` (${civName(ch.caster)})`),
            ` ${channelName(ch)} → ${civName(ch.target)}`,
            h("span.dim", ` since turn ${ch.since_turn}`),
          ),
        ),
      )
    : h("p.dim", "None seen.");
  const memories = r.memories.length
    ? h(
        "ul.culture-memories",
        r.memories.map((x) =>
          h(
            "li",
            h("b", civLabel(x.player)),
            x.hostility ? h("span.warn", ` hostility ${x.hostility}/${r.memory_max}`) : null,
            x.friendship ? h("span.good", ` friendship ${x.friendship}/${r.memory_max}`) : null,
          ),
        ),
      )
    : h("p.dim", "Nothing.");
  return [
    h(
      "p.dim",
      `A city may channel half its culture into one standing order aimed at a civilization: Foment Revolt (revolt pressure on its cities in reach: unrest, revolt at ${pts(th.disorder)}, and at ${pts(th.flip)} of one civilization's own pressure the city joins it; never a capital) or Spread Religion (a chance a turn that the religion spreads to one of its cities). The effect falls with the square of the distance beyond ${th.full_range} plots, nothing beyond ${th.range}. No channel is an act of war, and every one is public to the civilizations that have met its caster.`,
    ),
    h("h4", `Your cities under revolt pressure (${r.pressure.length})`),
    under,
    h("h4", `Aimed at you (${r.aimed.length})`),
    aimed,
    h("h4", `Your channels (${r.ours.length})`),
    ours,
    h("h4", `Between others (${r.others.length})`),
    others,
    h("h4", "What other leaders remember of your culture"),
    h("p.dim", "Hostile channels are remembered as hostility, a spread of their own state religion as friendship; it fades with time, and their attitude reads it."),
    memories,
  ];
}
