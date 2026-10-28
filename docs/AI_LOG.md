# AI changes that did not move the win rate

Every AI change that was measured and did not hold -- no gain, a loss, or a gain on development
seeds that fresh seeds did not confirm -- gets an entry here, so the next attempt starts from what
the games showed instead of repeating the idea. **Read the entries for the area before changing
the AI, and add one before reverting a change** (CLAUDE.md, "AI changes"). Older failures from
before this log are recorded where they were measured in docs/AI.md (in particular "Looked
promising on development seeds, failed on fresh ones" and the "Tried, not kept" notes of each
section).

Each entry answers three questions, from the games themselves (counts with `ffh stats
--detail`, per-checkpoint paired comparisons, `FFH_AI_TRACE` replays), not from the idea:
1. **Why it made sense**: the mechanism and the evidence that it was missing.
2. **Why it did not move the win rate in the games**: what actually happened when it fired --
   how often, when, what the units or cities did, what it cost.
3. **What it implies**: the correct version of the improvement, the generic gap underneath, or a
   balance verdict when the content itself is weak in this game.

## How changes are measured (and why entries pile up)

- Development: `crates/ffh-cli/scripts/civ_ab.py` (or `ai_metrics.py` / `ab_compare.py` for
  generic changes) on development seeds, against the base, the other changes switched off.
