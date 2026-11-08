//! Data spot-checks for engagement combat (this game's own rules, docs/ENGAGEMENTS.md): what each
//! unit type does from each row and its rank preference (`engage_rows`, `engage_row_order` in
//! units.jsonc; there are no roles since 2026-11-06), the large units, the casters' damage spells,
//! the plots' slot changes and the ENGAGEMENT_* defines. Expected values are the owner's (the
//! formation battles spec, revision 4: docs/ENGAGEMENTS.md) and docs/COMBAT_INVENTORY.md's answers.

use ffh_data::*;

fn d() -> &'static GameData {
    ffh_data::global()
}

/// A unit type's kind (the action of the row it prefers), as its token ("-" for a type without
/// rows: sea and air units).
fn unit_kind(name: &str) -> &'static str {
    let d = d();
    let t = d.t::<UnitType>(name);
    if d.units[t].engage_rows.is_none() {
        return "-";
    }
    d.engage_kind(t).token()
}

/// A row: `n` actions of `kind`.
fn row(kind: ActionKind, n: i32) -> RowActions {
    RowActions { kind, count: n }
}

#[test]
fn the_rows_of_the_seven_kinds() {
    use ActionKind::*;
    let d = d();
    // The spec's rows (front to rear) and rank preferences, one unit of each kind; the numbers
    // are each unit's own (docs/UNIT_STATS.md).
    type Want = (&'static str, [RowActions; 3], [usize; 3], ActionKind);
    let want: [Want; 7] = [
        ("UNIT_AXEMAN", [row(Strike, 2), row(Strike, 1), row(Strike, 1)], [0, 1, 2], Strike),
        ("UNIT_KNIGHT", [row(Charge, 2), row(Strike, 1), row(Strike, 1)], [0, 1, 2], Charge),
        ("UNIT_ASSASSIN", [row(Stab, 1), row(Stab, 2), row(Stab, 1)], [1, 2, 0], Stab),
        ("UNIT_ARCHER", [row(Shoot, 1), row(Shoot, 2), row(Shoot, 2)], [1, 2, 0], Shoot),
        ("UNIT_CATAPULT", [row(None, 0), row(Bombard, 1), row(Bombard, 2)], [2, 1, 0], Bombard),
        ("UNIT_ADEPT", [row(Strike, 1), row(Spell, 1), row(Spell, 2)], [2, 1, 0], Spell),
        ("UNIT_PRIEST_OF_LEAVES", [row(Strike, 1), row(Heal, 2), row(Heal, 2)], [1, 2, 0], Heal),
    ];
    for (name, rows, order, kind) in want {
        let t = d.t::<UnitType>(name);
        assert_eq!((d.engage_rows(t), d.engage_row_order(t), d.engage_kind(t)), (rows, order, kind), "{name}");
        // Only the chargers flank.
        assert_eq!(d.engage_flanks(t), kind == Charge, "{name}");
    }
    // Q7: Formation is the one promotion that makes a striker anti-flanking.
    let anti: Vec<&str> = d.promotions.iter().filter(|p| p.anti_flank).map(|p| p.base.type_.as_str()).collect();
    assert_eq!(anti, ["PROMOTION_FORMATION"]);
}

#[test]
fn tier_4_units_and_heroes_act_once_more_in_their_best_row() {
    use ActionKind::*;
    let d = d();
    // (unit, best row, its actions with the extra one); none of them tier 4, which has it always
    assert_eq!(d.engage_extra_actions(d.t::<UnitType>("UNIT_KNIGHT"), false), 1);
    for (name, best, rows) in [
        ("UNIT_AXEMAN", 0, [row(Strike, 3), row(Strike, 1), row(Strike, 1)]),
        ("UNIT_HORSEMAN", 0, [row(Charge, 3), row(Strike, 1), row(Strike, 1)]),
        ("UNIT_ASSASSIN", 1, [row(Stab, 1), row(Stab, 3), row(Stab, 1)]),
        // two rows with two: the first such
        ("UNIT_ARCHER", 1, [row(Shoot, 1), row(Shoot, 3), row(Shoot, 2)]),
        // the front row stays without an action
        ("UNIT_CATAPULT", 2, [row(None, 0), row(Bombard, 1), row(Bombard, 3)]),
        ("UNIT_ADEPT", 2, [row(Strike, 1), row(Spell, 1), row(Spell, 3)]),
        // two rows with two: the first such (the healers' rows, docs/UNIT_ROLES.md)
        ("UNIT_PRIEST_OF_LEAVES", 1, [row(Strike, 1), row(Heal, 3), row(Heal, 2)]),
    ] {
        let t = d.t::<UnitType>(name);
        assert_eq!(d.engage_best_row(t), best, "{name}");
        assert_eq!(d.engage_actions(t, true), rows, "{name}");
        assert_eq!(d.engage_actions(t, false), d.engage_rows(t), "{name}");
    }
}

/// The battle levers' spot checks (docs/UNIT_ROLES.md): patterns, type multipliers, armor,
/// piercing, heal power, target preference, round damage, Formation's.
#[test]
fn the_unit_roles() {
    let d = d();
    let u = |n: &str| &d.units[d.t::<UnitType>(n)];
    let c = |n: &str| d.t::<UnitCombatType>(n);
    assert_eq!(u("UNIT_LONGBOWMAN").engage_patterns, [AttackPattern::Single, AttackPattern::Volley, AttackPattern::Volley]);
    assert_eq!(u("UNIT_KNIGHT").engage_patterns[0], AttackPattern::Trample);
    assert_eq!(u("UNIT_BERSERKER").engage_patterns[0], AttackPattern::Cleave);
    assert_eq!(u("UNIT_ADEPT").engage_patterns[2], AttackPattern::Pierce);
    assert_eq!(u("UNIT_DISCIPLE_THE_ORDER").engage_patterns[2], AttackPattern::Sweep);
    assert_eq!(u("UNIT_HUNTER").damage_vs[c("UNITCOMBAT_MOUNTED")], 75);
    assert_eq!(u("UNIT_CHAMPION").resist_vs[c("UNITCOMBAT_MOUNTED")], 40);
    assert_eq!((u("UNIT_CHAMPION").armor, u("UNIT_PHALANX").armor), (3, 3));
    assert!(u("UNIT_CROSSBOWMAN").armor_piercing && !u("UNIT_ARCHER").armor_piercing);
    assert_eq!((u("UNIT_DISCIPLE_THE_ORDER").heal_percent, u("UNIT_AXEMAN").heal_percent), (Some(60), None));
    assert_eq!(u("UNIT_ASSASSIN").target_preference, [c("UNITCOMBAT_ADEPT"), c("UNITCOMBAT_DISCIPLE")]);
    assert_eq!((u("UNIT_BERSERKER").round_percent, u("UNIT_AXEMAN").round_percent), ([110, 130, 150], [100; 3]));
    assert_eq!((u("UNIT_CATAPULT").damage_vs_walls, u("UNIT_CATAPULT").damage_vs_large), (100, 100));
    let formation = &d.promotions[d.t::<PromotionType>("PROMOTION_FORMATION")];
    assert_eq!((formation.damage_vs[c("UNITCOMBAT_MOUNTED")], formation.resist_vs[c("UNITCOMBAT_MOUNTED")]), (50, 50));
    assert!(u("UNIT_PHALANX").free_promotions[d.t::<PromotionType>("PROMOTION_FORMATION")]);
    for p in AttackPattern::ALL {
        assert_eq!(AttackPattern::parse(p.token()), Some(p));
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
    // The game's own unit types (the cards' come after them, docs/CARDS.md).
    let own = d.units.len() - ffh_data::card_entry_count("units");
    let mut large: Vec<&str> = d.units.iter().take(own).filter(|u| u.large).map(|u| u.base.type_.as_str()).collect();
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
        d.engage_damage_spell(d.t::<UnitType>(unit), |x| held.contains(&x), None).map(|s| d.type_name(s))
    };
    let spell_of = |unit: &str, held: &[&str], religion: &str| {
        let held: Vec<PromotionType> = held.iter().map(|n| p(n)).collect();
        d.engage_damage_spell(d.t::<UnitType>(unit), |x| held.contains(&x), Some(d.t::<ReligionType>(religion))).map(|s| d.type_name(s))
    };
    // No damage spell: the magic bolt (Enchantment I's spell in a battle is a charm).
    assert_eq!(spell("UNIT_ADEPT", &[]), None);
    assert_eq!(spell("UNIT_ADEPT", &["PROMOTION_ENCHANTMENT1"]), None);
    assert_eq!(spell("UNIT_ADEPT", &["PROMOTION_AIR1"]), Some("SPELL_SHOCK"));
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_AIR2"]), Some("SPELL_MAELSTROM"));
    // A battle-only spell: Fire II's Fireball (damage 0, so a damage spell comes first).
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_FIRE2"]), Some("SPELL_FIREBALL"));
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_FIRE2", "PROMOTION_AIR2"]), Some("SPELL_MAELSTROM"));
    // The most output (power x what its pattern adds): Fireball's splash (100 x 200%) over
    // Firebolt (120, one target); Sunburst's volley (100 x 200%) over Flare (120).
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_FIRE1", "PROMOTION_FIRE2"]), Some("SPELL_FIREBALL"));
    assert_eq!(spell("UNIT_MAGE", &["PROMOTION_SUN1", "PROMOTION_SUN2"]), Some("SPELL_SUNBURST"));
    // Equal output: the damage, Snowfall (40) over Maelstrom (15).
    assert_eq!(spell("UNIT_ARCHMAGE", &["PROMOTION_AIR2", "PROMOTION_ICE3"]), Some("SPELL_SNOWFALL"));
    // Both promotions of a two-promotion spell and its religion: Ring of Flames (15) needs
    // Channeling II, Divine and the Ashen Veil.
    let ring = ["PROMOTION_CHANNELING2", "PROMOTION_DIVINE"];
    assert_eq!(spell_of("UNIT_PRIEST_OF_THE_VEIL", &["PROMOTION_CHANNELING2"], "RELIGION_THE_ASHEN_VEIL"), None);
    assert_eq!(spell_of("UNIT_PRIEST_OF_THE_VEIL", &ring, "RELIGION_THE_ASHEN_VEIL"), Some("SPELL_RING_OF_FLAMES"));
    assert_eq!(spell_of("UNIT_PRIEST_OF_THE_ORDER", &ring, "RELIGION_THE_ORDER"), None);
    assert_eq!(spell("UNIT_PRIEST_OF_THE_ORDER", &ring), None);
    // Equal damage: the lower id (Maelstrom before Ring of Flames, both 15).
    assert_eq!(spell_of("UNIT_MAGE", &["PROMOTION_AIR2", "PROMOTION_CHANNELING2", "PROMOTION_DIVINE"], "RELIGION_THE_ASHEN_VEIL"), Some("SPELL_MAELSTROM"));
    // A unit-class spell: Mary's Contagion, Auric Ascended's greater Snowfall.
    assert_eq!(spell("UNIT_MARY", &[]), Some("SPELL_CONTAGION"));
    assert_eq!(spell("UNIT_AURIC_ASCENDED", &[]), Some("SPELL_SNOWFALL_GREATOR"));
}

/// Every sphere I gives an adept a spell in a battle: a damage spell, a heal, a charm or a hex.
/// A caster heals with its strongest heal and casts every charm and hex it knows.
#[test]
fn every_sphere_has_a_battle_spell() {
    let d = d();
    let p = |n: &str| d.t::<PromotionType>(n);
    let spells = |unit: &str, held: &[&str]| {
        let held: Vec<PromotionType> = held.iter().map(|n| p(n)).collect();
        d.engage_battle_spells(d.t::<UnitType>(unit), |x| held.contains(&x), None)
    };
    let name = |s: Option<SpellType>| s.map(|s| d.type_name(s));
    for sphere in [
        "AIR", "BODY", "CHAOS", "DEATH", "EARTH", "ENCHANTMENT", "ENTROPY", "FIRE", "ICE", "LAW", "LIFE", "METAMAGIC", "MIND", "NATURE",
        "SHADOW", "SPIRIT", "SUN", "WATER",
    ] {
        let promo = format!("PROMOTION_{sphere}1");
        assert!(spells("UNIT_ADEPT", &[promo.as_str()]).any(), "{promo}");
    }
    assert_eq!(name(spells("UNIT_ADEPT", &["PROMOTION_LIFE1"]).heal), Some("SPELL_MEND"));
    assert_eq!(name(spells("UNIT_MAGE", &["PROMOTION_LIFE1", "PROMOTION_LIFE2"]).heal), Some("SPELL_HEALING_LIGHT"));
    let earth = spells("UNIT_MAGE", &["PROMOTION_EARTH1", "PROMOTION_EARTH2"]);
    let mut charms: Vec<&str> = earth.charms.iter().map(|&s| d.type_name(s)).collect();
    charms.sort();
    assert_eq!((earth.attack, earth.heal, charms), (None, None, vec!["SPELL_STONESKIN_WARD", "SPELL_TREMOR"]));
    // Every sphere III gives an archmage another battle spell (Ice III: Snowfall's battle form).
    for sphere in [
        "AIR", "BODY", "CHAOS", "DEATH", "EARTH", "ENCHANTMENT", "ENTROPY", "FIRE", "ICE", "LAW", "LIFE", "METAMAGIC", "MIND", "NATURE",
        "SHADOW", "SPIRIT", "SUN", "WATER",
    ] {
        let promos: Vec<String> = (1..=3).map(|i| format!("PROMOTION_{sphere}{i}")).collect();
        let names: Vec<&str> = promos.iter().map(|x| x.as_str()).collect();
        assert!(spells("UNIT_ARCHMAGE", &names[..2]) != spells("UNIT_ARCHMAGE", &names), "{sphere} III");
    }
    // The strongest sphere's charms first.
    let earth = spells("UNIT_ARCHMAGE", &["PROMOTION_EARTH1", "PROMOTION_EARTH2", "PROMOTION_EARTH3"]);
    let order: Vec<&str> = earth.charms.iter().map(|&s| d.type_name(s)).collect();
    assert_eq!(order, ["SPELL_QUAKE", "SPELL_STONESKIN_WARD", "SPELL_TREMOR"]);
    // No charm below the damage spell's tier: Firestorm (III) and Hold the Line (I); Aegis (III).
    let charms = |held: &[&str]| -> Vec<&str> { spells("UNIT_ARCHMAGE", held).charms.iter().map(|&s| d.type_name(s)).collect() };
    let fire3 = ["PROMOTION_FIRE1", "PROMOTION_FIRE2", "PROMOTION_FIRE3"];
    assert_eq!(charms(&[&fire3[..], &["PROMOTION_LAW1"]].concat()), Vec::<&str>::new());
    assert_eq!(charms(&[&fire3[..], &["PROMOTION_LAW1", "PROMOTION_LAW2", "PROMOTION_LAW3"]].concat()), ["SPELL_AEGIS"]);
    assert_eq!(charms(&["PROMOTION_FIRE1", "PROMOTION_ENCHANTMENT1"]), ["SPELL_KEEN_EDGE"]);
    // Wonder is the damage spell of a caster that knows it, before Fire III's Firestorm; its
    // outcomes are no unit's own.
    let chaos_fire = ["PROMOTION_CHAOS1", "PROMOTION_CHAOS2", "PROMOTION_CHAOS3", "PROMOTION_FIRE1", "PROMOTION_FIRE2", "PROMOTION_FIRE3"];
    assert_eq!(name(spells("UNIT_ARCHMAGE", &chaos_fire).attack), Some("SPELL_WONDER"));
    let wonder = &d.spells[d.t::<SpellType>("SPELL_WONDER")];
    assert!(wonder.battle_wonder.len() >= 6 && !wonder.battle_only, "a map spell too");
    for &(o, w) in &wonder.battle_wonder {
        assert!(w > 0 && d.spells[o].wonder_outcome && d.spells[o].battle_only, "{}", d.type_name(o));
    }
    assert!(!spells("UNIT_ARCHMAGE", &[]).any(), "no outcome is known without Wonder");
    let keen = &d.spells[d.t::<SpellType>("SPELL_KEEN_EDGE")];
    assert_eq!(keen.battle_charm, Some(BattleCharm { damage: 20, ..BattleCharm::default() }), "its own front row");
    // The map spells that only dealt damage on the map are battle spells now.
    for s in ["SPELL_WITHER", "SPELL_CONTAGION", "SPELL_CROWN_OF_BRILLANCE"] {
        assert!(d.spells[d.t::<SpellType>(s)].battle_only, "{s}");
    }
}

#[test]
fn named_units_have_their_kinds() {
    for (unit, want) in [
        ("UNIT_AXEMAN", "STRIKE"),
        ("UNIT_ARCHER", "SHOOT"),
        ("UNIT_KNIGHT", "CHARGE"),
        ("UNIT_ASSASSIN", "STAB"),
        ("UNIT_ADEPT", "SPELL"),
        ("UNIT_PALADIN", "STRIKE"),
        ("UNIT_FIRE_ELEMENTAL", "STRIKE"),
        // Marksman is free but the unit is an archer, not recon (Q6): it shoots.
        ("UNIT_MARKSMAN", "SHOOT"),
        // recon units fight from any row and prefer the middle, where they shoot (2026-11-06)
        ("UNIT_SCOUT", "SHOOT"),
        ("UNIT_HUNTER", "SHOOT"),
        ("UNIT_RANGER", "SHOOT"),
        ("UNIT_CATAPULT", "BOMBARD"),
        ("UNIT_TREBUCHET", "BOMBARD"),
        ("UNIT_BATTERING_RAM", "BOMBARD"),
        // more of section 3's table
        ("UNIT_WOLF", "STRIKE"),
        ("UNIT_HILL_GIANT", "STRIKE"),
        ("UNIT_WAR_ELEPHANT", "CHARGE"),
        ("UNIT_HORSE_ARCHER", "CHARGE"),
        ("UNIT_SHADOW", "STAB"),
        ("UNIT_RATHUS", "STAB"),
        ("UNIT_PRIEST_OF_LEAVES", "HEAL"),
        ("UNIT_DRUID", "HEAL"),
        ("UNIT_MAGE", "SPELL"),
        ("UNIT_IRON_GOLEM", "STRIKE"),
        // the only land unit with UNITCOMBAT_NAVAL
        ("UNIT_AIRSHIP", "SHOOT"),
        // defends only, immobile: still a striker
        ("UNIT_GUARDIAN_VINES", "STRIKE"),
        ("UNIT_MUD_GOLEM", "STRIKE"),
        // typed strength only (lightning 3)
        ("UNIT_LIGHTNING_ELEMENTAL", "STRIKE"),
        // missiles have rows (their strike targets by them); the rules never commit them
        ("UNIT_FIREBALL", "STRIKE"),
        // Types without strength have rows too: a unit's strength is its own, and promotions add
        // to it (a Worker that takes the Godslayer is a 3/3 unit, and it defends).
        ("UNIT_WORKER", "STRIKE"),
        ("UNIT_SETTLER", "STRIKE"),
        ("UNIT_COMMANDER", "STRIKE"),
        ("UNIT_CORLINDALE", "SPELL"),
        ("EQUIPMENT_GOLDEN_HAMMER", "STRIKE"),
        ("UNIT_BRIGIT_HELD", "STRIKE"),
        // no rows: sea and air units
        ("UNIT_TRIREME", "-"),
        ("UNIT_KRAKEN", "-"),
        ("UNIT_HAWK", "-"),
    ] {
        assert_eq!(unit_kind(unit), want, "{unit}");
    }
}

#[test]
fn the_religions_fighters_strike_and_every_stabber_has_marksman() {
    let d = d();
    // Section 3: seven disciples that never cast strike from the front, like melee units.
    let channeling = d.t::<PromotionType>("PROMOTION_CHANNELING1");
    let divine = d.t::<PromotionType>("PROMOTION_DIVINE");
    let disciple = d.t::<UnitCombatType>("UNITCOMBAT_DISCIPLE");
    // The game's own unit types (the cards' come after them, docs/CARDS.md).
    let own = d.units.len() - ffh_data::card_entry_count("units");
    let mut strikers: Vec<&str> = d
        .units
        .keys()
        .take(own)
        .filter(|&u| d.units[u].unit_combat_type == Some(disciple) && d.units[u].engage_rows.is_some() && d.engage_kind(u) == ActionKind::Strike)
        .map(|u| d.type_name(u))
        .collect();
    strikers.sort_unstable();
    assert_eq!(strikers, ["UNIT_CRUSADER", "UNIT_EIDOLON", "UNIT_EINHERJAR", "UNIT_MONK", "UNIT_PALADIN", "UNIT_PARAMANDER", "UNIT_TAR_DEMON"]);
    for name in strikers {
        let u = &d.units[d.t::<UnitType>(name)];
        assert!(!u.free_promotions[channeling] && !u.free_promotions[divine], "{name} casts");
    }
    // Every type that fights land battles has a stat sheet (docs/UNIT_STATS.md), rows and a rank
    // preference, and ships none.
    for u in d.units.iter() {
        assert_eq!(u.stats.is_some(), u.engage_rows.is_some(), "{}", u.base.type_);
        assert_eq!(u.engage_rows.is_some(), u.engage_row_order.is_some(), "{}", u.base.type_);
    }
    // Q6: the stabbers are exactly the recon units with Marksman free.
    let marksman = d.t::<PromotionType>("PROMOTION_MARKSMAN");
    let recon = d.t::<UnitCombatType>("UNITCOMBAT_RECON");
    let mut stabbers: Vec<&str> =
        d.units.keys().filter(|&u| d.units[u].engage_rows.is_some() && d.engage_kind(u) == ActionKind::Stab).map(|u| d.type_name(u)).collect();
    stabbers.sort_unstable();
    for name in &stabbers {
        let u = &d.units[d.t::<UnitType>(name)];
        assert!(u.free_promotions[marksman] && u.unit_combat_type == Some(recon), "{name}");
    }
    assert_eq!(
        stabbers,
        [
            "UNIT_ALAZKAN", "UNIT_ANGEL_OF_DEATH", "UNIT_ASSASSIN", "UNIT_CHANTER", "UNIT_COURTESAN", "UNIT_DWARVEN_SHADOW",
            "UNIT_GHOST", "UNIT_LIZARDMAN_ASSASSIN", "UNIT_RATHUS", "UNIT_SHADOW", "UNIT_TASKMASTER",
        ]
    );
}

#[test]
fn recon_units_fight_from_any_row_and_prefer_the_middle() {
    use ActionKind::*;
    let d = d();
    // The owner's decision (2026-11-06; the Ogre Battle 64 Valkyrie pattern): every recon unit but
    // the stabbers strikes twice in front and shoots once from the middle and the rear, and the
    // auto formation puts it in the middle first, then the front.
    let recon = d.t::<UnitCombatType>("UNITCOMBAT_RECON");
    let mut n = 0;
    for u in d.units.keys().filter(|&u| d.units[u].unit_combat_type == Some(recon) && d.units[u].engage_rows.is_some()) {
        if d.engage_kind(u) == Stab {
            continue;
        }
        n += 1;
        assert_eq!(d.engage_rows(u), [row(Strike, 2), row(Shoot, 1), row(Shoot, 1)], "{}", d.type_name(u));
        assert_eq!(d.engage_row_order(u), [1, 0, 2], "{}", d.type_name(u));
    }
    assert_eq!(n, 20);
}

#[test]
fn every_land_type_has_rows() {
    let d = d();
    // The game's own unit types (the cards' come after them, docs/CARDS.md).
    let own = d.units.len() - ffh_data::card_entry_count("units");
    let count = |want: Option<ActionKind>| d.units.keys().take(own).filter(|&u| d.units[u].engage_rows.map(|_| d.engage_kind(u)) == want).count();
    // (kind, unit types) at the time of writing; the sum is every unit type.
    let want = [
        (Some(ActionKind::Strike), 170),
        (Some(ActionKind::Charge), 38),
        (Some(ActionKind::Stab), 11),
        (Some(ActionKind::Shoot), 37), // 16, this game's Skirmisher and the 20 recon units that do not stab
        (Some(ActionKind::Bombard), 5),
        (Some(ActionKind::Spell), 18),
        (Some(ActionKind::Heal), 29),
        (None, 19),
    ];
    for (k, n) in want {
        assert_eq!(count(k), n, "{k:?}");
    }
    assert_eq!(want.iter().map(|w| w.1).sum::<usize>(), own);
    // Whether a unit fights is its own strength, not its type's (promotions add to it), so every
    // land and immobile type has rows, with or without strength; sea and air types have none.
    for (u, info) in d.units.iter_enumerated() {
        let land = matches!(info.domain_type, Some(DomainType::DOMAIN_LAND) | Some(DomainType::DOMAIN_IMMOBILE));
        assert_eq!(info.engage_rows.is_some(), land, "{}", info.base.type_);
        assert_eq!(d.type_engages_on_land(u), land, "{}", info.base.type_);
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
    // The attack patterns (docs/UNIT_ROLES.md).
    let patterns = (x.engagement_volley_percent, x.engagement_pierce_percent, x.engagement_sweep_percent, x.engagement_cleave_percent, x.engagement_trample_percent);
    assert_eq!(patterns, (50, 50, 50, 60, 60));
    assert_eq!((x.engagement_city_attacker_shot_percent, x.engagement_forest_ranged_percent), (50, 50));
    assert_eq!((x.engagement_hills_attacker_percent, x.engagement_hills_defender_ranged_percent), (85, 125));
    assert_eq!(x.engagement_auto_min_hp_percent, 50);
    assert_eq!((x.engagement_recon_reveal_range, x.engagement_reports_kept, x.engagement_report_logs_kept), (2, 128, 32));
    // The round model's own constants are no longer defines.
    for gone in ["ENGAGEMENT_MAX_ROUNDS", "ENGAGEMENT_WITHDRAW_HP_PERCENT"] {
        assert!(!d().define_map.map.contains_key(gone), "{gone}");
    }
}
