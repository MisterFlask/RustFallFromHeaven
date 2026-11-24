// More pieces for the game's music (docs/MUSIC.md), in other styles than Vigil's. Data only, in
// the format music_score.js compiles: bars (a chord and a tempo each), the chords' notes, the
// repeating figures, the held layers, the melodic lines, how loud each is from bar to bar.

// ---------------------------------------------------------------------------------------------
// Onslaught
//
// A battle: E minor, four in a bar, 152 to the minute. The low strings gallop on the chord's root
// (an eighth and two sixteenths a beat), the brass has the theme, the high strings join for its
// second time, a broader middle in G rides over the same gallop, four bars climb to the dominant
// and the theme comes back with everyone. The last bar turns back into the first: no silence.

/** `low`: the gallop's notes (root, fifth, octave). `mid`: the violas' eighths. `stab`: the brass
 *  chord. Two chords in a bar are separated by `|`. */
const ONSLAUGHT_CHORDS = {
  "Em": { bass: "E2", pad: "G3 B3 E4", low: "E2 B2 E3", mid: "E3 G3 B3", stab: "E3 B3 E4 G4" },
  "C": { bass: "C2", pad: "G3 C4 E4", low: "C2 G2 C3", mid: "E3 G3 C4", stab: "C3 G3 C4 E4" },
  "D": { bass: "D2", pad: "A3 D4 F#4", low: "D2 A2 D3", mid: "F#3 A3 D4", stab: "D3 A3 D4 F#4" },
  "B": { bass: "B1", pad: "F#3 B3 D#4", low: "B1 F#2 B2", mid: "D#3 F#3 B3", stab: "B2 F#3 B3 D#4" },
  "Am": { bass: "A1", pad: "A3 C4 E4", low: "A1 E2 A2", mid: "E3 A3 C4", stab: "A2 E3 A3 C4" },
  "B7": { bass: "B1", pad: "F#3 A3 D#4", low: "B1 F#2 B2", mid: "D#3 F#3 A3", stab: "B2 F#3 A3 D#4" },
  "Em-B": { bass: "E2 | B1", pad: "G3 B3 E4 | F#3 B3 D#4", low: "E2 B2 E3 | B1 F#2 B2", mid: "E3 G3 B3 | D#3 F#3 B3", stab: "E3 B3 E4 G4 | B2 F#3 B3 D#4" },
  "G": { bass: "G2", pad: "G3 B3 D4", low: "G2 D3 G3", mid: "D3 G3 B3" },
  "D/F#": { bass: "F#2", pad: "F#3 A3 D4", low: "F#2 D3 F#3", mid: "D3 F#3 A3" },
  "B/D#": { bass: "D#2", pad: "F#3 B3 D#4", low: "D#2 B2 D#3", mid: "D#3 F#3 B3" },
  "G/B": { bass: "B1", pad: "D3 G3 B3", low: "B1 G2 B2", mid: "D3 G3 B3" },
  "D#dim": { bass: "D#2", pad: "F#3 A3 C4", low: "D#2 A2 D#3", mid: "F#3 A3 C4" },
};

