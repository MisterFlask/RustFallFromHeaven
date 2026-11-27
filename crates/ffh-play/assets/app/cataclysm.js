// Cataclysm: the live HUD (top bar chips, the squad panel and its 3x3 formation, the city box),
// the map's input (select, move, attack, the war question, touch taps), the map tooltip and the
// stack plates' text. The modal screens are cataclysm_screens.js. The rule numbers come from the
// view (`S.view.cataclysm`, docs/CATACLYSM.md); every field added for the UX batch is read
// defensively, so an older server's view still draws.

import * as api from "./api.js";
import { S } from "./state.js";
import { h, fill } from "./dom.js";
import { plotIndex } from "./tiles.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { isoSkin } from "./isoui.js";
import { openReport } from "./engagements.js";
import { ask } from "./modals.js";
import { render as markup } from "./markup.js";
import { remindersNode } from "./keywords.js";
import { catTip, extractCatReminders, reinforceTip, reinforceWhy, upgradeTip, memberTip } from "./cataclysm_tips.js";
// A namespace import: a screen the screens module does not export (yet) is undefined here, not a
// link error that stops the page.
import * as screens from "./cataclysm_screens.js";

let selectedSquadId = null;
let selectedMemberId = null;
let mapViewRef = null;
/** The plot a first tap armed (touch): a second tap there moves or attacks. */
let armed = null;

const catCmd = (cmd) => screens.catCmd(cmd);

/** The screens, through the names the contract gives them (with the older names as a fallback). */
const open = {
  cities: () => screens.openCataclysmCities?.(),
  research: (tab = "research") => (screens.openResearch ? screens.openResearch({ tab }) : screens.openDoctrineTree?.()),
  court: () => screens.openCourtScreen?.(),
  houses: () => (screens.openHouses ?? screens.openHousesAndApocalypse)?.(),
  worldseal: () => (screens.openWorldseal ?? screens.openHousesAndApocalypse)?.(),
  diplomacy: () => (screens.openDiplomacy ?? screens.openHousesAndApocalypse)?.(),
  city: (id) => screens.openMacroCity?.(id),
};

export function isCataclysm() {
  return S.view?.mode === "cataclysm";
}

export function initCataclysm(mapView) {
  mapViewRef = mapView;
  // The stack plates' text (map.js, r3d/hud.js read it from the MapView): null outside Cataclysm,
  // so the classic plate stays.
  if (mapView) mapView.stackLabel = (x, y) => (isCataclysm() ? stackLabel(x, y) : null);
}

export function clearSelection() {
  selectedSquadId = null;
  selectedMemberId = null;
  armed = null;
}

function cat() {
  return S.view?.cataclysm ?? null;
}

function myId() {
  return S.view?.me?.player ?? 0;
}

function playerOf(id) {
  return (cat()?.players ?? []).find((p) => p.id === id) ?? null;
}

function ownerName(id) {
  const p = playerOf(id);
  return p?.name ?? p?.faction_name ?? `Player ${id}`;
}

function squadsOf(data) {
  return data?.squads ?? [];
}

function citiesOf(data) {
  return data?.cities ?? [];
}

function selectedSquad() {
  const data = cat();
  if (!data) return null;
  const mine = squadsOf(data).filter((s) => s.owner === myId());
  if (selectedSquadId !== null) {
    const found = mine.find((s) => s.id === selectedSquadId);
    if (found) return found;
  }
  return mine.find((s) => s.moves_left > 0) ?? mine[0] ?? null;
}

export function selectNextSquad() {
  const data = cat();
  if (!data) return;
  const mine = squadsOf(data).filter((s) => s.owner === myId() && s.moves_left > 0);
  if (!mine.length) return;
  const idx = mine.findIndex((s) => s.id === selectedSquadId);
  const next = mine[(idx + 1) % mine.length];
  selectedSquadId = next.id;
  selectedMemberId = null;
  syncSelectionToMap(next);
  mapViewRef?.centerOn(next.x, next.y);
  renderCataclysmPanel();
}

function syncSelectionToMap(sq) {
  if (!sq) {
    S.sel = null;
    return;
  }
  S.sel = { units: (sq.members ?? []).map((m) => m.id), x: sq.x, y: sq.y };
  mapViewRef?.request();
}

export async function endCataclysmTurn() {
  const reply = await catCmd({ c: "end_turn" });
  if (reply?.r?.report) {
    openReport(reply.r.report);
  }
}

/** The menu's and the F keys' screens (main.js `openScreen`) in a Cataclysm game. */
export function openCataclysmScreen(name) {
  switch (name) {
    case "cities":
      return open.cities();
    case "research":
    case "techs":
      return open.research("research");
    case "civics":
      return open.research("rituals");
    case "religion":
      return open.research("creeds");
    case "governors":
    case "armies":
      return open.court();
    case "houses":
      return open.houses();
    case "diplomacy":
    case "crusades":
      return open.diplomacy();
    case "finance":
    case "seas":
    case "trade":
    default:
      return open.worldseal();
  }
}

// ---------------------------------------------------------------------------------------------
// Map input

