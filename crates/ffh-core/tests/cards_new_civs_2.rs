//! Batch B8's cards (docs/CARDS.md, section 5): Rite of the Necropolis, Wake the Hill Clans, the
//! Usurper's Crown, Oath of Fealty and Call the Aifon (world spells), and the Armageddon cards The
//! Scaled Kingdom, The Brood and The Infernal Gate Opens (the Court of Mammon). Each test builds its
//! situation by hand on a generated duel map, casts the world spell through the rules (`unit_can_cast`, `unit_cast`,
//! `unit_cast_at`) or fires the Armageddon card, and checks what it makes and the rules it brings.

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::ffh::cards::new_civs_2 as nc;
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

fn world() -> World {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 9));
    g.game_init();
    World { g, p0: PlayerId(0), p1: PlayerId(1) }
}

impl World {
    /// `p`'s team knows card `card` (TECH_CARD_<card>).
    fn card(&mut self, p: PlayerId, card: &str) {
        let tech = self.g.d.t::<TechType>(&format!("TECH_CARD_{card}"));
        let team = self.g.players[p].team;
        self.g.team_set_has_tech(team, tech, true, Some(p), false, false);
    }

    /// A unit of `p`'s on the map: the caster.
    fn caster(&self, p: PlayerId) -> UnitId {
        *self.g.players[p].units.iter().find(|&&u| self.g.unit_plot_opt(u).is_some()).expect("a unit")
    }

    fn spell(&self, name: &str) -> SpellType {
        self.g.d.t::<SpellType>(name)
    }

    /// A city of `p`'s on the best free unowned site of the land, away from `taken`.
    fn city(&mut self, p: PlayerId, taken: &[PlotIdx]) -> CityId {
        let land: Vec<PlotIdx> = self.g.map.plot_ids().filter(|&q| !self.g.plot_is_water(q)).collect();
        let site = world::best_sites(&self.g, &land, 1, None, taken)[0];
        let (x, y) = self.g.map.xy(site);
        self.g.player_init_city(p, x, y, true, true)
    }

