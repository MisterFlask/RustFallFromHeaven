// The rules-testing tools (design §5.1 "Rules-testing tools", §7 P6): the raw inspector
// (`inspect`), debug reveal (`session debug`), AI autoplay (`autoplay`), taking over a slot of an
// AI save (`session take_over`), the player options (`set_option`), the raw request box, and the
// optional spectator mode: the AI plays every civilization (the human slot too, through FfH's
// AIAutoPlay one turn at a time) with the whole map shown, paused or stepped at will.

import * as api from "./api.js";
import { S, emit, dname, tidx, me, observing, unitsAt, playerName } from "./state.js";
import { h, fill, toast } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { run, sending } from "./commands.js";
import { rawRequest, loadScreen } from "./screens.js";
import { plotIndex } from "./tiles.js";
import { look, setRendererChoice, EXPERIMENTAL_3D, EXPERIMENTAL_HD2D } from "./look.js";

/** The tools menu. */
export function openTools() {
  const st = S.status ?? {};
  const item = (label, help, f, extra = {}) => h("button.tool", { onclick: () => (closeTag("tools"), f()), title: help, ...extra }, h("b", label), h("span.dim", help));
  return openModal({
    title: "Tools",
    tag: "tools",
    body: [
      h(
        "div.tools",
        item(st.reveal ? "Debug reveal: on" : "Debug reveal: off", "Show the whole map and every unit and city (marked DEBUG REVEAL)", () => setReveal(!st.reveal), { id: "tool-reveal" }),
        item("Inspector…", "The rules engine's own data of a unit, group, city, plot, player, team, deal or info entry", () => openInspector(), { id: "tool-inspect" }),
        item("AI autoplay…", "The AI plays your civilization for a number of turns", () => openAutoplay(), { id: "tool-autoplay" }),
        item(spectating() ? "Stop spectating" : "Spectate", "The AI plays every civilization, one turn at a time, with the whole map shown", () => (spectating() ? stopSpectating() : startSpectating()), { id: "tool-spectate" }),
        item("Take over…", "Play another slot of an AI-only save", () => openTakeOver(), { id: "tool-takeover" }),
        item("Options…", "Player options: quick moves, stack attack, no unit cycling, …", () => openOptions(), { id: "tool-options" }),
        item("Raw request…", "Send any protocol request", () => rawRequest(), { id: "tool-raw" }),
      ),
    ],
    buttons: [{ label: "Close", value: null }],
  });
}

/** Debug reveal on or off. */
export async function setReveal(on) {
  try {
    await api.session("debug", { reveal: on });
    toast(on ? "Debug reveal: the whole map is shown." : "Debug reveal off.");
  } catch (e) {
    toast(e.detail ?? e.message, "error");
  }
  emit("kick");
}

// ---------------------------------------------------------------------------------------------
// The inspector.

/** The objects the selection, the open city and the hovered plot give the inspector. */
function quickPicks() {
  const out = [];
  const sel = S.sel;
  if (sel) {
    out.push(["unit", sel.units[0], `the selected ${dname("UnitType", S.units.get(sel.units[0])?.type)}`]);
    const g = S.units.get(sel.units[0])?.group;
    if (g !== undefined && g !== null) out.push(["group", g, "its group"]);
    const city = [...S.cities.values()].find((c) => c.x === sel.x && c.y === sel.y && c.owner === me()?.player);
    if (city) out.push(["city", city.id, city.name]);
    if (S.status?.reveal && S.map) out.push(["plot", plotIndex(S.map, sel.x, sel.y), `the plot (${sel.x}, ${sel.y})`]);
  }
  if (me()) {
    out.push(["player", me().player, "your player"]);
    out.push(["team", me().team, "your team"]);
  }
  return out;
}

/** The inspector: `{:#?}` of a rules object (`inspect {kind, oid}`). Your own objects, any
 *  under debug reveal (a plot only then); an info entry by its kind (`UnitType`) and index. */
