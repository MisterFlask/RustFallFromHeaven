// The battle log and the attack preview as the battle screens read them (docs/HUMAN.md, "The
// battle screens"; the battle UI design, DESIGN.md section 10; phase6_plan 9.2 and 15), and the
// pure reading of the log: the script of segments the watch screen plays, the state folded at any
// moment of it, and what the report derives (fates, the plan checked, the beat lines, what decided
// it). No DOM here: assets/test/battle.test.mjs tests it in node.
//
// The screens never re-derive a rule: who fought, every A and Dv and their parts, every target
// choice with its rule and the set it was drawn from, every roll and every death come from the
// engine. The client only folds, times and words them.
//
// ---------------------------------------------------------------------------------------------
// The protocol fields the screens read (JSON; type-valued fields are indices as everywhere in the
// protocol, or type names as in the dev fixture: `tyName`/`tyIdx` take either; HP in centi-HP
// unless named `max_hp`, A and Dv x100, percents and r as integers; `b` is a fighter's index in
// the reply's `fighters`).
//
// BattleLog (`battle_log {report}`: `report` is the battle report's id, as `id` is the envelope's request id; the
// `engage` command's reply carries `report` and the report's summary, `battle`):
//   v, id, turn, viewer_side (0 attacker, 1 defender, null: neither)
//   plot, from: PlotSnap {x, y, terrain, feature, hills, river_crossing, city: {id, name, owner, size, defense_pct} | null}
//     (`plot` as it was at the battle, `from` as the viewer sees it now)
//   slots: [SideSlots, SideSlots] (attacker, defender): {base, changes: [SlotChange], total, river_only}
//     SlotChange {src: "Terrain" (terrain) | "Feature" (feature) | "Hills" | "River" | "CityDefense" (pct, next_at)
//                 | "Bombarded" (pct) | "Minimum" | "Summons" (n), delta}; the attacker's CityDefense is listed whenever
//                 the city has a defense, with delta 0 while it costs no slot
//   sides: [SideSnap, SideSnap]: {player, shown_player, civ, label, army, garrison_of, stance (null: unseen),
//     stance_known, auto_pick: [b], picked_by: {kind: "Auto"} | {kind: "Player", swaps: [[out b, in b]]}}
//     (`player`: the real one on the viewer's side, the one the viewer saw on the other: hidden nationality, G5)
//   fighters: [Fighter] (committed and staying out, both sides):
//     {b, unit, unit_type, name, owner (as the viewer may see it), side, slot (null: stays out), level, xp, hero, role,
//      pos ("Front" | "Back"), screen, ignores_shield, assassin_profile, guardsman, targets (its target order: "FRONT" |
//      "BACK" | "SCREEN" | "HERO_SUPPORT"), promotions, max_hp, hp_start, volleys, volley_chance, withdraw_pct,
//      def_strike ([chance, dmg] | null), collateral ([dmg %, limit %, max units] | null), combat_limit, immune
//      (["FirstStrikes", ...]), fear, a, dv: Stat {base_x100, role_pct, terms: [Term], total_x100}, fate (how it ended:
//      {kind: "Survived"} | {kind: "Killed", by, in} | {kind: "Withdrew", in, capped} | {kind: "SatOut"}, `in` a
//      {phase: "Opening" | "Volley" | "Round", n?}; null: it stayed out)}
//     Term {src, pct, and the source's payload: "Own" | "Promotion" (promotion) | "Spell" / "BattleEffect" (spell) |
//           "Hills" | "Terrain" (terrain) | "Feature" (feature) | "Fortify" | "CityDefense" | "UnitCityDefense" |
//           "UnitHillsDefense" | "UnitTerrainDefense" (terrain) | "UnitFeatureDefense" (feature) | "CityAttack" |
//           "UnitTerrainAttack" (terrain) | "UnitFeatureAttack" (feature) | "UnitHillsAttack" | "Amphibious" |
//           "VsUnitCombat" (unit_combat, promotion?) | "VsUnitClass" (unit_class, promotion?) | "VsPromotion"
//           (promotion) | "VsAnimal" | "VsBarbarian" | "VsDomain" (domain) | "InBorders" | "GlobalCounter" | "Handicap" |
//           "Aura" (unit) | "Typed" (damage) | "Improvement" (improvement) | "RangeDefense"}
//   pairs: [Pair] (every pair that hit): {h, t, a_x100, dv_x100, mean, a_extra: [Term], dv_extra: [Term]}
//   events, in resolution order, each {kind, ...}:
//     Phase {phase: "PreBattle" | "Morale" | "Opening" | "Volley" | "Round" | "Post", n (Volley, Round)}
//     Morale {unit, source, chance, roll, passed}; SatOut {unit, why: "Afraid" | "EveryTargetCapped"}
//     VolleyRoll {unit, chance, roll, total}
//     DefStrike {striker, target, rolled, chance, roll, pct, dmg}
//     Collateral {source, target, pct, limit_pct, protection_pct, dmg}
//     Hit {h, t, rule, step, blocked: [[step, "Empty" | "Shielded" | "VolleyImmune" | "AtLimit"]], eligible: [b], r, dmg, capped}
//     Withdraw {unit, hp_pct, chance, roll, ok, fatal, hp_kept}
//     RoundEnd {deaths: [{b, killer, assists, overkill}], hp?: [every fighter's HP after the round]} (the engine sends
//       no `hp`: the client folds the round's hits)
//     ShieldDown {side}; End {result: "AttackerWon" | "DefenderHeld" | "Draw" | "Mutual", volley_rounds, rounds}
//     Heal {unit, amount, source: {kind: "Medic", b (null: a medic not in the battle), unit_type, adjacent} |
//       {kind: "OnKill"} | {kind: "Spell", spell}}
//     Xp {unit, amount, from: [{kind: "Kill", victim, share_permille} | {kind: "Withdrew"} | {kind: "DefStrike"}]}
//     KillEffect {killer, victim, effect: {kind: "Capture" | "Enslave" | "Convert" | "Create", unit_type} |
//       {kind: "Gold", amount} | {kind: "Afflict" | "Promotion", promotion}}
//     Flee {unit, to: {x, y}}; Rout {units, to: [{x, y}]}; MoveIn {units, city (the city as the attacker's: its id
//       changes with its owner; null: none, or razed)}
//     (Cast, phase 7: carried through, not drawn yet.) Every Hit of a round comes before that round's Withdraws and
//     RoundEnd; the client folds a round's Withdraws whether they come before or after its RoundEnd.
//   forecast: FrozenForecast | null: {source: "Sampled" | "Expected", runs, win_permille, draw_permille,
//     rounds: {p10, p50, p90}, losses_if_win_x10 (null: never won), per: [{b, fall_pct, withdraw_pct, death_round}],
//     curve: [[centi-HP after Op, R1, ...], [...]] (each side's mean HP), alternatives: [{kind: "StancePick" | "Swap" |
//     "OtherStance", stance, force: [b], win_permille, losses_if_win_x10}]}
//   outcome: {result, volley_rounds, rounds, plot_taken, city_captured, moved_in: [b], defenders_left}
//   summary: the report's summary (below)
//
// Battle summary (`battle_reports` -> {on, reports: [summary], newest first}; the view's `battles`, this game turn's
//   and the last; the turn report's `engagements`; the `engage` reply's `battle`): {id, turn, x, y, from: {x, y}, place
//   (the city's name, or null), city, viewer_side, result, volley_rounds, rounds, plot_taken, city_captured,
//   defenders_left, has_log (false: only the summary is kept), sides: [{player, civ, label, stance (null: unseen),
//   slots, committed, lost, lost_types: [[unit_type, n]], withdrew, sat_out, fled}]}. A game-log message about a
//   battle carries `kind: "engagement"` and `report` (its id).
//
// EngagePreview (`engage_preview {army | units | from, to, force?: [unit ids], stance?}`): plot, from, slots, sides
//   (without auto_pick / picked_by), fighters (the pool and the defenders it sees), pairs (every pool unit with every
//   defender, both ways), force, auto_pick, reserve: [b], stance (the plan's), army_stance (the attack's own),
//   defender_pick: {kind: "Known", units: [b]} | {kind: "Likely", sure: [b], maybe: [b]}, forecast (FrozenForecast
//   without alternatives; null while the engine's estimate is not built: `estimate_pending`), declare_war (the team
//   the attack declares war on, or null), errors.
//   `engage_preview {.., alternatives: true}` -> {alternatives: [{kind, stance, force: [unit ids], win_permille,
//   losses_if_win_x10}]} (the plan's What-if list; a second call so the plan never waits).
// Static data (`/api/data`): the names of UnitType, PromotionType, CombatRoleType, StanceType (and its help),
//   TerrainType, FeatureType, UnitCombatType, UnitClassType, SpellType, CivilizationType.
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