    /// Casts world spell `name` with a unit of `p`'s; the player it gave birth to.
    fn cast(&mut self, p: PlayerId, name: &str) -> PlayerId {
        let spell = self.spell(name);
        let u = self.caster(p);
        let np = world::next_player(&self.g);
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

    fn vassal(&self, p: PlayerId, master: PlayerId) -> bool {
        self.g.teams[self.g.players[p].team].vassal[self.g.players[master].team]
    }

    fn dist(&self, a: PlotIdx, b: PlotIdx) -> i32 {
        let ((ax, ay), (bx, by)) = (self.g.map.xy(a), self.g.map.xy(b));
        self.g.plot_distance(ax, ay, bx, by)
    }

    fn hero(&self, p: PlayerId, name: &str) -> UnitId {
        let heroes = self.units_of(p, name);
        assert_eq!(heroes.len(), 1, "one {name}");
        assert!(self.has(heroes[0], "PROMOTION_HERO"), "{name} is a hero");
        heroes[0]
    }

    fn stat(&self, u: UnitId, s: StatType) -> i32 {
        self.g.d.units[self.g.units[u].unit_type].stats.unwrap()[s.idx()]
    }

    /// A `cols` x `rows` block of open land (unowned, no peak, no unit or city; not in `avoid`),
    /// row by row, made flat and cleared of its resources and improvements.
    fn block(&mut self, cols: i32, rows: i32, avoid: &[PlotIdx]) -> Vec<PlotIdx> {
        let g = &self.g;
        let open = |q: PlotIdx| {
            let p = &g.map.plots[q];
            !g.plot_is_water(q) && !g.plot_is_peak(q) && p.owner.is_none() && p.units.is_empty() && p.plot_city.is_none() && !avoid.contains(&q)
        };
        let block = g
            .map
            .plot_ids()
            .find_map(|q| {
                let (x, y) = g.map.xy(q);
                let plots: Option<Vec<PlotIdx>> = (0..cols * rows).map(|i| g.map.plot_xy(x + i % cols, y + i / cols).filter(|&t| open(t))).collect();
                plots.filter(|p| { let mut s = p.clone(); s.sort(); s.dedup(); s.len() == (cols * rows) as usize })
            })
            .expect("a block of open land");
        for &q in &block {
            self.g.plot_set_bonus_type(q, None);
            self.g.plot_set_improvement_type(q, None);
            if self.g.plot_is_hills(q) {
                self.g.plot_set_plot_type(q, PlotType::PLOT_LAND, true);
            }
        }
        block
    }

    /// `plots` and the plots within 2 of them: a block found away from them touches none.
    fn margin(&self, plots: &[PlotIdx]) -> Vec<PlotIdx> {
        let mut out = Vec::new();
        for &q in plots {
            let (x, y) = self.g.map.xy(q);
            out.extend((0..25).filter_map(|i| self.g.map.plot_xy(x + i % 5 - 2, y + i / 5 - 2)));
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Rite of the Necropolis.

/// Player 0's capital and four City Ruins on free sites; the Necropolis cast on them.
fn necropolis(w: &mut World) -> (PlayerId, Vec<PlotIdx>) {
    let d = w.g.d;
    let capital = w.city(w.p0, &[]);
    let cap = w.g.city_plot(capital);
    let land: Vec<PlotIdx> = w.g.map.plot_ids().filter(|&q| !w.g.plot_is_water(q)).collect();
    let sites = world::best_sites(&w.g, &land, 4, None, &[cap]);
    assert_eq!(sites.len(), 4);
    let ruins = d.t::<ImprovementType>("IMPROVEMENT_CITY_RUINS");
    w.card(w.p0, "RITE_OF_THE_NECROPOLIS");
    let spell = w.spell("SPELL_RITE_OF_THE_NECROPOLIS");
    for &q in &sites[..2] {
        w.g.plot_set_improvement_type(q, Some(ruins));
    }
    assert!(!w.g.unit_can_cast(w.caster(w.p0), spell, false), "two ruins are not enough");
    for &q in &sites[2..] {
        w.g.plot_set_improvement_type(q, Some(ruins));
    }
    let mut nearest = sites.clone();
    nearest.sort_by_key(|&q| (w.dist(q, cap), q));
    let named = nc::necropolis_sites(&w.g, w.p0).expect("three ruins");
    assert_eq!(named, nearest[..3], "the three ruins nearest the capital");
    (w.cast(w.p0, "SPELL_RITE_OF_THE_NECROPOLIS"), named)
}

#[test]
fn the_necropolis_rises_on_the_ruins_nearest_the_capital() {
    let mut w = world();
    let d = w.g.d;
    let (np, ruins) = necropolis(&mut w);
    assert_eq!(w.civ(np), "CIVILIZATION_NECROPOLIS");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 3);
    for &c in &cities {
        let plot = w.g.city_plot(c);
        assert!(ruins.contains(&plot), "on a ruin");
        assert_eq!(w.g.map.plots[plot].improvement_type, None, "the ruins are built over");
        assert_eq!(w.g.cities[c].population, nc::NECROPOLIS_POPULATION);
        assert!(w.has_class(c, "BUILDINGCLASS_WALLS"));
    }
    assert_eq!(w.units_of(np, "UNIT_PYRE_ZOMBIE").len(), 6, "two Pyre Zombies a city");
    let lich = w.hero(np, "UNIT_LICH_PRINCE");
    assert!(w.has(lich, "PROMOTION_IMMORTAL") && w.has(lich, "PROMOTION_UNDEAD"));
    assert_eq!(w.stat(lich, StatType::Magic), 70);
    assert_eq!(d.units[w.g.units[lich].unit_type].magic.spell, Some(d.t::<SpellType>("SPELL_DEATH_GRIP")), "Death II's battle spell");
    assert_eq!(w.g.unit_plot(lich), w.g.city_plot(cities[0]), "the hero in the capital");
    assert!(w.g.players[np].ignore_food, "Fallow: its cities need no food");
    assert!(!w.vassal(np, w.p0), "no vassal");
    assert_eq!(w.g.players[np].parent, Some(w.p0));
    assert_eq!(w.g.players[np].ai_attitude_extra[w.p0], nc::ATTITUDE);
    assert_eq!(w.g.players[np].alignment_attitude[AlignmentType::ALIGNMENT_GOOD], nc::NECROPOLIS_GOOD_ATTITUDE);
    // Evil or neutral: good civilizations do not draw it.
    let card = d.t::<TechType>("TECH_CARD_RITE_OF_THE_NECROPOLIS");
    assert_eq!(d.techs[card].card.as_ref().unwrap().block_alignment, Some(AlignmentType::ALIGNMENT_GOOD));
    let prophet = d.t::<UnitClassType>("UNITCLASS_PROPHET");
    w.g.team_set_has_tech(w.g.players[w.p0].team, card, false, Some(w.p0), false, false);
    // The deck deals its lowest band first: take the Prophet cards of lower bands out of it.
    let band = d.techs[card].card.as_ref().unwrap().band;
    let lower: Vec<TechType> =
        d.techs.keys().filter(|&t| d.techs[t].card.as_ref().is_some_and(|c| c.suit == prophet && c.band < band)).collect();
    for t in lower {
        w.g.team_set_has_tech(w.g.players[w.p0].team, t, true, Some(w.p0), false, false);
    }
    for (alignment, drawn) in [(AlignmentType::ALIGNMENT_GOOD, false), (AlignmentType::ALIGNMENT_NEUTRAL, true), (AlignmentType::ALIGNMENT_EVIL, true)] {
        w.g.players[w.p1].alignment = Some(alignment);
        assert_eq!(w.g.deck_candidates(w.p1, prophet).contains(&card), drawn, "{alignment:?}");
    }
}

#[test]
fn the_dead_of_a_battle_on_the_necropoliss_land_rise_as_its_zombies() {
    let mut w = world();
    let d = w.g.d;
    let (np, _) = necropolis(&mut w);
    let c = w.g.players[np].cities[0];
    let city = w.g.city_plot(c);
    let (cx, cy) = (w.g.cities[c].x, w.g.cities[c].y);
    let field = (0..9)
        .filter_map(|i| w.g.map.plot_xy(cx + i % 3 - 1, cy + i / 3 - 1))
        .find(|&q| q != city && w.g.map.plots[q].owner == Some(np) && !w.g.plot_is_water(q) && !w.g.plot_is_impassable(q) && w.g.map.plots[q].units.is_empty())
        .expect("a plot of the Necropolis's beside its capital");
    let (nt, t1) = (w.g.players[np].team, w.g.players[w.p1].team);
    w.g.team_meet(nt, t1, false);
    w.g.team_declare_war(t1, nt, false, None);
    let (fx, fy) = w.g.map.xy(field);
    let warrior = w.g.player_init_unit(w.p1, d.t::<UnitType>("UNIT_WARRIOR"), fx, fy, None, None);
    let zombie = w.units_of(np, "UNIT_PYRE_ZOMBIE")[0];
    assert_eq!(nc::necropolis_risers(&w.g, field, [warrior].into_iter()), Some((np, 1)), "a living unit rises");
    assert_eq!(nc::necropolis_risers(&w.g, field, [zombie].into_iter()), None, "the undead do not");
    let away = w.g.map.plot_ids().find(|&q| w.g.map.plots[q].owner.is_none() && !w.g.plot_is_water(q)).unwrap();
    assert_eq!(nc::necropolis_risers(&w.g, away, [warrior].into_iter()), None, "nor anyone off its land");
    // Where an enemy stands they rise in its nearest city, their moves spent.
    let before = w.units_of(np, "UNIT_PYRE_ZOMBIE").len();
    nc::necropolis_rise(&mut w.g, np, field, 2);
    let risen: Vec<UnitId> = w.units_of(np, "UNIT_PYRE_ZOMBIE").into_iter().filter(|&u| w.g.unit_plot(u) == city).collect();
    assert_eq!(w.units_of(np, "UNIT_PYRE_ZOMBIE").len(), before + 2);
    assert!(risen.len() >= 2 && risen.iter().filter(|&&u| !w.g.unit_can_move(u)).count() >= 2, "in the city, without moves");

    // A battle on its land: three Hill Giants of the Necropolis's attack the nearly dead Warrior.
    w.g.unit_set_damage(warrior, d.defines.max_hit_points - 1, None);
    let giant = d.t::<UnitType>("UNIT_CLAN_GIANT");
    let giants: Vec<UnitId> = (0..3).map(|_| w.g.player_init_unit(np, giant, cx, cy, None, None)).collect();
    let grp = w.g.units[giants[0]].group.unwrap();
    for &u in &giants[1..] {
        w.g.unit_join_group(u, Some(grp), true);
    }
    let on_field = |w: &World| w.units_of(np, "UNIT_PYRE_ZOMBIE").into_iter().filter(|&u| w.g.unit_plot(u) == field).count();
    let before = on_field(&w);
    w.g.group_attack(grp, fx, fy, MOVE_DIRECT_ATTACK);
    if !w.g.units.contains(warrior) || w.g.units[warrior].death_delay {
        assert_eq!(on_field(&w), before + 1, "the Warrior rises on the field its killers hold");
    }
}

// ---------------------------------------------------------------------------------------------
// Wake the Hill Clans.

#[test]
fn the_hill_clans_wake_on_hills_near_the_capital_and_their_hills_give_production() {
    let mut w = world();
    let d = w.g.d;
    w.city(w.p0, &[]);
    for q in w.g.map.plot_ids() {
        if w.g.plot_is_hills(q) {
            w.g.plot_set_plot_type(q, PlotType::PLOT_LAND, true);
        }
    }
    w.card(w.p0, "WAKE_THE_HILL_CLANS");
    let spell = w.spell("SPELL_WAKE_THE_HILL_CLANS");
    assert!(!w.g.unit_can_cast(w.caster(w.p0), spell, false), "no hills, no giants");
    let block = w.block(5, 5, &[]);
    for &q in &block[..5] {
        w.g.plot_set_plot_type(q, PlotType::PLOT_HILLS, true);
    }
    assert!(nc::hill_clan_sites(&w.g, w.p0).is_none(), "five hills are not enough");
    for &q in &block[5..] {
        w.g.plot_set_plot_type(q, PlotType::PLOT_HILLS, true);
    }
    let sites = nc::hill_clan_sites(&w.g, w.p0).expect("the clans' hills");
    let hill = *block.iter().find(|&&q| w.dist(sites[0], q) == 1 && !sites.contains(&q)).expect("a hill next to the first city");
    let production = |w: &World| w.g.plot_calculate_yield(hill, YieldType::YIELD_PRODUCTION);
    let before = production(&w);
    let np = w.cast(w.p0, "SPELL_WAKE_THE_HILL_CLANS");

    assert_eq!(w.civ(np), "CIVILIZATION_HILL_CLANS");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for &c in &cities {
        let plot = w.g.city_plot(c);
        assert!(block.contains(&plot), "on the clans' hills");
        assert!(w.has_class(c, "BUILDINGCLASS_PALISADE"));
    }
    assert_eq!(w.units_of(np, "UNIT_CLAN_GIANT").len(), 4);
    let thrumm = w.hero(np, "UNIT_THRUMM");
    assert!(d.units[w.g.units[thrumm].unit_type].large);
    assert_eq!((w.stat(thrumm, StatType::Hp), w.stat(thrumm, StatType::Attack), w.stat(thrumm, StatType::Defense)), (200, 100, 100));
    assert!(w.vassal(np, w.p0), "the caster's vassal");
    // +1 production on its hills.
    assert_eq!(w.g.map.plots[hill].owner, Some(np));
    assert_eq!(production(&w), before + 1);
    // It alone trains Hill Giants, at 90 hammers.
    let giant = d.t::<UnitType>("UNIT_CLAN_GIANT");
    assert_eq!(d.units[giant].production_cost, 90);
    assert!(w.g.player_can_train(np, giant, false, false, false));
    let class = d.t::<UnitClassType>("UNITCLASS_HILL_GIANT");
    let ours = d.civ_unit(w.g.player_get_civilization_type(w.p0).unwrap(), class).unwrap();
    assert!(!w.g.player_can_train(w.p0, ours, false, false, false), "the wild Hill Giant is nobody's to train");
}

// ---------------------------------------------------------------------------------------------
// The Usurper's Crown.

#[test]
fn the_usurpers_crown_splits_the_farthest_third_of_a_large_civilization() {
    let mut w = world();
    let d = w.g.d;
    let mut taken = Vec::new();
    for _ in 0..7 {
        let c = w.city(w.p1, &taken);
        taken.push(w.g.city_plot(c));
    }
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_meet(t0, t1, false);
    w.card(w.p0, "USURPERS_CROWN");
    let spell = w.spell("SPELL_USURPERS_CROWN");
    let u = w.caster(w.p0);
    assert!(w.g.unit_spell_targets(u, spell).is_empty(), "seven cities are too few");
    for _ in 0..2 {
        let c = w.city(w.p1, &taken);
        taken.push(w.g.city_plot(c));
    }
    assert_eq!(w.g.players[w.p1].cities.len(), 9);
    let target = SpellTarget::Player(w.p1);
    assert_eq!(w.g.unit_spell_targets(u, spell), vec![target]);
    let seceding = nc::usurper_cities(&w.g, w.p1);
    assert_eq!(seceding.len(), 3, "a third of nine");
    let capital = w.g.player_get_capital_city(w.p1).unwrap();
    let cap = w.g.city_plot(capital);
    let plots: Vec<PlotIdx> = seceding.iter().map(|&c| w.g.city_plot(c)).collect();
    let kept: Vec<PlotIdx> = w.g.players[w.p1].cities.iter().map(|&c| w.g.city_plot(c)).filter(|q| !plots.contains(q)).collect();
    assert!(plots.iter().all(|&s| kept.iter().all(|&k| w.dist(s, cap) >= w.dist(k, cap))), "the farthest from the capital");
    // A garrison in one of them.
    let (gx, gy) = w.g.map.xy(plots[1]);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    w.g.player_init_unit(w.p1, warrior, gx, gy, None, None);
    let hero_unit = nc::usurper_unit(&w.g, w.p1).expect("a melee unit");
    assert_eq!(d.units[hero_unit].unit_combat_type, d.ty::<UnitCombatType>("UNITCOMBAT_MELEE"));
    let np = world::next_player(&w.g);
    assert!(w.g.unit_can_cast_at(u, spell, target));
    w.g.unit_cast_at(u, spell, target);

    assert!(w.g.players[np].alive);
    assert_eq!(w.g.player_get_civilization_type(np), w.g.player_get_civilization_type(w.p1), "of its parent's civilization");
    assert_eq!(w.g.player_get_leader_type(np), Some(d.t::<LeaderHeadType>("LEADER_USURPER")));
    assert!(w.g.player_get_civilization_description(np).ends_with("Usurpers"));
    for &q in &plots {
        assert_eq!(w.g.map.plots[q].plot_city.map(|c| w.g.cities[c].owner), Some(np), "the far cities secede");
    }
    assert_eq!(w.g.players[w.p1].cities.len(), 6);
    assert!(w.g.map.plots[plots[1]].units.iter().any(|&v| w.g.units[v].owner == np && w.g.units[v].unit_type == warrior), "with their garrisons");
    let usurper: Vec<UnitId> = w.g.players[np].units.iter().copied().filter(|&v| w.has(v, "PROMOTION_HERO")).collect();
    assert_eq!(usurper.len(), 1, "the Usurper");
    assert_eq!(w.g.units[usurper[0]].unit_type, hero_unit);
    assert_eq!(w.g.players[np].parent, Some(w.p1));
    assert_eq!(w.g.players[np].alignment, w.g.players[w.p1].alignment);
    assert_eq!(w.g.players[np].ai_attitude_extra[w.p0], nc::ATTITUDE);
    assert_eq!(w.g.players[np].ai_attitude_extra[w.p1], nc::USURPER_GRUDGE);
    assert!(!w.vassal(np, w.p0) && !w.vassal(np, w.p1));
    let nt = w.g.players[np].team;
    assert!(!w.g.teams[nt].at_war[t1] && !w.g.teams[nt].at_war[t0], "no war is forced");
}

// ---------------------------------------------------------------------------------------------
// Oath of Fealty.

#[test]
fn the_oath_of_fealty_makes_a_weak_enemy_a_vassal() {
    let mut w = world();
    let c0 = w.city(w.p0, &[]);
    let taken = [w.g.city_plot(c0)];
    w.city(w.p1, &taken);
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    w.g.team_meet(t0, t1, false);
    w.card(w.p0, "OATH_OF_FEALTY");
    let spell = w.spell("SPELL_OATH_OF_FEALTY");
    let u = w.caster(w.p0);
    w.g.players[w.p0].power = 300;
    w.g.players[w.p1].power = 100;
    assert!(w.g.unit_spell_targets(u, spell).is_empty(), "only at war");
    w.g.team_declare_war(t0, t1, false, None);
    w.g.players[w.p1].power = 101;
    assert!(w.g.unit_spell_targets(u, spell).is_empty(), "a third of our power or less");
    w.g.players[w.p1].power = 100;
    let target = SpellTarget::Player(w.p1);
    assert_eq!(w.g.unit_spell_targets(u, spell), vec![target]);
    w.g.setup.options[GameOptionType::GAMEOPTION_NO_VASSAL_STATES] = true;
    assert!(w.g.unit_spell_targets(u, spell).is_empty(), "not with No Vassal States");
    w.g.setup.options[GameOptionType::GAMEOPTION_NO_VASSAL_STATES] = false;
    assert!(w.g.unit_can_cast_at(u, spell, target));
    w.g.unit_cast_at(u, spell, target);
    assert!(w.g.teams[t1].vassal[t0] && w.g.teams[t1].capitulated, "it capitulates");
    assert!(!w.g.teams[t0].at_war[t1] && !w.g.teams[t1].at_war[t0], "the war ends");
    assert!(!w.g.unit_can_cast_at(u, spell, target), "once");
}

// ---------------------------------------------------------------------------------------------
// Call the Aifon.

#[test]
fn the_aifon_found_two_coastal_cities_and_walk_on_water() {
    let mut w = world();
    let d = w.g.d;
    w.card(w.p0, "CALL_THE_AIFON");
    // A sea of their own (the duel map is nearly all land): a 6 x 6 ocean.
    let sea = w.block(6, 6, &[]);
    for &q in &sea {
        w.g.plot_set_plot_type(q, PlotType::PLOT_OCEAN, true);
    }
    let ocean = d.defines.min_water_size_for_ocean;
    let np = w.cast(w.p0, "SPELL_CALL_THE_AIFON");
    assert_eq!(w.civ(np), "CIVILIZATION_AIFON");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 2);
    for &c in &cities {
        assert!(w.g.plot_is_coastal_land(w.g.city_plot(c), ocean), "on the coast of an ocean");
    }
    assert_eq!(w.units_of(np, "UNIT_AXEMAN").len(), 4);
    assert_eq!(w.units_of(np, "UNIT_ARCHER").len(), 2);
    let mother = w.hero(np, "UNIT_DEEP_MOTHER");
    assert_eq!(w.g.unit_get_domain_type(mother), DomainType::DOMAIN_SEA, "a Kraken");
    assert!(w.vassal(np, w.p0));
    // Its land units walk on water.
    for u in w.g.players[np].units.clone() {
        if w.g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND {
            assert!(w.has(u, "PROMOTION_WATER_WALKING"));
        }
    }
    let axe = w.units_of(np, "UNIT_AXEMAN")[0];
    let (ax, ay) = (w.g.units[axe].x, w.g.units[axe].y);
    let water = (0..9).filter_map(|i| w.g.map.plot_xy(ax + i % 3 - 1, ay + i / 3 - 1)).find(|&q| w.g.plot_is_water(q)).expect("water beside the coastal city");
    assert!(w.g.unit_can_move_into(axe, water, false, false, false), "an Axeman steps onto the water");
}

// ---------------------------------------------------------------------------------------------
// A04 The Scaled Kingdom.

#[test]
fn the_scaled_kingdom_rises_in_the_largest_jungle_or_marsh() {
    let mut w = world();
    let d = w.g.d;
    let capital = w.city(w.p0, &[]);
    let (marsh, grass) = (d.t::<TerrainType>("TERRAIN_MARSH"), d.t::<TerrainType>("TERRAIN_GRASS"));
    let shallows = d.t::<TerrainType>("TERRAIN_SHALLOWS");
    let jungle = d.t::<FeatureType>("FEATURE_JUNGLE");
    for q in w.g.map.plot_ids() {
        if w.g.map.plots[q].feature_type == Some(jungle) {
            w.g.plot_set_feature_type(q, None);
        }
        if w.g.map.plots[q].terrain_type.is_some_and(|t| t == marsh || t == shallows) {
            w.g.plot_set_terrain_type(q, grass, true);
        }
    }
    assert!(!nc::valid_scaled_kingdom(&w.g), "no swamp, no kingdom");
    let region = w.block(7, 5, &w.margin(&[w.g.city_plot(capital)]));
    for &q in &region {
        w.g.plot_set_feature_type(q, None);
        w.g.plot_set_terrain_type(q, marsh, true);
    }
    assert!(nc::valid_scaled_kingdom(&w.g));
    let np = world::next_player(&w.g);
    nc::fire_scaled_kingdom(&mut w.g, 0);

    assert!(w.g.players[np].alive);
    assert_eq!(w.civ(np), "CIVILIZATION_SCALED_KINGDOM");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), nc::SCALED_CITIES);
    for &c in &cities {
        assert!(region.contains(&w.g.city_plot(c)), "in the marsh");
        assert!(w.has_class(c, "BUILDINGCLASS_PALISADE"));
    }
    assert_eq!(w.units_of(np, "UNIT_LIZARDMAN_RANGER").len(), 6);
    let king = w.hero(np, "UNIT_SLITHERKING");
    assert_eq!(w.stat(king, StatType::Attack), 90);
    for u in w.g.players[np].units.clone() {
        assert!(w.has(u, "PROMOTION_SCALED_NATIVE"), "jungle and marsh natives");
        assert!(w.g.unit_is_terrain_double_move(u, marsh) && w.g.unit_is_feature_double_move(u, jungle));
        assert!(w.g.unit_terrain_defense_modifier(u, marsh) >= 50 && w.g.unit_feature_defense_modifier(u, jungle) >= 50);
    }
    // No founder: nobody's vassal or child; it knows what its Rangers need.
    assert_eq!(w.g.players[np].parent, None);
    assert!(w.g.player_ids().all(|p| p == np || !w.vassal(np, p)));
    let nt = w.g.players[np].team;
    assert!(w.g.team_is_has_tech(nt, Some(d.t::<TechType>("TECH_ANIMAL_HANDLING"))));
    // -2 toward its neighbours.
    for p in [w.p0, w.p1] {
        let near = cities.iter().any(|&c| world::within_range_of_borders(&w.g, p, w.g.city_plot(c), nc::SCALED_NEIGHBOUR_RANGE));
        assert_eq!(w.g.players[np].ai_attitude_extra[p], if near { nc::SCALED_NEIGHBOUR_ATTITUDE } else { 0 });
    }
    // Its lizardmen are the Clan of Embers' units too.
    let ranger = d.t::<UnitClassType>("UNITCLASS_RANGER");
    let clan = d.t::<CivilizationType>("CIVILIZATION_CLAN_OF_EMBERS");
    let kingdom = w.g.player_get_civilization_type(np).unwrap();
    assert_eq!(d.civ_unit(kingdom, ranger), d.civ_unit(clan, ranger));
}

