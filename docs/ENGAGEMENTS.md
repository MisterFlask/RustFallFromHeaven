# Engagement combat

This game's own land combat (not FFH2's), and the game's only land combat: it is not an option
(the owner's decision, 2026-11-03: `GAMEOPTION_ENGAGEMENTS` was removed, and with it the
requirement of `GAMEOPTION_ARMIES`, which nothing in a battle needs: an army commander's aura
applies only to units in an army). Every game fights engagements, the AI-only games of
`ffh autoplay`, `ffh-stress` and `ai_metrics.py` too (AI measurements from before 2026-11-03 are of
the duel); saves made before do not load (the game options changed). When units attack a land plot, they fight a **formation battle** (revision 4, the
owner's Ogre Battle-style engagements, 2026-10-31, designed in a browser prototype): each side's
units stand on a grid of three rows by its lanes and act by row, role and speed for three rounds;
the plot falls only when no defender is left on it. The owner's answers on every FfH combat
ability (Q1-Q13) and the ground rules G1-G5 are in docs/COMBAT_INVENTORY.md (its answers about
rounds, stances and targeting are superseded by revision 4: this document).

Naval and air combat stay the duel.

Code: `crates/ffh-core/src/engagement/` (`mod.rs` lists the modules): the pure resolver in
`formation/` (`formation::resolve`, the auto formation `formation::auto_arrange`); on `Game` the
dispatch rule (`state.rs`), the units' roles, HP and A and Dv per hit (`stats.rs`),
the slots (`slots.rs`), who can fight (`pick.rs`), where each side's units stand (`arrange.rs`),
the engagement (`engage.rs`, `apply.rs`, `sink.rs`), the reports and the battle log
(`report.rs`), and the attack preview (`preview.rs`). Tests: the resolver's in
`formation/tests.rs`; scenarios in `crates/ffh-core/tests/engagements.rs`; slots, who fights and
the formations in `tests/engagement_pick.rs`; the preview in `tests/engagement_preview.rs`; the
units' stats in `tests/engagement_stats.rs`; state and saves in `tests/engagement_state.rs`; the
data in `crates/ffh-data/tests/engagement_data.rs` and `engagement_overrides.rs`.

## The rules

**The field.** Slots per side as before (`slots.rs`: 8 base, +2 open flat land, -2 a forest or
jungle, -2 hills, at least 3; a city's defense costs the attacker one per 25%; across a river two
of the attacker's slots are only for `bRiver` units, Q9). Each side stands on a grid of 3 rows
(front 0, middle 1, rear 2) by `max(2, ceil((slots + 2) / 3))` lanes (`formation::lanes_for`, of the
side's ordinary and river slots together; the sides' lane counts may differ). A large unit (`bLarge`,
below) fills two cells side by side in one row and counts 2 slots, has
`ENGAGEMENT_LARGE_HP_PERCENT` (150) of its role's HP, and is the front or back unit of both its
lanes.

**Roles** (`CIV4CombatRoleInfos.xml`):

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
chance first strike rolled at the battle's start (first strikes are speed now; immunity to first
strikes has no effect in an engagement). A flanker is a unit whose rows have a CHARGE. The home row
and the auto-pick order are the auto formation's, always the role's.

**Per-unit values** (the owner's request, 2026-10-31). A unit type may replace its role's values in
`CIV4UnitInfos.xml`; each falls back to the role's when absent, and no unit sets them yet (the
content is the owner's to decide):

| Tag | Replaces | Absent / -1 | Valid |
|---|---|---|---|
| `CombatRole` | the role its unit combat gives it | that role | a role |
| `iAttack`, `iDefense` | the base of A (`iCombat` x the role's A%), of Dv (`iCombatDefense` x Dv%) | 0: the role's | above 0 |
| `iHP` | the role's max HP (a large unit's 150% of it) | 0: the role's | above 0 |
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
rules warn at load and leave the unit its role's value. One accessor per value resolves the unit
type, then the unit's role: `Game::unit_engage_speed`, `unit_engage_dodge`,
`unit_engage_crit_bonus`, `unit_engage_rows`, `unit_engage_best_row`, `unit_engage_actions` (with the
extra action), `unit_engage_flanks`, `unit_engage_max_hp`, `unit_engage_large`, `unit_engage_spell`
(stats.rs, arrange.rs); the rules' fighters, the auto formation, the preview, the view and the
client read these, not the role's fields.

**Action kinds and reach** (`ffh_data::ActionKind`):
- STRIKE: the front unit of each enemy lane; physical; a crit knocks the target back.
- CHARGE: where flanking works (the defended plot has no city and no forest or jungle), every
  enemy on the field, the target the one with the highest threat for its HP left (threat: its A
  times its actions from its current row; compared `threat_x x hp_y`), and in the first round
  `ENGAGEMENT_CHARGE_BONUS` (100) % more damage (the cavalry's first-round charge in the field, the
  owner's request). Elsewhere it is a STRIKE (front reach, no bonus, the normal choice; the log says
  strike). Physical; a crit knocks back.
- STAB: any enemy. Physical.
- SHOOT: the backmost unit of each enemy lane. Physical, `ENGAGEMENT_SHOOT_MISS_PERCENT` (10) more
  miss.
- SPELL: the backmost unit of each enemy lane; never misses, never crits. A caster that knows a
  damage spell (`GameData::engage_damage_spell`: of the spells with `iDamage > 0` whose promotion,
  unit, unit class and unit combat prerequisites it meets, the highest `iDamage`, then the lower
  id) casts it at power 100 with a splash; one that knows none casts the magic bolt,
  `ENGAGEMENT_BOLT_PERCENT` (60), one target. The log names the spell.
- BOMBARD: the backmost unit of each enemy lane; physical, `ENGAGEMENT_BOMBARD_MISS_PERCENT` (15)
  more miss; never crits.
- HEAL: the most hurt ally on the field (most HP missing; ties by lane, then row), itself
  included: `ENGAGEMENT_BASE_DAMAGE x ENGAGEMENT_HEAL_PERCENT (50) x r`. No one hurt: the action is
  spent.
- Splash: a unit with collateral damage (`iCollateralDamage > 0`: siege, Berserker, Fire
  Elemental, ...) splashes its damaging hits (not a miss) on the target's neighbours (beside it in
  its row, in front and behind in its lanes) for `ENGAGEMENT_SPLASH_PERCENT` (50) % of a hit on
  each, never taking one past its collateral limit (`iCollateralDamageLimit`). As the duel's
  collateral damage, that splash is times `100 +` the splasher's extra collateral damage (Barrage
  I-III's `iCollateralDamageChange`: +40, +60, +100) and times `100 -` the splashed unit's
  collateral damage protection (Drill III and IV's `iCollateralDamageProtection`: 20 each) percent;
  a unit with 100 or more protection is not splashed (and draws nothing). A known spell splashes
  the same 50% as the spell: no limit, no Barrage, no protection (a caster's bolt is not a spell's
  splash: a unit with collateral damage splashes it as collateral damage). Splash hits never crit.
  This replaces the round model's collateral step before the battle (Q4).
- Siege (combat limit below 100, attackers only): its hits never take a target below
  `(100 - limit)%` of its max HP, and such targets are not in its reach.
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

**Targeting** (everyone, the player does not choose; `Formation::choose`):
1. A flanker's pool is the anti-flank units in its reach when there are any.
2. **Nearby first** (the owner's rule, 2026-10-31): of the pool, the units whose lanes are within
   one lane of the hitter's (`Formation::lane_distance`: the least over both units' lanes, a large
   unit's two; lanes are numbered the same on both sides) come first; only when none of those is
   in reach does it consider the rest. A charge where flanking works keeps going for its target
   wherever it is; heals are not affected; a splash's centre is chosen by this rule too.
3. A charge where flanking works takes the highest threat for its HP left; otherwise, if this hit's
   expected damage would kill some, the one with the highest A among them, else the one it hurts
   most for its HP left (expected / HP, by cross-multiplication). The expected damage is capped at
   what the hitter's combat limit lets it take, so a siege engine takes no kill. Ties by the higher
   A (the unit's A against nobody in particular), then lane, then row, never by list order.

**Hits.** Expected damage, centi-HP: `ENGAGEMENT_BASE_DAMAGE (40) x 100 x A / (A + Dv)` (an even
hit 20 HP; six kill a frontline unit), then times the hit's percentages (a spell's power, the
ground's, the first round's charge, a splash's) over one division. A hit: `max(1, expected x r)`, r
in `ENGAGEMENT_DAMAGE_RAND_MIN..=MAX` (80..120) %. A physical action misses with
`ENGAGEMENT_MISS_PERCENT` (10) + the kind's extra + the target's dodge (at most 95), rolled before
the crit and the damage. Strike, charge, stab and shoot crit with `ENGAGEMENT_CRIT_PERCENT` (10) +
the hitter's crit bonus (at most 95), for `ENGAGEMENT_CRIT_DAMAGE_PERCENT` (150) % of the hit; a
strike's or a charge's crit knocks the target back a row when every cell behind it is free (it acts
from that row then).

**Withdrawal** replaces a killing hit: a unit with a withdrawal chance (attackers: FfH's
`withdrawalProbability`; defenders: the defensive withdrawal) that a hit (a splash too) would kill
rolls it; on a success the hit does nothing and the unit leaves the battle alive. Never a city's
defenders (Q3; the resolver enforces it).

**The ground** (the defended plot's, `Game::engage_ground`: a city on it; a forest or jungle,
which is a feature with `iEngagementSlotsChange` below 0: Forest, New Forest, Ancient Forest,
Jungle; hills; several multiply; `formation::Ground::rules` lists those that apply): a city: the
attacker's SHOOT deals `ENGAGEMENT_CITY_ATTACKER_SHOT_PERCENT` (50) %; a forest or jungle: SHOOT and
BOMBARD deal `ENGAGEMENT_FOREST_RANGED_PERCENT` (50) % on both sides; hills: every attacker hit
(spells too) deals `ENGAGEMENT_HILLS_ATTACKER_PERCENT` (75) %, the defender's SHOOT and BOMBARD
`ENGAGEMENT_HILLS_DEFENDER_RANGED_PERCENT` (125) %. A city, a forest or jungle stop flanking.

**Large units** (`bLarge` in `CIV4UnitInfos.xml`): the spec's Hill Giant, War Elephant and Elephant,
the dragons (Abashi, Acheron, Drifa, Eurabatres), every golem (Bone, Clockwork, Flesh, Iron,
Mithril, Mud, Nullstone, Wood) and the Treant, and of that size class the giant heroes Gurid,
Margalard and Wilboman (a frost giant), Barnaxus (a golem), the War Machine, the Earth Elemental (a
golem of rock) and the War Tortoise. Demons (the Balor, the Pit Beast) and the great beasts (the
Giant Spider, the Manticore) stay one cell.

## A and Dv per hit

`Game::engage_hit` (stats.rs; G1, Q1, Q2, Q9): the hitter's A and the target's Dv are its base (the
type's strength times the role's percent, or the type's `iAttack` / `iDefense`, plus the unit's own
flat and typed strength) under one summed percentage modifier, through the duel's terms. Revision 4
changed two rows of the term table (stats.rs's module comment): **the defended plot's own defence
no longer adds to Dv** (its terrain or feature cover, hills, a fort's improvement defence, the
city's building and culture defence, FfH's range defence: the duel's `plot_defense_modifier`) and
**`CITY_BARBARIAN_DEFENSE_MODIFIER` is gone** (a defended-plot term). The ground's rules above and
the slots replace them. What stays on the defending side are the unit's own terms: its
fortification, its city defence in a city (`iCityDefense`, promotions), its hills defence on hills,
its feature (else terrain) defence. Everything else is as before: the promotions' own percentages,
Homeland, Stigmata, the bonuses against the opponent (unit combat, class, promotions, domain,
animals), the handicap's on the target, the attacker's city, hills, feature and terrain attack and
the amphibious term, the river (slots, Q9). A unit's A against nobody in particular
(`Game::engage_a_alone`) is a fighter's "most dangerous", its ties and the auto formation's
strength. A missile's strike (below) keeps both rows: against it the plot's own defence and the
city's barbarian defence still count in Dv (`HitCtx::cover`), as before revision 4.

## Where the units stand

`arrange.rs`. A `Place { unit, lane, row }` puts a unit in its side's grid (a large unit's first
lane).

**The auto formation** (`Game::engage_auto_formation`, over `formation::auto_arrange`, the
prototype's `autoArrange`): one unit of each role first in the roles' `iAutoPick` order (Screen,
Frontline, Archer, Flanker, Arcane, Divine, Siege, Assassin), then the rest in that order; within a
role the healthy units first (not below `ENGAGEMENT_AUTO_MIN_HP_PERCENT`), then the strongest (its
A on its side against nobody in particular, with its promotions; a large unit's counts double), then
the lower unit id; while the slots last (a large unit needs 2; units without `bRiver` at most the
ordinary slots, Q9's river rule). Then the large units are placed first, each unit in its role's
home row, the lanes nearest the centre first (then the lower), else the nearest row with room
(behind before in front). Deterministic, no RNG, the same for humans and the AI. Heroes and
veterans are not held back. The attacker fights it when its order gives no formation; the defender
always does (`Game::engage_defense_formation`: the hook for standing formations per army, a later
step).

**An ordered formation** (`Game::engage_validate_formation`, the `engage` command's): every unit in
the attacker's pool once, inside its side's lanes and rows, no two units in one cell (a large unit
on two free cells of one row), at most the attacker's slots (a large unit counts 2), the units
without `bRiver` at most its ordinary slots. The error is the reason the UI shows; a refused order
changes nothing.

## Which attacks are engagements

`Game::engage_applies(u, to)`: the option is on, the attacker is a land unit (one carried on a
transport counts: an amphibious attack) and `to` is land. The rule is applied at each entry:

- `group_attack_force(g, x, y, flags, formation)`, the one entry for every order of a group into an
  enemy plot (the human `move` and `engage` commands, AI and barbarian missions, the C++ AI
  benchmark); `group_attack(g, x, y, flags)` is it with no formation. Today's preamble (the path
  step for `MOVE_THROUGH_ENEMY`, the AI's pre-attack spells and bombard-first), then for an
  engagement: no candidate defender there means today's attack (it captures non-combatants, if
  any); Alertness for a human order without a formation; the AI's war declaration as today; the
  `doCombat` hook; then one engagement launched by the group, or for a group of missiles one
  missile strike per missile. `formation`: where the attacker's units stand (checked), `None` the
  auto formation; a call parameter, never saved. Each order into an enemy plot is one engagement; a
  follow-up is a new order. The AI attacks through it with no formation (the auto formation) and
  decides its attacks on the duel's odds as before (the AI is paused).
- `unit_attack(u, to)` (Taunt, the Shrine of Camulos, Gift of Kylorin): a one-unit engagement,
  forced (whatever the unit's moves and attacks this turn, as today's `attack`), in the auto
  formation, or a missile's strike; with only non-combatants there, today's attack.

## The engagement

`Game::engage` (`engage.rs`), in this order:

1. **Set-up.** Each side's slots and lanes. The attacker's pool: every unit of the player on the
   attacking plot that can attack the plot (`engage_attackers`: across its armies, garrison,
   heroes, mages and recon units); its formation: the order's (checked) or the auto formation. The
   defender's candidates: every unit on the plot that can defend, is an enemy (hidden nationality
   as in the duel) and is not invisible to the attacker (`engage_defenders`); its formation: the
   auto formation of them. An empty formation or no defender is a refusal, and nothing has
   changed. The committed units in the engagement's order (the attacker's by lane, then row; then
   the defender's), each a resolver `Fighter` (`Game::engage_fighter`), and the pair table of A and
   Dv (`engage_hit`, which nothing before the battle changes). Phase 7's pre-battle hook reads the
   set-up.
2. **Morale** (Q8, "Im afeared!"): a committed Fear defender makes each committed attacker not
   immune roll today's check; one that fails sits out (its attack spent, its cell empty: it never
   acts). Then a Fear attacker that did not sit out does the same to the committed defenders,
   which stay on the plot but do not fight.
3. **Opening.** Defensive strikes (Q5): for each committed attacker that did not sit out, every unit
   on the plot with a strike chance, at war and not used up, committed or in reserve, rolls; the
   best success strikes with today's damage, never past 95%, and is used up unless it has Blitz.
   It never kills. (The round model's collateral step is gone: splash, above.) Then phase 7's
   opening hook, on the set-up with its HP after the strikes.
4. **The battle**: the committed units that did not sit out, with their HP after the opening, on
   the formation resolver (`formation::resolve`) on the game's sync RNG.
5. **After the battle** (`apply.rs`, G2): damage written back; battle XP before any per-kill effect
   (each unit that damaged a victim, by its share of the damage, the killer at least the minimum;
   withdrawals get the withdrawal XP); per kill in kill order the war weariness, war success and the
   barbarian counter, then `combatWon`'s per-kill part (capture, enslavement, gold, created units,
   the promotion from combat, the heal on a kill, the Python callbacks; a conversion rolled per kill
   and applied once after the killer's last kill); per engagement the promotions removed by combat,
   the combat-apply promotions and poison; the victims die (immortals are reborn); medics heal once
   a turn per unit (Q12), then phase 7's post-battle hook; withdrawn defenders flee (outside a
   city); every committed attacker has made its attack (one a turn, Blitz one more, counting
   engagements, bombards and missile strikes) and pays the attack's moves unless it moves in.
6. **The result** (spec 1.7): defenders stay in place until annihilated. **The plot falls only when
   no defender is left on it** (in the line or in reserve, or among those that sat out): then the
   launching group moves in (a city is captured as today), and only it. Otherwise nobody moves: the
   attackers stay on their plot and every survivor keeps its damage. Withdrawn attackers stay with
   their army. The result (`report::EngageResult`): `Taken` (the plot fell), `Cleared` (the
   defender's line is gone, defenders are left on the plot), `Repulsed` (the attacker's line is
   gone), `Held` (both lines stand after the last round). There is no win, draw or mutual by
   damage, and no Fear rout (it needed a won battle). The armies of everyone involved are settled.
7. **The report** (`report.rs`): every engagement's summary (sides with their real and shown
   owners, slots, lanes, the slots used, committed, sat out, fallen by type, escaped, standing; the
   result, the rounds, whether the plot was taken and the city captured, the defenders left) is
   kept for the current and previous turn and the last `ENGAGEMENT_REPORTS_KEPT`; a battle a human
   could see also gets its full battle log (below). A game-log message to each side's players names
   the report.

It never sets the duel's combat state (`combat_unit`, the attack plot, combat timers), and nothing
outlives the call. Every loop is bounded (three rounds, passes by the most actions, finite lists),
so every engagement ends; a turn holds at most `1 + Blitz` engagements per unit in a pool, since
each spends an attack of every committed unit.

**Missiles** (Q11): a missile (Fireball, Meteor) ordered into an enemy plot strikes instead:
`formation::missile_hit` on the defender's auto formation (a strike's reach and choice from outside
the field: no nearby first, no miss, no crit, no splash, no withdrawal; r drawn; never past the
missile's combat limit), then its collateral (today's formula); it dies, with the kill if it killed
(counted as an engagement's kill is). It strikes at the strength it had before formation battles
(spec 1.9: missiles unchanged): `ENGAGEMENT_MISSILE_PERCENT` (50) of a hit
(`Formation::missile_expected`: `ENGAGEMENT_BASE_DAMAGE` doubled to 40 for the battles, so half of
it is the round model's 20), the defended plot's own defence in its target's Dv
(`HitCtx::missile`), and none of the ground's rules (it does not fight on the field). Its target's
choice reads that hit.

**RNG**, in this order and nowhere else: the morale rolls ("Im afeared!", attackers then
defenders); per committed attacker that did not sit out its defensive strike's roll and damage
("Defensive Strike"); then the resolver's (`formation/mod.rs`'s module comment): per battle, each
fighter with chance first strikes in fighter order ("Engagement first strikes", `0..=chance`); per
pass each mover's coin in fighter order ("Engagement order"); per action a physical one's miss
("Engagement miss"), if it lands the crit roll of strike, charge, stab and shoot ("Engagement
crit"), r ("Engagement hit"), the target's withdrawal if the hit would kill it ("Engagement
withdraw"), then each splashed neighbour's r and withdrawal in (lane, row) order (not one at its
floor or fully protected from collateral damage); a heal's r
("Engagement heal"); an action with no target and a heal with no one hurt draw nothing; then the
post-battle steps' (combat-apply, poison, the per-kill effects' own rolls, the flight's plot).

**The resolver's API** (`engagement::formation`, pure: nothing of `Game`): `Formation::new(params,
ground, lanes, fighters, pair)` (each `Fighter`'s side, tag, lane, row, large, max HP and HP, its
speed, first strikes and chance first strikes, actions per row, A against nobody, dodge, crit bonus,
anti-flank, flanker, withdrawal chance, combat limit, collateral limit, extra and protection,
spell), `Formation::validate`
(inside the grid, one unit a cell), `Params::from_defines`, `Ground` and `Ground::rules`;
`resolve(&formation, rng)` / `resolve_quiet` give an `Outcome`: each fighter's `End` (HP, final row,
speed, the first strikes it rolled, its fate: standing, fell by whom in which round, withdrew in
which round; damage dealt, taken, healed), the kills in order, the damage matrix (`dealt(h, t)`,
battle XP's shares), the rounds begun, which sides were cleared, and the events (`Event`: round,
pass, actor, kind as fought, spell, target, hits `{target, dmg, expected, r, crit, miss, killed,
withdrew, splash}`, heal, knock-back, note). `auto_arrange(candidates, lanes, slots, ordinary)` is
the auto formation; `missile_hit(formation, h, rng)` a missile's hit (`missile_expected` its
expected damage).

## The battle log

Written through an optional sink (`sink.rs`) only for a battle a human could see (a human fights on
either side, or a human team sees the plot), `report::BattleLog`:
- the slots and lanes per side, the ground (city, forest or jungle, hills) and its rules that
  apply, and whether each side's formation was ordered;
- the fighters (`LogFighter`): the committed units first in the engagement's order (sat-out ones in
  their place), then the reserves on both plots (no cell): the unit, its name, unit type, owner and
  shown owner, side, cell `(lane, row)`, large, level, XP, hero, role, max HP, HP before the opening
  and when the battle began, its speed in the battle, first strikes, their chance and what it
  rolled, actions per row, the spell it casts, dodge, crit bonus, anti-flank, flanker, withdrawal
  chance, combat limit, collateral limit, defensive strike, immunities, Fear, A and Dv with their
  parts, its fate (standing, fell by whom in which round, withdrew in which round, sat out) and the
  damage it dealt, took and healed;
- the events in resolution order (`LogEvent`): the stages (pre-battle, morale, opening, battle,
  post), the morale rolls and the units that sat out, the defensive strikes, every action of the
  battle (`Action`: the resolver's `Event` with fighters as log indices and a spell as its
  `SpellType` index: `{round, pass, actor, kind, spell, target, hits: [{target, dmg, expected, r,
  crit, miss, killed, withdrew, splash}], heal, knocked: {unit, from, to}, note}`), the end (the
  rounds and which lines are gone), then the post-battle steps (XP with its sources, kill effects,
  heals, flight, the move in);
- the A and Dv of every pair that hit, with the pair-only terms;
- the plan the attacker saw, for a human attacker (`FrozenForecast`: the preview recomputed with the
  same seed: the chances the defender's line is wiped out, the plot falls, the attacker's line is
  wiped out, the units lost per side, each fighter's chance to fall and to escape).

The view (`ffh-play`'s `view/battles.rs`) shapes it as JSON for the battle screens (the contract is
`crates/ffh-play/assets/app/battle_log.js`'s header; docs/HUMAN.md, `battle_log`): `events` are what
the battle shows (the opening's units that sat out and defensive strikes at round 0, then the
actions), `after` the post-battle steps; the stages, the morale rolls that passed and the end are
left out (the outcome says how it ended). Writing it draws no random number and
changes no state: the same engagement with and without it leaves the same RNG and the same game
(test `the_sink_changes_nothing`). Saved: a report keeps its log for the current and previous turn
and the last `ENGAGEMENT_REPORT_LOGS_KEPT` logged reports, its summary for the last
`ENGAGEMENT_REPORTS_KEPT` (`Game::battle_reports_trim`).

## The attack preview

`preview.rs` (`Game::engage_preview`, pure): the plan of an attack before it is made: the
attacker's pool, its slots and lanes, the plan's formation (the one given, checked, or the auto
formation) and the auto formation, the pool's units not in it; the defenders the attacker's team
sees (invisible units are never candidates) and their formation (the rules' auto formation of them,
as the real fight forms them) and reserve; the ground and its rules; the war the attack would
declare; and the forecast.

- **The forecast** plays `ENGAGEMENT_PREVIEW_RUNS` (200) battles of the real fight's code on that
  set-up: the morale check and the defensive strikes as the same pure rolls the real fight makes,
  then the formation resolver, on a private RNG seeded from the plan (both formations, sorted, the
  plots, the turn). The same plan always shows the same numbers whatever the game's RNGs hold, and
  the game's RNG is never drawn (test `the_preview_is_a_function_of_its_inputs_and_changes_nothing`).
  A move that declares war previews the battle after the declaration (its strikers test war as that
  fight does). It gives each side's expected units fallen and escaped (x100), the chance the
  defender's line is wiped out, the chance the plot falls (the line wiped out with no defender left:
  none in reserve, none that sat out; withdrawn defenders flee), the chance the attacker's line is
  wiped out, and per committed fighter its chance to fall and to escape.
- The round model's What-if alternatives, the swap search and the moment estimator are gone.
- **Time** (release, this 4-core shared machine; `preview_timing`, `--ignored`, 2026-10-31): a plan
  of 200 battles takes 5.4 ms for 6 v 6, 6.9 ms for 8 v 8 and 8.6 ms for 10 v 10; the auto formation
  a few microseconds.

## Telemetry

`engage*` counts and the `engagement` event (docs/TELEMETRY.md): the result (`taken`, `cleared`,
`repulsed`, `held`), the rounds, the slots, the committed attackers and the fallen by role, the
withdrawals, the morale failures, the defensive strikes, the missiles; the per-unit `attack` /
`defend` counts stay comparable with the duel's; each kill, a missile strike's too, counts as a
combat won (`kill_by`, the war's kills, a summon's kills). `telemetry.py`'s combat section prints
them.

## Decided while building it

The spec (the owner's, revision 4) holds; these were left open and are decided so:

- Sat-out attackers are not struck by defensive strikes (they never close in); the Fear source on
  the attacking side must itself fight. A sat-out defender stays on the plot: the plot does not
  fall while it is there.
- A defensive strike that does no damage is still a strike: the striker is used up, as today.
- Battle XP shares are of the battle's damage (the defensive strikes are not a hitter's share).
  Withdrawal XP takes the enemy side's least restrictive XP cap.
- `unit_attack` is forced: the unit given, whatever its moves and attacks this turn, if it is alive,
  can attack and has a role, in the auto formation.
- The fighters' order (the resolver's draw order for first strikes and coins) is the engagement's:
  the attacker's formation by lane then row, then the defender's; so the same formation is the same
  battle however the order lists it.
- A large unit's HP is 150% of its role's; a unit type's own `iHP` is its HP whether large or not.
- The forest flag of the ground is a feature with `iEngagementSlotsChange` below 0 (the forests and
  the jungle); the city flag a city on the plot (a fort is not a city here).
- The auto formation's strength is the unit's A against nobody in particular on its side (its
  attack with its promotions), for both sides.
- Barrage's extra collateral damage and Drill's protection scale only a collateral splash, as in
  the duel: not the hit on the target, not a known spell's splash. The battle log does not list
  them as values (the fighter's promotions name them).
- A missile's hit is a strike's choice without nearby first (it comes from outside the field), with
  no withdrawal (as the round model's missile hit had none). Its target is the formation's choice,
  not the round model's random pick by role order. Its strength is the round model's: half a
  battle's hit, the plot's own defence in Dv, no ground rules.
- The plan of an AI attacker is not frozen into the log (the moment estimator that made it is gone).
- Per-kill game-log messages are replaced by one message per side and engagement, naming the report.
- A medic's heal goes only to a hurt unit, and only then uses its once a turn; the patient's own
  same-tile heal counts.
- A land plot with no unit that can engage but a unit that can still defend in the duel (a ship in
  a port) fights today's duel (see "Open").

## Open

- Taking cities: Sentinels (walls that fight in the defender's front row), sieges that stop a
  city's healing, bombards that hurt walls and soldiers, and kill XP shared equally: specified in
  docs/SIEGES.md (the owner's design, 2026-11-04), not built yet.
- The AI decides its attacks on the duel's odds and attacks in the auto formation (the AI is paused,
  the owner's call). The defender always fights in the auto formation: standing formations per army
  are a later step (`Game::engage_defense_formation` is the hook).
- A city whose only defenders are ships in port is attacked with today's duel (ships never defend
  land in an engagement, but nothing else defends there).
- Phase 7: spells in combat phases, summons in battle, the Guardsman's bodyguard, commander auras
  (the hooks are in place: plan section 14). The three spell hooks, `engage_pre_battle`,
  `engage_opening_spells` and `engage_post_battle_spells` (`engage.rs`), are called at their places
  and return nothing yet; the first two return `BattleEffect`s (a modifier scales the pair table, a
  damage lowers HP before the battle), the third `Cast` records.
