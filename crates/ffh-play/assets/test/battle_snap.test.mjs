// node --test: the watch screen's presentation (app/battle_snap.js, the snap with the shade on
// crits and kills): each action's timing plan by kind, which actions get the shade, the roles an
// event stages and where they slide, and the pose each unit snaps to (its strip's frame, or its
// sprite leaning), over hand-made events and the dev fixture's battles.
import test from "node:test";
import assert from "node:assert/strict";
import * as S from "../app/battle_snap.js";
import { STRIKE_HIT, ANIMS } from "../app/battle_stage.js";
import { FIXTURES } from "../app/battle_fixture.js";

const events = Object.values(FIXTURES).flatMap((fx) => fx.logs.flatMap((l) => l.log.events));
const hit = (o = {}) => ({ target: 1, dmg: 1500, ...o });
const ev = (kind, hits, more = {}) => ({ round: 1, pass: 1, actor: 0, kind, target: hits[0]?.target ?? null, hits, ...more });

/** The frame of strip `anim` shown `ms` into it. */
function frameOf(anim, ms) {
  let t = ms;
  for (let k = 0; k < ANIMS[anim].length; k++) {
    if (t < ANIMS[anim][k]) return k;
    t -= ANIMS[anim][k];
  }
  return ANIMS[anim].length - 1;
}

test("the timing plan by kind (ms at 1x): a snap with no wind-up, a held beat, a quarter-second return", () => {
  const plan = (e) => {
    const P = S.planOf(e);
    return [P.hold, P.ret, P.rest, P.end, P.shake];
  };
  assert.deepEqual(plan(ev("Strike", [hit()])), [680, 240, 160, 1100, 3]);
  assert.deepEqual(plan(ev("Charge", [hit()])), [680, 240, 160, 1100, 3]);
  assert.deepEqual(plan(ev("Shoot", [hit()])), [620, 240, 160, 1040, 3]);
  assert.deepEqual(plan(ev("Spell", [hit(), hit({ target: 2 })])), [620, 240, 160, 1040, 3]);
  assert.deepEqual(plan(ev("Bombard", [hit()])), [620, 240, 160, 1040, 5]);
  assert.deepEqual(plan(ev("Strike", [hit({ crit: true })])), [1050, 280, 160, 1510, 7]);
  assert.deepEqual(plan(ev("Strike", [hit({ crit: true, killed: true })])), [1150, 280, 440, 1890, 7]);
  assert.deepEqual(plan(ev("Strike", [hit({ killed: true })])), [900, 260, 440, 1620, 5]);
  assert.deepEqual(plan(ev("Strike", [hit({ dmg: 0, miss: true })])), [480, 220, 140, 860, 0]);
  assert.deepEqual(plan(ev("Strike", [hit({ withdrew: true })])), [720, 240, 160, 1140, 3]);
  assert.deepEqual(plan({ round: 1, pass: 1, actor: 0, kind: "Heal", target: 1, hits: [], heal: { target: 1, amount: 900 } }), [560, 240, 160, 980, 0]);
  // a defensive strike is staged as a shot
  assert.equal(S.planOf({ round: 0, pass: 0, kind: "DefStrike", actor: 5, target: 1, hits: [hit()] }).kind, "Shoot");
  // the contact frame comes after a still frame; the fates show when the pair is back
  const P = S.planOf(ev("Strike", [hit({ killed: true })]));
  assert.equal(P.impact, S.LEAD);
  assert.equal(P.fates, S.LEAD + P.hold + P.ret);
  assert.ok(P.fates < P.end, "a fall gets its rest before the next cut");
  // no blow, no snap: a word floats up
  assert.deepEqual(S.planOf(ev("Strike", [], { note: "NoTarget" })), { snap: false, impact: 120, end: 520 });
  assert.deepEqual(S.planOf({ round: 0, pass: 0, kind: "Afraid", actor: 3, target: 4 }), { snap: false, impact: 300, end: 800 });
});

test("every fixture action holds 0.48 to 1.15 s; a crit or a kill holds longer than the same blow plain", () => {
  for (const e of events) {
    const P = S.planOf(e);
    if (!P.snap) continue;
    assert.ok(P.hold >= 480 && P.hold <= 1150, `hold ${P.hold}`);
    assert.equal(P.end, P.impact + P.hold + P.ret + P.rest);
    assert.ok(P.ret >= 220 && P.ret <= 280, "a quarter-second ease back");
    const plain = S.planOf({ ...e, hits: (e.hits ?? []).map((h) => ({ target: h.target, dmg: h.dmg, miss: h.miss })) });
    if (P.crit || P.kill) assert.ok(P.hold > plain.hold, `${e.kind}: ${P.hold} > ${plain.hold}`);
  }
});

