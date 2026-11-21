# Raids

This game's own rules, not FFH2's (the owner's design, 2026-11-21). An army parked outside an
enemy city either besieges it (docs/SIEGES.md: it wears the defenders down) or **raids** it: it
takes a share of the city's output as gold. The city's guard answers by rule, without its owner
moving a unit. Marked **(default)**: a value or rule chosen for a gap, open to change when it
plays badly.

## 1. Why

- **A war had one prize.** Going to war made sense only for a player who could capture a city.
  A raid is a smaller reason: an army that cannot take a city still earns outside it, and the
  defender must come out and fight, or lose the yield.
- **Pillaging was whack-a-mole.** One unit on one plot destroyed one improvement for a few gold;
  the defender answered each one by moving units by hand. Now the target is the city, the actor is
  the army, the loss is temporary, and the answer is a standing posture.
- **The sea already works this way** (docs/NAVAL.md, section 4): a region, a standing order, and
  "the patrol finds what enters it".

## 2. Words

- **Countryside** of a city: the land plots whose working city it is (`Plot::working_city`: the
  plots of its radius it holds, worked by a citizen or not), the city's own plot left out.
  `Game::city_countryside(c)`.
- **Fields**: an improvement the work crews build (a build with `city` in `builds.jsonc`) or one
  it grows into (`improvement_upgrade`), standing on a plot of some city's countryside.
- **Guard** of a city: the squads on its plot that belong to its garrison or to an army of its
  owner (docs/ENGAGEMENTS.md, "Squads"). Loose squads (heroes, mages, recon units on their own)
  are not in it **(default)**; a hero its owner placed in a garrison squad is.
- **Raiders** of a city: the armies holding the Raid order on it.

## 3. The Raid order

A land army's standing order (`Army::station`: `Station::Raid(city)`, beside the fleets' Patrol
and Escort; `Game::army_set_station`).

- **Who and where.** A land army of a civilization (the barbarians have no armies) standing on a
  plot of the countryside of a city whose owner its team is at war with. Not a barbarian city
  **(default)**: it has no guard, so it would be free gold. Not a stripped city (section 6).
  `Game::army_raid_refusal(a, c)` says why not.
- **Taking it ends the army's moves** this turn. Its units sleep (they do not wait for orders).
- **It ends** when its owner gives the army a move order (a move mission pushed for its group,
  `MISSION_MOVE_TO`, `MOVE_TO_UNIT` or `ROUTE_TO`; an attack, `group_attack_force`, is one), when
  its owner ends it (`army_set_station(.., Station::None)`), when the army no
  longer stands on that city's countryside, when the war ends, when the city changes hands or is
  gone, when the city is stripped, and when the guard drives the raiders off (section 5).
  `Game::army_raid_city(a)` is the city of a raid that still holds.
- **Raiders are dispersed.** Their units do not besiege the city they raid: they do not stop its
  healing or its Sentinels' repair, and do not count toward investment (docs/SIEGES.md). (Another
  city they stand next to is besieged by them as by any unit.) One army holds one
  order, so an attacker who wants the siege and the raid brings two armies.
- **Raiders live off the land.** An army holding a Raid order is supplied (docs/SUPPLY.md): its
  counter is set to 0 when it takes the order and stays 0 whatever its line **(default)**.

## 4. The posture

Each city has a sally posture (`City::sally`, saved), set by its owner from the city screen:

| Posture | The guard sallies |
|---|---|
| `Never` | never |
| `Favoured` (the default) | when the forecast gives its squad at least `RAID_SALLY_ODDS` (600) per mille to win the battle |
| `Always` | whenever it has a squad that can fight |

The forecast is the rules' own (`engage_forecast`'s sampled battles of the squad that would sally
against the plot, `won_permille`; on a private RNG, no draw of the game's). The AI's cities stay
`Favoured`: the estimate is the rules' code, not an AI heuristic.

## 5. The raid step

At the end of each player's turn (`Game::player_do_raids(p)` in `player_do_turn`, after the
fleets' standing orders and before its gold, research and cities), for each of its cities, in city
order (a stripped city counts down first, section 6; a city with no raiders takes nothing):

