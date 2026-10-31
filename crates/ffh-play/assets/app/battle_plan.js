// The battle plan (the battle UI design, DESIGN.md section 2): before an attack, the board with both
// forces in their ranks, who fights and why there are that many (the slots in words), each unit's
// risk, the win chance and the cost of winning, the What-if plans, and the stakes, with the doomed
// struck through. Swaps, the stance and the What-if lines re-ask the source for a preview: every
// number here is the engine's (the preview's forecast), none is worked out by the client.

import { h, toast } from "./dom.js";
import { S } from "./state.js";
import { tyLabel, uname, groupNames, committed, isBack, hitsScreensFirst, x100, pctText, termLabel, termGroup, plotDvTerms, pairOf, slotSum, wallsNote, whatIfLines, stakes, roleKey } from "./battle_log.js";
import { lessonText, targetsText } from "./battle_teach.js";
import { loadArt, layout, pic, faceFlip, roleIcon, arcAt, headChip, uiIcon, pixGlyph, GLYPH, at, btn, add, COLORS } from "./battle_stage.js";

export const NAME = "plan";

const P = { preview: null, board: null, pos: null, alts: null, sel: null, hover: null, settings: false, seq: 0, autoWin: null, busy: false, ctx: null };

const me = 0;

export async function enter(ctx) {
  P.ctx = ctx;
  P.sel = null;
  P.hover = null;
  P.settings = false;
  P.autoWin = null;
  await load(ctx, {});
}

export function leave() {
  P.seq++;
}

/** The units of the plan's force, by id (slot order). */
const forceIds = () => P.preview.force.map((b) => P.preview.fighters[b].unit);
const sameSet = (a, b) => a.length === b.length && a.every((x) => b.includes(x));

/** Asks the source for a preview (`req`: {force, stance}), then for its What-if plans. */
async function load(ctx, req) {
  const seq = ++P.seq;
  P.busy = true;
  render(ctx);
  let preview;
  try {
    preview = await ctx.source.preview({ stance: P.preview?.stance, ...req });
  } finally {
    P.busy = false;
  }
  if (seq !== P.seq) return;
  // No plan (the engine answers a plot the player does not see, or one not next to the
  // attackers, with its errors alone): say why and close.
  if (!preview?.fighters) {
    toast((preview?.errors ?? []).join("; ") || "There is no battle to plan there.", "info");
    ctx.close();
    return;
  }
  const board = boardFighters(preview);
  await loadArt({ fighters: board, plot: preview.plot, from: preview.from, sides: preview.sides });
  if (seq !== P.seq) return;
  // The preview, its board and their places change together (a fighter's `b` is the reply's).
  ctx.stage.me = me;
  ctx.stage.setBoard(preview, 0);
  P.preview = preview;
  P.board = board;
  P.pos = layout(board, { raise: ctx.stage.raise });
  const force = forceIds();
  const auto = preview.auto_pick.map((b) => preview.fighters[b].unit);
  if (sameSet(force, auto)) P.autoWin = preview.forecast?.win_permille ?? null;
  P.alts = null;
  render(ctx);
  ctx.source
    .alternatives({ force, stance: preview.stance })
    .then((alts) => {
      if (seq !== P.seq) return;
      P.alts = alts;
      const own = alts.find((a) => a.kind === "StancePick");
      if (own) P.autoWin = own.win_permille;
      render(ctx);
    })
    .catch((e) => console.warn("battle plan: the What-if plans:", e));
}

/** The fighters as the board shows them: yours in the force's slots, theirs in their pick's (the
 *  units the stance may or may not commit marked `maybe`), the others out of the fight. */
function boardFighters(pv) {
  const pick = pv.defender_pick ?? { kind: "Known", units: pv.fighters.filter((f) => f.side !== me).map((f) => f.b) };
  const sure = pick.kind === "Known" ? pick.units : pick.sure ?? [];
  const maybe = pick.kind === "Known" ? [] : pick.maybe ?? [];
  const order = [...sure, ...maybe];
  return pv.fighters.map((f) => {
    if (f.side === me) return { ...f, slot: pv.force.includes(f.b) ? pv.force.indexOf(f.b) : null };
    const k = order.indexOf(f.b);
    return { ...f, slot: k >= 0 ? k : null, maybe: maybe.includes(f.b) };
  });
}

