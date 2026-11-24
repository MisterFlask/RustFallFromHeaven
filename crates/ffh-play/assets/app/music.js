// The game's music (docs/MUSIC.md): played by the page from the score (music_score.js) on the
// orchestra (music_synth.js), no sound files. It starts with the player's first key or click (a
// browser lets a page make sound only then), fades in, and loops with a silence between playings.
// Options has its switch, the piece and the volume (tools.js); the choice is remembered (localStorage).
//
// Nothing here may break the game: every entry point catches, and a browser without Web Audio
// simply has no music (`music.available` is false).

import { PIECES, compile } from "./music_score.js";
import { createOrchestra, settingsFor } from "./music_synth.js";

const PREFS_KEY = "ffh.music";
/** How far ahead of the clock notes are handed to the orchestra, and how often. The map can hold
 *  the page for a second or two (a new game's first frame): the music must already be scheduled. */
const AHEAD = 4;
const TICK_MS = 400;
/** Seconds: the fade in at the start, the fade when the music is switched off or on. */
const FADE_IN = 3;
const FADE = 0.6;

/** Hands out a looping piece's notes in windows of the context's time: `take(now, until)` gives,
 *  in order, the notes that start before `until`, with `t` in the context's seconds. A playing is
 *  followed by `gap` seconds of silence and the next one (`pass` counts them: music_score.js
 *  `compile`). When the page stalled and a note's time has passed, the music does not skip or
 *  bunch up: everything from that note on moves later by the time lost. */
export class Conductor {
  constructor(piece, { gap = piece.gap ?? 8, lead = 0.25 } = {}) {
    this.piece = piece;
    this.gap = gap;
    this.lead = lead;
    this.pass = 0;
    this.origin = null;
    this.score = null;
    this.next = 0;
  }

  take(now, until) {
    if (this.origin === null) {
      this.origin = now + this.lead;
      this.score = compile(this.piece, this.pass);
    }
    const out = [];
    for (;;) {
      if (this.next >= this.score.events.length) {
        if (!this.score.events.length) break;
        this.origin += this.score.seconds + this.gap;
        this.pass += 1;
        this.score = compile(this.piece, this.pass);
        this.next = 0;
        continue;
      }
      const e = this.score.events[this.next];
      if (this.origin + e.t >= until) break;
      if (this.origin + e.t < now + 0.02) this.origin += now + this.lead - (this.origin + e.t);
      out.push({ ...e, t: this.origin + e.t });
      this.next += 1;
    }
    return out;
  }
}

/** One playing of a piece rendered to a buffer (an OfflineAudioContext: faster than real time and
 *  the same nodes as the page's): what the measurements read (tools/music-demo.mjs, docs/MUSIC.md).
 *  `only`: the voices to keep; `mix`: other levels than the piece's own; `hall`: another level for
 *  the hall (0: the dry sound alone). */
export async function renderPiece(piece, { sampleRate = 44100, pass = 0, tail = 7, only = null, mix, hall } = {}) {
  const score = compile(piece, pass);
  const total = score.seconds + tail;
  const ctx = new OfflineAudioContext(2, Math.ceil(total * sampleRate), sampleRate);
  const orchestra = createOrchestra(ctx, { ...settingsFor(piece), ...(mix ? { mix } : {}), ...(hall !== undefined ? { hall } : {}) });
  const events = score.events.filter((e) => !only || only.includes(e.voice));
  const lead = 0.1;
  const step = 2;
  let next = 0;
  // Notes are handed over a window at a time, as the page does it, so the graph stays small.
  const fill = (until) => {
    for (; next < events.length && events[next].t + lead < until; next++) orchestra.play({ ...events[next], t: events[next].t + lead });
  };
  fill(step + 1);
  for (let t = step; t < total; t += step) {
    ctx.suspend(t).then(() => {
      fill(t + step + 1);
      ctx.resume();
    });
  }
  return ctx.startRendering();
}

// ---------------------------------------------------------------------------------------------
// The player

const AudioCtx = globalThis.AudioContext ?? globalThis.webkitAudioContext ?? null;

/** The piece played when none was chosen, or the one chosen is gone. */
const DEFAULT_PIECE = "vigil";
/** Seconds: the fade out of the old piece when another is chosen. */
const SWAP_FADE = 0.3;