/** A click (or a tap: `ev.pointerType === "touch"`) on plot (x, y). An own squad there is
 *  selected (the next one on a second click), an own city opens. With a finger, a tap on another
 *  plot while the selected squad can move arms it (its tooltip is up: map.js shows it on the tap),
 *  and a second tap there does what a right-click does; an own city is such a plot too. */
export function handleClick(x, y, ev) {
  const data = cat();
  if (!data) return false;
  const sqs = squadsOf(data).filter((s) => s.x === x && s.y === y && s.owner === myId());
  if (sqs.length) {
    armed = null;
    const curIdx = sqs.findIndex((s) => s.id === selectedSquadId);
    const pick = sqs[(curIdx + 1) % sqs.length];
    selectedSquadId = pick.id;
    selectedMemberId = null;
    syncSelectionToMap(pick);
    renderCataclysmPanel();
    return true;
  }
  // A finger, with a squad that can still move: the two-tap move comes before opening a city, as
  // in the classic game (units.js `click`), so that a squad can enter its own city by touch. The
  // city opens from its panel (or by this tap when no squad can move).
  if (ev?.pointerType === "touch") {
    const sq = selectedSquad();
    if (sq && sq.moves_left > 0 && S.status?.phase === "human" && !(sq.x === x && sq.y === y)) {
      if (armed && armed.x === x && armed.y === y) {
        armed = null;
        mapViewRef?.hideTip();
        handleRightClick(x, y);
        return true;
      }
      armed = { x, y };
      showCatTip();
      return true;
    }
  }
  const city = citiesOf(data).find((c) => c.x === x && c.y === y);
  if (city && city.owner === myId()) {
    armed = null;
    open.city(city.id);
    return true;
  }
  return false;
}

export function handleDblClick(x, y) {
  const data = cat();
  if (!data) return false;
  const city = citiesOf(data).find((c) => c.x === x && c.y === y);
  if (city) {
    open.city(city.id);
    return true;
  }
  return false;
}

/** The players whose squad or city is on plot (x, y), other than the human. */
function foreignOwnersAt(data, x, y) {
  const owners = new Set();
  for (const s of squadsOf(data)) if (s.x === x && s.y === y && s.owner !== myId()) owners.add(s.owner);
  for (const c of citiesOf(data)) if (c.x === x && c.y === y && c.owner !== myId()) owners.add(c.owner);
  return [...owners];
}

/** Asks before an attack that declares war (on a player at peace with the human; the Wild Clans
 *  and players already at war are not asked about). Resolves whether to go on. */
async function confirmWar(data, owners) {
  for (const o of owners) {
    const p = playerOf(o);
    if (!p || p.is_neutral || p.war !== false) continue;
    // During a truce the attack is refused and says why.
    if (p.can_declare_war === false) continue;
    const who = p.faction_name ?? p.name ?? `Player ${o}`;
    const text = [`Attacking here declares war on ${who}.`];
    if (data.war_effect) text.push(data.war_effect);
    const yes = await ask("Declare war", text, [
      { label: "Declare war", value: true, danger: true },
      { label: "Cancel", value: null },
    ]);
    if (yes !== true) return false;
  }
  return true;
}

export async function handleRightClick(x, y) {
  const sq = selectedSquad();
  if (!sq) return false;
  if (sq.x === x && sq.y === y) return true;
  const sqId = sq.id;
  let curX = sq.x;
  let curY = sq.y;
  const maxSteps = Math.max(1, Math.min(sq.moves_left, 8));
  for (let step = 0; step < maxSteps; step++) {
    if (curX === x && curY === y) break;
    const data = cat();
    if (!data) break;
    const w = S.map?.w ?? 0;
    let dx = x - curX;
    if (S.map?.wrap_x && w > 0) {
      if (dx > w / 2) dx -= w;
      if (dx < -w / 2) dx += w;
    }
    const dy = y - curY;
    const nx = S.map?.wrap_x && w > 0 ? (((curX + Math.sign(dx)) % w) + w) % w : curX + Math.sign(dx);
    const ny = curY + Math.sign(dy);
    const owners = foreignOwnersAt(data, nx, ny);
    const enemyHere = owners.length > 0;
    if (enemyHere && !(await confirmWar(data, owners))) break;

    const reply = await catCmd({ c: enemyHere ? "cat_attack" : "cat_move", squad: sqId, x: nx, y: ny });
    if (!reply) break;
    if (reply.r?.report) {
      openReport(reply.r.report);
      break;
    }
    curX = nx;
    curY = ny;
    if (curX === x && curY === y) break;
  }
  return true;
}

// ---------------------------------------------------------------------------------------------
// The map tooltip (main.js `hover`): bare lines from the `plot` query and the view

let hover = null;
let hoverTimer = 0;
let plotInfo = null;
let forecast = null;

/** The pointer is over plot (x, y) (null: it left the map, or the finger panned). */
export function hoverAt(x, y) {
  clearTimeout(hoverTimer);
  if (x === null || x === undefined) {
    hover = null;
    armed = null;
    plotInfo = null;
    forecast = null;
    mapViewRef?.hideTip();
    return;
  }
  if (armed && (armed.x !== x || armed.y !== y)) armed = null;
  const same = hover && hover.x === x && hover.y === y;
  hover = { x, y };
  if (!same) {
    plotInfo = null;
    forecast = null;
  }
  showCatTip();
  hoverTimer = setTimeout(() => loadHover(x, y), 90);
}

