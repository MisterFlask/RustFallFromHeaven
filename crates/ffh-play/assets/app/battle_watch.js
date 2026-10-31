// Watching a battle (the battle UI design, DESIGN.md section 3): the log played on the plan's
// board. Every round, each hitter's target is picked at the start, the hits land as pending damage
// and every bar drops together at the bell; withdrawal rolls, deaths where they fall, the shield
// band cracking. The top bar has each side's strength against the plan's expectation (hollow gold)
// and the round track; a caption tells each round in a sentence; the hit card dissects one hit; the
// lessons come as callouts once, briefly the second time, then as `?` markers. The script and the
// state at any moment are battle_log.js's, so seeking and stepping back are free.

import { h } from "./dom.js";
import { buildScript, stateAt, pendingAt, roundNo, viewerSide, uname, hpOf, x100, pairOf, termLabel, pctText, committed } from "./battle_log.js";
import { assignLessons, captionFor, lessonText, resultWords, CLASS_LABEL, BLOCK_LABEL } from "./battle_teach.js";
import { loadArt, layout, exitPos, pic, deadPic, faceFlip, animated, roleIcon, arcAt, pixGlyph, GLYPH, uiIcon, btn, add, hpColor, COLORS } from "./battle_stage.js";
import { partsBar } from "./battle_plan.js";

export const NAME = "watch";

const W = { ctx: null, log: null, script: null, si: 0, t: 0, playing: true, speed: 1, mode: "full", callout: null, shown: new Set(), furthest: 0, focusOverride: null, cardOpen: false, arrowsAll: false, pos: null, exits: new Map(), xp: {}, bars: new Map(), nums: new Map(), me: 0, ready: false };

const sideColor = (side) => (side === W.me ? COLORS.you : COLORS.them);

export async function enter(ctx, log, { si = 0, mode } = {}) {
  W.ctx = ctx;
  W.ready = false;
  W.log = log;
  W.me = viewerSide(log);
  ctx.stage.me = W.me;
  await loadArt({ fighters: log.fighters, plot: log.plot, from: log.from, sides: log.sides });
  if (W.log !== log) return;
  ctx.stage.setBoard(log, 0);
  W.pos = layout(log.fighters, { raise: ctx.stage.raise });
  W.mode = mode ?? (ctx.ledger.watchMode() === "beats" ? "beats" : "full");
  W.shown = new Set();
  build(ctx);
  W.si = Math.min(si, W.script.segs.length - 1);
  W.t = 0;
  W.playing = true;
  W.callout = null;
  W.furthest = W.si;
  W.focusOverride = null;
  W.arrowsAll = ctx.ledger.setting("arrows_all", null) ?? ctx.ledger.data.battles < 3;
  W.cardOpen = ctx.ledger.seen("a_vs_dv") < 2;
  W.xp = {};
  for (const e of log.events) if (e.kind === "Xp") W.xp[e.unit] = (W.xp[e.unit] ?? 0) + e.amount;
  // Where each unit that pulls out goes: off its side of the board, in the order they leave.
  W.exits = new Map();
  const k = [0, 0];
  for (const s of W.script.segs) for (const w of s.wds) if (w.ev.ok) W.exits.set(w.ev.unit, exitPos(log.fighters[w.ev.unit], log.fighters, k[log.fighters[w.ev.unit].side]++));
  for (const s of W.script.segs) for (const x of s.satOut) if (!W.exits.has(x.ev.unit)) W.exits.set(x.ev.unit, exitPos(log.fighters[x.ev.unit], log.fighters, k[log.fighters[x.ev.unit].side]++));
  renderUI(ctx);
  W.ready = true;
}

export function leave() {
  closeCallout();
  W.ready = false;
}

/** The script for the current mode, with the lessons the ledger still has to give. */
function build(ctx) {
  const L = ctx.ledger;
  const pauses = L.setting("pause_lessons", true);
  W.script = buildScript(W.log, {
    mode: W.mode,
    animated: animated(W.log.sides),
    rulesSeen: L.rulesSeen(),
    lessons: (segs, log) => {
      assignLessons(segs, log, (id) => L.level(id));
      for (const s of segs) if (s.lesson?.mode === "pause" && !pauses) s.lesson.mode = "full";
    },
    caption: captionFor,
  });
  // The lessons already shown in this viewing do not come back (a mode change rebuilds the script).
  for (const s of W.script.segs) if (s.lesson && W.shown.has(s.lesson.id)) s.lesson = null;
}

function setMode(ctx, m) {
  W.mode = m;
  ctx.ledger.set("watch_mode", m);
  build(ctx);
  W.t = 0;
  renderUI(ctx);
}

// ---------------------------------------------------------------------------------------------
// Controls

function control(ctx, c) {
  const segs = W.script.segs;
  if (c === "play") {
    W.playing = !W.playing;
    if (W.callout?.mode === "pause") {
      closeCallout();
      W.playing = true;
    }
  } else if (c === "back") seek(W.t > 400 ? W.si : Math.max(0, W.si - 1), 0);
  else if (c === "round") {
    if (W.si + 1 < segs.length) seek(W.si + 1, 0);
    W.playing = false;
  } else if (c === "beat") {
    let k = W.si + 1;
    while (k < segs.length - 1 && !segs[k].beat) k++;
    seek(Math.min(k, segs.length - 1), 0);
    W.playing = false;
  } else if (c === "hit") {
    const next = segs[W.si].hits.find((x) => x.impact > W.t + 1);
    if (next) W.t = next.impact + 1;
    else if (W.si + 1 < segs.length) {
      seek(W.si + 1, 0);
      const n = segs[W.si].hits[0];
      if (n) W.t = n.impact + 1;
    }
    W.playing = false;
  } else if (c === "skip") ctx.toReport(W.log, { script: W.script, shown: W.shown });
  else if (c === "arrows") {
    W.arrowsAll = !W.arrowsAll;
    ctx.ledger.set("arrows_all", W.arrowsAll);
    renderUI(ctx);
  }
}

