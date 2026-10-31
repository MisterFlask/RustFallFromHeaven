//! Data spot-checks for engagement combat (this game's own rules, GAMEOPTION_ENGAGEMENTS): the
//! roles (Units/CIV4CombatRoleInfos.xml), the stances (Units/CIV4StanceInfos.xml), the role each
//! unit type derives, the plots' slot changes and the ENGAGEMENT_* defines. Expected values are
//! the owner's spec ("Engagement combat", its tuning table) and docs/COMBAT_INVENTORY.md's answers.

use ffh_data::*;

fn d() -> &'static GameData {
    ffh_data::global()
}

fn role(name: &str) -> CombatRoleType {
    d().t::<CombatRoleType>(name)
}

fn stance(name: &str) -> &'static StanceInfo {
    let d = d();
    &d.stances[d.t::<StanceType>(name)]
}

/// The derived role of a unit type, as a type name ("-" for none).
fn unit_role(name: &str) -> &'static str {
    let d = d();
    d.unit_roles[d.t::<UnitType>(name)].map_or("-", |r| d.type_name(r))
}

fn roles(names: &[&str]) -> Vec<CombatRoleType> {
    names.iter().map(|n| role(n)).collect()
}

#[test]
fn the_option_is_visible_and_off_by_default() {
    let o = &d().game_options[GameOptionType::GAMEOPTION_ENGAGEMENTS];
    assert_eq!(o.base.type_, "GAMEOPTION_ENGAGEMENTS");
    assert!(o.visible && !o.default);
    assert_eq!(d().text.get(&o.base.description), "Engagements");
}

#[test]
fn the_six_roles() {
    use TargetClass::*;
    let d = d();
    let names: Vec<&str> = d.combat_roles.iter().map(|r| r.base.type_.as_str()).collect();
    assert_eq!(
        names,
        ["COMBATROLE_FRONTLINE", "COMBATROLE_SCREEN", "COMBATROLE_BACKLINE", "COMBATROLE_FLANKER", "COMBATROLE_ASSASSIN", "COMBATROLE_SUPPORT"]
    );
    // (type, A %, Dv %, max HP, back, screen, ignores shield, targets)
    let want: [(&str, i32, i32, i32, bool, bool, bool, &[TargetClass]); 6] = [
        ("COMBATROLE_FRONTLINE", 100, 100, 120, false, false, false, &[Front, Back]),
        ("COMBATROLE_SCREEN", 100, 110, 120, false, true, false, &[Front, Back]),
        ("COMBATROLE_BACKLINE", 100, 60, 80, true, false, false, &[Front, Back]),
        ("COMBATROLE_FLANKER", 110, 80, 100, false, false, true, &[Screen, Back, Front]),
        ("COMBATROLE_ASSASSIN", 120, 60, 60, false, false, true, &[Back, Front]),
        ("COMBATROLE_SUPPORT", 100, 50, 70, true, false, false, &[Front, Back]),
    ];
    for (name, a, dv, hp, back, screen, ignores, targets) in want {
        let r = &d.combat_roles[role(name)];
        assert_eq!(
            (r.attack_percent, r.defense_percent, r.max_hp, r.back, r.screen, r.ignores_shield, r.targets.as_slice()),
            (a, dv, hp, back, screen, ignores, targets),
            "{name}"
        );
        assert!(d.text.get(&r.base.description) != r.base.description, "{name}: no text");
        assert!(d.text.get(&r.base.help) != r.base.help, "{name}: no help text");
    }
    // Defaults by unit combat (the frontline also takes units with none), Marksman for assassins.
    let uc = |n: &str| d.t::<UnitCombatType>(n);
    let frontline = &d.combat_roles[role("COMBATROLE_FRONTLINE")];
    assert!(frontline.default);
    assert_eq!(frontline.default_unit_combats, ["UNITCOMBAT_MELEE", "UNITCOMBAT_ANIMAL", "UNITCOMBAT_BEAST", "UNITCOMBAT_RECON"].map(uc));
    assert_eq!(d.combat_roles.iter().filter(|r| r.default).count(), 1);
    let assassin = &d.combat_roles[role("COMBATROLE_ASSASSIN")];
    assert_eq!(assassin.default_unit_combats, [uc("UNITCOMBAT_RECON")]);
    assert_eq!(assassin.default_free_promotion, Some(d.t::<PromotionType>("PROMOTION_MARKSMAN")));
    assert_eq!(d.combat_roles[role("COMBATROLE_BACKLINE")].default_unit_combats, ["UNITCOMBAT_ARCHER", "UNITCOMBAT_SIEGE", "UNITCOMBAT_NAVAL"].map(uc));
    assert_eq!(d.combat_roles[role("COMBATROLE_FLANKER")].default_unit_combats, [uc("UNITCOMBAT_MOUNTED")]);
    assert_eq!(d.combat_roles[role("COMBATROLE_SUPPORT")].default_unit_combats, ["UNITCOMBAT_ADEPT", "UNITCOMBAT_DISCIPLE"].map(uc));
    // Q7: Formation makes a frontline unit a screen.
    let screen = &d.combat_roles[role("COMBATROLE_SCREEN")];
    assert!(screen.default_unit_combats.is_empty());
    assert_eq!(d.screen_role, Some((role("COMBATROLE_FRONTLINE"), d.t::<PromotionType>("PROMOTION_FORMATION"), role("COMBATROLE_SCREEN"))));
}

