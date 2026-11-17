// A city performing a ritual, on the 3D renderer and so the HD-2D map (docs/HD2D.md §6b, docs/R3D.md
// §5, "ritual"): a column of violet-white light from the city's centre straight up out of the view,
// a dark storm turning high above the city round it (spiral bands of grey-violet cloud, an eye where
// the beam passes, its inner bands lit violet), the storm's shadow on the land round the city and a
// violet pool of light at the beam's foot. The city's view record carries `ritual` (its production's
// name) while it builds one; Map3D.sync hands the cities so marked here (`set`), with violet motes
// rising round them among the particles.
//
// Drawn in the overlay stage (pipeline.js), after the ground marks and the x-ray figures and before
// the particles, over the finished display colour: the storm's darks stay as chosen, not lifted by
// the grade. Per city: the shadow and the light pool (a quad on the ground round the city, depth test
// off, each pixel's surface found from the scene's depth: it falls on the ground, trees and town
// alike), the storm's two cloud layers (horizontal discs, depth-tested, alpha-blended), then the beam
// (a camera-facing strip, depth-tested: the town's front hides its foot) over them. One small
// program, a few quads a city; nothing drawn or bound while no city holds a ritual. Animated by the
// frame's time (a still frame of the same when the map does not animate).

import { Mesh } from "./gl.js";

/** The effect's shape, in plots (world units). */
export const RITUAL_LOOK = {
  shadowR: 2.1, // the storm's shadow on the land: its radius
  stormR: 2.6, // the storm's outer radius
  stormH: 1.9, // its lower layer's height over the city's ground (the upper one 0.32 higher)
  beamHalf: 0.42, // the beam's half-width with its halo (the white core about a quarter of it)
  beamMinPx: 7, // the halo's least half-width on screen, however far out (the core: 2 px)
  mote: "#c49aff",
};

// --------------------------------------------------------------------------------------------
// The program. Part (uPart): 0 the shadow and light pool on the ground, 1 a segment of the beam,
// 2 a layer of the storm. The quad is a strip of rows (aPos in [-1, 1]²): the beam's width is worked
// out per row, its depth changing along it.

export const RITUAL_VS = `
in vec3 aPos;
uniform int uPart;
uniform vec3 uCenter;  // the city's ground point (world, this copy's)
uniform vec4 uShape;   // shadow radius, storm radius, beam half-width, beam least half-width (px)
uniform vec2 uSpan;    // beam: its segment's heights over the ground point; storm: the layer's height, its scale
uniform vec2 uVp;      // the target's size (px)
uniform vec2 uTan;
out vec2 vQ;
out float vH;
out float vHalfPx;
void main() {
  vec3 c = uCenter + vec3(uCopyX, 0.0, 0.0);
  vec3 w;
  vQ = aPos.xy;
  vH = 0.0;
  vHalfPx = 1.0;
  if (uPart == 0) {
    // On the ground round the city; north it runs a plot further (what stands tall there shows
    // higher on the screen), south a little.
    float R = uShape.x * 1.1;
    w = c + vec3(aPos.x * R, 0.0, aPos.y > 0.0 ? -(R + 1.1) : R + 0.3);
  } else if (uPart == 1) {
    float y = mix(uSpan.x, uSpan.y, aPos.y * 0.5 + 0.5);
    vec3 axis = c + vec3(0.0, y, 0.0);
    vec3 toEye = uEye - axis;
    vec3 right = normalize(vec3(toEye.z, 0.0, -toEye.x) + vec3(1e-5, 0.0, 0.0));
    float depth = max(-(uView * vec4(axis, 1.0)).z, 0.05);
    float pxPerUnit = uVp.y / (2.0 * uTan.y * depth);
    float hw = max(uShape.z, uShape.w / pxPerUnit);
    vHalfPx = hw * pxPerUnit;
    vH = y;
    w = axis + right * aPos.x * hw;
  } else {
    float R = uShape.y * uSpan.y;
    w = c + vec3(aPos.x * R, uSpan.x, -aPos.y * R);
  }
  gl_Position = uViewProj * vec4(w, 1.0);
}`;

