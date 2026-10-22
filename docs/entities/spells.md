# Spells

Part of the entity inventory: what each entry does, how to play it, and how far the AI uses it. How to read the entries, the AI grades and the AI machinery they cite are in [docs/ENTITIES.md](../ENTITIES.md).

| Section | Entries | Directed | Valued | Partial | Incidental | Unused | Not an AI choice |
|---|---|---|---|---|---|---|---|
| [Sphere spells (arcane)](#sphere-spells-arcane) | 72 |  | 41 | 17 | 9 | 5 |  |
| [World spells](#world-spells) | 21 | 19 |  | 2 |  |  |  |
| [Unit, hero, religion and civilization spells](#unit-hero-religion-and-civilization-spells) | 47 | 4 | 9 | 7 | 5 | 22 |  |
| [Abilities](#abilities) | 127 | 7 | 10 | 31 | 24 | 55 |  |

## Sphere spells (arcane)

How the AI paths below fit together (ENTITIES.md, "Spells"): at the end of the turn
`casting::end_turn` takes the best spell by `magic::turn_value` (for spells `magic::understood`
models) or `casting::ffh_value` (for the others); only when no castable spell is worth more than 0
there does it fall back to FfH's own choice (`casting::choose_spell`, every spell by
`casting::ffh_value`, where an added promotion is always worth at least 1), which is how "filler"
casts happen. Before a strike (`magic::cast_phases`, from `campaign::strike` and
`military::assault`) every caster within two plots of the target may cast any castable spell but
world spells (abilities included) by `magic::attack_value`, in three phases (`magic::phase`:
Soften, Summon, Bless), each cast worth at least 5 hit points and at least the value of what the
caster would cast at the end of the turn (its reserve). Sphere promotions are chosen by
`units::promotion_value`, which adds the promotion's `iAIWeight` (50/100/150) and
`magic::promotion_value` (the best `magic::spell_worth` of the spells it opens, plus a half or a
quarter of the next level's). The `spell_worth` figures quoted below are computed from the XML by
that function's formula.

### Blaze (`SPELL_BLAZE`)
*Sphere spell, Fire I (with Channeling I) · sets the caster's own forest or jungle tile on fire · no range · `bAllowAI` 0*

**What it does.** Puts Smoke on the caster's tile when it holds a Forest or a Jungle and no
improvement. Smoke (-10% defense) turns into Flames, and Flames spread: each turn a 20% chance to
put Smoke on every adjacent flammable feature, and a 20% chance to burn out into a burnt forest.
It is a Fire I adept's only spell until Fire II opens Fireball.

**Playing it well.** A tool for scorched earth, not combat: walk an adept into the forests around
an enemy city (Ljosalfar, Fellowship of Leaves lands) and set them burning to strip their
production and defense bonuses, or clear jungle without workers. The fire does not respect
borders, so keep it away from your own woods; Spring and Snowfall put fires out.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`). Its
`magic::spell_worth` is 0, so Fire I is taken only for what Fire II opens (a quarter of
Fireball's worth for an adept, `magic::promotion_value`).

### Bless (`SPELL_BLESS`)
*Sphere spell (divine), Channeling II and Divine, caster of the Order · blesses the caster's stack · Confessor, Sphener*

**What it does.** Every living unit in the caster's tile of an adept, archer, beast, disciple,
melee, mounted or recon type that lacks it gains Blessed: +1 holy strength and +20% against
demons. Blessed is dispellable and is removed after the unit's next combat.

**Playing it well.** A free strike buff from the Order's priests: cast it on the attack stack the
turn it attacks, above all against demons (Infernals, Sheaim summons) and against undead, whom
holy damage hurts most. As it is spent by the first fight, there is no point in keeping a stack
blessed between battles; an enemy's Dispel Magic or Revelation takes it away.

**AI: Valued.** `magic::understood` (it adds a promotion), Bless phase (Blessed's `iAIWeight` is
100). Before a strike each attacker in the caster's stack counts its holy point as a share of its
strength, times 50 hit points (`magic::attack_value` through `promo_pct`); at the end of the turn
`magic::ours_worth` counts the same share plus a tenth of the weight, halved because combat
removes it, for every unit lacking it. `magic::caster_target` may walk the caster to a stack that
lacks it. The +20% against demons is not counted (`promo_pct` reads no combat-type bonuses).
`spell_worth` 70.

### Blinding Light (`SPELL_BLINDING_LIGHT`)
*Sphere spell, Sun II · range 1 · enemy units immobile for 2 turns · resistable, +30% · mages with Sun II; the Radiant Guard and Ratha have Sun II from birth*

**What it does.** Every enemy unit within one tile (team and neutral units are immune) that fails
its resist roll cannot move or attack for 2 turns; it still defends. Resistance is 20% plus 5 per
level plus this spell's 30, less the caster's Channeling II and III (10 each) and Metamagic
(5 each), +10 in a city: about half of a fresh enemy stack resists a mage.

**Playing it well.** Freeze the stack that is about to hit your city, or the relief force and
counter-attackers around a city you are besieging, so it cannot retreat or strike back while you
kill it. Its resist chance makes it a spell for big stacks; veterans and garrisons shrug it off.
Units already immobile are wasted targets.

**AI: Valued.** `magic::understood` (`iImmobileTurns`), Soften phase. At the end of the turn every
enemy in range that can attack and is not already immobile is worth a flat 50 hit points times
its chance to fail the roll (`magic::reached_value`); before a strike only enemies off the
target's plot count, 10 each, so it is cast there mainly to pin reinforcements. The flat value
ignores the enemy's strength: a Warrior pinned counts as much as a Dragon. `casting::end_turn`
runs over every unit, so a Radiant Guard casts it too. `spell_worth` 80.

### Bloom (`SPELL_BLOOM`)
*Sphere spell (divine), Channeling II and Divine, caster of the Fellowship of Leaves · New Forest on the caster's tile · inside own borders · takes 2 turns · Priest of Leaves, Yvain*

**What it does.** After two turns in which the caster cannot move, puts a New Forest (+1
production, a quarter of a health point) on its tile: inside our borders, outside cities, on a tile with no
feature and no improvement (unless the civilization keeps forests under improvements). The New
Forest grows into a Forest (+25% defense) and, under a Fellowship of Leaves state religion, into an
Ancient Forest (+1 food, +1 production, +50% defense).

**Playing it well.** For Leaves civilizations and the Ljosalfar, forests are a resource: plant
them on plains and grassland you will not farm, around border cities for defense, and where you
want Treetop Defense positions or a Forest for Summon Treant. Plan the two idle turns; do not
plant on tiles your cities will need for food.

**AI: Incidental.** Not modelled by `magic.rs` (a feature only), so it keeps FfH's terms: +10 for
the feature (`casting::ffh_value`). A Leaves priest outside a city in our borders casts it
wherever it stands whenever nothing else scores more than 10, a future farm included, and stands
still for two turns. Nothing sends a priest anywhere to plant; `spell_worth` 0.

### Blur (`SPELL_BLUR`)
*Sphere spell, Shadow I (with Channeling I) · the caster's stack becomes immune to first strikes and defensive strikes · about half a chance a turn to wear off*

**What it does.** Every unit in the caster's tile of a listed combat type (adept, animal, archer,
beast, disciple, melee, mounted, naval, recon) gains Blur: immune to first strikes and to
defensive strikes. Each turn it has a 51% chance to wear off.

**Playing it well.** Cast on attackers about to hit first-strike defenders (archers in a city,
Stoneskinned or Dance of Blades units, Longbowmen behind walls) so their free first-strike rounds
vanish. It is a short buff: cast it the turn you attack, not in advance.

**AI: Incidental.** `magic::understood`, Bless phase, but `promo_pct` counts the first strikes a
promotion gives, not immunity to them, so its value is 0 both before a strike (below the 5-point
floor) and at the end of the turn. It is cast only as FfH's filler (`casting::choose_spell`:
Blur's weight, 50) when nothing else has a value, typically on a quiet turn when it does nothing.
`spell_worth` 5.

### Charm Person (`SPELL_CHARM_PERSON`)
*Sphere spell, Mind II · range 1 · living enemy units cannot attack · resistable, +20% · mages with Mind II; the Succubus (Mind II from birth)*

**What it does.** Every living enemy unit within one tile that fails its resist roll gains
Charmed: it cannot attack (it still defends and moves). Charmed wears off with a 21% chance each
turn and is dispellable. (Loyalty's help promises 20% more resistance to it; nothing in the
rules applies that.)

**Playing it well.** A defensive spell: when an enemy stack parks next to your city or stack,
charm it so most of it cannot strike next turn, then hit it while it is helpless. It does nothing
against defenders (charmed units defend normally), so it is no siege tool, and undead, golems and
elementals are immune.

**AI: Valued.** `magic::understood`, Soften phase. At the end of the turn every enemy in range that
can attack is worth 60% of its hit points times its chance to fail the roll (Charmed makes it
defensive only, `promo_pct` gives -100%); before a strike it is worth nothing (charmed defenders
still defend), which is right. It is cast at the end of the turn on threatening enemies within one
tile. `spell_worth` 80 makes Mind II a good pick for mages.

### Courage (`SPELL_COURAGE`)
*Sphere spell, Spirit I (with Channeling I) · lasting blessing of the caster's stack · adepts with Spirit I, Corlindale*

**What it does.** Every living unit in the caster's tile (adept, animal, archer, disciple, melee,
mounted, recon) that lacks it gains Courage: immune to fear, and it heals the units on its tile
10% extra a turn. It lasts until dispelled.

**Playing it well.** Cheap insurance for an army facing Spectres, Wraiths and other Fear units
(feared units may refuse to attack them), and a small medic effect for a stack that heals in the
field. Cast it once on the main stack and on garrisons; Hope gives it to units in a city for free.

**AI: Partial.** `magic::understood`, Bless phase, but it adds no strength: worth 0 before a
strike, and at the end of the turn `magic::ours_worth` gives each unit lacking it a tenth of the
weight (5) plus up to 10 for its own damage (the heal). It is lasting, so `magic::caster_target`
walks a Spirit caster to a stack where those values reach 15. The fear immunity, its real point,
is not valued. `spell_worth` 5.

### Crown of Brilliance (`SPELL_CROWN_OF_BRILLANCE`)
*Sphere spell (divine), Channeling III and Divine, caster of the Empyrean · range 1 · 30 holy damage to 60% · the caster gains Crown of Brilliance · Luridus, Chalid Astrakein*

**What it does.** Damages every enemy unit within one tile with 30 holy damage (no further than
60% damage), and the caster gains Crown of Brilliance: it sees invisible units and casts the spell
again at the start of each of its turns, hitting everything around it again, until the crown
wears off (a 26% chance each turn). It cannot be cast while the caster wears the crown.

**Playing it well.** A walking holy aura for the Empyrean's best priest or Chalid: put on the crown
next to an enemy stack or city and it keeps softening every adjacent enemy for several turns, more
so undead (-50% holy resistance) and demons (-25%); angels are immune. It also exposes invisible
units around the caster. Keep the
caster in a strong stack, since it has to stand next to the enemy.

**AI: Partial.** `magic::understood` (damage), Soften phase. Its value is one hit on the enemies
in range (`magic::expected_damage`, at full weight on a strike's target, 60% or 20% on other
enemies at the end of the turn); the crown counts nothing (`ours_worth` of a promotion that
expires is its defense, here 0), so the repeated hits and the sight of invisible units, most of
the spell, go unvalued. Its high priests are reached only by upgrade (`iMinLevel` 6).
`spell_worth` 75.

### Dance of Blades (`SPELL_DANCE_OF_BLADES`)
*Sphere spell, Chaos I (with Channeling I) · +1 first strike to the caster's stack until the next turn · adepts with Chaos I, Loki, Gaelan*

**What it does.** Every unit in the caster's tile of a listed type (adept, animal, archer, beast,
disciple, melee, mounted, recon) that lacks it gains one extra first strike; the promotion always
wears off at the start of the caster's next turn.

**Playing it well.** Cast it the turn a stack attacks, or at the end of the turn on a stack that
expects to be attacked: one first strike is a free combat round for every unit of the stack. It
is the Chaos adept's bread-and-butter spell.

**AI: Valued.** `magic::understood`, Bless phase. Before a strike each attacker counts its first
strike as 10% of strength, worth 5 hit points, so a stack of attackers makes it worth casting; at
the end of the turn `magic::ours_worth` counts the defense it adds through the enemies' turn, in
full where a visible enemy can strike and a quarter elsewhere. It is excluded from
`magic::caster_target`'s lasting blessings, as it expires. `spell_worth` 35.

### Destroy Undead (`SPELL_DESTROY_UNDEAD`)
*Sphere spell, Life II · range 1 · 30 holy damage to every undead unit, no limit · mages with Life II; Arthendain (Life II from birth)*

**What it does.** Python (`spellDestroyUndead`): 30 holy damage, which can kill, to every undead
unit within one tile, ours included. Undead take 50% extra holy damage. An AI caster may cast it
only when an undead unit of a team at war with it stands in range; a human may cast it at any
undead (docs/AI_ONLY_RULES.md, A2).

**Playing it well.** The counter to undead armies (Skeletons, Spectres, Wraiths, Pyre Zombies,
Diseased Corpses, Drowns, Liches) and to barbarian undead from barrows: one mage cast badly hurts a
whole stack of them, and finishes the wounded. Vampires are living units and are not touched. Keep
your own undead away from the caster.

**AI: Valued.** One of the Python spells `magic.rs` models (`Py::DestroyUndead` in
`magic::py_value`): expected holy damage on every undead unit in range, enemies for, ours twice
against, and none at all if a neutral's undead would be hit. It is cast in the Soften phase before
a strike and at the end of the turn when undead enemies are next to the caster. Nothing seeks
undead out for it. `spell_worth` 30.

### Dispel Magic (`SPELL_DISPEL_MAGIC`)
*Sphere spell, Metamagic II · range 2 · strips enemy blessings and our curses · also returns a mana node to raw mana · mages with Metamagic II*

**What it does.** Within two tiles, enemy units lose their dispellable promotions with a positive
weight (Haste, Enchanted Blade, Stoneskin, Bless, Courage...) and every other unit loses its
dispellable curses (Charmed, Withered...). Python (`spellDispelMagic`): if the caster stands on a
mana resource whose improvement is not permanent, the node is removed and the resource becomes raw
mana. It can be cast when either effect has a target.

**Playing it well.** Cast it before a big fight to strip an enemy stack's buffs (Enchanted Blade,
Stoneskin, Shield of Faith) and cure your own units of Charm and Wither; it is also the way to take
a rival's mana node from him: stand on it, dispel it, then build your own node there.

**AI: Partial.** `magic::understood` (`bDispel`), Soften phase: an enemy's dispelled blessing is
worth its strength share of the unit's health (plus 5), a curse lifted from ours its share as
well (`magic::reached_value`). The mana-node part is not valued. When nothing has a value,
FfH's filler (`casting::choose_spell`) rates any dispel at 25 per tile of its area (225), so a
Metamagic II caster with nothing better casts it whenever it is castable, which includes standing
on any non-permanent mana node, its own as well (see notes). `spell_worth` 30.

### Domination (`SPELL_DOMINATION`)
*Sphere spell, Mind III · range 1 · takes control of an enemy unit · resistable, +10% · on a resisted attempt the caster loses Mind III · archmages with Mind III*

**What it does.** Python (`spellDomination`): among the living enemy units within one tile whose
resist chance is at most 20% for an AI caster (95% for a human, docs/AI_ONLY_RULES.md, A2), it
picks the strongest (strength x 10 plus the chance it does not resist); unless that unit resists,
it becomes ours, immobile for a turn. If it resists, the caster loses Mind III.

**Playing it well.** Steal the best unit next to you: a hero, a dragon-slayer, a wounded tier-4
unit. A unit's resistance grows with its level (5% each) and in cities, so aim at fresh high-tier
units in the field, and stack resist reductions (Channeling III, Metamagic) on the caster. A
failure costs the promotion, so it is a gamble with a high-level caster.

**AI: Valued.** One of the Python spells `magic.rs` models (`Py::Domination`): the unit the
Python would pick, its hit points plus a tenth of its attack power, times the chance it does not
resist, plus the spell's `iAIWeight` 50 at the end of the turn. Because that weight raises its
end-of-turn value above its value before a strike, `magic::cast_phases` never casts it before an
attack (a caster casts there only what beats its own end-of-turn best); it is cast at the end of
the turn. The lost Mind III on a failure is not counted as a cost. `spell_worth` 150, the highest
of Mind III.

### Earthquake (`SPELL_EARTHQUAKE`)
*Sphere spell (divine), Channeling III and Divine, caster of the Runes of Kilmorph · range 1 · wrecks buildings and improvements, unfortifies units · declares war · Runekeeper, the Mithril Golem*

**What it does.** Python (`spellEarthquake`): on every tile within one, each building of a city has
a 26% chance to be destroyed (those that always survive conquest are spared), each non-permanent
improvement a 26% chance, and every non-flying unit loses its fortification. Casting it declares
war on the owner of any city or improvement in range. An AI caster may cast it only when every
owned tile in range belongs to a team at war with it (docs/AI_ONLY_RULES.md, A2).

**Playing it well.** A siege opener: stand next to an enemy city and knock out its walls, temples
and production buildings while every fortified defender loses its bonus, then attack the same
turn. It also wrecks the towns and mines around it. Keep it away from neighbours you are at peace
with, since it declares war on them.

**AI: Partial.** One of the Python spells `magic.rs` models (`Py::Earthquake`), but only as a
strike opener: before an attack on a city it is worth 15 hit points per enemy unit there
(fortification lost), in the Soften phase; at the end of the turn it is worth 0, and FfH's filler
has no term for it, so it is never cast then. The buildings and improvements it destroys are not
valued, and `magic::py_value` does not check that the target city is within the caster's one-tile
range (casters up to two plots from the target cast in `magic::cast_phases`), so a caster two
tiles away may spend it on nothing. Its high priests come only by upgrade. `spell_worth` 40.

### Enchant Spellstaff (`SPELL_ENCHANT_SPELLSTAFF`)
*Sphere spell, Enchantment III · the caster gains a Spellstaff · archmages with Enchantment III*

**What it does.** The caster gains the Spellstaff promotion (dispellable), which gives it the
Spellstaff ability: break the staff to lose the promotion and regain the ability to cast this
turn. The Amurites' Wizard starts with one.

**Playing it well.** A second cast in the turn that matters: an archmage with a staff can summon
an elemental and then cast Snowfall or Fireball before the strike. Enchant the staff on a quiet
turn, break it in battle, enchant it again afterwards.

**AI: Incidental.** `magic::understood`, Bless phase. At the end of the turn `magic::ours_worth`
gives it a tenth of the Spellstaff's weight (15), so an archmage casts it once when nothing better
is on offer. The staff is never broken: the Spellstaff ability can be cast only after a cast, and
neither `casting::end_turn` (one cast a unit, then the next unit) nor `magic::cast_phases` (skips
units that have cast) asks for a second spell. `spell_worth` 15.

### Enchanted Blade (`SPELL_ENCHANTED_BLADE`)
*Sphere spell, Enchantment I (with Channeling I) · +20% strength to the melee units of the caster's stack · lasting · adepts with Enchantment I, Bambur*

**What it does.** Every melee unit in the caster's tile that lacks it gains Enchanted Blade: +20%
strength, attacking and defending, until dispelled; it also removes Rusted. (The promotion's
iCombatPercentDefense of 20 is never applied by the rules; its +20% strength already counts on
defense.)

**Playing it well.** The best early sphere spell for a melee army: one Enchantment adept walks with
the stack and blesses each new axeman or champion once, for good. It counters Rust directly. Cast
it on garrisons too, and keep a Metamagic-armed enemy (Dispel Magic) in mind.

**AI: Valued.** `magic::understood`, Bless phase. Before a strike each melee attacker counts 20%
of strength, 10 hit points; at the end of the turn `magic::ours_worth` counts 20% plus a tenth of
the weight (30) for every melee unit lacking it, and `magic::caster_target` walks Enchantment
casters to the melee stacks that lack it. The ffh-ai test
`an_adept_blesses_its_stack_rather_than_hasting_it` checks exactly this choice over Haste.
`spell_worth` 100, so Enchantment I is among an adept's best promotions with Enchantment mana.

### Fair Winds (`SPELL_FAIR_WINDS`)
*Sphere spell, Air I (with Channeling I) · ships in the caster's tile: +1 move, +25% withdrawal · 6% a turn to wear off · adepts with Air I*

**What it does.** Every naval unit in the caster's tile that lacks it gains Fair Winds: one more
move and +25% withdrawal chance, with a 6% chance each turn to wear off. It does nothing for land
units.

**Playing it well.** For sea powers, the Lanun above all: an adept aboard or in port blesses the
fleet before an invasion or a long crossing; it lasts many turns. Otherwise Air I is mostly a step
toward Maelstrom and the Air Elemental.

**AI: Incidental.** `magic::understood`, Bless phase, but `promo_pct` counts neither moves nor
withdrawal and the promotion expires, so `magic.rs` values it at 0. It is cast as FfH's filler
(`casting::choose_spell`, about 160 by the caster's own promotion value of it) when a ship shares
the caster's tile and nothing else has a value. Because it adds a weighted promotion,
`magic::combat_caster` counts an Air I adept as a combat caster, which then marches with land
strike stacks where Fair Winds has no target. `spell_worth` 5.

### Fireball (`SPELL_FIREBALL`)
*Sphere spell, Fire II · summons one Fireball for a turn · mages with Fire II; Meshabber, the Disciple of Acheron, the Son of the Inferno and the Arcane Barge have Fire II from birth*

**What it does.** Summons a temporary Fireball (`UNIT_FIREBALL`): 4 fire strength, flying, it
ignores terrain and first strikes, deals collateral damage (50, up to 4 units, to 30% at most),
and bombards city defenses by 10% a turn. Its death causes no war weariness. Like every temporary
summon it vanishes at the start of the caster's next turn (later with the Summoner trait).

**Playing it well.** The mage's siege tool: cast it next to a stack or a city before your
attackers go in, so its collateral and bombard soften every defender for free; several mages
each turn replace catapults. It cannot take a city by itself and dies against anything strong,
which costs nothing.

**AI: Valued.** `magic.rs` models it (`magic::understood`: a summon): before an attack it is a
Summon-phase cast by a caster next to the target, valued by the fight and collateral it would
deal (`magic::summon_fight`), and the Fireball attacks at once (`magic::summons_attack`); at the
end of the turn it is cast only if the Fireball reaches an enemy it beats at 5% or better
(`magic::summon_value`; it then attacks through `casting::summons_attack`). Casters take Fire I/II
for it (`magic::promotion_value`). The limit is having casters: the AI trains few
(`production::decide` caster step). `spell_worth` 120.

### Flaming Arrows (`SPELL_FLAMING_ARROWS`)
*Sphere spell, Enchantment II · +1 fire strength to the archers of the caster's stack · lasting · mages with Enchantment II*

**What it does.** Every archer unit in the caster's tile that lacks it gains Flaming Arrows: +1
fire strength, until dispelled.

**Playing it well.** Archers are a city's defenders: a mage that walks the garrisons once raises
every Longbowman and Archer for good, and fire damage bites Treants and other fire-vulnerable
units. Less useful for a melee army; pair it with Enchanted Blade in mixed stacks.

**AI: Valued.** As Enchanted Blade, for archers: before a strike each archer attacker counts its
fire point as a share of its strength, at the end of the turn `magic::ours_worth` counts the
share plus a tenth of the weight for each archer lacking it, and `magic::caster_target` walks the
mage to garrisons and stacks of archers without it. `spell_worth` 100, which also lifts
Enchantment I for adepts (a quarter of it).

### Floating Eye (`SPELL_FLOATING_EYE`)
*Sphere spell, Metamagic I · summons a Floating Eye for a turn · adepts with Metamagic I*

**What it does.** Summons a temporary Floating Eye (`UNIT_EYE`): an air unit of strength 0 with an
air range of 4 that sees invisible units and may explore rival territory.

**Playing it well.** Cheap reconnaissance: look into an enemy city or behind a hill before a
strike, and find the invisible units (Shadows, assassins, Mistforms, Hidden units) that threaten
your stacks. It cannot fight.

**AI: Unused.** `magic::understood` (a summon), but every term of its value needs strength: the
fight of `magic::summon_fight` (no attack: no value), FfH's summon term (`casting::summon_value`,
strength x 100 = 0) and `casting::summon_cast` (value above 0). It is never cast. Yet, being a
temporary summon, it makes a Metamagic I adept a combat caster (`magic::combat_caster`) that
marches with strike stacks. `spell_worth` 5.

### Graft Flesh (`SPELL_GRAFT_FLESH`)
*Sphere spell, Body III · summons a Flesh Golem for good · one per caster with Body III · archmages with Body III*

**What it does.** Creates a permanent Flesh Golem (`UNIT_FLESH_GOLEM`, 6/6, a golem), which cannot
move for 2 turns. The rules allow as many Flesh Golems as the player has units with Body III. Other
units can later be sacrificed into it to raise its strength (Add to Flesh Golem).

**Playing it well.** A free tier-3 unit that pays no hammers: every Body III archmage should keep
one. Feed it surplus or obsolete units (Add to Flesh Golem) to grow it into a monster, and keep it
alive, since it is your only one per caster.

**AI: Valued.** `magic::understood` (a summon). A lasting summon is never cast before a strike;
at the end of the turn it is worth its strength x 10 (60) while the player is at war or an enemy
threatens the caster's tile (`magic::summon_value`), so an archmage casts it at war and the golem
joins the army (summons are created with `UNITAI_ATTACK`). `magic::raised` marks Flesh Golems as
units summoned for good, which a strike spends right after its collateral attackers
(`campaign::strike`). Add to Flesh Golem is `bAllowAI` 0, so the AI's golems never grow.
`spell_worth` 90.

### Haste (`SPELL_HASTE`)
*Sphere spell, Body I (with Channeling I) · +1 move to our living units in the caster's tile until the next turn · removes Slow · adepts with Body I; Vampires, Brujah and Losha Valas*

**What it does.** Every living unit of our team in the caster's tile that lacks it gains Hasted:
one more move this turn (it always wears off at the start of the next turn). It also removes Slow.

**Playing it well.** Mobility on the turn it matters: cast it before moving a stack so it can
reach and attack a target one tile farther, escape a counter-attack, or let 1-move units attack
and still retreat into a city. Useless on a turn the stack does not move.

**AI: Incidental.** `magic::understood`, Bless phase, but moves are not strength: `magic.rs`
values it at 0 before a strike and at the end of the turn ("Haste's move adds nothing", docs/AI.md),
except +10 for each of our units it cures of Slow. Since the AI casts after moving, it is cast as
FfH's filler on quiet turns (`casting::choose_spell`, about 110 by the caster's own promotion value
of it); the round-5 Amurite casters spent over 95% of their casts on Haste and Regeneration
(docs/AI_LOG.md, "Casters through a Mage Guild"). Nothing casts it before a move. `spell_worth` 5.

### Hellfire (`SPELL_HELLFIRE`)
*Sphere spell (divine), Channeling III and Divine, caster of the Ashen Veil · creates a Hellfire on the caster's tile · Infernals only · Profane*

**What it does.** Python requirement (`reqHellfire`): the caster's civilization is the Infernals,
it stands outside a city on land with no improvement and no Hellfire within two tiles; an AI
caster may not cast it on its own territory (docs/AI_ONLY_RULES.md, A2). The Hellfire improvement
works like a lair: each turn it may spawn a barbarian Champion (`SpawnUnitType`) at the lair rate
while its area has unowned land. It can be pillaged (10 gold).

**Playing it well.** For the Infernals, a weapon against neighbours: plant Hellfires on enemy land
or the wilds between you and a rival, where the barbarians they spawn harass him. The spawns are
barbarians, so they attack you too: keep Hellfires far from your own cities.

**AI: Incidental.** Not modelled by `magic.rs` (an improvement), so FfH's terms give it +10
(`casting::ffh_value`): a Profane outside its borders casts it wherever it happens to stand,
whenever nothing else scores more, whether that tile is next to an enemy or next to its own
cities. Nothing chooses where to plant it. `spell_worth` 0.

### Hope (`SPELL_HOPE`)
*Sphere spell, Spirit II · creates Hope in the city · lost when the caster leaves · mages with Spirit II; Donal and the Royal Guard (Spirit II from birth)*

**What it does.** Creates the Hope building in the caster's city: +1 happiness, +4 culture, and
Courage for units built there (and units in the city). Like every caster-kept building it is
removed at the city's turn when no unit there can cast the spell (`bRequiresCaster`).

**Playing it well.** Park a Spirit mage in a new or unhappy city: the culture pushes borders out
and the happy face is worth a citizen at the cap. A Royal Guard garrison in the capital keeps one
for free. Leave the caster there; it is gone the turn it walks out.

**AI: Valued.** `magic::understood` (it creates a building): at the end of the turn it is worth a
quarter of the building's generic value (`production::building_value`: culture 4 a point, plus up
to 450 in a city at culture level 1 or less, the face 35 to 180), so about 57 to 197 hit-point
equivalents, a strong cast. `magic::caster_target` walks Spirit casters to our cities that lack
it, and `magic::upkeep_value` keeps the caster there unless twice as much waits elsewhere. The
free Courage is not valued. `spell_worth` 40.

### Inspiration (`SPELL_INSPIRATION`)
*Sphere spell, Mind I (with Channeling I) · creates Inspiration in the city · lost when the caster leaves · adepts with Mind I, Corlindale*

**What it does.** Creates the Inspiration building in the caster's city: +2 research and +1 Great
Sage point a turn, removed when no unit in the city can cast it.

**Playing it well.** A modest research bump for an adept with nothing better to do: park a Mind
adept in the capital or a Great Library city, where the Great Sage points join others. In war
the adept is worth more in the field (Charm Person comes with Mind II).

**AI: Valued.** `magic::understood` (a building): worth a quarter of its generic value at the end
of the turn (2 research at 9 a point and the great person point at 40: about 55), so Mind adepts
cast it in any city that lacks it; `magic::caster_target` sends them to such cities and
`magic::upkeep_value` keeps them there (without it, casters recast Inspiration a hundred times a
game, docs/AI.md "Spellcasting"). `spell_worth` 40.

### Lichdom (`SPELL_LICHDOM`)
*Sphere spell, Death III · the caster becomes a Lich · at most 4 Liches · archmages with Death III*

**What it does.** The caster turns into a Lich (`UNIT_LICH`: 5/5 with +2 death strength, an arcane
unit with the Undead race, national limit 4), keeping its level, experience and promotions; it
cannot move for a turn. Python requirement (`reqLichdom`): not a Flesh Golem, a puppet or a hero,
and fewer than 4 Liches. Undead are immune to death and poison damage and to fear, resist unholy
50%, but take 50% extra holy damage, and are no longer alive.

**Playing it well.** An archmage's last upgrade: +2 death strength and immunity to Wither, Charm
and the death spells of other necromancers. The price is weakness to holy damage (Destroy Undead,
Crown of Brilliance, Bless, holy units) and the loss of every living-only blessing (Haste,
Regeneration, Courage, Valor). Worth it for an evil caster in the field; risky against the Order or
the Empyrean.

**AI: Valued.** `magic::understood` (it adds a promotion and converts). The Undead promotion adds
no strength, but the conversion counts by FfH's term (`casting::ffh_misc_value`: the Lich's power
less the caster's, +300 for an Archmage), so a Death III archmage turns into a Lich at its first end
of turn with nothing worth more. The immunities and the holy weakness are not weighed. Its
`spell_worth` is 0, so Death III is chosen for Summon Wraith, not for this.

### Loyalty (`SPELL_LOYALTY`)
*Sphere spell, Law I (with Channeling I) · our units in the caster's tile cannot be captured · lasting · adepts with Law I, Chalid Astrakein*

**What it does.** Every unit of our team in the caster's tile (adept, animal, archer, beast,
disciple, melee, mounted, recon) that lacks it gains Loyalty until dispelled: immune to capture,
which also stops conversion after a lost fight and betrayal.

**Playing it well.** Insurance for valuable units against capture and conversion effects: heroes,
high-level veterans, units fighting enemies whose combat converts or enslaves. Otherwise Law I is a
step toward the Host of the Einherjar and Valor.

**AI: Partial.** `magic::understood`, Bless phase, but it adds no strength: 0 before a strike, and
at the end of the turn a tenth of the weight (5) for each unit lacking it (`magic::ours_worth`).
As a lasting blessing it draws a Law caster to stacks of three or more units without it
(`magic::caster_target`, 15 at least). What it protects against is not valued. `spell_worth` 5.

### Maelstrom (`SPELL_MAELSTROM`)
*Sphere spell, Air II · range 2 · 15 lightning damage to 30% · hits every unit around but the caster's tile · mages with Air II*

**What it does.** Every unit within two tiles, except those on the caster's own tile, takes about
15 lightning damage (the rules' damage roll less its resistance: twice its defense, twice its
level, a quarter of its city's defense, its lightning resistance), never past 30% damage. No unit
is immune by side: ours and neutrals are hit, and damaging a neutral unit on its own land declares
war. Bronze and Iron Weapons carry -25% lightning resistance.

**Playing it well.** The widest area spell before Snowfall: from two tiles away it softens an
entire enemy stack or a city garrison at once, and several mages stack its 30% cap. Pull your own
units onto the caster's tile or out of range first, and do not cast it near a neighbour's units.

**AI: Valued.** `magic::understood` (damage), Soften phase: the expected damage on each unit in
range (`magic::expected_damage`, the rules' formula), at full weight on a strike's target and 40%
on other enemies, 60% or 20% at the end of the turn (enemies that can attack us or not), our own
units twice against, and never cast when a neutral unit would be hurt (`magic::reached_value`
returns nothing). `spell_worth` 45.

### Mutation (`SPELL_MUTATION`)
*Sphere spell, Chaos II · random mutations for the living units of the caster's stack · once per unit · mages with Chaos II*

**What it does.** Every living unit in the caster's tile (listed combat types) that has not been
mutated gains Mutated, and with it each of 23 mutation promotions with a 9% chance: good ones
(Combat I, Blitz, Strong, Heroic Strength and Defense, Mobility I, Stoneskin, Regeneration,
resistances, Immune to Disease, Amphibious, Sentry, Empower I...), bad ones (Weak, Diseased,
Withered, Vulnerable to Fire) and trade-offs (Heavy, Light).

**Playing it well.** A gamble with a positive expectation: mutate cheap units and fresh stacks
before a war, not the hero you cannot afford to see Weak or Withered. Each unit can only be
mutated once, so do it early in a unit's life.

**AI: Incidental.** `magic::understood`, but Mutated has neither strength nor weight, so
`magic.rs` values it at 0 everywhere. FfH's filler gives it 1 (the floor of the caster's promotion
value), so it is cast only when no other castable spell has a value, which for a Chaos caster
means a turn when Dance of Blades cannot be cast (a second Chaos caster in a stack already
dancing). The random promotions are never weighed. `spell_worth` 0.

### Poisoned Blade (`SPELL_POISONED_BLADE`)
*Sphere spell, Nature II · +1 poison strength to the recon units of the caster's stack · lasting · mages with Nature II*

**What it does.** Every recon unit in the caster's tile that lacks it (not neutral units) gains
Poisoned Blade: +1 poison strength, for good (it is not dispellable).

**Playing it well.** Recon lines (Hunters, Rangers, Assassins, Beastmasters) get a permanent
damage-type point, a large share of their strength; bless them once when they come out. Undead,
golems and elementals are immune to poison, so it is wasted against them.

**AI: Valued.** As Flaming Arrows, for recon units: the poison point as a share of strength before
a strike (attackers) and at the end of the turn (units lacking it, plus a tenth of the weight),
and `magic::caster_target` walks the caster to recon units without it. The AI's recon units are
mostly explorers, so targets are few. `spell_worth` 100.

### Raise Skeleton (`SPELL_RAISE_SKELETON`)
*Sphere spell, Death I (with Channeling I) · a Skeleton for good · one per unit with Death I and Channeling I · three on a Graveyard · adepts with Death I; Vampires, Death Knights*

**What it does.** Creates a permanent Skeleton (`UNIT_SKELETON`, 2/2 with +1 death strength,
undead), which cannot move for 2 turns. The rules allow as many Skeletons as the player has units
with Death I and Channeling I. Python (`spellRaiseSkeleton`): cast on a Graveyard, it removes the
Graveyard and raises two more Skeletons.

**Playing it well.** Free fodder: every Death adept, Vampire and Death Knight keeps a Skeleton,
which soaks first attacks in front of the stack and throws itself at city defenders before the
real attackers. Raise them on Graveyards for three at once. They upkeep like any unit.

**AI: Valued.** `magic::understood` (a summon). Never cast before a strike (a lasting summon); at
the end of the turn worth 30 (strength x 10) while at war or with an enemy threatening the caster
(`magic::summon_value`), so Death casters keep their Skeleton at war. Skeletons have their own rule
in a strike: `magic::raised` marks them, and `campaign::strike` sends them in right after the
collateral attackers (docs/AI.md, "Summons and skeletons": in two replays, 15 and 10 skeleton
attacks where the old order made 1 and 8). Graveyards are not sought out. `spell_worth` 45.

### Regeneration (`SPELL_REGENERATION`)
*Sphere spell, Body II · faster healing for the living units of the caster's stack · gone when healed · mages with Body II*

**What it does.** Every living unit in the caster's tile (listed combat types) that lacks it
gains Regeneration: it heals even while moving and 10% extra a turn wherever it is, and loses the
promotion once fully healed.

**Playing it well.** Keep a wounded army moving: cast it after a battle so the stack heals while
marching to the next city instead of sitting for turns. Pointless on healthy units.

**AI: Valued.** `magic::understood`, Bless phase (worth 0 before a strike, as it adds no
strength). At the end of the turn `magic::ours_worth` counts, for each damaged unit lacking it, the
healing it brings (up to 10) plus a tenth of the weight; on an unhurt unit it is worth 0.
It is not a lasting blessing for `magic::caster_target`. `magic.rs` casts it where there are wounds;
on a quiet turn it can still be FfH's filler (the caster's promotion value of it, about 50), as for
the round-5 Amurite casters that cast it most turns (docs/AI_LOG.md). `spell_worth` 5.

### Repair (`SPELL_REPAIR`)
*Ability (sphere-gated), Enchantment I and Dwarven · heals golems, ships and siege engines in the tile by 20 · Bambur, Khazad and Luchuirp casters with Enchantment I*

**What it does.** Python (`spellRepair`, `reqRepair`): every golem, naval and siege unit in the
caster's tile has 20 damage repaired; it can be cast when one of them is damaged. As an ability it
is cast even where magic is forbidden.

**Playing it well.** For the Luchuirp above all, whose army is golems that heal slowly: a Dwarven
Enchantment adept with the golem stack puts them back into the fight every turn. Also mends
catapults and ships between sieges.

**AI: Unused.** An ability (`bAbility`), so `magic.rs` does not model it, and FfH's terms
(`casting::ffh_value`) have nothing for a Python heal: its value is 0, and its `iAIWeightWar` 50
is not read. No AI caster ever repairs. `spell_worth` 0.

### Resurrection (`SPELL_RESSURECTION`)
*Sphere spell, Life III · brings back the civilization's dead hero · takes 7 turns · archmages with Life III*

**What it does.** Python (`spellRessurection`, `reqRessurection`): after 7 turns in which the caster
cannot move, recreates the civilization's hero (Donal, Govannon, Loki, Losha Valas, Hyborem... one
for each of 18 civilizations) on its tile, without equipment. It can be cast only when that hero
has been built and is dead everywhere, and no other unit of ours is already casting it.

**Playing it well.** A dead tier-4 hero is a huge loss; an archmage with Life III gets it back for
7 turns of inaction. Cast it in a safe city, since the caster is helpless for the whole delay.

**AI: Partial.** Not modelled by `magic.rs` (Python only); FfH's terms are its `iAIWeight` 300, so
a Life III caster casts it at the first end of turn it is castable, wherever it stands, even next
to enemies, and is frozen for 7 turns. Its `spell_worth` is 0, so Life III is never chosen for it.
While it is castable its 300 is the caster's end-of-turn reserve, so that caster casts nothing
before strikes (`magic::cast_phases`).

### Resurrection (`SPELL_RESSURECTION_GRAVEYARD`)
*Sphere spell, Life III · raises a Champion from a Graveyard on the caster's tile · archmages with Life III*

**What it does.** Python (`spellRessurectionGraveyard`): creates the civilization's Champion with
0 to 29 experience and removes the Graveyard; no delay. (Its help text repeats the hero line of
the other Resurrection; the Python makes a Champion.)

**Playing it well.** Graveyards are rare (a random event places them); a Life archmage turns one
into a free tier-3 unit with some experience. The alternatives on the same tile: Sanctify makes a
Host of the Einherjar and lowers the Armageddon counter, Raise Skeleton makes three Skeletons.

**AI: Incidental.** FfH's terms only: its `iAIWeight` 300, so it is cast whenever a Life III caster
happens to stand on a Graveyard; nothing sends a caster to one (`magic::caster_target` has no such
target). `spell_worth` 0.

### Revelation (`SPELL_REVELATION`)
*Sphere spell (divine), Channeling II and Divine, caster of the Empyrean · range 3 · dispels, unmasks invisible and hidden units, destroys illusions · Vicar, Chalid Astrakein*

**What it does.** Within three tiles it dispels like Dispel Magic (enemies lose their blessings,
everyone else their curses) and, in Python (`spellRevelation`), every unit of another team loses
Hidden, Hidden Nationality and Invisible, and illusions of other teams die. It can be cast only
when such a hidden, invisible or illusory unit is in range.

**Playing it well.** The answer to the Svartalfar, the Sidar, Mistforms, assassins and illusion
armies: reveal them so your army can attack them, and kill the illusions outright. One Vicar with
the main stack is enough.

**AI: Partial.** `magic::understood` for its dispel only: dispelled enemy blessings and cured
curses count (`magic::reached_value`), Soften phase; the unmasking and the illusions it kills are
not modelled. When nothing else has a value, FfH's filler rates any dispel at 25 per tile of its
area (400), so it is cast whenever castable on otherwise quiet turns. `spell_worth` 30.

### Ring of Flames (`SPELL_RING_OF_FLAMES`)
*Sphere spell (divine), Channeling II and Divine, caster of the Ashen Veil · range 1 · 15 fire damage to 40% · sets nearby forests on fire · Ritualist, Profane*

**What it does.** Every unit within one tile but the caster's own takes about 15 fire damage, never
past 40%; no side is immune, so ours and neutrals burn too (a neutral unit hurt on its own land
means war). Python (`spellRingofFlames`): each adjacent forest, jungle or new forest without a
permanent improvement has a 21% chance to catch fire.

**Playing it well.** The Ashen Veil priest's softening spell: step next to an enemy stack or city
and burn every adjacent defender before the attack. The forest fires can spread into your own
lands, and it hurts your own units around the caster: keep the stack on the caster's tile.

**AI: Valued.** As Maelstrom: `magic::expected_damage` on every unit in range (the rules' formula,
fire resistance included, to the 40% cap), Soften phase, our units twice against, never when a
neutral would be hurt. The forest fires are not counted. `spell_worth` 30.

### Rust (`SPELL_RUST`)
*Sphere spell, Entropy I (with Channeling I) · range 1 · enemy melee units -10% strength, and every enemy loses its bronze, iron and mithril weapons · resistable, +20% · adepts with Entropy I, Mardero*

**What it does.** Each enemy unit within one tile (team and neutral units are immune) that fails
its resist roll gains Rusted if it is a melee unit (-10% strength, removed in a city with a Forge
and by Enchanted Blade) and loses its Bronze, Iron and Mithril Weapons promotions (+1, +2 and +4
strength): each effect rolls the resist separately.

**Playing it well.** The best anti-army spell of the first tier: an iron-armed axeman or champion
loses 2 of its 5 or 8 strength, more with mithril, before your attack. Cast it on the city or stack
you are about to hit; the weapons come back only when the unit returns to one of its own cities
that has the metal, so an army in the field stays disarmed. Useless against beasts and units
without weapons.

**AI: Valued.** `magic::understood` (it adds and removes promotions), Soften phase. Before a strike
each defender on the target counts its lost weapons and Rusted as their share of its defense, times
its health and its chance to fail the roll (`magic::reached_value`); at the end of the turn the same
on the attack strength of enemies in range that can strike. `spell_worth` 100 (a ffh-ai test holds it
at 60 or more), so Entropy I is among an adept's best promotions with Entropy mana.

### Sanctify (`SPELL_SANCTIFY`)
*Sphere spell, Life I (with Channeling I) · cleanses hell terrain around the caster, or removes City Ruins or a Graveyard to lower the Armageddon counter · adepts with Life I; Devout, Sphener, Yvain*

**What it does.** Python (`spellSanctify`, `reqSanctify`): on City Ruins it removes them and lowers
the Armageddon counter by 1; on a Graveyard it does the same and raises a Host of the Einherjar
for us; and it turns hell terrain within one tile back (the plot counters reset, or, with the No
Plot Counter option, Broken Lands to grassland, Burning Sands to desert, Fields of Perdition to
plains, Shallows to marsh). An AI caster needs the tile south-east of it to be its own (a Python
quirk) and may not be Infernal (docs/AI_ONLY_RULES.md, A2).

**Playing it well.** The good civilizations' brake on Armageddon: sweep ruins and graveyards with a
Life adept, and clean the hell terrain the Ashen Veil spreads around your cities before it ruins
their tiles. Graveyards also give a free Einherjar.

**AI: Unused.** Python only and not one of the spells `magic.rs` models, so FfH's terms apply, and
they have nothing for it: its value is 0, and it is never cast. `spell_worth` 0; Life I is taken
only for Destroy Undead behind it.

### Scorch (`SPELL_SCORCH`)
*Sphere spell, Sun I (with Channeling I) · plains to desert, Fields of Perdition to Burning Sands, snow to tundra, on the caster's tile · declares war on the tile's owner · adepts with Sun I, Chalid Astrakein*

**What it does.** Python (`spellScorch`): the caster's tile changes from plains to desert, Fields of
Perdition to Burning Sands, or snow to tundra, and casting it on another player's tile declares
war on him. An AI caster may scorch plains only on land not its own, and snow only on its own land
and unless it is the Doviello or the Illians (docs/AI_ONLY_RULES.md, A2).

**Playing it well.** Two uses: thaw your own snow into tundra (a food), or burn an enemy's farmland
into desert in a war, tile by tile, to starve his cities. The Doviello and Illians want their
snow.

**AI: Partial.** One of the Python spells `magic.rs` models (`Py::Scorch`), but only for thawing:
`magic::terraform` turns our snow into tundra (not for the Doviello and the Illians), worth a
quarter of the yield gain as a turn of building value on a tile one of our cities can work
(`magic::terraform_value`), and `magic::caster_target` walks a Sun adept to such tiles. Scorching
an enemy's plains is never valued. `spell_worth` 20.

### Shadowwalk (`SPELL_SHADOWWALK`)
*Sphere spell, Shadow II · the caster's stack ignores city building defense when attacking, immune to defensive strikes · a quarter chance a turn to wear off · mages with Shadow II*

**What it does.** Every unit in the caster's tile of a listed type (adept, animal, archer, beast,
disciple, melee, mounted, recon) that lacks it gains Shadowwalk: it ignores the defense of walls
and other buildings when it attacks a city, and is immune to defensive strikes. It has a 26%
chance to wear off each turn.

**Playing it well.** A wall-breaker without catapults: cast on the attack stack next to a
fortified city and the walls (and Wall of Stone) stop counting for every attacker. It does not
remove the defenders' own fortification bonus, so pair it with Earthquake or siege.

**AI: Valued.** `magic::understood`, Bless phase. Before a strike on a city each attacker counts the
building defense it would ignore (`promo_pct` with `bIgnoreBuildingDefense`: the city's defense
with buildings less without) times 50, often the best blessing against a walled city; at the end of
the turn it is worth 0 (it expires and adds no defense), so off strikes it is only FfH's filler.
`spell_worth` 15, which cannot see building defense, so Shadow II is rarely chosen for it.

### Shield of Faith (`SPELL_SHIELD_OF_FAITH`)
*Sphere spell (divine), Channeling II and Divine, caster of the Runes of Kilmorph · +10% strength to the caster's stack · lasting · Stonewarden, Runekeeper*

**What it does.** Every unit in the caster's tile (adept, archer, disciple, melee, mounted, recon)
that lacks it gains Shield of Faith: +10% strength, attacking and defending, until dispelled. (Its
iCombatPercentDefense of 30 is never applied by the rules.)

**Playing it well.** A Kilmorph priest's permanent blessing: bless every garrison and the main
army once. Ten percent is modest but free and lasting; stack it with Enchanted Blade on melee.

**AI: Valued.** `magic::understood`, Bless phase: before a strike 10% for each attacker (5 hit
points), at the end of the turn `magic::ours_worth` for every unit lacking it, and
`magic::caster_target` walks the priest to garrisons and stacks without it. It is overvalued:
`promo_pct` reads the 30% defense field the rules never apply, so at the end of the turn every unit
counts 30% instead of 10% (a field unit takes the better of attack and defense), and `spell_worth`
(145) reads it too.

### Slow (`SPELL_SLOW`)
*Sphere spell, Ice I (with Channeling I) · range 1 · enemy units -1 move · resistable · adepts with Ice I; the Priest and High Priest of Winter*

**What it does.** Each enemy unit within one tile that fails its resist roll gains Slow (-1 move,
a 31% chance each turn to wear off) and loses Hasted. A one-move unit slowed cannot move or attack
until it wears off.

**Playing it well.** Pins slow enemies next to you: a slowed axeman stack cannot reach your city
next turn or escape your counter-attack. Weak against cavalry and two-move units, which only lose
half their reach.

**AI: Valued.** `magic::understood`, Soften phase. At the end of the turn every one-move enemy in
range that can attack is worth 30 hit points times its chance to fail the roll (stopped until it
wears off); faster enemies, and every target before a strike, are worth nothing, since moves are
not strength. It is therefore cast at the end of the turn, against slow attackers next to the
caster. `spell_worth` 60.

### Snowfall (`SPELL_SNOWFALL`)
*Sphere spell, Ice III · range 1 · 40 cold damage to 80% · turns the land around to snow for 3 to 8 turns · archmages with Ice III*

**What it does.** Every unit within one tile but the caster's own takes about 40 cold damage,
never past 80%; no side is immune (a neutral hurt on its own land means war). Python
(`spellSnowfall`): every land tile within one, the caster's included, that is not snow becomes snow
for 3 to 8 turns, and fires there go out.

**Playing it well.** The strongest damage spell of the spheres: next to an enemy city it nearly
kills every defender, and the temporary snow strips the yields of the tiles around it for several
turns. Keep your own units on the caster's tile, and remember it freezes your own tiles too.

**AI: Valued.** As Maelstrom: `magic::expected_damage` on every unit in range (to the 80% cap),
Soften phase, our units twice against, never when a neutral would be hurt; with the largest damage
of any sphere spell it usually wins the Soften phase. The snow is not counted. `spell_worth` 80.

### Spring (`SPELL_SPRING`)
*Sphere spell, Water I (with Channeling I) · desert to plains on the caster's tile · puts out fires around · adepts with Water I*

**What it does.** Python (`spellSpring`): the caster's tile turns from desert to plains (not on
flood plains; scrub is removed), and flames and smoke within one tile go out. It can be cast on a
desert, or anywhere a fire burns nearby. An AI caster may turn only its own desert, and the
Infernals never cast it (docs/AI_ONLY_RULES.md, A2).

**Playing it well.** Terraforming for desert starts: every desert tile a city works becomes a
1-food 1-production plains. Also the fire brigade against Blaze, Ring of Flames and forest fires.

**AI: Valued.** One of the Python spells `magic.rs` models (`Py::Spring`): on our desert tiles a
city can work it is worth a quarter of the yield gain as a turn of building value (about 87 on a
worked tile, 52 otherwise, `magic::terraform_value`), and `magic::caster_target` walks Water adepts
to such tiles, one each. Putting out fires is not valued. `spell_worth` 20.

### Stoneskin (`SPELL_STONESKIN`)
*Sphere spell, Earth II · the caster gains Stoneskin: +2 defense, +3 first strikes, 50% elemental resistance · removed after combat · mages with Earth II*

**What it does.** The caster gains Stoneskin: +2 defense strength, three first strikes, and 50%
resistance to cold, death, fire, holy, lightning, poison and unholy damage, until its next combat.

**Playing it well.** Self-protection for the mage that walks with the army: it makes the caster a
far harder target for counter-attacks and enemy spells. Recast it after every fight.

**AI: Valued.** `magic::understood`, Bless phase (counted before a strike only when the caster
itself attacks). At the end of the turn `magic::ours_worth` counts the better of its attack and
defense share (the 2 defense and the first strikes) plus a tenth of the weight, halved because
combat removes it: about 45 for a Mage, so a caster that lacks it recasts it whenever nothing better
is on offer. The resistances are not counted. `spell_worth` 80.

### Summon Air Elemental (`SPELL_SUMMON_AIR_ELEMENTAL`)
*Sphere spell, Air III · summons one Air Elemental for a turn · archmages with Air III*

**What it does.** Summons a temporary Air Elemental (`UNIT_AIR_ELEMENTAL`): 5 strength and +2
lightning, 2 moves, flying, immune to lightning, +1 lightning strength per Air mana its owner
has, and it creates a Lightning Elemental when it wins a fight. It vanishes at the start of the
caster's next turn.

**Playing it well.** A fast striker that crosses water and terrain: it reaches targets two tiles
away and each win leaves a Lightning Elemental to strike again. Strong with several Air nodes.

**AI: Valued.** As Fireball: a Summon-phase cast by a caster next to a strike's target, valued by
the fight `magic::summon_fight` estimates from its base 7 strength, and at the end of the turn only
when it reaches an enemy it beats at 5% or better. The mana affinity and the Lightning Elementals
are not counted. `spell_worth` 140.

### Summon Aurealis (`SPELL_SUMMON_AUREALIS`)
*Sphere spell, Sun III · summons one Aurealis for a turn · archmages with Sun III*

**What it does.** Summons a temporary Aurealis (`UNIT_AUREALIS`): 6 strength, 1 move, an elemental,
with +2 fire strength per Sun mana its owner has (the only summon with affinity 2).

**Playing it well.** Weak without Sun mana and excellent with it: a Sun-node empire turns it into
one of the strongest summons. It must be cast next to its target, having one move.

**AI: Partial.** As Fireball, but `magic::summon_fight` reads only the base 6 strength, so the Sun
affinity, the reason to cast it, is not seen: the AI casts it as a 6-strength unit (only next to the
target before a strike, at the end of the turn on an enemy it beats at 5% or better).
`spell_worth` 120.

### Summon Balor (`SPELL_SUMMON_BALOR`)
*Sphere spell (divine), Channeling III and Divine, caster of the Ashen Veil · summons one Balor for a turn · Profane*

**What it does.** Summons a temporary Balor (`UNIT_BALOR`): 7/3 with +4 unholy strength, a demon,
resistant to fire, with Stigmata (stronger as the Armageddon counter rises).

**Playing it well.** The Ashen Veil's heaviest hitter: 11 attack strength every turn for free, more
late in an Armageddon game. Cast it next to the city or stack to hit; holy damage and demon-slaying
units are its bane.

**AI: Valued.** As Fireball, from its base 11 attack strength (Stigmata not counted): a Summon-phase
cast next to a strike's target, and at the end of the turn when it reaches an enemy it beats at 5%
or better. Its high priests come only by upgrade. `spell_worth` 200, the cap.

### Summon Djinn (`SPELL_SUMMON_DJINN`)
*Sphere spell, Metamagic III · summons one Djinn for a turn · archmages with Metamagic III*

**What it does.** Summons a temporary Djinn (`UNIT_DJINN`): 1 strength, 2 moves, an elemental,
with +1 strength for every mana resource of every kind its owner has (as fire, cold, lightning,
death or unholy strength for the mana with a damage type).

**Playing it well.** Worthless to a small empire and the strongest summon of a mana-rich one: count
your mana before taking Metamagic III for it. Its two moves let it strike from a safe distance.

**AI: Partial.** As Fireball, but `magic::summon_fight` reads the Djinn's base strength of 1, so it
almost never finds a fight at 5% and `magic.rs` almost never casts it, however much mana the
player has. FfH's filler casts it (strength x 100) when an enemy is within its two moves and nothing
else has a value; it then attacks only at 5% real odds (`casting::summons_attack`). `spell_worth` 20,
the lowest of the third-tier summons.

### Summon Earth Elemental (`SPELL_SUMMON_EARTH_ELEMENTAL`)
*Sphere spell, Earth III · summons one Earth Elemental for a turn · archmages with Earth III*

**What it does.** Summons a temporary Earth Elemental (`UNIT_EARTH_ELEMENTAL`): 11/11, 1 move, an
elemental, +1 strength per Earth mana.

**Playing it well.** The heavy siege summon: 11 strength attacks a city's best defender every turn
for free. It has one move, so the archmage must stand next to the target; protect it there.

**AI: Valued.** As Fireball, from its base 11 strength: a Summon-phase cast by an archmage next to a
strike's target, and at the end of the turn when it reaches an enemy it beats at 5% or better. The
Earth affinity is not counted. `spell_worth` 200, the cap.

### Summon Fire Elemental (`SPELL_SUMMON_FIRE_ELEMENTAL`)
*Sphere spell, Fire III · summons one Fire Elemental for a turn · archmages with Fire III, the Son of the Inferno*

**What it does.** Summons a temporary Fire Elemental (`UNIT_FIRE_ELEMENTAL`): 6 strength and +3
fire, 2 moves, collateral damage (50, up to 4 units, to 30%), immune to fire, +1 fire strength per
Fire mana; it cannot enter ice.

**Playing it well.** The Fireball's big brother: a 9-strength attacker with collateral that softens
a whole stack, from two tiles away. Fire-immune, fire-resistant and elemental defenders blunt it.

**AI: Valued.** As Fireball, from its base 9 strength, with its collateral damage counted on up to
four other defenders (`magic::summon_fight`). docs/AI.md ("Summons and skeletons"): once summons
attacked what they reach this turn, the Fire Elementals cast by FfH's rule that attacked rose from
19% to 59%. `spell_worth` 200.

### Summon Host of the Einherjar (`SPELL_SUMMON_EINHERJAR`)
*Sphere spell, Law II · summons one Host of the Einherjar for a turn · mages with Law II*

**What it does.** Summons a temporary Host of the Einherjar (`UNIT_EINHERJAR`): 3/3 with +2 holy, an
angel and a disciple unit; each fight it wins adds a turn to its life (`iDurationFromCombat`). It
can use the Sacrifice ability (not for the AI).

**Playing it well.** Cast it against weak units it can chain-kill: every win keeps it another turn,
so a Host fed on workers, scouts and wounded units lasts. Its holy strength bites hardest on undead
(-50% holy resistance) and demons (-25%).

**AI: Valued.** As Fireball, from its base 5 strength: a Summon-phase cast next to a strike's target,
and at the end of the turn when it reaches an enemy it beats at 5% or better. The extra turns it
earns are not counted (a temporary summon is valued as one fight). `spell_worth` 100.

### Summon Ice Elemental (`SPELL_SUMMON_ICE_ELEMENTAL`)
*Sphere spell, Ice II · summons one Ice Elemental for a turn · mages with Ice II*

**What it does.** Summons a temporary Ice Elemental (`UNIT_ICE_ELEMENTAL`): 2 strength and +2
cold, 1 move, +10% on tundra and +20% on snow, immune to cold, vulnerable to fire, +1 cold strength
per Ice mana; it cannot enter desert or Burning Sands.

**Playing it well.** Weak without Ice mana, decent with two or three nodes and on snow; mostly a step
toward Snowfall. Use it to finish wounded units next to the mage.

**AI: Valued.** As Fireball, from its base 4 strength, so it is cast only where it finds a weak or
wounded target (5% at least); terrain and affinity bonuses are not counted. `spell_worth` 80.

### Summon Kraken (`SPELL_SUMMON_KRAKEN`)
*Sphere spell (divine), Channeling III and Divine, caster of the Octopus Overlords · a Kraken for good · next to water · one per such caster · Speaker*

**What it does.** Creates a permanent Kraken (`UNIT_KRAKEN`): a 17/17 sea unit with 4 moves and
stealth (the Hide ability). The caster must stand next to water; the rules allow as many Krakens as
the player has units able to cast it.

**Playing it well.** A free capital ship that out-fights anything afloat in its era: use it to sink
fleets and transports, hold sea lanes and escort invasions. Losing it costs a high priest's whole
investment, so do not throw it at cities.

**AI: Partial.** `magic::understood` (a summon): worth its strength x 10 (170) at the end of the
turn while at war or with an enemy near the caster (`magic::summon_value`), so a Speaker summons it.
The Kraken is then a sea unit with an attack AI, which `units::role_of` puts in the Naval role,
and that role holds (there is no naval combat logic): it waits where it was created.
`spell_worth` 200.

### Summon Mistform (`SPELL_SUMMON_MISTFORM`)
*Sphere spell, Shadow III · summons one Mistform for a turn · archmages with Shadow III*

**What it does.** Summons a temporary Mistform (`UNIT_MISTFORM`): 6/6, 2 moves, an elemental,
invisible and of hidden nationality (it can attack units of players we are at peace with), and it
ignores city building defense.

**Playing it well.** An invisible assassin: strike wounded units, workers and walled cities without
being seen, or hit a rival you are not at war with without declaring war. Revelation and units that
see invisible expose it.

**AI: Valued.** As Fireball, from its base 6 strength: cast next to a strike's target or at the end
of the turn on an enemy it beats at 5% or better. Only enemies at war are targets
(`magic::summon_fight`, `casting::summon_target`), so the hidden nationality and the ignored walls
go unused or unvalued. `spell_worth` 120.

### Summon Pit Beast (`SPELL_SUMMON_PIT_BEAST`)
*Sphere spell, Entropy II · summons one Pit Beast for a turn · mages with Entropy II*

**What it does.** Summons a temporary Pit Beast (`UNIT_PIT_BEAST`): 4/4 with +1 unholy, a demon and a
beast unit; each fight it wins adds a turn to its life.

**Playing it well.** As the Einherjar for evil mages: feed it weak targets to keep it alive. Holy
damage and demon slayers kill it.

**AI: Valued.** As Fireball, from its base 5 strength; the turns it earns are not counted.
`spell_worth` 100, which also lifts Entropy I (a quarter of it).

### Summon Sand Lion (`SPELL_SUMMON_SAND_LION`)
*Civilization spell of the spheres, Channeling II, Malakim arcane units · summons one Sand Lion for a turn · on desert or Burning Sands only*

**What it does.** Python requirement (`reqSandLion`): the caster stands on desert or Burning Sands.
Summons a temporary Sand Lion (`UNIT_SAND_LION`): 5/5 with +1 fire, 3 moves, +40% on desert, sees
stealth, an elemental with Nomad.

**Playing it well.** The Malakim mage's own summon, with no sphere needed: 3 moves and the desert
bonus make it a raider that hits from far and deep in the Malakim's deserts. Position mages on
desert tiles near the front.

**AI: Valued.** As Fireball, from its base 6 strength, when a Malakim mage happens to stand on
desert or Burning Sands; nothing moves mages to deserts for it, and the desert bonus is not counted.
`spell_worth` 120.

### Summon Spectre (`SPELL_SUMMON_SPECTRE`)
*Sphere spell, Death II · summons one Spectre for a turn · mages with Death II; Vampires and Death Knights can learn it*

**What it does.** Summons a temporary Spectre (`UNIT_SPECTRE`): 3/3, 2 moves, undead, with Fear
(living units may refuse to attack it).

**Playing it well.** A weak but quick striker for finishing wounded units and workers up to two
tiles away; its Fear and undead immunities make it awkward to kill. Better as a step toward the
Wraith.

**AI: Valued.** As Fireball, from its base 3 strength. docs/AI.md ("Summons and skeletons"): before
summons attacked what they reach this turn, the Spectres FfH's rule cast for an enemy two plots away
never attacked (84 of 84); now `magic::summon_value` counts a fight two plots away, and an ffh-ai
test has a Spectre walk two plots to a wounded Warrior. `spell_worth` 60.

### Summon Tiger (`SPELL_SUMMON_TIGER`)
*Sphere spell (divine), Channeling II and Divine, caster of the Fellowship of Leaves · a Tiger for good · one per such caster · Priest of Leaves, Yvain*

**What it does.** Creates a permanent Tiger (`UNIT_TIGER`): a 4/4 animal with 2 moves, -25% city
attack and no defensive bonuses; it cannot move for 2 turns. As many Tigers as units able to cast it.

**Playing it well.** A free early fast unit for every Leaves priest: escort and picket duty,
barbarian hunting, finishing wounded units. Weak in cities and against walls.

**AI: Valued.** `magic::understood` (a summon): worth its strength x 10 (40) at the end of the turn
while at war or with an enemy near (`magic::summon_value`), never before a strike; the Tiger joins the
army (summoned with `UNITAI_ATTACK`) and `campaign::strike` spends it early as a unit summoned for
good (`magic::raised`). `spell_worth` 60.

### Summon Treant (`SPELL_SUMMON_TREANT`)
*Sphere spell (divine), Channeling III and Divine, caster of the Fellowship of Leaves · a Treant for good · in a Forest or Ancient Forest · one per such caster · High Priest of Leaves, Yvain*

**What it does.** Creates a permanent Treant (`UNIT_TREANT`): 10/10, 1 move, an elemental with
Woodsman I and II, vulnerable to fire; the caster must stand in a Forest or an Ancient Forest.

**Playing it well.** A free tier-4 defender and attacker for forest empires: Treants hold forest
chokepoints and city tiles with forests, and Woodsman makes them fight best in woods. Keep fire
mages away from them.

**AI: Valued.** As Summon Tiger: 100 at the end of the turn at war, when the high priest happens to
stand in a forest (nothing walks it to one); the Treant joins the army as a unit summoned for good.
`spell_worth` 150.

### Summon Water Elemental (`SPELL_SUMMON_WATER_ELEMENTAL`)
*Sphere spell, Water III · summons one Water Elemental for a turn · archmages with Water III*

**What it does.** Summons a temporary Water Elemental (`UNIT_WATER_ELEMENTAL`): 8/8, 1 move, an
elemental with Water Walking (it fights on land and on water), +1 cold strength per Water mana.

**Playing it well.** A strong all-round summon that can also strike ships and coastal stacks from
the sea. Like the Earth Elemental it needs the archmage next to its target.

**AI: Valued.** As Fireball, from its base 8 strength; at the end of the turn `magic::summon_value`
looks only at land targets for a land unit, so its use against ships is not seen. `spell_worth` 160.

### Summon Wraith (`SPELL_SUMMON_WRAITH`)
*Sphere spell, Death III · summons one Wraith for a turn · archmages with Death III*

**What it does.** Summons a temporary Wraith (`UNIT_WRAITH`): 6/6 with +3 death, 2 moves, undead,
with Fear and Vile Touch.

**Playing it well.** The necromancer's striker: 9 strength from two tiles away, which living
defenders may refuse to fight. Undead, elementals and golems ignore its death damage.

**AI: Valued.** As Fireball, from its base 9 strength (Fear and Vile Touch not counted). It is Death
III's best spell by `spell_worth` (180), so archmages with Death mana take Death III for it.

### Treetop Defense (`SPELL_TREETOP_DEFENCE`)
*Sphere spell, Nature I (with Channeling I) · our units in the caster's forest become fully fortified · in a Forest or Ancient Forest · adepts with Nature I, Yvain, Druids*

**What it does.** Python (`spellTreetopDefence`): every unit of our team in the caster's tile is set
to five turns of fortification at once, the full fortify bonus (+25%); the caster must stand in a
Forest or an Ancient Forest.

**Playing it well.** Instant entrenchment: a stack that steps into a forest next to the enemy gets
its full fortify bonus on top of the forest's defense the same turn, instead of waiting turns. Cast
it the turn you take a forest position in the enemy's path.

**AI: Partial.** Not modelled by `magic.rs` (Python only); FfH's terms are its `iAIWeight` 10, so a
Nature caster in a forest casts it whenever nothing scores more than 10, enemy near or not, and
nothing leads stacks into forests for it. `spell_worth` 0.

### Trust (`SPELL_TRUST`)
*Sphere spell, Spirit III · once a game · archmages with Spirit III*

**What it does.** Python (`spellTrust`): sets the player's Trust feat; it can be cast once a game
and never by the barbarians. In FFH2 the C++ AI read the feat as +4 attitude toward the caster's
owner. In this game no rule and no ffh-ai code reads it (ffh-ai's attitude is its own
alignment, religion, borders and war score, `diplomacy::relation_parts`; only the ffh-cppai port
reads it), so against this game's AI it does nothing.

**Playing it well.** Against the game's own AI, skip it and take Spirit III for something else; in
FFH2 it bought goodwill for trades and peace with the AIs.

**AI: Incidental.** Not modelled by `magic.rs`; FfH's terms are its `iAIWeight` 300, so a Spirit III
caster casts it at its first end of turn, which changes nothing. `spell_worth` 0.

### Tsunami (`SPELL_TSUNAMI`)
*Sphere spell (divine), Channeling II and Divine, caster of the Octopus Overlords · range 2 · 30 cold damage to 75% near water, wrecks improvements · next to water · `bAllowAI` 0 · Cultist*

**What it does.** Python (`spellTsunami`): every unit on a tile within two that is water or next to
water (not the caster's tile) takes 30 cold damage to 75%, ours included, and each improvement there
that is not permanent has a 26% chance to be destroyed. It can be cast only next to water and
declares war on neutrals it hurts on their land.

**Playing it well.** A coastal devastation spell for the Overlords: strike a harbour city's garrison
and the fishing boats, towns and mines along the shore; mind your own ships.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (it reaches the AI only through Wonder's
random casts). `spell_worth` 0.

### Unyielding Order (`SPELL_UNYIELDING_ORDER`)
*Sphere spell (divine), Channeling III and Divine, caster of the Order · creates Unyielding Order in our city · lost when the caster leaves · Prior, Sphener*

**What it does.** Creates the Unyielding Order building in the caster's city (our own, inside our
borders): no unhappiness, -100% maintenance, -10 crime (crime only gates some random events). Python
(`spellUnyieldingOrder`): it also ends the city's revolt timer and takes 9 turns off its hurry
anger. Removed when no unit there can cast it.

**Playing it well.** The fix for a conquered, rioting or overgrown city: park a Prior there and its
unhappiness and upkeep vanish, so it can grow past its happy cap and work every tile. Best in the
biggest city with the worst happiness.

**AI: Partial.** `magic::understood` (a building): worth a quarter of its generic value at the end of
the turn, which reads only the maintenance saved (`production::generic_building_value`); the end of
unhappiness, its main effect, and the Python are not valued. `magic::upkeep_value` keeps the caster
in the city once it is built. Priors come only by upgrade. `spell_worth` 40.

### Valor (`SPELL_VALOR`)
*Sphere spell, Law III · lasting blessing of the caster's stack: +1 experience per combat, +10% spell resistance · archmages with Law III*

**What it does.** Every living unit in the caster's tile (adept, archer, disciple, melee, mounted,
recon) that lacks it gains Valor until dispelled: one extra experience point from every combat and
+10% resistance to spells.

**Playing it well.** A veteran-maker for the main army: a stack that fights often levels much faster,
and the resistance helps against enemy curses. Cast it on new units joining the army.

**AI: Partial.** `magic::understood`, Bless phase, but no strength: 0 before a strike, and at the end
of the turn a tenth of the weight (10) for each unit lacking it; as a lasting blessing it draws a Law
archmage to stacks without it (`magic::caster_target`). The experience and resistance are not
valued. `spell_worth` 10.

### Vitalize (`SPELL_VITALIZE`)
*Sphere spell, Nature III · improves the terrain of the caster's tile · on our own land · archmages with Nature III*

**What it does.** Python (`spellVitalize`): on our own land tile (not water, grassland, marsh or the
hell terrains), snow becomes tundra, tundra or desert become plains (scrub removed), and plains
become grassland.

**Playing it well.** Late terraforming: turn the snow and tundra of a cold start into farmland over
a few casts, or plains into grassland for cottages. Mind the production plains lose.

**AI: Valued.** One of the Python spells `magic.rs` models (`Py::Vitalize`): on a tile one of our cities
can work it is worth a quarter of the yield gain as a turn of building value, plus its `iAIWeight` 10,
and `magic::caster_target` walks the archmage to the tiles that gain 15 or more (snow, tundra,
desert); plains to grassland (food for production) is worth 12 and is cast only where it stands.
`spell_worth` 40.

### Wall of Stone (`SPELL_WALL_OF_STONE`)
*Sphere spell, Earth I (with Channeling I) · creates a Wall of Stone in the city · lost when the caster leaves · adepts with Earth I, Corlindale*

**What it does.** Creates the Wall of Stone building in the caster's city: +25% defense and -25%
damage to its defenses from bombardment (neither against gunpowder units), removed when no unit in
the city can cast it.

**Playing it well.** Instant walls for a threatened city: an Earth adept in the garrison raises
them the turn a siege appears and keeps them by staying. Worth having in every border city at war.

**AI: Partial.** `magic::understood` (a building), but its value is a quarter of the building's
generic value computed without a threat (`production::building_value` with threat 0: 25 defense /2,
so 3), so it is cast only in a city where nothing else has a value, is never a reason for
`magic::caster_target` to move a caster (15 at least), and keeps an adept only weakly. A siege does
not raise it. `spell_worth` 40.

### Water Walking (`SPELL_WATER_WALKING`)
*Sphere spell, Water II · the caster can walk on water · lasting · mages with Water II*

**What it does.** The caster gains Water Walking for good (it cannot be dispelled): it moves over
water tiles, and suffers no penalty attacking from the sea or across a river.

**Playing it well.** Freedom of movement for the mage: cross straits and lakes without transports,
reach islands and strike coastal cities from the water. Cast it once and forget it.

**AI: Partial.** `magic::understood`: at the end of the turn a tenth of the weight (10), so a Water II
mage casts it once when nothing better is on offer; the AI's movement then uses it, since
`reach::Flood` asks the rules where the unit can go, but nothing plans a crossing for it.
`spell_worth` 10.

### Wither (`SPELL_WITHER`)
*Sphere spell, Entropy III · range 1 · 10 death damage to 40% and Withered on living enemies · resistable · archmages with Entropy III*

**What it does.** Each living enemy unit within one tile that fails its resist roll takes about 10
death damage (to 40%) and gains Withered: -10% strength, 20% less healing a turn, half the
experience from combat (dispellable).

**Playing it well.** A lasting curse more than a damage spell: a withered army heals slowly and fights
weaker until dispelled, so cast it on the stack you will grind down over several turns. Undead and
elementals are immune.

**AI: Valued.** `magic::understood` (damage and a curse), Soften phase: the expected damage, plus at
the end of the turn Withered's -10% on the attack of enemies that can strike us. Before a strike the
curse on the defenders counts nothing: `promo_pct` reads the promotion's defense field (absent) for
defense, while the rules' -10% strength also lowers defense. `spell_worth` 20.

### Wonder (`SPELL_WONDER`)
*Sphere spell, Chaos III · 3 to 5 random effects · archmages with Chaos III*

**What it does.** Python (`spellWonder`): 3 to 5 draws from 66 effects, most of them another spell
cast on the spot without its requirements (Blaze, Bless, Blinding Light, Charm Person, Maelstrom,
Earthquake, Fireball, every elemental summon, Tsunami, Snowfall...), some unique: a Kraken, Tiger,
Treant or Spiderkin for a turn, Penguins or Mushrooms on the tile, hell terrain spread or cleansed
around, the caster losing its race. Its damage spells hit our own and neutral units, and
Earthquake, Scorch or a neutral hurt on its land can declare war.

**Playing it well.** A gamble for a Chaos archmage in the middle of an enemy army, where most
outcomes hurt them: cast it inside enemy territory, away from your own units and from neighbours at
peace.

**AI: Partial.** Not modelled by `magic.rs` (Python only); FfH's terms are its `iAIWeight` 300, more
than any ordinary value, so a Chaos III archmage casts it at the end of every turn wherever it
stands, near our own units or a neutral border alike. That 300 is also its end-of-turn reserve in
`magic::cast_phases`, so such a caster never casts anything before a strike. `spell_worth` 0.

## World spells

### Arcane Lacuna (`SPELL_ARCANE_LACUNA`)
*World spell, Amurites · once a game · cast by an arcane unit (Adept line) · needs one typed mana node on the map · AI weight 300*

**What it does.** Two effects. Every arcane unit of the Amurites gains as much experience as there are typed mana resources (mana nodes that have been given a sphere) on the whole map, anyone's. And every player of another team loses spellcasting for 20 turns at normal speed (14 quick, 30 epic, 60 marathon): their units can use abilities, but they cannot cast spells, and world spells are spells too. The rules require at least one typed mana node somewhere on the map.

**Playing it well.** Cast it at the start of a war with a civilization that relies on magic: a Sheaim or Calabim army of summons and skeletons, or a rival's stack of Fireball mages. For twenty turns their summons, damage spells, Haste and healing are gone, while your mages keep casting. The experience grows with the number of typed nodes in the world, so a later cast promotes every adept further. Against a rival with no casters, or at peace, the lockout is wasted.

**AI: Directed.** The end-of-turn choice (`casting::end_turn`) values world spells by FfH's terms (`casting::ffh_value`), where this one is worth just its `iAIWeight` of 300, and casts it once `world_spells::ready` allows. The condition is an expert moment (`world_spells::expert_moment`): at war with a civilization. FfH's condition was 7 typed mana nodes on the map, which no measured game reached; with the expert moment the spell was cast in 13 of 24 fresh games instead of 0 (docs/AI.md, "World spells: AI timing"). No civilization overrides it. `civs/amurites.rs` only keeps casters (`civs::unique_casters`), and its comment that Arcane Lacuna is never cast is from before the expert moment. Any civilization war opens the gate: the AI does not check whether the enemy has casters, or how many typed nodes there are.

### Ardor (`SPELL_ARDOR`)
*World spell, Grigori · once a game · any Grigori unit · needs one great person born · AI weight 300*

**What it does.** It sets the player's count of great people born back to 0, and its great-people threshold modifier back to 0. The next great person then costs what the first did. In these rules every great person raises the threshold modifier by 100% twice (once for the player, once for its team, a BtS quirk). So the thresholds run 100, 300, 500 points and so on at normal speed, and after five great people the next one costs eleven times the first.

**Playing it well.** Wait. The spell saves more the steeper the threshold has become, so the Grigori run their specialist economy until great people come slowly, typically after six to ten, and cast it then. A wonder or great-person city pumping points into a threshold of 1000+ suddenly delivers a great person every few turns again. Casting it after two or three great people wastes it.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds (`world_spells::ready`): 5 great people born. Casting after the third great person instead was tried and lost share on fresh seeds (docs/AI_LOG.md, "World spells: For the Horde at a dozen orcs, Ardor after three great people"). The log's conclusion is that if anything an expert waits longer. No civilization override (`civs/grigori.rs` only trains Medics).

### Divine Retribution (`SPELL_DIVINE_RETRIBUTION`)
*World spell · once a game · cast only by Basium, the Mercurians' hero · AI weight 300*

**What it does.** Every demon and undead unit in the world takes holy damage: the caster's own, allies' and the barbarians' included. The damage is two draws of up to 50 (about 50 on average), less the unit's resistance (twice its defense, twice its level, a quarter of its city's defense, its holy resistance), up to 100%, so it can kill. There is no requirement besides Basium; the only AI-only condition FfH had is now the AI's timing.

**Playing it well.** This is the Mercurians' answer to the Infernals, Armageddon's horsemen and the undead armies of the Sheaim, Calabim and Ashen Veil. Cast it when those units are massed, ideally the turn your angels counterattack the wounded survivors. It does not distinguish friend from foe: an ally's skeleton army or your own vassal's demons suffer too. Cast at an ordinary war against living enemies, it does nothing.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: at war with two civilization teams (`world_spells::ready`). The AI does not count the demons and undead it would hit, or whose they are. docs/AI.md's group-D diagnosis suggests casting it only when the enemy's demons and undead outweigh ours and the neutrals'; that is not implemented. Basium is a hero army unit (`production::role_ok`, Hero role). No civilization override (`civs/mercurians.rs` only disbands an idle settler).

### For the Horde (`SPELL_FOR_THE_HORDE`)
*World spell, Clan of Embers · once a game · any Clan unit · requires peace with the barbarians and at least one barbarian unit · AI weight 300*

**What it does.** Each barbarian orc in the world (race Orc, not a hero) becomes the Clan's where it stands, with a 50% chance each. It keeps its type and experience (`convert`). Hidden-nationality units sharing its plot are moved to a clear plot first. The Clan cannot cast it while at war with the barbarians.

**Playing it well.** The Clan's Barbarian trait keeps them at peace with the barbarians, so orc lairs and barbarian cities fill the map with warbands that harass the Clan's rivals for free. Let the horde grow, and cast when it stands in or next to the enemy you are about to fight: the converts arrive inside the war, already in position. Cast early, it yields a handful of orcs you pay support for, and it ends the barbarians' pressure on your rivals.

**AI: Directed.** FfH's condition is that the barbarian player has 60 units of any kind (`world_spells::ready`). On top of it, `civs/clan_of_embers.rs::spell_value` gives the spell 0 unless the Clan is at war with a civilization or preparing one, so the end-of-turn choice casts it at 300 only then. This moved the cast from about turn 218 to about turn 320; in the development leave-one-out it was worth +141 share at turn 300 (t 2.4) (docs/AI.md, "Kept"). An earlier gate, a dozen barbarian orcs, cost wins and was dropped (docs/AI_LOG.md). Where the orcs stand is not looked at.

### Gifts of Nantosuelta (`SPELL_GIFTS_OF_NANTOSUELTA`)
*World spell, Luchuirp · once a game · Masonry · any Luchuirp unit · AI weight 300*

**What it does.** A Golden Hammer (an equipment unit) appears in each Luchuirp city. A hammer can be given to its city as a permanent free Engineer specialist (`SPELL_GIVE_HAMMER_TO_CRAFTSMAN`), or picked up by a unit for +1 strength (`SPELL_TAKE_GOLDEN_HAMMER_UNIT`). Cities founded later get nothing.

**Playing it well.** One free Engineer in every city for good: production and great-engineer points for golem-building dwarves. The value is the number of cities at the cast, so cast it once your last planned city is founded, not the turn Masonry comes in. Give the hammers to the cities rather than to warriors: +1 strength on one unit is worth far less than a specialist forever.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 5 cities (`world_spells::ready`). It is not timed to the last planned city, which docs/AI.md's group-C diagnosis names as the right moment. The Luchuirp override is for the hammers, not the cast: `civs/luchuirp.rs::spell_value` gives Take Golden Hammer 0 for a unit in our own city, so the hammer gives itself as an Engineer. In round 4, in the games where Gifts was cast, production at turn 300 was +11.8% (t 2.5) and great people +47% (docs/AI.md, "Kept").

### Hyborem's Whisper (`SPELL_HYBOREMS_WHISPER`)
*World spell, Infernals · once a game · only Hyborem · Malevolent Designs · needs three non-capital Ashen Veil cities owned by others · AI weight 300*

**What it does.** It fires an event that lets the Infernals take one of the three best cities of other players that hold the Ashen Veil and are not capitals. The cities are ranked by population x100, a third of their culture, buildings x10, world wonders x100 and improved plots. The chosen city becomes the Infernals' (acquired, not conquered). The requirement is that a third such city exists anywhere in the world.

**Playing it well.** Prepare the ground: spread the Ashen Veil into rivals' big cities with Veil missionaries, and research Malevolent Designs. Then take the best city on offer, a developed city with its wonders, without a fight. If the rivals purge the Veil, or it reaches only their capitals, the spell has nothing to offer.

**AI: Partial.** It has no timing: `world_spells` leaves it out because FfH never timed it for the AI. The end-of-turn choice therefore casts it at its weight of 300 the first turn Hyborem can. The event's answers carry `iAIValue` 1000, 900 and 800, so `events::choose` takes the first, the best city. Nothing in the AI works toward the requirement. Research does not value Malevolent Designs for the spell, and missionaries spread the state religion only in our own cities. docs/AI.md notes that Malevolent Designs was never reached in the measured games.

### Into the Mist (`SPELL_INTO_THE_MIST`)
*World spell, Sidar · once a game · any Sidar unit · AI weight 300*

**What it does.** Every unit the Sidar have at that moment gets the Hidden promotion: invisible to other teams, and +1 first strike. A unit loses it when it fights, casts or uses an ability. Units trained later do not get it.

**Playing it well.** Cast it when the army for a war is built, the turn before you declare or the turn you do. Your stacks then march unseen: the enemy can neither attack them nor count them. Each unit opens its first fight with an extra first strike, and hidden workers and settlers are safe from barbarians meanwhile. Cast too early, the army spends its invisibility on barbarians before the war.

**AI: Directed.** FfH's condition is 40 units (`world_spells::ready`). `civs/sidar.rs` holds the spell: its `spell_value` returns 0, so the end-of-turn choice never casts it. Its turn hook casts it through `civs/world_spell.rs::cast` (a city defender first, so the army keeps its turn) when the Sidar are at war with a civilization or preparing a war and the 40-unit gate holds, or at 60 units with no war. Fresh seeds: the hold changed the cast in 15 of 64 games, share at the end +126 (t 1.81) (docs/AI.md, "Kept"). Once the units are hidden, the AI plays them as usual. Nothing keeps them out of fights to stay hidden, or picks targets that gain from the first strike.

### Legends (`SPELL_LEGENDS`)
*World spell, Kuriotates · once a game · any Kuriotates unit · AI weight 300*

**What it does.** Every city of the Kuriotates, settlements included, gains 300 culture.

**Playing it well.** The Kuriotates claim land by culture, with few full cities and many settlements. Cast it once the settlements are founded (each one counts) and while the borders are still contested: 300 culture takes a new city from Poor straight to Developing (100 at normal speed; Refined is 500), widening its borders twice over disputed plots. Late in the game it is a small step toward a cultural victory.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 5 cities, settlements counted (`world_spells::ready`). docs/AI.md puts that at about turn 200. Its group-C diagnosis would cast it right after the third full city; that is untested. No civilization override (`civs/kuriotates.rs` handles settlements).

### March of the Trees (`SPELL_MARCH_OF_THE_TREES`)
*World spell, Ljosalfar · once a game · Way of the Forests · any Ljosalfar unit · AI weight 300*

**What it does.** On every forest and ancient forest of Ljosalfar land with no visible enemy unit, a Treant (10/10, 1 move) appears for 5 turns. Every such plot becomes a New Forest, so the Ancient Forests are gone for good.

**Playing it well.** A last stand. Dozens of 10-strength defenders in forest at once will break an invasion of your heartland. The price is permanent: Ancient Forests do not regrow, and the elves' forest economy suffers. Cast it only when cities would otherwise fall, never as part of an offensive: Treants move one plot a turn and vanish after five.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: at war with two civilization teams (`world_spells::ready`). That came late, around turn 420, and in 19 of 64 games. A moment of its own (a city in peril with 6 or more of our forests near) fired during the elves' own offensives and lost: wins 37.5% -> 28.1% fresh (docs/AI_LOG.md, "World spells at moments of their own"). The Treants are army units (default AI `UNITAI_ATTACK`, `military::army`). Research does not value Way of the Forests for the spell.

### Mother Lode (`SPELL_MOTHER_LODE`)
*World spell, Khazad · once a game · Mining · any Khazad unit · AI weight 300*

**What it does.** The Khazad gain 25 gold for every Mine they own. Each flat land plot of theirs (not water, peak or hill) also has a 10% chance to become a hill: one food less, one hammer more, and room for a mine.

**Playing it well.** The windfall scales with mines, and Khazad cities live by their gold: the Dwarven Vault in every city follows the treasury per city (50, 100, 150, 200, 300 and 500 gold per city are the steps). Mine the hills, cast when the payout lifts the vaults a level or two, and keep the gold, or spend it on a decisive purchase. The new hills are good for mines and production cities, but hurt farmland.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 10 mines (`world_spells::ready`). The AI does not wait for a vault step: docs/AI.md's group-C diagnosis names that as the expert moment, and saving gold for the vault itself was tried before and dropped (`civs/khazad.rs`). No civilization override.

### Raging Seas (`SPELL_RAGING_SEAS`)
*World spell, Lanun · once a game · Optics · any Lanun unit · AI weight 300*

**What it does.** Every unit not of the Lanun that stands next to water takes cold damage: on coastal land and at sea alike, ships included. The damage is two draws of up to 75, less its resistance (twice its defense and level, a quarter of its city's defense, its cold resistance), up to 100%, so it can kill. Flames and smoke next to water are put out. On owned plots next to water that are not Lanun, each non-permanent improvement is destroyed with a 26% chance.

**Playing it well.** Cast it the turn before your fleet and marines strike a coastal city: its defenders take about 75 damage before the fight. Or cast it when an invasion fleet sits off your coast. It hits every other civilization and the barbarians alike, and wrecks everyone's coastal improvements but yours. That reshuffles the whole world, and allies pay too.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds (`world_spells::ready`, `world_spells::enemy_fleet`). The condition is war with a player, the barbarians included, whose fleet is worth more than 10: frigates and galleons 2, men-o'-war and queens of the line 3, other warships and transports 1. That held in 5 of 40 games, around turn 590. A moment of its own (a strike on a coastal city, or a coastal city in peril) lost (docs/AI_LOG.md). Research never values Optics for it.

### Rally (`SPELL_RALLY`)
*World spell, Bannor · once a game · requires the Crusade civic · any Bannor unit · AI weight 300*

**What it does.** A Demagog (5/5 melee) appears in every Bannor city and on every Town of theirs that has no visible enemy unit. Each such Town is reduced to a Village.

**Playing it well.** An instant army, one 5-strength unit per city and per Town, for a Crusade war, which the civic needs anyway. The price is the Towns' commerce, which takes years to grow back. Cast it when your cottages are still hamlets and villages, so you lose little, or when a war hangs in the balance. Demagogs need support like any unit.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 5 cities (`world_spells::ready`). The rules add Crusade, which the generic civic choice adopts in any war through its `iAIWeight` (`civs/bannor.rs`'s comment), so Rally falls early in such a war. The Towns it costs are not weighed. The Demagogs (default AI `UNITAI_ATTACK`) join the army.

### Religious Fervor (`SPELL_RELIGIOUS_FERVOR`)
*World spell, Malakim · once a game · Priesthood · any Malakim unit, with a state religion · AI weight 300*

**What it does.** A priest of the state religion appears in every Malakim city: a Priest of Leaves, of Kilmorph, of the Empyrean, of the Order, of the Overlords or of the Veil, or an Assassin for the Council of Esus. Each has that religion, and as many experience points as the Malakim have cities with the state religion. The port's Python puts a priest in every city, whether or not the city has the religion; the help says only cities with the religion.

**Playing it well.** Spread the religion first, since every city holding it adds a point of experience to every priest. Then cast before a war: a priest per city, with healing, Bless and the religion's spells, gives every stack a medic and caster. A large Malakim empire of one faith gets a corps of veteran priests that can later become High Priests.

**AI: Directed.** The condition is an expert moment (`world_spells::expert_moment`): the state religion in five of our cities, or in three that are two thirds of ours. A small Malakim empire never reached FfH's five. Fresh seeds: casts in 13 -> 17 games, share at the end +47 (t < 0.3) (docs/AI.md). Holding the spell for a war lost, 30% -> 22.5% (`civs/malakim.rs`, docs/AI_LOG.md). The priests are casters (default AI `UNITAI_MEDIC`, `magic::caster_target`). They are not sent to spread the religion first.

### Revelry (`SPELL_REVELRY`)
*World spell, Balseraphs · once a game · Festivals · any Balseraphs unit, not during a golden age · AI weight 300*

**What it does.** It starts a golden age twice the normal length: 24 turns at normal speed (`golden_age_length` is 12 by `GlobalDefinesAlt`). It cannot be cast during a golden age.

**Playing it well.** A golden age's value is the extra yield of everything you have, so cast it when the empire is big: many cities at full size, specialists working, a wonder or a great-person farm going. It is also good during a war, for the production. It cannot be cast during a golden age, so plan the great people's golden ages around it.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: a total population of 25 (`world_spells::ready`). docs/AI.md ("Kept", Balseraphs) records that Revelry keeps the default timing. No civilization override (`civs/balseraphs.rs` is Loki's).

### River of Blood (`SPELL_RIVER_OF_BLOOD`)
*World spell, Calabim · once a game · any Calabim unit · AI weight 300*

**What it does.** Every city of every other player that is larger than 2 loses 2 population, and every Calabim city gains 2.

**Playing it well.** A huge swing in a large world, best when the rivals' cities are big and yours have the food, health and happiness to hold two more citizens. The Calabim can also turn surplus citizens into vampire experience with Feast. It hurts allies and vassals as much as enemies, and new citizens above your happiness cap only riot.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 5 cities (`world_spells::ready`). There is no civilization override. The round-5 hold, a civilization war or room in our cities for two citizens each, came back with the other Calabim rules but this part did not (`civs/calabim.rs`, docs/AI_LOG.md). Neither our cities' room nor the rivals' sizes is weighed.

### Sanctuary (`SPELL_SANCTUARY`)
*World spell, Elohim · once a game · any Elohim unit · needs a city · AI weight 300*

**What it does.** Every unit of another team inside Elohim borders is moved to the nearest valid plot outside, and for 30 turns no unit of another team can enter Elohim land (`sanctuary_timer`, checked by `unit_can_move_into`).

**Playing it well.** A lockout against an invasion: cast it when an enemy stack is about to take your cities. It throws them out, and you get 30 turns to rebuild or to fight on their soil. It also clears out missionaries, explorers and pillagers. The enemy can still bombard and attack from outside your borders, and a war you are winning anyway gains nothing from it.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: at war with two civilization teams (`world_spells::ready`), cast in 27 of 64 games around turn 420. Both a hold for an invasion (`civs/elohim.rs`) and a moment of its own (a city in peril in a civilization war) were tried, and the measured effect was within noise (docs/AI_LOG.md). The condition does not look at whether anyone is actually invading.

### Stasis (`SPELL_STASIS`)
*World spell, Illians · once a game · any Illian unit · AI weight 50*

**What it does.** Every player of another team, barbarians included, is frozen for 20 turns at normal speed (14 quick, 30 epic, 60 marathon). Their production is disabled, so their cities count as in disorder, and their research is disabled, so the player's commerce rates read 0.

**Playing it well.** The strongest world spell in the game, and strongest early. Twenty turns of every rival's production and research are worth most when the rivals are growing fastest. An Illian player who casts it in the opening takes the rivals' whole start, so the earlier the better. Later it still buys a war: no rival can build defenders while you attack.

**AI: Directed.** Its weight is only 50, but no other term applies, and any Illian unit with nothing better casts it at the end of the turn once FfH's condition holds: 5 cities (`world_spells::ready`), around turn 205. Holding it for a war or a crisis lost (docs/AI_LOG.md). Casting it on turn 1-2 won big on fresh seeds (4.7% -> 23.4%, 13 / 1), but it is held for a balance decision, since it cripples every rival's opening, a human's included (docs/AI.md, "World spells: AI timing"). No civilization override (`civs/illians.rs` builds rituals).

### Veil of Night (`SPELL_VEIL_OF_NIGHT`)
*World spell, Svartalfar · once a game · any Svartalfar unit · AI weight 300*

**What it does.** Every Svartalfar unit with strength gets Hidden Nationality. Such a unit looks barbarian to everyone else, enters any territory, and is always hostile: it can attack, and be attacked, without a declaration of war. Summons it makes share the promotion. Declare Nationality ends it. A unit with hidden nationality cannot join a group, and no unit can join one led by such a unit (`unit_can_join_group`), so the stacks break up into single units.

**Playing it well.** Use it for deniable war: raid, pillage and kill a rival's units and settlers without a declaration, its war weariness or diplomatic penalties, and choose when to declare. Because the units cannot stack, it suits raids and harassment more than sieges. Cast it when the army exists, since later units do not get it, and plan to declare nationality before a real conquest.

**AI: Partial.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 25 units and at peace with every civilization (`world_spells::ready`). Never casting it measured worse, 20% -> 10% (`civs/svartalfar.rs`, docs/AI_LOG.md). But the same choice then throws the veil away. Declare Nationality is an ability worth its `iAIWeight` of 50 less the Hidden Nationality promotion's value of 1 (`casting::ffh_value`, `units::promotion_value`), and the rules' AI-only gate lets ordinary units cast it. So every veiled unit with nothing better to cast declares at the end of the turn: those after the caster in the unit list the same turn, the rest the next. Alazkan, veiled from birth, keeps his while he can cast Mirror (50 beats 49). Nothing uses the deniable attacks in between, and the AI's stacks (`military::army` merges groups) cannot form while the veil lasts.

### Warcry (`SPELL_WARCRY`)
*World spell, Hippus · once a game · any Hippus unit · AI weight 300*

**What it does.** Every Hippus unit with a unit combat type gets Warcry: +1 strength, +1 move and Blitz (it can attack several times a turn). The promotion has a 5% chance a turn to wear off, so it lasts about 20 turns on average.

**Playing it well.** Cast it on the turn the big offensive starts. Horsemen and horse archers with an extra move and Blitz can each win two or three fights a turn and reach cities before defenders arrive. Cast it at peace, or for a war with no army ready, and it wears off for nothing.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: 25 units and at war with a civilization (`world_spells::ready`). That is usually the declaration turn, and this AI strikes within about 3 turns of declaring. Holding it for the strike did not help on fresh seeds (`civs/hippus.rs`, docs/AI_LOG.md, "Warcry held for the strike"). Blitz is not used: `campaign::strike` drops a unit once it has attacked, and letting Blitz units attack again was flat.

### Wild Hunt (`SPELL_WILD_HUNT`)
*World spell, Doviello · once a game · any Doviello unit · AI weight 300*

**What it does.** For every Doviello unit with strength, a Wolf appears on its plot for good. A wolf has 2 strength, and for a unit stronger than 3 it has 2 + (strength - 2) / 2: a 6-strength unit gives a 4-strength wolf, a 10-strength one a 6. The Doviello can then merge wolves into Wolf Packs (Form Wolf Pack).

**Playing it well.** It doubles the unit count of a big army, so cast it once the army is large and a war is on, and fold the wolves into packs. Wolves are fodder: they soak up defenders' first strikes, pillage, and guard flanks. The wolves need support too, so cast at peace with a small army and they are a cost.

**AI: Directed.** Cast by the end-of-turn choice at its weight of 300 once FfH's condition holds: at war with a civilization and 20 units (`world_spells::ready`). The wolves keep their default AI `UNITAI_ANIMAL`, so they play as animals (`units::barbarian`): they attack what they beat at 50% or better, hunt within four turns, or wander. They do not join the army. Holding the spell for the strike, with the wolves made army units, lost (`civs/doviello.rs`, docs/AI_LOG.md). Two wolves that end a turn on the same plot are merged by Form Wolf Pack, an ability worth its weight of 100 at the end of the turn, but nothing brings them together.

### Worldbreak (`SPELL_WORLDBREAK`)
*World spell, Sheaim · once a game · any Sheaim unit · needs an Armageddon counter above 0 · AI weight 300*

**What it does.** Its strength is the Armageddon counter, and it hits everything outside Sheaim land. Every unit in a city the Sheaim do not own takes fire damage of two draws of up to the counter (about the counter on average), less its resistance, up to 100%. Each such city catches fire with a chance of counter / 4 percent, and a fire destroys each of its ordinary buildings with an 11% chance. Each forest or jungle without an improvement starts burning with the same chance.

**Playing it well.** The counter rises as the Ashen Veil spreads and as evil units, wonders and rituals appear, which the Sheaim can push. Cast it at a high counter, on the turn your stacks strike. At a counter of 50 every enemy garrison starts at half health. At 10-20 it is barely worth the world spell. It hits every civilization, so everyone sees the Sheaim as the enemy afterwards.

**AI: Directed.** The condition is an expert moment (`world_spells::expert_moment`): at war with a civilization once the counter is 20. FfH waited for 50, which was never reached. Fresh seeds: casts in 0 -> 10 games, share at the end +24 (|t| < 0.3) (docs/AI.md). The cast falls at the end of any turn of such a war, not on the turn of a strike. A strike moment was tried and did not help, because the counter reaches 20 late, around turn 460, for a weak civilization (docs/AI_LOG.md). Nothing in the AI raises the counter.

## Unit, hero, religion and civilization spells

### Add to Flesh Golem (`SPELL_ADD_TO_FLESH_GOLEM`)
*Unit spell · any unit in a stack with our Flesh Golem, not immortal or immune to magic · the caster dies · not for the AI (`bAllowAI` 0)*

**What it does.** The caster dies, and the Flesh Golem in its stack takes its promotions. Channeling, Divine, racial and resource-based promotions, the sphere promotions that need Channeling II or III, and equipment do not pass. If the caster's strength (or defense) is higher than the golem's, the golem's strength (or defense) rises by 1.

**Playing it well.** The Flesh Golem, raised by Graft Flesh, is a unit you grow. Feed it obsolete veterans whose promotions are worth keeping (Combat, Mobility, Shock, Woodsman), and the strongest units you can spare for the +1 strength, until a single golem carries a whole army's promotions. Do not feed it units whose value is a race or equipment: those do not pass.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`), so AI Flesh Golems stay as they were raised.

### Burning Blood (`SPELL_BURNING_BLOOD`)
*Unit spell · Moroi (the Calabim Axeman) · caster only · not for the AI (`bAllowAI` 0)*

**What it does.** The Moroi gains Burning Blood: +20% strength, +1 move, Blitz and immunity to fear. Each turn it has a 10% chance to wear off and a 20% chance to make the unit a barbarian.

**Playing it well.** Use it for a decisive turn of attacks, a Moroi with Blitz taking two or three fights against a weakened stack. Do not keep a blooded Moroi around after the fight, or it may turn barbarian inside your stack.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Call Blizzard (`SPELL_CALL_BLIZZARD`)
*Priest spell · Priest of Winter (Illians) · needs a Blizzard on an adjacent plot, and neither a Blizzard nor an Ancient Forest on the caster's · AI weight 50*

**What it does.** It moves a Blizzard from an adjacent plot onto the caster's plot. If that plot is grassland, plains or desert, its terrain becomes tundra for good. A unit without Winterborn that enters a Blizzard takes cold damage: two draws of up to 10, less its resistance, to at most 50% (`onMoveBlizzard`). Blizzards come from the Illians' The Deepening ritual, and from this spell.

**Playing it well.** Herd Blizzards onto the routes and chokepoints an invader must use, or onto plots next to your cities, where enemy units take damage moving in while Illian Winterborn units are unhurt. Mind the terrain: every cast on your own grassland makes it tundra for good.

**AI: Incidental.** The spell lives in Python and is not one `magic.rs` models (`magic::understood`), so it keeps FfH's terms (`casting::ffh_value`): no term applies, and its value is its flat `iAIWeight` of 50. A Priest of Winter casts it at the end of any turn it stands next to a Blizzard and nothing else is worth more. Nothing moves priests toward Blizzards or toward enemies, and nothing counts the plot it turns to tundra. The priests come from The White Hand (`civs/illians.rs`).

### Call Blizzard (`SPELL_CALL_BLIZZARD_HIGH_PRIEST`)
*Priest spell · High Priest of Winter (Illians, by upgrade at level 6) · as Call Blizzard · AI weight 50*

**What it does.** As Call Blizzard (`SPELL_CALL_BLIZZARD`), for the High Priest of Winter.

**AI: Incidental.** As Call Blizzard: cast for its flat weight of 50 whenever a Blizzard is adjacent and nothing is worth more.

### Call Form (`SPELL_CALL_FORM`)
*Unit spell · Severed Soul (Sidar) · the caster dies · not for the AI (`bAllowAI` 0)*

**What it does.** The Divided Soul that summoned this Severed Soul is moved to the Severed Soul's plot (if it could move there), and the Severed Soul vanishes.

**Playing it well.** Half of a teleport: a Divided Soul (the Sidar Hunter) casts Sever Soul, sends the flying Severed Soul ahead as a scout, and then calls itself over to it. It moves a recon unit across water or mountains, or into position behind the lines, in one turn.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Call of the Grave (`SPELL_CALL_OF_THE_GRAVE`)
*Horseman spell · Ars Moriendi (an Armageddon unit of the barbarians) · range 2 · AI weight 0*

**What it does.** Within two plots, every unit of a team at war with the caster takes death damage (two draws of up to 40, less its resistance, up to 100%). Every wounded undead unit of a team not at war with the caster, of any owner, is fully healed. Each Graveyard on a plot holding a unit is removed and raises three Wraiths (6/6 undead) for the caster. The Ars Moriendi is the barbarians' world unit, spawned by an Armageddon event.

**Playing it well.** For whoever holds the Ars Moriendi, the barbarians, it is a mass attack, a heal and a summon at once. For the civilizations it means keeping living stacks out of its two-plot reach, and fighting it with units immune to death damage.

**AI: Unused.** The barbarians cast by FfH's terms (`magic::enabled` is false for them). The spell lives in Python, no term applies and its `iAIWeight` is 0, so its value is 0 and `casting::choose_spell` never casts it. The barbarians' Ars Moriendi only fights.

### Consume Soul (`SPELL_CONSUME_SOUL`)
*Archmage spell · Eater of Dreams (the Sheaim Archmage) · in our city · costs one population · not for the AI (`bAllowAI` 0)*

**What it does.** The city loses a citizen, and the Eater of Dreams regains its cast for the turn, so it can cast a second spell.

**Playing it well.** Two archmage spells in one turn: two Meteor Swarms, or two big summons, at the moment that decides a battle. Spend the citizens of a large city that has unhappy citizens to spare, not of your capital.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Contagion (`SPELL_CONTAGION`)
*Hero spell · Mary Morbus (an event unit) · range 1 · resistable · living units not on our team*

**What it does.** Every living unit not on our team within one plot (not on the caster's own plot) takes death damage, about 10 on average (a roll of 5-14, then two draws of up to that), less its resistance, to at most 40%. It also gains Plagued: -60% strength and -20% healing a turn until cured. The damage and the curse each roll separately for resistance. Mary Morbus comes from the alchemy-lab event, when the experiments are continued.

**Playing it well.** Mary walks next to an enemy stack or city and cuts its strength by more than half in one cast; your attackers then fight 40%-strength defenders. Cure Disease, medics and units that are not alive counter it. It hits neutral units too, and damaging them can start a war (C++ `doDamage`).

**AI: Valued.** `magic.rs` models it (damage and an added promotion, `magic::understood`). Before an attack it is a Soften-phase cast valued by the damage and the strength the Plagued defenders lose, times the chance they do not resist (`magic::before_attack`). At the end of the turn it is valued the same way on the enemies that can reach us. A cast that would hurt a neutral is ruled out (`magic::reached_value`). Mary plays as an army unit, so she casts when her stack takes her within two plots of a strike's target. Choosing the event answer that creates her is left to `events::choose`.

### Create Puppet (`SPELL_CREATE_PUPPET`)
*Arcane spell, Balseraphs · any Balseraphs arcane unit · summons one Puppet until our next turn · AI weight 1000*

**What it does.** It summons a Puppet (2/2, 1 move) that copies the caster's promotions (not equipment or race) and has the Puppet promotion: -20% strength, not alive, -10 to spell damage, +20% to its targets' magic resistance. Copied sphere promotions let it cast the caster's spells. Like every temporary summon, it is gone at the start of our next turn.

**Playing it well.** A disposable copy of your mage: send the puppet forward to cast where the mage would be in danger, at the front, next to an enemy stack, or for risky spells such as Domination or Wonder. It is also a free fighter or blocker for a turn. The mage itself stays safe in the second line.

**AI: Partial.** `magic.rs` values it as a summon (`magic::summon_value`, the fight it would take) and adds the flat `iAIWeight` of 1000 (`magic::turn_value`). So every Balseraphs caster creates a puppet at the end of every turn, whatever the situation, and before an attack a caster casts only what is worth more than that (`magic::cast_phases` reserve). The puppet attacks at once if it beats a target at 5% or better (`casting::summons_attack`), and otherwise dies unused. It is created after the turn's units have moved, so it never casts its copied spells. Valuing it as a plain summon instead of the 1000 lost share (docs/AI_LOG.md, "Tried in development").

### Create Puppet (`SPELL_CREATE_PUPPET_LOKI`)
*Hero spell · Loki (Balseraphs) · summons one Puppet until our next turn · AI weight 1000*

**What it does.** As Create Puppet (`SPELL_CREATE_PUPPET`), for Loki: the puppet (`UNIT_PUPPET_LOKI`, 2/2, with the Puppet promotion) copies Loki's promotions, including his Chaos I and Mind I spells.

**Playing it well.** Loki's puppet can carry his Dance of Blades or Inspiration where Loki should not go, or take a hit for him. His own turns are better spent on Disrupt in a weak foreign city.

**AI: Partial.** As Create Puppet: worth its summon value plus 1000 at the end of the turn, so Loki makes a puppet every turn that `civs/balseraphs.rs` does not spend on Disrupt (the census: 230 puppets per Loki, docs/AI.md "Unique units in play"). A puppet that cannot win a fight at 5% or better dies unused.

### Crush (`SPELL_CRUSH`)
*Druid spell · Dwarven Druid (Khazad and Luchuirp Druid) · range 2 · AI weight 0*

**What it does.** Of the plots within two that hold only units at war with the caster, it picks the one with the most of them. Every unit there takes physical damage: two draws of up to 50, less its resistance, to at most 75%. It needs such a plot in range.

**Playing it well.** A siege tool: crush the stack in an enemy city or the doomstack next to your border before the attack, so the defenders fight hurt, at a quarter of their health at worst. It ignores neutral-occupied plots, so it is safe to cast in mixed crowds. Physical damage is not raised by spell-damage promotions.

**AI: Valued.** `magic.rs` models its Python (`magic::py_kind`, `Py::Crush`): the plot with the most enemies and no one else, 50 damage on each unit there. It is cast before attacks, and at the end of the turn on enemies that can reach us. The limit is having Dwarven Druids. They need a Grove, Commune with Nature and a Neutral alignment, and production trains them only for the Attack role, where their 8 strength must win on strength squared per turn of production against cheaper units (`production::best_unit`); otherwise they come by upgrade.

### Cure Disease (`SPELL_CURE_DISEASE`)
*Medic spell · any unit with Medic II · the caster's plot · AI weight 0*

**What it does.** It removes Diseased (-30% strength), Plagued (-60%) and Withered (-10%, half experience) from the units on the caster's plot. Priests of the religions, the Grigori Medic and the healer heroes have Medic II.

**Playing it well.** Keep a Medic II priest in every stack facing plague carriers (Mary Morbus, Plague Carrier units, Wither): one cast gives a stack back its strength before it attacks. Cast it before healing, since the curses also slow healing.

**AI: Valued.** `magic.rs` values the removal on our units as the strength the curse took (`magic::reached_value`), so a medic casts it at the end of the turn, or before an attack, when its plot holds afflicted units. Medics are not moved to afflicted units, though: `magic::caster_target` sends casters to the operation's front, to lasting blessings, city spells and terraforming, not to curses to lift.

### Disrupt (`SPELL_DISRUPT`)
*Hero spell · Loki (Balseraphs) · in a city of another team · AI weight 50*

**What it does.** The city gets 2 turns of hurry anger, and its owner loses 0-2 of its culture there. If the owner's culture in the city falls below 1, the city becomes the Balseraphs' without war (acquired), and its owner's attitude toward them drops by 4. Loki may enter any territory.

**Playing it well.** Loki steals cities without war: pick new or low-culture cities of rivals that produce no culture, especially newly founded or captured ones, and cast every turn until they flip. A city that produces any culture practically never falls, since its culture grows about as fast as Disrupt drains it (one point a turn on average). Each flip costs attitude with a civilization that is still at peace with you.

**AI: Directed.** `civs/balseraphs.rs` (`Balseraphs::unit`, `loki`): at peace with every civilization, Loki goes to the nearest reachable (12 turns) revealed city of a civilization we are not at war with, where the owner's culture is below 20 and its culture rate is 0, and he casts Disrupt there every turn. That took 1-10 cities a game in traced games, and the logic was kept on fresh seeds (wins 9.4% -> 23.4%, docs/AI.md "Result on fresh seeds"). The end-of-turn choice would never pick it: Create Puppet's 1000 outbids its weight of 50. At war, or hurt (more than 20 damage), Loki moves as a caster instead.

### Drown (`SPELL_DROWN`)
*Religious spell · a Warrior-class unit (Warrior, Bloodpet, the Doviello's) · Octopus Overlords state religion and a Temple of the Overlords in the city · 60 gold*

**What it does.** For 60 gold (plus the player's upgrade cost modifier) the Warrior becomes a Drown: 3/3 with +1 unholy damage, undead, water walking, vulnerable to fire. The Drown cannot enter ocean, upgrades to the Stygian Guard, and keeps the Warrior's experience and promotions.

**Playing it well.** It turns the obsolete Warriors of an Overlords empire into undead troops, cheaper than training a 90-hammer Drown, and onto the Stygian Guard line. Water walking lets them cross coast and lakes to raid. Do it in the city with the Temple of the Overlords, with spare gold.

**AI: Unused.** It is not modelled by `magic.rs` (a conversion, `magic::understood`), so it keeps FfH's terms (`casting::ffh_value`). The conversion is worth the Drown's power less the Warrior's (600 - 450 = 150), and the gold costs 4 a gold (-240), so its value is always negative and it is never cast.

### Entangle (`SPELL_ENTANGLE`)
*Druid spell · Druid · range 1 · resistable · enemy units only, not flying · immobile 2 turns*

**What it does.** Every enemy unit within one plot (not the caster's own plot, not flying units, not neutrals) that fails to resist cannot move for 2 turns.

**Playing it well.** Root an attacking stack next to your city so it cannot strike this turn or the next while your archers and mages wear it down. Or root fleeing wounded units so your army can finish them. It does not stop the units from defending, and resistant targets (heroes, high-level units) often shrug it off.

**AI: Valued.** `magic.rs` models the immobility (`magic::reached_value`: 25 per turn, up to 2 turns, on each enemy that can attack, times its chance to fail to resist). It is cast at the end of the turn on enemies that can reach us, and before an attack on enemies around the target. The Druid comes only through the Attack role (Grove, Commune with Nature, Neutral alignment; `production::best_unit`) or an upgrade.

### Enter Portal (`SPELL_ENTER_PORTAL`)
*Unit spell · any unit on a Portal · not for the AI (`bAllowAI` 0)*

**What it does.** The unit is moved to the Portal's linked exit plot.

**Playing it well.** A free long jump across the map for scouts, settlers or an army, wherever map generation placed a Portal pair. Check where it leads before sending a valuable unit.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Entertain (`SPELL_ENTERTAIN`)
*Hero spell · Loki (Balseraphs) · in a city · AI weight 50*

**What it does.** The city gets 2 turns of extra happiness. In a city of another player, Loki also takes population / 2 + 1 gold from its owner.

**Playing it well.** Ends a city's unhappiness for two turns in a pinch, or milks gold from rivals' big cities while Loki waits for a Disrupt target.

**AI: Unused.** It lives in Python and keeps FfH's flat weight of 50. But Loki's end-of-turn choice is always won by Create Puppet (summon value plus 1000), which he can cast unless an enemy shares his plot, and his turn hook casts only Disrupt (`civs/balseraphs.rs`). So in practice Entertain is never cast.

### Escape (`SPELL_ESCAPE`)
*Unit spell · Chanter (the Amurite Assassin) · not for the barbarians · AI weight 0*

**What it does.** The Chanter is moved to its capital.

**Playing it well.** A way home: scout or assassinate deep in enemy land, then escape before the counterattack. A wounded Chanter can heal in safety.

**AI: Unused.** Its Python effect is not modelled and its `iAIWeight` is 0, so FfH's terms value it at 0 and it is never cast. The rules' AI-only gate, not at 50 damage or more (docs/AI_ONLY_RULES.md, A2), never comes into play.

### Feast (`SPELL_FEAST`)
*Vampire spell, Calabim · a unit with Vampirism, in our city of size 3 or more · costs one population*

**What it does.** The city loses a citizen and gets 3 turns of hurry anger. The vampire gains experience equal to the city's population less 2.

**Playing it well.** Grow vampires into high-level killers on surplus citizens. Feed in big cities above their happy cap, where the eaten citizen would only riot, and in cities that regrow fast. It pairs with River of Blood and Gift Vampirism. Never feast in a small city or in your best producer.

**AI: Directed.** FfH's terms give it -50 for the lost citizen, so it was never cast. `civs/calabim.rs::spell_value` (`feast_value`) replaces that with 50 per experience point gained, for a unit below 30 experience in a city with a citizen above its happy cap and at least 3 experience to gain. The end-of-turn choice then casts it. `Calabim::turn` puts vampires back on `UNITAI_ATTACK`, so they fight in the army. Vampires are rare and late, so it fires little: 1-14 times a game in round 5 (docs/AI_LOG.md).

### Feed (`SPELL_FEED`)
*Vampire spell · a unit with Vampirism, with our Bloodpet in its stack · AI weight 10*

**What it does.** The vampire heals 20 damage, and the lowest-level Bloodpet in the stack is killed. The help adds that the vampire may attack again. The port's Python only heals and kills; it does not reset the attack.

**Playing it well.** Bring a cheap Bloodpet (the Calabim Warrior) along with a vampire stack as a healing ration: after a hard fight, feed and fight on.

**AI: Incidental.** FfH's terms give it its flat `iAIWeight` of 10. The rules' AI-only gate allows it only at 20 damage or more (docs/AI_ONLY_RULES.md, A2). It is cast at the end of the turn when a wounded vampire happens to share a plot with a Bloodpet and nothing is worth more. Nothing brings Bloodpets to vampires.

### Gift Vampirism (`SPELL_GIFT_VAMPIRISM`)
*Vampire spell · a unit with Vampirism · living units of ours of level 6 or more (a Moroi at level 4) on its plot*

**What it does.** Every qualifying unit on the plot gains Vampirism: +10% strength, +5% healing a turn, and the vampire spells (Feast, Feed, Gift Vampirism). The help's "6th level" is the general rule; Moroi need only level 4.

**Playing it well.** Make your best veterans vampires, so they can Feast on surplus citizens and keep levelling. Gather level-6 units with a vampire once, and convert them all in one cast.

**AI: Directed.** FfH's terms see nothing (the effect is Python only, value 0). `civs/calabim.rs::spell_value` (`gift_value`) values it as the Vampire promotion (`units::promotion_value`, at least 50) for each unit that would gain it, so the end-of-turn choice casts it when a vampire shares a plot with such units. Nothing gathers them: it fires 2-4 times a game (docs/AI_LOG.md).

### Hastur's Razor (`SPELL_HASTURS_RAZOR`)
*Hero spell · Hemah (Octopus Overlords hero) · range 1 · AI weight 30*

**What it does.** It gathers the damage of every unit within one plot, of every owner, and gives each unit a random one of those values. Healthy units may come out badly wounded and wounded ones whole.

**Playing it well.** A gamble that pays when your units in range are hurt and the enemy's are healthy: on average it evens the damage out across both sides. Cast it with a mauled stack of yours next to a fresh enemy stack. Never cast it with only your own units around, since it just shuffles your damage.

**AI: Incidental.** The effect lives in Python and keeps FfH's flat weight of 30. The rules' AI-only gate allows it only with one of our own wounded units in range (docs/AI_ONLY_RULES.md, A2). So the Hemah casts it at the end of the turn whenever one of our units nearby is wounded and nothing else is worth more, without comparing our damage with the enemies'.

### Heal (`SPELL_HEAL`)
*Medic spell · any unit with Medic III · the caster's plot · AI weight 0*

**What it does.** Every unit on the caster's plot loses Poisoned, and every living one heals 20 damage. High priests, the Grigori Medic and the healer heroes have Medic III.

**Playing it well.** Recast the healing every turn in a siege stack: 20 hit points to every unit is worth several turns of rest, and lets a stack strike again a turn sooner. Combine it with Medic III's healing at the end of turn.

**AI: Valued.** `magic.rs` models its Python (`Py::Heal`: up to 20 per wounded unit of ours on the plot, at half weight). A medic casts it at the end of the turn, or before an attack for the attackers it stands with, whenever its plot holds wounded units and nothing is worth more.

### Herald's Call (`SPELL_HERALDS_CALL`)
*Angel spell · Herald (Mercurians, from an Angel at level 4) · not for the AI (`bAllowAI` 0)*

**What it does.** Every living unit of ours on the Herald's plot gains Valor, Blessed, Courage and Loyalty, but its duration is set to 1, so it dies at the start of our next turn. The help names Burning Blood where the port's Python gives Blessed.

**Playing it well.** A last charge: use it on units that are doomed anyway, or on cheap fodder, for one turn of all-out attacks.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Impersonate Leader (`SPELL_IMPERSONATE_LEADER`)
*Hero spell · Gibbon Goetia (Council of Esus hero) · in a city of a civilization run by the AI · the caster dies · not for the AI (`bAllowAI` 0)*

**What it does.** The caster's player takes control of the city owner's whole civilization for 5-14 turns (`reassign_player_advanced`). The Gibbon dies. It cannot target a human's city.

**Playing it well.** For a human against AI rivals: take over a rival for a few turns to make peace, gift cities and units, trade away its gold and techs, or wreck its production queues.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Ira Unleashed (`SPELL_IRA_UNLEASHED`)
*Horseman spell · Avatar of Wrath (an Armageddon unit of the barbarians) · up to 4 Ira per player · AI weight 300*

**What it does.** The caster's owner gets new Ira (8/8, +4 unholy damage, demon, water walking, Cannibalize) on the caster's plot, as many as it takes to have 4. The Avatar of Wrath is the barbarians' world unit, spawned by the Wrath Armageddon event, which also enrages some of the players' units.

**Playing it well.** For the barbarians, four tier-4 demons whenever the Ira fall below four: the Avatar is an army factory, and killing him is worth more than killing his Ira. Civilizations facing him should strike at the Avatar first, with units that beat 33 strength, or bring Divine Retribution.

**AI: Partial.** The barbarians cast by FfH's terms (`magic::enabled` is false for them): the Python effect is not valued, so the spell is worth its flat `iAIWeight` of 300, and the barbarians' end-of-turn choice (`casting::choose_spell`) casts it whenever they have fewer than four Ira. That is the right use, reached by the weight rather than by any reasoning; the Ira then fight with the barbarians' army (`military::army`).

### Kidnap (`SPELL_KIDNAP`)
*Recon spell, Svartalfar · a Svartalfar recon unit · in a city of another team that holds a settled great person · not for the AI (`bAllowAI` 0)*

**What it does.** The caster tries to steal a settled great person (a Great Prophet, Artist, Merchant, Engineer or Scientist specialist) from the city, with a chance of 8% per point of its base strength, at most 95%. On success, the city loses that specialist and the caster's owner gets the great person as a unit. On failure, the caster is either sent to its capital (51%) or killed, and its owner declares war on the city's owner.

**Playing it well.** Use strong recon units (Shadows, assassins at high strength) on rivals' great-person cities, the holy and wonder cities full of settled prophets and engineers. A success takes a rival's great person and gives it to you. A failure means war, so try it only on a rival you are ready to fight.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Mask (`SPELL_MASK`)
*Recon spell · a recon unit of the Council of Esus · Deception · alone in its group, not cargo and carrying none · not for the AI (`bAllowAI` 0)*

**What it does.** The caster gains Hidden Nationality: it looks barbarian to others, enters any territory, and can attack or be attacked by anyone without a declaration of war. Declare Nationality removes it.

**Playing it well.** The Esus agent's disguise: pillage, kill settlers and workers, or snipe wounded units of a rival you are at peace with, without the diplomatic cost. A masked unit cannot stack with others, so it works alone.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Mesmerize Animal (`SPELL_MEZMERIZE_ANIMAL`)
*Recon spell · Satyr (Fellowship of Leaves, from a Fawn at level 4) · range 1 · resistable · AI weight 50*

**What it does.** Every animal at war with the caster within one plot that fails to resist becomes the caster's, appearing on the caster's plot (`convert`, keeping its type and experience). The spell needs such an animal in range.

**Playing it well.** Free units: walk the Satyr into animal country and tame bears, lions and wolves as an early army, or as fodder for a Beastmaster's pack. Strong animals resist more often, so cast it again and again.

**AI: Partial.** Its Python effect is not modelled: FfH's terms halve its value for being resistable (0) and add its flat weight of 50. So a Satyr casts it at the end of the turn whenever an animal stands next to it and nothing is worth more. Every such cast is useful, but nothing walks the Satyr toward animals to tame, and Satyrs come only by upgrade.

### Mirror (`SPELL_MIRROR`)
*Equipment spell · a unit with the Black Mirror (Alazkan, the Svartalfar assassin, has it from birth) · no visible enemy on its plot, not immune to magic · AI weight 50*

**What it does.** It creates an illusion copy of the caster on its plot. The copy has the same type, promotions (not equipment), damage, level and experience, plus Illusion (not alive, immune to death and poison, heals fully after combat, deals at most 90% damage when it attacks). It has already cast, and it is gone at the start of our next turn.

**Playing it well.** A free duplicate fighter every turn: let the copy make the risky attack (losing it costs nothing), or keep it on the plot as an extra defender through the enemies' turn. With Alazkan's poison and first strike, his copy hurts.

**AI: Partial.** Its Python effect is not modelled, so it keeps FfH's flat weight of 50, and Alazkan casts it at the end of nearly every turn. Created then, the copy never acts: it only stands as an extra defender on his plot until our next turn. Nothing uses it to attack. The weight also keeps Alazkan's Hidden Nationality, since Declare Nationality is worth one point less (49).

### Morale (`SPELL_MORALE`)
*Unit spell · Flagbearer (Bannor, Fanaticism, under Crusade) · range 1 · our living units*

**What it does.** Every living unit of our team within one plot gains Morale: +10% strength and +10% city attack. Each turn it has a 10% chance to wear off.

**Playing it well.** Keep a Flagbearer in every attacking stack under Crusade: one cast a turn lifts the whole stack by a tenth, and a little more against cities, for the price of one 120-hammer unit.

**AI: Valued.** `magic.rs` models the promotion. Before an attack it is a Bless-phase cast on the attackers (`magic::bless`, +10% strength and city attack on each). At the end of the turn it is valued by the defense it adds through the enemies' turn (`magic::ours_worth`), since it can wear off. docs/AI.md ("Unique units in play") records that a Flagbearer in a stack casts it every turn, but the Bannor train one in only 18% of the games they could (it is trained only by the generic Defense and Attack roles, where it competes on strength).

### Peace (`SPELL_PEACE`)
*Hero spell · Corlindale (Elohim) · while at war with a civilization · the caster dies · AI weight 0*

**What it does.** The player makes peace with every civilization it is at war with (not the barbarians), and the Armageddon counter is halved. Corlindale dies. It is not possible under the option that forbids changing war and peace.

**Playing it well.** An emergency exit from a war you are losing on several fronts, and a way to push back Armageddon. It costs a 300-hammer hero, so use it when the war would cost more. A lost war ended this way still leaves the enemy free to declare again later.

**AI: Unused.** It keeps FfH's terms: the caster's sacrifice is worth minus its level times its power, and Corlindale's power is his AI weight of 200, so the value is always negative and it is never cast. The AI's wars end through diplomacy instead.

### Peace (`SPELL_PEACE_SEVEN_PINES`)
*Improvement spell · any unit on the Seven Pines, in our borders · while at war with a civilization · once · AI weight 0*

**What it does.** As Peace (`SPELL_PEACE`): peace with every civilization we are at war with, and the Armageddon counter halved. The caster survives, and the Seven Pines lose this power after one cast.

**Playing it well.** If the Seven Pines lie in your land, keep a unit there and use it when a war turns bad, or to slow Armageddon.

**AI: Unused.** Its Python effect is not modelled and its `iAIWeight` is 0, so FfH's terms value it at 0 and it is never cast.

### Pillar of Fire (`SPELL_PILLAR_OF_FIRE`)
*Hero spell · Chalid Astrakein (Empyrean hero) · range 2 · AI weight 0*

**What it does.** Of the plots within two that hold only units at war with the caster, it picks the one where their base strength adds up highest. Every unit there takes fire damage: two draws of up to 50, raised by the caster's spell damage, less its resistance, to at most 75%. By a Python quirk, it may also set fire to the forest or jungle of the last plot scanned (the corner plot at x + 2, y + 2), not of the target.

**Playing it well.** Chalid's siege strike: burn the strongest stack, the city garrison or the doomstack in reach, by up to three-quarters, before your army attacks. Fire resistance and neutral units on the plot make it pick another target.

**AI: Valued.** `magic.rs` models its Python (`Py::PillarOfFire`, valued like Crush on the best enemy-only plot). It is cast before attacks and at the end of the turn on enemies that can reach us. Chalid is trained as a hero (`production::role_ok`, Hero role) and, with 7 strength, plays as an army unit (`units::role_of`), so he is often within two plots of a strike's target.

### Rage (`SPELL_RAGE`)
*Horseman spell · Buboes (an Armageddon unit of the barbarians) · range 1 · resistable · living units*

**What it does.** Every living unit within one plot, of any owner, that fails to resist gains Burning Blood: +20% strength, +1 move, Blitz and immunity to fear, a 10% chance a turn to wear off, and a 20% chance a turn to turn the unit barbarian.

**Playing it well.** For the barbarians it is a recruiting drive: cast next to the civilizations' stacks, some of their units will defect each turn. Civilizations facing the Buboes should keep valuable units out of its reach and kill blooded units before they turn.

**AI: Partial.** The barbarians cast by FfH's terms (`casting::ffh_value`): the added promotion counts as Burning Blood's worth to the caster itself (`units::promotion_value`), halved for being resistable. That is positive, so the Buboes cast it whenever a unit in reach can take it, whether the targets are civilization units that may defect or its own barbarians.

### Read the Grimoire (`SPELL_READ_THE_GRIMOIRE`)
*Equipment spell · a unit with the Infernal Grimoire · not for the AI (`bAllowAI` 0)*

**What it does.** A random page: 20% the caster summons a Spectre, 20% it gains Unholy Taint, 20% it casts Wither, 10% it summons a Pit Beast, 10% it casts Burning Blood, 10% it becomes a Demon, and 10% it dies.

**Playing it well.** A gamble with a one-in-ten death. Read it with an expendable unit (a puppet or a cheap carrier), never with a hero.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Recruit (`SPELL_RECRUIT`)
*Hero spell · a unit with Recruiter (Donal Lugh of the Bannor; the Great Commander) · in a city · once*

**What it does.** It recruits population / 3 + 1 units in the city, twice that with a Temple of the Order. Each unit is a random class from six lines, as advanced as the player's techs allow: Scout to Hunter to Ranger, Scout to Hunter to Assassin, Scout to Horseman to Horse Archer, Scout to Horseman to Chariot, Warrior to Axeman to Champion, Warrior to Archer to Longbowman. The caster loses Recruiter. Any caster but Donal dies.

**Playing it well.** Once a game, an army for free: take Donal to your largest city (one with a Temple of the Order doubles it) and recruit when your techs make the recruits Champions, Rangers and Longbowmen. Size 12 gives 5 units, or 10 with the temple.

**AI: Directed.** `magic.rs` models it (`Py::Recruit`, `magic::recruit_value`): a unit's worth for each recruit, in the first city of ours that gives two or more (or as many as our best city). `civs/bannor.rs` casts it at the start of the Bannor's turn, before units move, so Donal recruits before the army takes him away. The census went from 0 to 1 recruit cast per Donal (docs/AI.md, "Unique units in play"). It does not wait for better techs or a Temple of the Order. A Great Commander is valued the same way, and nothing counts its death: one that ends a turn in a city of ours of size 3 or more casts Recruit there (`magic::turn_value`). That happens if it holds in the capital when no golden age can start (`units::great_person`).

### Recruit Mercenary (`SPELL_RECRUIT_MERCENARY`)
*Wonder spell · any unit of the owner of the Guild of the Nine · in our city · 180 gold*

**What it does.** For 180 gold, the civilization's Mercenary unit appears in the city, with its moves and cast used for the turn. Magnadine's hire gets Loyalty.

**Playing it well.** Gold into soldiers on the spot: when a city is about to be attacked, or to replace losses at the front without waiting for production. It is expensive, so keep it for emergencies.

**AI: Unused.** It keeps FfH's terms, where the gold costs 4 a gold: -720 and no gain counted, so it is never cast. The rules' AI-only gate (never for the Khazad, only with a unit at war within one plot; docs/AI_ONLY_RULES.md, A2) is moot.

### Sever Soul (`SPELL_SEVER`)
*Unit spell · Divided Soul (the Sidar Hunter) · summons a Severed Soul until our next turn*

**What it does.** It summons a Severed Soul: 0 strength, flying, an elemental that can only defend, gone at the start of our next turn. The Severed Soul can cast Call Form to pull its Divided Soul to itself.

**Playing it well.** Scout ahead with the flying Severed Soul, over water and mountains, then use Call Form to teleport the Divided Soul there: a two-step jump for a scout or raider.

**AI: Unused.** `magic.rs` understands it as a summon (`magic::understood`), but a summon is worth the fight it would take, and a 0-strength unit takes none (`magic::summon_fight`), so its value is 0. FfH's summon value is its strength, 0 too. Call Form is `bAllowAI` 0 anyway. It is never cast.

### Sirona's Touch (`SPELL_SIRONAS_TOUCH`)
*Wonder spell · any living unit of the owner of the Shrine of Sirona · once a turn for the player · does not use the unit's cast*

**What it does.** The caster heals 15 damage. The shrine gives the power once a turn to its owner (`FEAT_HEAL_UNIT_PER_TURN`), to any one unit, and the cast does not cost that unit its own spell.

**Playing it well.** A free heal every turn: use it on your most valuable wounded unit (a hero, a high-level mage), every turn, before it acts.

**AI: Unused.** Its Python effect is not modelled and its `iAIWeight` is 0, so FfH's terms value it at 0 and it is never cast (the rules' AI-only gate, 15 or more damage, is moot).

### Snowfall (`SPELL_SNOWFALL_GREATOR`)
*Avatar spell · Auric Ascended (the Illians' Ascension ritual) · range 2 · units not on our team*

**What it does.** Every unit not on our team within two plots (not on the caster's own plot) takes cold damage, about 80 on average (a roll of 40-119, raised by the caster's spell damage, then two draws of up to that), less its resistance, up to 100%, so it can kill. Every land plot in range that is not snow becomes snow for 6-17 turns, and flames and smoke there are put out.

**Playing it well.** The god of winter's area strike: in a single cast it shatters a city's garrison or a field army within two plots, before the Auric (30 strength, +30 cold) walks in. Keep neutral units out of the blast, since damaging them can start a war.

**AI: Valued.** `magic.rs` models the damage (`magic::understood`), and rules the cast out when it would damage a neutral (`magic::reached_value`). Before an attack and at the end of the turn it is valued by the hit points it takes from the enemies in reach. The Auric comes from the Ascension project, which production reaches only through `production::best_project` when no building is worth building.

### Spellstaff (`SPELL_SPELLSTAFF`)
*Equipment spell · a unit with the Spellstaff (the Amurite Wizard has it from birth) · after it has cast this turn · does not use the cast*

**What it does.** It breaks the Spellstaff (the promotion is removed) and restores the caster's cast, so it can cast another spell this turn. It works once per staff.

**Playing it well.** Save it for the turn a second spell decides a fight: two Fireballs into a stack, or a summon and then a blessing. Spending it on a quiet turn wastes it.

**AI: Unused.** `magic.rs` understands it (a removed promotion), but its removal is on the caster only (`bBuffCasterOnly`), which `magic::reached_value` skips, so it is worth 0. FfH's terms count the lost Spellstaff (`iAIWeight` 150) against it. It is never cast, so a Wizard casts once a turn like any other caster.

### Spores (`SPELL_SPORES`)
*Beast spell · Myconid (the Khazad Beastmaster) · range 1 · resistable · living units not on our team · immobile 3 turns*

**What it does.** Every living unit not on our team within one plot (not on the caster's own plot) that fails to resist cannot move for 3 turns. It works on neutral units too.

**Playing it well.** Pin an invading stack in place for three turns while your cannons and attackers grind it down, or hold the defenders of a city in place. Undead, golems and other units that are not alive ignore it.

**AI: Valued.** `magic.rs` models the immobility (`magic::reached_value`): 25 a turn (two counted) for each enemy that can attack, before attacks for the enemies around the target, and at the end of the turn on enemies that can reach us. The census saw Myconids cast it 16 times each (docs/AI.md, "Unique units in play"). Immobilizing neutral units costs nothing in the valuation (only neutral damage rules a cast out).

### Steal (`SPELL_STEAL`)
*Recon spell · a recon unit of the Council of Esus, Esus as state religion · AI weight 50*

**What it does.** The caster tries to take every piece of equipment held by units of other teams on its plot, and every equipment building in a foreign city it stands in. Each item has a 21% chance to start a war with the victim instead (a Python quirk: for a building, with the owner of the last unit on the plot), and otherwise passes to the caster as its promotion.

**Playing it well.** An Esus thief's tool for the world's artifacts: Orthus's Axe, the Godslayer, the Crown of Akharien, the Dragon's Hoard or Sylivan's Perfect Lyre kept in a city, a hero's Golden Hammer. Get onto the plot (Hidden Nationality helps) and steal. Every item is a one-in-five chance of war, so do it only to rivals you would fight.

**AI: Incidental.** Its Python effect is not modelled, so it keeps FfH's flat weight of 50. An Esus recon unit casts it at the end of a turn on which it happens to share a plot with a foreign unit carrying equipment, or stand in a foreign city holding an equipment building, without weighing the war it may start. Nothing sends recon units to do so.

### Teach Spellcasting (`SPELL_TEACH_SPELLCASTING`)
*Hero spell · Govannon (Amurites) · units of ours in his stack · AI weight 0*

**What it does.** Every living unit of ours in Govannon's stack (not animals or birds) gains each first-level sphere promotion Govannon has (Air I to Water I) that it lacks, and Channeling I. They become casters of those rank-1 spells.

**Playing it well.** Govannon turns soldiers into casters: give him several sphere-I promotions (Body for Haste, Enchantment for Enchanted Blade, Fire, Death for Raise Skeleton), then stack him with an army and cast. Every melee unit in the stack can then bless itself or summon.

**AI: Unused.** Its Python effect is not modelled and its `iAIWeight` is 0, so FfH's terms value it at 0 and it is never cast. docs/AI.md ("Unique units in play") notes it as a known gap: Govannon never casts Teach Spellcasting.

### Wane (`SPELL_WANE`)
*Unit spell, Sidar · a living Sidar unit of level 6 or more, not an animal, beast or immortal · up to 4 Shades · not for the AI (`bAllowAI` 0)*

**What it does.** The unit becomes a Shade (2/2, at most four per player). A Shade can join a city as a Great Bard, Sage, Merchant or Engineer specialist.

**Playing it well.** Retire veterans that have outlived their usefulness into super specialists: four extra great-person specialists in your best cities. Defend those cities well, since the specialists are lost with them.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit_can_cast`).

### Whiteout (`SPELL_WHITEOUT`)
*Recon spell, Illians · an Illian Assassin on snow · caster only · AI weight 50*

**What it does.** The Assassin gains Hidden: invisible, +1 first strike, lost when it fights or casts. It needs a snow plot and must not be hidden already.

**Playing it well.** The Illians' stealth: hide assassins in the snow around your lands, then strike wounded units or scouts with the extra first strike, or slip through enemy lines unseen.

**AI: Partial.** `magic.rs` models the promotion on the caster, and the turn value adds its flat `iAIWeight` of 50 (`magic::turn_value`). So an Illian Assassin standing on snow casts it at the end of the turn, for that weight more than for the small first-strike gain. Nothing uses the invisibility: assassins are not sent to snow, and hidden units are played like any other.

## Abilities

### Add to City (`SPELL_ADD_TO_CITY`)
*Ability · caster: a Manes (Infernal demon, 2/2 +1 unholy) · in a city of its own · sacrifices the caster · +1 population*

**What it does.** The Manes dies and its city gains one population. It needs a city the caster's owner holds (`bInCityOnly` + `bInBordersOnly`). Manes reach the Infernals as their starting units, as two gifts (each a chance) whenever a living unit of the Council of Esus, the Ashen Veil or the Octopus Overlords, or a Death or Entropy caster, is killed anywhere, and from razed evil cities (`ffh/events.rs`, `custom.rs::cf_gift_unit`); for an AI Infernal the rules turn a gifted Manes straight into population (100 - 5 x population)% of the time (docs/AI_ONLY_RULES.md A3k).

**Playing it well.** Infernal cities ignore food (`ignore_food`) and never grow on their own, so Manes are their population. Keep a Manes or two per city as cheap garrison and upgrade fodder (they upgrade to Warrior, Scout or Adept), and add the rest where a citizen is worth most: cities with good mines and specialist slots, below their happy cap. A 2-strength unit is worth far less than a working citizen once the borders are safe.

**AI: Directed.** FfH's terms would never cast it: +50 for the population against the caster's sacrifice (level x 450 for a Manes), -400 at level 1 (`casting::ffh_value`). The Infernal hooks do it instead (`civs/infernal.rs`): `Infernal::unit` walks a lone Manes outside our cities to the nearest city, and `Infernal::turn` casts Add to City for each Manes in one of our cities while the other defenders there still meet `units::garrison_quota` (1, 2 in the capital, more under threat), so the last defenders stay units. docs/AI_LOG.md ("Manes added to cities") records why: the first version, one defender kept, gave population +5.0 but cost a third of the army; the quota version is the one in the code.

### Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_DARK_ELF`)
*Ability · caster: a Balseraph Slave with the Dark Elf race (shown as "Elven") · in a city of its own · sacrifices the caster · creates an Elf Cage · AI weight 30*

**What it does.** As Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_HUMAN`), for a slave taken from dark elves (the Svartalfar's race promotion, `PROMOTION_DARK_ELF`, whose name reads "Elven"); it creates the same Elf Cage as the elf variant, so a city holds one elf in its show whichever kind of elf it is.

**Playing it well.** As for the elf variant: a Svartalfar neighbour at war is the source of these slaves.

**AI: Partial.** As Add to Freak Show (human): cast at the end of the turn by FfH's terms when the slave stands in a city of ours without an Elf Cage.

### Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_DWARF`)
*Ability · caster: a Balseraph Slave with the Dwarven race · in a city of its own · sacrifices the caster · creates a Dwarf Cage · AI weight 30*

**What it does.** As Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_HUMAN`), for a slave taken from dwarves (Khazad, Luchuirp), creating a Dwarf Cage.

**Playing it well.** As for the human cage; dwarven slaves come from wars on the Khazad or the Luchuirp.

**AI: Partial.** As Add to Freak Show (human), for the Dwarf Cage.

### Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_ELF`)
*Ability · caster: a Balseraph Slave with the Elven race · in a city of its own · sacrifices the caster · creates an Elf Cage · AI weight 30*

**What it does.** As Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_HUMAN`), for a slave taken from elves (`PROMOTION_ELF`: the Ljosalfar), creating an Elf Cage; the dark elf variant fills the same cage.

**Playing it well.** As for the human cage; one elf per city, from either kind of elf.

**AI: Partial.** As Add to Freak Show (human), for the Elf Cage.

### Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_HUMAN`)
*Ability · caster: a Balseraph Slave with no race · in a city of its own · sacrifices the caster · creates a Human Cage (+1 happy, +1 culture, +1 Great Bard point) · AI weight 30*

**What it does.** The slave dies and the city gets a Human Cage: +1 happiness, +1 culture (not lost with obsolescence) and +1 Great Bard point a turn. A city can hold one cage of each kind (human, elf, dwarf, orc: the spell is refused where the cage stands, `unit_can_create_building`), so four slaves of four races make +4 happiness, +4 culture and +4 Great Bard points. Only the Balseraphs can cast it, and the slave must have no race (`reqAddToFreakShowHuman`). Slaves come from Slavery's 25% enslavement of beaten living units, the Balseraphs' Taskmaster and Courtesan (25% each), and Buy a Slave; an enslaved unit keeps the loser's race (`unit/combat.rs`), which picks the cage.

**Playing it well.** A Balseraph war economy: run Slavery, fight a neighbour of another race, and walk one slave of each race into every city, the cities at their happy cap first. Further slaves are 50%-work-rate workers, Arena fodder or Sell a Slave gold. A 60-hammer slave for a permanent happy face, a culture point and Great Bard points is a good trade in any city that lacks that cage.

**AI: Partial.** No logic names it; FfH's terms value it at the end of the turn (`casting::end_turn`, ability, so `casting::ffh_value` also under `magic.rs`): the cage's building value (`production::building_value`: the happy face at 35-180 by the city's room, the culture point 40, the Great Bard point 40) + 30, minus the caster's sacrifice, which is 0 for a slave (no strength). So every Balseraph slave that ends a turn in a city of ours lacking its cage is caged. What it misses: the AI never takes a slave to a city for it (slaves are workers, `workers::worker`, and reach cities only on their way), the worker lost is not weighed, and nothing seeks the slaves (`civs/balseraphs.rs` handles only Loki).

### Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_ORC`)
*Ability · caster: a Balseraph Slave with the Orcish race · in a city of its own · sacrifices the caster · creates an Orc Cage · AI weight 30*

**What it does.** As Add to Freak Show (`SPELL_ADD_TO_FREAK_SHOW_HUMAN`), for a slave taken from orcs (Clan of Embers units, barbarian orcs), creating an Orc Cage.

**Playing it well.** As for the human cage; barbarian orcs are the easiest source, so this is usually the first cage after the human one.

**AI: Partial.** As Add to Freak Show (human), for the Orc Cage.

### Add to Wolf Pack (`SPELL_ADD_TO_WOLF_PACK`)
*Ability · caster: a Doviello Wolf (2/2) · a Wolf Pack of its owner on the plot · sacrifices the caster · the pack gains the next Empower (+10%, up to five) · AI weight 100*

**What it does.** The wolf dies and a Wolf Pack in its stack gains the lowest Empower promotion it lacks (Empower I to V, each +10% attack and defense; `spellAddToWolfPack`). It is refused once the pack has Empower V.

**Playing it well.** Five wolves for +50% on one 3/3 pack is a poor trade in strength; it pays only for a pack with experience and promotions worth concentrating, or to cut the support of surplus wolves after a Wild Hunt. Usually keep wolves as separate units for screening, pillaging and finishing wounded units.

**AI: Unused.** FfH's terms give it 100 minus the sacrifice, level x 300 for a wolf (2 strength, 2 defense), so -200 at level 1 and worse with levels (`casting::ffh_value`); no civilization hook changes it (`civs/doviello.rs` values only the worker upgrades) and `magic.rs` finds nothing to value before attacks. It is never cast.

### Arena Battle (`SPELL_ARENA_BATTLE`)
*Ability · caster: a melee or recon unit, or a Slave · in a city with an Arena (the Balseraphs' Training Yard) · 50% death · AI weight 10, bAllowAI 0*

**What it does.** The city gets +1 happiness for 3 more turns (the happiness timer) whatever happens. Then a coin flip (`spellArenaBattle`): the unit survives with 1-6 experience and 25 damage, a surviving Slave becoming a Warrior, or it dies.

**Playing it well.** Feed the arena with slaves: a slave that wins becomes a Warrior with experience, one that loses was a 60-hammer worker, and the happiness comes either way. Use it in a city one face short, and to promote a melee unit only when losing it costs little.

**AI: Unused.** `bAllowAI` 0: the rules refuse every AI cast (`unit/ffh.rs::unit_can_cast`).

### Breathe Fire (`SPELL_BREATH_FIRE`)
*Ability · caster: a dragon (Dragon promotion: Abashi, Eurabatres, Drifa, Acheron) · summons a Meteor for one attack*

**What it does.** Summons a Meteor (`UNIT_METEOR`) on the dragon's plot: 6 fire strength, flying, immune to first strikes, collateral damage on up to 4 more units (100, to 60% at most), and it explodes in its attack (suicide). Its death causes no war weariness. Any unit with the Dragon promotion can cast it once a turn: Abashi (Sheaim) and Eurabatres (Kuriotates), the Illians' Drifa (Stir from Slumber) and the barbarians' Acheron.

**Playing it well.** Free collateral every turn: breathe on the enemy stack or city before the dragon and the army attack, so every defender is softened; the Meteor ignores first strikes, so it is good against archers in cities. Against fire-immune or fire-resistant defenders it does little.

**AI: Valued.** An ability, so FfH's terms value it in `casting::end_turn`: 600 (the Meteor's strength x 100) when an enemy is within the Meteor's moves of the dragon, else 0 (`casting::summon_value`); the Meteor then attacks at once the adjacent target where its odds are best, at 5% or better (`casting::summons_attack`). The barbarians' Acheron, a garrison held in his city, breathes on what stands next to it (`units::attack_adjacent`). It is never cast before an attack: `magic::cast_phases` takes abilities, but a caster there casts only what is worth more than its end-of-turn choice, and the Meteor's fight, in hit points (a few hundred at most, `magic::summon_fight`), never beats the 600 FfH points the same dragon holds next to an enemy. So the Meteor comes after the army's attacks, not before them. The dragons are trained as heroes (`production::role_ok`, Hero).

### Buccaneer Crew (`SPELL_CREW_BUCCANEERS`)
*Ability · caster: a naval unit with strength 1+ · in a city · swaps the crew: Buccaneers (+1 strength and defense, -1 move) · bAllowAI 0*

**What it does.** The ship takes the Buccaneers promotion (+1 strength, +1 defense, -1 move) and loses Longshoremen or Skeleton Crew if it had one (`reqCrewBuccaneers`: not already Buccaneers, strength 1+). Crews change only in a city, and Crew Normally removes them.

**Playing it well.** For warships that fight rather than travel: a Frigate or Man-o'-War hunting pirates or holding a strait gains a sixth of its strength for one move. Change back in port before a long voyage.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`); the rules' AI-only clause (Frigates and Men-o'-War only) is dead code (docs/AI_ONLY_RULES.md A2). The AI has no naval combat logic either.

### Buy a Slave (`SPELL_SLAVE_TRADE_BUY`)
*Ability · any unit · in a city of its own · 30 gold · needs the Undercouncil's Slave Trade resolution · bAllowAI 0*

**What it does.** Creates a Slave in the city (`spellSlaveTradeBuy`): 60% no race, 10% Dwarven, 10% Elven, 20% Orcish. Any unit of a player whose Slave Trade resolution has passed can cast it, once a turn.

**Playing it well.** 30 gold for a 60-hammer worker is cheap labour, and for the Balseraphs it is a mixed supply of cage fillers (one cage per race per city) and Arena fodder; the Doviello can turn slaves into warriors. Buy in the turns after the vote, while gold is spare.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`).

### Convert City (`SPELL_CONVERT_CITY_BASIUM`)
*Ability · caster: Basium (Mercurian hero) · in a city with the Mercurian Gate that is not his owner's · the city becomes his, +300 culture · AI weight 1000*

**What it does.** The city with the Mercurian Gate passes to Basium's owner (`player_acquire_city`) and gains 300 of his culture (`spellConvertCityBasium`). When a player completes the Mercurian Gate, the Mercurians join its team and Basium appears in the gate city with six Angels, a Settler and two Workers (`ffh/events.rs::ev_mercurian_gate`); this spell then hands them that city, their first.

**Playing it well.** The builder gives the gate city to its new teammate, so build the gate in a city you can spare that sits where an angelic ally helps, not in your capital. A human builder is offered control of the Mercurians at once.

**AI: Valued.** No logic names it; FfH's weight decides (`casting::ffh_value`: no other term applies, + 1000) and the requirement allows it only where it is meant, so Basium casts it at the end of any Mercurian turn he ends in the gate city. Nothing holds him there (he is an army unit to `units::role_of`, and `civs/mercurians.rs` handles only their settler), but he starts in it, and the measured games show the Mercurians holding that one city (docs/AI.md, "Measured over the games they join").

### Convert City (`SPELL_CONVERT_CITY_RANTINE`)
*Ability · caster: Rantine (Clan of Embers hero, 4 strength) · in a barbarian city whose barbarian units are no stronger than him · the city becomes his · AI weight 1000*

**What it does.** A barbarian city where no barbarian unit on the plot has a base strength above Rantine's becomes his owner's (`reqConvertCityRantine`, `player_acquire_city`), without a fight. The Clan is at peace with the barbarians (Barbarian trait), so Rantine walks in.

**Playing it well.** The Clan's cheapest expansion: watch the barbarian towns near you and walk Rantine in while they hold warriors, archers and axemen, before they get stronger units; promotions that raise his base strength (Strong, Heroic Strength) widen what he can take. Keep him out of wars where he would die for little.

**AI: Directed.** `civs/clan_of_embers.rs::rantine` sends him, while no civilization is at war with us and he has 20 damage or less, to the nearest barbarian city within 15 turns whose visible barbarian units are no stronger than him, and casts on arrival (or at once when he already stands in one); the generic end-of-turn choice would also cast it there at 1000 (`casting::ffh_value`). docs/AI.md: +1.2 cities at turn 300 (t 3.2), share +93 in the leave-one-out.

### Create Battering Ram (`SPELL_CREATE_BATTERING_RAM`)
*Ability · caster: a Khazad melee unit on a Forest · Engineering · takes 3 turns · clears the forest, a permanent Battering Ram · bAllowAI 0*

**What it does.** After a 3-turn delay the forest is cut (`spellCreateBatteringRam`) and a Battering Ram (1/1 siege, 1 move, bombards city defenses by 25% a turn) joins for good; the rules keep the rams fewer than the player's units (`unit_can_create_unit`).

**Playing it well.** Khazad siege without hammers: before a war, have a few axemen in forests next to the front cut rams, then bombard the walls down and attack with the melee units. Use forests you meant to clear anyway.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`). The Khazad get siege engines from their Siege Workshop instead (docs/AI.md, "Unique units").

### Create Den (`SPELL_CREATE_DEN_BEAR`)
*Ability · caster: a Bear (5/5 animal) · alone, undamaged, on unowned land with no improvement, no Bear Den within 4 · a Bear Den, the bear returns to the wild · AI weight 300*

**What it does.** The plot gets a Bear Den (a lair that spawns bears), and the caster is replaced by a barbarian Bear standing on it (`spellCreateDenBear`): its owner loses the unit. It needs the lairs option on (`reqCreateDenBear`).

**Playing it well.** For the barbarians this is how animals spread lairs. A player who captured a bear (Subdue Animal) should almost never cast it: it gives away a 5-strength unit and puts a bear spawner on the map; keep the bear as a fighter or have it build its Dancing Bear in a city (the bear's build list) instead.

**AI: Partial.** FfH's weight alone values it (`casting::ffh_value`: 300), so any AI bear that meets the conditions at the end of its turn casts it (`casting::end_turn` runs for the barbarians too). For the barbarians that is the mechanic working as FfH meant. A civilization's captured bear is an animal-AI unit (`units::role_of`: `UNITAI_ANIMAL` roams like a barbarian's), so it soon stands alone on unowned land and releases itself; recon units take Subdue Animal readily (its AI weight 300 in `units::promotion_value`). Nothing in ffh-ai stops this.

### Create Den (`SPELL_CREATE_DEN_LION`)
*Ability · caster: a Lion Pride (4/4 animal) · same conditions with a Lion Den · AI weight 300*

**What it does.** As Create Den (`SPELL_CREATE_DEN_BEAR`), with a Lion Den (spawning lions) and the pride replaced by a barbarian Lion on it.

**Playing it well.** As for the bear: a barbarian mechanic; a captured pride is better kept, or built into its Lion Cage.

**AI: Partial.** As Create Den (bear): cast for its flat weight by any AI pride that meets the conditions, a captured one included.

### Crew Normally (`SPELL_CREW_NORMAL_CREW`)
*Ability · caster: a naval unit with a special crew · in a city · removes Buccaneers, Longshoremen and Skeleton Crew · bAllowAI 0*

**What it does.** Returns a ship with any of the three crews to its normal crew (`reqCrewNormalCrew`: it must have one), removing the crew's bonus and its drawback.

**Playing it well.** The reset between jobs: a transport back from an invasion drops Skeleton Crew to fight at full strength again, a warship leaving patrol drops Buccaneers to regain its move. Crews change only in a city, so plan the swap at the last port.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`); the requirement's AI clause even refuses it to every AI (docs/AI_ONLY_RULES.md A2, dead code).

### Declare Nationality (`SPELL_DECLARE_NATIONALITY`)
*Ability · caster: a unit with Hidden Nationality · removes it for good · AI weight 50*

**What it does.** The unit loses Hidden Nationality and becomes an ordinary unit of its owner. While hidden, a unit looks barbarian to others outside its owner's cities, enters any territory, is always hostile (it may attack, and be attacked by, anyone without a war), and never captures cities or units (`unit/movement.rs`, `unit/lifecycle.rs`, `unit/promotions.rs::unit_is_no_capture`). Hidden from birth: Nightwatch and Shadowrider (Council of Esus), Alazkan, the Black Wind, Mistform, Pirates and Privateers; the Mercenaries of Magnadine's hire and of a Guild of the Nine event get it, and the Mask spell gives it.

**Playing it well.** Hidden nationality is the point of these units: raid, pillage and kill a rival's workers and settlers while at peace, without diplomatic cost. Declare only when the unit must take a city or capture, or when a garrison should stop drawing attacks from every passer-by. There is no way back.

**AI: Partial.** FfH's terms: the promotion removed at its minimum value (1) and the weight 50, so 49 (`casting::ffh_value`), and the rules' AI-only gate keeps it from Pirates and Privateers (`UNITAI_PIRATE_SEA`), animals, hidden or invisible units and the barbarians (`reqDeclareNationality`, docs/AI_ONLY_RULES.md A2). Every other AI unit with Hidden Nationality casts it at the end of its first turn when nothing else is worth more: Nightwatch, Shadowriders, Alazkan and the Black Wind become ordinary units at once. Nothing in ffh-ai uses hidden nationality for raids at peace.

### Drop Crown of Akharien (`SPELL_DROP_CROWN_OF_AKHARIEN`)
*Ability · caster: a unit carrying the Crown of Akharien · in a city of its own without it · rebuilds the wonder there*

**What it does.** The Crown of Akharien is a world wonder (Arcane Lore, 900 hammers: +2 research, +100% research, +2 culture, +2 Great Scientist points) that a unit can pick up from its city (Take Crown of Akharien, building) and carry as equipment, immune to magic while it holds it; a carrier that dies leaves it on its plot as an equipment unit, which an enemy moving in always captures (`unit/lifecycle.rs`), and the Take spells (unit, promotion) pick it up again. Dropping it in a city of its own that lacks it rebuilds the wonder there, without freeing the class for anyone else (`ffh/spells_b.rs` test).

**Playing it well.** Move the Crown to the city with the most research, where +100% is worth most (usually the capital or the Great Library city). Carry it with an escort and never near a front, since the carrier's death gives it to the enemy; capturing a carrier is the cheap way to take a rival's wonder.

**AI: Incidental.** The AI never takes the wonder out of a city (the building Take spells are `bAllowAI` 0), so it holds the Crown only after a unit of ours picked up a dropped one (Take ... unit or promotion, AI-castable). Then FfH's terms drop it in the first city of ours where the carrier ends a turn: the wonder's building value (`production::building_value`: research, the 100% on the city's rate, culture, Great Scientist points, and +300/+400 for the leaders whose favourite it is) minus the promotion's minimum value (`casting::ffh_value`). No logic carries it to the best city.

### Drop Dragons Horde (`SPELL_DROP_DRAGONS_HORDE`)
*Ability · caster: a unit carrying the Dragon's Hoard · in a city of its own without it · creates The Dragon's Hoard there*

**What it does.** As Drop Crown of Akharien, for The Dragon's Hoard (`BUILDING_THE_DRAGONS_HORDE`): +4 gold, +8 culture, free Gems and Gold, and Enchanted Blade (+20%) for the units in the city. A barbarian city that trains Acheron gets it, Acheron stays there (Held), and while the city is barbarian it now and then spawns Disciples of Acheron (`ffh/events.rs`); taking the city (conquest keeps it) or carrying the hoard away ends that.

**Playing it well.** Kill Acheron and take his city: the hoard comes with it. If the city is poor or exposed, carry the hoard to a safe core city; the two resources count wherever it stands.

**AI: Incidental.** As Drop Crown of Akharien: held only after a pickup of a dropped hoard, dropped in the first city of ours where the carrier ends a turn, valued by the gold and culture (the free resources and Enchanted Blade are invisible to `production::building_value`). The AI gets the hoard mainly by conquering Acheron's city, which needs no drop.

### Drop Syliven's Perfect Lyre (`SPELL_DROP_SYLIVENS_PERFECT_LYRE`)
*Ability · caster: a unit carrying Syliven's Perfect Lyre · in a city of its own without it · rebuilds the wonder there*

**What it does.** As Drop Crown of Akharien, for Syliven's Perfect Lyre (Drama, 900 hammers: +4 culture, +100% culture, +2 Great Bard points).

**Playing it well.** Carry it to the city whose culture matters most: a border city losing tiles to a neighbour, or the cities chasing a cultural victory.

**AI: Incidental.** As Drop Crown of Akharien, valued by its culture and Great Bard points (and the leader-favourite bonus).

### Explore Lair (`SPELL_EXPLORE_LAIR_AIFON_ISLE`)
*Ability · caster: a ship on Aifon Isle (shown as "Maenalus"), a unique ocean site · an epic lair: 54% a monster, 46% a great reward · bAllowAI 0*

**What it does.** Rolls d100 plus the caster's level (`spellExploreLairEpic`): below 54 a named barbarian monster with a random promotion and up to four henchmen appears next to the caster (`cf_explore_lair_big_bad`); 54 or more a great reward (`cf_explore_lair_big_good`: at sea a treasure vault, a golden age, a tech, a clam, crab or fish resource, or a captive that joins: a Sea Serpent, or an Angel, Monk, Assassin, Champion or Mage). The caster's moves end and it gains 1 experience. Epic lairs stay (unique, permanent). Summons, spell and bird units, siege units, defensive-only units and barbarians cannot explore (`reqExploreLair`); on water no war with the barbarians is needed.

**Playing it well.** Each level is a point off the monster's share, so send a high-level unit that can win the fight that may follow, with support next to it; at sea a strong warship. The reward is often worth an early tech; the monster can sink a lone ship.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`). The epic lairs spawn nothing, so the AI's lair clearing ignores them (`military::find_targets` takes only improvements with a spawn unit), and no AI unit ever explores or removes one.

### Explore Lair (`SPELL_EXPLORE_LAIR_BARROW`)
*Ability · caster: a land unit on a Barrow, at war with the barbarians · a lair: risks and rewards by d100 + level · bAllowAI 0*

**What it does.** Rolls d100 plus the caster's level (`spellExploreLair`): under 14 a named monster with henchmen; 14-43 a bad outcome (collapse, a curse such as disease, plague or poison, rusted weapons, spawned enemies); 44-73 a neutral one (nothing, spawned skeletons, lizardmen or spiders, a portal, a cage); 74-93 a good one (gold, treasure, experience, a Healing Salve, supplies, a spirit guide, weapons or a blessing such as Enchanted Blade); 94 or more a great one. Some outcomes then destroy the lair; the caster's moves end and it gains 1 experience. The Barrow is permanent and spawns Skeletons.

**Playing it well.** An early-game lottery for scouts and hunters: with a few levels the bad results thin out. A human's units walking in destroy only lairs that are not permanent (the dens), so a Barrow can be explored again and again until a result destroys it. Keep the explorer healthy and close to help.

**AI: Unused.** `bAllowAI` 0. The AI destroys the Barrow instead: an AI unit at war with the barbarians that enters a lair with a spawn unit removes it (`unit/lifecycle.rs`, docs/AI_ONLY_RULES.md A4), and field armies target such lairs within 6 plots of our cities (`military::find_targets`, an empty one at value 300).

### Explore Lair (`SPELL_EXPLORE_LAIR_BRADELINES_WELL`)
*Ability · caster: a land unit on Bradeline's Well (unique site), at war with the barbarians · an epic lair · bAllowAI 0*

**What it does.** As Explore Lair (Aifon Isle), on land: below 54 (d100 + level) a monster with henchmen, else a great reward, where the land list adds the Jade Torc, Rod of Winds and Timor Mask, captive great people, and a mana or metal resource on the plot.

**Playing it well.** As Aifon Isle: a high-level unit with a stack next to it.

**AI: Unused.** As Aifon Isle: never explored, never cleared.

### Explore Lair (`SPELL_EXPLORE_LAIR_BROKEN_SEPULCHER`)
*Ability · caster: a land unit on the Broken Sepulcher (unique site), at war with the barbarians · an epic lair · bAllowAI 0*

**What it does.** As Explore Lair (Aifon Isle), on land.

**Playing it well.** As Aifon Isle.

**AI: Unused.** As Aifon Isle: never explored, never cleared.

### Explore Lair (`SPELL_EXPLORE_LAIR_DUNGEON`)
*Ability · caster: a land unit on a Dungeon, at war with the barbarians · a lair · bAllowAI 0*

**What it does.** As Explore Lair (Barrow); the Dungeon is permanent and has no spawn unit.

**Playing it well.** As the Barrow.

**AI: Unused.** `bAllowAI` 0, and with no spawn unit the Dungeon is neither destroyed by AI units entering it nor targeted by `military::find_targets`: the AI leaves it untouched.

### Explore Lair (`SPELL_EXPLORE_LAIR_GOBLIN_FORT`)
*Ability · caster: a land unit on a Goblin Fort, at war with the barbarians · a lair (spawns Scorpion Clan goblins) · bAllowAI 0*

**What it does.** As Explore Lair (Barrow); the Goblin Fort is permanent and spawns goblins. A player at peace with the barbarians hires units there instead (Hire Goblin, Archer, Wolf Rider, Chariot).

**Playing it well.** As the Barrow; the Clan of Embers should keep the fort for hiring rather than explore it.

**AI: Unused.** As the Barrow: the AI destroys it by walking in while at war with the barbarians.

### Explore Lair (`SPELL_EXPLORE_LAIR_PYRE_OF_THE_SERAPHIC`)
*Ability · caster: a land unit on the Pyre of the Seraphic (unique site), at war with the barbarians · an epic lair · bAllowAI 0*

**What it does.** As Explore Lair (Aifon Isle), on land.

**Playing it well.** As Aifon Isle.

**AI: Unused.** As Aifon Isle: never explored, never cleared.

### Explore Lair (`SPELL_EXPLORE_LAIR_RUINS`)
*Ability · caster: a land unit on Ruins, at war with the barbarians · a lair (spawns Lizardmen) · bAllowAI 0*

**What it does.** As Explore Lair (Barrow); the Ruins are permanent and spawn Lizardmen.

**Playing it well.** As the Barrow.

**AI: Unused.** As the Barrow: destroyed by AI units walking in.

### Explore Lair (`SPELL_EXPLORE_LAIR_SHIP_WRECK`)
*Ability · caster: a ship on a Ship Wreck · a lair at sea, no war with the barbarians needed · bAllowAI 0*

**What it does.** As Explore Lair (Barrow), explored by a ship; the outcome lists switch to sea monsters, drowned and sea serpents, and sea resources. The wreck is permanent and has no spawn unit.

**Playing it well.** A use for an early explorer ship between coasts: the good results (gold, a sea resource next to your coast) pay for the trip; a lone ship should not linger after a bad roll spawns sea monsters.

**AI: Unused.** `bAllowAI` 0; with no spawn unit it is never cleared either (and the AI has no naval combat logic).

### Form Wolf Pack (`SPELL_FORM_WOLF_PACK`)
*Ability · caster: a Doviello Wolf (2/2) with another of its owner's wolves on the plot · both die, one Wolf Pack (3/3) with their combined experience · AI weight 100*

**What it does.** Two wolves of the same owner on one plot become a Wolf Pack: 3/3, 2 moves, with the sum of both wolves' experience (`spellFormWolfPack`). A pack can then take Empower promotions from more wolves (Add to Wolf Pack). Doviello wolves come mostly from Wild Hunt (a wolf for every unit), and a wolf that wins a fight sometimes becomes a pack (10%).

**Playing it well.** Two wolves are four strength on two bodies, a pack is three on one: merge only to bank experience in one unit that will live, to cut unit support, or to get a defender that survives a stack fight. After a Wild Hunt, keep most wolves single for screening and pillaging, and merge the ones that collected experience.

**AI: Partial.** FfH's weight alone values it (`casting::ffh_value`: no term applies, + 100; the wolves' deaths happen in Python and are not counted as a sacrifice), so whenever two Doviello wolves share a plot at the end of the turn they merge, whatever the trade. A quirk of FfH's loop kept in `casting::end_turn`: the second wolf, already dying, still casts in its own turn and forms a further pack with a spent wolf, so two wolves can give two packs (docs/AI_LOG.md, "Skipping units that another unit's spell has just killed": skipping it cost the Doviello their packs).

### Found Temple of Kilmorph (`SPELL_FOUND_TEMPLE_OF_KILMORPH`)
*Ability · caster: a Stonewarden (Priest of Kilmorph, 120 hammers) · in any city · sacrifices the caster · creates a Temple of Kilmorph and spreads the Runes of Kilmorph · bAllowAI 0*

**What it does.** The priest dies; the city gets a Temple of Kilmorph (120 hammers' worth: +2 gold, +20% culture, +1 happiness with Gems, a Merchant and a Priest slot) and the Runes of Kilmorph spread there. The city need not be its owner's (`bInCityOnly` only), and the temple then opens the religion's disciple units in it (the Stonewarden itself needs a Temple of Kilmorph to be trained).

**Playing it well.** A way to found a second priest-training city at once, or to plant the religion and its temple in a fresh or conquered city without a missionary and the build. In a rival's city it spreads your religion, feeding your holy city's shrine. It costs a tier-3 priest that would otherwise heal and cast, so do it where the temple is needed now.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`).

### Found Temple of Leaves (`SPELL_FOUND_TEMPLE_OF_LEAVES`)
*Ability · caster: a Priest of Leaves (120 hammers) · in any city · sacrifices the caster · creates a Temple of Leaves and spreads the Fellowship of Leaves · bAllowAI 0*

**What it does.** As Found Temple of Kilmorph, for the Temple of Leaves (+1 health, +20% culture, +1 happiness with Incense, an Artist and a Priest slot) and the Fellowship of Leaves.

**Playing it well.** As Found Temple of Kilmorph; the Fellowship's priests are strong healers and summoners, so spend one only where the temple opens disciples you need.

**AI: Unused.** `bAllowAI` 0.

### Found Temple of the Empyrean (`SPELL_FOUND_TEMPLE_OF_THE_EMPYREAN`)
*Ability · caster: a Vicar (Priest of the Empyrean class, whose name reads "Luridus"; 120 hammers) · in any city · sacrifices the caster · creates a Temple of the Empyrean and spreads the Empyrean · bAllowAI 0*

**What it does.** As Found Temple of Kilmorph, for the Temple of the Empyrean (+10% research, +20% culture, +1 happiness with Incense, two Priest slots) and the Empyrean.

**Playing it well.** As Found Temple of Kilmorph.

**AI: Unused.** `bAllowAI` 0.

### Found Temple of the Order (`SPELL_FOUND_TEMPLE_OF_THE_ORDER`)
*Ability · caster: a Confessor (Priest of the Order, 120 hammers) · in any city · sacrifices the caster · creates a Temple of the Order and spreads the Order · bAllowAI 0*

**What it does.** As Found Temple of Kilmorph, for the Temple of the Order (+20% culture, +10% military production, -5 crime, +1 happiness with Incense, two Priest slots) and the Order; a Temple of the Order also doubles Donal's Recruit in that city.

**Playing it well.** As Found Temple of Kilmorph; for the Order, put the temple where Crusaders will be trained or where Donal will recruit.

**AI: Unused.** `bAllowAI` 0.

### Found Temple of the Overlords (`SPELL_FOUND_TEMPLE_OF_THE_OVERLORDS`)
*Ability · caster: a Cultist (Priest of the Overlords, 120 hammers) · in any city · sacrifices the caster · creates a Temple of the Overlords and spreads the Octopus Overlords · bAllowAI 0*

**What it does.** As Found Temple of Kilmorph, for the Temple of the Overlords (+3 culture, +20% culture, +1 happiness with Incense, a Priest and a Scientist slot) and the Octopus Overlords.

**Playing it well.** As Found Temple of Kilmorph; the temple opens Zealots, Cultists and Stygian Guards in that city.

**AI: Unused.** `bAllowAI` 0.

### Found Temple of the Veil (`SPELL_FOUND_TEMPLE_OF_THE_VEIL`)
*Ability · caster: a Ritualist (Priest of the Veil, 120 hammers) · in any city · sacrifices the caster · creates a Temple of the Veil and spreads the Ashen Veil · bAllowAI 0*

**What it does.** As Found Temple of Kilmorph, for the Temple of the Veil (+2 research, +20% culture, +1 happiness with Reagents, a Priest and a Scientist slot) and the Ashen Veil.

**Playing it well.** As Found Temple of Kilmorph; the temple opens Ritualists, Savants and the Beast of Agares in that city.

**AI: Unused.** `bAllowAI` 0.

### Give Hammer to Craftsman (`SPELL_GIVE_HAMMER_TO_CRAFTSMAN`)
*Ability · caster: a Golden Hammer (Luchuirp equipment, immobile) · in a city of its own · consumed · a free Engineer in the city · AI weight 20*

**What it does.** The hammer is used up and its city gains one free Engineer specialist for good (+2 production, 3 Great Engineer points; `spellGiveHammerToCraftsman`). Gifts of Nantosuelta, the Luchuirp world spell, leaves a Golden Hammer in every Luchuirp city; the alternative is Take Golden Hammer (unit), +1 strength on the unit that takes it.

**Playing it well.** Give it almost always: a permanent Engineer in every city is worth far more than +1 strength on one unit, and it brings Great Engineers for wonders. Cast Gifts of Nantosuelta once you have many cities, and take a hammer only for a hero or a key unit.

**AI: Directed.** FfH's terms cast it at the end of the turn (20; the hammer has no strength, so its sacrifice costs nothing in `casting::ffh_value`), and `civs/luchuirp.rs` (`Luchuirp::spell_value`) makes it happen: units in our own cities value Take Golden Hammer at 0, so the garrisons no longer take the hammers first (seeds 1-16 of round 4: 59 takes and 13 gives before; with it, Luchuirp production +11.8% at turn 300, t 2.5, and great people +47%, docs/AI.md).

### Healing Salve (`SPELL_HEALING_SALVE`)
*Ability · caster: a unit carrying a Healing Salve · consumed · heals the unit fully and cures disease, plague, poison and withering · AI weight 100*

**What it does.** The salve is used up; the unit's damage goes to 0 and Diseased, Plagued, Poisoned and Withered are removed (`spellHealingSalve`). It needs some damage. Salves come from lairs (a good Explore Lair result), the Alchemist event, and a dropped salve can be picked up (Take Healing Salve).

**Playing it well.** A one-shot full heal: keep it for a hero or a high-level unit in the middle of a campaign, right after a costly win so it faces the counterattack at full health, or to cure a plague that would spread through the stack. Do not spend it on damage a city would heal in two turns.

**AI: Valued.** FfH's terms (`casting::ffh_value`): the weight 100 less the promotion's minimum value, so 99, gated by the rules' AI-only condition of 25 or more damage (`reqHealingSalve`, docs/AI_ONLY_RULES.md A2). So an AI holder with 25+ damage heals at the end of its turn, after its fights. It does not keep the salve for a better moment or a better unit, and the AI rarely has one: it cannot explore lairs (`bAllowAI` 0), so salves come from the Alchemist event or a pickup.

### Hide (`SPELL_HIDE`)
*Ability · caster: a unit with Stealth (the Sidar's Ghost, Rathus, the Kraken) · not after attacking this turn · the Hidden promotion · bAllowAI 0*

**What it does.** The caster becomes Hidden (`reqHide`: not already hidden, has not attacked this turn): invisible to other teams and +1 first strike, until it fights or casts (removed by combat and by casting).

**Playing it well.** The Sidar's assassins and Rathus vanish after moving next to their target and strike first from hiding; scouts hide to watch an enemy's stacks. Hide again each turn you do not attack.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`). The AI's hidden units come only from Into the Mist, which the Sidar hold for a war (`civs/sidar.rs`); Hide is also one of Wonder's random casts (`ffh/spells_b.rs`).

### Hire Archer (`SPELL_HIRE_ARCHER`)
*Ability · any unit on a Goblin Fort, at peace with the barbarians · Archery · 60 gold · a Scorpion Clan Archer · bAllowAI 0*

**What it does.** Creates a Scorpion Clan Archer at the fort (3/5, +1 poison, a first strike, +25% city defense, Orcish; `spellHireArcher`). The Scorpion Clan hire spells need the caster's team at peace with the barbarians (`reqHireScorpionClan`), which in practice means the Clan of Embers (Barbarian trait); one hire per unit and turn.

**Playing it well.** For the Clan, a Goblin Fort near home is a gold-to-units converter: 60 gold for a 5-defense archer with a first strike, without hammers, in the turn you need it. Buy defenders when a war starts and hammers are committed elsewhere.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`). AI armies at war with the barbarians destroy Goblin Forts by walking in (`unit/lifecycle.rs`, `military::find_targets`); the Clan, at peace with them, leaves the forts standing but never hires.

### Hire Chariot (`SPELL_HIRE_CHARIOT`)
*Ability · any unit on a Goblin Fort, at peace with the barbarians · Trade · 120 gold · a Scorpion Clan Chariot · bAllowAI 0*

**What it does.** As Hire Archer, for a Scorpion Clan Chariot (5/3, 3 moves, +1 poison, 25% withdrawal).

**Playing it well.** As Hire Archer; a fast raider bought on the spot, dear at 120 gold.

**AI: Unused.** `bAllowAI` 0.

### Hire Goblin (`SPELL_HIRE_GOBLIN`)
*Ability · any unit on a Goblin Fort, at peace with the barbarians · 30 gold · a Scorpion Clan Goblin · bAllowAI 0*

**What it does.** As Hire Archer, for a Goblin (2/2, +1 poison, Orcish; it replaces the Scout), with no tech needed.

**Playing it well.** As Hire Archer; 30 gold for an early explorer or a worker escort.

**AI: Unused.** `bAllowAI` 0.

### Hire Units (`SPELL_HIRE_UNITS`)
*Ability · caster: Magnadine (Hippus hero) · in a city of its own · offers one unit for gold · bAllowAI 0*

**What it does.** Opens the Magnadine event (`spellHireUnits`, `EVENTTRIGGER_MAGNADINE_HIRE_UNITS`): pick one unit for gold, each needing its tech, among an Axeman (40 gold, with bronze weapons), a Champion (100, iron weapons), an Archer, a Longbowman, a Horseman, a Horse Archer, a Catapult and a Mercenary (80, with Hidden Nationality), or none.

**Playing it well.** Gold into soldiers in the city that needs them, with weapons already on: buy Champions or Mercenaries the turn a war starts. The Mercenary's hidden nationality lets it raid at peace.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`). The AI rarely has Magnadine anyway: research never plans a path to Warhorses for him (docs/AI.md, "Unique units").

### Hire Wolf Rider (`SPELL_HIRE_WOLF_RIDER`)
*Ability · any unit on a Goblin Fort, at peace with the barbarians · Horseback Riding · 60 gold · a Scorpion Clan Wolf Rider · bAllowAI 0*

**What it does.** As Hire Archer, for a Scorpion Clan Wolf Rider (4/4, 3 moves, +1 poison, 10% withdrawal, Orcish; it replaces the Horseman).

**Playing it well.** As Hire Archer; a cheap mobile counter-raider.

**AI: Unused.** `bAllowAI` 0.

### Inquisition (`SPELL_INQUISITION`)
*Ability · caster: a unit with the Inquisitor promotion (a disciple with Channeling II; the Grigori Luonnotar has it) · in a city of the same state religion · takes 3 turns · purges the other religions*

**What it does.** In a city whose owner shares the caster's state religion (`reqInquisition`), every other religion present, except one whose holy city it is, is removed, with all its buildings (temples, monasteries and the rest whose religion it is); the city then suffers hurry anger for 0-3 turns (one less under the Order) plus one per religion removed (`spellInquisition`). A caster with no state religion can purge every non-holy religion in its own cities.

**Playing it well.** Cleans a conquered city of a rival religion and its buildings and leaves your state religion alone there: for a city that should feed your faith (its shrine income, its units and civics) rather than a rival's. The cost is the purged buildings and the anger, so it is for cities whose other temples you do not need.

**AI: Unused.** No term of `casting::ffh_value` applies and its weight is 0, so its value is 0 and it is never cast; no hook names it. The Inquisitor promotion itself is worth only its weight (10) to `units::promotion_value`, since the spell it opens is an ability (`magic::spell_worth` 0).

### Join Commander (`SPELL_COMMANDER_JOIN`)
*Ability · caster: a unit in a stack with its owner's Great Commander (who has not cast) · the caster gains the Great Commander promotion (+1 strength, +1 defense, +1 experience a fight); the Commander is consumed*

**What it does.** The caster takes the Great Commander promotion and the Great Commander unit is removed (`spellCommanderJoin`). A unit that already has it cannot join (`reqCommanderJoin`), and Commander Split gives the commander back. The Great Commander is a great person born from great-people points (Command Post, Heroic and National Epic, several wonders) or found in a goody; it can also start a golden age or build a Command Post.

**Playing it well.** Join the hero or unit that fights most: +1 strength matters most on low-strength units, the extra experience on the unit that fights every turn, and a hero with it climbs levels fast. Commander Split takes the commander back out before a unit has to be risked.

**AI: Incidental.** FfH's terms value it for the joiner at the promotion's value, 55 (80 for a garrison: +1 strength at 40 and +1 defense at 15 or 40, `units::promotion_value`), but the rules' AI-only gate admits only casters of base strength 6 or more (`reqCommanderJoin`, docs/AI_ONLY_RULES.md A2). Nothing brings the two together: the Great Commander is a great person to the AI (`units::role_of`, `UNITAI_ENGINEER`), and `units::great_person` walks it to the capital to start a golden age. So it joins whichever unit of strength 6+ (in `players[p].units` order) ends a turn on its plot on the way or in the capital, not the army's best; the Split is `bAllowAI` 0.

### Join Commander (`SPELL_COMMANDER_JOIN_DECIUS`)
*Ability · caster: a unit in a stack with Decius · as Join Commander, remembering Decius for the Split*

**What it does.** As Join Commander, with Decius as the commander; the caster remembers him, so Commander Split returns Decius rather than a plain Great Commander (`spellCommanderJoinDecius`). Decius replaces the Great Commander for no civilization, and no rule of the base game creates him: it is a scenario unit.

**Playing it well.** Moot outside scenarios.

**AI: Incidental.** The code would treat it as Join Commander, but no Decius exists in the base game, so it is never cast.

### Longshoremen Crew (`SPELL_CREW_LONGSHOREMEN`)
*Ability · caster: a naval unit with cargo space and strength 1+ · in a city · swaps the crew: Longshoremen (+1 move, -1 cargo) · bAllowAI 0*

**What it does.** The ship takes Longshoremen (+1 move, -1 cargo) and loses Buccaneers or Skeleton Crew (`reqCrewLongshoremen`: it must have cargo space for land units and not have it already).

**Playing it well.** For transports and explorers that need speed more than room: a faster galley or caravel lands a small strike force a turn earlier or scouts coasts. Swap to Skeleton Crew for a full invasion load.

**AI: Unused.** `bAllowAI` 0 (`unit_can_cast`); its AI clause (Galleys and Caravels only) is dead code (docs/AI_ONLY_RULES.md A2).

### Open Chest (`SPELL_OPEN_CHEST`)
*Ability · any unit standing with its owner's Treasure Chest · the chest is used up · `bAllowAI` 0*

**What it does.** Opens a Treasure Chest (`EQUIPMENT_TREASURE`), an invisible, immobile item the
rules place for a player on an empty land plot, preferably unowned or on a small island (the
Treasure and Sailor's Dirge events, and the Treasure result of exploring a lair). A quarter of the
time the chest is trapped: poison (Poisoned and 25 poison damage), fire (50 fire damage), both up
to 90%, or spores (the opener cannot move for 3 turns). Then one of seven results, equally likely:
nothing, 20-120 gold, a Healing Salve, a Jade Torc, a Rod of Winds or a Timor Mask (an item unit on
the plot, for Take X (unit)), or a free tech (`spellOpenChest`, `ffh/spells_b.rs`).

**Playing it well.** Send a cheap unit, not a hero: the traps can leave it at 10% health or pin it
for three turns far from home. Bring the unit that should carry an item along, so it can take what
comes out the same turn. One result in seven is a tech, which makes an early chest worth a detour.

**AI: Unused.** `bAllowAI` 0: the rules refuse the cast to AI players (`unit_can_cast`), and nothing
in `ffh-ai` walks a unit to a chest, so an AI's chests stay where the rules put them.

### Pirate Cove (`SPELL_PIRATE_COVE`)
*Lanun ability · a Work Boat, used up · an empty coast plot of theirs: no improvement, no resource, no
cove, harbor or port within 2 plots · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** The boat becomes a Pirate Cove: 0/0/2 on the plot and +10% defense; it grows into
a Pirate Harbor (1/1/3, +20% defense, +10% healing, upgrade time 10) and then a Pirate Port (2/2/4,
+30%, +20% healing, 20 more), so a coast plot (1/0/2) ends at 3/2/6. The harbor and port grow
whether the plot is worked or not (they are `bOutsideBorders` upgrades), and all three act as a
city for ships. The conditions are `reqPirateCove` (`ffh/spells_b.rs`).

**Playing it well.** The Lanun's best early investment: a 30-hammer boat for a coast plot that ends
at 3/2/6, on water their cities work anyway. Put the first coves on coast worked by the capital and
big coastal cities, three plots apart, before the sea resources are all improved, and let them grow. The coves also give ships a place to heal and hide along a
long coast.

**AI: Directed.** `civs/lanun.rs` (a port of C++ `AI_buildPirateCove`): a Lanun city of size 3+
without threat that works a valid cove plot trains a Work Boat while the Lanun have none
(`cove_boat`), and a Lanun work boat with no unimproved sea resource of ours in reach sails to the
best site, worked plots of big cities first, and casts there (`sea_worker`). FfH's end-of-turn choice also casts it,
at 310 (the improvement's 10 and the weight 300; the sacrifice term is 0 for a strength-0 boat),
wherever a Lanun boat ends its turn on a valid plot, including one on its way to a resource. Round 3
measured it at +23% score in 23 games, the largest lever of its set (docs/AI.md, "Kept").

### Potion of Invisibility (`SPELL_POTION_OF_INVISIBILITY`)
*Item ability · the holder of a Potion of Invisibility, which is used up · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** The holder drinks the potion and becomes Hidden: invisible to other teams and +1
first strike, until it fights or casts. Nothing in a normal game creates the potion: the two lair
results that would give one are never put in the explore-lair list and the Treasure Chest's list
lacks them (FFH2 Python dead code, kept in `ffh/custom.rs` and `spells_b.rs`).

**Playing it well.** It would be a scouting and assassination tool: drink it to slip a hero or an
assassin past a border stack, then strike with the extra first strike; the first fight ends it.

**AI: Partial.** Moot, since no potion appears, but the code would cast it at the end of the turn
after the potion was taken, wherever the holder stood: FfH's terms give Hidden's promotion value (its
weight 50 and 30 for the first strike) minus the potion's (1), 79 (`casting::ffh_value`;
`magic::understood` excludes abilities). Nothing times it to a move that needs stealth.

### Potion of Restoration (`SPELL_POTION_OF_RESTORATION`)
*Item ability · the holder of a Potion of Restoration, which is used up · only after it has cast this
turn · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Lets a unit that has already cast this turn cast again (`bRemoveHasCasted`; not a
temporary unit). As with the other potion, nothing in a normal game creates one.

**Playing it well.** A one-off double cast: a second Fireball or a second heal on the turn that
decides a siege.

**AI: Unused.** FfH's terms see only the removed potion, worth at least 1 (`units::promotion_value`
floors every promotion at 1), so the value is -1 and the end-of-turn choice never casts it; it is
castable only after a cast, when `casting::end_turn` has finished with the unit anyway.

### Promote Settlement (`SPELL_PROMOTE_SETTLEMENT`)
*Kuriotates ability · in one of their settlements · while their full cities are below their limit ·
`bAllowAI` 0, `iAIWeight` 300*

**What it does.** Makes a settlement a full city: radius 3 and no longer a settlement (which has
radius 2, -75% yields and builds nothing), allowed only while the Kuriotates' full cities number
fewer than their Sprawling limit (`reqPromoteSettlement`).

**Playing it well.** When a full city is lost or razed, promote the best-placed settlement at once:
it keeps its population and land, and is worth far more than a fresh city founded in its place.

**AI: Unused.** `bAllowAI` 0. After losing a full city, `civs/kuriotates.rs` founds a new full city
instead (the next city founded below the limit is a full one; the module trains a settler for it).

### Rebuild Barnaxus (`SPELL_REBUILD_BARNAXUS`)
*Item ability · a unit carrying the Pieces of Barnaxus, in a Luchuirp city, not Barnaxus himself ·
`bAllowAI` 1, `iAIWeight` 0*

**What it does.** Barnaxus (the Luchuirp golem hero, a world unit that cannot be trained twice)
carries his own Pieces as a free item, so they drop where he dies. A unit bringing them into a
Luchuirp city rebuilds him there for the city's owner, at 75% damage and with no moves left, with
the city's production experience; a non-Luchuirp caster earns +2 attitude from the Luchuirp
(`spellRebuildBarnaxus`). It is the only way back for him.

**Playing it well.** If Barnaxus falls, retake the plot (the killer captures the pieces by stepping
on them) and walk them home. A rival holding the pieces can trade goodwill for them, or keep them to
deny the Luchuirp their hero.

**AI: Unused.** FfH's terms count only the removed Pieces, -1, and `civs/luchuirp.rs` does not name
the spell, so it is never cast: an AI Luchuirp whose Barnaxus died never gets him back, even when
its own units pick the pieces up (Take Pieces of Barnaxus (unit), below).

### Release from Cage (`SPELL_RELEASE_FROM_CAGE`)
*Ability · any unit on a Cage · frees non-enemy units · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Removes Held from every non-enemy unit on the plot and removes the Cage
(`spellReleaseFromCage`). A Cage comes only from a bad result of exploring a lair: the lair plot
becomes a Cage and every unit there is Held, unable to move (`ffh/custom.rs`).

**Playing it well.** The trapped unit casts it on itself the next turn; the cage costs that turn,
nothing more.

**AI: Unused.** Held has `iAIWeight` -100, but `units::promotion_value` floors it at 1, so removing it
counts -1 and the end-of-turn choice never casts it. Moot: the AI never explores lairs (its units
destroy them, docs/AI_ONLY_RULES.md A4), so its units are never caged.

### Reveal (`SPELL_REVEAL`)
*Ability · any unit in its owner's territory, when the owner has Nox Noctis · permanent for the unit ·
`bAllowAI` 0*

**What it does.** Nox Noctis (the Council of Esus shrine) hides its owner's units in their own
territory outside cities. Reveal exempts the caster for good (`ignore_hide`, `spellReveal`).

**Playing it well.** Rarely worth a cast, since being unseen is the point of Nox Noctis; it is for a
unit you want others to see, such as a border garrison meant to discourage an attack.

**AI: Unused.** `bAllowAI` 0.

### Roar (`SPELL_ROAR`)
*Dragon ability · Abashi, Eurabatres, Drifa and the barbarians' Acheron · range 1, resistable ·
`bAllowAI` 1, `iAIWeight` 30*

**What it does.** Every enemy unit at war within one plot that is not immune to fear and fails a
resist roll flees to a random adjacent plot not next to the dragon (`spellRoar`, `cf_do_fear`),
defenders of a city included. It needs one such unit next to the dragon.

**Playing it well.** Roar next to a city or a stack just before the attack: the units that fail to
resist leave the walls and the fortification behind and can be caught in the open, and the stack you
then attack is thinner. It competes with Breathe Fire for the dragon's single cast; Roar is the
better choice against a strong city garrison, Breathe Fire against a stack in the open.

**AI: Incidental.** FfH's terms give it only its weight, 30 (halving a 0 for resistance changes
nothing). The dragons' Breathe Fire (a Meteor summon, 6 fire strength) is worth 600 whenever an enemy
is within reach (`casting::summon_value`), which is nearly whenever Roar can be cast, so Roar comes
almost only when the adjacent enemy is one the dragon cannot see. `magic::before_attack` does not use it
(`magic::understood` excludes abilities).

### Rob Graveyard (`SPELL_ROB_GRAVE`)
*Ability · any unit on a Graveyard · +1 Armageddon counter · `bAllowAI` 1, `iAIWeight` 5*

**What it does.** Removes the Graveyard (an improvement the Graveyard event puts on one of your
plots), raises the Armageddon counter by 1, and gives one result, equally likely: 20-60 gold,
20-120 gold, hostile skeletons, a hostile spectre, or a tech when one can be received
(`spellRobGrave`).

**Playing it well.** For evil and neutral civilizations, rob it early with a unit that can handle a
skeleton or a spectre (a warrior alone may die): the gold or tech is worth more at turn 50 than at
turn 200. A good civilization that wants to keep Armageddon away leaves it.

**AI: Incidental.** FfH's terms give its weight, 5, and nothing else, so it is cast whenever a
non-Good AI unit ends its turn on a graveyard and has nothing better; the rules' AI-only gate
forbids it to Good AIs (docs/AI_ONLY_RULES.md A2). Nothing sends a unit there.

### Sacrifice (`SPELL_SACRIFICE_ALTAR`)
*Ability · a living unit of level 2+, not a temporary one, in a city with a Demons' Altar · the unit dies ·
`bAllowAI` 0*

**What it does.** The unit is sacrificed and 10 + level² research goes to the current tech: 14 at
level 2, 46 at level 6, 110 at level 10 (`spellSacrificeAltar`). The Demons' Altar is an Ashen Veil
building.

**Playing it well.** A sink for veterans that no longer earn their upkeep (an obsolete high-level
unit stuck in a quiet city), not for anything that still fights: below level 6 the research is
small.

**AI: Unused.** `bAllowAI` 0.

### Sacrifice (`SPELL_SACRIFICE_PYRE`)
*Ability · an Angel on the Pyre of the Seraphic · the unit dies for good · `bAllowAI` 0*

**What it does.** The angel casts Ring of Flames from the Pyre (fire damage to the units around it),
loses one immortality, and summons one permanent Fire Elemental (6/6, +3 fire, collateral damage),
seven if it is a world unit (Basium, Brigit, Sphener); then it dies (`spellSacrificePyre`). The
rules' help text ("Sacrifices the unit to gain research") is wrong: the Python gives no research.

**Playing it well.** A world angel's sacrifice on the Pyre turns one hero into seven elementals, an
army in one turn for a war about to start; an ordinary angel trades one unit for another, and is
only worth it to move an angel's strength into an elemental's collateral damage.

**AI: Unused.** `bAllowAI` 0.

### Sell a Slave (`SPELL_SLAVE_TRADE_SELL`)
*Ability · a Slave · the Undercouncil's Slave Trade resolution passed · the slave dies, +10 gold ·
`bAllowAI` 0*

**What it does.** Sells the slave for 10 gold (the spell's cost is -10).

**Playing it well.** Only for slaves with nothing to do: a slave works land at half a worker's rate,
which over a few turns is worth more than 10 gold.

**AI: Unused.** `bAllowAI` 0.

### Sing (`SPELL_SING`)
*Ability · Guybrush Threepwood (Lanun hero) · `bAllowAI` 0*

**What it does.** Nothing: it picks one of four songs to play (one draw of the game's random numbers,
kept for the port).

**Playing it well.** For fun only.

**AI: Unused.** `bAllowAI` 0, and there is nothing to use.

### Skeleton Crew (`SPELL_CREW_SKELETON_CREW`)
*Ability · a ship, in a city · `bAllowAI` 0*

**What it does.** The ship takes the Skeleton Crew promotion (+1 cargo, -1 strength and defense) and
loses Buccaneers and Longshoremen.

**Playing it well.** On a transport for an amphibious war, where an extra slot matters more than one
point of a weak ship's strength; never on a warship.

**AI: Unused.** `bAllowAI` 0; the rules' AI gate (Queen of the Line, Trireme, Galleon only) is dead
code (docs/AI_ONLY_RULES.md A2). The AI's transports (`naval::transport`) sail with their crews.

### Split Commander (`SPELL_COMMANDER_SPLIT`)
*Ability · a unit with the Great Commander promotion · `bAllowAI` 0*

**What it does.** The unit loses Great Commander (+1 strength, +1 XP per combat, given by Commander
Join when it absorbed a Great Commander) and a Great Commander unit (Decius, if he was the one
joined) appears on its plot (`spellCommanderSplit`).

**Playing it well.** Move the commander to a better carrier (a new hero, a higher-tier unit) or take
it off a unit about to fight at bad odds: the promotion dies with its carrier.

**AI: Unused.** `bAllowAI` 0: an AI's joined commander stays with its first carrier (Commander Join is
AI-castable, gated to casters of base strength above 5, docs/AI_ONLY_RULES.md A2).

### Spreads the Council of Esus (`SPELL_SPREAD_THE_COUNCIL_OF_ESUS`)
*Religion ability · any unit of the Council of Esus, in a city without it · 25 gold · `bAllowAI` 1,
`iAIWeight` 50*

**What it does.** Spreads the Council of Esus to the city. It is the only way to spread Esus by hand:
no unit has an Esus religion spread (there is no Esus missionary). Units get the religion by being
Esus units or by adopting it when trained in an Esus city.

**Playing it well.** Use the Shadows, Nightwatch and other Esus units on their way through your
cities: each Esus city adds to the shrine's income (Nox Noctis) and opens the religion's buildings.
At 25 gold a city it is cheap for what it gives.

**AI: Unused.** FfH's terms: 25 gold at -4 a gold is -100, plus the weight 50: -50, so the end-of-turn
choice never casts it, and the rules' AI-only gate (state religion Esus, docs/AI_ONLY_RULES.md A2)
never comes into play. Production's missionary step (`production::best_missionary`) needs a unit
that spreads the state religion, which Esus lacks, so an Esus AI never spreads it by hand.

### Sprint (`SPELL_SPRINT`)
*Centaur ability · the Kuriotates' Centaur, Centaur Archer, Charger and Lancer, and Herne · not while
Fatigued · `bAllowAI` 0*

**What it does.** The unit gets Hasted (+1 move, gone by the next turn) and Fatigued (-10%
attack and defense, 50% chance to wear off each turn).

**Playing it well.** A fourth move to reach a lone target, a city left empty or safety; the -10%
then matters, so sprint to reposition rather than straight into a close fight.

**AI: Unused.** `bAllowAI` 0.

### Take Athame (`SPELL_TAKE_ATHAME_PROMOTION`)
*Equipment ability (from a unit) · an own unit in the stack carries the Athame, the caster does not ·
`bAllowAI` 1, `iAIWeight` 0*

**What it does.** How equipment works, for every Take X entry below. An item exists in one of three
forms: a promotion on a unit (`bEquipment`), an immobile item unit on a plot (`EQUIPMENT_*`), or, for
four portable wonders, a building in a city. Take X (unit) picks an own item unit up (the item unit
goes, the caster gets the promotion); Take X (promotion), this spell, takes the item from another own
unit on the same plot that has not cast this turn; Take X (building) takes a portable wonder out of
an own city, which loses the building until a Drop spell puts it in a city again. Every take needs a
caster that is not a ship, a siege engine, a spell unit (Fireball, Meteor, Eye, Severed Soul) or an
Illusion and lacks that item; workers, settlers and summons qualify. When a carrier dies, its items drop as item units on its plot, still its owner's
(`unit_kill`); an enemy entering the plot captures them, as items are always captured, and at war an
enemy's item counts as a goody (`plot_is_revealed_goody`). Items move to the new unit when their
carrier upgrades or converts. The Athame itself is Yersinia's: the Horseman of the Apocalypse that
Armageddon spawns drops it when killed, +3 poison strength and Plagued passed to its victims.

**Playing it well.** Hand items to the unit that fights most and survives: a hero or a promoted
shock unit, kept in a stack with a good defender; reshuffle them when a stronger unit arrives.

**AI: Unused.** FfH's terms see nothing in a take-from-a-unit spell (its promotion is added by the
Python, not the XML), so its value is the weight, 0, and it is never cast. For the 17 items whose
Take (promotion) has weight 50, the value is 50, and the rules' AI-only gate (docs/AI_ONLY_RULES.md
A2) lets a caster take only from a unit whose type lacks the item as a free promotion and whose
strength (damage types and promotions included, so the item's own bonus too) is at least 3 below
its own. No AI code moves units to items or chooses a carrier: in `casting::end_turn`, the oldest
eligible unit of the stack whose best spell is the take picks an item up. Items reach AI units by
the rules: a carrier dying in a stack that holds, an attacker advancing onto a fallen carrier's
plot, the Golden Hammers and the Godslayer appearing in AI cities, and explorers, which score an
enemy's items at war as goodies (`units::explore_target`).

### Take Athame (`SPELL_TAKE_ATHAME_UNIT`)
*Equipment ability (pick up) · an own Athame item unit in the stack · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Picks up the Athame (+3 poison, Plagued to its victims) from the plot.

**Playing it well.** Kill Yersinia with a stack, so the attacker that advances captures the Athame
and a strong unit takes it at once; poison is wasted on undead, golems and demons (immune).

**AI: Partial.** FfH's terms give the promotion's value to the caster (`units::promotion_value`: the
3 poison at 40 a point, 120; Plagued is not counted), so any AI unit ending its turn with an Athame of
its own picks it up unless it has a better cast. Nothing brings a unit to an Athame lying elsewhere.

### Take Black Mirror (`SPELL_TAKE_BLACK_MIRROR_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** As Take Athame (promotion), for the Black Mirror, Alazkan the Assassin's
(Svartalfar hero): it allows Mirror, a one-turn Illusion copy of its carrier with its promotions,
damage and level.

**Playing it well.** Give it to the strongest unit you have, since Mirror copies the carrier: an
Illusion cannot kill (it deals at most 90% damage) but a copy of a strong hero softens a defender
for the real attack every turn.

**AI: Unused.** Value 0 (weight 0); never cast, so the Mirror stays with Alazkan or its first finder.

### Take Black Mirror (`SPELL_TAKE_BLACK_MIRROR_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Picks up the Black Mirror (allows Mirror).

**Playing it well.** Pick it up with a strong unit rather than the scout that found it.

**AI: Partial.** Value 1: the Mirror adds nothing `units::promotion_value` reads (Mirror is a
Python-only spell, `magic::spell_worth` 0), and the floor of 1 makes it castable, so the first unit
there takes it, whatever its strength.

### Take Crown of Akharien (`SPELL_TAKE_CROWN_OF_AKHARIEN_BUILDING`)
*Equipment ability (from a building) · in an own city with the Crown of Akharien · `bAllowAI` 0*

**What it does.** Takes the Crown of Akharien, a portable world wonder (900 hammers, Arcane Lore: +2
research, +100% research and 2 Great Scientist points in its city), out of the city: the city loses
it and the caster gets the Crown, which makes it immune to magic. Drop Crown of Akharien puts the
wonder in another own city.

**Playing it well.** Build it anywhere, then carry it to the city with the most research, or out of a
city about to fall. While it is carried, no city has the +100%.

**AI: Unused.** `bAllowAI` 0: the wonder stays where the AI built it (the generic building value reads
its research).

### Take Crown of Akharien (`SPELL_TAKE_CROWN_OF_AKHARIEN_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** As Take Athame (promotion), for the Crown of Akharien (immune to magic).

**Playing it well.** Pass it to whoever carries the wonder to its new city.

**AI: Unused.** Value 0; never cast.

### Take Crown of Akharien (`SPELL_TAKE_CROWN_OF_AKHARIEN_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Picks up a dropped Crown of Akharien (its carrier died).

**Playing it well.** Recover it and drop it in your best research city: a dead carrier's Crown is a
world wonder lying in the field.

**AI: Partial.** Value 1 (magic immunity is not counted), so a co-located unit picks it up. A
carrier that ends a turn in one of our cities puts it there (Drop Crown of Akharien is worth the
building's value in FfH's terms), but nothing carries the Crown to a city, let alone the best one.

### Take Crown of Command (`SPELL_TAKE_CROWN_OF_COMMAND_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** As Take Athame (promotion), for the Crown of Command, Stephanos's (the Horseman of
the Apocalypse): +30% chance to convert living units it defeats.

**Playing it well.** On a unit that wins many fights against living units, to turn kills into
units.

**AI: Unused.** Value 0; never cast.

### Take Crown of Command (`SPELL_TAKE_CROWN_OF_COMMAND_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Picks up the Crown of Command (+30% conversion of defeated living units).

**Playing it well.** As for the Athame: kill Stephanos with a stack and let a strong attacker take it.

**AI: Partial.** Value 1 (the conversion chance is not counted); the first unit there takes it.

### Take Dragons Hoard (`SPELL_TAKE_DRAGONS_HORDE_BUILDING`)
*Equipment ability (from a building) · in an own city with the Dragon's Hoard · `bAllowAI` 0*

**What it does.** Takes the Dragon's Hoard (+4 gold, +8 culture, free Gems and Gold, Enchanted Blade
for the city's units) out of the city. The rules put the Hoard in the barbarian city that raises
Acheron the Red Dragon, so a player gets it by taking that city.

**Playing it well.** Move it from the conquered dragon's lair, often a poor frontier city, to a core
city that can use the culture and is safe from recapture.

**AI: Unused.** `bAllowAI` 0: it stays in the captured city.

### Take Dragons Hoard (`SPELL_TAKE_DRAGONS_HORDE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** As Take Athame (promotion), for the Hoard (the promotion itself does nothing; it is
the wonder in transit).