#[test]
fn the_five_stances() {
    let d = d();
    let names: Vec<&str> = d.stances.iter().map(|s| s.base.type_.as_str()).collect();
    assert_eq!(names, ["STANCE_BALANCED", "STANCE_SHIELD_WALL", "STANCE_RANGED_LINE", "STANCE_MOBILE", "STANCE_PRESERVE_VETERANS"]);
    assert_eq!(d.default_stance, d.t::<StanceType>("STANCE_BALANCED"));
    assert_eq!(d.stances.iter().filter(|s| s.default).count(), 1);
    let step = |r: &[&str], count: i32, percent: i32| StanceStep { roles: roles(r), count, percent };
    let front = ["COMBATROLE_FRONTLINE", "COMBATROLE_SCREEN"];
    let back = ["COMBATROLE_BACKLINE"];
    let flank = ["COMBATROLE_FLANKER", "COMBATROLE_ASSASSIN"];
    let balanced = vec![step(&["COMBATROLE_SCREEN"], 1, 0), step(&front, 0, 50), step(&back, 0, 100), step(&flank, 0, 100)];
    let fill = roles(&["COMBATROLE_FRONTLINE", "COMBATROLE_SCREEN", "COMBATROLE_FLANKER", "COMBATROLE_ASSASSIN", "COMBATROLE_BACKLINE", "COMBATROLE_SUPPORT"]);
    // (type, steps, max level, holds heroes)
    let want = [
        ("STANCE_BALANCED", balanced.clone(), -1, false),
        ("STANCE_SHIELD_WALL", vec![step(&front, 0, 100), step(&back, 0, 100)], -1, false),
        // the spec's STANCE_MIN_FRONT = 3
        ("STANCE_RANGED_LINE", vec![step(&front, 3, 0), step(&back, 0, 100)], -1, false),
        ("STANCE_MOBILE", vec![step(&flank, 0, 100), step(&front, 0, 100)], -1, false),
        // the spec's STANCE_VET_LEVEL = 4; heroes are held too
        ("STANCE_PRESERVE_VETERANS", balanced, 4, true),
    ];
    for (name, steps, max_level, hold_heroes) in want {
        let s = stance(name);
        assert_eq!((&s.steps, s.max_level, s.hold_heroes, &s.fill), (&steps, max_level, hold_heroes, &fill), "{name}");
        assert!(d.text.get(&s.base.description) != s.base.description, "{name}: no text");
        assert!(d.text.get(&s.base.help) != s.base.help, "{name}: no help text");
    }
}

