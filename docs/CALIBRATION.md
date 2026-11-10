# Calibration: the AI's estimates as predictions

The AI decides on estimates: this stack takes that city, this garrison holds, this attack wins,
we can take their cities. The prediction ledger records each estimate as a prediction about
something the rules decide, lets the rules resolve it, and scores it, so that any estimator can
be measured as a predictor before a decision is built on it (CLAUDE.md, "How AI changes are
checked", step 3).

It is part of telemetry (docs/TELEMETRY.md): on only when telemetry is on, it draws no random
number, changes no game state and is not saved. The code is `crates/ffh-core/src/telemetry_predict.rs`
(the ledger) and `crates/ffh-ai/src/predict.rs` (the horizons and shared recording).

## API

```rust
use ffh_core::telemetry::{Obj, Outcome, Prediction, HORIZON_THIS_TURN};

// A probability: pct is the chance (0..=100) the outcome happens within the horizon.
g.tel_predict(p, "assault", Outcome::CityCapturedBy { city, team }, 3, pct, || Obj::new().b("covers", ok).end());

// With a raw margin (`score`) and whether the AI acted on it:
g.tel_prediction(p, Prediction::new("war_takes_city", outcome, 40, if takes { 100 } else { 0 }).score(edge).acted(declared), || ctx);
```

- `kind`: the estimator's name (one per estimator and call site that means the same thing).
- `pct`: the predicted chance. A yes/no estimator records 100 or 0 and puts its margin, if it has
  one, in `score`.
- `horizon`: turns after the current turn within which the outcome counts. `HORIZON_THIS_TURN`
  (0) resolves at the end of the predicting player's turn.
- `acted`: whether the AI acted on the estimate. Record declined estimates too: a declined attack
  that would have won is as informative as an attack that lost.
- `ctx`: a JSON object of facts to slice by (`Obj`), built only when events are written and the
  prediction is kept.

## Outcomes

| Outcome | True when | False when | Void when |
|---|---|---|---|
| `CityCapturedBy { city, team }` | a player of `team` owns the city (on its plot: a captured city is a new city) within the horizon | the horizon ends, or no city stands there | – |
| `CityLost { city }` | its owner at prediction time no longer owns it (captured, traded, flipped, razed) within the horizon | the horizon ends with the owner holding it | – |
| `AttackWon { attacker }` | its next attack wins: a duel in which the defender dies, or an engagement on its side that takes or clears the plot (the unit standing) | it falls, or its side is repulsed | it withdraws, the engagement is held, or it makes no attack this turn |
| `StackClears { team, plot }` | no unit of a team at war with `team` that can defend stands on the plot when the horizon ends | one does | – |
| `AnyCityCapturedFrom { team, victim }` | `team` conquers a city of `victim` within the horizon | the horizon ends, or `team` dies | `victim` is destroyed first |
| `CityAttackedBy { city, team }` | an engagement is launched at the city's plot, or a duel fought on it, by `team` (`None`: any team other than the owner's), or the city is conquered by it without one | the horizon ends with the owner holding it | the city changes hands otherwise (traded, flipped, conquered by another team, razed) |

Resolution happens at the events (`player_acquire_city`, a duel, an engagement; `CityAttackedBy` at
the engagement's launch, `tel_autopsy_battle`, before its battle), at the end of
each player's turn (this turn's `StackClears` and `AttackWon`) and at the end of each game turn
(cities by their plots, horizons that ran out). Predictions still pending when the game ends are
not scored (`open` in the aggregates).

## De-duplication

