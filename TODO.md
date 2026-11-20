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
- Open points or questions for the owner: docs/ARMIES.md, docs/CULTURE.md, docs/WORKS.md,
  docs/GOVERNORS.md, docs/EARLY_UNITS.md, docs/UNIT_ROLES.md, docs/STATS_CONTENT.md,
  docs/ART.md 16.4, docs/DECKS.md 8.4
- Other: "Next steps" in docs/PORTING.md, "Not in this change" in docs/SUPPLY.md, the "Not built"
  notes in docs/NAVAL.md

## Calculated numbers: the other screens

The owner's rule (2026-11-20; docs/PLAY_DESIGN.md 4.9): a number the rules calculate shows its
calculation in a tooltip. Built for culture and courting. A survey of the client (2026-11-20)
found these without one, most looked at first (file:function in `crates/ffh-play/assets/app/`):

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
- **Armies, governors, Houses, crusades, naval, rituals, battles**: an army's supply cost against
  its budget (armies.js); fleet wear; march, training and requisition turns; a role's suitability
  (governors.js) and the empire-order picker's turns and cost; tribute at each rate (persons.js
  `tributeBlock`: the House's commerce × rate); a crusade's obligation, lendable strength, credit
  and spoils (crusade.js); a sea's side strength and port range (seas.js, sea.js); a ritual's cost
  and next cost (ritual_rows.js); a battle's ATK and DEF and damage per hit (battle_plan.js,
  battle_log.js).

Most need the parts added to their payloads (`src/view/*.rs`) from a `*_calc` read in `ffh-core`
next to the formula; the city screen's production can be done from what the payload already sends.

## Trace the fourth build's research fall

The fourth build's guardrail (docs/AI_LOG.md, "Courting is a cultural channel") found the world's
research at turn 300 down 14% (lower in 9 of 12 seeds) and the world's military and unit upkeep up
about a fifth, with fewer wars declared, fewer units built and fewer battles. What turns the extra
upkeep into the research fall is not found. Candidates: units kept alive by the hold-march
(`courted_to_hold`, which never fired among AIs before), the lieges' grudge memories
(`MEMORY_COURTED_HOUSE`, `MEMORY_SPLIT_HOUSE`) moving the sliders, fewer spreads of a state
religion. Start from games (CLAUDE.md, "How AI changes are checked", step 1): per player, where
the upkeep and the research went between turns 200 and 300, base against new.

## Houses: what is not built

Houses as vassal civilizations (tribute, the levy, breaking away) are built (docs/PERSONS.md, "The
second build"); the third build's part 1 (2026-11-19) removed Persons of Note (the cast,
commanders, veterans and great persons as founders) and made a realm's first grant found a House;
its part 2 built splits, ultimatums and contracts, shared knowledge and gold tribute. The fourth
build (2026-11-20) made courting a cultural channel (Court, Guard, Spread Religion: a successful
courtship splits the House toward the courter) and retired the side deals, the price and revolt.
Still to build, each described there:

- Claims abroad (stage 3).
- The annals (stage 5): what each House did in the player's earlier games.
- Ultimatums among AIs: in AI games they do not fire (docs/AI_LOG.md, "Houses without Persons of
  Note": the AI keeps its Houses Pleased), so the AI's answer to an ultimatum is exercised only
  against a human. A probe that squeezes its Houses (tribute at the most, requests refused) would
  show it. Courting by culture does fire among AIs (docs/AI_LOG.md, "Courting is a cultural
  channel": 314 Court channels and 44 courtships at their requirement over 12 games).
- To be judged in play (docs/PERSONS.md, "Open points for the owner"): requests with a term, an
  Annoyed House's withheld levy, claims, the Crown's limit of 3, the tribute's loyalty cost.
