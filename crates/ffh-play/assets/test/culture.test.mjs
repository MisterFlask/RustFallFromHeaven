// node --test: the Culture screen's numbers and words (app/culture.js; docs/CULTURE.md, "UI";
// docs/PERSONS.md, "The fourth build", rule 10): hundredths with one decimal, favour and
// requirements as whole numbers, the head's lines, the requirement's table, the Guard button's
// preview, the city screen's line. Every number comes from the server; these only format it.
import test from "node:test";
import assert from "node:assert/strict";
import { tenths, signed, whole, chancePct, plots, reachText, headLines, splitWhen, prospect, turnsText, requirementRows, courtRows, favourText, favourBar, guardLossLine, guardButtonLine, guardCell, setLabel, cityCultureLine, otherLine, outOf } from "../app/culture.js";

test("hundredths with one decimal, whole numbers, chances", () => {
  assert.equal(tenths(640), "6.4");
  assert.equal(tenths(1280), "12.8");
  assert.equal(tenths(1200), "12.0");
  assert.equal(tenths(0), "0.0");
  assert.equal(tenths(-320), "−3.2");
  assert.equal(tenths(645), "6.5", "rounded to the tenth");
  assert.equal(signed(330), "+3.3");
  assert.equal(signed(330, true), "−3.3");
  assert.equal(signed(0, true), "0.0");
  assert.equal(whole(8099), "80", "favour as the rules' lines write it");
  assert.equal(whole(0), "0");
  assert.equal(chancePct(2500), "25%");
  assert.equal(chancePct(1250), "13%");
  assert.equal(chancePct(260), "2.6%");
  assert.equal(plots(1), "1 plot");
  assert.equal(reachText({ distance: 7, effect: 51 }), "7 plots: 51%");
  assert.equal(reachText(null), "none");
});

const court = { name: "Court House Aldric", since_turn: 140, action: 0, effect: "court" };

test("the head: the channel and its inflow, or what a channel would take", () => {
  assert.deepEqual(headLines({ channel: court, inflow: 640, base: 1280, percent: 50, refusal: null }), ["Channel: Court House Aldric, since turn 140.", "Inflow: 6.4 culture a turn (half of 12.8)."]);
  assert.deepEqual(headLines({ channel: null, inflow: 640, base: 1280, percent: 50, refusal: null }), ["Channel: none. Your cities keep all their culture.", "A channel would take 6.4 culture a turn (half of 12.8)."]);
  assert.deepEqual(headLines({ channel: null, inflow: 0, base: 0, percent: 50, refusal: "Only a civilization that can have Houses channels its culture." }), ["Channel: none. Your cities keep all their culture.", "Only a civilization that can have Houses channels its culture."]);
  assert.equal(headLines({ channel: null, inflow: 350, base: 1000, percent: 35, refusal: null })[1], "A channel would take 3.5 culture a turn (35% of 10.0).");
});

test("the split's turns and where a favour goes", () => {
  assert.equal(splitWhen(null), null);
  assert.equal(splitWhen(1), "a split next turn");
  assert.equal(splitWhen(6), "a split in ~6 turns");
  assert.equal(prospect(6, 0, 300), "a split in ~6 turns");
  assert.equal(prospect(null, 4099, 300), "settling near 40");
  assert.equal(prospect(null, 0, 0), "fading");
  assert.equal(turnsText(null), "Never at these inflows.");
  assert.equal(turnsText(1), "Next turn.");
  assert.equal(turnsText(31), "In ~31 turns.");
});

/** A courtship as the view sends it (`courtship_json`). */
const cv = {
  house: 2,
  label: "House Aldric",
  liege: 1,
  liege_name: "Bannor",
  courter: 0,
  courter_name: "Sheaim",
  favour: 8012,
  gain: 330,
  reach: { distance: 7, effect: 51 },
  guard: 120,
  requirement: { total: 168, loyalty: 3, attitude: 1, attitude_name: "Pleased", unit: 24, formula: "(3 + 6 − 2 × 1) × 24 = 168" },
  settles: 6612,
  turns: null,
  attention: false,
};

