# ffh-cppai: the FFH2 0.41 C++ AI, ported for comparison

`crates/ffh-cppai` is a port of the original FFH2 0.41 DLL AI (CvUnitAI, CvPlayerAI, CvCityAI,
CvTeamAI, CvSelectionGroupAI, CvGameAI, the AI functions of CvUnit/CvPlot, and the Python AI
hooks `AI_chooseTech`, `AI_chooseProduction`, `AI_unitUpdate`, the mage AI, `warScript` and the
`onUnitBuilt` unit roles) as a second `Controller`, so it can play against ffh-ai in the same
game. It is not the game's AI: ffh-ai is (docs/AI.md). It exists to answer "how does the new AI
compare with the one FFH2 shipped?".

**Not a work target** (the owner's call, 2026-10-21): performance and AI-improvement sprints ignore
it -- don't optimize, tune or extend it, and leave it out of sprint measurements unless the owner
asks for a comparison. Keep it compiling with minimal mechanical fixes when ffh-core changes.

## Layout

- Every C++ AI method is a method of `Cai` (`src/lib.rs`) named `<class>_ai_<snake name>`, taking
  the game and the object id (`CvUnitAI::AI_attackMove()` -> `self.unit_ai_attack_move(g, u)`).
  `unit_N.rs`, `player_N.rs`, `city_N.rs`, `team_N.rs` are consecutive line ranges of the C++
  files (each header names its range); `py_1.rs`/`py_2.rs` the Python and the rules-class AI.
- C++ AI members that ffh-core keeps (the rules read them) are core fields; the rest live in
  `state.rs`, inside the controller (not saved: a game with this AI cannot be resumed).
- The AI draws its random numbers from its own stream (`Cai::rand`), so the rules' stream is
  the same whichever AI plays.
- The rules call it where the DLL calls its AI: the usual `Controller` decisions plus
  `Controller::hook` (`AiHook`: player/city/team turn steps, end-of-turn and group-attack spells,
  summons, `AI_nowHasTech`, `AI_doCommerce`, found values, `warScript`, `AI_chooseGroupflag`,
  `onUnitBuilt` roles) and `Controller::assign_working_plots` (the city AI's citizen management
  replaces the rules' governor for its cities). All hooks are no-ops for ffh-ai.
- Code comments: `// C++ quirk:` the DLL's behaviour kept as is; `// FIX:` a clear DLL bug fixed
  (24, e.g. the gold-traded decay that grew instead of shrinking, a pirate-cove spell id that did
  not exist, barbarians pillaging their own plot); `// PORT ADAPT:` a sensible equivalent where
  ffh-core differs (73, mostly NULL/NO_X arguments and guards where the C++ would crash).
  AI-to-AI deals (`AI_doDiplo`, `AI_doPeace`) ask the other player's controller first when it is
  not this AI (the DLL signs them at once).

The port was written function by function from the C++/Python by parallel agents and is not
audited line by line; it was debugged until full games play sensibly (the two integration bugs
that mattered: the BETTER AI group flags were never assigned, so every military unit skipped its
turns; and civilizations created mid-game took the controller of their slot).

## Running it

```
FFH_CPPAI=all|even|odd|<p>,<p>,..|barb ffh stats --seed 1 ...    # players played by the C++ AI
FFH_PLAYER_HANDICAP=<slot>:HANDICAP_X,...                       # per-slot handicaps
FFH_CPPAI_TRACE=all|<p>                                          # trace its production and groups
python3 crates/ffh-cppai/scripts/h2h.py --bin $B/ffh --seeds 1-12 --turns 690 \
    --at 100,200,300,400,500,600,690 --until-supremacy [--handicap-ffh HANDICAP_X]
```

`h2h.py` plays each seed twice with the seats swapped (C++ AI on the even slots, then the odd
ones; barbarians always ffh-ai), attributes players by the controller that played them (stats rows
carry `ctrl`), and reports world share, cities, population, techs, military, survival, the side of
each game's winner (the victor, or the top civilization at the end) and how it won.

## Results (2026-10-21, 6 civs, WORLDSIZE_STANDARD, seeds 1-12 x 2 seatings, Noble)

Full games (to turn 690, a victory, or clear supremacy):

| | C++ AI | ffh-ai |
|---|---|---|
| games won | 5 | 19 |
| how | 2 time, 3 supremacy | 6 cultural, 2 conquest, 4 time, 7 supremacy |
| world share at the end (summed, 3 civs each) | 50.1% | 49.9% |
| cities / population at the end | 25.7 / 252 | 21.5 / 212 |
| techs at turn 300 / at the end | 95 / 168 | 84 / 182 |
| military strength at the end | 3547 | 1481 |
| original civs eliminated (of 72) | 5 | 24 |
| cities taken from / lost to civs, per game | 10.0 / 7.0 | 8.1 / 12.0 |
| culture per game | 77k | 217k |
| casters alive at the end, per game | 186 | 11 |

At turn 300 the sides were even (C++ AI ahead on share in 5 of 8 seed pairs, 8 games each). The
C++ AI builds wider, richer and far more military empires and kills more ffh-ai civilizations,
but rarely converts that into a win; ffh-ai wins by victory conditions (above all culture) and by
the single runaway civilization that supremacy counts.

Same seeds with the ffh-ai slots at HANDICAP_EMPEROR (C++ AI at Noble): the C++ AI wins 14, ffh-ai
10 (C++ AI 8 supremacy, 6 time; ffh-ai 5 cultural, 4 supremacy, 1 time); end share 56.7% / 43.3%,
C++ AI ahead in 9 of 12 seed pairs. ffh-ai's edge is worth less than the three steps from Noble to
Emperor (research and unit costs +15%, higher maintenance and civic upkeep).

Turn time with the C++ AI on half the slots: 470 ms/turn on average (max 2 s), several times
ffh-ai's.

Caveats: the C++ AI plays this port's rules, which differ from FFH2 0.41 in the ways listed in
docs/PORTING.md ("Changes from FFH2"), and ffh-ai was tuned on them; rules-side AI judgements the
rules port reads as "no denial" (trade denials in `getTradeDenial`, `AI_unitValue`-based conscript
roles and upgrade targets) are only partly restored inside the C++ AI; 12 seed pairs is a small
sample (19-5 in wins is two-sided p ~ 0.007 as independent games; the share comparison is not
significant).
