// Squads (docs/ENGAGEMENTS.md, "Squads"): the squad board and what the strip and the battle plan
// say of squads. A squad is a grid of 3 rows (front, middle, rear) by 3 lanes holding up to 6
// places (a large unit fills two cells side by side and counts 2); one battle is one squad
// against one squad. The board shows the squads of a plot (the `group` query's `squads`) or of an
// army (the Armies screen's): drag a unit to a cell (an empty one: it moves; a unit there: the two
// swap), to another squad, or to "New squad" (`squad_move`); on a touch screen tap the unit, then
// the cell. A squad is Auto (arranged by the rules) until a unit is moved in or out of it; Auto
// gives it back to the rules (`squad_auto`), Auto-sort deals the units of the ticked squads into as
// few squads as hold them (`squad_sort`). Every change is the server's: the board sends the
// command and redraws from the reply it is given.

import { h, coarse } from "./dom.js";
import { dname } from "./state.js";
import { run } from "./commands.js";
import { unitFigure } from "./isoui.js";
import { addHelp } from "./help.js";

addHelp({
  id: "squads",
  group: "Playing",
  order: 6,
  title: "Squads",
  body: () => [
    h(
      "ul",
      h("li", "One battle is one squad against one squad. A squad is a grid of 3 rows (front, middle, rear) by 3 lanes and holds up to 6 places. A large unit fills two cells side by side and counts 2."),
      h("li", "An army's units stand in its squads, a garrison's in the garrison's squads. A unit in neither (a hero, a mage, a scout) can stand in any of your squads on its plot."),
      h("li", "Auto: the rules arrange the squad. Moving a unit in or out of a squad makes it Hand-set: its units keep their cells. Auto gives it back to the rules."),
      h("li", "A new unit joins the first squad with room and a free cell in its best row, else forms a squad of its own."),
      h("li", "Auto-sort: the units of the ticked squads go into as few squads as hold them, front units first, the strongest first, so each squad has a front. Each becomes Auto."),
      h("li", "!: the unit stands outside the row where it has the most actions."),
      h("li", "Attacking: the battle plan names your squads that can attack; the strongest is picked unless you pick another."),
      h("li", "Defending: of the squads that fought the fewest battles this turn, the one with the best result against the attacking squad fights."),
      h("li", "Unit panel: Squads… opens the board of the plot. Armies screen [F5]: each army's board. Drag a unit to a cell (onto a unit of the same size: they swap), to another squad, or to New squad. On a touch screen: tap the unit, then the cell."),
    ),
  ],
});

export const ROWS = ["Front", "Middle", "Rear"];
const LANES = 3;
const ROMAN = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "XIII", "XIV", "XV", "XVI", "XVII", "XVIII", "XIX", "XX"];

/** A squad's number as its numeral: 2 -> "II". */
export const roman = (n) => ROMAN[n - 1] ?? String(n);

/** "Squad II" for a squad `{num}` (a unit's `squad` or a listing's entry). */
export const squadName = (s) => `Squad ${roman(s?.num ?? 0)}`;

const ACT = { Strike: ["strike", "strikes"], Charge: ["charge", "charges"], Stab: ["stab", "stabs"], Shoot: ["shot", "shots"], Spell: ["spell", "spells"], Bombard: ["bombard", "bombards"], Heal: ["heal", "heals"] };

/** One row's actions as words: "2 strikes", or "no action". */
function rowWords(a) {
  const k = typeof a?.kind === "string" ? a.kind[0].toUpperCase() + a.kind.slice(1).toLowerCase() : "";
  return a?.n && ACT[k] ? `${a.n} ${ACT[k][a.n > 1 ? 1 : 0]}` : "no action";
}

/** The row (0 front, 1 middle, 2 rear) where unit `u` (a view unit: `engage_actions`) has the most
 *  actions (ties: the first), or null without them. */
