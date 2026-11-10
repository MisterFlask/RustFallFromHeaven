// The battle report (formation battles, phase 6 revision 4): the battlefield frozen where the
// battle ended (the fallen where they fell, those knocked back where they landed, those who
// escaped gone), whether the plot fell, each side's fallen, escaped and standing, and the whole log.

import { h } from "./dom.js";
import { finalState, fates, resultWords, logLines, viewerSide, uname, hpOf, xpBy, tacticName } from "./battle_log.js";
import { loadArt, unitXY, pic, deadPic, faceFlip, animated, headChip, uiIcon, btn, add, partsEl, hpBar, groundLine } from "./battle_stage.js";

export const NAME = "report";

const R = { ctx: null, log: null, st: null, ready: false };

export async function enter(ctx, log) {
  R.ctx = ctx;
  R.ready = false;
  R.log = log;
  ctx.stage.me = viewerSide(log);
  await loadArt({ fighters: log.fighters, plot: log.plot, from: log.from, sides: log.sides });
  if (R.log !== log) return;
  ctx.stage.setBoard(log);
  R.st = finalState(log);
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
  const sides = L.sides;
  const anim = animated(sides);
  const figs = [];
  for (const f of L.fighters) {
    const pos = R.st.pos[f.b];
    const status = R.st.status[f.b];
    if (!pos || status === "gone") continue;
    const [x, y] = unitXY(ctx.stage.G, f, pos);
    const fig = { b: f.b, x, y, side: f.side, large: f.large, flip: faceFlip(f, sides) };
    if (status === "dead") {
      const dp = deadPic(f, sides);
      Object.assign(fig, dp ? { pic: dp, alpha: 0.85, ringAlpha: 0.25 } : { fallen: true, pic: pic(f, sides), alpha: 0.85, ringAlpha: 0.25 });
    } else if (status === "afraid") Object.assign(fig, { pic: pic(f, sides, "idle", now + f.b * 137, true), alpha: 0.45, ringAlpha: 0.3 });
    else Object.assign(fig, { pic: pic(f, sides, anim(f) ? "idle" : null, now + f.b * 137, true), bar: { hp: R.st.hp[f.b], max: f.max_hp * 100 } });
    figs.push(fig);
  }
  ctx.stage.draw({ figs });
}

const pnl = (style, ...children) => h("div.bt-pnl", { style }, ...children);

function render(ctx) {
  const ui = ctx.ui;
  ui.replaceChildren();
  ctx.ov.replaceChildren();
  const L = R.log;
  const me = viewerSide(L);
  const rw = resultWords(L);
  const where = L.plot?.city?.name;
  const rounds = L.outcome?.rounds ?? Math.max(0, ...L.events.map((e) => e.round ?? 0));
  // ---- top left: the result
  add(
    ui,
    pnl(
      { left: "6px", top: "4px", width: "380px" },
      h("div.bt-row", uiIcon("attack", 16), h("span.bt-h3", where ? `Battle of ${where}` : `Battle at (${L.plot?.x ?? "?"}, ${L.plot?.y ?? "?"})`)),
      h("div.bt-row", h(`b.bt-spaced.${rw.good ? "goldlt" : "bad"}#bt-result`, { "data-kind": rw.kind }, rw.ribbon), h("span.dim", `${rounds} round${rounds === 1 ? "" : "s"}`)),
      h("div.small", rw.line),
    ),
  );
  // ---- top right: the ground, the way on
  add(
    ui,
    pnl(
      { right: "6px", top: "4px", width: "470px" },
      h("div.bt-row", { style: { justifyContent: "flex-end", gap: "8px" } }, btn("Replay", { key: "replay", title: "Watch it again", onclick: () => ctx.toWatch(L) }), btn("Continue", { cls: "primary", key: "continue", title: "Back to the map [Enter]", onclick: () => ctx.close() })),
      groundLine(L.ground, L.plot),
    ),
  );
  // ---- bottom left: the log
  const list = h("ol.bt-log");
  for (const l of logLines(L)) {
    if (l.head) list.append(h("li.head", l.text));
    else list.append(h("li", h("span.sp", l.speed !== null ? String(l.speed) : "·"), partsEl(l.parts)));
  }
  add(ui, pnl({ left: "6px", top: "506px", width: "420px", height: "288px", display: "flex", flexDirection: "column" }, h("div.bt-row", h("span.bt-h4", "Battle log"), h("span.dim.small", `${L.events.length} actions`)), list));
  // ---- bottom right: each side's fates
  add(ui, fatesPanel(L, me));
}

/** Each side: its fallen, escaped and standing (the units of its line), with their heads. */
function fatesPanel(L, me) {
  const fs = fates(L, R.st);
  const xp = xpBy(L);
  const sideBlock = (side) => {
    const s = fs[side];
    const label = L.sides?.[side]?.label ?? (side === me ? "Your army" : "Them");
    const chip = (b, kind) => {
      const f = L.fighters[b];
      const title = `${uname(L, b)}: ${kind === "fallen" ? "fell" : kind === "escaped" ? "escaped a killing blow and left the field" : kind === "afraid" ? "sat out, afraid" : `${hpOf(R.st.hp[b])} of ${f.max_hp} HP`}${xp[b] ? `, +${xp[b]} XP` : ""}`;
      return h(
        `span.bt-fate.${kind}`,
        { title, "data-b": b },
        headChip(f, L.sides, { w: 26, h: 24, dead: kind === "fallen", flip: side !== 0 }),
        kind === "standing" ? hpBar(hpOf(R.st.hp[b]), f.max_hp, { w: 26 }) : h("span.small", kind === "fallen" ? "fell" : kind === "escaped" ? "↩" : "afraid"),
      );
    };
    const tactic = tacticName(L, side);
    const counts = [tactic ? `${tactic} tactic` : null, `${s.fallen.length} fallen`, s.escaped.length ? `${s.escaped.length} escaped` : null, `${s.standing.length} standing`, s.afraid.length ? `${s.afraid.length} sat out` : null].filter(Boolean).join(" · ");
    return h(
      "div.bt-side",
      h("div.bt-row", h(`span.bt-h4.${side === me ? "you" : "them"}`, label), h("span.dim.small", counts)),
      h("div.bt-chips", s.standing.map((b) => chip(b, "standing")), s.escaped.map((b) => chip(b, "escaped")), s.afraid.map((b) => chip(b, "afraid")), s.fallen.map((b) => chip(b, "fallen"))),
    );
  };
  const o = L.outcome ?? {};
  const left = o.defenders_left ?? fs[1].standing.length;
  return pnl(
    { right: "6px", top: "556px", width: "424px", height: "238px", display: "block", padding: "4px 12px" },
    sideBlock(me),
    sideBlock(1 - me),
    h("div.bt-rule"),
    h("div.small", o.plot_taken ? h("b.goldlt", L.plot?.city ? `${L.plot.city.name} is taken.` : "The plot is taken.") : h("span", { title: "Defenders stay until annihilated: the plot falls when none is left." }, h("b", `${left} defender${left === 1 ? "" : "s"}`), " still on the plot.")),
  );
}

/** Hooks for the screenshot and smoke scripts. */
export const dev = {
  get ready() {
    return R.ready;
  },
  get result() {
    return R.log ? resultWords(R.log).kind : null;
  },
  fates: () => (R.log ? fates(R.log, R.st) : null),
};
