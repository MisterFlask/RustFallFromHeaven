// The region moods of the HD-2D mode (docs/HD2D.md): a per-plot channel from the game state --
// hell terrain, the plot owner's civilization (or, failing a table entry, its alignment) and a few
// features -- that the mood pass (hd2d/shaders.js MOOD_FS) grades the land with: r hell (ember-red,
// glowing cracks), g corruption (the Sheaim's violet), b lush (the Ljosalfar's deep green and gold),
// a gloom (dark elves, vampires: cool and dim). Only what the player knows: unrevealed plots carry
// no mood, remembered plots the owner the map reply remembers.

import { S, me, tname } from "../../state.js";
import { KIND, GROUND, FEAT } from "../world.js";

/** A civilization's land: [hell, corruption, lush, gloom], each 0..1. */
const CIV_MOOD = {
  CIVILIZATION_LJOSALFAR: [0, 0, 1, 0],
  CIVILIZATION_SHEAIM: [0, 1, 0, 0],
  CIVILIZATION_INFERNAL: [0.55, 0.45, 0, 0],
  CIVILIZATION_SVARTALFAR: [0, 0.1, 0.2, 0.75],
  CIVILIZATION_CALABIM: [0, 0.2, 0, 0.7],
  CIVILIZATION_BALSERAPHS: [0, 0.25, 0, 0.15],
  CIVILIZATION_CLAN_OF_EMBERS: [0.1, 0, 0, 0.35],
  CIVILIZATION_DOVIELLO: [0, 0, 0, 0.35],
  CIVILIZATION_ILLIANS: [0, 0, 0, 0.4],
  CIVILIZATION_ELOHIM: [0, 0, 0.35, 0],
  CIVILIZATION_MERCURIANS: [0, 0, 0.3, 0],
};

/** Failing a table entry, the owner's alignment. */
const ALIGNMENT_MOOD = {
  ALIGNMENT_GOOD: [0, 0, 0.35, 0],
  ALIGNMENT_EVIL: [0, 0.2, 0, 0.3],
};

const HELL = new Set([GROUND.BURNING_SANDS, GROUND.BROKEN_LANDS, GROUND.PERDITION]);

/** The player record of `id` (the view's own player is `me()`, the others `S.players`). */
function playerOf(id) {
  const m = me();
  return m && id === m.player ? m : S.players.get(id) ?? null;
}

/** The mood of the land each player owns (by player id). */
function ownerMoods() {
  const out = new Map();
  const ids = new Set([...S.players.keys()]);
  if (me()) ids.add(me().player);
  for (const id of ids) {
    const p = playerOf(id);
    if (!p) continue;
    const civ = tname("CivilizationType", p.civ);
    const al = p.alignment !== null && p.alignment !== undefined ? tname("AlignmentType", p.alignment) : null;
    out.set(id, CIV_MOOD[civ] ?? ALIGNMENT_MOOD[al] ?? null);
  }
  return out;
}

/** The mood of every plot: RGBA bytes, one texel a plot (map3d's MapTextures.setMood). Also the
 *  plots' moods as numbers (`moods[i] = [h, c, l, g]`), for the particles. */
export function plotMoods(world, map) {
  const n = world.w * world.h;
  const data = new Uint8Array(n * 4);
  const moods = new Array(n);
  const owners = ownerMoods();
  for (let i = 0; i < n; i++) {
    if (world.kind[i] === KIND.VOID) {
      moods[i] = null;
      continue;
    }
    const m = [0, 0, 0, 0];
    const own = map.owner[i] >= 0 ? owners.get(map.owner[i]) : null;
    if (own) for (let k = 0; k < 4; k++) m[k] = own[k];
    // The sea takes half of its owner's mood: the coast of a corrupted land is dimmed, not dyed.
    if (world.isWater(i)) for (let k = 0; k < 4; k++) m[k] *= 0.5;
    if (HELL.has(world.ground[i])) m[0] = 1;
    const f = world.feat[i];
    if (f === FEAT.ANCIENT) m[2] = Math.max(m[2], 0.6);
    if (f === FEAT.FLAMES || f === FEAT.VOLCANO) m[0] = Math.max(m[0], 0.7);
    if (f === FEAT.SOULS) {
      m[0] = Math.max(m[0], 0.4);
      m[1] = Math.max(m[1], 0.5);
    }
    // Hell overrides the others where it holds (a corrupted forest is hell, not lush).
    const keep = 1 - m[0] * 0.8;
    m[1] *= keep;
    m[2] *= keep;
    m[3] *= keep;
    moods[i] = m;
    for (let k = 0; k < 4; k++) data[i * 4 + k] = Math.round(Math.min(1, m[k]) * 255);
  }
  return { data, moods };
}

/** The particles of the moods: violet motes drifting over corrupted land, gold-green ones in a
 *  lush wood (the 3D renderer's hell embers already rise from hell terrain). Deterministic. */
export function moodEmitters(world, moods) {
  const out = [];
  for (let i = 0; i < moods.length; i++) {
    const m = moods[i];
    if (!m || world.isWater(i)) continue;
    const x = i % world.w, y = (i - x) / world.w;
    const at = (dy) => ({ x: x + 0.5, y: world.heightAt(x + 0.5, y + 0.5) + dy, z: -(y + 0.5) });
    if (m[1] >= 0.5) out.push({ ...at(0.08), color: "#c07aff", size: 0.016, rise: 0.18, count: 3, spread: 0.4 });
    if (m[2] >= 0.6 && (world.feat[i] === FEAT.FOREST || world.feat[i] === FEAT.ANCIENT)) out.push({ ...at(0.12), color: "#e4ff9a", size: 0.012, rise: 0.12, count: 2, spread: 0.42 });
  }
  return out;
}
