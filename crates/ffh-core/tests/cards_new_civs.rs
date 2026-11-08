//! Batch B4's cards (docs/CARDS.md, section 5): the civilizations born mid-game (the Patrians, the
//! Knights of the Dawn, the Fey Court, the Free Companies, the Free City) and Mirror Colony. Each
//! test builds its situation by hand on a generated duel map and casts the card's world spell
//! through the rules (`unit_can_cast`, `unit_cast`), then checks where the new civilization is
//! born, its cities, units, hero and relation to the caster, and the rules it brings.

use ffh_core::ffh::cards::new_civs;
use ffh_core::ffh::cards::world;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
}

fn world_with(no_vassals: bool) -> World {
    let d = ffh_data::global();
    let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 9);
    setup.options[GameOptionType::GAMEOPTION_NO_VASSAL_STATES] = no_vassals;
    let mut g = Game::new(d, setup);
    g.game_init();
    // No Remnants of Patria but the tests' own.
    let remnants = d.t::<ImprovementType>("IMPROVEMENT_REMNANTS_OF_PATRIA");
    for q in g.map.plot_ids() {
        if g.map.plots[q].improvement_type == Some(remnants) {
            g.plot_set_improvement_type(q, None);
        }
    }
    World { g, p0: PlayerId(0), p1: PlayerId(1) }
}

fn world() -> World {
    world_with(false)
}

impl World {
    /// Player 0's team knows card `card` (TECH_CARD_<card>).
    fn card(&mut self, card: &str) {
        let tech = self.g.d.t::<TechType>(&format!("TECH_CARD_{card}"));
        let team = self.g.players[self.p0].team;
        self.g.team_set_has_tech(team, tech, true, Some(self.p0), false, false);
    }

    /// A unit of player 0's on the map: the caster.
    fn caster(&self) -> UnitId {
        *self.g.players[self.p0].units.iter().find(|&&u| self.g.unit_plot_opt(u).is_some()).expect("a unit of player 0")
    }

    fn spell(&self, name: &str) -> SpellType {
        self.g.d.t::<SpellType>(name)
    }

    /// A city of player 0's on the best free site of the land.
    fn city(&mut self, taken: &[PlotIdx]) -> CityId {
        let land: Vec<PlotIdx> = self.g.map.plot_ids().filter(|&q| !self.g.plot_is_water(q)).collect();
        let site = world::best_sites(&self.g, &land, 1, None, taken)[0];
        let (x, y) = self.g.map.xy(site);
        self.g.player_init_city(self.p0, x, y, true, true)
    }

    /// Casts world spell `name` with the caster; the player it gave birth to.
    fn cast(&mut self, name: &str) -> PlayerId {
        let spell = self.spell(name);
        let u = self.caster();
        let np = world::open_slot(&self.g).expect("a free slot");
        assert!(self.g.unit_can_cast(u, spell, false), "{name} can be cast");
        self.g.unit_cast(u, spell);
        assert!(self.g.players[np].alive, "{name}: the new civilization lives");
        assert!(!self.g.unit_can_cast(u, spell, false), "{name} is cast once");
        np
    }

    fn civ(&self, p: PlayerId) -> &str {
        let c = self.g.player_get_civilization_type(p).unwrap();
        &self.g.d.civilizations[c].base.type_
    }

    fn units_of(&self, p: PlayerId, name: &str) -> Vec<UnitId> {
        let t = self.g.d.t::<UnitType>(name);
        self.g.players[p].units.iter().copied().filter(|&u| self.g.units[u].unit_type == t).collect()
    }

    fn has(&self, u: UnitId, promotion: &str) -> bool {
        self.g.units[u].has_promotion[self.g.d.t::<PromotionType>(promotion)]
    }

    /// City `c` has a building of class `class`.
    fn has_class(&self, c: CityId, class: &str) -> bool {
        let d = self.g.d;
        let class = d.t::<BuildingClassType>(class);
        d.buildings.keys().any(|b| d.buildings[b].building_class_type == Some(class) && self.g.city_num_building(c, b) > 0)
    }

