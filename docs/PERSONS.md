# Persons of Note

This game's own rules, not FFH2's. **A design: nothing here is built.** A civilization has a small
cast of named people (its heroes, its best veterans, its great people, the leaders who
capitulated to it) who hold its armies, a few of its cities and its vassals, who each want one
thing, and who can take what they hold when they are disloyal. The owner's concept and decisions
of 2026-11-18; the numbers are first guesses, and "Open points" lists what is not decided.

| Stage | What | Status |
|---|---|---|
| 0 | the vassal levy: an army requisitions from its owner's vassals' cities | not built |
| 1 | the cast, the Court screen, wants and loyalty, the commander and governor posts; no consequences | not built |
| 2 | consequences: secession, a governor's city, dismissal | not built |
| 3 | vassal rulers: releasing cities to a person, a capitulated leader as a person, claims | not built |
| 4 | foreign persons: seeing them, courting them, agents abroad | not built |

Stage 0 needs no persons and can be built first. Stage 1 alone shows whether the cast is worth
looking at before anything can go wrong.

## What it is for

A large empire is shared with people who have a view of how it is ruled. Today growth adds more
cities and units of the same kind; with this system each step of growth beyond a core hands an
army or land to someone, and the decisions the player already makes (war and peace, religion,
government, who gets a command) have an audience inside its own borders that can act.

Wanted:
- **A cast known by name**, 6 to 10 people, each of whom arrived through something the player
  watched happen. Nobody arrives as a stranger.
- **Dilemmas about trust**: who to give power to, when the capable and the loyal differ.
- **Weight on choices already made.** No new decision screens: a war, a religion, a government
  cost or earn something at home.
- **Governments that differ in kind**: each a different deal with the holders (the levy quota
  below; docs/GOVERNORS.md part 5, not built, is the same idea for governors).
- **Flat attention as the empire grows.** A vassal runs its own cities; the player keeps the
  decisions (war and peace, where the armies go, who holds what).
- **Trouble seen coming and answered on the map**: a countdown, a stated outcome, a counter.
- **The same on the other side**: rivals have holders too, a way to hurt a rival without armies.

Not wanted:
- **Crusader Kings' breadth**: no families, marriages, ageing, trait lists or scheme screens.
- **A bench of stat sticks.** A person gives no yield, and nothing is matched to anything: the
  first governor rules were dropped for that (docs/GOVERNORS.md, History).
- **Dice.** No betrayal a roll decides, no event pop-up with two flavoured options.
- **Upkeep chores.** No meter topped up with gifts; a person asks for attention only when a
  threshold is crossed.
- **Kicking the loser.** Loyalty follows what the player chose, never how the game is going.
- **A second game beside the map.** Politics ends in armies, cities and borders.
- **Losses that end the game.** One defection takes one army and one city.
- **A human-only weapon.** What the human can do to an AI's cast the AI defends against and does.

