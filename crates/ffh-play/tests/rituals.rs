//! World rituals in the views (docs/CARDS.md, "World rituals"; docs/HUMAN.md): the view's
//! `rituals` rows, the world-spell button's words in each state, the production list's ritual line
//! and its reasons, the city's abandon question, and the ritual news marked in the log. The staged
//! games (`ffh-play stage rituals|rituals-charged`) and hand-built situations in games that cannot
//! end (`common::STEADY`); nothing here depends on how a seed's game goes.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdOk, Command};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;
use ffh_play::{Json, Session, SessionOptions, Stop};

fn arr<'a>(j: &'a Json, k: &str) -> &'a [Json] {
    j.get(k).and_then(Json::as_arr).unwrap_or(&[])
}

fn s_of<'a>(j: &'a Json, k: &str) -> Option<&'a str> {
    j.get(k).and_then(Json::as_str)
}

fn i_of(j: &Json, k: &str) -> Option<i64> {
    j.get(k).and_then(Json::as_i64)
}

fn load(save: &std::path::Path) -> Session {
    let (bytes, sidecar) = ffh_play::save::read_bundle(save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    Session::load(&bytes, sidecar.as_ref(), Arc::new(AdvisorAnswerer), opts).unwrap()
}

fn row(view: &Json, p: PlayerId) -> Json {
    arr(view, "rituals").iter().find(|r| i_of(r, "player") == Some(p.0 as i64)).cloned().unwrap_or_else(|| panic!("no row for player {}", p.0))
}

fn world_action(s: &Session, spell: SpellType) -> Option<Json> {
    let view = s.query(&Query::View).unwrap();
    arr(view.get("me").unwrap(), "world_actions").iter().find(|a| i_of(a, "spell") == Some(spell.idx() as i64)).cloned()
}

fn ritual_entry(s: &Session, c: CityId, project: ProjectType) -> Json {
    let city = s.query(&Query::City { city: c }).unwrap();
    arr(&city, "buildable").iter().find(|b| i_of(b, "order") == Some(OrderType::ORDER_CREATE.idx() as i64) && i_of(b, "type") == Some(project.idx() as i64)).cloned().expect("the ritual is listed")
}

fn city_name(s: &Session, c: CityId) -> String {
    ffh_play::view::city_name(&s.g, c)
}

/// A steady duel as `civ` at the human's first control point, its capital founded.
fn duel_as(seed: u32, civ: &str) -> Session {
    let d = ffh_data::global();
    let mut ng = common::steady_duel(seed);
    ng.civ = Some(d.t::<CivilizationType>(civ));
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).unwrap();
    let r = s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    s
}