- Validation: seeds never used to choose anything, the change on against the same binary with it
  off (`FFH_AI_CIV_OFF` for a civilization's logic). Keep only if neither the win rate nor the
  share at the end falls. Seeds 1-40 chose among variants and flatter them: in round 5, five of
  the nine civilization changes kept after development lost on fresh seeds 5001-5064. Seeds used
  for a validation are spent; the next validation takes new ones (6001+).
- A result from a handful of games is a hypothesis: the Infernal change below looked like
  +500 share over 6 games and was -29 over 60.
- Before reverting, look at the games: replay the seeds where the change fired with
  `FFH_AI_TRACE` (a throwaway trace line is fine on a scratch branch; it must not change the
  checksum) and compare the snapshots per checkpoint (`civ_ab.py --load`, the snapshot fields).

## Magic civilizations (2026-10-28)

The kept versions are in docs/AI.md, "Spellcasting", "Magic civilizations". Numbers are full games
(6 civs, standard map, to a decision or turn 690), the Amurites, Sheaim, Svartalfar and Balseraphs
each forced into slot 0 on seeds 1-30 (120 pairs), `civ_ab.py` against the same binary with the
switch off (for a switch every civilization may play, on for the forced civilization only:
`--new-env FFH_AI_<NAME>_ONLY={civ}`); "arcane" counts arcane casters (unit combat Adept).

### Sorcery right after Writing, whether or not we had an Adept (`ARCANE_PATH`, replaced)

1. A magic leader's Adepts study 30 to 70 turns before they can become Mages, so the school should
   open early and Sorcery (2000 beakers) come as the students reach level 4: Mysticism and
   Knowledge of the Ether after the first essentials, a sphere tech and Sorcery after Writing.
2. Knowledge of the Ether came at turns 115-150 instead of 170-250, arcane casters at turn 200
   0.1 -> 0.5, but the Sheaim lost share at turns 200-300 (-151, t -2.0; -179, t -2.1): Sorcery
   came before the Adepts it was for, and its 2000 beakers before the economy's techs. Pooled: win
   12.5% -> 11.7% (5 / 6), share@end -100 (t -0.97).
3. Sorcery waits for a student (an Adept of ours that can become a Mage). With that: win 12.5% on
   both sides (6 / 6; Svartalfar 2 -> 5 wins), share@300 -99 (t -1.70), share@end -81 (t -0.67);
   the cost is spread thin (population -2 at turns 200-250; cities, techs, production equal).

### A caster for every three field units, counted in the army (`MAGE_ARMY`, replaced)

1. At turn 400 a magic civilization had 4 arcane casters to 45 military units: its corps was
   (cities + 2) / 2 at most, trained only while the training city saw no threat at all, and an
   expert's magic army has its Mages in proportion. A magic leader kept a caster for every three
   field units, its war wizards counting toward the army target (so the army's size held), and
   trained casters while a raider the garrison held was in sight.
2. The casters came (made per game about 4.5 -> 10, arcane at turn 200 0.4 -> 1.1, Mage-turns
   x1.9) and died in proportion (about 30% of those made on both sides), but win 13.3% -> 10.0%
   (0 / 4, p 0.125), share@end -90 (t -1.30), legacy@end +64 (t 0.92). The army held its size, so
   the war wizards replaced fighters and defenders: cities lost to civilizations 3.02 -> 3.48 a
   game, cities taken 2.60 -> 2.67; research 7-11% lower at turns 250-400 (the slider, for the
   casters' upkeep). In the four games lost (Sheaim 20 and 30, Svartalfar 8, Balseraphs 18) the
   empire fell behind in research from turn 250 and lost cities it had held.
3. A Mage neither attacks nor holds a city as a fighter does; its Fireball adds to what the
   fighters do and does not replace them. The casters come on top of the army, one for every four
   field units.

### The first package on fresh seeds (Sorcery after Writing, Adepts for the army before Sorcery; replaced)

1. The four behaviours kept on development seeds (`ARCANE_PATH` with Sorcery after Writing once
   we had an Adept, `BARRAGE`, `CASTER_FORCE`, `MAGE_ARMY` with a caster for every four field
   units) and `CITY_SPELL_THREAT`, on for the forced civilization only against all off, fresh
   seeds 52001-52040 (160 pairs, telemetry on).
2. Every magic measure rose -- arcane casters at turn 200 0.06 -> 0.88 and at turn 400 3.6 ->
   5.4, Mages at turn 400 2.6 -> 4.3, the first Mage (median) at turn 450 -> 350, spell damage 72
   -> 99 and summon kills 9.5 -> 14.8 a game -- but win 16.2% -> 9.4% (7 / 18 seeds, p 0.043),
   the Balseraphs 13 -> 5 wins, share@end -48 (t -0.47), legacy@400 +62 (t 1.03). The economy
   paid in the window that decides these games: research -6% at turn 200, -11% at turn 250, -8%
   at turn 300, the slider at turn 200 67% -> 60%, population -1.4, unit costs up; most of the
   lost games split from the base between turns 150 and 250, before a single Mage. Sorcery's
   2000 beakers came right after Writing, ahead of Trade, Currency and the other economy techs,
   and the army's extra Adepts were trained a hundred turns before they could be Mages.
3. An expert times both to pay: Sorcery after the economy's tier 2, and the army's casters once
   they can be Mages (the corps the traits and unique casters keep comes before, as it did).

### Knowledge of the Ether right after the first essentials (`ARCANE_PATH`, replaced)

1. The package again with those two timings (seeds 52001-52040, now development seeds): an Adept
   studies 30 to 70 turns before it can be a Mage, so the school still opened early -- Mysticism
   and Knowledge of the Ether right after the early military tech, Exploration and Mining.
2. The late game recovered (share@400 +103, t 1.39; legacy@400 +111, t 1.74; share@end +2), but
   win 16.2% -> 10.0% (5 / 15, p 0.041; Balseraphs 13 -> 7, Amurites 4 -> 1, Sheaim 5 -> 2) and
   share@200 -43 (t -1.46): only 2 of 160 games stayed the same, and Knowledge of the Ether at
   turn ~120 let the corps the traits keep train its Adepts (0.9 at turn 200 instead of 0.06) that
   then waited 150 turns and more for Sorcery, at the economy's cost, while the games' leaders
   pulled away.
3. The Adepts should be in school when Sorcery is in sight, not a century before: the magic
   leader's beeline is Sorcery itself, with Knowledge of the Ether and the sphere tech on the way,
   after the economy's tier 2 (the Amurites' own plan already did this).

### Puppets counted as Fireballs (`MAGIC_WAR`, `CASTER_FORCE`; fixed)

1. The package with the Sorcery beeline after the economy, the mage army with Sorcery and
   `MAGIC_WAR` (each caster whose spells hurt counts 10% of a defender's health in the war
   gate's takeability test), on fresh seeds 53001-53040 (160 pairs).
2. Win 13.8% -> 10.6% (4 / 9, p 0.27), share@400 +45 (t 1.40), share@end +22 (t 0.38); the
   Amurites, Sheaim and Svartalfar together 10 -> 9 wins, the Balseraphs 12 -> 8 -- as in every
   version of the package. In the Balseraphs' lost games they declared more wars, and earlier,
   that took nothing (seed 53002: war at turn 376 instead of 455; 53004: 49 casters made instead
   of 9, 20 cities taken and 19 lost). Every Balseraph caster knows Create Puppet (a
   civilization spell, FfH weight 1000), a summon of strength 2, so each counted as a Fireball:
   the gate saw 40% off every defender.
3. Spells with an FfH weight of their own are cast for the weight, not the fight: the war gate
   and the force estimate leave them out, as the barrage did.

### City mages to the front (`MAGE_FRONT`, reverted)

1. A city mage stayed in its own city: in seed 4 (Amurites forced) a Wizard cast Haste in a quiet
   city while three cities a few turns away fell to a stack of Champions. An expert moves his
   Mages where the fighting is: a mage with a spell that hurts (a Fireball, not an Adept's Haste or
   Floating Eye) left a quiet city for a threatened city of ours without a mage within six turns.
2. On for the forced civilization only: win 12.5% -> 10.0% (1 / 4), players alive at the end
   down (t -2.03), legacy@end -66 (t -1.45); 51 of 120 games changed. In the changed games the
   casters lost rose sharply (Amurites seed 18: 2 -> 26, seed 2: 2 -> 13; Svartalfar 24: 3 -> 14;
   Balseraphs 6: 1 -> 13): the threatened cities they went to were the ones that fell, and every
   unit in a city that falls dies with it. A first version also sent Adepts (no damage spell) and
   one died in the city it walked into.
3. "Threatened" is the wrong test for where a Mage helps: a city the enemy is about to take is a
   trap for every unit in it. The barrage already lets a city's own mage hurt the stack at its
   gates. A Mage would go where the city holds with it and falls without it -- that needs an
   estimate of the siege's outcome, which the AI does not make yet.

## Leader traits (2026-10-26)

The kept version is in docs/AI.md, "Leader traits". A trait's play is kept whatever its measured
cost (the owner's decision: flavour as much as strength), so these entries record versions that
were replaced because the games showed a flaw, and what the games showed. Numbers are full games
(6 civs, standard map, to a decision or turn 690), seeds 1-40, `trait_ab.py` against the same
binary with `FFH_AI_TRAIT_OFF=all`; "pooled" is every trait on, "alone" one trait on (the others
off), which leaves out what the other traits' players do to their neighbours.

### Raiders pillaging the land of the city they besiege (replaced the same day)

1. A Raiders leader's stacks pillaged every improved enemy plot they stood on, and a siege holds
   its stacks on the plots around the target city for turns.
2. In the traced games the siege stacks stripped the target city's own land: Hannah (seed 2)
   pillaged a town four times over at the city she was taking (town, village, hamlet, cottage:
   164 gold) and then owned a city with bare land. Pooled, the Raiders leaders won 11.1% of their
   games instead of 17.8% (1 / 4 seeds won by one side only), share@end -119 (t -0.91).
3. The plots within two of the city the war operation is taking are spared (ours once it falls);
   the march and the raids pillage the rest. Pooled again: share@end +170 (t 0.93), cities +0.7,
   cities taken +0.7.

### Financial counting the threshold on every plot (replaced the same day)

1. A Financial leader gets +1 commerce on plots with 2 or more; the workers' valuation
   (`plot_calculate_improvement_yield_change`) leaves that out, so the first version added it on
   every plot, at the averaged grown yield of a cottage (a cottage counted as a town for two
   thirds).
2. Alone, the Financial leaders won 11.1% of their games instead of 26.7% (2 / 9, p 0.07),
   share@end -291 (t -1.42). Their alive players had a fifth fewer hammers around turn 250
   (-2.6 at turn 200, t -2.4; -4.5 at turn 250, t -2.5) and more research only from turn 400
   (+48, t 2.7). Traced (seed 22, Garrim): cottage decisions 50 -> 98 in 250 turns, farms
   94 -> 79, mines 83 -> 65, roads 96 -> 75 -- the workers' turns went into cottages, many on
   plots no city worked, where a cottage never grows and the extra commerce is never had.
3. The extra commerce counts only on a plot a city of ours works now, weighted as the yields are
   (the improvement 1, its grown form 2). On seeds 1-40 that version still looked worse (win
   26.7% -> 17.8%, share@end -506, t -2.52), but most of the seeds 1-40 numbers were the games'
   chaos, not the change: a change re-draws every game it touches, so a group that won more than
   its share of these seeds falls back and one that won less rises -- in runs where only
   Industrious or Spiritual was played, the Summoner leaders' win rate went 0% -> 14% and the
   Arcane leaders' 4.8% -> 9.5% with their own play off. On 60 more seeds chosen for a Financial
   leader (41-129, 87 Financial players), Financial alone against all traits off: this version
   win 18.4% -> 18.4% (7 / 7), share@end -32 (t -0.24); a variant skipping the extra commerce
   while the city is short of food 18.4% -> 20.7% (9 / 7), share@end -57 (t -0.52). Both had
   fewer improved plots worked around turn 250 (-3.4, t -2.7; -2.9, t -2.4); on an improved plot
   the extra commerce could tip a farm into a cottage that starts over, so the kept version counts
   it only on an unimproved worked plot: win 18.4% -> 17.2% (6 / 7), share@end -39 (t -0.33),
   improved plots worked at turn 250 still -2.9 (t -2.2; not the replacements, then -- unexplained,
   and within what a dozen compared fields throw up by chance).

### Aggressive counting its Combat I from the start (replaced the same day)

1. An Aggressive leader's fighters start with Combat I (+20%), which neither the power rating
   nor a unit type's strength counts, so the war bar (`diplomacy`: the power edge over a target,
   our best trainable attacker against its cities' defenders) undervalued them.
2. With every trait on (fresh seeds 7401-7496, 85 Aggressive players) they won 14.1% of their
   games instead of 21.2% (4 / 10), share@300 -91 (t -2.04). Alone (the other traits off) the
   win rate held (21.2% -> 20.0%, 7 / 8; share@end -24) and they took more cities (+1.3, t 2.0),
   but the war came early: at turn 200 wars +0.2 (t 3.3) and military +13 (t 3.1), and 0.6 fewer
   cities (t -4.4); at turn 300 research -28 (t -3.5). Preparing a war halves the settlers and
   turns production to the army while land is still free -- the cost docs/AI.md records for a
   looser war gate in general.
3. The bonus applies once the home landmass is full (`naval::home_is_full`: under a quarter of
   it unowned). Alone: win 21.2% -> 23.5% (5 / 3), share@end +30 (t 0.38), the same game to turn
   200, cities taken +0.8 (t 1.7), research at turn 300 -8 (t -2.3: the price of the wars).

### Summoners summoning more without more casters (kept, but did not do it alone)

1. The first version's Summoner leaders summoned 15 times a game (fresh seeds 7401-7420): casters
   came around turn 300 and summoned only in fights. Three changes aimed at that: research toward
   the Adept (Knowledge of the Ether), Raise Skeleton at peace while the army is short, and a
   temporary summon valued 1.5x for the turn longer it lasts.
2. On 40 seeds with a Summoner leader (45-136, 48 players) they summoned 16.8 times a game against
   16.9 before, though the first summon came at turn 266 instead of 295: 0-4 casters were alive at
   a time and they died in the fights where they summon (seed 45: one Adept at turn 200, none at
   250 and 300); the peace skeletons hardly fired, as these empires met their peacetime army
   target (permanent summons pay support like any unit, so the cap stays).
3. The bound was the number of casters. With the bigger corps of the civilizations with unique
   casters, (cities + 2) / 2, the same seeds gave 24.3 summons a game (Tebryn 43.5, Os-Gabella
   32.5), 15.0 Fireballs, win 12.5% -> 14.6% (1 / 0), share@end +59 (t 0.43); on fresh seeds
   7501-7604 16.4 against the first version's 10.3 (docs/AI.md, "Leader traits"). The three
   changes stay: they bring the casters sooner and give them more to do.

Lesson for measuring a trait (or any play a few players have): 40 seeds give each trait 15-45
players whose win rates move by 10 points on chaos alone. Play the trait alone (the other traits
off, so only games with its leaders change) on seeds chosen to contain its leaders, 60+ of them,
before reading a drop as a flaw.
## Round 7g: the simulation for every attack (2026-10-25)

### Field attacks as a pure trade of what kills and losses are worth (replaced)

1. **Why it made sense.** The field estimate (`military::assess`/`attack_ok`) counted a third of
   a kill for every lost fight whatever the odds, the defender's strength or first strikes (four
   Warriors at a Griffon at 1%), took each attacker's odds against the best defender at full
   health, and passed up good shots (a wounded Lizardman at 60%). The faithful simulation
   (`combat_sim::simulate`, the rules' group-attack order) gives the expected kills and
   losses; an expert attacks when the exchange favours him. So: attack when the kills are worth
   more than the expected losses (`combat_sim::worth`: cost or 15 hammers a strength point, whichever
   is more, plus a tenth per experience point), kills counting double on our land and half again
   against barbarians elsewhere.
2. **Why it did not hold.** One player with it against opponents without (`civ_ab.py --civ all`,
   seeds 33001-33016, 304 paired games, 300 turns): win rate 12.2% -> 9.2% (16 seeds won only with
   it, 25 only without), share -3.9% (t -1.34), legacy -4.1% (t -1.42). The player was behind in
   military units by turn 100 (-0.33, t -2.4) and in power, workers (t -2.5) and cities by turn
   200. In traced games the switch added about 110 attacks a game the old rule refused: 285 of 439
   by lone units, 126 whose best simulated fight stayed under 50%, a quarter against animals
   (which stay out of our borders). The weights made a Warrior (worth 45) a fair trade for a Bear
   (75 x 1.5) at 40%; every loss of a lone unit was a whole stack, early, when units are scarce.
   Reweighted (losses at 150%, animals at half), it swung the other way: a lone unit passed up a
   Lion at 75%, six units a barbarian Axeman they would surely kill.
3. **What it implies.** A trade sum is too sensitive to its weights to be the decision, and it
   prices neither the experience a winner gains nor the threat removed. The kept rule decides as
   an expert does, on the simulation's numbers: attack when the attacker the rules send has good
   odds (65%; 75% hurt); grind at worse odds only when the stack clears the plot (every defender
   90% sure to fall) and the trade is favourable; a capture when the city falls. The simulation's
   accuracy -- first strikes, wounded defenders, the real attacker order -- is what the old rule
   lacked, not a different notion of when to attack.

### Field attacks on good odds or a clearing grind, raiders at face value (replaced)

1. **Why it made sense.** After the pure trade (above): attack when the attacker the rules send
   has good odds, wear the defenders down only when the stack clears the plot at a favourable
   trade -- the old rule's intent on the simulation's accurate numbers, without its hopeless
   grinds (six units at an Elephant at 0.2%).
2. **Why it did not hold.** One player with it (304 paired games each, 300 turns): seeds
   33001-33016 win rate 12.2% -> 12.2%, share -3.1% (t -1.61), legacy -3.7% (t -1.89); seeds
   35001-35016 win 17.1% -> 14.1% (15 / 24), share -1.2%. The player was behind in military by
   turn 100 on the first set (t -2.9), and the games lost more cities to the barbarians (+0.33 a
   game, t 2.2); the player ended with fewer cities (-0.34, t -1.8 on the second set).
3. **What it implies.** Grinding at a raider near our cities is not a trade of units: the raider
   left alone takes a city. The old estimate's eagerness, wasteful by unit value, was protecting
   the cities. Kept: raiders (not animals) on our land or within three plots of one of our cities
   count four times their worth, so a grind that clears them is always worth it; elsewhere the
   rule stands.

## Round 7c: wars, diplomacy, councils (2026-10-22)

Two first versions of round 7c's behaviours (docs/AI.md, "Diplomacy" and "Round 7c") were
measured, traced and replaced by the versions kept.

### Tech-trade discipline by monopoly value, the leader refused outright (replaced)

1. More Naval AI's `CvTeamAI::AI_techTrade` keeps a technology while few civs know it, the more so
   the more it is worth (`AI_getTechMonopolyValue`: unbuilt wonders, world units, strong units),
   and an expert does not feed the runaway leader. The first version refused the clear leader
   (a share 1.5x the runner-up's) and the victory rival every technology, any military technology
   while we prepared or fought a war, and to a non-friend any technology of monopoly value 50+ that
   most civs we had met lacked.
2. In 24 full games (seeds 1-24, everyone with it against everyone without) it withheld 1,000
   swaps as monopolies, blocked 353 with the leader or rival and 64 military ones; the world fell
   4.5% behind in technologies at turn 300 (t -3.0), legacy -3.7% (t -2.7), power -5.5%, and games
   were decided 29 turns later. One civilization with it against the rest (`civ_ab.py`, seeds 1-4,
   76 pairs) won 18.4% of its games against 21.1% (5 seeds won by the base only, 3 by the new
   side), share at the end -143 (t -1.5), legacy at turn 200 -1.9 (t -2.4), rank 3.5 -> 3.7.
3. The swaps here are one for one at similar cost, so each side gains a technology against every
   other civ; refusing one costs the refuser the technology it would have received, while the
   others trade on. Keeping a monopoly pays only when the technology itself is a race -- the wonder
   one of our cities is building (More Naval AI's DENIAL_MYSTERY) -- or a weapon against a
   neighbour we may fight; and the leader's swaps are worth taking when they favour us. The kept
   version does exactly that (125% from the leader, the victory rival still refused).

### Re-check before declaring at the selection's 120% margin (replaced)

1. More Naval AI checks a prepared war once more before declaring it ("last chance to change our
   mind", `AI_doWar`): the target must still pass the gate that chose it. The first version used
   the gate unchanged but for 10% slack on the power bar, so takeability kept its 120% margin:
   our best trainable attacker against the best defender of their weakest city, fortified.
2. Seeds 1-48, 300 turns, against the same binary with it off (in seeds 1-24, 39 prepared wars
   dropped, 35 as no longer takeable): wars declared 3.04 -> 1.67 a game (t -6.3), civ cities captured -24%
   (t -2.8), legacy +0.8% (t 1.0). The test takes the weakest of their revealed cities, and
   revealing more cities can only lower it, so a target that passed at the selection and fails at
   the declaration has better defenders after the 30-40 turns of preparation (telemetry reason
   `not_takeable`; not traced city by city), while our massed army is the same.
3. The 120% margin selects wars (docs/AI.md, "War conversion vs. decisive wars"); at the
   declaration the army is massed at the border and the question is whether its units still match
   the defenders. The kept version asks for parity there: full games, wars +4%, legacy +3.7% at
   turn 400 (t 1.7).

## Low-hanging fixes (2026-10-21)

Six fixes for things a good player does differently, found by tracing seeds 1-18 (the two kept --
great people building shrines and rushing wonders, explorers moving one safe turn at a time --
are in docs/AI.md, "Measured together", round 6: neutral on fresh seeds, kept as plainly right
play), and two garrison quotas tried after review. Each was measured alone against the same binary
with it off (a scratch switch), seeds 1-48, 300 turns (6 civs, standard map); the great-person
change also in full games, the allied-barbarian one with the Clan of Embers forced.

### Young units never scrapped (removed on review)

1. Traced build-then-scrap churn: in the civs that scrapped most (seeds 3 and 5), 30-60 units a
   civ were trained in the windows they scrapped 8-34, and scrapped defenders were often 1-9 turns
   old (a Svartalfar bankruptcy scrapped one a turn while its cities trained new ones). The build
   rules (`production::decide`: a visible threat within 3 plots, a barbarian unit within 3-4) and
   the scrap rules (`finance::disband`, `empire::disband`: no threat in sight, a barbarian within
   4) both read the moment, so a raider walking in and out of range flipped both.
2. A unit trained in the last 20 turns was never scrapped for upkeep. Development (seeds 1-48):
   cities lost to the barbarians -24% (t -2.5), gold +8% (t 2.4), legacy +1.0% (t 0.8); validated
   only inside a combination with the great-person and explorer changes, which was neutral on
   fresh seeds (docs/AI.md, round 6).
3. Removed on the owner's review: the 20 turns is an arbitrary time constant on the scrap side,
   and the build and scrap rules still disagree. The right version is one notion of what a city
   needs that both sides read -- see the next entry for why that notion cannot be built from
   lasting map features alone.

### Garrison quota from lasting threats only (reverted)

1. The owner's rule: a city's defenders follow what it needs in expectation, not a barbarian
   wandering into range; what is needed is built and kept whatever it costs (research pays),
   what is not is not built. One quota for production, finance and the bankrupt rule: one, two
   in the capital, one more within 4 plots of a barbarian lair or town, at war, for a defensive
   leader, the victory guard; no affordability cap; nothing seen this turn (no +1 for a threat in
   sight, no barbarian unit counted). Above it only step 1's overwhelmed garrison trained more.
2. The churn went (seed 3: defenders trained 183 -> 78, units scrapped 110 -> 42, scrapped under
   10 turns old 30 -> 3) and the cities fell (seeds 1-48, 300 turns): military -18% at turn 100,
   -22% at turn 200 (t -11); cities -6.6% at turn 100 (t -3.9), -5.1% at turn 300 (t -2.4); cities
   lost to the barbarians +73% (t 2.5), razed +112% (t 3.7); civs eliminated 0.33 -> 0.58 a game
   (t 1.9); legacy -2.8%, -4.1%, -3.0%.
3. The raiders in sight are not noise: barbarians spawn in the unowned land around our frontier
   and roam, and lairs and towns account for few of them, so a quota from lasting map features
   undercounts the expected threat badly. The principle holds, but "in expectation" needs an
   estimate of how often raiders come by each city, which the moment cannot give and the map
   alone does not: a per-city record of the threats seen (a field on `City`, saved with the game:
   the turn and strength of the last threat, or a decaying average), read by both the build and
   the scrap rules.

### Garrison quota from a record of threats seen (reverted)

1. The entry above implied it: "in expectation" needs an estimate of how often raiders come by
   each city. `City::ai_threat_record` (saved) was the owner's AI record of the threats seen:
   once a turn it kept (D-1)/D of itself and gained 100 when an enemy (not an animal) was in
   sight within 3 plots. One quota for production, finance, the bankrupt rule and the defenders'
   placement (`units::garrison_quota` never below it): one, two in the capital, one more near a
   lair or barbarian town, one more once the record reached a first bar and two at a second, war,
   a defensive leader, the victory guard; trained whatever it cost, never scrapped below it
   (keeping at half the training bar, so a defender trained for raids outlasted the lull).
2. Every variant lost against master's quota (seeds 1-48, 300 turns; legacy / research at turn
   300, t in brackets; cities lost to the barbarians per game, master 1.75):

   | record (D, first bar, second bar) | legacy | research | lost to barbarians |
   |---|---|---|---|
   | 20, 150, 400 | -4.2% (-1.5) | -10.0% (-2.2) | 1.79 |
   | 20, 100, 300 (one sighting trains) | -6.4% (-2.4) | -9.7% (-1.9) | 2.50 |
   | 40, 150, 400 (longer memory) | -7.2% (-3.1) | -12.4% (-3.1) | 1.96 |
   | 20, 150, 400, master's affordability cap kept | -5.1% (-2.0) | -10.3% (-2.5) | 1.62 |
   | the same without the placement floor | -3.6% (-1.4) | -3.9% (-0.9; -7.5% at turn 200, t -2.1) | 2.04 |

   The longer and the more firmly defenders were held, the worse; none lost fewer cities to the
   barbarians. Keeping master's affordability cap changed nothing (the cap rarely binds); the
   placement floor (a defender never leaving a city below its need) cost a third of the loss --
   spare defenders no longer walked to the city under attack -- and the rest is the upkeep of
   the defenders held between raids.
3. In this game a defender is cheaper to rebuild than to keep: a Warrior is 25 hammers, its
   upkeep is paid every turn it stands. Training one when raiders are in sight and scrapping it
   when the upkeep bites is a reasonable rent-or-buy choice, not a bug, and holding garrisons
   by an estimate of the expected threat does not save the cities that would pay for them. If
   the build-and-scrap pattern is attacked again, the lever is the cost of the reaction (train
   the cheapest adequate defender, or move one from a city with a spare), not holding units.

### One garrison quota for production and placement (2026-10-28, reverted)

1. A review found two quotas that disagree. Production's (`production::decide`: one, the capital,
   a threat in sight within 3 plots, a lair or barbarians within 3, the rival defenders, a
   defensive leader, the victory guard; the second defender after turn 30 near barbarians within
   4) and placement's (`units::garrison_quota`: one, the capital, the danger map -- what can
   strike the city next turn --, the rival defenders). A defender trained for a term placement
   lacks is spare there and walks to a needier city while its city trains another. Traced (seeds
   1-12, 300 turns, 72 civs): of 1757 defenders trained and 1266 walks from one of our cities to
   another, 248 left their city below production's quota and 162 of those were followed within
   15 turns by a defender trained there; 480 walks in all were followed by one (the first of them
   a second defender in 196). The suspected terms were not the cause: the defensive leader term
   never fires (no FFH2 0.41 leader has `iAIValueDefense` above 100), the victory guard was on 0
   of the 1757 trainings, a lair behind 11 of the 248. The gap is the two threat models: in 167
   of the 248 the city saw barbarians within 3 plots that could not strike it next turn, and 187
   of them walked to a city that enemies could strike next turn. `defense::quota(g, c, danger,
   rival)` (switch `GARRISON_QUOTA`) was one quota for both: the union of the threat terms
   (unless the garrison holds the raiders), the lair and barbarian terms, the second defender as
   a floor of two, the rival defenders, the treasury cap, the victory guard; placement capped at 5.
2. It did not stop the walking. Traced (seeds 1-12): walks 1266 -> 1663 (more cities now asked
   for a defender, so more spares walked), walks followed by a defender trained in the city left
   480 -> 676, defenders trained 1757 -> 1707. Eight forks at walks that left a city below the
   one quota (seeds 1-4, the game played with the old quotas to the fork, 30 turns): in seed 1
   turn 111 (Luchuirp) both branches lost the threatened city to barbarians four turns later; the
   old placement retook it at turn 118 with the defenders it had sent, the one quota kept five
   in the sending city and never did (2 cities against 4 at turn 141); in seed 2 turn 170
   (Ljosalfar) the city the one quota held its defenders in fell at turn 196 while the old
   branch kept all five cities; the other six changed no city. Games, the same binary with the
   switch off (6 civs, standard, 300 turns): development seeds 1-48 legacy@300 +1.3% (t 0.59),
   share@300 -1.2% (t -1.43), cities lost to the barbarians -33% (t -2.1); fresh seeds
   47001-47048 legacy@300 +0.4% (t 0.24), share@300 -0.3% (t -0.37), cities lost to the
   barbarians -13.5% (t -0.74).
3. The disagreement is triage, not churn: a city that sees raiders three or four plots out gives
   its spare to the city they can strike next turn and trains a replacement, which is what the
   previous entry calls the cheap reaction (move one from a city with a spare). One count for
   both sides makes the city that only sees raiders hold the defender the struck city needs.
   It was reverted for that reason -- the concept was wrong, not its implementation: production's
   quota is how many defenders a city builds and keeps over the coming turns, placement's is where
   one must stand now, and the forks above lost cities through exactly that conflation (the share
   figures were not the ground). If
   the two are reconciled again, the shared notion is urgency, not a count: placement ranks
   cities by what can strike them against their garrison (`View::peril`), and production knows
   it trains reserves for the cities around it. Noticed then, fixed since (a bug, kept whatever
   it measured): in placement's search for a needy city (`units::garrison_unit`), a city whose
   defenders already took their orders this turn counted them twice (committed and present), so
   spares walked mostly to cities whose garrisons came later in the turn's order. Counted once
   (seeds 1-48, 6 civs, 300 turns, against the double count): cities lost to the barbarians
   128 -> 104, cities captured by civs 225 -> 194, civs alive at turn 300 273 -> 279,
   population +2.3%, production +4%, leader share and legacy score unchanged.

### Settlers under a threat the garrison holds (reverted)

1. `expansion::settler_choice` trains no settler while any visible enemy is within 3 plots,
   whatever the garrison (its `defense` argument is unused), while the rest of
   `production::decide` lets a threat the garrison holds twice over pass. In seeds 1-5, 100 of the
   619 capital decisions before turn 150 had such a held threat, the capital at or near its
   happy cap at size 3+, and went to something else (39 Warriors, 21 Workers); seed 3's Sidar sat
   at size 6 from turn 52 to 85 with 4 defenders against a threat of 293 and reached turn 100 with
   one city and 14 military units; 19 of 94 civs had one city at turn 100.
2. Training settlers under a held threat did found more cities early: +9.8% at turn 100 (t 4.3),
   +5.3% at turn 200 (t 2.5). They were smaller (3.67 -> 3.46 population a city at turn 100), the
   costs rose from 34% to 37% of the commerce at turn 200 (maintenance, civic upkeep and unit
   support all up), the research slider fell (61 -> 55% at turn 200, t -3.6) and more cities fell
   to the barbarians (80 -> 95 by turn 200, 121 -> 133 by turn 300). By turn 300 the cities were
   back to the same number (-2.3%) and techs were -7.0% (t -2.3), legacy score -5.1% (t -1.9).
3. The same result as the faster openings in docs/AI.md ("Settlers and escorts", "Tried and
   dropped") and the sparse-world note: in this game, cities the economy and the garrisons
   cannot carry yet are a cost, not a lead, and the barbarians (whose cities the rules create
   in step with ours) take the extra ones. The idle capital is a symptom of the opening's real
   bind -- the maintenance a second and third city brings before the techs that pay for it --
   so the lever is the economy that supports cities (Code of Laws / City States timing,
   Courthouses), not the settler gate.

### Allied barbarians no longer counted as a threat (reverted)

1. The Clan of Embers and Charadon's Doviello are at peace with the barbarians, but
   `production::barbarians_near` (an extra defender in the city's quota, the "second defender
   first" rule, finance's garrison keep) and research's military pressure counted barbarian units
   all the same (docs/AI.md listed it as a known gap). At turn 300 (seeds 1-8) the Clan had the
   most military per city of any civilization (6.7, against 1.6-5.3), unit support at 25% of its
   commerce and research at 40%.
2. Across all civilizations (seeds 1-48, 23 games changed) it looked good: legacy +2.3%, cities
   lost to the barbarians -31% (t -2.1). With the Clan forced into slot 0 (full games, seeds
   1-24, `civ_ab.py`): its finances improved as intended (unit support 22 -> 15 gold and the
   research slider 41 -> 54% at turn 300; Longbowmen 26 -> 17, Archers 11 -> 5 at the end), but
   it lost twice as many cities to its rivals (0.95 -> 2.25 by turn 300, 1.9 -> 3.3 by turn
   400), held fewer (11.7 -> 10.6 at turn 300, 14.5 -> 11.8 at turn 400), and won less: 58% ->
   42% (won by one side only 4 / 8), share@end -498 (t -1.40). The other civilizations' gain was
   the Clan's loss.
3. The barbarian term is the only garrison term that grows with a frontier, and for the Clan it
   was standing in for the missing one: the threat of neighbouring civilizations (the Clan, at
   peace with the barbarians and strong, is attacked often). The right version is a garrison
   quota that weighs rival civilizations' land power near the city (what a player watches
   along a border); with that in place the barbarian term can drop for barbarian allies.

### Worker demand only where a build is possible now (reverted)

1. Workers had nothing to build in 8-45% of their turns before turn 100 (seeds 1-5; a Malakim
   pair waited from turn 40 to 83 while Bronze Working was researched), and 27% of the hammers
   before turn 100 went into workers. `production::worker_target` counts every worked plot
   without an improvement as demand, including plots no known build can improve, so cities
   trained a second worker for land the first could not touch yet.
2. Counting only plots where a build is possible now cut the workers at turn 100 by 15% (2.11 ->
   1.79 a civ) and with them the improvements: worked plots improved per citizen 0.71 -> 0.62 at
   turn 100, 1.01 -> 0.94 at turn 200. Research fell at every checkpoint (-3.7%, -5.1%, -3.7%),
   techs at turn 300 -3.7% (t -1.2), legacy -2.4% (t -0.9); nothing gained.
3. The idle worker is the one standing ready when the tech arrives; its waiting turns are the
   price of improving the land the turn it can be. Same conclusion as gating the first worker
   (docs/AI.md, "Workers"): the early worker count is not where the waste is. If the waiting is
   worth reducing, the lever is research timing (the techs that give the waiting workers work),
   not fewer workers.

### Stuck workers take their next best job (reverted)

1. A worker's build target is chosen by this AI's danger map, but the move there
   (`MOVE_SAFE_TERRITORY`) can be refused by the rules' AI path finder (a non-combat group next
   to danger, C++ `pathValid`), and the worker asked for the same job again next turn without
   moving (docs/AI.md listed it as a known gap; 44-151 such worker-turns in 300-turn games).
   On a refused move the worker took the next best job it could walk to (up to four tries).
2. The repeats went (19 -> 0 and 6 -> 0 in two 200-turn games, seeds 2 and 5): they were rarer
   than the first count suggested. The workers switched jobs far more often (build orders 560 ->
   879 in seed 2), and the economy came out slightly worse (seeds 1-48): production -1.8% (t
   -2.1) and research -1.9% (t -2.2) at turn 100, production -3.9% (t -1.6) and legacy -1.0% at
   turn 300.
3. A refused move is usually a passing danger; waiting a turn for the best job costs less than
   walking to a worse one. A version worth trying would switch only after the same move has
   been refused for several turns (the group's mission plot could carry the job), but the cost
   it removes is small.

### Fog-busting posts (2026-10-22, reverted)

1. Barbarians and animals appear only on land no civilization sees, at least two plots from any
   unit (`create_barbarian_units`, `create_animals`: `RANDPLOT_NOT_VISIBLE_TO_CIV`,
   `MIN_BARBARIAN_STARTING_DISTANCE`), so they appear just past the ring our borders light, a
   turn or two from our cities; players keep that ring lit with a warrior on a hill. A lone army
   unit with nothing to do (the step before Gather in `military::army`) went to the plot within 5
   turns, out of sight of every enemy, that blocked the most unseen land within 6 plots of our
   cities (weighted by closeness, good ground preferred), and stayed there under a mission AI
   tag the gathering stacks left alone.
2. It lost on every measure (seeds 1-48, 300 turns, against the same binary with it off): cities
   -6.1% (t -3.1), workers -7.7% and improvements -9.0% at turn 100; research -14.7% (t -3.5) at
   turn 200; legacy -2.2%, -7.0% (t -3.4), -8.2% (t -3.0) at turns 100/200/300; cities lost to
   the barbarians 1.75 -> 2.40 a game, civs eliminated 0.33 -> 0.56 (both t 1.7). A probe over
   seeds 1-12 to turn 150 counted what it did. Posts were few (0.6 a civ on average, mostly
   Warriors) and blocked little: new barbarian units appearing 4-6 plots from a civ city went 455
   -> 415 (-9%), beyond 6 plots 508 -> 530, within 3 plots 185 -> 247 (lairs and towns spawn
   there; not traced further). Barbarian units within 3 plots of a civ city, summed over turns,
   rose 5% (16962 -> 17858). Barbarian attacks that destroyed a unit rose 20% (486 -> 585); 111
   posted units were lost. The posts were taken from units that had been holding in a city:
   idle units sent to the nearest city fell from 949 to 174 decisions in seeds 1-4 to turn 120.
   The garrison count does not include those units, but they fight for the city.
3. The premise is weak in this game. A landmass's barbarian count is fixed (its unowned plots /
   50, minus the barbarians already there), so blocking moves spawns, and the barbarians
   walk to cities from wherever they appear. Only the 4-6 plot band (40% of new barbarians,
   about 6 a civ in 150 turns) is within a post's reach. With every civ posting, spawns only
   move between neighbours, so an all-AI A/B shows the cost alone. A one-civ test (the change
   for player 0 only, as `civ_ab.py` does for civilizations) would measure the relative gain
   a player gets by pushing spawns onto neighbours, but those counts bound it to a few
   barbarians a game. The real cost was taking idle units out of cities, which, as the garrison
   entries above found, hold more of the AI's thin city defense than the garrison count shows. A
   retry should post only units a city can spare. The posts must also be a function of the
   saved state: this version made the quick bench's bare loop and session play different games
   (`the_quick_bench_plays_the_same_game_three_ways`; not diagnosed).

### More Naval AI's disband rule (2026-10-22, reverted; brought back the same day)

1. The owner's principle (keep what the empire needs and let research pay, rather than react to
   a raider in sight), in the form More Naval AI uses (C++ `CvPlayerAI::AI_doMilitary` /
   `AI_disbandUnit`): at peace, while costs take more than 60% of the commerce ("funded" below
   40%), disband the military unit of least value -- combat units with at most 1 experience, field
   units outside our borders first, city defenders last -- reading no threat. It replaced
   `finance::disband`'s military part and `empire::disband`'s bankrupt path (scrapping from 10% of
   the commerce, garrisons beyond one per city where no threat was in sight), which made cities
   train defenders the treasury scrapped a few turns later. It did end the churn: military units
   scrapped 61 -> 6 in seeds 1, 2, 4, 6 to turn 200, units trained unchanged (608 -> 611).
2. On the base before merging master's late-October AI work it looked neutral: seeds 1-48 (300
   turns) legacy -0.9% at turn 300; seeds 12001-12096 -1.6% (t -2.0), cities -2.6% (t -3.0); full
   games 1-24 +1.5% / +5.9% at turns 300 / 400, 12101-12148 +4.0% (t 2.2) / +2.5% / +3.4% at
   300 / 400 / 500. On the merged code (war staging, vassals, chopping) it was not: seeds 1-48
   legacy -3.4% at turn 300 (t -2.1), techs -4.8% (t -2.6); full games 1-24 -1.0% / -2.7% / -4.0%
   / -0.9% at 200-500; seeds 13001-13192 -1.0% (t -1.1), techs -1.4%; full games 13201-13296
   0.0% / -1.4% / +1.1% / -1.4%, research -5.7% at turn 200 (t -3.05). Every set kept 4-8% more
   military (t 2.4-5.5), and the units bought nothing: cities lost to the barbarians +18% (t 1.7)
   in one fresh set, -10% (t -0.8) in the other, civ cities taken unchanged.
3. In this game a unit kept at peace is upkeep for nothing: the garrison entries above found the
   same for held defenders, and this finds it for every unit the old rule would have scrapped.
   The reactive build-and-scrap, churn included, is the cheaper policy. A version worth trying
   would attack the churn's cost at the build end (do not train against a raider the garrison
   already holds; production.rs's +1 for a threat in sight), not keep the units. MNAI pairs this
   rule with a cap on unit spending at training time; ours already gates optional units on
   affording them at 60% research, and a 12% spending cap was tried with no gain.
4. **Brought back by the owner's decision** (round 7d, docs/AI.md; switch `KEEP_UNITS`, on by
   default), and its cost worked on at the build end. Measured against the same binary with the
   switch off (development seeds 1-96, 300 turns; t in brackets; fresh seeds in docs/AI.md,
   round 7d). The rule alone, restored as it was: research -3.9% at turn 200 (t -1.9), military
   +4.9% (t 3.7), unit support +12.6% (t 3.0), legacy -1.0% at turn 300. With the kept build
   side (below), in 16 games with telemetry (seeds 1-16, 300 turns): military units disbanded
   859 -> 258, two thirds of them Warriors, in bursts when costs pass 60% of the commerce; the
   player-turns with support above a tenth of the commerce and research below its target 849 ->
   1504. What was tried on the build side:
   - **No defender against a raider the garrison holds** (the +1 a threat in sight and a
     barbarian unit within 3-4 plots add to the quota), the visible threat no more than the
     garrison's defense. In every city and against any threat (seeds 1-48): military -17% at turn
     100 (t -8), cities -6.8% at turn 300 (t -2.8), cities razed 0.38 -> 1.23 a game (t 4.3); the
     free early units are the ones the raiders meet. Only once units cost gold: research +3.0%
     at turn 200, but civ cities captured +31% (t 2.0) and razed +117% (t 2.6); a civ threat in
     sight counted as held. Only against barbarians (no enemy civ unit within 3 plots), seeds
     1-96: research +5.5% at turn 200 (t 2.6), military -4.3%, unit support -11.8%, but cities
     lost to civs 391 -> 472 and civs eliminated 39 -> 51 over 96 games, techs -2.3% at turn
     300. The defenders trained against raiders are also the ones that hold a border city when a
     neighbour attacks (the same finding as the Clan of Embers' garrison term above).
   - Kept: the same only in interior cities (no other civilization's city within 8 plots):
     research +0.7% / -1.7% at turns 200 / 300, military -1.0% / +1.6%, unit support -1.0% /
     +0.9%, legacy -0.9% (t -1.2) / -1.1% (t -0.8), civs eliminated 39 -> 41; it removes the
     extra military the rule kept at turn 200 and most of its research cost, little else (most
     raiders show up at the frontier, where the border cities are).
   - **More Naval AI's cap on unit spending** at training time (peace: 20 + 4/3 of the leader's
     `iBuildUnitProb` + 6 per mil of the commerce, `AI_maxUnitCostPerMil`) on every optional
     unit: research -1.3% at turn 200 instead of -3.9%, but cities -2.5% (t -2.8) and -2.4% (t
     -2.2) at turns 200 / 300 -- it blocked the cheap units that escort settlers and halved the
     peacetime campaign force against barbarian towns (431 -> 218 trained in 16 games); with the
     campaign exempt the cities still fell (-2.0%). Dropped, as the 12% cap was before. Flat caps
     of 8% and 12%, a war-time extra defender only within 10 plots of an enemy city, a worker
     instead of a unit when the fallback cannot afford one, and disbanding from 50% instead of
     60% of the commerce were all within noise (seeds 1-48, legacy -1.1% to -2.6% at turn 300).
   Under every variant the AI took more barbarian towns and lost more cities to the barbarians
   (taken 297 -> 318-338, lost 233 -> 258-279 over 96 games): kept armies take towns they cannot
   hold, the siege operations' known pattern (docs/AI.md, "Measured together").
   On fresh seeds (the kept version against it off): 300 turns, 23001-23192, legacy -0.5% /
   +0.4% and research +0.6% / +1.8% at turns 200 / 300, military +0.4% / +0.2%; full games,
   23201-23296, research +4.1% (t 1.7) and +2.4% at turns 200 and 300, legacy +0.3% to +1.9%
   through turn 400. The cost left is late: research -1.2% at turn 400 and -5.7% at turn 500 (t
   -1.3, the 40 games still running), research slider -7.8% (t -1.8). It follows the wars the kept
   armies start: civs at war at turn 400 +50% (t 2.1), military +6.9% (t 1.9), unit support +8.6%;
   civ cities captured +11% and cities lost to the barbarians +20% over the games, time-limit games
   6 -> 10 of 96, cultural victories 34 -> 31.
   What it implies: the reactive build-and-scrap was cheap partly because the reactive
   defenders were the AI's only standing defense against its neighbours. Keeping units costs
   little once interior cities stop buying defenders against raiders they hold; the lever left
   is a garrison term for rival civilizations (docs/AI.md, "More Naval AI compared", item 2), with
   which the border cities could stop reacting to raiders too. The late cost is the war gate's:
   a kept army passes the power-ratio test more often, and the wars it starts run the slider at
   the war target. Weighing the army's upkeep in the gate, or the peace terms (item 3 of the same
   list), is where to work on it, not the disband threshold (disbanding from 50% of the commerce
   instead of 60% changed nothing at turn 300).

### Peacetime defenders by a rival-threat threshold (2026-10-22, replaced 2026-10-24)

1. Round 7b's `RIVAL_DEFENSE` gave a city one more defender in peacetime where More Naval AI's
   threat index (a rival's closeness x attitude x its power over ours) reached 40, two at 100.
2. Inside the merged round 7 build it measured legacy -3.2% (t -3.8) and research -7.2% (seeds
   26001-26192, 300 turns), and was switched off by default. Measured again on the same seeds once
   `HEAL` and `PRECIOUS` were off, it was neutral: legacy -0.4% (t -0.55), research -0.3%; and
   none of its parts carried a cost alone (the peacetime defenders -0.5%, the war pool +0.4%, the
   sortie check -0.7%, all |t| < 1.1). The earlier cost was an interaction with the other two
   military behaviours, not this rule's.
3. The threshold had no reason in it: it added defenders by how close and unfriendly a
   neighbour was, whatever our army already deterred. It was replaced by deterrence, sized by the
   gate every AI here uses to choose a war (docs/AI.md, "The standing army against rival
   civilizations"); neutral as well, and the rule an expert can explain. Lesson for measuring:
   switches measured inside a build with other new switches on can carry each other's costs;
   re-measure a behaviour alone once its neighbours are settled.

### Healing only when most of the stack is hurt (2026-10-22, replaced)

1. Our field stacks stopped to heal wherever they stood as soon as one unit passed 35% damage,
   committed stacks next to their target included, where they heal at 5-10% a turn beside
   defenders that heal faster. More Naval AI's `CvUnitAI::AI_heal` stops a stack in the field only
   when more than half of it is hurt, and only where healing is good.
2. Applied to every stack it lost units and cities (seeds 1-48, 300 turns): civ cities lost +41%
   (t 2.7), military -6.4% (t -2.1). Our loose stacks are two to four units hunting raiders; one
   badly hurt unit is a large share of such a stack, and it marched on into the next fight.
3. The rule fits More Naval AI's big stacks, not ours. The kept version (`military::heal`, switch
   `HEAL`; docs/AI.md, Field armies step 6) applies "most of the stack" only to a stack committed
   to a siege, keeps the one-unit rule for loose stacks, splits the wounded off in our cities and
   heals only where healing is good (walking into a city or onto our land when it is not).

## Wars: no peace sold before the war is decided (2026-10-21)

Item 5 of the strong-player list. In 24 full games (seeds 1-24) our civilizations declared 325
wars with armies of 23 units against a computed need of 10; 38% took a city, 22% saw no combat
at all, and in seeds 1-8 about 40% of the wars we declared ended in our own "cash out": a
winner's peace for tribute at turn 11, the first turn peace is allowed (traced, seed 2: the army
marched 19 plots for 11 turns, never arrived, and the war was sold for one tribute item).

- **No cash-out before a city or 30 turns.** 1: an expert does not sell a war of conquest
  before its army has fought for its first city. A war we declared could be sold for tribute
  only once it had taken a city from them or lasted 30 turns (losing wars still ended as before).
  2: everyone, full games: wars 448 -> 383, captures 423 -> 423 (a city per war 0.94 -> 1.10),
  every checkpoint flat; one civilization against the rest (114 pairs, seeds 1-6): wins 22.8%
  -> 19.3% (0 / 4 pairs, p 0.125), legacy@300 -23 (t -2.0), share@end -25. 3: the tribute this AI
  takes after ten turns (technologies, gold) is worth more than the extra turns of a war it wins
  38% of the time; the expert's test is the city the operation will take soon against the
  tribute on offer, which needs the operation's own estimate (its phase, the force near the
  target), not a turn count. Kept instead: declaring only once the army stands near the target
  (docs/AI.md, "Diplomacy"), which shortens the march the cash-out cut short.

