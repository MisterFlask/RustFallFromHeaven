//! Battle tactics (docs/TACTICS.md; docs/AI.md, "Battle tactics"): the standing tactic of each of
//! our armies and of each city's garrison, set by a rule from what the body is made of.
//!
//! The rule ([`choose`]) is a pure function of the units' kinds (`Game::unit_engage_kind`, a guard
//! or support by `TacticKind::guard`) and of the lanes of the slots a side usually has
//! (`ENGAGEMENT_SLOTS_BASE`: 8 slots, 4 lanes). Its thresholds come from the lab's matrices
//! (docs/TACTICS.md, section 8):
//!
//! - **Charge** (field armies only): chargers at least [`CHARGE_MIN_PCT`] of the fighting units
//!   and support at most [`CHARGE_MAX_SUPPORT_PCT`]. The mounted army (6 or 10 horse and 2
//!   archers) gained with it (+15.3 in a forest, about even in the open and at walls); every army
//!   with more support lost, worst the ranged army whose archers it left out (-28.1 in a forest).
//! - **Volley**: at least [`VOLLEY_MIN_SUPPORT`] support units, at least [`VOLLEY_MIN_SUPPORT_PCT`]
//!   of the army, and a guard for every lane (its screen). The melee core with 2 archers and 2
//!   mages won with it on every ground (+13.2 open, +8.1 forest, +4.7 walls); the ranged army, whose
//!   3 guards cannot screen 3 or 4 lanes, lost (-28.1 in a forest).
//! - **Line** otherwise. Never Vanguard (section 6). Garrisons: Volley by the same test, else Line
//!   (a charge is a strike in a city).
//!
//! Applied once a turn per player ([`turn`], after its units have moved, so merges and splits of
//! the turn are seen); a tactic is set only when the rule's answer differs from the pending one
//! (`Army::tactic_next`, `City::garrison_tactic_next`), and the rules apply it at the owner's next
//! turn start. Cost: one pass over each army's and garrison's units.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// Charge: chargers at least this share (percent) of the fighting units.
pub const CHARGE_MIN_PCT: i32 = 60;
/// Charge: support at most this share (percent): a Charge fills the slots with chargers first and
/// leaves the support out.
pub const CHARGE_MAX_SUPPORT_PCT: i32 = 25;
/// Volley: at least this many support units (fewer, and the screen protects nothing Line does
/// not).
pub const VOLLEY_MIN_SUPPORT: i32 = 2;
/// Volley: support at least this share (percent) of the fighting units.
pub const VOLLEY_MIN_SUPPORT_PCT: i32 = 25;

/// The tactics the AI uses.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Line,
    Charge,
    Volley,
}

impl Choice {
    pub fn type_name(self) -> &'static str {
        match self {
            Choice::Line => "TACTIC_LINE",
            Choice::Charge => "TACTIC_CHARGE",
            Choice::Volley => "TACTIC_VOLLEY",
        }
    }
}

/// The lanes of a side with `slots` slots (the rules' `max(2, ceil((slots + 2) / 3))`).
pub fn lanes_for(slots: i32) -> i32 {
    ((slots + 4) / 3).max(2)
}

/// The tactic for a body whose units have `kinds` (units that do nothing are left out), fighting
/// with `lanes` lanes; `field`: a field army (else a city's garrison, which never charges).
pub fn choose(kinds: &[TacticKind], lanes: i32, field: bool) -> Choice {
    let fighting = kinds.iter().filter(|&&k| k != TacticKind::None);
    let (mut n, mut chargers, mut support) = (0, 0, 0);
    for &k in fighting {
        n += 1;
        if k == TacticKind::Charge {
            chargers += 1;
        }
        if !k.guard() {
            support += 1;
        }
    }
    if n == 0 {
        return Choice::Line;
    }
    let guards = n - support;
    if field && chargers * 100 >= CHARGE_MIN_PCT * n && support * 100 <= CHARGE_MAX_SUPPORT_PCT * n {
        return Choice::Charge;
    }
    if support >= VOLLEY_MIN_SUPPORT && support * 100 >= VOLLEY_MIN_SUPPORT_PCT * n && guards >= lanes {
        return Choice::Volley;
    }
    Choice::Line
}

/// Unit `u`'s kind for the rule (a striker with Formation is a guard either way, so the
/// anti-flanking distinction is not read).
fn kind(g: &Game, u: UnitId) -> TacticKind {
    TacticKind::of(g.unit_engage_kind(u), false)
}

