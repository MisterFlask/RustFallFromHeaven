# Playing ffh2-rs as a human (`ffh-play`)

`crates/ffh-play` is the human-play front-end: a session core that lets a person (in a browser)
or a program (JSONL over stdio) play one slot while the AI plays the others. The design is
`docs/PLAY_DESIGN.md`; this file says what you can run today and how to use it.

## Status

Phases **P0** (skeleton: JSON, type names), **P1** (session core, in-process API), **P2**
(the AI-as-human soak, `ffh-play soak`), **P3** (views and queries, the turn report, deals),
**P4** (transports), **P5** (the browser client's MVP) and **P6** (the browser client complete:
city screen, civics, religion, sliders, diplomacy and trade, tech tree, Civ4's hotkeys and unit
cycle, map layers, the testing tools, the spectator mode, map deltas; `ffh-play bench`) are done.
You can play in a browser (`ffh-play serve`, below); a script or an LLM can play a whole game
headless with `ffh-play agent` (JSON lines over stdin/stdout). What is left (P7) needs other
crates' owners: see "Known gaps" and the status table at the top of `docs/PLAY_DESIGN.md`.

## Playing in the browser

```
cargo build --release
./target/release/ffh-play serve            # then open http://127.0.0.1:8642/
```

The page opens on the title screen: a small isometric island drawn from the map's own sprites (a
kingdom on one shore, the hells creeping in on the other) under drifting clouds, and the main menu:
**Continue** (the newest save or autosave, by its time), **New game**, **Load game**, **Options**
and **How to play** (the help at its first topic). With a game running the title screen goes; the menu's Main menu button
(the map icon) brings it back with Resume (or `Esc`). The new-game screen lists the civilizations
with their banners; picking one shows the chosen leader's portrait with its leaders as portrait
cards (or random) beside it, the first paragraph of its pedia (its trait and how it plays) and, beside it,
what the leader and the civilization bring: alignment, hero, world spell, each trait's effects,
starting techs, unique and blocked units and buildings, special abilities (the dialog takes the
window's height: the list of civilizations is as tall as the column beside it, whose description
keeps at least 8 lines and scrolls beyond; Start stays in view); then the world
(opponents, world size, speed, difficulty, climate, sea level, seed, the game's name), game
options, victory conditions and a closed "Debug" fold (Start at war, below), each choice's
description as its tooltip, and "Vanilla experience"
(leaders speak only FFH2's own lines; unticked, each civilization also has its own lines for
every other civilization, `data/text/barks.md`). The load screen has a
card per game (the current one first, then the most recently saved) and a row per save with its
turn and when it was written, the older autosaves folded away. Each game gets a directory in `./saves` (`--saves DIR` to change it).
Loading a save or starting a new game while you play a game asks first ("Leave the current game?
Moves since the last autosave (turn N) are lost.", Cancel focused); not on the title screen, in an
observed game, or once the game is over. Saving under the name of one of the game's saves asks
"Replace the save NAME?" first.

`serve` takes the GAME flags too (`--seed 3 --world WORLDSIZE_SMALL --new` starts a game at once,
`--load saves/game-3/auto/T0040.ffhsave` resumes one). Use a current Chrome, Firefox or Safari;
nothing is fetched from the internet.

**Start at war (debug).** The new-game screen's "Start at war (debug)" button (in its "Debug" fold,
after the victory conditions) starts a game in the
middle of a war, for trying combat and the war screens without playing there: an AI-only game of the screen's
settings is played until two civilizations at war fight a battle with 3 or more fighting units
on each side (the units each side commits to the formation battle), and you take over the attacked
civilization at its last turn before the attack (`crates/ffh-play/src/war_start.rs`). Turns are
sequential, so that is the turn of the battle when the attacker moves after you, else the turn
before. Since your civilization no longer plays as the AI did, each candidate is checked by
playing on with you idle, and kept only if a battle of that size against you comes within two of
your turns. The view opens on your threatened stack and a message names the attacker; when your
turn comes after the attack, the battle screens open on it (below, "After a battle"). The search
takes a few seconds on a small map (big battles come late: turn 250 or so) and up to a minute on a
large one (the screen shows the AI game's turn meanwhile); the same settings give the same start.
With "Random" any attacked civilization will do; a chosen one plays slot 0 and only battles against
it count. The game's directory is `war-<seed>`, and its `war-start` save is the start (load it to
play it again).

**The look.** The client draws with the game's own pixel art (docs/ART.md, "cheerful
readability", embedded in the binary like the rest of the page): a bright daylight map in the
spirit of the old Shining Force games, figures standing on team-coloured rings, and an interface of
navy panels with gold rules, gilt nails and maroon title ribbons, the Vigil
bitmap capitals for titles and numbers, the Vigil Book face for buttons and short labels (long
texts, such as events and help, stay in the system serif), the game's yield, commerce, mana and
Armageddon icons (and in the screens those of promotions, unit classes, buildings, specialists,
religions and civic categories), and an illuminated initial on events, votes and decisions. Until the art has
loaded (a moment after the page) the page shows the plain look of P5, and keeps it if the art
cannot load.

**The screen.** The top bar: the turn crescent (amber on your turn), turn and year, what is
happening (your turn; "Moving: …" with the players and the seconds while the AIs play); gold and
rate, research (click it for the tech tree, where research is chosen; the "Choose research" popup is
the same tree, with "Later", and closes on a pick) with its progress bar and turns ("11 turns"), the doctrine
(the adopted one, or "no doctrine", in the warning colour while one can be adopted; its tooltip
says what a doctrine does, a click opens the doctrine chooser), the commerce split by its
icons (click it for the sliders), the Armageddon eclipse (its corona reddens at 50, 75 and 100)
with the counter (its tooltip lists the Armageddon deck's slots, 10 to 100: a slot's card once it
is revealed, or seen ahead, with its effect; fired slots marked; the others hidden), mana by sphere, anarchy, golden age, Stasis, idle AI-controlled groups, score
(your share of the world, docs/SCORE.md, with the fair share in its tooltip); each number has a
tooltip naming it and an accessible name ("Research rate 100%", "Armageddon counter 0", "Mana 3",
"Score: 66.7% of the world"); the gold's tooltip
gives where the gold rate comes from (the cities' commerce, deals; units with the free ones, units
outside the borders, city maintenance, civic upkeep, inflation); beside them the empire's own
actions, a row of buttons that belong to no unit: your world spell (its tooltip has its effects,
where its ritual stands, and, while a charge is held, the unit it is cast through, one that can
cast it now and loses no promotion by casting, picked for you, placed by your nearest city ("Cast
by your Adept near Prespur", "in Prespur"; no place without a city); the unit panel no longer lists
world spells; "World rituals" below), and your
cards' world spells (docs/CARDS.md: "Card spell: cast once a game"; still shown, spent, once
cast). A spell cast at a distance asks for its target first: a plot, a city or a unit is picked on
the map, the valid plots outlined (a plot with several targets lists them), a player or one of the
spell's options from a list (a trait with its effects); each valid plot of a pick on the map is
marked on its own (a translucent cyan fill and a bright frame; on the HD-2D map the mark also shows,
a little fainter, through the trees and units in front of it). A world spell the player places
(`place_on_charge`: Wellspring) has no caster: its button reads `Place Wellspring` while the charge
is held (its tooltip: the spell's rules, where the charge is held, `Opens the placement: a marked
plot on the map, then a mana type.`). On the turn Ritual: Wellspring completes the placement opens by
itself, once (not after loading a save: the button opens it then): the prompt `Ritual: Wellspring
completed. Place the mana node: click a marked plot.` `Esc: later.` (with a Later button) and the
valid plots marked; a click on one opens the `Wellspring` menu of the mana types there (the
`place_menu` query, `view/place.rs`), under the line `Choose a mana type for the marked plot. I, II,
III: the spells of each tier for Arcane Units.`, one compact row each, by name: the mana's icon and
name, the count before and after placing it (`1 → 2`; its tooltip `You own 1 Fire Mana. After
placing: 2.`, the count after from the rule that counts a plot's resource: a node off the capital's
trade network adds nothing), then the effect chips, each only where it applies and each with its
sentences in its tooltip: health or happiness `+1` (Life, Enchantment), affinity (only when the
player has, or can build, a unit with an affinity for it: the data's sentence, `Yours: Earth
Elemental.`, `You can build: ...`), buildings (the count of its civilization's buildings and
projects the mana allows or speeds, with a dot when this node is what first meets one's mana
requirement: `This node meets the mana requirement of Storm Forge.`) and `i` (every sentence of
what owning it gives: health and happiness, `New Arcane Units get Fire I with 2 or more Fire Mana,
Fire II with 3 or more and Channeling II, Fire III with 4 or more and Channeling III.`, affinity,
the buildings it allows and the projects it speeds). The second line is the sphere's spells as
chips with their names (the spells have no art of their own), grouped by tier behind a mark `I`,
`II`, `III` (its tooltip: the promotion, `Needs Channeling I and Fire Mana.`, `Free to new Arcane
Units with 2 or more Fire Mana.`); a spell's tooltip is its name, `Needs Fire I.` and its help.
At most one line follows, on what this placement changes: `First node: Arcane Units with Channeling
I can learn Fire I.` (0 → 1), `New Arcane Units start with Fire I.` (1 → 2), `New Arcane Units with
Channeling II start with Fire II.` (2 → 3), `... Channeling III start with Fire III.` (3 → 4), or,
in red, `No effect: the plot is not connected to your capital.` A tap on a chip shows its tooltip
(Enter or Space on a focused chip too) and places nothing. A row (a click, Enter or Space on it)
places the node (`place {spell, target}`); Cancel goes back to the plot pick; Escape (or Later)
postpones: the charge stays held;
below them the menu (Research, Techs, Cities, Civics, Religion, Diplomacy, Options, Save, Load,
New, the help `?`) and End turn (the raw request box is under Tools). The map fills the middle (minimap
bottom right, zoom and the map layers bottom left), the selection panel is on the right, and the
bottom panel has the turn report, the messages by turn and the game's files (directory, journal,
last autosave).

**The map** is north up and wraps east-west like the game. It is the **HD-2D map**
(docs/HD2D.md): the pixel-art sprites of units, towns and props standing as cards in a lit 3D
world, seen through a camera tilted toward the horizon, with light shafts, glow, a mood for each
region and glowing borders. Where the browser cannot run it (no WebGL2 on a GPU, its shaders too
slow, a lost graphics context) the page says so and draws the **isometric 2.5D map**
(docs/ISO.md) instead. Zoom (`+` `-`, `Q` `E`, the wheel) goes through each map's steps.
Hovering a plot shows a tooltip
(terrain, resource, improvement, owner, yields and defence while you see it, the city and units
there, each unit with its strength, level and promotions; with the art, the yields, defence and move cost by their icons; over a unique improvement
such as Yggdrasil or the Maelstrom, what it does: its yields, defence, happiness and healing, the
resource and spells it offers, what happens to units that enter or come near), and with units selected
the path: dots from plot to plot that march, a roundel with the
turn of arrival at each turn's end, the part beyond this turn's moves dimmer, ochre for a route
to build; an attack ends in crimson brackets with the odds of winning on a chip, and the tooltip
gives the odds of winning and retreating with both strengths and their modifiers; with an army
selected it also says whether the army would be supplied there, "Supplied (cost 14 of 24)" or
"No supply line" (docs/SUPPLY.md), worked out from what you see: an enemy unit you do not see does
not cut it, so it can be wrong. **Supply lines** (`app/supply.js`): each of your armies' line
runs from the army to its source (a city, or a ship that can carry land units) as dashes in
vellum over a dark casing, and the lines of other players' armies you see in their owner's
colour, as you can work them out: over the plots you have revealed, around the enemy units you see
(a hidden unit does not bend them, so they can be wrong); a cut-off army carries a marker under its plot,
"Hunger: 2" (the turns before Starvation) in amber or "Starvation" in red. On the HD-2D map the
lines are drawn on the interface layer over the ground (under the stack badges) and the dashes
flow slowly toward the army while the map animates (Tools → the map's panel, "Animate"; still
under reduced motion); the isometric and flat maps draw them still (those maps redraw only on a
change). Not on the city screen. The layers
(buttons bottom left, `Ctrl+1/2/3/4`): **Terrain**; **Political** (each owner's land washed with
its colour); **Yields** (the ground dimmed, and the food, production and commerce of the plots in
sight with their icons; fogged plots show none); **Sea** (docs/NAVAL.md section 10, `app/sea.js`;
on the HD-2D map `app/r3d/sealayer.js`: the water out of your range under a fog bank, thinning
over about a plot at the range's edge, drifting while the map animates; on the flat and isometric
maps darkened and hatched, the range's edge drawn as a light line over a dark casing; over the
plots you have revealed; a worn warship of yours carries a wear pip, amber, red from half the
maximum, a notch for the share worn). The Sea layer turns on by itself while a ship or a unit in
boats is selected and goes back to the layer before when the selection ends (a layer you pick
meanwhile stays). On the HD-2D map it is part of the world: on the water, lit, under the ships and
what stands in front of it; on the flat and isometric maps it is drawn under the cities and units.
On the Sea layer a port of yours says in its tooltip
the range it gives (`Range: 12 plots.`). **Outposts** (docs/NAVAL.md, phase 4; `app/lanes.js`,
`view/outposts.rs`): the Sea layer draws each outpost's lane in its owner's colour (yours in vellum),
the dashes flowing to the home port (on the HD-2D map a ribbon on the water; on the others as
supply lines are drawn); a cut lane breaks where it enters the sea that cuts it, the rest of it to
the port dotted (as a supply ship's cut lane), with the break's mark (a ring and a cross) and
`Cut: 3` (the turns before your outpost starves); others' lanes over the plots you have revealed. The Colonist's action (and the
Settler's second) is **Found outpost**, its tooltip the site rule and, when it cannot, the reason
(`An outpost must be on another landmass than your capital's.`). An outpost's city screen has no
production: `Lane: open, to Ithralia (14 plots).` / `Lane: cut in Sirens' Water, turn 2 of 5.` /
`No lane to a home port in range, turn 2 of 5.` / `Starved: -1 population a turn, not a port.` and
the button `Raise to city (150 gold)` (command `raise_outpost {city}`). Its banner reads `Name
(outpost)`; the Cities list marks it `outpost` with the lane's state in place of production; a
resource none of your landmasses holds says `Not on your landmasses.` in its tooltip (the plot
query's `bonus_note`). **The sea** in the previews and panels: with a land
unit selected, a path over water adds `In boats from plot 3: strength 1, 1 plot a turn. 2
turns.` (the plot's index along the path, the civilization's real plots a turn, the turns spent
at sea) and, over water out of range, `Out of range from plot 6: -15% HP a turn.`; an army at sea
reads `Supplied: at sea, in range of Ithralia` there and in the Armies screen and unit panel; the
unit panel of a ship says `Range: in range (5 of 12).` or `Out of range: -15% HP a turn.` and
`Wear: none (turn 4 of 10).` or `Wear: -15% strength. Enter a port to clear.`, of a unit in boats
`In boats: strength 1, cannot attack, does not heal.`; a unit in boats is offered only moving,
waiting (skip, sleep, sentry) and deleting; the attack preview lists a warship's wear among the
modifiers ("wear at sea") and says `Defender in boats: strength 1, no modifiers.`. **Named seas**
(docs/NAVAL.md 4 and 10, phase 2; `app/sea.js`, `app/seas.js`, `view/seas.rs`). On the HD-2D map
(`app/r3d/sealayer.js`; docs/NAVAL.md 10 has the look) each sea you have revealed a plot of has its
name lettered on the water (pale, spaced italic serif along its long axis, sized to it; faint
close in, and with another layer shown faintly at the two farthest zooms), a soft band of its
holder's colour along the inside of its boundary (yours your colour, an enemy's its own, an
unknown holder grey; contested both in long dashes; patrolled by a team at peace with you fainter;
open none), a faint dotted line between seas, and, held, contested or patrolled, a buoy with the
holder's banner (crossed banners when contested) by its name; hovering the name or the buoy
outlines the sea and shows the badge's text and the rules in the tooltip, and a click opens the
panel; a tap does too, and the controller's cursor on the buoy's plot (A, with nothing
selected). On the flat and isometric maps each sea has its border dotted (dark dots with a light core), its water
tinted by its state as you can work it out (only ships you see count) -- held by you lightest and
plain, held by an enemy darker with wide amber stripes, contested with alternating light and amber
stripes, patrolled by a team at peace with you with sparse dots, open and out of range plain --
and, at its deepest revealed plot, its name in italic serif over a badge: `HELD: YOU 24 to 0`,
`CONTESTED 14 to 17`, `HELD: LANUN 0 to 31`, `OPEN`, `PATROLLED: LANUN`, `OUT OF RANGE` (yours
first); a sea closed to you that is cutting a lane or blockading a port of yours shows held even
with no ship in sight (`HELD: LANUN 4 to ?`, the seen part `7+`; `seas_felt_by`). Your supply ships' lanes are drawn as supply lines are, from the ship to the port; a cut
lane breaks with a ringed cross where it enters the closed sea and runs on dotted (on the HD-2D
map ribbons on the water). Clicking a name, buoy or badge opens **the sea's panel** (on the HD-2D
map the sea is outlined while it is open): plots, control and what holding takes (`Held
at twice the enemy's strength on patrol. You need 34.`), each side's strength bar with its ships
on Patrol (`Ships you do not see are not counted.`), `Effect now` (lanes, your ports, interception,
supply ships), `If Lanun holds it` (each lane that would be cut, each port blockaded, supply
ships), `If you hold it` (whose ships are intercepted, their ports blockaded), and with warships
of yours selected `Send the Trireme to patrol here (3 turns)` with the state it gives (`The Grey
Narrows: 38 to 17. Held by you.`): a go-to to the nearest plot of the sea they reach with Patrol
queued after it (Patrol at once when they stand in it). **Seas** (the menu, shown once you have
seen a sea): one row per sea, its control, your strength, the enemy's, your ports on it (a
blockaded one marked) and your supply ships' lanes through it; a row shows the sea on the map, its
Panel button opens the panel. A coastal city's banner carries an anchor mark when its harbour is
blockaded (red with a gold ring when it is besieged as well), on every map; its tooltip and the
city screen say `Harbour: open. Bay of Lorda is held by you.`, `Harbour: open.` or `Harbour:
blockaded. Sirens' Water is held by Lanun. Sea plots not worked, no sea trade.`. The path preview
adds `Enters The Grey Narrows (contested) at plot 4: will be intercepted by Lanun's patrol.` and,
for an attack on a coastal city, `Harbour open: Lanun holds Sirens' Water. The city heals and
repairs at 50% during the siege.` / `Harbour open. ...` / `Harbour blockaded.` with what
blockading it takes; `No supply line` gains the transport's reason (`the transport is in Sirens'
Water, held by Lanun.`, `the transport's lane is cut in ...`, `the transport has no lane to a port
in range.`), there and in the army's supply text. The unit panel says `On Patrol: The Grey
Narrows.`; the Patrol action's tooltip is the rule and the state it gives; Civ4's Sea Patrol is
no longer offered (it does nothing for control; the rules keep it). The battle log's opening says
`Your Trireme fires from the coast: Warrior 12.`, the plan lists the ships that would fire on each
side and the report each side's shots. The help (`?`, "The sea") has the rules with the defines'
numbers. The minimap is a pixel a plot in the same
colours, owned land in its owner's colour, cities as light dots.

**Sea Ice** (the Illians' Frozen Passage, docs/PORTING.md "Changes from FFH2"): the map's `sea_ice` layer. The HD-2D map draws it as the polar ice sheet (a pale blue-white floor with darker cracks at the water's level, not snow land); the isometric and flat maps give it a plain pale blue tint. The plot tooltip and the plot panel name it "Sea Ice" with its rule (the plot query's `sea_ice`: `{name, help}`). Frozen Passage is cast from the top bar's world actions at a plot; Icebreak is a selected ship's targeted spell. `ffh-play stage ice DIR` stages it (shots: docs/shots/world-rituals/sea-ice-map.png, sea-ice-icebreak.png).

**The flat renderer** of P5 (colours, glyphs and labels: lettered dots for resources, boxes for
improvements, units and cities; no sprites) is kept for debugging, in the classic interface of
docs/ART.md: open the page as `http://127.0.0.1:8642/?renderer=flat`, or choose it under Options →
**Map** (which also offers the HD-2D map, the default, and the isometric 2.5D map, its
fallback). The choice is remembered by this browser. The 2D pixel-art map that drew the
map of docs/ART.md, and the 3D renderer's fully modelled map, were retired on 2026-10-31.

