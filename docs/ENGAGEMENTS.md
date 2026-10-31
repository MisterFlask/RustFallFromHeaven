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
`pick.rs`, `engage.rs`, `apply.rs`, `sink.rs`, `report.rs`). Scenario tests:
`crates/ffh-core/tests/engagements.rs`; the round model's tests are in its modules.

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
   the stance's proposal. The defender's candidates: every unit on the plot that can defend, is an
   enemy (hidden nationality as in the duel) and is not invisible to the attacker; its pick: its
   stance's (the garrison's in a city, else the largest army's). An empty force or no defender is a
   refusal, and nothing has changed.
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
   Neither kills.
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
   a turn per unit (Q12); withdrawn defenders flee; a Fear attacker that won outside a city routs the
   rest (Q8); every committed attacker has made its attack (one a turn, Blitz one more, counting
   engagements, bombards and missile strikes) and pays the attack's moves unless it moves in; with
   no visible enemy defender left, the launching group moves in (a city is captured as today), and
   only it; the armies of everyone involved are settled.
6. **The report** (`report.rs`): every engagement's summary (sides with their real and shown
   owners, stances, committed, lost, withdrawn, sat out, fled; the outcome) is kept for the current
   and previous turn and the last `ENGAGEMENT_REPORTS_KEPT`; a battle a human could see also gets
   its full battle log (below). A game-log message to each side's players names the report.

It never sets the duel's combat state (`combat_unit`, the attack plot, combat timers), and nothing
outlives the call. Every loop is bounded (finite lists once each; volley rounds by the largest
volley count; rounds by 40), so every engagement ends; a turn holds at most `1 + Blitz` engagements
per unit in a pool, since each spends an attack of every committed unit.

**Missiles** (Q11): a missile (Fireball, Meteor) ordered into an enemy plot strikes instead:
one hit of the round model on a unit of the defender's pick (by the missile's role order), then
its collateral; it dies, with the kill if it killed.

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
counts stay comparable with the duel's; each kill counts as a combat won (`kill_by`, the war's
kills). `engage_est` records the moment estimator's win chance for the battle as fought against its
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

- The AI decides its attacks on the duel's odds until step 8 (the estimator is step 7).
- A city whose only defenders are ships in port is attacked with today's duel (ships never defend
  land in an engagement, but nothing else defends there).
- Phase 7: spells in combat phases, summons in battle, the assassin's and the Guardsman's targeting,
  commander auras (the hooks are in place: plan section 14).
