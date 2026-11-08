// The watch screen's presentation of a battle (docs/HUMAN.md, "The battle screens"): the owner's
// pick from the battle-feel spike (2026-10-31), "Snap, shade on crits and kills (no zoom)".
//
// Every standing unit breathes (a slow vertical squash). An action is a snap with no wind-up: on
// its contact frame both fighters cut to their impact poses (the strike strip's contact frame for a
// unit with an anim-sprint set, else its sprite leaning forward and a little larger; the target's
// flinch frame, else a lean back under a white flash) and slide toward each other, and the slash
// or the sparks, the damage number and a small shake land on that same frame. A held beat follows
// (0.5 to 1.1 s by kind) with embers or dust drifting and the breathing going on, then a quarter
// second's ease back to the cells. A crit holds longer; a kill holds on the contact frame and its
// death plays after the return; a miss is a dodge with a whiff; a heal glows with motes; a shot or
// a spell snaps the shooter on its own cell with the shot arriving as a streak, or, for a kind with
// an impact card (a bombard: the rubble), with one still image of the blow arriving fast over the
// target, braking hard, held and fading on the return (a unit with a card of its own, the ranger's
// hawk, strikes that way from its cell); splash targets snap
// together; a knock-back eases its target back a row during the return; one that escapes a killing
// blow leaves the field after it. On crits and kills only, the rest of the field drops into a shade
// round a pool of light on the pair. The camera never zooms; under reduced motion nothing shakes.
// No narrator's line (the owner's call; the spike had one on crits and kills).
//
// Key frames, not motion (the owner's call, 2026-11-05, on the catapult's rubble): each fighter of a
// snap holds one frame for the whole beat (the strike strip's blow and the flinch's recoil, or a
// unit's key poses: `attack` and `hit`, app/pose), and the blow itself is one held image (the impact
// card, app/fx). anim-sprint/ANIMATING_UNITS.md, "Impact cards and key poses", says how to add them.
//
// While it acts, the pair is lifted out of the board: the stage draws its rings but not its
// figures (`lifts`), and `drawOver` draws them above everything, the shade included, with an ink
// outline and their HP bars riding with them. The watch screen drives it with its replay clock (ms
// at 1x; it stops while the replay is paused): `begin` as an action starts, its event folded in at
// the plan's `impact` (the contact frame), its fates (the deaths, the escapes) shown at `fates`.
//
// The pure parts (an action's timing plan, which actions get the shade, the roles, their places
// and their poses) are tested in node (assets/test/battle_snap.test.mjs); the drawing needs a
// canvas.

import { kindOf, kindKey, dmgText } from "./battle_log.js";
import { drawPicX, leanTurn, silhouette, impactPic, STRIKE_HIT, FRAME_W, FRAME_H } from "./battle_stage.js";

/** ms of stillness before the cut (the snap needs a still frame to contrast with). */
export const LEAD = 20;
const BREATH_MS = 3000;
const BREATH_AMP = 0.02;
/** How long the shake on the contact frame lasts (ms). */
const SHAKE_MS = 220;
/** The strike strip's frames the snap holds (ms into the strip, ANIMS.strike): the cut (frame 3,
 *  the swing's last smear), the hold (frame 4, the blow, STRIKE_HIT) and the way back (frame 6). */
const STRIKE_MS = { cut: STRIKE_HIT - 30, hold: STRIKE_HIT + 10, back: 700 };
/** The flinch strip's (ANIMS.flinch): the hold and a slain unit's whole stay (frame 1, the
 *  recoil), then the recovery on the way back (frames 2 and 3). */
const FLINCH_MS = { hold: 140, back1: 260, back2: 380 };

/** The impact card each staged kind lands with (app/fx; anim-sprint/lib/impact_cards.py); a kind
 *  without one arrives as a streak. */
export const IMPACT_CARDS = { Bombard: "rubble" };
/** The impact card plan `P` lands with (the actor's own, else its kind's), or null. */
export const impactOf = (P) => (P?.snap && P.ranged ? P.card ?? IMPACT_CARDS[P.kind] ?? null : null);
/** Cards drawn facing right, mirrored for a blow toward the left. */
const DIRECTED_CARDS = new Set(["hawk"]);
/** How the card arrives (ms at 1x): its approach decays as exp(-t / BRAKE), so it covers most of
 *  the way in the first two frames and then all but stops; it starts ARRIVE px back along the line
 *  of the blow and RISE px up, a third larger (GROW), trailing ghosts while it is still fast, then
 *  settles DRIFT px a second. */
const CARD = { BRAKE: 55, ARRIVE: 150, RISE: 70, GROW: 0.3, DRIFT: 4 };
/** The card's size for a blow that strikes the units at `pts` (their feet): one unit's at 1x, a
 *  splash's over the group, 2x or 3x by how wide it spreads (whole-pixel steps keep the pixel art
 *  even). */
export function cardScale(pts) {
  if (pts.length < 2) return 1;
  const xs = pts.map((p) => p[0]);
  return Math.max(...xs) - Math.min(...xs) > 150 ? 3 : 2;
}

const lerp = (a, b, k) => a + (b - a) * k;
const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
export const ease = {
  out: (t) => 1 - (1 - t) * (1 - t),
  inOut: (t) => (t < 0.5 ? 2 * t * t : 1 - 2 * (1 - t) * (1 - t)),
  outBack: (t) => 1 + 2.7 * Math.pow(t - 1, 3) + 1.7 * Math.pow(t - 1, 2),
};

/** A small seeded generator (mulberry32): the same action draws the same sparks every time. */
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** The breathing of fighter `b` of side `side` at wall time `wall`: its vertical scale. */
export function breath(b, side, wall) {
  return 1 + BREATH_AMP * Math.sin((wall / BREATH_MS) * Math.PI * 2 + b * 0.83 + (side === 1 ? 1.7 : 0));
}

