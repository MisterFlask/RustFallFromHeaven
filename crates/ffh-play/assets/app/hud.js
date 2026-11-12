// The top bar (design §5.1 "Empire": gold, research, sliders, turn and year, the Armageddon
// counter, mana, anarchy, golden age, Stasis) with the phase and the progress line during the
// AI turns, and the bottom panel: the turn report, the messages by turn, the game's files.

import { S, dname, tname, turns, playerName, me, emit, endTurnMessage, scoreText, fairShareText, goldBreakdown, observing, isBarbarian } from "./state.js";
import { playerColor } from "./tiles.js";
import { h, fill } from "./dom.js";
import { render as markup } from "./markup.js";
import { look, pix, pixable, iconEl, uiIcon, barEl } from "./look.js";
import { openCount, run } from "./commands.js";
import { renderSpectatorBar, renderAutoplayBar, flagsView } from "./tools.js";
import { isoSkin, isoIcon } from "./isoui.js";
import { effectsOf, partsList } from "./infotips.js";
import { battleLine, battleLinks } from "./engagements.js";
import { castTargeted } from "./units.js";
import * as api from "./api.js";
import { openResearchPaths } from "./empire.js";
import { renderRituals } from "./rituals.js";

/** The parts of `#phase`: the phase line (redrawn each render), the spectator bar and the
 *  autoplay bar (each its own slot; `display: contents`, so #phase lays out their children). */
function phaseSlots(phase) {
  let main = phase.querySelector(":scope > #phase-main");
  if (!main) {
    const slot = (id) => h(`span#${id}`, { style: { display: "contents" } });
    main = slot("phase-main");
    // The AI autoplay's bar first: its Stop then stays put while the turn and the phase text
    // after it change width (`renderAutoplayBar`).
    phase.replaceChildren(slot("autoplay-slot"), main, slot("spectate-slot"));
  }
  return { main, spectate: phase.querySelector(":scope > #spectate-slot"), autoplay: phase.querySelector(":scope > #autoplay-slot") };
}

/** Year for people: -4000 -> "4000 BC". */
export function yearText(y) {
  if (y === null || y === undefined) return "";
  return y < 0 ? `${-y} BC` : `${y} AD`;
}

const PHASES = {
  human: "Your turn",
  running: "Other players are moving…",
  decision: "A decision is waiting",
  over: "Game over",
  stuck: "The turn did not finish",
  no_game: "No game",
  crashed: "Crashed",
};

/** The last phase seen, to flare the crescent once when control returns to the player. */
let lastPhase = null;
/** The game the tween state belongs to; a new or loaded game resets it so numbers do not animate
 *  from the previous game's values. */
let hudGame = null;

/** An icon of the ui set (look.js), or the plain glyph before the art loaded. */
function ic(name, glyph, title, cls = "") {
  return iconEl(name, { title }) ?? h(`span.icon${cls}`, { title }, glyph);
}

/** A number in the top bar: Vigil (docs/ART.md 11.6: big numbers in the top bar). */
function big(text, cls = "") {
  return h(`span.num${cls}`, pix(String(text), { font: "vigil" }));
}

/** Short text in the top bar: Vigil Book. */
function small(text, cls = "") {
  return h(`span${cls}`, pix(String(text)));
}

const sep = () => h("span.sep");

// ---------------------------------------------------------------------------------------------
// Tweened numbers (the top bar's gold, score and research bar count/fill smoothly when they
// change, rather than snapping). renderTop() rebuilds the whole #empire bar each tick (fill()),
// so there is no stable node to put a CSS transition on; instead a module-level driver keeps the
// currently-shown value per key, renderTop re-creates each node (marked data-tween) showing that
// value, and one rAF loop eases the values and re-finds the live node by its marker each frame.
// Same shape as map.js's camera glide: an rAF loop that runs only while animating then stops,
// ease-out cubic, and a duration in the glide's own band. prefers-reduced-motion snaps.

const tweens = new Map(); // key -> { value, shown, from, to, t0, dur, lastPaint, kind, cost, next, tech }
let tweenRaf = 0;
const TWEEN_MS = 420;

