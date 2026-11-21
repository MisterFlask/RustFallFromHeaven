// A city performing a ritual, on the 3D renderer and so the HD-2D map (docs/HD2D.md §6b, docs/R3D.md
// §5, "ritual"): a column of light from the city's centre straight up out of the view, a storm
// turning high above the city round it (spiral bands of cloud, an eye where the beam passes, its
// inner bands lit by the beam), the storm's shadow on the land round the city and a pool of light at
// the beam's foot. The city's view record carries `ritual` (its production's name) and `ritual_type`
// (the project's type) while it builds one; Map3D.sync hands the cities so marked here (`set`), with
// motes rising round them among the particles.
//
// Each ritual has its own look (RITUAL_LOOKS, chosen by RITUAL_LOOK_OF): the beam's colours, the
// storm's cloud and glow, its turn, arms and lightning, the shadow's tint and the motes (colours,
// sparks or smoke). A project with no entry takes the arcane look (violet).
//
// The storm hides what lies under it, so it fades as the camera rises (zoomFade): whole at the
// closest zooms, a light veil at the gameplay zoom, nearly gone far out; the shadow fades with it and
// the beam a little (it stays the ritual's mark on the far view).
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
  // The zoom fade (pixels a plot, U.uPx): the storm whole from `near` in, at its least (`floor`)
  // from `far` out; the shadow and the beam keep more.
  fade: { far: 64, near: 200, storm: 0.06, shadow: 0.2, beam: 0.55 },
};

// --------------------------------------------------------------------------------------------
// The looks. Colours are display colours (the effect is drawn over the graded frame). Per look:
//   core, halo   the beam's white-hot core and its halo
//   glow         the light the beam casts: the storm's inner bands, the pool at its foot, lightning
//   dark, mid, top   the storm's cloud: its shadowed depths, its body, the faces toward the sun
//   shade, shadeK    the storm's shadow on the land: its tint and strength (0..1)
//   spin         the storm's turn (1 the arcane one's speed; negative clockwise seen from above)
//   arms         its spiral arms (a whole number)
//   bolts        how often lightning flickers in it (a share of quarter-seconds; 0 none)
//   dens         how thick its cloud (1 the arcane one's)
//   motes        the particles rising round the city: [colour, size, rise, count, spread, smoke]
//                (scene.js setParticles; smoke: a billow that swells and thins)

