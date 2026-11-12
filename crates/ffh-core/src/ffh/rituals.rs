//! Rituals (the owner's decision, 2026-11-12; docs/CARDS.md, "World rituals"; docs/PORTING.md,
//! "Changes from FFH2"): every project is a ritual, and two rules hold for every ritual:
//! - **not delayed without loss**: a ritual with production that is not its city's first build
//!   (taken off the queue, or put behind another order) loses all its production in that city
//!   ([`Game::city_check_rituals`], after every change to the head of a queue); the armies'
//!   requisitions and the empire orders do not go in front of one in progress;
//! - **not hurried with gold**: a hurry with a gold cost (`gold_per_production`) is refused while
//!   the city's first build is a ritual ([`Game::city_can_hurry`]). Hurrying with population and a
//!   great person's hurry are allowed (in FFH2 a project took neither).
//!
//! The world rituals (`world_rituals.rs`) are rituals with rules of their own on top (one per
//! player, doubling cost, charge, news to every player).

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

impl Game {
    /// Is `project` a ritual? Every project is.
    pub fn project_is_ritual(&self, _project: ProjectType) -> bool {
        true
    }

    /// Is `hurry` paid with gold (a gold cost per production)? Such a hurry does not hurry a
    /// ritual.
    pub fn hurry_is_gold(&self, hurry: HurryType) -> bool {
        self.d.hurries[hurry].gold_per_production > 0
    }

    /// The ritual at the head of city `c`'s queue, whatever its progress.
    pub fn city_head_ritual(&self, c: CityId) -> Option<ProjectType> {
        self.city_production_project(c).filter(|&p| self.project_is_ritual(p))
    }

    /// The ritual city `c` is building: at the head of its queue with production put into it.
    /// Nothing goes in front of it without losing that production.
    pub fn city_ritual_in_progress(&self, c: CityId) -> Option<ProjectType> {
        self.city_head_ritual(c).filter(|&p| self.cities[c].project_production[p] > 0)
    }

    /// After a change to city `c`'s queue: a ritual with production that is no longer first in it
    /// (taken off, or put behind another order) loses that production. A world ritual's loss is
    /// news to every player.
    pub fn city_check_rituals(&mut self, c: CityId) {
        let d = self.d;
        let head = self.city_head_ritual(c);
        for project in d.projects.keys() {
            if self.project_is_ritual(project) && self.cities[c].project_production[project] > 0 && head != Some(project) {
                self.cities[c].project_production[project] = 0;
                if d.projects[project].world_ritual {
                    self.world_ritual_abandoned(c, project);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Game;
    use crate::ids::*;
    use crate::setup::GameSetup;
    use ffh_data::*;

    const US: PlayerId = PlayerId(0);

    /// The Hippus (Warcry) with a city of size 8 holding Slavery's and a gold hurry, plenty of gold.
    fn game() -> (Game, CityId) {
        let d = ffh_data::global();
        let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 7);
        for (slot, name) in ["CIVILIZATION_HIPPUS", "CIVILIZATION_BANNOR"].iter().enumerate() {
            let civ = d.t::<CivilizationType>(name);
            setup.players[slot].civ = Some(civ);
            setup.players[slot].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
        }
        let mut g = Game::new(d, setup);
        g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
        g.game_init();
        let (x, y) = g.players[US].units.iter().map(|&u| (g.units[u].x, g.units[u].y)).next().expect("a unit");
        let c = g.player_init_city(US, x, y, true, true);
        g.city_set_population(c, 8);
        g.city_change_extra_happiness(c, 20);
        g.city_clear_order_queue(c);
        for h in ["HURRY_GOLD", "HURRY_POPULATION"] {
            g.players[US].hurry_count[d.t::<HurryType>(h)] += 1;
        }
        g.players[US].gold = 100_000;
        (g, c)
    }

    fn create(g: &mut Game, c: CityId, project: ProjectType) {
        g.city_push_order(c, OrderType::ORDER_CREATE, project.i(), -1, false, false, false, true);
        assert_eq!(g.city_head_ritual(c), Some(project));
    }

    /// For an ordinary ritual (Rites of Oghma) and a world ritual (Warcry): put behind another
    /// order, the production is lost; a gold hurry is refused (shown, disabled); a population
    /// hurry and a great person's hurry are allowed.
    #[test]
    fn every_ritual_loses_its_production_off_the_head_and_takes_no_gold() {
        let (mut g, c) = game();
        let d = g.d;
        let (gold, pop) = (d.t::<HurryType>("HURRY_GOLD"), d.t::<HurryType>("HURRY_POPULATION"));
        assert!(g.hurry_is_gold(gold) && !g.hurry_is_gold(pop));
        let warrior = d.t::<UnitType>("UNIT_WARRIOR");
        for name in ["PROJECT_RITES_OF_OGHMA", "PROJECT_RITUAL_WARCRY"] {
            let project = d.t::<ProjectType>(name);
            assert!(g.project_is_ritual(project));
            g.city_clear_order_queue(c);
            create(&mut g, c, project);
            g.city_add_turn_production(c, 6);
            assert_eq!(g.city_ritual_in_progress(c), Some(project), "{name}");
            // Appended behind it: kept.
            g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, true, false);
            assert_eq!(g.cities[c].project_production[project], 6, "{name}");
            // Put behind a new order: lost.
            g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, false);
            assert_eq!(g.cities[c].project_production[project], 0, "{name}: production lost");
            // Taken off the queue: lost.
            g.city_clear_order_queue(c);
            create(&mut g, c, project);
            g.city_add_turn_production(c, 4);
            g.city_pop_order(c, 0, false, false);
            assert_eq!(g.cities[c].project_production[project], 0, "{name}: production lost");

            // Gold: shown, refused.
            g.city_clear_order_queue(c);
            create(&mut g, c, project);
            assert!(g.city_can_hurry(c, gold, true), "{name}: the gold hurry is shown");
            assert!(!g.city_can_hurry(c, gold, false), "{name}: no gold hurry");
            let before = g.players[US].gold;
            g.city_hurry(c, gold);
            assert_eq!((g.cities[c].project_production[project], g.players[US].gold), (0, before), "{name}");
            // Population: allowed (with a citizen's worth of production left, in a city at peace).
            assert!(!g.city_is_disorder(c));
            let left = g.city_production_left(c);
            g.city_add_turn_production(c, left - 20);
            assert!(g.city_can_hurry(c, pop, false), "{name}: population hurry");
            let size = g.cities[c].population;
            g.city_hurry(c, pop);
            assert!(g.cities[c].population < size, "{name}: citizens spent");
            assert!(g.city_production_left(c) <= 0, "{name}: production added");
        }
        // A unit's hurry (a great person) is production, not gold: allowed on a ritual.
        let project = d.t::<ProjectType>("PROJECT_RITUAL_WARCRY");
        g.city_clear_order_queue(c);
        g.city_set_population(c, 8);
        create(&mut g, c, project);
        let engineer = d.t::<UnitType>("UNIT_ENGINEER");
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let u = g.player_init_unit(US, engineer, x, y, None, None);
        let plot = g.unit_plot(u);
        assert!(g.unit_can_hurry(u, plot, false), "a great person hurries a ritual");
        assert!(g.unit_hurry(u));
        assert!(g.cities[c].project_production[project] > 0);
    }
}
