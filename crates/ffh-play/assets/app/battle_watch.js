// Watching a battle (the formation-battle prototype's replay, phase 6 revision 4): the log played
// on the plan's battlefield one action at a time. Each action winds up (a strike or a lunge, a
// shot, a spell or a stone in flight, a heal's sparkles), lands (its event is folded into the
// state then: the numbers float up, crits flash gold, the struck flinch, the fallen die where they
// stand, those who escape a killing blow leave the field, a crit knocks its target back a row)
// and settles. The log beside it gets a line an action, under round and pass headers; play, step,
// skip to the report, speeds 1x to 8x.

import { h } from "./dom.js";
import { initialState, applyEvent, logLines, timingOf, kindOf, kindKey, eventTarget, dmgText, viewerSide, resultWords, fates, headOf, uname } from "./battle_log.js";
import { loadArt, unitXY, cellXY, pic, deadPic, faceFlip, animated, figOf, pixGlyph, GLYPH, btn, add, partsEl, STRIKE_HIT, COLORS } from "./battle_stage.js";

export const NAME = "watch";

const SPEEDS = [1, 2, 4, 8];

const W = { ctx: null, log: null, F: null, me: 0, lines: [], st: null, disp: null, vis: [], fx: [], cur: null, i: 0, playing: true, stepping: false, speed: 2, done: false, doneAt: 0, autoReport: false, ready: false, anim: null, logEl: null, shown: 0 };

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
  W.speed = SPEEDS.includes(ctx.prefs.get("speed", 2)) ? ctx.prefs.get("speed", 2) : 2;
  reset(0);
  W.playing = true;
  renderUI(ctx);
  W.ready = true;
}

export function leave() {
  W.ready = false;
  W.cur = null;
  W.fx = [];
}

/** Back to the state after the first `n` events, at once (no motion). */
function reset(n) {
  W.st = initialState(W.log);
  for (let k = 0; k < n; k++) applyEvent(W.st, W.log.events[k], W.F);
  W.i = n;
  W.disp = { hp: W.st.hp.slice(), status: W.st.status.slice() };
  const long = performance.now() - 5000;
  W.vis = W.F.map((_, b) => (W.st.status[b] === "dead" ? { deathAt: long } : W.st.status[b] === "gone" ? { goneAt: long } : {}));
  W.fx = [];
  W.cur = null;
  W.done = n >= W.log.events.length;
  W.doneAt = W.done ? performance.now() - 5000 : 0;
  W.autoReport = false;
  W.shown = 0;
}

// ---------------------------------------------------------------------------------------------
// The timeline: each action winds up, lands (the event is folded in then), and settles

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

function beginEvent(now) {
  const e = W.log.events[W.i];
  if (!e) return finish(now, true);
  const sp = W.speed;
  const t = timingOf(e);
  W.cur = { e, impact: now + t.impact / sp, end: now + t.end / sp, applied: false };
  const a = e.actor;
  const K = e.kind === "DefStrike" ? kindOf("Shoot") : kindOf(e.kind);
  const tgt = eventTarget(e);
  if (a === null || a === undefined || e.kind === "Afraid") return;
  const v = W.vis[a];
  const [ax, ay] = feet(a);
  const txy = tgt !== null && tgt !== undefined ? feet(tgt) : null;
  if (K.melee && !e.note) {
    v.strikeAt = now;
    if (!W.anim(W.F[a]) && txy) v.lunge = { at: now, dur: (t.impact * 1.6) / sp, tx: txy[0], ty: txy[1] };
  } else if (K.ranged && txy && !e.note) {
    v.castAt = now;
    const kind = e.kind === "DefStrike" ? "Shoot" : kindKey(e.kind);
    W.fx.push({ type: "proj", kind, splash: kind === "Spell" && (e.hits?.length ?? 0) > 1, from: [ax, ay - 64], to: [txy[0], txy[1] - 46], at: now + 150 / sp, dur: (t.impact - 150) / sp });
  } else if (e.heal) {
    v.castAt = now;
    W.fx.push({ type: "sparkle", x: txy[0], y: txy[1], at: now, dur: 700 / sp });
  }
}

