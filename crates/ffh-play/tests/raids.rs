//! Raids as the human sees and commands them (docs/RAIDS.md, section 8; the protocol in
//! docs/HUMAN.md): the unit panel's `Raid <city>` (enabled with its take and calculation, or
//! disabled with the rules' reason) and `Stop raiding`, the army's station, the pillage order
//! refused on fields with its reason, a raided city's `raid` for both sides and its owner's
//! `sally`, and `city_sally`; on the staged saves (`stage raid-order|raid-overrun|raid-contested`).

use std::sync::Arc;

use ffh_core::army::Station;
use ffh_core::ids::*;
use ffh_core::raid::Sally;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::view::{self, Viewer};
use ffh_play::{Json, Session, SessionOptions};

/// The staged save `kind`, loaded, with the stage's reply.
fn staged(kind: &str) -> (Session, Json, std::path::PathBuf) {
    // (A directory of its own for each call: the tests run at once.)
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("ffh-raids-{kind}-{}-{n}", std::process::id()));
    let fixture = ffh_play::stage::raid(&root, kind).unwrap();
    let bytes = std::fs::read(fixture.req::<String>("save").unwrap()).unwrap();
    let s = Session::load(&bytes, None, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    (s, fixture, root)
}

fn arr(j: &Json, key: &str) -> Vec<Json> {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("`{key}` in {j}")).to_vec()
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("`{key}` in {j}"))
}

/// The selection's actions.
fn actions(s: &Session, units: &[UnitId]) -> Vec<Json> {
    let j = view::units::group(&Viewer::new(&s.g, s.human, false), units).unwrap();
    arr(&j, "actions")
}

/// The army action `army` (its `army` field) of the selection, if shown.
fn army_action(s: &Session, units: &[UnitId], army: &str) -> Option<Json> {
    actions(s, units).into_iter().find(|a| a.get("kind").and_then(Json::as_str) == Some("army") && a.get("army").and_then(Json::as_str) == Some(army))
}

/// Sends an action's command as the client does.
fn send(s: &mut Session, action: &Json) -> Result<CmdOk, CmdErr> {
    let cmd = Command::from_json(s.g.d, action.get("cmd").unwrap()).unwrap();
    s.execute(&cmd)
}

fn station(s: &Session, a: ArmyId) -> Json {
    let all = view::armies::armies(&Viewer::new(&s.g, s.human, false));
    all.as_arr().unwrap().iter().find(|j| j.get("id") == Some(&Json::from(a))).unwrap().get("station").unwrap().clone()
}

