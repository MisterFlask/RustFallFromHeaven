// The battle plan (the formation-battle prototype's set-up, phase 6 revision 4): before an attack,
// both lines on the battlefield (yours as the rules' auto formation proposes it, theirs as you know
// it), the ground's scenery and its rules in words, a tooltip of what each unit is, the pool's
// other units, the forecast, and Attack. Click one of your units, then a cell of your grid to move
// it there or another unit to swap (a large unit takes two cells of a row); Auto-arrange asks the
// rules again. Every formation is sent to the source for its forecast: every number here is the
// engine's, none is worked out by the client.

import { h, toast } from "./dom.js";
import {
  uname, rowsLine, ROWS, actionAt, actionText, reachText, specials, slotSum, slotCap,
  placeOf, placeFromList, formationOf, previewForecast, gridOf, slotsUsed, placeUnit, benchUnit, inLine, x100, stat, hpOf, tyLabel,
} from "./battle_log.js";
import { loadArt, unitXY, cellXY, cellAt, figBox, pic, faceFlip, headChip, uiIcon, btn, add, groundLine, FRAME_W, FRAME_H, COLORS } from "./battle_stage.js";

export const NAME = "plan";

// `fc`: the forecast of the formation shown (null: none); `pending`: a preview asked and not yet
// answered; `refused`: the last answer was for another formation than the one asked (the engine
// refused it and answered in its auto formation); `busy`: the attack is on its way (the plan stays
// open until its battle's screens take over); `visit`: counts the plan's openings and leavings (an
// answer that comes after the plan was left is dropped).
const P = { ctx: null, pv: null, F: null, place: null, def: null, sel: null, hover: null, hoverCell: null, fc: null, pending: false, refused: false, errors: [], declare: null, seq: 0, timer: 0, busy: false, visit: 0, figs: [], flash: null, tip: null };

const me = 0;

export async function enter(ctx) {
  const visit = ++P.visit;
  P.ctx = ctx;
  P.sel = null;
  P.hover = null;
  P.hoverCell = null;
  P.flash = null;
  P.busy = false;
  P.pv = null;
  render(ctx);
  const pv = await ctx.source.preview({});
  if (visit !== P.visit) return;
  // No plan (the engine answers a plot the player does not see, or one not next to the attackers,
  // with its errors alone): say why and close.
  if (!pv?.fighters) {
    toast((pv?.errors ?? []).join("; ") || "There is no battle to plan there.", "info");
    ctx.close();
    return;
  }
  await loadArt({ fighters: pv.fighters, plot: pv.plot, from: pv.from, sides: pv.sides });
  if (visit !== P.visit) return;
  ctx.stage.me = me;
  ctx.stage.setBoard(pv);
  P.pv = pv;
  P.F = pv.fighters;
  P.place = placeOf(P.F, me);
  P.def = placeOf(P.F, 1 - me);
  takePreview(pv);
  render(ctx);
}

export function leave() {
  P.visit++;
  P.seq++;
  clearTimeout(P.timer);
  P.tip?.remove();
  P.tip = null;
}

// ---------------------------------------------------------------------------------------------
// The formation and its forecast

/** The formation as the protocol sends it. */
const formation = () => formationOf(P.place, P.F);

/** Takes a preview's forecast, errors and war (its fighters' `b` mapped to ours by unit). `sent`:
 *  the formation asked for (none: the first preview, in the rules' own). A reply for another
 *  formation (the engine refused the one sent and answered in its auto formation) brings no
 *  forecast of this one: its numbers are dropped, its errors say why. */
function takePreview(pv, sent = null) {
  const byUnit = new Map(P.F.map((f) => [f.unit, f.b]));
  const { forecast, refused } = previewForecast(pv, sent);
  P.fc = forecast ? { ...forecast, per: (forecast.per ?? []).map((p) => ({ ...p, b: byUnit.get(pv.fighters?.[p.b]?.unit) ?? p.b })) } : null;
  P.pending = false;
  P.refused = refused;
  P.errors = pv.errors ?? [];
  P.declare = pv.declare_war ?? null;
  // The units the battle calls (docs/ENGAGEMENTS.md, "Summons"): made for it, no fighter here.
  P.summons = forecast ? (pv.summons ?? []).filter((x) => !x.on_kill) : [];
}

