//! Illians: their ritual projects raise their unique units (the owner's rule: civilizations use
//! their unique mechanics, CLAUDE.md "AI changes"). Wilboman (Hero promotion, an attacker's default
//! AI) is trained as a hero (`production::role_ok`); as an attacker he never won on cost.
//!
//! - **Ritual projects** (`projects.jsonc`, effects in `ffh/events.rs`): The White Hand
//!   (Philosophy, 400 hammers: three Priests of Winter, strength 5 + 2 cold, Medic, Channeling
//!   II, Ice) and Stir from Slumber (Divine Essence, 900 hammers: Drifa, a dragon; the rules
//!   allow it once a player has been eliminated). The generic AI reaches projects only when no
//!   building has any value left, so The White Hand came very late or never, and it never
//!   builds Stir from Slumber. Our most productive city builds them as soon as it can (not while
//!   threatened or broke).
//! - **Frozen Passage** (world spell, replacing FFH2's Stasis: the water within 2 plots of a
//!   target becomes Sea Ice, `ffh_core::ffh::world_ritual_spells::illians`) is cast only where
//!   it is worth [`MIN_WORTH`] ([`frozen_passage_worth`]): a bridge from our land to an enemy's
//!   (or, alone on our land, to a civilization no stronger than us), or the waters an enemy's
//!   cities work, never the waters our own cities work. Without such a target the cast stays
//!   pending, and the ritual is not started until one exists (`world_rituals::civ_gate`). Ad hoc
//!   (a rules change, CLAUDE.md "AI changes").
//!
//! First tried in round 5 (commit 76c1ffd; docs/AI_LOG.md): development seeds 1-40 win 12.5% ->
//! 17.5% (2/0), share at the end +269 (t 1.90); fresh seeds 5001-5064 win 15.6% -> 14.1% (2/3),
//! share at the end -40 (t -0.56): The White Hand came at turns 325-372, when Philosophy came.
//! Back since 2026-10-20 under the owner's rule.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;
use crate::production::Choice;
use crate::view::View;
use crate::Ai;
use ffh_core::ffh::world_ritual_spells::illians as ice;
use ffh_core::structs::SpellTarget;

pub struct Illians;

impl CivPlay for Illians {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }

    fn production(&self, _ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        if threat > 0 {
            return None;
        }
        ritual(g, c)
    }

    fn turn(&self, _ai: &Ai, g: &mut Game, v: &mut View) {
        frozen_passage(g, v.p);
    }
}

/// A bridge's worth ([`frozen_passage_worth`]): it joins a land where we have a city to one
/// holding an enemy's city, or, alone on our land, a civilization no stronger than us. Ad hoc.
pub const BRIDGE: i32 = 100;
/// A bridge that opens our land to other civilizations' cities and is no [`BRIDGE`]. Ad hoc.
pub const OPENS_OUR_LAND: i32 = -50;
/// Each frozen plot an enemy's city could work. Ad hoc.
pub const HARM: i32 = 10;
/// Each frozen plot a city of ours (our team, a vassal or our master) could work. Ad hoc.
pub const OWN_COST: i32 = -30;
/// The least worth a target needs to be cast at (and the ritual to be started). Ad hoc.
pub const MIN_WORTH: i32 = 40;

/// What player `p` knows to score Frozen Passage's targets ([`frozen_passage_worth`]), gathered
/// once per decision.
struct Worth {
    /// The land areas where `p` has a city.
    home: Vec<AreaId>,
    /// Each city of another civilization (another team, not the barbarians) our team has
    /// revealed: its area, whether its team is at war with us or our war preparation's target,
    /// and whether its team's power is not above ours.
    others: Vec<(AreaId, bool, bool)>,
    /// No area of `home` holds a city of another civilization.
    alone: bool,
    /// The plots a revealed city of a team at war with us (not the barbarians) can work.
    enemy_works: Vec<bool>,
    /// The plots a city of our team, of a vassal of ours or of our master can work.
    own_works: Vec<bool>,
}