    /// The new civilization's relation to player 0: its vassal, its attitude toward it.
    fn vassal(&self, np: PlayerId) -> bool {
        let (t0, nt) = (self.g.players[self.p0].team, self.g.players[np].team);
        self.g.teams[nt].vassal[t0]
    }

    fn plot_of(&self, c: CityId) -> PlotIdx {
        self.g.city_plot(c)
    }

    fn dist(&self, a: PlotIdx, b: PlotIdx) -> i32 {
        let ((ax, ay), (bx, by)) = (self.g.map.xy(a), self.g.map.xy(b));
        self.g.plot_distance(ax, ay, bx, by)
    }

    /// The new civilization knows player 0's techs, but its cards.
    fn knows_founders_techs(&self, np: PlayerId) {
        let d = self.g.d;
        let (t0, nt) = (self.g.players[self.p0].team, self.g.players[np].team);
        for t in d.techs.keys() {
            if self.g.team_is_has_tech(t0, Some(t)) {
                assert_eq!(self.g.team_is_has_tech(nt, Some(t)), d.techs[t].card.is_none(), "{}", d.techs[t].base.type_);
            }
        }
    }

    fn hero(&self, np: PlayerId, name: &str) -> UnitId {
        let heroes = self.units_of(np, name);
        assert_eq!(heroes.len(), 1, "one {name}");
        assert!(self.has(heroes[0], "PROMOTION_HERO"), "{name} is a hero");
        heroes[0]
    }
}

#[test]
fn the_patrians_wake_by_the_remnants_of_patria() {
    let mut w = world();
    let d = w.g.d;
    w.card("PATRIAN_AWAKENING");
    // The Remnants on an unowned plot with free sites around it.
    let remnants = d.t::<ImprovementType>("IMPROVEMENT_REMNANTS_OF_PATRIA");
    let land: Vec<PlotIdx> = w.g.map.plot_ids().filter(|&q| !w.g.plot_is_water(q) && !w.g.plot_is_impassable(q) && w.g.map.plots[q].units.is_empty()).collect();
    let mut at = None;
    for &q in &land {
        w.g.plot_set_improvement_type(q, Some(remnants));
        if new_civs::patrian_sites(&w.g, w.p0).is_some_and(|s| w.dist(s[0], q) == 1) {
            at = Some(q);
            break;
        }
        w.g.plot_set_improvement_type(q, None);
    }
    let r = at.expect("a plot for the Remnants");
    let np = w.cast("SPELL_PATRIAN_AWAKENING");

    assert_eq!(w.civ(np), "CIVILIZATION_PATRIANS");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    assert_eq!(w.dist(w.plot_of(cities[0]), r), 1, "the first city next to the Remnants");
    assert!(w.dist(w.plot_of(cities[1]), r) <= new_civs::PATRIA_RANGE, "the second within 4 plots");
    assert_eq!(w.g.map.plots[r].improvement_type, Some(remnants), "the Remnants stay");
    for &c in &cities {
        assert!(w.has_class(c, "BUILDINGCLASS_MAGE_GUILD"), "every Patrian city has a Mage Guild");
    }
    let archon = w.hero(np, "UNIT_ARCHON");
    assert_eq!(w.g.unit_plot(archon), w.plot_of(cities[0]), "the hero in the capital");
    let sheet = d.units[w.g.units[archon].unit_type].stats.unwrap();
    assert_eq!(sheet[StatType::Magic.idx()], 80);
    assert!(w.vassal(np), "the caster's vassal");
    assert_eq!(w.g.players[np].ai_attitude_extra[w.p0], new_civs::ATTITUDE);
    assert_eq!(w.g.players[np].parent, Some(w.p0));
    w.knows_founders_techs(np);

    // Its casters start at level 3 with Channeling II: those it starts with, and those it trains.
    for u in w.units_of(np, "UNIT_ADEPT") {
        assert_eq!(w.g.units[u].level, new_civs::PATRIAN_CASTER_LEVEL);
        assert!(w.has(u, "PROMOTION_CHANNELING2"));
    }
    let (x, y) = w.g.map.xy(w.plot_of(cities[1]));
    let trained = w.g.player_init_unit(np, d.t::<UnitType>("UNIT_ADEPT"), x, y, None, None);
    w.g.ev_unit_built(cities[1], trained);
    assert_eq!(w.g.units[trained].level, new_civs::PATRIAN_CASTER_LEVEL);
    assert!(w.g.units[trained].experience >= w.g.py_get_experience_needed(new_civs::PATRIAN_CASTER_LEVEL - 1, np));
    assert!(w.has(trained, "PROMOTION_CHANNELING2"));
    // Not other civilizations' casters.
    let p1_unit = w.g.players[w.p1].units[0];
    let (x, y) = (w.g.units[p1_unit].x, w.g.units[p1_unit].y);
    let other = w.g.player_init_unit(w.p1, d.t::<UnitType>("UNIT_ADEPT"), x, y, None, None);
    new_civs::on_unit_built(&mut w.g, cities[1], other);
    assert_eq!(w.g.units[other].level, 1);
}

