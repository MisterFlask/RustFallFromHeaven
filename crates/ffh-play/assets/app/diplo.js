// Diplomacy (design §5.1 "Diplomacy minimum", §7 P6; Civ4's foreign advisor, F4): the players you
// met (leader, civilization, war or peace, open borders, pacts, vassals, attitude, score), your
// deals with them, declaring war (`declare_war`), and the trade screen: what each side can give
// (the `trade` query, C++ `buildTradeTable`, refusals greyed), a deal put together from both
// tables and offered (`propose_deal`), which the AI accepts or declines at once; each item shows
// the points the AI puts on it, and under the tables the totals and the answer the proposal will
// get (the `trade_answer` query), both kept bare (CLAUDE.md, "Rules text and tooltips"); a gift
// (only your side) or a demand (only theirs) says what it does (ffh-core `tribute`), and your
// standing deals can be cancelled after their first turns (`cancel_deal`). Each leader shows
// their portrait (portraits.js).
//
// Leaders speak as in Civ4's diplomacy screen (`speech` query, view/speech.rs: FfH2's lines for
// each leader, by attitude): the trade screen opens with the leader's greeting (or its refusal to
// talk) and answers each proposal (accepted: thanks for a gift, peace, or a deal welcomed;
// declined: no peace, a demand rejected, or a deal refused). The moments when a leader contacts
// you (first contact, a war declared on you) open the leader screen on their own (the view's
// `moments_next`, `moments` query).

import * as api from "./api.js";
import { S, dname, tname, teamName, tradeItemLabel, tradeRefusal, DUAL_ITEMS, me, scoreText, fairShareText } from "./state.js";
import { h } from "./dom.js";
import { openModal, ask } from "./modals.js";
import { infoTip, partsList } from "./infotips.js";
import { render as markup } from "./markup.js";
import { run } from "./commands.js";
import { openTroops, troopsLabel } from "./crusade.js";
import { leaderPortrait, playerPortrait, PORTRAIT } from "./portraits.js";

/** The players screen. */
export async function openDiplomacy() {
  const m = openModal({ title: "Foreign affairs", body: [h("p.dim", "…")], tag: "diplomacy", wide: true, buttons: [{ label: "Close", value: null }] });
  const load = async () => {
    let r;
    try {
      r = await api.query("players");
    } catch (e) {
      return m.setBody(h("p.warn", e.detail ?? e.message));
    }
    if (m.closed) return;
    const rows = r.players
      .filter((p) => !p.barbarian)
      .map((p) => {
        const relation = [p.crusade_host ? h("b", "a crusade's Host") : null, p.war ? h("span.warn", "war") : "peace", p.open_borders ? "open borders" : null, p.defensive_pact ? "defensive pact" : null, p.vassal_of_us ? "your vassal" : null, p.our_master ? "your master" : null].filter(Boolean);
        // Your deals with them; one may be cancelled after its first turns (a peace treaty ends then).
        const deals = p.deals.map((d) => {
          const text = `${d.we_give.map(tradeItemLabel).join(", ") || "nothing"} ⇄ ${d.they_give.map(tradeItemLabel).join(", ") || "nothing"} (since turn ${d.turn})`;
          const peace = [...d.we_give, ...d.they_give].some((it) => tname("TradeableItem", it.item) === "TRADE_PEACE_TREATY");
          const action = d.cancelable
            ? h("button.link.small", { "data-cancel-deal": d.deal, onclick: () => cancelDeal(text, d.deal).then(load) }, "Cancel")
            : d.cancel_in > 0
              ? h("span", peace ? ` (ends in ${d.cancel_in} turns)` : ` (cancel in ${d.cancel_in} turns)`)
              : null;
          return h("div", { "data-deal": d.deal }, text, " ", action);
        });
        return h(
          "tr",
          { class: p.alive ? "" : "dead", "data-player": p.id },
          h("td", { tip: infoTip("LeaderHeadType", p.leader, { title: p.name }) }, h("div.leader-cell", leaderPortrait(p.leader, { size: PORTRAIT.medium }), h("div", h("b", p.name), h("div.dim", `${dname("CivilizationType", p.civ)} · team ${p.team}`)))),
          h("td", relation.flatMap((x, i) => (i ? [", ", x] : [x]))),
          h("td", { tip: p.attitude !== null ? () => attitudeTip(p) : null }, p.attitude !== null ? dname("AttitudeType", p.attitude) : "-"),
          h("td", p.alignment !== null && p.alignment !== undefined ? dname("AlignmentType", p.alignment) : h("span.dim", "-"), h("div.dim", p.state_religion !== null && p.state_religion !== undefined ? dname("ReligionType", p.state_religion) : "no state religion")),
          h("td.num", scoreText(p.score)),
          h(
            "td.actions",
            p.alive && p.contact
              ? h("button.small", { "data-trade": p.id, onclick: () => openTrade(p.id, p.name).then(load) }, "Trade…")
              : p.crusade_host && p.alive
                ? h("span.dim", { title: "A crusade's Host fights its crusade only: see the Crusades advisor" }, "crusade")
                : h("span.dim", !p.alive ? "dead" : "no contact"),
            p.can_declare_war && !p.war ? h("button.small.danger", { "data-war": p.team, onclick: () => declareWar(p).then(load) }, "Declare war") : null,
          ),
          deals.length ? h("td.dim", deals) : h("td"),
        );
      });
    const my = me();
    const you = my
      ? h(
          "p.dim",
          `You: ${my.alignment !== null && my.alignment !== undefined ? dname("AlignmentType", my.alignment) : "no alignment"}, ${my.state_religion !== null && my.state_religion !== undefined ? `${dname("ReligionType", my.state_religion)} as state religion` : "no state religion"}; score ${scoreText(my.score)}${my.civs_alive ? ` (a fair share is ${fairShareText(my.civs_alive)})` : ""}. Point at an attitude to see its reasons.`,
        )
      : null;
    m.setBody(
      you,
      rows.length
        ? h("table.scores.diplo", h("tr", h("th", "Leader"), h("th", "Relations"), h("th", "Attitude"), h("th", "Alignment"), h("th", "Score"), h("th", ""), h("th", "Deals")), rows)
        : h("p.dim", "You have not met anyone yet."),
    );
  };
  load();
  return m.done;
}

