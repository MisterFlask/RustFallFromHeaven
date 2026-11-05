#!/usr/bin/env node
// gen-types.mjs: extract the game types the art must cover from data/json into types.json.
//
//   node tools/gen-types.mjs            write ../types.json
//   node tools/gen-types.mjs --check    exit 1 if types.json is stale
//
// types.json lists, per kind, every type of the game data with the few facts the sprite mapping
// keys on (unit combat class, domain, tier, race, ...). It is generated, committed, and loaded by
// art.js so the browser can resolve fallbacks without the data files. Only facts come from the
// data; any curated grouping (which promotions are afflictions, ...) lives in the set manifests.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
export const ART_DIR = resolve(HERE, '..');
export const DATA_DIR = resolve(HERE, '../../../../../data/json');

/** JSON with comments (data/json/*.jsonc): drops comments and trailing commas outside strings. */
export function parseJsonc(src) {
  let out = '';
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    if (c === '"') {
      let j = i + 1;
      while (src[j] !== '"') j += src[j] === '\\' ? 2 : 1;
      out += src.slice(i, j + 1);
      i = j;
    } else if (c === '/' && src[i + 1] === '/') {
      while (i < src.length && src[i] !== '\n') i++;
      out += '\n';
    } else if (c === '/' && src[i + 1] === '*') {
      i = src.indexOf('*/', i + 2) + 1;
    } else if (c === ',') {
      let j = i + 1;
      while (/\s/.test(src[j] || '')) j++;
      if (src[j] !== ']' && src[j] !== '}') out += c;
    } else out += c;
  }
  return JSON.parse(out);
}

/** The entries of a table's file (`units`, `promotions`, ...). */
function table(dataDir, name) {
  return parseJsonc(readFileSync(join(dataDir, `${name}.jsonc`), 'utf8'));
}
const ref = (e, key) => e[key] || 'NONE';
const flag = (e, key) => (e[key] === true ? 1 : 0);
const num = (e, key) => (Number.isInteger(e[key]) ? e[key] : 0);
const short = (s, prefix) => (s && s !== 'NONE' ? s.replace(prefix, '') : 'NONE');

// Race and nature promotions that change a unit's look (docs/ART.md section 10.4).
const SPHERES = ['AIR', 'BODY', 'CHAOS', 'DEATH', 'EARTH', 'ENCHANTMENT', 'ENTROPY', 'FIRE', 'ICE', 'LAW', 'LIFE',
  'METAMAGIC', 'MIND', 'NATURE', 'SHADOW', 'SPIRIT', 'SUN', 'WATER'];