**Playing it well.** Pass it to a safer carrier on the way.

**AI: Unused.** Value 0; never cast.

### Take Dragons Hoard (`SPELL_TAKE_DRAGONS_HORDE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 0*

**What it does.** Picks up a dropped Hoard.

**Playing it well.** Recover it and drop it in a city.

**AI: Partial.** Value 1; a co-located unit picks it up and carries it as a promotion that does
nothing until the carrier happens to end a turn in one of our cities, where Drop (worth the
building's value) puts it back; nothing takes it there.

### Take Empty Bier (`SPELL_TAKE_EMPTY_BIER_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Empty Bier, Donal Lugh's (Bannor hero): +2
defense and double fortification bonus.

**Playing it well.** It belongs on the city's best defender, not on an attacker.

**AI: Incidental.** Value 50 (the weight), cast whenever the rules' gate allows: a unit at least 3
stronger than the holder takes it, which moves this defensive item to the strongest unit of the
stack, attacker or not. Never from Donal himself (a free promotion).

### Take Empty Bier (`SPELL_TAKE_EMPTY_BIER_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Empty Bier (+2 defense, double fortification).

**Playing it well.** Kill Donal in the open and take it to your capital's garrison.

**AI: Partial.** Value 180 for a field unit, 230 for a garrison (`units::promotion_value`: weight 100
and the +2 defense at 15 or 40 a point, plus the spell's 50); the double fortification is not
counted.

### Take Gela (`SPELL_TAKE_GELA_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for Gela, Hyborem's (the Infernal's hero): +2 unholy
strength.

**Playing it well.** On a fast attacker; wasted on demons (immune to unholy), half wasted on undead.

**AI: Incidental.** Value 50, under the rules' gate; never from Hyborem (a free promotion).

### Take Gela (`SPELL_TAKE_GELA_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up Gela (+2 unholy).

**Playing it well.** Killing Hyborem is a war goal in itself; take Gela with the unit that will keep
fighting.

**AI: Partial.** Value 230 (weight 100, the 2 unholy at 40 a point, the spell's 50).

### Take Godslayer (`SPELL_TAKE_GODSLAYER_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Godslayer: +3 strength, immunity to all seven
damage types, +100% spell resistance and +1000% against an Avatar. It appears when Auric Ulvin
ascends, for a player of another team (a human first, else mostly the leaders): for a human it is
hidden on a remote plot, for an AI in its capital (docs/AI_ONLY_RULES.md A4).

**Playing it well.** The sword that kills the god: give it to a fast unit that can reach Auric's
Avatar.

**AI: Incidental.** Value 50, under the rules' gate, which compares the holder's strength with the
sword's +3 included: the taker must be 6 stronger than the holder's own strength.

### Take Godslayer (`SPELL_TAKE_GODSLAYER_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Godslayer.

**Playing it well.** Go and get it the turn it appears; for a human it is placed like a treasure.

**AI: Partial.** Value 315 for a field unit, 390 for a garrison (weight 100, +3 strength at 40, +3
defense at 15 or 40, the spell's 50; the immunities and the Avatar bonus are not counted). It
appears in an AI's capital, so the oldest unit there, usually a garrison, takes it at the end of the
turn; nothing aims it at the Avatar.

### Take Golden Hammer (`SPELL_TAKE_GOLDEN_HAMMER_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for a Golden Hammer (+1 attack strength), which Gifts of
Nantosuelta, the Luchuirp world spell, leaves in each of their cities.

**Playing it well.** A hammer is worth more as a free Engineer (Give Hammer to Craftsman) than as +1
attack, except on a unit about to fight.

**AI: Incidental.** Value 50, under the rules' gate; rarely in play, since the Luchuirp AI leaves its
hammers alone (below).

### Take Golden Hammer (`SPELL_TAKE_GOLDEN_HAMMER_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up a Golden Hammer (+1 attack strength).

**Playing it well.** Leave them as Engineers in your cities; take one only for a hero's decisive
fight.

**AI: Directed.** FfH's end-of-turn choice valued the take (90: +1 strength at 40, the spell's 50)
above the hammer's own Give Hammer to Craftsman (20), and garrison warriors took 59 hammers against
13 Engineers on seeds 1-16.
`civs/luchuirp.rs` sets it to 0 for a unit in a Luchuirp city of its own, so each hammer becomes an
Engineer (production +11.8% at turn 300, docs/AI.md, "Kept"). A hammer taken in a captured Luchuirp
city by another civilization keeps FfH's 90.

### Take Healing Salve (`SPELL_TAKE_HEALING_SALVE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for a Healing Salve (allows Healing Salve, a one-use
heal; from lairs, Treasure Chests and an event).

**Playing it well.** Give it to the unit most likely to be hurt in the next fight.

**AI: Incidental.** Value 50, under the rules' gate: the salve goes to the strongest unit, not the
most hurt.

### Take Healing Salve (`SPELL_TAKE_HEALING_SALVE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up a Healing Salve.

**Playing it well.** Carry it with the stack's main attacker.

**AI: Partial.** Value 51 (the spell's 50 and the floor of 1; the heal it allows is an ability
`magic::spell_worth` does not value). Moot in most AI games: lairs and chests, its sources, are
never explored or opened by the AI.

### Take Infernal Grimoire (`SPELL_TAKE_INFERNAL_GRIMOIRE_BUILDING`)
*Equipment ability (from a building) · in an own city with the Infernal Grimoire · `bAllowAI` 0*

**What it does.** Takes the Infernal Grimoire (a 400-hammer world wonder, Infernal Pact: a free tech
when built, 1 Great Scientist point, about one chance in five of a barbarian Balor when completed)
out of the city; its carrier can then Read the Grimoire.

**Playing it well.** Only to read it: the wonder's lasting effect in the city is one great person
point, so carrying it costs little.

**AI: Unused.** `bAllowAI` 0.

### Take Infernal Grimoire (`SPELL_TAKE_INFERNAL_GRIMOIRE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 0*

**What it does.** As Take Athame (promotion), for the Infernal Grimoire.

**Playing it well.** Pass it to the unit that should read it.

**AI: Unused.** `bAllowAI` 0, like Read the Grimoire itself.

### Take Infernal Grimoire (`SPELL_TAKE_INFERNAL_GRIMOIRE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 0*

**What it does.** Picks up a dropped Infernal Grimoire.

**Playing it well.** Recover it before an enemy does.

**AI: Unused.** `bAllowAI` 0: the one item the AI never picks up.

### Take Jade Torc (`SPELL_TAKE_JADE_TORC_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Jade Torc (immune to poison, +20% spell
resistance; from lairs and Treasure Chests).

**Playing it well.** On a hero facing enemy casters or poison users.

**AI: Incidental.** Value 50, under the rules' gate.

### Take Jade Torc (`SPELL_TAKE_JADE_TORC_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Jade Torc.

**Playing it well.** As above.

**AI: Partial.** Value 51 (resistances are not counted); moot unless an AI captures one.

### Take Nether Blade (`SPELL_TAKE_NETHER_BLADE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Nether Blade, Rathus Denmora's (Sidar hero): +2
death strength, +80% against heroes.

**Playing it well.** On the unit that hunts enemy heroes.

**AI: Incidental.** Value 50, under the rules' gate; never from Rathus (a free promotion).

### Take Nether Blade (`SPELL_TAKE_NETHER_BLADE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Nether Blade.

**Playing it well.** Kill Rathus and turn his blade on the Sidar's other heroes.

**AI: Partial.** Value 130 (2 death at 40, the spell's 50; the bonus against heroes is not counted).

### Take Orthus's Axe (`SPELL_TAKE_ORTHUSS_AXE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for Orthus's Axe, the barbarian hero Orthus's: blitz
(several attacks a turn) and +1 fire strength.

**Playing it well.** Blitz is worth most on a strong attacker with high withdrawal or first strikes,
which can attack two or three times a turn.

**AI: Incidental.** Value 50, under the rules' gate.

### Take Orthus's Axe (`SPELL_TAKE_ORTHUSS_AXE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up Orthus's Axe (blitz, +1 fire).

**Playing it well.** Kill Orthus early (5 strength) with a stack and take the axe: an early blitz
unit clears barbarians fast.

**AI: Partial.** Value 90 (1 fire at 40, the spell's 50); blitz, the axe's real worth, is not
counted. An AI attacker that kills Orthus and moves onto his plot captures the axe and, if it is
not a siege engine, takes it at the end of the turn.

### Take Pieces of Barnaxus (`SPELL_TAKE_PIECES_OF_BARNAXUS_PROMOTION`)
*Equipment ability (from a unit) · not from Barnaxus himself · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Pieces of Barnaxus (no effect but Rebuild
Barnaxus in a Luchuirp city).

**Playing it well.** Pass them to the unit that will reach a Luchuirp city first.

**AI: Incidental.** Value 50, under the rules' gate (which, with the rule against taking them from
Barnaxus, never strips the golem).

### Take Pieces of Barnaxus (`SPELL_TAKE_PIECES_OF_BARNAXUS_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the pieces Barnaxus dropped.

**Playing it well.** Recover them where he fell and rebuild him.

**AI: Partial.** Value 51, so an AI unit standing on them picks them up, but no AI ever casts Rebuild
Barnaxus (above): the pieces are carried for nothing.

### Take Potion of Invisibility (`SPELL_TAKE_POTION_OF_INVISIBILITY_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for a Potion of Invisibility; no potion appears in a
normal game (see Potion of Invisibility).

**Playing it well.** Nothing to play.

**AI: Incidental.** Value 50, under the rules' gate; moot.

### Take Potion of Invisibility (`SPELL_TAKE_POTION_OF_INVISIBILITY_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up a Potion of Invisibility; none is ever created.

**Playing it well.** Nothing to play.

**AI: Partial.** Value 51; moot.

### Take Potion of Restoration (`SPELL_TAKE_POTION_OF_RESTORATION_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for a Potion of Restoration; none is ever created.

**Playing it well.** Nothing to play.

**AI: Incidental.** Value 50, under the rules' gate; moot.

### Take Potion of Restoration (`SPELL_TAKE_POTION_OF_RESTORATION_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up a Potion of Restoration; none is ever created.

**Playing it well.** Nothing to play.

**AI: Partial.** Value 51; moot, and the AI would never drink it (value -1).

### Take Rod of Winds (`SPELL_TAKE_ROD_OF_WINDS_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Rod of Winds (+1 affinity for Air mana; from
lairs and Treasure Chests).

**Playing it well.** It belongs on an Air caster, whose spells it strengthens.

**AI: Incidental.** Value 50, under a gate that sends it the wrong way: a caster is rarely 3 stronger
than anyone, so the rod drifts to the stack's strongest fighter.

### Take Rod of Winds (`SPELL_TAKE_ROD_OF_WINDS_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Rod of Winds.

**Playing it well.** Have the Air mage pick it up.

**AI: Partial.** Value 51 (affinity is not counted, even for a caster); moot unless an AI captures
one.

### Take Scorched Staff (`SPELL_TAKE_SCORCHED_STAFF_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Scorched Staff (+1 affinity for Fire mana; from
an outcome of the Meteor event).

