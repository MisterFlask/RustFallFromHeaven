# The music

The browser client plays one piece of music, "Vigil": D minor, four beats in a bar, 63 to the
minute slowing to 40 at the end, 43 bars, about 2:54 (173.7 s). The page plays it from code: the
score is data, the instruments are Web Audio nodes and a little arithmetic, and the client ships no
sound files. It loops, with a silence between playings.

Vigil, in outline: a harp alone states the ground (a bass that walks down from D to A); a flute
has the theme over it (bars 4-11); the strings and a choir join for its repeat (12-19); the horn
answers in F (20-27); a harp glissando and the timpani lead into the full orchestra's return of the
theme (28-35), which turns aside at the cadence (B flat for D minor); the coda walks B flat, C, D
up into D major (36-42) and ends on the major third and a silence.

## The modules

All three are in `crates/ffh-play/assets/app/`, served with the rest of the client
(`src/transport/http.rs` `ASSETS`).

| Module | What it owns |
|---|---|
| `music_score.js` | The pieces as data (`VIGIL`, `PIECES`) and `compile(piece, pass)`: the timed notes of one playing, `{ events, seconds }`, each event `{ voice, t, d, m, v, v2 }` (start and length in seconds, MIDI pitch, velocity at the note's start and end), in order of `t`. Also `tempoMap`, `levelAt`, `parseLine`, `midi`, `MARKS` and `mulberry32` (the seeded generator behind the small unevenness of the playing). Pure: no Web Audio, it runs in node. |
| `music_synth.js` | The orchestra: `createOrchestra(ctx)` builds every voice on an `AudioContext` (the page) or an `OfflineAudioContext` (a measurement) and returns `{ play(event), out, close() }`. `MIX` is each voice's level, pan and hall send. `pluckSamples(sampleRate, midi)`, the harp's string, is pure and runs in node. |
| `music.js` | The player. `Conductor` hands a looping piece's notes out in windows of the context's clock; `renderPiece(piece, opts)` renders one playing offline (what the measurements read); `music` is the page's switch: `init()`, `available`, `on`, `volume`, `playing`, `setOn(bool)`, `setVolume(0..1)`. |

## The score format

A piece is an object in `music_score.js` (see `VIGIL`):

- `id`, `title`, `seed` (the humanizing's seed), `gap` (seconds of silence between playings).
- `bars`: `[chord, bpm]` per bar, four beats each. Bars are counted from 0 everywhere in a piece
  (`layers`, `dynamics`, `lines`, `extras`; the bar numbers in this document too). `bpm` a number is a steady tempo; a pair
  `[from, to]` is a bar that slows (or speeds) evenly, beat by beat, from the first tempo to the
  second (`tempoMap` integrates it).
- `chords`: the table the bars name. Each chord has:
  - `bass`: the low strings' note; two separated by `|` for a bar of two chords (`"Eb2 | A1"`).
  - `harp`: the bar in eighths, eight notes (`-` an eighth left out).
  - `pad`: the voicing the strings hold, and the choir with them; two voicings separated by `|` for
    a bar of two chords.
  - `ladder`: five notes the harp's sixteenth sweep climbs (and their octaves above), or `sweep`:
    the sixteen notes written out.
- `layers`: which bars each accompaniment layer plays (first and last bar, both included):
  `harp.eighths` and `harp.sixteenths` (lists of spans), `harp.low` (a span: the bass note on the
  harp as well), `pad`, `bass`, and `choir` (spans, each with its `vowel`, `"ah"` or `"oo"`).
- `dynamics`: per layer, `[bar, velocity]` points; the level between two points is the straight
  line through them (`levelAt`).
- `lines`: the melodic lines, `{ voice, bar, notes, oct, alt }`. `bar` is where the line starts.
  `notes` is read by `parseLine`:
  - `C5:1.5` a note and its length in beats (one when left out); `Bb3`, `F#5` with accidentals;
  - `-` a rest (`-:.5` half a beat);
  - `<mf>` the dynamic from there on (`pp p mp mf f ff`, `MARKS`);
  - a trailing `^` a note that swells, `_` one that fades;
  - `|` a bar line, checked: a line whose bar does not add up to four beats is an error, not a late
    entrance.

  `oct` moves the line by octaves. `alt: { voice, oct }` is who plays the line instead on every
  second playing (so the loop does not wear).
- `extras`: single events, `{ voice, bar, beat, step, vel, notes, beats }`: `notes` one after
  another every `step` beats (0: together), the velocity going from `vel[0]` to `vel[1]`, each
  note `beats` long (2 when left out). Vigil's glissando, timpani strokes, celesta notes and last
  rolled chord are extras.

## The orchestra

Each voice goes to its bus (`MIX`: level, pan, hall send); the buses and the hall go to the mix, a
soft ceiling and the listener's volume (`out`).

| Voice | How it is made |
|---|---|
| `harp` | Karplus-Strong plucked strings: a delay line a period long fed back through a two-point average and a fractional-delay allpass, started from the string's shape at release. Computed into a buffer once per pitch (`pluckSamples`), normalized to a peak of 1 and faded at the end; each note plays the buffer through a lowpass that opens with the velocity. |
| `violins`, `pad`, `bass` | Detuned band-limited waves (a few oscillators a note on a computed periodic wave), each player a little out of tune with the others, shared slow vibrato, under a lowpass that tracks the pitch. |
| `choirAh`, `choirOo` | The same kind of detuned waves (a brighter spectrum) through a vowel's formant filters (bandpasses at the mouth's resonances) and a low chest band. |
| `flute` | A near-sine wave with a vibrato that arrives once the note is under way, and filtered noise for the breath (strong at the attack, a little under the note). |
| `horn` | Two detuned players under a lowpass that opens as the note speaks, wider the louder it is, and closes at its end. |
| `timpani`, `celesta` | Decaying sine partials (`[ratio, level, seconds to die]`); the drum's pitch bends down into the note and a burst of low noise is its stick. |
| the hall | A `ConvolverNode` with a computed impulse response: early reflections, then noise that dies away, the highs first (3.6 s in the bass, 1.3 s at the top), different in each ear. |
| the output | The mix through a `tanh` soft ceiling (a `WaveShaperNode`) in place of clipping, then the volume. |

## The player

- It starts on the player's first key or click (`mousedown`, `pointerup`, `touchend`, `keydown`):
  browsers let a page make sound only after one. It fades in over 3 s.
- It loops: a playing, 8 s of silence (`gap`), the next. Every second playing hands the first theme
  (bars 4-11) to the horn, an octave down, and the horn's answer (bars 20-23) to the flute, an
  octave up (the lines' `alt`).
- It suspends while the tab is hidden and goes on where it was when the tab comes back.
- Notes are scheduled 4 s ahead, every 0.4 s (`Conductor`). When the page stalls (a new game's
  first frame can hold it for a second or two) and a note's time has passed, everything from that
  note on moves later by the time lost: nothing is skipped and nothing bunches up.
- It never throws into the game: every entry point catches, an error stops the music (a warning in
  the console), and a browser without Web Audio has no music (`music.available` false).
- Options → Sound: **Music** (on or off) and **Volume** (the slider's square is the gain, so its
  middle sounds about half as loud). Both are remembered by this browser, in localStorage
  `ffh.music` (`{ on, volume }`). Where the music cannot play (no Web Audio, or it failed) the
  controls are shown disabled with the line "Music is not available in this browser."

`main.js` calls `music.init()` once at boot; `tools.js` `openOptions()` has the Sound section.

## Adding a piece

A new piece is a new object in `music_score.js`, added to `PIECES`, in the format above, using the
orchestra's voices (`MIX` lists them). The player plays `PIECES.vigil`; which piece plays when is
the player's choice to add (`new Conductor(piece)` in `music.js` `play`).

`crates/ffh-play/assets/test/music.test.mjs` (run by `bash tools/web-check.sh`) then checks every
piece in `PIECES` for: every bar names a chord in its table; every chord's `harp` has eight notes;
`compile` for passes 0 and 1 gives events in order, finite, with `d > 0`, `0 < v <= 1.1` and `t`
inside the piece, the same each time, the two passes different; every line parses and ends inside
the piece; every pitch is in its voice's range (harp 24-103, flute 60-96, horn 41-77, violins
55-100, pad 48-84, bass 28-55, choir 40-81, timpani 38-57, celesta 60-108); the first and last
chords share the bass's pitch class (the loop comes back to its root); the tempo map's bars meet
and each slowing bar lasts between its two tempos' lengths. Vigil's length and its second
playing's horn theme, the conductor, the harp's string and the player's switch are checked once.

## Checking it

The music is checked by measurement, not by ear. `node tools/music-demo.mjs OUT.html` builds a
single self-contained page with Play, a volume slider and a Measure button: an offline render of a
playing (`renderPiece`) and its peak, its level per section and its octave bands.

`tools/web-smoke.mjs` checks the player in the real client: the music is playing after the first
click, Options switches it off (the slider disabled) and on again. Every browser tool starts
Chrome with `--mute-audio` (`tools/web-lib.mjs` `chromiumLaunch` and the smoke tests' own
launches), so a test run makes no sound; the page's audio context runs all the same.

### Measured

One playing rendered offline at 44.1 kHz in Chrome (2026-11-23: 180.7 s with the tail, rendered in
about 20 s). dBFS is the mean square against full scale; a peak of 1 is full scale.

| Section | RMS dBFS | Peak |
|---|---|---|
| The ground alone (bars 0-3) | -26.4 | 0.23 |
| The theme (4-11) | -21.3 | 0.38 |
| The theme, with the strings (12-19) | -18.5 | 0.52 |
| The answer (20-27) | -18.4 | 0.66 |
| The theme, everyone (28-35) | -16.3 | 0.64 |
| Coda (36-42) | -22.7 | 0.49 |
| The whole piece | -19.2 | 0.66 |

- No NaN and no sample above 0.8 (the soft ceiling is 0.89), in either orchestration; the second
  playing reads within 1.5 dB of the first in every section.
- At 48 kHz every section reads within 0.5 dB of the above.
- The hall is 5.7 dB under the dry sound over the piece.
- Octave bands from 45 Hz to 20 kHz, the whole piece, in dB: -29.8, -27.0, -24.9, -23.6, -23.9,
  -31.5, -38.2, -49.3, -69.4. The weight sits between 180 Hz and 1.4 kHz, with little above
  5.6 kHz: a dark mix.
- Pitch: the flute's notes read within 8 cents of the score (the width of its vibrato), the harp's
  strings within 0.1 cent.
- In the game, in Chrome: silent until the first click, playing after it, off and on from Options,
  no console message.

What the numbers do not say is whether it is beautiful. Nobody had heard the piece when it was
merged: its levels, pitches and balance are measured, its taste is not. Change the score and the
`MIX` by ear, then measure again.
