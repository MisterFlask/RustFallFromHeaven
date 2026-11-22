// Trade (this game's own rules, docs/TRADE.md section 9; docs/HUMAN.md, "Trade"): the player's side.
//
// - The Trade layer (the map layers' fifth, Ctrl+5; on by itself while a Caravan is selected or
//   the Trade screen or the Establish dialog is open): each open route's line along its path, yours
//   in the interface's vellum, the others' (over the plots you have revealed) in their owner's
//   colour, drawn by the 2D line code of supply lines (supply.js) on all three maps: the flat map
//   (map.js), the isometric map (iso.js) and the HD-2D map's overlay (r3d/hud.js). No shader, mesh
//   or texture of the 3D maps changes.
// - The Establish dialog (`openEstablish`, the unit panel's Establish route on a Caravan in a city
//   of yours): one row per destination, by pay, from the `route_offers` query; a row under the
//   pointer or the focus draws its path on the map (the preview line); choosing one sends
//   `establish_route`.
// - The Trade screen (`openTradeScreen`, the menu's Trade): your routes with their state, pay and
//   terms, growth and caravans, Dissolve (asked first); the routes of others into your cities.
//
// The words come from the server (the view's `trade`); trade_view.js shapes them (tested).

import * as api from "./api.js";
import { S, me, isBarbarian, info } from "./state.js";
import { h, toast, errorText } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { addHelp } from "./help.js";
import { run } from "./commands.js";
import * as T from "./tiles.js";
import { OWN_COLOR, unwrapSegment, drawLine } from "./supply.js";
import { tradeLines as linesOf, caravanLine, routeTone, cargoPay, payText, choicePremiums, choiceRefusal, choiceWarnings } from "./trade_view.js";

addHelp({
  id: "trade",
  group: "Playing",
  order: 6,
  title: "Trade",
  body: () => [
    h("p", "A route links one of your cities (its origin) to another city, yours or another civilization's. A Caravan trained in the origin establishes it; it then runs by itself."),
    h("p", "Each route has two caravans. They travel between the two cities along the route's path and take no orders. Each pays while it lives; the route pays from the first caravan's arrival."),
    h("p", "Pay: the path's length (in full to 8 plots, at half to 24) times the two cities' population, then the percent terms: foreign, over water, the origin's buildings, growth. The destination city gets half of it as well."),
    h("p", "Growth rises each turn both caravans live and the route is open, to its maximum. A killed caravan halves it, gives its killer plunder and re-forms in the origin after 10 turns."),
    h("p", "A foreign route needs open borders and peace with the other civilization. Without them it closes: it pays nothing and its caravans wait in the origin. It opens again by itself."),
    h("p", "Routes: 2, plus 1 for some technologies, civics and votes. A city is the origin of 1 route, more with some buildings; the destination of any number."),
  ],
});

/** The map hooks (main.js): `centerOn(x, y)`, `select(ids)`, `request()` (redraw the map),
 *  `sync()` (the Trade layer's automatic switch). */
const hooks = { centerOn: null, select: null, request: null, sync: null };

export function init({ centerOn, select, request, sync } = {}) {
  hooks.centerOn = centerOn ?? null;
  hooks.select = select ?? null;
  hooks.request = request ?? null;
  hooks.sync = sync ?? null;
}

/** The dialogs open that want the Trade layer (the screen, the Establish dialog). */
const open = new Set();

/** Whether the Trade screen or the Establish dialog is open (the layer is on then). */
export function tradeOpen() {
  return open.size > 0;
}

function setOpen(tag, on) {
  if (on) open.add(tag);
  else open.delete(tag);
  hooks.sync?.();
}

/** The Establish dialog's preview: the segments of the row under the pointer or the focus. */
let preview = null;

function setPreview(segments) {
  preview = segments ?? null;
  hooks.request?.();
}

/** Whether `u` (a view unit) is a Caravan. */
export function isCaravan(u) {
  return info("UnitType", u?.type)?.type === "UNIT_CARAVAN";
}

/** The lines to draw (trade_view.js `tradeLines`), yours in the vellum of supply lines. */
export function tradeLines(view = S.view) {
  const mine = me()?.player ?? null;
  return linesOf(view?.trade, (owner, own) => (own || owner === mine ? OWN_COLOR : T.playerColor(owner, isBarbarian(owner))), preview);
}

