// Watching a battle (the formation-battle prototype's replay, phase 6 revision 4): the log played
// on the plan's battlefield one action at a time, staged by battle_snap.js (the owner's pick of
// the battle-feel spike: each action a snap to its contact frame, a held beat, an ease back; the
// shade on crits and kills). An action's event is folded into the state on its contact frame (the
// log gets its line then, under round and pass headers, and the HP bars drop); its deaths and
// escapes show when the pair is back on its cells (the fallen die where they stand, those who
// escape a killing blow leave the field). Play, step, skip to the report, speeds 1x to 8x.
//
// Time: the replay's clock runs at the speed (ms at 1x) and stops while the replay is paused in
// the middle of an action (the action freezes there); between actions it runs on, so a death
// started before a pause plays out. The breathing and the idle strips run on the page's clock.

import { h } from "./dom.js";
import { initialState, applyEvent, logLines, viewerSide, resultWords, fates, headOf, uname } from "./battle_log.js";
import { loadArt, unitXY, cellXY, pic, deadPic, faceFlip, animated, figOf, pixGlyph, GLYPH, btn, add, partsEl } from "./battle_stage.js";
import { Snap, breath, LEAD } from "./battle_snap.js";

export const NAME = "watch";

const SPEEDS = [1, 2, 4, 8];

const W = { ctx: null, log: null, F: null, me: 0, lines: [], st: null, disp: null, vis: [], fx: [], pend: [], cur: null, i: 0, clock: 0, wall: 0, snap: null, playing: true, stepping: false, speed: 2, done: false, doneAt: 0, autoReport: false, ready: false, anim: null, logEl: null, shown: 0 };

export async function enter(ctx, log) {
  W.ctx = ctx;
  W.ready = false;
  W.log = log;
  W.F = log.fighters;
  W.me = viewerSide(log);
  ctx.stage.me = W.me;
  await loadArt({ fighters: log.fighters, plot: log.plot, from: log.from, sides: log.sides });
  if (W.log !== log) return;
  ctx.stage.setBoard(log);
  W.anim = animated(log.sides);
  W.lines = logLines(log);
  // 1x by default: the snap's beats are timed at 1x (the spike the owner chose played at 1x)
  W.speed = SPEEDS.includes(ctx.prefs.get("speed", 1)) ? ctx.prefs.get("speed", 1) : 1;
  W.snap = new Snap(snapEnv(ctx, log));
  W.clock = 0;
  W.wall = performance.now();
  reset(0);
  W.playing = true;
  renderUI(ctx);
  W.ready = true;
}

export function leave() {
  W.ready = false;
  W.cur = null;
  W.fx = [];
  W.pend = [];
  W.snap?.reset();
}

/** The battle as the snap sees it (battle_snap.js, `Snap`). */
function snapEnv(ctx, log) {
  const sides = log.sides;
  return {
    feet,
    onField: (b) => !!W.st.pos[b],
    large: (b) => !!W.F[b].large,
    side: (b) => W.F[b].side,
    pic: (b, anim, ms, loop) => pic(W.F[b], sides, anim, ms, loop),
    animated: (b) => W.anim(W.F[b]),
    flip: (b) => faceFlip(W.F[b], sides),
    leftArt: (b) => figOf(W.F[b], sides).left,
    bar: (b) => (W.disp.status[b] === "on" ? { hp: W.disp.hp[b], max: W.F[b].max_hp * 100 } : null),
    reduced: () => ctx.reduced(),
  };
}

/** Back to the state after the first `n` events, at once (no motion). */
function reset(n) {
  W.st = initialState(W.log);
  for (let k = 0; k < n; k++) applyEvent(W.st, W.log.events[k], W.F);
  W.i = n;
  W.disp = { hp: W.st.hp.slice(), status: W.st.status.slice() };
  const long = W.clock - 5000;
  W.vis = W.F.map((_, b) => (W.st.status[b] === "dead" ? { deathAt: long } : W.st.status[b] === "gone" ? { goneAt: long } : {}));
  W.fx = [];
  W.pend = [];
  W.cur = null;
  W.stepping = false;
  W.snap?.reset();
  W.done = n >= W.log.events.length;
  W.doneAt = W.done ? W.wall - 5000 : 0;
  W.autoReport = false;
  W.shown = 0;
}

