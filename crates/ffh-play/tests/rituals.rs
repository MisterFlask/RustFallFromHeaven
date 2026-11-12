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

    // The capital's ritual in progress: the abandon question.
    let city = s.query(&Query::City { city: st.capital }).unwrap();
    let head = city.get("ritual_head").unwrap();
    assert_eq!(s_of(head, "confirm"), Some(format!("Abandon Ritual: Warcry? Its {progress} production is lost.").as_str()));
    let second = s.query(&Query::City { city: st.second }).unwrap();
    assert_eq!(second.get("ritual_head"), Some(&Json::Null));

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
    let rules = format!("Leaves a charge of Warcry. Next cost: {next}.");
    assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "rules")), Some(rules.as_str()));
    // The keywords' rules are its reminders (chips), not its rules line.
    let keys: Vec<&str> = entry.get("ritual").map(|r| arr(r, "reminders")).unwrap_or(&[]).iter().filter_map(|k| s_of(k, "key")).collect();
    assert_eq!(keys, ["KEYWORD_WORLD_RITUAL", "KEYWORD_RITUAL"]);
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

/// `ffh-play stage ritual-civ:CIVILIZATION_AMURITES`: Wellspring is placed by the player, no unit
/// casting it: the world action carries the placement (its prompts, every valid plot with every
/// mana type, labelled by the mana alone) and a `place` command; no caster.
#[test]
fn wellspring_is_placed_by_plot_then_mana_type() {
    let dir = common::TempDir::new("stage-ritual-civ-amurites");
    let st = ffh_play::stage::ritual_civ(&dir.0, "CIVILIZATION_AMURITES").expect("staged");
    let s = load(&st.save);
    let d = s.g.d;
    let spell = d.t::<SpellType>("SPELL_WELLSPRING");
    let a = world_action(&s, spell).expect("Wellspring is listed");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(true), "{a:?}");
    assert_eq!(a.get("place_on_charge").and_then(Json::as_bool), Some(true));
    assert!(a.get("caster").is_some_and(Json::is_null), "no caster");
    assert_eq!(s_of(&a, "target_kind"), Some("choice"));
    let place = a.get("place").expect("the placement");
    assert_eq!(s_of(place, "label"), Some("Place Wellspring"));
    assert_eq!(s_of(place, "completed"), Some("Ritual: Wellspring completed. Place the mana node: click a marked plot."));
    assert_eq!(s_of(place, "pick"), Some("Place the mana node of Wellspring: click a marked plot."));
    assert_eq!(s_of(place, "later"), Some("Esc: later."));
    assert_eq!(a.get("cmd").and_then(|c| s_of(c, "c")), Some("place"));
    let charge = s_of(&a, "charge").unwrap();
    assert!(charge.contains("Placing spends it."), "{charge}");
    let manas: Vec<String> = ffh_core::ffh::world_ritual_spells::small::wellspring_manas(d).iter().map(|&(b, _)| d.text.name(&d.bonuses[b].base.description).to_string()).collect();
    let targets = arr(&a, "targets");
    assert!(!targets.is_empty());
    let mut plots = std::collections::BTreeMap::<(i64, i64), Vec<String>>::new();
    for t in targets {
        let at = arr(t, "at");
        assert_eq!(at.len(), 2, "{t:?}");
        let label = s_of(t, "label").unwrap().to_string();
        assert!(manas.contains(&label), "a mana name alone: {label}");
        plots.entry((at[0].as_i64().unwrap(), at[1].as_i64().unwrap())).or_default().push(label);
    }
    for ((x, y), labels) in &plots {
        let q = s.g.map.plot_idx(*x as i32, *y as i32).unwrap();
        assert!(ffh_core::ffh::world_ritual_spells::small::wellspring_plot_ok(&s.g, s.human, q), "({x}, {y})");
        assert_eq!(labels, &manas, "every mana at ({x}, {y}), in the data's order");
    }
    // The tooltip's rules: placed, no caster, a mana type.
    let help = ffh_play::view::help::spell::effects(d, spell);
    assert!(help.contains("Spends a charge of its [KW=KEYWORD_WORLD_RITUAL]world ritual[\\KW], Ritual: Wellspring; placed when it completes."), "{help}");
    assert!(help.contains("The charge is held until the node is placed."), "{help}");
    assert!(help.contains("Choose a mana type."), "{help}");
    for no in ["Cast by", "caster", "Can only be used by", "option"] {
        assert!(!help.contains(no), "{no:?} in {help}");
    }
}

