//! Empire-level choices: research queue, civics, state religion, commerce sliders.

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

pub fn manage(g: &mut Game, p: PlayerId) {
    research(g, p);
    civics(g, p);
    religion(g, p);
    sliders(g, p);
    disband(g, p);
    hurry(g, p);
}

/// Hurry production where it matters (FfH allows it under some civics: gold under Arete,
/// Caste System, Guilds, Military State; population under Slavery): a defender for a city
/// under threat, a settler or worker; with gold well above the reserve, anything nearly done
/// buys its last turns. Population only from a city at its happy cap, one citizen at a time,
/// once the anger from the last one has passed.
fn hurry(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let hurries: Vec<HurryType> = d.hurries.keys().filter(|&h| g.player_can_hurry(p, h)).collect();
    if hurries.is_empty() {
        return;
    }
    let reserve = 30 + 5 * g.players[p].cities.len() as i32;
    for c in g.players[p].cities.clone() {
        let urgent = match g.city_production_unit(c) {
            Some(u) => {
                let info = &d.units[u];
                let plot = g.city_plot(c);
                let threatened = crate::production::local_threat(g, p, plot, 2) > crate::production::garrison(g, p, plot).1;
                info.found || info.work_rate > 0 && info.combat == 0 || threatened && info.combat > 0
            }
            None => false,
        };
        let turns = g.city_production_turns_left(c);
        if turns <= 1 {
            continue;
        }
        for &h in &hurries {
            if !g.city_can_hurry(c, h, false) {
                continue;
            }
            if d.hurries[h].gold_per_production > 0 {
                let cost = g.city_hurry_gold(c, h);
                let rich = g.players[p].gold - cost > reserve * 3 && turns <= 4;
                if (urgent && g.players[p].gold - cost >= reserve) || rich {
                    crate::trace!(g, p, "hurries {} with {} gold", g.cities[c].name, cost);
                    g.city_hurry(c, h);
                    break;
                }
            } else if d.hurries[h].production_per_population > 0 {
                let capped = g.city_happy_level(c) - g.city_unhappy_level(c, 0) <= 0;
                if urgent && capped && g.cities[c].population >= 4 && g.city_hurry_population(c, h) == 1 && g.cities[c].hurry_anger_timer == 0 {
                    crate::trace!(g, p, "hurries {} with a citizen", g.cities[c].name);
                    g.city_hurry(c, h);
                    break;
                }
            }
        }
    }
}

/// Units the empire cannot pay for: idle workers while support keeps research below its target
/// (`finance::disband_workers`), military units by More Naval AI's rule
/// (`finance::disband_military`).
fn disband(g: &mut Game, p: PlayerId) {
    if !g.player_is_commerce_flexible(p, CommerceType::COMMERCE_RESEARCH) || g.player_calculate_unit_cost(p) == 0 {
        return;
    }
    crate::finance::disband_workers(g, p);
    crate::finance::disband_military(g, p);
}

fn research(g: &mut Game, p: PlayerId) {
    if crate::finance::escape_research(g, p) { return; } // bankruptcy-guard
    if !g.players[p].research_queue.is_empty() {
        return;
    }
    if let Some(t) = crate::research::choose(g, p) {
        g.player_push_research(p, t, true);
    }
}

/// Value of running `civic` (AI heuristic).
pub(crate) fn civic_value(g: &Game, p: PlayerId, civic: CivicType) -> i32 {
    let d = g.d;
    let info = &d.civics[civic];
    let pl = &g.players[p];
    let n = pl.cities.len() as i32;
    let pop = pl.total_population;
    // Conquest (`bMilitaryFoodProduction`): a city training a military unit puts its whole food
    // surplus into the unit and stops growing, and production trains units most of the time.
    // Its XML weight (700 a city) outbid every other economy civic, so most civs ran it and their
    // cities stalled; it gets none, and is valued like any civic by its other terms.
    let mut v = if info.military_food_production { 0 } else { info.ai_weight * n.max(1) };
    if let Some(l) = pl.personality_type.map(|l| &d.leader_heads[l]) {
        if l.favorite_civic == Some(civic) {
            v += 300 + 50 * n;
        }
    }
    let yw = [3, 3, 2];
    for i in 0..YieldType::NUM {
        v += info.yield_modifier[i] * pop * yw[i] / 10;
        v += info.capital_yield_modifier[i] * yw[i];
    }
    for i in 0..CommerceType::NUM {
        v += info.commerce_modifier[i] * pl.commerce_rate[i].max(0) / 25;
        v += info.capital_commerce_modifier[i];
    }
    v += info.largest_city_happiness * 20 + info.extra_health * 15 * n;
    if pl.state_religion_count > 0 || g.player_get_state_religion(p).is_some() {
        v += info.state_religion_happiness * 20 * n;
    }
    v += info.non_state_religion_happiness * 10 * n;
    v += info.free_experience * 15 + info.free_specialist * 30 * n;
    v += info.trade_routes * 25 * n;
    v += info.worker_speed_modifier / 2 + info.military_production_modifier / 2;
    v -= info.war_weariness_modifier / 4;
    let broke = crate::economy::spend_pct(g, p) < 50;
    let gold_w = if broke { 40 } else { 15 };
    let gold_w = gold_w.max(crate::finance::gold_value(&crate::finance::state(g, p))); // bankruptcy-guard
    v -= g.player_get_single_civic_upkeep(p, Some(civic), true) * gold_w;
    // City maintenance (the cities' distance and number-of-cities parts) and FfH's AI-only build bans.
    v -= crate::finance::civic_maintenance_x100(g, p, civic) * gold_w / 100 + crate::finance::civic_penalty(g, p, civic); // bankruptcy-guard
    // Happiness from garrisons (FfH God King / BtS Hereditary Rule), worth most to capped cities.
    let capped = pl.cities.iter().filter(|&&c| g.city_happy_level(c) - g.city_unhappy_level(c, 0) <= 0).count() as i32;
    if info.happy_per_military_unit != 0 {
        v += info.happy_per_military_unit * 2 * (40 * capped + 10 * (n - capped));
    }
    v -= info.food_consumption_per_population * pop * 20;
    // Yields on the improvements we work.
    if info.improvement_yield_changes.iter().any(|y| y.iter().any(|&x| x != 0)) {
        for &c in &pl.cities {
            for k in 0..NUM_CITY_PLOTS {
                if k == CITY_HOME_PLOT || !g.cities[c].working_plot[k] {
                    continue;
                }
                let Some(q) = g.city_index_plot(c, k) else { continue };
                if let Some(imp) = g.map.plots[q].improvement_type {
                    let y = info.improvement_yield_changes[imp];
                    v += y[0] * 20 + y[1] * 15 + y[2] * 9;
                }
            }
        }
    }
    for k in 0..CommerceType::NUM {
        v += info.specialist_extra_commerce[k] * pl.cities.iter().map(|&c| g.cities[c].specialist_population).sum::<i32>() * 8;
    }
    crate::civs::civic_value(g, p, civic, v) // civ-mechanics
}

