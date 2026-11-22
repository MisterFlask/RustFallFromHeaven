# Houses

This game's own rules, not FFH2's. A civilization's cities are grouped into **Houses**: up to four
cities under a named leader with an alignment, a religion, a sympathy for another civilization and
a perk. The cities stay their owner's in every respect. The leader's opinion of the ruler is
happiness in the House's cities, and a Furious House's perk is off.

The owner's design of 2026-11-21. It replaces Houses as vassal civilizations (the builds of
2026-11-18 to 2026-11-20: grants, the Crown's limit, tribute, the levy, splits, requests,
ultimatums, break-aways, courting) and cultural channels (Court, Guard, Spread Religion), all
removed; `git log -- docs/PERSONS.md docs/CULTURE.md` has their history, and docs/AI_LOG.md what
their games showed. **Built on 2026-11-21 as written; "As built" below says where the build
completed or read a rule.** The numbers are first guesses.

Why (the owner's reasons):
- *Vassalage and governors were functionally mandatory.* Every government but two forced governors,
  and the Crown's limit of three cities forced grants. Now a House takes nothing from its owner, and
  only two governments mandate governors (rule 13).
- *Culture is allegiance*, a slow local stock, not a purse to spend at range. Its work here: where a
  rival's culture stands when a House forms, the House's leader favours that rival (rule 6), so
  attacking a cultured neighbour costs happiness at home. The engine's own culture rules stand
  beside it (borders, a city's unhappiness from foreign culture, revolt and flip).
- *Houses as players cost more than they gave*: more players, one-city Houses eliminated, and among
  AIs no ultimatum or break-away.
- *Perks* (docs/ROLLED_PERKS.md) arrive with Houses: a pick of three at each formation, the
  leader's alignment, religion and sympathy being each offer's price.

Code: rules `crates/ffh-core/src/houses/`; data `data/json/traits.jsonc` (the perks), the minor
leaders (`docs/minor-leaders/`), `data/json/rules_defines.jsonc` (`HOUSE_*`); AI
`crates/ffh-ai/src/houses.rs`; UI `crates/ffh-play/src/view/houses.rs`, `assets/app/houses*.js`.
Tests: `crates/ffh-core/tests/houses.rs`, `crates/ffh-play/tests/houses*.rs`.

Every game has Houses (no game option).

## Rules

1. **Who has Houses** (`player_has_houses`): every living civilization, human or AI, a vassal
   included. Not the barbarians, a minor civilization or a crusade Host. An outpost is in no House
   and counts for nothing below; a settlement is a city.

2. **A House** is a record of its owner: `House { id, owner, name, leader, origin, alignment,
   religion, sympathy, perk, outsider, formed_turn, cities }`, in `Game::houses` (a slab); a city
   has `City::house: Option<HouseId>`, a player `Player::houses`. Its *seat* is its first city in
   `cities` (join order). A House is not a player: its cities are its owner's, built, taxed,
   garrisoned and governed as any other. The leader is a minor leader (`LeaderHeadInfo` with
   `minor_leader_of`): portrait, name, alignment. The House's name is `House <name>`.

3. **A city's origin** is the civilization (`CivilizationType`) of its original owner
   (`City::original_owner`). Where that is the barbarians, or the current owner's own civilization,
   the city is a *home* city; otherwise a *foreign* one, of that civilization. A House's origin is
   that of the city it formed with, and it takes only cities of its origin.

4. **Membership.**
   - *Joining*, at once when a player gains a city (founded, captured, traded, flipped, born): the
     city joins the House of its owner and of its origin that has fewer than `HOUSE_MAX_CITIES` (4)
     cities and whose seat is nearest it, within `HOUSE_JOIN_RANGE` (8) plots of the seat (plot
     distance; a tie to the lower House id).
   - *Forming*, in its owner's turn processing, for each unhoused city in the owner's city order
     (first trying to join, as above):
     - a home city of an owner with no home House waits until the owner holds
       `HOUSE_FIRST_CITIES` (3) unhoused home cities. Then the first House forms: its seat the
       capital (if it is an unhoused home city, else the first unhoused home city), with the
       unhoused home cities within range of the seat, nearest first (a tie to the lower city id),
       up to four;
     - any other unhoused city forms a new House at once, as its seat.
   - *Leaving*: a city that changes owner or is destroyed leaves its House. When the seat leaves,
     the next city in join order is the seat. A House with no city ends; its leader is free again.
     Nothing else moves a city between Houses.

5. **The pick.** A House forms with a leader its owner picks among candidates, through its
   controller: a blocking decision for a human, the AI's choice (rule 14) otherwise.
   - *Candidates*: `HOUSE_CANDIDATES` (3) minor leaders drawn with the game RNG, without
     replacement, from the *free* minor leaders of the House's origin civilization. A minor leader
     is free when it leads no living player and no House in the game. Where fewer are free, the
     rest are drawn from the free minor leaders of other civilizations whose alignment is within
     one step of a major leader of the origin civilization. With no candidate at all the House does
     not form this turn.
   - *Each candidate shows* its alignment (rule 6), religion (rule 6), sympathy (rule 6) and perk
     (rule 9), and what its opinion and happiness would be now (rules 7, 8).
   - *The outsider*: a foreign House's pick always has one more candidate, from the owner's own
     civilization: the free minor leader of it whose alignment is nearest the owner's now (a tie by
     the game RNG). `outsider: true`. Its religion is the owner's state religion where that
     religion admits its alignment, else none. It has no sympathy, ever. Its cities have
     `HOUSE_OUTSIDER_HAPPY` (-3) happiness, for as long as it leads. With no free minor leader of
     the owner's civilization there is no outsider.
   - The leaders not picked stay free. A leader is for good: there is no replacing one yet.

6. **What a leader is rolled with**, when it becomes a candidate (the game RNG, in candidate
   order: religion, sympathy, perk).
   - *Alignment*: the leader's own, from the data, fixed. (A minor leader's alignment is within one
     step of a major leader of its civilization: `crates/ffh-data/tests/minor_leaders.rs`.)
   - *Religion*: one religion or none. A religion is admitted when the leader's alignment lies
     between its `alignment_worst` and `alignment_best`. Weights: each admitted religion 10, plus
     30 where one of the forming cities has it; none 20. A religion not yet founded can be drawn.
   - *Sympathy*: one civilization or none. The candidates are the living players the owner has
     met, of another team, that could have Houses (rule 1) and are not two alignment steps from
     the leader. With `C(x)` the plot culture of player `x` summed over the plots in the work
     radius of the forming cities, and `T` its total over all players: a candidate `X` weighs
     `5 + 100·C(X)/T`, none weighs `20 + 100·C(owner)/T` (`T` zero: 5 each, none 20). So a rival's
     culture standing there draws the leader to it, and the owner's own culture there draws it to
     no one.
   - *Perk*: rule 9.

