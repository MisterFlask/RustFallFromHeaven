//! Breakaway leaders have game metadata but cannot be selected for a new game.

use ffh_data::*;
use ffh_play::setup::setup_lists;
use ffh_play::{Json, NewGame};

#[test]
fn preferred_breakaway_leaders_are_absent_from_every_selectable_list() {
    let d = ffh_data::global();
    let lists = setup_lists(d);
    let civilizations = lists.req_arr("civilizations").unwrap();
    let metadata = lists.req_arr("leaders").unwrap();
    let preferred: Vec<_> = d.leader_heads.keys().filter(|&l| d.leader_heads[l].preferred_breakaway).collect();
    assert_eq!(preferred.len(), 105);
    for leader in preferred {
        let name = d.type_name(leader);
        assert_eq!(metadata[leader.idx()].req_str("type").unwrap(), name, "{name}: metadata stays indexed for in-game portraits");
        for civ in civilizations {
            assert!(!civ.req_arr("leaders").unwrap().contains(&Json::idx(leader)), "{name} is selectable for {}", civ.req_str("type").unwrap());
        }
    }
}

#[test]
fn explicit_new_game_requests_cannot_select_a_preferred_breakaway_leader() {
    let d = ffh_data::global();
    for leader in d.leader_heads.keys().filter(|&l| d.leader_heads[l].preferred_breakaway) {
        let name = d.type_name(leader);
        for lead_any_civ in [false, true] {
            let mut ng = NewGame::new(d, 1);
            ng.leader = Some(leader);
            if lead_any_civ {
                ng.options.push(GameOptionType::GAMEOPTION_LEAD_ANY_CIV);
            }
            assert!(ng.resolve_civ(d).is_err(), "{name}: leader-only request accepted");
            ng.civ = d.leader_heads[leader].minor_leader_of;
            assert!(ng.resolve_civ(d).is_err(), "{name}: civilization-plus-leader request accepted");
        }
    }
}