1. **Sallies.** For each plot holding raiders of the city, in plot order, the guard fights up to
   as many battles as enemy squads stand on that plot when its sallies begin (the owner's rule: a
   fight between the guard and each squad of raiders, every turn). Before each battle:
   - the guard's squad is the one that fought the fewest battles this game turn, then the
     strongest (`engage_strongest_squad`); no squad that can fight: no battle;
   - the posture decides (section 4); a refusal ends the sallies against that plot this turn.

   The battle is an engagement (docs/ENGAGEMENTS.md) of that squad, in its cells, against the
   plot; the raiders' squad is the engine's choice (fought least, then best). It differs from an
   attack in four ways:
   - **the guard does not move**: its units fight whatever their moves and attacks this turn, pay
     neither, and stay on the city's plot whatever the result (nobody advances);
   - the plot may be any plot of the countryside, next to the city or not; there is no river
     crossing and no landing;
   - **it is fought in the fields**: on open ground, whatever the raiders' plot is, and no unit
     counts its fortification or its hills, feature or terrain modifiers, attacking or defending
     (raiders are spread over the fields: they cannot hold a hill);
   - the raiders that lose are pushed back as any defender that loses outside a city, and **every
     Raid order on that plot ends** (driven off). No more sallies against them this turn.

   A battle the guard loses, or that has no loser, leaves the raiders where they are; the sallies
   go on with the next battle.
2. **The state.** With no raiders left: none. Otherwise the city is **contested** when every
   plot of its remaining raiders was sallied against this turn (at least one battle each), and
   **overrun** when some raiders were not fought.
3. **The take.** The city's raid percent is `RAID_CONTESTED_PERCENT` (10) contested and
   `RAID_TAKE_PERCENT` (50) overrun; 0 with no raiders or when stripped. It is a modifier of the
   city's production and commerce yield rates until its owner's next raid step
   (`City::raid_percent` in `city_base_yield_rate_modifier`: `-50%`, added to the city's other
   modifiers as every percentage is). Food is untouched: a raid starves nobody.
   The raiders gain what the city loses, as gold: the production and commerce yield rates
   without the modifier less with it, split equally between the raiding armies left (the
   remainder to the first by army id), each share raised by its owner's pillage gold modifier
   (`Player::pillaging_gold`, the Raiders trait). `Game::city_raid_calc(c)` gives the parts for
   the tooltips (CLAUDE.md, "a number the rules calculate shows its calculation").

## 6. Stripped

- Each turn a city is overrun adds 1 to `City::raid_turns`; a turn with no take removes 1 (not
  below 0); a contested turn leaves it.
