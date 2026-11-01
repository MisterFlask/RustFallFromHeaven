// The 3D renderer's asset modules (docs/R3D.md "Asset recipes"). Importing this registers every
// recipe and fills the catalog. The fallbacks (basic.js) come first; each art-directed module
// after them overrides the catalog entries it fills in.
//
// (Start-up prototype, docs/HD2D.md: static imports -- one module graph, fetched together and
// evaluated in this order. The original imported each art-directed module on its own, `await
// import()` in turn, so a broken one cost only its own models; but each import waited for the one
// before to load and run, a turn of the page's script each, and while the game page boots its
// script runs tasks of a second or more: the chain ended 2.4-3.1 s after the HD-2D import began
// there, 0.9 s as one graph (SwiftShader, the turn-200 save, medians of 3). Even one dynamic step
// (an `await import()` of these as one graph) lost that race (3.1 s). Trade-off: a module that
// fails to load fails the 3D renderer's start, and the client falls back to the isometric map.
// No runtime switch: the original is the commit before.)

import "./basic.js";
import "./fallbacks.js";
import "./vegetation.js";
import "./props.js";
import "./architecture.js";
import "./units.js";
import "./fx.js";