const CHARGE = "E5:1.5 G5:.5 F#5:.5 E5:.5 B4 | B4:.5 D5:.5 E5:1.5 -:.5 E5:.5 F#5:.5 | G5:1.5 F#5:.5 E5:.5 G5:.5 C6 | B5:.5 A5:.5 F#5:1.5 -:.5 D5:.5 F#5:.5 | G5:1.5 B5:.5 A5:.5 G5:.5 E5 | E5:.5 F#5:.5 G5:1.5 -:.5 G5:.5 A5:.5 | C6:1.5 B5:.5 A5:.5 G5:.5 E5 | D#5 F#5 B5:2";
const CHARGE_2 = "E5:1.5 G5:.5 F#5:.5 E5:.5 B4 | B4:.5 D5:.5 E5:1.5 -:.5 E5:.5 F#5:.5 | G5:1.5 F#5:.5 E5:.5 G5:.5 C6 | B5:.5 A5:.5 F#5 A5 D6 | E6:1.5 D6:.5 C6:.5 B5:.5 A5 | B5:.5 A5:.5 F#5:.5 D#5:.5 F#5 B5 | E6:2.5 B5:.5 G5:.5 B5:.5 | E5:2 - B4:.5 D#5:.5";
const BROAD = "D5:2 G5 B5 | A5:3 F#5 | G5:2 E5 G5 | F#5:3 D#5:.5 F#5:.5 | E5:2 G5 C6 | B5:3 G5 | A5:2 C6 E6 | F#5 B5 D#6:2^";
const CLIMB = "E6:2 C6 E6 | F#6:2 D6 F#6 | C6:.5 A5:.5 C6:.5 D#6:.5 F#6:2^ | F#6:.5 D#6:.5 B5:.5 A5:.5 F#5:.5 D#5:.5 B4";
const GALLOP = "0 - 0 0 | 1 - 0 0 | 2 - 0 0 | 1 - 0 0";

