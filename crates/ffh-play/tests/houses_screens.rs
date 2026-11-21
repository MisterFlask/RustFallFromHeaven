//! This game's Houses on the human's screens (docs/HOUSES.md, rule 15; docs/HUMAN.md, "Houses"):
//! the `houses` query, the city's Governance line and happiness lines, the plates' marks, the
//! diplomacy screen's Houses (for whoever met the owner), the previews in the questions where an
//! act is done, the pick's structured offer, the perk's help and the keywords. The staged games
//! (`ffh-play stage houses|houses-foreign`) and hand-built situations in games that cannot end;
//! nothing here depends on how a seed's game goes.

mod common;

use std::sync::{Arc, Mutex};

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdOk, Command};
use ffh_play::decision::{Decision, DecisionKind};
use ffh_play::human::{AdvisorAnswerer, Answer, Answerer, FnAnswerer};
use ffh_play::query::Query;
use ffh_play::stage::StagedHouses;
use ffh_play::view::Viewer;
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

fn strs(j: &Json, k: &str) -> Vec<String> {
    arr(j, k).iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()
}

/// The labels of a city's happiness parts (`[[label, n], ..]`).
fn part_labels(city: &Json, k: &str) -> Vec<String> {
    arr(city.get("happiness").unwrap(), k).iter().filter_map(|p| p.as_arr().and_then(|a| a[0].as_str().map(str::to_owned))).collect()
}

fn load_with(save: &std::path::Path, answerer: Arc<dyn Answerer>) -> Session {
    let (bytes, sidecar) = ffh_play::save::read_bundle(save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    Session::load(&bytes, sidecar.as_ref(), answerer, opts).unwrap()
}

fn load(save: &std::path::Path) -> Session {
    load_with(save, Arc::new(AdvisorAnswerer))
}

fn staged(name: &str, foreign: bool) -> (common::TempDir, StagedHouses) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::houses(&dir.0, foreign).expect("staged");
    (dir, st)
}

/// The `houses` query's card of House `id`.
fn card(screen: &Json, id: HouseId) -> Json {
    arr(screen, "houses").iter().find(|x| i_of(x, "id") == Some(id.0 as i64)).cloned().unwrap_or_else(|| panic!("no card for House {}", id.0))
}

fn row(players: &Json, p: PlayerId) -> Option<Json> {
    arr(players, "players").iter().find(|x| i_of(x, "id") == Some(p.0 as i64)).cloned()
}

/// The preview lines of House `h` in a preview list, if it is there.
fn preview_lines(list: &[Json], h: HouseId) -> Option<Vec<String>> {
    list.iter().find(|x| i_of(x, "house") == Some(h.0 as i64)).map(|x| strs(x, "lines"))
}

