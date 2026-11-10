### Warships (`warships.rs`, switch `WARSHIPS`)

Before, every sea unit without cargo room held in place every turn (`units::role_of` gave it
`Role::Naval` -> `hold`), and so did the barbarians' ships and sea animals: in 16 games, no duel
between ships, no blockade, no sea improvement pillaged, 25,000 barbarian sea unit-turns 100% still
(docs/AUTOPSY.md, "Naval"). `FFH_AI_WARSHIPS_OFF=ALL` plays exactly as before (checksums equal on
seeds 3 and 7, 200 turns).

**Who plays here.** A civilization's armed sea unit with no cargo room (`Role::Naval`); a sea
explorer with no transport room and nothing left to reveal (`naval::sea_explore_target` empty);
an idle transport (`free_transport`: armed, alone, carrying nothing, and none of `naval.rs`'s jobs
open: no assault target, no settler waiting for a ship or in port, no army waiting, no pickup under
way, no army of ours beside it), which plays rules 1, 2, 4 and 5 and otherwise goes back to
`naval::transport`. A blockading transport that `naval.rs` comes to want leaves the blockade.
Every barbarian sea unit (`Role::Naval`, `Transport`, `SeaExplore`) plays [`barbarian`].

**A warship's turn**, the first rule that applies (constants in `warships.rs`):

1. *Heal*: damaged `HEAL_PCT` (50%) or more outside a city of ours: to the nearest port it reaches
   (20 turns); in port it stays while damaged above `HEALED_PCT` (10%).
2. *Kill*: an enemy ship (a team at war with ours; barbarians always) it can attack this turn (next
   to a plot it reaches with moves left) at the rules' odds (`get_combat_odds`) of `ODDS_BAR`
   (65%), or `ODDS_BAR_PRIZE` (50%) against a transport seen carrying units, a ship blockading or a
   ship on a sea resource of ours. Order: loaded transports, other prizes, best odds. Each attack is
   recorded in the prediction ledger, kind `sea_attack` (`AttackWon`, this turn; ctx `src`
   kill/gang/barbarian, `prize`).
3. *Escort*: while an invasion is on (`naval::amphibious_target`, switch `FLEET`), the warships
   without cargo room nearest the staging port, one per `ESCORT_PER_TRANSPORTS` (2) assault
   transports and at most `ESCORT_SHARE_PCT` (50%) of them, wait there awake;
   `naval::fleet_waits` takes the idle warships in port along when the fleet sails.
4. *Guard the coast*: an enemy ship within `COAST_RADIUS` (2) of a water plot of ours that no
   ship beats alone. The free warships that can attack it this turn, strongest first, attack
   together when the duels in that order kill it with the bar's chance (the first at
   `GANG_FIRST_BAR` 35% or more, at most `GANG_MAX` 3; each loser's expected damage carried into the
   next duel with `combat_sim::fight`). Otherwise the warships within `GATHER_TURNS` (3) wait in
   our coastal city nearest it (ships in a city cannot be attacked by ships).
5. *Blockade* (at war with a civilization): the water plot within `BLOCKADE_TURNS` (8) where
   `MISSION_PLUNDER` is allowed that denies the most -- the enemies' water plots within
   `SHIP_BLOCKADE_RANGE` in its water area that no blockade covers yet: 1 each, 2 more worked, 2
   more with a resource; at least `BLOCKADE_MIN` (4) -- per turn of travel, the post it holds
   weighted a fifth more, skipping plots another ship of ours sails for (within 2) and plots an
   enemy ship that beats it at `DANGER_ODDS` (50%) can reach next turn. An enemy sea improvement
   it stands on is pillaged first. A blockade then holds without new orders (`keeps_station`,
   checked in `units::play` before the group is re-planned) until the war ends, it is hurt, it is
   endangered, the post denies under the minimum, or an enemy ship comes within its moves + 1.
6. *Guard post*: a coastal city of ours within `POST_RIVAL_DIST` (10) of a rival civilization's
   coastal city or `POST_BARB_DIST` (6) of a barbarian ship seen (those first) is a post; one
   warship each (a ship keeps the post it stands on), the rest fortified in the port with the most
   worked sea plots and sea resources.