export function openInspector(kind = "unit", oid = null) {
  const kinds = ["unit", "group", "city", "plot", "player", "team", "deal", ...(S.data?.kinds ?? [])];
  const kindSel = h("select#inspect-kind", kinds.map((k) => h("option", { value: k, selected: k === kind }, k)));
  const oidIn = h("input#inspect-oid", { type: "number", min: 0, value: oid ?? quickPicks().find(([k]) => k === kind)?.[1] ?? 0 });
  const out = h("pre.report.inspect", "");
  const go = async () => {
    try {
      const r = await api.query("inspect", { kind: kindSel.value, oid: Number(oidIn.value) });
      out.textContent = r.debug;
    } catch (e) {
      out.textContent = `${e.code ?? "error"}: ${e.detail ?? e.message}${e.code === "not_owner" ? " (turn debug reveal on to inspect other players' objects and plots)" : ""}`;
    }
  };
  const picks = quickPicks().map(([k, id, label]) => h("button.small", { onclick: () => ((kindSel.value = k), (oidIn.value = String(id)), go()) }, label));
  const m = openModal({
    title: "Inspector",
    tag: "inspect",
    wide: true,
    body: [h("div.row", h("label.field", h("span", "Kind"), kindSel), h("label.field", h("span", "Id or index"), oidIn)), picks.length ? h("div.row", picks) : null, out],
    buttons: [{ label: "Inspect", primary: true, onClick: go }, { label: "Close", value: null }],
  });
  if (oid !== null) go();
  oidIn.addEventListener("keydown", (ev) => ev.key === "Enter" && go());
  return m;
}

// ---------------------------------------------------------------------------------------------
// Autoplay and the spectator mode.

/** AI autoplay for N turns (`autoplay {turns}`, FfH `setAIAutoPlay`): the AI plays your slot
 *  meanwhile and gives it back at the start of the turn after the last. While it runs the top
 *  bar counts the turns left and offers Stop (`autoplayBar`). */
export function openAutoplay() {
  const n = h("input#autoplay-turns", { type: "number", min: 1, max: 10000, value: 5 });
  const err = h("p.error");
  return openModal({
    title: "AI autoplay",
    tag: "autoplay",
    body: [h("p", "The AI plays your civilization for these turns (popups and decisions included), then you have control again. The turns run without a pause (Stop in the top bar ends them); use Spectate to step through them."), h("label.field", h("span", "Turns"), n), err],
    buttons: [
      {
        label: "Run",
        primary: true,
        onClick: async (m) => {
          const turnsN = Number(n.value);
          const reply = await run({ c: "autoplay", turns: turnsN });
          if (reply) {
            toast(`The AI plays ${turnsN} turn${turnsN > 1 ? "s" : ""}.`);
            m.close("run");
          } else err.textContent = "Refused (1 to 10000 turns, on your turn).";
        },
      },
      { label: "Cancel", value: null },
    ],
  });
}

/** The spectator mode: `{playing, delay, reveal, busy}` while on (the reveal setting to restore
 *  in a human game; an observed game keeps its reveal). */
let spect = null;

/** The phases where the game does not go on (the spectator mode pauses). */
const TERMINAL = ["over", "crashed", "no_game", "stuck"];

/** The paces of the spectator mode: the pause between two turns (ms), remembered by this
 *  browser. */
export const PACES = [
  ["fast", 0, "Fast: turn after turn"],
  ["normal", 400, "Normal: a short pause between turns"],
  ["slow", 1500, "Slow: 1.5 s between turns"],
  ["crawl", 4000, "Very slow: 4 s between turns"],
];
const PACE_KEY = "ffh.spectate.pace";

function savedPace() {
  try {
    const v = localStorage.getItem(PACE_KEY);
    return PACES.find(([k]) => k === v)?.[1] ?? 400;
  } catch {
    return 400;
  }
}

