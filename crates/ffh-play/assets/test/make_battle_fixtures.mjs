// Makes the battle screens' dev fixture, app/battle_fixture.js (docs/HUMAN.md, "The battle
// screens"): the formation-battle prototype's engine (fixtures/proto/engine.js and scenarios.js,
// copied from the owner's prototype of phase 6 revision 4) plays its sample armies, and its set-up,
// forecasts and battle logs are converted to the protocol's shapes (app/battle_log.js lists them).
// The numbers are the prototype's, not the engine's. Type-valued fields hold type names where the
// protocol sends indices (the screens take either).
//
//   node crates/ffh-play/assets/test/make_battle_fixtures.mjs
//
// Each fixture: `preview` (the plan's first preview: the rules' auto formation for both sides and
// its forecast), `forecasts` (by formationKey: the auto formation's, and for `field` every
// formation one click of the plan makes from it), `logs` ([{formation, log}]: the auto
// formation's battle, and for `field` one of a moved unit's, for the screenshots).
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { placeUnit, benchUnit, formationOf, formationKey, slotCap } from "../app/battle_log.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const En = require("./fixtures/proto/engine.js");
const Sc = require("./fixtures/proto/scenarios.js");
const OUT = path.join(here, "..", "app", "battle_fixture.js");

/** Forecast runs a formation (the prototype's page runs 1,000; fewer keep the file small). */
const RUNS = 200;
const BASE = 40;
/** The rule values the screens word (the protocol's `params`): the prototype's (its magic bolt,
 *  its spells' splash of half a hit, its LARGE_HP, two speed a first strike). */
const PARAMS = { bolt_pct: En.SPELLS.bolt.power, splash_pct: 50, large_hp_pct: 150, first_strike_speed: 2 };

const TYPE = {
  Champion: "UNIT_CHAMPION",
  Axeman: "UNIT_AXEMAN",
  Swordsman: "UNIT_SWORDSMAN",
  Hunter: "UNIT_HUNTER",
  Ranger: "UNIT_RANGER",
  "Hill Giant": "UNIT_HILL_GIANT",
  Archer: "UNIT_ARCHER",
  Longbowman: "UNIT_LONGBOWMAN",
  Horseman: "UNIT_HORSEMAN",
  "Horse Archer": "UNIT_HORSE_ARCHER",
  "War Elephant": "UNIT_WAR_ELEPHANT",
  Assassin: "UNIT_ASSASSIN",
  Adept: "UNIT_ADEPT",
  Mage: "UNIT_MAGE",
  "Priest of the Order": "UNIT_PRIEST_OF_THE_ORDER",
  "Disciple of Leaves": "UNIT_DISCIPLE_FELLOWSHIP_OF_LEAVES",
  Catapult: "UNIT_CATAPULT",
};
const KIND = { strike: "Strike", charge: "Charge", stab: "Stab", shoot: "Shoot", spell: "Spell", bombard: "Bombard", heal: "Heal", none: "None" };
const SPELL = { maelstrom: "SPELL_MAELSTROM", snowfall: "SPELL_SNOWFALL" };
const TERRAIN = { grassland: "TERRAIN_GRASS", plains: "TERRAIN_PLAINS", desert: "TERRAIN_DESERT", tundra: "TERRAIN_TUNDRA", marsh: "TERRAIN_MARSH" };

const ATT = { player: 0, shown_player: 0, civ: "CIVILIZATION_GRIGORI", label: "1st Army", army: 3, garrison_of: null };
const CITY = { id: 7, name: "Strages", owner: 1, size: 6, defense_pct: 50 };