/// A row of the `place_menu` reply for mana `name` (its `name`).
fn menu_row<'a>(menu: &'a Json, name: &str) -> &'a Json {
    arr(menu, "rows").iter().find(|r| s_of(r, "name") == Some(name)).unwrap_or_else(|| panic!("no row {name}"))
}

/// Wellspring's menu on a plot (`place_menu`) in the staged game (Body 1, Chaos 2, Earth 3 owned,
/// roads everywhere, an Earth Elemental): a compact row per mana type: the count before and after,
/// the sphere's promotions in tier order with their spells from the data, one "now" line by
/// priority, the effect chips where they apply; every passive sentence in the info chip. Then a
/// row placed with no unit at all.
#[test]
fn wellspring_menu_lists_each_mana_type_compactly() {
    let dir = common::TempDir::new("stage-ritual-civ-amurites-menu");
    let st = ffh_play::stage::ritual_civ(&dir.0, "CIVILIZATION_AMURITES").expect("staged");
    let mut s = load(&st.save);
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_WELLSPRING");
    let a = world_action(&s, spell).unwrap();
    let cap = (s.g.cities[st.capital].x, s.g.cities[st.capital].y);
    // The plot nearest the capital (connected by the stage's roads).
    let (x, y) = arr(&a, "targets")
        .iter()
        .map(|t| { let at = arr(t, "at"); (at[0].as_i64().unwrap() as i32, at[1].as_i64().unwrap() as i32) })
        .min_by_key(|&(x, y)| (s.g.plot_distance(x, y, cap.0, cap.1), x, y))
        .unwrap();
    let menu = s.query(&Query::PlaceMenu { spell, x, y }).unwrap();
    assert_eq!(s_of(&menu, "title"), Some("Wellspring"));
    assert_eq!(s_of(&menu, "intro"), Some("Choose a mana type for the marked plot. I, II, III: the spells of each tier for Arcane Units."));
    let manas = ffh_core::ffh::world_ritual_spells::small::wellspring_manas(d);
    assert_eq!(arr(&menu, "rows").len(), manas.len(), "every mana type");
    let names: Vec<&str> = arr(&menu, "rows").iter().filter_map(|r| s_of(r, "name")).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "by name");
    let v = ffh_play::view::Viewer::new(&s.g, h, false);
    let owned_of = |name: &str| s.g.player_get_num_available_bonuses(h, d.t::<BonusType>(name));
    assert_eq!((owned_of("BONUS_MANA_BODY"), owned_of("BONUS_MANA_CHAOS"), owned_of("BONUS_MANA_EARTH")), (1, 2, 3), "the stage's nodes");
    for &(b, _) in &manas {
        let name = d.text.name(&d.bonuses[b].base.description).to_string();
        let row = menu_row(&menu, &name);
        assert_eq!(i_of(row, "bonus"), Some(b.idx() as i64));
        let owned = s.g.player_get_num_available_bonuses(h, b) as i64;
        assert_eq!(i_of(row, "owned"), Some(owned));
        assert_eq!(i_of(row, "after"), Some(owned + 1), "{name}: a node next to the capital counts");
        assert_eq!(s_of(row, "count"), Some(format!("{owned} → {}", owned + 1).as_str()));
        assert_eq!(s_of(row, "count_tip"), Some(format!("You own {owned} {name}. After placing: {}.", owned + 1).as_str()));
        let passive: Vec<&str> = arr(row, "passive").iter().filter_map(Json::as_str).collect();
        assert_eq!(passive, ffh_play::view::place::passive_lines(&v, b), "{name}");
        assert!(!passive.iter().any(|l| l.contains("Knowledge of the Ether")), "no tech line: the ritual needs the tech: {passive:?}");
        assert!(passive.contains(&ffh_play::view::place::free_promotions_line(d, b).unwrap().as_str()), "{name}");
        // The sphere promotions in tier order, each with its spells from the data.
        let tiers = ffh_core::ffh::events::adept_mana_promotions(d, b).expect("a sphere");
        let promos = arr(row, "promotions");
        assert_eq!(promos.iter().filter_map(|p| i_of(p, "promotion")).collect::<Vec<_>>(), tiers.iter().map(|p| p.idx() as i64).collect::<Vec<_>>(), "{name}");
        assert_eq!(promos.iter().filter_map(|p| s_of(p, "tier")).collect::<Vec<_>>(), ["I", "II", "III"]);
        assert_eq!(d.promotions[tiers[0]].bonus_prereq, Some(b), "tier I needs the mana");
        for (p, &t) in promos.iter().zip(&tiers) {
            let want: Vec<SpellType> = d.spells.keys().filter(|&sp| d.spells[sp].promotion_prereq1 == Some(t) || d.spells[sp].promotion_prereq2 == Some(t)).collect();
            assert!(!want.is_empty(), "{name}: every tier opens a spell");
            // One entry per spell name (Life III's two Resurrections are one).
            let mut want_named: Vec<SpellType> = Vec::new();
            for &sp in &want {
                if !want_named.iter().any(|&o| d.text.name(&d.spells[o].base.description) == d.text.name(&d.spells[sp].base.description)) {
                    want_named.push(sp);
                }
            }
            let want = want_named;
            let spells = arr(p, "spells");
            assert_eq!(spells.iter().filter_map(|x| i_of(x, "spell")).collect::<Vec<_>>(), want.iter().map(|x| x.idx() as i64).collect::<Vec<_>>(), "{name}");
            for (j, &sp) in spells.iter().zip(&want) {
                let req: Vec<String> = [d.spells[sp].promotion_prereq1, d.spells[sp].promotion_prereq2].into_iter().flatten().map(|q| d.text.name(&d.promotions[q].base.description).to_string()).collect();
                assert_eq!(s_of(j, "needs"), Some(format!("Needs {}.", req.join(" and ")).as_str()), "{j:?}");
                assert!(!s_of(j, "help").unwrap().contains("Requires the "), "the requirement is in `needs`: {j:?}");
            }
            assert!(s_of(p, "needs").is_some_and(|n| n.starts_with("Needs ")), "{p:?}");
            assert!(s_of(p, "free").is_some_and(|n| n.starts_with("Free to new Arcane Units with ")), "{p:?}");
        }
        // Effect chips: health and happiness from the data, the info chip always, last.
        let chips = arr(row, "chips");
        let kinds: Vec<&str> = chips.iter().filter_map(|c| s_of(c, "kind")).collect();
        assert_eq!(kinds.contains(&"health"), d.bonuses[b].health != 0, "{name}: {kinds:?}");
        assert_eq!(kinds.contains(&"happiness"), d.bonuses[b].happiness != 0, "{name}: {kinds:?}");
        assert_eq!(kinds.last(), Some(&"info"));
        let info = chips.last().unwrap();
        assert_eq!(arr(info, "tip").iter().filter_map(Json::as_str).collect::<Vec<_>>(), passive, "{name}: every passive line in the info chip");
        // Affinity: only for a mana a unit of the human's has one for, or one it can build.
        let ours = s.g.players[h].units.iter().any(|&u| s.g.units[u].bonus_affinity[b] != 0);
        let can = d.units.keys().any(|u| d.units[u].bonus_affinity[b] != 0 && s.g.player_can_train(h, u, false, false, false));
        assert_eq!(kinds.contains(&"affinity"), ours || can, "{name}: {kinds:?}");
        for c in chips {
            assert!(s_of(c, "aria").is_some_and(|a| !a.is_empty()), "{c:?}");
        }
    }
    // Life's health and Enchantment's happiness chips, from the data.
    let chip = |row: &Json, kind: &str| arr(row, "chips").iter().find(|c| s_of(c, "kind") == Some(kind)).cloned();
    let life = chip(menu_row(&menu, "Life Mana"), "health").expect("Life's health chip");
    assert_eq!(s_of(&life, "label"), Some("+1"));
    assert_eq!(arr(&life, "tip")[0].as_str(), Some("+1 health in each of your cities that has it."));
    assert_eq!(chip(menu_row(&menu, "Enchantment Mana"), "happiness").and_then(|c| s_of(&c, "label").map(str::to_string)).as_deref(), Some("+1"));
    // Earth: the staged Earth Elemental's affinity; Fire: no unit of the human's has one.
    let earth = chip(menu_row(&menu, "Earth Mana"), "affinity").expect("the Earth Elemental's affinity");
    let tip: Vec<&str> = arr(&earth, "tip").iter().filter_map(Json::as_str).collect();
    assert_eq!(tip[0], "Affinity: +1 strength per Earth Mana you own, for Djinn, Dwarven Druid and Earth Elemental.");
    assert!(tip.contains(&"Yours: Earth Elemental."), "{tip:?}");
    assert!(chip(menu_row(&menu, "Fire Mana"), "affinity").is_none(), "no Fire Elemental");
    // The now lines: the first node, and each free-promotion threshold crossed.
    let now = |name: &str| menu_row(&menu, name).get("now").map(|n| (s_of(n, "text").unwrap().to_string(), n.get("warn").and_then(Json::as_bool).unwrap()));
    let zero = arr(&menu, "rows").iter().find(|r| i_of(r, "owned") == Some(0)).expect("a mana not owned");
    let tier1 = s_of(&arr(zero, "promotions")[0], "name").unwrap();
    assert_eq!(zero.get("now").and_then(|n| s_of(n, "text")), Some(format!("First node: Arcane Units with Channeling I can learn {tier1}.").as_str()));
    assert_eq!(now("Body Mana"), Some(("New Arcane Units start with Body I.".to_string(), false)));
    assert_eq!(now("Chaos Mana"), Some(("New Arcane Units with Channeling II start with Chaos II.".to_string(), false)));
    assert_eq!(now("Earth Mana"), Some(("New Arcane Units with Channeling III start with Earth III.".to_string(), false)));
    // The buildings chip: Air's first node meets Storm Forge's requirement, not Tower of the
    // Elements' (Earth owned meets it already); Fire I's spells.
    let air = chip(menu_row(&menu, "Air Mana"), "buildings").expect("Air allows Tower of the Elements");
    assert_eq!(arr(&air, "tip")[0].as_str(), Some("This node meets the mana requirement of Storm Forge."), "not Tower of the Elements: Earth Mana meets it already: {air:?}");
    for r in arr(&menu, "rows") {
        if let Some(b) = chip(r, "buildings").filter(|b| b.get("new").and_then(Json::as_bool) == Some(true)) {
            assert_eq!(i_of(r, "owned"), Some(0), "{r:?}");
            assert!(arr(&b, "tip")[0].as_str().unwrap().starts_with("This node meets the mana requirement of "), "{b:?}");
        }
    }
    let fire = menu_row(&menu, "Fire Mana");
    let fire1 = &arr(fire, "promotions")[0];
    assert_eq!(s_of(fire1, "name"), Some("Fire I"));
    assert_eq!(arr(fire1, "spells").iter().filter_map(|x| s_of(x, "name")).collect::<Vec<_>>(), ["Blaze", "Firebolt"]);
    // Not a valid plot: refused.
    assert!(s.query(&Query::PlaceMenu { spell, x: st_capital_x(&s, st.capital), y: s.g.cities[st.capital].y }).is_err(), "the capital's plot");

    // A plot off the trade network (no road, no river): the node would not count.
    let q = s.g.map.plot_idx(x, y).unwrap();
    if !s.g.plot_is_river(q) {
        s.g.plot_set_route_type(q, None, true);
        let off = s.query(&Query::PlaceMenu { spell, x, y }).unwrap();
        let fire = menu_row(&off, "Fire Mana");
        assert_eq!(i_of(fire, "after"), i_of(fire, "owned"));
        assert_eq!(fire.get("now").and_then(|n| s_of(n, "text")), Some("No effect: the plot is not connected to your capital."));
        assert_eq!(fire.get("now").and_then(|n| n.get("warn")).and_then(Json::as_bool), Some(true));
        s.g.plot_set_route_type(q, Some(d.t::<RouteType>("ROUTE_ROAD")), true);
    }

    // Placed with no unit at all.
    for u in s.g.players[h].units.clone() {
        if s.g.units.contains(u) {
            s.g.unit_kill(u, false, None);
        }
    }
    let target = fire.get("target").unwrap().clone();
    let cmd = Command::from_json(d, &Json::parse(&format!(r#"{{"c":"place","spell":"SPELL_WELLSPRING","target":{target}}}"#)).unwrap()).unwrap();
    let r = s.execute(&cmd);
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    assert_eq!(s.g.map.plots[q].bonus_type, Some(d.t::<BonusType>("BONUS_MANA_FIRE")));
    assert_eq!(s.g.player_get_num_available_bonuses(h, d.t::<BonusType>("BONUS_MANA_FIRE")) as i64, i_of(fire, "after").unwrap(), "the node counts, as the menu said");
    let a = world_action(&s, spell).unwrap();
    assert!(a.get("place").is_none(), "the charge is spent: {a:?}");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(false));
    // No charge: a second placement is refused.
    assert!(s.execute(&cmd).is_err());
}

/// `ffh-play stage ritual-notech:CIVILIZATION_AMURITES`: Ritual: Wellspring needs Knowledge of the
/// Ether: the production list shows it disabled with the requirement, and the world-spell button
/// says so.
#[test]
fn wellspring_ritual_is_listed_with_its_tech_requirement() {
    let dir = common::TempDir::new("stage-ritual-notech-amurites");
    let st = ffh_play::stage::ritual_notech(&dir.0, "CIVILIZATION_AMURITES").expect("staged");
    let mut s = load(&st.save);
    let d = s.g.d;
    let h = s.human;
    let project = project_of(&s, h);
    let entry = ritual_entry(&s, st.capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(entry.get("ritual").and_then(|r| s_of(r, "why")), Some("Requires Knowledge of the Ether."));
    let a = world_action(&s, d.t::<SpellType>("SPELL_WELLSPRING")).unwrap();
    assert_eq!(s_of(&a, "why"), Some("No charge. Ritual: Wellspring requires Knowledge of the Ether."), "{a:?}");
    // With the tech: listed and enabled.
    let team = s.g.players[h].team;
    s.g.team_set_has_tech(team, d.t::<TechType>("TECH_KNOWLEDGE_OF_THE_ETHER"), true, Some(h), false, false);
    let entry = ritual_entry(&s, st.capital, project);
    assert_eq!(entry.get("enabled").and_then(Json::as_bool), Some(true));
}

fn st_capital_x(s: &Session, c: CityId) -> i32 {
    s.g.cities[c].x
}

/// `ffh-play stage ritual-due:CIVILIZATION_AMURITES`: Ritual: Wellspring completes at the end of the
/// turn; the next turn's world action holds the placement (the page opens it then).
#[test]
fn wellspring_placement_comes_with_the_completed_ritual() {
    let dir = common::TempDir::new("stage-ritual-due-amurites");
    let st = ffh_play::stage::ritual_due(&dir.0, "CIVILIZATION_AMURITES").expect("staged");
    let mut s = load(&st.save);
    let spell = s.g.d.t::<SpellType>("SPELL_WELLSPRING");
    let a = world_action(&s, spell).unwrap();
    assert!(a.get("place").is_none() && a.get("place_on_charge").and_then(Json::as_bool) == Some(true), "{a:?}");
    let r = s.execute(&Command::EndTurn { force: true });
    assert!(r.is_ok(), "{r:?}");
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let a = world_action(&s, spell).unwrap();
    assert!(a.get("place").is_some(), "{a:?}");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(true));
}

/// The caster line names no place when the human has no city, and a Delve's other end is named
/// by the nearest city the human knows (`ffh-play stage ritual-civ:CIVILIZATION_KHAZAD`: a tunnel
/// next to the capital).
#[test]
fn places_are_named_by_cities_not_coordinates() {
    let dir = common::TempDir::new("stage-ritual-civ-khazad");
    let st = ffh_play::stage::ritual_civ(&dir.0, "CIVILIZATION_KHAZAD").expect("staged");
    let mut s = load(&st.save);
    let name = city_name(&s, st.capital);
    let view = s.query(&Query::View).unwrap();
    let delves = arr(&view, "delves");
    assert!(!delves.is_empty());
    for e in delves {
        let place = s_of(e, "to_place").unwrap();
        assert!(place == name || place == format!("a plot near {name}"), "{place}");
    }
    let near = |s: &Session, x: i32, y: i32| ffh_play::view::near_own_city(&ffh_play::view::Viewer::new(&s.g, s.human, false), x, y);
    let (cx, cy) = (s.g.cities[st.capital].x, s.g.cities[st.capital].y);
    assert_eq!(near(&s, cx, cy), Some(format!("in {name}")));
    assert_eq!(near(&s, cx + 2, cy), Some(format!("near {name}")));
    let h = s.human;
    for c in s.g.players[h].cities.clone() {
        s.g.players[h].cities.retain(|&x| x != c);
    }
    assert_eq!(near(&s, cx, cy), None, "no city: no place");
}
