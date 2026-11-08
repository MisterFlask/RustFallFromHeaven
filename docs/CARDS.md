# The decks as built

The great-person deck of secret techs and the Armageddon deck: how they work in the code, and
how a card is added. The design, the owner's verdicts and every card's spec are in
`docs/DECKS.md` and `docs/DECKS_ROUND8.md`; this file is the implementation.

## 1. The great-person deck

**A card is a tech** with a `card_suit` (`TechInfo::card`, `crates/ffh-data/src/infos/civics.rs`).
A card tech is never researched or traded: the reader forces `disable` and `!trade`.

- **Suit:** `card_suit` is the unit class of the great person that draws it:

  | Suit | Class |
  |---|---|
  | Sage | `UNITCLASS_SCIENTIST` |
  | Prophet | `UNITCLASS_PROPHET` |
  | Engineer | `UNITCLASS_ENGINEER` |
  | Merchant | `UNITCLASS_MERCHANT` |
  | Bard | `UNITCLASS_ARTIST` |

- **Band:** the tech's `era` (I `ERA_ANCIENT`, II `ERA_CLASSICAL`, III `ERA_MEDIEVAL`).
- **Needs:** the tech's `and_pre_reqs` / `or_pre_reqs`, the ordinary techs its content needs. A
  card is discovered only once its owner knows them.
- **Alignment:** `card_alignment` (`ALIGNMENT_GOOD`, `ALIGNMENT_EVIL`) limits who draws it.
- **Faith:** `card_religion` names a religion card. It is drawn whatever the drawer's
  religion; discovering it spreads the religion to three of the owner's cities without it, the
  most populous first.

**Content gates on the card tech.** A card's units, buildings, promotions, spells, civics,
improvements and projects name the card tech as their tech prerequisite (`prereq_tech`,
`tech_prereq`, ...). Rules code tests `team_is_has_tech(team, Some(card))`. Since the card
cannot be discovered before its needs, nothing else needs to require them.

**The deal** (`crates/ffh-core/src/deck/mod.rs`):
- A great person of a suit is dealt a card at birth (`player_init_unit` ->
  `deck_deal_to_unit`). It is held in `Unit::card`.
- The deck has no state of its own. A card is in it while no team knows the card and no living
  unit holds it. A card whose great person dies, or is used another way, is back in the deck.
- **Draw:** from the cards of the suit still in the deck, of the player's alignment, in the
  band of the player's era (Medieval and later count as III). If that band is empty, the nearest
  band below is used, else the nearest above. One `soren_rand_num` over the candidates in id
  order.

**Discover:** `SPELL_DISCOVER_CARD` (`data/json/cards/000_deck.jsonc`). Its requirement is
`unit_can_discover_card`; its result is `unit_discover_card` (the team learns the card, the
religion spreads); `sacrifice_caster` uses the great person up.

**AI** (`crates/ffh-ai/src/cards.rs`):
- A great person discovers its card as soon as it can.
- The card's missing needs pull research (`research::goal_bonus`, `CARD_SCORE`).
- A great person under 30 turns old whose needs are at most two near techs away waits in the
  capital. Otherwise it is used the old way, and its card goes back into the deck.
- Each card's own content needs its own AI use. That goes in `ffh-ai`, next to the system it
  belongs to (production, casting, research). A card batch's own AI (when to cast its world
  spells and at what, which rituals to build) goes in a module of its own, `ffh-ai/src/cards_<batch>.rs`,
  with its turn function added to `cards::TURNS` (run once a turn, after the empire decisions).
  A rules change is not gated on the AI (CLAUDE.md, "AI changes").

## 2. A card's files

**Data:** one file per card, `data/json/cards/<suit>_<name>.jsonc` (`sage_abjurer.jsonc`).
The loader (`add_card_files`, `crates/ffh-data/src/lib.rs`) appends each member's entries to its
info table, after the table's own entries, files in name order:
- each member is an info table, named as its file is without `.jsonc` (`techs`, `units`,
  `unit_classes`, `promotions`, `buildings`, `building_classes`, `spells`, `improvements`,
  `builds`, `civics`, `projects`, `civilizations`, `leader_heads`, ...);
- `text` is an object of the card's English text (TXT_KEY -> string).

A card's file holds its tech and everything the card adds. The tech is
`TECH_CARD_<NAME>`, its name `TXT_KEY_TECH_CARD_<NAME>`, and its `help` is the card's effect in
bare rules text (CLAUDE.md, "Rules text and tooltips").