function seek(si, t) {
  W.si = si;
  W.t = t;
  W.focusOverride = null;
  closeCallout();
  W.furthest = Math.max(W.furthest, si);
  buildTrack();
  buildTimeline();
}

export function key(ctx, ev) {
  if (!W.ready) return false;
  if (ev.key === " ") control(ctx, "play");
  else if (ev.key === "ArrowRight") control(ctx, ev.ctrlKey || ev.metaKey ? "beat" : ev.shiftKey ? "round" : "hit");
  else if (ev.key === "ArrowLeft") control(ctx, "back");
  else if (ev.key === "Escape" || ev.key === "Enter") control(ctx, "skip");
  else if ("1234".includes(ev.key) && ev.key.length === 1) {
    W.speed = [0.5, 1, 2, 4][Number(ev.key) - 1];
    renderUI(ctx);
  } else if (ev.key === "t" || ev.key === "T") control(ctx, "arrows");
  else return false;
  return true;
}

// ---------------------------------------------------------------------------------------------
// The panels

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);
/** Replaces `el`'s children (arrays flattened, null skipped). */
const refill = (el, ...children) => (el.replaceChildren(), add(el, ...children));

function renderUI(ctx) {
  const ui = ctx.ui;
  ui.replaceChildren();
  ctx.ov.replaceChildren();
  W.bars = new Map();
  W.nums = new Map();
  const L = W.log;
  const them = L.sides?.[1 - W.me]?.label ?? L.plot?.city?.name ?? "Them";
  const sbar = (side) => h(`span.bt-sbar#bt-sb${side}`, h("i.v"), h("i.pend"), h("i.wdh"), h("i.plan", { title: "Where the plan expected this side to be now" }));
  const num = (side) => h("span.bt-num", { style: { alignItems: side === W.me ? "flex-end" : "flex-start" } }, h(`span.dim#bt-sn${side}`), h(`span.lilac.small#bt-lk${side}`, { title: "The average luck roll of this side's hits so far" }));
  add(ui, 
    h(
      "div.bt-top",
      pnl({ left: "0", width: "380px" }, h("span.bt-h3.you", W.me === 0 ? "You" : "Your garrison"), sbar(W.me), num(W.me)),
      pnl({ left: "386px", width: "496px", justifyContent: "center", padding: "0 6px" }, h("div.bt-track#bt-track")),
      pnl({ left: "888px", right: "0" }, num(1 - W.me), sbar(1 - W.me), h("span.bt-h3.them", them)),
    ),
  );
  const cap = h("div.bt-tag.bt-caption#bt-caption", { "aria-live": "polite" });
  add(ui, cap);
  add(ui, h("div.bt-pnl#bt-hitcard", { style: { left: "6px", top: "636px", width: "666px", height: "158px", padding: "2px 12px" } }));
  const playG = pixGlyph(W.playing ? GLYPH.pause : GLYPH.play, "#fff8e4", 3);
  const ctl = pnl(
    { left: "678px", top: "636px", right: "6px", height: "158px", padding: "4px 12px", display: "block" },
    h(
      "div.bt-row",
      btn(pixGlyph(GLYPH.back, "#fff8e4", 3), { cls: "sq", title: "Back to the round's start, then the round before [←]", onclick: () => control(ctx, "back") }),
      btn(playG, { cls: "sq", title: "Play or pause [Space]", key: "play", onclick: () => control(ctx, "play") }),
      btn("Hit", { title: "The next hit [→]", onclick: () => control(ctx, "hit") }),
      btn("Round", { title: "The next round [Shift+→]", onclick: () => control(ctx, "round") }),
      btn("Beat", { title: "The next death, withdrawal or broken shield [Ctrl+→]", onclick: () => control(ctx, "beat") }),
      h("span", { style: { flex: "1" } }),
      btn(["Skip ", pixGlyph(GLYPH.skip, "#fff8e4", 3)], { title: "To the report [Esc]", onclick: () => control(ctx, "skip") }),
    ),
    h(
      "div.bt-row",
      { style: { marginTop: "6px" } },
      h("span.dim", "Speed"),
      [0.5, 1, 2, 4].map((v, k) => btn(v === 0.5 ? "½" : `${v}×`, { cls: "sm", on: W.speed === v, title: `[${k + 1}]`, onclick: () => ((W.speed = v), renderUI(ctx)) })),
      h("span.dim", { style: { marginLeft: "10px" } }, "Pace"),
      btn("Full", { cls: "sm", on: W.mode === "full", onclick: () => setMode(ctx, "full") }),
      btn("Beats", { cls: "sm", on: W.mode === "beats", onclick: () => setMode(ctx, "beats") }),
      btn("T", { cls: "sm", on: W.arrowsAll, title: "An arrow for every hit of yours [T]", onclick: () => control(ctx, "arrows") }),
    ),
    h("div.bt-tl#bt-tl", {
      title: "Seek (up to the furthest point watched)",
      onclick: (e) => {
        const r = e.currentTarget.getBoundingClientRect();
        const u = (e.clientX - r.left) / r.width;
        const n = W.script.segs.length;
        seek(Math.max(0, Math.min(W.furthest, Math.round(u * (n - 1)))), 0);
      },
    }),
  );
  ctl.querySelector("[data-k=play]").id = "bt-play";
  add(ui, ctl);
  if (W.callout) add(ui, W.callout.el);
  buildTrack();
  buildTimeline();
}

/** The round track: Op, a chip a round up to the forecast's median length, End; over each chip the
 *  deaths seen (filled) and those the forecast expected then (hollow gold). */
