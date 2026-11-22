// Trade (docs/TRADE.md, section 9; docs/HUMAN.md, "Trade"): the pure helpers of the Trade layer,
// the Establish dialog, the Trade screen, the city panel and the tooltips. The words come from the
// server (the view's `trade`, the units' `caravan`, the cities' `routes`, the `route_offers`
// query); these only choose and shape them. No DOM and no state, so the node tests read them.

/** The colour of the Establish dialog's preview line (the row under the pointer or the focus). */
export const PREVIEW_COLOR = "#ffe14d";

/** The lines of the Trade layer: `view.trade.lines` (`[{route, owner, mine, segments}]`), each in
 *  its owner's colour (`colorOf(owner, mine)`), then `preview` (segments, or null) in the preview's
 *  colour. `[{route, owner, mine, preview, color, width, segments}]`, only segments of two plots or
 *  more; `width` is the stroke's factor (1, the preview 1.5). */
export function tradeLines(trade, colorOf, preview = null) {
  const out = [];
  for (const l of trade?.lines ?? []) {
    const segments = (l.segments ?? []).filter((s) => s.length > 1);
    if (segments.length) out.push({ route: l.route, owner: l.owner, mine: !!l.mine, preview: false, color: colorOf(l.owner, !!l.mine), width: 1, segments });
  }
  const segs = (preview ?? []).filter((s) => s.length > 1);
  if (segs.length) out.push({ route: null, owner: null, mine: true, preview: true, color: PREVIEW_COLOR, width: 1.5, segments: segs });
  return out;
}

/** Whether a selection of `units` (the view's units) wants the Trade layer: a Caravan among them
 *  (one in a route carries `caravan`; `isCaravan(unit)` tells one in none). */
export function wantsTrade(units, isCaravan) {
  return (units ?? []).some((u) => u && (u.caravan || isCaravan(u)));
}

/**
 * The Trade layer's automatic switch, as the Sea layer's (sea.js `autoSeaLayer`): `layer` the
 * map's layer now, `auto` its memory (null, or `{prev}`), `wants` whether the selection or an open
 * Trade screen or Establish dialog wants the layer. Returns `{layer, auto}`.
 */
export function autoTradeLayer(layer, auto, wants) {
  if (wants) return layer === "trade" ? { layer, auto } : { layer: "trade", auto: { prev: layer } };
  if (auto && layer === "trade") return { layer: auto.prev ?? "terrain", auto: null };
  return { layer, auto: null };
}

/** A city's route lines for the city panel (`city.routes`: `{from, into, limit}`, null for
 *  another's city): `[{kind: "from" | "into", route, text}]`, the routes from it first. */
export function cityRouteLines(routes) {
  if (!routes) return [];
  return [...(routes.from ?? []).map((r) => ({ kind: "from", route: r.route, text: r.text })), ...(routes.into ?? []).map((r) => ({ kind: "into", route: r.route, text: r.text }))];
}

/** The plot tooltip's lines of a caravan in a route (a view unit's `caravan`): `Caravan: Ithralia
 *  to Vallus.`, then `Plunder: 14 gold.` (an enemy's or another's), else `[]`. Your own caravan's
 *  plunder is what an enemy would take: shown too. */
export function caravanTipLines(u) {
  const c = u?.caravan;
  if (!c) return [];
  return [c.text, c.plunder_text].filter(Boolean);
}

/** The Trade screen's caravan line: `Caravan 1: On the road: plot 7 of 17, bound for Vallus.` */
export function caravanLine(c) {
  return `Caravan ${c.slot + 1}: ${c.text}`;
}

/** A route row's tone: "closed" (warn), "lost" (a caravan lost), else "open". */
export function routeTone(r) {
  if (!r?.open) return "closed";
  if ((r.caravans ?? []).some((c) => c.state === "lost")) return "lost";
  return "open";
}
