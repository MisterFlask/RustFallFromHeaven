//! The House perks (docs/HOUSES.md, rule 9): trait rows that are never a leader's own trait.

use ffh_data::*;

fn d() -> &'static GameData {
    global()
}

fn perk(name: &str) -> &'static TraitInfo {
    &d().traits[d().t::<TraitType>(name)]
}

fn combat(name: &str) -> UnitCombatType {
    d().t::<UnitCombatType>(name)
}

const FAMILIES: [&str; 14] = [
    "HORSEMASTERS",
    "SHIELDWALL",
    "MARKSMEN",
    "BATTLE_MAGES",
    "DRILLMASTERS",
    "SHIPWRIGHTS",
    "FREEHOLDERS",
    "PATRONS",
    "SCHOLARS",
    "MERCHANTS",
    "SMITHS",
    "MASTER_BUILDERS",
    "RITUALISTS",
    "LUMINARIES",
];

#[test]
fn fourteen_families_of_three_tiers_none_a_leader_trait() {
    let d = d();
    let perks: Vec<TraitType> = d.traits.keys().filter(|&t| d.traits[t].is_house_perk()).collect();
    assert_eq!(perks.len(), 42);
    for family in FAMILIES {
        for tier in 1..=3 {
            let name = format!("TRAIT_PERK_{family}_{tier}");
            let t = d.t::<TraitType>(&name);
            let info = &d.traits[t];
            assert_eq!(info.house_perk_tier, tier, "{name}");
            assert_eq!(info.house_perk_family, family, "{name}");
            assert!(!info.selectable, "{name}");
            assert_ne!(d.text.get(&info.base.description), info.base.description, "{name} has a name");
        }
    }
    for t in perks {
        assert!(d.leader_heads.iter().all(|l| !l.has_trait[t] && l.permanent_trait != Some(t)), "{}", d.type_name(t));
        assert!(d.civilizations.iter().all(|c| c.civ_trait != Some(t)), "{}", d.type_name(t));
    }
    for t in d.traits.keys().filter(|&t| !d.traits[t].is_house_perk()) {
        assert!(d.traits[t].house_perk_family.is_empty(), "{}", d.type_name(t));
    }
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
}

#[test]
fn house_perks_by_tier() {
    let d = d();
    for tier in 1..=3 {
        let list = d.house_perks(tier);
        assert_eq!(list.len(), 14, "tier {tier}");
        assert!(list.iter().all(|&t| d.traits[t].house_perk_tier == tier));
        assert!(list.windows(2).all(|w| w[0] < w[1]), "table order");
        let families: Vec<&str> = list.iter().map(|&t| d.traits[t].house_perk_family.as_str()).collect();
        assert_eq!(families, FAMILIES, "tier {tier}");
    }
    assert!(d.house_perks(0).is_empty());
    assert!(d.house_perks(4).is_empty());
}

#[test]
fn every_free_promotion_can_be_held_by_its_unit_combat() {
    let d = d();
    let mut given = 0;
    for &t in (1..=3).flat_map(|tier| d.house_perks(tier)) {
        for c in d.unit_combats.keys() {
            for &p in &d.traits[t].house_unit_combat_free_promotions[c] {
                assert!(d.promotions[p].unit_combat[c], "{}: {} for {}", d.type_name(t), d.type_name(p), d.type_name(c));
                given += 1;
            }
        }
    }
    assert_eq!(given, 4 * (1 + 2 + 3));
}

#[test]
fn values_from_the_table() {
    let d = d();
    let freeholders = perk("TRAIT_PERK_FREEHOLDERS_3");
    assert_eq!(freeholders.house_distance_maintenance_modifier, -100);
    assert_eq!(freeholders.house_num_cities_maintenance_modifier, -50);
    assert_eq!(perk("TRAIT_PERK_FREEHOLDERS_1").house_num_cities_maintenance_modifier, 0);

    let patrons = perk("TRAIT_PERK_PATRONS_2");
    assert_eq!(patrons.house_commerce_modifier[CommerceType::COMMERCE_CULTURE.idx()], 100);
    assert_eq!(patrons.house_commerce_modifier.iter().sum::<i32>(), 100);

    let mounted = combat("UNITCOMBAT_MOUNTED");
    let horse = perk("TRAIT_PERK_HORSEMASTERS_3");
    let names: Vec<&str> = horse.house_unit_combat_free_promotions[mounted].iter().map(|&p| d.type_name(p)).collect();
    assert_eq!(names, ["PROMOTION_FLANKING1", "PROMOTION_FLANKING2", "PROMOTION_MOBILITY1"]);
    assert_eq!(horse.house_unit_combat_production_modifier[mounted], 25);
    assert_eq!(perk("TRAIT_PERK_HORSEMASTERS_2").house_unit_combat_production_modifier[mounted], 0);

    let naval = combat("UNITCOMBAT_NAVAL");
    for (tier, xp, prod) in [(1, 2, 25), (2, 4, 50), (3, 6, 75)] {
        let ship = perk(&format!("TRAIT_PERK_SHIPWRIGHTS_{tier}"));
        assert!(ship.house_requires_coastal);
        assert_eq!(ship.house_unit_combat_free_experience[naval], xp);
        assert_eq!(ship.house_unit_combat_production_modifier[naval], prod);
    }
    let coastal: Vec<&str> = d.traits.iter().filter(|t| t.house_requires_coastal).map(|t| t.house_perk_family.as_str()).collect();
    assert_eq!(coastal, ["SHIPWRIGHTS"; 3]);

    assert_eq!(perk("TRAIT_PERK_SMITHS_3").house_yield_modifier[YieldType::YIELD_PRODUCTION.idx()], 25);
    assert_eq!(perk("TRAIT_PERK_SCHOLARS_3").house_commerce_modifier[CommerceType::COMMERCE_RESEARCH.idx()], 40);
    assert_eq!(perk("TRAIT_PERK_MERCHANTS_1").house_commerce_modifier[CommerceType::COMMERCE_GOLD.idx()], 15);
    assert_eq!(perk("TRAIT_PERK_DRILLMASTERS_2").house_free_experience, 4);
    assert_eq!(perk("TRAIT_PERK_MASTER_BUILDERS_3").house_wonder_production_modifier, 100);
    assert_eq!(perk("TRAIT_PERK_RITUALISTS_2").house_ritual_production_modifier, 50);
    assert_eq!(perk("TRAIT_PERK_LUMINARIES_1").house_great_people_rate_modifier, 50);
    assert_eq!(d.text.name(&perk("TRAIT_PERK_BATTLE_MAGES_3").base.description), "Battle-mages III");
}