function buildTrack() {
  const tr = document.getElementById("bt-track");
  if (!tr || !W.script) return;
  const segs = W.script.segs;
  const F = W.log.fighters;
  const fc = W.log.forecast;
  const cur = segs[W.si];
  const curN = cur.type === "round" ? roundNo(cur) : cur.type === "end" || cur.type === "post" ? 99 : -1;
  const lastSeen = Math.max(0, ...segs.slice(0, W.furthest + 1).map(roundNo));
  const planEnd = fc?.rounds?.p50 ?? W.log.outcome.rounds;
  const N = Math.max(planEnd, lastSeen, curN < 99 ? curN : 0) + (W.si >= segs.length - 2 ? 0 : 1);
  const planDeaths = {};
  for (const p of fc?.per ?? []) if (p.death_round !== null && p.death_round !== undefined && F[p.b] && committed(F[p.b])) (planDeaths[p.death_round] ??= []).push(F[p.b].side);
  const realDeaths = {};
  segs.forEach((s, k) => {
    if (k <= W.furthest && s.type === "round") for (const d of s.deaths) (realDeaths[roundNo(s)] ??= []).push(F[d.b].side);
  });
  const out = [h(`span.ph${curN >= -1 && curN !== -1 ? ".done" : ""}`, "Op")];
  const maxN = Math.min(N, 22);
  for (let n = 1; n <= maxN; n++) {
    const marks = [...(realDeaths[n] ?? []).map((s) => h(`i.mk.${s === W.me ? "you" : "them"}`)), ...(planDeaths[n] ?? []).map(() => h("i.mk.plan"))];
    const cls = n === curN ? ".now" : n < curN || curN === 99 ? ".done" : "";
    out.push(h(`span.ch${cls}`, { title: `Round ${n}` }, h("span.marks", marks), n % 5 === 0 || n === curN ? h("span.rn", String(n)) : null));
  }
  if (N > maxN) out.push(h("span.ph", "…"));
  out.push(h(`span.ph${curN === 99 ? ".now" : ""}`, "End"));
  tr.replaceChildren(...out);
}

function buildTimeline() {
  const tl = document.getElementById("bt-tl");
  if (!tl || !W.script) return;
  const segs = W.script.segs;
  const n = Math.max(1, segs.length - 1);
  const out = [h("div.trk"), h("div.fill#bt-tlfill")];
  segs.forEach((s, k) => {
    const x = `${(100 * k) / n}%`;
    if (s.type === "round" && s.kind === "round" && (s.n % 5 === 0 || s.n === 1)) out.push(h("div.tk", { style: { left: x } }), h("div.rn", { style: { left: x } }, String(s.n)));
    if (k <= W.furthest && s.deaths.length) out.push(h("div.mk", { style: { left: x } }, pixGlyph(GLYPH.skull, s.deaths.some((d) => W.log.fighters[d.b].side === W.me) ? COLORS.you : "#ffb4a8", 2)));
  });
  out.push(h("div.hd#bt-tlhead"));
  tl.replaceChildren(...out);
}

// ---------------------------------------------------------------------------------------------
// Lessons

function lessonTime(s) {
  const L = s.lesson;
  if (typeof L.trigger === "number") return L.trigger;
  if (L.trigger === "land") return s.land + 150;
  if (L.trigger === "shield") return (s.shieldDowns[0]?.start ?? s.land) + 300;
  if (L.trigger === "wd") return s.wds[0].result + 150;
  if (L.anchor?.hitIdx !== undefined) return s.hits[L.anchor.hitIdx].impact + 80;
  if (L.ctx?.focus) return s.hits[s.focus].impact + 120;
  return (s.pick ?? 0) + 100;
}

function anchorPoint(s, an = {}) {
  if (an.card) return { x: 300, y: 700 };
  if (an.band !== undefined) return { x: W.ctx.stage.bandX(an.band), y: 330 };
  if (an.hitIdx !== undefined) return W.pos[s.hits[an.hitIdx].t];
  if (an.b !== undefined) return W.pos[an.b];
  return { x: 640, y: 200 };
}

function openCallout(ctx, s, lesson = s.lesson, forceMode = null) {
  closeCallout();
  // A lesson shown counts in the ledger; one re-read from its marker does not.
  if (forceMode !== "replay") W.shown.add(lesson.id);
  const [title, body] = lessonText(lesson.id, W.log, lesson.ctx);
  const mode = forceMode ?? lesson.mode;
  const short = mode === "short";
  const an = lesson.anchor ?? {};
  const box = h(`div.bt-pnl.bt-callout${short ? ".short" : ""}`, { role: "status" });
  if (short) add(box, h("b.goldlt", `${title}. `), h("span", `${body.split(". ")[0].replace(/\.$/, "")}.`));
  else {
    add(box, h("div.bt-h3", mode !== "replay" ? h("span.newrule", "NEW RULE") : null, title), h("div", body));
    if (mode === "pause") {
      add(box, 
        h(
          "div.bt-row",
          { style: { marginTop: "6px" } },
          btn("Got it", { title: "Go on [Space]", onclick: () => closeCallout() }),
          btn("Teach less", { cls: "sm", title: "Short lessons from now on", onclick: () => (ctx.ledger.teachLess(), closeCallout()) }),
        ),
      );
    }
  }
  const bx = an.card ? 6 : 740;
  box.style.left = `${bx}px`;
  box.style.bottom = short ? "250px" : "234px";
  add(ctx.ui, box);
  const p = anchorPoint(s, an);
  W.callout = { el: box, mode, left: mode === "pause" ? Infinity : mode === "full" ? 3800 : mode === "replay" ? 6000 : 2600, anchor: { x: p.x, y: p.y - 60 }, id: lesson.id, bx, bw: short ? 520 : 500 };
}

function closeCallout() {
  if (!W.callout) return;
  W.callout.el.remove();
  W.callout = null;
}

// ---------------------------------------------------------------------------------------------
// The frame

export function frame(ctx, dt, now) {
  if (!W.ready || !W.script) return;
  const segs = W.script.segs;
  let s = segs[W.si];
  if (s.lesson && !W.shown.has(s.lesson.id) && W.t >= lessonTime(s)) {
    openCallout(ctx, s);
    if (s.lesson.mode === "pause") W.t = lessonTime(s);
  }
  const rate = W.speed * (W.callout?.mode === "full" ? 0.5 : 1);
  if (W.playing && W.callout?.mode !== "pause") {
    W.t += dt * rate;
    if (s.lesson && !W.shown.has(s.lesson.id) && s.lesson.mode === "pause") W.t = Math.min(W.t, lessonTime(s));
    while (W.t >= s.dur) {
      if (W.si + 1 >= segs.length) {
        W.t = s.dur;
        ctx.toReport(W.log, { script: W.script, shown: W.shown });
        return;
      }
      W.t -= s.dur;
      W.si++;
      s = segs[W.si];
      W.focusOverride = null;
      if (W.si > W.furthest) {
        W.furthest = W.si;
        buildTimeline();
      }
      buildTrack();
    }
  }
  if (W.callout) {
    W.callout.left -= dt;
    if (W.callout.mode !== "pause" && W.callout.left <= 0) closeCallout();
  }
  draw(ctx, s, W.t, now);
}

