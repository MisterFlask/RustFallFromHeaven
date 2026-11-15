// Mana deposits and their connected nodes share a sphere's colour and silhouette. These cards
// are drawn once on first use, then cached by the isometric / HD-2D atlases like other sprites.
// Symbols remain different in greyscale: a flame, mountain, wave, tree, scales, eye, skull, etc.
import { MANA } from "./r3d/assets/palette.js";

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
  return school ? `${built ? "feat" : "res"}_mana_${school.toLowerCase()}` : null;
}

export const MANA_SPRITES = Object.fromEntries(Object.keys(MANA_SPHERES).flatMap((s) => [
  [`res_mana_${s.toLowerCase()}`, { w: 128, h: 128, box: [12, 6, 116, 124] }],
  [`feat_mana_${s.toLowerCase()}`, { w: 192, h: 170, box: [12, 6, 180, 168] }],
]));

export function manaColour(type) {
  return MANA[manaSchool(type)] ?? MANA[""];
}

/** Contrasting lettering on the sphere's colour at distant zooms. */
export function manaInk(type) {
  const n = parseInt(manaColour(type).slice(1), 16);
  const light = (n >> 16) * .299 + ((n >> 8) & 255) * .587 + (n & 255) * .114;
  return light < 140 ? "#fff6e5" : "#201a28";
}

function canvas(w, h) {
  if (typeof OffscreenCanvas !== "undefined") return new OffscreenCanvas(w, h);
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return c;
}

