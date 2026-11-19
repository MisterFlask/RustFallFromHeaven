//! Keywords and reminder text (docs/PLAY_DESIGN.md, "Keywords and reminder text"): an entity's
//! rules text does not restate a keyword's reminder text. Over every project's and every spell's
//! payload (the data's `effects`, `help` and `strategy`, the production list's ritual line) no
//! reminder sentence and no fragment listed for a keyword below appears; every project has its
//! reminders; every `[KW=...]` mark of the text names a keyword. The reference pages (the World
//! Spells concept, the help topic) keep their prose and are not checked.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::human::AdvisorAnswerer;
use ffh_play::{Json, Session, SessionOptions};

/// What each keyword's reminder says, as distinctive fragments a rules text must not hold
/// (lowercase; matched without case).
const FRAGMENTS: &[(&str, &[&str])] = &[
    ("KEYWORD_RITUAL", &["progress is lost if the city switches", "switches to building something else", "hurried with gold", "cannot be hurried", "production is lost", "first build", "off the head of the queue", "visible to every player"]),
    ("KEYWORD_WORLD_RITUAL", &["one in progress per player", "doubles the next cost", "doubles its next cost", "every player is told"]),
    // Houses (docs/PERSONS.md, "UI", part J; "The second build", rule 13: the loyalty states are
    // the attitude words, no keywords; "The third build", rule 2: Request).
    ("KEYWORD_HOUSE", &["a vassal civilization with a leader of its own", "its cities its own", "a house that breaks away declares war", "every vassal is a house"]),
    ("KEYWORD_CROWN", &["the cities a civilization holds itself", "always the crown's", "a realm's first grant founds one", "pay triple city-count maintenance"]),
    ("KEYWORD_REQUEST", &["a house's wish, one at a time", "with a term of 40 turns", "made impossible by its liege's act", "the next one comes 20 turns later"]),
    // ("The third build", rule 5.)
    ("KEYWORD_ULTIMATUM", &["a furious house's one demand", "answered at once", "binds its liege for 20 turns", "while it runs"]),
    ("KEYWORD_TRIBUTE", &["a share of its cities' gold and research", "the research added to the liege's", "1 less for each 10% more", "toward the rate set every", "follow the rate in force"]),
    ("KEYWORD_LEVY", &["an army's requisition in a city of its owner's house", "costs the house -1 loyalty", "annoyed or furious takes no levy"]),
    ("KEYWORD_CLAIMANT", &["a house with a city within", "granting the city to another house costs"]),
];

/// The forbidden pieces of every keyword: its fragments and each sentence of its reminder text.
fn forbidden(d: &GameData) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for k in d.keywords.keys() {
        let ty = d.keywords[k].base.type_.clone();
        let reminder = ffh_play::view::keywords::reminder(d, k);
        for s in reminder.split(". ").map(|s| s.trim_end_matches('.').to_lowercase()).filter(|s| s.len() > 20) {
            out.push((ty.clone(), s));
        }
        let listed = FRAGMENTS.iter().find(|(t, _)| *t == ty).unwrap_or_else(|| panic!("{ty}: list its fragments in FRAGMENTS"));
        out.extend(listed.1.iter().map(|f| (ty.clone(), f.to_string())));
    }
    out
}

fn check(what: &str, text: &str, bad: &[(String, String)]) {
    let lower = ffh_play::names::resolve_keys(ffh_data::global(), text).to_lowercase();
    for (k, f) in bad {
        assert!(!lower.contains(f.as_str()), "{what}: its rules text restates {k}'s reminder ({f:?}): {text}");
    }
}

fn table<'a>(data: &'a Json, kind: &str) -> &'a [Json] {
    data.get("tables").and_then(|t| t.get(kind)).and_then(Json::as_arr).unwrap_or(&[])
}

