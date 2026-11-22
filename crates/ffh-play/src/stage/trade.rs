//! `ffh-play stage trade-ready|trade-routes|trade-closed|trade-raid DIR`: the scenes of trade (docs/TRADE.md,
//! section 9; tools/trade-shots.mjs; tests/trade.rs). A duel (seed 11 or the next that fits, turn
//! 4) whose land around the human's capital is redrawn by hand: the capital's island (grassland, 10
//! plots wide), a strait of 6 water plots (a named sea, the Strait of Lorda), the rival's island
//! beyond it, joined at the top and bottom rows. The human's port on the strait (`port`), the
//! rival's city across it (`far`), open borders both ways, the region revealed to both.
//!
//! - `trade-ready`: the human's foreign route from the port to `far` has arrived (both caravans on
//!   it); the rival's route from `far` into the human's capital, its first caravan on the human's
//!   land between the port and the capital; a Caravan of the human's ready in the capital
//!   (`caravan`: Establish offers the port and `far`).
//! - `trade-routes`: as `trade-ready`, the Caravan has established the domestic route from the
//!   capital to the port, which has arrived, both its caravans on the road; the foreign route's
//!   second caravan was lost to the barbarians this turn; a Warrior of the human's next to the
//!   rival's caravan (`warrior`: the attack preview, at peace).
//! - `trade-closed`: as `trade-routes`, open borders ended and the routes checked: the foreign
//!   routes (the human's and the rival's) are closed.
//! - `trade-raid`: as `trade-routes`, the rival has a second city (`second`) on its island with a
//!   domestic route from it to `far` (`home`), arrived; then war between the two: the foreign
//!   routes close, the rival's domestic one runs; an Axeman of the human's (`warrior`) next to a
//!   caravan of it (`target`) on the road: the attack preview's plunder.

use std::path::Path;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;

/// The scene's region around the capital at `(cx, cy)`, toward `dir` (+1 east, -1 west): plot,
/// `dx` (in the direction), `dy`.
fn region(g: &Game, cx: i32, cy: i32, dir: i32) -> Option<Vec<(PlotIdx, i32, i32)>> {
    let mut out = Vec::new();
    for dy in -5..=5 {
        for dx in -4..=19 {
            out.push((g.plot_xy(cx, cy, dx * dir, dy)?, dx, dy));
        }
    }
    // A wrap that lands the region on itself is no region.
    let mut seen: Vec<PlotIdx> = out.iter().map(|e| e.0).collect();
    seen.sort();
    seen.dedup();
    (seen.len() == out.len()).then_some(out)
}

/// Writes the scene `kind` into `dir` (a game directory).
pub fn trade(dir: &Path, kind: &str) -> Result<Json, String> {
    if !["trade-ready", "trade-routes", "trade-closed", "trade-raid"].contains(&kind) {
        return Err(format!("no stage {kind}"));
    }
    for seed in [11, 3, 5, 7, 9, 13, 15, 17, 2, 4, 6, 8] {
        if let Some(j) = trade_seed(dir, kind, seed)? {
            return Ok(j);
        }
    }
    Err("no seed with room for the scene".into())
}

