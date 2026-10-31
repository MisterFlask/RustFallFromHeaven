// Formation battle model (mock-up of an Ogre Battle-style engagement for the FfH port).
const ROWS = ['Front', 'Middle', 'Rear'];
const ROLES = {
  frontline: { label: 'Frontline', a: 100, dv: 100, hp: 120, speed: 4, rows: [['strike', 2], ['strike', 1], ['strike', 1]] },
  screen:    { label: 'Screen', a: 100, dv: 110, hp: 120, speed: 3, rows: [['strike', 2], ['strike', 1], ['strike', 1]], guard: true },
  flanker:   { label: 'Flanker', a: 110, dv: 80, hp: 100, speed: 6, rows: [['charge', 2], ['strike', 1], ['strike', 1]], dodge: 10 },
  assassin:  { label: 'Assassin', a: 120, dv: 60, hp: 60, speed: 7, rows: [['stab', 1], ['stab', 2], ['stab', 1]], dodge: 15, critBonus: 15 },
  archer:    { label: 'Archer', a: 100, dv: 60, hp: 80, speed: 4, rows: [['shoot', 1], ['shoot', 2], ['shoot', 2]] },
  siege:     { label: 'Siege', a: 100, dv: 60, hp: 80, speed: 1, rows: [['none', 0], ['bombard', 1], ['bombard', 2]] },
  arcane:    { label: 'Arcane', a: 100, dv: 50, hp: 70, speed: 3, rows: [['strike', 1], ['spell', 1], ['spell', 2]] },
  divine:    { label: 'Divine', a: 100, dv: 50, hp: 70, speed: 3, rows: [['strike', 1], ['heal', 1], ['heal', 2]] },
};
const LARGE_HP = 150; // a large unit has 150% of its role's HP
let HP_PCT = 100;     // every unit's HP, in % of its role's (the page's HP slider)
function setHpPct(p) { HP_PCT = p; }

// Reach (who an action can hit), per lane of the enemy grid:
//   front: the lane's frontmost unit (melee goes through whoever stands in front);
//   rear:  the lane's backmost unit (arrows, spells and stones fall on the back first);
//   flank: a cavalry charge goes round the line to the unit with the highest threat for its HP left, anywhere;
//          not in a city, forest or jungle, where it hits the front unit of a lane like a strike.
//   Whatever a flanker can reach, anti-flanking units (screens: Formation) in its reach come first.
//   any:   any unit (assassins slip through).
// Physical actions can miss (the miss chance, plus the kind's extra and the target role's dodge) and, when they land
// on a single target, crit (x1.5; a melee crit knocks the target back a row if the cells behind are free).
// Spells and heals always land.
const KINDS = {
  strike:  { verb: 'strikes', reach: 'front', phys: true, missAdd: 0, knock: true },
  charge:  { verb: 'charges', reach: 'flank', phys: true, missAdd: 0, knock: true },
  stab:    { verb: 'stabs', reach: 'any', phys: true, missAdd: 0 },
  shoot:   { verb: 'shoots', reach: 'rear', phys: true, missAdd: 10 },
  spell:   { verb: 'casts at', reach: 'rear', magic: true },
  bombard: { verb: 'bombards', reach: 'rear', phys: true, missAdd: 15, area: true },
  heal:    { verb: 'heals', reach: 'ally' },
  none:    { verb: 'waits', reach: 'none' },
};
// A caster's spell: the strongest damage spell it knows, else the magic bolt. Power is % of a normal hit on the
// target; a 'plus' spell also hits the target's neighbours (beside, in front, behind) for half of that.
const SPELLS = {
  bolt:      { name: 'Magic bolt', power: 60, shape: 'one' },
  maelstrom: { name: 'Maelstrom', power: 100, shape: 'plus' },
  snowfall:  { name: 'Snowfall', power: 130, shape: 'plus' },
};
const CRIT_X = 1.5;

