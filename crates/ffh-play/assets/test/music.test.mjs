// node --test: the game's music (docs/MUSIC.md). The score (app/music_score.js: every piece's
// chords, lines, figures, ranges and tempo map, and the format itself on small pieces built here),
// the conductor's scheduling over a simulated clock (app/music.js `Conductor`), the orchestra's
// settings and the harp's plucked string (app/music_synth.js `mixFor`, `settingsFor`,
// `pluckSamples`), and the player without Web Audio. All pure: no sound is made.
import test from "node:test";
import assert from "node:assert/strict";

// The player reads its remembered choice from localStorage in `init()`: a fake one, in place
// before the module is loaded.
const stored = new Map();
globalThis.localStorage = {
  getItem: (k) => (stored.has(k) ? stored.get(k) : null),
  setItem: (k, v) => stored.set(k, String(v)),
  removeItem: (k) => stored.delete(k),
};

const { VIGIL, PIECES, compile, tempoMap, parseLine, midi, MARKS } = await import("../app/music_score.js");
const { pluckSamples, MIX, RANGES, mixFor, settingsFor } = await import("../app/music_synth.js");
const { Conductor, music } = await import("../app/music.js");

const pitchesOf = (s) => s.split(/[\s|]+/).filter((w) => w && w !== "-").map(midi);
const near = (a, b, what, eps = 1e-9) => assert.ok(Math.abs(a - b) < eps, `${what}: ${a}, not ${b}`);
const inSpan = (span, bar) => bar >= span[0] && bar <= span[1];

const ALL = Object.values(PIECES);

// ---------------------------------------------------------------------------------------------
// Every piece

test("every piece has an id, a title and a chord for every bar", () => {
  assert.ok(ALL.length >= 1);
  assert.equal(PIECES.vigil, VIGIL);
  for (const [id, piece] of Object.entries(PIECES)) {
    assert.equal(piece.id, id, `the piece under "${id}" says it is "${piece.id}"`);
    assert.ok(typeof piece.title === "string" && piece.title, `${id}: no title`);
    assert.ok(piece.bars.length > 0, `${id}: no bars`);
    piece.bars.forEach(([name], i) => assert.ok(piece.chords[name], `${id}: bar ${i}: no chord "${name}"`));
  }
});

test("a harp layer: every chord it plays has a bar of eighths (or a sweep where it plays sixteenths)", () => {
  for (const piece of ALL) {
    const harp = piece.layers.harp;
    if (!harp) continue;
    piece.bars.forEach(([name], bar) => {
      const c = piece.chords[name];
      if (harp.sixteenths.some((s) => inSpan(s, bar))) {
        if (c.sweep) assert.equal(c.sweep.trim().split(/\s+/).length, 16, `${piece.id}: chord ${name}: the sweep has 16 notes`);
        else assert.equal(c.ladder?.trim().split(/\s+/).length, 5, `${piece.id}: chord ${name} (bar ${bar}): a ladder of 5 notes or a sweep`);
      } else if (harp.eighths.some((s) => inSpan(s, bar))) {
        assert.equal(c.harp?.trim().split(/\s+/).length, 8, `${piece.id}: chord ${name} (bar ${bar}): the harp has 8 eighths`);
      }
    });
  }
});

test("compile: ordered, finite, inside the piece, the same each time", () => {
  for (const piece of ALL) {
    for (const pass of [0, 1]) {
      const { events, seconds } = compile(piece, pass);
      assert.ok(events.length > 0, `${piece.id}, pass ${pass}: no events`);
      assert.ok(Number.isFinite(seconds) && seconds > 0, `${piece.id}: seconds ${seconds}`);
      for (let i = 0; i < events.length; i++) {
        const e = events[i];
        const where = `${piece.id}, pass ${pass}, event ${i} (${e.voice} at ${e.t})`;
        if (i > 0) assert.ok(events[i - 1].t <= e.t, `${where}: out of order`);
        for (const k of ["t", "d", "m", "v", "v2"]) assert.ok(Number.isFinite(e[k]), `${where}: ${k} is ${e[k]}`);
        assert.ok(e.d > 0, `${where}: d ${e.d}`);
        assert.ok(e.v > 0 && e.v <= 1.1, `${where}: v ${e.v}`);
        assert.ok(e.t >= 0 && e.t <= seconds, `${where}: t past the end ${seconds}`);
      }
      assert.deepEqual(compile(piece, pass), { events, seconds }, `${piece.id}, pass ${pass}: not the same twice`);
    }
  }
});