const SCENARIOS = {
  field: {
    sc: Sc.SCENARIOS.open,
    text: "The 1st Army (Grigori) meets a Calabim host on open grassland: 12 units for 10 slots against 11.",
    def: { player: 1, shown_player: 1, civ: "CIVILIZATION_CALABIM", label: "Calabim host", army: 9, garrison_of: null },
    variants: true,
  },
  forest: {
    sc: Sc.SCENARIOS.forest,
    text: "The 1st Army attacks a Calabim warband holding a forest: arrows deal half, no flanking.",
    def: { player: 1, shown_player: 1, civ: "CIVILIZATION_CALABIM", label: "Calabim warband", army: 11, garrison_of: null },
  },
  hills: {
    sc: { ...Sc.SCENARIOS.forest, ground: { terrain: "plains", feature: "none", hills: true, city: false, cityDef: 50 } },
    text: "The 1st Army attacks a Calabim warband on a hill: the attackers deal 25% less, the defenders' arrows 25% more.",
    def: { player: 1, shown_player: 1, civ: "CIVILIZATION_CALABIM", label: "Calabim warband", army: 11, garrison_of: null },
  },
  raid: {
    sc: {
      ground: { terrain: "plains", feature: "none", hills: false, city: true, cityDef: 20 },
      att: Sc.SCENARIOS.city.att,
      def: [Sc.UNITS.archer(), Sc.UNITS.axeman(), Sc.UNITS.disciple()],
    },
    text: "The 1st Army storms Strages, held by three units behind walls at 20%: the city should fall.",
    def: { player: 1, shown_player: 1, civ: "CIVILIZATION_CALABIM", label: "Strages", army: null, garrison_of: CITY.id },
    city: { ...CITY, defense_pct: 20, size: 3 },
  },
  city: {
    sc: Sc.SCENARIOS.city,
    text: "The 1st Army assaults Strages, a Calabim city behind walls at 50%: the walls cost two slots; their arrows strike the first attacker as it comes in.",
    def: { player: 1, shown_player: 1, civ: "CIVILIZATION_CALABIM", label: "Strages", army: null, garrison_of: CITY.id },
    defStrike: { pct: 15 },
  },
};

// ---------------------------------------------------------------------------------------------
// The set-up

function groundOf(g, city = CITY) {
  const fr = En.frontage(g);
  const terrain = TERRAIN[g.terrain];
  const forest = g.feature === "forest" || g.feature === "jungle";
  const change = (side) => {
    const out = [];
    if (g.hills) out.push({ src: "Hills", delta: -2 });
    if (forest) out.push({ src: "Feature", feature: `FEATURE_${g.feature.toUpperCase()}`, delta: -2 });
    else if (!g.hills) out.push({ src: "Terrain", terrain, delta: En.TERRAINS[g.terrain].slots });
    if (side === 0 && g.city) out.push({ src: "CityDefense", pct: g.cityDef, next_at: (Math.floor(g.cityDef / 25) + 1) * 25, delta: -Math.floor(g.cityDef / 25) });
    return out;
  };
  const rules = [];
  if (g.city) rules.push({ kind: "CityShots", pct: 50 });
  if (forest) rules.push({ kind: "ForestRanged", pct: 50 });
  if (g.hills) rules.push({ kind: "HillsAttacker", pct: 75 }, { kind: "HillsDefenderRanged", pct: 125 });
  // The cavalry's charge in the field (the prototype's chargePct): where flanking works.
  if (fr.ground.flanking && En.CHARGE() !== 0) rules.push({ kind: "ChargeFirstRound", pct: 100 + En.CHARGE() });
  const plot = { x: 31, y: 18, terrain, feature: forest ? `FEATURE_${g.feature.toUpperCase()}` : null, hills: !!g.hills, river_crossing: false, city: g.city ? city : null };
  return {
    fr,
    plot,
    from: { x: 30, y: 18, terrain: "TERRAIN_GRASS", feature: null, hills: false, river_crossing: false, city: null },
    ground: { city: fr.ground.city, forest: fr.ground.forest, hills: fr.ground.hills, flanking: fr.ground.flanking, rules },
    slots: [fr.slots.att, fr.slots.def].map((total, side) => ({ base: 8, changes: change(side), total, river_only: 0 })),
    lanes: [fr.lanes.att, fr.lanes.def],
  };
}

const strA = (u) => En.strA(u);
const dvOf = (u) => (u.def * En.ROLES[u.role].dv * (100 + 10 * (u.level - 1))) / 10000;