export function bestRow(u) {
  const acts = u?.engage_actions;
  if (!Array.isArray(acts) || acts.length !== 3) return null;
  let best = 0;
  acts.forEach((a, r) => {
    if ((a?.n ?? 0) > (acts[best]?.n ?? 0)) best = r;
  });
  return best;
}

/** Why unit `u` standing in `row` is outside its best row ("Rear: 1 strike. Best: Front, 2
 *  strikes."), or null when it is not. */
export function offRowText(u, row) {
  const best = bestRow(u);
  const acts = u?.engage_actions;
  if (best === null || (acts[row]?.n ?? 0) >= (acts[best]?.n ?? 0)) return null;
  return `${ROWS[row]}: ${rowWords(acts[row])}. Best: ${ROWS[best]}, ${rowWords(acts[best])}.`;
}

/** The tooltip of a unit on the board (units.js sets the unit tooltip of the strip). */
let unitTip = (u) => dname("UnitType", u.type);

export function setSquadUnitTip(f) {
  unitTip = f;
}

/** The ticked squads (ids), kept while their board is redrawn. */
const ticked = new Set();
/** The unit picked by a tap (touch: the next tap on a cell moves it there), or null. */
let picked = null;

/** Sends a squad command, then `done()` (the caller re-reads the squads). */
async function send(c, done) {
  picked = null;
  await run(c);
  await done?.();
}

/** The squad board for `squads` (a `squads` listing), with `unitOf(id)` the view's unit and
 *  `changed()` called after each command (it re-reads the squads and redraws). `bodies`: show
 *  each squad's body (an army's name, a garrison, Loose) when the squads are of several. */