// ---------------------------------------------------------------------------------------------
// The plan of an action

/** The action kind an event is staged as: a defensive strike is a shot (its striker may stand in
 *  the reserve, off the field). */
export const stagedKind = (e) => (e.kind === "DefStrike" ? "Shoot" : kindKey(e.kind));

/** How event `e` plays at 1x (ms); `card`: the actor's own impact card (its blows land as it,
 *  from its cell: staged as ranged, timed as its kind). `snap: false` for an event with no blow (a unit afraid, an
 *  action with no one in reach): `impact` when it is folded in, `end` when the next may start.
 *  Else the snap's: `impact` (= LEAD, the contact frame), `hold` (the held beat), `ret` (the ease
 *  back), `rest` (stillness before the next cut), `fates` (when its deaths and escapes show: the
 *  return's end), `end`, `shake` (px), `shade` (a crit or a kill), and what it is (`kind`,
 *  `melee`, `ranged`, `heal`, `crit`, `kill`, `miss`, `escape`). */
export function planOf(e, card = null) {
  if (e.kind === "Afraid" || e.kind === "Building") return { snap: false, impact: 300, end: 800 };
  const hits = e.hits ?? [];
  if (!hits.length && !e.heal) return { snap: false, impact: 120, end: 520 };
  const kind = stagedKind(e);
  const K = kindOf(kind);
  const heal = !!e.heal && !hits.length;
  const meleeKind = !heal && !!K.melee;
  const melee = meleeKind && !card;
  const crit = hits.some((h) => h.crit && !h.miss);
  const kill = hits.some((h) => h.killed);
  const miss = hits.length > 0 && hits.every((h) => h.miss);
  const escape = !kill && hits.length > 0 && !!hits[0].withdrew;
  let hold;
  let ret = 240;
  let rest = 160;
  let shake;
  if (heal) [hold, shake] = [560, 0];
  else if (miss) [hold, ret, rest, shake] = [480, 220, 140, 0];
  else if (crit) [hold, ret, shake] = [kill ? 1150 : 1050, 280, 7];
  else if (kill) [hold, ret, shake] = [900, 260, 5];
  else if (escape) [hold, shake] = [720, 3];
  else if (meleeKind) [hold, shake] = [680, 3];
  // a bombard's card needs a beat longer to read
  else [hold, shake] = kind === "Bombard" ? [760, 5] : [620, 3];
  // a kill leaves room for the fall to read before the next cut
  if (kill) rest = 440;
  return { snap: true, kind, card: heal ? null : card, melee, ranged: !melee && !heal, heal, crit, kill, miss, escape, shade: crit || kill, hold, ret, rest, shake, impact: LEAD, fates: LEAD + hold + ret, end: LEAD + hold + ret + rest };
}

/** Whether event `e` drops the rest of the field into the shade: a crit or a kill. */
export const shaded = (e) => !!planOf(e).shade;

/** The snap's phase `t` ms after the contact frame of plan `P`: `{t, k, q, c, hold}` (`k` 1
 *  through the hold, easing to 0 over the return; `q` = 1 - k; `c` the hold's progress), or null
 *  before the cut and after the return. */
export function phaseOf(P, t) {
  if (!P?.snap || t < 0 || t >= P.hold + P.ret) return null;
  if (t < P.hold) return { t, k: 1, q: 0, c: t / P.hold, hold: true };
  const q = ease.inOut((t - P.hold) / P.ret);
  return { t, k: 1 - q, q, c: 1, hold: false };
}

/** The units event `e` stages and their roles: a Map fighter -> `{b, kind, primary, knock, h}`.
 *  The actor's kind: "melee", "shoot", "bombard", "spell" or "healer"; a target's: "hit", "kill",
 *  "miss", "escape" or "healed" (`primary`: the first target; the rest are its splash). */
export function rolesOf(e, card = null) {
  const P = planOf(e, card);
  const roles = new Map();
  if (!P.snap) return roles;
  if (e.actor !== null && e.actor !== undefined) {
    const kind = P.heal ? "healer" : P.melee ? "melee" : { Shoot: "shoot", Bombard: "bombard", Spell: "spell" }[P.kind] ?? "shoot";
    roles.set(e.actor, { b: e.actor, kind, actor: true, primary: false, knock: false, h: null });
  }
  const list = P.heal ? [{ target: e.heal.target, heal: e.heal.amount }] : e.hits;
  list.forEach((h, n) => {
    const kind = h.heal !== undefined ? "healed" : h.miss ? "miss" : h.withdrew ? "escape" : h.killed ? "kill" : "hit";
    roles.set(h.target, { b: h.target, kind, actor: false, primary: n === 0, knock: e.knocked?.unit === h.target, h });
  });
  return roles;
}

/** The staging of event `e`: its roles with their places (`home`: the feet at the start; `off`:
 *  the slide on the cut; `creep`: the drift through the hold; `ease`: ms to reach the slide, for a
 *  dodge), the line of the blow `dir`, the `contact` point, and the sparks and motes; null when
 *  it has no blow or nobody to stage. `at(b)`: fighter b's feet `{x, y, large, field}` (`field`
 *  false: off the field, a striker in the reserve: its shot starts there, but it is not staged);
 *  `seed`: the action's. */