/** The display name of `v` in `kind` ("Formation"); from the type name when the data lacks it. */
export function tyLabel(kind, v) {
  const i = tyIdx(kind, v);
  if (i >= 0 && info(kind, i)) return dname(kind, i);
  const n = tyName(kind, v);
  return n ? humanize(n) : "-";
}

/** A role's short key ("flanker" for COMBATROLE_FLANKER), for its icon and motion. */
export function roleKey(f) {
  const n = tyName("CombatRoleType", f?.role);
  return n ? n.replace(/^COMBATROLE_/, "").toLowerCase() : "";
}

/** Whether fighter `f` is committed (has a slot). */
export const committed = (f) => f.slot !== null && f.slot !== undefined;
/** Whether `f` stands in the back rank. */
export const isBack = (f) => f.pos === "Back";
/** Whether `f` hits screens first (the flanker's order). */
export const hitsScreensFirst = (f) => f.targets?.[0] === "SCREEN";

/** HP from centi-HP, rounded up (a unit at 0.3 HP still stands). */
export const hpOf = (centi) => Math.max(0, Math.ceil(centi / 100));
/** A x100 value as a short decimal: 1500 -> "15", 440 -> "4.4". */
export const x100 = (v) => (v % 100 === 0 ? String(v / 100) : (v / 100).toFixed(1));

/** The side the viewer is on (0 when the log says nothing: an attack of the viewer's). */
export function viewerSide(log) {
  return log.viewer_side ?? 0;
}

// ---------------------------------------------------------------------------------------------
// Names

/** "Axeman L2" when another unit of its side has its type, else "Axeman" ("Archer L1 (2)" for the
 *  second of a type and level); `side`: with "your" / "their"; `cap`: capitalized. */
