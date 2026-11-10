# Probes: AIs that go all in on one strategy

A probe is the standard AI (`crates/ffh-ai`) with a few of its priorities overridden so that it
pushes one strategic direction as hard as it can and neglects the rest (CLAUDE.md, "Probes"). Probes
are measuring instruments: opponents unlike the standard AI (the standard AI meets an army it would
never build), and tests of whether a strategy is over- or underpowered. They do not play in the
released game, and no test asserts how their games go.

The code is `crates/ffh-ai/src/probes.rs`. It is not a second AI: every override is consulted at one
of the standard AI's own decision points, and everything a probe does not override plays as
standard. Nothing names a unit, tech, building or civilization, and no turn number is fixed:
attackers and defenders are what `production::best_unit` picks by the rules' strengths, targets come
from the reach and power helpers of `diplomacy.rs`, readiness from `combat_sim`. Every tuning
number is a named constant at the top of `probes.rs`.

## Choosing a probe

One variable per probe, read like the off-by-default switches (`switches.rs`): a comma-separated
list of civilizations (`HIPPUS` or `CIVILIZATION_HIPPUS`, any case) or `ALL`.

| Variable | Probe |
|---|---|
| `FFH_AI_PROBE_RUSH_ONLY` | Rush |
| `FFH_AI_PROBE_HORDE_ONLY` | Horde |
| `FFH_AI_PROBE_TURTLE_ONLY` | Turtle |
| `FFH_AI_PROBE_EXPAND_ONLY` | Expand |

A civilization listed for two probes plays the first in that order. Barbarians and human players
never probe. The table is read once (play stays deterministic, and the run cache keys runs by every
`FFH_*` variable). With no variable set every hook answers "standard": `ffh stats` output is
byte-identical to a build without the module (checked on seeds 1 and 2, 200 turns).

The probe is recorded in telemetry as the intent `probe` (`FFH_TELEMETRY=1`: the player's
`timelines.probe`), and its own decisions carry `probe_*` reasons: production `probe_rush_army`,
`probe_rush_building`, `probe_horde_army`, `probe_turtle_defense`; research `probe_rush_attacker`;
war preparation `probe_rush` / `probe_horde`, with war reasons `probe rush: ...` and
`probe horde: ...`.

## The probes

**Rush: kill the nearest neighbour early.**
- Target: once it has `RUSH_MIN_CITIES` (2, the standard war gate's own minimum) cities, the nearest
  civilization by land (`diplomacy::reach_distance`) it can declare on; it prepares a total war at
  once. When the target is dead or out of reach, or peace was made, the next nearest.
- No settlers from the moment it has a target (a settler in production then is the most it adds);
  one defender per city (the standard's emergency defender under a visible threat stays); no second
  defender.
- Every city trains the army: what the campaign's force lacks (`campaign::army_choice`: siege
  engines for walls), else the best attacker (`best_unit`, city attack at war or while preparing).
  A quarter of the decisions (`RUSH_BUILDING_SHARE_PCT`) may go to a building that makes units
  faster or stronger (production, military production, experience, promotions, unit tiers) done
  within `RUSH_BUILDING_MAX_TURNS`. One more unit only while `ARMY_RESEARCH_FLOOR_PCT` (20%)
  research stays affordable: past that the standard ladder decides (at 0% a rush trained warriors
  until bankrupt and never reached a better attacker).
- Research: toward the cheapest (research left, prerequisites included) land attacker of its
  civilization at least `RUSH_ATTACK_STEP_PCT` (120%) as strong against cities as its best
  trainable one, with the resources it needs owned; otherwise standard.
- Declaration: as soon as its army units near the target (no farther than its nearest city plus
  `RUSH_STAGE_SLACK`) take the target's nearest city with a reserve (`combat_sim::covers`, siege
  engines bombarding). The standard staging and campaign code then fights the war.
