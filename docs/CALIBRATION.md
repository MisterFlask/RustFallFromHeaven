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

Resolution happens at the events (`player_acquire_city`, a duel, an engagement), at the end of
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

This also means a probability is scored at its first appearance, not its last: an `assault`
prediction made while the stacks are still gathering is the one resolved, not the sharper
estimate of the turn they strike. Kinds that should be scored at the moment of the decision
(`strike`, `attack`) use a horizon of 0.

## The estimators hooked

| Kind | Where | Outcome, horizon | pct | acted |
|---|---|---|---|---|
| `attack` | the unit the AI sends: `campaign::strike` (each attacker), `military::assault` (the attacker the rules pick), `units::attack_adjacent` | `AttackWon`, this turn | the odds it weighed (`get_combat_odds`; the field simulation's first-attack odds) | always |
| `field` | `military::assault` (go or not), declined targets in `best_adjacent` and the opportune capture | `StackClears`, this turn | `Sim::p_fall`; score `gain` | go |
| `assault` | `campaign::build_op` when a stack is next to a city we are at war with | `CityCapturedBy`, 3 turns | `p_fall` of the adjacent ring (no reserve); score: defender hit points left | the phase is `Strike` |
| `strike` | `campaign::strike`, before the first attack | `CityCapturedBy`, this turn | `p_fall` of the units able to attack | the strike starts |
| `garrison_holds` | `production::garrison_holds` with a threat in sight | `CityLost`, 10 turns | 0 when it holds, else 100; score defense - threat | holds |
| `peril` | `View::analyze`, each city with a threat in sight | `CityLost`, 10 turns | 100 in peril (threat above 1.5x the garrison), else 0; score threat - defense | – |
| `sortie` | `defense::sortie_ok` with an enemy next to the city | `CityLost`, 3 turns | 0 when a sortie is allowed, else 100; score our margin | allowed |
| `war_takes_city` | `diplomacy::manage` at a prepared war's declaration (acted) or drop at the re-check | `AnyCityCapturedFrom`, 40 turns | `can_take_cities_at` at the re-check's 100% margin, yes/no; score attack / needed in percent | declared |
| `fight_on` | `diplomacy::manage`, a cash-out refused because our army is at their next city | `AnyCityCapturedFrom`, 10 turns | 100 | always |

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
