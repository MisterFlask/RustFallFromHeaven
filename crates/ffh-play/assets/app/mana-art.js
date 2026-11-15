// Sphere identity shared by the physical HD-2D / isometric PNGs and distant map labels.
import { MANA } from "./r3d/assets/palette.js";
import { CARDS } from "./r3d/hd2d/art/index.js";

export const MANA_SPHERES = {
  AIR: "Ai", BODY: "Bo", CHAOS: "Ch", DEATH: "De", EARTH: "Ea", ENCHANTMENT: "En",
  ENTROPY: "Et", FIRE: "Fi", ICE: "Ic", LAW: "La", LIFE: "Li", METAMAGIC: "Me",
  MIND: "Mi", NATURE: "Na", SHADOW: "Sh", SPIRIT: "Sp", SUN: "Su", WATER: "Wa",
};

export function manaSchool(type) {
  const school = /^(?:BONUS|IMPROVEMENT)_MANA_([A-Z]+)$/.exec(type ?? "")?.[1];
  return Object.hasOwn(MANA_SPHERES, school) ? school : null;
}

export function manaSpriteKey(type, built = false) {
  const school = manaSchool(type);
  const key = school ? `${built ? "feat" : "res"}_mana_${school.toLowerCase()}` : null;
  return Object.hasOwn(MANA_SPRITES, key) ? key : null;
}

export const MANA_SPRITES = Object.fromEntries(Object.entries(CARDS)
  .filter(([key]) => /^(?:res|feat)_mana_/.test(key)));

export function manaColour(type) {
  return MANA[manaSchool(type)] ?? MANA[""];
}

/** Contrasting lettering on the sphere's colour at distant zooms. */
export function manaInk(type) {
  const n = parseInt(manaColour(type).slice(1), 16);
  const light = (n >> 16) * .299 + ((n >> 8) & 255) * .587 + (n & 255) * .114;
  return light < 140 ? "#fff6e5" : "#201a28";
}
