// The title screen: the game's front page. It shows at boot while the client connects, stays when
// there is no game (Continue, New game, Load game, Keys, Options), fades out when a game is
// running, and comes back from the game menu's "Main menu" (then with Resume). Behind it, a small
// isometric island drawn from the isometric renderer's sprites (docs/ISO.md): a kingdom with its
// farms and castle on one shore, the hells creeping in on the other, as the game itself goes. The
// camera drifts slowly and clouds pass over the sea; prefers-reduced-motion keeps it still.

import * as api from "./api.js";
import { S } from "./state.js";
import { h } from "./dom.js";
import { look, UI } from "./look.js";
import { SPRITES } from "./iso/sprites.js";
import { saveTurn, whenText } from "./screens.js";

/** The M7 tile: 192 x 128 art px, the diamond in rows 0-95 (docs/ISO.md 3). */
const TW = 192;
const TH = 96;
const WATER_DROP = 8;

// ---------------------------------------------------------------------------------------------
// The island. Rows r (down-left on screen), columns c (down-right): screen x = (c - r) * TW / 2,
// y = (c + r) * TH / 2. Base tiles: `.` ocean, `~` coast, g grass, p plains, d desert, t tundra,
// s snow, m marsh, b burning sands, k broken lands, x fields of perdition.

const ISLAND = [
  "..................",
  "..................",
  "........~~~.......",
  "......~~sst~~.....",
  ".....~sstttpb~~...",
  "....~tttgppbbkk~..",
  "....~tggggpkbbxk~.",
  "...~gggggggpkbxx~.",
  "...~ggggggggpkkx~.",
  "...~gggggggggpp~..",
  "....~ggggggggpp~..",
  "....~gmggggggg~...",
  ".....~ggggggg~....",
  "......~~gggg~.....",
  "........~~~~......",
  "..................",
];

const BASE = {
  "~": "tile_water",
  g: "tile_grass",
  p: "tile_plains",
  d: "tile_desert",
  t: "tile_tundra",
  s: "tile_snow",
  m: "tile_marsh",
  b: "tile_burning_sands",
  k: "tile_broken_lands",
  x: "tile_fields_of_perdition",
};

/** What stands on the island: [c, r, sprite, options]. `ovl` overlays sit in the tile; the rest
 *  stand on it (features and buildings on the tile's bottom vertex, units 6 px below its centre,
 *  resources at its front-right). `flip`: facing left. */
const PLACED = [
  // The cold north: snowy peaks and hills.
  [8, 3, "feat_mountain"],
  [9, 3, "feat_mountain"],
  [8, 4, "ovl_hills_snow"],
  [10, 4, "ovl_hills_snow"],
  [6, 5, "feat_forest"],
  [7, 5, "feat_forest_new"],
  // The old forests of the west, the elves' temple among them.
  [5, 6, "feat_forest_ancient"],
  [6, 6, "feat_forest_ancient"],
  [4, 7, "feat_forest_ancient"],
  [5, 7, "bld_temple_leaves"],
  [4, 8, "feat_forest"],
  [5, 8, "feat_forest"],
  [4, 9, "feat_forest_new"],
  [6, 7, "unit_treant"],
  [5, 9, "res_deer"],
  // The kingdom: its castle, a town, farms, a windmill, a mage's tower and a harbour.
  [8, 8, "bld_city_castle"],
  [7, 6, "bld_mage_guild"],
  [9, 6, "ovl_hills"],
  [9, 6, "bld_mine"],
  [7, 9, "ovl_farm"],
  [8, 9, "ovl_farm"],
  [9, 9, "bld_windmill"],
  [8, 10, "ovl_farm"],
  [7, 8, "ovl_pasture"],
  [7, 8, "res_horse"],
  [6, 11, "bld_city_town"],
  [9, 11, "bld_harbor"],
  [7, 11, "bld_cottage"],
  [10, 10, "ovl_farm"],
  [10, 10, "res_wheat"],
  [6, 9, "res_cow"],
  [6, 12, "feat_scrub"],
  // Its champions, facing the east.
  [10, 7, "unit_paladin"],
  [10, 8, "unit_knight"],
  [9, 7, "unit_archmage"],
  [11, 9, "unit_phalanx"],
  [8, 7, "unit_angel"],
  // The hells: burning sands, souls, flames, a smoking mountain and their lord.
  [12, 4, "feat_flames"],
  [13, 5, "feat_mountain_hell"],
  [14, 5, "feat_volcano"],
  [12, 6, "feat_tormented_souls"],
  [14, 6, "feat_flames"],
  [13, 7, "bld_ruins"],
  [14, 8, "feat_tormented_souls"],
  [12, 5, "unit_hyborem", { flip: true }],
  [13, 6, "unit_demon", { flip: true }],
  [12, 8, "unit_manes", { flip: true }],
  [11, 6, "unit_manes", { flip: true }],
  // The sea.
  [10, 13, "unit_trireme", { flip: true }],
  [5, 13, "feat_fishing_boats"],
  [13, 15, "unit_sea_serpent", { flip: true }],
  [15, 11, "res_whale"],
  [3, 11, "res_fish"],
  [13, 12, "unit_frigate", { flip: true }],
];

