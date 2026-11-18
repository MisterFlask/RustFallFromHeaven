//! Trade (design §4.5 `propose_deal`, §4.6 `trade`): what the diplomacy screen offers, both ways,
//! and the checks a proposed deal must pass.
//!
//! The screen lists `CvPlayer::buildTradeTable` (ported here as [`trade_table`]: gold, gold per
//! turn, maps, vassal, open borders, defensive pact, permanent alliance, at war a peace treaty
//! and capitulation, then techs, resources, cities, and for an AI side peace, war and embargo
//! with third teams, civics and religions), each item greyed when `getTradeDenial` refuses it.
//! Gold and gold per turn are amounts up to the side's maximum (`AI_maxGoldTrade`,
//! `AI_maxGoldPerTurnTrade`: for a human its gold and its gold rate; for an AI the C++ AI's own
//! limits are not ported, so the same bounds apply). The C++ AI's denials are not ported either
//! (ffh-core reads them as "no denial" except the rules-based ones); whether an AI accepts is its
//! controller's `consider_offer`. Each item carries the points the AI puts on it
//! (`Controller::trade_value`), and [`trade_answer`] tells what a deal would get before it is
//! proposed (`Controller::judge_offer`, which agrees with `consider_offer`).
//!
//! Whether an AI talks at all is [`ai_willing_to_talk`] (`CvPlayerAI::AI_isWillingToTalk`, which
//! gates the exe's diplomacy screen: "refuses to talk"): at war, not before its war counter
//! reaches its leader's refuse-to-talk threshold. A deal between teams at war always makes peace
//! (`CvDeal::addTrades` calls `makePeace` for any deal at war), so at war a deal must say so: a
//! peace treaty on both sides, or a capitulation, as the C++ AI's own war offers and counter
//! proposals have it (`AI_doDiplo`, `AI_counterPropose`). A gift alone would end the war past the
//! AI's peace judgement.

use ffh_core::deal::Deal;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::TradeData;
use ffh_data::*;

use crate::json::Json;
use crate::query::{invalid, QueryErr};
use crate::view::Viewer;

fn item(item_type: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type, data, offering: false, hidden: false }
}

/// C++ `CvPlayer::buildTradeTable(eOtherPlayer, ourList)`: the items `from` can give `to`.
pub fn trade_table(g: &Game, from: PlayerId, to: PlayerId) -> Vec<TradeData> {
    let d = g.d;
    let mut out = Vec::new();
    let mut add = |t: TradeData, check: bool| {
        if !check || g.player_can_trade_item(from, to, &t, false) {
            out.push(t);
        }
    };
    add(item(TradeableItem::TRADE_GOLD, 0), true);
    add(item(TradeableItem::TRADE_GOLD_PER_TURN, 0), true);
    add(item(TradeableItem::TRADE_MAPS, 0), true);
    add(item(TradeableItem::TRADE_VASSAL, 0), true);
    add(item(TradeableItem::TRADE_OPEN_BORDERS, 0), true);
    add(item(TradeableItem::TRADE_DEFENSIVE_PACT, 0), true);
    add(item(TradeableItem::TRADE_PERMANENT_ALLIANCE, 0), true);
    // Not in C++: a House's side deals (docs/PERSONS.md, rule 9; only a House gives them).
    add(item(TradeableItem::TRADE_SECEDE, 0), true);
    add(item(TradeableItem::TRADE_HOUSE_VASSAL, 0), true);
    let (team, to_team) = (g.players[from].team, g.players[to].team);
    if g.at_war(team, to_team) {
        // "We are at war, allow a peace treaty option" (no canTradeItem check).
        add(item(TradeableItem::TRADE_PEACE_TREATY, 0), false);
        add(item(TradeableItem::TRADE_SURRENDER, 0), true);
    }
    for t in d.techs.keys() {
        add(item(TradeableItem::TRADE_TECHNOLOGIES, t.idx() as i32), true);
    }
    for b in d.bonuses.keys() {
        add(item(TradeableItem::TRADE_RESOURCES, b.idx() as i32), true);
    }
    for &c in &g.players[from].cities {
        add(item(TradeableItem::TRADE_CITIES, c.0 as i32), true);
    }
    if !g.players[from].human {
        for kind in [TradeableItem::TRADE_PEACE, TradeableItem::TRADE_WAR, TradeableItem::TRADE_EMBARGO] {
            for t in g.civ_team_ids() {
                if g.team_is_alive(t) && t != team && t != to_team {
                    add(item(kind, t.0 as i32), true);
                }
            }
        }
    }
    for c in d.civics.keys() {
        add(item(TradeableItem::TRADE_CIVIC, c.idx() as i32), true);
    }
    for r in d.religions.keys() {
        add(item(TradeableItem::TRADE_RELIGION, r.idx() as i32), true);
    }
    out
}

