// The modal screens of the Cataclysm alternate ruleset (docs/CATACLYSM.md): the new game, a city,
// research (foundation, doctrines, creeds, rituals), the Court, the Houses, the Worldseal,
// diplomacy and the cities list. Every cost, effect and rule sentence comes from the view
// (`S.view.cataclysm`); the client only adds up numbers the view sends.

import * as api from "./api.js";
import { S, emit, scoreText } from "./state.js";
import { h, toast, errorText } from "./dom.js";
import { openModal, closeTag, ask, openTags } from "./modals.js";
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

function myId() {
  return S.view?.me?.player;
}

function portrait(key, size = PORTRAIT.small, title = "") {
  return leaderPortrait(key, { size, title }) ?? h("span.cat-portrait-fallback", String(title ?? "").slice(0, 2));
}

// ---------------------------------------------------------------------------------------------
// Small helpers.

const plural = (n, one, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
const signed = (n) => (n > 0 ? `+${n}` : `${n}`);
const has = (v) => v !== null && v !== undefined && v !== "";

/** A disabled control's reason, as visible text. */
function why(text) {
  return has(text) ? h("small.cat-why", text) : null;
}

/** A rich tooltip: a title and lines (empty ones dropped); null when there is nothing to say. */
function tipOf(title, ...lines) {
  const rest = lines.flat().filter(has);
  if (!rest.length) return null;
  return [title ? h("div.tip-title", title) : null, ...rest.map((l) => h("div", l))];
}

/** A list of bare sentences. */
function lines(list, cls = "ul.cat-list") {
  const items = (list ?? []).filter(has);
  return items.length ? h(cls, ...items.map((l) => h("li", l))) : null;
}

/** Waits until the view the client holds is newer than `before` (the command's reply names the
 *  version it made, when it does), or a timeout. */
function viewAfter(before, reply, ms = 2500) {
  const target = reply?.status?.view_version;
  const fresh = () => S.view !== before && (!has(target) || (S.view?.view_version ?? target) >= target);
  return new Promise((resolve) => {
    const t0 = Date.now();
    const tick = () => (fresh() || Date.now() - t0 > ms ? resolve() : setTimeout(tick, 40));
    tick();
  });
}

/** Asks before an irreversible or hostile action; resolves true on the action. */
async function confirm(title, text, label) {
  const body = (Array.isArray(text) ? text : [text]).filter(has);
  const v = await ask(title, body.length ? body : [title], [
    { label, value: true, danger: true },
    { label: "Cancel", value: null },
  ]);
  return v === true;
}

/** A Cataclysm modal whose body `build(ctx)` draws from the view. `ctx.run(cmd)` sends a command
 *  and, when it succeeds, redraws the body from the fresh view (scroll position kept);
 *  `ctx.redraw()` redraws at once (a tab, a select). */
function screen({ title, tag, build, wide = true, buttons, dismissible = true }) {
  let m = null;
  const ctx = {
    redraw() {
      if (!m || m.closed) return;
      const b = m.el.querySelector(".modal-body");
      const top = b?.scrollTop ?? 0;
      m.setBody(build(ctx));
      if (b) b.scrollTop = top;
    },
    // One command at a time: a second click while one is pending (the request, then the wait
    // for the fresh view) sends nothing.
    busy: false,
    async run(cmd) {
      if (ctx.busy) return null;
      ctx.busy = true;
      try {
        const before = S.view;
        const r = await catCmd(cmd);
        if (!r) return null;
        await viewAfter(before, r);
        if (openTags().includes(tag)) ctx.redraw();
        return r;
      } finally {
        ctx.busy = false;
      }
    },
    close() {
      m?.close(null);
    },
  };
  m = openModal({
    title,
    tag,
    wide,
    dismissible,
    className: "cat-modal",
    body: build(ctx),
    buttons: buttons ?? [{ label: "Close", value: null }],
  });
  // The focus starts on the dialog's own buttons (Close), not on the first control of the body:
  // a control with a rich tip would show it as soon as the dialog opens, over the cards.
  const btns = m.el.querySelectorAll(".modal-buttons button");
  const close = btns[btns.length - 1];
  if (close) {
    m.el.ownerDocument?.activeElement?.blur?.();
    close.focus({ preventScroll: true });
  }
  return m;
}

function playerOf(id) {
  return cat()?.players?.find((p) => p.id === id) ?? null;
}

function playerLabel(p) {
  if (!p) return "unknown";
  return p.faction_name && p.faction_name !== p.name ? `${p.faction_name} (${p.name})` : p.name ?? "unknown";
}

// ---------------------------------------------------------------------------------------------
// 1. New game

export async function openCataclysmNewGame({ noGame = false } = {}) {
  let setup;
  try {
    setup = await api.query("cataclysm_setup");
  } catch (e) {
    toast(errorText(e), "error");
    return;
  }
  const factions = setup.factions ?? [];
  let chosen = factions[0]?.key ?? null;
  let size = "standard";
  let opponents = 5;
  let seed = Math.floor(Math.random() * 90000) + 1000;

  const detailEl = h("div.cat-ng-detail");
  const gridEl = h("div.cat-ng-factions");

  function renderFactions() {
    gridEl.replaceChildren(
      ...factions.map((f) =>
        h(
          `button.cat-fac-card${f.key === chosen ? ".selected" : ""}`,
          {
            type: "button",
            "data-faction": f.key,
            "aria-pressed": String(f.key === chosen),
            onclick: () => {
              chosen = f.key;
              renderFactions();
            },
          },
          portrait(f.leader_art, PORTRAIT.small, f.leader_name),
          h("div.cat-fac-meta", h("strong", f.name), h("small.dim", `${f.leader_name} · ${f.alignment}`)),
        ),
      ),
    );
    const f = factions.find((x) => x.key === chosen) ?? factions[0];
    if (!f) {
      detailEl.replaceChildren();
      return;
    }
    const victory = (f.victory_lines ?? []).filter(has);
    detailEl.replaceChildren(
      h(
        "div.cat-fac-hero",
        portrait(f.leader_art, PORTRAIT.medium, f.leader_name),
        h(
          "div.cat-fac-text",
          h("div.cat-card-title", `${f.name} · ${f.leader_name}`),
          h("div", `Alignment: ${f.alignment}`),
          has(f.preferred_doctrine) ? h("div", `Preferred doctrine: ${f.preferred_doctrine}`) : null,
          has(f.lore_blurb) ? h("p.dim", f.lore_blurb) : null,
          has(f.rule_summary) ? h("p", f.rule_summary) : null,
          victory.length ? h("div.cat-sub", "Victory") : null,
          lines(victory),
        ),
      ),
    );
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
    ...[2, 3, 4, 5, 6, 7].map((n) => h("option", { value: String(n), selected: n === opponents }, plural(n, "rival"))),
  );
  const seedIn = h("input", { type: "number", value: String(seed), oninput: (ev) => (seed = Number(ev.target.value) || 1) });

  let starting = false;
  const doStart = async () => {
    if (starting || !chosen) return;
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

  const maxTurns = setup.max_turns ?? setup.turns ?? null;
  openModal({
    title: "Cataclysm · The Failing Worldseal",
    tag: "cataclysm-new",
    wide: true,
    dismissible: !noGame,
    className: "cat-modal",
    body: [
      h("p.dim", `Alternate ruleset: squads on a 3×3 grid, cities with districts, Great People${has(maxTurns) ? `, a ${maxTurns}-turn clock` : ""}.`),
      gridEl,
      detailEl,
      h("div.cat-ng-controls", h("label", "World ", sizeSel), h("label", "Opponents ", oppSel), h("label", "Seed ", seedIn)),
    ],
    buttons: [
      { label: "Start Cataclysm", id: "cat-start-btn", primary: true, onClick: doStart },
      { label: noGame ? "Back" : "Cancel", value: null },
    ],
  });
}

// ---------------------------------------------------------------------------------------------
// 2. City

function sentinelPills(c) {
  const list = c.sentinels ?? [];
  if (!list.length) return h("div.cat-row", h("span.cat-label", "Sentinels"), h("span.dim", "none"));
  return h(
    "div.cat-row",
    h("span.cat-label", "Sentinels"),
    ...list.map((s, i) => {
      const hp = typeof s === "number" ? s : s?.hp ?? 0;
      const breached = typeof s === "number" ? s <= 0 : !!s?.breached;
      return h(`span.cat-pill${breached ? ".warn" : ""}`, `${i + 1}: ${breached ? "breached" : `${hp}%`}`);
    }),
  );
}

function namesLine(label, list) {
  const names = (list ?? []).map((x) => x?.name ?? x).filter(has);
  return h("div", `${label}: ${names.length ? names.join(", ") : "none"}.`);
}

function foreignCityBody(c) {
  const owner = playerOf(c.owner);
  return [
    h(
      "div.cat-stats-strip",
      h("span", `Owner: ${playerLabel(owner)}`),
      h("span", `Size ${c.size}`),
      h("span", `Defense +${c.defense_pct ?? 0}%`),
      c.capital ? h("span", "Capital") : null,
      c.besieged ? h("span.warn", "Besieged") : null,
    ),
    has(c.lifeboat) ? h("div", `Lifeboat: ${c.lifeboat}.`) : null,
    sentinelPills(c),
    namesLine("Districts", c.districts),
    namesLine("Wonders", c.wonders),
  ];
}

/** The four quick-fill presets, by class keys in order of preference (the first unlocked one is
 *  taken; a preset falls back to the first unlocked class). */
const PRESETS = [
  { label: "Vanguard", prefs: [["phalanx", "champion", "axeman", "guard"], ["pikeman", "axeman", "guard"], ["longbowman", "archer", "guard"]] },
  { label: "Postrider column", prefs: [["knight", "horseman", "guard"], ["ranger", "assassin", "horseman", "guard"], ["archer", "guard"]] },
  { label: "Arcane battery", prefs: [["phalanx", "axeman", "guard"], ["archmage", "mage", "adept", "guard"], ["acolyte", "adept", "guard"]] },
  { label: "Siege train", prefs: [["phalanx", "axeman", "guard"], ["trebuchet", "catapult", "archer", "guard"], ["catapult", "archer", "guard"]] },
];

export function openMacroCity(cityId) {
  const first = cat()?.cities?.find((x) => x.id === cityId);
  if (!first) return;
  const foreign = !!first.foreign || first.owner !== myId();
  const title = `${first.name}${first.capital ? " · Capital" : ""}`;
  // The squad composer's picks, kept across redraws (null: not yet filled).
  let picks = null;
  let caravanDest = null;

  /** Sends a build order; asks first when it replaces production in progress. */
  async function build(ctx, c, fields, label) {
    const b = c.building;
    if (b && (b.progress ?? 0) > 0) {
      const ok = await confirm("Replace production", `${b.label} has ${b.progress}/${b.cost} Industry. A new order discards it.`, `Build ${label}`);
      if (!ok) return;
    }
    await ctx.run({ c: "cat_city_build", city: c.id, ...fields });
  }

  function ownBody(ctx, c, data) {
    const out = [];
    const food = `Food ${c.food_stored ?? 0}/${c.food_needed ?? 0}${has(c.grow_turns) ? `, grows in ${plural(c.grow_turns, "turn")}` : ""}`;
    out.push(
      h(
        "div.cat-stats-strip",
        h("span", `Size ${c.size}${has(c.max_size) ? `/${c.max_size}` : ""}`),
        h("span", food),
        h("span.cat-calc-num", { tip: tipOf("Industry", c.industry_calc) }, `Industry ${signed(c.industry_rate ?? 0)}/t`),
        h("span", `Gold ${signed(c.gold_rate ?? 0)}/t`),
        h("span", `Lore ${signed(c.lore_rate ?? 0)}/t`),
        h("span", `Culture ${c.culture ?? 0} (${signed(c.culture_rate ?? 0)}/t)`),
        h("span", `Defense +${c.defense_pct ?? 0}%`),
      ),
    );
    if (has(c.lifeboat)) out.push(h("div", `Lifeboat: ${c.lifeboat}.`));
    if (Array.isArray(c.notes)) {
      for (const n of c.notes) if (has(n?.text)) out.push(h(`p.${n.tone === "warn" ? "warn" : "dim"}`, n.text));
    } else {
      if (c.besieged) out.push(h("p.warn", "Besieged."));
      if ((c.stripped_turns ?? 0) > 0) out.push(h("p.warn", `Countryside stripped: ${plural(c.stripped_turns, "turn")} left.`));
    }
    const b = c.building;
    out.push(
      b
        ? h("div", `Production: ${b.label}, ${b.progress}/${b.cost} Industry, ${plural(b.turns, "turn")}.`)
        : h("div.warn", "Production: none."),
    );
    if (has(c.crew?.text)) out.push(h("div", c.crew.text));
    else if (c.crew_task) out.push(h("div", `Work Crew: ${c.crew_task.label}, ${c.crew_task.progress} of ${plural(c.crew_task.needed, "turn")}.`));
    out.push(sentinelPills(c));

    // Districts.
    const built = c.districts ?? [];
    const slots = c.max_district_slots ?? built.length;
    const slotEls = [];
    for (let i = 0; i < slots; i++) {
      const d = built[i];
      slotEls.push(d ? h("div.cat-slot.filled", h("strong", d.name)) : h("div.cat-slot.empty", h("span.dim", "Open slot")));
    }
    out.push(h("h4", "Districts"), h("div.cat-slots-row", ...slotEls));
    if (has(c.district_slot_note)) out.push(h("small.cat-note", c.district_slot_note));
    if (built.length < slots) {
      const builtKeys = new Set(built.map((d) => d.key));
      const options = (data.available_districts ?? []).filter((d) => !builtKeys.has(d.key));
      out.push(
        h("h4", "Build district"),
        h(
          "div.cat-grid-2",
          ...options.map((d) =>
            h(
              "button.cat-card-btn",
              { disabled: !d.unlocked, onclick: () => build(ctx, c, { kind: "district", key: d.key }, d.name) },
              h("div.cat-tech-head", h("strong", d.name), h("span", `${d.cost} Industry`)),
              has(d.summary) ? h("small.cat-note", d.summary) : null,
              d.unlocked ? null : why(d.why),
            ),
          ),
        ),
      );
    }

    // Raise squad.
    out.push(h("h4", "Raise squad"), squadComposer(ctx, c, data));

    // Lifeboat wonders.
    const wonders = data.available_wonders ?? [];
    if (wonders.length) {
      out.push(
        h("h4", "Lifeboat wonders"),
        h(
          "div.cat-grid-2",
          ...wonders.map((w) =>
            h(
              "button.cat-card-btn",
              { disabled: !w.unlocked, onclick: () => build(ctx, c, { kind: "wonder", key: w.key }, w.name) },
              h("div.cat-tech-head", h("strong", w.name), h("span", `${w.cost} Industry`)),
              has(w.summary) ? h("small.cat-note", w.summary) : null,
              w.unlocked ? null : why(w.why),
            ),
          ),
        ),
      );
    }

    // Sally.
    const help = data.sally_help ?? {};
    out.push(
      h("h4", "Sally"),
      h(
        "div.cat-btn-row",
        ...["never", "favoured", "always"].map((p) =>
          h(
            `button${c.sally === p ? ".primary" : ""}`,
            { "aria-pressed": String(c.sally === p), tip: tipOf(null, help[p]), onclick: () => c.sally !== p && ctx.run({ c: "cat_sally", city: c.id, posture: p }) },
            p[0].toUpperCase() + p.slice(1),
          ),
        ),
      ),
    );

    // Caravan.
    out.push(h("h4", "Caravan"), caravanControls(ctx, c, data));

    // Raze.
    if (c.raze?.allowed) {
      out.push(
        h("h4", "Raze"),
        h(
          "div.cat-btn-row",
          h(
            "button.danger",
            {
              onclick: async () => {
                if (!(await confirm("Raze city", [`Raze ${c.name}?`, c.raze.effect], "Raze city"))) return;
                // The city is gone: the screen closes.
                if (await catCmd({ c: "cat_raze", city: c.id })) ctx.close();
              },
            },
            "Raze city",
          ),
        ),
        has(c.raze.effect) ? h("small.cat-note", c.raze.effect) : null,
      );
    }
    return out;
  }

  function squadComposer(ctx, c, data) {
    const classes = (data.recruitable_classes ?? []).filter((u) => u.unlocked);
    const byKey = new Map(classes.map((u) => [u.key, u]));
    const maxM = data.squad_build?.max_members ?? PRESETS[0].prefs.length;
    const minM = Math.max(1, data.squad_build?.min_members ?? 1);
    const count = data.squad_count ?? 0;
    const cap = data.squad_cap ?? 0;
    const capFull = count >= cap;

    const resolve = (pr) => pr.prefs.slice(0, maxM).map((keys) => keys.find((k) => byKey.has(k)) ?? classes[0]?.key ?? "");
    const shown = [];
    const seen = new Set();
    for (const pr of PRESETS) {
      const ks = resolve(pr);
      const sig = ks.join(",");
      if (seen.has(sig)) continue;
      seen.add(sig);
      shown.push({ label: pr.label, keys: ks });
    }
    if (!picks || picks.length !== maxM) picks = shown[0] ? [...shown[0].keys] : Array(maxM).fill("");
    while (picks.length < maxM) picks.push("");
    // A pick no longer unlocked is dropped.
    picks = picks.map((k, i) => (byKey.has(k) ? k : i < minM ? classes[0]?.key ?? "" : ""));

    const chosen = picks.filter((k) => byKey.has(k));
    const cost = chosen.reduce((s, k) => s + (byKey.get(k)?.cost_industry ?? 0), 0);
    const rate = c.industry_rate ?? 0;
    const costLine = `Cost: ${cost} Industry${rate > 0 ? `, ${plural(Math.ceil(cost / rate), "turn")}` : ""}.`;

    const sels = picks.map((k, i) =>
      h(
        "select",
        {
          "aria-label": `Member ${i + 1}`,
          onchange: (ev) => {
            picks[i] = ev.target.value;
            ctx.redraw();
          },
        },
        i < minM ? null : h("option", { value: "", selected: !k }, "none"),
        ...classes.map((u) => h("option", { value: u.key, selected: u.key === k }, `${u.name} (${u.cost_industry} Industry)`)),
      ),
    );
    const reason = capFull ? `Squads at cap (${count}/${cap}).` : !classes.length ? "No class is unlocked." : chosen.length < minM ? `Choose at least ${plural(minM, "member")}.` : null;
    return h(
      "div.cat-composer",
      h("div", `Squads: ${count}/${cap}.`),
      shown.length && classes.length
        ? h(
            "div.cat-btn-row",
            ...shown.map((pr) =>
              h(
                "button",
                {
                  tip: tipOf(pr.label, pr.keys.map((k) => byKey.get(k)?.name ?? k).join(", ")),
                  onclick: () => {
                    picks = [...pr.keys];
                    ctx.redraw();
                  },
                },
                pr.label,
              ),
            ),
          )
        : null,
      h("div.cat-composer-sels", ...sels),
      h("div", costLine),
      h(
        "div.cat-btn-row",
        h(
          "button.primary",
          { disabled: !!reason, onclick: () => build(ctx, c, { kind: "squad", classes: chosen }, "squad") },
          "Raise squad",
        ),
      ),
      why(reason),
    );
  }

  function caravanControls(ctx, c, data) {
    const legacy = () =>
      (data.cities ?? [])
        .filter((o) => o.id !== c.id && !playerOf(o.owner)?.war)
        .map((o) => ({ id: o.id, name: o.name, owner_name: playerOf(o.owner)?.name ?? "" }));
    const dests = c.caravan_destinations ?? legacy();
    const cv = data.caravan ?? null;
    const count = data.caravan_count ?? 0;
    const cap = data.caravan_cap ?? 0;
    const canLaunch = cv ? !!cv.can_launch : count < cap;
    const reason = cv ? cv.why : canLaunch ? null : `Caravans at cap (${count}/${cap}).`;
    if (!dests.length) return h("div.cat-composer", h("div", `Caravans: ${count}/${cap}.`), h("div.dim", "No city a caravan can reach."));
    if (!dests.some((d) => d.id === caravanDest)) caravanDest = dests[0].id;
    const sel = h(
      "select",
      { "aria-label": "Destination", onchange: (ev) => (caravanDest = Number(ev.target.value)) },
      ...dests.map((d) => h("option", { value: String(d.id), selected: d.id === caravanDest }, has(d.owner_name) ? `${d.name} (${d.owner_name})` : d.name)),
    );
    return h(
      "div.cat-composer",
      h("div", `Caravans: ${count}/${cap}.`),
      h(
        "div.cat-btn-row",
        h("span", "Destination"),
        sel,
        h(
          "button",
          { disabled: !canLaunch, onclick: () => ctx.run({ c: "cat_caravan", origin: c.id, destination: caravanDest }) },
          has(cv?.cost) ? `Launch caravan (${cv.cost} Gold)` : "Launch caravan",
        ),
      ),
      canLaunch ? null : why(reason),
    );
  }

  return screen({
    title,
    tag: "cat-city",
    build: (ctx) => {
      const data = cat();
      const c = data?.cities?.find((x) => x.id === cityId);
      if (!c) return [h("p.dim", "This city is no longer in view.")];
      if (foreign || c.foreign || c.owner !== myId()) return foreignCityBody(c);
      return ownBody(ctx, c, data);
    },
  });
}

// ---------------------------------------------------------------------------------------------
// 3. Research: research, creeds, rituals

const TABS = [
  ["research", "Research"],
  ["creeds", "Creeds"],
  ["rituals", "Rituals"],
];

export function openResearch({ tab = "research" } = {}) {
  if (!cat()) return;
  let current = TABS.some(([k]) => k === tab) ? tab : "research";
  const titleOf = (k) => TABS.find(([key]) => key === k)[1];
  return screen({
    // The screen is named after its tab (the menu's Research, Creeds and Rituals open one each).
    title: titleOf(current),
    tag: "cat-research",
    build: (ctx) => {
      const data = cat();
      if (!data) return [];
      const tabs = h(
        "div.cat-tabs",
        { role: "tablist" },
        ...TABS.map(([k, label]) =>
          h(
            `button.cat-tab${k === current ? ".on" : ""}`,
            {
              role: "tab",
              "aria-selected": String(k === current),
              onclick: () => {
                if (current === k) return;
                current = k;
                ctx.redraw();
                const box = document.querySelector('.modal[data-tag="cat-research"]');
                const b = box?.querySelector(".modal-body");
                if (b) b.scrollTop = 0;
                const h2 = box?.querySelector(".modal-head h2");
                if (h2) h2.textContent = titleOf(k);
                box?.setAttribute("aria-label", titleOf(k));
              },
            },
            label,
          ),
        ),
      );
      const body = current === "creeds" ? creedsTab(ctx, data) : current === "rituals" ? ritualsTab(ctx, data) : researchTab(ctx, data);
      return [tabs, ...body];
    },
  });
}

function techNode(ctx, t, res) {
  const isCur = res.current?.id === t.id;
  const strain = t.ac_delta ? `${signed(t.ac_delta)} Strain` : null;
  const costText = [`${t.cost} Lore`, strain].filter(Boolean).join(" · ");
  const tp = tipOf(t.name, t.unlocks, t.cost_note, t.known ? null : t.why);
  if (t.known) {
    return h("div.cat-tech-node.known", { tip: tp }, h("div.cat-tech-head", h("strong", t.name), h("span.cat-check", "✓ known")), has(t.unlocks) ? h("small.cat-note", t.unlocks) : null);
  }
  if (isCur) {
    const cur = res.current;
    const pct = cur.cost > 0 ? Math.min(100, Math.round((cur.progress * 100) / cur.cost)) : 0;
    return h(
      "div.cat-tech-node.active",
      { tip: tp },
      h("div.cat-tech-head", h("strong", t.name), h("span", "researching")),
      h("div", `${cur.progress}/${cur.cost} Lore, ${plural(cur.turns, "turn")}`),
      h("div.cat-progress", h("div.cat-progress-fill", { style: { width: `${pct}%` } })),
      strain ? h("small.cat-note", strain) : null,
      has(t.unlocks) ? h("small.cat-note", t.unlocks) : null,
    );
  }
  if (t.can_research) {
    return h(
      "button.cat-tech-node.avail",
      { type: "button", tip: tp, onclick: () => ctx.run({ c: "cat_research", tech: t.id }) },
      h("div.cat-tech-head", h("strong", t.name), h("span", costText)),
      has(t.unlocks) ? h("small.cat-note", t.unlocks) : null,
    );
  }
  return h(
    "div.cat-tech-node.locked",
    { tip: tp },
    h("div.cat-tech-head", h("strong", t.name), h("span", costText)),
    has(t.unlocks) ? h("small.cat-note", t.unlocks) : null,
    why(t.why),
  );
}

function researchTab(ctx, data) {
  const res = data.research ?? {};
  const techs = res.techs ?? [];
  const cur = res.current;
  const rule = data.doctrine_rule ?? res.doctrine_rule ?? null;
  const out = [
    cur
      ? h("div", `Researching: ${cur.name}, ${cur.progress}/${cur.cost} Lore, ${plural(cur.turns, "turn")}.`)
      : has(res.idle_lore_text)
        ? h("div", res.idle_lore_text)
        : h("div.warn", "No research chosen."),
    h("div", `Primary doctrine: ${res.primary_doctrine_name ?? "none"}. Secondary doctrine: ${res.secondary_doctrine_name ?? "none"}.`),
    has(rule) ? h("small.cat-note", rule) : null,
    h("h4", "Foundation"),
    h("div.cat-grid-4", ...techs.filter((t) => !t.doctrine).map((t) => techNode(ctx, t, res))),
    h("h4", "Doctrines"),
  ];
  for (const doc of res.doctrines ?? []) {
    let adopt = null;
    if (!res.primary_doctrine) adopt = { secondary: false, label: "Adopt as primary", q: `Adopt ${doc.name} as your primary doctrine?` };
    else if (!res.secondary_doctrine && !doc.primary) adopt = { secondary: true, label: "Adopt as secondary", q: `Adopt ${doc.name} as your secondary doctrine?` };
    out.push(
      h(
        "div.cat-doc-row",
        h(
          "div.cat-doc-head",
          h("strong", doc.name),
          doc.primary ? h("span.cat-badge", "Primary") : doc.secondary ? h("span.cat-badge", "Secondary") : null,
          adopt
            ? h(
                "button",
                {
                  onclick: async () => {
                    if (!(await confirm("Adopt doctrine", [adopt.q, rule], adopt.label))) return;
                    await ctx.run({ c: "cat_doctrine", doctrine: doc.key, secondary: adopt.secondary });
                  },
                },
                adopt.label,
              )
            : null,
        ),
        has(doc.summary) ? h("small.cat-note", doc.summary) : null,
        h("div.cat-grid-4", ...techs.filter((t) => t.doctrine === doc.key).map((t) => techNode(ctx, t, res))),
      ),
    );
  }
  return out;
}

function creedsTab(ctx, data) {
  const out = [
    h("div", `Your creed: ${data.human_religion ?? "none"}. Alignment: ${data.human_alignment ?? "none"}.`),
    has(data.creed_rule) ? h("small.cat-note", data.creed_rule) : null,
  ];
  const cards = (data.religions ?? []).map((r) => {
    const action = r.action ?? (r.adopted ? "adopted" : r.founded_by ? "adopt" : "found");
    const state = action === "adopted" || r.adopted ? "Adopted" : has(r.founded_by) ? `Founded by ${r.founded_by}` : "Not founded";
    const canAct = r.can_act ?? action !== "adopted";
    const label = action === "found" ? "Found" : "Adopt";
    return h(
      `div.cat-card${action === "adopted" ? ".selected" : ""}`,
      h("div.cat-tech-head", h("strong", r.name), h("span", state)),
      h("div", `Alignment: ${r.alignment}`),
      has(r.summary) ? h("div", r.summary) : null,
      has(r.cost_text) ? h("div", `Cost: ${r.cost_text}`) : null,
      has(r.effect_text) ? h("div", r.effect_text) : null,
      action === "adopted"
        ? null
        : h(
            "div.cat-btn-row",
            h(
              "button",
              {
                disabled: !canAct,
                onclick: async () => {
                  if (!(await confirm(`${label} ${r.name}`, [r.effect_text, has(r.cost_text) ? `Cost: ${r.cost_text}` : null], label))) return;
                  await ctx.run({ c: "cat_religion", religion: r.id });
                },
              },
              label,
            ),
          ),
      canAct ? null : why(r.why),
    );
  });
  out.push(h("h4", "Creeds"), h("div.cat-grid-2", ...cards));
  return out;
}

function ritualsTab(ctx, data) {
  const cur = data.currencies ?? {};
  const mana = cur.mana ?? 0;
  const out = [h("div", `Mana ${mana} (${signed(cur.mana_rate ?? 0)}/t)`), h("h4", "Rituals")];
  out.push(
    h(
      "div.cat-grid-2",
      ...(data.rituals ?? []).map((rk) => {
        const reason = rk.why !== undefined ? rk.why : rk.can_cast ? null : `Need ${rk.mana_cost} Mana (have ${mana}).`;
        const can = rk.why !== undefined ? !has(rk.why) : !!rk.can_cast;
        return h(
          "div.cat-card",
          h("div.cat-tech-head", h("strong", rk.name), h("span", [`${rk.mana_cost} Mana`, rk.ac_delta ? `${signed(rk.ac_delta)} Strain` : null].filter(Boolean).join(" · "))),
          has(rk.summary) ? h("small.cat-note", rk.summary) : null,
          h("div.cat-btn-row", h("button", { disabled: !can, onclick: () => ctx.run({ c: "cat_ritual", ritual: rk.key }) }, "Cast")),
          can ? null : why(reason),
        );
      }),
    ),
  );
  return out;
}

// ---------------------------------------------------------------------------------------------
// 4. Court

export function openCourtScreen() {
  if (!cat()) return;
  const sel = new Map(); // gp id -> { squad, house }
  return screen({
    title: "Court",
    tag: "cat-court",
    build: (ctx) => {
      const data = cat();
      if (!data) return [];
      const rules = data.court_rules ?? {};
      const ren = data.renown ?? {};
      const me = myId();
      // The arrival rule and the sources of Renown: in the tip of the Renown line and the meters.
      const renownTip = tipOf("Renown", rules.arrival, ren.sources) ? () => tipOf("Renown", rules.arrival, ren.sources) : null;
      const out = [
        h("div", { tip: renownTip }, `Renown ${ren.total ?? 0}/${ren.next_threshold ?? 0}`),
        h("div.cat-stats-strip", { tip: renownTip }, ...(ren.meters ?? []).map((m) => h("span", `${m.suit} ${m.value}`))),
        h("h4", "Waiting in your Court"),
      ];
      const court = data.court ?? [];
      if (!court.length) out.push(h("div.dim", "None."));
      for (const gp of court) out.push(gpCard(ctx, gp, data, rules, me, sel));

      out.push(h("h4", "Secret Techs"));
      const known = data.secrets_known ?? [];
      out.push(
        known.length
          ? h(
              "div.cat-grid-2",
              ...known.map((s) =>
                h(
                  "div.cat-card",
                  h("div.cat-tech-head", h("strong", s.name), s.active ? h("span.good", "active") : h("span.warn", `Needs ${s.needs_name}`)),
                  has(s.summary) ? h("small.cat-note", s.summary) : null,
                ),
              ),
            )
          : h("div.dim", "None."),
      );
      const rec = data.recovering_heroes ?? [];
      if (rec.length) out.push(h("div", `Recovering heroes: ${rec.map((r) => `${r.name} (${plural(r.turns_left, "turn")}, ${plural(r.wounds, "Wound")})`).join(", ")}.`));
      return out;
    },
  });
}

function gpCard(ctx, gp, data, rules, me, sel) {
  const st = sel.get(gp.id) ?? {};
  sel.set(gp.id, st);
  const squads = gp.hero_squads ?? (data.squads ?? []).filter((s) => s.owner === me && s.places_used < s.place_cap).map((s) => ({ id: s.id, name: s.name }));
  const houses = gp.lord_houses ?? (data.houses ?? []).map((ho) => ({ id: ho.id, name: ho.name, lord_name: ho.lord_name }));
  if (!squads.some((s) => s.id === st.squad)) st.squad = squads[0]?.id ?? null;
  if (!houses.some((x) => x.id === st.house)) st.house = houses[0]?.id ?? null;
  const who = `${gp.name}, Great ${gp.suit}`;
  const use = async (action, label, effect, extra = {}) => {
    if (!(await confirm(who, [effect], label))) return;
    await ctx.run({ c: "cat_gp", gp: gp.id, action, ...extra });
  };
  const sec = gp.secret_tech;
  const choice = (control, ...notes) => h("div.cat-choice", h("div.cat-btn-row", control), ...notes.filter(has).map((n) => h("small.cat-note", n)));

  const squadSel = squads.length
    ? h("select", { "aria-label": "Squad", onchange: (ev) => (st.squad = Number(ev.target.value)) }, ...squads.map((s) => h("option", { value: String(s.id), selected: s.id === st.squad }, s.name)))
    : null;
  const houseSel = houses.length
    ? h(
        "select",
        { "aria-label": "House", onchange: (ev) => (st.house = Number(ev.target.value)) },
        ...houses.map((x) => h("option", { value: String(x.id), selected: x.id === st.house }, has(x.lord_name) ? `${x.name} (Lord ${x.lord_name})` : x.name)),
      )
    : null;

  return h(
    "div.cat-gp-card",
    h(
      "div.cat-gp-top",
      portrait(gp.portrait, PORTRAIT.medium, gp.name),
      h(
        "div.cat-gp-text",
        h("div.cat-card-title", who),
        sec
          ? h(
              "div.cat-secret-box",
              h("strong", `Secret Tech: ${sec.name}`),
              has(sec.summary) ? h("div", sec.summary) : null,
              h("small.cat-note", `Requires ${sec.needs_name} (${sec.needs_known ? "known" : "not researched"})`),
            )
          : h("div.dim", "No Secret Tech left in this suit."),
      ),
    ),
    choice(
      [h("button.primary", { disabled: !sec, onclick: () => use("secret", sec ? `Discover ${sec.name}` : "Discover", rules.secret) }, sec ? `Discover ${sec.name}` : "Discover"), sec ? null : why("No Secret Tech left in this suit.")],
      rules.secret,
    ),
    choice(h("button", { onclick: () => use("burst", "Tempo burst", [gp.burst_summary, gp.burst_note].filter(has).join(" ")) }, "Tempo burst"), gp.burst_summary, gp.burst_note, rules.burst),
    choice(
      [
        h("span", `Commission as ${gp.hero_class_name ?? "a"} hero in`),
        squadSel,
        h("button", { disabled: !squads.length, onclick: () => use("hero", "Commission", gp.hero_effect ?? rules.hero, { squad: st.squad }) }, "Commission"),
        squads.length ? null : why("No squad has a free place."),
      ],
      gp.hero_effect ?? rules.hero,
    ),
    choice(
      [
        h("span", "Appoint Lord of"),
        houseSel,
        h("button", { disabled: !houses.length, onclick: () => use("lord", "Appoint", gp.lord_effect ?? rules.lord, { house: st.house }) }, "Appoint"),
        houses.length ? null : why("No House to appoint to."),
      ],
      gp.lord_effect ?? rules.lord,
    ),
  );
}

// ---------------------------------------------------------------------------------------------
// 5. Houses

export function openHouses() {
  if (!cat()) return;
  return screen({
    title: "Houses",
    tag: "cat-houses",
    build: () => {
      const houses = cat()?.houses ?? [];
      if (!houses.length) return [h("div.dim", "No Houses.")];
      return [
        h("h4", "Houses"),
        h(
          "div.cat-grid-2",
          ...houses.map((ho) => {
            const calc = ho.loyalty_calc ?? [];
            return h(
              "div.cat-house-card",
              { tip: tipOf("Loyalty", calc) },
              portrait(ho.lord_portrait, PORTRAIT.small, ho.lord_name),
              h(
                "div",
                h("div.cat-card-title", `${ho.name} · Lord ${ho.lord_name}`),
                h("div", `Mood: ${ho.mood} (${signed(ho.loyalty ?? 0)})`),
                h("div", `Output ${ho.output_pct}%`),
                h("div", `Perk: ${ho.perk}`),
                ho.perk_active === false ? h("small.warn", `Perk inactive (${ho.mood})`) : null,
                h("small.cat-note", `Cities: ${(ho.cities ?? []).join(", ") || "none"}`),
                has(ho.sympathy_name) ? h("small.cat-note", `Sympathy: ${ho.sympathy_name}`) : null,
                lines(calc, "ul.cat-list.cat-calc"),
              ),
            );
          }),
        ),
      ];
    },
  });
}

// ---------------------------------------------------------------------------------------------
// 6. Worldseal

export function openWorldseal() {
  if (!cat()) return;
  return screen({
    title: "Worldseal",
    tag: "cat-worldseal",
    build: (ctx) => {
      const data = cat();
      if (!data) return [];
      const ap = data.apocalypse ?? {};
      const strainMax = ap.max ?? ap.counter_max ?? null;
      const out = [
        h("div", [`Turn ${data.turn}${has(data.max_turns) ? ` of ${data.max_turns}` : ""}`, ap.act_label ?? data.act_label].filter(has).join(" · ")),
        h("div", `Strain ${ap.counter ?? 0}${has(strainMax) ? `/${strainMax}` : ""}`),
        has(ap.countdown) ? h("div.warn", `Blowout in ${plural(ap.countdown, "turn")}.`) : null,
      ];
      const vic = data.victory ?? {};
      if ((vic.lines ?? []).length || has(vic.status)) out.push(h("h4", "Victory"), lines(vic.lines), has(vic.status) ? h("div", vic.status) : null);

      out.push(h("h4", "Lifeboats"));
      const boats = ap.lifeboats ?? [];
      if (boats.length) {
        out.push(
          h(
            "div.cat-grid-2",
            ...boats.map((lb) =>
              h(
                "div.cat-card",
                h("div.cat-tech-head", h("strong", lb.kind), lb.razed ? h("span.warn", "razed") : null),
                h("div", lb.city_name),
                lb.razed ? null : h("div", `Held by ${lb.owner_name ?? "none"}`),
                !lb.razed && has(lb.x) && has(lb.y)
                  ? h("div.cat-btn-row", h("button", { onclick: () => (ctx.close(), emit("center", lb.x, lb.y)) }, "Show on map"))
                  : null,
              ),
            ),
          ),
        );
      } else out.push(lines(ap.lifeboat_rules) ?? h("div.dim", "None."));

      out.push(h("h4", "Strain"), lines(ap.rules));
      const recent = (ap.recent_changes ?? []).slice(0, 6);
      if (recent.length) out.push(h("div.cat-sub", "Recent changes"), lines(recent, "ul.cat-list.cat-calc"));

      const crises = ap.crises ?? [];
      if (crises.length) {
        out.push(
          h("h4", "Crises"),
          h(
            "ul.cat-crisis-list",
            ...crises.map((cr) =>
              h(
                `li${cr.fired ? ".fired" : ""}`,
                h("span.cat-crisis-at", String(cr.threshold)),
                h("div", h("strong", cr.name), cr.fired ? h("span.cat-badge.warn", "fired") : null, has(cr.summary) ? h("div.cat-note", cr.summary) : null),
              ),
            ),
          ),
        );
      }
      return out;
    },
  });
}

// ---------------------------------------------------------------------------------------------
// 7. Diplomacy

export function openDiplomacy() {
  if (!cat()) return;
  const lendSel = new Map(); // crusade id -> squad id
  return screen({
    title: "Diplomacy",
    tag: "cat-diplo",
    build: (ctx) => {
      const data = cat();
      if (!data) return [];
      const me = myId();
      const rivals = (data.players ?? []).filter((p) => p.id !== me && !p.is_neutral && p.alive);
      const canCall = data.can_call_crusade ?? true;
      const out = [
        h("h4", "Rival empires"),
        has(data.war_effect) ? h("small.cat-note", `Declaring war: ${data.war_effect}`) : null,
        has(data.peace_effect) ? h("small.cat-note", `Making peace: ${data.peace_effect}`) : null,
        canCall || !has(data.call_crusade_why) ? null : why(`Call crusade: ${data.call_crusade_why}`),
      ];
      if (!rivals.length) out.push(h("div.dim", "None."));
      else
        out.push(
          h(
            "div.cat-grid-2",
            ...rivals.map((p) =>
              h(
                "div.cat-house-card",
                portrait(p.leader_art, PORTRAIT.small, p.name),
                h(
                  "div",
                  h("div.cat-card-title", playerLabel(p)),
                  h("div", `Alignment: ${p.alignment}`),
                  h("div", `Creed: ${p.religion ?? "none"}`),
                  h("div", `${plural(p.cities ?? 0, "city", "cities")} · ${plural(p.squads ?? 0, "squad")}`),
                  h("div", `Score share ${scoreText(p.score)}`),
                  p.war ? h("div.warn", "At war") : h("div", "At peace"),
                  ...(p.peace_lines ?? []).map((l) => h("div", l)),
                  h(
                    "div.cat-btn-row",
                    p.war
                      ? h("button", { disabled: p.can_peace === false, onclick: () => ctx.run({ c: "cat_peace", target: p.id }) }, p.peace_label ?? "Offer peace")
                      : h(
                          "button.danger",
                          {
                            disabled: p.can_declare_war === false,
                            onclick: async () => {
                              if (!(await confirm("Declare war", [`Declare war on ${playerLabel(p)}?`, data.war_effect], "Declare war"))) return;
                              await ctx.run({ c: "cat_war", target: p.id });
                            },
                          },
                          "Declare war",
                        ),
                    h(
                      "button",
                      {
                        disabled: !canCall || p.can_call_crusade === false,
                        onclick: async () => {
                          if (!(await confirm("Call crusade", [`Call a crusade against ${playerLabel(p)}?`, data.crusade_effect], "Call crusade"))) return;
                          await ctx.run({ c: "cat_crusade", target: p.id });
                        },
                      },
                      "Call crusade",
                    ),
                  ),
                  p.war && p.can_peace === false ? why(p.peace_why) : null,
                  !p.war && p.can_declare_war === false ? why(p.war_why) : null,
                  canCall && p.can_call_crusade === false ? why(p.call_crusade_why) : null,
                ),
              ),
            ),
          ),
        );

      out.push(h("h4", "Crusades"));
      const crusades = data.crusades ?? [];
      if (!crusades.length) out.push(h("div.dim", "None."));
      const mine = (data.squads ?? []).filter((s) => s.owner === me && !has(s.lent_by));
      for (const cr of crusades) {
        const againstMe = cr.against_me ?? cr.target_id === me;
        const row = [h("div", `Crusade against ${cr.target_name}, called by ${cr.caller_name}, ${plural(cr.turns_left, "turn")} left, ${plural(cr.lent_count ?? 0, "squad")} lent.`)];
        if (!againstMe) {
          if (!mine.some((s) => s.id === lendSel.get(cr.id))) lendSel.set(cr.id, mine[0]?.id ?? null);
          const canLend = (cr.can_lend ?? true) && mine.length > 0;
          const reason = cr.can_lend === false ? cr.lend_why : mine.length ? null : "No squad to lend.";
          row.push(
            h(
              "div.cat-btn-row",
              mine.length
                ? h("select", { "aria-label": "Squad", onchange: (ev) => lendSel.set(cr.id, Number(ev.target.value)) }, ...mine.map((s) => h("option", { value: String(s.id), selected: s.id === lendSel.get(cr.id) }, s.name)))
                : null,
              h(
                "button",
                {
                  disabled: !canLend,
                  onclick: async () => {
                    if (cr.lend_declares_war && !(await confirm("Lend squad", `Lending a squad declares war on ${cr.target_name}.`, "Lend"))) return;
                    await ctx.run({ c: "cat_lend", crusade: cr.id, squad: lendSel.get(cr.id) });
                  },
                },
                "Lend",
              ),
            ),
            canLend ? null : why(reason),
          );
        }
        out.push(h("div.cat-card", ...row));
      }
      return out;
    },
  });
}

// ---------------------------------------------------------------------------------------------
// 8. Cities list

export function openCataclysmCities() {
  if (!cat()) return;
  return screen({
    title: "Cities",
    tag: "cities",
    build: (ctx) => {
      const data = cat();
      if (!data) return [];
      const me = myId();
      const mine = (data.cities ?? []).filter((c) => c.owner === me && !c.foreign);
      const out = [];
      out.push(
        mine.length
          ? h(
              "table.scores.cat-table",
              h("tr", h("th", "City"), h("th.num", "Size"), h("th.num", "Districts"), h("th", "Production"), h("th.num", "Industry"), h("th.num", "Gold"), h("th.num", "Lore"), h("th.num", "Culture")),
              ...mine.map((c) =>
                h(
                  "tr.clickable",
                  { onclick: () => (ctx.close(), emit("center", c.x, c.y), openMacroCity(c.id)) },
                  h("td", c.name, c.capital ? h("small.dim", " capital") : null, has(c.lifeboat) ? h("small.dim", ` · Lifeboat (${c.lifeboat})`) : null),
                  h("td.num", String(c.size)),
                  h("td.num", `${(c.districts ?? []).length}/${c.max_district_slots ?? 0}`),
                  h("td", c.building ? `${c.building.label}, ${plural(c.building.turns, "turn")}` : h("span.warn", "none")),
                  h("td.num", signed(c.industry_rate ?? 0)),
                  h("td.num", signed(c.gold_rate ?? 0)),
                  h("td.num", signed(c.lore_rate ?? 0)),
                  h("td.num", signed(c.culture_rate ?? 0)),
                ),
              ),
            )
          : h("div.dim", "No cities."),
      );
      const caravans = data.caravans ?? [];
      out.push(h("h4", "Caravans"), h("div", `Caravans ${data.caravan_count ?? 0}/${data.caravan_cap ?? 0}`));
      if (caravans.length) {
        out.push(
          h(
            "table.scores.cat-table",
            h("tr", h("th", "Route"), h("th.num", "Gold"), h("th", "State")),
            ...caravans.map((cv) =>
              h(
                "tr",
                h("td", `${cv.origin_name} → ${cv.dest_name}`, cv.owner !== me ? h("small.dim", " (incoming)") : null),
                h("td.num", `${signed(cv.gold_rate ?? 0)} Gold/t`),
                h("td", (cv.reform_turns ?? 0) > 0 ? `re-forming, ${plural(cv.reform_turns, "turn")}` : "running"),
              ),
            ),
          ),
        );
      }
      return out;
    },
  });
}