export const RITUAL_LOOKS = {
  arcane: {
    core: "#fff8ff", halo: "#b070ff", glow: "#b070ff",
    dark: "#0b0912", mid: "#3b2e4f", top: "#9987b3", shade: "#0f051c", shadeK: 0.66,
    spin: 1, arms: 3, bolts: 0.07, dens: 1,
    motes: [["#c49aff", 0.013, 0.22, 14, 0.7], ["#f0e0ff", 0.009, 0.4, 6, 0.25]],
  },
  // Rites of Oghma: the arcane storm, its light split into the mana's colours.
  prism: {
    core: "#ffffff", halo: "#7f86ff", glow: "#52d8ff",
    dark: "#0a0a16", mid: "#2f3156", top: "#8f94c4", shade: "#060a1c", shadeK: 0.6,
    spin: 1.2, arms: 4, bolts: 0.06, dens: 1,
    motes: [["#c49aff", 0.012, 0.24, 7, 0.7], ["#7ae0ff", 0.012, 0.3, 7, 0.6], ["#ffe080", 0.01, 0.36, 6, 0.5]],
  },
  // Wellspring (the Amurites): blue mana welling up.
  mana: {
    core: "#f2fbff", halo: "#4fb6ff", glow: "#59c8ff",
    dark: "#07111c", mid: "#23405a", top: "#8fb4cf", shade: "#031020", shadeK: 0.55,
    spin: 1, arms: 4, bolts: 0.04, dens: 0.95,
    motes: [["#7fd4ff", 0.013, 0.3, 16, 0.6], ["#e0f6ff", 0.009, 0.5, 8, 0.3]],
  },
  // Hyborem's Whisper, Elegy of the Sheaim: hellfire under a black-red storm.
  hellfire: {
    core: "#fff0c8", halo: "#ff4a14", glow: "#ff5a1a",
    dark: "#120404", mid: "#3d1410", top: "#8a4a3a", shade: "#1a0300", shadeK: 0.72,
    spin: -1.2, arms: 3, bolts: 0.12, dens: 1.1,
    motes: [["#ff7a2a", 0.012, 0.32, 14, 0.7], ["#ffd080", 0.008, 0.5, 6, 0.3], ["#2a1a16", 0.03, 0.12, 5, 0.5, true]],
  },
  // For the Horde, Purge the Unfaithful: fire and smoke.
  ember: {
    core: "#fff2d0", halo: "#ff8a2a", glow: "#ff9a3a",
    dark: "#140c08", mid: "#3e2a1e", top: "#8d6a52", shade: "#180800", shadeK: 0.6,
    spin: 1.1, arms: 3, bolts: 0.05, dens: 1,
    motes: [["#ff9a3a", 0.012, 0.34, 14, 0.7], ["#ffe0a0", 0.008, 0.5, 5, 0.3], ["#3a2a22", 0.03, 0.12, 6, 0.6, true]],
  },
  // Blood of the Phoenix: a storm of fire, lit from within.
  phoenix: {
    core: "#fffbe0", halo: "#ffb030", glow: "#ffc040",
    dark: "#3a1a08", mid: "#a0501c", top: "#ffc070", shade: "#200800", shadeK: 0.4,
    spin: 1.4, arms: 2, bolts: 0.06, dens: 0.95,
    motes: [["#ffb030", 0.014, 0.36, 12, 0.7], ["#fff0b0", 0.009, 0.5, 6, 0.3], ["#ff6020", 0.012, 0.28, 6, 0.6]],
  },
  // Divine Retribution, Glory Everlasting, Hallowing of the Elohim: a white-gold storm of light.
  holy: {
    core: "#ffffff", halo: "#ffe08a", glow: "#ffe9a8",
    dark: "#6b6250", mid: "#c9bfa0", top: "#fff8e0", shade: "#2a2410", shadeK: 0.35,
    spin: 0.7, arms: 4, bolts: 0.05, dens: 0.85,
    motes: [["#fff2c0", 0.012, 0.24, 12, 0.7], ["#ffffff", 0.009, 0.42, 6, 0.3]],
  },
  // Sanctuary (the Elohim): pale, still, no lightning.
  sanctuary: {
    core: "#ffffff", halo: "#bcdcff", glow: "#d6e8ff",
    dark: "#5a6474", mid: "#b6c4d6", top: "#f4f8ff", shade: "#10182a", shadeK: 0.3,
    spin: 0.5, arms: 5, bolts: 0, dens: 0.8,
    motes: [["#e6f2ff", 0.012, 0.18, 12, 0.7], ["#ffffff", 0.008, 0.3, 6, 0.35]],
  },
  // Religious Fervor (the Malakim): the sun's fire, turning fast.
  sun: {
    core: "#fffef0", halo: "#ffc23a", glow: "#ffd04a",
    dark: "#4a3010", mid: "#b88a3a", top: "#ffe6a0", shade: "#2a1400", shadeK: 0.4,
    spin: 1.6, arms: 5, bolts: 0.03, dens: 0.9,
    motes: [["#ffd060", 0.013, 0.3, 14, 0.7], ["#fff4c0", 0.009, 0.48, 6, 0.3]],
  },
  // Ardor (the Grigori): amber light under dusty cloud.
  ardor: {
    core: "#fff4e0", halo: "#ff9a40", glow: "#ffb060",
    dark: "#2a1a10", mid: "#6e5038", top: "#c8a078", shade: "#1a0c00", shadeK: 0.5,
    spin: 1, arms: 3, bolts: 0.02, dens: 0.95,
    motes: [["#ffb060", 0.012, 0.28, 14, 0.7], ["#ffe8c0", 0.009, 0.42, 6, 0.3]],
  },
  // March of the Trees, Nature's Revolt: green light, leaves on the wind.
  verdant: {
    core: "#f4fff0", halo: "#5ee06a", glow: "#6ef07a",
    dark: "#08140a", mid: "#24452a", top: "#84b088", shade: "#031406", shadeK: 0.6,
    spin: 0.8, arms: 3, bolts: 0.02, dens: 1,
    motes: [["#8af07a", 0.013, 0.24, 10, 0.75], ["#d8ffb0", 0.009, 0.4, 5, 0.3], ["#c8a040", 0.012, 0.18, 6, 0.8]],
  },
  // Genesis: a pale green-gold sky, the land renewed.
  genesis: {
    core: "#fffff0", halo: "#b8f060", glow: "#d0ff80",
    dark: "#2e4a20", mid: "#7ea860", top: "#e0f6b0", shade: "#102000", shadeK: 0.35,
    spin: 0.6, arms: 4, bolts: 0, dens: 0.85,
    motes: [["#c8ff80", 0.013, 0.22, 12, 0.75], ["#fff8c0", 0.009, 0.36, 6, 0.3]],
  },
  // Raging Seas (the Lanun): a sea storm, clockwise, much lightning, spray.
  sea: {
    core: "#f0ffff", halo: "#40d8e8", glow: "#60e0f0",
    dark: "#060c12", mid: "#26384a", top: "#8aa4b8", shade: "#001018", shadeK: 0.7,
    spin: -1.5, arms: 4, bolts: 0.15, dens: 1.15,
    motes: [["#7ae8ff", 0.012, 0.36, 12, 0.7], ["#e0ffff", 0.008, 0.5, 6, 0.3], ["#9ab8c8", 0.026, 0.14, 5, 0.6, true]],
  },
  // Frozen Passage, The White Hand, The Deepening, The Draw: ice and snow.
  ice: {
    core: "#ffffff", halo: "#8fd8ff", glow: "#b0e4ff",
    dark: "#3a4a5a", mid: "#8aa0b4", top: "#eef6ff", shade: "#0a1a2a", shadeK: 0.45,
    spin: -0.6, arms: 4, bolts: 0.02, dens: 0.95,
    motes: [["#e8f6ff", 0.01, 0.12, 20, 0.85], ["#ffffff", 0.008, 0.2, 8, 0.5]],
  },
  // Stir from Slumber, Ascension: Auric's cold sky, gold in it, lightning.
  auric: {
    core: "#ffffff", halo: "#a8e0ff", glow: "#ffe7a0",
    dark: "#2a3446", mid: "#7088a8", top: "#e8f0ff", shade: "#0a1222", shadeK: 0.5,
    spin: 1.2, arms: 2, bolts: 0.12, dens: 1,
    motes: [["#ffffff", 0.01, 0.16, 12, 0.8], ["#ffe7a0", 0.011, 0.36, 8, 0.4]],
  },
  // Samhain: a dark winter sky, the bonfire's orange in it.
  samhain: {
    core: "#fff0d8", halo: "#ff8a30", glow: "#ff9a40",
    dark: "#0a0c18", mid: "#283048", top: "#7080a0", shade: "#04060e", shadeK: 0.65,
    spin: -0.9, arms: 3, bolts: 0.04, dens: 1,
    motes: [["#ff9a40", 0.012, 0.3, 10, 0.6], ["#e8f0ff", 0.009, 0.14, 12, 0.85]],
  },
  // Into the Mist (the Sidar): grey mist, slow, no lightning.
  mist: {
    core: "#f4f8ff", halo: "#a8b8d0", glow: "#c0cce0",
    dark: "#4a525e", mid: "#8a94a2", top: "#d4dae2", shade: "#101418", shadeK: 0.4,
    spin: 0.4, arms: 2, bolts: 0, dens: 0.8,
    motes: [["#c8d0dc", 0.03, 0.1, 10, 0.8, true], ["#eef2f8", 0.009, 0.24, 6, 0.4]],
  },
  // The Tithe (the Calabim): blood under a black storm.
  blood: {
    core: "#ffe0e0", halo: "#e0142a", glow: "#ff2a3a",
    dark: "#100204", mid: "#3a0c14", top: "#7a3a44", shade: "#1a0006", shadeK: 0.7,
    spin: -1, arms: 3, bolts: 0.05, dens: 1.05,
    motes: [["#ff3040", 0.012, 0.24, 14, 0.7], ["#ffb0b0", 0.008, 0.4, 5, 0.3]],
  },
  // The Cull (the Doviello): red light under a grey winter storm.
  cull: {
    core: "#ffe8e0", halo: "#ff3a2a", glow: "#ff5040",
    dark: "#2a2a30", mid: "#6a6a74", top: "#c8c8d0", shade: "#0e0e14", shadeK: 0.55,
    spin: -1.1, arms: 3, bolts: 0.04, dens: 1,
    motes: [["#ff5040", 0.012, 0.28, 10, 0.6], ["#f0f0f0", 0.009, 0.12, 12, 0.85]],
  },
  // Veil of Night (the Svartalfar): near-black cloud, a dim violet beam, silver specks.
  night: {
    core: "#d8d0ff", halo: "#5a3aa8", glow: "#6a4ac0",
    dark: "#020206", mid: "#12101e", top: "#3a3650", shade: "#000004", shadeK: 0.8,
    spin: 0.7, arms: 3, bolts: 0, dens: 1.15,
    motes: [["#c0c8ff", 0.008, 0.2, 14, 0.8], ["#8070d0", 0.011, 0.3, 6, 0.4]],
  },
  // Worldbreak (the Sheaim): fel green, ash.
  fel: {
    core: "#f4ffe0", halo: "#8aff3a", glow: "#9aff40",
    dark: "#060a04", mid: "#1e2a14", top: "#5e6e48", shade: "#081000", shadeK: 0.72,
    spin: -1.3, arms: 3, bolts: 0.14, dens: 1.1,
    motes: [["#a0ff50", 0.012, 0.3, 12, 0.7], ["#e0ffb0", 0.008, 0.46, 5, 0.3], ["#2a3020", 0.03, 0.12, 6, 0.6, true]],
  },
  // Bane Divine: a plague's yellow-green.
  bane: {
    core: "#fcffe0", halo: "#c8e030", glow: "#d4ec40",
    dark: "#0a0a02", mid: "#2a2a10", top: "#6e6a3a", shade: "#0a0a00", shadeK: 0.72,
    spin: -0.8, arms: 3, bolts: 0.06, dens: 1.1,
    motes: [["#d4ec40", 0.012, 0.22, 12, 0.7], ["#3a3a20", 0.03, 0.1, 6, 0.6, true]],
  },
  // The Delve, Pact of the Nilhorn: the deep earth's dust and gold.
  earth: {
    core: "#fff4dc", halo: "#e0a040", glow: "#f0b050",
    dark: "#1a120a", mid: "#4e3a26", top: "#9a8060", shade: "#140a00", shadeK: 0.6,
    spin: 0.9, arms: 2, bolts: 0.03, dens: 1,
    motes: [["#f0c060", 0.011, 0.26, 10, 0.6], ["#6a5640", 0.032, 0.1, 7, 0.7, true]],
  },
  // Upgrade (the Luchuirp): the forge's sparks, rune-light in the smoke.
  forge: {
    core: "#ffffff", halo: "#4ae0c8", glow: "#ff9a40",
    dark: "#0e0c0a", mid: "#3a3630", top: "#8a8478", shade: "#0a0806", shadeK: 0.6,
    spin: 1, arms: 3, bolts: 0.08, dens: 1,
    motes: [["#ffa040", 0.01, 0.46, 12, 0.5], ["#5af0d0", 0.012, 0.26, 8, 0.7], ["#3a3632", 0.03, 0.12, 5, 0.5, true]],
  },
  // Revelry (the Balseraphs): a carnival's colours, turning fast.
  revel: {
    core: "#fff0ff", halo: "#ff4ab8", glow: "#ff6ad0",
    dark: "#1a0a20", mid: "#4a2a5a", top: "#b08ac8", shade: "#12061a", shadeK: 0.5,
    spin: 2, arms: 5, bolts: 0.06, dens: 0.95,
    motes: [["#ff6ad0", 0.012, 0.3, 6, 0.7], ["#ffe040", 0.012, 0.3, 6, 0.7], ["#40e0ff", 0.012, 0.3, 6, 0.7], ["#80ff80", 0.012, 0.3, 6, 0.7]],
  },
  // Rally, Birthright Regained (the Bannor): gold light, the banner's red.
  banner: {
    core: "#ffffff", halo: "#ffcc40", glow: "#ffd860",
    dark: "#20141a", mid: "#5a4048", top: "#b0a0a4", shade: "#10060a", shadeK: 0.5,
    spin: 1, arms: 4, bolts: 0.04, dens: 0.95,
    motes: [["#ffd040", 0.012, 0.28, 12, 0.7], ["#ff4a3a", 0.011, 0.24, 8, 0.6]],
  },
  // Warcry (the Hippus): orange war-light in the dust of hooves.
  war: {
    core: "#fff0d8", halo: "#ff6a2a", glow: "#ff8040",
    dark: "#1e140c", mid: "#6a5038", top: "#c0a07a", shade: "#140800", shadeK: 0.55,
    spin: 1.8, arms: 3, bolts: 0.03, dens: 1,
    motes: [["#ff7a3a", 0.012, 0.3, 10, 0.7], ["#8a7050", 0.032, 0.1, 8, 0.8, true]],
  },
  // Legends (the Kuriotates): gold light in a deep blue sky.
  legend: {
    core: "#fffaf0", halo: "#ffd36a", glow: "#ffd870",
    dark: "#0a1028", mid: "#2a3a6a", top: "#8a9ac8", shade: "#040818", shadeK: 0.55,
    spin: 0.9, arms: 4, bolts: 0.04, dens: 1,
    motes: [["#ffe080", 0.012, 0.28, 12, 0.7], ["#c0d0ff", 0.009, 0.42, 6, 0.3]],
  },
};

