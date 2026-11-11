// The Sea layer in the world, on the 3D renderer and so the HD-2D map (docs/NAVAL.md 10, docs/R3D.md
// §5, "sea"): what the flat and isometric maps draw over the ground on their 2D canvas (sea.js) is
// here part of the lit scene, drawn after the water and before the post passes, depth-tested
// against what stands on the water, so ships, coasts and the light lie over it, not under it:
//
// - each sea's name lettered on the water (italic serif, widely spaced, pale), along its long axis,
//   sized to the sea, fading with zoom; faint at the farthest zooms with the layer off;
// - a soft band of the holder's colour along the inside of each held sea's boundary (contested: the
//   two colours in long dashes; patrolled by a team at peace with you: fainter), a faint dotted line
//   between seas, and the outline of the sea hovered or whose panel is open;
// - a fog bank over the water out of your range, thinning over about a plot at the range's edge,
//   drifting while the map animates;
// - the outposts' and supply ships' lanes as ribbons on the water, dashes flowing to the home port,
//   a cut lane ending at a ring-and-cross with the rest of it dotted.
//
// Each sea's buoy and banner is a card of the HD-2D mode (hd2d/dress.js `dressSeaBuoys`) at the spot
// worked out here (`banners`); the HUD (hud.js) keeps the boxes a hover or a click finds a sea by,
// and the cut lanes' labels. Only the map's revealed plots and the view's seas are read: what the
// other renderers show, no more.

import { texture, upload, Mesh } from "./gl.js";
import { SEA } from "./world.js";
import { S, me, isBarbarian } from "../state.js";
import * as T from "../tiles.js";
import { OWN_COLOR, unwrapSegment } from "../supply.js";
import { outpostLanes, laneMark } from "../lanes.js";
import { SEA_LOOK, seaLabelLayout, fitNameHeight, placeNames, seaBuoySpot, seaBand, teamColorOf, seaNameAlpha, seaFogRamp, laneCurve, laneParts, LANE_STYLE, BAND } from "../sea.js";

/** The marks' vertex: position, colour + alpha, (u, v, kind, w) (shaders.js SEA_MARK_VS). */
const MARK_LAYOUT = { aPos: 3, aExtra: 4, aFlow: 4 };
const MARK_FLOATS = 11;
const NAME_FONT = '"Iowan Old Style", "Palatino Linotype", Palatino, "URW Palladio L", "Book Antiqua", Georgia, serif';
/** The atlas's letters (px) and their spacing (a share of the size). */
const NAME_PX = 72;
const NAME_SPACING = 0.2;
const ATLAS_W = 2048;
/** A lane's ribbon: half its width (plots), the height over the water or the ground. */
const LANE_HALF = 0.065;
const LANE_LIFT = 0.025;
/** The name's height over the water (plots): over the surface's small waves. */
const NAME_LIFT = 0.008;

