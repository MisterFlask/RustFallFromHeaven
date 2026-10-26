// The isometric map's interface (docs/ISO.md, "The interface"): the client's chrome redrawn in the
// look of the reference screenshot (tools/m7/ref/reference_full.png) whenever the isometric renderer
// draws the map. The map fills the page and the interface floats over it in gold-framed navy panels:
// a single bar at the top (the date and turn, the empire's numbers, the system buttons), a rail of
// advisor buttons at the left, the selection's card at the bottom left (a portrait, its numbers, its
// orders as icon tiles over a description of the one pointed at), a city panel at the right and the
// minimap with its layer buttons and End turn at the bottom right.
//
// Its parts: the lettering is Herald (isofont.js); the icons are the M7 package's interface icons
// (`ui_<name>`, drawn by the image model on the reference's panels, tools/m7/ui.py) and, for the
// worker's builds and the city's picture, the map's own sprites; the frames are drawn here, as
// nine-slices on the reference's 2 px grid (`--hu`), and the panels' navy carries a faint pixel
// grain. iso.css holds the layout and the states, all under `html.iso`.
//
// The skin follows the renderer (`setIsoSkin`, main.js): the same elements and the same screens,
// rearranged and restyled while the isometric map is on, as they were when it goes.

import { S, me, dname, tname, turns, orderName, emit, isBarbarian } from "./state.js";
import * as T from "./tiles.js";
import { h, fill, COMPACT, compactScreen } from "./dom.js";
import { installIsoFont } from "./isofont.js";
import { SPRITES } from "./iso/sprites.js";
import { ISO_TABLES } from "./iso.js";
import * as api from "./api.js";
import { infoTip } from "./infotips.js";

/** Whether the skin is on, and the MapView it dresses. */
export const skin = { on: false, view: null };

/** Whether the isometric interface is on. */
export function isoSkin() {
  return skin.on;
}

const spriteUrl = (key) => new URL(`./iso/${key}.png`, import.meta.url).href;

// ---------------------------------------------------------------------------------------------
// Icons

/** An interface icon of the M7 set: `ui_<name>` (48 art px, the action tiles) or `ui_<name>_s`
 *  (32, the bar and the lists) drawn at `px` CSS px; null when there is no such icon. The icons are
 *  re-gridded to 2 px blocks, so 16, 24, 32 and 48 px keep them crisp. */
export function isoIcon(name, { px = 32, title, cls } = {}) {
  const key = px > 32 ? `ui_${name}` : `ui_${name}_s`;
  if (!Object.hasOwn(SPRITES, key)) return null;
  const im = document.createElement("img");
  im.src = spriteUrl(key);
  im.width = px;
  im.height = px;
  im.alt = "";
  im.draggable = false;
  im.className = `iso-icon${cls ? ` ${cls}` : ""}`;
  if (title) im.title = title;
  return im;
}

/** A map sprite (a building, an overlay, a unit) fitted by its opaque box into `px` CSS px, as a
 *  span with the sprite as its background; null when there is no such sprite. */
export function spriteIcon(key, px, { title, cls, flip = false } = {}) {
  const s = SPRITES[key];
  if (!s) return null;
  const [x0, y0, x1, y1] = s.box;
  const k = px / Math.max(x1 - x0, y1 - y0);
  const el = document.createElement("span");
  el.className = `iso-sprite${cls ? ` ${cls}` : ""}`;
  el.style.cssText = `width:${Math.round((x1 - x0) * k)}px;height:${Math.round((y1 - y0) * k)}px;background-image:url(${spriteUrl(key)});background-size:${s.w * k}px ${s.h * k}px;background-position:${-x0 * k}px ${-y0 * k}px;${flip ? "transform:scaleX(-1);" : ""}`;
  if (title) el.title = title;
  el.setAttribute("aria-hidden", "true");
  return el;
}

/** The 2D art's icon names (look.js callers: `icon16/gold`, `icon8/moves`, ...) -> the M7 set. */
const ART_ICONS = {
  gold: "gold", commerce: "commerce", trade: "commerce", food: "food", production: "production", research: "research",
  research_needed: "research", culture: "culture", espionage: "sentry", happy: "happy", unhappy: "unhappy", angrypop: "unhappy",
  healthy: "health", unhealthy: "unhealthy", greatpeople: "greatpeople", strength: "strength", moves: "moves", defense: "defense",
  mana: "mana", golden_age: "goldenage", anarchy: "cancel", stasis: "endturn", ai_idle: "automate", turn: "endturn", turn_wait: "endturn",
};