/** Each ritual's look (a key of RITUAL_LOOKS) by its project's type; any other: `arcane`. */
export const RITUAL_LOOK_OF = {
  PROJECT_BANE_DIVINE: "bane",
  PROJECT_BLOOD_OF_THE_PHOENIX: "phoenix",
  PROJECT_GENESIS: "genesis",
  PROJECT_GLORY_EVERLASTING: "holy",
  PROJECT_RITES_OF_OGHMA: "prism",
  PROJECT_NATURES_REVOLT: "verdant",
  PROJECT_ELEGY_OF_THE_SHEAIM: "hellfire",
  PROJECT_HALLOWING_OF_THE_ELOHIM: "holy",
  PROJECT_PURGE_THE_UNFAITHFUL: "ember",
  PROJECT_BIRTHRIGHT_REGAINED: "banner",
  PROJECT_SAMHAIN: "samhain",
  PROJECT_THE_WHITE_HAND: "ice",
  PROJECT_THE_DEEPENING: "ice",
  PROJECT_STIR_FROM_SLUMBER: "auric",
  PROJECT_THE_DRAW: "ice",
  PROJECT_ASCENSION: "auric",
  PROJECT_PACT_OF_THE_NILHORN: "earth",
  PROJECT_RITUAL_WELLSPRING: "mana",
  PROJECT_RITUAL_ARDOR: "ardor",
  PROJECT_RITUAL_DIVINE_RETRIBUTION: "holy",
  PROJECT_RITUAL_FOR_THE_HORDE: "ember",
  PROJECT_RITUAL_UPGRADE: "forge",
  PROJECT_RITUAL_HYBOREMS_WHISPER: "hellfire",
  PROJECT_RITUAL_INTO_THE_MIST: "mist",
  PROJECT_RITUAL_LEGENDS: "legend",
  PROJECT_RITUAL_MARCH_OF_THE_TREES: "verdant",
  PROJECT_RITUAL_THE_DELVE: "earth",
  PROJECT_RITUAL_RAGING_SEAS: "sea",
  PROJECT_RITUAL_RALLY: "banner",
  PROJECT_RITUAL_RELIGIOUS_FERVOR: "sun",
  PROJECT_RITUAL_REVELRY: "revel",
  PROJECT_RITUAL_THE_TITHE: "blood",
  PROJECT_RITUAL_SANCTUARY: "sanctuary",
  PROJECT_RITUAL_FROZEN_PASSAGE: "ice",
  PROJECT_RITUAL_VEIL_OF_NIGHT: "night",
  PROJECT_RITUAL_WARCRY: "war",
  PROJECT_RITUAL_THE_CULL: "cull",
  PROJECT_RITUAL_WORLDBREAK: "fel",
};

