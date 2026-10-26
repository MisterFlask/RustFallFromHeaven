// Diplomacy (design §5.1 "Diplomacy minimum", §7 P6; Civ4's foreign advisor, F4): the players you
// met (leader, civilization, war or peace, open borders, pacts, vassals, attitude, score), your
// deals with them, declaring war (`declare_war`), and the trade screen: what each side can give
// (the `trade` query, C++ `buildTradeTable`, refusals greyed), a deal put together from both
// tables and offered (`propose_deal`), which the AI accepts or declines at once. Each leader shows
// their portrait (portraits.js).

import * as api from "./api.js";
import { S, dname, tname, teamName, tradeItemLabel, tradeRefusal, DUAL_ITEMS, me, scoreText, fairShareText } from "./state.js";
import { h, toast } from "./dom.js";
import { openModal, ask, closeTag } from "./modals.js";
import { infoTip } from "./infotips.js";
import { render as markup } from "./markup.js";
import { run } from "./commands.js";
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
        const relation = [p.war ? h("span.warn", "war") : "peace", p.open_borders ? "open borders" : null, p.defensive_pact ? "defensive pact" : null, p.vassal_of_us ? "your vassal" : null, p.our_master ? "your master" : null].filter(Boolean);
        const deals = p.deals.map((d) => `${d.we_give.map(tradeItemLabel).join(", ") || "nothing"} ⇄ ${d.they_give.map(tradeItemLabel).join(", ") || "nothing"} (since turn ${d.turn})`);
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
            p.alive && p.contact ? h("button.small", { "data-trade": p.id, onclick: () => openTrade(p.id, p.name).then(load) }, "Trade…") : h("span.dim", p.alive ? "no contact" : "dead"),
            p.can_declare_war && !p.war ? h("button.small.danger", { "data-war": p.team, onclick: () => declareWar(p).then(load) }, "Declare war") : null,
          ),
          deals.length ? h("td.dim", deals.map((d) => h("div", d))) : h("td"),
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
    parts.length ? parts.map(([why, n]) => h("div", n ? `${sign(n)} ${why}` : why)) : h("div.dim", "Nothing for or against you."),
    p.attitude_score !== null && p.attitude_score !== undefined ? h("div.tip-hint", `Total ${sign(p.attitude_score)}${p.attitude_next !== null && p.attitude_next !== undefined ? `; at ${sign(p.attitude_next)} they would be ${dname("AttitudeType", p.attitude + 1)}` : ""}.`) : null,
  ];
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

/** The trade screen with player `with`: both trade tables; clicking an item puts it into the
 *  deal (an item that goes on both sides, such as a peace treaty, goes on both, and is greyed on
 *  both when either side refuses it), gold as an amount; Propose offers it. */
export async function openTrade(withPlayer, name = null) {
  const m = openModal({ title: `Trade with ${name ?? S.players.get(withPlayer)?.name ?? `player ${withPlayer}`}`, body: [h("p.dim", "…")], tag: "trade", wide: true, buttons: [{ label: "Close", value: null }] });
  let t;
  try {
    t = await api.query("trade", { with: withPlayer });
  } catch (e) {
    m.setBody(h("p.warn", e.detail ?? e.message));
    return m.done;
  }
  if (!t.contact) {
    m.setBody(h("p.warn", t.why_not ?? "No contact."));
    return m.done;
  }
  // What a tech brings you (the techs query's help, in your context) for the techs on offer.
  let techHelp = new Map();
  try {
    techHelp = new Map((await api.query("techs")).techs.map((x) => [x.tech, x.help]));
  } catch {
    /* the tips go without */
  }
  // The deal: per side, the chosen items by key (gold: the amount typed).
  const deal = { give: new Map(), get: new Map() };
  const keyOf = (it) => `${it.item}:${it.data}`;
  const isDual = (it) => DUAL_ITEMS.includes(tname("TradeableItem", it.item));
  const isGold = (it) => ["TRADE_GOLD", "TRADE_GOLD_PER_TURN"].includes(tname("TradeableItem", it.item));
  const status = h("p.dim");
  // Who you are dealing with: their leader's portrait, name, civilization and mood.
  const pl = S.players.get(withPlayer);
  const head = pl
    ? h(
        "div.trade-head",
        playerPortrait(withPlayer, { size: PORTRAIT.large }),
        h("div", h("h3", pl.name), h("div.dim", dname("CivilizationType", pl.civ)), pl.attitude !== null && pl.attitude !== undefined ? h("div", `Attitude: ${dname("AttitudeType", pl.attitude)}`) : null),
      )
    : null;
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
                h("span", label),
                on && isGold(it)
                  ? h("input.amount", {
                      type: "number",
                      min: 1,
                      max: it.max,
                      value: deal[mapKey].get(k).data,
                      oninput: (ev) => (deal[mapKey].get(k).data = Math.max(1, Math.min(it.max, Number(ev.target.value) || 1))),
                    })
                  : null,
                denied ? h("span.dim", ` (${why})`) : null,
              );
              return row;
            })
          : h("p.dim", "Nothing."),
      );
    m.setBody(
      head,
      t.war ? h("p.warn", "You are at war: a deal makes peace, so it must hold the peace treaty (or a capitulation).") : null,
      h("div.trade", side(t.we_give, "give", t.they_give, "You give"), side(t.they_give, "get", t.we_give, "They give")),
      status,
    );
  };
  m.setButtons([
    {
      label: "Propose",
      primary: true,
      onClick: async () => {
        const give = [...deal.give.values()];
        const get = [...deal.get.values()];
        if (!give.length && !get.length) {
          status.textContent = "Put something into the deal first.";
          return;
        }
        const reply = await run({ c: "propose_deal", to: withPlayer, give, get });
        if (!reply) return;
        if (reply.r?.accepted) {
          toast("They accepted the deal.");
          closeTag("trade");
        } else {
          status.textContent = "They declined.";
          status.className = "warn";
        }
      },
    },
    { label: "Close", value: null },
  ]);
  draw();
  return m.done;
}