**Playing it well.** On a Fire caster.

**AI: Incidental.** Value 50, under the rules' gate, which favours fighters over casters.

### Take Scorched Staff (`SPELL_TAKE_SCORCHED_STAFF_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Scorched Staff.

**Playing it well.** Have the Fire mage pick it up.

**AI: Partial.** Value 51 (affinity is not counted); whoever stands there takes it.

### Take Staff of Souls (`SPELL_TAKE_STAFF_OF_SOULS_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Staff of Souls, Ars Moriendi's (the Horseman of
the Apocalypse): +3 death strength and +1 affinity for Death mana.

**Playing it well.** On a Death caster that also fights, or on a strong attacker.

**AI: Incidental.** Value 50, under the rules' gate.

### Take Staff of Souls (`SPELL_TAKE_STAFF_OF_SOULS_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Staff of Souls.

**Playing it well.** Kill Ars Moriendi in the open with a stack and take it with an attacker.

**AI: Partial.** Value 170 (3 death at 40, the spell's 50; the affinity is not counted).

### Take Syliven's Perfect Lyre (`SPELL_TAKE_SYLIVENS_PERFECT_LYRE_BUILDING`)
*Equipment ability (from a building) · in an own city with Syliven's Perfect Lyre · `bAllowAI` 0*

**What it does.** Takes Syliven's Perfect Lyre, a portable world wonder (900 hammers, Drama: +4
culture, +100% culture and 2 Great Artist points in its city), out of the city; Drop Syliven's
Perfect Lyre puts it in another own city. The promotion itself does nothing.

