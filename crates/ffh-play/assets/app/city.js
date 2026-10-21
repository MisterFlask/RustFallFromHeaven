// The city screen (design §5.1 "Cities", §7 P6): the plots the city works (click to work a plot
// or stop working it), the specialists with their +/- buttons, food, production, commerce,
// happiness and health with their parts, culture, great people, buildings, religions, the
// garrison; hurry, draft, emphasis, automated citizens and production; the production queue
// and the chooser (units, buildings, projects (FfH rituals) and processes with cost and turns):
// click replaces the current production, Shift-click appends, Ctrl-click inserts at the front
// (C++ `CvNetPushOrder`'s flags), ✕ removes an order while the queue holds several.
//
// Every button sends what the Civ4 city screen sends (`do_task`, `push_order`, `pop_order`), and
// the server says which are possible (the `city` query: `plus`/`minus` per specialist,
// `can_work` per plot, `enabled` per hurry and buildable item).

import * as api from "./api.js";
import { S, dname, orderName, info, turns, me, emit, tidx, ORDER_KINDS } from "./state.js";
import { h, signed, setText } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { render as markup, plain, YIELD_ICONS, COMMERCE_ICONS } from "./markup.js";
import { YIELD_COLORS } from "./tiles.js";
import { look, uiIcon, typeIcon, barEl, pix } from "./look.js";
import { focusOn, isoSkin, queueIcon, refreshCityPanel } from "./isoui.js";
import { infoTip } from "./infotips.js";

const GROUPS = [
  [0, "Units"],
  [1, "Buildings"],
  [2, "Projects and rituals"],
  [3, "Processes"],
];

/** The human's cities in the game's order (C++ city loop: creation order, here by id). */
export function ownCities() {
  return [...S.cities.values()].filter((c) => c.owner === me()?.player).sort((a, b) => a.id - b.id);
}

/** Whether the human owns city `id`. */
export function ownCity(id) {
  return S.cities.get(id)?.owner === me()?.player;
}

/** The open city screen's previous / next (`go`), for Home / End (C++ CONTROL_NEXTCITY /
 *  PREVCITY with the city screen up: `cycleCities`, the screen moves to that city); null while
 *  no city screen is open, or the screen is a production popup (it has no previous / next). */
let screenGo = null;

/** Home / End in the city screen: moves it to the next (`step` 1) or previous (-1) city. Whether
 *  a city screen took the key (it is the top dialog). */
export function stepCityScreen(step, topTag) {
  if (!screenGo || topTag !== "city") return false;
  screenGo(step);
  return true;
}

/** Opens city `id`'s screen. With `popup` (an idle city's chooser at a control point, design
 *  §2.3) the dialog offers "Later" and closes once production is chosen. Resolves with "chosen",
 *  "later" or null (closed). */
