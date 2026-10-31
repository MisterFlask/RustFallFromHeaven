# Engagement combat

This game's own land combat (not FFH2's), behind `GAMEOPTION_ENGAGEMENTS` (which requires
`GAMEOPTION_ARMIES`; off by default while phase 6 of the owner's spec "Engagement combat" is being
built). When an army attacks a land plot, each side commits up to its slots' worth of units, which
fight in rounds until one side's committed force is destroyed or has withdrawn. The design is the
phase 6 plan; the owner's answers on every FfH combat ability (Q1-Q13) and the ground rules G1-G5
are in docs/COMBAT_INVENTORY.md; the battle log the client replays is the battle UI design's.

With the option off nothing here runs: every attack is today's duel, byte for byte (checked at
every step: per-turn checksums of `ffh autoplay` against master). Naval and air combat stay the
duel with the option on too.

Code: `crates/ffh-core/src/engagement/` (`mod.rs` lists the modules: formation battles' pure
resolver in `formation/`; the round model in
`battle.rs`, `resolve.rs` and `expect.rs`; the rules in `state.rs`, `stats.rs`, `slots.rs`,
`pick.rs`, `engage.rs`, `apply.rs`, `sink.rs`, `report.rs`; the estimator in `estimate.rs` and the
attack preview in `preview.rs`). Scenario tests: `crates/ffh-core/tests/engagements.rs`; the
estimator's and the preview's: `tests/engagement_estimate.rs`, `tests/engagement_preview.rs`; the
round model's tests are in its modules.

## Formation battles (revision 4): the data and the resolver

The owner replaced the round model below with Ogre Battle-style formation battles (2026-10-31,
designed in a browser prototype). Built so far: the data and the pure resolver
(`engagement/formation/`); the rules still fight the round model until they move to it. Where this
section and the rest of this document disagree, this section is the design and the rest is the
round model's.

**The field.** Slots per side as before (`slots.rs`). Each side stands on a grid of 3 rows (front 0,
middle 1, rear 2) by `max(2, ceil((slots + 2) / 3))` lanes (`formation::lanes_for`). A large unit
(`bLarge`, below) fills two cells side by side in one row and counts 2 slots, has
`ENGAGEMENT_LARGE_HP_PERCENT` (150) of its role's HP, and is the front or back unit of both its
lanes.

**Roles** (`CIV4CombatRoleInfos.xml`; the round model's `bBack`, `bScreen`, `bIgnoresShield` and
`TargetOrder` are gone, and Backline is Archer and Siege now, Support Arcane and Divine):

| Role | defaults (unit combat) | A% | Dv% | HP | speed | Front | Middle | Rear | other | home row | auto pick |
|---|---|---|---|---|---|---|---|---|---|---|---|
| FRONTLINE | MELEE, ANIMAL, BEAST, RECON, bDefault | 100 | 100 | 120 | 4 | 2 strike | 1 strike | 1 strike | | front | 2 |
| SCREEN | FRONTLINE + Formation | 100 | 110 | 120 | 3 | 2 strike | 1 strike | 1 strike | anti-flank | front | 1 |
| FLANKER | MOUNTED | 110 | 80 | 100 | 6 | 2 charge | 1 strike | 1 strike | dodge 10 | front | 4 |
| ASSASSIN | RECON with Marksman free | 120 | 60 | 60 | 7 | 1 stab | 2 stab | 1 stab | dodge 15, crit +15 | middle | 8 |
| ARCHER | ARCHER, NAVAL (Airship) | 100 | 60 | 80 | 4 | 1 shoot | 2 shoot | 2 shoot | | middle | 3 |
| SIEGE | SIEGE | 100 | 60 | 80 | 1 | none | 1 bombard | 2 bombard | | rear | 7 |
| ARCANE | ADEPT | 100 | 50 | 70 | 3 | 1 strike | 1 spell | 2 spell | | rear | 5 |
| DIVINE | DISCIPLE | 100 | 50 | 70 | 3 | 1 strike | 1 heal | 2 heal | | middle | 6 |