export function stageOf(e, at, seed, card = null) {
  const P = planOf(e, card);
  if (!P.snap) return null;
  const R = rng(seed);
  const all = rolesOf(e, card);
  const roles = new Map();
  for (const [b, r] of all) {
    const p = at(b);
    if (p && p.field !== false) roles.set(b, { ...r, home: [p.x, p.y], large: !!p.large, off: [0, 0], creep: [0, 0], ease: 0 });
  }
  const aAt = e.actor !== null && e.actor !== undefined ? at(e.actor) : null;
  const first = [...all.values()].find((r) => !r.actor);
  const tAt = at(first.b) ?? aAt;
  if (!tAt) return null;
  const A = aAt ?? tAt;
  // the line of the blow, from the attacker's feet to the target's
  let dx = tAt.x - A.x;
  let dy = tAt.y - A.y;
  const L = Math.hypot(dx, dy) || 1;
  dx /= L;
  dy /= L;
  const sep = 62 + (A.large ? 30 : 0) + (tAt.large ? 30 : 0);
  const gap = Math.max(0, L - sep);
  const dodgeSign = R() < 0.5 ? 1 : -1;
  const ar = e.actor !== null && e.actor !== undefined ? roles.get(e.actor) : null;
  if (ar && ar.actor) {
    if (P.melee) [ar.off, ar.creep] = [[dx * gap * 0.78, dy * gap * 0.78], [dx * 6, dy * 6]];
    else if (P.heal) [ar.off, ar.creep] = [[0, -4], [0, -2]];
    else if (P.kind === "Bombard") [ar.off, ar.creep] = [[dx * 10, dy * 10], [dx * 5, dy * 5]];
    // one whose own card strikes (the ranger loosing his hawk) steps into it
    else if (P.card) [ar.off, ar.creep] = [[dx * 6, dy * 6], [dx * 3, dy * 3]];
    else [ar.off, ar.creep] = [[-dx * 4, -dy * 4], [-dx * 2, -dy * 2]];
  }
  for (const r of roles.values()) {
    if (r.actor) continue;
    if (r.kind === "healed") [r.off, r.creep] = [[0, -3], [0, -3]];
    else if (r.kind === "miss" || r.kind === "escape") {
      const nx = -dy * dodgeSign;
      const ny = dx * dodgeSign;
      const back = r.kind === "escape" ? 22 : 5;
      const side = r.kind === "escape" ? 6 : 17;
      r.off = [nx * side + dx * back, ny * side + dy * back];
      r.creep = [dx * 3, dy * 3];
      r.ease = 70;
      if (P.melee && r.primary) [r.off[0], r.off[1]] = [r.off[0] - dx * gap * 0.1, r.off[1] - dy * gap * 0.1];
    } else if (P.melee && r.primary) [r.off, r.creep] = [[-dx * gap * 0.22, -dy * gap * 0.22], [dx * 7, dy * 7]];
    else [r.off, r.creep] = [[dx * 5, dy * 5], [dx * 4, dy * 4]];
  }
  const held = (r) => [r.home[0] + r.off[0], r.home[1] + r.off[1]];
  const tr = roles.get(first.b);
  const [tx, ty] = tr ? held(tr) : [tAt.x, tAt.y];
  const [ax, ay] = ar ? held(ar) : [A.x, A.y];
  // where the blow lands: on the primary's torso, a little toward the attacker for melee
  const contact = P.melee ? [lerp(tx, ax, 0.22), lerp(ty, ay, 0.22) - 50] : [tx, ty - 46];
  // sparks off the contact; embers, dust or motes drifting through the hold
  const sparks = [];
  const nS = P.miss || P.heal ? 0 : P.crit ? 16 : P.kill ? 12 : 9;
  for (const r of roles.values()) {
    if (r.actor || r.kind === "miss" || r.kind === "healed" || r.kind === "escape") continue;
    const c = r.primary ? contact : [held(r)[0], held(r)[1] - 46];
    // the impact card is the blow: no sparks over it
    const k = impactOf(P) ? 0 : r.primary ? nS : Math.ceil(nS / 2);
    for (let s = 0; s < k; s++) {
      const a = Math.atan2(dy, dx) + (R() - 0.5) * 2.4;
      const v = 260 + R() * 320;
      sparks.push({ x: c[0], y: c[1], vx: Math.cos(a) * v, vy: Math.sin(a) * v * 0.8 - 60, gold: R() < 0.5, blood: P.kill && r.kind === "kill" && R() < 0.35 });
    }
  }
  const tone = toneOf(e, P);
  const motes = [];
  const mc = P.heal ? [tAt.x, tAt.y - 40] : contact;
  for (let s = 0, nM = P.crit ? 22 : 14; s < nM; s++) {
    motes.push({ x: mc[0] + (R() - 0.5) * 200, y: mc[1] + (R() - 0.5) * 110, vy: -(10 + R() * 22), vx: (R() - 0.5) * 10, sw: 3 + R() * 6, ph: R() * 6.28, sz: R() < 0.3 ? 3 : 2, col: tone.motes[Math.floor(R() * tone.motes.length)], fade: 0.5 + R() * 0.5 });
  }
  return { P, roles, dir: [dx, dy], A: [A.x, A.y], T: [tAt.x, tAt.y], contact, sparks, motes, tone, dodgeSign, tilt: (R() - 0.5) * 1.2 };
}

/** The colours of an action: a heal's green, a damage spell's cyan and the magic bolt's lilac,
 *  the dust of a miss, else embers. `rgb` the flare's, `light` a glow's, `motes` the drift's. */