**Motion.** The interface moves quietly, in keeping with the manuscript look (docs/ART.md 4.5):

- **The camera glides.** When the view jumps to a plot it is not already near — centring on the
  selection (`C`), the next waiting group (`Tab`, `Enter`), a city (`Home` / `End`, `F10`, the cities
  list), a message's "Show on the map" — it glides there across the page the short way round a
  wrapping map, rather than cutting. Dragging, zooming and the minimap stay immediate (you are
  moving the view yourself), and a new game opens on its start with no glide.
- **Units slide.** A unit that moves glides from its old tile to the new one (about 8 tiles a
  second), for your own moves within your turn; a long or multi-tile hop, and every unit on turn
  rollover, jumps.
- **Combat flashes.** When a fight resolves, the struck tile flashes a brief muted crimson — a
  little brighter and rising as a unit is lost.
- **The numbers ease.** Gold, score and the research bar count to their new value rather than
  snapping.
- **The turn breathes.** While the other players move, the turn crescent and its dot breathe
  slowly; when control returns to you the crescent warms for a moment.
- **Dialogs and toasts ease in.**

All of this is off under the system's "reduce motion" setting (`prefers-reduced-motion`): the view
and units jump, no flash or breathe plays, the numbers snap, and the dialogs appear at once.

**Mouse and keys** (also in the help, `?`: How to play first, then Keys; the list there says
what each key does, without the "ffh-play's own" notes below). The keys are Civ4's: the orders' keys come from the game's
XML (each button shows its key), and the global keys are Civ4's controls
(`CIV4ControlInfos.xml`, what `CvGame::doControl` does with them), except the few marked
"ffh-play's own" below.

