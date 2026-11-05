//! A unit type's own formation-battle values: its stat sheet (`stats` in units.jsonc:
//! docs/UNIT_STATS.md) and its own rows in place of its role's (`engage_rows`; docs/ENGAGEMENTS.md,
//! "Per-unit values"). This test loads a fixture: the game's data with a few units given rows and
//! sheet values, good and bad, and checks what each unit's values resolve to and the load warnings.

use std::path::{Path, PathBuf};

use ffh_data::*;

/// A change to the fixture: members inserted after a unit's `"type"`, or one value of its sheet.
enum Edit {
    Insert(&'static str),
    Stat(&'static str, i32),
}

/// Fixture changes, by unit.
const FIXTURE: &[(&str, Edit)] = &[
    // A frontline unit (tier 1) that acts like a skirmisher: its rows are its own.
    (
        "UNIT_WARRIOR",
        Edit::Insert(r#""engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "STAB", "count": 2}, {"action": "NONE", "count": 0}],"#),
    ),
    // A flanker with no evasion and a high crit (its sheet's values).
    ("UNIT_HORSEMAN", Edit::Stat("evasion", 0)),
    ("UNIT_HORSEMAN", Edit::Stat("crit", 35)),
    // A tier 4 flanker whose best row is the middle one: its extra action goes there.
    ("UNIT_KNIGHT", Edit::Insert(r#""engage_rows": [{"action": "STRIKE", "count": 1}, {"action": "CHARGE", "count": 2}, {"action": "NONE"}],"#)),
    // An archer whose rear row charges: a flanker by its own rows.
    ("UNIT_ARCHER", Edit::Insert(r#""engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "SHOOT", "count": 1}, {"action": "CHARGE", "count": 1}],"#)),
    // Bad values: each warns; a bad sheet value reads as its range's lowest, bad rows give the
    // role's.
    ("UNIT_SCOUT", Edit::Stat("evasion", 120)),
    ("UNIT_SCOUT", Edit::Stat("speed", -3)),
    ("UNIT_AXEMAN", Edit::Insert(r#""engage_rows": [{"action": "STRIKE", "count": 2}, {"action": "STRIKE", "count": 1}],"#)),
    ("UNIT_SWORDSMAN", Edit::Insert(r#""engage_rows": [{"action": "THROW", "count": 2}, {"action": "STRIKE", "count": 1}, {"action": "STRIKE", "count": 1}],"#)),
    ("UNIT_CHAMPION", Edit::Insert(r#""engage_rows": [{"action": "STRIKE", "count": 0}, {"action": "NONE", "count": 1}, {"action": "STRIKE", "count": 1}],"#)),
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

/// The game's data with [`FIXTURE`] applied, loaded from a copy in the temp directory.
fn load_fixture() -> GameData {
    let src = default_data_dir();
    let dir: PathBuf = std::env::temp_dir().join(format!("ffh-data-engage-overrides-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&src, &dir);
    let units = dir.join("json").join("units.jsonc");
    let mut text = std::fs::read_to_string(&units).unwrap();
    for (unit, edit) in FIXTURE {
        let key = format!("\"type\": \"{unit}\",");
        assert_eq!(text.matches(&key).count(), 1, "{unit}");
        match edit {
            Edit::Insert(snippet) => text = text.replacen(&key, &format!("{key}\n{snippet}"), 1),
            Edit::Stat(stat, value) => {
                // The sheet is the line after the type.
                let at = text.find(&key).unwrap();
                let line_start = at + text[at..].find('\n').unwrap() + 1;
                let line_end = line_start + text[line_start..].find('\n').unwrap();
                let line = &text[line_start..line_end];
                assert!(line.trim_start().starts_with("\"stats\""), "{unit}: {line}");
                let k = format!("\"{stat}\": ");
                let v0 = line.find(&k).unwrap() + k.len();
                let v1 = v0 + line[v0..].find(|c: char| c == ',' || c == '}').unwrap();
                let new_line = format!("{}{value}{}", &line[..v0], &line[v1..]);
                text = format!("{}{new_line}{}", &text[..line_start], &text[line_end..]);
            }
        }
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
    let stat = |n: &str, s: StatType| d.unit_stat(unit(n), s);

    // The warrior: its own rows; its speed is its sheet's.
    let (w, wr) = (unit("UNIT_WARRIOR"), role_of("UNIT_WARRIOR"));
    assert_eq!(wr, Some(d.t::<CombatRoleType>("COMBATROLE_FRONTLINE")));
    let skirmisher = [row(Shoot, 1), row(Stab, 2), row(None, 0)];
    assert_eq!(d.units[w].engage_rows, Some(skirmisher));
    assert_eq!(stat("UNIT_WARRIOR", StatType::Speed), 4);
    assert_eq!(d.engage_rows(w, wr), skirmisher);
    // Its best row and its extra action come from its own rows: the middle one, for a hero.
    assert_eq!(d.engage_best_row(w, wr), 1);
    assert_eq!(d.engage_extra_actions(w, false), 0);
    assert_eq!(d.engage_actions(w, wr, false), skirmisher);
    assert_eq!(d.engage_actions(w, wr, true), [row(Shoot, 1), row(Stab, 3), row(None, 0)]);
    assert!(!d.engage_flanks(w, wr));
    // The role is unchanged (its other units fight by it).
    assert_eq!(role("COMBATROLE_FRONTLINE").rows, [row(Strike, 2), row(Strike, 1), row(Strike, 1)]);
    // The type's rows hold whatever role the unit has (a promotion can change it).
    let screen = Some(d.t::<CombatRoleType>("COMBATROLE_SCREEN"));
    assert_eq!(d.engage_rows(w, screen), skirmisher);

    // The horseman: its sheet's evasion and crit; its role's rows charge.
    let (h, hr) = (unit("UNIT_HORSEMAN"), role_of("UNIT_HORSEMAN"));
    assert_eq!(hr, Some(d.t::<CombatRoleType>("COMBATROLE_FLANKER")));
    assert_eq!((stat("UNIT_HORSEMAN", StatType::Evasion), stat("UNIT_HORSEMAN", StatType::Crit), stat("UNIT_HORSEMAN", StatType::Speed)), (0, 35, 6));
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

    // Bad values warn: a sheet value reads as its range's lowest, rows fall back to the role's.
    assert_eq!((stat("UNIT_SCOUT", StatType::Evasion), stat("UNIT_SCOUT", StatType::Speed)), (0, 0));
    for name in ["UNIT_AXEMAN", "UNIT_SWORDSMAN", "UNIT_CHAMPION"] {
        let (u, r) = (unit(name), role_of(name));
        assert_eq!(d.units[u].engage_rows, Option::None, "{name}");
        assert_eq!(d.engage_rows(u, r), role("COMBATROLE_FRONTLINE").rows, "{name}");
    }
    let mut warnings: Vec<String> = d.warnings.iter().map(|w| w.replace("units.jsonc: ", "")).collect();
    warnings.sort();
    assert_eq!(
        warnings,
        [
            "UNIT_AXEMAN: UNIT_AXEMAN: engage_rows has 2 rows (3 expected: front, middle, rear)",
            "UNIT_CHAMPION: UNIT_CHAMPION: engage_rows: a row has a count above 0 exactly when its action is not NONE",
            "UNIT_SCOUT: UNIT_SCOUT: stats \"evasion\" 120 is not in 0..=95",
            "UNIT_SCOUT: UNIT_SCOUT: stats \"speed\" -3 is not in 0..=2147483647",
            "UNIT_SWORDSMAN: UNIT_SWORDSMAN: engage_rows, FRONT row: unknown action \"THROW\"",
        ]
    );

    // A unit type without a role (a ship) has no sheet and no rows.
    let ship = d.units.keys().find(|&u| d.unit_roles[u].is_none()).unwrap();
    assert_eq!((d.units[ship].stats, d.engage_rows(ship, Option::None)), (Option::None, [RowActions::default(); ROWS]));
}

#[test]
fn the_games_units_set_no_rows_of_their_own() {
    // The content is the owner's to decide: every unit of the game's data takes its role's rows,
    // but UNIT_SENTINEL, which is never trained (docs/SIEGES.md: a city's Sentinels fight with it).
    let d = ffh_data::global();
    assert!(d.units.iter().filter(|u| u.base.type_ != "UNIT_SENTINEL").all(|u| u.engage_rows.is_none()));
}