export function toneOf(e, P = planOf(e)) {
  if (P.heal) return { rgb: "125, 255, 176", light: "#a8ffcc", motes: ["#a8ffcc", "#e6fff0", "#7dffb0"] };
  if (P.kind === "Spell") {
    return e.spell !== null && e.spell !== undefined ? { rgb: "140, 235, 255", light: "#9feeff", motes: ["#9feeff", "#e0fbff", "#5cc8ff"] } : { rgb: "205, 165, 255", light: "#d8b8ff", motes: ["#d8b8ff", "#f4eaff", "#a77cff"] };
  }
  if (P.miss) return { rgb: "217, 205, 176", light: "#efe6d0", motes: ["#d9cdb0", "#bfb299", "#efe6d0"] };
  if (P.kind === "Bombard") return { rgb: "200, 180, 150", light: "#efe6d0", motes: ["#d9cdb0", "#c9a46a", "#a3804d", "#efe6d0"] };
  return { rgb: "255, 240, 200", light: "#ffd890", motes: ["#ffb347", "#ffd890", "#ff7b3a", "#fff2c0"] };
}

/** The pose of a unit in role `r` (`{kind, primary}`, and `tone` for a caster) at phase `ph`:
 *  `anim`/`ms` a frame of its strip (when `animated`: `clean` drops the strike frames' smear), or
 *  one of its key poses (`keys`, a Set: "attack" for the actor, "hit" for one struck), else its
 *  sprite under `lean` (radians, + forward) and `scale`; and `tint`/`tintA`, `glow`. */
export function poseOf(r, ph, animated, tone = null, keys = null) {
  const p = { anim: null, ms: 0, clean: false, lean: 0, scale: 1, tint: null, tintA: 0, glow: null };
  const { t, k, q, c } = ph;
  switch (r.kind) {
    case "melee":
      if (animated) Object.assign(p, { anim: "strike", clean: true, ms: t < 50 ? STRIKE_MS.cut : q > 0.5 ? STRIKE_MS.back : STRIKE_MS.hold });
      else Object.assign(p, { lean: 0.24 * k, scale: 1 + 0.08 * k });
      break;
    case "shoot":
      Object.assign(p, { lean: -0.07 * k, scale: 1 + 0.04 * k });
      break;
    case "bombard":
      Object.assign(p, { lean: -0.05 * k, scale: 1 + 0.04 * k });
      break;
    case "spell":
    case "healer": {
      const col = tone?.light ?? (r.kind === "healer" ? "#a8ffcc" : "#d8b8ff");
      Object.assign(p, { scale: 1 + 0.05 * k, glow: k > 0.4 ? col : null, tint: col, tintA: (0.35 - 0.2 * c) * k });
      break;
    }
    case "hit":
    case "kill": {
      // a hit recovers through the flinch's last frames on the way back; the slain keep the
      // recoil and fall after the return
      if (animated) Object.assign(p, { anim: "flinch", ms: ph.hold || r.kind === "kill" ? FLINCH_MS.hold : q < 0.5 ? FLINCH_MS.back1 : FLINCH_MS.back2 });
      else p.lean = -(r.kind === "kill" ? 0.26 : 0.2) * k;
      const flash = r.primary ? 150 : 100;
      if (t < flash) Object.assign(p, { tint: "#ffffff", tintA: 0.9 * (1 - t / flash) });
      else if (r.kind === "kill") Object.assign(p, { tint: "#ff3a2a", tintA: (0.28 + 0.1 * Math.sin(t / 90)) * k });
      break;
    }
    case "miss":
    case "escape":
      p.lean = -0.13 * k * (r.kind === "escape" ? 1.4 : 1);
      if (r.kind === "escape") Object.assign(p, { tint: "#bfe3ff", tintA: 0.3 * k });
      break;
    case "healed":
      Object.assign(p, { glow: k > 0.4 ? "#7dffb0" : null, tint: "#7dffb0", tintA: (0.3 - 0.15 * c) * k });
      break;
  }
  // a key pose replaces the lean: the one frame held for the whole snap, the return included
  const key = ACTOR_KINDS.has(r.kind) ? "attack" : r.kind === "hit" || r.kind === "kill" ? "hit" : null;
  if (key && !p.anim && keys?.has(key)) Object.assign(p, { anim: key, ms: 0, lean: 0, scale: 1 });
  return p;
}

const ACTOR_KINDS = new Set(["melee", "shoot", "bombard", "spell", "healer"]);

/** The camera's shake `t` ms after the contact frame of plan `P`: `[dx, dy]` px, or null. */
export function shakeAt(P, t) {
  if (!P?.snap || !P.shake || t < 0 || t >= SHAKE_MS) return null;
  const a = P.shake * Math.pow(1 - t / SHAKE_MS, 2);
  return [a * Math.sin(t * 0.13 + 1.1), a * 0.7 * Math.cos(t * 0.17 + 0.4)];
}

// ---------------------------------------------------------------------------------------------
// The snap on the stage

/** The strike frames carry a translucent smear behind the blade; frozen on the field it reads as
 *  a grey slab, and the slash replaces it. `destination-in` with the frame itself raises alpha to
 *  the 4th power: the smear (alpha about 0.5) all but vanishes, the solid pixels stay. */
const CLEAN = new Map();
function clean(p) {
  if (!p?.anim) return p;
  const key = `${p.im.src}|${p.sx}`;
  let c = CLEAN.get(key);
  if (!c) {
    c = document.createElement("canvas");
    c.width = p.w;
    c.height = p.h;
    // the silhouettes' cache keys on `src`
    c.src = `clean:${key}`;
    const g = c.getContext("2d");
    g.drawImage(p.im, p.sx, 0, p.w, p.h, 0, 0, p.w, p.h);
    g.globalCompositeOperation = "destination-in";
    for (let k = 0; k < 3; k++) g.drawImage(p.im, p.sx, 0, p.w, p.h, 0, 0, p.w, p.h);
    CLEAN.set(key, c);
  }
  return { im: c, sx: 0, w: p.w, h: p.h, ax: p.ax, ay: p.ay };
}

const FONT = "Herald, Georgia, serif";

