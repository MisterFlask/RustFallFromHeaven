//! This game's armies as the human sees them (docs/ARMIES.md): the view's `armies` (the human's
//! armies, their supply, requisition and the recruits marching to them), each marching unit's
//! `march`, the Armies screen (`armies` query) and the recruitment choices of the city screen.

use ffh_core::army::{RecruitDest, SupplyLine, SupplySource, SupplyState};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::{Idx, OrderType, UnitType};

use crate::command::recruit_dest_json;
use crate::json::Json;
use crate::view::{city_name, Viewer};

/// A marching recruit's march (the unit's `march`, and the Armies screen's `recruits`): where it
/// goes (`dest`, `to` its name), where it comes from, the turns it needs along the rules' path
/// (`turns_left`; `null`: no way there now), the turns it has made no progress (`stalled`), and
/// whether it was sent to a garrison instead of its first destination (`diverted`).
pub fn march_json(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let m = g.unit_march(u).expect("a marching unit");
    crate::jobj! {
        "dest" => recruit_dest_json(Some(m.dest)),
        "to" => g.recruit_dest_name(m.dest),
        "from" => g.cities.contains(m.from_city).then(|| city_name(g, m.from_city)),
        "turns_left" => g.unit_march_eta(u),
        "stalled" => m.stalled,
        "diverted" => m.diverted,
        "since" => m.left_turn,
    }
}

/// A recruit on the Armies screen: the unit (`id`, `unit` its type, where it is) and its march.
fn recruit_json(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let mut j = march_json(v, u);
    j.set("id", u);
    j.set("unit", Json::idx(g.units[u].unit_type));
    j.set("x", g.units[u].x);
    j.set("y", g.units[u].y);
    j
}

/// Army `a`'s requisition orders in production: the city, the unit, its place in the queue and
/// its production turns.
fn requisition_orders(v: &Viewer, a: ArmyId) -> Vec<(CityId, Json)> {
    let g = v.g;
    g.army_requisition_orders(a)
        .into_iter()
        .map(|(c, i)| {
            let o = &g.cities[c].order_queue[i];
            let unit = (o.order_type == OrderType::ORDER_TRAIN && o.data1 >= 0 && (o.data1 as usize) < g.d.units.len()).then(|| UnitType::from_usize(o.data1 as usize));
            let j = crate::jobj! {
                "city" => c,
                "name" => city_name(g, c),
                "unit" => unit.map(Json::idx),
                "index" => i,
                "turns" => unit.map(|t| g.city_production_turns_left_unit(c, t, i as i32)),
            };
            (c, j)
        })
        .collect()
}

/// Army `a`'s requisition: the cities it requisitions from (`cities`, in the order picked), its
/// orders in production (`orders`), the other orders bound for it that its cities build next
/// (`head_orders`: a count; their destination or the city's default names the army) and how many
/// it lacks of its target size counting its units, its marching recruits and both kinds of orders
/// (`shortfall`; 0 or less: none). With `candidates`
/// (the Armies screen), every city of the human's that it may requisition from: whether picked,
/// governed, the unit it would train there (the rules' choice for the human), its production
/// turns, its march turns to the army over land (`null`: no way by land), its order in progress.
fn requisition_json(v: &Viewer, a: ArmyId, candidates: bool) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    let orders = requisition_orders(v, a);
    let mut j = crate::jobj! {
        "cities" => army.requisition.iter().copied().filter(|&c| g.cities.contains(c)).collect::<Vec<_>>(),
        "orders" => Json::Arr(orders.iter().map(|(_, o)| o.clone()).collect()),
        "head_orders" => g.army_head_orders(a),
        "shortfall" => g.army_shortfall(a),
    };
    if candidates {
        let open = g.army_check_requisition(v.h, a).is_ok();
        let list = g.players[v.h].cities.iter().copied().filter(|&c| g.cities.contains(c)).map(|c| {
            let preview = if open { g.army_requisition_preview(c, a) } else { None };
            crate::jobj! {
                "city" => c,
                "name" => city_name(g, c),
                "picked" => army.requisition.contains(&c),
                "governed" => g.city_is_governed(c),
                "unit" => preview.map(|p| Json::idx(p.0)),
                "production_turns" => preview.map(|p| p.2),
                "march_turns" => preview.and_then(|p| p.3),
                "order" => orders.iter().find(|(oc, _)| *oc == c).map(|(_, o)| o.clone()),
            }
        });
        j.set("candidates", Json::Arr(list.collect()));
        j.set("open", open);
    }
    j
}

/// The name of a supply line's source: the city's, or "a Trireme" for a ship.
fn source_name(g: &Game, s: SupplySource) -> String {
    match s {
        SupplySource::City(c) => city_name(g, c),
        SupplySource::Ship(u) => format!("a {}", crate::names::display(g.d, g.units[u].unit_type)),
    }
}

