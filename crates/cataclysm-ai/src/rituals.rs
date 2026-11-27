//! Rituals: one stance on Worldseal Strain per civilization, then the useful rituals with the
//! Mana left.
//!
//! Strain decides how the game ends: at 100 the Blowout comes and the holders of a Lifeboat win;
//! at turn 160 without it the Keepers of the Seal win, or else the highest score. So a
//! civilization pushes Strain up (Crack the Crust) when the Blowout would make it a winner, pulls
//! it down (Vent the Seal) when the turn limit would, and leaves it alone otherwise.

use cataclysm_core::game::{LIFEBOAT_CULTURE_MIN, MANA_BOOST_MIN};
use cataclysm_core::{Command, Event, FactionId, Game, PlayerId, RitualKind};
use std::cmp::Reverse;

use crate::try_cmd;

/// From this turn every civilization that would lose at the turn limit pushes Strain.
const LATE_PUSH_TURN: i32 = 110;
/// A stance ritual ready within this many turns keeps its Mana in reserve.
const RESERVE_TURNS: i32 = 3;
/// Mana from which Vent-Ether Surge is cast even with nothing left to research (the cap is 500).
const MANA_NEAR_CAP: i32 = 450;
/// Vent-Ether Surges at most per turn.
const MAX_SURGES: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stance {
    Push,
    Pull,
    Neither,
}

/// We would hold a Lifeboat if the Blowout came now: a built Wonder, or the city the Blowout
/// would name the Last Metropolis (the non-neutral city with the most Culture, at
/// `LIFEBOAT_CULTURE_MIN` or more; `Game::trigger_carcosa`).
fn prospective_lifeboat(g: &Game, p: PlayerId) -> bool {
    if g.built_wonders.values().any(|c| g.cities.get(c).is_some_and(|c| c.owner == p)) {
        return true;
    }
    g.cities
        .values()
        .filter(|c| !c.is_carcosa && c.owner != g.neutral_player && c.culture >= LIFEBOAT_CULTURE_MIN)
        .max_by_key(|c| (c.culture, Reverse(c.id)))
        .is_some_and(|c| c.owner == p)
}

fn stance(g: &Game, p: PlayerId) -> Stance {
    if g.apocalypse.carcosa_risen {
        return Stance::Neither;
    }
    let fac = g.players[p.idx()].faction;
    if fac == FactionId::Elohim {
        return Stance::Pull;
    }
    // The Wreckers and King Cinder's Host want the Furnace (flavor, and their victory).
    if matches!(fac, FactionId::Sheaim | FactionId::Infernals) {
        return Stance::Push;
    }
    let lifeboat = prospective_lifeboat(g, p);
    if lifeboat {
        return Stance::Push;
    }
    let keepers_alive = g.players.iter().any(|o| o.alive && o.faction == FactionId::Elohim);
    let top = g.players.iter().filter(|o| o.alive && !o.is_neutral).map(|o| o.score).max().unwrap_or(0);
    let leader = g.players[p.idx()].score >= top;
    if g.turn >= LATE_PUSH_TURN && (keepers_alive || !leader) {
        return Stance::Push;
    }
    if !keepers_alive && leader {
        return Stance::Pull;
    }
    Stance::Neither
}

fn cast(g: &mut Game, p: PlayerId, ritual: RitualKind, events: &mut Vec<Event>) -> bool {
    g.check_ritual(p, ritual).is_ok() && try_cmd(g, p, Command::CastRitual { ritual }, events)
}

/// A city of ours whose walls or garrison Seal the Masonry would help: besieged with Sentinels or
/// a squad in it, or with half or more of its Sentinel strength gone.
fn walls_need_repair(g: &Game, p: PlayerId) -> bool {
    g.cities.values().filter(|c| c.owner == p).any(|c| {
        let n = c.sentinel_damage.len() as i32;
        let garrison = g.squads.values().any(|s| s.owner == p && s.plot == c.plot);
        (c.besieged && (n > 0 || garrison)) || (n > 0 && c.sentinel_damage.iter().sum::<i32>() * 2 >= n * 100)
    })
}

/// `quiet`: the measuring policy that never raises Strain.
pub(crate) fn manage(g: &mut Game, p: PlayerId, quiet: bool, events: &mut Vec<Event>) {
    let mut st = stance(g, p);
    if quiet && st == Stance::Push {
        st = Stance::Neither;
    }
    let stance_ritual = match st {
        Stance::Push => Some(RitualKind::BlightStrike),
        Stance::Pull => Some(RitualKind::Sanctify),
        Stance::Neither => None,
    };
    if let Some(r) = stance_ritual {
        cast(g, p, r, events);
    }
    // The stance ritual's Mana is kept when it is ready again soon; battle spells keep theirs.
    let reserve = MANA_BOOST_MIN
        + stance_ritual.filter(|&r| g.ritual_turns_left(p, r) <= RESERVE_TURNS && g.check_ritual(p, r).is_err()).map_or(0, |r| r.mana_cost());
    let spare = |g: &Game, r: RitualKind| g.players[p.idx()].mana - r.mana_cost() >= reserve;

    if walls_need_repair(g, p) && spare(g, RitualKind::WardWalls) {
        cast(g, p, RitualKind::WardWalls, events);
    }
    let at_war = g.players.iter().any(|o| o.alive && !o.is_neutral && o.id != p && g.at_war(p, o.id));
    if at_war && g.player_squads(p).len() < g.squad_cap(p) && spare(g, RitualKind::SummonHost) {
        cast(g, p, RitualKind::SummonHost, events);
    }
    for _ in 0..MAX_SURGES {
        let useful = g.can_research_any(p) || g.players[p.idx()].mana >= MANA_NEAR_CAP;
        if !useful || !spare(g, RitualKind::ArcaneSurge) || !cast(g, p, RitualKind::ArcaneSurge, events) {
            break;
        }
    }
}
