// Screens and deferred popups (design §5.1 "Setup and files", "Popups and decisions",
// "Empire"): new game, load, save, game over, crash and stuck screens, the research chooser,
// the event and vote popups, the keys, and a raw request box for testers.

import * as api from "./api.js";
import { S, emit, dname, tname, turns, voteChoiceLabel, teamName, me, saveNameOfPath, scoreText } from "./state.js";
import { h, fill, toast, tapModes, coarse } from "./dom.js";
import { tip } from "./tips.js";
import { openModal, ask } from "./modals.js";
import { addHelp, openHelp } from "./help.js";
import { run } from "./commands.js";
import { render as markup, plain } from "./markup.js";
import { illuminate, bannerEl } from "./look.js";
import { leaderPortrait, teamPortrait, PORTRAIT } from "./portraits.js";
import { techTip, unlockRow } from "./empire.js";

// ---------------------------------------------------------------------------------------------
// Research (the CHOOSETECH popup and the research button).

/** The research chooser. Click: research it now; Shift-click: add it to the queue. With
 *  `popup` (research needed at a control point) it offers "Later" and closes once chosen.
 *  Resolves with "chosen", "later" or null. */
export async function openResearch({ popup = false } = {}) {
  const m = openModal({ title: popup ? "Choose research" : "Research", body: [h("p.dim", "…")], tag: "research", wide: true, buttons: popup ? [{ label: "Later", value: "later" }] : [{ label: "Close", value: null }] });
  let result = null;
  /** What a tap does on a touch screen (for want of Shift): "now" or "queue". */
  let tapMode = "now";
  const load = async () => {
    let t;
    try {
      t = await api.query("techs");
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    const can = t.techs.filter((x) => x.can_research && !x.has && !x.disabled);
    can.sort((a, b) => (a.turns ?? 1e9) - (b.turns ?? 1e9) || a.cost - b.cost);
    const cur = t.current !== null ? t.techs[t.current] : null;
    const head = h(
      "div.research-head",
      cur ? h("div", "Researching ", h("b", dname("TechType", t.current)), ` (${cur.progress}/${cur.cost}, ${turns(cur.turns)})`) : h("div.warn", "Nothing is being researched."),
      t.queue.length > 1 ? h("div.dim", `Queue: ${t.queue.map((q) => dname("TechType", q)).join(" → ")}`) : null,
      researchPathLine(t, load),
      h("div.dim", `${t.rate} research per turn. Click: research now. Shift-click: add to the queue.`),
      tapModes(tapMode, [["now", "Research now", "A tap researches it now"], ["queue", "Add to queue", "A tap adds it to the research queue (Shift-click)"]], (v) => ((tapMode = v), load())),
    );
    const list = h(
      "div.build-grid",
      can.map((x) => {
        return h(
          "button.build",
          {
            "data-tech": x.tech,
            class: x.researching ? "current" : x.queued !== null ? "queued" : "",
            tip: techTip(x, "Click: research now. Shift-click: add to the queue."),
            onclick: async (ev) => {
              const shift = !!ev.shiftKey || tapMode === "queue";
              const reply = await run({ c: "research", tech: x.tech, shift });
              if (!reply) return;
              if (popup && !shift) {
                result = "chosen";
                m.close("chosen");
              } else load();
            },
          },
          h("span", dname("TechType", x.tech)),
          h("span.dim", `${turns(x.turns)} · ${x.cost}${x.progress ? ` (${x.progress})` : ""}${bonusLabel(x)}`),
          unlockRow(x),
        );
      }),
    );
    m.setBody(head, can.length ? list : h("p.dim", "No technology can be researched now."));
  };
  load();
  const v = await m.done;
  return result ?? v;
}

/** The research bonuses on tech `x` of the `techs` query, for its card: " · path +100%",
 *  " · feature +100%", both multiplying (" · path +100% · feature +100% (x4)"); "" without. */
export function bonusLabel(x) {
  const parts = [x.path_bonus ? `path +${x.path_bonus}%` : null, x.feature_bonus ? `feature +${x.feature_bonus}%` : null].filter(Boolean);
  if (!parts.length) return "";
  const times = ((100 + (x.path_bonus ?? 0)) * (100 + (x.feature_bonus ?? 0))) / 10000;
  return ` · ${parts.join(" · ")}${parts.length > 1 ? ` (x${times})` : ""}`;
}

/** The research screen's line for the research path (docs/PORTING.md, "Research paths"): the
 *  path committed to for the rest of the game with its progress, or none, and a button that
 *  opens the path chooser; `reload` refreshes the research screen after a commitment. */
function researchPathLine(t, reload) {
  const p = pathOf(t, t.path);
  return h(
    "div.research-path",
    p
      ? h("span", "Research path: ", h("b", dname("ResearchPathType", p.path)), ` (+${t.path_percent}% research on its techs) · ${pathProgress(p)} `)
      : h("span.warn", "No research path. "),
    h("button.link", { "data-paths": "", onclick: async () => ((await openResearchPaths()) === "chosen" ? reload() : null) }, p ? "Paths" : "Choose a path"),
  );
}

/** Entry `path` of the `techs` query's `paths`, or null. */
function pathOf(t, path) {
  return path === null || path === undefined ? null : t.paths.find((p) => p.path === path) ?? null;
}

/** A path's progress: "3 of 9 techs known" or "complete". */
function pathProgress(p) {
  return p.complete ? "complete" : `${p.known.length} of ${p.techs.length - p.closed.length} techs known`;
}

/** A path's techs on one line: known ones ticked, those the human can never research marked. */
function pathTechs(p) {
  return p.techs.map((x) => (p.known.includes(x) ? `${dname("TechType", x)} ✓` : p.closed.includes(x) ? `${dname("TechType", x)} (closed)` : dname("TechType", x))).join(" · ");
}

/** The research path chooser. Before the commitment: every path with its techs and the research
 *  left on them, a refused one greyed with its reason; a click asks for confirmation, then
 *  commits for the rest of the game. After: the committed path on top with its progress, the
 *  others below for reference. Resolves "chosen" or null. */
export async function openResearchPaths() {
  let result = null;
  const m = openModal({ title: "Research paths", body: [h("p.dim", "…")], tag: "research-paths", wide: true, buttons: [{ label: "Close", value: null }] });
  let t;
  try {
    t = await api.query("techs");
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  const name = (p) => dname("ResearchPathType", p.path);
  const mine = pathOf(t, t.path);
  if (mine) {
    // Committed: no choice left; the path, then the others as reference.
    m.setBody(
      h("p.dim", `One path for the rest of the game: its techs are researched ${t.path_percent}% faster.`),
      h(
        "div.path-card.current",
        { "data-path": mine.path },
        h("span", h("b", name(mine)), " (committed)"),
        h("small", pathTechs(mine)),
        h("small", mine.complete ? "Complete: every tech known." : `${pathProgress(mine)} · Research left: ${mine.left}`),
      ),
      h("h4", "Other paths"),
      h(
        "div.path-list",
        t.paths
          .filter((p) => p.path !== mine.path)
          .map((p) => h("div.path-card.other", { "data-path": p.path }, h("span", h("b", name(p))), h("small", pathTechs(p)), h("small", `Research left: ${p.left}`))),
      ),
    );
    const v = await m.done;
    return result ?? v;
  }
  const commit = async (p) => {
    const answer = await ask(`Commit to ${name(p)}?`, [`Its techs are researched ${t.path_percent}% faster for the rest of the game.`, "No other path can be chosen afterwards."], [
      { label: "Commit", value: "commit", primary: true },
      { label: "Cancel", value: null },
    ]);
    if (answer !== "commit") return;
    const reply = await run({ c: "research_path", path: p.path });
    if (reply) {
      result = "chosen";
      m.close("chosen");
    }
  };
  const rows = t.paths.map((p) =>
    h(
      "button.option",
      { "data-path": p.path, disabled: !p.can_commit, onclick: () => commit(p) },
      h("span", h("b", name(p))),
      h("small", pathTechs(p)),
      h("small", `Research left: ${p.left}`),
      p.refusal ? h("small.dim", p.refusal) : null,
    ),
  );
  m.setBody(
    h("p.dim", `Choose one path for the rest of the game: its techs are researched ${t.path_percent}% faster. The choice cannot be changed.`),
    h("div.options", rows),
  );
  const v = await m.done;
  return result ?? v;
}

// ---------------------------------------------------------------------------------------------
// Events, votes and elections (deferred popups, design §2.3).

/** An event popup (`pending.events[]`). Resolves "chosen", "later" or null. */
export async function openEvent(ev) {
  let result = null;
  const err = h("p.error");
  const options = h(
    "div.options",
    ev.options.map((o) =>
      h(
        "button.option",
        {
          disabled: !o.enabled,
          "data-event": o.event,
          onclick: async () => {
            const reply = await run({ c: "event", triggered: ev.triggered, event: o.event });
            if (reply) {
              result = "chosen";
              m.close("chosen");
            }
          },
        },
        h("span", markup(dname("EventType", o.event))),
        o.effects?.length ? h("small", o.effects.map((e) => plain(e)).join(" · ")) : null,
      ),
    ),
  );
  const where = ev.plot ? h("button.link", { onclick: () => emit("center", ev.plot[0], ev.plot[1]) }, "Show on the map") : null;
  const m = openModal({
    title: dname("EventTriggerType", ev.trigger),
    body: [h("p.page-text", illuminate(markup(ev.text))), where, options, err],
    buttons: [{ label: "Later", value: "later", title: "Answer later (ending the turn without an answer drops the event)" }],
    tag: "event",
    className: "event",
  });
  const v = await m.done;
  return result ?? v;
}

/** A vote popup (`pending.votes[]`): a candidate (a team) by its leader's portrait. Resolves
 *  "chosen", "later" or null. */
export async function openVote(v) {
  let result = null;
  const choices = h(
    "div.options",
    v.choices.map((c) =>
      h(
        "button.option",
        {
          "data-choice": c,
          onclick: async () => {
            const reply = await run({ c: "vote", vote: v.vote, choice: c });
            if (reply) {
              result = "chosen";
              m.close("chosen");
            }
          },
        },
        c >= 0 ? h("span.vote-choice", teamPortrait(c, { size: PORTRAIT.small }), h("span", voteChoiceLabel(c))) : voteChoiceLabel(c),
      ),
    ),
  );
  const m = openModal({
    title: dname("VoteType", v.resolution),
    body: [h("p.dim", dname("VoteSourceType", v.source)), h("p.page-text", illuminate(markup(v.text))), choices],
    buttons: [{ label: "Later", value: "later", title: "Unanswered votes get one more turn, then abstain" }],
    tag: "vote",
    className: "event",
  });
  const r = await m.done;
  return result ?? r;
}

/** The secretary general's election (`pending.elections[]`, C++ BUTTONPOPUP_CHOOSEELECTION):
 *  one button per resolution, then "None". Resolves "chosen", "later" or null. */
export async function openElection(e) {
  let result = null;
  const err = h("p.error");
  const send = async (option) => {
    const reply = await run({ c: "election", selection: e.selection, option });
    if (reply) {
      result = "chosen";
      m.close("chosen");
    }
  };
  const options = h(
    "div.options",
    e.options.map((o, i) => h("button.option", { "data-option": i, onclick: () => send(i) }, h("span", markup(o.text || dname("VoteType", o.vote))))),
    h("button.option", { "data-option": "none", onclick: () => send(null) }, h("span", "None")),
  );
  const m = openModal({
    title: dname("VoteSourceType", e.source),
    body: [h("p", "As secretary general, choose the resolution to put to the vote."), options, err],
    buttons: [{ label: "Later", value: "later", title: "Answer later (ending the turn without an answer puts no resolution to the vote)" }],
    tag: "election",
  });
  const r = await m.done;
  return result ?? r;
}

// ---------------------------------------------------------------------------------------------
// New game, load, save.

function select(name, list, value, { none = null } = {}) {
  return h(
    "select",
    { name, id: `ng-${name}` },
    none ? h("option", { value: "" }, none) : null,
    list.map((e) => h("option", { value: e.type, selected: e.id === value }, e.name)),
  );
}

/** The tag of the New game dialog opened from the title screen with no game; main.js closes it
 *  once a game exists (also one another client started). One opened over a game is "newgame". */
export const NEWGAME_REQUIRED = "newgame-required";

/** The first paragraph of a civilization's pedia text (its trait and how it plays), or null. */
function civSummary(pedia) {
  if (!pedia) return null;
  return pedia.split(/\[PARAGRAPH:\d+\]/)[0].replace(/\[TAB\]/g, "").replace(/(\[NEWLINE\]){2,}/g, "[NEWLINE]");
}

/** The new-game screen (design §5.1): the civilization (a list with its banners; the chosen one's
 *  summary and what it brings) and its leader (cards with their portraits; the chosen one's
 *  portrait and traits), or random; then the world: opponents, size, speed, difficulty, era,
 *  climate, sea level, seed and the game's name; the game options and victories. And the debug
 *  start at war (`session war_start`: these settings, an AI-only game played until a battle, the
 *  attacked civilization yours just before it). */
export function newGame({ dismissible = true, noGame = false } = {}) {
  const st = S.setup;
  if (!st) return null;
  const d = st.defaults;
  // The chosen civilization and leader (type names; "" is random) live in the two <select>s, kept
  // as the form's values (hidden: the list and the cards drive them).
  const civ = select("civ", st.civilizations, d.civ, { none: "Random" });
  civ.hidden = true;
  const leader = h("select", { name: "leader", id: "ng-leader", hidden: true });
  const civList = h("div.civ-list", { role: "listbox", "aria-label": "Civilization" });
  const civInfo = h("div.civ-info");
  const civRow = (type, name) =>
    h(
      "button.civ-item",
      { role: "option", "data-civ": type, title: name, onclick: () => pickCiv(type) },
      bannerEl(type || "CIVILIZATION_RANDOM", { u: 2 }) ?? h("span.civ-swatch"),
      h("span.civ-name", name),
    );
  const civs = [...st.civilizations].sort((x, y) => x.name.localeCompare(y.name));
  civList.append(civRow("", "Random"), ...civs.map((c) => civRow(c.type, c.name)));
  const leaderOf = (type) => (type ? st.leaders.find((x) => x.type === type) : null);
  const leaderCard = (l) =>
    h(
      "button.leader-card",
      { "data-leader": l ? l.type : "", title: l ? l.name : "A random leader", onclick: () => pickLeader(l ? l.type : "") },
      leaderPortrait(l ? l.type : "LEADER_RANDOM", { size: PORTRAIT.small, title: l ? l.name : "A random leader" }),
      h("span.leader-name", l ? l.name : "Random"),
    );
  // The chosen leader's portrait (a hooded stranger while it is random), name and traits.
  const face = h("div.ng-portrait");
  const who = h("div.ng-who");
  const about = h("div.ng-about");
  const showLeader = () => {
    const c = st.civilizations.find((x) => x.type === civ.value);
    const l = leaderOf(leader.value);
    face.replaceChildren(leaderPortrait(leader.value || "LEADER_RANDOM", { size: PORTRAIT.large, title: l?.name ?? "A random leader" }) ?? "");
    fill(who, h("div.ng-leader-name", l ? l.name : c ? "A random leader" : "A random civilization"), l?.help ? h("div.ng-leader", markup(l.help)) : h("div.dim", c ? "Any of this civilization's leaders." : "Any civilization, with any of its leaders: a surprise at the first turn."));
    for (const b of civInfo.querySelectorAll("button.leader-card")) b.classList.toggle("chosen", b.dataset.leader === leader.value);
  };
  const pickLeader = (type) => {
    leader.value = type;
    showLeader();
  };
  const pickCiv = (type) => {
    civ.value = type;
    for (const b of civList.querySelectorAll("button.civ-item")) {
      const on = b.dataset.civ === type;
      b.classList.toggle("chosen", on);
      b.setAttribute("aria-selected", String(on));
    }
    const c = st.civilizations.find((x) => x.type === type);
    leader.replaceChildren(h("option", { value: "" }, "Random"), ...(c ? c.leaders.map((l) => h("option", { value: st.leaders[l].type }, st.leaders[l].name)) : []));
    const summary = civSummary(c?.pedia);
    // What the chosen civilization brings (the server's C++ `parseCivInfos`), its pedia's first
    // paragraph above it and the whole pedia folded below.
    fill(
      about,
      summary ? h("div.civ-summary", markup(summary)) : null,
      c?.help ? h("div.ng-civ", markup(c.help)) : null,
      c?.pedia ? h("details.ng-pedia", h("summary", "From the Civilopedia"), h("div", markup(c.pedia))) : null,
    );
    fill(
      civInfo,
      h(
        "div.civ-head",
        face,
        h("div.civ-head-text", h("div.civ-title", bannerEl(type || "CIVILIZATION_RANDOM", { u: 3 }), h("h3", c ? c.name : "Random")), who),
      ),
      c ? h("div.leader-cards", leaderCard(null), c.leaders.map((l) => leaderCard(st.leaders[l]))) : null,
      about,
    );
    pickLeader("");
  };
  // A list's selected entry's help, as the select's tooltip.
  const helpTip = (list, el) => () => {
    const e = list.find((x) => x.type === el.value);
    return e ? [h("div.tip-title", e.name), e.help ? h("div.tip-effects", markup(e.help)) : h("div.dim", "No description.")] : null;
  };
  const seed = h("input", { type: "number", id: "ng-seed", min: 0, max: 4294967295, value: d.seed });
  const dice = h("button.small", { type: "button", title: "A random seed", onclick: () => (seed.value = String(Math.floor(Math.random() * 1e6))) }, "⚄");
  const opponents = h("input", { type: "number", id: "ng-opponents", min: 1, max: st.max_opponents, value: d.opponents });
  const world = select("world", st.worlds, d.world);
  const speed = select("speed", st.speeds, d.speed);
  const handicap = select("handicap", st.handicaps, d.handicap);
  const era = select("era", st.eras, d.era);
  const climate = select("climate", st.climates, d.climate);
  const sea = select("sea_level", st.sea_levels, d.sea_level);
  for (const [el, list] of [[world, st.worlds], [speed, st.speeds], [handicap, st.handicaps], [era, st.eras], [climate, st.climates], [sea, st.sea_levels]]) tip(el, helpTip(list, el));
  const name = h("input", { type: "text", id: "ng-name", placeholder: "game-<seed>", pattern: "[A-Za-z0-9_.\\-]{1,64}", title: "The game's directory under the saves directory" });
  const options = h(
    "div.checks",
    st.options.map((o) => h("label", { tip: [h("div.tip-title", o.name), o.help ? h("div.tip-effects", markup(o.help)) : h("div.dim", "No description.")] }, h("input", { type: "checkbox", value: o.type, checked: d.options.includes(o.id) }), ` ${o.name}`)),
  );
  const victories = h(
    "div.checks",
    st.victories.map((v) => h("label", { tip: v.help ? [h("div.tip-title", v.name), h("div.tip-effects", markup(v.help))] : null }, h("input", { type: "checkbox", value: v.type, checked: !d.disabled_victories.includes(v.id) }), ` ${v.name}`)),
  );
  const spectate = h("input", { type: "checkbox", id: "ng-spectate" });
  const vanilla = h("input", { type: "checkbox", id: "ng-vanilla", checked: !!d.vanilla });
  const err = h("p.error");
  const note = h("p.dim#ng-war-note");
  const field = (label, el, extra = null) => h("label.field", h("span", label), el, extra);
  const form = h(
    "div.newgame",
    h("div.ng-civ", h("div.ng-civ-col", h("h4", "Civilization"), civList, civ, leader), civInfo),
    h("h4", "World"),
    h(
      "div.fields",
      field("Opponents", opponents),
      field("World size", world),
      field("Speed", speed),
      field("Difficulty", handicap),
      field("Era", era),
      field("Climate", climate),
      field("Sea level", sea),
      field("Seed", h("div.inline", seed, dice)),
      field("Game name", name),
    ),
    h(
      "div.ng-more",
      h("details", h("summary", "Game options"), options),
      h("details", h("summary", "Victory conditions"), victories),
      h(
        "label.check",
        { title: "An AI-only game watched with the whole map shown (the AI plays every civilization; the one chosen above is followed first): play, pause, step, follow a civilization, and flag bad play for a bug report from the top bar. It is the `ffh autoplay` game of these settings." },
        spectate,
        " Spectate the AI",
      ),
      h(
        "label.check",
        { title: "Vanilla experience: leaders speak only FFH2's own lines. Unticked, each civilization also has lines of its own for every other civilization." },
        vanilla,
        " Vanilla experience",
      ),
    ),
    err,
    note,
  );
  pickCiv(civ.value);
  const setupOf = () => {
    const setup = {
      seed: Number(seed.value),
      opponents: Number(opponents.value),
      world: world.value,
      speed: speed.value,
      handicap: handicap.value,
      era: era.value,
      climate: climate.value,
      sea_level: sea.value,
      options: [...options.querySelectorAll("input:checked")].map((i) => i.value),
      disabled_victories: [...victories.querySelectorAll("input:not(:checked)")].map((i) => i.value),
    };
    if (civ.value) setup.civ = civ.value;
    if (leader.value) setup.leader = leader.value;
    if (vanilla.checked) setup.vanilla = true;
    return setup;
  };
  const start = async (m) => {
    const setup = setupOf();
    err.textContent = "";
    try {
      if (spectate.checked) setup.observe = true;
      await api.session("new", name.value ? { setup, name: name.value } : { setup });
      m.close("started");
      emit("game-started", { spectate: spectate.checked });
    } catch (e) {
      err.textContent = e.detail ?? e.message;
    }
  };
  // The war start plays an AI-only game for a few hundred turns: seconds, up to a minute on a big
  // map; its turn comes in the status (`war_search`) meanwhile. It is always played, never
  // spectated.
  const warStart = async (m) => {
    const setup = setupOf();
    const what = "Playing an AI game until two civilizations at war fight with 3 or more units a side";
    err.textContent = "";
    note.textContent = `${what}…`;
    const timer = setInterval(() => {
      const t = S.status?.war_search?.turn;
      if (t !== undefined && t !== null) note.textContent = `${what}: turn ${t}…`;
    }, 250);
    try {
      const reply = await api.session("war_start", name.value ? { setup, name: name.value } : { setup });
      m.close("started");
      emit("game-started", { war: reply.r.war });
    } catch (e) {
      err.textContent = e.detail ?? e.message;
    } finally {
      clearInterval(timer);
      note.textContent = "";
    }
  };
  return openModal({
    title: "New game",
    body: [form],
    tag: noGame || !dismissible ? NEWGAME_REQUIRED : "newgame",
    wide: true,
    className: "newgame-screen",
    dismissible,
    buttons: [
      { label: "Load a game…", onClick: () => loadScreen() },
      {
        label: "Start at war (debug)",
        value: "war",
        title: "An AI game of these settings is played until two civilizations at war fight a battle with 3 or more units a side; you take over the attacked one just before the attack (the civilization: yours if chosen, else whichever is attacked)",
        onClick: warStart,
      },
      { label: "Start", primary: true, onClick: start },
    ],
  });
}

/** A save's turn, when its name tells it: an autosave `auto/T0061`, `auto/start` (turn 0), or a
 *  save named as the save dialog names it (`turn-12`); else null. */
export function saveTurn(name) {
  const m = /(?:^|\/)T(\d+)$/.exec(name) ?? /^turn-(\d+)$/.exec(name);
  if (m) return Number(m[1]);
  return /(?:^|\/)start$/.test(name) ? 0 : null;
}

/** "just now", "5 min ago", "3 h ago", "yesterday", else the date; "" without a time. */
export function whenText(secs, now = Date.now()) {
  if (secs === null || secs === undefined) return "";
  const ago = Math.max(0, now / 1000 - secs);
  if (ago < 60) return "just now";
  if (ago < 3600) return `${Math.floor(ago / 60)} min ago`;
  if (ago < 86400) return `${Math.floor(ago / 3600)} h ago`;
  if (ago < 172800) return "yesterday";
  return new Date(secs * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

/** The load screen: every game (the current one first, then the most recently saved) with its
 *  saves, newest first; the autosaves folded under the newest one. */
export async function loadScreen() {
  const m = openModal({ title: "Load a game", body: [h("p.dim", "…")], tag: "load", wide: true, className: "load-screen", buttons: [{ label: "Close", value: null }] });
  let r;
  try {
    r = (await api.session("saves")).r;
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  const err = h("p.error");
  // One load at a time: a double click sends one request (P5 review).
  let loading = false;
  const load = async (name) => {
    if (loading) return;
    loading = true;
    const buttons = [...m.el.querySelectorAll("button.save")];
    buttons.forEach((b) => (b.disabled = true));
    err.textContent = "";
    try {
      await api.session("load", { name });
      m.close("loaded");
      emit("game-started");
    } catch (e) {
      err.textContent = e.detail ?? e.message;
    } finally {
      loading = false;
      if (!m.closed) buttons.forEach((b) => (b.disabled = false));
    }
  };
  const newest = (g) => Math.max(-1, ...g.saves.map((s) => s.modified ?? -1));
  const order = (a, b) => (b.modified ?? -1) - (a.modified ?? -1) || (saveTurn(b.name) ?? -1) - (saveTurn(a.name) ?? -1) || b.name.localeCompare(a.name);
  const games = [...r.games].sort((a, b) => (a.game === r.current ? -1 : b.game === r.current ? 1 : newest(b) - newest(a) || a.game.localeCompare(b.game)));
  const row = (g, s) => {
    const turn = saveTurn(s.name);
    const label = s.auto ? (s.name === "auto/start" ? "Game start" : "Autosave") : s.name;
    const when = s.modified ? new Date(s.modified * 1000).toLocaleString() : "";
    return h(
      "button.save",
      { "data-save": `${g.game}/${s.name}`, title: `${s.path}${when ? `\n${when}` : ""}`, onclick: () => load(`${g.game}/${s.name}`) },
      h("span.save-name", label, s.auto ? h("span.badge", "auto") : null),
      h("span.save-turn", turn !== null ? `Turn ${turn}` : ""),
      h("span.save-when", whenText(s.modified)),
    );
  };
  const list = games.map((g) => {
    const mine = [...g.saves].filter((s) => !s.auto).sort(order);
    const auto = [...g.saves].filter((s) => s.auto).sort(order);
    const count = `${mine.length} save${mine.length === 1 ? "" : "s"}, ${auto.length} autosave${auto.length === 1 ? "" : "s"}`;
    return h(
      "section.save-game",
      h("div.save-game-head", h("h4", g.game), g.game === r.current ? h("span.badge", "current") : null, h("span.dim.save-count", count)),
      g.saves.length
        ? h(
            "div.save-rows",
            mine.map((s) => row(g, s)),
            auto.length ? row(g, auto[0]) : null,
            auto.length > 1 ? h("details.save-more", h("summary", `${auto.length - 1} older autosave${auto.length === 2 ? "" : "s"}`), h("div.save-rows", auto.slice(1).map((s) => row(g, s)))) : null,
          )
        : h("p.dim", "No saves."),
    );
  });
  m.setBody(err, list.length ? list : h("p.dim", "No saved games yet: start a new game; it autosaves every turn."), h("p.dim.save-root", `Saves directory: ${r.root}`));
  return m.done;
}

/** The save dialog. */
export function saveDialog() {
  const name = h("input", { type: "text", id: "save-name", value: `turn-${S.status?.turn ?? 0}`, pattern: "[A-Za-z0-9_.\\-]{1,64}" });
  const err = h("p.error");
  const save = async (m) => {
    try {
      const r = (await api.session("save", { name: name.value })).r;
      toast(`Saved: ${r.save}`);
      m.close("saved");
    } catch (e) {
      err.textContent = e.detail ?? e.message;
    }
  };
  const m = openModal({
    title: "Save the game",
    body: [h("label.field", h("span", "Name"), name), h("p.dim", "Letters, digits, '.', '_' and '-'. Saved in the game's directory."), err],
    tag: "save",
    buttons: [
      { label: "Save", primary: true, onClick: save },
      { label: "Cancel", value: null },
    ],
  });
  // Enter is the Save button (one request at a time, as a click).
  name.addEventListener("keydown", (ev) => ev.key === "Enter" && m.el.querySelector(".modal-buttons button.primary")?.click());
  name.select();
  return m;
}

// ---------------------------------------------------------------------------------------------
// The end: game over, crash, stuck.

/** The game-over screen: winner, victory type, scores. */
export function gameOver(st) {
  const over = st.over ?? {};
  const mine = me();
  const won = over.winner_team !== null && over.winner_team !== undefined && mine && over.winner_team === mine.team;
  const title = won ? "Victory!" : mine?.alive === false ? "Your civilization has fallen" : "The game is over";
  const rows = [];
  if (mine) rows.push({ name: `${mine.name} (you)`, score: mine.score, alive: mine.alive });
  for (const p of S.players.values()) if (!p.barbarian) rows.push({ name: p.name, score: p.score, alive: p.alive });
  rows.sort((a, b) => b.score - a.score);
  const result = [
    over.winner_team !== null && over.winner_team !== undefined ? `Winner: ${teamName(over.winner_team)}` : "No winner",
    over.victory !== null && over.victory !== undefined ? `${dname("VictoryType", over.victory)} victory` : null,
    `turn ${st.turn}`,
  ].filter(Boolean).join(" · ");
  return openModal({
    title,
    tag: "over",
    wide: true,
    className: `game-over${won ? " won" : ""}`,
    body: [
      h(
        "div.over-head",
        mine ? bannerEl(tname("CivilizationType", mine.civ), { u: 6, title: dname("CivilizationType", mine.civ) }) : null,
        h("div", h("div.over-title", title), h("div.over-result", result)),
      ),
      h("table.scores", h("tr", h("th", "Player"), h("th.num", "Score")), rows.map((r) => h("tr", { class: r.alive ? "" : "dead" }, h("td", r.name), h("td.num", scoreText(r.score))))),
    ],
    buttons: [
      { label: "New game", primary: true, onClick: () => newGame() },
      { label: "Load…", onClick: () => loadScreen() },
      { label: "View the map", value: null },
    ],
  });
}

/** The crash screen (design §2.8): the message and the bug report with the replay command. */
export function crashScreen(st) {
  const c = st.crash ?? {};
  const auto = saveNameOfPath(c.autosave);
  const report = [`ffh-play crash: ${c.message ?? "?"}`, `at ${c.location ?? "?"}`, `autosave: ${c.autosave ?? "-"}`, `journal: ${c.journal ?? "-"}`, `repro: ${c.repro ?? "-"}`].join("\n");
  const pre = h("pre.report", report);
  const copy = h("button.small", { onclick: () => navigator.clipboard?.writeText(report).then(() => toast("Copied.")).catch(() => toast("Select the text to copy it.", "error")) }, "Copy");
  return openModal({
    title: "The game crashed",
    tag: "crash",
    wide: true,
    dismissible: false,
    body: [h("p", "A rules error stopped the game. The server is still up: load the last autosave to go on, and please report the bug with the text below."), pre, copy],
    buttons: [
      auto ? { label: "Load the autosave", primary: true, onClick: async (m) => {
        try {
          await api.session("load", { name: auto });
          m.close("loaded");
          emit("game-started");
        } catch (e) {
          toast(e.detail ?? e.message, "error");
        }
      } } : null,
      { label: "Load…", onClick: () => loadScreen() },
      { label: "New game", onClick: () => newGame() },
    ].filter(Boolean),
  });
}

/** The stuck screen: the watchdog stopped a turn that did not finish. */
export function stuckScreen(st) {
  return openModal({
    title: "The turn did not finish",
    tag: "stuck",
    wide: true,
    dismissible: false,
    body: [h("p", "The AI turns ran too long without finishing (a rules or AI bug). Load an autosave to go on."), h("pre.report", String(st.stuck ?? ""))],
    buttons: [
      { label: "Load…", primary: true, onClick: () => loadScreen() },
      { label: "New game", onClick: () => newGame() },
    ],
  });
}

// ---------------------------------------------------------------------------------------------
// Help and testing tools.

/** The keys: Civ4's (the action hotkeys come from the game's XML, as each button's tooltip shows). */
function keysTable() {
  const touch = [
    ["Touch", ""],
    ["Tap", "select units (again: the next group there), open your city, show a plot"],
    ["Tap a plot, tap it again", "with units selected: the first tap shows the way there (turns, attack odds), the second goes"],
    ["Drag, pinch", "pan and zoom the map; the minimap centres"],
    ["☰, ▾, ×", "the screens and files; fold the unit panel; deselect"],
    ["A tap: …", "in the city, research and tech screens: what a tap does (now, add to the queue, put first), where a keyboard holds Shift or Ctrl"],
  ];
  const keys = [
    ...(coarse() ? touch : []),
    ["Mouse", ""],
    ["Left click", "select units (again: the next group there), open your city, show a plot"],
    ["Right click", "move or attack with the selection; with Shift held, add waypoints (sent when Shift is released)"],
    ["Drag, wheel, arrows, + −", "pan and zoom the map; the minimap centres"],
    ["W A S D (held)", "scroll the map smoothly (ffh-play's: Civ4's W, A, S and D orders are on their buttons)"],
    ["Q / E", "zoom out / in (ffh-play's: Civ4's Q quarry and E explore are on their buttons)"],
    ["Turn and selection (Civ4's controls)", ""],
    ["Enter", "the next unit waiting for orders; when none waits (the End turn button glows), end the turn"],
    ["Shift+Enter", "end the turn whatever waits, without the checks"],
    ["Space", "with no unit selected, end the turn when none waits"],
    ["C", "centre on the selection"],
    ["Home / End", "the next / previous city (in the city screen: its next / previous city)"],
    ["F10", "the capital's city screen"],
    ["Esc", "cancel a plot pick, close a dialog, deselect"],
    ["ffh-play's own", ""],
    ["Tab / Shift+Tab", "the next / previous unit waiting (Civ4's W; in Civ4, Tab is chat): a stack whose units cannot all move is skipped; when it wraps, the automated units move"],
    ["F (watching the AI)", "flag the plot under the mouse: record it for a bug report (the turn, the units, the replay commands)"],
    ["Enter (an observed game)", "play the next turn"],
    ["Orders (Civ4's hotkeys)", ""],
    ["Space", "skip the turn"],
    ["F / H", "fortify (sleep for units that cannot) / heal; sentry is on its button (S scrolls)"],
    ["B", "found a city (bombard for siege units)"],
    ["G then click, Alt+R", "go to a plot (Shift-click: add waypoints, sent when Shift is released) / build a route to it"],
    ["R, I, M, …", "the worker's builds: road, farm, mine, … (each button shows its key)"],
    ["L / U / Shift+U", "load / unload / unload all"],
    ["Shift+P", "pillage"],
    ["Del / Backspace", "delete the units (asks first) / cancel the orders"],
    ["Screens", ""],
    ["F1 / F2 / F3 / F4", "cities / finances (the sliders) / civics / diplomacy and trade"],
    ["F5", "armies: your armies, their supply and the units on their way (this game's)"],
    ["F9", "governors: who runs your cities, their modes, the bench (this game's)"],
    ["F6 / F7 / F8", "the tech tree / religion / cultural pressure (ffh-play's)"],
    ["Ctrl+S / Ctrl+L", "save / load"],
    ["Ctrl+1 / 2 / 3", "map layer: terrain / political / yields (ffh-play's)"],
    ["?", "this list (ffh-play's)"],
  ];
  return h("table.kv", keys.map(([k, v]) => (v ? h("tr", h("th", k), h("td", v)) : h("tr", h("th.group", { colspan: 2 }, k)))));
}

addHelp({ id: "keys", group: "Playing", title: coarse() ? "Controls" : "Keys", order: 0, body: () => keysTable() });

/** The help display at the keys (the ? button and key). */
export function keysHelp() {
  return openHelp("keys");
}

/** A box for raw protocol requests (design §5.1 "Rules-testing tools"). */
export function rawRequest() {
  const box = h("textarea.raw", { rows: 4, spellcheck: false }, '{"k":"q","q":"pending"}');
  const out = h("pre.report");
  const send = async () => {
    try {
      out.textContent = JSON.stringify(await api.send(JSON.parse(box.value)), null, 2);
    } catch (e) {
      out.textContent = String(e.message ?? e);
    }
    emit("kick");
  };
  return openModal({ title: "Raw request", tag: "raw", wide: true, body: [box, out], buttons: [{ label: "Send", primary: true, onClick: send }, { label: "Close", value: null }] });
}

