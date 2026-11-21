//! Raids (docs/RAIDS.md, section 9): an army of ours that would only hold where it stands on an
//! enemy city's countryside takes the Raid order on that city, and keeps it while the guard would
//! stay in. The rules' side (the order, the guard's sallies, the take) is `ffh_core::raid`.
//!
//! - **Taking the order** ([`at_hold`]): at the hold points of `military::army` (the idle hold,
//!   where a stack with nothing to do would hold or walk home) and of `campaign::act` (a stack
//!   waiting two plots or more from its operation's target: March and Gather waiting, a move the
//!   supply gate refused), a land army at war takes the order on the city whose countryside it
//!   stands on when the rules allow it (`army_raid_refusal`), it is not the besieger next to its
//!   operation's target in a Strike, StepIn or March phase, taking it would not end that city's
//!   investment, its share of the forecast take is at least 1 gold, and the guard's sally
//!   forecast against its plot is below `RAID_SALLY_ODDS` (asked last: it can cost a game copy).
//! - **Keeping it** ([`keep`], before the hunt in `military::army`, and [`at_hold`] at the same
//!   hold points): a raiding stack holds, unless the guard's forecast is now at or above
//!   `RAID_SALLY_ODDS` (the order ends and the stack decides as any other; asked before the stack
//!   may stop to heal in place too, [`still_raiding`]: healing is no move). The branches above
//!   the hunt (an assault, a city of ours in peril, a cut supply line, the campaign's strike or
//!   step) move it, and a move ends the order by rule; the branches below (hunt, the Raiders'
//!   walk, gathering, the walk home) never see a stack whose raid holds and pays. Next to its
//!   operation's target, a stack raiding that city stops ([`stop_raiding`]): it is the besieger.
//! - The guard's posture stays the rules' default (`Favoured`).

use ffh_core::army::Station;
use ffh_core::game::Game;
use ffh_core::ids::*;

use crate::campaign::Phase;
use crate::units::{alive, hold};
use crate::view::View;

/// A city's name in the traces.
fn city_name(g: &Game, c: CityId) -> &str {
    g.d.text.name(&g.cities[c].name)
}

fn dist(g: &Game, a: PlotIdx, b: PlotIdx) -> i32 {
    let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
    g.plot_distance(ax, ay, bx, by)
}

/// The land army whose group `grp` is (its head's army, the army's group being `grp`).
pub(crate) fn stack_army(g: &Game, grp: GroupId) -> Option<ArmyId> {
    let a = crate::supply::army_of(g, grp)?;
    let army = &g.armies[a];
    (army.domain == ffh_core::army::ArmyDomain::Land && army.group == Some(grp)).then_some(a)
}

/// The guard's sally forecast (`won_permille`) of city `c` against `plot`, asked once per stack
/// and plot this turn (`View::raid_sally`).
fn sally_forecast(g: &Game, v: &mut View, grp: GroupId, c: CityId, plot: PlotIdx) -> Option<i32> {
    if let Some(&(_, _, won)) = v.raid_sally.iter().find(|&&(gr, q, _)| gr == grp && q == plot) {
        return won;
    }
    let won = g.raid_sally_forecast(c, plot);
    v.raid_sally.push((grp, plot, won));
    won
}

/// Would the guard of `c` sally against `plot` (its forecast at or above `RAID_SALLY_ODDS`)?
fn guard_comes_out(g: &Game, v: &mut View, grp: GroupId, c: CityId, plot: PlotIdx) -> Option<i32> {
    sally_forecast(g, v, grp, c, plot).filter(|&won| won >= g.d.defines.raid_sally_odds)
}

/// Is the stack on `here` the besieger next to its operation's target (Strike, StepIn or March)?
fn besieging(g: &Game, v: &View, grp: GroupId, here: PlotIdx) -> bool {
    v.campaign.op_of(grp).map(|i| &v.campaign.ops[i]).is_some_and(|op| matches!(op.phase, Phase::Strike | Phase::StepIn | Phase::March) && dist(g, here, op.plot) <= 1)
}

