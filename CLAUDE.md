# ffh2-rs: Fall from Heaven 2 (0.41) ported to Rust

**New session?** Read this file, `README.md` and the doc for the area you touch (`docs/PORTING.md`,
`docs/AI.md` + `docs/AI_LOG.md` + `docs/TELEMETRY.md`, `docs/PLAY_DESIGN.md` + `docs/HUMAN.md`, `docs/ART.md`,
`docs/R3D.md`, `docs/HD2D.md`, `docs/SCORE.md`, `docs/PERF.md`, `docs/CRUSADE.md`, `docs/CARDS.md`,
`docs/WORKS.md`, `docs/TACTICS.md`, `docs/SUPPLY.md`, `docs/NAVAL.md`). What is merged is what git says (`git log origin/master`);
open work and known gaps live in the area docs, not in a session log. Out of scope for now (the owner's
call): scenarios. The C++ AI is ported only as a benchmark to measure ffh-ai against (`crates/ffh-cppai`,
docs/CPPAI.md; kept on master by the owner's request, 2026-10-21), not as the game's AI.
Performance and AI-improvement work ignores it (the owner's call, 2026-10-21: no mileage in it):
don't optimize, tune or extend it, and leave it out of sprint measurements unless the owner asks
for a comparison; when an ffh-core change breaks its build, make the minimal mechanical fix.

Port of the FFH2 0.41 Civ4 mod (base mod only), the base for a game of its own. The rules
engine (DLL C++ + rules Python) was ported function by function; the AI is new. FFH2 0.41 is
the starting point, not an invariant: once the base is stable and attractive, the game will
change sweepingly (see "FFH2 is the starting point" under Hard rules).

## Layout

```
data/json/       the game data: JSON with comments, one file per info table (docs/DATA.md;
                 converted from FFH2 0.41's XML + the stock BTS files FFH2 inherits)
data/text/en.tsv English text (TXT_KEY -> string)
crates/ffh-data  data/json -> typed info tables (port of CvInfos.cpp)
crates/ffh-core  rules engine (port of CvGame/CvPlayer/CvTeam/CvCity/CvUnit/CvPlot/... + rules Python)
crates/ffh-ai    AI players (new design)
crates/ffh-cppai the FFH2 C++ AI ported as a second controller, for head-to-head comparison (docs/CPPAI.md)
crates/ffh-cli   `ffh` binary: autoplay, benchmarks, inspection
crates/ffh-harness what the game runners share: child processes, worker pool, panic capture, invariant checks
crates/ffh-stress `ffh-stress` binary: AI games over a settings matrix, crashes caught per game
crates/ffh-play  `ffh-play`: human play (docs/PLAY_DESIGN.md, docs/HUMAN.md)
crates/ffh-web   the engine as WebAssembly and the page side of the static site (docs/WEB.md)
tools/           fetch-reference.sh, gen_enums.py, gen_defines.py, jsonc.py (the data for Python),
                 agents/ (shared builds, run cache, CPU slots: docs/AGENTS.md)
docs/            ARCHITECTURE.md, DATA.md, PORTING.md (status), AGENTS.md (parallel agents)
anim-sprint/     animations for M7 unit sprites (its own project; anim-sprint/ANIMATING_UNITS.md)
```

## Unit animations

To make idle, strike, flinch or death animations for an M7 unit sprite, follow
`anim-sprint/ANIMATING_UNITS.md`. It describes the method that won the October 2026 animation
sprint (`lib/recipe.py` in that folder); do not start from an ad-hoc approach. Its first rule:
the strike always goes in the direction the sprite is looking. For the battle screen's held key
poses and impact cards (one hit frame and one still image of the blow, held: the catapult's
rubble), follow its section "Impact cards and key poses". Agent build worktrees are sparse
and leave that folder out; run `git sparse-checkout add anim-sprint` first.

## Parallel agents

Other agents share this machine (4 cores) and repository. Follow docs/AGENTS.md:
- Build with `B=$(tools/agents/build-rev.sh <rev>)` / `W=$(tools/agents/build-tree.sh)` (your
  worktree's current files; `--bins ffh` for AI work), not `cargo build` in your worktree: builds
  are shared, incremental and serialized. `tools/agents/build-tree.sh --test` is the
  `cargo build --release && cargo test --release` check below, run on your worktree's files.
- A/B against a base: `tools/agents/ab.sh <ai_metrics.py args>` (the base side's games come
  from the shared run cache). `ai_metrics.py` takes machine-wide CPU slots per game; wrap other
  game runs in `tools/agents/slot.sh` (`--exclusive` for `ffh bench`).
- If you did build in your worktree, `rm -rf target` before finishing.

## Fable designs, Opus implements

If you are a Fable model (the owner's rule, 2026-11-09), dispatch implementation to Opus
aggressively: start subagents with the Agent tool's `model: "opus"` and hand them the writing.
Your part is what needs the judgment: the design and its decisions, the rules' wording, the
briefs, reading what comes back, the measurements' conclusions, the final gate and the merge.
Theirs is the volume: writing the code of a settled design, mechanical edits across many files,
fixing the tests a change broke, updating docs, data conversions, the client's JavaScript.

- Dispatch by default. Keep a piece yourself only when it is a few lines, when it is the design
  itself, or when briefing it would take longer than doing it.
- Split the work into pieces that touch different files and run them in parallel; give each
  agent the decision already made, the files to touch, what to leave alone, and how to check
  its work (`tools/agents/build-tree.sh`: builds are serialized machine-wide, so say who builds).
- Read every result before building on it: an agent's report is not a verification.

A model other than Fable ignores this section.

## Reference source (read-only, NOT in this repo)

`tools/fetch-reference.sh` clones the pinned sources to `/home/user/ref`:

- `/home/user/ref/ffh2/CvGameCoreDLL/*.cpp|h` -- FFH2 0.41 DLL source (the reference for porting)
- `/home/user/ref/ffh2/Assets/python/**` -- FFH2 rules Python (spells, events, ...)
- `/home/user/ref/ffh2/Assets/XML/**` -- original XML (schemas: `*Schema.xml`)

When porting, read the C++/Python being ported -- never port from memory.

## Hard rules

- **No external crates.** crates.io is unreachable from this environment; std only.
- **Determinism.** Simulation state changes only through the game RNG
  (port of `CvRandom`). No `HashMap` iteration in simulation code (use `Vec`/
  `BTreeMap` or sort first). No wall-clock, no thread-dependent ordering.
- **Integer math in rules,** as in the C++ (it uses `int` with percentages).
  Port floating-point only where the C++ itself uses float, with the same ops.
- **FFH2 is the starting point, not an invariant.** The port reproduces FFH2 0.41 so the game
  starts from a known, working base; once that base is stable and attractive, the rules will
  change sweepingly. Equivalence to FFH2 is temporary: don't add tests, tools or machinery whose
  only purpose is keeping the port identical (or bit-exact) to FFH2, and a change is not wrong
  because it departs from FFH2.
- **Rules changes are deliberate and recorded.** Change rules on purpose, not as a side effect
  of other work: say what changed and why in the commit, and list gameplay changes from FFH2 0.41
  in docs/PORTING.md ("Changes from FFH2").
- **New game features are not behind game options** (the owner's decision, 2026-11-05): a new
  system is part of the rules in every game, not a feature flag. Governors, armies, cultural
  actions, engagement combat and crusades were options once; they are in every game now.
- **No switch without a reason to exist** (the owner's decision, 2026-11-10). A game option stays
  only if a player can choose it on the new-game screen and it works; an AI switch
  (`crates/ffh-ai/src/switches.rs`) only while its behaviour is being measured, and it is removed
  when the behaviour is kept or dropped (the module comparators `MAGIC`, `CIV` and the trait
  switches, and the probes, are instruments and stay).
- **Mirroring the C++ is no longer a goal** (the owner's decision, 2026-10-28). Echoing the C++
  closely was the method for the initial port, which is complete; it has no value in itself now.
  Restructure ported code, its C++-shaped APIs and its quirks whenever that makes present code or
  future changes more correct or clearer (the `// C++ quirk:` notes and the C++ names in doc
  comments are a map of where the code came from, not a constraint). A change that plays
  differently is still a rules change: deliberate and recorded (above). The rest of this file's
  C++ -> Rust conventions describe how the existing code is laid out.
- `cargo build --release && cargo test --release` must pass before committing.
  `cargo run --release -p ffh-cli` must report `0 warnings` for data loading.

## Naming conventions (C++ -> Rust)

Info tables (`ffh-data`):
- Each `CvFooInfo` is `FooInfo`; table `data.foos: IdxVec<FooType, FooInfo>`.
- Field name = C++ getter name without `get`/`is`, snake_case:
  `getPrereqAndTech()` -> `prereq_and_tech`, `isAnimal()` -> `animal`,
  `getYieldChange(i)` -> `yield_change` (an array).
- C++ `int` holding an enum/type with -1 = none -> `Option<XType>`.
- Dense per-type arrays (`m_piFoo` sized `GC.getNumXInfos()`) -> `IdxVec<XType, T>`.
  2D arrays -> `IdxVec<XType, [i32; YieldType::NUM]>` or nested `IdxVec`/`Vec`.
- Yields -> `[i32; YieldType::NUM]` (3), commerces -> `[i32; CommerceType::NUM]` (4).
- Python callback strings (`PyResult`, `PyRequirement`, ...) -> `String`.
- Hard-coded C++ enum values are consts: `UnitAIType::UNITAI_ATTACK`,
  `DomainType::DOMAIN_LAND`, `YieldType::YIELD_FOOD` (see `enums.rs`, generated).
- Global defines: `data.defines.max_hit_points` (generated `defines_gen.rs`; one
  field per define, lowercase name). Text defines listed in C++ `SetPostGlobalsGlobalDefines` are
  resolved type indices (i32, -1 = none; other text defines read as -1) -- convert with `opt_from_i32`.
- Type names hard-coded in C++/Python: `data.t::<PromotionType>("PROMOTION_HERO")`
  (panics if unknown) or `data.ty::<T>(..)` (Option). Cache in hot paths.

Data files (`data/json/*.jsonc`, docs/DATA.md) are read by `Rd` (`crates/ffh-data/src/registry.rs`),
one method per kind of value, each with the default an absent key takes: `rd.int_or("key", def)`
(`GetChildXmlValByName`), `rd.rf::<XType>("key")` (a type name: `FindInInfoClass`),
`rd.pairs_int::<XType>("key", def)` (an object from type names to values:
`SetVariableListTagPair`; also `pairs_bool`, `pairs_string`, `pairs_ref`), `rd.yields("key")`,
`rd.commerces("key")`, `rd.ref_list_fixed`. A key is the Civ4 tag without its `i`/`b`/`f` prefix,
in snake_case (`iCombatDefense` -> `combat_defense`). Replicate C++ defaults exactly: use
`int_or(key, d)` only where the C++ call passes a default `d` (most pass none, so absent ints are
0). Loading warns about a key nothing reads, so a new key needs its reader.

## Rules engine conventions (`ffh-core`)

`Game` (`game.rs`) owns all state: `d: &'static GameData`, `setup` (CvInitCore),
`map` (CvMap: plots + areas), `players`/`teams` (`IdxVec<PlayerId|TeamId, _>`, sized
to this game, barbarian last), slabs `units`/`cities`/`groups`/`plot_groups`/`deals`
(global, keyed by never-reused ids; per-owner id lists live on `Player`),
`soren_rand`/`map_rand` (`CvRandom`), `log`, `controllers`, plus CvGame's own members.

C++ -> Rust mapping:
- Methods of class `CvFoo` become `impl Game` methods named `<foo>_<method>` taking the
  object id first: `CvUnit::canMoveInto(pPlot, bAttack)` ->
  `fn unit_can_move_into(&self, u: UnitId, plot: PlotIdx, attack: bool) -> bool`.
  Prefixes: `unit_` (UnitId), `city_` (CityId), `player_` (PlayerId), `team_` (TeamId),
  `plot_` (PlotIdx), `group_` (GroupId), `area_` (AreaId), `plot_group_` (PlotGroupId),
  `deal_` (DealId), `map_` (CvMap), none for CvGame. C++ `const` -> `&self`, else `&mut self`.
- Trivial getters/setters (`return m_iFoo;`, `m_iFoo = iNewValue;` with no side effects)
  are NOT methods: use fields (`self.units[u].damage`). Anything with logic, asserts that
  clamp, or side effects IS a method (`change_damage` that kills, `set_owner`, ...).
- Struct fields: `m_iFooBar` -> `foo_bar: i32`; `m_bFoo` -> `foo: bool`; enum members ->
  `Option<XType>` (or `XType` if never NONE); `m_paiFoo` dense by X -> `IdxVec<XType, i32>`;
  per-player/per-team arrays -> `IdxVec<PlayerId|TeamId, T>` sized `num_players()/num_teams()`;
  `IDInfo` -> `Option<UnitId|CityId>`; `CLinkList<IDInfo>` -> `Vec<UnitId>` (ordered);
  coordinates stay `x: i32, y: i32` where C++ stores them.
- Pointers: `CvPlot*` -> `PlotIdx`/`Option<PlotIdx>`, `CvUnit*` -> `UnitId`/`Option<UnitId>`,
  `CvCity*` -> `CityId`/`Option<CityId>`, `CvArea*` -> `AreaId`, `CvSelectionGroup*` -> `GroupId`.
- Globals: `GET_PLAYER(e)` -> `self.players[e]`; `GET_TEAM(t)` -> `self.teams[t]`;
  `GC.getMapINLINE().plotINLINE(x, y)` -> `self.map.plot_xy(x, y)` (Option, wraps);
  `GC.getXInfo(e)` -> `self.d.xs[e]`; `GC.getDefineINT("X")` -> `self.d.defines.x`;
  `getSorenRandNum(n, "s")` -> `self.soren_rand_num(n, "s")`; `MAX_PLAYERS` loops ->
  `self.player_ids()`; `BARBARIAN_PLAYER` -> `self.barbarian_player()`.
- Iterating a player's units/cities/groups while mutating: clone the id list first
  (`for u in self.players[p].units.clone()`), and re-check `self.units.contains(u)`
  if the loop body can destroy units.
- AI calls inside rules code:
  * decisions (`AI_conquerCity`, `AI_chooseResearch`, `AI_chooseEvent`, ...) ->
    `self.controller(p).<method>(self, ..)` (see `controller.rs`; add a method with a
    sensible default if one is missing);
  * AI bookkeeping state read/written by rules (memory counts, war plans, war success,
    contact counters, unit AI type, group flag, mission AI) -> plain fields (e.g.
    `Unit::ai_type`, `Team::ai_war_plan`, `Player::ai_memory_count`); keep C++ semantics;
  * citizen assignment (`AI_assignWorkingPlots`, `AI_addBestCitizen`, `AI_removeWorstCitizen`)
    -> `self.city_assign_working_plots(c)` etc. in `governor.rs` (new code);
  * other AI (`AI_*Value`, `AI_update`, `AI_doTurn`, auto-cast...) -> omit, leaving a
    `// AI: <call>` comment where behaviour depended on it.
- Python callbacks (`CvEventReporter::*`, `gDLL->getPythonIFace()->callFunction(..)`,
  `PyResult`/`PyRequirement` spells, CvGameUtils hooks) -> functions in `crate::ffh::*`
  (ports of the FFH2 Python). Until ported, call a stub there rather than dropping the call.
- UI/graphics/sound/network (`gDLL->getInterfaceIFace()`, entities, flags, popups for
  humans) -> omit. Messages may go to `self.log.msg(turn, Some(p), || text)`.
- `GC.getGameINLINE().getActivePlayer()` -> `self.active_player` (always `None` headless, so
  "active player" UI branches never run; combat is never shown, i.e. always quick);
  `gDLL->GetAutorun()` -> `self.autorun`.
- `CvPlayerAI::AI_getAttitude(ePlayer, bForced = true)` read by rules ->
  `self.player_ai_get_attitude(p, other, forced)` (forced overrides, then the controller).
- `CvPlayerAI::AI_foundValue` (needed by starting plots and barbarian cities) ->
  `self.player_ai_found_value(p, x, y, min_rival_range, starting_loc, &[])` (`found_value.rs`).

Saved games (`src/save/`, see docs/ARCHITECTURE.md): every state struct reachable from `Game`
is saved by destructuring it exhaustively (`persist_struct!(Unit { id, group, .. every field })`
in `save/state.rs`; `Game` in `save/mod.rs`; `PathFinder` in `save/path_finder.rs`), so adding a
field to `Game`, `Unit`, `City`, `Player`, `Team`, `Plot`, `Area`, `Map`, ... breaks the build until
it is added to its save list (in declaration order) -- then bump `save::FORMAT_VERSION`. A new
field type needs a `Persist` impl (`save/io.rs` covers primitives, `Option`, `Vec`, arrays,
`IdxVec`, `BTreeMap`, tuples, `Slab`, `Cell`/`RefCell`; ids and info types in `save/state.rs`).
State must live in `Game` (not in statics or controllers) to survive save/load.

Every `todo!()` left in core code means "not yet ported"; keep the C++ function name in
the message (`todo!("CvUnit::airStrike")`) so remaining work is greppable.

## Python porting conventions (`ffh-core/src/ffh/`)

FFH2 Python talks to the DLL through the Cy* wrappers (CyUnit, CyCity, CyPlayer, ...),
which forward to the C++ methods -- so port each Cy call to the corresponding ported
`Game` method or field:
- `pUnit.getX()` -> `self.units[u].x`; `pUnit.setHasPromotion(e, True)` ->
  `self.unit_set_has_promotion(u, e, true)`; `pPlayer.initUnit(..)` -> `self.player_init_unit(..)`.
- `gc.getInfoTypeForString('PROMOTION_X')` -> `self.d.t::<PromotionType>("PROMOTION_X")`
  (hoist out of loops). `gc.getDefineINT('X')` -> `self.d.defines.x`.
- `CyMap().plot(x, y)` -> `self.map.plot_xy(x, y)`; it returns `None` off-map, and the
  Python NULL-plot then answers defaults (`isNone()` true, counts 0) -- port that explicitly.
- `CyGame().getSorenRandNum(n, "s")` -> `self.soren_rand_num(n, "s")` (same order/count).
- Python 2 integer `/` floors: use `crate::ffh::game_utils::py_div`; `%` on negative values
  follows Python (`rem_euclid` for positive divisors). `range(a, b)` -> `a..b`.
- `PyPlayer(i).getUnitList()` / `getCityList()` -> clone the id lists from `self.players[p]`.
- UI-only calls (`CyInterface().addMessage`, popups, sounds, `CyEngine` effects) -> omit
  (or `self.log.msg`). Human-only popup choices -> controller decision for that player.
- Callback names stored in the data are dispatched through tables built once from `PyCall`
  (`ffh/pycall.rs`); every name referenced by the data must resolve (a test checks this).

## Workflow

- Work in small, compiling steps; commit with clear messages.
- Update docs/PORTING.md when a C++/Python unit is ported (file, functions, status).
- Tests: unit tests next to code; data spot-checks in `crates/ffh-data/tests/`.
- No test relies on a seed's story (the owner's decision, 2026-11-05): how an AI game goes (a
  bot survives, a city is captured, a battle comes, a game ends in a victory) changes with every
  AI or rules change, so no test asserts it or needs it. A test that plays turns uses a game that
  cannot end (`common::STEADY` in `crates/ffh-play/tests`: always peace, no barbarians, lairs or
  goody huts) and asserts only what holds for any seed; a test that needs a situation (cities,
  a war, a battle) builds it by hand. Measure how games go with the tools (docs/AI.md, "Tools").
- **Merging to master** (the owner's standing permission, 2026-10-21): once a change passes the
  gate (`tools/agents/build-tree.sh --test`, `ffh data` with 0 warnings, `bash tools/web-check.sh`)
  and the end-to-end tests (`node tools/web-smoke.mjs --bin <built>/ffh-play`; the static site's
  smoke as well when the change reaches the site, below) all pass, merge it into master
  (fast-forward when possible) and push, without asking. This holds even when the session was
  told to develop on another branch: push that branch too. Master is deployed to the public site
  by CI, so never push it with a failing or skipped check; say what failed instead.
- **End the turn that pushes master** with the line `WORK ITEMS HAVE BEEN PUSHED TO MASTER`,
  followed by a clickable link to the game in the browser (the owner's request, 2026-10-21):
  `[Play in the browser](https://misterflask.github.io/RustFallFromHeaven/)` (CI deploys master
  there a few minutes after the push).

## AI changes

Rewritten 2026-11-10 (the owner's decision), from what the earlier rounds showed (docs/AI_LOG.md):
ideas were built before anyone counted how often their situation arises, single switches were
judged by a win rate that could not see them, rules were written where an estimate was missing,
and the AI was only ever measured against itself.

### What the AI is

- **The AI plays the way an expert human would** (the owner's decision, 2026-10-21). In broad
  strokes its strategy and tactics are the ones a strong player would conceptualize and could
  explain: settle good land while it is free, defend what it holds, fight when it has the edge and
  make peace when it does not, use every tool its civilization has, give up independence when
  hopelessly behind. Flavor and fun override this where a rule below says so.
- **It is an opponent for a human.** What matters is how it stands against a player who does
  what it does not: masses an army, rushes, turtles. A result that holds only among copies of
  itself (a defender is cheaper to rebuild than to keep, when no neighbour has an army) is a fact
  about those opponents, not about the game.
- **Civilizations use their unique mechanics** (the owner's decision, 2026-10-20): a
  civilization's own units, spells and abilities are part of how it plays, so its AI uses them
  whether or not they directly benefit it.
- **Conquest and domination are a flavor drive** (the owner's decision, 2026-10-21): the Clan of
  Embers, the Doviello, the Bannor, any other canonically warlike civilization, and every leader
  with `TRAIT_AGGRESSIVE` play with an expansion drive, not a victory goal pursued once in reach:
  they keep looking for the easiest way to grow -- a settler site or a weaker nearby civ's
  cities -- and take it, again and again, whether or not it raises their win rate (docs/AI.md,
  "Victory pursuit").
- **Hopeless civilizations become vassals** (the owner's decision, 2026-10-21). A civilization
  that has fallen hopelessly behind (in technology, power and its share of the world) offers to
  become the vassal of a much stronger neighbour it is at least decently aligned with (no
  good/evil opposition, an attitude its leader's pride accepts), and agrees when asked. One being
  crushed in a war capitulates to the conqueror, whatever it thinks of it (docs/AI.md, "Vassal
  states").
- **The AI does not pursue the Tower of Mastery or the Altar of the Luonnotar** (the owner's
  decision, 2026-10-21: boring solitaire victories). It still stops a rival about to win one.
- AI timing and AI-only restrictions belong in `ffh-ai`, not in the rules (docs/AI_ONLY_RULES.md
  lists the rules' remaining AI-only branches, each group moved by its own decision).

### How AI changes are checked

- **A rules change** (a new system, a changed rule) is not gated on the AI at all: make the AI do
  something with it that seems reasonably likely to be effective, ad hoc and unprincipled if need
  be, and move on.
- **An AI change is never kept or dropped on the standard AI's win rate or share against copies
  of itself.** That number is zero-sum, noisy at any affordable sample, and falls when every
  civilization conquers better. Do not measure it to decide.

An AI-specific change goes through this loop, in this order; stop at the first step that fails.

1. **Start from games, not from an idea.** The work list is what goes wrong in played games,
   counted: every city lost, civilization eliminated, army that stood idle, caster that never
   cast, attributed to a cause (telemetry, `review_game.py`, traces) and ranked by how often it
   happens times what it costs. "An expert would do X" picks the fix for a counted failure; it is
   not a reason to build X.
2. **Count the firings before building.** A throwaway trace line for the situation (not the
   behaviour) over a few dozen games: how often, on which turn, for whom. A situation that arises
   in a few games, or after the game is decided, is dropped here.
3. **Estimate first, and check the estimate against what happened.** A behaviour that depends on
   a prediction (the city falls, the line can be cut, the stack wins, raiders will come) gets the
   estimator built and measured as a predictor first: predicted against observed over many games.
   Build the decision only on an estimator that is right. Prefer an estimator that runs the
   rules' own code (`combat_sim`) to thresholds and weights: it stays right when the rules change.
4. **Judge the behaviour where it fired.** Did it do what it was meant to there: the city held,
   the stack took its city with fewer losses, the caster lived. Use forks of the game just before
   a firing (`tools/ai-branch.sh`), traces and counts over many games. One fork is an anecdote:
   judge on many firings, and on what the units and cities did, not on the score.
5. **Follow the chain before judging the first link.** The AI's heuristics cover for each other:
   fixing one often exposes the deficiency it was hiding (the reckless attacks were keeping
   raiders off the cities). When a sound fix costs something elsewhere, find what it exposed, fix
   that too, and judge the package. Revert only when the idea itself was wrong or the chain is not
   worth following; say which in docs/AI_LOG.md.
6. **Guardrail, not verdict.** Before keeping a package, one wide run against the base
   (`tools/agents/ab.sh`) to see that nothing broad broke: military at turn 100, cities lost to
   barbarians and to civilizations, eliminations, research. A clear break is a link in the chain
   (step 5); a flat result is a pass.
7. **Play it against something that is not itself.** Where the change concerns war, defense or
   force building, the opponents that matter are the probes (below), not six copies of the same
   build. For a general behaviour, also give the change to one player against opponents without
   it (`civ_ab.py --civ all --new-env FFH_AI_<NAME>_ONLY={civ}`) and read what happened to that
   player's cities and armies, not its win rate.

The instruments for these steps, the current baseline and the work list it gives are in
docs/AI.md, "The improvement scheme": the autopsy (step 1, docs/AUTOPSY.md), the prediction
ledger (step 3, docs/CALIBRATION.md), fork rollouts (step 4, `tools/ai-branch.sh --rolls`), the
probes (step 7, docs/PROBES.md).

`docs/AI_LOG.md` records attempts worth remembering and what their games showed: read the entries
for an area before working in it.

### Probes: AIs that go all in on one strategy

A probe is a variant of the AI that pushes one strategic direction as hard as it can and neglects
the rest (the owner's decision, 2026-11-10): an early rush, one unit line massed, settling without
pause, a turtle that only researches, casters and summons above all, pillaging and cutting supply,
culture, a religion spread everywhere. Built: Rush, Horde, Turtle, Expand (docs/PROBES.md);
build another when a question needs it.

- **What they are for.** (a) Balance: a strategy that wins far more or far less than the
  standard AI when pushed to the limit is a candidate overpowered or underpowered strategy, in the
  rules or the data. (b) Opponents: the standard AI is measured against them (step 7), which is
  how it meets an army it would never build. (c) Flavor: a probe that plays well and reads as a
  civilization's or a leader's character may become how that one plays (the owner's call).
- **Reading a result.** A probe that crushes the standard AI shows either a strategy too strong
  or an AI that does not answer it. Tell them apart: if an expert has an answer within the rules
  (and the AI can be given it), it is an AI gap and goes on the work list; if not, it is a balance
  finding for the owner, reported with the numbers, not fixed in the AI. A probe that loses badly
  shows a weak strategy only if it was played competently: look at its games first.
- **How they are built.** In `ffh-ai`, legal play and deterministic like the rest, chosen per
  player from the environment as the switches are (`switches.rs`), so the standard tools run
  them. A probe is exempt from "plays the way an expert human would" and from the flavor rules
  above; it must still execute its one strategy competently, or its result says nothing. Keep each
  small: overrides of the standard AI's priorities (what to build, research, and when to declare
  war), not a second AI. Probes are measuring tools: they do not play in the released game unless
  (c) above makes one a civilization's play, and no test asserts how their games go.
- A probe's win rate against the standard AI is a legitimate measurement, unlike the standard
  AI's against itself: the two sides differ in strategy, so the number says something about it.

## Rules text and tooltips

Mechanical text -- rules, tooltips, previews, results, reasons, effects -- is bare and without
flavor (the owner's rule, 2026-11-02): labels, numbers and the rule itself, no adjectives,
narration or mood. Write `Total offered: 240`, `This proposal will be refused.`,
`Reason: open borders requires attitude Pleased.`, `Effect: -1 relations for 30 turns.`; not
"they would welcome it" or "the forest thanks you". Flavor belongs to what characters say (the
leaders' speech) and to the names and descriptions of the game's data.

## Show UI changes in a screenshot

Whenever a change adds to or alters what the player sees in support of a feature (a panel, a
button's states, a dialog, an alert, a tooltip's text), show the owner the result in a screenshot
when it is done (the owner's rule, 2026-11-11): a PNG readable on a phone (a crop of what changed,
at `deviceScaleFactor: 2`, not a full desktop frame), sent with the turn's report. For a feature
with rules, the screenshots cover how its limits are told to the player, where its state is seen
at a glance, and the exact text of each new effect.

## Verifying renderer and visual work (r3d, lab, client look)

This is a game. The owner looks at the result and reports regressions, and a regression costs a
follow-up fix, not harm. Verify in proportion: fast iteration beats exhaustive pre-checking. These
rules hold even when ultracode is on: use workflows to fan out *work* (independent areas, design
alternatives, research), not to multiply verification. Rules-engine work keeps the hard rules above.

Tiers (pick by the change):
- **Small** (a constant, a palette, a doc, a one-function fix): self-check only. One cropped render
  of what changed, or none if it isn't visual. No review agent.
- **Medium** (a new effect, a reworked area): self-check, then at most one reviewer agent working
  from the code and the renders already made (at most 2 small new frames). No refute round: judge
  its findings yourself (would the author's one-line reply win? then drop it).
- **Large** (a new look across the map, a pipeline change): one review round with 2-3 critics in
  parallel; a refute pass only for high-severity findings that would change direction. At most one
  round per batch.

Screenshots for the owner are taken on the HD-2D map (the owner's rule, 2026-11-11: `?force3d=1`
where this machine falls back without a GPU). The flat and isometric maps are for debugging and as a
backup for older PCs: judge and show a look on HD-2D, and design for it first.

Rendering (SwiftShader here: 10-30 s a frame on 4 shared cores; workflows run 2 agents at a time):
- Iterate on small renders (`--w 512 --h 320`) or crops of the changed region. Full 1024x640 sets
  and a real-game capture (`tools/r3d-compare.mjs`) only for a batch's final check.
- Review agents read existing renders and render only if they truly must.
- Never wait idle on a render, perf run or review: run it in the background and work on the next
  independent item meanwhile.

Batching:
- Group related items: one verification round and one final render set per batch, not per item.
- `tools/r3d-perf.mjs` and `tools/r3d-lab.mjs --stability` only at milestones (a batch that changes
  shader cost or anything world-anchored), as one A/B against the previous milestone: same save,
  same binary, the old tree via `git archive`. Update docs/R3D.md section 11 then.

**Shader changes draw a Huge map fully revealed before they are done** (the owner's rule,
2026-11-11; this holds at every tier). A change to a shader, or to what the renderer builds for
the map each sync (meshes, textures, atlases, instance buffers), is tested end to end on the
largest case with `node tools/r3d-huge.mjs --bin <built>/ffh-play` (add `--layer sea` when the
change touches a map layer): a new Huge game, the HD-2D map zoomed out, debug reveal switched on.
The revealed map must arrive and one complete frame of it be drawn within 30 s under SwiftShader
(10 s with `--gpu`), with no console error and no crashed tab. A change that fails this is not
finished: say so, don't merge it. (Why: the sea names' halo was blurred
once per letter, unnoticed with five seas known and a frozen tab with the 200 of a Huge map
revealed.)

Every commit still needs the hard-rule gate (`tools/agents/build-tree.sh --test`, `ffh data` with 0
warnings) plus `bash tools/web-check.sh`. Run `node tools/web-smoke.mjs --bin <built>/ffh-play`
before pushing to master. Regressions the owner reports get fixed in the next batch; don't try to
pre-empt every conceivable one.

A change to the transport, the web host or `crates/ffh-web` also runs the static site's smoke test
before it goes to master (`tools/web-site.sh OUT`, then `node tools/web-site-smoke.mjs --site OUT`;
docs/WEB.md): master is deployed to the public site by CI.

When a batch passes the gate and the end-to-end tests, merge it to master and push it without
asking ("Merging to master" under Workflow).