/** A prototype unit as a protocol Fighter (no cell yet). */
function fighterOf(u, side, b, unit, owner) {
  const R = En.ROLES[u.role];
  const promotions = [];
  if (u.role === "screen") promotions.push("PROMOTION_FORMATION");
  if (u.level >= 2) promotions.push("PROMOTION_COMBAT1");
  if (u.level >= 3) promotions.push("PROMOTION_COMBAT2");
  if (u.note === "Air II") promotions.push("PROMOTION_AIR2");
  const actions = [0, 1, 2].map((r) => {
    const a = En.actionsAt(u, r);
    return { kind: KIND[a.kind], n: a.n };
  });
  const max = En.maxHp(u);
  return {
    b,
    unit,
    unit_type: TYPE[u.name],
    name: u.name,
    owner,
    side,
    level: u.level,
    hero: !!u.hero,
    promotions,
    role: `COMBATROLE_${u.role.toUpperCase()}`,
    large: !!u.large,
    lane: null,
    row: null,
    max_hp: max,
    hp_start: max * 100,
    a_x100: Math.round(strA(u) * 100),
    dv_x100: Math.round(dvOf(u) * 100),
    speed: En.speedOf(u),
    actions,
    bonus_action: (u.tier ?? 0) >= 4 || !!u.hero,
    spell: u.role === "arcane" && u.spell ? SPELL[u.spell] : null,
    dodge: R.dodge ?? 0,
    crit_bonus: R.critBonus ?? 0,
    anti_flank: !!R.guard,
    withdraw_pct: u.wd ?? 0,
    combat_limit: u.lim ?? 100,
    // collateral damage (the prototype's siege); a caster's spell splashes as the spell (`spell`)
    splash: u.role === "siege",
    first_strikes: u.fs ?? 0,
  };
}

/** The fighters of a scenario: the attacker's pool, then the defenders, in the pools' order. */
function fightersOf(def, sc) {
  const out = [];
  sc.att.forEach((u, i) => out.push(fighterOf(u, 0, out.length, 4101 + i, ATT.player)));
  sc.def.forEach((u, i) => out.push(fighterOf(u, 1, out.length, 5201 + i, def.player)));
  return out;
}

/** The fighters with side `side` in formation `place` (Map b -> {lane, row}). */
function withPlace(F, side, place) {
  return F.map((f) => (f.side !== side ? f : { ...f, lane: place.get(f.b)?.lane ?? null, row: place.get(f.b)?.row ?? null }));
}

/** The prototype's auto formation of side `side` as a formation. */
function autoPlace(F, sc, side, g) {
  const pool = side === 0 ? sc.att : sc.def;
  const off = side === 0 ? 0 : sc.att.length;
  const placed = Sc.autoArrange(pool, g.lanes[side], g.fr.slots[side === 0 ? "att" : "def"]);
  return new Map(placed.map((p) => [off + p.i, { lane: p.lane, row: p.row }]));
}

// ---------------------------------------------------------------------------------------------
// Battles

/** The prototype's battle over `F` (fighters in their cells): `{r, B}` (its run, and each of its
 *  fighters' `b`). `damage`: start damage percents by `b` (an opening's strikes). */
function battle(F, sc, g, seed, damage = {}) {
  const units = F.map((f) => (f.side === 0 ? sc.att[f.b] : sc.def[f.b - sc.att.length]));
  const line = F.filter((f) => f.lane !== null);
  const placed = line.map((f) => ({ unit: { ...units[f.b], damage: damage[f.b] ?? 0 }, side: f.side === 0 ? "att" : "def", lane: f.lane, row: f.row }));
  const reserves = F.filter((f) => f.side === 1 && f.lane === null).length;
  const opts = { lanes: g.fr.lanes, base: BASE, plotDef: 0, city: g.fr.city, ground: g.fr.ground, dice: true, miss: 10, crit: 10, knock: true, rounds: 3, reserves };
  return { r: En.run(placed, opts, seed), B: line.map((f) => f.b) };
}

