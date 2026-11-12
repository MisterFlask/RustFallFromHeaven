//! The decks in the views and commands (docs/CARDS.md): a great person's card and its Discover
//! action, the card world spells and their targets (`cast_at`), and the Armageddon deck as the
//! human knows it. Each test builds its situation by hand in a game that cannot end
//! (`common::STEADY`), and asserts what the views and commands do with it, never a seed's story.

mod common;

use std::sync::Arc;

use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::command::{CmdErr, CmdOk, Command, Confirm, TargetRef};
use ffh_play::human::AdvisorAnswerer;
use ffh_play::query::Query;
use ffh_play::{Json, Session, SessionOptions, Stop};

/// A duel at the human's first control point, its capital founded.
fn duel(seed: u32) -> Session {
    let ng = common::steady_duel(seed);
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).unwrap();
    let r = s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    s
}

/// The human's team learns `tech`.
fn learn(s: &mut Session, tech: &str) {
    let h = s.human;
    let team = s.g.players[h].team;
    let t = s.g.d.t::<TechType>(tech);
    s.g.team_set_has_tech(team, t, true, Some(h), false, false);
}

fn arr<'a>(j: &'a Json, k: &str) -> &'a [Json] {
    j.get(k).and_then(Json::as_arr).unwrap_or(&[])
}

/// The human's world action for `spell`, if listed.
fn world_action(s: &Session, spell: SpellType) -> Option<Json> {
    let view = s.query(&Query::View).unwrap();
    arr(view.get("me").unwrap(), "world_actions").iter().find(|a| a.get("spell").and_then(Json::as_i64) == Some(spell.idx() as i64)).cloned()
}

fn flag(j: &Json, k: &str) -> bool {
    j.get(k).and_then(Json::as_bool).unwrap_or(false)
}

/// A target of an action's `targets` as `cast_at` takes it.
fn target_ref(t: &Json) -> TargetRef {
    let j = ffh_play::jobj! { "target" => t.get("target").unwrap().clone() };
    TargetRef::from_json(&j, "target").unwrap()
}

#[test]
fn a_card_world_spell_is_cast_at_a_choice_target() {
    let mut s = duel(31);
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_LEGACY_OF_KINGS");
    assert!(world_action(&s, spell).is_none(), "not offered before the card is known");
    learn(&mut s, "TECH_CARD_LEGACY_OF_KINGS");
    let a = world_action(&s, spell).expect("the card's world spell is the empire's action");
    assert!(flag(&a, "enabled") && flag(&a, "once") && !flag(&a, "cast"));
    assert_eq!(a.get("target_kind").and_then(Json::as_str), Some("choice"));
    let targets = arr(&a, "targets");
    let lacks: Vec<TraitType> = ffh_core::ffh::cards::deck_ui::legacy_traits(&s.g, h);
    assert_eq!(targets.len(), lacks.len(), "one option per trait the leader lacks");
    for (t, &tr) in targets.iter().zip(&lacks) {
        assert_eq!(t.get("label").and_then(Json::as_str), Some(d.text.name(&d.traits[tr].base.description)), "a choice is named");
        assert_eq!(t.get("info"), Some(&ffh_play::jarr!["TraitType", tr.idx() as i64]), "and points at its trait's help");
        assert_eq!(t.get("at"), Some(&Json::Null));
    }
    let caster = UnitId(a.get("caster").and_then(Json::as_i64).unwrap() as u32);
    let before = s.g.save();

    // Refused: a trait the leader has, a target of the wrong kind, another's unit, the untargeted
    // command, a spell without a target.
    let has = d.traits.keys().find(|&t| s.g.players[h].traits[t]).expect("the leader has a trait");
    let cast = |target: TargetRef, confirmed: bool| Command::CastAt { unit: caster, spell, target, confirmed };
    assert!(matches!(s.execute(&cast(TargetRef::Choice(has.idx() as i32), true)), Err(CmdErr::Cannot(_))));
    assert!(matches!(s.execute(&cast(TargetRef::Plot(0, 0), true)), Err(CmdErr::Invalid(_))));
    let theirs = s.g.units.iter().find(|(_, u)| u.owner != h).map(|(id, _)| id).unwrap();
    assert!(matches!(s.execute(&Command::CastAt { unit: theirs, spell, target: TargetRef::Choice(0), confirmed: true }), Err(CmdErr::NotOwner)));
    let untargeted = Command::DoCommand { units: vec![caster], command: CommandType::COMMAND_CAST, data1: spell.idx() as i32, data2: -1, alt: false, confirmed: true };
    assert!(matches!(s.execute(&untargeted), Err(CmdErr::Invalid(_))));
    let plain = d.spells.keys().find(|&x| d.spells[x].target.is_none()).unwrap();
    assert!(matches!(s.execute(&Command::CastAt { unit: caster, spell: plain, target: TargetRef::Choice(0), confirmed: true }), Err(CmdErr::Invalid(_))));
    assert!(before == s.g.save(), "refusals change nothing");

    // A valid target: the card spell asks first, then casts.
    let pick = target_ref(&targets[0]);
    match s.execute(&cast(pick, false)) {
        Ok(CmdOk::NeedsConfirm(Confirm::CastGlobal { spell: x })) => assert_eq!(x, spell),
        other => panic!("{other:?}"),
    }
    assert!(before == s.g.save(), "the question changes nothing");
    assert!(matches!(s.execute(&cast(pick, true)), Ok(CmdOk::Applied { .. })));
    assert!(s.g.players[h].traits[lacks[0]] && s.g.players[h].card_traits == vec![lacks[0]]);
    // Spent: still listed, disabled.
    let a = world_action(&s, spell).expect("a spent card spell stays listed");
    assert!(!flag(&a, "enabled") && flag(&a, "cast"));
    assert!(matches!(s.execute(&cast(target_ref(&targets[1]), true)), Err(CmdErr::Cannot(_))), "once a game");
}