**Playing it well.** Carry it to the city whose culture wins border fights or a cultural victory, or
out of a city about to fall.

**AI: Unused.** `bAllowAI` 0: the Lyre stays where it was built (the generic building value and the
cultural victory pursuit read its culture).

### Take Syliven's Perfect Lyre (`SPELL_TAKE_SYLIVENS_PERFECT_LYRE_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Lyre in transit.

**Playing it well.** Pass it to a safer carrier.

**AI: Incidental.** Value 50, under the rules' gate; the AI holds a Lyre only by picking up one a
dead carrier dropped.

### Take Syliven's Perfect Lyre (`SPELL_TAKE_SYLIVENS_PERFECT_LYRE_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up a dropped Lyre.

**Playing it well.** Recover it and drop it in a city at once: carried, it gives nothing.

**AI: Partial.** Value 51; the carrier drops the wonder only if it happens to end a turn in one of
our cities (Drop is worth the building's value); nothing takes it to one.

### Take Timor Mask (`SPELL_TAKE_TIMOR_MASK_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for the Timor Mask (fear: units on a plot it attacks
scatter; immune to fear; from lairs and Treasure Chests).

**Playing it well.** On the attacker that opens assaults on stacks.

**AI: Incidental.** Value 50, under the rules' gate.

### Take Timor Mask (`SPELL_TAKE_TIMOR_MASK_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up the Timor Mask.