export class Snap {
  /** `env`, the watch screen's battle: `feet(b)` -> [x, y] (its cell now, the knock-backs so far
   *  included), `onField(b)`, `large(b)`, `side(b)`, `pic(b, anim, ms, loop)` -> a picture (its
   *  sprite when it has no such strip) or null, `animated(b)`, `flip(b)` (drawn mirrored),
   *  `leftArt(b)` (its art faces left: a lean follows the way it faces as drawn, `leanTurn`),
   *  `bar(b)` -> `{hp, max}` (null: no bar), `reduced()` (reduced motion). */
  constructor(env) {
    this.env = env;
    this.cur = null;
  }

  /** Nothing staged (a seek, a replay from the start). */
  reset() {
    this.cur = null;
  }

  /** Stages event `e`, the log's `seq`th, starting at replay time `now`; returns its plan
   *  (`planOf`: times after `now`). */
  begin(e, now, seq) {
    const card = e.actor !== null && e.actor !== undefined ? this.env.card?.(e.actor) ?? null : null;
    const P = planOf(e, card);
    this.cur = null;
    if (!P.snap) return P;
    const env = this.env;
    const at = (b) => {
      const [x, y] = env.feet(b);
      return { x, y, large: env.large(b), field: env.onField(b) };
    };
    const S = stageOf(e, at, (seq + 1) * 7919 + (e.actor ?? 0) * 131 + 17, card);
    if (S) this.cur = { e, t0: now + P.impact, ...S };
    return P;
  }

  /** The phase of the staged action at replay time `now` (`phaseOf`), or null. */
  phase(now) {
    return this.cur ? phaseOf(this.cur.P, now - this.cur.t0) : null;
  }

  /** Whether fighter `b` is lifted out of the board at `now` (drawn by `drawOver`, not the stage). */
  lifts(b, now) {
    return !!this.phase(now) && this.cur.roles.has(b);
  }

  /** The stage's shake at `now`, or null. */
  shake(now) {
    if (!this.cur || this.env.reduced()) return null;
    return shakeAt(this.cur.P, now - this.cur.t0);
  }

  /** What is staged at `now`, for the dev hooks: `{kind, shade, crit, kill, roles: {b: role's
   *  kind}, t (ms after the contact frame), hold}`, or null. */
  info(now) {
    const ph = this.phase(now);
    if (!ph) return null;
    const C = this.cur;
    return { kind: C.P.kind, shade: C.P.shade, crit: C.P.crit, kill: C.P.kill, roles: Object.fromEntries([...C.roles].map(([b, r]) => [b, r.kind])), t: ph.t, hold: ph.hold };
  }

  /** Draws the staged action over the stage `stage` (after its `draw`): the shade, the lifted pair,
   *  the blow's marks, the numbers. `now` replay time, `wall` the page's (the breathing). */
  drawOver(stage, now, wall) {
    const ph = this.phase(now);
    if (!ph) return;
    const C = this.cur;
    const g = stage.g;
    const [sx, sy] = (this.shake(now) ?? [0, 0]).map(Math.round);
    g.save();
    g.setTransform(1, 0, 0, 1, 0, 0);
    if (C.P.shade) this.drawShade(g, ph, sx, sy);
    g.setTransform(1, 0, 0, 1, sx, sy);
    const order = [...C.roles.values()].sort((a, b) => a.home[1] + a.off[1] - (b.home[1] + b.off[1]));
    this.drawImpact(g, ph, "back");
    for (const r of order) this.drawRole(stage, g, r, ph, wall);
    this.drawImpact(g, ph, "front");
    this.drawMarks(g, ph);
    this.drawTexts(g, ph);
    g.restore();
  }

  /** Layer `layer` ("back": behind the fighters; "front") of the action's impact card over its
   *  primary target: in from the shooter's side, braking hard, a slow settle through the hold, a
   *  fade over the return. It lands where the target stood (a dodge leaves it beside the card); a
   *  splash's lands larger (cardScale) in the middle of the units it strikes. */
  drawImpact(g, ph, layer) {
    const C = this.cur;
    const card = impactOf(C.P);
    const im = card ? impactPic(card, layer) : null;
    if (!im) return;
    const first = [...C.roles.values()].find((r) => !r.actor && r.primary);
    const struck = [...C.roles.values()].filter((r) => !r.actor && r.kind !== "miss" && r.kind !== "escape").map((r) => this.heldAt(r, ph));
    const big = cardScale(struck);
    let [fx, fy] = first && first.kind !== "miss" && first.kind !== "escape" ? this.heldAt(first, ph) : C.T;
    if (big > 1) [fx, fy] = [struck.reduce((a, p) => a + p[0], 0) / struck.length, struck.reduce((a, p) => a + p[1], 0) / struck.length + 24];
    const [dx, dy] = C.dir;
    const e = Math.exp(-ph.t / CARD.BRAKE);
    const back = layer === "back";
    // the dust keeps billowing after the stones have stopped
    const s = big * (back ? 0.9 + 0.2 * (1 - Math.exp(-ph.t / 260)) : 1 + CARD.GROW * e);
    const ox = -dx * CARD.ARRIVE * e * (back ? 0.5 : 1);
    const oy = (-dy * CARD.ARRIVE - CARD.RISE) * e * (back ? 0.4 : 1) + (CARD.DRIFT * ph.t) / 1000;
    const w = Math.round(im.width * s);
    const h = Math.round(im.height * s);
    const at = (k) => [Math.round(fx + ox * k - w / 2), Math.round(fy + 2 + oy * k - h)];
    // a directed card (drawn facing right) is mirrored about its own centre for a blow toward the left
    const mirror = DIRECTED_CARDS.has(card) && dx < 0;
    const put = (k) => {
      const [x, y] = at(k);
      if (!mirror) return g.drawImage(im, x, y, w, h);
      g.save();
      g.translate(x + w, y);
      g.scale(-1, 1);
      g.drawImage(im, 0, 0, w, h);
      g.restore();
    };
    g.save();
    g.imageSmoothingEnabled = false;
    const fade = ph.hold ? 1 : ph.k;
    // ghosts along the way it came, while it is still fast
    if (!back && e > 0.35) {
      for (const n of [3, 2, 1]) {
        g.globalAlpha = fade * (0.05 + (0.18 * n) / 3);
        put(1 + 0.35 * n);
      }
    }
    g.globalAlpha = fade;
    put(1);
    g.restore();
  }