- At `RAID_STRIP_TURNS` (5) the countryside is **stripped** for `RAID_RECOVER_TURNS` (15) turns
  (`City::raid_stripped`, counted down at the start of each of its owner's raid steps): the
  counter returns to 0, every Raid order on the city ends, none can be taken, and nothing is
  taken. The turn that reaches 5 takes (its take is the fifth); the next 15 raid steps take
  nothing.

So a city loses at most 5 turns of half its production and commerce in 20, and the same city is
not farmed.

## 7. Fields are not pillaged

`unit_can_pillage` refuses fields (section 2): a farm, a mine, a cottage or a town on a plot a
city works is raided through its city, by civilizations and barbarians alike. Unchanged: roads
(they carry supply, docs/SUPPLY.md), forts, the cards' field works, mana nodes, sea improvements
(docs/NAVAL.md), and any improvement on a plot no city works (a plantation on a far resource).
The pillage gold of what can still be pillaged is unchanged.

## 8. What the player sees

- **The army.** With an army selected on an enemy city's countryside: the order `Raid <city>`,
  with the forecast take in its tooltip, or the reason it is refused. A raiding army's line says
  `Raiding <city>` (unit panel, Armies screen); `Stop raiding` ends it.
- **The pillage order** on a field is refused with `Fields are raided, not pillaged.`
- **The city.** Its banner and city screen show `Contested`, `Overrun` or `Stripped (N turns)`
  for both sides while the city is visible; the city screen has the raid's line with its
  calculation and the sally posture's selector with the rule of each posture.
- **Messages.** The order taken (both sides); each sally (the battle report); the take each turn
  (both sides, with the gold); driven off; stripped; the order ended and why.

## 9. AI (ffh-ai)

A rules change is not gated on the AI (CLAUDE.md, "AI changes"); what it does:

- **The guard** is the rules' `Favoured` posture.
- **Raiding.** An army at war that stands outside an enemy city it will not assault this turn
  and is not part of an investment in force takes the Raid order, when the city can be raided and
  the guard's forecast against its plot is below `RAID_SALLY_ODDS` (the guard would stay in). It
  moves on when the order ends.
- It no longer pillages fields.

Not done: the AI does not yet go to war in order to raid, and does not send an army to relieve a
raided city beyond what its defence already does against enemy stacks on its land.

## 10. Defines (`rules_defines.jsonc`)

| Define | Value | |
|---|---|---|
| `RAID_TAKE_PERCENT` | 50 | the modifier of an overrun city's production and commerce |
| `RAID_CONTESTED_PERCENT` | 10 | the same, contested |
| `RAID_STRIP_TURNS` | 5 | overrun turns until the countryside is stripped |
| `RAID_RECOVER_TURNS` | 15 | turns a stripped countryside takes to recover |
| `RAID_SALLY_ODDS` | 600 | the `Favoured` posture's bar, per mille |
| `RAID_SALLY_FORECAST_RUNS` | 12 | sampled battles of a sally forecast |

## 11. Not in this change

The owner's discussion of 2026-11-21 also covered these; each is its own build (TODO.md):

- **A raid without a war of conquest**: a war declared for a raid, in which no city changes
  hands, ended by a truce when the raiders leave or are paid.
- **Paying raiders off**: a demand (docs/PORTING.md, "Gifts and demands") a raided AI pays when
  the raids cost more than the tribute.
- **A militia building**: more damage for the guard in a sally, or a militia unit in its squad.
- **A resource's term**: raiding a resource plot takes the resource for some turns.
- **Barbarians**: a barbarian party on a city's land met by the guard under the same rule.
- **The guard's sallies against besiegers**: today the posture acts on raiders only; against a
  besieging army the garrison's units attack by order, as before.
- **Loot other than gold**: slaves, research.

## 12. As built

The rules, the data, the saves and their tests (2026-11-21). The AI (section 9) and the views
(section 8) are separate builds on this API; until the AI build, no AI army takes the order (and
no AI unit pillages fields: the rule refuses it).

### Code map

- `crates/ffh-core/src/raid.rs`: everything below unless named otherwise.
- `army/fleet.rs`: `Station::Raid(CityId)`; `army_set_station` takes it for land armies only
  (Patrol and Escort stay fleet-only); `group_order_ends_station` ends it on a move mission,
  `group_attack_ends_raid` (called at the top of `group_attack_force`) on an attack;
  `player_fleet_stations` runs `army_raid_step` for every army with a Raid order.
- `player/turn.rs`: `player_do_raids(p)` right after `player_fleet_stations(p)`.
- `engagement/stats.rs`: `HitCtx::fields` and `HitCtx::sally(from, to)` (also `HitCtx::at(to)`,
  a context with no approach); `engage_ground` and `engage_combat_terms` read it.
- `engagement/engage.rs`: `SallyForce` and `Game::engage_as(ord, Some(sally))`; `apply.rs` step 8
  skips the attack cost for a sally.
- `engagement/preview.rs`: `engage_sally_forecast` (the forecast on the real sally's set-up).
- `engagement/siege.rs`: `unit_is_besieger` refuses a unit whose army raids that city (investment
  and `city_invested_after_move` follow from `city_besiegers`; a moving selection's order ends, so
  counting it there is right).
- `army/supply.rs`: `player_update_supply` keeps a raiding army's counter at 0.
- `city/yields_culture.rs`: `city_base_yield_rate_modifier` subtracts `City::raid_percent` for
  production and commerce.
- `unit/actions.rs`: `unit_can_pillage` refuses fields; `unit_pillage_refusal`.
- `telemetry.rs`: `tel_raid_*` (docs/TELEMETRY.md: `raid`, `raid_end`, `raid_sally`,
  `raid_take`, `raid_lost`; events `raid_order`, `raid_take`).
- `crates/ffh-harness/src/check.rs`: a fleet never holds Raid, a land army only None or Raid; a
  city's `raid_percent` is 0, contested's or overrun's, its counters in bounds.
- Saves: `Station::Raid` (tag 4), `City::sally`, `raid_percent`, `raid_turns`, `raid_stripped`;
  `FORMAT_VERSION` 62.
- Data: section 10's defines in `rules_defines.jsonc` (`defines_gen.rs` regenerated).
- Tests: `crates/ffh-core/tests/raids.rs`.

### Public API (what the AI and the views call)

```rust
// raid.rs
pub enum Sally { Never, Favoured /* default */, Always }   // Sally::ALL, name(), from_name()
pub enum RaidState { None, Contested, Overrun, Stripped(i32) }
pub struct RaidShare { army: ArmyId, owner: PlayerId, share: i32, modifier: i32, gold: i32 }
pub struct RaidCalc { percent, production_base, commerce_base, production_without,
    production_with, commerce_without, commerce_with, production_lost, commerce_lost,
    total, shares: Vec<RaidShare> }                       // all i32 but `shares`

impl Game {
    pub fn plot_in_countryside(&self, plot: PlotIdx, c: CityId) -> bool;
    pub fn city_countryside(&self, c: CityId) -> Vec<PlotIdx>;
    pub fn improvement_is_fields(&self, i: ImprovementType) -> bool;
    pub fn plot_is_fields(&self, plot: PlotIdx) -> bool;
    pub fn city_guard_squads(&self, c: CityId) -> Vec<FieldSquad>;
    pub fn raid_sally_squads(&self, c: CityId, to: PlotIdx) -> Vec<FieldSquad>;
    pub fn raid_sally_pick(&self, c: CityId, to: PlotIdx) -> Option<FieldSquad>;
    pub fn raid_sally_forecast(&self, c: CityId, to: PlotIdx) -> Option<i32>; // won_permille
    pub fn army_raid_refusal(&self, a: ArmyId, c: CityId) -> Option<&'static str>;
    pub fn army_raid_end_reason(&self, a: ArmyId, c: CityId) -> Option<&'static str>;
    pub fn army_raid_city(&self, a: ArmyId) -> Option<CityId>;
    pub fn unit_raid_city(&self, u: UnitId) -> Option<CityId>;
    pub fn city_raiders(&self, c: CityId) -> Vec<ArmyId>;          // army id order
    pub fn army_raid_step(&mut self, a: ArmyId);
    pub fn city_raid_state(&self, c: CityId) -> RaidState;
    pub fn raid_take_calc(&self, c: CityId, percent: i32, raiders: &[ArmyId]) -> RaidCalc;
    pub fn city_raid_calc(&self, c: CityId) -> RaidCalc;           // the take now
    pub fn army_raid_forecast(&self, a: ArmyId, c: CityId) -> RaidCalc; // overrun, with `a`
    pub fn city_set_sally(&mut self, p: PlayerId, c: CityId, posture: Sally) -> Result<(), &'static str>;
    pub fn player_do_raids(&mut self, p: PlayerId);
}
// army/fleet.rs: Station::Raid(c) to take it, Station::None to end it
pub fn army_set_station(&mut self, p: PlayerId, a: ArmyId, station: Station) -> ArmyResult;
// unit/actions.rs
pub fn unit_pillage_refusal(&self, u: UnitId, plot: PlotIdx) -> Option<&'static str>;
```

`ffh-play`'s `army_station` command takes `{"raid": city}` (`station_json` writes it); the
Armies view's station has `kind: "raid"`, `city`, `text: "Raiding <city>."`. Both are only what
the build needed to compile; the views are the UI build's.

### Decisions beyond the text

- **Refusal texts** (`army_raid_refusal`): `No such army.`, `A fleet does not raid.`, `The
  barbarians do not raid.`, `The army is disbanding.`, `No such city.`, `Barbarian cities are not
  raided.`, `Requires war with the city's owner.`, `The army is not on the map.`, `The army is not
  on the city's countryside.`, `The countryside is stripped.`, `The army raids this city
  already.` An army may take the order with no moves left.
- **Ending reasons** (`army_raid_end_reason`, the messages' and `raid_end`'s words): `the army is
  gone`, `the city is gone`, `the city changed hands` (a captured city is a new `CityId`, so this
  is the city's team becoming the raider's), `the war ended`, `the army left the countryside`,
  `the countryside is stripped`; and by the order's ending: `a move order`, `an attack`, `its
  order` (its owner ended it), `driven off`. An order that no longer holds is not valid at once
  (`army_raid_city` is `None`, the city's raiders leave it out) and is cleared, with its message,
  at its owner's turn end (`player_fleet_stations`). A raiding army destroyed in a sally has no
  "stops raiding" message (the battle report tells it).
- **Moves.** Only a pushed move mission or `group_attack_force` ends the order; a group moved by
  `group_move` without a mission keeps it while it stays on the city's countryside (the AI build:
  give a raiding army's moves as missions, or end the order first).
- **The guard** is built from the plot's squads (`engage_field_squads` of the owner's units that
  stand in squads), keeping the garrison's and the owner's armies' squads (a loose unit the owner
  placed in a garrison squad is in it). A unit can sally when it is alive, can attack, is a land
  unit that is not a missile, a battle summon or a marching recruit, and is within its combat
  limit against the plot; its moves and attacks are not read. "No squad that can fight" also
  covers a plot with nobody the guard can fight.
- **Battles a plot gets**: the number of enemy squads (`engage_field_squads` of the defenders the
  guard would meet, raiders' or not) when that plot's sallies begin. The raiders' squad is the
  engine's choice as in any battle. The sally has no launcher, no field Sentinels
  (`engage_def_plan` with `sentinels: false`) and no frozen forecast in its log; its report's
  `place.river_crossing` is false. Summons the guard's casters call are called as in any battle.
- **The forecast** (`raid_sally_forecast`, the `Favoured` test) runs the real sally's set-up
  through the preview's `engage_plan_finish` (the same defending squad, the same summons, in a copy
  of the game when there are any) and `engage_forecast_sampled` (a private RNG seeded from both
  formations, the plots and the turn): pure, deterministic, no draw of the game's RNG. Its opening
  reads the units of the plot the guard's team sees, as the attack preview does. `to` must hold
  units the guard can fight (an army on its way there gets `None`); they need not hold the order.
  With a summoner on either side the forecast is made in a copy of the game
  (`Game::load(save())`, as the preview does): deterministic, but a cost for a caller that asks
  for every army every turn.
- **Ships on the coast** still fire into a sally (`engage_shore_ships` reads the raiders' plot, as
  for any battle on land): they are not a ground term.
- **Driven off**: every army whose Raid order on the city stood on that plot before the battle
  loses it, whether or not its units were pushed back or fell.
- **The take** is computed with `city_base_yield_rate_modifier`'s `extra`: without = the base
  rate x the modifier with `+raid_percent` given back, with = at `-percent` in its place; lost =
  without - with (production and commerce, each at least 0). The shares go in army id order, the
  remainder to the first; `gold = share + share * pillaging_gold / 100` (at least 0), added to
  the owner's gold directly as pillage gold is. The modifier changes refresh the commerce caches
  and the citizens' assignment (`city_update_commerces`, `ai_assign_work_dirty`); the C++ comment's
  yield-rank cache does not exist in this engine.
- **State**: contested when every raiding army left after the sallies stands on a plot sallied
  against this turn; `city_raid_state` reads it from `raid_percent` (`RAID_TAKE_PERCENT`:
  overrun, any other non-zero: contested), and `Stripped(n)` first.
- **Every city of the player, every raid step** (so a modifier never outlives its raiders),
  barbarians' included (their cities cannot be raided; the step only resets them).
- **Fields** are found once from the data and cached (`OnceLock`, as `city_defender_defense`):
  builds with `city` and their improvement's `improvement_upgrade` chain (Farm, Mine, Workshop,
  Lumbermill, Windmill, Plantation, Quarry, Pasture, Camp, Winery, Cottage to Enclave; the boats'
  and the Pirate Cove's are on water, never a countryside). On a plot with fields and a road the
  road can be pillaged (`unit_pillage` takes the road and leaves the field); `unit_pillage_refusal`
  gives `Fields are raided, not pillaged.` only when the fields rule is what refuses.
- **Messages** (`log.msg_at` at the city): taken: `The 2nd Army raids Vallus.` (the raider) and
  `The Bannor 2nd Army raids Vallus.` (the city's owner); each take: `Raid on Vallus (overrun):
  the 2nd Army takes 14 gold (50% of production and commerce).` (each raider) and `Vallus is
  raided (overrun): -50% production and commerce, 12 lost.` (the owner); ended: `The 2nd Army
  stops raiding Vallus: driven off.` (and the owner's, with the adjective); stripped: `Vallus:
  countryside stripped. No raid for 15 turns.` (both). The sallies' battle reports and messages
  are the engagement's own.

### Departures from sections 2-7

None of substance; sections 2, 3, 5 and 6 above were reworded to say what was built (the
countryside's `working_city`, worked or not; which orders end the Raid; the raiders do not
besiege only the city they raid; the counter zeroed when the order is taken; every city's step;
the battles counted when a plot's sallies begin; the strip's last take and countdown).
