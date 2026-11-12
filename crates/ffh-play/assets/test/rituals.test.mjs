// node --test: the world rituals' helpers, from app/ritual_rows.js (docs/HUMAN.md): which rows the
// HUD element shows, their status words and accessible names, which log entries are alerts.
import test from "node:test";
import assert from "node:assert/strict";
import * as R from "../app/ritual_rows.js";

const row = (o) => ({ player: 0, name: "Hippus", ritual_name: "Ritual: Warcry", city_name: "Lycus", percent: 0, turns_left: null, ...o });

test("the HUD shows rituals in progress, queued by you, and charges held", () => {
  const rows = [row({ player: 0, state: "in_progress" }), row({ player: 1, state: "idle" }), row({ player: 2, state: "charged" }), row({ player: 3, state: "queued" })];
  assert.deepEqual(
    R.activeRows(rows).map((r) => r.player),
    [0, 2, 3],
  );
  assert.deepEqual(R.activeRows(undefined), []);
});

test("status words and accessible names", () => {
  assert.equal(R.turnsText(1), "1 turn");
  assert.equal(R.turnsText(6), "6 turns");
  const going = row({ state: "in_progress", percent: 37, turns_left: 6 });
  assert.deepEqual(R.rowStatus(going), { charged: false, pct: "37%", turns: "6 turns", text: "37%, 6 turns" });
  assert.equal(R.rowLabel(going), "Hippus: Ritual: Warcry in Lycus, 37%, 6 turns");
  const stalled = row({ state: "in_progress", percent: 5, turns_left: null });
  assert.equal(R.rowStatus(stalled).text, "5%");
  const held = row({ state: "charged", name: "Unknown civilization" });
  assert.equal(R.rowStatus(held).text, "Charge held");
  assert.equal(R.rowLabel(held), "Unknown civilization: Ritual: Warcry in Lycus, charge held");
  assert.deepEqual(R.listLines({ ...going, progress: 37, cost: 100, completed: 0, next_cost: 200 }), ["In progress in Lycus: 37/100 (37%, 6 turns).", "Completed: 0. Cost: 100. Next cost: 200."]);
  assert.equal(R.listLines(row({ state: "charged" }))[0], "Charge held in Lycus.");
  assert.equal(R.listLines(row({ state: "idle", city_name: null, completed: 1, cost: 200, next_cost: 400 }))[1], "Completed: 1. Cost: 200. Next cost: 400.");
  assert.equal(R.listLines(row({ state: "idle" }))[0], "Idle.");
  assert.equal(R.chipName(going), "Warcry");
  assert.equal(R.chipName(row({ ritual_name: "Sanctuary" })), "Sanctuary");
});

test("ritual news alerts: new entries, or the current turn's on a first read", () => {
  const entries = [
    { i: 0, t: 3, kind: "ritual", verb: "began" },
    { i: 1, t: 4, text: "x" },
    { i: 2, t: 4, kind: "ritual", verb: "completed" },
    { i: 3, t: 4, kind: "engagement" },
  ];
  assert.deepEqual(
    R.ritualAlerts(entries, { fresh: false, turn: 4 }).map((e) => e.i),
    [0, 2],
  );
  assert.deepEqual(
    R.ritualAlerts(entries, { fresh: true, turn: 4 }).map((e) => e.i),
    [2],
  );
});

test("world actions: the caster is placed by a city, and options on plots are picked on the map", () => {
  const player = { target_kind: "player", caster_where: "near Prespur", targets: [{ at: null }] };
  assert.equal(R.casterLine(player, "Scout"), "Cast by your Scout near Prespur; then pick its target (a player).");
  assert.equal(R.casterLine({ ...player, caster_where: "in Prespur" }, "Scout"), "Cast by your Scout in Prespur; then pick its target (a player).");
  assert.equal(R.casterLine({ ...player, caster_where: null }, "Scout"), "Cast by your Scout; then pick its target (a player).");
  assert.equal(R.casterLine({ caster_where: "in Lycus" }, "Warrior"), "Cast by your Warrior in Lycus.");
  const onPlots = { target_kind: "choice", caster_where: "in Cyrus", targets: [{ at: [3, 4], label: "Air Mana" }, { at: [3, 4], label: "Fire Mana" }] };
  assert.equal(R.choicesOnPlots(onPlots), true);
  const traits = { target_kind: "choice", targets: [{ at: null, label: "Aggressive" }] };
  assert.equal(R.choicesOnPlots(traits), false);
  assert.equal(R.casterLine(traits, "Adept"), "Cast by your Adept; then pick its target (one of its options).");
  assert.equal(R.choicesOnPlots({ target_kind: "plot", targets: [{ at: [1, 1] }] }), false);
});

// A world spell the player places (Wellspring): the world action as the server sends it.
const place = (held) => ({
  spell: 7,
  place_on_charge: true,
  enabled: held,
  ...(held ? { place: { completed: "Ritual: Wellspring completed. Place the mana node: click a marked plot.", pick: "Place the mana node of Wellspring: click a marked plot.", later: "Esc: later." } } : {}),
});

test("a placement opens by itself only when its charge appears, not on a loaded game", () => {
  const seen = new Map();
  // First view (a loaded game, a reload) with the charge held: nothing opens.
  assert.deepEqual(R.placementDue(seen, [place(true)]), []);
  assert.deepEqual(R.placementDue(seen, [place(true)]), [], "held still: postponed, not reopened");
  assert.deepEqual(R.placementDue(seen, [place(false)]), [], "placed: the charge is gone");
  assert.deepEqual(R.placementDue(seen, [place(true)]), [7], "the ritual completed again");
  assert.deepEqual(R.placementDue(seen, [place(true)]), []);
  // Other world actions are not placements.
  assert.deepEqual(R.placementDue(new Map([[3, false]]), [{ spell: 3, enabled: true, place: {} }]), []);
  assert.deepEqual(R.placementDue(seen, undefined), []);
});

test("the placement's prompt", () => {
  assert.equal(R.placePrompt(place(true), { completed: true }), "Ritual: Wellspring completed. Place the mana node: click a marked plot.");
  assert.equal(R.placePrompt(place(true)), "Place the mana node of Wellspring: click a marked plot.");
  assert.equal(R.placePrompt(place(true), { touch: true }), "Place the mana node of Wellspring: tap a marked plot.");
  assert.equal(R.placePrompt(place(false)), "");
});

test("the mana menu's rows: passive lines and the sphere promotions' tooltips", () => {
  assert.deepEqual(R.passiveText({ passive: ["+1 health in each of your cities that has it."] }), ["+1 health in each of your cities that has it."]);
  assert.deepEqual(R.passiveText({ passive: [] }), ["none."]);
  const fire1 = { name: "Fire I", needs: "Needs Channeling I and Fire Mana.", free: "Free to new Arcane Units with 2 or more Fire Mana.", spells: [{ name: "Blaze", help: "[ICON_BULLET]Creates Smoke" }] };
  assert.deepEqual(R.promoTipParts(fire1), {
    title: "Fire I",
    lines: ["Needs Channeling I and Fire Mana.", "Free to new Arcane Units with 2 or more Fire Mana."],
    spells: [{ name: "Blaze", help: "[ICON_BULLET]Creates Smoke" }],
    none: null,
  });
  assert.equal(R.promoTipParts({ name: "Ice III", needs: "", spells: [] }).none, "Opens no spell.");
});
