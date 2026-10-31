// Units: FfH 0.41 stats (iCombat / iCombatDefense, first strikes, withdrawal, combat limit, tier).
const U = (name, role, str, def, o = {}) => ({ name, role, str, def, level: 1, tier: 2, fs: 0, wd: 0, lim: 100, ...o });
const UNITS = {
  champion: () => U('Champion', 'frontline', 6, 6, { tier: 3 }),
  axeman: () => U('Axeman', 'frontline', 4, 4),
  swordsman: () => U('Swordsman', 'screen', 4, 4, { note: 'Formation' }),
  hunter: () => U('Hunter', 'frontline', 4, 4),
  ranger: () => U('Ranger', 'frontline', 7, 7, { tier: 3 }),
  hillgiant: () => U('Hill Giant', 'frontline', 7, 7, { large: true }),
  archer: () => U('Archer', 'archer', 3, 5, { fs: 1 }),
  longbow: () => U('Longbowman', 'archer', 5, 6, { fs: 1, tier: 3 }),
  horseman: () => U('Horseman', 'flanker', 4, 4, { fs: 1, wd: 25 }),
  horsearcher: () => U('Horse Archer', 'flanker', 6, 6, { fs: 1, wd: 35, tier: 3 }),
  elephant: () => U('War Elephant', 'flanker', 7, 7, { wd: 25, tier: 3, large: true }),
  assassin: () => U('Assassin', 'assassin', 5, 3, { fs: 1, tier: 3 }),
  adept: () => U('Adept', 'arcane', 3, 3),
  mage: () => U('Mage', 'arcane', 4, 4, { tier: 3, spell: 'maelstrom', note: 'Air II' }),
  priest: () => U('Priest of the Order', 'divine', 4, 4, { tier: 3 }),
  disciple: () => U('Disciple of Leaves', 'divine', 3, 3),
  catapult: () => U('Catapult', 'siege', 4, 3, { wd: 80, lim: 75 }),
};
const lv = (u, level, extra = {}) => ({ ...u, level, ...extra });

const SCENARIOS = {
  open: {
    name: 'Field battle on grassland',
    ground: { terrain: 'grassland', feature: 'none', hills: false, city: false, cityDef: 50 },
    att: [lv(UNITS.champion(), 4), UNITS.axeman(), UNITS.axeman(), lv(UNITS.swordsman(), 2), UNITS.longbow(), UNITS.longbow(),
      UNITS.elephant(), UNITS.horseman(), UNITS.mage(), UNITS.priest(), UNITS.catapult(), UNITS.assassin()],
    def: [lv(UNITS.ranger(), 3), UNITS.champion(), UNITS.hillgiant(), lv(UNITS.hunter(), 2), lv(UNITS.swordsman(), 2), UNITS.longbow(), UNITS.longbow(),
      UNITS.archer(), UNITS.horsearcher(), UNITS.adept(), UNITS.disciple()],
  },
  forest: {
    name: 'Attack into a forest',
    ground: { terrain: 'grassland', feature: 'forest', hills: false, city: false, cityDef: 50 },
    att: [lv(UNITS.champion(), 4), UNITS.axeman(), UNITS.axeman(), lv(UNITS.swordsman(), 2), UNITS.longbow(), UNITS.longbow(),
      UNITS.horseman(), UNITS.mage(), UNITS.priest(), UNITS.assassin()],
    def: [lv(UNITS.ranger(), 3), UNITS.axeman(), UNITS.hunter(), UNITS.archer(), UNITS.archer(), UNITS.horsearcher(), UNITS.disciple()],
  },
  city: {
    name: 'Assault on a walled city',
    ground: { terrain: 'plains', feature: 'none', hills: false, city: true, cityDef: 50 },
    att: [lv(UNITS.champion(), 4), UNITS.axeman(), UNITS.axeman(), lv(UNITS.swordsman(), 2), UNITS.longbow(), UNITS.longbow(), UNITS.mage(),
      UNITS.catapult(), UNITS.catapult(), UNITS.priest(), UNITS.assassin()],
    def: [lv(UNITS.ranger(), 3), UNITS.axeman(), UNITS.axeman(), UNITS.archer(), UNITS.archer(), UNITS.archer(), UNITS.adept(), UNITS.disciple()],
  },
};

