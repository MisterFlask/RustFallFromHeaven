//! Raids (docs/RAIDS.md, section 8) as the human sees them: a city's raid state (the city query's,
//! the city tile's and the banner's `raid`), its sally posture (the city query's `sally`, the
//! owner's), a land army's Raid order (`station`) and the unit panel's `Raid <city>` and `Stop
//! raiding` actions.

use ffh_core::army::Station;
use ffh_core::ids::*;
use ffh_core::raid::{RaidState, Sally};

use crate::command::Command;
use crate::json::Json;
use crate::view::{city_name, Viewer};

/// The sally postures' shared rule (the city screen's Sally section, under the selector).
pub const SALLY_RULE: &str = "A sally is a battle between a guard squad and a raider squad, each turn. The guard does not leave the city.";

/// A calculation's lines (the rules' `*_calc` reads), names resolved: the tooltip of the number
/// or line beside it (docs/PLAY_DESIGN.md 4.9).
fn calc_lines(v: &Viewer, lines: &[String]) -> Json {
    Json::Arr(lines.iter().map(|l| Json::from(crate::names::resolve_keys(v.g.d, l).into_owned())).collect())
}

/// `n turns` (`1 turn`).
fn turns(n: i32) -> String {
    if n == 1 { "1 turn".to_string() } else { format!("{n} turns") }
}

/// City `c`'s raid for the views, for both sides while the city is visible (as its siege):
/// `state` (`contested`, `overrun`, `stripped`), `label` (`Contested`, `Overrun`, `Stripped (12
/// turns)`), `text` (the rule in bare words), `percent` (the production and commerce modifier
/// in force), `raiders` (`[{army, name, owner, owner_name}]`, the armies holding the order),
/// `lost` (production and commerce a turn) and `gold` (the raiders' gold a turn, all armies) of
/// the take now, and `calc` (the take's calculation, [`ffh_core::game::Game::raid_calc_lines`]; empty when
/// stripped). Null with no raid, and for a city not seen.
pub fn city(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let Some(city) = g.cities.get(c) else { return Json::Null };
    if !v.city_visible(c) && city.owner != v.h && !v.reveal {
        return Json::Null;
    }
    let pct = city.raid_percent;
    let (state, label, text) = match g.city_raid_state(c) {
        RaidState::None => return Json::Null,
        RaidState::Contested => ("contested", "Contested".to_string(), format!("Contested: -{pct}% production and commerce. The raiders take what the city loses as gold.")),
        RaidState::Overrun => ("overrun", "Overrun".to_string(), format!("Overrun: -{pct}% production and commerce. The raiders take what the city loses as gold.")),
        RaidState::Stripped(n) => ("stripped", format!("Stripped ({})", turns(n)), format!("Stripped: no raid takes anything for {}.", turns(n))),
    };
    let raiders: Vec<Json> = g
        .city_raiders(c)
        .into_iter()
        .map(|a| {
            let owner = g.armies[a].owner;
            crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str(), "owner" => owner, "owner_name" => g.player_get_civilization_short_description(owner) }
        })
        .collect();
    let take = (state != "stripped").then(|| g.city_raid_calc(c));
    let calc = take.as_ref().map_or_else(Vec::new, |t| g.raid_calc_lines(c, t));
    crate::jobj! {
        "state" => state,
        "label" => label,
        "text" => text,
        "percent" => pct,
        "raiders" => Json::Arr(raiders),
        "lost" => take.as_ref().map_or(0, |t| t.total),
        "gold" => take.as_ref().map_or(0, |t| t.shares.iter().map(|s| s.gold).sum::<i32>()),
        "calc" => calc_lines(v, &calc),
    }
}

/// A posture's label and rule (the selector's option and the description under it).
fn posture_text(v: &Viewer, s: Sally) -> (&'static str, String) {
    match s {
        Sally::Never => ("Never", "The guard never sallies.".to_string()),
        Sally::Favoured => {
            let odds = v.g.d.defines.raid_sally_odds;
            let pct = if odds % 10 == 0 { format!("{}", odds / 10) } else { format!("{}.{}", odds / 10, odds % 10) };
            ("When favoured", format!("The guard sallies when its squad has at least {pct}% to win the battle."))
        }
        Sally::Always => ("Always", "The guard sallies whenever it has a squad that can fight.".to_string()),
    }
}