function draw(ctx, s, t, now) {
  const L = W.log;
  const F = L.fighters;
  const pos = W.pos;
  const sides = L.sides;
  const reduced = ctx.reduced();
  const st = stateAt(W.script, W.si, t);
  const pending = pendingAt(s, F.length, t);
  const figs = [];
  const arrows = [];
  const projs = [];
  const sparks = [];
  const rings = [];
  const shards = [];
  let shake = 0;
  const hitsBy = new Map();
  const hitsOn = new Map();
  const push = (m, k, v) => (m.get(k) ?? m.set(k, []).get(k)).push(v);
  for (const x of s.hits) {
    push(hitsBy, x.h, x);
    push(hitsOn, x.t, x);
  }
  if (s.type === "strikes") {
    for (const x of s.strikes) {
      const a = pos[x.ev.striker ?? x.ev.source];
      const b = pos[x.ev.target];
      if (!a || !b) continue;
      if (t >= x.start + 60 && t < x.impact) projs.push({ ...arcAt(a.x - 20, a.y - 56, b.x + 10, b.y - 50, 60, (t - x.start - 60) / (x.impact - x.start - 60)), kind: "shot" });
      if (t >= x.impact && t < x.impact + 260) sparks.push({ x: b.x, y: b.y - 50, t: (t - x.impact) / 260, tier: x.ev.dmg >= 600 ? 1 : 0 });
      push(hitsOn, x.ev.target, { impact: x.impact, dmg: x.ev.dmg, tier: x.ev.dmg >= 600 ? 1 : 0 });
    }
  }
  for (const f of F) {
    const p0 = pos[f.b];
    const fig = { x: p0.x, y: p0.y, side: f.side, flip: faceFlip(f, sides), pic: pic(f, sides, "idle", now + f.b * 170, true), roleIcon: roleIcon(f) };
    if (!committed(f)) {
      Object.assign(fig, { alpha: 0.62, ringAlpha: 0.35 });
      figs.push(fig);
      continue;
    }
    const wasOut = s.snap.out[f.b] || s.snap.sat[f.b];
    const ex = W.exits.get(f.b) ?? p0;
    if (s.snap.dead[f.b]) {
      const dp = deadPic(f, sides);
      Object.assign(fig, dp ? { pic: dp, alpha: 0.8, ringAlpha: 0.2 } : { fallen: true, pic: pic(f, sides), alpha: 0.8, ringAlpha: 0.2 });
      figs.push(fig);
      continue;
    }
    if (wasOut) {
      Object.assign(fig, { x: ex.x, y: ex.y, alpha: 0.8, ringAlpha: 0.4 });
      figs.push(fig);
      continue;
    }
    // sitting out (afraid) or pulling out now: it walks off its side of the board
    const w = s.wds.find((x) => x.ev.unit === f.b && x.ev.ok);
    const sat = s.type === "morale" && s.satOut.some((x) => x.ev.unit === f.b);
    if ((w && t >= w.result) || (sat && t >= s.satAt)) {
      const t0 = w ? w.result : s.satAt;
      const t1 = w ? w.walkEnd : s.satAt + 1100;
      const u = Math.min(1, (t - t0) / Math.max(1, t1 - t0));
      fig.x = p0.x + (ex.x - p0.x) * u;
      fig.y = p0.y + (ex.y - p0.y) * u;
      if (u < 1) {
        fig.flip = faceFlip(f, sides, f.side === 0 ? "left" : "right");
        const walk = pic(f, sides, "walk", t - t0, true);
        if (walk && walk !== fig.pic && walk.w === 160) fig.pic = walk;
        else if (!reduced) fig.dy = -Math.abs(Math.sin((t - t0) / 70)) * 2;
      } else Object.assign(fig, { alpha: 0.8, ringAlpha: 0.4 });
      figs.push(fig);
      continue;
    }
    // dying now
    if (s.deaths.some((d) => d.b === f.b) && t >= s.deathStart) {
      const u = t - s.deathStart;
      const dp = deadPic(f, sides);
      if (dp && !reduced) fig.pic = pic(f, sides, "death", u, false);
      else if (u < 260 && !reduced) Object.assign(fig, { tint: "#140d0c", tintA: Math.min(0.5, u / 500), dy: Math.round(u / 60) });
      else if (dp) Object.assign(fig, { pic: dp, alpha: 0.8 });
      else Object.assign(fig, { fallen: true, pic: pic(f, sides), alpha: 0.85 });
      fig.ringAlpha = Math.max(0.2, 1 - u / 800);
      figs.push(fig);
      continue;
    }
    // striking
    const mine = (hitsBy.get(f.b) ?? []).find((x) => t >= x.start && t < x.end);
    if (mine) {
      const tp = pos[mine.t];
      const u = (t - mine.start) / (mine.scale || 1);
      const dx = tp.x - p0.x;
      const dy = tp.y - p0.y;
      const len = Math.hypot(dx, dy) || 1;
      const reach = (1 + mine.tier * 0.35) * (reduced ? 0.15 : 1);
      if (mine.kind === "anim") {
        fig.pic = pic(f, sides, "strike", u, false) ?? fig.pic;
        const slide = u < 300 ? 0 : u < 500 ? (u - 300) / 200 : Math.max(0, 1 - (u - 500) / 300);
        fig.dx = (dx / len) * 10 * reach * slide;
        fig.dy = (dy / len) * 6 * slide * (reduced ? 0.15 : 1);
      } else if (mine.kind === "lunge") {
        const k = u < 200 ? u / 200 : Math.max(0, 1 - (u - 200) / 230);
        fig.dx = (dx / len) * 20 * reach * k;
        fig.dy = (dy / len) * 10 * reach * k;
      } else if (mine.kind === "ride" && !reduced) {
        const far = Math.min(0.72, Math.max(0.3, 1 - 70 / len));
        const k = u < 380 ? u / 380 : Math.max(0, 1 - (u - 380) / 340);
        const pt = arcAt(p0.x, p0.y, tp.x - Math.sign(dx) * 50, tp.y, 70, ((k * far) / 0.72) * 0.95);
        fig.x = pt.x;
        fig.y = pt.y;
        if (u > 380) fig.flip = faceFlip(f, sides, f.side === 0 ? "left" : "right");
      } else if (mine.kind === "shot" || mine.kind === "bolt" || mine.kind === "ride") {
        fig.dx = -Math.sign(dx) * (u < 120 ? 2 : 0);
        if (mine.kind !== "ride" && t - mine.start >= 60 && t < mine.impact) {
          const v = (t - mine.start - 60) / Math.max(1, mine.impact - mine.start - 60);
          projs.push({ ...arcAt(p0.x + Math.sign(dx) * 20, p0.y - 58, tp.x - Math.sign(dx) * 8, tp.y - 52, mine.kind === "shot" ? 50 : 18, v), kind: mine.kind });
        }
      }
    }
    // being hit
    const hitNow = (hitsOn.get(f.b) ?? []).filter((x) => t >= x.impact && t < x.impact + 440);
    if (hitNow.length && !mine) {
      const last = hitNow[hitNow.length - 1];
      const fl = pic(f, sides, "flinch", t - last.impact, false);
      if (fl && fl.w === 160) fig.pic = fl;
      else if (!reduced) fig.dx = t - last.impact < 160 ? (Math.floor((t - last.impact) / 40) % 2 ? 2 : -2) : 0;
      if (t - last.impact < 90) Object.assign(fig, { tint: "#fffaec", tintA: 0.75 });
    }
    for (const x of hitsOn.get(f.b) ?? []) {
      if (t < x.impact || t >= x.impact + 260) continue;
      sparks.push({ x: p0.x, y: p0.y - 52 - (x.tier === 2 ? 4 : 0), t: (t - x.impact) / 260, tier: x.tier ?? 0 });
      if (x.tier === 2 && t < x.impact + 90 && !reduced) shake = 1;
    }
    figs.push(fig);
  }
  // Targets: the focused hit (gold), every hit of yours in the first battles, cyan rings on a new choice.
  if (s.type === "round") {
    const fh = s.hits[W.focusOverride ?? s.focus];
    if (W.arrowsAll) {
      for (const x of s.hits) {
        if (F[x.h].side !== W.me || x === fh || t >= x.impact) continue;
        const a = pos[x.h];
        const b = pos[x.t];
        arrows.push({ x0: a.x, y0: a.y - 64, x1: b.x - 16, y1: b.y - 60, color: "#bfe0ff", width: 2, head: 8, bend: 16 + Math.abs(a.y - b.y) * 0.1, alpha: 0.75 });
      }
    }
    if (fh && t < Math.max(fh.impact + 500, s.land)) {
      const a = pos[fh.h];
      const b = pos[fh.t];
      arrows.push({ x0: a.x, y0: a.y - 64, x1: b.x + (F[fh.h].side === 0 ? -16 : 16), y1: b.y - 62, color: COLORS.goldLt, width: 4, head: 11, bend: fh.ev.rule === "FlankerAround" ? 80 : 30 });
      const les = s.lesson;
      const newChoice = !W.ctx.ledger.data.rules[fh.ev.rule] && fh.ev.eligible.length > 1;
      if ((les && (les.id === "flank_around" || les.id === "flankers") && W.shown.has(les.id)) || newChoice) for (const e of fh.ev.eligible) rings.push({ x: pos[e].x, y: pos[e].y, color: COLORS.cyan });
    }
  }
  // the shield bands while the front stands; shards when one breaks
  const bands = [];
  for (const side of [0, 1]) {
    if (st.shield[side]) bands.push({ side });
    const sd = s.shieldDowns.find((x) => x.side === side);
    if (sd && t >= sd.start && t < sd.start + 800 && !reduced) shards.push(...ctx.stage.shards(side, (t - sd.start) / 800));
  }
  ctx.stage.shake = shake;
  const o = L.outcome;
  const taken = o.plot_taken && o.result === "AttackerWon";
  const cityFlag = s.type === "post" && t > 1200 && taken ? (W.me === 0 ? COLORS.azure[0] : COLORS.gules[0]) : null;
  // the aftermath: the survivors step toward the city
  if (s.type === "post" && taken) {
    const u = Math.min(1, Math.max(0, (t - 400) / 1600));
    const c = ctx.stage.cityPoint();
    for (let i = 0; i < F.length; i++) {
      const f = F[i];
      const fig = figs[i];
      if (f.side !== 0 || !committed(f) || st.dead[f.b] || st.out[f.b]) continue;
      fig.x += (c.x - 80 - fig.x) * u * 0.35;
      fig.y += (c.y + 30 - fig.y) * u * 0.2;
    }
  }
  const lines = [];
  if (W.callout && W.callout.mode !== "short") {
    const c = W.callout;
    const ty = c.el.offsetTop + 4;
    const tx = Math.max(c.bx + 20, Math.min(c.bx + c.bw - 20, c.anchor.x));
    lines.push({ x0: tx, y0: ty, x1: c.anchor.x, y1: c.anchor.y });
  }
  ctx.stage.draw({ figs, arrows, projs, sparks, rings, bands, shards, cityFlag, lines });
  syncOverlay(ctx, s, t, st, pending, figs);
  syncTop(s, st, pending);
  syncCard(ctx, s, t);
  syncMisc(ctx, s, t);
}