test("the requirement's table and the courtship's numbers", () => {
  assert.deepEqual(requirementRows(cv, 0), [
    ["Loyalty of House Aldric to the Bannor", "3"],
    ["Attitude of its leader toward you", "Pleased (+1)"],
    ["World culture unit", "24"],
    ["Requirement", "(3 + 6 − 2 × 1) × 24 = 168"],
  ]);
  const rows = requirementRows({ ...cv, requirement: { ...cv.requirement, loyalty: -2, attitude: -1, attitude_name: "Annoyed" } }, 5);
  assert.deepEqual(rows[0], ["Loyalty of House Aldric to the Bannor", "−2"]);
  assert.deepEqual(rows[1], ["Attitude of its leader toward the Sheaim", "Annoyed (−1)"], "another courter: named");
  assert.deepEqual(courtRows(cv), [
    ["Reach", "7 plots: 51%"],
    ["Gain", "+3.3 a turn"],
    ["Guard", "−1.2 a turn"],
    ["Settles near", "66"],
    ["Split", "Never at these inflows."],
  ]);
  assert.deepEqual(courtRows({ ...cv, guard: 0, turns: 12 }).slice(2), [
    ["Guard", "none"],
    ["Settles near", "66"],
    ["Split", "In ~12 turns."],
  ]);
  assert.equal(favourText(cv.favour, 168), "Favour 80 of 168");
  assert.deepEqual(favourBar(8400, 168), { fill: 50, mark: 200 / 3 });
  assert.equal(favourBar(99999, 168).fill, 100, "a favour past the requirement fills the bar");
});

test("the Guard button's preview and the guard table's cells", () => {
  const gv = {
    loss: 320,
    courtships: [{ courter: 0, courter_name: "Sheaim", favour: 13000, requirement: 188, gain: 600, settles_with: 4000, settles_without: 21000, turns_with: null, turns_without: 6 }],
  };
  assert.equal(guardLossLine(gv), "Guard: −3.2 a turn to each courtship.");
  assert.equal(guardButtonLine(gv), "Guard: −3.2 a turn to each courtship. The Sheaim: a split in ~6 turns without it, settling near 40 with it.");
  assert.equal(guardButtonLine({ loss: 0, courtships: [] }), "Guard: 0.0 a turn to each courtship.");
  assert.equal(guardCell(null, 4000, 600), "near 40; no split");
  assert.equal(guardCell(12, 21000, 600), "near 210; a split in ~12 turns");
  assert.equal(guardCell(null, 0, 0), "fading; no split");
});

test("the Set button, the city screen's line, the channels between others, the memories", () => {
  assert.equal(setLabel(null), "Set");
  assert.equal(setLabel(court), "Set (replaces Court House Aldric)");
  assert.equal(cityCultureLine({ rate: 1280, diverted: 640, disorder: false, channel: court }), "Culture: 12.8 a turn; 6.4 of it to Court House Aldric.");
  assert.equal(cityCultureLine({ rate: 1280, diverted: 0, disorder: true, channel: court }), "Culture: 12.8 a turn; nothing diverted in disorder.");
  assert.equal(cityCultureLine({ rate: 1280, diverted: 0, disorder: false, channel: null }), "Culture: 12.8 a turn; no channel.");
  assert.equal(otherLine({ caster_name: "Sheaim", name: "Court House Bertram", since_turn: 130, target: { house: 3, liege_name: "Kuriotates" } }), "Sheaim: Court House Bertram (Kuriotates), since turn 130.");
  assert.equal(otherLine({ caster_name: "Sheaim", name: "Spread Religion (Esus) to the Kuriotates", since_turn: 90, target: { player: 4, name: "Kuriotates" } }), "Sheaim: Spread Religion (Esus) to the Kuriotates, since turn 90.");
  assert.equal(outOf(2, 8), "2 of 8");
});
