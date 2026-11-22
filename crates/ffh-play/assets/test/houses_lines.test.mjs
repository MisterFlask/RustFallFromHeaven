// node --test: this game's Houses' pure parts, from app/houses_lines.js (docs/HOUSES.md, rule 15):
// a card's sections, the previews of an act and of a deal, the alerts' entries, a plate's shield
// (its colour, its mark, its box and its tooltip).
import test from "node:test";
import assert from "node:assert/strict";
import * as L from "../app/houses_lines.js";
import * as P from "../app/plates.js";

const house = {
  id: 3,
  label: "House Aldric",
  lines: ["Alignment: Good", "Religion: Order", "Sympathy: the Lanun", "Perk: Horsemasters I"],
  perk: { name: "Horsemasters I", lines: ["Mounted Units trained: Flanking I."], on: true },
  perk_off: null,
  opinion_line: "Opinion: Pleased (1)",
  happiness_line: "Happiness: +1 in 4 cities.",
  state: "Pleased",
};

test("a card's sections: the leaning lines, the perk's name and lines, the opinion, the happiness", () => {
  const s = L.cardSections(house);
  assert.deepEqual(s.leaning, ["Alignment: Good", "Religion: Order", "Sympathy: the Lanun"]);
  assert.equal(s.perk, "Perk: Horsemasters I");
  assert.deepEqual(s.perkLines, ["Mounted Units trained: Flanking I."]);
  assert.equal(s.perkOff, null);
  assert.equal(s.opinion, "Opinion: Pleased (1)");
  assert.equal(s.happiness, "Happiness: +1 in 4 cities.");
  const off = L.cardSections({ ...house, perk: { ...house.perk, on: false }, perk_off: "Perk off: the House is Furious." });
  assert.equal(off.perkOff, "Perk off: the House is Furious.");
  // The outsider's line stays among the leaning lines; a card without lines has none.
  const outsider = L.cardSections({ lines: ["Alignment: Good", "Religion: none", "Outsider: -3 happiness in this House's cities. No sympathy.", "Perk: Merchants I"] });
  assert.deepEqual(outsider.leaning, ["Alignment: Good", "Religion: none", "Outsider: -3 happiness in this House's cities. No sympathy."]);
  assert.deepEqual(L.cardSections({}).leaning, []);
  assert.equal(L.cardSections(null).perk, null);
});

test("the previews: one list of lines; a deal's by the items it holds", () => {
  const list = [{ lines: ["House Aldric: Pleased to Annoyed. Happiness -3 in 4 cities."] }, { lines: ["House Bertram: Annoyed to Furious. Happiness -2 in 1 city.", "Its perk goes off."] }];
  assert.deepEqual(L.previewLines(list), ["House Aldric: Pleased to Annoyed. Happiness -3 in 4 cities.", "House Bertram: Annoyed to Furious. Happiness -2 in 1 city.", "Its perk goes off."]);
  assert.deepEqual(L.previewLines(null), []);
  const pv = { peace: [{ lines: ["a"] }], open_borders: [{ lines: ["b"] }], defensive_pact: [{ lines: ["c"] }] };
  assert.deepEqual(L.dealPreview(pv, ["TRADE_PEACE_TREATY", "TRADE_GOLD"]), ["a"]);
  assert.deepEqual(L.dealPreview(pv, ["TRADE_OPEN_BORDERS", "TRADE_DEFENSIVE_PACT"]), ["b", "c"]);
  assert.deepEqual(L.dealPreview(pv, ["TRADE_GOLD"]), []);
  assert.deepEqual(L.dealPreview(null, ["TRADE_PEACE_TREATY"]), []);
});

test("the alerts: the Houses' messages, this turn's only on a first read; the House they name", () => {
  const entries = [
    { t: 4, text: "House Aldric forms: Tovin Cinderwright leads Ventry.", mark: "house", kind: "house", house: 3 },
    { t: 5, text: "Astrum joins House Aldric.", mark: "house", kind: "house", house: 3 },
    { t: 5, text: "A battle.", kind: "engagement" },
  ];
  assert.equal(L.alertEntries(entries).length, 2);
  assert.deepEqual(L.alertEntries(entries, { fresh: true, turn: 5 }).map((e) => e.text), ["Astrum joins House Aldric."]);
  const brief = [{ id: 3, leader: 12, leader_name: "Tovin Cinderwright" }];
  assert.equal(L.alertHouse(entries[0], brief).leader, 12);
  assert.equal(L.alertHouse({ house: 9 }, brief), null);
  assert.equal(L.alertHouse({}, brief), null);
});

test("a plate's shield: a colour per House id, the mark of an Annoyed or Furious House, its box and tooltip", () => {
  assert.equal(L.houseColour(0), L.HOUSE_COLOURS[0]);
  assert.equal(L.houseColour(L.HOUSE_COLOURS.length + 2), L.HOUSE_COLOURS[2]);
  assert.notEqual(L.houseColour(1), L.houseColour(2));
  assert.equal(L.shieldMark("Pleased"), null);
  assert.equal(L.shieldMark("Annoyed").glyph, "◆");
  assert.equal(L.shieldMark("furious").glyph, "✖");
  const city = { id: 7, house: { id: 3, label: "House Aldric", state: "Pleased" } };
  const hit = L.shieldHit(city, 100, 50, 18);
  assert.equal(hit.kind, "house");
  assert.equal(hit.city, 7);
  assert.ok(hit.x1 - hit.x0 < 18 && hit.y1 - hit.y0 === 18);
  assert.equal(L.shieldHit({ id: 8 }, 0, 0, 18), null);
  assert.equal(P.plateHitAt([hit], 100, 50).kind, "house");
  const tip = P.plateTip("house", city);
  assert.equal(tip.title, "House Aldric: Pleased");
  assert.deepEqual(tip.reminders, [{ keyword: "KEYWORD_HOUSE" }]);
  assert.equal(P.plateTip("house", { id: 8 }), null);
});

test("the states: a name, a colour and a class for any case", () => {
  assert.equal(L.stateName("annoyed"), "Annoyed");
  assert.equal(L.stateName("nonsense"), "Cautious");
  assert.equal(L.stateClass("Furious"), "hs-furious");
  assert.match(L.stateColour("Friendly"), /^#[0-9a-f]{6}$/);
});
