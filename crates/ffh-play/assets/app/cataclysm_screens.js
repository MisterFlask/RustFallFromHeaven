// Modal screens for the Cataclysm alternate ruleset mode:
// - New Cataclysm game setup (9 asymmetric factions with leader portraits)
// - Macro City panel (4-6 District slots, Sentinels, Work Crew, Squads, Wonders, Caravans)
// - Foundation + 7 Doctrine Ladders + Religions + Strategic Rituals
// - Great People Court (6 suit Renown meters + visible Secret Tech + 3-way choice)
// - Regional Houses + Apocalypse Clock / Crisis Deck / Lifeboats / Crusades / Diplomacy

import * as api from "./api.js";
import { S, emit } from "./state.js";
import { h, toast, errorText } from "./dom.js";
import { openModal, closeTag } from "./modals.js";
import { leaderPortrait, PORTRAIT } from "./portraits.js";

export async function catCmd({ c, ...fields }) {
  try {
    const reply = await api.cmd(c, fields);
    emit("applied", reply);
    return reply;
  } catch (e) {
    toast(errorText(e), "error");
    return null;
  }
}

function cat() {
  return S.view?.cataclysm ?? null;
}

function portrait(key, size = PORTRAIT.small, title = "") {
  return leaderPortrait(key, { size, title }) ?? h("span.cat-portrait-fallback", title.slice(0, 2));
}

// ---------------------------------------------------------------------------------------------
// 1. New Cataclysm Game Screen

