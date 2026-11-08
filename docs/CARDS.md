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

### New data keys

**Battle magic** (the battle cards; `ffh_data::BattleMagic`, `crates/ffh-data/src/infos/battle_magic.rs`),
read the same way on a unit type and a promotion; a unit has its type's and its promotions'
(`GameData::unit_battle_magic`). The rules are `engagement/battle_magic.rs` (fighters, pairs,
illusions, familiars, Contingency's post-battle steps) and the resolver's `formation/magic.rs`;
ffh-play's promotion and unit help describe every key (`magic_lines`).

| Key | What |
|---|---|
| `battle_spell_power` | percent more damage of its damage spells, the magic bolt and Wonder's damage outcomes included |
| `battle_damage_only` | it casts no charm, hex or heal (nor Wonder's other outcomes) |
| `battle_spell_pattern` | its damage spells and bolt hit with this pattern in place of their own |
| `battle_front_caster` | from the front row it casts as many spells as from its best row |
| `battle_hex_ward` | while on the field, the enemy's hexes have no effect on its side, the pre-battle hexes included |
| `battle_end_charms` | while on the field, the enemy side's charms end at the start of each round after the first |
| `battle_presence` | a pre-battle hex of its presence in the line at the start: `GROUNDING` (the enemy's charges are strikes for the battle), `MADNESS` (in round 1 the enemy's fighter with the highest A, not a Sentinel, hits its own side) |
| `battle_revive` | once per battle its heal also brings back the first fighter of its side that fell, at this percent of its HP, in its cell if free, else the rear cell of its lanes; never a Sentinel; the kill is undone |
| `battle_dispel` | each hit it lands ends the target's charms and its dispellable promotions' attack, defence, magic defence and first strikes for the battle |
| `battle_damage_vs_summons` | percent more damage to units with a duration and to illusions |
| `battle_spell` | it knows this battle spell, whatever the spell's prerequisites; one such promotion a unit |
| `strike_spell` | in place of its first strike each round it casts this spell, with half its attack as its magic; one such promotion a unit |
| `opening_cast` | a defender: it casts its damage spell (else the bolt) before round 1; the promotion is then lost |
| `death_recall` | a killing hit takes it out of the battle (no roll, even in a city); it returns to its capital with 1 HP and the promotion is lost; only with a capital |
| `battle_familiar` | (unit) it binds to a caster (its `summoner`, `SPELL_BIND_FAMILIAR`); in a battle it takes the first spell or shot aimed at that caster, and the auto formation puts it in the caster's lane |

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
