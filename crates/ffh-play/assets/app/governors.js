// Governors (this game's own rules, docs/GOVERNORS.md): the city screen's Governance section and
// the Governors screen (F9).
//
// You rule a few cities directly, the capital among them; every other city is run by a governor
// from your bench, who chooses what it builds. You choose who governs a city and whether it
// develops itself or feeds one of your armies. The city query's `governance` and the `governors`
// query list the cities, the bench and the armies; the buttons send `govern_direct`,
// `govern_hand`, `govern_assign` and `govern_feed`, which the server checks. Cultural channels are
// the player's alone (culture.js): a governor never sets one.

import * as api from "./api.js";
import { dname, info, turns } from "./state.js";
import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";

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

/** The mode selector of governed city `c` (develop, or feed one of `armies`). */
function modeSelect(c, armies, act) {
  return h(
    "select.governor-mode",
    {
      "aria-label": "Mode",
      title: "Develop: buildings, workers and defenders. Feed: units for the army, sent to it, until it reaches its target size.",
      onchange: (ev) => act({ c: "govern_feed", city: c.city, army: ev.target.value === "" ? null : Number(ev.target.value) }),
    },
    h("option", { value: "", selected: c.feed_army === null || c.feed_army === undefined }, "Develop"),
    armies.map((a) => h("option", { value: String(a.id), selected: c.feed_army === a.id }, `Feed the ${a.name} (${a.units}${a.target_size ? ` of ${a.target_size}` : ""})`)),
  );
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
      h("p", `Ruled directly (${k.direct_count} of ${k.slots} cities)${k.capital ? ": the capital always is." : "."}`),
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
    h("div.row", modeSelect(k, k.armies, act), replaceSelect(k, k.bench, act)),
    h("div.row", direct),
  );
}

/** The Governors screen (F9): every city with its rule, governor and mode, and the bench. */
export async function openGovernors({ openCity } = {}) {
  const m = openModal({ title: "Governors", body: [h("p.dim", "…")], tag: "governors", wide: true, buttons: [{ label: "Close", value: null }] });
  const load = async () => {
    let r;
    try {
      r = await api.query("governors");
    } catch (e) {
      m.setBody(h("p.warn", e.detail ?? e.message));
      return;
    }
    if (m.closed) return;
    if (!r.on) {
      m.setBody(h("p.dim", "This game has no governors (the Governors option is off)."));
      return;
    }
    const act = async (cmd) => {
      await run(cmd);
      load();
    };
    const direct = r.cities.filter((c) => !c.governed);
    const rows = r.cities.map((c) =>
      h(
        "tr",
        { "data-city": c.city },
        h("td", h("button.link", { onclick: () => (m.close(null), openCity?.(c.city)), title: "Open the city" }, c.name), c.capital ? h("span.dim", " (capital)") : null),
        h("td", c.governed ? "governed" : "direct", c.cooldown ? h("span.dim", ` · ${turns(c.cooldown)}`) : null),
        h(
          "td",
          !c.governed
            ? h("span.dim", "—")
            : c.governor
              ? h("span", { title: governorTitle(c.governor) }, governorLabel(c.governor))
              : h("span.warn", "interim"),
          c.penalty_turns ? h("span.warn", ` · −${r.penalty}% for ${turns(c.penalty_turns)}`) : null,
        ),
        // The mode and the replacement stacked: the table fits the dialog with the wide pixel font.
        h("td", c.governed ? h("div.governor-controls", modeSelect(c, r.armies, act), replaceSelect(c, r.bench, act)) : h("span.dim", "—")),
        h("td.num", `${c.production}`),
      ),
    );
    m.setBody(
      h(
        "p.dim",
        `You rule ${direct.length} of ${r.slots} cities directly (the capital always); their status may change again ${turns(r.cooldown)} after a change. A governor chooses what its city builds: it develops, or feeds an army until the army reaches its target size. World wonders, rituals and settlers are built only in directly ruled cities. Replacing a governor costs the city ${r.penalty}% of its production and commerce for ${turns(r.penalty_turns)}.`,
      ),
      h("table.governors", h("thead", h("tr", h("th", "City"), h("th", "Rule"), h("th", "Governor"), h("th", "Mode"), h("th", { title: "Production" }, "Prod."))), h("tbody", rows)),
      h("h4", `The bench (${r.bench.length})`),
      r.bench.length ? h("ul.bench", r.bench.map((b) => h("li", { title: governorTitle(b) }, governorLabel(b)))) : h("p.dim", "Nobody yet: candidates appear now and then while posts are vacant or the bench is short."),
    );
  };
  await load();
  return m.done;
}