test("the second playing differs only where the piece asks for it (alt, human, the harp)", () => {
  for (const piece of ALL) {
    const varies = piece.lines.some((l) => l.alt) || (piece.patterns ?? []).some((p) => p.human) || !!piece.layers.harp;
    const a0 = compile(piece, 0).events;
    const a1 = compile(piece, 1).events;
    if (varies) assert.notDeepEqual(a0, a1, `${piece.id}: both playings the same`);
    else assert.deepEqual(a0, a1, `${piece.id}: nothing in it varies, yet the playings differ`);
  }
});

test("every line parses in the piece's bar and ends inside the piece", () => {
  for (const piece of ALL) {
    const time = tempoMap(piece);
    for (const line of piece.lines) {
      const notes = parseLine(line.notes, line.oct ?? 0, piece.beats ?? 4);
      assert.ok(notes.length > 0, `${piece.id}: an empty ${line.voice} line at bar ${line.bar}`);
      const last = notes[notes.length - 1];
      const end = time.at(line.bar, last.beat + last.beats);
      assert.ok(end <= time.seconds + 1e-9, `${piece.id}: the ${line.voice} line at bar ${line.bar} ends at ${end}, after ${time.seconds}`);
    }
  }
});

test("every voice is in the mix; every pitch in its voice's range, on both playings", () => {
  for (const piece of ALL) {
    for (const pass of [0, 1]) {
      for (const e of compile(piece, pass).events) {
        assert.ok(e.voice in MIX, `${piece.id}: voice "${e.voice}" is not in MIX`);
        const r = RANGES[e.voice];
        // A voice without a pitch (a snare, a cymbal) is not in RANGES.
        if (!r) continue;
        assert.ok(e.m >= r[0] && e.m <= r[1], `${piece.id}, pass ${pass}: ${e.voice} plays ${e.m} at ${e.t.toFixed(2)} s, outside ${r[0]}..${r[1]}`);
      }
    }
  }
  for (const [voice, r] of Object.entries(RANGES)) {
    assert.ok(voice in MIX, `RANGES has "${voice}", MIX does not`);
    assert.ok(r[0] < r[1], `${voice}: ${r}`);
  }
});

test("a piece with a bass ends on the root it starts on (it loops)", () => {
  for (const piece of ALL) {
    // The first half of each chord's bass (a bar of two chords ends on the second, which leads
    // back to the first bar: the loop's root is the bar's own). A piece whose chords have no
    // `bass` (its low notes are a pattern's) is not checked here.
    const first = piece.chords[piece.bars[0][0]];
    const last = piece.chords[piece.bars[piece.bars.length - 1][0]];
    if (first.bass === undefined || last.bass === undefined) continue;
    const rootOf = (c) => pitchesOf(c.bass.split("|")[0])[0];
    assert.equal(rootOf(first) % 12, rootOf(last) % 12, piece.id);
  }
});

test("sections and mix, where a piece has them", () => {
  for (const piece of ALL) {
    if (piece.sections !== undefined) {
      const s = piece.sections;
      assert.ok(Array.isArray(s) && s.length > 0, `${piece.id}: sections`);
      assert.equal(s[0][0], 0, `${piece.id}: the first section starts at bar ${s[0][0]}`);
      s.forEach(([bar, name], i) => {
        assert.ok(Number.isInteger(bar) && bar >= 0 && bar < piece.bars.length, `${piece.id}: section "${name}" at bar ${bar}, outside the piece`);
        assert.ok(typeof name === "string" && name, `${piece.id}: section ${i} has no name`);
        if (i > 0) assert.ok(bar > s[i - 1][0], `${piece.id}: section "${name}" at bar ${bar} is not after the one before`);
      });
    }
    for (const voice of Object.keys(piece.mix ?? {})) assert.ok(voice in MIX, `${piece.id}: mix has "${voice}", MIX does not`);
  }
});