function dprOf() {
  return Math.min(window.devicePixelRatio || 1, 2);
}

/** The island's extent: its land's middle in (c - r) and (c + r) (`cd`, `cs`), and its size in
 *  art px (`w`, `h`, with room for what stands up). */
let bounds = null;
function islandBounds() {
  if (bounds) return bounds;
  let d0 = Infinity, d1 = -Infinity, s0 = Infinity, s1 = -Infinity;
  ISLAND.forEach((row, r) => [...row].forEach((ch, c) => {
    if (ch === ".") return;
    d0 = Math.min(d0, c - r);
    d1 = Math.max(d1, c - r);
    s0 = Math.min(s0, c + r);
    s1 = Math.max(s1, c + r);
  }));
  bounds = { cd: (d0 + d1) / 2, cs: (s0 + s1) / 2 + 0.5, w: (d1 - d0 + 2) * (TW / 2), h: (s1 - s0 + 2) * (TH / 2) + 160 };
  return bounds;
}

/** Stable 32-bit hash of a cell (tile variants). */
function hash(c, r) {
  let x = (c * 73856093) ^ (r * 19349663);
  x = Math.imul(x ^ (x >>> 13), 0x5bd1e995);
  return (x ^ (x >>> 15)) >>> 0;
}

function baseAt(c, r) {
  const row = ISLAND[r];
  const ch = row?.[c];
  if (!ch || ch === ".") return null;
  return BASE[ch] ?? null;
}

/** The tile sprite of a cell: its base (or the ocean), `_b` variants on about half the cells. */
function tileOf(c, r) {
  const base = baseAt(c, r) ?? "tile_ocean";
  const b = `${base}_b`;
  return SPRITES[b] && hash(c, r) % 2 ? b : base;
}

// ---------------------------------------------------------------------------------------------
// The backdrop: the island painted once per size into a buffer, then drawn each frame with the
// camera's drift and the clouds.

class Diorama {
  constructor(canvas) {
    this.canvas = canvas;
    this.images = new Map();
    this.buffer = null;
    this.size = "";
    this.raf = 0;
    this.running = false;
    this.t0 = performance.now();
    this.clouds = [];
    this.loaded = false;
    this.gen = 0;
    this.ready = this.load().then(() => (this.loaded = true));
    // Still (reduced motion) the island is drawn once: a new window size draws it again (once the
    // sprites are in, and never beside a running loop).
    window.addEventListener("resize", () => this.running && this.loaded && !this.raf && this.frame());
  }

  /** Loads every sprite the scene uses. */
  load() {
    const keys = new Set(["tile_ocean", "tile_ocean_b", "tile_water"]);
    for (const row of ISLAND) for (const ch of row) if (BASE[ch]) keys.add(BASE[ch]), SPRITES[`${BASE[ch]}_b`] && keys.add(`${BASE[ch]}_b`);
    for (const [, , key] of PLACED) keys.add(key);
    return Promise.all(
      [...keys].filter((k) => SPRITES[k]).map(
        (k) =>
          new Promise((resolve) => {
            const im = new Image();
            im.onload = () => (this.images.set(k, im), resolve());
            im.onerror = () => resolve();
            im.src = new URL(`./iso/${k}.png`, import.meta.url).href;
          }),
      ),
    );
  }

