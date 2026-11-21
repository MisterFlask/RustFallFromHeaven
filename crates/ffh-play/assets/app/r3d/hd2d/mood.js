// The region moods of the HD-2D mode (docs/HD2D.md §6): per-plot channels from the game state --
// hell terrain, the plot owner's civilization (or, failing a table entry, its alignment), the owner's
// state religion and a few features -- that the mood pass (hd2d/shaders.js MOOD_FS) grades the land
// with. Eight moods, in two textures of four:
//   first:  r hell (ember-red, glowing cracks), g corruption (the Sheaim's violet), b lush (the
//           Ljosalfar's deep green and gold), a gloom (dark elves, vampires: cool and dim);
//   second: r frost (the Illians' rime and snow), g arcane (the Amurites' runes and ley lines),
//           b holy (the Mercurians' and the Order's warm light), a ash (the Khazad's soot).
// Only what the player knows: unrevealed plots carry no mood, remembered plots the owner the map
// reply remembers.

import { S, me, tname } from "../../state.js";
import { KIND, GROUND, FEAT } from "../world.js";

/** The moods, in their channels' order. */
export const MOODS = ["hell", "corruption", "lush", "gloom", "frost", "arcane", "holy", "ash"];
export const M = Object.fromEntries(MOODS.map((k, i) => [k, i]));
const N = MOODS.length;

/** A mood vector from its named parts. */
function mood(o) {
  const m = new Array(N).fill(0);
  for (const [k, v] of Object.entries(o)) m[M[k]] = v;
  return m;
}

/** A civilization's land, by its own character. Civilizations missing here take their alignment's. */
export const CIV_MOOD = {
  CIVILIZATION_LJOSALFAR: mood({ lush: 1 }),
  CIVILIZATION_SHEAIM: mood({ corruption: 1 }),
  CIVILIZATION_INFERNAL: mood({ hell: 0.55, corruption: 0.45, ash: 0.35 }),
  CIVILIZATION_SVARTALFAR: mood({ corruption: 0.1, lush: 0.2, gloom: 0.75 }),
  CIVILIZATION_CALABIM: mood({ corruption: 0.2, gloom: 0.7 }),
  CIVILIZATION_BALSERAPHS: mood({ corruption: 0.25, gloom: 0.15 }),
  CIVILIZATION_CLAN_OF_EMBERS: mood({ hell: 0.1, gloom: 0.3, ash: 0.35 }),
  CIVILIZATION_DOVIELLO: mood({ gloom: 0.25, frost: 0.5 }),
  CIVILIZATION_ILLIANS: mood({ gloom: 0.15, frost: 0.9 }),
  CIVILIZATION_SIDAR: mood({ gloom: 0.45 }),
  CIVILIZATION_ELOHIM: mood({ lush: 0.15, holy: 0.6 }),
  CIVILIZATION_MERCURIANS: mood({ holy: 0.85 }),
  CIVILIZATION_BANNOR: mood({ holy: 0.4 }),
  CIVILIZATION_MALAKIM: mood({ holy: 0.3 }),
  CIVILIZATION_AMURITES: mood({ arcane: 0.9 }),
  CIVILIZATION_LUCHUIRP: mood({ arcane: 0.5, ash: 0.1 }),
  CIVILIZATION_KHAZAD: mood({ ash: 0.6 }),
  // The Kuriotates, Hippus, Grigori and Lanun keep the land's own colours: their accents
  // (moodEmitters, dress.js CIV_PROPS) are particles and props only. A map where every people
  // tints its land is a patchwork in which none stands out.
  CIVILIZATION_KURIOTATES: mood({}),
  CIVILIZATION_HIPPUS: mood({}),
  CIVILIZATION_GRIGORI: mood({}),
  CIVILIZATION_LANUN: mood({}),
};

/** Failing a table entry, the owner's alignment. */
const ALIGNMENT_MOOD = {
  ALIGNMENT_GOOD: mood({ holy: 0.25 }),
  ALIGNMENT_EVIL: mood({ corruption: 0.2, gloom: 0.3 }),
};

/** The owner's state religion, laid over the civilization's mood (RELIGION_WEIGHT): it changes in
 *  the course of a game, so a land's look tells which faith holds it. */
