//! Research paths (this game's own rules, not FFH2's; `research_paths.jsonc`, docs/PORTING.md,
//! "Research paths"). A player commits to one path at a time; while committed it researches the
//! path's techs `RESEARCH_PATH_PERCENT` percent faster, and its capital holds the path's
//! resource. Once its team knows every tech of the path, the path is complete: the resource
//! stays, and the player may commit to another path.

use crate::game::Game;
use crate::ids::*;
use crate::utils::is_world_unit_class;
use ffh_data::*;

/// Why a player may not commit to a research path (`Game::player_research_path_refusal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchPathRefusal {
    /// Not a living civilization (the barbarians, a dead player).
    NotACiv,
    /// Committed to a path that is not complete yet.
    Committed(ResearchPathType),
    /// Completed earlier.
    Done,
    /// The team knows every tech of the path it can research.
    Known,
    /// None of the path's late techs unlocks a unit this civilization can train.
    Closed,
}

impl Game {
    /// The techs of `path` for player `p`: the path's techs and the tech the player's alignment of
    /// the moment adds (`ResearchPathInfo::alignment_techs`).
    pub fn player_research_path_techs(&self, p: PlayerId, path: ResearchPathType) -> Vec<TechType> {
        let info = &self.d.research_paths[path];
        let mut out = info.techs.clone();
        if let Some(t) = self.players[p].alignment.and_then(|a| info.alignment_techs[a]) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
        out
    }

    /// The research-rate bonus (percent) the research path gives player `p` on `tech`:
    /// `RESEARCH_PATH_PERCENT` when the tech is on the path `p` is committed to, else 0.
    pub fn player_research_path_percent(&self, p: PlayerId, tech: TechType) -> i32 {
        let Some(path) = self.players[p].research_path else {
            return 0;
        };
        let info = &self.d.research_paths[path];
        let on = info.techs.contains(&tech) || self.players[p].alignment.and_then(|a| info.alignment_techs[a]) == Some(tech);
        if on {
            self.d.defines.research_path_percent
        } else {
            0
        }
    }

    /// The research-rate bonus (percent) player `p` has on `tech`: the larger of a unique feature's
    /// (`team_tech_research_bonus_percent`) and its research path's; they do not add up.
    pub fn player_tech_research_bonus_percent(&self, p: PlayerId, tech: TechType) -> i32 {
        self.team_tech_research_bonus_percent(self.players[p].team, tech).max(self.player_research_path_percent(p, tech))
    }

    /// Whether a civilization of alignment `alignment` may commit to `path`: one of the path's late
    /// techs (`capstones`, and the tech its alignment adds) unlocks a unit the civilization can
    /// train -- no world unit (heroes), no unit that needs a religion, none of another alignment
    /// -- or none of the capstones unlocks a unit for any civilization (a path of civics and
    /// buildings).
    pub fn research_path_open_to(&self, civ: CivilizationType, alignment: Option<AlignmentType>, path: ResearchPathType) -> bool {
        let d = self.d;
        let info = &d.research_paths[path];
        let mut capstones = info.capstones.clone();
        if let Some(t) = alignment.and_then(|a| info.alignment_techs[a]) {
            capstones.push(t);
        }
        let unlocked_by = |u: UnitType| {
            let ui = &d.units[u];
            capstones.iter().any(|&c| ui.prereq_and_tech == Some(c) || ui.prereq_and_techs.contains(&c))
        };
        let unit_path = d.units.keys().any(|u| {
            unlocked_by(u) && d.units[u].unit_class_type.map_or(false, |cl| !is_world_unit_class(d, cl))
        });
        if !unit_path {
            return true;
        }
        d.unit_classes.keys().filter(|&cl| !is_world_unit_class(d, cl)).filter_map(|cl| d.civ_unit(civ, cl)).any(|u| {
            let ui = &d.units[u];
            unlocked_by(u)
                && ui.prereq_religion.is_none()
                && ui.state_religion.is_none()
                && ui.prereq_alignment.map_or(true, |a| Some(a) == alignment)
        })
    }

    /// Whether the team of `p` knows every tech of `path` (for `p`'s alignment) that `p` can ever
    /// research: a tech closed to it (one that needs another state religion, a religion tech for
    /// the agnostic) does not hold the path open.
    pub fn player_research_path_known(&self, p: PlayerId, path: ResearchPathType) -> bool {
        let team = self.players[p].team;
        self.player_research_path_techs(p, path).iter().all(|&t| self.team_is_has_tech(team, Some(t)) || !self.player_can_ever_research(p, t))
    }