/** The bars over the heads, their tags (death, withdrawal, the die, XP), the damage numbers and the
 *  lessons' `?` markers. */
function syncOverlay(ctx, s, t, st, pending, figs) {
  const L = W.log;
  const F = L.fighters;
  const ov = ctx.ov;
  F.forEach((f, i) => {
    const fig = figs[i];
    let bar = W.bars.get(f.b);
    if (!bar) {
      const v = h("i.v");
      const pe = h("i.pend");
      const hp = h("span.bt-hp", { style: { width: "36px" } }, v, pe, f.withdraw_pct ? h("b.tick", { style: { left: "13px" } }) : null);
      const tg = h("span.tg");
      bar = h("div.bt-ubar.click", { title: uname(L, f.b), onclick: () => focusOn(f.b) }, uiIcon(roleIcon(f), 16), hp, tg);
      bar.parts = { v, pe, hp, tg, icon: bar.firstChild };
      add(ov, bar);
      W.bars.set(f.b, bar);
    }
    const dead = st.dead[f.b] && (s.snap.dead[f.b] || t >= s.deathStart + 600);
    const hp = Math.max(0, st.hp[f.b]);
    const max = f.max_hp * 100;
    const q = Math.max(0, hp - pending[f.b]) / max;
    const p = hp / max;
    bar.style.left = `${Math.round(fig.x + (fig.dx || 0))}px`;
    bar.style.top = `${Math.round(fig.y - (dead ? 30 : 126))}px`;
    const walking = s.wds.some((x) => x.ev.unit === f.b && x.ev.ok && t < x.walkEnd);
    bar.classList.toggle("out", !committed(f) || ((st.out[f.b] || st.sat[f.b]) && !walking));
    bar.parts.v.style.width = `${Math.round(q * 36)}px`;
    bar.parts.v.style.background = hpColor(q);
    bar.parts.pe.style.left = `${Math.round(q * 36)}px`;
    bar.parts.pe.style.width = `${Math.round((p - q) * 36)}px`;
    bar.parts.icon.style.display = dead ? "none" : "";
    bar.parts.hp.style.display = dead ? "none" : "";
    let key = "";
    let tag = null;
    if (dead) {
      key = `d${st.diedIn[f.b]}`;
      tag = () => h("span.bt-chip.gone.tiny", pixGlyph(GLYPH.skull, "#f2e6d0", 2), ` R${st.diedIn[f.b] || "op"}`);
    } else if (st.out[f.b]) {
      key = `w${st.wdIn[f.b]}`;
      tag = () => h("span.bt-chip.wd.tiny", `↩ R${st.wdIn[f.b]}`);
    } else if (st.sat[f.b]) {
      key = "sat";
      tag = () => h("span.bt-chip.gone.tiny", "afraid");
    }
    if (s.type === "post" && !dead && committed(f) && t > 300 && W.xp[f.b]) {
      key = `x${Math.min(10, Math.floor((t - 300) / 60))}`;
      tag = () => h("span.bt-chip.xp", { style: { transform: `translateY(${-Math.min(10, (t - 300) / 60)}px)` } }, `+${W.xp[f.b]}★`);
    }
    const w = s.wds.find((x) => x.ev.unit === f.b);
    if (w && t >= w.start && t < w.result + 900) {
      const spin = t < w.result;
      const shown = spin ? Math.floor(((t - w.start) / 60) % 100) : w.ev.roll;
      key = `r${shown}${spin}`;
      tag = () => h("span.bt-chip.wd", pixGlyph(GLYPH.die, "#c9a8f0", 2), ` ${w.ev.chance}%: ${shown}${spin ? "" : w.ev.ok ? " ↩ out" : " stays"}`);
    }
    if (bar.parts.tg.dataset.k !== key) {
      bar.parts.tg.dataset.k = key;
      bar.parts.tg.replaceChildren(...(tag ? [tag()] : []));
    }
  });
  // Damage numbers: each hit's at its impact, tiered; at the bell one plate per target.
  const want = new Map();
  if (s.type === "round" || s.type === "strikes") {
    const list =
      s.type === "strikes"
        ? s.strikes.map((x) => ({ key: `s${x.i}`, t: x.ev.target, dmg: x.ev.dmg, impact: x.impact, tier: x.ev.dmg >= 600 ? 1 : 0, strike: true }))
        : s.hits.map((x) => ({ key: `h${x.i}`, t: x.t, dmg: x.dmg, impact: x.impact, tier: x.tier }));
    const perT = new Map();
    for (const x of list) {
      const k = perT.get(x.t) ?? 0;
      perT.set(x.t, k + 1);
      if (t >= x.impact && (t < s.land || x.strike) && t < x.impact + 1100) {
        const p = W.pos[x.t];
        const u = Math.min(1, (t - x.impact) / 500);
        want.set(x.key, { x: p.x - 18 + (k % 3) * 18, y: p.y - 92 - (k > 2 ? 18 : 0) - u * 12, txt: hpOf(x.dmg) === 0 && x.strike ? "glance" : String(hpOf(x.dmg)), cls: `bt-dmg${[".glance", "", ".heavy"][x.tier]}` });
      }
    }
    if (s.type === "round" && t >= s.land) {
      const tot = new Map();
      for (const x of s.hits) tot.set(x.t, (tot.get(x.t) ?? 0) + x.dmg);
      for (const [b, d] of tot) {
        const p = W.pos[b];
        const died = s.deaths.find((x) => x.b === b);
        if (!p || st.out[b]) continue;
        want.set(`t${W.si}:${b}`, { x: p.x, y: p.y - 100, txt: `−${hpOf(d)}`, skull: !!died, over: died && died.overkill >= 100 ? hpOf(died.overkill) : 0, cls: `bt-dmg.sum${F[b].side === W.me ? ".mine" : ""}` });
      }
    }
  }
  for (const [k, e] of W.nums) {
    if (k[0] !== "q" && !want.has(k)) {
      e.remove();
      W.nums.delete(k);
    }
  }
  for (const [k, v] of want) {
    let e = W.nums.get(k);
    if (!e) {
      e = h(`div.${v.cls}`, v.txt, v.skull ? pixGlyph(GLYPH.skull, "#f2e6d0", 2) : null, v.over ? h("span.small", ` overkill ${v.over}`) : null);
      add(ov, e);
      W.nums.set(k, e);
    }
    e.style.left = `${Math.round(v.x)}px`;
    e.style.top = `${Math.round(v.y)}px`;
  }
  // icon-level lessons: a ? marker at the spot; a click shows the lesson again
  const qs = new Set();
  for (const ic of s.icons ?? []) {
    const p = anchorPoint(s, ic.anchor);
    const k = `q${ic.id}`;
    qs.add(k);
    let e = W.nums.get(k);
    if (!e) {
      e = h("div.bt-q.click", {
        title: lessonText(ic.id, L, ic.ctx).join(": "),
        role: "button",
        onclick: () => openCallout(ctx, s, ic, "replay"),
      }, "?");
      add(ov, e);
      W.nums.set(k, e);
    }
    e.style.left = `${Math.round(p.x + 30)}px`;
    e.style.top = `${Math.round(ic.anchor?.card ? p.y : p.y - 70)}px`;
  }
  for (const [k, e] of W.nums) {
    if (k[0] === "q" && !qs.has(k)) {
      e.remove();
      W.nums.delete(k);
    }
  }
}

