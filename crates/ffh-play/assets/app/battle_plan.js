// The battle plan (the formation-battle prototype's set-up, phase 6 revision 4): before an attack,
// both lines on the battlefield as each side's battle tactic arranges them (docs/TACTICS.md: an
// army's tactic, or a garrison's; the grid of one battle is never edited by hand), both tactics
// named, the ground's scenery and its rules in words, a tooltip of what each unit is, the pool's
// other units, the forecast, and Attack. Every number here is the engine's (its preview), none is
// worked out by the client; a tactic is changed on the Armies screen and the city screen.

import { h, toast } from "./dom.js";
import { uname, ROWS, actionAt, actionText, reachText, specials, slotSum, rowsLine, placeOf, slotsUsed, stat, hpOf, tyLabel, tyIdx } from "./battle_log.js";
import { info } from "./state.js";
import { plain } from "./markup.js";
import { loadArt, unitXY, figBox, pic, faceFlip, headChip, uiIcon, btn, add, groundLine, FRAME_W, FRAME_H } from "./battle_stage.js";

export const NAME = "plan";

// `fc`: the forecast of the plan (null: none); `busy`: the attack is on its way (the plan stays
// open until its battle's screens take over); `visit`: counts the plan's openings and leavings (an
// answer that comes after the plan was left is dropped).
const P = { ctx: null, pv: null, F: null, place: null, def: null, hover: null, fc: null, errors: [], declare: null, busy: false, visit: 0, figs: [], tip: null, summons: [], clicks: 0 };

const me = 0;

export async function enter(ctx) {
  const visit = ++P.visit;
  P.ctx = ctx;
  P.hover = null;
  P.busy = false;
  P.pv = null;
  P.clicks = 0;
  render(ctx);
  const pv = await ctx.source.preview();
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
  P.fc = pv.forecast ?? null;
  P.errors = pv.errors ?? [];
  P.declare = pv.declare_war ?? null;
  // The units the battle calls (docs/ENGAGEMENTS.md, "Summons"): made for it, no fighter here.
  P.summons = P.fc ? (pv.summons ?? []).filter((x) => !x.on_kill) : [];
  render(ctx);
}

export function leave() {
  P.visit++;
  P.tip?.remove();
  P.tip = null;
}

/** Side `side`'s tactic: `{name, help}` (the data's; a fixture without tactics fights in Line). */
function tacticOf(side) {
  const v = P.pv?.tactics?.[side] ?? "TACTIC_LINE";
  const t = info("BattleTacticType", tyIdx("BattleTacticType", v));
  return { name: t?.name ?? tyLabel("BattleTacticType", v), help: t?.help ? plain(t.help) : "" };
}

/** Attacks. While the attack is on its way (the `engage` command, a declaration of war asked, the
 *  battle's log) the plan cannot be closed (Escape and Cancel wait): the battle's screens take
 *  over from it, and its message is theirs to show. */
