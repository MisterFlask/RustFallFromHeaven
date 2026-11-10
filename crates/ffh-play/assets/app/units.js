// Units (design §5.1 "Units"): the selection, the stack list, the actions panel from the `group`
// query, moving and attacking with the path and combat previews, Shift waypoints, interface
// modes (go-to, route-to, range attack: pick a plot), grouping, and cycling over the ready groups.

import * as api from "./api.js";
import { S, emit, on, dname, tname, tidx, me, unitsAt, ownUnitsAt, cityAt, playerName, readyGroups, selectableGroups, nextSelectableGroup, cycleGroups, selectableGroupsOnPlot, option, moves, turns, info, observing, eventKind } from "./state.js";
import * as K from "./hotkeys.js";
import { h, fill, toast, coarse } from "./dom.js";
import { openModal } from "./modals.js";
import { run, actionCommand, sending } from "./commands.js";
import { plotDistance, plotIndex, humanize } from "./tiles.js";
import { render as markup } from "./markup.js";
import { look, iconEl, unitIcon, barEl, pix, typeIcon, promotionIcon, uiIcon } from "./look.js";
import { isoSkin, isoIcon, actionIcon, portrait, isoBar, focusOn, unitFigure, updateInsets } from "./isoui.js";
import { effectsOf, infoTip, partsList } from "./infotips.js";
import { marchLine } from "./armies.js";
import { exploreShort, exploreFull } from "./explore.js";
import { battleTerrain, openPlan, preferredRow, flashBattle, engageNumbers, engageBrief } from "./engagements.js";

/** An 8 px icon of the ui set, or the glyph before the art loaded. */
const icon8 = (name, glyph, title) => iconEl(`icon8/${name}`, { title }) ?? glyph;

/** A unit's owner as the art draws it (the map's heraldry), or null. */
const ownerLook = (p) => mapView?.art?.owners?.get(p) ?? null;

let mapView = null;
let panel = null;
/** The last `group` query reply, the units it answers (`groupFor`, as `unitsKey`), the key of
 *  the latest request (units, view version, turn) and that request while it runs. Only a reply
 *  for the selected units is used (`reply()`): the previous selection's actions are neither
 *  drawn nor sent while the new selection's reply is on its way (P5 review). */
let group = null;
let groupFor = "";
let groupKey = "";
let groupPending = null;
/** An interface mode: `{kind: "target", action, waypoints?}` (`waypoints`: go-to waypoints
 *  were added with Shift, the mode ends when Shift is released). */
let mode = null;
/** After a command on the selection: the plot the selection was on (C++ `getOriginalPlot`),
 *  to move on from once the group no longer waits for orders (`updateSelection`); else null. */
let advance = null;
/** An attack whose outcome is inferred from the view diff (the server sends no combat result):
 *  the target tile, the attacker's plot, and the defenders'/attackers' HP before the attack,
 *  snapshotted when the command is applied (S.units is still pre-combat then) and consumed in
 *  `viewChanged` to flash the struck tile. Null when the last command was not an attack. */
let pendingCombat = null;
/** Shift-right-click waypoints, held while Shift is down and sent when it is released
 *  (`flushWaypoints`): `{units, points: [[x, y]]}` or null. The exe sends each one at once but
 *  no AutoMoves while Shift is down (CvGame::updateTestEndTurn `if (!gDLL->shiftKey())`), and
 *  the session runs the AutoMoves rule right after a command that leaves nothing ready, so the
 *  client holds them instead (P5 review). */
let waypoints = null;
/** Touch: the plot a first tap previewed a move to, as `units@x,y` (a second tap there sends
 *  the move: a finger has neither hover nor a right button), else null. */
let armed = null;
/** Hover previews: the plot, and the replies. */
let hover = null;
let hoverTimer = null;
let preview = { path: null, combat: null, plot: null };

/** A selection's units as a key. */
function unitsKey(units) {
  return (units ?? []).join(",");
}

/** The `group` reply when it answers the selected units, else null. */
function reply() {
  return group && S.sel && groupFor === unitsKey(S.sel.units) ? group : null;
}

export function initUnits(view, panelEl) {
  mapView = view;
  panel = panelEl;
  on("applied", (c, reply) => {
    if (c?.units && S.sel && c.units.some((u) => S.sel.units.includes(u))) advance = { x: S.sel.x, y: S.sel.y };
    // Engagement combat: the battle the command launched (several units may fall at once, and
    // the selected ones may not have fought) flashes from its report, not from the HP diff.
    if (reply?.r?.battle) {
      pendingCombat = null;
      flashBattle(mapView, reply.r.battle);
      return;
    }
    if (pendingCombat) {
      pendingCombat.applied = true;
      // The view that shows the attack's result is at least this new.
      pendingCombat.version = reply?.status?.view_version;
    }
  });
  on("refused", () => {
    pendingCombat = null;
  });
  on("sending", (c) => {
    // Combat carries no result, so snapshot an attack before it is sent (S.units is pre-combat)
    // and infer the outcome from the diff in viewChanged. The new view can arrive before the
    // command's reply, so the snapshot is taken here, not on "applied". An attack is a move onto
    // a tile with a foreign unit (the same gate the combat-odds preview uses). Under engagement
    // combat a land attack on land is a battle, whose flash comes from its report in the reply
    // ("applied"): no snapshot, or a view that arrives first would flash the HP diff as well.
    pendingCombat = null;
    if (c?.c === "move" && typeof c.x === "number" && typeof c.y === "number" && !battleTerrain(c.units, c.x, c.y)) {
      const mep = me()?.player;
      const defenders = unitsAt(c.x, c.y).filter((u) => u.owner !== mep);
      if (defenders.length) {
        pendingCombat = {
          tx: c.x,
          ty: c.y,
          ax: S.sel?.x,
          ay: S.sel?.y,
          defenders: defenders.map((u) => ({ id: u.id, hp: u.hp })),
          attackers: (c.units ?? []).map((id) => S.units.get(id)).filter(Boolean).map((u) => ({ id: u.id, hp: u.hp })),
          applied: false,
        };
      }
    }
  });
}

/** Flashes the struck tile when a snapshotted attack (`pendingCombat`) resolved: a defender lost
 *  (death) or was hurt (hit), else the attacker if it took the loss. Best-effort: a view that
 *  shows no change yet keeps the snapshot until the command's reply has come and a view as new
 *  as the reply's status (a view fetched before the attack can arrive after its reply); after
 *  that, an unmatched snapshot just clears. */
function flashCombat() {
  const pc = pendingCombat;
  if (!pc || !mapView) {
    pendingCombat = null;
    return;
  }
  try {
    const gone = (id) => !S.units.has(id);
    const hpNow = (id) => S.units.get(id)?.hp;
    const defDied = pc.defenders.some((d) => gone(d.id));
    const defHit = !defDied && pc.defenders.some((d) => hpNow(d.id) < d.hp);
    const atkDied = pc.attackers.some((a) => gone(a.id));
    const atkHit = !atkDied && pc.attackers.some((a) => hpNow(a.id) < a.hp);
    if (defDied) mapView.flashTile(pc.tx, pc.ty, "death");
    else if (defHit) mapView.flashTile(pc.tx, pc.ty, "hit");
    else if (atkDied && pc.ax != null) mapView.flashTile(pc.ax, pc.ay, "death");
    else if (atkHit && pc.ax != null) mapView.flashTile(pc.ax, pc.ay, "hit");
    else if (!pc.applied) return; // not resolved in this view yet
    else if (typeof pc.version === "number" && (S.view?.view_version ?? pc.version) < pc.version) return; // an older view
  } catch {
    /* a flash is never worth a thrown handler */
  }
  pendingCombat = null;
}

// ---------------------------------------------------------------------------------------------
// Selection

/** Selects own units `ids` (on one plot), or nothing. */
export function select(ids, { center = false } = {}) {
  // Waypoints of other units are sent before the selection changes.
  if (waypoints && unitsKey(waypoints.units) !== unitsKey(ids)) flushWaypoints();
  const units = (ids ?? []).map((id) => S.units.get(id)).filter((u) => u && u.owner === me()?.player);
  if (units.length === 0) {
    S.sel = null;
  } else {
    const { x, y } = units[0];
    S.sel = { units: units.filter((u) => u.x === x && u.y === y).map((u) => u.id), x, y };
    if (center) mapView.reveal(x, y);
  }
  setMode(null);
  advance = null;
  armed = null;
  // Another selection's panel starts at its top (a phone's card scrolls).
  panel?.parentElement?.scrollTo?.({ top: 0 });
  clearPreview();
  syncOverlay();
  refreshGroup();
  emit("selection");
}

/** Selects group `g` (all its units). */
export function selectGroup(g, opts) {
  const grp = S.groups.get(g);
  if (grp) select(grp.units, opts);
}

export function clearSelection() {
  select([]);
}

/** The groups the cycle selects (C++ `readyToSelect()`, every unit can move), as a Set. */
function selectableSet() {
  return new Set(selectableGroups().filter((g) => S.groups.has(g)));
}

/** C++ `CvGame::cycleSelectionGroups(bClear, bForward)` (W, Enter, Next; P6): the next group in
 *  the player's group cycle (`view.group_cycle`, C++ `m_groupCycle`) after the selection's that
 *  the cycle selects (`readyToSelect()`: every unit can move), going round; with no selection,
 *  the selectable group nearest the centre of the map view (`findSelectionGroup` at the look-at
 *  plot). When the search went round and a group has orders to continue, the exe sends
 *  AutoMoves (they move now). */
export function selectNext({ forward = true } = {}) {
  const cur = currentGroup();
  if (cur === null) {
    const near = mapView?.center?.() ?? null;
    const dist = near && S.map ? (ax, ay, bx, by) => plotDistance(S.map, ax, ay, bx, by) : null;
    const g = nextSelectableGroup(-1, near, dist);
    if (g === null || g === undefined) return false;
    selectGroup(g, { center: true });
    return true;
  }
  const { group, wrapped } = cycleGroups(S.view?.group_cycle ?? selectableGroups(), selectableSet(), cur, forward);
  if (wrapped && S.view?.has_auto_unit) autoMoves();
  if (group === null) return false;
  selectGroup(group, { center: true });
  return true;
}

/** C++ `CvGame::updateSelectionList` (after a command, P6): once the selected group no longer
 *  waits for orders (`readyToSelect(true)`: no unit can move, `readyGroups`), the next
 *  selectable group (`readyToSelect()`) on the plot it started from, then on its plot, then the
 *  next in the group cycle (`cycleSelectionGroups`, AutoMoves on a wrap), and no selection when
 *  none is selectable; nothing when the player turned unit cycling off
 *  (PLAYEROPTION_NO_UNIT_CYCLING). */
function updateSelection(origin) {
  if (option("PLAYEROPTION_NO_UNIT_CYCLING")) return false;
  const cur = currentGroup();
  for (const p of [origin, S.sel ? { x: S.sel.x, y: S.sel.y } : null]) {
    if (!p) continue;
    const g = selectableGroupsOnPlot(p.x, p.y, cur)[0];
    if (g !== undefined) {
      selectGroup(g, { center: true });
      return true;
    }
  }
  if (cur === null) {
    // The selected units are gone (killed, joined a city): the selectable group nearest the
    // plot they were on (C++ `findSelectionGroup` at the look-at plot).
    const dist = origin && S.map ? (ax, ay, bx, by) => plotDistance(S.map, ax, ay, bx, by) : null;
    const g = nextSelectableGroup(-1, origin, dist);
    if (g === null || g === undefined) return false;
    selectGroup(g, { center: true });
    return true;
  }
  const { group, wrapped } = cycleGroups(S.view?.group_cycle ?? selectableGroups(), selectableSet(), cur);
  if (wrapped && S.view?.has_auto_unit) autoMoves();
  if (group === null || group === cur) {
    S.sel = null;
    syncOverlay();
    refreshGroup();
    emit("selection");
    return false;
  }
  selectGroup(group, { center: true });
  return true;
}