test("the shade falls on crits and kills only", () => {
  assert.equal(S.shaded(ev("Strike", [hit()])), false);
  assert.equal(S.shaded(ev("Strike", [hit({ crit: true })])), true);
  assert.equal(S.shaded(ev("Shoot", [hit({ killed: true })])), true);
  // a splash target slain is a kill
  assert.equal(S.shaded(ev("Spell", [hit(), hit({ target: 2, killed: true })])), true);
  assert.equal(S.shaded(ev("Strike", [hit({ dmg: 0, miss: true })])), false);
  assert.equal(S.shaded(ev("Strike", [hit({ withdrew: true })])), false);
  assert.equal(S.shaded({ round: 1, pass: 1, actor: 0, kind: "Heal", target: 1, hits: [], heal: { target: 1, amount: 900 } }), false);
  let n = 0;
  for (const e of events) {
    const big = !!e.hits?.some((h) => h.crit || h.killed);
    assert.equal(S.shaded(e), big);
    n += big;
  }
  assert.ok(n > 10 && n < events.length / 2, `the fixtures shade ${n} of ${events.length} actions`);
});

test("the roles an event stages: the actor, the target, the splash", () => {
  const kinds = (e) => Object.fromEntries([...S.rolesOf(e)].map(([b, r]) => [b, `${r.kind}${r.primary ? "*" : ""}`]));
  assert.deepEqual(kinds(ev("Strike", [hit()])), { 0: "melee", 1: "hit*" });
  assert.deepEqual(kinds(ev("Stab", [hit({ killed: true })])), { 0: "melee", 1: "kill*" });
  assert.deepEqual(kinds(ev("Shoot", [hit({ dmg: 0, miss: true })])), { 0: "shoot", 1: "miss*" });
  assert.deepEqual(kinds(ev("Bombard", [hit({ withdrew: true })])), { 0: "bombard", 1: "escape*" });
  assert.deepEqual(kinds(ev("Spell", [hit(), hit({ target: 2 }), hit({ target: 3, killed: true })])), { 0: "spell", 1: "hit*", 2: "hit", 3: "kill" });
  assert.deepEqual(kinds({ round: 1, pass: 1, actor: 0, kind: "Heal", target: 1, hits: [], heal: { target: 1, amount: 900 } }), { 0: "healer", 1: "healed*" });
  assert.deepEqual(kinds({ round: 0, pass: 0, kind: "DefStrike", actor: 5, target: 1, hits: [hit()] }), { 5: "shoot", 1: "hit*" });
  const kn = S.rolesOf(ev("Strike", [hit({ crit: true })], { knocked: { unit: 1, from: 0, to: 1 } }));
  assert.equal(kn.get(1).knock, true);
  assert.equal(S.rolesOf(ev("Strike", [], { note: "NoTarget" })).size, 0);
});

test("where they slide: a melee pair meets, a shooter stays on its cell, a miss dodges aside", () => {
  const feet = { 0: { x: 400, y: 300 }, 1: { x: 600, y: 400 }, 2: { x: 700, y: 350 } };
  const at = (b) => feet[b] ?? null;
  const dist = (p, q) => Math.hypot(p[0] - q[0], p[1] - q[1]);
  const held = (r) => [r.home[0] + r.off[0], r.home[1] + r.off[1]];
  const m = S.stageOf(ev("Strike", [hit()]), at, 1);
  const [a, t] = [m.roles.get(0), m.roles.get(1)];
  assert.ok(dist(held(a), held(t)) < 70, "the pair meets on the cut");
  assert.ok(dist(a.off, [0, 0]) > 3 * dist(t.off, [0, 0]), "the attacker goes most of the way");
  assert.ok(m.dir[0] > 0 && m.dir[1] > 0, "the blow's line runs from the attacker to the target");
  assert.equal(m.sparks.length, 9);
  const s = S.stageOf(ev("Shoot", [hit(), hit({ target: 2 })]), at, 1);
  assert.ok(dist(s.roles.get(0).off, [0, 0]) <= 4.01, "the shooter recoils on its cell");
  for (const b of [1, 2]) assert.ok(dist(s.roles.get(b).off, [0, 0]) <= 5.01, "the struck snap together where they stand");
  const d = S.stageOf(ev("Strike", [hit({ dmg: 0, miss: true })]), at, 1);
  const r = d.roles.get(1);
  const along = r.off[0] * d.dir[0] + r.off[1] * d.dir[1];
  const across = Math.abs(-r.off[0] * d.dir[1] + r.off[1] * d.dir[0]);
  assert.ok(across > 15 && across > Math.abs(along), "the dodge goes aside");
  assert.equal(d.sparks.length, 0, "a miss throws no sparks");
  // a striker off the field (the reserve) is not staged; its shot is, from where it stands
  const reserve = (b) => (b === 9 ? { x: 200, y: 250, field: false } : at(b));
  const off = S.stageOf({ round: 0, pass: 0, kind: "DefStrike", actor: 9, target: 1, hits: [hit()] }, reserve, 1);
  assert.deepEqual([...off.roles.keys()], [1]);
  assert.deepEqual(off.A, [200, 250]);
  // the same action draws the same sparks
  assert.deepEqual(S.stageOf(ev("Strike", [hit({ crit: true })]), at, 7).sparks, S.stageOf(ev("Strike", [hit({ crit: true })]), at, 7).sparks);
});

