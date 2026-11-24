# The music

The browser client has several pieces of music; the player chooses one in Options, and "Vigil" is
the default. The page plays them from code: the score is data, the instruments are Web Audio nodes
and a little arithmetic, and the client ships no sound files. The chosen piece loops, with a silence
between playings where the piece has one (`gap`).

| Id | Title | Style | Key, meter, tempo | Length |
|---|---|---|---|---|
| `vigil` | Vigil | | D minor, four beats in a bar, 63 to the minute slowing to 40 at the end | 43 bars, about 2:54 (173.7 s); 8 s of silence between playings |
| `onslaught` | Onslaught | Battle | E minor, four beats in a bar, 152 to the minute | 40 bars, about 1:03 (63.2 s); no silence: the last bar turns back into the first |
| `harvest` | Harvest Round | Folk dance | D Dorian, a jig: six eighths in a bar, the eighth the beat, 348 to the minute, slowing to 300 at the end | 61 bars, about 1:03 (63.3 s); 3 s of silence |
| `umbra` | Umbra | Arcane | A minor slipping to minor chords a third away (F minor, C sharp minor), four beats in a bar, 60 to the minute, slowing to 52 at the end | 26 bars, about 1:45 (104.9 s); 6 s of silence |
| `banners` | Banners | 8-bit march | C major, four beats in a bar, 132 to the minute | 35 bars, about 1:04 (63.6 s); 2 s of silence |

Vigil, in outline: a harp alone states the ground (a bass that walks down from D to A); a flute
has the theme over it (bars 4-11); the strings and a choir join for its repeat (12-19); the horn
answers in F (20-27); a harp glissando and the timpani lead into the full orchestra's return of the
theme (28-35), which turns aside at the cadence (B flat for D minor); the coda walks B flat, C, D
up into D major (36-42) and ends on the major third and a silence.

Onslaught, in outline: the low strings gallop on the chord's root (an eighth and two sixteenths a
beat), alone, then the horn and the brass chords come in (bars 0-3); the brass has the theme (4-11); the high strings join for its
second time (12-19); a broader middle in G rides over the same gallop (20-27); four bars climb to
the dominant (28-31); the theme comes back with everyone (32-39).

Harvest Round, in outline: a whistle and a fiddle take turns and then play together over a lute, a
hand drum and a drone: the drum and the lute alone (bars 0-3), the first strain twice (4-19), the
second twice (20-35), a third over little but the drum (36-51), and the first again with everyone
(52-60).

Umbra, in outline: a bell and a drone (bars 0-1); a celesta turns a figure of three notes against
the bar's eight eighths, and a flute has a long note and its lower neighbour (2-9); the high
strings over a choir (10-17); the flute again, ending on the ninth and its resolution (18-25).

Banners, in outline: three console voices and a noise channel (a thin pulse for the tune, a square
running the chord in sixteenths, a triangle for the bass): a fanfare (bars 0-1), the tune twice
(2-17), a middle in A minor (18-25), the tune again and a close through A flat and B flat (26-34).

## The modules

All four are in `crates/ffh-play/assets/app/`, served with the rest of the client
(`src/transport/http.rs` `ASSETS`).

| Module | What it owns |
|---|---|
| `music_pieces.js` | The pieces besides Vigil, as data: `MORE_PIECES`, in the order Options lists them. |
| `music_score.js` | The pieces as data (`VIGIL`, and `PIECES`: `{ vigil, ...MORE_PIECES by id }`, in listing order) and `compile(piece, pass)`: the timed notes of one playing, `{ events, seconds }`, each event `{ voice, t, d, m, v, v2 }` (start and length in seconds, MIDI pitch, velocity at the note's start and end), in order of `t`. Also `tempoMap`, `levelAt`, `parseLine`, `midi`, `MARKS` and `mulberry32` (the seeded generator behind the small unevenness of the playing). Pure: no Web Audio, it runs in node. |
| `music_synth.js` | The orchestra: `createOrchestra(ctx, opts)` builds every voice on an `AudioContext` (the page) or an `OfflineAudioContext` (a measurement) and returns `{ play(event), out, close() }`. `MIX` is each voice's level, pan and hall send; `mixFor(piece)` is `MIX` with the piece's own `mix` laid over it; `settingsFor(piece)` is what `createOrchestra` takes for a piece (its mix, and its `trim` and `hall` where it has them). `RANGES` is the notes each pitched voice is written for, lowest and highest (MIDI). `pluckSamples(sampleRate, midi)`, the harp's string, is pure and runs in node. |
| `music.js` | The player. `Conductor` hands a looping piece's notes out in windows of the context's clock; `renderPiece(piece, opts)` renders one playing offline (what the measurements read); `music` is the page's switch: `init()`, `available`, `on`, `volume`, `piece` (the chosen piece's id), `pieces` (`[{ id, title, style }]` in `PIECES` order), `playing`, `setOn(bool)`, `setPiece(id)`, `setVolume(0..1)`. |