#[test]
fn a_plot_target_is_picked_from_the_listed_plots() {
    let mut s = duel(32);
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_STARMETAL_METEOR");
    learn(&mut s, "TECH_CARD_STARMETAL_METEOR");
    let a = world_action(&s, spell).expect("listed");
    assert_eq!(a.get("target_kind").and_then(Json::as_str), Some("plot"));
    assert!(flag(&a, "enabled"), "the capital's land is a target");
    let caster = UnitId(a.get("caster").and_then(Json::as_i64).unwrap() as u32);
    // Each target is picked on the map at its plot.
    let targets = arr(&a, "targets");
    for t in targets {
        let at = t.get("at").and_then(Json::as_arr).unwrap();
        let TargetRef::Plot(x, y) = target_ref(t) else { panic!("a plot target") };
        assert_eq!((at[0].as_i64(), at[1].as_i64()), (Some(x as i64), Some(y as i64)));
    }
    // The capital's plot is not one: refused.
    let c = s.g.players[h].cities[0];
    let (cx, cy) = (s.g.cities[c].x, s.g.cities[c].y);
    let cast = |target: TargetRef| Command::CastAt { unit: caster, spell, target, confirmed: true };
    assert!(matches!(s.execute(&cast(TargetRef::Plot(cx, cy))), Err(CmdErr::Cannot(_))));
    assert!(matches!(s.execute(&cast(TargetRef::Plot(-1, 0))), Err(CmdErr::Invalid(_))), "off the map");
    let TargetRef::Plot(x, y) = target_ref(&targets[0]) else { unreachable!() };
    assert!(matches!(s.execute(&cast(TargetRef::Plot(x, y))), Ok(CmdOk::Applied { .. })));
    let t = s.g.map.plot_xy(x, y).unwrap();
    assert_eq!(s.g.map.plots[t].bonus_type, Some(d.t::<BonusType>("BONUS_MITHRIL")));
    // The message reached the human.
    assert!(s.messages().iter().any(|m| m.text.starts_with("Starmetal Meteor: Mithril at")), "{:?}", s.messages());
}

#[test]
fn a_unit_target_lists_the_human_units_by_plot() {
    let mut s = duel(33);
    let d = s.g.d;
    let h = s.human;
    let spell = d.t::<SpellType>("SPELL_ANOINTED_CHAMPION");
    learn(&mut s, "TECH_CARD_ANOINTED_CHAMPION");
    let c = s.g.players[h].cities[0];
    let (cx, cy) = (s.g.cities[c].x, s.g.cities[c].y);
    // Two units on the capital's plot: the plot offers both.
    let a1 = s.g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), cx, cy, None, None);
    let a2 = s.g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), cx, cy, None, None);
    let a = world_action(&s, spell).expect("listed");
    assert_eq!(a.get("target_kind").and_then(Json::as_str), Some("unit"));
    let here: Vec<&Json> = arr(&a, "targets").iter().filter(|t| t.get("at") == Some(&ffh_play::jarr![cx, cy])).collect();
    for u in [a1, a2] {
        assert!(here.iter().any(|t| target_ref(t) == TargetRef::Unit(u)), "unit {} is a target at its plot", u.0);
    }
    assert!(here.iter().all(|t| t.get("label").and_then(Json::as_str).is_some_and(|l| l.contains("level"))));
    let caster = UnitId(a.get("caster").and_then(Json::as_i64).unwrap() as u32);
    assert!(matches!(s.execute(&Command::CastAt { unit: caster, spell, target: TargetRef::Unit(a2), confirmed: true }), Ok(CmdOk::Applied { .. })));
    assert!(s.g.unit_is_hero(a2) && !s.g.unit_is_hero(a1));
}