/// Would the Raid order on `c` end `c`'s investment? The raiders stop counting as its besiegers
/// (docs/RAIDS.md, section 3; another city they stand next to still counts them), so: `c` is next
/// to the stack and cut by siege now, and would not be invested without the stack's units.
/// (`city_invested_after_move` with the units "moved" onto the city's own plot counts them
/// neither as besiegers, not being adjacent, nor as defenders, not being of its team.)
fn breaks_investment(g: &Game, grp: GroupId, here: PlotIdx, c: CityId) -> bool {
    let cp = g.city_plot(c);
    dist(g, here, cp) == 1 && g.city_supply_cut_by_siege(c) && !g.city_invested_after_move(c, &g.groups[grp].units, cp)
}

/// The gold army `a`'s share of a raid on `c` would bring each turn (overrun).
pub(crate) fn forecast_gold(g: &Game, a: ArmyId, c: CityId) -> i32 {
    g.army_raid_forecast(a, c).shares.iter().find(|s| s.army == a).map_or(0, |s| s.gold)
}

/// A hold point (module comment): the stack takes the Raid order on the city whose countryside it
/// stands on, or keeps the one it holds. True if it raids (its turn is spent: taking the order
/// ends its moves; keeping it holds); false: the caller holds or goes on as before.
pub fn at_hold(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || v.civ_enemies.is_empty() || !alive(g, grp) || !g.units.contains(u) {
        return false;
    }
    let Some(a) = stack_army(g, grp) else { return false };
    if g.army_raid_city(a).is_some() {
        return keep(g, v, grp, u);
    }
    take(g, v, grp, u, a)
}

/// Rule 1 (module comment): take the order here if every condition holds. True if taken.
fn take(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, a: ArmyId) -> bool {
    let p = v.p;
    let here = g.unit_plot(u);
    let Some(c) = g.map.plots[here].working_city.filter(|&c| g.plot_in_countryside(here, c)) else { return false };
    if let Some(why) = g.army_raid_refusal(a, c) {
        crate::trace!(g, p, "raid: stack {:?} on {}'s countryside refused by the rules: {}", grp, city_name(g, c), why);
        return false;
    }
    if besieging(g, v, grp, here) {
        crate::trace!(g, p, "raid: stack {:?} besieges its target, does not raid {}", grp, city_name(g, c));
        return false;
    }
    if breaks_investment(g, grp, here, c) {
        crate::trace!(g, p, "raid: stack {:?} keeps {} invested, does not raid it", grp, city_name(g, c));
        return false;
    }
    let gold = forecast_gold(g, a, c);
    if gold < 1 {
        crate::trace!(g, p, "raid: stack {:?}: {} would yield no gold", grp, city_name(g, c));
        return false;
    }
    if let Some(won) = guard_comes_out(g, v, grp, c, here) {
        crate::trace!(g, p, "raid: stack {:?}: {}'s guard would sally ({} per mille)", grp, city_name(g, c), won);
        return false;
    }
    if g.army_set_station(p, a, Station::Raid(c)).is_err() {
        return false;
    }
    crate::trace!(g, p, "raid: stack {:?} raids {} from {:?} (+{} gold a turn)", grp, city_name(g, c), g.map.xy(here), gold);
    g.tel_decision(p, "military", "raid", "take", gold as i64, String::new);
    true
}

/// Rule 2 (module comment): a stack whose army's Raid order holds keeps it by holding, unless the
/// guard's forecast against its plot is now at or above `RAID_SALLY_ODDS`: then the order ends and
/// the stack decides as any other. True if it holds for the raid.
pub fn keep(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if !still_raiding(g, v, grp, u) {
        return false;
    }
    let p = v.p;
    if let Some(c) = stack_army(g, grp).and_then(|a| g.army_raid_city(a)) {
        crate::trace!(g, p, "raid: stack {:?} keeps raiding {}", grp, city_name(g, c));
    }
    g.tel_decision(p, "military", "raid", "keep", 1, String::new);
    hold(g, grp);
    true
}

