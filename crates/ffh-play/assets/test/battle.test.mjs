// node --test: the battle screens' pure modules over the dev fixture (app/battle_fixture.js, the
// protocol shapes of app/battle_log.js): the log folded into segments and states (every round's
// fold equals the engine's own HP after it), the timings, the focused hit, the report's fates and
// checks; the teaching ledger (levels, budgets, storage that fails) and that every code a log
// carries has a text.
import test from "node:test";
import assert from "node:assert/strict";
import * as L from "../app/battle_log.js";
import * as T from "../app/battle_teach.js";
import { setData } from "../app/state.js";
import { roleIcon } from "../app/battle_stage.js";

import { FIXTURES } from "../app/battle_fixture.js";

const fixture = FIXTURES.strages;
const [won, lost] = fixture.logs.map((x) => x.log);

/** The roles of formation battles, in the data's order (Units/CIV4CombatRoleInfos.xml). */
const ROLES = ["FRONTLINE", "SCREEN", "FLANKER", "ASSASSIN", "ARCHER", "SIEGE", "ARCANE", "DIVINE"];

setData({
  kinds: ["UnitType", "CombatRoleType", "StanceType", "PromotionType"],
  tables: {
    UnitType: [{ type: "UNIT_AXEMAN", name: "Axeman" }],
    CombatRoleType: ROLES.map((r) => ({ type: `COMBATROLE_${r}`, name: r[0] + r.slice(1).toLowerCase() })),
    StanceType: [{ type: "STANCE_BALANCED", name: "Balanced" }],
    PromotionType: [{ type: "PROMOTION_FORMATION", name: "Formation" }],
  },
});

/** An in-memory storage; `fail` makes every call throw (a blocked or full localStorage). */
function memStorage({ fail = false } = {}) {
  const m = new Map();
  return {
    getItem: (k) => {
      if (fail) throw new Error("blocked");
      return m.get(k) ?? null;
    },
    setItem: (k, v) => {
      if (fail) throw new Error("blocked");
      m.set(k, String(v));
    },
    m,
  };
}

test("types resolve by index or by name", () => {
  assert.equal(L.tyName("UnitType", 0), "UNIT_AXEMAN");
  assert.equal(L.tyName("UnitType", "UNIT_X"), "UNIT_X");
  assert.equal(L.tyIdx("UnitType", "UNIT_AXEMAN"), 0);
  assert.equal(L.tyLabel("UnitType", "UNIT_AXEMAN"), "Axeman");
  assert.equal(L.tyLabel("UnitType", "UNIT_WAR_ELEPHANT"), "War Elephant");
  assert.equal(L.roleKey({ role: 2 }), "flanker");
  assert.equal(L.roleKey({ role: 7 }), "divine");
  assert.equal(L.roleKey({ role: "COMBATROLE_SCREEN" }), "screen");
});

test("every role has its ring icon and its motion", () => {
  const want = {
    FRONTLINE: ["fortify", "lunge"],
    SCREEN: ["defense", "lunge"],
    FLANKER: ["moves", "ride"],
    ASSASSIN: ["attack", "ride"],
    ARCHER: ["rangeattack", "shot"],
    SIEGE: ["rangeattack", "shot"],
    ARCANE: ["spell", "bolt"],
    DIVINE: ["spell", "bolt"],
  };
  ROLES.forEach((r, i) => {
    for (const role of [i, `COMBATROLE_${r}`]) {
      assert.deepEqual([roleIcon({ role }), L.motionOf({ role })], want[r], r);
    }
  });
  // The fixture names only roles the data has.
  for (const fx of Object.values(FIXTURES)) {
    for (const { log } of fx.logs) {
      for (const f of log.fighters) assert.ok(ROLES.includes(f.role.replace(/^COMBATROLE_/, "")), f.role);
    }
  }
});