## World spells: For the Horde at a dozen orcs, Ardor after three great people (2026-10-21)

Item 4. FfH's condition for the Clan of Embers' For the Horde (60 barbarian units of any kind)
looked like it almost never held (1 cast in 10 unforced games), so the expert moment was a
dozen barbarian orcs to turn, with the Clan's own rule (only with a war to fight) kept. 2: with
the Clan forced into every game (seeds 1-24, full games) FfH's condition held in 14 of 24 games
already -- the Clan is at peace with the barbarians, whose units pile up -- and the earlier cast
(21 of 24) cost: wins 54.2% -> 41.7% (0 / 3), share@end -425 (t -2.0), legacy@end -450 (t -2.3).
3: half of a dozen orcs is a few units the Clan pays upkeep for, cast before the barbarians have
grown into the horde the spell is named for; FfH's condition is the expert moment here.

Ardor (Grigori: the great-people count and threshold start over) after the third great person
instead of FfH's fifth: development seeds flat (wins 16.7% -> 20.8%, share@end +21); fresh seeds
9601-9624 share@end -221 (t -1.1), legacy@end -210 (t -1.0), casts 16 -> 21 games. FfH's
condition already held in two games of three, and the spell is worth more the later it comes
(it resets a higher threshold); if anything the expert waits longer, not less.

