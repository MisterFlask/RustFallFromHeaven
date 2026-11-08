// The city screen (design §5.1 "Cities", §7 P6): the plots the city works (click to work a plot
// or stop working it), the specialists with their +/- buttons, food, production, commerce,
// happiness and health with their parts, culture, great people, buildings, religions, the
// garrison; hurry, draft, emphasis, automated citizens and production; the production queue
// and the chooser (units, buildings, projects (FfH rituals) and processes with cost and turns):
// click replaces the current production, Shift-click appends, Ctrl-click inserts at the front
// (C++ `CvNetPushOrder`'s flags), ✕ removes an order while the queue holds several (not an
// army's requisition order: "Requisitioned by the 1st Army", released on the Armies screen).
//
// Every button sends what the Civ4 city screen sends (`do_task`, `push_order`, `pop_order`), and
// the server says which are possible (the `city` query: `plus`/`minus` per specialist,
// `can_work` per plot, `enabled` per hurry and buildable item).

import * as api from "./api.js";
import { S, dname, orderName, info, turns, me, emit, tidx, ORDER_KINDS } from "./state.js";
import { h, signed, setText, tapModes, compactScreen } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { YIELD_COLORS } from "./tiles.js";
import { look, uiIcon, typeIcon, barEl, pix } from "./look.js";
import { focusOn, isoSkin, isoIcon, queueIcon, refreshCityPanel } from "./isoui.js";
import { plotState } from "./citywork.js";
import { infoTip, moodTip, roomText } from "./infotips.js";
import { cultureView } from "./culture.js";
import { recruitControls, recruitView, requisitionNote, requisitionTag } from "./armies.js";
import { governanceView } from "./governors.js";

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
  // A phone has no room for the screen's panels around the city: it gets the whole-screen dialog.
  const overlay = isoSkin() && !popup && !compactScreen();
  const m = openModal({ title: title(), body: [h("p.dim", "…")], tag: "city", wide: true, className: overlay ? "city-screen city-overlay" : "city-screen", buttons: [] });
  // A refused task changes nothing: the screen is read again either way (a toggle shows the
  // city's state, not the click).
  const task = async (t, data1 = -1, data2 = -1, option = false) => {
    await run({ c: "do_task", city: cityId, task: tidx("TaskType", t), data1, data2, option, alt: false, shift: false, ctrl: false });
    load();
  };
  /** What a tap on a build does on a touch screen, for want of Shift and Ctrl: "now" (a click),
   *  "queue" (Shift: append) or "first" (Ctrl: insert first). Kept while the screen is open. */
  let tapMode = "now";
  const setTapMode = (v) => {
    tapMode = v;
    load();
  };
  const choose = async (order, type, ev) => {
    const shift = !!ev.shiftKey || tapMode === "queue";
    const ctrl = !!(ev.ctrlKey || ev.metaKey) || tapMode === "first";
    const reply = await run({ c: "push_order", city: cityId, order, data: type, alt: false, shift, ctrl });
    if (!reply) return;
    if (popup && !shift) {
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
    m.setBody(screen(c, { task, choose, remove, go, popup, overlay, tap: [tapMode, setTapMode], reload: load }));
    if (overlay) {
      const mine = me()?.player;
      // A click works the plot of the city these plots are of (Previous / Next may have moved on).
      emit("city-overlay", { id: c.id, x: c.x, y: c.y, owner: c.owner, plots: c.plots, onPlot: (p) => cityId === c.id && workable(p, mine) && task("TASK_CHANGE_WORKING_PLOT", p.i), tipOf: (p) => plotTipNodes(p, mine, c) });
    }
  };
  m.setButtons(
    popup
      ? [{ label: "Later", value: "later", title: "Leave the city idle for now (the end of turn reminds you)" }]
      : [
          { label: "Previous city", onClick: () => go(-1), title: "The previous city [End]" },
          { label: "Next city", onClick: () => go(1), title: "The next city [Home]" },
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

// --- the screen's icons ----------------------------------------------------------------------

/** The art's yield and commerce icons outside the isometric interface (`uiIcon`). */
const ART_ICON = { food: ["yield", 0], production: ["yield", 1], commerce: ["yield", 2], gold: ["commerce", 0], research: ["commerce", 1], culture: ["commerce", 2], espionage: ["commerce", 3] };
const COMMERCE_NAMES = ["gold", "research", "culture", "espionage"];

/** Short stand-ins for the icons the art's interface lacks (the flat debug renderer's). */
const ICON_WORDS = { population: "●", happy: "☺", unhappy: "☹", health: "♥", unhealthy: "☠", greatpeople: "GP", defense: "⛨", cities: "bld", religion: "rel", civics: "emp" };

/** Interface icon `name` (the isometric interface's `ui_<name>_s`, else the art's), 16 CSS px or
 *  32 when `big`; a short stand-in when there is no icon. */
function ic(name, big = false, title = null) {
  const art = ART_ICON[name];
  const el = isoSkin() ? isoIcon(name, { px: big ? 32 : 16, title }) : art ? uiIcon(art[0], art[1], big ? 16 : 8, { title }) : null;
  return el ?? h("span.ic-word", { title }, ICON_WORDS[name] ?? name);
}

/** A citizen token (citywork.js draws the same on the map): the person, or `inner` (a
 *  specialist's icon), in a ring; `kind` "" (works a plot), "spec", "free", "idle" or "angry". */
function token(kind, inner = null) {
  return h(`span.ctoken${kind ? `.${kind}` : ""}`, inner ?? ic("population"));
}

/** A plot's yields as icons, one per point (citywork.js `drawPips`), `max` icons a yield at most. */
function pips(yields, max = 5) {
  const out = [];
  (yields ?? []).forEach((v, j) => {
    if (v <= 0) return;
    const name = ["food", "production", "commerce"][j];
    for (let k = 0; k < Math.min(v, max); k++) out.push(ic(name));
    if (v > max) out.push(h("span.pip-n", String(v)));
  });
  return h("span.pips", out);
}

/** The specialist a citizen taken off a plot becomes (DEFAULT_SPECIALIST), or null. */
function defaultSpecialist() {
  const s = S.data?.defines?.default_specialist ?? -1;
  return s >= 0 ? s : null;
}

// --- citizens ----------------------------------------------------------------------------------

/** The city's people, one token each, where they are: on the plots they work (the map shows the
 *  same tokens), in specialist slots, without work, or angry. A click on a plot's citizen takes
 *  them off it (they become the default specialist); on a specialist, back to the governor's
 *  choice of plot. Then the specialists by type: their slots, filled and open. */
function citizensView(c, task) {
  const worked = c.plots.filter((p) => p.i !== 0 && p.worked);
  const people = [];
  const dflt = defaultSpecialist();
  for (const p of worked) {
    people.push(
      h(
        "button.cperson",
        {
          tip: `Works a plot: ${yieldWords(p.yields)}. Click: take them off it${dflt !== null ? ` (they become a ${dname("SpecialistType", dflt)})` : ""}.`,
          onmouseenter: () => emit("city-hover", { x: p.x, y: p.y }),
          onmouseleave: () => emit("city-hover", null),
          onclick: () => task("TASK_CHANGE_WORKING_PLOT", p.i),
        },
        token(""),
        pips(p.yields, 3),
      ),
    );
  }
  let specialists = 0;
  for (const s of c.specialists) {
    for (let k = 0; k < s.count; k++) {
      specialists++;
      people.push(
        h(
          "button.cperson",
          { disabled: !s.minus, tip: infoTip("SpecialistType", s.specialist, { hint: s.minus ? "Click: back to work a plot." : null }), onclick: () => task("TASK_CHANGE_SPECIALIST", s.specialist, -1) },
          token("spec", typeIcon("specialist", s.specialist, 16)),
          specialistGives(s.specialist),
        ),
      );
    }
  }
  const idle = Math.max(0, c.extra_population);
  for (let k = 0; k < idle; k++) people.push(h("span.cperson", { tip: "Without work: click a plot on the map or an open slot." }, token("idle"), h("span.pips.dim", "idle")));
  const angry = Math.max(0, c.happiness.angry);
  for (let k = 0; k < angry; k++) people.push(h("span.cperson", { tip: "Angry: refuses to work. More happiness brings them back." }, token("angry"), h("span.pips.bad", "angry")));
  const counts = [
    `${worked.length} on plots`,
    specialists ? `${specialists} specialist${specialists > 1 ? "s" : ""}` : null,
    idle ? `${idle} idle` : null,
    angry ? h("span.bad", `${angry} angry`) : null,
  ].filter(Boolean);
  return h(
    "div.section.citizens",
    h(
      "div.sec-head",
      h("h4", "Citizens"),
      h(
        "label.toggle",
        { tip: "The governor picks the plots and specialists. Click a plot or a slot to choose yourself." },
        h("input", { type: "checkbox", checked: c.citizens_automated, id: "city-auto-citizens", onchange: (ev) => task("TASK_SET_AUTOMATED_CITIZENS", -1, -1, ev.target.checked) }),
        " Governor picks",
      ),
    ),
    h("div.cpeople", people),
    h("div.dim.ccounts", counts.flatMap((x, i) => (i ? [" · ", x] : [x]))),
    specialistSlots(c, task),
  );
}

/** "+3 food, +1 commerce". */
function yieldWords(y) {
  return (y ?? []).map((v, j) => (v ? `${signed(v)} ${dname("YieldType", j).toLowerCase()}` : null)).filter(Boolean).join(", ") || "nothing";
}

/** What one specialist of type `s` gives, as icons and numbers (`sign` -1: what leaving costs). */
function specialistGives(s, sign = 1) {
  const e = info("SpecialistType", s);
  if (!e) return null;
  const bits = [];
  (e.yields ?? []).forEach((n, j) => n && bits.push(h("span.give", ic(["food", "production", "commerce"][j]), signed(sign * n))));
  (e.commerces ?? []).forEach((n, j) => n && bits.push(h("span.give", ic(COMMERCE_NAMES[j]), signed(sign * n))));
  if (e.great_people) bits.push(h("span.give", ic("greatpeople"), signed(sign * e.great_people)));
  return bits.length ? h("span.gives", bits) : null;
}

/** The specialists by type: a filled slot per specialist (click: back to a plot), an open slot per
 *  free place (click: a citizen takes it), the free specialists a building gives in their own
 *  colour; the types the city cannot employ yet in one line. */
function specialistSlots(c, task) {
  const rows = [];
  const locked = [];
  for (const s of c.specialists) {
    const unlimited = s.can_add && s.count >= s.max;
    if (!s.count && !s.free && !s.max && !s.can_add) {
      // (The great people who settle in a city are specialists too, but never a slot to open.)
      if (!/^SPECIALIST_GREAT_/.test(info("SpecialistType", s.specialist)?.type ?? "")) locked.push(h("span.locked", { tip: infoTip("SpecialistType", s.specialist, { hint: "No slot here yet: buildings open them." }) }, typeIcon("specialist", s.specialist, 16) ?? dname("SpecialistType", s.specialist)));
      continue;
    }
    const name = dname("SpecialistType", s.specialist);
    const slots = [];
    for (let k = 0; k < s.free; k++) slots.push(h("span.slot.free", { tip: `A free ${name}: a building gives them, not a citizen.` }, typeIcon("specialist", s.specialist, 16)));
    for (let k = 0; k < s.count; k++) slots.push(h("button.slot.full", { disabled: !s.minus, "aria-label": `${name}: back to a plot`, title: s.minus ? `${name}: click to send back to a plot` : name, onclick: () => task("TASK_CHANGE_SPECIALIST", s.specialist, -1) }, typeIcon("specialist", s.specialist, 16)));
    const open = unlimited ? 1 : Math.max(0, s.max - s.count);
    for (let k = 0; k < open; k++) slots.push(h("button.slot.open", { disabled: !s.plus, "aria-label": `Make a citizen a ${name}`, title: s.plus ? `Click: a citizen becomes a ${name}` : `No citizen to spare`, onclick: () => task("TASK_CHANGE_SPECIALIST", s.specialist, 1) }, "+"));
    rows.push(
      h(
        "div.spec",
        { "data-specialist": s.specialist },
        typeIcon("specialist", s.specialist, 16),
        h("span.spec-name", { tip: infoTip("SpecialistType", s.specialist) }, name, specialistGives(s.specialist)),
        h("span.slots", slots),
      ),
    );
  }
  return [
    h("h4.sub", "Specialists"),
    rows.length ? h("div.specs", rows) : h("p.dim", "No specialist slots."),
    locked.length ? h("div.locked-line.dim", locked) : null,
  ];
}

// --- mood, health and the rest -----------------------------------------------------------------

/** How many more citizens the city takes before trouble (`room`), as a badge: green with room,
 *  amber at the limit, red past it. */
function roomBadge(room, tip) {
  const cls = room > 0 ? "good" : room === 0 ? "zero" : "bad";
  return h(`span.room.${cls}`, { tip }, ic(room < 0 ? "unhappy" : "population"), room > 0 ? `+${room}` : String(room));
}

function moodView(c) {
  const hp = c.happiness;
  const he = c.health;
  const happyNotes = [hp.hurry_anger_timer ? `Hurry anger for ${turns(hp.hurry_anger_timer)}` : null, hp.conscript_anger_timer ? `Draft anger for ${turns(hp.conscript_anger_timer)}` : null];
  return h(
    "div.section.mood",
    h("h4", "Mood and health"),
    h(
      "div.mood-row",
      { tip: () => moodTip(c, true, ic, happyNotes) },
      h("span.count", ic("happy"), String(hp.happy)),
      h("span.vs", "vs"),
      h("span.count", ic("unhappy"), String(hp.unhappy)),
      roomBadge(hp.happy - hp.unhappy, roomText(c, true)),
    ),
    h(
      "div.mood-row",
      { tip: () => moodTip(c, false, ic) },
      h("span.count", ic("health"), String(he.good)),
      h("span.vs", "vs"),
      h("span.count", ic("unhealthy"), String(he.bad)),
      roomBadge(he.good - he.bad, roomText(c, false)),
    ),
  );
}

function growthView(c) {
  const ct = c.culture_threshold;
  return h(
    "div.section.city-growth",
    h(
      "div.meter",
      { tip: `Culture ${c.culture}/${ct}${c.culture_level !== null ? ` (${dname("CultureLevelType", c.culture_level)})` : ""}: it grows the city's borders.` },
      ic("culture"),
      ct > 0 ? barEl(c.culture / ct, "culture", { w: 60 }) : null,
      h("span", `${c.culture}/${ct}`),
    ),
    h(
      "div.meter",
      { tip: "Great people points: a great person is born when they reach the empire's threshold." },
      ic("greatpeople"),
      h("span", `${c.great_people.progress} · ${signed(c.great_people.rate)} a turn`),
    ),
    h(
      "div.chips.stats",
      h("span.chip", { tip: c.defense_damage ? `Damaged by ${c.defense_damage}` : "Defence against attackers" }, ic("defense"), `${c.defense}%`),
      // Sieges (docs/SIEGES.md): the walls' Sentinels (HP left each) and a siege.
      c.sentinels?.length
        ? h(
            "span.chip",
            { tip: `Sentinels: the walls fight in the front row of the city's battles (${c.sentinel_max_hp} HP each, grown by its defence). They repair 10% a turn unless the city is besieged; the city falls only once they are breached.` },
            `Sentinels ${c.sentinels.map((p) => (p > 0 ? `${p}%` : "breached")).join(" · ")}`,
          )
        : null,
      c.besieged ? h("span.chip.bad", { tip: `Besieged by ${c.besiegers} enemy unit${c.besiegers === 1 ? "" : "s"} next to the city: its units get no heal from the city and its Sentinels do not repair` }, "Besieged") : null,
      h("span.chip", { tip: "Upkeep: gold a turn" }, ic("gold"), c.maintenance ? `−${c.maintenance}` : "0"),
      c.trade_routes ? h("span.chip", { tip: "Trade routes" }, `Trade ${c.trade_routes}`) : null,
      h("span.chip.dim", `Founded turn ${c.founded}`),
    ),
  );
}

// --- the yields at the top ---------------------------------------------------------------------

/** The city's yields at a glance: size, food (grown less eaten, the growth bar, when it grows),
 *  production (the current build's bar and turns), then where the commerce goes. */
function summaryView(c) {
  const f = c.food;
  const y = (i) => c.yields.find((v) => v.yield === i) ?? { rate: 0 };
  const prod = c.production;
  const foodCls = f.difference > 0 ? "good" : f.difference < 0 ? "bad" : "";
  const grows = f.difference > 0 ? h("span.when", { tip: `Grows to size ${c.size + 1} in ${turns(f.turns)}` }, "→ ", ic("population"), String(c.size + 1), ` in ${turns(f.turns)}`) : f.difference < 0 ? h("span.when.bad", "starving") : h("span.when.dim", "not growing");
  return h(
    "div.city-summary",
    h(
      "div.cs-row",
      h(
        "span.cs-size",
        { tip: `Size ${c.size}${c.capital ? ", the capital" : ""}` },
        ic("population", true),
        h("b", String(c.size)),
        c.capital ? h("span.capital", { title: "Capital" }, "★") : null,
        c.disorder ? h("span.bad", " in disorder") : null,
        c.occupation ? h("span.bad", ` occupied ${c.occupation}`) : null,
      ),
      h(
        "div.cs-gauge",
        { tip: `Food: ${y(0).rate} grown, ${f.consumption} eaten by the citizens; ${f.stored}/${f.threshold} stored` },
        h("div.cs-top", ic("food", true), h(`b.cs-big.${foodCls || "even"}`, signed(f.difference)), h("span.cs-sub", String(y(0).rate), ic("food"), " − ", String(f.consumption), ic("population"))),
        h("div.cs-top", f.threshold > 0 ? barEl(f.stored / f.threshold, "food", { w: 60, next: (f.stored + f.difference) / f.threshold }) : null, grows),
      ),
      h(
        "div.cs-gauge",
        { tip: prod.current ? `Production: ${prod.rate} a turn; ${prod.stored}/${prod.needed ?? "-"}` : "Nothing is being built" },
        h("div.cs-top", ic("production", true), h("b.cs-big.prod", String(prod.rate)), h("span.cs-sub", prod.current ? productionName(c) : h("span.bad", "nothing to build"))),
        prod.current && prod.needed ? h("div.cs-top", barEl(prod.stored / prod.needed, "production", { w: 60, next: (prod.stored + prod.rate) / prod.needed }), h("span.when", c.queue[0]?.turns !== null && c.queue[0]?.turns !== undefined ? `in ${turns(c.queue[0].turns)}` : "")) : null,
      ),
    ),
    commerceView(c),
  );
}

function productionName(c) {
  const o = c.queue[0];
  return o ? orderName(o.order, o.type) : "";
}

/** Where the commerce goes: the city's commerce, split by the empire's rates into a bar, then each
 *  commerce's total with its parts (the split's share, specialists, buildings, religions, the
 *  empire's own, processes, the modifier). */
function commerceView(c) {
  const raw = (c.yields.find((v) => v.yield === 2) ?? { rate: 0 }).rate;
  // (Espionage only when it counts: with espionage off its parts are culture's, server side.)
  const ks = c.commerces.filter((k) => (k.commerce === 3 ? k.rate !== 0 : k.rate || k.rates || k.specialists || k.buildings || k.religions || k.empire || k.process || k.percent > 0));
  const split = ks.filter((k) => k.rates > 0);
  const total = split.reduce((a, k) => a + k.rates, 0);
  const bar = h(
    "div.cs-split",
    h("div.cs-split-pct", split.map((k) => h("span", { style: { flex: `${k.rates} 1 0` } }, `${k.percent}%`))),
    h(
      "div.cs-split-bar",
      split.length
        ? split.map((k) => h(`span.seg.${COMMERCE_NAMES[k.commerce]}`, { style: { flex: `${k.rates} 1 0` }, title: `${dname("CommerceType", k.commerce)}: ${k.rates} (${k.percent}%)` }, ic(COMMERCE_NAMES[k.commerce]), String(k.rates)))
        : h("span.seg.none", "—"),
    ),
  );
  const outs = ks.map((k) => {
    const name = COMMERCE_NAMES[k.commerce];
    const parts = [];
    const add = (n, icon, tip) => n && parts.push(h("span.part", { title: tip }, parts.length ? ` ${n < 0 ? "−" : "+"} ` : n < 0 ? "−" : "", String(Math.abs(n)), icon));
    add(k.rates, null, `From the city's commerce at ${k.percent}%`);
    add(k.specialists, ic("population"), "From specialists");
    add(k.buildings, ic("cities"), "From buildings");
    add(k.religions, ic("religion"), "From religions");
    add(k.empire, ic("civics"), "From the empire (its traits and civics)");
    add(k.process, ic("production"), "Production turned to it");
    if (k.modifier) parts.push(h("span.part", { title: "Buildings, wonders and the empire's bonuses" }, ` ${k.modifier > 0 ? "+" : "−"}${Math.abs(k.modifier)}%`));
    return h(
      `div.cs-out.${name}`,
      { tip: `${dname("CommerceType", k.commerce)}: ${k.rate} a turn` },
      h("span.cs-out-top", ic(name, true), h("b", String(k.rate))),
      h("span.cs-out-parts", parts.length ? parts : "0"),
    );
  });
  return h(
    "div.cs-row.cs-commerce",
    h("span.cs-raw", { tip: `Commerce: ${raw} from the city's plots and trade routes, split by the empire's rates` }, ic("commerce", true), h("b.cs-big.com", String(raw))),
    h("span.cs-arrow", "→"),
    total || raw ? bar : h("span.dim", "no commerce"),
    h("span.cs-arrow", "→"),
    h("div.cs-outs", outs.length ? outs : h("span.dim", "nothing")),
  );
}

/** The map's tooltip over city plot `p` (the overlay's `tipOf`): what a click does, what the
 *  plot gives and, for a plot to work, who leaves their place for it (`next_worker`). */
function plotTipNodes(p, mine, c) {
  const line = (...xs) => h("div.ptip-line", xs);
  const gives = (y, sign = 1) => h("span.gives", (y ?? []).map((v, j) => (v ? h("span.give", ic(["food", "production", "commerce"][j]), signed(sign * v)) : null)));
  const state = plotState(p, mine);
  const dflt = defaultSpecialist();
  switch (state) {
    case "home":
      return [h("b", "The city's own plot"), line("Always worked, by nobody: it is free."), p.yields ? line(gives(p.yields)) : null, line(h("span.dim", "Click: the governor picks the citizens."))];
    case "hidden":
      return [h("b", "Unexplored")];
    case "worked":
      return [
        h("b", "Worked"),
        line(gives(p.yields)),
        line(h("span.dim", "Click: take the citizen off it")),
        dflt !== null ? line(token("spec", typeIcon("specialist", dflt, 16)), ` a ${dname("SpecialistType", dflt)}`, specialistGives(dflt)) : null,
      ];
    case "other":
      return [h("b", `Worked by ${S.cities.get(p.worked_by)?.name ?? "another city"}`), line(gives(p.yields)), p.owner === mine ? line(h("span.dim", "Click: take it for this city")) : null];
    case "free": {
      const w = c.next_worker;
      let from = null;
      if (w?.idle) from = line(token("idle"), " a citizen without work takes it");
      else if (w?.specialist !== undefined) from = line(token("spec", typeIcon("specialist", w.specialist, 16)), ` the ${dname("SpecialistType", w.specialist)} leaves`, specialistGives(w.specialist, -1));
      else if (w?.plot !== undefined) {
        const q = c.plots.find((x) => x.i === w.plot);
        from = line(token(""), " moves off a plot:", gives(q?.yields, -1));
      } else from = line(h("span.bad", "No citizen to spare"));
      return [h("b", "Work this plot"), line(token("ghost"), gives(p.yields)), from];
    }
    default:
      return [h("b", p.owner === mine ? "Cannot be worked now" : "Not your plot"), p.yields ? line(gives(p.yields)) : null];
  }
}

/** Building tiers (docs/SUITABILITY.md): the class that colours a building's name by its tier
 *  (`tier-0` Lesser .. `tier-4` Legendary; none for Standard or a building without a tier). */
export function tierSel(t) {
  return t === null || t === undefined || t === 1 ? "" : `.tier-${t}`;
}

/** The hint line of a national unit whose limit is a price: how many the player has, how many
 *  at the normal cost, and the cost past them. */
function nationalHint(b) {
  if (b.national_normal === undefined) return "";
  return `National: you have ${b.national_have}; ${b.national_normal} at the normal cost, then ${b.national_over_percent}%.`;
}

/** The hint line of a building in the chooser: its tier, and a wonder's cost there. */
function tierHint(b) {
  if (b.tier === undefined || b.tier === null) return "";
  const name = dname("BuildingTierType", b.tier);
  return b.cost_percent !== undefined ? `Site: ${name}. Cost: ${b.cost_percent}%.` : `Tier: ${name}.`;
}

/** The city's hand: its building for every class it rolled a tier other than Standard for, by
 *  tier (a wonder's tier is its cost there). */
function tierHandView(c) {
  if (!c.tier_hand || !c.tier_hand.length) return null;
  const rows = [];
  for (const t of [4, 3, 2, 0]) {
    const xs = c.tier_hand.filter((x) => x.tier === t);
    if (!xs.length) continue;
    rows.push(
      h(
        "div.tier-row",
        h(`span.tier-label${tierSel(t)}`, `${dname("BuildingTierType", t)}: `),
        h(
          "span.chips",
          xs.map((x) => h(`span.chip${tierSel(t)}`, { tip: infoTip("BuildingType", x.building, { hint: x.cost_percent !== undefined ? `Site: ${dname("BuildingTierType", t)}. Cost: ${x.cost_percent}%.` : "" }) }, dname("BuildingType", x.building), x.cost_percent !== undefined ? h("span.dim", ` ${x.cost_percent}%`) : null)),
        ),
      ),
    );
  }
  return h("details.section.tier-hand", h("summary", h("h4", "Building tiers")), rows);
}

function buildingsView(c, reload) {
  const b = c.buildings.map((x) => h(`span.chip${tierSel(x.tier)}`, { tip: infoTip("BuildingType", x.building, { hint: tierHint(x) }) }, typeIcon("building", x.building, 8), dname("BuildingType", x.building), x.free && !x.real ? h("span.dim", " (free)") : null));
  const r = c.religions.map((x) => h("span.chip", typeIcon("religion", x.religion, 8), dname("ReligionType", x.religion), x.holy ? h("span.good", " ★ holy city") : null));
  const units = c.garrison.map((u) => h("span.chip", `${dname("UnitType", u.type)}${u.owner !== me()?.player ? ` (${u.owner})` : ""}`));
  return [
    h("div.section", h("h4", `Buildings (${b.length})`), b.length ? h("div.chips", b) : h("p.dim", "None.")),
    tierHandView(c),
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
  // A governed city's emphasis is its governor's profile's (shown, not chosen).
  const governed = !!c.governance?.governed;
  const emphasize = c.emphasize.map((on, e) =>
    h(
      "label.toggle",
      { title: governed ? "Set by the governor's profile" : `${on ? "Turn off" : "Turn on"} ${dname("EmphasizeType", e)}` },
      h("input", { type: "checkbox", checked: on, disabled: governed, "data-emphasize": e, onchange: (ev) => task("TASK_SET_EMPHASIZE", e, -1, ev.target.checked) }),
      ` ${dname("EmphasizeType", e)}`,
    ),
  );
  return h(
    "div.section",
    h("h4", "Orders"),
    h(
      "div.row",
      hurries,
      c.can_conscript ? h("button.small", { onclick: () => task("TASK_CONSCRIPT"), title: "Draft a unit (costs population and anger)" }, "Draft") : null,
    ),
    // A governed city's production is always its governor's (this game's governors).
    c.governance?.governed
      ? null
      : h("label.toggle", h("input", { type: "checkbox", checked: c.production.automated, id: "city-auto-production", onchange: (ev) => task("TASK_SET_AUTOMATED_PRODUCTION", -1, -1, ev.target.checked) }), " Automate production (the governor chooses)"),
    emphasize.length ? h("details", h("summary", governed ? "Emphasis (the governor's)" : "Emphasis"), h("div.checks", emphasize)) : null,
  );
}

function productionView(c, choose, remove, [tapMode, setTapMode], reload) {
  const prod = c.production;
  // This game's governors: a governed city's production is its governor's (shown, not chosen).
  const locked = !!c.governance?.governed;
  const queue = h(
    "ol.queue",
    c.queue.length
      ? c.queue.map((o, i) =>
          h(
            "li",
            h("span", orderName(o.order, o.type)),
            h("span.dim", o.turns !== null ? ` ${turns(o.turns)}` : ""),
            // This game's armies: where a land combat unit goes when trained; a requisition order
            // is its army's.
            o.requisition ? requisitionTag(o) : locked ? (o.dest_name ? h("span.dim", ` → ${o.dest_name}`) : null) : recruitControls(c, i, o, reload),
            // The exe pops an order only while the queue holds more than one (CvGameInterface
            // `getOrderQueueLength() > 1`); the only order is replaced by choosing another. A
            // requisition order is released on the Armies screen, not here.
            c.queue.length > 1 && !locked && !o.requisition ? h("button.small", { title: "Remove from the queue", "aria-label": "Remove", "data-index": i, onclick: () => remove(i) }, "✕") : null,
          ),
        )
      : h("li.dim", locked ? "Nothing yet: the governor chooses when your turn ends." : "Nothing: choose below."),
  );
  const lists = locked ? [] : GROUPS.map(([order, label]) => {
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
              tip: infoTip(kind, b.type, { hint: [tierHint(b), nationalHint(b), b.enabled ? "Click: build now. Shift: add to the queue. Ctrl: put first." : "Not possible now."].filter(Boolean).join(" ") }),
              onclick: (ev) => choose(order, b.type, ev),
            },
            isoSkin() ? buildIcon(order, b.type) : null,
            h(`span${tierSel(b.tier)}`, dname(kind, b.type)),
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
      locked ? [h("p.dim.governed-note", "The governor chooses what this city builds: choose its governor above."), requisitionNote(c)] : recruitView(c, reload),
      locked ? null : tapModes(tapMode, [["now", "Build now", "A tap replaces what is being built"], ["queue", "Add to queue", "A tap adds to the end of the queue (Shift-click)"], ["first", "Put first", "A tap puts it first in the queue (Ctrl-click)"]], setTapMode),
    ),
    ...lists,
  ];
}

function screen(c, { task, choose, remove, popup, overlay, tap, reload }) {
  // On the map (the isometric interface) the plots are worked there: citywork.js draws the
  // citizens on them; the dialog shows them as a grid.
  const plots = overlay ? null : plotGrid(c, task);
  // This game's cultural channel after the city's numbers (its culture among them), above the
  // buildings: the right column's build lists are long.
  const left = h("div.city-col", plots, citizensView(c, task), moodView(c), growthView(c), popup ? null : cultureView(c, reload), buildingsView(c, reload));
  const right = h("div.city-col", governanceView(c, reload), manageView(c, task), productionView(c, choose, remove, tap, reload));
  return [summaryView(c), popup ? h("div.city-cols.popup", right, left) : h("div.city-cols", left, right)];
}
