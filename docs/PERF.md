# Performance

Where the CPU time of a game goes and how to cut it without changing the game. The workload
that matters is AI self-play: `ffh bench` / `ffh autoplay`, the AI batches (`ai_metrics.py`,
docs/AI.md), `ffh-stress`, `ffh-play soak`, and the AI turns of the browser client
(WebAssembly). Figures are CPU time on this container (4 shared cores), from `ffh bench`.

## Rules for performance changes

1. **The game stays bit-identical**: the same random draws, decisions and save bytes.
   `tools/agents/perf-ab.sh same --set dev --soak` must print IDENTICAL for every game; then check
   once with `--set val`, configs not used while working (docs/AGENTS.md, "Performance A/B").
   Passing those games is the check, not the argument: make the change identical by
   construction and say why in a comment.
2. **`&self` is pure.** Code reached through `&Game` writes no game state (the path finders are
   scratch storage, not saved, and ffh-core keeps no caches written by reads; path.rs, "No
   reuse"), so skipping, adding or reordering a read changes nothing else. Keep it that way: a
   cache must be filled from `&mut` code, or be a memo within one call whose inputs cannot change
   during it (`PathCallbacks::from_memo` and `DefenseMemo` in `path.rs`).
3. **A faster version that plays differently is a rules or AI change** (CLAUDE.md): a path
   heuristic that breaks ties differently, following a path instead of searching again, an AI
   that remembers failed targets. Those are listed under "Open: changes the game" and need a
   deliberate decision and the AI measurements of docs/AI.md.
4. **Timing**: `tools/agents/perf-ab.sh time` holds every CPU slot and the build lock, alternates
   the two builds and reports the child's CPU time. Whole runs drift by 5-20% between sessions on
   this host (contention that `steal` does not show): compare builds only within one session, by
   the median of the per-round ratios. `tools/perf-stacks.py` reads `perf` call stacks (only the
   cpu-clock event exists in this VM; `apt-get install linux-tools-6.8.0-31-generic` provides
   `/usr/lib/linux-tools-6.8.0-31/perf`).

## Where the time goes

Seed 1, 8 civs, standard, 400 turns (config D1), inclusive share of the CPU samples; "after" is
the same work in the October 2026 build as a share of the old total (the new total is about 49%
of the old):

| | before | after |
|---|---|---|
| the AI controller (`play_turn`, incl. the moves it orders) | 71% | 30% |
| rules path searches (`generate_path`) | 39% | 9.3% |
| - `pathValid`, of it the danger scan `AI_getPlotDanger` | 24% (18.5%) | 2.9% (2.0%) |
| - `pathCost`, of it the range defense | 12% (8.2%) | 4.2% (0.9%) |
| the AI's reachability flood (`reach::Flood`) | 20% | 7.0% |
| type lookups by name (`Registry::lookup`) | 2.2% | 0.5% |
| plot-group updates (`plot_update_plot_group_player`) | 4.9% | 2.0% |

On an 18-civ huge map (H1: seed 3, 300 turns) the plot groups were 15.5% before: every city
founding or capture runs a game-wide update that visits every plot for every player, the C++
design.

Whole games, interleaved in one session (3 rounds each):

| | before | after | |
|---|---|---|---|
| D1: 8 civs, standard, 400 turns | 40.5 s, 100 ms/turn | 19.9 s, 48.6 ms/turn | -53% (-49% to -53% in 3 sessions) |
| H1: 18 civs, huge, 300 turns | 69.7 s, 228 ms/turn | 36.9 s, 120 ms/turn | -47% (2 sessions) |
| huge-map setup (1 turn, median of 8) | 1144 ms | 993 ms | -13% |
| data load (`ffh data`, median of 8) | 243 ms | 187 ms | -23% |

(This host ran 20-35% slower in those sessions than on a quiet morning, when D1 took 33 s;
the ratios hold.)

## Done (October 2026)

Each change alone against the base, D1, median of 3 interleaved rounds:

| change | where | CPU |
|---|---|---|
| `pathValid`'s parent part (danger scan, territory and move-through tests) once per expanded node instead of once per child | `path.rs` (`path_valid_from`, `PathCallbacks::from_memo`) | -28% |
| AI flood: per-run memo of the plot-only tests; early stops for callers that only need the nearest turns (garrison, approach, caster); no expansion at the turn budget | `ffh-ai/src/reach.rs`, `units.rs`, `campaign.rs` | -11% |
| range defense: per-axis wrap, row walk, skip the checks that cannot raise the best; `pathCost`'s defense modifiers memoized per search; route tests first | `plot/units.rs`, `path.rs` (`DefenseMemo`) | -10% |
| hot predicates: the 9 promotions with a combat mod listed at load; `isAlwaysHostile` once in `isEnemy`; empty-plot early returns | `unit/combat.rs`, `unit/movement.rs`, `utils.rs`, `plot/units.rs` | -9% |
| workers: AI build bans resolved once per search, build-independent parts of `build_value` memoized; casting: one castable-spell list per unit | `ffh-ai/src/workers.rs`, `casting.rs`, `magic.rs` | -7% |
| plot groups: cheap tests before the per-plot heap arrays, flood fill on plot indices, buildings without an effect skipped in bonus processing | `plot/ownership.rs`, `plot_group.rs`, `city/buildings.rs` | -2% (H1: -13%) |
| XML element names interned; no per-player unit counts on empty plots in `plot_process_area` | `ffh-data/src/xml.rs`, `plot/geography.rs` | data load -23%, huge-map setup -13% |

Checked: `perf-ab.sh same` IDENTICAL on D1-D4 and V1-V4 and the 3 soak games (save hashes too),
the test suite, `ffh data` 0 warnings; each change reviewed for a state in which it could differ.

Measured and left out: a bool early-exit `AI_getPlotDanger` (-6% alone, nothing measurable after
the `pathValid` memo, and a second copy of a ported function); skipping feature growth for
features no neighbour has and the cultural-owner city loop for a lone non-vassal team (about 1%
together by samples, not measurable); a light first View per turn (0.3%, and it skips a call
that can fill a saved rank cache).

## Open: same game

Rough shares of the CPU before the changes above; a share of what is left is larger.

- **Worker flood, branch and bound** (6.4%): `workers::worker_with` scores every plot of an
  8-turn flood; with the candidate plots and their best builds known first, the flood can stop
  once no unreached plot can beat the best score (it falls with the turns, ties keep the first).
- **`plot_movement_cost`** (now about 8% of samples): split into a part that depends only on the
  entered plot and one per edge, so the flood and `pathCost` can memoize the first.
- **`plot_get_best_defender`** (2.5%): each candidate's defense strength is evaluated once per
  pair comparison; evaluate it once per candidate.
- **Line of sight** (1.4-1.9%): `plot_can_see_displacement_plot` recurses per target cell;
  one table per `plot_change_adjacent_sight` call.
- **Per-plot heap vectors** (about 3% in the per-turn rules): `Plot` holds about 15 `IdxVec`s
  (culture, visibility counts, revealed, plot group, ...), one heap allocation each, so full-map
  walks wait on memory. Map-level arrays per player or team would be contiguous; the save format
  can keep the same logical order. (A flat plot-group array alone measured nothing on top of the
  plot-group changes above.)
- **Starting plots** (about a quarter of a 1-turn huge game): cache the per-plot inputs of
  `player_ai_found_value` that stay fixed during the search (fresh water, best yields per team).
- Smaller: a registry hasher cheaper than SipHash (the remaining `d.ty` / `d.t` lookups);
  `production::best_unit`'s candidates per role from data once; `allUpgradesAvailable` over a
  precomputed class list; `Map::xy`'s division (3% self, but reading `Plot.x/y` instead measured
  nothing); `plot_group_change_num_bonuses` clones the owner's unit list.