export async function openCataclysmNewGame({ noGame = false } = {}) {
  let setup;
  try {
    setup = await api.query("cataclysm_setup");
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const factions = setup.factions ?? [];
  let chosen = factions[0]?.key ?? "bannor";
  let size = "standard";
  let opponents = 5;
  let seed = Math.floor(Math.random() * 90000) + 1000;

  const detailEl = h("div.cat-ng-detail");
  const gridEl = h("div.cat-ng-factions");

  function renderFactions() {
    gridEl.replaceChildren(
      ...factions.map((f) => {
        const active = f.key === chosen;
        return h(
          `button.cat-fac-card${active ? ".selected" : ""}`,
          {
            type: "button",
            "data-faction": f.key,
            onclick: () => {
              chosen = f.key;
              renderFactions();
            },
          },
          portrait(f.leader_art, PORTRAIT.small, f.leader_name),
          h(
            "div.cat-fac-meta",
            h("strong", f.name),
            h("small.dim", `${f.leader_name} · ${f.alignment}`),
          ),
        );
      }),
    );
    const f = factions.find((x) => x.key === chosen) ?? factions[0];
    if (f) {
      detailEl.replaceChildren(
        h(
          "div.cat-fac-hero",
          portrait(f.leader_art, PORTRAIT.medium, f.leader_name),
          h(
            "div",
            h("h4", `${f.name} — ${f.leader_name}`),
            h("p.dim", `Alignment: ${f.alignment} · Preferred Doctrine: ${f.preferred_doctrine} · Victory: ${f.can_mortal_win && f.can_demonic_win ? "Surface or Furnace" : f.can_demonic_win ? "Furnace (Raze All Lifeboats)" : "Surface (Hold a Lifeboat)"}`),
            f.lore_blurb ? h("p.dim", f.lore_blurb) : null,
            h("p", f.rule_summary),
          ),
        ),
      );
    }
  }
  renderFactions();

  const sizeSel = h(
    "select",
    { onchange: (ev) => (size = ev.target.value) },
    ...(setup.sizes ?? []).map((s) => h("option", { value: s.key, selected: s.key === size }, s.name)),
  );
  const oppSel = h(
    "select",
    { onchange: (ev) => (opponents = Number(ev.target.value)) },
    ...[2, 3, 4, 5, 6, 7].map((n) => h("option", { value: String(n), selected: n === opponents }, `${n} Rival Empires`)),
  );
  const seedIn = h("input", { type: "number", value: String(seed), oninput: (ev) => (seed = Number(ev.target.value) || 1) });

  let starting = false;
  const doStart = async () => {
    if (starting) return;
    starting = true;
    try {
      await api.session("new", { setup: { mode: "cataclysm", faction: chosen, size, opponents, seed } });
      closeTag("cataclysm-new");
      emit("game-started");
    } catch (e) {
      toast(errorText(e), "error");
      starting = false;
    }
  };

  openModal({
    title: "Cataclysm · The Failing Worldseal",
    tag: "cataclysm-new",
    wide: true,
    dismissible: !noGame,
    body: [
      h("p.dim", "The Great Empire sealed its rebel furnace-servitors beneath the central sea. Now the Worldseal groans · 3×3 Squads · Macro Cities · Great People Court · Worldseal Strain & Lifeboat Endgame."),
      gridEl,
      detailEl,
      h(
        "div.cat-ng-controls",
        h("label", "World ", sizeSel),
        h("label", "Opponents ", oppSel),
        h("label", "Seed ", seedIn),
      ),
    ],
    buttons: [
      { label: "Start Cataclysm", id: "cat-start-btn", primary: true, onClick: doStart },
      { label: noGame ? "Back" : "Cancel", value: null },
    ],
  });
}

// ---------------------------------------------------------------------------------------------
// 2. Macro City Screen (4-6 Districts, Sentinels, Work Crew, Squads, Wonders, Caravans)

export function openMacroCity(cityId) {
  const c = cat()?.cities?.find((x) => x.id === cityId);
  if (!c) return;
  const data = cat();
  const isOwn = c.owner === S.view?.me?.player;

  const districtSlots = [];
  for (let i = 0; i < c.max_district_slots; i++) {
    const d = c.districts[i];
    districtSlots.push(
      d
        ? h("div.cat-slot.filled", h("strong", d.name), h("small.dim", "Built"))
        : h("div.cat-slot.empty", h("span.dim", "Open District Slot")),
    );
  }

  const sentinelPips = c.sentinels.length
    ? h(
        "div.cat-row",
        h("strong", "Wall Sentinels: "),
        ...c.sentinels.map((s, idx) =>
          h("span.cat-pill", { className: s.breached ? "cat-pill warn" : "cat-pill" }, `#${idx + 1}: ${s.breached ? "Breached" : `${s.hp}% HP`}`),
        ),
      )
    : h("p.dim", "No Wall Sentinels yet (build a Bulwark district).");

  const buildSection = [];
  if (isOwn) {
    // Districts
    const builtKeys = new Set(c.districts.map((d) => d.key));
    const hasRoom = c.districts.length < c.max_district_slots;
    buildSection.push(h("h4", `Build District (${c.districts.length}/${c.max_district_slots} slots)`));
    buildSection.push(
      h(
        "div.cat-grid-2",
        ...data.available_districts.map((d) => {
          const already = builtKeys.has(d.key);
          const disabled = already || !hasRoom || !d.unlocked;
          const why = already ? "Already built" : !hasRoom ? "Grow city Size for another slot" : d.why;
          return h(
            "button.cat-card-btn",
            {
              disabled,
              title: why ?? d.summary,
              onclick: async () => {
                if (await catCmd({ c: "cat_city_build", city: c.id, kind: "district", key: d.key })) {
                  closeTag("cat-city");
                }
              },
            },
            h("strong", `${d.name} (${d.cost} Ind)`),
            h("small.dim", why ? `${d.summary} · ${why}` : d.summary),
          );
        }),
      ),
    );

    // Raise Squad Presets
    const unlocked = new Set(data.recruitable_classes.filter((u) => u.unlocked).map((u) => u.key));
    const className = new Map(data.recruitable_classes.map((u) => [u.key, u.name]));
    const pick = (...keys) => keys.find((k) => unlocked.has(k)) ?? "guard";
    const presets = [
      { label: "Vanguard Squad (Front + Archers)", classes: [pick("phalanx", "champion", "axeman", "guard"), pick("pikeman", "axeman", "guard"), pick("longbowman", "archer", "guard")] },
      { label: "Postrider Column (Fast Flanking)", classes: [pick("knight", "horseman", "guard"), pick("ranger", "assassin", "horseman", "guard"), pick("archer", "guard")] },
      { label: "Arcane Battery (Guard + Casters)", classes: [pick("phalanx", "axeman", "guard"), pick("archmage", "mage", "adept", "guard"), pick("acolyte", "adept", "guard")] },
      { label: "Siege Train (Guard + Bombards)", classes: [pick("phalanx", "axeman", "guard"), pick("trebuchet", "catapult", "archer", "guard"), pick("catapult", "archer", "guard")] },
    ];
    const capFull = data.squad_count >= data.squad_cap;
    buildSection.push(h("h4", `Raise Squad (${data.squad_count}/${data.squad_cap} Squad Cap)`));
    buildSection.push(
      h(
        "div.cat-grid-2",
        ...presets.map((pr) => {
          const labels = pr.classes.map((k) => className.get(k) ?? k);
          return h(
            "button.cat-card-btn",
            {
              disabled: capFull,
              title: capFull ? "Squad cap reached — build Barracks or expand" : labels.join(", "),
              onclick: async () => {
                if (await catCmd({ c: "cat_city_build", city: c.id, kind: "squad", classes: pr.classes })) {
                  closeTag("cat-city");
                }
              },
            },
            h("strong", pr.label),
            h("small.dim", labels.join(" + ")),
          );
        }),
      ),
    );

    // Lifeboat Wonders
    buildSection.push(h("h4", "Lifeboat Wonders"));
    buildSection.push(
      h(
        "div.cat-grid-2",
        ...data.available_wonders.map((w) =>
          h(
            "button.cat-card-btn",
            {
              disabled: !w.unlocked,
              title: w.why ?? w.summary,
              onclick: async () => {
                if (await catCmd({ c: "cat_city_build", city: c.id, kind: "wonder", key: w.key })) {
                  closeTag("cat-city");
                }
              },
            },
            h("strong", `${w.name} (${w.cost} Ind)`),
            h("small.dim", w.why ? `${w.summary} · ${w.why}` : w.summary),
          ),
        ),
      ),
    );

    // Caravans & Sally Posture
    const partners = data.cities.filter((o) => o.id !== c.id && !data.players.find((p) => p.id === o.owner)?.war);
    buildSection.push(
      h(
        "div.cat-row",
        h("strong", "Sally against Raiders: "),
        ...["never", "favoured", "always"].map((p) =>
          h(
            `button${c.sally === p ? ".primary" : ""}`,
            {
              onclick: async () => {
                if (await catCmd({ c: "cat_sally", city: c.id, posture: p })) closeTag("cat-city");
              },
            },
            p[0].toUpperCase() + p.slice(1),
          ),
        ),
        partners.length
          ? h(
              "button",
              {
                disabled: data.caravan_count >= data.caravan_cap,
                title: `Launch a Trade Caravan to ${partners[0].name} (35 Gold)`,
                onclick: async () => {
                  if (await catCmd({ c: "cat_caravan", origin: c.id, destination: partners[0].id })) closeTag("cat-city");
                },
              },
              `Launch Caravan (${data.caravan_count}/${data.caravan_cap})`,
            )
          : null,
        h(
          "button.danger",
          {
            onclick: async () => {
              if (await catCmd({ c: "cat_raze", city: c.id })) closeTag("cat-city");
            },
          },
          "Raze City (+6 Strain)",
        ),
      ),
    );
  }

  openModal({
    title: `${c.capital ? "★ " : ""}${c.name}${c.lifeboat ? ` · LIFEBOAT (${c.lifeboat})` : ""}`,
    tag: "cat-city",
    wide: true,
    body: [
      h(
        "div.cat-stats-strip",
        h("span", `Size ${c.size}/10 (Food ${c.food_stored}/${c.food_needed})`),
        h("span", { title: (c.industry_calc ?? []).join("\n") }, `Industry +${c.industry_rate}/t`),
        h("span", `Gold +${c.gold_rate}/t`),
        h("span", `Lore +${c.lore_rate}/t`),
        h("span", `Culture ${c.culture} (+${c.culture_rate}/t)`),
        h("span", `Defense +${c.defense_pct}%`),
      ),
      c.besieged ? h("p.warn", "BESIEGED: Sentinel repair and city healing are halted!") : null,
      c.stripped_turns > 0 ? h("p.warn", `STRIPPED COUNTRYSIDE: -50% output for ${c.stripped_turns} more turns.`) : null,
      sentinelPips,
      c.building
        ? h("p", h("strong", "Producing: "), `${c.building.label} (${c.building.progress}/${c.building.cost} · ${c.building.turns}t)`)
        : h("p.warn", "City queue is idle — choose a District, Squad, or Wonder below."),
      c.crew_task
        ? h("p.dim", `Automatic Work Crew: building ${c.crew_task.label} (${c.crew_task.progress}/${c.crew_task.needed}t)`)
        : h("p.dim", "Automatic Work Crew: all nearby tiles improved."),
      h("div.cat-slots-row", ...districtSlots),
      ...buildSection,
    ],
    buttons: [{ label: "Close", value: null }],
  });
}

// ---------------------------------------------------------------------------------------------
// 3. Doctrine, Tech Tree, Creeds & Strategic Rituals Screen

export function openDoctrineTree() {
  const data = cat();
  if (!data) return;
  const res = data.research;
  const foundation = res.techs.filter((t) => !t.doctrine);
  const byDoc = new Map();
  for (const d of res.doctrines) byDoc.set(d.key, res.techs.filter((t) => t.doctrine === d.key));

  const techBtn = (t) => {
    const isCur = res.current?.id === t.id;
    return h(
      `button.cat-tech-node${t.known ? ".known" : isCur ? ".active" : t.can_research ? ".avail" : ".locked"}`,
      {
        disabled: t.known || !t.can_research,
        title: `${t.unlocks}${t.why ? `\nLocked: ${t.why}` : ""}`,
        onclick: async () => {
          if (await catCmd({ c: "cat_research", tech: t.id })) {
            closeTag("cat-techs");
            openDoctrineTree();
          }
        },
      },
      h("div.cat-tech-head", h("strong", t.name), h("small", t.known ? "Known" : `${t.cost} Lore${t.ac_delta ? ` · ${t.ac_delta > 0 ? "+" : ""}${t.ac_delta} Strain` : ""}`)),
      h("small.dim", t.unlocks),
    );
  };

  openModal({
    title: "Foundation, Doctrines, Creeds & Rituals",
    tag: "cat-techs",
    wide: true,
    body: [
      h(
        "p.dim",
        `Primary Doctrine: ${res.primary_doctrine_name ?? "None chosen"} · Secondary (Tiers 1–2): ${res.secondary_doctrine_name ?? "None chosen"} · Lore: ${data.currencies.lore} (+${data.currencies.lore_rate}/t)`,
      ),
      h("h4", "Foundation (8 Opening Techs)"),
      h("div.cat-grid-4", ...foundation.map(techBtn)),
      h("h4", "7 Doctrine Ladders (Commit to 1 Primary + 1 Secondary Dip)"),
      ...res.doctrines.map((doc) =>
        h(
          "div.cat-doc-row",
          h(
            "div.cat-doc-head",
            h("strong", `${doc.name}${doc.primary ? " ★ PRIMARY" : doc.secondary ? " ◆ SECONDARY" : ""}`),
            h("small.dim", doc.summary),
            !res.primary_doctrine
              ? h(
                  "button.primary",
                  {
                    onclick: async () => {
                      if (await catCmd({ c: "cat_doctrine", doctrine: doc.key, secondary: false })) {
                        closeTag("cat-techs");
                        openDoctrineTree();
                      }
                    },
                  },
                  "Adopt Primary",
                )
              : !res.secondary_doctrine && !doc.primary
                ? h(
                    "button",
                    {
                      onclick: async () => {
                        if (await catCmd({ c: "cat_doctrine", doctrine: doc.key, secondary: true })) {
                          closeTag("cat-techs");
                          openDoctrineTree();
                        }
                      },
                    },
                    "Adopt Secondary",
                  )
                : null,
          ),
          h("div.cat-grid-4", ...(byDoc.get(doc.key) ?? []).map(techBtn)),
        ),
      ),
      h("h4", "Creeds (Separate World Race)"),
      h(
        "div.cat-grid-2",
        ...data.religions.map((r) =>
          h(
            `button.cat-card-btn${r.adopted ? ".selected" : ""}`,
            {
              disabled: r.adopted,
              onclick: async () => {
                if (await catCmd({ c: "cat_religion", religion: r.id })) {
                  closeTag("cat-techs");
                  openDoctrineTree();
                }
              },
            },
            h("strong", `${r.name} (${r.alignment}) ${r.adopted ? "★ Adopted" : r.founded_by ? `· Founded by ${r.founded_by}` : `· Found (${r.lore_cost} Lore/Gold)`}`),
            h("small.dim", r.summary),
          ),
        ),
      ),
      h("h4", `Strategic Rituals (Mana: ${data.currencies.mana}, +${data.currencies.mana_rate}/t)`),
      h(
        "div.cat-grid-2",
        ...data.rituals.map((rk) =>
          h(
            "button.cat-card-btn",
            {
              disabled: !rk.can_cast,
              onclick: async () => {
                if (await catCmd({ c: "cat_ritual", ritual: rk.key })) {
                  closeTag("cat-techs");
                }
              },
            },
            h("strong", `${rk.name} (${rk.mana_cost} Mana · ${rk.ac_delta >= 0 ? "+" : ""}${rk.ac_delta} Strain)`),
            h("small.dim", rk.summary),
          ),
        ),
      ),
    ],
    buttons: [{ label: "Close", value: null }],
  });
}

// ---------------------------------------------------------------------------------------------
// 4. Great People Court Screen (6 Suit Meters + Shared World Secret-Tech Deck + 3-Way Choice)

export function openCourtScreen() {
  const data = cat();
  if (!data) return;
  const ownSquads = data.squads.filter((s) => s.owner === S.view?.me?.player && s.places_used < s.place_cap);
  const ownHouses = data.houses;

  const gpCards = data.court.length
    ? data.court.map((gp) =>
        h(
          "div.cat-gp-card",
          h(
            "div.cat-gp-top",
            portrait(gp.portrait, PORTRAIT.medium, gp.name),
            h(
              "div",
              h("h4", `${gp.name} · Great ${gp.suit}`),
              gp.secret_tech
                ? h(
                    "div.cat-secret-box",
                    h("strong", `Secret Tech: ${gp.secret_tech.name}`),
                    h("p", gp.secret_tech.summary),
                    h("small.dim", `Needs: ${gp.secret_tech.needs_name} (${gp.secret_tech.needs_known ? "Known" : "Not yet researched"}) · Expert: ${gp.secret_tech.expert_line}`),
                  )
                : h("p.dim", "No undiscovered Secret Techs left in this suit."),
            ),
          ),
          h(
            "div.cat-actions-row",
            gp.secret_tech
              ? h(
                  "button.primary",
                  {
                    onclick: async () => {
                      if (await catCmd({ c: "cat_gp", gp: gp.id, action: "secret" })) {
                        closeTag("cat-court");
                      }
                    },
                  },
                  `1. Discover ${gp.secret_tech.name}`,
                )
              : null,
            h(
              "button",
              {
                title: gp.burst_summary,
                onclick: async () => {
                  if (await catCmd({ c: "cat_gp", gp: gp.id, action: "burst" })) closeTag("cat-court");
                },
              },
              `2. Tempo Burst: ${gp.burst_summary}`,
            ),
            ownSquads.length
              ? h(
                  "button",
                  {
                    onclick: async () => {
                      if (await catCmd({ c: "cat_gp", gp: gp.id, action: "hero", squad: ownSquads[0].id })) closeTag("cat-court");
                    },
                  },
                  `3a. Commission as ${gp.hero_class_name} Hero (${ownSquads[0].name})`,
                )
              : null,
            ownHouses.length
              ? h(
                  "button",
                  {
                    onclick: async () => {
                      if (await catCmd({ c: "cat_gp", gp: gp.id, action: "lord", house: ownHouses[0].id })) closeTag("cat-court");
                    },
                  },
                  `3b. Appoint Lord of ${ownHouses[0].name}`,
                )
              : null,
          ),
        ),
      )
    : [h("p.dim", "No Great People currently waiting in your Court. Earn Renown across the 6 suits via Districts, Caravans, Rituals, Culture, and Battles.")];

  openModal({
    title: "Great People Court & Secret-Tech Deck",
    tag: "cat-court",
    wide: true,
    body: [
      h("p", `Next Great Person at ${data.renown.total} / ${data.renown.next_threshold} Total Renown (highest suit spawns).`),
      h(
        "div.cat-stats-strip",
        ...data.renown.meters.map((m) => h("span", `${m.suit}: ${m.value}`)),
      ),
      h("h4", "Waiting in Your Court (3-Way Choice)"),
      ...gpCards,
      h("h4", `Discovered Secret Techs (${data.secrets_known.length})`),
      data.secrets_known.length
        ? h(
            "div.cat-grid-2",
            ...data.secrets_known.map((s) =>
              h("div.cat-slot.filled", h("strong", `${s.name} ${s.active ? "✓" : `(Needs ${s.needs_name})`}`), h("small.dim", s.summary)),
            ),
          )
        : h("p.dim", "None discovered yet."),
      data.recovering_heroes.length
        ? h("p.dim", `Recovering Wounded Heroes: ${data.recovering_heroes.map((r) => `${r.name} (${r.turns_left}t, ${r.wounds} Wounds)`).join(", ")}`)
        : null,
    ],
    buttons: [{ label: "Close", value: null }],
  });
}

// ---------------------------------------------------------------------------------------------
// 5. Regional Houses, Apocalypse Clock, Lifeboats, Crusades & Diplomacy

export function openHousesAndApocalypse() {
  const data = cat();
  if (!data) return;
  const ap = data.apocalypse;
  const meId = S.view?.me?.player;
  const ownSquads = data.squads.filter((s) => s.owner === meId && s.lent_by === null);

  openModal({
    title: `Houses, Worldseal (${ap.counter}/100) & Diplomacy`,
    tag: "cat-apoc",
    wide: true,
    body: [
      h("h4", "Regional Houses (Loyalty Replaces Happiness & Maintenance)"),
      data.houses.length
        ? h(
            "div.cat-grid-2",
            ...data.houses.map((ho) =>
              h(
                "div.cat-house-card",
                { title: (ho.loyalty_calc ?? []).join("\n") },
                portrait(ho.lord_portrait, PORTRAIT.small, ho.lord_name),
                h(
                  "div",
                  h("strong", `${ho.name} · ${ho.lord_name}`),
                  h("div.small", `Mood: ${ho.mood} (${ho.loyalty >= 0 ? "+" : ""}${ho.loyalty}) · Output ${ho.output_pct}%`),
                  h("small.dim", `Perk: ${ho.perk}${ho.perk_active ? "" : " [DISABLED - FURIOUS]"}`),
                  h("small.dim", `Cities: ${ho.cities.join(", ")}${ho.sympathy_name ? ` · Sympathy: ${ho.sympathy_name}` : ""}`),
                ),
              ),
            ),
          )
        : h("p.dim", "No Houses formed yet."),

      h("h4", `Worldseal Strain · ${ap.act_label} · ${ap.counter}/100${ap.countdown !== null ? ` · BLOWOUT COUNTDOWN: ${ap.countdown} TURNS` : ""}`),
      ap.recent_changes?.length ? h("p.dim", `Recent shifts: ${ap.recent_changes.join(" · ")}`) : null,
      ap.lifeboats?.length
        ? h(
            "div.cat-grid-2",
            ...ap.lifeboats.map((lb) =>
              h(
                "div.cat-slot.filled",
                h("strong", `${lb.kind}: ${lb.city_name}`),
                h("small.dim", lb.razed ? "RAZED" : `Held by ${lb.owner_name}`),
                !lb.razed && lb.x !== null
                  ? h("button", { onclick: () => (closeTag("cat-apoc"), emit("center", lb.x, lb.y)) }, "Show on Map")
                  : null,
              ),
            ),
          )
        : h("p.dim", "Lifeboats unlock when Worldseal Strain hits 100 (Caldera Keep rises from the sea + Siphon Spire + Great Cistern + Last Metropolis)."),
      h(
        "div.cat-crisis-strip",
        ...ap.crises.map((cr) =>
          h("span.cat-pill", { className: cr.fired ? "cat-pill warn" : "cat-pill", title: cr.summary }, `${cr.threshold}: ${cr.name}`),
        ),
      ),

      h("h4", "Rival Empires & Crusades"),
      h(
        "div.cat-grid-2",
        ...data.players
          .filter((p) => p.id !== meId && !p.is_neutral && p.alive)
          .map((p) =>
            h(
              "div.cat-house-card",
              portrait(p.leader_art, PORTRAIT.small, p.name),
              h(
                "div",
                h("strong", `${p.faction_name} (${p.name})`),
                h("small.dim", `${p.alignment}${p.religion ? ` · ${p.religion}` : ""} · ${p.cities} cities · ${p.squads} squads · ${(p.score / 100).toFixed(1)}%`),
                h(
                  "div.cat-actions-row",
                  p.war
                    ? h("button", { onclick: async () => (await catCmd({ c: "cat_peace", target: p.id })) && closeTag("cat-apoc") }, "Make Peace")
                    : h("button.danger", { onclick: async () => (await catCmd({ c: "cat_war", target: p.id })) && closeTag("cat-apoc") }, "Declare War"),
                  h("button", { onclick: async () => (await catCmd({ c: "cat_crusade", target: p.id })) && closeTag("cat-apoc") }, "Call Crusade"),
                ),
              ),
            ),
          ),
      ),
      data.crusades?.length
        ? h(
            "div.cat-row",
            h("strong", "Active Crusades: "),
            ...data.crusades.map((cr) =>
              h(
                "button",
                {
                  disabled: !ownSquads.length,
                  onclick: async () => {
                    if (await catCmd({ c: "cat_lend", crusade: cr.id, squad: ownSquads[0].id })) closeTag("cat-apoc");
                  },
                },
                `Crusade vs ${cr.target_name} (${cr.turns_left}t, ${cr.lent_count} lent) — Lend Squad`,
              ),
            ),
          )
        : null,
    ],
    buttons: [{ label: "Close", value: null }],
  });
}

// ---------------------------------------------------------------------------------------------
// 6. Cities List Screen (F1)

export function openCataclysmCities() {
  const data = cat();
  if (!data) return;
  const mine = data.cities.filter((c) => c.owner === S.view?.me?.player);
  openModal({
    title: "Your Macro Cities",
    tag: "cities",
    wide: true,
    body: [
      mine.length
        ? h(
            "table.scores",
            h("tr", h("th", "City"), h("th", "Size"), h("th", "Districts"), h("th", "Producing"), h("th", "Yields")),
            ...mine.map((c) =>
              h(
                "tr.clickable",
                { onclick: () => (closeTag("cities"), emit("center", c.x, c.y), openMacroCity(c.id)) },
                h("td", `${c.capital ? "★ " : ""}${c.name}${c.lifeboat ? " [LIFEBOAT]" : ""}`),
                h("td", String(c.size)),
                h("td", `${c.districts.length}/${c.max_district_slots}`),
                h("td", c.building ? `${c.building.label} (${c.building.turns}t)` : h("span.warn", "Idle")),
                h("td", `+${c.industry_rate}I +${c.gold_rate}G +${c.lore_rate}L +${c.culture_rate}C`),
              ),
            ),
          )
        : h("p.dim", "No cities."),
    ],
    buttons: [{ label: "Close", value: null }],
  });
}
