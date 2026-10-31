// The battle screens' teaching (the battle UI design, DESIGN.md section 6): the ledger of what the
// viewer has been shown, which lesson each moment of a battle teaches and in which form, and the
// texts, keyed by the codes the engine's log carries (target rules, block reasons, event kinds):
// when the rules change, these texts change, not the screens. No DOM: tested in node.

import { uname, groupNames, listNames, hpOf, x100, plural, committed, hitsScreensFirst, viewerSide, pairOf, termLabel, termGroup } from "./battle_log.js";

// ---------------------------------------------------------------------------------------------
// The codex: the rules a viewer can learn, and the lessons that teach them

export const CODEX = [
  ["slots", "Slots", "Slots"],
  ["roles", "Roles and ranks", "Roles"],
  ["shield", "Shielding", "Shield"],
  ["flankers", "Flankers and screens", "Flankers"],
  ["a_vs_dv", "A against Dv", "A vs Dv"],
  ["raises", "What raises A and Dv", "Bonuses"],
  ["luck", "Luck", "Luck"],
  ["all_at_once", "All at once", "At once"],
  ["opening", "The opening", "Opening"],
  ["withdrawal", "Withdrawal", "Withdraw"],
  ["losers_fate", "The loser's fate", "Loser's fate"],
  ["reserves", "Reserves and splash", "Reserves"],
  ["assassins", "Assassins and bodyguards", "Assassins"],
  ["fear_cap", "Fear and 40 rounds", "Fear, 40 rds"],
];

/** Lesson id -> the codex entry it teaches. */
export const LESSON_CODEX = {
  slots: "slots",
  roles: "roles",
  reserves: "reserves",
  shield_plan: "shield",
  shielded: "shield",
  shield_down: "shield",
  opening: "opening",
  a_vs_dv: "a_vs_dv",
  raises: "raises",
  luck: "luck",
  flankers: "flankers",
  flank_around: "flankers",
  all_at_once: "all_at_once",
  withdrawal: "withdrawal",
  losers_fate: "losers_fate",
  morale: "fear_cap",
  draw: "fear_cap",
};

/** The lessons that may pause the battle, in this order, at most `PAUSE_BUDGET` a battle. */
export const PAUSE_LESSONS = ["a_vs_dv", "shield_down", "losers_fate"];
export const PAUSE_BUDGET = 2;
/** Secondary lessons: at most `SECONDARY_BUDGET` in full a battle; the rest show as markers and
 *  wait, uncounted, for the next battle. */
export const SECONDARY = ["flankers", "raises", "luck", "all_at_once"];
export const SECONDARY_BUDGET = 2;

/** The lessons the plan screen shows (counted when the plan is left for the battle). */
export const PLAN_LESSONS = ["slots", "roles", "reserves", "shield_plan"];

// ---------------------------------------------------------------------------------------------
// The ledger: per viewer, in browser storage (a convenience: losing it brings the callouts back)

export const LEDGER_KEY = "ffh-battle-ledger-v1";

