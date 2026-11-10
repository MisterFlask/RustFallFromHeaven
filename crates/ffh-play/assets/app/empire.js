// The empire screens (design §5.1 "Empire", §7 P6): the tech tree, civics, religion and the
// commerce sliders. Each reads its query (`techs`, `civics`, `religions`, the view's `me`) and
// sends the Civ4 net message's command (`research`, `update_civics`, `convert`,
// `percent_change`); the server checks them as the exe's screens do.

import * as api from "./api.js";
import { S, dname, info, turns, me, goldBreakdown } from "./state.js";
import { h, toast, typeIconOf, tapModes, coarse, errorText } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { run } from "./commands.js";
import { render as markup } from "./markup.js";
import { isoSkin, isoIcon, unlockIcon } from "./isoui.js";
import { infoTip } from "./infotips.js";

/** A multiplier in percent (200) as the screens write it: "x2", "x2.7", "x1.35". */
export function times(pct) {
  return `x${Number((pct / 100).toFixed(2))}`;
}

/** The tech-specific research multipliers of entry `x` of the `techs` query (`research_mods`):
 *  the ones that apply, each `{ key, short, long }` (known by civilizations met, known
 *  alternative prerequisites, a held unique feature, the research path), in that order. */
export function researchMods(x) {
  const m = x.research_mods;
  if (!m) return [];
  return [
    m.others ? { key: "others", short: `others +${m.others}%`, long: `Known by civilizations met: +${m.others}%` } : null,
    m.prereqs ? { key: "prereqs", short: `prereqs +${m.prereqs}%`, long: `Alternative prerequisites known: +${m.prereqs}%` } : null,
    m.feature ? { key: "feature", short: `feature ${times(100 + m.feature)}`, long: `Unique feature held: ${times(100 + m.feature)}` } : null,
    m.path ? { key: "path", short: `doctrine ${times(100 + m.path)}`, long: `Doctrine: ${times(100 + m.path)}` } : null,
  ].filter(Boolean);
}

/** A tech card's line of research multipliers (`researchMods`) and their total ("= x2.7"); null
 *  for a known tech or one without any. */
export function researchModsRow(x) {
  const mods = researchMods(x);
  if (x.has || !mods.length) return null;
  return h("span.mods", mods.map((md) => h(`span.mod-chip.mod-${md.key}`, md.short)), h("span.mod-total", `= ${times(x.research_mods.total)}`));
}

/** The rich tooltip of entry `x` of the `techs` query: its name and era, what it brings the
 *  player (`help`, the server's C++ `setTechHelp`), the research multipliers on it, its strategy
 *  text, and what a click does. */
export function techTip(x, hint) {
  const e = info("TechType", x.tech);
  const mods = x.has ? [] : researchMods(x);
  return () => [
    h("div.tip-title", dname("TechType", x.tech), x.era !== null && x.era !== undefined ? h("span.dim", ` (${dname("EraType", x.era)})`) : null),
    x.help ? h("div.tip-effects", markup(x.help)) : h("div.dim", "Nothing new by itself."),
    mods.length
      ? h(
          "div.tip-mods",
          h("div", `Research on this tech: ${times(x.research_mods.total)}`),
          mods.map((md) => h("div.dim", `• ${md.long}`)),
          h("div.dim", "Others and prerequisites add up; the feature and the doctrine each multiply."),
        )
      : null,
    e?.strategy ? h("div.tip-strategy", markup(e.strategy)) : null,
    hint ? h("div.tip-hint", hint) : null,
  ];
}

/** The `techs` query's `unlocks` kinds, in the order a tech's card shows them, with their tables. */
const UNLOCKS = [["units", "UnitType"], ["builds", "BuildType"], ["buildings", "BuildingType"], ["religions", "ReligionType"], ["civics", "CivicType"]];

/** What entry `x` of the `techs` query unlocks (its units, builds, buildings, religions, civics),
 *  for a tech's card: in the isometric interface a row of their icons, each with its own tooltip,
 *  the first `max` and a count of the rest; else their names on a line. Null when it unlocks none of
 *  them. */
export function unlockRow(x, max = 6) {
  const items = UNLOCKS.flatMap(([kind, table]) => (x.unlocks?.[kind] ?? []).map((i) => ({ kind, table, i })));
  if (!items.length) return null;
  const names = items.map((it) => dname(it.table, it.i));
  if (!isoSkin()) return h("span.unlocks.dim", { title: names.join(", ") }, names.join(", "));
  const shown = items.length > max ? items.slice(0, max - 1) : items;
  // A build's tooltip is its improvement's (or road's): what the land gains.
  const tipOf = ({ kind, table, i }) => {
    const b = kind === "builds" ? info("BuildType", i) : null;
    if (b?.improvement !== null && b?.improvement !== undefined) return infoTip("ImprovementType", b.improvement);
    if (b?.route !== null && b?.route !== undefined) return infoTip("RouteType", b.route);
    return infoTip(table, i);
  };
  return h(
    "span.unlocks",
    shown.map((it) => h("span.unlock", { tip: tipOf(it) }, unlockIcon(it.kind, it.i))),
    shown.length < items.length ? h("span.unlock.more", { title: names.slice(shown.length).join(", ") }, `+${items.length - shown.length}`) : null,
  );
}

