//! Per-turn analysis for one player: who we are at war with, where enemy units can strike
//! next turn (danger map), and bookkeeping shared by the unit routines of this turn.

use std::collections::BTreeMap;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::reach::Flood;

pub struct View {
    pub p: PlayerId,
    pub team: TeamId,
    pub barbarian: bool,
    /// Teams we are at war with (barbarians included).
    pub enemies: Vec<TeamId>,
    /// Enemies other than the barbarian team.
    pub civ_enemies: Vec<TeamId>,
    /// Per plot: summed attack power of the enemy units that can reach it next turn, fog
    /// included (see [`known_threat`]).
    pub danger: Vec<i32>,
    /// Plots our units already target this turn (city sites, builds), to spread them out.
    pub claimed: Vec<PlotIdx>,
    /// Field army targets near our lands (see `military::find_targets`).
    pub targets: Vec<crate::military::Target>,
    /// Targets already taken by a stack this turn.
    pub target_claims: Vec<(PlotIdx, GroupId)>,
    /// This turn's siege operation (siege-campaigns; built on the first army decision).
    pub campaign: crate::campaign::Plan,
    /// This turn's amphibious invasion target, if any (an overseas enemy city we can take by
    /// shipping an army across water; `naval::amphibious_target`).
    pub amphibious: Option<crate::naval::Amph>,
    /// Our cities whose threat exceeds their garrison (city plot, shortfall).
    pub peril: Vec<(PlotIdx, i32)>,
    /// Roads to build this turn (see `workers::road_plan`).
    pub roads: Vec<(PlotIdx, i32)>,
    /// Plots (cities) where a settler waits for an escort.
    pub escort_requests: Vec<PlotIdx>,
    /// Our units committed to each city's garrison this turn (holding there, or sent there).
    pub garrison: BTreeMap<CityId, i32>,
    /// Of those, the ones sent there that have not arrived yet (on their way: not counted by the
    /// units standing in the city).
    pub garrison_incoming: BTreeMap<CityId, i32>,
    /// Extra defenders our cities want for the rival civilizations near them (`defense.rs`;
    /// cities that want none are absent).
    pub rival_def: BTreeMap<CityId, i32>,
    /// Reusable flood buffers.
    pub flood: Flood,
}

/// Does enemy unit `u` count when player `p` weighs the threats to itself (the danger map, a
/// city's local threat, barbarians or enemies near a city, sorties, peril, the front)? Every
/// unit its invisibility type does not hide from us, in the fog or not: the AI sees through
/// fog of war for its own defense, as the C++ AI did (`CvPlayerAI::AI_getPlotDanger` skips only
/// `isInvisible` units), but not through stealth, which is part of how FfH's hidden units and
/// civilizations play. Attacks see through fog the same way ([`sees_units`], docs/AI.md, "Fog
/// of war"). Switch `FOG_SIGHT` off: only units on plots we see.
pub fn known_threat(g: &Game, p: PlayerId, u: UnitId) -> bool {
    let team = g.players[p].team;
    !g.unit_is_invisible(u, team, false) && (crate::switches::FOG_SIGHT.on(g, p) || g.plot_is_visible(g.unit_plot(u), team))
}

/// Does `team` know which units stand on plot `q` when it plans an attack (a city's garrison,
/// a target to hunt or to walk into)? Visible plots always; with switch `FOG_ATTACK` (the
/// owner's call, 2026-10-24: conflict makes the game) every plot we have revealed, fog
/// included -- the units themselves still pass [`known_threat`]'s stealth test where they are
/// read (`unit_is_invisible`). Unrevealed land stays unknown (docs/AI.md, "Fog of war").
pub fn sees_units(g: &Game, team: TeamId, q: PlotIdx) -> bool {
    g.plot_is_visible(q, team)
        || g.map.plots[q].revealed[team] && g.team_get_leader_id(team).is_some_and(|l| crate::switches::FOG_ATTACK.on(g, l))
}

/// Rough attack power of a unit: strength x health x experience. 0 for non-combatants.
pub fn attack_power(g: &Game, u: UnitId) -> i32 {
    let unit = &g.units[u];
    let base = unit.base_combat + unit.total_damage_type_combat;
    if base <= 0 || unit.only_defensive > 0 {
        return 0;
    }
    scaled_power(g, u, base)
}

/// Rough defense power of a unit (FfH units have separate defense strength).
pub fn defense_power(g: &Game, u: UnitId) -> i32 {
    let unit = &g.units[u];
    let base = unit.base_combat_defense.max(0) + unit.total_damage_type_combat;
    if base <= 0 {
        return 0;
    }
    scaled_power(g, u, base)
}

fn scaled_power(g: &Game, u: UnitId, base: i32) -> i32 {
    let unit = &g.units[u];
    let max_hp = g.d.defines.max_hit_points.max(1);
    let hp = (max_hp - unit.damage).max(0);
    base * 100 * hp / max_hp * (100 + 10 * unit.level.max(0)) / 100
}

