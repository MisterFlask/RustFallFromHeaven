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

Code: `crates/ffh-core/src/engagement/` (`mod.rs` lists the modules: the round model in
`battle.rs`, `resolve.rs` and `expect.rs`; the rules in `state.rs`, `stats.rs`, `slots.rs`,
`pick.rs`, `engage.rs`, `apply.rs`, `sink.rs`, `report.rs`; the estimator in `estimate.rs` and the
attack preview in `preview.rs`). Scenario tests: `crates/ffh-core/tests/engagements.rs`; the
estimator's and the preview's: `tests/engagement_estimate.rs`, `tests/engagement_preview.rs`; the
round model's tests are in its modules.

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
  and damage moments, the reserve its expected collateral, until the expected defenders left are
  below one half, no attacker is left, or 12 engagements; shaped like the AI's `combat_sim::Sim`.
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