/** A unit's bar clicked: the hit card shows its hit of this round. */
function focusOn(b) {
  const k = W.script.segs[W.si].hits.findIndex((x) => x.h === b);
  if (k >= 0) W.focusOverride = k;
}

/** The strength bars: each side's living committed HP, the damage still to land (white), the HP
 *  that pulled out alive (hatched), the plan's expectation (hollow gold), and the running luck. */
function syncTop(s, st, pending) {
  const L = W.log;
  const F = L.fighters;
  const fc = L.forecast;
  for (const side of [0, 1]) {
    const bar = document.getElementById(`bt-sb${side}`);
    if (!bar) continue;
    const mine = F.filter((f) => f.side === side && committed(f));
    const max = mine.reduce((a, f) => a + f.hp_start, 0) || 1;
    const alive = mine.filter((f) => !st.dead[f.b] && !st.out[f.b] && !st.sat[f.b]);
    const now = alive.reduce((a, f) => a + Math.max(0, st.hp[f.b]), 0);
    const pend = alive.reduce((a, f) => a + Math.min(pending[f.b], Math.max(0, st.hp[f.b])), 0);
    const wdhp = mine.filter((f) => st.out[f.b]).reduce((a, f) => a + st.hp[f.b], 0);
    const [v, pe, wd, pl] = bar.children;
    const w = (x) => `${(100 * x) / max}%`;
    const fromLeft = side === W.me;
    const set = (el, start, width) => {
      el.style[fromLeft ? "left" : "right"] = w(start);
      el.style[fromLeft ? "right" : "left"] = "auto";
      el.style.width = w(width);
    };
    set(v, 0, now - pend);
    set(pe, now - pend, pend);
    set(wd, now, wdhp);
    v.style.background = side === W.me ? "linear-gradient(#8fb6ff,#3356b6)" : "linear-gradient(#ff8d7a,#c8272b)";
    if (fc?.curve?.[side]?.length) {
      const c = fc.curve[side];
      const step = s.type === "round" ? roundNo(s) : s.type === "strikes" || s.type === "morale" ? 0 : c.length - 1;
      pl.style.display = "";
      pl.style[fromLeft ? "left" : "right"] = w(c[Math.min(step, c.length - 1)]);
      pl.style[fromLeft ? "right" : "left"] = "auto";
    } else pl.style.display = "none";
    const n = document.getElementById(`bt-sn${side}`);
    const txt = `${hpOf(now)} / ${hpOf(max)}`;
    if (n && n.textContent !== txt) n.textContent = txt;
    const lk = document.getElementById(`bt-lk${side}`);
    const [sum, cnt] = st.luck[side];
    const lt = cnt ? `luck ×${(sum / cnt / 100).toFixed(2)}` : "";
    if (lk && lk.textContent !== lt) lk.textContent = lt;
  }
}

