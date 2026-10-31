// The battle report (the battle UI design, DESIGN.md section 4): the board frozen where the battle
// ended (the fallen where they fell, who pulled out off the board, the survivors with their XP),
// each unit's fate, the beat rounds (a line replays from there), the plan checked against what
// happened, what decided it and what to try next time, and the rules the viewer has seen. Counts
// the battle in the teaching ledger once.

import { h } from "./dom.js";
import { buildScript, viewerSide, uname, hpOf, committed, fates, planChecks, beatLines, decidingFacts, dealtBy, tyLabel, possessive, listNames } from "./battle_log.js";
import { CODEX, resultWords } from "./battle_teach.js";
import { loadArt, layout, exitPos, pic, deadPic, faceFlip, roleIcon, headChip, uiIcon, pixGlyph, GLYPH, at, btn, add, hpBar, partsEl, COLORS } from "./battle_stage.js";

export const NAME = "report";

const DY = -70;
const R = { ctx: null, log: null, script: null, pos: null, exits: new Map(), newly: new Set(), ready: false };
/** The logs already counted in the ledger (a replay is not another battle). */
const counted = new WeakSet();

export async function enter(ctx, log, { script, shown } = {}) {
  R.ctx = ctx;
  R.ready = false;
  R.log = log;
  ctx.stage.me = viewerSide(log);
  await loadArt({ fighters: log.fighters, plot: log.plot, from: log.from, sides: log.sides });
  if (R.log !== log) return;
  R.script = script ?? buildScript(log, { mode: "full" });
  if (!counted.has(log)) {
    counted.add(log);
    R.newly = ctx.ledger.recordBattle(shown ?? [], log);
  }
  ctx.stage.setBoard(log, DY);
  R.pos = layout(log.fighters, { dy: DY, squeeze: 0.72, raise: ctx.stage.raise });
  R.exits = new Map();
  const k = [0, 0];
  for (const f of log.fighters) {
    if (R.script.final.out[f.b] || R.script.final.sat[f.b]) {
      const e = exitPos(f, log.fighters, k[f.side]++, DY);
      R.exits.set(f.b, { x: e.x + (f.side === 0 ? 40 : -40), y: e.y + 20 });
    }
  }
  render(ctx);
  R.ready = true;
}

export function leave() {
  R.ready = false;
}

export function key(ctx, ev) {
  if (ev.key === "Escape" || ev.key === "Enter") {
    ctx.close();
    return true;
  }
  return false;
}

export function frame(ctx, _dt, now) {
  if (!R.ready) return;
  const L = R.log;
  const F = L.fighters;
  const st = R.script.final;
  const sides = L.sides;
  const figs = [];
  for (const f of F) {
    const p = R.pos[f.b];
    const fig = { x: p.x, y: p.y, side: f.side, flip: faceFlip(f, sides), pic: pic(f, sides, "idle", now + f.b * 170, true), roleIcon: roleIcon(f) };
    if (!committed(f)) Object.assign(fig, { alpha: 0.75, ringAlpha: 0.4 });
    else if (st.dead[f.b]) {
      const dp = deadPic(f, sides);
      Object.assign(fig, dp ? { pic: dp, alpha: 0.85, ringAlpha: 0.2 } : { fallen: true, pic: pic(f, sides), alpha: 0.85, ringAlpha: 0.2 });
    } else if (R.exits.has(f.b)) Object.assign(fig, R.exits.get(f.b), { alpha: 0.9, ringAlpha: 0.45 });
    figs.push(fig);
  }
  const arrows = [];
  const o = L.outcome;
  const taken = o.plot_taken && o.result === "AttackerWon";
  if (taken) {
    const surv = F.filter((f) => f.side === 0 && committed(f) && !st.dead[f.b] && !st.out[f.b]);
    if (surv.length) {
      const c = surv.reduce((a, f) => ({ x: a.x + R.pos[f.b].x / surv.length, y: a.y + R.pos[f.b].y / surv.length }), { x: 0, y: 0 });
      const city = ctx.stage.cityPoint();
      arrows.push({ x0: c.x + 40, y0: c.y - 40, x1: city.x - 20, y1: city.y + 10, color: COLORS.goldLt, dash: [10, 6], width: 4, bend: 50 });
    }
  }
  ctx.stage.draw({ figs, arrows, bands: [], cityFlag: taken && L.plot?.city ? (viewerSide(L) === 0 ? COLORS.azure[0] : COLORS.gules[0]) : null });
}

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);