export async function openCity(id, { popup = false } = {}) {
  focusOn(id);
  let cityId = id;
  let result = null;
  const title = () => {
    const city = S.cities.get(cityId);
    return popup ? `What should ${city?.name ?? "the city"} build?` : (city?.name ?? `City ${cityId}`);
  };
  // With the isometric map's interface the screen is Civ4's: the map frames the city and its plots
  // are worked by clicking them there (MapView.cityOverlay), the screen's parts in panels at the
  // sides. A production popup stays a dialog.
  const overlay = isoSkin() && !popup;
  const m = openModal({ title: title(), body: [h("p.dim", "…")], tag: "city", wide: true, className: overlay ? "city-screen city-overlay" : "city-screen", buttons: [] });
  // A refused task changes nothing: the screen is read again either way (a toggle shows the
  // city's state, not the click).
  const task = async (t, data1 = -1, data2 = -1, option = false) => {
    await run({ c: "do_task", city: cityId, task: tidx("TaskType", t), data1, data2, option, alt: false, shift: false, ctrl: false });
    load();
  };
  const choose = async (order, type, ev) => {
    const reply = await run({ c: "push_order", city: cityId, order, data: type, alt: false, shift: !!ev.shiftKey, ctrl: !!(ev.ctrlKey || ev.metaKey) });
    if (!reply) return;
    if (popup && !ev.shiftKey) {
      result = "chosen";
      m.close("chosen");
    } else load();
  };
  const remove = async (index) => {
    if (await run({ c: "pop_order", city: cityId, index })) load();
  };
  const go = (step) => {
    const list = ownCities();
    const i = list.findIndex((c) => c.id === cityId);
    if (list.length < 2 || i < 0) return;
    cityId = list[(i + step + list.length) % list.length].id;
    setText(m.el.querySelector(".modal-head h2"), title());
    emit("center", S.cities.get(cityId).x, S.cities.get(cityId).y);
    load();
  };
  let loading = 0;
  const load = async () => {
    const n = ++loading;
    let c;
    try {
      c = await api.query("city", { city: cityId });
    } catch (e) {
      m.setBody(h("p.warn", e.detail ?? e.message));
      return;
    }
    if (m.closed || n !== loading) return;
    m.el.dataset.city = String(cityId);
    m.setBody(screen(c, { task, choose, remove, go, popup, overlay }));
    if (overlay) {
      const mine = me()?.player;
      // A click works the plot of the city these plots are of (Previous / Next may have moved on).
      emit("city-overlay", { id: c.id, x: c.x, y: c.y, owner: c.owner, plots: c.plots, onPlot: (p) => cityId === c.id && workable(p, mine) && task("TASK_CHANGE_WORKING_PLOT", p.i), tipOf: (p) => plotTip(p, mine) });
    }
  };
  m.setButtons(
    popup
      ? [{ label: "Later", value: "later", title: "Leave the city idle for now (the end of turn reminds you)" }]
      : [
          { label: "‹ Previous city", onClick: () => go(-1), title: "The previous city" },
          { label: "Next city ›", onClick: () => go(1), title: "The next city" },
          { label: "Close", value: null },
        ],
  );
  load();
  if (!popup) screenGo = go;
  const v = await m.done;
  if (overlay) emit("city-overlay", null);
  refreshCityPanel();
  if (screenGo === go) screenGo = null;
  return result ?? v;
}

// ---------------------------------------------------------------------------------------------

function yieldNumbers(y) {
  if (!y) return h("span.dim", "?");
  // With the art: each yield's 8 px icon and its number (docs/ART.md 11.5).
  if (look.art || isoSkin()) return h("span.yields", y.map((v, j) => (v > 0 ? h("span.yield-n", { title: dname("YieldType", j) }, uiIcon("yield", j, 8), pix(String(v))) : null)));
  return h("span.yields", y.map((v, j) => (v > 0 ? h("span", { style: { color: YIELD_COLORS[j] } }, String(v)) : null)));
}

/** The isometric interface's icon of something to build (a unit's figure, a building's ...). */
function buildIcon(order, type) {
  const el = queueIcon({ order, type });
  el.classList.add("build-icon");
  return el;
}

/** Whether a click on city plot `p` does something: the centre (automates the citizens) or a
 *  revealed plot of `mine`. */
function workable(p, mine) {
  return (p.i === 0 || p.owner === mine) && !!p.revealed;
}

/** What a click on city plot `p` does (the grid's tooltip; the map's in the overlay). */
function plotTip(p, mine) {
  const ours = p.owner === mine;
  if (p.i === 0) return "The city's own plot (click: automate the citizens)";
  if (!p.revealed) return "Unexplored";
  if (p.worked) return "Worked: click to stop working it (the citizen becomes a specialist)";
  if (p.worked_by !== null && p.worked_by !== undefined) return `Worked by ${S.cities.get(p.worked_by)?.name ?? "another city"}${ours ? ": click to take it" : ""}`;
  if (!ours) return "Not your plot";
  return p.can_work ? "Click: work this plot" : "Not workable now";
}