/** The hit card: one hit a round dissected (who, why him, why that much, the parts). */
function syncCard(ctx, s, t) {
  const card = document.getElementById("bt-hitcard");
  if (!card) return;
  const L = W.log;
  const F = L.fighters;
  const fi = W.focusOverride ?? s.focus;
  const key = `${W.si}:${fi}:${W.cardOpen ? 1 : 0}`;
  if (card.dataset.k === key) return;
  card.dataset.k = key;
  if (s.type !== "round" || fi === undefined || fi < 0) {
    if (s.type === "strikes") {
      const strikers = [...new Map(s.strikes.filter((x) => x.ev.kind === "DefStrike").map((x) => [F[x.ev.striker].name, F[x.ev.striker]])).values()];
      refill(card, 
        h("div.bt-h4", "The opening"),
        h("div", `Defensive strikes: each attacker is shot at once, as it comes in, by the best of their archers that rolls under its chance${strikers.length ? ` (${strikers.map((f) => `${f.name} ${f.def_strike?.[0] ?? "?"}%`).join(", ")})` : ""}. Armour takes most of it off.`),
      );
    } else if (s.type === "end" || s.type === "post") refill(card, h("div.bt-h4", resultWords(L).ribbon), h("div", s.caption ?? ""));
    else refill(card, h("div.bt-h4", s.type === "morale" ? "Before the battle" : ""), h("div", s.caption ?? ""));
    return;
  }
  const x = s.hits[fi];
  const e = x.ev;
  const p = pairOf(L.pairs, x.h, x.t);
  const hf = F[x.h];
  const tf = F[x.t];
  if (!p) {
    refill(card, h("div", `${uname(L, x.h)} → ${uname(L, x.t)}: ${hpOf(x.dmg)} damage`));
    return;
  }
  const steps = (hf.targets ?? []).map((c, k) => {
    const blk = (e.blocked ?? []).find((b) => b[0] === k);
    const used = k === e.step;
    return h(`span.bt-step${used ? ".used" : ""}${blk ? ".gone" : ""}`, { title: blk ? BLOCK_LABEL[blk[1]] ?? blk[1] : "" }, `${k + 1} ${CLASS_LABEL[c] ?? c}`, blk ? h("span.small", ` ${BLOCK_LABEL[blk[1]] ?? blk[1]}`) : used ? " ✓" : "");
  });
  const share = p.a_x100 / (p.a_x100 + p.dv_x100);
  const lit = share * 20;
  const withLuck = (p.mean * e.r) / 100 / 100;
  const meter = h("span.bt-meter");
  for (let k = 0; k < 20; k++) {
    const on = k < Math.floor(lit) || (k === Math.floor(lit) && lit % 1 >= 0.5);
    const luck = !on && k < Math.round(withLuck);
    const lost = on && k >= Math.round(withLuck);
    add(meter, h(`i${luck ? ".luck" : lost ? ".lost" : on ? ".on" : ""}`));
  }
  const rnd = e.eligible.length > 1 ? h("span.cyan.small", `1 of ${e.eligible.length}, random`) : h("span.dim.small", "the only one");
  const aT = [...hf.a.terms, ...(p.a_extra ?? [])];
  const dvT = [...tf.dv.terms, ...(p.dv_extra ?? [])];
  const mx = Math.max(p.a_x100, p.dv_x100) * 1.05;
  const lbl = (txt) => h("span.small.dim.bt-ell", { style: { maxWidth: "290px" } }, txt);
  const roleName = (f) => `${f.name} ${x100(f.a.base_x100)} × ${(f.a.role_pct / 100).toFixed(1)}`;
  refill(card, 
    h(
      "div.bt-row",
      h(`b.${hf.side === W.me ? "you" : "them"}`, uname(L, x.h)),
      h("span.gold", "→"),
      h(`b.${tf.side === W.me ? "you" : "them"}`, uname(L, x.t)),
      h("span.dim", `· ${s.kind === "volley" ? "volley" : `round ${s.n}`}`),
      h("span", { style: { marginLeft: "auto" } }, h("b.goldlt", String(hpOf(x.dmg))), h("span.dim", " damage")),
    ),
    h("div.bt-row", h("span.dim", "Why him?"), h("div.bt-ladder", steps), rnd),
    h("div.bt-row", h("span.dim", `Why ${hpOf(x.dmg)}?`), h("span.amber", `A ${x100(p.a_x100)}`), h("span.dim", "vs"), h("span.frost", `Dv ${x100(p.dv_x100)}`), meter),
    h(
      "div.bt-row",
      { style: { height: "16px" } },
      h("span.small.dim", `20 × ${x100(p.a_x100)} ÷ (${x100(p.a_x100)} + ${x100(p.dv_x100)}) = ${(p.mean / 100).toFixed(1)} × luck `, h("span.lilac", (e.r / 100).toFixed(2)), ` = ${hpOf(x.dmg)}${e.capped ? " (cut at its limit)" : ""}`),
      h("span", { style: { marginLeft: "auto" } }, btn(W.cardOpen ? "hide parts" : "show parts", { cls: "tiny", onclick: () => ((W.cardOpen = !W.cardOpen), (card.dataset.k = "")) })),
    ),
    W.cardOpen ? h("div.bt-row", { style: { height: "18px" } }, h("span.amber.small", { style: { width: "16px" } }, "A"), partsBar({ ...hf.a, terms: aT, total_x100: p.a_x100 }, "a", mx, 300), lbl(aT.map((x2) => `${termLabel(x2)} ${pctText(x2.pct)}`).join(" · ") || roleName(hf))) : null,
    W.cardOpen ? h("div.bt-row", { style: { height: "18px" } }, h("span.frost.small", { style: { width: "16px" } }, "Dv"), partsBar({ ...tf.dv, terms: dvT, total_x100: p.dv_x100 }, "dv", mx, 300), lbl(dvT.map((x2) => `${termLabel(x2)} ${pctText(x2.pct)}`).join(" · ") || "no bonuses")) : null,
  );
}

