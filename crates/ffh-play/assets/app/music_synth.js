// The orchestra (docs/MUSIC.md): every instrument of the game's music, made of Web Audio nodes and
// a little arithmetic, so the client ships no sound files. A harp of plucked strings computed
// sample by sample, strings and horns from detuned band-limited waves under a filter, a choir
// through vowel formants, a flute with its breath, a drum, a celesta, and the hall they sit in (a
// computed impulse response). `createOrchestra(ctx)` works on any context: the page's
// AudioContext, or an OfflineAudioContext for measuring a piece (music.js `renderPiece`).

import { mulberry32 } from "./music_score.js";

const hz = (m) => 440 * 2 ** ((m - 69) / 12);
const clamp = (x, a, b) => Math.min(b, Math.max(a, x));

/** Each voice's level in the mix, where it sits (-1 left, 1 right) and how much of it the hall gets. */
export const MIX = {
  harp: { level: 0.45, pan: -0.15, hall: 0.42 },
  flute: { level: 0.36, pan: -0.1, hall: 0.55 },
  horn: { level: 0.42, pan: 0.3, hall: 0.55 },
  violins: { level: 0.3, pan: -0.3, hall: 0.5 },
  pad: { level: 0.24, pan: 0.05, hall: 0.55 },
  bass: { level: 0.32, pan: 0.2, hall: 0.25 },
  choirAh: { level: 0.7, pan: 0, hall: 0.8 },
  choirOo: { level: 0.55, pan: 0, hall: 0.8 },
  timpani: { level: 0.38, pan: 0.15, hall: 0.45 },
  celesta: { level: 0.2, pan: 0.35, hall: 0.6 },
  // The other pieces' instruments (music_pieces.js).
  spiccato: { level: 0.34, pan: 0.1, hall: 0.3 },
  marcato: { level: 0.3, pan: -0.3, hall: 0.4 },
  fiddle: { level: 0.26, pan: -0.25, hall: 0.4 },
  whistle: { level: 0.3, pan: 0.15, hall: 0.4 },
  brass: { level: 0.34, pan: 0.25, hall: 0.45 },
  lute: { level: 0.42, pan: 0.2, hall: 0.3 },
  drum: { level: 0.5, pan: 0, hall: 0.25 },
  snare: { level: 0.9, pan: 0.1, hall: 0.3 },
  shaker: { level: 0.6, pan: -0.3, hall: 0.2 },
  cymbal: { level: 0.5, pan: -0.2, hall: 0.5 },
  bell: { level: 0.45, pan: 0.1, hall: 0.7 },
  chipLead: { level: 0.55, pan: 0, hall: 0.05 },
  chipHarm: { level: 0.24, pan: 0, hall: 0.05 },
  chipBass: { level: 0.26, pan: 0, hall: 0.03 },
  chipNoise: { level: 0.66, pan: 0, hall: 0.04 },
};

/** The mix a piece is played with: `MIX`, with the piece's own `mix` laid over it voice by voice
 *  (a battle wants its strings drier than a vigil does). */
export function mixFor(piece) {
  const out = { ...MIX };
  for (const [voice, m] of Object.entries(piece?.mix ?? {})) out[voice] = { ...MIX[voice], ...m };
  return out;
}

/** What `createOrchestra` takes for a piece: its mix and, where the piece has them, its own `trim`
 *  (the level before the soft ceiling) and `hall` (the hall's level against the dry sound). */
export function settingsFor(piece) {
  const out = { mix: mixFor(piece) };
  if (piece?.trim !== undefined) out.trim = piece.trim;
  if (piece?.hall !== undefined) out.hall = piece.hall;
  return out;
}

/** The notes each voice is written for, lowest and highest (MIDI); a voice without a pitch (a
 *  snare, a shaker) is not listed. A piece that leaves them is caught by the tests. */
export const RANGES = {
  harp: [24, 103], flute: [60, 96], horn: [41, 77], violins: [55, 100], pad: [48, 84], bass: [28, 55],
  choirAh: [40, 81], choirOo: [40, 81], timpani: [38, 57], celesta: [60, 108],
  spiccato: [28, 79], marcato: [55, 100], fiddle: [55, 96], brass: [43, 84], lute: [36, 84], drum: [36, 60],
  whistle: [62, 98], bell: [45, 84], chipLead: [55, 96], chipHarm: [48, 91], chipBass: [28, 60],
};

