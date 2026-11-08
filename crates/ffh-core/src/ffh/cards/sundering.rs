//! The Sundering (Armageddon, 90, sharing the slot with Wrath; the owner's design, 2026-11-08):
//! every large civilization loses its far cities to a new faction of its own civilization, led by
//! another leader of that civilization. Its `valid` and `fire` functions are listed in
//! `deck::doom_cards::CARDS`; its text and the minor leaders invented for it are in
//! `data/json/cards/doom_sundering.jsonc`.
//!
//! - **Who splits** ([`sundering_plan`]): in player id order, every living civilization (not the
//!   barbarians, a minor civilization or a crusade Host; not the Sheaim or the Infernals) with
//!   [`MIN_CITIES`] cities or more (more than 3) and a capital, for which a leader is free ([`faction_leader`]).
//! - **What secedes** ([`seceding_cities`]): the quarter of its cities farthest from its capital
//!   (rounded down, at least one; ties in distance: the lower city id), released as the Usurper's
//!   Crown releases them (`world::birth`'s `release`: buildings, culture and land garrisons stay
//!   with the city, heroes with the parent). The most populous of them is the faction's capital.
//! - **The faction**: its parent's civilization, a leader of its own, its parent's techs (not its
//!   cards), met with every civilization its parent has met, nobody's vassal, at peace with
//!   everyone but the barbarians (no war is forced, docs/DECKS.md principle 27; a leader allied
//!   with the barbarians, the Barbarian trait, is at peace with them too, as at a game's start),
//!   [`GRUDGE`] attitude between it and its parent, both ways. The AI plays it (`Birth::wild`'s
//!   slot controller). It is named for its capital: "<civilization> of <city>".

use std::cmp::Reverse;

use crate::ffh::cards::world::{self, Birth};
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// The fewest cities of a civilization that splits ("more than 3").
pub const MIN_CITIES: usize = 4;
/// The share of its cities that secede: one in this many (rounded down, at least one).
pub const SECEDE_DIVISOR: usize = 4;
/// The attitude modifier between a faction and its parent, both ways (FFH's scale, 4 a step).
pub const GRUDGE: i32 = -4;
/// The civilizations that never split.
const NEVER_SPLIT: [&str; 2] = ["CIVILIZATION_SHEAIM", "CIVILIZATION_INFERNAL"];

/// Is leader `l` in play this game: the leader of a player that is or was alive?
fn leader_in_play(g: &Game, l: LeaderHeadType) -> bool {
    g.player_ids().any(|p| g.players[p].ever_alive && g.player_get_leader_type(p) == Some(l))
}

/// The leader a new faction of civilization `civ` gets: the first (by id) of the civilization's
/// own leaders (`leaders`) not in play this game and not in `taken`, else the first of its minor
/// leaders (`minor_leader_of`) that is free; `None` when every one is in play.
pub fn faction_leader(g: &Game, civ: CivilizationType, taken: &[LeaderHeadType]) -> Option<LeaderHeadType> {
    let d = g.d;
    let free = |l: LeaderHeadType| !taken.contains(&l) && !leader_in_play(g, l);
    let own = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l] && free(l));
    own.or_else(|| d.leader_heads.keys().find(|&l| d.leader_heads[l].minor_leader_of == Some(civ) && free(l)))
}

/// May civilization `q` split (a leader aside): living, not the barbarians, a minor civilization,
/// a crusade Host, the Sheaim or the Infernals, with [`MIN_CITIES`] cities or more and a capital.
fn may_split(g: &Game, q: PlayerId) -> bool {
    let d = g.d;
    let Some(civ) = g.player_get_civilization_type(q) else { return false };
    if NEVER_SPLIT.iter().any(|&name| d.ty::<CivilizationType>(name) == Some(civ)) {
        return false;
    }
    g.players[q].cities.len() >= MIN_CITIES && g.player_get_capital_city(q).is_some()
}