**Barbarian ships and sea animals**: attack at `BARB_ODDS` (50%, the land animals' bar), pillage a
sea improvement of a team at war they stand on, hunt a ship they beat within `BARB_HUNT_TURNS` (2),
else move to a water plot within their moves picked by a hash of unit id and turn (the land
animals' wander; no RNG draw). They do not blockade. The rules keep animals out of owned plots.

**Production** (`warships::production`, called from `production::decide` after the assault fleet,
coastal cities that are safe and can afford a unit): wanted = escorts (one per 2 assault
transports while an invasion is on) + guards (the non-animal enemy ships seen within
`SEEN_RADIUS` 3 of our ports that our best warship can fight: strength at most 1.5x ours) +
blockaders (at war with a civilization with a revealed coastal city within `SEA_REACH` 20 of a port
of ours on a water area our ports touch, whose ships seen are not stronger than ours: one per such
city, at most `BLOCKADERS_MAX` 3) + 1 for a seafaring civilization (coastal capital and a sea unit
class of its own in the data) when any reason holds. Trained only while warships (alive and in
training) are fewer than wanted and our armed sea units hold under `HAMMER_SHARE_PCT` (20%) of the
military's hammers; the best sea unit with strength and the attack or escort sea AI the city can
train, by strength squared per turn (Trireme, Frigate, Man-o-War... may be transport types: trained
as `UNITAI_ATTACK_SEA`, they serve `naval.rs` when it needs a ship).

**Measured** (seeds 1-16, 300 turns, 6 civs, standard map; the base is the switch off, which plays
as before; `autopsy.py --sections naval,falls`, `calibration.py --kind sea_attack`):

| | standard: off | on | high sea level: off | on |
|---|---|---|---|---|
| duels with a sea unit (civ-civ / civ-barb / animal) | 1 (0/1/0) | 66 (35/24/7) | 1 (0/1/0) | 48 (21/22/5) |
| civilizations' ships attacking / defending / lost | 0/1/1 | 61/40/38 | 0/1/0 | 38/26/25 |
| transports sunk with cargo, units drowned | 0, 0 | 0, 0 | 0, 0 | 0, 0 |
| armed-ship-turns blockading | 0 | 701 | 0 | 404 |
| armed-ship-turns next to an enemy city | 190 | 405 | 160 | 287 |
| sea improvements pillaged: by civs / by barbarians | 0 / 0 | 3 / 11 | 0 / 0 | 4 / 6 |
| ships built (transport class / warship class) | 104 / 87 | 166 / 160 | 73 / 81 | 158 / 139 |
| ships' share of unit+building hammers | 1.1% | 1.8% | 1.0% | 1.8% |
| warship-class unit-turns, port/own waters/elsewhere %, still % | 4435: 9/37/53, 18 | 7247: 25/29/45, 42 | 5575: 11/32/55, 23 | 7159: 28/31/40, 37 |
| `UNITAI_ATTACK_SEA` unit-turns (in port / moved) | 184 (184/0) | 8606 (5771/2169) | 23 (23/0) | 6165 (4669/1375) |
| barbarian sea unit-turns still | 100% | 0.1% | 100% | 0.2% |
| barbarian ship kills / sea pillages, by third (early/mid/late) | 0/0/0, 0/0/0 | 0/0/3, 0/0/11 | 0/0/0, 0/0/0 | 0/0/3, 0/0/6 |
| military landings (units); assault / capture | 15 (44); 4/4 | 5 (23); 1/1 | 14 (22); 2/2 | 17 (34); 1/2 |
| assault fleets sailed / with an escort | 33 / 0 | 69 / 0 | - | 39 / 2 |

Decisions in the standard run (`decision` kind `warship`): post 1921, gather 521, blockade_sail 317,
port 250, blockade 140, heal_stay 63, escort 62, kill 61, pillage 14, heal_sail 9, barbarian
attacks 5, barbarian hunts 6. The `sea_attack` ledger: standard 59 resolved, predicted 79.7%,
observed 78.0%, Brier 0.145 (reference 0.172, skill +0.16); high sea level 37, 78.7% / 81.1%,
skill +0.16. The duel odds are the right estimate for these fights.

Every fight, pillage and kill falls in the last third: ships come late (the first ship's median
turn is still about 225). Barbarian ships did not wreck the early game (no kill or pillage before
turn 200, sea animals cannot enter owned water), so their radius and aggression were left as built.

Guardrail (`ai_metrics.py`, seeds 1-16, 300 turns, against the switch off): military at turn 100
11.9 -> 12.1, cities at 300 7.4 -> 8.1, techs at 300 40.2 -> 40.5, research at 300 286 -> 270
(range 0-1231), civ cities captured by civs 315 -> 325, by barbarians 40 -> 51, eliminations 23 ->
17. Cost (`ffh bench`, seeds 1, 3, 5, small map, exclusive slot): 13.9 / 15.0 / 14.4 ms a turn off,
13.6 / 15.1 / 14.1 on.

**Trace review** (seeds 4 and 12, `FFH_AI_TRACE`), fixed in one pass: blockaders hopped between
posts because another ship's claim within 2 plots excluded the post a ship already held (the held
post is now exempt); two warships traded a guard post every turn by unit id (a ship now keeps the
post it stands on); sea animals of one move never moved (the flood counts a step that spends the
last move as next turn's; the wander now takes plots within the unit's moves); escorts went to the
staging port only once a transport there was loaded, after the fleet had usually sailed (they now
go while the invasion is on).

**What does not work**:
- Escorts almost never sail with a fleet (0 of 69 and 2 of 39): `naval::fleet_waits` takes only
  warships without cargo room, and the civilizations' warships after the Galley (Trireme,
  Frigate, Man-o-War) all have room; with an invasion on, `naval.rs` claims those as carriers.
  Loaded settler transports sail alone (no escort rule follows a ship at sea).
- Fewer military landings in the standard run (15 -> 5, also 7 in an earlier build) but more at
  high sea level (14 -> 17); fleets sailed twice as often. Games diverge by turn 225 and the
  amphibious wars are few (6 between teams sharing no landmass in 16 games), so this is not
  attributed; worth a look with more seeds before the warships are blamed.
- The guard rule waits in port as long as an enemy ship it cannot beat stays within 2 plots of our
  waters (521 gather decisions): a parked monster keeps a civilization's ships home and off its
  blockade.
- "Seen in our waters" is the present sight only (no memory in `Game`), so guard production
  follows what is visible on the turn a city chooses.
- No ship attacks a land target; ships do not bombard cities; transports never take part in a
  joint attack while `naval.rs` holds them.