/** CvMessageControl::sendAutoMoves (a cycle that went round with orders waiting). */
function autoMoves() {
  setTimeout(async () => {
    for (let i = 0; sending() && i < 200; i++) await new Promise((r) => setTimeout(r, 20));
    if (S.status?.phase === "human") run({ c: "auto_moves" }, { quiet: true });
  }, 0);
}

/** The group the selection is (its first unit's). */
function currentGroup() {
  if (!S.sel) return null;
  return S.units.get(S.sel.units[0])?.group ?? null;
}

/** Called when a new view arrived: keeps the selection on the units that still exist (they may
 *  have moved), or moves on to the next ready group after a command finished with it. */
export function viewChanged() {
  if (pendingCombat) flashCombat();
  if (S.sel) {
    const alive = S.sel.units.map((id) => S.units.get(id)).filter((u) => u && u.owner === me()?.player);
    if (alive.length === 0) {
      S.sel = null;
    } else {
      const { x, y } = alive[0];
      S.sel = { units: alive.filter((u) => u.x === x && u.y === y).map((u) => u.id), x, y };
    }
  }
  if (advance) {
    const origin = advance;
    advance = null;
    const g = currentGroup();
    if (!S.sel || g === null || !readyGroups().includes(g)) updateSelection(origin) || (S.sel && refreshGroup());
    else refreshGroup();
  } else {
    refreshGroup();
  }
  clearPreview();
  syncOverlay();
}

/** A click on plot (x, y): picks the plot in an interface mode, else selects the units there
 *  (again on the same plot: the next group there), else opens the city or shows the plot. A tap
 *  (`ev.pointerType` "touch", after the map showed the plot's preview) with units selected on a
 *  plot without units of ours is the right-click's move, in two taps: the first shows the way
 *  (path, turns, attack odds: the tooltip), a second tap on the same plot goes. */
export function click(x, y, ev) {
  const touch = ev?.pointerType === "touch";
  if (mode) {
    pickPlot(x, y, ev);
    if (touch) mapView.clearHover();
    return;
  }
  // An observed game has no units of yours: a click shows the plot (its stack, its events).
  if (observing()) {
    S.sel = null;
    syncOverlay();
    emit("selection");
    renderPlotInfo(x, y);
    return;
  }
  const own = ownUnitsAt(x, y);
  if (touch && S.sel && !own.length) {
    const key = `${unitsKey(S.sel.units)}@${x},${y}`;
    if (armed !== key) {
      armed = key;
      return;
    }
    armed = null;
    mapView.clearHover();
    moveTo(x, y);
    return;
  }
  armed = null;
  if (own.length) {
    if (touch) mapView.clearHover();
    const groups = [...new Set(own.map((u) => u.group))];
    let g = groups.find((gr) => readyGroups().includes(gr)) ?? groups[0];
    if (S.sel && S.sel.x === x && S.sel.y === y && groups.length > 1) {
      const i = groups.indexOf(currentGroup());
      g = groups[(i + 1) % groups.length];
    }
    if (ev?.shiftKey) select(allHere(own));
    else selectGroup(g);
    return;
  }
  const city = cityAt(x, y);
  if (city && city.owner === me()?.player) {
    emit("open-city", city.id);
    return;
  }
  // (The units selected until this click: the plot panel's lair or goody hut odds are theirs.)
  const before = S.sel?.units ?? null;
  S.sel = null;
  syncOverlay();
  emit("selection");
  renderPlotInfo(x, y, before);
}

/** A right-click on (x, y): move the selection there (Shift: add a waypoint). Under engagement
 *  combat an attack of an army or a stack opens the battle plan first; Shift on a plot next to the
 *  selection where it would fight a battle now (the `combat` query says so), with no waypoints
 *  laid, attacks at once in the rules' formation. Shift clicks go in order (`shiftChain`): one that
 *  asks the engine first does not let a later waypoint overtake it. */
export function rightClick(x, y, ev) {
  if (mode) {
    setMode(null);
    return;
  }
  if (!S.sel || observing()) return;
  if (!ev?.shiftKey) return moveTo(x, y);
  const units = S.sel.units;
  shiftChain = shiftChain.then(async () => {
    if (!S.sel || unitsKey(S.sel.units) !== unitsKey(units)) return;
    const laying = waypoints && unitsKey(waypoints.units) === unitsKey(units);
    // (The attack is not waited for: a war it asks to declare must not hold later clicks.)
    if (!laying && (await battleNow(x, y))) moveTo(x, y, { direct: true });
    else addWaypoint(x, y);
  }).catch((e) => console.warn("right-click:", e));
}

/** The Shift right-clicks in order (`rightClick`). */
let shiftChain = Promise.resolve();

/** Moves the selection to (x, y) now, replacing its orders (waypoints not sent yet are
 *  dropped: the move replaces them). Under engagement combat an attack of an army or a stack
 *  opens the battle plan instead, unless `direct` (the move attacks in the rules' formation). */
export async function moveTo(x, y, { direct = false } = {}) {
  if (!S.sel) return;
  if (waypoints && unitsKey(waypoints.units) === unitsKey(S.sel.units)) clearWaypoints();
  if (S.sel.x === x && S.sel.y === y) return;
  if (!direct && (await planAttack(x, y))) return;
  if (!S.sel) return;
  await run({ c: "move", units: S.sel.units, x, y, append: false });
}

// ---------------------------------------------------------------------------------------------
// Shift waypoints

/** Adds waypoint (x, y) for the selection (Shift is down). */
export function addWaypoint(x, y) {
  if (!S.sel || S.status?.phase !== "human") return;
  if (!waypoints || unitsKey(waypoints.units) !== unitsKey(S.sel.units)) {
    flushWaypoints();
    waypoints = { units: [...S.sel.units], points: [] };
  }
  waypoints.points.push([x, y]);
  mapView?.setOverlay({ waypoints: waypoints.points });
  renderPanel();
}

/** The waypoints not sent yet (for the tests and the panel). */
export function pendingWaypoints() {
  return waypoints ? { units: [...waypoints.units], points: waypoints.points.map((p) => [...p]) } : null;
}

/** Drops the waypoints not sent yet (Esc). Returns whether there were any. */
export function clearWaypoints() {
  const had = !!waypoints;
  waypoints = null;
  mapView?.setOverlay({ waypoints: null });
  if (had) renderPanel();
  return had;
}

/** Shift was released (or the window lost the focus): sends the waypoints, each a `move` with
 *  `append`, as the exe sent them, waiting for the control point between them (the first may
 *  let the AutoMoves rule run the game on). A refused or cancelled one drops the rest. Once
 *  sent, the selection moves on when the group no longer waits for orders (the `applied`
 *  rule). */
export async function flushWaypoints() {
  const w = waypoints;
  clearWaypoints();
  if (!w?.points.length) return;
  let st = S.status;
  for (const [x, y] of w.points) {
    if (!(await atControl(st))) return;
    for (let i = 0; sending() && i < 500; i++) await new Promise((r) => setTimeout(r, 20));
    const r = await run({ c: "move", units: w.units, x, y, append: true });
    if (!r?.r?.applied) return;
    st = r.status ?? S.status;
  }
}

/** Waits (long-polling from status `st`) until the player has control again; false when the
 *  game left the player's turn (over, crashed, no game). */
async function atControl(st) {
  for (let i = 0; st && i < 1000; i++) {
    if (st.phase === "human") return true;
    if (st.phase !== "running" && st.phase !== "decision") return false;
    try {
      st = await api.wait(st.version, 20000);
    } catch {
      return false;
    }
  }
  return false;
}

/** Sends the action (a `group` query action); a `target` action first asks for a plot, a spell
 *  cast at a distance for its target (`castTargeted`). An action of other units than the selected
 *  ones (a reply that came late) is not sent. */
export async function doAction(a) {
  if (!a.enabled) return;
  if (a.cmd?.units && unitsKey(a.cmd.units) !== unitsKey(S.sel?.units)) return;
  if (a.cmd?.c === "cast_at" && !(S.sel?.units ?? []).includes(a.cmd.unit)) return;
  if (a.kind === "spell" && a.target_kind) return castTargeted(a);
  if (a.kind === "target") {
    if (!a.any && (a.targets ?? []).length === 0) {
      toast("No target in range.", "error");
      return;
    }
    setMode({ kind: "target", action: a });
    return;
  }
  await run(a.cmd);
}

/** A key press for the selection's actions (P6: Civ4's hotkeys, from the XML through
 *  `/api/data`): pressed just after the selection changed, it waits for the selection's own
 *  actions (P5 review: not the previous selection's), then runs the enabled action bound to the
 *  key with the highest hotkey priority (`hotkeys.js`). Resolves with whether one ran. */
export async function hotkeyEvent(ev) {
  const e = { code: ev.code, key: ev.key, shiftKey: ev.shiftKey, altKey: ev.altKey, ctrlKey: ev.ctrlKey, metaKey: ev.metaKey };
  for (let i = 0; i < 3 && S.sel && !reply() && groupPending; i++) await groupPending;
  const r = reply();
  if (!r?.actions) return false;
  const a = K.actionForKey(r.actions, e, S.data);
  if (!a) return false;
  await doAction(a);
  return true;
}

/** The selection's actions (the `group` reply held; none while it is on its way). */
export function currentActions() {
  return reply()?.actions ?? [];
}

/** Enters an interface mode, `{kind: "target", action}` (pick a plot for the action); null
 *  leaves it. */
export function setMode(m) {
  mode = m;
  // A spell cast at a distance outlines the plots of its targets (`plots`).
  const targets = m?.kind === "target" ? (m.plots ?? (m.action.any ? null : m.action.targets)) : null;
  // The route-to mode's path preview is drawn in ochre (docs/ART.md 11.7).
  mapView?.setOverlay({ targets: targets ?? (m ? [] : null), routeTo: m?.kind === "target" && m.action.mode === "INTERFACEMODE_ROUTE_TO" });
  document.body.classList.toggle("picking", !!m);
  emit("mode", m);
}

export function currentMode() {
  return mode;
}

/** Whether action `a` is the go-to interface mode (INTERFACEMODE_GO_TO: a `move`). */
export function isGoTo(a) {
  return a?.kind === "target" && a.mode === "INTERFACEMODE_GO_TO" && a.cmd?.c === "move";
}

/** A plot picked in an interface mode. Go-to with Shift held adds a waypoint: C++ sends a
 *  go-to pick through `selectionListMove(pPlot, bAlt, bShift, bCtrl)`, which pushes
 *  MISSION_MOVE_TO with `bShift` as append (and keeps it through the DECLAREWARMOVE popup); the
 *  waypoints are held like Shift-right-click's and sent when Shift is released, and the mode
 *  stays on meanwhile so that more can be added (P6 review). */
async function pickPlot(x, y, ev) {
  const m = mode;
  if (m.cast) return pickCast(m, x, y);
  if (!S.sel) {
    setMode(null);
    return;
  }
  const a = m.action;
  if (!a.any && !(a.targets ?? []).some(([tx, ty]) => tx === x && ty === y)) {
    setMode(null);
    toast("Not a valid target.", "error");
    return;
  }
  if (ev?.shiftKey && isGoTo(a)) {
    m.waypoints = true;
    addWaypoint(x, y);
    return;
  }
  setMode(null);
  if (isGoTo(a)) await moveTo(x, y);
  else await run(actionCommand(a, x, y));
}

