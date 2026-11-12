// World rituals (docs/CARDS.md, "World rituals"; docs/HUMAN.md, "World rituals"): what the
// player sees of every civilization's ritual.
//
// - The rituals element (`#rituals`, `renderRituals`): one row per ritual in progress (or queued by
//   you) and per cast pending, anywhere: the civilization's colour, the ritual, the city, a
//   progress bar with `37%` and `6 turns`, or `Cast pending`. A click on a row centres the map on
//   the city; "All" opens the full list. With nothing in progress and no cast pending it shows
//   one muted line, `No rituals in progress`.
// - The full list (`openRitualsPanel`): every civilization, idle ones too, with the times
//   completed, the cost now and the next cost (and your ritual's own state: The Tithe's named
//   rival).
// - The alerts (`ritualAlerts`): each ritual's news (`<Civ> began|abandoned|completed <ritual> in
//   <city>.`, the log's `kind: "ritual"`) as a toast; a click centres the map on the city.
//
// The rows are the view's `rituals` (src/view/rituals.rs); a civilization not met is `Unknown
// civilization`, drawn without its colour. The pure helpers are in ritual_rows.js.

import { S, emit, isBarbarian, observing } from "./state.js";
import { h, fill, toastNode } from "./dom.js";
import { openModal } from "./modals.js";
import { playerColor } from "./tiles.js";
import { barEl } from "./look.js";
import { render as markup } from "./markup.js";
import { addHelp } from "./help.js";
import { keywordChips } from "./keywords.js";
import { activeRows, chipName, rowStatus, rowLabel, listLines, ritualAlerts as alertsOf } from "./ritual_rows.js";

addHelp({
  id: "rituals",
  group: "Playing",
  order: 7,
  title: "World rituals",
  // The reference page of the rituals' rules: full prose, the keywords marked (their reminder
  // texts are the same rules, said once).
  body: () => [
    h(
      "ul",
      [
        "A civilization's world spell comes with each completion of its [KW=KEYWORD_WORLD_RITUAL]world ritual[\\KW] (a project) in a city.",
        "A world spell with no target takes effect at the completion. One with a target must be cast before the turn ends; while it has no valid target, the turn may end and the cast stays pending.",
        "Every project is a [KW=KEYWORD_RITUAL]ritual[\\KW]. A ritual's progress is lost if its city switches to building something else.",
        "A ritual cannot be hurried with gold. Hurrying with population is allowed.",
        "One world ritual in progress per player, in one city; not started again while its cast is pending.",
        "Each completion doubles a world ritual's next cost.",
        "Every player is told when any world ritual begins, is abandoned or completes.",
        "Some world rituals bring no spell: they act at their completion.",
        "The rituals element (under the top bar) lists the world rituals in progress and the casts pending; All lists every civilization's.",
      ].map((t) => h("li", markup(t))),
    ),
  ],
});

/** A civilization's colour swatch (none for one not met). */
function swatch(r) {
  return h("span.swatch.ritual-swatch", { style: { background: r.met ? playerColor(r.player, isBarbarian(r.player)) : "transparent" }, "aria-hidden": "true" });
}

/** Centres the map on a row's city. */
function centre(r) {
  if (r.x === null || r.x === undefined) return;
  emit("center", r.x, r.y);
  emit("flash", r.x, r.y);
}

/** One row of the rituals element. */
function rowEl(r) {
  const st = rowStatus(r);
  return h(
    `button.ritual-row${r.mine ? ".mine" : ""}${st.pending ? ".pending" : ""}`,
    {
      "data-player": r.player,
      "aria-label": rowLabel(r),
      title: r.city_name ? `${rowLabel(r)}. Click: show ${r.city_name} on the map.` : rowLabel(r),
      onclick: () => centre(r),
    },
    h("span.ritual-l1", swatch(r), h("span.ritual-name", chipName(r))),
    h(
      "span.ritual-l2",
      r.city_name ? h("span.ritual-city", r.city_name) : null,
      st.pending
        ? h("span.ritual-charge", "Cast pending")
        : h(
            "span.ritual-progress",
            barEl((r.percent ?? 0) / 100, "production", { w: 16, title: `${r.progress}/${r.cost}` }),
            h("span.ritual-pct", { "aria-label": `${r.percent}% of ${r.cost}` }, st.pct),
            st.turns ? h("span.ritual-turns", { "aria-label": `${st.turns} left` }, st.turns) : null,
          ),
    ),
  );
}

/** Redraws the rituals element from the view. */
export function renderRituals() {
  const box = document.getElementById("rituals");
  if (!box) return;
  const v = S.view;
  if (!v || observing()) {
    box.hidden = true;
    return;
  }
  const rows = activeRows(v.rituals);
  box.hidden = !(v.rituals ?? []).length;
  box.classList.toggle("has-rows", rows.length > 0);
  const all = h("button.link.small.ritual-all", { title: "Every civilization's ritual: in progress, casts pending, times completed, costs", "aria-label": "All rituals", onclick: () => openRitualsPanel() }, "All");
  fill(
    box,
    rows.length
      ? [h("div.ritual-head", h("span.ritual-title", "Rituals"), all), h("div.ritual-rows", rows.map(rowEl))]
      : h("div.ritual-head", h("span.ritual-none.dim", "No rituals in progress"), all),
  );
}

/** The full list: every civilization's ritual, idle ones too. */
export function openRitualsPanel() {
  const rows = S.view?.rituals ?? [];
  const m = openModal({ title: "World rituals", tag: "rituals", className: "rituals-panel", buttons: [{ label: "Close", value: null }] });
  m.setBody(
    rows.length
      ? h(
          "ul.rituals-list",
          rows.map((r) => {
            const [where, counts] = listLines(r);
            return h(
              `li.ritual-entry.${r.state}${r.mine ? ".mine" : ""}`,
              { "data-player": r.player },
              h("div.ritual-entry-head", swatch(r), h("b", r.name), h("span.dim", " · "), h("span", r.ritual_name)),
              h(
                "div.ritual-entry-where",
                where,
                r.x !== null && r.x !== undefined ? [" ", h("button.link.small", { title: `Show ${r.city_name} on the map`, "aria-label": `Show ${r.city_name} on the map`, onclick: () => (m.close(null), centre(r)) }, "⌖")] : null,
              ),
              r.note ? h("div.ritual-entry-note", r.note) : null,
              keywordChips(r.reminders),
              h("div.ritual-entry-counts.dim", counts),
            );
          }),
        )
      : h("p.dim", "No civilization has a world ritual."),
    h("p.dim.small", "Cost: what the ritual costs its civilization now. Next cost: after one more completion."),
  );
  return m;
}

/** Shows the ritual news among new log `entries` as alerts (`fresh`: the game's log read for the
 *  first time, when only the current turn's news is shown). Not while observing. */
export function ritualAlerts(entries, { fresh = false } = {}) {
  if (observing()) return;
  for (const e of alertsOf(entries, { fresh, turn: S.view?.turn ?? S.status?.turn })) {
    const at = Array.isArray(e.at) ? e.at : null;
    const el = h(
      `div.toast.log.ritual-alert.${e.verb ?? "news"}`,
      { role: "status", title: at ? "Click: show the city on the map" : "Click to dismiss" },
      h("span.ritual-alert-mark", { "aria-hidden": "true" }, "✶"),
      " ",
      markup(e.text),
    );
    if (at) el.addEventListener("click", () => (emit("center", at[0], at[1]), emit("flash", at[0], at[1])));
    toastNode(el, e.verb === "abandoned" ? 8000 : 12000);
  }
}
