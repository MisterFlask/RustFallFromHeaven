// Cultural actions (this game's own rules, docs/CULTURE.md): the city screen's cultural queue and
// the Cultural Pressure panel (F8).
//
// A city channels one action at a time from its cultural queue: the action's rate of culture a
// turn goes to it (not to the city's borders or the culture victory) for its duration, then the
// effect lands on the target. The city query's `cultural` lists the queue and, for each action,
// the targets in reach with the predicted effectiveness or the reason it cannot be done; the
// buttons send `culture_push`, `culture_pop` and `culture_repeat`, which the server checks.
// Every channel is public to the civilizations that have met its caster: the `culture` query.

import * as api from "./api.js";
import { dname, info, turns, playerName } from "./state.js";
import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { run } from "./commands.js";
import { infoTip } from "./infotips.js";

/** "Incite Unrest" (with the religion spread, if any). */
function actionName(o) {
  const name = dname("CulturalActionType", o.action);
  return o.religion !== null && o.religion !== undefined ? `${name} (${dname("ReligionType", o.religion)})` : name;
}

/** "3/4 turns", "paused", ... for a channel or a queued order. */
function progressText(o, head) {
  if (!head) return o.progress ? `waiting (${o.progress}/${o.duration} done)` : `waiting · ${turns(o.duration)}`;
  const left = Math.max(0, o.duration - o.progress);
  return `${o.progress}/${o.duration} · ${left ? `${turns(left)} left` : "completes"}`;
}

/** What a target does at the predicted effectiveness: "E 32% · +1 unhappy for 5 turns". */
function effectText(action, t) {
  const kind = info("CulturalActionType", action)?.effect;
  const lasting = t.effect_turns ? ` for ${turns(t.effect_turns)}` : "";
  const what =
    kind === "festival"
      ? `+${t.magnitude} happy, resists hostile actions${lasting}`
      : kind === "trade_fair"
        ? `trade routes +${t.magnitude}%${lasting}`
        : kind === "spread_religion"
          ? `${t.magnitude}% chance`
          : kind === "incite_unrest"
            ? `+${t.magnitude} unhappy${lasting}`
            : "";
  return `${t.effect < 100 ? `E ${t.effect}% · ` : ""}${what}`;
}

/** A channel's line for the panel and the city screen's pressure list. */
function channelLine(ch, { from = true, at = true } = {}) {
  return h(
    "li",
    { class: ch.hostile ? "hostile" : "" },
    h("span", { tip: infoTip("CulturalActionType", ch.action) }, actionName(ch)),
    from ? h("span.dim", ` from ${ch.city_name} (${playerName(ch.caster)})`) : null,
    at ? h("span.dim", ` on ${ch.target_name ?? "a city you have not seen"}${ch.target_owner !== null && ch.target_owner !== undefined ? ` (${playerName(ch.target_owner)})` : ""}`) : null,
    h("span", ` · ${ch.progress}/${ch.duration}${ch.turns_left ? `, ${turns(ch.turns_left)} left` : ""}`),
    ch.paused ? h("span.warn", " · paused") : null,
    ch.hostile ? h("span.badge.warn", "hostile") : null,
    ch.effect !== undefined ? h("span.dim", ` · E ${ch.effect}%`) : null,
  );
}

/** The city screen's cultural section: the queue, what is aimed at the city, its festivals and
 *  unrest, and the actions it could start. `reload` re-reads the screen. Null when the game has no
 *  cultural actions. */