/// `ffh-play stage rituals`: the rows as the human may know them (met and unmet, in progress,
/// charged, idle with its cost doubled), the news in the log, the capital's abandon question.
#[test]
fn the_staged_rituals_game_shows_every_civilizations_ritual() {
    let dir = common::TempDir::new("stage-rituals");
    let st = ffh_play::stage::rituals(&dir.0).expect("staged");
    assert_eq!(s_of(&st.to_json(), "stage"), Some("rituals"));
    let s = load(&st.save);
    let g = &s.g;
    let h = s.human;
    let view = s.query(&Query::View).unwrap();
    let rows = arr(&view, "rituals");
    assert_eq!(rows.len(), 5, "one row per civilization: {rows:?}");

    let mine = row(&view, h);
    assert_eq!(mine.get("mine").and_then(Json::as_bool), Some(true));
    assert_eq!(s_of(&mine, "name"), Some("Hippus"));
    assert_eq!(s_of(&mine, "ritual_name"), Some("Ritual: Warcry"));
    assert_eq!(s_of(&mine, "state"), Some("in_progress"));
    assert_eq!(i_of(&mine, "city"), Some(st.capital.0 as i64));
    let (progress, cost) = (i_of(&mine, "progress").unwrap(), i_of(&mine, "cost").unwrap());
    assert!(progress > 0 && progress < cost, "{mine:?}");
    assert_eq!(i_of(&mine, "percent"), Some(progress * 100 / cost));
    assert!(i_of(&mine, "turns_left").is_some_and(|n| n > 0), "{mine:?}");

    let civ = |p: PlayerId| ffh_play::names::resolve_keys(g.d, &g.player_get_civilization_short_description(p)).into_owned();
    let going = row(&view, st.in_progress);
    assert_eq!(going.get("met").and_then(Json::as_bool), Some(true));
    assert_eq!(s_of(&going, "name"), Some(civ(st.in_progress).as_str()));
    assert_eq!(s_of(&going, "state"), Some("in_progress"));
    assert!(i_of(&going, "progress").unwrap() > 0 && i_of(&going, "turns_left").is_some(), "{going:?}");

    let held = row(&view, st.charged);
    assert_eq!(s_of(&held, "state"), Some("charged"));
    assert_eq!(held.get("charged").and_then(Json::as_bool), Some(true));
    assert!(s_of(&held, "city_name").is_some() && i_of(&held, "x").is_some(), "{held:?}");

    let idle = row(&view, st.idle);
    assert_eq!(s_of(&idle, "state"), Some("idle"));
    assert_eq!(i_of(&idle, "completed"), Some(1));
    assert_eq!(idle.get("city"), Some(&Json::Null));
    let project = g.player_world_ritual(st.idle).unwrap();
    let base = g.player_project_production_needed_at(st.idle, project, -1) as i64;
    let doubled = i_of(&idle, "cost").unwrap();
    assert!((doubled - 2 * base).abs() <= 1, "doubled: {doubled} of {base}");
    assert_eq!(i_of(&idle, "next_cost"), Some(g.player_project_production_needed_at(st.idle, project, 1) as i64));

    // Not met: named for no one, its city shown (the start is announced to everyone).
    let unmet = row(&view, st.unmet);
    assert_eq!(unmet.get("met").and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(&unmet, "name"), Some("Unknown civilization"));
    assert_eq!(unmet.get("civ"), Some(&Json::Null));
    assert_eq!(s_of(&unmet, "state"), Some("in_progress"));
    let c = g.player_get_capital_city(st.unmet).unwrap();
    assert_eq!(s_of(&unmet, "city_name"), Some(city_name(&s, c).as_str()));

    // The news of each start, marked as ritual news in the log.
    let log = s.query(&Query::Log { since: 0 }).unwrap();
    let began: Vec<&Json> = arr(&log, "entries").iter().filter(|e| s_of(e, "kind") == Some("ritual") && s_of(e, "verb") == Some("began")).collect();
    assert_eq!(began.len(), 3, "{log:?}");
    assert!(began.iter().all(|e| e.get("at").and_then(Json::as_arr).is_some_and(|a| a.len() == 2)));
    let capital = city_name(&s, st.capital);
    assert!(began.iter().any(|e| s_of(e, "text") == Some(format!("Hippus began Ritual: Warcry in {capital}.").as_str())), "{began:?}");

    // The capital's ritual in progress: the abandon question, no hurrying.
    let city = s.query(&Query::City { city: st.capital }).unwrap();
    let head = city.get("ritual_head").unwrap();
    assert_eq!(s_of(head, "confirm"), Some(format!("Abandon Ritual: Warcry? Its {progress} production is lost.").as_str()));
    assert_eq!(s_of(&city, "hurry_note"), Some("A world ritual cannot be hurried."));
    let second = s.query(&Query::City { city: st.second }).unwrap();
    assert_eq!(second.get("ritual_head"), Some(&Json::Null));
    assert_eq!(second.get("hurry_note"), Some(&Json::Null));

    // The second city cannot start it: in progress in the capital.
    let entry = ritual_entry(&s, st.second, project_of(&s, h));
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "why")), Some(format!("In progress in {capital}.").as_str()));
}

fn project_of(s: &Session, p: PlayerId) -> ProjectType {
    s.g.player_world_ritual(p).unwrap()
}