/// Whether the stack's army holds a Raid order the guard would not answer: when the guard's
/// forecast against its plot is now at or above `RAID_SALLY_ODDS`, the order ends here (called
/// before the stack stops to heal in place too, which is no move and would keep the order).
pub fn still_raiding(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if v.barbarian || !alive(g, grp) || !g.units.contains(u) {
        return false;
    }
    let p = v.p;
    let Some(a) = stack_army(g, grp) else { return false };
    let Some(c) = g.army_raid_city(a) else { return false };
    let here = g.unit_plot(u);
    if let Some(won) = guard_comes_out(g, v, grp, c, here) {
        crate::trace!(g, p, "raid: stack {:?} stops raiding {}: the guard would sally ({} per mille)", grp, city_name(g, c), won);
        g.tel_decision(p, "military", "raid", "end_sally", won as i64, String::new);
        let _ = g.army_set_station(p, a, Station::None);
        return false;
    }
    true
}

/// A stack next to its operation's target ends a Raid order it holds on that city (there it is
/// the besieger: a raider does not besiege the city it raids).
pub fn stop_raiding(g: &mut Game, v: &View, grp: GroupId, city: CityId) {
    let Some(a) = stack_army(g, grp) else { return };
    if g.army_raid_city(a) == Some(city) {
        crate::trace!(g, v.p, "raid: stack {:?} besieges {} instead of raiding it", grp, city_name(g, city));
        let _ = g.army_set_station(v.p, a, Station::None);
    }
}

