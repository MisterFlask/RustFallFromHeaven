#!/usr/bin/env node
// The game's music on a page of its own, to listen to and to measure (docs/MUSIC.md): writes ONE
// self-contained HTML file that plays every piece in `PIECES` with the client's own modules
// (crates/ffh-play/assets/app/music_pieces.js, music_score.js, music_synth.js, music.js),
// synthesized in the page with Web Audio. No sound files and no network requests: the modules are
// inlined, so the file opens from file://, from any static server and inside a sandboxed iframe.
//
//   node tools/music-demo.mjs OUT.html
//
// The page: the piece (a picker: choosing another while one plays lets the old orchestra go and
// starts the new piece from its beginning), Play / Pause (the same scheduling loop as music.js, a
// context suspended while paused or hidden), the volume, where the piece is (time, bar, the
// section from the piece's `sections`, which playing), a spectrum, and Measure: one playing of the
// chosen piece rendered offline (music.js `renderPiece`), its peaks, NaNs, samples above 0.8 and
// 0.85, RMS and peak per section (the whole piece when it has no `sections`), octave-band levels
// (whole piece and loudest section), the largest sample-to-sample jump, and the hall's level
// against the dry sound (a second render with `hall: 0`; the hall alone is the full render less
// the dry one, sample by sample); optionally the same for each voice the piece uses, alone. A script driving a browser
// reads the same numbers from `window.musicMeasure({ piece, perVoice })` (`piece`: an id, the
// chosen piece when left out).
//
// The bundling is a small strict transform, not a bundler: the modules are read from disk on every
// run in dependency order, each is wrapped in its own function scope, `export function|async
// function|const|let|class NAME` becomes a plain declaration returned in the module's object, and
// `import { a, b as c } from "./x.js"` becomes a destructuring of an earlier module's object (every
// imported name checked against that module's exports). Any other import or export form
// (`export default`, `export { }`, `export *`, a re-export, `import * as`, a default import, a
// multi-line import, a dynamic `import(`, `import.meta`, a generator or destructured export) and
// any `<script`, `</script` or `<!--` in a module stops the tool with the offending line: it never
// writes a page that would be silently broken.
//
// Exit codes: 0 written, 1 the transform refused a module (or a file could not be read/written),
// 2 a usage error.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const APP = path.join(ROOT, "crates", "ffh-play", "assets", "app");
/** The modules, in dependency order: each may import only from those before it. */
const MODULES = ["music_pieces.js", "music_score.js", "music_synth.js", "music.js"];
const USAGE = "usage: node tools/music-demo.mjs OUT.html";

class Refused extends Error {}

// ------------------------------------------------------------------------------------------
// The transform

const IDENT = "[A-Za-z_$][\\w$]*";

/** The module's source as a function scope returning its exports, or a `Refused` error. `done`
 *  maps a bundled module's file name to `{ name, exports }`. */