// ---------------------------------------------------------------------------------------------
// Spells cast at a distance (docs/CARDS.md, section 3)

/** The `cast_at` command of spell action `a` at target `t` (an entry of its `targets`). */
function castCommand(a, t) {
  return { ...a.cmd, target: t.target };
}

/** The distinct plots `[x, y]` of targets `targets` (those picked on the map). */
function targetPlots(targets) {
  const seen = new Set();
  const out = [];
  for (const t of targets) {
    if (!Array.isArray(t.at) || seen.has(`${t.at[0]},${t.at[1]}`)) continue;
    seen.add(`${t.at[0]},${t.at[1]}`);
    out.push([t.at[0], t.at[1]]);
  }
  return out;
}

/** Asks which of `targets` (`{target, label, info?}`) action `a` is cast at: a list of their names,
 *  each with its help when it is an info table's entry (a trait, a building, ...). Resolves with
 *  the target, or null. */
export function chooseTarget(a, targets, title = null) {
  const m = openModal({
    title: title ?? actionLabel(a),
    tag: "cast-target",
    wide: true,
    body: [
      h(
        "div.options",
        targets.map((t) => {
          // The entry's help, without a first line that only repeats its name (a trait's).
          const lines = t.info ? effectsOf(t.info[0], t.info[1]).split("[NEWLINE]") : [];
          if (lines.length && lines[0].replace(/\[[^\]]*\]/g, "").trim() === t.label) lines.shift();
          const help = lines.join("[NEWLINE]");
          return h(
            "button.option",
            { "data-target": JSON.stringify(t.target), tip: t.info ? infoTip(t.info[0], t.info[1]) : null, onclick: () => m.close(t) },
            h("span", t.label),
            help ? h("small", markup(help)) : null,
          );
        }),
      ),
    ],
    buttons: [{ label: "Cancel", value: null }],
  });
  return m.done;
}

/** Casts spell action `a`, one cast at a distance (`target_kind`, `targets`: the action carries
 *  its valid targets): a plot, city or unit is picked on the map, its plots outlined (`world`: a
 *  world action, picked without a selection); a player or one of the spell's options from a list.
 *  The cast is `cast_at` at the pick (a card spell then asks to confirm). */
export async function castTargeted(a, { world = false } = {}) {
  if (!a.enabled) return;
  const targets = a.targets ?? [];
  if (!targets.length) {
    toast(a.why ?? "No valid target now.", "error");
    return;
  }
  if (a.target_kind === "player" || a.target_kind === "choice") {
    const t = await chooseTarget(a, targets);
    if (t) await run(castCommand(a, t));
    return;
  }
  setMode({ kind: "target", action: a, cast: true, world, plots: targetPlots(targets) });
  if (world || !S.sel) toast(`${actionLabel(a)}: pick its target on the map (the outlined plots). Esc cancels.`, "info", 6000);
}

/** A plot picked for a spell cast at a distance: its target there (several there: which one). */
async function pickCast(m, x, y) {
  const here = (m.action.targets ?? []).filter((t) => Array.isArray(t.at) && t.at[0] === x && t.at[1] === y);
  setMode(null);
  if (!here.length) {
    toast("Not a valid target.", "error");
    return;
  }
  const t = here.length === 1 ? here[0] : await chooseTarget(m.action, here, `${actionLabel(m.action)}: which one?`);
  if (t) await run(castCommand(m.action, t));
}

/** Shift was released (or the window lost the focus): the held waypoints are sent, and a go-to
 *  mode that added them ends. */
export function shiftReleased() {
  if (mode?.waypoints) setMode(null);
  // After the Shift right-clicks still being answered (`rightClick`'s battle check), so their
  // waypoints go with the others; the sends themselves are not waited for.
  let flushed;
  shiftChain = shiftChain.then(() => {
    flushed = flushWaypoints();
  });
  return shiftChain.then(() => flushed);
}

/** The units here that take orders: not a marching recruit (this game's armies: the rules move
 *  it until it arrives). */
const orderable = (units) => units.filter((u) => !u.march);

/** The units "All" and Shift-click select on a plot: those that take orders (a marching recruit
 *  among them would leave the selection without orders), or all when none does (to look at). */
const allHere = (units) => {
  const ready = orderable(units);
  return (ready.length ? ready : units).map((u) => u.id);
};

/** The march of the first selected marching recruit (it takes no orders), or null. */
export function selectionMarch() {
  for (const id of S.sel?.units ?? []) {
    const m = S.units.get(id)?.march;
    if (m) return m;
  }
  return null;
}

/** Merges the units here into one group (`join_group` of each into the first's); marching
 *  recruits stay out (they take no orders). */
export async function groupAll() {
  if (!S.sel) return;
  const own = orderable(ownUnitsAt(S.sel.x, S.sel.y));
  if (!own.length) return;
  const head = S.sel.units.find((id) => own.some((u) => u.id === id)) ?? own[0].id;
  const headGroup = S.units.get(head)?.group;
  for (const u of own) {
    if (u.group !== headGroup) await run({ c: "join_group", unit: u.id, head }, { quiet: true });
  }
  select(own.map((u) => u.id));
}

/** Splits the selection: each selected unit its own group (a marching recruit is alone in its
 *  group already). */
export async function splitAll() {
  if (!S.sel) return;
  const ids = S.sel.units.filter((id) => !S.units.get(id)?.march);
  if (!ids.length) return;
  for (const u of ids) await run({ c: "join_group", unit: u, head: null }, { quiet: true });
  select([ids[0]]);
}

// ---------------------------------------------------------------------------------------------
// Previews: the path and the attack odds under the mouse.

function clearPreview() {
  preview = { path: null, combat: null, plot: null };
  mapView?.setOverlay({ path: null, odds: null });
}

/** The mouse is over (x, y) (null: left the map). */
/** The plot under the mouse, or null. */
export function hoveredPlot() {
  return hover;
}

export function hoverAt(x, y) {
  hover = x === null ? null : { x, y };
  // The preview is gone (the finger panned, tapped elsewhere): so is the move it offered.
  if (x === null) armed = null;
  clearTimeout(hoverTimer);
  preview = { path: null, combat: null, plot: null };
  mapView.setOverlay({ path: null, odds: null });
  if (!hover) return;
  showTip();
  hoverTimer = setTimeout(() => loadPreview(x, y), 90);
}

async function loadPreview(x, y) {
  const key = `${x},${y}`;
  const still = () => hover && `${hover.x},${hover.y}` === key;
  const sel = S.sel;
  const jobs = [];
  if (S.status?.phase === "human" || S.status?.phase === "decision") {
    jobs.push(
      // (The selection: the odds of a lair or goody hut there are for its first unit.)
      api.query("plot", sel ? { x, y, units: sel.units } : { x, y }).then((r) => still() && (preview.plot = r)).catch(() => {}),
    );
  }
  // (A marching recruit takes no orders: no move to preview.)
  if (sel && S.status?.phase === "human" && !(sel.x === x && sel.y === y) && !selectionMarch()) {
    jobs.push(
      api.query("path", { units: sel.units, x, y }).then((r) => {
        if (!still()) return;
        preview.path = r;
        mapView.setOverlay({ path: r });
      }).catch(() => {}),
    );
    const foreign = unitsAt(x, y).some((u) => u.owner !== me()?.player);
    if (foreign) {
      jobs.push(
        api.query("combat", { units: sel.units, x, y }).then((r) => {
          if (!still()) return;
          preview.combat = r;
          // The odds chip at the end of an attack path (permille; air strikes have none).
          mapView.setOverlay({ odds: r?.valid && r.odds !== null && r.odds !== undefined ? r.odds : null });
        }).catch(() => {}),
      );
    }
  }
  await Promise.all(jobs);
  if (still()) showTip();
}

/** The tooltip of the hovered plot: what the map arrays and the view say, then the plot query,
 *  the path and the attack odds once they arrive. */
function showTip() {
  if (!hover || !S.map) return mapView.hideTip();
  const { x, y } = hover;
  const i = plotIndex(S.map, x, y);
  if (i < 0) return mapView.hideTip();
  const m = S.map;
  const lines = [];
  if (m.vis[i] === 0) {
    lines.push(h("div.tip-title", `(${x}, ${y}) unexplored`));
    return mapView.showTip(lines);
  }
  const terrain = dname("TerrainType", m.terrain[i]);
  const pt = tname("PlotType", m.plot_type[i]);
  const parts = [pt === "PLOT_PEAK" ? "Peak" : terrain];
  if (pt === "PLOT_HILLS") parts.push("Hills");
  if (m.feature[i] >= 0) parts.push(dname("FeatureType", m.feature[i]));
  lines.push(h("div.tip-title", parts.join(", "), h("span.dim", ` (${x}, ${y})${m.vis[i] === 1 ? " fogged" : ""}`)));
  const extra = [];
  // A unique improvement gets a block of its own below, with what it does.
  const imp = info("ImprovementType", m.improvement[i]);
  const unique = imp?.unique ? imp : null;
  if (m.bonus[i] >= 0) extra.push(dname("BonusType", m.bonus[i]));
  // What the resource gives (its help: yields, happiness or health, what improves it).
  const bonusHelp = m.bonus[i] >= 0 ? effectsOf("BonusType", m.bonus[i]) : "";
  if (m.improvement[i] >= 0 && !unique) extra.push(dname("ImprovementType", m.improvement[i]));
  if (m.route[i] >= 0) extra.push(dname("RouteType", m.route[i]));
  if (m.river[i] > 0 && (m.river[i] & 3)) extra.push("River");
  if (extra.length) lines.push(h("div", extra.join(" · ")));
  if (bonusHelp) lines.push(h("div.tip-effects.tip-bonus", markup(bonusHelp)));
  // What the terrain's own rules do (the cards' terrains: the Sky Falls' Crater).
  const terrainHelp = m.terrain[i] >= 0 ? effectsOf("TerrainType", m.terrain[i]) : "";
  if (terrainHelp) lines.push(h("div.tip-effects", markup(terrainHelp)));
  if (m.owner[i] >= 0) lines.push(h("div.dim", `Owner: ${playerName(m.owner[i])}`));
  const p = preview.plot;
  // C++ plot help's line (the Ring of Carcer's).
  const minLevel = p && p.x === x && p.y === y && p.min_level > 0 ? h("div.warn", `Must be level ${p.min_level} to enter`) : null;
  if (unique) {
    lines.push(
      h(
        "div.tip-unique",
        h("div.tip-unique-name", dname("ImprovementType", m.improvement[i])),
        unique.effects ? h("div.tip-effects", markup(unique.effects)) : null,
        minLevel,
      ),
    );
  } else if (minLevel) lines.push(minLevel);
  // A lair or goody hut: the odds for the selection (view/explore.rs; the full table is in the
  // plot panel and on the Explore Lair action).
  if (p && p.x === x && p.y === y && p.explore) lines.push(h("div.tip-explore", markup(exploreShort(p.explore, dname))));
  if (p && p.x === x && p.y === y && p.yields) {
    lines.push(
      look.art || isoSkin()
        ? h(
            "div.tip-yields",
            p.yields.map((v, j) => h("span.yield-n", { title: dname("YieldType", j) }, uiIcon("yield", j, 8), String(v))),
            h("span.yield-n", uiIcon("icon", "defense", 8), `${p.defense}%`),
            h("span.yield-n", uiIcon("icon", "moves", 8), String(p.move_cost)),
            h("span.sr", `Yields ${p.yields[0]} food, ${p.yields[1]} production, ${p.yields[2]} commerce · defence ${p.defense}% · move cost ${p.move_cost}`),
          )
        : h("div", `Yields ${p.yields[0]} food, ${p.yields[1]} production, ${p.yields[2]} commerce · defence ${p.defense}% · move cost ${p.move_cost}`),
    );
  }
  const city = cityAt(x, y);
  if (city) {
    const walls = city.sentinels?.length ? `, Sentinels ${city.sentinels.map((p) => (p > 0 ? `${p}%` : "breached")).join("/")}` : "";
    lines.push(h("div.tip-city", `${city.capital ? "★ " : ""}${city.name}`, h("span.dim", ` (${playerName(city.owner)}${city.size !== null && city.size !== undefined ? `, size ${city.size}` : ""}${walls})`), city.besieged ? h("span.bad", " besieged") : null));
  }
  const units = unitsAt(x, y);
  if (units.length) {
    // The plot query's units carry their strength, level and promotions (C++ plot help).
    const known = new Map((p && p.x === x && p.y === y ? (p.units ?? []) : []).map((u) => [u.id, u]));
    const shown = units.slice(0, 8).map((u0) => {
      const u = { ...u0, ...(known.get(u0.id) ?? {}) };
      const promos = (u.promotions ?? []).map((pr) => dname("PromotionType", pr));
      return h(
        "div.tip-unit",
        `${dname("UnitType", u.type)}`,
        u.strength !== undefined ? ` ${u.strength}${u.defense_strength !== undefined && u.defense_strength !== u.strength ? `/${u.defense_strength}` : ""}⚔` : "",
        h("span.dim", ` ${playerName(u.owner)}${u.level > 1 ? `, level ${u.level}` : ""}${u.hp < u.max_hp ? `, ${Math.round((100 * u.hp) / u.max_hp)}% health` : ""}`),
        promos.length ? h("div.dim.tip-promos", promos.join(", ")) : null,
        // A great person of ours holding a card (docs/CARDS.md).
        u.card !== null && u.card !== undefined ? h("div.tip-card", `Card: ${dname("TechType", u.card)}`) : null,
        // This game's armies: a recruit of ours marching to its destination.
        u.march ? h("div.tip-march", marchLine(u.march)) : null,
      );
    });
    lines.push(...shown);
    if (units.length > 8) lines.push(h("div.dim", `and ${units.length - 8} more`));
  }
  // The located messages of the latest turn here (combats, cities taken): what happened on
  // this plot while the AIs moved.
  const recent = (S.messages ?? []).filter((m) => Array.isArray(m.at) && m.at[0] === x && m.at[1] === y && m.t >= latestEventTurn());
  for (const m of recent.slice(-4)) lines.push(h(`div.tip-event.ev-${eventKind(m.text)}`, `${m.player !== null && m.player !== undefined ? `${playerName(m.player)}: ` : ""}${m.text}`));
  const c = preview.combat;
  if (c?.valid) lines.push(combatLines(c, { x, y }));
  const path = preview.path;
  if (path && S.sel) {
    if (!path.ok) lines.push(h("div.warn", "No path"));
    else {
      const bits = [`${turns(path.turns)}`];
      if (path.attack) bits.push("attack");
      if (path.declare_war_team !== null) bits.push("declares war");
      if (path.approximate) bits.push("approximate");
      lines.push(h("div.tip-path", `Move: ${bits.join(", ")} · ${mapView.pad ? "A" : mapView.touch ? "tap again" : "right-click"} to go`));
    }
  }
  mapView.showTip(lines);
}

