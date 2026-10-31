// Formation battles as the battle screens read them (docs/HUMAN.md, "The battle screens";
// docs/ENGAGEMENTS.md; the formation-battle prototype, phase 6 revision 4): the protocol's shapes,
// the names and the words of the rules, the editing of a formation, and the pure reading of a
// battle log: the state after any number of its events, the lines of the log, each side's fates.
// No DOM here: assets/test/battle.test.mjs tests it in node.
//
// The screens never re-derive a rule: who stands where, every action, target, roll, hit, heal,
// knock-back and death come from the engine. The client only folds, times and words them (the
// words of a rule are keyed by the codes the engine sends: action kinds, ground rules, notes).
//
// ---------------------------------------------------------------------------------------------
// The protocol fields the screens read (JSON; type-valued fields are indices as everywhere in the
// protocol, or type names as in the dev fixture: `tyName`/`tyIdx` take either; HP in centi-HP
// unless named `max_hp`; A and Dv x100; percents as integers; `b` is a fighter's index in the
// reply's `fighters`; a side is 0 (the attacker) or 1 (the defender); rows are 0 Front, 1 Middle,
// 2 Rear, counted from each side's contact line; lanes from 0).
//
// BattleLog (`battle_log {report}`: `report` is the battle report's id, as `id` is the envelope's
// request id; the `engage` command's reply carries `report` and the report's summary, `battle`):
//   v, id, turn, viewer_side (0 attacker, 1 defender, null: neither)
//   plot, from: PlotSnap {x, y, terrain, feature, hills, river_crossing, city: {id, name, owner, size, defense_pct} | null}
//     (`plot` as it was at the battle, `from` as the viewer sees it now)
//   ground: {city, forest, hills, flanking (bool each: flanking is false in a city, a forest or a jungle),
//     rules: [{kind, pct}]} with kind "CityShots" (the attacker's Shoot deals pct), "ForestRanged" (Shoot and
//     Bombard deal pct on both sides), "HillsAttacker" (every attacker hit deals pct), "HillsDefenderRanged" (the
//     defender's Shoot and Bombard deal pct); empty: open ground
//   slots: [SideSlots, SideSlots] (attacker, defender): {base, changes: [SlotChange], total, river_only}
//     SlotChange {src: "Terrain" (terrain) | "Feature" (feature) | "Hills" | "River" | "CityDefense" (pct, next_at)
//                 | "Bombarded" (pct) | "Minimum" | "Summons" (n), delta}
//   lanes: [attacker's lanes, defender's lanes]
//   sides: [SideSnap, SideSnap]: {player, shown_player, civ, label, army, garrison_of}
//     (`player`: the real one on the viewer's side, the one the viewer saw on the other: hidden nationality)
//   fighters: [Fighter]: both lines, and the units of both sides on the two plots that stayed out of them (the
//     attacker's pool, the defenders' reserve the viewer sees: `lane` null):
//     {b, unit, unit_type, name, owner, side, level, hero, promotions, role (CombatRoleType), large (fills two
//      cells of a row, lanes `lane` and `lane + 1`), lane, row (null: not in the line), max_hp (HP), hp_start,
//      a_x100, dv_x100 (its attack and defence in this battle, before the terms of a pair), speed,
//      actions: [{kind, n}, {kind, n}, {kind, n}] (in its Front, Middle and Rear rows; kind an ActionKind),
//      bonus_action (tier 4 or a hero: the +1 in its best row is in `actions`), spell (SpellType: the damage spell
//      its Spell casts; null: the magic bolt), dodge (points of miss chance added to attacks on it), crit_bonus,
//      anti_flank, withdraw_pct, combat_limit (100: none), splash (its damaging hits splash the target's
//      neighbours), first_strikes}
//   ActionKind: "Strike" | "Charge" | "Stab" | "Shoot" | "Spell" | "Bombard" | "Heal" | "None"
//   events, in order:
//     the opening (round 0, pass 0):
//       {round: 0, pass: 0, kind: "Afraid", actor (the Fear unit, or null), target}: the target sits out the battle
//       {round: 0, pass: 0, kind: "DefStrike", actor (a defender, in the line or not), target, hits: [{target, dmg}]}
//     then the actions, one unit at a time (round 1.., pass 1..):
//       {round, pass, actor, kind (ActionKind), spell? (SpellType; null: the magic bolt), target (null: none),
//        hits: [{target, dmg, crit?, miss?, killed?, withdrew?}] (the first on `target`, the rest its splash; `dmg`
//        what the target lost), heal?: {target, amount}, knocked?: {unit, from, to} (rows), note?: "NoTarget" |
//        "NoOneHurt"}
//   outcome: {plot_taken, city_captured (the city as the attacker's: its id changes with its owner; null), rounds
//     (fought), defenders_left (on the plot after the battle, in the line or not), sides: [{fallen, escaped,
//     standing} (counts of the units in the line), ...]}
//   after: [{kind: "Xp", unit, amount} | {kind: "MoveIn", units, city} | {kind: "Flee", unit, to: {x, y}}] (what the
//     battle did after it: optional, newer kinds are ignored)
//   summary: the report's summary (below)
//
// Battle summary (`battle_reports` -> {on, reports: [summary], newest first}; the view's `battles`, this game turn's
//   and the last; the turn report's `engagements`; the `engage` reply's `battle`): {id, turn, x, y, from: {x, y}, place
//   (the city's name, or null), city, viewer_side, plot_taken, city_captured, rounds, defenders_left, has_log (false:
//   only the summary is kept), sides: [{player, civ, label, slots, lanes, committed, lost, lost_types: [[unit_type,
//   n]], escaped, sat_out, fled, standing}]}. A game-log message about a battle carries `kind: "engagement"` and
//   `report` (its id).
//
// EngagePreview (`engage_preview {army | units | from, to, formation?: [{unit, lane, row}]}`): plot, from, ground,
//   slots, lanes, sides, fighters (the attacker's pool in the formation asked for, checked, else in the rules' auto
//   formation; the defenders it sees in their formation as it knows it), formation and auto_formation ([{unit, lane,
//   row}]: the attacker's in `fighters`, and the rules' own), forecast: Forecast | null (with `estimate_pending`:
//   still being run), declare_war (the team the attack declares war on, or null), errors (why the formation or the
//   attack is refused). Forecast {runs, lost_x100: [attacker, defender] (units expected to fall, x100),
//   line_wiped_permille (the defenders' line wiped out), plot_falls_permille, attackers_wiped_permille?,
//   per: [{b, fall_pct, escape_pct}]}.
// The `engage` command: {army | units | from, to, formation?, declare_war?}; its reply's `r.report` is the battle's.
// Static data (`/api/data`): the names of UnitType, PromotionType, CombatRoleType, TerrainType, FeatureType,
//   SpellType, CivilizationType.
// ---------------------------------------------------------------------------------------------