/// The most gold `from` can give `to` (`AI_maxGoldTrade`: a human's gold; see the module docs).
pub fn max_gold(g: &Game, from: PlayerId, _to: PlayerId) -> i32 {
    g.players[from].gold.max(0)
}

/// The most gold per turn `from` can give (`AI_maxGoldPerTurnTrade`: for a human
/// `max(0, min(goldRate + gold / PEACE_TREATY_LENGTH, goldRate))`).
pub fn max_gold_per_turn(g: &Game, from: PlayerId, _to: PlayerId) -> i32 {
    let rate = g.player_calculate_gold_rate(from);
    let length = g.d.defines.peace_treaty_length.max(1);
    (rate + g.players[from].gold / length).min(rate).max(0)
}

/// The points the AI `judge` puts on `t` in a deal between `from` and `to` (`Controller::
/// trade_value`): positive for what it gets, negative for what it gives; gold and gold per turn
/// per piece. Null when it puts no price on the item (or `judge` is human).
fn value_json(g: &Game, judge: PlayerId, from: PlayerId, to: PlayerId, t: &TradeData) -> Json {
    let gets = judge == to;
    let item = if Deal::is_gold(t.item_type) { TradeData { data: 1, ..t.clone() } } else { t.clone() };
    let other = if gets { from } else { to };
    match g.controller(judge).trade_value(g, judge, other, &item, gets) {
        Some(v) => Json::from(if gets { v } else { -v }),
        None => Json::Null,
    }
}

fn item_json(g: &Game, judge: PlayerId, from: PlayerId, to: PlayerId, t: &TradeData) -> Json {
    let mut j = crate::jobj! {
        "item" => Json::idx(t.item_type),
        "data" => t.data,
        "denial" => Json::opt_idx(g.player_get_trade_denial(from, to, t)),
        "value" => value_json(g, judge, from, to, t),
    };
    // A lease (an item that pays every turn while the deal lasts): the turns its value counts.
    if matches!(t.item_type, TradeableItem::TRADE_RESOURCES | TradeableItem::TRADE_GOLD_PER_TURN) {
        j.set("value_turns", g.d.defines.peace_treaty_length);
    }
    match t.item_type {
        TradeableItem::TRADE_GOLD => {
            j.set("max", max_gold(g, from, to));
        }
        TradeableItem::TRADE_GOLD_PER_TURN => {
            j.set("max", max_gold_per_turn(g, from, to));
        }
        // The diplomacy screen names the cities either side offers.
        TradeableItem::TRADE_CITIES if t.data >= 0 && g.cities.contains(CityId(t.data as u32)) => {
            j.set("name", crate::view::city_name(g, CityId(t.data as u32)));
        }
        _ => {}
    }
    j
}

/// C++ `CvPlayerAI::AI_isWillingToTalk(ePlayer)`: whether the AI `ai` talks to `other` (the exe
/// opens the diplomacy screen only then; the contact button's help says "refuses to talk"). At
/// war it refuses until its war counter reaches its leader's `iRefuseToTalkWarThreshold` (doubled
/// for a war it chose, shortened when the other side is clearly winning), and a vassal never
/// talks at war. In peace C++ also refuses while it remembers the other stopping trade with it
/// (`MEMORY_STOPPED_TRADING_RECENT`); that branch is not ported: the AI memory decay
/// (`CvPlayerAI::AI_doCounter`) is AI code that ffh-ai does not run, so the memory would never
/// fade and the AI would refuse to talk for the rest of the game.
pub fn ai_willing_to_talk(g: &Game, ai: PlayerId, other: PlayerId) -> bool {
    // Not in C++: a House is a party for anyone who has met it, at war too (its side deals,
    // docs/PERSONS.md, rule 9; what it trades is the rules').
    if g.player_house(ai).is_some() {
        return true;
    }
    let (team, their) = (g.players[ai].team, g.players[other].team);
    if their == team || g.teams[their].vassal[team] || g.teams[team].vassal[their] {
        return true;
    }
    if g.team_is_human(team) {
        return false;
    }
    if g.at_war(team, their) {
        let threshold = g.players[ai].personality_type.map_or(0, |l| g.d.leader_heads[l].refuse_to_talk_war_threshold);
        let mut refuse = threshold * if g.team_ai_is_chosen_war(team, their) { 2 } else { 1 };
        let ours = 1 + g.teams[team].ai_war_success[their];
        let theirs = 1 + g.teams[their].ai_war_success[team];
        if theirs > ours * 2 {
            refuse *= 20 + (80 * ours * 2) / theirs.max(1);
            refuse /= 100;
        }
        if g.teams[team].ai_at_war_counter[their] < refuse {
            return false;
        }
        if g.team_is_a_vassal(team) {
            return false;
        }
    }
    true
}