test("names: duplicates by level, sides, plurals", () => {
  const axes = won.fighters.filter((f) => f.name === "Axeman");
  assert.ok(axes.length >= 2);
  assert.equal(L.uname(won, axes[0].b), `Axeman L${axes[0].level}`);
  assert.match(L.uname(won, axes[0].b, { side: true, cap: true }), /^Your Axeman L\d$/);
  const champ = won.fighters.find((f) => f.name === "Champion");
  assert.equal(L.uname(won, champ.b, { side: true }), "their Champion");
  assert.equal(L.plural("Horseman"), "Horsemen");
  assert.equal(L.listNames(["a", "b", "c"]), "a, b and c");
  const horses = won.fighters.filter((f) => f.name === "Horseman").map((f) => f.b);
  assert.equal(L.groupNames(won, horses), "the 2 Horsemen");
  // two of a type and a level: the second is numbered
  const pv = fixture.preview;
  const archers = pv.fighters.filter((f) => f.side === 0 && f.name === "Archer");
  assert.deepEqual(
    archers.map((f) => L.uname(pv, f.b)),
    ["Archer L1", "Archer L1 (2)"],
  );
});

test("slots: the sum in words and the walls' note", () => {
  assert.equal(L.slotSum(won.slots[0]), "8 slots − 2 for the hill = 6");
  assert.equal(L.wallsNote(won.slots[0]), "Walls 20%: −1 more at 25%");
  assert.equal(L.wallsNote(won.slots[1]), null);
});

test("segments: every event lands in the segment of its phase", () => {
  for (const log of [won, lost]) {
    const segs = L.segments(log);
    const rounds = segs.filter((s) => s.type === "round" && s.kind === "round");
    assert.equal(rounds.length, log.outcome.rounds);
    assert.deepEqual(
      rounds.map((s) => s.n),
      rounds.map((_, k) => k + 1),
    );
    const hits = log.events.filter((e) => e.kind === "Hit").length;
    assert.equal(segs.reduce((a, s) => a + s.hits.length, 0), hits);
    assert.equal(segs.filter((s) => s.type === "end").length, 1);
    assert.equal(segs[0].type, "strikes", "the walls' archers strike first");
    for (const s of segs.filter((x) => x.type === "round")) assert.ok(s.roundEnd, `round ${s.n} has its RoundEnd`);
  }
});

test("the fold reproduces the engine's HP after every round, and the outcome", () => {
  for (const log of [won, lost]) {
    const sc = L.buildScript(log);
    const st = L.initialState(log);
    for (const s of sc.segs) {
      L.applySeg(st, s, log.fighters, Infinity);
      if (s.roundEnd?.hp) assert.deepEqual(st.hp, s.roundEnd.hp, `HP after ${s.kind} ${s.n}`);
    }
    assert.deepEqual(st, sc.final);
    const F = log.fighters;
    const alive = (side) => F.filter((f) => f.side === side && L.committed(f) && !st.dead[f.b] && !st.out[f.b]).length;
    if (log.outcome.result === "AttackerWon") assert.equal(alive(1), 0);
    if (log.outcome.result === "DefenderHeld") assert.equal(alive(0), 0);
    for (const e of log.events.filter((x) => x.kind === "Withdraw")) assert.equal(st.out[e.unit], e.ok);
    for (const f of F.filter((x) => !L.committed(x))) assert.equal(st.hp[f.b], f.hp_start, "who stays out is untouched");
  }
});

test("the state at a moment: bars drop together at the bell; pending hits before it", () => {
  const sc = L.buildScript(won);
  const si = sc.segs.findIndex((s) => s.type === "round" && s.kind === "round" && s.deaths.length);
  const s = sc.segs[si];
  assert.ok(s.hits[0].impact < s.land && s.land <= s.deathStart && s.deathStart < s.dur);
  const before = L.stateAt(sc, si, s.land - 1);
  assert.deepEqual(before.hp, s.snap.hp, "no HP moves before the bell");
  const pend = L.pendingAt(s, won.fighters.length, s.land - 1);
  const want = won.fighters.map(() => 0);
  for (const h of s.hits) want[h.t] += h.dmg;
  assert.deepEqual(pend, want);
  const atBell = L.stateAt(sc, si, s.land);
  assert.ok(s.deaths.every((d) => !atBell.dead[d.b]), "the dead fall after the bell");
  const after = L.stateAt(sc, si, s.deathStart);
  assert.ok(s.deaths.every((d) => after.dead[d.b] && after.diedIn[d.b] === s.n));
});

