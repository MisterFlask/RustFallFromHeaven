//! The battle magic batch's callbacks (docs/CARDS.md): the Familiar's bond. A familiar
//! (`battle_familiar`) binds to a caster in its stack (`SPELL_BIND_FAMILIAR`); the bond is the
//! familiar's `summoner`. In a battle it guards that caster (`engagement/battle_magic.rs`), and
//! each turn the caster gains 1 XP while they share a plot (`PROMOTION_FAMILIAR`'s per-turn
//! effect).

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellBindFamiliar" => spell_bind_familiar,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqBindFamiliar" => req_bind_familiar,
        _ => return None,
    };
    Some(f)
}

/// Unit callbacks (promotion `PyPerTurn`).
pub fn find_unit_fn(name: &str) -> Option<UnitFn> {
    let f: UnitFn = match name {
        "effectFamiliar" => effect_familiar,
        _ => return None,
    };
    Some(f)
}

impl Game {
    /// Whether unit `u` is a familiar (its type's `battle_familiar`).
    pub fn unit_is_familiar(&self, u: UnitId) -> bool {
        self.d.units[self.units[u].unit_type].magic.familiar
    }

    /// The caster familiar `f` binds to (`SPELL_BIND_FAMILIAR`): of its owner's units on its plot
    /// with magic, not familiars, not bound by another familiar of its owner, the highest level,
    /// then the most magic, then the lowest id; `None` when its bound caster is there already or
    /// there is none.
    pub fn familiar_bind_target(&self, f: UnitId) -> Option<UnitId> {
        let owner = self.units[f].owner;
        let plot = self.unit_plot_opt(f)?;
        let bound = self.units[f].summoner.filter(|&c| self.units.contains(c));
        if bound.is_some_and(|c| self.unit_plot_opt(c) == Some(plot)) {
            return None;
        }
        let taken = |c: UnitId| self.players[owner].units.iter().any(|&o| o != f && self.units[o].summoner == Some(c) && self.unit_is_familiar(o));
        self.map.plots[plot]
            .units
            .iter()
            .copied()
            .filter(|&c| c != f && self.units[c].owner == owner && !self.unit_is_familiar(c) && self.unit_stat(c, StatType::Magic) > 0 && !taken(c))
            .max_by_key(|&c| (self.units[c].level, self.unit_stat(c, StatType::Magic), std::cmp::Reverse(c)))
    }
}

/// `reqBindFamiliar(caster)`: a caster to bind to on its plot (`Game::familiar_bind_target`).
fn req_bind_familiar(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.unit_is_familiar(caster) && g.familiar_bind_target(caster).is_some()
}

/// `spellBindFamiliar(caster)`: the familiar binds to that caster.
fn spell_bind_familiar(g: &mut Game, caster: UnitId, _call: &PyCall) {
    if let Some(c) = g.familiar_bind_target(caster) {
        g.units[caster].summoner = Some(c);
    }
}

/// `effectFamiliar(pUnit)`, each turn: the familiar's bound caster gains 1 XP while they share a
/// plot.
fn effect_familiar(g: &mut Game, unit: UnitId, _call: &PyCall) {
    let Some(c) = g.units[unit].summoner.filter(|&c| g.units.contains(c) && !g.unit_is_dead(c)) else { return };
    if g.unit_plot_opt(c).is_some() && g.unit_plot_opt(c) == g.unit_plot_opt(unit) && g.units[c].owner == g.units[unit].owner {
        g.unit_change_experience(c, 1, -1, false, false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call() -> PyCall {
        PyCall { name: String::new(), args: Vec::new() }
    }

    /// A familiar binds to the highest-level caster on its plot, the caster gains 1 XP a turn
    /// while they share it, and nothing once it is elsewhere.
    #[test]
    fn a_familiar_binds_to_a_caster_and_gives_it_xp() {
        let d = ffh_data::global();
        let mut g = Game::new(d, crate::setup::GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
        g.game_init();
        let at = g.map.plot_ids().find(|&q| !g.plot_is_water(q) && !g.plot_is_peak(q) && g.map.plots[q].units.is_empty()).unwrap();
        let p = PlayerId(0);
        let (x, y) = g.map.xy(at);
        let fam = g.player_init_unit(p, d.t::<UnitType>("UNIT_FAMILIAR"), x, y, None, None);
        let adept = g.player_init_unit(p, d.t::<UnitType>("UNIT_ADEPT"), x, y, None, None);
        let mage = g.player_init_unit(p, d.t::<UnitType>("UNIT_MAGE"), x, y, None, None);
        g.units[mage].level = 3;
        let _warrior = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        assert_eq!(g.familiar_bind_target(fam), Some(mage), "the highest level caster");
        spell_bind_familiar(&mut g, fam, &call());
        assert_eq!(g.units[fam].summoner, Some(mage));
        assert_eq!(g.familiar_bind_target(fam), None, "bound already, its caster here");
        let xp = g.units[mage].experience;
        effect_familiar(&mut g, fam, &call());
        assert_eq!(g.units[mage].experience, xp + 1);
        let _ = adept;
        // Elsewhere: no XP.
        let other = g.map.plot_ids().find(|&q| q != at && !g.plot_is_water(q) && !g.plot_is_peak(q) && g.map.plots[q].units.is_empty()).unwrap();
        let (ox, oy) = g.map.xy(other);
        g.unit_set_xy(mage, ox, oy, false);
        effect_familiar(&mut g, fam, &call());
        assert_eq!(g.units[mage].experience, xp + 1);
    }
}