/// Why the human cannot trade with `with` (or `Ok`): a met, alive civilization it can contact
/// and that is willing to talk ([`ai_willing_to_talk`]).
/// Met as `v` knows it: a command's check passes a viewer without debug reveal.
pub fn contact(v: &Viewer, with: PlayerId) -> Result<(), String> {
    let (g, h) = (v.g, v.h);
    if !g.is_civ_player_slot(with) || with == h || g.player_is_barbarian(with) {
        return Err(format!("player {} is not another civilization", with.0));
    }
    if !g.players[with].alive || !v.player_met(with) {
        return Err(format!("player {} is unknown or dead", with.0));
    }
    if !g.player_can_contact(h, with) {
        return Err(format!("player {} cannot be contacted now", with.0));
    }
    if !g.players[with].human && !ai_willing_to_talk(g, with, h) {
        return Err(format!("player {} refuses to talk", with.0));
    }
    Ok(())
}

/// `trade {with}`: the trade tables both ways (`we_give` from the human's table, `they_give`
/// from theirs), with denials and the gold maxima; and `tribute`, the gifts and demands between the two
/// (ffh-core `tribute`): what `with` recorded of the human's gifts (`value`, the `relations` they
/// give) and of the demands it paid (`value`, `relations`, `pause`: turns before it pays
/// another), with the rules' constants.
pub fn trade(v: &Viewer, with: PlayerId) -> Result<Json, QueryErr> {
    let g = v.g;
    if !g.is_civ_player_slot(with) || with == v.h || g.player_is_barbarian(with) || !v.player_met(with) {
        return Err(invalid(format!("field `with`: player {} is not a civilization the human knows", with.0)));
    }
    let can = contact(v, with);
    let (ours, theirs) = if can.is_ok() { (trade_table(g, v.h, with), trade_table(g, with, v.h)) } else { (Vec::new(), Vec::new()) };
    // A House (docs/PERSONS.md, "The second build", rule 9): it takes gold and gold per turn and
    // gives Secede and Become your vassal, each listed with the rules' refusal when it would not;
    // the values are its price and the gold offered (`house_trade_items`). No troops.
    let house = g.player_house(with);
    let troops = if can.is_ok() && house.is_none() { troops(v, with) } else { Json::Null };
    let (we_give, they_give): (Vec<Json>, Vec<Json>) = match house {
        Some(hh) if can.is_ok() => house_trade_items(v, hh, &ours),
        _ => (ours.iter().map(|t| item_json(g, with, v.h, with, t)).collect(), theirs.iter().map(|t| item_json(g, with, with, v.h, t)).collect()),
    };
    Ok(crate::jobj! {
        "with" => with,
        "contact" => can.is_ok(),
        "why_not" => can.err(),
        "war" => g.at_war(v.team, g.players[with].team),
        "we_give" => Json::Arr(we_give),
        "they_give" => Json::Arr(they_give),
        "troops" => troops,
        "house" => house.map_or(Json::Null, |hh| house_trade_json(v, hh)),
        "tribute" => tribute_json(g, with, v.h),
        // Persons of Note (docs/PERSONS.md, "UI", part I): what making peace with them does to
        // the cast (at war only; empty when nothing).
        "peace_preview" => if g.at_war(v.team, g.players[with].team) { crate::view::house_marks::act_preview(v, &ffh_core::persons::PersonsAct::MakePeace(g.players[with].team)) } else { Json::Arr(Vec::new()) },
    })
}

