# Persons of Note

This game's own rules, not FFH2's. **A design: nothing here is built.** A civilization has a small
cast of named people (its heroes, its best veterans, its great people, the leaders who
capitulated to it). Its cities are held by Houses, each presided over by one of them; its armies
are commanded by them; its vassals are ruled by them. Each wants one thing, and can take what
they hold when they are disloyal. The owner's concept and decisions of 2026-11-18; the numbers
are first guesses, and "Open points" lists what is not decided.

| Stage | What | Status |
|---|---|---|
| 0 | the vassal levy: an army requisitions from its owner's vassals' cities | not built |
| 1 | the cast, the Court screen, Houses and grants, the commander's post, wants and loyalty; no consequences. The per-city named governors and their traits go | not built |
| 2 | consequences: a House's and a commander's secession, dismissal, heirs | not built |
| 3 | vassals: a House released as a vassal, a capitulated leader as a person, claims | not built |
| 4 | foreign persons: seeing them, courting them, agents abroad | not built |

Stage 0 needs no persons and can be built first. Stage 1 alone shows whether the cast is worth
looking at before anything can go wrong.

## What it is for

A large empire is shared with people who have a view of how it is ruled. Today growth adds more
cities and units of the same kind; with this system each city gained is granted to someone, and
the decisions the player already makes (war and peace, religion, government, who gets a command)
have an audience inside its own borders that can act.

Wanted:
- **A cast known by name**, 6 to 10 people, each of whom arrived through something the player
  watched happen. Nobody arrives as a stranger.
- **Dilemmas about trust**: who to give power to, when the capable and the loyal differ.
- **Weight on choices already made.** No new decision screens: a war, a religion, a government
  cost or earn something at home.
- **Governments that differ in kind**: each a different deal with the holders (the Crown's size,
  the levy quota; docs/GOVERNORS.md part 5, not built, is the same idea).
- **Flat attention as the empire grows.** A thirty-city empire has five or six Houses to think
  about, not thirty governors.
- **Trouble seen coming and answered on the map**: a countdown, a stated outcome, a counter.
- **The same on the other side**: rivals have holders too, a way to hurt a rival without armies.

Not wanted:
- **Crusader Kings' breadth**: no families, marriages, ageing, trait lists or scheme screens.
- **A bench of stat sticks.** A person gives no yield and has no traits, and nothing is matched
  to anything: the first governor rules were dropped for that (docs/GOVERNORS.md, History), and
  a named governor with a trait in every city is the same fiddle (the owner, 2026-11-18).
- **Dice.** No betrayal a roll decides, no event pop-up with two flavoured options.
- **Upkeep chores.** No meter topped up with gifts; a person asks for attention only when a
  threshold is crossed.
- **Kicking the loser.** Loyalty follows what the player chose, never how the game is going.
- **A second game beside the map.** Politics ends in armies, cities and borders.
- **Losses that end the game.** A defection takes what one person holds, which the player chose.
- **A human-only weapon.** What the human can do to an AI's cast the AI defends against and does.