function shade(hex, scale) {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${[n >> 16, (n >> 8) & 255, n & 255].map((c) => Math.round(c * scale)).join(",")})`;
}

/** A sphere's physical motif, in a square from -1 to 1 (not a letter or a shared crystal). */
function sigil(g, school, colour) {
  g.lineJoin = "round";
  g.lineCap = "round";
  const path = (d, fill = colour, width = 0.12) => {
    const p = new Path2D(d);
    g.strokeStyle = "#201d32";
    g.lineWidth = width + 0.18;
    g.stroke(p);
    if (fill) { g.fillStyle = fill; g.fill(p); }
    g.strokeStyle = colour;
    g.lineWidth = width;
    g.stroke(p);
  };
  const line = (d, c = "#fff0d0", width = 0.09) => {
    g.strokeStyle = c;
    g.lineWidth = width;
    g.stroke(new Path2D(d));
  };
  switch (school) {
    case "AIR":
      path("M-.95 -.4 L.2 -.4 C1 -.4 .8 -1 .4 -.85 M-.8 .05 L.6 .05 C1.3 .05 1.1 -.65 .8 -.55 M-.7 .5 L.1 .5 C.8 .5 .6 1 .2 .85", null, .18);
      break;
    case "BODY":
      path("M0 .9 C-.3 .45 -1 -.05 -.85 -.55 C-.7 -1 -.15 -.9 0 -.5 C.2 -1 .8 -1 .9 -.4 C1 .1 .35 .6 0 .9Z");
      line("M-.72 .05 L-.35 .05 L-.15 -.3 L.05 .4 L.3 -.05 L.65 -.05");
      break;
    case "CHAOS": {
      const pts = Array.from({ length: 16 }, (_, i) => {
        const a = i * Math.PI / 8 - Math.PI / 2, r = i % 2 ? .35 : (i % 4 ? .8 : 1);
        return `${i ? "L" : "M"}${Math.cos(a) * r} ${Math.sin(a) * r}`;
      }).join(" ");
      path(`${pts}Z`);
      line("M-.2 -.15 L.25 -.15 L-.1 .15 L.2 .15");
      break;
    }
    case "DEATH":
      path("M-.55 .35 C-1 .05 -.9 -.8 -.35 -.9 C.4 -1 .95 -.55 .8 .1 L.5 .4 L.5 .8 L-.5 .8Z");
      g.fillStyle = "#211c32";
      g.fillRect(-.57, -.35, .35, .35); g.fillRect(.2, -.35, .35, .35);
      line("M-.25 .4 L-.25 .76 M.05 .4 L.05 .8 M.3 .4 L.3 .76", "#211c32", .09);
      break;
    case "EARTH":
      path("M-1 .8 L-.65 -.2 L-.2 -.8 L.2 -.55 L.85 .75Z", shade(colour, .6));
      path("M-.35 .8 L.15 -.45 L.5 -.7 L1 .8Z");
      line("M-.65 -.2 L-.2 -.8 L-.1 .15 M.15 -.45 L.5 -.7 L.7 .2");
      break;
    case "ENCHANTMENT":
      path("M0 -.75 L.4 -.05 L0 .65 L-.4 -.05Z");
      path("M-.9 .25 C-1.2 -.3 .65 -.8 .95 -.35 C1.2 .1 -.6 .8 -.9 .25Z", null, .11);
      line("M-.6 -.7 L-.6 -.95 M-.72 -.82 L-.48 -.82");
      break;
    case "ENTROPY":
      path("M-.7 -.9 L.7 -.9 L.15 -.1 L.65 .8 L-.65 .8 L-.15 .15Z", shade(colour, .55));
      line("M-.6 -.7 L.5 -.7 L0 -.1 M-.45 .65 L.45 .65 L0 .1", colour, .18);
      path("M.7 .05 L1 .25 L.75 .4Z");
      break;
    case "FIRE":
      path("M0 -.95 C.2 -.25 .65 -.5 .5 -.8 C1.3 .05 .8 .9 0 .95 C-.9 1 -1.2 .15 -.55 -.6 C-.65 .05 -.2 .1 0 -.95Z");
      path("M0 -.2 C.1 .15 .5 .3 .3 .7 C.1 1 -.5 .7 -.3 .3Z", "#ffe39a", .06);
      break;
    case "ICE":
      for (let i = 0; i < 3; i++) {
        g.save(); g.rotate(i * Math.PI / 3);
        path("M0 -1 L0 1 M-.25 -.75 L0 -.5 L.25 -.75 M-.25 .75 L0 .5 L.25 .75", null, .13);
        g.restore();
      }
      break;
    case "LAW":
      path("M0 -.95 L0 .85 M-.7 .85 L.7 .85 M-.85 -.5 L.85 -.5", null, .13);
      path("M-.85 -.45 L-1 .25 L-.3 .25 L-.5 -.45 M.5 -.45 L.3 .25 L1 .25 L.85 -.45", null, .08);
      path("M-1 .25 Q-.65 .8 -.3 .25Z"); path("M.3 .25 Q.65 .8 1 .25Z");
      break;
    case "LIFE":
      path("M0 -.1 C-.8 -.1 -.65 -.95 0 -.95 C.65 -.95 .8 -.1 0 -.1Z", null, .18);
      path("M0 -.1 L0 1 M-.65 .25 L.65 .25", null, .23);
      break;
    case "METAMAGIC":
      path("M0 -1 L1 0 L0 1 L-1 0Z", null, .12);
      path("M0 -.5 L.5 0 L0 .5 L-.5 0Z", null, .12);
      line("M-.2 0 L.2 0 M0 -.2 L0 .2");
      break;
    case "MIND":
      path("M-1 0 Q0 -1.1 1 0 Q0 1.1 -1 0Z");
      path("M0 -.46 C.6 -.46 .6 .46 0 .46 C-.6 .46 -.6 -.46 0 -.46Z", "#433052", .07);
      line("M-.08 -.25 L.08 -.25", "#fff0d0", .17);
      break;
    case "NATURE":
      path("M0 1 L0 -.7 M0 .1 L-.6 -.45 M0 .45 L.65 -.15", null, .18);
      path("M0 -.5 Q-.7 -.8 -.15 -1 Q.5 -.9 0 -.5Z");
      path("M-.25 0 Q-1 .1 -.9 -.6 Q-.15 -.7 -.25 0Z");
      path("M.2 .3 Q.2 -.6 .95 -.45 Q1 .25 .2 .3Z");
      break;
    case "SHADOW":
      path("M.5 -.95 C-.9 -1 -1.15 .9 .4 1 C-.25 .6 -.25 -.65 .5 -.95Z", shade(colour, .65));
      path("M.6 -.15 L.9 .1 L.6 .4 L.35 .1Z", "#b5a1ed", .06);
      break;
    case "SPIRIT":
      path("M-.6 .9 C-.3 .4 -.95 .15 -.65 -.55 C-.35 -1 .5 -1 .65 -.4 C.9 .45 .2 .8 .5 1 L.05 .75 L-.2 1 L-.35 .7Z");
      line("M-.25 -.3 L-.25 -.1 M.25 -.3 L.25 -.1", "#584773", .13);
      break;
    case "SUN":
      for (let i = 0; i < 8; i++) {
        g.save(); g.rotate(i * Math.PI / 4);
        path("M0 -.7 L0 -1.02", null, .15); g.restore();
      }
      path("M.55 0 A.55 .55 0 1 1 -.55 0 A.55 .55 0 1 1 .55 0Z", "#ffe9a0");
      break;
    case "WATER":
      path("M0 -1 C-.2 -.4 -.95 .1 -.75 .55 C-.4 1.2 .55 1.15 .8 .55 C1 .1 .2 -.5 0 -1Z");
      line("M-.45 .35 C-.5 .9 .45 .85 .5 .4 M-.25 .05 L-.4 .25");
      break;
  }
}

/** Original pixel-art card. No source image is recoloured; geometry identifies each sphere. */
export function drawManaSprite(key) {
  const meta = MANA_SPRITES[key];
  if (!meta) return null;
  const built = key.startsWith("feat_");
  const school = key.slice(key.lastIndexOf("_") + 1).toUpperCase();
  const colour = MANA[school];
  const c = canvas(meta.w, meta.h), small = canvas(meta.w / 2, meta.h / 2);
  const g = small.getContext("2d", { willReadFrequently: true });
  const cx = small.width / 2, floor = small.height - 4;
  const polygon = (pts, fill, stroke = "#292635") => {
    g.beginPath(); pts.forEach(([x, y], i) => i ? g.lineTo(x, y) : g.moveTo(x, y)); g.closePath();
    g.fillStyle = fill; g.fill(); g.strokeStyle = stroke; g.lineWidth = 1; g.stroke();
  };
  // A rocky spring in the wild; once connected, a carved dais and four standing stones.
  g.fillStyle = "#201e2d66";
  g.beginPath(); g.ellipse(cx, floor - 1, built ? 40 : 25, built ? 9 : 5, 0, 0, Math.PI * 2); g.fill();
  if (built) {
    polygon([[cx - 34, floor - 7], [cx, floor - 18], [cx + 34, floor - 7], [cx, floor + 1]], "#514c60");
    polygon([[cx - 34, floor - 7], [cx, floor + 1], [cx, floor - 3], [cx - 34, floor - 11]], "#383344");
    for (const [x, y, h] of [[-30, -16, 27], [29, -17, 31], [-24, -2, 20], [25, -3, 18]]) {
      polygon([[cx + x - 4, floor + y], [cx + x - 4, floor + y - h], [cx + x + 2, floor + y - h - 3], [cx + x + 5, floor + y - 1]], "#8b8091");
      g.fillStyle = colour; g.fillRect(cx + x - 1, floor + y - h + 5, 2, 7);
    }
    g.strokeStyle = colour; g.lineWidth = 1;
    g.beginPath(); g.ellipse(cx, floor - 9, 23, 7, 0, 0, Math.PI * 2); g.stroke();
  } else {
    for (const [x, y, w, h] of [[-17, -3, 7, 10], [13, -2, 8, 13], [-5, 0, 12, 8]]) {
      polygon([[cx + x - w, floor + y], [cx + x - w + 3, floor + y - h], [cx + x, floor + y - h - 2], [cx + x + w, floor + y - 3]], shade(colour, .45));
      g.fillStyle = shade(colour, .8); g.fillRect(cx + x - 2, floor + y - h + 1, 3, 4);
    }
  }
  g.save();
  g.translate(cx, built ? 29 : 25); g.scale(built ? 22 : 19, built ? 22 : 19);
  sigil(g, school, colour);
  g.restore();
  const out = c.getContext("2d", { willReadFrequently: true });
  out.imageSmoothingEnabled = false;
  out.drawImage(small, 0, 0, meta.w, meta.h);
  return c;
}
