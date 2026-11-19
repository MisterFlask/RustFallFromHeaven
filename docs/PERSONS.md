# Persons of Note

This game's own rules, not FFH2's. **"The second build" (Houses as vassal civilizations, rules 1
to 14) is built: its rules, its AI and its screens. "The first build" is what it replaced where
they differ; the rest is a design.** A civilization has a small
cast of named, authored characters (its heroes, and the minor leaders of docs/minor-leaders who
join it as its best veterans and great people). Its cities are held by Houses, each presided over
by one of them; its armies are commanded by them; its vassals are ruled by them. Each makes
demands, gives less when crossed, and can take what they hold. The owner's concept and decisions
of 2026-11-18; the numbers are first guesses. "Open points" says which rules are the owner's
decisions and which are proposals still to be confirmed.

| Stage | What | Status |
|---|---|---|
| 0 | the vassal levy: an army requisitions in its owner's Houses' cities | built (second build, rule 6; no quota: an Annoyed House takes none) |
| 1 | the cast, commanders, demands and loyalty; the Court screen and the grant list; persons in the city, Armies and unit screens, on the map's plates, in the alerts and in the previews of civics, religion, war, peace and razing. The per-city named governors and their traits are gone | built (first build; rules `crates/ffh-core/src/persons/`, tests `crates/ffh-core/tests/persons.rs`; client `crates/ffh-play/src/view/persons.rs`, `view/house_marks.rs`, `assets/app/persons*.js`; AI `crates/ffh-ai/src/persons.rs`). Houses reworked by the second build |
| 2 | consequences: a commander's withheld attack and secession, the civil war of direct rule | built; the first build's order quota, a head's trait in the cities, half maintenance, heirs and leagues retired by the second build |
| 3 | vassals: every vassal a House, its leader a person of the liege's cast, tribute, breaking away, freeing | built (second build, `persons/vassals.rs`, `persons/secession.rs`); claims abroad not built |
| 4 | foreign persons: side deals with a rival's Houses (Secede, Become your vassal); seeing persons, courting, agents abroad | side deals built (second build, rule 9, `persons/bribes.rs`); the rest not built |
| 5 | the annals: what each character did in the player's earlier games | not built |
| second build | Houses as vassal civilizations, rules 1 to 14 below | built (2026-11-18): the rules (ffh-core), the AI (rule 12, `crates/ffh-ai/src/persons.rs`) and the screens (rule 13: the Court, the grant list, the levy, diplomacy, the map's plates) |
| third build | Houses without Persons of Note: the first grant founds, splits, ultimatums and contracts, courtship, the world unit, shared knowledge, gold tribute (rules 1 to 12 below) | being built (2026-11-19): part 1 the persons removed (rules 1 to 3, 9); part 2 splits and ultimatums (4 to 6) with courtship, price, knowledge and tribute (7 to 10) |

**"The third build" (2026-11-19, below) is being built: it removes Persons of Note and leaves the
Houses alone. Where it differs from the second build, it holds; everything below it on persons
(the cast, commanders, posts, persons' demands) is retired by it.**

"The second build" below is the contract in force, with its "as built" notes; "The first build"
is what was built before it, in force where the second build does not change it (the cast,
commanders, demands, the civil war); "UI" is the interface as first built. The sections between
them ("Rules") are the design the builds came from and describe the stages not built yet.

## The third build (2026-11-19): Houses without Persons of Note

The owner's decisions of 2026-11-19. Persons of Note did no work the Houses did not ("the work is
all being done by the Great Houses"): they go, and Houses stand alone. The player never founds a
House: a realm's first grant makes one, and Houses split on their own as they grow. A House that
becomes Furious no longer counts down: it issues an ultimatum, and its liege chooses between
meeting it and a civil war ("a player decision to appease a House vs have a civil war"). A rival
who wants to buy a House courts it first with gifts, in the open ("we want to force telegraphing
this"). Knowledge is shared within a realm, and tribute is gold. **This section is the contract
for the third build; where it differs from the second build, it holds.**

1. **No persons.** The cast and every person go: posts, commanders (an army has no commander: no
   withheld attack, no commander's secession, no sworn units, no commander's claims), heroes and
   veterans as persons, the veteran bar (`PERSON_VETERAN_LEVEL`), great persons founding Houses,
   persons without a post, rivals, persons' records. Heroes and great people are as in FFH2. A
   House carries what its head carried: its leader (the House player's leader: portrait, name,
   title, and the ambition its first trait gives), its request (rule 2), its record. The size part
   of loyalty (second build rule 14 (e), `Holds N cities, more than T`) goes, with its defines.
2. **Requests and ultimatums.** A House's slow wish, one at a time with a term (the second build's
   demands: met `+3 (60 turns)`, lapsed `-3 (60 turns)`), is a **request** in every text the
   player reads; an **ultimatum** is rule 5's, and each is a keyword. Request kinds: a city
   granted, a city taken, war with a civilization, a state religion adopted, a state religion
   spread (a post and a House of one's own go with the persons).
3. **The first grant makes a House.** A realm without a House founds one by granting a Crown city
   (never the capital; not under direct rule): a new player, the liege's vassal, born with the
   city (`world::birth`, as a founding was), named `House <name>`, its leader drawn with the game
   RNG from (a) the minor leaders of the liege's civilization that lead no living player, else
   (b) the minor leaders of other civilizations of the liege's alignment that lead no living
   player; when none is free, no House can be founded and the grant list says so (`No leader is
   free to found a House.`). A realm with a House grants to a House it picks (the city moves to
   that House's player). The garrison, the liege's army units and the city itself as in the
   second build's rule 3. The grant list shows a founding as `A new House: its leader is drawn
   from the Bannor's minor leaders.`
4. **Houses split.** In its liege's turn processing, each House holding 6 or more great cities
   (settlements left out; the define `PERSON_HOUSE_SPLIT_CITIES`) splits with a chance of 5%
   (`PERSON_HOUSE_SPLIT_PERCENT`: one draw `soren_rand_num(100) < 5` per such House a turn, in id
   order). The parent keeps its capital and the half of its cities nearest it (half rounded up,
   by plot distance from its capital, ties to the lower city id); the others go to a **cadet
   House**: a new player born with them as rule 3's founding, of the parent's civilization, its
   leader drawn as in rule 3, the liege's vassal. Units go with their cities as in a grant; units
   elsewhere stay the parent's. **The cadet inherits nothing:** no modifiers, no request, no
   contract, no memory, no ultimatum count; its tribute is its realm's default, in force. When no
   leader is free, the House does not split that turn. The liege is told: `House Aldric splits:
   House Bertram, led by Tovin Cinderwright, takes Ventry, Corona and Astrum.` The House card says
   `Holds 7 cities: 5% a turn to split.`; a grant that brings a House to 6 says `House Aldric will
   hold 6 cities: 5% a turn to split.`
5. **Ultimatums replace the countdown.** In its liege's turn processing, a House that is Furious
   and has no contract running (rule 6) issues an ultimatum: one demand, which its liege answers
   at once, through its controller (the AI) or as a blocking decision (a human: the decision idiom
   of an offered deal, `Decision`). Accepting does what the demand says and starts a contract
   (rule 6). Refusing is breaking away now (the second build's rule 7, its hold contest included).
   The countdown, its cancellation and its texts go. The demand comes from the House's grievances
   and is only one its liege can meet this turn: the first of these that applies.
   - (a) **Its refused request again**, while a `Request refused` modifier runs and the request
     can be met now: the city granted (a Crown city its liege can grant), war on that civilization
     (as (d)), that state religion adopted (its liege can adopt it now). Bound: for war, as (d); for
     a religion, no change of state religion.
   - (b) **Lower tribute**, when its rate set is 20% or more: the rate set lowered to the highest
     rate whose loyalty once in force is -4 or more, else 0%. Bound: no rate set above it.
   - (c) **Peace** with a civilization its liege is at war with that the House remembers taking
     its cities (its own `MEMORY_TOOK_CITY` toward it above 0), when that civilization would accept
     peace now (its controller is asked). Bound: no war on it.
   - (d) **War** on a civilization the House remembers a wrong from (its own `MEMORY_DECLARED_WAR`
     or `MEMORY_TOOK_CITY` toward it above 0), at peace with its liege, outside its realm, that its
     liege can declare war on now. Bound: no peace with it.
   - (e) **A resource** its liege has a spare source of and the House has none of, given for the
     contract's turns. Bound: the trade is not cancelled.
   - (f) **Gold per turn**, always possible: per House city, a share of the world unit (rule 9)
     set so it is about 2 gold a city a turn around turn 150, rising by half of that for each
     earlier ultimatum of this House; at least 1 a city; never more than half its liege's total
     commerce a turn. Paid each turn of the contract from the liege's treasury to the House's, as
     an expense in the liege's budget (an empty treasury has the usual consequences). Bound: it
     runs its turns.

   No levy demand: a House under contract is at least Annoyed, which takes no levy already, and a
   levy's grievance runs 20 turns, as a contract does. The decision says what accepting binds
   (`For 20 turns: no tribute above 10% for House Aldric.`) and what refusing does (the
   break-away's outcome, the held cities included).
6. **The contract.** An accepted ultimatum binds both sides for 20 turns
   (`PERSON_ULTIMATUM_TURNS`): its liege cannot do what would void it (the act is refused with the
   reason: `Your contract with House Aldric forbids it until turn 160.`), and the House's loyalty
   counts as at least Annoyed (-5) for every rule (its state, its price, its levy, its next
   ultimatum). The House card shows `Contract until turn 160: no tribute above 10%.` When it ends
   the House's loyalty is what its parts make it; Furious again, it issues a new ultimatum.
7. **Courtship.** A civilization that has met a House (not of its liege's team, not a House
   itself) courts it with gifts in the trade table: gold (a lump sum) or a resource (a trade it
   keeps giving), nothing asked back. Its courtship's value: the gold it gave that House in the
   last 20 turns, plus a third of the requirement for each resource it gives it now. The
   requirement: 3 turns of the world unit (rule 9). The courtship **opens** the turn its value
   reaches the requirement and stays open while the value does. A House accepts an offer (Secede,
   Become your vassal) only from a civilization whose courtship has been open 10 turns or more
   (`PERSON_COURTSHIP_TURNS`), besides the second build's conditions (Cautious or worse, the price
   covered). When a courtship opens, its liege is told (`The Bannor court House Aldric: offers
   possible from turn 140.`) and remembers it (`MEMORY_COURTED_HOUSE`, "courted our House": -200
   on FfH's scale, the AI's relation -1, fading at 1 in 50; the House, given gifts, is not
   angered). The House card and the diplomacy list show `Courted by the Bannor (gifts: 120 gold,
   Wine). Offers possible from turn 140.`; the courter's trade table shows `Courting House
   Aldric: 140 of 180 given (Wine counts 60).`
8. **The price scales with the economy.** A House's price is `(loyalty + 6) x P x its cities`
   gold, P a share of the world unit (rule 9) set so that P is 25 around turn 150 (the second
   build's flat 25), at least one step (`P x its cities`).
9. **The world unit:** the mean total commerce a turn (every commerce type) of the living
   civilizations that are not Houses, barbarians, minor civilizations or crusade Hosts
   (`Game::world_commerce_unit`, at least 1). It scales what passes in gold between a realm and
   the world: the courtship, the price, an ultimatum's gold. Total commerce, not gold: the gold
   slider is a choice, and a civilization at full research shows little gold however rich.
10. **Knowledge is shared in a realm; tribute is gold.** A tech one team of a realm (a liege's
    team and its Houses' teams) gains by any means, every other team of the realm gains too (not
    as its first discoverer); a team that joins a realm and the realm share what they know at
    once. Tribute takes gold only: the second build's research part goes (once knowledge is shared
    it counted a House's beakers twice). Techs, like war, are the liege's alone to trade: a House
    trades none.
11. **AI.** As liege: it accepts an ultimatum unless every city of the House would be held
    (refusing then takes nothing) or the demand is war on a civilization stronger than it
    (power); its grants of the Crown's cities over the limit go to the House whose capital is
    nearest (the first one founds a House); its tribute rule stays (the second build's rule 12);
    a House of its that a rival courts and that is Cautious or worse has an idle army within reach
    march to hold its most valuable city (the second build's march, its trigger moved). As buyer:
    it courts with gold the House it means to buy (the second build's choice of House), then
    offers when the courtship allows.
12. **What stays:** every rule of the second build not named here (a House is a player, every
    vassal is a House, the tribute's lag and the realm's default, the attitude scale with Furious
    now the moment of an ultimatum, the levy, breaking away and its hold contest, freeing, side
    deals with rule 7's courtship, direct rule, the Crown's limit and settlements of rule 14 (a) to
    (d), the memory of wrongs). The numbers are first guesses.

## The second build (2026-11-18): Houses as vassal civilizations

The owner's decisions of 2026-11-18, after the first build: **a House is a civilization of its
own, a vassal of its liege**, which gives the liege part of its income and can be requisitioned
from; a slider (the tribute) moves its attitude toward the Crown; **any break-away a House
starts is a declaration of war**, whether from its own loyalty or from a bribe; capitulation and
vassalage happen at the level of the House; **side deals with Houses are ordinary diplomacy**,
while war and peace go through the Crown; **heads leave the map** and lead their House (heirs
go). The aim (the owner): intuitive, not cumbersome, reusing the game's existing idioms
(vassalage, attitude, the diplomacy screen, requisition, war).

**This section is the contract for the second build; where it and "The first build" differ, this
section holds.** What it does not mention of the first build stays as built (the cast and how
persons enter, commanders and their secession, demands of commanders and of persons without a
post, the Court screen, the previews, claims).

1. **A House is a player.** `House { id, owner (the liege player), player, name, head, standing,
   founded_turn, color, tribute }`; the House player has `Player::house: Some(id)`. Its team is a
   vassal of the liege's team. It is played by the game's AI, never by the liege's controller.
   Its leader is its head's leader; its civilization is the liege's (a founded House) or its own
   (a civilization that became a vassal). A founded House is named `House <name>`; a vassal
   civilization keeps its name. Its colour is its liege's (the realm reads as one on the map; the
   shield on a city's plate tells the Houses apart). It has met everyone its liege has met.
   `city_house(c)` is the House of the city's owner.
2. **Every vassal is a House.** When a team becomes a vassal by any means (capitulation, a vassal
   deal, a card's vassal birth, a founding), each of its players gets a House record owned by the
   master team's leading player, headed by a person for its leader (origin `Ruler`, no unit).
   Starting modifier: `Capitulated: -4 (100 turns)` for a capitulation, `Sought protection: +2
   (60 turns)` for a vassal deal, none for a founding or a card. When a vassalage ends by any
   means, its House record ends and its head leaves the cast.
3. **Founding and grants.** Granting a Crown city (never the capital; not under direct rule) to a
   person with a leader (a veteran, or a great person, who draws one and is consumed) founds a
   House: a new player (`world::birth` with `release: [city]`, the liege's vassal). The person
   leaves the map (their unit is removed) and is the House's leader. A hero cannot found a House
   (it has no leader; heroes command). Granting a Crown city to a House moves the city to that
   House's player. Either way the city keeps its buildings and culture, its garrison (the land
   combat units in it in no army and not marching) goes with it, and the liege's army units in it
   stay the liege's (a vassal's land is friendly to its master). Claims and snubs stay as built.
   **No heirs:** a head is a leader and does not die.
4. **Tribute.** `House::tribute` is the rate the liege sets, one of 0, 10, 20, 30, 40, 50 (%); a
   new House takes its liege's realm default (`Player::house_tribute`, 20). **The rate in force
   lags it, both ways** (the owner's decision of 2026-11-19: "reputations and relationships are
   sticky and slow moving"; it replaces the raise's modifier of 2026-11-18, `Tribute raised: -k
   (30 turns)` with cuts free, which made the rate a ratchet: start high, cut once, never raise):
   `House::tribute_now` moves one step of 10% toward `House::tribute` every 5 turns
   (`TRIBUTE_LAG_TURNS`), only while the two differ, counted from its last step or from the
   setting of the rate; 0% to 50% takes 25 turns, and so does 50% back to 0%. In the liege's turn
   processing, for each of its Houses in id order: gold = the rate in force% of the House's gold
   commerce (the sum of its cities' gold commerce), taken from the House's treasury (no more than
   it holds) and given to the liege; research = the rate in force% of the House's research
   commerce, added to the liege team's current research (not taken from the House: knowledge is
   shared; lost when the liege researches nothing). Recorded per House for display: `Tribute 30%:
   12 gold, 8 research last turn.`, and while the rate in force moves `Tribute 30%, now 10%:
   reaches 30% in 10 turns. Paid last turn: 4 gold, 6 research.` A standing loyalty part, as
   `State religion opposed` is: `Tribute N%: 1 - N/10` with N the rate in force (0%: +1, 10%: 0,
   20%: -1, ..., 50%: -4). One slowly moving rate drives both: lagging only the loyalty part would
   let a liege collect a raised rate before its cost arrived. A new House starts with the default
   in force (a founding, a capitulation, a vassal deal, a card, a bought House: nothing to lag);
   setting the default changes no House that stands.
5. **The attitude scale replaces the four states.** Loyalty total: **Friendly** 3 or more,
   **Pleased** 1 to 2, **Cautious** -2 to 0, **Annoyed** -5 to -3, **Furious** -6 or less. A
   House's AI attitude toward its liege is this (`player_ai_get_attitude`). Commanders and persons
   without a post use the same scale. Effects: a House Annoyed or worse takes no levy; a House
   Furious starts the stated countdown of 10 turns to breaking away (rule 7). A commander Annoyed
   or worse: the army takes no order to attack; Furious: the countdown as built. A person without
   a post Furious leaves the cast.
6. **The levy (stage 0).** An army's *Requisition from* list accepts the cities of its owner's
   Houses (every vassal). The order is placed in the House city's queue as in the liege's own
   cities; the unit trained is the liege's (created for the army's owner) and marches to the
   army. Each order placed is a House grievance `Levied: -1 (20 turns)`. A House Annoyed or worse
   takes no levy: the requisition passes its cities over, and the screen says why.
7. **Breaking away is war.** At the end of a Furious House's countdown, when its liege comes under
   a direct-rule civic (every House at once: the civil war), or when the House accepts a bribe to
   secede (rule 9): first every city of the House in which the liege's army units outnumber the
   House's own land combat units goes back to the liege (**holding a city is a contest**, the
   owner's decision of the same day, after a one-unit army in each House city made every
   break-away take nothing: the liege's units are the land units of its armies standing in the
   city whose commander is not Furious, the House's are the land combat units that would go with
   the city, as a commander's seizure counts them; a tie goes to the House; `Corona returns to
   you: your army's 4 units outnumber its 2.`, and where an army of the liege stands but does not
   outnumber, `Corona goes with it: its 3 units against your army's 2.`); then the House's team leaves the
   vassalage and declares war on the liege's team. Its House record ends; its head leads the
   civilization. Everyone who has met the liege is told. Civ4's own vassal freedom rule
   (`team_can_vassal_revolt`: free when master or vassal lost land) no longer frees a House, and
   the AI never cancels a House's vassalage on its own: the countdown and bribes are the only ways
   out by the House's will. Houses breaking away the same turn are not merged (leagues go); being
   at war with the same liege, they ally by ordinary diplomacy.
8. **Freeing a House** (the liege's act; was "release as a vassal"): the vassalage ends at peace;
   the House record ends.
9. **Side deals.** A House is a party on the diplomacy screen for anyone who has met it. War,
   peace, open borders, pacts, techs and vassalage are its liege's to trade; the House trades only
   this: it takes gold, a lump sum only (the owner's decision of 2026-11-18: gold per turn could
   be paid for one turn and ended by a war on the seceded House), and gives **Secede** (it breaks
   away at once, rule 7) or **Become your vassal** (it breaks away and becomes the buyer's House; by the vassal rules
   the buyer then shares its war with the old liege). It accepts when its loyalty is Cautious or
   worse and the offer covers its price: `(loyalty + 6) x 25 x its cities` gold (one step, `25 x its cities`, at least: a Furious House
   is never free, its countdown being its liege's window to answer); refusals in the screen's
   words (`House Arne is Pleased with the Bannor.`). The liege learns of a bribe: it remembers the
   buyer (`MEMORY_BOUGHT_HOUSE`, rule 15: -300 on FfH's scale, the AI's relation -1, fading; the
   House was paid and is not angered); the House's loyalty list shows `Offered 300 gold by the
   Hippus` and a human liege gets an alert. **Only an accepted offer is noted** (the owner's decision of the
   same day): a refused offer leaves no trace, since the trade table already says in advance
   whether an offer will be accepted, so the player never risks a refusal. The price is one rules
   function every controller reads. Threats are not in this build.
10. **Retired:** the order quota, the half maintenance (a grant lowers the liege's city count, and
    its city-count maintenance with it), the head's trait in a House's cities (the House's leader
    has its traits), heirs and empty seats, leagues, the Works demand of a head (a head with
    Works ambition demands land), `house_orders` in civics and `PERSON_HOUSE_MAINTENANCE_PERCENT`.
11. **Direct rule** stays as built: no Houses and no grants under it; adopting it is every House
    breaking away (rule 7), the counter on the map holding.
12. **AI.** As a liege (the owner's decisions of the same day, replacing "20% by default, 10% for
    a Cautious House, 0% for an Annoyed one", after an expert bought the AI's Cautious Houses at
    about 150 gold a city; revised for rule 4's lag on 2026-11-19): every House's rate set to the
    highest whose loyalty once in force is Pleased with a point to spare (2 or more), and lowered as
    soon as the projection at the rate set falls below that (a cut arrives a step every 5 turns, so
    the point to spare is what covers a grievance while it comes), so its Houses cannot be bought
    and a countdown a cut can stop in time is stopped; a levy only while the House's loyalty after
    it (`Levied -1`) stays Pleased or better; a House counting down that no cut to 0% brings back
    above Furious before its countdown ends has an idle army within reach march to hold its most
    valuable city, with enough units to
    outnumber the garrison (rule 7); grants of the Crown's cities over its limit (rule 14), the
    least populous first, the capital and the best production city kept, a House enlarged only up
    to its size threshold (rule 14). As a House: its own turn, trimmed for cost (no war planning,
    no diplomacy of its own beyond answering offers). As a buyer: at war with a civilization, it
    offers that civilization's Houses that are Cautious or worse **Become your vassal** when it can
    pay their price with up to half its treasury (it gets the cities, their units and a House, and
    the war is already on), else Secede with up to a third; gold only, one offer per House per 10
    turns.
13. **Interface.** The Court lists the realm: the Crown's cities, and each House as a card (its
    leader's portrait, attitude mark and parts, the tribute selector and what it gave last turn,
    whether it takes levies, the countdown, Free); the realm's default tribute. The grant list
    states what a grant does now (`Your city-count maintenance: -3.2 gold a turn.`, the tribute,
    the founding person leaving the map). The Armies screen lists the Houses' cities in
    *Requisition from* with their status. On the map a House's cities show its shield on their
    plates for everyone, and its attitude mark for the liege. The diplomacy screen lists a House
    under its liege (`House Arne, vassal of the Bannor`) and offers Secede and Become your vassal
    in its trade table.
14. **The Crown's limit and a House's size** (the owner's request of 2026-11-18: "the typical
    state should be the player managing 2-5 Houses depending on empire size"; revised by the
    owner the same day, replacing per-civic numbers). (a) Under every governing civic the Crown
    holds 3 cities at the normal rate (the define `PERSON_CROWN_CITIES`); under a direct-rule
    civic, no limit. The Crown's cities in the player's city order with the capital first: those
    beyond the limit pay triple city-count maintenance. The screens read the limit, the Crown's
    count, the cities over it and the gold a turn they cost beyond the normal rate (`The Crown
    holds 7 cities, 4 over its limit of 3: +8.4 gold a turn.`); a grant's maintenance line
    includes the saving of coming back under the limit. (b) Settlements (`City::settlement`, the
    Kuriotates') never count toward the Crown's limit and are never over it; they are granted
    like any city (granting one only moves borders). (c) The Kuriotates: the Crown's limit is the
    capital alone, as data: the trait key `crown_cities_change` (`TraitInfo`, default 0),
    `TRAIT_SPRAWLING` -2, added to the limit, never below 1. (d) The Sprawling city cap
    (`max_cities`: a city beyond it is a settlement) counts the realm: the liege's great cities
    and its Houses' together (settlements left out), so a Kuriotate House founding or receiving
    cities does not raise the realm's number of great cities; a city a Kuriotate House founds
    beyond the realm's cap is a settlement. A granted great city stays a great city (the count is
    unchanged by a grant). (e) A House's size: a House holding more great cities `N` (settlements
    left out) than its realm's threshold `T` has a standing loyalty part `Holds N cities, more
    than T: -(N-T)`, computed like `State religion opposed`, shown in its parts and counted in
    previews. The threshold scales with the realm (the owner's change of the same day): `T =
    max(3, 2 + realm cities / 8)` (integer division; the realm: the liege's cities and all its
    Houses', settlements left out), the defines `PERSON_HOUSE_SIZE_MIN` 3, `PERSON_HOUSE_SIZE_BASE`
    2 and `PERSON_HOUSE_SIZE_PER` 8: 3 up to 15 cities, 4 from 16, 5 from 24. A grant or a founding
    moves a city within the realm and leaves `T` as it is; a capture or a loss moves it. The
    numbers are first guesses.
15. **An act against a House angers the House and its liege** (the owner's rule, 2026-11-18),
    with the AI's memory of wrongs (the owner's decision of the same day): a civilization
    remembers who declared war on it (`MEMORY_DECLARED_WAR`), took its cities in battle
    (`MEMORY_TOOK_CITY`) and bought one of its Houses (`MEMORY_BOUGHT_HOUSE`), and the memories
    fade (each at 1 in 50 a turn). One rules function records a wrong (`player_add_memory`): when
    the wronged player is a House, its liege remembers it too. A war declared on a liege reaches
    its Houses by the vassal rules, and each player declared on remembers it once (the liege no
    more for its Houses). The AI weighs the three as terms of its attitude (`declared war on us`,
    `took our cities`, `bought our House`; docs/AI.md, "Relations"), and the screens say what an
    act will cost where it is done: the war question (`They remember it: attitude toward you -1
    (fades). So do House Arne.`) and a House's trade table (`The Svartalfar's attitude toward
    you -1 (fades).`).

### The second build as built (2026-11-18)

Every rule above is built in the rules (`crates/ffh-core/src/persons/`: `vassals.rs` the records,
tribute, levy and freeing; `houses.rs` grants; `secession.rs` breaking away; `bribes.rs` side deals;
`crown.rs` rule 14; `read.rs` and `preview.rs` the screens' API; tests
`crates/ffh-core/tests/persons.rs`). Where the build differs from the text above, or settles what
it left open:

1. *As built:* `House { id, owner, player, origin, name, head, standing, founded_turn, color,
   tribute, last_gold, last_research, last_turn }`; `Player::house` on the House's player,
   `Player::house_tribute` on the liege. `City::house` and `City::house_trait` are gone:
   `city_house(c)` is the House of the city's owner (`Player::house`, no scan). A player's colour
   is the client's (by player id; the setup has none): `player_color_player(p)` names the liege for
   a House, for the client to draw. A vassal civilization's label is its own description when it
   has one (`House Arne`, `Bannor of House Arne`), else its short description (`Hippus`).
2. *As built:* the hook is `team_vassal_houses`, called at the end of `team_set_vassal` both ways;
   the head is the person of the liege's cast with the vassal's leader (a founder), else a new one
   (origin `Ruler`, entered without a message: the liege is told `Hippus is your House: it
   capitulated.`). `HouseOrigin`: `Founded`, `Capitulated`, `Vassal` (a vassal deal, a card's
   birth, a colony; the births clear the starting modifier). A human vassal gets no House (out of
   scope). The record also ends when the House's player dies. A player that becomes a House loses
   its own cast; its own Houses are freed by the vassal rules (a vassal holds no vassals).
   **Vassals under direct rule** (a call the contract left open: rule 2 against rule 11's "no
   Houses under it"): a master under a direct-rule civic takes vassals as the Civ4 rules allow
   (capitulation, vassal deals, a card's vassal birth, a colony, the Oath of Fealty), and every one
   is a House; direct rule refuses grants and foundings only, and *adopting* it (a change of civics
   into one) is the civil war of rule 11. So a House can stand under a direct-rule liege, taken
   after the change. (Built first the other way, refusing every vassal to a direct-rule master;
   that stopped capitulation and vassalage under Despotism, where every civilization starts,
   against the AI's standing vassal rules, and every card's vassal birth.)
3. *As built:* a founding is `world::birth` with the city released (`Birth::keep_armies`: the
   owner's units in an army or marching stay its own), the founder's unit removed first; the
   vassal rules make the record, which the founding names (`House <name>`, origin `Founded`, no
   starting modifier). Grants are also refused to a vassal and under No Vassal States. Refusal for
   a hero: `A hero cannot head a House.`
4. *As built:* `player_set_house_tribute(p, h, pct)` sets the rate (`House::tribute`),
   `player_set_house_tribute_default(p, pct)` the realm's default (refusals `Not a House of
   yours.`, `Tribute: 0 to 50 percent, in steps of 10.`). The rate in force is
   `House::tribute_now`; `House::tribute_since` is the game turn its count runs from: the turn the
   rate in force took effect, or the turn a new rate was set while the rate in force was at rest.
   A rate set while the rate in force moves keeps the count (a call the rule left open: a change of
   mind neither costs nor gains time, and an AI whose rate changes with its House's loyalty does
   not stall the movement; it never moves faster than a step every 5 turns). Paid in
   `player_persons_turn` of the liege, after the break-aways, at the rate in force; right after
   the payment the rate in force steps when its count has run (`house_tribute_step`), in force
   from the next turn: so the rate the card shows in a turn is the one paid at its end and the one
   the states are told from, and `reaches 30% in 10 turns` means the card shows 30% in force ten
   turns on. The reads: `house_tribute_turns(h, pct)` (the turns until `pct` would be in force were
   it set now; 0 for the rate in force; for the rate set, until it is reached),
   `house_tribute_in_force_turn(h, pct)`, `house_tribute_in_force_after(h, pct, turns)` (the rate in
   force that many turns on, the rate set to `pct` now), `house_loyalty_at_tribute(h, pct)` (the
   loyalty once `pct` is in force: the steady state), `house_tribute_at(h, pct)` (what it pays at
   `pct` at its commerce now), `house_commerce(h)` the gold and research commerce it is taken
   from. `TributeView { pct, now, turns, last_gold, last_research, last_turn, at }`; `at` lists 0 to
   50 as `TributeRate { pct, gold, research, part, turns, loyalty, state }`, each once in force.
   `TRIBUTE_LAG_TURNS` (5) in `persons/mod.rs`; save format 52 (the raise's
   `LoyaltyCause::TributeRaised`, `TRIBUTE_RAISED` and `tribute_raise`, format 51, are gone).
5. *As built:* `LoyaltyState` keeps its name, with the five variants; `withholds()` is Annoyed or
   worse. The keywords Devoted, Loyal, Restless and Disloyal are gone: the states are the game's
   attitude words, no keywords (the client build; a state mark's tooltip is the attitude's own
   help in the data, `attitudes.jsonc`; "UI", part J). The define
   `PERSON_DISLOYAL_TURNS` keeps its name: the Furious countdown.
6. *As built:* an order already placed stays when its House turns Annoyed; only new ones are
   passed over. The AI levies only from a House that stays Pleased or better after the levy
   ("AI" below).
7. *As built:* `house_break_away`; the break-away and the liege take `GRUDGE` (-4) toward each
   other; the news is marked for the alerts (`LogMark::Person`, untagged: the client knows it by
   its wording, `breaks away from the ... : war.`). The vassal rules bring the liege's other Houses
   into the war (the House declares war on them too). A House whose every city is held ends
   (its player dies without a city). The contest: `city_hold(c)` (`CityHold { liege, armies,
   house }`, `held()` when the liege's units outnumber the House's), `unit_holds_for_liege(u, p)`,
   `city_held_by_army(c)`, `house_cities_held(h)`; its lines `city_hold_line(c, house)` and
   `house_hold_text(h)` (the outcome, the civics preview, the Court's cards and city chips). At
   the break-away the liege is told each held city (`Corona returns to you: your army's 4 units
   outnumber its 2.`) and each city contested and lost (`Corona goes with House Arne: its 3 units
   against your army's 2.`), counted before the cities move.
8. *As built:* `player_check_free_house`, `player_free_house`.
9. *As built:* the trade items `TRADE_SECEDE` and `TRADE_HOUSE_VASSAL` (`tools/gen_enums.py`
   `EXTRA_MEMBERS`); `player_can_trade_item` lets a House give only them and take only gold
   (without the gold-trading tech; no gold per turn since 2026-11-18, and ffh-ai's House refuses
   any other item), from a civilization that met it and is not its liege; a House can be contacted at war (`player_can_contact`). An offer made at war makes peace
   between the buyer and the House first (a deal at war makes peace), then the House breaks away.
   `player_offer_house(buyer, house_player, give, get) -> bool` is the one way to propose it (the
   client's `propose_deal` to a House and the AI use it): the House's controller answers
   (`consider_offer`; ffh-ai by `house_bribe_price`); an accepted offer is noted where the deal
   takes effect (`house_trade`, after `house_bribe_refusal` passes: `house_note_offer`), and a
   refused one, or one the rules refuse at its implementation, moves nothing (no memory, no
   `Offered` entry, no alert, no telemetry; built first noting every offer, changed by the
   owner's decision of 2026-11-18). The AI's `offered_lately` reads the `Offered` entry, which an
   accepted offer leaves only on a House that then breaks away. The liege's memory is
   `MEMORY_BOUGHT_HOUSE` toward the buyer (rule 15; built first as an attitude modifier,
   `BRIBE_GRUDGE`, -4 on FfH's scale, which did not fade); `Offered N gold by the X` stands in the
   House's parts for 20 turns, amount 0.
10. *As built:* all retired, with `house_orders`, `PERSON_HOUSE_MAINTENANCE_PERCENT` and
    `PERSON_LEVY_TURNS`. A first build's `DemandKind::Wonder` stays readable in saves; nothing
    makes one.
11. *As built:* `player_persons_civil_war` breaks every House away (each its own war), called by
    `player_set_civics` on a change from a governing civic into a direct-rule one; a vassal taken
    later under direct rule is a House and stays (note 2).
12. *As built (the AI build, 2026-11-18):* as the rule says, with rule 14 read into the grants:
    a grant only while the Crown is over its limit, and a House enlarged only up to its realm's
    size threshold; a House's own turn skips diplomacy, the war targets and the campaign. The
    tribute, the levy, the army holding a city and the buyer as the rule says since the owner's
    decisions of the same day (batch H). Details in "AI" below and docs/AI.md, "Persons of
    Note".
13. *As built (the client build, 2026-11-18):* as the rule says; the screens are described in
    "UI" (parts A to J and their "as built" notes) and docs/HUMAN.md, "Persons of Note in the
    screens".
14. *As built:* `player_crown_limit` (`PERSON_CROWN_CITIES` plus the player's traits'
    `crown_cities_change`, at least 1), `player_crown_cities`, `player_crown_over`,
    `player_crown_over_cost_times100` (the gold a turn beyond the normal rate, inflation
    included), on `CourtView` too; `house_size_part(h)`, `house_great_cities(h)`,
    `player_house_size_threshold(p)` / `house_size_threshold(h)` (on `CourtView` and `HouseView`
    too); `player_realm_great_cities(p)`. The
    Crown's cities also leave out outposts (the naval outposts, which pay no city-count
    maintenance and cannot be granted); there is no limit wherever no city can be granted (direct
    rule, a vassal, No Vassal States, no persons): a grant is the remedy. The realm's count
    (d) is used wherever the cap is read: a city founded or acquired (`city_init`), an outpost
    raised, the Promote Settlement spell's requirement. A city moving between a liege and its
    House (a grant, a founding, a held city back from a break-away) keeps its settlement status
    and radius (`city_keep_settlement`), where its new owner has a cap; a city captured is judged
    by the count, as before. The realm's count, a House's size and the threshold leave out the naval
    outposts with the settlements. The AI's site values (`found_value`) still read the player's
    own count.

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

**Known gaps of the first build** (2026-11-18):
- A Disloyal head's unit is marched to its House's largest city by the rules, but once there
  only the human's command layer refuses its orders (`unit_house_refusal`); the AI moves it as
  any unit and the rules march it back.
- The secession news other civilizations receive carries no log tag: the client recognises it
  by its wording (`ffh-play` `report.rs`, `is_secession_text`).
- For the AI, a commander's demand for a House is the demand most often refused: meeting it
  ends the command, and the Crown runs short of cities once the AI grants one a turn.
- A civilization has five minor leaders, so at most five persons who are not heroes in a game.
- A commander going over to a rival waits for courting (stage 4): a Disloyal commander joins a
  league formed that turn, seizes a city or resigns.
- Measured once (8 AI games of 300 turns, 7 civilizations, standard maps, the merged build):
  8 to 22 persons entered a game, 3 to 12 Houses were founded, 30 to 71 cities granted; one
  secession in the eight games, of 3 cities, after the AI stopped enlarging a House past the
  Crown (before: four secessions of 5 to 8 cities).

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
`Adopting God King: 3 Houses secede with 9 cities, as one civilization at war with us: ... Torrin and Elam stay: an army of ours stands in each.`

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
something reasonable with it.

**Built** (the second build's AI, 2026-11-18, `crates/ffh-ai/src/persons.rs`; docs/AI.md,
"Persons of Note"), for AI players only:
- *As a liege.* Tribute (revised 2026-11-19 for rule 4's lag): every House's rate set to the
  highest whose loyalty once in force is Pleased with a point to spare (2 or more,
  `TRIBUTE_KEEP_LOYALTY`), lowered as soon as the projection at the rate set falls below that, as
  far as needed (to 0% when no rate keeps it); set only when it changes. The rate in force follows
  a step every 5 turns, so a raise is collected step by step and a cut starts moving at once. The
  levy: the main army's requisition takes
  the cities of its Houses that stay Pleased or better after the levy beside its own. Grants, one a turn, only while
  the Crown is over its limit (rule 14), never the capital nor the best production city, the
  least populous first: a demand met first, then a
  claimant in the best state, then a person without a post founding a House, then a great person
  founding one (only when nobody else can take the city and the card logic does not keep it), then
  the House with the fewest cities, enlarged only up to its realm's size threshold; a grant that makes a holder Furious waits; a hero never founds a House.
- *As a House.* It answers Secede and Become your vassal by the rules' price alone. Its own turn
  skips what a vassal cannot do: diplomacy (peace, vassalage, trades, war planning), offers to
  humans, loans, the field army targets, the amphibious target and the campaign; it keeps its city
  management, its defence and its units' moves.
- *As a buyer.* At war with a civilization, an offer to one of its Houses that is Cautious or worse
  at its price, in gold: Become your vassal when that is at most half the treasury, else Secede
  when it is at most a third; the cheapest first, one offer a turn, the same House again after ten
  turns.
- *Elsewhere.* No direct-rule civic while a House stands (unless it rules directly already); an
  idle army goes to hold the most valuable city it can hold (its units with ours there
  outnumbering the House's) of a House counting down that a cut to 0% cannot stop in time
  (`house_beyond_tribute`: still Furious at the rate in force when its countdown ends); a Furious
  commander's army goes to the most garrisoned city; a demanded war weighs a tenth more in the war
  planning.
- *Retired with their rules:* heirs, the order quota's skip, the seat's wonder order.

**Not built:** freeing a House; commands by the AI (the
rules' default commander stands); threats.

**Measured** (8 AI games, seeds 1-8, 300 turns, 7 civilizations, standard size; the base is the
rules' build, whose AI granted any Crown city and took 20% from every House): no panic, no failed
check. Per game: Houses founded 2-8 (base 5-12), Houses at the end 2-6 (5-7), cities granted to
Houses 3-15 (15-31), levies 0-6 (0), tribute 16-170 gold and 135-1116 research (207-858 and
1609-5276), 2 offers in 8 games, both accepted, and no break-away by the countdown (base: 11). Most
Houses stand Cautious at 20% and pay 10% or nothing.

The design's first version (before the builds; superseded where the above differs):
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

To measure as each stage lands: break-aways per AI game and what caused them, the share of
demands met, the turns Houses spend Annoyed or worse (telemetry `person_*`: `entered`, `granted`,
`house`, `levy`, `tribute`, `bribe`, `demand`, `state`, `seceded`). An AI that loses a House every
game is an AI the human need not fight: if so, the AI's demands are tuned before the human's are.

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

**Built (2026-11-18):** parts A (the Court screen), B (the grant list) and J (help and keywords):
`crates/ffh-play/src/view/persons.rs` (queries `court` and `grant {city}`, the view's
`court_attention`, the checks of `grant_city`, `name_heir`, `set_commander`, `release_house`),
`assets/app/persons.js`, tests `crates/ffh-play/tests/persons.rs`, `tools/web-persons-smoke.mjs`;
the shapes in docs/HUMAN.md. Departures: the key is `Shift+F9` (F1 to F10 are taken and F11, F12
are the browser's); `<person> has no post. [Grant a city]` is not listed while an heir is to be
named (those persons are the candidates, and naming one is the first act); a House's `[Show]` in
the lines that need attention opens its card in the Court (its city chips open the cities), a
commander's centres nothing but opens their card too (the ⌖ on the card shows the unit); `Release
as a vassal` asks on the card itself, as a grant does; the Crown's maintenance baseline and a
grant's saving are shown at the city's lasting rate, with the turns it pays none said apart
(`Maintenance: -2.1 gold a turn (none while disorder lasts).`: a grant is for good, disorder
passes; the critic pass of 2026-11-18); the trait's tooltip lists
the parts of it a city carries (`Applies in a city: health, happiness.`), from the trait's data.
The rules have no check without the act for `player_name_heir` and `player_set_commander`: the
view reads the same conditions with the same words (`check_name_heir`, `check_set_commander`).

**The critic pass (2026-11-18).** A review of the screenshots changed: a commander's countdown
names what it costs (`Sera Vell will seize Vallus in 6 turns with 3 units. 1 stays in Torrin.`,
or the House secession it joins, or `will resign: no city to seize.`), from where the sworn units
stand now; a grant's preview states the orders by the grantee's state after the grant, and its
maintenance at the city's lasting rate (disorder said apart); the civics warning says the Houses
become one civilization at war with us, and which cities stay (`Censura stays: an army of ours
stands in it.`), its button `Adopt God King: 4 Houses secede`; the Disloyal reminder says the
same; Restless lines and an empty seat are amber, scarlet is Disloyal's alone; a House card's
demand comes right under its head; a demand's map button reads `On the map`; a grant preview's
`Becomes` uses the one state mark, the grantee's own lines join its card, and a snub reads
`Becomes the rival of …`; a paused term reads `41 turns (paused).`; a state toast drops its first
sentence under the mark; a hero's name is not repeated as its unit type; the keyword chips close
the Court and the grant list instead of opening them; a Disloyal plate's pill sits under the
shield's point; the modal's (i) stays round on a phone.

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
- Civics screen: choosing a direct-rule civic while Houses stand shows, in the warning style, `Adopting God King: 3 Houses secede with 9 cities, as one civilization at war with us: Prespur, Elam, … Torrin stays: an army of ours stands in it.` and the revolution's button reads `Adopt God King: 3 Houses secede`.
- Religion screen, declaring war, making peace (the diplomacy dialogs), the raze choice: the preview lines for that act under the existing text, each `Subject: text. Becomes State.` Nothing when there are none.

#### Parts C to I as built (2026-11-18)
Built in the existing screens (`crates/ffh-play/src/view/house_marks.rs`, `assets/app/persons_lines.js`;
docs/HUMAN.md, "Persons of Note in the screens"; tests `crates/ffh-play/tests/persons_screens.rs`,
`tools/web-persons-screens-smoke.mjs`). Departures and choices the spec left open:
- C: a House's line ends with its quota in words (`Orders: no limit.`, `Orders: 2 of 3 left.`) or the
  rules' named reason in amber; a Disloyal House adds the countdown's outcome. In a House's city that
  takes no order, the build lists say the reason once above them and every entry is disabled; the
  empire order's city step lists such cities disabled, the reason as the row's text.
- D: `[Change]` is a `Change…` list of the other persons with the army (one tap less than a list in a
  dialog). Restless and Disloyal both say `Takes no order to attack.`; Disloyal adds the outcome. An
  attack the commander withholds is refused in the client before the move, with the rules' reason (the
  rules refuse it too), and the path's tooltip says it.
- E: the House tag carries the state mark only when Restless or Disloyal.
- F: `Found a House` is a button under the person line, not one of the server's actions; it stays,
  disabled with the reason, where no city can be granted. The human's command layer refuses every
  order for a unit with `unit_house_refusal` set, where it refuses a marching recruit's.
- G: Disloyal's turns are a number in a dark-red pill at the shield's lower right; the ◆ and ✖ sit on a
  dark disc, so they read on every tincture. The HD-2D plates read the view's city as it is now (not
  the one the dressing kept), so a change of state shows without a redress.
- H: the rules log a secession's news to other civilizations untagged; the client knows them by the
  rules' wording (`kind: "secession"`, `report.rs` `is_secession_text`); a rules tag would be firmer. A
  change of command is left to the log (it follows every regroup). At most three alerts a batch, the
  rest counted in one more (`Persons of note: N more messages in the log.`).
- I: making peace is previewed on the trade screen at war (`If peace is made:`), where peace is
  proposed; razing in the keep/raze question's text (`If razed:`). Adopting a direct-rule civic asks
  once more with the warning and `This cannot be undone.` (the anarchy in the same question).

#### The second build's map, diplomacy, alerts, previews and unit panel as built (2026-11-18)
Rule 13's parts outside the Court (`view/house_marks.rs`, `view/diplo.rs`, `assets/app/persons_lines.js`,
`diplo.js`, `tiles.js`; docs/HUMAN.md, "Persons of Note in the screens"; tests
`crates/ffh-play/tests/persons_screens.rs`, `tools/web-persons-screens-smoke.mjs`):
- **Colours** (rule 1): the view serves, with each player, `color_player` (`player_color_player`)
  and, with each House city, `house.liege`; `tiles.js` `playerColor` draws a player in its colour
  player's colour, so every place that colours by owner (territory, borders, plates, unit flags,
  banners, the minimap, the lists) draws a realm in one colour. A change (a House breaking away,
  a new House) redraws the flat map's territory and minimap and redresses the HD-2D map (its
  dressing key carries the colours). The border between a liege and its House stays drawn (the
  map's owners differ), in the one colour.
- **Plates** (part G): every House's city carries its shield for every viewer; for its liege the
  shield carries the House's state mark on a dark disc for every state (rule 13: "its attitude
  mark for the liege"; the first build marked only Restless and Disloyal), and Furious the turns
  left in a red pill at the shield's left (under its point, a garrison's stack badge, drawn over
  the plates, covered it). The plates read the view's city as it is now, so a change of state
  reaches the page with the next view (no tile delta needed). A House's capital keeps its star.
  *The second critic pass (2026-11-18; the owner: keep the compact marks, give each a tooltip):*
  the shield, the countdown's pill and a capital's star each have a tooltip (`plates.js`):
  `House Aldric (your vassal)` and the state (`✖ Furious 8 turns`), `House Aldric: Furious.` and
  the rules' outcome (`house_breakaway_outcome`, the view's `players[].house.outcome`), `Capital of
  House Aldric (your vassal).` / `Capital of the Svartalfar.`; with the House keyword's reminder
  and the state's attitude help. The plates had no hover of their own (the pointer over a plate
  reached the plot behind it, whose tooltip names the city); each renderer now records the boxes
  of the marks it drew (`plateHits`: `r3d/hud.js`, `iso.js`, the flat map in `map.js`) as the
  HD-2D map records its seas' names (`seaHits`), and `MapView.hoverAt` gives a mark the hover
  before the plot (`plateAt`, `plateHoverAt`); a tap on a mark shows its tooltip and does nothing
  else. The map's own tooltip draws the reminders below its lines.
- **Diplomacy** (rules 9 and 13): the list shows the human's Houses first, then each civilization
  with its Houses under it, by `players[].house` (`{id, label, liege, liege_name, colour}`, and for
  the liege `state`, `turns`, `outcome`, `parts`, `loyalty`); a House's Attitude toward its liege is
  its state's mark with its loyalty parts as the tip. The trade query serves a House's tables from
  the rules (`house_bribe_refusal`, `house_bribe_price`, `house_offer_value`): Secede and Become
  your vassal always listed, each with the rules' refusal or valued at minus the price, gold at 1
  (no gold per turn: the rules' table has none for a House); the screen's totals and answer then read as for
  any deal (`Total offered: 1`, `Total requested: 150`, `Reason: House Alyndra asks 150 gold.`).
  `trade_answer` for a House is the rules' answer whoever plays it. The war a deal means is said
  under the answer (`If accepted:`, `secede_war`, `vassal_war`). *The second critic pass
  (2026-11-18):* each House in the list carries its state under its line, where a phone sees it
  (yours: `✖ Furious 8 turns` under `House Aldric, your vassal`, and no `peace` among its
  relations; another's: its state toward its liege and, when it would take the human's offer,
  `Secede: 150 gold`, from `players[].house_offer`; Pleased or Friendly, the state's mark alone
  says why it takes none, the liege being named just above, rather than the rules' `Pleased with
  the Svartalfar`). Each part is a line of its own rather than the brief's `· ` run on one line:
  the leader column is narrow on a desktop dialog too, and a run-on line wrapped at a separator or
  widened the table past the dialog (the Attitude column cut); the parts stay within the portrait's
  height. The war
  question names the parties the war brings in for certain by the vassal rules (`At war with you
  as well: House Alyndra, Malakim.`, `Your Houses join: House Aldric.`: `war_against`, `war_join`,
  `view::player::war_parties`, which follows `team_declare_war`'s vassal cascade, truces and
  crusade Hosts included; a war declared on a House, which the rules allow, names its liege and
  the liege's other Houses: `At war with you as well: the Svartalfar, House Bertram.`) and keeps
  `may` for defensive pacts, which the human does not see; Become
  your vassal names the same for its war with the liege (`vassal_war_against`, `vassal_war_join`).
  An accepted offer's cost with an AI liege is one of the `If accepted:` lines (`The Svartalfar's
  attitude toward you -1 (fades).`, `grudge` for Secede, `vassal_grudge` for Become your vassal),
  shown when the proposal will be accepted: the memory's effect (rule 15; ffh-ai `memory_added`,
  the term with one more memory less the term now) of the House bought, and for Become your
  vassal also of the war the buyer then declares on the liege (unless at war already); nothing
  when it moves nothing (the cap), the number the liege's attitude tooltip then shows (`bought
  our House`, `declared war on us`). The war question carries the same for a declaration
  (`war_memory`: `They remember it: attitude toward you -1 (fades).`, and `So do House Alyndra,
  the Svartalfar.` for the parties the vassal rules bring in whose attitude it moves). A House's vassal deal, which the
  rules never let be cancelled, shows no `cancel in` count in the list (it showed one). Departure: **no gift to one's own House.** The
  contract asked that the usual gift items (gold) work for one's own Houses; the rules refuse
  every item between a House and its liege's team (`house_trade_item_allowed`), and a gift would
  not move a House's loyalty (it is not one of its parts), so the own House's trade screen shows
  the rules' line (`House Aldric pays its liege by tribute.`), its attitude and `Open the Court`,
  and no tables.
- **Alerts** (part H): the cast in brief carries each person's House and army, so a head's news
  opens the Court at the House, a message naming one of the human's Houses without a person
  likewise. A House turned Annoyed or Furious adds `House Arne takes no levy.`, a commander `The 3rd
  Army takes no order to attack.` (no rules line marks a levy refused: the change of state that
  refuses levies is the alert). An offer to one of the human's Houses is the rules' line (`The
  Hippus offered House Arne 300 gold to secede.`), not the contract's wording.
- **Previews and the unit panel** (parts F and I) stay as the first build laid them out, on the
  rules' second-build lines; a great person's `Found a House` says what it does (`A leader is
  drawn; the House is a new civilization, your vassal.`); a hero has none.
- The staged scene gives the human at least 400 gold and twice the neighbour's House's price, so
  the trade screen's offer can be accepted.

#### J. Help and keywords
A help topic `Persons of Note` (the rules in short bullets, as the Governors topic does). Keywords with reminders (data/json/keywords.jsonc, data/text/en.tsv): House, Crown, Tribute, Levy, Sworn, Claimant (the second build). Reminders are bare and short. The loyalty states are the game's attitude words (Friendly, Pleased, Cautious, Annoyed, Furious), not keywords: a state mark's tooltip is the attitude's own help in the data (`attitudes.jsonc` `help`: what the state does for a House and for a commander). The texts of this system mark the keywords and never restate a reminder (tests/keywords.rs).

#### The second build's Court, grant list, levy and Governance line as built (2026-11-18)
Rule 13's interface for the Court, the grant list, the levy and a Crown city's Governance line
(`crates/ffh-play/src/view/persons.rs`: `court`, `grant`, `levy_house`, `governance_fields`;
`assets/app/persons.js`, `armies.js`, `governors.js`; tests `crates/ffh-play/tests/persons.rs`,
`tools/web-persons-smoke.mjs`; the shapes in docs/HUMAN.md). Where it settles what the contract
left open, or departs from it:
- **The Court is the realm.** The header states the Crown's limit as the rules do, without its
  cost (`The Crown holds 4 cities, 1 over its limit of 3.`, amber when over, `Settlements are not
  counted.` for a civilization with settlements), the realm's size and a House's threshold, and the
  realm's default tribute as a selector. The cost is the line that needs attention's (below), at
  the cities' lasting rate: a city over the limit in disorder pays nothing now, so the rules'
  current cost (`player_crown_over_cost_times100`) is 0; the line counts it at what it pays once the
  disorder ends (`player_crown_over_cost_without_times100(p, None, lasting: true)`) and says `It
  pays none while disorder lasts.` there, once. The grant list's baseline and maintenance line do
  the same.
- **The lines that need attention**, in the contract's order: a Furious House or commander with
  its outcome, a House that takes no levy (an Annoyed one; a Furious one is told above), a demand
  with 10 turns or fewer, a person without a post, the Crown's cities over its limit (`Myrean is
  over the Crown's limit of 3: maintenance +7 gold a turn. [Grant Myrean]`, the first of them; a
  maintenance change is always labelled `maintenance` and signed as a change in what the player
  pays, + costs more, - costs less), the grantable
  cities. The badge still counts only the Furious holders and the demands near their term (a levy
  refused or a city over the limit can last for long: a standing count would stop meaning
  "act now").
- **A House's card**: the leader's portrait, the shield and label, the leader's name and title, its
  origin (`Founded on turn 3.`, `Capitulated on turn 120.`, `Vassal since turn 40.`), the state mark,
  the outcome when Furious, the head's demand, the loyalty parts and total on the card (not folded:
  they explain the state, and the tribute list changes one of them), the tribute, the levy, its
  cities as chips (a tap shows the city on the map: it is the House's player's, the liege's city
  screen answers NotOwner for it), its size against the threshold (`Size: 2 of 3 cities without a
  grievance.`; the contract's wording, `before it grows uppity`, is flavour, against the rule for
  rules text), its price to secede while it would accept an offer (Cautious or worse: `Price to
  secede: 125 gold.`), Free (two steps), and folded under `Record`: `Refuses any offer to secede.`
  (Pleased or Friendly), the head's ambition, rival, record and backstory. A demand with a term
  says what its lapse does, from the rules' refusal and state function: `Demands: Vallus granted to
  Malakim. 40 turns. At its term: -3 (60 turns), becomes ✖ Furious.` (the state after in the one
  mark, only when it changes), on the House's card, a person's card and the attention line.
- **A Furious person's card** (a commander) shows its loyalty parts on the card, by its countdown,
  as a House's card does; the rest stays under `Record`.
- **The tribute selector** is a list under the tribute line (`Tribute 20%: 12 gold, 8 research last
  turn. Change`; while the rate in force moves, `Tribute 30%, now 10%: reaches 30% in 10 turns.
  Paid last turn: 4 gold, 6 research. Change`): a row per rate, 0 to 50%, each stating, once in
  force (rule 4's lag), what the House would pay (`Pays 12 gold, 8 research.`, the rules'
  `house_tribute_at`), when it would be in force were it set now, its tribute part and the total
  then (`In force in 15 turns. Tribute: -2 · Loyalty: 0`; the rate in force's row `In force.`) and
  the state mark then; the rate set is marked `set`. One tap sets a rate (no confirmation: it can
  be set back any turn), and the card is read again with the list kept open: the line says both
  rates, and the parts, total and state stay the rate in force's until the rate set arrives,
  when they are the row's (a test checks they are). The realm's default is a plain selector in
  the header.
- **The grant list** states what a grant does now: `Moves to House Arne's realm: its production
  and commerce are the House's.`, its garrison (rule 3, the rules' own predicate,
  `city_grant_garrison`: `Its garrison goes to House Bertram: 2 Warriors, 1 Archer.` or `No garrison
  goes with it.`, and `Units of your armies in it stay yours.` when the liege's army units stand in
  it; the confirmation repeats them), `Your maintenance: -3.2 gold a turn.` (when the grant takes
  the Crown back toward its limit, the limit's part of the saving, from the rules' cost of the
  cities over it before and after: `Your maintenance: -11 gold a turn (7 of it the Crown's
  limit).`), `The Crown after the
  grant: 3 cities, within its limit of 3.` while the Crown is over its limit, `Tribute from it at
  20%.`, and for a founding `Command of the 3rd Army ends.` and `Odran Vale leaves the map and leads
  House Gunthar.` (a great person: `The Great Merchant is consumed. A person of note is drawn and
  leads House Gunthar.`); then the loyalty changes (the grantee's lines with `Becomes` in the one
  mark, each snubbed claimant's). The maintenance line is the whole change of the owner's
  maintenance (`GrantMaintenance::gold_change_times100`: the city's own maintenance leaves with it,
  the others' city-count part falls, the Crown's limit included), labelled `Your maintenance`
  rather than the contract's `Your city-count maintenance`, which would name a part of the number
  only. Heroes are not candidates (the rules refuse them: `A hero cannot head a House.`).
- **The levy**: *Requisition from* lists the Houses' cities after the Crown's, each with the House's
  shield, name and state mark and `Takes levies. Each levy: -1 (20 turns).` or `Takes no levy.`
  (amber; red when Furious). The row is crowded, so the rest is the state mark's tooltip (the
  owner's note of 2026-11-18: a compact mark explains itself on hover or tap), over what the state
  does: how many levies the House takes before it takes none (`Annoyed after 2 levies.`, from the
  rules' `LEVIED` and state function), or the rules' reason it takes none (`Malakim is Annoyed. It
  takes no levy.`). The city can still be picked: the requisition passes it over while its House
  refuses, and the army's requisition head says so (`Requisition from Torrin · Ventry: passed over
  (House Aldric takes no levy).`). The House's card in the Court says the count in its levy line
  (`Takes levies. Each levy: -1 (20 turns). Annoyed after 2 levies.`), which has no mark.
- **The Governance line**: every city of the human's is the Crown's: `Held by the Crown. [Grant]`
  (the reason in its place when it cannot be granted) and `Over the Crown's limit of 3.` for a city
  beyond the limit. The House branch of the first build is gone (a House's city is never the
  liege's).
- **The civics warning** names each House (`civics_preview`, from `house_cities_held` as the rules'
  preview reads it): `Adopting God King: 3 Houses break away, taking 5 cities, each at war with
  you.`, then a line per House: `House Aldric: Ventry. Corona returns to you (an army of yours
  stands in it).`, `Malakim (a vassal): Astrum and Golden Leane.` (the period between the cities a
  House takes and the one that returns: `Ventry, Corona returns` would read as one list).
- **The unit panel's Found a House** marks the House keyword and states the cost, no reminder
  restated: `[Found a House] in Vallus. The great person is consumed; a person of note leads the
  House.` with the House chip (the button's title likewise).
- **Second person and "taking"** (the second critic pass): the persons texts the player reads say
  `you`/`your`, not `us`/`ours` (`Hippus is your House: it capitulated.`, `Free House Arne? It is
  no longer your vassal, at peace with you.`); a break-away names its cities after `taking`, never
  after `with`, which reads as a person (`House Alyndra breaks away from the Svartalfar, taking
  Gereth Minar: war.`, `..., taking Gereth Minar. Bought by the Bannor: war.`, `..., taking Gereth
  Minar, as a vassal of the Bannor: war.`, the buyer reading `Bought by you` / `as your vassal`;
  `House Aldric will break away in 8 turns, taking Ventry: war.`); a held city `Corona returns to
  you: your army's 4 units outnumber its 2.`, a city contested and lost `Corona goes with it: its
  3 units against your army's 2.` (the outcome, the civics preview, the House card's city chips
  and the message; batch H, rule 7's contest).
- **Retired from the screens**: orders and quota lines, maintenance saved, a head's trait in its
  cities, heirs, seat lines, `Release as a vassal` (now `Free`), and the four first-build states'
  keywords.

#### K. A staged scene: `ffh-play stage persons DIR`
One save that shows every state, for the tests and the screenshots: the player under Aristocracy with a capital, three Crown cities (one with two claimant Houses and a commander who took it), House A (Devoted, three cities, a wonder demand in production), House B (Restless after a refused demand, two cities), House C (Disloyal, countdown at 8, two cities, one held by an army of two, its seat contested and lost by an army of one against two of the House's units), an empty seat with two candidates, a person without a post, a great person unit, a commander (Loyal) and a commander (Disloyal, countdown), a hero. A second, `stage persons-direct DIR`: the same empire one click from adopting God King.

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
