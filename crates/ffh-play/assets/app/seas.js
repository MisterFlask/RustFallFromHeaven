// Named seas, the screens (this game's own rules, docs/NAVAL.md section 10; the map's part is in
// sea.js):
//
// - the sea's panel (`openSeaPanel`, a click on a sea's name or badge on the Sea layer, or from
//   the Seas screen): its plots, control and what holding takes, each side's strength with its
//   ships on Patrol, the effect now, what follows if an enemy or you hold it, and, with your
//   warships selected, `Send ... to patrol here (N turns)` with the state it would give (a go-to
//   to the nearest plot of the sea they reach, with Patrol queued after it);
// - the Seas screen (`openSeasScreen`, the menu's Seas): one row per sea you have seen, its
//   control, your strength, the enemy's, your ports on it and your supply ships' lanes through it;
//   a row shows the sea on the map, its button opens the panel.
//
// The words come from the server (the `sea` and `seas` queries: what you can work out; only ships
// you see count). The helpers that shape them are pure (tested in sea.test.mjs).

import * as api from "./api.js";
import { S, tidx } from "./state.js";
import { h, toast, errorText } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { run } from "./commands.js";
import { SEA_STATES } from "./sea.js";

/** The map hooks (main.js): `centerOn(x, y)`, `seaLayer()` (turns the Sea layer on), `focus(id)`
 *  (the sea whose panel is open, null when it closes: the HD-2D map outlines it). */
const hooks = { centerOn: null, seaLayer: null, focus: null };

export function init({ centerOn, seaLayer, focus } = {}) {
  hooks.centerOn = centerOn ?? null;
  hooks.seaLayer = seaLayer ?? null;
  hooks.focus = focus ?? null;
}

/** A state chip (the badge's look) with `text`. */
export function stateChip(state, text) {
  const look = SEA_STATES[state] ?? SEA_STATES.open;
  return h("span.sea-chip", { "data-state": state, style: { background: look.fill, color: look.ink, borderColor: look.ring } }, text);
}

/** A strength bar's width in percent: `x` against the largest of `others` (and `x`), at least 2%
 *  for a strength above 0. */
export function barPercent(x, others) {
  const max = Math.max(x, ...others, 1);
  if (!(x > 0)) return 0;
  return Math.max(2, Math.round((100 * x) / max));
}

/** The commands that send `send.units` to patrol the sea (the panel's `send`): Patrol now where
 *  they stand, else a go-to to (x, y) with Patrol queued after it. */
export function sendCommands(send) {
  const plunder = tidx("MissionType", "MISSION_PLUNDER");
  const patrol = { c: "mission", units: send.units, mission: plunder, data1: -1, data2: -1, flags: 0, append: !send.here };
  if (send.here) return [patrol];
  return [{ c: "move", units: send.units, x: send.x, y: send.y, append: false }, patrol];
}

/** The panel of sea `id` (the `sea` query, with the selection for its `send`). */
export async function openSeaPanel(id) {
  let p;
  try {
    p = await api.query("sea", { id, units: S.sel?.units ?? [] });
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const sides = p.sides ?? [];
  const strengths = sides.map((s) => s.strength);
  const section = (title, lines) => h("div.sea-section", h("div.sea-head", title), (lines ?? []).map((t) => h("div", t)));
  const side = (s) =>
    h(
      `div.sea-side${s.own ? ".own" : ""}`,
      h("div.sea-side-head", h("span", s.label), h("b", String(s.strength))),
      h("div.sea-bar", h(`span${s.own ? "" : ".them"}`, { style: { width: `${barPercent(s.strength, strengths)}%` } })),
      s.ships?.length ? h("div.small", s.ships.join(", ")) : null,
      h("div.small.dim", s.note),
    );
  const send = p.send;
  const sendRow = send
    ? h(
        "div.sea-send",
        h(
          "button.primary",
          {
            disabled: !send.enabled,
            title: send.why ?? null,
            onclick: async () => {
              closeTag("sea");
              closeTag("seas");
              for (const c of sendCommands(send)) {
                const r = await run(c);
                if (r === null) return;
              }
            },
          },
          send.label,
        ),
        (send.result ?? []).map((t, i) => h(`div.small${i ? ".dim" : ""}`, i ? t : `Then: ${t}`)),
        send.why ? h("div.small.warn", send.why) : null,
      )
    : null;
  const m = openModal({
    title: p.name,
    tag: "sea",
    className: "sea-panel",
    help: "sea",
    body: [
      h("div.sea-title", h("span.dim", p.plots_text), stateChip(p.state, p.badge)),
      h("div.sea-section", h("div.sea-head", "Control"), h("div.sea-control", p.control), h("div.dim", p.hold)),
      h("div.sea-sides", sides.map(side)),
      section("Effect now", p.now),
      p.if_they ? section(p.if_they.title, p.if_they.lines) : null,
      p.if_you ? section(p.if_you.title, p.if_you.lines) : null,
      sendRow,
    ],
    buttons: [
      { label: "Show on the map", onClick: (m) => (m.close(null), hooks.seaLayer?.(), hooks.centerOn?.(p.x, p.y)) },
      { label: "Close", value: null },
    ],
  });
  // The map outlines the sea while its panel is open.
  hooks.focus?.(id);
  m.done.then(() => hooks.focus?.(null));
}

/** The Seas screen (the `seas` query). */
export async function openSeasScreen() {
  let r;
  try {
    r = await api.query("seas");
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const rows = r.seas ?? [];
  const m = openModal({
    title: "Seas",
    tag: "seas",
    wide: true,
    help: "sea",
    body: [
      h("p.dim", "One row per sea you have seen. Click a row: show it on the map. Only ships you see are counted."),
      rows.length
        ? h(
            "table.scores.seas",
            h("tr", h("th", "Sea"), h("th", "Control"), h("th.num", "You"), h("th", "Enemy"), h("th", "Your ports"), h("th", "Your lanes"), h("th", "")),
            rows.map((s) =>
              h(
                "tr.clickable",
                { "data-sea": s.id, onclick: () => (m.close(null), hooks.seaLayer?.(), hooks.centerOn?.(s.x, s.y)) },
                h("td.sea-name", s.name),
                h("td", stateChip(s.state, s.control)),
                h("td.num", String(s.ours)),
                h("td", s.enemy),
                h("td", s.ports),
                h(`td${s.lanes_cut ? ".warn" : ""}`, s.lanes),
                h("td", h("button.small", { onclick: (ev) => (ev.stopPropagation(), openSeaPanel(s.id)) }, "Panel")),
              ),
            ),
          )
        : h("p.dim", "No sea seen yet."),
    ],
    buttons: [{ label: "Close", value: null }],
  });
  return m;
}