  start() {
    if (this.running) return;
    this.running = true;
    this.ready.then(() => this.running && !this.raf && this.frame());
  }

  stop() {
    this.running = false;
    this.gen++;
    cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  /** Art px -> device px for this viewport: the island in about 62% of the window's width and
   *  78% of its height, in eighths. */
  scale(vw, vh, dpr) {
    const b = islandBounds();
    const k = Math.min((0.62 * vw) / b.w, (0.78 * vh) / b.h) * dpr;
    return Math.max(0.25, Math.round(k * 8) / 8);
  }

  /** Paints the island into the buffer (the viewport plus a margin for the drift). */
  paint(w, h, k) {
    const margin = Math.round(40 * k + 24);
    const bw = w + margin * 2;
    const bh = h + margin * 2;
    const buf = document.createElement("canvas");
    buf.width = bw;
    buf.height = bh;
    const g = buf.getContext("2d");
    g.imageSmoothingEnabled = !Number.isInteger(k);
    g.imageSmoothingQuality = "high";
    // The island's middle at 66% across, 52% down (the menu is at the left); on a narrow screen,
    // in the middle.
    const b = islandBounds();
    const across = w / dprOf() < 760 ? 0.5 : 0.66;
    const ox = bw * across - b.cd * (TW / 2) * k;
    const oy = bh * 0.52 - b.cs * (TH / 2) * k;
    const at = (c, r) => ({ x: ox + (c - r) * (TW / 2) * k, y: oy + (c + r) * (TH / 2) * k });
    // The cells that cover the buffer.
    const span = Math.ceil(Math.max(bw / (TW * k), bh / (TH * k))) + 4;
    const cells = [];
    for (let c = -span; c < span + ISLAND[0].length; c++) {
      for (let r = -span; r < span + ISLAND.length; r++) {
        const p = at(c, r);
        if (p.x < -TW * k || p.x > bw + TW * k || p.y < -TH * k || p.y > bh + 2 * TH * k) continue;
        cells.push({ c, r, ...p });
      }
    }
    cells.sort((a, b) => a.c + a.r - (b.c + b.r) || a.c - b.c);
    const draw = (key, x, y, flip = false) => {
      const im = this.images.get(key);
      if (!im) return;
      const dw = im.width * k;
      const dh = im.height * k;
      if (!flip) {
        g.drawImage(im, Math.round(x), Math.round(y), Math.round(dw), Math.round(dh));
        return;
      }
      g.save();
      g.translate(Math.round(x + dw), Math.round(y));
      g.scale(-1, 1);
      g.drawImage(im, 0, 0, Math.round(dw), Math.round(dh));
      g.restore();
    };
    // The tiles, back to front: water ones lower, so the land's earth skirts show as banks.
    for (const cell of cells) {
      const key = tileOf(cell.c, cell.r);
      const water = key.startsWith("tile_water") || key.startsWith("tile_ocean");
      draw(key, cell.x - (TW / 2) * k, cell.y - (TH / 2) * k + (water ? WATER_DROP * k : 0));
      for (const [c, r, s] of PLACED) if (c === cell.c && r === cell.r && s.startsWith("ovl_")) draw(s, cell.x - (TW / 2) * k, cell.y - (TH / 2) * k);
    }
    // Everything that stands up, by depth; on one plot: feature, building, resource, unit.
    const order = (s) => (s.startsWith("feat_") ? 0 : s.startsWith("bld_") ? 1 : s.startsWith("res_") ? 2 : 3);
    const standing = PLACED.filter(([, , s]) => !s.startsWith("ovl_")).sort((a, b) => a[0] + a[1] - (b[0] + b[1]) || a[0] - b[0] || order(a[2]) - order(b[2]));
    for (const [c, r, s, o = {}] of standing) {
      const im = this.images.get(s);
      if (!im) continue;
      const p = at(c, r);
      const water = !baseAt(c, r) || baseAt(c, r) === "tile_water";
      const drop = water ? WATER_DROP * k : 0;
      if (s.startsWith("unit_")) {
        // A contact shadow under the figure.
        g.fillStyle = "rgba(20,13,12,0.28)";
        g.beginPath();
        g.ellipse(p.x, p.y + 6 * k + drop, 30 * k, 11 * k, 0, 0, Math.PI * 2);
        g.fill();
        draw(s, p.x - (im.width / 2) * k, p.y + 6 * k - im.height * k + drop, !!o.flip);
      } else if (s.startsWith("res_")) {
        draw(s, p.x + 40 * k - (im.width / 2) * k, p.y + 30 * k - im.height * k + drop);
      } else {
        draw(s, p.x - (im.width / 2) * k, p.y + (TH / 2) * k - im.height * k + drop, !!o.flip);
      }
    }
    this.buffer = { c: buf, margin };
  }

  /** Puffy clouds drifting west to east, seeded once. */
  seedClouds() {
    this.clouds = [];
    for (let i = 0; i < 6; i++) {
      const h1 = hash(i, 7);
      this.clouds.push({ x: (h1 % 1000) / 1000, y: 0.08 + (((h1 >>> 10) % 1000) / 1000) * 0.8, s: 0.7 + ((h1 >>> 20) % 100) / 160, v: 0.1 + ((h1 >>> 4) % 100) / 400, n: 3 + (h1 % 3) });
    }
  }

  /** The clouds: shaded undersides, then lit tops (iso.js `drawClouds`), fading out over the
   *  left of the screen where the menu is. Each cloud is painted opaque into a scratch canvas and
   *  laid down once at its fade, so its two passes do not show through each other. */
  drawClouds(g, w, h, k, t) {
    const sc = (this.scratch ??= document.createElement("canvas"));
    for (const cl of this.clouds) {
      const span = w + 600 * k;
      const x = ((cl.x * span + cl.v * t * 60 * k) % span) - 300 * k;
      const y = cl.y * h;
      const fade = Math.max(0, Math.min(1, (x / w - 0.36) / 0.18));
      if (fade <= 0) continue;
      const puffs = [];
      for (let i = 0; i < cl.n; i++) {
        const hh = hash(i, Math.round(cl.y * 1000));
        const r = (44 + (hh % 30)) * cl.s * k;
        puffs.push({ r, x: x + (i - cl.n / 2) * 56 * cl.s * k + ((hh >>> 8) % 20) * k, y: y + (((hh >>> 12) % 24) - 12) * cl.s * k });
      }
      const x0 = Math.floor(Math.min(...puffs.map((p) => p.x - p.r))) - 2;
      const y0 = Math.floor(Math.min(...puffs.map((p) => p.y - p.r))) - 2;
      const x1 = Math.ceil(Math.max(...puffs.map((p) => p.x + p.r))) + 2;
      const y1 = Math.ceil(Math.max(...puffs.map((p) => p.y + p.r))) + 2;
      sc.width = Math.max(1, x1 - x0);
      sc.height = Math.max(1, y1 - y0);
      const sg = sc.getContext("2d");
      for (const lit of [false, true]) {
        sg.fillStyle = lit ? "#f7f9ff" : "#b9cbe6";
        sg.beginPath();
        for (const p of puffs) {
          const py = p.y - y0 + (lit ? -p.r * 0.12 : p.r * 0.08);
          sg.moveTo(p.x - x0 + p.r, py);
          sg.ellipse(p.x - x0, py, lit ? p.r * 0.92 : p.r, (lit ? 0.52 : 0.6) * p.r, 0, 0, Math.PI * 2);
        }
        sg.fill();
      }
      g.globalAlpha = 0.9 * fade;
      g.drawImage(sc, x0, y0);
    }
    g.globalAlpha = 1;
  }

  frame() {
    if (!this.running) return;
    const c = this.canvas;
    const dpr = dprOf();
    const vw = Math.max(1, c.clientWidth);
    const vh = Math.max(1, c.clientHeight);
    const w = Math.round(vw * dpr);
    const h = Math.round(vh * dpr);
    const k = this.scale(vw, vh, dpr);
    const key = `${w}x${h}@${k}`;
    if (key !== this.size || !this.buffer) {
      c.width = w;
      c.height = h;
      this.size = key;
      this.paint(w, h, k);
      if (!this.clouds.length) this.seedClouds();
    }
    const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    const t = still ? 0 : performance.now() - this.t0;
    const g = c.getContext("2d");
    const m = this.buffer.margin;
    const dx = Math.sin(t / 9000) * m * 0.6;
    const dy = Math.cos(t / 12000) * m * 0.35;
    g.drawImage(this.buffer.c, Math.round(-m + dx), Math.round(-m + dy));
    this.drawClouds(g, w, h, k, t / 1000);
    if (still) {
      this.raf = 0;
      return;
    }
    // About 30 frames a second is plenty for a slow drift. `raf` stays set until the next frame
    // runs, so no second loop starts meanwhile.
    const gen = this.gen;
    this.raf = requestAnimationFrame(() =>
      setTimeout(() => {
        if (gen !== this.gen) return; // stopped (and maybe started again) meanwhile
        this.raf = 0;
        this.frame();
      }, 16),
    );
  }
}

// ---------------------------------------------------------------------------------------------
// The screen

let el = null;
let diorama = null;
let actions = null;
/** "loading" (the client connects), "menu" (no game), "game" (opened over a running game). */
let mode = "loading";
let visible = false;

/** Callbacks of the menu (main.js): `newGame()`, `load()`, `keys()`, `options()`, `loadSave(name)`. */
export function initTitle(handlers) {
  actions = handlers;
  el = document.getElementById("title-screen");
  if (!el) return;
  const canvas = h("canvas.title-backdrop", { "aria-hidden": "true" });
  el.replaceChildren(
    canvas,
    h("div.title-shade", { "aria-hidden": "true" }),
    h(
      "div.title-column",
      h("div.title-logo.pending", { role: "heading", "aria-level": "1" }, h("span.title-logo-text", "Fall from Heaven")),
      h("p.title-tagline", "Turn-based fantasy strategy: build a civilization, master the spheres of magic, and decide whether the world ends."),
      h("nav.title-menu", { "aria-label": "Main menu" }),
      h("p.title-status", { "aria-live": "polite" }),
    ),
    h(
      "footer.title-foot",
      h("span", "Based on Fall from Heaven 2 (0.41), the Civilization IV mod by Kael and the Fall from Heaven team, ported to Rust."),
      h("span.title-build"),
    ),
  );
  diorama = new Diorama(canvas);
  // The logo waits a moment for the pixel art (titleArt), so the serif stand-in does not flash;
  // without the art it shows anyway.
  setTimeout(() => el.querySelector(".title-logo")?.classList.remove("pending"), 2500);
  el.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape" && mode === "game" && !document.querySelector("#modals .backdrop")) {
      ev.stopPropagation();
      hideTitle();
    }
  });
  showTitle({ loading: true });
}