#[test]
fn rules_text_does_not_restate_reminder_text() {
    let d = ffh_data::global();
    let bad = forbidden(d);
    let data = ffh_play::view::data::data(d);
    for kind in ["ProjectType", "SpellType"] {
        let entries = table(data, kind);
        assert!(!entries.is_empty(), "{kind}");
        for e in entries {
            let ty = e.get("type").and_then(Json::as_str).unwrap_or("?");
            for field in ["effects", "help", "strategy"] {
                if let Some(t) = e.get(field).and_then(Json::as_str) {
                    check(&format!("{ty} {field}"), t, &bad);
                }
            }
            if kind == "ProjectType" {
                let keys: Vec<&str> = e.get("reminders").and_then(Json::as_arr).unwrap_or(&[]).iter().filter_map(|r| r.get("key").and_then(Json::as_str)).collect();
                assert!(keys.contains(&"KEYWORD_RITUAL"), "{ty}: every project is a Ritual: {keys:?}");
            }
        }
    }
    // The production list's line of every world ritual, in a city of a steady game.
    let ng = common::steady_duel(5);
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    s.advance(&mut |_| {});
    let h = s.human;
    let (x, y) = s.g.players[h].units.iter().map(|&u| (s.g.units[u].x, s.g.units[u].y)).next().unwrap();
    let c = s.g.player_init_city(h, x, y, true, true);
    for p in d.projects.keys().filter(|&p| d.projects[p].world_ritual) {
        let line = ffh_play::view::rituals::rules_line(&s.g, c, p);
        check(&format!("{} production line", d.projects[p].base.type_), &line, &bad);
        assert!(line.contains("Next cost: "), "{line}");
    }
    let _: CityId = c;
}

#[test]
fn every_keyword_mark_names_a_keyword() {
    let d = ffh_data::global();
    assert!(!d.warnings.iter().any(|w| w.contains("[KW=")), "{:?}", d.warnings);
    let mut marks = 0;
    for (key, text) in d.text.entries() {
        for name in keyword_marks(text) {
            marks += 1;
            assert!(d.ty::<KeywordType>(name).is_some(), "{key}: [KW={name}] names no keyword");
        }
    }
    assert!(marks >= 2, "the World Spells concept marks its keywords: {marks}");
}

/// Every string of an object (but its `reminders` and a House leader's `backstory`, flavour), deep.
fn strings_of(j: &Json, out: &mut Vec<String>) {
    match j {
        Json::Str(s) => out.push(s.clone()),
        Json::Arr(a) => a.iter().for_each(|x| strings_of(x, out)),
        Json::Obj(o) => o.iter().filter(|(k, _)| k != "reminders" && k != "backstory").for_each(|(_, v)| strings_of(v, out)),
        _ => {}
    }
}

/// Houses (docs/PERSONS.md, "UI", part J): the Court's and every grant list's lines, on the staged
/// scene (and the founding's, the human's Houses freed), do not restate a keyword's reminder.
#[test]
fn the_court_does_not_restate_reminder_text() {
    let bad = forbidden(ffh_data::global());
    let dir = common::TempDir::new("keywords-persons");
    let st = ffh_play::stage::persons(&dir.0, false).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    let mut s = Session::load(&bytes, sidecar.as_ref(), Arc::new(AdvisorAnswerer), opts).unwrap();
    let mut texts = Vec::new();
    strings_of(&s.query(&ffh_play::query::Query::Court).unwrap(), &mut texts);
    for c in s.g.players[s.human].cities.clone() {
        strings_of(&s.query(&ffh_play::query::Query::Grant { city: c }).unwrap(), &mut texts);
    }
    // The founding (a realm without a House): the Court's line and the grant list's card.
    for hs in st.houses.iter().filter(|x| x.role != "neighbour") {
        s.g.player_free_house(st.human, hs.id).unwrap();
    }
    strings_of(&s.query(&ffh_play::query::Query::Court).unwrap(), &mut texts);
    strings_of(&s.query(&ffh_play::query::Query::Grant { city: st.crown[1] }).unwrap(), &mut texts);
    assert!(texts.len() > 100, "{}", texts.len());
    for t in &texts {
        check("persons payload", t, &bad);
    }
}
