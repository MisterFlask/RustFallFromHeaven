# The Armageddon ending and the lifeboats: design brief

Status: design only (2026-11-14, the owner's design). Nothing is built. This file records the
purpose, the rules as ruled, what was considered and dropped, and what is still open.

Not FFH2: it replaces most of FFH2's victory conditions. When built it is in every game, not
behind an option, and its changes are listed in docs/PORTING.md ("Changes from FFH2").

## Purpose

More than one player can win a game, but not everyone can. When the Armageddon counter reaches
100 the world is ending, and the winners are those who hold one of a few safe cities, the
*lifeboats*, when the timer runs out. Lifeboats are scarce and only their holders win, so most
players are left behind: a lifeboat dynamic. A demonic side wins the opposite way, by leaving
no lifeboat standing.

The ending is a place on the map, so the last act of a game is a war over cities, which the
rules already know how to run.

## Before the counter reaches 100

- **Victories.** Conquest, domination and culture win the game as they do now. There is no
  religious victory, no time victory and no score victory.
- **The Tower of Mastery and the Altar of the Luonnotar** do not win the game. Each gives its
  owner substantial military or economic bonuses, and completing either raises the Armageddon
  counter by 20. Their cities are lifeboats once the counter reaches 100 (below).
- **The counter.** It rises by 1 every fourth turn. The mechanics already in the game that
  raise or lower it stay. The Elohim may lower it as far as they are able. Nothing else forces
  it to 100.
- **Hyborem** rises when the counter reaches 50, if nobody has summoned him by then.
- **Razing one's own cities.** Only the Infernals and the Sheaim can.
- **Turn 400 without Armageddon.** If turn 400 comes and the counter has not reached 100: the
  Elohim and their strongest vassal win, if the Elohim are in the game; otherwise the highest
  score wins. If the counter reaches 100 on turn 400 itself, Armageddon resolves first.

## When the counter reaches 100

- A timer of 100 turns starts.
- Every victory condition other than the two below switches off.
- **Carcosa rises.** A tile of water becomes a small landmass, joined to the major landmasses
  by small land bridges. Ships can pass the bridges. Carcosa is a city, and it is the default
  lifeboat: it exists even if nobody built any other.
  - It starts with a neutral garrison. Whoever takes it first holds it; no rule protects it
    from the nearest fast army.
  - It cannot be razed for the first 30 turns of the timer.

## Lifeboats

- Carcosa.
- The city of the Tower of Mastery.
- The city of the Altar of the Luonnotar.
- The most legendary city: the legendary city with the most culture, while it is still
  legendary. If it is razed, the title does not pass to another city.

A lifeboat is a city like others: it can be captured, and anyone can raze one as they could
raze any other city. There are no special razing rules beyond Carcosa's first 30 turns.

## Winning

- **Mortal victory.** Hold at least one lifeboat when the timer runs out.
  - Only the holder wins. Allies do not.
  - The Mercurians are always on a team, and their teammate shares their victory.
  - The Elohim take one vassal along.
  - A lifeboat held as a Kuriotates settlement counts.
- **Demonic victory.** The last lifeboat is razed. The game ends at that moment. Because of
  Carcosa's protection this cannot come before turn 31 of the timer.
- **Who can win which way.**

| Who | Mortal | Demonic |
|---|---|---|
| The Infernals | no | yes |
| The Sheaim | yes | yes |
| The owner of the world wonder Hyborem's Contract | yes | yes |
| Everyone else | yes | no |

- **Nobody wins** if the last lifeboat is razed and no surviving civilization has access to
  the demonic victory. This is deliberate.
- There are no other victory conditions for angels or demons (no score comparison, no
  "kill Hyborem"). The reason to summon the angels is to fight the demons. Because players
  choose whether to play those civilizations, the design does not need to balance their
  chances of winning.

## Vassals

- Oaths do not end when the counter reaches 100.
- After the counter reaches 100, a vassal may break free and declare war on its master.
- What vassalage is for under this ending: a beaten civilization keeps its cities, rebuilds
  under protection, and can then make its own bid for a lifeboat, which needs one city held
  at one moment and not a large empire.

## How the civilizations fit

- **The Sheaim** drive the counter up and may win either way.
- **The Elohim** hold the counter down, win at turn 400 if they keep it from 100, and are well
  suited to holding one city if they do not.
- **The Illians.** Auric's ascension is a military event. It helps take a lifeboat and is not
  a lifeboat itself.
- **Culture civilizations** (the Kuriotates, the Sidar and others) win outright before 100, or
  hold the most legendary city after.
- **Naval powers.** Carcosa rises from the sea, so it is coastal.
- **Religion spreaders** have no route of their own: the religious victory is gone.
- **Wide empires.** Owning much of the map wins by domination before 100. After 100 it is only
  the means of holding one city.

## Left as it is on purpose

- **The last turn decides.** A lifeboat captured on the final turn of the timer counts for its
  captor, even if another civilization held it for the 99 turns before. Not to be solved.
- **Carcosa goes to whoever reaches it first** after beating the garrison.
- **The Elohim can drain the counter without limit.** The drift and the other civilizations'
  actions are what oppose them.

## Considered and dropped

- **A compatibility map of victories** (several victories that can coexist or not, with a
  countdown started by the first one completed). Replaced by this simpler ending.
- **Two risen cities,** and a number of them scaled to the player count. One Carcosa, plus the
  lifeboats players build.
- **A lifeboat that shelters the holder's allies** (a garrison inside, with a cap). Only
  holders win.
- **Holding a lifeboat for the last ten turns,** or a hidden end to the timer, against the
  last-turn capture.
- **The legendary lifeboat passing to the next legendary city** when razed.
- **The Tower and the Altar as outright victories** before 100.
- **A religious lifeboat** (a holy city whose faith reaches a share of the world).
- **Special angel and demon conditions:** the Mercurians winning by killing Hyborem and
  outscoring the Infernals; the demons needing to outscore the angels' team.
- **A prophecy of omens** to drive the counter, and a turn by which the counter is forced to
  100. The drift of 1 every fourth turn and the existing mechanics drive it.
- **Razing a lifeboat taking several turns.** Carcosa's 30 turns of protection and its
  garrison are the only protection.
- **All vassal oaths ending at 100,** and the further vassal ideas (a seat earned by
  contribution, the vassal as heir, absorption, a pledge to a contender, a "survived" result
  between winning and losing).
- **Rolled perks keyed to the counter** (docs/ROLLED_PERKS.md): perks stay on turns 100, 200
  and 300.

## Open

1. **The AI.** Deferred by the owner's decision; it is to be taught this ending. CLAUDE.md's
   rule that the AI does not pursue the Tower or the Altar was written when they were
   solitaire victories and needs the owner's revision then.
2. **The bonuses of the Tower and the Altar:** what they are.
3. **Hyborem's Contract:** its cost and requirements, and whether its owner can also raze
   their own cities as the Infernals and the Sheaim can.
4. **Carcosa's site:** which water tile (far from every capital is the working assumption),
   whether the site is known before it rises, and how many and how wide the bridges are.
   Narrow bridges favour the holder strongly.
5. **The Elohim's vassal:** which one they take along when they have several (at turn 400 it
   is the strongest).
6. **The timer's length:** whether 100 turns scales with player count or game speed.
7. **Tuning:** the Elohim's draining against a world that raises the counter; the size of
   Carcosa's garrison.