/// Whether the human's team is House `hh`'s liege's (its own House: no side deal, it pays tribute).
fn own_house(v: &Viewer, hh: HouseId) -> bool {
    v.g.players[v.g.houses[hh].owner].team == v.team
}

/// The trade tables with House `hh` (docs/PERSONS.md, "The second build", rule 9): what the human
/// can give it (`ours`, the rules' table: gold and gold per turn), each worth what it counts toward
/// the House's price (gold 1, gold per turn [`BRIBE_GOLD_PER_TURN_TURNS`]: `value_turns`); and
/// what it gives, Secede and Become your vassal, each with `refusal` (the rules' reason it would
/// not give it to the human now, `House Arne is Pleased with the Bannor.`; null: it would) and
/// worth its price (negative: what it gives; null when refused). None for the human's own House.
fn house_trade_items(v: &Viewer, hh: HouseId, ours: &[TradeData]) -> (Vec<Json>, Vec<Json>) {
    use ffh_core::persons::{BribeKind, BRIBE_GOLD_PER_TURN_TURNS};
    let g = v.g;
    let hp = g.houses[hh].player;
    if own_house(v, hh) {
        return (Vec::new(), Vec::new());
    }
    let we = ours
        .iter()
        .map(|t| {
            let mut j = item_json(g, hp, v.h, hp, t);
            match t.item_type {
                TradeableItem::TRADE_GOLD => {
                    j.set("value", 1);
                }
                TradeableItem::TRADE_GOLD_PER_TURN => {
                    j.set("value", BRIBE_GOLD_PER_TURN_TURNS);
                    j.set("value_turns", BRIBE_GOLD_PER_TURN_TURNS);
                }
                _ => {
                    j.set("value", Json::Null);
                }
            }
            j
        })
        .collect();
    let price = g.house_bribe_price(hh);
    let they = [BribeKind::Secede, BribeKind::Vassal]
        .into_iter()
        .map(|kind| {
            let t = item(kind.item(), 0);
            let refusal = g.house_bribe_refusal(hh, v.h, kind);
            let mut j = item_json(g, hp, hp, v.h, &t);
            j.set("value", if refusal.is_none() { price.map_or(Json::Null, |p| Json::from(-p)) } else { Json::Null });
            j.set("refusal", refusal);
            j
        })
        .collect();
    (we, they)
}

/// House `hh` as a party, the `trade` reply's `house` (docs/PERSONS.md, "The second build", rules
/// 7 and 9): `{id, label, liege, liege_name, colour, own, price, trades_through, liege_contact,
/// line, secede_war, vassal_war, vassal_war_against, vassal_war_join, grudge}`. `own`: the human is
/// its liege (no side deal: `line` says why, the rules' `House Arne pays its liege by tribute.`);
/// `price`: the gold it asks (gold per turn counted ten times; null: it takes no offer);
/// `trades_through`: `Trades through the Bannor.` (the rest is its liege's to trade;
/// `liege_contact`: the human can open the liege's trade screen); `secede_war` and `vassal_war`:
/// what accepting does (`House Arne will break away from the Bannor: war between them.`, `You
/// become its liege: war with the Bannor.`), with `vassal_war_against` and `vassal_war_join` the
/// parties Become your vassal brings into that war by the vassal rules (the liege's other vassals,
/// the human's own: `view::player::war_parties`, labels); `grudge`: what an accepted offer costs
/// with its liege (`The Bannor's attitude toward you -2 (lasting).`, under `If accepted:`; null
/// for the human's own House and for a human liege, which has no attitude).
fn house_trade_json(v: &Viewer, hh: HouseId) -> Json {
    use crate::view::player::{party_labels, war_parties};
    use ffh_core::persons::BribeKind;
    let g = v.g;
    let liege = g.houses[hh].owner;
    let label = g.house_label(hh);
    let civ = g.player_get_civilization_short_description(liege);
    let own = own_house(v, hh);
    let (against, join) = if own { (Vec::new(), Vec::new()) } else { war_parties(g, v.team, g.players[liege].team, Some(g.players[g.houses[hh].player].team)) };
    crate::jobj! {
        "id" => hh,
        "label" => label.as_str(),
        "liege" => liege,
        "liege_name" => civ.as_str(),
        "colour" => g.houses[hh].color as i32,
        "own" => own,
        "price" => if own { None } else { g.house_bribe_price(hh) },
        "trades_through" => (!own).then(|| format!("Trades through the {civ}.")),
        "liege_contact" => !own && liege != v.h && contact(v, liege).is_ok(),
        "line" => if own { g.house_bribe_refusal(hh, v.h, BribeKind::Secede) } else { None },
        "secede_war" => format!("{label} will break away from the {civ}: war between them."),
        "vassal_war" => format!("You become its liege: war with the {civ}."),
        "vassal_war_against" => party_labels(g, &against),
        "vassal_war_join" => party_labels(g, &join),
        "grudge" => (!own && !g.players[liege].human).then(|| format!("The {civ}'s attitude toward you {} (lasting).", signed(offer_grudge(g, liege, v.h)))),
    }
}

