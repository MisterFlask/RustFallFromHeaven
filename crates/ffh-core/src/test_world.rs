//! A world for tests that exercise combat strengths over many kinds of units and plots
//! (the duel's refactor guard, `unit/duel_reference.rs`; engagement stats, `engagement/stats.rs`).

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// A world with many kinds of fighting units (land types with strength or typed strength, with
/// random promotions, damage and fortification; ships on water; animals and barbarians; a human
/// player) on many kinds of plots (cities with and without walls, forts, hills, forests, rivers,
/// water), the Armageddon counter raised.
pub(crate) fn world() -> (Game, Vec<UnitId>, Vec<PlotIdx>) {
    use crate::rng::Rng;
    use crate::setup::GameSetup;

    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 3, "WORLDSIZE_SMALL", 11));
    g.game_init();
    let mut rng = Rng::new(7);
    let mut r = |n: usize| rng.get(n as u16, "test") as usize;
    let civs: Vec<PlayerId> = g.player_ids().filter(|&p| g.players[p].alive && p != g.barbarian_player()).collect();
    g.players[civs[0]].human = true;
    // The easiest level: a barbarian bonus against humans.
    g.handicap = d.handicaps.keys().find(|&h| d.handicaps[h].barbarian_combat_modifier != 0).expect("a handicap with a barbarian bonus");
    g.global_counter = g.global_counter_limit / 3;

    // Plots of every kind: hills, forests and jungles, open land, snow, tundra and desert, rivers,
    // water.
    let land: Vec<PlotIdx> = g.map.plot_ids().filter(|&p| !g.plot_is_water(p) && !g.plot_is_impassable(p)).collect();
    let water: Vec<PlotIdx> = g.map.plot_ids().filter(|&p| g.plot_is_water(p)).collect();
    let kinds: [&dyn Fn(PlotIdx) -> bool; 5] = [
        &|p| g.plot_is_hills(p),
        &|p| g.map.plots[p].feature_type.is_some(),
        &|p| g.map.plots[p].feature_type.is_none() && !g.plot_is_hills(p),
        &|p| g.plot_is_river(p),
        &|_| true,
    ];
    let mut plots = Vec::new();
    let mut by_kind = Vec::new();
    for kind in kinds {
        let of_kind: Vec<PlotIdx> = land.iter().copied().filter(|&p| kind(p)).collect();
        let picked: Vec<PlotIdx> = if of_kind.is_empty() { Vec::new() } else { (0..10).map(|_| of_kind[r(of_kind.len())]).collect() };
        plots.extend(&picked);
        by_kind.push(picked);
    }
    // Snow, tundra, desert and marsh, whatever the map made.
    let open: Vec<PlotIdx> = by_kind[2].clone();
    for (i, &p) in open.iter().enumerate() {
        let name = ["TERRAIN_SNOW", "TERRAIN_TUNDRA", "TERRAIN_DESERT", "TERRAIN_MARSH"][i % 4];
        g.map.plots[p].terrain_type = Some(d.t::<TerrainType>(name));
    }
    by_kind.push(open);
    plots.extend((0..6).map(|_| water[r(water.len())]));

    // Cities (half of them walled) and forts of each civ.
    let walls = d.t::<BuildingType>("BUILDING_WALLS");
    let fort = d.t::<ImprovementType>("IMPROVEMENT_FORT");
    for (i, &p) in plots.clone().iter().enumerate().take(30) {
        if g.plot_is_water(p) || g.map.plots[p].plot_city.is_some() || !g.map.plots[p].units.is_empty() {
            continue;
        }
        let owner = civs[i % civs.len()];
        let (x, y) = g.map.xy(p);
        match i % 3 {
            0 => {
                let c = g.player_init_city(owner, x, y, true, true);
                if i % 2 == 0 {
                    g.city_set_num_real_building(c, walls, 1);
                }
            }
            1 => g.plot_set_improvement_type(p, Some(fort)),
            _ => {}
        }
    }

    let fighters: Vec<UnitType> = d
        .units
        .keys()
        .filter(|&t| {
            let info = &d.units[t];
            info.domain_type == Some(DomainType::DOMAIN_LAND) && (info.combat > 0 || info.combat_defense > 0 || info.damage_type_combat.iter().any(|&x| x != 0))
        })
        .collect();
    let ships: Vec<UnitType> = d.units.keys().filter(|&t| d.units[t].domain_type == Some(DomainType::DOMAIN_SEA) && d.units[t].combat > 0).collect();
    let promotions: Vec<PromotionType> = d.promotions.keys().collect();
    // Promotions with a terrain, hills, feature, city, unit combat, promotion or Homeland term.
    let combat_promotions: Vec<PromotionType> = [
        "PROMOTION_WOODSMAN1",
        "PROMOTION_GUERILLA1",
        "PROMOTION_ELF",
        "PROMOTION_FROSTLING",
        "PROMOTION_NOMAD",
        "PROMOTION_WINTERBORN",
        "PROMOTION_DEMON",
        "PROMOTION_LIZARDMAN",
        "PROMOTION_HOMELAND",
        "PROMOTION_STIGMATA",
        "PROMOTION_CITY_GARRISON1",
        "PROMOTION_CITY_RAIDER1",
        "PROMOTION_SHOCK",
        "PROMOTION_FORMATION",
        "PROMOTION_DEMON_SLAYING",
        "PROMOTION_UNDEAD",
        "PROMOTION_NETHER_BLADE",
        "PROMOTION_HERO",
        "PROMOTION_FIRE_RESISTANCE",
        "PROMOTION_FLAMING_ARROWS",
        "PROMOTION_DEFENSIVE",
        "PROMOTION_SHADOWWALK",
        "PROMOTION_AMPHIBIOUS",
    ]
    .iter()
    .map(|name| d.t::<PromotionType>(name))
    .collect();
    let barbarian = g.barbarian_player();
    let mut units = Vec::new();
    for i in 0..120 {
        let p = plots[r(plots.len())];
        let (x, y) = g.map.xy(p);
        let ty = if g.plot_is_water(p) { ships[r(ships.len())] } else { fighters[(i * 37 + r(7)) % fighters.len()] };
        let owner = if d.units[ty].animal || i % 9 == 0 { barbarian } else { civs[r(civs.len())] };
        let u = g.player_init_unit(owner, ty, x, y, None, None);
        for _ in 0..r(3) {
            g.unit_set_has_promotion(u, combat_promotions[r(combat_promotions.len())], true);
        }
        for _ in 0..r(4) {
            if g.units.contains(u) {
                g.unit_set_has_promotion(u, promotions[r(promotions.len())], true);
            }
        }
        if g.units.contains(u) {
            g.units[u].fortify_turns = r(6) as i32;
            g.units[u].damage = r(70) as i32;
            units.push(u);
        }
    }
    // Terrain, feature and hills specialists where their terms apply.
    let warrior = d.t::<UnitType>("UNIT_AXEMAN");
    let (frostling, winterborn, nomad, lizardman) = ("PROMOTION_FROSTLING", "PROMOTION_WINTERBORN", "PROMOTION_NOMAD", "PROMOTION_LIZARDMAN");
    for (kind, names) in [
        (0, vec!["PROMOTION_GUERILLA1", "PROMOTION_GUERILLA2", "PROMOTION_GUERILLA1", "PROMOTION_GUERILLA2"]),
        (1, vec!["PROMOTION_WOODSMAN1", "PROMOTION_ELF", "PROMOTION_WOODSMAN2", "PROMOTION_ORC"]),
        // (snow, tundra, desert, marsh, twice)
        (5, vec![frostling, winterborn, nomad, lizardman, frostling, frostling, nomad, lizardman]),
    ] {
        for (i, name) in names.iter().enumerate() {
            let Some(&p) = by_kind[kind].get(i) else { continue };
            let (x, y) = g.map.xy(p);
            let u = g.player_init_unit(civs[i % civs.len()], warrior, x, y, None, None);
            g.unit_set_has_promotion(u, d.t::<PromotionType>(name), true);
            units.push(u);
        }
    }
    // (Some promotions end a unit, some placements bump one.)
    units.retain(|&u| g.units.contains(u) && g.unit_plot_opt(u).is_some());
    (g, units, plots)
}