## Sanctuary and March of the Trees at a losing war (2026-10-22, off by default)

1. More Naval AI casts both defensive world spells only when the civilizations at war with it
   have 1.5 times its power (`reqSanctuary`, `reqMarchOfTheTrees`), and the round-5 entry below
   ("World spells at moments of their own") names a losing war as the right trigger: a 30-turn
   lockout (Sanctuary) or a Treant on every forest at the price of the Ancient Forests (March of
   the Trees) are last stands. FfH's own condition is two wars at once, which says nothing about
   who is winning. Kept behind `WS_LOSING_WAR` (`world_spells::losing_war`, `city_in_peril`),
   off by default.
2. Measured with the civilization forced, development seeds 1-24, full games, the switch on
   against the same binary with it off; four versions, each traced where it lost:
   - the power ratio alone: Elohim win 12.5% -> 12.5%, share@end -162 (t -1.69), legacy@end -176
     (t -1.55); Sanctuary in 17 games instead of 14, first seen at turn ~358 instead of ~441;
   - with a city whose threat exceeds its garrison: share@end -127 (t -1.12), legacy@end -196
     (t -1.84), alive -0.1 (t -1.81);
   - with four or more units of a civilization at war at that city: share@end -153 (t -1.74),
     legacy@end -189 (t -1.97);
   - with the capital or two cities so threatened (the last version): Elohim win 16.7% ->
     16.7%, share@end -146 (t -1.42), legacy@end -175 (t -1.42), alive t -1.81; Ljosalfar win
     33.3% -> 29.2% (1 / 2), share@end -34 (t -0.31), legacy@400 +48 (t 1.27).
   Every version moved the one-shot spell and lost it where it mattered. Seed 21: the Luchuirp,
   stronger, sent a raid of three or four units at turn 138-145 and every version cast Sanctuary
   then (the city held without it in the game without the change, its warriors killed the
   raiders); their invasion of turns 210-300 took nine Elohim cities, where FfH's condition had
   cast it (turn 272) and lost two. Seed 7: the last version waited for the second city in peril
   and cast at turn 396, ten turns after FfH's two wars (turn 386), when two cities had already
   fallen; the Elohim were eliminated by turn 500 instead of surviving with 19% of the world.