/// The civilizations that split, in player id order, each with its faction's leader (two
/// factions of one civilization get different leaders).
pub fn sundering_plan(g: &Game) -> Vec<(PlayerId, LeaderHeadType)> {
    let mut plan: Vec<(PlayerId, LeaderHeadType)> = Vec::new();
    for q in world::civ_players(g) {
        if !may_split(g, q) {
            continue;
        }
        let Some(civ) = g.player_get_civilization_type(q) else { continue };
        let taken: Vec<LeaderHeadType> = plan.iter().map(|&(_, l)| l).collect();
        if let Some(l) = faction_leader(g, civ, &taken) {
            plan.push((q, l));
        }
    }
    plan
}

/// The cities of `q` that secede: the quarter of them (rounded down, at least one) farthest from
/// its capital (plot distance; ties: the lower city id), the most populous first (ties: the lower
/// city id): the first is the faction's capital.
pub fn seceding_cities(g: &Game, q: PlayerId) -> Vec<CityId> {
    let Some(capital) = g.player_get_capital_city(q) else { return Vec::new() };
    let (cx, cy) = g.map.xy(g.city_plot(capital));
    let n = (g.players[q].cities.len() / SECEDE_DIVISOR).max(1);
    let mut far: Vec<(Reverse<i32>, CityId)> = g.players[q]
        .cities
        .iter()
        .copied()
        .filter(|&c| c != capital && g.cities.contains(c))
        .map(|c| {
            let (x, y) = g.map.xy(g.city_plot(c));
            (Reverse(g.plot_distance(cx, cy, x, y)), c)
        })
        .collect();
    far.sort();
    let mut out: Vec<CityId> = far.into_iter().take(n).map(|(_, c)| c).collect();
    out.sort_by_key(|&c| (Reverse(g.cities[c].population), c));
    out
}

/// The Sundering is played while some civilization would split.
pub fn valid_sundering(g: &Game) -> bool {
    !sundering_plan(g).is_empty()
}

/// Every civilization of [`sundering_plan`] splits, in player id order: its [`seceding_cities`]
/// form a new faction of its civilization with its leader (the module comment). The plan is drawn
/// before the first split, so a faction born now does not split again.
pub fn fire_sundering(g: &mut Game, _: usize) {
    let turn = g.game_turn();
    for (q, leader) in sundering_plan(g) {
        if !g.players[q].alive {
            continue;
        }
        let Some(civ) = g.player_get_civilization_type(q) else { continue };
        let cities = seceding_cities(g, q);
        let Some(&capital) = cities.first() else { continue };
        let mut b = Birth::wild(civ, leader);
        b.parent = Some(q);
        let city = g.d.text.name(&g.cities[capital].name).to_string();
        b.description = Some(format!("{} of {}", g.player_get_civilization_short_description(q), city));
        b.release = cities;
        b.parent_attitude = GRUDGE;
        let np = world::birth(g, &b);
        g.players[q].ai_attitude_extra[np] += GRUDGE;
        let (pt, nt) = (g.players[q].team, g.players[np].team);
        // It has met everyone its parent has met.
        for t in g.civ_team_ids() {
            if t != nt && t != pt && g.teams[pt].has_met[t] && !g.teams[nt].has_met[t] {
                g.team_meet(nt, t, true);
            }
        }
        // A leader allied with the barbarians (the Barbarian trait) is at peace with them, as at a
        // game's start (`init_diplomacy`); `birth` set war on both sides.
        if g.teams[nt].barbarian_ally {
            let barbarians = g.players[g.barbarian_player()].team;
            g.teams[nt].at_war[barbarians] = false;
            g.teams[barbarians].at_war[nt] = false;
        }
        let (name, by) = (g.player_get_civilization_description(np), g.player_get_civilization_description(q));
        let n = g.players[np].cities.len();
        g.log.msg(turn, None, || format!("The Sundering: {n} cities of {by} secede as {name}."));
    }
}
