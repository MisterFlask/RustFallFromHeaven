//! This game's own info class for keywords (not FFH2's; docs/PLAY_DESIGN.md, "Keywords and
//! reminder text"): `keywords.jsonc`. A keyword names a rule that holds for every entity it
//! applies to (every project is a Ritual); its reminder text says the rule once, shown beside an
//! entity's own rules text, never inside it. A text marks a keyword with `[KW=KEYWORD_X]word[\KW]`.

use super::InfoBase;
use crate::define_idx;
use crate::registry::Rd;

define_idx!(/// A keyword (`KEYWORD_*`, keywords.jsonc).
KeywordType);

/// A keyword: its name (`base.description`), its reminder text and the keywords whose reminders
/// are shown with it.
#[derive(Debug, Clone, Default)]
pub struct KeywordInfo {
    pub base: InfoBase,
    /// Text key of the reminder text: the rule the keyword stands for.
    pub reminder: String,
    /// The keywords this one implies (a World Ritual is a Ritual): their reminders are shown with
    /// it.
    pub implies: Vec<KeywordType>,
}

pub fn read_keyword(rd: &Rd) -> KeywordInfo {
    KeywordInfo { base: InfoBase::read(rd), reminder: rd.string("reminder"), implies: rd.ref_list::<KeywordType>("implies") }
}

/// The keywords `text` marks (`[KW=KEYWORD_X]`), by type name, in the order of first mention,
/// each once.
pub fn keyword_marks(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("[KW=") {
        let tail = &rest[i + 4..];
        let Some(end) = tail.find(']') else { break };
        let name = &tail[..end];
        if !out.contains(&name) {
            out.push(name);
        }
        rest = &tail[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::keyword_marks;

    #[test]
    fn marks_in_order_of_first_mention() {
        assert_eq!(keyword_marks("Leaves a [KW=KEYWORD_CHARGE]charge[\\KW]; a [KW=KEYWORD_RITUAL]ritual[\\KW], [KW=KEYWORD_CHARGE]x[\\KW]"), vec!["KEYWORD_CHARGE", "KEYWORD_RITUAL"]);
        assert!(keyword_marks("no marks [LINK=X]y[\\LINK]").is_empty());
        assert!(keyword_marks("[KW=broken").is_empty());
    }
}