3. "We are losing" is right, but the moment needs what the invasion will take: a raid on a city
   the garrison holds is not it, and a city that has already fallen is too late. The expert reads
   the size of the force coming against what we can put in its way -- the enemy stacks heading for
   our land against our army there -- which this AI does not estimate yet (the defense gap of
   docs/AI.md, "More Naval AI compared", item 2: threat weighed by rival civilizations). Until it
   does, FfH's two wars, a proxy for "the whole neighbourhood is at us", did better on these
   seeds. The rule and its test stay (`FFH_AI_WS_LOSING_WAR_ONLY=ALL` turns it on).

## Unit choices and an upgrade reserve (2026-10-21)

Item 3 of the strong-player list: the head-to-head games had the original AI fielding 3.9 axemen
and 2.4 swordsmen a civilization at turn 200 against our 0.8 and 0.4, and banking 438 gold
against our 62 (it keeps the gold to upgrade its units). By turn 300 our civilizations had built
12.2 warriors each, 122 of those in turns 200-300 as peacetime army fill (seeds 1-8). Three
behaviours, each behind a per-civilization switch, measured for everyone (seeds 1-48, 300
turns) and as one civilization against opponents without it (`civ_ab.py --civ all`, seeds 1-6,
450 turns, 114 pairs):

- **Upkeep in a unit's price.** 1: FfH units beyond the free ones cost gold every turn (unit
  support was 35-48% of the costs at turn 300), so an expert buys strength per total cost:
  `best_unit` scored strength squared per (build turns + 4 + 40 turns of upkeep as hammers).
  2: warriors 12.2 -> 10.5 a civilization, archers 4.4 -> 3.1, more champions; everyone: flat
  (cities lost to barbarians -0.6 a game, t -2.0); one civilization: wins 23.7% -> 17.5% (2 / 9
  pairs, p 0.065), share@end -70 (t -1.0), the Clan of Embers worst (share@end -676, t -2.2: its
  Warrens copy every unit, so fewer, dearer units lose it copies). 3: the quotas count units
  (defenders a city, attackers an army), so dearer units at the same count put more hammers into
  the army, not fewer units; "fewer, better units" needs quotas in strength, not in heads.