import { info, tname, tidx, dname } from "./state.js";
import { humanize } from "./tiles.js";

// ---------------------------------------------------------------------------------------------
// Types by index or by name

/** The type name of `v` in `kind` (an index from the protocol, or already a name), or null. */
export function tyName(kind, v) {
  if (v === null || v === undefined || v === -1) return null;
  return typeof v === "string" ? v : tname(kind, v);
}

/** The index of `v` in `kind` (an index, or a type name), or -1. */
export function tyIdx(kind, v) {
  if (v === null || v === undefined) return -1;
  return typeof v === "number" ? v : tidx(kind, v);
}

/** The display name of `v` in `kind` ("Maelstrom"); from the type name when the data lacks it. */
export function tyLabel(kind, v) {
  const i = tyIdx(kind, v);
  if (i >= 0 && info(kind, i)) return dname(kind, i);
  const n = tyName(kind, v);
  return n ? humanize(n) : "-";
}

// ---------------------------------------------------------------------------------------------
// Roles, rows, actions

export const ROWS = ["Front", "Middle", "Rear"];

/** The roles' old names (data from before revision 4) as their new keys. */
const OLD_ROLE = { backline: "archer", support: "arcane" };

/** A role's short key ("flanker" for COMBATROLE_FLANKER), for its icon and its words. */
export function roleKey(f) {
  const n = tyName("CombatRoleType", f?.role);
  const k = n ? n.replace(/^COMBATROLE_/, "").toLowerCase() : "";
  return OLD_ROLE[k] ?? k;
}

/** A role's display name ("Frontline"). */
export function roleLabel(f) {
  const i = tyIdx("CombatRoleType", f?.role);
  if (i >= 0 && info("CombatRoleType", i)) return dname("CombatRoleType", i);
  const k = roleKey(f);
  return k ? k[0].toUpperCase() + k.slice(1) : "-";
}