/** The whole mix's level before the soft ceiling: set so the loudest bars stay under it. */
const TRIM = 0.7;

/** The hall's level against the dry sound. */
const HALL = 1.5;

// ---------------------------------------------------------------------------------------------
// The harp: a plucked string

/** One plucked string as samples (Karplus-Strong: a delay line as long as the note's period, fed
 *  back through a two-point average that wears the high harmonics away first, and an allpass that
 *  makes up the fraction of a sample the pitch needs). The line starts as the string's shape when
 *  the finger lets go: a pull at about a third of its length, smoothed by the fingertip's width. */
export function pluckSamples(sampleRate, m) {
  const f = hz(m);
  const period = sampleRate / f;
  const len = Math.floor(period - 0.6);
  const frac = period - 0.5 - len;
  const c = (1 - frac) / (1 + frac);
  // Low strings ring long, high ones briefly.
  const t60 = 6 * 2 ** (-(m - 36) / 30);
  const n = Math.floor(Math.min(6, t60 * 0.85 + 0.3) * sampleRate);
  const rho = Math.min(0.99999, 10 ** (-3 / (f * t60)) / Math.cos((Math.PI * f) / sampleRate));

  // One period of the wave on the string. The line holds a round trip (there and back, turned
  // over at each end), so the second half mirrors the first with its sign changed. What reaches
  // the soundboard is the string's slope at the bridge: a pulse wave, its edges rounded.
  const half = len / 2;
  const at = 0.36 * half;
  let line = new Float32Array(len);
  for (let i = 0; i < len; i++) {
    const x = i < half ? i : len - i;
    const slope = x < at ? 1 : -at / (half - at);
    const pull = x < at ? x / at : (half - x) / (half - at);
    line[i] = 0.75 * slope + 0.25 * (i < half ? pull : -pull);
  }
  // The fingertip: two passes of an average as wide as the harmonics to keep allow.
  const keep = clamp(9000 / f, 8, 60);
  const w = Math.max(1, Math.round(len / keep));
  for (let pass = 0; pass < 2 && w > 1; pass++) {
    const smooth = new Float32Array(len);
    let sum = 0;
    for (let k = 0; k < w; k++) sum += line[k % len];
    for (let i = 0; i < len; i++) {
      smooth[(i + (w >> 1)) % len] = sum / w;
      sum += line[(i + w) % len] - line[i];
    }
    line = smooth;
  }
  let mean = 0;
  for (let i = 0; i < len; i++) mean += line[i];
  mean /= len;
  // Start where the wave crosses zero, so the note begins without a step.
  let zero = 0;
  for (let i = 0; i < len; i++) {
    const a = line[i] - mean;
    const b = line[(i + 1) % len] - mean;
    if (a <= 0 && b > 0) {
      zero = (i + 1) % len;
      break;
    }
  }
  const start = new Float32Array(len);
  for (let i = 0; i < len; i++) start[i] = line[(i + zero) % len] - mean;
  line = start;

  const out = new Float32Array(n);
  let idx = 0;
  let x1 = line[len - 1];
  let apIn = x1;
  let apOut = x1;
  for (let i = 0; i < n; i++) {
    const x = line[idx];
    out[i] = x;
    const avg = 0.5 * (x + x1);
    x1 = x;
    const y = c * avg + apIn - c * apOut;
    apIn = avg;
    apOut = y;
    line[idx] = y * rho;
    if (++idx === len) idx = 0;
  }
  // The ends: in over a moment (a short string's wave has no sample at its zero), out to nothing.
  const rise = Math.min(24, len >> 1);
  const fade = Math.min(n, Math.floor(0.05 * sampleRate));
  let peak = 0;
  for (let i = 0; i < n; i++) {
    out[i] *= (i < rise ? i / rise : 1) * (i >= n - fade ? (n - 1 - i) / fade : 1);
    if (Math.abs(out[i]) > peak) peak = Math.abs(out[i]);
  }
  const gain = peak > 0 ? 1 / peak : 0;
  for (let i = 0; i < n; i++) out[i] *= gain;
  return out;
}

