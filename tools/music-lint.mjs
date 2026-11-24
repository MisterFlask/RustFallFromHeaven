#!/usr/bin/env node
// Reads the music's pieces for what an ear would catch and a composer working blind would not
// (docs/MUSIC.md, "Checking it"). It reports, it does not fail: a suspended note is a dissonance
// on purpose. Read each line and keep the ones meant.
//
//   node tools/music-lint.mjs            every piece
//   node tools/music-lint.mjs onslaught  the pieces named
//   node tools/music-lint.mjs --all      every line (the first 14 of each kind otherwise)
//
// Per piece:
//   range   a note outside what its voice is written for (music_synth.js RANGES)
//   clash   two notes a semitone apart (in any octave) sounding together for 0.4 s or more
//   off     a melodic line's note on a strong beat that is not in the bar's chord
//   mud     two notes a third or less apart, both under C3, sounding together

import { PIECES, compile, tempoMap, parseLine, midi } from "../crates/ffh-play/assets/app/music_score.js";
import { RANGES } from "../crates/ffh-play/assets/app/music_synth.js";

const NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
const name = (m) => NAMES[((m % 12) + 12) % 12] + (Math.floor(m / 12) - 1);
/** Voices whose notes ring on after they are played: one against the next is a figure, not a clash. */
const RINGING = new Set(["harp", "lute", "celesta", "bell"]);
const LIMIT = process.argv.includes("--all") ? Infinity : 14;

function lint(piece) {
  const beats = piece.beats ?? 4;
  const time = tempoMap(piece);
  const { events, seconds } = compile(piece, 0);
  const where = (t) => {
    let bar = 0;
    while (bar < piece.bars.length - 1 && time.starts[bar + 1] <= t + 1e-6) bar++;
    const beat = ((t - time.starts[bar]) / (time.starts[bar + 1] - time.starts[bar])) * beats;
    return `bar ${bar} beat ${beat.toFixed(1)}`;
  };
  const found = { range: [], clash: [], off: [], mud: [] };

  const pitched = events.filter((e) => RANGES[e.voice]);
  for (const e of pitched) {
    const [lo, hi] = RANGES[e.voice];
    if (e.m < lo || e.m > hi) found.range.push(`${where(e.t)}: ${e.voice} ${name(e.m)} (written for ${name(lo)} to ${name(hi)})`);
  }

  const seen = new Set();
  for (let i = 0; i < pitched.length; i++) {
    const a = pitched[i];
    for (let j = i + 1; j < pitched.length && pitched[j].t < a.t + a.d; j++) {
      const b = pitched[j];
      const together = Math.min(a.t + a.d, b.t + b.d) - b.t;
      const apart = Math.abs(a.m - b.m);
      if (a.m < 48 && b.m < 48 && apart >= 1 && apart <= 4 && together >= 0.3) {
        const key = `mud ${where(b.t).split(" beat")[0]} ${a.voice} ${b.voice} ${a.m} ${b.m}`;
        if (!seen.has(key)) found.mud.push(`${where(b.t)}: ${a.voice} ${name(a.m)} with ${b.voice} ${name(b.m)} for ${together.toFixed(1)} s`);
        seen.add(key);
      }
      if (together < 0.4 || (apart % 12 !== 1 && apart % 12 !== 11)) continue;
      if (a.voice === b.voice && RINGING.has(a.voice)) continue;
      const key = `clash ${where(b.t).split(" beat")[0]} ${a.voice} ${b.voice} ${a.m % 12} ${b.m % 12}`;
      if (!seen.has(key)) found.clash.push(`${where(b.t)}: ${a.voice} ${name(a.m)} against ${b.voice} ${name(b.m)} for ${together.toFixed(1)} s`);
      seen.add(key);
    }
  }

  // The chord of a bar's half, as pitch classes: every note its entry names.
  const classes = (bar, beat) => {
    const chord = piece.chords[piece.bars[bar]?.[0]];
    const set = new Set();
    for (const field of Object.values(chord ?? {})) {
      if (typeof field !== "string") continue;
      const parts = field.split("|");
      const part = parts[Math.min(parts.length - 1, Math.floor((beat * parts.length) / beats))];
      for (const w of part.trim().split(/\s+/)) if (w !== "-" && w) set.add(midi(w) % 12);
    }
    return set;
  };
  const strong = beats % 2 === 0 ? [0, beats / 2] : [0];
  for (const line of piece.lines) {
    for (const n of parseLine(line.notes, line.oct ?? 0, beats)) {
      if (n.m === null) continue;
      const bar = line.bar + Math.floor(n.beat / beats + 1e-9);
      const beat = n.beat - (bar - line.bar) * beats;
      if (bar >= piece.bars.length || !strong.some((s) => Math.abs(beat - s) < 1e-9)) continue;
      if (!classes(bar, beat).has(n.m % 12)) found.off.push(`bar ${bar} beat ${beat}: ${line.voice} ${name(n.m)}, ${n.beats} beats, over ${piece.bars[bar][0]}`);
    }
  }

  console.log(`\n${piece.id}: ${seconds.toFixed(1)} s, ${events.length} notes`);
  for (const [kind, list] of Object.entries(found)) {
    console.log(`  ${kind}: ${list.length}`);
    for (const line of list.slice(0, LIMIT)) console.log(`    ${line}`);
    if (list.length > LIMIT) console.log(`    ... and ${list.length - LIMIT} more`);
  }
}

const ids = process.argv.slice(2).filter((a) => !a.startsWith("--"));
for (const id of ids.length ? ids : Object.keys(PIECES)) {
  if (!PIECES[id]) {
    console.error(`music-lint: no piece "${id}" (${Object.keys(PIECES).join(", ")})`);
    process.exit(1);
  }
  lint(PIECES[id]);
}