impl Worth {
    fn new(g: &Game, p: PlayerId) -> Worth {
        let team = g.players[p].team;
        let barbarian = g.barbarian_player();
        let preparing = crate::diplomacy::preparing_target(g, team);
        let our_power = g.team_get_power(team, true);
        let n = g.map.num_plots();
        let mut w = Worth { home: Vec::new(), others: Vec::new(), alone: true, enemy_works: vec![false; n], own_works: vec![false; n] };
        let mark = |mask: &mut Vec<bool>, c: CityId| {
            for i in 0..g.city_num_city_plots(c) as usize {
                if let Some(q) = g.city_index_plot(c, i) {
                    mask[q.idx()] = true;
                }
            }
        };
        for c in g.cities.ids() {
            let o = g.cities[c].owner;
            let at = g.city_plot(c);
            let ot = g.players[o].team;
            if ot == team || g.teams[ot].vassal[team] || g.teams[team].vassal[ot] {
                mark(&mut w.own_works, c);
            }
            if o == p {
                w.home.extend(g.map.plots[at].area);
            }
            if o == barbarian || ot == team || !g.map.plots[at].revealed[team] {
                continue;
            }
            let war = g.teams[team].at_war[ot];
            if war {
                mark(&mut w.enemy_works, c);
            }
            if let Some(a) = g.map.plots[at].area {
                w.others.push((a, war || preparing == Some(ot), g.team_get_power(ot, true) <= our_power));
            }
        }
        w.home.sort();
        w.home.dedup();
        w.alone = !w.others.iter().any(|o| w.home.contains(&o.0));
        w
    }

    /// The worth of Frozen Passage at `t` ([`frozen_passage_worth`]).
    fn of(&self, g: &Game, t: PlotIdx) -> i32 {
        let frozen = ice::frozen_passage_plots(g, t);
        // The land areas next to the plots it freezes (Sea Ice is land already).
        let mut areas: Vec<AreaId> = Vec::new();
        for &q in &frozen {
            for n in ice::plots_within(g, q, 1) {
                if !g.plot_is_water(n) {
                    areas.extend(g.map.plots[n].area);
                }
            }
        }
        areas.sort();
        areas.dedup();
        let far: Vec<AreaId> = areas.iter().copied().filter(|a| !self.home.contains(a)).collect();
        let joins = !far.is_empty() && areas.iter().any(|a| self.home.contains(a));
        let mut worth = 0;
        if joins {
            let beyond = || self.others.iter().filter(|o| far.contains(&o.0));
            if beyond().any(|o| o.1) || (self.alone && beyond().any(|o| o.2)) {
                worth += BRIDGE;
            } else if beyond().next().is_some() {
                worth += OPENS_OUR_LAND;
            }
        }
        let harm = frozen.iter().filter(|q| self.enemy_works[q.idx()]).count() as i32;
        let own = frozen.iter().filter(|q| self.own_works[q.idx()]).count() as i32;
        worth + harm * HARM + own * OWN_COST
    }
}

/// What Frozen Passage at `t` is worth to player `p`, from the water plots it freezes: a bridge
/// (the land areas next to them include one where we have a city and one where we have none)
/// is [`BRIDGE`] when a far area holds a city of a team at war with us or that we prepare a war
/// on, or when we are alone on our land and a far area holds a city of a civilization whose
/// team's power is not above ours; [`OPENS_OUR_LAND`] when it reaches other civilizations' cities
/// otherwise; nothing toward land with no city. Plus [`HARM`] for each frozen plot an enemy's
/// city can work and [`OWN_COST`] for each one a city of ours, a vassal's or our master's can
/// work. Barbarian cities count as nobody's; only cities our team has revealed count.
pub fn frozen_passage_worth(g: &Game, p: PlayerId, t: PlotIdx) -> i32 {
    Worth::new(g, p).of(g, t)
}

/// Frozen Passage's target for player `p` among `targets`: the one of the highest worth
/// ([`frozen_passage_worth`]), if it is at least [`MIN_WORTH`] (ties: the lower plot); else
/// none, and the cast waits.
pub fn frozen_passage_target(g: &Game, p: PlayerId, targets: &[PlotIdx]) -> Option<PlotIdx> {
    if targets.is_empty() {
        return None;
    }
    let w = Worth::new(g, p);
    targets.iter().map(|&t| (w.of(g, t), t)).filter(|&(v, _)| v >= MIN_WORTH).max_by_key(|&(v, t)| (v, std::cmp::Reverse(t))).map(|(_, t)| t)
}

