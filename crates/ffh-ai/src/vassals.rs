//! Vassal states (docs/AI.md, "Vassal states"; the owner's decision, 2026-10-21): what an expert
//! does once the game has gone against it.
//!
//! - **Hopelessly behind** -- far behind a strong neighbour in technology and power, with a small
//!   share of the world and no way back -- ask that neighbour for protection as its vassal, when
//!   the two are at least decently aligned: no good/evil opposition, and an attitude the leader's
//!   pride accepts (its `iVassalRefuseAttitudeThreshold`, one step more willing than FfH's AI,
//!   which refuses *at* the threshold). Never a neighbour whose wars would set us against someone
//!   next door who is stronger than we are: the rules make a vassal join its master's wars.
//! - **Crushed in a war** -- the enemy far stronger, the war going its way city after city, and
//!   no peace on offer -- capitulate to the one doing the damage, whatever we think of it.
//!
//! The same judgement answers a request (a human, or another AI, asking us to become its
//! vassal). As the master: a vassal is taken unless its wars would drag us into one we would lose
//! or against a friend (the rules make the master join its vassal's wars); a capitulation is taken
//! unless we can soon take the loser's last cities ourselves.
//!
//! Only what a human could see is read: power, technologies, the world's score shares, distance,
//! wars and their success. Pure functions of the game state; no RNG, no saved state.
//! `FFH_AI_VASSAL_OFF=ALL` (or `1`) turns all of it off, `FFH_AI_VASSAL_OFF=X,..` for the players
//! of those civilizations, each deciding for itself (the measurement switch
//! [`crate::switches::VASSAL`]; read once).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::*;
use ffh_data::*;

use crate::diplomacy::{attitude, is_team_leader, relation};

/// Neighbours: nearest cities at most this many plots apart (any landmass).
const NEIGHBOUR: i32 = 14;
/// Hopelessly behind: the master's power is at least this multiple of ours ...
const POWER_BEHIND: i64 = 3;
/// ... it knows this many more technologies, and at least this share (%) more than we do ...
const TECHS_BEHIND: i32 = 6;
const TECHS_BEHIND_PCT: i32 = 40;
/// ... and our share of the world is below this share (%) of an average civilization's.
const SHARE_BEHIND_PCT: i32 = 50;
/// Crushed: the enemy's power is at least this multiple of ours, and its war success exceeds ours
/// by the worth of this many captured cities (two when we have three cities or fewer left).
const POWER_CRUSHED: i64 = 2;
const CITIES_CRUSHED: i32 = 3;

/// Is vassal play off for player `p` (switch [`crate::switches::VASSAL`])?
pub fn off(g: &Game, p: PlayerId) -> bool {
    !crate::switches::VASSAL.on(g, p)
}

pub fn vassal_item(surrender: bool) -> TradeData {
    let item_type = if surrender { TradeableItem::TRADE_SURRENDER } else { TradeableItem::TRADE_VASSAL };
    TradeData { item_type, data: 0, offering: false, hidden: false }
}

fn is_vassal_item(t: &TradeData) -> bool {
    matches!(t.item_type, TradeableItem::TRADE_VASSAL | TradeableItem::TRADE_SURRENDER)
}

fn team_leader(g: &Game, t: TeamId) -> Option<PlayerId> {
    g.player_ids().find(|&q| g.players[q].alive && g.players[q].team == t)
}

fn team_techs(g: &Game, t: TeamId) -> i32 {
    g.teams[t].has_tech.iter().filter(|&&h| h).count() as i32
}

/// Plots between the nearest cities of two teams, on any landmass.
fn nearest(g: &Game, a: TeamId, b: TeamId) -> Option<i32> {
    let cities = |t: TeamId| -> Vec<(i32, i32)> {
        g.player_ids()
            .filter(|&q| g.players[q].alive && g.players[q].team == t)
            .flat_map(|q| g.players[q].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y)))
            .collect()
    };
    let (ca, cb) = (cities(a), cities(b));
    ca.iter().flat_map(|&(x, y)| cb.iter().map(move |&(u, v)| (x, y, u, v))).map(|(x, y, u, v)| g.plot_distance(x, y, u, v)).min()
}

fn opposed(g: &Game, a: PlayerId, b: PlayerId) -> bool {
    let (x, y) = (g.players[a].alignment, g.players[b].alignment);
    let (good, evil) = (Some(AlignmentType::ALIGNMENT_GOOD), Some(AlignmentType::ALIGNMENT_EVIL));
    (x == good && y == evil) || (x == evil && y == good)
}

