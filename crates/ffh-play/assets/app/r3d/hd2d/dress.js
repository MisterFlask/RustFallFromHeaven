// What the HD-2D mode stands in the world as sprites (docs/HD2D.md): the isometric renderer's art
// (docs/ISO.md), chosen as iso.js chooses it -- a unit's sprite by its civilization, type, class and
// stand-ins (iso.js unitSpriteKeys), a city's by its size and civilization, an improvement's and a
// resource's by kind -- and placed on the 3D renderer's ground. A stack is a group of two or three
// figures of its head unit under the owner's banner; a city, a standing improvement and a resource
// are one card each. The 3D models of what these replace are not placed (`claims`, read by
// ../dress.js); forests, rocks, fields, flat improvements and the land stay the 3D renderer's.
//
// Records (hd2d/sprites.js packs them): { key, x, y, z (the base point, world), w, h (the card,
// world units), anchor (0..1 across the card: where the base point is), flip, banner, shadow (a
// figure: casts into the sun's map, has a contact shadow), team [r, g, b], bias (depth toward the
// eye), id (object id), glow, phase, uid (walking: hd2d/index.js) }.

import { S, tname } from "../../state.js";
import * as T from "../../tiles.js";
import { stackState } from "../../stacks.js";
import { isBarbarian } from "../../state.js";
import { unitSpriteKeys, civShortName, ISO_TABLES } from "../../iso.js";
import { hashf, hash32 } from "../noise.js";
import { SEA, DRAFT, FEAT, KIND, RIVER_BANK } from "../world.js";
import { standHeight, hexRgb } from "../dress.js";
import { MANA } from "../assets/palette.js";

/** World units per art pixel: figures (a 96 x 120 unit canvas is 0.46 x 0.58 of a plot), cities (a
 *  192-wide settlement 1.15 plots: its painted ground spans the plot), the other buildings, the
 *  resources' props, the banners. */
export const PX = { unit: 0.0048, city: 0.006, building: 0.0038, resource: 0.0034, banner: 0.0053 };
/** Depth toward the eye (world units): a figure before the card of the building it stands at. */
const BIAS = { unit: 0.26, garrison: 0.4, building: 0, resource: 0.08 };
/** Where a stack stands (plot-local x east, z south); on a city's plot, its front-left corner; on a
 *  standing improvement's, in front of it. */
const SPOT = [0, 0.06];
const BUILT_SPOT = [0.05, 0.3];
const VOLCANO_SPOT = [-0.14, 0.38];
const BONUS_SPOT = [-0.26, 0.2];
/** Where a city's garrison stands (plot-local): in front of the city's card (whose foot is on the
 *  plot's front edge), the first of a few spots out of a river, else the one furthest from it. */
const GARRISON = [[-0.27, 0.36], [0.27, 0.36], [-0.1, 0.42], [0.12, 0.42]];
function garrisonSpot(world, x, y) {
  if (!world.riverCells.size) return GARRISON[0];
  let best = GARRISON[0], bestD = -1;
  for (const s of GARRISON) {
    const d = world.riverDist(x + 0.5 + s[0], y + 0.5 - s[1]);
    if (d >= RIVER_BANK + 0.05) return s;
    if (d > bestD) [best, bestD] = [s, d];
  }
  return best;
}

/** A group of figures about its spot: two in front, one behind between them, far enough apart that
 *  each reads as a figure (about 0.6 of a figure's width), not a single blot. */
const GROUP = {
  3: [[-0.1, 0.07], [0.11, 0.05], [0.0, -0.08]],
  2: [[-0.08, 0.05], [0.09, -0.03]],
  1: [[0, 0]],
};

const ids = new Map();
let nextId = 1;
/** A stable object id in 0..1 (the ghost pass tells a figure's own pixels by it). */
function idOf(key) {
  let v = ids.get(key);
  if (v === undefined) {
    v = ((hash32(nextId++ * 2246822519) % 9973) + 1) / 9975;
    ids.set(key, v);
  }
  return v;
}

function teamCss(owner) {
  return T.playerColor(owner, isBarbarian(owner));
}

/** The first of `keys` the atlas has, or null. */
function first(atlas, keys) {
  for (const k of keys) if (k && atlas.has(k)) return k;
  return null;
}

/** How many figures stand for a unit: one for a big creature, a ship, an engine or a hero-sized
 *  sprite (wider or taller than the unit contract's 96 x 120), two for a settler or a worker, three
 *  for the rest (a company). */