/// `ffh-play stage rituals-charged`: the human holds its charge; the ritual cannot be started.
#[test]
fn the_staged_charged_game_holds_a_charge() {
    let dir = common::TempDir::new("stage-rituals-charged");
    let st = ffh_play::stage::rituals_charged(&dir.0).expect("staged");
    assert_eq!(s_of(&st.to_json(), "stage"), Some("rituals-charged"));
    let s = load(&st.save);
    let h = s.human;
    let view = s.query(&Query::View).unwrap();
    let mine = row(&view, h);
    assert_eq!(s_of(&mine, "state"), Some("charged"));
    assert_eq!(i_of(&mine, "city"), Some(st.capital.0 as i64));
    let project = project_of(&s, h);
    // Every city of the owner lists it, refused: the one holding the charge too.
    for c in [st.second, st.capital] {
        let entry = ritual_entry(&s, c, project);
        assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
        assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "why")), Some("Charge held: cast Warcry first."));
    }
    let city = s.query(&Query::City { city: st.capital }).unwrap();
    assert_eq!(city.get("ritual_head"), Some(&Json::Null));
}

/// The top bar's world spell in each state: no charge and nothing queued, the ritual in progress,
/// a charge held and castable, a charge held with no unit to cast it.
#[test]
fn the_world_spell_button_says_where_the_ritual_stands() {
    let mut s = duel_as(33, "CIVILIZATION_HIPPUS");
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_WARCRY");
    let project = d.t::<ProjectType>("PROJECT_RITUAL_WARCRY");
    let capital = s.g.player_get_capital_city(h).unwrap();
    let name = city_name(&s, capital);
    let cost = s.g.player_get_production_needed_project(h, project);
    let why = |s: &Session| world_action(s, spell).and_then(|a| s_of(&a, "why").map(str::to_string));

    assert_eq!(why(&s).as_deref(), Some(format!("No charge. Build Ritual: Warcry in a city (cost {cost}).").as_str()));

    // In progress: the production list's entry first, with its rules line.
    let entry = ritual_entry(&s, capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(true));
    let next = s.g.player_project_production_needed_at(h, project, 1);
    let rules = format!("World ritual. One in progress per player. Taken off the head of the queue, its production is lost. Cannot be hurried. Each completion doubles the next cost (next: {next}).");
    assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "rules")), Some(rules.as_str()));
    assert_eq!(entry.get("ritual").and_then(|r| r.get("why")), Some(&Json::Null));
    let r = s.execute(&Command::PushOrder { city: capital, order: OrderType::ORDER_CREATE, data: project.idx() as i32, alt: false, shift: false, ctrl: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    s.g.city_add_turn_production(capital, 7);
    let turns = s.g.city_production_turns_left_project(capital, project, 0);
    let turns = if turns == 1 { "1 turn".to_string() } else { format!("{turns} turns") };
    assert_eq!(why(&s).as_deref(), Some(format!("No charge. Ritual: Warcry: 7/{cost} in {name}, {turns}.").as_str()));
    let city = s.query(&Query::City { city: capital }).unwrap();
    assert_eq!(city.get("ritual_head").and_then(|r| s_of(r, "confirm")), Some("Abandon Ritual: Warcry? Its 7 production is lost."));
    assert!(arr(&city, "hurries").is_empty());

    // Taken off the queue (the city builds a warrior instead), then a charge held: castable.
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let r = s.execute(&Command::PushOrder { city: capital, order: OrderType::ORDER_TRAIN, data: warrior.idx() as i32, alt: false, shift: false, ctrl: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    assert_eq!(s.g.cities[capital].project_production[project], 0, "abandoned");
    assert_eq!(s.g.player_grant_world_ritual_charge(h), Some(capital));
    let a = world_action(&s, spell).unwrap();
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(true), "{a:?}");
    assert_eq!(s_of(&a, "charge"), Some(format!("Charge held in {name}. Casting spends it. Next ritual cost: {cost}.").as_str()));

    // No unit left to cast it: still shown, disabled, with the reason.
    for u in s.g.players[h].units.clone() {
        s.g.unit_kill(u, false, None);
    }
    let a = world_action(&s, spell).expect("still listed");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(&a, "why"), Some("No unit of yours can cast it now."));
    assert!(a.get("charge").is_none(), "{a:?}");
}

/// The Sanctuary's ritual while the Sanctuary holds, and the reason the list does not know.
#[test]
fn the_production_list_says_why_a_ritual_cannot_start() {
    let mut s = duel_as(34, "CIVILIZATION_ELOHIM");
    let h = s.human;
    let project = project_of(&s, h);
    let capital = s.g.player_get_capital_city(h).unwrap();
    assert_eq!(ffh_play::view::rituals::cannot_reason(&s.g, capital, project), "Not available now.");
    s.g.players[h].sanctuary_timer = 3;
    let entry = ritual_entry(&s, capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "why")), Some("Sanctuary holds: 3 turns."));
}

