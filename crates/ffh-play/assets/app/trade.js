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
import { tradeLines as linesOf, caravanLine, routeTone } from "./trade_view.js";

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
// The Establish dialog

/** The Establish dialog of Caravan `unit` (the `route_offers` query). */
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
  const choose = async (o) => {
    m.close(null);
    await run(o.cmd);
  };
  const row = (o) =>
    h(
      "button.trade-offer",
      {
        type: "button",
        "data-city": o.city,
        title: `Establish the route to ${o.name}`,
        onclick: () => choose(o),
        onmouseenter: () => setPreview(o.segments),
        onfocus: () => setPreview(o.segments),
        onmouseleave: () => setPreview(null),
        onblur: () => setPreview(null),
      },
      h("span.trade-offer-text", o.text),
      // Cargo (docs/TRADE.md, section 5): each caravan's cargo selector goes here.
    );
  m = openModal({
    title: r.origin ? `Establish route from ${r.origin.name}` : "Establish route",
    tag: "establish",
    className: "trade-establish",
    help: "trade",
    body: [
      h("p.trade-limits", r.limits),
      r.why ? h("p.warn", r.why) : null,
      offers.length ? h("div.trade-offers", { role: "list" }, offers.map(row)) : h("p.dim", r.empty ?? "No destination."),
      offers.length ? h("p.dim.small", "Pay a turn to you at growth 0. Point at a row: its path on the map.") : null,
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
          h("button.small.danger", { onclick: () => dissolve(r), title: "End the route: its caravans are removed" }, "Dissolve"),
        ),
        h(`div.trade-state${r.open ? "" : ".warn"}`, r.state),
        h("div.trade-pay", h("b", r.pay), " ", h("span.dim", r.dest_pay)),
        h("div.trade-terms.small.dim", (r.terms ?? []).join(" ")),
        h("div.trade-growth.small", r.growth_text),
        h(
          "ul.trade-caravans",
          (r.caravans ?? []).map((c) =>
            h(
              `li.trade-caravan.${c.state}`,
              { "data-slot": c.slot },
              c.x !== null && c.x !== undefined ? h("button.link", { onclick: () => go(c.x, c.y), title: "Show the caravan on the map" }, caravanLine(c)) : h("span", caravanLine(c)),
              c.plunder ? h("span.dim", ` Plunder: ${c.plunder} gold.`) : null,
              // Cargo (docs/TRADE.md, section 5): the caravan's cargo line goes here.
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
  const m = openModal({ title: "Trade", tag: "trade-screen", wide: true, className: "trade-screen", help: "trade", body: draw(), buttons: [{ label: "Close", value: null }] });
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