- Peace: none while it is not losing (the standard's test) and the enemy has a city on a landmass
  it shares that its best attacker can take (`diplomacy::can_take_city`) or its army there covers.
  Raze or keep by the standard rule.

**Horde: mass and never stop.**
- Standard opening and economy. From `HORDE_MIN_CITIES` (3) cities on, `HORDE_ARMY_SHARE_PCT` (67%)
  of the cities' production decisions (a deterministic draw per city and turn) go to the army (the
  campaign's force, else the best attacker), while `ARMY_RESEARCH_FLOOR_PCT` research stays
  affordable.
- Always at war or preparing one: at peace, it prepares a total war on the reachable civilization
  of least defensive power (the rules' `getDefensivePower`); it declares on the standard readiness
  test (the army at `army_needed_for_war` and staged at the border after 5 turns, or after 40 turns
  in any case), without the standard's re-check of the target. Peace as standard.

**Turtle: never be taken, out-tech everyone.**
- Never declares war (preparations dropped, war bribes refused) and takes every peace offered, and
  offers peace in every war it is in.
- Garrison: `TURTLE_GARRISON_MULT` (3) times the standard quota, at least `TURTLE_GARRISON_MIN` (3),
  in every city, trained while `ARMY_RESEARCH_FLOOR_PCT` research stays affordable (beyond it the
  rules would disband units for the deficit) and kept in place (the placement quota too).
- Defensive buildings (city defense) first after the garrison, done within
  `TURTLE_DEFENSE_MAX_TURNS`.
- No field army beyond the standard's small force against raiders; no campaign force.
- Settles normally (the standard code settles only sites its escorts cover); the rest is the
  standard economy.

**Expand: land above all.**
- Sites down to `EXPAND_SITE_VALUE_PCT` (75%) of the standard's lowest site value; as many settlers
  at a time as it has cities (bounded by the sites); from cities of size `EXPAND_MIN_POP` (2);
  settlers come from every governed city (the standard asks only its four empire cities).
- One defender per city and no second defender while its landmass has free land
  (`naval::home_is_full`); workers as standard.
- No wars declared, every peace taken. With no site left it plays standard.

## Hooks

Each is a call into `probes.rs`; nothing else in the AI knows about probes.

| Where | What | `probes.rs` |
|---|---|---|
| `lib.rs` `telemetry_intents` | intent `probe` | `probe`, `name` |
| `production.rs` `decide` | garrison quota, its caps when broke | `garrison_need`, `keeps_garrison` |
| | second defender | `skips_second_defender` |
| | settlers | `settlers_allowed` |
| | army and buildings, after the garrison quota | `production` |
| | army target and campaign force | `small_army` |
| `governor.rs` `decide` (governed cities: most of the AI's) | garrison, second defender, army and buildings | `garrison_need`, `keeps_garrison`, `skips_second_defender`, `production` |
| `governor.rs` `empire_cities` | cities that take the ladder's choices (settlers) as empire orders | `empire_cities` |
| `units.rs` `garrison_quota` | placement quota | `garrison_need` |
| `expansion.rs` `settler_choice` | settlers at a time, city size | `settler_limits` |
| `sites.rs` `compute` | lowest site value | `site_min_value` |
| `research.rs` `choose` | the Rush's research | `research` |
| `diplomacy.rs` `manage` | peace sought or refused; the probe's war preparation and declaration | `peace_stance`, `manage_war` |
| `diplomacy.rs` `judge_offer` | peace offered; war bribes | `accept_peace`, `declares_wars` |
| `finance.rs` `disband_military` | no disbanding for upkeep (Rush, Horde, Turtle) | `keeps_army` |

## Running a probe against the standard AI

Slot 0 plays the probe, the other slots the standard AI:

```
W=$(tools/agents/build-tree.sh --bins ffh)
python3 crates/ffh-cli/scripts/civ_ab.py --civ all --seeds 1-8 --turns 300 --no-supremacy \
    --base-bin $W/ffh --new-bin $W/ffh --new-env FFH_AI_PROBE_RUSH_ONLY={civ} --out-dir rush_ab
```

The base side is the standard AI in the same slot on the same seeds; the report is slot 0's win
rate, share and survival. For a few civilizations, `ai_metrics.py --civ CIVILIZATION_X` with
`FFH_AI_PROBE_RUSH_ONLY=X` in the environment plays the probe in slot 0 (the other slots avoid that
civilization).

## Checking that a probe executed

A probe that does not execute its strategy measures nothing. Play with `FFH_TELEMETRY=1` and read
slot 0 with `crates/ffh-cli/scripts/probe_check.py`, beside the standard AI on the same seeds:

```
for side in std rush; do
  env FFH_TELEMETRY=1 $( [ $side = rush ] && echo FFH_AI_PROBE_RUSH_ONLY=HIPPUS ) \
    python3 crates/ffh-cli/scripts/ai_metrics.py --bin $W/ffh --seeds 1-6 --turns 300 \
    --at 50,100,150,200,250,300 --civ CIVILIZATION_HIPPUS --detail --out $side.json
done
python3 crates/ffh-cli/scripts/probe_check.py std.json rush.json
```

What to look for: Rush, an earlier first war, attackers and military hammers far above the
standard's, cities taken; Turtle, no war its AI declared (`wars` 0; `forced` counts the rules'
declarations: defensive pacts, council resolutions), defenders per city, cities lost; Horde,
military share of the hammers, wars declared, turns at war; Expand, cities at turns 100 and 200.
To find a probe stuck (an army that never declares, no target found, bankrupt), trace one game:
`FFH_AI_TRACE=0` prints the `probe rush:` / `probe horde:` lines (target, research goal,
declaration with the force that covers the city) among the standard decisions.

## Measured (2026-11-10)

Slot 0 forced to the Hippus (seeds 1-6) and the Ljosalfar (seeds 1-6): 12 games per side, 6 civs,
standard map, 300 turns, the standard AI everywhere else; the standard side is the same civilization
in the same slot on the same seeds. Means over the 12 games (a dead player counts 0 cities and 0
share).

| | standard | Rush | Horde | Turtle | Expand |
|---|---|---|---|---|---|
| cities at 100 / 200 / 300 | 3.4 / 7.0 / 8.6 | 2.6 / 5.3 / 3.8 | 3.2 / 6.2 / 9.6 | 3.9 / 6.9 / 7.8 | 4.3 / 7.1 / 8.3 |
| first war its AI declared (games with one) | 141 (10) | 97 (12) | 91 (11) | none | none |
| wars its AI declared | 2.0 | 2.8 | 5.4 | 0 | 0 |
| turns at war with a civilization | 89 | 174 | 158 | 10 | 29 |
| attackers among military units trained | 69% | 92% | 85% | 60% | 76% |
| military share of hammers | 44% | 85% | 64% | 52% | 36% |
| city defenders per city at 300 | 3.2 | 0.6 | 4.0 | 8.3 | 1.8 |
| civ cities taken / lost | 2.8 / 2.9 | 7.7 / 6.8 | 6.4 / 2.8 | 0.6 / 1.2 | 1.4 / 2.8 |
| players eliminated by it | 0.2 | 0.8 | 0.8 | 0.1 | 0 |
| techs at 300 | 39 | 10 | 26 | 33 | 40 |
| share at 300 (basis points) | 1829 | 501 | 1657 | 1529 | 1899 |
| alive at 300 / first in share | 11 / 4 | 9 / 0 | 12 / 1 | 12 / 2 | 11 / 4 |

(Defenders per city count units whose default AI is city defense; Warriors, the first defenders,
count as attackers, so early garrisons are undercounted on every side.) N = 12: the share
differences of Horde, Turtle and Expand against the standard are within noise; the Rush's is not.

What the games show:
- **Rush** executes: it declares 44 turns earlier, puts 85% of its hammers into an army of
  attackers, takes nearly three times the standard's cities and eliminates at least one player in 7
  of 12 games (the standard in 3). It loses almost as many (one defender a city
  through constant war) and its research collapses (10 techs at 300 against 39), so it ends with
  less than half the standard's cities and a quarter of its share; 3 of 12 were eliminated. Read
  as a balance finding for an all-in rush from two or three cities in a 300-turn game: the land it
  takes does not pay for the economy it gives up. It still plays weakly in places: its first
  declarations (warriors against fortified garrisons) take little, and its cities fall to
  counterattacks while its army is away.
- **Horde** executes: at war 158 turns of 300 with 5.4 wars declared, two thirds of its hammers in
  the army, more cities at 300 than the standard (9.6 against 8.6) for fewer techs (26 against 39).
  One game (Hippus, seed 3) never reached 3 cities and played as the standard.
- **Turtle** executes: no war its AI declared in 12 games (two declared for it by the rules), 8.3
  city defenders a city, never eliminated, 1.2 cities lost against 2.9; the garrisons cost it
  research (33 techs against 39), so it does not out-tech the standard AI.
- **Expand** executes in the opening: 4.3 cities at turn 100 against 3.4, the land fills by turn
  150-200 and it plays standard from then on (8.3 cities at 300). Settlers trained just before the
  last sites were taken by others stand idle (the standard settler logic holds a settler with no
  site).
