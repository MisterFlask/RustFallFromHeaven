# Telemetry: what the players did, with what, and why

`ffh_core::telemetry` records what each player does (missions, casts, hurries, upgrades,
promotions, deals, wars, cities, combats, builds, techs, civics) and why the AI did it (plans
that hold until they change, decisions with the rule that made them and the alternatives,
cast and war reasons). It covers both AIs (ffh-ai and the C++ AI port, docs/CPPAI.md) and human
players the same way, because the rules record the actions and each AI only adds its reasons.

It only observes. Recording draws no random number and changes no game state, and nothing
reads it back during play, so a game plays the same with it on or off. The test
`crates/ffh-ai/tests/telemetry.rs` checks that every turn's checksum and the final save are
identical. Telemetry is not saved; a loaded game has it off.

## Turning it on

```
FFH_TELEMETRY=1 ffh stats --seed 1 ...              # aggregates: "telemetry" in the stats JSON
FFH_TELEMETRY_FILE=game.jsonl ffh stats ...         # also every event, one JSON object per line
python3 crates/ffh-cli/scripts/telemetry.py game.json [more.json ..] [--by side|civ|player]
    [--sections actions,combat,deaths,spells,wars,decisions,intents] [--top 12]
python3 crates/ffh-cppai/scripts/h2h.py ... --telemetry   # head-to-head with the summary per AI
```

`FFH_TELEMETRY` is an `FFH_*` variable, so the run cache keys on it. A run with
`FFH_TELEMETRY_FILE` bypasses the cache (`ffh_agents.py`), because a replayed stdout would not
write the file. The summarizer accepts `ffh stats` output or lists of games (`h2h.py --out`). It
groups players by the AI that played them (the default when both played), by civilization or by
slot, and reports per civilization and game. In a program, set
`g.telemetry = Telemetry::new(Some(writer))` (or `None` for aggregates only) before `game_init`,
and read `g.telemetry_json()`.

Overhead: within run-to-run noise at 300 turns (4 games); the spell scans are the main cost.
A 300-turn mixed game writes about 0.5 MB of events.

## What is recorded

**Counts** (`players.<p>.counts.<category>.<key> = [count, sum]`):

| Category | Key | Sum |
|---|---|---|
| `mission` | mission type pushed (`group_push_mission`) | – |
| `mission_ai` | mission AI tag | – |
| `hurry` / `hurry_pop` | hurry type | gold / population spent |
| `hurried` | what was hurried | – |
| `upgrade_to` / `upgrade_from` | unit types | gold spent |
| `promote` | promotion taken | – |
| `build` | worker build finished | – |
| `built_unit` / `built_building` / `built_project` | type | – |
| `deal` / `deal_gave` / `deal_got` | `n` / trade items | gold for gold items |
| `war_declared` / `war_plan` | war plan type | – |
| `city` | `founded`, `razed` | razed population |
| `city_taken` / `city_lost` | `conquest`, `trade`, `other`, `from_barbarians` | population |
| `tech` | `n` | – |
| `revolution` | `n` | anarchy turns |
| `civic` | civic adopted | – |
| `convert` | religion | – |
| `attack` | `win`, `loss`, `withdraw`, `undefended` | – |
| `attack_odds` / `attack_odds_exp` | odds decile `00`..`90` | wins / sum of odds (per mille) |
| `attack_vs` | `civ`, `barbarian`, `animal` | wins |
| `defend` | `win`, `loss`, `withdraw` | – |
| `defend_in_city` | `yes`, `no` | defenders lost |
| `kill_by` / `lost_unit` | unit type of the winner / of a combat unit lost (combat, spell, killed) | – |
| `died` | cause | – |
| `casters` | `unit_turns`, `idle_castable` | – |
| `intent_changes` | intent kind | – |
| `vassal` / `vassal_master` | `became`, `capitulated`, `freed` / `gained`, `lost` | – |

The `died` causes are:

- `combat`;
- `spell`: killed during a cast, sacrifices included;
- `mission`: used up by a mission, e.g. settlers, great people, missionaries;
- `upgraded`: replaced by its upgrade;
- `expired`: summons and spell units;
- `scrap`;
- `disbanded`: the unit could not be paid for;
- `killed`: killed by a player outside these causes;
- `reborn`: an immortal reborn;
- `other`.

Combat odds are the rules' `getCombatOdds` as the battle begins, after defensive strikes. That is
the true chance, not the AI's own estimate. Ranged, air and collateral damage are not counted as
combats.

**Spells** (`players.<p>.spells.<SPELL>`): availability, use and effect, per player and spell.