test("the phases: still before the cut, held, eased back, then gone", () => {
  const P = S.planOf(ev("Strike", [hit()]));
  assert.equal(S.phaseOf(P, -1), null);
  assert.deepEqual(S.phaseOf(P, 0), { t: 0, k: 1, q: 0, c: 0, hold: true });
  assert.equal(S.phaseOf(P, P.hold - 1).k, 1);
  const mid = S.phaseOf(P, P.hold + P.ret / 2);
  assert.ok(!mid.hold && mid.k > 0.4 && mid.k < 0.6);
  assert.equal(S.phaseOf(P, P.hold + P.ret), null);
  // the shake lands on the contact frame and is gone within a quarter second
  assert.ok(Math.hypot(...S.shakeAt(P, 0)) > 0);
  assert.equal(S.shakeAt(P, 230), null);
  assert.equal(S.shakeAt(S.planOf(ev("Strike", [hit({ dmg: 0, miss: true })])), 0), null);
});

test("the poses each unit snaps to: its strip's contact frames, else its sprite leaning", () => {
  const ph = (P, t) => S.phaseOf(P, t);
  const P = S.planOf(ev("Strike", [hit()]));
  const melee = { kind: "melee" };
  const target = { kind: "hit", primary: true };
  // an animated striker cuts to the swing's last frame, holds the blow's, recovers on the way back
  const cut = S.poseOf(melee, ph(P, 0), true);
  assert.equal(cut.anim, "strike");
  assert.equal(frameOf("strike", cut.ms), frameOf("strike", STRIKE_HIT) - 1);
  assert.equal(frameOf("strike", S.poseOf(melee, ph(P, 300), true).ms), frameOf("strike", STRIKE_HIT));
  assert.ok(frameOf("strike", S.poseOf(melee, ph(P, P.hold + P.ret - 1), true).ms) > frameOf("strike", STRIKE_HIT));
  assert.equal(cut.clean, true, "the frozen frame drops the blade's smear");
  // one without a strip leans forward, a little larger, and settles back
  const lean = S.poseOf(melee, ph(P, 300), false);
  assert.equal(lean.anim, null);
  assert.ok(lean.lean > 0.2 && lean.scale > 1.05);
  const back = S.poseOf(melee, ph(P, P.hold + P.ret - 1), false);
  assert.ok(back.lean < 0.01 && back.scale < 1.01);
  // the struck: its flinch frame, else a lean back; a white flash on the cut either way
  const fl = S.poseOf(target, ph(P, 0), true);
  assert.equal(fl.anim, "flinch");
  assert.equal(frameOf("flinch", fl.ms), 1);
  assert.equal(fl.tint, "#ffffff");
  assert.ok(fl.tintA > 0.8);
  const lb = S.poseOf(target, ph(P, 0), false);
  assert.ok(lb.anim === null && lb.lean < -0.15 && lb.tint === "#ffffff");
  assert.equal(S.poseOf(target, ph(P, 300), false).tintA, 0, "the flash is gone in the hold");
  // the slain keep the recoil through the return, under a red pulse
  const K = S.planOf(ev("Strike", [hit({ killed: true })]));
  const slain = S.poseOf({ kind: "kill", primary: true }, ph(K, K.hold + 100), true);
  assert.equal(frameOf("flinch", slain.ms), 1);
  assert.equal(slain.tint, "#ff3a2a");
  // shooters lean back on their cells; casters and healers glow; a miss leans away
  assert.ok(S.poseOf({ kind: "shoot" }, ph(P, 300), true).lean < 0);
  assert.equal(S.poseOf({ kind: "shoot" }, ph(P, 300), true).anim, null);
  assert.ok(S.poseOf({ kind: "spell" }, ph(P, 300), false, { light: "#9feeff" }).glow === "#9feeff");
  assert.ok(S.poseOf({ kind: "healed", primary: true }, ph(P, 300), false).glow);
  assert.ok(S.poseOf({ kind: "miss", primary: true }, ph(P, 300), true).lean < 0);
  assert.equal(S.poseOf({ kind: "escape", primary: true }, ph(P, 300), true).tint, "#bfe3ff");
});

test("the breathing: a slow squash of 2% at most, out of step between units", () => {
  for (let w = 0; w < 6000; w += 250) assert.ok(Math.abs(S.breath(3, 0, w) - 1) <= 0.02 + 1e-9);
  assert.notEqual(S.breath(1, 0, 1000), S.breath(2, 0, 1000));
});