**Playing it well.** As above.

**AI: Partial.** Value 51 (fear is not counted); moot unless an AI captures one.

### Take War (`SPELL_TAKE_WAR_PROMOTION`)
*Equipment ability (from a unit) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** As Take Athame (promotion), for War, Buboes's (the Horseman of the Apocalypse): +3
unholy strength, and each living unit its carrier kills adds 1 to the Armageddon counter.

**Playing it well.** Evil civilizations that want Armageddon carry it into every fight; good ones
should leave it or keep it out of combat.

**AI: Incidental.** Value 50, under the rules' gate; the counter is not weighed.

### Take War (`SPELL_TAKE_WAR_UNIT`)
*Equipment ability (pick up) · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** Picks up War.

**Playing it well.** See above: a good civilization gains 3 strength and speeds Armageddon.

**AI: Partial.** Value 170 (3 unholy at 40, the spell's 50), taken whatever the AI's alignment; the
Armageddon counter is not considered.

### Taunt (`SPELL_TAUNT`)
*Unit ability · the Harlequin (Balseraphs' Ranger, 6/6, 2 moves, needs a Carnival) · range 1,
resistable (+40% to resist) · `bAllowAI` 0*

**What it does.** Every enemy unit within one plot that is alive, can attack, shares the Harlequin's
domain and fails to resist attacks the Harlequin's plot at once (`spellTaunt`); the plot's best
defender takes each attack, whoever it is. It needs one such unit next to the Harlequin.

**Playing it well.** Stand the Harlequin with your strongest defender on a hill or forest next to an
enemy city or stack, then taunt: their units leave their walls and fortification to attack you on
your terms, and the weakened, emptied city or stack is yours to hit on your turn. Unresisted
attacks can still win, so taunt only with a defender that beats what is next to you.

**AI: Unused.** `bAllowAI` 0: the AI's Harlequins fight as the Rangers they replace.

### Upgrade Archer (`SPELL_UPGRADE_ARCHER`)
*Barbarian ability · a Scorpion Clan Goblin becomes a Scorpion Clan Archer · the barbarians know
Archery · free · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** The Goblin (2/2, +1 poison; Goblin Forts spawn them) becomes an Archer (3/5, +1
poison, a first strike, +25% city defense), keeping its promotions and experience. Only the
barbarian player can cast it; the barbarians gain the techs the civilizations know.

**Playing it well.** Against it: clear Goblin Forts before the barbarians know Archery, while they
are guarded by 2-strength goblins rather than archers that hold them.

**AI: Valued.** The barbarian player casts by FfH's choice (`casting::choose_spell`; `magic::enabled`
excludes the barbarians): the conversion is worth 700 - 450 by this AI's unit value (attack plus half
the defense), plus the weight 300, 550. With Horseback Riding also known, Upgrade Wolf Rider (600)
wins, so a goblin then becomes a Wolf Rider instead.

