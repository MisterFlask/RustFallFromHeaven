// Governors (this game's own rules, docs/GOVERNORS.md): the city screen's Governance section and
// the Governors screen (F9).
//
// You rule a few cities directly, the capital among them; every other city is run by a governor
// from your bench, who chooses what it builds. You choose who governs a city. An army's
// requisition interrupts a governed city as any city (you pick the cities on the Armies screen,
// armies.js). The city query's `governance` and the `governors` query list the cities and the
// bench; the buttons send `govern_direct`, `govern_hand` and `govern_assign`, which the server
// checks. Cultural channels are the player's alone (culture.js): a governor never sets one.

import * as api from "./api.js";
import { dname, info, turns } from "./state.js";
import { h, errorText } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { addHelp } from "./help.js";

// ---- the rules, in the help display (help.js; the Governors screen's (i) opens it) ------------

/** The numbers of the rules as the last `governors` query gave them (null before one). */
let rules = null;

addHelp({
  id: "governors",
  group: "Game options",
  title: "Governors",
  body: () => [
    h("p", "You rule a few cities directly and a governor from your bench runs every other one: for a governed city you choose only who governs it."),
    h("h4", "Direct rule"),
    h(
      "ul",
      h("li", `You rule ${rules ? `up to ${rules.slots} cities` : "a few cities"} directly, the capital always among them; they play as cities always have. A new city takes a free direct slot if there is one, else it is governed.`),
      h("li", `You may rule a governed city directly in place of a directly ruled one (or in a free slot), or hand a directly ruled city to a governor. A city whose rule changed keeps it for ${rules ? turns(rules.cooldown) : "some turns"}.`),
      h("li", "World wonders, palaces, rituals and settlers are built only in directly ruled cities."),
    ),
    h("h4", "Governed cities"),
    h(
      "ul",
      h("li", "The governor chooses what the city builds and which yields its citizens favour (its profile); its traits add their bonuses."),
      h("li", "An army's requisition goes first, as in any city: you pick the cities on the Armies screen [F5]."),
      h("li", "A city with no governor of its own is run by an interim governor, with no leanings and no traits, until a candidate appears."),
    ),
    h("h4", "The bench"),
    h(
      "ul",
      h("li", "Your governors without a city. Candidates appear now and then while posts are vacant or the bench is short; a newly governed city takes the one that suits it best."),
      h("li", `Replacing a governor costs the city ${rules ? `${rules.penalty}% of its production and commerce for ${turns(rules.penalty_turns)}` : "part of its production and commerce for a few turns"}; the replaced governor returns to the bench. Giving an interim city a governor costs nothing.`),
    ),
  ],
});

/** "Steward · Frugal, Beloved" with the profile's and traits' help as the title. */
function governorLabel(gv) {
  const traits = gv.traits.map((t) => dname("GovernorTraitType", t));
  return `${gv.name}, ${dname("GovernorProfileType", gv.profile)}${traits.length ? ` (${traits.join(", ")})` : ""}`;
}

function governorTitle(gv) {
  const lines = [`${dname("GovernorProfileType", gv.profile)}: ${info("GovernorProfileType", gv.profile)?.help ?? ""}`];
  for (const t of gv.traits) lines.push(`${dname("GovernorTraitType", t)}: ${info("GovernorTraitType", t)?.help ?? ""}`);
  return lines.join("\n");
}

/** "requisitioned by the 1st Army" for a city some armies requisition from, or null. */
function requisitionedBy(c) {
  const by = c.requisitioned_by ?? [];
  return by.length ? h("span.dim", { title: "While an army is short of its target size, the city trains a unit for it first (the Armies screen [F5])" }, ` · requisitioned by the ${by.join(", the ")}`) : null;
}

/** The bench as a selector: assign a candidate to city `c` (`bench` with `match` when known). */
function replaceSelect(c, bench, act) {
  if (!bench.length) return h("span.dim", "the bench is empty");
  return h(
    "select.governor-assign",
    {
      "aria-label": c.governor ? "Replace the governor" : "Assign a governor",
      title: c.governor ? "Replace the governor (the city loses some production and commerce for a few turns)" : "Assign a governor from the bench",
      onchange: (ev) => {
        if (ev.target.value !== "") act({ c: "govern_assign", city: c.city, governor: Number(ev.target.value) });
      },
    },
    h("option", { value: "", selected: true }, c.governor ? "Replace with…" : "Assign…"),
    bench.map((b) => h("option", { value: String(b.id), title: governorTitle(b) }, `${governorLabel(b)}${b.match !== undefined ? ` · fit ${b.match}` : ""}`)),
  );
}