/** The first turn whose located messages are this turn's events: observed, the turn that just
 *  ended (the status's); in a human game, also the previous turn (the others moved then). */
export function latestEventTurn() {
  const t = S.status?.turn ?? 0;
  return observing() ? t : t - 1;
}

/** Labels of CombatDetails members (C++ `setCombatPlotHelp`). */
const MODIFIERS = {
  extra_combat_percent: "strength bonus",
  plot_defense_modifier: "terrain defence",
  fortify_modifier: "fortified",
  city_defense_modifier: "city defence",
  hills_attack_modifier: "hills attack",
  hills_defense_modifier: "hills defence",
  feature_attack_modifier: "feature attack",
  feature_defense_modifier: "feature defence",
  terrain_attack_modifier: "terrain attack",
  terrain_defense_modifier: "terrain defence (unit)",
  city_attack_modifier: "city attack",
  domain_defense_modifier: "domain defence",
  city_barbarian_defense_modifier: "city defence vs barbarians",
  class_defense_modifier: "vs unit class",
  class_attack_modifier: "vs unit class",
  combat_modifier_t: "combat modifier",
  combat_modifier_a: "combat modifier",
  domain_modifier_a: "domain",
  domain_modifier_t: "domain",
  animal_combat_modifier_a: "vs animals",
  animal_combat_modifier_t: "vs animals",
  animal_combat_modifier_ta: "animal",
  animal_combat_modifier_aa: "animal",
  barbarian_combat_modifier_tb: "barbarian",
  barbarian_combat_modifier_ab: "barbarian",
  river_attack_modifier: "across a river",
  amphib_attack_modifier: "amphibious",
  kamikaze_modifier: "kamikaze",
};

/** A side's combat modifiers as `[label, percent]` rows (`partsList`). */
function modifierList(d) {
  return Object.entries(MODIFIERS)
    .filter(([k]) => d?.[k])
    .map(([k, label]) => [label, d[k]]);
}

/** The attack preview of plot `at` ({x, y}): odds, strengths, the modifiers of both sides; under
 *  engagement combat the battle first. */
export function combatLines(c, at = null) {
  const pct = (v) => `${(v / 10).toFixed(1)}%`;
  const str = (v) => (v / 100).toFixed(2);
  const att = S.units.get(c.attacker);
  const box = h("div.tip-combat");
  if (c.engagement) {
    // Engagement combat: a battle of the units that can attack from here against the defenders
    // there (`on_arrival`: of the selected units that could fight there, once they are next to it
    // and can attack; the battle is fought with whoever stands there then); the engine's estimate
    // of it comes later (the duel's odds below are the strongest pair's, a rough guide).
    const foes = `${c.defenders} defender${c.defenders === 1 ? "" : "s"}`;
    if (c.on_arrival) {
      const next = !!(at && S.sel && S.map) && plotDistance(S.map, S.sel.x, S.sel.y, at.x, at.y) === 1;
      box.append(h("div.battle", `Battle ${next ? "once they can attack" : "on arrival"}: ${c.pool} of yours could fight, against ${foes}`));
      box.append(h("div.dim.small", `${mapView?.pad ? "A" : mapView?.touch ? "Tap again" : "Right-click"}: ${next ? "attack when they can" : "go there and attack"}, in the rules' formation`));
    } else {
      box.append(h("div.battle", `Battle: ${c.pool} of yours can fight, against ${foes}`));
      box.append(h("div.dim.small", mapView?.pad ? "A: the battle plan" : mapView?.touch ? "Tap again: the battle plan" : "Right-click: the battle plan · Shift+right-click: attack in the rules' formation"));
    }
    if (c.odds === undefined || c.odds === null) {
      if (c.declare_war_team !== null && c.declare_war_team !== undefined) box.append(h("div.warn", "Attacking declares war"));
      return box;
    }
  }
  box.append(h(`div.odds${c.odds !== null && c.odds < 500 ? ".bad" : ""}`, c.odds === null ? "Air strike" : `${c.engagement ? "Duel odds (strongest pair)" : "Odds"} ${pct(c.odds)}`, c.withdraw ? h("span.dim", ` · retreat ${pct(c.withdraw)}`) : null));
  box.append(h("div", `${att ? dname("UnitType", att.type) : "Attacker"} ${str(c.attacker_strength)} vs ${dname("UnitType", c.defender_type)} ${str(c.defender_strength)}`, h("span.dim", ` (${playerName(c.defender_owner)})`)));
  // The attacker is yours: its bonuses in green, the defender's in red.
  const mods = partsList([
    { head: "Attacker", rows: modifierList(c.attacker_details) },
    { head: "Defender", rows: modifierList(c.defender_details), sense: -1 },
  ], { unit: "%" });
  if (mods) box.append(h("div.mods", mods));
  if (c.declare_war_team !== null) box.append(h("div.warn", "Attacking declares war"));
  return box;
}

// ---------------------------------------------------------------------------------------------
// The panel

function syncOverlay() {
  mapView?.setOverlay({ sel: S.sel ? { x: S.sel.x, y: S.sel.y } : null, units: S.sel?.units ?? [] });
}

/** Re-reads the selection's actions (`group` query) and redraws the panel. Until the reply for
 *  newly selected units arrives the panel shows "…" and the hotkeys wait (a reply for the same
 *  units at an older view stays shown meanwhile). */
export async function refreshGroup() {
  if (!S.sel) {
    group = null;
    groupFor = "";
    groupKey = "";
    groupPending = null;
    renderPanel();
    return;
  }
  const units = [...S.sel.units];
  const uk = unitsKey(units);
  const key = `${uk}@${S.view?.view_version}:${S.view?.turn}`;
  if (key === groupKey && (groupPending || (group && groupFor === uk))) {
    renderPanel();
    return groupPending ?? undefined;
  }
  groupKey = key;
  if (groupFor !== uk) {
    group = null;
    groupFor = "";
  }
  renderPanel();
  const pending = (async () => {
    let r;
    try {
      r = await api.query("group", { units });
    } catch (e) {
      r = { error: e.detail ?? e.message };
    }
    if (key !== groupKey) return;
    group = r;
    groupFor = uk;
    groupPending = null;
    renderPanel();
  })();
  groupPending = pending;
  return pending;
}

function actionLabel(a) {
  switch (a.kind) {
    case "mission": {
      const m = tname("MissionType", a.mission);
      if (m === "MISSION_BUILD") return dname("BuildType", a.data1);
      if (m === "MISSION_SPREAD") return `Spread ${dname("ReligionType", a.data1)}`;
      if (m === "MISSION_JOIN") return `Join as ${dname("SpecialistType", a.data1)}`;
      if (m === "MISSION_CONSTRUCT") return `Build ${dname("BuildingType", a.data1)}`;
      if (m === "MISSION_LEAD") return `Lead ${dname("UnitType", S.units.get(a.data1)?.type)}`;
      if (m === "MISSION_DISCOVER") return "Discover a technology";
      if (m === "MISSION_HURRY") return "Hurry production";
      if (m === "MISSION_TRADE") return "Trade mission";
      if (m === "MISSION_GREAT_WORK") return "Great work";
      if (m === "MISSION_GOLDEN_AGE") return "Start a golden age";
      return dname("MissionType", a.mission);
    }
    case "target":
      return humanize(a.mode.replace(/^INTERFACEMODE_/, "MODE_"));
    case "command":
      return dname("CommandType", a.command);
    case "load":
      return "Load";
    case "automate":
      return AUTOMATE_LABELS[tname("AutomateType", a.automate)] ?? dname("AutomateType", a.automate);
    case "spell":
      return a.label ?? dname("SpellType", a.spell);
    case "promotion":
      return dname("PromotionType", a.promotion);
    case "army":
      return a.label;
    case "upgrade": {
      const u = info("UnitType", a.unit);
      return `Upgrade: ${dname("UnitType", a.unit)} (${a.price} gold${u ? `; ${u.combat}/${u.combat_defense} strength, ${u.moves} moves` : ""})`;
    }
  }
  return a.kind;
}

function hotkeyOf(a) {
  return K.label(K.bindings(a, S.data)[0]);
}

