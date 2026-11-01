//! Khazad: the Dwarven Vaults. The rules give every Khazad city the vault level of the treasury's
//! gold per city (`cf_do_turn_khazad`, [`DWARVEN_VAULT_LEVELS`]): -2 happiness under 50, -1 under
//! 100, nothing under 150, +1 from 150, +2 and +10% production from 200, +2 and +25% from 300,
//! +3 and +40% from 500 (the vault buildings' data). The generic AI spends its treasury down to a
//! small reserve, so its cities sat in the Empty or Low vault: two unhappy citizens each.
//!
//! An expert Khazad keeps the gold: [`Khazad::gold_floor`] is the current level's threshold (the
//! spenders leave it), and [`Khazad::gold_target`] the highest level whose gain repays the gold
//! still missing within [`PAYBACK`] turns (half at war, when gold buys the army): the gain is the
//! level's happiness in the cities near their happy cap (a working citizen, about 3 commerce a
//! turn; 1 elsewhere) and its production modifier on the cities' production (a hammer worth 1.5
//! commerce), against the commerce the gold costs. `empire::sliders` then keeps a gold rate that
//! reaches the target in about 20 turns.
//!
//! Tried before (round 3, docs/AI.md): a gold target for the vault level, by score at turn 300:
//! more population, fewer techs, the same score. That version was judged by its research cost
//! alone; the owner's rule since (CLAUDE.md, "AI changes": civilizations use their unique
//! mechanics) keeps the vaults in play, and this one counts their production too.
//!
//! Also tried and measured (docs/AI_LOG.md): their siege engines (Trebuchet, Dwarven Cannon) need
//! a Siege Workshop and their knight (Hornguard) a Stable; generic now (`production::unit_building`).

use ffh_core::ffh::custom::DWARVEN_VAULT_LEVELS;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;

pub struct Khazad;

/// Turns within which a vault level must repay the gold it still needs.
const PAYBACK: i32 = 30;

impl CivPlay for Khazad {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { religion: Some("TECH_WAY_OF_THE_EARTHMOTHER"), beelines: vec!["TECH_IRON_WORKING"], ..Default::default() }
    }

    fn gold_target(&self, g: &Game, p: PlayerId) -> i32 {
        target(g, p)
    }

    fn gold_floor(&self, g: &Game, p: PlayerId) -> i32 {
        let n = g.players[p].cities.len() as i32;
        if n == 0 {
            return 0;
        }
        let level = current_level(g, p);
        DWARVEN_VAULT_LEVELS[level].0.max(0) * n
    }
}

/// The vault level of the treasury now.
fn current_level(g: &Game, p: PlayerId) -> usize {
    let n = g.players[p].cities.len() as i32;
    let gpc = if n == 0 { 0 } else { ffh_core::ffh::game_utils::py_div(g.players[p].gold, n) };
    DWARVEN_VAULT_LEVELS.iter().rposition(|&(min, _)| gpc >= min).unwrap_or(0)
}

/// What being at vault level `level` is worth a turn, in commerce: its happiness where a city is
/// near its happy cap, its production modifier on the city's production.
fn level_worth(g: &Game, p: PlayerId, level: usize) -> i32 {
    let d = g.d;
    let Some(b) = d.ty::<BuildingType>(DWARVEN_VAULT_LEVELS[level].1) else { return 0 };
    let info = &d.buildings[b];
    let mut v = 0;
    for &c in &g.players[p].cities {
        let room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
        v += info.happiness * if room <= 1 { 3 } else { 1 };
        let prod = g.cities[c].base_yield_rate[YieldType::YIELD_PRODUCTION.idx()].max(0);
        v += prod * info.yield_modifier[YieldType::YIELD_PRODUCTION.idx()] * 3 / 200;
    }
    v
}

/// The gold of the highest vault level that repays what it still needs within [`PAYBACK`] turns
/// (half at war), compared with the level we are at.
fn target(g: &Game, p: PlayerId) -> i32 {
    let n = g.players[p].cities.len() as i32;
    if n == 0 {
        return 0;
    }
    let gold = g.players[p].gold;
    let now = current_level(g, p);
    let base = level_worth(g, p, now);
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    let war = g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t));
    let payback = if war { PAYBACK / 2 } else { PAYBACK };
    let mut best = DWARVEN_VAULT_LEVELS[now].0.max(0) * n;
    for level in now + 1..DWARVEN_VAULT_LEVELS.len() {
        let need = DWARVEN_VAULT_LEVELS[level].0 * n - gold;
        let gain = level_worth(g, p, level) - base;
        if gain > 0 && need <= gain * payback {
            best = DWARVEN_VAULT_LEVELS[level].0 * n;
        }
    }
    best
}