#[test]
fn without_the_remnants_the_patrians_take_the_largest_unowned_region() {
    let mut w = world();
    w.card("PATRIAN_AWAKENING");
    let region = world::largest_region(&w.g, |_, _| true).unwrap();
    let np = w.cast("SPELL_PATRIAN_AWAKENING");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for c in cities {
        assert!(region.contains(&w.plot_of(c)));
    }
}

#[test]
fn the_dawn_knights_found_a_fortress_near_the_casters_borders() {
    let mut w = world();
    let d = w.g.d;
    w.city(&[]);
    w.card("DAWN_KNIGHTS");
    let near = world::near_borders(&w.g, w.p0, new_civs::DAWN_RANGE);
    let np = w.cast("SPELL_DAWN_KNIGHTS");

    assert_eq!(w.civ(np), "CIVILIZATION_DAWN_KNIGHTS");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for &c in &cities {
        assert!(near.contains(&w.plot_of(c)), "within 6 plots of the caster's borders");
    }
    assert!(w.has_class(cities[0], "BUILDINGCLASS_WALL_OF_STONE"), "the fortress city's Wall of Stone");
    assert!(w.has_class(cities[0], "BUILDINGCLASS_TEMPLE_OF_THE_ORDER"), "its temple of the Order");
    let order = d.t::<ReligionType>("RELIGION_THE_ORDER");
    assert_eq!(w.g.player_get_state_religion(np), Some(order));
    let knights = w.units_of(np, "UNIT_DAWN_KNIGHT");
    assert_eq!(knights.len(), 4);
    assert_eq!(d.units[d.t::<UnitType>("UNIT_DAWN_KNIGHT")].production_cost * 2, d.units[d.t::<UnitType>("UNIT_PALADIN")].production_cost, "half a Paladin's cost");
    let master = w.hero(np, "UNIT_GRAND_MASTER");
    let (gm, paladin) = (d.units[w.g.units[master].unit_type].stats.unwrap(), d.units[d.t::<UnitType>("UNIT_PALADIN")].stats.unwrap());
    assert!(gm[StatType::Attack.idx()] > paladin[StatType::Attack.idx()] && gm[StatType::Hp.idx()] > paladin[StatType::Hp.idx()]);
    assert!(w.vassal(np));
    assert_eq!(w.g.players[np].alignment_attitude[AlignmentType::ALIGNMENT_GOOD], new_civs::ATTITUDE);

    // The Vow: a win against an evil civilization heals fully; against another, nothing.
    let k = knights[0];
    assert!(w.has(k, "PROMOTION_DAWN_VOW"));
    w.g.players[w.p1].alignment = Some(AlignmentType::ALIGNMENT_NEUTRAL);
    w.g.unit_set_damage(k, 40, None);
    new_civs::dawn_vow_heal(&mut w.g, &[k], &[w.p1]);
    assert_eq!(w.g.units[k].damage, 40);
    w.g.players[w.p1].alignment = Some(AlignmentType::ALIGNMENT_EVIL);
    new_civs::dawn_vow_heal(&mut w.g, &[k], &[w.p1]);
    assert_eq!(w.g.units[k].damage, 0);
}

