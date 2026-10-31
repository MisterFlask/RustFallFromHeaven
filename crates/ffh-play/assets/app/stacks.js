// The stacks on the map, computed without the DOM (tested by assets/test/stacks.test.mjs): each
// tile's stack (head unit and its state: docs/ART.md 10.1, drawn by the HD-2D and 3D maps' dressing)
// and the units' moves since the last view (the isometric map's move slide).

/** Civ4's activities as the art's state marks (ui/mark_<state>). Fortified units sleep in C++
 *  (MISSION_FORTIFY sets ACTIVITY_SLEEP), so a sleeping unit that can fight shows the shield. */
export function markOf(activity, { canFight = false, automated = false, promote = false } = {}) {
  if (promote) return "promote";
  if (automated) return "auto";
  switch (activity) {
    case "ACTIVITY_SLEEP":
      return canFight ? "fortify" : "sleep";
    case "ACTIVITY_SENTRY":
    case "ACTIVITY_SENTRY_NAVAL_UNITS":
    case "ACTIVITY_SENTRY_LAND_UNITS":
      return "sentry";
    case "ACTIVITY_HEAL":
      return "heal";
    default:
      return null;
  }
}

/** The head of a stack (the unit drawn): the selection's first, else the viewer's own unit that
 *  can move, else its first, else the strongest foreign one (the best defender stands in front:
 *  docs/ART.md 10.1). `strength(unit)` rates a foreign unit. */
export function headUnit(units, me, selected = [], strength = () => 0) {
  const sel = units.find((u) => selected.includes(u.id));
  if (sel) return sel;
  const own = units.filter((u) => u.owner === me);
  if (own.length) return own.find((u) => (u.moves ?? 0) > 0) ?? own[0];
  let best = units[0] ?? null;
  for (const u of units) if (strength(u) > strength(best)) best = u;
  return best;
}

/**
 * The stack on one tile as the art draws it: `{head, state}` with state `{own, moves, selected,
 * hp, mark, hero, count, facing}` (web/art/units/stack.js). `units`: the view's units on the tile;
 * `ctx`: { me, selected (unit ids), activity(i) -> name, unitInfo(type) -> {hero, combat},
 * automated(group) -> bool, facing(id) -> 'left'|'right' }.
 */
export function stackState(units, ctx) {
  const info = (u) => ctx.unitInfo?.(u.type) ?? {};
  const head = headUnit(units, ctx.me, ctx.selected ?? [], (u) => info(u).strength ?? 0);
  if (!head) return null;
  const own = head.owner === ctx.me;
  const state = {
    own,
    selected: (ctx.selected ?? []).includes(head.id),
    hp: head.max_hp > 0 ? Math.round((100 * head.hp) / head.max_hp) : 100,
    count: units.length,
    hero: units.some((u) => !!info(u).hero),
    facing: ctx.facing?.(head.id) ?? "right",
  };
  if (own) {
    state.moves = (head.moves ?? 0) > 0;
    const promote = units.some((u) => u.owner === ctx.me && u.promo_ready);
    const a = head.activity === null || head.activity === undefined ? null : ctx.activity?.(head.activity);
    const fights = (info(head).combat ?? "NONE") !== "NONE" && (info(head).strength ?? 1) > 0;
    // This game's armies: a marching recruit (the rules move it, it takes no orders) shows the
    // march mark whatever else it is.
    state.mark = head.march ? "march" : markOf(a, { canFight: fights, automated: !!ctx.automated?.(head.group), promote });
  }
  return { head, state };
}

/** The units that changed tile since the last view, for the move slide (iso.js): `prev` is
 *  Map(id -> {x, y}) from the last view; returns `{ positions: Map(id -> {x, y}) for `units`,
 *  moved: [{id, dx, dy}] }` where dx/dy are the wrap-aware net displacement (east, north) of each
 *  unit the client already knew and that moved. A new or vanished unit is never "moved". */
export function unitMoves(prev, units, w, wrap) {
  const positions = new Map();
  const moved = [];
  for (const u of units) {
    positions.set(u.id, { x: u.x, y: u.y });
    const p = prev.get(u.id);
    if (!p || (p.x === u.x && p.y === u.y)) continue;
    let dx = u.x - p.x;
    if (wrap && w > 0) {
      if (dx > w / 2) dx -= w;
      if (dx < -w / 2) dx += w;
    }
    moved.push({ id: u.id, dx, dy: u.y - p.y });
  }
  return { positions, moved };
}