- **Gold kept for upgrades at war** (C++ `AI_goldTarget` counts the cost of upgrading every
  unit): while at war or preparing one, the research slider (never below 30%) saved toward the
  price of upgrading every military unit that had a better unit to become. 2: upgrades 9.5 ->
  10.2 a civilization by turn 300; everyone: flat; one civilization: wins 23.7% -> 19.3% (6 / 11,
  p 0.33), share@end +4, legacy@300 -17 (t -1.4). 3: an upgrade costs 3 gold a hammer plus 20,
  about what hurrying costs; it pays for veterans and emergencies, not as the way to field an
  army, and the research it takes during war preparation is not won back.
- **The peacetime army from the cities that train good units**: a city whose best attacker was
  under 75% (then 90%) of the best any of our cities trains (a Training Yard's axeman against a
  warrior) built no peacetime army units. 2: at 75% it never fired (a warrior is exactly 75% of
  an axeman); at 90% warriors 12.2 -> 10.9 and buildings +0.8 a civilization by turn 300;
  everyone: flat; one civilization: wins 23.7% -> 19.3% (5 / 10, p 0.30), share@300 +33 (t 1.7),
  share@end -7. 3: the warriors persist where no city has the enabling building: tier-2 units
  need a Training Yard or Archery Range in the training city, and `enabler_building` builds one
  in at most half the cities; the lever, if any, is which cities get them (military cities),
  not refusing to train.

## Expansion: the expert opening (2026-10-21)

Reopened under the expert-play rule (docs/AI.md, "Settlers and escorts", had dropped faster
openings in 2026-10 for costing research). Four parts, behind a per-civilization switch so they
could be measured for everyone (seeds 1-48, 300 turns) and as one civilization with them against
opponents without (`civ_ab.py --civ all`, seeds 1-8, 152 paired games):
(a) a threat the garrison holds (a lair guard next door) no longer stops a city building
settlers, the escort deciding when it leaves; (b) one settler in flight per city instead of one
per two; (c) the capital's second defender only once barbarians are within 4 plots; (d) no
buildings while a city grows toward its next settler.

1. **Why it made sense.** The second city came at turn 70 and the third at 105 (seeds 1-8). By
   turn 100 a civilization had spent 29% of its hammers on settlers, 30% on military (3.7
   warriors), 23% on workers and 15% on buildings; its first settler started at a median turn
   45; one capital (seed 1, the Grigori, a goblin fort two plots away) built none by turn 80
   because the lair guard counted as a threat, though four warriors held it.
2. **Why it did not pay.** All four, everyone: cities +0.6 at turn 150 (t 6.7), then -0.2 at
   300; techs -3.6 at 300 (t -4.5), research -16%, legacy -110 (t -3.3); cities lost to the
   barbarians +1.9 a game (t 3.2), barbarian cities +0.9. One civilization with them: wins 15.8%
   -> 16.4%, share@300 -52 (t -0.7), legacy@300 -74 (t -1.6) -- no competitive gain either. Part
   by part (everyone): (a) carries nearly all of it (cities lost to barbarians +1.8, t 3.0;
   improvements -1.9 at 150, t -4.8; techs -2.4 at 300, t -3.5): the cities a held threat let
   the city found were the ones next to lairs, and the settlers took the hammers the workers had
   had. (b), (c) and (d) did not bring cities sooner (cities at turn 100 +0.00, at 150 +0.19)
   and together cost techs for everyone (-1.8 at 300, t -2.7) while one civilization with them
   gained noise (wins 15.8% -> 17.8%, 16 / 13 pairs, share@300 +54, t 0.9). Where the faster
   opening's cities went: at turn 150 commerce +0.4 for +0.6 cities, while maintenance rose 1.6
   gold (t 5.4) and the research slider fell 7.6 points -- a size-1 FfH city costs more than it
   gives for dozens of turns.
3. **What it implies.** The pace is bounded by what a settler costs (120 food and hammers at
   6-9 a turn from a size-3 capital) and by FfH's early maintenance, not by the production
   ladder: freeing slots and hammers does not bring settlers sooner, and settling past a lair
   loses the city. The original AI expands slower still (4.4 cities at turn 200 against our 5.3,
   head-to-head games) and is richer (research 78 against 46, markets in 68% of its cities). The
   expert levers left are development, not pace: more commerce per new city (cottages, river and
   coastal sites, markets) and clearing a lair that blocks expansion (the lair guard on a fort
   beats warriors; it takes axemen or archers -- item 3, unit choices). A balance note: the rules
   start barbarian cities once civilizations average two cities and size them by the number of
   civ cities (`createBarbarianCities`), so any faster opening brings them sooner.

## Workers and citizens (2026-10-21)

The kept version (docs/AI.md, "Citizens" and "Chops") came out of six variants measured on
seeds 1-24 (300 turns) and validated on 401-472; the governor change was in all of them.

- **Every clearing counted** (workers v1-v3, v5): a farm or mine on a forest or jungle valued
  with the feature's lost yields on a worked plot and the hammers of the clearing (half again to
  a city rushing a settler, a worker or a world wonder, 20 turns' worth otherwise), the kept
  features (Ancient Forest, happy and healthy features, a city's last three forests) refusing
  the improvement as well as the chop, and in v1-v2 chops for any city. 1: the C++ AI and an
  expert both weigh what a forest gives against what replaces it; our workers ignored both the
  hammers and the loss. 2: research at turn 300 -15 to -18 (t -1.9 to -2.5), techs -1.4 to
  -1.8, population flat; workers alone (v1 without the governor change) population -3.5,
  research -11. The forests kept around cities blocked mines and farms on the plots the cities
  worked, and chops for buildings took worker turns from farms. 3: for this AI an improvement's
  yield gain matters more than the feature it replaces; chops pay only when they bring a city
  sooner. The kept v6 (chops only for settlers, workers, world wonders; improvements valued as
  before) measured +10 research on seeds 1-24 but flat on 72 fresh seeds: the spread between
  variants (research -18 to +10) is about what choosing among six variants on 24 seeds
  produces. Separating effects this small takes 48+ development seeds.

## Unique units (2026-10-20)

The kept version is in docs/AI.md, "Unique units". It is kept under the owner's rule (CLAUDE.md,
"AI changes"), so what its measurement shows is a cost to improve, not a gate. Back with it from
round 5 (below): the Amurite casters (now `civs::unique_casters`, also for the Svartalfar,
Doviello and Clan of Embers), the Illians' rituals, the Calabim vampire rules, the Grigori
Medics, the Infernal Manes beyond a garrison quota (point 3 of their entry), and the
building-gated units of the Khazad, Luchuirp and Lanun, now generic (`production::unit_building`).
Their entries stay as the record of what they cost then.

### An enabling building by strength squared per hammer (`48a0bf9`, reverted by `d9bee1e`)

1. With the Sculptor's Studio visible, the Luchuirp reached Construction (Wood Golems, 6) before
   Archery and built Studios from turn 150 (2.6 by turn 200 against 0.4). The Archery Range then
   looked only 14% better than the golems for their Dwarven Slingers (5 with a first strike, +25%
   in cities), under the 33% bar, so golems at 90 hammers held their cities (turn 300: 15 golems
   and 8.5 slingers, against 2.7 and 20) and research at turn 300 fell 211 -> 179. Weighed as
   `best_unit` weighs a choice, strength squared per hammer, the slinger is worth twice the golem.
2. Re-measured on five civilizations (Khazad, Kuriotates, Luchuirp, Malakim, Sidar; seeds
   8701-8720): squared strength per hammer overrates cheap units -- a Warrior (3, 25 hammers)
   outscores an Axeman (4, 60) -- so Training Yards were no longer built (turn 300: Khazad
   3.3 -> 0.9, Malakim 3.5 -> 1.5, Sidar 1.3 -> 0.3) and Warriors piled up (+3 to +6 a game in
   all five).
3. A building that opens a unit has to compare it with the city's best the way the role uses
   them, linearly: a third stronger, or a third stronger per hammer. The Archery Range then comes
   where golems hold a city (72% better per hammer) and the Training Yard where Warriors do (33%
   stronger). That is the kept rule; the Luchuirp's remaining cost is their army's, not their
   garrison's (docs/AI.md, "Unique units").

### Workers cast into Beastmen fight, without a gate (replaced the same day)

1. The Doviello's Worker -> Beastman (`SPELL_UPGRADE_DOVIELLO_WARRIOR`, cast by the rules' AI
   gate at war while they have as many workers as cities) keeps the worker's AI on the Beastman
   (`CvUnit::convert`). In 190 census games 558 Beastmen came from workers; they built nothing,
   sat in our cities, and production counted them as workers and trained none (Battlemasters, the
   same units cast on, spent 28% of their turns on the worker AI).
2. Giving them their own default AI (attack) let production train workers at war, and the cast
   took the new workers too: conversions 63 -> 79 and workers trained 80 -> 104 on the ten census
   seeds. With the other fixes (seeds 9201-9220) the Doviello survived less often: alive 0.7 ->
   0.5 (t -2.52), share@end -113 (t -0.62). The idle "workers" had been extra defenders in our
   cities.
3. Cast only on workers beyond production's own target (`production::worker_target`), so the
   cast never takes a worker production would replace. Against the same build without the fix and
   the gate (seeds 9201-9240): share@end -17 (t -0.20), alive flat, and from turn 300 1.3 more
   workers (t 2.9), improvements +4-10%, research +8-13%.

### Donal's Recruit in a city half as good as our best (replaced the same day)

1. Recruit is once, and a bigger city gives more units (population / 3 + 1).
2. Donal marches with the army: 5 of 9 Donals on the census seeds left their city and never
   recruited. And at two units a city, 3 of 9 still did not: a unit trained at the end of our
   turn first moves on the next, before the end-of-turn cast.
3. Recruit in the first city of ours giving two or more, at the start of our turn
   (`civs::bannor`): every Donal recruited (9 of 9).

## Spellcasting (2026-10-19, `magic.rs`)

The kept version is in docs/AI.md, "Spellcasting". Development measured each civilization forced
into slot 0, its spellcasting on against the same binary with `FFH_AI_MAGIC_OFF` for it (every
other civilization casting by `magic.rs`), seeds 1-20, full games (6 civs, standard map). These
variants failed there (the last one on a first fresh set, 8101-8120, spent by it) and were
changed before the final validation; the way each failed was found by replaying the seeds where
the two sides first split (`ffh autoplay --turns N` checksums bisect the turn, `FFH_AI_TRACE` and
`FFH_AI_TRACE_CAST` show the casts).