const typeOf = (a) => (a.kind === "mission" ? tname("MissionType", a.mission) : a.kind === "command" || a.kind === "load" ? tname("CommandType", a.command) : "");
const GREAT_PERSON = ["MISSION_JOIN", "MISSION_CONSTRUCT", "MISSION_DISCOVER", "MISSION_HURRY", "MISSION_TRADE", "MISSION_GREAT_WORK", "MISSION_GOLDEN_AGE", "MISSION_LEAD", "MISSION_INFILTRATE"];
const RELIGION = ["MISSION_SPREAD", "MISSION_SPREAD_CORPORATION"];
const CARGO = ["COMMAND_LOAD", "COMMAND_LOAD_UNIT", "COMMAND_UNLOAD", "COMMAND_UNLOAD_ALL"];
/** The automate buttons (Civ4's TXT_KEY_ACTION_AUTOMATE_*, not in FfH's text files). */
const AUTOMATE_LABELS = {
  AUTOMATE_BUILD: "Automate: improve the land",
  AUTOMATE_NETWORK: "Automate: build the trade network",
  AUTOMATE_CITY: "Automate: improve this city",
  AUTOMATE_EXPLORE: "Explore (automated)",
  AUTOMATE_RELIGION: "Automate: spread the religion",
};

/** The actions panel's sections; each lists its actions in the action list's order
 *  (`iOrderPriority`, C++ `SetGlobalActionInfo`), but for Delete (`sectionActions`). */
const SECTIONS = [
  ["Orders", (a) => (a.kind === "mission" && ![...GREAT_PERSON, ...RELIGION, "MISSION_BUILD"].includes(typeOf(a))) || a.kind === "target" || (a.kind === "command" && !CARGO.includes(typeOf(a)))],
  ["Build", (a) => typeOf(a) === "MISSION_BUILD"],
  ["Great person", (a) => GREAT_PERSON.includes(typeOf(a))],
  ["Religion", (a) => RELIGION.includes(typeOf(a))],
  ["Cargo", (a) => a.kind === "load" || CARGO.includes(typeOf(a))],
  ["Army", (a) => a.kind === "army"],
  ["Automation", (a) => a.kind === "automate"],
  ["Spells", (a) => a.kind === "spell"],
  ["Promotions", (a) => a.kind === "promotion"],
  ["Upgrades", (a) => a.kind === "upgrade"],
];

/** Deleting units: a danger button (docs/ART.md 11.4: delete, raze, declare war). */
const isDelete = (a) => a.kind === "command" && tname("CommandType", a.command) === "COMMAND_DELETE";

/** The actions of a section (`test`) in the action list's order, except that Delete goes last:
 *  its order priority puts it first, under the pointer or the finger that meant another order. */
function sectionActions(actions, test) {
  const list = K.sortActions(actions.filter(test), S.data);
  return [...list.filter((a) => !isDelete(a)), ...list.filter(isDelete)];
}

/** An action's help text (markup): a spell's, a promotion's, a build's or a mission's. */
function helpOf(a) {
  // A spell's, promotion's and upgrade's help: the data's `effects` (the server's help texts).
  if (a.kind === "spell") {
    // Why it cannot be cast now (a card's missing needs, no valid target), and for the Discover
    // action the card's own help (what it is and gives).
    const why = !a.enabled && a.why ? `[COLOR_NEGATIVE_TEXT]${a.why}[COLOR_REVERT]` : "";
    const card = a.card !== undefined && a.card !== null ? effectsOf("TechType", a.card) : "";
    // Explore Lair: the odds for the selected units (view/explore.rs).
    const explore = a.explore ? exploreFull(a.explore, dname) : "";
    return [why, card, effectsOf("SpellType", a.spell) || a.help || "", explore].filter(Boolean).join("[NEWLINE]");
  }
  if (a.kind === "promotion") return effectsOf("PromotionType", a.promotion);
  if (a.kind === "upgrade") return effectsOf("UnitType", a.unit);
  if (a.kind === "army") return a.help ?? "";
  if (a.kind === "mission" && tname("MissionType", a.mission) === "MISSION_BUILD") return info("BuildType", a.data1)?.help ?? "";
  if (a.kind === "mission") return info("MissionType", a.mission)?.help ?? "";
  if (a.kind === "command") return info("CommandType", a.command)?.help ?? "";
  return "";
}

const actionType = (a) =>
  a.kind === "mission" ? tname("MissionType", a.mission) : a.kind === "target" ? a.mode : a.kind === "command" || a.kind === "load" ? tname("CommandType", a.command) : a.kind === "automate" ? tname("AutomateType", a.automate) : a.kind === "spell" ? tname("SpellType", a.spell) : a.kind === "promotion" ? tname("PromotionType", a.promotion) : a.kind === "upgrade" ? tname("UnitType", a.unit) : a.kind === "army" ? a.army : "";

/** The kinds the isometric card lists as labelled buttons rather than icon tiles (their icons
 *  would all look alike), with the icon each label carries. */
const LISTED = ["spell", "promotion", "upgrade", "army"];
const ARMY_ICONS = { form: "units", split: "units", join: "units", garrison: "defense", disband: "disband", upgrade: "upgrade" };
const listedIcon = (a) => (a.kind === "spell" ? "spell" : a.kind === "promotion" ? "promote" : a.kind === "army" ? (ARMY_ICONS[a.army] ?? "units") : "upgrade");

/** Whether the card lists action `a` as a labelled button: spells, promotions, upgrades and army
 *  actions, and on
 *  a touch screen every order (a finger cannot point at a tile to read what it does). */
const listedAction = (a) => LISTED.includes(a.kind) || coarse();

function actionButton(a) {
  const key = hotkeyOf(a);
  const help = LISTED.includes(a.kind) ? helpOf(a) : "";
  const extra = a.turns !== undefined ? ` (${turns(a.turns)})` : "";
  if (isoSkin()) return isoAction(a, key, extra);
  const btn = h(
    `button.action${isDelete(a) ? ".danger" : ""}`,
    {
      disabled: !a.enabled,
      "data-kind": a.kind,
      "data-type": actionType(a),
      // The help as a rich tooltip (its bullets and colours kept), else the key as the title.
      title: help ? null : key ? `[${key}]` : null,
      tip: help ? [h("div.tip-title", actionLabel(a), key ? h("kbd", key) : null), h("div.tip-effects", markup(help))] : null,
      onclick: () => doAction(a),
    },
    a.kind === "promotion" ? promotionIcon(a.promotion, { u: 1 }) : null,
    actionLabel(a) + extra,
    key ? h("kbd", key) : null,
  );
  return btn;
}

/** The isometric card's action: an icon tile (or, for spells, promotions and upgrades, a
 *  labelled button) whose description (name, key and help) the card's description box shows
 *  while it is pointed at or focused. A labelled button's help (what the spell, the promotion, the
 *  unit upgraded to does: many lines) is a rich tooltip instead: in the box, which is as large as
 *  its largest description, it would push the labelled buttons out of the card. */
function isoAction(a, key, extra) {
  const listed = listedAction(a);
  const label = actionLabel(a) + extra;
  const help = helpOf(a);
  // A finger cannot point at an order to read it: on a touch screen the orders say what they are.
  if (!coarse()) descs.set(a, description(a, label, key, listed ? "" : help));
  const show = () => describe(a);
  return h(
    `button.action${listed ? ".listed" : ".tile"}${isDelete(a) ? ".danger" : ""}`,
    {
      disabled: !a.enabled,
      "data-kind": a.kind,
      "data-type": actionType(a),
      title: listed && help ? null : `${label}${key ? ` [${key}]` : ""}`,
      tip: listed && help ? [h("div.tip-title", label, key ? h("kbd", key) : null), h("div.tip-effects", markup(help))] : null,
      "aria-label": label,
      onclick: () => doAction(a),
      onmouseenter: show,
      onfocus: show,
      onmouseleave: () => describe(null),
      onblur: () => describe(null),
    },
    !listed ? actionIcon(a, 40) : LISTED.includes(a.kind) ? isoIcon(listedIcon(a), { px: 16 }) : actionIcon(a, 32),
    listed ? h("span", label) : null,
    !listed && key ? h("kbd", key) : null,
  );
}

/** The card's description box shows the action pointed at (its name, key, turns and help), else
 *  the selection's state (its first child). It holds them all from the start, stacked in one
 *  cell (iso.css), so it is as large as the largest and pointing at an order moves nothing in
 *  the card. `descs`: each action's description, made by `isoAction` as the card is built. */
const descs = new Map();

function description(a, label, key, help) {
  return h(
    "div.iso-desc-one",
    h("div.iso-desc-title", label, key ? h("kbd", key) : null, a.enabled ? null : h("span.dim", " (not now)")),
    help ? h("div.iso-desc-text", markup(help)) : null,
  );
}

/** Shows action `a`'s description in the card's box (null: the selection's state). */
function describe(a) {
  const box = panel?.querySelector(".iso-desc");
  if (!box) return;
  const shown = (a && descs.get(a)) || box.firstElementChild;
  for (const c of box.children) c.classList.toggle("shown", c === shown);
}

/** A great person's card (docs/CARDS.md) as a badge: its name, its help in the tooltip. */
function cardBadge(u) {
  if (u.card === null || u.card === undefined) return null;
  return h("span.badge.card", { tip: infoTip("TechType", u.card, { title: `Card: ${dname("TechType", u.card)}` }) }, `card: ${dname("TechType", u.card)}`);
}

/** The card the lead unit holds, for the panel (the `group` query's `card_detail`): its name, suit
 *  and band, its needs with the ones the team knows, and whether it can be discovered now (else
 *  why not); its help in the tooltip. Null without a card. */
function cardLine(u) {
  const c = u?.card_detail;
  if (!c) return null;
  const need = (n) => `${dname("TechType", n.tech)} (${n.known ? "known" : "not known"})`;
  const needs = [
    c.needs?.length ? c.needs.map(need).join(", ") : null,
    c.needs_one_of?.length ? `one of ${c.needs_one_of.map(need).join(", ")}` : null,
  ].filter(Boolean);
  const e = info("TechType", c.tech);
  const suit = c.suit !== null && c.suit !== undefined ? dname("UnitClassType", c.suit).replace(/^Great /, "") : null;
  const band = c.band !== null && c.band !== undefined ? ["I", "II", "III"][Math.min(2, c.band)] : null;
  // The needs line says what is missing; another reason (the team knows the card) gets a line.
  const missing = (c.needs ?? []).some((n) => !n.known) || ((c.needs_one_of ?? []).length > 0 && !c.needs_one_of.some((n) => n.known));
  return h(
    "div.card-line",
    { tip: infoTip("TechType", c.tech, { title: `Card: ${dname("TechType", c.tech)}`, hint: c.discoverable ? "Discover it with the great person's Discover action." : c.why }) },
    h("b", `Card: ${dname("TechType", c.tech)}`),
    suit || band ? h("span.dim", ` (${[suit, band ? `band ${band}` : null].filter(Boolean).join(", ")})`) : null,
    needs.length ? h(`div.small${missing ? ".warn" : ""}`, `Needs: ${needs.join("; ")}`) : null,
    c.discoverable ? h("div.small.good", "Can be discovered now.") : !missing && c.why ? h("div.small.warn", c.why) : null,
    e?.help && !isoSkin() ? h("div.small.dim", markup(e.help)) : null,
  );
}

/** "automated" (the AI moves the unit's group: Stop automation gives it back) or "AI control"
 *  (an FfH AI-controlled unit, e.g. enraged), else null. */