#[test]
fn astral_leap_is_cast_from_a_selected_unit_in_a_city() {
    let mut s = duel(35);
    let d = s.g.d;
    let h = s.human;
    let leap = d.t::<SpellType>("SPELL_ASTRAL_LEAP");
    learn(&mut s, "TECH_CARD_ASTRAL_LEAP");
    // Not in the top bar: the unit casting it decides whose units leap (`cast_from_selection`).
    assert!(world_action(&s, leap).is_none());
    let c = s.g.players[h].cities[0];
    let (cx, cy) = (s.g.cities[c].x, s.g.cities[c].y);
    let u = s.g.player_init_unit(h, d.t::<UnitType>("UNIT_WARRIOR"), cx, cy, None, None);
    let gq = s.query(&Query::Group { units: vec![u] }).unwrap();
    let a = arr(&gq, "actions").iter().find(|a| a.get("spell").and_then(Json::as_i64) == Some(leap.idx() as i64)).cloned().expect("offered on a unit in a city");
    assert_eq!(a.get("target_kind").and_then(Json::as_str), Some("plot"));
    assert!(!arr(&a, "targets").is_empty());
}

#[test]
fn a_held_card_shows_in_the_unit_panel_with_its_discover_action() {
    let mut s = duel(34);
    let d = s.g.d;
    let h = s.human;
    let c = s.g.players[h].cities[0];
    let (cx, cy) = (s.g.cities[c].x, s.g.cities[c].y);
    let civ = s.g.player_get_civilization_type(h).unwrap();
    let sage = d.civ_unit(civ, d.t::<UnitClassType>("UNITCLASS_SCIENTIST")).expect("a sage");
    let u = s.g.player_init_unit(h, sage, cx, cy, None, None);
    let card = d.t::<TechType>("TECH_CARD_STARMETAL_METEOR");
    s.g.units[u].card = Some(card);
    // The view's unit names its card.
    let view = s.query(&Query::View).unwrap();
    let unit = arr(&view, "units").iter().find(|x| x.get("id").and_then(Json::as_i64) == Some(u.0 as i64)).unwrap();
    assert_eq!(unit.get("card").and_then(Json::as_i64), Some(card.idx() as i64));
    // The panel: the card's needs (unknown), and the Discover action named for it, disabled with
    // what it lacks.
    let group = |s: &Session| s.query(&Query::Group { units: vec![u] }).unwrap();
    let gq = group(&s);
    let detail = arr(&gq, "units")[0].get("card_detail").cloned().unwrap();
    assert_eq!(detail.get("tech").and_then(Json::as_i64), Some(card.idx() as i64));
    assert_eq!(detail.get("discoverable"), Some(&Json::Bool(false)));
    assert_eq!(detail.get("why").and_then(Json::as_str), Some("Needs Astronomy."));
    let discover = d.t::<SpellType>("SPELL_DISCOVER_CARD");
    let action = |gq: &Json| arr(gq, "actions").iter().find(|a| a.get("spell").and_then(Json::as_i64) == Some(discover.idx() as i64)).cloned();
    let a = action(&gq).expect("the Discover action shows while its needs are missing");
    assert_eq!(a.get("label").and_then(Json::as_str), Some("Discover: Starmetal Meteor"));
    assert!(!flag(&a, "enabled"));
    assert_eq!(a.get("why").and_then(Json::as_str), Some("Needs Astronomy."));
    // With its needs known it can be discovered.
    learn(&mut s, "TECH_ASTRONOMY");
    let gq = group(&s);
    assert_eq!(arr(&gq, "units")[0].get("card_detail").and_then(|x| x.get("discoverable")), Some(&Json::Bool(true)));
    let a = action(&gq).unwrap();
    assert!(flag(&a, "enabled") && a.get("why").is_none());
    let cmd = Command::from_json(d, a.get("cmd").unwrap()).unwrap();
    assert!(matches!(s.execute(&cmd), Ok(CmdOk::Applied { .. })));
    assert!(s.g.teams[s.g.players[h].team].has_tech[card], "the card is discovered");
    // The card tech's help says what the card is.
    let data = s.query(&Query::Data).unwrap();
    let entry = &arr(data.get("tables").unwrap(), "TechType")[card.idx()];
    let effects = entry.get("effects").and_then(Json::as_str).unwrap();
    assert!(effects.contains("Card: Sage, band II") && effects.contains("Needs: Astronomy") && effects.contains("Enables spell"), "{effects}");
}