Units of tier `ENGAGEMENT_EXTRA_ACTION_TIER` (4) and above and heroes get
`ENGAGEMENT_EXTRA_ACTIONS` (1) more actions in their best row, the row with the most (the first
such). Speed is the unit's (below) plus `ENGAGEMENT_FIRST_STRIKE_SPEED` (2) per first strike and per
chance first strike rolled at the battle's start. A flanker is a unit whose rows have a CHARGE. The
home row and the auto-pick order are the rules' auto formation's (`formation::auto_arrange`, the
prototype's `autoArrange`), always the role's.

**Per-unit values** (the owner's request, 2026-10-31). A unit type may replace its role's values in
`CIV4UnitInfos.xml`; each falls back to the role's when absent, and no unit sets them yet (the
content is the owner's to decide):

| Tag | Replaces | Absent / -1 | Valid |
|---|---|---|---|
| `CombatRole` | the role its unit combat gives it | that role | a role |
| `iAttack`, `iDefense` | the base of A (`iCombat` x the role's A%), of Dv (`iCombatDefense` x Dv%) | 0: the role's | above 0 |
| `iHP` | the role's max HP | 0: the role's | above 0 |
| `bLarge` | one cell | 0: one cell | |
| `iEngageSpeed` | the role's speed | -1: the role's | 0 and above |
| `iEngageDodge` | the role's dodge | -1: the role's | 0..=95 |
| `iEngageCritBonus` | the role's crit bonus | -1: the role's | 0..=95 |
| `EngageRows` | the role's rows | the role's | three `Row`s |

`EngageRows` holds three `Row` elements, front, middle and rear, each an `Action` (STRIKE, CHARGE,
STAB, SHOOT, SPELL, BOMBARD, HEAL, NONE) and its `iCount` (note: `iCount` here, `iActions` in the
roles' `FrontRow`, `MiddleRow`, `RearRow`):

```xml
<EngageRows>
    <Row><Action>SHOOT</Action><iCount>1</iCount></Row>
    <Row><Action>STAB</Action><iCount>2</iCount></Row>
    <Row><Action>NONE</Action><iCount>0</iCount></Row>
</EngageRows>
```

The rows' rules are the roles': a count above 0 exactly when the action is not NONE, and some row
acts. A value outside its range, rows that are not three, an unknown action or rows that break those
rules warn at load and leave the unit its role's value. The best row (the extra action's), whether
the unit flanks, and its actions come from its own rows when it has them. The type's values hold
whichever role the unit has (a Frontline type with Formation is a Screen, with the type's own speed
if it sets one). One accessor per value resolves the unit type, then the unit's role:
`Game::unit_engage_speed`, `unit_engage_dodge`, `unit_engage_crit_bonus`, `unit_engage_rows`,
`unit_engage_best_row`, `unit_engage_actions` (with the extra action), `unit_engage_flanks`,
`unit_engage_max_hp` (stats.rs), over `GameData::engage_speed`, `engage_dodge`, `engage_crit_bonus`,
`engage_rows`, `engage_best_row`, `engage_extra_actions`, `engage_actions`, `engage_flanks` (by unit
type and role); the rules' fighters, the auto formation, the preview, the view and the client read
these, not the role's fields. Test: `crates/ffh-data/tests/engagement_overrides.rs` (a fixture of the
game's data with units given overrides, good and bad).

**Action kinds and reach** (`ffh_data::ActionKind`):
- STRIKE: the front unit of each enemy lane; physical; a crit knocks the target back.
- CHARGE: where flanking works (the defended plot has no city and no forest or jungle), every
  enemy on the field, the target the one with the highest threat for its HP left (threat: its A
  times its actions from its current row; compared `threat_x x hp_y`), and in the first round
  `ENGAGEMENT_CHARGE_BONUS` (100) % more damage. Elsewhere it is a STRIKE (front reach, no bonus,
  the normal choice; the log says strike). Physical; a crit knocks back.
- STAB: any enemy. Physical.
- SHOOT: the backmost unit of each enemy lane. Physical, `ENGAGEMENT_SHOOT_MISS_PERCENT` (10) more
  miss.
- SPELL: the backmost unit of each enemy lane; never misses, never crits. A caster that knows a
  damage spell (`GameData::engage_damage_spell`: of the spells with `iDamage > 0` whose promotion,
  unit, unit class and unit combat prerequisites it meets, the highest `iDamage`, then the lower
  id) casts it at power 100 with a splash; one that knows none casts the magic bolt,
  `ENGAGEMENT_BOLT_PERCENT` (60), one target.
- BOMBARD: the backmost unit of each enemy lane; physical, `ENGAGEMENT_BOMBARD_MISS_PERCENT` (15)
  more miss; never crits.
- HEAL: the most hurt ally on the field (most HP missing; ties by lane, then row), itself
  included: `ENGAGEMENT_BASE_DAMAGE x ENGAGEMENT_HEAL_PERCENT (50) x r`. No one hurt: the action is
  spent.
- Splash: a unit with collateral damage splashes its damaging hits (not a miss) on the target's
  neighbours (beside it in its row, in front and behind in its lanes) for
  `ENGAGEMENT_SPLASH_PERCENT` (50) % of a hit on each, never taking one past its collateral
  limit; a known spell splashes the same way (no limit). Splash hits never crit.
- Siege (combat limit below 100): its hits never take a target below `(100 - limit)%` of its max
  HP, and such targets are not in its reach. (Both sides' engines: the resolver applies a
  fighter's limit whichever side it is on; the rules decide what they pass.)
- Anti-flank: whatever a flanker can reach (with any of its actions), units of an anti-flank role
  in that reach are taken first.

**Order.** `ENGAGEMENT_ROUNDS` (3) rounds; in each, pass k (k from 1 to the most actions any unit
on the field has from its row at the round's start) lets every unit on the field with at least k
actions from its current row act once. Within a pass the faster act first, equal speeds by a coin
each mover draws at the pass's start (`0..65535`: the prototype's `2^20` does not fit the game's
CvRandom; equal coins keep fighter order). One unit acts at a time and each hit lands at once; a
unit killed, withdrawn or knocked to a row with fewer actions before its turn in a pass does not
act in it. The battle ends after the last round, or at once when a side has no unit left on the
field.

**Targeting** (everyone, the player does not choose; `Formation::choose`): a flanker's pool is the
anti-flank units in its reach when there are any; a charge where flanking works takes the highest
threat for its HP left; otherwise, if this hit's expected damage would kill some, the one with the
highest A among them, else the one it hurts most for its HP left (expected / HP, by
cross-multiplication). The expected damage is capped at what the hitter's combat limit lets it
take, so a siege engine takes no kill. Ties by the higher A (the unit's A against nobody in
particular), then lane, then row, never by list order. A splash's centre is chosen so too.

**Hits.** Expected damage, centi-HP: `ENGAGEMENT_BASE_DAMAGE (40) x 100 x A / (A + Dv)` (an even
hit 20 HP; six kill a frontline unit), then times the hit's percentages (a spell's power, the
ground's, the first round's charge, a splash's) over one division. A hit: `max(1, expected x r)`, r
in `ENGAGEMENT_DAMAGE_RAND_MIN..=MAX` (80..120) %. A physical action misses with
`ENGAGEMENT_MISS_PERCENT` (10) + the kind's extra + the target's dodge (at most 95), rolled
before the crit and the damage. Strike, charge, stab and shoot crit with `ENGAGEMENT_CRIT_PERCENT`
(10) + the hitter's crit bonus (at most 95), for `ENGAGEMENT_CRIT_DAMAGE_PERCENT` (150) % of
the hit; a strike's or a charge's crit knocks the target back a row when every cell behind it is
free (it acts from that row then).

**Withdrawal** replaces a killing hit: a unit with a withdrawal chance that a hit (a splash too)
would kill rolls it; on a success the hit does nothing and the unit leaves the battle alive. Never
a city's defenders (the resolver enforces it).

**The ground** (the defended plot's; several multiply): a city: the attacker's SHOOT deals
`ENGAGEMENT_CITY_ATTACKER_SHOT_PERCENT` (50) %; a forest or jungle: SHOOT and BOMBARD deal
`ENGAGEMENT_FOREST_RANGED_PERCENT` (50) % on both sides; hills: every attacker hit (spells too)
deals `ENGAGEMENT_HILLS_ATTACKER_PERCENT` (75) %, the defender's SHOOT and BOMBARD
`ENGAGEMENT_HILLS_DEFENDER_RANGED_PERCENT` (125) %.

**Large units** (`bLarge` in `CIV4UnitInfos.xml`): the spec's Hill Giant, War Elephant and Elephant,
the dragons (Abashi, Acheron, Drifa, Eurabatres), every golem (Bone, Clockwork, Flesh, Iron,
Mithril, Mud, Nullstone, Wood) and the Treant, and of that size class the giant heroes Gurid,
Margalard and Wilboman (a frost giant), Barnaxus (a golem), the War Machine, the Earth Elemental (a
golem of rock) and the War Tortoise. Demons (the Balor, the Pit Beast) and the great beasts (the
Giant Spider, the Manticore) stay one cell.

**RNG**, in this order and nowhere else (the module comment of `formation/mod.rs`): per battle,
each fighter with chance first strikes, in fighter order ("Engagement first strikes",
`0..=chance`); per pass, each mover's coin in fighter order ("Engagement order"); per action, a
physical one's miss ("Engagement miss"), if it lands the crit roll of strike, charge, stab and shoot
("Engagement crit"), r ("Engagement hit"), the target's withdrawal if the hit would kill it
("Engagement withdraw"), then each splashed neighbour's r and withdrawal in (lane, row) order; a
heal's r ("Engagement heal"). An action with no target and a heal with no one hurt draw nothing.

**The resolver's API** (`engagement::formation`, pure: nothing of `Game`): `Formation::new(params,
ground, lanes, fighters, pair)` (each `Fighter`'s side, tag, lane, row, large, max HP and HP, its
speed, first strikes and chance first strikes, actions per row, A against nobody, dodge, crit bonus,
anti-flank, flanker, withdrawal chance, combat limit, collateral limit, spell; the pair table of A
and Dv from `pair(h, t)`), `Formation::validate` (inside the grid, one unit a cell), `Params` (the
defines: `Params::from_defines`), `Ground`; `resolve(&formation, rng)` / `resolve_quiet` give an
`Outcome`: each fighter's `End` (HP, final row, speed, the first strikes it rolled, its fate:
standing, fell by whom in which round, withdrew in which round; damage dealt, taken, healed), the
kills in order, the damage matrix (`dealt(h, t)`, battle XP's shares), the rounds fought, which
sides were cleared, and the events (`Event`: round, pass, actor, kind as fought, spell, target,
hits `{target, dmg, expected, r, crit, miss, killed, withdrew, splash}`, heal, knock-back, note).
`auto_arrange(candidates, lanes, slots)` is the rules' auto formation. Its tests:
`formation/tests.rs`; the data's: `crates/ffh-data/tests/engagement_data.rs`.

**What is left of the round model.** It fights on (the rules, the estimator, the preview and the
battle log are the round model's) with its own constants (`engage_params`: B 20, 40 rounds,
withdrawal below 40%; the defines `ENGAGEMENT_MAX_ROUNDS` and `ENGAGEMENT_WITHDRAW_HP_PERCENT` are
gone) and its view of the new roles (`battle::RoundRole`: a role whose best row shoots, bombards,
casts or heals is back position, an anti-flank role a screen, a role that charges or stabs ignores
the shield), and the stances (`CIV4StanceInfos.xml`, whose Backline steps name Archer and Siege, and
Support Arcane and Divine). Not built yet (the rules' part): A and Dv without the plot's own
defence, the large units' HP and slots in the rules, the formation in the `engage` order and the
auto formation in the rules, the result (the plot falls only when no defender is left), the
report, the preview and the battle log of formation battles, and removing the stances, the round
model, the estimator and the What-if list.

## Which attacks are engagements

`Game::engage_applies(u, to)`: the option is on, the attacker is a land unit (one carried on a
transport counts: an amphibious attack) and `to` is land. The rule is applied at each entry:

- `group_attack_force(g, x, y, flags, force)`, the one entry for every order of a group into an
  enemy plot (the human `move` and the coming `engage` command, AI and barbarian missions, the C++
  AI benchmark); `group_attack(g, x, y, flags)` is it with no force. Today's preamble (the path step
  for `MOVE_THROUGH_ENEMY`, the AI's pre-attack spells and bombard-first), then for an engagement:
  no candidate defender there means today's attack (it captures non-combatants, if any); Alertness
  for a human order without a force; the AI's war declaration as today; the `doCombat` hook; then
  one engagement launched by the group, or for a group of missiles only one missile strike per
  missile. The stack-attack loop, the sacrifice choice and `PLAYEROPTION_STACK_ATTACK` are not used.
  Each order into an enemy plot is one engagement; a follow-up is a new order.
  `group_attack_plan(g, x, y, flags, force, stance)` is the same entry with the stance the
  attacker chose for this attack (the human's battle plan and its stance picker, through the
  `engage` command): that stance proposes the force when none is given, a given force is judged
  against its pick (the log's `picked_by`), and the report records it as the attacker's stance;
  `group_attack_force` passes none (the attack's own stance: its army's, ...). A call parameter
  (`EngageOrder::stance`), not state: nothing is saved.
- `unit_attack(u, to)` (Taunt, the Shrine of Camulos, Gift of Kylorin): a one-unit engagement,
  forced (whatever the unit's moves and attacks this turn, as today's `attack`), or a missile's
  strike; with only non-combatants there, today's attack.

## The engagement

`Game::engage` (`engage.rs`), in this order (its RNG draws in this order, each with its label):

1. **Set-up.** Each side's slots (`slots.rs`: 10 a side on open flat land, the plot's change, a
   city's defense costing the attacker one per 25%, the river's two that only `bRiver` units fill).
   The attacker's pool: every unit of the player on the attacking plot that can attack the plot,
   across its armies, garrison, heroes, mages and recon units; its force: the order's (checked) or
   the stance's proposal (its quota pick improved by single swaps: "The auto-pick consults the
   estimate", below). The defender's candidates: every unit on the plot that can defend, is an
   enemy (hidden nationality as in the duel) and is not invisible to the attacker; its pick: its
   stance's (the garrison's in a city, else the largest army's). An empty force or no defender is a
   refusal, and nothing has changed. The round model's `Battle` is built here (the pair table of A
   and Dv does not change before the rounds), and phase 7's pre-battle hook reads it.
2. **Morale** (Q8, "Im afeared!"): a committed Fear defender makes each committed attacker not
   immune roll today's check; one that fails sits out (its attack spent, its slot empty). Then a
   Fear attacker that did not sit out does the same to the committed defenders, which stay on the
   plot but do not fight.
3. **Opening.** Defensive strikes (Q5): for each committed attacker that did not sit out, every unit
   on the plot with a strike chance, at war and not used up, committed or in reserve, rolls; the
   best success strikes with today's damage, never past 95%, and is used up unless it has Blitz.
   Collateral (Q4): each committed attacker with collateral damage (siege, Berserker, Fire
   Elemental, ...) that did not sit out splashes every visible enemy defender on the plot, reserve
   included, with today's formula and limit (the duel's `- 1` for its own defender is gone).
   Neither kills. Then phase 7's opening hook, on the battle with its HP after them.
4. **The battle**: the round model (`battle.rs`, `resolve.rs`) on the game's sync RNG: opening
   volleys (first strikes), then up to 40 rounds; in each, every engaged fighter hits once in the
   interleaved slot order (A0, D0, A1, D1, ...), the target class by its role's order over the
   round-start state, a target in the class at random, damage `20 x A / (A + Dv) x r` (r in
   80-120%) applied at the end of the round, all at once. Withdrawal: one roll, the first round a
   unit ends below 40% (attackers always; defenders outside cities only, Q3). An attacker with a
   combat limit never takes a target past it and leaves when every target is at it; defenders have
   none. A side with no engaged fighter has lost (both at once: mutual); 40 rounds: a draw.
5. **After the battle** (`apply.rs`, G2): damage written back; battle XP before any per-kill effect
   (each unit that damaged a victim, by its share of the damage, the killer at least the minimum;
   withdrawals get the withdrawal XP); per kill in kill order the war weariness, war success and the
   barbarian counter, then `combatWon`'s per-kill part (capture, enslavement, gold, created units,
   the promotion from combat, the heal on a kill, the Python callbacks; a conversion rolled per kill
   and applied once after the killer's last kill); per engagement the promotions removed by combat,
   the combat-apply promotions and poison; the victims die (immortals are reborn); medics heal once
   a turn per unit (Q12), then phase 7's post-battle hook; withdrawn defenders flee; a Fear attacker that won outside a city routs the
   rest (Q8); every committed attacker has made its attack (one a turn, Blitz one more, counting
   engagements, bombards and missile strikes) and pays the attack's moves unless it moves in; with
   no visible enemy defender left, the launching group moves in (a city is captured as today), and
   only it; the armies of everyone involved are settled.
6. **The report** (`report.rs`): every engagement's summary (sides with their real and shown
   owners, stances, slots, committed, lost, withdrawn, sat out, fled; the outcome, with the
   defender's uncommitted units still on the plot) is kept for the current
   and previous turn and the last `ENGAGEMENT_REPORTS_KEPT`; a battle a human could see also gets
   its full battle log (below). A game-log message to each side's players names the report.

It never sets the duel's combat state (`combat_unit`, the attack plot, combat timers), and nothing
outlives the call. Every loop is bounded (finite lists once each; volley rounds by the largest
volley count; rounds by 40), so every engagement ends; a turn holds at most `1 + Blitz` engagements
per unit in a pool, since each spends an attack of every committed unit.

**Missiles** (Q11): a missile (Fireball, Meteor) ordered into an enemy plot strikes instead:
one hit of the round model on a unit of the defender's pick (by the missile's role order), then
its collateral; it dies, with the kill if it killed (counted as an engagement's kill is, below).

## Overkill (a rule of this game, not of the spec)

The spec draws a hit's target at random within its priority class. The engagement chooses targets
in the interleaved hit order with the round's pending damage known: a hitter draws among the units
of its class that the round's pending damage does not already kill, and among all of them only when
every one is doomed (`targets()` in `resolve.rs`; plan revision 3, item 4). The class is still
chosen over the round-start state. Why: with simultaneous damage, random pile-ons on a unit that is
already dead read as bad play, in the replay and in the outcome (a stack would waste hits a human
commander never would). The estimator (`expect.rs`) works in moments and has no pile-ons to avoid;
its calibration bounds (`estimator_calibration_bounds`) still hold against the sampled resolver
with the rule.

## The auto-pick consults the estimate (a rule of this game, not of the spec)

The spec has each side commit the stance's quota pick. The attacker's proposal
(`engage_propose_force`, `pick.rs`; what an attack with no force given fights, the AI's too) starts
from that pick and keeps its best single swaps by the moment estimator (`engage_swap_search`,
`estimate.rs`; plan revision 3, item 3). Why: on the battle UI design's test battles the stance's
own pick won 70% where one swap won 91-94%, so a default taken as is would teach the player to
override it every time.

- Each round tries every single swap (a committed unit out, a pool unit not committed in, the
  river slots kept for `bRiver` units), one per pair of kinds the estimate cannot tell apart (the
  same fighter, pairs, damage and opening stats), and keeps the best if it beats the plan so far
  and the stance's pick: a higher win chance by more than 5 per mille, or one at most 2 lower with
  0.05 fewer expected units lost (`swap_better`). At most three rounds. So the proposal is the
  stance's pick or beats it.
- The estimate is against the defense as the attacker knows it: the candidates it can see, their
  stance if it is its own team's, else the default stance (as the preview assumes).
- A unit the stance holds (Preserve Veterans: above its level, heroes) is never swapped in; only the
  player commits one.
- The defender's pick stays the blind stance pick (the spec): it does not look at the attacker's
  force.
- Cost (release, `preview_timing`): 1-2.5 ms a proposal for 6-10 a side from a pool of mixed
  units with a reserve of three, up to 7.5 ms when every unit of the pool differs.

## The estimator

`estimate.rs` (plan 8.2): the expected outcome of an engagement, for the AI's attack decisions
(step 8, not wired yet) and the swap search. `&self`: no random number drawn, nothing changed (test
`estimates_draw_nothing_and_change_nothing`). Its battle is built by the real fight's own set-up
(`engage_setup`), so its fighters, pairs and HP are the fight's (test
`the_estimate_is_of_the_real_battle`).

- **Inputs** (`EstimateInput`), all explicit, nothing read from the true plot but what anyone sees
  (its terrain, a city and its defense): the attacking player, the plot and its approach (`None`:
  no river, by land), the attackers (`Attackers::Pick(pool, stance)`: the proposal from a pool;
  `Attackers::Force`: the force as given, or what the default stance commits of one too big for the
  slots), which of them count (`Eligible::Now`: `unit_can_engage`; `Eligible::Hypothetical`: a land
  unit with a role that can attack, not a missile, the plot passable to it and, with a combat
  limit, some listed defender below it, not from the water; moves, attacks left and adjacency not
  asked), the war test, the caller's list of the units on the plot (the AI: what it sees through
  its fog; the defender's candidates are those of them that can defend, and the opening's strikers
  and collateral targets come from it too), the defender's assumed stance, damage overrides
  (planned softening, damage carried between engagements) and a hypothetical city defense (a
  planned bombard: slots and Dv follow).
- **What it integrates**: in expectation what the fight rolls. The morale check: each fighter's
  chance to sit out. The opening: per attacker in slot order, the defensive strikers sorted by
  damage, `sum P(i is the best success) x E[damage_i]`, each striker available with the chance an
  earlier attacker did not use it up; collateral with today's formula on the expected damage of the
  healthiest targets, the reserve's included; both folded into the HP. Then the rounds by the moment
  integrator (`expect.rs`) and its calibrated win table. Under a declare-war test the opening tests
  war as the fight after the declaration does (a striker strikes, a unit takes collateral, once its
  team is in the war), so the estimate is the one made after declaring (test
  `a_declare_war_preview_is_the_battle_after_the_declaration`).
- **Outputs** (`Estimate`): the win chance (a draw is not a win), expected losses and withdrawals
  per side, HP left, rounds, whether winning takes the plot (the defender commits every candidate),
  the force and the defender's pick, each fighter's HP at the rounds' start and the integrator's
  per-fighter result.
- **The assault** (`engage_estimate_assault`): engagements one after another, each committing the
  next proposal from the attackers not used yet, the defenders committed carrying their alive weight
  and damage moments, the reserve its expected collateral, the strikers their used-up strikes (the
  real fight's striker keeps `made_attack` for the turn: a striker strikes with the chance it is
  still on the plot and has its strike left; a defender gone neither strikes nor takes collateral;
  test `the_assault_spends_the_defensive_strikes`), until the expected defenders left are below one
  half, no attacker is left, or 12 engagements; shaped like the AI's `combat_sim::Sim`.
- **The plot wrapper** (`engage_estimate_plot`): a player's real pool on the plot it attacks from,
  the force given (checked) or proposed, the defenders its team sees, the defender's stance as its
  team knows it (its own team's: the real one; anyone else's: the default, until phase 7's recon
  reveal).

## The attack preview

`preview.rs` (plan revision 3, item 2; the battle UI design's plan screen): what the player sees
before an attack, pure. `engage_preview` gives the slots, the pool, the force (given and checked,
or the proposal), the proposal, the reserve and the stance; the defenders the attacker's team sees,
their stance when known (its own team's only, until phase 7), the stance assumed (else the default)
and its pick, and the pick as the attacker knows it (`DefPick::Known`, or with more candidates than
slots and the stance unseen `Likely { sure, maybe }`: committed under every stance, under some);
the war the attack would declare; and the forecast.

- **The forecast** plays `ENGAGEMENT_PREVIEW_RUNS` (200) battles of the real fight's code on that
  set-up: the morale check and the opening's strikes and collateral as the same pure rolls the real
  fight makes, then the round model, on a private RNG seeded from the plan (the force and the
  defender's pick, sorted, the plots, the turn, the defender's assumed stance). The attacker's
  stance is not in the seed: the same force is the same battle, whichever stance proposed it. The
  same plan always shows the same numbers whatever the game's RNGs hold, and the game's RNG is
  never drawn (test `the_preview_is_a_function_of_its_inputs_and_changes_nothing`). A move that
  declares war previews the battle after the declaration: its opening's strikes and collateral test
  war as that fight does, so the plan, its What-if list and the forecast frozen into the log are the
  ones made once the war is declared. It gives the win and draw
  chances, the rounds' p10/p50/p90, the units lost when it wins, each committed fighter's chance to
  fall and to withdraw and the round by which it has fallen in half the battles, and each side's
  mean HP after the opening and each round.
- **What if** (`engage_preview_alternatives`, a second call so the plan never waits): the stance's
  own quota pick, the best three single swaps of the force of different make-ups (every swap ranked
  by the moment estimator, those three sampled), and the other stances' proposals (what choosing
  that stance proposes), each forecast like the plan.
- **Frozen into the log**: when an attack whose battle gets a log is made, a human attacker's
  preview and its What-if list are recomputed (the same seed: the numbers it saw) into the log's
  `forecast`; an AI attacker's is the moment estimate (`source: Expected`).
- **Time** (release, this 4-core shared machine, `preview_timing`): the plan as it opens (its
  proposal and the forecast) and its What-if list together take 28 ms for 6 v 6, 34 ms for 8 v 8 and
  43 ms for 10 v 10 with a pool of mixed units and a reserve of three, and 31, 65 and 78 ms when
  every unit of the pool differs (the swap search's worst case); a re-run after a swap 4-12 ms.
- **Against the estimator**: on 288 battles of real units near parity (open land, a forest, a 40%
  city; test `the_preview_against_the_estimator`) the moment estimator's win chance is 44 per mille
  from the preview's on average, 85 on the 73 contested ones (the preview between 5% and 95%), 237
  at worst.

## The battle log

Written through an optional sink (`sink.rs`) only for a battle a human could see (a human fights on
either side, or a human team sees the plot): the fighters (committed first, in the round model's
order, then the reserves on both plots) with their A and Dv parts, the sides' slots and picks, every
event in resolution order (morale rolls, defensive strikes, collateral, volley rolls, every hit with
its target rule, the steps passed and why, the eligible set and r; round ends with deaths, kill
credit, assists and overkill; withdrawals; the shield falling; the end; XP, kill effects, heals,
flight, the rout, the move in), and the A and Dv of every pair that hit. Writing it draws no random
number and changes no state: the same engagement with and without it leaves the same RNG and the
same game (test `the_sink_changes_nothing`).

## Telemetry

`engage*` counts and the `engagement` event (docs/TELEMETRY.md); the per-unit `attack` / `defend`
counts stay comparable with the duel's; each kill, a missile strike's too, counts as a combat
won (`kill_by`, the war's kills, a summon's kills). `engage_est` records the moment estimator's win chance for the battle as fought against its
result: the estimator's calibration in real games. `telemetry.py`'s combat section prints them.

## Decided while building it (phase 6 step 6)

The plan's own list (docs/COMBAT_INVENTORY.md and the plan's section 13) holds; these were left
open and are decided so:

- Sat-out attackers are not struck by defensive strikes and do not splash collateral (they never
  close in); the Fear source on the attacking side must itself fight.
- A defensive strike that does no damage is still a strike: the striker is used up, as today.
- Battle XP shares are of the round model's damage (the opening's strikes and collateral are not a
  hitter's share). Withdrawal XP takes the enemy side's least restrictive XP cap.
- `unit_attack` is forced: the unit given, whatever its moves and attacks this turn, if it is alive,
  can attack and has a role.
- The capture of defending victims (equipment) is decided before the rout, on the defenders left.
- Per-kill game-log messages are replaced by one message per side and engagement, naming the report.
- A medic's heal goes only to a hurt unit, and only then uses its once a turn; the patient's own
  same-tile heal counts.
- A land plot with no unit that can engage but a unit that can still defend in the duel (a ship in
  a port) fights today's duel (see "Open").

## Open

- The AI still decides its attacks on the duel's odds: the estimator is built (step 7), and its
  wiring into the AI (step 8) waits, as all AI work does while the combat style is in flux (the
  owner, 2026-10-31). An AI attack with no force given already fights the proposal (the auto-pick
  is a rule).
- The battle UI design has the forecast average over the plausible stances when the defender's is
  unseen and its candidates exceed the slots; the build forecasts against the default stance's pick
  (the plan's: the AI and the player assume the defender's stance the same way) and shows which
  units may or may not fight (`DefPick::Likely`).
- A city whose only defenders are ships in port is attacked with today's duel (ships never defend
  land in an engagement, but nothing else defends there).
- Phase 7: spells in combat phases, summons in battle, the assassin's and the Guardsman's targeting,
  commander auras (the hooks are in place: plan section 14). The three spell hooks,
  `engage_pre_battle`, `engage_opening_spells` and `engage_post_battle_spells` (`engage.rs`), are
  called at their places in the sequence and return nothing yet; the first two return
  `BattleEffect`s added to the `Battle` (both integrators apply them), the third `Cast` records.
