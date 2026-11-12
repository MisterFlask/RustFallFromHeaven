// node --test: the world rituals' helpers, from app/ritual_rows.js (docs/HUMAN.md): which rows the
// HUD element shows, their status words and accessible names, which log entries are alerts.
import test from "node:test";
import assert from "node:assert/strict";
import * as R from "../app/ritual_rows.js";

const row = (o) => ({ player: 0, name: "Hippus", ritual_name: "Ritual: Warcry", city_name: "Lycus", percent: 0, turns_left: null, ...o });

test("the HUD shows rituals in progress, queued by you, and charges held", () => {
  const rows = [row({ player: 0, state: "in_progress" }), row({ player: 1, state: "idle" }), row({ player: 2, state: "pending" }), row({ player: 3, state: "queued" })];
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
  assert.deepEqual(R.rowStatus(going), { pending: false, pct: "37%", turns: "6 turns", text: "37%, 6 turns" });
  assert.equal(R.rowLabel(going), "Hippus: Ritual: Warcry in Lycus, 37%, 6 turns");
  const stalled = row({ state: "in_progress", percent: 5, turns_left: null });
  assert.equal(R.rowStatus(stalled).text, "5%");
  const held = row({ state: "pending", name: "Unknown civilization", city_name: null });
  assert.equal(R.rowStatus(held).text, "Cast pending");
  assert.equal(R.rowLabel(held), "Unknown civilization: Ritual: Warcry, cast pending");
  assert.deepEqual(R.listLines({ ...going, progress: 37, cost: 100, completed: 0, next_cost: 200 }), ["In progress in Lycus: 37/100 (37%, 6 turns).", "Completed: 0. Cost: 100. Next cost: 200."]);
  assert.equal(R.listLines(row({ state: "pending" }))[0], "Cast pending.");
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
  pending: held,
  enabled: held,
  ...(held ? { place: { completed: "Ritual: Wellspring completed. Place the mana node: click a marked plot.", pick: "Place the mana node of Wellspring: click a marked plot." } } : {}),
});

test("a pending cast's pick opens by itself once a turn", () => {
  const opened = new Set();
  assert.deepEqual(R.castsDue(opened, [place(true)], 5), [7]);
  assert.deepEqual(R.castsDue(opened, [place(true)], 5), [], "closed with Escape: not reopened this turn");
  assert.deepEqual(R.castsDue(opened, [place(true)], 6), [7], "the next turn");
  assert.deepEqual(R.castsDue(opened, [place(false)], 7), [], "nothing pending");
  // A targeted cast opens too; one with no valid target (disabled) or with no target does not.
  assert.deepEqual(R.castsDue(new Set(), [{ spell: 3, pending: true, enabled: true, target_kind: "plot" }], 1), [3]);
  assert.deepEqual(R.castsDue(new Set(), [{ spell: 3, pending: true, enabled: false, target_kind: "plot" }], 1), []);
  assert.deepEqual(R.castsDue(new Set(), [{ spell: 3, pending: false, enabled: true, target_kind: "plot" }], 1), []);
  assert.deepEqual(R.castsDue(opened, undefined, 1), []);
});

test("the placement's prompt", () => {
  assert.equal(R.placePrompt(place(true), { completed: true }), "Ritual: Wellspring completed. Place the mana node: click a marked plot.");
  assert.equal(R.placePrompt(place(true)), "Place the mana node of Wellspring: click a marked plot.");
  assert.equal(R.placePrompt(place(true), { touch: true }), "Place the mana node of Wellspring: tap a marked plot.");
  assert.equal(R.placePrompt(place(false)), "");
});

test("the mana menu's rows: the tier marks', spell chips' and effect chips' tooltips", () => {
  const fire1 = { name: "Fire I", needs: "Needs Channeling I and Fire Mana.", free: "Free to new Arcane Units with 2 or more Fire Mana.", spells: [] };
  assert.deepEqual(R.tierTipParts(fire1), { title: "Fire I", lines: ["Needs Channeling I and Fire Mana.", "Free to new Arcane Units with 2 or more Fire Mana."] });
  assert.deepEqual(R.tierTipParts({ name: "Ice III", needs: "" }).lines, []);
  const blaze = { name: "Blaze", needs: "Needs Fire I.", help: "[ICON_BULLET]Creates Smoke" };
  assert.deepEqual(R.spellTipParts(blaze), { title: "Blaze", lines: ["Needs Fire I."], help: "[ICON_BULLET]Creates Smoke" });
  assert.equal(R.spellLabel(blaze), "Blaze. Needs Fire I.");
  assert.equal(R.spellTipParts({ name: "Wave" }).help, "");
  assert.equal(R.chipGlyph("health"), "♥");
  assert.equal(R.chipGlyph("info"), "i");
  assert.equal(R.chipArt("happiness"), "happy");
  assert.equal(R.chipArt("buildings"), null);
});