#[test]
fn a_civilization_without_a_founder_knows_the_worlds_techs() {
    let mut w = world();
    let d = w.g.d;
    let (t0, t1) = (w.g.players[w.p0].team, w.g.players[w.p1].team);
    let known: Vec<TechType> = world::world_techs(&w.g);
    // Of two civilizations, a tech one knows is the world's; one neither knows is not.
    let fresh = d.techs.keys().find(|&t| d.techs[t].card.is_none() && !w.g.team_is_has_tech(t0, Some(t)) && !w.g.team_is_has_tech(t1, Some(t))).unwrap();
    assert!(!known.contains(&fresh));
    w.g.team_set_has_tech(t1, fresh, true, Some(w.p1), false, false);
    assert!(world::world_techs(&w.g).contains(&fresh));
    // Never a card.
    let card = d.t::<TechType>("TECH_CARD_USURPERS_CROWN");
    w.g.team_set_has_tech(t0, card, true, Some(w.p0), false, false);
    w.g.team_set_has_tech(t1, card, true, Some(w.p1), false, false);
    assert!(!world::world_techs(&w.g).contains(&card));
}

// ---------------------------------------------------------------------------------------------
// A06 The Brood.

#[test]
fn the_brood_nests_in_the_forests_and_hatches_where_a_nest_still_stands() {
    let mut w = world();
    let d = w.g.d;
    let forest = d.t::<FeatureType>("FEATURE_FOREST");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    for q in w.g.map.plot_ids() {
        if w.g.map.plots[q].feature_type.is_some() {
            w.g.plot_set_feature_type(q, None);
        }
    }
    assert!(nc::nest_sites(&w.g).is_empty() && !nc::valid_brood(&w.g), "no forest, no nest");
    // Two forests of 16 plots and one of 4, none touching another.
    let a = w.block(4, 4, &[]);
    let b = w.block(4, 4, &w.margin(&a));
    let small = w.block(2, 2, &w.margin(&[a.clone(), b.clone()].concat()));
    for &q in a.iter().chain(&b).chain(&small) {
        w.g.map.plots[q].terrain_type = Some(grass);
        w.g.plot_set_feature_type(q, Some(forest));
    }
    let sites = nc::nest_sites(&w.g);
    assert_eq!(sites.len(), 2, "a nest in each forest of 8 plots or more");
    assert!(sites.iter().all(|q| a.contains(q) || b.contains(q)));
    assert!(nc::valid_brood(&w.g));
    w.g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
    assert!(!nc::valid_brood(&w.g), "not without barbarians");
    w.g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = false;

    nc::fire_brood(&mut w.g, 0);
    let nest = nc::spider_nest(d).unwrap();
    for &q in &sites {
        assert_eq!(w.g.map.plots[q].improvement_type, Some(nest));
        assert_eq!(w.g.plot_lair_spawn_unit(q), Some(d.t::<UnitType>("UNIT_BABY_SPIDER")), "a lair of Baby Spiders");
    }
    assert!(!d.improvements[nest].permanent, "it can be destroyed");
    // One nest is cleared; a barbarian spider sits on the other.
    w.g.plot_set_improvement_type(sites[1], None);
    let barbarian = w.g.barbarian_player();
    let (x, y) = w.g.map.xy(sites[0]);
    let baby = w.g.player_init_unit(barbarian, d.t::<UnitType>("UNIT_BABY_SPIDER"), x, y, None, None);
    let np = world::next_player(&w.g);
    nc::fire_brood(&mut w.g, 1);

    assert!(w.g.players[np].alive);
    assert_eq!(w.civ(np), "CIVILIZATION_BROOD");
    let cities = w.g.players[np].cities.clone();
    assert_eq!(cities.len(), 1, "where a nest still stands");
    assert_eq!(w.g.city_plot(cities[0]), sites[0]);
    assert_eq!(w.g.map.plots[sites[0]].improvement_type, None);
    assert_eq!(w.units_of(np, "UNIT_BROOD_SPIDER").len(), nc::BROOD_SPIDERS);
    let queen = w.hero(np, "UNIT_SPIDER_QUEEN");
    assert!(d.units[w.g.units[queen].unit_type].large);
    assert_eq!((w.stat(queen, StatType::Hp), w.stat(queen, StatType::Attack)), (160, 90));
    assert!(w.g.units[queen].damage_type_combat[DamageType::DAMAGE_POISON] > 0, "poisoned hits");
    assert!(!w.g.units.contains(baby) || w.g.unit_plot(baby) != sites[0], "the barbarian spider is not in the city");
    // The Brood trains Giant Spiders at 60 hammers; nobody else trains them.
    let spider = d.t::<UnitType>("UNIT_BROOD_SPIDER");
    assert_eq!(d.units[spider].production_cost, 60);
    assert!(w.g.player_can_train(np, spider, false, false, false));
    let class = d.t::<UnitClassType>("UNITCLASS_GIANT_SPIDER");
    let wild = d.civ_unit(w.g.player_get_civilization_type(w.p0).unwrap(), class).unwrap();
    assert!(!w.g.player_can_train(w.p0, wild, false, false, false));
    assert_eq!(w.g.players[np].parent, None);
}

