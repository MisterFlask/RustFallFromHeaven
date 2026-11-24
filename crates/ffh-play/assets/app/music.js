// The game's music (docs/MUSIC.md): played by the page from the score (music_score.js) on the
// orchestra (music_synth.js), no sound files. It starts with the player's first key or click (a
// browser lets a page make sound only then), fades in, and loops with a silence between playings.
// Options has its switch and its volume (tools.js); the choice is remembered (localStorage).
//
// Nothing here may break the game: every entry point catches, and a browser without Web Audio
// simply has no music (`music.available` is false).

import { PIECES, compile } from "./music_score.js";
import { createOrchestra } from "./music_synth.js";

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
 *  `only`: the voices to keep; `mix`: other levels than the orchestra's own. */
export async function renderPiece(piece, { sampleRate = 44100, pass = 0, tail = 7, only = null, mix } = {}) {
  const score = compile(piece, pass);
  const total = score.seconds + tail;
  const ctx = new OfflineAudioContext(2, Math.ceil(total * sampleRate), sampleRate);
  const orchestra = createOrchestra(ctx, mix ? { mix } : {});
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

const state = {
  on: true,
  volume: 0.8,
  ctx: null,
  orchestra: null,
  conductor: null,
  timer: null,
  /** A timer that suspends the context once the fade out is over. */
  resting: null,
  inited: false,
  error: null,
};

function loadPrefs() {
  try {
    const p = JSON.parse(globalThis.localStorage?.getItem(PREFS_KEY) ?? "null");
    if (p && typeof p.on === "boolean") state.on = p.on;
    if (p && Number.isFinite(p.volume)) state.volume = Math.min(1, Math.max(0, p.volume));
  } catch {
    /* no storage, or not ours: the defaults */
  }
}

function savePrefs() {
  try {
    globalThis.localStorage?.setItem(PREFS_KEY, JSON.stringify({ on: state.on, volume: state.volume }));
  } catch {
    /* no storage */
  }
}

/** The volume as a gain: the slider's middle should sound like half. */
const gainOf = (volume) => volume * volume;

function tick() {
  try {
    const { ctx, orchestra, conductor } = state;
    if (!ctx || ctx.state !== "running") return;
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
  try {
    state.ctx?.close().catch(quiet);
  } catch {
    /* already closed */
  }
  state.ctx = null;
}

/** Moves the volume to `level` over `seconds`, from wherever a fade under way has brought it. */
function fadeTo(level, seconds) {
  const g = state.orchestra.out.gain;
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
    state.orchestra = createOrchestra(state.ctx);
    state.conductor = new Conductor(PIECES.vigil);
    state.orchestra.out.gain.value = 0;
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

/** The music's switch and volume (Options). */
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
