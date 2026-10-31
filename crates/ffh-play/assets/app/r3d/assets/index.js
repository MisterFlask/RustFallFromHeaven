// The 3D renderer's asset modules (docs/R3D.md "Asset recipes"). Importing this registers every
// recipe and fills the catalog. The fallbacks (basic.js) come first; each art-directed module
// after them overrides the catalog entries it fills in. A module that fails to load is reported
// and skipped: what it would have modelled falls back to the basic recipes.

import "./basic.js";
import "./fallbacks.js";

function failed(name, e) {
  console.error(`r3d assets: ${name} did not load:`, e);
}

const MODULES = ["vegetation", "props", "architecture", "units", "fx"];

// (Start-up prototype, `fastStart`: the modules are fetched together -- module preloads -- and still
// evaluated one after another below, each on its own. Unhinted, each import waited for the one
// before to load and run: five turns of the page's script, and while the game page boots its
// script runs tasks of a second or more, so the chain took 2.6 s there. `?r3d={"fastStart":0}`.)
function fastStart() {
  try {
    const q = new URLSearchParams(globalThis.location?.search ?? "").get("r3d");
    return !(q && q.startsWith("{") && JSON.parse(q).fastStart === 0);
  } catch {
    return true;
  }
}
if (typeof document !== "undefined" && fastStart()) {
  for (const name of MODULES) {
    const link = document.createElement("link");
    link.rel = "modulepreload";
    link.href = new URL(`./${name}.js`, import.meta.url).href;
    document.head.append(link);
  }
}

try { await import("./vegetation.js"); } catch (e) { failed("vegetation", e); }
try { await import("./props.js"); } catch (e) { failed("props", e); }
try { await import("./architecture.js"); } catch (e) { failed("architecture", e); }
try { await import("./units.js"); } catch (e) { failed("units", e); }
try { await import("./fx.js"); } catch (e) { failed("fx", e); }
