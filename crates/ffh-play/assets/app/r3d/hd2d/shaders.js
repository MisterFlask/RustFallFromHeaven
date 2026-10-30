// GLSL of the HD-2D mode (docs/HD2D.md): the sprite cards that stand in the 3D world (the isometric
// renderer's pixel art, facing the camera), their shadows and x-ray ghosts, the light beams over
// mana and towers, and the post passes the mode adds to the 3D pipeline -- light shafts through the
// haze, the region moods, a wide bloom and the tilt-shift blur. The 3D renderer's own shaders
// (../shaders.js, COMMON) supply the noise, light, shadow and fog helpers.

import { SCENE_OUT } from "../shaders.js";

// --------------------------------------------------------------------------------------------
// Sprite cards

/** One card per instance, from a unit quad (aPos.x -0.5..0.5 across, aPos.y 0..1 up). The card's
 *  base point (where its feet or its painted ground's front stand) is iPosRot.xyz.
 *    iPosRot  base x, y, z, layer of the sprite array
 *    iScale   width, height (world units), phase, flags (1 flip, 2 banner, 4 shadow caster,
 *             8 fade the painted ground at the foot, 16 a town: graded lighter, only its warm
 *             lights glow)
 *    iTint    u extent, v extent (of the 256-texel layer), object id, glow
 *    iTeam    owner colour, depth bias (toward the eye, world units)
 *    aExtra   (instanced) anchor across the card (0..1: where the base point is), the map point
 *             whose fog of war it takes (x, y), and 1 when that point is given (else the base's)
 *  uMode: 0 the scene (facing the camera, by uLean), 1 the sun's shadow map (an upright card
 *  turned to the sun), 2 the contact shadow (an ellipse draped on the ground). The card is drawn
 *  where it faces the camera, but its depth is that of an upright card at the same base (pulled
 *  toward the eye by the bias): it hides and is hidden as a standing figure would be, and the
 *  ground behind it never cuts into its tilted top. */
