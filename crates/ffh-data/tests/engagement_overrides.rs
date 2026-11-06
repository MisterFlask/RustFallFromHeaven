//! A unit type's own formation-battle values: its stat sheet (`stats` in units.jsonc:
//! docs/UNIT_STATS.md), its rows (`engage_rows`) and its rank preference (`engage_row_order`;
//! docs/ENGAGEMENTS.md, "Per-unit values"). This test loads a fixture: the game's data with a few
//! units given other rows, orders and sheet values, good and bad, and checks what each unit's
//! values resolve to and the load warnings.

use std::path::{Path, PathBuf};

use ffh_data::*;

/// A change to the fixture: the line of a unit's member `key` replaced (`Replace(key, line)`), or
/// one value of its sheet.
enum Edit {
    Replace(&'static str, &'static str),
    Stat(&'static str, i32),
}

/// Fixture changes, by unit.
const FIXTURE: &[(&str, Edit)] = &[
    // A tier-1 melee unit that shoots and stabs: its rows are its own.
    (
        "UNIT_WARRIOR",
        Edit::Replace("engage_rows", r#""engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "STAB", "count": 2}, {"action": "NONE", "count": 0}],"#),
    ),
    // A flanker with no evasion and a high crit (its sheet's values).
    // A hunter that prefers the rear row, then the front.
    ("UNIT_HUNTER", Edit::Replace("engage_row_order", r#""engage_row_order": ["REAR", "FRONT", "MIDDLE"],"#)),
    ("UNIT_HORSEMAN", Edit::Stat("evasion", 0)),
    ("UNIT_HORSEMAN", Edit::Stat("crit", 35)),
    // A tier 4 flanker whose best row is the middle one: its extra action goes there.
    ("UNIT_KNIGHT", Edit::Replace("engage_rows", r#""engage_rows": [{"action": "STRIKE", "count": 1}, {"action": "CHARGE", "count": 2}, {"action": "NONE"}],"#)),
    // An archer whose rear row charges: a flanker by its own rows.
    ("UNIT_ARCHER", Edit::Replace("engage_rows", r#""engage_rows": [{"action": "SHOOT", "count": 1}, {"action": "SHOOT", "count": 1}, {"action": "CHARGE", "count": 1}],"#)),
    // Bad values: each warns; a bad sheet value reads as its range's lowest, bad rows or a bad
    // rank order read as none (and the loader warns for a land unit without them).
    ("UNIT_SCOUT", Edit::Stat("evasion", 120)),
    ("UNIT_SCOUT", Edit::Stat("speed", -3)),
    ("UNIT_AXEMAN", Edit::Replace("engage_rows", r#""engage_rows": [{"action": "STRIKE", "count": 2}, {"action": "STRIKE", "count": 1}],"#)),
    ("UNIT_SWORDSMAN", Edit::Replace("engage_rows", r#""engage_rows": [{"action": "THROW", "count": 2}, {"action": "STRIKE", "count": 1}, {"action": "STRIKE", "count": 1}],"#)),
    ("UNIT_CHAMPION", Edit::Replace("engage_rows", r#""engage_rows": [{"action": "STRIKE", "count": 0}, {"action": "NONE", "count": 1}, {"action": "STRIKE", "count": 1}],"#)),
    ("UNIT_RANGER", Edit::Replace("engage_row_order", r#""engage_row_order": ["FRONT", "FRONT", "REAR"],"#)),
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
            Edit::Replace(member, line) => {
                // The unit's own line of the member: the first after its type.
                let at = text.find(&key).unwrap();
                let m = format!("\n    \"{member}\": ");
                let line_start = at + text[at..].find(&m).unwrap() + 1;
                let line_end = line_start + text[line_start..].find('\n').unwrap();
                text = format!("{}    {line}{}", &text[..line_start], &text[line_end..]);
            }
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
fn a_unit_types_own_values() {
    use ActionKind::*;
    let d = load_fixture();
    let unit = |n: &str| d.t::<UnitType>(n);
    let stat = |n: &str, s: StatType| d.unit_stat(unit(n), s);

    // The warrior: its own rows; its speed is its sheet's.
    let w = unit("UNIT_WARRIOR");
    let shoot_stab = [row(Shoot, 1), row(Stab, 2), row(None, 0)];
    assert_eq!(d.units[w].engage_rows, Some(shoot_stab));
    assert_eq!(stat("UNIT_WARRIOR", StatType::Speed), 4);
    assert_eq!(d.engage_rows(w), shoot_stab);
    // Its best row and its extra action come from its own rows: the middle one, for a hero.
    assert_eq!(d.engage_best_row(w), 1);
    assert_eq!(d.engage_extra_actions(w, false), 0);
    assert_eq!(d.engage_actions(w, false), shoot_stab);
    assert_eq!(d.engage_actions(w, true), [row(Shoot, 1), row(Stab, 3), row(None, 0)]);
    assert!(!d.engage_flanks(w));
    // It prefers the front row (its rank order is unchanged), so its kind is a shot.
    assert_eq!((d.engage_row_order(w), d.engage_kind(w)), ([0, 1, 2], Shoot));

    // The horseman: its sheet's evasion and crit; its rows charge.
    let h = unit("UNIT_HORSEMAN");
    assert_eq!((stat("UNIT_HORSEMAN", StatType::Evasion), stat("UNIT_HORSEMAN", StatType::Crit), stat("UNIT_HORSEMAN", StatType::Speed)), (0, 35, 6));
    assert_eq!(d.engage_rows(h), [row(Charge, 2), row(Strike, 1), row(Strike, 1)]);
    assert!(d.engage_flanks(h));

    // The hunter prefers the rear row, then the front: its kind is what it does there.
    let hu = unit("UNIT_HUNTER");
    assert_eq!((d.engage_row_order(hu), d.engage_kind(hu)), ([2, 0, 1], Strike));

    // The knight, tier 4: its extra action goes to its own best row, the middle one.
    let k = unit("UNIT_KNIGHT");
    assert_eq!(d.units[k].tier, 4);
    assert_eq!(d.engage_extra_actions(k, false), 1);
    assert_eq!(d.engage_best_row(k), 1);
    assert_eq!(d.engage_actions(k, false), [row(Strike, 1), row(Charge, 3), row(None, 0)]);
    assert!(d.engage_flanks(k));

    // The archer flanks by its own rows.
    let a = unit("UNIT_ARCHER");
    assert!(d.engage_flanks(a));
    // Of three rows with one action each, the first is the best.
    assert_eq!(d.engage_best_row(a), 0);

    // Bad values warn: a sheet value reads as its range's lowest; bad rows read as none (no
    // action from any row), a bad rank order as none (front, middle, rear).
    assert_eq!((stat("UNIT_SCOUT", StatType::Evasion), stat("UNIT_SCOUT", StatType::Speed)), (0, 0));
    for name in ["UNIT_AXEMAN", "UNIT_SWORDSMAN", "UNIT_CHAMPION"] {
        let u = unit(name);
        assert_eq!(d.units[u].engage_rows, Option::None, "{name}");
        assert_eq!(d.engage_rows(u), [RowActions::default(); ROWS], "{name}");
    }
    assert_eq!((d.units[unit("UNIT_RANGER")].engage_row_order, d.engage_row_order(unit("UNIT_RANGER"))), (Option::None, [0, 1, 2]));
    let mut warnings: Vec<String> = d.warnings.iter().map(|w| w.replace("units.jsonc: ", "")).collect();
    warnings.sort();
    assert_eq!(
        warnings,
        [
            "UNIT_AXEMAN: UNIT_AXEMAN: engage_rows has 2 rows (3 expected: front, middle, rear)",
            "UNIT_AXEMAN: a land unit without engage_rows",
            "UNIT_CHAMPION: UNIT_CHAMPION: engage_rows: a row has a count above 0 exactly when its action is not NONE",
            "UNIT_CHAMPION: a land unit without engage_rows",
            "UNIT_RANGER: UNIT_RANGER: engage_row_order [\"FRONT\", \"FRONT\", \"REAR\"] is not FRONT, MIDDLE and REAR once each",
            "UNIT_RANGER: a land unit without engage_row_order",
            "UNIT_SCOUT: UNIT_SCOUT: stats \"evasion\" 120 is not in 0..=95",
            "UNIT_SCOUT: UNIT_SCOUT: stats \"speed\" -3 is not in 0..=2147483647",
            "UNIT_SWORDSMAN: UNIT_SWORDSMAN: engage_rows, FRONT row: unknown action \"THROW\"",
            "UNIT_SWORDSMAN: a land unit without engage_rows",
        ]
    );

    // A sea unit has no sheet, no rows and no rank preference.
    let ship = d.units.keys().find(|&u| !d.type_engages_on_land(u)).unwrap();
    assert_eq!((d.units[ship].stats, d.units[ship].engage_rows, d.units[ship].engage_row_order), (Option::None, Option::None, Option::None));
}