export function uname(log, b, { side = false, cap = false } = {}) {
  const F = log.fighters;
  const f = F[b];
  if (!f) return "?";
  const same = F.filter((g) => g.side === f.side && sameType(g.unit_type, f.unit_type));
  let s = f.name;
  if (same.length > 1) {
    s += ` L${f.level}`;
    // two of a type and level: the second and later ones are numbered
    const twins = same.filter((g) => g.level === f.level);
    const k = twins.indexOf(f);
    if (k > 0) s += ` (${k + 1})`;
  }
  if (side) s = (f.side === viewerSide(log) ? "your " : "their ") + s;
  if (cap) s = s[0].toUpperCase() + s.slice(1);
  return s;
}

function sameType(a, b) {
  return a === b || tyName("UnitType", a) === tyName("UnitType", b);
}

/** "Mounted" -> "Mounted"; "Horseman" -> "Horsemen", "Archer" -> "Archers". */
export function plural(n) {
  return n.endsWith("man") ? `${n.slice(0, -3)}men` : n.endsWith("s") ? n : `${n}s`;
}

/** "a, b and c". */
export function listNames(names) {
  return names.length <= 1 ? names.join("") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** "the 2 Horsemen and Archer L1": several of a type counted, the rest by name. */
export function groupNames(log, bs) {
  const by = new Map();
  for (const b of bs) {
    const k = tyName("UnitType", log.fighters[b].unit_type) ?? log.fighters[b].name;
    by.set(k, [...(by.get(k) ?? []), b]);
  }
  return listNames([...by.values()].map((g) => (g.length > 1 ? `the ${g.length} ${plural(log.fighters[g[0]].name)}` : uname(log, g[0]))));
}

// ---------------------------------------------------------------------------------------------
// Terms (the parts of A and Dv)

/** How a term's source groups in the parts bar: the place, the unit's own bonuses, promotions and
 *  spells, bonuses against a kind of unit, and the rest. */
const TERM_GROUP = {
  Hills: "place", Terrain: "place", Feature: "place", Fortify: "place", CityDefense: "place",
  UnitCityDefense: "unit", UnitHillsDefense: "unit", UnitTerrainDefense: "unit", UnitFeatureDefense: "unit",
  CityAttack: "unit", UnitTerrainAttack: "unit", UnitFeatureAttack: "unit", Amphibious: "unit",
  UnitHillsAttack: "unit", Own: "unit", Improvement: "place", RangeDefense: "place",
  Promotion: "promo", Spell: "promo", BattleEffect: "promo", Aura: "promo",
  VsUnitCombat: "vs", VsUnitClass: "vs", VsPromotion: "vs", VsAnimal: "vs", VsBarbarian: "vs", VsDomain: "vs", Typed: "vs",
};
export const termGroup = (t) => TERM_GROUP[t.src] ?? "other";

/** A term's name ("Hills", "Formation vs Mounted", "City Garrison I"). */
export function termLabel(t) {
  const from = t.promotion !== undefined && t.promotion !== null ? `${tyLabel("PromotionType", t.promotion)} ` : "";
  switch (t.src) {
    case "Promotion":
      return tyLabel("PromotionType", t.promotion);
    case "Spell":
    case "BattleEffect":
      return tyLabel("SpellType", t.spell);
    case "Hills":
      return "Hills";
    case "Terrain":
      return tyLabel("TerrainType", t.terrain);
    case "Feature":
      return tyLabel("FeatureType", t.feature);
    case "Fortify":
      return "Fortified";
    case "CityDefense":
      return "Walls";
    case "UnitCityDefense":
      return "In a city";
    case "UnitHillsDefense":
      return "On hills";
    case "UnitTerrainDefense":
      return `On ${tyLabel("TerrainType", t.terrain)}`;
    case "UnitFeatureDefense":
      return `In ${tyLabel("FeatureType", t.feature)}`;
    case "CityAttack":
      return "City attack";
    case "UnitTerrainAttack":
      return `Attacking ${tyLabel("TerrainType", t.terrain)}`;
    case "UnitFeatureAttack":
      return `Attacking ${tyLabel("FeatureType", t.feature)}`;
    case "Amphibious":
      return "Amphibious";
    case "VsUnitCombat":
      return `${from}vs ${tyLabel("UnitCombatType", t.unit_combat)}`;
    case "VsUnitClass":
      return `${from}vs ${tyLabel("UnitClassType", t.unit_class)}`;
    case "VsPromotion":
      return `vs ${tyLabel("PromotionType", t.promotion)}`;
    case "VsAnimal":
      return "vs animals";
    case "VsBarbarian":
      return "vs barbarians";
    case "VsDomain":
      return `vs ${tyLabel("DomainType", t.domain)}`;
    case "Own":
      return "Its own";
    case "UnitHillsAttack":
      return "Attacking hills";
    case "Improvement":
      return tyLabel("ImprovementType", t.improvement);
    case "RangeDefense":
      return "Nearby defenses";
    case "InBorders":
      return "In its borders";
    case "GlobalCounter":
      return "Armageddon";
    case "Handicap":
      return "Difficulty";
    case "Aura":
      return "Commander";
    case "Typed":
      return humanize(String(t.damage ?? "damage"));
    default:
      return humanize(String(t.src ?? "?"));
  }
}

/** "+25%" or "-10%". */
export const pctText = (p) => (p >= 0 ? `+${p}%` : `${p}%`);

/** The defended plot's own Dv parts as its defenders get them (the plan's top bar): the place
 *  terms of the committed defenders, the largest of each source. */
export function plotDvTerms(fighters, defSide = 1) {
  const best = new Map();
  for (const f of fighters) {
    if (f.side !== defSide) continue;
    for (const t of f.dv?.terms ?? []) {
      if (termGroup(t) !== "place") continue;
      const k = `${t.src}|${t.terrain ?? ""}|${t.feature ?? ""}`;
      if (!best.has(k) || best.get(k).pct < t.pct) best.set(k, t);
    }
  }
  const order = ["Hills", "Terrain", "Feature", "CityDefense", "Fortify"];
  return [...best.values()].sort((a, b) => order.indexOf(a.src) - order.indexOf(b.src));
}

/** The pair of hitter `h` on target `t` (by `b`), or null. */
export function pairOf(pairs, h, t) {
  return pairs.find((p) => p.h === h && p.t === t) ?? null;
}

// ---------------------------------------------------------------------------------------------
// Slots: the sum in words

/** What a slot change is called in the side header's sum ("for the hill"). */
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

/** The attacker's city-defense note ("Walls 20%: −1 more at 25%"), or null. */
export function wallsNote(s) {
  const c = (s.changes ?? []).find((x) => x.src === "CityDefense");
  return c && c.next_at ? `Walls ${c.pct}%: −1 more at ${c.next_at}%` : null;
}

// ---------------------------------------------------------------------------------------------
// The script: the log cut into segments (the opening's strikes, each volley round and round, the
// end, the aftermath), each with its timings, and the state at each segment's start.

/** ms: every hitter has picked its target at the start of a round. */
const PICK = 260;
/** ms from a hitter's launch to its impact, and to the end of its motion, by kind of motion. */
const LEAD = { anim: 430, shot: 450, bolt: 420, ride: 380, lunge: 210 };
const ACT = { anim: 870, shot: 520, bolt: 480, ride: 720, lunge: 430 };

/** A hit's tier: a glance (under 6 HP), a hit (6-11) or heavy (12 and more). */
export const tierOf = (dmg) => (dmg >= 1200 ? 2 : dmg >= 600 ? 1 : 0);

/** How a fighter strikes: its animation's strike, or an arrow, a bolt, a ride or a lunge. */
export function motionOf(f, animated = () => false) {
  if (animated(f)) return "anim";
  const r = roleKey(f);
  if (r === "backline") return "shot";
  if (r === "support") return "bolt";
  if (r === "flanker" || r === "assassin") return "ride";
  return "lunge";
}

function newSeg(type, extra = {}) {
  return { type, n: 0, hits: [], wds: [], deaths: [], shieldDowns: [], strikes: [], satOut: [], morale: [], post: [], roundEnd: null, ...extra };
}

/** Cuts `log.events` into segments (no timings). */
export function segments(log) {
  const segs = [];
  let cur = null;
  const opening = () => {
    if (!cur || cur.type !== "strikes") cur = pushSeg(newSeg("strikes"));
    return cur;
  };
  const round = () => {
    if (!cur || cur.type !== "round") cur = pushSeg(newSeg("round", { kind: "round", n: 0 }));
    return cur;
  };
  const post = () => {
    if (!cur || cur.type !== "post") cur = pushSeg(newSeg("post"));
    return cur;
  };
  function pushSeg(s) {
    segs.push(s);
    return s;
  }
  log.events.forEach((e, i) => {
    switch (e.kind) {
      case "Phase":
        if (e.phase === "Volley" || e.phase === "Round") cur = pushSeg(newSeg("round", { kind: e.phase === "Volley" ? "volley" : "round", n: e.n ?? 0, ev0: i }));
        else if (e.phase === "Post") cur = pushSeg(newSeg("post", { ev0: i }));
        else if (e.phase === "Morale") cur = null;
        break;
      case "Morale":
        if (!cur || cur.type !== "morale") cur = pushSeg(newSeg("morale", { ev0: i }));
        cur.morale.push({ ev: e, i });
        break;
      case "SatOut":
        (cur ?? (cur = pushSeg(newSeg("morale", { ev0: i })))).satOut.push({ ev: e, i });
        break;
      case "DefStrike":
      case "Collateral":
        opening().strikes.push({ ev: e, i });
        break;
      case "Hit":
        round().hits.push({ ev: e, i, h: e.h, t: e.t, dmg: e.dmg, r: e.r });
        break;
      case "Withdraw":
        round().wds.push({ ev: e, i });
        break;
      case "RoundEnd":
        round().deaths.push(...e.deaths);
        cur.roundEnd = e;
        break;
      case "ShieldDown":
        round().shieldDowns.push({ side: e.side });
        break;
      case "End":
        cur = pushSeg(newSeg("end", { n: e.rounds, result: e.result, ev0: i }));
        break;
      case "Heal":
      case "Xp":
      case "KillEffect":
      case "Flee":
      case "Rout":
      case "MoveIn":
        post().post.push({ ev: e, i });
        break;
      default:
        break; // VolleyRoll, Cast and anything newer: read where they matter, not staged
    }
  });
  return segs;
}

/** The state of the fighters before the first event. */
export function initialState(log) {
  const F = log.fighters;
  const front = (s) => F.some((f) => f.side === s && committed(f) && !isBack(f));
  return {
    hp: F.map((f) => f.hp_start),
    dead: F.map(() => false),
    out: F.map(() => false),
    sat: F.map(() => false),
    diedIn: F.map(() => null),
    wdIn: F.map(() => null),
    shield: [front(0), front(1)],
    luck: [
      [0, 0],
      [0, 0],
    ],
    dealt: [0, 0],
  };
}

function cloneState(st) {
  return {
    hp: st.hp.slice(),
    dead: st.dead.slice(),
    out: st.out.slice(),
    sat: st.sat.slice(),
    diedIn: st.diedIn.slice(),
    wdIn: st.wdIn.slice(),
    shield: st.shield.slice(),
    luck: st.luck.map((x) => x.slice()),
    dealt: st.dealt.slice(),
  };
}

/** The round number a segment's events belong to (0: the opening and its volleys). */
export const roundNo = (s) => (s.type === "round" && s.kind === "round" ? s.n : 0);

/** Folds segment `s`'s effects into `st` up to its local time `t` (Infinity: all of it). */
export function applySeg(st, s, F, t = Infinity) {
  if (s.type === "morale") {
    for (const x of s.satOut) if (t >= (s.satAt ?? 0)) st.sat[x.ev.unit] = true;
    return;
  }
  if (s.type === "strikes") {
    for (const x of s.strikes) if (t >= (x.impact ?? 0)) st.hp[x.ev.target] = Math.max(0, st.hp[x.ev.target] - x.ev.dmg);
    return;
  }
  if (s.type !== "round") return;
  if (t >= (s.land ?? 0)) {
    for (const h of s.hits) {
      st.hp[h.t] = Math.max(0, st.hp[h.t] - h.dmg);
      const side = F[h.h].side;
      st.luck[side][0] += h.r;
      st.luck[side][1] += 1;
      st.dealt[side] += h.dmg;
    }
  }
  for (const w of s.wds) {
    if (t < (w.result ?? 0) || !w.ev.ok) continue;
    st.out[w.ev.unit] = true;
    st.wdIn[w.ev.unit] = roundNo(s);
    if (w.ev.fatal && w.ev.hp_kept !== null && w.ev.hp_kept !== undefined) st.hp[w.ev.unit] = w.ev.hp_kept;
  }
  if (t >= (s.deathStart ?? 0)) {
    for (const d of s.deaths) {
      st.dead[d.b] = true;
      st.diedIn[d.b] = roundNo(s);
    }
  }
  for (const sd of s.shieldDowns) if (t >= (sd.start ?? 0)) st.shield[sd.side] = false;
}

/** Sets a segment's timings (ms from its start) for watch mode `mode` ("full" or "beats"). */
export function timeSeg(s, F, { mode = "full", animated } = {}) {
  const fast = mode === "beats";
  const quiet = fast && !s.beat;
  if (s.type === "morale") {
    s.satAt = 400;
    s.dur = fast ? 1200 : 2000;
    return;
  }
  if (s.type === "strikes") {
    s.strikes.forEach((x, k) => {
      x.start = 300 + k * (fast ? 160 : 260);
      x.impact = x.start + 450;
    });
    s.land = Math.max(0, ...s.strikes.map((x) => x.impact)) + 200;
    s.dur = s.land + (fast ? 500 : 900);
    return;
  }
  if (s.type === "end") {
    s.dur = fast ? 1500 : 2200;
    return;
  }
  if (s.type === "post") {
    s.dur = fast ? 1800 : 2600;
    return;
  }
  // Full: every round at the same pace. Beats: quiet rounds at about 0.6 s, beat rounds faster.
  const sc = quiet ? 0.5 : fast ? 0.7 : 1;
  const pick = quiet ? 60 : fast ? 150 : PICK;
  const stag = quiet ? 14 : fast ? 30 : 55;
  s.pick = pick;
  s.hits.forEach((h, k) => {
    h.start = pick + k * stag;
    h.kind = motionOf(F[h.h], animated);
    h.scale = sc;
    h.impact = h.start + LEAD[h.kind] * sc;
    h.end = h.start + ACT[h.kind] * sc;
    h.tier = tierOf(h.dmg);
  });
  s.land = s.hits.length ? Math.max(...s.hits.map((h) => h.impact)) + (quiet ? 60 : fast ? 120 : 200) : pick;
  let t = s.land + (quiet ? 60 : fast ? 140 : 260);
  const k = fast ? 0.6 : 1;
  for (const w of s.wds) {
    w.start = t;
    w.result = t + 700 * k;
    w.walkEnd = w.result + (w.ev.ok ? 1100 * k : 0);
    t += (w.ev.ok ? 1500 : 1200) * k;
  }
  s.deathStart = t;
  if (s.deaths.length) t += fast ? 900 : 1150;
  for (const sd of s.shieldDowns) {
    sd.start = s.deaths.length ? s.deathStart + 600 : t;
    t = Math.max(t, sd.start + 800);
  }
  s.dur = Math.max(t, s.hits.length ? Math.max(...s.hits.map((h) => h.end)) : 0) + (quiet ? 60 : fast ? 140 : 260);
}

/**
 * The script the watch screen plays: `{segs, final, me}`. `opts`: `mode` ("full" or "beats"),
 * `animated(f)` (the fighter has strike frames), `rulesSeen` (the rule codes the viewer has seen:
 * the focused hit prefers a new one), `lessons(segs, log)` (battle_teach.js `assignLessons`, which
 * gives segments their lessons before the timings and captions), `caption(seg, log, script)`.
 * Each segment gets `beat` (it changes the situation), `focus` (the index of its focused hit, -1:
 * none), `screenGone`, its timings, `caption`, and `snap` (the state at its start).
 */
export function buildScript(log, opts = {}) {
  const F = log.fighters;
  const me = viewerSide(log);
  const segs = segments(log);
  // Their screens are gone: a death leaves the other side with no screen while yours has a flanker.
  const screensOf = (side) => F.filter((f) => f.side === side && committed(f) && f.screen).map((f) => f.b);
  const flankers = F.some((f) => f.side === me && committed(f) && hitsScreensFirst(f));
  const deadSoFar = new Set();
  for (const s of segs) {
    for (const d of s.deaths) deadSoFar.add(d.b);
    s.screenGone =
      flankers && s.deaths.some((d) => F[d.b].side !== me && F[d.b].screen) && screensOf(1 - me).every((b) => deadSoFar.has(b));
    s.beat = s.type !== "round" || s.kind === "volley" || s.deaths.length > 0 || s.wds.length > 0 || s.shieldDowns.length > 0;
  }
  // The focused hit of each round: the first of yours showing a rule the viewer has not seen, else
  // a kill of yours, else your biggest hit (else theirs).
  const seen = new Set(opts.rulesSeen ?? []);
  for (const s of segs) {
    s.focus = -1;
    if (!s.hits.length) continue;
    const mine = (h) => F[h.h].side === me;
    let k = s.hits.findIndex((h) => mine(h) && !seen.has(h.ev.rule));
    if (k < 0) k = s.hits.findIndex((h) => mine(h) && s.deaths.some((d) => d.b === h.t && d.killer === h.h));
    if (k < 0) k = biggest(s.hits, mine);
    if (k < 0) k = biggest(s.hits, () => true);
    s.focus = k;
    seen.add(s.hits[k].ev.rule);
  }
  opts.lessons?.(segs, log);
  for (const s of segs) timeSeg(s, F, opts);
  const script = { segs, log, me, final: null };
  for (const s of segs) s.caption = opts.caption ? opts.caption(s, log, script) : "";
  const st = initialState(log);
  for (const s of segs) {
    s.snap = cloneState(st);
    applySeg(st, s, F, Infinity);
  }
  script.final = st;
  return script;
}

function biggest(hits, pred) {
  let best = -1;
  hits.forEach((h, j) => {
    if (pred(h) && (best < 0 || h.dmg > hits[best].dmg)) best = j;
  });
  return best;
}

/** The state at local time `t` of segment `si`. */
export function stateAt(script, si, t) {
  const s = script.segs[si];
  const st = cloneState(s.snap);
  applySeg(st, s, script.log.fighters, t);
  return st;
}

/** The damage of segment `s` still to land on each fighter at time `t` (the white segments). */
export function pendingAt(s, nF, t) {
  const pending = new Array(nF).fill(0);
  if (s.type === "round" && t < s.land) for (const h of s.hits) if (t >= h.impact) pending[h.t] += h.dmg;
  return pending;
}

// ---------------------------------------------------------------------------------------------
// The report

/** Each fighter of side `side`'s fate: `{b, kind: "out" (stayed out) | "dead" | "withdrew" |
 *  "alive", round, killer, roll, chance, hp, xp, untouched}`. */
export function fates(log, final, side = viewerSide(log)) {
  const xp = xpBy(log);
  return log.fighters
    .filter((f) => f.side === side)
    .map((f) => {
      const b = f.b;
      const w = log.events.find((e) => e.kind === "Withdraw" && e.unit === b);
      const base = { b, xp: xp[b] ?? 0, hp: final.hp[b], untouched: final.hp[b] === f.hp_start };
      if (!committed(f)) return { ...base, kind: "out" };
      if (final.sat[b]) return { ...base, kind: "sat" };
      if (final.dead[b]) {
        const d = log.events.flatMap((e) => (e.kind === "RoundEnd" ? e.deaths : [])).find((x) => x.b === b);
        return { ...base, kind: "dead", round: final.diedIn[b], killer: d?.killer ?? null, roll: w?.roll ?? null, chance: w?.chance ?? null };
      }
      if (final.out[b]) return { ...base, kind: "withdrew", round: final.wdIn[b], roll: w?.roll, chance: w?.chance };
      return { ...base, kind: "alive" };
    });
}

/** XP gained by each fighter (`Xp` events). */
export function xpBy(log) {
  const xp = {};
  for (const e of log.events) if (e.kind === "Xp") xp[e.unit] = (xp[e.unit] ?? 0) + e.amount;
  return xp;
}

/** A forecast item's verdict from the chance the plan gave what happened: "ok" (as planned, 35%
 *  or more), "luck" (rare but inside the plan, 8-35%), "off" (off plan, under 8%). */
export function verdict(p) {
  return p >= 35 ? "ok" : p >= 8 ? "luck" : "off";
}

/** The plan checked against the battle: rows `{verdict, label, result}` (the viewer attacking). */
export function planChecks(log, final) {
  const fc = log.forecast;
  if (!fc) return [];
  const F = log.fighters;
  const me = viewerSide(log);
  const o = log.outcome;
  const won = o.result === "AttackerWon";
  const win = Math.round(fc.win_permille / 10);
  const rows = [{ verdict: verdict(won ? win : 100 - win), label: `win ${win}%`, result: won ? "won" : o.result === "Draw" ? "drawn" : "lost" }];
  const inRange = o.rounds >= fc.rounds.p10 && o.rounds <= fc.rounds.p90;
  rows.push({ verdict: inRange ? "ok" : "luck", label: `${fc.rounds.p10}–${fc.rounds.p90} rounds`, result: String(o.rounds) });
  const lost = F.filter((f) => f.side === me && committed(f) && final.dead[f.b]).length;
  if (won && fc.losses_if_win_x10 !== null && fc.losses_if_win_x10 !== undefined) {
    const exp = fc.losses_if_win_x10 / 10;
    rows.push({ verdict: Math.abs(lost - exp) <= 1.5 ? "ok" : "luck", label: `~${exp.toFixed(1)} lost if you win`, result: String(lost) });
  }
  for (const per of fc.per ?? []) {
    const f = F[per.b];
    if (!f || f.side !== me || !committed(f) || (per.fall_pct < 50 && per.withdraw_pct < 20)) continue;
    const fate = final.dead[f.b] ? "fell" : final.out[f.b] ? "out" : "lived";
    const p = final.dead[f.b] ? per.fall_pct : final.out[f.b] ? per.withdraw_pct : 100 - per.fall_pct - per.withdraw_pct;
    rows.push({ verdict: verdict(p), label: `${uname(log, f.b)} falls ${per.fall_pct}%`, result: fate });
  }
  return rows;
}

/** The beat rounds as report lines: `{seg, label ("Op", "R4"), parts: [{text, cls}]}`. */
export function beatLines(script) {
  const L = script.log;
  const F = L.fighters;
  const me = script.me;
  const o = L.outcome;
  const out = [];
  script.segs.forEach((s, k) => {
    const label = s.type === "round" && s.kind === "round" ? `R${s.n}` : "Op";
    const parts = [];
    if (s.type === "strikes") {
      // the opening's strikes make a line when one of them hurt (10 HP or more)
      const hurt = s.strikes.filter((x) => x.ev.dmg >= 1000 && x.ev.kind === "DefStrike");
      if (hurt.length) parts.push({ text: `the walls' archers hit ${listNames(hurt.map((x) => uname(L, x.ev.target)))}` });
    }
    const theirs = s.deaths.filter((d) => F[d.b].side !== me);
    const yours = s.deaths.filter((d) => F[d.b].side === me);
    if (theirs.length) {
      parts.push({ text: `${listNames(theirs.map((d) => uname(L, d.b)))} ${theirs.length > 1 ? "fall" : "falls"}` });
      const next = script.segs[k + 1];
      if (next?.type === "end" && o.plot_taken) parts.push({ text: ": " }, { text: "taken", cls: "good b" });
      else if (s.shieldDowns.some((x) => x.side !== me)) parts.push({ text: ": " }, { text: "shield down", cls: "b" });
      else if (s.screenGone) parts.push({ text: ": " }, { text: "no screen", cls: "b" });
    }
    if (yours.length) {
      if (parts.length) parts.push({ text: " · " });
      parts.push({ text: `your ${listNames(yours.map((d) => uname(L, d.b)))}`, cls: "you" }, { text: ` ${yours.length > 1 ? "fall" : "falls"}` });
    }
    for (const w of s.wds) {
      if (parts.length) parts.push({ text: " · " });
      parts.push({ text: `${uname(L, w.ev.unit)} ${w.ev.ok ? "pulls out" : "stays"} ` }, { text: `(${w.ev.roll}${w.ev.ok ? "<" : "≥"}${w.ev.chance})`, cls: "lilac" });
    }
    if (parts.length) out.push({ seg: k, label, parts });
  });
  return out;
}

/** Damage dealt by each fighter in the battle's hits (centi-HP, as rolled). */
export function dealtBy(log) {
  const dealt = log.fighters.map(() => 0);
  for (const e of log.events) if (e.kind === "Hit") dealt[e.h] += e.dmg;
  return dealt;
}

/** The facts that decided the battle, as `[{text, ...}]` parts lists (at most three). */
export function decidingFacts(log, final) {
  const F = log.fighters;
  const me = viewerSide(log);
  const hits = log.events.filter((e) => e.kind === "Hit");
  const dealt = dealtBy(log);
  const mine = F.filter((f) => f.side === me && committed(f));
  const theirs = F.filter((f) => f.side !== me && committed(f));
  const facts = [];
  const tot = mine.reduce((a, f) => a + dealt[f.b], 0) || 1;
  const top = mine.slice().sort((a, b) => dealt[b.b] - dealt[a.b])[0];
  if (top && dealt[top.b] > 0) {
    const dvs = log.pairs.filter((p) => p.h === top.b).map((p) => p.dv_x100);
    const range = dvs.length ? (Math.min(...dvs) === Math.max(...dvs) ? x100(dvs[0]) : `${x100(Math.min(...dvs))}–${x100(Math.max(...dvs))}`) : null;
    facts.push([
      { text: `Your ${uname(log, top.b)} did ` },
      { text: `${Math.round((100 * dealt[top.b]) / tot)}%`, cls: "b" },
      { text: " of your damage" },
      ...(range ? [{ text: " (" }, { text: `A ${x100(top.a.total_x100)}`, cls: "amber" }, { text: " vs " }, { text: `Dv ${range}`, cls: "frost" }, { text: ")" }] : []),
      { text: "." },
    ]);
  }
  const sd = segmentsShieldDown(log, 1 - me);
  if (sd !== null) {
    const reach = [...new Set(hits.filter((h) => F[h.h].side === me && isBack(F[h.t]) && h.rule !== "FrontRankDown").map((h) => h.h))];
    facts.push([
      { text: "Their front held " },
      { text: String(sd), cls: "b" },
      { text: ` rounds${reach.length ? `: only ${groupNames(log, reach)} reached their back rank` : ""}.` },
    ]);
  }
  const backUntouched = mine.filter((f) => isBack(f) && final.hp[f.b] === f.hp_start && !final.dead[f.b]);
  if (backUntouched.length && !segmentsShieldDownHappened(log, me)) {
    facts.push([{ text: `Your front never broke: your ${listNames(backUntouched.map((f) => uname(log, f.b)))} took no damage.` }]);
  }
  const mults = theirs.map((f) => f.dv.total_x100 / Math.max(1, Math.floor((f.dv.base_x100 * f.dv.role_pct) / 100)));
  if (mults.length && Math.max(...mults) > 1.05) {
    const lo = Math.min(...mults).toFixed(1);
    const hi = Math.max(...mults).toFixed(1);
    facts.push([{ text: "The place and their bonuses: their Dv " }, { text: lo === hi ? `×${lo}` : `×${lo}–${hi}`, cls: "frost" }, { text: "." }]);
  }
  // Luck, only when the result went against a firm forecast.
  const fc = log.forecast;
  const won = log.outcome.result === "AttackerWon";
  if (fc && ((won && fc.win_permille <= 300) || (!won && fc.win_permille >= 700))) {
    const r = [me, 1 - me].map((s) => {
      const hs = hits.filter((h) => F[h.h].side === s);
      return hs.reduce((a, h) => a + h.r, 0) / (hs.length || 1) / 100;
    });
    facts.unshift([{ text: "Luck", cls: "lilac" }, { text: `: your hits rolled ×${r[0].toFixed(2)} on average, theirs ×${r[1].toFixed(2)}.` }]);
  }
  return facts.slice(0, 3);
}

/** The round side `side`'s shield went down in (null: it held). */
function segmentsShieldDown(log, side) {
  let n = 0;
  for (const e of log.events) {
    if (e.kind === "Phase" && e.phase === "Round") n = e.n;
    if (e.kind === "ShieldDown" && e.side === side) return n;
  }
  return null;
}

function segmentsShieldDownHappened(log, side) {
  return segmentsShieldDown(log, side) !== null;
}

// ---------------------------------------------------------------------------------------------
// The plan (an EngagePreview)

/** A plan's force composition key (type, level and promotions), so plans that pick the same
 *  kinds of units merge in the What-if list. */
export function compositionKey(fighters, units) {
  return units
    .map((id) => {
      const f = fighters.find((x) => x.unit === id);
      return f ? `${tyName("UnitType", f.unit_type)}:${f.level}:${(f.promotions ?? []).map((p) => tyName("PromotionType", p)).sort().join("+")}` : `?${id}`;
    })
    .sort()
    .join(",");
}

/** The stance's possessive ("Balanced's", "Ranged Line's", "Preserve Veterans'"). */
export function possessive(name) {
  return name.endsWith("s") ? `${name}'` : `${name}'s`;
}

/** The What-if lines: this plan first, then each alternative of a different composition; an
 *  alternative of the same composition as an earlier line is folded into it (`also`, the stances
 *  whose pick it is). `alts`: `[{kind, stance, force: [unit ids], win_permille, losses_if_win_x10}]`. */
export function whatIfLines(preview, alts, stanceName) {
  const F = preview.fighters;
  const force = preview.force.map((b) => F[b].unit);
  const lines = [{ kind: "Current", label: "This plan", force, stance: preview.stance, win_permille: preview.forecast?.win_permille ?? 0, also: [], key: compositionKey(F, force) }];
  const name = (id) => {
    const f = F.find((x) => x.unit === id);
    return f ? uname(preview, f.b) : "?";
  };
  for (const a of alts ?? []) {
    const key = compositionKey(F, a.force);
    const dup = lines.find((l) => l.key === key);
    const st = stanceName(a.stance);
    if (dup) {
      if (a.kind !== "Swap") dup.also.push(st);
      continue;
    }
    let label;
    if (a.kind === "Swap") {
      const ins = a.force.filter((x) => !force.includes(x));
      const outs = force.filter((x) => !a.force.includes(x));
      label = `${ins.map(name).join(", ")} in, ${outs.map(name).join(", ")} out`;
    } else if (a.kind === "StancePick") label = `${possessive(st)} own pick`;
    else label = `${possessive(st)} pick`;
    lines.push({ ...a, label, also: [], key });
  }
  return lines;
}

/** The stakes of a plan: who dies if you win, if you lose, and who stays out either way. */
export function stakes(preview) {
  const F = preview.fighters;
  const me = viewerSide(preview);
  const force = preview.force.map((b) => F[b]);
  const reserve = (preview.reserve ?? []).map((b) => F[b]);
  const pick = preview.defender_pick ?? { kind: "Known", units: F.filter((f) => f.side !== me).map((f) => f.b) };
  const defs = (pick.kind === "Known" ? pick.units : [...(pick.sure ?? []), ...(pick.maybe ?? [])]).map((b) => F[b]);
  const allDefs = F.filter((f) => f.side !== me);
  const win = Math.round((preview.forecast?.win_permille ?? 0) / 10);
  const draw = Math.round((preview.forecast?.draw_permille ?? 0) / 10);
  return {
    win,
    draw,
    lose: Math.max(0, 100 - win - draw),
    force,
    defenders: defs,
    defendersLeft: allDefs.length - defs.length,
    withdrawers: force.filter((f) => f.withdraw_pct > 0),
    reserve,
  };
}