/// `ffh-play stage tithe`: Ritual: The Tithe is listed, disabled, until a rival is named; Name the
/// Tithe's targets say each rival's civilization and war state and carry the question asked
/// before the cast; once named, the entry and the human's ritual row say who.
#[test]
fn the_tithe_says_who_is_named_and_asks_before_the_war() {
    let dir = common::TempDir::new("stage-tithe");
    let st = ffh_play::stage::tithe(&dir.0).expect("staged");
    assert_eq!(s_of(&st.to_json(), "stage"), Some("tithe"));
    let mut s = load(&st.save);
    let d = s.g.d;
    let h = s.human;
    let project = d.t::<ProjectType>("PROJECT_RITUAL_THE_TITHE");
    let spell = d.t::<SpellType>("SPELL_NAME_THE_TITHE");
    let civ = |s: &Session, p: PlayerId| ffh_play::names::resolve_keys(d, &s.g.player_get_civilization_short_description(p)).into_owned();

    // Not named: listed, disabled, the reason.
    let entry = ritual_entry(&s, st.capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
    let ritual = entry.get("ritual").unwrap();
    assert_eq!(s_of(ritual, "why"), Some("No rival named: use Name the Tithe."));
    assert!(ritual.get("note").is_some_and(Json::is_null), "{ritual:?}");

    // The targets: one per rival met, with its war state and the question.
    let action = world_action(&s, spell).expect("Name the Tithe is a world action");
    let targets = arr(&action, "targets");
    assert_eq!(targets.len(), 3, "{targets:?}");
    let target = |p: PlayerId| targets.iter().find(|t| s_of(t, "label").is_some_and(|l| l.contains(&civ(&s, p)))).cloned().unwrap_or_else(|| panic!("no target for {}", p.0));
    let war = target(st.war);
    assert!(s_of(&war, "label").unwrap().ends_with(&format!(": {}, at war", civ(&s, st.war))), "{war:?}");
    assert_eq!(s_of(war.get("confirm").unwrap(), "text"), Some(format!("Name {}?", civ(&s, st.war)).as_str()));
    let peace = target(st.peace[0]);
    let name = civ(&s, st.peace[0]);
    assert!(s_of(&peace, "label").unwrap().ends_with(&format!(": {name}, at peace")), "{peace:?}");
    let confirm = peace.get("confirm").unwrap();
    assert_eq!(s_of(confirm, "title"), Some("Name the Tithe"));
    assert_eq!(s_of(confirm, "text"), Some(format!("Name {name}? This declares war on {name}.").as_str()));
    assert_eq!(s_of(confirm, "ok"), Some("Name"));

    // Named: the entry is enabled and says who; so does the human's row.
    s.g.players[h].tithe_target = Some(st.peace[0]);
    let entry = ritual_entry(&s, st.capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(true));
    let ritual = entry.get("ritual").unwrap();
    assert_eq!(s_of(ritual, "note"), Some(format!("Named rival: {name}.").as_str()));
    assert!(ritual.get("why").is_some_and(Json::is_null));
    let view = s.query(&Query::View).unwrap();
    assert_eq!(s_of(&row(&view, h), "note"), Some(format!("Named rival: {name}.").as_str()));
}