export function buildTypes(dataDir = DATA_DIR) {
  const K = {};
  K.terrain = table(dataDir, 'terrains').map((e) => ({
    type: e.type, water: flag(e, 'water'), impassable: flag(e, 'impassable'), graphical: flag(e, 'graphical_only'),
    up: ref(e, 'terrain_up'), down: ref(e, 'terrain_down'),
  }));
  // a hell terrain is one some other terrain turns into
  const hell = new Set(K.terrain.map((t) => t.up).filter((u) => u !== 'NONE'));
  for (const t of K.terrain) {
    t.hell = hell.has(t.type) ? 1 : 0;
    t.twin = t.hell ? K.terrain.find((o) => o.up === t.type).type : 'NONE';
  }
  K.feature = table(dataDir, 'features').map((e) => ({
    type: e.type, impassable: flag(e, 'impassable'), graphical: flag(e, 'graphical_only'),
  }));
  K.bonus = table(dataDir, 'bonuses').map((e) => {
    const y = e.yield_changes || [];
    const cls = short(ref(e, 'bonus_class_type'), 'BONUSCLASS_');
    const health = num(e, 'health'), happy = num(e, 'happiness');
    // heuristic resource class (docs/ART.md 6.1: food, strategic, luxury, mana); manifests may override per type
    let cat = 'luxury';
    if (cls === 'MANA' || cls === 'RAWMANA') cat = 'mana';
    else if (cls === 'RUSH' || cls === 'WONDER' || cls === 'MODERN') cat = 'strategic';
    else if (cls === 'GRAIN' || cls === 'LIVESTOCK' || (health > 0 && happy === 0)) cat = 'food';
    return { type: e.type, cls, cat, health, happy, food: y[0] || 0 };
  });
  K.improvement = table(dataDir, 'improvements').map((e) => {
    const type = e.type;
    return {
      type, permanent: flag(e, 'permanent'), unique: flag(e, 'unique'), goody: flag(e, 'goody'),
      graphical: flag(e, 'graphical_only'), water: flag(e, 'water'), actsAsCity: flag(e, 'acts_as_city'),
      spawn: ref(e, 'spawn_unit_type'), civ: ref(e, 'prereq_civilization'),
      mana: type.startsWith('IMPROVEMENT_MANA_') ? 1 : 0,
    };
  });
  K.route = table(dataDir, 'routes').map((e) => ({ type: e.type }));

  const promoEls = table(dataDir, 'promotions');
  const raceSet = new Set(promoEls.filter((e) => flag(e, 'race')).map((e) => e.type));
  K.promotion = promoEls.map((e) => {
    const type = e.type;
    const name = type.replace('PROMOTION_', '');
    const m = /^(.*?)(\d)$/.exec(name);
    const line = m ? m[1] : name;
    const bonus = ref(e, 'bonus_prereq');
    let sphere = 'NONE';
    if (bonus.startsWith('BONUS_MANA_')) sphere = bonus.replace('BONUS_MANA_', '');
    else if (m && SPHERES.includes(line)) sphere = line;
    return {
      type, race: flag(e, 'race'), equipment: flag(e, 'equipment'), sphere, line, level: m ? +m[2] : 0,
      dispellable: flag(e, 'dispellable'), expires: num(e, 'expire_chance') > 0 ? 1 : 0, mutation: flag(e, 'mutation'),
    };
  });

  const unitClassEls = table(dataDir, 'unit_classes');
  const classDefault = new Map(unitClassEls.map((e) => [e.type, ref(e, 'default_unit')]));
  K.unit = table(dataDir, 'units').map((e) => {
    const type = e.type;
    const cls = e.class || '';
    const free = Object.entries(e.free_promotions || {}).filter(([, v]) => v === true).map(([p]) => p);
    const races = free.filter((p) => raceSet.has(p)).map((p) => p.replace('PROMOTION_', ''));
    const def = classDefault.get(cls) || 'NONE';
    return {
      type, cls, combat: short(ref(e, 'unit_combat'), 'UNITCOMBAT_'), domain: short(ref(e, 'domain'), 'DOMAIN_'),
      tier: num(e, 'tier'), special: short(ref(e, 'special'), 'SPECIALUNIT_'), race: races.length ? races : ['NONE'],
      hero: free.includes('PROMOTION_HERO') ? 1 : 0, equipment: type.startsWith('EQUIPMENT_') ? 1 : 0,
      invisible: ref(e, 'invisible_type') !== 'NONE' ? 1 : 0, strength: num(e, 'combat'),
      base: def !== 'NONE' && def !== type ? def : 'NONE',
    };
  });

  K.civ = table(dataDir, 'civilizations').map((e) => ({
    type: e.type, playable: flag(e, 'playable'),
  }));
  K.religion = table(dataDir, 'religions').map((e) => ({ type: e.type }));

  const bclassEls = table(dataDir, 'building_classes');
  const bclass = new Map(bclassEls.map((e) => [e.type, { global: num(e, 'max_global_instances'), player: num(e, 'max_player_instances') }]));
  K.building = table(dataDir, 'buildings').map((e) => {
    const cls = e.building_class || '';
    const bc = bclass.get(cls) || { global: -1, player: -1 };
    const religion = ref(e, 'prereq_religion');
    const relType = ref(e, 'religion_type');
    return {
      type: e.type, cls,
      wonder: bc.global > 0 ? 'WORLD' : bc.player > 0 ? 'NATIONAL' : 'NONE',
      religion: religion !== 'NONE' ? religion : relType,
      defense: num(e, 'defense') > 0 ? 1 : 0,
      graphical: flag(e, 'graphical_only'),
    };
  });
  K.unitcombat = table(dataDir, 'unit_combats').map((e) => ({ type: e.type }));
  K.specialist = table(dataDir, 'specialists').map((e) => {
    const type = e.type;
    return { type, great: type.startsWith('SPECIALIST_GREAT_') ? 1 : 0 };
  });
  return K;
}

/** Stable text form: one entry per line, so diffs stay readable. */
export function formatTypes(K) {
  const out = ['{"source": "data/json via crates/ffh-play/web/art/tools/gen-types.mjs; do not edit", "kinds": {'];
  const kinds = Object.keys(K);
  kinds.forEach((k, ki) => {
    out.push(`"${k}": [`);
    K[k].forEach((e, i) => out.push(JSON.stringify(e) + (i < K[k].length - 1 ? ',' : '')));
    out.push(']' + (ki < kinds.length - 1 ? ',' : ''));
  });
  out.push('}}');
  return out.join('\n') + '\n';
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!existsSync(DATA_DIR)) { console.error(`gen-types: no game data at ${DATA_DIR}`); process.exit(2); }
  const text = formatTypes(buildTypes());
  const target = join(ART_DIR, 'types.json');
  if (process.argv.includes('--check')) {
    const cur = existsSync(target) ? readFileSync(target, 'utf8') : '';
    if (cur !== text) { console.error('gen-types: types.json is stale; run node tools/gen-types.mjs'); process.exit(1); }
    console.log('gen-types: types.json is current');
  } else {
    writeFileSync(target, text);
    const K = JSON.parse(text).kinds;
    console.log('gen-types: wrote types.json:', Object.entries(K).map(([k, v]) => `${k} ${v.length}`).join(', '));
  }
}