/** The banner at the end, the caption, the play button, the timeline's head. */
function syncMisc(ctx, s, t) {
  const L = W.log;
  let ban = document.getElementById("bt-banner");
  const wantBan = s.type === "end" || s.type === "post";
  if (wantBan && !ban) {
    const rw = resultWords(L);
    ban = h("div.bt-pnl.bt-banner.bt-ribbon#bt-banner", { style: { top: "70px" } }, h(rw.won ? "span.goldlt" : "span", rw.banner));
    add(ctx.ui, ban);
  } else if (!wantBan && ban) ban.remove();
  const cap = document.getElementById("bt-caption");
  if (cap && cap.dataset.si !== String(W.si)) {
    cap.dataset.si = String(W.si);
    cap.textContent = s.caption ?? "";
    cap.style.display = s.caption ? "" : "none";
  }
  const pb = document.getElementById("bt-play");
  const want = W.playing && W.callout?.mode !== "pause" ? "pause" : "play";
  if (pb && pb.dataset.s !== want) {
    pb.dataset.s = want;
    pb.replaceChildren(pixGlyph(want === "pause" ? GLYPH.pause : GLYPH.play, "#fff8e4", 3));
  }
  const head = document.getElementById("bt-tlhead");
  const fill = document.getElementById("bt-tlfill");
  if (head && fill) {
    const n = Math.max(1, W.script.segs.length - 1);
    const x = Math.min(100, (100 * (W.si + Math.min(1, t / s.dur))) / n);
    head.style.left = `${x}%`;
    fill.style.width = `${x}%`;
  }
}

/** Hooks for the screenshot and smoke scripts. */
export const dev = {
  seek(si, t = 0) {
    seek(si, t);
    W.t = t;
  },
  pause() {
    W.playing = false;
  },
  play() {
    W.playing = true;
  },
  /** The first segment matching `pred(seg)`. */
  segOf(pred) {
    return W.script ? W.script.segs.findIndex(pred) : -1;
  },
  openCard(v) {
    W.cardOpen = v;
  },
  state() {
    return { ready: W.ready, si: W.si, t: W.t, segs: W.script?.segs.length ?? 0, callout: W.callout?.id ?? null, playing: W.playing, mode: W.mode };
  },
  get script() {
    return W.script;
  },
};