Out of scope (the owner's decision, 2026-11-18): **a human as a vassal.** No rule here acts on a
human player's cities as a vassal's: a human vassal is levied nothing, and its leader is in
nobody's cast.

## Rules

### The cast

A person (`Game::persons`, saved) has a name, an origin, one want, at most one judgment trait, a
loyalty, at most one post and at most one rival. A person is tied to a unit while that unit is on
the map, and outlives it only in a post that takes the unit off the map (below).

**Who enters.**
- *Heroes*: a unit with `PROMOTION_HERO`, when its owner gains it. Its want and trait are
  authored in its unit's data.
- *Veterans*: an army unit (docs/ARMIES.md, "Army units") that reaches level
  `PERSON_VETERAN_LEVEL` (5) plus the number of its owner's living veterans in the cast. The bar
  rises as the cast fills, so the cast stays small without a cap. It takes a name from its
  civilization's list (as a governor does) and a want by the list below.
- *Great people*: a great person unit, while it lives. Spending it (a card, settling, hurrying)
  ends the person as it ends the unit; appointing it to a post is a third use and keeps it.
- *Capitulated leaders* (stage 3): the leader of a civilization that becomes this player's
  vassal, holding the vassal ruler post from the start.

**Who leaves.** The unit dies; the great person is spent; the person defects, secedes or resigns
(below); the post that took them off the map is lost (the city falls, the vassal dies). There is
no ageing.

**Wants.** One each, pointing at a decision the player already makes:

| Origin | Want |
|---|---|
| Hero | authored, one line of data per hero |
| Veteran | a command (an army commander's post); or land (a governor's or a vassal ruler's post); or war on a named civilization (the one it earned most of its experience against) |
| Great Prophet | the state religion kept |
| Great Merchant | peace |
| Great Engineer | a world wonder under way or built in the last `PERSON_WONDER_TURNS` (30) turns |
| Great Commander | a command |
| Great Bard, Sage | a post (a governor's seat; an agent's, stage 4) |
| Capitulated leader | its levy quota kept (below) |

**Traits.** The governors' judgment traits (`governor_traits.jsonc`: Cautious, Ambitious), one
at most, with the same chance. A trait says how the person acts in a post, never a yield.

**Rivals.** When a post is filled, each other person of the cast who wants that kind of post and
holds none becomes the appointee's rival (one rival at most: the first passed over keeps it).
A rival's appointment is a grievance (below).

### Loyalty

Loyalty is a sum of listed modifiers, each with a cause and, where it is timed, its turns left,
shown as attitude is shown: `Passed over for the 2nd Army: -2 (18 turns)`.

- *Only chosen acts move it*: an appointment or a dismissal, a change of civics or state
  religion, a declaration of war or a peace, a levy beyond the quota. Losing a city, a battle or
  ground in the score moves nothing.
- *The want*: met, +2 while it is; unmet with no post, -1 every `PERSON_IDLE_TURNS` (20) turns,
  to -3 at most. Idle ambition sours, slowly.
- *States* (thresholds, `PERSON_*` defines): **Loyal**; **Restless** at -3 or below: nothing
  happens, and rivals may court the person (stage 4); **Disloyal** at -6 or below: a countdown of
  `PERSON_DISLOYAL_TURNS` (10) turns begins and states its outcome:
  `Will secede with the 2nd Army and Prespur in 8 turns.` Loyalty back above -6 stops it.
- *Only holders act.* A person with no post who reaches Disloyal leaves the game (stage 4: to
  the rival that courted them). No modifier is told to the player before a state changes, except
  in the preview of the decision that causes it (UI, below).

There is no RNG in loyalty: every change has a listed cause and every outcome a stated date.

### Posts

A post is something of the player's that a person holds. Appointing is one click on the Court
screen or on the thing held; the list of who may hold it shows each candidate's want and state.

| Post | Who | The unit | Loyal | Disloyal, at the countdown's end |
|---|---|---|---|---|
| Army commander | veteran, hero, Great Commander | stays on the map, with the army | the commander slot of docs/ARMIES.md phases 6-7 (its effect is decided there) | secedes with the army and one city (below) |
| Governor | any | leaves the map | replaces the city's governor; counts as `PERSON_GOVERNOR_GARRISON` (3) defenders against revolt pressure (docs/CULTURE.md) | the city goes independent, or to the civilization whose pressure on it leads; never the capital |
| Vassal ruler (stage 3) | any who wants land; a capitulated leader | leaves the map | rules the released cities as a vassal; levied by the quota | the vassalage ends; it may pledge itself to the rival that courted it |
| Agent abroad (stage 4) | Great Bard, Great Prophet, hero | leaves the map | a channel (docs/CULTURE.md) aimed at one city, at full effect whatever the distance | the channel ends and its target is told who ran it |

The posts are a ladder of trust: an agent can take nothing, a governor a city, a commander an
army and a city, a vassal ruler a country.

**A hero or a veteran in a governor's or a ruler's post is off the map.** That is the price of
the appointment: the level-8 hero made king no longer fights. Dismissed while Loyal, the unit
returns in the capital.

**Dismissal.** Before the countdown: the post is empty, `Dismissed: -3 (30 turns)`. After it
began: the outcome happens at once.

### A commander's secession

An army needs a supply line to a city of its owner (docs/SUPPLY.md), so a seceding army needs a
home. At the countdown's end, in this order:

1. **It seizes a city**: the nearest city of its owner its sworn units can reach by the rules'
   path whose garrison is smaller than they are. The city and the sworn units become a new
   civilization of the owner's kind led by the person, as the Usurper's Crown card's is
   (`world::birth` with `release`, docs/CARDS.md), at war with its former owner. An ordinary war:
   peace, and taking it back as a vassal, are possible.
2. **It goes over** to the rival that courted the person (stage 4), if that rival borders the
   owner or is at war with it: the sworn units change owner.
3. **Otherwise the person resigns** and leaves the game. The army stays.

- *Sworn units*: the army's units on the turn the countdown begins. They are the ones that go,
  wherever they are moved after: splitting the army on the last turn saves nothing.
- *The counter on the map*: a sworn unit standing in a city of its owner with more of the
  owner's other land combat units than sworn units there does not go. March the army home, meet
  the want, or accept the loss.
- *One person, one army, one city.* Nothing else follows a secession: no other city, no other
  army, no modifier on other persons.

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
- *Why not make everything a vassal*: a vassal's city pays the master no commerce and no
  research, builds only what its own ruler chooses apart from the levies, and can leave with its
  ruler. A city of the master's own pays, builds to order and costs maintenance. That is the core
  and the periphery.

A human vassal is levied nothing (above).

### Vassal rulers (stage 3)

- **Release.** The player picks cities (not the capital) and a person who wants land: the cities
  become a civilization of the player's kind, its vassal, led by the person (`world::birth` with
  `release` and `vassal`). Their garrisons go with them; the person's unit leaves the map.
- **Why release**: the cities stop counting for city-count and distance maintenance; a commander
  who wants land and holds a strong army is otherwise a secession in waiting, and the grant makes
  the same person a vassal who is levied.
- **Staying a vassal** is the ruler's loyalty: Loyal or Restless it stays; Disloyal, the
  vassalage ends at the countdown. The rules' own conditions (`team_can_vassal_revolt`: the
  vassal or the master having lost half its land) still free a vassal. This replaces the AI's own
  choice to leave for a vassal with a ruler in a cast.