7. **Opinion.** A House's opinion of its ruler is a sum of parts, read whenever it is needed
   (nothing is stored, nothing lags):
   - (a) *Alignment*, against the owner's alignment now: the same `+1`; else `-2` a step.
   - (b) *Religion*, where the House has one and the owner a state religion: the same `+2`,
     another `-1`.
   - (c) *Sympathy*, toward a living civilization `X`: at war with it `-3`; else a defensive pact
     or a permanent alliance with it, or it is the owner's vassal or master, `+2`; else open
     borders with it `+1`.
   - (d) *Wars and alignment*: for each civilization the owner is at war with (living, with a
     city, one that could have Houses, not `X` of (c)): `-1` where it has the House's alignment,
     `+1` where it is two steps from it; the sum kept within -2 and +2.
   - An outsider has parts (a) and (b) only.

   States: **Friendly** 3 or more, **Pleased** 1 to 2, **Cautious** -2 to 0, **Annoyed** -5 to
   -3, **Furious** -6 or less.

8. **What opinion does.** Every city of the House has happiness by its state: Friendly `+2`,
   Pleased `+1`, Cautious `0`, Annoyed `-2`, Furious `-4` (`HOUSE_HAPPY_*`), and an outsider's
   cities `-3` besides. A Furious House's perk is off (rule 9). Nothing else: a House never
   leaves, revolts, pays or withholds.

