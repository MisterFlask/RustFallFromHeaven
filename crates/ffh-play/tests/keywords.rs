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
    // The Armageddon counter (docs/CARDS.md, section 4).
    ("KEYWORD_ARMAGEDDON_COUNTER", &["100 over 300 turns", "raise or lower it"]),
    ("KEYWORD_ARMAGEDDON_CARD", &["one card or none at each slot", "within 5 of its slot", "reaches its slot"]),
    // This game's Houses (docs/HOUSES.md, rule 15; the cards' lines are checked in
    // tests/houses_screens.rs).
    ("KEYWORD_HOUSE", &["up to 4 of a civilization's cities", "whose seat is nearest"]),
    ("KEYWORD_SYMPATHY", &["the civilization a house's leader favours", "kept for good"]),
    ("KEYWORD_OUTSIDER", &["offered for a foreign house", "counts alignment and religion only"]),
    // Trade (docs/TRADE.md, section 9).
    ("KEYWORD_ROUTE", &["two caravans travel it", "each pays while it lives", "the destination of any number", "re-forms in the origin"]),
    // Engagement combat (docs/ENGAGEMENTS.md, "The rules"): the unit card's action kinds, attack
    // patterns, splash and large units.
    ("KEYWORD_STRIKE", &["front unit of an enemy lane", "pushes the target back"]),
    ("KEYWORD_CHARGE", &["where flanking works", "highest threat for its hp left"]),
    ("KEYWORD_STAB", &["hits any enemy"]),
    ("KEYWORD_SHOOT", &["back unit of an enemy lane", "lanes within one of its own first"]),
    ("KEYWORD_SPELL", &["never misses or crits", "against the target's magic defense"]),
    ("KEYWORD_BOMBARD", &["a city's sentinels first", "never crits"]),
    ("KEYWORD_HEAL", &["the most hurt ally", "the action is spent"]),
    ("KEYWORD_VOLLEY", &["in each lane beside the target's"]),
    ("KEYWORD_PIERCE", &["every other unit in the target's lane"]),
    ("KEYWORD_SWEEP", &["every other unit in the target's row"]),
    ("KEYWORD_CLEAVE", &["front unit of each lane beside"]),
    ("KEYWORD_TRAMPLE", &["nearest unit behind the target"]),
    ("KEYWORD_SPLASH", &["beside, in front of and behind"]),
    ("KEYWORD_LARGE", &["fills two cells", "counts 2 places"]),
    ("KEYWORD_FIRST_STRIKE", &["speed in a battle", "rolled at the battle's start"]),
    ("KEYWORD_WITHDRAWAL", &["rolls its withdrawal chance", "leaves the battle alive"]),
    ("KEYWORD_COMBAT_LIMIT", &["never take a target below", "does not aim at a target already there"]),
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
