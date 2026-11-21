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
/// (`MEMORY_STOPPED_TRADING_RECENT`); that branch is not ported: when it was written the memory
/// never faded (the decay, C++ `CvPlayerAI::AI_doCounter`, was AI code ffh-ai did not run). The
/// decay is a rule now (`player_do_memory_decay`, 2026-11-18); the branch was not taken up with
/// it (ffh-ai reads three memories, docs/AI.md "Diplomacy").
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
    // A House (docs/PERSONS.md, "The fourth build", rule 3): it takes gifts, gold (no gold per
    // turn) and resources, and gives nothing (`house_trade_items`). No troops.
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
        // Houses (docs/PERSONS.md, "UI", part I): what making peace with them does to
        // the Houses (at war only; empty when nothing).
        "peace_preview" => if g.at_war(v.team, g.players[with].team) { crate::view::house_marks::act_preview(v, &ffh_core::persons::PersonsAct::MakePeace(g.players[with].team)) } else { Json::Arr(Vec::new()) },
    })
}

/// Whether the human's team is House `hh`'s liege's (its own House: no gift, it pays tribute).
fn own_house(v: &Viewer, hh: HouseId) -> bool {
    v.g.players[v.g.houses[hh].owner].team == v.team
}

/// The trade tables with House `hh` (docs/PERSONS.md, "The fourth build", rule 3): what the human
/// can give it (`ours`, the rules' table: gold and resources, gifts; it gives nothing). None for
/// the human's own House.
fn house_trade_items(v: &Viewer, hh: HouseId, ours: &[TradeData]) -> (Vec<Json>, Vec<Json>) {
    let g = v.g;
    let hp = g.houses[hh].player;
    if own_house(v, hh) {
        return (Vec::new(), Vec::new());
    }
    (ours.iter().map(|t| item_json(g, hp, v.h, hp, t)).collect(), Vec::new())
}

/// House `hh` as a party, the `trade` reply's `house` (docs/PERSONS.md, "The fourth build", rules
/// 3 and 10): `{id, label, liege, liege_name, colour, own, trades_through, liege_contact, line,
/// takes, courting, courts, requirement, reminders}`. `own`: the human is its liege (no gift: `line`
/// says why, the rules' `House Arne pays its liege by tribute.`); `trades_through`: `Trades through
/// the Bannor.` (the rest is its liege's to trade; `liege_contact`: the human can open the liege's
/// trade screen). A House of another realm takes gifts only: `takes` (`House Aldric takes gifts of
/// gold and resources and gives nothing.`, its side of the table); `courting`: the human's
/// courtship of it (`Your courtship of House Aldric: favour 80 of 188, ...`, the rules'
/// `house_courting_line`; `You do not court House Aldric.` while `courts` is false); `requirement`:
/// that courtship's requirement against its leader's attitude toward the human (`Its leader's
/// attitude toward you: Cautious. Requirement: 168; at Pleased 120; at Friendly 72.`: what a gift
/// that raises the attitude brings); `courting_calc` and `requirement_calc`: how their numbers are
/// made (docs/PLAY_DESIGN.md, "Calculated numbers"); `reminders`: the Court keyword's. Null for the
/// human's own.
fn house_trade_json(v: &Viewer, hh: HouseId) -> Json {
    let g = v.g;
    let d = g.d;
    let liege = g.houses[hh].owner;
    let label = g.house_label(hh);
    let civ = g.player_get_civilization_short_description(liege);
    let own = own_house(v, hh);
    let courting = if own { None } else { g.house_courting_line(hh, v.h) };
    let courts = courting.is_some();
    let court = d.ty::<KeywordType>("KEYWORD_COURT");
    crate::jobj! {
        "id" => hh,
        "label" => label.as_str(),
        "liege" => liege,
        "liege_name" => civ.as_str(),
        "colour" => g.houses[hh].color as i32,
        "own" => own,
        "trades_through" => (!own).then(|| format!("Trades through the {civ}.")),
        "liege_contact" => !own && liege != v.h && contact(v, liege).is_ok(),
        "line" => if own { g.house_gift_refusal(hh, v.h) } else { None },
        "takes" => (!own).then(|| format!("{label} takes gifts of gold and resources and gives nothing.")),
        "courts" => courts,
        "courting" => if own { None } else { Some(courting.unwrap_or_else(|| format!("You do not court {label}."))) },
        "courting_calc" => if courts { crate::view::culture::courtship_calc_json(v, &g.house_courtship_view(hh, v.h)) } else { Json::Null },
        "requirement" => (!own).then(|| crate::view::persons::requirement_line(g, hh, v.h)),
        "requirement_calc" => if own { Json::Null } else { crate::view::culture::calc_lines(v, &g.house_requirement_steps_calc(hh, v.h)) },
        "reminders" => if own { Json::Arr(Vec::new()) } else { crate::view::keywords::reminders(d, court) },
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

/// What House `hh` answers if the human proposes `give` for `get` (docs/PERSONS.md, "The fourth
/// build", rule 3), from the rules alone: a gift (nothing asked back: gold or resources) is
/// accepted when the deal's checks pass; anything else is refused (`House Arne takes gifts only`).
fn house_answer(g: &Game, hh: HouseId, h: PlayerId, give: &[TradeData], get: &[TradeData]) -> ffh_core::controller::OfferVerdict {
    use ffh_core::controller::OfferVerdict;
    let no = |why: String| OfferVerdict { accept: false, margin: None, why: Some(reason_text(why)) };
    if get.is_empty() && !give.is_empty() {
        return match check_deal(g, h, g.houses[hh].player, give, get) {
            Ok(()) => OfferVerdict { accept: true, margin: None, why: None },
            Err(e) => no(e.detail()),
        };
    }
    no(format!("{} takes gifts only", g.house_label(hh)))
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
    // A House answers by the rules (docs/PERSONS.md, "The fourth build", rule 3: gifts only),
    // whatever plays it.
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
        .map(|u| crate::jobj! { "unit" => u, "type" => Json::idx(g.units[u].unit_type), "strength" => g.crusade_unit_force(u), "x" => g.units[u].x, "y" => g.units[u].y })
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
    // At war every deal makes peace (`CvDeal::addTrades`): it has to be a peace deal. (Not a gift to
    // a House, docs/PERSONS.md, "The fourth build", rule 3, which makes no peace.)
    let house_gift = get.is_empty() && g.player_house(to).is_some();
    if !house_gift && g.at_war(g.players[h].team, g.players[to].team) {
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
