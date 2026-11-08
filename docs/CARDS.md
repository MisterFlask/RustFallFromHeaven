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
- **Alignment:** `card_alignment` (`ALIGNMENT_GOOD`, `ALIGNMENT_EVIL`) limits who draws it;
  `card_block_alignment` keeps one alignment from drawing it (a [good or neutral] card blocks
  `ALIGNMENT_EVIL`).
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

**New data keys** (each read in `ffh-data`, described in ffh-play's help):
- techs: `card_block_alignment` (above).
- buildings:
  - `train_units`: a list of `{unit, cost, bonus, national}`. The city trains each unit (one
    without a cost of its own) at `cost`, where it has `bonus`; past `national` units of the
    class, each costs `NATIONAL_UNIT_OVER_LIMIT_COST_PERCENT`. The Beast Market, the Elemental
    Foundry.
  - `free_bonus_choices`: resources. When the building is built, its owner chooses one
    (`Controller::choose_free_bonus`; kept in `City::free_bonus_choices` while the city has the
    building); the city has one of it. Agares's Well.
  - `free_promotion_choices`: promotions. A unit trained in the city takes one it can acquire,
    its owner's choice (`Controller::choose_free_promotion`). The Spire of the Archmagi.
  - `rebirth_range`, `rebirth_promotion`: the first time a unit of the owner would die in a battle
    within the range, it is carried to the city at 10% of its HP and gains the promotion; a unit
    with it dies normally (`engagement/apply.rs`). The Font of Rebirth.
  - `free_unit_turns`, `free_unit_classes`: every N turns after it is built, a free unit of the
    first class that applies (a city founder only while a site within 10 plots is free). Maponos's
    Cradle.
- civics: `unit_cost_modifier`, `unit_combat_cost_modifiers` (Magocracy); `no_war_weariness`
  (Stoicism).
- hurries: `units_only` (Gold Standard's hurry, 2 gold a hammer).
- processes: `production_to_experience`: hammers per point of experience to the units in the city
  (Muster).
- improvements: `claim_range` (the plot and the land plots within it are the builder's territory,
  whatever the culture; built only outside every border), `min_city_distance`. The Trading Post.
- builds: `same_builders_as`: every unit that can do that build can do this one (set in the units'
  `builds` at load). The Trading Post.

**Tiers and counts:** a card's buildings have no tier variants (`building_variants.jsonc` is read
before the card files): an ordinary one is Standard everywhere, a national one rolls a tier for its
cost only. Tests that count a table, or check every entry of it against FFH2's numbers, count the
game's own entries: the first `len() - card_entry_count(table)` ids.

**Records:** each card that changes the rules is listed in docs/PORTING.md ("Changes from
FFH2", the decks entry). Its row in section 5 below gets its status.

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
  mid-game, ...

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
| E11 Pioneers | Engineer | built |
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