export const SPRITE_VS = `
in vec3 aPos;
in vec4 iPosRot;
in vec4 iScale;
in vec4 iTint;
in vec4 iTeam;
in vec4 aExtra;
uniform float uMode;
uniform float uLean;
uniform sampler2D uHeight;
out vec2 vUv;
out vec2 vCard;
flat out float vLayer;
flat out vec4 vTeam;
flat out float vId;
flat out float vGlow;
flat out vec3 vTint;
flat out float vVis;
flat out float vRim;
flat out float vFade;
flat out float vTown;
out float vDepth;
invariant gl_Position;

// The sun's visibility at a point (the props' shadow map, a few taps): whether a figure stands in
// the shade of a tree or a house. The card is lit as a whole, never by its own shadow.
float sunAt(vec3 wp) {
  if (uShadowOn < 0.5) return 1.0;
  vec4 sp = uShadowMat * vec4(wp, 1.0);
  vec3 c = sp.xyz / sp.w * 0.5 + 0.5;
  if (c.x < 0.0 || c.x > 1.0 || c.y < 0.0 || c.y > 1.0 || c.z > 1.0) return 1.0;
  float s = 0.0;
  for (int i = 0; i < 5; i++) {
    vec2 o = i == 0 ? vec2(0.0) : vec2(i == 1 ? 1.0 : i == 2 ? -1.0 : 0.0, i == 3 ? 1.0 : i == 4 ? -1.0 : 0.0) * 2.5;
    float d = textureLod(uShadowMap, c.xy + o * uShadowTexel, 0.0).r;
    s += c.z - 0.002 > d ? 0.0 : 1.0;
  }
  return s / 5.0;
}

void main() {
  vec3 base = iPosRot.xyz + vec3(uCopyX, 0.0, 0.0);
  float w = iScale.x, h = iScale.y;
  int flags = int(iScale.w + 0.5);
  bool flip = (flags & 1) != 0;
  bool banner = (flags & 2) != 0;
  float u = aPos.x + 0.5;
  float ax = u - aExtra.x;
  vec3 right = vec3(1.0, 0.0, 0.0);
  vec3 camUp = vec3(uView[0][1], uView[1][1], uView[2][1]);
  vec3 up = normalize(mix(vec3(0.0, 1.0, 0.0), camUp, uLean));
  float y = aPos.y;
  // A banner's cloth ripples in the wind (the pole, its left eighth, stands still).
  float sway = 0.0;
  if (banner) sway = sin(uTime * 2.3 + iScale.z * 6.2831 - y * 4.0) * 0.035 * smoothstep(0.15, 0.5, u) * y;
  vec3 p, pu;
  if (uMode > 1.5) {
    // Contact shadow: an ellipse on the ground under the figure, draped on the height field.
    vec2 m = vec2(base.x - uCopyX, -base.z) + vec2(aPos.x * w * 0.95, (aPos.y - 0.5) * w * 0.5);
    float g = max(0.0, texture(uHeight, m / uMapSize).r);
    p = vec3(m.x + uCopyX, max(g, base.y - 0.04) + 0.012, -m.y);
    pu = p;
    vCard = vec2(aPos.x * 2.0, aPos.y * 2.0 - 1.0);
  } else if (uMode > 0.5) {
    // The sun's shadow map: upright and turned to the sun, so its shadow is the figure's outline.
    vec2 s = normalize(uSunDir.xz + vec2(1e-4));
    right = vec3(-s.y, 0.0, s.x);
    p = base + right * (ax * w + sway) + vec3(0.0, y * h, 0.0);
    pu = p;
    vCard = aPos.xy;
  } else {
    p = base + right * (ax * w + sway) + up * (y * h);
    pu = base + right * (ax * w + sway) + vec3(0.0, y * h, 0.0);
    pu += normalize(uEye - pu) * iTeam.a;
    vCard = aPos.xy;
  }
  vUv = vec2((flip ? 1.0 - u : u) * iTint.x, (1.0 - y) * iTint.y);
  vLayer = iPosRot.w;
  vTeam = iTeam;
  vId = iTint.z;
  vGlow = iTint.w;
  // Figures and banners take the rim; buildings' cards carry their own painted edges.
  vRim = iTeam.a > 0.05 ? 0.85 : 0.0;
  vFade = (flags & 8) != 0 ? 0.2 : 0.0;
  vTown = (flags & 16) != 0 ? 1.0 : 0.0;
  // In sight or remembered as the plot it stands for (a building's foot is on the plot's edge).
  vVis = texture(uVis, (aExtra.w > 0.5 ? aExtra.yz : vec2(iPosRot.x, -iPosRot.z)) / uMapSize).r;
  // Tinted by the light where the figure stands (not re-lit): the sun if it reaches the base, the
  // sky's ambient and the local light pools (towns, lava, mana), relative to full sun on open ground
  // -- so in the open a sprite shows its own painted colours, and in a wood's shade it darkens and
  // cools.
  if (uMode < 0.5) {
    // A town's piece takes the sun where its walls and roof are, not at its foot: the town's taller
    // pieces behind it would otherwise shade a whole house by its doorstep.
    bool town = (flags & 16) != 0;
    float sun = sunAt(base + vec3(0.0, town ? 0.55 * h : 0.06, 0.0) + uSunDir * (town ? 0.03 : 0.02));
    float k = 0.85;
    vec3 amb = mix(uGroundColor, uSkyColor, 0.75);
    vec3 full = uSunColor * k + amb;
    vec3 pool = texture(uLight, vec2(iPosRot.x, -iPosRot.z) / uMapSize).rgb;
    vec3 here = uSunColor * k * mix(0.32, 1.0, sun) + amb + pool * 0.25;
    vTint = clamp(here / full, vec3(0.3), vec3(1.15));
  } else vTint = vec3(1.0);
  vec4 c = uViewProj * vec4(p, 1.0);
  vec4 cu = uViewProj * vec4(pu, 1.0);
  vDepth = -(uView * vec4(pu, 1.0)).z;
  gl_Position = vec4(c.xy, clamp(cu.z / cu.w, -1.0, 1.0) * c.w, c.w);
}`;

/** The sprite's texel at vUv in layer vLayer: sharp bilinear -- each texel a flat square, blended
 *  with its neighbour over one screen pixel at its edge -- while a texel covers more than a pixel
 *  (crisp pixels at any magnification, no shimmer when the perspective scales them), mip-filtered
 *  when the card is smaller than the art (no sparkle). The texture holds premultiplied colour. */