function automationBadge(u) {
  const grp = u.group !== null && u.group !== undefined ? S.groups.get(u.group) : null;
  if (!grp) return null;
  if (grp.automated !== null && grp.automated !== undefined) {
    const label = AUTOMATE_LABELS[tname("AutomateType", grp.automated)] ?? "Automated";
    return h("span.badge.auto", { title: `${label}: the AI moves this unit (Stop automation to take it back)` }, "automated");
  }
  return grp.ai_control ? h("span.badge.auto", { title: "AI-controlled (FfH): the AI moves this unit" }, "AI control") : null;
}

/** This game's armies (docs/ARMIES.md): the army in the view's `armies`, or null. */
export function armyById(id) {
  return id === null || id === undefined ? null : (S.view?.armies ?? []).find((a) => a.id === id) ?? null;
}

/** The unit's army (its name; out of supply in warning colours), "garrison", or "marching" (a
 *  recruit on its way, which takes no orders); null without armies or for a unit outside them
 *  (heroes, mages, recon units, summons, ships, workers). */
function armyBadge(u) {
  if (u.march) return h("span.badge.march", { title: `${marchLine(u.march)}: it takes no orders until it arrives` }, "marching");
  const a = armyById(u.army);
  if (a) return h(`span.badge.army${a.in_supply ? "" : ".warn"}`, { title: `${a.name}${a.in_supply ? "" : ": no supply line (no healing, its recruits wait)"}` }, a.name);
  if (u.garrison !== null && u.garrison !== undefined) return h("span.badge.garrison", { title: `In the garrison of ${S.cities.get(u.garrison)?.name ?? "the city"}` }, "garrison");
  return null;
}

/** Engagement combat (docs/ENGAGEMENTS.md): the row the unit prefers and what it does there as a
 *  badge, its A, Dv and battle HP in the tooltip; null without engagements (or a sea or air unit). */
function roleBadge(u) {
  const row = preferredRow(u);
  if (!row) return null;
  return h("span.badge.role", { title: engageText(u) }, row);
}

/** "Front: 2 strikes · A 4 · Dv 5 · HP 55/120 · speed 4". */
const engageText = (u) => `${preferredRow(u)} · ${engageNumbers(u)}`;

const ENGAGE_TITLE =
  "In a battle: the row it stands in decides what it does and how many times (front, middle, rear: its own rows); the auto formation puts it in the row it prefers first. ATK and DEF: its attack and defence; MAG and MDEF: its magic (spells) and magic defence; ACC: its chance to hit, less the target's EVA (evasion); CRIT: its crit chance; HP: its health in a battle; speed: when it acts (the fastest first)";

/** The lead unit's battle numbers (engagement combat): the row it prefers and what it does there,
 *  A, Dv and HP; null without engagements (or for a sea or air unit). */
function engageLine(u) {
  const row = u ? preferredRow(u) : "";
  if (!row) return null;
  return h("div.engage-line", { title: ENGAGE_TITLE }, h("b", row), h("span.dim", ` · ${engageNumbers(u)}`));
}

/** The selection's army in one line: its size (and target), its recruits marching to it and in
 *  training for it, its supply; the Armies screen [F5] has the
 *  rest. Null unless every selected unit is in one army. */
function armyLine() {
  const ids = (S.sel?.units ?? []).map((id) => S.units.get(id)?.army);
  const a = ids.length && ids.every((x) => x === ids[0]) ? armyById(ids[0]) : null;
  if (!a) return null;
  const n = a.units.length;
  const training = (a.requisition?.orders?.length ?? 0) + (a.requisition?.head_orders ?? 0);
  return h(
    "div.army-line",
    h("b", a.name),
    h("span.dim", ` · ${n} unit${n === 1 ? "" : "s"}${a.target_size ? ` of ${a.target_size}` : ""}`),
    a.marching.length ? h("span.dim", { title: "Recruits marching to it" }, ` · ${a.marching.length} marching to it`) : null,
    training ? h("span.dim", { title: "Units cities train for it: its requisition's, and those they build next for it" }, ` · ${training} in training`) : null,
    a.in_supply ? null : h("span.warn", { title: "No supply line: no healing, its recruits wait" }, " · out of supply"),
    a.disband_to ? h("span.dim", ` · disbanding to ${a.disband_to}`) : null,
    " ",
    h("button.link.small", { onclick: () => emit("open-screen", "armies"), title: "The Armies screen [F5]" }, "armies"),
  );
}

/** The selection's army (its id) when the selection is all of one army's units, else null: an
 *  attack of the selection is then the army's (`engage {army}`). */
function selectionArmy(sel) {
  const ids = (sel?.units ?? []).map((id) => S.units.get(id)?.army);
  const a = ids.length && ids.every((x) => x === ids[0]) ? armyById(ids[0]) : null;
  return a && a.units.length === sel.units.length && a.units.every((u) => sel.units.includes(u)) ? a.id : null;
}

/** Engagement combat: whether (x, y) is a plot next to the selection holding units of others,
 *  where the selection's attack would be a battle (`battleTerrain`: land units on land): an attack
 *  there may be one. */
function attackPlot(x, y) {
  if (!S.sel || !S.map || !battleTerrain(S.sel.units, x, y)) return false;
  if (plotDistance(S.map, S.sel.x, S.sel.y, x, y) !== 1) return false;
  return unitsAt(x, y).some((u) => u.owner !== me()?.player);
}

/** Engagement combat: the `combat` query's answer for the selection on (x, y) when moving there
 *  is a battle the selection can fight now (`engagement`, `valid` and not `on_arrival`: it meets
 *  defenders it is at war with or would declare war on, and some unit can attack now), else
 *  null. Asks only for a plot `attackPlot` admits. */
async function battleNow(x, y) {
  if (!attackPlot(x, y)) return null;
  let c;
  try {
    c = await api.query("combat", { units: [...S.sel.units], x, y });
  } catch {
    return null;
  }
  return c?.engagement && c.valid && !c.on_arrival ? c : null;
}

/** Engagement combat (docs/ENGAGEMENTS.md): when moving the selection onto (x, y) attacks it with
 *  a battle and the force is a choice (an army, or several units that can attack there), opens
 *  the battle plan with the rules' auto formation (Attack sends `engage` with the formation) and returns true;
 *  else false (the move goes as it is: a lone unit's attack, a duel, no defender). */
async function planAttack(x, y) {
  if (S.status?.phase !== "human") return false;
  const sel = { units: [...S.sel.units] };
  const c = await battleNow(x, y);
  if (!c) return false;
  const army = selectionArmy(sel);
  if (army === null && (c.pool ?? 0) < 2) return false;
  mapView?.clearHover?.();
  await openPlan(army !== null ? { army } : { units: sel.units }, { x, y });
  return true;
}

/** A click on a unit of the stack (a row of the list, an icon of the card's strip): its group;
 *  with Ctrl, that unit added to the selection or taken out of it. */
function pickUnit(u, ev) {
  if (ev.ctrlKey || ev.metaKey) {
    const set = new Set(S.sel?.units ?? []);
    set.has(u.id) ? set.delete(u.id) : set.add(u.id);
    select([...set]);
  } else {
    selectGroup(u.group);
  }
}

/** What a unit is doing as the strip's corner mark shows it (the group's activity). */
const ACTIVITY_ICONS = { ACTIVITY_SLEEP: "sleep", ACTIVITY_HEAL: "heal", ACTIVITY_SENTRY: "sentry", ACTIVITY_INTERCEPT: "sentry", ACTIVITY_PATROL: "sentry", ACTIVITY_MISSION: "goto", ACTIVITY_PLUNDER: "plunder" };

/** The unit's activity when it is not awake (sleeping, fortified, healing, ...), else null. */
function activityOf(u) {
  const a = u.activity !== null && u.activity !== undefined ? tname("ActivityType", u.activity) : null;
  return a && a !== "ACTIVITY_AWAKE" ? a : null;
}

/** Whether the unit's group is automated or under AI control. */
function automated(u) {
  const grp = u.group !== null && u.group !== undefined ? S.groups.get(u.group) : null;
  return !!grp && ((grp.automated !== null && grp.automated !== undefined) || !!grp.ai_control);
}

/** One unit of the isometric card's strip (Civ4's unit buttons over its bottom panel): the unit's
 *  figure, its health under it, a pip for its moves (green: all left, amber: some, dark red: none)
 *  and a corner mark (a promotion waiting, else marching, automated or what it is doing); lit when
 *  selected, brightest for the unit the card describes. The rest is in its tooltip. */
function unitChip(u, selected, lead) {
  const name = u.name ?? dname("UnitType", u.type);
  const act = activityOf(u);
  const mark = u.promo_ready
    ? isoIcon("promote", { px: 16, cls: "chip-mark" })
    : u.march
      ? h("span.chip-mark.march", "»")
      : automated(u)
        ? isoIcon("automate", { px: 16, cls: "chip-mark" })
        : act && ACTIVITY_ICONS[act]
          ? isoIcon(ACTIVITY_ICONS[act], { px: 16, cls: "chip-mark" })
          : null;
  const mv = u.moves === undefined || u.march ? null : u.moves <= 0 ? "spent" : u.moves < u.max_moves ? "part" : "full";
  const frac = u.hp / Math.max(1, u.max_hp);
  return h(
    "button.unit-chip",
    {
      "data-unit": u.id,
      class: [selected ? "selected" : "", lead ? "lead" : "", u.transport !== null && u.transport !== undefined ? "cargo" : ""].filter(Boolean).join(" "),
      "aria-label": name,
      "aria-pressed": selected ? "true" : "false",
      tip: () => chipTip(u),
      onclick: (ev) => pickUnit(u, ev),
    },
    unitFigure(u.type, u.owner, 34) ?? h("span.chip-abbr", name.slice(0, 2)),
    mv ? h(`span.chip-moves.${mv}`) : null,
    mark,
    u.cargo?.length ? h("span.chip-count", { title: "Units on board" }, String(u.cargo.length)) : null,
    h("span.chip-hp", h(`span${frac * 4 < 1 ? ".low" : frac < 1 ? ".hurt" : ""}`, { style: { width: `${Math.round(100 * Math.max(0, Math.min(1, frac)))}%` } })),
  );
}

/** A strip unit's tooltip: its name, numbers, state and what a click does. */
function chipTip(u) {
  const act = activityOf(u);
  const numbers = [
    u.strength !== undefined ? `${u.strength} strength` : null,
    `${Math.round((100 * u.hp) / Math.max(1, u.max_hp))}% health${u.besieged ? " (besieged: no heal from the city)" : ""}`,
    u.moves !== undefined ? `${moves(u.moves)}/${moves(u.max_moves)} moves` : null,
    u.level !== undefined ? `level ${u.level}${u.xp !== undefined && u.xp_needed ? ` (${u.xp}/${u.xp_needed} xp)` : ""}` : null,
  ].filter(Boolean);
  const badges = [
    u.promo_ready ? h("span.badge.promo", "promote") : null,
    automationBadge(u),
    armyBadge(u),
    u.summon_turns ? h("span.badge", `${u.summon_turns} turns left`) : null,
    u.cargo?.length ? h("span.badge", `${u.cargo.length} aboard`) : null,
    u.transport !== null && u.transport !== undefined ? h("span.badge", "aboard") : null,
    act ? h("span.badge", humanize(act)) : null,
    cardBadge(u),
  ].filter(Boolean);
  return [
    h("div.tip-title", u.name ?? dname("UnitType", u.type)),
    h("div.tip-stats", numbers.join(" · ")),
    preferredRow(u) ? h("div.tip-stats", engageText(u)) : null,
    badges.length ? h("div.chip-badges", badges) : null,
    h("div.tip-hint", touchy() ? "Tap: select its group." : "Click: select its group. Ctrl-click: add or remove this unit."),
  ];
}