| | |
|---|---|
| left click | select your units on a plot (again: the next group there; Shift: all of them), open your city, or show a plot |
| right click | move or attack with the selection; with Shift held, each right click adds a waypoint (numbered on the map), and releasing Shift sends them (`Esc` drops them) |
| drag, wheel, arrows, `+` `-` | pan and zoom; click or drag the minimap to jump |
| `W` `A` `S` `D` (held) | ffh-play's own: scroll the map smoothly up, left, down, right (by the keys' place, so `Z` `Q` `S` `D` on AZERTY); Civ4's plain `W`, `A`, `S` and `D` orders (cycle, automate, sentry, destroy) are on their buttons and `Tab` / `Enter` |
| `Q` / `E` | ffh-play's own: zoom out / in (by the keys' place, so `A` / `E` on AZERTY); Civ4's plain `Q` and `E` orders (quarry, explore) are on their buttons |
| double click | open your city |
| `Enter` | the next group waiting for orders, as `Tab`; when none waits (Civ4's "end of turn" state: the End turn button glows), end the turn |
| `Shift+Enter` | end the turn whatever waits, without the checks |
| `Space` | with a unit selected: skip its turn; with none, end the turn when no group waits |
| `C` | centre on the selection |
| `Home` / `End` (`Num+` / `Num-`) | the next / previous of your cities (the first: the one nearest the centre of the map); in the city screen, its next / previous city |
| `F10` | your capital's city screen |
| `Tab` / `Shift+Tab` | the next / previous group waiting for orders, in Civ4's group cycle (Civ4's `W`; in Civ4, `Tab` is chat): a stack in which a unit has used its moves is skipped, as in Civ4; when it comes round, units with orders left move (Civ4's automatic moves) |
| `F` / `H` | fortify (sleep for a unit that cannot fortify) / heal; sentry is on its button (`S` scrolls) |
| `B` | found a city (bombard or a ranged attack for units that cannot) |
| `G` then click, `Alt+R` then click | go to a plot (Shift-clicks add waypoints, sent when Shift is released, as with right clicks) / build a route to it |
| `R`, … | a unit's builds: a soldier's road and fort, a mage's mana nodes (no unit improves the land: the cities' crews do, docs/WORKS.md) |
| `L` / `U` / `Shift+U` | load / unload / unload all |
| `Shift+P` | pillage |
| `Del` / `Backspace` | delete the selected units (asks first) / cancel their orders |
| `F1` `F2` `F3` `F4` `F6` `F7` | cities, finances (the sliders), civics, diplomacy, the tech tree, religion |
| `F5` | armies (this game's armies: docs/ARMIES.md) |
| `F9` | governors (this game's governors: docs/GOVERNORS.md) |
| `F8` | cultural pressure (this game's cultural actions: docs/CULTURE.md) |
| `Ctrl+S` / `Ctrl+L` | save / load |
| `Ctrl+1` `Ctrl+2` `Ctrl+3`, `?` | ffh-play's own: the terrain, political and yield layers; the help (How to play, then the keys) |
| `Esc` | leave a plot pick, close a dialog, clear the selection |

**Phones and touch screens.** The page fits a phone, upright or on its side (the layout follows
the screen's size: up to 760 px wide, or up to 520 px high). In both looks the menu folds into a
☰ button at the top left, which drops it down as a sheet of labelled buttons (a pick, or a touch
anywhere else, folds it again; a touch on the map that folds it does nothing else), the turn
report starts folded (a dot on "Turn report" says a new one waits), dialogs take the whole screen,
and the city screen is the dialog with its plot grid. The isometric map (the HD-2D map's fallback) is
the look phones must support; there the
top bar is one row (☰, the turn, the empire's numbers scrolling sideways), the advisors join the
menu's sheet instead of the rail, a bottom bar holds the layers and End turn, the selection's card
sits on it (upright; a column at the right on its side) and folds with ▾, and the minimap is
small at a corner (docs/ISO.md, "Phones"). The 2D map has a small-screen layout too (untested:
the top bar keeps the turn and End turn, the selection panel is under the map or a narrower
column, the minimap is at the map's top right). With a
finger as the main pointer (`pointer: coarse`), buttons are at least 40 px high, the buttons' key
labels are hidden, the card's orders are labelled buttons (a finger cannot point at a tile to read
it), the zoom buttons give way to the pinch, and text fields do not make the phone zoom. Touch
replaces the mouse:

| | |
|---|---|
| tap | as a left click: select your units (again: the next group there), open your city, show a plot (with the plot's tooltip, above the finger) |
| tap a plot, tap it again | with units selected, on a plot without units of yours: the first tap shows the way there (the path, its turns, the attack odds: the right click's preview, which a finger cannot hover), a second tap on the same plot moves or attacks. Panning, or a tap anywhere else, drops the preview and the move it offered. To move onto your own units (a stack, a garrisoned city), use Go to |
| drag, pinch | pan; zoom through the renderer's steps about the fingers (two fingers also pan) |
| ×, Cancel | deselect (the panel's head); leave a plot pick (its hint) |
| "A tap:" | in the build prompt, the city screen and the tech tree: what a tap on a build or a tech does (build now, add to the queue, put first; research now, add to the queue): Shift- and Ctrl-click with a mouse (the tech tree, the build prompt and the city screen offer it to a mouse too, as "A click:") |
| tap a tech, tap it again | in the tech tree: the first tap lights the tech's path and the techs it leads to, a second tap on it researches it (or queues it, by the "A tap:" row) |

A long press and a double tap do nothing on the map. What touch does not have: Shift waypoints
(use Go to for one destination), Ctrl-click on a unit row (Split, then a row), and the keys
(the panel's buttons, the menu and the Cities list cover them; `?` in the menu lists the controls).

**Upright, on its side, locked.** The page follows the phone: held upright it has the upright
layout, turned on its side the one on its side, back and forth without a reload. Options (on a
device whose main pointer is a finger; remembered by this browser) has **Screen**: "Turns with
the phone" (the default), "Landscape" or "Portrait" (`assets/app/orient.js`). A lock is the
browser's where it has one: Android's browsers lock the screen in fullscreen, so choosing a lock
enters fullscreen and locks it (and on a later visit the first tap does; leaving fullscreen with
the back gesture ends the browser's lock). Where the browser has no lock (an iPhone's Safari, which
has no fullscreen either), or once fullscreen ended, the page turns itself: while the phone's
orientation is not the locked one, the page's body is turned 90 degrees so that its content stays
put on the phone (the direction the one the browser last showed), and the page lays out the locked
orientation. For that the page is shown a turned viewport: the stylesheets' media queries and
viewport units have width and height swapped (through the CSSOM, and back when the turn ends), and
`innerWidth`, `innerHeight`, `matchMedia`, pointer and touch coordinates, `getBoundingClientRect`
and `elementFromPoint` answer in the turned axes, so the layout code, the map's pointer and
pinch, the tooltips and the controller's focus work unchanged; taps reach their element in any
case (the browser hit-tests through the turn). On an iPhone with the system's rotation lock on,
the screen stays upright: a landscape lock then shows the game on its side, read by turning the
phone. "Turns with the phone" again ends the turn and the fullscreen the lock entered. On a
computer the choice is not offered, and nothing of this runs.

**A controller on a phone.** A game controller paired with the phone (a Bluetooth pad, or a phone
grip such as a Backbone or a Kishi) plays the game through the browser's Gamepad API
(`assets/app/gamepad.js`; its pure parts, the button reading, the repeat and the direction picks,
in `pad.js`, tested by `assets/test/pad.test.mjs`). Only a pad with the API's standard mapping is
read (its buttons are in the same places on every brand; the labels below are the Xbox ones, so
A is Cross and B is Circle on a PlayStation pad). Only a device whose main pointer is a finger
(`pointer: coarse`) has the controller's mode: on a computer a pad does nothing, and a phone
without one runs nothing of it. A browser shows a pad to the page only after one of its buttons is
pressed, so the pad's first press starts the mode and does nothing else (a note points at the
menu's Controls, which lists the buttons on a touch screen). A finger on the screen or a key on a keyboard ends the mode (the last input wins: the
touch controls above work as before), and the pad's next press starts it again.

On the map the mode has a cursor, a ring on a plot. The cursor hovers its plot as the mouse does:
the outline, the tooltip above the plot, and with units selected the way there with its turns
and attack odds. The stick and the D-pad move it plot by plot by screen direction, in every
renderer, repeating while held.

| | |
|---|---|
| left stick, D-pad | move the cursor |
| right stick | pan the view (the cursor comes to its centre when the pan leaves it out of view) |
| A | a plot pick: pick the cursor's plot. Units selected, none of ours there: go there or attack (the right-click's move, its preview was the cursor's hover). Else the click: select the units there (again: the next group), open our city, show the plot |
| B | back: as Esc (leave a plot pick, close the dialog, clear the selection) |
| X | as Space: skip the unit, or end the turn when nothing waits |
| Y | the interface: the focus goes to the selection's orders (else End turn) |
| LB, RB | the previous and next unit (Shift+Tab, Tab); the cursor goes to it |
| LT, RT | zoom out and in, about the cursor |
| Start | End turn (its checks as the button's) |
| Back | the menu (☰), with the focus on its buttons |
| a stick's click | centre on the selection (C) |

Every other layer has a focus, an outlined button: the battle screens, the top dialog, the title
screen, the open menu and, after Y, the interface around the map (only what is on screen). The
D-pad or the left stick moves the focus to the nearest button that way, the right stick scrolls
the list it is in, A presses it (a drop-down opens its list where the browser lets a script; a
text field takes the focus, which brings the keyboard up), and B is Esc (around the map: back to
the map's cursor; in a text field: leave it). A layer opening takes the focus (a dialog on its
primary button), and a re-render keeps it at the same place. An order or a unit pressed on the
selection's card gives the focus back to the map, for the next unit or the plot pick. What the
controller does not reach: the battle plan's board (it only shows the lines; its buttons are
reached), Shift waypoints, and Ctrl-click on a unit row.

**Units.** The selection panel lists your units on the plot (click a row: its group;
Ctrl-click: add or remove that unit; on the isometric and HD-2D maps they are a strip of icons
standing over the card, as Civ4's unit buttons: the selected ones lit, each with its health,
a pip for its moves and a mark for a promotion waiting or what it is doing, the rest in its
tooltip; inside the card on a phone), "All", "Group" (one group), "Split" (each unit its own
group), "Next", the group's orders, and the actions the server says the selection can take now
(greyed when not now): orders such as skip, sleep, fortify, sentry, heal, pillage, found, join,
spread, the great-person missions; go-to and route-to and the other modes that need a plot (click
it then; the valid targets are outlined); builds with their turns; spells with their help;
promotions; upgrades with their price; load, unload, wake, cancel, delete. The sections follow
Civ4's button order (Orders, Build, Great person, Religion, Cargo, Spells, Promotions,
Upgrades). "Promote…" lists the promotions with their help, "details" by Spells lists every
spell with its help and whether it can be cast now. A great person shows the card it holds
(docs/CARDS.md): its name, suit and band, its needs with the ones your team knows, and whether it
can be discovered now (its help in the tooltip; a "card" badge on the unit's row and tooltip); its
Discover action names the card ("Discover: Abjurer") and, while it cannot be cast, says what is
missing ("Needs Astronomy."). Units in a transport are listed under it.
With the art, the panel's head shows the lead unit's figure in its owner's colours, each row its
combat class, strength, health bar, moves and level, the lead unit's promotions are seals under
the list (and each promotion button carries its seal). With armies on (this game's own system,
docs/ARMIES.md) each row shows the unit's army (red when out of supply), "garrison" or "marching",
a whole army selected has a line with its size, target, the recruits marching to it and in
training for it, and its supply, and an **Army** section lists what the selection can do: form a
new army, split into a new army, join or merge into an army on the plot, join the city's garrison,
disband (march to the nearest city's garrison), upgrade all (the units, the most experienced
first, with the total price). A marching recruit (a unit trained for an army or another city's
garrison, on its way there) can be selected to look at: the panel says "Marching to the 2nd Army
(3 turns)" (or how long it has waited, or that it has no way there now) and where it came from,
and offers no orders (the rules move it until it arrives; "All", Shift-click on the plot, Group and
Split leave it out, so a selection of the units there keeps its orders); it is never among the
units waiting for orders. On the map its figure carries a small ochre double
chevron in place of the moves pip (the isometric map), and the plot's tooltip
says where it marches.
When a group is done, the next one waiting for orders on the plot it left, then on its plot,
then the next in Civ4's group cycle is selected, as in Civ4 (not with the player option "No
unit cycling", Options → Game). As in Civ4, the cycle (`Tab`, `Enter`, the next group after an
order) selects only groups whose units can all still move: a stack in which one unit has used its
moves still counts as waiting (the end-turn check counts it) but is not selected for you. Moving into a rival's land asks whether to declare war;
deleting, world spells and spells that start a war ask first; loading with more than one
transport asks which.

**Each turn.** When your turn starts the turn report fills the bottom panel (units and cities
gained or lost, growth, starvation, disorder, buildings, techs, contacts, wars and peace,
enemies near your cities, the Armageddon counter, borders, and the messages), the first unit
waiting for orders is selected, and the open questions come up one at a time in Civ4's order:
events (each choice with its effects; greyed when not possible now), votes, research (each
tech's tooltip says what it brings you; click: research it; Shift-click: add it to the queue),
then each city with nothing to build, in the build prompt: the city's size, food (surplus and
turns to grow) and production a turn in one row, its queue, and what it can build by group
(units, buildings, projects and rituals, processes) in a grid, each with its tooltip, turns and
production cost (click: build it now; Shift-click: add to the queue; Ctrl-click: put it first; the
"A click:" row does the same with a plain click). With the isometric and HD-2D maps on a desktop
the prompt stands at the right and the map centres on the city. "Open city" opens the city screen
(the prompt is set aside for this turn); "Later" sets it aside for this turn; End turn reminds you
of it. End turn (the button, or `Enter` once no group waits) asks first when an event, a vote or the research is
open, a city has nothing to build (the client's check: the server's gate does not count them) or
units wait for orders: answer them, choose production (the first such city's build list), select
a unit, or end the turn anyway (Civ4's
consequences: open events are dropped, votes get one more turn and then abstain, empty research
takes the automatic pick). A question the rules must have answered at once (keep or raze a
captured city, with its buildings and holy cities; a free tech, each with its worth in research
and what it brings; an election) appears as a dialog that cannot be closed, with the choices, the
advisor's pick marked, and "AI default".

A confirmation of something that cannot be undone (declare war, also by moving, casting or lending;
delete a unit; a world or card spell; adopt a doctrine; a revolution with anarchy; disband an army;
leave the game; replace a save) opens with
Cancel focused, so `Enter` does not confirm it. Toasts stand at the top of the map (in the HD-2D and
isometric looks on a screen 1100 px wide or more, between the turn report's panel and the city
panel's column, so they do not cover the report's tabs). A refused order shows a toast with the reason in
plain words (the game's names, never type names, ids or request details; those go to the browser
console); an error toast stays 10 seconds, a click dismisses any toast, and each error is also
listed under Messages at its turn, marked "Refused" (kept by the page only, until the game changes
or the page reloads).

**World rituals** (docs/CARDS.md, "World rituals"; `app/rituals.js`, `src/view/rituals.rs`; shots
in `docs/shots/world-rituals/`, regenerated by `node tools/world-rituals-shots.mjs --bin
<built>/ffh-play`). A civilization's world spell spends a charge made by building its ritual.
- *The rituals element* (`#rituals`, a group of the top bar): one chip per ritual in progress
  anywhere (yours also while queued with no production) and per charge held: the civilization's
  colour (none for one not met), the ritual (without "Ritual: "), the city, a progress bar with
  `37%` and `13 turns`, or `Charge held`. A civilization not met is `Unknown civilization`; its
  city shows (the start is announced to everyone). A click on a chip centres the map on the city;
  each chip's accessible name says it all ("Hippus: Ritual: Warcry in Lycus, 37%, 13 turns"). With
  chips it takes a row of its own under the bar (scrolling sideways when they do not fit; on a
  phone always its own row); with none it is one muted line, `No rituals in progress`, beside the
  world spell. **All** opens the full list: every civilization, idle ones too, each with where it
  stands (`In progress in Lycus: 37/100 (37%, 13 turns).`, `Charge held in Lycus.`, `Idle.`), a ⌖
  to show the city, and `Completed: N. Cost: C. Next cost: N2.`
- *Alerts*: each `<Civ> began|abandoned|completed <ritual> in <city>.` message (the log marks them
  `kind: "ritual"`) also shows as a toast with the ritual mark (12 seconds; abandoned 8); a click
  centres the map on the city. A loaded game shows its current turn's.
- *The world-spell button's tooltip* (`me.world_actions`: `why`, `charge`): no charge and nothing
  queued, `No charge. Build Ritual: Warcry in a city (cost 100).` (without the ritual's tech, `No
  charge. Ritual: Wellspring requires Knowledge of the Ether.`); queued or in progress, `No
  charge. Ritual: Warcry: 37/100 in Lycus, 13 turns.`; a charge held and castable, the effects then
  `Charge held in Lycus. Casting spends it. Next ritual cost: 200.` and the caster; a charge held
  that no unit can spend, `No unit of yours can cast it now.` (the button stays, disabled). Its
  cast confirmation shows the charge line. A placed world spell (Wellspring) says `Placing spends
  it.` and, in place of the caster, `Opens the placement: a marked plot on the map, then a mana
  type.`; with a charge and no valid plot, `Charge held. No valid plot now.`
- *The production dialog*: a world ritual's entry takes a whole row of the build list, with its
  cost and turns and `World ritual. One in progress per player. Taken off the head of the queue,
  its production is lost. Cannot be hurried with gold. Each completion doubles the next cost (next: 200).`;
  when it cannot be chosen, disabled with the reason: `Requires Knowledge of the Ether.` (a
  ritual whose tech the team lacks is listed, where the rules' visibility test would hide it),
  `Charge held: cast Warcry first.`, `In
  progress in Lycus.`, `Sanctuary holds: 3 turns.`, `No rival named: use Name the Tithe.` (Ritual:
  The Tithe), else `Not available now.` (the city holding the charge lists it too, disabled).
  Ritual: The Tithe with a rival named adds `Named rival: Luchuirp.`, as does your row of the full
  list. While the ritual with production is
  the city's current build, choosing another build (without Shift), putting one first or removing
  it asks `Abandon Ritual: Warcry? Its 37 production is lost.` (Abandon / Cancel, Cancel focused;
  the same for any ritual: every project is one, docs/CARDS.md). The list's group is `Rituals`.
  With a ritual first in the queue the gold hurry stays, disabled, with `Cannot be hurried with
  gold (Ritual).`; the population hurry is enabled when the rules allow it.
- *Name the Tithe* (the Calabim's top-bar action): its list of rivals labels each
  `<Leader>: <Civ>, at war|at peace`; picking one asks `Name <Civ>? This declares war on <Civ>.`
  (`Name <Civ>?` when at war already), Name / Cancel, Cancel focused.
- *Help*: the ritual's help lists its rules one a line; the civilization's World Spell part names
  the ritual and its base cost (`Warcry (charge: Ritual: Warcry, base cost 100)`); the help display
  has a "World rituals" topic.

**Cities.** Double-click your city, use the button in the selection panel when your units
stand in it, or the Cities list (`F1`). The city screen: the plots of the city's radius with
their yields (worked ones green, the city's own plot gold, plots another city works dashed;
click a plot to work it or to stop working it, which turns the citizen into a specialist, as in
Civ4, and stops the automatic citizens), the citizens as tokens (on a plot, a specialist, without
work, angry), the specialists' slots by type with what each yields (a click on an open slot makes a
citizen that specialist, on a filled one sends them back to a plot; greyed when the rules would
refuse), "Governor picks" (the automatic citizens), food and growth, production, commerce split by
the empire's rates with each total's parts, happiness and health with the room before trouble and
their parts in the tooltips (each list adds up to its total: the difficulty, buildings,
resources, ... against the population and the rest; a city short of health loses that much food),
culture, great people, maintenance, buildings, religions and the garrison;
hurry (gold or population, with the price), draft, the emphasis flags and "Automate
production"; the queue (✕ removes an entry while there are several; the only one is replaced by
choosing another) and everything it can build with turns and production cost, each with its tooltip (a
unit's strength, moves and class, what it does; click: build it; Shift: append; Ctrl: put it
first; the "A click:" row above the lists makes a plain click append or put first). The right column has the production first (its progress, the queue, the build lists),
then the orders, where trained units go (with armies on) and the governance.
"‹ Previous city" and "Next city ›" go through your cities. With cultural actions on
(this game's own system, docs/CULTURE.md) the screen also has a **Culture** section, under the
city's numbers: the culture rate and the half the channel takes; the city's channel (the action,
the civilization, the religion) with what it does now and "Stop": for Foment Revolt each of their
cities in reach with its distance, the pressure it adds a turn against the city's resistance, a
small scale (your pressure there now, where it settles) and what that brings (no effect, unrest,
revolt, the city joins you; a capital never) and when; for Spread Religion the chance a turn and
each city's part of it (or that it has the religion); then "Channel culture" (folded as "Change
the channel" under a channel): the action, the religion, the civilizations in reach with a
summary or the reason it is not possible, the preview of the picked one, and the button that
sets it. A city with no channel, no city of another civilization in reach and no revolt pressure
has the section as one line ("Culture: 2 a turn. No city of another civilization in reach.") with
"Channel culture" folded under it. Last, the revolt pressure on the city (one muted line when
there is none): a scale by civilization against the thresholds
(each point of unrest, revolt, the line where one civilization's pressure takes the city), its
level and unhappiness, what each civilization's channels add a turn and where that settles, and
its resistance (half its culture, 2 per defender; in revolt only its defenders, which the
predictions count). With armies on, each queued land combat unit
has a destination selector (the city's default, its garrison, another city's garrison, an army
with its size and whether it is in supply: a unit bound elsewhere marches there); an army's
requisition order leads the queue as "Requisitioned by the 2nd Army", without ✕ (it is released on
the Armies screen); after the orders, the city's default destination, its garrison and the armies
that requisition from it, with the garrison's battle tactic (a selector of the four tactics,
each with its rules as its tooltip; "from next turn (now Line)" while a change is pending;
docs/TACTICS.md). A **Governance** section (docs/GOVERNORS.md) ends the right column: "Ruled
directly. Government: Despotism." under a direct-rule government; otherwise the city's governor
(its trait's rule as the tooltip) and role ("Research (proposed)" or "(overruled)", with its
suitability). A governed city's build lists offer only what may be an empire order (units,
wonders, palaces, rituals; "Empire orders: units, wonders, palaces and rituals. The governor
builds the rest: see Governance below."): a click places it ahead of the governor's choice, an
empire order in the queue has a ✕ and the tag "empire order", the governor's orders and
requisition orders have none (a requisition order shows there too). The build prompt of a
governed city has the same lists under the same note.
With the isometric map the screen is Civ4's city view: the map glides to the city at the largest
zoom that shows its radius, veils what lies outside it and draws the radius in gilt, each plot
with its yields and the worked ones with a citizen; click a plot there to work it or stop. The
rest of the screen stands in panels at the sides, the interface steps aside, and the camera goes
back when the screen closes (docs/ISO.md, "The city screen").

**Empire.** **Techs** (`F6`): the tech tree, columns by prerequisite depth and rows in a band
per research path (doctrine), each tech a box with its name,
what it unlocks (icons, at most five with "+n"), and its turns and cost for
one you can research now, its cost otherwise, "known" for one you have; the states read without
colour too: known (a check mark, muted), researching (a progress bar with its percentage), queued
(its place in the queue in a corner), can research now (every prerequisite known: a gold ground, name and border), not yet
(dimmed). Zoom: the − and + buttons above the tree or Ctrl+wheel over it (40% to 100%, 80% at
first; the level is remembered). The bands (`treeLayout` in empire.js) are
the seven doctrines' lines of development: a tech of one doctrine is in its band (of several, the
one it is a capstone of), a tech of none in the band of most of its neighbours ("General" for one
with none), the bands ordered to keep the lines between them short; in a band each column's techs
sit level with their prerequisites, so a line runs straight. Alternating stripes mark the bands,
each named in the gap above it at the left edge (held there while the tree scrolls), the adopted
doctrine's band in gold. The prerequisite lines are faint, those between two bands fainter, and a
line that passes columns by runs along a row gap, not through boxes; pointing
at a tech (on touch, the first tap) lights its whole path and the techs it leads to and dims the
rest. The head says what is being researched and in how many turns; the tree opens scrolled to
that tech (else the first one you can research), with a scrollbar and edge fades where more lies
beyond. Each tech has its tooltip (over the tree, the cards you
know and those your great people hold, each with what it is and unlocks: its suit, band, needs,
content and effect; cards stay out of the tree); click a tech to research
it (with what it needs first), Shift-click to queue it; the "A click:" row (on touch "A tap:", where
a second tap researches) switches a plain click to queueing. **Civics** (`F3`): each civic option with its
civics (the current ones marked, the ones you cannot adopt greyed, their tooltip saying why: the
tech, a war, an alignment, a state religion); pick civics and the screen shows the
revolution's anarchy and upkeep (computed by the rules) and, when the government changes who runs
the cities, "Every city will be handed to governors." or "Every city will be ruled directly.";
then "Revolution!" (with anarchy, it asks first: `Start the revolution? 2 turns of anarchy.`).
**Religion** (`F7`):
the religions (founded or not; a religion's tooltip says what it does and, before it is founded,
the tech whose first discoverer founds it), their holy cities as far as you know them, your cities
with each, and "Convert" (with the anarchy). **Cultural pressure** (`F8`, the
menu's Culture; with cultural actions on): your cities under revolt pressure (each with its scale,
level, unhappiness, each civilization's pressure with what its channels add a turn and where it
settles, and when the next level comes), the channels aimed at you (with the cities of yours they
reach and what they do there), your channels with what they do (and "Stop"), the channels you see
between others (all channels are public to civilizations that have met the caster), and what
other leaders remember of your cultural actions. On the map a city you see under revolt pressure
has a mark at its banner's end: a small ochre dot while the pressure has no effect yet, an amber
"!" for unrest, a red "!" in revolt, a red "!!" in a gold ring when close to changing hands.
**Armies** (`F5`, the menu's Armies; with armies on): every army with its name
(type to rename), units, place (click: select it on the map), its supply in bare words
(`Supplied: 14 of 24, from Ithralia.`, `Hunger: turn 1 of 2. No healing, no recruits.`,
`Starvation: -10% HP a turn (to 25%), -25% damage dealt.`, or `No supply line: Hunger from your
next turn.` for an army supplied this turn that stands where it has no line now; a hungry or
starving army whose line is back adds `Supply line again: ... Supplied from your next turn.`;
docs/SUPPLY.md), its battle tactic (a selector of the four tactics, each with its rules as its
tooltip; "from next turn (now Line)" while a change is pending: a change takes effect at the
start of your next turn; docs/TACTICS.md), target size (with its units,
recruits marching and units in training against it: its requisition orders and the units its
cities build next for it, as "more wanted" counts them), "Upgrade all", "Disband" (it asks first:
its units join the garrison of the city they stand in, or march to your nearest city and join its
garrison; its requisition ends), its
**requisition** (the cities it requisitions from, how many more it wants, its orders in
production with their turns, and "Requisition from…": every city of yours with a checkbox, the
unit it would train for the army, its production turns and its march to the army; "Release all"),
and the recruits marching to it (where from, where they are (click: select it on the map), how
many turns away, or how long they have waited); then each city's garrison with the recruits
marching to it. Each army also lists its units on order (unit, city, turns) and has **Order
unit**: a unit that serves in armies (one no governed city can train is shown unavailable), then
the city step: a panel of every governed city able to build it, with its turns (behind the orders
ahead of it), what it would delay and its distance to the army; the nearest is pre-selected
(Enter or "Order in <city>" is one click), and "On the map" outlines the cities on the map, where a
click on one orders it there (Esc cancels). Below the armies, the **Fleets** section
(docs/NAVAL.md, section 7): every fleet with its name (type to rename), ships, place (click:
select it) and the sea it is in, its wear (`Wear: none.` / `Wear: -15% strength. Enter a port to
clear.`), its **standing order** (a selector: `No standing order`, `Patrol: <sea>` for each sea
with a plot in your range, `Escort: <army or fleet>`; a move order given to the fleet ends it),
target size, "Upgrade all", "Disband" (it asks first: its ships sail to your nearest port and are
then in no fleet; in port at once) and its requisition as an army's (the coastal cities only; the
ship each would train and its turns of sailing; the ships sailing to it). No supply, no tactic.
The unit panel of ships offers the fleet's actions (form a new fleet, split, join or merge, disband,
upgrade all; no garrison), and a whole fleet selected shows its line with its standing order.
**Governors** (`F9`, the menu's Governors): the mode
line ("Government: Aristocracy. Every city is governed."; under Despotism or God King "... ruled
directly." and the city list alone); then the six roles as priority toggles (at most two; "Priority:
+30% weight in role proposals."), the diagnoses ("No city has role Research.", "<City>: role Culture
at suitability 60%.", "<City>: <order> takes 34 turns.", each city a link to it), **Empire order**
(any unit, wonder, palace or ritual, through the same city step) and a role card per city: its
governor and trait, its role marked proposed or overruled, the role's key buildings with the city's
tier of each (in the tier's colour), the other roles with their suitability as buttons (disabled,
with the reason, under 75%), the governor's plan ("Plan: Forge, Granary, Market"), its empire and
requisition orders with their turns (an empire order with a ✕) and what the governor builds.
**Finances** (`F2`, or click the
commerce split in the top bar): each commerce's share with `−`/`+` by Civ4's step (only the ones
you may change; gold takes the rest), and the gold per turn by its parts, as in the gold's tooltip.
**Diplomacy** (`F4`): every player you met, by their leader's portrait (war or peace, open
borders, pacts, vassals, attitude with its reasons as its tooltip, alignment and state religion,
score, your deals with them; your own alignment and state religion above the list) with "Trade…" and "Declare war"
(it asks first). Each playable leader has a portrait, a half-length HD pixel-art painting
(`tools/m7/leaders.py`), also shown at the top of the trade screen, on a deal offered to you, on
the question to take over a civilization that appeared and by each candidate of a vote. The trade screen lists what each side can give (refusals greyed with Civ4's reason); click items into the deal
(gold with an amount; peace, open borders and pacts go both ways, so they are greyed on both
sides when either side refuses them), "Propose": the AI accepts or declines at once. Each item
shows the points the AI puts on it (+ for what it gets, - for what it gives; gold per piece;
"swap" or "no value" for an item it puts no price on; the tooltip says how the value is counted,
a resource or gold per turn, a lease, over 10 turns). Under the tables: "Total offered", "Total
requested", and whether the proposal will be accepted or refused, with the reason when a rule
rather than the totals decides. With only your side filled the deal is a gift: the summary gives
the current value of your gifts, this gift's value, the relations it brings (+1 per tier of 150
points, at most +3, for 30 turns) and the tiers. With only
their side filled it is a demand: the most they would hand over (nothing unless your power is at
least 1.5 times theirs and one of your cities is within 16 plots of theirs by land), what you ask,
and the effect if they pay (-1 relations for 30 turns, no further demands for 10 turns).
Resources and gold per turn are leases: the deal goes on until it is cancelled, which either side
may do after 10 turns (the deals column: "Cancel", or the turns left); the AI reviews its deals
with you from 20 turns and cancels one it would no longer make, saying why.

**Leaders speak** as in Civ4's diplomacy screen, with FfH2's own lines for each leader and,
unless the game was started with "Vanilla experience", lines their civilization has for yours
(`data/text/barks.md`: every civilization to every other, at each of the moments below). The trade
screen opens on the leader's greeting (by its attitude towards you; sometimes, as in Civ4, a word
about its worst enemy or a boast about its army), or on its refusal to talk (at war, until it is
ready for peace); the leader answers each proposal: thanks for a gift, its line for peace made or
refused, a deal welcomed or turned down, a demand rejected. A leader also contacts you, in the
leader screen: when you first meet (answer "Let's make a deal…" for the trade screen, or
"Farewell") and when it declares war on you ("So be it."). Those moments show when they happen, or
at the start of your turn when they happened in the AI turns; loading a game shows the moments
of its turn and the one before again.

**Options** (the gear; the title screen's Options too), in sections: **Game** (the player options
`set_option` changes: quick moves, no unit cycling, safe automation, wait at the end of the turn;
a refused change puts the box back and says why; without a game, a note), **Battles** (remembered
by this browser: "Battles fought against you": Watch, the default, Report, or Turn report only, see
"Engagement combat"; "Your attacks": Watch or Report, the battle plan's switch), **Map** (the
renderer: HD-2D, the default, isometric 2.5D or flat) and, on a phone, **Screen** (below). It
ends with **Debug tools…**, the only way to the Tools dialog.

**Tools** (Options → Debug tools…): **Debug reveal** (the whole map and every unit and city; the top
bar says DEBUG REVEAL); the **Inspector** (the rules engine's own data of your unit, group, city,
player or team, and of anything under debug reveal, plots too; an info entry by kind and index);
**AI autoplay** (the AI plays your civilization for N turns, then you have control again; while
it runs the top bar counts the turns left and "Stop" ends it: you have control at your next turn);
**Spectate** (below); **Take over…** (play another slot of an AI-only save); **Raw request…** (the
raw request box). From the title screen (no game) it lists only Take over… and Raw request….

**Help.** What a choice does is in its tooltip, as Civ4 shows it: one box that follows the pointer
(or the keyboard focus), stays inside the window, and keeps the help's icons, colours and lines.
The help texts are written by the server from the rules' tables (`src/view/help/`, ports of
C++ `CvGameTextMgr`): what a tech brings you (`setTechHelp`, with your own units and buildings and
only what you cannot do yet), what a civilization, a leader and a trait bring (`parseCivInfos`,
`parseLeaderTraits`, `parseTraits`), and what every building, unit, promotion, spell, civic,
religion, project, specialist, process and resource does and requires (`setBuildingHelp` with
`buildBuildingRequiresString`, `setBasicUnitHelp` with `setUnitHelp`, `parsePromotionHelp`,
`parseSpellHelp`, `parseCivicInfo`, `setReligionHelp`, `setProjectHelp`, `parseSpecialistHelp`,
`setProcessHelp`, `setBonusHelp`). Those are static: what the type does anywhere, as the
Civilopedia says it, not what it would add in a given city. Their requirements are written
plainly, since nothing is known to be unmet (Civ4 colours only those a player lacks); a screen
that knows the player says which it lacks (the civics screen, above the civic's help). Where
FFH2's help says nothing of a rule the game checks (a building's religion or coast, a religion's
alignment and what it unlocks, a resource's affinities), ffh-play adds the line. Every field of
every table is either described or listed as not shown, so a new field cannot go unsaid. Built
from the data, they follow the rules as they change. Each appears wherever its type does: the
build list and the buildings of a city, the unit panel's actions, the promotions and spells
dialogs, the civics and religion screens, the trade table, the map's resources.

**Rules stay out of the screens** (the owner's call, 2026-11-03). A screen holds the state and the
decision at hand; a rule (how something works, true in any game) is a tooltip on what it explains,
or, for a screen's whole system, an article of the help display behind the dialog's (i): the
crusade dialogs, the Armies, Governors and Cultural pressure screens. A decision's option shows
what that choice brings (an event's effects, a tech's help, a cost); the rules behind it (what a
settlement is, who gets a returned city) are its tooltip (the option's `tip`).

**Watching the AI.** Tick "Spectate the AI" in the new-game screen for an **observed
game**: an AI-only game (the AI plays every civilization, also the one you pick, which is the one
the view follows first) watched with the whole map shown. It is the game `ffh autoplay` plays
with the same settings, move for move (`ffh-cli`'s tests check it at every turn boundary), so
what you see can be replayed by the AI's developers. The top bar has ▶ Play / ⏸ Pause, ⏭ Step
(one turn), the pace (Fast, Normal, Slow, Very slow: the pause between two turns, remembered by
this browser), **Follow** (the civilization whose gold, research and score the top bar shows; its
stacks are drawn as yours) and **⚑ Flag** (on a phone, a row of their own above the bottom
bar). Nothing can be ordered: a click on the map shows the
plot in the side panel (each unit with its strength, health and level, the AI's role for it and
its group's orders, and a line for the stack: how many units, how many can fight, the strongest,
the owners), commands are refused, and Next turn (Enter) plays one turn. After each turn the map
marks where that turn's fights and captures happened (a crimson ring with a cross: a unit died; a
dashed amber ring: a retreat; a gold square: a city captured or razed; the hover tooltip and the
side panel say what), and the minimap shows every stack of 2 or more units as a dot in its
owner's colour, larger for 5 and for 10 units or more. Messages has every civilization's messages with
their sender (a box narrows them to the followed one); combat messages give the attacker's odds,
and ⌖ shows the plot on the map. The game is over when it is decided (a victory or the turn
limit: where `ffh autoplay` stops too). Tools → AI autoplay… plays many turns at once (the top
bar's Stop, or Pause, ends them after the turn under way). An autosave of an observed game loads
paused, as observed (one taken during such an autoplay goes on with it); so does any other
AI-only save (Take over… plays one of its slots instead).

Tools → Spectate in a game of your own still lets the AI play your civilization too, one turn at
a time with debug reveal on (Stop gives you control back). That game has a human player, so it
is not an `ffh autoplay` game: its flags quote its journal instead of its settings.

**Flags.** When play looks wrong, flag it: F with the mouse over the plot, ⚑ Flag in the top bar
(the plot in the side panel, else the next one clicked), "⚑ Flag this plot…" in the side panel,
or ⚑ beside a message. Watching pauses after the turn under way, you add a note, and the flag is
saved as a line of the game's `flags.jsonl` and copied to the clipboard as text for a bug report:
the plot, the turn whose moves are on screen (observed, the turn that just ended; at your own
turn, the previous one), the units on the plot and next to it (ids, owners, groups, the AI's
role, orders, hit points), the plot's combat and capture messages, the build (the commit; also
under Game files), the game's settings, its checksum, and the commands that replay that turn with
the AI's trace on for the player involved:

```
FFH_AI_TRACE=3 ./target/release/ffh-play observe --setup '{"seed":3,..,"observe":true}' --to-turn 58 --plot 12,34
FFH_AI_TRACE=3 ./target/release/ffh-play observe --load saves/game-3/flags/flag-1.ffhsave --to-turn 58 --plot 12,34
FFH_AI_TRACE=3 ./target/release/ffh autoplay --seed 3 --civs 4 --size WORLDSIZE_SMALL --turns 59
```

The first replays the game from its settings, the second from the save at the start of that
turn, which the flag copies to `flags/flag-N` (the autosaves keep only the last 20 turns, seconds
of watching at the Fast pace); both print each turn's checksum (the last is the flag's) and the
units at the plot. The third is the same game in `ffh` (docs/AI.md, "Tools"). The trace lines of
that turn are tagged `[t58 p3]`. A flag in a game with a human player quotes `ffh-play replay
JOURNAL --from flags/flag-N.ffhsave --to-turn T` instead (pinned only when that autosave is
this session's: after a load, an earlier session's autosaves do not replay with its journal). The Flags tab of the bottom panel lists
the game's flags (⌖ shows the plot, "record" the text, "copy" copies it) and the map marks them
with numbered pennants; Load lists the pinned saves as `flags/flag-N`. On the static site the
files stay in the browser: the copied text is the report (its settings replay the game).

**Saving.** Save asks for a name and writes it to the game's directory; the game also autosaves
at every turn boundary (`auto/T0057` is the end of turn 57; the last 20 are kept). Load lists
every game directory with its saves (yours first, then the autosaves, newest first).

**When something breaks.** If the rules crash, the page shows the message and a bug report
(the last autosave, the journal and the `ffh-play replay` command that reproduces it; Copy puts
it on the clipboard) and offers to load the autosave: the server stays up. If a turn does not
finish (the watchdog), load an autosave. If the page loses the server it says so and retries;
a reload is always safe, also in the middle of a decision.

**For testers.** `ffh-play serve --assets crates/ffh-play/assets` serves the client from disk,
so an edit shows on reload; the art then comes from `crates/ffh-play/web/art` too (the
directory beside the assets: its modules, and the sprite bundle made on each request), so an
edit to a sprite file shows on reload as well. `window.ffh.mapView.art.stats` has the last
frame's times by phase and the chunks drawn. `ffh-play stage capture|attack|automoves|load|trade|diplomacy DIR` writes a save at a
staged situation into `DIR` (put it under the saves directory, e.g. `saves/staged`, then load
`staged/capture`): `capture`, a warrior next to an undefended enemy city (moving in raises the
keep/raze dialog); `attack`, a warrior next to a barbarian warrior (the odds tooltip, an attack);
`automoves`, one unit waiting while another has a long move queued (skip the first: the second
moves on, Civ4's AutoMoves); `load`, a warrior and two triremes in a coastal city (Load asks
which trireme); `trade`, a civilization met at peace with gold trading and 100 gold on each side
(Diplomacy, Trade…: a gift of gold is accepted); `diplomacy`, an AI leader that has just met you
and then declared war on you (loading it shows both leader moments; it then refuses to talk). It writes only `DIR/KIND.ffhsave` and its sidecar (no journal, no autosaves), so
`DIR` may be a game's directory. `ffh-play stage supply DIR` writes two armies of yours: one
supplied, its line three plots or more to a city, and one in Hunger (turn 1) where no line
reaches (docs/SUPPLY.md). `ffh-play stage sea DIR` writes the sea (docs/NAVAL.md): a coastal
capital with the plots within 16 revealed, a Trireme next to it (in range), a Trireme just out of
range with 14 turns at sea (worn), a Warrior in boats and a Warrior on land beside the water; at
war with the other civilization, every sea the capital touches held by its patrols (the capital
blockaded), the nearest other sea held by a patrol of yours, the next contested, and a Galleon of
yours whose lane is cut. `ffh-play stage rituals DIR` writes the world rituals (docs/CARDS.md):
you play the Hippus with two cities, the capital building Ritual: Warcry (37% done); of four other
civilizations three are met (one building its ritual, one holding a charge, one idle with one
completion, its cost doubled) and one is not (building its ritual); the log holds the `began`
messages. `ffh-play stage rituals-charged DIR` is the same with you holding the Warcry charge and
no ritual queued. `ffh-play stage tithe DIR`: you play the Calabim (two cities, two Vampires, no
rival named), three civilizations met (two at peace, one at war), one not. `ffh-play stage
ritual-civ:CIVILIZATION_X DIR`: a duel as X with its ritual's tech, a charge held when its ritual
leaves one, and its ritual actions offered (the Lanun's storm raised, a Khazad tunnel with a warrior
on its end, two Doviello warriors). `ffh-play stage ritual-due:CIVILIZATION_X DIR`: the same, but
instead of the charge the ritual stands 1 production short in the capital, completing at the end
of the turn (Wellspring's placement then opens by itself). See
"Testing the browser client" below for the checklist.

**Engagement combat** (docs/ENGAGEMENTS.md; always on: the game's land combat, not an option). An
attack by land on a land plot is one formation battle: each side's line on a grid of three rows by its
lanes. In the browser:
- **Attacking.** The tooltip over an enemy plot next to the selection says it is a battle ("Battle:
  5 of yours can fight, against 3 defenders"; the odds beside it are the duel's of the strongest
  pair, a rough guide). A right-click there (a second tap on a phone) with an army or a stack of
  units that can attack opens **the battle plan**, each side in the formation its battle tactic
  gives (docs/TACTICS.md: the tactic of the army or garrison most of the side's units belong to;
  Line for a side with none); Attack (or `Enter`) sends `engage` (an attack that declares war says
  so on the button), Cancel (`Esc`) leaves the map as it was (not once the attack is on its way:
  the battle's screens take over from the plan). Shift+right-click on such a plot
  attacks at once (a `move`); on any other plot it lays a waypoint as
  before. Over a plot further away, or next to units that have used their moves or their attack,
  the tooltip says "Battle on arrival" ("once they can attack") with the selected units that could
  fight there, and the path's odds chip stays; a right-click sends the move, and the battle on
  arrival takes the tactics of whoever stands next to the plot then. A lone unit's
  attack, and every attack at sea or by air, goes as before.
- **Every battle you fight opens the battle screens** (the owner's request, 2026-11-03). Your
  attacks are watched or go straight to their report as you chose on the plan (its Watch / Report
  switch, also in Options → Battles as "Your attacks"): an attack made from the plan goes on to
  them, an attack without the plan (Shift, a move, a move order that attacks) opens them when it is
  made. The battles fought against you during the other players' turns open when your turn comes,
  oldest first, one after the other (the deferred popups wait for them), as Options → Battles,
  "Battles fought against you", says: Watch (the default), Report (straight to each report), or
  Turn report only (no screen opens; they stay in the turn report's Battles list and under
  Messages with their links, and the map markers stay). With more than one queued, the battle and
  report screens say "Battle 2 of 5" and offer **Skip the rest** (this one closes, the rest count
  as shown and stay in the turn report). A battle whose log cannot be loaded says so in a toast
  ("Battle at X could not be loaded. It is listed in the turn report."). The other messages of an attack (a defensive strike, the city captured) wait
  until the screens close, so they do not tell the end before the replay does; a city's
  keep-or-raze question still comes at once, as the engine asks it while the attack is made.
  After a load or a reload, the battles of the turn so far are not shown again by themselves; nor
  while you spectate or observe (the AI plays your slot). The struck plot flashes (from the
  battle's report: several units may fall at once). The battle's message under Messages and the
  turn report's **Battles** list (your battles since your last turn began, against you too) have
  **Watch** and **Report** links. A marker sits on the plot's top-right corner for this turn
  and the last (crossed swords: a gold rim for a battle won, azure held, crimson lost or
  repelled; drawn at 32 px tiles and up, inside its plot): a click on it watches it again, a click
  elsewhere on the plot selects as usual. A battle older than the kept logs shows its summary
  only.
- **Units.** The unit panel shows the row the lead unit prefers in a battle and what it does there ("Front: 2 strikes"), its A, Dv and HP in a battle, its speed,
  whether it is large and its actions in the front, middle and rear rows (the view's `engage_*`);
  a unit row's battle badge has the same in its tooltip. The help of first strikes, withdrawal, collateral damage, Blitz and combat
  limits (units and promotions) adds what they mean in a battle. The Stack attack player option is
  ignored (Options does not list it).

**The battle screens** (formation battles, phase 6 revision 4; the owner's prototype's
battlefield). They open from the game as above (the plan from an attack, the battle and the report
from a battle's links and marker), through the engine's `engage_preview`, `engage` and `battle_log`
(`apiSource` in `battle.js`); a dev entry opens them over a fixture, `?battle=field` (or `forest`,
`hills`, `city`, `raid`: a city that falls). In an overlay over the dimmed map, on one isometric
battlefield of M7 tiles (six ranks deep, the attacker's rear to the defender's rear, by the lanes;
the ground's scenery round it: trees round a forest or jungle, the city behind its defenders, hills
on a hill): **the plan** (both lines as each side's battle tactic arranges them, theirs as you
know it, and both tactics named ("Tactic: Charge", its rules as the tooltip; docs/TACTICS.md);
the board is never edited by hand: it takes no drags, a click on it moves nobody, and there is
no Auto-arrange; each unit's HP bar and its chance to fall; a tooltip over a unit says what it
is: A, Dv, HP, speed, its actions in each row and whom they reach, its spell, its specials, its
chance to fall, that it starves; the pool's units its tactic left out of the line listed, with
where the tactics are set (the Armies screen, the city screen); the slots and lanes of each side
and why; the ground (Hills, Forest, City, Open ground: its rules as each name's tooltip); the
forecast: each side's expected losses, their line wiped out, the plot falls, yours wiped out,
and the units of starving armies on either side ("Starving: 2 of yours, 0 of theirs: -25% damage
dealt."); Watch or Report after the attack; Attack or `Enter`), **the battle** (the log played
one action at a time as a snap, the owner's pick of the battle-feel spike, 2026-10-31: every
unit breathes; on an action's contact frame the pair cuts to its impact poses (the strike strip's
contact frame and the target's flinch frame where the unit has anim-sprint frames, else its key
poses (`app/pose`: one attack and one hit frame, every unit without strips), else the sprite leaning in, a
little larger, and the target leaning back under a white flash) and slides
together, lifted above the field with an ink outline, and the slash or the sparks, the number and
a small shake land; a held beat of 0.5-1.1 s with embers drifting, then a quarter-second ease
back. Crits hold longer and kills hold, the death playing after the return; on crits and kills only
the rest of the field drops into a shade round the pair, the camera never zooms; CRITICAL and
SLAIN under the numbers; a miss is a dodge with a whiff, a heal glows, a shot or a spell arrives
as a streak while its shooter stays on its cell, a bombard lunges and its blow lands as one held
image (the impact card, `app/fx`: the rubble arrives fast over the target, brakes hard, holds and
fades, larger over a splash's group; anim-sprint/ANIMATING_UNITS.md, "Impact cards and key poses"), a
ranger stays on his cell and his hawk strikes for him (his card), splash targets snap together, a knock-back eases
back a row on the return, an escape leaves after it; no shake under reduced motion; each side's
standing units; the log beside it under round and pass headers, a unit's speed before its line,
an action's line on its contact frame (a starving side's line before the battle, "Starving: Axeman,
Archer: -25% damage dealt.", with "starving" over each of them; each side's tactic under its
name); play/pause `Space` (the action freezes where it is), step
`→`, skip to the report `Esc`, speeds 1x (the default) to 8x `1`-`4`; played through, the report
follows) and **the report** (the battlefield where the battle ended, whether the plot fell, each
side's tactic and its fallen, escaped and standing with their heads, the whole log; Replay,
Continue). While the overlay is open no key reaches the map: the game's own hotkeys (the F-keys,
`Ctrl+S`, `Ctrl+L`, `Ctrl+1`-`3`, `Tab`, `Backspace`) do nothing, the browser's other keys keep
theirs. The
modules: `battle.js` (the overlay, the keys, the sources; the viewer's preferences are kept by
`engagements.js`),
`battle_plan.js`, `battle_watch.js` (the replay's clock and timeline), `battle_snap.js` (the
snap: each action's timing plan, roles, poses and drawing over the stage), `battle_report.js`,
`battle_stage.js` (the battlefield, the figures and the effects; the anim-sprint strips in
`app/anim/`), and the pure `battle_log.js` (the protocol's shapes in its header, a formation's
cells, the log folded into states and lines, the words of the rules by their codes); `battle_fixture.js` is the fixture, made by `assets/test/make_battle_fixtures.mjs`
from the owner's prototype (`assets/test/fixtures/proto/`: its engine and sample armies, not the
engine's numbers).

## Commands

```
cargo build --release
./target/release/ffh-play kinds                       # every named kind, its source and count
./target/release/ffh-play names UnitType              # JSON array of a kind's type names (index order)
./target/release/ffh-play names --pretty              # {kind: [names]} for all kinds
./target/release/ffh-play lookup UNIT_WARRIOR UnitType:3 ORDER_TRAIN
./target/release/ffh-play bot --seed 3 --turns 150 [--opponents 3] [--world WORLDSIZE_SMALL]
                              [--saves DIR] [--quiet] [--verbose]
./target/release/ffh-play replay DIR/journal.jsonl [--from DIR/auto/T0110.ffhsave] [--to-turn T]
./target/release/ffh-play observe (--setup JSON | --load SAVE) [--to-turn T | --turns N] [--plot X,Y] [--saves DIR]
./target/release/ffh-play journal print DIR/journal.jsonl
./target/release/ffh-play soak run --seed 2 --size tiny --civ kuriotates [--options raging] [--turns 300]
                              [--speed quick] [--handicap deity] [--saves DIR] [--resume DIR/auto/T0123.ffhsave]
                              [--check-every 5] [--save-check-every 25]
./target/release/ffh-play soak [--seeds 1..2] [--sizes tiny,small,standard] [--civs ..] [--options default,raging,occ,challenges]
                              [--speeds normal,quick] [--turns 300] [--jobs N] [--timeout SECS] [--saves DIR]
                              [--keep-saves] [--out results.tsv] [--filter TEXT] [--list] [--check-every N] [--save-check-every N]
./target/release/ffh-play agent [GAME] [--on-eof quit|end-turn] [--progress]
./target/release/ffh-play serve [GAME] [--port 8642] [--bind ADDR] [--assets DIR]
./target/release/ffh-play stage capture|attack|automoves|load|trade|...|rituals|rituals-charged DIR
./target/release/ffh-play bench [--quick] [--seed N] [--turns T] [--runs R] [--world W] [--opponents N] [--ffh PATH] [--json]
    GAME: [--saves DIR | --no-saves] [--autosaves N] [--setup JSON] [--seed N] [--opponents N] [--world W]
          [--civ C] [--leader L] [--handicap H] [--speed S] [--new] [--no-game] [--load SAVE]
          [--take-over SAVE --player P] [--panic-at T]
```

* `lookup` prints one JSON line per reference (`kind`, `index`, `name`, `display`, `text_key`, or
  `error`) and exits with code 1 if any reference does not resolve. A kind is named by its Rust
  type (`UnitType`) or its table (`units`).
* `bot` plays the human slot with the scripted test bot (design §8 T1), which reads the game only
  through the queries below and acts with commands, and prints one line per turn and the bot's
  and session's counters; `--verbose` also prints the `text` query each turn (what an agent
  reads). With `--saves DIR` it writes the journal and the autosaves there.
* `replay` re-plays a journal from its first line (a new game), or `--from` an autosave of the
  same game; a journal that starts with a load or take-over line replays `--from` the save it was
  loaded from (its sidecar next to it) or taken over from. It stops with exit code 1 at the first
  difference (a stamp, a checkpoint checksum, a decision of another kind). Use it to reproduce a
  bug: send the journal and the last autosave (or the save the session was loaded from).
  `--to-turn T` stops at the first control point after the moves of turn T (a flag's repro).
* `observe` plays an observed (AI-only) game headless: from new-game settings (`observe`
  implied; the `--setup` of a flag) or from an observed or AI-only save, to the end of turn T
  (`--to-turn`) or N turns; it prints each turn boundary's checksum and, with `--plot`, the units
  on and next to that plot. With `FFH_AI_TRACE=<player>` it is a flag's repro (Flags, above).
* `journal print` shows a journal with type names.
* `soak run` lets the AI play the human slot through the session for one game (design §7 P2):
  production, research, event popups and votes are answered with the AI's choice through the
  commands (one turn in seven they are left open and the turn is ended with `force`; the
  research the AI's own turn sets meanwhile is cleared again with `research {tech: null}`, so
  Civ4's automatic research pick runs), blocking decisions take the advisor's pick, the AI moves
  the units, and ffh-stress's invariant checks run every 5 turns (`--check-every`), a save round
  trip every 25 (`--save-check-every`; 0 = never). It prints `PROGRESS`, `END`, `CITIES`,
  `STATS` lines, or `PANIC` (with the workspace frames of the backtrace, exit code 101),
  `STUCK` (exit 4) or `ERROR` (a command the session offered was refused, a check failed;
  exit 5), each with a `REPRO` line: the game's own command with its check settings when they
  are not the defaults, plus `--resume <autosave>` when `--saves` kept autosaves (every game is
  deterministic, and resuming an autosave ends exactly as the uninterrupted game). Sizes,
  civilizations, options, speeds and handicaps may be shortened (`tiny`, `mercurian`, `raging`,
  `quick`); numbers out of range are refused. `occ` (one-city challenge)
  and `challenges` (cut losers, high to low, increasing and flexible difficulty) are option sets
  that act only on human players. `--civ infernal` or `mercurians` (not playable at the start)
  plays an AI-only game whose player 0 founds them at turn 30 (`--spawn-turn`) and takes the new
  player over, with your `--handicap`.
* `agent` plays one game over JSON lines: each line you write to its stdin is a request, each
  line it prints is a reply or an event (see "Transports" below). It starts a new game (the
  defaults, or `--setup '{"seed":3,"world":"WORLDSIZE_DUEL","opponents":1}'`, or the flags
  `--seed`, `--world`, ... which set the same members) unless `--no-game`, `--load SAVE` or
  `--take-over SAVE --player P`. Example: `printf '%s\n' '{"k":"q","q":"text"}' | ffh-play agent --seed 3`.
* `serve` starts the HTTP server on `http://127.0.0.1:8642/` with the browser client (open it in a
  browser; "Playing in the browser" above). No game is started unless the GAME flags say so; the
  page or a client starts one. `--port 0` picks a free port (the address is printed). `--bind
  0.0.0.0` listens on every interface (then the `Host` check below is off: only do this on a
  network you trust). `--assets DIR` serves the client files from `DIR` (`crates/ffh-play/assets`:
  live editing).
* `stage` writes a save at a staged situation into `DIR` for testing clients (`capture`: a warrior
  next to an undefended enemy city, moving in raises the keep/raze decision; `attack`: a warrior
  next to a barbarian warrior; `automoves`: skipping the one ready unit runs the AutoMoves rule;
  `load`: a warrior and two triremes in a coastal city, loading asks which trireme; `trade`: a
  civilization met at peace, Currency known and 100 gold each, so the trade tables hold gold;
  `diplomacy`: an AI leader's first contact and war declaration, shown as leader moments on load;
  `engagement`: armies and engagement combat on, an army of two Axemen, an Archer, a Horseman and
  a Warrior next to an AI city at war held by two Warriors and an Archer: attacking it opens the
  battle plan, and the battle takes the city; `cards`: the decks in play (docs/CARDS.md), the
  Starmetal Meteor and Legacy of Kings cards known (their world spells in the top bar), a Great
  Bard holding the Anointed Champion card with its need missing, the Armageddon deck dealt with
  its slot 30 revealed; `rituals` and `rituals-charged`: the world rituals, above),
  and prints
  its path and the ids as JSON. It writes only `DIR/KIND.ffhsave` and its sidecar (the staged
  game keeps no journal and no autosaves). Nothing in play uses it.
* `bench` measures the session's performance (design T13): the AI time per turn of a game the
  test bot plays against `ffh bench` (the `ffh` binary next to `ffh-play`, or `--ffh`), the
  session's own cost on one AI-played trajectory (its own work against the `update()` time
  inside its runs, which is budgeted; and, reported only, a bare update loop, the session and the
  session with its autosaves timed whole, `--runs` interleaved rounds, the same game each time:
  separate runs differ by a quarter or more on a busy host, even in the thread's CPU time),
  command latency on a standard map at turn 200, the view on a huge map and the status query; it
  prints each against its budget (`--json`: the figures) and exits 1 when one is missed.
  `--quick` is a small variant (seconds).
  The full run takes about a minute.
* GAME flags: each game gets a directory in `--saves DIR` (default `./saves`), `game-<seed>`
  (`-2`, ... when taken), with its autosaves (`auto/`, `--autosaves N` kept, default 20), journal
  and manual saves; `--no-saves` writes nothing. `--panic-at T` panics once when the game reaches
  turn T (or as a loaded save of turn T or later starts), a self-test of the crash report.
* `soak` runs the matrix (default: 2 seeds × tiny, small, standard × Kuriotates, Infernal,
  Sheaim, Mercurians, Bannor, Calabim × default, raging barbarians, `occ`, `challenges`, 300
  turns, normal speed; `--speeds` adds speeds, a game given twice runs once), each game in a
  child `soak run` with a timeout, `--jobs` at a time; it prints a summary (per civilization,
  size and option set; the human's cities against the AI civs'; every distinct failure with its
  repro commands, which include `--check-every` / `--save-check-every` when given) and exits
  with code 1 if a game failed. Its arguments are checked before any game runs (`--out` is
  created first; a bad setting is a usage error, exit 2). With `--saves DIR` each game keeps
  its last 3 autosaves in `DIR/<game>/` (the description and a hash of the settings) while it
  runs (and after a failure, for the repro). For a checked build (debug assertions and overflow
  checks): `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true
  cargo build --release --target-dir target/checked -p ffh-play`, then run
  `target/checked/release/ffh-play soak ...`. A checked build stops long games on an ffh-core
  debug assertion when the Syliven's Perfect Lyre is taken out of a city and dropped again
  (reported; design §7 P2): `soak run --seed 9 --size tiny --civ bannor --speed quick --turns 400`
  at turn 276. Release builds play on.

## The session (library API)

```rust
use ffh_play::{NewGame, Session, SessionOptions, Command, CmdOk, Stop};
let mut s = Session::new_game(&NewGame::new(ffh_data::global(), 7), answerer, SessionOptions::default())?;
loop {
    match s.advance(&mut |_| {}) {           // runs AI turns until the human has control
        Stop::Human => {}
        Stop::Over | Stop::Stuck(_) => break,
    }
    let pending = s.pending();               // events, votes, research, idle cities, ready groups
    s.execute(&Command::Research { tech: Some(t), shift: false })?;   // a CvNet* port
    s.execute(&Command::EndTurn { force: true })?;
}
```

* `advance` runs `update()` until the human's next **control point** (its turn is active and no
  group of it is busy) or the end of the game. When nothing is ready but units have queued
  orders it sends `auto_moves` once (C++ `updateTestEndTurn`), so multi-turn moves continue.
  Civ4 checks this every frame: after a command that leaves nothing ready while orders are
  queued, `auto_moves_due()` is true and a client calls `advance` again (the transports do).
* `execute` applies one `Command` at a control point. It validates first (ids yours, types in
  range, the rules' own `can*` gates, as the Civ4 interface only offers legal actions); refusals
  (`not_your_turn`, `decision_pending`, `invalid`, `not_owner`, `gone`, `cannot`) change nothing.
  Where Civ4 asks for confirmation the reply is `NeedsConfirm` (declare war to move, delete,
  global or war spells, which transport to load into, ending the turn with things open); resend
  with the flag set.
* `query(&Query)` (or `query_json` of a `{"q": ..}` object) answers a protocol query from your
  side of the fog (below); `set_reveal(true)` switches debug reveal on (the true state, every
  reply marked `"reveal": true`).
* `read(|g| ..)` reads the game (`&Game` reads have no side effects on the game).
* `save(name)` / `save_bytes()` work at a control point or turn boundary; `Session::load`,
  `Session::take_over` (an AI save, debugging) resume.
* FfH can move the human to another slot (`reassignPlayerAdvanced`: the Impersonate Leader
  spell for its 5-14 turns, the "switch civilizations" event, the High to Low challenge). The
  session follows: `Session::human` is the slot you play now, and the active player moves with
  you as in Civ4. Messages follow you too: when the move happens during the turn processing
  (High to Low at turn 50, the end of the Impersonate Leader timer), the log of that game update
  is read as the new slot's, so the old slot's messages from its turn as an AI reach nobody, as
  in Civ4; its messages from just before the move in that update are lost (design §7 P2).

### Commands

`move`, `mission`, `do_command`, `join_group`, `auto_mission`, `push_order`, `pop_order`,
`do_task`, `culture_channel {city, action?, target?, religion?}` (this game's cultural channel:
`{"c": "culture_channel", "city": .., "action": "CULTURALACTION_FOMENT_REVOLT", "target": <player
id>}`, with `"religion"` for `CULTURALACTION_SPREAD_RELIGION`; the city channels that from now on,
replacing its channel; no `action` (or null): it channels nothing; checked by the rules'
`culture_channel_check`, a refusal says why; docs/CULTURE.md), `army_form {units}`, `army_split {units}`, `army_join {units, army}`,
`army_garrison {units}`, `army_rename {army, name}`, `army_disband {army}`, `army_upgrade {army}`,
`army_target {army, size}`, `army_requisition {army, city, on}` (the army requisitions from the
city, last in its list, or no longer does: its order there is withdrawn), `army_requisition_set
{army, cities}` (the army requisitions from these cities, in this order, and no other),
`army_tactic {army, tactic}` and `garrison_tactic {city, tactic}` (battle tactics, docs/TACTICS.md:
`tactic` a `BattleTacticType`, by name (`"TACTIC_CHARGE"`) or index; the army, or the city's
garrison, fights in it from the start of your next turn (`tactic_next` until then; setting the
tactic it has cancels a pending change); refused for another player's army or city, `invalid` for
an unknown tactic; refused for a fleet), `army_station {army, station}` (a fleet's standing
order, docs/NAVAL.md 7.2: `station` null (none), `{"patrol": sea}` (a sea index; refused out of your
range or for a fleet with no warship), `{"escort": army}` (an army or fleet of yours) or
`{"escort_group": group}`; refused for an army), `recruit_dest
{city, index, dest}` (this game's armies: `dest` is `{"garrison": city}`, `{"army": id}` or null
for the city's default; `index` -1 sets the city's default; a requisition order's is its army's;
docs/ARMIES.md. A marching recruit takes no orders: `move`, `mission`, `do_command`
(also the units of its type an `alt` command reaches), `join_group` (as the unit or the head),
`auto_mission` and the army commands refuse it, the reason naming where it marches, "the Warrior
is marching to the 2nd Army: it takes no orders until it arrives". A requisition order cannot be
taken off by hand: `pop_order` refuses it, and `do_task TASK_CLEAR_ORDERS` while one is queued,
"requisitioned by the 2nd Army: release the city from the army's requisition on the Armies
screen"), `govern_role {city, role}`, `govern_priorities {roles}`, `empire_order {city, order,
data, dest?}` (`dest` as `recruit_dest`'s, a unit only), `empire_order_remove {city, index}`
(this game's governors, docs/GOVERNORS.md: the rules' own checks and reasons, "Role requires
suitability 75%.", "Empire orders: units, wonders, palaces and projects only."; in a governed
city `push_order` places an empire order and `pop_order` takes one off, the governor's and the
requisition orders refused, and `recruit_dest` and the production tasks are refused), `engage {army | units | from, to,
declare_war?}` (engagement combat, docs/ENGAGEMENTS.md: one formation battle on plot `to`, `{x,
y}`, next to the attackers; `army`, the selected `units` or a plot `from` say who attacks (every
unit of yours on the attacking plot that can attack the plot now is in the pool), and the
launcher's group moves in when no defender is left on the plot; each side fights in the formation
its battle tactic gives (docs/TACTICS.md); a `formation` is refused, "An army fights in its
tactic." (the rules' own entry still takes one: the lab, tests); an attack that would declare
war answers `{"confirm": {"kind": "declare_war", ..}}` as `move` does until it is resent with
`declare_war: true` (`false` cancels it); refused, with the reason, for a plot you do not see,
not next to the attackers, at sea, with nobody to fight or no unit that can attack; the reply
carries `report`, the battle's report id, and `battle`, its summary; through
`group_attack_force`, as every attack; a `move` onto an enemy land plot is one battle too, and
its reply carries the same `report` and `battle`;
`PLAYEROPTION_STACK_ATTACK` is ignored), `research`, `research_path {path}` (adopt a doctrine, the
players' name for a research path, for the rest of the game, docs/PORTING.md "Research paths"; refused with the reason once
committed, for a path whose techs are all known, or one closed to the civilization; the browser
asks for confirmation first), `update_civics`, `percent_change`,
`convert`, `declare_war`,
`propose_deal`, `event`, `vote`, `set_option`, `auto_moves`, `end_turn`, `autoplay`, `cast_at {unit,
spell, target, confirmed}` (a spell cast at a distance, docs/CARDS.md: `target` is `{"plot": {"x":
x, "y": y}}`, `{"city": id}`, `{"unit": id}`, `{"player": id}` or `{"choice": n}`, one of the
`targets` its action lists; a world or card spell answers `{"confirm": {"kind": "cast_global"}}`
until resent with `confirmed: true`; refused for a spell without a target, a target of another
kind, a target not listed, or a unit that cannot cast it now; `do_command COMMAND_CAST` refuses a
targeted spell), `place {spell, target}` (a world spell the player places, `place_on_charge`:
Wellspring; `target` one of its world action's `targets`; refused without a charge, for a target
not listed, or for a spell a unit casts), and (see Crusades below) `crusade_answer`, `crusade_lend`, `propose_loan`. Each is
the port of what Civ4 sends for that input; design §4.5 and the "As built" notes of P1 and P3 in
§7 list the rules calls and checks. The `group` query lists the actions of a selection with the
command that performs each, ready to send, except a `target` action's plot: its command carries
the first of its `targets`, or your units' own plot as a placeholder (with `any`, or when no
target is in range), and you put in the plot you pick (`x`, `y` of a `move`, `data1`, `data2` of
a `mission`). Go-to is a `move`; a `mission` `MISSION_MOVE_TO` into a rival's borders asks the
declare-war question too (answer it with `move` and `declare_war`). Their JSON form is the
protocol request (`{"c": "move", "units": [..], "x": .., "y": ..}`), with type names or indices.
Parameters are accepted only in
the form the Civ4 interface sends them: missions without movement `flags` (0) and with the data
their button carries (a plot, a build/religion/specialist/building, else -1), no
`MISSION_MOVE_TO_UNIT`, specialist changes of +1/-1 where the city screen shows the button,
slider steps of ±`COMMERCE_PERCENT_CHANGE_INCREMENTS` (10), `TASK_LIBERATE` with `data1` 0.

### Deals

`propose_deal {"to": 2, "give": [{"item": "TRADE_PEACE_TREATY"}], "get": [{"item":
"TRADE_PEACE_TREATY"}]}` offers a deal to an AI, as the diplomacy screen does. The `trade {with}`
query lists what each side can give (Civ4's trade table: gold and gold per turn up to a maximum,
maps, open borders, pacts, a peace treaty while at war, techs, resources, cities, ...) with
Civ4's refusals. A deal must use those items (gold as an amount), not twice, and put a peace
treaty, open borders, a defensive pact or a permanent alliance on both sides. At war an AI
refuses to talk at all until the war has lasted its leader's refuse-to-talk threshold (5 to 20
turns, doubled for a war it chose, shorter when you are clearly winning; Civ4's
`AI_isWillingToTalk`): `trade` then answers `contact: false`, `why_not: "... refuses to talk"`,
and `propose_deal` `cannot`. As every deal between enemies ends the war (Civ4's `CvDeal`), a deal
at war must carry the peace treaty (both sides) or a capitulation; a gift alone is `invalid`.
The AI answers at once: `accepted: false` changes nothing; `accepted: true` signs it. Today's
AI (ffh-ai's `consider_offer`) accepts gifts, demands from a much stronger neighbour up to a
limit, peace after 10 turns of war unless it is clearly winning, open borders when it likes you,
and a fair tech swap, and refuses anything else. A gift or a demand it accepts is recorded
(ffh-core `tribute`; see the screen above); the `trade` query's `tribute` gives what it recorded
of yours: `gifts {value, relations, tier, max_relations}`, `demands {value, relations, pause,
pause_turns}` and `turns`. `cancel_deal {"deal": 3}` cancels a standing deal of yours (the
`players` query lists your deals with `deal`, `cancelable` and `cancel_in`, the turns left).
Each item of the `trade` tables carries `value`, the points the AI puts on it (positive for what
it gets, negative for what it gives, per piece for gold and gold per turn, null for no price;
`Controller::trade_value`) and, for a lease (resources, gold per turn), `value_turns`, the turns
the value counts. `trade_answer {with, give, get}` answers what `propose_deal` with those items
would get now: `accept`, `margin` (what the AI gets minus what it gives, when value decides)
and `why` (the rule, when one decides; a deal `propose_deal` refuses outright is refused with
its error); `accept` is null for a human.

### Crusades

Not Civ4 (docs/CRUSADE.md): part of every game. A council's war resolution calls its members to lend
units to a crusade's own AI player (its Host) instead of declaring war for them, and civs can
lend units to an ally's war (allied loans). Strengths are combat strength x100 (scaled by health:
a healthy warrior is 200), as the `crusades` query counts them.

* A call waits in `pending.crusade_calls` (`crusade`, `source`: the council, `target` team,
  `obligation`, what you can lend, `deadline`, `end`), counts in the end-turn gate
  (`confirm end_turn` has `crusade_calls`), and is answered with
  `crusade_answer {"crusade": 0, "strength": 600}` (`"strength": null` refuses). No answer by the
  deadline counts as a refusal; at the deadline what you lent is weighed against the obligation
  (nothing: you defy the council; less: part of the defiance anger).
* `crusade_lend {"crusade": 0, "units": [12, 15]}` hands units to the Host where they stand (any
  of your combat land units, not summoned, not aboard or carrying units; called or not). Lending
  is war: while you are at peace with the target it answers `confirm lend_war {team}`; resend
  with `"declare_war": true`. Lent units cannot be recalled; they come home when the crusade ends.
  The view's units of a Host carry `lent_by`, their lender.
* `propose_loan {"to": 2, "lend": false, "target": 1, "mission": "attack", "strength": 600,
  "duration": 10, "give": [{"item": "TRADE_GOLD", "data": 50}], "get": [], "units": []}` offers an
  allied loan: you borrow from (or, with `"lend": true`, lend to) player `to` a strength against
  a team the borrower is at war with, for `duration` turns, to attack it or (`"mission":
  "defend"`) to defend the borrower; `give` / `get` are the rest of the deal, its price (any trade
  items, Deals rules: gold once a team knows its tech; an ongoing item, gold per turn or a
  resource, runs as long as the loan, and cancelling it ends the loan), `units` the units you
  lend (empty: your advisor picks). The AI answers at once: `accepted: true` makes the loan,
  `accepted: false` changes nothing, or `confirm loan_counter {cmd}`: the terms it would take
  (more gold, or gold per turn), as the `propose_loan` to resend (with your units while they
  still cover its strength). The `trade` query's `troops` offers it each way: `lend` and `borrow`
  with the `targets` the loan could be against (or `why_not`), `lend.units` your lendable units,
  `max_turns`; null without contact.
* An AI may offer you a loan (blocking decision `loan`: accept or refuse), and when a crusade you
  fought in ends you pick your cities of the spoils (blocking decision `draft`: one option per
  city, then `stop` to take the rest of your share as gold).
* The `crusades` query lists the running crusades and loan generals (your call, what you
  delivered, your units serving, your credit, whether you may lend, the Host's name, its largest
  stack in view `host_at` and the target's city nearest your capital `target_at`), the units you
  can lend, the known wars, and your past crusades with their spoils; `text` mentions your calls
  and crusades. The view says whether the game has crusades (`crusades`); the `players` query
  marks a Host (`crusade_host`: no contact, no war to declare; it fights its crusade only).
* In the browser (made for the HD-2D map's floating interface, which the classic one shares): a
  call is a popup with the target's leader (pledge, refuse, then the lend dialog or later); the
  advisors' rail has a *Crusades* button in a game with crusades (marked while a call waits):
  each crusade with its Host's banner colour, your part, *Lend units…* and *Show the Host* /
  *Show the target*, which close it and bring the camera there. Troops are an item of the trade
  screen, each side ("Troops: lend your units…", "Troops: borrow theirs…"): adding it opens the
  troops screen (the enemy, the mission, the turns, and the units you send, picked there, or the
  strength you ask for), the rest of the deal its price; the leader answers the proposal as for
  any deal. Units are lent only from these screens, never as an order of the unit itself. The
  rules are in the help display (help.js, the top bar's ?: topics at the left, the article at the
  right; How to play (the turn's loop in a dozen lines, opened by ? and the title screen), Keys,
  the crusade topics, then the game options' armies, governors and cultural
  actions), not in the dialogs: each dialog's (i), in its title bar,
  opens the help at its topic over the dialog, so the body holds only the decision.

### Queries

Requests are `{"q": "<name>", ..}`; replies are JSON with type indices (`data` names them).
Errors: `invalid`, `not_owner`, `gone`, `busy`.

| `q` | Reply |
|---|---|
| `status` | phase (`human`, `running`, `decision`, `over`; over the transports also `stuck`, `no_game`, `crashed` with the report), turn, year, you, the active players, the open decision, the winner, `view_version` (changes whenever the view does: a command, a decision opening or closing, debug reveal), `ai_auto_play` (the AI autoplay's turns left, 0 when off; observed, the turns left to play), `observe` (an observed game: `human` is the player the view follows, phase `human` means stopped at a turn boundary), `build` (the commit the binary was built from), and over the transports `game {dir, journal, autosave}` |
| `pending` | open events (with each option's effects), votes, research needed, idle cities, promotions, ready groups, idle AI-controlled groups, crusade calls |
| `view` | `me` (gold and rate, research, sliders, civics, score, Armageddon counter, the Armageddon deck as you know it (`doom`: null before the deal; else `counter` and `slots`, each `{threshold, state}`, `state` `fired`, `revealed`, `seen` (ahead) or `hidden`, and for a known slot `card` (its key, null for an empty slot), `name`, `help`, `stage`, `stages`), mana, Stasis, the player options, AI autoplay, `world_actions` (the world spell and the cards' world spells: `once`, `cast`, `why`, `caster_where` (`in`/`near` your nearest city, null without one), and for a spell cast at a distance `target_kind` and `targets`; `place_on_charge` for a world spell the player places, with `place` while its charge is held: `label`, `completed`, `pick`, `later`, `hint`, `title`, and a `place` command), `sea_range` (your team's range at sea, null while it has no city: docs/NAVAL.md)), the players you met, the cities you revealed (each `sea_port`: a port of your team; `harbour`: `{state, besieged, text}` or null on no sea), the units you see (yours with `boats`, in boats now, and `wear`, a warship's wear in percent), your groups (`ready`: some unit can move, as `pending.ready_groups`; `selectable`: every unit can, what Civ4's cycle selects; neither for a marching recruit's group), `group_cycle` (your groups in Civ4's cycle order) and `has_auto_unit`, `seas` (named seas you have revealed a plot of: `{id, name, x, y, plots, state, badge, control, ours, theirs, party}`), `sea_lanes` (your supply ships' lanes: `{ship, owner, path, port, cut}`), `pending`, `log_next`; with armies (null without), `armies` (your armies and fleets, each with `domain` `land` or `sea`; a fleet as the `armies` query's `fleets`; each: `id`, `name`, `group`, `units`, `x`, `y`, `in_supply` (its counter is 0), `supply`
(docs/SUPPLY.md: `state` `supplied`, `hunger` or `starvation` from its counter, `turn` and `of`
(Hunger's turn of `SUPPLY_HUNGER_TURNS`), `turns_to_starvation` (Hunger only), `line` (as traced
now, null when cut: `path`, `[x, y]` from the army's plot to the source's, `cost`, `budget`,
`source` (the city's name, or "a Trireme"), `source_kind` `city` or `ship`), `text` (the state in
the Armies screen's words)), `tactic` and `tactic_next` (its battle tactic and the one from your
next turn, `BattleTacticType`), `target_size`, `marching` (the recruits marching to it: unit ids), `requisition` (`cities`: the cities it requisitions from, in order; `orders`: its requisition orders in production, each `{city, name, unit, index, turns}`; `head_orders`: how many other orders bound for it its cities build next (the first order behind any requisition orders, whose destination or the city's default names the army); `shortfall`: how many it lacks of its target size counting its units, recruits marching, `orders` and `head_orders`, 0 or less: none), `disband_to`, and `upgrades` with their `upgrade_price`) and `garrison_marching` (the recruits marching to garrisons: unit ids); `supply_lines` (the lines
of other players' armies you see: an army on a plot you see with a unit you see under its true
owner (hidden nationality hides it), and a line as you know it: `{owner, x, y, segments}`, the army's
plot and its known line (the rules' `plot_supply_line_known` with your team as viewer: over plots you
have revealed, around the units you see; no entry without one), `segments` one run `[[x, y], ...]`
from the army toward its source; nothing else of its supply: not its counter, cost or
source); and each unit's `army`, `garrison`, `army_unit` and `march` (null, or a marching recruit's `{dest, to, from, turns_left, stalled, diverted, since}`: where it goes and its name, the city that trained it, the turns it needs along the rules' path (null: no way there now), the turns without progress, whether it was sent to the nearest garrison instead, the turn it set out); `battles` (your battles of this game turn and the last, as `battle_reports`' summaries: the map's markers); each land unit its `engage_row_order` (its rank preference: the rows, front 0, middle 1, rear 2, in the order the auto formation tries them), `engage_attack` (its A before the bonuses of a plot it attacks, x100), `engage_defense` (its Dv where it stands, x100), `engage_max_hp` (its HP in a battle, Reinforced included; `hp` and `max_hp` stay the health percent), `reinforced` (on a unit with stacks of Reinforced, Ritual: Upgrade's: the line `Reinforced x2: +20% hit points.`, shown in the plot tooltip), `engage_speed` (before first strikes), `engage_actions` (`[{kind, n}]` in its front, middle and rear rows, `kind` an action kind: `Strike`, `Charge`, `Stab`, `Shoot`, `Spell`, `Bombard`, `Heal` or `None`), `engage_dodge`, `engage_crit_bonus` and `engage_large` (it fills two cells of a row and counts two slots); with cultural actions, each of your cities' `channel` (`{action, target, religion}`, absent when it channels nothing), and `revolt` (the level: 0 pressure without effect yet, 1 unrest, 2 revolt, 3 close to changing hands) and `unrest` (its unhappiness) for the cities you see under revolt pressure |
| `map` | flat arrays `y * w + x`: `vis` (0 unrevealed, 1 fogged, 2 visible), `plot_type`, `terrain`, `feature`, `bonus`, `improvement`, `route`, `owner`, `river` (bit 0 river on the south edge, bit 1 on the east edge, bits 2-4 north/south flow + 1, bits 5-7 west/east flow + 1), `city`, `yield` (`food \| production << 8 \| commerce << 16`, visible plots only), `sea` (docs/NAVAL.md section 10: on revealed water, the steps from your team's nearest port while in range, 0 out of range; -1 on land, unrevealed, or while your team has no city and so no range rule), `sea_id` (phase 2: the plot's named sea on revealed water, else -1), `sea_ice` (1 on Sea Ice, the Illians' Frozen Passage, else 0), `map_rev`, and `epoch` and `rev`. `map {since: R, epoch: E}` answers only the tiles changed since revision `R` (`full: false`, `tiles` = their indices, each array the values of those tiles), or the full map (`full: true`) when `E` is not the current epoch, `R` is unknown or much of the map changed |
| `plot {x,y,units?}` | a plot as you know it (`min_level` when units below a level cannot enter); while visible also yields, defence, culture and the units (under debug reveal each with `ai_type`, the AI's role, and `missions`, its group's orders). On a lair or goody hut, `explore`: the odds for the first of `units` (the selection) that is yours (`view/explore.rs`, below "Lair and goody hut odds") |
| `group {units}` | the units in detail and the actions: `mission`, `target` (an interface mode, `mode`: go-to, route-to for army units, range attack, the air modes; needs a plot), `command`, `load`, `spell` (a spell cast at a distance: `target_kind`, `targets` `[{target, at, label, info?}]` (`at` is also set on a choice that is a plot's option) and a `cast_at` command; the Discover action of a great person holding a card: `label` "Discover: <card>", `card`, and `why` while it cannot be cast), `promotion`, `upgrade`, `army` (this game's armies: `army` the action, `label`, `help`), each `enabled` or not, each with its `cmd`; a great person's unit carries `card_detail` (its card's `tech`, `suit`, `band`, `alignment`, `religion`, `needs` and `needs_one_of` (`{tech, known}`), `known`, `discoverable`, `why`), and every unit of yours in the view its `card`; a selection with a marching recruit has none and says why in `marching` ("marching to the 2nd Army"); each unit's `sea` (docs/NAVAL.md section 10; null for a land unit on land): `boats`, `range` (its plot's steps from your nearest port), `of` (your team's range, null without a range rule), `out`, `wear`, `turns` (its turns at sea, warships), `free_turns` and `lines` (the panel's words). A unit in boats is offered only skip, sleep, sentry, go-to and the commands the rules leave it |
| `path {units,x,y}` | the path preview: nodes with the turn of arrival and moves left, total turns, a war it would declare, whether it ends in an attack; `supply` (docs/SUPPLY.md): with the units of one army of yours selected and a plot you have revealed, whether the army would be supplied there, `{ok, cost, budget, sea_port}` (`cost` null without a line; `sea_port`: at sea, the port it would be in range of, else null; the rules' `plot_supply_line_known` with your team as viewer: what you see, not the truth), else null; `sea` (docs/NAVAL.md section 10), null when the path neither goes in boats nor out of range: `boats_from` (the first node index where a selected land unit would be in boats: water where no ship of its team has room for it; 0 when it is in boats now), `boats_moves` (water plots a turn), `boats_turns` (the turns from the first water node's turn of arrival to the last's), `out_from` (the first revealed water node out of your range for a selection the range rule applies to there), `attrition`, `strength` and `lines` (the preview's words; phase 2 adds the seas entered where an enemy you see patrols, and for an attack on a coastal city its harbour; `sea` is then not null); `supply.sea_why` (no line: why a supply ship of yours there is no source) |
| `sea {id, units}` | a named sea's panel (docs/NAVAL.md 10; a sea you have revealed a plot of, else `gone`): `name`, `x`, `y`, `plots_text`, `state`, `badge`, `control`, `ours`, `theirs`, `hold`, `sides` (`{label, own, strength, ships, note, party}`), `now`, `if_they` / `if_you` (`{title, lines}` or null), `send` (with warships of yours among `units`: `{label, here, turns, x, y, result, units, enabled, why}`; else null) |
| `place_menu {spell, x, y}` | a placed world spell's menu on a plot (`view/place.rs`; refused for a plot that is not one of its targets now): `title`, `x`, `y`, `where`, `rows` (one per mana type: `target`, `bonus`, `name`, `owned` ("You own: N."), `passive` (lines; empty: none), `promotions` (tier I to III: `promotion`, `type`, `name`, `tier`, `needs`, `free`, `spells` `[{spell, name, help}]`)) |
| `seas` | the Seas screen: `{seas: [{id, name, x, y, state, badge, control, ours, theirs, enemy, ports, lanes, lanes_cut}]}` |
| `combat {units,x,y}` | the attack preview: attacker, defender, odds of winning and of retreating (per mille), strengths, the modifiers of both sides. Under engagement combat, for land units attacking a land plot that holds a defender: `engagement: true`, `defenders` (their number), `declare_war_team` and `on_arrival`. An attack the selection can launch now (the plot is next to it and some unit on its plot can attack the plot now): `on_arrival: false`, `valid: true`, `pool` the units on its plot that can (other armies and the garrison there included). Otherwise (a go-to that ends in an attack, units that have used their moves or their attack): `on_arrival: true`, the duel's `valid` and odds kept, `pool` the selected units that could fight there once they can attack it. The duel's odds and strengths of the strongest pair stay beside them (the forecast of a battle is `engage_preview`'s). Ships, air units and the option off: the duel's preview. The sea (docs/NAVAL.md): each side's details carry `sea_wear` (a warship's wear, negative percent), and `defender_in_boats` with `embarked_strength` says the defender fights in boats |
| `engage_preview {army \| units \| from, to, formation?}` | engagement combat: the plan of an attack (the battle screens' plan), in the battle log's shapes (`assets/app/battle_log.js`'s header lists what the screens read): `plot` and `from`, the `ground` and its rules, `params` (the rule values the screens word), both sides' `slots` and why and their `lanes`, `sides`, `fighters` (the pool, then the defenders you see, each in its cell or `lane` null: its A and Dv, HP, speed, actions per row, spell, dodge, crit bonus, size and specials), `formation` (the attacker's line: its tactic's formation; the query still takes a `formation` to forecast, which the client no longer sends and `engage` refuses), `auto_formation`, `reserve` (the pool's units not in it), `def_formation` and `def_reserve` (the defenders' line in their tactic's formation, as the real fight forms it, and the rest on the plot), `tactics` (each side's battle tactic, attacker and defender, `BattleTacticType`), `starving` (`[{b, pct}]`: the fighters you see of starving armies and the percent change of the damage they deal), `forecast` (`runs` battles of the real resolver on a private RNG seeded from the plan, never the game's: `lost_x100` and `escaped_x100` per side, `line_wiped_permille`, `plot_falls_permille`, `attackers_wiped_permille`, and `per` fighter its `fall_pct` and `escape_pct`), `declare_war` (the team the attack declares war on), `errors`. A formation the rules refuse gets the plan in the auto formation, `forecast` null and why in `errors`; a plot you do not see, one not next to the attackers, or an attack the rules refuse has no plan: the reply is `errors` and `declare_war` alone. Refused without the option |
| `battle_log {report}` | engagement combat: a battle's report with its full log as the battle screens replay it (`report` is the report's id: `id` is the envelope's): `battle_log.js`'s header lists every member (among them `tactics`, each side's battle tactic, and a starving side's `Starvation` event before the battle). Only for a battle you fought on a side (or under debug reveal), while its log is kept (the current and previous turn's battles and the last `ENGAGEMENT_REPORT_LOGS_KEPT`): else `gone`. Your side shows its real owners, the other side the owners you saw (hidden nationality). `events` are what the battle shows (the opening at round 0: units that sat out afraid, defensive strikes; then every action, one unit at a time, with its hits, heals, knock-backs and escapes), `after` what it did after (XP, heals, kill effects, flight, the move in), `outcome` whether the plot fell and each side's line by fate |
| `battle_reports` | engagement combat: `{reports}`, your kept battle reports as summaries, newest first (`battle_log.js`'s header lists a summary: where, when, the sides with what each committed and lost, the outcome, `has_log`) |
| `city {city}` | the city screen of one of your cities: plots (with `can_work`), specialists (with `plus` and `minus`: what `do_task TASK_CHANGE_SPECIALIST` accepts now), yields, `commerces` (each with its `rate`, the empire's `percent` and its parts: `rates`, `specialists`, `buildings`, `religions`, `empire`, `process`, `modifier`), `next_worker` (who a click on a plot the city does not work takes: `{idle}`, `{specialist}` or `{plot}`; null for nobody), happiness, health, buildings, hurry, emphasis, queue, the buildable list with cost and turns; `cultural` (null without cultural actions; pressures and amounts in hundredths of culture, chances in hundredths of a percent): `rate`, `amount` (what a channel takes a turn), `diverted` (what it took this turn), `channel` (null, or `{city, caster, action, target, religion, since_turn, hostile, amount, reach, unseen, chance}`), `options` (every channel it could set, one per action, civilization with a city in reach and religion of the city: `ok` or `why`, `hostile`, `reach`, `unseen` (cities in reach you have not seen), a spread's `chance`), each `reach` entry `{city, name, owner, distance, effect}` (the distance effect, percent) and `amount` (pressure or chance a turn), for Foment Revolt with `resist` (the city's resistance a turn now), `resist_revolt` (its defenders' part: all it resists in revolt, making no culture), `rest` (the other casters' pressure there), `in_revolt` (in revolt now: its full resistance cannot be read, so `resist_revolt` holds both ways), `settles` (where the caster's pressure settles from now under this city's channel alone, with the city's resistance falling to `resist_revolt` from the turn after the total reaches the revolt: docs/CULTURE.md, "Predictions"), `now` (the caster's pressure there), `others` (what the caster's other cities add a turn), `capital`, for Spread Religion with `has` and `religions`; `pressure` (on this city: `total`, `casters` `[{caster, pressure, inflow, settles, rest}]` (`rest`: the others' pressure), `unknown`, `resist`, `resist_revolt`, `in_revolt`, `defenders`, `level`, `unhappiness`, `capital`); `thresholds` (`unhappy_step`, `unhappy_max`, `disorder`, `near_flip`, `flip`, `fade_percent`, `resist_percent`, `garrison`, `channel_percent`, `full_range`, `range`, `spread_per_culture`, `spread_max`, `flipping`); `recruit` (null without armies): the city's default destination, the destinations it may choose, its garrison, `garrison_tactic` and `garrison_tactic_next` (its garrison's battle tactic and the one from your next turn, docs/TACTICS.md), the armies that requisition from it (`requisitioned_by`: `{army, name, target_size, shortfall}`); with armies, each queue entry of a land combat unit has `dest`, `dest_name` and `requisition` (an army's requisition order: first in the queue, not removable by hand) |
| `governors` | the Governors screen: `government` (the government civic), `direct_rule`, `min_suitability`, `max_priorities`, `priority_weight`, `priorities`, `diagnoses` (`{kind: role_unfilled, role}`, `{kind: poor_fit, city, name, role, suitability}`, `{kind: slow_order, city, name, turns, order, type}`), each city (capital first: `governed`, `governor` `{id, name, trait, city, since, born}`, `role`, `role_overridden`, `proposal`, `suitability`, `production`, `population`, `requisitioned_by`; a governed one its `card`: `roles` (each `{role, suitability, allowed, why}`), `key_buildings` (the role's, each `{class, building, tier, effect, none}`), `plan` (the governor's next buildings, `ffh_ai::governor::plan`), `orders` (its requisition and empire orders, each `{index, order, type, turns, empire, requisition, dest_name}`), `governor_order`); the city query's `governance` is the city's entry without the card, with `government` |
| `empire_items {army?}` | an empire order's first step: `governed` (your governed cities), `items` (every unit, wonder, palace and project one of your cities lists: `{order, type, cities}`, `cities` the governed cities that can build it now, 0 unavailable; a unit's `army_unit`); with `army`, the units that serve in armies only |
| `empire_cities {order, data, army?}` | the second step: each governed city able to build it, `{city, name, x, y, turns, own_turns, ahead, ahead_turns, displaces, distance, cost_percent?, tier?, suggested}` (`turns` behind the `ahead` orders; `displaces` the governor's order it goes ahead of, delayed `own_turns`; `distance` in plots to the army; one `suggested`: with an army the nearest, ties to the fewest turns, without the fewest turns), and `dest` (the army, as `empire_order` takes it) |
| `armies` | the Armies screen: the `armies` (as the view's, with `recruits`: the recruits marching to it in full, each its `march` with `id`, `unit`, `x`, `y`; and the requisition's `candidates`: every city of yours with `picked`, `governed`, the `unit` it would train for the army, its `production_turns` and `march_turns` (null: no way by land), its `order` in progress; `open`: whether the requisition may change; `on_order`: its empire orders in your cities, each `{city, name, unit, index, turns}`), each city's garrison (`units`, `governed`) with the recruits marching to it (`recruits`), `supply_budget` (`SUPPLY_LINE_BUDGET`), `stall_turns`; the Fleets section (docs/NAVAL.md, section 7): `fleets` (as `armies`, with `domain` `sea`, `supply`, `tactic` and `tactic_next` null, `station` (`kind` `none`, `patrol` with `sea`, or `escort` with `army` or `group`; `text`, the order in bare words), `wear` (its ships' highest, in percent) and `wear_text`, `sea` (the name of the sea it is in), its requisition's candidates the coastal cities only), `seas` (the seas a fleet may patrol: `{id, name}`), `escort_targets` (your armies and fleets: `{army, name, fleet}`) |
| `culture` | the Cultural Pressure panel: the channels you see (each `{city, city_name, x, y, caster, action, religion, target, since_turn, hostile}`): `ours` and `aimed` (at your team) also with `amount`, `reach`, `unseen` and a spread's `chance` as in the city query (for `aimed` the reach is your cities), `others`; `pressure`: your cities under revolt pressure (the city query's `pressure` with `city` and `name`); `memories`: what met leaders remember of your cultural actions (`{player, hostility, friendship}`, at most `memory_max`); `thresholds` as in the city query |
| `techs`, `civics`, `religions`, `players` | the tech tree (prerequisites, depth, turns; each tech's `help`, `research_mods` (its tech-specific research multipliers in percent: `others`, known by civilizations met, and `prereqs`, alternative prerequisites known, added to 100; `feature`, a held unique feature, and `path`, the research path, each multiplying; `total`; the research chooser and the tech tree show them) and, unless you have it, `unlocks`: the units, buildings, builds, civics and religions of its help as type indices, which the research chooser shows as icons; `path`, `path_percent` and `paths`: each research path's techs, those known and closed, the research left, `complete`, and `can_commit` or the `refusal`), civics (availability, upkeep, anarchy; `civics {with: [civic, ..]}` adds `proposed`: the set a revolution to those civics makes, whether `update_civics` would be applied, its anarchy and upkeep, and `rule_change`: `governed`, `direct` or null, whether the change hands every city to governors or takes every city back), religions, the players you met with your deals |
| `trade {with}` | the trade tables both ways, with the AI's values (see Deals) |
| `trade_answer {with, give, get}` | what `propose_deal` with these items would get now (see Deals) |
| `speech {with,comment?,salt?}` | what the AI leader of `with` says to you: `{with, comment, text}`; `comment` an `AI_DIPLOCOMMENT_*` (`DiploCommentType`), or none for what it says when you contact it (its greeting, Civ4's `AI_getGreeting`, or `AI_DIPLOCOMMENT_REFUSE_TO_TALK`); `salt` picks among its lines (Civ4 uses the interface's random numbers); `text` null when it has no line |
| `moments {since}` | the leader moments from index `since` (`view.moments_next` is the next one): `{i, t, player, comment, text}`, an AI leader opening the diplomacy screen with you (first contact, a war declared on you) and what it said; the last 20 are kept in a save |
| `log {since}` | your messages from index `since` (`view.log_next` is the next one): `{i, t, text, player, at?}`, `at` the plot `[x, y]` a combat or a city captured or razed happened on; observed, every player's messages. A battle's message (engagement combat) also has `kind: "engagement"` and `report` (its report's id), so a client links it without reading its words; so do the messages of a command's reply and of the turn report |
| `report` | the turn report: what changed between your end of turn and now, or the end of the game (units lost and gained, cities founded, lost, grown, starving, buildings, techs, civilizations met, wars and peace between those you had met, enemies near your cities, the Armageddon counter, borders; under engagement combat `engagements`, the battles you fought since your last turn began, as summaries, newest first, each also a line "battle #12 at Prespur: held; 2 lost, 5 of theirs fell"), with the messages |
| `text` | a plain-text summary for agents |
| `ascii {x,y,w,h,layer}` | an ASCII map, 2 characters per plot, north up, `::` unrevealed; layers `terrain`, `political`, `yield`, `units`; a legend |
| `inspect {kind,oid}` | `{:#?}` of your unit, group, city, player, team, deal, or of an info entry (`UnitType`, `units`, ...) with id or index `oid`; any object, and plots, under debug reveal (a plot holds every team's data: use `plot`) |
| `data` | the static tables (every kind: type, name, texts, the main gameplay fields; missions, commands and builds with their Civ4 hotkeys `keys` and button order `order`), the interface modes' hotkeys, a few global defines (among them the sea's: `sea_range`, `sea_attrition_percent`, `embarked_moves`, `embarked_strength`, `sea_wear_free_turns`, `sea_wear_percent`, `sea_wear_max_percent`) |
| `setup`, `saves` | the new-game lists; the saves of the game's directory |
| `crusades` | crusades and loans (see Crusades): `enabled`, `wars`, `rules`, `military_strength`, `lendable`, `crusades`, `reports` (the `trade` query's `troops` offers loans) |

What you see is what Civ4 shows the player: your units, and others only on plots you see and
when they are not invisible to you (hidden-nationality units show as barbarians; the cargo of a
foreign ship is hidden); cities you revealed (size only while you see them); players you met;
bonuses your techs reveal; improvements, roads and borders as you last saw them.

### Lair and goody hut odds

The plot query's `explore` (and the Explore Lair action's, in the `group` query) gives what
entering a goody hut or exploring a lair can bring, with each chance for the unit, in parts per
million. The map tooltip shows the short form, and the plot panel and the Explore Lair action
show the full table (`assets/app/explore.js`). The lists are the ones the rules draw from
(`ffh_core::ffh::lairs`: each tier's outcome list is built by one read-only function that the
roll and the odds share). The odds treat the RNG as uniform.

- **Lair** (`kind: "lair"`): `epic` (Explore Lair on an epic site: two tiers), `unit`,
  `unit_type`, `level` (the roll is `rand(100) + level`), `tiers` (`[{tier, ppm}]`, tiers
  `big_bad`, `bad`, `neutral`, `good`, `big_good`), and, while the plot is in view (`in_view`),
  `outcomes` (`[{tier, key, label, ppm, destroy_pct, replaces_lair}]`), `lair_survives_ppm`,
  `grace_ppm` (the chance that a grace check passes: early on, the big tiers leave out their
  strongest entries), `big_bad` (the monsters, henchmen and promotions the big bad result picks
  from, by name) and `others` (the other selected units' tiers: every selected unit explores in
  turn). Without a unit: the tiers of a level-1 unit.
- **Goody hut** (`kind: "hut"`): `unit` (the unit that enters first: the odds depend on its
  damage, promotions and experience), `outcomes` (`[{goody, label, ppm, slots, allowed}]`, the
  difficulty's list with each goody's share of its `slots_total` slots and whether the unit can
  receive it now), `attempts` (draws before the hut gives nothing) and `nothing_ppm`.

### Decisions and popups

Civ4 answers some rules questions with popups for a human. `ffh-play` keeps the C++ timing:

* **Deferred** (answered at your next control point, like an unanswered C++ popup): choosing
  production for an idle city, events, diplomatic votes, the research choice. They are listed by
  `pending()` and answered with `push_order`, `event`, `vote`, `research`. An event popup is
  checked when it is shown (it is dropped if none of its choices is possible then); once shown
  it stays until you answer it, even while your own actions make its choices impossible for a
  while (C++ event popups are modal). Ending the turn with
  any open needs `force`; then open events are dropped, votes stay (they abstain after Civ4's
  extra turn) and empty research takes Civ4's automatic pick after turn 4.
* **Blocking** (answered during the rules call that asks, through the session's `Answerer`):
  a free tech, keeping or abandoning an acquired city, the secretary general's election, a deal
  offered to you, founding a religion. Each offers the C++ popup's options and the advisor's pick.
  An answerer that implements `ask_with` can run the queries meanwhile (through the
  `DecisionReader`; messages, the report and the saves list are not available then).

The advisor is a private `ffh_ai::Ai`; with `autoplay` it plays your slot for N turns.

### Saves, autosaves, journal

With `SessionOptions::saves_dir = DIR`: autosaves at every turn boundary in `DIR/auto/` (the
last 20, `T0057.ffhsave` = the end of turn 57) and `DIR/auto/start.ffhsave` at a new game's
first control point, the journal `DIR/journal.jsonl` (one line per command, decision answer and turn
checkpoint), manual saves `DIR/<name>.ffhsave`. A save is two files: `.ffhsave` (the game, any
tool loads it) and `.ffhsess` (session state: your open event popups, the journal position, the
latest messages and the turn report).

## Transports: `ffh-play agent` and `ffh-play serve`

Both speak the same envelope (design §4.1). A request is a JSON object with `k` and an optional
`id` (an integer or a string) that the reply echoes:

```
{"id":1,"k":"q","q":"view"}                                     a query (the table above)
{"id":2,"k":"cmd","c":"move","units":[881],"x":15,"y":33}       a command
{"id":3,"k":"answer","decision":17,"option":1}                  or {"advisor":true}
{"id":4,"k":"session","op":"save","name":"before-war"}          a session operation
```

Replies are `{"id":1,"ok":true,"r":{..}}` (a command's adds `log`, the messages it produced, and
`status`; a command Civ4 would confirm answers `{"confirm":{"kind":"declare_war",..}}`, resend
it with the flag), or `{"id":1,"ok":false,"error":"<code>","detail":".."}` with the codes
`invalid`, `not_your_turn`, `decision_pending`, `not_owner`, `gone`, `cannot`, `busy`,
`no_game`, and over HTTP `crashed`.

Session operations (`op`):

| `op` | |
|---|---|
| `new {setup, name?}` | a new game (`setup` as `--setup`; `name`: its directory, default `game-<seed>`; not `auto`, which names autosaves) |
| `war_start {setup, name?}` | the debug start at war (see "Start at war" above; `name` default `war-<seed>`): answered once the search found a start, `{game, war: {player, turn, battle, confirmed, rejected, attacker_name, player_name}}` (`battle`: the AI game's, `confirmed`: the check's, each `{turn, attacker, defender, x, y, attacker_stack, defender_stack}`), or `cannot` when no battle came in 500 turns; meanwhile the published status carries `war_search: {turn}` |
| `load {name}` | a save by name: `NAME`, `auto/NAME` or `flags/NAME` in the current game's directory, `GAME/NAME`, `GAME/auto/NAME` or `GAME/flags/NAME` in another game's (as the `saves` query and operation list them); nothing outside the saves directory. An AI-only save is observed |
| `take_over {player, name}` | take over slot `player` of an AI-only save (debugging) |
| `save {name}` | save the game in its directory (`[A-Za-z0-9_.-]`, 1-64, not starting with a dot) |
| `debug {reveal}` | debug reveal on or off |
| `follow {player}` | an observed game: the view follows `player` (the game is not changed) |
| `flag {x, y, note?}` | flag a plot while the game is stopped: the record (above, "Flags"), appended to the game's `flags.jsonl`, its replay's start save pinned in `flags/` |
| `flags` | the current game's flags |
| `saves` | every game directory with its saves (each `{name, path, auto, sidecar, modified}`; `modified`: when the file was written, seconds since 1970, or null) |
| `quit` | end the agent / stop the server |

**Blocking decisions** (keep or abandon a captured city, a free tech, an election, ...) are
asked in the middle of a command or of the AI turns. While one is open, queries are answered
(from your side; the message log, report and saves list are `busy`), commands and session
operations get `decision_pending`, and `answer` with the decision's id and an enabled option (or
`advisor`) closes it; then the command that raised it gets its reply.

**The agent** prints events as well as replies, one per line:

* `{"ev":"control","turn","year","human","pending","log_next"}`: you have control (at the start,
  after your end of turn, after a load);
* `{"ev":"decision","decision":{id, kind, title, text, options, advisor, context}}`: the next
  lines you send go to the decision;
* `over` (the game ended), `stuck` (a turn did not finish: load an autosave), `no_game`,
  `progress` (with `--progress`, during the AI turns), `answered` (the end of input gave a
  decision to the advisor), `error` (the start failed, exit code 2), `crashed` (a panic, with the
  message, the last autosave, the journal and the `ffh-play replay` command; exit code 3).

A reply that hands control back (`end_turn`, `auto_moves`, `autoplay`, `new`, `load`, or any
command after which nothing is ready while some group has orders to continue: the last ready
unit skipped, fortified or sent on a long move) says `running` and comes first, then the game
runs (the AI turns, or your groups' orders, as Civ4 continues them as soon as nothing is left to
move), then the event. At the end of your input an open decision takes the
advisor's pick, then the agent quits, or with `--on-eof end-turn` ends the turn and prints the
next `control` first.

**The server** answers:

| Request | |
|---|---|
| `GET /`, `/app/*` | the browser client (`index.html`, `app/*.js` modules, `app/style.css`; embedded in the binary, or from `--assets DIR`) |
| `GET /api/status` | the status: `version`, `phase` (`human`, `running`, `decision`, `over`, `stuck`, `no_game`, `crashed`), `turn`, `year`, `human`, `active_players`, `view_version`, `decision`, `over`, `reveal`, `ai_auto_play`, `ms_in_phase`, `map_rev`, `game {dir, journal, autosave}`, `crash` |
| `GET /api/wait?since=V&timeout_ms=T` | the status as soon as its `version` is above `V` (long-poll, at most 60 s) |
| `GET /api/view`, `/api/map` | the `view` and `map` of the last control point or decision (409 without a game); `/api/map?since=R&epoch=E` the tiles changed since (as the `map` query) |
| `GET /api/data`, `/api/setup` | the static tables (with an ETag) and the new-game lists |
| `POST /api` | the envelope; needs the header `X-FFH: 1` |

The server listens on 127.0.0.1 only (unless `--bind`), refuses requests whose `Host` is not
`127.0.0.1`, `localhost`, `[::1]` or the loopback address it is bound to, and refuses a `POST`
without `X-FFH: 1`, so other web pages cannot drive it. A request must arrive whole within 15 s
(408); at most 64 connections are served at once (more get 503). A command's request waits while a decision it raised is open: answer the
decision on another connection (the status shows `phase: decision`). While the game runs (the
AI turns), queries are answered, session operations get `busy` and commands `not_your_turn`,
except `autoplay {turns: 0}` while an AI autoplay runs: it answers `{"stopping": true,
"turns_left": N, "turn": T}` (`T` the game turn the stop was taken at) and the autoplay stops before the next turn slice (FfH `setAIAutoPlay(0)`,
journaled at that slice; your slot is human again at once, and you have control at your next
turn, or at once during yours). The agent reads no input while the game runs, so there an
autoplay runs all its turns. If the rules crash, the
status becomes `crashed` with the report and the server stays up: load the autosave. Report the
bug with the journal and that autosave (`ffh-play replay JOURNAL --from AUTOSAVE`; before a loaded
game's first autosave, the report names the save it was loaded from).

## Protocol conventions

These hold for every request and reply of the protocol (design §4.1):

* **Type references** are accepted as the XML/C++ type name (`"UNIT_WARRIOR"`, `"ORDER_TRAIN"`,
  `"MISSION_FORTIFY"`) or as the integer index. Replies use indices for every type-valued field,
  hard-coded C++ enums included (an `activity` is `2`, not `"ACTIVITY_SLEEP"`); `ffh-play names`
  (later `/api/data`) maps them back. Where a type is optional, `null` or `-1` means none. A name
  of the wrong kind is refused (`unknown UnitType "TECH_X"`).
* **Kinds** with names: all 61 info tables, the 6 GlobalTypes lists without a table, and 15
  hard-coded C++ enums (activities, orders, tasks, player options, tradeable items, ...).
* **Vote choices** are what the Civ4 vote popup offers: `YES`, `NO`, `NEVER`, `ABSTAIN` (also
  `PLAYER_VOTE_YES`, ..., or the values -2, -3, -5, -4), or, in a team vote (secretary general,
  victory), the **team id** voted for (0-34). `NO_PLAYER_VOTE` and `NO_PLAYER_VOTE_CHECKED`
  (-1, -6) are vote state, not choices, and are refused. The `vote` command also refuses a
  button the popup would not show for that vote: team ids in a yes/no vote, YES/NO/NEVER in a
  team vote, a team that is not eligible, NEVER when you cannot defy the resolution, and, for a
  vassal, anything but its master or its own team.
* **Ids** (units, cities, groups, triggered events, votes) are plain unsigned integers; players
  and teams are 0-255. JSON integers are 64-bit signed; a 64-bit hash is sent as a hex string.
* **JSON** is strict: no comments or trailing commas, no duplicate keys, nesting at most 128.
  Errors name the field: ``field `units`: expected an array of (a unit id (...)), got "x"``.
  A member that a command, a `setup`, a session operation or an answer does not have is refused
  (``field `oponents`: unknown field (known: ...)``), so a typo cannot start a default game or drop
  a flag; queries ignore members they do not use.
* **Display names** are the whole description text, with Civ4 markup such as `[ICON_TRADE]` kept
  (`VOTE_LIBERTY` -> "Global Civic: Liberty"). Text FfH2 inherits from BtS is not vendored, so
  those names are derived from the type name (`UNIT_WARRIOR` -> "Warrior"), as are descriptions
  with an unfilled placeholder (`MEMORY_SOMNIUM_POSITIVE` -> "Somnium Positive").

## Testing the browser client

Automated (none of these is part of `cargo test`; `tests/web.rs` there checks that every client
file is embedded and every module import resolves):

```
tools/web-check.sh                       # node --check of every module, node --test of the pure modules
tools/web-check.sh --smoke --browser     # also the smoke tests below
node tools/web-api-smoke.mjs             # 20 turns through the client's api.js against `ffh-play serve`
node tools/web-smoke.mjs                 # headless Chromium plays through the page; screenshots in target/web-smoke
node tools/web-touch-smoke.mjs           # the same on an emulated phone, with touch only; target/web-touch-smoke
node tools/web-pad-smoke.mjs             # an emulated phone with an emulated controller; target/web-pad-smoke
node tools/web-orient-smoke.mjs          # an emulated phone turned and locked; target/web-orient-smoke
```

* `web-check.sh` needs node (it is skipped without it); the pure modules (map maths and map
  deltas, markup, state and Civ4's group cycle, the hotkeys, the tech tree's layout, the API layer
  against a fake `fetch`, the battle screens' log folding, log lines, a formation's cells, the
  tactics' names, a starving side's line and the ground's words over the battle fixture, the watch screen's snap: each action's timing plan,
  the shade, the roles and their poses) are tested in `crates/ffh-play/assets/test/*.test.mjs`.
* `web-api-smoke.mjs [--turns 20] [--seed 5] [--world WORLDSIZE_TINY] [--opponents 2] [--url URL]
  [--keep] [--verbose]` starts `target/release/ffh-play serve --port 0` (or `--bin`), checks the
  page and its modules are served, plays the turns as the page does (popups, research,
  production, found, builds, moves along path previews, the end-turn gate), checks the cached
  view and map against the status, saves, lists, loads, plays a staged capture whose decision is
  answered while the move's request waits, answers a staged transport picker (`stage load`) as
  the page does, and quits. It also follows the map by tile deltas (`/api/map?since=&epoch=`)
  and compares it with the full map at every check, and at turn 8 and at the end uses the
  screens' requests (a specialist's `+` and `−`, a civics proposal and its revolution, the
  research slider down and back, the trade tables, the inspector) and lets the AI play one
  turn (`autoplay`). A refusal of anything the server offered fails it.
* `web-touch-smoke.mjs [--seed 3] [--out DIR] [--assets DIR] [--headed]` plays on an emulated
  phone (Playwright's Chromium, touch only, a coarse pointer; skipped without the `playwright`
  package), with the isometric map (the look phones must support) upright (390x844) and on its
  side (844x390), each against its own `ffh-play serve --new`: the page fits (no sideways scroll,
  End turn on screen, the map's clear part at least 30% of the height, the report folded), the ☰
  menu (every button on screen; a tap on the map only closes it), a tap selects a unit, a first
  tap on a free plot previews the move ("tap again to go") without moving and a second tap
  moves, a two-finger pinch zooms in and out, the settler's Found button, the research popup's
  tap modes, "Add to queue" in the production popup (the queue grows, the popup stays), the city
  screen from the Cities list (the whole-screen dialog with its plot grid, within the screen),
  the tech tree from the menu, a tab opens the report, and End turn through the gate to the next
  turn. Any console error, page error, failed request or HTTP error fails it.
* `web-pad-smoke.mjs [--seed 3] [--out DIR] [--assets DIR] [--headed]` drives the page with an
  emulated standard gamepad (the page's `navigator.getGamepads` answers a pad the test presses),
  against one `ffh-play serve --new`. On a computer (a fine pointer) the pad changes nothing. On
  an emulated phone (touch, a coarse pointer, the isometric map, on its side): the first press
  starts the controller's mode and does nothing else, RB selects a unit and the cursor goes to it,
  the D-pad walks the cursor to a free plot ("A to go" in its tooltip) and A moves the unit there,
  Y gives the focus to the interface and B gives it back to the map, Back opens the menu with the
  focus on its buttons, the D-pad moves it and B closes the menu, Start ends the turn (the end-turn
  check's dialog takes the focus; the D-pad reaches "End turn anyway" and A answers it), and a tap
  ends the mode. Any console error, page error, failed request or HTTP error fails it.
* `web-orient-smoke.mjs [--seed 3] [--out DIR] [--assets DIR] [--headed]`, against one
  `ffh-play serve --new`: on a computer a stored lock changes nothing and Options does not offer
  it; on an emulated phone the layout follows it upright, on its side and upright again (the card
  across the screen, then a column at its right; the map's shape; End turn on screen), then Options
  locks it to landscape while it is upright (headless Chromium does not turn its emulated screen
  for a lock, so the page turns itself, as on an iPhone): the side layout in the turned page, a tap
  where a unit is drawn selects it, taps on Close and End turn reach them, and on its side the turn
  ends; locked to portrait while on its side, the upright layout turned; the lock outlasts a
  reload; "Turns with the phone" ends the turn and its styles. Any console error, page error,
  failed request or HTTP error fails it.
* `web-smoke.mjs [--turns 4] [--seed 5] [--out DIR] [--assets DIR] [--cdp] [--headed]` drives the
  page with the mouse and keyboard in headless Chromium (the `playwright` package if it resolves,
  else Chromium over the DevTools protocol; skipped without a browser): new game (Start
  double-clicked: one game), `Tab` over every group the cycle selects, `D` and `A` held (the map scrolls, the selection stays), hotkeys pressed in the same tick as a
  selection change, found a city with `B`, the research and production popups, the city queue
  (Shift-click appends, ✕ removes), select a unit, the path preview, a right-click move, `Enter`
  while groups wait (it selects one and does not end the turn), end turns with the button through
  the gate, the turn report, save and load through the dialogs, the odds tooltip and an
  attack (`stage attack`), the keep/raze dialog (`stage capture`), popups set aside and asked
  again after loading the same turn, the transport picker (`stage load`), Shift-right-click
  waypoints and go-to (`G`) Shift-click waypoints held until Shift is released (`stage
  automoves`), engagement combat (`stage engagement`: the army's role, A, Dv and HP in the unit
  panel, the battle in the tooltip, a right-click opening the battle plan with both sides'
  tactics named (Line) and its forecast, no Auto-arrange, a click on a unit of the line moving
  nobody, `Enter` attacking and the keep/raze decision over the overlay, the combat
  flash from the battle's report, the watch screen and the report of the engine's log, the map's
  battle marker and the message's link opening it again), this game's armies (`stage armies`:
  the Armies screen's tactic selector, a change shown as "from next turn", the city screen's
  garrison tactic), governors v2 (`stage governors`: Aristocracy; the Governors screen's mode line,
  a priority, the role cards; an army's Order unit placed in one click on the suggested city, a
  second by a click on a city outlined on the map; the governed city screen; the civics preview),
  supply lines on the HD-2D map (`stage supply`: the supplied army's line drawn,
  the hungry army's marker, "Supplied (cost N of 24)" in the tooltip with the army selected, the
  unit panel's and the Armies screen's words), a game started by another client
  while the page shows the title screen's New game dialog (both go), and the crash screen of a server started with
  `--panic-at`; and (P6) the city screen (a worked plot clicked off and on, the automation), the
  tech tree (`F6`, a click researches), civics (`F3`, a pick and the revolution), religion
  (`F7`), the sliders (top bar and `F2`), diplomacy (`F4`; the trade screen when a player was
  met), the cities list (`F1`), the keys (`?`), the go-to mode (`G`, `Esc`), the map layers, the
  inspector, debug reveal, an AI autoplay turn and a 5,000-turn one stopped from the top bar, the spectator mode (play, pause, step, stop),
  and the trade screen (`stage trade`: 7 gold offered and accepted); the decks (`stage cards`: the
  Armageddon deck's slots in the counter's tooltip, a Great Bard's card in the unit panel and its
  Discover action named for the card, Legacy of Kings cast at a trait picked from a list and
  Starmetal Meteor at a plot picked on the map, each confirmed); and the classic interface on
  the flat map, which those steps drive: the art loads, the interface has its bitmap type and
  icons, the map is not blank, a frame takes under 60 ms (median of 15), the zoom goes through
  the flat map's steps with `+` and `-`, Options (the gear) switches to the isometric map and back,
  and `?renderer=flat` keeps the flat one; then the isometric and HD-2D maps (their steps), and
  the plain URL's default (the HD-2D map, or its isometric fallback). Any console error, page
  error, failed request or HTTP error fails it. The screenshots (`target/web-smoke/*.png`: the
  flat map and every screen above, the isometric and HD-2D maps) are for review, not committed.

**Manual checklist** (a human tester, `ffh-play serve` and a real browser; also try a narrow
window, and a phone: `ffh-play serve --bind 0.0.0.0` on the same network, or the site):

1. The title screen: the island draws and drifts (still under "reduce motion"); New game: change
   the civilization (its leaders' cards follow), world size, difficulty, a game
   option and a victory condition; start. The map centres on your units; the top bar shows the
   turn, gold and "choose research".
2. Map: drag to pan across the east-west seam (no seam in the ground, borders or rivers), zoom
   with the wheel and `+`/`-` through every step (the chart at 8 and 16, then 32 to 128: the art
   stays crisp, never blurred), click and drag the minimap; pan fast to a part you have not seen
   (flat colours for a moment, then the art). Hover plots: the tooltip names the terrain,
   feature, resource, improvement, owner, and yields while visible. Check hills, peaks, forests,
   rivers on edges, roads, lairs and goody huts, resource seals, borders, fog (darkened and
   hatched) and unexplored (black, ragged) plots, a city's light, plaque and banner, the stack
   standards (bright on your units that can move), and the political and yield layers. Reload
   with `?renderer=flat`: the flat map of P5 comes back (and Options → Map toggles it).
3. Select the settler (click), found a city (`B` or the Found button): the research popup comes
   first, then the production chooser; choose with a click, try Shift-click (queue) and "Later".
4. Select a warrior: the path preview follows the mouse with turn numbers; right-click moves;
   hold Shift and right-click two or three plots: they are numbered on the map and the warrior
   stays selected; release Shift: the warrior gets them as orders in turn (it moves once nothing
   else waits for orders); `G` then click goes there, `G` then Shift-clicks add waypoints the
   same way (the go-to mode stays on until Shift is released); `Esc` cancels. A move into a
   rival's land asks to declare war (try "Cancel").
5. Group units on one plot (`All`, `Group`), split them, Ctrl-click rows; `Tab` and `Enter`
   go through every waiting group before any comes up again (`Shift+Tab` backwards); move one unit
   of a two-unit stack with different moves (a warrior and a scout grouped, one plot): `Tab` skips
   the stack from then on, as Civ4 does; once nothing waits the End turn button glows and `Enter`
   (or `Space` with nothing selected) ends the turn; `Home`/`End` go through your cities, `F10`
   opens the capital; `Space`, `F`, `S`, `G`
   work on the selection (also pressed right after clicking another unit); a worker's build
   buttons show turns and their keys (`R` road, `I` farm, `M` mine); compare a few keys with
   Civ4's.
6. End the turn with units waiting (the End turn button; `Enter` selects the next unit then): the
   gate lists them; "Select a unit" goes back, "End turn anyway" ends it. During the AI turns the top bar says who is moving. The next turn's report
   and messages appear in the bottom panel.
7. Open a city (double-click, the Cities list, or the button when your units stand in it):
   replace, append (Shift), insert first (Ctrl) and remove (✕) production; click worked and
   unworked plots, `+`/`−` a specialist, hurry, toggle an emphasis and the automation; Next city.
   Then the empire screens: `F6` research from the tree, `F3` pick civics and read the anarchy,
   `F7`, `F2` move a slider, `F4` open a trade with a player you met and propose something
   (`ffh-play stage trade saves/staged-trade`, load `staged-trade/trade`, if you met nobody).
   Switch the map layers (political, yields).
8. Save under a name, load it back (Load lists it and the autosaves); reload the page in the
   middle of a turn and while a decision is open: the page comes back in the same state.
9. `ffh-play stage attack saves/staged-attack`, load `staged-attack/attack`: hover the barbarian
   with the warrior selected (odds, strengths, modifiers), right-click to attack, read the result.
   `ffh-play stage capture saves/staged-capture`, load `staged-capture/capture`: move onto the
   city; the "Raze or keep?" dialog cannot be closed; answer it (or "AI default").
   `ffh-play stage load saves/staged-load`, load `staged-load/load`: "Later" on the research and
   city popups, load the same save again (they come back); select the warrior in the city's
   stack list, Load: it asks which trireme; pick one, and the warrior is in it.
10. Play 20-50 turns of a normal game: events and votes when they come (each choice's effects,
    "Later"), research done, cities growing, a war with an AI; note anything that looks wrong
    with the turn, the autosave and the journal shown under "Game files".
11. Crash: `ffh-play serve --new --panic-at 3`, end turns until it crashes: the crash screen shows
    the replay command and loads the autosave.
12. Options → Debug tools…: debug reveal on and off, the inspector on your unit, city and player, AI autoplay for
    3 turns, Spectate (pause, step, stop).
13. Watching the AI: a new game with "Spectate" ticked (and "Raging barbarians" for early
    fights): it opens paused, Observing in the top bar; play at the Fast pace, pause, step; a
    move or a city screen change is refused; Follow another civilization (the top bar changes,
    the game does not); after a turn with fights, the map marks them and the tooltip and the
    side panel say what happened; click a stack (its line, each unit's AI role and orders); F
    over it, a note, Flag: the text is copied; the Flags tab lists it, the map has its pennant;
    run the flag's `ffh-play observe --load ...` line: its last checksum is the flag's. Reload
    the page: it comes back paused, observing.

Report a bug with the save or autosave, the journal (`Game files` tab), a screenshot and what you
did.

## Known gaps

* Shift-right-click waypoints are sent when Shift is released, not one by one as Civ4 does (it
  holds the automatic moves while Shift is down instead): the end result is the same.
* Tools → Spectate in a game of your own is the AI autoplay of one turn at a time (FfH's "AI
  autoplay" for your slot, then your turn starts and the next step begins), not an observer slot:
  between two steps your turn starts and ends without you, and the events and votes of your
  civilization are answered by the AI. An observed game (the new-game screen's "Spectate") has no
  such slot: it is an AI-only game.
* An observed game shows the units' moves turn by turn: what moved during a turn shows at its
  end, with the turn's fights and captures marked on the map (not each move as it happens).
* Located messages: only combat (killed, died attacking, fled, withdrew) and cities captured or
  razed carry their plot; the other messages are listed without one.
* The keys of the interface modes (go to, route to, the targeted missions) are written into
  ffh-play from Civ4's `CIV4InterfaceModeInfos.xml` (not vendored); the other orders' keys are read
  from the vendored XML.
* The autosave at each turn boundary adds about half to the time of an AI turn on a standard
  map (`ffh-play bench`: `Game::save` and the file write, about 7-8 ms a turn against 17 ms of
  `update()`); it is the session's largest cost (its other work is about 0.2%).
* AI players never offer you deals or ask anything (ffh-ai) except allied loans (crusade games);
  you propose. The trade screen shows
  an AI's whole gold and gold rate as what it can give (Civ4's AI caps it; that AI code is not
  ported), and Civ4's AI refusals ("we don't want to trade that") are mostly not ported either.
* A path preview for part of a stack uses the whole stack's speed (`approximate: true`).
* Fogged plots show no yield, defence or culture (the rules engine computes them from the real
  improvement, not the one you last saw).
* Automation and FfH AI-controlled units idle (ffh-ai has no `update_automated_group` yet); the
  automate commands are refused, and such groups do not count as ready.
* Civ4 ends your turn by itself when nothing is left to move; here you always end it.
* Messages Civ4 sends only to some players (wars between others, captured or razed cities) are
  recognized by their text and shown under Civ4's condition (you met both teams, you revealed
  the city); event world news are not shown yet, as their sender is not recorded. When FfH moves
  you to another slot during the turn processing, the old slot's messages from just before the
  move are lost (the log has no marker at the move yet, design P7).
* A checked build (debug assertions) panics in ffh-core when the Syliven's Perfect Lyre is taken
  out of a city (a human-only spell) and dropped again: the wonder is counted as built once more
  each time (as in C++, which only asserts). Reported to ffh-core's owners; release builds play
  on.
* Human-only popups without a rules hook (Kuriotates settlements, the raze popup's gift button,
  telling disband from raze, taking control of the Infernals or Mercurians you summoned): see
  design §5.2 and P7. To play the Infernals or Mercurians, take over their slot from an AI save.

## Checklist: a new `Controller` hook

A method added to `ffh_core::Controller` fails `tests/drift.rs` until `human::POLICIES` and
`HumanController` handle it. Decide what Civ4 does for a human there: a popup whose "no answer"
equals not answering yet is **deferred** (record it, answer at the control point with a
command); one where "no answer" loses information is **blocking** (`HumanController::block`
with a `Decision`); AI work Civ4 also runs for humans goes to the advisor.
