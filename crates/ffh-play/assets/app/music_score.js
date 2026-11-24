// The game's music, written down (docs/MUSIC.md). A piece is its bars (a chord and a tempo each),
// the figures each chord gives the accompaniment (the harp's notes, the strings' and the choir's
// voicing, the bass), the melodic lines, and how loud each layer is from bar to bar. `compile`
// turns a piece into timed notes for the orchestra (music_synth.js). Data and arithmetic only: no
// Web Audio here, so the node tests read it (test/music.test.mjs).

const STEP = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };

/** A note name as its MIDI number: "C4" is 60, "Bb3" 58, "F#5" 78. */
export function midi(name) {
  const m = /^([A-G])([#b]?)(-?\d)$/.exec(name);
  if (!m) throw new Error(`music: not a note: "${name}"`);
  return STEP[m[1]] + (m[2] === "#" ? 1 : m[2] === "b" ? -1 : 0) + 12 * (Number(m[3]) + 1);
}

/** A small seeded generator (the humanizing and the orchestra's noise): the same seed, the same
 *  performance. Not the game's RNG: nothing here touches the simulation. */
export function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** The dynamic marks a line may carry (`<mf>`), as velocities. */
export const MARKS = { pp: 0.22, p: 0.34, mp: 0.46, mf: 0.6, f: 0.76, ff: 0.9 };

const words = (s) => s.trim().split(/\s+/);
/** "A3 D4 F4" → [57, 62, 65]; "-" is a rest (null). */
const pitches = (s) => words(s).map((w) => (w === "-" ? null : midi(w)));
/** "A3 D4 E4 | A3 C#4 E4" → the two halves of a bar; one voicing → one whole bar. */
const halves = (s) => s.split("|").map(pitches);

// ---------------------------------------------------------------------------------------------
// Vigil
//
// D minor, four in a bar, 63 to the minute. A harp alone states the ground (a bass that walks down
// from D to A); a flute sings the theme over it; the strings and a choir join for its repeat; the
// horn answers in the relative major; the full orchestra returns with the theme and turns aside at
// the cadence (B flat for D minor), and the coda walks B flat, C, D up into the major. It ends on
// the third of D major and a silence, and starts again.

/** The chords of Vigil. `bass`: the low strings' note (two for a bar of two chords). `harp`: the
 *  bar in eighths. `pad`: the voicing the strings hold, and the choir with them. `ladder`: five
 *  notes the harp's sixteenth sweep climbs (and their octaves above), or `sweep`: the sixteen
 *  written out. */
const VIGIL_CHORDS = {
  "Dm": { bass: "D2", harp: "D3 A3 D4 E4 F4 E4 D4 A3", pad: "A3 D4 F4", ladder: "D3 A3 D4 E4 F4" },
  "Dm/C": { bass: "C2", harp: "C3 A3 D4 E4 F4 E4 D4 A3", pad: "A3 D4 F4", ladder: "C3 A3 D4 E4 F4" },
  "BbM7": { bass: "Bb1", harp: "Bb2 F3 A3 C4 D4 C4 A3 F3", pad: "A3 D4 F4", ladder: "Bb2 F3 A3 C4 D4" },
  "Asus-A": { bass: "A1", harp: "A2 E3 A3 D4 E4 C#4 A3 E3", pad: "A3 D4 E4 | A3 C#4 E4", sweep: "A2 E3 A3 D4 E4 A4 D5 E5 A5 E5 C#5 A4 E4 C#4 A3 E3" },
  "Gm": { bass: "G2", harp: "G2 D3 G3 A3 Bb3 A3 G3 D3", pad: "Bb3 D4 G4", ladder: "G3 D4 G4 A4 Bb4" },
  "Dm/F": { bass: "F2", harp: "F2 D3 A3 D4 F4 D4 A3 D3", pad: "A3 D4 F4", ladder: "F3 D4 F4 G4 A4" },
  "Eb": { bass: "Eb2", harp: "Eb3 Bb3 Eb4 F4 G4 F4 Eb4 Bb3", pad: "Bb3 Eb4 G4" },
  "Eb-A7": { bass: "Eb2 | A1", harp: "Eb3 Bb3 Eb4 G4 A2 E3 G3 C#4", pad: "Bb3 Eb4 G4 | A3 C#4 G4", sweep: "Eb3 Bb3 Eb4 G4 Bb4 Eb5 G5 Bb5 A5 G5 E5 C#5 A4 G4 E4 C#4" },
  // The middle section's chords sit higher and open wider.
  "BbM7^": { bass: "Bb2", harp: "Bb2 F3 A3 D4 F4 D4 A3 F3", pad: "A3 D4 F4" },
  "C/Bb": { bass: "Bb2", harp: "Bb2 E3 G3 C4 E4 C4 G3 E3", pad: "G3 C4 E4" },
  "Am7": { bass: "A2", harp: "A2 E3 G3 C4 E4 C4 G3 E3", pad: "G3 C4 E4" },
  "Dm^": { bass: "D3", harp: "D3 A3 D4 F4 A4 F4 D4 A3", pad: "A3 D4 F4" },
  "C7": { bass: "C3", harp: "C3 G3 Bb3 E4 G4 E4 Bb3 G3", pad: "Bb3 C4 E4" },
  "F": { bass: "F2", harp: "F2 C3 A3 C4 F4 C4 A3 C3", pad: "A3 C4 F4" },
  // The harp leaves the second half of this bar to its glissando (`extras`).
  "A7sus-A7": { bass: "A1", harp: "A2 E3 G3 D4 - - - -", pad: "A3 D4 G4 | A3 C#4 G4" },
  "C": { bass: "C2", harp: "C3 G3 C4 D4 E4 D4 C4 G3", pad: "G3 C4 E4" },
  "Dsus-D": { bass: "D2", harp: "D3 A3 D4 G4 A4 F#4 D4 A3", pad: "A3 D4 G4 | A3 D4 F#4" },
  "D": { bass: "D2", harp: "D3 A3 D4 E4 F#4 E4 D4 A3", pad: "A3 D4 F#4" },
  "D-rise": { bass: "D2", harp: "D3 A3 D4 F#4 A4 D5 F#5 A5", pad: "A3 D4 F#4" },
  "D-end": { bass: "D2", harp: "- - - - - - - -", pad: "A3 D4 F#4" },
};

// The theme, as the flute first has it and as it returns.
const THEME = "A4:1.5 D5:.5 E5 F5 | A5:2.5 G5:.5 F5 | G5 F5 D5:2 | E5:3 A5 | <mf> Bb5:2.5 A5:.5 G5 | A5 F5 D5:2 | <mp> G5:1.5 F5:.5 Eb5:2 | D5:2 <p> C#5:1.5 -:.5";
const THEME_2 = "A4:1.5 D5:.5 E5 F5 | A5:2 C6 A5:.5 G5:.5 | A5:1.5 G5:.5 F5 D5 | E5:3 A5 | Bb5:2.5 A5:.5 G5 | A5 D6:2 A5 | Bb5 G5 F5 E5 | D5:3.5 -:.5";
/** The return: `peak` is the dynamic of its high phrase, `close` of its cadence. */
const theme3 = (peak, close) => `A4:1.5 D5:.5 E5 F5 | A5:2 C6 A5:.5 G5:.5 | A5:1.5 G5:.5 F5 D5 | E5:3 A5 | <${peak}> Bb5:2.5 A5:.5 G5 | A5 D6:2 A5 | <${close}> Bb5 G5 F5 E5 | D5:4_`;
const ANSWER_1 = "F4 G4 A4:2 | G4 A4 C5:2 | C5:1.5 D5:.5 E5:2 | D5:3 -";
const ANSWER_2 = "F4 G4 A4:2 | <mf> G4 A4 Bb4:2 | <f> A4:1.5 C5:.5 F5:2 | E5:4^";

/** Vigil, the game's theme. */
export const VIGIL = {
  id: "vigil",
  title: "Vigil",
  seed: 41,
  /** Seconds of silence between one playing and the next. */
  gap: 8,
  chords: VIGIL_CHORDS,
  // [chord, beats a minute]; a pair of tempos is a bar that slows from the first to the second.
  bars: [
    // The ground alone.
    ["Dm", 63], ["Dm/C", 63], ["BbM7", 63], ["Asus-A", 63],
    // The theme.
    ["Dm", 63], ["Dm/C", 63], ["BbM7", 63], ["Asus-A", 63], ["Gm", 63], ["Dm/F", 63], ["Eb", 63], ["Asus-A", [63, 58]],
    // The theme again, with the strings.
    ["Dm", 63], ["Dm/C", 63], ["BbM7", 63], ["Asus-A", 63], ["Gm", 63], ["Dm/F", 63], ["Eb-A7", [63, 60]], ["Dm", [60, 57]],
    // The answer, in F.
    ["BbM7^", 63], ["C/Bb", 63], ["Am7", 63], ["Dm^", 63], ["BbM7^", 63], ["C7", 63], ["F", 63], ["A7sus-A7", [62, 52]],
    // The theme, everyone.
    ["Dm", 60], ["Dm/C", 60], ["BbM7", 60], ["Asus-A", 60], ["Gm", 60], ["Dm/F", 60], ["Eb-A7", [60, 56]], ["BbM7", [56, 54]],
    // The coda: up into the major.
    ["C", 56], ["Dsus-D", [56, 53]], ["BbM7", 54], ["C", 54], ["D", [54, 50]], ["D-rise", [50, 40]], ["D-end", 40],
  ],
  // Which bars each layer of the accompaniment plays (first and last bar, both included).
  layers: {
    harp: { eighths: [[0, 27], [36, 42]], sixteenths: [[28, 35]], low: [12, 37] },
    pad: [8, 42],
    bass: [8, 42],
    choir: [
      { bars: [16, 19], vowel: "oo" },
      { bars: [24, 37], vowel: "ah" },
      { bars: [38, 42], vowel: "oo" },
    ],
  },
  // How loud each layer is: [bar, velocity] points, a straight line between them.
  dynamics: {
    harp: [[0, 0.5], [4, 0.44], [12, 0.5], [20, 0.5], [26, 0.54], [28, 0.54], [35, 0.5], [36, 0.48], [38, 0.42], [42, 0.36]],
    pad: [[8, 0.16], [11, 0.22], [12, 0.32], [19, 0.34], [20, 0.36], [24, 0.42], [27, 0.58], [28, 0.62], [34, 0.6], [36, 0.44], [38, 0.26], [42, 0.14]],
    bass: [[8, 0.2], [12, 0.36], [20, 0.4], [27, 0.56], [28, 0.66], [35, 0.6], [36, 0.44], [38, 0.3], [42, 0.2]],
    choir: [[16, 0.3], [19, 0.36], [24, 0.28], [27, 0.5], [28, 0.62], [35, 0.56], [36, 0.4], [38, 0.26], [42, 0.14]],
  },
  // The melodic lines. `bar`: where the line starts. `oct`: octaves up or down from what is
  // written. `alt`: who plays it instead on every second playing (so the loop does not wear).
  lines: [
    { voice: "flute", bar: 4, notes: `<mp> ${THEME}`, alt: { voice: "horn", oct: -1 } },
    { voice: "flute", bar: 12, notes: `<mf> ${THEME_2}` },
    { voice: "violins", bar: 12, notes: `<p> ${THEME_2}` },
    { voice: "horn", bar: 20, notes: `<mf> ${ANSWER_1}`, alt: { voice: "flute", oct: 1 } },
    { voice: "horn", bar: 24, notes: `<mf> ${ANSWER_2}` },
    { voice: "violins", bar: 24, oct: 1, notes: `<mp> ${ANSWER_2}` },
    { voice: "violins", bar: 28, notes: `<f> ${theme3("ff", "f")}` },
    { voice: "violins", bar: 28, oct: -1, notes: `<mf> ${theme3("f", "mf")}` },
    { voice: "flute", bar: 28, notes: `<mp> ${theme3("mf", "mp")}` },
    { voice: "horn", bar: 28, notes: "<mp> D4:2 F4:2 | F4:3 E4 | D4:4 | D4:2 C#4:2 | D4:2 G4:2 | F4:4 | G4:4 | F4:4_" },
    { voice: "violins", bar: 36, notes: "<mf> E5:2 G5:2 | <mp> G5:2 <p> F#5:2_" },
    { voice: "violins", bar: 36, oct: -1, notes: "<mp> E5:2 G5:2 | <p> G5:2 F#5:2_" },
    { voice: "flute", bar: 38, notes: "<p> A4:1.5 D5:.5 E5 F5 | G5:3 E5 | <pp> F#5:7_ -" },
  ],
  // Single events: `notes` one after another every `step` beats (0: together), from `vel[0]` to
  // `vel[1]`.
  extras: [
    // The harp's glissando into the return, up the dominant's scale, and where it lands.
    { voice: "harp", bar: 27, beat: 2.5, step: 1.5 / 17, vel: [0.22, 0.6], notes: "A3 Bb3 C#4 D4 E4 F4 G4 A4 Bb4 C#5 D5 E5 F5 G5 A5 Bb5 C#6" },
    { voice: "harp", bar: 28, beat: 0, step: 0, vel: [0.6, 0.6], notes: "D6" },
    // The drum: a roll under the glissando, then the downbeats of the return.
    { voice: "timpani", bar: 27, beat: 2, step: 0.25, vel: [0.1, 0.5], notes: "A2 A2 A2 A2 A2 A2 A2 A2" },
    { voice: "timpani", bar: 28, beat: 0, step: 0, vel: [0.7, 0.7], notes: "D2" },
    { voice: "timpani", bar: 30, beat: 0, step: 0, vel: [0.4, 0.4], notes: "Bb2" },
    { voice: "timpani", bar: 32, beat: 0, step: 0, vel: [0.6, 0.6], notes: "G2" },
    { voice: "timpani", bar: 34, beat: 2, step: 0, vel: [0.5, 0.5], notes: "A2" },
    { voice: "timpani", bar: 35, beat: 0, step: 0, vel: [0.6, 0.6], notes: "Bb2" },
    { voice: "timpani", bar: 37, beat: 0, step: 0, vel: [0.3, 0.3], notes: "D2" },
    // The celesta: the turn to the major, and the last bars.
    { voice: "celesta", bar: 37, beat: 2, step: 0, vel: [0.3, 0.3], notes: "D6 F#6" },
    { voice: "celesta", bar: 40, beat: 0, step: 2, vel: [0.24, 0.24], notes: "A5 D6" },
    { voice: "celesta", bar: 41, beat: 2, step: 0.5, vel: [0.2, 0.3], notes: "A5 D6 F#6 A6" },
    { voice: "celesta", bar: 42, beat: 0, step: 0, vel: [0.3, 0.3], notes: "D6 A6 D7" },
    // The last chord, rolled from the bottom.
    { voice: "harp", bar: 42, beat: 0, step: 0.07, vel: [0.4, 0.5], notes: "D2 A2 D3 F#3 A3 D4 F#4 A4 D5 D6" },
  ],
};

/** Every piece, by id. */
export const PIECES = { vigil: VIGIL };

// ---------------------------------------------------------------------------------------------
// From a piece to timed notes

const BEATS = 4;

/** The bars' start times in seconds and `at(bar, beat)`. A bar with two tempos slows evenly, beat
 *  by beat, from the first to the second. */
export function tempoMap(piece) {
  const within = (bpm, beat) => {
    if (typeof bpm === "number") return (60 * beat) / bpm;
    const [a, b] = bpm;
    if (a === b) return (60 * beat) / a;
    // The tempo falls linearly over the bar's beats: the time is the integral of 60 / tempo.
    return ((60 * BEATS) / (b - a)) * Math.log((a + ((b - a) * beat) / BEATS) / a);
  };
  const starts = [0];
  for (const [, bpm] of piece.bars) starts.push(starts[starts.length - 1] + within(bpm, BEATS));
  const last = piece.bars.length - 1;
  const lastBpm = piece.bars[last][1];
  const endBpm = typeof lastBpm === "number" ? lastBpm : lastBpm[1];
  const at = (bar, beat = 0) => {
    bar += Math.floor(beat / BEATS);
    beat -= Math.floor(beat / BEATS) * BEATS;
    // Past the last bar (a note held over the end): the last tempo goes on.
    if (bar > last) return starts[last + 1] + (60 * ((bar - last - 1) * BEATS + beat)) / endBpm;
    return starts[bar] + within(piece.bars[bar][1], beat);
  };
  return { starts, seconds: starts[last + 1], at };
}

/** The level of a layer at a bar: the line through its `[bar, velocity]` points. */
export function levelAt(points, bar) {
  if (bar <= points[0][0]) return points[0][1];
  for (let i = 1; i < points.length; i++) {
    const [b1, v1] = points[i];
    if (bar <= b1) {
      const [b0, v0] = points[i - 1];
      return v0 + ((v1 - v0) * (bar - b0)) / (b1 - b0);
    }
  }
  return points[points.length - 1][1];
}

/** A line's notes: `{ beat, beats, m, v, v2 }` from its start (`m` null: a rest). `C5:1.5` is a
 *  note and its length in beats (one when left out), `-` a rest, `<mf>` the dynamic from there on,
 *  a trailing `^` a note that swells and `_` one that fades, `|` a bar line (checked: a line whose
 *  bars do not add up is an error, not a late entrance). */
export function parseLine(notes, oct = 0) {
  const out = [];
  let beat = 0;
  let v = MARKS.mf;
  for (const w of words(notes)) {
    if (w === "|") {
      if (Math.abs(beat / BEATS - Math.round(beat / BEATS)) > 1e-9) throw new Error(`music: a bar of ${beat % BEATS} beats before "|" in "${notes}"`);
      continue;
    }
    const mark = /^<(\w+)>$/.exec(w);
    if (mark) {
      if (!(mark[1] in MARKS)) throw new Error(`music: no dynamic "${mark[1]}"`);
      v = MARKS[mark[1]];
      continue;
    }
    const m = /^([^:^_]+)(?::([\d.]+))?([\^_]?)$/.exec(w);
    if (!m) throw new Error(`music: cannot read "${w}"`);
    const beats = m[2] ? Number(m[2]) : 1;
    const pitch = m[1] === "-" ? null : midi(m[1]) + 12 * oct;
    out.push({ beat, beats, m: pitch, v, v2: m[3] === "^" ? v * 1.35 : m[3] === "_" ? v * 0.55 : v });
    beat += beats;
  }
  return out;
}

/** The sixteen notes of a harp sweep: written out, or up the ladder and its octave and back. */
function sweepOf(chord) {
  if (chord.sweep) return pitches(chord.sweep);
  const l = pitches(chord.ladder);
  const up = [...l, ...l.slice(1).map((m) => m + 12)];
  return [...up, ...up.slice(2, 8).reverse(), l[1]];
}

const inSpan = (span, bar) => bar >= span[0] && bar <= span[1];

/** The timed notes of one playing of a piece: `{ events, seconds }`, each event
 *  `{ voice, t, d, m, v, v2 }` (seconds from the start, MIDI pitch, velocity at the note's start
 *  and its end), in order of time. `pass` counts the playings: it seeds the small unevenness of
 *  the harp and, on every second playing, hands some lines to another instrument. */
export function compile(piece, pass = 0) {
  const rng = mulberry32((piece.seed ?? 1) * 7919 + pass);
  const time = tempoMap(piece);
  const events = [];
  const note = (voice, bar, beat, beats, m, v, v2 = v, nudge = 0) => {
    const t = time.at(bar, beat);
    events.push({ voice, t: Math.max(0, t + nudge), d: time.at(bar, beat + beats) - t, m, v, v2 });
  };
  const chordAt = (bar) => {
    const c = piece.chords[piece.bars[bar][0]];
    if (!c) throw new Error(`music: no chord "${piece.bars[bar][0]}"`);
    return c;
  };
  const { layers, dynamics } = piece;

  // The harp. The first note of a bar leans, the notes off the beat step back, and no two are
  // quite alike.
  const harp = layers.harp;
  for (let bar = 0; bar < piece.bars.length; bar++) {
    const chord = chordAt(bar);
    const level = levelAt(dynamics.harp, bar);
    const fine = harp.sixteenths.some((s) => inSpan(s, bar));
    if (!fine && !harp.eighths.some((s) => inSpan(s, bar))) continue;
    const fig = fine ? sweepOf(chord) : pitches(chord.harp);
    const step = BEATS / fig.length;
    fig.forEach((m, i) => {
      if (m === null) return;
      const onBeat = (i * step) % 1 === 0;
      const lean = i === 0 ? 1.2 : onBeat ? 1 : 0.84;
      note("harp", bar, i * step, 2, m, level * lean * (0.94 + rng() * 0.12), undefined, i === 0 ? 0 : (rng() - 0.5) * 0.012);
    });
    if (harp.low && inSpan(harp.low, bar)) note("harp", bar, 0, 4, halves(chord.bass)[0][0], level * 0.9);
  }

  // The held layers: the strings' pad, the bass, the choir. A voice that keeps its note from one
  // chord to the next holds it (one long note), so the pad breathes with the harmony, not the bar.
  const held = (voice, span, voicingOf, points) => {
    let open = [];
    const close = () => {
      for (const o of open) note(voice, o.bar, o.beat, o.beats, o.m, o.v, o.v2);
      open = [];
    };
    for (let bar = span[0]; bar <= span[1]; bar++) {
      const parts = voicingOf(chordAt(bar));
      parts.forEach((voicing, k) => {
        const beats = BEATS / parts.length;
        const v = levelAt(points, bar + k / parts.length);
        if (voicing.length !== open.length) close();
        voicing.forEach((m, i) => {
          const o = open[i];
          if (o && o.m === m) {
            o.beats += beats;
            o.v2 = v;
            return;
          }
          if (o) note(voice, o.bar, o.beat, o.beats, o.m, o.v, o.v2);
          open[i] = { bar, beat: k * beats, beats, m, v, v2: v };
        });
      });
    }
    close();
  };
  if (layers.pad) held("pad", layers.pad, (c) => halves(c.pad), dynamics.pad);
  if (layers.bass) held("bass", layers.bass, (c) => halves(c.bass), dynamics.bass);
  for (const part of layers.choir ?? []) {
    // The choir sings the pad's voicing over the bass note, brought up into the men's range.
    const voicing = (c) => {
      const pads = halves(c.pad);
      const basses = halves(c.bass);
      return pads.map((p, k) => {
        let low = basses[Math.min(k, basses.length - 1)][0];
        while (low < 45) low += 12;
        return [low, ...p];
      });
    };
    held(part.vowel === "oo" ? "choirOo" : "choirAh", part.bars, voicing, dynamics.choir);
  }

  // The melodic lines.
  for (const line of piece.lines) {
    const alt = pass % 2 === 1 ? line.alt : null;
    const voice = alt?.voice ?? line.voice;
    for (const n of parseLine(line.notes, (line.oct ?? 0) + (alt?.oct ?? 0))) {
      if (n.m !== null) note(voice, line.bar, n.beat, n.beats, n.m, n.v, n.v2);
    }
  }

  // The single events.
  for (const x of piece.extras ?? []) {
    const ms = pitches(x.notes);
    ms.forEach((m, i) => {
      const v = x.vel[0] + ((x.vel[1] - x.vel[0]) * i) / Math.max(1, ms.length - 1);
      note(x.voice, x.bar, x.beat + i * x.step, x.beats ?? 2, m, v);
    });
  }

  events.sort((a, b) => a.t - b.t);
  return { events, seconds: time.seconds };
}