/// City `c`'s sally posture, its owner's only (the city screen): `posture` (`Sally::name`),
/// `options` (`[{name, label, text}]`, every posture) and `rule` ([`SALLY_RULE`]). Null for
/// another player's city.
pub fn sally(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let Some(city) = g.cities.get(c) else { return Json::Null };
    if city.owner != v.h && !v.reveal {
        return Json::Null;
    }
    let options: Vec<Json> = Sally::ALL
        .into_iter()
        .map(|s| {
            let (label, text) = posture_text(v, s);
            crate::jobj! { "name" => s.name(), "label" => label, "text" => text }
        })
        .collect();
    crate::jobj! { "posture" => city.sally.name(), "options" => Json::Arr(options), "rule" => SALLY_RULE }
}

/// A land army's standing order (the unit panel's army line, the Armies screen): `kind` (`none`,
/// `raid`), `city` and `text` (`Raiding Vallus.`; an order that no longer holds: `Raiding
/// Vallus: ends at the end of your turn (the war ended).`). `none` has no text.
pub fn station(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    match g.armies[a].station {
        Station::Raid(c) => {
            let name = if g.cities.contains(c) { city_name(g, c) } else { "a city".to_string() };
            let text = match g.army_raid_end_reason(a, c) {
                None => format!("Raiding {name}."),
                Some(why) => format!("Raiding {name}: ends at the end of your turn ({why})."),
            };
            crate::jobj! { "kind" => "raid", "city" => c, "text" => text, "holds" => g.army_raid_city(a) == Some(c) }
        }
        _ => crate::jobj! { "kind" => "none", "text" => Json::Null },
    }
}

/// The unit panel's raid actions of the human's land army `a` (the whole army selected) on
/// `plot`, each `(enabled, cmd, extra)` as the other army actions: `Stop raiding` while it holds a
/// Raid order (`army: "stop_raid"`); on the countryside of a city its team is at war with, `Raid
/// <city>` (`army: "raid"`, `city`), with its `help` (`Take: 7 gold a turn at 50%.`, then that
/// the guard sallies under the city's posture) and the take's calculation (`calc`), or disabled
/// with the rules' refusal (`why`). Nothing on no enemy countryside.
pub fn actions(v: &Viewer, a: ArmyId, plot: PlotIdx) -> Vec<(bool, Json, Json)> {
    let g = v.g;
    let mut out = Vec::new();
    let held = match g.armies[a].station {
        Station::Raid(c) => Some(c),
        _ => None,
    };
    if let Some(c) = held {
        let name = if g.cities.contains(c) { city_name(g, c) } else { "a city".to_string() };
        out.push((
            true,
            Command::ArmyStation { army: a, station: Station::None }.to_json(),
            crate::jobj! { "army" => "stop_raid", "label" => "Stop raiding", "help" => format!("The {} ends its Raid order on {name}.", g.armies[a].name), "city" => c },
        ));
    }
    let Some(c) = g.map.plots[plot].working_city.filter(|&c| g.plot_in_countryside(plot, c)) else { return out };
    if held == Some(c) || !g.at_war(v.team, g.city_team(c)) {
        return out;
    }
    let name = city_name(g, c);
    let refusal = g.army_raid_refusal(a, c);
    let forecast = g.army_raid_forecast(a, c);
    let gold = forecast.shares.iter().find(|s| s.army == a).map_or(0, |s| s.gold);
    let help = format!("Take: {gold} gold a turn at {}%.[NEWLINE]The guard sallies under the city's posture.", g.d.defines.raid_take_percent);
    let mut extra = crate::jobj! { "army" => "raid", "label" => format!("Raid {name}"), "help" => help, "city" => c };
    match refusal {
        Some(why) => {
            extra.set("why", why);
        }
        None => {
            extra.set("calc", calc_lines(v, &g.raid_calc_lines(c, &forecast)));
        }
    }
    out.push((refusal.is_none(), Command::ArmyStation { army: a, station: Station::Raid(c) }.to_json(), extra));
    out
}