```jsonc
// G05 Abjurer (docs/DECKS_ROUND8.md)
{
  "techs": [
    {
      "type": "TECH_CARD_ABJURER",
      "description": "TXT_KEY_TECH_CARD_ABJURER",
      "help": "TXT_KEY_TECH_CARD_ABJURER_HELP",
      "civilopedia": "TXT_KEY_TECH_CARD_ABJURER_PEDIA",
      "advisor": "ADVISOR_SCIENCE",
      "cost": -1,
      "advanced_start_cost": -1,
      "era": "ERA_CLASSICAL",
      "and_pre_reqs": ["TECH_ALTERATION"],
      "card_suit": "UNITCLASS_SCIENTIST",
      "flavors": {"FLAVOR_SCIENCE": 1}
    }
  ],
  "promotions": [ /* PROMOTION_ABJURER, "tech_prereq": "TECH_CARD_ABJURER", ... */ ],
  "text": {
    "TXT_KEY_TECH_CARD_ABJURER": "Abjurer",
    "TXT_KEY_TECH_CARD_ABJURER_HELP": "...",
    "TXT_KEY_TECH_CARD_ABJURER_PEDIA": "..."
  }
}
```

**Rules code:** a card's callbacks (`py_result`, `py_requirement`, `py_targets`, promotion
`py_per_turn`) go in `crates/ffh-core/src/ffh/cards/<suit>.rs`. Each module registers them by name
in its `find_*` lookups. Rules that are not callbacks go where the rule they change lives, with a
comment naming the card.

