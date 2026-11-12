// Keywords and reminder text (docs/PLAY_DESIGN.md, "Keywords and reminder text"; docs/HUMAN.md):
// the pure helpers of the reminder boxes and the keyword chips (keywords.js), tested in
// test/keywords.test.mjs. The server decides which reminders an entity has (`reminders: [{key,
// name, text, with}]` beside its rules text); this module only lays them out.

/** The chips of an entry: one per reminder, in the server's order (`{key, name}`). */
export function chipsOf(reminders) {
  return (reminders ?? []).map((r) => ({ key: r.key, name: r.name }));
}

/** The reminders a chip of keyword `key` shows: its own, then those it implies (`with`), in the
 *  server's order; `[]` for a key the list does not hold. */
export function remindersFor(reminders, key) {
  const list = reminders ?? [];
  const own = list.find((r) => r.key === key);
  if (!own) return [];
  const keys = own.with?.length ? own.with : [key];
  return keys.map((k) => list.find((r) => r.key === k)).filter(Boolean);
}

/** Whether the reminder boxes go below the tooltip (a narrow window) rather than beside it. */
export function stackedLayout(vw) {
  return vw < 600;
}

/** Where a tooltip (`main`, `{w, h}`; 0 x 0 when it has nothing but reminders) and its reminder
 *  boxes (`side`, `{w, h}`, or null) go near the pointer (`x`, `y`) in a `vw` x `vh` window: beside
 *  it at the right on a wide window, below it on a narrow one (`stacked`), the whole kept inside
 *  the window. Returns `{main: {left, top}, side: {left, top} | null, stacked}`. */
export function tipLayout({ x, y, vw, vh, main, side = null, pad = 14, gap = 6, margin = 4 }) {
  const mw = main?.w ?? 0;
  const mh = main?.h ?? 0;
  const has = !!side && side.w > 0 && side.h > 0;
  const stacked = has && stackedLayout(vw);
  const sw = has ? side.w : 0;
  const sh = has ? side.h : 0;
  const g = mw && mh && has ? gap : 0;
  const W = stacked ? Math.max(mw, sw) : mw + g + sw;
  const H = stacked ? mh + g + sh : Math.max(mh, sh);
  let left = x + pad;
  let top = y + pad;
  if (left + W > vw - margin) left = Math.max(margin, x - pad - W);
  if (top + H > vh - margin) top = Math.max(margin, y - pad - H);
  return {
    main: { left, top },
    side: has ? (stacked ? { left, top: top + mh + g } : { left: left + mw + g, top }) : null,
    stacked,
  };
}