async function loadHover(x, y) {
  const still = () => hover && hover.x === x && hover.y === y;
  const jobs = [
    api
      .query("plot", { x, y })
      .then((r) => still() && (plotInfo = r ?? null))
      .catch(() => {}),
  ];
  const data = cat();
  const sq = selectedSquad();
  if (data && sq && S.status?.phase === "human" && !(sq.x === x && sq.y === y) && foreignOwnersAt(data, x, y).length) {
    jobs.push(
      api
        .query("cataclysm_preview", { squad: sq.id, x, y })
        .then((r) => still() && (forecast = r ?? null))
        .catch(() => {}),
    );
  }
  await Promise.all(jobs);
  if (still()) showCatTip();
}

function plotIdx(x, y) {
  return S.map ? plotIndex(S.map, x, y) : -1;
}

function pctList(list) {
  return (list ?? []).map((p) => {
    const n = typeof p === "number" ? p : (p?.hp ?? 0);
    return n > 0 ? `${n}%` : "breached";
  });
}

function round1(n) {
  return typeof n === "number" && !Number.isInteger(n) ? n.toFixed(1) : String(n ?? 0);
}

function showCatTip() {
  if (!hover || !mapViewRef) return;
  const { x, y } = hover;
  const data = cat();
  const dict = data?.keywords ?? null;
  const p = plotInfo && plotInfo.x === x && plotInfo.y === y ? plotInfo : null;
  const i = plotIdx(x, y);
  const unexplored = p ? p.revealed === false : i >= 0 && S.map?.vis?.[i] === 0;
  const lines = [];
  const rawTexts = [];
  const line = (text, cls = "") => lines.push(h(`div.cat-tip${cls}`, text));
  const mline = (text, cls = "") => {
    rawTexts.push(text);
    lines.push(h(`div.cat-tip${cls}`, markup(text)));
  };
  if (unexplored) {
    lines.push(h("div.tip-title.cat-tip", `Unexplored (${x}, ${y})`));
    return mapViewRef.showTip(lines);
  }
  if (p) {
    const feats = (p.features ?? []).filter(Boolean);
    lines.push(h("div.tip-title.cat-tip", [p.terrain_name, ...feats].filter(Boolean).join(", ") || "Plot", h("span.dim", ` (${x}, ${y})`)));
    const impTag = p.improvement === "Fort" ? "[KW=CAT_FORT]Fort[\\KW]" : p.improvement;
    const extra = [impTag, p.road ? "[KW=CAT_ROAD]Road[\\KW]" : null, p.resource].filter(Boolean);
    if (extra.length) mline(extra.join(" · "));
    if (p.owner_name) line(`Owner: ${p.owner_name}`, ".dim");
    const yl = p.yields;
    if (yl) line(`Food ${yl.food ?? 0} · Industry ${yl.industry ?? 0} · Gold ${yl.gold ?? 0}`);
    if (p.move_cost === null) line("Impassable", ".warn");
    else if (p.move_cost !== undefined) line(`Move cost ${p.move_cost}`);
    if (p.tip_tags?.length) {
      for (const tag of p.tip_tags) mline(tag, ".caution");
    } else {
      if (p.lair) mline("Unexplored [KW=CAT_IMPERIAL_RUIN]Imperial Ruin[\\KW]", ".caution");
      for (const n of p.notes ?? []) line(n, ".caution");
    }
  } else {
    lines.push(h("div.tip-title.cat-tip", `(${x}, ${y})`));
  }
  if (data) {
    const city = citiesOf(data).find((c) => c.x === x && c.y === y);
    if (city) {
      const walls = pctList(city.sentinels);
      const bits = [`size ${city.size}`];
      if (city.defense_pct !== undefined) bits.push(`defense +${city.defense_pct}%`);
      if (walls.length) bits.push(`[KW=CAT_SENTINELS]Sentinels[\\KW] ${walls.join(" · ")}`);
      mline(`${city.capital ? "★ " : ""}${city.name} (${ownerName(city.owner)}): ${bits.join(", ")}`, ".tip-city");
      if (city.besieged) mline("[KW=CAT_BESIEGED]Besieged[\\KW]", ".warn");
      if (city.lifeboat) mline(`[KW=CAT_LIFEBOAT]Lifeboat[\\KW]: ${city.lifeboat}`);
    }
    for (const s of squadsOf(data).filter((q) => q.x === x && q.y === y)) {
      const mine = s.owner === myId();
      const bits = [];
      if (s.power !== undefined) bits.push(`power ${s.power}`);
      const n = s.members?.length;
      if (n) bits.push(`${n} unit${n === 1 ? "" : "s"}`);
      if (mine) bits.push(`moves ${s.moves_left}/${s.max_moves}`);
      else {
        const pl = playerOf(s.owner);
        if (pl && !pl.is_neutral) bits.push(pl.war ? "at war" : "at peace");
      }
      line(`${s.name} (${ownerName(s.owner)})${bits.length ? `: ${bits.join(", ")}` : ""}`, mine ? "" : ".tip-foreign");
    }
    for (const cv of (data.caravans ?? []).filter((c) => c.x === x && c.y === y)) {
      mline(`[KW=CAT_CARAVAN]Caravan[\\KW] ${cv.origin_name} → ${cv.dest_name}, +${cv.gold_rate} Gold/t`);
    }
  }
  const f = forecast;
  if (f && f.runs !== undefined) {
    line(`Attack forecast (${f.runs} runs): plot taken ${f.plot_falls_pct}% · your squad wiped ${f.attackers_wiped_pct}%`, ".cat-tip-forecast");
    line(`Units lost on average: yours ${round1(f.expected_att_lost)}, theirs ${round1(f.expected_def_lost)}`, ".cat-tip-forecast");
  }
  if (armed && armed.x === x && armed.y === y) line("Tap again: move or attack.", ".dim");
  const rem = remindersNode(extractCatReminders(rawTexts, [], dict));
  if (rem) lines.push(rem);
  mapViewRef.showTip(lines);
}