function unitRow(u, selected) {
  const busy = u.activity !== null && u.activity !== undefined && tname("ActivityType", u.activity) !== "ACTIVITY_AWAKE";
  const row = h(
    "div.unit-row",
    {
      "data-unit": u.id,
      class: [selected ? "selected" : "", u.transport !== null && u.transport !== undefined ? "cargo" : ""].filter(Boolean).join(" "),
      title: "Click: select its group. Ctrl-click: add or remove this unit.",
      onclick: (ev) => pickUnit(u, ev),
    },
    h(
      "span.unit-name",
      u.transport !== null && u.transport !== undefined ? h("span.dim", "↳ ") : null,
      u.unit_combat !== undefined && u.unit_combat !== null && !isoSkin() ? typeIcon("unitcombat", u.unit_combat, 8, { title: dname("UnitCombatType", u.unit_combat) }) : null,
      u.name ?? dname("UnitType", u.type),
      u.promo_ready ? h("span.badge.promo", "promote") : null,
      automationBadge(u),
      armyBadge(u),
      roleBadge(u),
      u.summon_turns ? h("span.badge", { title: "A summoned unit: turns left" }, `${u.summon_turns}t`) : null,
      u.cargo?.length ? h("span.badge", { title: "Units on board" }, `${u.cargo.length} aboard`) : null,
      cardBadge(u),
    ),
    look.art || isoSkin()
      ? h(
          "span.unit-stats",
          u.strength !== undefined ? h("span.stat-n", pix(String(u.strength)), icon8("strength", "⚔", "Strength")) : null,
          barEl(u.hp / Math.max(1, u.max_hp), u.hp * 4 < u.max_hp ? "hp_low" : "hp", { w: 14, title: `${Math.round((100 * u.hp) / u.max_hp)}% health` }),
          u.moves !== undefined ? h("span.stat-n", pix(`${moves(u.moves)}/${moves(u.max_moves)}`), icon8("moves", "➜", "Moves")) : null,
          u.level !== undefined ? h("span.stat-n", { title: `Level ${u.level}` }, pix(`L${u.level}`)) : null,
          busy ? h("span.dim", ` ${humanize(tname("ActivityType", u.activity))}`) : null,
        )
      : h(
          "span.unit-stats",
          `${u.strength ?? ""}${u.strength !== undefined ? "⚔ " : ""}`,
          `${Math.round((100 * u.hp) / u.max_hp)}% `,
          u.moves !== undefined ? `${moves(u.moves)}/${moves(u.max_moves)}➜ ` : "",
          u.level !== undefined ? `L${u.level}` : "",
          busy ? h("span.dim", ` ${humanize(tname("ActivityType", u.activity))}`) : null,
        ),
  );
  return row;
}

/** Whether the hints speak of taps: the last pointer on the map was a finger, or the device's
 *  main pointer is one. */
function touchy() {
  return !!mapView?.touch || coarse();
}

/** The side panel for the selection. */
export function renderPanel() {
  if (!panel) return;
  // A world spell's target picked on the map, with no selection: the pick's hint.
  if (!S.sel && mode?.world) {
    delete panel.dataset.plot;
    fill(panel, h("div.mode-hint", h("span", `Pick a target: ${actionLabel(mode.action)} (Esc cancels).`), h("button.small", { onclick: () => setMode(null), title: "Leave the pick [Esc]" }, "Cancel")));
    return;
  }
  if (!S.sel) {
    // The isometric map's card shows only with a selection or a plot (the map stays clear).
    if (!panel.dataset.plot) {
      const hint = observing()
        ? `${touchy() ? "Tap" : "Click"} a plot to see its stack (the AI's role and orders of each unit) and this turn's fights there; ⚑ flags it for a bug report.`
        : mapView?.pad
          ? "A: select the unit or open the city under the cursor. With units selected, the cursor shows the way; A goes. Y: the orders."
          : touchy()
          ? "Tap a unit or a city. With units selected, tap a plot to see the way there; tap it again to go."
          : "Select a unit or a city. Right-click moves the selected units.";
      fill(panel, isoSkin() ? [] : h("p.dim", hint));
    }
    return;
  }
  delete panel.dataset.plot;
  const { x, y } = S.sel;
  const own = ownUnitsAt(x, y);
  // Only a reply for the selected units: the previous selection's actions are not drawn under
  // the new selection's header while its reply is on its way.
  const r = reply();
  const details = new Map((r?.units ?? []).map((u) => [u.id, u]));
  const foreign = unitsAt(x, y).filter((u) => u.owner !== me()?.player);
  const g = currentGroup();
  const grp = g !== null ? S.groups.get(g) : null;
  const lead = S.units.get(S.sel.units[0]) ?? own[0];
  const head = h(
    "div.panel-head",
    lead ? h("span.unit-head", { tip: infoTip("UnitType", lead.type) }, unitIcon(lead.type, ownerLook(lead.owner))) : null,
    h("h3", own.length === 1 ? dname("UnitType", own[0].type) : `${S.sel.units.length} of ${own.length} units`),
    h("span.dim", ` (${x}, ${y})`),
    h("button.close.deselect", { title: "Deselect [Esc]", "aria-label": "Deselect", onclick: () => clearSelection() }, "×"),
  );
  // Transports with their cargo under them (the units in a transport are listed after it).
  const ids = new Set(own.map((u) => u.id));
  const ordered = [];
  for (const u of own.filter((u) => u.transport === null || u.transport === undefined || !ids.has(u.transport))) {
    ordered.push(u);
    for (const c of own.filter((c) => c.transport === u.id)) ordered.push(c);
  }
  const list = h("div.unit-list", ordered.map((u) => unitRow({ ...u, ...(details.get(u.id) ?? {}) }, S.sel.units.includes(u.id))));
  const march = selectionMarch();
  const tools = h(
    "div.row",
    h("button", { onclick: () => select(allHere(own)), disabled: orderable(own).length < 2, title: "Select every unit here that takes orders (not a marching recruit)" }, "All"),
    h("button", { onclick: groupAll, disabled: orderable(own).length < 2, title: "Join the units here into one group (not a marching recruit)" }, "Group"),
    h("button", { onclick: splitAll, disabled: S.sel.units.filter((id) => !S.units.get(id)?.march).length < 2, title: "Each selected unit its own group" }, "Split"),
    h("button", { onclick: () => selectNext(), title: "Next unit waiting for orders [Tab]" }, "Next"),
  );
  if (r?.actions?.some((a) => a.kind === "promotion" && a.enabled)) {
    tools.append(h("button.promote", { onclick: () => openPromotions(), title: "Choose a promotion" }, "Promote…"));
  }
  const city = cityAt(x, y);
  if (city && city.owner === me()?.player) {
    tools.append(h("button", { onclick: () => emit("open-city", city.id), title: "Open the city (or double-click it)" }, `Open ${city.name}`));
  }
  const status = [];
  // This game's armies: a marching recruit goes on its own and takes no orders until it arrives.
  if (march) {
    status.push(
      h(
        "div.march-line",
        { title: "The rules march it there; it takes no orders until it arrives." },
        h("b", marchLine(march)),
        march.from ? h("span.dim", ` · from ${march.from}`) : null,
      ),
    );
  }
  // Engagement combat: the lead unit's role, A, Dv and battle HP.
  const engage = engageLine(lead ? { ...lead, ...(details.get(lead.id) ?? {}) } : null);
  if (engage && !isoSkin()) status.push(engage);
  const army = armyLine();
  if (army) status.push(army);
  // A great person's card (docs/CARDS.md).
  const card = cardLine(lead ? { ...lead, ...(details.get(lead.id) ?? {}) } : null);
  if (card) status.push(card);
  if (grp?.missions?.length) status.push(h("div.dim", `Orders: ${grp.missions.map((m) => dname("MissionType", m.m)).join(", ")}`));
  if (grp && !grp.ready && !march) status.push(h("div.dim", grp.busy ? "Busy" : "Done for this turn"));
  const actions = [];
  if (r?.error) actions.push(h("p.warn", r.error));
  else if (!r?.actions) actions.push(h("p.dim", "…"));
  else {
    for (const [title, test] of SECTIONS) {
      const list = sectionActions(r.actions, test);
      if (!list.length) continue;
      const more = title === "Spells" ? h("button.link.small", { onclick: () => openSpells(), title: "Every spell with its description" }, "details") : null;
      actions.push(h("div.section", h("h4", title, more ? " " : null, more), h("div.actions", list.map(actionButton))));
    }
  }
  const others = foreign.length ? h("div.section", h("h4", "Others here"), foreign.map((u) => h("div.dim", `${dname("UnitType", u.type)} (${ownerText(u)})`))) : null;
  const what = mode?.cast ? "a target" : "a plot";
  const pick = mapView?.pad ? (isGoTo(mode?.action) ? "Move the cursor to the destination and press A (B cancels)." : `Move the cursor to ${what} and press A: ${mode && actionLabel(mode.action)} (B cancels).`) : touchy() ? (isGoTo(mode?.action) ? "Tap a plot to go there." : `Tap ${what}: ${mode && actionLabel(mode.action)}.`) : isGoTo(mode?.action) ? "Click a plot to go there (Shift: add waypoints; Esc cancels)." : `Pick ${what}: ${mode && actionLabel(mode.action)} (Esc cancels).`;
  // One hint at a time: waypoints being laid say more than the go-to mode they are laid in.
  const way = waypoints && unitsKey(waypoints.units) === unitsKey(S.sel.units)
    ? h("p.mode-hint", `${waypoints.points.length === 1 ? "1 waypoint" : `${waypoints.points.length} waypoints`}: release Shift to send (Esc drops them).`)
    : null;
  const hint = mode && !way ? h("div.mode-hint", h("span", pick), h("button.small", { onclick: () => setMode(null), title: "Leave the plot pick [Esc]" }, "Cancel")) : null;
  // The lead unit's promotions as their seals (docs/ART.md 11.5), with the level as pips.
  const leadDetails = details.get(S.sel.units[0]);
  const seals = look.art && leadDetails?.promotions?.length
    ? h("div.seals", leadDetails.promotions.map((p) => h("span.seal", { tip: infoTip("PromotionType", p) }, promotionIcon(p, { u: isoSkin() ? 2 : 1 }))))
    : null;
  if (isoSkin()) return renderCard({ own, ordered, r, details, lead, promotions: leadDetails?.promotions, tools, status, hint, way, others, x, y });
  fill(panel, head, hint, way, list, seals, tools, status, actions, others);
}

/** The selection as the isometric map's card (the reference's unit panel), kept low so the map
 *  keeps the screen: the lead unit's portrait; its name and its numbers two by two with its
 *  promotions' seals under them; the stack's buttons over its orders as icon tiles, over a
 *  description of the one pointed at (spells, promotions and upgrades as labelled buttons under
 *  them). The units of the plot are a strip of icons over the card (Civ4's), the selected ones
 *  lit; on a small screen the strip stays in the card, under the numbers (iso.css). */