/// How far an accepted offer to one of `liege`'s Houses moves its attitude toward `buyer`
/// (docs/PERSONS.md, "The second build", rule 9): the rules' modifier (`BRIBE_GRUDGE`, on FfH's scale) as the
/// attitude's parts count it (ffh-ai's `attitude_extra_part`: 2 a step), from its modifiers now.
fn offer_grudge(g: &Game, liege: PlayerId, buyer: PlayerId) -> i32 {
    use ffh_ai::diplomacy::attitude_extra_part;
    let extra = g.players[liege].ai_attitude_extra[buyer];
    attitude_extra_part(extra + ffh_core::persons::BRIBE_GRUDGE) - attitude_extra_part(extra)
}

/// `+2`, `-2`, `0`.
fn signed(n: i32) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// A rules' sentence as the screen's `Reason: ...` takes it (no closing full stop: the screen adds
/// one).
fn reason_text(s: String) -> String {
    match s.strip_suffix('.') {
        Some(t) => t.to_string(),
        None => s,
    }
}

/// What House `hh` answers if the human proposes `give` for `get` (docs/PERSONS.md, "The second
/// build", rule 9), from the rules alone: refused with the rules' reason when it would not give
/// an item asked for (`House Arne is Pleased with the Bannor`), or when the deal is not one it
/// takes (`propose_deal`'s checks); else accepted when the gold offered (gold per turn counted ten
/// times) covers its price (`margin`: offered less price; refused: `House Arne asks 450 gold`).
fn house_answer(g: &Game, hh: HouseId, h: PlayerId, give: &[TradeData], get: &[TradeData]) -> ffh_core::controller::OfferVerdict {
    use ffh_core::controller::OfferVerdict;
    use ffh_core::persons::{house_item, house_offer_value};
    let no = |why: String| OfferVerdict { accept: false, margin: None, why: Some(reason_text(why)) };
    let label = g.house_label(hh);
    let kinds: Vec<_> = get.iter().filter_map(|t| house_item(t.item_type)).collect();
    if kinds.is_empty() {
        return no(format!("{label} trades only Secede and Become your vassal"));
    }
    if let Some(why) = kinds.iter().find_map(|&k| g.house_bribe_refusal(hh, h, k)) {
        return no(why);
    }
    if let Err(e) = check_deal(g, h, g.houses[hh].player, give, get) {
        return no(e.detail());
    }
    let offered = house_offer_value(give);
    match g.house_bribe_price(hh) {
        Some(price) => OfferVerdict { accept: offered >= price, margin: Some(offered - price), why: (offered < price).then(|| format!("{label} asks {price} gold")) },
        None => no(format!("{label} takes no offer")),
    }
}

fn tribute_json(g: &Game, ai: PlayerId, h: PlayerId) -> Json {
    use ffh_core::tribute::*;
    let (gift_relations, demand_relations) = g.tribute_relations(ai, h);
    crate::jobj! {
        "turns" => TRIBUTE_TURNS,
        "gifts" => crate::jobj! {
            "value" => g.gifts_value(ai, h),
            "relations" => gift_relations,
            // `[threshold, relations]` per step, increasing: the relations a total value at or
            // above the threshold gives (the rules' `gift_steps`).
            "steps" => Json::Arr(g.gift_steps().into_iter().map(|(at, n)| Json::Arr(vec![Json::from(at), Json::from(n)])).collect()),
        },
        "demands" => crate::jobj! {
            "value" => g.demands_value(ai, h),
            "relations" => demand_relations,
            "pause" => g.demand_pause(ai, h),
            "pause_turns" => DEMAND_PAUSE_TURNS,
        },
    }
}

