//! Cross-checks of the redundant bookkeeping the rules keep (C++ caches and counters that
//! must agree with the objects they count). A disagreement means a port bug: the C++ updates
//! all of them together.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::Idx;

/// Panics with `invariant: ...` on the first disagreement.
pub fn check(g: &Game) {
    let fail = |msg: String| -> ! { panic!("invariant: {msg} (turn {})", g.game_turn()) };

    // Slots (players added mid-game, `Game::add_player_slot`): one team slot, setup entry and
    // controller per player slot, and the per-slot arrays sized to them.
    let n = g.num_players();
    if g.num_teams() != n || g.setup.players.len() != n || g.controllers.len() != n {
        fail(format!("{n} player slots, {} teams, {} setups, {} controllers", g.num_teams(), g.setup.players.len(), g.controllers.len()));
    }
    for p in g.player_ids() {
        let pl = &g.players[p];
        if pl.alive && (pl.ai_attitude_extra.len() != n || pl.gold_per_turn_by_player.len() != n) {
            fail(format!("player {}'s per-player arrays are not sized to {n} slots", p.0));
        }
    }
    for t in g.team_ids() {
        let tm = &g.teams[t];
        if tm.at_war.len() != n || tm.has_met.len() != n {
            fail(format!("team {}'s per-team arrays are not sized to {n} slots", t.0));
        }
    }
    if let Some(plot) = g.map.plots.first() {
        if plot.culture.len() != n || plot.revealed.len() != n {
            fail(format!("the plots' per-slot arrays are not sized to {n} slots"));
        }
    }

    // The range fields at sea (docs/NAVAL.md): a cached field equals a fresh one.
    let stale = g.sea_range_cache_stale();
    if !stale.is_empty() {
        fail(format!("the cached range field at sea of teams {stale:?} is stale"));
    }

    // Cities: owner lists, plots, population, team and area counters.
    let mut team_cities = vec![0i32; g.num_teams()];
    let mut team_pop = vec![0i32; g.num_teams()];
    for p in g.player_ids() {
        let pl = &g.players[p];
        let mut pop = 0;
        for &c in &pl.cities {
            if !g.cities.contains(c) {
                fail(format!("player {} lists dead city {c:?}", p.0));
            }
            let city = &g.cities[c];
            if city.owner != p {
                fail(format!("player {} lists city {} owned by {}", p.0, city.name, city.owner.0));
            }
            let plot = g.city_plot(c);
            if g.map.plots[plot].plot_city != Some(c) {
                fail(format!("city {} of player {} is not on its plot", city.name, p.0));
            }
            pop += city.population;
        }
        if pop != pl.total_population {
            fail(format!("player {} total_population {} != sum of city populations {pop}", p.0, pl.total_population));
        }
        team_cities[pl.team.idx()] += pl.cities.len() as i32;
        team_pop[pl.team.idx()] += pop;
    }
    if g.cities.len() != g.player_ids().map(|p| g.players[p].cities.len()).sum::<usize>() {
        fail(format!("{} cities exist but players list {}", g.cities.len(), g.player_ids().map(|p| g.players[p].cities.len()).sum::<usize>()));
    }
    for t in g.team_ids() {
        if g.teams[t].num_cities != team_cities[t.idx()] {
            fail(format!("team {} num_cities {} != its players' cities {}", t.0, g.teams[t].num_cities, team_cities[t.idx()]));
        }
        if g.teams[t].total_population != team_pop[t.idx()] {
            fail(format!("team {} total_population {} != its players' {}", t.0, g.teams[t].total_population, team_pop[t.idx()]));
        }
    }
    for a in g.map.areas.ids() {
        let area = &g.map.areas[a];
        let mut n = 0;
        let mut per_player = vec![0i32; g.num_players()];
        let mut pop_per_player = vec![0i32; g.num_players()];
        for (c, city) in g.cities.iter() {
            if g.city_area(c) == a {
                n += 1;
                per_player[city.owner.idx()] += 1;
                pop_per_player[city.owner.idx()] += city.population;
            }
        }
        if area.num_cities != n {
            fail(format!("area {a:?} num_cities {} != {n}", area.num_cities));
        }
        for p in g.player_ids() {
            if area.cities_per_player[p] != per_player[p.idx()] {
                fail(format!("area {a:?} cities_per_player[{}] {} != {}", p.0, area.cities_per_player[p], per_player[p.idx()]));
            }
            if area.population_per_player[p] != pop_per_player[p.idx()] {
                fail(format!("area {a:?} population_per_player[{}] {} != {}", p.0, area.population_per_player[p], pop_per_player[p.idx()]));
            }
        }
    }

    // Building tiers (docs/SUITABILITY.md): a city holds only its own tier's variant of a
    // building, and its queue builds only that one.
    for (c, city) in g.cities.iter() {
        for b in g.d.buildings.keys() {
            if (city.num_real_building[b] > 0 || city.num_free_building[b] > 0) && !g.city_is_own_tier(c, b) {
                fail(format!("city {} holds {} of another tier than its own", city.name, g.d.buildings[b].base.type_));
            }
        }
        for o in &city.order_queue {
            if o.order_type == ffh_core::data::OrderType::ORDER_CONSTRUCT {
                let b = ffh_core::data::BuildingType::from_usize(o.data1 as usize);
                if !g.city_is_own_tier(c, b) {
                    fail(format!("city {} queues {} of another tier than its own", city.name, g.d.buildings[b].base.type_));
                }
            }
        }
    }

    // Units: owner lists, plot lists, groups.
    let mut listed = 0;
    for p in g.player_ids() {
        for &u in &g.players[p].units {
            listed += 1;
            if !g.units.contains(u) {
                fail(format!("player {} lists dead unit {u:?}", p.0));
            }
            let unit = &g.units[u];
            if unit.owner != p {
                fail(format!("player {} lists unit {u:?} owned by {}", p.0, unit.owner.0));
            }
            if let Some(plot) = g.map.plot_xy(unit.x, unit.y) {
                if !g.map.plots[plot].units.contains(&u) {
                    fail(format!("unit {u:?} of player {} at ({}, {}) is not in its plot's list", p.0, unit.x, unit.y));
                }
            }
            match unit.group {
                Some(grp) if g.groups.contains(grp) => {
                    if !g.groups[grp].units.contains(&u) {
                        fail(format!("unit {u:?} of player {} is not in its group's list", p.0));
                    }
                    if g.groups[grp].owner != p {
                        fail(format!("unit {u:?} of player {} is in a group of player {}", p.0, g.groups[grp].owner.0));
                    }
                }
                Some(grp) => fail(format!("unit {u:?} of player {} is in dead group {grp:?}", p.0)),
                None => {}
            }
            if let Some(t) = unit.transport_unit {
                if !g.units.contains(t) {
                    fail(format!("unit {u:?} of player {} is carried by dead unit {t:?}", p.0));
                }
            }
        }
    }
    if listed != g.units.len() {
        fail(format!("{} units exist but players list {listed}", g.units.len()));
    }
    for plot in 0..g.map.plots.len() {
        let pl = &g.map.plots[PlotIdx::from_usize(plot)];
        for &u in &pl.units {
            if !g.units.contains(u) {
                fail(format!("plot ({}, {}) lists dead unit {u:?}", pl.x, pl.y));
            }
            if (g.units[u].x, g.units[u].y) != (pl.x, pl.y) {
                fail(format!("plot ({}, {}) lists unit {u:?} at ({}, {})", pl.x, pl.y, g.units[u].x, g.units[u].y));
            }
        }
    }
    for (grp, group) in g.groups.iter() {
        for &u in &group.units {
            if !g.units.contains(u) {
                fail(format!("group {grp:?} lists dead unit {u:?}"));
            }
            if g.units[u].group != Some(grp) {
                fail(format!("group {grp:?} lists unit {u:?} of group {:?}", g.units[u].group));
            }
        }
    }

    // This game's armies (docs/ARMIES.md): players list their own live armies, and a unit's army
    // is its owner's.
    let mut armies = 0;
    for p in g.player_ids() {
        for &a in &g.players[p].armies {
            armies += 1;
            match g.armies.get(a) {
                None => fail(format!("player {} lists dead army {a:?}", p.0)),
                Some(army) if army.owner != p => fail(format!("player {} lists army {a:?} of player {}", p.0, army.owner.0)),
                Some(_) => {}
            }
        }
        for &u in &g.players[p].units {
            if let Some(a) = g.units[u].army {
                if g.armies.get(a).map_or(true, |army| army.owner != p) {
                    fail(format!("unit {u:?} of player {} is in army {a:?}, dead or not its owner's", p.0));
                }
                if g.units[u].march.is_some() {
                    fail(format!("unit {u:?} of player {} marches but is in army {a:?}", p.0));
                }
            }
        }
        // Requisition orders, then empire orders, lead their city's queue (docs/GOVERNORS.md);
        // empire orders only in governed cities.
        for &c in &g.players[p].cities {
            let q = &g.cities[c].order_queue;
            let req = q.iter().take_while(|o| o.requisition).count();
            if q[req..].iter().any(|o| o.requisition) {
                fail(format!("city {} of player {} has a requisition order behind its own production", g.cities[c].name, p.0));
            }
            let lead = g.city_lead_orders(c);
            if q[lead..].iter().any(|o| o.empire) {
                fail(format!("city {} of player {} has an empire order behind its own production", g.cities[c].name, p.0));
            }
            if q.iter().any(|o| o.empire && o.requisition) {
                fail(format!("city {} of player {} has an order both empire and requisition", g.cities[c].name, p.0));
            }
            if lead > req && !g.city_is_governed(c) {
                fail(format!("directly ruled city {} of player {} has empire orders", g.cities[c].name, p.0));
            }
        }
    }
    if armies != g.armies.len() {
        fail(format!("{} armies exist but players list {armies}", g.armies.len()));
    }

    // This game's engagement combat (docs/ENGAGEMENTS.md): engagements leave no duel state
    // behind (no unit fighting or attacking between turns, no living unit at full damage), and
    // every kept report is consistent: no battle ran past ENGAGEMENT_ROUNDS, no side's formation
    // used more slots than it had or fewer than its units, every committed unit is accounted for
    // (sat out, fallen, escaped or standing), a taken plot has no defender left; a logged report's
    // fighters stand inside their side's lanes and rows, one per cell, and match its counts.
    let max_hp = g.d.defines.max_hit_points;
    for (u, unit) in g.units.iter() {
        if unit.combat_unit.is_some() || unit.attack_plot_x != ffh_core::defs::INVALID_PLOT_COORD {
            fail(format!("unit {u:?} of player {} is still in combat after the turn", unit.owner.0));
        }
        if unit.damage >= max_hp && !unit.death_delay {
            fail(format!("unit {u:?} of player {} is alive at {} damage", unit.owner.0, unit.damage));
        }
    }
    let max_rounds = g.d.defines.engagement_rounds;
    for r in &g.battle_reports {
        if r.outcome.rounds > max_rounds || r.outcome.rounds < 0 {
            fail(format!("battle report #{} ran {} rounds", r.id, r.outcome.rounds));
        }
        if r.outcome.result == ffh_core::engagement::EngageResult::Taken && r.outcome.defenders_left != 0 {
            fail(format!("battle report #{}: the plot fell with {} defenders left", r.id, r.outcome.defenders_left));
        }
        for (i, s) in r.sides.iter().enumerate() {
            // A walled city without soldiers fights with its Sentinels alone (docs/SIEGES.md):
            // its side commits no unit.
            let sentinels_only = i == 1 && r.outcome.sentinels > 0;
            if s.sat_out + s.fallen + s.escaped + s.standing != s.committed || (s.committed <= 0 && !sentinels_only) {
                fail(format!(
                    "battle report #{} side {i}: {} committed, {} sat out, {} fell, {} escaped, {} standing",
                    r.id, s.committed, s.sat_out, s.fallen, s.escaped, s.standing
                ));
            }
            if s.lost.iter().map(|(_, n)| n).sum::<i32>() != s.fallen {
                fail(format!("battle report #{} side {i}: {} fell, its losses by type {:?}", r.id, s.fallen, s.lost));
            }
            // The units called into the battle (summons, units a kill created) fill no slot.
            if s.used > s.slots || s.used < s.committed - s.summons {
                fail(format!("battle report #{} side {i}: {} units filled {} of {} slots", r.id, s.committed, s.used, s.slots));
            }
        }
        if let Some(log) = &r.log {
            for (i, s) in r.sides.iter().enumerate() {
                let slots = &log.slots[i];
                if s.slots != slots.total + slots.river_only || s.lanes != log.lanes[i] as i32 {
                    fail(format!(
                        "battle report #{} side {i} has {} slots and {} lanes, its log {} and {}",
                        r.id,
                        s.slots,
                        s.lanes,
                        slots.total + slots.river_only,
                        log.lanes[i]
                    ));
                }
                // A city's Sentinels are placed fighters of the defender, counted in the outcome's
                // `sentinels`, not among its committed units (docs/SIEGES.md).
                let placed: Vec<&ffh_core::engagement::report::LogFighter> = log.fighters.iter().filter(|f| f.side.index() == i && f.place.is_some()).collect();
                let sentinels = placed.iter().filter(|f| f.sentinel).count() as i32;
                // A unit a kill created enters the field with an event (no cell in the log); one
                // none created never fought.
                let entered = log.fighters.iter().filter(|f| f.side.index() == i && f.on_kill && !matches!(f.fate, Some(ffh_core::engagement::report::LogFate::SatOut))).count() as i32;
                let units = placed.len() as i32 - sentinels + entered;
                let want_sentinels = if i == 1 { r.outcome.sentinels } else { 0 };
                if units != s.committed || sentinels != want_sentinels || log.fighters.iter().any(|f| !f.on_kill && f.place.is_some() != f.fate.is_some()) {
                    fail(format!(
                        "battle report #{} side {i}: {} committed and {want_sentinels} Sentinels, {units} placed units and {sentinels} placed Sentinels in its log",
                        r.id, s.committed
                    ));
                }
                let mut cells = std::collections::BTreeSet::new();
                for f in placed {
                    let Some((lane, row)) = f.place else { continue };
                    let last = lane as i32 + f.large as i32;
                    if row as usize >= ffh_core::data::ROWS || last >= log.lanes[i] as i32 {
                        fail(format!("battle report #{} side {i}: unit {:?} stands off its grid at {:?}", r.id, f.unit, f.place));
                    }
                    for l in lane as i32..=last {
                        if !cells.insert((l, row)) {
                            fail(format!("battle report #{} side {i}: two units in lane {l} row {row}", r.id));
                        }
                    }
                }
            }
        }
    }

    // This game's governors (docs/GOVERNORS.md): a governed city always has production (an FfH
    // settlement builds nothing at all) and a role; a governor is its owner's and holds the city
    // that names it; every city of a player is in the mode its civics give.
    for p in g.player_ids() {
        let direct = g.player_rules_directly(p);
        for &c in &g.players[p].cities {
            if g.city_is_governed(c) == direct {
                fail(format!("city {} of player {} is not in its owner's mode (direct rule: {direct})", g.cities[c].name, p.0));
            }
            if g.city_is_governed(c) && !g.cities[c].settlement && g.cities[c].order_queue.is_empty() {
                fail(format!("governed city {} of player {} has no production", g.cities[c].name, p.0));
            }
            if g.city_is_governed(c) && (g.cities[c].role.is_none() || g.cities[c].governor.is_none()) {
                fail(format!("governed city {} of player {} has no role or no governor", g.cities[c].name, p.0));
            }
            if let Some(gv) = g.cities[c].governor {
                match g.governors.get(gv) {
                    Some(x) if x.owner == p && x.city == c => {}
                    _ => fail(format!("city {} of player {} names governor {gv:?}, not its own", g.cities[c].name, p.0)),
                }
            }
        }
        for &gv in &g.players[p].governors {
            match g.governors.get(gv) {
                None => fail(format!("player {} lists dead governor {gv:?}", p.0)),
                Some(x) if x.owner != p => fail(format!("player {} lists governor {gv:?} of player {}", p.0, x.owner.0)),
                Some(x) => {
                    if !g.cities.contains(x.city) || g.cities[x.city].governor != Some(gv) {
                        fail(format!("governor {gv:?} of player {} holds city {:?}, which does not name it", p.0, x.city));
                    }
                }
            }
        }
    }
}
