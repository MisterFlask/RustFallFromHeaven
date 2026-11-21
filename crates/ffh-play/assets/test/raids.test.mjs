// node --test: how a raided city reads on the maps (app/raids.js): the banner's labels and the
// HD-2D plate's ring.
import test from "node:test";
import assert from "node:assert/strict";
import * as R from "../app/raids.js";

const overrun = { state: "overrun", label: "Overrun" };
const contested = { state: "contested", label: "Contested" };
const stripped = { state: "stripped", label: "Stripped (12 turns)" };

test("the banner appends the siege's label, then the raid's", () => {
  assert.equal(R.plateTag({ name: "Vallus" }), "");
  assert.equal(R.plateTag({ raid: overrun }), " · Overrun");
  assert.equal(R.plateTag({ siege: { label: "Hunger 2/3" }, raid: contested }), " · Hunger 2/3 · Contested");
  assert.equal(R.plateTag({ siege: { label: "Invested" }, raid: null }), " · Invested");
  assert.equal(R.plateTag(null), "");
});

test("the plate's ring: solid red besieged, dashed amber contested or overrun, none stripped", () => {
  assert.deepEqual(R.plateRing({ raid: overrun }), { colour: R.RAID_RING, dash: [6, 4] });
  assert.deepEqual(R.plateRing({ raid: contested }), { colour: R.RAID_RING, dash: [6, 4] });
  assert.equal(R.plateRing({ raid: stripped }), null);
  assert.deepEqual(R.plateRing({ besieged: true, raid: overrun }), { colour: R.SIEGE_RING, dash: [] });
  assert.deepEqual(R.plateRing({ besieged: true }), { colour: R.SIEGE_RING, dash: [] });
  assert.equal(R.plateRing({}), null);
  assert.notEqual(R.RAID_RING, R.SIEGE_RING);
});

test("a raid line's tone", () => {
  assert.equal(R.raidTone(overrun), "warn");
  assert.equal(R.raidTone(contested), "warn");
  assert.equal(R.raidTone(stripped), "dim");
  assert.equal(R.raidTone(null), "");
});