/** Whether a map draws the Trade layer's lines now: its layer is "trade", or a preview is up. */
export function tradeShown(layer) {
  return layer === "trade" || !!preview;
}

/** map.js (the flat map, drawn on demand). `m` the MapView. */
export function drawTradeFlat(m, g) {
  const ts = m.cam.ts;
  const drawn = { lines: 0 };
  m.tradeDrawn = drawn;
  if (!tradeShown(m.layer)) return;
  for (const L of tradeLines()) {
    for (const seg of L.segments) {
      const s0 = T.mapToScreen(m.cam, m.map, m.vw, m.vh, seg[0][0], seg[0][1]);
      const pts = unwrapSegment(seg, 0, m.map).map(([dx, y]) => ({ x: s0.sx + dx * ts + ts / 2, y: T.columnToScreen(m.cam, m.vw, m.vh, 0, y).sy + ts / 2 }));
      drawLine(g, pts, L.color, 2.5 * L.width, 0);
      drawn.lines++;
    }
  }
}

/** iso.js (the isometric map, drawn on demand). `iso` the IsoMap, `f` its frame. */
export function drawTradeIso(iso, g, f) {
  const v = iso.view;
  const drawn = { lines: 0 };
  iso.tradeDrawn = drawn;
  if (!tradeShown(v.layer)) return;
  const width = Math.max(2, 4 * f.s);
  for (const L of tradeLines()) {
    for (const seg of L.segments) {
      const c0 = iso.center(seg[0][0], seg[0][1]);
      const pts = unwrapSegment(seg, seg[0][0], v.map).map(([ux, y]) => {
        const st = iso.step(ux - seg[0][0], y - seg[0][1]);
        const x = T.wrapCoord(ux, v.map.w, v.map.wrap_x) ?? ux;
        return { x: c0.sx + st.x, y: c0.sy + st.y + iso.dropAt(x, y) };
      });
      drawLine(g, pts, L.color, width * L.width, 0);
      drawn.lines++;
    }
  }
}

// ---------------------------------------------------------------------------------------------
// Cargo (docs/TRADE.md, section 5): the selectors of the Establish and Refit dialogs

/**
 * One selector per caravan over `cargo` (the query's `{options, preselect, halves}`): `No cargo` or
 * a resource, each option's line bare; under them the chosen options' warnings (an only copy), the
 * pay with the chosen cargos, and the refusal of a choice the rules would refuse (the same resource
 * twice). Returns `{el, choice()}`: `choice()` the `cargo` member to send (`[bonus | null, ...]`) or
 * null while refused.
 */
function cargoPicker(cargo, { label = "Pay with these cargos" } = {}) {
  const choice = [...(cargo.preselect ?? [null, null])];
  const options = cargo.options ?? [];
  const notes = h("div.cargo-notes");
  const draw = () => {
    const refusal = choiceRefusal(choice);
    const pay = cargoPay(cargo.halves, choicePremiums(options, choice));
    notes.replaceChildren(
      ...choiceWarnings(options, choice).map((w) => h("div.warn.cargo-warn", w)),
      refusal ? h("div.warn.cargo-refusal", refusal) : h("div.cargo-pay", `${label}: ${payText(pay)} a turn.`),
    );
  };
  const select = (i) => {
    const line = h("div.cargo-line.small");
    const showLine = () => {
      const o = options.find((x) => x.bonus === choice[i]);
      line.textContent = o ? o.text : "No cargo: no premium.";
    };
    const sel = h(
      "select.cargo-select",
      {
        "aria-label": `Caravan ${i + 1}'s cargo`,
        "data-slot": i,
        onchange: (ev) => {
          choice[i] = ev.target.value === "" ? null : Number(ev.target.value);
          showLine();
          draw();
        },
      },
      h("option", { value: "", selected: choice[i] === null }, "No cargo"),
      options.map((o) => h("option", { value: String(o.bonus), selected: choice[i] === o.bonus }, `${o.name} (${o.class}): +${o.premium}%${o.only_copy ? ", your only copy" : ""}`)),
    );
    showLine();
    return h("label.cargo-slot", h("span.cargo-slot-head", `Caravan ${i + 1}`), sel, line);
  };
  draw();
  const el = h(
    "div.cargo-picker",
    options.length ? [select(0), select(1)] : h("p.dim.small", "No resource to send: no free copy of any."),
    notes,
  );
  return { el, choice: () => (choiceRefusal(choice) ? null : [...choice]) };
}