const state = {
  on: true,
  volume: 0.8,
  /** The chosen piece's id: a key of `PIECES`. */
  piece: DEFAULT_PIECE,
  ctx: null,
  orchestra: null,
  conductor: null,
  timer: null,
  /** A timer that suspends the context once the fade out is over. */
  resting: null,
  /** A timer that lets the old orchestra go and starts the chosen piece once the old one has
   *  faded (`setPiece`). While it runs nothing is scheduled and nothing fades. */
  swap: null,
  inited: false,
  error: null,
};

const isPiece = (id) => typeof id === "string" && Object.prototype.hasOwnProperty.call(PIECES, id);

function loadPrefs() {
  try {
    const p = JSON.parse(globalThis.localStorage?.getItem(PREFS_KEY) ?? "null");
    if (p && typeof p.on === "boolean") state.on = p.on;
    if (p && Number.isFinite(p.volume)) state.volume = Math.min(1, Math.max(0, p.volume));
    // A piece that is no longer there (or never was): the default, else no music on every load.
    if (p && isPiece(p.piece)) state.piece = p.piece;
  } catch {
    /* no storage, or not ours: the defaults */
  }
}

function savePrefs() {
  try {
    globalThis.localStorage?.setItem(PREFS_KEY, JSON.stringify({ on: state.on, volume: state.volume, piece: state.piece }));
  } catch {
    /* no storage */
  }
}

/** A new orchestra and conductor for the chosen piece on the context, silent until faded in. */
function startPiece() {
  const piece = PIECES[state.piece] ?? PIECES[DEFAULT_PIECE];
  state.orchestra = createOrchestra(state.ctx, settingsFor(piece));
  state.conductor = new Conductor(piece);
  state.orchestra.out.gain.value = 0;
}

/** Lets the old orchestra go (its notes scheduled ahead go with it) and starts the chosen piece;
 *  it fades in if the music is on and sounding, else the next `play` brings it in. */
function swapNow() {
  try {
    state.swap = null;
    const old = state.orchestra;
    state.orchestra = null;
    state.conductor = null;
    try {
      old?.close();
    } catch {
      /* its context is gone */
    }
    if (!state.ctx) return;
    startPiece();
    if (state.on && state.ctx.state === "running" && !state.resting && !globalThis.document?.hidden) {
      fadeTo(gainOf(state.volume), FADE);
      tick();
    }
  } catch (e) {
    fail(e);
  }
}

/** The volume as a gain: the slider's middle should sound like half. */
const gainOf = (volume) => volume * volume;

function tick() {
  try {
    const { ctx, orchestra, conductor } = state;
    if (!ctx || ctx.state !== "running" || state.swap || !conductor) return;
    const now = ctx.currentTime;
    for (const e of conductor.take(now, now + AHEAD)) orchestra.play(e);
  } catch (e) {
    fail(e);
  }
}

/** A context's suspend, resume and close answer with promises: one refused is no one's error. */
const quiet = () => {};

function fail(e) {
  state.error = e;
  console.warn("music: stopped:", e);
  clearInterval(state.timer);
  state.timer = null;
  clearTimeout(state.swap);
  state.swap = null;
  try {
    state.ctx?.close().catch(quiet);
  } catch {
    /* already closed */
  }
  state.ctx = null;
}

/** Moves the volume to `level` over `seconds`, from wherever a fade under way has brought it.
 *  While a piece is being swapped the old one keeps fading out: the swap fades the new one in. */
function fadeTo(level, seconds) {
  if (state.swap || !state.orchestra) return;
  ramp(state.orchestra.out.gain, level, seconds);
}

function ramp(g, level, seconds) {
  const now = state.ctx.currentTime;
  if (g.cancelAndHoldAtTime) {
    g.cancelAndHoldAtTime(now);
  } else {
    g.cancelScheduledValues(now);
    g.setValueAtTime(g.value, now);
  }
  g.linearRampToValueAtTime(level, now + seconds);
}

/** Starts or resumes the music. Only from a key or a click: the browser refuses sound before. */
function play() {
  if (!AudioCtx || state.error) return;
  clearTimeout(state.resting);
  state.resting = null;
  if (!state.ctx) {
    state.ctx = new AudioCtx({ latencyHint: "playback" });
    startPiece();
    fadeTo(gainOf(state.volume), FADE_IN);
  } else {
    fadeTo(gainOf(state.volume), FADE);
  }
  state.ctx.resume().catch(quiet);
  if (!state.timer) state.timer = setInterval(tick, TICK_MS);
  tick();
}

