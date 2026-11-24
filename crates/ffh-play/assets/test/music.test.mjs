// node --test: the game's music (docs/MUSIC.md). The score (app/music_score.js: chords, lines,
// ranges, the tempo map), the conductor's scheduling over a simulated clock (app/music.js
// `Conductor`), the harp's plucked string (app/music_synth.js `pluckSamples`), and the player's
// switch without Web Audio. All pure: no sound is made.
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

const { VIGIL, PIECES, compile, tempoMap, parseLine, midi } = await import("../app/music_score.js");
const { pluckSamples } = await import("../app/music_synth.js");
const { Conductor, music } = await import("../app/music.js");

/** Each voice's playable range (MIDI, both ends included). */
const RANGE = {
  harp: [24, 103],
  flute: [60, 96],
  horn: [41, 77],
  violins: [55, 100],
  pad: [48, 84],
  bass: [28, 55],
  choirAh: [40, 81],
  choirOo: [40, 81],
  timpani: [38, 57],
  celesta: [60, 108],
};

const pitchesOf = (s) => s.split(/[\s|]+/).filter((w) => w && w !== "-").map(midi);

// ---------------------------------------------------------------------------------------------
// The score

test("every bar names a chord, every harp figure is a bar of eighths", () => {
  for (const piece of Object.values(PIECES)) {
    piece.bars.forEach(([name], i) => assert.ok(piece.chords[name], `${piece.id}: bar ${i}: no chord "${name}"`));
    for (const [name, c] of Object.entries(piece.chords)) {
      assert.equal(c.harp.trim().split(/\s+/).length, 8, `${piece.id}: chord ${name}: the harp has 8 eighths`);
    }
  }
});

const ALL = Object.values(PIECES);

test("compile: ordered, finite, inside the piece", () => {
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
    }
  }
  // Vigil itself: about 2:54.
  const { events, seconds } = compile(VIGIL, 0);
  assert.ok(events.length > 500 && seconds > 170 && seconds < 178, `Vigil: ${events.length} events, ${seconds} s`);
});

test("compile is the same each time; the second playing differs", () => {
  for (const piece of ALL) {
    const a0 = compile(piece, 0);
    assert.deepEqual(compile(piece, 0), a0, piece.id);
    const a1 = compile(piece, 1);
    assert.deepEqual(compile(piece, 1), a1, piece.id);
    assert.notDeepEqual(a0.events, a1.events, piece.id);
  }
});

test("every line parses and ends inside the piece; a bar that does not add up is an error", () => {
  for (const piece of ALL) {
    const time = tempoMap(piece);
    for (const line of piece.lines) {
      const notes = parseLine(line.notes, line.oct ?? 0);
      assert.ok(notes.length > 0);
      const last = notes[notes.length - 1];
      const end = time.at(line.bar, last.beat + last.beats);
      assert.ok(end <= time.seconds + 1e-9, `${piece.id}: the ${line.voice} line at bar ${line.bar} ends at ${end}, after ${time.seconds}`);
    }
  }
  assert.throws(() => parseLine("C5 D5 E5 | F5"), /beats before "\|"/);
  assert.throws(() => parseLine("C5:3 | D5"), /beats before "\|"/);
  assert.throws(() => parseLine("<loud> C5"), /no dynamic/);
  assert.throws(() => parseLine("H5"), /not a note/);
  assert.doesNotThrow(() => parseLine("C5:1.5 D5:.5 E5:2 | F5:4"));
});

test("pitches stay in each voice's range, on both playings", () => {
  for (const piece of ALL) {
    for (const pass of [0, 1]) {
      for (const e of compile(piece, pass).events) {
        const r = RANGE[e.voice];
        assert.ok(r, `${piece.id}: no range for voice "${e.voice}"`);
        assert.ok(e.m >= r[0] && e.m <= r[1], `${piece.id}, pass ${pass}: ${e.voice} plays ${e.m} at ${e.t.toFixed(2)} s, outside ${r[0]}..${r[1]}`);
      }
    }
  }
});

test("a piece ends on the root it starts on (it loops)", () => {
  for (const piece of ALL) {
    const first = piece.chords[piece.bars[0][0]];
    const last = piece.chords[piece.bars[piece.bars.length - 1][0]];
    const lowFirst = pitchesOf(first.bass)[0];
    const lowLast = pitchesOf(last.bass).at(-1);
    assert.equal(lowFirst % 12, lowLast % 12, piece.id);
  }
});

// ---------------------------------------------------------------------------------------------
// The tempo map

test("tempoMap: bars in order, slowing bars between their tempos, bar ends meet", () => {
  for (const piece of ALL) {
    const time = tempoMap(piece);
    assert.equal(time.starts.length, piece.bars.length + 1);
    for (let i = 1; i < time.starts.length; i++) assert.ok(time.starts[i] > time.starts[i - 1], `${piece.id}: bar ${i - 1} has no length`);
    piece.bars.forEach(([, bpm], bar) => {
      const len = time.starts[bar + 1] - time.starts[bar];
      if (typeof bpm === "number") {
        assert.ok(Math.abs(len - 240 / bpm) < 1e-9, `${piece.id}: bar ${bar}: ${len}`);
      } else {
        const [a, b] = bpm;
        const lo = Math.min(240 / a, 240 / b);
        const hi = Math.max(240 / a, 240 / b);
        assert.ok(len > lo - 1e-9 && len < hi + 1e-9, `${piece.id}: bar ${bar} [${a}, ${b}]: ${len} not in ${lo}..${hi}`);
      }
      assert.ok(Math.abs(time.at(bar, 4) - time.at(bar + 1, 0)) < 1e-9, `${piece.id}: bar ${bar}: its end is not the next bar's start`);
      for (const beat of [1, 2, 3]) assert.ok(time.at(bar, beat) > time.at(bar, beat - 1), `${piece.id}: bar ${bar}: beat ${beat}`);
    });
    assert.equal(time.seconds, time.starts.at(-1));
  }
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
  assert.deepEqual(JSON.parse(stored.get("ffh.music")), { on: false, volume: 0.5 });
});