const fighterOfUnit = (id) => P.board.find((f) => f.unit === id);

// ---------------------------------------------------------------------------------------------
// Actions

function swap(ctx, outId, inId) {
  const force = forceIds().map((x) => (x === outId ? inId : x));
  P.sel = null;
  load(ctx, { force });
}

function setStance(ctx, stance) {
  P.sel = null;
  P.autoWin = null;
  load(ctx, { stance, force: undefined });
}

function adopt(ctx, alt) {
  P.sel = null;
  load(ctx, { force: alt.force, stance: alt.stance });
}

function clickUnit(ctx, id) {
  const f = fighterOfUnit(id);
  if (!f || f.side !== me || P.busy) return;
  const inForce = committed(f);
  if (P.sel === null || P.sel === id) {
    P.sel = P.sel === id ? null : id;
    render(ctx);
    return;
  }
  const selIn = committed(fighterOfUnit(P.sel));
  if (selIn !== inForce) swap(ctx, selIn ? P.sel : id, selIn ? id : P.sel);
  else {
    P.sel = id;
    render(ctx);
  }
}

async function attack(ctx) {
  if (P.busy || !P.preview || P.preview.errors?.length) return;
  P.busy = true;
  render(ctx);
  let r = null;
  try {
    r = await ctx.source.attack({ force: forceIds(), stance: P.preview.stance, declare_war: P.preview.declare_war !== null && P.preview.declare_war !== undefined });
  } catch (e) {
    console.warn("battle plan: the attack:", e);
  } finally {
    P.busy = false;
  }
  // Not made (refused, a war not declared): the plan stays.
  if (!r?.log) return render(ctx);
  ctx.ledger.recordPlan();
  if (ctx.ledger.watchMode() === "result") ctx.toReport(r.log);
  else ctx.toWatch(r.log);
}

/** The stances in the data's order (indices, or names when the preview names them). */
function stanceList(cur) {
  const tbl = S.data?.tables?.StanceType ?? [];
  if (!tbl.length) return [cur];
  return typeof cur === "string" ? tbl.map((e) => e.type) : tbl.map((_, i) => i);
}

// ---------------------------------------------------------------------------------------------
// The board

function scene(now) {
  const F = P.board;
  const pos = P.pos;
  const sides = P.preview.sides;
  const figs = [];
  const arrows = [];
  const numbered = [];
  const rings = [];
  for (const f of F) {
    const p = pos[f.b];
    const out = !committed(f);
    const fig = {
      x: p.x,
      y: p.y,
      side: f.side,
      pic: pic(f, sides, "idle", now + f.b * 170, true),
      flip: faceFlip(f, sides),
      alpha: out ? 0.62 : f.maybe ? 0.55 : 1,
      ringAlpha: out ? 0.35 : 1,
      roleIcon: roleIcon(f),
      plate: f.maybe,
    };
    if (P.sel === f.unit) fig.sel = COLORS.goldLt;
    else if (P.sel !== null && f.side === me && committed(fighterOfUnit(P.sel)) === out) fig.sel = COLORS.cyan;
    if (P.hover === f.unit) {
      fig.glow = COLORS.goldLt;
      fig.glowA = 0.55;
    }
    figs.push(fig);
  }
  const hv = P.hover !== null ? fighterOfUnit(P.hover) : null;
  // The flankers' way round: dashed to their screen, dotted on to their back rank (until the
  // viewer has seen a rider ride round a few times).
  if (!hv && P.ctx.ledger.seen("flank_around") < 3) {
    for (const f of F.filter((x) => x.side === me && committed(x) && hitsScreensFirst(x))) {
      const scr = F.filter((x) => x.side !== me && committed(x) && x.screen);
      const back = F.filter((x) => x.side !== me && committed(x) && isBack(x));
      if (!scr.length || !back.length) continue;
      const a = pos[f.b];
      const s = pos[scr[0].b];
      const bc = centre(back.map((x) => pos[x.b]));
      arrows.push({ x0: a.x + 10, y0: a.y - 70, x1: s.x - 14, y1: s.y - 74, color: COLORS.you, dash: [8, 6], bend: 40, width: 3, alpha: 0.9 });
      arrows.push({ x0: s.x + 10, y0: s.y - 84, x1: bc.x - 18, y1: bc.y - 78, color: COLORS.you, dash: [3, 6], bend: 30, width: 3, alpha: 0.75 });
    }
  }
  // A unit pointed at: its targeting order as numbered arrows, each class ringed.
  if (hv && committed(hv)) {
    const enemies = F.filter((x) => x.side !== hv.side && committed(x));
    const from = pos[hv.b];
    const done = new Set();
    let k = 0;
    for (const cls of hv.targets ?? []) {
      let set = enemies.filter((e) => (cls === "FRONT" ? !isBack(e) : cls === "BACK" ? isBack(e) : cls === "SCREEN" ? e.screen : ["arcane", "divine"].includes(roleKey(e)) || e.hero));
      set = set.filter((e) => !done.has(e.b));
      if (!set.length) continue;
      k++;
      set.forEach((e) => done.add(e.b));
      const c = centre(set.map((e) => pos[e.b]));
      const shielded = cls === "BACK" && !hv.ignores_shield;
      const x1 = c.x + (hv.side ? 30 : -30);
      arrows.push({ x0: from.x, y0: from.y - 60, x1, y1: c.y - 60, color: k === 1 ? COLORS.goldLt : COLORS.gold, dash: k === 1 ? [] : [7, 5], bend: 30 + k * 26, width: k === 1 ? 4 : 3, alpha: shielded ? 0.6 : 1 });
      const m = arcAt(from.x, from.y - 60, x1, c.y - 60, 30 + k * 26, 0.55);
      numbered.push({ x: m.x, y: m.y, n: k });
      for (const e of set) rings.push({ x: pos[e.b].x, y: pos[e.b].y, color: k === 1 ? COLORS.goldLt : COLORS.gold, alpha: 0.8, dash: k !== 1 });
    }
  }
  const bands = [0, 1].filter((s) => F.some((f) => f.side === s && committed(f) && !isBack(f))).map((side) => ({ side }));
  return { figs, arrows, numbered, rings, bands };
}