function render(ctx) {
  const ui = ctx.ui;
  const ov = ctx.ov;
  ui.replaceChildren();
  ov.replaceChildren();
  const L = R.log;
  const F = L.fighters;
  const me = viewerSide(L);
  const st = R.script.final;
  const o = L.outcome;
  const rw = resultWords(L);
  const where = L.plot?.city?.name;
  // ---- the ribbon
  add(ui, 
    h(
      "div.bt-top",
      pnl({ left: "0", width: "300px" }, uiIcon("attack", 16), h("span.bt-h3", where ? `Battle of ${where}` : `Battle at ${L.plot?.x ?? "?"}, ${L.plot?.y ?? "?"}`)),
      ribbon(rw, o),
      pnl({ left: "962px", right: "0", justifyContent: "flex-end" }, btn("Replay", { title: "Watch it again", onclick: () => ctx.toWatch(L, { si: 0 }) }), btn("Continue", { cls: "primary", title: "Back to the map [Enter]", onclick: () => ctx.close() })),
    ),
  );
  // ---- tags on the frozen board
  const xp = {};
  for (const e of L.events) if (e.kind === "Xp") xp[e.unit] = (xp[e.unit] ?? 0) + e.amount;
  for (const f of F) {
    if (!committed(f)) continue;
    const p = R.exits.get(f.b) ?? R.pos[f.b];
    let tag;
    if (st.dead[f.b]) tag = h("span.bt-chip.gone", pixGlyph(GLYPH.skull, "#f2e6d0", 2), ` R${st.diedIn[f.b] || "op"}`);
    else if (st.out[f.b]) tag = h("span.bt-chip.wd", `↩ R${st.wdIn[f.b]}`);
    else if (st.sat[f.b]) tag = h("span.bt-chip.gone", "afraid");
    else tag = [hpBar(hpOf(st.hp[f.b]), f.max_hp, { w: 36 }), xp[f.b] ? h("span.bt-chip.xp", `+${xp[f.b]}★`) : null];
    const ty = p.y - (st.dead[f.b] ? 40 : 124);
    at(ov, p.x, ty < 66 ? p.y + 8 : ty, h("div.bt-ubar", tag));
  }
  const res = F.filter((f) => !committed(f) && f.side === me);
  if (res.length) at(ov, 40, Math.max(70, Math.min(...res.map((f) => R.pos[f.b].y)) - 140), h("div.bt-tag", h("span.you", "Stayed out"), " ", h("span.dim", "untouched")));
  if (o.plot_taken && where) {
    const c = ctx.stage.cityPoint();
    at(ov, c.x - 10, c.y + 46, h("div.bt-tag", h("span.bt-swatch", { style: { background: me === 0 ? COLORS.azure[0] : COLORS.gules[0] } }), ` ${where} `, h(me === 0 ? "span.you" : "span.them", me === 0 ? "yours" : "theirs")));
  }
  // ---- panel 1: each unit's fate, their side, the codex
  const rows = fates(L, st, me).map((x) => {
    const f = F[x.b];
    let fate;
    if (x.kind === "out") fate = [h("span.you", "stayed out"), h("span.dim.small", " untouched")];
    else if (x.kind === "sat") fate = [h("span.dim", "sat out"), h("span.dim.small", " afraid")];
    else if (x.kind === "dead") fate = [h("span.bad", `fell R${x.round || "op"}`), h("span.dim.small", ` ${x.killer !== null ? F[x.killer]?.name ?? "" : ""}${x.roll !== null ? ` · rolled ${x.roll}≥${x.chance}` : ""}`)];
    else if (x.kind === "withdrew") fate = [h("span.lilac", `out R${x.round}`), h("span.dim.small", ` rolled ${x.roll}<${x.chance}`)];
    else fate = x.untouched ? [h("span.good", "no damage")] : [hpBar(hpOf(x.hp), f.max_hp, { w: 40 }), h("span.dim", ` ${hpOf(x.hp)} left`)];
    return h(`div.bt-fate${x.kind === "dead" ? ".dead" : ""}`, headChip(f, L.sides, { w: 28, h: 24, dead: x.kind === "dead" }), h("span.bt-fname", uname(L, x.b)), fate, x.xp ? h("span.gold", { style: { marginLeft: "auto" } }, `+${x.xp}★`) : null);
  });
  const theirs = F.filter((f) => f.side !== me && committed(f));
  const tFell = theirs.filter((f) => st.dead[f.b]).length;
  const tOut = theirs.filter((f) => st.out[f.b]).length;
  const dealt = dealtBy(L);
  const sum = (side) => hpOf(F.filter((f) => f.side === side).reduce((a, f) => a + dealt[f.b], 0));
  const theirLine =
    tFell === theirs.length
      ? [h("span.bad", `all ${tFell} fell`), L.plot?.city && me === 0 ? h("span.dim.small", " in a city nobody pulls out") : null]
      : [h("span", `${tFell} of ${theirs.length} fell`), tOut ? h("span.lilac", ` · ${tOut} pulled out`) : null];
  const seen = CODEX.filter(([k]) => ctx.ledger.codexSeen(k)).length;
  const p1 = pnl(
    { left: "6px", top: "392px", width: "448px", height: "402px", display: "block" },
    h("div.bt-h4", "Your force"),
    rows,
    h("div.bt-rule"),
    h("div", me === 0 ? "Their garrison: " : "Their force: ", theirLine),
    h("div.dim", `You dealt ${sum(me)} · took ${sum(1 - me)}`),
    h("div.small.dim", { style: { margin: "3px 0 2px" } }, `Rules seen: ${seen} of ${CODEX.length}`),
    h(
      "div.bt-codexrow",
      CODEX.map(([k, name, short]) => {
        const isNew = R.newly.has(k);
        const was = ctx.ledger.codexSeen(k) && !isNew;
        return h(`span.bt-codex${isNew ? ".new" : was ? ".seen" : ""}`, { title: name }, `${was ? "✓" : ""}${short}`);
      }),
    ),
    h("div.small.dim", "gold: new in this battle"),
  );
  // ---- panel 2: the battle (a line replays from its round), the plan checked
  const lines = beatLines(R.script).slice(0, 7);
  const p2 = pnl(
    { left: "460px", top: "392px", width: "420px", height: "402px", display: "block" },
    h("div.bt-h4", "The battle ", h("span.dim.small", "a line replays from there")),
    lines.map((l) => h("div.bt-beat", { role: "button", tabIndex: 0, title: "Replay from here", onclick: () => ctx.toWatch(L, { si: l.seg }) }, h("span.r", l.label), h("span", partsEl(l.parts)))),
  );
  const checks = planChecks(L, st);
  if (checks.length) {
    const mark = { ok: "✓", luck: "~", off: "!" };
    add(p2, 
      h("div.bt-rule"),
      h("div.bt-h4", "The plan, checked"),
      checks.slice(0, 4).map((c) => h("div.bt-row.bt-check", h(`span.bt-verdict.${c.verdict}`, mark[c.verdict]), h("span", c.label), h("span.dim", "→"), h("b", c.result))),
      h("div.small.dim", h("span.bt-verdict.ok.tiny", "✓"), " as planned ", h("span.bt-verdict.luck.tiny", "~"), " luck: rare but inside the plan ", h("span.bt-verdict.off.tiny", "!"), " off plan"),
    );
  }
  // ---- panel 3: what decided it, next time, the rule
  const facts = decidingFacts(L, st);
  const next = nextTime(L, st);
  const p3 = pnl(
    { left: "886px", top: "392px", right: "6px", height: "402px", padding: "2px 10px", display: "block" },
    h("div.bt-h4", "What decided it"),
    facts.map((parts, k) => h("div.bt-fact", h("b.gold", `${k + 1} `), partsEl(parts))),
    next.length ? [h("div.bt-h4", "Next time"), next.map((t) => h("div.bt-fact", `· ${t}`))] : null,
    h("div.bt-rule"),
    h("div.bt-fact", h("b.gold", "Rule "), o.result === "Draw" ? "After 40 rounds both sides draw back; the plot does not change hands." : "A battle ends when one side is gone; the loser loses every unit it sent."),
  );
  add(ui, p1, p2, p3);
}