### Upgrade Chariot (`SPELL_UPGRADE_CHARIOT`)
*Barbarian ability · a Scorpion Clan Wolf Rider becomes a Scorpion Clan Chariot · the barbarians
know Trade · free · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** The Wolf Rider (4/4, 3 moves, +1 poison) becomes a Chariot (5/3, 3 moves, +1
poison, 25% withdrawal).

**Playing it well.** Against it: the chariots strike harder but defend worse, so meet them with
defenders on hills and counterattack the stack that hits you.

**AI: Valued.** Barbarian FfH choice: 800 - 750 + 300 = 350, cast as soon as the tech is in.

### Upgrade Spectre (`SPELL_UPGRADE_SPECTRE`)
*Barbarian ability · a Skeleton of level 3+ becomes a Spectre · free · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** A barbarian Skeleton (2/2, +1 death) with three levels becomes a Spectre (3/3, 2
moves, Fear).

**Playing it well.** Against it: kill barbarian skeletons before they gain levels on your units; a
skeleton that has won a few fights is about to become a faster, fear-causing spectre.

**AI: Valued.** Barbarian FfH choice: the conversion term sees no gain (450 against 450: the
skeleton's death damage counts like the spectre's extra strength, and moves and Fear are not in the
unit value), so the weight 300 alone casts it.