// ---------------------------------------------------------------------------------------------
// The hall

/** The hall's impulse response: a few early reflections, then noise that dies away, the highs
 *  first (3.6 s in the bass, 1.3 s at the top), different in each ear. */
function hallImpulse(ctx, rng) {
  const sr = ctx.sampleRate;
  const n = Math.floor(sr * 3.8);
  const buf = ctx.createBuffer(2, n, sr);
  const aLow = 1 - Math.exp((-2 * Math.PI * 450) / sr);
  const aMid = 1 - Math.exp((-2 * Math.PI * 3200) / sr);
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch);
    let low = 0;
    let mid = 0;
    for (let i = 0; i < n; i++) {
      const t = i / sr;
      const white = rng() * 2 - 1;
      low += aLow * (white - low);
      mid += aMid * (white - mid);
      const build = 1 - Math.exp(-t / 0.018);
      d[i] = build * (low * Math.exp((-6.91 * t) / 3.6) + (mid - low) * Math.exp((-6.91 * t) / 2.9) + (white - mid) * Math.exp((-6.91 * t) / 1.3));
    }
    [0.0113, 0.0171, 0.0233, 0.0317, 0.0409, 0.0531, 0.0673].forEach((t, k) => {
      const i = Math.floor((t + ch * 0.0019 * (k % 3)) * sr);
      d[i] += (k % 2 === ch ? 1 : -0.7) * (1.6 - k * 0.18);
    });
    const fade = Math.floor(0.2 * sr);
    for (let i = n - fade; i < n; i++) d[i] *= (n - i) / fade;
  }
  return buf;
}

/** The last stage's curve: nothing below a moderate level, then a soft ceiling (0.89) in place of
 *  clipping. The node before it halves the signal, so the curve spans inputs up to 2. */
function softClip() {
  const n = 4097;
  const curve = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const x = ((i / (n - 1)) * 2 - 1) * 2;
    curve[i] = Math.tanh(1.1 * x) / 1.1;
  }
  return curve;
}

// ---------------------------------------------------------------------------------------------
// The orchestra

/** The orchestra on `ctx`. `play(event)` sounds one note of a compiled piece (music_score.js
 *  `compile`: `{ voice, t, d, m, v, v2 }`, `t` in the context's seconds); a voice's notes must
 *  come in order of time. `out` is the last node before the speakers (its gain is the listener's
 *  volume). */