/** The browser's storage, or null where it is blocked. */
function browserStorage() {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export class Ledger {
  /** `storage`: an object with getItem/setItem (localStorage by default; null: in memory only). */
  constructor(storage = browserStorage()) {
    this.storage = storage;
    this.data = Ledger.fresh();
    try {
      const s = storage?.getItem(LEDGER_KEY);
      if (s) this.data = { ...Ledger.fresh(), ...JSON.parse(s) };
    } catch {
      /* blocked or corrupt: start over */
    }
  }

  static fresh() {
    return { battles: 0, seen: {}, rules: {}, settings: {} };
  }

  save() {
    try {
      this.storage?.setItem(LEDGER_KEY, JSON.stringify(this.data));
    } catch {
      /* storage may be blocked or full */
    }
  }

  /** How many times lesson `id` has been shown. */
  seen(id) {
    return this.data.seen[id] ?? 0;
  }

  /** How lesson `id` shows now: "full" (the first time), "short" (the second), "icon" (after). */
  level(id) {
    const n = this.seen(id);
    return n === 0 ? "full" : n === 1 ? "short" : "icon";
  }

  /** Whether codex entry `key` has been taught. */
  codexSeen(key) {
    return Object.entries(LESSON_CODEX).some(([id, k]) => k === key && this.seen(id) > 0);
  }

  /** The rule codes (`Hit.rule`) the viewer has watched decide a target. */
  rulesSeen() {
    return Object.keys(this.data.rules);
  }

  /** A battle watched to its report: its shown lessons count once more and its rules are seen.
   *  Returns the codex keys taught for the first time. */
  recordBattle(shownLessons, log) {
    const newly = new Set();
    for (const id of shownLessons) {
      if (!this.seen(id) && LESSON_CODEX[id]) newly.add(LESSON_CODEX[id]);
      this.data.seen[id] = this.seen(id) + 1;
    }
    for (const e of log?.events ?? []) if (e.kind === "Hit") this.data.rules[e.rule] = 1;
    this.data.battles += 1;
    this.save();
    return newly;
  }

  /** The plan's own notes were shown (counted when the battle starts). */
  recordPlan() {
    for (const id of PLAN_LESSONS) if (this.level(id) !== "icon") this.data.seen[id] = this.seen(id) + 1;
    this.save();
  }

  /** "Teach less": every lesson drops to its short form at least. */
  teachLess() {
    for (const id of Object.keys(LESSON_CODEX)) this.data.seen[id] = Math.max(1, this.seen(id));
    this.save();
  }

  /** The teaching level the settings show. */
  viewer() {
    return this.data.battles === 0 && !Object.keys(this.data.seen).length ? "new" : this.data.battles < 6 ? "some" : "veteran";
  }

  /** Resets the ledger to a new player, one who has watched a few battles, or a veteran. */
  setViewer(kind) {
    const settings = this.data.settings;
    this.data = Ledger.fresh();
    this.data.settings = settings;
    const n = kind === "some" ? 1 : kind === "veteran" ? 3 : 0;
    if (n) for (const id of Object.keys(LESSON_CODEX)) this.data.seen[id] = n;
    this.data.battles = kind === "some" ? 3 : kind === "veteran" ? 12 : 0;
    if (n) for (const k of Object.keys(RULE_TEXT)) this.data.rules[k] = 1;
    this.save();
  }

  /** A setting (`watch_mode`, `show_plan`, `pause_lessons`), or `def`. */
  setting(k, def) {
    return this.data.settings[k] ?? def;
  }

  set(k, v) {
    this.data.settings[k] = v;
    this.save();
  }

  /** The watch mode for an attack of the viewer's: Full for the first three battles, then Beats
   *  (unless set). */
  watchMode() {
    return this.setting("watch_mode", null) ?? (this.data.battles < 3 ? "full" : "beats");
  }

  /** Whether an attack opens its plan: "always" (the default for five battles), "below75" (then),
   *  or "never". */
  showPlan(winPermille) {
    const s = this.setting("show_plan", null) ?? (this.data.battles < 5 ? "always" : "below75");
    return s === "always" || (s === "below75" && winPermille < 750);
  }
}

// ---------------------------------------------------------------------------------------------
// Texts keyed by codes

/** What each target rule (`Hit.rule`) means. */
export const RULE_TEXT = {
  FrontFirst: "front rank first",
  FrontRankDown: "their front rank is gone, so the back rank",
  FlankerScreensFirst: "a flanker must hit screens first",
  FlankerAround: "no screen left: it rides round to the back rank",
  FlankerFrontLast: "screens and back rank gone: the front",
  AssassinSupportFirst: "an assassin goes for support units and heroes",
  AssassinBack: "then the back rank, past the shield",
  AssassinFrontLast: "the front rank last",
  BodyguardFirst: "a bodyguard draws assassins",
};

/** A target class (`Fighter.targets`) as the ladder names it. */
export const CLASS_LABEL = { FRONT: "Front rank", BACK: "Back rank", SCREEN: "Screens", HERO_SUPPORT: "Support, heroes" };

/** Why a step of the ladder was passed (`Hit.blocked`). */
export const BLOCK_LABEL = { Empty: "none", Shielded: "shielded", VolleyImmune: "immune to volleys", AtLimit: "at its limit" };

/** A battle's result as the banner, the ribbon and the end caption say it, for the viewer. */
export function resultWords(log) {
  const o = log.outcome;
  const me = viewerSide(log);
  const city = log.plot?.city?.name;
  const youAttack = me === 0;
  const youWon = (o.result === "AttackerWon" && youAttack) || (o.result === "DefenderHeld" && !youAttack);
  if (o.result === "Draw") return { banner: "BOTH SIDES DRAW BACK", ribbon: "DRAWN", line: "both sides draw back", won: false, cls: "warn" };
  if (o.result === "Mutual") return { banner: "BOTH FORCES FALL", ribbon: "BOTH LOST", line: "nobody is left standing", won: false, cls: "warn" };
  if (o.result === "AttackerWon") {
    const what = o.plot_taken ? (city ? `${city} is ${youAttack ? "yours" : "lost"}` : `the plot is ${youAttack ? "yours" : "lost"}`) : "the plot holds";
    return { banner: youAttack ? (o.plot_taken && city ? `${city.toUpperCase()} FALLS` : "THEIR FORCE IS GONE") : "YOUR FORCE IS LOST", ribbon: youWon ? "VICTORY" : "DEFEAT", line: what, won: youWon, cls: youWon ? "good" : "bad" };
  }
  return { banner: youAttack ? "YOUR FORCE IS LOST" : `${(city ?? "THE PLOT").toUpperCase()} HOLDS`, ribbon: youWon ? "HELD" : "DEFEAT", line: youAttack ? "your force is lost" : `${city ?? "the plot"} holds`, won: youWon, cls: youWon ? "good" : "bad" };
}

/** The flankers of side `side` named for a caption ("your Horsemen"). */
function flankerNames(log, side) {
  const fl = log.fighters.filter((f) => f.side === side && committed(f) && hitsScreensFirst(f));
  if (!fl.length) return "your flankers";
  const names = [...new Set(fl.map((f) => plural(f.name)))];
  return `your ${listNames(fl.length > 1 ? names : [fl[0].name])}`;
}

/** A lesson's title and body, from the log and the lesson's context. */
export function lessonText(id, log, ctx = {}) {
  const me = viewerSide(log);
  switch (id) {
    case "opening":
      return ["The opening", "Before the melee, units with first strikes shoot a volley, and nobody hits back. Archers on the walls also strike at each attacker as it comes in."];
    case "a_vs_dv": {
      const h = ctx.hit;
      const p = ctx.pair;
      if (!h || !p) return ["A against Dv", "A hit does up to 20 damage: the hitter's share of A + Dv decides how much."];
      const share = Math.round((100 * p.a_x100) / (p.a_x100 + p.dv_x100));
      return [
        `Why ${hpOf(h.dmg)} damage?`,
        `A hit does up to 20. The hitter's share of A + Dv decides how much: A ${x100(p.a_x100)} against Dv ${x100(p.dv_x100)} is ${share}% of 20 = ${(p.mean / 100).toFixed(1)}; luck (×0.8 to ×1.2) made it ${hpOf(h.dmg)}. Hills, walls and fortification only raise Dv.`,
      ];
    }
    case "raises":
      return ["Bonuses against a kind of unit", `${ctx.text ?? ""} A bonus against a kind of unit counts only when that kind hits or is hit.`];
    case "luck":
      return ["Luck", `Same two units, ${ctx.a} then ${ctx.b} damage: every hit is multiplied by a luck roll of 0.8 to 1.2.`];
    case "shielded":
      return ["Shielded", "Their back rank cannot be hit while their front rank stands. Only flankers and assassins reach past."];
    case "flankers": {
      const who = ctx.hit ? uname(log, ctx.hit.h, { side: true, cap: true }) : "A flanker";
      return ["Flankers hit screens first", `${who} must hit their screen first.${ctx.formation ? ` ${ctx.formation}` : ""}`];
    }
    case "flank_around":
      return ["Round the front rank", `Their screen is gone, so ${ctx.hit ? uname(log, ctx.hit.h, { side: true }) : "your flanker"} rides round the front rank to any unit of their back rank (ringed), picked at random.`];
    case "all_at_once":
      return ["All at once", `Targets are chosen at the start of the round and every hit lands together at its end. ${ctx.text ?? ""}`];
    case "shield_down":
      return ["Their shield is down", `Their front rank is gone: from round ${(ctx.n ?? 0) + 1} every unit of yours can hit their back rank. A back rank is safe only while its front stands.`];
    case "withdrawal": {
      const e = ctx.ev;
      if (!e) return ["Withdrawal", "Below 40% HP a unit that can withdraw rolls once to leave the battle alive."];
      return ["Withdrawal", `Below 40% HP a unit that can withdraw rolls once: ${e.chance}%. ${uname(log, e.unit, { cap: true })} rolled ${e.roll}: ${e.ok ? "it leaves the battle alive, with its wounds." : "it must fight on."}`];
    }
    case "losers_fate": {
      const o = log.outcome;
      const city = log.plot?.city?.name;
      const youWon = (o.result === "AttackerWon") === (me === 0);
      if (o.result === "Draw") return lessonText("draw", log, ctx);
      return youWon
        ? ["The loser loses every unit it sent", `Their whole committed force is gone${log.plot?.city ? ", and in a city nobody can pull out" : ""}.${o.plot_taken ? ` No defender is left: ${city ?? "the plot"} is yours.` : ""} Units that stay out are never at risk, except from splash.`]
        : ["The loser loses every unit it sent", "Every unit you committed is gone, except those that pulled out. Units that stayed out were never at risk."];
    }
    case "draw":
      return ["Forty rounds", "After 40 rounds both sides draw back and the plot does not change hands. Every unit that still stands lives; the loser's fate falls on nobody."];
    case "morale":
      return ["Fear", "Units that fear their enemy roll before the battle; those that fail sit it out, unhurt."];
    case "slots":
      return ["Slots", "Each side fights with at most this many units. Hills, forests and rivers take slots; every 25% of a city's defense takes one from the attacker."];
    case "roles":
      return ["Roles and ranks", "Front-rank units shield the back rank. Flankers (riders) must hit screens first, then ride round to the back rank."];
    case "reserves":
      return ["Stays out", "Units that do not fight are safe whatever happens, except from splash damage."];
    case "shield_plan":
      return ["Shielded", "A back rank cannot be hit while its front rank stands."];
    default:
      return [id, ""];
  }
}

// ---------------------------------------------------------------------------------------------
// Which lesson each segment teaches, and in which form

/**
 * Gives the segments of a battle their lessons (DESIGN.md 6): `s.lesson` `{id, mode ("pause" |
 * "full" | "short"), trigger, ctx, anchor}`, at most one a segment, and `s.icons` (the lessons
 * shown as `?` markers). `level(id)` is the ledger's; at most two pauses and two secondary lessons
 * in full a battle. Only the viewer's own hits teach.
 */
export function assignLessons(segs, log, level) {
  const F = log.fighters;
  const me = viewerSide(log);
  const taught = new Set();
  let pauses = 0;
  let secondary = 0;
  const give = (s, id, trigger, ctx, anchor, override = false) => {
    if (taught.has(id)) return false;
    const lvl = level(id);
    if (lvl === "icon") {
      (s.icons ??= []).push({ id, ctx, anchor });
      taught.add(id);
      return true;
    }
    if (s.lesson && !override) return false;
    if (SECONDARY.includes(id) && lvl === "full" && secondary >= SECONDARY_BUDGET) {
      (s.icons ??= []).push({ id, ctx, anchor, deferred: true });
      taught.add(id);
      return true;
    }
    if (SECONDARY.includes(id) && lvl === "full") secondary++;
    if (s.lesson) {
      // An override (the shield breaking) takes the segment's lesson, which waits for a later moment.
      taught.delete(s.lesson.id);
      if (s.lesson.mode === "pause") pauses--;
      if (s.lesson.mode === "full" && SECONDARY.includes(s.lesson.id)) secondary--;
    }
    let mode = lvl;
    if (lvl === "full" && PAUSE_LESSONS.includes(id) && pauses < PAUSE_BUDGET) {
      mode = "pause";
      pauses++;
    }
    s.lesson = { id, mode, trigger, ctx, anchor };
    taught.add(id);
    return true;
  };
  const mineHit = (h) => F[h.h].side === me;
  const lastDmg = new Map();
  for (const s of segs) {
    if (s.type === "morale" && s.morale.length) give(s, "morale", 300, {}, { b: s.morale[0].ev.unit });
    if (s.type === "strikes" && s.strikes.length) give(s, "opening", 200, {}, { b: s.strikes[0].ev.striker ?? s.strikes[0].ev.source });
    if (s.type === "round" && s.kind === "volley" && s.hits.length) give(s, "opening", 200, {}, { b: s.hits[0].h });
    if (s.type === "round" && s.kind === "round") {
      const fh = s.focus >= 0 ? s.hits[s.focus] : null;
      if (fh && mineHit(fh)) give(s, "a_vs_dv", null, { hit: fh.ev, pair: pairOf(log.pairs, fh.h, fh.t), focus: true }, { card: true });
      const sh = s.hits.find((h) => mineHit(h) && h.ev.blocked?.some((b) => b[1] === "Shielded") && h.ev.rule !== "FlankerAround");
      if (sh) give(s, "shielded", null, {}, { band: 1 - me });
      const fl = s.hits.find((h) => mineHit(h) && h.ev.rule === "FlankerScreensFirst");
      if (fl) {
        const p = pairOf(log.pairs, fl.h, fl.t);
        const extra = p?.dv_extra?.[0];
        const formation = extra ? `${termLabel(extra)}: ${extra.pct >= 0 ? "+" : ""}${extra.pct}% Dv for their ${F[fl.t].name} when riders hit it.` : null;
        give(s, "flankers", null, { hit: fl.ev, formation }, { hitIdx: s.hits.indexOf(fl) });
      }
      const fa = s.hits.find((h) => mineHit(h) && h.ev.rule === "FlankerAround");
      if (fa) give(s, "flank_around", null, { hit: fa.ev }, { hitIdx: s.hits.indexOf(fa), rings: true });
      const vs = s.hits.find((h) => {
        const p = pairOf(log.pairs, h.h, h.t);
        return mineHit(h) && (p?.dv_extra?.some((t) => termGroup(t) === "vs") || p?.a_extra?.some((t) => termGroup(t) === "vs"));
      });
      if (vs) {
        const p = pairOf(log.pairs, vs.h, vs.t);
        const dvx = p.dv_extra?.find((t) => termGroup(t) === "vs");
        const ax = p.a_extra?.find((t) => termGroup(t) === "vs");
        const text = dvx
          ? `${termLabel(dvx)}: ${dvx.pct >= 0 ? "+" : ""}${dvx.pct}% Dv when ${uname(log, vs.h, { side: true })} hits it.`
          : `${termLabel(ax)}: ${ax.pct >= 0 ? "+" : ""}${ax.pct}% A for ${uname(log, vs.h, { side: true })} against it.`;
        give(s, "raises", null, { text }, { hitIdx: s.hits.indexOf(vs) });
      }
      for (const h of s.hits) {
        const k = `${h.h}:${h.t}`;
        const prev = lastDmg.get(k);
        if (prev !== undefined && mineHit(h) && Math.abs(hpOf(prev) - hpOf(h.dmg)) >= 2) {
          give(s, "luck", null, { a: hpOf(prev), b: hpOf(h.dmg) }, { hitIdx: s.hits.indexOf(h) });
          break;
        }
      }
      for (const h of s.hits) lastDmg.set(`${h.h}:${h.t}`, h.dmg);
      const over = s.deaths.find((d) => d.overkill >= 300 && F[d.b].side !== me);
      if (over) give(s, "all_at_once", "land", { text: `${uname(log, over.b, { cap: true })} still struck as it fell, and ${hpOf(over.overkill)} of your damage went into a dead man.` }, { b: over.b });
    }
    if (s.shieldDowns.some((x) => x.side !== me)) give(s, "shield_down", "shield", { n: s.n }, { band: 1 - me }, true);
    const w = s.wds[0];
    if (w && !s.lesson) give(s, "withdrawal", "wd", { ev: w.ev }, { b: w.ev.unit });
    if (s.type === "end") give(s, s.result === "Draw" ? "draw" : "losers_fate", 600, {}, { center: true });
  }
}

// ---------------------------------------------------------------------------------------------
// Captions: one or two plain sentences a segment, from its events

export function captionFor(seg, log, script) {
  const F = log.fighters;
  const me = viewerSide(log);
  const name = (b, o) => uname(log, b, o);
  if (seg.type === "morale") {
    const sat = seg.satOut.map((x) => name(x.ev.unit, { side: true }));
    return sat.length ? `Before the battle: ${listNames(sat)} ${sat.length > 1 ? "lose their nerve and sit" : "loses its nerve and sits"} it out.` : "Before the battle, the afraid roll for their nerve: all stand.";
  }
  if (seg.type === "strikes") {
    const ds = seg.strikes.filter((x) => x.ev.kind === "DefStrike");
    const col = seg.strikes.filter((x) => x.ev.kind === "Collateral");
    const parts = [];
    if (ds.length) {
      const hurt = ds.filter((x) => x.ev.dmg > 0);
      const glance = ds.length - hurt.length;
      parts.push(
        `From the walls, ${ds.length} of their archers strike first as you come in` +
          (hurt.length ? `: ${listNames(hurt.map((x) => `${name(x.ev.target)} −${hpOf(x.ev.dmg)}`))}${glance ? `; ${glance} ${glance > 1 ? "glance" : "glances"} off armour` : ""}.` : ", but they glance off your armour."),
      );
    }
    if (col.length) parts.push(`Splash: ${listNames([...new Set(col.map((x) => name(x.ev.target)))])} ${col.length > 1 ? "are" : "is"} hit before the melee.`);
    return parts.join(" ");
  }
  if (seg.type === "end") {
    const r = log.outcome.result;
    const city = log.plot?.city?.name;
    if (r === "Draw") return "Forty rounds: both sides draw back, and the plot does not change hands.";
    if (r === "Mutual") return "Both sides are gone.";
    if ((r === "AttackerWon") === (me === 0)) return me === 0 ? `Their force is gone. ${log.outcome.plot_taken ? (city ? `${city} falls.` : "The plot is yours.") : ""}` : "Their force is gone. You hold.";
    return me === 0 ? "Your force is gone. They hold." : `Your garrison is gone.${log.outcome.plot_taken && city ? ` ${city} falls.` : ""}`;
  }
  if (seg.type === "post") {
    const xp = log.events.filter((e) => e.kind === "Xp" && F[e.unit]?.side === me).reduce((a, e) => a + e.amount, 0);
    const moved = seg.post.find((x) => x.ev.kind === "MoveIn");
    const heals = seg.post.filter((x) => x.ev.kind === "Heal" && F[x.ev.unit]?.side === me);
    return [
      xp ? `Survivors gain ${xp} XP between them.` : "",
      heals.length ? `${listNames(heals.map((x) => name(x.ev.unit)))} ${heals.length > 1 ? "are" : "is"} healed.` : "",
      moved && log.plot?.city && me === 0 ? `Your army marches into ${log.plot.city.name}.` : moved && me === 0 ? "Your army moves in." : "",
    ]
      .filter(Boolean)
      .join(" ");
  }
  const parts = [];
  if (seg.kind === "volley") {
    const ours = [...new Set(seg.hits.filter((h) => F[h.h].side === me).map((h) => plural(F[h.h].name)))];
    const theirs = seg.hits.some((h) => F[h.h].side !== me);
    parts.push(`Opening volley: only units with first strikes shoot${ours.length ? ` (your ${listNames(ours)}${theirs ? ", and theirs" : ""})` : theirs ? " (theirs)" : ""}. Nobody hits back yet.`);
  }
  for (const side of [1 - me, me]) {
    const d = seg.deaths.filter((x) => F[x.b].side === side).map((x) => name(x.b));
    if (d.length) parts.push(`${side === me ? "Your" : "Their"} ${listNames(d)} ${d.length > 1 ? "fall" : "falls"}.`);
  }
  if (seg.screenGone) parts.push(`Their screen is gone: ${flankerNames(log, me)} can ride round to their back rank.`);
  for (const s of seg.shieldDowns) parts.push(s.side !== me ? `Their front rank is gone: from round ${seg.n + 1} everyone can hit their back rank.` : "Your front rank is gone: they can reach your back rank.");
  for (const w of seg.wds) {
    const e = w.ev;
    parts.push(`${name(e.unit, { side: true, cap: true })}, at ${e.hp_pct}%, rolls to pull out: ${e.chance}%, rolled ${e.roll}. ${e.ok ? "It leaves the battle." : "It must fight on."}`);
  }
  if (!parts.length && seg.kind === "round") {
    const dealt = seg.hits.filter((h) => F[h.h].side === me).reduce((a, h) => a + h.dmg, 0);
    const took = seg.hits.filter((h) => F[h.h].side !== me).reduce((a, h) => a + h.dmg, 0);
    let s = `Round ${seg.n}: you deal ${hpOf(dealt)}, take ${hpOf(took)}.`;
    const mine = seg.hits.filter((h) => F[h.h].side === me);
    if (mine.some((h) => h.ev.rule === "FrontRankDown")) s += " Their front rank is gone: everyone hits their back rank now.";
    else if (mine.some((h) => h.ev.rule === "FlankerAround")) s += ` ${flankerNames(log, me).replace(/^your/, "Your")} ride round to their back rank; the rest still hit the front rank.`;
    else if (mine.some((h) => h.ev.rule === "FlankerScreensFirst")) s += ` ${flankerNames(log, me).replace(/^your/, "Your")} are held up by their screen.`;
    else if (mine.some((h) => h.ev.blocked?.some((b) => b[1] === "Shielded"))) s += " Their back rank is still shielded.";
    return s;
  }
  return parts.slice(0, 3).join(" ");
}

/** The plan's per-unit note about its targets ("Targets: 1 Screens → 2 Back rank → 3 Front rank"). */
export function targetsText(f) {
  const order = (f.targets ?? []).map((c, k) => `${k + 1} ${CLASS_LABEL[c] ?? c}`).join(" → ");
  return order ? `Targets: ${order}${f.ignores_shield ? " (reaches past the shield)" : ""}` : "";
}

/** The units of side `side` that stay out, for the stakes ("Archer L1 and Mage stay out, unhurt"). */
export function stayOutText(log, bs) {
  return bs.length ? `${groupNames(log, bs)} stay out, unhurt.` : "Everyone fights.";
}