export function spectating() {
  return spect !== null;
}

/** The spectator mode changed: `S.spectating` follows it (read by modules that do not import
 *  this one), and the top bar redraws (`spectate`). */
function spectateChanged() {
  S.spectating = spect !== null;
  // The phone layout gives the spectator bar its own row (iso.css).
  document.body?.classList.toggle("spectating", S.spectating);
  emit("spectate");
}

/** Whether the spectator mode is playing (not paused). */
export function spectatorPlaying() {
  return !!spect?.playing;
}

/** Whether a spectated turn is under way. */
export function spectatorBusy() {
  return !!spect?.busy || looping;
}

/** Starts the spectator mode: the AI plays one turn at a time (`autoplay {turns: 1}` at each
 *  stop) until paused or stopped, the whole map shown. In an observed game (`observe`: the AI
 *  plays every slot) the server already shows the whole map; in a human game debug reveal is
 *  switched on and the AI plays your slot too. For a new game (`newGame`) the reveal is off to
 *  start with (the status may still be the old game's). */
export async function startSpectating({ playing = true, newGame = false } = {}) {
  if (spect && !newGame) return;
  const wasOn = !newGame && !!S.status?.reveal;
  spect = { playing, delay: savedPace(), reveal: wasOn, busy: false };
  // The old game's status may still be over (or no game): wait for the new one.
  for (let i = 0; newGame && i < 100 && TERMINAL.includes(S.status?.phase); i++) await new Promise((r) => setTimeout(r, 50));
  if (!spect) return;
  if (!observing() && !wasOn) await setReveal(true);
  spectateChanged();
  if (playing) stepLoop();
}

export function pauseSpectating() {
  if (!spect) return;
  spect.playing = false;
  // An observed game playing several turns at once (Tools → AI autoplay): stop it at the end of
  // the turn under way (`autoplay {turns: 0}`, answered while the game runs).
  if (observing() && S.status?.phase === "running" && (S.status?.ai_auto_play ?? 0) > 1) api.cmd("autoplay", { turns: 0 }).catch(() => {});
  spectateChanged();
}

export function resumeSpectating() {
  if (!spect) return;
  spect.playing = true;
  spectateChanged();
  stepLoop();
}

/** One turn of the spectator mode. */
export async function stepSpectating() {
  if (!spect) await startSpectating({ playing: false });
  if (!spect || spect.busy) return;
  spect.busy = true;
  try {
    await waitHuman();
    if (S.status?.phase === "human") await run({ c: "autoplay", turns: 1 }, { quiet: true });
    await waitHuman();
  } finally {
    if (spect) spect.busy = false;
  }
}

/** Leaves the spectator mode (a human game: you have control again, debug reveal as it was). */
export async function stopSpectating() {
  if (!spect) return;
  const restore = spect.reveal;
  spect = null;
  spectateChanged();
  if (!observing() && !restore && S.status?.reveal) await setReveal(false);
}

/** Forgets the spectator mode without touching the game (another game was started). */
export function dropSpectating() {
  spect = null;
  spectateChanged();
}

export function setPace(ms) {
  if (!spect) return;
  spect.delay = ms;
  try {
    localStorage.setItem(PACE_KEY, PACES.find(([, v]) => v === ms)?.[0] ?? "normal");
  } catch {
    /* not remembered */
  }
  spectateChanged();
}

/** The playing loop (one at a time: resuming or a new game while it runs keeps it). */
let looping = false;

async function stepLoop() {
  if (looping) return;
  looping = true;
  try {
    while (spect?.playing) {
      if (TERMINAL.includes(S.status?.phase)) {
        pauseSpectating();
        break;
      }
      await stepSpectating();
      await new Promise((r) => setTimeout(r, spect?.delay ?? 0));
    }
  } finally {
    looping = false;
  }
}

