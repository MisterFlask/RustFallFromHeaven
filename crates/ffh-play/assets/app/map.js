// The map (design §5.1 "Map"): a canvas, square tiles, north up, the map's wrap honoured.
//
// Two renderers draw it. The art renderer (`artmap.js`, the original pixel art of docs/ART.md)
// is used once the art has loaded; the flat renderer below (the P5 one: flat colours, glyphs and
// labels) before that, when the art cannot load, and when asked for (`?renderer=flat` or Tools →
// Options), for debugging.
//
// Flat: the terrain (everything the `map` arrays hold: terrain, hills, peaks, features, rivers,
// roads, improvements, bonuses, borders, fog) is drawn into offscreen chunks of CHUNK x CHUNK
// plots per zoom level, redrawn only where the map changed; cities, units, the selection, the
// path preview and the interface-mode targets are drawn over them each frame. A minimap shows
// the whole map and the viewport.

import { S, tname, dname, info, me, isBarbarian, ORDER_KINDS } from "./state.js";
import * as T from "./tiles.js";
import { ArtMap } from "./artmap.js";
import { IsoMap, ISO_DEFAULT_ZOOM } from "./iso.js";
import { artZooms, defaultArtZoom } from "./stacks.js";

const ZOOMS = [12, 16, 24, 32, 48, 64];
/** Keyboard scrolling (W A S D, `scrollHold`): the speed is SCROLL_VIEWS of the view's short side a
 *  second, within [SCROLL_MIN, SCROLL_MAX] CSS px; the velocity eases with this time constant (s)
 *  and the view is at rest below SCROLL_REST px a second. */
const SCROLL_VIEWS = 0.8;
const SCROLL_MIN = 480;
const SCROLL_MAX = 1000;
const SCROLL_EASE_S = 0.07;
const SCROLL_REST = 8;
/** The city screen's panels over the map (iso.css, `.city-overlay`): the CSS px they take at each
 *  side, above and below; the city is framed in what is left (`frameCity`). */
const CITY_FRAME = { side: 384, top: 150, bottom: 84 };
/** The terrain cache's budget in device pixels (least recently used chunks go first). */
const PIXEL_BUDGET = 32e6;
/** Activities drawn as asleep (a grey pip): fortified (C++ sleeps them), sleeping, healing,
 *  on sentry. */
const ASLEEP = ["ACTIVITY_SLEEP", "ACTIVITY_HEAL", "ACTIVITY_SENTRY"];
const FONT = "system-ui, 'DejaVu Sans', 'Segoe UI', sans-serif";

function withAlpha(hex, a) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) return hex;
  return `rgba(${parseInt(m[1], 16)},${parseInt(m[2], 16)},${parseInt(m[3], 16)},${a})`;
}

function darken(hex, f) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) return hex;
  const c = [1, 2, 3].map((i) => Math.round(parseInt(m[i], 16) * f));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

/** The plot type indices (PlotType), from the data. */
let PLOT = null;
function plotTypes() {
  if (!PLOT) {
    const t = S.data?.tables?.PlotType ?? [];
    const at = (n, d) => {
      const i = t.findIndex((e) => e.type === n);
      return i < 0 ? d : i;
    };
    PLOT = { peak: at("PLOT_PEAK", 0), hills: at("PLOT_HILLS", 1), land: at("PLOT_LAND", 2), ocean: at("PLOT_OCEAN", 3) };
  }
  return PLOT;
}

export function ownerColor(p) {
  return T.playerColor(p, isBarbarian(p));
}

export class MapView {
  /** `wrap`: the element holding `canvas.map`, `canvas.minimap` and `.tooltip`; `handlers`:
   *  `click(x, y, ev)`, `rightClick(x, y, ev)`, `hover(x, y, ev)` (x null when off the map),
   *  `dblclick(x, y, ev)`. */
  constructor(wrap, handlers) {
    this.wrap = wrap;
    this.canvas = wrap.querySelector("canvas.map");
    this.mini = wrap.querySelector("canvas.minimap");
    this.tip = wrap.querySelector(".tooltip");
    // The art renderer reads parts of the frame back (the vignette, light pools, range marks).
    this.ctx = this.canvas.getContext("2d", { willReadFrequently: true });
    this.h = handlers;
    this.cam = { cx: 0, cy: 0, ts: 32 };
    this.vw = 1;
    this.vh = 1;
    this.dpr = 1;
    this.map = null;
    this.chunks = new Map();
    this.pixels = 0;
    this.clock = 0;
    this.miniImage = null;
    /** Overlays: `path` (path query reply), `targets` ([[x, y]] or "any"), `hover` ({x, y}),
     *  `sel` ({x, y}), `units` (selected unit ids), `waypoints` ([[x, y]] held while Shift is
     *  down). */
    this.overlay = { path: null, targets: null, hover: null, sel: null, units: [], waypoints: null, city: null };
    /** The camera before a city screen framed its city (`cityOverlay`), to come back to. */
    this.cityCamera = null;
    /** CSS px of the view that floating panels cover at each side (the isometric interface sets
     *  them, isoui.js): `centerOn` puts a plot in the middle of the rest, `reveal` keeps it there. */
    this.insets = { top: 0, right: 0, bottom: 0, left: 0 };
    /** The map's layer (design §5.1 "Layers"): "terrain", "political" (owners' colours) or
     *  "yield" (the visible plots' food, production and commerce). */
    this.layer = "terrain";
    /** The art renderer (`ArtMap`) when it draws the map, else null (the flat renderer). */
    this.art = null;
    /** The isometric renderer (`IsoMap`, `?renderer=iso`) when it draws the map, else null. */
    this.iso = null;
    /** The experimental 3D renderer (`r3d/map3d.js`, `?renderer=3d`, docs/R3D.md) when it draws
     *  the map, else null. Loaded on demand: the page never fetches it otherwise. */
    this.r3d = null;
    /** The error from the last failed `useR3d(true)`, so the caller can report why (e.g. a
     *  software WebGL2 renderer: `code === "software-webgl"`). Cleared on a successful start. */
    this.r3dError = null;
    this.queued = false;
    /** An in-flight camera glide (`glideTo`): `{ sx, sy, dx, dy, t0, dur }`, else null. Direct
     *  manipulation (drag, zoom, pan) cancels it; `prefers-reduced-motion` skips it. */
    this.anim = null;
    this.animRaf = 0;
    /** Keyboard scrolling (`scrollHold`, W A S D held): the directions held and the view's
     *  velocity in screen px a second, eased towards SCROLL_SPEED and back to rest. */
    this.scroll = { held: new Set(), vx: 0, vy: 0, t: 0, raf: 0 };
    /** One-shot combat flashes (`flashTile`): `{ x, y, kind, t0, dur }`, drawn by both renderers
     *  and self-expiring. */
    this.flashes = [];
    this.flashRaf = 0;
    this.listen();
    new ResizeObserver(() => this.resize()).observe(wrap);
    this.resize();
  }

  // ------------------------------------------------------------------------------------------
  // Input