/// A supply line (docs/SUPPLY.md) as the human's view has it: `path` (`[x, y]` from the army's
/// plot to the source's), `cost`, `budget` (`SUPPLY_LINE_BUDGET`), `source` (its name) and
/// `source_kind` (`city` or `ship`).
pub fn supply_line_json(g: &Game, line: &SupplyLine) -> Json {
    crate::jobj! {
        "path" => Json::Arr(line.path.iter().map(|&q| { let (x, y) = g.map.xy(q); crate::jarr![x, y] }).collect()),
        "cost" => line.cost,
        "budget" => g.d.defines.supply_line_budget,
        "source" => source_name(g, line.source),
        "source_kind" => match line.source { SupplySource::City(_) => "city", SupplySource::Ship(_) => "ship" },
    }
}

/// Army `a`'s supply (docs/SUPPLY.md, section 5), the human's own: `state` (`supplied`, `hunger`
/// or `starvation`, from its counter), `turn` and `of` (Hunger's turn of `SUPPLY_HUNGER_TURNS`),
/// `turns_to_starvation` (Hunger: the turn starts with the line cut before Starvation; else
/// null), `line` (the line as it is traced now, [`supply_line_json`]; null: cut) and `text`, the
/// state in the bare words of the Armies screen and the unit panel.
pub fn supply_json(v: &Viewer, a: ArmyId) -> Json {
    let g = v.g;
    let d = g.d;
    let state = g.army_supply_state(a);
    let line = g.army_supply_line(a);
    let budget = d.defines.supply_line_budget;
    let supplied = |l: &SupplyLine| format!("{} of {budget}, from {}.", l.cost, source_name(g, l.source));
    let mut text = match state {
        SupplyState::Supplied => match &line {
            Some(l) => format!("Supplied: {}", supplied(l)),
            None => "No supply line: Hunger from your next turn.".to_string(),
        },
        SupplyState::Hunger { turn, of } => format!("Hunger: turn {turn} of {of}. No healing, no recruits."),
        SupplyState::Starvation => format!(
            "Starvation: -{}% HP a turn (to {}%), {:+}% damage dealt.",
            d.defines.starvation_damage_percent, d.defines.starvation_floor_percent, d.defines.starvation_damage_dealt_percent
        ),
    };
    if let (Some(l), true) = (&line, state != SupplyState::Supplied) {
        text.push_str(&format!(" Supply line again: {} Supplied from your next turn.", supplied(l)));
    }
    let (name, turn, of) = match state {
        SupplyState::Supplied => ("supplied", Json::Null, Json::Null),
        SupplyState::Hunger { turn, of } => ("hunger", Json::from(turn), Json::from(of)),
        SupplyState::Starvation => ("starvation", Json::Null, Json::Null),
    };
    crate::jobj! {
        "state" => name,
        "turn" => turn,
        "of" => of,
        "turns_to_starvation" => state.turns_to_starvation(),
        "line" => line.as_ref().map(|l| supply_line_json(g, l)),
        "text" => text,
    }
}

/// The supply lines of the other players' armies the human sees (docs/SUPPLY.md, section 5), each
/// `{owner, x, y, segments}`: the army's plot and its line clipped to the plots the human has
/// revealed (`segments`: the runs of consecutive revealed plots of the path, each `[[x, y], ...]`,
/// from the army toward its source). An army counts as seen when its plot is visible to the human
/// and one of its units is visible and shows its true owner (hidden nationality hides the
/// army); an army with no line now has no entry. Nothing else of the army's supply is told (its
/// counter, the cost, the source).
pub fn foreign_supply_lines(v: &Viewer) -> Json {
    let g = v.g;
    let mut out = Vec::new();
    for p in g.player_ids() {
        if p == v.h || !g.players[p].alive || g.player_is_barbarian(p) {
            continue;
        }
        for &a in &g.players[p].armies {
            if !g.armies.contains(a) {
                continue;
            }
            let Some(plot) = g.army_plot(a) else { continue };
            if !v.plot_visible(plot) || !g.army_units(a).iter().any(|&u| v.unit_visible(u) && v.unit_owner(u) == p) {
                continue;
            }
            let Some(line) = g.army_supply_line(a) else { continue };
            let mut segments: Vec<Json> = Vec::new();
            let mut run: Vec<Json> = Vec::new();
            for &q in &line.path {
                if v.plot_revealed(q) {
                    let (x, y) = g.map.xy(q);
                    run.push(crate::jarr![x, y]);
                } else if !run.is_empty() {
                    segments.push(Json::Arr(std::mem::take(&mut run)));
                }
            }
            if !run.is_empty() {
                segments.push(Json::Arr(run));
            }
            let (x, y) = g.map.xy(plot);
            out.push(crate::jobj! { "owner" => p, "x" => x, "y" => y, "segments" => Json::Arr(segments) });
        }
    }
    Json::Arr(out)
}