/** Why player `p` feels as it does about you (the `players` query's `attitude_parts`, which sum to
 *  its `attitude_score`), and what the next better attitude takes. */
function attitudeTip(p) {
  const parts = p.attitude_parts ?? [];
  const sign = (n) => (n > 0 ? `+${n}` : `${n}`);
  return [
    h("div.tip-title", `${p.name}: ${dname("AttitudeType", p.attitude)} towards you`),
    partsList([{ rows: parts }]) ?? h("div.dim", "Nothing for or against you."),
    p.attitude_score !== null && p.attitude_score !== undefined ? h("div.tip-hint", `Total ${sign(p.attitude_score)}${p.attitude_next !== null && p.attitude_next !== undefined ? `; at ${sign(p.attitude_next)} they would be ${dname("AttitudeType", p.attitude + 1)}` : ""}.`) : null,
  ];
}

/** The human's side of the conversation: Civ4's USER_DIPLO_* lines. */
const SAY = {
  trade: "Let's make a deal…",
  exit: "Farewell ... (Exit)",
  warResponse: "So be it.",
};

/** What player `p`'s leader says (the `speech` query): for `comment` (an AI_DIPLOCOMMENT_* name),
 *  or with none what it says when you contact it, its greeting or its refusal to talk. Returns
 *  `{comment, text}`, or null when the query fails. */
export async function speech(p, comment = null) {
  try {
    return await api.query("speech", { with: p, comment, salt: Math.floor(Math.random() * 2 ** 31) });
  } catch {
    return null;
  }
}

/** A leader speaking: the large portrait, name, civilization and mood, and the line in quotes;
 *  `say(text)` changes the line. */
export function leaderHead(p, text) {
  const pl = S.players.get(p);
  const quote = h("blockquote.leader-says", text ?? "…");
  const mood = pl && pl.attitude !== null && pl.attitude !== undefined ? ` · ${dname("AttitudeType", pl.attitude)}` : "";
  const el = h(
    "div.leader-head",
    { "data-player": p },
    playerPortrait(p, { size: PORTRAIT.large }),
    h("div.leader-words", h("h3", pl?.name ?? `Player ${p}`), h("div.dim", `${pl ? dname("CivilizationType", pl.civ) : ""}${mood}`), quote),
  );
  return {
    el,
    say(t) {
      quote.textContent = t ?? "…";
    },
  };
}

/** The leader screen: player `p`'s leader says `text` and you answer with one of `replies`
 *  (`[{label, value}]`, Civ4's list of answers). Resolves with the chosen value (null: closed). */
