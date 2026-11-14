# Standing rules

The register of the owner's standing rules for this project: one entry per rule, each with a name
that docs, commits and code comments can cite, and a status. The full wording of a rule stays
where it is applied (CLAUDE.md for what every session must read, the area doc otherwise); the
entry here says what the rule is in a sentence, why it exists, and where its text lives.

**What is listed.** A rule about how work is done here, or a standing limit on what future work
may add to the game. Not listed: a decision about one feature (how Fireball works, what a
Sentinel does). Those live in the feature's spec and in docs/PORTING.md, "Changes from FFH2", and
change when the design does.

**Using it.**

- Cite a rule by its name: `R-DETERMINISM`. Where a rule's text lives, its lead carries the name
  in brackets. `grep -rn 'R-DETERMINISM' .` finds the text and every place that leans on it.
- A new standing rule gets an entry here in the commit that states it: name, date, the rule in a
  sentence, why, where the text is. Names are never reused or renamed.
- A rule that no longer holds is not deleted: its entry moves to "Deprecated" with the date, the
  reason and what replaced it, so an old citation still resolves. Its text is removed from
  CLAUDE.md or the area doc in the same commit, and the places that cite it (the grep above) are
  revisited.
- Only the owner makes, changes or deprecates a rule. When a rule's reason no longer seems to
  hold, say so to the owner; don't work around it.

Dates are when the owner decided. "start" means the rule has held since the port began.

## Active

### Engine and code

- **R-NO-CRATES** (start; CLAUDE.md, "Hard rules"). No external crates: std only.
  *Why:* crates.io is unreachable from the build environment.
- **R-DETERMINISM** (start; CLAUDE.md, "Hard rules"). Simulation state changes only through the
  game RNG; no `HashMap` iteration, wall clock or thread-dependent order in simulation code.
  *Why:* a game replays from its seed, which the run cache, forks, saves and every measurement
  depend on.
- **R-INTEGER-MATH** (start; CLAUDE.md, "Hard rules"). Rules use integer math; floating point only
  where the C++ used it, with the same operations. *Why:* results that are the same on every
  machine.
- **R-PORT-FROM-SOURCE** (start; CLAUDE.md, "Reference source"). When porting, read the C++ or
  Python being ported; never port from memory. *Why:* remembered rules are wrong in the details.
- **R-FFH2-START** (start; CLAUDE.md, "Hard rules"). FFH2 0.41 is the starting point, not an
  invariant: no tests, tools or machinery whose only purpose is keeping the port identical to
  FFH2, and a change is not wrong because it departs from it. *Why:* the game will change
  sweepingly once the base is stable.
- **R-RULES-RECORDED** (start; CLAUDE.md, "Hard rules"). A rules change is made on purpose, said in
  its commit, and listed in docs/PORTING.md, "Changes from FFH2". *Why:* so the game's rules never
  drift as a side effect of other work.
- **R-NO-MIRROR** (2026-10-28; CLAUDE.md, "Hard rules"). Mirroring the C++ is not a goal:
  restructure ported code when that makes it more correct or clearer. *Why:* the initial port is
  complete, and closeness to the C++ was its method, not a value. Replaced R-MIRROR-CPP.
- **R-NO-FEATURE-FLAGS** (2026-11-05; CLAUDE.md, "Hard rules"). A new game system is part of the
  rules in every game, not behind a game option. *Why:* one game to design, test and teach the AI.
  Replaced R-FEATURES-AS-OPTIONS.
- **R-NO-IDLE-SWITCH** (2026-11-10; CLAUDE.md, "Hard rules"). A game option stays only if a player
  can choose it on the new-game screen and it works; an AI switch only while its behaviour is
  being measured. *Why:* every switch is a second code path nobody plays.
- **R-GATE** (start; CLAUDE.md, "Hard rules"). `cargo build --release && cargo test --release`
  pass before a commit, and data loading reports 0 warnings.

### Scope

- **R-NO-SCENARIOS** (start; CLAUDE.md, top). Scenarios are out of scope for now.
- **R-CPPAI-BENCH** (2026-10-21; CLAUDE.md, top; docs/CPPAI.md). The ported C++ AI is a benchmark
  only: not optimized, tuned, extended or included in sprint measurements; a build break gets the
  minimal mechanical fix. *Why:* no mileage in it; the game's AI is ffh-ai.

### Workflow