function float(b, text, kind, now) {
  const [x, y] = feet(b);
  W.fx.push({ type: "float", x: x + ((b * 7) % 11) - 5, y: y - 70, text, kind, at: now, dur: 1000 });
}

function burst(b, color, now) {
  const [x, y] = feet(b);
  W.fx.push({ type: "burst", x, y: y - 40, color, at: now, dur: 420 });
}

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
  (e.hits ?? []).forEach((x, k) => {
    const v = W.vis[x.target];
    if (x.miss) return float(x.target, "miss", "miss", now);
    if (x.withdrew) {
      v.goneAt = now;
      W.disp.status[x.target] = "gone";
      return float(x.target, "escapes", "miss", now);
    }
    W.disp.hp[x.target] = W.st.hp[x.target];
    float(x.target, `−${dmgText(x.dmg)}${x.crit ? "!" : ""}`, x.crit ? "crit" : "dmg", now);
    if (x.killed) {
      v.deathAt = now;
      W.disp.status[x.target] = "dead";
    } else v.flinchAt = now;
    if (k > 0 || kindKey(e.kind) === "Bombard" || kindKey(e.kind) === "Spell") burst(x.target, kindKey(e.kind) === "Spell" ? "#9feeff" : "#ffd890", now);
    if (x.crit) burst(x.target, COLORS.goldLt, now);
  });
  if (e.heal) {
    W.disp.hp[e.heal.target] = W.st.hp[e.heal.target];
    W.vis[e.heal.target].healAt = now;
    float(e.heal.target, `+${dmgText(e.heal.amount)}`, "heal", now);
  }
  if (e.knocked && knockFrom) W.vis[e.knocked.unit].knock = { from: knockFrom, at: now };
  if (e.note || (!e.hits?.length && !e.heal && e.kind !== "Afraid")) float(e.actor, e.note === "NoOneHurt" || kindKey(e.kind) === "Heal" ? "no one to heal" : "no target", "miss", now);
  syncLog();
  syncTop();
}

/** The last event has settled; `auto`: played through (not stepped), so the report follows. */
function finish(now, auto) {
  if (W.done) return;
  W.done = true;
  W.doneAt = now;
  W.autoReport = auto;
  W.playing = false;
  renderControls();
  syncBanner();
}

function tick(now) {
  if (!W.cur) {
    if (W.playing && !W.done) beginEvent(now);
    return;
  }
  if (!W.cur.applied && now >= W.cur.impact) applyImpact(now);
  if (W.cur?.applied && now >= W.cur.end) {
    W.cur = null;
    const stepped = W.stepping;
    if (W.stepping) {
      W.playing = false;
      W.stepping = false;
      renderControls();
    }
    if (W.i >= W.log.events.length) finish(now, !stepped);
  }
}

export function frame(ctx, _dt, now) {
  if (!W.ready) return;
  tick(now);
  draw(ctx, now);
  // played through to the end: the report after a moment
  if (W.done && W.autoReport && now - W.doneAt > 2200 / W.speed + 600) {
    W.autoReport = false;
    ctx.toReport(W.log);
  }
}

// ---------------------------------------------------------------------------------------------
// Drawing

