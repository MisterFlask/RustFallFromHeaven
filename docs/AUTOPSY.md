# Autopsy: what goes wrong in AI games, counted

The AI work list starts from what goes wrong in played games, counted and attributed to a cause
(CLAUDE.md, "How AI changes are checked", step 1). The autopsy is the tool for that: the rules
record facts at the moments a game goes wrong (telemetry, docs/TELEMETRY.md), and
`crates/ffh-cli/scripts/autopsy.py` classifies them and counts them over many games.

The rules side records facts only, never a judgement. The classes are computed by the script
from those facts, with their cut-offs as named constants at the top of the script, printed in
the report's header. Nothing names a unit, building, technology or civilization, and no turn is
fixed: strengths are the rules' own, the game's thirds are thirds of its length.

## Running it

```
python3 crates/ffh-cli/scripts/autopsy.py --bin $W --seeds 1-24 --turns 300 --out DIR \
    [--civs 6] [--size WORLDSIZE_STANDARD] [--env FFH_X=1 ...] [--jobs N] [--by civ] [--json FILE]
python3 crates/ffh-cli/scripts/autopsy.py --load DIR [--by civ] [--json FILE]
```

`--bin` is a build output directory (`W=$(tools/agents/build-tree.sh --bins ffh)`) or an `ffh`
binary. Each seed plays `ffh stats` with `FFH_TELEMETRY_FILE`, writing `DIR/seed_N.json` (the stats,
with the telemetry aggregates) and `DIR/seed_N.jsonl` (the events); a seed whose two files exist is
not played again, so a directory grows by adding seeds. Each game holds one machine-wide CPU slot
(docs/AGENTS.md). These runs bypass the run cache, which cannot replay the events file. `--load`
reports from a directory without playing. `--by civ` adds every section per civilization (the
old owner for cities fallen); `--json` writes everything, each fallen city with its facts and class.

## What the rules record

In `crates/ffh-core/src/telemetry_autopsy.rs`, only when telemetry is on.

**`battle`** (event), as each engagement is launched (before summons, defensive strikes and the
battle): the attacker `p`, the defender `vs` (the city's owner, else the first defender's owner),
`x`, `y`, `city`, the plot's owner `owner` and the launch plot's `from_owner` (-1: unowned),
`att_n` / `att_str` (the committed force), `def_n` / `def_str` / `def_full` (the military of teams
at war with the attacker on the plot; `def_full` at full hit points). The `engagement` event that
follows (docs/TELEMETRY.md) has the result and the losses; the script pairs them by turn, player
and plot.

**`city_fell`** (event), a city taken by conquest (`player_acquire_city`), its player the old owner:
- `to` (the captor), `barb` (the captor is the barbarians), `host` / `owner_host` (the captor / the
  old owner is a crusade Host slot, docs/CRUSADE.md), `razed` (razed on capture, else kept);
- the city: `city` (id), `x`, `y`, `pop`, `age` (turns since founded), `held` (turns since its
  owner acquired it), `capital`, `defense` (its total defense percentage), `defense_damage`;
- the defenders: `siege_turn` (the turn of the latest day of engagements against the plot, null
  when none), `def_n`, `def_str`, `def_full` (the military on the plot when that day's first
  engagement was launched), `battles`, `att_n`, `att_str` (that day's engagements and the attackers
  committed in them, summed), `left_in_city` (the old owner's military still on the plot at the
  capture). The capture itself happens after its defenders died, so the defenders are those of the
  day's first engagement; a city taken with no engagement that turn had none at that moment. A
  city whose last defenders died in a duel (ships, air units) rather than an engagement shows the
  last engagement's day or none;
- `captor_near_n`, `captor_near_str`: the captor team's military within 2 plots (the city's plot
  included);
- the old owner's military, each `[count, strength]`, by where it stood, the first that applies:
  `in_city`, `near` (within 5 plots), `other_cities`, `home_field` (its own territory outside
  cities), `abroad`; and `captor_mil` (the captor's whole military);
- `power` / `captor_power` (the rules' player power), `cities` / `captor_cities` (before the
  capture), `wars` (civilizations the old owner's team is at war with), `war_turns` (turns since
  the war between the two teams began, null when telemetry saw no declaration),
  `declared_by_owner`;
- the old owner's economy: `gold`, `commerce_pct` (the commerce sliders), `research` and
  `gold_rate` (its commerce rates), and `producing` (what the city was building).

**`civ_died`** (event), a civilization eliminated (`player_set_alive(p, false)`; not the
barbarians or crusade Hosts): `killer` (the last captor of one of its cities, null when it lost
none by conquest), `lost` and `lost_to` (cities lost by conquest, by captor), `chain` (`[turn,
captor]` for each), `peak_cities` and `peak_turn` (sampled at the end of its turns and at each
loss), `units` (left when it died).

**`army`** (counts, per player): a census of the land military at the end of each of the
player's turns. The key is the war state, `war_` (its team at war with a civilization) or `peace_`,
then the place: `city`, `home` (its territory outside cities), `enemy` (territory of a team it is
at war with), `other`; `_still` counts those that spent no movement that turn. The sum is
unit-turns; the count is the player-turns in that war state.

Military: a land unit on the map that can fight (the rules' `canFight`: some strength).
Strength: the rules' contribution value of a unit (`crusade_unit_strength`, docs/CRUSADE.md): the
better of its attack and defense strengths x100, with its general combat bonus, scaled by its
hit points. The report shows it /100.

Combat odds: duels (ships, air) have the rules' odds in the existing `attack_odds` counts.
Engagements have no single odds; the report buckets attacks by the attacker's share of the
summed strength at launch instead, which is not a probability.

## The classes of a fallen city

The first that applies:

1. `empty`: no military defender when the day's first engagement was launched, or no engagement
   that day.
2. `outclassed`: the old owner's power below `OUTCLASSED_POWER_RATIO` (0.60) of the captor's
   (not used for barbarian captors).
3. `army_away`: the old owner's military beyond 5 plots (in its other cities, in the field at
   home, abroad) had more than `ARMY_AWAY_RATIO` (1.0) times the strength of the captor's units
   within 2 plots: the force existed but was not there. Sub-split by where most of it stood.
4. `thin`: defended, not outclassed, the army not away: too little military anywhere. The report
   gives the owner's military strength relative to the captor's.
5. `other`: facts missing (a file from before a field existed).

The report also gives, per class, the five most typical examples (closest to the class's median
of its defining ratio) with seed, turn, city and plot, to fork with `tools/ai-branch.sh`.

## The rest of the report

- Cities fallen per game and in total, to the barbarians and to civilizations, by third of the
  game. Barbarian cities and crusade Host cities taken are counted apart, not classified.
- Eliminations: when, the peak, and the chain of cities lost with each one's captor and class.
- Army census: unit-turns by place and war state; `still`; `idle@war`: the share of army-turns at
  war spent outside any city and outside enemy territory.
- Losses in engagements: committed and lost attacking and defending, by whose territory the battle
  was on, and attacks by strength share with their losses and how often the plot was taken.