export const RITUAL_FS = `
in vec2 vQ;
in float vH;
in float vHalfPx;
uniform int uPart;
uniform vec3 uCenter;
uniform vec4 uShape;
uniform vec2 uSpan;
uniform vec2 uVp;
uniform vec2 uTan;
uniform float uSeed;
uniform float uLayer;
uniform sampler2D uND;
uniform mat4 uInvView;
out vec4 o;

const vec3 VIOLET = vec3(0.69, 0.44, 1.0);

float rnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash12(i), hash12(i + vec2(1.0, 0.0)), u.x), mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), u.x), u.y);
}
float rfbm(vec2 p) {
  float s = 0.0, a = 0.5;
  for (int k = 0; k < 5; k++) {
    s += a * rnoise(p);
    p = mat2(1.6, 1.2, -1.2, 1.6) * p + 3.1;
    a *= 0.5;
  }
  return s / 0.97;
}
// The storm's turn at radius r (in storm radii): a logarithmic spiral, its inner bands turning
// faster, the whole slowly anticlockwise seen from above.
float swirl(float r) {
  return 2.3 * log(max(r, 0.03)) - uTime * (0.05 + 0.17 * (1.0 - min(r, 1.0))) + uSeed;
}
mat2 rot(float t) { return mat2(cos(t), sin(t), -sin(t), cos(t)); }

void main() {
  float pulse = 0.88 + 0.12 * sin(uTime * 1.6 + uSeed * 3.0);
  if (uPart == 0) {
    // The surface under this pixel, from the scene's depth.
    vec2 uv = gl_FragCoord.xy / uVp;
    vec4 nd = texture(uND, uv);
    if (nd.w <= 0.0) discard;
    vec3 vp = vec3((uv * 2.0 - 1.0) * uTan * nd.w, -nd.w);
    vec3 wp = (uInvView * vec4(vp, 1.0)).xyz;
    vec3 c = uCenter + vec3(uCopyX, 0.0, 0.0);
    vec2 d = vec2(wp.x - c.x, c.z - wp.z);
    float r = length(d) / uShape.x;
    if (r > 1.25) discard;
    // The storm's shadow: its arms turning as the clouds do, a soft ragged edge.
    float th = swirl(r * uShape.x / uShape.y);
    float a = atan(d.y, d.x);
    float arms = 0.5 + 0.5 * cos(3.0 * (a + th));
    float n = rfbm(rot(th) * d * 1.3 + uSeed);
    float sh = (1.0 - smoothstep(0.55, 1.1, r + (n - 0.5) * 0.35)) * (0.75 + 0.25 * arms);
    // The light pool at the beam's foot, on the ground and on what stands there (to about a
    // plot up: the town's cards).
    float g = exp(-dot(d, d) / 0.45) * (1.0 - smoothstep(0.3, 1.4, wp.y - c.y)) * pulse;
    float hot = exp(-dot(d, d) / 0.03);
    float k = 0.66 * sh * (1.0 - 0.5 * g);
    o = vec4(vec3(0.06, 0.02, 0.11) * k + VIOLET * g * 0.6 + vec3(1.0, 0.9, 1.0) * hot * 0.45, k);
    return;
  }
  if (uPart == 1) {
    float x = abs(vQ.x);
    // A white-hot core (at least 2 px across each side), a violet halo wider than it, both
    // streaming upward.
    float xp = x * vHalfPx;
    float corePx = max(2.0, vHalfPx * 0.27);
    float core = exp(-pow(xp / corePx, 2.0) * 1.4);
    float halo = exp(-x * x * 3.0) * (1.0 - x);
    float s = rnoise(vec2(vQ.x * 2.2 + uSeed, vH * 1.4 - uTime * 2.0));
    float s2 = rnoise(vec2(vQ.x * 5.0 - uSeed, vH * 3.6 - uTime * 3.4));
    vec3 col = vec3(1.0, 0.97, 1.0) * core * (0.82 + 0.18 * s2) + VIOLET * halo * (0.45 + 0.55 * s) * 1.15;
    // A flare where it leaves the ground.
    col += VIOLET * exp(-vH * vH / 0.02) * halo * 0.6;
    col *= pulse;
    o = vec4(col, clamp(core * 0.9 + halo * 0.15, 0.0, 1.0));
    return;
  }
  // A layer of the storm (uLayer 0 the main one, 1 the one over it, 2 the funnel under it, a half
  // as wide): density in the storm's radii.
  float up = uLayer == 1.0 ? 1.0 : 0.0;
  vec2 q = vQ;
  float r = length(q);
  if (r > 1.0) discard;
  float th = swirl(r) + uLayer * 1.9;
  vec2 p = rot(th) * q;
  float a = atan(q.y, q.x);
  float arms = 0.5 + 0.5 * cos(3.0 * (a + th));
  vec2 off = vec2(uSeed * 3.7, uLayer * 11.3);
  float n = rfbm(p * 2.4 + off);
  float n2 = rfbm(p * 6.0 + off + 5.2);
  // Lit from the sun (the upper left of the view, the map's north-west): the billows' faces
  // toward it lighter.
  vec2 sun = rot(th) * normalize(vec2(-1.0, 0.8));
  float lit = clamp((n - rfbm((p - sun * 0.05) * 2.4 + off)) * 9.0, -1.0, 1.0);
  // Dense and darkest in a ring round the eye, thinning to a ragged edge.
  float ring = smoothstep(0.09, 0.3, r) * (1.0 - smoothstep(0.4, 1.0, r));
  float dens = ring * (0.3 + 0.7 * arms) * (0.3 + 1.2 * n) + 0.3 * (n2 - 0.5) * ring;
  dens *= 1.0 - smoothstep(0.6, 0.98, r + (n - 0.5) * 0.4);
  dens *= smoothstep(0.1, 0.22, r + (n2 - 0.5) * 0.08);
  float alpha = clamp(dens * (up > 0.5 ? 1.5 : 2.0), 0.0, up > 0.5 ? 0.8 : 0.95);
  vec3 dark = vec3(0.045, 0.035, 0.07), mid = vec3(0.23, 0.18, 0.31), top = vec3(0.6, 0.53, 0.7);
  vec3 col = mix(dark, mid, smoothstep(0.35, 0.85, n * (0.6 + 0.4 * arms) + 0.15 * up));
  col += (top - mid) * max(lit, 0.0) * 0.9 * (0.5 + 0.5 * up) - mid * max(-lit, 0.0) * 0.6;
  // The inner bands catch the beam's light.
  float inner = exp(-max(r - 0.16, 0.0) / 0.13);
  col += VIOLET * inner * (0.4 + 0.6 * n2) * 1.1 * pulse;
  // Now and then lightning flickers in the ring.
  float flash = step(0.93, hash12(vec2(floor(uTime * 4.0), uSeed + uLayer))) * exp(-pow((r - 0.45) / 0.15, 2.0)) * smoothstep(0.55, 0.8, n2);
  col += VIOLET * flash * 0.5;
  o = vec4(col * alpha, alpha);
}`;

