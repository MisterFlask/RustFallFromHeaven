// node --test: engagement combat in the client's words and marks (app/engagements.js,
// docs/ENGAGEMENTS.md): a battle's outcome for the viewer, its line, a unit's battle numbers, and
// the map's markers from the view's battles.
import test from "node:test";
import assert from "node:assert/strict";
import { S } from "../app/state.js";
import { outcomeFor, battleLine, battleMarkers, battleById, battleTerrain, engageNumbers, actionsLine, battlesToShow } from "../app/engagements.js";

const battle = (extra = {}) => ({
  id: 3,
  turn: 4,
  x: 10,
  y: 5,
  from: { x: 9, y: 5 },
  place: "Prespur",
  city: 1,
  viewer_side: 0,
  plot_taken: true,
  city_captured: 2,
  has_log: true,
  sides: [{ lost: 0 }, { lost: 3 }],
  ...extra,
});

test("a battle's outcome is the viewer's: the plot taken or not", () => {
  assert.equal(outcomeFor(battle()), "won");
  assert.equal(outcomeFor(battle({ plot_taken: false, city_captured: null })), "repelled");
  assert.equal(outcomeFor(battle({ viewer_side: 1 })), "lost");
  assert.equal(outcomeFor(battle({ viewer_side: 1, plot_taken: false })), "held");
});

test("a battle's line names the place, the outcome and both sides' losses as the viewer sees them", () => {
  assert.equal(battleLine(battle()), "Prespur: won, the city taken; 0 lost, 3 of theirs");
  assert.equal(battleLine(battle({ city_captured: null, place: null })), "(10, 5): won, the plot taken; 0 lost, 3 of theirs");
  assert.equal(battleLine(battle({ plot_taken: false, city_captured: null })), "Prespur: they hold; 0 lost, 3 of theirs");
  assert.equal(battleLine(battle({ viewer_side: 1, plot_taken: false, place: null })), "(10, 5): held; 3 lost, 0 of theirs");
});

test("a unit's battle numbers: its stats, HP, speed, large, and its actions by row", () => {
  const u = { hp: 50, max_hp: 100, engage_attack: 440, engage_defense: 500, engage_max_hp: 120 };
  assert.equal(engageNumbers(u), "ATK 44 · DEF 50 · HP 60/120");
  const acts = [{ kind: "Charge", n: 2 }, { kind: "Strike", n: 1 }, { kind: "None", n: 0 }];
  assert.equal(actionsLine(acts), "2 charges / 1 strike / —");
  assert.equal(
    engageNumbers({ ...u, engage_magic: 1, engage_magic_defense: 320, engage_accuracy: 90, engage_evasion: 10, engage_crit: 10, engage_speed: 6, engage_large: true, engage_actions: acts }),
    "ATK 44 · DEF 50 · HP 60/120 · MDEF 32 · ACC 90 · EVA 10 · CRIT 10 · speed 6 · large · front / middle / rear: 2 charges / 1 strike / —",
  );
  // A caster's magic.
  assert.equal(engageNumbers({ ...u, engage_magic: 300 }), "ATK 44 · DEF 50 · HP 60/120 · MAG 30");
  assert.equal(actionsLine(null), "");
});

test("the map's markers: one per plot, the newest battle there (the view lists newest first)", () => {
  S.view = { battles: [battle({ id: 7, plot_taken: false, city_captured: null }), battle({ id: 3 }), battle({ id: 2, x: 4, y: 4, has_log: false })] };
  const m = battleMarkers();
  assert.deepEqual(
    m.map((x) => [x.id, x.x, x.y, x.kind, x.has_log]),
    [
      [7, 10, 5, "repelled", true],
      [2, 4, 4, "won", false],
    ],
  );
  assert.match(m[0].title, /^Battle #7, Prespur: they hold/);
  assert.equal(battleById(3)?.id, 3);
  assert.equal(battleById(99), null);
  S.view = null;
  assert.deepEqual(battleMarkers(), []);
});

test("a battle is a land attack on land (the rules' engage_applies)", () => {
  const saved = { view: S.view, data: S.data, map: S.map, units: S.units };
  S.data = {
    tables: {
      DomainType: [{ type: "DOMAIN_SEA" }, { type: "DOMAIN_AIR" }, { type: "DOMAIN_LAND" }],
      PlotType: [{ type: "PLOT_PEAK" }, { type: "PLOT_HILLS" }, { type: "PLOT_LAND" }, { type: "PLOT_OCEAN" }],
      UnitType: [{ type: "UNIT_AXEMAN", domain: 2 }, { type: "UNIT_TRIREME", domain: 0 }, { type: "UNIT_GRIFFON", domain: 1 }],
    },
  };
  // A 3 x 1 map: hills, land, ocean.
  S.map = { w: 3, h: 1, wrap_x: false, wrap_y: false, plot_type: [1, 2, 3] };
  S.units = new Map([
    [1, { id: 1, type: 0 }],
    [2, { id: 2, type: 1 }],
    [3, { id: 3, type: 2 }],
  ]);
  try {
    S.view = {};
    assert.equal(battleTerrain([1], 0, 0), true, "land on hills");
    assert.equal(battleTerrain([1], 1, 0), true, "land on land");
    assert.equal(battleTerrain([1], 2, 0), false, "land on water");
    assert.equal(battleTerrain([2], 2, 0), false, "a ship: the duel");
    assert.equal(battleTerrain([3], 1, 0), false, "air: the duel");
    assert.equal(battleTerrain([1], 5, 0), false, "off the map");
    assert.equal(battleTerrain([9], 1, 0), false, "an unknown unit");
  } finally {
    Object.assign(S, saved);
  }
});

test("the battles shown when the turn comes: those with a log, not shown yet, oldest first", () => {
  const list = [battle({ id: 9 }), battle({ id: 7, has_log: false }), battle({ id: 8 }), battle({ id: 5 })];
  assert.deepEqual(battlesToShow(list, new Set()), [5, 8, 9]);
  assert.deepEqual(battlesToShow(list, new Set([8])), [5, 9], "the player's own attack, shown already");
  assert.deepEqual(battlesToShow(null, new Set()), []);
});
