# Rolled perks: design brief

Status: design only (2026-11-14, the owner's design). Nothing is built. This file records the
purpose, the decisions made, what was considered and dropped, and what is still open.

Not FFH2: a new rules layer of this game. When built it is in every game, not behind an option.

## Purpose

By turn 100 a player's strategy is mostly fixed by their civilization, their land and their
opening; the rest of the game is execution of a plan they already had. A rolled perk is an
outside shock that reopens the decision: it makes a strategy that was not the best one for this
player the best one, so they re-plan. It happens three times a game and differently each game,
so the same civilization does not play out the same script every time.

What follows from that:

- **A perk is measured by how much it changes what the player does for the next fifty turns,
  not by how strong it is.** Power is the means: the perk has to be strong enough that turning
  toward it beats continuing as before.
- **A perk changes a rule.** A lump sum (two Great Engineers) gives nothing to build around and
  is not a perk; it can be filler in a bundle (below).
- **The test for a candidate perk:** would a player who takes it play differently? If a player
  already going that way takes it and nothing changes, and a player not going that way would
  never take it, it fails.

## The system

- **Schedule.** On each of turns 100, 200 and 300 every player is offered three bundles and
  picks one. The three tiers rise in power. Turns scale with game speed. The schedule is fixed
  turns (the owner's decision, 2026-11-14): it is not keyed to the Armageddon counter, though
  a game whose counter reaches 100 early may end before the third offer.
- **A bundle is a perk plus a hidden adjustment.** The player is shown three finished bundles
  ("this set of bonuses, or this set, or this set"). How each was put together is never shown
  or explained.
- **The perk becomes a leader trait.** The unique passive of the bundle picked is displayed as
  a trait of the leader, visible to every player as traits are. Leaders accrue traits over the
  game: two at the start, up to five by turn 300. The rest of the bundle (the adjustment) is
  not displayed to other players.
- **No exclusivity for now.** More than one leader may hold the same perk trait in a game.
- **No catch-up mechanic for now.** The adjustment does not lean toward trailing players. It
  could later.

## The hidden adjustment

The problem it solves is the strategy-alignment lottery: the Hippus rolling a cavalry perk get
its full value at no cost, while anyone else would have to pivot to use it. Without an
adjustment the aligned offer is always the obvious pick and the system changes nobody's game.

- **What it is for.** To make the pivot feasible and attractive, so that the offer that would
  change a player's game the most is one they can actually take. Fairness between players is a
  side effect, not the aim.
- **How it is sized.** Each offer has a target worth for its tier. The system estimates what
  the bare perk is worth to this player now and fills the gap with techs, units, buildings or
  other grants. The target is that the pivot offers are genuinely competitive with an aligned
  one; erring toward the pivot is acceptable.
- **The cases it covers.**
  - Pivot: an archer civilization offered the cavalry perk also gets Horseback Riding and
    horsemen.
  - Too small to benefit: a civilization with one mana node offered "nodes give double mana"
    also gets nodes or the means to build them.
  - Already aligned: the Hippus offered the cavalry perk get the bare perk.
- **Filler.** Related to the perk where possible (horsemen with a cavalry perk). Not an
  ironclad rule: generic filler is allowed, and is needed where nothing related exists.
- **One estimator, two uses.** "What is this perk worth to this player now" both sizes the
  bundle and scores the AI's own choice among its three offers.
- **A side effect that helps.** An aligned player sees one bare perk beside two offers that come
  with techs and units, so the pivot options look richer.

## Perks as traits

- FFH2 already has traits that change mid-game (Adaptive, Insane). Perk traits are exempt from
  both: they are never swapped or reshuffled away.
- Many perks are ordinary trait effects with larger numbers (free promotions for a unit class,
  yield and cost modifiers) and can use the existing trait machinery. Perks that add a rule
  (the dead rise as zombies, dual-cast) need their own code.
- The AI already keys behaviour on traits (`TRAIT_AGGRESSIVE` and the expansion drive). A perk
  trait carries its behaviour hook the same way: a leader who gains the cavalry trait builds
  cavalry. A perk the AI holds but does not play toward is wasted, so the AI hook is the real
  cost of each perk, more than the rule.
- Each perk needs a trait name (flavor allowed) and a bare rules description
  (CLAUDE.md, "Rules text and tooltips").

## Candidate perks

The owner's list (2026-11-14):

- Cavalry are supercharged.
- All units gain buffs from the Armageddon counter and hell terrain; the player now worships
  the Ashen Veil.
- Wizards dual-cast their fire spells.
- Overseas colonies cost no maintenance.
- Arcane barges cost half; gain three.
- Resources offered in diplomacy are worth twice as much to the receiving civilization.
- Units put on crusade are supercharged.
- Units the player kills rise again as zombies.
- Mana nodes provide double mana.
- Unit class X has affinity for mana Y.
- Cities produce double culture.

Brainstormed, not decided:

- War: captured cities keep their buildings and do not riot; pillaging yields triple gold plus
  research; the hero returns to the capital some turns after dying.
- Magic: summons are permanent, up to a cap; rituals cost half; vassals and open-borders
  partners share their mana.
- Hell: hell terrain is fertile for the player; the player can move the Armageddon counter
  once every N turns.
- Economy: the capital works a third ring and other cities are capped in size; any unit can be
  bought with gold anywhere in the player's borders; barbarians are at peace with the player
  and their units can be hired; roads in the player's borders give double movement to the
  player and none to enemies.
- Diplomacy and culture: each civilization following the player's state religion pays gold per
  city; a culture flip also takes the garrison.
- Perks that bend this game's own systems (governors, armies, cultural actions, crusades,
  supply, works, cards) are the ones no other game has.

Design notes on perks:

- **Counterplay.** A boost with a shape (open terrain only, on the attack only) changes what
  the opponent does; a flat bonus only makes them need more units.
- **Bargains.** The Veil perk takes something away (religion, alignment, friends), which is
  FFH2's idiom, and a price is a second answer to the alignment lottery: a strong perk with a
  cost is not a free win for the civilization that already fits it. What share of the pool
  should be bargains is open.

## Considered and dropped

- **A guaranteed "deepening" offer at turn 200** (one offer always extends the turn-100 pick).
  It rewards continuing in the direction already picked, the opposite of reopening the
  decision. If picks are to compound, a later perk should be a new direction that uses what
  was built, not more of the same.
- **Existing traits as cheap first-tier offers.** Most are modest stat bonuses nobody would
  change course for.
- **The adjustment as a visible "top-up to a floor" rule** (grant whatever prerequisites the
  player lacks, stated in the offer). Its argument was that it could be explained in a
  tooltip; the adjustment is never explained, so the value budget above replaces it.
- **Private picks.** Considered, with discovery through effects; replaced by perks shown as
  leader traits.

## Open questions

1. (Settled: the schedule stays at turns 100, 200 and 300.)
2. **The worth estimator.** How crude it can be and still make pivot offers competitive. It is
   the piece both the bundles and the AI's choice rest on.
3. **Pool size and tiers.** How many perks per tier, and whether the tiers are separate pools
   or one pool at three strengths. Without exclusivity, roughly ten to twelve per tier keeps
   repeats rare.
4. **Bargains:** what share of the pool.
5. **How later tiers relate to earlier picks,** given that deepening is dropped.
6. **When the offer is rolled and shown:** on the turn, or some turns ahead. A preview lets a
   player prepare, and lets them shape their position to change the hidden adjustment.