## The score format

A piece is an object (see `VIGIL` in `music_score.js`, the others in `music_pieces.js`):

- `id`, `title`, `seed` (the humanizing's seed), `gap` (seconds of silence between playings).
- `style`: a short label; `sections`: `[[bar, name], ...]` for the listening page; `mix`: per-voice
  overrides of `MIX` (`{ voice: { level, pan, hall } }`, any subset); `trim`, `hall`: the piece's
  own level before the soft ceiling and hall level.
- `beats`: beats in a bar (4 when left out); the tempo counts those beats (a 6/8 jig is `beats: 6`
  with the eighth as the beat).
- `bars`: `[chord, bpm]` per bar. Bars are counted from 0 everywhere in a piece
  (`layers`, `patterns`, `dynamics`, `lines`, `extras`; the bar numbers in this document too). `bpm` a number is a steady tempo; a pair
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
  `layers.harp` is optional (Vigil's harp figure); `pad`, `bass`, `choir` as before.
- `patterns`: repeating figures (an ostinato, a strum, a drum):
  `{ voice, bars: [[from, to], ...], figure, of, step, len, level, oct, every, roll, human }`.
  `figure` is one bar of tokens `step` beats apart (0.5 when left out), repeated to fill the bar; a
  list of figures takes turns bar by bar. Tokens: a number is the index of a note in the bar's
  chord field named by `of` (a two-chord bar's field has two halves separated by `|`, and the token
  takes the half it falls in); `*` is all that field's notes together (`roll`: seconds between
  them); a note name (`D4`) is that note whatever the chord (drums); `-` a rest; `|` is ignored
  (for the eye). After a token: `'` an octave up, `,` an octave down, `!` stressed (x1.3), `?`
  ghosted (x0.6). `len`: beats each note sounds (`step` when left out); `level`: the key in
  `dynamics` (the voice's name when left out; a `dynamics` entry may be a plain number); `oct`:
  octaves to move the whole figure; `every: n`: only every n-th bar of a span; `human`: seconds of
  looseness in the timing (and a little in the loudness).
- `dynamics`: per layer, `[bar, velocity]` points; the level between two points is the straight
  line through them (`levelAt`).
- `lines`: the melodic lines, `{ voice, bar, notes, oct, alt }`. `bar` is where the line starts.
  `notes` is read by `parseLine`:
  - `C5:1.5` a note and its length in beats (one when left out); `Bb3`, `F#5` with accidentals;
  - `-` a rest (`-:.5` half a beat);
  - `<mf>` the dynamic from there on (`pp p mp mf f ff`, `MARKS`);
  - a trailing `^` a note that swells, `_` one that fades; after a note, `'` cuts it short (half
    its length) and `!` stresses it;
  - `|` a bar line, checked against the piece's `beats`: a line whose bar does not add up to them
    is an error, not a late entrance.

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
| `spiccato` | The bowed strings' waves, two players, no vibrato, a 12 ms attack and a quick release, the note already sunk to 0.4 of its level by its end: short strokes off the string. |
| `marcato` | The bowed strings' waves, three players, a light vibrato, a 30 ms attack and a short release, a little fall over the note: marked strokes. |
| `fiddle` | The bowed strings' waves, one player (two oscillators 5 cents apart), a full vibrato, brighter and centred. |
| `brass` | The horn's method (two detuned players, a lowpass that opens as the note speaks) with a brighter wave, a wider opening and a faster attack: trumpets and trombones. |
| `lute` | The harp's plucked string (`pluckSamples`), under a brighter lowpass, damped by the hand once the note's length is over. |
| `drum` | A hand drum: decaying sine partials whose pitch bends down into the note, and a short burst of lowpassed noise for the hand. |
| `snare` | A short pitched shell (fixed pitch) and two bursts of noise, bandpassed and highpassed, for the wires; the written note is ignored. |
| `shaker` | A very short burst of noise through a high bandpass; the note is ignored. |
| `cymbal` | Two bursts of noise, highpassed and through a high bandpass, dying over a few tenths of a second; the note is ignored. |
| `bell` | Decaying sine partials of a cast bell: the hum an octave under the note, a minor third, the fifth, the octave and the upper partials, the low ones ringing longest. |
| `chipLead` | A pulse a quarter of the period wide (a band-limited wave), up at once, sinking to 0.7 of its level and stopped dead; a long note wavers. |
| `chipHarm` | A square wave made the same way, sinking to half its level. |
| `chipBass` | A triangle wave made the same way, held at its level. |
| `chipNoise` | The console's drums, by the note: below C3 a kick (a sine that drops from 150 to 42 Hz), up to B4 a snare (bandpassed noise), from C5 a hat (highpassed noise). |
| the hall | A `ConvolverNode` with a computed impulse response: early reflections, then noise that dies away, the highs first (3.6 s in the bass, 1.3 s at the top), different in each ear. |
| the output | The mix through a `tanh` soft ceiling (a `WaveShaperNode`) in place of clipping, then the volume. |

## The player

- It starts on the player's first key or click (`mousedown`, `pointerup`, `touchend`, `keydown`):
  browsers let a page make sound only after one. It fades in over 3 s.
- It loops: a playing, the piece's silence (`gap`: 8 s for Vigil, none for Onslaught), the next.
  In Vigil, every second playing hands the first theme (bars 4-11) to the horn, an octave down, and
  the horn's answer (bars 20-23) to the flute, an octave up (the lines' `alt`).
- It suspends while the tab is hidden and goes on where it was when the tab comes back.
- Notes are scheduled 4 s ahead, every 0.4 s (`Conductor`). When the page stalls (a new game's
  first frame can hold it for a second or two) and a note's time has passed, everything from that
  note on moves later by the time lost: nothing is skipped and nothing bunches up.
- It never throws into the game: every entry point catches, an error stops the music (a warning in
  the console), and a browser without Web Audio has no music (`music.available` false).
- Options → Sound: **Music** (on or off), **Piece** (a select of `music.pieces` by title,
  `#opt-music-piece`) and **Volume** (the slider's square is the gain, so its middle sounds about
  half as loud). All three are remembered by this browser, in localStorage `ffh.music`
  (`{ on, volume, piece }`; `piece` is `"vigil"` when none was chosen). A remembered piece that is
  not in `PIECES` (a piece removed or renamed) is read as `"vigil"`, so that browser still has
  music. Where the music cannot play (no Web Audio, or it failed) the controls are shown disabled
  with the line "Music is not available in this browser."
- Choosing a piece (`music.setPiece(id)`; an unknown id is ignored) while the music sounds: the
  old piece's notes are already handed to its orchestra up to 4 s ahead, so the whole orchestra
  is let go, not only the `Conductor`. Its volume fades to 0 over 0.3 s, then its `close()`
  disconnects it and stops its shared sources, and a new orchestra (`settingsFor(piece)`) and
  `Conductor` start from silence and fade in over 0.6 s. Nothing is scheduled and the volume and
  switch do not fade during those 0.3 s; the new piece comes in with the switch and volume as
  they are then. A second choice during the fade swaps to the latest. When the music is off,
  resting or the tab hidden, the swap is made at once without a fade, and the next start plays the
  new piece. Before the first click there is no orchestra: the first start plays the chosen piece.

`main.js` calls `music.init()` once at boot; `tools.js` `openOptions()` has the Sound section.

## Adding a piece

A new piece is a new object in `music_pieces.js`, added to `MORE_PIECES` (its place there is its
place in the Options list), in the format above, using the orchestra's voices (`MIX` lists them).
`PIECES` picks it up by its `id`, and Options offers it by its `title`.

`node tools/music-lint.mjs [id]` reads a piece (every piece when no id is given) for notes outside
their voice's range (`RANGES`), clashes (two notes a semitone apart sounding together for 0.4 s or
more), a line's strong-beat notes outside the bar's chord, and low thirds (two notes a third or
less apart, both under C3). It reports; it does not fail.