test("timings: beats compress quiet rounds; hits launch in slot order", () => {
  const full = L.buildScript(won, { mode: "full" });
  const beats = L.buildScript(won, { mode: "beats" });
  const total = (sc) => sc.segs.reduce((a, s) => a + s.dur, 0);
  assert.ok(total(beats) < total(full) * 0.75, `${total(beats)} vs ${total(full)}`);
  for (const s of full.segs) {
    for (let k = 1; k < s.hits.length; k++) assert.ok(s.hits[k].start > s.hits[k - 1].start);
    const quiet = beats.segs[full.segs.indexOf(s)];
    if (s.type === "round" && !s.beat) assert.ok(quiet.dur < s.dur);
  }
  const animated = (f) => f.name === "Axeman";
  const sc = L.buildScript(won, { animated });
  const axeHit = sc.segs.flatMap((s) => s.hits).find((h) => won.fighters[h.h].name === "Axeman");
  assert.equal(axeHit.kind, "anim");
  const archer = sc.segs.flatMap((s) => s.hits).find((h) => won.fighters[h.h].name === "Longbowman");
  assert.equal(archer.kind, "shot");
});

test("the focused hit prefers a rule the viewer has not seen, then a kill", () => {
  const fresh = L.buildScript(won, { rulesSeen: [] });
  const rules = new Set();
  for (const s of fresh.segs.filter((x) => x.hits.length)) {
    const h = s.hits[s.focus];
    assert.equal(won.fighters[h.h].side, 0, "the focus is a hit of yours");
    rules.add(h.ev.rule);
  }
  assert.ok(rules.has("FlankerAround") && rules.has("FrontFirst"));
  const vet = L.buildScript(won, { rulesSeen: Object.keys(T.RULE_TEXT) });
  const s = vet.segs.find((x) => x.deaths.some((d) => won.fighters[d.b].side === 1));
  const h = s.hits[s.focus];
  assert.ok(s.deaths.some((d) => d.b === h.t && d.killer === h.h), "a veteran's focus is the kill");
});

test("screens gone and beats", () => {
  const sc = L.buildScript(won);
  const gone = sc.segs.filter((s) => s.screenGone);
  assert.equal(gone.length, 1);
  assert.ok(gone[0].deaths.some((d) => won.fighters[d.b].screen));
  for (const s of sc.segs) if (s.deaths.length || s.wds.length) assert.ok(s.beat);
});

test("the report: fates, XP, the plan checked, beat lines, facts", () => {
  const sc = L.buildScript(won);
  const fates = L.fates(won, sc.final);
  assert.equal(fates.length, won.fighters.filter((f) => f.side === 0).length);
  const by = (k) => fates.filter((f) => f.kind === k);
  assert.equal(by("out").length, 2);
  assert.ok(by("dead").every((f) => f.round > 0 && f.killer !== null));
  assert.equal(by("withdrew").length, 1);
  assert.ok(by("withdrew")[0].roll < by("withdrew")[0].chance);
  const failed = by("dead").find((f) => f.roll !== null);
  assert.ok(failed && failed.roll >= failed.chance, "a failed withdrawal roll is told");
  assert.ok(by("alive").some((f) => f.xp > 0));
  const checks = L.planChecks(won, sc.final);
  assert.deepEqual(checks[0], { verdict: "ok", label: "win 91%", result: "won" });
  assert.equal(L.verdict(35), "ok");
  assert.equal(L.verdict(34), "luck");
  assert.equal(L.verdict(8), "luck");
  assert.equal(L.verdict(7), "off");
  const lines = L.beatLines(sc);
  const text = lines.map((l) => `${l.label} ${l.parts.map((p) => p.text).join("")}`).join("\n");
  assert.match(text, /R\d+ .*shield down/);
  assert.match(text, /taken/);
  assert.match(text, /pulls out \(\d+<\d+\)/);
  const facts = L.decidingFacts(won, sc.final);
  assert.ok(facts.length >= 2 && facts.length <= 3);
  const lostSc = L.buildScript(lost);
  assert.deepEqual(L.planChecks(lost, lostSc.final)[0].result, "lost");
  assert.ok(L.decidingFacts(lost, lostSc.final).length >= 1);
});