const SPRITE_SAMPLE = `
uniform highp sampler2DArray uSprites;
const float LAYER_TEXELS = 256.0;
vec4 spriteTexel(vec2 uv, float layer) {
  vec2 tex = uv * LAYER_TEXELS;
  vec2 dx = dFdx(tex), dy = dFdy(tex);
  vec2 duvx = dFdx(uv), duvy = dFdy(uv);
  vec2 fw = max(abs(dx) + abs(dy), vec2(1e-4));
  vec4 c;
  if (max(fw.x, fw.y) < 1.0) {
    vec2 t = tex - 0.5;
    vec2 i = floor(t);
    vec2 f = t - i;
    vec2 s = clamp((f - 0.5) / fw + 0.5, 0.0, 1.0);
    c = textureLod(uSprites, vec3((i + 0.5 + s) / LAYER_TEXELS, layer), 0.0);
  } else {
    c = textureGrad(uSprites, vec3(uv, layer), duvx, duvy);
  }
  return c;
}`;

/** ACES fitted (COMPOSITE_FS's filmic) inverted: the HDR value the compose pass turns into display
 *  colour y, so a sprite in full light comes out of the pipeline as its art's own colour. */
const INV_FILMIC = `
vec3 invFilmic(vec3 y) {
  y = clamp(y, 0.0, 0.96);
  vec3 a = 2.43 * y - 2.51;
  vec3 b = 0.59 * y - 0.03;
  vec3 c = 0.14 * y;
  return (-b - sqrt(max(b * b - 4.0 * a * c, 0.0))) / (2.0 * a);
}`;

export const SPRITE_FS = `
in vec2 vUv;
in vec2 vCard;
flat in float vLayer;
flat in vec4 vTeam;
flat in float vId;
flat in float vGlow;
flat in vec3 vTint;
flat in float vVis;
flat in float vRim;
flat in float vFade;
flat in float vTown;
in float vDepth;
uniform float uExposure;
uniform float uTownLight;
${SCENE_OUT}
${SPRITE_SAMPLE}
${INV_FILMIC}
void main() {
  oLink = vec4(0.0);
  // Derivatives first, before any discard (undefined in a quad where some pixels have left).
  vec2 ex = dFdx(vUv), ey = dFdy(vUv);
  vec4 t = spriteTexel(vUv, vLayer);
  if (t.a < 0.5) discard;
  // A building's painted ground thins out toward the card's foot (a dither fixed to the art's own
  // texels, so it holds still as the view moves): it melts into the 3D land instead of ending in
  // the hard edge of the isometric diamond.
  if (vFade > 0.0 && vCard.y < vFade && hash12(floor(vUv * 256.0) + vLayer * 7.0) > vCard.y / vFade) discard;
  vec3 c = t.rgb / t.a;
  // A dark rim one screen pixel wide inside the silhouette (from the alpha a pixel away, at the
  // card's own mip level), so a figure reads on any ground, a busy wood included, at any zoom.
  float edge = 0.0;
  for (int i = 0; i < 4; i++) {
    vec2 o = i == 0 ? ex : i == 1 ? -ex : i == 2 ? ey : -ey;
    edge = max(edge, 1.0 - smoothstep(0.3, 0.6, textureGrad(uSprites, vec3(vUv + o, vLayer), ex, ey).a));
  }
  // (Lit towns, uTownLight: each piece outlined too, so the town's roofs read against a wood.)
  float town = vTown * uTownLight;
  c = mix(c, c * vec3(0.22, 0.2, 0.24), edge * max(vRim, town * 0.8));
  // The art's colours, in the light where it stands, then dimmed and cooled where only remembered.
  // A lit town is brighter and never in deep shade.
  vec3 lin = srgb(c) * mix(vTint, max(vTint, vec3(0.95)), town) * (1.0 + 0.24 * town);
  lin = fogOfWar(lin, vVis);
  // The keep mask: 1 for figures and banners (the moods leave them their colours), 0.95 for towns
  // (about half graded), 0.9 for other buildings and props (graded as the land).
  oColor = vec4(invFilmic(lin) / uExposure, vRim > 0.0 ? 1.0 : vTown > 0.5 ? 0.95 : 0.9);
  oND = vec4(0.0, 0.0, 1.0, vDepth);
  // Glowing sprites (mana, gems, crystals): their bright, saturated pixels feed the bloom. A town's
  // only where they are a warm light (windows, fires, braziers: orange to yellow), not its red roofs
  // or blue domes.
  float sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
  float hot = smoothstep(0.35, 0.8, sat) * smoothstep(0.45, 0.9, max(c.r, max(c.g, c.b)));
  // (Only the brightest, most saturated warm pixels: a sandstone town's walls are warm too.)
  if (vTown > 0.5) hot = smoothstep(0.6, 0.78, sat) * smoothstep(0.9, 0.99, c.r) * smoothstep(0.15, 0.3, c.g - c.b) * step(c.g, c.r);
  // A lit town also glows softly as a whole (a little of every pixel feeds the bloom, warmed): it
  // reads from afar as a lit place, the way the look target's golden town does.
  vec3 warm = srgb(c) * vec3(1.0, 0.86, 0.62);
  oEmit = vec4((srgb(c) * hot * vGlow * (1.0 + 1.6 * town) + warm * 0.3 * town) * smoothstep(0.5, 0.9, vVis), vId);
}`;

