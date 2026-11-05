//! Data spot-checks for engagement combat (this game's own rules, docs/ENGAGEMENTS.md): the
//! roles of formation battles (combat_roles.jsonc), the role each unit type derives,
//! the large units, the casters'
//! damage spells, the plots' slot changes and the ENGAGEMENT_* defines. Expected values are the
//! owner's (the formation battles spec, revision 4: docs/ENGAGEMENTS.md) and
//! docs/COMBAT_INVENTORY.md's answers.

use ffh_data::*;

fn d() -> &'static GameData {
    ffh_data::global()
}

fn role(name: &str) -> CombatRoleType {
    d().t::<CombatRoleType>(name)
}

/// The derived role of a unit type, as a type name ("-" for none).
fn unit_role(name: &str) -> &'static str {
    let d = d();
    d.unit_roles[d.t::<UnitType>(name)].map_or("-", |r| d.type_name(r))
}

/// A row: `n` actions of `kind`.
fn row(kind: ActionKind, n: i32) -> RowActions {
    RowActions { kind, count: n }
}

#[test]
fn the_eight_roles() {
    use ActionKind::*;
    let d = d();
    let names: Vec<&str> = d.combat_roles.iter().map(|r| r.base.type_.as_str()).collect();
    assert_eq!(
        names,
        [
            "COMBATROLE_FRONTLINE", "COMBATROLE_SCREEN", "COMBATROLE_FLANKER", "COMBATROLE_ASSASSIN", "COMBATROLE_ARCHER", "COMBATROLE_SIEGE",
            "COMBATROLE_ARCANE", "COMBATROLE_DIVINE",
        ]
    );
    // The spec's table: (type, anti-flank, rows front to rear, home row, auto-pick place); the
    // numbers are each unit's own (docs/UNIT_STATS.md).
    type Want = (&'static str, bool, [RowActions; 3], usize, i32);
    let want: [Want; 8] = [
        ("COMBATROLE_FRONTLINE", false, [row(Strike, 2), row(Strike, 1), row(Strike, 1)], 0, 2),
        ("COMBATROLE_SCREEN", true, [row(Strike, 2), row(Strike, 1), row(Strike, 1)], 0, 1),
        ("COMBATROLE_FLANKER", false, [row(Charge, 2), row(Strike, 1), row(Strike, 1)], 0, 4),
        ("COMBATROLE_ASSASSIN", false, [row(Stab, 1), row(Stab, 2), row(Stab, 1)], 1, 8),
        ("COMBATROLE_ARCHER", false, [row(Shoot, 1), row(Shoot, 2), row(Shoot, 2)], 1, 3),
        ("COMBATROLE_SIEGE", false, [row(None, 0), row(Bombard, 1), row(Bombard, 2)], 2, 7),
        ("COMBATROLE_ARCANE", false, [row(Strike, 1), row(Spell, 1), row(Spell, 2)], 2, 5),
        ("COMBATROLE_DIVINE", false, [row(Strike, 1), row(Heal, 1), row(Heal, 2)], 1, 6),
    ];
    for (name, anti_flank, rows, home, pick) in want {
        let r = &d.combat_roles[role(name)];
        assert_eq!((r.anti_flank, r.rows, r.home_row, r.auto_pick), (anti_flank, rows, home, pick), "{name}");
        assert!(d.text.get(&r.base.description) != r.base.description, "{name}: no text");
        assert!(d.text.get(&r.base.help) != r.base.help, "{name}: no help text");
        // Only the flanker charges.
        assert_eq!(r.flanks(), name == "COMBATROLE_FLANKER", "{name}");
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
    assert_eq!(d.combat_roles[role("COMBATROLE_ARCHER")].default_unit_combats, ["UNITCOMBAT_ARCHER", "UNITCOMBAT_NAVAL"].map(uc));
    assert_eq!(d.combat_roles[role("COMBATROLE_SIEGE")].default_unit_combats, [uc("UNITCOMBAT_SIEGE")]);
    assert_eq!(d.combat_roles[role("COMBATROLE_FLANKER")].default_unit_combats, [uc("UNITCOMBAT_MOUNTED")]);
    assert_eq!(d.combat_roles[role("COMBATROLE_ARCANE")].default_unit_combats, [uc("UNITCOMBAT_ADEPT")]);
    assert_eq!(d.combat_roles[role("COMBATROLE_DIVINE")].default_unit_combats, [uc("UNITCOMBAT_DISCIPLE")]);
    // Q7: Formation makes a frontline unit a screen.
    let screen = &d.combat_roles[role("COMBATROLE_SCREEN")];
    assert!(screen.default_unit_combats.is_empty());
    assert_eq!(d.screen_role, Some((role("COMBATROLE_FRONTLINE"), d.t::<PromotionType>("PROMOTION_FORMATION"), role("COMBATROLE_SCREEN"))));
}

#[test]
fn tier_4_units_and_heroes_act_once_more_in_their_best_row() {
    use ActionKind::*;
    let d = d();
    // (role, best row, its actions with the extra one)
    for (name, best, rows) in [
        ("COMBATROLE_FRONTLINE", 0, [row(Strike, 3), row(Strike, 1), row(Strike, 1)]),
        ("COMBATROLE_SCREEN", 0, [row(Strike, 3), row(Strike, 1), row(Strike, 1)]),
        ("COMBATROLE_FLANKER", 0, [row(Charge, 3), row(Strike, 1), row(Strike, 1)]),
        ("COMBATROLE_ASSASSIN", 1, [row(Stab, 1), row(Stab, 3), row(Stab, 1)]),
        // two rows with two: the first such
        ("COMBATROLE_ARCHER", 1, [row(Shoot, 1), row(Shoot, 3), row(Shoot, 2)]),
        // the front row stays without an action
        ("COMBATROLE_SIEGE", 2, [row(None, 0), row(Bombard, 1), row(Bombard, 3)]),
        ("COMBATROLE_ARCANE", 2, [row(Strike, 1), row(Spell, 1), row(Spell, 3)]),
        ("COMBATROLE_DIVINE", 2, [row(Strike, 1), row(Heal, 1), row(Heal, 3)]),
    ] {
        let r = &d.combat_roles[role(name)];
        assert_eq!(r.best_row(), best, "{name}");
        assert_eq!(r.actions(d.defines.engagement_extra_actions), rows, "{name}");
        assert_eq!(r.actions(0), r.rows, "{name}");
    }
}

#[test]
fn the_kinds() {
    use ActionKind::*;
    for k in ActionKind::ALL {
        assert_eq!(ActionKind::parse(k.token()), Some(k));
    }
    assert_eq!(ActionKind::parse("FRONT"), Option::None);
    let set = |f: fn(ActionKind) -> bool| ActionKind::ALL.into_iter().filter(|&k| f(k)).collect::<Vec<_>>();
    assert_eq!(set(ActionKind::physical), [Strike, Charge, Stab, Shoot, Bombard]);
    assert_eq!(set(ActionKind::can_crit), [Strike, Charge, Stab, Shoot]);
    assert_eq!(set(ActionKind::melee), [Strike, Charge]);
    assert_eq!(set(ActionKind::ranged), [Shoot, Spell, Bombard]);
}

#[test]
fn the_large_units() {
    let d = d();
    let mut large: Vec<&str> = d.units.iter().filter(|u| u.large).map(|u| u.base.type_.as_str()).collect();
    large.sort_unstable();
    // The spec's (Hill Giant, the elephants, the dragons, every golem, the Treant) and the units of
    // that size class: the giant heroes Gurid, Margalard and Wilboman (a frost giant), Barnaxus (a
    // golem), the War Machine, the Earth Elemental (a golem of rock) and the War Tortoise.
    assert_eq!(
        large,
        [
            "UNIT_ABASHI", "UNIT_ACHERON", "UNIT_BARNAXUS", "UNIT_BONE_GOLEM", "UNIT_CLOCKWORK_GOLEM", "UNIT_DRIFA", "UNIT_EARTH_ELEMENTAL",
            "UNIT_ELEPHANT", "UNIT_EURABATRES", "UNIT_FLESH_GOLEM", "UNIT_GURID", "UNIT_HILL_GIANT", "UNIT_IRON_GOLEM", "UNIT_MARGALARD",
            "UNIT_MITHRIL_GOLEM", "UNIT_MUD_GOLEM", "UNIT_NULLSTONE_GOLEM", "UNIT_TREANT", "UNIT_WAR_ELEPHANT", "UNIT_WAR_MACHINE",
            "UNIT_WAR_TORTOISE", "UNIT_WILBOMAN", "UNIT_WOOD_GOLEM",
        ]
    );
    // Every golem, every elephant; all of them fight on land.
    for u in d.units.iter() {
        let t = u.base.type_.as_str();
        if t.ends_with("_GOLEM") || t.contains("ELEPHANT") {
            assert!(u.large, "{t}");
        }
        if u.large {
            assert_eq!(u.domain_type, Some(DomainType::DOMAIN_LAND), "{t}");
        }
    }
}

#[test]
fn casters_cast_their_strongest_damage_spell() {
    let d = d();
    let p = |n: &str| d.t::<PromotionType>(n);
    let spell = |unit: &str, held: &[&str]| {
        let held: Vec<PromotionType> = held.iter().map(|n| p(n)).collect();
        d.engage_damage_spell(d.t::<UnitType>(unit), |x| held.contains(&x)).map(|s| d.type_name(s))
    };
    // No damage spell: the magic bolt.
    assert_eq!(spell("UNIT_ADEPT", &[]), None);
    assert_eq!(spell("UNIT_ADEPT", &["PROMOTION_AIR1"]), None);
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_AIR2"]), Some("SPELL_MAELSTROM"));
    // The strongest: Snowfall (40) over Maelstrom (15).
    assert_eq!(spell("UNIT_ARCHMAGE", &["PROMOTION_AIR2", "PROMOTION_ICE3"]), Some("SPELL_SNOWFALL"));
    // Both promotions of a two-promotion spell: Ring of Flames (15) needs Channeling II and Divine.
    assert_eq!(spell("UNIT_PRIEST_OF_THE_ORDER", &["PROMOTION_CHANNELING2"]), None);
    assert_eq!(spell("UNIT_PRIEST_OF_THE_ORDER", &["PROMOTION_CHANNELING2", "PROMOTION_DIVINE"]), Some("SPELL_RING_OF_FLAMES"));
    // Equal damage: the lower id (Maelstrom before Ring of Flames, both 15).
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_AIR2", "PROMOTION_CHANNELING2", "PROMOTION_DIVINE"]), Some("SPELL_MAELSTROM"));
    // A unit-class spell: Mary's Contagion, Auric Ascended's greater Snowfall.
    assert_eq!(spell("UNIT_MARY", &[]), Some("SPELL_CONTAGION"));
    assert_eq!(spell("UNIT_AURIC_ASCENDED", &[]), Some("SPELL_SNOWFALL_GREATOR"));
}

#[test]
fn named_units_take_their_roles() {
    for (unit, want) in [
        ("UNIT_AXEMAN", "COMBATROLE_FRONTLINE"),
        ("UNIT_ARCHER", "COMBATROLE_ARCHER"),
        ("UNIT_KNIGHT", "COMBATROLE_FLANKER"),
        ("UNIT_ASSASSIN", "COMBATROLE_ASSASSIN"),
        ("UNIT_ADEPT", "COMBATROLE_ARCANE"),
        ("UNIT_PALADIN", "COMBATROLE_FRONTLINE"),
        ("UNIT_FIRE_ELEMENTAL", "COMBATROLE_FRONTLINE"),
        // Marksman is free but the unit is an archer, not recon (Q6): an archer.
        ("UNIT_MARKSMAN", "COMBATROLE_ARCHER"),
        // recon fight as frontline (spec)
        ("UNIT_SCOUT", "COMBATROLE_FRONTLINE"),
        ("UNIT_CATAPULT", "COMBATROLE_SIEGE"),
        ("UNIT_TREBUCHET", "COMBATROLE_SIEGE"),
        ("UNIT_BATTERING_RAM", "COMBATROLE_SIEGE"),
        // more of section 3's table
        ("UNIT_WOLF", "COMBATROLE_FRONTLINE"),
        ("UNIT_HILL_GIANT", "COMBATROLE_FRONTLINE"),
        ("UNIT_WAR_ELEPHANT", "COMBATROLE_FLANKER"),
        ("UNIT_HORSE_ARCHER", "COMBATROLE_FLANKER"),
        ("UNIT_SHADOW", "COMBATROLE_ASSASSIN"),
        ("UNIT_RATHUS", "COMBATROLE_ASSASSIN"),
        ("UNIT_PRIEST_OF_LEAVES", "COMBATROLE_DIVINE"),
        ("UNIT_DRUID", "COMBATROLE_DIVINE"),
        ("UNIT_MAGE", "COMBATROLE_ARCANE"),
        ("UNIT_IRON_GOLEM", "COMBATROLE_FRONTLINE"),
        // the only land unit with UNITCOMBAT_NAVAL
        ("UNIT_AIRSHIP", "COMBATROLE_ARCHER"),
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
        ("UNIT_CORLINDALE", "COMBATROLE_ARCANE"),
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
    // UNIT_SENTINEL (docs/SIEGES.md: never trained; a city's Sentinels fight with its values) is
    // the one other unit that sets them.
    let not_sentinel = |u: &&UnitInfo| u.base.type_ != "UNIT_SENTINEL";
    let mut overridden: Vec<&str> = d.units.iter().filter(not_sentinel).filter(|u| u.combat_role.is_some()).map(|u| u.base.type_.as_str()).collect();
    overridden.sort_unstable();
    assert_eq!(
        overridden,
        ["UNIT_CRUSADER", "UNIT_EIDOLON", "UNIT_EINHERJAR", "UNIT_MONK", "UNIT_PALADIN", "UNIT_PARAMANDER", "UNIT_TAR_DEMON"]
    );
    let channeling = d.t::<PromotionType>("PROMOTION_CHANNELING1");
    let divine = d.t::<PromotionType>("PROMOTION_DIVINE");
    let disciple = d.t::<UnitCombatType>("UNITCOMBAT_DISCIPLE");
    for u in d.units.iter().filter(not_sentinel).filter(|u| u.combat_role.is_some()) {
        assert_eq!(u.combat_role, Some(role("COMBATROLE_FRONTLINE")), "{}", u.base.type_);
        assert_eq!(u.unit_combat_type, Some(disciple), "{}", u.base.type_);
        assert!(!u.free_promotions[channeling] && !u.free_promotions[divine], "{} casts", u.base.type_);
    }
    // Every type that fights land battles has a stat sheet (docs/UNIT_STATS.md), and ships none.
    assert!(d.units.keys().all(|u| d.unit_roles[u].is_some() == d.units[u].stats.is_some()), "a role without a sheet or a sheet without a role");
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
        (Some("COMBATROLE_FRONTLINE"), 190),
        (Some("COMBATROLE_SCREEN"), 0),
        (Some("COMBATROLE_FLANKER"), 38),
        (Some("COMBATROLE_ASSASSIN"), 11),
        (Some("COMBATROLE_ARCHER"), 16),
        (Some("COMBATROLE_SIEGE"), 5),
        (Some("COMBATROLE_ARCANE"), 18),
        (Some("COMBATROLE_DIVINE"), 29),
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
        let c = |s: StatType| p.stat_changes[s.idx()].percent;
        assert_eq!((c(StatType::Attack), c(StatType::Defense), c(StatType::MagicDefense)), (pct, pct, pct), "{name}");
    }
}

#[test]
fn engagement_defines() {
    let x = &d().defines;
    assert_eq!((x.engagement_slots_base, x.engagement_slots_min, x.engagement_slots_hills, x.engagement_slots_river), (8, 3, -2, 2));
    assert_eq!(x.engagement_summon_extra_slots, 2);
    // Sieges (docs/SIEGES.md).
    assert_eq!((x.sentinel_repair_percent, x.siege_bombard_floor_percent), (10, 50));
    // The owner's choice: an even-strength hit takes 20 HP (six kill a frontline unit).
    assert_eq!((x.engagement_base_damage, x.engagement_damage_rand_min, x.engagement_damage_rand_max), (40, 80, 120));
    assert_eq!((x.engagement_rounds, x.engagement_first_strike_speed), (3, 2));
    assert_eq!((x.engagement_extra_action_tier, x.engagement_extra_actions), (4, 1));
    assert_eq!((x.engagement_shoot_miss_percent, x.engagement_bombard_miss_percent), (10, 15));
    assert_eq!((x.engagement_crit_damage_percent, x.engagement_charge_bonus), (150, 100));
    assert_eq!((x.engagement_splash_percent, x.engagement_bolt_percent, x.engagement_heal_percent), (50, 60, 50));
    assert_eq!((x.engagement_city_attacker_shot_percent, x.engagement_forest_ranged_percent), (50, 50));
    assert_eq!((x.engagement_hills_attacker_percent, x.engagement_hills_defender_ranged_percent), (75, 125));
    assert_eq!(x.engagement_auto_min_hp_percent, 50);
    assert_eq!((x.engagement_recon_reveal_range, x.engagement_reports_kept, x.engagement_report_logs_kept), (2, 128, 32));
    // The round model's own constants are no longer defines.
    for gone in ["ENGAGEMENT_MAX_ROUNDS", "ENGAGEMENT_WITHDRAW_HP_PERCENT"] {
        assert!(!d().define_map.map.contains_key(gone), "{gone}");
    }
}