/// Does Frozen Passage have a use for player `p` now: a target of its casters
/// (`frozen_passage_plots_ok`: the water we have revealed within range of our plots or of Sea
/// Ice) worth at least [`MIN_WORTH`]?
pub fn frozen_passage_has_use(g: &Game, p: PlayerId) -> bool {
    frozen_passage_target(g, p, &ice::frozen_passage_plots_ok(g, p)).is_some()
}

/// Casts Frozen Passage when `p`'s cast is pending (module doc).
fn frozen_passage(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_FROZEN_PASSAGE") else { return };
    let Some(u) = g.players[p].units.iter().copied().find(|&u| g.units.contains(u) && g.unit_can_cast_untargeted(u, spell, false)) else {
        return;
    };
    let targets: Vec<PlotIdx> = g.unit_spell_targets(u, spell).into_iter().filter_map(|t| if let SpellTarget::Plot(q) = t { Some(q) } else { None }).collect();
    let Some(t) = frozen_passage_target(g, p, &targets) else { return };
    let (x, y) = g.map.xy(t);
    crate::trace!(g, p, "Frozen Passage at ({x}, {y}): {} plots, worth {}", ice::frozen_passage_plots(g, t).len(), frozen_passage_worth(g, p, t));
    g.tel_cast_reason(u, "world", 0);
    g.unit_cast_at(u, spell, SpellTarget::Plot(t));
}