- **R-AGENT-BUILDS** (start; CLAUDE.md, "Parallel agents"; docs/AGENTS.md). Build and run games
  through `tools/agents/` (shared builds, CPU slots), not `cargo build` in a worktree. *Why:*
  agents share one 4-core machine.
- **R-FABLE-OPUS** (2026-11-09; CLAUDE.md, "Fable designs, Opus implements"). A Fable model
  designs, briefs, reads results and gates; it hands the volume of writing to Opus subagents.
- **R-MERGE-MASTER** (2026-10-21; CLAUDE.md, "Workflow"). A change that passes the gate and the
  end-to-end tests is merged to master and pushed without asking; master is never pushed with a
  failing or skipped check. *Why:* master is deployed to the public site by CI.
- **R-PUSHED-LINE** (2026-10-21; CLAUDE.md, "Workflow"). The turn that pushes master ends with
  `WORK ITEMS HAVE BEEN PUSHED TO MASTER` and the link to the game in the browser.
- **R-NO-SEED-STORY** (2026-11-05; CLAUDE.md, "Workflow"). No test relies on how a seed's game
  goes; a test that needs a situation builds it by hand. *Why:* every AI or rules change changes
  every game's story.
- **R-ANIM-RECIPE** (2026-10; CLAUDE.md, "Unit animations"; anim-sprint/ANIMATING_UNITS.md).
  Unit animations follow the sprint's winning recipe; the strike goes the way the sprite looks.

### The AI: what it is

- **R-AI-EXPERT** (2026-10-21; CLAUDE.md, "What the AI is"). The AI plays the way an expert human
  would and could explain. Flavor and fun override this where another rule says so.
- **R-AI-OPPONENT** (2026-11-10; CLAUDE.md, "What the AI is"). The AI is an opponent for a human:
  what matters is how it stands against a player who does what it does not. *Why:* results among
  copies of itself are facts about those opponents.
- **R-AI-UNIQUE** (2026-10-20; CLAUDE.md, "What the AI is"). A civilization's AI uses its own
  units, spells and abilities whether or not they benefit it. *Why:* flavor.
- **R-AI-CONQUEST** (2026-10-21; CLAUDE.md, "What the AI is"). Warlike civilizations and
  Aggressive leaders play with an expansion drive, whatever it does to their win rate.
- **R-AI-VASSALS** (2026-10-21; CLAUDE.md, "What the AI is"). A hopelessly behind civilization
  offers vassalage to a stronger, decently aligned neighbour; one crushed in war capitulates.
- **R-AI-NO-SOLITAIRE** (2026-10-21; CLAUDE.md, "What the AI is"). The AI does not pursue the
  Tower of Mastery or the Altar of the Luonnotar; it still stops a rival about to win one.
  *Why:* boring solitaire victories.
- **R-AI-IN-AI** (start; CLAUDE.md, "What the AI is"; docs/AI_ONLY_RULES.md). AI timing and
  AI-only restrictions belong in `ffh-ai`, not in the rules.
- **R-AI-FOG** (2026-10-24; docs/AI.md, "Fog of war"). The AI sees units through fog of war, for
  defense and attack, never through stealth; unrevealed land stays unknown. *Why:* conflict is
  much of the fun, and honest inference would need a memory of what was last seen.
- **R-AI-TRAIT-PLAY** (2026-10-26; docs/AI.md, `traits.rs`). A trait's play is kept whatever the
  win rate does; a drop is a flaw to fix. *Why:* flavor as much as strength.

### The AI: how changes are checked

- **R-AI-RULES-UNGATED** (2026-11-03; CLAUDE.md, "How AI changes are checked"). A rules change is
  not gated on the AI: give the AI something reasonable to do with it and move on.
- **R-AI-NO-SELF-WINRATE** (2026-11-10; CLAUDE.md, "How AI changes are checked"). An AI change is
  never kept or dropped on the standard AI's win rate or share against copies of itself. *Why:*
  zero-sum, noisy, and it falls when every civilization conquers better.
- **R-AI-LOOP** (2026-11-10; CLAUDE.md, "How AI changes are checked"). An AI change goes through
  seven steps in order: start from counted failures in games, count the firings, build and check
  the estimator, judge the behaviour where it fired, follow the chain, one wide guardrail run,
  play it against something that is not itself. *Why:* earlier rounds built ideas before counting
  their situations. Replaced R-AI-LOCAL-EFFECT.
