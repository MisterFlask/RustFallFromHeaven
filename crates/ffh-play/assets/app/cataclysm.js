// Cataclysm live HUD, map interaction, and interactive 3x3 Ogre Battle Squad panel.
// Reuses 100% of the HD-2D / ISO map renderer, animated unit sprites, 248 leader portraits,
// and `battle_watch.js` battle playback!

import * as api from "./api.js";
import { S, emit } from "./state.js";
import { h, fill, toast, errorText } from "./dom.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";
import { isoSkin } from "./isoui.js";
import { openReport } from "./engagements.js";
import { catCmd, openMacroCity, openDoctrineTree, openCourtScreen, openHousesAndApocalypse, openCataclysmCities } from "./cataclysm_screens.js";

let selectedSquadId = null;
let selectedMemberId = null;
let mapViewRef = null;

export function isCataclysm() {
  return S.view?.mode === "cataclysm";
}

export function initCataclysm(mapView) {
  mapViewRef = mapView;
}

export function clearSelection() {
  selectedSquadId = null;
  selectedMemberId = null;
}

function cat() {
  return S.view?.cataclysm ?? null;
}

function myId() {
  return S.view?.me?.player ?? 0;
}

function selectedSquad() {
  const data = cat();
  if (!data) return null;
  if (selectedSquadId !== null) {
    const found = data.squads.find((s) => s.id === selectedSquadId && s.owner === myId());
    if (found) return found;
  }
  return data.squads.find((s) => s.owner === myId() && s.moves_left > 0) ?? data.squads.find((s) => s.owner === myId()) ?? null;
}

export function selectNextSquad() {
  const data = cat();
  if (!data) return;
  const mine = data.squads.filter((s) => s.owner === myId() && s.moves_left > 0);
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
  S.sel = { units: sq.members.map((m) => m.id), x: sq.x, y: sq.y };
  mapViewRef?.request();
}

export async function endCataclysmTurn() {
  const reply = await catCmd({ c: "end_turn" });
  if (reply?.r?.report) {
    openReport(reply.r.report);
  }
}

export function openCataclysmScreen(name) {
  switch (name) {
    case "cities":
      return openCataclysmCities();
    case "research":
    case "techs":
    case "religion":
    case "civics":
      return openDoctrineTree();
    case "armies":
    case "governors":
    case "trade":
      return openCourtScreen();
    case "houses":
    case "diplomacy":
    case "crusades":
    case "seas":
    case "finance":
      return openHousesAndApocalypse();
    default:
      return openHousesAndApocalypse();
  }
}

// ---------------------------------------------------------------------------------------------
// Map Click & Right-Click Handlers

export function handleClick(x, y) {
  const data = cat();
  if (!data) return false;
  const sqs = data.squads.filter((s) => s.x === x && s.y === y && s.owner === myId());
  if (sqs.length) {
    const curIdx = sqs.findIndex((s) => s.id === selectedSquadId);
    const pick = sqs[(curIdx + 1) % sqs.length];
    selectedSquadId = pick.id;
    selectedMemberId = null;
    syncSelectionToMap(pick);
    renderCataclysmPanel();
    return true;
  }
  const city = data.cities.find((c) => c.x === x && c.y === y);
  if (city && city.owner === myId()) {
    openMacroCity(city.id);
    return true;
  }
  return false;
}