async function attack(ctx) {
  if (P.busy || !P.pv || P.errors.length || !P.place.size) return;
  const visit = P.visit;
  P.busy = true;
  render(ctx);
  let r = null;
  try {
    r = await ctx.source.attack({ declare_war: P.declare !== null });
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
// The board: it shows the lines; the pointer only tells what a unit is

/** Where fighter `b` stands (in its side's line), or null. */
const posOf = (b) => (P.F[b].side === me ? P.place.get(b) : P.def.get(b)) ?? null;

function scene(now) {
  const sides = P.pv.sides;
  const G = P.ctx.stage.G;
  const figs = [];
  for (const f of P.F) {
    const pos = posOf(f.b);
    if (!pos) continue;
    const [x, y] = unitXY(G, f, pos);
    const per = P.fc?.per?.find((p) => p.b === f.b);
    figs.push({
      b: f.b,
      x,
      y,
      side: f.side,
      large: f.large,
      pic: pic(f, sides, "idle", now + f.b * 137, true),
      flip: faceFlip(f, sides),
      sel: null,
      glow: P.hover === f.b ? "#ffffff" : null,
      bar: { hp: f.hp_start, max: f.max_hp * 100 },
      odds: per ? `${per.fall_pct}%` : null,
      oddsHot: per && per.fall_pct > 50,
      box: figBox(f, x, y),
    });
  }
  return { figs, cells: [] };
}

export function frame(ctx, _dt, now) {
  if (!P.pv) return;
  const sc = scene(now);
  P.figs = sc.figs;
  ctx.stage.draw(sc);
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

function onMove(ev) {
  if (!P.pv) return;
  const [x, y] = framePoint(ev);
  const f = figUnder(x, y);
  const hover = f ? f.b : null;
  ev.currentTarget.style.cursor = f ? "help" : "default";
  if (hover !== P.hover) P.hover = hover;
  if (hover !== null) showTip(P.F[hover], ev.clientX, ev.clientY);
  else hideTip();
}

/** A click on the board moves nobody (the tactics arrange the lines); it is counted for the smoke
 *  test, which checks that. */
function onClick() {
  P.clicks++;
}

export function key(ctx, ev) {
  if (ev.key === "Enter") {
    attack(ctx);
    return true;
  }
  if (ev.key === "Escape") {
    // The attack on its way: the plan stays until the battle's screens take over.
    if (P.busy) return true;
    ctx.close();
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
    pos ? null : h("div.small.dim", f.side === me ? "Not in the line: its tactic left it out." : "Not in their line: it waits on the plot."),
    (pv.starving ?? []).some((x) => x.b === f.b) ? h("div.small.warn", `Starving: ${pv.starving.find((x) => x.b === f.b).pct}% damage dealt.`) : null,
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
  // the pointer's layer over the board (under the panels): tooltips only
  add(ov, h("div.bt-board", { onmousemove: onMove, onmouseleave: () => ((P.hover = null), hideTip()), onclick: onClick }));
  const where = pv.plot?.city?.name ?? (pv.sides?.[1 - me]?.label ? `the ${pv.sides[1 - me].label}` : "the plot");
  const used = slotsUsed(P.place, F);
  const mineIn = F.filter((f) => f.side === me && P.place.has(f.b));
  const mineOut = F.filter((f) => f.side === me && !P.place.has(f.b));
  const theirsIn = F.filter((f) => f.side !== me && P.def.has(f.b));
  const theirsOut = F.filter((f) => f.side !== me && !P.def.has(f.b));
  const watch = ctx.prefs.get("watch", "watch");
  const tac = [tacticOf(me), tacticOf(1 - me)];
  const tacticLine = (side) => h("div.bt-row.small.bt-tactic", { "data-side": side, title: tac[side].help }, h("span.dim", "Tactic: "), h("b", tac[side].name));
  // ---- top left: the attack, your side
  add(
    ui,
    pnl(
      { left: "6px", top: "4px", width: "380px" },
      h("div.bt-row", uiIcon("attack", 16), h("span.bt-h3", `Attack ${where}`)),
      h("div.bt-row", h("span.bt-h3.you", pv.sides?.[me]?.label ?? "Your army"), h("span.dim", `${used} of ${pv.slots[me].total}${pv.slots[me].river_only ? ` (+${pv.slots[me].river_only} river)` : ""} slots · ${pv.lanes[me]} lanes`)),
      h("div.small.dim", { title: "Why this many slots" }, slotSum(pv.slots[me])),
      tacticLine(me),
      h(
        "div.bt-row",
        { style: { marginTop: "2px" } },
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
      tacticLine(1 - me),
      theirsOut.length
        ? h("div.bt-row.small", { title: "Defenders stay in place until annihilated: the plot falls only when none is left" }, h("span.dim", `${theirsOut.length} more wait${theirsOut.length > 1 ? "" : "s"} on the plot`), theirsOut.slice(0, 8).map((f) => tipped(f, headChip(f, pv.sides, { w: 22, h: 22, flip: true }))))
        : null,
      groundLine(pv.ground, pv.plot),
    ),
  );
  // ---- bottom left: the pool's other units (what its tactic left out), the rule
  const benchRows = mineOut.map((f) =>
    tipped(
      f,
      h(
        "div.bt-bench",
        { "data-b": f.b, title: "" },
        headChip(f, pv.sides, { w: 28, h: 26 }),
        h("span.bt-ell", h("b", uname(pv, f.b)), h("br"), h("span.small.dim", `${rowsLine(f)}${f.large ? " · large" : ""} · ATK ${stat(f.a_x100)} · DEF ${stat(f.dv_x100)}`)),
      ),
    ),
  );
  add(
    ui,
    pnl(
      { left: "6px", top: "506px", width: "420px", height: "288px", display: "block" },
      h("div.bt-row", h("span.bt-h4", "Not in the line"), h("span.dim.small", `${mineIn.length} of ${mineIn.length + mineOut.length} in the line`)),
      benchRows.length ? h("div.bt-benchlist", benchRows) : h("div.dim.small", "Every unit is in the line."),
      h("div.bt-rule"),
      h("div.small.dim", "Each side stands as its tactic arranges it. An army's tactic is set on the Armies screen [F5], a garrison's on its city's screen; a change applies from your next turn."),
    ),
  );
  // ---- bottom right: the forecast, Attack
  add(ui, forecastPanel(ctx, { mineIn, theirsIn, theirsOut }));
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
  } else body.push(h("div.bt-row", h("span.bt-h4", "Forecast")), h("div.small.dim", "No forecast."));
  if (fc && P.summons?.length) {
    const each = P.summons.map((x) => `${tyLabel("UnitType", x.unit_type)} (${x.side === me ? "yours" : "theirs"}, falls ${x.fall_pct}%)`);
    body.push(h("div.small", { title: "Called into this battle by a caster in the line; not in the counts above." }, h("span.dim", "Summons: "), each.join(", ")));
  }
  // Supply (docs/SUPPLY.md): the units of starving armies on either side.
  const starving = pv.starving ?? [];
  if (starving.length) {
    const n = [me, 1 - me].map((side) => starving.filter((x) => P.F[x.b]?.side === side).length);
    const pctText = `${starving[0].pct > 0 ? "+" : ""}${starving[0].pct}%`;
    body.push(h("div.small.warn.bt-starving", `Starving: ${n[me]} of yours, ${n[1 - me]} of theirs: ${pctText} damage dealt.`));
  }
  // The sea (docs/NAVAL.md 4.7): the warships that fire from the coast before round 1, one shot
  // each (the forecast counts them).
  const shore = pv.shore ?? [];
  if (shore.length || pv.shore_unseen) {
    const list = (side) => shore.filter((x) => x.side === side).map((x) => `${tyLabel("UnitType", x.unit_type)} (A ${(x.a_x100 / 100).toFixed(1)})`);
    const [mine, theirs] = [list(me), list(1 - me)];
    const unseen = pv.shore_unseen ? `, ${pv.shore_unseen} unseen` : "";
    body.push(h("div.small.bt-shore", { title: "Each fires one shot before round 1; a shot never takes a unit below 1 HP." }, h("span.dim", "Ships fire from the coast: "), `yours: ${mine.length ? mine.join(", ") : "none"}; theirs: ${theirs.length ? theirs.join(", ") : "none"}${unseen}.`));
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
        title: war ? "Declare war and attack [Enter]" : "Attack [Enter]",
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

/** Offsets in [-1, 1], from the middle out. */
const SPREAD = [0, -0.2, 0.2, -0.4, 0.4, -0.6, 0.6, -0.8, 0.8];

export const dev = {
  attack() {
    return P.ctx && attack(P.ctx);
  },
  /** A point in client coordinates over fighter `b`'s figure (inside its hit box, the frontmost
   *  figure there, not under a panel), or null when it is not on the board. */
  point(b) {
    const pos = P.pv && posOf(b);
    if (!pos) return null;
    P.figs = scene(performance.now()).figs;
    const [fx, fy] = unitXY(P.ctx.stage.G, P.F[b], pos);
    const box = figBox(P.F[b], fx, fy);
    const cx = (box.x0 + box.x1) / 2;
    const cy = fy - 40;
    for (const v of SPREAD) {
      for (const u of SPREAD) {
        const x = cx + (u * (box.x1 - box.x0)) / 2;
        const y = cy + (v * (box.y1 - box.y0)) / 2;
        if (figUnder(x, y)?.b !== b) continue;
        const c = clientOf(x, y);
        if (document.elementFromPoint(c.x, c.y)?.closest?.(".bt-board")) return c;
      }
    }
    return null;
  },
  /** The formation shown: your side's `[{unit, lane, row}]`. */
  get formation() {
    return P.pv ? [...P.place.entries()].sort((a, b) => a[0] - b[0]).map(([b, p]) => ({ unit: P.F[b].unit, lane: p.lane, row: p.row })) : null;
  },
  /** Both sides' tactics as named on the board. */
  get tactics() {
    return P.pv ? [tacticOf(me).name, tacticOf(1 - me).name] : null;
  },
  get preview() {
    return P.pv;
  },
  /** The clicks the board took (it moves nobody). */
  get clicks() {
    return P.clicks;
  },
  /** The preview's errors (the engine's reasons it refuses the attack). */
  get errors() {
    return P.errors;
  },
  /** The forecast (null: none). */
  get forecast() {
    return P.fc;
  },
  get busy() {
    return P.busy;
  },
  get ready() {
    return !!P.pv && !P.busy;
  },
};
