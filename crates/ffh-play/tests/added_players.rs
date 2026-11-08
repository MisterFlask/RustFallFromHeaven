//! Players added mid-game (`Game::add_player_slot`, docs/ARCHITECTURE.md, "Players added
//! mid-game"): more civilizations are born than the four spare slots a game used to reserve, each
//! in a slot of its own after the barbarian's; they play turns under the AI, the human's views list
//! them, and the game saves and loads with them. The game cannot end (`common::STEADY`) and the
//! births are built by hand; nothing here depends on how a seed's game goes.

mod common;

use common::{bot_session, check, checkpoint, peaceful_game};
use ffh_core::ffh::cards::world::{self, Birth, NewCity, NewUnit};
use ffh_core::ids::*;
use ffh_data::*;
use ffh_play::query::Query;
use ffh_play::{bot, Json, Session, SessionOptions, Stop};

/// The civilizations born here: the cards' own (neither playable nor AI-playable, so no game
/// starts with them) and the Infernals.
const BIRTHS: [(&str, &str); 5] = [
    ("CIVILIZATION_PATRIANS", "LEADER_AURELIAN"),
    ("CIVILIZATION_DAWN_KNIGHTS", "LEADER_HALRIC"),
    ("CIVILIZATION_FEY_COURT", "LEADER_AINE"),
    ("CIVILIZATION_FREE_COMPANIES", "LEADER_VESK"),
    ("CIVILIZATION_HILL_CLANS", "LEADER_THRUMM"),
];

/// The ids of the players listed in the human's view (`players`).
fn viewed_players(s: &Session) -> Vec<i64> {
    let view = s.query(&Query::View).unwrap();
    let players = view.get("players").and_then(Json::as_arr).expect("the view lists the players");
    players.iter().filter_map(|p| p.get("id").and_then(Json::as_i64)).collect()
}

#[test]
fn civilizations_born_mid_game_play_turns_and_survive_a_reload() {
    let ng = peaceful_game(6);
    let (mut s, mut b) = bot_session(&ng, None);
    assert_eq!(s.advance(&mut |_| {}), Stop::Human);
    let d = s.g.d;
    let h = s.human;
    let start = s.g.num_players();
    let barbarian = s.g.barbarian_player();
    assert_eq!(barbarian.idx(), start - 1, "the game starts with the barbarian last, and no spare slot");

    // Six births: five of the cards' civilizations, one city each on the best free sites (two of
    // them the human's, three wild, as the Armageddon cards make them), then the Infernals.
    let land: Vec<PlotIdx> = s.g.map.plot_ids().filter(|&q| !s.g.plot_is_water(q)).collect();
    let sites = world::best_sites(&s.g, &land, BIRTHS.len() + 1, None, &[]);
    assert_eq!(sites.len(), BIRTHS.len() + 1, "a small map has room for six more cities at the start");
    let warrior = NewUnit::of(d.t::<UnitType>("UNIT_WARRIOR"));
    let mut born = Vec::new();
    for (i, &(civ, leader)) in BIRTHS.iter().enumerate() {
        let (civ, leader) = (d.t::<CivilizationType>(civ), d.t::<LeaderHeadType>(leader));
        let mut birth = if i < 2 { Birth::new(civ, leader, h) } else { Birth::wild(civ, leader) };
        birth.found = vec![NewCity { plot: sites[i], buildings: Vec::new(), units: vec![warrior.clone(), warrior.clone()] }];
        let expected = world::next_player(&s.g);
        let p = world::birth(&mut s.g, &birth);
        assert_eq!(p, expected);
        born.push(p);
    }
    let infernals = s.g.infernals_arrive(h, sites[BIRTHS.len()], false);
    born.push(infernals);

    let n = start + born.len();
    assert_eq!((s.g.num_players(), s.g.num_teams(), s.g.setup.players.len()), (n, n, n));
    assert_eq!(born, (start..n).map(|i| PlayerId(i as u8)).collect::<Vec<_>>(), "each in a new slot, after the barbarian's");
    assert_eq!(s.g.barbarian_player(), barbarian, "the barbarian keeps its id");
    assert_eq!(s.g.turn_order().last(), Some(barbarian), "and still moves last");
    for &p in &born {
        assert!(s.g.players[p].alive, "player {p:?} lives");
        assert!(s.g.is_civ_player_slot(p) && !s.g.player_is_barbarian(p));
        assert_eq!(s.g.players[p].team, TeamId(p.0), "on a team of its own");
        assert!(s.g.teams[TeamId(p.0)].at_war[s.g.barbarian_team()] || p == infernals, "at war with the barbarians");
        assert_eq!(s.g.players[p].cities.len(), (p != infernals) as usize, "one city each (the Infernals come with settlers)");
    }
    s.read(check::check);

    // The human's view lists the players its team has met: the two it founded.
    let viewed = viewed_players(&s);
    for &p in &born[..2] {
        assert!(viewed.contains(&(p.idx() as i64)), "the view lists born player {p:?}");
    }

    // They play turns (the AI plays them: the wild ones by the barbarian slot's controller).
    let until = s.g.game_turn() + 20;
    let stop = bot::run(&mut s, &mut b, until, |s, _| {
        assert!(s.g.num_game_turn_active <= 1, "turn {}: {} players active", s.g.game_turn(), s.g.num_game_turn_active);
    });
    assert_eq!(stop, Stop::Human);
    assert!(s.g.game_turn() >= until, "the game played its turns");
    s.read(check::check);
    for &p in &born {
        assert!(s.g.players[p].alive, "player {p:?} still lives (peace, no barbarians)");
        assert_eq!(s.g.players[p].score_history.len(), s.g.players[h].score_history.len(), "player {p:?} is scored every turn");
    }

    // The game saves and loads with them, and plays on.
    let (bytes, sidecar) = s.save_bytes().unwrap();
    let mut loaded = Session::load(&bytes, Some(&sidecar), b.answerer(), SessionOptions::default()).unwrap();
    assert!(loaded.g.save() == bytes, "save(load(save)) == save");
    assert_eq!(loaded.g.num_players(), n);
    assert_eq!(loaded.g.barbarian_player(), barbarian);
    assert_eq!(checkpoint(&loaded), checkpoint(&s));
    let until = s.g.game_turn() + 10;
    let mut b2 = ffh_play::bot::Bot::new(ng.seed);
    assert_eq!(bot::run(&mut loaded, &mut b2, until, |_, _| {}), Stop::Human);
    loaded.read(check::check);
    for &p in &born {
        assert!(loaded.g.players[p].alive);
    }
}
