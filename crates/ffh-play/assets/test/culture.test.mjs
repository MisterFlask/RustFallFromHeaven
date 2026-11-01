// node --test: the cultural channels' numbers and words (app/culture.js): hundredths of culture
// and of a percent for people, how far revolt pressure goes, where it settles and when it gets
// there, as the rules run a turn (docs/CULTURE.md).
import test from "node:test";
import assert from "node:assert/strict";
import { pts, chancePct, pressureStage, unrestOf, outcome, settleOf, turnsTo, turnsBelow, nextStage, reachRow, revoltSummary } from "../app/culture.js";
import { revoltMark } from "../app/iso.js";

// The defaults of data/xml/RulesDefines.xml, as the server's `thresholds` sends them.
const th = { unhappy_step: 2500, unhappy_max: 4, disorder: 10000, near_flip: 20000, flip: 30000, fade_percent: 5, resist_percent: 50, garrison: 200, spread_max: 2500, flipping: true };

test("hundredths of culture and of a percent", () => {
  assert.equal(pts(0), "0");
  assert.equal(pts(306), "3.1");
  assert.equal(pts(300), "3");
  assert.equal(pts(4210), "42");
  assert.equal(pts(996), "10");
  assert.equal(chancePct(2500), "25%");
  assert.equal(chancePct(1250), "13%");
  assert.equal(chancePct(260), "2.6%");
  assert.equal(chancePct(34), "0.3%");
  assert.equal(chancePct(0), "0%");
});

test("stages and their words", () => {
  assert.equal(pressureStage(2499, th), 0);
  assert.equal(pressureStage(2500, th), 1);
  assert.equal(pressureStage(10000, th), 2);
  assert.equal(pressureStage(20000, th), 3);
  assert.equal(pressureStage(30000, th), 4);
  assert.equal(pressureStage(30000, th, true), 3, "a capital never changes hands");
  assert.equal(pressureStage(30000, { ...th, flipping: false }), 3);
  assert.equal(unrestOf(7600, th), 3);
  assert.equal(unrestOf(50000, th), 4);
  assert.equal(outcome(0, th), "no effect");
  assert.equal(outcome(5000, th), "unrest (−2 happy)");
  assert.equal(outcome(12000, th), "revolt (disorder)");
  assert.equal(outcome(31000, th, { who: "the Sheaim" }), "the city joins the Sheaim");
  assert.match(outcome(31000, th, { capital: true }), /capital never/);
});

/** A city's resistance as the server sends it: `resist` now, `resist_revolt` (its defenders'
 *  part), `rest` (the other civilizations' pressure), `in_revolt`. */
const city = (resist, resist_revolt = resist, rest = 0, in_revolt = false) => ({ resist, resist_revolt, rest, in_revolt });

/** The rules' turns (`city_do_revolt_pressure`), written out: +inflow, then -resistance (the
 *  defenders' alone in a revolt, which follows a turn whose total reached it) and
 *  -max(5%, 1 culture). The pressure after each of `n` turns. */
function rulesTurns(now, inflow, r, n) {
  let p = now;
  let revolt = r.in_revolt;
  const out = [];
  for (let t = 1; t <= n; t++) {
    const q = p + inflow;
    p = Math.max(0, q - (revolt ? r.resist_revolt : r.resist) - Math.max(Math.floor((q * 5) / 100), 100));
    revolt = p + r.rest >= th.disorder;
    out.push(p);
  }
  return out;
}

test("where pressure settles and when it gets there", () => {
  // 10 culture a turn channelled 7 plots away (51%): +5.1 a turn against 2 resisted settles
  // near 19 x 5.1 - 20 x 2 = 56.9 where the city's turn weighs it.
  const inflow = 510;
  const r = city(200);
  const s = settleOf(0, inflow, r, th);
  assert.ok(Math.abs(s - 5690) <= 20, `settles at ${s}`);
  assert.ok(Math.abs(settleOf(9000, inflow, r, th) - 5690) <= 20, "from above, down to the same point");
  assert.equal(settleOf(0, 100, city(300), th), 0);
  // Less than a culture a turn net: the minimum fade eats it.
  assert.equal(settleOf(0, 250, city(200), th), 0);
  // Never more than it settles at; never at all when the city resists it all.
  assert.equal(turnsTo(0, inflow, r, 10000, th), null);
  assert.equal(turnsTo(0, 100, city(300), 2500, th), null);
  const n = turnsTo(0, inflow, r, 2500, th);
  assert.ok(n > 0 && n < 20, `unrest after ${n} turns`);
  assert.equal(turnsTo(3000, inflow, r, 2500, th), 0);
  // The turn it gets there, by the rules' step.
  const ps = rulesTurns(0, inflow, r, n);
  assert.ok(ps[n - 1] >= 2500 && (n === 1 || ps[n - 2] < 2500));
  assert.equal(nextStage(0, inflow, r, th), `unrest in ~${n} turns`);
  assert.equal(nextStage(6000, inflow, r, th), null, "as far as it settles");
  // Without inflow it fades: below unrest after a few turns.
  const f = turnsBelow(5000, city(300), 2500, th);
  assert.ok(f > 0 && f < 10, `${f} turns`);
});