test("Vigil: about 2:54", () => {
  const { events, seconds } = compile(VIGIL, 0);
  assert.ok(events.length > 500 && seconds > 170 && seconds < 178, `Vigil: ${events.length} events, ${seconds} s`);
});

// ---------------------------------------------------------------------------------------------
// The tempo map

test("tempoMap: bars in order, slowing bars between their tempos, bar ends meet", () => {
  for (const piece of ALL) {
    const beats = piece.beats ?? 4;
    const time = tempoMap(piece);
    assert.equal(time.starts.length, piece.bars.length + 1);
    for (let i = 1; i < time.starts.length; i++) assert.ok(time.starts[i] > time.starts[i - 1], `${piece.id}: bar ${i - 1} has no length`);
    piece.bars.forEach(([, bpm], bar) => {
      const len = time.starts[bar + 1] - time.starts[bar];
      if (typeof bpm === "number") {
        near(len, (60 * beats) / bpm, `${piece.id}: bar ${bar}`);
      } else {
        const [a, b] = bpm;
        const lo = Math.min((60 * beats) / a, (60 * beats) / b);
        const hi = Math.max((60 * beats) / a, (60 * beats) / b);
        assert.ok(len > lo - 1e-9 && len < hi + 1e-9, `${piece.id}: bar ${bar} [${a}, ${b}]: ${len} not in ${lo}..${hi}`);
      }
      near(time.at(bar, beats), time.at(bar + 1, 0), `${piece.id}: bar ${bar}: its end against the next bar's start`);
      for (let beat = 1; beat < beats; beat++) assert.ok(time.at(bar, beat) > time.at(bar, beat - 1), `${piece.id}: bar ${bar}: beat ${beat}`);
    });
    assert.equal(time.seconds, time.starts.at(-1));
  }
});

test("tempoMap: six in a bar, and a bar that slows", () => {
  const time = tempoMap({ beats: 6, bars: [["X", 120], ["X", [120, 60]], ["X", 60]] });
  near(time.starts[1], 3, "a bar of six at 120");
  near(time.starts[3] - time.starts[2], 6, "a bar of six at 60");
  // From 120 to 60 over six beats: the time is (60 * 6 / (60 - 120)) ln((120 - 60 * beat / 6) / 120).
  near(time.starts[2] - time.starts[1], 6 * Math.log(2), "the slowing bar");
  near(time.at(1, 3) - time.at(1, 0), -6 * Math.log(90 / 120), "half way through the slowing bar");
  near(time.at(0, 6), time.at(1, 0), "beat 6 is the next bar");
  near(time.at(0, 7), time.at(1, 1), "beat 7 is the next bar's second");
  // Past the end, the last tempo goes on.
  near(time.at(3, 3), time.seconds + 3, "past the end");
  // Four in a bar when left out.
  near(tempoMap({ bars: [["X", 60]] }).seconds, 4, "four in a bar");
});

// ---------------------------------------------------------------------------------------------
// The format, on small pieces