// ---------------------------------------------------------------------------------------------
// Stack plates (map.js, r3d/hud.js): a plot's text

let labelCache = { data: null, map: new Map() };

/** The plate text of plot (x, y) in a Cataclysm game: the summed power of the squads there, `Ca`
 *  for a caravan alone; null when the view knows nothing there (the plate keeps its own text).
 *  Built once per view. */
export function stackLabel(x, y) {
  const data = cat();
  if (!data) return null;
  if (labelCache.data !== data) {
    const map = new Map();
    const power = new Map();
    for (const s of squadsOf(data)) {
      const k = `${s.x},${s.y}`;
      power.set(k, (power.get(k) ?? 0) + (s.power ?? 0));
    }
    for (const [k, n] of power) map.set(k, String(n));
    for (const cv of data.caravans ?? []) {
      if (cv.x === null || cv.x === undefined) continue;
      const k = `${cv.x},${cv.y}`;
      if (!map.has(k)) map.set(k, "Ca");
    }
    labelCache = { data, map };
  }
  return labelCache.map.get(`${x},${y}`) ?? null;
}

// ---------------------------------------------------------------------------------------------
// The top bar

const fmtRate = (n) => `${(n ?? 0) >= 0 ? "+" : ""}${n ?? 0}/t`;

function calcTip(title, lines, more = [], dict = null, extraTexts = []) {
  const rem = remindersNode(extractCatReminders(extraTexts, [], dict));
  return [
    h("div.tip-title", title),
    ...(lines?.length ? lines.map((l) => h("div.tip-calc-line", l)) : [h("div.dim", "No modifiers.")]),
    ...more.filter(Boolean),
    rem,
  ].filter(Boolean);
}

function chip(cls, tip, onclick, ...label) {
  return h(`${onclick ? "button" : "span"}.cat-top-chip${cls}`, { tip, onclick }, h("span.cat-chip-text", ...label));
}

