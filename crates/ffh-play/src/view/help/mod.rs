//! Help texts: what a tech, a building, a unit, a promotion, ... does, as Civ4 markup for the
//! page (ports of C++ `CvGameTextMgr`, which the exe shows in its tooltips and pedia).
//!
//! The texts are built from the info tables, so they follow the rules as the data changes. Each
//! line is Civ4 markup (`[ICON_BULLET]`, `[COLOR_X]..[COLOR_REVERT]`, `[ICON_FOOD]`, ...) and
//! lines are joined by `[NEWLINE]`; the page renders it (`markup.js`). Text keys are looked up in
//! FfH2's text first; the keys FfH2 inherits from BtS are not vendored, so each call carries the
//! BtS English as its fallback.
//!
//! * [`tech`]: what researching a tech brings the human (C++ `setTechHelp`, in the player's
//!   context: its own units and buildings, only what it cannot do yet).
//! * [`civ`]: what a civilization, a leader and a trait bring (C++ `parseCivInfos`,
//!   `parseLeaderTraits`, `parseTraits`; the new-game screen).

pub mod civ;
pub mod tech;

use ffh_data::*;

use crate::names;

/// A `getText` argument.
pub(crate) enum Arg<'a> {
    N(i32),
    S(&'a str),
}

/// Civ4 `getText` placeholders: `%d1` (a number), `%D1` (a number with its sign), `%s1` and
/// `%F1` (text), each with an optional grammatical suffix (`%s1_unit`, `%d1_Num`), `%%`, and the
/// plural form `[NUM1:Route:Routes]` (the first word when argument 1 is 1, else the second).
pub(crate) fn fill(t: &str, args: &[Arg]) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    loop {
        let pct = rest.find('%');
        let num = rest.find("[NUM");
        let at = match (pct, num) {
            (Some(p), Some(n)) => p.min(n),
            (Some(p), None) => p,
            (None, Some(n)) => n,
            (None, None) => break,
        };
        out.push_str(&rest[..at]);
        if Some(at) == num {
            // `[NUMn:singular:plural]`.
            let tail = &rest[at + 4..];
            let digit = tail.as_bytes().first().copied().unwrap_or(0);
            if let (true, Some(end)) = ((b'1'..=b'9').contains(&digit) && tail.as_bytes().get(1) == Some(&b':'), tail.find(']')) {
                let forms: Vec<&str> = tail[2..end].split(':').collect();
                let n = match args.get((digit - b'1') as usize) {
                    Some(Arg::N(n)) => *n,
                    _ => 0,
                };
                out.push_str(if n.abs() == 1 { forms[0] } else { forms.get(1).copied().unwrap_or(forms[0]) });
                rest = &tail[end + 1..];
            } else {
                out.push_str("[NUM");
                rest = tail;
            }
            continue;
        }
        let tail = &rest[at + 1..];
        let b = tail.as_bytes();
        if b.first() == Some(&b'%') {
            out.push('%');
            rest = &tail[1..];
        } else if b.len() >= 2 && matches!(b[0], b'd' | b'D' | b's' | b'F') && (b'1'..=b'9').contains(&b[1]) {
            match (b[0], args.get((b[1] - b'1') as usize)) {
                (b'D', Some(Arg::N(n))) => out.push_str(&format!("{n:+}")),
                (_, Some(Arg::N(n))) => out.push_str(&n.to_string()),
                (_, Some(Arg::S(s))) => out.push_str(s),
                (_, None) => {}
            }
            // The suffix is a word that may hold underscores (`%s1_great_person`).
            let mut len = 2;
            if b.get(2) == Some(&b'_') {
                let word = b[3..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == b'_').count();
                let word = b[3..3 + word].iter().rposition(|c| c.is_ascii_alphanumeric()).map_or(0, |i| i + 1);
                if word > 0 {
                    len += 1 + word;
                }
            }
            rest = &tail[len..];
        } else {
            out.push('%');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// A text key filled with `args`; `fallback` when the key has no text (the keys FFH2 inherits
/// from BtS are not vendored).
pub(crate) fn txt(d: &GameData, key: &str, fallback: &str, args: &[Arg]) -> String {
    let t = names::text(d, key);
    fill(if t == key { fallback } else { t }, args)
}

/// A help text on one line, for after a name: "[ICON_BULLET]A[NEWLINE][ICON_BULLET]B" -> "A; B".
pub(crate) fn one_line(t: &str) -> String {
    t.split("[NEWLINE]").map(|l| l.replace("[ICON_BULLET]", "").trim().to_string()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("; ")
}

/// Line break of the help texts (C++ `NEWLINE`).
pub(crate) const NEWLINE: &str = "[NEWLINE]";

/// `text` in a Civ4 text colour (C++ `SETCOLR .. ENDCOLR` with `TEXT_COLOR(name)`).
pub(crate) fn color(name: &str, text: &str) -> String {
    format!("[{name}]{text}[COLOR_REVERT]")
}

/// C++ `setListHelp`: `start` and `item` for the first item of a list, else `sep` and `item`.
pub(crate) fn list_help(buf: &mut String, start: &str, item: &str, sep: &str, first: bool) {
    buf.push_str(if first { start } else { sep });
    buf.push_str(item);
}

/// The markup icon of each yield (C++ `getYieldInfo(i).getChar()`).
pub(crate) const YIELD_ICONS: [&str; YieldType::NUM] = ["[ICON_FOOD]", "[ICON_PRODUCTION]", "[ICON_COMMERCE]"];

/// The markup icon of each commerce (C++ `getCommerceInfo(i).getChar()`).
pub(crate) const COMMERCE_ICONS: [&str; CommerceType::NUM] = ["[ICON_GOLD]", "[ICON_RESEARCH]", "[ICON_CULTURE]", "[ICON_ESPIONAGE]"];

/// C++ `setYieldChangeHelp` / `setCommerceChangeHelp`: "start: +1⚒, +2¤" (`percent`: "+10%⚒"),
/// on a new bulleted line when `new_line`; nothing when every change is 0.
pub(crate) fn changes_help(buf: &mut String, start: &str, space: &str, end: &str, changes: &[i32], icons: &[&str], percent: bool, new_line: bool) {
    let mut started = false;
    for (i, &n) in changes.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let sign = if n > 0 { "+" } else { "" };
        let pct = if percent { "%" } else { "" };
        if !started {
            if new_line {
                buf.push_str(NEWLINE);
                buf.push_str("[ICON_BULLET]");
            }
            buf.push_str(&format!("{start}{space}{sign}{n}{pct}{}", icons[i]));
        } else {
            buf.push_str(&format!(", {sign}{n}{pct}{}", icons[i]));
        }
        started = true;
    }
    if started {
        buf.push_str(end);
    }
}

/// A help buffer built as C++ builds one (each line starts with `NEWLINE`), without its leading
/// line break.
pub(crate) fn finish(buf: String) -> String {
    match buf.strip_prefix(NEWLINE) {
        Some(rest) => rest.to_string(),
        None => buf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled() {
        assert_eq!(fill("[ICON_BULLET]%D1%% Tile Defense", &[Arg::N(25)]), "[ICON_BULLET]+25% Tile Defense");
        assert_eq!(fill("%D1% heal rate", &[Arg::N(-10)]), "-10% heal rate");
        assert_eq!(fill("+%d1%F2 in nearby cities", &[Arg::N(1), Arg::S("[ICON_HAPPY]")]), "+1[ICON_HAPPY] in nearby cities");
        assert_eq!(fill("Must be level %d1_Num to enter", &[Arg::N(15)]), "Must be level 15 to enter");
        assert_eq!(fill("Spawns %s1_unit barbarian units", &[Arg::S("Skeleton")]), "Spawns Skeleton barbarian units");
        assert_eq!(fill("by 75%, then %s2", &[]), "by 75%, then ");
        assert_eq!(fill("by a [COLOR_UNIT_TEXT]%s1_great_person[COLOR_REVERT]", &[Arg::S("Great Sage")]), "by a [COLOR_UNIT_TEXT]Great Sage[COLOR_REVERT]");
        assert_eq!(fill("%s1_Name_ and", &[Arg::S("X")]), "X_ and");
    }

    #[test]
    fn plural_forms_follow_the_number() {
        let t = "[ICON_BULLET]%D1_Change Trade [NUM1:Route:Routes] per City";
        assert_eq!(fill(t, &[Arg::N(1)]), "[ICON_BULLET]+1 Trade Route per City");
        assert_eq!(fill(t, &[Arg::N(2)]), "[ICON_BULLET]+2 Trade Routes per City");
        assert_eq!(fill(t, &[Arg::N(-1)]), "[ICON_BULLET]-1 Trade Route per City");
        // Not a plural form: kept.
        assert_eq!(fill("[NUMBER] [NUM1]", &[Arg::N(1)]), "[NUMBER] [NUM1]");
    }

    #[test]
    fn yield_changes_read_as_civ4_writes_them() {
        let mut b = String::new();
        changes_help(&mut b, "Farm", ": ", "", &[1, 0, 2], &YIELD_ICONS, false, true);
        assert_eq!(b, "[NEWLINE][ICON_BULLET]Farm: +1[ICON_FOOD], +2[ICON_COMMERCE]");
        let mut b = String::new();
        changes_help(&mut b, "", "", " in all cities", &[0, -10, 0, 0], &COMMERCE_ICONS, true, false);
        assert_eq!(b, "-10%[ICON_RESEARCH] in all cities");
        let mut b = String::new();
        changes_help(&mut b, "x", "", "end", &[0, 0, 0], &YIELD_ICONS, false, true);
        assert!(b.is_empty());
    }
}