function renderCard({ own, ordered, r, details, lead, promotions, tools, status, hint, way, others, x, y }) {
  const city = cityAt(x, y);
  if (city && city.owner === me()?.player) focusOn(city.id);
  const d = lead ? { ...lead, ...(details.get(lead.id) ?? {}) } : null;
  // Each number by its icon; the label (shown on a wide screen only where there is room) is the
  // line's tooltip as well.
  const stat = (icon, label, value, title, cls = "") =>
    h(`div.iso-statline${cls}`, { title: title ?? label }, isoIcon(icon, { px: 32 }), h("span.label", label), value);
  const plate = d
    ? h(
        "div.iso-plate",
        d.strength !== undefined ? stat("strength", "Strength", h("span.value", String(d.strength))) : null,
        d.moves !== undefined ? stat("moves", "Movement", h("span.value", `${moves(d.moves)}/${moves(d.max_moves)}`)) : null,
        d.max_hp ? stat("hp", "Health", isoBar(d.hp / Math.max(1, d.max_hp), { w: 64, kind: d.hp * 4 < d.max_hp ? "low" : "hp" }), `Health: ${Math.round((100 * d.hp) / d.max_hp)}%`) : null,
        d.level !== undefined ? stat("xp", "Level", h("span.value", `${d.level}${d.xp !== undefined && d.xp_needed ? ` (${d.xp}/${d.xp_needed})` : ""}`), d.xp !== undefined && d.xp_needed ? `Level ${d.level}: ${d.xp} of the ${d.xp_needed} experience the next promotion needs` : null) : null,
        // Engagement combat: the row it prefers and what it does there, A, Dv and battle HP (the
        // rest in the tooltip).
        preferredRow(d)
          ? stat("strength", "Battle", h("span.value.engage", `${preferredRow(d)} · ${engageBrief(d)}`), `${engageText(d)}\n\n${ENGAGE_TITLE}`, ".engage-stat")
          : null,
      )
    : null;
  // The lead unit's promotions as their seals (docs/ART.md 11.5).
  const seals = look.art && promotions?.length ? h("div.seals", promotions.map((p) => h("span.seal", { tip: infoTip("PromotionType", p) }, promotionIcon(p, { u: 2 })))) : null;
  // The plot's units, transports with their cargo after them.
  const strip =
    own.length > 1
      ? h(
          "div.iso-strip",
          { role: "group", "aria-label": "The units here" },
          ordered.map((u) => unitChip({ ...u, ...(details.get(u.id) ?? {}) }, S.sel.units.includes(u.id), u.id === S.sel.units[0])),
        )
      : null;
  const tiles = [];
  const listed = [];
  descs.clear();
  if (r?.error) tiles.push(h("p.warn", r.error));
  else if (!r?.actions) tiles.push(h("p.dim", "…"));
  else {
    // The tiles run on without headings: Delete's tile ends the row (after the automation's).
    const del = r.actions.filter((a) => isDelete(a) && !listedAction(a));
    for (const [title, test] of SECTIONS) {
      const acts = sectionActions(r.actions, test);
      if (!acts.length) continue;
      const tl = acts.filter((a) => !listedAction(a) && !del.includes(a));
      const ls = acts.filter((a) => listedAction(a));
      if (tl.length) tiles.push(h("div.iso-tiles", { "data-section": title, title: null }, tl.map(actionButton)));
      if (ls.length) {
        const more = title === "Spells" ? h("button.link.small", { onclick: () => openSpells(), title: "Every spell with its description" }, "details") : null;
        listed.push(h("div.iso-listed", h("h4", title, more ? " " : null, more), h("div.actions", ls.map(actionButton))));
      }
    }
    if (del.length) tiles.push(h("div.iso-tiles", { "data-section": "Delete", title: null }, del.map(actionButton)));
  }
  let state = [hint, way, ...[status].flat()].filter(Boolean);
  // On a touch screen the box holds only the selection's state (a plot pick, done for the turn).
  const touch = coarse();
  if (!state.length && !touch) state = [h("div.iso-desc-text.dim", "Point at an order to read it. Right-click a plot to move there.")];
  const desc = state.length ? h("div.iso-desc", h("div.iso-desc-one.shown", state), [...descs.values()]) : null;
  const name = own.length === 1 ? dname("UnitType", own[0].type) : `${S.sel.units.length} of ${own.length} units`;
  fill(
    panel,
    h(
      "div.iso-card",
      lead ? portrait(lead.type, lead.owner) : null,
      h("div.iso-card-mid", h("h3.iso-name", name, h("span.dim.coords", ` (${x}, ${y})`), h("button.close.deselect", { title: "Deselect [Esc]", "aria-label": "Deselect", onclick: () => clearSelection() }, "×")), plate, seals, strip, others),
      h("div.iso-card-right", h("div.iso-tools", tools), tiles.length ? h("div.iso-orders", tiles) : null, desc, listed.length ? h("div.iso-listed-all", listed) : null),
    ),
  );
  // The strip stands over the card: the map's clear part (isoui.js) is measured with it.
  updateInsets();
}

/** The promotions the selection can take, with their descriptions: a click promotes (every
 *  selected unit that can, as `CvNetDoCommand` with COMMAND_PROMOTION). */
export function openPromotions() {
  const r = reply();
  const list = K.sortActions((r?.actions ?? []).filter((a) => a.kind === "promotion"), S.data);
  const m = openModal({
    title: "Promotions",
    tag: "promotions",
    wide: true,
    body: [
      list.length
        ? h(
            "div.options",
            list.map((a) =>
              h(
                "button.option",
                { disabled: !a.enabled, "data-promotion": a.promotion, onclick: async () => (await doAction(a), m.close("promoted")) },
                h("span", dname("PromotionType", a.promotion)),
                effectsOf("PromotionType", a.promotion) ? h("small", markup(effectsOf("PromotionType", a.promotion))) : null,
              ),
            ),
          )
        : h("p.dim", "No promotion now."),
    ],
    buttons: [{ label: "Close", value: null }],
  });
  return m;
}

/** The selection's spells with their help (the spell's XML help, as the button's tooltip has
 *  it): castable ones cast on a click. */
export function openSpells() {
  const r = reply();
  const list = K.sortActions((r?.actions ?? []).filter((a) => a.kind === "spell"), S.data);
  const m = openModal({
    title: "Spells",
    tag: "spells",
    wide: true,
    body: [
      h(
        "div.options",
        list.map((a) => {
          const e = info("SpellType", a.spell);
          const notes = [a.global ? "world spell (once per game)" : null, a.causes_war ? "may declare war" : null, e?.range ? `range ${e.range}` : null].filter(Boolean).join(", ");
          return h(
            "button.option",
            { disabled: !a.enabled, "data-spell": a.spell, onclick: async () => (m.close("cast"), await doAction(a)) },
            h("span", dname("SpellType", a.spell), notes ? h("span.dim", ` (${notes})`) : null, a.enabled ? null : h("span.dim", " — not now")),
            helpOf(a) ? h("small", markup(helpOf(a))) : null,
          );
        }),
      ),
    ],
    buttons: [{ label: "Close", value: null }],
  });
  return m;
}

/** The side panel for a plot without units of ours. */
/** A unit's owner for people; a crusade Host's unit also names who lent it (docs/CRUSADE.md). */
function ownerText(u) {
  const lent = u.lent_by !== null && u.lent_by !== undefined ? `, lent by ${u.lent_by === me()?.player ? "you" : playerName(u.lent_by)}` : "";
  return `${playerName(u.owner)}${lent}`;
}

async function renderPlotInfo(x, y, selUnits = S.sel?.units ?? null) {
  panel.dataset.plot = `${x},${y}`;
  fill(panel, h("div.panel-head", h("h3", `Plot (${x}, ${y})`)), h("p.dim", "…"));
  let p;
  try {
    p = await api.query("plot", selUnits?.length ? { x, y, units: selUnits } : { x, y });
  } catch (e) {
    return fill(panel, h("p.warn", e.detail ?? e.message));
  }
  if (panel.dataset.plot !== `${x},${y}`) return;
  if (p.vis === 0) return fill(panel, h("div.panel-head", h("h3", `Plot (${x}, ${y})`)), h("p.dim", "Unexplored."));
  const rows = [
    ["Terrain", [dname("TerrainType", p.terrain), p.hills ? "Hills" : null, p.feature !== null ? dname("FeatureType", p.feature) : null].filter(Boolean).join(", ")],
    ["Bonus", p.bonus !== null ? dname("BonusType", p.bonus) : null],
    ["Improvement", p.improvement !== null ? dname("ImprovementType", p.improvement) : null],
    ["Route", p.route !== null ? dname("RouteType", p.route) : null],
    ["Owner", p.owner !== null ? playerName(p.owner) : null],
    ["City", p.city ? `${p.city.name} (${playerName(p.city.owner)})` : null],
    ["Yields", p.yields ? `${p.yields[0]} / ${p.yields[1]} / ${p.yields[2]}` : null],
    ["Defence", p.defense !== undefined ? `${p.defense}%` : null],
    ["Move cost", p.move_cost],
  ].filter(([, v]) => v !== null && v !== undefined);
  const list = p.units ?? [];
  const role = (u) => (u.ai_type ? humanize(u.ai_type.replace(/^UNITAI_/, "")) : null);
  const orders = (u) => (u.missions?.length ? u.missions.map((m) => humanize(m.replace(/^MISSION_/, ""))).join(" › ") : null);
  const units = list.map((u) =>
    h(
      "div.plot-unit",
      `${dname("UnitType", u.type)} (${ownerText(u)}) ⚔${u.strength} ${Math.round((100 * u.hp) / u.max_hp)}%`,
      u.level > 1 ? h("span.dim", ` L${u.level}`) : null,
      role(u) || orders(u) ? h("div.dim.small", [role(u) ? `AI: ${role(u)}` : null, orders(u) ? `orders: ${orders(u)}` : null].filter(Boolean).join(" · "), ` · #${u.id}${u.group !== undefined && u.group !== null ? ` g${u.group}` : ""}`) : null,
    ),
  );
  // The stack at a glance: how many, how many can fight, by owner.
  const fighters = list.filter((u) => (u.strength ?? 0) > 0);
  const owners = [...new Set(list.map((u) => u.owner))];
  const strongest = fighters.reduce((b, u) => (!b || u.strength > b.strength ? u : b), null);
  const summary = list.length
    ? h(
        "p.stack-summary",
        `${list.length} unit${list.length > 1 ? "s" : ""}`,
        fighters.length !== list.length ? `, ${fighters.length} can fight` : "",
        strongest ? ` · strongest ${dname("UnitType", strongest.type)} ⚔${strongest.strength}` : "",
        owners.length > 1 ? ` · ${owners.map((o) => `${playerName(o)} ${list.filter((u) => u.owner === o).length}`).join(", ")}` : "",
      )
    : null;
  const recent = (S.messages ?? []).filter((m) => Array.isArray(m.at) && m.at[0] === x && m.at[1] === y && m.t >= latestEventTurn());
  const events = recent.length
    ? h("div.section", h("h4", "This turn here"), recent.map((m) => h(`div.small.ev-${eventKind(m.text)}`, `${m.player !== null && m.player !== undefined ? `${playerName(m.player)}: ` : ""}${m.text}`)))
    : null;
  const flag = S.spectating || observing() ? h("button#flag-plot", { onclick: () => emit("flag-plot", x, y), title: "Record this plot for a bug report: the turn, the units, the game's settings and the replay commands [F]" }, "⚑ Flag this plot…") : null;
  fill(
    panel,
    h("div.panel-head", h("h3", `Plot (${x}, ${y})`), p.vis === 1 ? h("span.dim", " fogged") : null),
    h("table.kv", rows.map(([k, v]) => h("tr", h("th", k), h("td", String(v))))),
    summary,
    units.length ? h("div.section", h("h4", "Units"), units) : null,
    p.explore ? h("div.section.plot-explore", h("h4", p.explore.kind === "hut" ? "Goody hut odds" : "Explore Lair odds"), h("div.small", markup(exploreFull(p.explore, dname)))) : null,
    events,
    flag,
  );
}