export function renderCataclysmTop() {
  const data = cat();
  const info = document.getElementById("empire");
  if (!data || !info) return;
  const dict = data.keywords ?? null;
  const over = S.status?.phase === "over";
  const accent = over ? "" : ".accent";
  const cur = data.currencies ?? {};
  const res = data.research?.current ?? null;
  const idleLore = data.research?.idle_lore_tip ?? data.research?.idle_lore_text ?? null;
  const adoptTip = data.research?.adopt_tip ?? data.research?.adopt_text ?? null;
  const ap = data.apocalypse ?? {};
  const court = data.court ?? [];
  const renown = data.renown ?? {};
  const houses = data.houses ?? [];
  const turn = data.turn ?? S.status?.turn ?? 0;
  const maxTurns = data.max_turns;
  const left = data.turns_left ?? (maxTurns !== undefined ? Math.max(0, maxTurns - turn) : null);
  const act = ap.act_label ?? data.act_label ?? "";
  const faction = data.human_faction ?? "";

  const actLines = [
    data.human_lore_blurb ? { cls: ".dim", text: data.human_lore_blurb } : null,
    data.human_rule_tip ?? data.human_rule_summary ?? null,
    ...(data.victory?.tip_lines ?? data.victory?.lines ?? []),
    data.victory?.status ? { cls: ".caution", text: data.victory.status } : null,
  ].filter(Boolean);
  const actTip = catTip({
    title: `${faction} · ${act}`,
    lines: actLines,
    hint: maxTurns !== undefined ? `Turn ${turn} of ${maxTurns}.${left !== null ? ` ${left} turns left.` : ""}` : null,
    dict,
  });

  const extraResText = !res ? idleLore ?? adoptTip : null;
  const researchTip = calcTip(
    res ? `Research: ${res.name}` : "Research: none",
    cur.lore_calc,
    [
      res ? h("div.tip-hint", `${res.progress}/${res.cost} Lore`) : h("div.tip-hint", `Lore ${cur.lore ?? 0} (${fmtRate(cur.lore_rate)})`),
      extraResText ? h("div", markup(extraResText)) : null,
    ].filter(Boolean),
    dict,
    extraResText ? [extraResText] : [],
  );

  const counting = ap.countdown !== null && ap.countdown !== undefined;
  const strainCls = counting ? ".warn.strong" : (ap.counter ?? 0) >= 50 ? ".warn" : "";
  const recent = (ap.recent_changes ?? []).slice(0, 5);
  const strainTip = catTip({
    title: counting ? `Blowout in ${ap.countdown} turns` : `Worldstrain ${ap.counter ?? 0}/100`,
    right: act || null,
    lines: [
      ...(ap.tip_lines ?? ap.rules ?? []),
      data.victory?.status ? { cls: ".caution", text: data.victory.status } : null,
    ].filter(Boolean),
    extra: recent.length ? [h("div.tip-hint", "Recent changes"), ...recent.map((l) => h("div.tip-calc-line", l))] : [],
    keywords: counting ? ["CAT_BLOWOUT", "CAT_LIFEBOAT", "CAT_WORLDSTRAIN"] : ["CAT_WORLDSTRAIN"],
    dict,
  });

  const courtTip = catTip({
    title: court.length ? `Court: ${court.length} waiting` : "Court",
    lines: [renown.tip ?? `[KW=CAT_RENOWN]Renown[\\KW] ${renown.total ?? 0}/${renown.next_threshold ?? 0}.`],
    keywords: ["CAT_RENOWN", "CAT_GREAT_PERSON"],
    dict,
  });

  const inactive = houses.filter((x) => x.perk_active === false);
  const housesTip = catTip({
    title: `Houses ${houses.length}`,
    lines: houses.length
      ? houses.map((x) => ({
          cls: x.perk_active === false ? ".warn" : "",
          text: `${x.name}: ${x.mood}, [KW=CAT_LOYALTY]Loyalty[\\KW] ${x.loyalty} (${x.output_pct}%). ${x.perk}: ${x.perk_active === false ? "inactive" : "active"}.`,
        }))
      : [{ cls: ".dim", text: "No [KW=CAT_HOUSE]Houses[\\KW] yet." }],
    keywords: ["CAT_HOUSE", "CAT_LOYALTY"],
    dict,
  });

  fill(
    info,
    chip(".act", actTip, null, h("span.cat-chip-faction", `${faction} · `), act),
    chip((cur.gold_rate ?? 0) < 0 ? ".warn" : "", calcTip(`Gold ${cur.gold ?? 0} (${fmtRate(cur.gold_rate)})`, cur.gold_calc), () => open.cities(), `Gold ${cur.gold ?? 0} (${fmtRate(cur.gold_rate)})`),
    chip(`.research${res || idleLore ? "" : accent}`, researchTip, () => open.research("research"), res ? `Research: ${res.name}, ${res.turns} t` : idleLore ? "Research: none open" : "Research: none"),
    chip("", calcTip(`Mana ${cur.mana ?? 0} (${fmtRate(cur.mana_rate)})`, cur.mana_calc), () => open.research("rituals"), `Mana ${cur.mana ?? 0} (${fmtRate(cur.mana_rate)})`),
    chip(`.strain${strainCls}`, strainTip, () => open.worldseal(), counting ? `Blowout in ${ap.countdown} turns` : `Worldstrain ${ap.counter ?? 0}/100`),
    chip(court.length ? accent : "", courtTip, () => open.court(), court.length ? `Court: ${court.length} waiting` : `Court ${renown.total ?? 0}/${renown.next_threshold ?? 0}`),
    chip(inactive.length ? ".warn" : "", housesTip, () => open.houses(), `Houses ${houses.length}`),
  );

  // hud.js keeps the End turn button's icon and style; only the classic block state is cleared.
  const end = document.getElementById("end-turn");
  if (end) {
    end.disabled = S.status?.phase !== "human";
    end.classList.remove("blocked");
    end.title = "End the turn [Enter]";
  }
}

// ---------------------------------------------------------------------------------------------
// The side panel: the squad switcher, the selected squad's orders and its 3x3 formation

/** "Aelwen's 1st Squad" -> "Aelwen 1st". */
function shortName(name) {
  const m = /^(.+?)'s (\S+) Squad$/.exec(name ?? "");
  if (m) return `${m[1]} ${m[2]}`;
  return String(name ?? "").split(" ").slice(0, 2).join(" ");
}

const SUPPLY_CLASS = { supplied: "good", cut: "caution", recovering: "caution", hungry: "warn", starving: "warn" };
const ROW_NAMES = ["Front", "Mid", "Rear"];

/** A button with a rich tip; a disabled one is wrapped, so its tip still shows on hover. */
function orderBtn(tag, { tip, disabled, onclick, pressed }, label) {
  const b = h(tag, { disabled: !!disabled, onclick, "aria-pressed": pressed === undefined ? undefined : String(!!pressed), tip: disabled ? null : tip }, label);
  return disabled && tip ? h("span.cat-tipwrap", { tip }, b) : b;
}

function helpTip(data, title, tipText, why) {
  return catTip({ title, lines: tipText ? [tipText] : [], why, dict: data?.keywords ?? null });
}

function placeName(data, s) {
  const city = citiesOf(data).find((c) => c.x === s.x && c.y === s.y);
  return city ? `${city.name} (${s.x}, ${s.y})` : `(${s.x}, ${s.y})`;
}