export function cultureView(c, reload) {
  const k = c.cultural;
  if (!k) return null;
  const push = async (action, t, ev) => {
    const cmd = { c: "culture_push", city: c.id, action, target: t.city, religion: t.religion, repeat: !!ev.altKey, first: !!(ev.shiftKey || ev.ctrlKey || ev.metaKey) };
    if (await run(cmd)) reload();
  };
  const pop = async (index) => {
    if (await run({ c: "culture_pop", city: c.id, index })) reload();
  };
  const repeat = async (index, value) => {
    if (await run({ c: "culture_repeat", city: c.id, index, repeat: value })) reload();
  };
  const queue = h(
    "ol.culture-queue",
    k.queue.length
      ? k.queue.map((o, i) =>
          h(
            "li",
            h("span", { tip: infoTip("CulturalActionType", o.action) }, actionName(o)),
            h("span.dim", ` on ${o.target_name ?? "a lost city"} · ${progressText(o, i === 0)}`),
            i === 0 && o.paused ? h("span.warn", { title: "No progress while paused; after a few paused turns the channel is cancelled" }, ` · paused: ${o.paused}`) : null,
            h("label.toggle", { title: "Queue it again once it completes" }, h("input", { type: "checkbox", checked: o.repeat, onchange: (ev) => repeat(i, ev.target.checked) }), " repeat"),
            h("button.small", { title: i === 0 && o.started ? "Cancel the channel (nothing is refunded)" : "Remove from the queue", "aria-label": "Remove", onclick: () => pop(i) }, "✕"),
          ),
        )
      : h("li.dim", "Nothing: the culture grows the city's borders."),
  );
  const marks = (k.marks ?? []).map((m) => {
    const what = m.kind === "festival" ? `Festival (+${m.amount} happy, resists)` : m.kind === "trade_fair" ? `Trade fair with ${playerName(m.from)} (+${m.amount}% routes)` : m.kind === "unrest" ? `Unrest from ${playerName(m.from)} (+${m.amount} unhappy)` : m.kind;
    return h("span.chip", `${what}, ${turns(m.turns)}`);
  });
  const actions = k.actions.map((a) => {
    const ok = a.targets.filter((t) => t.ok);
    const not = a.targets.filter((t) => !t.ok);
    return h(
      "div.culture-action",
      h("div", h("b", { tip: infoTip("CulturalActionType", a.action) }, dname("CulturalActionType", a.action)), h("span.dim", ` · tier ${a.tier} · ${a.rate} culture a turn for ${turns(a.duration)} · reach ${a.range}`)),
      ok.length
        ? h(
            "div.build-grid",
            ok.map((t) =>
              h(
                "button.build",
                { title: "Click: queue it. Shift: put it first (the current channel waits). Alt: repeat it.", onclick: (ev) => push(a.action, t, ev) },
                h("span", `${t.name}${t.religion !== null && t.religion !== undefined ? ` (${dname("ReligionType", t.religion)})` : ""}`),
                h("span.dim", ` ${playerName(t.owner)} · ${effectText(a.action, t)}`),
                t.hostile ? h("span.badge.warn", "hostile") : null,
              ),
            ),
          )
        : null,
      not.length ? h("details", h("summary.dim", `${not.length} not possible`), h("ul.dim", not.map((t) => h("li", `${t.name}${t.religion !== null && t.religion !== undefined ? ` (${dname("ReligionType", t.religion)})` : ""}: ${t.why}`)))) : null,
    );
  });
  return h(
    "div.section.culture",
    h("h4", "Culture"),
    h("p.dim", `${k.rate} culture a turn${k.diverted ? `; ${k.diverted} went to the channel last turn` : ""}. One action channels at a time.`),
    queue,
    k.pressure.length ? h("div", h("b.warn", "Aimed at this city:"), h("ul.culture-pressure", k.pressure.map((ch) => channelLine(ch, { at: false })))) : null,
    marks.length ? h("div.chips", marks) : null,
    h("details", h("summary", "Start a cultural action"), actions),
  );
}

/** The Cultural Pressure panel (F8): every channel aimed at your cities, every channel you run,
 *  the others you see, and what other leaders remember of your actions. */
export async function openCulture() {
  const m = openModal({ title: "Cultural pressure", body: [h("p.dim", "…")], tag: "culture", wide: true, buttons: [{ label: "Close", value: null }] });
  let r;
  try {
    r = await api.query("culture");
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  if (m.closed) return m.done;
  if (!r.on) {
    m.setBody(h("p.dim", "This game has no cultural actions (the Cultural actions option is off)."));
    return m.done;
  }
  const list = (items, opts, empty) => (items.length ? h("ul.culture-list", items.map((ch) => channelLine(ch, opts))) : h("p.dim", empty));
  const memories = r.memories.length
    ? h(
        "ul",
        r.memories.map((x) => h("li", `${playerName(x.player)}: ${x.hostility ? `hostility ${x.hostility}` : ""}${x.hostility && x.friendship ? ", " : ""}${x.friendship ? `friendship ${x.friendship}` : ""}`)),
      )
    : h("p.dim", "None.");
  m.setBody(
    h("p.dim", "Every cultural action is public to the civilizations that have met its caster. A festival in a city resists hostile actions; so does its garrison, against unrest."),
    h("h4", `Aimed at your cities (${r.incoming.length})`),
    list(r.incoming, { at: true }, "Nothing."),
    h("h4", `Yours (${r.ours.length})`),
    list(r.ours, { from: true }, "None: queue them from a city's screen."),
    h("h4", `Between others (${r.others.length})`),
    list(r.others, {}, "None seen."),
    h("h4", "What other leaders remember of your cultural actions"),
    memories,
  );
  return m.done;
}
