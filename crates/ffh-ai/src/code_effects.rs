//! Building effects the rules implement in code. FfH 0.41 gives a building's plain effects
//! (yields, happiness, defense, experience...) in the XML, which `production::building_value`
//! reads, and its special ones in Python, ported to Rust in `ffh_core::ffh` (`events.rs`):
//! the Warrens' copies, the Planar Gate's creatures, the Cave of Ancestors' experience... Those are
//! invisible in the data, so a building whose worth lies there was valued by its XML alone, often
//! 0 (FfH's own AI had the same blind spot: `CvCityAI::AI_buildingValue` sees them only through
//! `iAIWeight`, which is 0 for all of them). This table says what each such effect does, and
//! [`building_value`] values it on the generic scale (one hammer a turn is worth 150).
//!
//! An entry describes the rules' code, not a judgement of the building: when the code changes,
//! change its entry. The switch `CODE_EFFECTS` (`FFH_AI_CODE_EFFECTS_OFF` / `_ONLY`, per
//! civilization) leaves the table out but for the Warrens, which the Clan of Embers' module valued
//! before the table took them. Not covered:
//! the Asylum (10% Crazed: an Enraged unit, +20% attack and -20% defense, fights on its own; a
//! wash), buildings that only open a spell (Arena, Demon's Altar sacrifice, Drown: the spell's
//! logic decides when they are worth it), the Dwarven Vaults (set by the rules, never built: the
//! Khazad's finance keeps the gold for them), the barbarians' Dragon's Horde and the Grand
//! Menagerie (a human trophy).

