// node --test: waypoints in words and values (app/armies.js, docs/ARMIES.md): the city screen's
// line, its selector's values and the command's `to`, an army card's "Waypoint of".
import test from "node:test";
import assert from "node:assert/strict";
import { waypointText, waypointValue, waypointTo, waypointOfText } from "../app/armies.js";

test("the city screen's waypoint line", () => {
  assert.equal(waypointText({ kind: "none", name: null }), "Waypoint: none");
  assert.equal(waypointText({ kind: "army", army: 2, name: "the 2nd Army" }), "Waypoint: the 2nd Army");
  assert.equal(waypointText({ kind: "plot", x: 14, y: 9, name: "plot 14, 9" }), "Waypoint: plot 14, 9");
  assert.equal(waypointText(null), "Waypoint: none");
});

test("the selector's values round-trip to the command's `to`", () => {
  assert.equal(waypointValue({ kind: "none" }), "none");
  assert.equal(waypointValue({ kind: "army", army: 3 }), "army:3");
  assert.equal(waypointValue({ kind: "plot", x: 14, y: 9 }), "plot:14,9");
  assert.equal(waypointTo("none"), null);
  assert.deepEqual(waypointTo("army:3"), { army: 3 });
  assert.deepEqual(waypointTo("plot:14,9"), { plot: [14, 9] });
});

test("an army card's Waypoint of", () => {
  assert.equal(waypointOfText({ waypoint_cities: [] }), "Waypoint of: no city");
  assert.equal(waypointOfText({ waypoint_cities: [{ city: 1, name: "Vallus" }, { city: 4, name: "Torrolerial" }] }), "Waypoint of: Vallus, Torrolerial");
});
