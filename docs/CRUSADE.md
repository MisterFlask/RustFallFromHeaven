# Crusades and allied loans

A crusade turns a call to war into committed force. Civs lend units to a temporary AI player,
the *Host*, whose only goal is that war, so nobody can declare war and then sit. An *allied loan*
is a small, time-limited crusade made by agreement. This is the implementation of the design
spec "Crusade Mechanic: Design Spec" (2026-11-01); this file records what was built, the choices
made where the spec left questions open, and the self-play measures.

Not FFH2: a rules layer of this game, **off by default**. With it off no Host slot is reserved,
nothing in `ffh_core::crusade` runs, and games are bit-identical to the faithful port (checked:
per-turn sync checksums of 3 seeds x 200 turns equal those of the pre-crusade binary).

## Turning it on

- Rules: `SetupOptions::crusade = CrusadeRules::on()` (or set `enabled` and tune the other
  fields): `GameSetup::crusade`, saved with the game (save format 3).
- `ffh autoplay|stats|bench --crusades`; `--crusade-call T1,T2` also calls a council crusade
  against the leading civ at those turns (the measure harness: AI councils rarely pass war
  resolutions by themselves), `--free-rider P` makes player P pledge the minimum.
- `crates/ffh-cli/scripts/crusade_metrics.py`: the self-play measures over seeds.

## Code map

| Where | What |
|---|---|
| `ffh-core/src/crusade/mod.rs` | state (`Crusade`, `Lender`, `Call`, `LentRecord` on `Unit::lent`, reports), `CrusadeRules`, strength and city value |
| `crusade/host.rs` | Host slots: activation, its wars, passage, healing, freeing |
| `crusade/call.rs` | calls, obligations, pledges, standing at the deadline; council resolutions |
| `crusade/lend.rs` | lending, credit, and the rules hooks (combat, death, convert, captured cities, hero escape) |
| `crusade/dissolve.rs` | the crusade turn, dissolution, the draft, the return of units |
| `crusade/loan.rs` | allied loans |
| `ffh-ai/src/crusade.rs` | the Host's state machine; pledges, delivery, draft picks, loans |
| `ffh-core/tests/crusade.rs` | the spec's scenario tests (rules) and the loan tests |
| `ffh-ai/tests/crusade.rs` | AI lenders + the Host taking a city; the Host-competence report (ignored) |

Rules hooks elsewhere (each marked "Not in C++" and doing nothing without a crusade):
`unit_kill` (credit a death; a lent hero escapes home), `unit_convert` (a Host unit's record
moves to what it becomes), `unit_update_combat` (fought, kill credit), `player_acquire_city`
(victim's cities back), `CvGame::doTurn` (`crusade_do_turn` after the votes), `processVote`
(force war), `isValidVoteSelection` (no resolution names a Host), `team_can_declare_war` /
`team_declare_war` / `team_make_peace` (Host wars), `player_can_contact` (no diplomacy),
`player_can_trade_item` (no separate peace), `calculatePreInflatedCosts` (no costs), `doTurn`
(no research), `doEvents` (none), `verifyAlive` (the crusade decides), heal rate (friendly land),
`AI_conquerCity` (Host keeps cities), world scores and `countCivPlayersAlive` (Hosts are not
civilizations), FfH's open-slot searches (Host slots are reserved).

## The rules as built

**The Host.** `CrusadeRules::host_slots` (4) open slots after the spare slots are reserved
(`PlayerSetup::host`). A Host is brought alive like FfH's mid-game players (`addPlayer`: team and
player reset, which also clears the other players' and teams' entries about the slot) with the
**target's civilization** (cities it captures keep their buildings by class) and the barbarian
leader (no traits), its civilization trait removed and a neutral alignment, so a unit gains
nothing by changing owner. It has no techs, no research and no costs (commerce goes to gold: the
sacked wealth), no events and no diplomacy. It is at war with the target, the target's vassals
and the barbarians, set directly (no declaration cascade: no defensive pacts or vassal chains
reach it), and at peace with everyone else: nobody can declare war on it and it never makes
peace before dissolution. It has open borders with its lenders and the victim, heals there as at
home, and learns each lender's map when it lends. At dissolution the slot dies, its culture is
wiped from the map, and the slot is free for the next crusade. Hosts are not scored and not
counted as civilizations.