export function createOrchestra(ctx, { seed = 7, mix: levels = MIX, trim = TRIM, hall: hallLevel = HALL } = {}) {
  const rng = mulberry32(seed);
  const sr = ctx.sampleRate;

  const gainNode = (v) => {
    const g = ctx.createGain();
    g.gain.value = v;
    return g;
  };
  const filter = (type, freq, q = 0.7) => {
    const f = ctx.createBiquadFilter();
    f.type = type;
    f.frequency.value = freq;
    f.Q.value = q;
    return f;
  };
  const panner = (pan) => {
    if (!ctx.createStereoPanner) return gainNode(1);
    const p = ctx.createStereoPanner();
    p.pan.value = clamp(pan, -1, 1);
    return p;
  };

  // The way out: the mix, the soft ceiling, the listener's volume.
  const mix = gainNode(trim);
  const halve = gainNode(0.5);
  const ceiling = ctx.createWaveShaper();
  ceiling.curve = softClip();
  ceiling.oversample = "2x";
  const out = gainNode(1);
  mix.connect(halve);
  halve.connect(ceiling);
  ceiling.connect(out);
  out.connect(ctx.destination);

  // The hall: no rumble into it, a moment before it answers.
  const hallIn = filter("highpass", 150, 0.5);
  const before = ctx.createDelay(0.1);
  before.delayTime.value = 0.022;
  const hall = ctx.createConvolver();
  hall.buffer = hallImpulse(ctx, rng);
  hallIn.connect(before);
  before.connect(hall);
  hall.connect(gainNode(hallLevel)).connect(mix);

  const buses = {};
  for (const [voice, m] of Object.entries(levels)) {
    const input = gainNode(m.level);
    const place = panner(m.pan);
    input.connect(place);
    place.connect(mix);
    place.connect(gainNode(m.hall)).connect(hallIn);
    buses[voice] = input;
  }

  // Shared sources: noise (the flute's breath, the drum's stick) and the slow wobble of bowed
  // and sung notes.
  const noiseBuf = ctx.createBuffer(1, sr * 2, sr);
  const nd = noiseBuf.getChannelData(0);
  for (let i = 0; i < nd.length; i++) nd[i] = rng() * 2 - 1;
  const noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
  noise.loop = true;
  noise.start();
  const wobble = [4.7, 5.3, 5.9].map((rate) => {
    const o = ctx.createOscillator();
    o.frequency.value = rate;
    o.start();
    return o;
  });
  /** The wobbles at a depth in cents: one per player of a note. */
  const vibrato = (cents) => wobble.map((o) => o.connect(gainNode(cents)));

  // Band-limited waves: `amp(k)` is the level of the k-th harmonic.
  const wave = (count, amp) => {
    const re = new Float32Array(count + 1);
    const im = new Float32Array(count + 1);
    for (let k = 1; k <= count; k++) im[k] = amp(k);
    return ctx.createPeriodicWave(re, im);
  };
  const waves = {
    bowed: wave(48, (k) => Math.exp(-k / 26) / k),
    sung: wave(40, (k) => 1 / k ** 1.15),
    flute: wave(6, (k) => [0, 1, 0.3, 0.1, 0.04, 0.015, 0.008][k]),
    horn: wave(24, (k) => Math.exp(-k / 7) / k),
    brass: wave(32, (k) => Math.exp(-k / 12) / k ** 0.85),
    whistle: wave(4, (k) => [0, 1, 0.16, 0.05, 0.02][k]),
    // A pulse a quarter of the period wide: the thin, nasal square of an old game console.
    pulse: wave(48, (k) => (2 / (k * Math.PI)) * Math.sin(k * Math.PI * 0.25)),
  };

  /** When a note's oscillator has ended, lets go of what fed it and what it fed: the shared
   *  wobbles stay for the whole piece and must not keep every note they ever touched. */
  const release = (o, wobbling, last) => {
    o.onended = () => {
      try {
        wobbling?.disconnect(o.detune);
        o.disconnect();
        last?.disconnect();
      } catch {
        /* already apart */
      }
    };
  };

  /** A note's loudness: up to `a`, on to `b` by its end, then away. `tau` is how fast it leaves. */
  const envelope = (param, t, d, a, b, attack, tau) => {
    param.setValueAtTime(0, t);
    param.linearRampToValueAtTime(a, t + Math.min(attack, d * 0.5));
    param.linearRampToValueAtTime(b, t + d);
    param.setTargetAtTime(0, t + d, tau);
  };

  // --- the harp
  const strings = new Map();
  const stringOf = (m) => {
    let buf = strings.get(m);
    if (!buf) {
      const s = pluckSamples(sr, m);
      buf = ctx.createBuffer(1, s.length, sr);
      buf.getChannelData(0).set(s);
      strings.set(m, buf);
    }
    return buf;
  };
  const harp = (e) => {
    const src = ctx.createBufferSource();
    src.buffer = stringOf(e.m);
    // A soft pluck is a duller one.
    const tone = filter("lowpass", clamp(2500 + hz(e.m) * (4 + 14 * e.v), 3000, 16000), 0.5);
    const place = panner((e.m - 62) / 44);
    src.connect(tone);
    tone.connect(gainNode(e.v)).connect(place);
    place.connect(buses.harp);
    src.start(e.t);
  };

  // --- the lute: the same string, brighter, and the hand stops it when the note is over
  const lute = (e) => {
    const src = ctx.createBufferSource();
    src.buffer = stringOf(e.m);
    const tone = filter("lowpass", clamp(1800 + hz(e.m) * (5 + 10 * e.v), 2500, 12000), 0.5);
    const g = gainNode(e.v);
    const end = e.t + Math.max(e.d, 0.06);
    g.gain.setValueAtTime(e.v, end);
    g.gain.setTargetAtTime(0, end, 0.09);
    src.connect(tone);
    tone.connect(g);
    g.connect(buses.lute);
    src.start(e.t);
    src.stop(end + 0.8);
    src.onended = () => g.disconnect();
  };

  // --- the bowed strings: a few players a note, each a little out of tune with the others
  const bowed = (kind) => {
    const vib = kind.vibrato ? vibrato(kind.vibrato) : [];
    return (e) => {
      const f = hz(e.m);
      const env = gainNode(0);
      const tone = filter("lowpass", clamp(kind.cut + kind.track * f, 300, 7000), 0.6);
      const stop = e.t + e.d + kind.tau * 7;
      kind.detune.forEach((cents, i) => {
        const o = ctx.createOscillator();
        o.setPeriodicWave(waves.bowed);
        o.frequency.value = f;
        o.detune.value = cents + (rng() - 0.5) * 5;
        vib[i % 3]?.connect(o.detune);
        o.connect(tone);
        o.start(e.t);
        o.stop(stop);
        release(o, vib[i % 3], i === 0 ? env : null);
      });
      const place = panner((rng() - 0.5) * kind.width);
      tone.connect(env);
      env.connect(place);
      place.connect(kind.bus);
      const n = kind.detune.length;
      envelope(env.gain, e.t, e.d, e.v / n, ((e.v2 ?? e.v) * (kind.fall ?? 1)) / n, kind.attack, kind.tau);
    };
  };
  const violins = bowed({ bus: buses.violins, detune: [-8, 3, 10], vibrato: 7, attack: 0.16, tau: 0.13, cut: 2600, track: 2.5, width: 0.5 });
  const pad = bowed({ bus: buses.pad, detune: [-9, 0, 8], vibrato: 4, attack: 0.55, tau: 0.3, cut: 900, track: 3, width: 1.2 });
  const bass = bowed({ bus: buses.bass, detune: [-5, 6], vibrato: 0, attack: 0.3, tau: 0.28, cut: 400, track: 6, width: 0.2 });
  // Short strokes off the string (`fall`: how far the note has died by its end), marked ones, and
  // one fiddler.
  const spiccato = bowed({ bus: buses.spiccato, detune: [-7, 6], vibrato: 0, attack: 0.012, tau: 0.04, cut: 1800, track: 4, width: 0.5, fall: 0.4 });
  const marcato = bowed({ bus: buses.marcato, detune: [-8, 3, 10], vibrato: 5, attack: 0.03, tau: 0.07, cut: 2800, track: 2.5, width: 0.4, fall: 0.85 });
  const fiddle = bowed({ bus: buses.fiddle, detune: [0, 5], vibrato: 7, attack: 0.025, tau: 0.05, cut: 3200, track: 3, width: 0 });

  // --- the choir: voices through the mouth's resonances (the formants of a vowel)
  const vowel = (bus, formants, chest) => {
    const input = gainNode(1);
    for (const [freq, q, level] of formants) {
      const band = filter("bandpass", freq, q);
      input.connect(band);
      band.connect(gainNode(level)).connect(bus);
    }
    const low = filter("lowpass", 420, 0.5);
    input.connect(low);
    low.connect(gainNode(chest)).connect(bus);
    return input;
  };
  const mouths = {
    choirAh: vowel(buses.choirAh, [[760, 7, 1], [1150, 9, 0.5], [2700, 12, 0.25], [3500, 12, 0.1]], 0.12),
    choirOo: vowel(buses.choirOo, [[360, 5, 1], [760, 8, 0.22], [2500, 12, 0.03]], 0.2),
  };
  const sungVib = vibrato(9);
  const choir = (e) => {
    const f = hz(e.m);
    const env = gainNode(0);
    const stop = e.t + e.d + 3;
    [-8, 7].forEach((cents, i) => {
      const o = ctx.createOscillator();
      o.setPeriodicWave(waves.sung);
      o.frequency.value = f;
      o.detune.value = cents + (rng() - 0.5) * 8;
      const wobbling = sungVib[(i + (e.m % 2)) % 3];
      wobbling.connect(o.detune);
      o.connect(env);
      o.start(e.t);
      o.stop(stop);
      release(o, wobbling, i === 0 ? env : null);
    });
    env.connect(mouths[e.voice]);
    envelope(env.gain, e.t, e.d, e.v / 2, (e.v2 ?? e.v) / 2, 0.5, 0.4);
  };

  // --- the flute: nearly a pure tone, a vibrato that arrives once the note is under way, and
  // the player's breath (one stream for the instrument: it plays one note at a time)
  const breathTone = filter("bandpass", 2000, 1.2);
  const breath = gainNode(0);
  noise.connect(breathTone);
  breathTone.connect(breath);
  breath.connect(buses.flute);
  const flute = (e) => {
    const f = hz(e.m);
    const end = e.t + e.d;
    const o = ctx.createOscillator();
    o.setPeriodicWave(waves.flute);
    o.frequency.value = f;
    const lfo = ctx.createOscillator();
    lfo.frequency.value = 4.8 + rng() * 0.6;
    const depth = gainNode(0);
    if (e.d > 0.45) {
      depth.gain.setValueAtTime(0, e.t + 0.15);
      depth.gain.linearRampToValueAtTime(e.d > 0.9 ? 12 : 6, e.t + Math.min(0.75, e.d * 0.7));
    }
    lfo.connect(depth);
    depth.connect(o.detune);
    const env = gainNode(0);
    o.connect(env);
    env.connect(buses.flute);
    envelope(env.gain, e.t, e.d, e.v, e.v2 ?? e.v, 0.07, 0.06);
    o.start(e.t);
    o.stop(end + 0.5);
    lfo.start(e.t);
    lfo.stop(end + 0.5);
    o.onended = () => env.disconnect();
    breathTone.frequency.setValueAtTime(clamp(f * 2.5, 900, 6000), e.t);
    breath.gain.setTargetAtTime(e.v * 0.4, e.t, 0.006);
    breath.gain.setTargetAtTime(e.v * 0.06, e.t + 0.035, 0.05);
    breath.gain.setTargetAtTime(0, end, 0.05);
  };

  // --- the brass: two players, a filter that opens as the note speaks and the louder it is
  const brassy = (kind) => (e) => {
    const f = hz(e.m);
    const end = e.t + e.d;
    const tone = filter("lowpass", f * 1.2, 0.8);
    tone.frequency.setValueAtTime(f * 1.2, e.t);
    tone.frequency.linearRampToValueAtTime(f * (kind.open + kind.more * e.v), e.t + Math.min(kind.rise, e.d * 0.5));
    tone.frequency.linearRampToValueAtTime(f * (kind.open + kind.more * (e.v2 ?? e.v)), end);
    tone.frequency.setTargetAtTime(f * 1.2, end, kind.close);
    const env = gainNode(0);
    kind.detune.forEach((cents, i) => {
      const o = ctx.createOscillator();
      o.setPeriodicWave(kind.wave);
      o.frequency.value = f;
      o.detune.value = cents;
      o.connect(tone);
      o.start(e.t);
      o.stop(end + 0.8);
      if (i === 0) o.onended = () => env.disconnect();
    });
    tone.connect(env);
    env.connect(kind.bus);
    envelope(env.gain, e.t, e.d, e.v / 2, (e.v2 ?? e.v) / 2, kind.attack, kind.tau);
  };
  const horn = brassy({ bus: buses.horn, wave: waves.horn, detune: [-4, 5], open: 2.2, more: 4, rise: 0.13, close: 0.12, attack: 0.09, tau: 0.09 });
  // Trumpets and trombones: brighter, and quick to speak.
  const brass = brassy({ bus: buses.brass, wave: waves.brass, detune: [-3, 4], open: 3, more: 6, rise: 0.05, close: 0.06, attack: 0.03, tau: 0.06 });

  /** A struck note: sine partials `[ratio, level, seconds to die]` that start together. */
  const struck = (bus, partials, bend) => (e) => {
    const f = hz(e.m);
    for (const [ratio, level, seconds] of partials) {
      const o = ctx.createOscillator();
      const g = gainNode(0);
      if (bend) {
        o.frequency.setValueAtTime(f * ratio * bend, e.t);
        o.frequency.exponentialRampToValueAtTime(f * ratio, e.t + 0.05);
      } else {
        o.frequency.value = f * ratio;
      }
      g.gain.setValueAtTime(0, e.t);
      g.gain.linearRampToValueAtTime(level * e.v, e.t + 0.004);
      g.gain.setTargetAtTime(0, e.t + 0.004, seconds / 6.9);
      o.connect(g);
      g.connect(bus);
      o.start(e.t);
      o.stop(e.t + seconds * 1.3 + 0.1);
      o.onended = () => g.disconnect();
    }
  };

  // --- the drum (a kettledrum's partials: the fifth and the octave over the note) and its stick
  const drumHead = struck(buses.timpani, [[1, 1, 1.7], [1.5, 0.45, 1], [1.99, 0.28, 0.6], [2.44, 0.12, 0.35]], 1.12);
  const stickTone = filter("lowpass", 500, 0.5);
  const stick = gainNode(0);
  noise.connect(stickTone);
  stickTone.connect(stick);
  stick.connect(buses.timpani);
  const timpani = (e) => {
    drumHead(e);
    stick.gain.setValueAtTime(0, e.t);
    stick.gain.linearRampToValueAtTime(e.v * 0.6, e.t + 0.002);
    stick.gain.setTargetAtTime(0, e.t + 0.002, 0.03);
  };

  // --- the celesta: a struck bar, bright for a moment
  const celesta = struck(buses.celesta, [[1, 1, 2.2], [2, 0.3, 1.2], [4.07, 0.2, 0.35], [6.3, 0.08, 0.12]], 0);

  // --- a bell: the partials of a cast bell (the hum an octave under the note, a minor third in it)
  const bell = struck(buses.bell, [[0.5, 0.35, 5.5], [1, 1, 4.5], [1.183, 0.6, 3.2], [1.506, 0.45, 2.6], [2, 0.5, 2.2], [2.514, 0.3, 1.5], [2.662, 0.22, 1.2], [3.011, 0.18, 0.9], [4.166, 0.1, 0.6]], 0);

  /** A hit of noise: a stretch of the shared noise through a filter, up at once and away. Each hit
   *  has its own source, so hits may overlap. */
  const burst = (bus, t, { type, freq, q = 0.7, level, attack = 0.002, tau }) => {
    const src = ctx.createBufferSource();
    src.buffer = noiseBuf;
    src.loop = true;
    const tone = filter(type, freq, q);
    const g = gainNode(0);
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(level, t + attack);
    g.gain.setTargetAtTime(0, t + attack, tau);
    src.connect(tone);
    tone.connect(g);
    g.connect(bus);
    src.start(t, rng() * 1.5);
    src.stop(t + attack + tau * 8);
    src.onended = () => g.disconnect();
  };

  // --- the whistle: the flute's cousin for a dance, quick to speak, a puff of air at each note
  const whistle = (e) => {
    const f = hz(e.m);
    const end = e.t + e.d;
    const o = ctx.createOscillator();
    o.setPeriodicWave(waves.whistle);
    o.frequency.value = f;
    if (e.d > 0.4) {
      const lfo = ctx.createOscillator();
      lfo.frequency.value = 5.2;
      const depth = gainNode(0);
      depth.gain.setValueAtTime(0, e.t + 0.15);
      depth.gain.linearRampToValueAtTime(9, e.t + 0.35);
      lfo.connect(depth);
      depth.connect(o.detune);
      lfo.start(e.t);
      lfo.stop(end + 0.3);
    }
    const env = gainNode(0);
    o.connect(env);
    env.connect(buses.whistle);
    envelope(env.gain, e.t, e.d, e.v, e.v2 ?? e.v, 0.018, 0.035);
    o.start(e.t);
    o.stop(end + 0.3);
    o.onended = () => env.disconnect();
    burst(buses.whistle, e.t, { type: "bandpass", freq: clamp(f * 2.5, 1500, 7000), q: 1.5, level: e.v * 0.25, tau: 0.015 });
  };

  // --- the small drums: a hand drum (its skin and the hand), a snare (its shell and its wires), a
  // shaker, a cymbal. The snare, the shaker and the cymbal have no pitch: the note is ignored.
  const drumSkin = struck(buses.drum, [[1, 1, 0.45], [1.52, 0.4, 0.22], [2.4, 0.15, 0.1]], 1.3);
  const drum = (e) => {
    drumSkin(e);
    burst(buses.drum, e.t, { type: "lowpass", freq: 900, level: e.v * 0.5, tau: 0.02 });
  };
  const snareShell = struck(buses.snare, [[1, 0.5, 0.11], [1.59, 0.3, 0.07]], 1.25);
  const snare = (e) => {
    snareShell({ ...e, m: 54 });
    burst(buses.snare, e.t, { type: "bandpass", freq: 2100, q: 0.6, level: e.v * 0.9, tau: 0.045 });
    burst(buses.snare, e.t, { type: "highpass", freq: 5200, level: e.v * 0.3, tau: 0.03 });
  };
  const shaker = (e) => burst(buses.shaker, e.t, { type: "bandpass", freq: 7200, q: 1.1, level: e.v, attack: 0.004, tau: 0.018 });
  const cymbal = (e) => {
    burst(buses.cymbal, e.t, { type: "highpass", freq: 4200, level: e.v, attack: 0.003, tau: 0.4 });
    burst(buses.cymbal, e.t, { type: "bandpass", freq: 9000, q: 2, level: e.v * 0.5, tau: 0.25 });
  };

  // --- the chip: the three voices and the noise of an old game console. A note starts at once,
  // sinks to `hold` of its level and stops dead; a long one wavers.
  const chip = (kind) => (e) => {
    const end = e.t + e.d;
    const o = ctx.createOscillator();
    if (kind.wave) o.setPeriodicWave(kind.wave);
    else o.type = kind.type;
    o.frequency.value = hz(e.m);
    const g = gainNode(0);
    g.gain.setValueAtTime(0, e.t);
    g.gain.linearRampToValueAtTime(e.v, e.t + 0.003);
    g.gain.setTargetAtTime(e.v * kind.hold, e.t + 0.003, 0.06);
    g.gain.setTargetAtTime(0, Math.max(e.t + 0.01, end - 0.012), 0.006);
    if (kind.waver && e.d > 0.5) {
      const lfo = ctx.createOscillator();
      lfo.frequency.value = 6;
      const depth = gainNode(0);
      depth.gain.setValueAtTime(0, e.t + 0.25);
      depth.gain.linearRampToValueAtTime(kind.waver, e.t + 0.4);
      lfo.connect(depth);
      depth.connect(o.detune);
      lfo.start(e.t);
      lfo.stop(end + 0.08);
    }
    o.connect(g);
    g.connect(kind.bus);
    o.start(e.t);
    o.stop(end + 0.08);
    o.onended = () => g.disconnect();
  };
  const chipLead = chip({ bus: buses.chipLead, wave: waves.pulse, hold: 0.7, waver: 10 });
  const chipHarm = chip({ bus: buses.chipHarm, type: "square", hold: 0.5 });
  const chipBass = chip({ bus: buses.chipBass, type: "triangle", hold: 1 });
  /** The console's drums, by the note: low a kick (a tone that drops), middle a snare, high a hat. */
  const chipNoise = (e) => {
    if (e.m >= 72) return burst(buses.chipNoise, e.t, { type: "highpass", freq: 7000, level: e.v * 0.45, tau: 0.012 });
    if (e.m >= 48) return burst(buses.chipNoise, e.t, { type: "bandpass", freq: 2400, q: 0.5, level: e.v * 0.8, tau: 0.04 });
    const o = ctx.createOscillator();
    o.frequency.setValueAtTime(150, e.t);
    o.frequency.exponentialRampToValueAtTime(42, e.t + 0.09);
    const g = gainNode(0);
    g.gain.setValueAtTime(0, e.t);
    g.gain.linearRampToValueAtTime(e.v, e.t + 0.002);
    g.gain.setTargetAtTime(0, e.t + 0.002, 0.045);
    o.connect(g);
    g.connect(buses.chipNoise);
    o.start(e.t);
    o.stop(e.t + 0.4);
    o.onended = () => g.disconnect();
  };

  const voices = {
    harp, violins, pad, bass, choirAh: choir, choirOo: choir, flute, horn, timpani, celesta,
    spiccato, marcato, fiddle, whistle, brass, lute, drum, snare, shaker, cymbal, bell, chipLead, chipHarm, chipBass, chipNoise,
  };

  return {
    ctx,
    out,
    /** Sounds one note; a voice the orchestra does not have is left out. */
    play(e) {
      voices[e.voice]?.(e);
    },
    /** Stops the shared sources and lets go of the speakers. */
    close() {
      noise.stop();
      for (const o of wobble) o.stop();
      out.disconnect();
    },
  };
}