/** The logo in the Vigil title face once the art has loaded (a serif title before). */
export function titleArt() {
  const logo = el?.querySelector(".title-logo");
  if (!logo || !look.art || logo.querySelector(".ui-ptext")) return;
  const u = window.innerWidth < 700 ? 4 : window.innerWidth < 1400 ? 6 : 8;
  const span = UI.textSpan(look.art, "Fall from Heaven", { font: "vigil", u, dpr: look.dpr, cls: "title-logo-px" });
  logo.replaceChildren(span);
  logo.classList.remove("pending");
  renderMenu();
}

/** Whether the title screen is up. */
export function titleShown() {
  return visible;
}

/** Shows the title screen. `loading`: while the client connects (no menu yet); `game`: over a
 *  running game (Resume, Escape closes it). */
export function showTitle({ loading = false, game = false } = {}) {
  if (!el) return;
  mode = loading ? "loading" : game ? "game" : "menu";
  el.hidden = false;
  el.classList.remove("leaving");
  document.body.classList.add("title-up");
  visible = true;
  diorama?.start();
  const build = el.querySelector(".title-build");
  if (build) build.textContent = globalThis.ffhWeb?.build ? `build ${globalThis.ffhWeb.build}` : "";
  renderMenu();
}

/** Fades the title screen out (a game is running, or Resume). */
export function hideTitle() {
  if (!el || !visible) return;
  visible = false;
  document.body.classList.remove("title-up");
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  if (reduce) {
    el.hidden = true;
    diorama?.stop();
    return;
  }
  el.classList.add("leaving");
  setTimeout(() => {
    if (visible) return;
    el.hidden = true;
    el.classList.remove("leaving");
    diorama?.stop();
  }, 450);
}

