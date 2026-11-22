// node --test: trade's pure helpers (app/trade_view.js; docs/TRADE.md section 9): the Trade
// layer's lines and its automatic switch, the city panel's route lines, the tooltip's caravan
// lines, the Trade screen's caravan line and row tone.
import test from "node:test";
import assert from "node:assert/strict";
import * as V from "../app/trade_view.js";

const colorOf = (owner, mine) => (mine ? "own" : `p${owner}`);

test("the layer's lines: each route in its owner's colour, short segments dropped, the preview last", () => {
  const trade = {
    lines: [
      { route: 0, owner: 0, mine: true, segments: [[[1, 1], [2, 2], [3, 2]], [[9, 9]]] },
      { route: 1, owner: 3, mine: false, segments: [[[5, 5], [6, 5]]] },
      { route: 2, owner: 3, mine: false, segments: [[[7, 7]]] },
    ],
  };
  const out = V.tradeLines(trade, colorOf);
  assert.deepEqual(out.map((l) => [l.route, l.color, l.segments.length, l.preview]), [[0, "own", 1, false], [1, "p3", 1, false]]);
  const withPreview = V.tradeLines(trade, colorOf, [[[0, 0], [1, 0]]]);
  const last = withPreview.at(-1);
  assert.equal(last.preview, true);
  assert.equal(last.color, V.PREVIEW_COLOR);
  assert.ok(last.width > 1);
  assert.deepEqual(V.tradeLines(null, colorOf), []);
  assert.deepEqual(V.tradeLines({ lines: [] }, colorOf, [[[0, 0]]]), []);
});

test("a Caravan selected wants the layer; the switch goes back to the layer it replaced", () => {
  const isCaravan = (u) => u.type === 7;
  assert.equal(V.wantsTrade([{ type: 1 }], isCaravan), false);
  assert.equal(V.wantsTrade([{ type: 1 }, { type: 7 }], isCaravan), true);
  assert.equal(V.wantsTrade([{ type: 1, caravan: { route: 2 } }], isCaravan), true);
  assert.equal(V.wantsTrade(null, isCaravan), false);
  let r = V.autoTradeLayer("political", null, true);
  assert.deepEqual(r, { layer: "trade", auto: { prev: "political" } });
  r = V.autoTradeLayer(r.layer, r.auto, true);
  assert.deepEqual(r, { layer: "trade", auto: { prev: "political" } });
  assert.deepEqual(V.autoTradeLayer(r.layer, r.auto, false), { layer: "political", auto: null });
  // A layer picked meanwhile stays.
  assert.deepEqual(V.autoTradeLayer("yield", { prev: "terrain" }, false), { layer: "yield", auto: null });
  // Already on by hand: nothing to go back to.
  assert.deepEqual(V.autoTradeLayer("trade", null, false), { layer: "trade", auto: null });
});

test("the city panel's lines: from the city first, then into it", () => {
  const routes = { from: [{ route: 2, text: "To Vallus (Lanun): 2.8 a turn." }], into: [{ route: 4, text: "From Ithralia: 1.4 a turn." }], limit: "This city: 1 of 1." };
  assert.deepEqual(V.cityRouteLines(routes), [
    { kind: "from", route: 2, text: "To Vallus (Lanun): 2.8 a turn." },
    { kind: "into", route: 4, text: "From Ithralia: 1.4 a turn." },
  ]);
  assert.deepEqual(V.cityRouteLines(null), []);
  assert.deepEqual(V.cityRouteLines({ from: [], into: [] }), []);
});

test("the tooltip's caravan lines, the screen's caravan line and a row's tone", () => {
  assert.deepEqual(V.caravanTipLines({ caravan: { text: "Caravan: Ithralia to Vallus.", plunder_text: "Plunder: 14 gold." } }), ["Caravan: Ithralia to Vallus.", "Plunder: 14 gold."]);
  assert.deepEqual(V.caravanTipLines({ type: 3 }), []);
  assert.equal(V.caravanLine({ slot: 1, text: "Lost on turn 112 to the barbarians. Re-forms in 6 turns." }), "Caravan 2: Lost on turn 112 to the barbarians. Re-forms in 6 turns.");
  assert.equal(V.routeTone({ open: false, caravans: [] }), "closed");
  assert.equal(V.routeTone({ open: true, caravans: [{ state: "road" }, { state: "lost" }] }), "lost");
  assert.equal(V.routeTone({ open: true, caravans: [{ state: "road" }, { state: "road" }] }), "open");
});

test("the pay with chosen cargos: each half times its premium, as the rules' route_pay", () => {
  assert.equal(V.cargoPay([413, 412], [0, 0]), 825);
  assert.equal(V.cargoPay([413, 412], [150, 0]), Math.floor(413 * 2.5) + 412);
  assert.equal(V.cargoPay([413, 412], [150, 50]), 1032 + 618);
  assert.equal(V.payText(1650), "16.5");
  assert.equal(V.payText(825), "8.3");
  const options = [{ bonus: 3, premium: 150, warn: null }, { bonus: 5, premium: 50, warn: "Your only copy: your cities lose Silk." }];
  assert.deepEqual(V.choicePremiums(options, [3, null]), [150, 0]);
  assert.deepEqual(V.choicePremiums(options, [5, 3]), [50, 150]);
  assert.equal(V.choiceRefusal([3, 3]), "The two caravans carry different resources.");
  assert.equal(V.choiceRefusal([3, null]), null);
  assert.equal(V.choiceRefusal([null, null]), null);
  assert.deepEqual(V.choiceWarnings(options, [5, 3]), ["Your only copy: your cities lose Silk."]);
  assert.deepEqual(V.choiceWarnings(options, [3, null]), []);
});

test("an Escort of a caravan in the stations' selectors", () => {
  assert.equal(V.caravanStationValue({ kind: "escort", route: 2, slot: 1 }), "caravan:2:1");
  assert.equal(V.caravanStationValue({ kind: "escort", army: 4, route: null }), null);
  assert.equal(V.caravanStationValue({ kind: "raid" }), null);
  assert.deepEqual(V.caravanStation("caravan:2:1"), { escort_caravan: 2, slot: 1 });
  assert.equal(V.caravanStation("escort:4"), null);
});

test("the tooltip's caravan lines carry the cargo", () => {
  assert.deepEqual(V.caravanTipLines({ caravan: { text: "Caravan: A to B.", cargo_text: "Cargo: Gems.", plunder_text: "Plunder: 42 gold." } }), ["Caravan: A to B.", "Cargo: Gems.", "Plunder: 42 gold."]);
});
