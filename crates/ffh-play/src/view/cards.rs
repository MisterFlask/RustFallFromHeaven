//! The decks in the views (docs/CARDS.md): the card a great person holds ([`card_json`]), the
//! targets of a spell cast at a distance ([`targets_json`]) and the Armageddon deck as the human
//! knows it ([`doom_json`]).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::command::TargetRef;
use crate::json::Json;
use crate::names;
use crate::view::Viewer;

/// A spell's target kind as the protocol names it (`target_kind`).
pub fn target_kind_name(k: SpellTargetKind) -> &'static str {
    match k {
        SpellTargetKind::Plot => "plot",
        SpellTargetKind::City => "city",
        SpellTargetKind::Unit => "unit",
        SpellTargetKind::Player => "player",
        SpellTargetKind::Choice => "choice",
    }
}

/// Whether target `t` is of kind `k`.
pub fn target_is_kind(t: SpellTarget, k: SpellTargetKind) -> bool {
    matches!(
        (t, k),
        (SpellTarget::Plot(_), SpellTargetKind::Plot)
            | (SpellTarget::City(_), SpellTargetKind::City)
            | (SpellTarget::Unit(_), SpellTargetKind::Unit)
            | (SpellTarget::Player(_), SpellTargetKind::Player)
            | (SpellTarget::Choice(_), SpellTargetKind::Choice)
    )
}

/// The plot target `t` stands on (a plot, a city's plot, a unit's plot), for the map pick; `None`
/// for a player or a choice.
pub fn target_plot(g: &Game, t: SpellTarget) -> Option<(i32, i32)> {
    match t {
        SpellTarget::Plot(p) => Some(g.map.xy(p)),
        SpellTarget::City(c) => g.cities.get(c).map(|city| (city.x, city.y)),
        SpellTarget::Unit(u) => g.units.get(u).map(|unit| (unit.x, unit.y)),
        SpellTarget::Player(_) | SpellTarget::Choice(_) => None,
    }
}

/// The info table a choice spell's options are entries of: the kind its `py_target_label` names
/// with `labelInfo(pCaster, iChoice, 'TraitType')` (docs/CARDS.md, section 3).
pub fn choice_kind(d: &GameData, spell: SpellType) -> Option<String> {
    let call = ffh_core::ffh::pycall::PyCall::parse(&d.spells[spell].py_target_label)?;
    (call.name == "labelInfo").then(|| call.str_arg(2).map(str::to_string)).flatten()
}

/// The targets of `spell` for `caster`, as an action carries them: each `{target, at, label}`,
/// `target` the `cast_at` command's form, `at` the plot `[x, y]` it is picked on (null for a
/// player or a choice), `label` its name; a choice that is an info table's entry adds `info`:
/// `[kind, index]`, whose help the client shows; a target the client asks about before the cast
/// adds `confirm: {title, text, ok}` ([`crate::view::rituals::target_confirm`]).
pub fn targets_json(v: &Viewer, caster: UnitId, spell: SpellType, targets: &[SpellTarget]) -> Json {
    let g = v.g;
    let kind = choice_kind(g.d, spell);
    Json::Arr(
        targets
            .iter()
            .map(|&t| {
                let mut j = crate::jobj! {
                    "target" => TargetRef::of(g, t).to_json(),
                    "at" => target_plot(g, t).map(|(x, y)| crate::jarr![x, y]),
                    "label" => names::resolve_keys(g.d, &g.spell_target_label(caster, spell, t)).into_owned(),
                };
                if let (SpellTarget::Choice(i), Some(k)) = (t, kind.as_deref()) {
                    j.set("info", crate::jarr![k, i]);
                }
                // The question asked before the cast (Name the Tithe: the war it declares).
                if let Some(c) = crate::view::rituals::target_confirm(g, v.h, spell, t) {
                    j.set("confirm", c);
                }
                j
            })
            .collect(),
    )
}

/// A list of tech names for people: "A", "A and B", "A, B and C" (`or`: "or").
fn tech_list(d: &GameData, techs: &[TechType], or: bool) -> String {
    let names: Vec<String> = techs.iter().map(|&t| names::display(d, t).into_owned()).collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} {} {}", names[..n - 1].join(", "), if or { "or" } else { "and" }, names[n - 1]),
    }
}

