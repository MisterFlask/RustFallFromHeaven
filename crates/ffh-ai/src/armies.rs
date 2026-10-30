//! Armies (this game's own rules, `GAMEOPTION_ARMIES`; docs/ARMIES.md; docs/AI.md "Armies").
//!
//! The AI's groups become armies by themselves (the rules' army sync names every group of army
//! units), so its movement logic drives each army unchanged: a city-attack stack is an army, a
//! city's defenders are its garrison. What an expert adds is logistics:
//!
//! - **Reinforcements by transit.** At war, or preparing one, the cities training attack units
//!   (attack, city attack, collateral, counter roles) send them to the main field army -- the
//!   largest army in supply, nearest the enemy on ties -- instead of walking them there: they
//!   travel off the map and join it. Defenders and everything else stay in their city's garrison.
//! - **Target size.** The main army's target is the army the war needs
//!   (`diplomacy::army_needed_for_war`), so its reinforcements show how far it is from it.
//!
//! Switch `ARMIES` (`FFH_AI_ARMIES_OFF`): off, every unit stays in its city's garrison when
//! trained (the armies still form from the groups).

use ffh_core::army::RecruitDest;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// The unit roles that go to the field army.
const FIELD_ROLES: [UnitAIType; 4] = [UnitAIType::UNITAI_ATTACK, UnitAIType::UNITAI_ATTACK_CITY, UnitAIType::UNITAI_COLLATERAL, UnitAIType::UNITAI_COUNTER];

pub fn turn(g: &mut Game, p: PlayerId) {
    if !g.armies_on() || !crate::switches::ARMIES.on(g, p) || g.player_is_barbarian(p) {
        return;
    }
    let team = g.players[p].team;
    let target = crate::diplomacy::preparing_target(g, team).or_else(|| g.team_ids().find(|&t| t != team && g.team_is_alive(t) && !g.team_is_barbarian(t) && g.at_war(team, t)));
    let main = target.and_then(|t| main_army(g, p, t));
    if let (Some(a), Some(t)) = (main, target) {
        let need = crate::diplomacy::army_needed_for_war(g, p, t);
        if g.armies[a].target_size != need {
            let _ = g.army_set_target_size(p, a, need);
        }
    }
    for c in g.players[p].cities.clone() {
        for i in 0..g.cities[c].order_queue.len() {
            let o = &g.cities[c].order_queue[i];
            if o.order_type != OrderType::ORDER_TRAIN || o.dest.is_some() {
                continue;
            }
            let unit = UnitType::from_usize(o.data1 as usize);
            let field = opt_from_i32::<UnitAIType>(o.data2).is_some_and(|r| FIELD_ROLES.contains(&r));
            if !field || !g.unit_type_is_army_unit(unit) {
                continue;
            }
            if let Some(a) = main {
                g.cities[c].order_queue[i].dest = Some(RecruitDest::Army(a));
                crate::trace!(g, p, "armies: {} trains for the {}", g.d.text.name(&g.cities[c].name), g.armies[a].name);
            }
        }
    }
}

/// Our main field army against `target`: the largest army (at least two units) in supply and not
/// disbanding, the nearest to `target`'s cities on ties.
fn main_army(g: &Game, p: PlayerId, target: TeamId) -> Option<ArmyId> {
    let mut best: Option<(i32, i32, ArmyId)> = None;
    for &a in &g.players[p].armies {
        if !g.armies.contains(a) || g.armies[a].disband_to.is_some() || !g.army_in_supply(a) {
            continue;
        }
        let size = g.army_units(a).len() as i32;
        if size < 2 {
            continue;
        }
        let Some(plot) = g.army_plot(a) else { continue };
        let (x, y) = g.map.xy(plot);
        let dist = g
            .player_ids()
            .filter(|&q| g.players[q].alive && g.players[q].team == target)
            .flat_map(|q| g.players[q].cities.iter().map(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y)).collect::<Vec<_>>())
            .min()
            .unwrap_or(i32::MAX / 2);
        let key = (size, -dist, a);
        if best.map_or(true, |(s, d, b)| key > (s, d, b)) {
            best = Some(key);
        }
    }
    best.map(|(_, _, a)| a)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At war, a city's attack unit goes to the main field army (in transit); a defender stays in
    /// the city's garrison, and the army's target size is the war's.
    #[test]
    fn at_war_attack_units_reinforce_the_main_army() {
        let mut g = crate::testkit::one_landmass_game(4);
        g.setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        let ours = g.player_init_city(p0, 4, 5, true, true);
        g.player_init_city(p1, 14, 5, true, true);
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        let w = g.d.t::<UnitType>("UNIT_WARRIOR");
        // Three plots out of the city (no roads: its plot is the network), so in supply.
        let army: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, w, 7, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &army).unwrap();
        let (attack, defend) = (UnitAIType::UNITAI_ATTACK.idx() as i32, UnitAIType::UNITAI_CITY_DEFENSE.idx() as i32);
        g.city_push_order(ours, OrderType::ORDER_TRAIN, w.idx() as i32, attack, false, false, true, false);
        g.city_push_order(ours, OrderType::ORDER_TRAIN, w.idx() as i32, defend, false, false, true, false);
        turn(&mut g, p0);
        let q = &g.cities[ours].order_queue;
        assert_eq!(q[0].dest, Some(RecruitDest::Army(a)), "the attacker to the army");
        assert_eq!(q[1].dest, None, "the defender stays home");
        assert_eq!(g.armies[a].target_size, crate::diplomacy::army_needed_for_war(&g, p0, t1));
    }
}