## Open: changes the game

These are the largest remaining items, and each changes play, so each is a decision:

- **Follow a found path instead of searching at every step** (about 48% of the search time on
  D1): `continueMission` calls `groupPathTo` once per step and uses only the first plot of each
  path (`group/missions.rs`, `group/movement.rs`). Searching again only when the next step is
  blocked, and once per turn, would remove most of it; the paths then differ as the state
  changes along the way.
- **A path heuristic in the units of the step costs** (about 41% of the search time):
  `pathHeuristic` is `stepDistance * 1000` while a step costs 20,000-60,000, so the A* expands
  nearly what a Dijkstra would. A heuristic closer to the step costs expands far fewer nodes but
  can return other paths (ties, and costs where it overestimates an end-of-turn step). The two
  overlap: together they remove less than the sum.
- **AI memory of failed moves**: workers pick targets the rules' path then refuses (the flood
  ignores `MOVE_SAFE_TERRITORY` and the danger test), and the failed search floods its area
  again on later turns (1.5-2.6%); campaign marches search to the far target at every step (3%).
- **Incremental plot groups** instead of the game-wide update on every city founding or capture
  (15.5% of H1 before the changes above; most of the first turn of a huge map, when every civ
  founds its capital).
- AI caches per turn: rivals' `best_race` in `victory::plan` (3% of H1), city sites per turn
  slice, worker scores shared by a player's workers.

## Build settings

Thin or fat LTO, `codegen-units = 1`, `panic = "abort"` and `target-cpu=native` measured within
noise of the default release profile (the hot code is inside ffh-core, where the default
thin-local LTO already inlines). Profile-guided optimisation (fat LTO, one codegen unit,
trained on 3 games of seeds no benchmark uses) is the exception: 16% less CPU on the base, and
on top of the changes above another 11% on D1 and 15% on H1 (2-3 interleaved rounds, noisy).
It needs the `llvm-tools` rustup component (an `llvm-profdata` of rustc's LLVM), a training run
and two release builds of ~4 minutes each, and the profile goes stale as the code changes, so it
is a pipeline decision (the shared builds of tools/agents, the web build), not done here.