`crates/ffh-play/assets/test/music.test.mjs` (run by `bash tools/web-check.sh`) then checks every
piece in `PIECES` for: every bar names a chord in its table; where the piece has a harp layer, the
chords of its bars have the eight notes (or the sweep) the harp needs; `compile` for passes 0 and
1 gives events in order, finite, with `d > 0`, `0 < v <= 1.1` and `t` inside the piece, the same
each time, and the two passes differ exactly when the piece has something that changes between
playings (a line with `alt`, a pattern with `human`, a harp layer); every line parses with the
piece's `beats` and ends inside the piece; every voice is one the orchestra has (`MIX`) and every
pitch is in its voice's range (`RANGES`; a voice not listed there has no pitch); the first and
last chords share the bass's pitch class where the chords have a `bass` (the loop comes back to
its root); the tempo map's bars meet and each slowing bar lasts between its two tempos' lengths;
`sections` start at bar 0, ascend and lie inside the piece; `mix` names voices of `MIX`. The
format itself (lines, meters, patterns, `mixFor`, `settingsFor`) is tested on small pieces built in
the test; Vigil's length and its second playing's horn theme, the conductor, the harp's string and
the player's switch and piece choice are checked once.

## Checking it

The music is checked by measurement, not by ear. `node tools/music-demo.mjs OUT.html` builds a
single self-contained page (it opens from disk) with a piece menu, Play, a volume slider and a
Measure button. Measure renders the chosen piece offline (`renderPiece`) and shows its peak, its
NaN count, its samples above 0.8, its level per section (`sections`) and its octave bands; it
renders the piece once more without the hall (`hall: 0`) and gives the hall's level against the dry
sound (the hall alone is the full render less the dry one, sample by sample); with "per voice" it
renders each voice the piece uses alone. A script driving a browser reads the same numbers from
`window.musicMeasure({ piece, perVoice })`.