/** The city's plots as a grid north up, centred on the city. */
function plotGrid(c, task) {
  const w = S.map?.w ?? 0;
  const wrap = !!S.map?.wrap_x;
  const rel = (p) => {
    let dx = p.x - c.x;
    if (wrap && w) {
      if (dx > w / 2) dx -= w;
      if (dx < -w / 2) dx += w;
    }
    return { dx, dy: p.y - c.y };
  };
  let r = 1;
  for (const p of c.plots) {
    const { dx, dy } = rel(p);
    r = Math.max(r, Math.abs(dx), Math.abs(dy));
  }
  const cells = new Map(c.plots.map((p) => [`${rel(p).dx},${rel(p).dy}`, p]));
  const mine = me()?.player;
  const rows = [];
  for (let dy = r; dy >= -r; dy--) {
    for (let dx = -r; dx <= r; dx++) {
      const p = cells.get(`${dx},${dy}`);
      if (!p) {
        rows.push(h("div.cplot.none"));
        continue;
      }
      const centre = p.i === 0;
      const ours = p.owner === mine;
      const cls = [centre ? "centre" : "", p.worked ? "worked" : "", p.worked_by !== null && p.worked_by !== undefined ? "other" : "", !p.revealed ? "hidden" : "", ours ? "" : "foreign"].filter(Boolean).join(" ");
      rows.push(
        h(
          "button.cplot",
          {
            class: cls,
            title: plotTip(p, mine),
            "data-i": p.i,
            disabled: !workable(p, mine),
            onclick: () => task("TASK_CHANGE_WORKING_PLOT", p.i),
          },
          centre ? h("span.ccity", "★") : null,
          p.revealed ? yieldNumbers(p.yields) : null,
        ),
      );
    }
  }
  return h("div.cgrid", { style: { gridTemplateColumns: `repeat(${2 * r + 1}, 1fr)` } }, rows);
}

/** What one specialist of type `s` adds: its yields, commerces, great people and experience,
 *  as a small line (markup icons). */
function specialistYields(s) {
  const e = info("SpecialistType", s);
  if (!e) return null;
  const bits = [];
  (e.yields ?? []).forEach((n, j) => n && bits.push(`${signed(n)}${YIELD_ICONS[j]}`));
  (e.commerces ?? []).forEach((n, j) => n && bits.push(`${signed(n)}${COMMERCE_ICONS[j]}`));
  if (e.great_people) bits.push(`${signed(e.great_people)}[ICON_GREATPEOPLE]`);
  if (e.experience) bits.push(`${signed(e.experience)} XP to new units`);
  return bits.length ? h("span.spec-yields", " ", markup(bits.join(" "))) : null;
}

function specialistsView(c, task) {
  const rows = c.specialists
    .filter((s) => s.count > 0 || s.plus || s.minus || s.free > 0)
    .map((s) =>
      h(
        "div.spec",
        { "data-specialist": s.specialist },
        typeIcon("specialist", s.specialist, 16),
        h("span.spec-name", { tip: infoTip("SpecialistType", s.specialist, { extra: specialistYields(s.specialist) }) }, dname("SpecialistType", s.specialist), specialistYields(s.specialist)),
        h("span.dim", `${s.count}${s.forced ? ` (${s.forced} set)` : ""}${s.free ? `, ${s.free} free` : ""}${s.max ? ` / ${s.max}` : ""}`),
        h("button.small.minus", { disabled: !s.minus, title: "One less", onclick: () => task("TASK_CHANGE_SPECIALIST", s.specialist, -1) }, "−"),
        h("button.small.plus", { disabled: !s.plus, title: "One more", onclick: () => task("TASK_CHANGE_SPECIALIST", s.specialist, 1) }, "+"),
      ),
    );
  return h(
    "div.section",
    h("h4", "Citizens"),
    h(
      "label.toggle",
      h("input", { type: "checkbox", checked: c.citizens_automated, id: "city-auto-citizens", onchange: (ev) => task("TASK_SET_AUTOMATED_CITIZENS", -1, -1, ev.target.checked) }),
      " Automate citizens (the governor picks plots and specialists)",
    ),
    c.extra_population > 0 ? h("div.dim", `${c.extra_population} citizen${c.extra_population > 1 ? "s" : ""} without work`) : null,
    rows.length ? h("div.specs", rows) : h("p.dim", "No specialists."),
  );
}

function kv(rows) {
  return h("table.kv", rows.filter(([, v]) => v !== null && v !== undefined && v !== false).map(([k, v]) => h("tr", h("th", k), h("td", v instanceof Node ? v : String(v)))));
}


/** A city's happiness or health by its parts (the `city` query's `*_parts`: each list sums to its
 *  total): "+4 difficulty, +1 buildings; unhappy: 3 population". */
function partsLine(good, bad, badWord, notes = []) {
  const list = (ps) => (ps ?? []).map(([label, n]) => `${n} ${label}`).join(", ");
  const text = [good?.length ? list(good) : null, bad?.length ? `${badWord}: ${list(bad)}` : null, ...notes].filter(Boolean).join("; ");
  return h("span.dim.tip-parts", text || "-");
}