/// A raidable city for a stack of ours looking for one (the Raiders' walk, `traits::raid`):
/// a city of a civilization we are at war with, not stripped.
pub(crate) fn raidable(g: &Game, v: &View, c: CityId) -> bool {
    if !g.cities.contains(c) {
        return false;
    }
    let owner = g.cities[c].owner;
    !g.player_is_barbarian(owner) && v.civ_enemies.contains(&g.players[owner].team) && !matches!(g.city_raid_state(c), ffh_core::raid::RaidState::Stripped(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_data::*;

    /// Player 0's city on (3, 5) and player 1's on (12, 5) of the testkit's landmass, at war, no
    /// roads; player 1 owns every plot within 2 of its city (its countryside), the rest is nobody's.
    /// The enemy city has some production and commerce to take.
    fn raid_game() -> (Game, CityId) {
        let mut g = crate::testkit::one_landmass_game(7);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        g.player_init_city(p0, 3, 5, true, true);
        let theirs = g.player_init_city(p1, 12, 5, true, true);
        for q in g.map.plot_ids() {
            if g.map.plots[q].plot_city.is_none() {
                let (x, y) = g.map.xy(q);
                let near = (x - 12).abs() <= 2 && (y - 5).abs() <= 2;
                g.plot_set_owner(q, if near { Some(p1) } else { None }, false, false);
                g.plot_set_route_type(q, None, false);
            }
        }
        for q in g.map.plot_ids() {
            g.plot_update_working_city(q);
        }
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        g.city_change_base_yield_rate(theirs, YieldType::YIELD_PRODUCTION, 10);
        g.city_change_base_yield_rate(theirs, YieldType::YIELD_COMMERCE, 10);
        g.players[p0].turn_active = true;
        (g, theirs)
    }

    /// An army of `n` of player 0's units of `name` on (x, y).
    fn army(g: &mut Game, name: &str, x: i32, y: i32, n: usize) -> (ArmyId, GroupId, UnitId) {
        let t = g.d.t::<UnitType>(name);
        let units: Vec<UnitId> = (0..n).map(|_| g.player_init_unit(PlayerId(0), t, x, y, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let grp = g.units[units[0]].group.unwrap();
        for &o in &units[1..] {
            g.unit_join_group(o, Some(grp), false);
        }
        let a = g.army_form(PlayerId(0), &units).unwrap();
        let grp = g.armies[a].group.unwrap();
        (a, grp, g.group_get_head_unit(grp).unwrap())
    }

    /// Player 1's guard: `n` units of `name` in its city's garrison.
    fn guard(g: &mut Game, name: &str, n: usize) {
        let t = g.d.t::<UnitType>(name);
        for _ in 0..n {
            g.player_init_unit(PlayerId(1), t, 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        g.player_army_sync(PlayerId(1));
    }

    /// Player 0's view with no operation planned (the tests stage the stacks' situation).
    fn view(g: &Game) -> View {
        let mut v = View::new(g, PlayerId(0));
        v.campaign.built = true;
        v
    }

    #[test]
    fn a_holding_stack_on_a_weak_citys_countryside_takes_the_order() {
        let (mut g, theirs) = raid_game();
        guard(&mut g, "UNIT_WARRIOR", 1);
        let (a, grp, u) = army(&mut g, "UNIT_AXEMAN", 10, 5, 3);
        assert!(g.plot_in_countryside(g.unit_plot(u), theirs));
        assert!(forecast_gold(&g, a, theirs) >= 1);
        let mut v = view(&g);
        crate::military::army(&mut g, &mut v, grp, u);
        assert_eq!(g.armies[a].station, Station::Raid(theirs), "the idle stack raids instead of walking home");
        assert_eq!(g.army_raid_city(a), Some(theirs));
        assert_eq!(g.map.xy(g.unit_plot(u)), (10, 5));
    }

    #[test]
    fn a_strong_guard_keeps_the_stack_from_raiding() {
        let (mut g, theirs) = raid_game();
        guard(&mut g, "UNIT_CHAMPION", 4);
        let (a, grp, u) = army(&mut g, "UNIT_WARRIOR", 10, 5, 1);
        let here = g.unit_plot(u);
        let won = g.raid_sally_forecast(theirs, here).expect("the guard can sally");
        assert!(won >= g.d.defines.raid_sally_odds, "the guard is favoured: {won}");
        let mut v = view(&g);
        assert!(!at_hold(&mut g, &mut v, grp, u));
        assert_eq!(g.armies[a].station, Station::None);
    }

    #[test]
    fn no_raid_that_ends_an_investment_or_from_the_besiegers_plot() {
        let (mut g, theirs) = raid_game();
        // Three Warriors next to the (unguarded) city cut it by siege: they are its investment.
        let (a, grp, u) = army(&mut g, "UNIT_WARRIOR", 11, 5, 3);
        assert!(g.city_supply_cut_by_siege(theirs));
        let mut v = view(&g);
        assert!(!at_hold(&mut g, &mut v, grp, u), "the raid would end the investment");
        assert_eq!(g.armies[a].station, Station::None);
        for o in g.groups[grp].units.clone() {
            g.unit_kill(o, false, None);
        }
        // One Warrior next to the city (no investment): it raids, unless the city is its
        // operation's target and it marches on it.
        let (b, solo, w) = army(&mut g, "UNIT_WARRIOR", 13, 6, 1);
        assert!(!g.city_supply_cut_by_siege(theirs));
        let mut v = view(&g);
        let tp = g.city_plot(theirs);
        v.campaign.ops.push(crate::campaign::Op { city: theirs, plot: tp, civ: true, phase: Phase::March, groups: vec![solo], rally: None, assembly: None, staging: None, struck: false, at_war: true, need: 3, have: 1, probe: false });
        assert!(!at_hold(&mut g, &mut v, solo, w), "the besieger does not raid");
        assert_eq!(g.armies[b].station, Station::None);
        let mut v = view(&g);
        assert!(at_hold(&mut g, &mut v, solo, w), "without the operation it raids");
        assert_eq!(g.armies[b].station, Station::Raid(theirs));
    }

    #[test]
    fn a_raiding_stack_keeps_the_order_and_is_not_walked_away() {
        let (mut g, theirs) = raid_game();
        guard(&mut g, "UNIT_WARRIOR", 1);
        let (a, grp, u) = army(&mut g, "UNIT_AXEMAN", 10, 6, 2);
        let mut v = view(&g);
        assert!(at_hold(&mut g, &mut v, grp, u));
        assert_eq!(g.armies[a].station, Station::Raid(theirs));
        // A bigger stack of ours within reach (the gathering would walk the smaller one to it).
        army(&mut g, "UNIT_AXEMAN", 8, 6, 4);
        // The next turn: its units have their moves, the group is awake.
        for o in g.groups[grp].units.clone() {
            g.units[o].moves = 0;
        }
        g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
        let mut v = view(&g);
        crate::military::army(&mut g, &mut v, grp, u);
        assert_eq!(g.army_raid_city(a), Some(theirs), "it raids on");
        assert_eq!(g.map.xy(g.unit_plot(u)), (10, 6), "not walked to the other stack or home");
        // Without the order, the same stack the same turn walks away (gathering or home).
        g.army_set_station(PlayerId(0), a, Station::None).unwrap();
        for o in g.groups[grp].units.clone() {
            g.units[o].moves = 0;
        }
        g.group_clear_mission_queue(grp);
        g.group_set_activity_type(grp, ActivityType::ACTIVITY_AWAKE);
        let mut v = view(&g);
        // (Not the idle raid either: the gathering comes first.)
        crate::military::army(&mut g, &mut v, grp, u);
        assert_ne!(g.map.xy(g.unit_plot(u)), (10, 6), "a stack that does not raid is walked away");
    }

    #[test]
    fn a_raiders_stack_walks_to_a_raidable_countryside() {
        let mut g = crate::testkit::one_landmass_game_with(7, &["CIVILIZATION_DOVIELLO"]);
        let (p0, p1) = (PlayerId(0), PlayerId(1));
        // The Raiders trait for player 0 (whatever its leader).
        let raiders = g.d.t::<TraitType>("TRAIT_RAIDERS");
        if !g.players[p0].traits[raiders] {
            g.player_set_has_trait(p0, raiders, true);
        }
        assert!(crate::traits::has(&g, p0, crate::traits::Trait::Raiders));
        g.player_init_city(p0, 3, 5, true, true);
        let theirs = g.player_init_city(p1, 12, 5, true, true);
        for q in g.map.plot_ids() {
            if g.map.plots[q].plot_city.is_none() {
                let (x, y) = g.map.xy(q);
                let near = (x - 12).abs() <= 2 && (y - 5).abs() <= 2;
                g.plot_set_owner(q, if near { Some(p1) } else { None }, false, false);
                g.plot_set_route_type(q, None, false);
            }
        }
        for q in g.map.plot_ids() {
            g.plot_update_working_city(q);
        }
        let (t0, t1) = (g.players[p0].team, g.players[p1].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
        g.city_change_base_yield_rate(theirs, YieldType::YIELD_PRODUCTION, 10);
        g.city_change_base_yield_rate(theirs, YieldType::YIELD_COMMERCE, 10);
        g.players[p0].turn_active = true;
        let t = g.d.t::<UnitType>("UNIT_AXEMAN");
        let units: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(p0, t, 8, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let grp = g.units[units[0]].group.unwrap();
        g.unit_join_group(units[1], Some(grp), false);
        let a = g.army_form(p0, &units).unwrap();
        let grp = g.armies[a].group.unwrap();
        let u = g.group_get_head_unit(grp).unwrap();
        assert!(!g.plot_in_countryside(g.unit_plot(u), theirs));
        let mut v = view(&g);
        assert!(crate::traits::raid(&mut g, &mut v, grp, u), "the stack sets out");
        let here = g.unit_plot(u);
        let (x, _) = g.map.xy(here);
        assert!(x > 8, "toward the city: now at {:?}", g.map.xy(here));
        let mission_to_countryside = g.groups.get(grp).and_then(|gr| gr.mission_queue.first()).and_then(|m| g.map.plot_xy(m.data1, m.data2)).is_some_and(|q| g.plot_in_countryside(q, theirs));
        assert!(g.plot_in_countryside(here, theirs) || mission_to_countryside, "bound for the countryside");
    }

    /// A farm on the enemy's countryside is no pillage target (fields are raided, not pillaged);
    /// a fort there is, and so is nothing the rules refuse.
    #[test]
    fn pillage_target_names_no_field() {
        let (mut g, theirs) = raid_game();
        let (_, grp, _) = army(&mut g, "UNIT_AXEMAN", 6, 5, 1);
        let (farm, fort) = (g.d.t::<ImprovementType>("IMPROVEMENT_FARM"), g.d.t::<ImprovementType>("IMPROVEMENT_FORT"));
        let (a, b) = (g.map.plot_xy(11, 4).unwrap(), g.map.plot_xy(13, 4).unwrap());
        g.plot_set_improvement_type(a, Some(farm));
        g.plot_set_improvement_type(b, Some(fort));
        assert!(g.plot_is_fields(a) && g.plot_in_countryside(a, theirs));
        assert!(!g.plot_is_fields(b));
        let v = view(&g);
        assert!(!crate::traits::pillage_target(&g, &v, grp, a), "a farm is raided, not pillaged");
        assert!(crate::traits::pillage_target(&g, &v, grp, b), "a fort is pillaged");
    }
}