/** The M7 icon standing for a 2D art icon (`icon16/<x>`, 32 px; `icon8/<x>`, 16 px inline), or null. */
export function isoIconFor(artName, { title, cls } = {}) {
  const m = /^icon(8|16)\/(.+)$/.exec(artName ?? "");
  if (!m) return null;
  let n = m[2];
  if (n.startsWith("armageddon_")) n = "armageddon";
  else if (n.startsWith("sphere_")) n = "mana";
  const name = ART_ICONS[n] ?? (Object.hasOwn(SPRITES, `ui_${n}_s`) ? n : null);
  return name ? isoIcon(name, { px: m[1] === "8" ? 16 : 32, title, cls }) : null;
}

/** A progress bar of the skin (the reference's population bar): a gold-framed black track with a
 *  bright fill, `w` CSS px wide; `next` is the part expected next turn. */
export function isoBar(frac, { w = 120, next = null, kind = "", title } = {}) {
  const f = Math.max(0, Math.min(1, Number(frac) || 0));
  const bar = h(`span.iso-bar${kind ? `.${kind}` : ""}`, { style: { width: `${w}px` }, title: title ?? null });
  if (next !== null && next > f) bar.append(h("span.next", { style: { width: `${Math.round(100 * Math.min(1, next))}%` } }));
  bar.append(h("span.fill", { style: { width: `${Math.round(100 * f)}%` } }));
  return bar;
}

// --- the orders' icons ------------------------------------------------------------------------

const MISSION_ICONS = {
  MISSION_SKIP: "skip", MISSION_SLEEP: "sleep", MISSION_FORTIFY: "fortify", MISSION_HEAL: "heal", MISSION_SENTRY: "sentry",
  MISSION_PLUNDER: "pillage", MISSION_PILLAGE: "pillage", MISSION_SABOTAGE: "pillage", MISSION_DESTROY: "pillage",
  MISSION_STEAL_PLANS: "sentry", MISSION_ESPIONAGE: "sentry", MISSION_INFILTRATE: "sentry", MISSION_RECON: "explore",
  MISSION_RANGE_ATTACK: "rangeattack", MISSION_BOMBARD: "bombard", MISSION_AIRBOMB: "bombard", MISSION_AIRSTRIKE: "bombard",
  MISSION_FOUND: "found", MISSION_SPREAD: "spread", MISSION_SPREAD_CORPORATION: "spread", MISSION_JOIN: "population",
  MISSION_CONSTRUCT: "cities", MISSION_DISCOVER: "research", MISSION_HURRY: "production", MISSION_TRADE: "commerce",
  MISSION_GREAT_WORK: "greatwork", MISSION_GOLDEN_AGE: "goldenage", MISSION_LEAD: "promote", MISSION_AIRLIFT: "goto",
  MISSION_PARADROP: "goto", MISSION_MOVE_TO: "goto", MISSION_ROUTE_TO: "road", MISSION_AIRPATROL: "sentry", MISSION_SEAPATROL: "sentry",
};
const MODE_ICONS = {
  INTERFACEMODE_GO_TO: "goto", INTERFACEMODE_GO_TO_TYPE: "goto", INTERFACEMODE_GO_TO_ALL: "goto", INTERFACEMODE_ROUTE_TO: "road",
  INTERFACEMODE_AIRLIFT: "goto", INTERFACEMODE_RECON: "explore", INTERFACEMODE_PARADROP: "goto", INTERFACEMODE_AIRBOMB: "bombard",
  INTERFACEMODE_RANGE_ATTACK: "rangeattack", INTERFACEMODE_AIRSTRIKE: "bombard", INTERFACEMODE_REBASE: "goto",
};
const COMMAND_ICONS = {
  COMMAND_PROMOTION: "promote", COMMAND_UPGRADE: "upgrade", COMMAND_AUTOMATE: "automate", COMMAND_AUTOMATE_SPELL: "spell",
  COMMAND_WAKE: "goldenage", COMMAND_CANCEL: "cancel", COMMAND_CANCEL_ALL: "cancel", COMMAND_STOP_AUTOMATION: "cancel",
  COMMAND_DELETE: "disband", COMMAND_GIFT: "diplomacy", COMMAND_LOAD: "board", COMMAND_LOAD_UNIT: "board",
  COMMAND_UNLOAD: "board", COMMAND_UNLOAD_ALL: "board", COMMAND_CAST: "spell",
};
const AUTOMATE_ICONS = { AUTOMATE_EXPLORE: "explore", AUTOMATE_RELIGION: "spread" };

/** The sprite of a worker's build: the map's own improvement (a farm, a mine ...), a road, or the
 *  feature it clears. */
