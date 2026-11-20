//! The Culture screen and the city screen's Culture section (docs/PERSONS.md, "The fourth build",
//! rule 10, and "as built, part 2A"; docs/CULTURE.md, "UI") on the staged scene (`ffh-play stage
//! culture`: the `persons` stage with a religion in every city of the human) through the session's
//! queries: every number the view sends is the rules' read API's (the head's inflow and its base,
//! the channel's preview and its requirement's parts, the options and their refusals, the courted
//! Houses' courtships and their Guard previews), and the city's line reads its culture and what it
//! diverts. Nothing here depends on how a game goes.

mod common;

use std::sync::Arc;

use ffh_core::culture::{ChannelOrder, ChannelTarget};
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::json::Json;
use ffh_play::stage::StagedPersons;

fn load(name: &str) -> (common::TempDir, StagedPersons, ffh_play::Session) {
    let dir = common::TempDir::new(name);
    let st = ffh_play::stage::culture(&dir.0).expect("staged");
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = ffh_play::SessionOptions { journal: false, ..ffh_play::SessionOptions::default() };
    let s = ffh_play::Session::load(&bytes, sidecar.as_ref(), Arc::new(ffh_play::human::AdvisorAnswerer), opts).unwrap();
    (dir, st, s)
}

fn q(s: &ffh_play::Session, text: &str) -> Json {
    s.query_json(&Json::parse(text).unwrap()).unwrap_or_else(|e| panic!("{text}: {e:?}"))
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::as_arr).unwrap_or_else(|| panic!("no array `{key}` in {j}"))
}

fn int(j: &Json, key: &str) -> i64 {
    j.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("no integer `{key}` in {j}"))
}

fn opt_int(j: &Json, key: &str) -> Option<i64> {
    j.get(key).and_then(Json::as_i64)
}

fn s_of<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("no string `{key}` in {j}"))
}

fn obj<'a>(j: &'a Json, key: &str) -> &'a Json {
    j.get(key).filter(|v| !matches!(v, Json::Null)).unwrap_or_else(|| panic!("no object `{key}` in {j}"))
}

fn house(st: &StagedPersons, role: &str) -> HouseId {
    st.houses.iter().find(|x| x.role == role).unwrap().id
}

