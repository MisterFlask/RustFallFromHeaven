# Persons of Note

This game's own rules, not FFH2's. **Stages 1 and 2 are built ("The first build"); the rest is a design.** A civilization has a small
cast of named, authored characters (its heroes, and the minor leaders of docs/minor-leaders who
join it as its best veterans and great people). Its cities are held by Houses, each presided over
by one of them; its armies are commanded by them; its vassals are ruled by them. Each makes
demands, gives less when crossed, and can take what they hold. The owner's concept and decisions
of 2026-11-18; the numbers are first guesses. "Open points" says which rules are the owner's
decisions and which are proposals still to be confirmed.

| Stage | What | Status |
|---|---|---|
| 0 | the vassal levy: an army requisitions from its owner's vassals' cities, by a quota | not built |
| 1 | the cast, the Court screen, Houses and grants, the commander's post, demands and loyalty shown; no consequences. The per-city named governors and their traits go | rules, state and read API built (`crates/ffh-core/src/persons/`, tests `crates/ffh-core/tests/persons.rs`); the governors gone; Court screen and AI to come |
| 2 | consequences: quotas and withholding, secession, leagues, the civil war of direct rule, heirs | rules built (`persons/secession.rs`, `persons/city.rs`, hooks in the order code, the attack rule, the city's reads); release as a vassal built too |
| 3 | vassals: a House released as a vassal, a capitulated leader as a person, claims abroad | not built |
| 4 | foreign persons: seeing them, courting them, agents abroad | not built |
| 5 | the annals: what each character did in the player's earlier games | not built |

Stage 0 needs no persons and can be built first. Stage 1 alone shows whether the cast is worth
looking at before anything can go wrong.

## The first build (2026-11-18)

The owner asked for the system to be built, with the ambiguous cases settled as seems sensible.
This section is the contract for that build: **where it and the rules further down differ, this
section holds.** The rules below remain the design; after the build they are rewritten as built.

**In the build:** the cast (heroes, veterans, great people as founders), the Court screen,
Houses, grants and claims, heirs, commanders, demands and loyalty with the four states, the
order quota and withholding, a head's trait in their House's cities, half city-count
maintenance, secession and leagues, a commander's secession, the civil war of direct rule, a
House released as a vassal, the AI's minimum, telemetry, the removal of per-city governors.

**Not in the build:** the vassal levy (stage 0), a vassal ruler's loyalty and demands (a
released head leaves the cast), capitulated leaders as persons, foreign persons, courting and
agents (stage 4), the annals (stage 5), a commander's effect in battle (docs/ENGAGEMENTS.md,
phase 7), dismissal and its grievance, the Crown's size (`crown_cities`: no city counts
double), a House's role for its cities (roles stay the player's in every city), the Trade
ambition (a peace or open borders are not the player's alone to make).

**Who has persons.** Every living civilization, human or AI. Not the barbarians, a minor
civilization or a crusade Host.

**The pool.** A civilization's persons other than heroes are its `preferred_breakaway` minor
leaders (`LeaderHeadInfo::minor_leader_of`). One is drawn with the game RNG among those not in
play as a leader and not drawn before in this game (`Game::persons_drawn`); a pool of one needs
no draw. With none left, the event makes no person. The Sundering's and the other revolts'
leader choice (`ffh/cards/sundering.rs`, `leader_in_play`) counts a drawn leader as in play.

**Entry**, in its owner's turn processing, once a turn:
- *Heroes*: every unit of the owner with `PROMOTION_HERO`, no duration, that is no person yet.
  Name: the unit's. No leader, so no portrait of a leader, no alignment of its own (its owner's)
  and no trait. Ambition: Faith for `UNITCOMBAT_DISCIPLE`, Works for `UNITCOMBAT_ADEPT`, War
  otherwise.
- *Veterans*: at most one a turn. The army unit (`unit_is_army_unit`) of the highest level
  (ties: the lower unit id) that is no person, if its level is at least `PERSON_VETERAN_LEVEL`
  (5) plus the number of the owner's living persons of origin Veteran, and the pool is not
  empty. The unit takes the person's name (`unit_set_name`).