function buildSprite(b) {
  if (b?.route !== null && b?.route !== undefined) return { icon: "road" };
  const imp = b?.improvement !== null && b?.improvement !== undefined ? tname("ImprovementType", b.improvement) : null;
  if (imp) {
    const key = ISO_TABLES.improvementSprites(imp).find((k) => Object.hasOwn(SPRITES, k));
    if (key) return { sprite: key };
  }
  const t = b?.type ?? "";
  if (/JUNGLE/.test(t)) return { sprite: "feat_jungle", cancel: true };
  if (/FOREST/.test(t)) return { sprite: "feat_forest", cancel: true };
  if (/SCRUB|FALLOUT/.test(t)) return { sprite: "feat_scrub", cancel: true };
  return { icon: "production" };
}

/** An order's icon for its tile (`px` CSS px): an M7 interface icon or, for a build, the map
 *  sprite of what it builds. `a` is a `group` query action. */
export function actionIcon(a, px = 40) {
  let name = null;
  if (a.kind === "mission") {
    const m = tname("MissionType", a.mission);
    if (m === "MISSION_BUILD") {
      const b = buildSprite(S.data?.tables?.BuildType?.[a.data1]);
      if (b.sprite) {
        const box = h("span.iso-build", spriteIcon(b.sprite, px));
        if (b.cancel) box.append(isoIcon("cancel", { px: 16, cls: "badge" }));
        return box;
      }
      name = b.icon;
    } else name = MISSION_ICONS[m] ?? "goto";
  } else if (a.kind === "target") name = MODE_ICONS[a.mode] ?? "goto";
  else if (a.kind === "command" || a.kind === "load") {
    const c = tname("CommandType", a.command);
    // Cancelling every order and stopping the automation each carry the cancel mark on another icon.
    if (c === "COMMAND_CANCEL_ALL") return h("span.iso-build", isoIcon("goto", { px: 32 }), isoIcon("cancel", { px: 16, cls: "badge" }));
    if (c === "COMMAND_STOP_AUTOMATION") return h("span.iso-build", isoIcon("automate", { px: 32 }), isoIcon("cancel", { px: 16, cls: "badge" }));
    name = COMMAND_ICONS[c] ?? (a.kind === "load" ? "board" : "cancel");
  }
  else if (a.kind === "automate") name = AUTOMATE_ICONS[tname("AutomateType", a.automate)] ?? "automate";
  else if (a.kind === "spell") name = "spell";
  else if (a.kind === "promotion") name = "promote";
  else if (a.kind === "upgrade") {
    const key = skin.view?.iso?.unitKey(a.unit, me()?.player);
    if (key && Object.hasOwn(SPRITES, key)) return h("span.iso-build", spriteIcon(key, px), isoIcon("upgrade", { px: 16, cls: "badge" }));
    name = "upgrade";
  }
  return isoIcon(name ?? "goto", { px: px > 32 ? 48 : 32 });
}

/** The selection's portrait: the lead unit's map figure on the painted meadow (`ui_backdrop`), in
 *  a gold frame; `flip` faces it left. */
export function portrait(type, owner) {
  const key = skin.view?.iso?.unitKey(type, owner);
  const fig = key ? spriteIcon(key, 96, { cls: "figure" }) : null;
  return h("div.iso-portrait", { tip: infoTip("UnitType", type) }, fig);
}

// ---------------------------------------------------------------------------------------------
// Frames: nine-slices drawn on the 2 px grid

const C = {
  k: "#0c0703", // the outline
  L: "#ffe58c", // gold, lit
  G: "#f0b443", // gold
  D: "#a86d1c", // gold in shade
  B: "#5a3a10", // bronze, deep
  Y: "#fff7d2", // a glint
  M: "#7a6a4c", // a dulled rule (disabled)
  m: "#4a4030",
  E: "#c84a38", // crimson (danger)
  e: "#6a1a14",
};

/** The frames: `band` the rule's colours from its upper or left side to its lower or right side
 *  (every edge the same way, so the frame reads as a gilt bar lit from the upper left), `nail` a
 *  corner ornament (rows of colour keys, centred on the corner), `out` the art px the frame reaches
 *  outside its box (for the nails), `in` the art px its nails reach inside. */
const NAIL = [
  ".kkkkkkk.",
  "kLLLLLGDk",
  "kLGGGGGDk",
  "kLGkkkGDk",
  "kLGkYkGDk",
  "kLGkkkGDk",
  "kLGGGGGDk",
  "kGDDDDDDk",
  ".kkkkkkk.",
];
const STUD = [".kkk.", "kLLGk", "kLYDk", "kGDDk", ".kkk."];
/** The panels' corner: the reference's gilt knot (cut from its city panel's corner on the 2 px grid
 *  and cleaned), anchored at the rule's outer corner and mirrored at the others. ' ' cuts the
 *  rule away (the corner is open), '.' keeps it, the rest are colour keys. */