// ---------------------------------------------------------------------------------------------
// A22 The Infernal Gate Opens: the Court of Mammon.

impl World {
    /// Fires the Infernal Gate: the Court of Mammon's player.
    fn court(&mut self) -> PlayerId {
        assert!(nc::valid_infernal_gate(&self.g));
        let np = world::next_player(&self.g);
        nc::fire_infernal_gate(&mut self.g, 0);
        assert!(self.g.players[np].alive);
        assert_eq!(self.civ(np), "CIVILIZATION_COURT_OF_MAMMON");
        np
    }

    /// `p`'s team learns the Infernal Pact (the first team: the Infernals arrive); their player.
    fn pact(&mut self, p: PlayerId) -> PlayerId {
        let np = world::next_player(&self.g);
        let team = self.g.players[p].team;
        self.g.team_set_has_tech(team, self.g.d.t::<TechType>("TECH_INFERNAL_PACT"), true, Some(p), true, false);
        assert!(self.g.players[np].alive, "the Pact brings the Infernals");
        assert_eq!(self.civ(np), "CIVILIZATION_INFERNAL");
        np
    }

    /// A city of `p`'s on a free site of the land away from every city.
    fn city_away(&mut self, p: PlayerId) -> CityId {
        let taken: Vec<PlotIdx> = self.g.player_ids().flat_map(|q| self.g.players[q].cities.clone()).map(|c| self.g.city_plot(c)).collect();
        self.city(p, &taken)
    }
}