/** The formation changed: the board now, the forecast when the source answers. */
function changed(ctx) {
  P.fc = null;
  P.pending = true;
  P.refused = false;
  render(ctx);
  clearTimeout(P.timer);
  const seq = ++P.seq;
  P.timer = setTimeout(async () => {
    const sent = formation();
    let pv;
    try {
      pv = await ctx.source.preview({ formation: sent });
    } catch (e) {
      if (seq !== P.seq) return;
      P.pending = false;
      P.errors = [`The forecast failed: ${e.message ?? e}`];
      render(ctx);
      return;
    }
    if (seq !== P.seq) return;
    takePreview(pv, sent);
    render(ctx);
  }, 120);
}

function flash(ctx, text) {
  P.flash = { text, until: performance.now() + 4000 };
  render(ctx);
}

/** Puts your unit `b` at (`lane`, `row`) (a swap when someone stands there). */
function moveTo(ctx, b, lane, row) {
  const r = placeUnit(P.place, P.F, { lanes: P.pv.lanes[me], cap: slotCap(P.pv.slots[me]) }, b, lane, row);
  if (r.error) return flash(ctx, r.error);
  P.place = r.place;
  P.sel = null;
  if (r.out.length) P.flash = { text: `${r.out.map((o) => uname(P.pv, o)).join(" and ")} ${r.out.length > 1 ? "leave" : "leaves"} the line: no room where ${uname(P.pv, b)} stood.`, until: performance.now() + 4000 };
  changed(ctx);
}

function takeOut(ctx, b) {
  P.place = benchUnit(P.place, b);
  P.sel = null;
  changed(ctx);
}

async function autoArrange(ctx) {
  if (P.busy) return;
  let list;
  try {
    list = await ctx.source.autoArrange();
  } catch (e) {
    return flash(ctx, `Auto-arrange failed: ${e.message ?? e}`);
  }
  P.place = placeFromList(P.F, list);
  P.sel = null;
  changed(ctx);
}

/** Attacks with the formation shown. While the attack is on its way (the `engage` command, a
 *  declaration of war asked, the battle's log) the plan cannot be closed (Escape and Cancel wait):
 *  the battle's screens take over from it, and its message is theirs to show. */
async function attack(ctx) {
  if (P.busy || !P.pv || P.errors.length || !P.place.size) return;
  const visit = P.visit;
  P.busy = true;
  render(ctx);
  let r = null;
  try {
    r = await ctx.source.attack({ formation: formation(), declare_war: P.declare !== null });
  } catch (e) {
    console.warn("battle plan: the attack:", e);
  } finally {
    P.busy = false;
  }
  // The plan was left meanwhile (the overlay closed): nothing to show the battle in.
  if (visit !== P.visit) return;
  // Not made (refused, a war not declared): the plan stays.
  if (!r?.log) return render(ctx);
  if (ctx.prefs.get("watch", "watch") === "report") ctx.toReport(r.log);
  else ctx.toWatch(r.log);
}

// ---------------------------------------------------------------------------------------------
// The board

/** Where fighter `b` stands now (yours as edited, theirs as known), or null. */
const posOf = (b) => (P.F[b].side === me ? P.place.get(b) : P.def.get(b)) ?? null;

function scene(now) {
  const sides = P.pv.sides;
  const G = P.ctx.stage.G;
  const figs = [];
  for (const f of P.F) {
    const pos = posOf(f.b);
    if (!pos) continue;
    const [x, y] = unitXY(G, f, pos);
    const sel = P.sel === f.b;
    const per = P.fc?.per?.find((p) => p.b === f.b);
    figs.push({
      b: f.b,
      x,
      y,
      side: f.side,
      large: f.large,
      pic: pic(f, sides, "idle", now + f.b * 137, true),
      flip: faceFlip(f, sides),
      sel: sel ? COLORS.gold : null,
      glow: sel ? COLORS.gold : P.hover === f.b ? "#ffffff" : null,
      bar: { hp: f.hp_start, max: f.max_hp * 100 },
      odds: per ? `${per.fall_pct}%` : null,
      oddsHot: per && per.fall_pct > 50,
      box: figBox(f, x, y),
    });
  }
  const cells = [];
  if (P.sel !== null) {
    for (let r = 0; r < 3; r++) for (let l = 0; l < P.pv.lanes[me]; l++) cells.push({ side: me, lane: l, row: r });
    if (P.hoverCell && P.hoverCell.side === me) cells.push({ ...P.hoverCell, fill: "rgba(246, 198, 80, 0.32)", color: COLORS.goldLt });
  }
  return { figs, cells };
}