/// `trade_answer {with, give, get}`: what `with` answers if the human proposes this deal now (the
/// diplomacy screen shows it while the deal is put together): `accept`, and the `margin` and
/// `why` of its judgement (`Controller::judge_offer`). A deal `propose_deal` refuses outright is
/// refused with that refusal as `why`; a human's answer is not known (`accept` null).
pub fn trade_answer(v: &Viewer, with: PlayerId, give: &[TradeData], get: &[TradeData]) -> Result<Json, QueryErr> {
    let g = v.g;
    if !g.is_civ_player_slot(with) || with == v.h || g.player_is_barbarian(with) || !v.player_met(with) {
        return Err(invalid(format!("field `with`: player {} is not a civilization the human knows", with.0)));
    }
    if g.players[with].human {
        return Ok(crate::jobj! { "accept" => Json::Null, "margin" => Json::Null, "why" => Json::Null });
    }
    // A House answers by the rules' price and reasons (docs/PERSONS.md, rule 9), whatever plays it.
    if let Some(hh) = g.player_house(with) {
        let verdict = house_answer(g, hh, v.h, give, get);
        return Ok(crate::jobj! { "accept" => verdict.accept, "margin" => verdict.margin, "why" => verdict.why });
    }
    let verdict = match check_deal(g, v.h, with, give, get) {
        Ok(()) => g.controller(with).judge_offer(g, with, v.h, give, get),
        Err(e) => ffh_core::controller::OfferVerdict { accept: false, margin: None, why: Some(e.detail()) },
    };
    Ok(crate::jobj! { "accept" => verdict.accept, "margin" => verdict.margin, "why" => verdict.why })
}

/// Troops (docs/CRUSADE.md, "Allied loans"), the crusades' item of the deal: a loan of
/// units, sent as `propose_loan` with the rest of the deal as its `give` / `get`. Each way --
/// `lend` (the human lends to `with`) and `borrow` (`with` lends to the human) -- the teams the
/// loan could be against (the borrower's enemies the lender may fight), or `why_not` when there
/// are none; `lend` lists the human's units that can be lent. How much `with` can spare is its
/// own business: its answer says.
fn troops(v: &Viewer, with: PlayerId) -> Json {
    use ffh_core::crusade::loan::{LoanError, LoanMission, LoanTerms};
    let g = v.g;
    let way = |lender: PlayerId, borrower: PlayerId| -> Json {
        let mut targets = Vec::new();
        let mut why: Option<LoanError> = None;
        for t in g.civ_team_ids() {
            let terms = LoanTerms { lender, borrower, target: t, mission: LoanMission::Attack, strength: 1, duration: 1 };
            match g.loan_check(&terms) {
                Ok(()) | Err(LoanError::NotEnoughUnits) => targets.push(t),
                Err(LoanError::NotAtWar | LoanError::BadTerms) => {}
                Err(e) => why = why.or(Some(e)),
            }
        }
        let why_not = if !targets.is_empty() {
            None
        } else {
            Some(why.map_or_else(|| "the borrower is at war with no one the lender may fight".to_string(), crate::command::loan_error))
        };
        crate::jobj! { "targets" => targets, "why_not" => why_not }
    };
    let mut lend = way(v.h, with);
    let units: Vec<Json> = g.players[v.h]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u) && g.crusade_unit_lendable(u))
        .map(|u| crate::jobj! { "unit" => u, "type" => Json::idx(g.units[u].unit_type), "strength" => g.crusade_unit_strength(u), "x" => g.units[u].x, "y" => g.units[u].y })
        .collect();
    lend.set("units", Json::Arr(units));
    crate::jobj! {
        "lend" => lend,
        "borrow" => way(with, v.h),
        "max_turns" => ffh_core::crusade::speed_turns(g, g.crusade_rules().loan_max_turns),
    }
}

