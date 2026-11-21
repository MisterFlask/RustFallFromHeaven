// node --test: squads in words (app/squads.js; docs/ENGAGEMENTS.md, "Squads"): the numerals, a
// unit's best row and the board's ! for a unit outside it, the strip's groups; and the ground's new
// rules in the battle screens' words (app/battle_log.js).
import test from "node:test";
import assert from "node:assert/strict";
import { roman, squadName, bestRow, offRowText, stripGroups } from "../app/squads.js";
import * as L from "../app/battle_log.js";

const archer = { id: 7, engage_actions: [{ kind: "Shoot", n: 1 }, { kind: "Shoot", n: 2 }, { kind: "Shoot", n: 2 }] };
const warrior = { id: 8, engage_actions: [{ kind: "Strike", n: 2 }, { kind: "Strike", n: 1 }, { kind: "Strike", n: 1 }] };

test("squad numerals", () => {
  assert.deepEqual([1, 2, 3, 4, 9].map(roman), ["I", "II", "III", "IV", "IX"]);
  assert.equal(squadName({ num: 2 }), "Squad II");
});

test("a unit's best row and the ! for a unit outside it", () => {
  assert.equal(bestRow(archer), 1, "ties: the first");
  assert.equal(bestRow(warrior), 0);
  assert.equal(bestRow({}), null);
  assert.equal(offRowText(archer, 2), null, "as many actions as its best row");
  assert.equal(offRowText(archer, 0), "Front: 1 shot. Best: Middle, 2 shots.");
  assert.equal(offRowText(warrior, 2), "Rear: 1 strike. Best: Front, 2 strikes.");
  assert.equal(offRowText({ engage_actions: [{ kind: "None", n: 0 }, { kind: "Bombard", n: 1 }, { kind: "Bombard", n: 2 }] }, 0), "Front: no action. Best: Rear, 2 bombards.");
});

test("the strip's groups follow the squads, the units in none last", () => {
  const units = [{ id: 1 }, { id: 2 }, { id: 3 }, { id: 4 }];
  const squads = [
    { id: 10, num: 1, cells: [null, 2, null, null, 1, null, null, null, null] },
    { id: 11, num: 2, cells: [null, 3, null, null, null, null, null, null, null] },
    { id: 12, num: 3, cells: [null, 99, null, null, null, null, null, null, null] },
  ];
  const g = stripGroups(units, squads);
  assert.deepEqual(g.map((x) => [x.squad?.id ?? null, x.units.map((u) => u.id)]), [[10, [1, 2]], [11, [3]], [null, [4]]]);
});

test("the ground's squad-era rules in words", () => {
  const rule = (kind, pct, side, kinds, first_round, units = "all") => ({ kind, pct, side, kinds, first_round, units });
  const forest = { city: false, forest: true, hills: false, open: false, marsh: false, crossing: false, flanking: false, rules: [rule("ForestRanged", 50, null, ["Shoot", "Bombard"], false), rule("ForestStab", 150, null, ["Stab"], false), rule("LargeDealt", 75, null, null, false, "large")] };
  assert.deepEqual(L.groundRules(forest, null), ["Forest: arrows and stones deal half damage on both sides; stabs deal 150%; large units deal 75%; no flanking."]);
  const open = { city: false, forest: false, hills: false, open: true, marsh: false, crossing: true, flanking: true, rules: [rule("LargeDealt", 120, null, null, false, "large"), rule("Wading", 50, 0, null, true, "not_river"), rule("ChargeFirstRound", 200, null, ["Charge"], true)] };
  assert.deepEqual(L.groundRules(open, null), [
    "Open ground: large units deal 120%; cavalry flanks, and its first-round charges deal double damage.",
    "Wading: the attackers' units without river crossing deal 50% in round 1.",
  ]);
  const hills = { city: false, forest: false, hills: true, open: false, marsh: false, crossing: false, flanking: true, rules: [rule("HillsAttacker", 85, 0, null, false), rule("HillsDefenderRanged", 125, 1, ["Shoot", "Bombard"], false), rule("Trench", 60, 0, null, true), rule("Uphill", 200, 1, ["Charge"], true)] };
  assert.deepEqual(L.groundRules(hills, null), [
    "Hills: the attackers deal 15% less damage; the defenders' arrows and stones deal 25% more; cavalry flanks; no charge bonus for the attacker; the defenders' first-round charges deal double damage.",
    "Trench: the attackers deal 60% in round 1.",
  ]);
  const marsh = { city: false, forest: false, hills: false, open: false, marsh: true, crossing: false, flanking: false, rules: [rule("LargeDealt", 75, null, null, false, "large")] };
  assert.deepEqual(L.groundRules(marsh, null), ["Marsh: large units deal 75%; no flanking."]);
  // Whose hits a rule changes: a large unit's, a unit's that does not cross rivers.
  assert.equal(L.groundPct(open, 0, "Strike"), 100);
  assert.equal(L.groundPct(open, 0, "Strike", { large: true }), 120);
  assert.equal(L.groundPct(open, 0, "Strike", { firstRound: true }), 50);
  assert.equal(L.groundPct(open, 0, "Strike", { firstRound: true, river: true }), 100);
  assert.equal(L.groundPct(open, 1, "Strike", { firstRound: true }), 100, "the defender does not wade");
  // On hills only the defender's charges get the first-round bonus.
  assert.equal(L.groundPct(hills, 0, "Charge", { firstRound: true }), 51, "85% and the Trench's 60%");
  assert.equal(L.groundPct(hills, 1, "Charge", { firstRound: true }), 200);
  assert.equal(L.chargeRule(hills, 0), null);
  assert.equal(L.chargeRule(hills, 1)?.pct, 200);
});