export function frame(ctx, _dt, now) {
  if (!P.pv) return;
  const sc = scene(now);
  P.figs = sc.figs;
  ctx.stage.draw(sc);
  if (P.flash && now > P.flash.until) {
    P.flash = null;
    document.getElementById("bt-flash")?.remove();
  }
}

/** Frame coordinates of a pointer event. */
function framePoint(ev) {
  const r = P.ctx.stage.canvas.getBoundingClientRect();
  return [((ev.clientX - r.left) * FRAME_W) / r.width, ((ev.clientY - r.top) * FRAME_H) / r.height];
}

function figUnder(x, y) {
  let best = null;
  for (const f of P.figs) if (x >= f.box.x0 && x <= f.box.x1 && y >= f.box.y0 && y <= f.box.y1 && (!best || f.y > best.y)) best = f;
  return best;
}

/** What a click at frame point (x, y) is on: `{b, cell}`, the fighter under it (the frontmost hit
 *  box) and its cell (of a large unit, the one of its two the point is over), else the cell there
 *  (`b` null; `cell` null off the grids). */
function hitAt(x, y) {
  const G = P.ctx.stage.G;
  const f = figUnder(x, y);
  if (!f) return { b: null, cell: cellAt(G, x, y) };
  const pos = posOf(f.b);
  const c = cellAt(G, x, y);
  const lane = f.large && c && c.side === f.side && c.row === pos.row && c.lane === pos.lane + 1 ? pos.lane + 1 : pos.lane;
  return { b: f.b, cell: { side: f.side, lane, row: pos.row } };
}

function onMove(ev) {
  if (!P.pv) return;
  const [x, y] = framePoint(ev);
  const f = figUnder(x, y);
  const hover = f ? f.b : null;
  const cell = cellAt(P.ctx.stage.G, x, y);
  P.hoverCell = cell;
  ev.currentTarget.style.cursor = (f && P.F[f.b].side === me) || (P.sel !== null && cell?.side === me) ? "pointer" : f ? "help" : "default";
  if (hover !== P.hover) P.hover = hover;
  if (hover !== null) showTip(P.F[hover], ev.clientX, ev.clientY);
  else hideTip();
}

function onClick(ev) {
  if (!P.pv || P.busy) return;
  const [x, y] = framePoint(ev);
  const { b, cell } = hitAt(x, y);
  clickAt(P.ctx, b, cell);
}

/** A click on fighter `b` (or null) in `cell` (or null). */
function clickAt(ctx, b, cell) {
  if (b !== null && P.sel === b) {
    P.sel = null;
    return render(ctx);
  }
  if (P.sel !== null && cell && cell.side === me) return moveTo(ctx, P.sel, cell.lane, cell.row);
  P.sel = b !== null && P.F[b].side === me ? b : null;
  render(ctx);
}