export function renderCataclysmPanel() {
  const panel = document.getElementById("unit-panel");
  const data = cat();
  if (!panel || !data) return;

  const sq = selectedSquad();
  if (sq) {
    selectedSquadId = sq.id;
    syncSelectionToMap(sq);
  }

  const ownSquads = squadsOf(data).filter((s) => s.owner === myId());
  const ownCities = citiesOf(data).filter((c) => c.owner === myId());
  const help = data.order_help ?? {};

  const nodes = [];
  nodes.push(
    h(
      "div.cat-switcher",
      h("strong", `Squads ${ownSquads.length}/${data.squad_cap ?? ownSquads.length}`),
      h(
        "div.cat-chip-list",
        ...ownSquads.map((s) =>
          h(
            `button.cat-mini-btn.cat-squad-chip${sq && s.id === sq.id ? ".primary" : ""}${s.moves_left <= 0 ? ".spent" : ""}`,
            {
              tip: [h("div.tip-title", s.name), h("div", `Power ${s.power ?? 0}`), h("div", `Moves ${s.moves_left}/${s.max_moves}`), h("div.dim", placeName(data, s))],
              onclick: () => {
                selectedSquadId = s.id;
                selectedMemberId = null;
                syncSelectionToMap(s);
                mapViewRef?.centerOn(s.x, s.y);
                renderCataclysmPanel();
              },
            },
            `${shortName(s.name)} ${s.moves_left}/${s.max_moves}`,
          ),
        ),
      ),
    ),
  );

  if (sq) nodes.push(squadCard(data, sq, help));

  if (!isoSkin()) {
    nodes.push(
      h(
        "div.cat-switcher",
        h("strong", `Cities ${ownCities.length}`),
        h(
          "div.cat-chip-list",
          ...ownCities.map((c) =>
            h(
              `button.cat-mini-btn${!c.building ? ".warn" : ""}`,
              { onclick: () => (mapViewRef?.centerOn(c.x, c.y), open.city(c.id)) },
              `${c.capital ? "★ " : ""}${c.name} (${c.size})${c.building ? ` · ${c.building.turns} t` : " · no production"}`,
            ),
          ),
        ),
      ),
    );
    const battles = battlesBlock(4);
    if (battles) nodes.push(battles);
  }

  fill(panel, ...nodes);
  renderCataclysmCityPanel();
}

