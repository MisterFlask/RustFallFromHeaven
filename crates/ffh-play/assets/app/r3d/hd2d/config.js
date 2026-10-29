// Tunables of the HD-2D mode (docs/HD2D.md): its preset over the 3D renderer's switches
// (../config.js), its light and the panel rows of its own passes. Remembered by this browser apart
// from the 3D renderer's look; `?r3d=` overrides it for one page as it does the 3D one.

import { hexLinear } from "../math.js";
import { DEFAULTS, PRESETS } from "../config.js";

export const STORE = "ffh.hd2d.config";

/** The light: the sprites are painted lit from the upper left, so the sun stands there -- to the
 *  west and a little north of the view (the camera looks north and never turns), about 45 degrees
 *  up. Shadows fall to the lower right, toward the camera; the light shafts come down from the
 *  upper left. */
export const LIGHT = {
  sunDir: [-0.63, 0.7, -0.33],
  sun: [hexLinear("#ffe4c0"), 2.9],
  sky: [hexLinear("#88a2d4"), 0.72],
  ground: [hexLinear("#6c5c48"), 0.26],
  haze: hexLinear("#a8b6d4"),
  shadowTint: [0.7, 0.76, 1.05],
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
  ssao: 0.6,
  glow: 0.8,
  saturation: 1.14,
  contrast: 1.14,
  grain: 0,
  vignette: 0.32,
  rim: 0.4,
  clouds: 0.22,
  haze: 0.22,
  mist: 0.16,
  grid: 0,
  borders: 1,
  borderGlow: 1,
  pitch: 36,
  fov: 30,
  tiltShift: 0.4,
  rays: 0.55,
  mood: 1,
  bloom: 0.45,
  beams: 1,
  contact: 1,
  lean: 1,
};

export const DEFAULTS_HD2D = { ...DEFAULTS, ...HD2D, preset: "hd2d" };
export const PRESETS_HD2D = { hd2d: HD2D, plain: { ...PRESETS.plain, tiltShift: 0, rays: 0, mood: 0, bloom: 0, beams: 0, borderGlow: 0 } };

/** The panel rows of the mode's own switches (after the 3D renderer's). */
export const PANEL_HD2D = [
  ["tiltShift", "Tilt-shift blur", "range", 0, 1, 0.05],
  ["rays", "Light shafts", "range", 0, 1.5, 0.05],
  ["mood", "Region moods", "range", 0, 1, 0.05],
  ["bloom", "Wide bloom", "range", 0, 1.5, 0.05],
  ["beams", "Light beams (mana, towers)", "range", 0, 1.5, 0.05],
  ["borderGlow", "Glowing borders", "range", 0, 1.5, 0.05],
  ["contact", "Contact shadows", "range", 0, 1, 0.05],
  ["lean", "Sprites face the camera", "range", 0, 1, 0.05],
  ["pitch", "Camera pitch (degrees)", "range", 28, 52, 1],
  ["fov", "Lens (field of view)", "range", 20, 36, 1],
];