- **Summons at the end of the turn attacking where they take most hit points.** 1: a temporary
  summon dies at the start of our next turn anyway, so a hit point it takes from an enemy looked
  free whatever the odds; FfH's AI attacks with it only at 5% or better (`AI_summonAttackMove`),
  taking the best odds. 2: the Balseraphs' Loki casts Create Puppet every turn (FfH's weight
  1000) on his way to Disrupt a city; in seed 9 a barbarian Spectre shadowed him and his puppet
  attacked it every turn at hopeless odds, the Spectre then attacked the stack, Loki defended and
  lost 75 hit points, and hurt he was no longer sent to Disrupt (`civs/balseraphs.rs` gives up
  over 20 damage): no city taken by turn 110 instead of 2. Balseraphs win 45% -> 30% (3 / 6),
  share@end -456 (t -1.14), captures down from turn 200 (2.0 -> 1.6, t -2.3). With FfH's 5% floor
  but the target still chosen by hit points they still lost (45% -> 20%, 0 / 5; share@300 -208,
  t -2.13): fewer puppets alive at every checkpoint (turn 200 1.2 -> 0.9, t -2.3), and seed 8 split
  from the game without the change at turn 165 on a puppet's target alone. 3: a summon also
  defends what it stands with, and Loki's puppets are worth more as FfH plays them (the round-5
  entry on Create Puppet below says the same); summons attack as FfH's do
  (`casting::summons_attack`), and seed 8 is then the same game until turn 172.
- **Training casters for their spells** (two variants). 1: with spells cast where they help, a
  caster's worth could be read from them: one caster for every five field units when an adept
  could cast something worth taking to a fight with our mana, and the Mage Guild in our most
  productive safe city while no city could train one. The first variant also counted the second
  sphere promotions' spells (Fireball) at 3/4 for Arcane leaders (1/2 otherwise) once mages were
  within reach, and one more caster at war. 2: First variant: the Amurites' palace mana (Fire,
  Body, Metamagic) gives adepts Haste and Floating Eye (Blaze is not for the AI); their guild and
  one or two adepts came around turn 250 (casters at turn 300 0.1 -> 1.1) and cast nothing -- no
  Amurite cast in 100 turns of seed 9, where a one-city empire at war built the guild and an adept
  (the war's extra caster) and died at turn 338. Share@300 -16 (t -1.85), land at turn 350 -10 (t
  -2.1), research -22 (t -2.0), share@end -188 (t -1.56). Second variant, an adept's own spells
  only and no war bonus: only the Enchantment civilizations trained casters (Grigori, Illians,
  Luchuirp, Sidar: Enchanted Blade); 1.4 casters at turn 300 cost the Luchuirp a fifth of their
  research (t -1.7) and the Grigori a tenth by turn 350, the Illians and the Sidar had more army
  later, and against the same spellcasting without caster production (a scratch switch) the four
  lost wins: 17.5% -> 10.0% (3 / 9), share@300 -10 (t -1.5), share@end -4 (t -0.04). 3: a caster
  is hammers and upkeep paid around turn 250 for blessings and curses that pay only in the fights
  that follow, and this AI's late, small wars do not repay them (the third such result, after the
  Amurites and the Sheaim in round 5). Not kept: `magic.rs` trains no casters. Before trying
  again: magic earlier (research toward Knowledge of the Ether), or casters for a lasting summon
  (Raise Skeleton: an army unit a turn).
- **Skipping units that another unit's spell has just killed.** 1: a unit in delayed death
  should not act. 2: FfH's end-of-turn loop (`CvGame::updateMoves`) skips a unit only after its
  own cast killed it, so the second wolf a Form Wolf Pack consumes still casts Form Wolf Pack in
  its turn, forming a further pack from wolves already spent. Skipping it cost the Doviello packs
  (seed 13 split at turn 166 on that alone; share@end -250, t -1.83). 3: that is a C++ quirk of the
  AI loop, which `casting::end_turn` keeps for both paths (marked there); whether the AI should
  keep the extra packs is a separate decision, not part of the spellcasting change.
- **Casting nothing when nothing is worth casting.** 1: a cast that does nothing (Haste on a quiet
  turn, Dance of Blades where no unit can take it) is not worth a unit's turn, so the end of the
  turn cast only what had a value. 2: a cast is never quite a no-op -- it rolls for a miscast when
  the unit or spell has a chance of one, ends the promotions casting removes -- so every skipped
  cast split the game from FfH's: the Balseraphs' games split on a puppet's Dance of Blades on
  nobody (seed 10 at turn 85, seed 8114 at turn 193), and on fresh seeds 8101-8120 the first six
  civilizations went 1 / 7 in wins, share@end -85 (t -1.76), the Balseraphs -332 (t -2.29), most
  of it play that had nothing to do with spells after such splits. 3: when nothing is worth
  casting, the unit casts what FfH would: free, and the same game where the values see no
  difference (seed 8114 then plays the same through turn 300), so what a measurement shows is the
  change's own.

The kept version passed on fresh seeds 8201-8220: win 17.7% -> 18.5% (11 / 8), share@end +36
(t 1.39) (docs/AI.md, "Spellcasting").

## Round 5 (2026-10-18): civilization mechanics

Numbers are full games (6 civs, standard map, to a decision or turn 690); "dev" is seeds 1-40
against base `2d95e9c`, "fresh" is seeds 5001-5064 at `d4ff7e3`, the civilization's logic on
against off; wins as "seeds won by one side only (with / without)"; share is score v2 in basis
points (fair 1667); t is the paired t over seeds.

### Casters through a Mage Guild (Amurites `5c6fdbe`, Sheaim `4f00cf1`)

The most productive safe city built a Mage Guild once Knowledge of the Ether was known and
trained casters (Amurites up to (cities + 2) / 2; Sheaim at war, one per two cities, Death
promotions first). Amurites: dev 5.0% -> 7.5% (2 / 1), share@end +108 (t 1.06); fresh 4.7% ->
1.6% (0 / 2), share@end -25. Sheaim: dev 2.5% -> 2.5% (1 / 1), share@end +95 (t 0.79); fresh
0.0% -> 1.6% (1 / 0), share@400 -144 (t -2.17), share@end -88.

1. Both civilizations are built around casters (Arcane and Summoner leaders, palace mana), and
   the base AI had none: adepts need a Mage Guild in the training city, and no AI builds one
   (no yields, `iAIWeight` 0, value 0).
2. The guilds and casters came (Amurites 1.4 guilds and ~1.4 casters a game from turn 300;
   Sheaim 2.0 guilds, 1.3-1.6 casters from turn 400) and cast a lot -- 277 to 1185 casts a game
   in six traced fresh games -- but almost only buffs, from the ported `AI_BuffCast` at the end of
   every turn: Amurite Wizards and Adepts cast Haste and Regeneration (over 95% of their casts; in
   three games no caster cast a single Fireball), Sheaim mages Dance of Blades (65-75%), with
   Fireball 10-60, Summon Spectre 23-28 and Raise Skeleton 2-9 times a game. The Amurite casters
   summoned nothing (summons 0.0-0.1 alive at any checkpoint). The cost was real: at turn 400
   the Amurites had military strength -29 (t -2.1) and research -25 (t -2.1), 6.5 fewer
   buildings by turn 500 (the guild and caster hammers came out of Forges, Alchemy Labs,
   Courthouses, Moneychangers); the Sheaim military -25 (t -2.1), 0.9 fewer cities (t -2.4), and
   by turn 500 they had taken 1.8 -> 1.0 cities (t -2.8).
3. Casters are not worth their cost to this AI until it casts combat spells in combat. The port
   omits FfH's casting before a stack attacks (`AI_DirectDamageCast` and `AI_SummonCast` before
   `groupAttack`, docs/PORTING.md), so damage spells and summons happen only at the end of the
   turn when an enemy happens to be in range, and the buff cast fills every other caster turn.
   The correct version is generic, not per civilization: casters march with the strike stack and
   cast damage and summons right before it attacks, promotions chosen for that (Fire for
   Fireball, Death for Raise Skeleton), and only then does a Mage Guild have a value (the casters
   it enables), for every civilization. Not a balance verdict: FfH's casters are strong in human
   hands. (That generic version is "Spellcasting" above.)

### Warcry held for the strike (Hippus, `ebb1427`)

Warcry (every unit +1 strength, +1 move and Blitz, about 20 turns) was held until the war
operation stepped next to its target or struck, a city was in peril, or 20 turns of war. Dev
15.0% -> 22.5% (4 / 1), share@300 +91 (t 2.10); fresh 14.1% -> 10.9% (1 / 3), share@300 -1,
share@end -6.

1. The generic cast came the turn war was declared, and the diagnosis assumed the war operation
   spends about half its turns gathering, so the buff would be gone before the strike.
2. The assumption was wrong for this AI: it declares war after 30-40 turns of preparation and
   strikes within about 3 turns (seed 5003: declared at turn 217, strikes at 220 and 226), well
   inside Warcry's ~20 turns, so the cast at the declaration already covers the strike. In two of
   four traced seeds both versions cast on the same turn; where they differed (seed 5001) the
   generic cast fell on a war fought with 3 units that ended in peace after 11 turns, and the held
   cast, at turn 306, changed nothing either. Over 64 games captures, losses and military strength
   are the same at every checkpoint.
3. The cast turn is not where the Hippus' wars are decided. The one waste the traces show is the
   default casting at any war, even one with no army; with the timing now in the AI
   (`world_spells.rs`), "at war with an army ready to strike" is the natural default to measure.
   Warcry's Blitz goes unused by `campaign::strike` (it drops units once they have attacked), but
   letting Blitz units attack again was flat in development, so the buff is small for this AI's
   stacks.

### Ritual projects (Illians, `76c1ffd`)

The most productive city built The White Hand (Philosophy, 400 hammers: three Priests of Winter)
and Stir from Slumber (Drifa) as soon as it could. Dev 12.5% -> 17.5% (2 / 0), share@end +269
(t 1.90); fresh 15.6% -> 14.1% (2 / 3), share@end -40 (t -0.56).

1. The generic AI reaches projects only when no building has value left, so The White Hand came
   very late or never (0.0 a game), and Stir from Slumber is excluded.
2. It was built more often (0.1 -> 0.6 a game) but late, because Philosophy is late: in traced
   fresh games the project finished at turns 325-372, and the three priests then spent the game
   summoning Ice Elementals (8-38 summons a game) and nothing else. Stir from Slumber was never
   built (it needs Divine Essence and an eliminated player). No snapshot measure moved (military,
   cities, captures, research flat at every checkpoint): the gain on seeds 1-40 was noise.
3. By turn 325 three strength-5 priests and their elementals are worth less than 400 hammers of
   the empire's best city. For this AI The White Hand is weak for its price (a balance note: it
   is priced for an earlier game than the one in which the AI reaches Philosophy); it would only
   matter with earlier Philosophy or with the priests casting in strikes (see casters).

### Vampires, Feast, Gift Vampirism, River of Blood held (Calabim, `efb8d23`)

Vampires were set back from `UNITAI_FEASTING` to `UNITAI_ATTACK`; Feast was cast in a city above
its happy cap by units below 30 XP; Gift Vampirism valued by the Vampire promotion; River of Blood
held until a civ war or until our cities had room for its two citizens each. Dev 7.7% -> 7.7%
(2 / 2), share@end +109 (t 0.59); fresh 10.9% -> 4.7% (2 / 6), share@end -77 (t -0.61).