export function squadBoard(squads, { unitOf, changed }) {
  const list = (squads ?? []).filter((s) => s.id !== null && s.id !== undefined);
  for (const id of [...ticked]) if (!list.some((s) => s.id === id)) ticked.delete(id);
  if (picked !== null && !list.some((s) => s.cells.includes(picked))) picked = null;
  const bodies = new Set(list.map((s) => `${s.kind}:${s.army ?? s.city ?? ""}`));
  const root = h("div.sq-board", { "data-squads": list.length });
  const redraw = () => root.replaceWith(squadBoard(squads, { unitOf, changed }));
  const done = () => changed?.();
  const move = (unit, squad, cell) => send({ c: "squad_move", unit, squad, cell }, done);

  const card = (s) => {
    const cells = [];
    for (let c = 0; c < 9; c++) {
      const row = Math.floor(c / LANES);
      cells.push(
        h("div.sq-cell", {
          "data-squad": s.id,
          "data-cell": c,
          class: [row === 0 ? "front" : "", picked !== null && !s.cells[c] ? "target" : ""].filter(Boolean).join(" "),
          style: { gridRow: String(row + 1), gridColumn: String((c % LANES) + 2) },
          "aria-label": `${ROWS[row]}, lane ${(c % LANES) + 1}`,
        }),
      );
    }
    // The units, each once (a large unit over its two cells).
    const seen = new Set();
    const tokens = [];
    s.cells.forEach((id, c) => {
      if (id === null || id === undefined || seen.has(id)) return;
      seen.add(id);
      const u = unitOf(id) ?? { id, type: null };
      const row = Math.floor(c / LANES);
      const span = s.cells[c + 1] === id && (c % LANES) < LANES - 1 ? 2 : 1;
      const off = offRowText(u, row);
      const name = u.name ?? (u.type !== null ? dname("UnitType", u.type) : `#${id}`);
      tokens.push(
        h(
          "div.sq-unit",
          {
            "data-unit": id,
            "data-squad": s.id,
            "data-cell": c,
            tabindex: 0,
            role: "button",
            "aria-label": `${name}, ${ROWS[row].toLowerCase()}, lane ${(c % LANES) + 1}`,
            class: [span > 1 ? "large" : "", off ? "off" : "", picked === id ? "picked" : ""].filter(Boolean).join(" "),
            style: { gridRow: String(row + 1), gridColumn: `${(c % LANES) + 2} / span ${span}` },
            tip: () => [unitTip(u), off ? h("div.tip-stats.warn", off) : null, h("div.tip-hint", coarse() ? "Tap it, then tap a cell: it moves there (onto a unit: they swap)." : "Drag it to a cell (onto a unit: they swap), to another squad or to New squad.")],
          },
          u.type !== null ? unitFigure(u.type, u.owner, span > 1 ? 40 : 30) ?? h("span.sq-abbr", name.slice(0, 2)) : h("span.sq-abbr", "?"),
          u.level !== undefined ? h("span.sq-lv", String(u.level)) : null,
          off ? h("span.sq-off", { "aria-hidden": "true" }, "!") : null,
        ),
      );
    });
    const labels = ROWS.map((r, i) => h("div.sq-rlab", { style: { gridRow: String(i + 1), gridColumn: "1" } }, r));
    return h(
      `div.sq-card${ticked.has(s.id) ? ".sel" : ""}`,
      { "data-squad": s.id },
      h(
        "div.sq-head",
        h(
          "label.sq-name",
          h("input", {
            type: "checkbox",
            checked: ticked.has(s.id),
            "aria-label": `Select ${squadName(s)}`,
            onchange: (ev) => {
              if (ev.target.checked) ticked.add(s.id);
              else ticked.delete(s.id);
              redraw();
            },
          }),
          squadName(s),
        ),
        h(`span.sq-badge.${s.auto ? "auto" : "hand"}`, { title: s.auto ? "Arranged by the rules." : "Arranged by hand." }, s.auto ? "Auto" : "Hand-set"),
        h("span.sq-count", { title: "Places: a large unit counts 2." }, `${s.places}/${s.max_places ?? 6}`),
      ),
      bodies.size > 1 ? h("div.sq-body.dim", s.body) : null,
      h("div.sq-grid", labels, cells, tokens),
      s.auto ? null : h("div.sq-foot", h("button.small.sq-auto", { "data-squad": s.id, title: "The rules arrange this squad again.", onclick: () => send({ c: "squad_auto", squad: s.id }, done) }, "Auto")),
    );
  };

  const all = list.length > 0 && list.every((s) => ticked.has(s.id));
  const n = list.filter((s) => ticked.has(s.id)).length;
  root.append(
    h(
      "div.sq-bar",
      h("button.small.sq-all", { onclick: () => (all ? ticked.clear() : list.forEach((s) => ticked.add(s.id)), redraw()), disabled: list.length < 2 }, all ? "Select none" : "Select all"),
      h(
        "button.small.sq-sort",
        {
          disabled: n < 2,
          title: "The units of the ticked squads in as few squads as hold them, front units first, each squad Auto.",
          onclick: () => {
            const ids = list.filter((s) => ticked.has(s.id)).map((s) => s.id);
            ticked.clear();
            send({ c: "squad_sort", squads: ids }, done);
          },
        },
        n >= 2 ? `Auto-sort ${n} squads` : "Auto-sort",
      ),
      h("span.dim.small.sq-hint", coarse() ? "Tap a unit, then a cell." : "Drag a unit to a cell or a squad."),
    ),
    h("div.sq-list", list.map(card), h("div.sq-new", { "data-new": 1, class: picked !== null ? "target" : "" }, "New squad")),
  );
  wire(root, { list, move, toNew: (unit) => send({ c: "squad_move", unit, squad: null, cell: null }, done), redraw });
  return root;
}

/** The squad and cell a drop on `el` means: `{squad, cell}`, `{fresh: true}` (New squad), or
 *  null. A drop on a large unit's second cell aims at its first. */
function dropOf(el, list) {
  if (!el) return null;
  if (el.closest("[data-new]")) return { fresh: true };
  const t = el.closest("[data-cell]");
  if (!t) return null;
  const squad = Number(t.dataset.squad);
  let cell = Number(t.dataset.cell);
  const s = list.find((x) => x.id === squad);
  const id = s?.cells[cell];
  if (id !== null && id !== undefined && cell % LANES > 0 && s.cells[cell - 1] === id) cell -= 1;
  return { squad, cell };
}