test("a revolt takes the city's culture out of its resistance", () => {
  // +18 a turn against 10 (8 of half its culture, 2 of its defender). Its full resistance alone
  // would hold the pressure near 19 x 18 - 20 x 10 = 142, past the revolt at 100: in revolt it
  // makes no culture, only the defender resists, and the pressure runs on to 19 x 18 - 20 x 2 =
  // 302, past the flip at 300.
  const r = city(1000, 200);
  const s = settleOf(0, 1800, r, th);
  assert.ok(Math.abs(s - 30200) <= 20, `settles at ${s}`);
  assert.equal(outcome(s, th, { who: "the Sheaim" }), "the city joins the Sheaim");
  assert.equal(outcome(s, th, { capital: true }), "revolt; a capital never changes hands");
  // When: the revolt, then the flip, on the turns the rules take.
  const ps = rulesTurns(0, 1800, r, 200);
  const revoltAt = ps.findIndex((p) => p >= th.disorder) + 1;
  const flipAt = ps.findIndex((p) => p >= th.flip) + 1;
  assert.ok(revoltAt > 0 && flipAt > revoltAt, `${revoltAt}, ${flipAt}`);
  assert.equal(turnsTo(0, 1800, r, th.disorder, th), revoltAt);
  assert.equal(turnsTo(0, 1800, r, th.flip, th), flipAt);
  assert.equal(nextStage(0, 1800, r, th), `unrest in ~${turnsTo(0, 1800, r, th.unhappy_step, th)} turns`);
  assert.equal(nextStage(25000, 1800, r, th), `changes hands in ~${turnsTo(25000, 1800, r, th.flip, th)} turns`);
  // Held below the revolt, its culture keeps resisting: 19 x 12 - 20 x 10 = 28.
  const low = settleOf(0, 1200, r, th);
  assert.ok(Math.abs(low - 2800) <= 20, `settles at ${low}`);
  assert.equal(outcome(low, th), "unrest (−1 happy)");
  // The others' 80 bring the total to the revolt: then 19 x 12 - 20 x 2 = 188, "revolt" at once
  // with the others' pressure, and it never gets to the flip.
  const pushed = city(1000, 200, 8000);
  const s2 = settleOf(0, 1200, pushed, th);
  assert.ok(Math.abs(s2 - 18800) <= 20, `settles at ${s2}`);
  assert.equal(outcome(s2, th, { rest: 8000 }), "revolt (disorder)");
  assert.equal(nextStage(0, 1200, pushed, th), `revolt in ~${turnsTo(0, 1200, pushed, th.disorder - 8000, th)} turns`);
  // In revolt now: the defenders alone resist from the start (its full resistance cannot be read).
  const inRevolt = city(200, 200, 0, true);
  assert.ok(Math.abs(settleOf(12000, 1200, inRevolt, th) - 18800) <= 20);
  // Fading in revolt: only the defenders and the fade take it out until the total is below.
  assert.ok(turnsBelow(12000, inRevolt, th.unhappy_step, th) > turnsBelow(12000, city(1000, 200), th.unhappy_step, th));
});

test("the chooser's line names what the strongest city in reach would do", () => {
  // A capital settles past the flip but never changes hands; a farther city settles in revolt.
  const capital = { city: 1, distance: 3, amount: 1800, others: 0, now: 0, capital: true, ...city(1000, 200) };
  const town = { city: 2, distance: 6, amount: 1200, others: 0, now: 0, capital: false, ...city(1000, 200, 8000) };
  const line = revoltSummary({ reach: [capital, town], unseen: 1 }, th);
  assert.equal(line, "3 cities in reach · up to revolt; a capital never changes hands");
  assert.doesNotMatch(line, /joins/);
  // The same city out of a capital joins you.
  assert.equal(revoltSummary({ reach: [{ ...capital, capital: false }, town] }, th), "2 cities in reach · up to the city joins you");
  // The highest stage first: a city in revolt beats one with more pressure that only stirs unrest.
  const unrest = { city: 3, distance: 2, amount: 1500, others: 0, now: 0, capital: false, ...city(1000, 200) };
  const revolt = { ...unrest, city: 4, amount: 400, ...city(200, 200, 9800) };
  assert.equal(reachRow(revolt, th).stage, 2);
  assert.ok(reachRow(unrest, th).settle > reachRow(revolt, th).settle);
  assert.equal(revoltSummary({ reach: [unrest, revolt] }, th), "2 cities in reach · up to revolt (disorder)");
  assert.equal(revoltSummary({ reach: [{ ...unrest, amount: 100 }] }, th), "1 city in reach · too weak for an effect");
  assert.equal(revoltSummary({ reach: [], unseen: 2 }, th), "2 cities in reach");
});

test("the map's revolt marks by level", () => {
  assert.equal(revoltMark({}), null);
  assert.equal(revoltMark({ revolt: 0 }).glyph, "");
  assert.equal(revoltMark({ revolt: 1, unrest: 2 }).glyph, "!");
  assert.match(revoltMark({ revolt: 1, unrest: 2 }).title, /−2 happy/);
  assert.notEqual(revoltMark({ revolt: 2 }).fill, revoltMark({ revolt: 1 }).fill);
  assert.equal(revoltMark({ revolt: 3 }).glyph, "!!");
});