  /** A soft pool of light on the pair, the rest of the field dropped into the dark. */
  drawShade(g, ph, sx, sy) {
    const pts = [...this.cur.roles.values()].map((r) => [r.home[0] + r.off[0], r.home[1] + r.off[1] - 50]);
    if (!pts.length) return;
    const cx = pts.reduce((a, p) => a + p[0], 0) / pts.length;
    const cy = pts.reduce((a, p) => a + p[1], 0) / pts.length;
    const spread = Math.max(...pts.map((p) => Math.hypot(p[0] - cx, (p[1] - cy) * 1.6)));
    const r0 = spread + 50;
    const gr = g.createRadialGradient(cx + sx, cy + sy, r0 * 0.5, cx + sx, cy + sy, r0 + 260);
    gr.addColorStop(0, `rgba(5, 4, 14, ${0.3 * ph.k})`);
    gr.addColorStop(0.45, `rgba(5, 4, 14, ${0.52 * ph.k})`);
    gr.addColorStop(1, `rgba(5, 4, 14, ${0.8 * ph.k})`);
    g.fillStyle = gr;
    g.fillRect(0, 0, FRAME_W, FRAME_H);
  }

  drawRole(stage, g, r, ph, wall) {
    const env = this.env;
    const b = r.b;
    const [fx, fy] = env.feet(b);
    const sk = r.ease ? ease.out(Math.min(1, ph.t / r.ease)) : 1;
    const held = [r.home[0] + r.off[0] * sk + r.creep[0] * ph.c, r.home[1] + r.off[1] * sk + r.creep[1] * ph.c];
    // the return goes to the cell it stands on now (a row back after a knock-back, with a hop)
    const pos = ph.hold ? held : [lerp(held[0], fx, ph.q), lerp(held[1], fy, ph.q) - (r.knock ? Math.sin(Math.PI * ph.q) * 16 : 0)];
    const animd = env.animated(b);
    const po = poseOf(r, ph, animd, this.cur.tone, env.keys?.(b) ?? null);
    let pic = po.anim ? env.pic(b, po.anim, po.ms, false) : null;
    if (pic && po.clean) pic = clean(pic);
    pic = pic ?? env.pic(b, animd ? "idle" : null, wall + b * 137, true);
    if (!pic) return;
    const flip = env.flip(b);
    const left = env.leftArt(b);
    const sy = breath(b, env.side(b), wall);
    // a contact shadow once the figure has left its cell
    const moved = Math.hypot(pos[0] - fx, pos[1] - fy);
    if (moved > 6) {
      g.save();
      g.globalAlpha = Math.min(0.32, moved / 60);
      g.fillStyle = "#05040c";
      g.beginPath();
      g.ellipse(pos[0], pos[1] + 1, r.large ? 46 : 24, r.large ? 15 : 8, 0, 0, Math.PI * 2);
      g.fill();
      g.restore();
    }
    this.ink(g, pic, pos[0], pos[1], { flip, left, scale: po.scale, sy, lean: po.lean, alpha: Math.min(1, 0.35 + ph.k) });
    drawPicX(g, pic, pos[0], pos[1], { flip, left, scale: po.scale, sy, lean: po.lean, tint: po.tint, tintA: po.tintA, glow: po.glow });
    const bar = env.bar(b);
    if (bar) stage.bar({ x: pos[0], y: pos[1], large: r.large, side: env.side(b), bar });
  }

  /** A heavy ink line round a lifted figure, so the pair reads as one cut-out against the crowd
   *  (drawPicX's transform). */
  ink(g, p, x, y, { flip, left = false, scale = 1, sy = 1, lean = 0, alpha = 1 }) {
    const sil = silhouette(p, false, "#0b0816");
    g.save();
    g.translate(Math.round(x), Math.round(y));
    if (lean) g.rotate(leanTurn(lean, flip, left));
    g.scale(flip ? -scale : scale, scale * sy);
    g.globalAlpha = alpha * 0.92;
    for (const [ox, oy] of [[-2, 0], [2, 0], [0, -2], [0, 2], [-2, -1], [2, 1], [-1, 2], [1, -2], [-1, -2], [1, 2]]) g.drawImage(sil, -p.ax + ox, -p.ay + oy);
    g.restore();
  }

  /** Where role `r` stands in the hold at phase `ph` (its slide and creep). */
  heldAt(r, ph) {
    return [r.home[0] + r.off[0] + r.creep[0] * ph.c, r.home[1] + r.off[1] + r.creep[1] * ph.c];
  }