/** The look of the ritual of project type `type` (a RITUAL_LOOKS entry). */
export function ritualLook(type) {
  return RITUAL_LOOKS[RITUAL_LOOK_OF[type]] ?? RITUAL_LOOKS.arcane;
}

const rgb = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
const uniformsMemo = new Map();
/** A look's shader uniforms (made once per look). */
function lookUniforms(look) {
  let u = uniformsMemo.get(look);
  if (!u) {
    u = {
      uCore: rgb(look.core),
      uHalo: rgb(look.halo),
      uGlow: rgb(look.glow),
      uDark: rgb(look.dark),
      uMid: rgb(look.mid),
      uTop: rgb(look.top),
      uShade: [...rgb(look.shade), look.shadeK],
      uLook: [look.spin, look.arms, look.bolts, look.dens],
    };
    uniformsMemo.set(look, u);
  }
  return u;
}

const smooth = (t) => (t <= 0 ? 0 : t >= 1 ? 1 : t * t * (3 - 2 * t));
/** The zoom fade at `px` pixels a plot: [shadow, storm, beam] multipliers (RITUAL_LOOK.fade). */
export function zoomFade(px) {
  const F = RITUAL_LOOK.fade;
  const t = smooth(Math.log(Math.max(px, 1) / F.far) / Math.log(F.near / F.far));
  return [F.shadow + (1 - F.shadow) * t, F.storm + (1 - F.storm) * t, F.beam + (1 - F.beam) * t];
}

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
// The ritual's look (RITUAL_LOOKS): the beam's core and halo, its light, the storm's cloud, the
// shadow's tint and strength; uLook: spin, arms, lightning, density.
uniform vec3 uCore, uHalo, uGlow, uDark, uMid, uTop;
uniform vec4 uShade;
uniform vec4 uLook;
// The zoom fade: the shadow and pool, the storm, the beam (zoomFade).
uniform vec3 uFade;
out vec4 o;

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
// faster, the whole slowly anticlockwise seen from above (uLook.x: the look's speed and sense).
float swirl(float r) {
  return 2.3 * sign(uLook.x) * log(max(r, 0.03)) - uTime * uLook.x * (0.05 + 0.17 * (1.0 - min(r, 1.0))) + uSeed;
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
    float arms = 0.5 + 0.5 * cos(uLook.y * (a + th));
    float n = rfbm(rot(th) * d * 1.3 + uSeed);
    float sh = (1.0 - smoothstep(0.55, 1.1, r + (n - 0.5) * 0.35)) * (0.75 + 0.25 * arms);
    // The light pool at the beam's foot, on the ground and on what stands there (to about a
    // plot up: the town's cards).
    float g = exp(-dot(d, d) / 0.45) * (1.0 - smoothstep(0.3, 1.4, wp.y - c.y)) * pulse;
    float hot = exp(-dot(d, d) / 0.03);
    float k = uShade.w * sh * (1.0 - 0.5 * g);
    o = vec4(uShade.rgb * k + uGlow * g * 0.6 + uCore * hot * 0.45, k) * uFade.x;
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
    vec3 col = uCore * core * (0.82 + 0.18 * s2) + uHalo * halo * (0.45 + 0.55 * s) * 1.15;
    // A flare where it leaves the ground.
    col += uHalo * exp(-vH * vH / 0.02) * halo * 0.6;
    col *= pulse;
    o = vec4(col, clamp(core * 0.9 + halo * 0.15, 0.0, 1.0)) * uFade.z;
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
  float arms = 0.5 + 0.5 * cos(uLook.y * (a + th));
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
  float alpha = clamp(dens * uLook.w * (up > 0.5 ? 1.5 : 2.0), 0.0, up > 0.5 ? 0.8 : 0.95);
  vec3 col = mix(uDark, uMid, smoothstep(0.35, 0.85, n * (0.6 + 0.4 * arms) + 0.15 * up));
  col += (uTop - uMid) * max(lit, 0.0) * 0.9 * (0.5 + 0.5 * up) - uMid * max(-lit, 0.0) * 0.6;
  // The inner bands catch the beam's light.
  float inner = exp(-max(r - 0.16, 0.0) / 0.13);
  col += uGlow * inner * (0.4 + 0.6 * n2) * 1.1 * pulse;
  // Now and then lightning flickers in the ring (uLook.z: how often).
  float flash = step(1.0 - uLook.z, hash12(vec2(floor(uTime * 4.0), uSeed + uLayer))) * exp(-pow((r - 0.45) / 0.15, 2.0)) * smoothstep(0.55, 0.8, n2);
  col += mix(uGlow, uCore, 0.4) * flash * 0.6;
  // Faded as the camera rises: what lies under the storm shows through it.
  alpha *= uFade.y;
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
    if (c?.ritual) out.push({ x: e.x, y: e.y, z: e.z, id: c.id ?? 0, ritual: c.ritual, look: ritualLook(c.ritual_type) });
  }
  return out;
}

/** The motes rising round a city holding a ritual, its look's (scene.js setParticles emitters). */
export function ritualEmitters(list) {
  const out = [];
  for (const c of list) {
    for (const [color, size, rise, count, spread, smoke] of (c.look ?? RITUAL_LOOKS.arcane).motes) {
      out.push({ x: c.x, y: c.y + (smoke ? 0.12 : 0.06), z: c.z, color, size, rise, count, spread, smoke: !!smoke });
    }
  }
  return out;
}

export class RitualLayer {
  constructor(gl) {
    this.gl = gl;
    /** The cities holding a ritual: [{x, y, z, id, ritual, look}] (`ritualCities`). */
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
    const fade = zoomFade(U.uPx ?? 96);
    const p = pl.p.ritual.use().set({
      uFade: fade,
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
        p.set({ uCenter: [c.x, c.y, c.z], uSeed: seed, ...lookUniforms(c.look ?? RITUAL_LOOKS.arcane) });
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