function rng32(seed) {
  let a = seed >>> 0;
  return (n) => {
    a = (a + 0x6D2B79F5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (((t ^ (t >>> 14)) >>> 0) % n);
  };
}

// The ground: slots as the current build counts them (8, open flat land +2, forest or jungle -2, hills -2, a city's
// defense costing the attacker 1 per 25%, at least 3), lanes from the slots, and the defender's bonus.
const TERRAINS = {
  grassland: { name: 'Grassland', slots: 2 }, plains: { name: 'Plains', slots: 2 }, desert: { name: 'Desert', slots: 2 },
  tundra: { name: 'Tundra', slots: 2 }, marsh: { name: 'Marsh', slots: 0 },
};
const FEATURES = { none: { name: 'No feature', slots: null, def: 0 }, forest: { name: 'Forest', slots: -2, def: 50 }, jungle: { name: 'Jungle', slots: -2, def: 50 } };
function lanesFor(slots) { return Math.max(2, Math.ceil((slots + 2) / 3)); }
function frontage(g) {
  const F = FEATURES[g.feature];
  const change = (g.hills ? -2 : 0) + (F.slots !== null ? F.slots : (g.hills ? 0 : TERRAINS[g.terrain].slots));
  const def = Math.max(3, 8 + change);
  const att = Math.max(3, 8 + change - (g.city ? Math.floor(g.cityDef / 25) : 0));
  const forest = g.feature === 'forest' || g.feature === 'jungle';
  return { slots: { att, def }, lanes: { att: lanesFor(att), def: lanesFor(def) }, plotDef: 0, city: !!g.city,
    ground: { city: !!g.city, forest, hills: !!g.hills, flanking: !g.city && !forest } };
}

// Actions a unit has in a row: the role's, plus one in its best row for tier 4 units and heroes.
function actionsAt(u, row) {
  const R = ROLES[u.role];
  let [kind, n] = R.rows[row];
  if (kind !== 'none' && (u.tier >= 4 || u.hero)) {
    let best = 0;
    R.rows.forEach((r, i) => { if (r[1] > R.rows[best][1]) best = i; });
    if (best === row) n += 1;
  }
  return { kind, n };
}
function speedOf(u) { return ROLES[u.role].speed + 2 * (u.fs || 0); }
function maxHp(u) { return Math.max(1, Math.round(ROLES[u.role].hp * (u.large ? LARGE_HP : 100) / 100 * HP_PCT / 100)); }
function sizeOf(u) { return u.large ? 2 : 1; }
function spellOf(u) { return SPELLS[u.spell || 'bolt']; }
function lanesOf(u) { return u.large ? [u.lane, u.lane + 1] : [u.lane]; }

// opts: { lanes: {att, def}, base, plotDef, city, dice, miss, crit, knock }
function setup(placed, opts) {
  const fs = placed.map((p, i) => ({
    ...p.unit, side: p.side, lane: p.lane, row: p.row, idx: i,
    maxhp: maxHp(p.unit), hp: Math.round(maxHp(p.unit) * (100 - (p.unit.damage || 0)) / 100),
    alive: true, gone: false,
  }));
  return { fs, opts };
}

function strA(u) { const R = ROLES[u.role]; return u.str * R.a / 100 * (100 + 10 * (u.level - 1)) / 100; }
function strDv(b, t, magic) {
  const R = ROLES[t.role];
  let pct = 100 + 10 * (t.level - 1);
  if (t.side === 'def' && !magic) pct += b.opts.plotDef;
  return t.def * R.dv / 100 * pct / 100;
}
// The ground's damage rules, multiplied together: in a city the attackers' arrows deal half; in a forest or jungle
// arrows and stones deal half on both sides; on hills the attackers deal 25% less and the defenders' arrows and
// stones 25% more.
function groundPct(b, h, kind) {
  const gr = b.opts.ground;
  if (!gr) return 100;
  const ranged = kind === 'shoot' || kind === 'bombard';
  let pct = 100;
  if (gr.city && h.side === 'att' && kind === 'shoot') pct = pct * 50 / 100;
  if (gr.forest && ranged) pct = pct * 50 / 100;
  if (gr.hills && h.side === 'att') pct = pct * 75 / 100;
  if (gr.hills && h.side === 'def' && ranged) pct = pct * 125 / 100;
  return pct;
}
// The cavalry's charge in the field: its first-round charges hit harder where flanking works.
let CHARGE_BONUS = 100;
function setChargeBonus(p) { CHARGE_BONUS = p; }
function chargePct(b, kind) { return kind === 'charge' && b.round === 1 && b.opts.ground && b.opts.ground.flanking ? 100 + CHARGE_BONUS : 100; }
function meanHit(b, h, t, kind) {
  const A = strA(h), Dv = strDv(b, t, KINDS[kind].magic);
  const power = kind === 'spell' ? spellOf(h).power : 100;
  return b.opts.base * A / (A + Dv) * power / 100 * groundPct(b, h, kind) / 100 * chargePct(b, kind) / 100;
}
// How dangerous a unit is: its attack times the actions it has from where it stands.
function threatOf(t) { const a = actionsAt(t, t.row); return a.kind === 'none' ? 0 : strA(t) * a.n; }
function antiFlank(t) { return !!(ROLES[t.role].guard || t.antiFlank); }
function floorHp(h, t) { return h.lim && h.lim < 100 ? Math.ceil(t.maxhp * (100 - h.lim) / 100) : 0; }
const onField = (u) => u.alive && !u.gone;
const foeOf = (s) => (s === 'att' ? 'def' : 'att');

function laneUnits(b, side, lane) { return b.fs.filter((u) => onField(u) && u.side === side && lanesOf(u).includes(lane)).sort((x, y) => x.row - y.row); }
function guardOf(b, side, lane) {
  return b.fs.filter((u) => onField(u) && u.side === side && ROLES[u.role].guard && Math.abs(u.lane - lane) <= 1)
    .sort((x, y) => Math.abs(x.lane - lane) - Math.abs(y.lane - lane) || x.row - y.row || x.lane - y.lane)[0];
}
function reachable(b, h, kind) {
  const reach = KINDS[kind].reach, foe = foeOf(h.side);
  let list = [];
  const flanks = reach === 'flank' && b.opts.ground && b.opts.ground.flanking;
  if (reach === 'any' || flanks) list = b.fs.filter((t) => onField(t) && t.side === foe);
  else for (let l = 0; l < b.opts.lanes[foe]; l++) {
    const us = laneUnits(b, foe, l);
    if (!us.length) continue;
    if (reach === 'rear') list.push(us[us.length - 1]);
    else list.push(us[0]);
  }
  return [...new Set(list)].filter((t) => t.hp > floorHp(h, t));
}

// Every unit fights the same way: a kill when one is on offer (the most dangerous such unit), otherwise the unit
// this hit hurts most for its HP left. Ties go by the battlefield, never by list order.
const place = (x, y) => x.lane - y.lane || x.row - y.row;
function choose(b, h, kind, list) {
  const share = (t) => meanHit(b, h, t, kind) / Math.max(1, t.hp);
  const tie = (x, y) => share(y) - share(x) || strA(y) - strA(x) || place(x, y);
  const kills = list.filter((t) => meanHit(b, h, t, kind) >= t.hp);
  if (kills.length) return kills.slice().sort((x, y) => strA(y) - strA(x) || tie(x, y))[0];
  return list.slice().sort(tie)[0];
}
function isPlus(h, kind) { return kind === 'bombard' || (kind === 'spell' && spellOf(h).shape === 'plus'); }
function adjacent(x, y) {
  const lx = lanesOf(x), ly = lanesOf(y);
  if (x.row === y.row) return lx.some((a) => ly.some((c) => Math.abs(a - c) === 1)) && !lx.some((a) => ly.includes(a));
  return Math.abs(x.row - y.row) === 1 && lx.some((a) => ly.includes(a));
}
function areaAround(b, c) { return b.fs.filter((t) => onField(t) && t.side === c.side && t !== c && adjacent(t, c)).sort(place); }
function missChance(b, h, t, kind) { return Math.min(95, b.opts.miss + KINDS[kind].missAdd + (ROLES[t.role].dodge || 0)); }
function critChance(b, h) { return Math.min(95, b.opts.crit + (ROLES[h.role].critBonus || 0)); }

// One hit lands at once: damage, then death or a withdrawal from the killing blow.
function applyHit(b, h, t, kind, factor, rnd, ev, crit) {
  const r = 80 + rnd(41);
  let dmg = Math.max(1, Math.round(meanHit(b, h, t, kind) * factor * r / 100));
  dmg = Math.min(dmg, Math.max(0, t.hp - floorHp(h, t)));
  const hit = { target: t.idx, dmg, r };
  if (crit) hit.crit = true;
  if (dmg >= t.hp) {
    const canFlee = t.wd > 0 && !(t.side === 'def' && b.opts.city);
    if (canFlee && rnd(100) < t.wd) { t.gone = true; hit.withdrew = true; hit.dmg = 0; }
    else { hit.dmg = t.hp; t.hp = 0; t.alive = false; hit.killed = true; }
  } else t.hp -= dmg;
  ev.hits.push(hit);
}
function knockBack(b, t, ev) {
  if (!onField(t) || t.row === 2) return;
  const behind = lanesOf(t).map((l) => ({ lane: l, row: t.row + 1 }));
  if (b.fs.some((u) => onField(u) && u.side === t.side && u.row === t.row + 1 && lanesOf(u).some((l) => behind.some((c) => c.lane === l)))) return;
  ev.knocked = { unit: t.idx, from: t.row, to: t.row + 1 };
  t.row += 1;
}

function act(b, h, kind, rnd, log) {
  const ev = { round: b.round, pass: b.pass, actor: h.idx, kind, hits: [], heal: null, note: null };
  if (kind === 'spell') ev.spell = spellOf(h).name;
  if (kind === 'heal') {
    const hurt = b.fs.filter((u) => onField(u) && u.side === h.side && u.hp < u.maxhp);
    if (!hurt.length) { ev.note = 'no one hurt'; log.push(ev); return; }
    const t = hurt.slice().sort((x, y) => (y.maxhp - y.hp) - (x.maxhp - x.hp) || place(x, y))[0];
    const amt = Math.round(b.opts.base / 2 * (80 + rnd(41)) / 100);
    const got = Math.min(amt, t.maxhp - t.hp);
    t.hp += got; ev.heal = { target: t.idx, amt: got };
    log.push(ev); return;
  }
  const list = reachable(b, h, kind);
  if (!list.length) { ev.note = 'no target in reach'; log.push(ev); return; }
  let c;
  if (isPlus(h, kind)) {
    const val = (cc) => [cc, ...areaAround(b, cc)].reduce((s, t, i) => s + Math.max(0, Math.min(meanHit(b, h, t, kind) * (i ? 0.5 : 1), t.hp - floorHp(h, t))), 0);
    c = list.slice().sort((x, y) => val(y) - val(x) || place(x, y))[0];
  } else {
    // a flanker goes for anti-flanking units in its reach before anything else
    const pool = h.role === 'flanker' && list.some(antiFlank) ? list.filter(antiFlank) : list;
    if (kind === 'charge' && b.opts.ground && b.opts.ground.flanking) {
      const score = (t) => threatOf(t) / Math.max(1, t.hp);
      c = pool.slice().sort((x, y) => score(y) - score(x) || place(x, y))[0];
    } else c = choose(b, h, kind, pool);
  }
  const K = KINDS[kind];
  let crit = false;
  if (b.opts.dice && K.phys) {
    if (rnd(100) < missChance(b, h, c, kind)) { ev.hits.push({ target: c.idx, dmg: 0, miss: true }); log.push(ev); return; }
    if (!K.area) crit = rnd(100) < critChance(b, h);
  }
  const around = isPlus(h, kind) ? areaAround(b, c) : [];
  applyHit(b, h, c, kind, crit ? CRIT_X : 1, rnd, ev, crit);
  for (const t of around) if (onField(t) && t.hp > floorHp(h, t)) applyHit(b, h, t, kind, 0.5, rnd, ev, false);
  if (crit && K.knock && b.opts.knock) knockBack(b, c, ev);
  log.push(ev);
}

function sideDown(b, s) { return !b.fs.some((u) => u.side === s && onField(u)); }

// A battle is opts.rounds rounds (default 1). In each round, pass 1: every unit acts once; pass 2: those with a
// second action; and so on. Faster units act first and equal speeds are ordered by a coin flip each pass. Every hit
// lands at once. opts.reserves: the defenders on the tile that are not in the line.
function run(placed, opts, seed) {
  const b = setup(placed, opts);
  const rnd = rng32(seed);
  const log = [];
  let ended = false;
  for (let round = 1; round <= (opts.rounds || 1) && !ended; round++) {
    b.round = round;
    const maxN = Math.max(0, ...b.fs.filter(onField).map((u) => actionsAt(u, u.row).n));
    for (let k = 1; k <= maxN && !ended; k++) {
      b.pass = k;
      const movers = b.fs.filter((u) => onField(u) && actionsAt(u, u.row).n >= k).map((u) => ({ u, coin: rnd(1 << 20) }));
      movers.sort((x, y) => speedOf(y.u) - speedOf(x.u) || x.coin - y.coin);
      for (const { u: h } of movers) {
        if (!onField(h) || actionsAt(h, h.row).n < k) continue;
        act(b, h, actionsAt(h, h.row).kind, rnd, log);
        if (sideDown(b, 'att') || sideDown(b, 'def')) { ended = true; break; }
      }
    }
  }
  const lineGone = sideDown(b, 'def');
  const result = lineGone && !(opts.reserves > 0) ? 'taken' : lineGone ? 'cleared' : sideDown(b, 'att') ? 'repulsed' : 'held';
  return { b, log, result };
}

if (typeof module !== 'undefined') module.exports = { setChargeBonus, CHARGE: () => CHARGE_BONUS, antiFlank, groundPct, threatOf, setHpPct, ROWS, ROLES, KINDS, SPELLS, TERRAINS, FEATURES, frontage, lanesFor, actionsAt, speedOf, maxHp, sizeOf, spellOf, lanesOf, run, meanHit, strA, missChance, critChance, CRIT_X };