export const RELIGION_MOOD = {
  RELIGION_FELLOWSHIP_OF_LEAVES: mood({ lush: 1 }),
  RELIGION_THE_ORDER: mood({ holy: 1 }),
  RELIGION_THE_EMPYREAN: mood({ holy: 0.8, arcane: 0.2 }),
  RELIGION_RUNES_OF_KILMORPH: mood({ arcane: 0.8 }),
  RELIGION_OCTOPUS_OVERLORDS: mood({ gloom: 0.8, corruption: 0.25 }),
  RELIGION_THE_ASHEN_VEIL: mood({ corruption: 0.8, ash: 0.35 }),
  RELIGION_COUNCIL_OF_ESUS: mood({ gloom: 0.8 }),
};
/** How strongly a religion shows over its civilization: a religion's 1 makes 0.65 of a mood on
 *  land that had none, its 0.8 0.52 -- enough for its props or particles (dress.js clutterOf,
 *  moodEmitters: 0.5). It only adds: a
 *  civilization's own mood never weakens under a faith. */
export const RELIGION_WEIGHT = 0.65;

/** A civilization and its religion: each mood screened over the civilization's (a + b - ab). */
export function combine(civ, rel) {
  const out = civ ? civ.slice() : new Array(N).fill(0);
  if (!rel) return out;
  for (let k = 0; k < N; k++) {
    const r = rel[k] * RELIGION_WEIGHT;
    out[k] = out[k] + r - out[k] * r;
  }
  return out;
}

const HELL = new Set([GROUND.BURNING_SANDS, GROUND.BROKEN_LANDS, GROUND.PERDITION]);

/** The player record of `id` (the view's own player is `me()`, the others `S.players`). */
function playerOf(id) {
  const m = me();
  return m && id === m.player ? m : S.players.get(id) ?? null;
}

const has = (v) => v !== null && v !== undefined;

/** The mood of the land each player owns, and its civilization's name (by player id). */
function owners() {
  const out = new Map();
  const ids = new Set([...S.players.keys()]);
  if (me()) ids.add(me().player);
  for (const id of ids) {
    const p = playerOf(id);
    if (!p) continue;
    const civ = has(p.civ) ? tname("CivilizationType", p.civ) : null;
    const al = has(p.alignment) ? tname("AlignmentType", p.alignment) : null;
    const rel = has(p.state_religion) ? tname("ReligionType", p.state_religion) : null;
    const base = CIV_MOOD[civ] ?? ALIGNMENT_MOOD[al] ?? null;
    const m = combine(base, RELIGION_MOOD[rel] ?? null);
    out.set(id, { mood: m.some((v) => v > 0) ? m : null, civ });
  }
  return out;
}

/** The mood of every plot: RGBA bytes, one texel a plot, in two textures of four moods (`data`,
 *  `data2`: map3d's MapTextures.setMood). Also the plots' moods as numbers (`moods[i]`, the eight of
 *  MOODS), for the particles and props, and the owner's civilization a plot (`civs[i]`, a type
 *  name or null), for the civilizations' own accents. */
export function plotMoods(world, map) {
  const n = world.w * world.h;
  const data = new Uint8Array(n * 4);
  const data2 = new Uint8Array(n * 4);
  const moods = new Array(n);
  const civs = new Array(n).fill(null);
  const own = owners();
  for (let i = 0; i < n; i++) {
    if (world.kind[i] === KIND.VOID) {
      moods[i] = null;
      continue;
    }
    const m = new Array(N).fill(0);
    const o = map.owner[i] >= 0 ? own.get(map.owner[i]) : null;
    if (o) civs[i] = o.civ;
    if (o?.mood) for (let k = 0; k < N; k++) m[k] = o.mood[k];
    // The sea takes half of its owner's mood: the coast of a corrupted land is dimmed, not dyed.
    if (world.isWater(i)) for (let k = 0; k < N; k++) m[k] *= 0.5;
    if (HELL.has(world.ground[i])) m[M.hell] = 1;
    const f = world.feat[i];
    if (f === FEAT.ANCIENT) m[M.lush] = Math.max(m[M.lush], 0.6);
    if (f === FEAT.FLAMES || f === FEAT.VOLCANO) m[M.hell] = Math.max(m[M.hell], 0.7);
    if (f === FEAT.SOULS) {
      m[M.hell] = Math.max(m[M.hell], 0.4);
      m[M.corruption] = Math.max(m[M.corruption], 0.5);
    }
    // Hell overrides the others where it holds (a corrupted forest is hell, not lush), all but its
    // own ash.
    const keep = 1 - m[M.hell] * 0.8;
    for (let k = 1; k < N; k++) if (k !== M.ash) m[k] *= keep;
    moods[i] = m;
    for (let k = 0; k < 4; k++) {
      data[i * 4 + k] = Math.round(Math.min(1, m[k]) * 255);
      data2[i * 4 + k] = Math.round(Math.min(1, m[k + 4]) * 255);
    }
  }
  return { data, data2, moods, civs };
}

