# TODO

The owner's queue of planned work: design work that is agreed in outline and not yet built. One
entry per item: what it is, the decisions already made, what is still open, and the area docs that
hold the detail. Open work on a built area lives in that area's doc (CLAUDE.md). Remove an entry
when its work is merged (the area doc then describes it as built).

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

## Persons of Note

A cast of named people (heroes, veterans, great people, capitulated leaders) who preside over the
Houses that hold cities, command armies and rule vassals, each want one thing, and can take what
they hold. Design: docs/PERSONS.md.

- Decided (2026-11-18): a disloyal holder takes what it holds; a master levies its vassals
  through the armies' requisition, with a quota by government; a human as a vassal is out of scope.
  Cities are held by Houses of several cities, not by per-city governors (the named governors and
  their traits go); a want is always an act the player can do on the board.
  A grant is for good: no city is revoked and no head replaced; a House outlives its head and
  takes its heir from the cast.
- Open: docs/PERSONS.md, "Open points for the owner".
- First step: the vassal levy (stage 0), which needs no persons.