export const SPRITE_SHADOW_FS = `
in vec2 vUv;
flat in float vLayer;
out vec4 o;
${SPRITE_SAMPLE}
void main() {
  if (spriteTexel(vUv, vLayer).a < 0.5) discard;
  o = vec4(1.0);
}`;

/** The contact shadow: a soft dark ellipse under a figure, laid over the finished frame. */
export const CONTACT_FS = `
in vec2 vCard;
flat in float vVis;
uniform float uContact;
out vec4 o;
void main() {
  float r = length(vCard);
  float a = (1.0 - smoothstep(0.35, 1.0, r)) * 0.42 * uContact * smoothstep(0.5, 0.9, vVis);
  if (a < 0.004) discard;
  o = vec4(0.04, 0.03, 0.06, a);
}`;

/** A figure behind scenery: its outline in the owner's colour, hatched, where something else
 *  stands in front of it (as the 3D figures' GHOST_FS). */
export const SPRITE_GHOST_FS = `
in vec2 vUv;
flat in float vLayer;
flat in vec4 vTeam;
flat in float vId;
in float vDepth;
uniform sampler2D uND;
uniform sampler2D uEmitId;
out vec4 o;
${SPRITE_SAMPLE}
void main() {
  if (vTeam.a < 0.05) discard;
  if (spriteTexel(vUv, vLayer).a < 0.5) discard;
  ivec2 px = ivec2(gl_FragCoord.xy);
  float frontId = texelFetch(uEmitId, px, 0).a;
  float frontDepth = texelFetch(uND, px, 0).w;
  if (abs(frontId - vId) < 1.5e-3 || frontDepth > vDepth - 0.04) discard;
  float hatch = step(0.5, fract((gl_FragCoord.x + gl_FragCoord.y) * 0.25));
  o = vec4(mix(vTeam.rgb * 0.8, vec3(1.0, 0.97, 0.9), 0.25), 0.5 * (0.7 + 0.3 * hatch));
}`;

// --------------------------------------------------------------------------------------------
// Light beams: a soft column of light over a mana node or a mage's tower, added over the frame.

/** Beams reuse the card's vertex shader (iTint.w = the beam's strength, iTeam.rgb its colour). */
export const BEAM_FS = `
in vec2 vCard;
flat in vec4 vTeam;
flat in float vGlow;
flat in float vVis;
uniform float uBeams;
out vec4 o;
void main() {
  float x = abs(vCard.x) * 2.0;
  float core = exp(-x * x * 9.0);
  float halo = exp(-x * x * 2.2) * 0.45;
  float y = vCard.y;
  // Brightest a little above the ground, fading into the sky; it shimmers slowly.
  float along = smoothstep(0.0, 0.12, y) * (1.0 - smoothstep(0.35, 1.0, y));
  float flicker = 0.85 + 0.15 * sin(uTime * 1.7 + y * 9.0 + vTeam.r * 20.0);
  float a = (core + halo) * along * flicker * vGlow * uBeams * smoothstep(0.6, 0.95, vVis);
  if (a < 0.003) discard;
  vec3 c = mix(vTeam.rgb, vec3(1.0, 0.98, 0.92), core * 0.6);
  o = vec4(c * a, a * 0.5);
}`;

// --------------------------------------------------------------------------------------------
// Post passes

/** Light shafts (half resolution), two parts. The sunlit haze: the sun's light scattered along each
 *  view ray through the air near the ground, through the gaps of a high, slowly drifting cloud layer
 *  and past whatever the props' shadow map says stands in the sun's way (anchored to the world:
 *  it holds still when the view pans). Seen from above, though, a ray crosses too little air for
 *  shafts to show, so the beams themselves are painted, as the look target has them: broad soft
 *  beams fanning down from the sun beyond the upper left of the view, brightest near it, over the
 *  far land; their fan turns slowly with the camera's position (a distant layer, not stuck to the
 *  lens) and breathes with the clouds. */