const KNOT = [
  "     kDDkk....",
  "    DGLLGD....",
  "   kDLGGD.....",
  "  kkDLGDkk....",
  " kkDGLGkkk....",
  "kGLLGGGGGkGLGk",
  "kGLGGDkLLDkDkk",
  "kGLGkDDGLDk...",
  "kGLDkGLLDkk...",
  "kDGDkDDDkk....",
  "kDDDkDkkk.....",
  ".....LDk......",
  ".....GD.......",
  "..............",
];
const FRAMES = {
  panel: { band: "kLGDk", knot: KNOT, out: 0, in: 9 },
  inset: { band: "kDk", nail: null, out: 0, in: 0 },
  tile: { band: "kGDk", nail: null, out: 0, in: 0 },
  "tile-hover": { band: "kLGk", nail: null, out: 0, in: 0 },
  "tile-down": { band: "kDBk", nail: null, out: 0, in: 0 },
  "tile-off": { band: "kMmk", nail: null, out: 0, in: 0 },
  "tile-danger": { band: "kEek", nail: null, out: 0, in: 0 },
  primary: { band: "kLGDk", nail: STUD, out: 1, in: 1 },
  bar: { band: "kGk", nail: null, out: 0, in: 0 },
};

/** Draws frame `f` at `k` device px per art px: { canvas, slice (art px), band (art px) }. */
function drawFrame(f, k) {
  const n = f.band.length;
  const nail = f.nail ?? [];
  const slice = f.out + n + f.in;
  const size = slice * 2 + 2;
  const c = document.createElement("canvas");
  c.width = size * k;
  c.height = size * k;
  const g = c.getContext("2d");
  const px = (x, y, key) => {
    const col = C[key];
    if (!col) return;
    g.fillStyle = col;
    g.fillRect(x * k, y * k, k, k);
  };
  const o = f.out;
  const lo = o;
  const hi = size - o - n; // the far band's first row / column
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const inH = (y >= lo && y < lo + n) || (y >= hi && y < hi + n);
      const inV = (x >= lo && x < lo + n) || (x >= hi && x < hi + n);
      const within = x >= lo && x < hi + n && y >= lo && y < hi + n;
      if (!within || (!inH && !inV)) continue;
      const i = y >= hi ? y - hi : y - lo; // the band row, top to bottom
      const j = x >= hi ? x - hi : x - lo; // the band column, left to right
      if (inH && !inV) px(x, y, f.band[i]);
      else if (inV && !inH) px(x, y, f.band[j]);
      else {
        // A corner square: mitred, each pixel from the edge it is nearer.
        const dh = y >= hi ? n - 1 - i : i;
        const dv = x >= hi ? n - 1 - j : j;
        px(x, y, f.band[dh <= dv ? i : j]);
      }
    }
  }
  if (nail.length) {
    const s = nail.length;
    for (const [cx, cy] of [[lo, lo], [hi, lo], [lo, hi], [hi, hi]]) {
      const x0 = cx + Math.floor((n - s) / 2);
      const y0 = cy + Math.floor((n - s) / 2);
      nail.forEach((row, dy) => [...row].forEach((key, dx) => px(x0 + dx, y0 + dy, key)));
    }
  }
  if (f.knot) {
    for (const [right, bottom] of [[false, false], [true, false], [false, true], [true, true]]) {
      f.knot.forEach((row, dy) =>
        [...row].forEach((key, dx) => {
          const x = right ? hi + n - 1 - dx : lo + dx;
          const y = bottom ? hi + n - 1 - dy : lo + dy;
          if (key === " ") g.clearRect(x * k, y * k, k, k);
          else px(x, y, key);
        }),
      );
    }
  }
  return { canvas: c, slice, band: n };
}

/** The panels' navy with a faint grain: a tile of 2 px blocks a few shades apart (seeded, so
 *  every page draws the same grain). */
function grain(k) {
  const n = 48;
  const c = document.createElement("canvas");
  c.width = n * k;
  c.height = n * k;
  const g = c.getContext("2d");
  let seed = 0x2f6e2b1;
  const rnd = () => ((seed = (Math.imul(seed, 1103515245) + 12345) >>> 0) / 2 ** 32);
  for (let y = 0; y < n; y += 2) {
    for (let x = 0; x < n; x += 2) {
      const r = rnd();
      if (r < 0.62) continue;
      g.fillStyle = r < 0.8 ? "rgba(90,140,255,0.025)" : r < 0.93 ? "rgba(0,0,10,0.08)" : "rgba(140,180,255,0.035)";
      g.fillRect(x * k, y * k, 2 * k, 2 * k);
    }
  }
  return c;
}