    /// Why `p` may not commit to `path` now (`None`: it may).
    pub fn player_research_path_refusal(&self, p: PlayerId, path: ResearchPathType) -> Option<ResearchPathRefusal> {
        let pl = &self.players[p];
        let Some(civ) = self.player_get_civilization_type(p).filter(|_| pl.alive && !self.player_is_barbarian(p)) else {
            return Some(ResearchPathRefusal::NotACiv);
        };
        if let Some(current) = pl.research_path {
            return Some(ResearchPathRefusal::Committed(current));
        }
        if pl.research_paths_done.contains(&path) {
            return Some(ResearchPathRefusal::Done);
        }
        if self.player_research_path_known(p, path) {
            return Some(ResearchPathRefusal::Known);
        }
        if !self.research_path_open_to(civ, pl.alignment, path) {
            return Some(ResearchPathRefusal::Closed);
        }
        None
    }

    /// Commits `p` to `path`: its techs are researched faster, and the capital holds the path's
    /// resource from now on. Returns false (and changes nothing) when `p` may not.
    pub fn player_commit_research_path(&mut self, p: PlayerId, path: ResearchPathType) -> bool {
        if self.player_research_path_refusal(p, path).is_some() {
            return false;
        }
        // The capital's plot group recounts its bonuses around the change, as for a resource
        // import (`player_change_bonus_import`).
        let capital = self.player_get_capital_city(p).map(|c| self.city_plot(c));
        if let Some(plot) = capital {
            self.plot_update_plot_group_bonus(plot, false);
        }
        self.players[p].research_path = Some(path);
        if let Some(plot) = capital {
            self.plot_update_plot_group_bonus(plot, true);
        }
        let d = self.d;
        let turn = self.game_turn();
        self.log.msg(turn, Some(p), || format!("Research path: {}", d.text.name(&d.research_paths[path].base.description)));
        self.player_update_research_path(p);
        true
    }

    /// Completes the research path of `p` once its team knows every tech of it: the path joins
    /// `research_paths_done` (its resource stays) and `p` may commit to another.
    pub fn player_update_research_path(&mut self, p: PlayerId) {
        let Some(path) = self.players[p].research_path else {
            return;
        };
        if self.player_research_path_known(p, path) {
            let pl = &mut self.players[p];
            pl.research_path = None;
            pl.research_paths_done.push(path);
            let d = self.d;
            let turn = self.game_turn();
            self.log.msg(turn, Some(p), || format!("Research path complete: {}", d.text.name(&d.research_paths[path].base.description)));
        }
    }