- **Claims.** A person who defects to another civilization arrives with a claim on the city
  they held or seized. A civilization that captures a city one of its persons has a claim on may
  install that person as its vassal ruler there at once, without the capture's disorder.

### Foreign persons (stage 4)

- **Seen by contact**: a rival's person is known only through what they hold that the player
  touches: the commander of an army it has fought, the governor of a city it channels at or
  borders, a vassal ruler it has met. Their state is shown once they are Restless.
- **Courting**: an offer to a rival's Restless person the player knows: gold a turn, a post, or
  their want. While it stands it is a grievance of theirs against their own ruler
  (`Courted by the Hippus: -1`) and names the courting civilization as where they go. It is
  public to the person's owner, as a hostile channel is.
- **A flipped city's governor** comes with it, in the same post.

## AI

A rules change is not gated on the AI (CLAUDE.md, "How AI changes are checked"): the AI does
something reasonable with it. First version:
- commands and vassals go to its most loyal eligible person; a Restless person gets an agent's
  or a governor's post, not an army;
- a Disloyal commander's army marches to its most garrisoned city;
- it does not make a choice that would turn a commander or a ruler Disloyal when another choice
  of near value does not;
- it levies its vassals to the quota and no further for its main army (`requisition_cities`);
- it courts a rival's Restless commander when at war with that rival.

To measure before stage 4 is kept: secessions per AI game and who caused them (telemetry
`person`: `entered`, `posted`, `restless`, `disloyal`, `seceded`, `courted`). If a human can peel
an AI empire apart by courting and the AI cannot answer, courting is an exploit and is cut back.

## UI

- **The Court screen**: one row a person: portrait, name, origin, want, post, state, rival.
  Opening a row lists the loyalty modifiers. No other screen is new.
- **Where the state is seen at a glance**: a mark on the army's banner and on the city's name
  for a Restless or Disloyal holder, with the countdown.
- **Previews at the decision**: the civics screen, the war and peace dialogs, the appointment
  list and the Armies screen's *Requisition from* list state what the choice does to each holder
  before it is made: `2nd Army's commander: becomes Restless.` `Over quota. Ruler: -1.`
- **Text** is bare (CLAUDE.md, "Rules text and tooltips"). Loyal, Restless, Disloyal and Sworn
  are keywords (`data/json/keywords.jsonc`).

## Open points for the owner

- **A governor's seat needs a reason beyond the want.** The revolt resistance above is a first
  guess; without a reason nobody appoints a hero to a city.
- **How many governor seats.** The cast's size limits them today; a number by government civic
  would be one more thing governments differ in.
- **Steadfast heroes**: whether a hero's data may say it never passes Restless.
- **The usurper's war** is ordinary here; the Usurper's Crown card's is not forced either, but
  its -6 attitude keeps it hostile. Whether a seceded commander should be as hard to win back.
- **Calling up a vassal's standing units** (not only its production) at a larger grievance: the
  map counter to a Restless ruler, as marching home is to a commander.
- **A released vassal's leader** needs a leader head: the person's name over a
  `preferred_breakaway` minor leader's portrait and traits (docs/CARDS.md), or its own.
- **Quotas by government**: which civics levy freely and which hardly at all.
- **Whether a levied unit is the master's for good** or returns when the war ends.