/** ms of replay time `ms` as the deaths, escapes and floating words play it: at the replay's
 *  speed up to 2x, no faster (at 4x and 8x they would not read). */
const shown = (ms) => (ms * Math.min(W.speed, 2)) / W.speed;

// ---------------------------------------------------------------------------------------------
// The timeline: each action is staged (battle_snap.js), lands on its contact frame (the event is
// folded in then), shows its fates when the pair is back, and rests

/** Fighter `b`'s feet now (its cell after the knock-backs so far), or, for one not in the line
 *  (a defensive strike from the reserve), behind its side's rear row. */
function feet(b) {
  const f = W.F[b];
  const pos = W.st.pos[b];
  const G = W.ctx.stage.G;
  if (pos) return unitXY(G, f, pos);
  const lanes = G.lanes[f.side];
  return cellXY(G, f.side, (lanes - 1) / 2, 2.6);
}

/** Starts the next action at replay time `now`. */
function beginEvent(now) {
  const e = W.log.events[W.i];
  if (!e) return finish(W.wall, true);
  const P = W.snap.begin(e, now, W.i);
  W.cur = { e, impact: now + P.impact, fates: now + (P.snap ? P.fates : P.impact), end: now + P.end, applied: false };
}

/** A word floating up from fighter `b` (an action with no blow: afraid, no target). */
function float(b, text, kind, now) {
  const [x, y] = feet(b);
  W.fx.push({ type: "float", x: x + ((b * 7) % 11) - 5, y: y - 70, text, kind, at: now, dur: 1000 });
}

/** The contact frame: the event folded into the state, the HP shown, the deaths and escapes held
 *  until the pair is back (`W.cur.fates`); the snap draws the numbers. */
function applyImpact(now) {
  const e = W.cur.e;
  const knockFrom = e.knocked && W.st.pos[e.knocked.unit] ? { ...W.st.pos[e.knocked.unit] } : null;
  applyEvent(W.st, e, W.F);
  W.cur.applied = true;
  W.i++;
  if (e.kind === "Afraid") {
    W.disp.status[e.target] = W.st.status[e.target];
    float(e.target, "afraid", "miss", now);
  }
  for (const x of e.hits ?? []) {
    if (x.miss) continue;
    if (x.withdrew) {
      W.pend.push({ b: x.target, status: "gone", at: W.cur.fates });
      continue;
    }
    W.disp.hp[x.target] = W.st.hp[x.target];
    if (x.killed) W.pend.push({ b: x.target, status: "dead", at: W.cur.fates });
  }
  if (e.heal) W.disp.hp[e.heal.target] = W.st.hp[e.heal.target];
  // the snap eases a knocked unit it lifted back during the return; one it did not, eases here
  if (e.knocked && knockFrom) W.vis[e.knocked.unit].knock = { from: knockFrom, at: now };
  if (e.kind !== "Afraid" && e.kind !== "Cast" && !e.hits?.length && !e.heal) float(e.actor, e.note === "NoOneHurt" || e.kind === "Heal" ? "no one to heal" : "no target", "miss", now);
  settle(now);
  syncLog();
  syncTop();
}

/** Shows the deaths and escapes that are due at `now`. */
function settle(now) {
  if (!W.pend.length) return;
  const n = W.pend.length;
  W.pend = W.pend.filter((p) => {
    if (p.at > now) return true;
    W.disp.status[p.b] = p.status;
    W.vis[p.b][p.status === "dead" ? "deathAt" : "goneAt"] = p.at;
    return false;
  });
  if (W.pend.length !== n) syncTop();
}

/** The last event has settled (`wall`: the page's time); `auto`: played through (not stepped),
 *  so the report follows. */
function finish(wall, auto) {
  if (W.done) return;
  W.done = true;
  W.doneAt = wall;
  W.autoReport = auto;
  W.playing = false;
  renderControls();
  syncBanner();
}