function squadCard(data, sq, help) {
  const tips = data.order_tips ?? help;
  const port = leaderPortrait(sq.captain_portrait, { size: PORTRAIT.small, title: sq.captain_name });
  const supply = sq.supply ?? null;
  const supplyKey = supply?.key ?? (sq.supplied === false ? "cut" : "supplied");
  const supplyText = supply?.label ?? (sq.supplied === false ? "Not supplied" : "Supplied");
  const order = sq.order ?? "none";
  const badge = order === "fortify" ? "Fortified" : order === "raid" ? "Raiding" : null;
  // A finger: the map's last pointer was one, or (before any tap) the device's pointer is coarse.
  const touch = !!mapViewRef?.touch || !!window.matchMedia?.("(pointer: coarse)").matches;

  // The 3x3 formation; a Large unit spans its two lanes.
  const gridCells = [];
  for (let r = 0; r < 3; r++) {
    gridCells.push(h("div.cat-grid-row-label", ROW_NAMES[r]));
    for (let l = 0; l < 3; l++) {
      const covered = (sq.members ?? []).find((m) => m.large && m.row === r && m.lane + 1 === l);
      if (covered) continue;
      const occ = (sq.members ?? []).find((m) => m.row === r && m.lane === l);
      gridCells.push(formationCell(data, sq, occ, r, l));
    }
  }

  // Reinforce: the classes that fit the free places (a Large unit takes 2). Both the selector and
  // the Reinforce button carry a live rich tooltip (unit stats, rows, keywords, and disabled reason).
  const free = (sq.place_cap ?? 0) - (sq.places_used ?? 0);
  const fitting = (data.recruitable_classes ?? []).filter((u) => u.unlocked && (u.large ? 2 : 1) <= free);
  let reinforceKey = fitting[0]?.key ?? null;
  const chosenCls = () => fitting.find((u) => u.key === reinforceKey) ?? fitting[0] ?? null;
  let reinforceRow = null;
  if (free > 0 && fitting.length) {
    const getTip = () => reinforceTip(data, sq, chosenCls());
    const initWhy = reinforceWhy(data, sq, chosenCls());
    const whyEl = h("small.cat-why", initWhy ?? "");
    whyEl.hidden = !initWhy;
    const btn = h(
      "button.primary",
      {
        disabled: !!initWhy,
        tip: getTip,
        onclick: () => reinforceKey && catCmd({ c: "cat_reinforce", squad: sq.id, class: reinforceKey }),
      },
      "Reinforce",
    );
    const sel = h(
      "select",
      {
        "aria-label": "Class to recruit",
        tip: getTip,
        onchange: (ev) => {
          reinforceKey = ev.target.value;
          const w = reinforceWhy(data, sq, chosenCls());
          btn.disabled = !!w;
          whyEl.textContent = w ?? "";
          whyEl.hidden = !w;
        },
      },
      ...fitting.map((u) => h("option", { value: u.key }, `${u.name}, ${u.cost_gold} Gold${u.large ? ", 2 places" : ""}`)),
    );
    reinforceRow = h(
      "div.cat-reinforce-block",
      h("div.cat-reinforce-row", sel, h("span.cat-tipwrap", { tip: getTip }, btn)),
      whyEl,
    );
  } else if (free > 0) {
    const getTip = () => reinforceTip(data, sq, null);
    reinforceRow = h(
      "div.cat-reinforce-block",
      h("div.cat-reinforce-row", h("span.cat-tipwrap", { tip: getTip }, h("button.primary", { disabled: true }, "Reinforce"))),
      h("small.cat-why", sq.in_home_borders === false ? "Inside your borders only." : "No unlocked unit fits the remaining place."),
    );
  }

  const upgradeable = (sq.members ?? []).filter((m) => m.upgrades_to && sq.in_home_borders);
  const byKey = new Map((data.recruitable_classes ?? []).map((u) => [u.key, u]));
  const gold = data.currencies?.gold;
  const raid = order === "raid";
  const foundWhy = sq.found_why ?? null;

  const actions = h(
    "div.cat-actions-row",
    orderBtn("button", { tip: helpTip(data, "Auto-arrange", tips.auto_arrange ?? help.auto_arrange), onclick: () => catCmd({ c: "cat_auto_arrange", squad: sq.id }) }, "Auto-arrange"),
    orderBtn(`button${order === "fortify" ? ".on" : ""}`, { tip: helpTip(data, "Fortify", tips.fortify ?? help.fortify), pressed: order === "fortify", onclick: () => catCmd({ c: "cat_fortify", squad: sq.id }) }, "Fortify"),
    orderBtn("button", { tip: helpTip(data, "Road", tips.road ?? help.road, sq.road_why), disabled: sq.can_road === undefined ? sq.moves_left <= 0 : !sq.can_road, onclick: () => catCmd({ c: "cat_road", squad: sq.id }) }, "Road"),
    orderBtn("button", { tip: helpTip(data, "Fort", tips.fort ?? help.fort, sq.fort_why), disabled: sq.can_fort === undefined ? sq.moves_left <= 0 : !sq.can_fort, onclick: () => catCmd({ c: "cat_fort", squad: sq.id }) }, "Fort"),
    sq.on_enemy_countryside || raid
      ? orderBtn(`button.primary${raid ? ".on" : ""}`, { tip: helpTip(data, "Raid", tips.raid ?? help.raid), pressed: raid, onclick: () => catCmd({ c: "cat_raid", squad: sq.id, active: !raid }) }, raid ? "Stop raid" : "Raid")
      : null,
    sq.has_lair ? orderBtn("button.primary", { tip: helpTip(data, "Explore ruin", tips.explore ?? help.explore), onclick: () => catCmd({ c: "cat_explore", squad: sq.id }) }, "Explore ruin") : null,
    sq.can_found_city
      ? orderBtn("button.primary", { tip: helpTip(data, "Found city", tips.found_city ?? help.found_city, foundWhy), disabled: !!foundWhy, onclick: () => catCmd({ c: "cat_found_city", squad: sq.id }) }, `Found city (${sq.found_cost} Gold)`)
      : null,
    sq.can_disband
      ? orderBtn(
          "button",
          {
            tip: helpTip(data, "Disband", tips.disband ?? help.disband),
            onclick: async () => {
              const text = [`Disband ${sq.name}.`];
              if (help.disband) text.push(help.disband);
              const yes = await ask("Disband squad", text, [
                { label: "Disband", value: true, danger: true },
                { label: "Cancel", value: null },
              ]);
              if (yes === true) catCmd({ c: "cat_disband", squad: sq.id });
            },
          },
          "Disband",
        )
      : null,
  );

  const supplyTipText = supply?.tip ?? supply?.detail ?? null;
  const supplyTip = supplyTipText ? catTip({ title: supplyText, lines: [supplyTipText], dict: data.keywords ?? null }) : null;

  return h(
    "div.cat-squad-card",
    h(
      "div.cat-squad-body",
      h(
        "div.cat-squad-left",
        h(
          "div.cat-squad-head",
          port,
          h(
            "div",
            h("strong", sq.name),
            badge ? h("span.cat-order-badge", ` ${badge}`) : null,
            sq.captain_name ? h("div.small.dim", `Captain ${sq.captain_name}${sq.captain_aura ? ` · ${sq.captain_aura}` : ""}`) : null,
            h(
              "div.small",
              `Moves ${sq.moves_left}/${sq.max_moves} · Places ${sq.places_used}/${sq.place_cap} · Power ${sq.power} · `,
              h(`span.cat-supply.${SUPPLY_CLASS[supplyKey] ?? "dim"}`, { tip: supplyTip }, supplyText),
            ),
          ),
        ),
        h("div.small.dim", touch ? "Tap a tile twice: move or attack." : "Right-click a tile: move or attack. Click two cells: swap them."),
        actions,
        reinforceRow,
        ...upgradeable.map((m) => {
          const targetCls = byKey.get(m.upgrades_to_key) ?? (data.recruitable_classes ?? []).find((u) => u.name === m.upgrades_to) ?? null;
          const shortGold = typeof gold === "number" && typeof m.upgrade_cost === "number" && gold < m.upgrade_cost;
          return orderBtn(
            "button.cat-mini-btn",
            {
              tip: () => upgradeTip(data, sq, m, targetCls),
              disabled: shortGold,
              onclick: () => catCmd({ c: "cat_upgrade", squad: sq.id, member: m.id }),
            },
            `${ROW_NAMES[m.row] ?? ""} ${m.lane + 1}: ${m.class} → ${m.upgrades_to}, ${m.upgrade_cost} Gold`,
          );
        }),
      ),
      h("div.cat-formation-grid", ...gridCells),
    ),
  );
}

