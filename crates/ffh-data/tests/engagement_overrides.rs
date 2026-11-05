//! A unit type's own formation-battle values in place of its role's (`engage_speed`,
//! `engage_dodge`, `engage_crit_bonus`, `engage_rows` in units.jsonc; docs/ENGAGEMENTS.md,
//! "Per-unit values"). No unit of the game's data sets them (the content is the owner's to
//! decide), so this test loads a fixture: the game's data with a few units given overrides, good
//! and bad, and checks what each unit's values resolve to (its own, else its role's) and the load
//! warnings.

use std::path::{Path, PathBuf};

use ffh_data::*;

/// Fixture overrides: the members inserted after each unit's `"type"`.
const FIXTURE: &[(&str, &str)] = &[
    // A frontline unit (tier 1) that is fast and acts like a skirmisher: its speed and rows are
    // its own, its dodge and crit bonus its role's.
    (
        "UNIT_WARRIOR",
        r#""engage_speed": 9,
           "engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "STAB", "count": 2}, {"action": "NONE", "count": 0}],"#,
    ),
    // A flanker with no dodge (0 is a value, not the role's) and a crit bonus; -1 is the role's.
    ("UNIT_HORSEMAN", r#""engage_dodge": 0, "engage_crit_bonus": 25, "engage_speed": -1,"#),
    // A tier 4 flanker whose best row is the middle one: its extra action goes there.
    ("UNIT_KNIGHT", r#""engage_rows": [{"action": "STRIKE", "count": 1}, {"action": "CHARGE", "count": 2}, {"action": "NONE"}],"#),
    // An archer whose rear row charges: a flanker by its own rows.
    ("UNIT_ARCHER", r#""engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "SHOOT", "count": 1}, {"action": "CHARGE", "count": 1}],"#),
    // Bad values: each warns, and the unit keeps its role's.
    ("UNIT_SCOUT", r#""engage_dodge": 120, "engage_speed": -3,"#),
    ("UNIT_AXEMAN", r#""engage_rows": [{"action": "STRIKE", "count": 2}, {"action": "STRIKE", "count": 1}],"#),
    ("UNIT_SWORDSMAN", r#""engage_rows": [{"action": "THROW", "count": 2}, {"action": "STRIKE", "count": 1}, {"action": "STRIKE", "count": 1}],"#),
    ("UNIT_CHAMPION", r#""engage_rows": [{"action": "STRIKE", "count": 0}, {"action": "NONE", "count": 1}, {"action": "STRIKE", "count": 1}],"#),
];

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    let mut entries: Vec<_> = std::fs::read_dir(from).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &dest);
        } else {
            std::fs::copy(&p, &dest).unwrap();
        }
    }
}

/// The game's data with [`FIXTURE`] inserted, loaded from a copy in the temp directory.
fn load_fixture() -> GameData {
    let src = default_data_dir();
    let dir: PathBuf = std::env::temp_dir().join(format!("ffh-data-engage-overrides-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&src, &dir);
    let units = dir.join("json").join("units.jsonc");
    let mut text = std::fs::read_to_string(&units).unwrap();
    for (unit, snippet) in FIXTURE {
        let key = format!("\"type\": \"{unit}\",");
        assert_eq!(text.matches(&key).count(), 1, "{unit}");
        text = text.replacen(&key, &format!("{key}\n{snippet}"), 1);
    }
    std::fs::write(&units, text).unwrap();
    let d = GameData::load(&dir).expect("the fixture loads");
    let _ = std::fs::remove_dir_all(&dir);
    d
}

fn row(kind: ActionKind, count: i32) -> RowActions {
    RowActions { kind, count }
}

#[test]
fn a_unit_types_own_values_replace_its_roles() {
    use ActionKind::*;
    let d = load_fixture();
    let unit = |n: &str| d.t::<UnitType>(n);
    let role_of = |n: &str| d.unit_roles[unit(n)];
    let role = |n: &str| &d.combat_roles[d.t::<CombatRoleType>(n)];

    // The warrior: its own speed and rows, its role's dodge and crit bonus.
    let (w, wr) = (unit("UNIT_WARRIOR"), role_of("UNIT_WARRIOR"));
    assert_eq!(wr, Some(d.t::<CombatRoleType>("COMBATROLE_FRONTLINE")));
    let skirmisher = [row(Shoot, 1), row(Stab, 2), row(None, 0)];
    assert_eq!(d.units[w].engage_rows, Some(skirmisher));
    assert_eq!((d.engage_speed(w, wr), d.engage_dodge(w, wr), d.engage_crit_bonus(w, wr)), (9, 0, 0));
    assert_eq!(d.engage_rows(w, wr), skirmisher);
    // Its best row and its extra action come from its own rows: the middle one, for a hero.
    assert_eq!(d.engage_best_row(w, wr), 1);
    assert_eq!(d.engage_extra_actions(w, false), 0);
    assert_eq!(d.engage_actions(w, wr, false), skirmisher);
    assert_eq!(d.engage_actions(w, wr, true), [row(Shoot, 1), row(Stab, 3), row(None, 0)]);
    assert!(!d.engage_flanks(w, wr));
    // The role is unchanged (its other units fight by it).
    assert_eq!(role("COMBATROLE_FRONTLINE").rows, [row(Strike, 2), row(Strike, 1), row(Strike, 1)]);
    // The type's values hold whatever role the unit has (a promotion can change it).
    let screen = Some(d.t::<CombatRoleType>("COMBATROLE_SCREEN"));
    assert_eq!((d.engage_speed(w, screen), d.engage_rows(w, screen)), (9, skirmisher));

    // The horseman: dodge 0 and crit 25 its own; speed -1 is its role's; its role's rows charge.
    let (h, hr) = (unit("UNIT_HORSEMAN"), role_of("UNIT_HORSEMAN"));
    assert_eq!(hr, Some(d.t::<CombatRoleType>("COMBATROLE_FLANKER")));
    assert_eq!((d.units[h].engage_speed, d.units[h].engage_dodge, d.units[h].engage_crit_bonus), (Option::None, Some(0), Some(25)));
    assert_eq!((d.engage_speed(h, hr), d.engage_dodge(h, hr), d.engage_crit_bonus(h, hr)), (6, 0, 25));
    assert_eq!(d.engage_rows(h, hr), role("COMBATROLE_FLANKER").rows);
    assert!(d.engage_flanks(h, hr));

    // The knight, tier 4: its extra action goes to its own best row, the middle one.
    let (k, kr) = (unit("UNIT_KNIGHT"), role_of("UNIT_KNIGHT"));
    assert_eq!(d.units[k].tier, 4);
    assert_eq!(d.engage_extra_actions(k, false), 1);
    assert_eq!(d.engage_best_row(k, kr), 1);
    assert_eq!(d.engage_actions(k, kr, false), [row(Strike, 1), row(Charge, 3), row(None, 0)]);
    assert!(d.engage_flanks(k, kr));

    // The archer flanks by its own rows; its role does not.
    let (a, ar) = (unit("UNIT_ARCHER"), role_of("UNIT_ARCHER"));
    assert_eq!(ar, Some(d.t::<CombatRoleType>("COMBATROLE_ARCHER")));
    assert!(d.engage_flanks(a, ar) && !role("COMBATROLE_ARCHER").flanks());
    // Of three rows with one action each, the first is the best.
    assert_eq!(d.engage_best_row(a, ar), 0);

    // Bad values warn and fall back to the role's.
    for (name, role_name) in [
        ("UNIT_SCOUT", "COMBATROLE_FRONTLINE"),
        ("UNIT_AXEMAN", "COMBATROLE_FRONTLINE"),
        ("UNIT_SWORDSMAN", "COMBATROLE_FRONTLINE"),
        ("UNIT_CHAMPION", "COMBATROLE_FRONTLINE"),
    ] {
        let (u, r) = (unit(name), role_of(name));
        let info = &d.units[u];
        assert_eq!((info.engage_speed, info.engage_dodge, info.engage_crit_bonus, info.engage_rows), (Option::None, Option::None, Option::None, Option::None), "{name}");
        let want = role(role_name);
        assert_eq!((d.engage_speed(u, r), d.engage_dodge(u, r), d.engage_rows(u, r)), (want.speed, want.dodge, want.rows), "{name}");
    }
    let mut warnings: Vec<String> = d.warnings.iter().map(|w| w.replace("units.jsonc: ", "")).collect();
    warnings.sort();
    assert_eq!(
        warnings,
        [
            "UNIT_AXEMAN: UNIT_AXEMAN: engage_rows has 2 rows (3 expected: front, middle, rear)",
            "UNIT_CHAMPION: UNIT_CHAMPION: engage_rows: a row has a count above 0 exactly when its action is not NONE",
            "UNIT_SCOUT: UNIT_SCOUT: engage_dodge 120 is neither -1 (the role's) nor in 0..=95",
            "UNIT_SCOUT: UNIT_SCOUT: engage_speed -3 is neither -1 (the role's) nor in 0..=2147483647",
            "UNIT_SWORDSMAN: UNIT_SWORDSMAN: engage_rows, FRONT row: unknown action \"THROW\"",
        ]
    );

    // A unit type without a role (a ship) answers its own values, else nothing.
    let ship = d.units.keys().find(|&u| d.unit_roles[u].is_none()).unwrap();
    assert_eq!((d.engage_speed(ship, Option::None), d.engage_rows(ship, Option::None)), (0, [RowActions::default(); ROWS]));
}

#[test]
fn the_games_units_set_none_of_them() {
    // The content is the owner's to decide: every unit of the game's data takes its role's, but
    // UNIT_SENTINEL, which is never trained (docs/SIEGES.md: a city's Sentinels fight with it).
    let d = ffh_data::global();
    assert!(d
        .units
        .iter()
        .filter(|u| u.base.type_ != "UNIT_SENTINEL")
        .all(|u| u.engage_speed.is_none() && u.engage_dodge.is_none() && u.engage_crit_bonus.is_none() && u.engage_rows.is_none()));
}