function groupSize(info, art) {
  if (!art || art.w > 110 || art.h > 130) return 1;
  const dom = info ? tname("DomainType", info.domain) : null;
  const combat = info ? tname("UnitCombatType", info.unit_combat) : null;
  if (dom === "DOMAIN_SEA" || dom === "DOMAIN_AIR" || combat === "UNITCOMBAT_SIEGE" || combat === "UNITCOMBAT_NAVAL") return 1;
  if (info?.found || info?.worker) return 2;
  return 3;
}

/** The owner's banner as a sprite (drawn here, so the colour stays a runtime choice): a gonfalon on
 *  a pole -- the cloth in the owner's colour with a pale charge and gold trim, hanging from a
 *  crossbar under a gold finial. 40 x 150 art px, its pole's foot the card's base. */
export function bannerKey(atlas, css) {
  const key = `banner:${css}`;
  if (atlas.has(key)) return key;
  const W = 40, H = 150;
  const c = typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(W, H) : Object.assign(document.createElement("canvas"), { width: W, height: H });
  const g = c.getContext("2d");
  const px = (x, y, w, h, col) => {
    g.fillStyle = col;
    g.fillRect(x, y, w, h);
  };
  const [r, gg, b] = hexRgb(cssHex(css)).map((v) => v * 255);
  const shade = (k) => `rgb(${Math.round(r * k)},${Math.round(gg * k)},${Math.round(b * k)})`;
  // Pole and finial.
  px(4, 8, 4, 142, "#4a2c14");
  px(4, 8, 1, 142, "#8a5a2c");
  px(3, 2, 6, 6, "#f0c040");
  px(4, 1, 4, 1, "#fff0a0");
  px(3, 7, 6, 1, "#8a5a10");
  // Crossbar.
  px(4, 12, 32, 3, "#6a4018");
  px(34, 11, 3, 5, "#f0c040");
  // The cloth, a swallow tail at its foot.
  const top = 15, bottom = 92, L = 9, R = 35;
  for (let y = top; y < bottom; y++) {
    const notch = y > bottom - 14 ? Math.round(((y - (bottom - 14)) / 14) * 7) : 0;
    for (let x = L; x < R; x++) {
      const mid = (L + R) / 2;
      if (notch && Math.abs(x + 0.5 - mid) < notch) continue;
      const edge = x === L || x === R - 1 || y === top;
      const fold = (x - L) % 9 < 2 ? 0.86 : 1;
      px(x, y, 1, 1, edge ? "#e8b838" : shade(fold * (y < top + 3 ? 1.12 : 1)));
    }
  }
  // The charge: a pale four-pointed star in a ring.
  const cx = 22, cy = 44;
  const pale = "#f4eedc";
  for (let k = -9; k <= 9; k++) {
    const t = 9 - Math.abs(k);
    px(cx + k, cy - Math.floor(t / 4), 1, Math.floor(t / 2) + 1, pale);
    px(cx - Math.floor(t / 4), cy + k, Math.floor(t / 2) + 1, 1, pale);
  }
  px(cx - 2, cy - 2, 5, 5, pale);
  g.strokeStyle = "rgba(244,238,220,0.9)";
  g.lineWidth = 1;
  g.beginPath();
  g.arc(cx + 0.5, cy + 0.5, 12, 0, Math.PI * 2);
  g.stroke();
  // Gold trim across the cloth's foot.
  px(L, 70, R - L, 2, "#e8b838");
  atlas.put(key, c);
  return key;
}