function formationCell(data, sq, occ, r, l) {
  const isSel = occ && occ.id === selectedMemberId;
  const rowAct = occ?.actions?.[r];
  const actText = rowAct && rowAct.n > 0 ? `${rowAct.kind} ×${rowAct.n}` : "No action";
  const selMember = selectedMemberId !== null ? (sq.members ?? []).find((m) => m.id === selectedMemberId) : null;
  const tip = occ
    ? () => memberTip(data, occ, r, selMember)
    : catTip({
        title: `${ROW_NAMES[r]} ${l + 1}`,
        lines: [{ cls: ".dim", text: selMember ? `Click: move ${selMember.name} here.` : "Empty cell in the [KW=CAT_FORMATION]Formation[\\KW]." }],
        dict: data?.keywords ?? null,
      });
  return h(
    `button.cat-cell${occ ? ".occupied" : ""}${isSel ? ".selected" : ""}${occ?.large ? ".large" : ""}`,
    {
      tip,
      "aria-label": occ ? `${ROW_NAMES[r]} ${l + 1}: ${occ.name}` : `${ROW_NAMES[r]} ${l + 1}: empty`,
      style: occ?.large ? { gridColumn: "span 2" } : undefined,
      onclick: async () => {
        if (selectedMemberId !== null) {
          if (occ && occ.id === selectedMemberId) {
            selectedMemberId = null;
            renderCataclysmPanel();
            return;
          }
          const mId = selectedMemberId;
          selectedMemberId = null;
          await catCmd({ c: "cat_set_cell", squad: sq.id, member: mId, row: r, lane: l });
        } else if (occ) {
          selectedMemberId = occ.id;
          renderCataclysmPanel();
        }
      },
    },
    occ
      ? [
          h("span.cat-cell-name", `${occ.is_hero ? "★" : ""}${occ.class}`),
          h("small.cat-cell-act", actText),
          h("div.cat-hp-bar", h("div.cat-hp-fill", { style: { width: `${occ.hp_pct}%` } })),
        ]
      : h("span.dim", "·"),
  );
}

function battlesBlock(n) {
  const recent = S.view?.battles ?? [];
  if (!recent.length) return null;
  return h(
    "div.cat-switcher",
    h("strong", "Recent battles"),
    ...recent.slice(0, n).map((b) =>
      h("button.cat-mini-btn", { onclick: () => openReport(b.id) }, `Battle ${b.id}: ${b.sides?.[0]?.label ?? "?"} vs ${b.sides?.[1]?.label ?? "?"}`),
    ),
  );
}

export function renderCataclysmCityPanel() {
  const el = document.getElementById("iso-city");
  if (!el || !isoSkin()) return;
  const data = cat();
  if (!data || !isCataclysm()) {
    el.hidden = true;
    fill(el);
    return;
  }
  const ownCities = citiesOf(data).filter((c) => c.owner === myId());
  const cap = ownCities.find((c) => c.capital) ?? ownCities[0] ?? null;
  el.hidden = false;
  const crewText = cap?.crew?.text ?? (cap?.crew_task ? `Work Crew: ${cap.crew_task.label}, ${cap.crew_task.progress} of ${cap.crew_task.needed} turns.` : null);
  fill(
    el,
    h("h3.iso-title", cap ? `${cap.capital ? "★ " : ""}${cap.name}` : "No city"),
    cap
      ? h(
          "div.cat-iso-city-box",
          h("div.small", `Size ${cap.size} · Districts ${(cap.districts ?? []).length}/${cap.max_district_slots} · Defense +${cap.defense_pct}%`),
          h("div.small.dim", `Industry +${cap.industry_rate} · Gold +${cap.gold_rate} · Lore +${cap.lore_rate}`),
          h("div.small", cap.building ? `Production: ${cap.building.label}, ${cap.building.turns} t` : h("span.warn", "Production: none")),
          crewText ? h("div.small.dim", crewText) : null,
          ...(cap.notes ?? []).map((n) => h(`div.small.${n.tone === "warn" ? "warn" : "caution"}`, n.text)),
          h("button.iso-open", { onclick: () => (mapViewRef?.centerOn(cap.x, cap.y), open.city(cap.id)) }, "Open city"),
        )
      : null,
    ownCities.length > 1
      ? h(
          "div.cat-switcher",
          h("strong", `Cities ${ownCities.length}`),
          h(
            "div.cat-chip-list",
            ...ownCities.map((c) =>
              h(
                `button.cat-mini-btn${!c.building ? ".warn" : ""}`,
                { tip: c.building ? `Production: ${c.building.label}, ${c.building.turns} t` : "Production: none", onclick: () => (mapViewRef?.centerOn(c.x, c.y), open.city(c.id)) },
                `${c.name} (${c.size})`,
              ),
            ),
          ),
        )
      : null,
    battlesBlock(3),
  );
}