function centre(ps) {
  return ps.reduce((a, p) => ({ x: a.x + p.x / ps.length, y: a.y + p.y / ps.length }), { x: 0, y: 0 });
}

export function frame(ctx, _dt, now) {
  if (P.preview && P.pos) ctx.stage.draw(scene(now));
}

export function key(ctx, ev) {
  if (ev.key === "Enter") {
    attack(ctx);
    return true;
  }
  if (ev.key === "Escape") {
    if (P.settings) P.settings = false;
    else if (P.sel !== null) P.sel = null;
    else {
      ctx.close();
      return true;
    }
    render(ctx);
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------------------------
// The panels

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);
const sideName = (side) => P.preview.sides?.[side]?.label ?? (side === me ? "Your army" : "Them");

/** Slot pips: `used` lit, the rest of `total` open, up to `base` crossed (the slots the place takes). */
function pips(base, total, used, color) {
  const row = h("span.bt-pips");
  for (let k = 0; k < Math.max(base, total); k++) {
    const cls = k < used ? "lit" : k < total ? "open" : "lost";
    add(row, h(`i.${cls}`, { style: k < used ? { background: color } : undefined }));
  }
  return row;
}

const PLACE_ICON = { Hills: "terrain", Terrain: "terrain", Feature: "terrain", CityDefense: "defense", Fortify: "fortify" };

function render(ctx) {
  const ui = ctx.ui;
  const ov = ctx.ov;
  ui.replaceChildren();
  ov.replaceChildren();
  const pv = P.preview;
  if (!pv || !P.pos) {
    add(ui, pnl({ left: "540px", top: "360px", width: "200px", textAlign: "center" }, h("span.dim", "Planning…")));
    return;
  }
  const F = P.board;
  const pos = P.pos;
  const fc = pv.forecast ?? {};
  const att = F.filter((f) => f.side === me && committed(f));
  const def = F.filter((f) => f.side !== me && committed(f));
  const res = F.filter((f) => f.side === me && !committed(f));
  const where = pv.plot?.city?.name ?? "the plot";
  // ---- the top bar
  const top = h("div.bt-top");
  add(top, 
    pnl({ left: "0", width: "250px" }, uiIcon("attack", 16), h("span.bt-h3", `Attack ${where}`), btn(uiIcon("settings", 16), { cls: "sm", title: "Settings", onclick: () => ((P.settings = !P.settings), render(ctx)) })),
    pnl(
      { left: "256px", width: "610px", justifyContent: "center" },
      h("span.dim", "Their Dv"),
      plotDvTerms(F, 1 - me).map((t) => h("span.frost", uiIcon(PLACE_ICON[t.src] ?? "defense", 16), ` ${termLabel(t)} ${pctText(t.pct)}`)),
      plotDvTerms(F, 1 - me).length ? null : h("span.dim", "none from the place"),
    ),
    pnl(
      { left: "872px", right: "0" },
      h("span.dim", "Watch"),
      ["full", "beats", "result"].map((m) =>
        btn(m === "full" ? "Full" : m === "beats" ? "Beats" : "Result", {
          cls: "sm",
          on: ctx.ledger.watchMode() === m,
          title: m === "full" ? "Every round at the same pace, with the lessons" : m === "beats" ? "Quiet rounds fast, the beats at a watchable pace" : "Straight to the report",
          onclick: () => (ctx.ledger.set("watch_mode", m), render(ctx)),
        }),
      ),
    ),
  );
  add(ui, top);
  // ---- the side headers: slots as pips, the sum in words
  const slotsA = pv.slots[me];
  const slotsD = pv.slots[1 - me];
  const sumDim = ctx.ledger.level("slots") === "icon";
  const walls = wallsNote(slotsA);
  add(ui, 
    pnl(
      { left: "6px", top: "66px", width: "336px" },
      h("div.bt-row", h("span.bt-h3.you", sideName(me)), h("span.dim", `${att.length} of ${att.length + res.length} fight`)),
      h("div.bt-row", pips(slotsA.base, slotsA.total, att.length, "#5f8ff0")),
      h(`div${sumDim ? ".dim" : ""}`, slotSum(slotsA).replace(/= (\d+)$/, "= "), h("b", String(slotsA.total))),
      walls ? h("div.dim", walls) : null,
      slotsA.river_only ? h("div.dim", `${slotsA.river_only} more for units that cross rivers`) : null,
    ),
  );
  const side1 = pv.sides?.[1 - me] ?? {};
  const nDef = F.filter((f) => f.side !== me).length;
  const stanceLine = side1.stance_known
    ? `Stance: ${tyLabel("StanceType", side1.stance)}`
    : nDef <= slotsD.total
      ? `Stance unseen: all ${nDef} fight`
      : `Stance unseen: ${slotsD.total} of ${nDef} fight`;
  add(ui, 
    pnl(
      { right: "6px", top: "66px", width: "320px" },
      h("div.bt-row", h("span.bt-h3.them", sideName(1 - me)), h("span.dim", `${def.length} of ${slotsD.total} slots`)),
      h("div.bt-row", pips(slotsD.base, slotsD.total, def.length, "#f15b47")),
      h("div.dim", stanceLine),
    ),
  );
  // ---- over your units: their risk and their withdrawal; roles sit on the rings
  for (const f of att) {
    const p = pos[f.b];
    const per = (fc.per ?? []).find((x) => x.b === f.b);
    const chips = [];
    if (per && per.fall_pct >= 40) chips.push(h("span.bt-chip.risk", { title: `falls in ${per.fall_pct}% of the forecast's battles` }, pixGlyph(GLYPH.skull, "#f2e6d0", 2), `${per.fall_pct}%`));
    if (f.withdraw_pct) chips.push(h("span.bt-chip.wd", { title: "one roll to pull out below 40% HP" }, `↩${f.withdraw_pct}%`));
    if (chips.length) at(ov, p.x, p.y - 124, h("div.bt-ubar", chips));
  }
  for (const f of F.filter((x) => x.maybe)) at(ov, pos[f.b].x, pos[f.b].y - 124, h("div.bt-ubar", h("span.bt-chip.gone", "may fight")));
  if (res.length) {
    const rx = Math.min(...res.map((f) => pos[f.b].x));
    const ry = Math.max(...res.map((f) => pos[f.b].y));
    at(ov, rx - 40, ry + 20, h("div.bt-tag", h("span.you", "Stays out"), h("br"), h("span.dim", "safe either way")));
  }
  // First-battle notes on the board (gold), faded later.
  if (ctx.ledger.level("shield_plan") !== "icon" && att.some((f) => isBack(f))) {
    at(ov, 376, 478, h("div.bt-tag", { title: lessonText("shield_plan", pv).join(": ") }, pixGlyph(GLYPH.shield, COLORS.you, 2), " ", h("span.gold", "Shielded"), " while the front stands"));
  }
  if (ctx.ledger.level("roles") !== "icon") {
    const roles = [...new Map(F.map((f) => [roleKey(f), f])).values()];
    at(ov, 8, 540, h("div.bt-tag", { title: lessonText("roles", pv).join(": ") }, h("span.gold", "Roles"), roles.map((f) => h("span", " ", uiIcon(roleIcon(f), 16), ` ${tyLabel("CombatRoleType", f.role)}`))));
  }
  // ---- the card of the unit pointed at
  if (P.hover !== null && fighterOfUnit(P.hover)) add(ui, unitCard(fighterOfUnit(P.hover)));
  // ---- the dock
  dock(ctx, ui, { att, def, res, fc, where });
  if (P.settings) add(ui, settingsPanel(ctx));
  // hit areas over the units
  for (const f of F) {
    const p = pos[f.b];
    const hit = h("div.bt-hit", {
      title: uname(P.preview, f.b),
      style: { left: `${p.x - 30}px`, top: `${p.y - 104}px`, width: "60px", height: "108px", cursor: f.side === me ? "pointer" : "help" },
      onmouseenter: () => {
        P.hover = f.unit;
        render(ctx);
      },
      onmouseleave: () => {
        if (P.hover !== f.unit) return;
        P.hover = null;
        render(ctx);
      },
      onclick: () => clickUnit(ctx, f.unit),
    });
    add(ov, hit);
  }
}

/** A or Dv as stacked segments by source, on one scale of `scaleMax` (x100) over `wpx` px. */
export function partsBar(stat, kind, scaleMax, wpx, extra = []) {
  const baseRole = Math.floor((stat.base_x100 * stat.role_pct) / 100);
  const terms = [...stat.terms, ...extra];
  const segs = [{ v: baseRole, cls: `base-${kind}`, l: "base × role" }];
  for (const t of terms) segs.push({ v: Math.floor((baseRole * Math.max(0, t.pct)) / 100), cls: `${kind}-${termGroup(t)}`, l: `${termLabel(t)} ${pctText(t.pct)}` });
  const total = kind === "a" ? stat.total_x100 : stat.total_x100;
  const bar = h("div.bt-parts", { style: { width: `${Math.max(4, Math.round((wpx * total) / scaleMax))}px` } });
  for (const s of segs) add(bar, h(`i.${s.cls}`, { title: s.l, style: { width: `${Math.max(2, Math.round((wpx * s.v) / scaleMax))}px` } }));
  return bar;
}

function unitCard(f) {
  const pv = P.preview;
  const mine = f.side === me;
  const stat = mine ? f.a : f.dv;
  const baseRole = Math.max(1, Math.floor((stat.base_x100 * stat.role_pct) / 100));
  const mult = (stat.total_x100 / baseRole).toFixed(1);
  const role = tyLabel("CombatRoleType", f.role);
  const card = h("div.bt-pnl.bt-card", { style: { top: "66px", width: "372px", [mine ? "right" : "left"]: "6px", zIndex: 6 } });
  add(card, 
    h("div.bt-row", h(`span.bt-h3.${mine ? "you" : "them"}`, uname(pv, f.b)), h("span.dim", `${role} · L${f.level}`)),
    (f.promotions ?? []).length ? h("div.dim.small", f.promotions.map((p) => tyLabel("PromotionType", p)).join(", ")) : null,
    h(
      "div.bt-kv",
      h(`span.${mine ? "amber" : "frost"}`, mine ? "Attack A " : "Defense Dv ", h("b", x100(stat.total_x100))),
      h("span.dim", `= ${x100(stat.base_x100)} × ${(stat.role_pct / 100).toFixed(1)} ${role.toLowerCase()}${mult !== "1.0" ? ` × ${mult}` : ""}`),
    ),
    partsBar(stat, mine ? "a" : "dv", 1800, 340),
    h("div.small.dim", stat.terms.map((t) => `${termLabel(t)} ${pctText(t.pct)}`).join(" · ") || "no bonuses"),
    h("div.gold.small", { style: { marginTop: "3px" } }, targetsText(f)),
  );
  // per hit: what it does to each of theirs, or what each of yours does to it
  const rows = [];
  const seen = new Set();
  for (const e of P.board.filter((x) => x.side !== f.side && committed(x))) {
    const p = mine ? pairOf(pv.pairs, f.b, e.b) : pairOf(pv.pairs, e.b, f.b);
    if (!p) continue;
    const victimHp = (mine ? e.hp_start : f.hp_start) / 100;
    const n = uname(pv, e.b);
    if (seen.has(n)) continue;
    seen.add(n);
    const vs = (mine ? [...p.a_extra, ...p.dv_extra] : [...p.dv_extra, ...p.a_extra]).filter((t) => termGroup(t) === "vs");
    rows.push(h("div.bt-kv", h("span", n, vs.length ? h("span.cyan", { title: vs.map((t) => `${termLabel(t)} ${pctText(t.pct)}`).join(", ") }, " ±vs") : null), h("span", h("b", (p.mean / 100).toFixed(1)), h("span.dim", ` · ${Math.ceil(victimHp / Math.max(0.01, p.mean / 100))}`))));
  }
  add(card, h("div.bt-rule"), h("div.bt-kv", h("span.bt-h4", mine ? "Its hits on them" : "Your hits on it"), h("span.dim.small", "a hit · hits to kill")), rows);
  if (f.withdraw_pct) add(card, h("div.lilac.small", `Below 40% HP: one roll to pull out, ${f.withdraw_pct}%`));
  if (f.volleys) add(card, h("div.small.dim", `First strike${f.volleys > 1 ? `s (${f.volleys})` : ""}: shoots in the opening volley`));
  if (f.def_strike) add(card, h("div.small.dim", `Strikes each attacker as it comes in: ${f.def_strike[0]}%, up to ${f.def_strike[1]}`));
  return card;
}

function dock(ctx, ui, { att, def, res, fc, where }) {
  const pv = P.preview;
  const F = P.board;
  const stanceName = (s) => tyLabel("StanceType", s);
  // ---- left: the stance and the What-if plans (each a real estimate, one click to adopt)
  const list = stanceList(pv.stance);
  const step = (d) => list[(list.indexOf(pv.stance) + d + list.length) % list.length];
  const help = S.data?.tables?.StanceType?.[typeof pv.stance === "number" ? pv.stance : list.indexOf(pv.stance)]?.help ?? "";
  const left = h(
    "div.bt-pnl",
    { style: { left: "6px", top: "578px", width: "430px", height: "216px" } },
    h("div.bt-row", h("span.bt-h4", "Stance"), btn("◂", { cls: "sm", title: "The previous stance's pick", onclick: () => setStance(ctx, step(-1)) }), h("b", stanceName(pv.stance)), btn("▸", { cls: "sm", title: "The next stance's pick", onclick: () => setStance(ctx, step(1)) })),
    h("div.dim.small.bt-gloss", plainHelp(help)),
    h("div.bt-h4", "What if"),
  );
  if (!pv.forecast) add(left, h("div.dim.small", "The What-if plans come with the estimate."));
  else if (!P.alts) add(left, h("div.dim", "estimating…"));
  else {
    const lines = whatIfLines(pv, P.alts, stanceName).slice(0, 4);
    const win = Math.round((fc.win_permille ?? 0) / 10);
    for (const l of lines) {
      const cur = l.kind === "Current";
      const w = Math.round(l.win_permille / 10);
      const also = l.also.length ? h("span.dim.small", ` = ${l.also[0]}${l.also.length > 1 ? ` +${l.also.length - 1}` : ""}`) : null;
      add(left, 
        h(
          `div.bt-line${cur ? ".cur" : ""}`,
          { role: cur ? undefined : "button", tabIndex: cur ? -1 : 0, title: cur ? "The plan you have" : "Adopt this plan", onclick: cur ? undefined : () => adopt(ctx, l) },
          h("span.bt-ell", `${cur ? "▸ " : ""}${l.label}`, also),
          h(`span.pct${w >= win + 3 ? ".good" : w <= win - 3 ? ".bad" : ""}`, `${w}%`),
        ),
      );
    }
  }
  // ---- middle: the forecast
  const win = Math.round((fc.win_permille ?? 0) / 10);
  const draw = Math.round((fc.draw_permille ?? 0) / 10);
  const likely = att.filter((f) => ((fc.per ?? []).find((x) => x.b === f.b)?.fall_pct ?? 0) >= 40).sort((a, b) => perOf(fc, b).fall_pct - perOf(fc, a).fall_pct);
  const mid = pv.forecast
    ? h(
        "div.bt-pnl",
        { style: { left: "442px", top: "578px", width: "410px", height: "216px" } },
        h("div.bt-row", h(`span.bt-big.${win >= 70 ? "good" : win >= 40 ? "warn" : "bad"}`, `${win}%`), h("span", pv.plot?.city ? `to take ${where}` : "to win", h("br"), h("span.dim", fc.rounds ? `about ${fc.rounds.p50} rounds (${fc.rounds.p10}–${fc.rounds.p90})` : ""))),
        h("div.bt-wl", { style: { margin: "4px 0" } }, h("i", { style: { width: `${win}%` } }), draw ? h("i.draw", { style: { left: `${win}%`, width: `${draw}%` } }) : null),
        h("div", "Cost if you win: ", h("b", fc.losses_if_win_x10 === null || fc.losses_if_win_x10 === undefined ? "–" : `~${(fc.losses_if_win_x10 / 10).toFixed(1)}`), h("span.dim", ` of ${att.length} units`)),
        likely.length
          ? h("div.bt-row", { style: { marginTop: "2px" } }, h("span.dim", "Likely to fall"), likely.map((f) => h("span.bt-col", headChip(f, pv.sides), h("span.bt-chip.risk.tiny", `${perOf(fc, f).fall_pct}%`))))
          : h("div.dim", "No unit is likely to fall."),
      )
    : // The engine's forecast (the sampled estimate) is not built yet: the plan shows the force
      // and the stakes, and says so.
      h(
        "div.bt-pnl.bt-estimate-pending",
        { style: { left: "442px", top: "578px", width: "410px", height: "216px" } },
        h("div.bt-row", h("span.bt-big.warn", "?"), h("span", pv.plot?.city ? `to take ${where}` : "to win", h("br"), h("span.dim", "estimate coming"))),
        h("div.small.dim", { style: { margin: "4px 0" } }, "The win chance, the rounds and the cost come with the engine's estimate. Until then, judge by the A and Dv of each pair: point at a unit."),
        h("div", `${att.length} of yours fight`, h("span.dim", ` (${tyLabel("StanceType", pv.stance)}'s pick)`), ` against ${def.length}${pv.defender_pick?.kind === "Likely" ? " or more" : ""}.`),
      );
  const force = forceIds();
  const auto = pv.auto_pick.map((b) => pv.fighters[b].unit);
  if (!sameSet(force, auto)) {
    const outs = auto.filter((x) => !force.includes(x)).map((id) => uname(pv, fighterOfUnit(id).b));
    const ins = force.filter((x) => !auto.includes(x)).map((id) => uname(pv, fighterOfUnit(id).b));
    add(mid, 
      h(
        "div.small.dim",
        `Your swap: ${outs.join(", ")} out, ${ins.join(", ")} in${P.autoWin !== null ? ` · was ${Math.round(P.autoWin / 10)}%` : ""} `,
        btn("auto-pick", { cls: "tiny", title: "Back to the stance's own pick", onclick: () => setStance(ctx, pv.stance) }),
      ),
    );
  }
  if (pv.declare_war !== null && pv.declare_war !== undefined) add(mid, h("div.warn.small", "Attacking declares war."));
  for (const e of pv.errors ?? []) add(mid, h("div.bad.small", e));
  // ---- right: the stakes, then Attack
  const st = stakes(pv);
  const strike = (list) => list.map((f) => h(`span.bt-struck${f.side === me ? ".mine" : ""}`, headChip(f, pv.sides, { w: 24, h: 24, flip: f.side !== me })));
  const right = h("div.bt-pnl", { style: { left: "858px", top: "578px", right: "6px", height: "216px", padding: "2px 10px" } });
  const wonText = st.defendersLeft
    ? `All ${st.defenders.length} defenders die. ${st.defendersLeft} stay out: ${where} holds.`
    : `All ${st.defenders.length} defenders die. ${pv.plot?.city ? `${where} is yours; your army moves in.` : "The plot is yours."}`;
  const lostText = `All ${st.force.length} you send die${st.withdrawers.length ? `; only ${groupNames(pv, st.withdrawers.map((f) => f.b))} may pull out` : ""}.`;
  const est = !!pv.forecast;
  const war = pv.declare_war !== null && pv.declare_war !== undefined;
  add(right,
    h("div.bt-stake", h("div.bt-t", h("b.good", est ? `If you win · ${st.win}%` : "If you win"), strike(st.defenders)), h("div.small", wonText)),
    h("div.bt-stake", h("div.bt-t", h("b.bad", est ? `If you lose · ${st.lose}%` : "If you lose"), strike(st.force)), h("div.small", lostText)),
    st.draw > 1 ? h("div.bt-stake", h("div.bt-t", h("b.warn", `A draw · ${st.draw}%`)), h("div.small", "Forty rounds: both sides draw back; nobody's losses are total.")) : null,
    h("div.bt-stake", h("div.bt-t", h("b.you", "Either way"), st.reserve.map((f) => headChip(f, pv.sides, { w: 24, h: 24 }))), h("div.small", st.reserve.length ? `${groupNames(pv, st.reserve.map((f) => f.b))} stay out, unhurt.` : "Everyone fights.")),
    h(
      "div.bt-row",
      { style: { justifyContent: "flex-end", gap: "8px" } },
      btn("Cancel", { title: "Back to the map [Esc]", onclick: () => ctx.close() }),
      btn([uiIcon("attack", 16), war ? " Declare war and attack" : " Attack"], { cls: "primary", title: war ? "Declare war and attack with this force [Enter]" : "Attack with this force [Enter]", disabled: P.busy || !!pv.errors?.length, onclick: () => attack(ctx) }),
    ),
  );
  add(ui, left, mid, right);
}

const perOf = (fc, f) => (fc.per ?? []).find((x) => x.b === f.b) ?? { fall_pct: 0, withdraw_pct: 0 };

/** A help text without Civ4 markup. */
function plainHelp(s) {
  return String(s ?? "").replace(/\[[^\]]*\]/g, "").trim();
}