#[test]
fn the_infernal_gate_brings_the_court_of_mammon_beside_the_richest_civilization() {
    let mut w = world();
    let d = w.g.d;
    let c0 = w.city(w.p0, &[]);
    w.city(w.p1, &[w.g.city_plot(c0)]);
    // The most gold; ties to the lower id.
    w.g.players[w.p0].gold = 100;
    w.g.players[w.p1].gold = 100;
    assert_eq!(nc::court_patron(&w.g), Some(w.p0));
    w.g.players[w.p1].gold = 300;
    assert_eq!(nc::court_patron(&w.g), Some(w.p1));
    // Hyborem's option does not touch Mammon's court.
    w.g.setup.options[GameOptionType::GAMEOPTION_NO_HYBOREM_OR_BASIUM] = true;
    let site = nc::court_site(&w.g, w.p1).unwrap();
    assert!(world::within_range_of_borders(&w.g, w.p1, site, nc::COURT_RANGE), "beside it");
    assert!(world::free_site(&w.g, site, None), "a free site");
    let np = w.court();

    assert_eq!(w.g.player_get_leader_type(np), Some(d.t::<LeaderHeadType>("LEADER_AURIVANE")));
    assert_eq!(w.g.players[np].alignment, Some(AlignmentType::ALIGNMENT_EVIL));
    assert!(w.g.players[np].cities.is_empty(), "it arrives without a city");
    let hero = w.hero(np, "UNIT_AURIVANE");
    assert_eq!(w.g.unit_plot(hero), site);
    for (unit, n) in [("UNIT_CHAMPION", 2), ("UNIT_LONGBOWMAN", 2), ("UNIT_SETTLER", 2), ("UNIT_MANES", 2)] {
        let units = w.units_of(np, unit);
        assert_eq!(units.len(), n, "{unit}");
        assert!(units.iter().all(|&u| w.g.unit_plot(u) == site), "{unit} at the site");
    }
    for &u in &w.g.players[np].units {
        assert!(w.has(u, "PROMOTION_MAMMONS_COIN"), "every land unit may buy out");
    }
    assert!(w.g.players[np].gold >= nc::COURT_GOLD);
    for t in ["TRAIT_FALLOW", "TRAIT_AVARICIOUS"] {
        assert!(w.g.players[np].traits[d.t::<TraitType>(t)], "{t}");
    }
    assert!(w.g.players[np].ignore_food, "Fallow");
    assert_eq!(w.g.player_unit_rush_gold(np), 2);
    let civ = w.g.player_get_civilization_type(np).unwrap();
    assert!(!d.civilizations[civ].playable && !d.civilizations[civ].ai_playable, "no game starts with it");
    assert_eq!(w.g.players[np].parent, None);

    // Its cities start as the Infernals' do: size 3, Demonic Citizens.
    let (x, y) = w.g.map.xy(site);
    w.g.player_found(np, x, y);
    let c = w.g.map.plots[site].plot_city.expect("its city");
    assert_eq!(w.g.cities[c].population, 3);
    assert!(w.has_class(c, "BUILDINGCLASS_DEMONIC_CITIZENS"));
    assert!(w.has_class(c, "BUILDINGCLASS_PALACE"));
}

