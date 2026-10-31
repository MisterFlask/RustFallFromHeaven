// The 3D renderer's asset modules (docs/R3D.md "Asset recipes"). Importing this registers every
// recipe and fills the catalog. The fallbacks (basic.js) come first; each art-directed module
// after them overrides the catalog entries it fills in. A module that fails to load is reported
// and skipped: what it would have modelled falls back to the basic recipes.

import "./basic.js";
import "./fallbacks.js";

function failed(name, e) {
  console.error(`r3d assets: ${name} did not load:`, e);
}

// (Start-up prototype, `fastStart`, docs/HD2D.md: the art-directed modules as one module graph,
// all.js -- fetched together, evaluated in the same order -- at the cost of their isolation: a
// broken one fails the 3D renderer's start, which falls back to the isometric map. Imported one by
// one, each import waited for the one before to load and run: five turns of the page's script, and
// while the game page boots its script runs tasks of a second or more, so the chain ended 2.4-2.9 s
// after it began there (0.9 s as one graph; SwiftShader, the turn-200 save). `?r3d={"fastStart":0}`.)
function fastStart() {
  try {
    const q = new URLSearchParams(globalThis.location?.search ?? "").get("r3d");
    return !(q && q.startsWith("{") && JSON.parse(q).fastStart === 0);
  } catch {
    return true;
  }
}

if (fastStart()) {
  try { await import("./all.js"); } catch (e) { failed("the art-directed modules", e); }
} else {
  try { await import("./vegetation.js"); } catch (e) { failed("vegetation", e); }
  try { await import("./props.js"); } catch (e) { failed("props", e); }
  try { await import("./architecture.js"); } catch (e) { failed("architecture", e); }
  try { await import("./units.js"); } catch (e) { failed("units", e); }
  try { await import("./fx.js"); } catch (e) { failed("fx", e); }
}