export const ONSLAUGHT = {
  id: "onslaught",
  title: "Onslaught",
  style: "Battle",
  seed: 73,
  gap: 0,
  // A battle is close: less of the hall than a vigil.
  hall: 0.9,
  chords: ONSLAUGHT_CHORDS,
  sections: [[0, "The gallop"], [4, "The theme"], [12, "The theme, with the high strings"], [20, "The broad middle"], [28, "The climb"], [32, "The theme, everyone"]],
  bars: [
    // The gallop alone, then the horns.
    ["Em", 152], ["Em", 152], ["Em", 152], ["B", 152],
    // The theme, the brass.
    ["Em", 152], ["Em", 152], ["C", 152], ["D", 152], ["Em", 152], ["Em", 152], ["C", 152], ["B", 152],
    // The theme again, with the high strings.
    ["Em", 152], ["Em", 152], ["C", 152], ["D", 152], ["Am", 152], ["B7", 152], ["Em", 152], ["Em-B", 152],
    // The broad middle, in G.
    ["G", 152], ["D/F#", 152], ["Em", 152], ["B/D#", 152], ["C", 152], ["G/B", 152], ["Am", 152], ["B7", 152],
    // The climb to the dominant.
    ["C", 152], ["D", 152], ["D#dim", 152], ["B7", 152],
    // The theme, everyone.
    ["Em", 152], ["Em", 152], ["C", 152], ["D", 152], ["Am", 152], ["B7", 152], ["Em", 152], ["Em-B", 152],
  ],
  layers: {
    pad: [20, 39],
    bass: [4, 39],
  },
  patterns: [
    { voice: "spiccato", level: "low", of: "low", bars: [[0, 39]], step: 0.25, len: 0.22, figure: GALLOP, human: 0.006 },
    { voice: "spiccato", level: "mid", of: "mid", bars: [[12, 39]], step: 0.5, len: 0.4, figure: "0 1 2 1 0 1 2 1", human: 0.006 },
    // The brass chords: three, three and two eighths.
    { voice: "brass", level: "stab", of: "stab", bars: [[2, 19], [32, 39]], step: 0.5, len: 0.4, figure: "*! - - * - - * -" },
    { voice: "timpani", of: "bass", oct: 1, bars: [[0, 3]], step: 0.5, len: 2, figure: "0 - - - - - - -" },
    { voice: "timpani", of: "bass", oct: 1, bars: [[4, 30], [32, 39]], step: 0.5, len: 2, figure: "0! - - - - 0 - -" },
    { voice: "snare", bars: [[4, 30], [32, 39]], step: 0.25, len: 0.2, figure: "- - - - | D4 - - D4? | - - D4? - | D4 - D4? D4?", human: 0.004 },
  ],
  dynamics: {
    low: [[0, 0.5], [2, 0.56], [4, 0.6], [20, 0.52], [28, 0.56], [31, 0.7], [32, 0.68], [39, 0.64]],
    mid: [[12, 0.36], [20, 0.34], [28, 0.4], [32, 0.44]],
    stab: [[2, 0.36], [4, 0.44], [19, 0.44], [32, 0.52]],
    timpani: [[0, 0.4], [4, 0.5], [20, 0.4], [28, 0.5], [30, 0.6], [32, 0.62]],
    snare: [[4, 0.42], [20, 0.34], [28, 0.44], [32, 0.52]],
    bass: [[4, 0.5], [20, 0.44], [28, 0.5], [32, 0.6]],
    pad: [[20, 0.3], [27, 0.36], [28, 0.4], [31, 0.5], [32, 0.42]],
  },
  lines: [
    { voice: "horn", bar: 2, notes: "<mp> E4:4^ | <mf> F#4:2 B4:2^" },
    { voice: "brass", bar: 4, oct: -1, notes: `<f> ${CHARGE}` },
    { voice: "brass", bar: 12, oct: -1, notes: `<f> ${CHARGE_2}` },
    { voice: "marcato", bar: 12, notes: `<f> ${CHARGE_2}` },
    // The middle: marked strokes on top, so its quarter notes speak in time, the slow bow an octave under.
    { voice: "marcato", bar: 20, notes: `<mf> ${BROAD}` },
    { voice: "violins", bar: 20, oct: -1, notes: `<mp> ${BROAD}` },
    { voice: "horn", bar: 20, notes: "<mp> B3:4 | A3:4 | G3:4 | F#3:4 | G3:4 | G3:2 B3:2 | C4:4 | B3:2 A3:2" },
    { voice: "marcato", bar: 28, notes: `<f> ${CLIMB}` },
    { voice: "brass", bar: 28, oct: -1, notes: `<f> ${CLIMB}` },
    { voice: "brass", bar: 32, oct: -1, notes: `<ff> ${CHARGE_2}` },
    { voice: "marcato", bar: 32, notes: `<ff> ${CHARGE_2}` },
    { voice: "horn", bar: 32, notes: "<mf> E4:4 | E4:4 | E4:4 | F#4:4 | E4:4 | F#4:2 A4:2 | G4:4 | G4:2 F#4:2" },
  ],
  extras: [
    { voice: "cymbal", bar: 4, beat: 0, step: 0, vel: [0.5, 0.5], notes: "C5" },
    { voice: "cymbal", bar: 20, beat: 0, step: 0, vel: [0.4, 0.4], notes: "C5" },
    // The rolls under the last bar of the climb, and where they land.
    { voice: "timpani", bar: 31, beat: 0, step: 0.25, beats: 1, vel: [0.2, 0.7], notes: "B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2 B2" },
    { voice: "snare", bar: 31, beat: 0, step: 0.25, beats: 0.2, vel: [0.15, 0.6], notes: "D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4 D4" },
    { voice: "cymbal", bar: 32, beat: 0, step: 0, vel: [0.6, 0.6], notes: "C5" },
  ],
};

// ---------------------------------------------------------------------------------------------
// Harvest Round
//
// A dance: a jig in D Dorian (D minor with a B natural: the G major chord is its colour), six
// eighths in a bar, the eighth the beat, 348 of them to the minute. A whistle and a fiddle take
// turns and then play together over a lute, a hand drum and a drone: the first strain twice, the
// second twice, a third over little but the drum, and the first again with everyone.

/** `lb`: the lute's two bass notes (the bar's first beat, then its fourth). `lc`: the chord it
 *  strums. In the bar of two chords the second half lists its fifth first, so the strum's bass
 *  (the second of the two) is still the root. */