| Field | Meaning |
|---|---|
| `known` | Unit-turns a unit met the spell's unit-side prerequisites at the start of its owner's turn. These are promotions, unit type or class, combat type, civilization, religion and level, plus AI permission. The lair, equipment and hire spells have no unit-side prerequisites and are never "known"; only their casts count. A spell whose only prerequisite is the civilization (most world spells) counts once a turn, not once per unit. |
| `castable` | Of the `known` unit-turns, those where `canCast` held at the start of the turn. |
| `idle` | Unit-turns that ended with the spell castable by a unit that cast nothing that turn: opportunities passed up. |
| `turns_known`, `turns_castable`, `first_known`, `first_castable` | Per player-turn; the first turn each held. |
| `casts`, `first_cast`, `last_cast`, `by_century` | When it was cast. |
| `reasons` | Casts by the reason the controller gave (`untagged` when it gave none, `nested` for a cast inside another spell, `delayed` for a delayed spell going off, `command` for a human's order). |
| `value` | Sum of the values the AI gave. |
| `at_war`, `in_city`, `enemies` | Context: the caster's team was at war; the caster stood in a city; units of teams at war with it within reach (summed). |
| `kills`, `damage`, `cursed`, `converted`, `others_buffed` | Effect on others' units within reach: max(spell range, 2) plots, or the whole map for world spells. `cursed`: enemy units that gained promotions. That is a curse's effect, or a blessing that reached enemies too; the rules' blessings reach every unit in range. `converted`: units that changed to the caster's side. `others_buffed`: non-enemy units that gained promotions. |
| `summons`, `summon_kills` | Units the caster's owner gained during the cast, and the combats those summons won later. |
| `buffs`, `healed`, `own_lost` | Effect on our own units: promotions gained, hit points healed, units lost. |
| `world` | A world spell. |

A caster (`casters.unit_turns`) is a unit that knows a spell granted by a promotion.
`casters.idle_castable` counts the casters that ended a turn without casting while one of their
own spells (not a civilization spell) was still castable.

**Wars** (`wars`) are one record per war between two teams, from the declaration to the peace:
- `start`, `end`, `turns`;
- the declaring team (`a`, `a_players`) and the target (`b`, `b_players`);
- `barb` for wars with the barbarians (which start at turn 0);
- `plan`: the declarer's war plan;
- `reason`: the AI's reasons, joined with `; `;
- `cities`, `pop` and `kills` for each side: cities taken, their population, and combats won against the other side.

**Timelines** (`players.<p>.timelines.<kind> = [[turn, value], ..]`) record a plan each time it
changes. **Decisions** (`decisions.<kind>.<choice> = [count, value sum]`) record each choice made,
and **whys** (`whys.<kind>.<rule> = count`) record which rule made it.

**Events**, in the file. Every event carries `t`, `e` and `p`. The kinds are:
- `cast` and `cast_result`;
- `combat`;
- `war`, `peace` and `war_plan`;
- `deal`;
- `found`, `acquire` and `raze`;
- `hurry` and `upgrade`;
- `tech`, `civics`, `religion` and `project`;
- `vassal`;
- `intent` and `decision`.

Missions, promotions, improvements and builds are only counted.

## What the AIs report

| | ffh-ai | C++ AI port |
|---|---|---|
| Intents | `goal`: the victory planner's goal. `finance`: `ok`, `losing_gold`, `support_heavy` or `structural_deficit`. `war_prep`: the team being prepared against. `campaign`: the city its operation targets. | `strategy`: the `AI_STRATEGY_*` flags this turn's decisions used. |
| `production` | The step of `production::decide` that chose it as why (`garrison`, `settler`, `unit_enabler`, `building_interleave`, `caster`, ... or `fallback`). | Why is the unit AI role, `building`, `project` or `process`. |
| `research` | Why is `value`, `victory_goal` or `escape_costs` (finance's cost-cutting tech); the top four scores are recorded as alternatives. | The choice only (`AI_doResearch` and the rules' requests). |
| `settle_target` | The site: `here`, `walk` or `first_city`, with the top four sites and their scores. | – |
| `settler` | A settler that `hold`s, `wait`s for an escort or `fall_back`s, with why (`no_site`, `no_safe_site`). | – |
| `vassal_offer` / `vassal_asked` / `vassal_taken` | Offers to become a vassal (`vassal`, `capitulate` or `refused`; why `hopeless` or `crushed`), answers when asked (`agreed`, `refused`), and answers as the master (`accepted`, `refused`; why `take`, `their_wars`, `conquer_instead`). The value is the other team. | – |
| `war_prep` / `peace` | `territory` (with every target that passed the gates and its score) or `victory_plan`; peace as `want_peace` or `cash_out`. | – |
| Cast reasons | The context given to `magic::cast`: `attack` (before an attack), `turn` (the value-based end-of-turn choice), `turn_ffh` (FfH's own fallback choice), `buff`, `summon`, `civ`. The civilization modules add `pirate_cove`, `loki_disrupt`, `rantine_convert` and `world_spell`. | The routine that cast: `end_turn_spells`, `mage_cast`, `summon_cast`, `buff_cast`, `direct_damage_cast`, `movement_cast`, `ai_mage_turn`, `feastingmove`, `lokimove`, `rantinemove`, `govannonmove`, `build_pirate_cove`, `do_turn_units_post`. |
| War reasons | `territory (power ratio R, needed N)` or `victory plan (why)`, then at the declaration `army ready after N turns: army A of B` (or `waited long`). | `AI_doWar maximum war`, `limited war` or `dogpile`, each with its start value. |

## Adding telemetry

- **Rules side:** add a `tel_*` method in `telemetry.rs` and call it behind
  `if self.telemetry.on()`. It may only read state. Never call anything with a side effect: not
  the RNG, not a cache that a later rules or AI call would see (path finders, AI caches), not an
  AI method. The C++ AI port's strategy flags are read from its cache for this reason, not
  recomputed.
- **AI side:** `g.tel_intent(p, kind, || value)`, `g.tel_decision(p, kind, choice, why, value, || alts)`,
  `g.tel_cast_reason(u, reason, value)` just before a cast, and `g.tel_war_reason(t, target, || reason)`.
  Build values only inside the closures or behind `g.telemetry.on()`, so play costs nothing
  when telemetry is off. These calls only record; nothing that changes play may depend on
  telemetry being on.

## Limits

- Spell availability is sampled twice a turn: at the start of the owner's turn and at its end.
  A spell that becomes castable only after the unit moves, and is then cast, shows more casts
  than turn-start `castable`. `idle` is the measure of opportunities passed up.
- Spell effects are measured within max(range, 2) plots of the caster. A Python spell that
  reaches farther is undercounted. Buffed units are not followed into their later fights.
- Kills in war records are combats won; units killed by spells are not attributed to a war.