let framesDpr = 0;
/** Defines, for each frame `x`, `--iso-x` (a border-image at the 2 px grid, pre-scaled to the
 *  device pixels) and `--iso-x-w` (its band, the box's border width); and `--iso-grain`. */
export function installFrames() {
  const dpr = Math.max(1, Math.round(Math.min(globalThis.devicePixelRatio || 1, 3)));
  if (dpr === framesDpr) return;
  framesDpr = dpr;
  const hu = 2;
  const k = hu * dpr;
  const lines = [];
  for (const [name, f] of Object.entries(FRAMES)) {
    const d = drawFrame(f, k);
    lines.push(`  --iso-${name}: url(${d.canvas.toDataURL()}) ${d.slice * k} / ${d.slice * hu}px / ${f.out * hu}px stretch;`);
    lines.push(`  --iso-${name}-w: ${d.band * hu}px;`);
  }
  lines.push(`  --iso-grain: url(${grain(dpr).toDataURL()});`);
  lines.push(`  --iso-grain-size: 96px 96px;`);
  let st = document.getElementById("iso-frames");
  if (!st) {
    st = document.createElement("style");
    st.id = "iso-frames";
    document.head.append(st);
  }
  st.textContent = `:root {\n${lines.join("\n")}\n}\n`;
}

// ---------------------------------------------------------------------------------------------
// The skin on and off: the page's elements rearranged

/** The advisor buttons the left rail takes (the reference's map, helmet and castle), with their
 *  icons; the rest of the menu stays in the top bar as icon buttons. */
const RAIL = [
  ["btn-research", "research"],
  ["btn-techs", "techs"],
  ["btn-cities", "cities"],
  ["btn-civics", "civics"],
  ["btn-religion", "religion"],
  ["btn-diplo", "diplomacy"],
];
const SYSTEM = [
  ["btn-tools", "settings"],
  ["btn-save", "save"],
  ["btn-load", "load"],
  ["btn-new", "newgame"],
  ["btn-keys", "help"],
  ["btn-raw", "pedia"],
  ["btn-title", "map"],
];
const LAYERS = { terrain: "terrain", political: "political", yield: "yields" };

/** Where each moved element came from, to put it back when the skin goes. */
const homes = new Map();

function moveTo(el, parent) {
  if (!el || !parent || el.parentElement === parent) return;
  if (!homes.has(el)) homes.set(el, { parent: el.parentElement, next: el.nextSibling });
  parent.append(el);
}

function restore() {
  for (const [el, { parent, next }] of [...homes].reverse()) {
    if (next && next.parentElement === parent) parent.insertBefore(el, next);
    else parent.append(el);
  }
  homes.clear();
}

/** Gives a static button its icon (kept, hidden by CSS, when the skin is off). */
function dress(el, icon, px) {
  if (!el || el.querySelector(":scope > .iso-only")) return;
  const im = isoIcon(icon, { px, cls: "iso-only" });
  if (im) el.prepend(im);
}

/** The panels' reach into the map (MapView.insets): the top bar, the rail, the right-hand
 *  column, and the selection's card (at least its usual height, so a unit selected under where
 *  the card is about to appear is brought clear of it). */
function updateInsets() {
  const v = skin.view;
  if (!v) return;
  if (!skin.on) {
    v.insets = { top: 0, right: 0, bottom: 0, left: 0 };
    return;
  }
  const side = document.getElementById("side");
  // getClientRects: a fixed element has no offsetParent even when shown.
  const shown = (el) => (el && el.getClientRects().length ? el.getBoundingClientRect() : null);
  const card = shown(side)?.height ?? 0;
  const top = document.getElementById("top")?.getBoundingClientRect().bottom ?? 0;
  if (compactScreen()) {
    // A phone (iso.css): the bars span the width; the card sits on the bottom bar (upright) or is
    // a column at the right (on its side). As on a desktop, the card counts with at least the room
    // it usually takes, so a unit selected where it is about to appear is brought clear of it.
    const vh = window.innerHeight;
    const below = Math.max(top, shown(document.getElementById("bottom"))?.bottom ?? 0);
    const bar = vh - (shown(document.getElementById("iso-corner"))?.top ?? vh);
    const upright = globalThis.matchMedia?.("(max-width: 760px)").matches ?? true;
    v.insets = upright
      ? { top: Math.ceil(below) + 4, left: 0, right: 0, bottom: Math.ceil(bar + Math.max(card, Math.min(0.4 * vh, 320))) + 8 }
      : { top: Math.ceil(below) + 4, left: 0, right: 352, bottom: Math.ceil(bar) + 8 };
  } else {
    v.insets = { top: Math.ceil(top) + 8, left: 70, right: 332, bottom: Math.ceil(Math.max(card, 240)) + 20 };
  }
  // The city panel ends above the bottom-right cluster (its minimap keeps the map's proportions).
  const corner = document.getElementById("iso-corner");
  if (corner) document.documentElement.style.setProperty("--iso-corner-h", `${Math.ceil(corner.getBoundingClientRect().height)}px`);
}