function statsView(c) {
  const y = (i) => c.yields.find((v) => v.yield === i) ?? { rate: 0 };
  const commerce = c.commerces.filter((k) => k.rate).map((k) => `${dname("CommerceType", k.commerce)} ${k.rate}`).join(", ");
  const f = c.food;
  const hp = c.happiness;
  const he = c.health;
  return h(
    "div.section",
    h("h4", "The city"),
    kv([
      ["Size", `${c.size}${c.capital ? " (capital)" : ""}${c.disorder ? " — in disorder" : ""}${c.occupation ? ` — occupied ${c.occupation}` : ""}`],
      ["Food", `${f.stored}/${f.threshold} (${signed(f.difference)}; eats ${f.consumption}) ${f.difference > 0 ? `grows in ${turns(f.turns)}` : f.difference < 0 ? "starving" : "stagnant"}`],
      ["Production", `${y(1).rate} per turn${y(1).modifier ? ` (+${y(1).modifier}%)` : ""}`],
      ["Commerce", `${y(2).rate}: ${commerce || "-"}`],
      ["Happy", `${hp.happy} happy, ${hp.unhappy} unhappy${hp.angry ? ` (${hp.angry} angry: they do not work)` : ""}`],
      ["", partsLine(hp.happy_parts, hp.unhappy_parts, "unhappy", [hp.hurry_anger_timer ? `hurry anger for ${turns(hp.hurry_anger_timer)}` : null, hp.conscript_anger_timer ? `draft anger for ${turns(hp.conscript_anger_timer)}` : null])],
      ["Health", `${he.good} healthy, ${he.bad} unhealthy${he.bad > he.good ? ` (${he.bad - he.good} short: −${he.bad - he.good} food a turn)` : ""}`],
      ["", partsLine(he.good_parts, he.bad_parts, "unhealthy")],
      ["Culture", `${c.culture}/${c.culture_threshold}${c.culture_level !== null ? ` (${dname("CultureLevelType", c.culture_level)})` : ""}`],
      ["Great people", `${c.great_people.progress} (+${c.great_people.rate} per turn)`],
      ["Defence", `${c.defense}%${c.defense_damage ? ` (damaged ${c.defense_damage})` : ""}`],
      ["Maintenance", `${c.maintenance} gold`],
      ["Crime", c.crime || null],
      ["Trade routes", c.trade_routes || null],
      ["Founded", `turn ${c.founded}`],
    ]),
  );
}

function buildingsView(c) {
  const b = c.buildings.map((x) => h("span.chip", { tip: infoTip("BuildingType", x.building) }, typeIcon("building", x.building, 8), dname("BuildingType", x.building), x.free && !x.real ? h("span.dim", " (free)") : null));
  const r = c.religions.map((x) => h("span.chip", typeIcon("religion", x.religion, 8), dname("ReligionType", x.religion), x.holy ? h("span.good", " ★ holy city") : null));
  const units = c.garrison.map((u) => h("span.chip", `${dname("UnitType", u.type)}${u.owner !== me()?.player ? ` (${u.owner})` : ""}`));
  return [
    h("div.section", h("h4", `Buildings (${b.length})`), b.length ? h("div.chips", b) : h("p.dim", "None.")),
    r.length ? h("div.section", h("h4", "Religions"), h("div.chips", r)) : null,
    h("div.section", h("h4", `Garrison (${units.length})`), units.length ? h("div.chips", units) : h("p.dim", "Nobody.")),
  ];
}

