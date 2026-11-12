//! Keywords and reminder text (docs/PLAY_DESIGN.md, "Keywords and reminder text"; docs/HUMAN.md):
//! which keywords an entity has, and the `reminders` the view sends with its rules text.
//!
//! An entity's rules text is what is unique to it; a keyword's reminder text (`keywords.jsonc`)
//! is a rule that holds for every entity with the keyword, said once and shown beside the rules
//! text, never inside it. An entity's keywords come from its properties (every project is a
//! Ritual; a world ritual is a World Ritual) and from the marks in its texts (`[KW=KEYWORD_X]word[\KW]`). The page shows
//! the reminders the view sends and decides none itself.

use ffh_data::*;

use crate::json::Json;

/// Keyword `name` (`KEYWORD_RITUAL`), if the data has it.
fn kw(d: &GameData, name: &str) -> Option<KeywordType> {
    d.ty::<KeywordType>(name)
}

/// The keywords of project `p` by its properties: World Ritual (a `world_ritual`, which implies
/// Ritual) or Ritual (every project).
pub fn of_project(d: &GameData, p: ProjectType) -> Vec<KeywordType> {
    kw(d, if d.projects[p].world_ritual { "KEYWORD_WORLD_RITUAL" } else { "KEYWORD_RITUAL" }).into_iter().collect()
}

/// The keywords `text` marks (`[KW=KEYWORD_X]`), in the order of first mention.
pub fn in_text(d: &GameData, text: &str) -> Vec<KeywordType> {
    keyword_marks(text).into_iter().filter_map(|n| kw(d, n)).collect()
}

/// `keys` with the keywords each implies (transitively) after it, each once, in the order of
/// first mention.
pub fn expand(d: &GameData, keys: impl IntoIterator<Item = KeywordType>) -> Vec<KeywordType> {
    fn add(d: &GameData, k: KeywordType, out: &mut Vec<KeywordType>) {
        if out.contains(&k) {
            return;
        }
        out.push(k);
        for &i in &d.keywords[k].implies {
            add(d, i, out);
        }
    }
    let mut out = Vec::new();
    for k in keys {
        add(d, k, &mut out);
    }
    out
}

/// A keyword's name (`Ritual`).
pub fn name(d: &GameData, k: KeywordType) -> String {
    crate::names::text(d, &d.keywords[k].base.description).to_string()
}

/// A keyword's reminder text.
pub fn reminder(d: &GameData, k: KeywordType) -> String {
    crate::names::text(d, &d.keywords[k].reminder).to_string()
}

/// The `reminders` of an entity whose keywords are `keys` (properties first, then the marks of
/// its texts in reading order): `[{key, name, text, with}]`, each keyword once with the ones it
/// implies after it; `with`: the keys of the reminders a chip of this keyword shows (itself and
/// what it implies). An empty array for none.
pub fn reminders(d: &GameData, keys: impl IntoIterator<Item = KeywordType>) -> Json {
    Json::Arr(
        expand(d, keys)
            .into_iter()
            .map(|k| {
                let with: Vec<Json> = expand(d, [k]).into_iter().map(|w| Json::from(d.keywords[w].base.type_.as_str())).collect();
                crate::jobj! {
                    "key" => d.keywords[k].base.type_.as_str(),
                    "name" => name(d, k),
                    "text" => reminder(d, k),
                    "with" => Json::Arr(with),
                }
            })
            .collect(),
    )
}

/// The keywords of entry `i` of data table `kind` with rules text `texts` (its effects, help and
/// strategy): its properties' keywords, then the marks of the texts.
pub fn of_entry(d: &GameData, kind: &str, i: usize, texts: &[&str]) -> Vec<KeywordType> {
    let mut keys = match kind {
        "ProjectType" => of_project(d, ProjectType::from_usize(i)),
        _ => Vec::new(),
    };
    for t in texts {
        keys.extend(in_text(d, t));
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &Json) -> Vec<String> {
        list.as_arr().unwrap().iter().map(|r| r.get("key").and_then(Json::as_str).unwrap().to_string()).collect()
    }

    #[test]
    fn every_project_is_a_ritual_and_a_world_ritual_implies_it() {
        let d = ffh_data::global();
        for p in d.projects.keys() {
            let r = reminders(d, of_project(d, p));
            let k = keys(&r);
            assert!(k.contains(&"KEYWORD_RITUAL".to_string()), "{}: {k:?}", d.projects[p].base.type_);
            if d.projects[p].world_ritual {
                assert_eq!(k[..2], ["KEYWORD_WORLD_RITUAL".to_string(), "KEYWORD_RITUAL".to_string()], "{k:?}");
            }
        }
        let warcry = reminders(d, of_project(d, d.t::<ProjectType>("PROJECT_RITUAL_WARCRY")));
        assert_eq!(keys(&warcry), ["KEYWORD_WORLD_RITUAL", "KEYWORD_RITUAL"]);
        let first = &warcry.as_arr().unwrap()[0];
        assert_eq!(first.get("name").and_then(Json::as_str), Some("World Ritual"));
        let with: Vec<&str> = first.get("with").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
        assert_eq!(with, ["KEYWORD_WORLD_RITUAL", "KEYWORD_RITUAL"]);
        // A mark repeated after the properties adds nothing.
        let k = of_entry(d, "SpellType", d.t::<SpellType>("SPELL_WARCRY").idx(), &["a [KW=KEYWORD_RITUAL]ritual[\\KW], a [KW=KEYWORD_WORLD_RITUAL]x[\\KW]"]);
        assert_eq!(keys(&reminders(d, k)), ["KEYWORD_RITUAL", "KEYWORD_WORLD_RITUAL"]);
    }

    #[test]
    fn every_keyword_has_a_name_and_a_reminder() {
        let d = ffh_data::global();
        assert!(d.keywords.len() >= 2);
        for k in d.keywords.keys() {
            let (n, r) = (name(d, k), reminder(d, k));
            assert!(!n.starts_with("TXT_KEY") && !r.starts_with("TXT_KEY") && r.ends_with('.'), "{n}: {r}");
        }
    }
}