export const RAYS_FS = `
in vec2 vUv;
uniform sampler2D uND;
uniform vec2 uTan;
uniform mat4 uInvView;
uniform float uFarDepth;
uniform float uAspect;
uniform float uBeamZoom; // the painted beams fade out as the camera rises (far zooms)
out vec4 o;
float noise1(float x) {
  float i = floor(x), f = fract(x);
  float u = f * f * (3.0 - 2.0 * f);
  return mix(hash12(vec2(i, 3.7)), hash12(vec2(i + 1.0, 3.7)), u);
}
// The painted beams at screen point uv: a fan from the light beyond the upper left.
float beams(vec2 uv) {
  vec2 d = (uv - vec2(-0.18, 1.32)) * vec2(uAspect, 1.0);
  float r = length(d);
  float a = atan(d.y, d.x);
  float ph = uEye.x * 0.012 - uEye.z * 0.006 + uTime * 0.01;
  float n = noise1(a * 16.0 + ph * 16.0) * 0.6 + noise1(a * 37.0 + ph * 37.0 + 5.0) * 0.4;
  float b = smoothstep(0.48, 0.78, n);
  // Strongest near the light and over the upper (far) part of the view.
  return b * exp(-r * 1.15) * smoothstep(0.15, 0.85, uv.y) * 0.85 * uBeamZoom;
}
float gap(vec3 q) {
  // Where the ray from q to the sun meets the cloud layer (3 plots up).
  vec2 m = vec2(q.x - uCopyX, -q.z) + vec2(uSunDir.x, -uSunDir.z) * (3.0 - q.y) / max(uSunDir.y, 0.2);
  m += vec2(uTime * 0.05, uTime * 0.02);
  // Gaps a plot or two across, so a view ray crosses several shafts and lit columns stand apart
  // from shaded ones.
  float n = mnoise(m, 0.42) * 0.6 + mnoise(m + 17.0, 1.1) * 0.4;
  return smoothstep(0.52, 0.66, n);
}
float occl(vec3 q) {
  if (uShadowOn < 0.5) return 1.0;
  vec4 sp = uShadowMat * vec4(q, 1.0);
  vec3 c = sp.xyz / sp.w * 0.5 + 0.5;
  if (c.x < 0.0 || c.x > 1.0 || c.y < 0.0 || c.y > 1.0 || c.z > 1.0) return 1.0;
  return c.z - 0.003 > textureLod(uShadowMap, c.xy, 0.0).r ? 0.0 : 1.0;
}
void main() {
  vec4 nd = texture(uND, vUv);
  float d = nd.w > 0.0 ? nd.w : uFarDepth;
  vec3 vp = vec3((vUv * 2.0 - 1.0) * uTan * d, -d);
  vec3 P = (uInvView * vec4(vp, 1.0)).xyz;
  vec3 E = uEye;
  vec3 D = P - E;
  // The slab of haze: from the ground up to 1.6 plots.
  float t0 = 0.0, t1 = 1.0;
  if (abs(D.y) > 1e-4) {
    float ta = (1.6 - E.y) / D.y, tb = (-0.1 - E.y) / D.y;
    t0 = clamp(min(ta, tb), 0.0, 1.0);
    t1 = clamp(max(ta, tb), 0.0, 1.0);
  }
  float len = length(D) * (t1 - t0);
  float painted = beams(vUv);
  if (len <= 1e-4) { o = vec4(uSunColor * painted * 0.3, 1.0); return; }
  const int N = 14;
  float jit = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
  float s = 0.0;
  for (int i = 0; i < N; i++) {
    float t = mix(t0, t1, (float(i) + jit) / float(N));
    vec3 q = E + D * t;
    float dens = exp(-max(q.y, 0.0) * 1.6);
    s += gap(q) * occl(q) * dens;
  }
  s /= float(N);
  // Forward scattering: brighter looking toward the sun (the upper left of the view).
  vec3 V = normalize(D);
  float ph = 0.55 + 0.9 * pow(max(dot(V, uSunDir) * 0.5 + 0.5, 0.0), 3.0);
  o = vec4(uSunColor * (s * ph * min(len, 6.0) * 0.035 + painted * 0.3), 1.0);
}`;

/** The HD-2D compose: the 3D compose (occlusion, glow, exposure, filmic) plus the wide bloom and
 *  the light shafts, added in HDR before the filmic curve. */