  /** The blow's marks: speed lines behind a lunge, the shot's streak, the slash, flare or whiff on
   *  each target, the sparks and the drifting motes. */
  drawMarks(g, ph) {
    const C = this.cur;
    const P = C.P;
    const t = ph.t;
    const ts = t / 1000;
    const [dx, dy] = C.dir;
    const fade = ph.hold ? 1 : ph.k;
    const ar = C.e.actor !== null && C.e.actor !== undefined ? C.roles.get(C.e.actor) : null;
    // speed lines behind a lunging attacker, on the cut only
    if (P.melee && ar && t < 190) {
      const ax = ar.home[0] + ar.off[0];
      const ay = ar.home[1] + ar.off[1] - 50;
      const a = 1 - t / 190;
      g.save();
      g.strokeStyle = "rgba(255, 248, 225, 0.8)";
      g.lineCap = "round";
      g.lineWidth = 2;
      for (let s = -2; s <= 2; s++) {
        const ox = -dy * s * 13;
        const oy = dx * s * 13;
        const len = 50 + 22 * (2 - Math.abs(s));
        g.globalAlpha = a * (0.8 - 0.12 * Math.abs(s));
        g.beginPath();
        g.moveTo(ax + ox - dx * 30, ay + oy - dy * 30);
        g.lineTo(ax + ox - dx * (30 + len), ay + oy - dy * (30 + len));
        g.stroke();
      }
      g.restore();
    }
    // the shot, already arriving as a streak (on the trajectory the flight had: arrows arc low,
    // stones high, spells flat)
    const card = impactOf(P);
    if (P.ranged && !card) {
      const kind = P.kind;
      const arc = kind === "Bombard" ? 150 : kind === "Shoot" ? 60 : 30;
      const [x0, y0] = [C.A[0], C.A[1] - 64];
      const [x1, y1] = [C.T[0], C.T[1] - 46];
      const tr = (u) => [x0 + (x1 - x0) * u, y0 + (y1 - y0) * u - arc * 4 * u * (1 - u)];
      const head = P.miss ? 1.06 : 0.985;
      const tail = (kind === "Shoot" ? 0.3 : 0.5) + 0.08 * ph.c;
      const N = 14;
      g.save();
      g.lineCap = "round";
      for (let s = 0; s < N; s++) {
        const [ax0, ay0] = tr(tail + ((head - tail) * s) / N);
        const [ax1, ay1] = tr(tail + ((head - tail) * (s + 1)) / N);
        const k = (s + 1) / N;
        if (kind === "Shoot") {
          g.strokeStyle = `rgba(30, 18, 8, ${0.55 * k * fade})`;
          g.lineWidth = 4 + 2 * k;
          g.beginPath();
          g.moveTo(ax0, ay0);
          g.lineTo(ax1, ay1);
          g.stroke();
          g.strokeStyle = `rgba(255, 246, 220, ${(0.25 + 0.75 * k) * fade})`;
          g.lineWidth = 1.5 + 2 * k;
        } else if (kind === "Bombard") {
          g.strokeStyle = `rgba(120, 104, 88, ${0.6 * k * fade})`;
          g.lineWidth = 3 + 9 * k;
        } else {
          g.strokeStyle = `rgba(${C.tone.rgb}, ${0.75 * k * fade})`;
          g.lineWidth = 2 + 8 * k;
        }
        g.beginPath();
        g.moveTo(ax0, ay0);
        g.lineTo(ax1, ay1);
        g.stroke();
      }
      const [hx, hy] = tr(head);
      g.globalAlpha = fade;
      if (kind === "Spell") {
        g.fillStyle = "#ffffff";
        g.beginPath();
        g.arc(hx, hy, 4, 0, Math.PI * 2);
        g.fill();
      } else if (kind === "Bombard" && !P.miss) {
        g.fillStyle = "#3b3530";
        g.beginPath();
        g.arc(hx, hy, 8, 0, Math.PI * 2);
        g.fill();
        g.fillStyle = "#6d645a";
        g.beginPath();
        g.arc(hx - 2, hy - 2, 3, 0, Math.PI * 2);
        g.fill();
      }
      g.restore();
    }
    // on each target: a slash, a flare or a glow; a miss gets a faint whiff beside it
    for (const r of C.roles.values()) {
      if (r.actor) continue;
      const [px, py] = this.heldAt(r, ph);
      const big = P.crit && r.primary;
      const aFade = ph.hold ? (ph.c < 0.45 ? 1 : 1 - ((ph.c - 0.45) / 0.55) * 0.75) : 0.25 * ph.k;
      if (r.kind === "healed") {
        g.save();
        g.globalAlpha = 0.5 * fade;
        const gr = g.createRadialGradient(px, py - 40, 4, px, py - 40, 60 + 10 * ph.c);
        gr.addColorStop(0, "rgba(200, 255, 220, 0.9)");
        gr.addColorStop(1, "rgba(125, 255, 176, 0)");
        g.fillStyle = gr;
        g.fillRect(px - 80, py - 120, 160, 160);
        g.restore();
        continue;
      }
      if (P.melee) {
        const c = r.primary ? C.contact : [px, py - 46];
        const ang = Math.atan2(dy, dx) + C.tilt;
        const drift = 7 * ph.c;
        if (r.kind === "miss" || r.kind === "escape") {
          const ox = dy * C.dodgeSign * 14;
          const oy = -dx * C.dodgeSign * 14;
          crescent(g, c[0] + ox + dx * drift, c[1] + oy + dy * drift, ang, 34, 9, "rgba(255,255,255,0.55)", 0.7 * aFade);
        } else {
          const R0 = (big ? 48 : r.primary ? 38 : 28) * (1 + 0.06 * ph.c);
          crescent(g, c[0] + dx * drift, c[1] + dy * drift, ang, R0 + 4, big ? 14 : 11, big ? "rgba(255, 200, 70, 0.9)" : "rgba(170, 200, 255, 0.6)", aFade);
          crescent(g, c[0] + dx * drift, c[1] + dy * drift, ang, R0, big ? 10 : 8, "#ffffff", aFade);
        }
      } else if (r.kind !== "miss" && !card) {
        // a flare that blooms on the cut and slowly opens
        const col = C.tone.rgb;
        const rr = (r.primary ? 26 : 18) + 18 * ph.c;
        g.save();
        g.globalAlpha = aFade;
        const gr = g.createRadialGradient(px, py - 44, 2, px, py - 44, rr + 14);
        gr.addColorStop(0, "rgba(255,255,255,0.95)");
        gr.addColorStop(0.35, `rgba(${col}, 0.7)`);
        gr.addColorStop(1, `rgba(${col}, 0)`);
        g.fillStyle = gr;
        g.beginPath();
        g.arc(px, py - 44, rr + 14, 0, Math.PI * 2);
        g.fill();
        g.lineWidth = 2;
        g.strokeStyle = `rgba(${col}, ${0.8 * aFade})`;
        g.beginPath();
        g.ellipse(px, py - 36, rr * 1.4, rr * 0.7, 0, 0, Math.PI * 2);
        g.stroke();
        g.restore();
        if (P.kind === "Shoot" && r.primary) {
          // the arrow, stuck where it struck
          g.save();
          g.globalAlpha = fade;
          g.strokeStyle = "#3a2410";
          g.lineWidth = 2;
          g.beginPath();
          g.moveTo(px - dx * 4, py - 50 - dy * 2);
          g.lineTo(px - dx * 20, py - 56 - dy * 10);
          g.stroke();
          g.restore();
        }
      }
    }
    // sparks: out fast on the cut, then coasting (drag), so the hold keeps moving
    if (C.sparks.length) {
      const tau = 0.09;
      const at = (s, tt) => {
        const k = tau * (1 - Math.exp(-tt / tau));
        return [s.x + s.vx * (k + 0.04 * tt), s.y + s.vy * (k + 0.04 * tt) + 70 * tt * tt];
      };
      g.save();
      g.lineCap = "round";
      g.globalAlpha = fade * (1 - 0.6 * ph.c);
      for (const s of C.sparks) {
        const [x1, y1] = at(s, ts);
        const [x0, y0] = at(s, Math.max(0, ts - 0.035));
        g.strokeStyle = s.blood ? "#a3121c" : s.gold ? "#ffd36a" : "#ffffff";
        g.lineWidth = s.blood ? 3 : 2;
        g.beginPath();
        g.moveTo(x0, y0);
        g.lineTo(x1 + 0.01, y1);
        g.stroke();
      }
      g.restore();
    }
    // embers, dust or motes drifting through the held beat
    g.save();
    for (const m of C.motes) {
      g.globalAlpha = Math.min(1, t / 140) * fade * m.fade;
      g.fillStyle = m.col;
      g.fillRect(Math.round(m.x + m.vx * ts + Math.sin(ts * 2.2 + m.ph) * m.sw), Math.round(m.y + m.vy * ts), m.sz, m.sz);
    }
    g.restore();
  }