use ffh_core::ffh::events::{MERCURIAN_GATE_UNITS, PLANAR_GATE_SUMMONS};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// Which of our units an effect on units applies to.
#[derive(Copy, Clone, Debug)]
enum Units {
    /// Units with this promotion (a race, Golem, Demon...).
    Promotion(&'static str),
    /// Units of this unit combat.
    Combat(&'static str),
    /// Every unit with a unit combat.
    All,
}

/// How much experience an effect gives.
#[derive(Copy, Clone, Debug)]
enum Xp {
    Fixed(i32),
    /// One per mana resource the city has (Cave of Ancestors).
    PerMana,
}

#[derive(Copy, Clone, Debug)]
enum Effect {
    /// Every unit finished in the city gets a free copy (Warrens: not world, national, siege or
    /// naval units; settlers and workers too).
    Copy,
    /// Units finished in the city get experience, weighted in percent (a caster's levels open
    /// its spheres and its upgrade: twice a fighter's).
    Experience(Units, Xp, i32),
    /// Units finished in the city get a promotion with a chance in percent.
    Promotion(Units, i32),
    /// Every turn a chance to summon a creature for each of the city's buildings in
    /// `PLANAR_GATE_SUMMONS`, up to a limit per Planar Gate (`ev_planar_gate`).
    PlanarGate,
    /// Works on the enemies next to the city (fire on the strongest stack, a copy of an attacker,
    /// our fallen back as demons, production from the dead): worth this much while the city is
    /// threatened; in a city near a rival civilization, half at war and a quarter at peace (an
    /// expert builds it on the border before the war comes); elsewhere a quarter at war, a tenth
    /// at peace.
    Siege(i32),
    /// With open borders to three players, every tech three of them know (`ev_eyes_and_ears_network`).
    OpenBordersTechs,
    /// When built, the land within the radius becomes this terrain (forest stays, jungle becomes
    /// forest, flames, flood plains and scrub go).
    Terrain(&'static str, i32),
    /// When built, our units with this promotion get Strong; so do those created later.
    StrongFor(&'static str),
    /// When built, a free unit of one of these types (the rules pick one).
    FreeUnit(&'static [&'static str]),
    /// When built, all these units, for a new civilization on our team (the Mercurians).
    FreeUnits(&'static [&'static str]),
    /// When built, a chance in percent of this unit appearing near the city for the barbarians.
    HostileSpawn(&'static str, i32),
}

const ELEMENTALS: [&str; 4] = ["UNIT_AIR_ELEMENTAL", "UNIT_EARTH_ELEMENTAL", "UNIT_FIRE_ELEMENTAL", "UNIT_WATER_ELEMENTAL"];

/// Each building's effects in the rules' code (`ffh_core::ffh::events`).
const TABLE: [(&str, &[Effect]); 19] = [
    // ev_unit_built
    ("BUILDING_WARRENS", &[Effect::Copy]),
    ("BUILDING_CAVE_OF_ANCESTORS", &[Effect::Experience(Units::Combat("UNITCOMBAT_ADEPT"), Xp::PerMana, 200)]),
    ("BUILDING_BREWERY", &[Effect::Experience(Units::Promotion("PROMOTION_DWARF"), Xp::Fixed(2), 100)]),
    ("BUILDING_DEMONS_ALTAR", &[Effect::Experience(Units::Promotion("PROMOTION_DEMON"), Xp::Fixed(2), 100)]),
    ("BUILDING_CHANCEL_OF_GUARDIANS", &[Effect::Promotion(Units::All, 20)]),
    ("BUILDING_BLASTING_WORKSHOP", &[Effect::Promotion(Units::Promotion("PROMOTION_GOLEM"), 100)]),
    ("BUILDING_PALLENS_ENGINE", &[Effect::Promotion(Units::Promotion("PROMOTION_GOLEM"), 100)]),
    ("BUILDING_ADULARIA_CHAMBER", &[Effect::Promotion(Units::Promotion("PROMOTION_GOLEM"), 100)]),
    // ev_city_do_turn
    ("BUILDING_PLANAR_GATE", &[Effect::PlanarGate]),
    ("BUILDING_CITADEL_OF_LIGHT", &[Effect::Siege(600)]),
    ("BUILDING_HALL_OF_MIRRORS", &[Effect::Siege(450)]),
    ("BUILDING_EYES_AND_EARS_NETWORK", &[Effect::OpenBordersTechs]),
    // ev_building_built: the Mercurians join our team with these units (its `iAIWeight` of 300
    // was FfH's stand-in for them)
    ("BUILDING_MERCURIAN_GATE", &[Effect::FreeUnits(&MERCURIAN_GATE_UNITS)]),
    // ev_unit_killed
    ("BUILDING_SOUL_FORGE", &[Effect::Siege(300)]),
    ("BUILDING_MOKKAS_CAULDRON", &[Effect::Siege(200)]),
    // ev_building_built (and ev_unit_created for the towers' later units)
    ("BUILDING_TEMPLE_OF_THE_HAND", &[Effect::Terrain("TERRAIN_SNOW", 2)]),
    ("BUILDING_TOWER_OF_THE_ELEMENTS", &[Effect::StrongFor("PROMOTION_ELEMENTAL"), Effect::FreeUnit(&ELEMENTALS)]),
    ("BUILDING_TOWER_OF_NECROMANCY", &[Effect::StrongFor("PROMOTION_UNDEAD")]),
    ("BUILDING_INFERNAL_GRIMOIRE", &[Effect::HostileSpawn("UNIT_BALOR", 20)]),
];

/// One hammer a turn, on `building_value`'s scale (yield weight 15 x 10).
const HAMMER: i32 = 150;
/// The turns a free or summoned unit is taken to serve: a one-time unit is worth its hammers
/// spread over them.
const UNIT_LIFE: i32 = 50;
/// A free promotion, as a share of the unit's cost in percent (a first promotion takes 2
/// experience: Combat I is +10% strength, Defensive or Fire II more for the unit it suits).
const PROMOTION_WORTH: i32 = 15;

/// The table by building type (most have no effects), and the two buildings named in the hot
/// path. Built once from the game data.
struct ByType {
    effects: IdxVec<BuildingType, Vec<Effect>>,
    gate: Option<BuildingType>,
    warrens: Option<BuildingType>,
}

fn by_type(d: &GameData) -> &'static ByType {
    static BY_TYPE: std::sync::OnceLock<ByType> = std::sync::OnceLock::new();
    BY_TYPE.get_or_init(|| {
        let mut effects = IdxVec::from_elem(Vec::new(), d.buildings.len());
        for &(name, list) in &TABLE {
            if let Some(b) = d.ty::<BuildingType>(name) {
                effects[b] = list.to_vec();
            }
        }
        ByType { effects, gate: d.ty::<BuildingType>("BUILDING_PLANAR_GATE"), warrens: d.ty::<BuildingType>("BUILDING_WARRENS") }
    })
}

/// What the effects the rules give building `b` in code are worth in city `c`, on
/// `production::building_value`'s scale; 0 for a building without any. Also the creature a
/// Planar Gate already in the city would summon for `b`.
pub fn building_value(g: &Game, c: CityId, b: BuildingType, threat: i32) -> i32 {
    let t = by_type(g.d);
    let gate_here = t.gate.filter(|&gate| gate != b && g.cities[c].num_real_building[gate] > 0);
    if t.effects[b].is_empty() && gate_here.is_none() {
        return 0;
    }
    // The Warrens were valued before the table (the Clan of Embers' module), so the switch leaves
    // them as they were.
    if Some(b) != t.warrens && !crate::switches::CODE_EFFECTS.on(g, g.cities[c].owner) {
        return 0;
    }
    let mut v: i32 = t.effects[b].iter().map(|&e| effect_value(g, c, b, e, threat)).sum();
    if gate_here.is_some() {
        v += summons_value(g, c, Some(b));
    }
    v
}

fn effect_value(g: &Game, c: CityId, b: BuildingType, e: Effect, threat: i32) -> i32 {
    let d = g.d;
    let p = g.cities[c].owner;
    let prod = g.city_yield_rate(c, YieldType::YIELD_PRODUCTION).max(0);
    match e {
        // The copies double what the city puts into units, about half of its production.
        Effect::Copy => prod * HAMMER / 2,
        // As `production::building_value` values free experience (`xp * prod.min(15) * 4`), for
        // the share of our units the effect applies to.
        Effect::Experience(units, xp, weight) => {
            let xp = match xp {
                Xp::Fixed(n) => n,
                Xp::PerMana => {
                    let mana = d.ty::<BonusClassType>("BONUSCLASS_MANA");
                    d.bonuses.keys().filter(|&x| mana.is_some() && d.bonuses[x].bonus_class_type == mana && g.city_has_bonus(c, x)).count() as i32
                }
            };
            xp * prod.min(15) * 4 * share(g, c, units) / 100 * weight / 100
        }
        // The hammers that go into such units (half the production) gain PROMOTION_WORTH.
        Effect::Promotion(units, chance) => prod * HAMMER / 2 * share(g, c, units) / 100 * PROMOTION_WORTH / 100 * chance / 100,
        Effect::PlanarGate => summons_value(g, c, None),
        Effect::Siege(value) => {
            let war = civ_war(g, p);
            if threat > 0 {
                value
            } else if frontier(g, c) {
                if war { value / 2 } else { value / 4 }
            } else if war {
                value / 4
            } else {
                value / 10
            }
        }
        Effect::OpenBordersTechs => open_borders_techs_value(g, p),
        Effect::Terrain(terrain, radius) => d.ty::<TerrainType>(terrain).map_or(0, |t| terrain_value(g, c, t, radius)),
        Effect::StrongFor(promotion) => {
            let Some(promo) = d.ty::<PromotionType>(promotion) else { return 0 };
            let strong = d.t::<PromotionType>("PROMOTION_STRONG");
            // Our units that gain it now, each worth a promotion over its life.
            let now: i32 = g.players[p]
                .units
                .iter()
                .filter(|&&u| g.units[u].has_promotion[promo] && !g.units[u].has_promotion[strong])
                .map(|&u| unit_hammers(d, &d.units[g.units[u].unit_type]) * HAMMER / UNIT_LIFE * PROMOTION_WORTH / 100)
                .sum();
            now + effect_value(g, c, b, Effect::Promotion(Units::Promotion(promotion), 100), threat)
        }
        Effect::FreeUnit(units) => {
            let costs: Vec<i32> = units.iter().filter_map(|&n| d.ty::<UnitType>(n)).map(|u| unit_hammers(d, &d.units[u])).collect();
            if costs.is_empty() {
                return 0;
            }
            costs.iter().sum::<i32>() / costs.len() as i32 * HAMMER / UNIT_LIFE
        }
        Effect::FreeUnits(units) => units.iter().filter_map(|&n| d.ty::<UnitType>(n)).map(|u| unit_hammers(d, &d.units[u]) * HAMMER / UNIT_LIFE).sum(),
        // A barbarian of that strength near our city costs more than its hammers: twice them.
        Effect::HostileSpawn(unit, chance) => d.ty::<UnitType>(unit).map_or(0, |u| -2 * unit_hammers(d, &d.units[u]) * HAMMER / UNIT_LIFE * chance / 100),
    }
}

/// A unit's worth in hammers: its cost, or for a unit that cannot be trained (summons,
/// elementals) the larger of the cost of a trained unit of its strength (Axeman 4 for 60,
/// Champion 6 for 120: strength squared x 10 / 3) and the mean cost of the trained units of its
/// tier (a summoned caster is worth more than its strength).
fn unit_hammers(d: &GameData, info: &UnitInfo) -> i32 {
    static TIER_COST: std::sync::OnceLock<Vec<i32>> = std::sync::OnceLock::new();
    if info.production_cost > 0 {
        return info.production_cost;
    }
    let tiers = TIER_COST.get_or_init(|| {
        let max = d.units.iter().map(|u| u.tier).max().unwrap_or(0).max(0) as usize;
        (0..=max)
            .map(|t| {
                let costs: Vec<i32> = d.units.iter().filter(|u| u.tier == t as i32 && u.production_cost > 0 && u.combat > 0).map(|u| u.production_cost).collect();
                if costs.is_empty() {
                    0
                } else {
                    costs.iter().sum::<i32>() / costs.len() as i32
                }
            })
            .collect()
    });
    let by_tier = usize::try_from(info.tier).ok().and_then(|t| tiers.get(t).copied()).unwrap_or(0);
    (info.combat * info.combat * 10 / 3).max(by_tier)
}

/// Does unit type `info` (of civilization `civ`'s line) come with what `units` covers?
fn type_matches(d: &GameData, civ: CivilizationType, info: &UnitInfo, units: Units) -> bool {
    match units {
        Units::All => info.unit_combat_type.is_some(),
        Units::Combat(name) => d.ty::<UnitCombatType>(name).is_some_and(|c| info.unit_combat_type == Some(c)),
        // A race comes with the civilization (`CvUnit::init`'s default race) or the unit.
        Units::Promotion(name) => d.ty::<PromotionType>(name).is_some_and(|pr| {
            info.free_promotions[pr] || (d.civilizations[civ].default_race == Some(pr) && info.domain_type == Some(DomainType::DOMAIN_LAND) && info.combat > 0)
        }),
    }
}

/// The share, in percent, of the units city `c` trains that `units` covers: the share of our
/// units (those with a unit combat) it covers, and in a city that can train such units, that
/// share times the cities over those that can (adepts come only from Mage Guild cities, so a guild
/// city trains most of them).
fn share(g: &Game, c: CityId, units: Units) -> i32 {
    let d = g.d;
    let p = g.cities[c].owner;
    let matches: Box<dyn Fn(UnitId) -> bool> = match units {
        Units::All => Box::new(|_| true),
        Units::Promotion(name) => {
            let Some(promo) = d.ty::<PromotionType>(name) else { return 0 };
            Box::new(move |u| g.units[u].has_promotion[promo])
        }
        Units::Combat(name) => {
            let Some(combat) = d.ty::<UnitCombatType>(name) else { return 0 };
            Box::new(move |u| g.unit_get_unit_combat_type(u) == Some(combat))
        }
    };
    let (mut all, mut hit) = (0, 0);
    for &u in &g.players[p].units {
        if g.unit_get_unit_combat_type(u).is_some() {
            all += 1;
            hit += i32::from(matches(u));
        }
    }
    if all == 0 || hit == 0 {
        return 0;
    }
    let share = hit * 100 / all;
    if matches!(units, Units::All) {
        return share;
    }
    let Some(civ) = g.player_get_civilization_type(p) else { return share };
    let types: Vec<UnitType> = d
        .units
        .keys()
        .filter(|&u| d.units[u].unit_class_type.is_some_and(|cl| d.civ_unit(civ, cl) == Some(u)) && type_matches(d, civ, &d.units[u], units))
        .collect();
    let trains = |city: CityId| types.iter().any(|&u| g.city_can_train(city, u, false, false, false, false));
    if !trains(c) {
        return share;
    }
    let cities = g.players[p].cities.len() as i32;
    let trainers = g.players[p].cities.iter().filter(|&&o| trains(o)).count() as i32;
    (share * cities / trainers.max(1)).min(100)
}

/// The creatures a Planar Gate in city `c` would summon, each worth its hammers over
/// [`UNIT_LIFE`] (the summons are capped per class, so a creature comes back only once one
/// dies): those of the city's buildings, and at half those of the buildings it can build now;
/// or with `extra` the one creature building `extra` would add.
fn summons_value(g: &Game, c: CityId, extra: Option<BuildingType>) -> i32 {
    let d = g.d;
    let p = g.cities[c].owner;
    let Some(gate) = d.ty::<BuildingType>("BUILDING_PLANAR_GATE") else { return 0 };
    let (per_gate, _) = g.planar_gate_limits();
    let gates = g.player_count_num_buildings(p, gate) + i32::from(g.cities[c].num_real_building[gate] == 0);
    let max = per_gate * gates.max(1);
    let mut v = 0;
    for (building, class, unit) in PLANAR_GATE_SUMMONS {
        let (Some(bt), Some(cl), Some(ut)) = (d.ty::<BuildingType>(building), d.ty::<UnitClassType>(class), d.ty::<UnitType>(unit)) else { continue };
        if g.players[p].unit_class_count[cl] >= max {
            continue;
        }
        let worth = unit_hammers(d, &d.units[ut]) * HAMMER / UNIT_LIFE;
        let has = g.city_num_building(c, bt) > 0;
        v += match extra {
            Some(x) => if x == bt && !has { worth } else { 0 },
            None if has => worth,
            None if g.city_can_construct(c, bt, false, false, false) => worth / 2,
            None => 0,
        };
    }
    v
}

/// The distance within which a rival civilization's city makes ours a frontier city.
const FRONTIER: i32 = 8;

/// Is city `c` near a city of a civilization we have met (not ours, not the barbarians)?
fn frontier(g: &Game, c: CityId) -> bool {
    let p = g.cities[c].owner;
    let team = g.players[p].team;
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    g.player_ids().any(|o| {
        let ot = g.players[o].team;
        ot != team
            && g.players[o].alive
            && !g.player_is_barbarian(o)
            && g.teams[team].has_met[ot]
            && g.players[o].cities.iter().any(|&oc| g.plot_distance(x, y, g.cities[oc].x, g.cities[oc].y) <= FRONTIER)
    })
}

/// At war with a civilization (not the barbarians).
fn civ_war(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t))
}