/// Static power of a unit type (for production choices).
pub fn type_attack_power(d: &GameData, ut: UnitType) -> i32 {
    let info = &d.units[ut];
    let dmg: i32 = info.damage_type_combat.iter().sum();
    if info.only_defensive {
        return 0;
    }
    (info.combat + dmg) * 100
}

pub fn type_defense_power(d: &GameData, ut: UnitType) -> i32 {
    let info = &d.units[ut];
    let dmg: i32 = info.damage_type_combat.iter().sum();
    (info.combat_defense + dmg) * 100
}

impl View {
    /// The full view: the header and the analysis ([`View::analyze`]).
    pub fn new(g: &Game, p: PlayerId) -> View {
        let mut v = View::header(g, p);
        v.analyze(g);
        v
    }

    /// Only who we are and whom we are at war with (`p`, `team`, `barbarian`, `enemies`,
    /// `civ_enemies`); the analysis fields stay empty (`danger` too: [`View::danger_at`] panics
    /// until [`View::analyze`]). Enough for the empire-level decisions at the start of a turn
    /// (diplomacy, offers), which read nothing else.
    pub fn header(g: &Game, p: PlayerId) -> View {
        let team = g.players[p].team;
        let barbarian = g.player_is_barbarian(p);
        let enemies: Vec<TeamId> = g.teams.keys().filter(|&t| t != team && g.teams[t].alive_count > 0 && g.at_war(team, t)).collect();
        let barb_team = g.players[g.barbarian_player()].team;
        let civ_enemies = enemies.iter().copied().filter(|&t| t != barb_team).collect();
        View {
            p,
            team,
            barbarian,
            enemies,
            civ_enemies,
            danger: Vec::new(),
            claimed: Vec::new(),
            escort_requests: Vec::new(),
            roads: Vec::new(),
            peril: Vec::new(),
            targets: Vec::new(),
            target_claims: Vec::new(),
            campaign: Default::default(),
            amphibious: None,
            garrison: BTreeMap::new(),
            garrison_incoming: BTreeMap::new(),
            rival_def: BTreeMap::new(),
            flood: Flood::default(),
        }
    }

    /// The analysis of the turn: the danger map, and for a civilization the field army
    /// targets, the amphibious target, the road plan, the rival defenders and the cities in peril.
    pub fn analyze(&mut self, g: &Game) {
        let p = self.p;
        let v = self;
        v.danger = vec![0; g.map.num_plots()];
        v.build_danger(g);
        if !v.barbarian {
            v.targets = crate::military::find_targets(g, &*v);
            v.amphibious = crate::naval::amphibious_target(g, p);
            v.roads = crate::workers::road_plan(g, p);
            v.rival_def = crate::defense::all_rival_defenders(g, p);
            for &c in &g.players[p].cities {
                let cp = g.city_plot(c);
                let threat = crate::production::local_threat(g, p, cp, 3);
                let defense = crate::production::garrison(g, p, cp).1 * 3 / 2;
                if threat > defense {
                    v.peril.push((cp, threat - defense));
                }
            }
        }
    }

    fn build_danger(&mut self, g: &Game) {
        for pl in g.players.iter() {
            if !pl.alive || !self.enemies.contains(&pl.team) {
                continue;
            }
            for &u in &pl.units {
                if !g.units.contains(u) {
                    continue;
                }
                let unit = &g.units[u];
                let plot = match g.map.plot_xy(unit.x, unit.y) {
                    Some(p) => p,
                    None => continue,
                };
                if !known_threat(g, self.p, u) {
                    continue;
                }
                let power = attack_power(g, u);
                if power == 0 {
                    continue;
                }
                // City garrisons stay home.
                if g.map.plots[plot].plot_city.is_some() && unit.ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE) {
                    continue;
                }
                // Animals cannot enter owned land.
                let animal = g.d.units[unit.unit_type].animal;
                let range = (g.d.units[unit.unit_type].moves.max(1) + 1).min(4);
                for dx in -range..=range {
                    for dy in -range..=range {
                        if let Some(q) = g.plot_xy(unit.x, unit.y, dx, dy) {
                            if animal && g.map.plots[q].owner.is_some() {
                                continue;
                            }
                            self.danger[q.idx()] += power;
                        }
                    }
                }
            }
        }
    }

    /// At war with another civilization (not just the barbarians).
    pub fn at_war(&self) -> bool {
        !self.civ_enemies.is_empty()
    }

    pub fn danger_at(&self, p: PlotIdx) -> i32 {
        self.danger[p.idx()]
    }

    pub fn is_claimed(&self, p: PlotIdx) -> bool {
        self.claimed.contains(&p)
    }
}