/** Which units a role takes, in words (the tooltip's gloss). */
export const ROLE_UNITS = {
  frontline: "melee, beasts, animals, recon",
  screen: "a frontline unit with Formation",
  flanker: "mounted",
  assassin: "recon with Marksman",
  archer: "archers",
  siege: "catapults, rams, trebuchets",
  arcane: "adepts, mages",
  divine: "disciples, priests",
};

/** The action kinds: their words, their reach and how they look (`melee`: the unit goes in;
 *  `ranged`: something flies; `phys`: it can miss and, `crit`, be critical). */
export const KINDS = {
  Strike: { verb: "strikes", noun: "strike", nouns: "strikes", reach: "front", melee: true, phys: true, crit: true, knock: true },
  Charge: { verb: "charges", noun: "charge", nouns: "charges", reach: "flank", melee: true, phys: true, crit: true, knock: true },
  Stab: { verb: "stabs", noun: "stab", nouns: "stabs", reach: "any", melee: true, phys: true, crit: true },
  Shoot: { verb: "shoots", noun: "shot", nouns: "shots", reach: "rear", ranged: true, phys: true, crit: true },
  Spell: { verb: "casts at", noun: "spell", nouns: "spells", reach: "rear", ranged: true, magic: true },
  Bombard: { verb: "bombards", noun: "bombard", nouns: "bombards", reach: "rear", ranged: true, phys: true },
  Heal: { verb: "heals", noun: "heal", nouns: "heals", reach: "ally" },
  None: { verb: "waits", noun: "", nouns: "", reach: "none" },
};

/** A kind code as the table's key, whatever its case ("STRIKE", "strike" -> "Strike"). */
export function kindKey(k) {
  if (typeof k !== "string" || !k) return "None";
  const n = k[0].toUpperCase() + k.slice(1).toLowerCase();
  return KINDS[n] ? n : "None";
}

/** A kind's entry of `KINDS`. */
export const kindOf = (k) => KINDS[kindKey(k)];

/** Fighter `f`'s action in row `row`: `{kind, n}` (kind a `KINDS` key). */
export function actionAt(f, row) {
  const a = f?.actions?.[row];
  if (!a || !a.n) return { kind: "None", n: 0 };
  return { kind: kindKey(a.kind), n: a.n };
}

/** The name of the spell fighter `f` (or event `e`) casts: its damage spell's, or the magic bolt. */
export function spellName(spell) {
  return spell === null || spell === undefined ? "Magic bolt" : tyLabel("SpellType", spell);
}

/** "2 strikes", "1 shot", "2 × maelstrom", "no action": `f`'s actions in `row`. */
export function actionText(f, row) {
  const { kind, n } = actionAt(f, row);
  if (!n || kind === "None") return "no action";
  if (kind === "Spell") return `${n} × ${spellName(f.spell).toLowerCase()}`;
  const K = KINDS[kind];
  return `${n} ${n > 1 ? K.nouns : K.noun}`;
}

/** Whom an action of `kind` can hit, in words (`ground`: the battle's, for the charge). */
export function reachText(kind, ground) {
  switch (kindKey(kind)) {
    case "Strike":
      return "the front unit of any enemy lane";
    case "Charge":
      return ground?.flanking === false ? "the front unit of any lane (no flanking here)" : "the most dangerous enemy for its HP left, anywhere; double damage in the first round";
    case "Stab":
      return "any enemy";
    case "Shoot":
    case "Bombard":
      return "the back unit of any enemy lane";
    case "Spell":
      return "the back unit of any enemy lane; never misses";
    case "Heal":
      return "the most hurt ally";
    default:
      return "";
  }
}

// ---------------------------------------------------------------------------------------------
// Fighters and names

/** Whether fighter `f` stands in the line (has a cell). */
export const inLine = (f) => f?.lane !== null && f?.lane !== undefined && f?.row !== null && f?.row !== undefined;
/** The slots fighter `f` takes. */
export const sizeOf = (f) => (f?.large ? 2 : 1);
/** The lanes fighter `f` at `pos` ({lane}) fills. */
export const lanesAt = (f, pos) => (f?.large ? [pos.lane, pos.lane + 1] : [pos.lane]);