/// The `culture` query on the staged scene: the head, the human's Court channel and its preview,
/// the chooser's three actions with their options (Spread Religion's refused one), the courted
/// House with its Guard preview, the keywords.
#[test]
fn the_culture_screen_reads_the_rules() {
    let (_dir, st, s) = load("culture-screen");
    let g = &s.g;
    let h = s.human;
    let r = q(&s, r#"{"q":"culture"}"#);
    assert!(matches!(r.get("refusal"), Some(Json::Null)), "the human may channel");
    assert_eq!(int(&r, "inflow"), g.player_culture_inflow_now(h) as i64);
    assert_eq!(int(&r, "base"), g.player_culture_channel_base_now(h) as i64);
    assert_eq!(int(&r, "percent"), g.d.defines.culture_channel_percent as i64);
    let keys: Vec<&str> = arr(&r, "reminders").iter().map(|k| s_of(k, "key")).collect();
    assert_eq!(keys, ["KEYWORD_CHANNEL", "KEYWORD_COURT", "KEYWORD_GUARD"]);

    // The human's channel: the Court of the neighbour's House, when the stage could set it.
    let neighbour = house(&st, "neighbour");
    if let Some(ch) = g.players[h].culture_channel.clone() {
        let c = obj(&r, "channel");
        assert_eq!(s_of(c, "name"), g.culture_channel_name(&ch.order()));
        assert_eq!(int(c, "since_turn"), ch.since_turn as i64);
        assert_eq!(ch.target, ChannelTarget::House(neighbour));
        let pv = obj(&r, "preview");
        let court = obj(pv, "court");
        let cv = g.house_courtship_view(neighbour, h);
        let req = g.house_requirement(neighbour, h);
        let rq = obj(court, "requirement");
        assert_eq!((int(rq, "total"), int(rq, "loyalty"), int(rq, "attitude"), int(rq, "unit")), (req.total as i64, req.loyalty as i64, req.attitude as i64, req.unit as i64));
        assert_eq!(s_of(rq, "formula"), req.formula());
        assert_eq!((int(court, "favour"), int(court, "gain"), int(court, "settles")), (cv.favour as i64, cv.gain as i64, cv.settles as i64));
        assert_eq!(opt_int(court, "turns"), cv.turns.map(i64::from));
        let split = obj(court, "split");
        let sp = cv.split.clone().expect("a split view");
        assert_eq!(arr(split, "going").iter().map(|c| int(c, "id")).collect::<Vec<_>>(), sp.going.iter().map(|c| c.0 as i64).collect::<Vec<_>>());
        assert_eq!(split.get("whole").and_then(Json::as_bool), Some(sp.whole));
    } else {
        assert!(st.hand_set.iter().any(|l| l.starts_with("the human's Court of the neighbour's House refused")), "the stage lists the refusal");
    }

    // The chooser: one entry per action, each with targets or the line of why none.
    let actions = arr(&r, "actions");
    assert_eq!(actions.len(), g.d.cultural_actions.len());
    for a in actions {
        let options = arr(a, "options");
        assert!(!options.is_empty() || a.get("none").and_then(Json::as_str).is_some(), "targets or why none: {a}");
        for o in options {
            // A refused target has no preview; an allowed one has its full preview.
            match o.get("refusal").and_then(Json::as_str) {
                Some(why) => assert!(!why.is_empty() && matches!(o.get("preview"), Some(Json::Null))),
                None => assert!(obj(o, "preview").get("order").is_some()),
            }
        }
    }
    let spread = actions.iter().find(|a| s_of(a, "effect") == "spread_religion").expect("Spread Religion");
    let religion = ReligionType::from_usize(arr(spread, "religions")[0].as_i64().unwrap() as usize);
    let own = arr(spread, "options").iter().find(|o| opt_int(obj(o, "target"), "player") == Some(h.0 as i64)).expect("your own civilization");
    let order = ChannelOrder { action: g.culture_action(CultureEffect::SpreadReligion).unwrap(), target: ChannelTarget::Player(h), religion: Some(religion) };
    assert_eq!(own.get("refusal").and_then(Json::as_str), g.culture_channel_check(h, &order).err().as_deref(), "the rules' refusal");
    assert!(own.get("refusal").and_then(Json::as_str).is_some(), "every city of yours has the religion: refused");
    let civ = g.player_get_civilization_short_description(h);
    assert_eq!(s_of(own, "label"), format!("{civ} (yours)"));
    // A House is a civilization of its own: Spread Religion names it as a House.
    let friendly = st.houses.iter().find(|x| x.role == "friendly").unwrap();
    let o = arr(spread, "options").iter().find(|o| opt_int(obj(o, "target"), "player") == Some(friendly.player.0 as i64)).expect("your House among Spread Religion's targets");
    assert_eq!(s_of(o, "label"), format!("{} ({civ})", friendly.label));

    // Aimed at you: the neighbour's courtship of House `pleased`, past two thirds, with the Guard
    // button's preview at your inflow now.
    let pleased = house(&st, "pleased");
    let aimed = arr(&r, "aimed");
    let x = aimed.iter().find(|x| int(x, "house") == pleased.0 as i64).expect("House pleased is courted");
    let cs = arr(x, "courtships");
    let ai = st.neighbour;
    let line = cs.iter().find(|c| int(c, "courter") == ai.0 as i64).expect("the neighbour's courtship");
    assert_eq!(line.get("attention").and_then(Json::as_bool), Some(true));
    assert_eq!(s_of(line, "line"), g.house_courtship_view(pleased, ai).line);
    let gv = g.house_guard_with(pleased, g.player_culture_inflow_now(h));
    let guard = obj(x, "guard");
    assert_eq!(int(guard, "loss"), gv.loss as i64);
    let gc = arr(guard, "courtships").iter().find(|c| int(c, "courter") == ai.0 as i64).expect("the guard's row");
    let want = gv.courtships.iter().find(|c| c.courter == ai).unwrap();
    assert_eq!((int(gc, "settles_with"), int(gc, "settles_without")), (want.settles_with as i64, want.settles_without as i64));
    assert_eq!((opt_int(gc, "turns_with"), opt_int(gc, "turns_without")), (want.turns_with.map(i64::from), want.turns_without.map(i64::from)));
    assert_eq!(int(gc, "requirement"), g.house_requirement(pleased, ai).total as i64);
    assert_eq!(x.get("guarded").and_then(Json::as_bool), Some(false));
    assert_eq!(int(obj(obj(x, "guard_order"), "target"), "player"), g.houses[pleased].player.0 as i64);
}

/// The city screen's Culture section reads the city's culture and what it diverts now, with the
/// channel's name and the keyword Channel.
#[test]
fn the_city_section_reads_the_city() {
    let (_dir, st, s) = load("culture-city");
    let g = &s.g;
    let c = st.capital;
    let city = q(&s, &format!(r#"{{"q":"city","city":{}}}"#, c.0));
    let k = obj(&city, "cultural");
    assert_eq!(int(k, "rate"), g.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE) as i64);
    assert_eq!(int(k, "diverted"), g.city_culture_diverted_times100(c) as i64);
    assert_eq!(k.get("disorder").and_then(Json::as_bool), Some(g.city_is_disorder(c)));
    match &g.players[s.human].culture_channel {
        Some(ch) => assert_eq!(s_of(obj(k, "channel"), "name"), g.culture_channel_name(&ch.order())),
        None => assert!(matches!(k.get("channel"), Some(Json::Null))),
    }
    assert_eq!(arr(k, "reminders").iter().map(|r| s_of(r, "key")).collect::<Vec<_>>(), ["KEYWORD_CHANNEL"]);
}