test("parseLine: lengths, cut short, stressed, the bar line in any metre", () => {
  const n = parseLine("<mf> C5:2' D5! E5:1 -", 0, 4);
  assert.equal(n.length, 4);
  assert.deepEqual([n[0].beat, n[0].beats, n[0].len], [0, 2, 1], "' halves how long it sounds, not its place");
  assert.equal(n[0].m, 72);
  near(n[0].v, MARKS.mf, "no stress");
  assert.deepEqual([n[1].beat, n[1].beats, n[1].len], [2, 1, 1]);
  near(n[1].v, MARKS.mf * 1.25, "! stresses");
  assert.equal(n[2].beat, 3);
  assert.equal(n[3].m, null, "a rest");
  assert.equal(parseLine("<ff> C5!")[0].v, 1, "a stress stops at 1");
  assert.equal(parseLine("C5", 1)[0].m, 84, "oct");
  // Three in a bar.
  assert.doesNotThrow(() => parseLine("C5 D5 E5 | F5:3 | G5:1.5 A5:1.5", 0, 3));
  assert.throws(() => parseLine("C5 D5 | E5", 0, 3), /beats before "\|"/);
  assert.throws(() => parseLine("C5 D5 E5 F5 | G5", 0, 3), /beats before "\|"/);
  // Six in a bar.
  assert.doesNotThrow(() => parseLine("C5:3 D5:3 | E5:6", 0, 6));
  assert.throws(() => parseLine("C5:4 | D5", 0, 6), /beats before "\|"/);
  // Four when left out.
  assert.throws(() => parseLine("C5 D5 E5 | F5"), /beats before "\|"/);
  assert.throws(() => parseLine("C5:3 | D5"), /beats before "\|"/);
  assert.throws(() => parseLine("<loud> C5"), /no dynamic/);
  assert.throws(() => parseLine("H5"), /not a note/);
  assert.doesNotThrow(() => parseLine("C5:1.5 D5:.5 E5:2 | F5:4"));
});

/** A small piece at 60 to the minute, four in a bar (a beat is a second, a bar four), with only
 *  the patterns given. */
const mini = (patterns, over = {}) => ({
  id: "mini",
  title: "Mini",
  seed: 5,
  chords: {
    C: { bass: "C2", tones: "C4 E4 G4", two: "C4 E4 | D4 F4" },
    G: { bass: "G2", tones: "G3 B3 D4", two: "G3 B3 | A3 C4" },
  },
  bars: [["C", 60], ["G", 60], ["C", 60], ["G", 60]],
  layers: {},
  dynamics: { lute: 0.5, ramp: [[0, 0.2], [4, 0.6]] },
  lines: [],
  patterns,
  ...over,
});
const play = (patterns, over) => compile(mini(patterns, over), 0).events;
const brief = (events) => events.map((e) => [e.t, e.m]);

test("patterns: chord indices, step, len, the level", () => {
  const ev = play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, figure: "0 1 2 -" }]);
  assert.deepEqual(brief(ev), [[0, 60], [1, 64], [2, 67]]);
  for (const e of ev) {
    assert.equal(e.voice, "lute");
    near(e.d, 1, "len is step when left out");
    near(e.v, 0.5, "a numeric dynamics entry");
  }
  // A short figure repeats to fill the bar; `|` is ignored; `len`.
  const rep = play([{ voice: "lute", of: "tones", bars: [[1, 1]], step: 0.5, len: 0.25, figure: "0 | 2" }]);
  assert.deepEqual(brief(rep), [[4, 55], [4.5, 62], [5, 55], [5.5, 62], [6, 55], [6.5, 62], [7, 55], [7.5, 62]]);
  for (const e of rep) near(e.d, 0.25, "len");
  // The level from `level`, along its points.
  const lev = play([{ voice: "lute", level: "ramp", of: "tones", bars: [[2, 2]], step: 4, figure: "0" }]);
  assert.equal(lev.length, 1);
  near(lev[0].v, 0.4, "the ramp at bar 2");
});

test("patterns: octave marks, oct, stress and lightness, a literal note", () => {
  const ev = play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, figure: "0' 1, 2'' D4" }]);
  assert.deepEqual(brief(ev), [[0, 72], [1, 52], [2, 91], [3, 62]]);
  const up = play([{ voice: "lute", of: "tones", oct: 1, bars: [[0, 0]], step: 1, figure: "0 1, - D4" }]);
  assert.deepEqual(brief(up), [[0, 72], [1, 64], [3, 74]], "oct moves every note, the literal one too");
  const acc = play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, figure: "0! 1? 2 -" }]);
  assert.deepEqual(
    acc.map((e) => Math.round(e.v * 1e6) / 1e6),
    [0.65, 0.3, 0.5],
  );
  // A literal note needs no chord field.
  assert.deepEqual(brief(play([{ voice: "lute", bars: [[1, 1]], step: 2, figure: "A3 -" }])), [[4, 57]]);
});

