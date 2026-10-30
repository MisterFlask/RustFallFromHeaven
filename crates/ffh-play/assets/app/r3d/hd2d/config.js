// Tunables of the HD-2D mode (docs/HD2D.md): its preset over the 3D renderer's switches
// (../config.js), its light and the panel rows of its own passes. Remembered by this browser apart
// from the 3D renderer's look; `?r3d=` overrides it for one page as it does the 3D one.

import { hexLinear } from "../math.js";
import { DEFAULTS, PRESETS } from "../config.js";

// Renamed when the look changes enough that a panel tuning saved for the old one would mislead
// (batch 3: the muted land palette, the wider default view).
export const STORE = "ffh.hd2d.config.3";

/** The light: the sprites are painted lit from the upper left, so the sun stands there -- to the
 *  west and a little north of the view (the camera looks north and never turns), about 45 degrees
 *  up. Shadows fall to the lower right, toward the camera; the light shafts come down from the
 *  upper left. */
export const LIGHT = {
  sunDir: [-0.63, 0.7, -0.33],
  sun: [hexLinear("#ffe4c0"), 2.9],
  // Little fill from the sky and the ground: shade goes deep (the look target's near-black woods and
  // hollows under bright accents), cool rather than grey.
  sky: [hexLinear("#7f98cc"), 0.5],
  ground: [hexLinear("#5c4c3c"), 0.15],
  haze: hexLinear("#9fb0d0"),
  shadowTint: [0.6, 0.68, 1.02],
  exposure: 1.0,
};

/** The HD-2D look over the 3D renderer's painterly switches: a lower camera with a wider lens
 *  (tilted toward the horizon), no grid, the painted grade a little richer, more haze in the
 *  distance, and the mode's own passes: tilt-shift, light shafts, region moods, wide bloom, the
 *  light beams over mana, contact shadows; `lean` stands the sprite cards up (0) or turns them to
 *  face the camera (1). */
export const HD2D = {
  ...PRESETS.painterly,
  lighting: "hd2d",
  ssao: 0.85,
  glow: 1.0,
  // The land's own palette is graded in the mood pass (landGrade): the finish keeps the colours.
  saturation: 1.0,
  contrast: 1.12,
  grain: 0,
  vignette: 0.42,
  rim: 0.4,
  clouds: 0.3,
  haze: 0.1,
  mist: 0.08,
  // Cast shadows less floored than the painterly look's (lightAt: uStylize lifts their floor).
  stylize: 0.7,
  grid: 0,
  borders: 1,
  borderGlow: 1,
  pitch: 32,
  fov: 30,
  tiltShift: 0.6,
  rays: 0.6,
  mood: 1,
  landGrade: 1,
  bloom: 0.8,
  beams: 1,
  contact: 1,
  lean: 1,
  // How the towns (docs/HD2D.md §5c, §5d) stand out from the land: their pieces and spread larger
  // (townScale), lit (townLight: an outline round each piece, brighter, glowing softly, a warmer
  // pool of light on the ground), on a clearing (townClearing: the woods and clutter pushed back,
  // fields round them) and flying their owner's banners over the roofs (townBanners).
  // (Each alone, and together at these strengths, compared in docs/hd2d/towns-stand-out.jpg.)
  townScale: 1.25,
  townLight: 0.5,
  townClearing: 0.5,
  townBanners: 0.6,
};

export const DEFAULTS_HD2D = { ...DEFAULTS, ...HD2D, preset: "hd2d" };
export const PRESETS_HD2D = { hd2d: HD2D, plain: { ...PRESETS.plain, tiltShift: 0, rays: 0, mood: 0, landGrade: 0, bloom: 0, beams: 0, borderGlow: 0, townScale: 1, townLight: 0, townClearing: 0, townBanners: 0 } };

/** The panel rows of the mode's own switches (after the 3D renderer's). */
export const PANEL_HD2D = [
  ["tiltShift", "Tilt-shift blur", "range", 0, 1, 0.05],
  ["rays", "Light shafts", "range", 0, 1.5, 0.05],
  ["mood", "Region moods", "range", 0, 1, 0.05],
  ["landGrade", "Muted land palette", "range", 0, 1, 0.05],
  ["bloom", "Wide bloom", "range", 0, 1.5, 0.05],
  ["beams", "Light beams (mana, towers)", "range", 0, 1.5, 0.05],
  ["borderGlow", "Glowing borders", "range", 0, 1.5, 0.05],
  ["contact", "Contact shadows", "range", 0, 1, 0.05],
  ["townScale", "Towns: size", "range", 1, 1.6, 0.05],
  ["townLight", "Towns: lit and outlined", "range", 0, 1, 0.05],
  ["townClearing", "Towns: clearing and fields", "range", 0, 1, 0.05],
  ["townBanners", "Towns: banners over the roofs", "range", 0, 1, 0.05],
  ["lean", "Sprites face the camera", "range", 0, 1, 0.05],
  ["pitch", "Camera pitch (degrees)", "range", 28, 52, 1],
  ["fov", "Lens (field of view)", "range", 20, 36, 1],
];
