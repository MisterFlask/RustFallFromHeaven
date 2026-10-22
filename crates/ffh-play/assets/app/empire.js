// The empire screens (design §5.1 "Empire", §7 P6): the tech tree, civics, religion and the
// commerce sliders. Each reads its query (`techs`, `civics`, `religions`, the view's `me`) and
// sends the Civ4 net message's command (`research`, `update_civics`, `convert`,
// `percent_change`); the server checks them as the exe's screens do.

import * as api from "./api.js";
import { S, dname, info, turns, me, goldBreakdown } from "./state.js";
import { h, toast, typeIconOf } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { render as markup } from "./markup.js";

/** The rich tooltip of entry `x` of the `techs` query: its name and era, what it brings the
 *  player (`help`, the server's C++ `setTechHelp`), its strategy text, and what a click does. */
export function techTip(x, hint) {
  const e = info("TechType", x.tech);
  return () => [
    h("div.tip-title", dname("TechType", x.tech), x.era !== null && x.era !== undefined ? h("span.dim", ` (${dname("EraType", x.era)})`) : null),
    x.help ? h("div.tip-effects", markup(x.help)) : h("div.dim", "Nothing new by itself."),
    e?.strategy ? h("div.tip-strategy", markup(e.strategy)) : null,
    hint ? h("div.tip-hint", hint) : null,
  ];
}

// ---------------------------------------------------------------------------------------------
// The tech tree (Civ4's technology advisor, F6).

/** Rows of the tech tree: each column (prerequisite depth) ordered by the mean row of the
 *  techs' prerequisites (fewer crossing lines), then by era and index. Pure (tested). */
export function treeLayout(techs) {
  const byDepth = new Map();
  for (const t of techs) {
    if (!byDepth.has(t.depth)) byDepth.set(t.depth, []);
    byDepth.get(t.depth).push(t);
  }
  const row = new Map();
  const depths = [...byDepth.keys()].sort((a, b) => a - b);
  let rows = 0;
  for (const d of depths) {
    const col = byDepth.get(d);
    const key = (t) => {
      const pre = [...t.and, ...t.or].map((p) => row.get(p)).filter((r) => r !== undefined);
      return pre.length ? pre.reduce((a, b) => a + b, 0) / pre.length : (t.era ?? 0) * 100 + t.tech / 1000;
    };
    col.sort((a, b) => key(a) - key(b) || (a.era ?? 0) - (b.era ?? 0) || a.tech - b.tech);
    col.forEach((t, i) => row.set(t.tech, i));
    rows = Math.max(rows, col.length);
  }
  return { row, columns: depths.length ? depths[depths.length - 1] + 1 : 0, rows };
}

/** The tech tree: every tech by prerequisite depth with its state. Click: research it (a tech
 *  whose prerequisites are missing queues them first, C++ `pushResearch`); Shift-click: add it to
 *  the queue. */
