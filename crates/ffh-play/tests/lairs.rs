//! Lair choices cross the human decision boundary, including ordinary and epic spells.
use std::sync::{Arc, Mutex};
use ffh_core::game::Game;
use ffh_core::rng::Rng;
use ffh_data::*;
use ffh_play::command::{CmdOk, Command};
use ffh_play::decision::{Decision, DecisionKind};
use ffh_play::human::{Answer, FnAnswerer};
use ffh_play::{Session, SessionOptions, Stop};

mod common;

#[test]
fn ordinary_and_epic_explorations_ask_once_before_effects_and_journal_the_second_choice() {
    for site in ["RUINS", "BROKEN_SEPULCHER"] {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let seen = asked.clone();
        let answerer = Arc::new(FnAnswerer(move |g: &Game, decision: &Decision| {
            if let DecisionKind::LairOutcome { unit } = decision.kind {
                assert_eq!(decision.options.len(), 2);
                assert_ne!(decision.options[0].label, decision.options[1].label);
                assert!(decision.options.iter().all(|o| o.enabled && o.help == "Lair remains."));
                assert_eq!(g.units[unit].experience, 0, "experience is awarded after choosing");
                assert_eq!(g.units[unit].moves, 0, "movement is finished after choosing");
                seen.lock().unwrap().push(decision.clone());
                Answer::Option(1)
            } else {
                Answer::Advisor
            }
        }));
        let mut s = Session::new_game(&common::steady_duel(3), answerer, SessionOptions::default()).unwrap();
        assert_eq!(s.advance(&mut |_| {}), Stop::Human);
        let h = s.human;
        let g = &mut s.g;
        common::allow_war(g);
        let barbarian = g.players[g.barbarian_player()].team;
        g.team_declare_war(g.players[h].team, barbarian, false, None);
        let starting = g.players[h].units[0];
        let (x, y) = (g.units[starting].x, g.units[starting].y);
        let unit = g.player_init_unit(h, g.d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        let plot = g.unit_plot(unit);
        let improvement = g.d.t::<ImprovementType>(&format!("IMPROVEMENT_{site}"));
        g.plot_set_improvement_type(plot, Some(improvement));
        g.soren_rand = Rng::new(0); // Explicitly arrange a Big bad category.
        let spell = g.d.t::<SpellType>(&format!("SPELL_EXPLORE_LAIR_{site}"));
        assert!(g.unit_can_cast(unit, spell, false));
        let command = Command::DoCommand { units: vec![unit], command: CommandType::COMMAND_CAST, data1: spell.idx() as i32, data2: -1, alt: false, confirmed: true };
        assert!(matches!(s.execute(&command), Ok(CmdOk::Applied { .. })));
        let seen = asked.lock().unwrap();
        assert_eq!(seen.len(), 1, "one question for this exploration");
        assert_eq!(seen[0].title, "Explore Lair: Big bad");
        assert_eq!(s.g.units[unit].experience, 1);
        assert_eq!(s.g.unit_moves_left(unit), 0);
        assert_eq!(s.g.map.plots[plot].improvement_type, Some(improvement));
        let lines = s.journal_lines().unwrap();
        assert!(lines.iter().any(|line| line.contains("\"kind\":\"lair_outcome\"") && line.contains("\"option\":1")), "{lines:?}");
    }
}

#[test]
fn event_triggered_exploration_uses_the_same_choice() {
    let asked = Arc::new(Mutex::new(0));
    let seen = asked.clone();
    let answerer = Arc::new(FnAnswerer(move |_: &Game, decision: &Decision| {
        if matches!(decision.kind, DecisionKind::LairOutcome { .. }) {
            assert_eq!(decision.options.len(), 2);
            *seen.lock().unwrap() += 1;
        }
        Answer::Option(1)
    }));
    let mut s = Session::new_game(&common::steady_duel(3), answerer, SessionOptions::default()).unwrap();
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let unit = s.g.players[s.human].units[0];
    assert_eq!(s.g.cf_explore_lair_big_bad(unit), 0);
    assert_eq!(*asked.lock().unwrap(), 1);
}