#[test]
fn the_infernal_pact_brings_the_infernals_with_the_court_in_play_and_before_it() {
    // The Court first, then the Pact.
    let mut w = world();
    let c0 = w.city(w.p0, &[]);
    w.city(w.p1, &[w.g.city_plot(c0)]);
    let court = w.court();
    let infernals = w.pact(w.p0);
    assert_eq!(w.g.players[court].ai_attitude_extra[infernals], nc::COURT_INFERNAL_ATTITUDE);
    assert_eq!(w.g.players[infernals].ai_attitude_extra[court], nc::COURT_INFERNAL_ATTITUDE);
    assert_eq!(w.g.players[infernals].ai_attitude_extra[w.p0], 4, "the Pact's founder");

    // The Pact first, then the Court.
    let mut w = world();
    let c0 = w.city(w.p0, &[]);
    w.city(w.p1, &[w.g.city_plot(c0)]);
    let infernals = w.pact(w.p1);
    let court = w.court();
    assert_eq!(w.g.players[court].ai_attitude_extra[infernals], nc::COURT_INFERNAL_ATTITUDE);
    assert_eq!(w.g.players[infernals].ai_attitude_extra[court], nc::COURT_INFERNAL_ATTITUDE);
    assert_eq!(w.g.players[court].ai_attitude_extra[w.p1], 0);
    assert!(!w.g.at_war(w.g.players[court].team, w.g.players[infernals].team), "no war is forced");
}