test("patterns: the whole chord, rolled", () => {
  const ev = play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, roll: 0.05, figure: "* - - -" }]);
  assert.equal(ev.length, 3);
  assert.deepEqual(ev.map((e) => e.m), [60, 64, 67]);
  ev.forEach((e, k) => {
    near(e.t, k * 0.05, `note ${k}: rolled`);
    near(e.d, 1, `note ${k}: d`);
    near(e.v, 0.5 * 0.8, `note ${k}: each at 0.8 of the level`);
  });
  const together = play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 4, figure: "*!" }]);
  assert.deepEqual(together.map((e) => e.t), [0, 0, 0], "no roll: together");
  for (const e of together) near(e.v, 0.5 * 1.3 * 0.8, "stressed and together");
});

test("patterns: a bar of two chords takes each half in turn", () => {
  const ev = play([{ voice: "lute", of: "two", bars: [[0, 1]], step: 1, figure: "0 1 0 1" }]);
  assert.deepEqual(brief(ev), [[0, 60], [1, 64], [2, 62], [3, 65], [4, 55], [5, 59], [6, 57], [7, 60]]);
  const all = play([{ voice: "lute", of: "two", bars: [[0, 0]], step: 2, figure: "*" }]);
  assert.deepEqual(brief(all), [[0, 60], [0, 64], [2, 62], [2, 65]]);
});

test("patterns: every n-th bar, a list of figures taking turns, several spans", () => {
  const every = play([{ voice: "lute", of: "tones", bars: [[0, 3]], every: 2, step: 4, figure: "0" }]);
  assert.deepEqual(brief(every), [[0, 60], [8, 60]]);
  const fromSpan = play([{ voice: "lute", of: "tones", bars: [[1, 3]], every: 2, step: 4, figure: "0" }]);
  assert.deepEqual(brief(fromSpan), [[4, 55], [12, 55]], "every counts from the span's first bar");
  const turns = play([{ voice: "lute", of: "tones", bars: [[0, 3]], step: 4, figure: ["0", "1", "2"] }]);
  assert.deepEqual(brief(turns), [[0, 60], [4, 59], [8, 67], [12, 55]]);
  const spans = play([{ voice: "lute", of: "tones", bars: [[0, 0], [2, 3]], step: 4, figure: ["0", "1"] }]);
  assert.deepEqual(brief(spans), [[0, 60], [8, 60], [12, 59]], "the turns start again in each span");
});

test("patterns: human looseness varies with the playing; without it both playings are alike", () => {
  const tight = mini([{ voice: "lute", of: "tones", bars: [[0, 3]], step: 0.5, figure: "0 1 2 1" }]);
  assert.deepEqual(compile(tight, 0), compile(tight, 1));
  const loose = mini([{ voice: "lute", of: "tones", bars: [[0, 3]], step: 0.5, figure: "0 1 2 1", human: 0.01 }]);
  const a = compile(loose, 0).events;
  assert.deepEqual(compile(loose, 0).events, a, "the same playing, the same each time");
  assert.notDeepEqual(compile(loose, 1).events, a);
  const exact = compile(tight, 0).events;
  assert.equal(a.length, exact.length);
  // Within its looseness of the written time, and within 4% of the level.
  const byPitchAndBeat = (e) => `${e.m}`;
  a.forEach((e) => {
    const nearest = exact.filter((x) => byPitchAndBeat(x) === byPitchAndBeat(e)).reduce((best, x) => (Math.abs(x.t - e.t) < Math.abs(best.t - e.t) ? x : best));
    assert.ok(Math.abs(nearest.t - e.t) <= 0.005 + 1e-9, `${e.m} at ${e.t}: ${nearest.t}`);
    assert.ok(Math.abs(e.v / nearest.v - 1) <= 0.04 + 1e-9, `${e.m} at ${e.t}: v ${e.v}`);
  });
});