// ---------------------------------------------------------------------------------------------
// Great-person cards (docs/CARDS.md): techs a great person discovers, outside the tree.

/** The cards the player knows (entries of the `techs` query that are cards), and those its great
 *  people hold (the view's units' `card`), each with its help as a tooltip; null when there are
 *  none. */
export function cardsLine(t) {
  const known = (t.techs ?? []).filter((x) => x.has && info("TechType", x.tech)?.card);
  const mine = me()?.player;
  const held = [...(S.units?.values?.() ?? [])].filter((u) => u.owner === mine && u.card !== null && u.card !== undefined);
  if (!known.length && !held.length) return null;
  const card = (tech, extra = "") => h("span.card-name", { tip: infoTip("TechType", tech, { title: `Card: ${dname("TechType", tech)}` }) }, `${dname("TechType", tech)}${extra}`);
  const list = (items) => items.flatMap((x, i) => (i ? [", ", x] : [x]));
  return h(
    "div.cards-known",
    known.length ? h("div", "Cards known: ", list(known.map((x) => card(x.tech)))) : h("div.dim", "Cards known: none."),
    held.length ? h("div", "Cards held by your great people: ", list(held.map((u) => card(u.card, ` (${dname("UnitType", u.type)} at ${u.x}, ${u.y})`)))) : null,
  );
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

/** The techs related to tech `id` in the tree, from `byId` (tech -> entry with `and`/`or`): `path`,
 *  `id` and every tech it needs (its and/or prerequisites, all the way back); `next`, the techs
 *  that list `id` as a prerequisite. Pure (tested). */
export function treeRelated(byId, id) {
  const path = new Set([id]);
  const stack = [id];
  while (stack.length) {
    const x = byId.get(stack.pop());
    for (const p of [...(x?.and ?? []), ...(x?.or ?? [])]) if (byId.has(p) && !path.has(p)) (path.add(p), stack.push(p));
  }
  const next = new Set();
  for (const x of byId.values()) if (x.tech !== id && (x.and.includes(id) || x.or.includes(id))) next.add(x.tech);
  return { path, next };
}

/** A tech's research cost for its box: the research icon and the number (the word in the flat look). */
function costLabel(cost) {
  const icon = isoSkin() ? isoIcon("research", { px: 16, cls: "tech-cost-icon" }) : null;
  return icon ? h("span.tech-cost", icon, String(cost)) : h("span.tech-cost", `${cost} research`);
}

/** The tech tree: every tech by prerequisite depth with its state, what it unlocks and its era
 *  (the box's left edge; columns are depths, which eras cut across, so a legend names the
 *  colours). Click: research it (a tech whose prerequisites are missing queues them first, C++
 *  `pushResearch`); Shift-click, or the "Add to queue" mode: add it to the queue. Pointing at a
 *  tech (on touch, the first tap) lights its path and the techs it leads to. */
export async function openTechTree() {
  const m = openModal({ title: "Technology", body: [h("p.dim", "…")], tag: "techtree", wide: true, className: "techtree", buttons: [{ label: "Close", value: null }] });
  const W = 176;
  const H = 66;
  const GX = 40;
  const GY = 10;
  /** What a click or tap does (Shift-click queues in either mode): "now" or "queue". */
  let tapMode = "now";
  /** On touch: the tech whose path the first tap lit; a second tap on it researches it. */
  let armed = null;
  /** The pointer of the press that leads to a click ("touch", "mouse", "pen"; "key" without one). */
  let pointer = "key";
  const load = async () => {
    let t;
    try {
      t = await api.query("techs");
    } catch (e) {
      return m.setBody(h("p.warn", errorText(e)));
    }
    if (m.closed) return;
    // A reload (after a click) keeps the view where it was; the first one scrolls to the research.
    const was = m.el.querySelector(".tree-scroll");
    const keep = was ? { left: was.scrollLeft, top: was.scrollTop } : null;
    armed = null;
    const techs = t.techs.filter((x) => !x.disabled);
    const { row, columns, rows } = treeLayout(techs);
    const pos = (x) => ({ x: x.depth * (W + GX), y: row.get(x.tech) * (H + GY) });
    const width = Math.max(0, columns * (W + GX) - GX);
    const height = Math.max(0, rows * (H + GY) - GY);
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
    svg.setAttribute("class", "tree-lines");
    const byId = new Map(techs.map((x) => [x.tech, x]));
    const lines = [];
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
          if (from.has) path.setAttribute("class", "have");
          path.dataset.from = String(p);
          path.dataset.to = String(x.tech);
          if (dash) path.setAttribute("stroke-dasharray", dash);
          svg.appendChild(path);
          lines.push(path);
        }
      }
    }
    const hint = coarse() ? "Tap: show its path. Tap again: research it (with what it needs)." : "Click: research it (with what it needs). Shift-click: add it to the queue.";
    const boxes = techs.map((x) => {
      const p = pos(x);
      const state = x.has ? "has" : x.researching ? "researching" : x.queued !== null ? "queued" : x.can_research ? "can" : "later";
      const mult = !x.has && x.research_mods && x.research_mods.total !== 100 ? times(x.research_mods.total) : null;
      const pct = !x.has && x.progress ? Math.min(100, Math.round((100 * x.progress) / Math.max(1, x.cost))) : 0;
      // A bar for the tech being researched (from 0%) and for any with research put into it.
      const bar = !x.has && (x.researching || x.progress > 0);
      const sub = x.has
        ? [h("span", "known")]
        : [
            x.can_research && x.turns !== null ? h("span", turns(x.turns)) : null,
            costLabel(x.cost),
            // Its research multiplier (the tooltip has the parts).
            mult ? h("span.tech-mult", mult) : null,
          ].filter(Boolean).flatMap((e, i) => (i ? [h("span.sep", "·"), e] : [e]));
      return h(
        "button.tech",
        {
          // A tech of the research path committed to (docs/PORTING.md, "Research paths").
          class: [state, x.can_research && !x.has ? "can-now" : "", x.research_mods?.path ? "on-path" : ""].filter(Boolean).join(" "),
          "data-tech": x.tech,
          "data-era": x.era ?? "",
          "aria-label": `${dname("TechType", x.tech)}: ${x.has ? "known" : x.researching ? `researching, ${pct}%` : x.queued !== null ? `queued ${x.queued + 1}` : x.can_research ? "can be researched" : "prerequisites missing"}`,
          onmouseenter: () => focus(x.tech),
          onmouseleave: () => focus(null),
          onfocus: () => focus(x.tech),
          onblur: () => focus(null),
          onpointerdown: (ev) => (pointer = ev.pointerType || "key"),
          style: { left: `${p.x}px`, top: `${p.y}px`, width: `${W}px`, height: `${H}px` },
          tip: techTip(x, x.has ? null : hint),
          disabled: x.has,
          onclick: async (ev) => {
            // On touch the first tap only shows the path (a finger has no hover); a key press
            // (no pointer before it) researches at once.
            const by = pointer;
            pointer = "key";
            if (by === "touch" && armed !== x.tech) {
              armed = x.tech;
              focus(x.tech);
              return;
            }
            if (await run({ c: "research", tech: x.tech, shift: !!ev.shiftKey || tapMode === "queue" })) load();
          },
        },
        h("span.tech-name", dname("TechType", x.tech)),
        x.has ? h("span.tech-check", { "aria-hidden": "true" }) : null,
        x.queued !== null && !x.has && !x.researching ? h("span.tech-queue", { title: `Queue position ${x.queued + 1}` }, String(x.queued + 1)) : null,
        unlockRow(x, 5) ?? h("span.unlocks.none"),
        h("span.tech-sub", sub),
        bar ? h("span.tech-bar", h("span", { style: { width: `${pct}%` } })) : null,
        bar ? h("span.tech-pct", `${pct}%`) : null,
      );
    });
    // Pointing at a tech lights it, every tech it needs (its and/or prerequisites, all the way
    // back) and the techs it leads to, with the lines between them; the rest of the tree dims.
    const tree = h("div.tree", { style: { width: `${width}px`, height: `${height}px` } }, svg, boxes);
    function focus(id) {
      if (id === null && armed !== null) id = armed;
      tree.classList.toggle("focus", id !== null);
      const { path, next } = id === null ? { path: new Set(), next: new Set() } : treeRelated(byId, id);
      for (const b of boxes) {
        const k = Number(b.dataset.tech);
        b.classList.toggle("lit", path.has(k) || next.has(k));
        b.classList.toggle("next", next.has(k));
      }
      for (const l of lines) {
        const from = Number(l.dataset.from);
        const to = Number(l.dataset.to);
        const onPath = path.has(to) && path.has(from);
        const toNext = from === id && next.has(to);
        l.classList.toggle("lit", onPath || toNext);
        l.classList.toggle("next", toNext);
      }
    }
    // The era of each box's left edge: the eras present, in order.
    const eras = [...new Set(techs.map((x) => x.era).filter((e) => e !== null && e !== undefined))].sort((a, b) => a - b);
    const legend = eras.length
      ? h("div.era-legend", h("span.dim", "Era (left edge):"), eras.map((e) => h("span.era-key", { "data-era": e }, dname("EraType", e))))
      : null;
    const cur = t.current !== null ? byId.get(t.current) ?? t.techs[t.current] : null;
    const modes = tapModes(
      tapMode,
      coarse()
        ? [["now", "Research now", "A tap researches it now"], ["queue", "Add to queue", "A tap adds it to the research queue (Shift-click)"]]
        : [["now", "Research now", "A click researches it now"], ["queue", "Add to queue", "A click adds it to the research queue (as Shift-click)"]],
      (v) => ((tapMode = v), load()),
    );
    // The mouse has the selector too: "A click:".
    modes.classList.add("tree-modes");
    if (!coarse()) modes.querySelector(".dim").textContent = "A click:";
    const scroll = h("div.tree-scroll", tree);
    const wrap = h("div.tree-wrap", scroll);
    // Edge fades where more of the tree lies beyond the view (updated on scroll only).
    const edges = () => {
      wrap.classList.toggle("more-right", scroll.scrollLeft + scroll.clientWidth < scroll.scrollWidth - 2);
      wrap.classList.toggle("more-down", scroll.scrollTop + scroll.clientHeight < scroll.scrollHeight - 2);    };
    scroll.addEventListener("scroll", edges, { passive: true });
    m.setBody(
      h(
        "div.research-head",
        h(
          "div.research-state",
          cur ? h("span", "Researching ", h("b", dname("TechType", t.current)), `: ${turns(cur.turns)}.`) : h("span.warn", "Nothing is being researched."),
          h("span.dim", ` ${t.rate} research per turn.`),
        ),
        t.path !== null && t.path !== undefined
          ? h("div", "Doctrine: ", h("b", dname("ResearchPathType", t.path)), ` (+${t.path_percent}% research on its techs, outlined `, h("span.path-mark"), ")")
          : h("div.warn", "No doctrine (adopt one in the research screen)."),
        cardsLine(t),
        t.queue.length > 1 ? h("div.dim", `Queue: ${t.queue.map((q) => dname("TechType", q)).join(" → ")}`) : null,
        h("div.dim", coarse() ? "Tap: show its path. Tap again: research (with what it needs)." : "Click: research (with what it needs). Shift-click: queue."),
        modes,
        legend,
      ),
      wrap,
    );
    if (keep) {
      scroll.scrollLeft = keep.left;
      scroll.scrollTop = keep.top;
    } else {
      // The tech being researched, else the leftmost one that can be researched now, in view.
      const can = techs.filter((x) => x.can_research && !x.has).sort((a, b) => a.depth - b.depth || row.get(a.tech) - row.get(b.tech));
      const goal = cur && byId.has(t.current) ? byId.get(t.current) : can[0];
      if (goal) {
        const p = pos(goal);
        scroll.scrollLeft = Math.max(0, p.x + W / 2 - scroll.clientWidth / 2);
        scroll.scrollTop = Math.max(0, p.y + H / 2 - scroll.clientHeight / 2);
      }
    }
    edges();
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
      return m.setBody(h("p.warn", errorText(e)));
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
          p.valid ? h("span", `Revolution: ${p.anarchy ? `${turns(p.anarchy)} of anarchy` : "no anarchy"}, upkeep ${p.upkeep} gold per turn.`) : (p.changed ? h("span.warn", { title: "The revolution timer, a civic you cannot leave, or anarchy" }, "This change is not possible now.") : h("span.warn", "No change.")),
        )
      : h("div.civic-summary.dim", `Upkeep ${c.upkeep} gold per turn.${c.anarchy ? ` Anarchy: ${turns(c.anarchy)}.` : ""}${c.revolution_timer ? ` Next revolution in ${turns(c.revolution_timer)}.` : ""} Pick civics to see the change.`);
    m.setBody(h("div.civics", columns), summary);
    m.setButtons([
      {
        label: "Revolution!",
        primary: true,
        disabled: !p?.valid,
        onClick: async () => {
          // Anarchy is the price: ask first (Cancel has the focus); none, no question.
          if (p?.anarchy && !(await ask("Start the revolution?", `Start the revolution? ${turns(p.anarchy)} of anarchy.`, [
            { label: "Revolution", value: true, primary: true, danger: true },
            { label: "Cancel", value: null },
          ]))) return;
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
      return m.setBody(h("p.warn", errorText(e)));
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
        h("td", x.founded ? "founded" : h("span.dim", "not founded")),
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
          h("tr.total", my.gold_parts.research_fixed ? { title: "Research cannot be adjusted: only a loss counts." } : {}, h("th", `${my.gold_rate >= 0 ? "+" : ""}${my.gold_rate}`), h("td", "a turn")),
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