/** Fades out, then rests the context: its clock stops, so the music picks up where it was. */
function rest() {
  if (!state.ctx) return;
  fadeTo(0, FADE);
  clearTimeout(state.resting);
  state.resting = setTimeout(() => {
    state.resting = null;
    clearInterval(state.timer);
    state.timer = null;
    state.ctx?.suspend().catch(quiet);
  }, FADE * 1000 + 100);
}

/** Whether the page has had the key or click the browser asks for before sound. */
function allowed() {
  return globalThis.navigator?.userActivation?.hasBeenActive ?? true;
}

function onGesture() {
  try {
    if (!state.on || !allowed() || globalThis.document?.hidden) return;
    if (state.ctx && state.ctx.state === "running" && state.timer) return;
    play();
  } catch (e) {
    fail(e);
  }
}

function onVisibility() {
  try {
    if (!state.ctx) return;
    if (globalThis.document.hidden) {
      clearInterval(state.timer);
      state.timer = null;
      state.ctx.suspend().catch(quiet);
    } else if (state.on) {
      play();
    }
  } catch (e) {
    fail(e);
  }
}

/** The music's switch, piece and volume (Options). */
export const music = {
  /** Whether the music can play here: the browser has Web Audio and nothing has failed. */
  get available() {
    return !!AudioCtx && !state.error;
  },
  /** The remembered choice: music on. */
  get on() {
    return state.on;
  },
  /** The remembered volume, 0 to 1. */
  get volume() {
    return state.volume;
  },
  /** Whether it is sounding now. */
  get playing() {
    return !!state.ctx && state.ctx.state === "running" && !state.resting;
  },

  /** Reads the remembered choice and waits for the first key or click (main.js, once). */
  init() {
    if (state.inited) return;
    state.inited = true;
    loadPrefs();
    if (!AudioCtx || !globalThis.addEventListener) return;
    // The events that count as the player's own act for the browser.
    for (const type of ["mousedown", "pointerup", "touchend", "keydown"]) globalThis.addEventListener(type, onGesture, { capture: true, passive: true });
    globalThis.document?.addEventListener("visibilitychange", onVisibility);
  },

  /** Switches the music on or off (from a click, so it may start at once) and remembers it. */
  setOn(on) {
    state.on = !!on;
    savePrefs();
    try {
      if (state.on) play();
      else rest();
    } catch (e) {
      fail(e);
    }
  },

  /** The chosen piece's id (a key of `PIECES`). */
  get piece() {
    return state.piece;
  },
  /** The pieces to choose from, in `PIECES` order: `[{ id, title, style }]`. */
  get pieces() {
    return Object.values(PIECES).map((p) => ({ id: p.id, title: p.title, style: p.style ?? "" }));
  },

  /** Chooses the piece (an unknown id is ignored) and remembers it. If the music is sounding, the
   *  old piece fades out quickly and its orchestra is let go (its notes are scheduled 4 s ahead),
   *  then the new one fades in; if it is off or resting, the swap is made at once and the next
   *  `play` starts the new piece. A second choice during the fade swaps to the latest. */
  setPiece(id) {
    if (!isPiece(id)) return;
    if (id === state.piece && !state.swap) return;
    state.piece = id;
    savePrefs();
    try {
      if (!state.ctx) return;
      if (state.swap) {
        clearTimeout(state.swap);
        state.swap = setTimeout(swapNow, SWAP_FADE * 1000);
        return;
      }
      if (state.on && state.ctx.state === "running" && !state.resting && state.orchestra) {
        ramp(state.orchestra.out.gain, 0, SWAP_FADE);
        state.swap = setTimeout(swapNow, SWAP_FADE * 1000);
      } else {
        swapNow();
      }
    } catch (e) {
      fail(e);
    }
  },

  /** Sets the volume, 0 to 1, and remembers it. */
  setVolume(volume) {
    state.volume = Math.min(1, Math.max(0, Number(volume) || 0));
    savePrefs();
    try {
      if (state.ctx && state.on && !state.resting) fadeTo(gainOf(state.volume), 0.1);
    } catch (e) {
      fail(e);
    }
  },
};