/** "#rrggbb" for any CSS colour tiles.js gives (#rgb, #rrggbb, hsl()). */
function cssHex(css) {
  if (/^#[0-9a-f]{6}$/i.test(css)) return css;
  if (/^#[0-9a-f]{3}$/i.test(css)) return `#${[...css.slice(1)].map((c) => c + c).join("")}`;
  const c = (typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(1, 1) : document.createElement("canvas")).getContext("2d");
  c.fillStyle = css;
  return c.fillStyle.startsWith("#") ? c.fillStyle : "#888888";
}

/** A card record. */
function card(key, art, px, x, y, z, o = {}) {
  return { key, x, y, z, w: art.w * px, h: art.h * px, anchor: 0.5, flip: false, banner: false, shadow: false, team: [1, 1, 1], bias: 0, id: 0, glow: 0, phase: 0, ...o };
}

/** The mana school of a node or a resource, from its name (`IMPROVEMENT_MANA_FIRE`), else null. */
function manaColour(name) {
  const m = /MANA_([A-Z]+)/.exec(name ?? "");
  return m ? MANA[m[1]] ?? "#b48aff" : /MANA/.test(name ?? "") ? "#b48aff" : null;
}

/** The static cards (cities, standing improvements, resources and their banners) and the light
 *  beams over mana nodes and towers; `claims` names what the 3D dressing no longer models. */
export function dressSpriteStatic(world, map, atlas) {
  const list = [];
  const beams = [];
  const claims = { city: new Set(), improvement: new Set(), bonus: new Set() };
  const w = world.w;
  const H = (mx, my) => world.heightAt(mx, my);
  const cityAt = new Map();
  for (const c of S.cities.values()) cityAt.set(c.y * w + c.x, c);
  for (let i = 0; i < w * world.h; i++) {
    if (world.kind[i] === KIND.VOID) continue;
    const x = i % w, y = (i - x) / w;
    const at = (lx, lz) => [x + 0.5 + lx, y + 0.5 - lz];
    const city = cityAt.get(i);
    if (city) {
      const key = first(atlas, ISO_TABLES.citySprites(city.size ?? 1, civShortName(city.owner)));
      if (key) {
        // The card's foot on the plot's front edge: its painted ground then lies over the plot.
        const art = atlas.info(key);
        const [mx, my] = at(0, 0.5);
        list.push(card(key, art, PX.city, mx, H(x + 0.5, y + 0.5), -my, { id: idOf(`c${city.id}`), bias: BIAS.building, vis: [x + 0.5, y + 0.5] }));
        const css = teamCss(city.owner);
        const bk = bannerKey(atlas, css);
        const [bx, by] = at(-0.44, 0.38);
        list.push(card(bk, atlas.info(bk), PX.banner, bx, H(bx, by), -by, { anchor: 5.5 / 40, banner: true, team: hexRgb(cssHex(css)), bias: BIAS.unit, id: idOf(`cb${city.id}`), phase: hashf(city.id, 5) }));
        claims.city.add(i);
      }
    }
    const imp = map.improvement[i] >= 0 ? tname("ImprovementType", map.improvement[i]) : null;
    if (imp && !city) {
      const key = first(atlas, ISO_TABLES.improvementSprites(imp).filter((k) => k.startsWith("bld_") || k.startsWith("feat_")));
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        const [mx, my] = at(0, 0.5);
        const base = water ? SEA - DRAFT : H(x + 0.5, y + 0.5);
        const mana = manaColour(imp);
        list.push(card(key, art, PX.building, mx, base, -my, { id: idOf(`i${i}`), glow: mana ? 1.4 : 0, fade: !water, vis: [x + 0.5, y + 0.5] }));
        claims.improvement.add(i);
        const tower = /TOWER/.test(imp) && !/WATCH|WARNING/.test(imp);
        if (mana || tower) {
          const col = hexRgb(mana ?? "#ffe6a8");
          const [cx, cy] = at(0, 0.02);
          beams.push({ key: null, x: cx, y: H(cx, cy), z: -cy, w: 0.4, h: 1.5, anchor: 0.5, team: col, glow: mana ? 0.45 : 0.3, id: 0, phase: hashf(i, 9) });
        }
      }
    }
    const bonus = map.bonus[i] >= 0 ? tname("BonusType", map.bonus[i]) : null;
    if (bonus && !city) {
      const key = first(atlas, [ISO_TABLES.resourceSprite(bonus)]);
      if (key) {
        const art = atlas.info(key);
        const water = world.isWater(i);
        const [mx, my] = at(...BONUS_SPOT);
        const base = water ? SEA : H(mx, my);
        const glow = /MANA|GEMS|JEWELS|MITHRIL/.test(bonus) ? 1.2 : 0;
        list.push(card(key, art, PX.resource, mx, base, -my, { id: idOf(`b${i}`), glow, bias: BIAS.resource }));
        claims.bonus.add(i);
        if (/MANA/.test(bonus)) beams.push({ key: null, x: mx, y: base, z: -my, w: 0.26, h: 0.9, anchor: 0.5, team: hexRgb(manaColour(bonus) ?? "#b48aff"), glow: 0.3, id: 0, phase: hashf(i, 11) });
      }
    }
  }
  return { list, beams, claims };
}

/** The unit stacks in sight as groups of figures under their banners. Returns `{list, stacks}` in the
 *  shape of ../dress.js dressUnits (stacks: for the HUD's badges and the team rings). */
export function dressSpriteUnits(world, map, atlas) {
  const list = [];
  const stacks = [];
  const w = world.w;
  const mine = S.view?.me?.player;
  const sel = S.sel?.units ?? [];
  const strength = (u) => S.data?.tables?.UnitType?.[u.type]?.combat ?? 0;
  const sctx = {
    me: mine,
    selected: sel,
    unitInfo: (t) => {
      const s = strength({ type: t });
      return { hero: false, combat: s > 0 ? "COMBAT" : "NONE", strength: s };
    },
    activity: (a) => tname("ActivityType", a),
    automated: (g) => {
      const grp = S.groups?.get?.(g);
      return !!grp && grp.automated !== null && grp.automated !== undefined;
    },
  };
  const standing = new Set();
  for (let i = 0; i < map.improvement.length; i++) {
    if (map.improvement[i] < 0) continue;
    const imp = tname("ImprovementType", map.improvement[i]);
    if (ISO_TABLES.improvementSprites(imp).some((k) => (k.startsWith("bld_") || k.startsWith("feat_")) && atlas.has(k))) standing.add(i);
  }
  for (const units of S.unitsAt.values()) {
    const u0 = units[0];
    const i = u0.y * w + u0.x;
    const st = stackState(units, sctx);
    const top = st?.head ?? u0;
    const info = S.data?.tables?.UnitType?.[top.type] ?? null;
    const key = first(atlas, unitSpriteKeys(top.type, civShortName(top.owner)));
    if (!key) continue;
    const art = atlas.info(key);
    const onCity = map.city[i] >= 0;
    // A city's garrison is one figure at its front, so it does not crowd the town's walls and gate:
    // at the front-left corner, else the front-right or the front's middle, out of a river.
    const spot = onCity ? garrisonSpot(world, u0.x, u0.y) : world.feat[i] === FEAT.VOLCANO ? VOLCANO_SPOT : standing.has(i) ? BUILT_SPOT : SPOT;
    const n = onCity ? 1 : groupSize(info, art);
    const dom = info ? tname("DomainType", info.domain) : null;
    const css = teamCss(top.owner);
    const team = hexRgb(cssHex(css));
    const scale = onCity ? 0.8 : 1;
    let anchor = null;
    GROUP[n].forEach(([ox, oz], k) => {
      const lx = spot[0] + ox, lz = spot[1] + oz;
      const mx = u0.x + 0.5 + lx, my = u0.y + 0.5 - lz;
      const ground = Math.max(SEA, standHeight(world, mx, my, 0.06));
      const y = dom === "DOMAIN_SEA" ? SEA - DRAFT : dom === "DOMAIN_AIR" ? ground + 0.12 : ground;
      // The figures behind the first a touch smaller, as the 3D renderer's company.
      const s = scale * (k === 0 ? 1 : 0.94);
      const r = card(key, art, PX.unit * s, mx, y, -my, { shadow: true, team, bias: (onCity ? BIAS.garrison : BIAS.unit) + (k === 0 ? 0.02 : 0), id: idOf(`u${top.id}`), phase: hashf(top.id * 3 + k, 3) });
      r.uid = `${top.id}:${k}`;
      r.ground = dom !== "DOMAIN_SEA" && dom !== "DOMAIN_AIR";
      list.push(r);
      if (k === 0) anchor = { x: mx, y, z: -my, height: r.h * 0.85 };
    });
    // The banner behind the group's left, its cloth above the figures' heads.
    const bk = bannerKey(atlas, css);
    const bx = u0.x + 0.5 + spot[0] - (n === 1 ? Math.min(0.3, art.w * PX.unit * 0.42) : 0.26);
    const by = u0.y + 0.5 - (spot[1] - 0.11);
    const bg = Math.max(SEA, world.heightAt(bx, by));
    const b = card(bk, atlas.info(bk), PX.banner * scale, bx, dom === "DOMAIN_SEA" ? SEA : bg, -by, { anchor: 5.5 / 40, banner: true, team, bias: BIAS.unit, id: idOf(`ub${top.id}`), phase: hashf(top.id, 7) });
    b.uid = `${top.id}:b`;
    b.ground = dom !== "DOMAIN_SEA";
    list.push(b);
    stacks.push({ units, top, x: u0.x, y: u0.y, k: i, anchor, picked: sel.includes(top.id), team: css, spot, mark: st?.state?.mark ?? null, hero: st?.state?.hero ?? false });
  }
  return { list, stacks };
}