test("patterns: the errors", () => {
  assert.throws(() => play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, figure: "3 - - -" }]), /has no note in chord "C"/);
  assert.throws(() => play([{ voice: "lute", of: "two", bars: [[0, 0]], step: 1, figure: "2 - - -" }]), /has no note in chord "C"/);
  assert.throws(() => play([{ voice: "drum", bars: [[0, 0]], step: 1, figure: "C3" }]), /no dynamics "drum"/);
  assert.throws(() => play([{ voice: "lute", level: "nope", bars: [[0, 0]], step: 1, figure: "C3" }]), /no dynamics "nope"/);
  assert.throws(() => play([{ voice: "lute", of: "tones", bars: [[0, 0]], step: 1, figure: "x" }]), /cannot read "x"/);
});

// ---------------------------------------------------------------------------------------------
// The orchestra's settings

test("mixFor: a piece's mix laid over MIX, voice by voice", () => {
  const before = JSON.stringify(MIX);
  assert.deepEqual(mixFor({}), MIX);
  for (const piece of ALL) if (!piece.mix) assert.deepEqual(mixFor(piece), MIX, piece.id);
  const m = mixFor({ mix: { brass: { level: 0.1 }, harp: { pan: 0.5, hall: 0 } } });
  assert.deepEqual(Object.keys(m), Object.keys(MIX), "the same voices");
  assert.deepEqual(m.brass, { ...MIX.brass, level: 0.1 }, "the other fields kept");
  assert.deepEqual(m.harp, { ...MIX.harp, pan: 0.5, hall: 0 });
  for (const v of Object.keys(MIX)) if (v !== "brass" && v !== "harp") assert.deepEqual(m[v], MIX[v], v);
  assert.equal(JSON.stringify(MIX), before, "MIX itself unchanged");
});

test("settingsFor: the mix, and trim and hall only where the piece has them", () => {
  assert.deepEqual(settingsFor({}), { mix: mixFor({}) });
  assert.deepEqual(Object.keys(settingsFor({ trim: 0.5 })).sort(), ["mix", "trim"]);
  assert.equal(settingsFor({ trim: 0.5 }).trim, 0.5);
  assert.deepEqual(Object.keys(settingsFor({ hall: 0 })).sort(), ["hall", "mix"]);
  assert.equal(settingsFor({ hall: 0 }).hall, 0, "a hall of 0 is a hall");
  const s = settingsFor({ trim: 0.6, hall: 1.2, mix: { lute: { level: 0.2 } } });
  assert.deepEqual(s, { mix: mixFor({ mix: { lute: { level: 0.2 } } }), trim: 0.6, hall: 1.2 });
  for (const piece of ALL) assert.deepEqual(settingsFor(piece).mix, mixFor(piece), piece.id);
});

// ---------------------------------------------------------------------------------------------
// The conductor

const STEP = 0.4;
const AHEAD = 4;

/** Runs a conductor on a simulated clock from `from` to `to`; `jump(now)` may move the clock on. */
function drive(conductor, from, to, jump = (now) => now) {
  const out = [];
  for (let now = from; now < to; now += STEP) {
    now = jump(now);
    const got = conductor.take(now, now + AHEAD);
    for (let i = 0; i < got.length; i++) {
      assert.ok(got[i].t >= now, `at ${now}: a note at ${got[i].t}, in the past`);
      // (1e-9: after a shift, notes written together can come out a rounding error apart.)
      if (i > 0) assert.ok(got[i].t >= got[i - 1].t - 1e-9, `at ${now}: out of order`);
    }
    if (out.length && got.length) assert.ok(got[0].t >= out.at(-1).t - 1e-9, `at ${now}: before the last window's notes`);
    out.push(...got);
  }
  return out;
}

const same = (a, b) => a.voice === b.voice && a.m === b.m && a.d === b.d && a.v === b.v && a.v2 === b.v2;