Out of scope (the owner's decision, 2026-11-18): **a human as a vassal.** No rule here acts on a
human player's cities as a vassal's: a human vassal is levied nothing, and its leader is in
nobody's cast.

## Rules

### The cast

A person (`Game::persons`, saved) has a name, an origin, one want, a loyalty, at most one post
and at most one rival. No traits. A person is tied to a unit while that unit is on the map, and
outlives it only in a post that takes the unit off the map (below).

**Who enters.**
- *Heroes*: a unit with `PROMOTION_HERO`, when its owner gains it. Its want is authored in its
  unit's data.
- *Veterans*: an army unit (docs/ARMIES.md, "Army units") that reaches level
  `PERSON_VETERAN_LEVEL` (5) plus the number of its owner's living veterans in the cast. The bar
  rises as the cast fills, so the cast stays small without a cap. It takes a name from its
  civilization's list and a want by the list below.
- *Great people*: a great person unit, while it lives. Spending it (a card, settling, hurrying)
  ends the person as it ends the unit; a post is a third use and keeps it.
- *Capitulated leaders* (stage 3): the leader of a civilization that becomes this player's
  vassal, holding the vassal ruler's post from the start.

**Who leaves.** The unit dies; the great person is spent; the person secedes, defects or resigns
(below); a vassal ruler's vassal dies. There is no ageing. A House's head who leaves is
followed by an heir (Houses, below).

**Wants.** One each. A want is always something the player can do on the board at that moment
(the owner's rule, 2026-11-18): a city granted, an army given, a war declared, a city taken, a
wonder ordered. Never an outcome the player does not control (a score, a rival's choice, luck),
and never something out of reach: a want whose act becomes impossible (the named civilization is
dead or was never met, the city is razed) is replaced by another from the same list, without a
grievance. The Court screen states the act: `Wants: war on the Sheaim.`

| Origin | Want |
|---|---|
| Hero | authored, one line of data per hero |
| Veteran | a command; or land (a House, then a larger one than its rival's); or war on a named civilization (the one it earned most of its experience against) |
| Great Prophet | the state religion kept |
| Great Merchant | peace |
| Great Engineer | a world wonder under way or built in the last `PERSON_WONDER_TURNS` (30) turns |
| Great Commander | a command |
| Great Bard, Sage | land |
| Vassal ruler | below |

**A vassal ruler's wants** (stage 3; a released House's head changes to one of these, a
capitulated leader starts with one), each an act on the map:
- *a city returned*: a named city it once held, taken (if another holds it) and given to it;
- *war on a named neighbour*: the civilization it likes least among those the master has met;
- *protection*: while it is at war, an army of the master's standing in its land;
- *a resource*: a named bonus it lacks and the master has, given in a standing deal.

**Rivals.** When a command is given or a House founded, each other person of the cast who wants
that and holds no post becomes the appointee's rival (one rival at most: the first passed over
keeps it). What a rival is given after is a grievance.

### Loyalty

Loyalty is a sum of listed modifiers, each with a cause and, where it is timed, its turns left,
shown as attitude is shown: `Passed over for the 2nd Army: -2 (18 turns)`.

- *Only chosen acts move it*: a grant, an appointment or a dismissal, a change of
  civics or state religion, a declaration of war or a peace, a levy beyond the quota. Losing a
  city, a battle or ground in the score moves nothing.
- *The want*: met, +2 while it is; unmet with no post, -1 every `PERSON_IDLE_TURNS` (20) turns,
  to -3 at most. Idle ambition sours, slowly.
- *States* (thresholds, `PERSON_*` defines): **Loyal**; **Restless** at -3 or below: nothing
  happens, and rivals may court the person (stage 4); **Disloyal** at -6 or below: a countdown of
  `PERSON_DISLOYAL_TURNS` (10) turns begins and states its outcome:
  `House Valdric will secede with Prespur, Elam and Torrin in 8 turns.` Loyalty back above -6
  stops it.
- *Only holders act.* A person with no post who reaches Disloyal leaves the game (stage 4: to
  the rival that courted them).

There is no RNG in loyalty: every change has a listed cause and every outcome a stated date.

### Houses

Every city belongs to a House. The **Crown** is the leader's own House: it always holds the
capital, it has no person at its head and it cannot be disloyal. Every other House has a name of
its own (from its civilization's list: `governor_names.jsonc` becomes the Houses'), is presided
over by one person of the cast and holds one city or more. A House outlives its head.

- **A city gained** (founded, captured, traded, flipped) is the Crown's until it is granted.
- **Granting** a Crown city (never the capital) is one click, on the city or on the Court
  screen: to a House that exists, or to a person without a post, which founds a House
  ("House Valdric"). The list shows each candidate's want, state and cities.
  `Granted Torrin: +1 (30 turns)` for the House; `Torrin granted to House Valdric: -1 (30
  turns)` for its head's rival.
- **A grant is for good** (the owner's decision, 2026-11-18). A city cannot be revoked from a
  House, and a House's head cannot be replaced. A city leaves its House only by leaving its
  owner (captured, traded, flipped, razed), by the House's secession or by its release as a
  vassal; a city won back is the Crown's again. A House that has lost its last city ends, and
  its head is a person without a post.
- **The head stays on the map.** A hero or a veteran who presides over a House still fights.
- **Heirs.** When a House's head dies or leaves the cast, the House takes its heir from the
  cast: the player names one of its persons without a post, at once (with one such person, that
  one). With none the seat is empty until a person without a post enters the cast, who takes it.
  While the seat is empty the House cannot become Disloyal and its cities count as the Crown's
  for maintenance.
- **Loyalty is the House's.** The modifiers earned by what was done to the House (grants,
  levies, its rival's gains) stay with it when its head changes; the heir brings only their own
  want. Changing the head does not clear a grievance.
- **A Disloyal head's unit is the House's, not the player's**: from the turn the countdown
  begins it goes to the House's largest city and takes no orders (as a marching recruit takes
  none, docs/ARMIES.md), so the countdown cannot be ended by sending the head to die. Before
  that the unit is the player's to risk as any other.
- **What a House does.** Nothing to the city's yields. Its cities are built as governed cities
  are today (docs/GOVERNORS.md: roles, priorities, empire orders and requisition are the
  player's, unchanged). The House's name is the governor's: the per-city named governor and its
  trait are removed.
- **Why grant at all.** A city of a House counts half toward its owner's city-count maintenance
  (`PERSON_HOUSE_MAINTENANCE_PERCENT`, 50), and the government civic sets how many cities the
  Crown holds before each further one counts double (`CivicInfo::crown_cities`). And a landless
  person who wants land sours.
- **Under a direct-rule civic** (Despotism, God King) Houses stand as under any government:
  the civic says who picks the cities' queues (docs/GOVERNORS.md), not who holds them.

**A House's secession.** At the countdown's end the House's cities become a new civilization of
the owner's kind led by its head (`world::birth` with `release`, as the Sundering's factions:
buildings, culture and garrisons go with the cities; docs/CARDS.md), at war with its former
owner. An ordinary war: peace, and taking it back as a vassal, are possible.
- *The counter on the map*: a city of the House in which an army of the owner stands at the
  countdown's end does not go (a Disloyal commander's army does not count). Occupy the cities, meet the want, or accept the loss.
- *The exposure is chosen, once*: a House can take what it was granted and nothing else, and
  what it was granted cannot be taken back. One large House is cheap to keep content and costly
  to lose; several small ones are the reverse.

### Commanders

An army has one commander's post (the slot docs/ARMIES.md leaves for phases 6-7; what a loyal
commander is worth in battle is decided there). A veteran in the army, a hero travelling with it
or a Great Commander may hold it. A House's head holds no command (one post each).