/// The Houses screen: every House a card with its lines, state, calculation, perk and cities; the
/// city in no House with its reason; the keywords' reminders.
#[test]
fn the_houses_screen_shows_each_house_and_the_city_in_none() {
    let (_dir, st) = staged("houses-screen", false);
    let s = load(&st.save);
    let g = &s.g;
    let screen = s.query(&Query::Houses).unwrap();
    assert_eq!(screen.get("has_houses").and_then(Json::as_bool), Some(true));
    assert_eq!(arr(&screen, "houses").len(), 4, "{screen:?}");

    let pleased = card(&screen, st.house("pleased").id);
    assert_eq!(s_of(&pleased, "state"), Some("Pleased"));
    let lines = strs(&pleased, "lines");
    assert_eq!(lines[0], "Alignment: Good");
    assert_eq!(lines[1], "Religion: Order");
    assert_eq!(lines[2], format!("Sympathy: {}", g.house_civ_name(st.rival)));
    assert!(lines[3].starts_with("Perk: Horsemasters"), "{lines:?}");
    let perk = pleased.get("perk").unwrap();
    assert_eq!(perk.get("on").and_then(Json::as_bool), Some(true));
    assert!(!strs(perk, "lines").is_empty(), "the perk's effect lines");
    assert_eq!(pleased.get("perk_off"), Some(&Json::Null));
    assert_eq!(s_of(&pleased, "happiness_line"), Some("Happiness: +1 in 4 cities."));
    let calc = strs(&pleased, "calc");
    assert!(calc.iter().any(|l| l.starts_with("Opinion: ") && l.ends_with(": Pleased.")), "{calc:?}");
    assert!(calc.iter().any(|l| l == "Happiness: +1 (Pleased) in each of its 4 cities."), "{calc:?}");
    let parts = arr(&pleased, "parts");
    assert_eq!(parts.iter().map(|p| i_of(p, "value").unwrap()).sum::<i64>(), i_of(&pleased, "opinion").unwrap(), "the parts sum to the opinion");
    let cities = arr(&pleased, "cities");
    assert_eq!(cities.len(), 4);
    assert_eq!(i_of(&cities[0], "id"), Some(st.capital.0 as i64));
    assert_eq!(cities[0].get("seat").and_then(Json::as_bool), Some(true));
    assert!(cities.iter().all(|c| s_of(c, "name").is_some_and(|n| !n.is_empty()) && i_of(c, "x").is_some()), "{cities:?}");

    let annoyed = card(&screen, st.house("annoyed").id);
    assert_eq!(s_of(&annoyed, "state"), Some("Annoyed"));
    assert_eq!(s_of(&annoyed, "happiness_line"), Some("Happiness: -2 in 1 city."));

    let furious = card(&screen, st.house("furious").id);
    assert_eq!(s_of(&furious, "state"), Some("Furious"));
    assert_eq!(furious.get("foreign").and_then(Json::as_bool), Some(true));
    assert_eq!(furious.get("perk").and_then(|p| p.get("on")).and_then(Json::as_bool), Some(false));
    assert_eq!(s_of(&furious, "perk_off"), Some("Perk off: the House is Furious."));
    assert_eq!(strs(&furious, "lines")[2], format!("Sympathy: {}", g.house_civ_name(st.enemy)));

    let outsider = card(&screen, st.house("outsider").id);
    assert_eq!(outsider.get("outsider").and_then(Json::as_bool), Some(true));
    assert_eq!(strs(&outsider, "lines")[2], "Outsider: -3 happiness in this House's cities. No sympathy.");
    assert_eq!(i_of(&outsider, "happiness"), Some(-2));
    let calc = strs(&outsider, "calc");
    assert!(calc.iter().any(|l| l == "Happiness: +1 (Pleased) -3 (outsider) = -2 in its city."), "{calc:?}");
    assert!(arr(&outsider, "parts").iter().all(|p| matches!(s_of(p, "key"), Some("alignment" | "religion"))), "an outsider has parts (a) and (b) only");

    let unhoused = arr(&screen, "unhoused");
    assert_eq!(unhoused.len(), 1, "{unhoused:?}");
    assert_eq!(i_of(&unhoused[0], "id"), Some(st.unhoused.0 as i64));
    assert_eq!(s_of(&unhoused[0], "reason"), Some("No House yet: it joins or forms one at your next turn."));

    let keys: Vec<&str> = arr(&screen, "reminders").iter().filter_map(|r| s_of(r, "key")).collect();
    for k in ["KEYWORD_HOUSE", "KEYWORD_SYMPATHY", "KEYWORD_OUTSIDER"] {
        assert!(keys.contains(&k), "{keys:?}");
    }
}