export function openLeader(p, text, replies, { comment = null } = {}) {
  const head = leaderHead(p, text);
  const m = openModal({
    title: S.players.get(p)?.name ?? `Player ${p}`,
    body: [head.el, h("div.options.replies", replies.map((r) => h("button.option.reply", { "data-reply": r.value, onclick: () => m.close(r.value) }, r.label)))],
    tag: "leader",
    className: "leader",
    buttons: [],
  });
  if (comment) m.el.dataset.comment = comment;
  return m.done;
}

// ---- leader moments ---------------------------------------------------------------------------

/** The next moment index this page has not shown (null: none seen yet in this game). */
let momentsNext = null;
let momentsRunning = false;
let momentsChecking = false;
let momentsAgain = false;
/** Bumped by another game: a reply for the last game is dropped. */
let momentsGame = 0;
const momentQueue = [];

/** Another game: its moments start afresh. */
export function resetMoments() {
  momentsNext = null;
  momentsAgain = false;
  momentsGame++;
  momentQueue.length = 0;
}

/** Shows the leader moments the view announces (`moments_next`). A page that opens a game shows
 *  only the moments of this turn and the last (a reload shows them again, as the turn report);
 *  older ones are history. */
export async function checkMoments() {
  // One request at a time: a view that arrives meanwhile asks again after it.
  if (momentsChecking) {
    momentsAgain = true;
    return;
  }
  momentsChecking = true;
  try {
    do {
      momentsAgain = false;
      const next = S.view?.moments_next;
      if (next === null || next === undefined) return;
      if (momentsNext !== null && next < momentsNext) momentsNext = 0;
      if (next === momentsNext) continue;
      const first = momentsNext === null;
      const game = momentsGame;
      let r;
      try {
        r = await api.query("moments", { since: first ? 0 : momentsNext });
      } catch {
        return; /* busy inside a decision: the next view asks again */
      }
      if (game !== momentsGame) {
        momentsAgain = true;
        continue;
      }
      momentsNext = r.next;
      const turn = S.view?.turn ?? 0;
      momentQueue.push(...r.entries.filter((e) => !first || e.t >= turn - 1));
      runMoments();
    } while (momentsAgain);
  } finally {
    momentsChecking = false;
  }
}

async function runMoments() {
  if (momentsRunning) return;
  momentsRunning = true;
  try {
    while (momentQueue.length) {
      const e = momentQueue.shift();
      const comment = tname("DiploCommentType", e.comment);
      if (comment === "AI_DIPLOCOMMENT_DECLARE_WAR") {
        await openLeader(e.player, e.text, [{ label: SAY.warResponse, value: "ok" }], { comment });
      } else {
        const v = await openLeader(e.player, e.text, [{ label: SAY.trade, value: "trade" }, { label: SAY.exit, value: "exit" }], { comment });
        if (v === "trade") await openTrade(e.player, null, { opening: e.text });
      }
    }
  } finally {
    momentsRunning = false;
  }
}

/** The tooltip of a trade item: what a tech brings you, what a resource, civic or religion
 *  does; null for the others. */
function itemTip(it, techHelp) {
  const type = tname("TradeableItem", it.item);
  const head = h("div.tip-title", tradeItemLabel(it));
  if (type === "TRADE_TECHNOLOGIES") {
    const help = techHelp.get(it.data);
    return help ? [head, h("div.tip-effects", markup(help))] : null;
  }
  const kind = { TRADE_RESOURCES: "BonusType", TRADE_CIVIC: "CivicType", TRADE_RELIGION: "ReligionType" }[type];
  return kind ? infoTip(kind, it.data) : null;
}

async function declareWar(p) {
  const v = await ask("Declare war?", `Declare war on ${teamName(p.team)}? Their allies may join them.`, [
    { label: "Declare war", value: true, primary: true },
    { label: "Cancel", value: null },
  ]);
  if (v) await run({ c: "declare_war", team: p.team });
}

/** Cancels a standing deal (`cancel_deal`), after asking. */
async function cancelDeal(text, deal) {
  const v = await ask("Cancel deal?", `${text}. Cancelling ends every item of the deal now.`, [
    { label: "Cancel deal", value: true, primary: true },
    { label: "Keep", value: null },
  ]);
  if (v) await run({ c: "cancel_deal", deal });
}

/** The trade screen with player `with`: both trade tables; clicking an item puts it into the
 *  deal (an item that goes on both sides, such as a peace treaty, goes on both, and is greyed on
 *  both when either side refuses it), gold as an amount; Propose offers it. */
