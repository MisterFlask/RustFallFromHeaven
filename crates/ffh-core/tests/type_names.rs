//! Every type name hard-coded in the code (`d.t::<PromotionType>("PROMOTION_HERO")`, tables of
//! `"BONUS_MANA_FIRE"`, ...) is a type of the game data. `GameData::t` panics on an unknown name
//! only when that code runs, so a typo, or a type renamed or removed from the XML, would otherwise
//! show up as a crash in some rare game.
//!
//! Scans every string literal in the source of the crates that hold rules, AI and client code. A
//! literal is a type name when it is written like one (upper case, digits, `_`) and starts with
//! the prefix of a type class (`UNIT_`, `UNITCLASS_`, `PROMOTION_`, ... as the data has them).
//! Names passed to `ty` (`d.ty::<T>("X")`, `d.ty("X")`) are exempt: that lookup answers `None` for
//! an unknown name and the code handles it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The crates scanned (`ffh-cppai`, the unmaintained benchmark port, is left out).
const CRATES: &[&str] = &["ffh-core", "ffh-ai", "ffh-play", "ffh-cli", "ffh-stress", "ffh-web"];

/// Literals written like type names that are not meant as types.
const NOT_TYPES: &[&str] = &[
    "LEADER_FOO", "UNIT_BAR_NOPICK", // inputs of the `is_pickable_name` test (utils.rs)
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The string literals of `src` with the text before each (comments skipped; raw strings and
/// escapes are rare enough in this code base to be read as plain strings).
fn literals(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let b = line.as_bytes();
        let mut i = 0;
        let mut start = None;
        while i < b.len() {
            match (b[i], start) {
                (b'/', None) if b.get(i + 1) == Some(&b'/') => break,
                (b'\\', Some(_)) => i += 1,
                (b'"', None) => start = Some(i + 1),
                (b'"', Some(s)) => {
                    out.push((line[..s - 1].to_string(), line[s..i].to_string()));
                    start = None;
                }
                // A char literal `'"'`.
                (b'\'', None) if b.get(i + 2) == Some(&b'\'') => i += 2,
                _ => {}
            }
            i += 1;
        }
    }
    out
}

fn looks_like_type_name(s: &str) -> bool {
    s.len() > 2
        && s.contains('_')
        && !s.starts_with('_')
        && !s.ends_with('_')
        && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Is the literal the argument of a `ty` lookup (`d.ty("X")`, `d.ty::<T>("X")`), which answers
/// `None` for an unknown name?
fn is_ty_lookup(before: &str) -> bool {
    let Some(call) = before.strip_suffix('(') else { return false };
    let name = match call.strip_suffix('>').and_then(|c| c.rfind("::<").map(|i| &c[..i])) {
        Some(turbofish) => turbofish,
        None => call,
    };
    name.ends_with(".ty") || name.ends_with(" ty") || name.ends_with("::ty")
}

/// `UNITCLASS_WARRIOR` -> `UNITCLASS_`.
fn prefix(s: &str) -> &str {
    &s[..=s.find('_').unwrap()]
}

#[test]
fn hard_coded_type_names_exist() {
    let d = ffh_data::global();
    // The classes each type-name prefix belongs to.
    let mut classes_of: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for class in d.reg.classes() {
        for name in d.reg.names_of(class) {
            if looks_like_type_name(name) {
                classes_of.entry(prefix(name).to_string()).or_default().insert(class);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for c in CRATES {
        rust_files(&root.join(c).join("src"), &mut files);
    }
    assert!(files.len() > 100, "only {} source files found", files.len());

    let mut checked = 0;
    let mut problems = Vec::new();
    for file in &files {
        let src = std::fs::read_to_string(file).unwrap();
        for (before, s) in literals(&src) {
            if !looks_like_type_name(&s) {
                continue;
            }
            let Some(classes) = classes_of.get(prefix(&s)) else { continue };
            let before = before.trim_end();
            // ffh-ai's switches (`Switch::new("TECH_DISCIPLINE")`, `with_vars`) are named like types.
            if is_ty_lookup(before) || before.ends_with("Switch::new(") || before.ends_with("Switch::with_vars(") || NOT_TYPES.contains(&s.as_str()) {
                continue;
            }
            checked += 1;
            match d.reg.entry(&s) {
                Some(e) if classes.contains(e.class) => {}
                Some(e) => problems.push(format!("{}: {s} is a {}, not one of {classes:?}", file.display(), e.class)),
                None => problems.push(format!("{}: unknown type {s}", file.display())),
            }
        }
    }
    assert!(problems.is_empty(), "{} hard-coded type names do not resolve:\n{}", problems.len(), problems.join("\n"));
    assert!(checked > 2000, "only {checked} type names checked");
}