export function handleDblClick(x, y) {
  const data = cat();
  if (!data) return false;
  const city = data.cities.find((c) => c.x === x && c.y === y);
  if (city) {
    openMacroCity(city.id);
    return true;
  }
  return false;
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
    const w = S.map?.w ?? 0;
    let dx = x - curX;
    if (S.map?.wrap_x && w > 0) {
      if (dx > w / 2) dx -= w;
      if (dx < -w / 2) dx += w;
    }
    const dy = y - curY;
    const nx = S.map?.wrap_x && w > 0 ? ((curX + Math.sign(dx)) % w + w) % w : curX + Math.sign(dx);
    const ny = curY + Math.sign(dy);
    const enemyHere =
      data.squads.some((s) => s.x === nx && s.y === ny && s.owner !== myId()) ||
      data.cities.some((c) => c.x === nx && c.y === ny && c.owner !== myId());

    const reply = await catCmd({ c: enemyHere ? "cat_attack" : "cat_move", squad: sqId, x: nx, y: ny });
    if (!reply) break;
    if (reply.r?.report) {
      // Launch the existing animated 3x3 Ogre Battle watch screen!
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
// Cataclysm Top Bar (4 Decoupled Currencies + Apocalypse Clock + Court Alert)

export function renderCataclysmTop() {
  const data = cat();
  const info = document.getElementById("empire");
  if (!data || !info) return;
  const cur = data.currencies;
  const res = data.research.current;
  const ap = data.apocalypse;
  const courtN = data.court.length;
  const fmtRate = (n) => `${n >= 0 ? "+" : ""}${n}/t`;

  const calcTip = (title, lines) => [
    h("div.tip-title", title),
    ...(lines?.length ? lines.map((l) => h("div.tip-calc-line", l)) : [h("div.dim", "No active modifiers.")]),
  ];

  fill(
    info,
    h("span.cat-top-chip.act", { tip: [h("div.tip-title", `${data.human_faction} · ${ap.act_label}`), data.human_lore_blurb ? h("div.dim", data.human_lore_blurb) : null, h("div", data.human_rule_summary)].filter(Boolean) }, `${data.human_faction} · ${ap.act_label}`),
    h(
      "button.cat-top-chip",
      { tip: calcTip(`Gold ${cur.gold} (${fmtRate(cur.gold_rate)})`, cur.gold_calc), onclick: () => openHousesAndApocalypse() },
      `Gold ${cur.gold} (${fmtRate(cur.gold_rate)})`,
    ),
    h(
      "button.cat-top-chip",
      {
        tip: calcTip(`Lore ${cur.lore} (${fmtRate(cur.lore_rate)})`, cur.lore_calc),
        onclick: () => openDoctrineTree(),
      },
      res ? `Lore: ${res.name} (${res.turns}t)` : `Lore ${cur.lore} (${fmtRate(cur.lore_rate)}) · Pick Tech!`,
    ),
    h(
      "button.cat-top-chip",
      {
        tip: calcTip(`Vent-Ether Mana ${cur.mana} (${fmtRate(cur.mana_rate)})`, cur.mana_calc),
        onclick: () => openDoctrineTree(),
      },
      `Mana ${cur.mana} (${fmtRate(cur.mana_rate)})`,
    ),
    h(
      `button.cat-top-chip${ap.counter >= 50 ? ".warn" : ""}`,
      {
        tip: calcTip(
          `Worldseal Strain ${ap.counter}/100`,
          ap.recent_changes?.length ? ap.recent_changes : ["Rises from vent-tapping, deep smelting, Lifeboat wonders, razing, and creeping pressure.", "Strain 50: King Cinder's Host marches out. Strain 100: Caldera Keep rises + 40-turn Blowout countdown."],
        ),
        onclick: () => openHousesAndApocalypse(),
      },
      ap.carcosa_risen ? `CALDERA KEEP: ${ap.countdown}t LEFT` : `Strain ${ap.counter}/100`,
    ),
    h(
      `button.cat-top-chip${courtN > 0 ? ".accent" : ""}`,
      { tip: [h("div.tip-title", "Great People Court"), h("div", `Renown ${data.renown.total}/${data.renown.next_threshold} across 6 suits.`)], onclick: () => openCourtScreen() },
      courtN > 0 ? `Court (${courtN} READY!)` : `Court (${data.renown.total}/${data.renown.next_threshold})`,
    ),
    h(
      "button.cat-top-chip",
      { tip: [h("div.tip-title", "Regional Houses & Diplomacy"), h("div", "Loyalty scales city output; Furious disables the House perk.")], onclick: () => openHousesAndApocalypse() },
      `Houses (${data.houses.length})`,
    ),
  );

  // Keep hud.js's native End Turn icon/styling; just clear any classic block state.
  const end = document.getElementById("end-turn");
  if (end) {
    end.disabled = S.status?.phase !== "human";
    end.classList.remove("blocked");
    end.title = "End the turn [Enter]";
  }
}

// ---------------------------------------------------------------------------------------------
// Cataclysm Side Panel: Interactive 3x3 Ogre Battle Squad Editor & Orders

export function renderCataclysmPanel() {
  const panel = document.getElementById("unit-panel");
  const data = cat();
  if (!panel || !data) return;

  const sq = selectedSquad();
  if (sq) {
    selectedSquadId = sq.id;
    syncSelectionToMap(sq);
  }

  const ownSquads = data.squads.filter((s) => s.owner === myId());
  const ownCities = data.cities.filter((c) => c.owner === myId());

  const nodes = [];
  // Quick switcher across own squads & cities
  nodes.push(
    h(
      "div.cat-switcher",
      h("strong", `Squads (${ownSquads.length}/${data.squad_cap})`),
      h(
        "div.cat-chip-list",
        ...ownSquads.map((s) =>
          h(
            `button.cat-mini-btn${sq && s.id === sq.id ? ".primary" : ""}`,
            {
              onclick: () => {
                selectedSquadId = s.id;
                selectedMemberId = null;
                syncSelectionToMap(s);
                mapViewRef?.centerOn(s.x, s.y);
                renderCataclysmPanel();
              },
            },
            `${s.name.split(" ")[0]} (${s.moves_left}/${s.max_moves})`,
          ),
        ),
      ),
    ),
  );

  if (sq) {
    const port = leaderPortrait(sq.captain_portrait, { size: PORTRAIT.small, title: sq.captain_name });
    const supplyLabel = !sq.supplied
      ? sq.unsupplied_turns >= 2
        ? "STARVING (-35%)"
        : "HUNGRY (-15%)"
      : "Supplied";

    // Build 3x3 interactive Ogre Battle formation grid!
    const gridCells = [];
    const rowNames = ["Front", "Mid", "Rear"];
    for (let r = 0; r < 3; r++) {
      gridCells.push(h("div.cat-grid-row-label", rowNames[r]));
      for (let l = 0; l < 3; l++) {
        const occ = sq.members.find((m) => m.row === r && (m.lane === l || (m.large && m.lane + 1 === l)));
        const isSel = occ && occ.id === selectedMemberId;
        const rowAct = occ?.actions?.[r];
        const actText = rowAct && rowAct.n > 0 ? `${rowAct.kind} x${rowAct.n}` : "Idle";
        gridCells.push(
          h(
            `button.cat-cell${occ ? ".occupied" : ""}${isSel ? ".selected" : ""}`,
            {
              title: occ
                ? `${occ.name} (Lv ${occ.level}${occ.is_hero ? " ★Hero" : ""}${occ.wounds ? ` · ${occ.wounds} Wounds` : ""})\nHP ${occ.hp_pct}% · ATK ${occ.atk} · DEF ${occ.def} · MAG ${occ.magic}\nFrom ${rowNames[r]} row: ${actText}\nClick to select, then click another cell to reposition!`
                : selectedMemberId
                  ? `Move selected unit to ${rowNames[r]} lane ${l + 1}`
                  : "Empty cell",
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
                  h("span.cat-cell-name", `${occ.is_hero ? "★" : ""}${occ.class.slice(0, 7)}`),
                  h("small.cat-cell-act", actText),
                  h("div.cat-hp-bar", h("div.cat-hp-fill", { style: { width: `${occ.hp_pct}%` } })),
                ]
              : h("span.dim", "·"),
          ),
        );
      }
    }

    // Reinforce selector when inside home borders & places remain
    const unlockedClasses = data.recruitable_classes.filter((u) => u.unlocked);
    let reinforceKey = unlockedClasses[0]?.key ?? "guard";
    const reinforceSel = h(
      "select",
      { onchange: (ev) => (reinforceKey = ev.target.value) },
      ...unlockedClasses.map((u) => h("option", { value: u.key }, `${u.name} (${u.cost_gold}G${u.large ? ", 2pl" : ""})`)),
    );

    const upgradeable = sq.members.filter((m) => m.upgrades_to && sq.in_home_borders);

    nodes.push(
      h(
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
                h("div.small.dim", `Capt. ${sq.captain_name} · ${sq.captain_aura}`),
                h(
                  "div.small",
                  `Moves ${sq.moves_left}/${sq.max_moves} · Places ${sq.places_used}/${sq.place_cap} · Pwr ${sq.power} · `,
                  h("span", { className: sq.supplied ? "good" : "warn" }, supplyLabel),
                ),
              ),
            ),
            h("div.small.dim", "Right-click tile to move / battle · Click 2 cells to swap"),
            h(
              "div.cat-actions-row",
              h("button", { onclick: () => catCmd({ c: "cat_auto_arrange", squad: sq.id }) }, "Auto-Arrange"),
              h("button", { onclick: () => catCmd({ c: "cat_fortify", squad: sq.id }) }, "Fortify"),
              h("button", { disabled: sq.moves_left <= 0, onclick: () => catCmd({ c: "cat_road", squad: sq.id }) }, "Road"),
              h("button", { disabled: sq.moves_left <= 0, onclick: () => catCmd({ c: "cat_fort", squad: sq.id }) }, "Fort"),
              sq.on_enemy_countryside
                ? h("button.primary", { onclick: () => catCmd({ c: "cat_raid", squad: sq.id, active: sq.order !== "raid" }) }, sq.order === "raid" ? "Stop Raid" : "Raid")
                : null,
              sq.has_lair
                ? h("button.primary", { onclick: () => catCmd({ c: "cat_explore", squad: sq.id }) }, "Explore Ruin")
                : null,
              sq.can_found_city
                ? h("button.primary", { onclick: () => catCmd({ c: "cat_found_city", squad: sq.id }) }, `Found City (${sq.found_cost}G)`)
                : null,
            ),
            sq.in_home_borders && sq.places_used < sq.place_cap
              ? h(
                  "div.cat-reinforce-row",
                  reinforceSel,
                  h("button.primary", { onclick: () => catCmd({ c: "cat_reinforce", squad: sq.id, class: reinforceKey }) }, "+ Reinforce"),
                )
              : null,
            ...upgradeable.map((m) =>
              h(
                "button.cat-mini-btn",
                { onclick: () => catCmd({ c: "cat_upgrade", squad: sq.id, member: m.id }) },
                `Upgrade ${m.name} → ${m.upgrades_to} (${m.upgrade_cost}G)`,
              ),
            ),
          ),
          h("div.cat-formation-grid", ...gridCells),
        ),
      ),
    );
  }

  const citiesBlock = h(
    "div.cat-switcher",
    h("strong", `Macro Cities (${ownCities.length})`),
    h(
      "div.cat-chip-list",
      ...ownCities.map((c) =>
        h(
          `button.cat-mini-btn${!c.building ? ".warn" : ""}`,
          { onclick: () => (mapViewRef?.centerOn(c.x, c.y), openMacroCity(c.id)) },
          `${c.capital ? "★" : ""}${c.name} (${c.size})${c.building ? ` · ${c.building.turns}t` : " · Idle"}`,
        ),
      ),
    ),
  );

  const recent = S.view?.battles ?? [];
  const battlesBlock = recent.length
    ? h(
        "div.cat-switcher",
        h("strong", "Recent 3×3 Battles"),
        ...recent.slice(0, 4).map((b) =>
          h(
            "button.cat-mini-btn",
            { onclick: () => openReport(b.id) },
            `Watch #${b.id}: ${b.sides[0].label} vs ${b.sides[1].label}`,
          ),
        ),
      )
    : null;

  if (!isoSkin()) {
    nodes.push(citiesBlock);
    if (battlesBlock) nodes.push(battlesBlock);
  }

  fill(panel, ...nodes);
  renderCataclysmCityPanel();
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
  const ownCities = data.cities.filter((c) => c.owner === myId());
  const cap = ownCities.find((c) => c.capital) ?? ownCities[0] ?? null;
  const recent = S.view?.battles ?? [];
  el.hidden = false;
  fill(
    el,
    h("h3.iso-title", cap ? `${cap.capital ? "★ " : ""}${cap.name}` : "Empire Overview"),
    cap
      ? h(
          "div.cat-iso-city-box",
          h("div.small", `Size ${cap.size}/10 · Districts ${cap.districts.length}/${cap.max_district_slots} · Def +${cap.defense_pct}%`),
          h("div.small.dim", `+${cap.industry_rate} Ind · +${cap.gold_rate} Gold · +${cap.lore_rate} Lore`),
          h("div.small", cap.building ? `Building: ${cap.building.label} (${cap.building.turns}t)` : h("span.warn", "Idle — open to build")),
          cap.crew_task ? h("div.small.dim", `Crew: ${cap.crew_task.label} (${cap.crew_task.progress}/${cap.crew_task.needed}t)`) : null,
          h("button.iso-open", { onclick: () => (mapViewRef?.centerOn(cap.x, cap.y), openMacroCity(cap.id)) }, "Manage Macro City"),
        )
      : null,
    ownCities.length > 1
      ? h(
          "div.cat-switcher",
          h("strong", `All Cities (${ownCities.length})`),
          h(
            "div.cat-chip-list",
            ...ownCities.map((c) =>
              h(
                `button.cat-mini-btn${!c.building ? ".warn" : ""}`,
                { onclick: () => (mapViewRef?.centerOn(c.x, c.y), openMacroCity(c.id)) },
                `${c.name} (${c.size})`,
              ),
            ),
          ),
        )
      : null,
    recent.length
      ? h(
          "div.cat-switcher",
          h("strong", "Recent 3×3 Battles"),
          ...recent.slice(0, 3).map((b) =>
            h(
              "button.cat-mini-btn",
              { onclick: () => openReport(b.id) },
              `Watch #${b.id}: ${b.sides[0].label} vs ${b.sides[1].label}`,
            ),
          ),
        )
      : null,
  );
}