The AI re-evaluates the same estimate every turn. The ledger keeps one pending prediction per
player, kind and subject (the outcome's city plot, unit, plot or pair of teams): the first one
made, until it resolves. Later calls for the same key do nothing (their `ctx` is not built). So
each resolved prediction is one sample: a city judged "in peril" for eight turns before it falls
counts once, at the first turn. One exception: within the turn it was made, a prediction the AI
declined (`acted(false)`) gives way to the same one acted on, so that the decision of that turn
is the one scored.

**By answer** (`Prediction::per_answer()`): for a yes/no state estimator the first answer is not
the only one worth scoring: a city judged "not in peril" at turn t and "in peril" at t+3 is two
answers. A prediction marked `per_answer` adds the tenth of its pct (0-9, .., 90-100) to the key, so
each distinct answer about a subject is scored on its own horizon from the turn it was first given
(the same answer given again waits for the pending one). Used by `peril`, `garrison_holds`,
`sortie`, `holds2_now` and `holds2_reinforced`. Samples of one city are then correlated; the
counts below are of answers, not of cities.

This also means a probability is scored at its first appearance, not its last: an `assault`
prediction made while the stacks are still gathering is the one resolved, not the sharper
estimate of the turn they strike. Kinds that should be scored at the moment of the decision
(`strike`, `attack`) use a horizon of 0.

## The estimators hooked

| Kind | Where | Outcome, horizon | pct | acted |
|---|---|---|---|---|
| `attack` | the unit the AI sends: `campaign::strike` (each attacker), `military::assault` (the attacker the rules pick), `units::attack_adjacent` | `AttackWon`, this turn | the odds it weighed (`get_combat_odds`; the field simulation's first-attack odds) | always |
| `sea_attack` | the warship the AI sends against ships on a water plot: `warships.rs` (`src` kill, gang, barbarian; ctx `prize`) | `AttackWon`, this turn | the rules' duel odds (`get_combat_odds`) | always |
| `field` | `military::assault` (go or not), declined targets in `best_adjacent` and the opportune capture | `StackClears`, this turn | `Sim::p_fall`; score `gain` | go |
| `assault` | `campaign::build_op` when a stack is next to a city we are at war with | `CityCapturedBy`, 3 turns | `p_fall` of the adjacent ring (no reserve); score: defender hit points left | the phase is `Strike` |
| `strike` | `campaign::strike`, before the first attack | `CityCapturedBy`, this turn | `p_fall` of the units able to attack | the strike starts |
| `garrison_holds` | `production::garrison_holds` with a threat in sight | `CityLost`, 10 turns | 0 when it holds, else 100; score defense - threat | holds |
| `peril` | `View::analyze`, each city with a threat in sight | `CityLost`, 10 turns | 100 in peril (threat above 1.5x the garrison), else 0; score threat - defense | – |
| `sortie` | `defense::sortie_ok` with an enemy next to the city | `CityLost`, 3 turns | 0 when a sortie is allowed, else 100; score our margin | allowed |
| `war_takes_city` | `diplomacy::manage` at a prepared war's declaration (acted) or drop at the re-check | `AnyCityCapturedFrom`, 40 turns | `can_take_cities_at` at the re-check's 100% margin, yes/no; score attack / needed in percent | declared |
| `fight_on` | `diplomacy::manage`, a cash-out refused because our army is at their next city | `AnyCityCapturedFrom`, 10 turns | 100 | always |
| `incoming3`, `incoming6`, `incoming10` | `View::analyze` (`predict::incoming`), once per player-turn: every city with a hostile stack in reach, and at war with a civilization every other city too (pct 0); by answer | `CityAttackedBy { team: None }`, 3 / 6 / 10 turns | `threat::incoming`'s chance of an attack within the horizon | – |
| `lost2_as_is`, `lost2_reinforced` | the same cities | `CityLost`, 10 turns | the chance of an attack within 10 turns x the forecast of the expected force against the garrison as it stands / reinforced; score the forecast (per mille); by answer | – |

Beside each of these, the rules' battle forecast (`forecast.rs`, below) answers the same question
as its own kind, with the same outcome and horizon and the old decision's `acted` (the AI still
acts on the old estimate, so the pairs are scored on identical events):

| Kind | Beside | pct |
|---|---|---|
| `field2` | `field` (every site) | the plot falls in the battles the pool on the group's plot fights this turn (its formation, then its reserve); ctx `one`: in the first battle alone |
| `assault2`, `strike2` | `assault`, `strike` | the city falls in the battles the units next to it fight, the plot with the most of them first; ctx `one` |
| `attack2` | `attack` (every site) | the battle the unit's plot fights wipes out the defender's line (taken or cleared) times the chance the unit does not fall |
| `holds2_now`, `holds2_reinforced` | `peril` (each city with a threat in sight; `garrison_holds` asks the same question of the same cities and is not repeated) | the city falls to the worst of the enemy stacks within reach and the union of one player's stacks, as the garrison stands / with the units of ours that arrive first; by answer |

`Sim::p_fall` (`combat_sim.rs`) is the product over the defenders of the simulation's chance each
has fallen, treated as independent: the simulation's own idea of "every defender falls", which
`covers` turns into yes/no at its sureness thresholds.

Not hooked: `defense::deterrence` (it predicts what a rival's war gate would decide, not an
outcome the rules resolve); `diplomacy::can_take_city` in `naval.rs` (an amphibious target's
choice, with no time at which the landing is expected); the war-of-expansion gate when a war is
only prepared (the declaration's re-check is the estimate acted on); `end_war_val` and the peace
offers (they price a peace, they do not predict an event).

## Adding a prediction for a new estimator

1. Find where the estimate is computed and the decision taken on it; pick the `Outcome` the
   estimate is about (add a variant in `telemetry_predict.rs` only if none fits and the rules
   can resolve it without AI code).
2. Pick a horizon (a named constant in `crates/ffh-ai/src/predict.rs`) and a pct: the
   estimator's probability, or 100/0 with its margin as `score`.
3. Record both branches, acted and declined, behind `if g.telemetry.on()`, computing nothing
   the decision does not already compute unless telemetry is on.
4. Put the facts to slice by in `ctx` (`Obj::new().b(..).i(..).end()`).
5. Check that nothing changed: `FFH_TELEMETRY` unset, the same seed gives the same `ffh stats`
   checksum before and after; the test `crates/ffh-ai/tests/telemetry.rs` passes.

## Output and the report

Each resolved prediction is one event in `FFH_TELEMETRY_FILE`:
`{"t":turn_resolved,"e":"prediction","p":..,"kind":..,"civ":..,"turn_made":..,"turn_resolved":..,"horizon":..,"pct":..,"score":..,"acted":..,"outcome":true|false|null,"void":true?,"ctx":{..}}`.
The aggregates are in the stats JSON, `telemetry.predictions.<player>.<kind>`: `n`, `void`,
`hits`, `sum_pct`, `brier` (sum of (pct - 100 x outcome)^2; the Brier score is
`brier / n / 10000`), `bins` (ten `[count, hits]` by predicted pct) and `open`.

```
python3 crates/ffh-cli/scripts/calibration.py --bin $W --seeds 1-24 --turns 300 --out DIR
python3 crates/ffh-cli/scripts/calibration.py --load DIR [--kind assault,strike] [--slice acted] [--by civ] [--json]
```

Per kind: N resolved and void; the mean predicted pct against the observed rate (a gap is a
bias: the estimator is optimistic or pessimistic overall); the Brier score against the reference
of always predicting the observed rate (skill below 0: the estimator knows less than the base
rate); the reliability table (each tenth of predicted pct against what happened: a calibrated
estimator's rows sit on the diagonal); and the 2x2 table at `--threshold` (50%) with precision
(when it says yes, how often it happens) and recall (of what happened, how much it said). For a
yes/no estimator the 2x2 table is the whole story. `--slice acted` separates the estimates acted
on from those declined: a declined estimate is not tested by an action of ours (a declined
`field` attack resolves on what the plot holds anyway, usually false), so judge an estimator's
calibration on the acted slice and its missed chances on the declined one.

## The battle forecast (`forecast.rs`)

An estimator built on the rules' own battle code (CLAUDE.md, step 3: "prefer an estimator that runs
the rules' own code"): the engagement preview's forecast (`Game::engage_preview`, docs/ENGAGEMENTS.md,
"The attack preview") made for battles that are not on the board yet. Measured here against the
old estimators. The AI's attack decisions read it since switch `FORECAST` (docs/AI.md, "On the
forecast" under Field armies; `crates/ffh-ai/src/military.rs`, `campaign.rs`); the ledger's kinds
below were measured before, when no decision used it. Recording stays pure: `ffh stats` gives the
same game with telemetry on and off.

### In the rules (`crates/ffh-core/src/engagement/preview.rs`)

- `Game::engage_forecast_if(attackers, from, target, extra_defenders, runs, sight, when)`: one
  battle of `attackers` (one player's, wherever they stand) attacking `target` as if from `from`
  (a plot next to it: the approach's slots, river crossing and ground), against the defenders on
  `target` plus `extra_defenders` as if they stood there. It builds the preview's own plan
  (`engage_plan_finish`: the defender's formation, the summons in a copy of the game, the set-up)
  and samples it with the preview's per-battle code (`engage_sample_battle`) on the preview's seed
  (both formations, the plots, the turn). For a real attack (the pool on `from`, no extras) it
  equals `engage_preview(..).forecast` at the same runs (test
  `the_forecast_if_is_the_preview_of_the_battle_it_imagines`, which also moves the imagined units
  there for real and gets the same numbers).
  - The pool: units on `from` that can attack now as the real fight picks them, others if they
    could stand on `from` and attack (`engage_could_attack_from`: a land unit able to attack, not
    marching, not a missile, within its combat limit; moves not counted). `when`:
    `ForecastTurn::Now`, or `ForecastTurn::Next` (moves and attacks restored: an enemy stack
    forecast during our turn; without it, enemy stacks next to a city with their moves spent had
    no attacker and the city looked safe).
  - `sight`: `ForecastSight::Attacker` refuses a target the attacker's team does not see, as the
    preview does; `ForecastSight::Defender` does not test it. The AI's layer passes `Defender` and
    applies its own rule (`view::sees_units`, the fog rule of its attacks; `known_threat` for the
    enemy units it defends against). Invisible units are never defenders either way (the rules'
    candidates). An extra defender keeps its own state (damage, fortification).
  - `Err`: attackers of several players, `from` not next to the target, a water target, no unit
    that could attack, nobody to fight, the target unseen (`Attacker`).
- `Game::engage_forecast_waves(attacks, target, extras, runs, sight, when)`: the battles a turn's
  attacks fight one after another: each entry's pool in its auto formation, then its reserve in
  theirs (every unit once, at most `FORECAST_MAX_WAVES` = 8 battles), then the next entry. In each
  run the defenders that fell or fled are gone, the survivors keep their damage, the strikers used
  up stay used up, the city's Sentinels keep their HP; each battle meets the defenders left (its
  plan built once per set of defenders left). Approximation: the defender's formation of the
  survivors is the rules' auto formation from the game's state (earlier damage in the run does not
  reorder it). Test `the_forecast_of_waves_carries_the_battle_on`.

Cost (release, this shared 4-core machine; test `forecast_if_timing`, `--ignored`): one battle at
200 runs (the preview's) ~1.5 ms small (3 v 2), 3.3 ms medium (6 v 4), 9.8 ms large (13 v 10 in a
city); at 50 runs 0.4 / 1.0 / 3.3 ms; at 20 runs 0.19 / 0.47 / 1.6 ms: linear in runs and in
battles (waves).

### The AI's layer (`crates/ffh-ai/src/forecast.rs`)

- `attack(g, cache, units, from, target) -> Option<Est>`: one battle (`RUNS` = 60).
- `attack_turn(g, cache, attacks, target)`, `assault(g, cache, attacks, city)`: the turn's battles
  (`by_plot` groups our units next to the target by plot, the plot with the most first).
- `Est`: `p_take` (the plot falls), `p_wipe`, `p_repulsed` (per mille), own and enemy losses
  (x100), committed fighters per side.
- `holds(g, cache, flood, city) -> Hold`: the enemy units the owner sees (`known_threat`; at war,
  land units able to attack, no animals, no garrison in its own city) within `HOLD_RADIUS` = 3,
  grouped by plot and owner; each stack's approach (the first plot next to the city its slowest
  unit reaches, `reach::Flood` as that unit moves) and turns (at most `HOLD_STRIKE_TURNS` = 2; the
  soonest `HOLD_MAX_STACKS` = 6). Each stack's battles (`ForecastTurn::Next`, `HOLD_RUNS` = 40) as
  the garrison stands and with the units of ours that reach the city first (the nearest
  `REINFORCE_K` = 6 within `REINFORCE_RADIUS` = 6, each flooded to the city); and the union of one
  player's stacks, each from its own approach. `Hold`: `p_fall_now`, `p_fall_reinforced` (the worst
  of single and union), the parts, the reinforcements, the turns to the soonest strike, the stacks.
- `Cache`: one per player and game turn, kept by the controller (`Ai::forecasts`) and lent to the
  `View`; entries keyed by the inputs (sorted) and checked against a fingerprint of what changes
  within a turn (every unit involved and on the target: place, damage, moves, attacks, experience,
  level, promotions, fortification; the target's owner and its city's defence damage).

Calls (16 games, 300 turns, telemetry on, every shadow prediction above): 5,190 forecasts asked per
game (17 per game turn; 6.5 per player-turn call that asked any), 4% answered by the cache; the
games took +6.5 s each (+97% of the game time with telemetry on; ~1.3 ms per forecast played,
waves included). Counted in the stats JSON as `decisions.forecast` (`calls`, `played`).

### Calibration against the old estimators

16 games (seeds 1-16, 6 civs, small map, 300 turns), the acted slice (`calibration.py --slice
acted`): the same events for each pair.

| Pair (acted) | N | predicted | observed | Brier | skill |
|---|---|---|---|---|---|
| `field` | 2191 | 91.5% | 85.7% | 0.099 | +0.20 |
| `field2` | 2215 | 86.9% | 85.8% | 0.053 | **+0.57** |
| `strike` | 291 | 84.4% | 90.0% | 0.102 | -0.14 |
| `strike2` | 291 | 94.0% | 90.0% | 0.048 | **+0.47** |
| `assault` | 269 | 85.0% | 87.7% | 0.124 | -0.15 |
| `assault2` | 265 | 92.2% | 87.5% | 0.117 | -0.07 |
| `attack` | 2765 | 72.8% | 92.9% | 0.194 | -1.95 |
| `attack2` | 2737 | 87.8% | 92.8% | 0.034 | **+0.48** |

Reliability (tenth of predicted pct: n / observed):

- `field`: 0-9 73/18%, 60-69 33/42%, 70-79 115/49%, 80-89 250/84%, 90-100 1709/93%. `field2`:
  0-9 115/8%, 10-19 22/18%, 20-29 30/27%, 30-39 54/24%, 40-49 50/40%, 50-59 50/56%, 60-69 55/69%,
  70-79 43/67%, 80-89 51/84%, 90-100 1745/98%: on the diagonal.
- `strike`: 40-49 9/78%, 50-59 22/77%, 60-69 30/77%, 70-79 28/89%, 80-89 41/95%, 90-100 157/94%
  (pessimistic in the middle). `strike2`: 0-29 12/0%, 60-69 5/40%, 70-79 5/60%, 80-89 6/67%,
  90-100 260/97%.
- `assault`: 40-49 7/100%, 50-59 17/82%, 60-69 25/88%, 90-100 153/92%. `assault2`: 0-9 13/62%,
  90-100 231/92%. Its misses are of the horizon (3 turns: units that come after the estimate join
  the assault), not of the battle.
- `attack`: 0-9 215/91%, 20-29 168/94%, 30-39 97/95% (no skill). `attack2`: 0-9 147/5%, 10-49
  141/74%, 50-89 264/94%, 90-100 2185/99.9% (pessimistic in the middle: the unit's fall and the
  line's wipe are not independent, a unit falls less when its side wins). These `attack2` numbers
  are of `p_wipe` x (1 - fall) as the code now records it, rescored from the same games' ctx; the
  run recorded `p_take` x (1 - fall), which missed the attacks that cleared a plot and left a
  reserve (0-9: 224/37%). Under engagements "this unit wins" keeps a meaning (the ledger's
  `AttackWon`: its side takes or clears the plot and it does not fall), but it is the stack's
  question with a unit's survival attached: decisions belong on `field2`.

What the first version got wrong, found in the misses and fixed: it forecast one battle against an
outcome of the turn (`field2` 90-100: 68% observed; `strike2` 0-9: 58% taken): a plot falls to the
reserve and to the next plot's stack after the first battle, so the forecast now plays the turn's
battles (`engage_forecast_waves`); and the enemy stacks with their moves spent (below).

Defensive (every answer, by answer; outcome: the city lost within 10 turns):

| Estimator | N | predicted | observed | Brier | skill | at 50%: precision / recall | best F1 |
|---|---|---|---|---|---|---|---|
| `peril` | 4279 | 20.2% | 9.0% | 0.172 | -1.10 | 30% / 66% | 0.41 (yes/no: any threshold) |
| `holds2_now` | 4605 | 23.7% | 9.1% | 0.167 | -1.01 | 27% / 69% | 0.44 at 98%: 38% / 53% |
| `holds2_reinforced` | 4221 | 16.8% | 11.4% | 0.127 | -0.26 | 40% / 59% | 0.49 at 73%: 45% / 54% |

`holds2_reinforced` reliability: 0-9 3226/5%, 10-19 111/12%, 20-29 65/18%, 30-39 60/12%, 40-49
55/22%, 50-59 44/18%, 60-69 44/18%, 70-79 65/31%, 80-89 61/33%, 90-100 490/46%.

The defensive forecast answers "if the stacks attack, does the city fall"; the outcome also needs
the attack. Split by whether an engagement was fought on the city within the horizon (the
`engagement` events at its plot): not attacked, 1% of the answers saw the city lost whatever the
forecast said (90-100: 2%); attacked, 89-92% did (by civilizations only, 97%). The attacks come when
the attacker has the force, often more than was in reach when the answer was given: of the answers
followed by an attack 5-10 turns later, `holds2_reinforced` had said 0-9% for 73 of 121, and 79% of
those cities fell. Where the attack came on the turn of the answer or the next, the battle is
forecast close (`holds2_now`: predicted 79%, observed 90%; 0-9 9/44%, 90-100 53/96%). So the battle
is forecast well and the attack not at all: a decision on `holds2` needs a second estimate, whether
(and with what) they will attack, which the ledger can measure the same way. The reinforced figure
assumes the units come: of the 18 cities lost at once with `holds2_reinforced` below 30%, 12 had
reinforcements counted (whether they came is not recorded).

Done since: the strike and field go/no-go on `strike2` and `field2` (switch `FORECAST`). Not done:
the formation of the survivors reordered by their damage within a run of waves. The
attack-likelihood estimate is below; no defence is built on it yet.

## Incoming attacks (`threat.rs`)

The second half of the defensive question: will a stack attack this city, when, and with what.
Recorded in the ledger only; **no decision uses it**, and with telemetry off none of it runs
(`ffh stats` gives the same checksum before and after, with telemetry on or off: checked on seeds
1-4, 300 turns).

### The estimator (`crates/ffh-ai/src/threat.rs`)

- **Stacks** (`Threats::of(g, cache, flood, p)`): land units of teams at war with `p` (barbarians
  included) that `known_threat` lets it see (through fog, not through stealth), able to attack, no
  animals, no garrisons in their own cities; grouped by owner, units on the same or adjacent plots
  merged. Each stack floods as its slowest unit moves (`reach::Flood`, full moves, the rules'
  costs) for `THREAT_TURNS` = 6 turns; its candidates are our cities it reaches: the soonest plot
  next to each and the turns to stand there (0: it can attack on its next turn).
- **Forces**: each owner's stacks are one army. For the soonest `MAX_CANDIDATES` = 4 cities it
  reaches, the rules' forecast (`forecast_turn`, `ForecastTurn::Next`, `TARGET_RUNS` = 20) of the
  army gathered there by each arrival turn (at most `MAX_STEPS` = 3 steps; all the units that have
  arrived, from the approach of the biggest stack) against the garrison as it stands, until it takes
  the city with at least `TAKE_BAR` = 50%: `take_turns`. No forecast for barbarians.
- **The rule**, percent chance that an owner attacks a city within `h` turns: the city it can take
  soonest is its target (ties: the better chance, the nearer city). Target taken at turn `w < h`:
  `P_TARGET` (40) x (2/3)^w; another city it takes (the army walks on to the next): `P_NEXT` (25) x
  (2/3)^w; a city beyond the forecast ones of an owner with a target counts as one it takes, from
  the later of the turn it reaches it and the target's turn. Taken only at `w >= h`: `P_LATE` (1).
  Reached within `h` and not taken: `P_REACH` (1). Any city a barbarian stack reaches within `h`:
  `P_BARBARIAN` (1). A city's chance: at least one owner attacks (owners independent). The levels
  were read off the attacks per category in seeds 1-16 (the table below) and not checked on other
  seeds.
- **`threat::incoming(g, cache, flood, threats, city) -> Incoming`**: `p_attacked_within` for
  `HORIZONS` = [1, 3, 6, 10] turns; `turns_until` (the soonest an owner takes it, else the soonest
  stack next to it); `force` (each expected stack with its approach and turns: the owners that take
  it, their stacks there by then; else every stack in reach); `p_lost_as_is` (the worst single
  owner's army against the garrison, `CITY_RUNS` = 40) and `p_lost_reinforced` (the same with our
  units that reach the city by `turns_until`: the nearest `REINFORCE_K` = 8 within the best-case
  distance, each flooded to the city); counts of owners, stacks, takers; the rule's category.
- No memory: only what is in sight this turn. With `FOG_SIGHT` on (the default) the AI sees enemy
  units through fog, so the cost of no memory is in stealth and units not yet built (below).

The shadow predictions of this section are recorded only with `FFH_AI_SHADOW_THREAT=1`
(set it in the environment of `calibration.py`; the games inherit it): as built the estimator costs about 2.8 times the game's own time.

The recording (`predict::incoming`) also writes, with events on, one `decision` event `threat`
per player-turn whose `alts` holds every city's estimate (and the land military of civilizations
not at war with us within 4 plots), every stack and every force: the misses below come from it.

### Calibration (seeds 1-16, 6 civs, standard map, 300 turns)

The ledger (by answer: one sample per city and distinct tenth of pct):

| Kind | N | predicted | observed | Brier | skill | best F1 |
|---|---|---|---|---|---|---|
| `incoming3` | 21616 | 2.7% | 2.8% | 0.020 | +0.26 | 0.48 at >=20%: 38% / 66% |
| `incoming6` | 13503 | 4.1% | 5.4% | 0.038 | +0.27 | 0.52 at >=20%: 46% / 59% |
| `incoming10` | 9304 | 5.2% | 8.9% | 0.062 | +0.24 | 0.53 at >=20%: 52% / 54% |
| `lost2_as_is` | 9199 | 4.4% | 8.7% | 0.060 | +0.25 | 0.53 at >=10%: 41% / 78% |
| `lost2_reinforced` | 8607 | 2.9% | 7.8% | 0.055 | +0.24 | 0.58 at >=10%: 52% / 66% |
| `peril` (same games) | 4279 | 20.2% | 9.0% | 0.172 | -1.10 | 0.41 (yes/no): 30% / 66% |
| `holds2_reinforced` | 4221 | 16.8% | 11.4% | 0.127 | -0.26 | 0.48 at >=70%: 43% / 55% |

(The pcts are low, so precision / recall at 50% say nothing: under 1% of the answers reach it.)
Reliability (n / observed): `incoming6` 0-9 11570/1%, 10-19 994/16%, 20-29 463/36%, 30-39 52/37%,
40-49 394/57%, 50-59 19/68%; `lost2_reinforced` 0-9 7759/3%, 10-19 271/36%, 20-29 212/52%, 40-49
265/68%. Ordered and useful, but under-confident by a factor of about 1.5 in the ledger's sampling
(each answer at its first appearance); the per-turn rates by category the levels came from are
those of every turn. `peril` and `holds2` are recorded only for cities with a threat in sight, so
the sets differ: compare F1 and the reliability, not N.

The first version (a per-stack target, every stack independent, levels guessed) had skill -1.5 to
-2.2: barbarian stacks attack a city 0.5% of the time whatever they could take, and a civilization
attacks with its stacks gathered, so a stack that could take nothing alone was the attack 8.7% of
the time. Per category, per (owner, city) and turn of trace (seeds 1-16, attacked by that owner
within 6 turns): target taken at turn 0: 43%, 1: 29%, 2: 15%, 3: 9%, 4: 6%, 5: 4%, 6+: 1%; another
city it takes, at turn 0: 27%, 1: 14%, 2: 10%, 3: 7%, 4: 3%, 5+: 1-2%; reached, not taken: 1.0%;
barbarian: 0.4%; no stack in reach: 0.2%.

### The misses

Onsets of an attack (the first on a city in 10 turns), seeds 1-16: 212 by civilizations, 23 by
barbarians (all 23 with a barbarian stack in reach, rated 1%). Where the 212 civilization onsets
stood in the defender's trace before them:

| | 0 turns before | 2 before | 5 before | 9 before |
|---|---|---|---|---|
| the attacker's army in reach and rated as taking the city (its target or a next city) | 206 (97%) | 183 (86%) | 142 (67%) | 96 (45%) |
| ... of which its target | 184 | 109 | 52 | 24 |
| ... of which another city was its target | 22 | 74 | 90 | 72 |
| in reach, could not take it as it stood | 2 | 6 | 13 | 12 |
| units seen, none within 6 turns of this city | 0 | 0 | 2 | 5 |
| no unit of the attacker seen (not yet built, stealth, in ships) | 2 | 12 | 29 | 56 |
| a stack at sea | 0 | 1 | 0 | 0 |
| not at war yet | 2 | 7 | 20 | 35 |
| ... with its land military within 4 plots of the city | 2 | 6 | 13 | 21 |
| the city not ours then | 0 | 3 | 6 | 8 |

- **Assigned to another city** is the largest seen miss: 90 of the onsets 5 turns ahead were rated
  as a next city of an army whose target was elsewhere (low pct: `P_NEXT` decayed by the turn). The
  armies are large and walk from one city to the next; which one comes first within a few turns is
  the part the rule gets wrong, not that the army is coming.
- **Unseen**: 14% of the onsets 5 turns ahead, 26% 9 turns ahead, had no unit of the attacker in
  sight at all, and 1-2% had units beyond 6 turns of every city. Memory would not catch these (the
  AI already sees through fog): they are armies built or arriving later, or hidden by stealth.
  Extending the flood beyond 6 turns would catch at most the 1-2%.
- **Declarations**: 24 of the 212 onsets (11%) came within 3 turns of the declaration of the war (0:
  2, 1: 5, 2: 9, 3: 8); 9% (5 turns ahead) to 17% (9 turns ahead) of the onsets were by a
  civilization not at war with the city's owner when the estimate was made, two thirds of them with
  their military already within 4 plots. Catching these needs reading neutral stacks at the border;
  out of scope here.
- **Sea landings**: under 1%.
- **False alarms**: of the (owner, target) pairs rated at turn 0, 57% are not followed by an attack
  within 6 turns. Not broken down further.

### Cost

Per game (300 turns, 6 civs, standard map): 1,825 calls (once per player-turn), 40,000 floods (most
of them the reinforcement search, up to 8 per threatened city) and 9,500 forecasts. With telemetry
on, seeds 1-4: 32.8 s a game against 13.9 s for the same games without the estimate, and 6.7 s
with telemetry off: about 10 ms per player-turn, **about 2.8 times the game's own time, far above
the 10% a per-turn decision could afford**. Before a decision reads it: the forecasts of the target
choice at fewer runs or only when an owner's army changes, one reverse flood from each city for the
reinforcements instead of one flood per unit, and the per-player stacks shared within a game turn.