/** A plot's own die roll in [0, 1), for the accents that stand on some plots only. */
function roll(i, salt) {
  let h = (i * 374761393 + salt * 668265263) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

const WOODS = new Set([FEAT.FOREST, FEAT.ANCIENT, FEAT.JUNGLE]);

/** The particles of the moods and the civilizations' accents. Deterministic. Rising motes over
 *  corrupted land (violet), the arcane (cyan) and lush woods (gold-green); snow, ash and the holy
 *  lands' pale motes falling (a negative rise: scene.js PARTICLE_VS runs their life backwards);
 *  wisps in gloomy woods. (The 3D renderer's hell embers already rise from hell terrain.) */
export function moodEmitters(world, moods, civs = []) {
  const out = [];
  for (let i = 0; i < moods.length; i++) {
    const m = moods[i];
    if (!m || world.isWater(i)) continue;
    const x = i % world.w, y = (i - x) / world.w;
    const at = (dy) => ({ x: x + 0.5, y: world.heightAt(x + 0.5, y + 0.5) + dy, z: -(y + 0.5) });
    const f = world.feat[i];
    const g = world.ground[i];
    if (m[M.corruption] >= 0.5) out.push({ ...at(0.08), color: "#c07aff", size: 0.016, rise: 0.18, count: 3, spread: 0.4 });
    if (m[M.lush] >= 0.6 && (f === FEAT.FOREST || f === FEAT.ANCIENT)) out.push({ ...at(0.12), color: "#e4ff9a", size: 0.012, rise: 0.12, count: 2, spread: 0.42 });
    // Snow: flakes falling to the ground through the height of a tree.
    if (m[M.frost] >= 0.5) out.push({ ...at(0.0), color: "#eef4ff", size: 0.011, rise: -0.11, count: Math.round(2 + 3 * m[M.frost]), spread: 0.46 });
    if (m[M.arcane] >= 0.5) out.push({ ...at(0.06), color: "#7fe2ff", size: 0.014, rise: 0.15, count: 2, spread: 0.4 });
    if (m[M.holy] >= 0.5) out.push({ ...at(0.04), color: "#fff1cc", size: 0.012, rise: -0.06, count: 2, spread: 0.44 });
    // Ash: dark flakes (the particles are composited over the frame, so a dark one reads).
    if (m[M.ash] >= 0.5) out.push({ ...at(0.0), color: "#57514c", size: 0.012, rise: -0.09, count: 3, spread: 0.46 });
    if (m[M.gloom] >= 0.5 && WOODS.has(f)) out.push({ ...at(0.1), color: "#5fe0c4", size: 0.014, rise: 0.05, count: 1, spread: 0.36 });
    // The civilizations' own accents.
    const civ = civs[i];
    if (civ === "CIVILIZATION_SIDAR" && roll(i, 1) < 0.4) out.push({ ...at(0.12), color: "#cfd8ec", size: 0.02, rise: 0.04, count: 1, spread: 0.3 });
    if (civ === "CIVILIZATION_KURIOTATES" && g === GROUND.GRASS && !WOODS.has(f) && roll(i, 2) < 0.35) out.push({ ...at(0.05), color: "#f2ff8e", size: 0.008, rise: 0.03, count: 2, spread: 0.4 });
    // Dust kicked up on the open steppe (the Hippus) and the desert (the Malakim): soft puffs.
    const dusty = (civ === "CIVILIZATION_HIPPUS" && (g === GROUND.PLAINS || g === GROUND.GRASS)) || (civ === "CIVILIZATION_MALAKIM" && g === GROUND.DESERT);
    if (dusty && !WOODS.has(f) && roll(i, 3) < 0.3) out.push({ ...at(0.01), color: "#bfa47e", size: 0.03, rise: 0.08, count: 1, spread: 0.38, smoke: true });
  }
  return out;
}