/// The deal checks of `propose_deal` (the diplomacy screen only offers these): `to` can be
/// contacted and is willing to talk ([`contact`]); something is traded; every item is in its
/// side's trade table (gold and gold per turn as an amount from 1 to the side's maximum), not
/// twice, and passes `canTradeItem(.., bTestDenial = true)`; a dual item (peace treaty, open
/// borders, defensive pact, permanent alliance) is on both sides, as the screen puts it; at war,
/// the deal carries the peace treaty or a capitulation (see the module docs).
pub fn check_deal(g: &Game, h: PlayerId, to: PlayerId, give: &[TradeData], get: &[TradeData]) -> Result<(), crate::command::CmdErr> {
    use crate::command::CmdErr;
    if to.idx() >= g.num_players() {
        return Err(CmdErr::Invalid(format!("field `to`: player {} out of range", to.0)));
    }
    // A command acts on the rules' state: debug reveal does not make an unmet civilization met.
    contact(&Viewer::new(g, h, false), to).map_err(CmdErr::Cannot)?;
    if give.is_empty() && get.is_empty() {
        return Err(CmdErr::Invalid("an empty deal".into()));
    }
    let side = |list: &[TradeData], from: PlayerId, other: PlayerId, field: &str| -> Result<(), CmdErr> {
        let table = trade_table(g, from, other);
        for (i, t) in list.iter().enumerate() {
            if list[..i].iter().any(|u| u.item_type == t.item_type && u.data == t.data) || (Deal::is_gold(t.item_type) && list[..i].iter().any(|u| u.item_type == t.item_type)) {
                return Err(CmdErr::Invalid(format!("field `{field}`: {} twice", crate::names::name(g.d, t.item_type))));
            }
            let listed = match t.item_type {
                TradeableItem::TRADE_GOLD | TradeableItem::TRADE_GOLD_PER_TURN => {
                    let max = if t.item_type == TradeableItem::TRADE_GOLD { max_gold(g, from, other) } else { max_gold_per_turn(g, from, other) };
                    if !(1..=max).contains(&t.data) {
                        return Err(CmdErr::Cannot(format!("field `{field}`: {} {} (1 to {max} possible)", crate::names::name(g.d, t.item_type), t.data)));
                    }
                    table.iter().any(|u| u.item_type == t.item_type)
                }
                _ => table.iter().any(|u| u.item_type == t.item_type && u.data == t.data),
            };
            if !listed {
                return Err(CmdErr::Cannot(format!("field `{field}`: {} {} is not tradeable", crate::names::name(g.d, t.item_type), t.data)));
            }
            if !g.player_can_trade_item(from, other, t, true) {
                return Err(CmdErr::Cannot(format!("field `{field}`: {} {} is refused", crate::names::name(g.d, t.item_type), t.data)));
            }
        }
        Ok(())
    };
    side(give, h, to, "give")?;
    side(get, to, h, "get")?;
    for (list, other, field) in [(give, get, "give"), (get, give, "get")] {
        for t in list {
            if Deal::is_dual(t.item_type, false) && !other.iter().any(|u| u.item_type == t.item_type) {
                return Err(CmdErr::Invalid(format!(
                    "field `{field}`: {} goes on both sides of a deal",
                    crate::names::name(g.d, t.item_type)
                )));
            }
        }
    }
    // At war every deal makes peace (`CvDeal::addTrades`): it has to be a peace deal. (Not a House's
    // side deal, docs/PERSONS.md: it secedes, and the deal ends its share in its liege's war.)
    let house_deal = get.iter().any(|t| ffh_core::persons::house_item(t.item_type).is_some());
    if !house_deal && g.at_war(g.players[h].team, g.players[to].team) {
        let has = |kind: TradeableItem| give.iter().chain(get).any(|t| t.item_type == kind);
        if !has(TradeableItem::TRADE_PEACE_TREATY) && !has(TradeableItem::TRADE_SURRENDER) {
            return Err(CmdErr::Invalid("at war a deal makes peace: it needs the peace treaty (both sides) or a capitulation".into()));
        }
    }
    Ok(())
}

/// A trade item from the protocol: `{"item": TRADE_X or index, "data": n}` (`data` 0 when
/// absent).
pub fn trade_item_from_json(d: &GameData, j: &Json) -> Result<TradeData, crate::json::FieldError> {
    let item_type = crate::names::field::<TradeableItem>(d, j, "item")?;
    Ok(item(item_type, j.get_or("data", 0)?))
}

pub fn trade_item_json(t: &TradeData) -> Json {
    crate::jobj! { "item" => Json::idx(t.item_type), "data" => t.data }
}