#[test]
fn the_raid_order_is_offered_taken_and_ended_from_the_unit_panel() {
    let (mut s, st, root) = staged("raid-order");
    let units: Vec<UnitId> = st.req("units").unwrap();
    let army: ArmyId = st.req("army").unwrap();
    let city: CityId = st.req("city").unwrap();
    let name = view::city_name(&s.g, city);
    // Before the order: a land army has no standing order.
    assert_eq!(text(&station(&s, army), "kind"), "none");
    assert_eq!(station(&s, army).get("text"), Some(&Json::Null));
    assert!(army_action(&s, &units, "stop_raid").is_none());
    // Offered, enabled, with the take and its calculation.
    let raid = army_action(&s, &units, "raid").expect("the Raid action");
    assert_eq!(text(&raid, "label"), format!("Raid {name}"));
    assert_eq!(raid.get("enabled").and_then(Json::as_bool), Some(true));
    let help = text(&raid, "help");
    assert!(help.starts_with("Take: ") && help.contains(" gold a turn at 50%."), "{help}");
    assert!(help.contains("The guard sallies under the city's posture."), "{help}");
    let calc = arr(&raid, "calc");
    assert!(calc.iter().any(|l| l.as_str().unwrap().starts_with("Production: ")), "{calc:?}");
    assert!(calc.iter().any(|l| l.as_str().unwrap().starts_with("Loss: ")), "{calc:?}");
    assert!(raid.get("why").is_none());
    // Disabled with the rules' reason (a stripped countryside).
    s.g.cities[city].raid_stripped = 3;
    let refused = army_action(&s, &units, "raid").expect("shown while refused");
    assert_eq!(refused.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(text(&refused, "why"), "The countryside is stripped.");
    s.g.cities[city].raid_stripped = 0;
    // Taken by its command: the station says so, the messages tell it.
    assert!(matches!(send(&mut s, &raid).unwrap(), CmdOk::Applied { .. }));
    assert_eq!(s.g.armies[army].station, Station::Raid(city));
    let st_view = station(&s, army);
    assert_eq!(text(&st_view, "kind"), "raid");
    assert_eq!(text(&st_view, "text"), format!("Raiding {name}."));
    assert!(s.messages().iter().any(|m| m.text == format!("The {} raids {name}.", s.g.armies[army].name)), "{:?}", s.messages().iter().map(|m| &m.text).collect::<Vec<_>>());
    // Now: Stop raiding, and no second Raid.
    assert!(army_action(&s, &units, "raid").is_none());
    let stop = army_action(&s, &units, "stop_raid").expect("Stop raiding");
    assert_eq!(text(&stop, "label"), "Stop raiding");
    assert!(matches!(send(&mut s, &stop).unwrap(), CmdOk::Applied { .. }));
    assert_eq!(s.g.armies[army].station, Station::None);
    assert_eq!(text(&station(&s, army), "kind"), "none");
    // No enemy countryside (the peace): no Raid action at all.
    let (ht, ct) = (s.g.players[s.human].team, s.g.city_team(city));
    s.g.team_make_peace(ht, ct, false);
    assert!(army_action(&s, &units, "raid").is_none());
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn pillage_on_fields_is_shown_disabled_with_the_reason() {
    let (s, st, root) = staged("raid-order");
    let units: Vec<UnitId> = st.req("units").unwrap();
    let pillage = Json::idx(MissionType::MISSION_PILLAGE);
    let p = actions(&s, &units).into_iter().find(|a| a.get("kind").and_then(Json::as_str) == Some("mission") && a.get("mission") == Some(&pillage)).expect("the pillage order");
    assert_eq!(p.get("enabled").and_then(Json::as_bool), Some(false));
    assert_eq!(text(&p, "why"), "Fields are raided, not pillaged.");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn an_overrun_city_shows_its_raid_to_both_sides_and_its_owner_sets_the_posture() {
    let (mut s, st, root) = staged("raid-overrun");
    let city: CityId = st.req("city").unwrap();
    let army: ArmyId = st.req("army").unwrap();
    let h = s.human;
    let rival = s.g.armies[army].owner;
    for p in [h, rival] {
        let r = view::raids::city(&Viewer::new(&s.g, p, false), city);
        assert_eq!(text(&r, "state"), "overrun", "{r}");
        assert_eq!(text(&r, "label"), "Overrun");
        assert_eq!(text(&r, "text"), "Overrun: -50% production and commerce. The raiders take what the city loses as gold.");
        assert_eq!(r.get("percent"), Some(&Json::from(50)));
        let raiders = arr(&r, "raiders");
        assert_eq!(raiders.len(), 1);
        assert_eq!(raiders[0].get("army"), Some(&Json::from(army)));
        assert_eq!(raiders[0].get("owner"), Some(&Json::from(rival)));
        let calc = arr(&r, "calc");
        assert!(calc.len() >= 4, "{calc:?}");
        assert!(calc.last().unwrap().as_str().unwrap().ends_with(" gold."), "{calc:?}");
    }
    // The tile payload (the banner) carries it too.
    let tiles = view::cities(&Viewer::new(&s.g, h, false));
    let tile = tiles.as_arr().unwrap().iter().find(|j| j.get("id") == Some(&Json::from(city))).unwrap().clone();
    assert_eq!(text(tile.get("raid").unwrap(), "label"), "Overrun");
    // The city query: the raid and the owner's posture with every option's rule.
    let q = view::city::city(&Viewer::new(&s.g, h, false), city).unwrap();
    assert_eq!(text(q.get("raid").unwrap(), "state"), "overrun");
    let sally = q.get("sally").unwrap();
    assert_eq!(text(sally, "posture"), "Favoured");
    let opts = arr(sally, "options");
    let labels: Vec<&str> = opts.iter().map(|o| text(o, "label")).collect();
    assert_eq!(labels, ["Never", "When favoured", "Always"]);
    assert_eq!(text(&opts[1], "text"), "The guard sallies when its squad has at least 60% to win the battle.");
    assert_eq!(text(sally, "rule"), "A sally is a battle between a guard squad and a raider squad, each turn. The guard does not leave the city.");
    // The take reached the owner as a message (with its map position).
    assert!(s.g.cities.contains(city));
    // city_sally: set by the owner, refused for another player's city, a bad name refused.
    assert!(matches!(s.execute(&Command::CitySally { city, posture: Sally::Always }).unwrap(), CmdOk::Applied { .. }));
    assert_eq!(s.g.cities[city].sally, Sally::Always);
    let q = view::city::city(&Viewer::new(&s.g, h, false), city).unwrap();
    assert_eq!(text(q.get("sally").unwrap(), "posture"), "Always");
    let theirs = *s.g.players[rival].cities.first().unwrap();
    assert!(matches!(s.execute(&Command::CitySally { city: theirs, posture: Sally::Never }), Err(CmdErr::NotOwner)));
    let bad = Json::parse(&format!(r#"{{"c":"city_sally","city":{},"posture":"Sometimes"}}"#, city.0)).unwrap();
    assert!(Command::from_json(s.g.d, &bad).is_err());
    let good = Json::parse(&format!(r#"{{"c":"city_sally","city":{},"posture":"never"}}"#, city.0)).unwrap();
    assert_eq!(Command::from_json(s.g.d, &good).unwrap(), Command::CitySally { city, posture: Sally::Never });
    // The raider's side sees no posture (not its city).
    assert_eq!(view::raids::sally(&Viewer::new(&s.g, rival, false), city), Json::Null);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_contested_city_and_a_stripped_one() {
    let (mut s, st, root) = staged("raid-contested");
    let city: CityId = st.req("city").unwrap();
    let v = Viewer::new(&s.g, s.human, false);
    let r = view::raids::city(&v, city);
    assert_eq!(text(&r, "state"), "contested", "{r}");
    assert_eq!(text(&r, "label"), "Contested");
    assert_eq!(r.get("percent"), Some(&Json::from(10)));
    assert!(text(&r, "text").starts_with("Contested: -10% production and commerce."));
    assert!(!arr(&r, "calc").is_empty());
    // Stripped: the turns left, no calculation.
    s.g.cities[city].raid_stripped = 12;
    let r = view::raids::city(&Viewer::new(&s.g, s.human, false), city);
    assert_eq!(text(&r, "label"), "Stripped (12 turns)");
    assert_eq!(text(&r, "text"), "Stripped: no raid takes anything for 12 turns.");
    assert!(arr(&r, "calc").is_empty());
    // No raid: null.
    s.g.cities[city].raid_stripped = 0;
    s.g.cities[city].raid_percent = 0;
    assert_eq!(view::raids::city(&Viewer::new(&s.g, s.human, false), city), Json::Null);
    std::fs::remove_dir_all(root).ok();
}