test("the plan: What-if lines merge plans of the same units; the stakes", () => {
  const pv = fixture.preview;
  const F = pv.fighters;
  const bOf = (id) => F.find((f) => f.unit === id).b;
  const force = fixture.stance_picks.STANCE_BALANCED;
  const preview = { ...pv, force: force.map(bOf), reserve: F.filter((f) => f.side === 0 && !force.includes(f.unit)).map((f) => f.b), stance: "STANCE_BALANCED", forecast: fixture.forecasts[force.slice().sort((a, b) => a - b).join(",")] };
  const alts = [
    { kind: "StancePick", stance: "STANCE_BALANCED", force, win_permille: 700 },
    { kind: "OtherStance", stance: "STANCE_SHIELD_WALL", force, win_permille: 700 },
    { kind: "OtherStance", stance: "STANCE_MOBILE", force: fixture.stance_picks.STANCE_MOBILE, win_permille: 900 },
  ];
  const lines = L.whatIfLines(preview, alts, (s) => L.tyLabel("StanceType", s));
  assert.equal(lines.length, 2);
  assert.deepEqual(lines[0].also, ["Balanced", "Shield Wall"]);
  assert.equal(lines[1].label, "Mobile's pick");
  // two Archers L1 are the same kind of unit: swapping one for the other is the same plan
  const archers = F.filter((f) => f.side === 0 && f.name === "Archer").map((f) => f.unit);
  const swapped = force.map((x) => (x === archers[0] ? archers[1] : x === archers[1] ? archers[0] : x));
  assert.equal(L.compositionKey(F, swapped), L.compositionKey(F, force));
  const st = L.stakes(preview);
  assert.equal(st.force.length, 6);
  assert.equal(st.defenders.length, 5);
  assert.equal(st.defendersLeft, 0);
  assert.equal(st.reserve.length, 2);
  assert.equal(st.win + st.lose + st.draw, 100);
  assert.deepEqual(L.plotDvTerms(F).map((t) => t.src), ["Hills", "CityDefense", "Fortify"]);
});

test("the ledger: full, then short, then a marker; storage that fails is harmless", () => {
  const store = memStorage();
  const led = new T.Ledger(store);
  assert.equal(led.level("a_vs_dv"), "full");
  assert.equal(led.viewer(), "new");
  const newly = led.recordBattle(["a_vs_dv", "shield_down"], won);
  assert.deepEqual([...newly].sort(), ["a_vs_dv", "shield"]);
  assert.equal(led.level("a_vs_dv"), "short");
  assert.ok(led.codexSeen("shield"));
  assert.ok(led.rulesSeen().includes("FlankerAround"));
  const again = new T.Ledger(store);
  assert.equal(again.seen("a_vs_dv"), 1, "the ledger is kept");
  again.recordBattle(["a_vs_dv"], won);
  assert.equal(again.level("a_vs_dv"), "icon");
  const blocked = new T.Ledger(memStorage({ fail: true }));
  blocked.recordBattle(["luck"], won);
  assert.equal(blocked.level("luck"), "short", "a blocked storage still counts in memory");
  const none = new T.Ledger(null);
  none.recordPlan();
  assert.equal(none.level("slots"), "short");
  const bad = memStorage();
  bad.setItem(T.LEDGER_KEY, "{not json");
  assert.equal(new T.Ledger(bad).level("luck"), "full", "a corrupt ledger starts over");
  const v = new T.Ledger(memStorage());
  v.setViewer("veteran");
  assert.equal(v.level("a_vs_dv"), "icon");
  assert.equal(v.viewer(), "veteran");
  assert.equal(v.watchMode(), "beats");
  assert.equal(v.showPlan(900), false);
  assert.equal(v.showPlan(700), true);
  v.set("watch_mode", "result");
  v.setViewer("new");
  assert.equal(v.watchMode(), "result", "settings survive a teaching reset");
  const less = new T.Ledger(memStorage());
  less.teachLess();
  assert.equal(less.level("luck"), "short");
});