fn trade_seed(dir: &Path, kind: &str, seed: u32) -> Result<Option<Json>, String> {
    let mut s = super::duel_with(dir, seed, 4, None, &[])?;
    let h = s.human;
    let Some(rival) = s.g.player_ids().find(|&p| p != h && !s.g.player_is_barbarian(p) && s.g.players[p].alive) else { return Ok(None) };
    let g = &mut s.g;
    let d = g.d;
    let Some(&capital) = g.players[h].cities.first() else { return Ok(None) };
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let Some((plots, dir)) = [1, -1].into_iter().find_map(|dir| region(g, cx, cy, dir).map(|r| (r, dir))) else { return Ok(None) };
    let cap_plot = g.city_plot(capital);
    if plots.iter().any(|&(q, _, _)| q != cap_plot && g.map.plots[q].plot_city.is_some()) {
        return Ok(None);
    }
    // Clear the region: no units, the land redrawn.
    let remove: Vec<UnitId> = plots.iter().flat_map(|&(q, _, _)| g.map.plots[q].units.clone()).collect();
    for u in remove {
        if g.units.contains(u) {
            g.unit_kill(u, false, None);
        }
    }
    let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
    let inner = |dx: i32, dy: i32| dy.abs() <= 4 && (-3..=18).contains(&dx);
    let strait = |dx: i32, dy: i32| dy.abs() <= 3 && (7..=12).contains(&dx);
    for &(q, dx, dy) in &plots {
        if inner(dx, dy) {
            g.plot_set_plot_type(q, PlotType::PLOT_LAND, false);
            g.plot_set_terrain_type(q, grass, false);
            g.plot_set_feature_type(q, None);
            g.plot_set_bonus_type(q, None);
            g.plot_set_improvement_type(q, None);
        }
    }
    let mut water = Vec::new();
    for &(q, dx, dy) in &plots {
        if strait(dx, dy) {
            g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, false);
            g.plot_set_terrain_type(q, coast, false);
            water.push(q);
        }
    }
    g.map_recalculate_areas();
    g.update_plot_groups();
    let fresh: Vec<PlotIdx> = water.iter().copied().filter(|&q| g.map.plots[q].sea.is_none()).collect();
    if !fresh.is_empty() {
        g.map_cut_sea("Strait of Lorda", &fresh);
    }
    let at = |dx: i32, dy: i32| plots.iter().find(|e| e.1 == dx && e.2 == dy).map(|e| e.0).expect("in the region");
    let xy = |g: &Game, q: PlotIdx| g.map.xy(q);
    // Both teams know the region; they have met and keep open borders.
    let (ht, rt) = (g.players[h].team, g.players[rival].team);
    for &(q, _, _) in &plots {
        for t in [ht, rt] {
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    g.team_meet(ht, rt, false);
    g.team_set_open_borders(ht, rt, true);
    g.team_set_open_borders(rt, ht, true);
    // The cities: the human's port on the strait, the rival's city across it.
    let (px, py) = xy(g, at(6, 0));
    let port = g.player_init_city(h, px, py, true, true);
    let (fx, fy) = xy(g, at(13, 0));
    let far = g.player_init_city(rival, fx, fy, true, true);
    g.city_set_population(capital, 6);
    g.city_set_population(port, 4);
    g.city_set_population(far, 5);
    for &(q, _, _) in &plots {
        g.plot_update_working_city(q);
    }
    for c in [capital, port, far] {
        g.city_assign_working_plots(c);
    }
    // Revealed again: the new cities on their plots.
    for &(q, _, _) in &plots {
        for t in [ht, rt] {
            g.plot_set_revealed(q, t, false, false, None, false);
            g.plot_set_revealed(q, t, true, false, None, false);
        }
    }
    let caravan = g.caravan_unit_type().ok_or("no Caravan in the data")?;
    let make = |g: &mut Game, p: PlayerId, c: CityId| {
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        g.player_init_unit(p, caravan, x, y, None, None)
    };
    // Resources (docs/TRADE.md, 5): the human's Gems x3 and Fur x2 around the capital, Silk x2
    // and Dye x1 around the port; the rival's Silk x1 (around its capital). Each on a plot of the city's first ring,
    // improved, with a road; the techs that reveal and trade them given.
    // (The rival's Silk around its capital: a resource counts in its capital's network.)
    let Some(&rival_capital) = g.players[rival].cities.first() else { return Ok(None) };
    let stock: [(CityId, PlayerId, &str, &str, usize); 5] = [
        (capital, h, "BONUS_GEMS", "IMPROVEMENT_MINE", 3),
        (capital, h, "BONUS_FUR", "IMPROVEMENT_CAMP", 2),
        (port, h, "BONUS_SILK", "IMPROVEMENT_PLANTATION", 2),
        (port, h, "BONUS_DYE", "IMPROVEMENT_PLANTATION", 1),
        (rival_capital, rival, "BONUS_SILK", "IMPROVEMENT_PLANTATION", 1),
    ];
    let road = d.t::<RouteType>("ROUTE_ROAD");
    let mut used: Vec<PlotIdx> = Vec::new();
    for (c, p, b, imp, n) in stock {
        let (b, imp) = (d.t::<BonusType>(b), d.t::<ImprovementType>(imp));
        let team = g.players[p].team;
        for tech in [d.bonuses[b].tech_reveal, d.bonuses[b].tech_city_trade].into_iter().flatten() {
            g.team_set_has_tech(team, tech, true, Some(p), false, false);
        }
        let (cx0, cy0) = (g.cities[c].x, g.cities[c].y);
        let ring: Vec<PlotIdx> = (0..DirectionType::NUM).filter_map(|i| g.plot_direction(cx0, cy0, DirectionType::from_usize(i))).filter(|&q| !g.plot_is_water(q) && !used.contains(&q) && g.map.plots[q].plot_city.is_none() && g.map.plots[q].owner == Some(p)).collect();
        if ring.len() < n {
            return Ok(None);
        }
        for &q in &ring[..n] {
            g.plot_set_bonus_type(q, Some(b));
            g.plot_set_improvement_type(q, Some(imp));
            g.plot_set_route_type(q, Some(road), false);
            used.push(q);
        }
    }
    // A road from the capital to the port: one trade network (the capital's resources and the
    // port's are the same player's to send).
    for dx in 1..6 {
        g.plot_set_route_type(at(dx, 0), Some(road), false);
    }
    g.update_plot_groups();
    let gems = d.t::<BonusType>("BONUS_GEMS");
    let silk = d.t::<BonusType>("BONUS_SILK");
    // The foreign routes: the human's from the port (Gems and Silk), the rival's into the capital.
    let u = make(g, h, port);
    let Ok(foreign) = g.unit_establish_route(u, far, [Some(gems), Some(silk)]) else { return Err(format!("the foreign route: {:?}", g.unit_establish_refusal(u, far, [Some(gems), Some(silk)]))) };
    let u = make(g, rival, far);
    let Ok(theirs) = g.unit_establish_route(u, capital, [Some(silk), None]) else { return Ok(None) };
    // Turn ends until the human's route has arrived, and the rival's has and its first caravan,
    // on its way back, stands on land on the human's side of the strait (its own or no one's), off
    // the cities.
    let on_land = |g: &Game| -> Option<UnitId> {
        let v = g.routes[theirs].slots[0].unit?;
        let q = g.unit_plot(v);
        (g.routes[theirs].arrived && !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none() && g.plot_get_team(q).is_none_or(|t| t == ht)).then_some(v)
    };
    let mut theirs_unit = None;
    for _ in 0..30 {
        if !g.routes[foreign].arrived {
            g.player_move_caravans(h);
        }
        if theirs_unit.is_none() {
            g.player_move_caravans(rival);
            theirs_unit = on_land(g);
        }
        if g.routes[foreign].arrived && theirs_unit.is_some() {
            break;
        }
    }
    let Some(theirs_unit) = theirs_unit.filter(|_| g.routes[foreign].arrived) else { return Ok(None) };
    // Some growth: turn starts with both caravans alive.
    for _ in 0..6 {
        g.player_update_routes(h);
    }
    g.player_update_routes(rival);
    // A Scout of the human's on the land bridge over the strait: the caravans at sea there are in
    // sight (the shots draw one as a ship).
    let (bx, by) = xy(g, at(9, -4));
    g.player_init_unit(h, d.t::<UnitType>("UNIT_SCOUT"), bx, by, None, None);
    let ready = make(g, h, capital);
    let mut out = crate::jobj! {
        "stage" => kind,
        "capital" => capital,
        "port" => port,
        "far" => far,
        "rival" => rival,
        "foreign" => foreign,
        "theirs" => theirs,
        "theirs_unit" => theirs_unit,
        "cx" => cx, "cy" => cy,
        "px" => px, "py" => py,
        "fx" => fx, "fy" => fy,
        "dir" => dir,
    };
    if kind == "trade-ready" {
        out.set("caravan", ready);
    } else {
        let Ok(domestic) = g.unit_establish_route(ready, port, [None, None]) else { return Ok(None) };
        // Until it arrives, then one turn end more: both caravans on the road.
        for _ in 0..20 {
            g.player_move_caravans(h);
            if g.routes[domestic].arrived {
                break;
            }
        }
        if !g.routes[domestic].arrived {
            return Ok(None);
        }
        g.player_update_routes(h);
        g.player_move_caravans(h);
        g.player_update_routes(h);
        // The foreign route's second caravan, lost to the barbarians this turn.
        let barbs = g.barbarian_player();
        if let Some(v) = g.routes[foreign].slots[1].unit {
            g.unit_kill(v, false, Some(barbs));
        }
        // An army of the human's escorting the domestic route's first caravan (docs/TRADE.md 4.4).
        let (ex, ey) = (g.cities[capital].x, g.cities[capital].y);
        let warriors: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), ex, ey, None, None)).collect();
        let escort = g.army_form(h, &warriors).map_err(|e| format!("army: {e}"))?;
        g.army_set_station(h, escort, ffh_core::army::Station::Escort(ffh_core::army::EscortTarget::Caravan(domestic, 0))).map_err(|e| format!("escort: {e}"))?;
        out.set("escort", escort);
        out.set("escort_units", warriors);
        // A Caravan of the human's in the port, the foreign route's origin: Refit route.
        let (px0, py0) = (g.cities[port].x, g.cities[port].y);
        let refit = g.player_init_unit(h, caravan, px0, py0, None, None);
        out.set("refit_caravan", refit);
        // A Warrior of the human next to the rival's caravan.
        let tq = g.unit_plot(theirs_unit);
        let (tx, ty) = g.map.xy(tq);
        let spot = (0..DirectionType::NUM)
            .filter_map(|i| g.plot_direction(tx, ty, DirectionType::from_usize(i)))
            .find(|&q| !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none() && g.map.plots[q].units.is_empty())
            .ok_or("no plot for the warrior")?;
        let (wx, wy) = g.map.xy(spot);
        let warrior = g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), wx, wy, None, None);
        out.set("domestic", domestic);
        out.set("warrior", warrior);
        out.set("wx", wx);
        out.set("wy", wy);
        out.set("tx", tx);
        out.set("ty", ty);
        if kind == "trade-closed" {
            g.team_set_open_borders(ht, rt, false);
            g.team_set_open_borders(rt, ht, false);
            g.player_update_routes(h);
            g.player_update_routes(rival);
        }
        if kind == "trade-raid" {
            // A second city of the rival's on its island and its domestic route from there to
            // `far`; then war: the foreign routes close, the domestic one runs.
            let (sx, sy) = xy(g, at(17, 3));
            let second = g.player_init_city(rival, sx, sy, true, true);
            g.city_set_population(second, 4);
            for &(q, _, _) in &plots {
                for t in [ht, rt] {
                    g.plot_set_revealed(q, t, false, false, None, false);
                    g.plot_set_revealed(q, t, true, false, None, false);
                }
            }
            let u = make(g, rival, second);
            let Ok(home) = g.unit_establish_route(u, far, [None, None]) else { return Ok(None) };
            for _ in 0..20 {
                g.player_move_caravans(rival);
                if g.routes[home].arrived {
                    break;
                }
            }
            if !g.routes[home].arrived {
                return Ok(None);
            }
            g.player_update_routes(rival);
            g.player_move_caravans(rival);
            g.team_declare_war(ht, rt, false, None);
            // The warrior moves next to a caravan of the domestic route off the cities.
            let target = g.routes[home].caravans().into_iter().find(|&v| g.map.plots[g.unit_plot(v)].plot_city.is_none()).ok_or("no caravan on the road")?;
            let tq = g.unit_plot(target);
            let (tx, ty) = g.map.xy(tq);
            let spot = (0..DirectionType::NUM)
                .filter_map(|i| g.plot_direction(tx, ty, DirectionType::from_usize(i)))
                .find(|&q| !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none() && g.map.plots[q].units.is_empty())
                .ok_or("no plot for the warrior")?;
            let (wx, wy) = g.map.xy(spot);
            if g.units.contains(warrior) {
                g.unit_kill(warrior, false, None);
            }
            let w = g.player_init_unit(h, d.t::<UnitType>("UNIT_AXEMAN"), wx, wy, None, None);
            out.set("second", second);
            out.set("home", home);
            out.set("target", target);
            out.set("warrior", w);
            out.set("wx", wx);
            out.set("wy", wy);
            out.set("tx", tx);
            out.set("ty", ty);
        }
    }
    // Keep unrelated research/build prompts out of the walkthrough.
    if let Some(t) = d.techs.keys().find(|&t| g.player_can_research(h, t, false)) {
        g.player_push_research(h, t, true);
    }
    for city in g.players[h].cities.clone() {
        if !g.city_is_production(city) {
            if let Some(u) = d.units.keys().find(|&u| g.city_can_train(city, u, false, false, false, false)) {
                g.city_push_order(city, OrderType::ORDER_TRAIN, u.idx() as i32, -1, false, false, false, false);
            }
        }
    }
    out.set("turn", s.g.game_turn());
    let paths = s.save(kind).map_err(|e| format!("save: {e:?}"))?;
    out.set("save", paths.save.display().to_string());
    Ok(Some(out))
}