**Dismissal.** Before the countdown: the post is empty, `Dismissed: -3 (30 turns)`. After it
began: the outcome happens at once.

**A commander's secession.** An army needs a supply line to a city of its owner
(docs/SUPPLY.md), so a seceding army needs a home. At the countdown's end, in this order:

1. **It joins a Disloyal House**: if the commander's rival is not that House's head, the sworn
   units go with the House when it secedes, and wait for it.
2. **It seizes a city**: the nearest city of its owner its sworn units can reach by the rules'
   path whose garrison is smaller than they are. The city and the sworn units become a new
   civilization as a seceding House does, led by the commander.
3. **It goes over** to the rival that courted the commander (stage 4), if that rival borders
   the owner or is at war with it: the sworn units change owner.
4. **Otherwise the commander resigns** and leaves the game. The army stays.

- *Sworn units*: the army's units on the turn the countdown begins. They are the ones that go,
  wherever they are moved after: splitting the army on the last turn saves nothing.
- *The counter on the map*: a sworn unit standing in a city of its owner with more of the
  owner's other land combat units than sworn units there does not go.
- *One army, one city.* Nothing else follows a commander's secession.

### The vassal levy (stage 0)

A master commands its vassals' soldiers through the armies' requisition (docs/ARMIES.md,
"Requisition"), the owner's decision of 2026-11-18. An army's *Requisition from* list accepts the
cities of its owner's vassals as well as its own.

- *The order* is a requisition order at the front of the vassal city's queue, as in a city of
  the master's. The vassal pays its production; what it was building keeps its progress.
- *The unit* is the master's when trained (its type by the master's `requisition_unit`, among
  what the vassal's city can train) and marches to the army as any recruit.
- *The quota*: the master's government civic sets how many levies a vassal owes in every
  `PERSON_LEVY_TURNS` (30) turns, per city of the vassal's (`CivicInfo::levy_quota`, in
  hundredths of a unit per city). Within the quota a levy costs the master nothing.
- *Over the quota* each order placed is a grievance of the vassal's ruler:
  `Levied beyond quota: -1 (20 turns)`. Before stage 3 there is no ruler: an AI vassal counts it
  in its attitude toward its master, which its choice to stay a vassal reads.

A human vassal is levied nothing (above). Requisition from the master's own cities, the Crown's
and the Houses', is free as today.

### Vassals (stage 3)

Crown, House, vassal is one ladder: each step gives up more of a city and costs less to hold.

| | Crown | House | Vassal |
|---|---|---|---|
| Commerce and research | the owner's | the owner's | the vassal's |
| What it builds | the owner's roles and orders | the owner's roles and orders | its ruler's choice, and the levies |
| City-count maintenance | full (double beyond the Crown's size) | half | none |
| Requisition | free | free | by the quota |
| Can leave | no | with its head | with its ruler |

- **Release.** A House is released as a vassal: its cities become a civilization of the owner's
  kind, its vassal, led by the House's head (`world::birth` with `release` and `vassal`).
  `Released as a vassal: +3`, and the want for land is met for good.
- **Staying a vassal** is the ruler's loyalty: Loyal or Restless it stays; Disloyal, the
  vassalage ends at the countdown. The rules' own conditions (`team_can_vassal_revolt`: the
  vassal or the master having lost half its land) still free a vassal. For a vassal whose ruler
  is in a cast this replaces the AI's own choice to leave.