/** The phase changed (main.js): no game shows the menu, a game hides the screen unless the
 *  player opened it over the game. */
export function titlePhase(phase) {
  if (!el) return;
  if (phase === "no_game" || phase === "crashed") {
    if (!visible || mode !== "menu") showTitle();
    return;
  }
  if (visible && mode !== "game") hideTitle();
}

let savesAsked = false;
/** The newest save is known (or the lookup gave up): the menu can show. */
let savesKnown = false;
let latest = null;

/** The newest save (by time, else the newest autosave's turn) for Continue. */
async function findLatest() {
  try {
    const r = (await api.session("saves")).r;
    let best = null;
    for (const g of r.games ?? []) {
      for (const s of g.saves ?? []) {
        const turn = saveTurn(s.name);
        const rank = s.modified ?? turn ?? -1;
        if (!best || rank > best.rank) best = { name: `${g.game}/${s.name}`, game: g.game, save: s.name, rank, turn, modified: s.modified ?? null };
      }
    }
    return best;
  } catch {
    return null;
  }
}

function menuButton(label, opts, onClick) {
  return h(`button.title-item${opts.primary ? ".primary" : ""}`, { id: opts.id, title: opts.title ?? "", disabled: !!opts.disabled, onclick: onClick }, label, opts.sub ? h("small", opts.sub) : null);
}