function reduceMotion() {
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

function ensureTweenLoop() {
  if (!tweenRaf) stepTweens();
}

function stepTweens() {
  tweenRaf = requestAnimationFrame(() => {
    tweenRaf = 0;
    const now = performance.now();
    let active = false;
    for (const [key, s] of tweens) {
      if (s.to === s.shown) continue;
      const t = Math.min(1, (now - s.t0) / s.dur);
      const e = 1 - Math.pow(1 - t, 3); // ease-out cubic, as map.js stepGlide
      s.shown = t >= 1 ? s.to : s.from + (s.to - s.from) * e;
      paintTween(key, s);
      if (t < 1) active = true;
    }
    if (active) stepTweens();
  });
}

/** Repaints the live node for `key` with its current eased value (the node may have just been
 *  rebuilt by renderTop; re-find it by its marker). */
function paintTween(key, s) {
  const node = document.querySelector(`#empire [data-tween="${key}"]`);
  if (!node) return;
  if (s.kind === "num") {
    const disp = (s.fmt ?? String)(Math.round(s.shown));
    if (disp === s.lastPaint) return;
    s.lastPaint = disp;
    node.replaceChildren(pix(disp, { font: "vigil" }));
  } else {
    const nb = barEl(Math.min(1, s.shown / Math.max(1, s.cost)), "research", { w: 30, next: s.next, title: `Research ${Math.round(s.shown)}/${s.cost}` });
    nb.dataset.tween = key;
    node.replaceWith(nb);
  }
}

/** A big top-bar number (Vigil) that eases from its last value to `value`. */
function tweenNum(key, value, cls = "", fmt = String) {
  const node = h(`span.num${cls}`, { "data-tween": key });
  const s = tweens.get(key);
  if (!s || reduceMotion()) {
    tweens.set(key, { value, shown: value, to: value, kind: "num", fmt });
    node.append(pix(fmt(value), { font: "vigil" }));
  } else if (value !== s.value) {
    Object.assign(s, { value, from: s.shown, to: value, t0: performance.now(), dur: TWEEN_MS, kind: "num", fmt });
    node.append(pix(fmt(Math.round(s.shown)), { font: "vigil" }));
    ensureTweenLoop();
  } else {
    node.append(pix(fmt(Math.round(s.shown)), { font: "vigil" }));
  }
  return node;
}

/** The research bar, whose fill (and tooltip) eases as progress grows; a new tech resets the
 *  progress, so it snaps rather than sliding backwards. */
function tweenBar(key, progress, cost, next, tech) {
  const s = tweens.get(key);
  const techChanged = s && s.tech !== tech;
  const make = (p) => {
    const b = barEl(Math.min(1, p / Math.max(1, cost)), "research", { w: 30, next, title: `Research ${Math.round(p)}/${cost}` });
    b.dataset.tween = key;
    return b;
  };
  if (!s || reduceMotion() || techChanged) {
    tweens.set(key, { value: progress, shown: progress, to: progress, cost, next, tech, kind: "bar" });
    return make(progress);
  }
  if (progress !== s.value) {
    Object.assign(s, { value: progress, from: s.shown, to: progress, t0: performance.now(), dur: TWEEN_MS, cost, next, tech, kind: "bar" });
    ensureTweenLoop();
  } else {
    Object.assign(s, { cost, next, tech });
  }
  return make(s.shown);
}

/** Redraws the top bar's empire summary and phase line. */
export function renderTop() {
  const st = S.status ?? {};
  // A new or loaded game: drop the tween and flare state so the numbers snap to this game's values
  // instead of counting from the last game's, and no stale crescent flare fires.
  const gameKey = st.game?.journal ?? st.game?.dir ?? "memory";
  if (gameKey !== hudGame) {
    hudGame = gameKey;
    tweens.clear();
    if (tweenRaf) {
      cancelAnimationFrame(tweenRaf);
      tweenRaf = 0;
    }
    lastPhase = null;
  }
  const v = S.view;
  const m = v?.me;
  const info = document.getElementById("empire");
  const phase = document.getElementById("phase");
  let line = PHASES[st.phase] ?? st.phase ?? "connecting…";
  // Observed, the game stops between turns (nobody's turn is waiting).
  if (observing(st) && st.phase === "human") line = "Between turns";
  if (st.phase === "running") {
    const active = (st.active_players ?? []).filter((p) => p !== st.human).map((p) => playerName(p));
    if (active.length) line = `Moving: ${active.join(", ")}`;
    if (st.ms_in_phase !== undefined) line += ` · ${(st.ms_in_phase / 1000).toFixed(1)} s`;
  }
  // The crescent is amber on your turn (the active-turn mark), else the waiting moon.
  const yours = st.phase === "human";
  if (phase) {
    fill(
      phaseSlots(phase).main,
      h("span.phase-dot", { "data-phase": st.phase ?? "" }),
      look.art && !isoSkin() ? h("span.turn-crescent", ic(yours ? "icon16/turn" : "icon16/turn_wait", "", yours && !observing(st) ? "Your turn" : line)) : null,
      st.turn !== undefined ? h("b.turn", small("Turn", ".dim"), " ", big(st.turn)) : null,
      st.year !== undefined ? h("span.dim.year", " ", small(yearText(st.year)), " ") : null,
      h("span#phase-text", pixable(line) ? pix(line) : line),
      st.reveal && !observing(st) ? h("span.badge.reveal", "DEBUG REVEAL") : null,
    );
    // The spectator bar keeps its buttons between renders (it redraws only when its state
    // changes), so a click on Pause lands while the AIs move and the status changes many times
    // a second.
    renderSpectatorBar(phaseSlots(phase).spectate);
    renderAutoplayBar(phaseSlots(phase).autoplay);
  }
  document.body.dataset.phase = st.phase ?? "";
  // A quiet one-shot flare on the crescent when control returns to the player (running -> human);
  // the breathe while the AIs move is pure CSS on the stable #phase (style.css). Added to #phase,
  // which fill() keeps (only its children are replaced), so it plays once, not on every render.
  if (phase && st.phase === "human" && lastPhase !== null && lastPhase !== "human" && !reduceMotion() && !phase.classList.contains("turn-arrived")) {
    phase.classList.add("turn-arrived");
    const done = () => phase.classList.remove("turn-arrived");
    phase.addEventListener("animationend", done, { once: true });
    setTimeout(done, 1200); // in case the animation is interrupted and animationend never fires
  }
  if (st.phase !== undefined) lastPhase = st.phase;
  const end = document.getElementById("end-turn");
  if (end) {
    end.disabled = st.phase !== "human";
    const n = observing() ? 0 : openCount();
    // "(1 open)" keeps together where the label wraps (a phone's bottom bar).
    const label = observing() ? "Next turn" : n ? `End turn (${n}\u00a0open)` : "End turn";
    const skinKey = `${label}|${!!look.art}|${isoSkin()}`;
    if (end.dataset.label !== skinKey) {
      end.dataset.label = skinKey;
      fill(end, look.art || isoSkin() ? iconEl("icon16/turn") : null, look.art ? pix(label, { font: "vigil", smallCaps: true }) : label);
    }
    // Civ4's end-turn message: no group waits, so Enter (or Space) ends the turn; else Enter
    // selects the next group.
    const msg = endTurnMessage();
    end.classList.toggle("end-turn-message", msg);
    end.title = msg ? "End the turn [Enter, Space]; Shift: without the checks" : "End the turn [Shift+Enter: without the checks]; Enter selects the next unit waiting";
  }
  // Crusades (docs/CRUSADE.md): the advisor, marked while a call to arms waits.
  const crusades = document.getElementById("btn-crusades");
  if (crusades) {
    crusades.hidden = !v;
    const calls = observing() ? 0 : (v?.pending?.crusade_calls?.length ?? 0);
    crusades.classList.toggle("attention", calls > 0);
    crusades.title = calls ? `Crusades: ${calls === 1 ? "a call to arms waits" : `${calls} calls to arms wait`} for your answer` : "Crusades: the calls to arms, the units you lent, past crusades";
  }
  // Named seas (docs/NAVAL.md 10): the Seas screen, once a sea has been seen.
  const seasBtn = document.getElementById("btn-seas");
  if (seasBtn) seasBtn.hidden = !v?.seas?.length;
  renderWorldActions(m, st);
  // World rituals (docs/CARDS.md): every civilization's ritual in progress and charge held.
  renderRituals();
  if (!m) {
    fill(info);
    return;
  }
  const r = m.research;
  const pct = r ? Math.min(1, r.progress / Math.max(1, r.cost)) : 0;
  const next = r ? Math.min(1, (r.progress + (r.rate ?? 0)) / Math.max(1, r.cost)) : null;
  // The isometric interface writes rates as the reference's top bar does: "245 (+18)".
  const iso = isoSkin();
  const rate = (n) => (iso ? `(${n >= 0 ? "+" : ""}${n})` : `${n >= 0 ? "+" : ""}${n}`);
  const signed = (n) => `${n >= 0 ? "+" : ""}${n}`;
  const adopted = m.research_path !== null && m.research_path !== undefined;
  const items = [
    h(
      "span.stat.gold",
      { tip: () => goldTip(m), role: "group", "aria-label": `Gold ${m.gold}, ${signed(m.gold_rate)} a turn` },
      ic("icon16/gold", "¤", "Gold", ".gold"),
      tweenNum("gold", m.gold),
      small(rate(m.gold_rate), m.gold_rate < 0 ? ".warn" : ".dim"),
    ),
    sep(),
    h(
      "button.stat.research",
      {
        id: "research-button",
        title: "Research: choose what to research",
        "aria-label": r ? `Research: ${dname("TechType", r.tech)}, ${r.progress}/${r.cost}, ${turns(r.turns)}` : "Research: none chosen",
        onclick: () => document.dispatchEvent(new CustomEvent("ffh-research")),
      },
      ic(r ? "icon16/research" : "icon16/research_needed", "⚗", "Research"),
      r ? small(dname("TechType", r.tech)) : h("span.warn", pix("choose research")),
      r ? tweenBar("research", r.progress, r.cost, next, r.tech) : null,
      r ? h("span.dim", { title: "Turns to finish this tech" }, small(turns(r.turns))) : null,
    ),
    // The doctrine (a research path, docs/PORTING.md "Doctrines"): the adopted one, or a reminder
    // in the warning colour while one can be adopted; a click opens the doctrine chooser.
    doctrineButton(m, adopted),
    sep(),
    h(
      "button.stat.sliders",
      { id: "sliders-button", title: "Commerce sliders: click to change them [F2]", onclick: () => emit("open-screen", "finance") },
      m.commerce_percent
        .map((p, i) => [p, i])
        .filter(([p, i]) => p > 0 || m.flexible[i])
        .map(([p, i]) => {
          const label = `${dname("CommerceType", i)} rate ${p}%`;
          return h("span.slider-item", { title: label, "aria-label": label }, look.art || iso ? uiIcon("commerce", i, 8, { title: label }) : `${dname("CommerceType", i).slice(0, 3)} `, small(`${p}%`));
        }),
    ),
    sep(),
    h(
      "span.stat.armageddon",
      { tip: () => doomTip(m), role: "group", "aria-label": `Armageddon counter ${m.armageddon}` },
      look.art || iso ? uiIcon("armageddon", m.armageddon, 16) : h("span.icon", "☄"),
      big(m.armageddon),
    ),
  ];
  const mana = m.mana.filter((x) => x.count > 0);
  if (mana.length) {
    const total = mana.reduce((n, x) => n + x.count, 0);
    const manaTip = [h("div.tip-title", `Mana ${total}`), partsList([{ rows: mana.map((x) => [dname("BonusType", x.bonus).replace(/ Mana$/, ""), x.count]), sense: 0 }], { sign: false })];
    // The isometric interface: one crystal and the total (the spheres in its tooltip).
    const spheres = iso
      ? [big(total)]
      : look.art
        ? mana.map((x) => h("span.sphere", uiIcon("mana", tname("BonusType", x.bonus) ?? "", 8, { title: dname("BonusType", x.bonus) }) ?? null, small(String(x.count))))
        : [` ${total} mana`];
    items.push(h("span.stat.mana", { tip: manaTip, role: "group", "aria-label": `Mana ${total}` }, ic("icon16/mana", "✦", "Mana"), ...spheres));
  }
  if (m.anarchy > 0) items.push(sep(), h("span.stat.warn", { title: `Anarchy: ${turns(m.anarchy)}`, "aria-label": `Anarchy: ${turns(m.anarchy)}`, role: "group" }, ic("icon16/anarchy", "", "Anarchy"), small(`Anarchy ${m.anarchy}`)));
  if (m.golden_age > 0) items.push(sep(), h("span.stat.good", { title: `Golden age: ${turns(m.golden_age)}`, "aria-label": `Golden age: ${turns(m.golden_age)}`, role: "group" }, ic("icon16/golden_age", "", "Golden age"), small(`Golden age ${m.golden_age}`)));
  const stasis = Object.entries(m.stasis ?? {}).filter(([, n]) => n > 0);
  if (stasis.length) items.push(sep(), h("span.stat.warn", ic("icon16/stasis", "", "Stasis"), small(`Stasis: ${stasis.map(([k, n]) => `${k} ${n}`).join(", ")}`)));
  const idle = v.pending?.ai_groups_idle?.length ?? 0;
  if (idle) items.push(sep(), h("span.stat.dim", { title: "Automated and FfH AI-controlled units: the AI moves them during your turn" }, ic("icon16/ai_idle", "", "Automated groups to move"), small(`${idle} automated group${idle > 1 ? "s" : ""} to move`)));
  const scoreTip = [h("div.tip-title", `Score ${scoreText(m.score)}`), h("div", "Your share of the world: the average of your shares of all people, all land and all knowledge."), m.civs_alive ? h("div.tip-hint", `With ${m.civs_alive} civilizations alive, a fair share is ${fairShareText(m.civs_alive)}.`) : null];
  items.push(sep(), h("span.stat.dim.score", { tip: scoreTip, role: "group", "aria-label": `Score: ${scoreText(m.score)} of the world` }, iso ? iconEl("icon16/score") ?? small("Score") : small("Score"), tweenNum("score", m.score, "", scoreText)));
  fill(info, items);
}

/** Whether a doctrine can be adopted (some research path's `can_commit`, the `techs` query), asked
 *  once a turn while none is adopted; true until the answer comes. */
const doctrineState = { key: null, open: true, percent: null };

function doctrineOpen() {
  const key = `${hudGame}|${S.status?.turn ?? ""}`;
  if (doctrineState.key !== key && !observing()) {
    doctrineState.key = key;
    api
      .query("techs")
      .then((t) => {
        const open = (t.paths ?? []).some((p) => p.can_commit);
        const percent = t.path_percent ?? null;
        if (open !== doctrineState.open || percent !== doctrineState.percent) {
          Object.assign(doctrineState, { open, percent });
          renderTop();
        }
      })
      .catch(() => {});
  }
  return doctrineState.open;
}

/** The top bar's doctrine: the adopted one's name, or "no doctrine" (the warning colour while one
 *  can be adopted); its tooltip says what a doctrine does; a click opens the doctrine chooser. */
function doctrineButton(m, adopted) {
  const faster = doctrineState.percent ? `${doctrineState.percent}% faster` : "faster";
  const what = `A doctrine makes its techs research ${faster} for the rest of the game; it is adopted in the tech tree (Doctrines).`;
  const open = adopted ? false : doctrineOpen();
  const name = adopted ? dname("ResearchPathType", m.research_path) : null;
  return h(
    "button.stat.doctrine",
    {
      id: "doctrine-button",
      title: adopted ? `Doctrine: ${name}. ${what}` : what,
      "aria-label": adopted ? `Doctrine: ${name}` : "No doctrine",
      onclick: () => openResearchPaths(),
    },
    adopted ? small(name, ".path-tag") : small("no doctrine", open && !observing() ? ".warn" : ".dim"),
  );
}

/** What a spell's target is, in words (`target_kind`). */
const TARGET_WORDS = { plot: "a plot", city: "a city", unit: "a unit", player: "a player", choice: "one of its options" };

/** The empire's own actions, a row of buttons beside its numbers (`me.world_actions`): its world
 *  spell, cast with a charge of its ritual (the tooltip says where the ritual stands: `why` while
 *  no charge is held or no unit can cast it, `charge` while it can be cast), and its cards' world spells (docs/CARDS.md: once a game each, still
 *  shown once cast), each through the unit the server picked (`caster`), after the cast's
 *  confirmation. A spell cast at a distance first picks its target (`castTargeted`: on the map, or
 *  from a list). */
function renderWorldActions(m, st) {
  const box = document.getElementById("world-actions");
  if (!box) return;
  const list = m && !observing(st) ? (m.world_actions ?? []) : [];
  const yours = st.phase === "human";
  fill(
    box,
    list.map((a) => {
      const name = dname("SpellType", a.spell);
      const caster = a.caster !== null && a.caster !== undefined ? S.units?.get?.(a.caster) : null;
      const help = effectsOf("SpellType", a.spell) || a.help || "";
      const what = a.once ? "Card spell" : "World spell";
      const pick = a.target_kind ? `; then pick its target (${TARGET_WORDS[a.target_kind] ?? a.target_kind})` : "";
      const tip = [
        h("div.tip-title", `${what}: ${name}`),
        a.once ? h("div", "Card spell: cast once a game.") : null,
        help ? h("div.tip-effects", markup(help)) : null,
        a.enabled && a.charge ? h("div.tip-hint.ritual-charge-line", a.charge) : null,
        a.enabled ? (caster ? h("div.tip-hint", `Cast by your ${dname("UnitType", caster.type)} at (${caster.x}, ${caster.y})${pick}.`) : null) : h("div.tip-hint.world-action-why", a.why ?? "No unit of yours can cast it now."),
      ];
      return h(
        `button.world-action${a.cast ? ".spent" : ""}`,
        {
          disabled: !a.enabled || !yours,
          "data-spell": a.spell,
          "aria-label": `Cast the ${a.once ? "card" : "world"} spell ${name}`,
          tip,
          onclick: () => (a.target_kind ? castTargeted(a, { world: true }) : run(a.cmd)),
        },
        isoSkin() ? isoIcon("spell", { px: 32 }) : h("span.icon", a.once ? "✦" : "✶"),
        small(name, ".label"),
      );
    }),
  );
}

/** The Armageddon counter's tooltip: the counter, and the Armageddon deck's slots as the player
 *  knows them (`me.doom`, docs/CARDS.md): a slot's card once revealed (or seen ahead), with its
 *  effect; fired slots marked; the others hidden. */
export function doomTip(m) {
  const d = m.doom;
  const head = h("div.tip-title", `Armageddon counter ${m.armageddon}`);
  if (!d?.slots?.length) return [head, h("div.dim", "No Armageddon card is dealt yet: the deck is dealt at the end of the first turn.")];
  const STATES = { fired: "fired", revealed: "revealed", seen: "seen ahead", hidden: "hidden" };
  const rows = d.slots.map((s) => {
    const known = s.state !== "hidden";
    const name = !known ? "Hidden" : s.card ? `${s.name}${s.stages > 1 ? ` (stage ${s.stage + 1} of ${s.stages})` : ""}` : "No card";
    return h(
      `div.doom-slot.${s.state}`,
      h("span.doom-at", String(s.threshold)),
      h("span.doom-card", name),
      known ? h("span.dim", ` · ${STATES[s.state] ?? s.state}`) : null,
      known && s.card && s.help ? h("div.doom-help", markup(s.help)) : null,
    );
  });
  return [head, h("div.doom-slots", rows), h("div.tip-hint", "A card is revealed to every player when the counter is within 5 of its slot, and plays when the counter reaches it.")];
}

/** The top bar's gold tooltip: the treasury and where the gold rate comes from. */
export function goldTip(m) {
  const b = goldBreakdown(m.gold_parts);
  const sum = (l) => l.reduce((n, [, g]) => n + g, 0);
  return [
    h("div.tip-title", `Gold ${m.gold} (${m.gold_rate >= 0 ? "+" : ""}${m.gold_rate} a turn)`),
    b
      ? partsList([
          { head: "Income", total: `+${sum(b.income)}`, rows: b.income },
          { head: "Costs", total: `−${sum(b.costs)}`, rows: b.costs.map(([what, n]) => [what, -n]) },
        ])
      : null,
    b && m.gold_parts.research_fixed ? h("div.tip-hint", "Research cannot be adjusted: the gold rate is only what the treasury loses.") : null,
    h("div.tip-hint", "The commerce sliders: click the shares beside it, or F2."),
  ];
}

// ---------------------------------------------------------------------------------------------
// The bottom panel.

let tab = "report";

export function setTab(t) {
  tab = t;
  for (const b of document.querySelectorAll("#bottom-tabs button")) b.classList.toggle("active", b.dataset.tab === t);
  renderBottom();
}

export function currentTab() {
  return tab;
}

/** Redraws the bottom panel's open tab. */
export function renderBottom() {
  const box = document.getElementById("bottom-body");
  if (!box) return;
  if (tab === "report") fill(box, reportView());
  else if (tab === "messages") fill(box, messagesView());
  else if (tab === "flags") fill(box, flagsView());
  else fill(box, gameView());
}

function reportView() {
  if (observing()) return h("p.dim", "Observing an AI-only game: every civilization's messages are under Messages (newest turn first); the map marks where this turn's fights and captures happened.");
  const r = S.report;
  if (!r) return h("p.dim", "The turn report appears at the start of each turn.");
  // Engagement combat: the battles you fought since your last turn began, each with its links
  // (their text lines, "battle #12 at ...", are the list's).
  const battles = r.engagements ?? [];
  const lines = (r.lines ?? []).filter((l) => !(battles.length && /^battle #\d+ at /.test(l)));
  const msgs = r.messages ?? [];
  return h(
    "div.report",
    h("h4", r.from_turn === r.turn ? `Turn ${r.turn}` : `Turns ${r.from_turn}-${r.turn}`),
    lines.length || battles.length ? (lines.length ? h("ul", lines.map((l) => h("li", l))) : null) : h("p.dim", "Nothing to report."),
    battles.length
      ? h(
          "div.section.report-battles",
          h("h4", `Battles (${battles.length})`),
          h("ul.battles", battles.map((b) => h("li", { "data-battle": b.id }, h("span", `#${b.id} ${battleLine(b)}`), battleLinks(b.id, { hasLog: b.has_log })))),
        )
      : null,
    msgs.length ? h("div.section", h("h4", "Messages"), h("ul.messages", msgs.map((m) => h("li", markup(m.text), m.kind === "engagement" ? battleLinks(m.report) : null)))) : null,
  );
}

/** Messages from players other than the one followed are shown while observing: "only the
 *  followed civilization" narrows them (remembered while the page is open). */
let followedOnly = false;

function messageLine(m) {
  const watching = observing() || S.spectating;
  const who = watching && m.player !== null && m.player !== undefined ? h("b.msg-who", { style: { color: playerColor(m.player, isBarbarian(m.player)) } }, `${playerName(m.player)}: `) : null;
  const at = Array.isArray(m.at) ? m.at : null;
  return h(
    "li",
    who,
    markup(m.text),
    // A battle's message (engagement combat): its watch and report screens.
    m.kind === "engagement" && m.report !== undefined && !watching ? battleLinks(m.report) : null,
    at ? " " : null,
    at ? h("button.link.small", { title: `Show (${at[0]}, ${at[1]}) on the map`, onclick: () => (emit("center", at[0], at[1]), emit("flash", at[0], at[1])) }, "⌖") : null,
    at && watching ? h("button.link.small", { title: "Flag this plot for a bug report", onclick: () => emit("flag-plot", at[0], at[1]) }, "⚑") : null,
  );
}

function messagesView() {
  if (!S.messages.length) return h("p.dim", "No messages yet.");
  const observed = observing();
  const mine = me()?.player;
  const shown = observed && followedOnly ? S.messages.filter((m) => m.player === null || m.player === undefined || m.player === mine) : S.messages;
  const filter = observed
    ? h(
        "label.check.small",
        h("input", { type: "checkbox", checked: followedOnly, onchange: (ev) => ((followedOnly = ev.target.checked), renderBottom()) }),
        " only the followed civilization (and world news)",
      )
    : null;
  const byTurn = new Map();
  for (const m of shown) {
    if (!byTurn.has(m.t)) byTurn.set(m.t, []);
    byTurn.get(m.t).push(m);
  }
  const turnsDesc = [...byTurn.keys()].sort((a, b) => b - a);
  return h(
    "div.messages",
    filter,
    turnsDesc.map((t) => h("div.turn-group", h("h4", `Turn ${t}`), h("ul", byTurn.get(t).map(messageLine)))),
  );
}

function gameView() {
  const g = S.status?.game;
  const rows = [
    ["Game directory", g?.dir],
    ["Journal", g?.journal],
    ["Last autosave", g?.autosave],
    ["Mode", observing() ? "observed: an AI-only game (the AI plays every civilization)" : null],
    ["Build", S.status?.build],
    ["Map", S.map ? `${S.map.w} x ${S.map.h}${S.map.wrap_x ? ", wraps east-west" : ""}` : null],
    ["You", me() ? `${playerName(me().player)} (player ${me().player}, ${dname("CivilizationType", me().civ)})` : null],
    ["Leader", me() ? tname("LeaderHeadType", me().leader) : null],
  ].filter(([, v]) => v);
  if (!rows.length) return h("p.dim", "No game.");
  return h("table.kv", rows.map(([k, v]) => h("tr", h("th", k), h("td", h("code", String(v))))));
}