export const COMPOSE_FS = `
in vec2 vUv;
uniform sampler2D uColor;
uniform sampler2D uAO;
uniform sampler2D uGlow;
uniform sampler2D uBloom;
uniform sampler2D uRays;
uniform float uAOAmount;
uniform float uGlowAmount;
uniform float uBloomAmount;
uniform float uRaysAmount;
uniform float uExposure;
out vec4 o;
vec3 filmic(vec3 x) {
  return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
bool bad(vec3 c) { return any(isnan(c)) || any(isinf(c)); }
void main() {
  vec4 c = texture(uColor, vUv);
  // A guard: a non-finite pixel takes its finite neighbours' mean (the 3D renderer's terrain wrote
  // dashed lines of them on a real GPU, spread by its water, until fixed: docs/R3D.md §5.2).
  if (bad(c.rgb)) {
    vec2 px = 1.0 / vec2(textureSize(uColor, 0));
    vec3 s = vec3(0.0);
    float n = 0.0;
    for (int i = 0; i < 8; i++) {
      vec2 o = vec2(i < 3 ? -1.0 : i < 5 ? 0.0 : 1.0, i == 0 || i == 3 || i == 5 ? -1.0 : i == 1 || i == 6 ? 0.0 : 1.0);
      if (i == 4) o = vec2(0.0, 1.0);
      vec3 q = texture(uColor, vUv + o * px * 1.0).rgb;
      if (!bad(q)) { s += q; n += 1.0; }
    }
    c.rgb = n > 0.0 ? s / n : vec3(0.0);
  }
  float ao = mix(1.0, texture(uAO, vUv).x, uAOAmount);
  vec3 col = c.rgb * ao;
  col += texture(uGlow, vUv).rgb * uGlowAmount;
  col += texture(uBloom, vUv).rgb * uBloomAmount;
  // The shafts lie in the air before the land, not over the figures (keep mask 1).
  col += texture(uRays, vUv).rgb * uRaysAmount * (c.a > 0.99 ? 0.35 : 1.0);
  col = filmic(col * uExposure);
  o = vec4(pow(col, vec3(1.0 / 2.2)), c.a);
}`;

/** The bloom's source (half resolution): the emission and the brightest of the lit frame. */
export const BRIGHT_FS = `
in vec2 vUv;
uniform sampler2D uColor;
uniform sampler2D uEmit;
uniform float uThreshold;
out vec4 o;
// A stray NaN or infinity in the scene (a lone black pixel after the filmic curve) would spread over
// the blur into a black block: those pixels give no light.
vec3 finite(vec3 c) { return any(isnan(c)) || any(isinf(c)) ? vec3(0.0) : min(c, vec3(32.0)); }
void main() {
  vec3 c = finite(texture(uColor, vUv).rgb);
  float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
  vec3 e = finite(texture(uEmit, vUv).rgb);
  o = vec4(e + c * smoothstep(uThreshold, uThreshold * 1.8, l) * 0.6, 1.0);
}`;

/** The region moods (display colour, before the finish pass's grade): the per-plot mood channel
 *  (r hell, g corruption, b lush, a gloom; LINEAR between plot centres, its lookup warped by noise
 *  so a region's edge is organic, not the plot grid) grades the land under each pixel, found from
 *  the depth. Sprites and units (the keep mask, alpha 1) take a lighter grade: they must read on
 *  every ground. */