#[test]
fn the_fey_court_needs_a_forest_and_fights_well_in_it() {
    let mut w = world();
    let d = w.g.d;
    w.card("FEY_COURT");
    // Clear the map's forests, then plant a 4x4 forest on free flat land.
    let forest = d.t::<FeatureType>("FEATURE_FOREST");
    for q in w.g.map.plot_ids() {
        if w.g.map.plots[q].feature_type.is_some() {
            w.g.plot_set_feature_type(q, None);
        }
    }
    let open = |g: &Game, q: PlotIdx| {
        let p = &g.map.plots[q];
        !g.plot_is_water(q) && !g.plot_is_peak(q) && p.owner.is_none() && p.units.is_empty() && p.plot_city.is_none() && p.bonus_type.is_none() && p.improvement_type.is_none()
    };
    let block = w
        .g
        .map
        .plot_ids()
        .find_map(|q| {
            let (x, y) = w.g.map.xy(q);
            let plots: Option<Vec<PlotIdx>> = (0..16).map(|i| w.g.map.plot_xy(x + i % 4, y + i / 4).filter(|&t| open(&w.g, t))).collect();
            plots
        })
        .expect("a 4x4 block of free land");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for &q in &block {
        w.g.map.plots[q].terrain_type = Some(grass);
    }
    assert!(!w.g.unit_can_cast(w.caster(), w.spell("SPELL_FEY_COURT"), false), "no forest, no Fey Court");
    for &q in &block[..11] {
        w.g.plot_set_feature_type(q, Some(forest));
    }
    assert!(new_civs::fey_region(&w.g).is_none(), "11 forest plots are not enough");
    for &q in &block[11..] {
        w.g.plot_set_feature_type(q, Some(forest));
    }
    assert_eq!(new_civs::fey_region(&w.g).map(|r| r.len()), Some(16));
    let np = w.cast("SPELL_FEY_COURT");

    assert_eq!(w.civ(np), "CIVILIZATION_FEY_COURT");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for &c in &cities {
        assert!(block.contains(&w.plot_of(c)), "in the forest region");
    }
    assert_eq!(w.g.player_get_state_religion(np), Some(d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES")));
    for name in ["UNIT_SATYR", "UNIT_FAWN", "UNIT_TREANT"] {
        assert_eq!(w.units_of(np, name).len(), 2, "{name}");
    }
    let erlking = w.hero(np, "UNIT_ERLKING");
    let sheet = d.units[w.g.units[erlking].unit_type].stats.unwrap();
    assert_eq!((sheet[StatType::Hp.idx()], sheet[StatType::Attack.idx()]), (160, 110));
    assert!(d.units[w.g.units[erlking].unit_type].large && d.units[w.g.units[erlking].unit_type].tier == 4);
    assert!(w.vassal(np));

    // The Glamour: +2 first strikes and +50% defence in forest, every land unit (Treants too).
    let satyr = w.units_of(np, "UNIT_SATYR")[0];
    for u in w.g.players[np].units.clone() {
        assert!(w.has(u, "PROMOTION_FEY_GLAMOUR"), "{}", d.units[w.g.units[u].unit_type].base.type_);
    }
    let in_forest = *block.iter().find(|&&q| w.g.map.plots[q].feature_type == Some(forest) && w.g.map.plots[q].plot_city.is_none()).unwrap();
    let base = w.g.unit_first_strikes(satyr);
    let (x, y) = w.g.map.xy(in_forest);
    w.g.unit_set_xy(satyr, x, y, false);
    assert_eq!(w.g.unit_first_strikes(satyr), base + new_civs::GLAMOUR_FIRST_STRIKES);
    assert!(w.g.unit_feature_defense_modifier(satyr, forest) >= 50);
}

#[test]
fn the_free_companies_buy_their_army_at_two_gold_a_hammer() {
    let mut w = world();
    let d = w.g.d;
    w.city(&[]);
    w.card("CONDOTTIERI");
    let near = world::near_borders(&w.g, w.p0, new_civs::CONDOTTIERI_RANGE);
    let np = w.cast("SPELL_CONDOTTIERI");

    assert_eq!(w.civ(np), "CIVILIZATION_FREE_COMPANIES");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for &c in &cities {
        assert!(near.contains(&w.plot_of(c)), "within 8 plots of the caster's borders");
    }
    assert_eq!(w.units_of(np, "UNIT_MERCENARY").len(), 6);
    assert!(w.g.players[np].gold >= new_civs::CONDOTTIERI_GOLD);
    let captain = w.hero(np, "UNIT_CAPTAIN_GENERAL");
    assert_eq!(d.units[w.g.units[captain].unit_type].stats.unwrap()[StatType::Attack.idx()], 130);
    assert!(w.vassal(np));

    // Double gold, and units bought at 2 gold per hammer left under any civic.
    assert!(w.g.players[np].commerce_rate_modifier[CommerceType::COMMERCE_GOLD.idx()] >= 100);
    assert_eq!(w.g.player_unit_rush_gold(np), 2);
    let gold = d.t::<HurryType>("HURRY_GOLD");
    let c = cities[0];
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    w.g.city_push_order(c, OrderType::ORDER_TRAIN, warrior.i(), -1, false, false, false, true);
    assert!(w.g.city_can_hurry(c, gold, false));
    assert_eq!(w.g.city_hurry_gold(c, gold), 2 * w.g.city_production_left(c));
    // Not the caster: it pays the hurry's own price where its civics allow one.
    assert_eq!(w.g.player_unit_rush_gold(w.p0), 0);
}

#[test]
fn a_city_released_by_the_charter_keeps_its_garrison_behind_stronger_walls() {
    let mut w = world();
    let d = w.g.d;
    let capital = w.city(&[]);
    let cap_plot = w.plot_of(capital);
    let town = w.city(&[cap_plot]);
    let plot = w.plot_of(town);
    let (x, y) = w.g.map.xy(plot);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    for _ in 0..2 {
        w.g.player_init_unit(w.p0, warrior, x, y, None, None);
    }
    let granary = w.g.city_building_variant(town, d.t::<BuildingType>("BUILDING_GRANARY"));
    w.g.city_set_num_real_building(town, granary, 1);
    w.card("FREE_CITY_CHARTER");
    let spell = w.spell("SPELL_FREE_CITY_CHARTER");
    let u = w.caster();
    let targets = w.g.unit_spell_targets(u, spell);
    assert!(targets.contains(&SpellTarget::City(town)));
    assert!(!targets.contains(&SpellTarget::City(capital)), "never the capital");
    let np = world::open_slot(&w.g).unwrap();
    assert!(w.g.unit_can_cast_at(u, spell, SpellTarget::City(town)));
    w.g.unit_cast_at(u, spell, SpellTarget::City(town));

    assert_eq!(w.civ(np), "CIVILIZATION_FREE_CITY");
    let free = w.g.map.plots[plot].plot_city.unwrap();
    assert_eq!(w.g.cities[free].owner, np);
    assert_eq!(w.g.players[np].cities.len(), 1);
    assert!(w.has_class(free, "BUILDINGCLASS_GRANARY"), "it keeps its buildings");
    let garrison: Vec<UnitId> = w.g.map.plots[plot].units.iter().copied().filter(|&v| w.g.units[v].owner == np && w.g.units[v].unit_type == warrior).collect();
    assert_eq!(garrison.len(), 2, "it keeps its garrison");
    assert!(w.has_class(free, "BUILDINGCLASS_WALL_OF_STONE"));
    assert_eq!(w.g.city_sentinel_count(free), 3, "three Sentinels instead of two");
    assert_eq!(w.g.city_defender_defense(free), 50);
    assert_eq!(w.g.players[np].ai_attitude_extra[w.p0], new_civs::ATTITUDE);
    assert!(!w.vassal(np), "a Free City is no vassal");
    assert!(w.g.player_get_civilization_description(np).starts_with("Free City of "));
    assert!(w.g.cities.contains(capital) && w.g.cities[capital].owner == w.p0);
}

#[test]
fn mirror_colony_founds_two_cities_joined_by_the_mirror_gate() {
    let mut w = world();
    let d = w.g.d;
    let capital = w.city(&[]);
    w.card("MIRROR_COLONY");
    let before = w.g.players[w.p0].cities.len();
    let region = world::largest_region(&w.g, |_, _| true).unwrap();
    let spell = w.spell("SPELL_MIRROR_COLONY");
    let u = w.caster();
    assert!(w.g.unit_can_cast(u, spell, false));
    w.g.unit_cast(u, spell);

    let cities = w.g.players[w.p0].cities.clone();
    assert_eq!(cities.len(), before + 2, "two cities of the caster's own");
    let colony: Vec<CityId> = cities.iter().copied().filter(|&c| c != capital).collect();
    for &c in &colony {
        assert!(region.contains(&w.plot_of(c)), "on the largest unclaimed region");
    }
    let gate = d.t::<BuildingType>("BUILDING_MIRROR_GATE");
    assert!(w.g.city_num_building(capital, gate) > 0 && w.g.city_num_building(colony[0], gate) > 0);

    // A land unit in the capital steps through the gate, its moves spent.
    let (x, y) = w.g.map.xy(w.plot_of(capital));
    let warrior = w.g.player_init_unit(w.p0, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
    let step = w.spell("SPELL_MIRROR_GATE");
    let to = SpellTarget::City(colony[0]);
    assert_eq!(w.g.unit_spell_targets(warrior, step), vec![to]);
    assert!(w.g.unit_can_cast_at(warrior, step, to));
    w.g.unit_cast_at(warrior, step, to);
    assert_eq!(w.g.unit_plot(warrior), w.plot_of(colony[0]));
    assert!(!w.g.unit_can_move(warrior));
}

#[test]
fn a_birth_needs_a_free_slot_and_without_vassals_signs_a_pact() {
    // No free slot: no birth.
    let mut w = world();
    w.city(&[]);
    w.card("DAWN_KNIGHTS");
    let spell = w.spell("SPELL_DAWN_KNIGHTS");
    let u = w.caster();
    assert!(w.g.unit_can_cast(u, spell, false));
    for p in w.g.player_ids() {
        w.g.players[p].ever_alive = true;
    }
    assert!(world::open_slot(&w.g).is_none());
    assert!(!w.g.unit_can_cast(u, spell, false), "no slot, no card");

    // No Vassal States: open borders and a defensive pact with the founder instead.
    let mut w = world_with(true);
    w.card("PATRIAN_AWAKENING");
    let np = w.cast("SPELL_PATRIAN_AWAKENING");
    let (t0, nt) = (w.g.players[w.p0].team, w.g.players[np].team);
    assert!(!w.vassal(np));
    assert!(w.g.team_is_open_borders(t0, nt));
    assert!(w.g.teams[t0].defensive_pact[nt] && w.g.teams[nt].defensive_pact[t0]);
    // No war is forced on it.
    assert!(w.g.team_ids().all(|t| t == nt || !w.g.teams[nt].at_war[t] || w.g.team_is_barbarian(t)));
}