const HARVEST_CHORDS = {
  "Dm": { bass: "D2", lb: "D3 A2", lc: "D4 F4 A4" },
  "C": { bass: "C2", lb: "C3 G2", lc: "C4 E4 G4" },
  "G": { bass: "G2", lb: "G2 D3", lc: "B3 D4 G4" },
  "Am": { bass: "A2", lb: "A2 E3", lc: "C4 E4 A4" },
  "F": { bass: "F2", lb: "F2 C3", lc: "C4 F4 A4" },
  "Am-C": { bass: "A2 | C3", lb: "A2 E3 | G2 C3", lc: "C4 E4 A4 | C4 E4 G4" },
};

const ROUND_A = "D5:2 E5 F5:2 A5 | A5 F5 E5 F5:2 D5 | E5:2 G5 C6:2 G5 | G5 E5 C5 E5:2 G5 | A5:2 D6 A5:2 F5 | A5 G5 F5 D5:2 F5 | E5:2 A5 G5:2 E5 | D5:2 A4 D5:3";
const ROUND_B = "A5:2 C6 A5:2 F5 | G5:2 C6 G5:2 E5 | B5 A5 G5 D5:2 G5 | F5 E5 D5 A5:3 | C6:2 A5 F5:2 A5 | G5 A5 G5 E5:2 C5 | E5 A5 C6 A5:2 G5 | F5 E5 D5 D5:3";
const ROUND_C = "D5 F5 A5 D5 F5 A5 | D6:2 A5 F5:2 A5 | G5 B5 D6 G5 B5 D6 | D6:2 B5 G5:2 B5 | A5 D6 A5 F5 A5 F5 | D5 F5 A5 D6:2 A5 | G5 E5 C5 E5 G5 C6 | A5:2 F5 D5:3";
const STRAIN_A = ["Dm", "Dm", "C", "C", "Dm", "Dm", "Am-C", "Dm"];
const STRAIN_B = ["F", "C", "G", "Dm", "F", "C", "Am", "Dm"];
const STRAIN_C = ["Dm", "Dm", "G", "G", "Dm", "Dm", "C", "Dm"];
const jig = (chords) => chords.map((c) => [c, 348]);

export const HARVEST = {
  id: "harvest",
  title: "Harvest Round",
  style: "Folk dance",
  seed: 29,
  beats: 6,
  gap: 3,
  hall: 1.0,
  chords: HARVEST_CHORDS,
  sections: [[0, "The drum and the lute"], [4, "First strain, the whistle"], [12, "First strain, both"], [20, "Second strain, the fiddle"], [28, "Second strain, both"], [36, "Third strain, the whistle"], [44, "Third strain, both"], [52, "First strain, everyone"]],
  bars: [
    ...jig(["Dm", "Dm", "C", "Dm"]),
    ...jig(STRAIN_A), ...jig(STRAIN_A),
    ...jig(STRAIN_B), ...jig(STRAIN_B),
    ...jig(STRAIN_C), ...jig(STRAIN_C),
    ...jig(STRAIN_A.slice(0, 7)), ["Dm", [348, 300]], ["Dm", 300],
  ],
  layers: {
    bass: [0, 60],
  },
  patterns: [
    { voice: "lute", level: "lute", of: "lb", bars: [[0, 59]], step: 1, len: 2, figure: "0 - - 1 - -", human: 0.008 },
    { voice: "lute", level: "strum", of: "lc", bars: [[2, 35], [44, 59]], step: 1, len: 1, figure: "- - * - - *", roll: 0.012, human: 0.008 },
    { voice: "drum", bars: [[0, 59]], step: 1, len: 1, figure: "D2! - A2? D2 - A2", human: 0.006 },
    { voice: "shaker", bars: [[28, 35], [44, 59]], step: 1, len: 0.5, figure: "C5! C5? C5? C5! C5? C5?", human: 0.006 },
  ],
  dynamics: {
    lute: [[0, 0.5], [4, 0.46], [36, 0.4], [44, 0.5], [52, 0.56], [60, 0.5]],
    strum: [[2, 0.3], [4, 0.34], [28, 0.38], [44, 0.38], [52, 0.44]],
    drum: [[0, 0.5], [4, 0.46], [36, 0.36], [44, 0.5], [52, 0.58]],
    shaker: [[28, 0.4], [44, 0.4], [52, 0.5]],
    bass: [[0, 0.3], [36, 0.24], [44, 0.3], [52, 0.36], [60, 0.3]],
  },
  lines: [
    { voice: "whistle", bar: 4, notes: `<mf> ${ROUND_A}`, alt: { voice: "fiddle" } },
    { voice: "whistle", bar: 12, notes: `<mf> ${ROUND_A}` },
    { voice: "fiddle", bar: 12, notes: `<mp> ${ROUND_A}` },
    { voice: "fiddle", bar: 20, notes: `<mf> ${ROUND_B}`, alt: { voice: "whistle" } },
    { voice: "fiddle", bar: 28, notes: `<mf> ${ROUND_B}` },
    { voice: "whistle", bar: 28, notes: `<mf> ${ROUND_B}` },
    { voice: "whistle", bar: 36, notes: `<mp> ${ROUND_C}` },
    { voice: "whistle", bar: 44, notes: `<mf> ${ROUND_C}` },
    { voice: "fiddle", bar: 44, notes: `<mf> ${ROUND_C}` },
    { voice: "whistle", bar: 52, notes: `<f> ${ROUND_A} | D5:6_` },
    { voice: "fiddle", bar: 52, notes: `<f> ${ROUND_A} | D4:6_` },
  ],
  extras: [
    // The last chord, strummed from the bottom, and the drum under it.
    { voice: "lute", bar: 60, beat: 0, step: 0.1, beats: 6, vel: [0.5, 0.6], notes: "D3 A3 D4 F4 A4 D5" },
    { voice: "drum", bar: 60, beat: 0, step: 0, beats: 1, vel: [0.6, 0.6], notes: "D2" },
  ],
};

