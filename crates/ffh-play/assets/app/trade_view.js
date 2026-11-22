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
  return [c.text, c.cargo_text, c.plunder_text].filter(Boolean);
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

/** The pay a turn to the owner with the chosen cargos (docs/TRADE.md 5 and 6.1, the rules'
 *  `route_pay`): each caravan's half x (100 + its cargo's premium) / 100, rounded down as the rules
 *  do, summed; `halves` the two halves in hundredths (the server's `cargo.halves`), `premiums` the
 *  chosen options' premiums (0 for no cargo). In hundredths. */
export function cargoPay(halves, premiums) {
  return (halves ?? []).reduce((t, h, i) => t + Math.floor((h * (100 + (premiums?.[i] ?? 0))) / 100), 0);
}

/** Hundredths as people read them: one decimal, rounded half up (the view's `pay_text`). */
export function payText(x100) {
  const t = x100 >= 0 ? Math.floor((x100 + 5) / 10) : -Math.floor((-x100 + 5) / 10);
  return `${t < 0 ? "-" : ""}${Math.floor(Math.abs(t) / 10)}.${Math.abs(t) % 10}`;
}

/** The premiums of a choice `[bonus | null, bonus | null]` among `options` (`[{bonus, premium}]`). */
export function choicePremiums(options, choice) {
  return (choice ?? []).map((b) => (b === null || b === undefined ? 0 : (options ?? []).find((o) => o.bonus === b)?.premium ?? 0));
}

/** Why a choice is refused before it is sent (the rules say it again): the two caravans carry
 *  different resources. Null when it may be sent. */
export function choiceRefusal(choice) {
  const [a, b] = choice ?? [];
  return a !== null && a !== undefined && a === b ? "The two caravans carry different resources." : null;
}

/** The warnings of a choice: each chosen option's `warn` (an only copy), once each. */
export function choiceWarnings(options, choice) {
  const out = [];
  for (const b of choice ?? []) {
    const w = (options ?? []).find((o) => o.bonus === b)?.warn;
    if (w && !out.includes(w)) out.push(w);
  }
  return out;
}

/** A standing order's value in the stations' selectors: `caravan:<route>:<slot>` for an Escort of
 *  a caravan, `null` for another order. */
export function caravanStationValue(st) {
  return st?.kind === "escort" && st.route !== null && st.route !== undefined ? `caravan:${st.route}:${st.slot}` : null;
}

/** The `army_station` station of a selector's value `caravan:<route>:<slot>`, else null. */
export function caravanStation(value) {
  const m = /^caravan:(\d+):(\d+)$/.exec(value ?? "");
  return m ? { escort_caravan: Number(m[1]), slot: Number(m[2]) } : null;
}