/** The city screen's Governance section (null without governors). `reload` re-reads the screen. */
export function governanceView(c, reload) {
  const k = c.governance;
  if (!k) return null;
  // Re-read either way: a refused command leaves the selector showing the city's state, not the
  // click.
  const act = async (cmd) => {
    await run(cmd);
    reload();
  };
  const wait = k.cooldown ? ` (its rule may change again in ${turns(k.cooldown)})` : "";
  if (!k.governed) {
    return h(
      "div.section.governance",
      h("h4", "Governance"),
      h("p", k.capital ? { title: "The capital is always ruled directly." } : {}, `Ruled directly (${k.direct_count} of ${k.slots} cities).`),
      k.capital
        ? null
        : h("button.small", { disabled: !k.ready, title: k.ready ? "A governor will run this city and choose what it builds; its slot is free for another city" : `Not yet${wait}`, onclick: () => act({ c: "govern_hand", city: k.city }) }, "Hand to a governor"),
    );
  }
  const free = k.direct_count < k.slots;
  const direct = !k.ready
    ? h("span.dim", ` Rule directly: not yet${wait}.`)
    : free
      ? h("button.small", { onclick: () => act({ c: "govern_direct", city: k.city, instead_of: null }), title: "Rule this city directly: you choose what it builds" }, "Rule directly")
      : k.swappable.length
        ? h(
            "select.governor-swap",
            { "aria-label": "Rule directly instead of", onchange: (ev) => ev.target.value !== "" && act({ c: "govern_direct", city: k.city, instead_of: Number(ev.target.value) }) },
            h("option", { value: "", selected: true }, "Rule directly instead of…"),
            k.swappable.map((o) => h("option", { value: String(o.city) }, o.name)),
          )
        : h("span.dim", " No direct slot can be swapped now.");
  return h(
    "div.section.governance",
    h("h4", "Governance"),
    h(
      "p",
      k.governor ? h("span", { title: governorTitle(k.governor) }, `Governed by ${governorLabel(k.governor)}`) : h("span.warn", { title: "No leanings, no traits: assign a governor from the bench" }, "An interim governor runs the city"),
      k.penalty_turns ? h("span.warn", { title: "The cost of replacing a governor" }, ` · new governor: less production and commerce for ${turns(k.penalty_turns)}`) : null,
    ),
    h("div.row", replaceSelect(k, k.bench, act)),
    h("div.row", direct),
  );
}

/** The Governors screen (F9): every city with its rule and governor, and the bench. */
export async function openGovernors({ openCity } = {}) {
  const m = openModal({ title: "Governors", body: [h("p.dim", "…")], tag: "governors", wide: true, buttons: [{ label: "Close", value: null }], help: "governors" });
  const load = async () => {
    let r;
    try {
      r = await api.query("governors");
    } catch (e) {
      m.setBody(h("p.warn", errorText(e)));
      return;
    }
    if (m.closed) return;
    const act = async (cmd) => {
      await run(cmd);
      load();
    };
    const direct = r.cities.filter((c) => !c.governed);
    const rows = r.cities.map((c) =>
      h(
        "tr",
        { "data-city": c.city },
        h("td", h("button.link", { onclick: () => (m.close(null), openCity?.(c.city)), title: "Open the city" }, c.name), c.capital ? h("span.dim", " (capital)") : null, requisitionedBy(c)),
        h("td", c.governed ? "governed" : "direct", c.cooldown ? h("span.dim", { title: `Its rule may change again in ${turns(c.cooldown)}` }, ` · ${turns(c.cooldown)}`) : null),
        h(
          "td",
          !c.governed
            ? h("span.dim", "—")
            : c.governor
              ? h("span", { title: governorTitle(c.governor) }, governorLabel(c.governor))
              : h("span.warn", "interim"),
          c.penalty_turns ? h("span.warn", ` · −${r.penalty}% for ${turns(c.penalty_turns)}`) : null,
        ),
        h("td", c.governed ? h("div.governor-controls", replaceSelect(c, r.bench, act)) : h("span.dim", "—")),
        h("td.num", `${c.production}`),
      ),
    );
    rules = { slots: r.slots, cooldown: r.cooldown, penalty: r.penalty, penalty_turns: r.penalty_turns };
    m.setBody(
      h("p.dim", `You rule ${direct.length} of ${r.slots} cities directly.`),
      h("table.governors", h("thead", h("tr", h("th", "City"), h("th", "Rule"), h("th", "Governor"), h("th", "Replace"), h("th", { title: "Production" }, "Prod."))), h("tbody", rows)),
      h("h4", `The bench (${r.bench.length})`),
      r.bench.length ? h("ul.bench", r.bench.map((b) => h("li", { title: governorTitle(b) }, governorLabel(b)))) : h("p.dim", { title: "Candidates appear now and then while posts are vacant or the bench is short." }, "Nobody yet."),
    );
  };
  await load();
  return m.done;
}