// ---------------------------------------------------------------------------------------------
// The Establish dialog

/** The Establish dialog of Caravan `unit` (the `route_offers` query). A domestic row sends its
 *  order at once; a foreign row opens its cargo selectors under it (one per caravan), then
 *  Establish sends the order with the cargos. */
export async function openEstablish(unit) {
  let r;
  try {
    r = await api.query("route_offers", { unit });
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const offers = r.offers ?? [];
  let m = null;
  let openRow = null;
  const send = async (cmd) => {
    m.close(null);
    await run(cmd);
  };
  const row = (o) => {
    const box = h("div.trade-offer-box", { "data-city": o.city });
    const btn = h(
      "button.trade-offer",
      {
        type: "button",
        title: o.cargo ? `Choose the cargos of the route to ${o.name}` : `Establish the route to ${o.name}`,
        "aria-expanded": o.cargo ? "false" : undefined,
        onclick: () => {
          if (!o.cargo) return send(o.cmd);
          if (openRow && openRow !== box) openRow.querySelector(".cargo-panel")?.remove();
          if (box.querySelector(".cargo-panel")) {
            box.querySelector(".cargo-panel").remove();
            btn.setAttribute("aria-expanded", "false");
            openRow = null;
            return;
          }
          const pick = cargoPicker(o.cargo);
          const go = h("button.primary.cargo-go", { type: "button", onclick: () => {
            const cargo = pick.choice();
            if (cargo) send({ ...o.cmd, cargo });
          } }, `Establish to ${o.name}`);
          box.append(h("div.cargo-panel", pick.el, go));
          btn.setAttribute("aria-expanded", "true");
          openRow = box;
          setPreview(o.segments);
        },
        onmouseenter: () => setPreview(o.segments),
        onfocus: () => setPreview(o.segments),
        onmouseleave: () => openRow !== box && setPreview(null),
        onblur: () => openRow !== box && setPreview(null),
      },
      h("span.trade-offer-text", o.text),
    );
    box.append(btn);
    return box;
  };
  m = openModal({
    title: r.origin ? `Establish route from ${r.origin.name}` : "Establish route",
    tag: "establish",
    className: "trade-establish",
    help: "trade",
    body: [
      h("p.trade-limits", r.limits),
      r.why ? h("p.warn", r.why) : null,
      offers.length ? h("div.trade-offers", { role: "list" }, offers.map(row)) : h("p.dim", r.empty ?? "No destination."),
      offers.length ? h("p.dim.small", "Pay a turn to you at growth 0. Point at a row: its path on the map. A foreign route: choose each caravan's cargo.") : null,
    ],
    buttons: [{ label: "Cancel", value: null }],
  });
  setOpen("establish", true);
  if (r.origin) hooks.centerOn?.(r.origin.x, r.origin.y);
  m.done.then(() => {
    preview = null;
    setOpen("establish", false);
    hooks.request?.();
  });
  return m;
}

/** The Refit dialog of Caravan `unit` for route `route` (the `route_offers` query's `refits`):
 *  the route's cargo selectors, starting from its cargos, and Refit. */
export async function openRefit(unit, route) {
  let r;
  try {
    r = await api.query("route_offers", { unit });
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const f = (r.refits ?? []).find((x) => x.route === route);
  if (!f) {
    toast("The route no longer exists.", "error");
    return;
  }
  const pick = f.cargo ? cargoPicker(f.cargo, { label: "Pay with these cargos at its growth" }) : null;
  const m = openModal({
    title: `Refit route: ${f.title.replace(/\.$/, "")}`,
    tag: "refit",
    className: "trade-establish trade-refit",
    help: "trade",
    body: [h("p.trade-refit-line", f.line), f.why ? h("p.warn", f.why) : null, pick ? pick.el : h("p.dim", "A domestic route carries no cargo.")],
    buttons: [
      {
        label: "Refit",
        primary: true,
        disabled: !pick || !!f.why,
        onClick: async (m) => {
          const cargo = pick.choice();
          if (!cargo) return;
          m.close(null);
          await run({ ...f.cmd, cargo });
        },
      },
      { label: "Cancel", value: null },
    ],
  });
  setOpen("refit", true);
  m.done.then(() => setOpen("refit", false));
  return m;
}

// ---------------------------------------------------------------------------------------------
// The Trade screen

/** The Trade screen (the view's `trade`). */
export function openTradeScreen() {
  const draw = () => {
    const t = S.view?.trade ?? { routes: [], into: [], limits: { text: "" } };
    const go = (x, y) => {
      if (x === null || x === undefined) return;
      hooks.centerOn?.(x, y);
    };
    const dissolve = async (r) => {
      const v = await ask("Dissolve the route?", `Dissolve the route ${r.title.replace(/\.$/, "")}? Its caravans are removed and its growth is lost. A new route starts at growth 0.`, [
        { label: "Dissolve", value: true, primary: true, danger: true },
        { label: "Cancel", value: null },
      ]);
      if (!v) return;
      await run({ c: "dissolve_route", route: r.id });
    };
    const routeCard = (r) =>
      h(
        `div.trade-route.${routeTone(r)}`,
        { "data-route": r.id },
        h(
          "div.trade-route-head",
          h("button.link.trade-route-title", { onclick: () => go(r.origin.x, r.origin.y), title: "Show the origin on the map" }, r.title),
          h("button.small.danger.trade-dissolve", { onclick: () => dissolve(r), title: "End the route: its caravans are removed" }, "Dissolve"),
        ),
        h(`div.trade-state${r.open ? "" : ".warn"}`, r.state),
        h("div.trade-pay", h("b", r.pay), " ", h("span.dim", r.dest_pay)),
        r.terms?.length ? h("div.trade-terms.small.dim", r.terms.join(" ")) : null,
        h("div.trade-growth.small", r.growth_text),
        h(
          "ul.trade-caravans",
          (r.caravans ?? []).map((c) =>
            h(
              `li.trade-caravan.${c.state}`,
              { "data-slot": c.slot },
              c.x !== null && c.x !== undefined ? h("button.link", { onclick: () => go(c.x, c.y), title: "Show the caravan on the map" }, caravanLine(c)) : h("span", caravanLine(c)),
              c.plunder ? h("span.dim", ` Plunder: ${c.plunder} gold.`) : null,
              // Cargo (docs/TRADE.md, section 5) and the escorts (4.4).
              c.cargo_text ? h(`div.trade-cargo.${c.cargo_state ?? "none"}`, c.cargo_text) : null,
              c.escorts?.length ? h("div.trade-escorts.small", `Escort: ${c.escorts.map((e) => e.name).join(", ")}.`) : null,
            ),
          ),
        ),
      );
    const intoRow = (r) =>
      h(
        `li.trade-into${r.open ? "" : ".dim"}`,
        { "data-route": r.id },
        h("button.link", { onclick: () => go(r.destination.x, r.destination.y), title: "Show the city on the map" }, r.text),
        " ",
        h("span", r.pay),
        r.brings ? h("span.trade-brings", ` ${r.brings}`) : null,
        r.open ? null : h("span.warn", " Closed."),
      );
    return [
      h("p.trade-limits", t.limits?.text ?? ""),
      h("h4", `Your routes (${t.routes.length})`),
      t.routes.length ? h("div.trade-list", t.routes.map(routeCard)) : h("p.dim", "None. A Caravan in one of your cities establishes one."),
      h("h4", `Routes of others into your cities (${t.into.length})`),
      t.into.length ? h("ul.trade-into-list", t.into.map(intoRow)) : h("p.dim", "None."),
    ];
  };
  const m = openModal({ title: "Trade", tag: "trade-screen", className: "trade-screen", help: "trade", body: draw(), buttons: [{ label: "Close", value: null }] });
  screen = { m, draw };
  setOpen("trade-screen", true);
  m.done.then(() => {
    if (screen?.m === m) screen = null;
    setOpen("trade-screen", false);
  });
  return m;
}

/** The open Trade screen (`{m, draw}`), re-read when the view changes. */
let screen = null;

/** The view changed (main.js): the open Trade screen shows it. */
export function viewChanged() {
  if (screen && !screen.m.closed) screen.m.setBody(...screen.draw());
}