#[test]
fn named_units_take_their_roles() {
    for (unit, want) in [
        ("UNIT_AXEMAN", "COMBATROLE_FRONTLINE"),
        ("UNIT_ARCHER", "COMBATROLE_BACKLINE"),
        ("UNIT_KNIGHT", "COMBATROLE_FLANKER"),
        ("UNIT_ASSASSIN", "COMBATROLE_ASSASSIN"),
        ("UNIT_ADEPT", "COMBATROLE_SUPPORT"),
        ("UNIT_PALADIN", "COMBATROLE_FRONTLINE"),
        ("UNIT_FIRE_ELEMENTAL", "COMBATROLE_FRONTLINE"),
        // Marksman is free but the unit is an archer, not recon (Q6): backline.
        ("UNIT_MARKSMAN", "COMBATROLE_BACKLINE"),
        // recon fight as frontline (spec)
        ("UNIT_SCOUT", "COMBATROLE_FRONTLINE"),
        ("UNIT_CATAPULT", "COMBATROLE_BACKLINE"),
        // more of section 3's table
        ("UNIT_WOLF", "COMBATROLE_FRONTLINE"),
        ("UNIT_HILL_GIANT", "COMBATROLE_FRONTLINE"),
        ("UNIT_HORSE_ARCHER", "COMBATROLE_FLANKER"),
        ("UNIT_SHADOW", "COMBATROLE_ASSASSIN"),
        ("UNIT_RATHUS", "COMBATROLE_ASSASSIN"),
        ("UNIT_PRIEST_OF_LEAVES", "COMBATROLE_SUPPORT"),
        ("UNIT_DRUID", "COMBATROLE_SUPPORT"),
        ("UNIT_IRON_GOLEM", "COMBATROLE_FRONTLINE"),
        // the only land unit with UNITCOMBAT_NAVAL
        ("UNIT_AIRSHIP", "COMBATROLE_BACKLINE"),
        // defends only, immobile: still frontline
        ("UNIT_GUARDIAN_VINES", "COMBATROLE_FRONTLINE"),
        ("UNIT_MUD_GOLEM", "COMBATROLE_FRONTLINE"),
        // typed strength only (lightning 3)
        ("UNIT_LIGHTNING_ELEMENTAL", "COMBATROLE_FRONTLINE"),
        // missiles have a role (their strike targets by it); the rules never commit them
        ("UNIT_FIREBALL", "COMBATROLE_FRONTLINE"),
        // Types without strength have a role too: a unit's strength is its own, and promotions
        // add to it (a Worker that takes the Godslayer is a 3/3 unit, and it defends).
        ("UNIT_WORKER", "COMBATROLE_FRONTLINE"),
        ("UNIT_SETTLER", "COMBATROLE_FRONTLINE"),
        ("UNIT_COMMANDER", "COMBATROLE_FRONTLINE"),
        ("UNIT_CORLINDALE", "COMBATROLE_SUPPORT"),
        ("EQUIPMENT_GOLDEN_HAMMER", "COMBATROLE_FRONTLINE"),
        ("UNIT_BRIGIT_HELD", "COMBATROLE_FRONTLINE"),
        // no role: sea and air units
        ("UNIT_TRIREME", "-"),
        ("UNIT_KRAKEN", "-"),
        ("UNIT_HAWK", "-"),
    ] {
        assert_eq!(unit_role(unit), want, "{unit}");
    }
}

#[test]
fn the_religions_fighters_are_frontline_and_every_assassin_has_marksman() {
    let d = d();
    // Section 3: seven disciples that never cast are frontline by their CombatRole; nothing else
    // sets one, and no unit sets the iAttack / iDefense / iHP overrides yet.
    let mut overridden: Vec<&str> = d.units.iter().filter(|u| u.combat_role.is_some()).map(|u| u.base.type_.as_str()).collect();
    overridden.sort_unstable();
    assert_eq!(
        overridden,
        ["UNIT_CRUSADER", "UNIT_EIDOLON", "UNIT_EINHERJAR", "UNIT_MONK", "UNIT_PALADIN", "UNIT_PARAMANDER", "UNIT_TAR_DEMON"]
    );
    let channeling = d.t::<PromotionType>("PROMOTION_CHANNELING1");
    let divine = d.t::<PromotionType>("PROMOTION_DIVINE");
    let disciple = d.t::<UnitCombatType>("UNITCOMBAT_DISCIPLE");
    for u in d.units.iter().filter(|u| u.combat_role.is_some()) {
        assert_eq!(u.combat_role, Some(role("COMBATROLE_FRONTLINE")), "{}", u.base.type_);
        assert_eq!(u.unit_combat_type, Some(disciple), "{}", u.base.type_);
        assert!(!u.free_promotions[channeling] && !u.free_promotions[divine], "{} casts", u.base.type_);
    }
    assert!(d.units.iter().all(|u| u.attack_override == 0 && u.defense_override == 0 && u.hp_override == 0));
    // Q6: the assassins are exactly the recon units with Marksman free.
    let mut assassins: Vec<&str> =
        d.units.keys().filter(|&u| d.unit_roles[u] == Some(role("COMBATROLE_ASSASSIN"))).map(|u| d.type_name(u)).collect();
    assassins.sort_unstable();
    assert_eq!(
        assassins,
        [
            "UNIT_ALAZKAN", "UNIT_ANGEL_OF_DEATH", "UNIT_ASSASSIN", "UNIT_CHANTER", "UNIT_COURTESAN", "UNIT_DWARVEN_SHADOW",
            "UNIT_GHOST", "UNIT_LIZARDMAN_ASSASSIN", "UNIT_RATHUS", "UNIT_SHADOW", "UNIT_TASKMASTER",
        ]
    );
}