**Calling.** `crusade_call(kind, target, victim, objective, term, deadline, called)`. Objectives:
`Term`, `Capture(plots)`, `Recover` (the victim's cities the target took). Obligation:
`obligation_percent` (25) of the called civ's military strength (its lendable units' strength),
falling linearly to `far_obligation_percent` (40% of that) at the map's farthest distance from its
capital to the target. Each called civ answers through `Controller::crusade_pledge` (asked every
turn until it answers). **Council war resolutions** (`VOTE_FORCE_WAR`) now call the voting members
that could fight (at war with the target already or able to declare it) instead of declaring war
for them: "dedicate or defy". If no Host slot is free, the BtS declarations stand.

**Standing** (interpretation): at the deadline each call is settled on what was *delivered*. A
refusal, no answer, or a pledge with nothing delivered against a positive obligation costs the
full standing: for a council, the existing defiance (`setDefiedResolution`: defiance anger in the
cities, loyal membership lost). Delivering less than the obligation costs the shortfall's share of
the defiance anger (membership kept); meeting it endorses the resolution. This gives meaning to
the spec's "pledges that amount or refuses outright, whichever costs less". Faith and council
standing as such are a separate spec; this reuses what the council already has.

**Lending.** `crusade_lend(id, lender, units)`: each unit becomes the Host's where it stands
(`initUnit` + `convert`, experience kept exactly) and carries a `LentRecord` (lender, strength at
handover, turn). Lendable: combat land units, not temporary (no summons), not aboard or carrying
anything. A civ's first transfer declares war on the target through the normal path; a civ that
could not legally declare that war cannot lend (`LendError::CannotDeclareWar`). Lending is allowed
until dissolution, by called civs and **volunteers** alike (open question 5: yes). Units cannot be
recalled, and a lender whose units serve against the target cannot trade a peace treaty with it
(open question 6, **separate peace: no**). Upkeep: none (Host units cost no one anything).

**Contribution.** A unit's value is its combat strength at handover: the better of attack and
defense with its general combat bonus, x100, **scaled by its health** (so lending wounded units to
die cheaply earns what they are worth). It is credited to the lender once, on the unit's first kill
of an enemy of the crusade (a combat won against the target bloc) or its death (any cause),
whichever comes first. Share = credit / all credits. Draft order: descending credit, ties to the
earliest first credit, then slot order.

**Captured cities.** The Host keeps what it takes (it never razes; size-1 cities still burn by the
auto-raze rule). A victim's own cities (founded by it, or lost to the target during the crusade)
return to it the moment the Host takes them and never enter the pool.

**Dissolution** fires at the start of a turn when the objective is met, the target is gone, the
term is over, or the Host has no unit left (after it had some, or past the deadline). In order:
(1) the Host makes peace; (2) the draft; (3) the wealth payout; (4-5) every surviving unit goes
home to its lender's nearest city with its promotions and experience (units of a dead or cityless
lender, and units the Host got otherwise, die); (6) war end; (7) the slot is freed. The report
(`Game::crusade_reports`) keeps the outcome.

**The draft.** Pool value V = the cities' values + the Host's gold. Budgets = credit shares of V by
largest remainders (they sum to V exactly). In order each lender picks through
`Controller::crusade_draft_pick` until it stops; a city worth more than its remaining budget is
paid for from its treasury into the gold (**overdraw**); stopping keeps the rest as wealth.
Unspent budgets are then paid from the gold, pro rata (largest remainders) when it is short.
Lenders with no credit get nothing.

Open questions, as decided here:

1. **War end:** at dissolution a lender whose war with the target *began with its lending* makes
   peace with a standard peace treaty (`forcePeace`), unless its units still serve another Host
   against the same target. A war a lender was already fighting is its own and goes on. (The
   spec recommends "yes, every lender"; this keeps the intent, "crusade wars don't linger",
   without ending wars the crusade did not start.)
2. **Unclaimed cities** go to the victim if there is one, else free as independent (barbarian)
   cities.
3. **Non-unit pledges:** not allowed (gold can't kill or die).
4. **City value:** `city_pop_gold` (30) per population plus `city_building_percent` (100%) of the
   buildings' production cost at the game's speed, in gold.
5. **Volunteers:** yes. 6. **Separate peace:** no. 7. **Host healing:** lenders' and the victim's
   territory heal as friendly.

**Allied loans** (`crusade/loan.rs`). `LoanProposal { terms: LoanTerms { lender, borrower, target,
mission, strength, duration }, from, lender_gives, borrower_gives }`: either side proposes
(`loan_propose`); the other side's `Controller::consider_loan` accepts, refuses or counters; an
agreed loan (`loan_agree`) implements the paired items as an ordinary deal and lends the lender's
choice of units (`choose_loan_units`, at least the agreed strength). The loan is a deal-table item
in its own right rather than a new `TradeableItem` (that enum mirrors the C++ one, and the
diplomacy screen and AI trade valuation index it): the payment and anything else rides along as
normal trade items. One general per borrower and target; each lender's units leave when its own
term ends. Captures go to the borrower at once (no draft); at the end its gold and any city go to
the borrower. Open loan questions: **upkeep** is paid by the borrower, `loan_upkeep_gold` (1) per
lent unit per turn, and a borrower that cannot pay ends that loan; **return at expiry**: straight
home; **lender's war after expiry**: as War end above. Ends early when the borrower makes peace
with the target, the target is gone, or lender and borrower go to war.

## AI (`ffh-ai/src/crusade.rs`)

- **The Host** runs the spec's state machine, its state saved in `Crusade::ai`: Muster at the
  friendly city nearest the objective until `muster_percent` (60) of the pledges have gathered or
  the deadline passes; March to the objective; Engage: every stack next to the city strikes
  together through the civ AI's siege strike (`campaign::strike`, the expected outcome over the
  whole force, attackers in order, victors moving in), an undefended city is simply entered, and
  short of the force it besieges, bombarding or pillaging (pillage gold is sacked wealth); Recover
  in the nearest lender's or victim's city when the army's average health falls below
  `recover_health_percent` (45), for at most `recover_turns` (5). The objective is the nearest
  enemy city the whole army is simulated to take, else the least defended one. One unit
  garrisons each city it holds. Stacks re-decide every turn.
- **Pledge** (`pledge`): value of lending S = expected spoils + goal value - expected losses -
  homeland risk - the standing a shortfall still costs, kept apart in `LendValue` (traced with
  `FFH_AI_TRACE`); it steps S up from 0 to what it can spare and pledges the best (0 = refuse).
  The terms are rough gold equivalents: spoils = the target's cities the force can expect to take
  times S's share; goal = at war with the target already (+150), the target is the rival for our
  victory (+300) or disliked (+100); losses = half of S at production cost; risk grows with the
  target's strength relative to ours and its closeness, three times smaller when already at war.
- **Delivery** (`deliver`, on the lender's turn before its units move): spare units only (one
  defender per city, two in the capital, city defenders and casters stay, and a home-defence floor
  of 35% of its strength), army units strongest first, the last one the weakest that covers what
  is still owed.
- **Draft** (`draft_pick`): a city on our landmass within 20 plots of our cities, the nearest
  worth most; overdraw up to half our treasury for a city within 10 plots; otherwise wealth.
- **Loans**: a lender accepts when the payment covers losses and risk less what helping a friend
  (and fighting its own enemy) is worth, and otherwise counters with the gold it would need; a
  civ at war with a team 20% stronger asks one friendly civ every 5 turns, paying a quarter of its
  gold and borrowing only what it can keep paying upkeep for; a civ that borrowed against an enemy
  does not seek peace with it while the general serves.