fn civics(g: &mut Game, p: PlayerId) {
    if g.players[p].revolution_timer > 0 || g.player_is_anarchy(p) {
        return;
    }
    // Re-evaluate at most every 10 turns per player (staggered), as revolutions cost anarchy.
    if (g.game_turn() + p.0 as i32) % 10 != 0 && !crate::finance::civics_due(g, p) { // bankruptcy-guard
        return;
    }
    let d = g.d;
    let current = g.players[p].civics.clone();
    let mut new = current.clone();
    let mut gain = 0;
    for opt in d.civic_options.keys() {
        let mut best: Option<(CivicType, i32)> = current[opt].map(|c| (c, civic_value(g, p, c)));
        let cur_value = best.map_or(i32::MIN, |(_, v)| v);
        for c in d.civics.keys() {
            if d.civics[c].civic_option_type != Some(opt) || !g.player_can_do_civics(p, Some(c)) {
                continue;
            }
            let v = civic_value(g, p, c);
            if best.map_or(true, |(_, b)| v > b) {
                best = Some((c, v));
            }
        }
        if let Some((c, v)) = best {
            if Some(c) != current[opt] {
                new[opt] = Some(c);
                gain += v - cur_value.max(0);
            }
        }
    }
    if new != current && gain > 100 && g.player_can_revolution(p, Some(&new)) {
        g.player_revolution(p, &new, false);
    }
}

fn religion(g: &mut Game, p: PlayerId) {
    if !g.player_can_change_religion(p) {
        return;
    }
    let d = g.d;
    let current = g.player_get_state_religion(p);
    let fav = g.players[p].personality_type.and_then(|l| d.leader_heads[l].favorite_religion);
    let mut best: Option<(ReligionType, i32)> = None;
    for r in d.religions.keys() {
        let count = g.players[p].has_religion_count[r];
        if count == 0 || !g.player_can_convert(p, Some(r)) && current != Some(r) {
            continue;
        }
        let mut v = count * 10;
        if fav == Some(r) {
            v += 25;
        }
        if g.player_has_holy_city(p, r) {
            v += 20;
        }
        if current == Some(r) {
            v += 15; // conversion costs anarchy
        }
        if best.map_or(true, |(_, b)| v > b) {
            best = Some((r, v));
        }
    }
    if let Some((r, _)) = best {
        if current != Some(r) && g.player_can_convert(p, Some(r)) {
            g.player_convert(p, Some(r));
        }
    }
}

/// Highest research rate that keeps the treasury from running dry. A cultural victory first
/// sets the culture slider (`victory::culture_slider`), lowered in steps when even no research
/// leaves the treasury short. Saving gold to upgrade the army at war was measured and reverted
/// (docs/AI_LOG.md, "Unit choices and an upgrade reserve").
fn sliders(g: &mut Game, p: PlayerId) {
    if !g.player_is_commerce_flexible(p, CommerceType::COMMERCE_RESEARCH) {
        return;
    }
    let gold = g.players[p].gold;
    let reserve = 20 + 5 * g.players[p].cities.len() as i32;
    let mut culture = crate::victory::culture_slider(g, p); // victory-pursuit
    loop {
        if g.players[p].commerce_percent[CommerceType::COMMERCE_CULTURE.idx()] != culture {
            g.player_set_commerce_percent(p, CommerceType::COMMERCE_CULTURE, culture);
        }
        let mut pct = 100 - culture;
        let mut paid = false;
        while pct >= 0 {
            if g.players[p].commerce_percent[CommerceType::COMMERCE_RESEARCH.idx()] != pct {
                g.player_set_commerce_percent(p, CommerceType::COMMERCE_RESEARCH, pct);
            }
            let rate = g.player_calculate_gold_rate(p);
            if rate >= 0 || gold + rate * 10 > reserve {
                paid = true;
                break;
            }
            pct -= 10;
        }
        if paid || culture == 0 {
            break;
        }
        culture = (culture - 20).max(0);
    }
}