export const MOOD_FS = `
in vec2 vUv;
uniform sampler2D uSrc;
uniform sampler2D uND;
uniform sampler2D uEmit;
uniform sampler2D uMood;
uniform vec2 uTan;
uniform mat4 uInvView;
uniform float uMoodAmount;
uniform float uLand;
uniform float uFarDepth;
out vec4 o;
vec3 sat(vec3 c, float s) { float l = lum(c); return mix(vec3(l), c, s); }
void main() {
  vec4 src = texture(uSrc, vUv);
  vec3 col = src.rgb;
  vec4 nd = texture(uND, vUv);
  if ((uMoodAmount <= 0.0 && uLand <= 0.0) || nd.w <= 0.0) { o = src; return; }
  vec3 vp = vec3((vUv * 2.0 - 1.0) * uTan * nd.w, -nd.w);
  vec3 wp = (uInvView * vec4(vp, 1.0)).xyz;
  vec2 m = vec2(wp.x, -wp.z);
  if (uWrapX > 0.5) m.x = mod(m.x, uMapSize.x);
  vec2 warp = vec2(mnoise(m, 0.8), mnoise(m + 7.3, 0.8)) - 0.5;
  vec4 md = texture(uMood, (m + warp * 0.7) / uMapSize);
  // Off the map (north and south, or east and west of a map that does not wrap) the land has no
  // mood: the clamped lookup would raise a region's colour into a wall over the void.
  vec2 inside = min(m, uMapSize - m);
  md *= smoothstep(-0.2, 0.3, uWrapX > 0.5 ? inside.y : min(inside.x, inside.y));
  // Figures and banners (keep mask 1) take only a little of the grade, and only its light: they
  // must read on every ground, a red figure on hell's red included. Buildings' cards (0.9) take it
  // whole, as the land they stand on.
  bool figure = src.a > 0.99;
  float k = uMoodAmount;
  float hell = md.r * k, corrupt = md.g * k, lush = md.b * k, gloom = md.a * k;
  float L = lum(col);
  // Lush: deeper greens, golden light, a sap-green cast in the shade (richer in value, not in
  // saturation: the land's palette below keeps foliage an olive).
  vec3 lushC = sat(col, 1.04) * mix(vec3(0.9, 1.02, 0.94), vec3(1.07, 1.04, 0.9), smoothstep(0.25, 0.75, L));
  col = mix(col, lushC, lush);
  // Corruption (the Sheaim): the land darkened and cooled toward violet and slate by a multiply (its
  // contrast kept), its shade deepened, only its bright accents (crystals, magic) pushed to magenta.
  float accent = smoothstep(0.35, 0.7, max(col.r, max(col.g, col.b)) - min(col.r, min(col.g, col.b))) * smoothstep(0.45, 0.8, L);
  vec3 cor = sat(col, 0.6) * vec3(0.74, 0.62, 0.92);
  cor = mix(cor * vec3(0.72, 0.62, 0.92), cor, smoothstep(0.0, 0.45, L));
  cor = mix(cor, col * vec3(1.05, 0.8, 1.2), accent);
  col = mix(col, cor, corrupt);
  // Hell: charred, the land pulled to a dark charcoal-brown, while what glows (the smouldering
  // fissures, lava, fire: the scene's emission) stays hot -- not what is merely light, a road or a
  // sand bank, which would read as lava. Water stays water: dark steel with a red sheen.
  float hot = smoothstep(0.04, 0.3, lum(texture(uEmit, vUv).rgb));
  float wet = smoothstep(0.03, 0.14, col.b - max(col.r, col.g) * 0.92);
  vec3 hel = mix(vec3(L), col, 0.3) * vec3(0.66, 0.44, 0.36);
  hel = mix(hel * vec3(0.7, 0.55, 0.5), hel, smoothstep(0.05, 0.5, L));
  hel = mix(hel, col * vec3(0.6, 0.55, 0.68) + vec3(0.06, 0.0, 0.0), wet);
  hel = mix(hel, col * vec3(1.25, 0.72, 0.4), hot);
  col = mix(col, hel, hell);
  // And its ground is split by glowing cracks: a world-anchored network (the edges of cells about
  // a plot across), pulsing slowly, on the ground only (a surface facing up, not a card).
  if (hell > 0.05 && src.a < 0.85) {
    vec3 wn = mat3(uInvView) * nd.xyz;
    float up = smoothstep(0.5, 0.85, wn.y);
    vec2 c = mcells(m + warp * 0.6, 0.95);
    float e = c.y - c.x;
    float crack = 1.0 - smoothstep(0.008, 0.035, e);
    float halo = 1.0 - smoothstep(0.02, 0.16, e);
    // Only some of the cells' edges are open: the net breaks into separate fissures.
    float open = smoothstep(0.42, 0.62, mnoise(m + 41.0, 1.6));
    float pulse = 0.75 + 0.25 * sin(uTime * 1.2 + c.x * 7.0 + m.x * 0.9);
    vec3 lava = vec3(1.0, 0.46, 0.12);
    float k2 = smoothstep(0.3, 0.85, hell) * up * pulse * open;
    col += lava * halo * k2 * 0.14;
    col = mix(col, mix(lava, vec3(1.0, 0.86, 0.5), crack * 0.4), crack * k2 * 0.75);
  }
  // Gloom (the dark elves, the vampires): cool, dim, half the colour gone.
  vec3 glo = sat(col, 0.62) * vec3(0.84, 0.88, 1.04) * 0.88;
  col = mix(col, glo, gloom);
  // Each mood's air: a tinted haze thickening with distance.
  float far = smoothstep(uFarDepth * 0.35, uFarDepth, nd.w);
  vec3 air = vec3(0.0);
  float wsum = corrupt + hell + gloom + 1e-4;
  air += vec3(0.3, 0.16, 0.42) * corrupt + vec3(0.36, 0.16, 0.1) * hell + vec3(0.26, 0.3, 0.4) * gloom;
  col = mix(col, air / wsum, clamp(far * (corrupt + hell + gloom), 0.0, 1.0) * 0.1);
  // The land's palette (the look target's, measured: docs/HD2D.md §6): earthy and muted -- foliage
  // a half-saturated olive, water a slate teal, the shade a dark olive rather than black -- so that
  // saturation is left to the accents: what glows (mana, crystals, lava, fire), gold, banners.
  if (!figure && uLand > 0.0) {
    float mx = max(col.r, max(col.g, col.b)), mn = min(col.r, min(col.g, col.b));
    float ch = mx - mn;
    // The hue in degrees (0 red, 60 yellow, 120 green, 240 blue).
    float hue = ch < 1e-4 ? 0.0 : mx == col.r ? 60.0 * mod((col.g - col.b) / ch, 6.0) : mx == col.g ? 60.0 * ((col.b - col.r) / ch + 2.0) : 60.0 * ((col.r - col.g) / ch + 4.0);
    float s = ch / max(mx, 1e-4);
    // Earth and foliage: orange-brown through green; water: cyan through blue.
    float earth = smoothstep(8.0, 30.0, hue) * (1.0 - smoothstep(150.0, 180.0, hue));
    float green = smoothstep(62.0, 85.0, hue) * (1.0 - smoothstep(150.0, 180.0, hue));
    float water = smoothstep(170.0, 190.0, hue) * (1.0 - smoothstep(235.0, 255.0, hue)) * (src.a < 0.85 ? 1.0 : 0.3);
    // Accents keep their colour: the emission, and gold (bright, saturated yellow-orange).
    float gold = smoothstep(0.62, 0.82, s) * smoothstep(0.55, 0.8, mx) * (1.0 - smoothstep(52.0, 64.0, hue)) * smoothstep(25.0, 35.0, hue);
    float keepC = max(hot, gold);
    float drop = (earth * 0.32 + water * 0.38 + (1.0 - max(earth, water)) * 0.1) * (1.0 - keepC) * uLand;
    float Lc = lum(col);
    col = mix(vec3(Lc), col, 1.0 - drop);
    // Greens toward olive (a little red), water toward teal (a little green, less blue).
    col.r += (col.g - col.r) * 0.14 * green * (1.0 - keepC) * uLand;
    col += vec3(0.0, 0.02, -0.03) * water * uLand;
    // The land a little lighter in its middle values, and the shade lifted toward a dark olive
    // floor (the target's darkest woods are a deep olive, not black).
    col = mix(col, pow(max(col, vec3(0.0)), vec3(0.84)), uLand);
    float dark = 1.0 - smoothstep(0.02, 0.3, lum(col));
    col = mix(col, max(col, vec3(0.0)) * 0.85 + vec3(0.075, 0.08, 0.05), dark * 0.4 * uLand);
  }
  // A town takes about half the grade: it keeps its colours, its fires their heat.
  if (!figure && src.a > 0.93) col = mix(src.rgb, col, 0.45);
  // A figure keeps its own colours: of the grade, it takes a quarter of the change in value only.
  if (figure) col = src.rgb * mix(1.0, lum(col) / max(lum(src.rgb), 1e-3), 0.25);
  o = vec4(col, src.a);
}`;

/** The tilt-shift (to the canvas): sharp in a band across the view at uFocus (screen uv y), the
 *  blur growing above and below it -- toward the half- then the quarter-resolution blurred frame --
 *  but never to more than a mild softening, so the top of the view still reads. */
export const TILT_FS = `
in vec2 vUv;
uniform sampler2D uSrc;
uniform sampler2D uBlurA;
uniform sampler2D uBlurB;
uniform float uFocus;
uniform float uBand;
uniform float uTilt;
out vec4 o;
void main() {
  vec3 s = texture(uSrc, vUv).rgb;
  float d = max(0.0, abs(vUv.y - uFocus) - uBand);
  // The top of the view (far away) blurs a little more than the bottom.
  float f = clamp(d / (vUv.y > uFocus ? 0.42 : 0.55), 0.0, 1.0) * uTilt;
  vec3 a = texture(uBlurA, vUv).rgb;
  vec3 b = texture(uBlurB, vUv).rgb;
  vec3 c = f < 0.5 ? mix(s, a, f * 2.0) : mix(a, b, f * 2.0 - 1.0);
  o = vec4(c, 1.0);
}`;