test("Conductor: one playing's notes exactly once, on time", () => {
  const score = compile(VIGIL, 0);
  const c = new Conductor(VIGIL);
  const origin = 0.25;
  const got = drive(c, 0, score.seconds + 2);
  // The last note handed out, the conductor is set for the next playing: no time was lost.
  assert.ok(Math.abs(c.origin - (origin + score.seconds + VIGIL.gap)) < 1e-9, `next playing at ${c.origin}`);
  assert.equal(got.length, score.events.length);
  got.forEach((e, i) => {
    assert.ok(same(e, score.events[i]), `note ${i}`);
    assert.ok(Math.abs(e.t - (origin + score.events[i].t)) < 1e-9, `note ${i} at ${e.t}`);
  });
});

test("Conductor: after a stall nothing is skipped and nothing is late", () => {
  const score = compile(VIGIL, 0);
  const c = new Conductor(VIGIL);
  let stalled = false;
  const got = drive(c, 0, score.seconds + 40, (now) => {
    if (!stalled && now >= 60) {
      stalled = true;
      return now + 30;
    }
    return now;
  });
  assert.ok(stalled);
  // The first playing, whole (the second starts at least `gap` seconds after its end).
  const first = got.filter((e) => e.t < c.origin);
  assert.equal(first.length, score.events.length, "every note of the playing came");
  first.forEach((e, i) => assert.ok(same(e, score.events[i]), `note ${i}`));
  // The notes after the stall keep their spacing: the music moved later, it did not bunch up.
  const after = first.findIndex((e) => e.t > 90);
  assert.ok(after > 0);
  for (let i = after + 1; i < first.length; i++) {
    const want = score.events[i].t - score.events[i - 1].t;
    assert.ok(Math.abs(first[i].t - first[i - 1].t - want) < 1e-9, `note ${i}: spacing`);
  }
});

test("Conductor: the second playing follows the silence and hands the theme to the horn", () => {
  const score = compile(VIGIL, 0);
  const time = tempoMap(VIGIL);
  const c = new Conductor(VIGIL);
  const firstOrigin = 0.25;
  const got = drive(c, 0, score.seconds + VIGIL.gap + time.at(13, 0) + 1);
  assert.equal(c.pass, 1);
  assert.ok(Math.abs(c.origin - (firstOrigin + score.seconds + VIGIL.gap)) < 1e-9, `second playing at ${c.origin}`);
  const inTheme = (origin) => (e) => e.t - origin >= time.at(4, 0) - 0.02 && e.t - origin < time.at(12, 0) - 0.02;
  const once = got.filter((e) => e.t < c.origin).filter(inTheme(firstOrigin));
  const twice = got.filter((e) => e.t >= c.origin).filter(inTheme(c.origin));
  const themeNotes = parseLine(VIGIL.lines[0].notes).filter((n) => n.m !== null);
  assert.equal(once.filter((e) => e.voice === "flute").length, themeNotes.length, "first playing: the flute has the theme");
  assert.equal(once.filter((e) => e.voice === "horn").length, 0);
  const horn = twice.filter((e) => e.voice === "horn");
  assert.equal(twice.filter((e) => e.voice === "flute").length, 0, "second playing: no flute in bars 4..11");
  assert.deepEqual(
    horn.map((e) => e.m),
    themeNotes.map((n) => n.m - 12),
    "second playing: the horn has the theme, an octave down",
  );
});

// ---------------------------------------------------------------------------------------------
// The harp's string

/** The period in samples by autocorrelation over the first half second, near `expected`, with a
 *  parabola through the best lag and its neighbours. */
function periodOf(x, sampleRate, expected) {
  const n = Math.floor(0.5 * sampleRate);
  const r = (k) => {
    let s = 0;
    for (let i = 0; i < n; i++) s += x[i] * x[i + k];
    return s;
  };
  let best = 0;
  let bestR = -Infinity;
  for (let k = Math.floor(expected * 0.97); k <= Math.ceil(expected * 1.03); k++) {
    const v = r(k);
    if (v > bestR) {
      bestR = v;
      best = k;
    }
  }
  const a = r(best - 1);
  const b = bestR;
  const c = r(best + 1);
  return best + (0.5 * (a - c)) / (a - 2 * b + c);
}