/** Waits until the status says it is the human's turn (or the game cannot go on). */
async function waitHuman() {
  for (let i = 0; i < 600; i++) {
    const st = S.status;
    if (st && !["running", "decision"].includes(st.phase) && !sending()) return;
    await new Promise((r) => setTimeout(r, 50));
  }
}

/** Waits until the spectator mode has no turn under way and the game is stopped. */
async function waitStopped() {
  for (let i = 0; i < 1200 && spect?.busy; i++) await new Promise((r) => setTimeout(r, 50));
  await waitHuman();
}

/** Observed: the view follows player `p` (its empire in the top bar). */
export async function follow(p) {
  try {
    await api.session("follow", { player: p });
  } catch (e) {
    toast(e.detail ?? e.message, "error");
  }
  emit("kick");
}

// ---------------------------------------------------------------------------------------------
// Flags: a spectator's record of play that looks wrong (`session flag`, ffh-play `flag.rs`).

/** This game's flags (`session flags`), oldest first. */
export async function loadFlags() {
  try {
    S.flags = (await api.session("flags")).r?.flags ?? [];
  } catch {
    S.flags = [];
  }
  emit("flags");
}

/** Copies `text` to the clipboard (with a toast either way). */
export function copyText(text) {
  const done = navigator.clipboard?.writeText(text);
  if (!done) return toast("Select the text to copy it.", "error");
  done.then(() => toast("Copied.")).catch(() => toast("Select the text to copy it.", "error"));
}

/** The units on a plot, in one line ("3 units: Axeman (Sheaim), ..."). */
function plotUnitsLine(x, y) {
  const us = unitsAt(x, y);
  if (!us.length) return "No units.";
  const names = us.slice(0, 6).map((u) => `${dname("UnitType", u.type)} (${playerName(u.owner)})`);
  return `${us.length} unit${us.length > 1 ? "s" : ""}: ${names.join(", ")}${us.length > 6 ? ", …" : ""}`;
}

/** Flags plot (x, y): the spectator pauses first (a flag is taken while the game is stopped),
 *  a note is asked, then the record is saved (the game's `flags.jsonl`, its replay's start save
 *  pinned) and its text, ready for a bug report, copied. */
export async function openFlag(x, y) {
  if (!S.map) return;
  if (spect?.playing) pauseSpectating();
  const note = h("textarea#flag-note", { rows: 3, placeholder: "What looks wrong? (e.g. the stack walks past an undefended city)" });
  const err = h("p.error");
  const waiting = h("p.dim", spect ? "Pausing after this turn…" : "");
  const m = openModal({
    title: `Flag plot (${x}, ${y})`,
    tag: "flag",
    body: [
      h("p", "Records this plot at this turn: the units and events here, the build, the game's settings and the commands that replay the turn with the AI's trace on."),
      h("p.dim", plotUnitsLine(x, y)),
      h("label.field", h("span", "Note"), note),
      waiting,
      err,
    ],
    buttons: [
      {
        label: "Flag",
        primary: true,
        onClick: async (mm) => {
          err.textContent = "";
          await waitStopped();
          try {
            const rec = (await api.session("flag", { x, y, note: note.value.trim() })).r;
            mm.close("flagged");
            S.flags = [...(S.flags ?? []), rec];
            emit("flags");
            showFlag(rec, { fresh: true });
          } catch (e) {
            err.textContent = e.detail ?? e.message;
          }
        },
      },
      { label: "Cancel", value: null },
    ],
  });
  waitStopped().then(() => (waiting.textContent = ""));
  setTimeout(() => note.focus(), 0);
  return m;
}