/// Why the human's team cannot discover `card` now, for people (`None`: it can): the needs it
/// lacks, or that it knows the card.
pub fn card_why_not(v: &Viewer, card: TechType) -> Option<String> {
    let g = v.g;
    let d = g.d;
    if g.teams[v.team].has_tech[card] {
        return Some("Your team knows this card.".into());
    }
    let info = &d.techs[card];
    let known = |t: TechType| g.team_is_has_tech(v.team, Some(t));
    let and: Vec<TechType> = info.prereq_and_techs.iter().copied().filter(|&t| !known(t)).collect();
    let or_missing = !info.prereq_or_techs.is_empty() && !info.prereq_or_techs.iter().any(|&t| known(t));
    match (and.is_empty(), or_missing) {
        (true, false) => None,
        (false, false) => Some(format!("Needs {}.", tech_list(d, &and, false))),
        (true, true) => Some(format!("Needs {}.", tech_list(d, &info.prereq_or_techs, true))),
        (false, true) => Some(format!("Needs {}, and {}.", tech_list(d, &and, false), tech_list(d, &info.prereq_or_techs, true))),
    }
}

/// The card `card` a great person of the human's holds, as the unit panel shows it: its suit
/// (the great person's class), band (1 to 3), alignment and faith, its needs with whether the
/// team knows each (`needs`: all of them; `needs_one_of`: one of them), whether the team knows the
/// card, whether it can be discovered now and, when not, why (`why`).
pub fn card_json(v: &Viewer, card: TechType) -> Json {
    let g = v.g;
    let d = g.d;
    let info = &d.techs[card];
    let c = info.card.as_ref();
    let need = |&t: &TechType| crate::jobj! { "tech" => Json::idx(t), "known" => g.team_is_has_tech(v.team, Some(t)) };
    let why = card_why_not(v, card);
    crate::jobj! {
        "tech" => Json::idx(card),
        "suit" => Json::opt_idx(c.map(|c| c.suit)),
        "band" => c.map_or(Json::Null, |c| Json::from(c.band)),
        "alignment" => Json::opt_idx(c.and_then(|c| c.alignment)),
        "religion" => Json::opt_idx(c.and_then(|c| c.religion)),
        "needs" => Json::Arr(info.prereq_and_techs.iter().map(need).collect()),
        "needs_one_of" => Json::Arr(info.prereq_or_techs.iter().map(need).collect()),
        "known" => g.teams[v.team].has_tech[card],
        "discoverable" => why.is_none(),
        "why" => why,
    }
}

/// The Armageddon deck as the human knows it (`me.doom`): null before the deal or with the
/// counter off; else `{counter, slots}`, each slot `{threshold, state}` with `state` `fired`,
/// `revealed` (shown to everyone), `seen` (not revealed, but the human sees it ahead:
/// `doom_slot_known`) or `hidden`, and for a slot the human knows its card: `card` (its key, or
/// null for an empty slot), `name`, `help` (the card's effect), `stage` (0-based) and `stages`.
pub fn doom_json(v: &Viewer) -> Json {
    let g = v.g;
    doom_json_with(g, |i| v.reveal || g.doom_slot_known(v.h, i))
}