/** A strip of `rows` quads over [-1, 1]². */
function strip(rows) {
  const v = [];
  for (let k = 0; k < rows; k++) {
    const y0 = -1 + (2 * k) / rows, y1 = -1 + (2 * (k + 1)) / rows;
    v.push(-1, y0, 0, 1, y0, 0, 1, y1, 0, -1, y0, 0, 1, y1, 0, -1, y1, 0);
  }
  return new Float32Array(v);
}

/** The cities' view records holding a ritual, from the renderer's dressed cities (map3d.js
 *  `cities`: {city, x, y, z}, the ground point at the city's centre) and the view's current
 *  records by plot (`cityAt`; a dressed record may be the one before this turn's). */
export function ritualCities(cities, cityAt, w) {
  const out = [];
  for (const e of cities) {
    const i = Math.round(-e.z - 0.5) * w + Math.round(e.x - 0.5);
    const c = cityAt.get(i) ?? e.city;
    if (c?.ritual) out.push({ x: e.x, y: e.y, z: e.z, id: c.id ?? 0, ritual: c.ritual });
  }
  return out;
}

/** The violet motes rising round a city holding a ritual (scene.js setParticles emitters). */
export function ritualEmitters(list) {
  const out = [];
  for (const c of list) {
    out.push({ x: c.x, y: c.y + 0.06, z: c.z, color: RITUAL_LOOK.mote, size: 0.013, rise: 0.22, count: 14, spread: 0.7 });
    out.push({ x: c.x, y: c.y + 0.1, z: c.z, color: "#f0e0ff", size: 0.009, rise: 0.4, count: 6, spread: 0.25 });
  }
  return out;
}