/// Sets the tactic of each of `p`'s armies and garrisons by [`choose`] (once a turn).
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.player_is_barbarian(p) || g.d.battle_tactics.is_empty() {
        return;
    }
    let d = g.d;
    let ty = |c: Choice| d.ty::<BattleTacticType>(c.type_name());
    let lanes = lanes_for(d.defines.engagement_slots_base);
    let tel = g.telemetry.on();
    for a in g.players[p].armies.clone() {
        if !g.armies.contains(a) {
            continue;
        }
        let kinds: Vec<TacticKind> = g.army_units(a).iter().map(|&u| kind(g, u)).collect();
        if kinds.is_empty() {
            continue;
        }
        let c = choose(&kinds, lanes, true);
        if let Some(t) = ty(c).filter(|&t| g.armies[a].tactic_next != t) {
            let _ = g.army_set_tactic(p, a, t);
        }
        if tel {
            g.tel_decision(p, "tactic_army", d.battle_tactics[g.armies[a].tactic].base.type_.as_str(), "", kinds.len() as i64, String::new);
        }
    }
    for c in g.players[p].cities.clone() {
        if !g.cities.contains(c) {
            continue;
        }
        let kinds: Vec<TacticKind> = g.city_garrison_units(c).iter().map(|&u| kind(g, u)).collect();
        let want = choose(&kinds, lanes, false);
        if let Some(t) = ty(want).filter(|&t| g.cities[c].garrison_tactic_next != t) {
            let _ = g.city_set_garrison_tactic(p, c, t);
        }
        if tel && !kinds.is_empty() {
            g.tel_decision(p, "tactic_garrison", d.battle_tactics[g.cities[c].garrison_tactic].base.type_.as_str(), "", kinds.len() as i64, String::new);
        }
    }
}