/** The timeline at replay time `now`. */
function tick(now) {
  settle(now);
  if (!W.cur) {
    if (W.playing && !W.done) beginEvent(now);
    return;
  }
  if (!W.cur.applied && now >= W.cur.impact) applyImpact(now);
  settle(now);
  if (W.cur?.applied && now >= W.cur.end) {
    W.cur = null;
    const stepped = W.stepping;
    if (W.stepping) {
      W.playing = false;
      W.stepping = false;
      renderControls();
    }
    if (W.i >= W.log.events.length) finish(W.wall, !stepped);
  }
}

export function frame(ctx, dt, now) {
  if (!W.ready) return;
  W.wall = now;
  // paused in the middle of an action, the replay stands still there
  if (W.playing || W.done || !W.cur) W.clock += dt * W.speed;
  tick(W.clock);
  draw(ctx, W.clock, now);
  // played through to the end: the report after a moment
  if (W.done && W.autoReport && now - W.doneAt > 2200 / W.speed + 600) {
    W.autoReport = false;
    ctx.toReport(W.log);
  }
}

// ---------------------------------------------------------------------------------------------
// Drawing

/** The battlefield at replay time `now` (`wall`: the page's time, for the breathing and the idle
 *  strips): the stage's figures, those the snap lifts out drawn by it over the rest. */
function draw(ctx, now, wall) {
  const sides = W.log.sides;
  const G = ctx.stage.G;
  const figs = [];
  for (const f of W.F) {
    const pos = W.st.pos[f.b];
    if (!pos) continue;
    const status = W.disp.status[f.b];
    const v = W.vis[f.b];
    const animd = W.anim(f);
    let [x, y] = unitXY(G, f, pos);
    if (v.knock) {
      const k = Math.min(1, (now - v.knock.at) / 300);
      if (k < 1) {
        const [x0, y0] = unitXY(G, f, v.knock.from);
        x = x0 + (x - x0) * k;
        y = y0 + (y - y0) * k;
      }
    }
    const fig = { b: f.b, x, y, side: f.side, large: f.large, flip: faceFlip(f, sides), alpha: 1, bar: null };
    if (status === "gone") {
      const t = Math.min(1, shown(now - (v.goneAt ?? -1e9)) / 600);
      fig.alpha = 1 - t;
      fig.dx = (f.side === 0 ? -1 : 1) * 60 * t;
      fig.dy = (f.side === 0 ? -1 : 1) * 30 * t;
      fig.ring = false;
      if (fig.alpha <= 0) continue;
      fig.pic = pic(f, sides, animd ? "idle" : null, wall + f.b * 137, true);
      figs.push(fig);
      continue;
    }
    if (status === "dead") {
      const since = shown(now - (v.deathAt ?? -1e9));
      fig.ringAlpha = 0.25;
      if (animd) {
        fig.pic = pic(f, sides, "death", since, false) ?? deadPic(f, sides);
        fig.alpha = since > 1140 ? 0.85 : 1;
      } else if (since < 220) {
        fig.pic = pic(f, sides);
        fig.tint = "#ff4030";
        fig.tintA = 0.6;
      } else {
        fig.pic = pic(f, sides);
        fig.fallen = true;
        fig.alpha = 0.85;
      }
      figs.push(fig);
      continue;
    }
    fig.sy = breath(f.b, f.side, wall);
    if (status === "afraid") {
      fig.pic = pic(f, sides, "idle", wall + f.b * 137, true);
      fig.alpha = 0.45;
      fig.ringAlpha = 0.3;
      figs.push(fig);
      continue;
    }
    // lifted out by the snap: the stage keeps its ring, the snap draws the rest
    if (!W.snap.lifts(f.b, now)) {
      fig.pic = pic(f, sides, animd ? "idle" : null, wall + f.b * 137, true);
      fig.bar = { hp: W.disp.hp[f.b], max: f.max_hp * 100 };
    }
    figs.push(fig);
  }
  W.fx = W.fx.filter((e) => shown(now - e.at) < e.dur + 60);
  const fx = W.fx.map((e) => ({ ...e, t: shown(now - e.at) / e.dur }));
  ctx.stage.draw({ figs, fx, shake: W.snap.shake(now) });
  W.snap.drawOver(ctx.stage, now, wall);
  syncHead();
}