1. The rules put every Vampire on `UNITAI_FEASTING` (from C++ `AI_feastingmove`), which no part
   of this AI accepts, so vampires sat out of every stack; Feast (FfH's value -50) and Gift
   Vampirism (value 0) were never cast.
2. The mechanics barely fire: vampires come late and only through Moroi upgrades (traced: from
   turns 280-370, 61 gold each), Feast was cast 1-14 times a game and Gift Vampirism 2-4, and the
   vampires, once in stacks, spent their casts on Haste and Regeneration like the casters above.
   River of Blood's hold released early (turns 147-175 in traced games). No snapshot measure moved
   (research slightly up, everything else flat); the fresh 2 / 6 is noise around zero.
3. The Calabim's vampire play is gated behind Feudalism, Governor's Manors and Moroi upgrades,
   which this AI reaches late, and then needs the generic caster/strike play. The stance fix is a
   correctness fix with nothing to show while vampires are rare; it is worth re-measuring with the
   caster work, not alone.

### Manes added to cities (Infernals)

Manes in one of our cities with another defender there cast Add to City (+1 population); lone
Manes walked to the nearest city. Dev: 6 games where the Infernals appear, population 29.5 ->
64.5, share 1056 -> 1557 (t 2.21). Fresh: the 60 games of seeds 5001-5064 (all forced
civilizations) where they appear, replayed with the Infernal logic off: population +5.0 (t 2.23),
units -6.2 (t -2.77), alive 0.8 -> 0.7 (t -1.66), share -29 (t -0.68), legacy score -66; the
Infernal team won no game on either side.

1. Fallow: Infernal cities produce no food and never grow; Manes are their only growth, and FfH's
   spell value (-250) never casts Add to City.
2. The Manes did become population, but they were most of the Infernals' army: the rule kept one
   defender per city and converted the rest, and the Infernals, arriving at turns 300-350 into a
   world at war, had a third fewer units and survived less often.
3. Growth must not cost the Infernals their defense: convert only the Manes beyond a garrison
   quota (the generic defense requirement) or at peace. Six games could not show this; the
   protocol above now asks for a fresh check over at least tens of games where the mechanic
   fires.

### World spells at moments of their own (round 5, `0e4945b` on `scratch/ws-timing`)

After the world spells' AI-only gates moved into the AI (`world_spells.rs`), five spells got a
moment of their own instead of FfH's condition; each was measured with its civilization forced,
against the same binary with FfH's condition (`FFH_AI_WS_DEFAULT=SPELL_X`), development seeds 1-40
and fresh seeds 6001-6064. Stasis as soon as the Illians have a city won big (fresh win 4.7% ->
23.4%, 13 / 1) and is held for a balance decision: it freezes every rival's opening, a human's
included (docs/AI.md). Not kept:

- **Raging Seas** (Lanun: 75 damage to every non-Lanun unit on a plot next to water, a quarter
  of the other civilizations' coastal improvements destroyed) at a strike on a coastal city or a
  coastal city of ours in peril in a civ war. Dev 37.5% -> 25.0% (0 / 5), share@end +42; fresh
  28.1% -> 25.0% (2 / 4), share@end +44 (t 0.55). 1: FfH's condition (at war with a fleet worth
  10+) held in 6-7 of 40-64 games, around turn 580, and 75 damage to every coastal defender looked
  decisive. 2: cast in 25 / 44 games instead, around turn 370-390, mostly on "a coastal city in
  peril" during the Lanun's own winning offensives (traced: turns 322-359, cities falling to them
  around the cast). The Lanun ended slightly richer (commerce +47 and research +36 at turn 600,
  captures and losses unchanged), but the damage is world-wide and reshuffles who else rises
  (share at turn 450, own minus default: Clan -565 in seed 30, Ljosalfar -911 in seed 6023). In
  the three traced lost games another civilization won instead: a rival's supremacy at turn 526,
  ten turns before the Lanun's cultural victory without the cast (seed 14, where the Lanun were
  weaker after it: share at turn 500 2026 against 2219); a time victory for the team that got the
  Mercurians (seed 30, where the Lanun were stronger and held 36% of the world); a Lanun without
  the cultural goal at the end and stronger in share (seed 6023). 3: the
  win-rate loss (2 / 9 over both sets, p about 0.07) is not explained by one mechanism; what is
  clear is that a defensive trigger spends the spell on fights already being won, and that a spell
  that hits everyone but us mostly changes which rival rises. If tried again: only the strike
  trigger, gated on a strike the odds say fails without it.
- **Sanctuary** (Elohim: no foreign unit may enter our land for 30 turns, those inside thrown
  out) at a city in peril in a civ war. Dev 10.3% -> 10.3% (2 / 2), share@end +198 (t 1.51);
  fresh 7.8% -> 6.2% (0 / 1), share@end +8 (t 0.10). 1: FfH's condition (two wars at once) opened
  in few games and late (cast in 27 of 64, around turn 420). 2: cast in 47 of 64 around turn 323;
  cities lost, captures and every checkpoint measure unchanged (lost cities at turn 600 2.9 ->
  2.8): "a city in peril" (the visible threat exceeds the garrison) is a weak signal, and the
  30-turn lockout delays an attacker the Elohim would have held anyway. The development gain did
  not repeat. 3: a lockout only matters against an invasion that would take cities; the trigger
  has to be that (several cities threatened, or a losing war), and even then the measured value
  of delaying a war is small for this AI.
- **March of the Trees** (Ljosalfar: a Treant for 5 turns on every forest of ours, the forests
  turned to New Forest) at a city in peril with 6+ of our forests within 2. Dev 32.5% -> 30.0% (2 /
  3), share@end -158 (t -1.33); fresh 37.5% -> 28.1% (2 / 8), share@end -60. 1: FfH's condition
  (two wars) cast it in 19 of 64 games, late (turn ~420), and Treants on every forest plot looked
  like a decisive defense. 2: it fired at the start of the Ljosalfar's own offensives (traced:
  turns 235, 331, 386, each a few turns after their declaration, while their stacks were taking
  cities), so the Treants defended against little; and the price is permanent: every Ancient
  Forest becomes a New Forest, losing its +1 food for good (Ancient Forests never regrow) and half
  its health. After the cast the elves fell behind: military -34 at turn 500 (t -2.7), units -3.7
  (t -2.0), population -12 and production -14 at turn 600 (population t -2.3), one more city lost
  (t 1.9). 3: the spell trades the elves' forest economy for five turns of Treants; it is a last
  stand, and FfH's two-wars condition approximates "only in desperation". If cast at all: when
  cities would fall (several in peril, or the capital), never on an offensive.
- **Worldbreak** (Sheaim: every unit in a foreign city takes the Armageddon counter as damage,
  cities may burn) at a strike with a counter of 20+. Dev 5.0% -> 5.0% (0 / 0), share@end +23;
  fresh 3.1% -> 1.6% (0 / 1), share@end -58 (t -1.45); 49 of 64 fresh games unchanged. 1: FfH's
  condition (counter 50) was never met (0 casts). 2: the counter reaches 20 late (casts around
  turn 457-475, in 8 / 15 games), so the damage was 20-40 on defenders the Sheaim, the weakest
  civilization here (2-5% wins), mostly could not exploit; no checkpoint measure moved
  significantly. 3: Worldbreak's value grows with the counter, which this AI rarely raises; the
  Sheaim's problem is elsewhere (their economy and army), and the spell is not the lever.

### Tried in development, not kept (round 5)

Each failed already on seeds 1-40 (most also on a second development set); the numbers and more
detail are in docs/AI.md, "Civilization-specific".

- **Holding a world spell with a lasting effect** -- Stasis (Illians), Religious Fervor
  (Malakim), Wild Hunt (Doviello), never casting Veil of Night (Svartalfar), and on fresh seeds
  Warcry and River of Blood above. 1: each was cast the first turn its gate allowed, often at
  peace, and a "better moment" looked obvious. 2: the held casts came later and the games show
  the cost of the wait, not a better effect: Stasis held lost share up to turn 400 (share@200
  -48), Fervor held 0 / 3, Veil never cast 0 / 4 (the army's ability to capture was worth less
  than the veil), Wild Hunt held with its wolves made army units 0 / 3. 3: a world
  spell cast at its gate keeps its effect for the rest of the game, and early effects compound;
  with the timing now in the AI (`world_spells.rs`), a better time has to be measured as
  "earlier or as early", and a hold only pays for a short effect tied to a battle (none
  confirmed yet).
- **Siege Workshop and Stable for the Khazad** (Trebuchets, Dwarven Cannons, Hornguard).
  1: these units need `PrereqBuilding` buildings the generic production never builds. 2: the
  engines were built (2.5 and 1.4 a game) but changed no war (win 28.2% -> 23.1%, 1 / 3); no
  Hornguard ever, because research never values Warhorses (`research::unit_available` skips
  `PrereqBuilding` units). 3: generic gap -- `PrereqBuilding` units are invisible to production
  and research; a per-civ building rule cannot fix research, and the siege engines' value depends
  on how strikes use collateral damage.
- **Sculptor's Studio for the Luchuirp** (Wood Golem, Gargoyle). 1: same `PrereqBuilding` gap.
  2: with both golems, Wood Golems replaced the Dwarven Slingers as defenders (no promotions,
  slow healing) and the Studios cost markets (share@300 -91, t -1.28); Gargoyle only, share@end
  -86. 3: golems are worse city defenders than promoted slingers for this AI; the Studio is worth
  it only for units the defense logic values correctly (healing and promotions counted).
- **Siege Workshop for the Lanun's War Tortoise**: 0.3 Tortoises a game, no effect (same gap;
  too few Tortoises to show anything, not traced).
- **Lanun coves only by boats without orders**: seeds 1-40 share@300 +34 (t 1.39), seeds 41-80
  -58 (t -1.75): the two development sets disagree, so the effect is zero and the rule is noise.
- **More Ljosalfar workers** (elves build ~twice as slowly on forest). 2: improvements +3.2 at
  turn 200 but fewer cities (legacy@200 -37.5, t -2.46: the workers' hammers came from settlers);
  only once no site is left: share@300 -46. 3: for this AI, cities beat improvements early; if
  the shortfall is worth fixing, the lever to try is which plots get improved first, not the
  worker count (untested).
- **Grigori Medics at war**: 0.9 a game, no effect (too few to matter; not traced). Not tried:
  keeping the Adventurers (the Grigori's first great people) as casters; they become ordinary
  army units upgraded by the generic code.
- **Malakim Citadel of Light** valued at 250 in frontier cities at war: every game
  byte-identical to the base -- the Malakim AI never researches Sorcery, so it is never
  buildable (the research plan's gap, not the building's).
- **Elohim Sanctuary held for an invasion**: the AI-only gate (two wars) opened in 4 of 40
  games; nothing to measure. With the gate now in the AI, Sanctuary needs a timing of its own
  (task: world-spell timing).
- **Bannor Crusade only in won wars**: taken in 7 instead of 15 games, no effect -- Crusade comes
  late (Fanaticism, turn 280-390) and costs 2 turns of anarchy, so its timing is not a lever.
- **Doviello**: a gold reserve for the upgrade spells (fewer Sons of Asena: worse -- spending the
  treasury on the only upgrade path is right); a Siege Workshop and Stables for the War Machine
  and Bison Riders never fired (research never got there in time). Not measured on fresh seeds:
  no Worker -> Beastman conversion at war (share@end +108, t 0.96, legacy@end t 2.08, win flat),
  the best untested candidate here.
- **Kuriotates variants**: no settlements at all (share@300 -183, t -1.9: the land matters);
  placement without the garrisons (share@300 +81 and +72 on two development sets, but share@end
  -4 and +16); an extra defender in every full city (share@end -318, t -2.0: unit support). The
  kept version is placement, garrisons and rebuild together.
- **Balseraphs: Create Puppet valued as a plain summon** instead of its `iAIWeight` 1000:
  share@200 -124 (t -1.66) -- the suicide puppets that FfH's weight produces are worth more than
  their summon value says.
- **Sheaim Planar Gate valued by its creatures**: built 0.1 a game, no effect (rarely eligible).
- **Hippus Blitz units attacking again in a strike**: flat (see Warcry).
