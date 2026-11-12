// World rituals (docs/CARDS.md, "World rituals"; docs/HUMAN.md): the pure helpers of the rituals
// element and its list (rituals.js), tested in test/rituals.test.mjs. The rows are the view's
// `rituals` (src/view/rituals.rs), every word of the rules written by the server.

/** `1 turn`, `3 turns`. */
export function turnsText(n) {
  return n === 1 ? "1 turn" : `${n} turns`;
}

/** The rows the HUD element shows: rituals in progress (or queued by you) and charges held, in
 *  the server's order (player order). */
export function activeRows(rows) {
  return (rows ?? []).filter((r) => r.state === "in_progress" || r.state === "queued" || r.state === "charged");
}

/** A row's ritual in the HUD element, which is titled "Rituals": its name without the "Ritual: "
 *  of the project's name (`Warcry`); the full name is in its accessible name and the list. */
export function chipName(r) {
  return String(r.ritual_name ?? "").replace(/^Ritual: /, "");
}

/** What a row's status says: `37%` and `6 turns` (no turns while the city makes no production), or
 *  `Charge held`. */
export function rowStatus(r) {
  if (r.state === "charged") return { charged: true, text: "Charge held" };
  const pct = `${r.percent ?? 0}%`;
  const turns = r.turns_left === null || r.turns_left === undefined ? null : turnsText(r.turns_left);
  return { charged: false, pct, turns, text: turns ? `${pct}, ${turns}` : pct };
}

/** A row's accessible name: `Hippus: Ritual: Warcry in Lycus, 37%, 6 turns` or `..., charge held`. */
export function rowLabel(r) {
  const where = r.city_name ? ` in ${r.city_name}` : "";
  const st = rowStatus(r);
  return `${r.name}: ${r.ritual_name}${where}, ${st.charged ? "charge held" : st.text}`;
}

/** The full list's lines of a row: where it stands (`In progress in Lycus: 37/100 (37%, 6 turns).`,
 *  `Queued in Lycus: 0/100.`, `Charge held in Lycus.`, `Idle.`) and its counts (`Completed: 1.
 *  Cost: 200. Next cost: 400.`). */
export function listLines(r) {
  const st = rowStatus(r);
  const where = {
    in_progress: `In progress in ${r.city_name}: ${r.progress}/${r.cost} (${st.text}).`,
    queued: `Queued in ${r.city_name}: ${r.progress}/${r.cost}.`,
    charged: `Charge held in ${r.city_name}.`,
  }[r.state] ?? "Idle.";
  return [where, `Completed: ${r.completed}. Cost: ${r.cost}. Next cost: ${r.next_cost}.`];
}

/** The ritual news (`kind: "ritual"` in the log) to show as alerts among `entries` (newest last):
 *  every one that came after the page had the log (`fresh` false), or, on the first read of a
 *  game's log, those of the current turn `turn`. */
export function ritualAlerts(entries, { fresh, turn }) {
  return (entries ?? []).filter((e) => e.kind === "ritual" && (!fresh || e.t === turn));
}

/** Whether a choice spell's options are each on a plot (`at`: Wellspring's plot and mana): the
 *  plot is picked on the map first, then the option there from a list. */
export function choicesOnPlots(a) {
  const t = a?.targets ?? [];
  return a?.target_kind === "choice" && t.length > 0 && t.every((x) => Array.isArray(x.at));
}

/** What a spell's target is, in words (`target_kind`). */
const TARGET_WORDS = { plot: "a plot", city: "a city", unit: "a unit", player: "a player", choice: "one of its options" };

/** A world action's caster line: `Cast by your Adept near Prespur; then pick its target (a
 *  player).` (`unitName`: the caster's unit type; `caster_where`: the server's `in`/`near` the
 *  human's nearest city, none without a city). */
export function casterLine(a, unitName) {
  const words = choicesOnPlots(a) ? "a plot on the map, then one of its options" : (TARGET_WORDS[a.target_kind] ?? a.target_kind);
  const pick = a.target_kind ? `; then pick its target (${words})` : "";
  return `Cast by your ${unitName}${a.caster_where ? ` ${a.caster_where}` : ""}${pick}.`;
}

// ---------------------------------------------------------------------------------------------
// World spells the player places (`place_on_charge`: Wellspring; the world action's `place`).

/** The placed spells whose placement opens by itself now: those whose charge appeared since the
 *  last view seen (`seen`: a Map of spell -> whether its charge was held, updated here). A spell
 *  seen for the first time (a loaded game, a reload) opens nothing: only the turn its ritual
 *  completed does. */
export function placementDue(seen, actions) {
  const due = [];
  for (const a of actions ?? []) {
    if (!a?.place_on_charge) continue;
    const held = !!a.place;
    if (seen.get(a.spell) === false && held) due.push(a.spell);
    seen.set(a.spell, held);
  }
  return due;
}

/** The placement's prompt: the server's `completed` (the turn the ritual completed) or `pick`
 *  sentence, "click" said "tap" on a touch screen. */
export function placePrompt(a, { completed = false, touch = false } = {}) {
  const p = a?.place ?? {};
  const s = (completed ? p.completed : p.pick) ?? "";
  return touch ? s.replace(/\bclick\b/g, "tap") : s;
}

/** A tier mark's tooltip in the placement's menu (`promotions` of a `place_menu` row): the
 *  promotion's name, then its requirement and its free rule. */
export function tierTipParts(p) {
  return { title: `${p.name}`, lines: [p.needs, p.free].filter(Boolean) };
}

/** A spell chip's tooltip: its name, `Needs Fire I.`, then its help (markup). */
export function spellTipParts(s) {
  return { title: `${s.name}`, lines: [s.needs].filter(Boolean), help: s.help ?? "" };
}

/** A spell chip's accessible name: `Blaze. Needs Fire I.` */
export function spellLabel(s) {
  return [`${s.name}.`, s.needs].filter(Boolean).join(" ");
}

/** The text drawn in place of an effect chip's icon before the art loaded (or without one). */
const CHIP_GLYPHS = { health: "♥", happiness: "☺", affinity: "⚔", buildings: "⌂", info: "i" };
export function chipGlyph(kind) {
  return CHIP_GLYPHS[kind] ?? "?";
}

/** The `icon8/<x>` art name of an effect chip's icon (null: the building's own icon, or the
 *  glyph). */
const CHIP_ART = { health: "healthy", happiness: "happy", affinity: "strength" };
export function chipArt(kind) {
  return CHIP_ART[kind] ?? null;
}