    /// How many of resource `bonus` the research paths of `p` grant at its capital: one for its
    /// current path and one for each completed path whose resource it is.
    pub fn player_research_path_bonus_count(&self, p: PlayerId, bonus: BonusType) -> i32 {
        let pl = &self.players[p];
        pl.research_path.iter().chain(&pl.research_paths_done).filter(|&&path| self.d.research_paths[path].bonus == Some(bonus)).count() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::GameSetup;

    /// A duel whose player 0 (the Amurites of Valledia) has founded its capital.
    fn small_world_game() -> Game {
        let d = ffh_data::global();
        let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
        setup.players[0].civ = Some(d.t::<CivilizationType>("CIVILIZATION_AMURITES"));
        setup.players[0].leader = Some(d.t::<LeaderHeadType>("LEADER_VALLEDIA"));
        let mut g = Game::new(d, setup);
        g.autorun = true;
        g.game_init();
        let p = PlayerId(0);
        let site = g
            .map
            .plot_ids()
            .find(|&q| !g.plot_is_water(q) && !g.plot_is_peak(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].bonus_type.is_none())
            .expect("a land plot");
        let (x, y) = g.map.xy(site);
        g.player_init_city(p, x, y, true, true);
        g
    }

    fn path(g: &Game, name: &str) -> ResearchPathType {
        g.d.t::<ResearchPathType>(name)
    }
    fn tech(g: &Game, name: &str) -> TechType {
        g.d.t::<TechType>(name)
    }

    #[test]
    fn commitment_speeds_up_the_paths_techs_only() {
        let mut g = small_world_game();
        let p = PlayerId(0);
        let arcane = path(&g, "RESEARCHPATH_ARCANE");
        let sorcery = tech(&g, "TECH_SORCERY");
        let bronze = tech(&g, "TECH_BRONZE_WORKING");
        let before = (g.player_calculate_research_modifier(p, Some(sorcery)), g.player_calculate_research_modifier(p, Some(bronze)));
        assert_eq!(g.player_research_path_refusal(p, arcane), None);
        assert!(g.player_commit_research_path(p, arcane));
        assert_eq!(g.players[p].research_path, Some(arcane));
        let pct = g.d.defines.research_path_percent;
        assert_eq!(g.player_calculate_research_modifier(p, Some(sorcery)), before.0 * (100 + pct) / 100);
        assert_eq!(g.player_calculate_research_modifier(p, Some(bronze)), before.1);
        // One path at a time.
        let steel = path(&g, "RESEARCHPATH_STEEL");
        assert_eq!(g.player_research_path_refusal(p, steel), Some(ResearchPathRefusal::Committed(arcane)));
        assert!(!g.player_commit_research_path(p, steel));
    }

    #[test]
    fn a_path_completes_with_its_last_tech_and_frees_the_commitment() {
        let mut g = small_world_game();
        let p = PlayerId(0);
        let team = g.players[p].team;
        let cavalry = path(&g, "RESEARCHPATH_CAVALRY");
        assert!(g.player_commit_research_path(p, cavalry));
        let techs = g.player_research_path_techs(p, cavalry);
        let (last, rest) = techs.split_last().unwrap();
        for &t in rest {
            g.team_set_has_tech(team, t, true, Some(p), false, false);
        }
        assert_eq!(g.players[p].research_path, Some(cavalry));
        g.team_set_has_tech(team, *last, true, Some(p), false, false);
        assert_eq!(g.players[p].research_path, None);
        assert_eq!(g.players[p].research_paths_done, vec![cavalry]);
        // Completed paths are not taken again; another one is.
        assert!(g.player_research_path_known(p, cavalry));
        assert_eq!(g.player_research_path_refusal(p, cavalry), Some(ResearchPathRefusal::Done));
        assert_eq!(g.player_research_path_refusal(p, path(&g, "RESEARCHPATH_ARCANE")), None);
        // The resource stays.
        let horse = g.d.t::<BonusType>("BONUS_HORSE");
        assert_eq!(g.player_research_path_bonus_count(p, horse), 1);
    }

    #[test]
    fn a_tech_the_player_can_never_research_does_not_hold_the_path_open() {
        let mut g = small_world_game();
        let p = PlayerId(0);
        let team = g.players[p].team;
        let shadow = path(&g, "RESEARCHPATH_SHADOW");
        assert!(g.player_commit_research_path(p, shadow));
        // Infernal Pact needs the Ashen Veil as the state religion, Mind Stapling the Octopus
        // Overlords: without them the path completes with the rest.
        for t in g.player_research_path_techs(p, shadow) {
            if g.player_can_ever_research(p, t) {
                g.team_set_has_tech(team, t, true, Some(p), false, false);
            }
        }
        assert!(!g.team_is_has_tech(team, Some(tech(&g, "TECH_INFERNAL_PACT"))));
        assert_eq!(g.players[p].research_paths_done, vec![shadow]);
        assert_eq!(g.players[p].research_path, None);
    }

    #[test]
    fn the_capital_holds_the_paths_resource() {
        let mut g = small_world_game();
        let p = PlayerId(0);
        let capital = g.player_get_capital_city(p).expect("a capital");
        let reagents = g.d.t::<BonusType>("BONUS_REAGENTS");
        let count = g.city_num_bonuses(capital, reagents);
        assert!(g.player_commit_research_path(p, path(&g, "RESEARCHPATH_ARCANE")));
        assert!(g.city_has_bonus(capital, reagents));
        assert_eq!(g.city_num_bonuses(capital, reagents), count + 1);
        // It is not for trade.
        assert_eq!(g.player_get_num_tradeable_bonuses(p, reagents), 0);
    }

    #[test]
    fn a_path_whose_late_units_a_civ_lacks_is_closed_to_it() {
        let g = small_world_game();
        let d = g.d;
        let civ = |n: &str| d.t::<CivilizationType>(n);
        let al = |n: &str| Some(d.t::<AlignmentType>(n));
        let arcane = path(&g, "RESEARCHPATH_ARCANE");
        // No Archmage for the dwarves of Khazad.
        assert!(!g.research_path_open_to(civ("CIVILIZATION_KHAZAD"), al("ALIGNMENT_NEUTRAL"), arcane));
        assert!(g.research_path_open_to(civ("CIVILIZATION_AMURITES"), al("ALIGNMENT_NEUTRAL"), arcane));
        // The Mercantile path's late techs bring civics and ships: open to every civilization.
        let mercantile = path(&g, "RESEARCHPATH_MERCANTILE");
        assert!(d.civilizations.keys().filter(|&c| d.civilizations[c].playable).all(|c| g.research_path_open_to(c, al("ALIGNMENT_NEUTRAL"), mercantile)));
    }

    #[test]
    fn the_faith_path_follows_the_alignment() {
        let mut g = small_world_game();
        let p = PlayerId(0);
        let faith = path(&g, "RESEARCHPATH_FAITH");
        let good = g.d.t::<AlignmentType>("ALIGNMENT_GOOD");
        let evil = g.d.t::<AlignmentType>("ALIGNMENT_EVIL");
        g.players[p].alignment = Some(good);
        assert!(g.player_research_path_techs(p, faith).contains(&tech(&g, "TECH_RIGHTEOUSNESS")));
        g.players[p].alignment = Some(evil);
        let techs = g.player_research_path_techs(p, faith);
        assert!(techs.contains(&tech(&g, "TECH_MALEVOLENT_DESIGNS")) && !techs.contains(&tech(&g, "TECH_RIGHTEOUSNESS")));
    }
}
