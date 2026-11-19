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

## Houses: what is not built

Houses as vassal civilizations (tribute, the levy, breaking away, side deals with a rival's
Houses) are built (docs/PERSONS.md, "The second build"); the third build's part 1 (2026-11-19)
removed Persons of Note (the cast, commanders, veterans and great persons as founders) and made a
realm's first grant found a House; its part 2 built splits, ultimatums and contracts, courtship,
the price on the world unit, shared knowledge and gold tribute. Still to build, each described
there:

- Claims abroad (stage 3).
- The annals (stage 5): what each House did in the player's earlier games.
- Ultimatums and courtships among AIs: in AI games neither fires (docs/AI_LOG.md, "Houses without
  Persons of Note": the AI keeps its Houses Pleased, and courts only an enemy's Cautious House),
  so the AI's answer to an ultimatum and its courting are exercised only against a human. A probe
  that squeezes its Houses (tribute at the most, requests refused) would show both.
- To be judged in play (docs/PERSONS.md, "Open points for the owner"): requests with a term, an
  Annoyed House's withheld levy, claims, the Crown's limit of 3, the tribute's loyalty cost.
