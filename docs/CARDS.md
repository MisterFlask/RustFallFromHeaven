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

- **Band:** the card's `band`, 1, 2 or 3 (shown I, II, III): the order in which the deck deals
  its cards.
- **Needs:** the tech's `and_pre_reqs` / `or_pre_reqs`, the ordinary techs its content needs. A
  card is discovered only once its owner knows them.
- **Alignment:** `card_alignment` (`ALIGNMENT_GOOD`, `ALIGNMENT_EVIL`) limits who draws it;
  `card_block_alignment` keeps one alignment from drawing it (a [good or neutral] card blocks
  `ALIGNMENT_EVIL`, an [evil or neutral] card `ALIGNMENT_GOOD`).
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
- **Draw:** from the cards of the suit still in the deck that the player's alignment may draw,
  those of the lowest band among them: the deck is dealt in band order, whoever draws. One
  `soren_rand_num` over the candidates in id order.

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
      "band": 2,
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

**Tiers and counts:** a card's buildings have no tier variants (`building_variants.jsonc` is read
before the card files): an ordinary one is Standard everywhere, a national one rolls a tier for its
cost only. Tests that count a table, or check every entry of it against FFH2's numbers, count the
game's own entries: the first `len() - card_entry_count(table)` ids.

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
| units | `engage_first_round` `{action, pattern}` | Siege Colossus | what it does in a battle's first round from any row, with the row's count, in place of the row's action; its pattern then (`SINGLE` without one) |
| units | `bombard_range` | Siege Colossus | it bombards a city this many plots away (a city on a plot its team has revealed; absent: the adjacent ones) |
| units | `bombard_besieges` | Siege Colossus | a city it bombards counts as besieged until the city's next turn (`City::besieged_by_bombard`) |
| promotions | `first_melee_pattern` | Wardance | the attack pattern of its unit's first STRIKE or CHARGE each round (pass 1) |
| promotions | `heal_behind` | Templar | at the end of each round its unit heals the nearest ally behind it in its lanes this many HP (an event with pass 0; nothing drawn) |
| promotions | `unit_classes` `[class]` | Templar | only units of these classes may take it |
| promotions | `attack_damage_type` | Elemental Forge | its unit's physical attack deals this damage type: the target's resistance to it cuts the attack (as a typed strength's) |
| terrains | `no_improvement` | The Sky Falls (Crater) | no improvement can be built on a plot of this terrain, or of a temporary terrain over it, whatever its plot type allows (no mine on Crater's hills); routes can (`plot_can_have_improvement`) |
| terrains | `unworkable` | The Sky Falls (Crater) | no city works a plot of this terrain, or of a temporary terrain over it (`city_can_work`; the governor and the AI place citizens through it) |
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
| traits, civics | `unit_rush_gold` | The Condottieri Free State (a trait), Gold Standard (a civic), the Court of Mammon (a trait) | its player buys units at this many gold per hammer left, under any Labor civic (the most its traits and civics give; the gold hurry, `player_unit_rush_gold`) |
| buildings | `defender_defense` | Free City Charter | percent more defence for the units fighting in the city (the most of its buildings; an engagement's defence terms) |
| techs | `card_block_alignment` | Font of Rebirth | players of this alignment do not draw the card (a [good or neutral] card blocks evil; section 1) |
| buildings | `train_units` `[{unit, cost, bonus, national}]` | Beast Market, Elemental Foundry | the city trains each unit (one without a cost of its own) at `cost`, where it has `bonus`; past `national` units of the class, each costs `NATIONAL_UNIT_OVER_LIMIT_COST_PERCENT` |
| buildings | `free_bonus_choices` | Agares's Well | when it is built its owner chooses one of these resources (`Controller::choose_free_bonus`; kept in `City::free_bonus_choices` while the city has the building); the city has one of it |
| buildings | `free_promotion_choices` | Spire of the Archmagi | a unit trained in the city takes one it can acquire, its owner's choice (`Controller::choose_free_promotion`) |
| buildings | `rebirth_range`, `rebirth_promotion` | Font of Rebirth | the first time a unit of the owner would die in a battle within the range, it is carried to the city at 10% of its HP and gains the promotion; a unit with it dies normally (`engagement/apply.rs`) |
| buildings | `free_unit_turns`, `free_unit_classes` | Maponos's Cradle | every N turns after it is built, a free unit of the first class that applies (a city founder only while a site within 10 plots is free) |
| civics | `unit_cost_modifier`, `unit_combat_cost_modifiers` | Magocracy | percent change to the cost of every unit, and of the units of each unit combat |
| civics | `no_war_weariness` | Stoicism | war weariness causes no unhappiness |
| processes | `production_to_experience` | Muster | hammers per point of experience to the units in the city |
| improvements | `claim_range`, `min_city_distance` | Trading Post | its plot and the land plots within the range are its builder's territory whatever the culture (it is built only outside every border); built only that many plots or more from every city |
| builds | `same_builders_as` | Trading Post (the Fort's builders); Siege Lines, Canal Locks, Wayside Shrines, Trench (the Road's: the cities' crews and army units, docs/WORKS.md); Causeway (the Fishing Boats': Work Boats) | every unit that can do that build can do this one (set in the units' `builds` at load; a card's file cannot edit the units') |
| spells | `ritual_charge` (a project) | Weather Engine, Tidecall of Danalin | each completion of the ritual leaves its city a charge (`City::ritual_charges`); the spell is cast only by a unit in a city of its owner holding one, and spends it; a city holding a charge does not build the ritual again (section 3, "Rituals") |
| projects | `repeat_cost_percent` | Pillar of Dawn | each completion by a team makes the ritual's next cost this percent of the last (200: twice; at most 16 times) |
| buildings | `reveals_sites` | The Second Sky | when built, its owner's team has every plot with a mana resource, a unique feature, a lair (an improvement that spawns) or a goody hut revealed |
| civilizations | `hills_yield_change` (yields) | Wake the Hill Clans | the yields each plot on hills it owns gives more (`plot_calculate_nature_yield`) |
| units, promotions | `battle_shield` | Shieldbearer | in the front row, the SHOOT, BOMBARD and SPELL aimed at a unit in its lanes hit it instead (as a Sentinel takes them); it takes this percent less damage from SHOOT, BOMBARD and SPELL hits |
| units, promotions | `shots_at_walls` | Fire Arrows | its SHOOT takes the enemy's standing Sentinels first, as BOMBARD does, at a stone's damage (no city penalty on a Sentinel) |
| promotions | `damage_vs_walls` (the unit key, on a promotion too) | Fire Arrows | percent more damage to Sentinels, added to its unit type's |
| units, promotions | `battle_drums` | War Drums | while it is committed (and does not sit out), its side is immune to fear at the morale check, and has this much more speed in round 1 (the most of its side's drums) |
| units, promotions | `charge_anywhere` | Gryphon Riders | its CHARGE is a charge where flanking works, whatever the ground: any enemy, the highest threat for its HP, the first round's bonus |
| units | `no_rest_on_water` | Gryphon Riders | a path ends none of its turns over water, and at the end of its owner's turn it lands on the nearest land plot it could enter (`player_do_turn`) |
| units, promotions | `battle_immune` `["SPELLS", "CHARMS", "HEXES"]` | Runeguard, Witch-Hunter, Automatons | spell damage does nothing to it (its expected damage is 0, so casters aim elsewhere); its side's charms, the enemy's hexes (a presence's too: not maddened, not grounded) do not land on it |
| units | `attack_damage_type` (the promotion key, on a unit type too) | Lantern of Lugus | its physical attack deals this damage type (the Dawnbringer: holy) |
| units | `no_experience` | Automatons | it gains no experience |
| units | `heal_building_class` | Automatons | it heals only in a city of its team with a building of this class (a Forge) |
| units, promotions | `battle_sentinels`, `battle_sentinel_defense` | Bastion | when its plot is attacked, its side fights with this many more Sentinels, their HP and defence grown by `battle_sentinel_defense` percent (below, "Field Sentinels") |
| improvements | `battle_sentinels`, `engagement_defense` | Cairn Golem | in battles on its plot the defender fights with this many Sentinels (grown by `engagement_defense`), and its units have `engagement_defense` percent more Dv (not A) |
| promotions | `naval_damage_vs` (unit classes) | Leviathan Harpoons | percent more damage its ship deals in the rounds it wins of a naval battle against units of these classes |
| leader_heads | `minor_leader_of` (a civilization) | The Sundering | a minor leader of the civilization, which none lists: a faction the card splits from it may have it when the civilization's own leaders are in play (section 4); the leader's help names the civilization |
| buildings | `legion_cost_percent` | The Gilded Legion | once built, the city chooses a civilization it has met (`SPELL_GILDED_LEGION`, at a player: `City::legion_civ`); it then trains that civilization's unit of each class (not world units) as that civilization's city would, at this percent of its cost (`city_legion_unit_civ`) |

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

**Battle kit** (the batch "battle more"; `ffh_data::BattleKit`, `crates/ffh-data/src/infos/battle_kit.rs`),
read the same way on a unit type and a promotion; a unit has its type's and its promotions'
(`GameData::unit_battle_kit`). The rules are `engagement/more_cards.rs` (the fighters' `Kit`, the field
Sentinels, the Cairn's defence, the drums at the morale check, the harpoons at sea) and the resolver's
`formation/more_cards.rs` (reach, flanking, damage, charms and hexes, the drums' round); their AI is
`crates/ffh-ai/src/cards_battle_more.rs`; ffh-play's help describes every key (`kit_lines`).

**Field Sentinels** (the Bastion, the Cairn): the defender's candidates' `battle_sentinels` and the
defended plot's improvement's are Sentinels of the defender: `UNIT_SENTINEL`'s values, HP and
defence grown by the most of their sources' growth (`battle_sentinel_defense`, the improvement's
`engagement_defense`). They take the defender's cells after the city's Sentinels (`Slots::field_sentinels`,
the front row, the lanes nearest the centre first), cost no slot, are whole at the start of each
battle (nothing keeps their damage: `Setup::field`, `SentinelSource::Field`) and hold no plot: the
plot falls once the defender's units are gone. A Bastion that cannot attack brings none to its stack's
attacks.

## 3. Card world spells and remote targets

- **A card's world spell** is a spell with `"once": true` and `"tech_prereq"` its card. Any unit of
  its owner can cast it, once a game for the owner's team: a card is its team's, so a teammate
  cannot cast it again (`player_cast_once`). Its flag is its own (`Player::once_spells`, the
  caster's record), never the civilization's world spell (`global`, `FEAT_GLOBAL_SPELL`).
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

**Rituals** (a card's projects, `crates/ffh-core/src/ffh/cards/world_spells_2.rs`):
- A ritual is a project with `max_global_instances` and `max_team_instances` -1: built again
  after it completes. Its effect is the project's own, at completion (`ev_project_built` calls the
  batch's `project_built`; FFH2's rituals act there too).
- **A ritual with a choice** (where its effect falls) leaves a charge instead: a spell naming it as
  `ritual_charge` is targeted, cast by a unit in the city that holds the charge (a selected unit's
  spell in the UI: the spell is not `once`), and the cast spends the charge. The city builds the
  ritual again only once the charge is spent (`cannot_create`). A captured city loses its charges.
- **A ritual whose cost grows** names `repeat_cost_percent` (`team_repeat_project_cost`, from the
  team's `project_count`).

**World rituals** (the civilizations' world spells; `crates/ffh-core/src/ffh/world_rituals.rs`,
the owner's decision of 2026-11-11): the world spell of each civilization (`global`) names its
ritual as `ritual_charge`, a project with `world_ritual` (`PROJECT_RITUAL_<NAME>`, repeatable,
`prereq_civilization` the spell's, `repeat_cost_percent` 200). The variant of the charge:
- The charge is the empire's, held in the city that built it: any unit of the owner that meets the
  spell's other requirements casts it while any of the owner's cities holds one, and the cast spends
  that city's (`ritual_charge_city`, `unit_has_ritual_charge`). A captured city loses its charges.
- One charge at most per player, one city building it at a time (`world_rituals::cannot_create`,
  player-wide, outside a visibility test).
- Two limits are no longer the world rituals' own: since the owner's decision of 2026-11-12
  **every project is a Ritual** (`crates/ffh-core/src/ffh/rituals.rs`; the keyword `Ritual`,
  docs/PLAY_DESIGN.md "Keywords and reminder text"), and for every ritual:
  - Not parked: with production in it, it stays the head of its city's queue; taken off or put
    behind another order, its production is lost (`city_check_rituals`, after every insertion and
    removal in `city_push_order_kind` / `city_pop_order`; the "abandoned" news only for a world
    ritual). Requisitions and empire orders are refused in front of one in progress
    (`city_push_order_kind` returns `None`).
  - Not hurried with gold: a hurry with a gold cost (`gold_per_production > 0`) is refused while
    the city's first build is a ritual (`city_can_hurry`: shown, disabled). Hurrying with
    population is allowed, and so is a great person's hurry (`unit_can_hurry`: production, not
    gold); in FFH2 neither hurried a project. One-step production (`city_change_production`: the
    Soul Forge, a Great Engineer) goes into a ritual as into any build.
- The cost doubles per completion by the player, `max(0, world_ritual_completions -
  world_ritual_refunds)` times, with no cap on the count (`grow_cost`, at most
  `WORLD_RITUAL_MAX_COST`); `player_project_production_needed_at(p, project, extra)` gives the
  cost after `extra` more. The Pillar of Dawn keeps the team count and its 16-step cap.
- News to every living player (`GameLog::msg_at` per player, at the city): `<Civilization> began
  <ritual> in <city>.` when production first goes into it, `abandoned` and `completed` likewise.
- The Sanctuary's waits while `sanctuary_timer` is above 0; `GAMEOPTION_NO_WORLD_SPELLS` forbids
  every world ritual. A `world_ritual` project no spell names simply completes and counts.
- The rituals and spells that replaced FFH2's world spells have their rules in
  `crates/ffh-core/src/ffh/world_ritual_spells/` (one module per batch; `mod.rs` lists each
  module's `Hooks`: its callbacks, a `cannot_create`, a `project_built` at completion, and a
  `check` at the start of each player's turn and after a war, a peace or a death). `small.rs`: the
  Calabim's The Tithe (needs a rival named by Name the Tithe, a `world_action`; acts at completion;
  cancelled by peace or the rival's death), the Doviello's The Cull (a plot), the Luchuirp's
  Upgrade (acts at completion: `Unit::reinforced`), the Amurites' Wellspring (a choice: plot and
  mana type; placed by the player, no unit casting it: below; its ritual needs Knowledge of the
  Ether, the tech Fire and Death nodes need to count).
- **Placed world spells** (`place_on_charge` in spells.jsonc, only Wellspring): the player places
  the spell with its ritual's charge, no unit casting it (`Game::player_can_place`,
  `player_place_targets`, `player_place`: the charge spent from the first city holding one, the
  cast counted in `Player::world_spell_casts` and announced, then the effect). `unit_can_cast`
  refuses such a spell to every unit. The spell's `py_targets` names its placement
  (`world_rituals.rs` `place_hooks`: Wellspring's `wellspring_targets` and `wellspring_place`). The
  page opens the placement on the turn the ritual completes (docs/HUMAN.md).
- Queries: `Game::world_rituals() -> Vec<WorldRitual>` (each living player's: `project`, `spell`,
  `city`, `progress`, `cost`, `turns_left`, `charged`, `completed`, `next_cost`),
  `player_world_ritual(p)`, `project_is_world_ritual(project)`, `city_world_ritual_in_progress(c)`.
  Tests give a player a charge with `player_grant_world_ritual_charge(p)`.

**Area effects:** `Game::area_effects` (saved) holds the timed effects over the plots within a
radius of a plot (`AreaEffect`: its kind, owner, plot, radius and the first turn it no longer
holds; `moved`, the turn its owner last moved it): the Weather Engine's Blizzard, Storm and Fog,
the Pillar of Dawn, and the Lanun's Raging Seas (`RagingSeas`, `ffh/world_ritual_spells/lanun_khazad.rs`:
radius 2, 15 turns, one per player, its strike at the start of the owner's turn from
`player_set_turn_active`, moved by Steer the Storm). An effect ended is
dropped at the end of a game turn (`area_effects_do_turn`). The client draws each effect a player
sees a plot of, and the Khazad's tunnel ends (`Game::delves`, The Delve), from the view's
`area_effects` and `delves` (ffh-play `view/marks.rs`, `assets/app/marks.js`). Where the rules ask
(`plot_has_weather`, `unit_in_pillar_of_dawn`, `plot_fog_hides`): healing (`unit_heal_rate`), the
moves a turn starts with (`unit_turn_moves_at`), a battle's set-up (`engage_arms_weather`: a
fighter's `ranged_resist`, its `hold_shot_rounds`), invisibility (`unit_is_invisible`,
`unit_is_invisible_in_plot`), a unit's turn (`unit_do_turn`: the Pillar's heal).

**Truces:** `Team::truce_until` (saved, the same on both teams): Judgment of Junil's. While
`team_has_truce` holds there is no war between the two teams: `team_can_declare_war` refuses it
(and a war on a vassal of the other), `team_declare_war` does nothing (so the declarations other
wars drag in, defensive pacts and vassals, skip the pair; `team_wars_after_declaring` previews the
same), False Colours do not reach them, and their always-hostile (hidden nationality) units are no
enemies of each other (`unit_is_enemy`, `unit_get_combat_owner`, `puf_is_enemy_with`,
`puf_is_potential_enemy`). The AI's war planning sees it through `team_can_declare_war`.

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
- **Lasting cards:** a card whose effect lasts for the rest of the game (The Long Night) changes
  a rule that asks `Game::doom_fired(key)` where the rule lives, with a comment naming the card.
- **No barbarians:** a card that brings barbarian units (The Wild Awakens, Werewolf Moon, The
  Blade Returns, The Brood's nests) is not valid under No Barbarians, so it is dealt again.
- **The deal with the new cards:** slots 20, 30, ... 100 always hold a card, but 70, which only
  the Four Horsemen's chain reaches (a test checks this); slot 10 has none. A slot's cards share
  it evenly (a test checks the lists): 20 the Wild Awakens; 30 four ways (Blight, the Sea
  Recedes, the Scaled Kingdom, the Brood, which then takes 50); 40 three ways (Volcanic Spring,
  Werewolf Moon, the Horsemen, who then take 50-70); 50 the Blade Returns; 60 the Infernal Gate
  Opens; 80 two ways (Hellfire, the Sky Falls); 90 two ways (Wrath, the Sundering); 100 two ways
  (Apocalypse, the Long Night).
- **Crater** (`TERRAIN_CRATER`, defined in `cards/doom_sky_falls.jsonc`): what The Sky Falls
  leaves. In each struck city (largest first) half of its work radius's land plots, rounded down,
  become Crater: not the city's plot, peaks, water, another city's plot or Crater already; drawn
  one at a time with `soren_rand_num` from the plots in the radius's order (`crater_sites`). The
  plot becomes hills (`PLOT_HILLS`) with terrain Crater and loses its improvement, feature and
  resource; its route stays. Its movement, defence, sight and battle ground are the hills'
  (terrain movement 1, defence 0, as Grassland); it yields nothing (no yields of its own, and
  `hills_yield_change` takes back the hills' production); no feature or resource is valid on it.
  It is unimprovable and unworkable (section 2: `no_improvement`, `unworkable`). The clients draw
  it with the tundra's ground and hills (`TERRAIN_CRATER` in `iso.js`, `r3d/world.js`,
  `tiles.js`, `battle_stage.js`, `web/art/terrain/manifest.txt`); the plot tooltip shows its
  rules (the terrain's `effects` in `data`).
- **Cut cards** (the owner's call, 2026-11-08): The Godless Age, The Gathering, The Twilight of
  the Gods, The Lairs Deepen and The Second Sun are gone, with their rules and text.
- **Civilizations of the deck** (batch B8, `ffh/cards/new_civs_2.rs`): the Scaled Kingdom (30),
  the Brood (its nests at 30, the civilization at 50 where a nest still stands) and the Infernal
  Gate (60: the Court of Mammon, below). They are born as section 6 says, without a founder
  (`Birth::wild`), each in a player slot added for it.
- **The Court of Mammon** (the Infernal Gate, the owner's design of 2026-11-08; its data in
  `doom_infernal_gate.jsonc`): an evil infernal faction of Mammon's, not Hyborem's, and no part of
  the Infernal Pact (which is FFH2's again). It arrives without a city (`Birth::camp`) on the best
  free site within 4 plots of the borders of the civilization with the most gold (`court_patron`:
  ties to the lower id; `court_site`): Aurivane (its hero), two Champions, two Longbowmen, two
  Settlers, two Manes, 200 gold. Its leader is Avaricious (`TRAIT_AVARICIOUS`: +100% gold,
  `unit_rush_gold` 2) and Organized; every land unit of it has Mammon's Coin (its civilization's
  `unit_promotion`), whose Buy Out (`SPELL_COURT_BUY_OUT`, while the Court owns the unit) is the
  Paymaster's spell (`cards/economy.rs`'s callbacks and price). **Population:** the Infernals' rules
  are the Court's (`infernal_population`: Fallow, its civilization trait; cities of 3 with Demonic
  Citizens, `ev_city_built`, `ev_city_acquired`; no Manes from a city it founded, `ev_city_razed`;
  the Beast of Agares costs its cities no population; the Blight and the Pestilence spare its
  cities), and the Manes of the evil dead (`cf_gift_unit` from `ev_city_razed` and
  `ev_unit_killed`) go to one of the two (`manes_civ`): the one in play with a city, else, while
  both are, one drawn with the game RNG (one `soren_rand_num`, only then). The AI plays its Manes
  as the Infernals' (`civs`: they walk home and join cities) and buys with every unit
  (`cards_economy`, at the start of its turn and once its units have moved). -4 attitude between
  it and the Infernals, both ways, set when the second of them arrives (`court_infernal_grudge`;
  no war is forced). It is played with a free site beside the richest civilization.
- **The Sundering** (90, the owner's design of 2026-11-08; `ffh/cards/sundering.rs`, its text
  and invented leaders in `doom_sundering.jsonc`): in player id order, every living civilization
  (`world::civ_players`: not the barbarians, minor civilizations or crusade Hosts; not the Sheaim
  or the Infernals) with 4 cities or more and a capital splits (`sundering_plan`, drawn before the
  first split, so a faction born now does not split again). The quarter of its cities farthest
  from its capital (plot distance, rounded down, at least one; ties: the lower city id;
  `seceding_cities`) go to a new faction, a `world::birth` of its civilization type with `release`
  (as the Usurper's Crown: buildings, culture and land garrisons stay, heroes go with the parent;
  the most populous city its capital), named "<short description> of <capital>" ("Bannor of
  Elam"). It is `Birth::wild` with a `parent`: the AI plays it, it knows its parent's techs (not
  the cards), it is nobody's vassal and at peace with every civilization (a leader with the
  Barbarian trait, `barbarian_ally`, is at peace with the barbarians as at a game's start), it
  has met every civilization its parent has met, and `ai_attitude_extra` is -4 between it and
  its parent both ways. **Its leader** (`faction_leader`): the first by id of its civilization's
  `leaders` not in play this game (the leader of a player ever alive) nor given to a faction of
  the same firing, else the first such of its minor leaders (`leader_heads` `minor_leader_of`,
  which no civilization lists, so no game starts with one): FFH2's scenario leaders, each by the
  civilization its scenario gave it (Tethira: Bannor; Kane: Malakim; Thessalonica: Elohim;
  Shekinah: Sidar; Koun: Grigori; Ostanes, Uldanor: Hippus; Tya Kiri: Amurites; Duin Halfmorn:
  Doviello; Furia, Melisandre: Balseraphs; Hafgan: Clan of Embers; Rivanna, Volanna: Svartalfar;
  Mahon: Calabim; Dumannios, Riuros, Anagantios: Illians; Weevil and Braeden, whose traits are
  Foolish and Weak, are left out), and six invented for the civilizations FFH2 gave none, each
  with the AI personality of a leader of its civilization: Torrin Coldforge (Luchuirp: Industrious,
  Organized), Ysolde Lorda (Kuriotates: Expansive, Organized), Caelith Sunbough (Ljosalfar:
  Creative, Defender), Durgrim Deepvault (Khazad: Defender, Financial), Selka Saltreach (Lanun:
  Expansive, Raiders), Raziel (Mercurians: Defender, Spiritual). A civilization with no free leader
  does not split: the civilizations born of cards have one leader each, in play. The card is
  played only when some civilization splits (`valid_sundering`). The minor leaders show the
  random portrait (`PORTRAIT_ALIASES`). **Under the AI** (the card fired by hand at turn 150 of
  four 6-civilization standard games, seeds 1-4, then 40 turns played; 16 factions): the
  factions keep their garrisons (none shrank: with one or two cities they are fully funded at 0
  gold) and build more (1-4 land units at the split, 3-7 forty turns later). They are weak (most
  have one city), and in 4 of 16 the parent, or once a neighbour, declared war and took the
  faction's city within 20-35 turns; the others lived on at peace.
- **No card alters a victory condition** (the owner's rule, 2026-11-08): what a card produces
  counts normally toward every victory.
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
| E11 Pioneers | Engineer | built (+100% work rate of the cities' crews; a national worker until 2026-11-09, docs/WORKS.md) |
| M01 Gold Standard | Merchant | built |
| M06 Beast Market | Merchant | built |
| M10 Admiralty | Merchant | built (free picks: 5 experience, two promotions) |
| G01 Elemental Foundry | Sage | built (300 hammers an elemental) |
| G19 Magocracy | Sage | built |
| P07 Golems of the Earthmother | Prophet (Runes of Kilmorph) | built |
| Trading Post | Merchant | built (II; claims land plots only) |
| Font of Rebirth | Prophet [good or neutral] | built |
| Spire of the Archmagi | Sage | built |
| Academy of War | Bard | built (free picks: 5 experience, two promotions) |
| Mana Grafting | Sage | built (once a node, for good) |
| Stoicism | Sage | built |
| Maponos's Cradle | Prophet | built |
| Agares's Well | Prophet [evil] | built (II) |
| Paymaster | Merchant | built |
| Muster | Bard | built (II) |
| E07 Wall of the Kings | Engineer | built |
| E10 Harbor Chain | Engineer | built |
| E12 Siege Lines | Engineer | built |
| E13 Powder Charge | Engineer | built |
| M02 Prize Courts | Merchant | built |
| P03 Hall of Judgment (Courts of Junil renamed) | Prophet | built |
| Causeway | Engineer | built (a dam: ships cannot enter it; built by the cities' crews) |
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
| A33 The Blade Returns (50) | Armageddon | built |
| A40 The Sky Falls (80) | Armageddon | built: Crater (half the land of each struck city's radius; unimprovable, unworkable) |
| A49 The Long Night (100) | Armageddon | built: lasting |
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
| B02 Sing the Stones (`bard_sing_the_stones`) | Bard | built (world spell; batch B7, `cards/world_spells_2.rs`): the City Ruins within 3 plots of the borders where a city may be founded |
| B13 Dragon's Oath (`bard_dragons_oath`: `UNIT_SYLVARITH`) | Bard | built (world spell; B7): Sylvarith in the capital, one in the world; Breath Fire is its own copy (no Dragon promotion, which gives Fear) |
| P06 Call the Leviathan (`prophet_call_the_leviathan`) | Prophet (the Octopus Overlords) | built (world spell at a coastal city; B7): a Kraken while a Leviathan lives |
| Judgment of Junil (`prophet_judgment_of_junil`) | Prophet [good] | built (world spell at a player; B7): needs Orders from Heaven (the Scales of Junil's); a war ends; 20 turns of truce (`Team::truce_until`) |
| Earthshaper's Decree (`engineer_earthshapers_decree`) | Engineer | built (world spell; B7): the crews' builds that keep the plot's feature and resource |
| The Second Sky (`sage_second_sky`) | Sage | built (national wonder, `reveals_sites`; B7): at completion; recon units trained there +1 sight |
| The Plagiarist's Lens (`sage_plagiarists_lens`) | Sage | built (world spell at a choice; B7): a once-a-game spell, not a national building; -2 attitude from each holder, for good (no 50-turn timer) |
| Weather Engine (`engineer_weather_engine`) | Engineer | built (ritual, 300, with a charge; B7): Blizzard, Storm, Fog |
| Tidecall of Danalin (`prophet_tidecall_of_danalin`) | Prophet | built (ritual in a coastal city, 150, with a charge; B7) |
| P12 Pillar of Dawn (`prophet_pillar_of_dawn`) | Prophet (the Empyrean) | built (ritual, 500, each completion twice the last; B7) |
| G12 The Patrian Awakening | Sage | built (batch B4) |
| P18 Founding of the Dawn Knights | Prophet | built (batch B4) |
| P19 The Fey Court Awakens | Prophet | built (batch B4) |
| M07 The Condottieri Free State | Merchant | built (batch B4) |
| Free City Charter (reworked) | Engineer | built (batch B4) |
| Mirror Colony | Sage | built (batch B4) |
| Rite of the Necropolis | Prophet [evil or neutral: `card_block_alignment` good] | built (batch B8; its cities start with two Pyre Zombies each) |
| Wake the Hill Clans | Engineer | built (batch B8; the hills: 6 or more within 3 plots of its first city) |
| The Usurper's Crown | Bard | built (batch B8; world spell at a player) |
| Oath of Fealty | Merchant | built (batch B8; world spell at a player; not under No Vassal States) |
| Call the Aifon | Merchant | built (batch B8; four Axemen and two Archers) |
| A04 The Scaled Kingdom (30) | Armageddon | built (batch B8) |
| A06 The Brood (30, 50) | Armageddon | built (batch B8; not played without barbarians) |
| A22 The Infernal Gate Opens (60) | Armageddon | built (batch B8; reworked 2026-11-08: the Court of Mammon, beside the richest civilization; no tie to the Infernal Pact) |
| The Sundering (90) | Armageddon | built (the owner's design, 2026-11-08; `doom_sundering.jsonc`: six invented minor leaders, 18 of FFH2's scenario leaders repurposed; the card-born civilizations do not split) |

| P17 Sanctified Steel (`prophet_sanctified_steel`) | Prophet | built (no religion needed in the city) |
| Shieldbearer (`engineer_shieldbearer`) | Engineer | built |
| Fire Arrows (`engineer_fire_arrows`) | Engineer | built |
| War Drums (`bard_war_drums`: the War Drummer) | Bard | built |
| Leviathan Harpoons (`merchant_leviathan_harpoons`) | Merchant | built (the large ships are the tier 4 ones, the Man-o-War and the Queen of the Line) |
| Runeguard (`engineer_runeguard`) | Engineer | built (`battle_immune`; FFH2's `immune_to_magic` is still map only) |
| Witch-Hunter (`prophet_witch_hunter`) | Prophet | built |
| Gryphon Riders (`bard_gryphon_riders`) | Bard | built (withdrawal 25%, as the other riders: not in the spec's sheet) |
| The Gilded Legion (`merchant_gilded_legion`) | Merchant | built (the choice is a spell of a unit in the city) |
| Cairn Golem (`engineer_cairn_golem`: Raise the Cairn) | Engineer | built |
| Automatons (`engineer_automatons`) | Engineer | built (no supply) |
| Bastion (`engineer_bastion`) | Engineer | built (its Sentinels fight when its plot is attacked, not in its stack's attacks) |
| Lantern of Lugus (`prophet_lantern_of_lugus`: the Dawnbringer) | Prophet | built (no Glare; an archer's rows and levers, its shots holy) |
## 6. New civilizations

`world::birth(g, &Birth)` gives a new civilization a player slot mid-game and returns it; the
cards of batches B4 (`cards/new_civs.rs`) and B8 (`cards/new_civs_2.rs`, the Armageddon cards'
too) use it, and later cards can.

- **The slot:** a new one, after every existing id, the barbarian's included
  (`Game::add_player_slot`, docs/ARCHITECTURE.md, "Players added mid-game"; `world::next_player`
  names it before the birth). Nothing is reserved and there is no limit, so no card waits for a
  slot: its requirement or `valid` tests only its land and its once-a-game rule.
- **Who it is:** a civilization and a leader of its own (`civilizations` and `leader_heads` in the
  card's file; neither playable nor AI-playable, so no game starts with it and no colony takes it),
  or a civilization that exists with a leader of its own (the Usurper's Crown: its parent's
  civilization, `LEADER_USURPER`, its parent's alignment; `description` names it, "Bannor
  Usurpers"). The slot is added as the Infernals' is (`add_player_advanced`, on a team of its own)
  and played by the founder's controller. Its parent (`Player::parent`) is the founder, or
  `parent` (the civilization the Usurper secedes from).
- **Without a founder** (`Birth::wild`: an Armageddon card's civilization) it is played by its
  new slot's own controller (the barbarian slot's: the game's AI, never a human), has no parent, is
  nobody's vassal and knows the world's techs (`world::world_techs`: those at least half the
  civilizations' teams know, never a card), with `techs` (the Scaled Kingdom's Animal Handling).
- **What it starts with** (`Birth`): the cities it founds at given plots (`NewCity`: the first its
  capital, each with its buildings and units), or units on a plot without a city (`camp`: the Court
  of Mammon's, whose settlers found its cities; its hero stands there), or cities it takes over
  from their owner (`release`: a trade, so they keep their buildings, with their culture and
  garrison: the owner's land combat units in them but heroes and the caster), a hero in its
  capital, every tech its parent's team knows but the cards (and `techs`), a state religion in all
  its cities, gold.
- **Its relation to the founder:** it has met it (and its parent). As a vassal (`vassal`) it is the founder's, by
  the ordinary vassal rules (it joins its master's wars). Like every civilization it is at war
  with the barbarians on both sides (`add_player_advanced` sets only its own side; the other left
  at peace made `ai_vassal_trade` deny the vassal trade). Its AI keeps the vassalage until the
  ordinary revolt rule would free it (`team_can_vassal_revolt`; `diplomacy::reconsider_deals`),
  where a vassal that is not hopeless would otherwise leave after 20 turns. **Where vassals cannot be** (No Vassal
  States, or a founder that is a vassal itself) it signs open borders and a defensive pact with the
  founder instead, each held in a standing deal like any other, so that a war or a cancellation
  ends it (a pact no deal recorded outlived every war, and once had the civilization declare war
  on itself). No other war or peace is forced (docs/DECKS.md, principle
  27): its leanings are attitude modifiers, toward the founder (`ai_attitude_extra`), toward its
  parent when that is another (`parent_attitude`: the Usurper's -6), and toward every civilization
  of an alignment (`Player::alignment_attitude`).
- **The AI's attitude** (`ffh-ai`, `diplomacy::relation_parts`) counts both, at half (they are on
  FFH's scale, where 4 is about a step; the AI's step is 2, as for the Trust spell).
- **City sites** (`world::free_site`, `site_value`, `best_sites`, `land_near_borders`): a free site is
  land a city can stand on (`player_can_found`'s terrain, feature, peak, permanent improvement and
  mana rules), with no unit, no city within `MIN_CITY_RANGE`, unowned; the best are taken by the
  land, bonuses, water and fresh water in their radius.
- **Its own rules** are data where they can be: a civilization's `unit_promotion` (the Fey
  Glamour, the Dawn Vow, the Aifon's Water Walking, the Scaled Kingdom's Swampborn, the Court of
  Mammon's Coin), a civilization trait (`civ_trait`: the Free Companies', the Necropolis's and the
  Court of Mammon's Fallow), a palace's
  `free_building` (the Patrians' Mage Guilds), unique units in its `units` (the Hill Clans' Hill
  Giant and the Brood's Giant Spider are their own versions of classes whose default nobody
  trains), `hills_yield_change` (the Hill Clans). What data cannot say is a rule hooked where it
  lives (section 2), each naming its card: the Necropolis's risen dead (the living units a battle
  on its land kills rise as its Pyre Zombies, there or in its nearest city:
  `engagement/apply.rs`); the Court of Mammon's share of the Infernals' population rules
  (`new_civs_2::infernal_population`, `manes_civ`; section 4).
- **Art:** a new leader shows an existing leader's portrait and a new unit an existing unit's
  sprite (`PORTRAIT_ALIASES` in `crates/ffh-play/assets/app/portraits.js`, `UNIT_ALIASES` in
  `iso.js`).
