// node --test: the battle screens' pure module (app/battle_log.js: formation battles, phase 6
// revision 4) over the dev fixture (app/battle_fixture.js, made by make_battle_fixtures.mjs from
// the owner's prototype): the log folded into states (the fold's fates are the outcome's counts,
// nobody acts or is hit off the field, no two units share a cell after a knock-back), the log's
// lines, the opening's events, the editing of a formation (moves, swaps, large units, the slots),
// the ground's rules in words, and damage in words (the replay's pace is battle_snap.test.mjs's).
import test from "node:test";
import assert from "node:assert/strict";
import * as L from "../app/battle_log.js";
import { setData } from "../app/state.js";

import { FIXTURES } from "../app/battle_fixture.js";

setData({
  kinds: ["UnitType", "CombatRoleType", "SpellType", "FeatureType"],
  tables: {
    UnitType: [{ type: "UNIT_AXEMAN", name: "Axeman" }],
    // the data's roles, in its order (Units/CIV4CombatRoleInfos.xml)
    CombatRoleType: ["FRONTLINE", "SCREEN", "FLANKER", "ASSASSIN", "ARCHER", "SIEGE", "ARCANE", "DIVINE"].map((r) => ({ type: `COMBATROLE_${r}`, name: r[0] + r.slice(1).toLowerCase() })),
    SpellType: [{ type: "SPELL_MAELSTROM", name: "Maelstrom" }],
    FeatureType: [{ type: "FEATURE_FOREST", name: "Forest" }],
  },
});

const logs = Object.entries(FIXTURES).flatMap(([name, fx]) => fx.logs.map((l, k) => ({ name: `${name}#${k}`, log: l.log, formation: l.formation })));
const field = FIXTURES.field.logs[0].log;

test("types resolve by index or by name; roles by their key", () => {
  assert.equal(L.tyName("UnitType", 0), "UNIT_AXEMAN");
  assert.equal(L.tyIdx("UnitType", "UNIT_AXEMAN"), 0);
  assert.equal(L.tyLabel("UnitType", "UNIT_WAR_ELEPHANT"), "War Elephant");
  assert.equal(L.roleKey({ role: 2 }), "flanker");
  assert.equal(L.roleKey({ role: 7 }), "divine");
  assert.equal(L.roleKey({ role: "COMBATROLE_ARCANE" }), "arcane");
  assert.equal(L.roleLabel({ role: "COMBATROLE_SIEGE" }), "Siege");
  assert.equal(L.kindKey("STRIKE"), "Strike");
  assert.equal(L.kindKey("bombard"), "Bombard");
  assert.equal(L.kindKey("whatever"), "None");
});

test("names: several of a name numbered in the fighters' order, sides", () => {
  const axes = field.fighters.filter((f) => f.side === 0 && f.name === "Axeman");
  assert.equal(axes.length, 2);
  assert.deepEqual(
    axes.map((f) => L.uname(field, f.b)),
    ["Axeman 1", "Axeman 2"],
  );
  assert.match(L.uname(field, axes[0].b, { side: true, cap: true }), /^Your Axeman 1$/);
  const giant = field.fighters.find((f) => f.name === "Hill Giant");
  assert.equal(L.uname(field, giant.b, { side: true }), "their Hill Giant");
  assert.equal(L.listNames(["a", "b", "c"]), "a, b and c");
  assert.equal(L.plural("Horseman"), "Horsemen");
});