/// Eyes and Ears Network: with open borders to three players, the techs three of their teams
/// know (spread over [`UNIT_LIFE`] turns), and the ones they learn later (a quarter of our
/// research), scaled down while fewer than three have open borders with us.
fn open_borders_techs_value(g: &Game, p: PlayerId) -> i32 {
    let d = g.d;
    let team = g.players[p].team;
    let partners: Vec<TeamId> = g.player_ids().filter(|&o| o != p && g.players[o].alive && g.team_is_open_borders(team, g.players[o].team)).map(|o| g.players[o].team).collect();
    // One beaker a turn on the generic scale: commerce weight 9 x 10.
    let research = g.player_get_commerce_rate(p, CommerceType::COMMERCE_RESEARCH).max(0);
    let n = (partners.len() as i32).min(3);
    let mut v = research * 90 / 4 * n / 3;
    if partners.len() >= 3 {
        let excluded = ["TECH_ARETE", "TECH_MIND_STAPLING", "TECH_HIDDEN_PATHS", "TECH_INFERNAL_PACT", "TECH_SEAFARING"];
        let beakers: i32 = d
            .techs
            .keys()
            .filter(|&t| !excluded.contains(&d.techs[t].base.type_.as_str()) && !g.team_is_has_tech(team, Some(t)))
            .filter(|&t| partners.iter().filter(|&&pt| g.team_is_has_tech(pt, Some(t))).count() >= 3)
            .map(|t| g.team_get_research_cost(team, t))
            .sum();
        v += beakers * 90 / UNIT_LIFE;
    }
    v
}