/** HP from centi-HP, rounded up (a unit at 0.3 HP still stands). */
export const hpOf = (centi) => Math.max(0, Math.ceil(centi / 100));
/** A hit's or a heal's HP as the numbers show it: at least 1 when anything happened. */
export const dmgText = (centi) => (centi > 0 ? String(Math.max(1, Math.round(centi / 100))) : "0");
/** A x100 value as a short decimal: 1500 -> "15", 440 -> "4.4". */
export const x100 = (v) => (v % 100 === 0 ? String(v / 100) : (v / 100).toFixed(1));

/** The side the viewer is on (0 when the log says nothing: an attack of the viewer's). */
export function viewerSide(log) {
  return log?.viewer_side ?? 0;
}

/** Fighter `b`'s name: "Axeman 2" when its side has more than one of that name (numbered in the
 *  fighters' order), else "Axeman"; `side`: with "your" / "their"; `cap`: capitalized. */
export function uname(log, b, { side = false, cap = false } = {}) {
  const F = log.fighters;
  const f = F[b];
  if (!f) return "?";
  const same = F.filter((g) => g.side === f.side && g.name === f.name);
  let s = f.name;
  if (same.length > 1) s += ` ${same.indexOf(f) + 1}`;
  if (side) s = (f.side === viewerSide(log) ? "your " : "their ") + s;
  if (cap) s = s[0].toUpperCase() + s.slice(1);
  return s;
}