export async function openTrade(withPlayer, name = null, { opening = null } = {}) {
  const m = openModal({ title: `Trade with ${name ?? S.players.get(withPlayer)?.name ?? `player ${withPlayer}`}`, body: [h("p.dim", "…")], tag: "trade", wide: true, buttons: [{ label: "Close", value: null }] });
  let t;
  try {
    t = await api.query("trade", { with: withPlayer });
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  // The leader greets you, or refuses to talk (Civ4 opens the screen on its line).
  const leader = t.contact || /refuses to talk/.test(t.why_not ?? "") ? leaderHead(withPlayer, opening) : null;
  if (leader && !opening) speech(withPlayer).then((r) => !m.closed && leader.say(r?.text));
  if (!t.contact) {
    const pl = S.players.get(withPlayer);
    m.setBody(leader?.el, h("p.warn", leader && pl ? `${pl.name} refuses to talk.` : (t.why_not ?? "No contact.")));
    return m.done;
  }
  // What a tech brings you (the techs query's help, in your context) for the techs on offer.
  let techHelp = new Map();
  try {
    techHelp = new Map((await api.query("techs")).techs.map((x) => [x.tech, x.help]));
  } catch {
    /* the tips go without */
  }
  // The deal: per side, the chosen items by key (gold: the amount typed); `troops`, a loan of
  // troops (the crusade ruleset, docs/CRUSADE.md: `{lend, target, mission, duration, strength,
  // units}`, lent by you when `lend`), at most one.
  const deal = { give: new Map(), get: new Map(), troops: null };
  const keyOf = (it) => `${it.item}:${it.data}`;
  const isDual = (it) => DUAL_ITEMS.includes(tname("TradeableItem", it.item));
  const isGold = (it) => ["TRADE_GOLD", "TRADE_GOLD_PER_TURN"].includes(tname("TradeableItem", it.item));
  const status = h("p.dim");
  // Values (the trade query's `value`: the points the leader puts on an item, + for what it gets,
  // - for what it gives; gold and gold per turn per piece; null for no value) and the summary of
  // the deal, with the answer the leader would give (the `trade_answer` query).
  const hasValue = (it) => it.value !== null && it.value !== undefined;
  const points = (it, amount) => (!hasValue(it) ? 0 : isGold(it) ? it.value * amount : it.value);
  const signed = (n) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : "0");
  const valueText = (it, sel) => (!hasValue(it) ? (isDual(it) ? "swap" : "no value") : isGold(it) && !sel ? `${signed(it.value)} each` : signed(points(it, sel?.data ?? 1)));
  const valueTip = (it) => {
    if (!hasValue(it)) return isDual(it) ? "No value. Traded only for the same item." : "No value. Adds 0 to the deal.";
    const v = Math.abs(it.value);
    const turns = it.value_turns;
    switch (tname("TradeableItem", it.item)) {
      case "TRADE_GOLD":
        return `${v} per gold.`;
      case "TRADE_GOLD_PER_TURN":
        return `${v} per gold per turn: ${v / turns} per gold × ${turns} turns. Lease: ${turns} turns at least, then either side may cancel it.`;
      case "TRADE_RESOURCES":
        return `${v}: ${v / turns} per turn × ${turns} turns. Lease: ${turns} turns at least, then either side may cancel it.`;
      case "TRADE_TECHNOLOGIES":
        return "Research cost for the receiving side.";
      case "TRADE_PEACE_TREATY":
        return "Value of ending the war for the receiving side.";
      case "TRADE_WAR":
        return "Price of declaring war.";
    }
    return null;
  };
  const summary = h("div.trade-summary", { hidden: true });
  // A gift's lines: what they recorded of your gifts, this gift, the relations after it, the tiers.
  const giftLines = (tr, offered) => {
    const g = tr.gifts;
    const after = Math.min(Math.floor((g.value + offered) / g.tier), g.max_relations);
    const tiers = [`<${g.tier}: no effect`];
    for (let n = 1; n <= g.max_relations; n++) tiers.push(n < g.max_relations ? `${n * g.tier}-${(n + 1) * g.tier}: +${n} relations` : `${n * g.tier} or more: +${n} relations`);
    return [
      h("div", `Current value of gifts: ${g.value}.`),
      h("div", `Value of this gift: ${offered}.`),
      h("div", after > 0 ? `Effect: +${after} relations for ${tr.turns} turns${after !== g.relations ? ` (now +${g.relations})` : ""}.` : "Effect: none."),
      h("div.dim", `Gift tiers: ${tiers.join(". ")}. Gifts count for ${tr.turns} turns.`),
    ];
  };
  let asked = 0;
  const refreshSummary = async () => {
    const give = [...deal.give.values()];
    const get = [...deal.get.values()];
    if (!give.length && !get.length && !deal.troops) {
      summary.hidden = true;
      return;
    }
    let offered = 0;
    let requested = 0;
    for (const it of t.we_give) if (deal.give.has(keyOf(it))) offered += points(it, deal.give.get(keyOf(it)).data);
    for (const it of t.they_give) if (deal.get.has(keyOf(it))) requested -= points(it, deal.get.get(keyOf(it)).data);
    // Only your side: a gift; only theirs: a demand (the trade query's `tribute`); else a trade.
    const mode = deal.troops ? "trade" : !get.length ? "gift" : !give.length ? "demand" : "trade";
    const tr = t.tribute;
    const lines =
      mode === "gift"
        ? giftLines(tr, offered)
        : mode === "demand"
          ? [h("div", `This would ask for: ${requested}.`)]
          : [h("div", `Total offered: ${offered}`), h("div", `Total requested: ${requested}`)];
    summary.dataset.mode = mode;
    summary.replaceChildren(...lines);
    summary.hidden = false;
    // A loan of troops is judged on its own terms (`propose_loan`): no answer in advance.
    if (deal.troops) return;
    const ask = ++asked;
    let r;
    try {
      r = await api.query("trade_answer", { with: withPlayer, give, get });
    } catch {
      return;
    }
    if (ask !== asked || m.closed || r.accept === null || r.accept === undefined) return;
    const answer = h("div", { "data-answer": r.accept ? "accepted" : "refused" }, "This proposal will be ", h(r.accept ? "b.good" : "b.warn", r.accept ? "accepted" : "refused"), ".");
    const reason = r.why ? h("div.dim", `Reason: ${r.why}.`) : null;
    if (mode === "gift") summary.replaceChildren(...lines, answer);
    else if (mode === "demand") {
      // The most they would hand over is what is asked plus the answer's margin.
      const most = Math.max(0, requested + (r.margin ?? -requested));
      const effect = `Effect: −1 relations for ${tr.turns} turns. No further demands for ${tr.demands.pause_turns} turns.`;
      summary.replaceChildren(h("div", `Maximum possible value extorted: ${most}.`), ...lines, answer, r.accept ? h("div", effect) : reason);
    } else summary.replaceChildren(...lines, answer, reason);
  };
  const draw = () => {
    const side = (list, mapKey, other, title) =>
      h(
        "div.trade-side",
        h("h4", title),
        list.length
          ? list.map((it) => {
              const k = keyOf(it);
              const on = deal[mapKey].has(k);
              // Refused by its side, or (an item on both sides) by the other side.
              const refusal = tradeRefusal(it, other);
              const denied = !!refusal;
              const why = refusal ? `${refusal.twin ? "the other side refuses: " : ""}${dname("DenialType", refusal.denial)}` : "";
              const label = isGold(it) ? `${tradeItemLabel({ ...it, data: 0 })} (up to ${it.max})` : tradeItemLabel(it);
              const value = h("span.trade-value", { class: !hasValue(it) ? "none" : it.value > 0 ? "plus" : "minus", tip: valueTip(it) }, valueText(it, deal[mapKey].get(k)));
              const row = h(
                "div.trade-item",
                { class: on ? "on" : "", "data-item": tname("TradeableItem", it.item), "data-data": it.data, tip: itemTip(it, techHelp) },
                h(
                  "button.small",
                  {
                    disabled: denied || (isGold(it) && !it.max),
                    title: denied ? `Refused (${why})` : "Add to the deal / take it out",
                    onclick: () => {
                      const toggle = (map, item) => (map.has(keyOf(item)) ? map.delete(keyOf(item)) : map.set(keyOf(item), { item: item.item, data: isGold(item) ? Math.min(item.max, 1) : item.data }));
                      toggle(deal[mapKey], it);
                      if (isDual(it)) {
                        const twin = other.find((o) => o.item === it.item);
                        const otherKey = mapKey === "give" ? "get" : "give";
                        if (twin && deal[mapKey].has(k) !== deal[otherKey].has(keyOf(twin))) toggle(deal[otherKey], twin);
                      }
                      draw();
                    },
                  },
                  on ? "✓" : "+",
                ),
                h("span", label, it.value_turns ? h("span.dim", " (lease)") : null),
                on && isGold(it)
                  ? h("input.amount", {
                      type: "number",
                      min: 1,
                      max: it.max,
                      value: deal[mapKey].get(k).data,
                      oninput: (ev) => {
                        const sel = deal[mapKey].get(k);
                        sel.data = Math.max(1, Math.min(it.max, Number(ev.target.value) || 1));
                        value.textContent = valueText(it, sel);
                        refreshSummary();
                      },
                    })
                  : null,
                denied ? h("span.dim", ` (${why})`) : null,
                value,
              );
              return row;
            })
          : h("p.dim", "Nothing."),
        troopsRow(mapKey),
      );
    // Troops (the crusade ruleset): a loan of units, each way; adding it opens the troops screen
    // (the enemy, the mission, the turns, the units you send), the rest of the deal its price.
    const troopsRow = (mapKey) => {
      const lend = mapKey === "give";
      const way = lend ? t.troops?.lend : t.troops?.borrow;
      if (!way || t.war) return null;
      const on = !!deal.troops && deal.troops.lend === lend;
      const denied = !way.targets.length;
      const edit = async () => {
        const terms = await openTroops({ lend, other: withPlayer, way, maxTurns: t.troops.max_turns, current: on ? deal.troops : null });
        if (terms && !m.closed) {
          deal.troops = terms;
          draw();
        }
      };
      return h(
        "div.trade-item.troops",
        { class: on ? "on" : "", "data-item": "TROOPS", "data-side": mapKey },
        h(
          "button.small",
          {
            disabled: denied,
            title: denied ? `Not now (${way.why_not})` : on ? "Take the troops out of the deal" : "Add troops to the deal…",
            onclick: () => {
              if (on) {
                deal.troops = null;
                draw();
              } else edit();
            },
          },
          on ? "✓" : "+",
        ),
        h("span", on ? troopsLabel(deal.troops) : lend ? "Troops: lend your units…" : "Troops: borrow theirs…"),
        on ? h("button.link.small", { "data-troops-edit": mapKey, onclick: edit }, "change") : null,
        denied ? h("span.dim", ` (${way.why_not})`) : null,
      );
    };
    m.setBody(
      leader.el,
      t.war ? h("p.warn", { title: "A deal makes peace, so it must hold the peace treaty (or a capitulation)." }, "You are at war.") : null,
      h("div.trade", side(t.we_give, "give", t.they_give, "You give"), side(t.they_give, "get", t.we_give, "They give")),
      summary,
      status,
    );
    refreshSummary();
  };
  m.setButtons([
    {
      label: "Propose",
      primary: true,
      onClick: async () => {
        const give = [...deal.give.values()];
        const get = [...deal.get.values()];
        const k = deal.troops;
        if (!give.length && !get.length && !k) {
          status.textContent = "Put something into the deal first.";
          return;
        }
        // With troops in it the deal is a loan, the other items its price (`propose_loan`).
        const reply = k
          ? await run({ c: "propose_loan", to: withPlayer, lend: k.lend, target: k.target, mission: k.mission, strength: k.strength, duration: k.duration, give, get, units: k.units })
          : await run({ c: "propose_deal", to: withPlayer, give, get });
        if (!reply) return;
        // The leader answers (Civ4's lines for a deal: peace, a gift, a demand, a trade).
        const accepted = !!reply.r?.accepted;
        const weGet = get.length || (k && !k.lend);
        const weGive = give.length || (k && k.lend);
        const comment = accepted ? (t.war ? "AI_DIPLOCOMMENT_PEACE" : weGet ? "AI_DIPLOCOMMENT_ACCEPT" : "AI_DIPLOCOMMENT_THANKS") : t.war ? "AI_DIPLOCOMMENT_NO_PEACE" : weGive ? "AI_DIPLOCOMMENT_NO_DEAL" : "AI_DIPLOCOMMENT_REJECT";
        if (accepted) {
          // The tables change with the deal (gold spent, peace made): read them again.
          try {
            t = await api.query("trade", { with: withPlayer });
          } catch {
            /* keep the old tables */
          }
          deal.give.clear();
          deal.get.clear();
          deal.troops = null;
          draw();
          status.textContent = "Accepted.";
          status.className = "good";
        } else {
          status.textContent = "Refused.";
          status.className = "warn";
        }
        status.dataset.answer = accepted ? "accepted" : "declined";
        const r = await speech(withPlayer, comment);
        if (!m.closed) leader.say(r?.text);
      },
    },
    { label: "Close", value: null },
  ]);
  draw();
  return m.done;
}