export async function openTechTree() {
  const m = openModal({ title: "Technology", body: [h("p.dim", "…")], tag: "techtree", wide: true, className: "techtree", buttons: [{ label: "Close", value: null }] });
  const W = 168;
  const H = 38;
  const GX = 36;
  const GY = 8;
  const load = async () => {
    let t;
    try {
      t = await api.query("techs");
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    const techs = t.techs.filter((x) => !x.disabled);
    const { row, columns, rows } = treeLayout(techs);
    const pos = (x) => ({ x: x.depth * (W + GX), y: row.get(x.tech) * (H + GY) });
    const width = columns * (W + GX);
    const height = rows * (H + GY);
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
    svg.setAttribute("class", "tree-lines");
    const byId = new Map(techs.map((x) => [x.tech, x]));
    for (const x of techs) {
      const to = pos(x);
      for (const [list, dash] of [[x.and, ""], [x.or, "4 3"]]) {
        for (const p of list) {
          const from = byId.get(p);
          if (!from) continue;
          const a = pos(from);
          const path = document.createElementNS(ns, "path");
          const x1 = a.x + W;
          const y1 = a.y + H / 2;
          const x2 = to.x;
          const y2 = to.y + H / 2;
          path.setAttribute("d", `M${x1},${y1} C${x1 + GX / 2},${y1} ${x2 - GX / 2},${y2} ${x2},${y2}`);
          path.setAttribute("class", from.has ? "have" : "");
          if (dash) path.setAttribute("stroke-dasharray", dash);
          svg.appendChild(path);
        }
      }
    }
    const boxes = techs.map((x) => {
      const p = pos(x);
      const state = x.has ? "has" : x.researching ? "researching" : x.queued !== null ? "queued" : x.can_research ? "can" : "later";
      const sub = x.has ? "known" : x.researching ? `researching · ${turns(x.turns)}` : x.queued !== null ? `queued ${x.queued + 1}${x.turns !== null ? ` · ${turns(x.turns)}` : ""}` : x.can_research ? `${turns(x.turns)} · ${x.cost}` : `${x.cost}`;
      return h(
        "button.tech",
        {
          class: state,
          "data-tech": x.tech,
          style: { left: `${p.x}px`, top: `${p.y}px`, width: `${W}px`, height: `${H}px` },
          tip: techTip(x, x.has ? null : "Click: research it (with its missing prerequisites). Shift-click: add it to the queue."),
          disabled: x.has,
          onclick: async (ev) => {
            if (await run({ c: "research", tech: x.tech, shift: !!ev.shiftKey })) load();
          },
        },
        h("span.tech-name", dname("TechType", x.tech)),
        h("span.tech-sub", sub),
        x.progress && !x.has ? h("span.tech-bar", h("span", { style: { width: `${Math.min(100, Math.round((100 * x.progress) / Math.max(1, x.cost)))}%` } })) : null,
      );
    });
    const cur = t.current !== null ? dname("TechType", t.current) : null;
    m.setBody(
      h(
        "div.research-head",
        cur ? h("div", "Researching ", h("b", cur)) : h("div.warn", "Nothing is being researched."),
        t.queue.length > 1 ? h("div.dim", `Queue: ${t.queue.map((q) => dname("TechType", q)).join(" → ")}`) : null,
        h("div.dim", `${t.rate} research per turn. Click a technology to research it (with what it needs); Shift-click to queue it.`),
      ),
      h("div.tree-scroll", h("div.tree", { style: { width: `${width}px`, height: `${height}px` } }, svg, boxes)),
    );
    const now = m.el.querySelector(".tech.researching") ?? m.el.querySelector(".tech.can");
    now?.scrollIntoView({ block: "center", inline: "center" });
  };
  load();
  return m.done;
}

// ---------------------------------------------------------------------------------------------
// Civics (F3).

/** The civics screen: each civic option with its civics (the current one marked, those not
 *  available greyed); clicking civics picks a new set, whose anarchy and upkeep the server
 *  computes (`civics {with}`); Revolution sends `update_civics`. */
export async function openCivics() {
  const m = openModal({ title: "Civics", body: [h("p.dim", "…")], tag: "civics", wide: true, buttons: [{ label: "Close", value: null }] });
  let picked = [];
  const load = async () => {
    let c;
    try {
      c = await api.query("civics", picked.length ? { with: picked } : {});
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    const p = c.proposed;
    const columns = c.options.map((o) =>
      h(
        "div.civic-option",
        h("h4", typeIconOf("civicoption", o.option), dname("CivicOptionType", o.option)),
        o.civics.map((x) => {
          const chosen = picked.includes(x.civic);
          return h(
            "button.civic",
            {
              class: [x.current ? "current" : "", chosen ? "chosen" : ""].filter(Boolean).join(" "),
              "data-civic": x.civic,
              disabled: !x.available && !x.current,
              tip: () => civicTip(x),
              onclick: () => {
                // One civic per option: a pick replaces the option's other pick; the current civic
                // takes the pick back.
                const same = o.civics.map((k) => k.civic);
                picked = picked.filter((k) => !same.includes(k));
                if (!x.current) picked.push(x.civic);
                load();
              },
            },
            h("span", dname("CivicType", x.civic)),
            h("span.dim", `${x.upkeep} gold${x.current ? " · now" : ""}`),
          );
        }),
      ),
    );
    const summary = p
      ? h(
          "div.civic-summary",
          p.valid ? h("span", `Revolution: ${p.anarchy ? `${turns(p.anarchy)} of anarchy` : "no anarchy"}, upkeep ${p.upkeep} gold per turn.`) : h("span.warn", p.changed ? "This change is not possible now (the revolution timer, a civic you cannot leave, or anarchy)." : "No change."),
        )
      : h("div.civic-summary.dim", `Upkeep ${c.upkeep} gold per turn.${c.anarchy ? ` Anarchy: ${turns(c.anarchy)}.` : ""}${c.revolution_timer ? ` Next revolution in ${turns(c.revolution_timer)}.` : ""} Pick civics to see the change.`);
    m.setBody(h("div.civics", columns), summary);
    m.setButtons([
      {
        label: "Revolution!",
        primary: true,
        disabled: !p?.valid,
        onClick: async () => {
          if (await run({ c: "update_civics", civics: picked })) {
            toast("Revolution.");
            picked = [];
            m.close("revolution");
          }
        },
      },
      { label: "Close", value: null },
    ]);
  };
  load();
  return m.done;
}

/** The rich tooltip of civic `x` of the `civics` query: why it cannot be adopted now, what it
 *  does, its upkeep and the anarchy of switching to it alone. */
function civicTip(x) {
  const e = info("CivicType", x.civic);
  // The row's help leaves out the requirements, which `requires` lists when one is not met.
  const does = (x.help ?? e?.effects) || e?.help;
  return [
    h("div.tip-title", dname("CivicType", x.civic), x.current ? h("span.dim", " (current)") : null),
    (x.requires ?? []).length ? h("div.tip-requires", x.requires.map((r) => h("div", r))) : null,
    does ? h("div.tip-effects", markup(does)) : null,
    e?.strategy ? h("div.tip-strategy", markup(e.strategy)) : null,
    h("div.tip-hint", `Upkeep ${x.upkeep} gold${x.anarchy ? `; ${turns(x.anarchy)} of anarchy alone` : ""}`),
  ];
}

// ---------------------------------------------------------------------------------------------
// Religion (F7).

/** "first to discover <tech>" for a religion not founded yet (C++ `buildFoundReligionString`). */
function foundedBy(religion) {
  const t = info("ReligionType", religion)?.tech;
  return h("span.dim", t !== null && t !== undefined ? `not founded: the first to discover ${dname("TechType", t)} founds it` : "not founded");
}

/** The rich tooltip of row `x` of the `religions` query: what the religion does, and how it is
 *  founded. */
function religionTip(x) {
  const e = info("ReligionType", x.religion);
  const does = e?.effects || e?.help;
  return [
    h("div.tip-title", dname("ReligionType", x.religion), e?.alignment !== null && e?.alignment !== undefined ? h("span.dim", ` (${dname("AlignmentType", e.alignment)})`) : null),
    does ? h("div.tip-effects", markup(does)) : null,
    !x.founded && e?.tech !== null && e?.tech !== undefined ? h("div.tip-hint", `The first civilization to discover ${dname("TechType", e.tech)} founds it.`) : null,
  ];
}

/** The religion screen: every religion (founded, its holy city as far as revealed, your cities
 *  with it), the state religion, conversion (`convert`). */
export async function openReligion() {
  const m = openModal({ title: "Religion", body: [h("p.dim", "…")], tag: "religion", wide: true, buttons: [{ label: "Close", value: null }] });
  const load = async () => {
    let r;
    try {
      r = await api.query("religions");
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    const convert = async (religion) => {
      if (await run({ c: "convert", religion })) load();
    };
    const rows = r.religions.map((x) =>
      h(
        "tr",
        { class: r.state_religion === x.religion ? "state" : "", "data-religion": x.religion },
        h("td", { tip: () => religionTip(x) }, typeIconOf("religion", x.religion, 16), dname("ReligionType", x.religion), r.state_religion === x.religion ? h("span.badge", "state religion") : null),
        h("td", x.founded ? "founded" : foundedBy(x.religion)),
        h("td", x.holy_city !== null ? (S.cities.get(x.holy_city)?.name ?? `city ${x.holy_city}`) : x.our_holy_city ? "yours" : h("span.dim", "-")),
        h("td", String(x.our_cities)),
        h("td", x.can_convert ? h("button.small", { onclick: () => convert(x.religion) }, "Convert") : null),
      ),
    );
    m.setBody(
      h("p.dim", `${r.state_religion !== null ? `State religion: ${dname("ReligionType", r.state_religion)}. ` : "No state religion. "}Converting costs ${turns(r.anarchy)} of anarchy${r.conversion_timer ? `; the next conversion is possible in ${turns(r.conversion_timer)}` : ""}.`),
      h("table.scores", h("tr", h("th", "Religion"), h("th", "Founded"), h("th", "Holy city"), h("th", "Your cities"), h("th", "")), rows),
      r.can_convert_none ? h("button", { onclick: () => convert(null) }, "No state religion") : null,
    );
  };
  load();
  return m.done;
}

// ---------------------------------------------------------------------------------------------
// The commerce sliders (F2, the financial advisor).

/** The sliders: each commerce's share with the −/+ buttons of the main interface (one step of
 *  COMMERCE_PERCENT_CHANGE_INCREMENTS, flexible commerces only: `percent_change`). */
export function openFinance() {
  const step = S.data?.defines?.commerce_percent_change_increments ?? 10;
  const m = openModal({ title: "Finances", body: [], tag: "finance", buttons: [{ label: "Close", value: null }] });
  const draw = () => {
    const my = me();
    if (!my) return m.setBody(h("p.dim", "No game."));
    const rows = my.commerce_percent.map((p, i) =>
      h(
        "tr",
        { "data-commerce": i },
        h("th", dname("CommerceType", i)),
        h("td.num", `${p}%`),
        h("td", h("span.bar.wide", h("span.fill", { style: { width: `${p}%` } }))),
        h(
          "td",
          my.flexible[i]
            ? [
                h("button.small.minus", { disabled: p < step, title: `−${step}%`, onclick: () => change(i, -step) }, "−"),
                h("button.small.plus", { disabled: p + step > 100, title: `+${step}%`, onclick: () => change(i, step) }, "+"),
              ]
            : h("span.dim", i === 0 ? "the rest" : "fixed"),
        ),
      ),
    );
    // Where the gold rate comes from (the financial advisor).
    const b = goldBreakdown(my.gold_parts);
    const money = b
      ? h(
          "table.kv.finance",
          b.income.map(([what, n]) => h("tr", h("th", `+${n}`), h("td", what))),
          b.costs.map(([what, n]) => h("tr", h("th.warn", `−${n}`), h("td", what))),
          h("tr.total", h("th", `${my.gold_rate >= 0 ? "+" : ""}${my.gold_rate}`), h("td", my.gold_parts.research_fixed ? "a turn (research cannot be adjusted: only a loss counts)" : "a turn")),
        )
      : null;
    m.setBody(
      h("table.kv.sliders", rows),
      h("p", `Gold ${my.gold} (${my.gold_rate >= 0 ? "+" : ""}${my.gold_rate} per turn) · research ${my.research_rate} per turn${my.anarchy ? ` · anarchy ${turns(my.anarchy)}` : ""}`),
      money ? h("h4", "Gold per turn") : null,
      money,
    );
  };
  const change = async (commerce, by) => {
    await run({ c: "percent_change", commerce, change: by });
  };
  draw();
  financeRedraw = () => (m.closed ? (financeRedraw = null) : draw());
  return m;
}

let financeRedraw = null;

/** A new view arrived: the open sliders show it. */
export function viewChanged() {
  financeRedraw?.();
}