/// A civ team that can take part: alive, not the barbarians, not a vassal.
fn free_civ(g: &Game, t: TeamId) -> bool {
    t != g.barbarian_team() && g.team_is_alive(t) && !g.team_is_a_vassal(t) && g.teams[t].num_cities > 0
}

/// Why player `p` (leading its team) would become `master`'s vassal in peace, or `None`.
pub fn hopeless_vassal(g: &Game, p: PlayerId, master: TeamId) -> Option<&'static str> {
    let team = g.players[p].team;
    if master == team || !free_civ(g, master) || !free_civ(g, team) || !g.teams[team].has_met[master] || g.at_war(team, master) {
        return None;
    }
    let master_leader = team_leader(g, master)?;
    if nearest(g, team, master)? > NEIGHBOUR {
        return None;
    }
    let ours = g.team_get_power(team, true).max(1) as i64;
    let theirs = g.team_get_power(master, true) as i64;
    if theirs < ours * POWER_BEHIND {
        return None;
    }
    let (our_techs, their_techs) = (team_techs(g, team), team_techs(g, master));
    if their_techs < our_techs + TECHS_BEHIND || their_techs * 100 < our_techs * (100 + TECHS_BEHIND_PCT) {
        return None;
    }
    let civs = g.civ_team_ids().filter(|&t| free_civ(g, t) || g.team_is_a_vassal(t) && g.team_is_alive(t)).count().max(1) as i32;
    if g.team_score[team] * civs * 100 > 10_000 * SHARE_BEHIND_PCT {
        return None;
    }
    // Decently aligned.
    if opposed(g, p, master_leader) {
        return None;
    }
    let threshold = g.players[p]
        .personality_type
        .and_then(|l| g.d.leader_heads[l].vassal_refuse_attitude_threshold)
        .unwrap_or(AttitudeType::ATTITUDE_CAUTIOUS);
    if attitude(g, p, master_leader) < threshold {
        return None;
    }
    // The master's wars become ours: not against a stronger neighbour we are at peace with.
    for x in g.civ_team_ids() {
        if x != team && x != master && g.team_is_alive(x) && g.at_war(master, x) && !g.at_war(team, x) {
            if nearest(g, team, x).is_some_and(|d| d <= NEIGHBOUR) && g.team_get_power(x, true) as i64 > ours {
                return None;
            }
        }
    }
    Some("hopeless")
}

/// Why `team` would capitulate to `enemy`, or `None`: crushed in the war, whatever it thinks of
/// the enemy.
pub fn crushed_by(g: &Game, team: TeamId, enemy: TeamId) -> Option<&'static str> {
    if !free_civ(g, team) || !free_civ(g, enemy) || !g.at_war(team, enemy) || g.teams[team].ai_at_war_counter[enemy] < 10 {
        return None;
    }
    let ours = g.team_get_power(team, true).max(1) as i64;
    let theirs = g.team_get_power(enemy, true) as i64;
    if theirs < ours * POWER_CRUSHED {
        return None;
    }
    let city = g.d.defines.war_success_city_capturing;
    let cities = if g.teams[team].num_cities <= 3 { CITIES_CRUSHED - 1 } else { CITIES_CRUSHED };
    if g.teams[enemy].ai_war_success[team] < g.teams[team].ai_war_success[enemy] + cities * city {
        return None;
    }
    Some("crushed")
}

/// Asks to become a vassal (once per turn from `diplomacy::manage`, after peace was sought): a
/// capitulation to the enemy crushing us, else, every 5 turns, vassalage to the strongest
/// neighbour we are hopelessly behind. True when we became a vassal.
pub fn seek(g: &mut Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    if off(g, p) || g.players[p].human || g.team_is_human(team) || g.player_is_barbarian(p) || !is_team_leader(g, p) || !free_civ(g, team) {
        return false;
    }
    // Capitulation: to the enemy that has done us the most damage.
    let enemy = g
        .civ_team_ids()
        .filter(|&e| crushed_by(g, team, e).is_some())
        .max_by_key(|&e| (g.teams[e].ai_war_success[team], std::cmp::Reverse(e)));
    if let Some(e) = enemy {
        if offer(g, p, e, true, "crushed") {
            return true;
        }
    }
    if (g.game_turn() + 3 * p.0 as i32) % 5 != 0 {
        return false;
    }
    let master = g
        .civ_team_ids()
        .filter(|&m| hopeless_vassal(g, p, m).is_some())
        .max_by_key(|&m| (g.team_get_power(m, true), std::cmp::Reverse(m)));
    match master {
        Some(m) => offer(g, p, m, false, "hopeless"),
        None => false,
    }
}