function draw(ctx, now) {
  const sides = W.log.sides;
  const G = ctx.stage.G;
  const figs = [];
  const actor = W.cur && !W.cur.applied ? W.cur.e.actor : null;
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
      const t = Math.min(1, (now - (v.goneAt ?? 0)) / 600);
      fig.alpha = 1 - t;
      fig.dx = (f.side === 0 ? -1 : 1) * 60 * t;
      fig.dy = (f.side === 0 ? -1 : 1) * 30 * t;
      fig.ring = false;
      if (fig.alpha <= 0) continue;
      fig.pic = pic(f, sides, animd ? "idle" : null, now, true);
      figs.push(fig);
      continue;
    }
    if (status === "dead") {
      const since = (now - (v.deathAt ?? -1e9)) * Math.min(W.speed, 2);
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
    if (status === "afraid") {
      fig.pic = pic(f, sides, "idle", now + f.b * 137, true);
      fig.alpha = 0.45;
      fig.ringAlpha = 0.3;
      figs.push(fig);
      continue;
    }
    // The strips play at the replay's speed, so a strike's blow falls on its impact.
    const sp = W.speed;
    let p = null;
    if (animd && v.strikeAt !== undefined && (now - v.strikeAt) * sp < 870) p = pic(f, sides, "strike", (now - v.strikeAt) * sp, false);
    else if (animd && v.flinchAt !== undefined && (now - v.flinchAt) * sp < 440) p = pic(f, sides, "flinch", (now - v.flinchAt) * sp, false);
    else p = pic(f, sides, animd ? "idle" : null, now + f.b * 137, true);
    fig.pic = p;
    let dx = 0;
    let dy = 0;
    if (v.lunge) {
      const t = (now - v.lunge.at) / v.lunge.dur;
      if (t >= 0 && t <= 1) {
        const k = Math.sin(Math.PI * t) * 0.3;
        dx += (v.lunge.tx - x) * k;
        dy += (v.lunge.ty - y) * k;
      }
    }
    if (v.castAt !== undefined && !animd) {
      const t = (now - v.castAt) / 260;
      if (t >= 0 && t <= 1) dy -= Math.sin(Math.PI * t) * 7;
    }
    if (v.flinchAt !== undefined && !animd) {
      const t = (now - v.flinchAt) / 360;
      if (t >= 0 && t <= 1) {
        fig.tint = "#ffffff";
        fig.tintA = 0.75 * (1 - t);
        dx += Math.sin(t * 30) * 3 * (1 - t);
      }
    }
    if (v.healAt !== undefined) {
      const t = (now - v.healAt) / 500;
      if (t >= 0 && t <= 1) {
        fig.tint = "#7dffb0";
        fig.tintA = 0.5 * (1 - t);
      }
    }
    fig.dx = dx;
    fig.dy = dy;
    if (actor === f.b) {
      fig.glow = COLORS.goldLt;
      fig.glowA = 0.8;
    }
    fig.bar = { hp: W.disp.hp[f.b], max: f.max_hp * 100 };
    figs.push(fig);
  }
  W.fx = W.fx.filter((e) => now - e.at < e.dur + 60);
  const fx = W.fx.map((e) => ({ ...e, t: (now - e.at) / e.dur }));
  ctx.stage.draw({ figs, fx });
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
      btn(["Step ", pixGlyph(GLYPH.step, "#fff8e4", 2)], { title: "The next action [→]", key: "step", disabled: W.done, onclick: () => control("step") }),
      btn(W.done ? "Report" : ["Skip ", pixGlyph(GLYPH.skip, "#fff8e4", 2)], { cls: W.done ? "primary" : "", key: "skip", title: "To the report [Esc]", onclick: () => control("skip") }),
    ),
    h(
      "div.bt-row",
      { style: { marginTop: "6px" } },
      h("span.dim", "Speed"),
      SPEEDS.map((s, k) => btn(`${s}×`, { cls: "sm", on: W.speed === s, key: `speed${s}`, title: `[${k + 1}]`, onclick: () => setSpeed(s) })),
    ),
    h("div.small.dim", { style: { marginTop: "4px" } }, W.done ? "The battle is over." : "One unit acts at a time, the fastest first; its hits land at once."),
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

/** Each side's standing units and their HP. */
function syncTop() {
  for (const side of [0, 1]) {
    const inLine = W.F.filter((f) => f.side === side && W.st.status[f.b] !== "out");
    const up = inLine.filter((f) => W.st.status[f.b] === "on");
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
  /** The event index where `pred(e)` first holds, or -1. */
  find(pred) {
    return W.log ? W.log.events.findIndex(pred) : -1;
  },
  state() {
    return { ready: W.ready, i: W.i, n: W.log?.events.length ?? 0, playing: W.playing, speed: W.speed, done: W.done, acting: W.cur ? W.cur.e.actor : null };
  },
  get log() {
    return W.log;
  },
  fates: () => (W.log ? fates(W.log, W.st) : null),
  name: (b) => (W.log ? uname(W.log, b) : ""),
  fig: (b) => (W.log ? figOf(W.F[b], W.log.sides) : null),
  STRIKE_HIT,
};