// ---------------------------------------------------------------------------------------------
// The panels

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);

function sideBox(side) {
  const L = W.log;
  const label = L.sides?.[side]?.label ?? (side === W.me ? "You" : "Them");
  return pnl(
    { [side === W.me ? "left" : "right"]: "6px", top: "4px", width: side === W.me ? "380px" : "470px" },
    h("div.bt-row", h(`span.bt-h3.${side === W.me ? "you" : "them"}`, label), h(`span.dim#bt-n${side}`)),
    h("div.bt-sbar", h(`i#bt-sb${side}`, { style: { background: side === W.me ? "linear-gradient(#8fb6ff,#3356b6)" : "linear-gradient(#ff8d7a,#c8272b)" } })),
  );
}

function renderUI(ctx) {
  const ui = ctx.ui;
  ui.replaceChildren();
  ctx.ov.replaceChildren();
  add(ui, sideBox(W.me), sideBox(1 - W.me));
  add(ui, h("div.bt-tag.bt-phase#bt-head", { "aria-live": "polite" }));
  W.logEl = h("ol.bt-log#bt-log", { "aria-live": "polite" });
  add(ui, pnl({ left: "6px", top: "506px", width: "420px", height: "288px", display: "flex", flexDirection: "column" }, h("div.bt-row", h("span.bt-h4", "Battle log"), h("span.dim.small", "the number before a line is its unit's speed")), W.logEl));
  add(ui, pnl({ right: "6px", top: "640px", width: "424px", height: "154px", display: "block", padding: "6px 12px" }, h("div#bt-ctl")));
  W.shown = 0;
  syncLog();
  syncTop();
  renderControls();
  syncBanner();
}

function renderControls() {
  const el = document.getElementById("bt-ctl");
  if (!el) return;
  el.replaceChildren();
  add(
    el,
    h(
      "div.bt-row",
      btn(pixGlyph(W.playing ? GLYPH.pause : GLYPH.play, "#fff8e4", 3), { cls: "sq", key: "play", title: W.playing ? "Pause [Space]" : "Play [Space]", disabled: W.done, onclick: () => control("play") }),
      btn(["Step ", pixGlyph(GLYPH.step, "#fff8e4", 2)], { title: "The next action [→]: one unit acts at a time, the fastest first; its hits land at once", key: "step", disabled: W.done, onclick: () => control("step") }),
      btn(W.done ? "Report" : ["Skip ", pixGlyph(GLYPH.skip, "#fff8e4", 2)], { cls: W.done ? "primary" : "", key: "skip", title: "To the report [Esc]", onclick: () => control("skip") }),
    ),
    h(
      "div.bt-row",
      { style: { marginTop: "6px" } },
      h("span.dim", "Speed"),
      SPEEDS.map((s, k) => btn(`${s}×`, { cls: "sm", on: W.speed === s, key: `speed${s}`, title: `[${k + 1}]`, onclick: () => setSpeed(s) })),
    ),
    W.done ? h("div.small.dim", { style: { marginTop: "4px" } }, "The battle is over.") : null,
  );
}

function setSpeed(s) {
  W.speed = s;
  W.ctx.prefs.set("speed", s);
  renderControls();
}

function control(c) {
  if (c === "play") {
    if (W.done) return;
    W.playing = !W.playing;
    W.stepping = false;
  } else if (c === "step") {
    if (W.done) return;
    W.playing = true;
    W.stepping = true;
  } else if (c === "skip") {
    W.ctx.toReport(W.log);
    return;
  }
  renderControls();
}

export function key(ctx, ev) {
  if (!W.ready) return false;
  if (ev.key === " ") control("play");
  else if (ev.key === "ArrowRight") control("step");
  else if (ev.key === "Escape" || ev.key === "Enter") control("skip");
  else if ("1234".includes(ev.key) && ev.key.length === 1) setSpeed(SPEEDS[Number(ev.key) - 1]);
  else return false;
  return true;
}