function manageView(c, task) {
  const hurries = c.hurries.map((x) =>
    h(
      "button.small",
      { disabled: !x.enabled, "data-hurry": x.hurry, title: dname("HurryType", x.hurry), onclick: () => task("TASK_HURRY", x.hurry) },
      `${dname("HurryType", x.hurry)}${x.gold ? ` (${x.gold} gold)` : ""}${x.population ? ` (${x.population} pop.)` : ""}`,
    ),
  );
  const emphasize = c.emphasize.map((on, e) =>
    h("label.toggle", { title: plain(info("EmphasizeType", e)?.help ?? "") }, h("input", { type: "checkbox", checked: on, "data-emphasize": e, onchange: (ev) => task("TASK_SET_EMPHASIZE", e, -1, ev.target.checked) }), ` ${dname("EmphasizeType", e)}`),
  );
  return h(
    "div.section",
    h("h4", "Orders"),
    h(
      "div.row",
      hurries,
      c.can_conscript ? h("button.small", { onclick: () => task("TASK_CONSCRIPT"), title: "Draft a unit (costs population and anger)" }, "Draft") : null,
    ),
    h("label.toggle", h("input", { type: "checkbox", checked: c.production.automated, id: "city-auto-production", onchange: (ev) => task("TASK_SET_AUTOMATED_PRODUCTION", -1, -1, ev.target.checked) }), " Automate production (the governor chooses)"),
    emphasize.length ? h("details", h("summary", "Emphasis"), h("div.checks", emphasize)) : null,
  );
}

function productionView(c, choose, remove) {
  const prod = c.production;
  const queue = h(
    "ol.queue",
    c.queue.length
      ? c.queue.map((o, i) =>
          h(
            "li",
            h("span", orderName(o.order, o.type)),
            h("span.dim", o.turns !== null ? ` ${turns(o.turns)}` : ""),
            // The exe pops an order only while the queue holds more than one (CvGameInterface
            // `getOrderQueueLength() > 1`); the only order is replaced by choosing another.
            c.queue.length > 1 ? h("button.small", { title: "Remove from the queue", "aria-label": "Remove", "data-index": i, onclick: () => remove(i) }, "✕") : null,
          ),
        )
      : h("li.dim", "Nothing: choose below."),
  );
  const lists = GROUPS.map(([order, label]) => {
    const items = c.buildable.filter((b) => b.order === order);
    if (!items.length) return null;
    return h(
      "div.section",
      h("h4", label),
      h(
        "div.build-grid",
        items.map((b) => {
          const kind = ORDER_KINDS[order];
          return h(
            "button.build",
            {
              disabled: !b.enabled,
              "data-order": order,
              "data-type": b.type,
              tip: infoTip(kind, b.type, { hint: b.enabled ? "Click: build now. Shift: add to the queue. Ctrl: put first." : "Not possible now." }),
              onclick: (ev) => choose(order, b.type, ev),
            },
            isoSkin() ? buildIcon(order, b.type) : null,
            h("span", dname(kind, b.type)),
            h("span.dim", b.turns !== null ? `${turns(b.turns)} · ${b.cost}` : ""),
          );
        }),
      ),
    );
  });
  return [
    h(
      "div.section",
      h("h4", "Production"),
      h(
        "div.prod-line",
        prod.current && (look.art || isoSkin()) ? uiIcon("yield", 1, 16) : null,
        prod.current ? `${prod.stored}/${prod.needed ?? "-"} (+${prod.rate} per turn)` : h("span.warn", "Nothing is being built."),
        prod.current && prod.needed ? barEl(prod.stored / prod.needed, "production", { w: 50, next: (prod.stored + prod.rate) / prod.needed }) : null,
      ),
      queue,
    ),
    ...lists,
  ];
}

function screen(c, { task, choose, remove, popup, overlay }) {
  const f = c.food;
  const head = h(
    "div.city-summary",
    h("b", `Size ${c.size}`),
    c.capital ? " · capital" : "",
    c.disorder ? h("span.warn", " · in disorder") : "",
    c.occupation ? h("span.warn", ` · occupied ${c.occupation}`) : "",
    ` · food ${signed(c.food.difference)} · production ${c.production.rate}`,
    (look.art || isoSkin()) && f.threshold > 0 ? h("span.growth", uiIcon("yield", 0, 8), barEl(f.stored / f.threshold, "food", { w: 40, next: (f.stored + f.difference) / f.threshold, title: `${f.stored}/${f.threshold}` })) : null,
  );
  const plots = overlay ? h("p.dim.city-hint", "Click the city's plots on the map to work them; its own plot hands the choice back to the governor.") : plotGrid(c, task);
  const left = h("div.city-col", plots, specialistsView(c, task), statsView(c), buildingsView(c));
  const right = h("div.city-col", manageView(c, task), productionView(c, choose, remove));
  return [head, popup ? h("div.city-cols.popup", right, left) : h("div.city-cols", left, right)];
}