test("lessons: one a segment, two pauses and two secondary lessons in full a battle, then short, then markers", () => {
  const led = new T.Ledger(memStorage());
  const lessonsOf = () => {
    const sc = L.buildScript(won, { rulesSeen: led.rulesSeen(), lessons: (segs, log) => T.assignLessons(segs, log, (id) => led.level(id)) });
    return sc.segs;
  };
  const first = lessonsOf();
  const shown = first.filter((s) => s.lesson).map((s) => s.lesson);
  assert.ok(shown.length >= 6, `${shown.map((x) => x.id)}`);
  assert.equal(shown.filter((x) => x.mode === "pause").length, 2);
  assert.deepEqual(
    shown.filter((x) => x.mode === "pause").map((x) => x.id),
    ["a_vs_dv", "shield_down"],
  );
  assert.ok(shown.filter((x) => T.SECONDARY.includes(x.id)).length <= T.SECONDARY_BUDGET);
  const ids = shown.map((x) => x.id);
  assert.equal(new Set(ids).size, ids.length, "each lesson once a battle");
  for (const s of first) for (const ic of s.icons ?? []) assert.ok(ic.deferred, "only deferred lessons are markers in the first battle");
  led.recordBattle(ids, won);
  const second = lessonsOf();
  for (const s of second) if (s.lesson && ids.includes(s.lesson.id)) assert.equal(s.lesson.mode, "short");
  assert.ok(second.some((s) => s.lesson?.mode === "full"), "the deferred lessons come in full");
  led.recordBattle(second.filter((s) => s.lesson).map((s) => s.lesson.id), won);
  led.recordBattle(Object.keys(T.LESSON_CODEX), won);
  const third = lessonsOf();
  assert.ok(third.every((s) => !s.lesson), "a veteran sees markers only");
  assert.ok(third.some((s) => s.icons?.length));
});

test("every code a log carries has a text, and every lesson has words", () => {
  for (const log of [won, lost]) {
    for (const e of log.events) {
      if (e.kind === "Hit") {
        assert.ok(T.RULE_TEXT[e.rule], `rule ${e.rule}`);
        for (const [, why] of e.blocked) assert.ok(T.BLOCK_LABEL[why], `block ${why}`);
      }
    }
    for (const f of log.fighters) for (const c of f.targets) assert.ok(T.CLASS_LABEL[c], `class ${c}`);
    for (const f of log.fighters) for (const t of [...f.a.terms, ...f.dv.terms]) assert.notEqual(L.termGroup(t), "other", `term ${t.src}`);
    const segs = L.buildScript(log, { lessons: (s, l) => T.assignLessons(s, l, () => "full"), caption: T.captionFor }).segs;
    for (const s of segs) {
      if (s.lesson) {
        const [title, body] = T.lessonText(s.lesson.id, log, s.lesson.ctx);
        assert.ok(title && body && !/undefined|NaN/.test(title + body), `${s.lesson.id}: ${title} ${body}`);
      }
      if (s.beat || s.type === "round") assert.ok(s.caption && !/undefined|NaN/.test(s.caption), `caption of ${s.type} ${s.n}: ${s.caption}`);
    }
  }
  for (const id of Object.keys(T.LESSON_CODEX)) {
    const [title, body] = T.lessonText(id, won, {});
    assert.ok(title && body, id);
    assert.ok(T.CODEX.some(([k]) => k === T.LESSON_CODEX[id]), id);
  }
  assert.equal(T.resultWords(won).banner, "STRAGES FALLS");
  assert.equal(T.resultWords(lost).ribbon, "DEFEAT");
});