### Upgrade to Battlemaster (`SPELL_UPGRADE_DOVIELLO_CHAMPION`)
*Doviello ability · an Axeman-class unit (their Sons of Asena) becomes a Battlemaster · Iron Working ·
80 gold · `bAllowAI` 1, `iAIWeight` 100*

**What it does.** Sons of Asena (4/4) become Battlemasters (6/6), anywhere on the map, keeping their
promotions and experience. It is the Doviello's only way to get Battlemasters from older units: the
Battlemaster has `bDisableUpgradeTo`, so no ordinary upgrade reaches it.

**Playing it well.** The Doviello's war engine: build cheap Sons of Asena, let them gain levels, and
upgrade them in the field the turn Iron Working arrives, mid-campaign, without walking back to
your territory. Keep gold for it before a war.

**AI: Valued.** FfH's terms: 900 - 600 for the conversion, 80 gold at -4 a gold, plus the weight
100: 80, so the end-of-turn choice casts it for every Sons of Asena while the treasury allows, with
no reserve (`units::upkeep` keeps one for ordinary upgrades). A reserve for these spells was tried
and was worse (docs/AI_LOG.md: "spending the treasury on the only upgrade path is right").

### Upgrade to Son of Asena (`SPELL_UPGRADE_DOVIELLO_AXEMAN`)
*Doviello ability · a Warrior-class unit (their Beastman) becomes Sons of Asena · Bronze Working ·
40 gold · `bAllowAI` 1, `iAIWeight` 50*