`node tools/music-lint.mjs` reads the notes themselves (above): run it on a new piece before
rendering it.

`tools/web-smoke.mjs` checks the player in the real client: the music is playing after the first
click, Options switches it off (the slider disabled) and on again, then chooses the second piece in
the list and Vigil again, the music playing after each. Every browser tool starts
Chrome with `--mute-audio` (`tools/web-lib.mjs` `chromiumLaunch` and the smoke tests' own
launches), so a test run makes no sound; the page's audio context runs all the same.

### Measured

Every piece, one playing rendered offline at 44.1 kHz in Chrome (2026-11-24). dBFS is the mean
square against full scale over the piece; a peak of 1 is full scale; "hall" is the hall's level
against the dry sound; "speed" is the piece's length over the time the render took, on one laptop
(a measure of how much the page has to compute).

| Piece | Whole, dBFS | Peak | Quietest and loudest four bars, dBFS | Hall, dB | Notes | Speed |
|---|---|---|---|---|---|---|
| Vigil | -19.2 | 0.66 | -26.4, -16.3 | -5.7 | 864 | 15x |
| Onslaught | -18.9 | 0.74 | -23.4, -16.2 | -10.6 | 1709 | 10x |
| Harvest Round | -20.3 | 0.56 | -24.8, -17.5 | -12.9 | 1250 | 16x |
| Umbra | -21.2 | 0.46 | -23.5, -19.8 | -4.5 | 309 | 16x |
| Banners | -20.8 | 0.53 | -21.2, -20.2 | -39.2 | 1343 | 25x |

- No NaN and no sample above 0.8 in any piece, in the first playing or the second.
- At 48 kHz the four new pieces read within 0.3 dB of the above, with no NaN and no sample above
  0.8 (peaks 0.51 to 0.72).
- Two playings back to back on one orchestra, handed out by the `Conductor`: each reads as it does
  alone (within 0.2 dB), the silence between them is silent (-67 dB or less) where the piece has
  one, and Onslaught, which has none, runs through its seam with no NaN and a peak of 0.74.
- Voice by voice (each rendered alone, dB over the whole piece): Onslaught's brass -23.5, spiccato
  -26.8, marcato -27.9, bass -28.7, horn -28.6, timpani -29.1, pad -33.1, violins -37.6, snare
  -36.7, cymbal -46.0; Harvest Round's whistle -24.6, lute -26.2, fiddle -28.7, drum -30.5, bass
  -33.0, shaker -46.0; Umbra's flute -25.2, bass -29.1, celesta -30.7, choir -31.1, pad -31.2,
  violins -33.9, harp -35.5, bell -35.7; Banners' bass -25.0, tune -25.3, square -28.3, noise
  -30.6. A drum's figure is low because it sounds for a moment at a time: its peaks are 0.2 to 0.3.
- `music-lint` on the four new pieces: no note out of range, no low thirds, no clash in Onslaught,
  Harvest Round or Banners; Umbra's 31 are its half-second lower neighbours and anticipations and
  the ninth it ends on, all written. The strong-beat notes outside the chord (12 in Onslaught, 3
  in Banners) are half-beat appoggiaturas.
- Vigil reads as it did before the other pieces and their instruments were added: its compiled
  notes are identical, and so are the numbers below.

Vigil alone, in more detail (2026-11-23: 180.7 s with the tail, rendered in about 20 s):

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
- Two playings back to back on one orchestra, handed out by the `Conductor` (364 s offline): each
  reads as it does alone (within 0.1 dB in every section), the silence between them is silent
  (-143 dB), no NaN, peak 0.69. Hours of looping on one orchestra were not measured.
- In the game, in Chrome, the gain of the node that feeds the speakers read back: no audio context
  before the first click; 0.19 one second after it and 0.64 (the default volume 0.8, squared) once
  the fade is over; 0 and a suspended context with Music unticked, 0.64 again when ticked; 0.16
  with the slider at 40, also when it is moved during the fade in; suspended, its clock stopped,
  while the page is hidden (the page was told so: headless Chrome hides none) and running again
  when it is shown. No console message.

What the numbers do not say is whether any of it is beautiful. Nobody had heard a piece when it
was merged: levels, pitches and balance are measured, taste is not. Change the scores and the
`MIX` by ear, then measure again.