#[test]
fn the_manes_of_the_evil_dead_go_to_the_court_or_the_infernals() {
    let mut w = world();
    let d = w.g.d;
    let infernal = d.t::<CivilizationType>("CIVILIZATION_INFERNAL");
    let court_civ = nc::court_civ(d).unwrap();
    let c0 = w.city(w.p0, &[]);
    w.city(w.p1, &[w.g.city_plot(c0)]);
    // Neither in play: the Infernals, without a draw.
    let rng = w.g.soren_rand.clone();
    assert_eq!(nc::manes_civ(&mut w.g), infernal);
    assert_eq!(w.g.soren_rand, rng, "no draw");
    // The Court alone with a city: the Court, without a draw.
    let court = w.court();
    let rng = w.g.soren_rand.clone();
    assert_eq!(nc::manes_civ(&mut w.g), infernal, "the Court has no city yet");
    assert_eq!(w.g.soren_rand, rng, "no draw");
    w.city_away(court);
    let rng = w.g.soren_rand.clone();
    assert_eq!(nc::manes_civ(&mut w.g), court_civ);
    assert_eq!(w.g.soren_rand, rng, "no draw");
    // A dying unit of the Ashen Veil gifts the Court its Manes (or the population they make).
    let size = |g: &Game, p: PlayerId| -> i32 { g.players[p].cities.iter().map(|&c| g.cities[c].population).sum::<i32>() + g.players[p].units.iter().filter(|&&u| g.units[u].unit_type == d.t::<UnitType>("UNIT_MANES")).count() as i32 };
    let before = size(&w.g, court);
    let veil = d.t::<ReligionType>("RELIGION_THE_ASHEN_VEIL");
    let at = w.g.city_plot(c0);
    let (x, y) = w.g.map.xy(at);
    for _ in 0..10 {
        let u = w.g.player_init_unit(w.p0, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        w.g.units[u].religion = Some(veil);
        w.g.unit_kill(u, false, Some(w.p1));
    }
    assert!(size(&w.g, court) > before, "the Court took Manes");
    // Both in play: one of the two each time, drawn.
    let infernals = w.pact(w.p0);
    w.city_away(infernals);
    let mut got = Vec::new();
    for _ in 0..64 {
        let rng = w.g.soren_rand.clone();
        got.push(nc::manes_civ(&mut w.g));
        assert_ne!(w.g.soren_rand, rng, "one draw");
    }
    assert!(got.contains(&infernal) && got.contains(&court_civ));
    assert!(got.iter().all(|&c| c == infernal || c == court_civ));
    // A city the Court founded gives no Manes when razed, as an Infernal one.
    assert!(nc::infernal_population(d, Some(court_civ)) && nc::infernal_population(d, Some(infernal)));
    assert!(!nc::infernal_population(d, w.g.player_get_civilization_type(w.p0)));
}

#[test]
fn a_court_unit_buys_out_an_enemy_unit() {
    let mut w = world();
    let d = w.g.d;
    let c0 = w.city(w.p0, &[]);
    w.city(w.p1, &[w.g.city_plot(c0)]);
    let court = w.court();
    let spell = w.spell("SPELL_COURT_BUY_OUT");
    let buyer = w.units_of(court, "UNIT_CHAMPION")[0];
    let here = w.g.unit_plot(buyer);
    let (x, y) = w.g.map.xy(here);
    let next = (0..8).find_map(|i| {
        let q = w.g.plot_xy(x, y, [1, -1, 0, 0, 1, 1, -1, -1][i], [0, 0, 1, -1, 1, -1, 1, -1][i])?;
        (!w.g.plot_is_water(q) && w.g.map.plots[q].units.is_empty() && w.g.map.plots[q].plot_city.is_none()).then_some(q)
    });
    let next = next.expect("a land plot next to the Court");
    let (nx, ny) = w.g.map.xy(next);
    let axeman = d.t::<UnitType>("UNIT_AXEMAN");
    let t = w.g.player_init_unit(w.p0, axeman, nx, ny, None, None);
    assert!(!w.g.unit_can_cast_at(buyer, spell, SpellTarget::Unit(t)), "not at peace");
    let (tc, t0) = (w.g.players[court].team, w.g.players[w.p0].team);
    w.g.teams[tc].at_war[t0] = true;
    w.g.teams[t0].at_war[tc] = true;
    let price = w.g.unit_buy_out_price(t).unwrap();
    w.g.players[court].gold = price + 10;
    assert!(w.g.unit_can_cast_at(buyer, spell, SpellTarget::Unit(t)));
    // The Paymaster's spell is the Paymaster's; the Court's needs the Court.
    assert!(!w.g.unit_can_cast(buyer, w.spell("SPELL_BUY_OUT"), false));
    let (cx, cy) = w.g.map.xy(w.g.city_plot(c0));
    let enemy = w.g.player_init_unit(w.p0, axeman, cx, cy, None, None);
    w.g.unit_set_has_promotion(enemy, d.t::<PromotionType>("PROMOTION_MAMMONS_COIN"), true);
    assert!(!w.g.unit_can_cast(enemy, spell, false), "only while the Court owns it");
    w.g.unit_kill(enemy, false, None);

    w.g.unit_cast_at(buyer, spell, SpellTarget::Unit(t));
    assert!(!w.g.units.contains(t) || w.g.units[t].death_delay, "the unit it was is gone");
    assert_eq!(w.g.players[court].gold, 10);
    let bought = w.units_of(court, "UNIT_AXEMAN");
    assert_eq!(bought.len(), 1);
    assert_eq!(w.g.unit_plot(bought[0]), here);
    assert!(w.units_of(w.p0, "UNIT_AXEMAN").iter().all(|&u| w.g.units[u].death_delay));
}