function ensure(id, parent, tag = "div") {
  let el = document.getElementById(id);
  if (!el) {
    el = document.createElement(tag);
    el.id = id;
    parent.append(el);
  }
  return el;
}

/** The advisors' buttons: on the rail at the left, or on a phone (dom.js `compactScreen`) first
 *  in the menu, which folds into its ☰ button there (iso.css), so no rail covers the map. */
function placeRail() {
  const rail = document.getElementById("iso-rail");
  const menu = document.getElementById("menu");
  const buttons = RAIL.map(([id]) => document.getElementById(id)).filter(Boolean);
  // Their homes in order (`restore` puts them back last first, each before the next).
  for (const b of buttons) if (!homes.has(b)) homes.set(b, { parent: b.parentElement, next: b.nextSibling });
  (compactScreen() ? menu : rail)?.prepend(...buttons);
}

/** Turns the skin on or off for MapView `view` (main.js, on every renderer change). */
export function setIsoSkin(on, view) {
  on = !!on;
  skin.view = view ?? skin.view;
  if (on === skin.on) return false;
  skin.on = on;
  const root = document.documentElement;
  root.classList.toggle("iso", on);
  if (!on) {
    restore();
    // Take back what the skin added: the static buttons' icons (the 2D art letters a button with
    // one text child only), the folded report, the city panel.
    for (const im of document.querySelectorAll("button > img.iso-only")) im.remove();
    document.body.classList.toggle("bottom-closed", !!skin.bottomWasClosed);
    const city = document.getElementById("iso-city");
    if (city) {
      city.hidden = true;
      city.replaceChildren();
    }
    cityKey = null;
    updateInsets();
    return true;
  }
  installIsoFont().then((ok) => ok && root.classList.add("iso-font"));
  installFrames();
  const mapwrap = document.getElementById("mapwrap");
  const rail = ensure("iso-rail", mapwrap, "nav");
  rail.setAttribute("aria-label", "Advisors");
  const corner = ensure("iso-corner", mapwrap);
  ensure("iso-city", mapwrap, "section").setAttribute("aria-label", "City");
  for (const [id, icon] of RAIL) dress(document.getElementById(id), icon, 32);
  placeRail();
  // A phone turned (or a window resized) across the small-screen layout's edge.
  if (!skin.compactWatch && globalThis.matchMedia) {
    skin.compactWatch = globalThis.matchMedia(COMPACT);
    skin.compactWatch.addEventListener?.("change", () => {
      if (!skin.on) return;
      placeRail();
      updateInsets();
    });
  }
  for (const [id, icon] of SYSTEM) dress(document.getElementById(id), icon, 32);
  const end = document.getElementById("end-turn");
  moveTo(document.querySelector("#mapwrap canvas.minimap"), corner);
  moveTo(document.querySelector("#mapwrap .layers"), corner);
  moveTo(document.querySelector("#mapwrap .zoom"), corner);
  moveTo(end, corner);
  for (const b of document.querySelectorAll("#mapwrap .layers button")) dress(b, LAYERS[b.dataset.layer] ?? "terrain", 32);
  dress(document.getElementById("zoom-in"), "zoomin", 32);
  dress(document.getElementById("zoom-out"), "zoomout", 32);
  dress(document.querySelector('#bottom-tabs button[data-tab="report"]'), "messages", 16);
  // The panels under the top bar follow its height (it wraps onto a second row when narrow).
  if (!skin.topWatch && typeof ResizeObserver !== "undefined") {
    const top = document.getElementById("top");
    // --iso-top: the bar's bottom (the right column, under the system buttons); --iso-top1: the
    // bottom of the parts that start at its left edge (the left column, beside a wrapped second
    // row): the first row's, or the second's when a wide first row (the phase with the spectator
    // bar) puts the empire's numbers under it.
    const phase = document.getElementById("phase");
    skin.topWatch = new ResizeObserver(() => {
      if (!skin.on) return;
      const left = top.getBoundingClientRect().left;
      let first = phase.getBoundingClientRect().bottom;
      for (const el of top.children) {
        const r = el.getBoundingClientRect();
        if (r.height > 0 && r.left < left + 80) first = Math.max(first, r.bottom);
      }
      root.style.setProperty("--iso-top", `${Math.ceil(top.getBoundingClientRect().bottom)}px`);
      root.style.setProperty("--iso-top1", `${Math.ceil(first)}px`);
    });
    skin.topWatch.observe(top);
  }
  // The map aims at the part of the view the panels leave clear (MapView.insets).
  if (!skin.insetWatch && typeof ResizeObserver !== "undefined") {
    skin.insetWatch = new ResizeObserver(() => updateInsets());
    for (const id of ["top", "side", "iso-city", "iso-corner"]) {
      const el = document.getElementById(id);
      if (el) skin.insetWatch.observe(el);
    }
  }
  updateInsets();
  // The turn report floats at the top left, folded to its tabs until one is clicked (as it was is
  // put back when the skin goes).
  skin.bottomWasClosed = document.body.classList.contains("bottom-closed");
  document.body.classList.add("bottom-closed");
  if (!skin.folded) {
    skin.folded = true;
    for (const b of document.querySelectorAll("#bottom-tabs button[data-tab]")) {
      b.addEventListener("click", () => skin.on && document.body.classList.remove("bottom-closed"));
    }
  }
  return true;
}