/// The yields the land within `radius` of city `c` gains (or loses) as terrain `to`, at half
/// weight (the city works some of it), on the generic yield weights (food 20, production 15,
/// commerce 7, x 10). The features follow `ev_building_built`'s Temple of the Hand: forest
/// stays, jungle becomes forest, flames, flood plains and scrub go.
fn terrain_value(g: &Game, c: CityId, to: TerrainType, radius: i32) -> i32 {
    let d = g.d;
    let p = g.cities[c].owner;
    let civ = g.player_get_civilization_type(p);
    let forest = d.ty::<FeatureType>("FEATURE_FOREST");
    let jungle = d.ty::<FeatureType>("FEATURE_JUNGLE");
    let removed: Vec<FeatureType> = ["FEATURE_FLAMES", "FEATURE_FLOOD_PLAINS", "FEATURE_SCRUB"].iter().filter_map(|&n| d.ty::<FeatureType>(n)).collect();
    // The terrain- and feature-dependent part of `CvPlot::calculateNatureYield`.
    let nature = |q: PlotIdx, t: TerrainType, f: Option<FeatureType>, y: usize| {
        let ti = &d.terrains[t];
        let fi = f.map(|f| &d.features[f]);
        let mut v = ti.yield_[y];
        if g.plot_is_river(q) {
            v += fi.map_or(ti.river_yield_change[y], |fi| fi.river_yield_change[y]);
        }
        if g.plot_is_hills(q) {
            v += fi.map_or(ti.hills_yield_change[y], |fi| fi.hills_yield_change[y]);
        }
        v += fi.map_or(0, |fi| fi.yield_change[y]);
        if civ.is_some() && civ == ti.civilization_yield_type {
            v += ti.civilization_yield_change[y];
        }
        v
    };
    let weight = [20, 15, 7];
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    let mut v = 0;
    for xx in x - radius..=x + radius {
        for yy in y - radius..=y + radius {
            let Some(q) = g.map.plot_xy(xx, yy) else { continue };
            if g.plot_is_water(q) || g.plot_is_peak(q) {
                continue;
            }
            let Some(from) = g.map.plots[q].terrain_type else { continue };
            let feature = g.map.plots[q].feature_type;
            let after = if feature.is_some() && feature == jungle {
                forest
            } else if feature.is_some_and(|f| removed.contains(&f)) {
                None
            } else {
                feature
            };
            for i in 0..YieldType::NUM {
                v += (nature(q, to, after, i) - nature(q, from, feature, i)) * weight[i] * 10 / 2;
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_names_a_building_and_its_types() {
        let d = ffh_data::global();
        for &(name, list) in &TABLE {
            assert!(d.ty::<BuildingType>(name).is_some(), "{name}");
            for &e in list {
                match e {
                    Effect::Experience(u, _, _) | Effect::Promotion(u, _) => match u {
                        Units::Promotion(n) => assert!(d.ty::<PromotionType>(n).is_some(), "{n}"),
                        Units::Combat(n) => assert!(d.ty::<UnitCombatType>(n).is_some(), "{n}"),
                        Units::All => {}
                    },
                    Effect::Terrain(t, _) => assert!(d.ty::<TerrainType>(t).is_some(), "{t}"),
                    Effect::StrongFor(n) => assert!(d.ty::<PromotionType>(n).is_some(), "{n}"),
                    Effect::FreeUnit(us) | Effect::FreeUnits(us) => us.iter().for_each(|n| assert!(d.ty::<UnitType>(n).is_some(), "{n}")),
                    Effect::HostileSpawn(n, _) => assert!(d.ty::<UnitType>(n).is_some(), "{n}"),
                    Effect::Copy | Effect::PlanarGate | Effect::Siege(_) | Effect::OpenBordersTechs => {}
                }
            }
        }
        for (b, c, u) in PLANAR_GATE_SUMMONS {
            assert!(d.ty::<BuildingType>(b).is_some() && d.ty::<UnitClassType>(c).is_some() && d.ty::<UnitType>(u).is_some(), "{b}");
        }
    }

    #[test]
    fn summons_are_worth_their_strength() {
        let d = ffh_data::global();
        // Untrainable creatures: at least their strength's worth (Minotaur 8 -> 213 hammers), more
        // when their tier's trained units cost more; trained units by cost.
        let minotaur = &d.units[d.t::<UnitType>("UNIT_MINOTAUR")];
        assert!(unit_hammers(d, minotaur) >= 213, "{}", unit_hammers(d, minotaur));
        assert_eq!(unit_hammers(d, &d.units[d.t::<UnitType>("UNIT_AXEMAN")]), 60);
    }
}