function transform(file, src, done) {
  const name = `__mod_${file.replace(/\.js$/, "").replace(/\W/g, "_")}`;
  const lines = src.split(/\r?\n/);
  const exports = [];
  const refuse = (i, why) => {
    throw new Refused(`${path.relative(ROOT, path.join(APP, file))}:${i + 1}: ${why}\n    ${lines[i].trim()}`);
  };
  const body = lines.map((line, i) => {
    if (/<\/?script|<!--/i.test(line)) refuse(i, "a module may not contain <script, </script or <!-- (it is inlined in a <script>)");
    if (/\bimport\s*\(/.test(line)) refuse(i, "dynamic import( is not handled");
    if (/\bimport\s*\.\s*meta\b/.test(line)) refuse(i, "import.meta is not handled");
    if (/^\s*import\b/.test(line)) {
      const m = new RegExp(`^import\\s*\\{([^}]*)\\}\\s*from\\s*"\\./([\\w.-]+\\.js)"\\s*;?\\s*$`).exec(line);
      if (!m) refuse(i, 'only `import { a, b as c } from "./x.js";` on one line is handled');
      const dep = done.get(m[2]);
      if (!dep) refuse(i, `"./${m[2]}" is not a module bundled before ${file} (order: ${MODULES.join(", ")})`);
      const parts = m[1]
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)
        .map((spec) => {
          const s = new RegExp(`^(${IDENT})(?:\\s+as\\s+(${IDENT}))?$`).exec(spec);
          if (!s) refuse(i, `cannot read the import "${spec}"`);
          if (!dep.exports.includes(s[1])) refuse(i, `${m[2]} does not export "${s[1]}" (it exports ${dep.exports.join(", ")})`);
          return s[2] ? `${s[1]}: ${s[2]}` : s[1];
        });
      if (!parts.length) refuse(i, "an import of nothing");
      return `const { ${parts.join(", ")} } = ${dep.name};`;
    }
    if (/^\s*export\b/.test(line)) {
      const m = new RegExp(`^export\\s+(async\\s+function|function|const|let|class)\\s+(${IDENT})(.*)$`).exec(line);
      if (!m) refuse(i, "only `export function|async function|const|let|class NAME` at the start of a line is handled");
      const [, kind, id, rest] = m;
      if (/function$/.test(kind) && !/^\s*\(/.test(rest)) refuse(i, "an exported function must be `function NAME(` (no generators)");
      if ((kind === "const" || kind === "let") && !/^\s*=/.test(rest)) refuse(i, "an exported const/let must be `NAME = ...`");
      if ((kind === "const" || kind === "let") && new RegExp(`,\\s*${IDENT}\\s*=(?!=|>)`).test(rest) && /;\s*$/.test(rest)) refuse(i, "one exported name per declaration");
      if (exports.includes(id)) refuse(i, `"${id}" is exported twice`);
      exports.push(id);
      return line.replace(/^export\s+/, "");
    }
    if (/(^|[;{}])\s*export\s/.test(line.replace(/\/\/.*$/, ""))) refuse(i, "an export that does not start its line");
    return line;
  });
  const code = [`// ==== bundle: ${file}`, `const ${name} = (() => {`, '"use strict";', ...body, `return Object.freeze({ ${exports.join(", ")} });`, "})();"].join("\n");
  return { name, exports, code };
}

function bundle() {
  const done = new Map();
  const parts = [];
  for (const file of MODULES) {
    const src = fs.readFileSync(path.join(APP, file), "utf8");
    const mod = transform(file, src, done);
    done.set(file, mod);
    parts.push(mod.code);
  }
  return parts.join("\n\n");
}

// ------------------------------------------------------------------------------------------
// The page's own code. Written as real functions (inlined with `.toString()`), so nothing in them
// is a template literal of the tool's. `demoPure` touches no browser API: a node script can run
// it from the page (the position and the analysis).

function demoPure() {
  const BAND_EDGES = [45, 90, 180, 355, 710, 1400, 2800, 5600, 11200, 20000];

  const ordinal = (n) => {
    const t = n % 100;
    const s = t >= 11 && t <= 13 ? "th" : n % 10 === 1 ? "st" : n % 10 === 2 ? "nd" : n % 10 === 3 ? "rd" : "th";
    return `${n}${s}`;
  };
  /** The section at a bar: the last of the piece's `sections` (`[[firstBar, name], ...]`, in
   *  order) that starts at or before it; null when the piece has none. */
  const sectionOf = (sections, bar) => {
    let name = null;
    for (const [first, n] of sections ?? []) if (bar >= first) name = n;
    return name;
  };

  /** Where the music is at `now` (the context's clock), from the conductor's `origin` and `pass`.
   *  The conductor moves `origin` to the next playing as soon as it has handed out the last note
   *  (several seconds before the playing ends), so a time before `origin` belongs to the playing
   *  before, or to the silence after it. `playing` counts from 1. */
  function positionAt({ now, origin, pass, seconds, gap, starts, barCount, sections }) {
    if (origin === null || origin === undefined) return { state: "idle" };
    let rel = now - origin;
    let index = pass;
    if (rel < 0 && pass > 0) {
      rel += seconds + gap;
      index = pass - 1;
    }
    if (rel > seconds) return { state: "gap", playing: index + 1, next: index + 2, elapsed: seconds, wait: Math.max(0, seconds + gap - rel) };
    if (rel < 0) rel = 0;
    let bar = 0;
    for (let i = 0; i < barCount; i++) if (starts[i] <= rel) bar = i;
    return { state: "playing", playing: index + 1, elapsed: rel, bar, section: sectionOf(sections, bar) };
  }

  const clock = (s) => {
    const t = Math.max(0, Math.floor(s));
    return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
  };

  /** The time regions a render is measured by: the piece's sections (the whole piece when it has
   *  none), from the score's times plus `lead` (the render starts its first note that late), and
   *  the tail after the last bar. */
  function regionsOf(sections, starts, seconds, total, lead) {
    const barCount = starts.length - 1;
    const list = sections && sections.length ? sections : [[0, "The whole piece"]];
    const out = list.map(([first, name], i) => {
      const next = i + 1 < list.length ? list[i + 1][0] : barCount;
      return { name, from: i === 0 ? 0 : starts[first] + lead, to: starts[next] + lead };
    });
    out.push({ name: "Tail", from: seconds + lead, to: total });
    return out;
  }

  /** The mean square over every channel (NaN read as 0). */
  function meanSquare(channels) {
    let s = 0;
    let n = 0;
    for (const d of channels) {
      for (let i = 0; i < d.length; i++) {
        const x = d[i];
        if (x === x) s += x * x;
      }
      n += d.length;
    }
    return n ? s / n : 0;
  }

  /** The hall against the dry sound, from the mean squares of a render with the hall, of one
   *  without it (the same length) and of the hall alone (`wetSquare`: the two renders' difference
   *  sample by sample; when left out, the difference of the two mean squares, which is only right
   *  where the hall and the dry sound do not move together). */
  function hallLevel(total, dry, wetSquare) {
    const wet = wetSquare ?? total - dry;
    return { dryDb: db(dry), wetDb: wet > 0 ? db(wet) : null, wetMinusDryDb: wet > 0 && dry > 0 ? Math.round(10 * Math.log10(wet / dry) * 10) / 10 : null };
  }

  /** dB of a mean square (null: exactly silent). */
  const db = (ms) => (ms > 0 ? Math.round(10 * Math.log10(ms) * 10) / 10 : null);
  const round = (x, k = 4) => (Number.isFinite(x) ? Math.round(x * 10 ** k) / 10 ** k : x);

  /** An in-place radix-2 FFT of size `n` (a power of two). */
  function makeFft(n) {
    const bits = Math.round(Math.log2(n));
    if (1 << bits !== n) throw new Error(`fft: ${n} is not a power of two`);
    const cos = new Float64Array(n / 2);
    const sin = new Float64Array(n / 2);
    for (let k = 0; k < n / 2; k++) {
      cos[k] = Math.cos((2 * Math.PI * k) / n);
      sin[k] = Math.sin((2 * Math.PI * k) / n);
    }
    const rev = new Uint32Array(n);
    for (let i = 0; i < n; i++) {
      let r = 0;
      for (let b = 0; b < bits; b++) r |= ((i >> b) & 1) << (bits - 1 - b);
      rev[i] = r;
    }
    return (re, im) => {
      for (let i = 0; i < n; i++) {
        const j = rev[i];
        if (j > i) {
          let t = re[i];
          re[i] = re[j];
          re[j] = t;
          t = im[i];
          im[i] = im[j];
          im[j] = t;
        }
      }
      for (let size = 2; size <= n; size *= 2) {
        const half = size / 2;
        const step = n / size;
        for (let i = 0; i < n; i += size) {
          for (let j = 0; j < half; j++) {
            const k = j * step;
            const a = i + j;
            const b = a + half;
            const tre = re[b] * cos[k] + im[b] * sin[k];
            const tim = im[b] * cos[k] - re[b] * sin[k];
            re[b] = re[a] - tre;
            im[b] = im[a] - tim;
            re[a] += tre;
            im[a] += tim;
          }
        }
      }
    };
  }

  /** The numbers of one rendered playing. `channels`: Float32Arrays; `regions`: `[{ name, from,
   *  to }]` in seconds (the sections). Levels are dB of the mean square against a full-scale
   *  constant (a full-scale sine reads -3.0), the bands on the same scale (they add up to the
   *  whole). `pause()` is awaited now and then so the page keeps answering. */
  async function analyse(channels, sampleRate, regions, pause = () => null) {
    const len = channels[0].length;
    const nch = channels.length;
    const peak = [];
    let nan = 0;
    let over = 0;
    let over08 = 0;
    let jump = { value: 0, time: 0, channel: 0 };
    let sumsq = 0;
    const reg = regions.map((r) => ({ ...r, a: Math.max(0, Math.floor(r.from * sampleRate)), b: Math.min(len, Math.floor(r.to * sampleRate)), sumsq: 0, peak: 0 }));
    for (let ch = 0; ch < nch; ch++) {
      const d = channels[ch];
      let p = 0;
      let prev = 0;
      let ri = 0;
      for (let i = 0; i < len; i++) {
        let x = d[i];
        if (x !== x) {
          nan++;
          x = 0;
        }
        const ax = x < 0 ? -x : x;
        if (ax > p) p = ax;
        if (ax > 0.8) over08++;
        if (ax > 0.85) over++;
        const j = x - prev;
        const aj = j < 0 ? -j : j;
        if (i > 0 && aj > jump.value) jump = { value: aj, time: i / sampleRate, channel: ch };
        prev = x;
        sumsq += x * x;
        while (ri < reg.length - 1 && i >= reg[ri].b) ri++;
        const r = reg[ri];
        if (r && i >= r.a && i < r.b) {
          r.sumsq += x * x;
          if (ax > r.peak) r.peak = ax;
        }
      }
      peak.push(p);
      await pause();
    }

    // Octave bands: 8192-point Hann frames, half overlap, of the mono sum.
    const N = 8192;
    const hop = N / 2;
    const fft = makeFft(N);
    const win = new Float64Array(N);
    let wsum = 0;
    for (let i = 0; i < N; i++) {
      win[i] = 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / N);
      wsum += win[i] * win[i];
    }
    const nb = BAND_EDGES.length - 1;
    const binBand = new Int16Array(N / 2).fill(-1);
    for (let k = 1; k < N / 2; k++) {
      const f = (k * sampleRate) / N;
      for (let b = 0; b < nb; b++) if (f >= BAND_EDGES[b] && f < BAND_EDGES[b + 1]) binBand[k] = b;
    }
    const re = new Float64Array(N);
    const im = new Float64Array(N);
    const whole = new Float64Array(nb);
    let frames = 0;
    const regBands = reg.map(() => ({ sum: new Float64Array(nb), frames: 0 }));
    const scale = 2 / (N * wsum);
    for (let s = 0; s + N <= len; s += hop) {
      for (let i = 0; i < N; i++) {
        let x = 0;
        for (let ch = 0; ch < nch; ch++) {
          const v = channels[ch][s + i];
          if (v === v) x += v;
        }
        re[i] = (x / nch) * win[i];
        im[i] = 0;
      }
      fft(re, im);
      const centre = s + N / 2;
      const rb = regBands[reg.findIndex((r) => centre >= r.a && centre < r.b)];
      for (let k = 1; k < N / 2; k++) {
        const b = binBand[k];
        if (b < 0) continue;
        const p = (re[k] * re[k] + im[k] * im[k]) * scale;
        whole[b] += p;
        if (rb) rb.sum[b] += p;
      }
      frames++;
      if (rb) rb.frames++;
      if (frames % 24 === 0) await pause();
    }
    const sections = reg.map((r) => ({
      name: r.name,
      from: round(r.from, 2),
      to: round(r.to, 2),
      rmsDb: r.b > r.a ? db(r.sumsq / ((r.b - r.a) * nch)) : null,
      peak: round(r.peak),
    }));
    let loud = -1;
    sections.forEach((s, i) => {
      if (s.rmsDb !== null && (loud < 0 || s.rmsDb > sections[loud].rmsDb)) loud = i;
    });
    const bandsOf = (sum, n) => Array.from(sum, (v) => (n ? db(v / n) : null));
    return {
      seconds: round(len / sampleRate, 2),
      sampleRate,
      peak: peak.map((p) => round(p)),
      nan,
      over080: over08,
      over085: over,
      meanSquare: sumsq / (len * nch),
      rmsDb: db(sumsq / (len * nch)),
      sections,
      bands: {
        edges: BAND_EDGES.slice(),
        whole: bandsOf(whole, frames),
        loudestSection: loud >= 0 ? sections[loud].name : null,
        loudest: loud >= 0 ? bandsOf(regBands[loud].sum, regBands[loud].frames) : null,
      },
      maxJump: { value: round(jump.value), time: round(jump.time, 3), channel: jump.channel },
    };
  }

  return { BAND_EDGES, ordinal, sectionOf, positionAt, regionsOf, meanSquare, hallLevel, clock, makeFft, analyse, db };
}