/** The pointer: a press that moves more than 6 px drags the unit (a ghost under the pointer, the
 *  cell under it lit); a press that does not is a tap (pick the unit; with one picked, a tap on a
 *  cell or a unit moves it there). */
function wire(root, { list, move, toNew, redraw }) {
  let drag = null;
  let over = null;
  const light = (el) => {
    const t = el ? el.closest("[data-cell]") ?? el.closest("[data-new]") : null;
    if (t === over) return;
    over?.classList.remove("drop");
    over = t;
    over?.classList.add("drop");
  };
  const drop = (d, target) => {
    const at = dropOf(target, list);
    if (!at) return redraw();
    if (at.fresh) return toNew(d);
    const from = list.find((s) => s.cells.includes(d));
    if (from && from.id === at.squad && from.cells.indexOf(d) === at.cell) return redraw();
    return move(d, at.squad, at.cell);
  };
  root.addEventListener("pointerdown", (e) => {
    const tk = e.target.closest(".sq-unit");
    if (!tk || e.button > 0) return;
    drag = { id: Number(tk.dataset.unit), x: e.clientX, y: e.clientY, on: false, ghost: null, el: tk };
    tk.setPointerCapture?.(e.pointerId);
  });
  root.addEventListener("pointermove", (e) => {
    if (!drag) return;
    if (!drag.on && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 6) {
      drag.on = true;
      const fig = drag.el.querySelector(".figure");
      const g = h("div.sq-ghost", fig ? fig.cloneNode(true) : h("span.sq-abbr", "·"));
      document.body.append(g);
      drag.ghost = g;
      drag.el.classList.add("dragging");
    }
    if (drag.on) {
      drag.ghost.style.left = `${e.clientX}px`;
      drag.ghost.style.top = `${e.clientY}px`;
      light(document.elementFromPoint(e.clientX, e.clientY));
    }
  });
  const end = (e, cancel) => {
    if (!drag) return;
    const d = drag;
    drag = null;
    over?.classList.remove("drop");
    over = null;
    d.ghost?.remove();
    d.el.classList.remove("dragging");
    if (cancel) return;
    if (d.on) {
      picked = null;
      drop(d.id, document.elementFromPoint(e.clientX, e.clientY));
      return;
    }
    // A tap on a unit: with another one picked, the picked one goes to this one's cell (a swap).
    if (picked !== null && picked !== d.id) {
      const p = picked;
      picked = null;
      drop(p, d.el);
      return;
    }
    picked = picked === d.id ? null : d.id;
    redraw();
  };
  root.addEventListener("pointerup", (e) => end(e, false));
  root.addEventListener("pointercancel", (e) => end(e, true));
  root.addEventListener("click", (e) => {
    if (picked === null || e.target.closest(".sq-unit") || e.target.closest("button,input,label")) return;
    const target = e.target.closest("[data-cell]") ?? e.target.closest("[data-new]");
    const p = picked;
    picked = null;
    if (target) drop(p, target);
    else redraw();
  });
  root.addEventListener("keydown", (e) => {
    const tk = e.target.closest(".sq-unit");
    if (tk && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      const id = Number(tk.dataset.unit);
      picked = picked === id ? null : id;
      redraw();
    }
  });
}

/** The units of the strip grouped by squad: `[{squad, units}]` in the order of `squads` (the
 *  `group` query's listing), then a group of the units in no listed squad (`squad` null). */
export function stripGroups(units, squads) {
  const out = [];
  const placed = new Set();
  for (const s of squads ?? []) {
    const ids = new Set(s.cells.filter((x) => x !== null && x !== undefined));
    const us = units.filter((u) => ids.has(u.id));
    if (!us.length) continue;
    us.forEach((u) => placed.add(u.id));
    out.push({ squad: s, units: us });
  }
  const rest = units.filter((u) => !placed.has(u.id));
  if (rest.length) out.push({ squad: null, units: rest });
  return out;
}