// ---------------------------------------------------------------------------------------------
// The city panel (the reference's right-hand panel)

let focusCity = null;
let cityToken = 0;
let cityKey = null;

/** The city the panel shows: the one last pointed at (`focusOn`), else the capital. */
export function focusOn(id) {
  if (id === focusCity) return;
  focusCity = id;
  renderCityPanel();
}

/** Reads the panel's city again (its screen changed it: worked plots, specialists, the queue). */
export function refreshCityPanel() {
  cityKey = null;
  renderCityPanel();
}

function panelCity() {
  const mine = me()?.player;
  const own = [...(S.cities?.values?.() ?? [])].filter((c) => c.owner === mine);
  return own.find((c) => c.id === focusCity) ?? own.find((c) => c.capital) ?? own[0] ?? null;
}

/** The city's picture: the settlement of its size, in its owner's colour, before the meadow. */
function cityPicture(city) {
  const keys = ISO_TABLES.citySprites(city.size ?? 1).filter((k) => Object.hasOwn(SPRITES, k));
  const key = keys[0];
  const box = h("div.iso-picture");
  if (!key) return box;
  const store = skin.view?.iso?.sprites;
  const id = store ? store.owned(key, T.playerColor(city.owner, isBarbarian(city.owner))) : key;
  const src = store?.source(id);
  if (src && id !== key) {
    const s = SPRITES[key];
    const [x0, y0, x1, y1] = s.box;
    const k = Math.min(232 / (x1 - x0), 100 / (y1 - y0));
    const c = document.createElement("canvas");
    const dpr = Math.max(1, Math.round(globalThis.devicePixelRatio || 1));
    c.width = Math.round((x1 - x0) * k * dpr);
    c.height = Math.round((y1 - y0) * k * dpr);
    c.style.width = `${Math.round((x1 - x0) * k)}px`;
    c.style.height = `${Math.round((y1 - y0) * k)}px`;
    c.className = "iso-figure";
    const g = c.getContext("2d");
    g.imageSmoothingQuality = "high";
    g.drawImage(src, x0, y0, x1 - x0, y1 - y0, 0, 0, c.width, c.height);
    box.append(c);
  } else box.append(spriteIcon(key, 100, { cls: "iso-figure" }));
  return box;
}

const signed = (n) => `${n >= 0 ? "+" : ""}${n}`;