/** The log's lines up to the events applied so far. */
function syncLog() {
  const el = W.logEl;
  if (!el) return;
  while (W.shown < W.lines.length && W.lines[W.shown].i < W.i) {
    const l = W.lines[W.shown++];
    if (l.head) el.append(h("li.head", l.text));
    else el.append(h("li", l.speed !== null ? h("span.sp", String(l.speed)) : h("span.sp", "·"), partsEl(l.parts)));
  }
  el.scrollTop = el.scrollHeight;
}

/** Each side's standing units (a death or an escape counts once it shows) and their HP. */
function syncTop() {
  for (const side of [0, 1]) {
    const inLine = W.F.filter((f) => f.side === side && W.st.status[f.b] !== "out");
    const up = inLine.filter((f) => W.disp.status[f.b] === "on");
    const max = inLine.reduce((a, f) => a + f.hp_start, 0) || 1;
    const now = up.reduce((a, f) => a + Math.max(0, W.disp.hp[f.b]), 0);
    const n = document.getElementById(`bt-n${side}`);
    if (n) n.textContent = `${up.length} of ${inLine.length} standing`;
    const bar = document.getElementById(`bt-sb${side}`);
    if (bar) bar.style.width = `${Math.round((100 * now) / max)}%`;
  }
}

/** The round and pass of the action under way. */
function syncHead() {
  const el = document.getElementById("bt-head");
  if (!el) return;
  const e = W.cur?.e ?? W.log.events[Math.max(0, W.i - 1)];
  const t = W.done ? "The battle is over" : e ? headOf(e) : "";
  if (el.textContent !== t) el.textContent = t;
}

function syncBanner() {
  const has = document.getElementById("bt-banner");
  if (!W.done) return has?.remove();
  if (has) return;
  const rw = resultWords(W.log);
  add(W.ctx.ui, h("div.bt-pnl.bt-banner.bt-ribbon#bt-banner", { style: { top: "40px" } }, h(rw.good ? "span.goldlt" : "span", rw.ribbon), h("div.small", rw.line)));
}

// ---------------------------------------------------------------------------------------------
// Hooks for the screenshot and smoke scripts

export const dev = {
  pause() {
    W.playing = false;
    renderControls();
  },
  play() {
    W.playing = true;
    renderControls();
  },
  step() {
    control("step");
  },
  skip() {
    control("skip");
  },
  speed(s) {
    setSpeed(s);
  },
  /** The state after the first `n` events, paused there. */
  seek(n) {
    if (!W.ready) return;
    reset(Math.max(0, Math.min(n, W.log.events.length)));
    W.playing = false;
    renderUI(W.ctx);
  },
  /** The state after the first `n` events with the next one staged and `ms` past its contact
   *  frame, paused there (the screenshots' moments); what the snap stages then (`snap()`). */
  freeze(n, ms) {
    if (!W.ready) return null;
    reset(Math.max(0, Math.min(n, W.log.events.length)));
    renderUI(W.ctx);
    W.playing = true;
    tick(W.clock);
    W.playing = false;
    if (W.cur) {
      W.clock = W.cur.impact + ms;
      tick(W.clock);
    }
    renderControls();
    return this.snap();
  },
  /** What the snap stages now: `{kind, shade, crit, kill, roles: {b: role}, t, hold}` or null. */
  snap() {
    return W.snap?.info(W.clock) ?? null;
  },
  /** The event index where `pred(e)` first holds, or -1. */
  find(pred) {
    return W.log ? W.log.events.findIndex(pred) : -1;
  },
  state() {
    return { ready: W.ready, i: W.i, n: W.log?.events.length ?? 0, playing: W.playing, speed: W.speed, done: W.done, acting: W.cur ? W.cur.e.actor : null, clock: W.clock };
  },
  get log() {
    return W.log;
  },
  fates: () => (W.log ? fates(W.log, W.st) : null),
  name: (b) => (W.log ? uname(W.log, b) : ""),
  fig: (b) => (W.log ? figOf(W.F[b], W.log.sides) : null),
  LEAD,
};