function ribbon(rw, o) {
  return h(
    "div.bt-pnl.bt-ribbon",
    { style: { left: "306px", width: "650px", justifyContent: "center" } },
    h("b.goldlt.bt-spaced", rw.ribbon),
    h("span", `${rw.line} · ${o.rounds} round${o.rounds === 1 ? "" : "s"}`),
  );
}

/** "Next time" as actions: the best plan the What-if list showed, the wounded to heal, the walls. */
function nextTime(L, st) {
  const F = L.fighters;
  const me = viewerSide(L);
  const out = [];
  const fc = L.forecast;
  const force = F.filter((f) => f.side === me && committed(f)).map((f) => f.b);
  if (fc?.alternatives?.length && me === 0) {
    const best = fc.alternatives.slice().sort((a, b) => b.win_permille - a.win_permille)[0];
    if (best && best.win_permille > fc.win_permille + 10) {
      let label;
      if (best.kind === "Swap") {
        const ins = best.force.filter((b) => !force.includes(b)).map((b) => uname(L, b));
        const outs = force.filter((b) => !best.force.includes(b)).map((b) => uname(L, b));
        label = `${listNames(ins)} in, ${listNames(outs)} out`;
      } else label = `${possessive(tyLabel("StanceType", best.stance))} ${best.kind === "StancePick" ? "own pick" : "pick"}`;
      out.push(`${label}: ${Math.round(best.win_permille / 10)}%`);
    }
  }
  const hurt = F.filter((f) => f.side === me && committed(f) && !st.dead[f.b] && !st.out[f.b] && st.hp[f.b] < f.max_hp * 50);
  if (hurt.length) out.push(`Heal ${listNames(hurt.map((f) => `${uname(L, f.b)} (${Math.ceil(st.hp[f.b] / f.max_hp)}%)`))}`);
  const walls = (L.slots?.[0]?.changes ?? []).find((c) => c.src === "CityDefense" && c.delta < 0);
  if (walls && me === 0) out.push(`Bring the walls under ${-walls.delta * 25}%: one more slot`);
  return out.slice(0, 3);
}

/** Hooks for the screenshot and smoke scripts. */
export const dev = {
  get ready() {
    return R.ready;
  },
};