/** Shows a flag's record (its text to copy, the map spot); a new one is copied at once. */
export function showFlag(rec, { fresh = false } = {}) {
  const text = rec.text ?? JSON.stringify(rec, null, 2);
  if (fresh) copyText(text);
  const [x, y] = rec.plot ?? [0, 0];
  return openModal({
    title: `Flag ${rec.flag}${rec.note ? `: ${rec.note}` : ""}`,
    tag: "flag-record",
    wide: true,
    body: [
      fresh ? h("p", "Saved in the game's ", h("code", "flags.jsonl"), " and copied to the clipboard: paste it into the bug report.") : null,
      h("pre.report", text),
    ],
    buttons: [
      { label: "Copy", onClick: () => copyText(text) },
      { label: "Copy as JSON", onClick: () => copyText(JSON.stringify(rec)) },
      { label: "Show on the map", onClick: (mm) => (mm.close("show"), emit("center", x, y)) },
      { label: "Close", value: null, primary: true },
    ],
  });
}

/** The flags tab of the bottom panel: this game's flags, newest first. */
export function flagsView() {
  const flags = S.flags ?? [];
  const intro = h(
    "p.dim",
    spectating() || observing()
      ? "Flag play that looks wrong: click the plot, then “⚑ Flag this plot” (or press F over it). Each flag records what a developer needs to replay that turn."
      : "Flags record play that looks wrong while you watch the AI (Spectate, or a new game with “Spectate” ticked).",
  );
  if (!flags.length) return h("div.flags", intro);
  return h(
    "div.flags",
    intro,
    h(
      "ul.flag-list",
      [...flags].reverse().map((f) =>
        h(
          "li",
          h("b", `⚑ ${f.flag}`),
          ` turn ${f.moved_turn ?? f.turn} · (${f.plot?.[0]}, ${f.plot?.[1]})`,
          f.note ? h("span", ` · ${f.note}`) : null,
          " ",
          h("button.link.small", { onclick: () => emit("center", f.plot?.[0] ?? 0, f.plot?.[1] ?? 0), title: "Centre the map on it" }, "⌖"),
          " ",
          h("button.link.small", { onclick: () => showFlag(f), title: "The record" }, "record"),
          " ",
          h("button.link.small", { onclick: () => copyText(f.text ?? JSON.stringify(f)), title: "Copy the record's text" }, "copy"),
        ),
      ),
    ),
  );
}

// ---------------------------------------------------------------------------------------------
// Take over, options.

/** Take over a slot of an AI-only save (a debugging aid: `session take_over`). */
export async function openTakeOver() {
  const player = h("input#takeover-player", { type: "number", min: 0, max: 63, value: 0 });
  const name = h("input#takeover-name", { type: "text", placeholder: "GAME/auto/T0010", title: "A save of the saves directory: GAME/NAME or GAME/auto/NAME" });
  const err = h("p.error");
  return openModal({
    title: "Take over a slot",
    tag: "takeover",
    body: [
      h("p", "Load an AI-only save at a turn boundary (an autosave of a game played with `ffh-play soak run --saves DIR`, or `ffh` autoplay saves copied into the saves directory) and play one of its slots."),
      h("label.field", h("span", "Save"), name),
      h("label.field", h("span", "Player"), player),
      h("button.link", { onclick: () => loadScreen() }, "Browse the saves…"),
      err,
    ],
    buttons: [
      {
        label: "Take over",
        primary: true,
        onClick: async (m) => {
          try {
            await api.session("take_over", { player: Number(player.value), name: name.value });
            m.close("taken");
            emit("game-started");
          } catch (e) {
            err.textContent = e.detail ?? e.message;
          }
        },
      },
      { label: "Cancel", value: null },
    ],
  });
}

/** The player options `set_option` may change. */
export const OPTIONS = [
  ["PLAYEROPTION_QUICK_MOVES", "Quick moves (units jump to their destination)"],
  ["PLAYEROPTION_STACK_ATTACK", "Stack attack (the whole stack attacks)"],
  ["PLAYEROPTION_NO_UNIT_CYCLING", "No unit cycling (no automatic selection of the next unit)"],
  ["PLAYEROPTION_SAFE_AUTOMATION", "Safe automation (automated workers keep improvements)"],
  ["PLAYEROPTION_WAIT_END_TURN", "Wait at the end of the turn"],
];