#[test]
fn the_armageddon_deck_shows_revealed_fired_and_hidden_slots() {
    let mut s = duel(35);
    let view = s.query(&Query::View).unwrap();
    assert_eq!(view.get("me").and_then(|m| m.get("doom")), Some(&Json::Null), "not dealt before the first game turn ends");
    // The counter near 30: at the turn's end the deck is dealt and the card at 30 revealed (10 and
    // 20 fire), and the human is told.
    s.g.global_counter_limit = 100;
    s.g.global_counter = 26;
    let r = s.execute(&Command::EndTurn { force: true });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let view = s.query(&Query::View).unwrap();
    let doom = view.get("me").and_then(|m| m.get("doom")).cloned().unwrap();
    assert_eq!(arr(&doom, "slots").len(), 10);
    assert!(arr(&doom, "slots").iter().filter(|x| x.get("state").and_then(Json::as_str) == Some("hidden")).all(|x| x.get("card").is_none()), "a hidden slot names nothing");
    let slot = |t: i64| arr(&doom, "slots").iter().find(|x| x.get("threshold").and_then(Json::as_i64) == Some(t)).cloned().unwrap();
    let state = |t: i64| slot(t).get("state").and_then(Json::as_str).unwrap().to_string();
    assert_eq!(state(10), "fired");
    assert_eq!(state(20), "fired");
    assert_eq!(state(30), "revealed");
    assert_eq!(state(40), "hidden");
    let s30 = slot(30);
    let name = s30.get("name").and_then(Json::as_str).expect("a revealed card is named");
    assert!(s30.get("help").and_then(Json::as_str).is_some_and(|h| !h.is_empty()));
    assert!(s.messages().iter().any(|m| m.text == format!("Armageddon card revealed at 30: {name}.")), "{:?}", s.messages());
}

/// `ffh-play stage cards` (the browser smoke test's situation): the saved game loads with the two
/// card spells offered in the top bar, the Great Bard holding its card and the slot 30 revealed.
#[test]
fn the_staged_cards_save_has_card_spells_a_held_card_and_a_revealed_slot() {
    let dir = common::TempDir::new("stage-cards");
    let st = ffh_play::stage::cards(&dir.0).expect("staged");
    assert_eq!(st.to_json().get("stage").and_then(Json::as_str), Some("cards"));
    let (bytes, sidecar) = ffh_play::save::read_bundle(&st.save).unwrap();
    let opts = SessionOptions { journal: false, ..SessionOptions::default() };
    let s = Session::load(&bytes, sidecar.as_ref(), Arc::new(AdvisorAnswerer), opts).unwrap();
    for spell in [st.meteor, st.legacy] {
        let a = world_action(&s, spell).expect("a card spell in the top bar");
        assert!(flag(&a, "enabled"), "{}", s.g.d.spells[spell].base.type_);
    }
    assert_eq!(s.g.units[st.bard].card, Some(s.g.d.t::<TechType>("TECH_CARD_ANOINTED_CHAMPION")));
    assert!(s.g.doom.slots.iter().any(|x| x.threshold == 30 && x.revealed));
}

/// The civilization's world spell (docs/CARDS.md, "World rituals"): listed only while its cast is
/// pending.
#[test]
fn the_world_spell_is_listed_while_its_cast_is_pending() {
    let d = ffh_data::global();
    let mut ng = common::steady_duel(33);
    ng.civ = Some(d.t::<CivilizationType>("CIVILIZATION_HIPPUS"));
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| s.g.d.units[s.g.units[u].unit_type].found).unwrap();
    let r = s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false });
    assert!(matches!(r, Ok(CmdOk::Applied { .. })), "{r:?}");
    let spell = d.t::<SpellType>("SPELL_WARCRY");
    assert!(world_action(&s, spell).is_none(), "nothing pending: the civilization's world spell is not listed");
    assert!(s.g.player_grant_world_ritual_pending(h).is_some());
    let a = world_action(&s, spell).expect("listed while pending");
    assert!(flag(&a, "pending") && !flag(&a, "enabled"), "untargeted: it takes effect by itself: {a:?}");
    let other = d.t::<SpellType>("SPELL_RALLY");
    assert!(world_action(&s, other).is_none(), "another civilization's is not listed");
}