- *Great people*: not at birth. A great person unit (the five card suits' unit classes and the
  Great Commander's) is a candidate in the grant list: granting a city to it draws a person,
  who founds the House, and **the unit is consumed** (the third use of a great person, beside
  the card and the settled specialist). That head has no unit on the map and so never dies.
- A person follows its unit through an upgrade or a conversion that keeps the owner
  (`unit_convert`), and leaves the cast when the unit dies or changes owner.

**Ambition**, from the leader's first personal trait: Aggressive, Raiders, Horselord,
Dexterous, Barbarian: War. Spiritual, Tolerant, Sinister: Faith. Industrious, Creative,
Philosophical, Arcane, Ingenuity, Summoner, Financial: Works. Any other: Land.

**Commanders** are not appointed one by one. A person is *with* an army when its unit is in the
army's group (a veteran in it, a hero grouped with it). An army's commander is the person with
it that the player named (`player_set_commander`, among the persons with it), else the one of
the highest level (ties: the earlier in the cast). A House's head never commands. The command
follows the unit: a person who leaves the group loses it without a grievance. `Army::commander`
holds the person.

**Demands** (one standing at a time for each person; checked and issued in the owner's turn
processing). A demand that names a city names its plot, since a city changes id with its owner.
- *A person without a post*: `a post` (met by a command or a House).
- *A commander*: War: the War list. Faith: the Faith list. Land, Works: `a House` (met when
  they head one).
- *A head*: the list of their ambition, then the Land list, then none this turn.
- *Land list*: (1) the Crown's city nearest their seat (not the capital) granted to their
  House; (2) else a city of another owner within 12 plots of a city of their House, of the
  barbarians first, else of a civilization that is not our team, vassal or master and whose
  team has less power than ours, the nearest: taken and granted to their House.
- *War list*: (1) while our team is at war with a civilization: its city nearest the person's
  unit (a head: their seat), taken; (2) else war with a named civilization: one we have met,
  are at peace with and may declare war on, whose team has less power than ours, of the
  alignment opposite the person's first, then the nearest capital. Met while our team is at war
  with it.
- *Faith list*: (1) with no state religion and a religion in one of our cities that we may
  adopt: a state religion adopted; (2) else the state religion in a named city of ours without
  it (a city of their House first, then the nearest to their seat or unit).
- *Works list*: a named wonder (world, team or national) their seat can build now, the one
  with the least production left: built in their seat. The term does not run while the wonder
  is in the seat's queue.
- *Met*: `Demand met: +3 (60 turns)`. *Lapsed* at its term (`PERSON_DEMAND_TURNS`, 40):
  `Demand refused: -3 (60 turns)`. *Made impossible by an act of ours* (the city granted to
  another House or razed by us, peace made with the civilization of a city to be taken): refused
  at once. *Made impossible otherwise*: replaced, no grievance. The next demand is issued
  `PERSON_DEMAND_REST` (20) turns after one was met or refused. With these numbers a House that
  is only ever refused stays Restless and does not reach Disloyal without a further grievance.

**Loyalty** belongs to the House for a head (`House::standing`) and to the person otherwise
(`Person::standing`); a founder's own modifiers move to the House they found. Modifiers:
`Demand met` +3 (60), `Demand refused` -3 (60), `Granted <city>` +1 (30),
`<city> granted to <House>` -1 (30) for a claimant House and -2 (30) for the claimant
commander, `State religion opposed` -2 while the state religion's alignment is the opposite of
the person's (a head's; standing, not stored), `Released kin` none. A modifier ends on the turn
its turns run out.

**States.** Devoted at +3 or more, Restless at -3 or less, Disloyal at -6 or less; an empty
seat is Loyal. A change of state is told to the owner. Disloyal starts the countdown
(`PERSON_DISLOYAL_TURNS`, 10) unless one runs; loyalty above the threshold stops it.

**Claims.** A city's claimants: each House of its owner with a city within
`PERSON_CLAIM_RANGE` (6) plots, and the commander whose army took it in the last 20 turns
(`City::taken_by`). The grantee's head becomes the rival of each snubbed claimant.

**Quota.** Each empire order and each requisition order placed in a House's cities is recorded
(`House::orders`, the turns placed). Orders left: Devoted: no limit. Loyal: the larger of 1 and
`cities x house_orders / 100`, less those placed in the last `PERSON_LEVY_TURNS` (30) turns.
Restless, Disloyal: none. An empty seat: no limit. `house_orders` is the government civic's
(Aristocracy 150, Republic 100, Theocracy 100, City States 50; a civic without the key: 100).
At no orders left an empire order is refused with the reason, and the requisition passes the
city over.
*As built:* `city_check_empire_order` keeps its `&'static str` refusal (`The House is Restless. It
takes no orders.`, `The House takes no more orders now.`); the reason with the House's name and the
turns is `city_house_takes_order`. Withdrawn or removed orders are not refunded.

**A head's trait** applies in their House's cities for every state. A trait with nothing a
city can carry gives nothing (no falling through to the next).
*As built:* the saved `City::house_trait` (`city_house_trait`), refreshed on a grant, a head's
change, a House's end and in the owner's turn (its own traits may change); each effect is added
where the city reads its owner's value (`persons/city.rs` lists them).

**Maintenance.** A city of a House with a head pays `PERSON_HOUSE_MAINTENANCE_PERCENT` (50) of
its number-of-cities maintenance.
*As built:* read functions `city_house_maintenance_saved_times100(c)` (what a grant of `c` saves)
and `house_maintenance_saved_times100(h)`, in hundredths of gold a turn.

**Heirs.** When a head leaves, an owner with exactly one person without a post has that one
succeed at once. With several the seat is empty until the owner names one (`player_name_heir`;
the AI in its turn); with none, until a person without a post exists, who then takes it.