export class RitualLayer {
  constructor(gl) {
    this.gl = gl;
    /** The cities holding a ritual: [{x, y, z, id, ritual}] (`ritualCities`). */
    this.list = [];
    this.mesh = null;
    /** The scene (its wrapping copies and view rect), set by Map3D. */
    this.scene = null;
  }

  set(list) {
    this.list = list;
    if (list.length && !this.mesh) this.mesh = new Mesh(this.gl, strip(8), { aPos: 3 });
  }

  active() {
    return this.list.length > 0;
  }

  /** Draws each city's ritual into the pipeline's final target (the overlay stage's state: blend on,
   *  depth test on, depth not written). */
  draw(pl, U, fr) {
    const gl = this.gl;
    const L = RITUAL_LOOK;
    const r = this.scene?.visible;
    const eyeY = U.uEye[1];
    const p = pl.p.ritual.use().set({
      ...U,
      uND: pl.t.nd,
      uInvView: fr.invView,
      uTan: fr.tan,
      uVp: [pl.w, pl.h],
      uShape: [L.shadowR, L.stormR, L.beamHalf, L.beamMinPx],
      uMapSize: this.scene?.world ? [this.scene.world.w, this.scene.world.h] : [1, 1],
    });
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    for (const copy of this.scene?.copies ?? [0]) {
      p.set({ uCopyX: copy });
      for (const c of this.list) {
        const mx = c.x + copy, my = -c.z;
        // (The storm stands high: it shows over the view's top edge from cities south of it.)
        if (r && (mx < r.x0 - 3.5 || mx > r.x1 + 3.5 || my < r.y0 - 7 || my > r.y1 + 3.5)) continue;
        const seed = ((c.id * 0.6180339) % 1) * 6.283;
        // The beam runs up past the top of the view: to nearly the camera's height.
        const top = Math.max(L.stormH + 1.5, (eyeY - c.y) * 0.92);
        p.set({ uCenter: [c.x, c.y, c.z], uSeed: seed });
        gl.disable(gl.DEPTH_TEST);
        p.set({ uPart: 0 });
        this.mesh.draw();
        gl.enable(gl.DEPTH_TEST);
        gl.depthFunc(gl.LEQUAL);
        // The camera over the storm sees the beam's lower part through it, its upper part over it;
        // from under it (the closest zooms), the other way round.
        // The storm's layers, the nearer last; then the beam over them, whole: it is the light, and
        // reads through the cloud it rises through.
        const above = eyeY > c.y + L.stormH + 0.3;
        const layers = [[2, L.stormH - 0.55, 0.5], [0, L.stormH, 1], [1, L.stormH + 0.32, 0.86]];
        for (const [k, h, scale] of above ? layers : layers.reverse()) {
          p.set({ uPart: 2, uLayer: k, uSpan: [h, scale] });
          this.mesh.draw();
        }
        p.set({ uPart: 1, uSpan: [0, top] });
        this.mesh.draw();
      }
    }
  }

  dispose() {
    this.mesh?.dispose();
    this.mesh = null;
  }
}