test("pluckSamples: finite, normalised, faded, in tune", () => {
  for (const sampleRate of [44100, 48000]) {
    // Across the harp's compass in Vigil (A1 to D6). (Above about A6 the first sample is larger,
    // up to 0.38: a period of a couple of dozen samples leaves the zero crossing coarse.)
    for (const m of [33, 45, 57, 62, 69, 81, 86]) {
      const x = pluckSamples(sampleRate, m);
      const where = `m ${m} at ${sampleRate}`;
      assert.ok(x.length > sampleRate, `${where}: ${x.length} samples`);
      let peak = 0;
      for (let i = 0; i < x.length; i++) {
        assert.ok(Number.isFinite(x[i]) && x[i] >= -1 && x[i] <= 1, `${where}: sample ${i} is ${x[i]}`);
        peak = Math.max(peak, Math.abs(x[i]));
      }
      assert.ok(Math.abs(peak - 1) < 1e-6, `${where}: peak ${peak}`);
      assert.ok(Math.abs(x[0]) < 0.2, `${where}: starts at ${x[0]}`);
      assert.ok(Math.abs(x[x.length - 1]) < 1e-5, `${where}: ends at ${x[x.length - 1]}`);
      const f = 440 * 2 ** ((m - 69) / 12);
      const period = periodOf(x, sampleRate, sampleRate / f);
      const cents = 1200 * Math.log2(sampleRate / period / f);
      assert.ok(Math.abs(cents) < 3, `${where}: ${cents.toFixed(2)} cents out`);
    }
  }
});

// ---------------------------------------------------------------------------------------------
// The player, without Web Audio

test("music without Web Audio: no sound, no throw, the choice remembered", () => {
  assert.equal(music.available, false);
  stored.set("ffh.music", JSON.stringify({ on: false, volume: 0.3 }));
  assert.doesNotThrow(() => music.init());
  assert.equal(music.on, false, "init reads the remembered choice");
  assert.equal(music.volume, 0.3);
  assert.doesNotThrow(() => music.init(), "a second init is harmless");
  assert.equal(music.playing, false);
  assert.doesNotThrow(() => music.setOn(true));
  assert.equal(music.on, true);
  assert.equal(music.playing, false);
  assert.doesNotThrow(() => music.setVolume(2));
  assert.equal(music.volume, 1);
  assert.doesNotThrow(() => music.setVolume(-1));
  assert.equal(music.volume, 0);
  assert.doesNotThrow(() => music.setVolume(0.5));
  assert.equal(music.volume, 0.5);
  assert.doesNotThrow(() => music.setOn(false));
  assert.equal(music.on, false);
  const saved = JSON.parse(stored.get("ffh.music"));
  assert.equal(saved.on, false);
  assert.equal(saved.volume, 0.5);
});

test("music: the pieces, and the one chosen", () => {
  // After the test above: init has read a remembered choice with no piece in it.
  assert.ok(Array.isArray(music.pieces), "music.pieces is a list");
  assert.deepEqual(
    music.pieces.map((p) => [p.id, p.title]),
    Object.values(PIECES).map((p) => [p.id, p.title]),
    "every piece, in PIECES order",
  );
  for (const p of music.pieces) assert.equal(p.style || undefined, PIECES[p.id].style || undefined, `${p.id}: style`);
  assert.equal(music.piece, "vigil", "Vigil when none is remembered");
  assert.doesNotThrow(() => music.setPiece("nope"));
  assert.equal(music.piece, "vigil", "an unknown id is ignored");
  const second = Object.keys(PIECES)[1];
  if (second === undefined) return;
  assert.doesNotThrow(() => music.setPiece(second));
  assert.equal(music.piece, second);
  assert.equal(JSON.parse(stored.get("ffh.music")).piece, second, "the choice remembered");
  assert.equal(music.playing, false);
  assert.doesNotThrow(() => music.setPiece("vigil"));
  assert.equal(music.piece, "vigil");
});