  listen() {
    const c = this.canvas;
    let down = null;
    c.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0 && ev.button !== 1) return;
      down = { x: ev.clientX, y: ev.clientY, cx: this.cam.cx, cy: this.cam.cy, dragged: false, button: ev.button, id: ev.pointerId };
      c.setPointerCapture(ev.pointerId);
    });
    c.addEventListener("pointermove", (ev) => {
      if (down) {
        const dx = ev.clientX - down.x;
        const dy = ev.clientY - down.y;
        if (!down.dragged && Math.hypot(dx, dy) > 5) down.dragged = true;
        if (down.dragged) {
          this.cancelGlide();
          const alt = this.iso ?? this.r3d;
          const pan = alt
            ? alt.panDelta(down.cx, down.cy, dx, dy)
            : { cx: down.cx - dx / this.cam.ts, cy: down.cy + dy / this.cam.ts };
          this.cam = T.clampCamera({ ...this.cam, ...pan }, this.map ?? { w: 1, h: 1 });
          this.hideTip();
          this.request();
          return;
        }
      }
      this.hoverAt(ev);
    });
    c.addEventListener("pointerup", (ev) => {
      const d = down;
      down = null;
      if (!d || d.dragged || ev.button !== 0) return;
      if (this.overlay.city) return this.cityClick(this.plotAt(ev, { ground: true }));
      const p = this.plotAt(ev);
      if (p) this.h.click?.(p.x, p.y, ev);
    });
    c.addEventListener("pointercancel", () => (down = null));
    c.addEventListener("contextmenu", (ev) => {
      ev.preventDefault();
      if (this.overlay.city) return;
      const p = this.plotAt(ev);
      if (p) this.h.rightClick?.(p.x, p.y, ev);
    });
    c.addEventListener("dblclick", (ev) => {
      if (this.overlay.city) return;
      const p = this.plotAt(ev);
      if (p) this.h.dblclick?.(p.x, p.y, ev);
    });
    c.addEventListener("mouseleave", () => {
      this.overlay.hover = null;
      this.hideTip();
      this.h.hover?.(null, null, null);
      this.request();
    });
    c.addEventListener(
      "wheel",
      (ev) => {
        ev.preventDefault();
        const r = c.getBoundingClientRect();
        this.zoom(ev.deltaY < 0 ? 1 : -1, ev.clientX - r.left, ev.clientY - r.top);
      },
      { passive: false },
    );
    const miniMove = (ev) => {
      if (!this.map) return;
      this.cancelGlide();
      // The canvas's content box (a frame may surround it).
      const r = this.mini.getBoundingClientRect();
      const w = this.mini.clientWidth || r.width;
      const hh = this.mini.clientHeight || r.height;
      const x = ((ev.clientX - r.left - this.mini.clientLeft) / w) * this.map.w;
      const y = this.map.h - ((ev.clientY - r.top - this.mini.clientTop) / hh) * this.map.h;
      this.cam = T.clampCamera({ ...this.cam, cx: x, cy: y }, this.map);
      this.request();
    };
    let miniDown = false;
    this.mini.addEventListener("pointerdown", (ev) => {
      miniDown = true;
      this.mini.setPointerCapture(ev.pointerId);
      miniMove(ev);
    });
    this.mini.addEventListener("pointermove", (ev) => miniDown && miniMove(ev));
    this.mini.addEventListener("pointerup", () => (miniDown = false));
  }

  /** The plot under pointer event `ev`: a figure drawn there, else the ground (`ground`: the
   *  ground only). */
  plotAt(ev, { ground = false } = {}) {
    if (!this.map) return null;
    const r = this.canvas.getBoundingClientRect();
    if (this.iso) return this.iso.screenToMap(ev.clientX - r.left, ev.clientY - r.top, { ground });
    if (this.r3d) return this.r3d.screenToMap(ev.clientX - r.left, ev.clientY - r.top);
    return T.screenToMap(this.cam, this.map, this.vw, this.vh, ev.clientX - r.left, ev.clientY - r.top);
  }

  hoverAt(ev) {
    const city = this.overlay.city;
    const p = this.plotAt(ev, { ground: !!city });
    const r = this.wrap.getBoundingClientRect();
    this.tipAt = { x: ev.clientX - r.left, y: ev.clientY - r.top };
    if (city) {
      // The city screen: what a click on the plot does, nothing else (redrawn when the plot
      // pointed at changes, not on every move).
      const plot = this.cityPlot(p);
      const mark = plot ? p : null;
      if (mark?.x !== this.overlay.hover?.x || mark?.y !== this.overlay.hover?.y) {
        this.overlay.hover = mark;
        this.request();
      }
      if (plot) this.showTip(city.tipOf(plot));
      else this.hideTip();
      return;
    }
    const old = this.overlay.hover;
    if (p?.x === old?.x && p?.y === old?.y) {
      this.placeTip();
      return;
    }
    this.overlay.hover = p;
    this.request();
    this.h.hover?.(p?.x ?? null, p?.y ?? null, ev);
  }

  // ------------------------------------------------------------------------------------------
  // The city screen's work area (city.js, with the isometric map's interface)

  /** Shows city screen overlay `o` ({ id, x, y, owner, plots, onPlot(plot), tipOf(plot) }: the
   *  `city` query's plots, what a click on one does and says) over the map, or ends it (`null`).
   *  A new city is framed (`frameCity`); the camera from before the screen comes back at the end. */
  cityOverlay(o) {
    if (!o) {
      if (!this.overlay.city) return;
      this.overlay.city = null;
      this.overlay.hover = null;
      this.hideTip();
      const back = this.cityCamera;
      this.cityCamera = null;
      if (back && this.map) {
        // The renderer may have changed meanwhile: its own zoom steps.
        if (this.zooms().includes(back.ts)) this.cam = { ...this.cam, ts: back.ts };
        this.glideTo(back.cx, back.cy);
      }
      this.request();
      return;
    }
    const fresh = this.overlay.city?.id !== o.id;
    this.overlay.city = o;
    if (fresh) {
      if (!this.cityCamera) this.cityCamera = { ...this.cam };
      this.overlay.hover = null;
      this.hideTip();
      this.frameCity(o);
    } else {
      // The plot pointed at says what a click does now (a click just changed it).
      const plot = this.cityPlot(this.overlay.hover);
      if (plot && !this.tip.hidden) this.showTip(o.tipOf(plot));
    }
    this.request();
  }

  /** The city's plot at map plot `p` (x, y), or null. */
  cityPlot(p) {
    const o = this.overlay.city;
    if (!o || !p) return null;
    return o.plots.find((q) => q.x === p.x && q.y === p.y) ?? null;
  }

  cityClick(p) {
    const plot = this.cityPlot(p);
    if (plot) this.overlay.city.onPlot(plot);
  }

  /** Centres city overlay `o`'s city at the largest zoom step that shows all its plots in the room
   *  the screen's panels leave (CITY_FRAME). */
  frameCity(o) {
    if (!this.map) return;
    // The plots' extent on screen, in tiles: north up for the square renderers; for the isometric
    // one a plot (dx, dy) from the city is (dx + dy) / 2 tiles right and (dx - dy) / 4 down.
    const ext = { x0: 0, x1: 0, y0: 0, y1: 0 };
    for (const p of o.plots) {
      let dx = p.x - o.x;
      if (this.map.wrap_x) dx = ((((dx + this.map.w / 2) % this.map.w) + this.map.w) % this.map.w) - this.map.w / 2;
      const dy = p.y - o.y;
      const sx = this.iso ? (dx + dy) / 2 : dx;
      const sy = this.iso ? (dx - dy) / 4 : -dy;
      ext.x0 = Math.min(ext.x0, sx);
      ext.x1 = Math.max(ext.x1, sx);
      ext.y0 = Math.min(ext.y0, sy);
      ext.y1 = Math.max(ext.y1, sy);
    }
    const tw = ext.x1 - ext.x0 + 1;
    const th = ext.y1 - ext.y0 + (this.iso ? 0.5 : 1);
    const w = this.vw - 2 * CITY_FRAME.side;
    const h = this.vh - CITY_FRAME.top - CITY_FRAME.bottom;
    const fits = (ts) => tw * ts <= w && th * ts <= h;
    const zs = this.zooms();
    const ts = zs.filter(fits).at(-1) ?? zs[0];
    if (ts !== this.cam.ts) {
      this.cancelGlide();
      this.cam = T.clampCamera({ ...this.cam, ts, cx: o.x + 0.5, cy: o.y + 0.5 }, this.map);
      this.request();
    } else this.glideTo(o.x + 0.5, o.y + 0.5);
  }

  // ------------------------------------------------------------------------------------------
  // Camera

  resize() {
    const r = this.wrap.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    if (this.art && dpr !== this.dpr) {
      // Another device-pixel ratio: the art's zoom steps change with it.
      this.dpr = dpr;
      this.snapZoom();
    }
    this.dpr = dpr;
    this.vw = Math.max(1, Math.round(r.width));
    this.vh = Math.max(1, Math.round(r.height));
    this.canvas.width = Math.round(this.vw * this.dpr);
    this.canvas.height = Math.round(this.vh * this.dpr);
    this.canvas.style.width = `${this.vw}px`;
    this.canvas.style.height = `${this.vh}px`;
    this.fitMini();
    // The bitmap fills the canvas's content box (a frame may surround it).
    const mr = this.mini.getBoundingClientRect();
    this.mini.width = Math.max(1, Math.round((this.mini.clientWidth || mr.width) * this.dpr));
    this.mini.height = Math.max(1, Math.round((this.mini.clientHeight || mr.height) * this.dpr));
    this.request();
  }

  /** The minimap keeps the map's proportions: its height follows its width (its content box: a
   *  frame may surround it), which the interface may change (isoui.js moves it). */
  fitMini() {
    const map = this.map;
    if (!map || !this.mini.isConnected) return;
    const w = this.mini.clientWidth || this.mini.getBoundingClientRect().width || 220;
    const frame = Math.max(0, this.mini.offsetHeight - this.mini.clientHeight);
    this.mini.style.height = `${Math.round((w * map.h) / map.w) + frame}px`;
  }

  /** The zoom steps (tile sizes in CSS px) of the renderer in use: the art's are whole numbers
   *  of device pixels per art pixel (docs/ART.md 3.2). */
  zooms() {
    if (this.iso) return this.iso.zooms();
    if (this.r3d) return this.r3d.zooms();
    return this.art ? artZooms(this.dpr) : ZOOMS;
  }

  zoom(step, sx = this.vw / 2, sy = this.vh / 2) {
    const zs = this.zooms();
    const i = zs.findIndex((z) => Math.abs(z - this.cam.ts) < 0.01);
    const next = zs[Math.min(zs.length - 1, Math.max(0, (i < 0 ? 3 : i) + step))];
    if (next === this.cam.ts) return;
    this.cancelGlide();
    if (this.iso || this.r3d) {
      this.cam = { ...this.cam, ts: next };
      this.hideTip();
      this.request();
      return;
    }
    this.cam = T.clampCamera(T.zoomAt(this.cam, this.vw, this.vh, next, sx, sy), this.map ?? { w: 1, h: 1 });
    this.hideTip();
    this.request();
  }

  pan(dx, dy) {
    if (!this.map) return;
    this.cancelGlide();
    this.cam = T.clampCamera({ ...this.cam, cx: this.cam.cx + dx, cy: this.cam.cy + dy }, this.map);
    this.request();
  }

  /** Starts (`on`) or ends scrolling in `dir` ("up", "left", "down", "right"): while a direction
   *  is held the view glides that way on screen, in any renderer (diagonals at the same speed),
   *  easing in from rest and out to it after the last key is released. */
  scrollHold(dir, on) {
    const s = this.scroll;
    if (on) {
      s.held.add(dir);
      this.cancelGlide();
    } else s.held.delete(dir);
    if (!s.raf && this.map && (s.held.size || s.vx || s.vy)) {
      s.t = performance.now();
      this.stepScroll();
    }
  }

  /** Lets go of every scroll key (the page lost the keyboard): the view eases to rest. */
  scrollRelease() {
    for (const d of [...this.scroll.held]) this.scrollHold(d, false);
  }

  /** The keyboard scroll speed, CSS px a second: most of the view's short side a second. */
  scrollSpeed() {
    return Math.max(SCROLL_MIN, Math.min(SCROLL_MAX, SCROLL_VIEWS * Math.min(this.vw, this.vh)));
  }

  stepScroll() {
    const s = this.scroll;
    s.raf = requestAnimationFrame(() => {
      s.raf = 0;
      const now = performance.now();
      const dt = Math.min(0.05, Math.max(0, (now - s.t) / 1000));
      s.t = now;
      let tx = (s.held.has("right") ? 1 : 0) - (s.held.has("left") ? 1 : 0);
      let ty = (s.held.has("down") ? 1 : 0) - (s.held.has("up") ? 1 : 0);
      const n = Math.hypot(tx, ty) || 1;
      const v = this.scrollSpeed();
      tx = (tx / n) * v;
      ty = (ty / n) * v;
      const k = this.reduceMotion() ? 1 : 1 - Math.exp(-dt / SCROLL_EASE_S);
      s.vx += (tx - s.vx) * k;
      s.vy += (ty - s.vy) * k;
      if (!s.held.size && Math.hypot(s.vx, s.vy) < SCROLL_REST) {
        s.vx = s.vy = 0;
        return;
      }
      this.moveView(s.vx * dt, s.vy * dt);
      this.stepScroll();
    });
  }

  /** Moves the view (dx, dy) CSS px on screen, right and down, as a drag the other way would. */
  moveView(dx, dy) {
    if (!this.map) return;
    const alt = this.iso ?? this.r3d;
    const pan = alt ? alt.panDelta(this.cam.cx, this.cam.cy, -dx, -dy) : { cx: this.cam.cx + dx / this.cam.ts, cy: this.cam.cy - dy / this.cam.ts };
    this.cam = T.clampCamera({ ...this.cam, ...pan }, this.map);
    this.hideTip();
    this.draw();
  }

  /** Whether the viewer asked for less motion; the glide then jumps instead. */
  reduceMotion() {
    return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  }

  cancelGlide() {
    if (this.animRaf) cancelAnimationFrame(this.animRaf);
    this.animRaf = 0;
    this.anim = null;
  }

  /** Eases the camera's centre to the plot centre (cx, cy) in map units: a glide across the page,
   *  the short way round a wrapping map. A tiny move, no map, or prefers-reduced-motion jumps. */
  glideTo(cx, cy) {
    if (!this.map) return;
    const end = T.clampCamera({ ...this.cam, cx, cy }, this.map);
    let dx = end.cx - this.cam.cx;
    if (this.map.wrap_x) {
      dx = ((dx % this.map.w) + this.map.w) % this.map.w;
      if (dx > this.map.w / 2) dx -= this.map.w;
    }
    const dy = end.cy - this.cam.cy;
    const dist = Math.hypot(dx, dy);
    if (this.reduceMotion() || dist < 0.75) {
      this.cancelGlide();
      this.cam = end;
      this.hideTip();
      this.request();
      return;
    }
    this.hideTip();
    // The camera stays unwrapped through the glide (the renderer handles any centre); it is
    // normalised at the end. Longer hops take a little longer, capped so a cross-map jump is a
    // quick sweep, not a slog.
    this.anim = { sx: this.cam.cx, sy: this.cam.cy, dx, dy, t0: performance.now(), dur: Math.min(460, 170 + dist * 7) };
    if (!this.animRaf) this.stepGlide();
  }

  stepGlide() {
    this.animRaf = requestAnimationFrame(() => {
      this.animRaf = 0;
      const a = this.anim;
      if (!a) return;
      const t = Math.min(1, (performance.now() - a.t0) / a.dur);
      const e = 1 - Math.pow(1 - t, 3); // ease-out cubic
      this.cam = { ...this.cam, cx: a.sx + a.dx * e, cy: a.sy + a.dy * e };
      if (t >= 1) {
        this.anim = null;
        this.cam = T.clampCamera(this.cam, this.map);
      }
      this.draw();
      if (this.anim) this.stepGlide();
    });
  }

  /** A brief combat light on a tile (docs/ART.md 4.5: "the loser's rim flashes crimson"); `kind`
   *  is "hit" (a struck defender) or "death" (a unit lost). It fades on its own; both renderers
   *  draw it. prefers-reduced-motion shows nothing (stillness is kept). */
  flashTile(x, y, kind) {
    if (this.reduceMotion() || !this.map) return;
    this.flashes.push({ x, y, kind, t0: performance.now(), dur: kind === "death" ? 620 : 460 });
    if (this.flashes.length > 8) this.flashes.splice(0, this.flashes.length - 8);
    if (!this.flashRaf) this.stepFlash();
  }

  stepFlash() {
    this.flashRaf = requestAnimationFrame(() => {
      this.flashRaf = 0;
      const now = performance.now();
      this.flashes = this.flashes.filter((fl) => now - fl.t0 < fl.dur);
      this.draw();
      if (this.flashes.length) this.stepFlash();
    });
  }

  /** The combat flashes, in the flat renderer (CSS pixels). The art renderer draws them in
   *  `ArtMap.drawFlashes`; both read this one list. */
  drawFlashes(g) {
    if (!this.flashes.length) return;
    const ts = this.cam.ts;
    const now = performance.now();
    for (const fl of this.flashes) {
      const t = Math.min(1, (now - fl.t0) / fl.dur);
      const a = (1 - t) * (fl.kind === "death" ? 0.5 : 0.58);
      this.eachCopy(fl.x, fl.y, (sx, sy) => {
        const cx = sx + ts / 2;
        const cy = sy + ts / 2 - (fl.kind === "death" ? t * ts * 0.25 : 0);
        const r = ts * (fl.kind === "death" ? 0.3 + t * 0.22 : 0.46);
        const grd = g.createRadialGradient(cx, cy, 0, cx, cy, r);
        grd.addColorStop(0, `rgba(119,37,43,${a})`); // crimson (#77252b), ART.md's war/damage hue
        grd.addColorStop(1, "rgba(119,37,43,0)");
        g.fillStyle = grd;
        g.beginPath();
        g.arc(cx, cy, r, 0, Math.PI * 2);
        g.fill();
      });
    }
  }

  /** Centres on plot (x, y). By default the camera glides there (`glideTo`); `animate: false`
   *  jumps (a new game's first look, where there is nothing to glide from). */
  centerOn(x, y, { animate = true } = {}) {
    if (!this.map) return;
    const at = this.aimAt(x + 0.5, y + 0.5);
    if (animate) return this.glideTo(at.cx, at.cy);
    this.cancelGlide();
    this.cam = T.clampCamera({ ...this.cam, ...at }, this.map);
    this.request();
  }

  /** The part of the view no floating panel covers (`insets`), in CSS px; the whole view when
   *  what is left would be too small to aim at. */
  clearRect() {
    const i = this.insets;
    const r = { x0: i.left, y0: i.top, x1: this.vw - i.right, y1: this.vh - i.bottom };
    return r.x1 - r.x0 < 160 || r.y1 - r.y0 < 120 ? { x0: 0, y0: 0, x1: this.vw, y1: this.vh } : r;
  }

  /** The camera centre that shows map point (px, py) in the middle of `clearRect`. */
  aimAt(px, py) {
    const r = this.clearRect();
    const dx = (r.x0 + r.x1) / 2 - this.vw / 2;
    const dy = (r.y0 + r.y1) / 2 - this.vh / 2;
    if (!dx && !dy) return { cx: px, cy: py };
    const alt = this.iso ?? this.r3d;
    return alt ? alt.panDelta(px, py, dx, dy) : { cx: px - dx / this.cam.ts, cy: py + dy / this.cam.ts };
  }

  /** The plot at the centre of the view (C++ `getLookAtPlot`), or null without a map. */
  center() {
    if (!this.map) return null;
    return { x: T.wrapCoord(Math.floor(this.cam.cx), this.map.w, true), y: Math.min(this.map.h - 1, Math.max(0, Math.floor(this.cam.cy))) };
  }

  /** Centres on (x, y) unless it is well inside the viewport's clear part (`clearRect`). */
  reveal(x, y) {
    const s = this.screenOf(x, y);
    const r = this.clearRect();
    const m = this.cam.ts * (r.x0 || r.y0 || r.x1 < this.vw || r.y1 < this.vh ? 1 : 2);
    if (!s || s.x < r.x0 + m || s.y < r.y0 + m || s.x > r.x1 - m || s.y > r.y1 - m) this.centerOn(x, y);
  }

  /** The centre of plot (x, y) on the canvas (CSS pixels), the copy nearest the centre. */
  screenOf(x, y) {
    if (!this.map) return null;
    if (this.iso) return this.iso.screenOf(x, y);
    if (this.r3d) return this.r3d.screenOf(x, y);
    const p = T.mapToScreen(this.cam, this.map, this.vw, this.vh, x, y);
    return { x: p.sx + this.cam.ts / 2, y: p.sy + this.cam.ts / 2, ts: this.cam.ts };
  }

  // ------------------------------------------------------------------------------------------
  // Data

  /** Switches to the art renderer (`on`, once the art has loaded) or back to the flat one; the
   *  zoom moves to the nearest step of the renderer. */
  useArt(on) {
    if (!!on === !!this.art) return;
    if (on) {
      this.art = new ArtMap(this);
      if (this.map) this.art.setMap(null, this.map, "all");
    } else {
      this.art.dispose();
      this.art = null;
    }
    this.snapZoom(on ? defaultArtZoom(this.dpr) : null);
    this.chunks.clear();
    this.pixels = 0;
    this.miniImage = null;
    this.request();
  }

  /** Switches to the isometric renderer (`?renderer=iso`, docs/ISO.md) or back to
   *  the default renderer. */
  useIso(on) {
    if (!!on === !!this.iso) return;
    if (on) {
      if (this.art) {
        this.art.dispose();
        this.art = null;
      }
      this.iso = new IsoMap(this);
      this.snapZoom(ISO_DEFAULT_ZOOM);
    } else {
      this.iso.dispose?.();
      this.iso = null;
      this.snapZoom(); // the renderer drawn now has its own steps (the isometric reach 384)
    }
    this.chunks.clear();
    this.pixels = 0;
    this.miniImage = null;
    this.request();
  }

  /** Switches to the experimental 3D renderer (`?renderer=3d`, docs/R3D.md) or back. Its module
   *  is imported here, on first use; resolves to false when it cannot run (no WebGL2), leaving the
   *  renderer as it was. */
  async useR3d(on) {
    if (!!on === !!this.r3d) return true;
    if (on) {
      let mod;
      this.r3dError = null;
      try {
        mod = await import("./r3d/map3d.js");
        if (this.iso) this.useIso(false);
        if (this.art) {
          this.art.dispose();
          this.art = null;
        }
        this.r3d = new mod.Map3D(this);
      } catch (e) {
        console.error("3d map:", e);
        this.r3dError = e;
        return false;
      }
      this.snapZoom(mod.DEFAULT_ZOOM);
    } else {
      this.r3d.dispose();
      this.r3d = null;
      this.ctx.setTransform(1, 0, 0, 1, 0, 0);
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
    this.chunks.clear();
    this.pixels = 0;
    this.miniImage = null;
    this.request();
    return true;
  }

  /** The zoom moved to the renderer's nearest step (or `ts` when given). */
  snapZoom(ts = null) {
    const zs = this.zooms();
    const want = ts ?? this.cam.ts;
    const next = zs.reduce((a, b) => (Math.abs(b - want) < Math.abs(a - want) ? b : a), zs[0]);
    this.cam = { ...this.cam, ts: next };
  }

  /** The renderer drawing the map: "art" or "flat". */
  renderer() {
    return this.r3d ? "3d" : this.iso ? "iso" : this.art ? "art" : "flat";
  }

  /** A new map (the `map` reply, or the map a delta brought up to date): redraws the chunks
   *  that changed (`changed`: the tile indices a delta listed; else found by comparing). */
  setMap(map, changed = null) {
    const prev = this.map;
    this.art?.setMap(prev, map, changed);
    const first = !this.map || this.map.w !== map.w || this.map.h !== map.h;
    const dirty = Array.isArray(changed) && !first ? T.chunksOfTiles(map, changed) : T.dirtyChunks(this.map, map);
    this.map = map;
    PLOT = null;
    if (dirty === "all") {
      this.chunks.clear();
      this.pixels = 0;
    } else {
      for (const [key, c] of [...this.chunks]) {
        if (dirty.has(key.slice(key.indexOf(":") + 1))) {
          this.pixels -= c.canvas.width * c.canvas.height;
          this.chunks.delete(key);
        }
      }
    }
    this.miniImage = null;
    if (first) {
      this.flashes = [];
      this.cam = T.clampCamera(this.cam, map);
      this.resize();
    }
    this.request();
    return first;
  }

  setOverlay(o) {
    Object.assign(this.overlay, o);
    this.request();
  }

  /** Switches the map's layer (the terrain cache is kept per layer). */
  setLayer(layer) {
    if (!T.LAYERS.includes(layer) || layer === this.layer) return;
    this.layer = layer;
    this.request();
  }

  // ------------------------------------------------------------------------------------------
  // Drawing

  request() {
    if (this.queued) return;
    this.queued = true;
    requestAnimationFrame(() => {
      this.queued = false;
      this.draw();
    });
  }

  draw() {
    if (this.r3d) {
      try {
        this.r3d.draw();
      } catch (e) {
        console.error("3d map:", e);
      }
      this.drawMini();
      return;
    }
    if (this.iso) {
      try {
        this.iso.draw();
      } catch (e) {
        console.error("iso map:", e);
      }
      this.drawMini();
      return;
    }
    if (this.art) {
      try {
        this.art.draw();
      } catch (e) {
        console.error("art map:", e);
      }
      this.drawMini();
      return;
    }
    const g = this.ctx;
    g.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    g.fillStyle = "#07080a";
    g.fillRect(0, 0, this.vw, this.vh);
    if (!this.map) {
      this.drawMini();
      return;
    }
    this.drawTerrain(g);
    this.drawCities(g);
    this.drawUnits(g);
    this.drawOverlays(g);
    this.drawFlashes(g);
    this.drawMini();
  }

  drawTerrain(g) {
    const map = this.map;
    const ts = this.cam.ts;
    const ncx = Math.ceil(map.w / T.CHUNK);
    const ncy = Math.ceil(map.h / T.CHUNK);
    const left = this.cam.cx - this.vw / 2 / ts;
    const right = this.cam.cx + this.vw / 2 / ts;
    const k0 = map.wrap_x ? Math.floor(left / map.w) : 0;
    const k1 = map.wrap_x ? Math.floor(right / map.w) : 0;
    this.clock++;
    for (let k = k0; k <= k1; k++) {
      for (let cx = 0; cx < ncx; cx++) {
        const x0 = cx * T.CHUNK + k * map.w;
        const cw = Math.min(T.CHUNK, map.w - cx * T.CHUNK);
        if (x0 + cw < left || x0 > right) continue;
        for (let cy = 0; cy < ncy; cy++) {
          const ch = Math.min(T.CHUNK, map.h - cy * T.CHUNK);
          const top = T.columnToScreen(this.cam, this.vw, this.vh, x0, cy * T.CHUNK + ch - 1);
          if (top.sy > this.vh || top.sy + ch * ts < 0) continue;
          const c = this.chunk(cx, cy, cw, ch);
          g.drawImage(c.canvas, top.sx, top.sy, cw * ts, ch * ts);
        }
      }
    }
  }

  chunk(cx, cy, cw, ch) {
    const ts = this.cam.ts;
    const key = `${this.layer}|${ts}@${this.dpr}:${cx},${cy}`;
    let c = this.chunks.get(key);
    if (!c) {
      const canvas = document.createElement("canvas");
      canvas.width = Math.round(cw * ts * this.dpr);
      canvas.height = Math.round(ch * ts * this.dpr);
      const g = canvas.getContext("2d");
      g.scale(this.dpr, this.dpr);
      for (let j = 0; j < ch; j++) {
        for (let i = 0; i < cw; i++) {
          this.drawPlot(g, cx * T.CHUNK + i, cy * T.CHUNK + j, i * ts, (ch - 1 - j) * ts, ts);
        }
      }
      c = { canvas, used: 0 };
      this.chunks.set(key, c);
      this.pixels += canvas.width * canvas.height;
      this.evict();
    }
    c.used = this.clock;
    return c;
  }

  evict() {
    if (this.pixels <= PIXEL_BUDGET) return;
    const old = [...this.chunks].filter(([, c]) => c.used < this.clock).sort((a, b) => a[1].used - b[1].used);
    for (const [key, c] of old) {
      if (this.pixels <= PIXEL_BUDGET * 0.75) break;
      this.chunks.delete(key);
      this.pixels -= c.canvas.width * c.canvas.height;
    }
  }

  /** One plot of the terrain cache at (px, py), `ts` pixels wide. */
  drawPlot(g, x, y, px, py, ts) {
    const m = this.map;
    const k = y * m.w + x;
    const vis = m.vis[k];
    if (vis === 0) {
      g.fillStyle = "#000";
      g.fillRect(px, py, ts, ts);
      return;
    }
    const P = plotTypes();
    const pt = m.plot_type[k];
    const terrain = tname("TerrainType", m.terrain[k]);
    g.fillStyle = pt === P.peak ? "#5d5955" : T.terrainColor(terrain);
    g.fillRect(px, py, ts, ts);
    if (ts >= 24) {
      g.strokeStyle = "rgba(0,0,0,0.13)";
      g.lineWidth = 1;
      g.strokeRect(px + 0.5, py + 0.5, ts - 1, ts - 1);
    }
    const cx = px + ts / 2;
    const cy = py + ts / 2;
    // Hills: hatching; peaks: a mountain.
    if (pt === P.hills) {
      g.strokeStyle = "rgba(40,30,20,0.35)";
      g.lineWidth = Math.max(1, ts / 20);
      g.beginPath();
      for (let d = ts / 4; d < ts * 2; d += ts / 4) {
        g.moveTo(px + Math.max(0, d - ts), py + Math.min(ts, d));
        g.lineTo(px + Math.min(ts, d), py + Math.max(0, d - ts));
      }
      g.stroke();
    }
    // Features.
    const f = T.featureStyle(tname("FeatureType", m.feature[k]));
    if (f?.tint) {
      g.fillStyle = f.tint;
      g.fillRect(px, py, ts, ts);
    }
    if (f?.glyph && ts >= 12) {
      g.fillStyle = f.color ?? "#ddd";
      g.font = `${Math.round(ts * 0.5)}px ${FONT}`;
      g.textAlign = "center";
      g.textBaseline = "middle";
      g.fillText(f.glyph, cx - ts * 0.18, cy + ts * 0.08);
      if (ts >= 24) g.fillText(f.glyph, cx + ts * 0.2, cy - ts * 0.12);
    }
    if (pt === P.peak) {
      g.fillStyle = "#cfc9c0";
      g.beginPath();
      g.moveTo(cx, py + ts * 0.18);
      g.lineTo(px + ts * 0.85, py + ts * 0.82);
      g.lineTo(px + ts * 0.15, py + ts * 0.82);
      g.closePath();
      g.fill();
      g.fillStyle = "#f4f4f4";
      g.beginPath();
      g.moveTo(cx, py + ts * 0.18);
      g.lineTo(cx + ts * 0.12, py + ts * 0.4);
      g.lineTo(cx - ts * 0.12, py + ts * 0.4);
      g.closePath();
      g.fill();
    }
    // Rivers on the south and east edges.
    const r = T.riverEdges(m.river[k]);
    if (r.south || r.east) {
      g.strokeStyle = "#3d8fe0";
      g.lineWidth = Math.max(2, ts / 10);
      g.lineCap = "round";
      g.beginPath();
      if (r.south) {
        g.moveTo(px, py + ts);
        g.lineTo(px + ts, py + ts);
      }
      if (r.east) {
        g.moveTo(px + ts, py);
        g.lineTo(px + ts, py + ts);
      }
      g.stroke();
    }
    // Roads: lines toward the neighbours with a road.
    if (m.route[k] >= 0) {
      const rail = tname("RouteType", m.route[k]) === "ROUTE_RAILROAD";
      g.strokeStyle = rail ? "#2c2c2c" : "#8a6a3c";
      g.lineWidth = Math.max(1.5, ts / (rail ? 9 : 12));
      g.lineCap = "round";
      g.beginPath();
      let any = false;
      for (const [dx, dy] of T.DIRS8) {
        const n = T.plotIndex(m, x + dx, y + dy);
        if (n >= 0 && m.route[n] >= 0 && m.vis[n] > 0) {
          g.moveTo(cx, cy);
          g.lineTo(cx + (dx * ts) / 2, cy - (dy * ts) / 2);
          any = true;
        }
      }
      if (!any) {
        g.moveTo(cx - ts * 0.12, cy);
        g.lineTo(cx + ts * 0.12, cy);
      }
      g.stroke();
    }
    // Improvements (lairs and mana nodes marked), bonuses.
    const imp = m.improvement[k];
    if (imp >= 0 && ts >= 12) {
      const type = tname("ImprovementType", imp) ?? "";
      const lair = T.isLair(type, info("ImprovementType", imp));
      const goody = type === "IMPROVEMENT_GOODY_HUT";
      const mana = type.startsWith("IMPROVEMENT_MANA_");
      const label = goody ? "?" : lair ? "☠" : mana ? "✦" : T.abbrev(dname("ImprovementType", imp));
      const s = Math.max(8, ts * 0.38);
      g.fillStyle = goody ? "#e8c547" : lair ? "#5a1414" : mana ? "#6b3fa0" : "rgba(30,25,15,0.78)";
      g.fillRect(px + 2, py + ts - s - 2, s * (label.length > 1 ? 1.35 : 1), s);
      if (ts >= 20) {
        g.fillStyle = goody ? "#2a2000" : "#f2ead8";
        g.font = `${Math.round(s * 0.72)}px ${FONT}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText(label, px + 2 + (s * (label.length > 1 ? 1.35 : 1)) / 2, py + ts - s / 2 - 1.5);
      }
    }
    const bonus = m.bonus[k];
    if (bonus >= 0 && ts >= 12) {
      const type = tname("BonusType", bonus) ?? "";
      const mana = type.startsWith("BONUS_MANA");
      const rad = Math.max(3.5, ts * 0.17);
      const bx = px + ts - rad - 2;
      const by = py + rad + 2;
      g.fillStyle = mana ? "#8e5cd6" : "#f0e2b6";
      g.strokeStyle = "rgba(0,0,0,0.6)";
      g.lineWidth = 1;
      g.beginPath();
      g.arc(bx, by, rad, 0, Math.PI * 2);
      g.fill();
      g.stroke();
      if (ts >= 32) {
        g.fillStyle = mana ? "#fff" : "#3a2a10";
        g.font = `bold ${Math.round(rad * 1.05)}px ${FONT}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText(T.abbrev(dname("BonusType", bonus)), bx, by + 0.5);
      }
    }
    // Borders: a tint, and lines on the edges toward another owner.
    const owner = m.owner[k];
    if (owner >= 0) {
      const col = ownerColor(owner);
      g.fillStyle = withAlpha(col, 0.16);
      g.fillRect(px, py, ts, ts);
      g.strokeStyle = col;
      g.lineWidth = Math.max(1.5, ts / 14);
      g.lineCap = "butt";
      const inset = g.lineWidth / 2 + 0.5;
      g.beginPath();
      for (const [dx, dy] of [[0, 1], [1, 0], [0, -1], [-1, 0]]) {
        const n = T.plotIndex(m, x + dx, y + dy);
        if (n >= 0 && m.owner[n] === owner) continue;
        if (dy === 1) (g.moveTo(px, py + inset), g.lineTo(px + ts, py + inset));
        if (dy === -1) (g.moveTo(px, py + ts - inset), g.lineTo(px + ts, py + ts - inset));
        if (dx === 1) (g.moveTo(px + ts - inset, py), g.lineTo(px + ts - inset, py + ts));
        if (dx === -1) (g.moveTo(px + inset, py), g.lineTo(px + inset, py + ts));
      }
      g.stroke();
    }
    // The political layer: each owner's colour over its plots, unowned land grey.
    if (this.layer === "political") {
      if (owner >= 0) {
        g.fillStyle = withAlpha(ownerColor(owner), 0.62);
        g.fillRect(px, py, ts, ts);
      } else if (pt !== P.ocean) {
        g.fillStyle = "rgba(170,170,170,0.45)";
        g.fillRect(px, py, ts, ts);
      }
    }
    // The yield layer: the terrain darkened, the food, production and commerce of the plots in
    // sight (fogged plots show none: design §4.4).
    if (this.layer === "yield") {
      g.fillStyle = "rgba(0,0,0,0.5)";
      g.fillRect(px, py, ts, ts);
      const y = T.unpackYield(m.yield[k]);
      if (y && ts >= 16) {
        g.font = `bold ${Math.max(8, Math.round(ts * 0.3))}px ${FONT}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        y.forEach((v, j) => {
          if (v <= 0) return;
          g.fillStyle = T.YIELD_COLORS[j];
          g.fillText(String(v), px + ts * (0.22 + 0.28 * j), py + ts * (j === 1 ? 0.62 : 0.4));
        });
      }
    }
    if (vis === 1) {
      g.fillStyle = "rgba(0,0,0,0.42)";
      g.fillRect(px, py, ts, ts);
    }
  }

  /** Calls `f(sx, sy)` with the top-left corner of every copy of plot (x, y) in view. */
  eachCopy(x, y, f) {
    for (const ux of T.copiesOf(this.cam, this.map, this.vw, x)) {
      const p = T.columnToScreen(this.cam, this.vw, this.vh, ux, y);
      if (p.sy > this.vh || p.sy + this.cam.ts < 0) continue;
      f(p.sx, p.sy);
    }
  }

  drawCities(g) {
    const ts = this.cam.ts;
    const mine = me()?.player;
    for (const c of S.cities.values()) {
      this.eachCopy(c.x, c.y, (sx, sy) => {
        const col = ownerColor(c.owner);
        const w = ts * 0.84;
        const hh = Math.max(10, ts * 0.42);
        const bx = sx + (ts - w) / 2;
        const by = sy + ts * 0.12;
        g.fillStyle = darken(col, 0.45);
        g.strokeStyle = col;
        g.lineWidth = 2;
        roundRect(g, bx, by, w, hh, 3);
        g.fill();
        g.stroke();
        g.fillStyle = "#fff";
        g.font = `bold ${Math.round(hh * 0.72)}px ${FONT}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText(c.size === null || c.size === undefined ? "?" : String(c.size), sx + ts / 2, by + hh / 2 + 0.5);
        if (ts >= 16) {
          const label = `${c.capital ? "★ " : ""}${c.name}`;
          g.font = `${ts >= 32 ? 12 : 10}px ${FONT}`;
          const tw = g.measureText(label).width + 8;
          g.fillStyle = "rgba(10,10,12,0.78)";
          roundRect(g, sx + ts / 2 - tw / 2, sy - 9, tw, 14, 6);
          g.fill();
          g.fillStyle = c.owner === mine ? "#ffe9a8" : col;
          g.fillText(label, sx + ts / 2, sy - 1.5);
          if (c.owner === mine && ts >= 32) {
            const prod = c.production ? `${dname(ORDER_KINDS[c.production.order], c.production.type)}${c.production.turns !== null ? ` ${c.production.turns}` : ""}` : c.idle ? "idle!" : "";
            if (prod) {
              g.font = `10px ${FONT}`;
              const pw = g.measureText(prod).width + 6;
              g.fillStyle = c.idle ? "rgba(140,20,20,0.85)" : "rgba(10,10,12,0.7)";
              roundRect(g, sx + ts / 2 - pw / 2, sy + ts - 1, pw, 12, 5);
              g.fill();
              g.fillStyle = "#e8e2d4";
              g.fillText(prod, sx + ts / 2, sy + ts + 5);
            }
          }
        }
      });
    }
  }

  drawUnits(g) {
    const ts = this.cam.ts;
    const mine = me()?.player;
    const sel = this.overlay.units ?? [];
    for (const units of S.unitsAt.values()) {
      const u0 = units[0];
      const top = T.topUnit(units, mine, sel);
      const city = this.map.city[T.plotIndex(this.map, u0.x, u0.y)] >= 0;
      this.eachCopy(u0.x, u0.y, (sx, sy) => {
        const s = city ? ts * 0.5 : ts * 0.7;
        const bx = city ? sx + 1 : sx + (ts - s) / 2;
        const by = city ? sy + ts - s - 1 : sy + (ts - s) / 2 + ts * 0.04;
        const col = ownerColor(top.owner);
        g.fillStyle = col;
        g.strokeStyle = sel.includes(top.id) ? "#fff7a0" : "rgba(0,0,0,0.85)";
        g.lineWidth = sel.includes(top.id) ? 2 : 1;
        roundRect(g, bx, by, s, s, Math.max(2, s / 6));
        g.fill();
        g.stroke();
        if (s >= 9) {
          g.fillStyle = "#101010";
          g.font = `bold ${Math.round(s * 0.42)}px ${FONT}`;
          g.textAlign = "center";
          g.textBaseline = "middle";
          g.fillText(T.abbrev(dname("UnitType", top.type)), bx + s / 2, by + s / 2 + 0.5);
        }
        // Health bar.
        if (top.hp < top.max_hp && s >= 10) {
          const f = Math.max(0, top.hp / top.max_hp);
          g.fillStyle = "#300";
          g.fillRect(bx + 2, by + s - 4, s - 4, 3);
          g.fillStyle = f > 0.6 ? "#4caf50" : f > 0.3 ? "#e0b030" : "#e04030";
          g.fillRect(bx + 2, by + s - 4, (s - 4) * f, 3);
        }
        // Moves pip (own units): green with moves left, red without, grey when asleep.
        if (top.owner === mine && s >= 10) {
          const r = Math.max(2, s / 9);
          const asleep = ASLEEP.includes(tname("ActivityType", top.activity));
          g.fillStyle = asleep ? "#9a9a9a" : top.moves > 0 ? "#3fdc5a" : "#d03a2a";
          g.beginPath();
          g.arc(bx + r + 1.5, by + r + 1.5, r, 0, Math.PI * 2);
          g.fill();
        }
        // Count badge.
        if (units.length > 1 && s >= 10) {
          const t = String(units.length);
          g.font = `bold ${Math.max(9, Math.round(s * 0.3))}px ${FONT}`;
          const w = g.measureText(t).width + 5;
          g.fillStyle = "#f4f0e6";
          roundRect(g, bx + s - w + 3, by - 4, w, Math.max(11, s * 0.34), 4);
          g.fill();
          g.fillStyle = "#111";
          g.fillText(t, bx + s - w / 2 + 3, by - 4 + Math.max(11, s * 0.34) / 2 + 0.5);
        }
      });
    }
  }

  drawOverlays(g) {
    const ts = this.cam.ts;
    const o = this.overlay;
    const outline = (x, y, color, width, dash = []) => {
      this.eachCopy(x, y, (sx, sy) => {
        g.strokeStyle = color;
        g.lineWidth = width;
        g.setLineDash(dash);
        g.strokeRect(sx + width / 2, sy + width / 2, ts - width, ts - width);
        g.setLineDash([]);
      });
    };
    if (Array.isArray(o.targets)) for (const [x, y] of o.targets) outline(x, y, "#4fe3ff", 2, [4, 3]);
    if (o.sel) outline(o.sel.x, o.sel.y, "#ffe14d", 2.5);
    if (o.hover) outline(o.hover.x, o.hover.y, o.targets ? "#4fe3ff" : "rgba(255,255,255,0.7)", 1.5);
    // Waypoints not sent yet: numbered, in order.
    if (Array.isArray(o.waypoints)) {
      o.waypoints.forEach(([x, y], i) => {
        outline(x, y, "#ffe14d", 2, [3, 3]);
        this.eachCopy(x, y, (sx, sy) => {
          g.fillStyle = "#ffe14d";
          g.beginPath();
          g.arc(sx + ts / 2, sy + ts / 2, Math.max(6, ts * 0.18), 0, Math.PI * 2);
          g.fill();
          g.fillStyle = "#111";
          g.font = `bold ${Math.max(9, Math.round(ts * 0.22))}px ${FONT}`;
          g.textAlign = "center";
          g.textBaseline = "middle";
          g.fillText(String(i + 1), sx + ts / 2, sy + ts / 2 + 0.5);
        });
      });
    }
    const path = o.path;
    if (path?.ok && path.nodes?.length > 1) {
      // The path is drawn in one copy: from the selection's copy nearest the centre.
      const n0 = path.nodes[0];
      const start = T.mapToScreen(this.cam, this.map, this.vw, this.vh, n0.x, n0.y);
      let ux = start.sx;
      let prevX = n0.x;
      const pts = [{ sx: start.sx, sy: start.sy, n: n0 }];
      for (const n of path.nodes.slice(1)) {
        let dx = n.x - prevX;
        if (this.map.wrap_x) {
          if (dx > this.map.w / 2) dx -= this.map.w;
          if (dx < -this.map.w / 2) dx += this.map.w;
        }
        ux += dx * ts;
        prevX = n.x;
        pts.push({ sx: ux, sy: T.columnToScreen(this.cam, this.vw, this.vh, 0, n.y).sy, n });
      }
      const danger = path.attack ? "#ff4a3a" : path.declare_war_team !== null ? "#ff9d2e" : null;
      g.strokeStyle = "rgba(0,0,0,0.6)";
      g.lineWidth = 5;
      g.lineJoin = "round";
      g.beginPath();
      pts.forEach((p, i) => (i ? g.lineTo(p.sx + ts / 2, p.sy + ts / 2) : g.moveTo(p.sx + ts / 2, p.sy + ts / 2)));
      g.stroke();
      g.strokeStyle = danger ?? "#f5f1e0";
      g.lineWidth = 2.5;
      g.stroke();
      for (const m of T.pathTurnMarks(path.nodes)) {
        const p = pts[m.i];
        g.fillStyle = m.last ? (danger ?? "#ffe14d") : "#f5f1e0";
        g.beginPath();
        g.arc(p.sx + ts / 2, p.sy + ts / 2, Math.max(6, ts * 0.18), 0, Math.PI * 2);
        g.fill();
        g.fillStyle = "#111";
        g.font = `bold ${Math.max(9, Math.round(ts * 0.22))}px ${FONT}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText(String(m.turn), p.sx + ts / 2, p.sy + ts / 2 + 0.5);
      }
    }
  }

  drawMini() {
    const c = this.mini;
    const g = c.getContext("2d");
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.fillStyle = "#000";
    g.fillRect(0, 0, c.width, c.height);
    const map = this.map;
    if (!map) return;
    if (this.art) this.miniImage = this.art.minimap();
    if (!this.miniImage) {
      const img = new ImageData(map.w, map.h);
      const rgb = (css) => {
        const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(css) ?? /rgb\((\d+),(\d+),(\d+)\)/.exec(css);
        if (!m) return [128, 128, 128];
        return css.startsWith("rgb") ? [+m[1], +m[2], +m[3]] : [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
      };
      for (let y = 0; y < map.h; y++) {
        for (let x = 0; x < map.w; x++) {
          const k = y * map.w + x;
          const o = ((map.h - 1 - y) * map.w + x) * 4;
          let col = [0, 0, 0];
          if (map.vis[k] > 0) {
            col = rgb(T.terrainColor(tname("TerrainType", map.terrain[k])));
            if (map.owner[k] >= 0) {
              const pc = rgb(ownerColor(map.owner[k]));
              col = col.map((v, i) => Math.round(v * 0.4 + pc[i] * 0.6));
            }
            if (map.city[k] >= 0) col = [255, 255, 255];
            if (map.vis[k] === 1) col = col.map((v) => Math.round(v * 0.6));
          }
          img.data[o] = col[0];
          img.data[o + 1] = col[1];
          img.data[o + 2] = col[2];
          img.data[o + 3] = 255;
        }
      }
      const off = document.createElement("canvas");
      off.width = map.w;
      off.height = map.h;
      off.getContext("2d").putImageData(img, 0, 0);
      this.miniImage = off;
    }
    g.imageSmoothingEnabled = false;
    g.drawImage(this.miniImage, 0, 0, c.width, c.height);
    // The viewport.
    const sx = c.width / map.w;
    const sy = c.height / map.h;
    const ts = this.cam.ts;
    const w = (this.vw / ts) * sx;
    const hh = (this.vh / ts) * sy;
    const x = (this.cam.cx - this.vw / 2 / ts) * sx;
    const y = (map.h - this.cam.cy - this.vh / 2 / ts) * sy;
    g.strokeStyle = this.art ? this.art.art.css("8") : "#fff";
    g.lineWidth = Math.max(1, this.dpr);
    for (const off of map.wrap_x ? [-c.width, 0, c.width] : [0]) g.strokeRect(x + off, y, w, hh);
  }

  // ------------------------------------------------------------------------------------------
  // Tooltip

  /** Shows `nodes` in the tooltip near the mouse. */
  showTip(nodes) {
    if (!nodes || (Array.isArray(nodes) && nodes.length === 0)) return this.hideTip();
    this.tip.replaceChildren(...(Array.isArray(nodes) ? nodes : [nodes]));
    this.tip.hidden = false;
    this.placeTip();
  }

  placeTip() {
    if (this.tip.hidden || !this.tipAt) return;
    const r = this.wrap.getBoundingClientRect();
    const tw = this.tip.offsetWidth;
    const th = this.tip.offsetHeight;
    // The city screen's panels cover the sides (CITY_FRAME): its tooltips stay between them.
    const lo = this.overlay.city ? CITY_FRAME.side : 4;
    const hi = this.overlay.city ? r.width - CITY_FRAME.side : r.width - 4;
    let x = this.tipAt.x + 16;
    let y = this.tipAt.y + 16;
    if (x + tw > hi) x = this.tipAt.x - tw - 12;
    if (y + th > r.height - 4) y = Math.max(4, r.height - th - 4);
    this.tip.style.left = `${Math.max(lo, x)}px`;
    this.tip.style.top = `${Math.max(4, y)}px`;
  }

  hideTip() {
    this.tip.hidden = true;
  }
}

function roundRect(g, x, y, w, h, r) {
  g.beginPath();
  g.moveTo(x + r, y);
  g.arcTo(x + w, y, x + w, y + h, r);
  g.arcTo(x + w, y + h, x, y + h, r);
  g.arcTo(x, y + h, x, y, r);
  g.arcTo(x, y, x + w, y, r);
  g.closePath();
}