test("actions and reach in words", () => {
  const mage = field.fighters.find((f) => f.name === "Mage");
  assert.equal(L.actionText(mage, 2), "2 × maelstrom");
  const adept = FIXTURES.field.preview.fighters.find((f) => f.name === "Adept");
  assert.equal(L.actionText(adept, 1), "1 × magic bolt");
  const cat = field.fighters.find((f) => f.name === "Catapult");
  assert.equal(L.actionText(cat, 0), "no action");
  assert.equal(L.actionText(cat, 2), "2 bombards");
  const champ = field.fighters.find((f) => f.name === "Champion" && f.side === 0);
  assert.equal(L.actionText(champ, 0), "2 strikes");
  assert.equal(L.actionText(champ, 1), "1 strike");
  // the charge's first-round damage is the ground's rule's, worded from its percent
  assert.match(L.reachText("Charge", field.ground), /anywhere; double damage in the first round$/);
  assert.match(L.reachText("Charge", { flanking: true, rules: [{ kind: "ChargeFirstRound", pct: 150 }] }), /anywhere; 150% damage in the first round$/);
  assert.match(L.reachText("Charge", { flanking: true, rules: [] }), /anywhere$/);
  assert.match(L.reachText("Charge", { flanking: false }), /no flanking here/);
  assert.match(L.reachText("Shoot"), /back unit/);
  // Nearby first: every damaging action but a flanking charge (docs/ENGAGEMENTS.md, Targeting).
  for (const k of ["Strike", "Stab", "Shoot", "Bombard", "Spell"]) assert.match(L.reachText(k), /its own and the next lanes first/, k);
  assert.match(L.reachText("Charge", { flanking: false }), /its own and the next lanes first/);
  assert.doesNotMatch(L.reachText("Charge", field.ground), /next lanes/);
  assert.doesNotMatch(L.reachText("Heal"), /next lanes/);
  // every special of a unit is worded from its own fields, the rule values from the params
  const elephant = field.fighters.find((f) => f.name === "War Elephant");
  const notes = L.specials(elephant, field.ground, field.params).join(" ");
  assert.match(notes, /Large/);
  assert.match(notes, /150% of its role's HP/);
  assert.match(notes, /Withdrawal 25%/);
  assert.match(notes, /first-round charges deal double damage/);
  assert.match(L.specials(elephant, { flanking: true, rules: [{ kind: "ChargeFirstRound", pct: 175 }] }, { large_hp_pct: 140 }).join(" "), /140% of its role's HP.*first-round charges deal 175% damage/);
  const bare = L.specials(elephant, { flanking: true, rules: [] }).join(" ");
  assert.doesNotMatch(bare, /150%|double|first-round/, "no rule values without the params and the ground's rule");
  assert.match(bare, /more HP than its role's/);
  const catNotes = L.specials(cat, field.ground, field.params).join(" ");
  assert.match(catNotes, /Never takes a target below 25% of its HP/);
  assert.match(catNotes, /splash .* for half damage/);
  assert.match(L.specials(cat, field.ground, { splash_pct: 40 }).join(" "), /splash .* for 40% of the damage/);
  assert.match(L.specials(adept, field.ground, field.params).join(" "), /magic bolt: 60% of a normal hit/);
  assert.match(L.specials(adept, field.ground, { bolt_pct: 70 }).join(" "), /magic bolt: 70% of a normal hit/);
  assert.match(L.specials(adept, field.ground).join(" "), /magic bolt, weaker than a spell/);
  const mageNotes = L.specials(mage, field.ground, field.params).join(" ");
  assert.match(mageNotes, /Casts Maelstrom: a full hit on its target and half damage on the target's neighbours/);
  assert.doesNotMatch(mageNotes, /Collateral damage/, "a caster's spell splashes, not its other hits");
  const quick = { ...champ, first_strikes: 2 };
  assert.match(L.specials(quick, field.ground, field.params).join(" "), /First strikes: \+4 speed/);
  assert.match(L.specials(quick, field.ground, { first_strike_speed: 3 }).join(" "), /First strikes: \+6 speed/);
  assert.match(L.specials(quick, field.ground).join(" "), /First strikes: it acts sooner/);
});

test("the ground's rules in words, and what they do to a unit's hits", () => {
  const g = (k) => FIXTURES[k].preview.ground;
  assert.deepEqual(L.groundRules(g("field"), null), ["Open ground: cavalry flanks, and its first-round charges deal double damage."]);
  assert.deepEqual(L.groundRules({ city: false, forest: false, hills: false, flanking: true, rules: [] }, null), ["Open ground: no damage changes; cavalry flanks."]);
  assert.deepEqual(L.groundRules(g("city"), FIXTURES.city.preview.plot), ["City: the attackers' arrows deal half damage; no flanking."]);
  assert.deepEqual(L.groundRules(g("forest"), FIXTURES.forest.preview.plot), ["Forest: arrows and stones deal half damage on both sides; no flanking."]);
  assert.deepEqual(L.groundRules(g("hills"), null), ["Hills: the attackers deal 25% less damage; the defenders' arrows and stones deal 25% more; cavalry flanks, and its first-round charges deal double damage."]);
  // the charge's rule: only where flanking works, and only in the first round
  for (const k of ["field", "hills"]) assert.ok(g(k).rules.some((r) => r.kind === "ChargeFirstRound" && r.pct === 200), `${k}: the charge's rule`);
  for (const k of ["city", "forest", "raid"]) assert.ok(!g(k).rules.some((r) => r.kind === "ChargeFirstRound"), `${k}: no charge's rule`);
  assert.equal(L.groundPct(g("field"), 0, "Charge"), 100);
  assert.equal(L.groundPct(g("field"), 1, "Charge", { firstRound: true }), 200);
  assert.equal(L.groundPct(g("field"), 0, "Strike", { firstRound: true }), 100);
  assert.equal(L.groundPct(g("hills"), 0, "Charge", { firstRound: true }), 150);
  const rider = { side: 0, actions: [{ kind: "Charge", n: 2 }, { kind: "Strike", n: 1 }, { kind: "Strike", n: 1 }] };
  assert.equal(L.groundNote(rider, g("field")), "");
  assert.equal(L.groundNote(rider, g("hills")), "Here its charges deal 75% (150% in the first round), strikes deal 75% of normal damage.");
  assert.equal(L.groundPct(g("city"), 0, "Shoot"), 50);
  assert.equal(L.groundPct(g("city"), 1, "Shoot"), 100);
  assert.equal(L.groundPct(g("forest"), 1, "Bombard"), 50);
  assert.equal(L.groundPct(g("hills"), 0, "Strike"), 75);
  assert.equal(L.groundPct(g("hills"), 0, "Shoot"), 75);
  assert.equal(L.groundPct(g("hills"), 1, "Shoot"), 125);
  assert.equal(L.groundPct(g("hills"), 1, "Strike"), 100);
  const archer = FIXTURES.hills.preview.fighters.find((f) => f.side === 1 && f.name === "Archer");
  assert.equal(L.groundNote(archer, g("hills")), "Here its shots deal 125% of normal damage.");
});

test("slots: the sum in words and the cap", () => {
  const s = FIXTURES.city.preview.slots[0];
  assert.equal(L.slotSum(s), "8 slots + 2 for plains − 2 for the walls' 2 Sentinels = 8");
  assert.equal(L.slotCap({ total: 6, river_only: 2 }), 8);
});

test("every log folds: its fates are the outcome's, nobody acts or is hit off the field", () => {
  for (const { name, log } of logs) {
    const st = L.initialState(log);
    const F = log.fighters;
    log.events.forEach((e, i) => {
      if (e.round > 0) {
        assert.equal(st.status[e.actor], "on", `${name} event ${i}: its actor is on the field`);
        for (const h of e.hits ?? []) assert.equal(st.status[h.target], "on", `${name} event ${i}: its target is on the field`);
        if (e.heal) assert.equal(F[e.heal.target].side, F[e.actor].side, `${name} event ${i}: heals an ally`);
      }
      L.applyEvent(st, e, F);
      // no two units in one cell (a knock-back only moves a unit into free cells)
      for (const side of [0, 1]) {
        const seen = new Set();
        F.forEach((f, b) => {
          if (f.side !== side || !st.pos[b] || st.status[b] === "dead" || st.status[b] === "gone") return;
          for (const l of L.lanesAt(f, st.pos[b])) {
            const k = `${l}:${st.pos[b].row}`;
            assert.ok(!seen.has(k), `${name} event ${i}: two units in cell ${k}`);
            seen.add(k);
          }
        });
      }
    });
    const fs = L.fates(log, st);
    for (const side of [0, 1]) {
      const o = log.outcome.sides[side];
      assert.deepEqual([fs[side].fallen.length, fs[side].escaped.length, fs[side].standing.length], [o.fallen, o.escaped, o.standing], `${name}: side ${side}'s fates`);
      for (const b of fs[side].fallen) assert.equal(st.hp[b], 0);
      for (const b of fs[side].standing) assert.ok(st.hp[b] > 0 && st.hp[b] <= F[b].max_hp * 100);
    }
    assert.deepEqual(L.finalState(log), st);
    assert.equal(L.resultWords(log).kind, log.outcome.plot_taken ? "taken" : fs[0].standing.length ? "held" : "repulsed");
  }
});

test("the state after n events is the fold of the first n", () => {
  const n = Math.floor(field.events.length / 2);
  const st = L.stateAfter(field, n);
  const copy = L.cloneState(st);
  for (let i = n; i < field.events.length; i++) L.applyEvent(st, field.events[i], field.fighters);
  assert.deepEqual(st, L.finalState(field));
  assert.notDeepEqual(copy, st, "a copy is its own");
  assert.deepEqual(L.stateAfter(field, 0), L.initialState(field));
});

test("the log's lines: a head for each round and pass, a line an event, worded from its codes", () => {
  for (const { name, log } of logs) {
    const lines = L.logLines(log);
    const acts = lines.filter((l) => !l.head);
    assert.equal(acts.length, log.events.length, name);
    assert.deepEqual(
      acts.map((l) => l.i),
      log.events.map((_, i) => i),
    );
    const heads = lines.filter((l) => l.head).map((l) => l.text);
    assert.deepEqual(heads, [...new Set(log.events.map(L.headOf))], `${name}: one head per pass, in order`);
    const text = (l) => l.parts.map((p) => p.text).join("");
    log.events.forEach((e, i) => {
      const t = text(acts[i]);
      for (const p of acts[i].parts) if (p.b !== undefined) assert.equal(p.cls, log.fighters[p.b].side === 0 ? "you" : "them");
      if (e.hits?.some((h) => h.killed)) assert.match(t, /falls/, `${name} ${i}`);
      if (e.hits?.[0]?.crit) assert.match(t, / critical/, `${name} ${i}`);
      if (e.hits?.[0]?.miss) assert.match(t, /and misses/, `${name} ${i}`);
      if (e.hits?.some((h) => h.withdrew)) assert.match(t, /escape/, `${name} ${i}`);
      if (e.knocked) assert.match(t, /is knocked back to the (middle|rear) row/, `${name} ${i}`);
      if (e.heal) assert.match(t, /heals .* \+\d+/, `${name} ${i}`);
      if (e.note === "NoOneHurt") assert.match(t, /no one to heal/);
      if (e.note === "NoTarget") assert.match(t, /no one in reach/);
      if (L.kindKey(e.kind) === "Spell" && e.hits?.length && !e.hits[0].miss) assert.match(t, /casts (Maelstrom|Magic bolt) at/);
      if (e.hits?.length > 1) assert.match(t, / splash /, `${name} ${i}`);
      if (e.round > 0) assert.equal(acts[i].speed, log.fighters[e.actor].speed);
    });
  }
  // the fixtures between them show every kind of moment
  const all = logs.flatMap(({ log }) => log.events);
  for (const [what, pred] of [
    ["a kill", (e) => e.hits?.some((h) => h.killed)],
    ["a crit", (e) => e.hits?.some((h) => h.crit)],
    ["a miss", (e) => e.hits?.some((h) => h.miss)],
    ["a heal", (e) => e.heal],
    ["a knock-back", (e) => e.knocked],
    ["an escape", (e) => e.hits?.some((h) => h.withdrew)],
    ["a splash", (e) => e.hits?.length > 1],
    ["a charge", (e) => e.kind === "Charge"],
    ["a shot", (e) => e.kind === "Shoot"],
    ["a spell", (e) => e.kind === "Spell"],
    ["a bombard", (e) => e.kind === "Bombard"],
    ["a stab", (e) => e.kind === "Stab"],
    ["a defensive strike", (e) => e.kind === "DefStrike"],
  ])
    assert.ok(all.some(pred), `no fixture battle has ${what}`);
});

test("the opening: a defensive strike and a unit afraid", () => {
  const F = [
    { b: 0, unit: 1, name: "Axeman", side: 0, lane: 0, row: 0, max_hp: 120, hp_start: 12000, speed: 4, actions: [{ kind: "Strike", n: 2 }, { kind: "Strike", n: 1 }, { kind: "Strike", n: 1 }] },
    { b: 1, unit: 2, name: "Horseman", side: 0, lane: 1, row: 0, max_hp: 100, hp_start: 10000, speed: 6, actions: [{ kind: "Charge", n: 2 }, { kind: "Strike", n: 1 }, { kind: "Strike", n: 1 }] },
    { b: 2, unit: 3, name: "Longbowman", side: 1, lane: 0, row: 1, max_hp: 80, hp_start: 8000, speed: 6, actions: [{ kind: "Shoot", n: 1 }, { kind: "Shoot", n: 2 }, { kind: "Shoot", n: 2 }] },
    { b: 3, unit: 4, name: "Hill Giant", side: 1, lane: 0, row: 0, large: true, max_hp: 180, hp_start: 18000, speed: 4, actions: [{ kind: "Strike", n: 2 }, { kind: "Strike", n: 1 }, { kind: "Strike", n: 1 }] },
    { b: 4, unit: 5, name: "Archer", side: 1, lane: null, row: null, max_hp: 80, hp_start: 8000, speed: 6, actions: [] },
  ];
  const log = {
    viewer_side: 0,
    fighters: F,
    events: [
      { round: 0, pass: 0, kind: "Afraid", actor: 3, target: 1 },
      { round: 0, pass: 0, kind: "DefStrike", actor: 4, target: 0, hits: [{ target: 0, dmg: 1800 }] },
      { round: 1, pass: 1, actor: 2, kind: "Shoot", target: 0, hits: [{ target: 0, dmg: 2250, crit: true }] },
    ],
    outcome: { plot_taken: false, rounds: 1, defenders_left: 3, sides: [{ fallen: 0, escaped: 0, standing: 1 }, { fallen: 0, escaped: 0, standing: 2 }] },
  };
  const st = L.finalState(log);
  assert.equal(st.status[1], "afraid");
  assert.equal(st.status[4], "out");
  assert.equal(st.hp[0], 12000 - 1800 - 2250);
  const fs = L.fates(log, st);
  assert.deepEqual(fs[0].afraid, [1]);
  assert.deepEqual(fs[1].out, [4]);
  const lines = L.logLines(log);
  const text = (l) => l.parts.map((p) => p.text).join("");
  assert.equal(lines[0].text, "Before the battle");
  assert.equal(text(lines[1]), "Horseman is afraid of Hill Giant and sits out the battle.");
  assert.equal(text(lines[2]), "Archer strikes Axeman as it comes in 18");
  assert.equal(lines[3].text, "Round 1 · pass 1");
  assert.equal(text(lines[4]), "Longbowman shoots Axeman 23 critical.");
  // A strike that did no harm (the attacker too strong for it), as the engine sends it.
  const harmless = { ...log, events: [{ round: 0, pass: 0, kind: "DefStrike", actor: 4, target: 0, hits: [{ target: 0, dmg: 0, miss: true }], chance: 40, roll: 12, pct: 0 }] };
  assert.equal(text(L.logLines(harmless)[1]), "Archer strikes Axeman as it comes in, to no effect");
  assert.equal(L.finalState(harmless).hp[0], 12000);
  // Phase 7's opening spells (none yet): a cast's hits fold as an action's.
  const cast = { ...log, events: [{ round: 0, pass: 0, kind: "Cast", actor: 2, spell: "SPELL_MAELSTROM", targets: [0], effect: { kind: "Damage", hits: [[0, 1500]] }, hits: [{ target: 0, dmg: 1500 }] }] };
  assert.equal(text(L.logLines(cast)[1]), "Longbowman casts Maelstrom on Axeman 15.");
  assert.equal(L.finalState(cast).hp[0], 12000 - 1500);
});

test("damage and HP in words: centi-HP rounded, never a hit of 0 shown for one that hurt", () => {
  assert.equal(L.dmgText(2250), "23");
  assert.equal(L.dmgText(1), "1");
  assert.equal(L.hpOf(30), 1);
});

test("a formation: moves, swaps, a large unit's two cells, the slots", () => {
  const pv = FIXTURES.field.preview;
  const F = pv.fighters;
  const lim = { lanes: pv.lanes[0], cap: L.slotCap(pv.slots[0]) };
  const place = L.placeOf(F, 0);
  assert.equal(L.slotsUsed(place, F), 10, "the auto formation fills the slots");
  assert.deepEqual(L.placeFromList(F, L.formationOf(place, F)), place);
  assert.equal(L.formationKey(L.formationOf(place, F)), L.formationKey(L.formationOf(place, F).reverse()));
  const grid = L.gridOf(place, F, lim.lanes);
  // every cell of the grid holds at most one unit, a large one two side by side
  const elephant = F.find((f) => f.name === "War Elephant");
  const ep = place.get(elephant.b);
  assert.equal(grid[ep.row][ep.lane], elephant.b);
  assert.equal(grid[ep.row][ep.lane + 1], elephant.b);
  // a free cell: the unit moves there
  const free = [];
  for (let r = 0; r < 3; r++) for (let l = 0; l < lim.lanes; l++) if (grid[r][l] === null) free.push({ lane: l, row: r });
  assert.ok(free.length, "the field fixture's grid has a free cell");
  const mover = F.find((f) => f.side === 0 && place.has(f.b) && !f.large);
  const m = L.placeUnit(place, F, lim, mover.b, free[0].lane, free[0].row);
  assert.deepEqual(m.out, []);
  assert.deepEqual(m.place.get(mover.b), free[0]);
  assert.equal(m.place.size, place.size);
  // another unit's cell: they swap
  const other = F.find((f) => f.side === 0 && place.has(f.b) && !f.large && f.b !== mover.b && place.get(f.b).row !== place.get(mover.b).row);
  const s = L.placeUnit(place, F, lim, mover.b, place.get(other.b).lane, place.get(other.b).row);
  assert.deepEqual(s.place.get(mover.b), place.get(other.b));
  assert.equal(s.place.get(other.b).row, place.get(mover.b).row, "the displaced unit goes to the mover's row");
  // a large unit at the last lane takes the last two
  const big = L.placeUnit(place, F, lim, elephant.b, lim.lanes - 1, 2);
  assert.equal(big.place.get(elephant.b).lane, lim.lanes - 2);
  const g2 = L.gridOf(big.place, F, lim.lanes);
  assert.equal(g2[2][lim.lanes - 1], elephant.b);
  // a unit out of the line onto a full line's cell: the one there leaves the line
  const benched = F.find((f) => f.side === 0 && !place.has(f.b) && !f.large);
  const into = L.placeUnit(place, F, lim, benched.b, place.get(mover.b).lane, place.get(mover.b).row);
  assert.deepEqual(into.out, [mover.b]);
  assert.equal(L.slotsUsed(into.place, F), L.slotsUsed(place, F));
  // the slots: a large unit from out of the line on a free cell and an occupied one would take
  // one slot more than the full line has
  const giantOut = { ...elephant, b: F.length, lane: null, row: null };
  const F2 = [...F, giantOut];
  const cell = free.find((c) => [c.lane + 1, c.lane - 1].some((l) => l >= 0 && l < lim.lanes && grid[c.row][l] !== null));
  const anchor = cell.lane + 1 < lim.lanes && grid[cell.row][cell.lane + 1] !== null ? cell.lane : cell.lane - 1;
  const err = L.placeUnit(place, F2, lim, giantOut.b, anchor, cell.row);
  assert.match(err.error ?? "", /The line is full/);
  // a large unit on two small units' cells takes their two slots: they leave the line
  const pair = [0, 1, 2].flatMap((r) => [...Array(lim.lanes - 1).keys()].map((l) => ({ lane: l, row: r }))).find((c) => grid[c.row][c.lane] !== null && grid[c.row][c.lane + 1] !== null && grid[c.row][c.lane] !== grid[c.row][c.lane + 1] && ![grid[c.row][c.lane], grid[c.row][c.lane + 1]].some((b) => F[b].large));
  const two = L.placeUnit(place, F2, lim, giantOut.b, pair.lane, pair.row);
  assert.equal(two.out.length, 2);
  assert.equal(L.slotsUsed(two.place, F2), 10);
  assert.match(L.placeUnit(place, F, lim, mover.b, lim.lanes, 0).error ?? "", /not a cell/);
  // taking a unit out
  assert.equal(L.benchUnit(place, mover.b).has(mover.b), false);
});

test("the fixtures hold together: formations fit their grids, forecasts and logs match them", () => {
  for (const [name, fx] of Object.entries(FIXTURES)) {
    const pv = fx.preview;
    for (const side of [0, 1]) {
      const place = L.placeOf(pv.fighters, side);
      assert.ok(L.slotsUsed(place, pv.fighters) <= L.slotCap(pv.slots[side]), `${name}: side ${side} within its slots`);
      const g = L.gridOf(place, pv.fighters, pv.lanes[side]);
      const cells = g.flat().filter((b) => b !== null).length;
      assert.equal(cells, L.slotsUsed(place, pv.fighters), `${name}: side ${side}: no unit overlaps another`);
    }
    assert.ok(fx.forecasts[L.formationKey(pv.auto_formation)], `${name}: the auto formation's forecast`);
    assert.deepEqual(pv.formation, pv.auto_formation);
    for (const l of fx.logs) {
      assert.deepEqual(
        l.log.fighters.map((f) => f.unit),
        pv.fighters.map((f) => f.unit),
        `${name}: the log's fighters are the preview's`,
      );
      assert.equal(L.formationKey(L.formationOf(L.placeOf(l.log.fighters, 0), l.log.fighters)), L.formationKey(l.formation));
    }
  }
  assert.ok(Object.values(FIXTURES).some((fx) => fx.logs.some((l) => l.log.outcome.plot_taken)), "a fixture battle takes its plot");
  assert.equal(L.resultWords(FIXTURES.raid.logs[0].log).ribbon, "VICTORY");
  assert.match(L.resultWords(FIXTURES.city.logs[0].log).line, /^The defenders hold Strages: \d+ still stand/);
});

test("the result and the names are the viewer's: a battle seen by its defender", () => {
  const raid = { ...FIXTURES.raid.logs[0].log, viewer_side: 1 };
  assert.deepEqual([L.resultWords(raid).ribbon, L.resultWords(raid).good], ["DEFEAT", false]);
  const held = { ...field, viewer_side: 1 };
  assert.deepEqual([L.resultWords(held).ribbon, L.resultWords(held).good], ["HELD", true]);
  const giant = held.fighters.find((f) => f.name === "Hill Giant");
  assert.equal(L.uname(held, giant.b, { side: true }), "your Hill Giant");
  const line = L.logLines(held).find((l) => !l.head && l.parts.some((p) => p.b === giant.b));
  assert.equal(line.parts.find((p) => p.b === giant.b).cls, "you");
});

test("a preview's forecast is the formation's asked for, not the auto formation the engine answers a refusal in", () => {
  const pv = FIXTURES.field.preview;
  const auto = pv.auto_formation;
  // the first preview (nothing sent) and a reply for the formation sent bring their forecast
  assert.deepEqual(L.previewForecast(pv), { forecast: pv.forecast, refused: false });
  assert.equal(L.previewForecast(pv, [...auto].reverse()).forecast, pv.forecast, "the same formation in another order");
  // a refused formation, answered in the auto formation: no forecast of it
  const moved = auto.map((p, i) => (i === 0 ? { ...p, row: (p.row + 1) % 3 } : p));
  assert.deepEqual(L.previewForecast({ ...pv, errors: ["Not across the river."] }, moved), { forecast: null, refused: true });
  assert.deepEqual(L.previewForecast({ ...pv, formation: undefined }, moved), { forecast: null, refused: true });
  // a reply with no forecast is answered: nothing is pending
  assert.deepEqual(L.previewForecast({ ...pv, formation: moved, forecast: null }, moved), { forecast: null, refused: false });
});

test("the result when no attacker is left: worded from the fates (fallen, escaped, afraid)", () => {
  const fighter = (b, side, name, lane) => ({ b, unit: 900 + b, unit_type: "UNIT_AXEMAN", name, owner: side, side, role: "COMBATROLE_FLANKER", large: false, lane, row: 0, max_hp: 100, hp_start: 10000, speed: 6, actions: [] });
  const fighters = [fighter(0, 0, "Horseman", 0), fighter(1, 0, "Horseman", 1), fighter(2, 0, "Axeman", 2), fighter(3, 1, "Axeman", 0)];
  const kill = (t) => ({ round: 1, pass: 1, actor: 3, kind: "Strike", target: t, hits: [{ target: t, dmg: 10000, killed: true }] });
  const flee = (t) => ({ round: 1, pass: 2, actor: 3, kind: "Strike", target: t, hits: [{ target: t, dmg: 0, withdrew: true }] });
  const afraid = (t) => ({ round: 0, pass: 0, kind: "Afraid", actor: null, target: t });
  const log = (events) => ({ viewer_side: 0, plot: { x: 1, y: 1, city: null }, fighters, events, outcome: { plot_taken: false, rounds: 1, defenders_left: 1 } });
  // the last attacker withdraws: escaped, not wiped out
  const w = L.resultWords(log([kill(0), kill(2), flee(1)]));
  assert.equal(w.kind, "repulsed");
  assert.equal(w.line, "No attacker is left on the field (2 fell, 1 escaped). The defenders hold.");
  assert.doesNotMatch(w.line, /wiped out/);
  const fs = L.fates(log([kill(0), kill(2), flee(1)]), L.finalState(log([kill(0), kill(2), flee(1)])));
  assert.deepEqual([fs[0].fallen.length, fs[0].escaped.length], [2, 1]);
  // everyone fell: wiped out
  assert.equal(L.resultWords(log([kill(0), kill(1), kill(2)])).line, "The attackers are wiped out. The defenders hold.");
  // one sat out afraid, the others fled
  assert.equal(L.resultWords(log([afraid(2), flee(0), flee(1)])).line, "No attacker is left on the field (2 escaped, 1 sat out afraid). The defenders hold.");
  // seen by the defender: the same words, its ribbon
  const d = L.resultWords({ ...log([kill(0), kill(2), flee(1)]), viewer_side: 1 });
  assert.deepEqual([d.ribbon, d.good, d.line], ["HELD", true, w.line]);
});