function forecastOf(F, sc, g, damage) {
  const dead = new Map();
  const fled = new Map();
  const lost = [0, 0];
  const res = { taken: 0, cleared: 0, held: 0, repulsed: 0 };
  for (let s = 1; s <= RUNS; s++) {
    const { r, B } = battle(F, sc, g, 7919 * s + 13, damage);
    res[r.result]++;
    r.b.fs.forEach((u, i) => {
      const b = B[i];
      if (!u.alive) {
        dead.set(b, (dead.get(b) ?? 0) + 1);
        lost[F[b].side]++;
      }
      if (u.gone) fled.set(b, (fled.get(b) ?? 0) + 1);
    });
  }
  const per = F.filter((f) => f.lane !== null).map((f) => {
    const p = { b: f.b, fall_pct: Math.round((100 * (dead.get(f.b) ?? 0)) / RUNS) };
    const e = Math.round((100 * (fled.get(f.b) ?? 0)) / RUNS);
    if (e) p.escape_pct = e;
    return p;
  });
  return {
    runs: RUNS,
    lost_x100: lost.map((n) => Math.round((100 * n) / RUNS)),
    line_wiped_permille: Math.round((1000 * (res.taken + res.cleared)) / RUNS),
    plot_falls_permille: Math.round((1000 * res.taken) / RUNS),
    attackers_wiped_permille: Math.round((1000 * res.repulsed) / RUNS),
    per,
  };
}

/** The battle log of fighters `F` (seed `seed`) in the protocol's shape. */
function logOf(key, def, F, sc, g, seed, { id, defStrike }) {
  const damage = {};
  const opening = [];
  if (defStrike) {
    // The walls' archers strike the first attacker in the front row as it comes in (the
    // prototype has no opening: the strike's damage is its start damage, the event says so).
    const striker = F.find((f) => f.side === 1 && f.lane !== null && f.role === "COMBATROLE_ARCHER");
    const target = F.filter((f) => f.side === 0 && f.row === 0).sort((a, b) => a.lane - b.lane)[0];
    damage[target.b] = defStrike.pct;
    const hp = Math.round((target.max_hp * (100 - defStrike.pct)) / 100);
    opening.push({ round: 0, pass: 0, kind: "DefStrike", actor: striker.b, target: target.b, hits: [{ target: target.b, dmg: (target.max_hp - hp) * 100 }] });
  }
  const { r, B } = battle(F, sc, g, seed, damage);
  const events = [...opening];
  for (const ev of r.log) {
    const e = { round: ev.round, pass: ev.pass, actor: B[ev.actor], kind: KIND[ev.kind] };
    if (ev.kind === "spell") e.spell = F[B[ev.actor]].spell;
    e.target = ev.hits.length ? B[ev.hits[0].target] : ev.heal ? B[ev.heal.target] : null;
    e.hits = ev.hits.map((h) => {
      const x = { target: B[h.target], dmg: h.dmg * 100 };
      if (h.crit) x.crit = true;
      if (h.miss) x.miss = true;
      if (h.killed) x.killed = true;
      if (h.withdrew) x.withdrew = true;
      return x;
    });
    if (ev.heal) e.heal = { target: B[ev.heal.target], amount: ev.heal.amt * 100 };
    if (ev.knocked) e.knocked = { unit: B[ev.knocked.unit], from: ev.knocked.from, to: ev.knocked.to };
    if (ev.note) e.note = ev.note === "no one hurt" ? "NoOneHurt" : "NoTarget";
    events.push(e);
  }
  const sides = [0, 1].map(() => ({ fallen: 0, escaped: 0, standing: 0 }));
  const standing = [];
  r.b.fs.forEach((u, i) => {
    const f = F[B[i]];
    const s = sides[f.side];
    if (!u.alive) s.fallen++;
    else if (u.gone) s.escaped++;
    else {
      s.standing++;
      standing.push(f.b);
    }
  });
  const taken = r.result === "taken";
  const reserve = F.filter((f) => f.side === 1 && f.lane === null).length;
  const rounds = Math.max(0, ...events.map((e) => e.round));
  // XP: a point for a survivor that hit, two more a kill (the prototype has none: the fixture's own).
  const xp = new Map();
  for (const e of events) {
    if (!e.round || !e.hits?.some((h) => h.dmg > 0) || !standing.includes(e.actor)) continue;
    const kills = e.hits.filter((h) => h.killed).length;
    xp.set(e.actor, Math.min(6, (xp.get(e.actor) ?? 1) + 2 * kills));
  }
  const after = [...xp].sort((a, b) => a[0] - b[0]).map(([unit, amount]) => ({ kind: "Xp", unit, amount }));
  if (taken) after.push({ kind: "MoveIn", units: standing.filter((b) => F[b].side === 0), city: g.plot.city ? g.plot.city.id : null });
  const outcome = { plot_taken: taken, city_captured: taken && g.plot.city ? g.plot.city.id : null, rounds, defenders_left: sides[1].standing + reserve, sides };
  const lostTypes = (side) => {
    const m = new Map();
    r.b.fs.forEach((u, i) => {
      const f = F[B[i]];
      if (f.side === side && !u.alive) m.set(f.unit_type, (m.get(f.unit_type) ?? 0) + 1);
    });
    return [...m];
  };
  const summary = {
    id,
    turn: 87,
    x: g.plot.x,
    y: g.plot.y,
    from: { x: g.from.x, y: g.from.y },
    place: g.plot.city?.name ?? null,
    city: g.plot.city?.id ?? null,
    viewer_side: 0,
    plot_taken: taken,
    city_captured: outcome.city_captured,
    rounds,
    defenders_left: outcome.defenders_left,
    has_log: true,
    sides: [ATT, def].map((s, side) => ({
      player: s.player,
      civ: s.civ,
      label: s.label,
      slots: g.slots[side].total,
      lanes: g.lanes[side],
      committed: F.filter((f) => f.side === side && f.lane !== null).length,
      lost: sides[side].fallen,
      lost_types: lostTypes(side),
      escaped: sides[side].escaped,
      sat_out: 0,
      fled: side === 1 ? sides[1].escaped : 0,
      standing: sides[side].standing,
    })),
  };
  return {
    v: 1,
    id,
    turn: 87,
    viewer_side: 0,
    plot: g.plot,
    from: g.from,
    ground: g.ground,
    params: PARAMS,
    slots: g.slots,
    lanes: g.lanes,
    sides: [ATT, def],
    fighters: F,
    events,
    outcome,
    after,
    summary,
    _key: key,
  };
}