/// The White Hand and Stir from Slumber in our most productive city as soon as it can build
/// them.
fn ritual(g: &Game, c: CityId) -> Option<Choice> {
    let p = g.cities[c].owner;
    let best = g.players[p].cities.iter().copied().max_by_key(|&o| (g.city_yield_rate(o, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(o)));
    if best != Some(c) || crate::production::financial_trouble(g, p) {
        return None;
    }
    for name in ["PROJECT_THE_WHITE_HAND", "PROJECT_STIR_FROM_SLUMBER"] {
        let Some(pr) = g.d.ty::<ProjectType>(name) else { continue };
        if g.city_can_create(c, pr, false, false) {
            return Some(Choice::Create(pr));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    //! Frozen Passage's targets, the ritual's gate and the Illians' Icebreak, in worlds built by
    //! hand: one landmass (x 2..=17, y 3..=8, `testkit`), cut by a strait where a test needs two.

    use super::*;
    use crate::testkit::one_landmass_game_with;

    const US: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    /// The Illians (player 0) and the Bannor (player 1); columns `strait` turned to coast.
    fn game(strait: Option<(i32, i32)>) -> Game {
        let mut g = one_landmass_game_with(11, &["CIVILIZATION_ILLIANS", "CIVILIZATION_BANNOR"]);
        if let Some((x0, x1)) = strait {
            let coast = g.d.t::<TerrainType>("TERRAIN_COAST");
            for x in x0..=x1 {
                for y in 3..=8 {
                    let q = g.map.plot_xy(x, y).unwrap();
                    g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, false);
                    g.plot_set_terrain_type(q, coast, false);
                }
            }
            g.map_recalculate_areas();
            g.update_plot_groups();
        }
        g
    }

    fn plot(g: &Game, x: i32, y: i32) -> PlotIdx {
        g.map.plot_xy(x, y).expect("on the map")
    }

    fn war(g: &mut Game) {
        let (a, b) = (g.players[US].team, g.players[THEM].team);
        g.team_declare_war(a, b, false, None);
        assert!(g.teams[a].at_war[b]);
    }

    /// The ritual's gate on a turn it looks (`world_rituals::civ_gate`).
    fn gate(g: &mut Game) -> bool {
        let pr = g.player_world_ritual(US).expect("the Illians' ritual");
        assert_eq!(g.d.projects[pr].base.type_, "PROJECT_RITUAL_FROZEN_PASSAGE");
        g.set_game_turn(10);
        crate::world_rituals::civ_gate(g, US, pr)
    }

    fn target(g: &Game) -> Option<PlotIdx> {
        let candidates = ice::frozen_passage_plots_ok(g, US);
        assert!(!candidates.is_empty(), "water in range");
        frozen_passage_target(g, US, &candidates)
    }

    /// The old AI froze the water by its capital when no rival was overseas.
    #[test]
    fn no_rival_overseas_and_no_war_no_target_and_no_ritual() {
        let mut g = game(None);
        g.player_init_city(US, 5, 5, true, true);
        g.player_init_city(THEM, 14, 5, true, true);
        assert_eq!(target(&g), None);
        assert!(!gate(&mut g));
    }

    #[test]
    fn a_strait_to_an_enemy_is_bridged() {
        let mut g = game(Some((9, 11)));
        let ours = g.player_init_city(US, 5, 5, true, true);
        let theirs = g.player_init_city(THEM, 15, 5, true, true);
        let area = |g: &Game, c: CityId| g.map.plots[g.city_plot(c)].area;
        assert_ne!(area(&g, ours), area(&g, theirs));
        g.players[US].power = 100;
        g.players[THEM].power = 1000;
        assert_eq!(target(&g), None, "at peace with a stronger civilization: nothing");
        assert!(!gate(&mut g));
        war(&mut g);
        let t = target(&g).expect("a bridge");
        assert!(frozen_passage_worth(&g, US, t) >= BRIDGE);
        assert!(gate(&mut g), "the ritual has a use");
        g.set_game_turn(11);
        let pr = g.player_world_ritual(US).unwrap();
        assert!(!crate::world_rituals::civ_gate(&g, US, pr), "not looked at on the other turns");
        let frozen = ice::frozen_passage_plots(&g, t);
        g.plot_freeze_sea_ice(&frozen);
        assert_eq!(area(&g, ours), area(&g, theirs), "one land now");
    }

    #[test]
    fn freezing_the_water_our_city_works_costs() {
        let mut g = game(Some((9, 11)));
        // Our city works x 5..=9: (9, 4..=6) are its water.
        let ours = g.player_init_city(US, 7, 5, true, true);
        g.player_init_city(THEM, 15, 5, true, true);
        war(&mut g);
        let by_us = plot(&g, 10, 5);
        let covers = |g: &Game, t: PlotIdx| ice::frozen_passage_plots(g, t).into_iter().filter(|&q| g.city_plot_index(ours, q).is_some()).count();
        assert!(covers(&g, by_us) > 0);
        let near = frozen_passage_worth(&g, US, by_us);
        assert!(near < MIN_WORTH, "a bridge that freezes our water nets {near}");
        let t = target(&g).expect("a bridge away from our water");
        assert_ne!(t, by_us);
        assert_eq!(covers(&g, t), 0);
        assert!(frozen_passage_worth(&g, US, t) > near);
    }

    /// The strait toward the Bannor at peace: `rival_here` puts a Bannor city on our land too.
    fn peace(rival_here: bool, their_power: i32) -> Option<PlotIdx> {
        let mut g = game(Some((9, 11)));
        g.player_init_city(US, 5, 5, true, true);
        g.player_init_city(THEM, 15, 5, true, true);
        if rival_here {
            g.player_init_city(THEM, 2, 8, true, true);
        }
        g.players[US].power = 100;
        g.players[THEM].power = their_power;
        target(&g)
    }

    #[test]
    fn a_bridge_to_a_civilization_at_peace_only_when_alone_and_no_weaker() {
        assert_eq!(peace(true, 1000), None, "stronger, and a rival on our land");
        assert_eq!(peace(true, 10), None, "a rival on our land");
        assert_eq!(peace(false, 1000), None, "stronger");
        assert!(peace(false, 10).is_some(), "alone on our land, toward a weaker civilization");
    }

    /// The Illians keep their Sea Ice; the Bannor in the same place break theirs.
    #[test]
    fn the_illians_break_no_ice() {
        let mut g = game(Some((9, 11)));
        g.player_init_city(US, 5, 5, true, true);
        g.player_init_city(THEM, 15, 5, true, true);
        g.plot_freeze_sea_ice(&[plot(&g, 9, 5), plot(&g, 11, 5)]);
        let galley = g.d.t::<UnitType>("UNIT_GALLEY");
        let ours = g.player_init_unit(US, galley, 9, 4, None, None);
        let theirs = g.player_init_unit(THEM, galley, 11, 4, None, None);
        for p in [US, THEM] {
            g.players[p].turn_active = true;
            crate::sea_ice::end_turn(&mut g, p);
        }
        assert!(g.units[theirs].delayed_spell.is_some(), "the Bannor break it");
        assert!(g.units[ours].delayed_spell.is_none(), "the Illians keep it");
        assert!(g.map.plots[plot(&g, 9, 5)].sea_ice);
    }
}