export function key(ctx, ev) {
  if (ev.key === "Enter") {
    attack(ctx);
    return true;
  }
  if (ev.key === "Escape") {
    // The attack on its way: the plan stays until the battle's screens take over.
    if (P.busy) return true;
    if (P.sel !== null) {
      P.sel = null;
      render(ctx);
    } else ctx.close();
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------------------------
// What a unit is: the tooltip

/** The tooltip's body for fighter `f`. */
export function unitInfo(pv, f, { pos, fc } = {}) {
  const rows = [0, 1, 2].map((r) => {
    const { kind, n } = actionAt(f, r);
    return h(`tr${pos && pos.row === r ? ".here" : ""}`, h("th", ROWS[r]), h("td", actionText(f, r)), h("td.dim", n ? reachText(kind, pv.ground) : ""));
  });
  const per = fc?.per?.find((p) => p.b === f.b);
  return [
    h("div.bt-row", h(`span.bt-chip.tiny.${f.side === me ? "you" : "them"}`, f.side === 0 ? "Attacker" : "Defender"), h("b", uname(pv, f.b)), f.level > 1 ? h("span.dim.small", `level ${f.level}`) : null),
    f.large ? h("div.small", "Large") : null,
    h("div.small", h("span.amber", `ATK ${stat(f.a_x100)}`), " · ", h("span.frost", `DEF ${stat(f.dv_x100)}`), f.ma_x100 >= 10 ? ` · MAG ${stat(f.ma_x100)}` : "", ` · MDEF ${stat(f.md_x100)} · ${hpOf(f.hp_start)} of ${f.max_hp} HP · speed ${f.speed}`),
    (f.promotions ?? []).length ? h("div.small.dim", f.promotions.map((p) => tyLabel("PromotionType", p)).join(", ")) : null,
    h("table.bt-rows", h("thead", h("tr", h("th", "Row"), h("th", "Actions"), h("th", "Hits"))), h("tbody", rows)),
    h(
      "ul.bt-notes",
      specials(f, pv.ground, pv.params).map((n) => h("li", n)),
    ),
    per ? h("div.small", "In the forecast it falls in ", h("b", `${per.fall_pct}%`), " of battles", per.escape_pct ? ` and escapes a killing blow in ${per.escape_pct}%` : "", ".") : null,
    pos ? null : h("div.small.dim", f.side === me ? "Not in the line: click it, then a cell, to place it." : "Not in their line: it waits on the plot."),
  ];
}

function showTip(f, cx, cy) {
  if (!P.tip) {
    P.tip = h("div.bt-pnl.bt-tip", { role: "tooltip" });
    P.ctx.ui.append(P.tip);
  }
  if (P.tip.dataset.b !== String(f.b) || P.tip.dataset.fc !== String(!!P.fc)) {
    P.tip.dataset.b = String(f.b);
    P.tip.dataset.fc = String(!!P.fc);
    P.tip.replaceChildren();
    add(P.tip, unitInfo(P.pv, f, { pos: posOf(f.b), fc: P.fc }));
  }
  const fr = P.ctx.stage.canvas.getBoundingClientRect();
  const tw = P.tip.offsetWidth;
  const th = P.tip.offsetHeight;
  let x = cx - fr.left + 18;
  let y = cy - fr.top + 14;
  if (x + tw > FRAME_W - 6) x = cx - fr.left - tw - 18;
  if (x < 6) x = 6;
  if (y + th > FRAME_H - 6) y = Math.max(6, cy - fr.top - th - 14);
  P.tip.style.left = `${Math.round(x)}px`;
  P.tip.style.top = `${Math.round(y)}px`;
}

function hideTip() {
  P.tip?.remove();
  P.tip = null;
}

// ---------------------------------------------------------------------------------------------
// The panels

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);

function render(ctx) {
  const ui = ctx.ui;
  const ov = ctx.ov;
  hideTip();
  ui.replaceChildren();
  ov.replaceChildren();
  const pv = P.pv;
  if (!pv) {
    add(ui, pnl({ left: "540px", top: "360px", width: "200px", textAlign: "center" }, h("span.dim", "Planning…")));
    return;
  }
  const F = P.F;
  // the pointer's layer over the board (under the panels)
  add(ov, h("div.bt-board", { onmousemove: onMove, onmouseleave: () => ((P.hover = null), (P.hoverCell = null), hideTip()), onclick: onClick }));
  const where = pv.plot?.city?.name ?? (pv.sides?.[1 - me]?.label ? `the ${pv.sides[1 - me].label}` : "the plot");
  const used = slotsUsed(P.place, F);
  const mineIn = F.filter((f) => f.side === me && P.place.has(f.b));
  const mineOut = F.filter((f) => f.side === me && !P.place.has(f.b));
  const theirsIn = F.filter((f) => f.side !== me && P.def.has(f.b));
  const theirsOut = F.filter((f) => f.side !== me && !P.def.has(f.b));
  const watch = ctx.prefs.get("watch", "watch");
  // ---- top left: the attack, your side
  add(
    ui,
    pnl(
      { left: "6px", top: "4px", width: "380px" },
      h("div.bt-row", uiIcon("attack", 16), h("span.bt-h3", `Attack ${where}`)),
      h("div.bt-row", h("span.bt-h3.you", pv.sides?.[me]?.label ?? "Your army"), h("span.dim", `${used} of ${pv.slots[me].total}${pv.slots[me].river_only ? ` (+${pv.slots[me].river_only} river)` : ""} slots · ${pv.lanes[me]} lanes`)),
      h("div.small.dim", { title: "Why this many slots" }, slotSum(pv.slots[me])),
      h(
        "div.bt-row",
        { style: { marginTop: "2px" } },
        btn("Auto-arrange", { cls: "sm", title: "The rules' formation: one of each kind first, the strongest, each in the row it prefers", onclick: () => autoArrange(ctx) }),
        h("span.dim", "Then"),
        btn("Watch", { cls: "sm", on: watch === "watch", title: "Attack, then watch the battle", onclick: () => (ctx.prefs.set("watch", "watch"), render(ctx)) }),
        btn("Report", { cls: "sm", on: watch === "report", title: "Attack, then straight to the report", onclick: () => (ctx.prefs.set("watch", "report"), render(ctx)) }),
      ),
    ),
  );
  // ---- top right: their side, the ground
  add(
    ui,
    pnl(
      { right: "6px", top: "4px", width: "470px" },
      h("div.bt-row", h("span.bt-h3.them", pv.sides?.[1 - me]?.label ?? where), h("span.dim", `${slotsUsed(P.def, F)} of ${pv.slots[1 - me].total} slots · ${pv.lanes[1 - me]} lanes`)),
      theirsOut.length
        ? h("div.bt-row.small", { title: "Defenders stay in place until annihilated: the plot falls only when none is left" }, h("span.dim", `${theirsOut.length} more wait${theirsOut.length > 1 ? "" : "s"} on the plot`), theirsOut.slice(0, 8).map((f) => tipped(f, headChip(f, pv.sides, { w: 22, h: 22, flip: true }))))
        : null,
      groundLine(pv.ground, pv.plot),
    ),
  );
  // ---- bottom left: the pool's other units, the selection
  const benchRows = mineOut.map((f) =>
    tipped(
      f,
      h(
        `div.bt-bench${P.sel === f.b ? ".sel" : ""}`,
        { role: "button", tabIndex: 0, "data-b": f.b, title: "", onclick: () => ((P.sel = P.sel === f.b ? null : f.b), render(ctx)) },
        headChip(f, pv.sides, { w: 28, h: 26 }),
        h("span.bt-ell", h("b", uname(pv, f.b)), h("br"), h("span.small.dim", `${rowsLine(f)}${f.large ? " · large" : ""} · ATK ${stat(f.a_x100)} · DEF ${stat(f.dv_x100)}`)),
      ),
    ),
  );
  const sel = P.sel !== null ? F[P.sel] : null;
  add(
    ui,
    pnl(
      { left: "6px", top: "506px", width: "420px", height: "288px", display: "block" },
      h("div.bt-row", h("span.bt-h4", "Not in the line"), h("span.dim.small", `${mineIn.length} of ${mineIn.length + mineOut.length} in the line`)),
      benchRows.length ? h("div.bt-benchlist", benchRows) : h("div.dim.small", "Every unit is in the line."),
      h("div.bt-rule"),
      sel
        ? h(
            "div",
            h("div.small", h("b.goldlt", uname(pv, sel.b)), P.place.has(sel.b) ? ": click a cell of yours to move it there, or another unit to swap." : ": click a cell of yours to place it (a unit there leaves the line)."),
            P.place.has(sel.b) ? btn("Take out of the line", { cls: "sm", onclick: () => takeOut(ctx, sel.b) }) : null,
          )
        : h("div.small.dim", "Hover a unit to see what it is. Click one of yours, then a cell of your grid to move it there or another unit to swap."),
    ),
  );
  // ---- bottom right: the forecast, Attack
  add(ui, forecastPanel(ctx, { mineIn, theirsIn, theirsOut }));
  if (P.flash) add(ui, h("div.bt-tag.bt-flash#bt-flash", { role: "status" }, P.flash.text));
}

/** `el` with fighter `f`'s tooltip on hover. */
function tipped(f, el) {
  el.addEventListener("mouseenter", (ev) => showTip(f, ev.clientX, ev.clientY));
  el.addEventListener("mousemove", (ev) => showTip(f, ev.clientX, ev.clientY));
  el.addEventListener("mouseleave", hideTip);
  return el;
}

function forecastPanel(ctx, { mineIn, theirsIn, theirsOut }) {
  const pv = P.pv;
  const fc = P.fc;
  const war = P.declare !== null;
  const pct = (p) => `${Math.round(p / 10)}%`;
  const body = [];
  if (fc) {
    const lost = fc.lost_x100 ?? [0, 0];
    body.push(
      h("div.bt-row", h("span.bt-h4", "Forecast"), h("span.dim.small", `${fc.runs} simulated battles`)),
      h("div.bt-kv", h("span.dim", "You lose"), h("span", h("b.you", (lost[me] / 100).toFixed(1)), h("span.dim", ` of ${mineIn.length} in the line`))),
      h("div.bt-kv", h("span.dim", "They lose"), h("span", h("b.them", (lost[1 - me] / 100).toFixed(1)), h("span.dim", ` of ${theirsIn.length} in the line`))),
      h("div.bt-kv", h("span.dim", "Their line wiped out"), h("b", pct(fc.line_wiped_permille ?? 0))),
      h(
        "div.bt-kv",
        { title: `Defenders stay until annihilated: the plot falls when none is left.${theirsOut.length ? ` ${theirsOut.length} more of theirs wait on the plot to form the next line.` : ""}` },
        h("span.dim", pv.plot?.city ? `${pv.plot.city.name} falls` : "The plot falls"),
        h(`b${(fc.plot_falls_permille ?? 0) >= 500 ? ".good" : ""}`, pct(fc.plot_falls_permille ?? 0)),
      ),
      fc.attackers_wiped_permille ? h("div.bt-kv", h("span.dim", "Your line wiped out"), h("b.bad", pct(fc.attackers_wiped_permille))) : null,
    );
  } else if (P.pending) body.push(h("div.bt-row", h("span.bt-h4", "Forecast"), h("span.dim", "estimating…")), h("div.small.dim", "The engine plays this formation many times over."));
  else body.push(h("div.bt-row", h("span.bt-h4", "Forecast")), h("div.small.dim", !P.place.size ? "Put at least one unit in the line." : P.refused ? "No forecast: the engine refuses this formation." : "No forecast for this formation."));
  if (fc && P.summons?.length) {
    const each = P.summons.map((x) => `${tyLabel("UnitType", x.unit_type)} (${x.side === me ? "yours" : "theirs"}, falls ${x.fall_pct}%)`);
    body.push(h("div.small", { title: "Called into this battle by a caster in the line; not in the counts above." }, h("span.dim", "Summons: "), each.join(", ")));
  }
  if (war) body.push(h("div.warn.small", "Attacking declares war."));
  for (const e of P.errors) body.push(h("div.bad.small", e));
  return pnl(
    { right: "6px", top: "556px", width: "424px", height: "238px", display: "flex", flexDirection: "column", padding: "4px 12px" },
    h("div", { style: { flex: "1" } }, body),
    h(
      "div.bt-row",
      { style: { justifyContent: "flex-end", gap: "8px" } },
      btn("Cancel", { title: "Back to the map [Esc]", disabled: P.busy, onclick: () => !P.busy && ctx.close() }),
      btn([uiIcon("attack", 16), war ? " Declare war and attack" : " Attack"], {
        cls: "primary",
        key: "attack",
        title: war ? "Declare war and attack with this formation [Enter]" : "Attack with this formation [Enter]",
        disabled: P.busy || !!P.errors.length || !P.place.size,
        onclick: () => attack(ctx),
      }),
    ),
  );
}

// ---------------------------------------------------------------------------------------------
// Hooks for the screenshot and smoke scripts

/** Client coordinates of a frame point. */
function clientOf(x, y) {
  const r = P.ctx.stage.canvas.getBoundingClientRect();
  return { x: r.left + (x * r.width) / FRAME_W, y: r.top + (y * r.height) / FRAME_H };
}

/** The client point of the first frame point of `pts` that a real click would take as `want(hit)`
 *  says: the plan's own pointer test (`hitAt`) over the figures as they stand now, at a point of the
 *  board no panel covers. Throws when there is none (`what` says what was aimed at). */
function aim(pts, want, what) {
  P.figs = scene(performance.now()).figs;
  for (const [x, y] of pts) {
    if (x < 0 || y < 0 || x >= FRAME_W || y >= FRAME_H || !want(hitAt(x, y))) continue;
    const c = clientOf(x, y);
    if (document.elementFromPoint(c.x, c.y)?.closest?.(".bt-board")) return c;
  }
  throw new Error(`battle plan: no point of the board a click takes as ${what}`);
}

/** Offsets in [-1, 1], from the middle out. */
const SPREAD = [0, -0.2, 0.2, -0.4, 0.4, -0.6, 0.6, -0.8, 0.8];

export const dev = {
  /** A click on fighter `b` (selects yours, or moves the selection there). */
  click(b) {
    if (!P.pv) return;
    const pos = posOf(b);
    clickAt(P.ctx, b, pos ? { side: P.F[b].side, lane: pos.lane, row: pos.row } : null);
  },
  /** A click on cell (`lane`, `row`) of your grid. */
  clickCell(lane, row) {
    if (!P.pv) return;
    const g = gridOf(P.place, P.F, P.pv.lanes[me]);
    clickAt(P.ctx, g[row]?.[lane] ?? null, { side: me, lane, row });
  },
  auto() {
    return P.ctx && autoArrange(P.ctx);
  },
  attack() {
    return P.ctx && attack(P.ctx);
  },
  /** A point in client coordinates where a real mouse clicks fighter `b` (inside its hit box, not
   *  under a unit in front of it nor a panel), or null when it is not on the board; throws when
   *  no point of its box is its own. */
  point(b) {
    const pos = P.pv && posOf(b);
    if (!pos) return null;
    const [fx, fy] = unitXY(P.ctx.stage.G, P.F[b], pos);
    const box = figBox(P.F[b], fx, fy);
    const cx = (box.x0 + box.x1) / 2;
    const cy = fy - 40;
    const pts = [];
    for (const v of SPREAD) for (const u of SPREAD) pts.push([cx + (u * (box.x1 - box.x0)) / 2, cy + (v * (box.y1 - box.y0)) / 2]);
    return aim(pts, (hit) => hit.b === b, `fighter ${b}`);
  },
  /** A point in client coordinates where a real mouse clicks cell (`lane`, `row`) of side `side`:
   *  inside the cell's diamond, on the cell itself or on the unit standing there (of a large unit,
   *  its half over this cell), not on a unit in front of it nor under a panel. Throws when there
   *  is none. */
  cellPoint(side, lane, row) {
    if (!P.pv) return null;
    const [fx, fy] = cellXY(P.ctx.stage.G, side, lane, row);
    // the diamond's centre is 2 px above the feet; it is 96 px wide each way and 48 tall
    const pts = [];
    for (const v of SPREAD) for (const u of SPREAD) if (Math.abs(u) + Math.abs(v) <= 0.8) pts.push([fx + u * 96, fy - 2 + v * 48]);
    return aim(pts, (hit) => !!hit.cell && hit.cell.side === side && hit.cell.lane === lane && hit.cell.row === row, `cell (${lane}, ${row}) of side ${side}`);
  },
  get formation() {
    return P.pv ? formation() : null;
  },
  get preview() {
    return P.pv;
  },
  get sel() {
    return P.sel;
  },
  /** The last preview's errors (the engine's reasons it refuses the formation or the attack). */
  get errors() {
    return P.errors;
  },
  /** Whether the engine answered the last formation sent in another (it refused that one). */
  get refused() {
    return P.refused;
  },
  /** The forecast of the formation shown (null: none, or not yet). */
  get forecast() {
    return P.fc;
  },
  get busy() {
    return P.busy;
  },
  get ready() {
    return !!P.pv && !P.busy && !P.pending;
  },
  inLine,
};