export function openOptions() {
  const my = me();
  const rows = OPTIONS.map(([type, label]) => {
    const i = tidx("PlayerOptionType", type);
    return h(
      "label.toggle",
      h("input", {
        type: "checkbox",
        "data-option": type,
        checked: !!my?.options?.[i],
        onchange: async (ev) => {
          await run({ c: "set_option", option: i, value: ev.target.checked });
          emit("kick");
        },
      }),
      ` ${label}`,
    );
  });
  // The map's renderer (not a game option: remembered by this browser). The flat one is the P5
  // renderer, kept for debugging; `?renderer=flat` in the address does the same for one page.
  const current = globalThis.ffh?.mapView?.renderer?.() ?? "art";
  const artFailed = look.art ? "" : `The art did not load${look.error ? `: ${look.error.message ?? look.error}` : ""}`;
  const renderer = [
    ["art", "Pixel art (2D)", artFailed || "The 2D pixel-art map"],
    ["iso", "Isometric 2.5D", "The isometric map in HD pixel art (docs/ISO.md)"],
    ["flat", "Flat (debug)", "The flat debugging renderer (colours, glyphs, labels)"],
    ...(EXPERIMENTAL_3D ? [["3d", "3D painterly (experimental)", "The experimental 3D renderer (docs/R3D.md); needs WebGL2"]] : []),
    ...(EXPERIMENTAL_HD2D ? [["hd2d", "HD-2D (experimental)", "Pixel-art sprites standing in a lit 3D world (docs/HD2D.md); needs WebGL2"]] : []),
  ].map(([val, label, title]) =>
    h(
      "label.toggle",
      { title },
      h("input", {
        type: "radio",
        id: `option-${val}-renderer`,
        name: "option-renderer",
        value: val,
        disabled: val === "art" && !look.art,
        checked: current === val,
        onchange: (ev) => {
          if (!ev.target.checked) return;
          setRendererChoice(val);
          emit("renderer", val);
        },
      }),
      ` ${label}`,
    ),
  );
  // Without a game (the title screen) there is no player to set them on.
  const game = my ? h("div.checks.single", rows) : h("p.dim", "The player options (quick moves, stack attack, unit cycling, …) are set during a game.");
  return openModal({
    title: "Options",
    tag: "options",
    body: [game, h("h4", "This browser"), h("div.checks.single", renderer)],
    buttons: [{ label: "Close", value: null }],
  });
}

/** The spectator bar's buttons (the top bar shows them while spectating). */
/** The top bar while an AI autoplay runs (not the spectator mode's own): the turns left and
 *  Stop, which sends `autoplay {turns: 0}` (FfH `setAIAutoPlay(0)`; the server stops it before
 *  its next turn slice and you have control at your next turn, or at once during yours). */
export function autoplayBar() {
  const st = S.status ?? {};
  // Observed, the spectator mode plays one turn at a time; this bar is for longer runs.
  if ((spect && !observing()) || st.phase !== "running" || !(st.ai_auto_play > (observing() ? 1 : 0))) return null;
  const left = st.ai_auto_play;
  return h(
    "span.spectate",
    h("b", "AI autoplay"),
    h("span.dim", ` ${left} turn${left === 1 ? "" : "s"} left `),
    h(
      "button.small",
      {
        id: "autoplay-stop",
        title: "Stop the AI autoplay: you have control again at your next turn",
        onclick: async (ev) => {
          ev.currentTarget.disabled = true;
          try {
            await api.cmd("autoplay", { turns: 0 });
            toast("The AI autoplay stops.");
          } catch (e) {
            toast(e.detail ?? e.message ?? String(e), "error");
          }
        },
      },
      "■ Stop",
    ),
  );
}

/** The spectator bar's state when it was last drawn (`renderSpectatorBar`). */
let barKey = null;

