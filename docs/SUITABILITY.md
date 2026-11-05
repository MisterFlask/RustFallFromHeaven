# Building suitability (proposal)

Status: **proposal, not built** (2026-11-05). Nothing here is in the rules yet; the code paths named
are the ones the change would touch.

This proposal adds a roguelike layer of randomness and selection to city building. Each city gets
a few **site traits** (cards with a name), dealt when it is founded and drafted as it grows. A
trait marks two to four related building classes as *Suited* or *Unsuited* in that city. A
building's tier changes what it costs **and** how much it does there. Every building stays
buildable everywhere and does the same kind of thing it does in FFH2. Only its numbers in that one
city change. A Library is still a Library. In one city it is a cheap +38% science building. In
another it is an expensive +12% one. The FFH economy is kept: no building, tech, prerequisite or
effect type is added or removed. A rules option controls the whole layer, and it is off by
default.

## 1. Goals and constraints

- **Keep the FFH economy.** The tech tree, the building list, the prerequisites and the kinds of
  effects all stay. Suitability only scales numbers that already exist. A player who ignores it
  still plays FFH2, a bit less well.
- **Selection, not only randomness.** If every city simply rolls 40 numbers at founding, there is
  randomness but nothing to choose. The player has to choose something. This design gives three
  choices: where to settle (part of the deal can be seen in advance), which card to draft as the
  city grows, and what to build given what the city is good at.
- **Legible.** One look at a city should say what it is: "Ley Confluence, Deep Harbor, Bad
  Water". A spread of 40 small percentages would not. That is why the unit of randomness is a
  named trait, not a roll per building class (section 3).
- **Bounded.** A city has at most about 6 traits, and each touches 2 to 4 classes. So at most about
  a third of the roughly 40 generic building classes ever leave Plain. Most of a city's buildings
  are vanilla.
- **Engine rules hold.** All math is integer. Every random draw goes through the game RNG, and
  the draws happen only when the option is on, so games with it off stay bit-identical. All state
  lives on `City` and is saved.

## 2. Tiers: what suitability does

There are four tiers. Each one sets a cost percentage and an effect percentage:

| Tier | Cost | Effect | Rounding of effects | Source |
|---|---|---|---|---|
| Unsuited | 150% | 50% | down | flaw traits |
| Plain | 100% | 100% | — | default |
| Suited | 75% | 150% | up | traits |
| Exemplary | 50% | 200% | up | a second Suited step (drafts only) |

Steps from all of a city's traits add up per building class and are clamped to Unsuited..Exemplary.
A Suited trait and a flaw on the same class cancel to Plain. The tier values are defines
(`SUITABILITY_COST_*`, `SUITABILITY_EFFECT_*` in `RulesDefines.xml`), so tuning means editing
data.