// ---------------------------------------------------------------------------------------------
// Umbra
//
// Something old and watching: A minor that keeps slipping to minor chords a third away (F minor,
// C sharp minor), four in a bar, 60 to the minute. A bell and a drone; a celesta turning a figure
// of three notes against the bar's eight eighths, so it never lands the same way twice running; a
// flute with a long note and its lower neighbour; the high strings over a choir; the flute again,
// ending on the ninth and its resolution.

/** `cel`: the celesta's three notes. `hb`: the harp's rolled chord. `toll`: the bell's note. */
const UMBRA_CHORDS = {
  "Am": { bass: "A1", pad: "A3 C4 E4", cel: "E5 A5 C6", hb: "A2 E3 A3", toll: "A3" },
  "Fm": { bass: "Ab1", pad: "Ab3 C4 F4", cel: "F5 Ab5 C6", hb: "Ab2 F3 C4", toll: "C4" },
  "C#m": { bass: "G#1", pad: "G#3 C#4 E4", cel: "E5 G#5 C#6", hb: "G#2 E3 C#4" },
  "Db": { bass: "Ab1", pad: "Ab3 Db4 F4", cel: "F5 Ab5 Db6", hb: "Ab2 F3 Db4" },
  "Dm": { bass: "A1", pad: "A3 D4 F4", cel: "F5 A5 D6", hb: "A2 F3 D4", toll: "A3" },
  "Dm6": { bass: "A1", pad: "B3 D4 F4", cel: "F5 B5 D6", hb: "A2 F3 B3" },
  "Esus": { bass: "E2", pad: "B3 E4 A4", cel: "E5 A5 B5", hb: "E2 B2 A3" },
  "E": { bass: "E2", pad: "B3 E4 G#4", cel: "E5 G#5 B5", hb: "E2 B2 G#3" },
  "Am9": { bass: "A1", pad: "A3 C4 E4", cel: "E5 A5 B5", hb: "A2 E3 A3" },
  "Am-end": { bass: "A1", pad: "A3 C4 E4", cel: "E5 A5 E6", hb: "A2 E3 A3", toll: "A3" },
};