**What it does.** A Beastman (3/3, +20% city attack, +25% city defense) becomes Sons of Asena (4/4,
+10% city attack), anywhere, keeping promotions and experience; Sons of Asena also have
`bDisableUpgradeTo`, so this is the only upgrade into them.

**Playing it well.** Upgrade every Beastman you can afford before a war: 40 gold is less than the 60
hammers of a new Sons of Asena, and veterans keep their promotions. With Battlemaster next, a
Beastman becomes a 6/6 veteran for 120 gold.

**AI: Valued.** FfH's terms: 600 - 450 for the conversion, 40 gold at -4, the weight 50: 40, positive,
so it is cast for every Beastman the treasury allows at the end of the turn, with no reserve and
whatever the unit's level.

### Upgrade to Warrior (`SPELL_UPGRADE_DOVIELLO_WARRIOR`)
*Doviello ability · a Worker becomes a Beastman · 20 gold · the rules' AI gate: at war, with at least
as many workers as cities · `bAllowAI` 1, `iAIWeight` 25*

**What it does.** A Worker becomes a Beastman (3/3, +20% city attack, +25% city defense).

**Playing it well.** An emergency levy: at war, turn the workers you can spare into fighters on the
spot (20 gold instead of 25 hammers and a turn), and in peace keep them working; a worker costs 75
hammers, three times a Beastman.

**AI: Directed.** FfH's value is 395 (450 for the conversion, -80 for the gold, the weight 25), but
`civs/doviello.rs` sets it to 0 unless our workers outnumber production's target
(`production::worker_target`), and the rules' AI-only gate allows it only at war with as many workers
as cities (docs/AI_ONLY_RULES.md A2); the Beastmen it makes fight (`units::fighters_off_worker_ai`,
since the cast keeps the worker's AI). The ungated cast had made 558 Beastmen from workers in 190
games, idle and counted as workers; with both fixes, workers +1.3 and research +8-13% from turn 300,
share at the end flat (-17, t -0.20; docs/AI.md, "Unique units").

### Upgrade to Warrior (`SPELL_UPGRADE_DOVIELLO_WARRIOR_SLAVE`)
*Doviello ability · a Slave becomes a Beastman · 20 gold · the rules' AI gate as for the worker ·
`bAllowAI` 1, `iAIWeight` 25*

**What it does.** As the worker version, for a Slave (a half-rate worker made from defeated living
units by an enslavement chance, such as the Slavery civic's).

**Playing it well.** Slaves are the better candidates: a slave is a poor worker, and a Beastman is a
real soldier, so convert slaves before workers.

**AI: Directed.** Same value (395) and the same `civs/doviello.rs` hook, which counts our
worker-AI units, slaves included, against the target; the rules' gate counts only Worker-class units
against the cities, so at war with few true workers the AI's slaves stay slaves.

### Upgrade Wolf Rider (`SPELL_UPGRADE_WOLF_RIDER`)
*Barbarian ability · a Scorpion Clan Goblin becomes a Scorpion Clan Wolf Rider · the barbarians know
Horseback Riding · free · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** The Goblin (2/2) becomes a Wolf Rider (4/4, 3 moves, +1 poison, 10% withdrawal),
which can later become a Chariot (Upgrade Chariot).

**Playing it well.** Against it: Goblin Forts become raiding bases once Horseback Riding spreads;
guard workers and isolated cities near them, or take the forts first.

**AI: Valued.** Barbarian FfH choice: 750 - 450 + 300 = 600, ahead of Upgrade Archer (550) when both
are possible.

### Upgrade Wraith (`SPELL_UPGRADE_WRAITH`)
*Barbarian ability · a Spectre of level 5+ becomes a Wraith · free · `bAllowAI` 1, `iAIWeight` 300*

**What it does.** A barbarian Spectre (3/3) with five levels becomes a Wraith (6/6, +3 death, 2
moves, Fear, Vile Touch).

**Playing it well.** Against it: a barbarian spectre that keeps winning becomes a 9-strength wraith;
kill veteran spectres before they reach level 5; holy damage hurts the undead most.

**AI: Valued.** Barbarian FfH choice: 1350 - 450 + 300 = 1200, cast the turn the level is reached.