- Other AIs need nothing: the target sees the Host's army as an enemy's (it is at war with it), and
  lent units no longer count as their lenders' (they are the Host's).

## Human play

`ffh-play` (docs/HUMAN.md, "Crusades"): the new-game setup's `crusades` flag (the *Crusades* box
of the browser's new-game screen); the commands `crusade_answer` (pledge or refuse a call),
`crusade_lend` (lend units; at peace with the target it asks to declare the war first) and
`propose_loan` (borrow or lend, with paired trade items; the AI accepts, refuses or counters);
calls wait in `pending.crusade_calls` and count in the end-turn gate; the `crusades` query; the
draft pick and loan offers are blocking decisions. In the browser a call is a popup (pledge,
refuse, then the lend dialog), Foreign affairs has a *Crusades* screen (calls, lending, past
crusades) and a *Loan* form per player. `ffh-play stage crusade DIR` saves a game with a call
waiting, for trying it.

## Self-play measures

`python3 crates/ffh-cli/scripts/crusade_metrics.py --seeds 1-12 --turns 300 --call 120,200
--free-rider 1` (6 civs, small maps; council crusades injected against the leading civ at turns
120 and 200, player 1 pledging the minimum):

| measure | result |
|---|---|
| crusades | 24 council crusades, 8 loan generals in 12 games |
| engagement (delivered strength that fought within `engage_turns`, 10, of delivery) | council 40%, loans 12% |
| bandwagon leak (spoils going to strength lent in a crusade's final quarter) | 0% of 1,236 gold-equivalent of spoils |
| free-rider probe (player 1 pledges strength 1, so lends one unit) | 7% of the spoils of its 19 calls, for 10% of the delivered strength (an equal split among the lenders would have given it 68%) |
| refusals | 78 of 119 calls (the target is the leading civ: the homeland-risk term weighs heavily) |
| drafted cities | none: the Hosts took few cities and lost them or had them retaken before dissolution; the pools were plunder and pillage gold |
| dissolutions | council: 13 term, 11 no units left; loans: 7 ended (borrower's peace, term, upkeep), 1 no units |

The contribution gate does what the spec wants of it: late or token strength earns in proportion
to what actually fought, never to what was merely handed over. The free rider's one unit was its
weakest, and it usually fought, so its share is small but not zero; "close to zero"
holds as the other pledges grow (with these small crusades one unit is a tenth of the force).

Host competence (`cargo test --release -p ffh-ai --test crusade -- --ignored --nocapture`): the
Host against a defender that sits fortified in its city, 8 attackers of one type (axemen,
champions, horsemen, axemen) against defenders (archers, longbowmen, axemen, warriors) of a given
share of their raw strength, 10 seeds each, city taken within 30 turns of the call, 10 plots away;
next to the ordinary civ AI with the same 8 units at war with the same city:

| attackers' raw strength | Host | civ AI |
|---|---|---|
| 1x the defenders' | 0/40 | 0/40 |
| 1.5x | 0/40 | 1/40 |
| 2x | 6/40 | 9/40 |
| 3x | 40/40 | 40/40 |

Raw strength (the contribution metric) leaves out fortification and city defense, so "equal
strength" is the defender's game for any attacker; the civ AI column is the yardstick. Before the
Host used the civ AI's strike it took 11/40 at 3x (it judged each stack alone).

## Known gaps

- AI councils rarely elect a Head Councilor or pass war resolutions in self-play, so council
  crusades seldom arise by themselves (the measures inject them).
- Crusade armies at the AI's pledge sizes (about a quarter of a civ's army) seldom take the
  leading civ's cities; most crusades end with a small pool. Pledges, the obligation share and
  the risk term are first guesses: tune them (and measure win rates) before relying on the
  mechanic in play.
- League defence (a call source with a victim) is a separate spec; the victim machinery is here
  and exercised by the tests and loans.
- With warbands (not in the game) the warband would be the unit of credit.