/// Before a civilization's first House: its cities wait, and the reason counts them.
#[test]
fn the_first_house_waits_for_three_cities() {
    let mut s = Session::new_game(&common::steady_duel(7), Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let u = s.g.players[h].units[0];
    let (x, y) = (s.g.units[u].x, s.g.units[u].y);
    s.g.player_init_city(h, x, y, true, true);
    let g = &s.g;
    let q = g
        .map
        .plot_ids()
        .filter(|&q| !g.plot_is_water(q))
        .map(|q| g.map.xy(q))
        .find(|&(qx, qy)| g.plot_distance(qx, qy, x, y) == 4 && g.player_can_found(h, qx, qy, false))
        .expect("a second site");
    s.g.player_init_city(h, q.0, q.1, true, true);
    let screen = s.query(&Query::Houses).unwrap();
    assert!(arr(&screen, "houses").is_empty());
    let reasons: Vec<&str> = arr(&screen, "unhoused").iter().filter_map(|u| s_of(u, "reason")).collect();
    assert_eq!(reasons, vec!["No House yet: the first forms at 3 cities (2 of 3)."; 2]);
}

/// The city screen: the Governance line and the happiness lines of its House, with the
/// calculation; the plates' marks and the alerts' brief in the view.
#[test]
fn the_city_screen_and_the_map_show_a_citys_house() {
    let (_dir, st) = staged("houses-city", false);
    let s = load(&st.save);
    let g = &s.g;
    let pleased = st.house("pleased");
    let leader = ffh_core::houses::leader_name(g.d, g.houses[pleased.id].leader);
    let city = s.query(&Query::City { city: st.capital }).unwrap();
    let house = city.get("happiness").and_then(|x| x.get("house")).expect("the capital's House");
    let line = s_of(house, "line").unwrap();
    assert!(line.starts_with(&format!("{} ({leader}): Pleased, +1 happiness. Perk: Horsemasters", pleased.label)), "{line}");
    assert!(!strs(house, "calc").is_empty());
    assert!(part_labels(&city, "happy_parts").contains(&format!("{} is Pleased", pleased.label)), "{city:?}");
    assert!(i_of(&city, "culture_rate").is_some(), "the city's culture a turn");

    let furious = st.house("furious");
    let city = s.query(&Query::City { city: furious.cities[0] }).unwrap();
    let line = s_of(city.get("happiness").and_then(|x| x.get("house")).unwrap(), "line").unwrap().to_string();
    assert!(line.contains(": Furious, -4 happiness. Perk: ") && line.ends_with(" (off)."), "{line}");
    assert!(part_labels(&city, "unhappy_parts").contains(&format!("{} is Furious", furious.label)));

    let outsider = st.house("outsider");
    let city = s.query(&Query::City { city: outsider.cities[0] }).unwrap();
    assert!(part_labels(&city, "unhappy_parts").iter().any(|l| l == "Ruled by an outsider"));

    let city = s.query(&Query::City { city: st.unhoused }).unwrap();
    assert_eq!(city.get("happiness").and_then(|x| x.get("house")), Some(&Json::Null));
    assert_eq!(s_of(&city, "house_reason"), Some("No House yet: it joins or forms one at your next turn."));

    // The view: an own city's House for its plate; another's city carries none.
    let view = s.query(&Query::View).unwrap();
    let mark = arr(&view, "cities").iter().find(|c| i_of(c, "id") == Some(st.capital.0 as i64)).and_then(|c| c.get("house")).cloned().expect("the capital's mark");
    assert_eq!(i_of(&mark, "id"), Some(pleased.id.0 as i64));
    assert_eq!(s_of(&mark, "label"), Some(pleased.label.as_str()));
    assert_eq!(s_of(&mark, "state"), Some("Pleased"));
    for c in arr(&view, "cities") {
        if i_of(c, "owner") != Some(st.human.0 as i64) || i_of(c, "id") == Some(st.unhoused.0 as i64) {
            assert!(c.get("house").is_none(), "{c:?}");
        }
    }
    let brief = arr(&view, "houses");
    assert_eq!(brief.len(), 4);
    assert!(brief.iter().all(|b| i_of(b, "leader").is_some() && s_of(b, "leader_name").is_some() && s_of(b, "state").is_some()));
}

/// Diplomacy: a civilization's Houses for whoever has met it, not for one who has not; the war
/// question's preview.
#[test]
fn diplomacy_lists_a_civilizations_houses_for_whoever_met_it() {
    let (_dir, st) = staged("houses-diplo", false);
    let s = load(&st.save);
    let g = &s.g;
    let players = s.query(&Query::Players).unwrap();
    let rival = row(&players, st.rival).expect("the rival is listed");
    let houses = arr(&rival, "houses");
    assert_eq!(houses.len(), g.players[st.rival].houses.len());
    let x = &houses[0];
    assert!(s_of(x, "label").is_some_and(|l| l.starts_with("House ")) && s_of(x, "leader_name").is_some() && s_of(x, "state").is_some() && s_of(x, "perk").is_some(), "{x:?}");
    assert!(strs(x, "lines").iter().any(|l| l.starts_with("Alignment: ")), "{x:?}");
    assert!(row(&players, st.unmet).is_none(), "the unmet civilization is not listed");

    // As the rival sees the human (met): the human's four Houses.
    let v = Viewer::new(g, st.rival, false);
    let seen = ffh_play::view::player::players(&v);
    let human = row(&seen, st.human).expect("the rival lists the human");
    assert_eq!(arr(&human, "houses").len(), 4);
    let states: Vec<&str> = arr(&human, "houses").iter().filter_map(|x| s_of(x, "state")).collect();
    assert!(states.contains(&"Furious") && states.contains(&"Annoyed"), "{states:?}");
    // As the unmet civilization: the human is not listed, nor are its Houses.
    let v = Viewer::new(g, st.unmet, false);
    let unseen = ffh_play::view::player::players(&v);
    assert!(row(&unseen, st.human).is_none(), "the unmet civilization lists the human");
    assert_eq!(ffh_play::view::houses::player_houses(&v, st.human), Json::Arr(Vec::new()));

    // The war question on the rival: House pleased favours it.
    let pleased = st.house("pleased");
    let lines = preview_lines(arr(&rival, "war_house_preview"), pleased.id).expect("House pleased's preview");
    assert!(lines[0].starts_with(&format!("{}: Pleased to ", pleased.label)) && lines[0].ends_with(" in 4 cities."), "{lines:?}");
    // At war already with the enemy: no war question, no preview.
    let enemy = row(&players, st.enemy).unwrap();
    assert!(arr(&enemy, "war_house_preview").is_empty());
}

/// The previews in the trade screen (peace, a defensive pact) and on the religion screen.
#[test]
fn the_questions_preview_what_an_act_does_to_the_houses() {
    let (_dir, st) = staged("houses-previews", false);
    let s = load(&st.save);
    let trade = s.query(&Query::Trade { with: st.enemy }).unwrap();
    let pv = trade.get("house_preview").unwrap();
    let furious = st.house("furious");
    let lines = preview_lines(arr(pv, "peace"), furious.id).expect("peace changes House furious");
    assert_eq!(lines, vec![format!("{}: Furious to Annoyed. Happiness +2 in 1 city.", furious.label), "Its perk is on again.".to_string()]);
    assert!(arr(pv, "defensive_pact").is_empty(), "no pact while at war");

    let trade = s.query(&Query::Trade { with: st.rival }).unwrap();
    let pv = trade.get("house_preview").unwrap();
    assert!(arr(pv, "peace").is_empty(), "at peace: no peace to make");
    let pleased = st.house("pleased");
    let lines = preview_lines(arr(pv, "defensive_pact"), pleased.id).expect("a pact changes House pleased");
    assert_eq!(lines, vec![format!("{}: Pleased to Friendly. Happiness +1 in 4 cities.", pleased.label)]);

    let rel = s.query(&Query::Religions).unwrap();
    let order = s.g.d.t::<ReligionType>("RELIGION_THE_ORDER");
    let r = arr(&rel, "religions").iter().find(|r| i_of(r, "religion") == Some(order.idx() as i64)).cloned().unwrap();
    assert_eq!(r.get("can_convert").and_then(Json::as_bool), Some(true));
    let pv = arr(&r, "house_preview");
    let annoyed = st.house("annoyed");
    let lines = preview_lines(pv, annoyed.id).expect("adopting The Order changes House annoyed");
    assert_eq!(lines, vec![format!("{}: Annoyed to Furious. Happiness -2 in 1 city.", annoyed.label), "Its perk goes off.".to_string()]);
    assert!(pv.iter().any(|x| i_of(x, "house") == Some(pleased.id.0 as i64) && s_of(x, "after") == Some("Friendly")), "{pv:?}");
}

/// The House picks raised by ending the turn of a staged game (`foreign`: `houses-foreign`),
/// answered with option `answer(n)` of `n`: the decisions' JSON and the session after.
fn picks(name: &str, foreign: bool, answer: fn(usize) -> Option<usize>) -> (StagedHouses, common::TempDir, Vec<Json>, Session) {
    let (dir, st) = staged(name, foreign);
    let asked: Arc<Mutex<Vec<Json>>> = Arc::new(Mutex::new(Vec::new()));
    let seen = asked.clone();
    let answerer = Arc::new(FnAnswerer(move |_g: &Game, d: &Decision| {
        if let DecisionKind::HouseLeader { .. } = d.kind {
            seen.lock().unwrap().push(d.to_json());
            if let Some(i) = answer(d.options.len()) {
                return Answer::Option(i);
            }
        }
        Answer::Advisor
    }));
    let mut s = load_with(&st.save, answerer);
    assert!(matches!(s.execute(&Command::EndTurn { force: true }), Ok(CmdOk::Applied { .. })));
    let _ = s.advance(&mut |_| {});
    let asked = asked.lock().unwrap().clone();
    (st, dir, asked, s)
}

/// Ending the turn of `houses` forms the waiting city's House: the pick is a blocking decision
/// with the offer as structured JSON (the cards, the head, the advisor's card), and the card
/// answered leads the House.
#[test]
fn the_pick_carries_its_cards() {
    let (st, _dir, asked, s) = picks("houses-pick", false, |n| Some(n - 1));
    assert_eq!(asked.len(), 1, "one pick");
    let d = &asked[0];
    let offer = d.get("offer").expect("the structured offer");
    let cards = arr(offer, "cards");
    assert_eq!(cards.len(), arr(d, "options").len(), "a card per option");
    assert!(cards.len() >= 2);
    assert_eq!(offer.get("foreign").and_then(Json::as_bool), Some(false));
    let head = strs(offer, "head");
    assert!(head[0].starts_with(&format!("{} forms with ", s_of(offer, "label").unwrap())), "{head:?}");
    for (i, c) in cards.iter().enumerate() {
        assert_eq!(i_of(c, "index"), Some(i as i64));
        let lines = strs(c, "lines");
        assert!(lines[0].starts_with("Alignment: ") && lines[1].starts_with("Religion: ") && lines[2].starts_with("Sympathy: ") && lines[3].starts_with("Perk: "), "{lines:?}");
        assert!(s_of(c, "opinion_line").is_some_and(|l| l.starts_with("Opinion now: ")), "{c:?}");
        assert!(s_of(c, "happiness_line").is_some_and(|l| l.starts_with("Happiness: ") && l.ends_with(" in 1 city.")), "{c:?}");
        let calc = strs(c, "calc");
        assert!(calc.iter().any(|l| l.starts_with("Opinion: ")) && calc.iter().any(|l| l.starts_with("Happiness: ")), "{calc:?}");
        assert!(!strs(c.get("perk").unwrap(), "lines").is_empty());
        assert_eq!(c.get("outsider").and_then(Json::as_bool), Some(false));
    }
    let advisor = i_of(d, "advisor").expect("advice") as usize;
    assert_eq!(i_of(offer, "advisor"), Some(advisor as i64));
    assert_eq!(cards[advisor].get("advisor").and_then(Json::as_bool), Some(true));
    assert_eq!(cards.iter().filter(|c| c.get("advisor").and_then(Json::as_bool) == Some(true)).count(), 1);
    // The card answered leads the new House, which holds the waiting city.
    let h = st.human;
    assert_eq!(s.g.players[h].houses.len(), 5);
    let new = *s.g.players[h].houses.last().unwrap();
    assert_eq!(s.g.d.type_name(s.g.houses[new].leader), s_of(cards.last().unwrap(), "leader_type").unwrap());
    assert_eq!(s.g.houses[new].cities, vec![st.unhoused]);
}

/// `houses-foreign`: the pick of a foreign House has the outsider's card last.
#[test]
fn a_foreign_pick_has_the_outsiders_card() {
    let (_st, _dir, asked, _s) = picks("houses-pick-foreign", true, |_| None);
    assert_eq!(asked.len(), 1, "one pick");
    let offer = asked[0].get("offer").unwrap();
    assert_eq!(offer.get("foreign").and_then(Json::as_bool), Some(true));
    let cards = arr(offer, "cards");
    let last = cards.last().unwrap();
    assert_eq!(last.get("outsider").and_then(Json::as_bool), Some(true));
    assert_eq!(strs(last, "lines")[2], "Outsider: -3 happiness in this House's cities. No sympathy.");
    assert!(cards[..cards.len() - 1].iter().all(|c| c.get("outsider").and_then(Json::as_bool) == Some(false)));
}

/// A perk's help (the data's TraitType entry) says it is a House perk of its tier and lists its
/// effects.
#[test]
fn a_perks_help_says_what_it_does() {
    let d = ffh_data::global();
    for (name, tier) in [("TRAIT_PERK_HORSEMASTERS_1", 1), ("TRAIT_PERK_SCHOLARS_3", 3)] {
        let t = d.t::<TraitType>(name);
        let help = ffh_play::view::help::civ::trait_help(d, t, None);
        assert!(help.contains(&format!("A House perk, tier {tier}.")), "{help}");
        for l in ffh_core::houses::perk_lines(d, t) {
            assert!(help.contains(&l), "{name}: {l} in {help}");
        }
    }
}

/// The cards' lines (the screen's and the pick's) do not restate the Houses' keywords'
/// reminders (docs/PLAY_DESIGN.md 4.8).
#[test]
fn the_cards_do_not_restate_the_keywords() {
    let d = ffh_data::global();
    let (st, _dir, asked, _s) = picks("houses-keywords", true, |_| None);
    let s = load(&st.save);
    let screen = s.query(&Query::Houses).unwrap();
    let mut texts: Vec<String> = Vec::new();
    for c in arr(&screen, "houses") {
        texts.extend(strs(c, "lines"));
        texts.extend(c.get("perk").map(|p| strs(p, "lines")).unwrap_or_default());
        texts.extend(s_of(c, "perk_off").map(str::to_owned));
    }
    for u in arr(&screen, "unhoused") {
        texts.extend(s_of(u, "reason").map(str::to_owned));
    }
    for c in arr(asked[0].get("offer").unwrap(), "cards") {
        texts.extend(strs(c, "lines"));
        texts.extend(s_of(c, "happiness_line").map(str::to_owned));
    }
    assert!(!texts.is_empty());
    for k in ffh_play::view::houses::keywords(d) {
        let reminder = ffh_play::view::keywords::reminder(d, k);
        assert!(!reminder.is_empty(), "{}", d.keywords[k].base.type_);
        for sentence in reminder.split(". ").map(|x| x.trim_end_matches('.').to_lowercase()).filter(|x| x.len() > 20) {
            for t in &texts {
                assert!(!t.to_lowercase().contains(&sentence), "{t:?} restates {}: {sentence:?}", d.keywords[k].base.type_);
            }
        }
    }
}