/// One army of the human's: name, units, where, in supply, target size, the recruits marching to
/// it (`marching`: unit ids; each unit's `march` says where and how long), its requisition,
/// whether it marches to a garrison (disbanded), Upgrade all. The Armies screen's
/// (`screen`) adds the recruits in full (`recruits`) and the requisition's candidates.
fn army_json(v: &Viewer, a: ArmyId, screen: bool) -> Json {
    let g = v.g;
    let army = &g.armies[a];
    let plot = g.army_plot(a);
    let units = g.army_units(a);
    let marching = g.army_marching(a);
    // Upgrade all: the units with an upgrade (given the gold) and its total price.
    let upgrades: Vec<(UnitId, ffh_data::UnitType)> = units.iter().filter_map(|&u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
    let mut j = crate::jobj! {
        "id" => a,
        "name" => army.name.as_str(),
        "group" => army.group,
        "units" => units,
        "x" => plot.map(|p| g.map.xy(p).0),
        "y" => plot.map(|p| g.map.xy(p).1),
        "in_supply" => g.army_in_supply(a),
        "supply" => supply_json(v, a),
        // Its battle tactic (docs/TACTICS.md) and the one it takes at the start of the next turn.
        "tactic" => Json::idx(army.tactic),
        "tactic_next" => Json::idx(army.tactic_next),
        "target_size" => army.target_size,
        "marching" => marching.clone(),
        "requisition" => requisition_json(v, a, screen),
        "disband_to" => army.disband_to.filter(|&c| g.cities.contains(c)).map(|c| city_name(g, c)),
        "upgrades" => upgrades.len(),
        "upgrade_price" => upgrades.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum::<i32>(),
    };
    if screen {
        j.set("recruits", Json::Arr(marching.into_iter().map(|u| recruit_json(v, u)).collect()));
    }
    j
}

/// The human's armies (formation order), as the view has them.
pub fn armies(v: &Viewer) -> Json {
    let g = v.g;
    Json::Arr(g.players[v.h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| army_json(v, a, false)).collect())
}

/// The recruits marching to garrisons (unit ids).
pub fn garrison_marching(v: &Viewer) -> Json {
    let g = v.g;
    Json::from(g.players[v.h].units.iter().copied().filter(|&u| g.unit_march(u).is_some_and(|m| matches!(m.dest, RecruitDest::Garrison(_)))).collect::<Vec<_>>())
}

/// `armies`: the Armies screen. The armies (as the view's, with their recruits in full and the
/// cities each may requisition from), each city's garrison with the recruits marching to it, the
/// supply range and how many turns a recruit waits without progress before it goes to the
/// nearest garrison instead.
pub fn screen(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let garrisons = g.players[h].cities.iter().map(|&c| {
        crate::jobj! {
            "city" => c,
            "name" => city_name(g, c),
            "governed" => g.city_is_governed(c),
            "units" => g.city_garrison_units(c),
            "recruits" => Json::Arr(g.garrison_marching(c).into_iter().map(|u| recruit_json(v, u)).collect()),
        }
    });
    crate::jobj! {
        "armies" => Json::Arr(g.players[h].armies.iter().filter(|&&a| g.armies.contains(a)).map(|&a| army_json(v, a, true)).collect()),
        "garrisons" => Json::Arr(garrisons.collect()),
        "supply_budget" => g.d.defines.supply_line_budget,
        "stall_turns" => g.d.defines.army_transit_stall_turns,
    }
}

/// The city screen's recruitment: the city's default destination and the destinations it may
/// choose (its own and the other cities' garrisons, the armies), its garrison and the armies that
/// requisition from it; `null` without armies.
pub fn recruit(v: &Viewer, c: CityId) -> Json {
    let g = v.g;
    let h = v.h;
    let mut options = vec![crate::jobj! { "dest" => recruit_dest_json(Some(RecruitDest::Garrison(c))), "name" => "this city's garrison" }];
    for &o in &g.players[h].cities {
        if o != c {
            options.push(crate::jobj! { "dest" => recruit_dest_json(Some(RecruitDest::Garrison(o))), "name" => format!("garrison of {}", city_name(g, o)) });
        }
    }
    let mut requisitioned_by = Vec::new();
    for &a in &g.players[h].armies {
        if !g.armies.contains(a) {
            continue;
        }
        if g.armies[a].requisition.contains(&c) {
            requisitioned_by.push(crate::jobj! { "army" => a, "name" => g.armies[a].name.as_str(), "target_size" => g.armies[a].target_size, "shortfall" => g.army_shortfall(a) });
        }
        if g.recruit_dest_valid(h, RecruitDest::Army(a)) {
            let size = g.army_units(a).len();
            options.push(crate::jobj! {
                "dest" => recruit_dest_json(Some(RecruitDest::Army(a))),
                "name" => format!("{} ({} units{})", g.armies[a].name, size, if g.army_in_supply(a) { "" } else { ", out of supply" }),
            });
        }
    }
    let garrison: Vec<UnitId> = g.city_garrison_units(c);
    crate::jobj! {
        "default" => recruit_dest_json(g.cities[c].recruit_default),
        "options" => Json::Arr(options),
        "garrison" => garrison,
        // The garrison's battle tactic (docs/TACTICS.md) and the one it takes next turn.
        "garrison_tactic" => Json::idx(g.cities[c].garrison_tactic),
        "garrison_tactic_next" => Json::idx(g.cities[c].garrison_tactic_next),
        "requisitioned_by" => Json::Arr(requisitioned_by),
    }
}
