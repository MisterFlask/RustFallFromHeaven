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
and **Keys and controls**. With a game running the title screen goes; the menu's Main menu button
(the map icon) brings it back with Resume (or `Esc`). The new-game screen lists the civilizations
with their banners; picking one shows the chosen leader's portrait and its leaders as portrait
cards (or random), the first paragraph of its pedia (its trait and how it plays) and, beside it,
what the leader and the civilization bring: alignment, hero, world spell, each trait's effects,
starting techs, unique and blocked units and buildings, special abilities; then the world
(opponents, world size, speed, difficulty, era, climate, sea level, seed, the game's name), game
options and victory conditions, each choice's description as its tooltip, and "Vanilla experience"
(leaders speak only FFH2's own lines; unticked, each civilization also has its own lines for
every other civilization, `data/text/barks.md`). The load screen has a
card per game (the current one first, then the most recently saved) and a row per save with its
turn and when it was written, the older autosaves folded away. Each game gets a directory in `./saves` (`--saves DIR` to change it).

`serve` takes the GAME flags too (`--seed 3 --world WORLDSIZE_SMALL --new` starts a game at once,
`--load saves/game-3/auto/T0040.ffhsave` resumes one). Use a current Chrome, Firefox or Safari;
nothing is fetched from the internet.

**Start at war (debug).** The new-game screen's "Start at war (debug)" button starts a game in the
middle of a war, for trying combat and the war screens without playing there: an AI-only game of the screen's
settings is played until two civilizations at war fight a battle with 3 or more fighting units
on each side (the attacking stack and the attacked one; with Engagements on, the units each side
commits), and you take over the attacked
civilization at its last turn before the attack (`crates/ffh-play/src/war_start.rs`). Turns are
sequential, so that is the turn of the battle when the attacker moves after you, else the turn
before. Since your civilization no longer plays as the AI did, each candidate is checked by
playing on with you idle, and kept only if a battle of that size against you comes within two of
your turns. The view opens on your threatened stack and a message names the attacker. The search
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
rate, research (click it to choose) with its progress bar and turns, the commerce split by its
icons (click it for the sliders), the Armageddon eclipse (its corona reddens at 50, 75 and 100)
with the counter, mana by sphere, anarchy, golden age, Stasis, idle AI-controlled groups, score
(your share of the world, docs/SCORE.md, with the fair share in its tooltip); the gold's tooltip
gives where the gold rate comes from (the cities' commerce, deals; units with the free ones, units
outside the borders, city maintenance, civic upkeep, inflation); beside them the empire's own
actions, a row of buttons that belong to no unit: your world spell while you can cast it (its
tooltip has its effects and the unit it is cast through, one that can cast it now and loses no
promotion by casting, picked for you; the unit panel no longer lists world spells);
below them the menu (Research, Techs, Cities, Civics, Religion, Diplomacy, Tools, Save, Load,
New, keys `?`, a raw request box `⌘` for testing) and End turn. The map fills the middle (minimap
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
gives the odds of winning and retreating with both strengths and their modifiers. The layers
(buttons bottom left, `Ctrl+1/2/3`): **Terrain**; **Political** (each owner's land washed with
its colour); **Yields** (the ground dimmed, and the food, production and commerce of the plots in
sight with their icons; fogged plots show none). The minimap is a pixel a plot in the same
colours, owned land in its owner's colour, cities as light dots.

**The flat renderer** of P5 (colours, glyphs and labels: lettered dots for resources, boxes for
improvements, units and cities; no sprites) is kept for debugging, in the classic interface of
docs/ART.md: open the page as `http://127.0.0.1:8642/?renderer=flat`, or choose it under Tools →
Options → "This browser" (which also offers the HD-2D map, the default, and the isometric 2.5D
map, its fallback). The choice is remembered by this browser. The 2D pixel-art map that drew the
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

**Mouse and keys** (also under `?`). The keys are Civ4's: the orders' keys come from the game's
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
| `R`, `I`, `M`, … | a worker's builds: road, farm, mine, … (`Shift+W` windmill, `Alt+C` chop) |
| `L` / `U` / `Shift+U` | load / unload / unload all |
| `Shift+P` | pillage |
| `Del` / `Backspace` | delete the selected units (asks first) / cancel their orders |
| `F1` `F2` `F3` `F4` `F6` `F7` | cities, finances (the sliders), civics, diplomacy, the tech tree, religion |
| `F5` | armies (this game's armies: docs/ARMIES.md) |
| `F9` | governors (this game's governors: docs/GOVERNORS.md) |
| `F8` | cultural pressure (this game's cultural actions: docs/CULTURE.md) |
| `Ctrl+S` / `Ctrl+L` | save / load |
| `Ctrl+1` `Ctrl+2` `Ctrl+3`, `?` | ffh-play's own: the terrain, political and yield layers; the keys list |
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
| "A tap:" | in the city screen, the research chooser and the tech tree: what a tap on a build or a tech does (build now, add to the queue, put first; research now, add to the queue): Shift- and Ctrl-click with a mouse |

A long press and a double tap do nothing on the map. What touch does not have: Shift waypoints
(use Go to for one destination), Ctrl-click on a unit row (Split, then a row), and the keys
(the panel's buttons, the menu and the Cities list cover them; `?` in the menu lists the controls).

**Units.** The selection panel lists your units on the plot (click a row: its group;
Ctrl-click: add or remove that unit), "All", "Group" (one group), "Split" (each unit its own
group), "Next", the group's orders, and the actions the server says the selection can take now
(greyed when not now): orders such as skip, sleep, fortify, sentry, heal, pillage, found, join,
spread, the great-person missions; go-to and route-to and the other modes that need a plot (click
it then; the valid targets are outlined); builds with their turns; spells with their help;
promotions; upgrades with their price; load, unload, wake, cancel, delete. The sections follow
Civ4's button order (Orders, Build, Great person, Religion, Cargo, Spells, Promotions,
Upgrades). "Promote…" lists the promotions with their help, "details" by Spells lists every
spell with its help and whether it can be cast now. Units in a transport are listed under it.
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
unit cycling", Tools → Options). As in Civ4, the cycle (`Tab`, `Enter`, the next group after an
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
then each city with nothing to build (click:
build it now; Shift-click: add to the queue; Ctrl-click: put it first). "Later" sets one aside
for this turn. End turn (the button, or `Enter` once no group waits) asks first when an event, a vote or the research is
open or units wait for orders: answer them, select a unit, or end the turn anyway (Civ4's
consequences: open events are dropped, votes get one more turn and then abstain, empty research
takes the automatic pick). A question the rules must have answered at once (keep or raze a
captured city, with its buildings and holy cities; a free tech, each with its worth in research
and what it brings; an election) appears as a dialog that cannot be closed, with the choices, the
advisor's pick marked, and "AI default".

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
choosing another) and everything it can build with turns and cost, each with its tooltip (a
unit's strength, moves and class, what it does; click: build it; Shift: append; Ctrl: put it
first). "‹ Previous city" and "Next city ›" go through your cities. With cultural actions on
(this game's own system, docs/CULTURE.md) the screen also has a **Culture** section, under the
city's numbers: the culture rate and the half the channel takes; the city's channel (the action,
the civilization, the religion) with what it does now and "Stop": for Foment Revolt each of their
cities in reach with its distance, the pressure it adds a turn against the city's resistance, a
small scale (your pressure there now, where it settles) and what that brings (no effect, unrest,
revolt, the city joins you; a capital never) and when; for Spread Religion the chance a turn and
each city's part of it (or that it has the religion); then "Channel culture" (folded as "Change
the channel" under a channel): the action, the religion, the civilizations in reach with a
summary or the reason it is not possible, the preview of the picked one, and the button that
sets it. Last, the revolt pressure on the city: a scale by civilization against the thresholds
(each point of unrest, revolt, the line where one civilization's pressure takes the city), its
level and unhappiness, what each civilization's channels add a turn and where that settles, and
its resistance (half its culture, 2 per defender; in revolt only its defenders, which the
predictions count). With armies on, each queued land combat unit
has a destination selector (the city's default, its garrison, another city's garrison, an army
with its size and whether it is in supply: a unit bound elsewhere marches there); an army's
requisition order leads the queue as "Requisitioned by the 2nd Army", without ✕ (it is released on
the Armies screen); under the queue, the city's default destination, its garrison and the armies
that requisition from it. With governors on (docs/GOVERNORS.md) a **Governance** section heads
the right column: a directly ruled city says so (with "Hand to a governor"); a governed city shows
its governor (or the interim one), "Replace with…" (the bench, with each candidate's fit) and
"Rule directly" (or "Rule directly instead of…" a direct city); its production is shown but not
chosen (a requisition order shows there too).
With the isometric map the screen is Civ4's city view: the map glides to the city at the largest
zoom that shows its radius, veils what lies outside it and draws the radius in gilt, each plot
with its yields and the worked ones with a citizen; click a plot there to work it or stop. The
rest of the screen stands in panels at the sides, the interface steps aside, and the camera goes
back when the screen closes (docs/ISO.md, "The city screen").

**Empire.** **Techs** (`F6`): the tech tree by prerequisite depth, what you know, research and
can research now, with costs, turns and progress, and each tech's tooltip; click a tech to research
it (with what it needs first), Shift-click to queue it. **Civics** (`F3`): each civic option with its
civics (the current ones marked, the ones you cannot adopt greyed, their tooltip saying why: the
tech, a war, an alignment, a state religion); pick civics and the screen shows the
revolution's anarchy and upkeep (computed by the rules), then "Revolution!". **Religion** (`F7`):
the religions, the tech whose first discoverer founds each, their holy cities as far as you know
them, your cities with each, and "Convert" (with the anarchy). **Cultural pressure** (`F8`, the
menu's Culture; with cultural actions on): your cities under revolt pressure (each with its scale,
level, unhappiness, each civilization's pressure with what its channels add a turn and where it
settles, and when the next level comes), the channels aimed at you (with the cities of yours they
reach and what they do there), your channels with what they do (and "Stop"), the channels you see
between others (all channels are public to civilizations that have met the caster), and what
other leaders remember of your cultural actions. On the map a city you see under revolt pressure
has a mark at its banner's end: a small ochre dot while the pressure has no effect yet, an amber
"!" for unrest, a red "!" in revolt, a red "!!" in a gold ring when close to changing hands.
**Armies** (`F5`, the menu's Armies; with armies on): every army with its name
(type to rename), units, place (click: select it on the map), supply, target size (with its units,
recruits marching and units in training against it: its requisition orders and the units its
cities build next for it, as "more wanted" counts them), "Upgrade all", "Disband", its
**requisition** (the cities it requisitions from, how many more it wants, its orders in
production with their turns, and "Requisition from…": every city of yours with a checkbox, the
unit it would train for the army, its production turns and its march to the army; "Release all"),
and the recruits marching to it (where from, where they are (click: select it on the map), how
many turns away, or how long they have waited); then each city's garrison with the recruits
marching to it. **Governors** (`F9`, the menu's Governors; with governors on): every city with
the armies that requisition from it, its rule (direct or governed, and when it may change), its
governor (name, profile, traits; the replacement penalty) and a selector to replace its governor
from the bench; then the bench. **Finances** (`F2`, or click the
commerce split in the top bar): each commerce's share with `−`/`+` by Civ4's step (only the ones
you may change; gold takes the rest), and the gold per turn by its parts, as in the gold's tooltip.
**Diplomacy** (`F4`): every player you met, by their leader's portrait (war or peace, open
borders, pacts, vassals, attitude with its reasons as its tooltip, alignment and state religion,
score, your deals with them; your own alignment and state religion above the list) with "Trade…" and "Declare war"
(it asks first). Each playable leader has a portrait in the interface's pixel art
(`tools/m7/leaders.py`), also shown at the top of the trade screen, on a deal offered to you, on
the question to take over a civilization that appeared and by each candidate of a vote. The trade screen lists what each side can give (refusals greyed with Civ4's reason); click items into the deal
(gold with an amount; peace, open borders and pacts go both ways, so they are greyed on both
sides when either side refuses them), "Propose": the AI accepts or declines at once.

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

**Tools** (the Tools menu): **Debug reveal** (the whole map and every unit and city; the top
bar says DEBUG REVEAL); the **Inspector** (the rules engine's own data of your unit, group, city,
player or team, and of anything under debug reveal, plots too; an info entry by kind and index);
**AI autoplay** (the AI plays your civilization for N turns, then you have control again; while
it runs the top bar counts the turns left and "Stop" ends it: you have control at your next turn);
**Spectate** (below); **Take over…** (play another slot of an AI-only save); **Options** (Civ4's
player options); the raw request box.

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
`DIR` may be a game's directory. See "Testing the browser client" below for the checklist.

**Engagement combat** (docs/ENGAGEMENTS.md; the game options Armies and Engagements). An attack
by land on a land plot is one formation battle: each side's line on a grid of three rows by its
lanes. In the browser:
- **Attacking.** The tooltip over an enemy plot next to the selection says it is a battle ("Battle:
  5 of yours can fight, against 3 defenders"; the odds beside it are the duel's of the strongest
  pair, a rough guide). A right-click there (a second tap on a phone) with an army or a stack of
  units that can attack opens **the battle plan** with the rules' auto formation; Attack (or
  `Enter`) sends `engage` with the formation arranged there (an attack that declares war says so
  on the button), Cancel (`Esc`) leaves the map as it was (not once the attack is on its way:
  the battle's screens take over from the plan). Shift+right-click on such a plot
  attacks at once in the rules' formation (a `move`); on any other plot it lays a waypoint as
  before. Over a plot further away, or next to units that have used their moves or their attack,
  the tooltip says "Battle on arrival" ("once they can attack") with the selected units that could
  fight there, and the path's odds chip stays; a right-click sends the move, and the battle on
  arrival takes the rules' formation of whoever stands next to the plot then. A lone unit's
  attack, and every attack at sea or by air, goes as before.
- **After a battle** the struck plot flashes (from the battle's report: several units may fall
  at once). An attack made from the plan goes on to the battle screens, and its other messages (a
  defensive strike, the city captured) wait until they close, so they do not tell the end before
  the replay does; a city's keep-or-raze question still comes at once, over the plan, as the
  engine asks it while the attack is made. After an attack without the screens (Shift, a move) a
  toast names the battle with **Watch** and **Report** links; so do the battle's message under
  Messages and the turn report's **Battles** list (your battles since your last turn began,
  against you too). A marker sits on the plot's top-right corner for this turn
  and the last (crossed swords: a gold rim for a battle won, azure held, crimson lost or
  repelled; drawn at 32 px tiles and up, inside its plot): a click on it watches it again, a click
  elsewhere on the plot selects as usual. A battle older than the kept logs shows its summary
  only.
- **Units.** The unit panel shows the lead unit's role, its A, Dv and HP in a battle, its speed,
  whether it is large and its actions in the front, middle and rear rows (the view's `engage_*`);
  a unit row's role badge has the same in its tooltip. The help of first strikes, withdrawal, collateral damage, Blitz and combat
  limits (units and promotions) adds what they mean in a battle. The Stack attack option is
  ignored (its line says so).

**The battle screens** (formation battles, phase 6 revision 4; the owner's prototype's
battlefield). They open from the game as above (the plan from an attack, the battle and the report
from a battle's links and marker), through the engine's `engage_preview`, `engage` and `battle_log`
(`apiSource` in `battle.js`); a dev entry opens them over a fixture, `?battle=field` (or `forest`,
`hills`, `city`, `raid`: a city that falls). In an overlay over the dimmed map, on one isometric
battlefield of M7 tiles (six ranks deep, the attacker's rear to the defender's rear, by the lanes;
the ground's scenery round it: trees round a forest or jungle, the city behind its defenders, hills
on a hill): **the plan** (both lines, yours in the rules' auto formation, theirs as you know it;
each unit's HP bar and its chance to fall; a tooltip over a unit says what it is: role, A, Dv, HP,
speed, its actions in each row and whom they reach, its spell, its specials, its chance to fall;
click one of your units, then a cell of your grid to move it there or another unit to swap (a
large unit takes two cells of a row; a unit with nowhere to go leaves the line), "Take out of the
line"; the pool's other units listed (click one, then a cell); Auto-arrange; the slots and lanes
of each side and why; the ground's rules in words; the forecast of the formation, asked of the
engine at each change: each side's expected losses, their line wiped out, the plot falls, yours
wiped out; Watch or Report after the attack; Attack or `Enter`), **the battle** (the log played
one action at a time as a snap, the owner's pick of the battle-feel spike, 2026-10-31: every
unit breathes; on an action's contact frame the pair cuts to its impact poses (the strike strip's
contact frame and the target's flinch frame where the unit has anim-sprint frames, else the
sprite leaning in, a little larger, and the target leaning back under a white flash) and slides
together, lifted above the field with an ink outline, and the slash or the sparks, the number and
a small shake land; a held beat of 0.5-1.1 s with embers drifting, then a quarter-second ease
back. Crits hold longer and kills hold, the death playing after the return; on crits and kills only
the rest of the field drops into a shade round the pair, the camera never zooms; CRITICAL and
SLAIN under the numbers; a miss is a dodge with a whiff, a heal glows, a shot or a spell arrives
as a streak while its shooter stays on its cell, splash targets snap together, a knock-back eases
back a row on the return, an escape leaves after it; no shake under reduced motion; each side's
standing units; the log beside it under round and pass headers, a unit's speed before its line,
an action's line on its contact frame; play/pause `Space` (the action freezes where it is), step
`→`, skip to the report `Esc`, speeds 1x (the default) to 8x `1`-`4`; played through, the report
follows) and **the report** (the battlefield where the battle ended, whether the plot fell, each
side's fallen, escaped and standing with their heads, the whole log; Replay, Continue). The
modules: `battle.js` (the overlay, the keys, the viewer's preferences, the sources),
`battle_plan.js`, `battle_watch.js` (the replay's clock and timeline), `battle_snap.js` (the
snap: each action's timing plan, roles, poses and drawing over the stage), `battle_report.js`,
`battle_stage.js` (the battlefield, the figures and the effects; the anim-sprint strips in
`app/anim/`), and the pure `battle_log.js` (the protocol's shapes in its header, the formation's
editing, the log folded into states and lines, the words of the rules by their codes); `battle_fixture.js` is the fixture, made by `assets/test/make_battle_fixtures.mjs`
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
./target/release/ffh-play stage capture|attack|automoves|load|trade DIR
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
  `quick`); numbers out of range are refused. `occ` (one-city challenge, no healing for humans)
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
  battle plan, and the battle takes the city),
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
{army, cities}` (the army requisitions from these cities, in this order, and no other), `recruit_dest
{city, index, dest}` (this game's armies: `dest` is `{"garrison": city}`, `{"army": id}` or null
for the city's default; `index` -1 sets the city's default; a requisition order's is its army's;
docs/ARMIES.md. A marching recruit takes no orders: `move`, `mission`, `do_command`
(also the units of its type an `alt` command reaches), `join_group` (as the unit or the head),
`auto_mission` and the army commands refuse it, the reason naming where it marches, "the Warrior
is marching to the 2nd Army: it takes no orders until it arrives". A requisition order cannot be
taken off by hand: `pop_order` refuses it, and `do_task TASK_CLEAR_ORDERS` while one is queued,
"requisitioned by the 2nd Army: release the city from the army's requisition on the Armies
screen"), `govern_direct {city, instead_of}`, `govern_hand {city}`, `govern_assign {city,
governor}` (this game's governors, docs/GOVERNORS.md; a governed city refuses `push_order`,
`pop_order`, `recruit_dest` and the production tasks), `engage {army | units | from, to,
formation?, declare_war?}` (engagement combat, docs/ENGAGEMENTS.md: one formation battle on plot
`to`, `{x, y}`, next to the attackers; `army`, the selected `units` or a plot `from` say who
attacks, and the launcher's group moves in when no defender is left on the plot; `formation`
`[{unit, lane, row}]` the attacker's line (rows 0 front, 1 middle, 2 rear; a large unit at its
first lane fills that lane and the next), checked against the pool on the attacking plot (every
unit of yours there that can attack the plot now), its side's lanes and rows, one unit a cell, the
slots and the river's slots; without it the rules' auto formation; an attack that would declare
war answers `{"confirm": {"kind": "declare_war", ..}}` as `move` does until it is resent with
`declare_war: true` (`false` cancels it); refused, with the reason, for a plot you do not see,
not next to the attackers, at sea, with nobody to fight, no unit that can attack or a formation
the rules refuse, and refused, "engagements are off in this game", without the option; the reply
carries `report`, the battle's report id, and `battle`, its summary; through
`group_attack_force`, as every attack; a `move` onto an enemy plot under the option is one battle
too, in the rules' auto formation, and its reply carries the same `report` and `battle`;
`PLAYEROPTION_STACK_ATTACK` is ignored under the option), `research`, `update_civics`, `percent_change`, `convert`, `declare_war`,
`propose_deal`, `event`, `vote`, `set_option`, `auto_moves`, `end_turn`, `autoplay`, and with the crusade ruleset (see Crusades
below) `crusade_answer`, `crusade_lend`, `propose_loan`. Each is
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
AI (ffh-ai's `consider_offer`) accepts gifts, peace after 10 turns of war unless it is clearly
winning, open borders when it likes you, and a fair tech swap, and refuses anything else.

### Crusades

Not Civ4 (docs/CRUSADE.md): a rules variant, on when the new game says `"crusades": true` (the
new-game screen's *Crusades* box). A council's war resolution then calls its members to lend
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
  `max_turns`; null without the crusade ruleset or contact.
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
  right; Keys, then the crusade topics), not in the dialogs: each dialog's (i), in its title bar,
  opens the help at its topic over the dialog, so the body holds only the decision.

### Queries

Requests are `{"q": "<name>", ..}`; replies are JSON with type indices (`data` names them).
Errors: `invalid`, `not_owner`, `gone`, `busy`.

| `q` | Reply |
|---|---|
| `status` | phase (`human`, `running`, `decision`, `over`; over the transports also `stuck`, `no_game`, `crashed` with the report), turn, year, you, the active players, the open decision, the winner, `view_version` (changes whenever the view does: a command, a decision opening or closing, debug reveal), `ai_auto_play` (the AI autoplay's turns left, 0 when off; observed, the turns left to play), `observe` (an observed game: `human` is the player the view follows, phase `human` means stopped at a turn boundary), `build` (the commit the binary was built from), and over the transports `game {dir, journal, autosave}` |
| `pending` | open events (with each option's effects), votes, research needed, idle cities, promotions, ready groups, idle AI-controlled groups, crusade calls |
| `view` | `me` (gold and rate, research, sliders, civics, score, Armageddon counter, mana, Stasis, the player options, AI autoplay), the players you met, the cities you revealed, the units you see, your groups (`ready`: some unit can move, as `pending.ready_groups`; `selectable`: every unit can, what Civ4's cycle selects; neither for a marching recruit's group), `group_cycle` (your groups in Civ4's cycle order) and `has_auto_unit`, `pending`, `log_next`; with armies (null without), `armies` (each: `id`, `name`, `group`, `units`, `x`, `y`, `in_supply`, `target_size`, `marching` (the recruits marching to it: unit ids), `requisition` (`cities`: the cities it requisitions from, in order; `orders`: its requisition orders in production, each `{city, name, unit, index, turns}`; `head_orders`: how many other orders bound for it its cities build next (the first order behind any requisition orders, whose destination or the city's default names the army); `shortfall`: how many it lacks of its target size counting its units, recruits marching, `orders` and `head_orders`, 0 or less: none), `disband_to`, and `upgrades` with their `upgrade_price`) and `garrison_marching` (the recruits marching to garrisons: unit ids), and each unit's `army`, `garrison`, `army_unit` and `march` (null, or a marching recruit's `{dest, to, from, turns_left, stalled, diverted, since}`: where it goes and its name, the city that trained it, the turns it needs along the rules' path (null: no way there now), the turns without progress, whether it was sent to the nearest garrison instead, the turn it set out); `engagements` (whether the game has engagement combat) and `battles` (null without it: your battles of this game turn and the last, as `battle_reports`' summaries: the map's markers); under engagement combat each unit with a role its `engage_role` (a `CombatRoleType`), `engage_attack` (its A before the bonuses of a plot it attacks, x100), `engage_defense` (its Dv where it stands, x100), `engage_max_hp` (its HP in a battle; `hp` and `max_hp` stay the health percent), `engage_speed` (before first strikes), `engage_actions` (`[{kind, n}]` in its front, middle and rear rows, `kind` an action kind: `Strike`, `Charge`, `Stab`, `Shoot`, `Spell`, `Bombard`, `Heal` or `None`), `engage_dodge`, `engage_crit_bonus` and `engage_large` (it fills two cells of a row and counts two slots); with cultural actions, each of your cities' `channel` (`{action, target, religion}`, absent when it channels nothing), and `revolt` (the level: 0 pressure without effect yet, 1 unrest, 2 revolt, 3 close to changing hands) and `unrest` (its unhappiness) for the cities you see under revolt pressure |
| `map` | flat arrays `y * w + x`: `vis` (0 unrevealed, 1 fogged, 2 visible), `plot_type`, `terrain`, `feature`, `bonus`, `improvement`, `route`, `owner`, `river` (bit 0 river on the south edge, bit 1 on the east edge, bits 2-4 north/south flow + 1, bits 5-7 west/east flow + 1), `city`, `yield` (`food \| production << 8 \| commerce << 16`, visible plots only), `map_rev`, and `epoch` and `rev`. `map {since: R, epoch: E}` answers only the tiles changed since revision `R` (`full: false`, `tiles` = their indices, each array the values of those tiles), or the full map (`full: true`) when `E` is not the current epoch, `R` is unknown or much of the map changed |
| `plot {x,y}` | a plot as you know it (`min_level` when units below a level cannot enter); while visible also yields, defence, culture and the units (under debug reveal each with `ai_type`, the AI's role, and `missions`, its group's orders) |
| `group {units}` | the units in detail and the actions: `mission`, `target` (an interface mode, `mode`: go-to, route-to for workers, range attack, the air modes; needs a plot), `command`, `load`, `spell`, `promotion`, `upgrade`, `army` (this game's armies: `army` the action, `label`, `help`), each `enabled` or not, each with its `cmd`; a selection with a marching recruit has none and says why in `marching` ("marching to the 2nd Army") |
| `path {units,x,y}` | the path preview: nodes with the turn of arrival and moves left, total turns, a war it would declare, whether it ends in an attack |
| `combat {units,x,y}` | the attack preview: attacker, defender, odds of winning and of retreating (per mille), strengths, the modifiers of both sides. Under engagement combat, for land units attacking a land plot that holds a defender: `engagement: true`, `defenders` (their number), `declare_war_team` and `on_arrival`. An attack the selection can launch now (the plot is next to it and some unit on its plot can attack the plot now): `on_arrival: false`, `valid: true`, `pool` the units on its plot that can (other armies and the garrison there included). Otherwise (a go-to that ends in an attack, units that have used their moves or their attack): `on_arrival: true`, the duel's `valid` and odds kept, `pool` the selected units that could fight there once they can attack it. The duel's odds and strengths of the strongest pair stay beside them (the forecast of a battle is `engage_preview`'s). Ships, air units and the option off: the duel's preview |
| `engage_preview {army \| units \| from, to, formation?}` | engagement combat: the plan of an attack (the battle screens' plan), in the battle log's shapes (`assets/app/battle_log.js`'s header lists what the screens read): `plot` and `from`, the `ground` and its rules, `params` (the rule values the screens word), both sides' `slots` and why and their `lanes`, `sides`, `fighters` (the pool, then the defenders you see, each in its cell or `lane` null: its role, A and Dv, HP, speed, actions per row, spell, dodge, crit bonus, size and specials), `formation` (the one asked for; else the rules' auto formation), `auto_formation`, `reserve` (the pool's units not in it), `def_formation` and `def_reserve` (the defenders' line in the rules' auto formation, as the real fight forms it, and the rest on the plot), `forecast` (`runs` battles of the real resolver on a private RNG seeded from the plan, never the game's: `lost_x100` and `escaped_x100` per side, `line_wiped_permille`, `plot_falls_permille`, `attackers_wiped_permille`, and `per` fighter its `fall_pct` and `escape_pct`), `declare_war` (the team the attack declares war on), `errors`. A formation the rules refuse gets the plan in the auto formation, `forecast` null and why in `errors`; a plot you do not see, one not next to the attackers, or an attack the rules refuse has no plan: the reply is `errors` and `declare_war` alone. Refused without the option |
| `battle_log {report}` | engagement combat: a battle's report with its full log as the battle screens replay it (`report` is the report's id: `id` is the envelope's): `battle_log.js`'s header lists every member. Only for a battle you fought on a side (or under debug reveal), while its log is kept (the current and previous turn's battles and the last `ENGAGEMENT_REPORT_LOGS_KEPT`): else `gone`. Your side shows its real owners, the other side the owners you saw (hidden nationality). `events` are what the battle shows (the opening at round 0: units that sat out afraid, defensive strikes; then every action, one unit at a time, with its hits, heals, knock-backs and escapes), `after` what it did after (XP, heals, kill effects, flight, the move in), `outcome` whether the plot fell and each side's line by fate |
| `battle_reports` | engagement combat: `{on, reports}`, your kept battle reports as summaries, newest first (`battle_log.js`'s header lists a summary: where, when, the sides with what each committed and lost, the outcome, `has_log`) |
| `city {city}` | the city screen of one of your cities: plots (with `can_work`), specialists (with `plus` and `minus`: what `do_task TASK_CHANGE_SPECIALIST` accepts now), yields, `commerces` (each with its `rate`, the empire's `percent` and its parts: `rates`, `specialists`, `buildings`, `religions`, `empire`, `process`, `modifier`), `next_worker` (who a click on a plot the city does not work takes: `{idle}`, `{specialist}` or `{plot}`; null for nobody), happiness, health, buildings, hurry, emphasis, queue, the buildable list with cost and turns; `cultural` (null without cultural actions; pressures and amounts in hundredths of culture, chances in hundredths of a percent): `rate`, `amount` (what a channel takes a turn), `diverted` (what it took this turn), `channel` (null, or `{city, caster, action, target, religion, since_turn, hostile, amount, reach, unseen, chance}`), `options` (every channel it could set, one per action, civilization with a city in reach and religion of the city: `ok` or `why`, `hostile`, `reach`, `unseen` (cities in reach you have not seen), a spread's `chance`), each `reach` entry `{city, name, owner, distance, effect}` (the distance effect, percent) and `amount` (pressure or chance a turn), for Foment Revolt with `resist` (the city's resistance a turn now), `resist_revolt` (its defenders' part: all it resists in revolt, making no culture), `rest` (the other casters' pressure there), `in_revolt` (in revolt now: its full resistance cannot be read, so `resist_revolt` holds both ways), `settles` (where the caster's pressure settles from now under this city's channel alone, with the city's resistance falling to `resist_revolt` from the turn after the total reaches the revolt: docs/CULTURE.md, "Predictions"), `now` (the caster's pressure there), `others` (what the caster's other cities add a turn), `capital`, for Spread Religion with `has` and `religions`; `pressure` (on this city: `total`, `casters` `[{caster, pressure, inflow, settles, rest}]` (`rest`: the others' pressure), `unknown`, `resist`, `resist_revolt`, `in_revolt`, `defenders`, `level`, `unhappiness`, `capital`); `thresholds` (`unhappy_step`, `unhappy_max`, `disorder`, `near_flip`, `flip`, `fade_percent`, `resist_percent`, `garrison`, `channel_percent`, `full_range`, `range`, `spread_per_culture`, `spread_max`, `flipping`); `recruit` (null without armies): the city's default destination, the destinations it may choose, its garrison, the armies that requisition from it (`requisitioned_by`: `{army, name, target_size, shortfall}`); with armies, each queue entry of a land combat unit has `dest`, `dest_name` and `requisition` (an army's requisition order: first in the queue, not removable by hand) |
| `governors` | the Governors screen: `on`, `slots`, `cooldown`, `penalty`, `penalty_turns`, each city (`governed`, `cooldown`, `governor` with name, profile, traits, `penalty_turns`, `requisitioned_by`: the names of the armies that requisition from it), the `bench`; the city query's `governance` adds the city's `bench` with each candidate's `match`, `swappable` direct cities, `slots`, `direct_count`, `ready` |
| `armies` | the Armies screen: `on`, the `armies` (as the view's, with `recruits`: the recruits marching to it in full, each its `march` with `id`, `unit`, `x`, `y`; and the requisition's `candidates`: every city of yours with `picked`, `governed`, the `unit` it would train for the army, its `production_turns` and `march_turns` (null: no way by land), its `order` in progress; `open`: whether the requisition may change), each city's garrison (`units`, `governed`) with the recruits marching to it (`recruits`), `supply_range`, `stall_turns` |
| `culture` | the Cultural Pressure panel: `on`; the channels you see (each `{city, city_name, x, y, caster, action, religion, target, since_turn, hostile}`): `ours` and `aimed` (at your team) also with `amount`, `reach`, `unseen` and a spread's `chance` as in the city query (for `aimed` the reach is your cities), `others`; `pressure`: your cities under revolt pressure (the city query's `pressure` with `city` and `name`); `memories`: what met leaders remember of your cultural actions (`{player, hostility, friendship}`, at most `memory_max`); `thresholds` as in the city query |
| `techs`, `civics`, `religions`, `players` | the tech tree (prerequisites, depth, turns; each tech's `help` and, unless you have it, `unlocks`: the units, buildings, builds, civics and religions of its help as type indices, which the research chooser shows as icons), civics (availability, upkeep, anarchy; `civics {with: [civic, ..]}` adds `proposed`: the set a revolution to those civics makes, whether `update_civics` would be applied, its anarchy and upkeep), religions, the players you met with your deals |
| `trade {with}` | the trade tables both ways (see Deals) |
| `speech {with,comment?,salt?}` | what the AI leader of `with` says to you: `{with, comment, text}`; `comment` an `AI_DIPLOCOMMENT_*` (`DiploCommentType`), or none for what it says when you contact it (its greeting, Civ4's `AI_getGreeting`, or `AI_DIPLOCOMMENT_REFUSE_TO_TALK`); `salt` picks among its lines (Civ4 uses the interface's random numbers); `text` null when it has no line |
| `moments {since}` | the leader moments from index `since` (`view.moments_next` is the next one): `{i, t, player, comment, text}`, an AI leader opening the diplomacy screen with you (first contact, a war declared on you) and what it said; the last 20 are kept in a save |
| `log {since}` | your messages from index `since` (`view.log_next` is the next one): `{i, t, text, player, at?}`, `at` the plot `[x, y]` a combat or a city captured or razed happened on; observed, every player's messages. A battle's message (engagement combat) also has `kind: "engagement"` and `report` (its report's id), so a client links it without reading its words; so do the messages of a command's reply and of the turn report |
| `report` | the turn report: what changed between your end of turn and now, or the end of the game (units lost and gained, cities founded, lost, grown, starving, buildings, techs, civilizations met, wars and peace between those you had met, enemies near your cities, the Armageddon counter, borders; under engagement combat `engagements`, the battles you fought since your last turn began, as summaries, newest first, each also a line "battle #12 at Prespur: held; 2 lost, 5 of theirs fell"), with the messages |
| `text` | a plain-text summary for agents |
| `ascii {x,y,w,h,layer}` | an ASCII map, 2 characters per plot, north up, `::` unrevealed; layers `terrain`, `political`, `yield`, `units`; a legend |
| `inspect {kind,oid}` | `{:#?}` of your unit, group, city, player, team, deal, or of an info entry (`UnitType`, `units`, ...) with id or index `oid`; any object, and plots, under debug reveal (a plot holds every team's data: use `plot`) |
| `data` | the static tables (every kind: type, name, texts, the main gameplay fields; missions, commands and builds with their Civ4 hotkeys `keys` and button order `order`), the interface modes' hotkeys, a few global defines |
| `setup`, `saves` | the new-game lists; the saves of the game's directory |
| `crusades` | crusades and loans (see Crusades): `enabled`, `wars`, `rules`, `military_strength`, `lendable`, `crusades`, `reports` (the `trade` query's `troops` offers loans) |

What you see is what Civ4 shows the player: your units, and others only on plots you see and
when they are not invisible to you (hidden-nationality units show as barbarians; the cargo of a
foreign ship is hidden); cities you revealed (size only while you see them); players you met;
bonuses your techs reveal; improvements, roads and borders as you last saw them.

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
"turns_left": N}` and the autoplay stops before the next turn slice (FfH `setAIAutoPlay(0)`,
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
```

* `web-check.sh` needs node (it is skipped without it); the pure modules (map maths and map
  deltas, markup, state and Civ4's group cycle, the hotkeys, the tech tree's layout, the API layer
  against a fake `fetch`, the battle screens' log folding, log lines, formation editing and the
  ground's words over the battle fixture, the watch screen's snap: each action's timing plan,
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
  panel, the battle in the tooltip, a right-click opening the battle plan in the rules' auto
  formation with its forecast, a unit moved to another cell by mouse (the forecast asked again),
  `Enter` attacking with that formation and the keep/raze decision over the overlay, the combat
  flash from the battle's report, the watch screen and the report of the engine's log, the map's
  battle marker and the message's link opening it again), a game started by another client
  while the page shows the title screen's New game dialog (both go), and the crash screen of a server started with
  `--panic-at`; and (P6) the city screen (a worked plot clicked off and on, the automation), the
  tech tree (`F6`, a click researches), civics (`F3`, a pick and the revolution), religion
  (`F7`), the sliders (top bar and `F2`), diplomacy (`F4`; the trade screen when a player was
  met), the cities list (`F1`), the keys (`?`), the go-to mode (`G`, `Esc`), the map layers, the
  inspector, debug reveal, an AI autoplay turn and a 5,000-turn one stopped from the top bar, the spectator mode (play, pause, step, stop),
  and the trade screen (`stage trade`: 7 gold offered and accepted); and the classic interface on
  the flat map, which those steps drive: the art loads, the interface has its bitmap type and
  icons, the map is not blank, a frame takes under 60 ms (median of 15), the zoom goes through
  the flat map's steps with `+` and `-`, Tools → Options switches to the isometric map and back,
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
   with `?renderer=flat`: the flat map of P5 comes back (and Tools → Options toggles it).
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
12. Tools: debug reveal on and off, the inspector on your unit, city and player, AI autoplay for
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
