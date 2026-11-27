// The Help of the Cataclysm mode (docs/CATACLYSM.md). Its topics carry `mode: "cataclysm"`, so the
// help display (help.js) lists them in a Cataclysm game and the classic game's topics in a classic
// one. Only the controls are written here: the rules' sentences and their numbers come from the
// view (`S.view.cataclysm`: `rules_help`, `order_help`, `apocalypse.rules`, `victory`, ...), which
// the server writes from the rules' own constants, so the Help cannot drift from the rules.

import { h } from "./dom.js";
import { S } from "./state.js";
import { addHelp } from "./help.js";

const GROUP = "Cataclysm";
const cat = () => S.view?.cataclysm ?? null;

/** A list of the bare sentences `lines` (null when there are none). */
function list(lines) {
  const l = (Array.isArray(lines) ? lines : lines ? [lines] : []).filter(Boolean);
  return l.length ? h("ul", l.map((t) => h("li", t))) : null;
}

/** A heading and its list, both left out when the view gives no sentence. */
function section(title, lines) {
  const body = list(lines);
  return body ? [h("h4", title), body] : [];
}

/** A list of `name: sentence` from the object `o`, in the order of `names` (`[key, label]`). */
function named(o, names) {
  const rows = names.filter(([k]) => o?.[k]).map(([k, label]) => h("li", h("b", `${label}: `), o[k]));
  return rows.length ? h("ul", rows) : null;
}

const KEYS = [
  ["Enter", "End the turn"],
  ["Tab", "Select the next squad with moves left"],
  ["C", "Centre the map on the selected squad"],
  ["Arrow keys", "Move the map"],
  ["+ / -", "Zoom"],
  ["F1", "Cities and caravans"],
  ["F2", "Worldseal: Worldstrain, Lifeboats, how the game ends"],
  ["F3", "Rituals"],
  ["F4", "Diplomacy and crusades"],
  ["F6", "Research and doctrines"],
  ["F7", "Creeds"],
  ["F9", "Great People Court"],
  ["Shift+F9", "Houses"],
  ["F10", "The capital's city screen"],
  ["?", "This help"],
  ["Esc", "Close the screen on top"],
];

addHelp({
  id: "cat-play",
  mode: "cataclysm",
  group: GROUP,
  order: 1,
  title: "How to play",
  body: () => [
    h(
      "ul",
      h("li", "Click a squad to select it. Tab selects the next squad with moves left."),
      h("li", "Right-click a tile to move the selected squad there, or to attack what stands on it. On a touch screen, tap the tile twice."),
      h("li", "Attacking an empire you are at peace with declares war: the game asks first. During a truce the attack is refused."),
      h("li", "Point at a tile for what is on it. With a squad selected, an enemy on a tile next to it also shows a forecast of the attack."),
      h("li", "The selected squad's panel shows its formation. Click a unit, then another cell, to move it there."),
      h("li", "Click one of your cities to open it and choose what it builds."),
      h("li", "End turn (Enter) ends the turn. The battles fought against you during the other empires' turns open when your turn comes."),
      h("li", "The top bar shows the turn and the turn limit, Gold, the research, Mana, Worldstrain, the Court and the Houses. Each opens its screen."),
    ),
    ...section("How the game ends", cat()?.victory?.lines),
    cat()?.victory?.status ? h("p", cat().victory.status) : null,
  ],
});

addHelp({
  id: "cat-keys",
  mode: "cataclysm",
  group: GROUP,
  order: 2,
  title: "Keys",
  body: () => [h("table.kv", KEYS.map(([k, what]) => h("tr", h("th", k), h("td", what))))],
});

addHelp({
  id: "cat-squads",
  mode: "cataclysm",
  group: GROUP,
  order: 3,
  title: "Squads and supply",
  body: () => [
    list(cat()?.rules_help?.squads),
    h("h4", "Orders"),
    named(cat()?.order_help, [
      ["auto_arrange", "Auto-arrange"],
      ["fortify", "Fortify"],
      ["road", "Road"],
      ["fort", "Fort"],
      ["raid", "Raid"],
      ["explore", "Explore ruin"],
      ["found_city", "Found city"],
      ["disband", "Disband"],
    ]),
    ...section("Supply", cat()?.rules_help?.supply),
  ],
});

addHelp({
  id: "cat-battle",
  mode: "cataclysm",
  group: GROUP,
  order: 4,
  title: "Battles",
  body: () => [
    list(cat()?.rules_help?.battle),
    h("p", "A battle you fight opens on the battle screen, which plays it round by round; Skip goes to its end. The side panel lists the latest battles to watch again."),
  ],
});

addHelp({
  id: "cat-cities",
  mode: "cataclysm",
  group: GROUP,
  order: 5,
  title: "Cities",
  body: () => [
    list(cat()?.rules_help?.cities),
    h("h4", "Sally"),
    named(cat()?.sally_help, [
      ["never", "Never"],
      ["favoured", "Favoured"],
      ["always", "Always"],
    ]),
    cat()?.caravan?.cost !== undefined ? h("p", `A caravan costs ${cat().caravan.cost} Gold to launch; the city screen lists where one can go.`) : null,
  ],
});

addHelp({
  id: "cat-research",
  mode: "cataclysm",
  group: GROUP,
  order: 6,
  title: "Research, creeds, rituals",
  body: () => [
    h("p", "Lore goes to the technology being researched. The eight Foundation technologies open the districts and the first units; each doctrine is a ladder of four more."),
    cat()?.doctrine_rule ? h("p", cat().doctrine_rule) : null,
    h("p", "A technology's card shows its cost, the Worldstrain it adds if no civilization has researched it yet and, when it is locked, what it needs."),
    h("p", "Each creed's card gives what founding or adopting it costs and changes. Each ritual's card gives its Mana cost and its change to Worldstrain."),
  ],
});

addHelp({
  id: "cat-court",
  mode: "cataclysm",
  group: GROUP,
  order: 7,
  title: "Great People",
  body: () => {
    const r = cat()?.court_rules ?? {};
    return [
      r.arrival ? h("p", r.arrival) : null,
      cat()?.renown?.sources ? h("p", cat().renown.sources) : null,
      h("h4", "The four choices"),
      named(r, [
        ["secret", "Discover the Secret Tech"],
        ["burst", "Tempo burst"],
        ["hero", "Commission a hero"],
        ["lord", "Appoint a Lord"],
      ]),
    ];
  },
});

addHelp({
  id: "cat-houses",
  mode: "cataclysm",
  group: GROUP,
  order: 8,
  title: "Houses",
  body: () => [list(cat()?.rules_help?.houses)],
});

addHelp({
  id: "cat-world",
  mode: "cataclysm",
  group: GROUP,
  order: 9,
  title: "The Worldseal",
  body: () => {
    const d = cat();
    return [
      d ? h("p", `Turn ${d.turn} of ${d.max_turns}.`) : null,
      ...section("Worldstrain", d?.apocalypse?.rules),
      ...section("Lifeboats", d?.apocalypse?.lifeboat_rules),
      ...section("How the game ends", d?.victory?.lines),
      h("h4", "War and peace"),
      list([d?.war_effect, d?.peace_effect, d?.crusade_effect]),
    ];
  },
});