export const UMBRA = {
  id: "umbra",
  title: "Umbra",
  style: "Arcane",
  seed: 57,
  gap: 6,
  // A large, dark room, and the celesta nearer than a vigil has it. A sparse piece: its level
  // before the ceiling is raised to sit with the others.
  hall: 1.6,
  trim: 0.9,
  mix: { celesta: { level: 0.5 } },
  chords: UMBRA_CHORDS,
  sections: [[0, "The bell"], [2, "The flute"], [10, "The high strings"], [18, "The flute again"]],
  bars: [
    ["Am", 60], ["Am", 60],
    ["Am", 60], ["Am", 60], ["Fm", 60], ["Fm", 60], ["Am", 60], ["Am", 60], ["C#m", 60], ["C#m", 60],
    ["Fm", 60], ["Db", 60], ["Fm", 60], ["Db", 60], ["Dm", 60], ["Dm6", 60], ["Esus", 60], ["E", 60],
    ["Am", 60], ["Am", 60], ["Fm", 60], ["Fm", 60], ["Am", 60], ["Am", 60], ["Am9", [60, 52]], ["Am-end", 52],
  ],
  layers: {
    pad: [0, 25],
    bass: [0, 25],
    choir: [{ bars: [10, 25], vowel: "oo" }],
  },
  patterns: [
    // Three notes against eight eighths: the figure comes round every three bars. The last eighth
    // of a bar is left empty, so the notes have died before the chord slips.
    { voice: "celesta", level: "cel", of: "cel", bars: [[2, 24]], step: 0.5, len: 1, figure: ["0 1 2 0 1 2 0 -", "2 0 1 2 0 1 2 -", "1 2 0 1 2 0 1 -"], human: 0.012 },
    { voice: "harp", level: "harp", of: "hb", bars: [[2, 25]], step: 4, len: 4, figure: "*", every: 2, roll: 0.07 },
    { voice: "bell", level: "bell", of: "toll", bars: [[0, 0], [2, 2], [6, 6], [10, 10], [14, 14], [18, 18], [22, 22], [25, 25]], step: 4, len: 4, figure: "0" },
  ],
  dynamics: {
    cel: [[2, 0.2], [10, 0.26], [17, 0.3], [18, 0.26], [24, 0.16]],
    harp: [[2, 0.34], [18, 0.36], [25, 0.3]],
    bell: [[0, 0.4], [25, 0.3]],
    pad: [[0, 0.14], [2, 0.2], [10, 0.26], [16, 0.34], [18, 0.26], [24, 0.18], [25, 0.1]],
    bass: [[0, 0.3], [10, 0.36], [16, 0.42], [18, 0.34], [25, 0.2]],
    choir: [[10, 0.13], [14, 0.2], [17, 0.22], [18, 0.2], [23, 0.17], [25, 0.08]],
  },
  lines: [
    { voice: "flute", bar: 2, notes: "<p> E5:3.5 D#5:.5 | E5:2 A5:2 | Ab5:3.5 G5:.5 | F5:4_ | E5:3.5 D#5:.5 | E5:2 C6:2 | <mp> C#6:3 B5 | G#5:4_" },
    { voice: "violins", bar: 10, notes: "<p> C6:3 Bb5 | Ab5:2 F5:2 | C6:3.5 Db6:.5 | <mp> F6:2 Db6:2 | F6:3.5 E6:.5 | D6:2 B5:2 | <p> A5:4 | G#5:4_" },
    { voice: "flute", bar: 18, notes: "<mp> E5:3.5 D#5:.5 | E5:2 A5:2 | Ab5:3.5 G5:.5 | F5:4 | E5:3.5 D#5:.5 | E5:2 C6:2 | <p> B5:4 | A5:4_" },
  ],
  extras: [
    { voice: "celesta", bar: 25, beat: 2, step: 1, vel: [0.2, 0.16], notes: "E6 A6" },
  ],
};

/** The pieces besides Vigil, in the order Options lists them. */
export const MORE_PIECES = [ONSLAUGHT, HARVEST, UMBRA];