/** "a, b and c". */
export function listNames(names) {
  return names.length <= 1 ? names.join("") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** "Horseman" -> "Horsemen", "Archer" -> "Archers". */
export function plural(n) {
  return n.endsWith("man") ? `${n.slice(0, -3)}men` : n.endsWith("s") ? n : `${n}s`;
}

/** What makes fighter `f` special, in sentences (the tooltip's notes). `ground`: the battle's;
 *  `side`: the side it fights on. Nothing here is computed: each line words one of its fields. */
export function specials(f, ground) {
  const notes = [];
  const role = roleKey(f);
  if (f.large) notes.push("Large: fills two cells side by side and counts two slots; it is the front or back unit of both its lanes; 150% HP.");
  if (f.anti_flank) notes.push("Anti-flanking: enemy flankers attack it first whenever it is in their reach.");
  if (role === "flanker" || f.actions?.some((a) => kindKey(a?.kind) === "Charge")) {
    notes.push(
      `${ground?.flanking === false ? "No flanking in a city, forest or jungle: its charges hit the front unit of a lane." : "Flanks: its charges ride round the line to the enemy with the highest threat for its HP left, wherever it stands, and its first-round charges deal double damage."} Anti-flanking units in its reach come first.`,
    );
  }
  const g = groundNote(f, ground);
  if (g) notes.push(g);
  if (f.actions?.some((a) => kindKey(a?.kind) === "Spell")) {
    notes.push(
      f.spell !== null && f.spell !== undefined
        ? `Casts ${spellName(f.spell)}: a full hit on its target and half on the target's neighbours. Spells never miss.`
        : "Knows no damage spell, so it casts a magic bolt: 60% of a normal hit. Spells never miss.",
    );
  }
  if (f.dodge) notes.push(`Hard to hit: physical attacks on it miss ${f.dodge} points more often.`);
  if (f.crit_bonus) notes.push(`Critical ${f.crit_bonus} points more often.`);
  if (f.withdraw_pct) notes.push(`Withdrawal ${f.withdraw_pct}%: a blow that would kill it is instead escaped ${f.withdraw_pct}% of the time, and it leaves the battle.`);
  if (f.combat_limit !== undefined && f.combat_limit !== null && f.combat_limit < 100) notes.push(`Never takes a target below ${100 - f.combat_limit}% of its HP.`);
  if (f.splash) notes.push("Its hits splash the units beside, in front of and behind the target for half damage.");
  if (f.first_strikes) notes.push(`First strike${f.first_strikes > 1 ? "s" : ""}: +${2 * f.first_strikes} speed.`);
  if (f.bonus_action) notes.push("Tier 4 or a hero: one more action in its best row.");
  return notes;
}

// ---------------------------------------------------------------------------------------------
// The ground

/** The kinds a ground rule touches, and on which side (null: both). */
const RULE_REACH = {
  CityShots: { side: 0, kinds: ["Shoot"] },
  ForestRanged: { side: null, kinds: ["Shoot", "Bombard"] },
  HillsAttacker: { side: 0, kinds: null },
  HillsDefenderRanged: { side: 1, kinds: ["Shoot", "Bombard"] },
};

/** What the ground's rules make of a hit of `kind` by side `side`, in percent (the rules' own
 *  percents multiplied). */
export function groundPct(ground, side, kind) {
  let pct = 100;
  const k = kindKey(kind);
  for (const r of ground?.rules ?? []) {
    const reach = RULE_REACH[r.kind];
    if (!reach || (reach.side !== null && reach.side !== side) || (reach.kinds && !reach.kinds.includes(k))) continue;
    pct = (pct * r.pct) / 100;
  }
  return Math.round(pct);
}

/** What the ground does to fighter `f`'s damage, in a sentence, or "". */
export function groundNote(f, ground) {
  const kinds = [...new Set([0, 1, 2].map((r) => actionAt(f, r).kind))].filter((k) => k !== "None" && k !== "Heal");
  const parts = kinds.map((k) => ({ k, p: groundPct(ground, f.side, k) })).filter((x) => x.p !== 100);
  return parts.length ? `Here its ${parts.map((x) => `${KINDS[x.k].nouns} deal ${x.p}%`).join(", ")} of normal damage.` : "";
}

const half = (p) => (p === 50 ? "half damage" : `${p}%`);
const less = (p) => (p < 100 ? `${100 - p}% less` : `${p - 100}% more`);

/** The ground's rules in words: a sentence each (the plan and the report show them). `plot`:
 *  the defended plot (its feature's name). */
export function groundRules(ground, plot) {
  const rule = (k) => (ground?.rules ?? []).find((r) => r.kind === k);
  const out = [];
  const c = rule("CityShots");
  if (ground?.city || c) out.push(`City: ${c ? `the attackers' arrows deal ${half(c.pct)}` : "no change to damage"}; no flanking.`);
  const f = rule("ForestRanged");
  if (ground?.forest || f) {
    const name = plot?.feature !== null && plot?.feature !== undefined ? tyLabel("FeatureType", plot.feature) : "Forest";
    out.push(`${name}: ${f ? `arrows and stones deal ${half(f.pct)} on both sides` : "no change to damage"}; no flanking.`);
  }
  const ha = rule("HillsAttacker");
  const hd = rule("HillsDefenderRanged");
  if (ground?.hills || ha || hd) {
    const parts = [];
    if (ha) parts.push(`the attackers deal ${less(ha.pct)} damage`);
    if (hd) parts.push(`the defenders' arrows and stones deal ${less(hd.pct)}`);
    out.push(`Hills: ${parts.length ? parts.join("; ") : "no change to damage"}.`);
  }
  if (!out.length) out.push(`Open ground: no damage changes; ${ground?.flanking === false ? "no flanking" : "cavalry flanks"}.`);
  return out;
}

// ---------------------------------------------------------------------------------------------
// Slots

/** What a slot change is called in a side's sum ("for the hill"). */
export function slotChangeText(c) {
  switch (c.src) {
    case "Hills":
      return "for the hill";
    case "Terrain":
      return `for ${tyLabel("TerrainType", c.terrain).toLowerCase()}`;
    case "Feature":
      return `for the ${tyLabel("FeatureType", c.feature).toLowerCase()}`;
    case "River":
      return "across the river";
    case "CityDefense":
      return `for walls at ${c.pct}%`;
    case "Bombarded":
      return `back from the bombardment (${c.pct}%)`;
    case "Minimum":
      return "the minimum";
    case "Summons":
      return "for summons";
    default:
      return humanize(String(c.src));
  }
}

/** "8 slots − 2 for the hill = 6" (the changes that moved the count). */
export function slotSum(s) {
  const parts = [`${s.base} slots`];
  for (const c of s.changes ?? []) {
    if (!c.delta) continue;
    parts.push(`${c.delta > 0 ? "+" : "−"} ${Math.abs(c.delta)} ${slotChangeText(c)}`);
  }
  return `${parts.join(" ")} = ${s.total}`;
}

/** The slots side `s` may fill: its total, and the river's for the units that cross rivers (the
 *  engine checks which unit fills them). */
export const slotCap = (s) => (s?.total ?? 0) + (s?.river_only ?? 0);

// ---------------------------------------------------------------------------------------------
// A formation: where each unit of a side stands (`Map b -> {lane, row}`), and its editing (the
// prototype's: click a unit, then a cell). The engine checks the formation it is sent.

/** The formation of side `side` in `fighters` (their `lane` and `row`). */
export function placeOf(fighters, side) {
  const m = new Map();
  for (const f of fighters) if (f.side === side && inLine(f)) m.set(f.b, { lane: f.lane, row: f.row });
  return m;
}

/** A formation from a protocol list `[{unit, lane, row}]` (unknown units left out). */
export function placeFromList(fighters, list) {
  const m = new Map();
  for (const p of list ?? []) {
    const f = fighters.find((x) => x.unit === p.unit);
    if (f) m.set(f.b, { lane: p.lane, row: p.row });
  }
  return m;
}

/** A formation as the protocol sends it: `[{unit, lane, row}]`, in the fighters' order. */
export function formationOf(place, fighters) {
  return [...place.entries()].sort((a, b) => a[0] - b[0]).map(([b, p]) => ({ unit: fighters[b].unit, lane: p.lane, row: p.row }));
}

/** A key for a formation (the same units in the same cells, in any order). */
export function formationKey(list) {
  return list
    .map((p) => `${p.unit}:${p.lane}:${p.row}`)
    .sort()
    .join(",");
}

/** `grid[row][lane]`: the `b` standing there (a large unit fills two lanes), or null. */
export function gridOf(place, fighters, lanes) {
  const g = Array.from({ length: 3 }, () => Array(lanes).fill(null));
  for (const [b, p] of place) {
    if (p.row < 0 || p.row > 2) continue;
    for (const l of lanesAt(fighters[b], p)) if (l >= 0 && l < lanes) g[p.row][l] = b;
  }
  return g;
}

/** The slots formation `place` uses. */
export function slotsUsed(place, fighters) {
  let n = 0;
  for (const b of place.keys()) n += sizeOf(fighters[b]);
  return n;
}

/** The lanes of `row` where `f` fits in `grid`, nearest `nearLane` first. */
function freeLanes(f, row, nearLane, grid, lanes) {
  const fits = (l) => (f.large ? l + 1 < lanes && grid[row][l] === null && grid[row][l + 1] === null : grid[row][l] === null);
  return [...Array(lanes).keys()].filter(fits).sort((a, b) => Math.abs(a - nearLane) - Math.abs(b - nearLane) || a - b);
}

/**
 * Puts fighter `b` at (`lane`, `row`) of formation `place` (its side's; `lanes` and `cap`: the
 * side's lanes and the slots it may fill): whoever stood in the cells it needs moves to where it
 * stood if they fit there, else out of the line. Returns `{place, out: [b]}` (a new formation and
 * who left the line), or `{error}` (nothing changed).
 */
export function placeUnit(place, fighters, { lanes, cap }, b, lane, row) {
  const f = fighters[b];
  if (!f || row < 0 || row > 2 || lane < 0 || lane >= lanes) return { error: "That is not a cell of its side." };
  if (f.large && lanes < 2) return { error: "There is no room for a large unit here." };
  const anchor = f.large ? Math.min(lane, lanes - 2) : lane;
  const from = place.get(b);
  const next = new Map(place);
  next.delete(b);
  const g0 = gridOf(next, fighters, lanes);
  const cells = f.large ? [anchor, anchor + 1] : [anchor];
  const displaced = [...new Set(cells.map((l) => g0[row][l]).filter((x) => x !== null))];
  for (const o of displaced) next.delete(o);
  next.set(b, { lane: anchor, row });
  const out = [];
  for (const o of displaced) {
    const spot = from ? freeLanes(fighters[o], from.row, from.lane, gridOf(next, fighters, lanes), lanes)[0] : undefined;
    if (spot !== undefined) next.set(o, { lane: spot, row: from.row });
    else out.push(o);
  }
  if (slotsUsed(next, fighters) > cap) return { error: `The line is full: ${cap} slots here, and a large unit takes two. Click a unit in the line to swap them.` };
  return { place: next, out };
}

/** Formation `place` without fighter `b`. */
export function benchUnit(place, b) {
  const next = new Map(place);
  next.delete(b);
  return next;
}

// ---------------------------------------------------------------------------------------------
// The log folded: the state after any number of events

/** The state before the first event: each fighter's HP, status ("on": in the line; "out": not in
 *  it; "afraid": sat out; "dead"; "gone": escaped) and cell. */
export function initialState(log) {
  return {
    hp: log.fighters.map((f) => f.hp_start),
    status: log.fighters.map((f) => (inLine(f) ? "on" : "out")),
    pos: log.fighters.map((f) => (inLine(f) ? { lane: f.lane, row: f.row } : null)),
  };
}

/** A copy of state `st`. */
export function cloneState(st) {
  return { hp: st.hp.slice(), status: st.status.slice(), pos: st.pos.map((p) => (p ? { ...p } : null)) };
}

/** Folds event `e` into state `st` (in place); `F`: the fighters (for their max HP). */
export function applyEvent(st, e, F) {
  if (e.kind === "Afraid") {
    if (st.status[e.target] === "on") st.status[e.target] = "afraid";
    return st;
  }
  for (const h of e.hits ?? []) {
    if (h.miss) continue;
    if (h.withdrew) {
      st.status[h.target] = "gone";
      continue;
    }
    st.hp[h.target] = Math.max(0, st.hp[h.target] - (h.dmg ?? 0));
    if (h.killed) {
      st.hp[h.target] = 0;
      st.status[h.target] = "dead";
    }
  }
  if (e.heal) st.hp[e.heal.target] = Math.min(F[e.heal.target].max_hp * 100, st.hp[e.heal.target] + e.heal.amount);
  if (e.knocked && st.pos[e.knocked.unit]) st.pos[e.knocked.unit] = { ...st.pos[e.knocked.unit], row: e.knocked.to };
  return st;
}

/** The state after the first `n` events. */
export function stateAfter(log, n) {
  const st = initialState(log);
  for (let i = 0; i < Math.min(n, log.events.length); i++) applyEvent(st, log.events[i], log.fighters);
  return st;
}

/** The state after the whole battle. */
export const finalState = (log) => stateAfter(log, log.events.length);

/** Each side's units of the line by fate after state `st`: `[{fallen, escaped, standing, afraid}]`
 *  (fighter indices), and those that stayed out (`out`). */
export function fates(log, st) {
  const out = [0, 1].map(() => ({ fallen: [], escaped: [], standing: [], afraid: [], out: [] }));
  log.fighters.forEach((f, b) => {
    const s = out[f.side];
    if (!s) return;
    const k = st.status[b];
    if (k === "dead") s.fallen.push(b);
    else if (k === "gone") s.escaped.push(b);
    else if (k === "afraid") s.afraid.push(b);
    else if (k === "out") s.out.push(b);
    else s.standing.push(b);
  });
  return out;
}

/** The battle's result for the viewer: `{kind, ribbon, line, good}`: kind "taken" (the plot fell),
 *  "repulsed" (no attacker stands), "held" (the defenders hold). */
export function resultWords(log) {
  const me = viewerSide(log);
  const o = log.outcome ?? {};
  const fs = fates(log, finalState(log));
  const city = log.plot?.city?.name;
  const where = city ?? "the plot";
  if (o.plot_taken) {
    return { kind: "taken", good: me === 0, ribbon: me === 0 ? "VICTORY" : "DEFEAT", line: city ? `${city} falls: no defender is left, and the attackers move in.` : "The plot falls: no defender is left, and the attackers move in." };
  }
  if (!fs[0].standing.length) return { kind: "repulsed", good: me === 1, ribbon: me === 0 ? "REPULSED" : "HELD", line: "The attackers are wiped out. The defenders hold." };
  const left = o.defenders_left ?? fs[1].standing.length;
  return {
    kind: "held",
    good: me === 1,
    ribbon: me === 0 ? "THEY HOLD" : "HELD",
    line: `The defenders hold ${where}: ${left} still stand${left === 1 ? "s" : ""} there. They stay in place until annihilated; the next attack meets what is left.`,
  };
}

// ---------------------------------------------------------------------------------------------
// The log in lines

/** "Round 2 · pass 1" (the opening: "Before the battle"). */
export function headOf(e) {
  return e.round ? `Round ${e.round} · pass ${e.pass}` : "Before the battle";
}

/** Each event's line, with a head line before an event that starts a new round or pass:
 *  `[{i (the event's index), head: true, text} | {i, parts: [{text, cls?, b?}], speed}]`. A part
 *  with `b` names fighter `b` (its `cls` "you" or "them", the viewer's side or not). */
export function logLines(log) {
  const me = viewerSide(log);
  const F = log.fighters;
  const who = (b) => ({ text: uname(log, b), b, cls: F[b]?.side === me ? "you" : "them" });
  const t = (text, cls) => (cls ? { text, cls } : { text });
  const out = [];
  let last = null;
  log.events.forEach((e, i) => {
    const head = headOf(e);
    if (head !== last) {
      out.push({ i, head: true, text: head });
      last = head;
    }
    const parts = [];
    if (e.kind === "Afraid") {
      parts.push(who(e.target), t(" is afraid"));
      if (e.actor !== null && e.actor !== undefined) parts.push(t(" of "), who(e.actor));
      parts.push(t(" and sits out the battle."));
      out.push({ i, parts, speed: null });
      return;
    }
    if (e.kind === "DefStrike") {
      const h = e.hits?.[0];
      parts.push(who(e.actor), t(" strikes "), who(e.target), t(" as it comes in "), t(dmgText(h?.dmg ?? 0), "bt-n"));
      out.push({ i, parts, speed: null });
      return;
    }
    const K = kindOf(e.kind);
    const speed = F[e.actor]?.speed ?? null;
    if (e.heal) {
      parts.push(who(e.actor), t(" heals "), who(e.heal.target), t(" "), t(`+${dmgText(e.heal.amount)}`, "bt-n heal"));
    } else if (e.hits?.length && e.hits[0].miss) {
      parts.push(who(e.actor), t(` ${K.verb} `), who(e.hits[0].target), t(" and misses", "dim"));
    } else if (e.hits?.length) {
      const [first, ...rest] = e.hits;
      const what = kindKey(e.kind) === "Spell" ? `casts ${spellName(e.spell)} at` : K.verb;
      parts.push(who(e.actor), t(` ${what} `), who(first.target), t(" "));
      parts.push(first.withdrew ? t("a killing blow", "dim") : t(dmgText(first.dmg), "bt-n"));
      if (first.crit) parts.push(t(" critical", "crit"));
      if (rest.length) {
        parts.push(t(" splash ", "dim"));
        rest.forEach((h, k) => {
          if (k) parts.push(t(", ", "dim"));
          parts.push(who(h.target), t(` ${h.withdrew ? "escapes" : dmgText(h.dmg)}`, "dim"));
        });
      }
      const fatesText = [];
      for (const h of e.hits) {
        if (h.withdrew) fatesText.push([who(h.target), t(" escapes the blow and leaves the field")]);
        else if (h.killed) fatesText.push([who(h.target), t(" falls")]);
      }
      if (e.knocked) fatesText.push([who(e.knocked.unit), t(` is knocked back to the ${ROWS[e.knocked.to].toLowerCase()} row`)]);
      if (fatesText.length) {
        parts.push(t(". "));
        fatesText.forEach((p, k) => {
          if (k) parts.push(t("; "));
          parts.push(...p);
        });
      }
      parts.push(t("."));
    } else {
      parts.push(who(e.actor), t(` has ${e.note === "NoOneHurt" || kindKey(e.kind) === "Heal" ? "no one to heal" : "no one in reach"}.`, "dim"));
    }
    out.push({ i, parts, speed });
  });
  return out;
}

/** The fighter an event acts on first (its target, or the healed), or null. */
export function eventTarget(e) {
  if (e.heal) return e.heal.target;
  if (e.hits?.length) return e.hits[0].target;
  return e.target ?? null;
}

// ---------------------------------------------------------------------------------------------
// The replay's pace (ms at 1x; the watch screen divides by its speed)

/** When event `e` lands (its impact, where the state changes) and when its motion ends. */
export function timingOf(e) {
  if (e.kind === "Afraid") return { impact: 300, end: 800 };
  if (e.kind === "DefStrike") return { impact: 620, end: 1000 };
  if (e.note || (!e.hits?.length && !e.heal)) return { impact: 150, end: 450 };
  const K = kindOf(e.kind);
  if (K.melee) return { impact: 430, end: 900 };
  if (K.ranged) return { impact: 620, end: 1000 };
  return { impact: 300, end: 700 };
}

// ---------------------------------------------------------------------------------------------
// After the battle

/** XP gained by each fighter after the battle (`after`'s `Xp`). */
export function xpBy(log) {
  const xp = {};
  for (const a of log.after ?? []) if (a.kind === "Xp") xp[a.unit] = (xp[a.unit] ?? 0) + a.amount;
  return xp;
}