- **R-AI-PROBES** (2026-11-10; CLAUDE.md, "Probes"; docs/PROBES.md). Probes (AIs all in on one
  strategy) are measuring tools for balance and opponents for the standard AI; they do not play in
  the released game unless the owner makes one a civilization's play.

### Text and UI

- **R-BARE-TEXT** (2026-11-02; CLAUDE.md, "Rules text and tooltips"). Mechanical text is labels,
  numbers and the rule: no flavor. Flavor belongs to leaders' speech and the data's names and
  descriptions.
- **R-KEYWORDS** (2026-11-12; CLAUDE.md, "Rules text and tooltips"; docs/PLAY_DESIGN.md 4.8). A
  concept shared by many entities is a keyword with its reminder in its own box, never restated
  in an entity's rules text.
- **R-RULES-OFF-SCREENS** (2026-11-03; docs/HUMAN.md). A screen holds the state and the decision
  at hand; how something works is a tooltip or a help article, not text on the screen.
- **R-UI-SCREENSHOT** (2026-11-11; CLAUDE.md, "Show UI changes in a screenshot"). A change to what
  the player sees is shown to the owner in a phone-readable screenshot when it is done.

### Rendering and visual work

- **R-HD2D-DEFAULT** (2026-10-31; docs/HD2D.md). HD-2D is the default map renderer; the isometric
  map is its fallback and the flat map is for debugging.
- **R-SHOTS-HD2D** (2026-11-11; CLAUDE.md, "Verifying renderer and visual work"). Screenshots for
  the owner are taken on the HD-2D map; a look is designed for it first.
- **R-VERIFY-TIERS** (start; CLAUDE.md, "Verifying renderer and visual work"). Visual work is
  verified in proportion to the change (small, medium, large), not exhaustively. *Why:* the owner
  sees regressions, and one costs a follow-up fix, not harm.
- **R-HUGE-MAP** (2026-11-11; CLAUDE.md, "Verifying renderer and visual work"). A shader or
  per-sync renderer change draws a Huge map fully revealed within the time limit before it is
  done. *Why:* the sea names' halo froze the tab at 200 seas, unnoticed at five.
- **R-SITE-SMOKE** (start; CLAUDE.md, "Verifying renderer and visual work"; docs/WEB.md). A change
  to the transport, the web host or `crates/ffh-web` runs the static site's smoke test before
  master.

### Limits on the game's design

- **R-NO-ERAS** (2026-11-10; docs/PORTING.md, "No eras"). The game has no eras; nothing new is
  keyed to one.
- **R-NO-ROLES** (2026-11-06; docs/ENGAGEMENTS.md, docs/UNIT_STATS.md). Units have no battle
  roles: each unit type has its own rows and stats.
- **R-NATIONAL-UNITS** (2026-11-06; docs/PORTING.md, "The Skirmisher"; docs/EARLY_UNITS.md).
  A national unit is above its tier's curve and has no upgrade.
- **R-CARDS-NO-VICTORY** (2026-11-08; docs/CARDS.md, docs/DECKS.md). No card alters a victory
  condition.
- **R-GOVERNORS-NO-CHANNEL** (2026-10-30; docs/GOVERNORS.md, docs/CULTURE.md). A governor never
  sets, changes or ends a city's cultural channel. *Why:* a channel is the player's decision.

## Deprecated

Kept so that old citations resolve. None of these holds.

- **R-MIRROR-CPP** (start to 2026-10-28). Ported code echoes the C++ closely: its structure, its
  APIs, its quirks. *Deprecated:* it was the method of the initial port, which is complete.
  Replaced by R-NO-MIRROR.
- **R-FEATURES-AS-OPTIONS** (to 2026-11-05). A new system (governors, armies, cultural actions,
  engagement combat, crusades) is a game option, off by default. *Deprecated:* every game has
  them. Replaced by R-NO-FEATURE-FLAGS.
- **R-AI-WINRATE-GATE** (to 2026-11-03). An AI change is kept only if neither the win rate nor the
  share fell, on development and fresh seeds, with forks before any win-rate measurement and a
  docs/AI_LOG.md entry before every revert. *Deprecated:* the win rate cannot show the effect of
  most changes. Replaced by R-AI-LOCAL-EFFECT, then R-AI-NO-SELF-WINRATE.
- **R-AI-LOCAL-EFFECT** (2026-11-03 to 2026-11-10). An AI change is checked only for its local
  effect where it fired. *Deprecated:* it did not say where ideas come from or what they are
  measured against. It survives as step 4 of R-AI-LOOP.