/** Draws the spectator bar into `slot` when its state changed (the buttons stay the same
 *  elements between status updates, so clicks on them are not lost), and the "playing…" mark. */
export function renderSpectatorBar(slot) {
  if (!slot) return;
  const me = S.view?.me;
  const civs = [...S.players.values()].map((p) => `${p.id}:${p.alive}`).join(",");
  const key = spect ? JSON.stringify([observing(), spect.playing, spect.delay, me?.player, me?.name, civs]) : "";
  if (key !== barKey || !slot.firstChild === !!spect) {
    barKey = key;
    fill(slot, spectatorBar());
  }
  const running = slot.querySelector("#spectate-running");
  if (running) running.textContent = S.status?.phase === "running" ? " playing…" : "";
}

export function spectatorBar() {
  if (!spect) return null;
  const st = S.status ?? {};
  const observed = observing();
  const pace = h(
    "select.small#spectate-pace",
    { title: "The pause between two turns", onchange: (ev) => setPace(Number(ev.target.value)) },
    PACES.map(([, ms, label]) => h("option", { value: ms, selected: ms === spect.delay, title: label }, label.split(":")[0])),
  );
  let followSel = null;
  if (observed && S.view?.me) {
    const civs = [{ id: S.view.me.player, name: S.view.me.name }, ...[...S.players.values()].filter((p) => !p.barbarian).map((p) => ({ id: p.id, name: p.name, alive: p.alive }))];
    civs.sort((a, b) => a.id - b.id);
    followSel = h(
      "select.small#spectate-follow",
      { title: "The civilization the top bar shows (the game is not changed)", onchange: (ev) => follow(Number(ev.target.value)) },
      civs.map((c) => h("option", { value: c.id, selected: c.id === S.view.me.player }, `${c.name}${c.alive === false ? " †" : ""}`)),
    );
  }
  return h(
    "span.spectate",
    h("b.sp-label", observed ? "Observing" : "Spectating"),
    spect.playing
      ? h("button.small", { id: "spectate-pause", onclick: pauseSpectating, title: "Pause after this turn" }, "⏸", h("span.sp-label", " Pause"))
      : h("button.small", { id: "spectate-play", onclick: resumeSpectating, title: "Play turn after turn" }, "▶", h("span.sp-label", " Play")),
    h("button.small", { id: "spectate-step", onclick: stepSpectating, disabled: spect.playing, title: "One turn" }, "⏭", h("span.sp-label", " Step")),
    pace,
    followSel ? h("span.dim.sp-label", " Follow ") : null,
    followSel,
    h("button.small", { id: "spectate-flag", onclick: () => flagPrompt(), title: "Flag play that looks wrong: pick the plot [F over it]" }, "⚑", h("span.sp-label", " Flag")),
    observed ? null : h("button.small", { id: "spectate-stop", onclick: stopSpectating, title: "Stop spectating (debug reveal goes back off)" }, "■", h("span.sp-label", " Stop")),
    h("span.dim#spectate-running", st.phase === "running" ? " playing…" : ""),
  );
}

/** "⚑ Flag": the plot shown in the side panel, else the next plot clicked on the map. */
export function flagPrompt() {
  const shown = document.getElementById("unit-panel")?.dataset.plot;
  if (shown) {
    const [x, y] = shown.split(",").map(Number);
    return openFlag(x, y);
  }
  if (spect?.playing) pauseSpectating();
  pickingFlag = true;
  toast("Click the plot to flag (Esc cancels).");
}

/** A flag waits for its plot (the next map click). */
let pickingFlag = false;

/** The map click that picks a flag's plot (true when it did). */
export function flagClick(x, y) {
  if (!pickingFlag) return false;
  pickingFlag = false;
  openFlag(x, y);
  return true;
}

export function cancelFlagPick() {
  const was = pickingFlag;
  pickingFlag = false;
  return was;
}