/** [r, g, b] (0..1) of a CSS colour ("#rgb", "#rrggbb", or anything a canvas understands). */
function rgbOf(css) {
  let hex = String(css ?? "");
  if (/^#[0-9a-f]{3}$/i.test(hex)) hex = `#${[...hex.slice(1)].map((c) => c + c).join("")}`;
  if (!/^#[0-9a-f]{6}$/i.test(hex)) {
    try {
      const g = (typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(1, 1) : document.createElement("canvas")).getContext("2d");
      g.fillStyle = hex;
      hex = g.fillStyle;
    } catch {
      hex = "#888888";
    }
    if (!/^#[0-9a-f]{6}$/i.test(hex)) hex = "#888888";
  }
  return [1, 3, 5].map((k) => parseInt(hex.slice(k, k + 2), 16) / 255);
}

function canvas2d(w, h) {
  const c = typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(w, h) : Object.assign(document.createElement("canvas"), { width: w, height: h });
  return [c, c.getContext("2d", { willReadFrequently: true })];
}

export class SeaLayer {
  constructor(gl) {
    this.gl = gl;
    this.plane = new Mesh(gl, new Float32Array([0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1]), { aPos: 3 });
    /** Textures: per plot its sea (NEAREST), the fog bank (filtered), per sea its band. */
    this.plotsTex = this.fogTex = this.infoTex = this.namesTex = null;
    /** Meshes: the names, the lanes. */
    this.names = null;
    this.lanes = null;
    /** Per sea with a name: `{id, corners: [[mx, my] x 4], h}` (the HUD's hover and click boxes). */
    this.labels = [];
    /** Per held, contested or patrolled sea: its buoy `{id, x, y, a, b}` (map point; colours, b
     *  the second banner's when contested, else null). */
    this.banners = [];
    /** The outposts' cut lanes' labels: `{x, y, mark}` (lanes.js laneMark). */
    this.cuts = [];
    this.hasFog = false;
    /** The scene (its map uniforms and wrapping copies), set by Map3D. */
    this.scene = null;
    /** Set by Map3D before each frame: the layer on, the zoom (px a plot), the sea outlined (id or
     *  null), the camera's pitch (radians) and the map rect in view. */
    this.frame = { layer: false, ts: 96, focus: null, pitch: 0.8, rect: null };
    this.key = "";
  }

  /** Rebuilds what the map, the view or the ground changed (Map3D.sync). */
  sync(map, view, world) {
    if (!map || !world) return;
    const seas = view?.seas ?? [];
    // (The map's arrays change with every move: the textures are cheap and rebuilt. The names'
    // atlas only when the seas change -- a sea's revealed plots change its entry's `plots` -- and
    // the lanes when they or the ground change.)
    this.syncPlots(map, seas);
    const key = JSON.stringify([map.w, map.h, seas, view?.me?.player, view?.me?.team]);
    if (key !== this.key) {
      this.key = key;
      this.syncNames(map, seas);
    }
    const laneKey = JSON.stringify([view?.outpost_lanes ?? [], view?.sea_lanes ?? [], view?.me?.player]);
    if (laneKey !== this.laneKey || this.worldSeen !== world) {
      this.laneKey = laneKey;
      this.worldSeen = world;
      this.syncLanes(map, view, world);
    }
  }

  syncPlots(map, seas) {
    const gl = this.gl;
    const w = map.w, h = map.h, n = w * h;
    const wrap = map.wrap_x ? gl.REPEAT : gl.CLAMP_TO_EDGE;
    if (!this.plotsTex || this.plotsTex.w !== w || this.plotsTex.h !== h) {
      for (const t of [this.plotsTex, this.fogTex]) if (t) gl.deleteTexture(t);
      this.plotsTex = texture(gl, w, h, { filter: gl.NEAREST });
      this.fogTex = texture(gl, w, h, { wrapS: wrap });
    }
    const p = new Uint8Array(n * 4);
    const ids = map.sea_id;
    for (let i = 0; i < n; i++) {
      const id = ids ? ids[i] ?? -1 : -1;
      if (id < 0) continue;
      p[i * 4] = (id + 1) & 255;
      p[i * 4 + 1] = (id + 1) >> 8;
    }
    upload(gl, this.plotsTex, p);
    const ramp = seaFogRamp(map);
    const f = new Uint8Array(n * 4);
    let any = false;
    for (let i = 0; i < n; i++) {
      f[i * 4] = ramp[i];
      if (ramp[i]) any = true;
      const s = map.sea?.[i];
      f[i * 4 + 1] = (s !== undefined && s !== null && s >= 0) || (ids && ids[i] >= 0) ? 255 : 0;
    }
    upload(gl, this.fogTex, f);
    this.hasFog = any;
    // Per sea: its band's code and colours (shaders.js SEA_FS uSeaInfo).
    const maxId = Math.max(0, ...seas.map((s) => s.id));
    if (!this.infoTex || this.infoTex.w !== maxId + 1) {
      if (this.infoTex) gl.deleteTexture(this.infoTex);
      this.infoTex = texture(gl, maxId + 1, 2, { filter: gl.NEAREST });
    }
    const info = new Uint8Array((maxId + 1) * 2 * 4);
    const mine = me();
    const myColor = T.playerColor(mine?.player, false);
    const players = [...(S.players?.values?.() ?? [])];
    this.bands = new Map();
    for (const s of seas) {
      const party = teamColorOf(players, s.party, (id) => T.playerColor(id, isBarbarian(id)));
      const b = seaBand(s, myColor, party);
      this.bands.set(s.id, b);
      if (b.code === BAND.NONE) continue;
      const a = rgbOf(b.a), c = rgbOf(b.b);
      const o0 = s.id * 4, o1 = (maxId + 1 + s.id) * 4;
      info.set([...a.map((v) => Math.round(v * 255)), b.code], o0);
      info.set([...c.map((v) => Math.round(v * 255)), Math.round(b.strength * 255)], o1);
    }
    upload(gl, this.infoTex, info);
  }

  /** The names' atlas (each name lettered once, white, with a soft halo in red) and their quads on
   *  the water; the buoys' spots. */
  syncNames(map, seas) {
    const gl = this.gl;
    this.labels = [];
    this.banners = [];
    this.names?.dispose();
    this.names = null;
    const laid = [];
    const [, m] = canvas2d(8, 8);
    const font = `italic 500 ${NAME_PX}px ${NAME_FONT}`;
    m.font = font;
    const spacing = NAME_PX * NAME_SPACING;
    const pad = Math.round(NAME_PX * 0.35);
    const rowH = Math.round(NAME_PX * 1.45);
    for (const sea of seas) {
      const lay = seaLabelLayout(map, sea);
      if (!lay) continue;
      const chars = [...(sea.name ?? "")];
      const widths = chars.map((ch) => m.measureText(ch).width);
      const textW = widths.reduce((a, b) => a + b, 0) + spacing * Math.max(0, chars.length - 1);
      laid.push({ sea, lay, chars, widths, textW, boxW: Math.min(ATLAS_W, Math.ceil(textW + pad * 2)) });
    }
    if (!laid.length) return;
    // Shelf packing into the atlas.
    let x = 0, y = 0;
    for (const L of laid) {
      if (x + L.boxW > ATLAS_W) {
        x = 0;
        y += rowH;
      }
      L.ax = x;
      L.ay = y;
      x += L.boxW;
    }
    const H = y + rowH;
    const [, gA] = canvas2d(ATLAS_W, H);
    const [, gB] = canvas2d(ATLAS_W, H);
    for (const [g, blur] of [[gA, 0], [gB, Math.round(NAME_PX * 0.12)]]) {
      g.font = font;
      g.textBaseline = "middle";
      g.fillStyle = "#ffffff";
      if (blur) g.filter = `blur(${blur}px)`;
      for (const L of laid) {
        let tx = L.ax + pad;
        const ty = L.ay + rowH / 2;
        L.chars.forEach((ch, k) => {
          g.fillText(ch, tx, ty);
          tx += L.widths[k] + spacing;
        });
      }
    }
    const a = gA.getImageData(0, 0, ATLAS_W, H).data;
    const b = gB.getImageData(0, 0, ATLAS_W, H).data;
    const px = new Uint8Array(ATLAS_W * H * 4);
    for (let i = 0; i < ATLAS_W * H; i++) {
      px[i * 4] = Math.min(255, b[i * 4 + 3] * 1.6);
      px[i * 4 + 3] = a[i * 4 + 3];
    }
    if (this.namesTex) gl.deleteTexture(this.namesTex);
    this.namesTex = texture(gl, ATLAS_W, H, { data: px, mips: true });
    const verts = [];
    // The letters' height in plots (sized to the sea, fitted to its length), then moved or made
    // smaller where two names would cover each other.
    const sizes = laid.map((L) => fitNameHeight(L.lay, L.textW / NAME_PX));
    const spots = placeNames(laid.map((L, i) => ({ lay: L.lay, w: (L.textW * sizes[i]) / NAME_PX, h: sizes[i] })));
    laid.forEach((L, i) => {
      const at = spots[i];
      const lay = L.lay;
      L.lay = { ...lay, x: at.x, y: at.y };
      sizes[i] = at.h;
    });
    laid.forEach((L, i) => {
      const { lay, sea } = L;
      const hN = sizes[i];
      // The quad's height from the atlas row's.
      const k = hN / NAME_PX;
      const W = L.boxW * k, Hq = rowH * k;
      const ux = Math.cos(lay.angle), uy = Math.sin(lay.angle);
      const nx = -uy, ny = ux;
      const c = (su, sv) => [lay.x + ux * su * (W / 2) + nx * sv * (Hq / 2), lay.y + uy * su * (W / 2) + ny * sv * (Hq / 2)];
      const corners = [c(-1, -1), c(1, -1), c(1, 1), c(-1, 1)];
      const u0 = L.ax / ATLAS_W, u1 = (L.ax + L.boxW) / ATLAS_W, t0 = L.ay / H, t1 = (L.ay + rowH) / H;
      const uv = [[u0, t1], [u1, t1], [u1, t0], [u0, t0]];
      const v = (j) => [corners[j][0], SEA + NAME_LIFT, -corners[j][1], 1, 1, 1, 1, uv[j][0], uv[j][1], 0, 0];
      for (const j of [0, 1, 2, 0, 2, 3]) verts.push(...v(j));
      // The box a hover or a click finds the sea by: the letters' (not the quad's padding).
      const tw = L.textW * k;
      const hc = (su, sv) => [lay.x + ux * su * (tw / 2) + nx * sv * (hN / 2), lay.y + uy * su * (tw / 2) + ny * sv * (hN / 2)];
      this.labels.push({ id: sea.id, corners: [hc(-1, -1), hc(1, -1), hc(1, 1), hc(-1, 1)], h: hN });
      const band = this.bands?.get(sea.id);
      if (band && band.code !== BAND.NONE) {
        const spot = seaBuoySpot(map, sea, { ...lay, h: hN }, tw);
        this.banners.push({ id: sea.id, x: spot.x, y: spot.y, a: band.a, b: band.code === BAND.CONTESTED ? band.b : null });
      }
    });
    this.names = new Mesh(gl, new Float32Array(verts), MARK_LAYOUT);
  }

  /** The lanes' ribbons and cut marks. */
  syncLanes(map, view, world) {
    const gl = this.gl;
    this.lanes?.dispose();
    this.lanes = null;
    this.cuts = [];
    const verts = [];
    const ground = (mx, my) => {
      const x = map.wrap_x ? ((mx % map.w) + map.w) % map.w : mx;
      return Math.max(SEA, world.heightAt(x, my)) + LANE_LIFT;
    };
    const ribbon = (pts, col, alpha, kind, dir) => {
      const c = laneCurve(pts);
      if (c.length < 2) return;
      // Subdivided so the ribbon follows the ground where it comes ashore at a port.
      const p = [];
      for (let k = 0; k + 1 < c.length; k++) {
        const a = c[k], b = c[k + 1];
        const n = Math.max(1, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y) / 0.2));
        for (let j = 0; j < n; j++) {
          const t = j / n;
          p.push({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, s: a.s + (b.s - a.s) * t });
        }
      }
      p.push(c[c.length - 1]);
      const [r, g, b] = rgbOf(col);
      const side = [];
      for (let k = 0; k < p.length; k++) {
        const q0 = p[Math.max(0, k - 1)], q1 = p[Math.min(p.length - 1, k + 1)];
        let tx = q1.x - q0.x, ty = q1.y - q0.y;
        const l = Math.hypot(tx, ty) || 1;
        tx /= l;
        ty /= l;
        const h = ground(p[k].x, p[k].y);
        const w = kind === 2 ? LANE_HALF * 0.8 : LANE_HALF;
        side.push([
          [p[k].x - ty * w, h, -(p[k].y + tx * w), r, g, b, alpha, p[k].s, -1, kind, kind === 2 ? w : dir],
          [p[k].x + ty * w, h, -(p[k].y - tx * w), r, g, b, alpha, p[k].s, 1, kind, kind === 2 ? w : dir],
        ]);
      }
      for (let k = 0; k + 1 < side.length; k++) {
        const [a0, a1] = side[k], [b0, b1] = side[k + 1];
        verts.push(...a0, ...a1, ...b1, ...a0, ...b1, ...b0);
      }
    };
    const cutMark = (mx, my, col) => {
      const R = 0.17;
      const [r, g, b] = rgbOf(col);
      const h = ground(mx, my) + 0.004;
      const v = (su, sv) => [mx + su * R, h, -(my + sv * R), r, g, b, 1, su, sv, 3, 0];
      const q = [v(-1, -1), v(1, -1), v(1, 1), v(-1, 1)];
      for (const j of [0, 1, 2, 0, 2, 3]) verts.push(...q[j]);
    };
    // Outposts' lanes: each segment from the port's end toward the outpost (the dashes flow toward
    // its first point); a cut lane's part beyond the cut dotted, its mark where it enters the sea.
    for (const L of outpostLanes(view)) {
      for (const seg of L.segments) ribbon(unwrapSegment(seg, seg[0][0], map), L.color, 1, 1, 1);
      for (const seg of L.cutSegments) ribbon(unwrapSegment(seg, seg[0][0], map), L.color, 0.75, 2, 0);
      if (L.cut) {
        cutMark(L.cut.x + 0.5, L.cut.y + 0.5, LANE_STYLE.cutColor);
        this.cuts.push({ x: L.cut.x, y: L.cut.y, mark: laneMark(L.cut) });
      }
    }
    // Your supply ships' lanes: from the ship to the port (the dashes flow toward its last point).
    for (const lane of view?.sea_lanes ?? []) {
      const path = lane.path ?? [];
      if (!path.length) continue;
      const parts = laneParts(lane);
      const un = unwrapSegment(path, path[0][0], map);
      const k = parts.open.length;
      if (k > 1) ribbon(un.slice(0, k), OWN_COLOR, 1, 1, -1);
      if (parts.cut) {
        ribbon(un.slice(k - 1), OWN_COLOR, 0.75, 2, 0);
        cutMark(un[k - 1][0] + 0.5, un[k - 1][1] + 0.5, LANE_STYLE.cutColor);
      }
    }
    if (verts.length) this.lanes = new Mesh(gl, new Float32Array(verts), MARK_LAYOUT);
    this.laneCount = verts.length / MARK_FLOATS;
  }

  /** Whether the lit pass draws anything: the Sea layer on, or the names showing faintly with the
   *  layer off. */
  active() {
    const f = this.frame;
    if (!f.rect || !this.plotsTex) return false;
    return f.layer || (!!this.names && seaNameAlpha(f.ts, false) > 0);
  }

  /** The lit pass (Pipeline.render, after the water): the fog bank and the names. `pl` the
   *  pipeline, `U` the frame's uniforms, `fr` its state. */
  draw(pl, U, fr) {
    const gl = this.gl;
    const f = this.frame;
    const scene = this.scene;
    if (!scene?.world) return;
    gl.bindFramebuffer(gl.FRAMEBUFFER, pl.f.sea);
    gl.viewport(0, 0, pl.w, pl.h);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
    gl.depthMask(false);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.colorMask(true, true, true, false);
    gl.enable(gl.POLYGON_OFFSET_FILL);
    gl.polygonOffset(-1, -2);
    const r = f.rect;
    const rect = [r.x0 - 1, r.y0 - 1, r.x1 + 1, r.y1 + 1];
    const MU = { ...U, ...scene.mapUniforms(), uExposure: fr.exposure, uCopyX: 0 };
    const p = pl.p.sea;
    if (f.layer && this.hasFog) {
      const lift = SEA_LOOK.fog.lift;
      p.use().set(MU).set({
        uMode: 1,
        uLift: SEA + lift,
        uRect: rect,
        uSeaFog: this.fogTex,
        uSceneND: pl.t.nd,
        uFogCol: rgbOf(SEA_LOOK.fog.color),
        // The bank's depth over open water (view depth: the lift seen along the camera's pitch).
        uFogLook: [SEA_LOOK.fog.alpha, (lift / Math.max(0.2, Math.sin(f.pitch))) * 0.85, 1],
      });
      this.plane.draw();
    }
    const nameAlpha = this.names ? seaNameAlpha(f.ts, f.layer) : 0;
    if (nameAlpha > 0) {
      const q = pl.p.seaMark;
      q.use().set(MU).set({ uDisplay: 0, uNames: this.namesTex, uNameAlpha: nameAlpha, uNameCol: rgbOf(SEA_LOOK.name.color) });
      for (const copy of scene.copies ?? [0]) {
        q.set({ uCopyX: copy });
        this.names.draw();
      }
    }
    gl.disable(gl.POLYGON_OFFSET_FILL);
    gl.colorMask(true, true, true, true);
    gl.disable(gl.BLEND);
    gl.depthMask(true);
  }

  /** Whether the overlay stage draws the surface: the Sea layer on, or a sea outlined. */
  activeOverlay() {
    const f = this.frame;
    return !!f.rect && !!this.plotsTex && (f.layer || (f.focus !== null && f.focus !== undefined));
  }

  /** The surface (Pipeline.render's overlay stage, after the post passes, its state set: depth
   *  tested, offset toward the eye, blending): the holders' bands, the dotted lines between seas,
   *  the outlined sea; the lanes. */
  drawOverlay(pl, U, fr) {
    const gl = this.gl;
    const f = this.frame;
    const scene = this.scene;
    if (!scene?.world) return;
    const r = f.rect;
    const focus = f.focus === null || f.focus === undefined ? 0 : f.focus + 1;
    gl.depthFunc(gl.LEQUAL);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    pl.p.sea.use().set({ ...U, ...scene.mapUniforms(), uExposure: fr.exposure, uCopyX: 0 }).set({
      uMode: 0,
      uLift: SEA + NAME_LIFT * 0.5,
      uRect: [r.x0 - 1, r.y0 - 1, r.x1 + 1, r.y1 + 1],
      uSeaPlots: this.plotsTex,
      uSeaInfo: this.infoTex,
      uBands: f.layer ? 1 : 0,
      uFocus: focus,
      uBandLook: [SEA_LOOK.band.width, SEA_LOOK.band.alpha],
      uOutline: rgbOf(SEA_LOOK.outline),
    });
    this.plane.draw();
    if (f.layer && this.lanes) {
      const q = pl.p.seaMark;
      q.use().set({ ...U, ...scene.mapUniforms(), uExposure: fr.exposure }).set({ uDisplay: 1, uNames: this.namesTex, uLaneMotion: 0.5 });
      for (const copy of scene.copies ?? [0]) {
        q.set({ uCopyX: copy });
        this.lanes.draw();
      }
    }
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  dispose() {
    const gl = this.gl;
    this.plane.dispose();
    this.names?.dispose();
    this.lanes?.dispose();
    for (const t of [this.plotsTex, this.fogTex, this.infoTex, this.namesTex]) if (t) gl.deleteTexture(t);
  }
}