/// Telemetry only (reads, changes nothing): an attack `p` is about to make from `from` on `to`,
/// counted by the tactic each side fights in (`Game::engage_side_tactic` of `p`'s units on `from`,
/// and of the units of the defender's owner on `to`), for the measures of docs/TACTICS.md
/// (`tactic_battle`, `attack` for `p`, `defend` for the defender).
pub(crate) fn note_attack(g: &Game, p: PlayerId, from: PlotIdx, to: PlotIdx) {
    if !g.telemetry.on() {
        return;
    }
    let side = |owner: PlayerId, q: PlotIdx| -> Vec<UnitId> { g.map.plots[q].units.iter().copied().filter(|&u| g.units.contains(u) && g.units[u].owner == owner).collect() };
    let name = |t: BattleTacticType| g.d.battle_tactics[t].base.type_.as_str();
    let ours = side(p, from);
    if !ours.is_empty() {
        g.tel_decision(p, "tactic_battle", name(g.engage_side_tactic(&ours, from)), "attack", 1, String::new);
    }
    let team = g.players[p].team;
    let defender = g.map.plots[to].plot_city.filter(|&c| g.cities.contains(c)).map(|c| g.cities[c].owner).or_else(|| g.map.plots[to].units.iter().copied().find(|&u| g.units.contains(u) && g.unit_get_team(u) != team).map(|u| g.units[u].owner));
    if let Some(d) = defender {
        let theirs = side(d, to);
        if !theirs.is_empty() {
            g.tel_decision(d, "tactic_battle", name(g.engage_side_tactic(&theirs, to)), "defend", 1, String::new);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TacticKind as K;

    fn kinds(spec: &[(TacticKind, usize)]) -> Vec<TacticKind> {
        spec.iter().flat_map(|&(k, n)| std::iter::repeat(k).take(n)).collect()
    }

    #[test]
    fn lanes_are_the_rules() {
        assert_eq!(lanes_for(8), 4);
        assert_eq!(lanes_for(6), 3);
        assert_eq!(lanes_for(3), 2);
        assert_eq!(lanes_for(1), 2);
    }

    /// The armies of the lab's matrices (docs/TACTICS.md, sections 7 and 8), on 4 lanes.
    #[test]
    fn the_lab_armies_get_the_tactic_that_won_with_them() {
        // The mounted army: 6 (or 10) horse and 2 archers: Charge.
        assert_eq!(choose(&kinds(&[(K::Charge, 6), (K::Shoot, 2)]), 4, true), Choice::Charge);
        assert_eq!(choose(&kinds(&[(K::Charge, 10), (K::Shoot, 2)]), 4, true), Choice::Charge);
        // The melee core with 2 archers and 2 mages, of 8 and of 12: Volley.
        assert_eq!(choose(&kinds(&[(K::Strike, 4), (K::Shoot, 2), (K::Spell, 2)]), 4, true), Choice::Volley);
        assert_eq!(choose(&kinds(&[(K::Strike, 8), (K::Shoot, 2), (K::Spell, 2)]), 4, true), Choice::Volley);
        // The ranged army with 3 guards: it cannot screen 4 lanes: Line (Volley and Charge lost
        // 28 points with it in a forest).
        assert_eq!(choose(&kinds(&[(K::Strike, 3), (K::Shoot, 3), (K::Spell, 2)]), 4, true), Choice::Line);
        assert_eq!(choose(&kinds(&[(K::Strike, 3), (K::Shoot, 5), (K::Spell, 4)]), 4, true), Choice::Line);
    }

    #[test]
    fn charge_needs_chargers_and_little_support() {
        // 6 of 10 chargers, 2 support (20%): Charge.
        assert_eq!(choose(&kinds(&[(K::Charge, 6), (K::Strike, 2), (K::Shoot, 2)]), 4, true), Choice::Charge);
        // 5 of 10 chargers: not Charge (Volley: 2 support of 10 is under 25%: Line).
        assert_eq!(choose(&kinds(&[(K::Charge, 5), (K::Strike, 3), (K::Shoot, 2)]), 4, true), Choice::Line);
        // 6 chargers and 3 archers of 9 (33% support): the archers would be left out: Volley.
        assert_eq!(choose(&kinds(&[(K::Charge, 6), (K::Shoot, 3)]), 4, true), Choice::Volley);
        // A garrison never charges.
        assert_eq!(choose(&kinds(&[(K::Charge, 4)]), 4, false), Choice::Line);
    }

    #[test]
    fn volley_needs_a_screen_and_support_to_bring_forward() {
        // One archer: nothing for the screen to protect that Line would not.
        assert_eq!(choose(&kinds(&[(K::Strike, 3), (K::Shoot, 1)]), 4, true), Choice::Line);
        // Two archers of 6 with 4 guards: Volley; with 3 guards on 4 lanes: Line; on 3 lanes
        // (a forest's 6 slots): Volley.
        assert_eq!(choose(&kinds(&[(K::Strike, 4), (K::Shoot, 2)]), 4, true), Choice::Volley);
        assert_eq!(choose(&kinds(&[(K::Strike, 3), (K::Shoot, 2)]), 4, true), Choice::Line);
        assert_eq!(choose(&kinds(&[(K::Strike, 3), (K::Shoot, 2)]), 3, true), Choice::Volley);
        // A garrison of archers behind 4 guards: Volley; of archers alone: Line.
        assert_eq!(choose(&kinds(&[(K::AntiFlank, 2), (K::Strike, 2), (K::Shoot, 3)]), 4, false), Choice::Volley);
        assert_eq!(choose(&kinds(&[(K::Shoot, 3)]), 4, false), Choice::Line);
        // Units that do nothing are no fighting units.
        assert_eq!(choose(&kinds(&[(K::None, 5)]), 4, true), Choice::Line);
        assert_eq!(choose(&kinds(&[(K::Charge, 3), (K::None, 4)]), 4, true), Choice::Charge);
    }

    /// The rule sets a field army's pending tactic, and the rules apply it at the owner's turn
    /// start; a garrison of one defender stays in Line.
    #[test]
    fn an_army_of_horsemen_is_set_to_charge() {
        let mut g = crate::testkit::one_landmass_game(4);
        let p0 = PlayerId(0);
        let c = g.player_init_city(p0, 4, 5, true, true);
        let d = g.d;
        let charger = d.units.keys().find(|&t| g.d.units[t].combat > 0 && d.engage_kind(t) == ActionKind::Charge && g.d.units[t].domain_type == Some(DomainType::DOMAIN_LAND)).expect("a charger");
        let units: Vec<UnitId> = (0..3).map(|_| g.player_init_unit(p0, charger, 7, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let a = g.army_form(p0, &units).unwrap();
        let w = d.t::<UnitType>("UNIT_WARRIOR");
        g.player_init_unit(p0, w, 4, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        let line = d.t::<BattleTacticType>("TACTIC_LINE");
        let charge = d.t::<BattleTacticType>("TACTIC_CHARGE");
        turn(&mut g, p0);
        assert_eq!((g.armies[a].tactic, g.armies[a].tactic_next), (line, charge), "pending until the turn start");
        assert_eq!(g.cities[c].garrison_tactic_next, line);
        g.player_apply_tactics(p0);
        assert_eq!(g.armies[a].tactic, charge);
        // Run again: nothing to change.
        turn(&mut g, p0);
        assert_eq!(g.armies[a].tactic_next, charge);
    }
}