#[test]
fn every_land_type_has_a_role() {
    let d = d();
    let count = |want: Option<&str>| d.unit_roles.iter().filter(|r| r.map(|r| d.type_name(r)) == want).count();
    // (role, unit types) at the time of writing; the sum is every unit type.
    let want = [
        (Some("COMBATROLE_FRONTLINE"), 189),
        (Some("COMBATROLE_SCREEN"), 0),
        (Some("COMBATROLE_BACKLINE"), 21),
        (Some("COMBATROLE_FLANKER"), 38),
        (Some("COMBATROLE_ASSASSIN"), 11),
        (Some("COMBATROLE_SUPPORT"), 47),
        (None, 19),
    ];
    for (r, n) in want {
        assert_eq!(count(r), n, "{r:?}");
    }
    assert_eq!(want.iter().map(|w| w.1).sum::<usize>(), d.units.len());
    // Whether a unit fights is its own strength, not its type's (promotions add to it), so every
    // land and immobile type has a role, with or without strength; sea and air types have none.
    for (u, info) in d.units.iter_enumerated() {
        let land = matches!(info.domain_type, Some(DomainType::DOMAIN_LAND) | Some(DomainType::DOMAIN_IMMOBILE));
        assert_eq!(d.unit_roles[u].is_some(), land, "{}", info.base.type_);
    }
}

#[test]
fn plots_change_the_slots() {
    let d = d();
    let t = |n: &str| d.t::<TerrainType>(n);
    let f = |n: &str| Some(d.t::<FeatureType>(n));
    for n in ["TERRAIN_GRASS", "TERRAIN_PLAINS", "TERRAIN_DESERT", "TERRAIN_TUNDRA", "TERRAIN_SNOW", "TERRAIN_BURNING_SANDS", "TERRAIN_FIELDS_OF_PERDITION"] {
        assert_eq!(d.terrains[t(n)].engagement_slots_change, 2, "{n}");
    }
    assert_eq!(d.terrains.iter().filter(|x| x.engagement_slots_change != 0).count(), 7);
    for n in ["FEATURE_JUNGLE", "FEATURE_FOREST", "FEATURE_FOREST_ANCIENT", "FEATURE_FOREST_NEW"] {
        assert_eq!(d.features[f(n).unwrap()].engagement_slots_change, -2, "{n}");
    }
    assert_eq!(d.features.iter().filter(|x| x.engagement_slots_change != 0).count(), 4);
    // Each side's slots before the minimum, the city and the river (plan section 2.4).
    let s = |terrain: &str, feature: Option<FeatureType>, hills: bool| {
        d.defines.engagement_slots_base + d.engagement_slots_change(t(terrain), feature, hills)
    };
    assert_eq!(s("TERRAIN_GRASS", None, false), 10, "open grassland");
    assert_eq!(s("TERRAIN_GRASS", f("FEATURE_FOREST"), false), 6, "forest");
    assert_eq!(s("TERRAIN_GRASS", f("FEATURE_JUNGLE"), false), 6, "jungle");
    assert_eq!(s("TERRAIN_PLAINS", None, true), 6, "hills");
    assert_eq!(s("TERRAIN_PLAINS", f("FEATURE_FOREST"), true), 4, "forested hills");
    assert_eq!(s("TERRAIN_MARSH", None, false), 8, "marsh");
    assert_eq!(s("TERRAIN_BROKEN_LANDS", None, false), 8, "broken lands");
    // a feature without a change leaves the terrain's
    assert_eq!(s("TERRAIN_DESERT", f("FEATURE_FLOOD_PLAINS"), false), 10, "flood plains");
    assert_eq!(s("TERRAIN_GRASS", f("FEATURE_SCRUB"), false), 10, "scrub");
}

#[test]
fn withered_and_puppet_lower_defence_too() {
    // Q1: the defence percentage applies to Dv in an engagement; Withered and Puppet, which FFH2
    // gave only an attack penalty, get one on defence as well.
    let d = d();
    for (name, pct) in [("PROMOTION_WITHERED", -10), ("PROMOTION_PUPPET", -20)] {
        let p = &d.promotions[d.t::<PromotionType>(name)];
        assert_eq!((p.combat_percent, p.combat_percent_defense), (pct, pct), "{name}");
    }
}

#[test]
fn engagement_defines() {
    let x = &d().defines;
    assert_eq!((x.engagement_slots_base, x.engagement_slots_min, x.engagement_slots_hills, x.engagement_slots_river), (8, 3, -2, 2));
    assert_eq!((x.engagement_slots_defense_divisor, x.engagement_summon_extra_slots), (25, 2));
    assert_eq!((x.engagement_base_damage, x.engagement_damage_rand_min, x.engagement_damage_rand_max), (20, 80, 120));
    assert_eq!((x.engagement_max_rounds, x.engagement_withdraw_hp_percent, x.engagement_auto_min_hp_percent), (40, 40, 50));
    assert_eq!((x.engagement_recon_reveal_range, x.engagement_reports_kept), (2, 128));
}
