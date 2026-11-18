# Persons of Note

This game's own rules, not FFH2's. **A design: nothing here is built.** A civilization has a small
cast of named, authored characters (its heroes, and the minor leaders of docs/minor-leaders who
join it as its best veterans and great people). Its cities are held by Houses, each presided over
by one of them; its armies are commanded by them; its vassals are ruled by them. Each makes
demands, gives less when crossed, and can take what they hold. The owner's concept and decisions
of 2026-11-18; the numbers are first guesses. "Open points" says which rules are the owner's
decisions and which are proposals still to be confirmed.

| Stage | What | Status |
|---|---|---|
| 0 | the vassal levy: an army requisitions from its owner's vassals' cities, by a quota | not built |
| 1 | the cast, the Court screen, Houses and grants, the commander's post, demands and loyalty shown; no consequences. The per-city named governors and their traits go | not built |
| 2 | consequences: quotas and withholding, secession, leagues, the civil war of direct rule, heirs | not built |
| 3 | vassals: a House released as a vassal, a capitulated leader as a person, claims abroad | not built |
| 4 | foreign persons: seeing them, courting them, agents abroad | not built |
| 5 | the annals: what each character did in the player's earlier games | not built |

Stage 0 needs no persons and can be built first. Stage 1 alone shows whether the cast is worth
looking at before anything can go wrong.

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
  nothing: only a House's head runs cities. A hero has no leader traits.
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

- *What a commander is worth* in battle rises with the commander's unit's level (the commander
  auras of docs/ENGAGEMENTS.md, phase 7, decided there). The capable commander is the one who
  has fought most, and so the one with the most claims.
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

- **The Court screen**: one row a person: portrait, name and title, post (House with its
  cities, or army), state, demand with its turns, rival. Opening a row shows the loyalty
  modifiers, the ambition, the trait their House's cities have, and the record. No other screen is new.
- **Where the state is seen at a glance**: each city's name plate carries its House's colour;
  a Restless or Disloyal holder marks its cities' plates and its army's banner, with the
  countdown.
- **Previews at the decision**: the grant list, the civics screen, the war and peace dialogs,
  the order list and the Armies screen's *Requisition from* list state what the choice does to
  each holder before it is made: `Meets House Arne's demand.` `House Ashledger: becomes
  Restless.` `House Arne: 1 of 3 orders left.`
- **Text** is bare (CLAUDE.md, "Rules text and tooltips"). House, Crown, Devoted, Loyal,
  Restless, Disloyal, Sworn and Claimant are keywords
  (`data/json/keywords.jsonc`).

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
- **A commander brings no trait.** Their worth is their unit's level alone.
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