**Records:** each card that changes the rules is listed in docs/PORTING.md ("Changes from
FFH2", the decks entry). Its row in section 5 below gets its status.

**New data keys** the cards read (each has its reader in `ffh-data` and its line in ffh-play's
help):

| Table | Key | Card | What it does |
|---|---|---|---|
| techs | `peak_movement` | Mountaineering | the team's land units enter peaks (3 moves); +50% defence and sight 3 there |
| techs | `strait_crossing` | Ferries | the team's land units enter a one-plot strait (water with land on two opposite sides) as land |
| techs | `forest_paths` (features) | Forest Paths | in the team's borders its units cross these features as open ground; units at war with it that start their turn in them there have one move fewer (never below one) |
| improvements | `land_bridge` | Causeway | land units move onto and over the water plot as over land; ships cannot enter it; built next to land |
| improvements | `canal` | Canal Locks | ships enter the land plot unless it is in the land of a civ at war with them; built next to water or another canal |
| improvements | `city_heal_tech` (tech) | Siege Lines, Wayside Shrines | units of a team that knows the tech heal on the plot as in a city of their own |
| improvements | `enemy_city_adjacent` | Siege Lines | built only next to a city of a civ the builder is at war with, then also in that civ's land |
| improvements | `requires_route` | Wayside Shrines | built only on a plot with a route |
| builds | `built_by` (unit classes) | Siege Lines, Causeway, Canal Locks, Wayside Shrines | the classes whose units can make the build (added to their `builds` at load) |
| buildings | `border_stops_enemies` | Wall of the Kings | land units at war with the owner, and barbarians, that enter its borders end their move there |
| buildings | `blocks_enemy_ships` | Harbor Chain | ships at war with the city's owner cannot enter the water plots next to the city |
| buildings | `conquest_keeps_all` | Hall of Judgment | the owner's conquests keep every building that can be captured, and lose no population |
| buildings | `naval_capture_percent` | Prize Courts | the owner's ships capture the ships they defeat with this chance |
| promotions | `false_colours` | False Colours | the unit may attack units of a civ at peace without a declaration; the war is declared at the battle, whose defenders get no defensive strikes and no fortification |
| buildings | `pre_battle_damage` `{row, percent, damage_type}` | Thunderhead Tower | at the start of every battle at its city, each attacking unit in `row` (`FRONT`, `MIDDLE`, `REAR`) loses `percent` of its max HP, cut by its resistance to `damage_type`, never below 1 HP; after the morale check and the defensive strikes, not on units that sat out; logged as `LogEvent::BuildingStrike` |
| buildings | `sentinel_ranged_resist` | Arcane Fortress | percent less damage its city's Sentinels take from bombards on the map and from BOMBARD and SHOOT in battle |
| buildings | `sentinel_unit` | Living Walls | its city's Sentinels fight with this unit type's attack and HP (grown by the city's defence as before; one cell each) |
| buildings | `hold_melee_rounds` | Moat | in the first battle at its city each game turn (no battle report at the city this turn), the attacker's STRIKE, CHARGE and STAB actions in rounds 1 to N are spent (`Note::Held`, nothing drawn) |
| buildings | `last_stand_damage` | Last Stand Hymn | in a battle at its city, once half the defender's committed units (not Sentinels) have fallen, the rest deal this percent more damage |
| buildings | `never_lost_damage` | The Unconquered City | while its city has never changed hands (`never_lost` and its original owner owns it), its defenders (units and Sentinels) deal this percent more damage (the pairs' percentage) |
| buildings | `exclusive_with` `[class]` | Elemental Forge | a city with a building of one of these classes cannot build it |
| improvements | `engagement_attacker_slots_change` | Trench | the attacker's slots in battles on its plot (before the minimum; `SlotSource::Improvement`) |
| builds | `all_workers` | Trench | every unit type that builds an improvement can do it (`GameData::derive` adds it to their `builds`: a card's file cannot edit the units') |
| units | `engage_first_round` `{action, pattern}` | Siege Colossus | what it does in a battle's first round from any row, with the row's count, in place of the row's action; its pattern then (`SINGLE` without one) |
| units | `bombard_range` | Siege Colossus | it bombards a city this many plots away (a city on a plot its team has revealed; absent: the adjacent ones) |
| units | `bombard_besieges` | Siege Colossus | a city it bombards counts as besieged until the city's next turn (`City::besieged_by_bombard`) |
| promotions | `first_melee_pattern` | Wardance | the attack pattern of its unit's first STRIKE or CHARGE each round (pass 1) |
| promotions | `heal_behind` | Templar | at the end of each round its unit heals the nearest ally behind it in its lanes this many HP (an event with pass 0; nothing drawn) |
| promotions | `unit_classes` `[class]` | Templar | only units of these classes may take it |
| promotions | `attack_damage_type` | Elemental Forge | its unit's physical attack deals this damage type: the target's resistance to it cuts the attack (as a typed strength's) |

The battle effects are in `crates/ffh-core/src/engagement/arms.rs` (the set-up, `Game`) and
`engagement/formation/arms_cards.rs` (the resolver, a fighter's `Arms`); their AI in
`crates/ffh-ai/src/cards_battle_arms.rs`. `cards::end_turn` (`END_TURNS`) runs a batch's AI once the
player's units have moved (the Siege Colossus bombards then, so its stack is not held back).

A card's improvement comes after the game's own (`GameData::own_improvements`). Map generation
draws nothing for one that never appears (`appearance_probability` 0, as the Trench), so the cards
leave the generated maps as they were. Tests that count or check a table's entries take its own
entries only (`card_entry_count`): the cards' units, buildings and promotions follow their own rules.

## 3. Card world spells and remote targets

- **A card's world spell** is a spell with `"once": true` and `"tech_prereq"` its card. Any unit of
  its owner can cast it, once a game. Its flag is its own (`Player::once_spells`), never the
  civilization's world spell (`global`, `FEAT_GLOBAL_SPELL`).
- **A spell cast at a distance** names its target kind, `"target"`: `plot`, `city`, `unit`,
  `player` or `choice` (one of its own numbered options). It also names the callback listing the
  valid targets for a caster, `"py_targets"`: a `TargetsFn`, `fn(&Game, UnitId, &PyCall) ->
  Vec<SpellTarget>`.
  - `unit_can_cast` passes only while there is a valid target.
  - `unit_cast_at(u, spell, target)` sets `Game::spell_target` for the spell's result to read.
  - A targeted spell cast without a target does nothing.
- **Shared helpers** for the cards go in `crates/ffh-core/src/ffh/cards/world.rs`: a region (the
  largest connected group of unowned plots of a terrain or feature), a new civilization born
  mid-game, "within N plots of your borders" (`within_range_of_borders`), "+N to the Armageddon
  counter" in the points of its percentage, whatever its limit (`counter_change_for_points`), ...
- **Attitude:** a card that changes what a leader thinks of another (Purchase Charter, Cultural
  Annexation: -4) adds to `Player::ai_attitude_extra`. The AI reads it as a term of its relation,
  "acts toward us" (`ffh-ai/src/diplomacy.rs`), on the relation's own scale (2 is a step).

## 4. The Armageddon deck

`crates/ffh-core/src/deck/doom.rs`; its state is `Game::doom`.

- **Slots:** 10, 20, ... 90, and 100 for the ending: one card each, or none.
- **The deal:** on the first turn, slot by slot from the lowest, one card from those whose
  first threshold is the slot. A chain's later stages take the later slots they name; a chain
  is dealt only where those slots are free.
- **Reveal:** every player is shown a card once the counter (`get_global_counter`) is within 5 of
  its slot. A card whose `valid` fails then is dealt again from its slot's cards, or the slot
  stays empty.
- **Firing:** once, when the counter first reaches the slot (`doom_do_turn`, each game turn).
- **The cards** are code: `doom::CARDS` holds FFH2's events (Blight, the Four Horsemen as one
  chain, Hellfire, Wrath, Apocalypse), and `doom_cards::CARDS` the new ones. Each card has a key
  `DOOM_<NAME>`, its thresholds, what each stage does, and an optional condition. Its text is
  `TXT_KEY_DOOM_<NAME>` (the name) and `TXT_KEY_DOOM_<NAME>_HELP` (the effect), in a card file's
  `text`.
- **Sight:** a player may see unrevealed slots ahead (`doom_slot_known`, `player_doom_sight`:
  the Orrery of Ceridwen shows two).
- FFH2's counter triggers (`prereq_global_counter`, weight -1) no longer fire by themselves.

**Random events are gone:** `player_do_events` no longer rolls for a weighted event. The
triggers rules code fires itself still work: the Adaptive and Insane traits, lairs, the
always-firing triggers without a counter threshold.

## 5. Status

Which cards are built. The specs are in docs/DECKS_ROUND8.md (round 8: G, P, E, M, B and A
ids) and docs/DECKS.md (sections 3, 9.4, 10.1, 12, 13).

| Card | Suit | Status |
|---|---|---|
| (the deck itself: deal, discover, once spells, targets, Armageddon deck) | | built |
| E07 Wall of the Kings | Engineer | built |
| E10 Harbor Chain | Engineer | built |
| E12 Siege Lines | Engineer | built |
| E13 Powder Charge | Engineer | built |
| M02 Prize Courts | Merchant | built |
| P03 Hall of Judgment (Courts of Junil renamed) | Prophet | built |
| Causeway | Engineer | built (a dam: ships cannot enter it; built by Work Boats) |
| Canal Locks | Engineer | built |
| Mountaineering | Engineer | built |
| Ferries | Engineer | built |
| Wheeled City | Engineer | built (not in the capital or a holy city) |
| Forest Paths | Prophet | built |
| Wayside Shrines | Prophet | built (the +1 culture left out) |
| False Colours | Prophet | built (land battles only) |
| G07 Astral Leap | Sage | built (world spell at a plot; batch B3, `cards/world_spells.rs`) |
| G10 Seal of Binding | Sage | built (world spell at a unit; B3) |
| B08 Masque of the Red Death | Bard | built (world spell at a city; B3) |
| M08 Purchase Charter | Merchant | built (world spell at a city; B3) |
| B06 Cultural Annexation | Bard | built (world spell at a city; B3) |
| M09 Transmutation Charter | Merchant | built (world spell at a choice: plot and resource; B3) |
| M03 Bazaar of Wonders | Merchant | built (world spell at a city of yours building the wonder; B3) |
| P16 Call of the Faithful | Prophet | built (world spell; B3) |
| M19 Grand Order | Merchant | built (world spell; B3) |
| M18 Mammon's Bargain | Merchant | built (world spell; B3) |
| G14 Thunderhead Tower | Sage | built (`sage_thunderhead_tower.jsonc`) |
| G16 Arcane Fortress | Sage | built (`sage_arcane_fortress.jsonc`) |
| Elemental Forge (G09's rework) | Sage | built: the Fire Forge and the Storm Forge (`sage_elemental_forge.jsonc`) |
| P01 Living Walls | Prophet (the Fellowship of Leaves) | built (`prophet_living_walls.jsonc`) |
| P04 Templar | Prophet (the Order) | built: a promotion for Crusaders (`prophet_templar.jsonc`) |
| P10 Herald of Agares | Prophet (the Ashen Veil) | built (`prophet_herald_of_agares.jsonc`) |
| E04 Siege Ballista | Engineer | built (`engineer_siege_ballista.jsonc`) |
| E05 Trench | Engineer | built (`engineer_trench.jsonc`) |
| E14 Moat | Engineer | built (`engineer_moat.jsonc`) |
| E15 Siege Colossus (the owner's rework) | Engineer | built (`engineer_siege_colossus.jsonc`) |
| B11 The Unconquered City | Bard | built (`bard_unconquered_city.jsonc`) |
| B14 Last Stand Hymn | Bard | built (`bard_last_stand_hymn.jsonc`) |
| B15 Wardance | Bard | built (`bard_wardance.jsonc`) |