function pageMain() {
  const { tempoMap, compile, PIECES } = __mod_music_score;
  const { MIX, createOrchestra, settingsFor } = __mod_music_synth;
  const { Conductor, renderPiece } = __mod_music;
  const { BAND_EDGES, ordinal, positionAt, regionsOf, meanSquare, hallLevel, clock, analyse } = DEMO;

  const AHEAD = 4;
  const TICK_MS = 400;
  const FADE_IN = 2;
  const FADE = 0.6;
  /** The fade in of a piece chosen while another plays (the old one stops at once). */
  const SWITCH_IN = 0.3;
  /** renderPiece starts the first note this late into the buffer (music.js). */
  const RENDER_LEAD = 0.1;
  /** Renders at once after the first. */
  const AT_ONCE = 2;

  const $ = (id) => document.getElementById(id);
  const statusEl = $("status");
  const say = (text, kind = "") => {
    statusEl.textContent = text;
    statusEl.className = kind;
  };
  const fail = (e) => say(`Error: ${(e && (e.message || e.reason)) || e}`, "err");
  const guard = (fn) => (...args) => {
    try {
      const r = fn(...args);
      if (r && typeof r.catch === "function") r.catch(fail);
      return r;
    } catch (e) {
      fail(e);
    }
  };
  window.addEventListener("error", (ev) => fail(ev.error || ev.message));
  window.addEventListener("unhandledrejection", (ev) => fail(ev.reason));

  const AudioCtx = globalThis.AudioContext ?? globalThis.webkitAudioContext ?? null;
  const OfflineCtx = globalThis.OfflineAudioContext ?? globalThis.webkitOfflineAudioContext ?? null;
  const reduced = globalThis.matchMedia ? matchMedia("(prefers-reduced-motion: reduce)").matches : false;

  /** What the page needs to know of a piece. */
  const describe = (piece) => {
    const map = tempoMap(piece);
    return {
      piece,
      map,
      gap: piece.gap ?? 8,
      barCount: piece.bars.length,
      sections: piece.sections && piece.sections.length ? piece.sections : null,
      // A line handed to another voice on every second playing.
      alt: piece.lines.some((l) => l.alt),
    };
  };
  const labelOf = (piece) => (piece.style ? `${piece.title} (${piece.style})` : piece.title);

  let cur = describe(PIECES.vigil ?? Object.values(PIECES)[0]);

  const st = { ctx: null, orchestra: null, conductor: null, analyser: null, timer: null, resting: null, want: false, frame: null };
  const playBtn = $("play");
  const vol = $("vol");
  const picker = $("piece");
  const gainOf = (v) => v * v;
  const volume = () => Number(vol.value) / 100;

  for (const p of Object.values(PIECES)) {
    const o = document.createElement("option");
    o.value = p.id;
    o.textContent = labelOf(p);
    picker.appendChild(o);
  }
  picker.value = cur.piece.id;

  /** The title, the length and the position shown for the chosen piece, from its start. */
  function showPiece() {
    $("title").textContent = cur.piece.title;
    $("style").textContent = cur.piece.style ? cur.piece.style : "";
    document.title = `${cur.piece.title}: music`;
    $("total").textContent = clock(cur.map.seconds);
    $("pos").textContent = clock(0);
    $("bar").textContent = "-";
    $("section").textContent = "-";
    $("section").hidden = !cur.sections;
    $("sectiondt").hidden = !cur.sections;
    $("playing").textContent = "-";
  }
  showPiece();

  /** Lets the orchestra go: everything scheduled on it stops sounding (its output is cut). */
  function dropOrchestra() {
    const o = st.orchestra;
    st.orchestra = null;
    st.conductor = null;
    if (!o) return;
    try {
      o.close();
    } catch {
      /* already let go */
    }
  }

  /** A new orchestra and conductor for the chosen piece, silent until faded in. */
  function buildOrchestra() {
    st.orchestra = createOrchestra(st.ctx, settingsFor(cur.piece));
    st.conductor = new Conductor(cur.piece);
    // A tap only: the analyser goes nowhere.
    st.orchestra.out.connect(st.analyser);
    st.orchestra.out.gain.value = 0;
  }

  function stopWith(e) {
    fail(e);
    clearInterval(st.timer);
    st.timer = null;
    clearTimeout(st.resting);
    st.resting = null;
    dropOrchestra();
    try {
      st.ctx?.close();
    } catch {
      /* already closed */
    }
    st.ctx = null;
    st.analyser = null;
    st.want = false;
    playBtn.textContent = "Play";
    playBtn.setAttribute("aria-pressed", "false");
  }

  function tick() {
    try {
      const { ctx, orchestra, conductor } = st;
      if (!ctx || !orchestra || !conductor || ctx.state !== "running") return;
      const now = ctx.currentTime;
      for (const e of conductor.take(now, now + AHEAD)) orchestra.play(e);
    } catch (e) {
      stopWith(e);
    }
  }

  function fadeTo(level, seconds) {
    if (!st.orchestra) return;
    const g = st.orchestra.out.gain;
    const now = st.ctx.currentTime;
    g.cancelScheduledValues(now);
    g.setValueAtTime(g.value, now);
    g.linearRampToValueAtTime(level, now + seconds);
  }

  function play() {
    if (!AudioCtx) throw new Error("This browser has no Web Audio (AudioContext): the music cannot play here.");
    clearTimeout(st.resting);
    st.resting = null;
    st.want = true;
    if (!st.ctx) {
      st.ctx = new AudioCtx({ latencyHint: "playback" });
      st.analyser = st.ctx.createAnalyser();
      st.analyser.fftSize = 2048;
      st.analyser.smoothingTimeConstant = 0.8;
    }
    if (!st.orchestra) {
      buildOrchestra();
      fadeTo(gainOf(volume()), FADE_IN);
    } else {
      fadeTo(gainOf(volume()), FADE);
    }
    st.ctx.resume().then(tick, fail);
    if (!st.timer) st.timer = setInterval(tick, TICK_MS);
    tick();
    playBtn.textContent = "Pause";
    playBtn.setAttribute("aria-pressed", "true");
    startVisual();
  }

  function rest() {
    st.want = false;
    playBtn.textContent = "Play";
    playBtn.setAttribute("aria-pressed", "false");
    if (!st.ctx) return;
    fadeTo(0, FADE);
    clearTimeout(st.resting);
    st.resting = setTimeout(
      guard(() => {
        st.resting = null;
        clearInterval(st.timer);
        st.timer = null;
        st.ctx?.suspend().catch(fail);
        stopVisual();
      }),
      FADE * 1000 + 100,
    );
  }

  /** Another piece: the old one stops at once (its orchestra let go, nothing of it left
   *  scheduled) and, if the music is playing, the new one starts from its beginning; if it is
   *  paused, the new one waits for Play. */
  function choose(id) {
    const piece = PIECES[id];
    if (!piece || piece === cur.piece) return;
    cur = describe(piece);
    showPiece();
    if (!st.ctx) return;
    dropOrchestra();
    if (!st.want) return;
    buildOrchestra();
    fadeTo(gainOf(volume()), SWITCH_IN);
    tick();
  }

  picker.addEventListener(
    "change",
    guard(() => choose(picker.value)),
  );

  playBtn.addEventListener(
    "click",
    guard(() => {
      if (st.want) rest();
      else play();
    }),
  );

  vol.addEventListener(
    "input",
    guard(() => {
      $("volval").textContent = vol.value;
      if (!st.ctx || !st.want || !st.orchestra) return;
      const g = st.orchestra.out.gain;
      const now = st.ctx.currentTime;
      // Cancel the fade in, else it would still land on the old volume.
      g.cancelScheduledValues(now);
      g.setValueAtTime(g.value, now);
      g.setTargetAtTime(gainOf(volume()), now, 0.05);
    }),
  );

  document.addEventListener(
    "visibilitychange",
    guard(() => {
      if (!st.ctx) return;
      if (document.hidden) {
        clearInterval(st.timer);
        st.timer = null;
        st.ctx.suspend().catch(fail);
        stopVisual();
      } else if (st.want) {
        play();
      }
    }),
  );

  // --- where the piece is
  function showPosition() {
    const posEl = $("pos");
    const barEl = $("bar");
    const secEl = $("section");
    const playingEl = $("playing");
    if (!st.ctx || !st.conductor) return;
    const { map, gap, barCount, sections } = cur;
    const p = positionAt({ now: st.ctx.currentTime, origin: st.conductor.origin, pass: st.conductor.pass, seconds: map.seconds, gap, starts: map.starts, barCount, sections });
    if (p.state === "idle") return;
    if (p.state === "gap") {
      posEl.textContent = clock(map.seconds);
      barEl.textContent = "-";
      secEl.textContent = "Silence before the next playing";
      playingEl.textContent = `${ordinal(p.next)} playing next`;
      return;
    }
    posEl.textContent = clock(p.elapsed);
    barEl.textContent = `${p.bar + 1} of ${barCount}`;
    secEl.textContent = p.section ?? "-";
    playingEl.textContent = `${ordinal(p.playing)} playing${cur.alt && p.playing % 2 === 0 ? " (the second orchestration)" : ""}`;
  }
  setInterval(guard(showPosition), 250);

  // --- the visual
  const canvas = $("viz");
  const levelEl = $("level");
  let freq = null;
  let wave = null;
  let lastDraw = 0;
  const css = getComputedStyle(document.documentElement);
  const barColor = css.getPropertyValue("--accent").trim() || "#c9a45c";
  const dimColor = css.getPropertyValue("--line").trim() || "#2a2d31";
  if (reduced) {
    canvas.hidden = true;
    levelEl.hidden = false;
  }
  function sizeCanvas() {
    const dpr = Math.min(3, globalThis.devicePixelRatio || 1);
    const w = Math.max(1, Math.round(canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(canvas.clientHeight * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
  }
  function drawSpectrum(t) {
    st.frame = requestAnimationFrame(drawSpectrumSafe);
    if (t - lastDraw < 33) return;
    lastDraw = t;
    const a = st.analyser;
    if (!a) return;
    if (!freq) freq = new Uint8Array(a.frequencyBinCount);
    a.getByteFrequencyData(freq);
    sizeCanvas();
    const g = canvas.getContext("2d");
    const W = canvas.width;
    const H = canvas.height;
    g.clearRect(0, 0, W, H);
    const bands = 40;
    const lo = Math.log(40);
    const hi = Math.log(16000);
    const binHz = st.ctx.sampleRate / a.fftSize;
    const slot = W / bands;
    for (let b = 0; b < bands; b++) {
      const f0 = Math.exp(lo + ((hi - lo) * b) / bands);
      const f1 = Math.exp(lo + ((hi - lo) * (b + 1)) / bands);
      const k0 = Math.max(1, Math.floor(f0 / binHz));
      const k1 = Math.max(k0 + 1, Math.ceil(f1 / binHz));
      let m = 0;
      for (let k = k0; k < k1 && k < freq.length; k++) if (freq[k] > m) m = freq[k];
      const h = Math.max(1, (m / 255) * H);
      g.fillStyle = m > 0 ? barColor : dimColor;
      g.fillRect(Math.round(b * slot + slot * 0.15), H - h, Math.max(1, Math.round(slot * 0.7)), h);
    }
  }
  const drawSpectrumSafe = (t) => {
    try {
      drawSpectrum(t);
    } catch (e) {
      stopVisual();
      fail(e);
    }
  };
  let levelTimer = null;
  function showLevel() {
    const a = st.analyser;
    if (!a) return;
    if (!wave) wave = new Float32Array(a.fftSize);
    a.getFloatTimeDomainData(wave);
    let s = 0;
    for (const x of wave) s += x * x;
    const ms = s / wave.length;
    levelEl.textContent = ms > 0 ? `Level: ${(10 * Math.log10(ms)).toFixed(0)} dBFS` : "Level: silent";
  }
  function startVisual() {
    if (reduced) {
      if (!levelTimer) levelTimer = setInterval(guard(showLevel), 1000);
      return;
    }
    if (st.frame === null) st.frame = requestAnimationFrame(drawSpectrumSafe);
  }
  function stopVisual() {
    if (st.frame !== null) cancelAnimationFrame(st.frame);
    st.frame = null;
    clearInterval(levelTimer);
    levelTimer = null;
  }

  if (!AudioCtx) {
    playBtn.disabled = true;
    vol.disabled = true;
    $("noaudio").hidden = false;
  }

  // --- Measure
  let measuring = false;
  const pause = () => new Promise((r) => setTimeout(r, 0));

  /** One render of `info`'s piece: `job.only` the voices kept, `job.dry` without the hall (only
   *  its mean square is read). */
  async function renderOne(info, job) {
    const t0 = performance.now();
    const opts = {};
    if (job.only) opts.only = job.only;
    if (job.dry) opts.hall = 0;
    let buf = await renderPiece(info.piece, opts);
    const renderSeconds = Math.round(((performance.now() - t0) / 1000) * 10) / 10;
    const t1 = performance.now();
    const channels = [];
    for (let c = 0; c < buf.numberOfChannels; c++) channels.push(buf.getChannelData(c));
    let r;
    if (job.dry) {
      // The hall alone is the full mix less the dry render, sample by sample (`job.against`: the
      // full mix's job, which kept its samples for this).
      let wetSquare = null;
      const full = job.against?.kept;
      if (full) {
        let s = 0;
        let n = 0;
        channels.forEach((d, c) => {
          const f = full[c];
          const len = Math.min(d.length, f.length);
          for (let i = 0; i < len; i++) s += (f[i] - d[i]) * (f[i] - d[i]);
          n += len;
        });
        wetSquare = n ? s / n : 0;
        job.against.kept = null;
      }
      r = { seconds: Math.round(buf.duration * 100) / 100, meanSquare: meanSquare(channels), wetSquare };
    } else {
      r = await analyse(channels, buf.sampleRate, regionsOf(info.sections, info.map.starts, info.map.seconds, buf.duration, RENDER_LEAD), pause);
      if (job.keep) job.kept = channels;
    }
    buf = null;
    return { label: job.label, only: job.only, renderSeconds, analyseSeconds: Math.round(((performance.now() - t1) / 1000) * 10) / 10, ...r };
  }

  /** Measures a piece (`piece`: its id; the chosen one when left out), shows the numbers and
   *  returns them. */
  async function musicMeasure({ piece: id, perVoice = false, onProgress = () => {} } = {}) {
    if (!OfflineCtx) throw new Error("This browser has no OfflineAudioContext: nothing to measure with.");
    if (id !== undefined && !PIECES[id]) throw new Error(`No piece "${id}" (the pieces: ${Object.keys(PIECES).join(", ")}).`);
    if (measuring) throw new Error("A measurement is already running.");
    measuring = true;
    const info = id === undefined ? cur : describe(PIECES[id]);
    const t0 = performance.now();
    try {
      // The voices this piece uses, in MIX's order (then any MIX does not have).
      const used = new Set(compile(info.piece, 0).events.map((e) => e.voice));
      const voices = [...Object.keys(MIX).filter((v) => used.has(v)), ...[...used].filter((v) => !(v in MIX))];
      const fullJob = { label: "full mix", only: null, keep: true };
      const jobs = [fullJob, { label: "dry (no hall)", only: null, dry: true, against: fullJob }, ...(perVoice ? voices.map((v) => ({ label: v, only: [v] })) : [])];
      const results = new Array(jobs.length);
      let next = 1;
      let finished = 0;
      const running = new Set();
      const report = () => onProgress({ done: finished, total: jobs.length, running: [...running] });
      const run = async (i) => {
        running.add(jobs[i].label);
        report();
        results[i] = await renderOne(info, jobs[i]);
        running.delete(jobs[i].label);
        finished++;
        report();
      };
      // The full mix alone first (the numbers that matter most come soonest), then the rest.
      await run(0);
      const worker = async () => {
        while (next < jobs.length) await run(next++);
      };
      await Promise.all(Array.from({ length: Math.min(AT_ONCE, jobs.length - 1) }, worker));
      const full = results[0];
      const dry = results[1];
      const m = {
        piece: info.piece.id,
        title: info.piece.title,
        perVoice: !!perVoice,
        scoreSeconds: Math.round(info.map.seconds * 100) / 100,
        renderLead: RENDER_LEAD,
        dbNote: "dB of the mean square against full scale (a full-scale sine is -3.0); null: exactly silent",
        totalSeconds: Math.round(((performance.now() - t0) / 1000) * 10) / 10,
        full,
        hall: { ...hallLevel(full.meanSquare, dry.meanSquare, dry.wetSquare), totalDb: full.rmsDb, dryRenderSeconds: dry.renderSeconds },
        voices: results.slice(2),
      };
      showResults(m);
      return m;
    } finally {
      measuring = false;
    }
  }
  window.musicMeasure = musicMeasure;

  const el = (tag, text, cls) => {
    const e = document.createElement(tag);
    if (text !== undefined) e.textContent = text;
    if (cls) e.className = cls;
    return e;
  };
  const fmtDb = (v) => (v === null ? "silent" : v.toFixed(1));
  const table = (head, rows) => {
    const wrap = el("div", undefined, "tablewrap");
    const t = el("table");
    const tr = el("tr");
    for (const h of head) tr.appendChild(el("th", h));
    const thead = el("thead");
    thead.appendChild(tr);
    t.appendChild(thead);
    const tb = el("tbody");
    for (const r of rows) {
      const row = el("tr");
      r.forEach((c, i) => row.appendChild(el(i === 0 ? "th" : "td", String(c))));
      tb.appendChild(row);
    }
    t.appendChild(tb);
    wrap.appendChild(t);
    return wrap;
  };
  const bandName = (i) => `${BAND_EDGES[i]}-${BAND_EDGES[i + 1]} Hz`;

  function showResults(m) {
    const out = $("results");
    out.textContent = "";
    const f = m.full;
    out.appendChild(el("h3", `${m.title}: full mix`));
    out.appendChild(
      table(
        ["", "Value"],
        [
          ["Seconds rendered", f.seconds],
          ["Render took (s)", f.renderSeconds],
          ["Analysis took (s)", f.analyseSeconds],
          ["Peak left", f.peak[0]],
          ["Peak right", f.peak[1] ?? "-"],
          ["NaN samples", f.nan],
          ["Samples above 0.8", f.over080],
          ["Samples above 0.85", f.over085],
          ["RMS (dBFS)", fmtDb(f.rmsDb)],
          ["Largest jump", f.maxJump.value],
          ["Largest jump at (s)", `${f.maxJump.time} (ch ${f.maxJump.channel})`],
        ],
      ),
    );
    out.appendChild(el("h3", "The hall"));
    out.appendChild(
      table(
        ["", "dB"],
        [
          ["Total", fmtDb(m.hall.totalDb)],
          ["Dry (hall 0)", fmtDb(m.hall.dryDb)],
          ["Hall (total less dry)", m.hall.wetDb === null ? "-" : fmtDb(m.hall.wetDb)],
          ["Hall against dry", m.hall.wetMinusDryDb === null ? "-" : m.hall.wetMinusDryDb.toFixed(1)],
        ],
      ),
    );
    out.appendChild(el("h3", "Sections"));
    out.appendChild(table(["Section", "From-to (s)", "RMS dB", "Peak"], f.sections.map((s) => [s.name, `${s.from}-${s.to}`, fmtDb(s.rmsDb), s.peak])));
    out.appendChild(el("h3", "Octave bands (dB)"));
    out.appendChild(table(["Band", "Whole", `Loudest: ${f.bands.loudestSection ?? "-"}`], f.bands.whole.map((v, i) => [bandName(i), fmtDb(v), f.bands.loudest ? fmtDb(f.bands.loudest[i]) : "-"])));
    if (m.voices.length) {
      out.appendChild(el("h3", "Each voice alone"));
      out.appendChild(
        table(
          ["Voice", "Peak L/R", "RMS dB", "NaN", ">0.8", ">0.85", "Jump", "Render s"],
          m.voices.map((v) => [v.label, `${v.peak[0]}/${v.peak[1] ?? "-"}`, fmtDb(v.rmsDb), v.nan, v.over080, v.over085, `${v.maxJump.value} @${v.maxJump.time}`, v.renderSeconds]),
        ),
      );
      out.appendChild(el("h3", "Each voice: RMS dB per section"));
      out.appendChild(table(["Voice", ...f.sections.map((s) => s.name)], m.voices.map((v) => [v.label, ...v.sections.map((s) => fmtDb(s.rmsDb))])));
      out.appendChild(el("h3", "Each voice: octave bands, whole piece (dB)"));
      out.appendChild(table(["Voice", ...BAND_EDGES.slice(0, -1).map((e, i) => `${e}-${BAND_EDGES[i + 1]}`)], m.voices.map((v) => [v.label, ...v.bands.whole.map(fmtDb)])));
    }
    out.appendChild(el("p", m.dbNote + `. Sections are timed from the score plus the render's lead (${m.renderLead} s). The hall's power is the full mix's less the dry render's. Total ${m.totalSeconds} s.`, "note"));
  }

  const measureBtn = $("measure");
  if (!OfflineCtx) {
    measureBtn.disabled = true;
    $("progress").textContent = "This browser has no OfflineAudioContext: nothing to measure with.";
  }
  measureBtn.addEventListener(
    "click",
    guard(async () => {
      const prog = $("progress");
      measureBtn.disabled = true;
      const t0 = performance.now();
      let last = { done: 0, total: 1, running: [] };
      const paint = () => {
        const s = Math.round((performance.now() - t0) / 1000);
        prog.textContent = `${last.done} of ${last.total} renders done, ${s} s` + (last.running.length ? `. Rendering: ${last.running.join(", ")}` : "");
      };
      const timer = setInterval(paint, 500);
      try {
        const m = await musicMeasure({ piece: cur.piece.id, perVoice: $("pervoice").checked, onProgress: (p) => ((last = p), paint()) });
        prog.textContent = `Done: ${m.title}, ${m.full.label}, dry${m.voices.length ? ` and ${m.voices.length} voices` : ""} in ${m.totalSeconds} s.`;
      } catch (e) {
        prog.textContent = "Failed.";
        throw e;
      } finally {
        clearInterval(timer);
        measureBtn.disabled = !OfflineCtx;
      }
    }),
  );

  say(AudioCtx ? "Ready." : "No Web Audio in this browser.");
}

// ------------------------------------------------------------------------------------------
// The page

const CSS = `
:root {
  color-scheme: dark;
  --bg: #0f1113;
  --panel: #171a1d;
  --line: #2a2e33;
  --fg: #e8e4da;
  --muted: #9c978c;
  --accent: #c9a45c;
  --err: #ef8a7a;
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.5 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
main { max-width: 640px; margin: 0 auto; padding: 32px 16px 48px; }
h1 { font-size: 2rem; font-weight: 600; margin: 0; letter-spacing: 0.01em; }
.sub { color: var(--muted); margin: 4px 0 28px; }
.style { margin-bottom: 0; min-height: 1.5em; }
.pick { display: flex; align-items: center; gap: 12px; margin: 0 0 24px; }
.pick label { color: var(--muted); min-width: 4.2em; }
#piece {
  flex: 1; min-width: 0; min-height: 44px; padding: 0 10px;
  font: inherit; color: var(--fg); background: var(--panel);
  border: 1px solid var(--line); border-radius: 10px;
}
#play {
  display: block; width: 100%; min-height: 72px;
  font: inherit; font-size: 1.4rem; font-weight: 600;
  color: var(--bg); background: var(--accent);
  border: 0; border-radius: 12px; cursor: pointer;
}
#play:focus-visible, #measure:focus-visible, #piece:focus-visible, summary:focus-visible { outline: 3px solid var(--fg); outline-offset: 3px; }
#play:disabled, #measure:disabled { opacity: 0.45; cursor: default; }
.vol { display: flex; align-items: center; gap: 12px; margin: 20px 0 8px; }
.vol label { color: var(--muted); min-width: 4.2em; }
.vol output { min-width: 2.2em; text-align: right; font-variant-numeric: tabular-nums; }
input[type=range] { flex: 1; min-width: 0; height: 44px; accent-color: var(--accent); }
.where {
  display: grid; grid-template-columns: auto 1fr; gap: 4px 16px;
  margin: 20px 0; padding: 16px; background: var(--panel); border-radius: 12px;
  font-variant-numeric: tabular-nums;
}
.where dt { color: var(--muted); }
.where dd { margin: 0; overflow-wrap: anywhere; }
.where [hidden] { display: none; }
#viz { display: block; width: 100%; height: 88px; background: var(--panel); border-radius: 12px; }
#level { color: var(--muted); margin: 0; padding: 16px; background: var(--panel); border-radius: 12px; }
#status { min-height: 1.5em; margin: 16px 0 0; color: var(--muted); font-size: 0.9rem; overflow-wrap: anywhere; }
#status.err { color: var(--err); }
#noaudio { color: var(--err); }
details { margin-top: 32px; border-top: 1px solid var(--line); padding-top: 16px; }
summary { cursor: pointer; font-size: 1.15rem; font-weight: 600; min-height: 44px; display: flex; align-items: center; }
.row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 20px; margin: 12px 0; }
.row label { display: flex; align-items: center; gap: 8px; min-height: 44px; }
.row input[type=checkbox] { width: 22px; height: 22px; accent-color: var(--accent); }
#measure {
  min-height: 48px; padding: 0 22px; font: inherit; font-weight: 600;
  color: var(--fg); background: var(--panel); border: 1px solid var(--line); border-radius: 10px; cursor: pointer;
}
#progress { color: var(--muted); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
h3 { font-size: 1rem; margin: 24px 0 8px; }
.tablewrap { overflow-x: auto; max-width: 100%; }
table { border-collapse: collapse; font-size: 0.85rem; font-variant-numeric: tabular-nums; }
th, td { padding: 4px 10px 4px 0; text-align: left; border-bottom: 1px solid var(--line); white-space: nowrap; }
thead th { color: var(--muted); font-weight: 500; }
tbody th { font-weight: 500; white-space: normal; min-width: 6.5em; }
.note { color: var(--muted); font-size: 0.85rem; }
`;

const BODY = `
<main>
  <div class="pick">
    <label for="piece">Piece</label>
    <select id="piece"></select>
  </div>
  <h1 id="title">Music</h1>
  <p id="style" class="sub style"></p>
  <p class="sub">Fall from Heaven: music played by the page from code. No sound files.</p>
  <noscript><p>This page needs JavaScript to play.</p></noscript>
  <p id="noaudio" hidden>This browser has no Web Audio (AudioContext), so the music cannot play here.</p>
  <button id="play" type="button" aria-pressed="false">Play</button>
  <div class="vol">
    <label for="vol">Volume</label>
    <input id="vol" type="range" min="0" max="100" step="1" value="70">
    <output id="volval" for="vol">70</output>
  </div>
  <dl class="where" aria-live="off">
    <dt>Time</dt><dd><span id="pos">0:00</span> / <span id="total">-</span></dd>
    <dt>Bar</dt><dd id="bar">-</dd>
    <dt id="sectiondt">Section</dt><dd id="section">-</dd>
    <dt>Playing</dt><dd id="playing">-</dd>
  </dl>
  <canvas id="viz" aria-label="Spectrum of the music"></canvas>
  <p id="level" hidden>Level: -</p>
  <p id="status" role="status"></p>
  <details>
    <summary>Measure</summary>
    <p class="note">Renders one playing of the chosen piece offline, and again without the hall (about a minute for a three-minute piece), and prints its numbers. Per voice renders each voice the piece uses alone as well, two at a time.</p>
    <div class="row">
      <button id="measure" type="button">Measure</button>
      <label><input id="pervoice" type="checkbox"> Per voice</label>
    </div>
    <p id="progress"></p>
    <div id="results"></div>
  </details>
</main>
`;

function page(modules) {
  const script = [modules, "", "// ==== demo: pure", "const DEMO = (" + demoPure.toString() + ")();", "", "// ==== demo: page", "(" + pageMain.toString() + ")();"].join("\n");
  if (/<\/script|<script|<!--/i.test(script)) throw new Refused("the page's script contains <script, </script or <!--");
  return [
    "<!doctype html>",
    '<html lang="en">',
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    '<meta name="color-scheme" content="dark">',
    "<title>Music</title>",
    '<link rel="icon" href="data:,">',
    "<style>" + CSS + "</style>",
    "</head>",
    "<body>",
    BODY,
    '<script type="module">',
    script,
    "</script>",
    "</body>",
    "</html>",
    "",
  ].join("\n");
}

// ------------------------------------------------------------------------------------------

function main(argv) {
  if (argv.includes("-h") || argv.includes("--help")) {
    console.log(USAGE);
    return 0;
  }
  if (argv.length !== 1 || argv[0].startsWith("-")) {
    console.error(USAGE);
    return 2;
  }
  const out = path.resolve(argv[0]);
  let html;
  try {
    html = page(bundle());
  } catch (e) {
    if (e instanceof Refused) {
      console.error(`music-demo: refused: ${e.message}`);
      return 1;
    }
    console.error(`music-demo: ${e.message}`);
    return 1;
  }
  try {
    fs.mkdirSync(path.dirname(out), { recursive: true });
    fs.writeFileSync(out, html);
  } catch (e) {
    console.error(`music-demo: cannot write ${out}: ${e.message}`);
    return 1;
  }
  console.log(`music-demo: wrote ${out} (${Math.round(html.length / 1024)} KB)`);
  return 0;
}

process.exitCode = main(process.argv.slice(2));