  /** The numbers over the targets: the damage (CRITICAL, SLAIN under it), miss, escapes, a heal. */
  drawTexts(g, ph) {
    const C = this.cur;
    const t = ph.t;
    const alpha = ph.hold ? 1 : ph.k;
    const pop = t < 120 ? 1.7 - 0.7 * ease.outBack(t / 120) : 1;
    g.save();
    g.textAlign = "center";
    g.lineJoin = "round";
    for (const r of C.roles.values()) {
      if (r.actor) continue;
      const [px, py] = this.heldAt(r, ph);
      const h = r.h;
      let text;
      let col;
      let size = r.primary ? 30 : 20;
      let tag = null;
      let tagCol = null;
      if (r.kind === "healed") [text, col] = [`+${dmgText(h.heal)}`, "#8dffb4"];
      else if (r.kind === "miss") [text, col, size] = ["miss", "#d6dbe8", 20];
      else if (r.kind === "escape") [text, col, size] = ["escapes", "#bfe3ff", 20];
      else {
        text = `−${dmgText(h.dmg)}`;
        col = h.crit ? "#ffe58c" : h.killed ? "#ff8a78" : "#ffffff";
        if (h.crit) [size, tag, tagCol] = [r.primary ? 40 : 30, "CRITICAL", "#f6c650"];
        else if (h.killed) [tag, tagCol] = ["SLAIN", "#ff9a88"];
      }
      g.save();
      g.globalAlpha = alpha;
      g.translate(Math.round(px), Math.round(py - (r.large ? 136 : 120) - 10 * ph.c));
      g.scale(pop, pop);
      g.font = `700 ${size}px ${FONT}`;
      g.lineWidth = 6;
      g.strokeStyle = "rgba(8, 6, 18, 0.92)";
      g.strokeText(text, 0, 0);
      g.fillStyle = col;
      g.fillText(text, 0, 0);
      if (tag) {
        const s = tag.split("").join(" ");
        g.font = `700 10px ${FONT}`;
        g.lineWidth = 4;
        g.strokeText(s, 0, 15);
        g.fillStyle = tagCol;
        g.fillText(s, 0, 15);
      }
      g.restore();
    }
    g.restore();
  }
}

/** A crescent (the slash) at (x, y) along `ang`, radius `R`, `w` thick, squashed to the ground's
 *  slant. */
function crescent(g, x, y, ang, R, w, fill, alpha) {
  const yi = Math.sqrt(Math.max(1, R * R - (w * w) / 4));
  const aOut = Math.atan2(yi, -w / 2);
  const aIn = Math.atan2(yi, w / 2);
  g.save();
  g.translate(x, y);
  g.scale(1, 0.78);
  g.rotate(ang);
  g.globalAlpha = clamp(alpha, 0, 1);
  g.beginPath();
  g.arc(0, 0, R, -aOut, aOut);
  g.arc(-w, 0, R, aIn, -aIn, true);
  g.closePath();
  g.fillStyle = fill;
  g.fill();
  g.restore();
}