// ---------------------------------------------------------------------------------------------
// The fixtures

/** Every formation one click of the plan makes from `place` (a unit to a cell, a unit out). */
function oneClick(F, place, g) {
  const out = new Map();
  const lim = { lanes: g.lanes[0], cap: slotCap(g.slots[0]) };
  for (const f of F.filter((x) => x.side === 0)) {
    for (let row = 0; row < 3; row++) {
      for (let lane = 0; lane < g.lanes[0]; lane++) {
        const p = placeUnit(place, F, lim, f.b, lane, row);
        if (p.error) continue;
        out.set(formationKey(formationOf(p.place, F)), p.place);
      }
    }
    if (place.has(f.b)) {
      const p = benchUnit(place, f.b);
      out.set(formationKey(formationOf(p, F)), p);
    }
  }
  return out;
}

function make(name, s, n) {
  const sc = s.sc;
  const g = groundOf(sc.ground, s.city);
  let F = fightersOf(s.def, sc);
  F = withPlace(F, 1, autoPlace(F, sc, 1, g));
  const auto = autoPlace(F, sc, 0, g);
  const autoF = withPlace(F, 0, auto);
  const autoList = formationOf(auto, F);
  const damageOf = (Fx) => {
    if (!s.defStrike) return {};
    const t = Fx.filter((f) => f.side === 0 && f.row === 0).sort((a, b) => a.lane - b.lane)[0];
    return t ? { [t.b]: s.defStrike.pct } : {};
  };
  const forecasts = {};
  const autoKey = formationKey(autoList);
  forecasts[autoKey] = forecastOf(autoF, sc, g, damageOf(autoF));
  if (s.variants) {
    for (const [key, place] of oneClick(F, auto, g)) {
      if (forecasts[key]) continue;
      const Fx = withPlace(F, 0, place);
      // Without each unit's chance to fall (the totals only), to keep the file small.
      forecasts[key] = { ...forecastOf(Fx, sc, g, damageOf(Fx)), per: [] };
    }
  }
  const logs = [{ formation: autoList, log: logOf(autoKey, s.def, autoF, sc, g, 104729 * (n + 1) + 1, { id: 12 + 2 * n, defStrike: s.defStrike }) }];
  // A moved unit's battle: the first attacker in the middle row goes to the rear row of its lane.
  const mover = s.variants ? autoF.find((f) => f.side === 0 && f.row === 1 && !f.large) : null;
  if (mover) {
    const p = placeUnit(auto, F, { lanes: g.lanes[0], cap: slotCap(g.slots[0]) }, mover.b, mover.lane, 2);
    if (!p.error) {
      const list = formationOf(p.place, F);
      const key = formationKey(list);
      const Fx = withPlace(F, 0, p.place);
      forecasts[key] = forecastOf(Fx, sc, g, damageOf(Fx));
      logs.push({ formation: list, moved: { unit: mover.unit, lane: mover.lane, row: 2 }, log: logOf(key, s.def, Fx, sc, g, 104729 * (n + 1) + 7, { id: 13 + 2 * n, defStrike: s.defStrike }) });
    }
  }
  for (const l of logs) delete l.log._key;
  return {
    scenario: s.text,
    preview: {
      v: 1,
      viewer_side: 0,
      plot: g.plot,
      from: g.from,
      ground: g.ground,
      params: PARAMS,
      slots: g.slots,
      lanes: g.lanes,
      sides: [ATT, s.def],
      fighters: autoF,
      formation: autoList,
      auto_formation: autoList,
      forecast: forecasts[autoKey],
      declare_war: null,
      errors: [],
    },
    forecasts,
    logs,
  };
}