9. **Perks.** A perk is a trait (`TraitInfo` with `house_perk_tier` 1 to 3, never a leader's own
   trait and never selectable) shown as the House leader's trait. It applies in the House's
   cities while it is on (`House::perk_on`). Happiness follows the opinion at once; the perk
   switches in the House's owner's turn processing, in the pass that tells a change of state
   (rule 11): off when the House is Furious then, on again when it is not. (What a perk changes
   is kept in the cities' stored rates, so it switches at one known moment.)
   - *Tier*, by the turn the House forms: tier 1 before turn 100, tier 2 from turn 100, tier 3
     from turn 200 (`HOUSE_PERK_TIER_TURNS`, at Normal speed; scaled by the game speed as
     `crusade::speed_turns` scales). A House keeps its perk.
   - *The roll*: each candidate of a pick has a different perk family, drawn equally among the
     families of the tier that the House can use (`house_requires_coastal`: one of the forming
     cities is coastal).
   - *Effects* (keys of `TraitInfo`, read only for a perk, all at the scope of one city):
     `house_yield_modifier` (percent, by yield), `house_commerce_modifier` (percent, by commerce),
     `house_great_people_rate_modifier`, `house_distance_maintenance_modifier` and
     `house_num_cities_maintenance_modifier` (percent of that city's part),
     `house_free_experience` (every unit trained there that takes experience),
     `house_unit_combat_free_experience`, `house_unit_combat_free_promotions` (given to a unit of
     that combat when it is trained there, as a building's free promotions are; a unit keeps them
     when the perk goes off or it leaves), `house_unit_combat_production_modifier`,
     `house_wonder_production_modifier` (world and national wonders),
     `house_ritual_production_modifier` (projects).
   - *The families* (`TRAIT_PERK_<FAMILY>_<tier>`; tiers 1 / 2 / 3):

     | Family | Effect in the House's cities |
     |---|---|
     | Horsemasters | Mounted units trained: Flanking I / Flanking I, Mobility I / Flanking I, Flanking II, Mobility I; tier 3 also +25% production of Mounted units |
     | Shieldwall | Melee units trained: Combat I / Combat I, Shock / Combat I, Combat II, Shock; tier 3 also +25% production of Melee units |
     | Marksmen | Archer units trained: Drill I / Drill I, Drill II / Drill I, Drill II, City Garrison I; tier 3 also +25% production of Archer units |
     | Battle-mages | Adept units trained: Potency / Potency, Combat I / Potency, Combat I, Twincast |
     | Drillmasters | units trained: +2 / +4 / +6 experience |
     | Shipwrights (coastal) | Naval units: +2 / +4 / +6 experience and +25% / +50% / +75% production |
     | Freeholders | distance maintenance -50% / -100% / -100%, and at tier 3 city-count maintenance -50% |
     | Patrons | culture +50% / +100% / +150% |
     | Scholars | research +15% / +25% / +40% |
     | Merchants | gold +15% / +25% / +40% |
     | Smiths | production +10% / +15% / +25% |
     | Master Builders | wonder production +25% / +50% / +100% |
     | Ritualists | ritual production +25% / +50% / +100% |
     | Luminaries | great people rate +50% / +100% / +150% |

     A promotion a unit combat cannot hold is replaced in the data by the nearest one it can
     (reported in "As built"). Not in this build, from docs/ROLLED_PERKS.md's list: mana nodes
     giving double mana, the dead rising, dual-cast, a unit class's affinity, crusade units (each
     needs a rule of its own), and the two that no region can carry (the Ashen Veil conversion,
     resources in diplomacy).

10. **Vassals** are plain vassals, as before Houses: no tribute, no levy, no record. A vassal has
    Houses of its own like any civilization.

11. **Told** (bare, to the owner; a human gets an alert):
    `House Aldric forms: Tovin Cinderwright leads Ventry, Corona and Astrum.`
    `Astrum joins House Aldric.` `House Aldric ends: it holds no city.`
    `House Aldric is now Furious: its perk is off.` (and each other change of state:
    `House Aldric is now Annoyed.`)

12. **Reads for the screens** (`houses/read.rs`; every number with its `*_calc` lines,
    docs/PLAY_DESIGN.md 4.9): a House's parts and state (`house_opinion_parts`), its happiness by
    city, its perk and whether it is on; a candidate's card; and **previews**: a House's state
    before and after the owner declares war on a civilization, makes peace with one, signs or ends
    open borders or a pact, or changes its state religion (`House Aldric: Pleased to Annoyed.
    Happiness -3 in 4 cities.`; `Its perk goes off.`).

13. **Governors** (docs/GOVERNORS.md). The civics' key `direct_rule` is replaced by `governed`:
    City States and Aristocracy have it. Under a governed civic every city is governed, as before.
    Under any other, a city is run by hand unless its owner hands it to a governor, city by city,
    and takes it back at will (no cost, no delay); a city gained is run by hand. Adopting a
    governed civic hands every city over; leaving one changes no city (each stays governed until
    taken back). The size costs of Despotism and God King stay as they are.

14. **The AI** (a rules change: reasonable, not measured; CLAUDE.md, "How AI changes are checked").
    - *The pick*: the candidate with the highest `P + 30 × h`, where `h` is the happiness its
      state now would give (the outsider's -3 included) and `P` is 60, or 0 where it would be
      Furious now; a tie to the first.
    - *War*: choosing whom to attack, a target loses value for each of the AI's Houses the war
      would make Furious, and less for each it would make Annoyed (a small term beside the
      alignment flavor in `diplomacy.rs`).
    - *Governors*: it hands every city to a governor unless its government is Despotism or God
      King, which is what the rules made it do before; whether it should govern from the first
      turn is not measured.

15. **Screens.**
    - *The pick*: a dialog of the candidates side by side, each a card: portrait, name, epithet,
      `Alignment: Neutral`, `Religion: Runes of Kilmorph`, `Sympathy: the Balseraphs`, the perk's
      name and its rules text, `Opinion now: Annoyed (-3)` with its parts, `Happiness: -2 in 3
      cities.`; the outsider's card says `Outsider: -3 happiness in this House's cities. No
      sympathy.` The dialog's head names the cities and, for the sympathies, the culture there
      (`Culture here: the Balseraphs 62%, yours 30%, the Lanun 8%.`).
    - *The Houses screen* (the menu entry "Houses"): each House a card (leader, the same lines,
      its cities, the perk and whether it is on); the cities in no House and why (`No House yet:
      the first forms at 3 cities (2 of 3).`).
    - *The city screen*: a Governance line (`House Aldric (Tovin Cinderwright): Pleased, +1
      happiness. Perk: Horsemasters.`), the happiness list's lines (`House Aldric is Annoyed: -2`,
      `Ruled by an outsider: -3`), and the perk's part in every calculation it enters.
    - *The map*: a House's shield on its cities' plates, for the owner.
    - *Diplomacy*: a civilization's page lists its Houses (leader, alignment, religion, sympathy,
      perk, state), for whoever has met it; the war, peace, open borders and pact questions and
      the religion screen show rule 12's previews.
    - *Governors*: the city screen's toggle (`Run by hand` / `Hand to a governor`) under a civic
      that does not govern, and the civics screen's line for one that does (`Every city is run by
      a governor.`).
    - Keywords `House`, `Sympathy`, `Outsider` (`data/json/keywords.jsonc`), their reminders
      beside the tooltips; rules text bare (CLAUDE.md, "Rules text and tooltips").
    - A staged scene for tests and screenshots: `ffh-play stage houses DIR`.

## As built (2026-11-21)

Rules `crates/ffh-core/src/houses/` (`mod.rs` the record, `membership.rs`, `pick.rs`,
`opinion.rs`, `perks.rs`, `read.rs`); every weight, threshold and range above is a `HOUSE_*`
define. The harness checks the invariants in every stress game (`crates/ffh-harness/src/check.rs`):
a House's cities are its owner's, of its origin and at most four, no minor leader leads two Houses
or a House and a living player, no city is left unhoused after its owner's pass but those rule 4
makes wait, and a perk is off exactly when its House was Furious at its owner's last pass.

- **Names.** A House's name is drawn from its origin civilization's name list
  (`governor_names.jsonc`), then the generic list, never twice for one owner in a game.
- **A leader is in play.** A House's leader is not taken by a breakaway faction or a colony
  (`sundering::leader_in_play`): a stress game had a revolt take one.
- **The rolls' order** (rule 6): the regular candidates are drawn, then the outsider (with its
  tie-break), then for each candidate in order its religion, sympathy and perk; the outsider rolls
  only its perk. A weighted draw lists none first, then religions or players in id order. The
  culture total is summed over the union of the forming cities' radius plots.
- **Opinion** (rule 7). A permanent alliance puts both players in one team, so a favoured teammate
  counts as the pact case (+2). In a preview, a war declared on a team covers its vassals and ends
  open borders and the pact with them; pacts drawing third parties in are not modelled. A
  conversion's preview moves the owner's alignment into the new religion's range, as converting
  does. A pact is a defensive pact.
- **Told** (rule 11). A new House is not told its first state. A House leaving Furious is told its
  new state only; the previews add `Its perk is on again.`
- **Perks** (rule 9). A percent adds to the other percents of its value (a city's yield, commerce
  and great-people modifiers; the factor of its distance and city-count maintenance, so -100 makes
  the part 0 whatever the civics add); experience and production modifiers enter where a
  building's do; the free promotions are given after a building's, where the promotion admits the
  unit's combat. A wonder is a world or national wonder (not a team one); a ritual is any project.
  A House forms with its perk on unless it is Furious then. Twincast (Battle-mages III) normally
  needs Combat V and Hero: it is the strongest line in the table.
- **Not housed.** A city gained since its owner's last pass, the home cities waiting for the first
  House, and a city of an origin with no free leader.
- **The AI** (rule 14): the pick's score is the controller's default (`house_pick_default`), so
  every controller without its own answer picks by it, and it is the human's advice; the war term
  is `house_war_cost` in `crates/ffh-ai/src/diplomacy.rs` (9 a city for a House turned Furious, 3
  for one turned Annoyed, beside the alignment flavor's 30). The first games and the guardrail:
  docs/AI_LOG.md.
- **Governors** (rule 13): `player_set_city_governed`, the command `govern_city`; the controller is
  asked when civics change, when a city is gained and each turn (`choose_city_governance`,
  `govern_gained_city`), which keeps the AI's cities governed exactly when the rules used to
  govern them.
- **Screens** (rule 15; docs/HUMAN.md has the queries): the pick is the `house_leader` decision
  with the offer as JSON; the Houses screen is Shift+F9; a House's shield is at the right end of
  its cities' plates, one of 8 colours by House id, with a mark when Annoyed or Furious; previews
  stand in the war question, the trade screen (a deal holding peace, open borders or a pact), the
  cancel-deal question and the religion screen; `ffh-play stage houses|houses-foreign DIR`,
  `tools/web-houses-smoke.mjs`, `tools/houses-shots.mjs`. The city screen shows its culture a turn
  again (the Culture section went with the channels).
- **Left in the data**: the memory types `MEMORY_SPLIT_HOUSE` and `MEMORY_COURTED_HOUSE` are listed
  by the leaders' data and nothing records or reads them.

## Open points for the owner

- **A leader's death or deposition** (planned by the owner, later): until then a leader, its
  sympathy and its perk are for good.
- **Aristocracy** shares City States' mandatory governors for now; its own identity is open.
- **The first perk** comes with the first House, at three cities, and covers the whole realm then;
  tier 1 is sized for that.
- **The perks left out** (rule 9), and whether a family should be authored per leader instead of
  rolled.
- **Opinion does not lag**: a war declared and ended in a turn costs a turn of unhappiness. A
  fading part after a war on a favoured civilization would make it stick.
- **Whether the AI should weigh a House's opinion** in peace and in its state religion, as it does
  in choosing a war.
- **The size costs of Despotism and God King** were raised (2026-11-10) to price ruling by hand;
  ruling by hand is now the default of four governments.