**Why both cost and effect, on one tier.** A cost-only tier changes the *order* in which a city
builds, but cities converge. By the late game an FFH city has most of the generic buildings, and a
cost difference paid 150 turns earlier no longer shows. An effect-only tier makes a lasting
difference, but leaves the build order flat: the player still builds everything, just valuing
some of it more. Putting both on the same tier makes the two point the same way: a building is
cheap where it works well. That gives the player a clear signal ("build into this city's
strengths"), and the AI follows it with no special code (section 6). Two simpler variants exist:
effect-only, or cost-only as a first phase. Section 8 asks the owner to choose. I recommend both.

**Why no "cannot build" tier.** A ban is the strongest selection pressure, but it falls on the
wrong buildings. A frontier city that may not build Walls, or a city with no happiness building at
all, gets punished for terrain the player could not see. Unsuited (150% cost, 50% effect) already
says "probably skip this here" and still leaves a way out.

### 2.1 Which numbers scale

Effects apply additively in `city_process_building` (`crates/ffh-core/src/city/buildings.rs`):
each info value is multiplied by `change = ±1`. Suitability scales the city-local, additive,
**positive** numbers:

- Scaled: `yield_change`, `yield_modifier`, `sea_plot_yield_change`, `river_plot_yield_change`,
  `commerce_modifier`, `commerce_change` and `obsolete_safe_commerce_change` (on the read side,
  in `city_building_commerce_by_building`), positive `happiness` and `health`,
  `specialist_count`, `free_specialist_count`, `great_people_rate_change` and `_modifier`,
  `free_experience`, `unit_combat_free_experience`, `domain_free_experience`, `food_kept`,
  `defense_modifier`, `bombard_defense_modifier`, `heal_rate_change`, `trade_routes`,
  `trade_route_modifier`, `military_production_modifier`, `domain_production_modifier`,
  `resist_magic`.
- Not scaled:
  - Negative entries: the Market's -1 research and the Forge's -1 health stay as they are, so a
    Suited Forge does not get unhealthier.
  - Yes/no and one-off effects: `power`, `government_center`, free bonuses, `no_bonus`,
    `plot_radius`, `free_promotion`, founding a corporation, `religion_change`.
  - Effects outside the city: everything in `player_process_building`,
    `team_process_building`, `process_building`, area happiness, `asset_value`, `power_value`.
  - Building-class changes from civics and traits (`city_building_yield_change`): they are
    added after scaling.
- Code effects: some buildings have effects implemented in code, listed in the table in
  `crates/ffh-ai/src/code_effects.rs` (Cave of Ancestors experience, Planar Gate summon chance,
  ...). These scale where the effect has a natural number. The Warrens' copy of each unit does
  not scale.

**Rounding.** `scale(v, pct)` returns `v` when `v <= 0`. Otherwise it rounds up when `pct > 100`
and down when `pct < 100`. This matters because FFH building values are small integers. With
plain truncation, a Suited +1 health would stay +1 and the tier would do nothing to it. With this
rule, small flat values move by at least one step: +1 becomes +2 when Suited and 0 when Unsuited.
Percentages move smoothly: the Library's +25% becomes +38% or +12%.

Rounding up has a cost: it doubles a +1. The worst case is the Harbor's +1 food per sea tile,
which becomes +2 food per sea tile when Suited. That is a large swing. Each trait can therefore
list fields it leaves unscaled (`ScaleExcludes`). The first data pass should apply it to the
per-plot yield changes and keep them at +1.

**Invariant.** The tier used to add a building's effects must be the tier used to remove them, so
a city's tiers never change while a building stands, except through one helper. When a draft
changes a class's tier, `city_rescale_building(c, b, old, new)` removes the scaled part at the old
tier and adds it back at the new one. To make that possible, `city_process_building` is split into
`city_process_building_scalars` (the scaled list above, which takes a tier) and the rest
(unchanged). Removal through sale, destruction or obsolescence already goes through
`city_process_building`, so it uses the same tier with no further work.

### 2.2 Cost

Cost is multiplied in `city_production_needed_building`
(`crates/ffh-core/src/city/production.rs`), after the Python cost mod. It is not added to the
additive pool in `city_production_modifier_building`, for two reasons. In that pool, a -25% would
be diluted by traits and bonuses, and "this costs 75% here" would stop being true as stated. And
because hurry cost reads production needed, rushing an Unsuited building also costs 150%, which
is consistent. Hammers already invested stay when a draft changes the tier in the middle of a
build. Only the target moves.

## 3. Site traits: the unit of randomness

A site trait is a named card. It has:

- a list of building classes, each with a step (+1 or -1);
- an **affinity** to the site: how strongly the city's surroundings at founding call for it. This
  is computed from the plots in the city's radius, using terrain, features, bonuses, mana nodes,
  river, coast and hills, each with a weight;
- a draft weight, and a flag for flaws.

The data goes in a new `data/xml/Gameinfo/CIV4CityTraitInfos.xml`, read by `ffh-data`
`infos/city_trait.rs`, alongside the governor trait files. Trait names and descriptions are data,
so they may have flavor. Their tooltips are bare text (section 7).

A first set to size the idea. The data pass would aim for about 20 traits, roughly a third of them
flaws:

| Trait | Classes | Affinity |
|---|---|---|
| Deep Harbor | Harbor, Shipyard, Lighthouse +1 | coastal, sea bonuses |
| Ley Confluence | Mage Guild, Alchemy Lab +1 | mana nodes in radius |
| Old Stones | Walls, Palisade, Dungeon +1 | hills, rock |
| Crossroads | Market, Moneychanger, Inn +1 | river, luxury bonuses |
| Fertile Plain | Granary, Herbalist, Smokehouse +1 | food bonuses, grassland, floodplains |
| Wild Country | Hunting Lodge, Stable, Grove +1 | forest, jungle, animal bonuses |
| Hallowed Ground | Pagan Temple, Monument, religious temples +1 | random (no site term) |
| Scholars' Seat | Library, Elder Council +1 | random |
| Forge Hills | Forge, Siege Workshop, Archery Range +1 | hills, metal bonuses |
| Bad Water *(flaw)* | Aqueduct, Public Baths -1; Herbalist +1 | marsh, no fresh water |
| Restless Folk *(flaw)* | Courthouse, Theatre -1; Training Yard +1 | random |
| Exposed *(flaw)* | Walls -1; Archery Range +1 | flat, open land |

Some flaws come with a small compensating step ("Bad Water" makes the Herbalist Suited). A flaw
then still points somewhere, which keeps it from being a pure tax.

**Scope.** Traits cover the generic, non-wonder building classes: about 40 of the 68
`BuildingInfo` rows with no instance limit, counted from `CIV4BuildingInfos.xml`. They leave
out:

- world and national wonders (phase 3 at the earliest; section 8);
- the civilization classes `BUILDINGCLASS_CIV_BUILDING1-3`, since a unique building is part of
  a civilization's identity, not its site's;
- the cost -1 rows (cages, vaults, the Dancing Bear and the like).

A civilization's unique replacement for a generic class, such as the Governor's Manor for the
Courthouse, inherits the class's tier, because the tier is stored per class.

## 4. When traits are dealt: where the selection is

**At founding: two traits.**

1. The **site trait** can be seen in advance. It is the trait with the highest affinity among
   those the site qualifies for, with ties broken by a hash of the game seed and the plot. The
   preview is deterministic and uses no RNG, so a settler's plot preview can show it ("Site:
   Deep Harbor"). Moving one tile changes it only when the city's surroundings change enough to
   matter. Choosing where to settle is the first selection.
2. The **founding draw** is the gamble. One trait is drawn with `soren_rand`, with each trait
   weighted by draft weight × affinity. About 40% of the time it is a flaw. It cannot be seen in
   advance.

**As the city grows: drafts.** When a city first reaches population 4, 8, 12 and 16 (high-water
marks, so shrinking and regrowing does not deal again), it is offered **3 traits** drawn by the
same weighting, flaws excluded. The owner picks one, or takes a **refinement** instead:

- raise one Suited class to Exemplary, or
- cancel one flaw step.

An offer that is not answered waits. It does not expire, and the city keeps growing.

This draft is the roguelike part. Offers arrive at random, choices add up, and by the mid-game
each large city has a build. A city can reach 6 traits only by growing to 16, so the variety goes
where the investment went. The system fits players like the Kuriotates, with few large cities:
they get the most drafts per city.

**Captured cities keep their traits.** Traits belong to the site, not the owner. A city with
Exemplary classes is a target worth taking, which fits the conquest drive.

**Expected effect on the economy.** Drafts are pure upside, so with no counterweight the system
makes the economy richer. Founding flaws are the counterweight. Calibration target: with the
option on, the median city's total building output at turn 200 should be within ±5% of what it is
with the option off. Specialization should change *where* output comes from, not how much there
is. The measure is listed in section 9.

## 5. Engine changes

- **Option:** `GAMEOPTION_BUILDING_SUITABILITY` in `CIV4GameOptionInfos.xml`, visible, checked by
  `g.suitability_on()` in the same way as `governors_on()`. When it is off, nothing runs and no RNG
  draws happen.
- **Data:** `CityTraitInfo` (`Type`, `Description`, `bFlaw`, `iDraftWeight`, `BuildingClassSteps`,
  `Affinities` as terrain, feature, bonus or flag with a weight, and `ScaleExcludes`). The defines
  for the tier percentages, the draft population thresholds and the offer size go in
  `RulesDefines.xml`.
- **State on `City`:**
  - `traits: Vec<CityTraitType>`;
  - `refinements: Vec<(BuildingClassType, i8)>`;
  - `draft_high_water: i32`;
  - `draft_offer: Vec<CityTraitType>`;
  - a derived cache `suitability: IdxVec<BuildingClassType, i8>`, rebuilt on load and kept out of
    the save, or saved; either way it is checked by the harness.

  Each of these goes in the `persist_struct!(City { .. })` list in `save/state.rs`, and
  `FORMAT_VERSION` is bumped.
- **Hooks:**
  - `player_init_city` deals the site trait and the founding draw, like the governors' hook
    there;
  - the population-change path offers drafts;
  - `city_production_needed_building` applies cost (section 2.2);
  - `city_process_building` is split and `city_building_commerce_by_building` scales the commerce
    it reads (section 2.1);
  - a new command `ChooseCityTrait { city, pick }` answers an offer;
  - the controller gets `choose_city_trait(g, c, offer) -> pick`, with a default for headless
    games: the AI policy.
- **Harness invariant:** `ffh_harness::check` recomputes every city's scaled building
  contributions from scratch and compares them with the incremental fields. This catches a
  forgotten rescale or a mismatched tier between adding and removing a building, which is the
  most likely bug class.
- **Docs:** list the rule in docs/PORTING.md under "Changes from FFH2"; once built, this file
  becomes the record of what was built, like docs/CRUSADE.md.

## 6. AI

This is a rules change, so under the AI policy it is not gated on the AI: the AI just needs to use
the system reasonably. Most of that comes for free:

- **Building choice.** `production.rs` already ranks buildings by value over
  `city_production_turns_left_building`, so cheaper Suited buildings rise and expensive Unsuited
  ones sink with no change. The one edit: in `building_value`, multiply the scaled terms by the
  effect percentage. In practice that means wrapping the generic estimate's per-field terms in
  the same `scale()`, so value and rules cannot drift apart.
- **Settling.** `player_ai_found_value` adds a small term for the previewable site trait: its
  classes' summed building value at a nominal population. Most of a site's value stays with food,
  land and resources. The term should be large enough to break near-ties, no larger.
- **Drafts.** Pick the offer, or the refinement, with the largest gain in summed building value
  over the classes the city has built or can build within the next era. For a governed city
  (docs/GOVERNORS.md), weight the pick by the governor's profile: a Scholar leans to science
  classes, a Marshal to military ones. That makes a governor's "taste" visible through the city's
  growth.
- **Local check** (the AI change process): count how much of each city's building production goes
  to Suited and Unsuited classes, with the option on. The expected sign is a clear lean toward
  Suited classes. If the AI builds Unsuited classes as often as Suited ones, the value scaling is
  not reaching the chooser.

## 7. Interface

The rules text is bare, as the house rule requires:

- **Build list:** each building row carries its tier when it is not Plain:
  `Suited: cost 75%, effect 150%.`
- **Building tooltip:** the scaled numbers, then the source: `+38% research (Plain: +25%).
  Source: Scholars' Seat.`
- **City screen:** a traits strip listing the trait names. Each trait's tooltip lists its classes
  and steps.
- **Draft offer:** a popup on the city, also queued in the turn's to-do list, with three trait
  cards and the refinement choices. Each card lists `Library, Elder Council: Suited.`
- **Settler preview:** `Site trait: Deep Harbor.` and `Founding draw: unknown.`

## 8. Decisions for the owner

1. **Cost, effect or both.** I recommend both on one tier (section 2). The cheaper first phase is
   cost only: about one function, no change to `processBuilding`. It would show whether the
   variety feels good before the larger effect change.
2. **Can the site trait be seen before settling?** I recommend yes, since it is the settling
   decision's selection. The alternative is all hidden, which is pure gamble at founding.
3. **Draft trigger: population or culture level.** I recommend population (4, 8, 12, 16), since
   every city grows. A culture trigger would reward culture cities twice.
4. **Wonders.** Leave them out at first. National wonders, such as the national epic, are a
   natural later extension: "where do I put it" is already a choice, and suitability would make it
   a better-informed one.
5. **Default for the browser's new-game screen.** Unticked, until the measures in section 9 look
   right.

## 9. Phases and measures

| Phase | What | Check |
|---|---|---|
| 1 | Option, data file with about 20 traits, founding deal, cost tier, save, city screen strip | gate; games with the option off are bit-identical to the current binary (sync checksums, as for crusades) |
| 2 | Effect tier: split `processBuilding`, commerce read side, rounding, rescale helper, harness invariant | invariant clean over the stress matrix; AI `building_value` scaled |
| 3 | Drafts: offers, refinement, command, popup, AI and governor pick | local AI check (section 6); calibration at turn 200 within ±5% |
| 4 | Settler preview of the site trait, `found_value` term | the preview shows the trait the city then gets: same function, tested |

Measures, over seeds, with the option on against off:

- median and spread of per-city building output at turns 100 and 200;
- the share of building production spent on Suited and on Unsuited classes, for the AI and for
  bots;
- the number of distinct "city identities", meaning sets of Suited classes, per civilization;
- the share of flaws drawn at founding: the calibration knob for the ±5% target.
