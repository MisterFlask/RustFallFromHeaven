# TODO

The owner's queue of planned work: design work that is agreed in outline and not yet built. One
entry per item: what it is, the decisions already made, what is still open, and the area docs that
hold the detail. Open work on a built area lives in that area's doc (CLAUDE.md). Remove an entry
when its work is merged (the area doc then describes it as built).

## Starving a besieged garrison

Decided by the owner, 2026-11-14. Fills the gap docs/SUPPLY.md section 8 lists ("Starving a
besieged garrison"). Area docs: docs/SIEGES.md, docs/SUPPLY.md, docs/NAVAL.md (4.5).

**The rule.**

- A city has a counter of siege turns (saved, on the city), as an army has `unsupplied_turns`.
- **A turn counts** when, at the start of the city owner's turn, the city is invested: land units
  at war with its owner, seen by its team, on the land plots next to it, numbering at least half
  the units of the garrison and at least 3; animals do not count. Healing from the city and
  Sentinel repair keep today's test (one adjacent enemy unit).
- **An open harbour prevents it**: a city whose harbour is open (docs/NAVAL.md 4.4) does not
  count turns.
- **The effect is the armies' Hunger and Starvation** (docs/SUPPLY.md section 3), unchanged, on
  every unit standing in the city: counter 1 to `SUPPLY_HUNGER_TURNS` (2): Hunger, no healing of
  any kind (a unit's own heal and medics included); above: Starvation, -10% of full HP a turn
  down to 25%, and -25% damage dealt in a battle. It never kills.
- **Sentinels are exempt.**
- **Reset**: a turn that does not count pauses the counter; two such turns in a row set it to 0.
- **A supply source must not be besieged**: an invested city (the test above) supplies no army
  (docs/SUPPLY.md section 2); one adjacent enemy unit does not cut it. A port whose harbour is
  open stays a source while invested.
- **Sallies have no rule of their own**: a unit that attacks out of the city carries what
  Starvation gives any starving unit (the HP it lost, -25% damage dealt in a battle) and nothing
  more.
- **The bombard floor** (50% HP) and Starvation's floor (25%) stack as written.

**What the player sees.**

- The city's banner and panel: the siege turn and the state (`Hunger: turn 1 of 2.`,
  `Starvation: -10% HP a turn.`), for both sides.
- The path preview, for a move that would end the investment: `Ends the siege of Vallus
  (turn 7, Starvation).`
- A confirmation before such a move, only when the counter is past Hunger.

**The AI** (a rules change is not gated on it): the besieger keeps enough units next to the city
while the count runs; the defender sallies or sends relief when its city is invested.

## Vet the area docs' open lists

The area docs hold about 20 sections of open work, many probably stale. Check each entry against
the code and mark it done, still open, or dropped; delete the stale ones; move what the owner
wants done next into this file.

- Known gaps: docs/AI.md, docs/HUMAN.md, docs/CRUSADE.md
- Open: docs/ENGAGEMENTS.md, docs/SIEGES.md, docs/TACTICS.md, docs/PERF.md (two sections)
- Open points or questions for the owner: docs/ARMIES.md, docs/CULTURE.md, docs/WORKS.md,
  docs/GOVERNORS.md, docs/EARLY_UNITS.md, docs/UNIT_ROLES.md, docs/STATS_CONTENT.md,
  docs/ART.md 16.4, docs/DECKS.md 8.4
- Other: "Next steps" in docs/PORTING.md, "Not in this change" in docs/SUPPLY.md, the "Not built"
  notes in docs/NAVAL.md