/** Redraws the city panel (the city may have changed, or the turn). */
export async function renderCityPanel() {
  const el = document.getElementById("iso-city");
  if (!el || !skin.on) return;
  const city = panelCity();
  if (!city || !S.view) {
    cityKey = null;
    el.hidden = true;
    fill(el);
    return;
  }
  // Not every command's view (the server counts each): the turn and what the view says of the city.
  const key = `${city.id}|${S.view.turn}|${city.size}|${city.production?.order}|${city.production?.type}|${city.production?.turns}|${city.food_turns}|${city.disorder}`;
  if (key === cityKey && el.childElementCount) return;
  cityKey = key;
  const n = ++cityToken;
  let c;
  try {
    c = await api.query("city", { city: city.id });
  } catch {
    c = null;
  }
  if (n !== cityToken || !skin.on) return;
  el.hidden = false;
  const food = c?.food;
  const prod = c?.production;
  const cur = prod?.current ?? city.production;
  const rates = Object.fromEntries((c?.commerces ?? []).map((x) => [x.commerce, x.rate]));
  const yields = Object.fromEntries((c?.yields ?? []).map((x) => [x.yield, x.rate]));
  const row = (icon, label, value, title, tip = null) => h("div.iso-yield", { title: tip ? null : (title ?? null), tip }, isoIcon(icon, { px: 32 }), h("span.label", label), h("span.value", value));
  // The mood's and health's parts (the `city` query's lists, which sum to the totals).
  const partsTip = (head, good, goodWord, bad, badWord) => {
    const list = (ps) => (ps ?? []).map(([l, n]) => `${n} ${l}`).join(", ");
    return [h("div.tip-title", head), good?.length ? h("div", `${goodWord}: ${list(good)}`) : null, bad?.length ? h("div", `${badWord}: ${list(bad)}`) : null];
  };
  const queue = (c?.queue ?? []).slice(0, 4);
  fill(
    el,
    h("h3.iso-title", { title: city.capital ? "Your capital" : null }, city.capital ? isoIcon("civics", { px: 16, cls: "capital" }) : null, city.name ?? ""),
    cityPicture(city),
    h(
      "div.iso-stat",
      { title: food ? `${food.stored}/${food.threshold} food; ${food.turns > 0 ? `grows in ${turns(food.turns)}` : food.difference < 0 ? "starving" : "not growing"}` : null },
      isoIcon("population", { px: 32 }),
      h("span.num", String(city.size ?? "?")),
      h("div.iso-stack", h("span.label", "Population"), food ? isoBar(food.stored / Math.max(1, food.threshold), { w: 168, next: (food.stored + Math.max(0, food.difference)) / Math.max(1, food.threshold), kind: "food" }) : null),
    ),
    h("div.iso-rule"),
    h(
      "div.iso-stat",
      { title: prod ? `${prod.stored}/${prod.needed} production, +${prod.rate} a turn` : null },
      isoIcon("production", { px: 32 }),
      h(
        "div.iso-stack",
        h("span.label", "Production"),
        h("span.sub", cur ? `${orderName(cur.order, cur.type)}${cur.turns !== undefined && cur.turns !== null ? ` (${cur.turns})` : ""}` : "nothing"),
      ),
    ),
    h("div.iso-rule"),
    c
      ? h(
          "div.iso-yields",
          row("food", "Food", signed(food?.difference ?? 0), "Food left over a turn"),
          row("production", "Production", signed(yields[1] ?? 0)),
          row("gold", "Gold", signed(rates[0] ?? 0)),
          row("research", "Research", signed(rates[1] ?? 0)),
          row("culture", "Culture", signed(rates[2] ?? 0), c.culture !== undefined ? `Culture ${c.culture}/${c.culture_threshold}` : null),
          h(
            "div.iso-mood",
            c.happiness ? row(c.happiness.unhappy > c.happiness.happy ? "unhappy" : "happy", "Mood", `${c.happiness.happy}/${c.happiness.unhappy}`, null, partsTip(`${c.happiness.happy} happy, ${c.happiness.unhappy} unhappy`, c.happiness.happy_parts, "Happy", c.happiness.unhappy_parts, "Unhappy")) : null,
            c.health ? row(c.health.bad > c.health.good ? "unhealthy" : "health", "Health", `${c.health.good}/${c.health.bad}`, null, partsTip(`${c.health.good} healthy, ${c.health.bad} unhealthy${c.health.bad > c.health.good ? ` (−${c.health.bad - c.health.good} food a turn)` : ""}`, c.health.good_parts, "Healthy", c.health.bad_parts, "Unhealthy")) : null,
          ),
        )
      : null,
    queue.length
      ? h(
          "div.iso-queue",
          h("div.iso-rule"),
          h("h4", "Build queue"),
          queue.map((q) => h("div.iso-qrow", queueIcon(q), h("span.label", orderName(q.order, q.type)), h("span.value", q.turns !== undefined && q.turns !== null ? String(q.turns) : ""))),
        )
      : null,
    h("button.iso-open", { onclick: () => emit("open-city", city.id), title: "Open the city screen (or double-click the city)" }, "Open city"),
  );
}

/** A queued item's icon (`q`: { order, type }): a unit's figure, a building's or project's icon. */
export function queueIcon(q) {
  const order = tname("OrderType", q.order);
  if (order === "ORDER_TRAIN") {
    const key = skin.view?.iso?.unitKey(q.type, me()?.player);
    const el = key ? spriteIcon(key, 32) : null;
    if (el) return h("span.iso-qicon", el);
  }
  const process = order === "ORDER_MAINTAIN" ? tname("ProcessType", q.type) ?? "" : "";
  const name = order === "ORDER_CREATE" ? "greatwork" : process ? (/RESEARCH/.test(process) ? "research" : /CULTURE/.test(process) ? "culture" : "gold") : "cities";
  return h("span.iso-qicon", isoIcon(name, { px: 32 }));
}
