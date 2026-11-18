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

## Persons of Note: the stages not built

The cast, Houses, demands, loyalty and their consequences are built (docs/PERSONS.md, stages 1
and 2, "The first build"). Still to build, each described there:

- The vassal levy (stage 0): an army requisitions from its owner's vassals' cities, by a quota.
  Needs no persons.
- Vassal rulers (stage 3): a released head stays in the cast as the vassal's ruler, with
  loyalty and demands; a capitulated leader as a person; claims abroad.
- Foreign persons (stage 4): seeing a rival's holders, courting them, agents abroad. Courting
  is kept only if the AI can answer it.
- The annals (stage 5): what each character did in the player's earlier games.
- The cast itself: a civilization has five minor leaders, so at most five persons who are not
  heroes. How many it needs, and which enter as soldiers and which as great people, is the
  owner's.
- To be judged in play (docs/PERSONS.md, "Open points for the owner"): the proposals the first
  build made (demands with a term, the order quota, Restless withholding, claims, leagues),
  the Crown's size, a House's say over its cities' roles, a commander's effect in battle.