fn offer(g: &mut Game, p: PlayerId, master: TeamId, surrender: bool, why: &'static str) -> bool {
    let Some(to) = team_leader(g, master) else { return false };
    let item = vassal_item(surrender);
    if !g.player_can_trade_item(p, to, &item, true) || !g.player_can_contact(p, to) {
        return false;
    }
    // A human is asked on the leader's contact cadence, not every turn.
    let contact = if surrender { "CONTACT_PEACE_TREATY" } else { "CONTACT_PERMANENT_ALLIANCE" };
    if g.players[to].human && !crate::offers::contact_due(g, p, to, contact) {
        return false;
    }
    let ours = vec![item];
    let accepted = g.controller(to).consider_offer(g, to, p, &ours, &[]);
    let kind = if surrender { "capitulate" } else { "vassal" };
    g.tel_decision(p, "vassal_offer", if accepted { kind } else { "refused" }, why, master.0 as i64, String::new);
    crate::trace!(g, p, "offers to {} team {:?} ({}): {}", if surrender { "capitulate to" } else { "become the vassal of" }, master, why, if accepted { "accepted" } else { "refused" });
    if accepted {
        g.implement_deal(p, to, &ours, &[], false);
    }
    accepted && g.team_is_a_vassal(g.players[p].team)
}

/// `consider_offer` for a deal in which we would become `from`'s vassal (`we_give` holds the
/// vassal or capitulation item): agree when the same judgement would have us ask. `None` when the
/// deal is not of that kind.
pub fn agree(g: &Game, p: PlayerId, from: PlayerId, we_give: &[TradeData]) -> Option<bool> {
    let item = we_give.iter().find(|t| is_vassal_item(t))?;
    if off(g, p) || g.players[p].human || !we_give.iter().all(|t| is_vassal_item(t) || t.item_type == TradeableItem::TRADE_PEACE_TREATY) {
        return Some(false);
    }
    let master = g.players[from].team;
    let yes = if item.item_type == TradeableItem::TRADE_SURRENDER {
        crushed_by(g, g.players[p].team, master).is_some()
    } else {
        hopeless_vassal(g, p, master).is_some()
    };
    g.tel_decision(p, "vassal_asked", if yes { "agreed" } else { "refused" }, "", master.0 as i64, String::new);
    Some(yes)
}