function renderMenu() {
  if (!el) return;
  const nav = el.querySelector(".title-menu");
  const status = el.querySelector(".title-status");
  if (!nav) return;
  if (mode === "loading") {
    nav.replaceChildren();
    if (status) status.textContent = "Preparing the world"; // style.css animates the dots
    return;
  }
  if (status) status.textContent = "";
  // With no game the menu waits for the newest save (a moment), so Continue does not appear
  // above the other items after they are up and move them under the pointer.
  if (mode === "menu" && !savesAsked) {
    savesAsked = true;
    savesKnown = false;
    const done = (b) => {
      if (savesKnown) return;
      savesKnown = true;
      latest = b;
      renderMenu();
    };
    findLatest().then(done);
    setTimeout(() => done(null), 3000);
  }
  if (mode === "menu" && !savesKnown) {
    nav.replaceChildren();
    if (status) status.textContent = "Preparing the world";
    return;
  }
  const items = [];
  if (mode === "game") items.push(menuButton("Resume", { id: "title-resume", primary: true, title: "Back to the game [Esc]" }, () => hideTitle()));
  if (mode === "menu" && latest) {
    items.push(
      menuButton("Continue", { id: "title-continue", primary: true, sub: [latest.game, latest.turn !== null ? `turn ${latest.turn}` : latest.save, whenText(latest.modified)].filter(Boolean).join(" · "), title: `Load ${latest.name}` }, async (ev) => {
        const b = ev.currentTarget;
        b.disabled = true;
        try {
          await actions?.loadSave(latest.name);
        } finally {
          b.disabled = false;
        }
      }),
    );
  }
  items.push(menuButton("New game", { id: "title-new", primary: mode === "menu" && !latest, title: "Choose a civilization and a world" }, () => actions?.newGame()));
  items.push(menuButton("Load game", { id: "title-load", title: "Your saved games and autosaves" }, () => actions?.load()));
  items.push(menuButton("Options", { id: "title-options", title: "The map renderer and the player options" }, () => actions?.options()));
  items.push(menuButton("Keys and controls", { id: "title-keys", title: "Civ4's controls, the hotkeys" }, () => actions?.keys()));
  const had = nav.contains(document.activeElement);
  nav.replaceChildren(...items);
  // The first item takes the keys (Enter), unless a dialog over the screen has them.
  if ((had || document.activeElement === document.body) && !document.querySelector("#modals .backdrop")) items[0]?.focus({ preventScroll: true });
}

/** Forgets the newest save (a game was saved or started): Continue asks again next time. */
export function titleSavesChanged() {
  savesAsked = false;
  savesKnown = false;
  latest = null;
}

/** Whether a game exists (S.view): used by Resume. */
export function hasGame() {
  return !!S.view;
}
