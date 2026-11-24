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
export function createOrchestra(ctx, { seed = 7, mix: levels = MIX } = {}) {
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
  const mix = gainNode(TRIM);
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
  hall.connect(gainNode(HALL)).connect(mix);

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
  const harp = (e) => {
    let buf = strings.get(e.m);
    if (!buf) {
      const s = pluckSamples(sr, e.m);
      buf = ctx.createBuffer(1, s.length, sr);
      buf.getChannelData(0).set(s);
      strings.set(e.m, buf);
    }
    const src = ctx.createBufferSource();
    src.buffer = buf;
    // A soft pluck is a duller one.
    const tone = filter("lowpass", clamp(2500 + hz(e.m) * (4 + 14 * e.v), 3000, 16000), 0.5);
    const place = panner((e.m - 62) / 44);
    src.connect(tone);
    tone.connect(gainNode(e.v)).connect(place);
    place.connect(buses.harp);
    src.start(e.t);
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
      envelope(env.gain, e.t, e.d, e.v / n, (e.v2 ?? e.v) / n, kind.attack, kind.tau);
    };
  };
  const violins = bowed({ bus: buses.violins, detune: [-8, 3, 10], vibrato: 7, attack: 0.16, tau: 0.13, cut: 2600, track: 2.5, width: 0.5 });
  const pad = bowed({ bus: buses.pad, detune: [-9, 0, 8], vibrato: 4, attack: 0.55, tau: 0.3, cut: 900, track: 3, width: 1.2 });
  const bass = bowed({ bus: buses.bass, detune: [-5, 6], vibrato: 0, attack: 0.3, tau: 0.28, cut: 400, track: 6, width: 0.2 });

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

  // --- the horn: two players, a filter that opens as the note speaks and the louder it is
  const horn = (e) => {
    const f = hz(e.m);
    const end = e.t + e.d;
    const tone = filter("lowpass", f * 1.2, 0.8);
    tone.frequency.setValueAtTime(f * 1.2, e.t);
    tone.frequency.linearRampToValueAtTime(f * (2.2 + 4 * e.v), e.t + Math.min(0.13, e.d * 0.5));
    tone.frequency.linearRampToValueAtTime(f * (2.2 + 4 * (e.v2 ?? e.v)), end);
    tone.frequency.setTargetAtTime(f * 1.2, end, 0.12);
    const env = gainNode(0);
    [-4, 5].forEach((cents, i) => {
      const o = ctx.createOscillator();
      o.setPeriodicWave(waves.horn);
      o.frequency.value = f;
      o.detune.value = cents;
      o.connect(tone);
      o.start(e.t);
      o.stop(end + 0.8);
      if (i === 0) o.onended = () => env.disconnect();
    });
    tone.connect(env);
    env.connect(buses.horn);
    envelope(env.gain, e.t, e.d, e.v / 2, (e.v2 ?? e.v) / 2, 0.09, 0.09);
  };

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

  const voices = { harp, violins, pad, bass, choirAh: choir, choirOo: choir, flute, horn, timpani, celesta };

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