- **Claims.** A person who defects to another civilization arrives with a claim on the cities
  they held or seized. A civilization that captures such a city may grant it to that person at
  once, without the capture's disorder.

### Foreign persons (stage 4)

- **Seen by contact**: a rival's person is known only through what they hold that the player
  touches: the commander of an army it has fought, the head of a House with a city it channels
  at or borders, a vassal ruler it has met. Their state is shown once they are Restless.
- **Courting**: an offer to a rival's Restless person the player knows: gold a turn, a House in
  the player's realm, or their want. While it stands it is a grievance of theirs against their
  own ruler (`Courted by the Hippus: -1`) and names the courting civilization as where they go:
  a courted House secedes as the courter's vassal, not as an independent civilization. It is
  public to the person's owner, as a hostile channel is.
- **Agents abroad**: a Great Bard, a Great Prophet or a hero, off the map, running a channel
  (docs/CULTURE.md) aimed at one city at full effect whatever the distance. An agent holds
  nothing: Disloyal, the channel ends and its target is told who ran it.

## AI

A rules change is not gated on the AI (CLAUDE.md, "How AI changes are checked"): the AI does
something reasonable with it. First version:
- it grants each city gained beyond the Crown's size to the Loyal House with the fewest cities,
  and founds a House for a landless person who wants land before enlarging one;
- commands go to its most loyal eligible person; a Restless person gets no army;
- an army marches into the cities of a Disloyal House, the largest first; a Disloyal
  commander's army marches to its most garrisoned city;
- it does not make a choice that would turn a holder Disloyal when another choice of near value
  does not;
- it levies its vassals to the quota and no further for its main army (`requisition_cities`);
- it courts a rival's Restless holder when at war with that rival.

To measure before stage 4 is kept: secessions per AI game and who caused them (telemetry
`person`: `entered`, `posted`, `granted`, `restless`, `disloyal`, `seceded`, `courted`). If a
human can peel an AI empire apart by courting and the AI cannot answer, courting is cut back.

## UI

- **The Court screen**: one row a person: portrait, name, origin, want, post (House with its
  cities, or army), state, rival. Opening a row lists the loyalty modifiers. No other screen is
  new.
- **Where the state is seen at a glance**: each city's name plate carries its House's colour;
  a Restless or Disloyal holder marks its cities' plates and its army's banner, with the
  countdown.
- **Previews at the decision**: the grant list, the civics screen, the war and peace dialogs
  and the Armies screen's *Requisition from* list state what the choice does to each holder
  before it is made: `House Valdric: becomes Restless.` `Over quota. Ruler: -1.`
- **Text** is bare (CLAUDE.md, "Rules text and tooltips"). House, Crown, Loyal, Restless,
  Disloyal and Sworn are keywords (`data/json/keywords.jsonc`).

## Open points for the owner

- **The Crown.** Whether the leader's own cities are a House like any other, and what holds its
  size down: here, city-count maintenance doubling beyond a number the government sets.
- **Who names the heir.** Here the player, among its persons without a post. The other
  reading: the House takes one by a rule (the longest in the cast), and the player has no say.
- **A Restless head sent to die** is allowed here (the unit is the player's until the
  countdown); the loyalty stays with the House, so it buys a new want and nothing else.
- **A House's secession when its head dies during the countdown**: here the heir inherits the
  countdown.
- **A House takes every city it holds.** The alternative is its seat alone, the others
  returning to the Crown; that makes large Houses nearly safe.
- **Whether a House's cities owe a levy quota too**, larger than a vassal's, so that
  requisition has a political price inside the realm as well.
- **A person holding both** a House and a command (the feudal lord leading his own levy): here
  one post each.
- **Steadfast heroes**: whether a hero's data may say it never passes Restless.
- **The vassal rulers' wants** above are a first list; which of them a ruler has is by its
  situation (a lost city first, then an enemy, then a missing resource), not drawn.
- **Calling up a vassal's standing units** (not only its production) at a larger grievance.
- **A seceded or released civilization's leader** needs a leader head: the person's name over a
  `preferred_breakaway` minor leader's portrait and traits (docs/CARDS.md), or its own.
- **Quotas and Crown sizes by government**: which civics levy freely, which hold a large Crown.
- **Whether a levied unit is the master's for good** or returns when the war ends.