**Secession.** Processed in the owner's turn, Houses before commanders.
- The new civilization: the owner's civilization type; its leader the head's leader, or for a
  hero a faction leader as the Sundering chooses one (`choose_faction_leader`); with no leader
  to be had there is no secession: the head resigns (leaves the cast) and the seat is empty.
  Named "<civilization> of House <name>" (a commander's: "<civilization> of <city>"). Its
  parent is the owner (`Birth::parent`: the owner's techs); it is played by the game's AI; it
  declares war on its former owner; `ai_attitude_extra` -4 both ways.
- The cities that go are released as the Sundering's are (buildings, culture, the owner's land
  combat units in them but heroes). A city where an army of the owner stands (an army that is
  not a Disloyal commander's) stays and is the Crown's. The head's unit goes with the House.
  With every city held the House ends and its head leaves the cast (their unit is removed).
- A league: every other House whose countdown is running joins, unless its head is the rival
  of a head already in it (or the reverse); the leader is the head of the House with the most
  cities (ties: the lower House id).
- A commander's sworn units are the army's units and the commander's own when the countdown
  starts. At its end a sworn unit standing in a city of its owner with more of the owner's
  other land combat units (not sworn) than sworn units there stays. The others: join a league
  formed this turn whose leader is not the commander's rival (they become its units in its
  capital, as the Sundering's defectors do); else seize the nearest city of the owner (not the
  capital) on their land area whose land combat units of the owner that are not sworn are fewer
  than they are; else the commander resigns and the units stay.
- Everyone who has met the owner is told: `House Ashledger secedes from the Bannor with
  Prespur, Elam and Torrin.`
- *As built:* in a league, the head of a member House whose every city is held leaves the cast
  with their unit removed; the others' units go to the new capital. A person of the owner whose
  unit leaves with a released city or a sworn unit leaves the cast (`X goes over to ...`). A
  commander joins only a league formed this turn; the city to seize is on the commander's land
  area (no path search); going over to a rival (3) is stage 4. A Disloyal head's unit is sent
  marching (`Unit::march`) to the House's largest city, and again whenever it is elsewhere while
  the countdown runs; `unit_house_refusal(u)` is the refusal (the rules do not stop an order
  given to it in that city: the client and the AI ask). A Restless or Disloyal commander's army:
  `unit_can_move_into` with `attack` refuses, `unit_attack_refusal(u)` says why. Battles go into
  the commanders' records (`Won a battle at <place>.`), a held city into its head's.

**Direct rule.** While `player_rules_directly` there are no Houses and no grant
(`Government: direct rule. No city can be granted.`). When a player with Houses comes under a
direct-rule civic, by any means, every House secedes at once as one league, whatever its state
and its rivalries; the counter on the map holds. Commanders are not touched.
*As built:* hooked in `player_governance_sync`; a House that cannot secede (no leader to be had)
is dissolved (its cities the Crown's, its head out of the cast), so no House stands under direct
rule (a harness invariant).

**Release.** `player_release_house`: a House with a head, not Disloyal, of an owner that is no
vassal, becomes the owner's vassal civilization (`Birth::vassal`), led as a secession is, at
peace; the head leaves the cast. Refused under No Vassal States.
*As built:* `player_check_release_house` / `player_release_house(p, h) -> Result<PlayerId, _>`;
also refused with an empty seat or without a leader to be had; all the House's cities go; the
vassal is played by the game's AI.

## What it is for

Two goals (the owner, 2026-11-18): **administering an empire is tricky**, and **the game makes
stories about particular people**, the stellar ones and the pains.

A large empire is shared with people who have a view of how it is ruled. Each city gained is
granted to someone, for good; what the ruler may order in it then depends on how that someone
stands with the ruler; and the decisions the player already makes (war and peace, religion,
government, who gets a command, who gets the city) are watched by people who can withhold and,
in the end, leave with what they hold.

Wanted:
- **A cast known by name**, 6 to 10 people, each of whom arrived through something the player
  watched happen, and each the same character in every game, so that a player meets them again.
- **Dilemmas about trust**: the general who wins is the general who can take a city.
- **Demands that cannot all be met.** Trickiness is conflict between what the holders want and
  what the board asks for, not a meter to watch.
- **A middle register.** A person can be a pain for a hundred turns without being a traitor.
- **Governments that differ in kind**: each a different deal with the holders (the Crown's size,
  the quotas, and direct rule, which has no holders at all).
- **Flat attention as the empire grows.** A thirty-city empire has five or six Houses to deal
  with, and less to decide in a House's city than in the Crown's.
- **Trouble seen coming and answered on the map**: a term, a stated outcome, a counter.
- **The same on the other side**: rivals have holders too.

Not wanted:
- **Crusader Kings' breadth**: no families, marriages, ageing, trait lists or scheme screens.
- **A bench to match.** Nothing is slotted and re-slotted to fit: a grant is for good and a
  head is not replaced. The first governor rules were dropped for the matching
  (docs/GOVERNORS.md, History), and a named governor with a trait in every city is the same
  fiddle (the owner, 2026-11-18). A number a person brings to what they hold is welcome.
- **Dice.** No betrayal a roll decides, no event pop-up with two flavoured options.
- **Upkeep chores.** No meter topped up with gifts. A demand is an act on the board (a city
  taken, a war declared, a wonder ordered), one at a time per holder, with a long term.
- **Kicking the loser.** Loyalty follows what the player chose, never how the game is going,
  and no demand asks for what the Crown lacks the means to do.
- **A second game beside the map.** Politics ends in armies, cities and borders.
- **A human-only weapon.** What the human can do to an AI's cast the AI defends against and does.

Out of scope (the owner's decision, 2026-11-18): **a human as a vassal.** No rule here acts on a
human player's cities as a vassal's: a human vassal is levied nothing, and its leader is in
nobody's cast.

## Rules

### The cast

**A person is an authored character** (the owner's decision, 2026-11-18): a hero, or a minor
leader of the civilization (docs/minor-leaders: a name, a title, a portrait, a backstory, an
alignment, personal traits). The same person is the same in every game. Which of them a game
sees is drawn with the game RNG, among those of the civilization not yet seen in this game, when
an entry below happens. A person (`Game::persons`, saved) is tied to a unit on the map, which
takes their name.

**Who enters.**
- *Heroes*: a unit with `PROMOTION_HERO`, when its owner gains it.
- *Veterans*: an army unit (docs/ARMIES.md, "Army units") that reaches level
  `PERSON_VETERAN_LEVEL` (5) plus the number of its owner's living veterans in the cast. The bar
  rises as the cast fills. The unit becomes a drawn person.
- *Great people*: a great person unit, while it lives, becomes a drawn person. Spending it (a
  card, settling, hurrying) ends the person as it ends the unit; a post is a third use.
- *Capitulated leaders* (stage 3): the leader of a civilization that becomes this player's
  vassal, holding the vassal ruler's post from the start.

**Who leaves.** The unit dies; the great person is spent; the person secedes, defects or resigns
(below); a vassal ruler's vassal dies. There is no ageing. A person who has left does not return
in that game.

**What a person brings**, the same in every game, and what a player learns about a character:
- *An ambition*: the kind of demand they make (below). Until one is authored it follows the
  person's first personal trait: Aggressive, Raiders: War. Expansive, Organized: Land. Spiritual:
  Faith. Industrious, Creative, Philosophical, Arcane: Works. Financial: Trade. Any other: Land.
- *A trait for their House's cities* (the owner's decision, 2026-11-18): below.
- *Their alignment* (authored already): a state religion of the opposite alignment is
  `State religion opposed: -2` while it lasts.
- When a person leads a civilization (a secession, a released vassal) they are its leader, with
  their portrait and traits, as breakaway factions' leaders already are (docs/CARDS.md).

**A head's trait.** The first of a head's leader traits that their House's owner does not have
applies in the cities of their House, and nowhere else.
- *Which trait*: the person's traits in their authored order (personal, then inherited), the
  first that is not a trait of the owner's leader or civilization. A commander's trait does
  nothing, and nothing takes its place (the owner's decision, 2026-11-18): a commander is told
  apart by the promotions their unit has earned. A hero has no leader traits.
- *What applies*, as if the city's owner had the trait, for that city alone: the trait's
  health and happiness; its commerce (`commerce_change`, `commerce_modifier`) and its plots'
  extra yield (`extra_yield_threshold`) and trade yield; its great people rate; its wonder
  production modifiers; the buildings and units it trains at double speed
  (`production_traits`); and for the units trained there, its free promotions.
- *What does not*: what a trait gives the player as a whole (civic upkeep, anarchy, the great
  general rate, pillage gold, upgrade cost, summons) and its flags (Adaptive, Agnostic,
  Barbarian, Insane, Sprawling, Fallow).
- *It follows the head*: an heir brings their own trait, and the House's cities change with
  it; units already trained keep their promotions. An empty seat has none.
- A person who leads a civilization (a secession, a released vassal) has all their traits, as
  any leader.

The cast to be authored (how many a civilization needs, which enter as soldiers and which as
great people) is the owner's, later.

**The record.** Every person keeps a list of their deeds (`Person::deeds`: battles won and lost
in command, cities taken, sieges their House's cities withstood, grants, demands met and
refused, who was preferred to them). The Court screen shows it, and the game's messages name the
person, not the army: `Maereth Ashledger took Torrin with the 2nd Army.`

### Demands and loyalty

**Loyalty** is a sum of listed modifiers, each with a cause and, where it is timed, its turns
left: `Demand refused: -3 (41 turns)`. Only chosen acts move it: a demand met or let lapse, a
grant, an appointment, a dismissal, an order beyond the quota, the state religion. Losing a city,
a battle or ground in the score moves nothing. There is no RNG in it.

**Demands.** Every person has one standing demand at a time, of their ambition's kind: an act
the player can do on the board (the owner's rule, 2026-11-18), named exactly:
`Demands: Torrin (Sheaim) taken and granted to House Ashledger. 34 turns.`

| Ambition | The demand, the first that applies |
|---|---|
| any, without a post | a command (War, Works) or a House (the others) |
| Land | the Crown's city nearest its lands granted; else a named foreign city nearest its lands taken and granted |
| War | war declared on a named civilization; at war with it, a named city of it taken |
| Faith | its religion the state religion; else its temple ordered in its seat; else its religion spread to a named city of ours |
| Works | a named wonder, or the costliest building its seat can build, ordered in its seat |
| Trade | peace with a named civilization we are at war with; else open borders with a named neighbour |
| Vassal ruler | a named city it once held taken and returned; else as its ambition |

- *Within the Crown's means when it is made*: a war or a conquest only against a civilization
  of less power than ours; a building the city can build. A demand that becomes impossible (the
  civilization dead, the city razed) is replaced, without a grievance.
- *The term* is `PERSON_DEMAND_TURNS` (40). Met: `Demand met: +3 (60 turns)`. Lapsed:
  `Demand refused: -3 (60 turns)`. The next demand comes `PERSON_DEMAND_REST` (20) turns later.
- *Demands conflict on purpose*: one House wants the war another wants ended; two want the same
  city. The player meets some and not others, and which is the politics.

**States**, by loyalty. In the plain case they are the record of the last two demands:

| State | Loyalty | A House | A commander |
|---|---|---|---|
| **Devoted** | +3 or more | takes every order, without a quota; joins no secession | cannot be courted |
| **Loyal** | | takes orders within its quota | commands |
| **Restless** | -3 or less | takes no new order: no empire order, no requisition | its army takes no order to attack |
| **Disloyal** | -6 or less | a countdown of `PERSON_DISLOYAL_TURNS` (10) turns to its secession | the same |

The countdown states its outcome: `House Ashledger will secede with Prespur, Elam and Torrin in
8 turns.` Loyalty back above -6 stops it. A person without a post who reaches Disloyal leaves
the game (stage 4: for the rival that courted them).

### Houses

Every city belongs to a House. The **Crown** is the leader's own House: it always administers
the capital, it has no person at its head and it cannot be disloyal. Every other House has a
name of its own (from its civilization's list: `governor_names.jsonc` becomes the Houses'), is
presided over by one person and holds one city or more.

**Grants.**
- *A city gained* (founded, captured, traded, flipped) is the Crown's until it is granted.
- *Granting* a Crown city (never the capital) is one click: to a House that exists, or to a
  person without a post, which founds a House. `Granted Torrin: +1 (30 turns)`.
- *Claims.* A city has claimants: each House with a city within `PERSON_CLAIM_RANGE` (6) plots
  of it, and the commander whose army took it in the last 20 turns. Granting it to one snubs the
  others: `Torrin granted to House Arne: -1 (30 turns)`, -2 for the commander who took it. The
  one preferred becomes the snubbed person's rival (one rival each: the latest). The list of
  candidates shows the claimants and what each choice does. Keeping the city in the Crown snubs
  nobody and costs maintenance (below).
- *A grant is for good* (the owner's decision, 2026-11-18). A city cannot be revoked from a
  House, and a House's head cannot be replaced. A city leaves its House only by leaving its
  owner (captured, traded, flipped, razed), by the House's secession or by its release as a
  vassal; a city won back is the Crown's again. A House that has lost its last city ends, and
  its head is a person without a post.
- *Granting a city to a commander* founds their House and ends their command (one post each):
  the way to reward a general and take the army from them at once.

**What the Crown may do in a House's city.** A Crown city is the player's as cities are today
(docs/GOVERNORS.md: roles, priorities, empire orders, requisition). A House's city is the
House's:
- *Its role* is proposed with the head's ambition as the priority (War: Garrison or Production.
  Land: Growth. Faith: Culture. Works: Production or Research. Trade: Commerce), not the
  empire's priorities, and the player cannot overrule it.
- *Orders by quota.* Each empire order and each requisition order placed in a House's cities
  counts against the House's quota: `CivicInfo::house_orders` (hundredths of an order per city
  of the House) in every `PERSON_LEVY_TURNS` (30) turns. At the quota the House takes no more
  until the turns run out. Devoted, it has no quota; Restless, it takes none (the states above).
  The Armies screen and the order list show what is left: `House Arne: 1 of 3 orders left.`
- *Everything else is unchanged*: its commerce, research and culture are the owner's, its
  garrison defends it, its cultural channel is the player's (docs/CULTURE.md).
- The House's name is the governor's: the per-city named governor and its trait are removed.

**Why grant at all.** A city of a House counts half toward its owner's city-count maintenance
(`PERSON_HOUSE_MAINTENANCE_PERCENT`, 50), and the government civic sets how many cities the
Crown holds before each further one counts double (`CivicInfo::crown_cities`). A person without
a post demands one. And a commander with a claim is safer landed than snubbed.

**Heads and heirs.**
- *The head stays on the map.* A hero or a veteran who presides over a House still fights.
- *Heirs.* A House outlives its head. When the head dies or leaves the cast, the player names
  the heir among its persons without a post, at once (with one such person, that one). With
  none, the seat is empty until a person without a post enters the cast, who takes it. While the
  seat is empty the House makes no demand, cannot become Disloyal, and its cities count as the
  Crown's for maintenance.
- *Loyalty is the House's.* Its modifiers stay with it when its head changes; the heir brings
  their own ambition and trait, and a new demand. Changing the head clears no grievance.
- *A Disloyal head's unit is the House's, not the player's*: from the turn the countdown begins
  it goes to the House's largest city and takes no orders (as a marching recruit takes none,
  docs/ARMIES.md). Before that the unit is the player's to risk as any other.

**A House's secession.** At the countdown's end the House's cities become a new civilization of
the owner's kind led by its head (`world::birth` with `release`, as the Sundering's factions:
buildings, culture and garrisons go with the cities; docs/CARDS.md), at war with its former
owner. An ordinary war: peace, and taking it back as a vassal, are possible.
- *The counter on the map*: a city of the House in which an army of the owner stands at the
  countdown's end does not go, and is the Crown's from then (a Disloyal commander's army does
  not count). Occupy the cities, meet the demand, or accept the loss.
- *Leagues.* Every other House whose countdown is running goes with it at once, as one
  civilization led by the head of the House with the most cities. Not a House whose head is the
  rival of a head already in the league: rivals never secede together.
- *The exposure is chosen, once*: a House takes what it was granted, and what it was granted
  cannot be taken back.

**Direct rule is civil war** (the owner's decision, 2026-11-18). Under a direct-rule civic
(Despotism, God King; docs/GOVERNORS.md) there are no Houses: every city is the Crown's, no city
counts double and none can be granted. A game starts so. Adopting one while Houses stand is
their secession, all of them, Devoted and rivals included, at once and as one league; the
counter on the map holds as above, and a House with every city held ends, its head leaving the
cast. The civics screen states it before the choice:
`Adopting God King: 3 Houses secede with 9 cities. Held by armies: Torrin, Elam.`

### Commanders

An army has one commander's post. A veteran in the army, a hero travelling with it or a Great
Commander may hold it; a House's head may not (one post each).

- *What a commander is worth* is their unit: its level and the promotions it has earned. A
  commander has no attribute of their own. What the commander's post adds in battle (the
  commander auras of docs/ENGAGEMENTS.md, phase 7) rises with the unit's level and is decided
  there. The capable commander is the one who has fought most, and so the one with the most
  claims.
- *Claims*: the commander whose army takes a city is a claimant to it (Houses, above).
- *Restless*, the army takes no order to attack: it moves and defends.
- *Dismissal.* Before the countdown: the post is empty, `Dismissed: -3 (30 turns)`. After it
  began: the outcome happens at once.

**A commander's secession.** An army needs a supply line to a city of its owner
(docs/SUPPLY.md), so a seceding army needs a home. At the countdown's end, in this order:

1. **It joins a league**: if a House is seceding or its countdown is running, and its head is
   not the commander's rival, the sworn units go with that House.
2. **It seizes a city**: the nearest city of its owner its sworn units can reach by the rules'
   path whose garrison is smaller than they are. The city and the sworn units become a new
   civilization as a seceding House does, led by the commander.
3. **It goes over** to the rival that courted the commander (stage 4), if that rival borders
   the owner or is at war with it: the sworn units change owner.
4. **Otherwise the commander resigns** and leaves the game. The army stays.

- *Sworn units*: the army's units on the turn the countdown begins. They are the ones that go,
  wherever they are moved after.
- *The counter on the map*: a sworn unit standing in a city of its owner with more of the
  owner's other land combat units than sworn units there does not go.

### The vassal levy (stage 0)

A master commands its vassals' soldiers through the armies' requisition (docs/ARMIES.md,
"Requisition"), the owner's decision of 2026-11-18. An army's *Requisition from* list accepts the
cities of its owner's vassals as well as its own.

- *The order* is a requisition order at the front of the vassal city's queue. The vassal pays
  its production; what it was building keeps its progress.
- *The unit* is the master's when trained (its type by the master's `requisition_unit`, among
  what the vassal's city can train) and marches to the army as any recruit.
- *The quota*: `CivicInfo::levy_quota` of the master's government, counted as a House's is, and
  smaller. Before stage 3 a vassal has no ruler in a cast: at its quota it takes no more.
  From stage 3 its ruler's state sets it as a House's does (Devoted, Loyal, Restless).

A human vassal is levied nothing (above).

### Vassals (stage 3)

Crown, House, vassal is one ladder: each step gives up more of a city and costs less to hold.

| | Crown | House | Vassal |
|---|---|---|---|
| Commerce and research | the owner's | the owner's | the vassal's |
| What it builds | the owner's roles and orders | the House's role; orders by quota | its ruler's choice; levies by a smaller quota |
| City-count maintenance | full (double beyond the Crown's size) | half | none |
| Can leave | no | with its head | with its ruler |

- **Release.** A House is released as a vassal: its cities become a civilization of the owner's
  kind, its vassal, led by the House's head (`world::birth` with `release` and `vassal`).
  `Released as a vassal: +3`.
- **Staying a vassal** is the ruler's loyalty: Disloyal, the vassalage ends at the countdown.
  The rules' own conditions (`team_can_vassal_revolt`) still free a vassal. For a vassal whose
  ruler is in a cast this replaces the AI's own choice to leave.
- **Claims abroad.** A person who defects to another civilization arrives as a claimant to the
  cities they held or seized. A civilization that captures such a city may grant it to that
  person at once, without the capture's disorder.

### Foreign persons (stage 4)

- **Seen by contact**: a rival's person is known only through what they hold that the player
  touches: the commander of an army it has fought, the head of a House with a city it channels
  at or borders, a vassal ruler it has met. Their state and their demand are shown once they
  are Restless.
- **Courting**: an offer to a rival's Restless person the player knows: gold a turn, a House in
  the player's realm, or their demand met by the player. While it stands it is a grievance of
  theirs against their own ruler (`Courted by the Hippus: -1`) and names the courting
  civilization as where they go: a courted House secedes as the courter's vassal. It is public
  to the person's owner, as a hostile channel is.
- **Agents abroad**: a Great Bard, a Great Prophet or a hero, off the map, running a channel
  (docs/CULTURE.md) aimed at one city at full effect whatever the distance. An agent holds
  nothing: Disloyal, the channel ends and its target is told who ran it.

### The annals (stage 5)

The client keeps, outside the game's state, what each character did in the player's finished
games (served to the end, seceded, fell in battle, cities taken). The Court screen shows it
under the record: `Earlier games: seceded twice; held Prespur to the end once.`

## AI

A rules change is not gated on the AI (CLAUDE.md, "How AI changes are checked"): the AI does
something reasonable with it. First version:
- it grants each city gained beyond the Crown's size to the claimant in the best state, and
  founds a House for a person without a post before enlarging one;
- each turn it works toward the standing demand that is cheapest for it among those of its
  Restless holders, then its Loyal ones (a demand for a war it was preparing anyway first);
- commands go to its highest-level eligible person who is not Restless;
- an army marches into the cities of a Disloyal House, the largest first; a Disloyal
  commander's army marches to its most garrisoned city;
- it adopts a direct-rule civic only with no House, or when its armies hold every House's city;
- it places orders in Houses' and vassals' cities up to the quota and no further;
- it courts a rival's Restless holder when at war with that rival.

To measure as each stage lands: secessions and leagues per AI game and what caused them, the
share of demands met, the turns Houses spend Restless (telemetry `person`: `entered`, `posted`,
`granted`, `demand`, `state`, `seceded`, `courted`). An AI that loses a House every game is an
AI the human need not fight: if so, the AI's demands are tuned before the human's are.

## UI

The interface of the first build (the owner asked for particular attention to it). The rules it
shows are "The first build"; the client's conventions are docs/HUMAN.md and docs/PLAY_DESIGN.md
(4.8, keywords).

### Principles (every part below follows them; when a case is not covered, decide by them)
1. **No surprise.** Every act that moves a person's or a House's loyalty shows the exact change before it is done, in the place where the act is done: the grant list, the civics screen, the religion screen, declaring war, making peace, razing a city. Wording is the rules' own preview lines (`House Arne: -1 (30 turns). Becomes Restless.`).
2. **State at a glance, never by colour alone.** One mark for a state everywhere (Court, city screen, Armies, map plates, unit panel): a word and a shape with a colour: Devoted ▲ green, Loyal ● neutral, Restless ◆ amber, Disloyal ✖ red with the turns left. One function draws it; do not restyle it per screen.
3. **From a demand to its act in one click.** A demand line carries the button that goes where it is met (below). A demand never requires hunting through screens.
4. **A grant is for good, and the screen says so.** Granting takes two steps (choose, then confirm on a line that names the city and the House and says `This cannot be undone.`). No other act of this system needs a confirmation except releasing a House and adopting a direct-rule civic.
5. **Nothing blocks the turn.** No new gate on End turn. What needs attention is an alert and a line at the top of the Court screen.
6. **Nothing empty.** Under direct rule, or before the first person, the Court screen says in one or two bare lines why there is nothing (`Government: Despotism. Every city is the Crown's.`), not empty tables. The Court button exists from the start but carries a count badge only when something needs attention (an heir to name, a Disloyal holder, a demand with 10 turns or fewer).
7. **Readable on a phone.** Cards in one column, not wide tables; every button at least the size the other screens use; the grant list scrolls inside the dialog.
8. **Bare text.** Labels, numbers, the rule. No adjectives, no narration. A person's backstory (the leader's civilopedia text) may be shown as flavour in the person's opened card, clearly apart from the rules lines.
9. **Portraits.** A person with a leader shows that leader's portrait as the diplomacy screen does (app/portraits.js); a hero shows its unit's sprite or icon as the unit panel does. The same person always shows the same picture and name, with the title (`The Auditor of Oaths`) under the name where there is room.

### The parts

#### A. The Court screen (menu entry "Court"; a free function key, shown in the key list)
Top to bottom:
1. **Header**: `Government: Aristocracy.` and, under a governing civic, `A House takes 1.5 orders a city every 30 turns.` An (i) opens the help topic.
2. **Needs attention** (only when there is something; each line has its button): `House Arne has no head. [Name an heir]` · `House Ashledger: Disloyal. Will secede with Prespur, Elam and Torrin in 8 turns. [Show]` · `Hrolf, 2nd Army: Disloyal. Will seize Vallus in 6 turns. [Show]` · `Maereth Ashledger has no post. [Grant a city]` · `3 cities of the Crown can be granted. [Show]`.
3. **Houses**, the Crown first. A House's card: its colour as a small shield, `House Ashledger`, the state mark, the head (portrait, name, title), `Seat: Prespur`, its cities as chips (a tap opens the city), `Orders: 2 of 3 left. Next in 12 turns.` (Devoted: `Orders: no limit.` Restless: `Takes no orders.`), `Trait in its cities: Philosophical` (the trait's own help as the tooltip, with a note of which parts apply to a city), `Maintenance saved: 9 gold a turn.`, the demand line (below), and `[Release as a vassal]`. Opening the card shows the loyalty parts (`Demand refused: -3 (41 turns)`), one per line, with the total, and the record.
   The Crown's card: `The Crown`, its cities; a city that can be granted has `[Grant]`.
4. **Persons without a House**: commanders and persons with no post, as cards: portrait, name, title, `Veteran: level 7 Axeman, 2nd Army` with a ⌖ to show the unit, `Commander of the 2nd Army` or `No post`, the state mark, the demand line, and opened: loyalty parts, ambition, the trait a House of theirs would give its cities, the record.
5. A person who left this game is not listed.

**The demand line**: `Demands: Torrin granted to House Ashledger. 34 turns.` with one button by kind: a post / a House → `[Grant a city]` (opens the grant list of the Crown's nearest grantable city; when the Crown has none: no button, the line adds `No city of the Crown can be granted.`) · a city granted → `[Grant Torrin]` (the grant list of that city, that House first) · a city taken → `[Show]` (centres the map; the line adds `Owner: Sheaim. At peace.` when it needs a war) · war → `[Diplomacy]` (that civilization) · a state religion adopted → `[Religion]` · the state religion in a city → `[Show]` · a wonder → `[Order]` (the empire order's city step for that wonder with the seat chosen) or `In production: 12 turns. The term does not run.` With 10 turns or fewer the turns are in the amber of Restless. Between demands: `No demand. Next in 14 turns.`

#### B. The grant list (a dialog for one city)
- Title `Grant Torrin`. Under it: `A grant is for good.` and the baseline `Held by the Crown: city-count maintenance 6.2 gold a turn.`
- Candidates as cards, claimants first (chip `Claimant`, a keyword), then other Houses, then persons without a post (`Found a House: Maereth Ashledger`), then great person units (`Found a House: Great Merchant in Prespur. A person of note is drawn. The Great Merchant is consumed.`).
- Each card lists, bare, everything the grant does: `Maintenance: -3.1 gold a turn.` · `Trait in Torrin: Philosophical.` · `House Ashledger: +1 (30 turns).` · `Meets its demand: +3 (60 turns). Becomes Devoted.` · every snub: `House Arne (claimant): -1 (30 turns). Becomes Restless.` · `Hrolf, 2nd Army (took Torrin): -2 (30 turns).` · `Orders in Torrin: by House Ashledger's quota (2 of 3 left).`
- `[Grant to House Ashledger]` replaces the card's foot with `Grant Torrin to House Ashledger? This cannot be undone. [Grant] [Back]`.
- A city that cannot be granted does not open the list; where its `[Grant]` would be, the reason stands (`The capital is the Crown's.`, `Government: direct rule. No city can be granted.`).
- After a grant: the dialog closes, a toast `Torrin granted to House Ashledger.`, the Court and the map refresh.

#### C. The city screen, Governance section
`Held by the Crown. [Grant]` or `House Ashledger. Head: Maereth Ashledger. Loyal ●. Trait here: Philosophical. Orders: 2 of 3 left.` A tap on the House opens the Court at its card. Where the player places an empire order and the House takes none, the entry is disabled with the rules' reason as its text (`House Arne is Restless. It takes no orders.`).

#### D. Armies screen and the army's panel
Each army: `Commander: Hrolf (level 7 Axeman). Loyal ●.` with the portrait small, or `No commander.` With more than one person with the army: `[Change]` lists them. Restless: `Takes no order to attack.` Disloyal: the outcome and the turns. In *Requisition from*, a House's city shows `House Arne: 1 of 3 orders left.` or the reason it gives none.

#### E. Lists
The Governors screen's and the cities list's city rows gain the House (its shield and name, or `Crown`).

#### F. The unit panel
A person's unit: its name is the person's; one line `Person of note: commander of the 2nd Army. Loyal ●.` (a tap opens the Court at the person). A great person unit while a city can be granted: an action `Found a House` (opens the grant list of the Crown's grantable city nearest the unit, this unit first). A Disloyal head's unit, when ordered: the rules' refusal.

#### G. The map (HD-2D first; the plates are drawn on the 2D canvas in app/r3d/hud.js; then the isometric and flat maps the same way where they draw plates)
On the player's own cities' name plates: the House's shield in its colour at the plate's left (none for the Crown). Restless: the amber ◆ on the shield. Disloyal: the red ✖ and the turns left as a number. No shader or mesh change.

#### H. Alerts (toasts from the log's person tag; a tap opens the Court at the subject)
With the portrait: a person enters · a demand is issued · 10 turns left on a demand · met · refused · a change of state (Restless, Disloyal with the stated outcome; back to Loyal) · an heir is needed · a succession · a House founded or ended · a secession or a league (to everyone who met the owner).

#### I. Previews where the acts are done
- Civics screen: choosing a direct-rule civic while Houses stand shows, in the warning style, `Adopting God King: 3 Houses secede with 9 cities: Prespur, Elam, … Held by armies: Torrin.` and the revolution's button reads `Adopt and face the secession`.
- Religion screen, declaring war, making peace (the diplomacy dialogs), the raze choice: the preview lines for that act under the existing text, each `Subject: text. Becomes State.` Nothing when there are none.

#### J. Help and keywords
A help topic `Persons of Note` (the rules in short bullets, as the Governors topic does). Keywords with reminders (data/json/keywords.jsonc, data/text/en.tsv): House, Crown, Devoted, Loyal, Restless, Disloyal, Sworn, Claimant. Reminders are bare and short (`Restless: a House takes no new order; a commander's army takes no order to attack.`). The texts of this system mark the keywords and never restate a reminder (tests/keywords.rs).

#### K. A staged scene: `ffh-play stage persons DIR`
One save that shows every state, for the tests and the screenshots: the player under Aristocracy with a capital, three Crown cities (one with two claimant Houses and a commander who took it), House A (Devoted, three cities, a wonder demand in production), House B (Restless after a refused demand, two cities), House C (Disloyal, countdown at 8, two cities, one held by an army), an empty seat with two candidates, a person without a post, a great person unit, a commander (Loyal) and a commander (Disloyal, countdown), a hero. A second, `stage persons-direct DIR`: the same empire one click from adopting God King.

## Open points for the owner

**Proposed on 2026-11-18 to meet the two goals, not yet confirmed.** The owner's decisions so
far are: the three sources of the cast; authored, recurring characters from the minor leaders;
Houses of several cities; no per-city governors or traits; grants for good, heads not replaced,
heirs from the cast; a head's first trait the owner lacks applying in their House's cities; the capital the Crown's; direct rule as civil war; leverage; the vassal
levy by requisition; wants as acts on the board; no human vassals. The rest is proposal:
- **Demands with a term**, for holders too, renewing: without them a holder's unmet want cost
  nothing and a careful player never saw a secession.
- **A House's city is the House's**: its role by the head's ambition, the Crown's orders by a
  quota, none while Restless, no limit while Devoted. Without it a loyal House was a name and a
  maintenance discount, and there was nothing to administer.
- **Restless withholds** (a House its orders, a commander its attacks): the register between
  content and traitor.
- **Tempers were dropped** (the owner, 2026-11-18), after four forms of them: one authored
  rule a person was no better than the leader traits the minor leaders already have. A head's
  first trait the owner lacks applies in their House's cities instead (decided; "A head's
  trait"). Proposed within it: which of a trait's effects a city can carry, as listed there.
- **Claims at the grant**, and the commander's claim on what the army took.
- **Leagues**, and rivals never seceding together.
- **A commander's worth by the unit's level**, in place of any authored skill.
- **The record and the annals.**

Still open beyond those:
- **A trait with nothing a city can carry** (Agnostic, Barbarian): here the person's House has
  no trait. The other reading: the next trait in their order.
- **Whether a Restless head's trait is withheld**, as its orders are. It would make crossing a
  good head cost what made them good; here the trait applies whatever the state.
- **Who names the heir.** Here the player. The other reading: the House takes one by a rule.
- **A head who never fights never dies** (a Great Merchant). The House then changes hands only
  by secession or release.
- **A House setting its own cultural channel** by its head's ambition (a Faith head spreading
  its religion, a War head fomenting revolt in the civilization it wants war with), forbidden
  at a grievance. It would give Houses acts of their own on the map; channels are the player's
  by the decision of 2026-10-30.
- **A person holding both** a House and a command: here one post each.
- **Leaving direct rule**: every city is the Crown's and those beyond its size count double
  until granted; with few persons free to found Houses that is a steep first step.
- **Calling up a vassal's standing units** (not only its production) at a grievance.
- **Quotas and Crown sizes by government**, and the numbers throughout.
- **Whether a levied unit is the master's for good** or returns when the war ends.