/// [`doom_json`] with `known(i)`: whether the viewer knows slot `i`'s card.
pub fn doom_json_with(g: &Game, known: impl Fn(usize) -> bool) -> Json {
    if !g.doom.dealt || g.is_option(GameOptionType::GAMEOPTION_NO_GLOBAL_COUNTER) {
        return Json::Null;
    }
    let d = g.d;
    let slots = g.doom.slots.iter().enumerate().map(|(i, s)| {
        let state = if s.fired {
            "fired"
        } else if s.revealed {
            "revealed"
        } else if known(i) {
            "seen"
        } else {
            "hidden"
        };
        let mut j = crate::jobj! { "threshold" => s.threshold, "state" => state };
        if state != "hidden" {
            match g.doom_slot_card(i) {
                Some((c, stage)) => {
                    j.set("card", c.key);
                    j.set("name", names::text(d, &format!("TXT_KEY_{}", c.key)).to_string());
                    j.set("help", names::text(d, &format!("TXT_KEY_{}_HELP", c.key)).to_string());
                    j.set("stage", stage);
                    j.set("stages", c.stages.len());
                }
                None => {
                    j.set("card", Json::Null);
                }
            }
        }
        j
    });
    crate::jobj! { "counter" => g.get_global_counter(), "slots" => Json::Arr(slots.collect()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    fn slot<'a>(j: &'a Json, threshold: i64) -> &'a Json {
        j.get("slots").and_then(Json::as_arr).unwrap().iter().find(|s| s.get("threshold").and_then(Json::as_i64) == Some(threshold)).unwrap()
    }

    #[test]
    fn the_doom_slots_show_what_the_player_knows() {
        let mut g = game();
        assert_eq!(doom_json_with(&g, |_| false), Json::Null, "nothing before the deal");
        g.doom_deal();
        // Slot 30 revealed, slot 10 fired (revealed first), slot 50 seen ahead, the rest hidden.
        let i10 = g.doom.slots.iter().position(|s| s.threshold == 10).unwrap();
        let i30 = g.doom.slots.iter().position(|s| s.threshold == 30).unwrap();
        let i50 = g.doom.slots.iter().position(|s| s.threshold == 50).unwrap();
        g.doom.slots[i10].revealed = true;
        g.doom.slots[i10].fired = true;
        g.doom.slots[i30].revealed = true;
        let j = doom_json_with(&g, |i| g.doom.slots[i].revealed || i == i50);
        assert_eq!(j.get("slots").and_then(Json::as_arr).unwrap().len(), 10);
        let state = |t| slot(&j, t).get("state").and_then(Json::as_str).unwrap().to_string();
        assert_eq!(state(10), "fired");
        assert_eq!(state(30), "revealed");
        assert_eq!(state(50), "seen");
        assert_eq!(state(70), "hidden");
        // A hidden slot says nothing of its card; a known one names it (or says it is empty).
        assert!(slot(&j, 70).get("card").is_none());
        for t in [30, 50] {
            let s = slot(&j, t);
            match s.get("card") {
                Some(Json::Str(key)) => {
                    assert_eq!(s.get("name").and_then(Json::as_str), Some(g.d.text.get(&format!("TXT_KEY_{key}"))));
                    assert!(s.get("help").and_then(Json::as_str).is_some_and(|h| !h.is_empty() && !h.starts_with("TXT_KEY")));
                }
                Some(Json::Null) => {}
                other => panic!("slot {t}: {other:?}"),
            }
        }
        // The revealed slot names the card dealt there, and its stage.
        let (c, stage) = g.doom_slot_card(i30).expect("a card is dealt at 30");
        assert_eq!(slot(&j, 30).get("card").and_then(Json::as_str), Some(c.key));
        assert_eq!(slot(&j, 30).get("stage").and_then(Json::as_i64), Some(stage as i64));
    }

    #[test]
    fn a_held_card_names_its_needs() {
        let mut g = game();
        let d = g.d;
        let h = PlayerId(0);
        let v = Viewer::new(&g, h, false);
        let card = d.t::<TechType>("TECH_CARD_STARMETAL_METEOR");
        let j = card_json(&v, card);
        assert_eq!(j.get("discoverable"), Some(&Json::Bool(false)));
        assert_eq!(j.get("why").and_then(Json::as_str), Some("Needs Astronomy."));
        let astronomy = d.t::<TechType>("TECH_ASTRONOMY");
        let needs = j.get("needs").and_then(Json::as_arr).unwrap();
        assert_eq!(needs[0].get("tech").and_then(Json::as_i64), Some(astronomy.idx() as i64));
        assert_eq!(needs[0].get("known"), Some(&Json::Bool(false)));
        let team = g.players[h].team;
        g.team_set_has_tech(team, astronomy, true, Some(h), false, false);
        let v = Viewer::new(&g, h, false);
        let j = card_json(&v, card);
        assert_eq!(j.get("discoverable"), Some(&Json::Bool(true)));
        assert_eq!(j.get("why"), Some(&Json::Null));
    }
}