// ---------------------------------------------------------------------------------------------
// The file: JSON, an array element or an object's member a line where that keeps it readable.

function fmt(v, ind = "") {
  const next = `${ind} `;
  if (Array.isArray(v)) {
    if (!v.length || v.every((x) => x === null || typeof x !== "object")) return JSON.stringify(v);
    if (v.every((x) => x && typeof x === "object" && JSON.stringify(x).length < 1200)) return `[\n${v.map((x) => next + JSON.stringify(x)).join(",\n")}\n${ind}]`;
    return `[\n${v.map((x) => next + fmt(x, next)).join(",\n")}\n${ind}]`;
  }
  if (v && typeof v === "object") {
    const s = JSON.stringify(v);
    if (s.length < 160) return s;
    return `{\n${Object.entries(v)
      .map(([k, x]) => `${next}${JSON.stringify(k)}: ${fmt(x, next)}`)
      .join(",\n")}\n${ind}}`;
  }
  return JSON.stringify(v);
}

const fixtures = {};
Object.entries(SCENARIOS).forEach(([name, s], n) => {
  fixtures[name] = make(name, s, n);
});
const head = `// The battle screens' dev fixture (docs/HUMAN.md, "The battle screens"; \`?battle=field\` opens it, or
// forest, hills, city): previews, forecasts and battle logs of formation battles in the protocol's
// shapes (battle_log.js lists them). Made by the owner's formation-battle prototype (its engine and
// sample armies), not by the engine: the numbers are the prototype's. Type-valued fields hold type
// names where the protocol sends indices (the screens take either). Generated: do not edit;
// \`node crates/ffh-play/assets/test/make_battle_fixtures.mjs\` makes it again.
`;
fs.writeFileSync(OUT, `${head}export const FIXTURES = ${fmt(fixtures)};\n`);
const size = fs.statSync(OUT).size;
console.log(`make_battle_fixtures: ${OUT}: ${Object.keys(fixtures).join(", ")}; ${Object.values(fixtures).map((f) => Object.keys(f.forecasts).length).join("/")} forecasts; ${(size / 1024).toFixed(0)} KiB`);
