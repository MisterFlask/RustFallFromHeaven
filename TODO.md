# TODO

The owner's queue of planned work: design work that is agreed in outline and not yet built. One
entry per item: what it is, the decisions already made, what is still open, and the area docs that
hold the detail. Open work on a built area lives in that area's doc (CLAUDE.md). Remove an entry
when its work is merged (the area doc then describes it as built).

## Vet the area docs' open lists

The area docs hold about 20 sections of open work, many probably stale. Check each entry against
the code and mark it done, still open, or dropped; delete the stale ones; move what the owner
wants done next into this file.

- Known gaps: docs/AI.md, docs/HUMAN.md, docs/CRUSADE.md
- Open: docs/ENGAGEMENTS.md, docs/SIEGES.md, docs/TACTICS.md, docs/PERF.md (two sections)
- Open points or questions for the owner: docs/ARMIES.md, docs/WORKS.md, docs/GOVERNORS.md,
  docs/EARLY_UNITS.md, docs/UNIT_ROLES.md, docs/STATS_CONTENT.md, docs/ART.md 16.4,
  docs/DECKS.md 8.4
- Other: "Next steps" in docs/PORTING.md, "Not in this change" in docs/SUPPLY.md, the "Not built"
  notes in docs/NAVAL.md

## Raids: the second build

The first build (docs/RAIDS.md, 2026-11-21) is the Raid order, the guard's sallies, the take and
the strip timer. The owner's discussion of the same day agreed these in outline; each is its own
build (docs/RAIDS.md, section 11):

- **A raid without a war of conquest.** A war declared for a stated goal (a raid, a demand); in
  a raid's war no city changes hands, and it ends with a truce when the raiders leave or are
  paid. The defender can end a limited war by conceding its goal. Open: what a war of conquest
  costs beyond it (weariness, relations), and what the victim's escalation costs.
- **Paying raiders off.** A refused demand as the war's goal; a raided AI pays when the raids
  cost more than the tribute (the demands of docs/PORTING.md, "Gifts and demands").
- **A militia building.** The owner's idea: a building that strengthens the defence of the land
  a city works. As discussed: more damage for the guard in a sally, or a militia unit in its
  squad (the Cairn card's Sentinel is the precedent), its own cheap early building, seen on the
  map from outside. Open: name, cost, technology, numbers.
- **A resource's term.** Raiding a resource plot takes the resource for some turns (the raider
  has it, the owner does not): the one plot outside a city's work that matters.
- **Barbarians.** A barbarian party on a city's land met by the guard under the raid's rule.
- **The AI goes to war to raid**, and relieves a raided city on purpose. Today it raids in the
  wars it fights anyway (docs/RAIDS.md, section 9).
- **Measure a contest's length.** A contested raid should last two to four turns; count it in
  played games before the numbers are tuned.

## Calculated numbers: the other screens

The owner's rule (2026-11-20; docs/PLAY_DESIGN.md 4.9): a number the rules calculate shows its
calculation in a tooltip. Built for the Armageddon counter's change a turn (hud.js), a hero's
Wounds (units.js) and a battle's verdict (battle_report.js, battle_watch.js). A survey of the
client (2026-11-20) found these without one, most looked at first (file:function in
`crates/ffh-play/assets/app/`):

- **Top bar** (hud.js `renderTop`): the turns to finish research (no research rate shown); the
  score % (the tip names "average of your shares" without the three shares; no tip at all in
  diplo.js and the game-over screen); the Armageddon counter (the tip lists the deck, not what
  raised it).
- **City screen** (city.js): production a turn (`summaryRow`, `productionView`: the payload's
  `base` and `modifier` are sent and unused); build and production turns; turns to grow; the food
  grown; gold, research and espionage (`commerceView`: parts inline, the modifier never combined);
  upkeep and defence (`growthView`); great people progress and rate; hurry cost; build cost (the
  tier and national percentages only as words); plot yields; work crews; outpost cost. The iso
  city panel's rows (isoui.js `renderCityPanel`) and the plates' production turns (map.js
  `drawCities`, iso.js `drawCityLabels`).
- **Units and combat** (units.js): a plot's yields, defence and move cost; path turns and supply
  cost; the retreat chance and how the odds come from the two strengths (`combatLines` explains
  the modifiers only); XP needed; upgrade price; action and build turns.
- **Research** (empire.js): research a turn; turns per tech; tech cost (the multipliers are
  explained, not the cost); doctrine research left; finances' research.
- **Civics and religion** (empire.js): each civic's upkeep, the total, anarchy turns.
- **Diplomacy and trade** (diplo.js): the AI's value of techs, peace and war (`valueTip`, words
  only); total offered and requested.
- **Armies, governors, crusades, naval, rituals, battles**: an army's supply cost against its
  budget (armies.js); fleet wear; march, training and requisition turns; a role's suitability
  (governors.js) and the empire-order picker's turns and cost; a crusade's obligation, lendable
  strength, credit and spoils (crusade.js); a sea's side strength and port range (seas.js, sea.js); a ritual's cost
  and next cost (ritual_rows.js); a battle's ATK and DEF and damage per hit (battle_plan.js,
  battle_log.js).

Most need the parts added to their payloads (`src/view/*.rs`) from a `*_calc` read in `ffh-core`
next to the formula; the city screen's production can be done from what the payload already sends.
