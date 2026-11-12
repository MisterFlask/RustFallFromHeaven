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
    assert!(help.contains("Placed when Ritual: Wellspring completes."), "{help}");
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

/// Wellspring's menu on a plot (`place_menu`): a row per mana type with what owning it gives, from
/// the data and the rules (a resource's health and happiness, the free sphere promotions of new
/// arcane units, affinity, what it allows), how many the human owns, and its sphere promotions in
/// tier order, each with the spells it opens and their help. Then a row placed with no unit at all.
#[test]
fn wellspring_menu_lists_each_mana_type_with_its_passive_and_its_promotions() {
    let dir = common::TempDir::new("stage-ritual-civ-amurites-menu");
    let st = ffh_play::stage::ritual_civ(&dir.0, "CIVILIZATION_AMURITES").expect("staged");
    let mut s = load(&st.save);
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_WELLSPRING");
    let a = world_action(&s, spell).unwrap();
    let at = arr(&arr(&a, "targets")[0], "at").to_vec();
    let (x, y) = (at[0].as_i64().unwrap() as i32, at[1].as_i64().unwrap() as i32);
    let menu = s.query(&Query::PlaceMenu { spell, x, y }).unwrap();
    assert_eq!(s_of(&menu, "title"), Some("Wellspring"));
    let manas = ffh_core::ffh::world_ritual_spells::small::wellspring_manas(d);
    assert_eq!(arr(&menu, "rows").len(), manas.len(), "every mana type");
    let v = ffh_play::view::Viewer::new(&s.g, h, false);
    for &(b, _) in &manas {
        let name = d.text.name(&d.bonuses[b].base.description).to_string();
        let row = menu_row(&menu, &name);
        assert_eq!(i_of(row, "bonus"), Some(b.idx() as i64));
        assert_eq!(s_of(row, "owned"), Some(format!("You own: {}.", s.g.player_get_num_available_bonuses(h, b)).as_str()));
        let passive: Vec<&str> = arr(row, "passive").iter().filter_map(Json::as_str).collect();
        assert_eq!(passive, ffh_play::view::place::passive_lines(&v, b), "{name}");
        let info = &d.bonuses[b];
        if info.health != 0 {
            assert!(passive.contains(&format!("{:+} health in each of your cities that has it.", info.health).as_str()), "{name}: {passive:?}");
        }
        if info.happiness != 0 {
            assert!(passive.contains(&format!("{:+} happiness in each of your cities that has it.", info.happiness).as_str()), "{name}: {passive:?}");
        }
        // The free promotions rule, and the sphere promotions in tier order.
        let tiers = ffh_core::ffh::events::adept_mana_promotions(d, b).expect("a sphere");
        assert!(passive.contains(&ffh_play::view::place::free_promotions_line(d, b).unwrap().as_str()), "{name}");
        let promos = arr(row, "promotions");
        let got: Vec<i64> = promos.iter().filter_map(|p| i_of(p, "promotion")).collect();
        assert_eq!(got, tiers.iter().map(|p| p.idx() as i64).collect::<Vec<_>>(), "{name}");
        assert_eq!(promos.iter().filter_map(|p| s_of(p, "tier")).collect::<Vec<_>>(), ["I", "II", "III"]);
        assert_eq!(d.promotions[tiers[0]].bonus_prereq, Some(b), "tier I needs the mana");
        for (p, &t) in promos.iter().zip(&tiers) {
            let want: Vec<SpellType> = d.spells.keys().filter(|&sp| d.spells[sp].promotion_prereq1 == Some(t) || d.spells[sp].promotion_prereq2 == Some(t)).collect();
            let spells = arr(p, "spells");
            assert_eq!(spells.iter().filter_map(|x| i_of(x, "spell")).collect::<Vec<_>>(), want.iter().map(|x| x.idx() as i64).collect::<Vec<_>>(), "{name}");
            for (j, &sp) in spells.iter().zip(&want) {
                assert_eq!(s_of(j, "help"), Some(ffh_play::view::help::spell::effects(d, sp).as_str()));
            }
            assert!(s_of(p, "needs").is_some_and(|n| n.starts_with("Needs ")), "{p:?}");
        }
        // Affinity: every unit type with one for this mana is named.
        for u in d.units.keys().filter(|&u| d.units[u].bonus_affinity[b] != 0) {
            let unit = d.text.name(&d.units[u].base.description).to_string();
            assert!(passive.iter().any(|l| l.starts_with("Affinity: ") && l.contains(&unit)), "{name}: {unit} in {passive:?}");
        }
    }
    // A sample read from the data: Life Mana's health, Enchantment Mana's happiness, Fire Mana's
    // free Fire I with 2 or more, and the Fire Elemental's affinity for it.
    let life = arr(menu_row(&menu, "Life Mana"), "passive").iter().filter_map(Json::as_str).map(str::to_string).collect::<Vec<_>>();
    assert!(life.contains(&"+1 health in each of your cities that has it.".to_string()), "{life:?}");
    let ench = arr(menu_row(&menu, "Enchantment Mana"), "passive").iter().filter_map(Json::as_str).map(str::to_string).collect::<Vec<_>>();
    assert!(ench.contains(&"+1 happiness in each of your cities that has it.".to_string()), "{ench:?}");
    let fire = menu_row(&menu, "Fire Mana");
    let fire_passive = arr(fire, "passive").iter().filter_map(Json::as_str).collect::<Vec<_>>().join(" ");
    assert!(fire_passive.contains("Fire I with 2 or more Fire Mana"), "{fire_passive}");
    assert!(fire_passive.contains("Fire Elemental"), "{fire_passive}");
    let fire1 = &arr(fire, "promotions")[0];
    assert_eq!(s_of(fire1, "name"), Some("Fire I"));
    assert!(arr(fire1, "spells").iter().any(|x| s_of(x, "name") == Some("Blaze")), "{fire1:?}");
    // Not a valid plot: refused.
    assert!(s.query(&Query::PlaceMenu { spell, x: st_capital_x(&s, st.capital), y: s.g.cities[st.capital].y }).is_err(), "the capital's plot");

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
    let q = s.g.map.plot_idx(x, y).unwrap();
    assert_eq!(s.g.map.plots[q].bonus_type, Some(d.t::<BonusType>("BONUS_MANA_FIRE")));
    let a = world_action(&s, spell).unwrap();
    assert!(a.get("place").is_none(), "the charge is spent: {a:?}");
    assert_eq!(a.get("enabled").and_then(Json::as_bool), Some(false));
    // No charge: a second placement is refused.
    assert!(s.execute(&cmd).is_err());
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