// The rules' proposal: one unit of each role first, then the strongest, up to the slots (a large unit counts two);
// each in its role's home row, centre lanes first, large units placed before the rest.
const HOME = { frontline: 0, screen: 0, flanker: 0, assassin: 1, archer: 1, siege: 2, arcane: 2, divine: 1 };
const PICK = ['screen', 'frontline', 'archer', 'flanker', 'arcane', 'divine', 'siege', 'assassin'];
function autoArrange(pool, lanes, slots) {
  const value = (u) => u.str * (100 + 10 * (u.level - 1)) * (u.large ? 2 : 1);
  const order = pool.map((u, i) => ({ u, i })).sort((x, y) => PICK.indexOf(x.u.role) - PICK.indexOf(y.u.role) || value(y.u) - value(x.u) || x.i - y.i);
  const taken = [], rest = [], seen = new Set();
  for (const o of order) { if (!seen.has(o.u.role)) { taken.push(o); seen.add(o.u.role); } else rest.push(o); }
  const chosen = [];
  let used = 0;
  for (const o of [...taken, ...rest]) { const s = o.u.large ? 2 : 1; if (used + s <= slots) { chosen.push(o); used += s; } }
  const grid = Array.from({ length: 3 }, () => Array(lanes).fill(null));
  const centre = (l) => Math.abs(l - (lanes - 1) / 2);
  const placed = [];
  const rowsFrom = (row) => [row, row + 1, row - 1, row + 2, row - 2].filter((x) => x >= 0 && x < 3);
  for (const o of [...chosen.filter((o) => o.u.large), ...chosen.filter((o) => !o.u.large)]) {
    let cell = null;
    for (const r of rowsFrom(HOME[o.u.role])) {
      const ls = [...Array(lanes).keys()].filter((l) => o.u.large ? l + 1 < lanes && grid[r][l] === null && grid[r][l + 1] === null : grid[r][l] === null)
        .sort((a, b) => centre(o.u.large ? a + 0.5 : a) - centre(o.u.large ? b + 0.5 : b) || a - b);
      if (ls.length) { cell = { r, l: ls[0] }; break; }
    }
    if (!cell) continue;
    grid[cell.r][cell.l] = o.i;
    if (o.u.large) grid[cell.r][cell.l + 1] = o.i;
    placed.push({ i: o.i, lane: cell.l, row: cell.r });
  }
  return placed;
}
if (typeof module !== 'undefined') module.exports = { UNITS, SCENARIOS, autoArrange };

// Repeated attacks: both sides re-form with what is left (the attackers' withdrawn units fight again, the defenders'
// withdrawn units have fled the tile), one battle after another with no healing and no reinforcements, until the tile
// is empty or the attackers are gone.
function campaign(att, def, ground, opts, seed, maxBattles = 8) {
  const En = typeof run === 'function' ? { run, frontage } : require('./engine.js');
  const fr = En.frontage(ground);
  let A = att.map((u) => ({ ...u, damage: u.damage || 0 })), D = def.map((u) => ({ ...u, damage: u.damage || 0 }));
  for (let n = 1; n <= maxBattles; n++) {
    const pa = autoArrange(A, fr.lanes.att, fr.slots.att), pd = autoArrange(D, fr.lanes.def, fr.slots.def);
    const placed = [...pa.map((p) => ({ unit: A[p.i], side: 'att', lane: p.lane, row: p.row, src: p.i })),
      ...pd.map((p) => ({ unit: D[p.i], side: 'def', lane: p.lane, row: p.row, src: p.i }))];
    const r = En.run(placed, { lanes: fr.lanes, base: opts.base, plotDef: fr.plotDef, city: fr.city, ground: fr.ground, dice: opts.dice, miss: opts.miss, crit: opts.crit, knock: opts.knock, rounds: opts.rounds, reserves: D.length - pd.length }, seed * 7349 + n * 101);
    r.b.fs.forEach((f, i) => {
      const p = placed[i], u = (p.side === 'att' ? A : D)[p.src];
      if (!f.alive || (f.gone && p.side === 'def')) u.out = true;
      else u.damage = Math.round(100 * (1 - f.hp / f.maxhp));
    });
    A = A.filter((u) => !u.out); D = D.filter((u) => !u.out);
    if (!D.length) return { battles: n, fell: true };
    if (!A.length) return { battles: n, fell: false, spent: true };
  }
  return { battles: maxBattles, fell: false };
}
if (typeof module !== 'undefined') module.exports.campaign = campaign;