/// `consider_offer` for a deal in which `from` would become our vassal (`they_give` holds the
/// item and we give nothing): take it unless its wars would drag us into one we would lose or
/// against a friend, or, for a capitulation, unless we can soon take its last cities ourselves.
/// `None` when the deal is not of that kind.
pub fn accept_master(g: &Game, p: PlayerId, from: PlayerId, they_give: &[TradeData], we_give: &[TradeData]) -> Option<bool> {
    let item = they_give.iter().find(|t| is_vassal_item(t))?;
    if off(g, p) {
        return Some(we_give.is_empty());
    }
    if !we_give.is_empty() {
        return Some(false);
    }
    let team = g.players[p].team;
    let them = g.players[from].team;
    let ours = g.team_get_power(team, true).max(1) as i64;
    let mut yes = true;
    let mut why = "take";
    if item.item_type == TradeableItem::TRADE_VASSAL {
        for x in g.civ_team_ids() {
            if x == team || x == them || !g.team_is_alive(x) || !g.at_war(them, x) || g.at_war(team, x) {
                continue;
            }
            let friend = team_leader(g, x).is_some_and(|q| relation(g, p, q) >= 2);
            if g.teams[team].force_peace[x] || !g.team_can_change_war_peace(team, x, false) || friend || g.team_get_power(x, true) as i64 > ours {
                yes = false;
                why = "their_wars";
                break;
            }
        }
    } else if g.teams[them].num_cities <= 2 && crate::diplomacy::press_conquest(g, team, them) {
        yes = false;
        why = "conquer_instead";
    }
    g.tel_decision(p, "vassal_taken", if yes { "accepted" } else { "refused" }, why, them.0 as i64, String::new);
    Some(yes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::tests::one_landmass_game;

    /// Player 0 strong (Feudalism, 30 techs, six times the power, most of the world), player 1 a
    /// one-city neighbour with 5 techs; both good.
    fn hopeless_pair() -> (Game, PlayerId, PlayerId) {
        let d = ffh_data::global();
        let mut g = one_landmass_game(31);
        let (strong, weak) = (PlayerId(0), PlayerId(1));
        g.player_found(strong, 4, 5);
        g.player_found(strong, 8, 5);
        g.player_found(weak, 14, 5);
        let (ts, tw) = (g.players[strong].team, g.players[weak].team);
        g.team_meet(ts, tw, false);
        g.team_set_has_tech(ts, d.t::<TechType>("TECH_FEUDALISM"), true, Some(strong), false, false);
        for (i, t) in d.techs.keys().enumerate() {
            if i < 30 {
                g.teams[ts].has_tech[t] = true;
            }
            if i < 5 {
                g.teams[tw].has_tech[t] = true;
            }
        }
        let good = Some(AlignmentType::ALIGNMENT_GOOD);
        g.players[strong].alignment = good;
        g.players[weak].alignment = good;
        g.players[strong].power = 600;
        g.players[weak].power = 100;
        g.team_score[ts] = 9000;
        g.team_score[tw] = 1000;
        (g, strong, weak)
    }

    #[test]
    fn a_hopeless_neighbour_asks_to_become_a_vassal_and_is_taken() {
        let (mut g, strong, weak) = hopeless_pair();
        let (ts, tw) = (g.players[strong].team, g.players[weak].team);
        assert_eq!(hopeless_vassal(&g, weak, ts), Some("hopeless"));
        assert_eq!(hopeless_vassal(&g, strong, tw), None, "the strong side is not behind");
        assert!(offer(&mut g, weak, ts, false, "hopeless"));
        assert!(g.teams[tw].vassal[ts] && !g.teams[tw].capitulated);
    }

    #[test]
    fn opposed_alignments_or_a_close_race_keep_a_civ_free() {
        let (mut g, strong, weak) = hopeless_pair();
        let ts = g.players[strong].team;
        g.players[weak].alignment = Some(AlignmentType::ALIGNMENT_EVIL);
        assert_eq!(hopeless_vassal(&g, weak, ts), None, "good and evil do not join");
        g.players[weak].alignment = Some(AlignmentType::ALIGNMENT_GOOD);
        g.players[weak].power = 300;
        assert_eq!(hopeless_vassal(&g, weak, ts), None, "half the power is not hopeless");
        g.players[weak].power = 100;
        g.team_score[g.players[weak].team] = 4000;
        assert_eq!(hopeless_vassal(&g, weak, ts), None, "a fair share of the world is not hopeless");
    }

    #[test]
    fn asked_to_become_a_vassal_it_agrees_only_when_hopeless() {
        let (mut g, strong, weak) = hopeless_pair();
        let ask = [vassal_item(false)];
        assert!(crate::diplomacy::consider_offer(&g, weak, strong, &[], &ask));
        g.players[weak].power = 400;
        assert!(!crate::diplomacy::consider_offer(&g, weak, strong, &[], &ask));
    }

    #[test]
    fn a_crushed_civ_capitulates_whatever_its_alignment() {
        let (mut g, strong, weak) = hopeless_pair();
        let (ts, tw) = (g.players[strong].team, g.players[weak].team);
        g.players[weak].alignment = Some(AlignmentType::ALIGNMENT_EVIL);
        g.team_declare_war(ts, tw, true, None);
        assert_eq!(crushed_by(&g, tw, ts), None, "a war that has just begun");
        g.teams[tw].ai_at_war_counter[ts] = 12;
        g.teams[ts].ai_war_success[tw] = 25;
        g.teams[tw].ai_war_success[ts] = 2;
        assert_eq!(crushed_by(&g, tw, ts), Some("crushed"));
        // A last city the conqueror can take is taken, not accepted as a vassal.
        let surrender = [vassal_item(true)];
        assert!(!crate::diplomacy::consider_offer(&g, strong, weak, &surrender, &[]));
        g.player_found(weak, 17, 8);
        g.player_found(weak, 14, 8);
        assert!(g.teams[tw].num_cities >= 3);
        assert!(crate::diplomacy::consider_offer(&g, strong, weak, &surrender, &[]));
        assert!(offer(&mut g, weak, ts, true, "crushed"));
        assert!(g.teams[tw].vassal[ts] && g.teams[tw].capitulated && !g.at_war(ts, tw));
    }
}
