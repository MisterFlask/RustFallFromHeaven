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
| improvements | `deep_spawn_unit_type` (a unit type) | The Lairs Deepen | what a lair spawns once The Lairs Deepen has been played (every lair has one) |
| buildings | `global_counter_commerces` (commerces) | Orrery of Ceridwen | its city's commerce per 10 points of the Armageddon counter |
| buildings | `doom_sight` | Orrery of Ceridwen | how many unrevealed Armageddon cards ahead its owner sees (the Orrery: 2) |
| spells | `py_target_label` | (choice spells; Purchase Charter, Bazaar of Wonders) | the callback naming a `choice` target's option, or adding to another target's name (a city's price; section 3) |
| spells | `cast_from_selection` | Astral Leap | a card's world spell whose effect depends on its caster: offered on a selected unit that can cast it, not among the empire's world actions |
| units, promotions | `battle_spell_power` | Evoker | percent more damage of its damage spells, the magic bolt and Wonder's damage outcomes included |
| units, promotions | `battle_damage_only` | Evoker | it casts no charm, hex or heal (nor Wonder's other outcomes) |
| units, promotions | `battle_spell_pattern` | Battlemage | its damage spells and bolt hit with this pattern in place of their own |
| units, promotions | `battle_front_caster` | Battlemage | from the front row it casts as many spells as from its best row |
| units, promotions | `battle_hex_ward` | Abjurer | while on the field, the enemy's hexes have no effect on its side, the pre-battle hexes included |
| units, promotions | `battle_end_charms` | Abjurer | while on the field, the enemy side's charms end at the start of each round after the first |
| units, promotions | `battle_presence` | Gravity Well, The Whisperer | a pre-battle hex of its presence in the line at the start: `GROUNDING` (the enemy's charges are strikes for the battle), `MADNESS` (in round 1 the enemy's fighter with the highest A, not a Sentinel, hits its own side) |
| units, promotions | `battle_revive` | Halo | once per battle its heal also brings back the first fighter of its side that fell, at this percent of its HP, in its cell if free, else the rear cell of its lanes; never a Sentinel; the kill is undone |
| units, promotions | `battle_dispel` | Spellbreaker | each hit it lands ends the target's charms and its dispellable promotions' attack, defence, magic defence and first strikes for the battle |
| units, promotions | `battle_damage_vs_summons` | Spellbreaker | percent more damage to units with a duration and to illusions |
| units, promotions | `battle_spell` | Homunculus Vats | it knows this battle spell, whatever the spell's prerequisites; one such promotion a unit |
| units, promotions | `strike_spell` | Spellsword | in place of its first strike each round it casts this spell, with half its attack as its magic; one such promotion a unit |
| units, promotions | `opening_cast` | Contingency | a defender: it casts its damage spell (else the bolt) before round 1; the promotion is then lost |
| units, promotions | `death_recall` | Contingency | a killing hit takes it out of the battle (no roll, even in a city); it returns to its capital with 1 HP and the promotion is lost; only with a capital |
| units | `battle_familiar` | Familiars | it binds to a caster (its `summoner`, `SPELL_BIND_FAMILIAR`); in a battle it takes the first spell or shot aimed at that caster, and the auto formation puts it in the caster's lane |
| spells | `battle_illusions` `{count, hp}` | Illusory Host | illusory soldiers cast with the caster's first spell action (below) |
| spells | `battle_charm` `"rows": "REAR"`, `"skip"` | Lullaby of Ceridwen | a charm or hex on the side's rearmost row in use; a hex that makes the fighters it lands on skip their next action |
| civilizations | `unit_promotion` | The Fey Court Awakens, Founding of the Dawn Knights | a promotion every land unit of the civilization gets when it is created (`unit_init`) |
| traits | `unit_rush_gold` | The Condottieri Free State | its player buys units at this many gold per hammer left, under any civic (the hurry rules) |
| buildings | `defender_defense` | Free City Charter | percent more defence for the units fighting in the city (the most of its buildings; an engagement's defence terms) |

The battle effects are in `crates/ffh-core/src/engagement/arms.rs` (the set-up, `Game`) and
`engagement/formation/arms_cards.rs` (the resolver, a fighter's `Arms`); their AI in
`crates/ffh-ai/src/cards_battle_arms.rs`. `cards::end_turn` (`END_TURNS`) runs a batch's AI once the
player's units have moved (the Siege Colossus bombards then, so its stack is not held back).

A card's improvement comes after the game's own (`GameData::own_improvements`). Map generation
draws nothing for one that never appears (`appearance_probability` 0, as the Trench), so the cards
leave the generated maps as they were. Tests that count or check a table's entries take its own
entries only (`card_entry_count`): the cards' units, buildings and promotions follow their own rules.

**Battle magic** (the battle cards; `ffh_data::BattleMagic`, `crates/ffh-data/src/infos/battle_magic.rs`),
read the same way on a unit type and a promotion; a unit has its type's and its promotions'
(`GameData::unit_battle_magic`). The rules are `engagement/battle_magic.rs` (fighters, pairs,
illusions, familiars, Contingency's post-battle steps) and the resolver's `formation/magic.rs`;
ffh-play's promotion and unit help describe every key (`magic_lines`).

**Spells:** `battle_illusions` `{"count", "hp"}`: cast with the caster's first spell action, its
illusory soldiers take the free cells of its side (the front row first, the lanes nearest the
centre first; cells assigned when the battle starts, a cell taken by then stays empty). They do
not act, hold no field (a side of illusions alone is down), die to any spell's hit, give no XP and
exist only in the battle (fighters of the formation after the set-up's; the log names them
`TXT_KEY_UNIT_ILLUSORY_SOLDIER`). `battle_charm` takes `"rows": "REAR"` (the side's rearmost row in use: the middle row of an army of two rows) and `"skip"` (actions each
fighter it lands on skips: its next ones; the log's `Skipped` note).

**The log:** an action's `entered` lists the fighters that enter the field with it (illusions cast,
a fighter revived), each `{unit, lane, row, hp}`; the battle screens fold it
(`battle_log.js`).

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
  largest connected group of unowned plots of a terrain or feature), the plots within N plots of
  a player's borders (`near_borders`, by steps, for every plot at once; `within_range_of_borders`
  for one plot, by plot distance), a unit gaining levels (`unit_gain_levels`, rule 10: the XP for
  N more promotions than it can take now), "+N to the Armageddon counter" in the points of its
  percentage, whatever its limit (`counter_change_for_points`), city sites and a new civilization
  born mid-game (section 6), ...
- **Attitude:** a card that changes what a leader thinks of another (Purchase Charter, Cultural
  Annexation: -4; a born civilization toward its founder: +4) adds to `Player::ai_attitude_extra`,
  on FFH's scale (4 is about a step). The AI counts it at half, its "attitude modifier" term
  (`ffh-ai/src/diplomacy.rs`, `relation_parts`; its step is 2, as for the Trust spell).
- **Naming a target for people:** `Game::spell_target_label(caster, spell, target)`: a plot's
  terrain and feature with its coordinates, a city's name, a unit's name and level, a player's
  name. A `choice` target's option is named by the spell's `py_target_label`, a `TargetLabelFn`
  (`fn(&Game, UnitId, SpellTarget, &PyCall) -> String`; a module with labels adds its lookup to
  `cards::LABELS`); without one it is "Option n". For the other kinds, what the label callback
  returns is added to the target's name (a city's price: `Name: Price: 450 gold`). When the options are entries of an info table
  (a trait, a building, a damage type, ...), the shared `labelInfo(pCaster, iChoice, 'TraitType')`
  names entry `n` of the table its third argument names (`world::LABEL_KINDS`), and the UI shows
  that entry's help beside it.
- **Cast from a selection:** a card's world spell whose effect depends on the unit casting it
  (Astral Leap: the units of the caster's city leap) has `cast_from_selection`: the UI offers it
  on a selected unit that can cast it, not among the empire's world actions.
- **Listing targets once:** `unit_can_cast_untargeted` is `unit_can_cast` without asking a
  targeted spell for its targets: a caller that tries many units (the UI's world actions, the AI)
  asks it for each and lists the targets once, for the caster.
- **In the human's UI** (ffh-play, docs/HUMAN.md): the actions of a targeted spell (the empire's
  world actions, and a selection's spells) carry `target_kind` and `targets`, each `{target, at,
  label, info?}` (`target` the command's form, `at` the plot it is picked on, `info` `[kind, n]` for
  an info table's entry), and a `cast_at {unit, spell, target, confirmed}` command. A plot, city or
  unit is picked on the map (its plots outlined; a plot with several targets asks which), a player
  or a choice from a list. A card's world spell asks to confirm (once a game) and stays listed,
  spent, once cast. `do_command COMMAND_CAST` refuses a targeted spell.

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
- **Sight:** a player may see unrevealed cards ahead (`doom_slot_known`: the next unrevealed
  slots that hold a card; `player_doom_sight` -> `doom_cards::doom_sight`, the most any of the
  player's buildings shows by its `doom_sight`: the Orrery of Ceridwen, two).
- **Named at the reveal:** a card whose targets must be known before it plays (a sea, six hills)
  has an `on_reveal` hook. It is called when its slot is revealed (after a redeal, for the card
  that stays) and returns the targets, which the slot keeps (`DoomSlot::targets`, saved: plot
  indices for the cards so far). Its fire reads them back with `Game::doom_targets(key, stage)`.
  A redeal clears a slot's targets.
- **Lasting cards:** a card whose effect lasts for the rest of the game (The Lairs Deepen, The
  Long Night) fires nothing itself; the rule it changes asks `Game::doom_fired(key)` where the
  rule lives, with a comment naming the card.
- **No barbarians:** a card that brings barbarian units (The Wild Awakens, Werewolf Moon, The
  Blade Returns, The Lairs Deepen) is not valid under No Barbarians, so it is dealt again.
- **The deal with the new cards:** slots 20, 30, ... 100 always hold a card, but 70, which only
  the Four Horsemen's chain reaches (a test checks this); slot 10 has none. A slot's cards share
  it evenly: 30 Blight or the Sea Recedes, 40 three ways (Volcanic Spring, Werewolf Moon, the
  Horsemen, who then take 50-70), 100 four ways.
- **The Gathering's vassals** are marked (`Team::gathered`, saved; cleared when freed): they do
  not count toward their master's domination victory (`team_victory_population`,
  `team_victory_land`) or conquest victory.
- **The human's view:** `me.doom` (`view/cards.rs`) lists the slots with what the human knows:
  a revealed or fired card, or one it sees ahead, by key, name and effect; hidden slots say
  nothing of their card. The reveal and the firing are messages to every player (the log's fog
  lets them through to the human).
- FFH2's counter triggers (`prereq_global_counter`, weight -1) no longer fire by themselves.

**Random events are gone:** `player_do_events` no longer rolls for a weighted event. The
triggers rules code fires itself still work: the Adaptive and Insane traits, lairs, the
always-firing triggers without a counter threshold. Crime, which only gated random events, is
gone too: no state, no `crime` / `prereq_crime` data keys, no help line.

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
| Orrery of Ceridwen (DECKS.md 3.1) | Sage II | built: world wonder, `sage_orrery_of_ceridwen.jsonc`; sight 2, +2 research per 10 counter |
| A01 The Wild Awakens (20) | Armageddon | built |
| A02 The Sea Recedes (30) | Armageddon | built: the sea named at the reveal |
| A09 Volcanic Spring (40) | Armageddon | built: the hills named at the reveal |
| A11 Werewolf Moon (40) | Armageddon | built |
| A30 The Lairs Deepen (60) | Armageddon | built: lasting (`deep_spawn_unit_type`) |
| A32 The Second Sun (60) | Armageddon | built |
| A33 The Blade Returns (50) | Armageddon | built |
| A39 The Twilight of the Gods (80) | Armageddon | built |
| A40 The Sky Falls (80) | Armageddon | built |
| A48 The Gathering (100) | Armageddon | built (not played under No Vassal States) |
| A49 The Long Night (100) | Armageddon | built: lasting |
| A50 The Godless Age (100) | Armageddon | built |
| (the decks' UI: a held card, Discover, cards in the help and the tech tree, card world spells, `cast_at` and target picks, the Armageddon deck in the HUD) | | built |
| G17 Starmetal Meteor | Sage | built |
| B07 Legacy of Kings | Bard | built |
| B18 Anointed Champion | Bard | built |
| G04 Evoker (`sage_evoker`) | Sage | built |
| G05 Abjurer (`sage_abjurer`) | Sage | built |
| G06 Spellsword (`sage_spellsword`: six promotions, one a unit) | Sage | built |
| G18 Gravity Well (`sage_gravity_well`) | Sage | built |
| P05 The Whisperer (`prophet_whisperer`) | Prophet | built |
| P11 Halo (`prophet_halo`) | Prophet | built |
| Spellbreaker (`sage_spellbreaker`) | Sage | built |
| Contingency (`sage_contingency`: Strike and Recall, one held at a time) | Sage | built |
| Battlemage (`sage_battlemage`) | Sage | built |
| Lullaby of Ceridwen (`bard_lullaby_of_ceridwen`: the Lutist) | Bard | built |
| Illusory Host (`bard_illusory_host`: `UNIT_HOST_ILLUSIONIST`) | Bard | built |
| Familiars (`sage_familiars`) | Sage | built |
| Homunculus Vats (`sage_homunculus_vats`: a spell promotion chosen when trained) | Sage | built |
| G12 The Patrian Awakening | Sage | built (batch B4) |
| P18 Founding of the Dawn Knights | Prophet | built (batch B4) |
| P19 The Fey Court Awakens | Prophet | built (batch B4) |
| M07 The Condottieri Free State | Merchant | built (batch B4) |
| Free City Charter (reworked) | Engineer | built (batch B4) |
| Mirror Colony | Sage | built (batch B4) |

## 6. New civilizations

`world::birth(g, &Birth)` gives a new civilization a player slot mid-game; the cards of batch B4
(`cards/new_civs.rs`) use it, and later cards can.

- **The slot:** `world::open_slot`, a spare slot never alive (`GameSetup::SPARE_SLOTS`, 4, shared
  with Hyborem and Basium; never a crusade Host's). With none free, a card that makes a
  civilization cannot be cast: its requirement tests the slot.
- **Who it is:** a civilization and a leader of its own (`civilizations` and `leader_heads` in the
  card's file; neither playable nor AI-playable, so no game starts with it and no colony takes it).
  The slot is added as the Infernals' is (`add_player_advanced`, on a team of its own) and played
  by the founder's controller. Its parent is the founder (`Player::parent`).
- **What it starts with** (`Birth`): the cities it founds at given plots (`NewCity`: the first its
  capital, each with its buildings and units), or cities of the founder it takes over (`release`:
  a trade, so they keep their buildings, with their culture and garrison: the founder's land
  combat units in them but heroes and the caster), a hero in its capital, every tech the founder's
  team knows but the cards (and `techs`), a state religion in all its cities, gold.
- **Its relation to the founder:** it has met it. As a vassal (`vassal`) it is the founder's, by
  the ordinary vassal rules (it joins its master's wars). Like every civilization it is at war
  with the barbarians on both sides (`add_player_advanced` sets only its own side; the other left
  at peace made `ai_vassal_trade` deny the vassal trade). Its AI keeps the vassalage until the
  ordinary revolt rule would free it (`team_can_vassal_revolt`; `diplomacy::reconsider_deals`),
  where a vassal that is not hopeless would otherwise leave after 20 turns. **Where vassals cannot be** (No Vassal
  States, or a founder that is a vassal itself) it signs open borders and a defensive pact with the
  founder instead, a deal like any other. No other war or peace is forced (docs/DECKS.md, principle
  27): its leanings are attitude modifiers, toward the founder (`ai_attitude_extra`) and toward
  every civilization of an alignment (`Player::alignment_attitude`).
- **The AI's attitude** (`ffh-ai`, `diplomacy::relation_parts`) counts both, at half (they are on
  FFH's scale, where 4 is about a step; the AI's step is 2, as for the Trust spell).
- **City sites** (`world::free_site`, `site_value`, `best_sites`, `land_near_borders`): a free site is
  land a city can stand on (`player_can_found`'s terrain, feature, peak, permanent improvement and
  mana rules), with no unit, no city within `MIN_CITY_RANGE`, unowned; the best are taken by the
  land, bonuses, water and fresh water in their radius.
- **Its own rules** are data where they can be: a civilization's `unit_promotion` (the Fey
  Glamour, the Dawn Vow), a civilization trait (`civ_trait`: the Free Companies'), a palace's
  `free_building` (the Patrians' Mage Guilds), unique units in its `units`. What data cannot say is
  a rule hooked where it lives (section 2), each naming its card.
- **Art:** a new leader shows an existing leader's portrait and a new unit an existing unit's
  sprite (`PORTRAIT_ALIASES` in `crates/ffh-play/assets/app/portraits.js`, `UNIT_ALIASES` in
  `iso.js`).
