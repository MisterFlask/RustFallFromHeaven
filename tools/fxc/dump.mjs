// node tools/fxc/dump.mjs OUT_DIR [--mode 3d|hd2d] [--all] [--tree ROOT]: writes the fragment
// shaders of the 3D renderer's programs as Chrome on Windows builds them (the mode's hooks,
// Map3D.pipelineArgs, with a Direct3D 11 renderer string: the HD-2D scene packed, its noise from
// the hash texture, its lean terrain and lite programs; r3d-lab/compile.html builds them the same
// way), one OUT_DIR/NAME.frag a program, its #defines prepended as gl.js Program does. --tree takes
// another checkout's (the old side of an A/B). Without --all, only the scene's programs (terrain,
// props, water, rivers): the long compiles (docs/R3D.md §11.1).
//
// One change to the source, to match the HLSL ANGLE makes: a texture read in a loop that breaks
// (the water's reflection march) is read at LOD 0, as ANGLE reads it there, so FXC keeps the loop
// instead of unrolling it (about 1200 instructions the browser never compiles).
import { pathToFileURL } from "node:url";
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const out = args.find((a, i) => !a.startsWith("--") && !["--mode", "--tree"].includes(args[i - 1]));
const mode = args.includes("--mode") ? args[args.indexOf("--mode") + 1] : "hd2d";
const all = args.includes("--all");
if (!out) {
  console.error("usage: node tools/fxc/dump.mjs OUT_DIR [--mode 3d|hd2d] [--all] [--tree ROOT]");
  process.exit(2);
}
const tree = args.includes("--tree") ? path.resolve(args[args.indexOf("--tree") + 1]) : path.join(path.dirname(new URL(import.meta.url).pathname), "../..");
const R3D = path.join(tree, "crates/ffh-play/assets/app/r3d");
const imp = (p) => import(pathToFileURL(path.join(R3D, p)).href);
const { pipelinePrograms } = await imp("pipeline.js");
const { hd2dPrograms } = await imp("hd2d/passes.js");
const cls = (await imp(mode === "3d" ? "map3d.js" : "hd2d/index.js")).Map3D;
const conf = await imp(mode === "3d" ? "config.js" : "hd2d/config.js");
// The mode's hooks read only its config and the renderer string: a bare object of its class, with
// the default look (a stored panel tuning would change the programs).
const m = Object.assign(Object.create(cls.prototype), { glName: "ANGLE (NVIDIA, NVIDIA GeForce RTX 4060 Laptop GPU Direct3D11 vs_5_0 ps_5_0, D3D11)" });
m.cfg = { ...(mode === "3d" ? conf.DEFAULTS : conf.DEFAULTS_HD2D) };
const [terrainDefines, opts] = m.pipelineArgs();
const progs = { ...pipelinePrograms(terrainDefines, opts), ...(mode === "hd2d" ? hd2dPrograms(!!opts.packed) : {}) };
const scene = ["terrain", "prop", "water", "river"];
fs.mkdirSync(out, { recursive: true });
const names = [];
for (const [key, [name, , fsrc, defs = {}]] of Object.entries(progs)) {
  if (!all && !scene.includes(key)) continue;
  const head = "#version 300 es\nprecision highp float;\nprecision highp int;\nprecision highp sampler2D;\n" + Object.entries(defs).map(([k, v]) => `#define ${k} ${v}\n`).join("");
  let s = head + "#define FRAGMENT 1\n" + fsrc;
  s = s
    .replace("float zs = texture(uSceneND, q).w;", "float zs = textureLod(uSceneND, q, 0.0).w;")
    .replace("hitCol = texture(uSceneColor, q).rgb * 0.5 + (texture(uSceneColor, q - dy).rgb + texture(uSceneColor, q + dy).rgb) * 0.25;", "hitCol = textureLod(uSceneColor, q, 0.0).rgb * 0.5 + (textureLod(uSceneColor, q - dy, 0.0).rgb + textureLod(uSceneColor, q + dy, 0.0).rgb) * 0.25;");
  fs.writeFileSync(path.join(out, `${name}.frag`), s);
  names.push(name);
}
console.log(names.join(" "));