function settingsPanel(ctx) {
  const L = ctx.ledger;
  const lv = L.viewer();
  const showPlan = L.setting("show_plan", null) ?? "default";
  const pause = L.setting("pause_lessons", true);
  return h(
    "div.bt-pnl",
    { style: { right: "6px", top: "66px", width: "380px", zIndex: 8 } },
    h("div.bt-h4", "Teaching"),
    h(
      "div.bt-row",
      [
        ["new", "New player"],
        ["some", "Some battles"],
        ["veteran", "Veteran"],
      ].map(([k, label]) =>
        btn(label, {
          cls: "sm",
          on: lv === k,
          onclick: () => {
            L.setViewer(k);
            P.settings = false;
            render(ctx);
          },
        }),
      ),
    ),
    h("div.small.dim", { style: { margin: "4px 0" } }, "A lesson shows in full the first time, short the second, then as a ? marker. Choosing resets what this browser remembers."),
    h("div.bt-row", btn("Pause on new rules", { cls: "sm", on: pause, onclick: () => (L.set("pause_lessons", !pause), render(ctx)) })),
    h("div.bt-h4", "Show the plan"),
    h(
      "div.bt-row",
      [
        ["always", "Always"],
        ["below75", "Below 75%"],
        ["never", "Never"],
      ].map(([k, label]) => btn(label, { cls: "sm", on: showPlan === k, onclick: () => (L.set("show_plan", k), render(ctx)) })),
    ),
    h("div.small.dim", "Until set: always for your first five attacks, then below 75%."),
  );
}

/** Hooks for the screenshot and smoke scripts. */
export const dev = {
  hover(id) {
    P.hover = id;
    if (P.ctx) render(P.ctx);
  },
  click(id) {
    if (P.ctx) clickUnit(P.ctx, id);
  },
  attack() {
    return P.ctx && attack(P.ctx);
  },
  pos(id) {
    const f = fighterOfUnit(id);
    return f && P.pos[f.b];
  },
  get preview() {
    return P.preview;
  },
  get ready() {
    return !!P.preview && !P.busy && !!P.alts;
  },
};
