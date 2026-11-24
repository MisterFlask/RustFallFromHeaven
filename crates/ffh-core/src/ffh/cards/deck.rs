//! The deck's own callbacks: `SPELL_DISCOVER_CARD` (docs/DECKS.md, "The deck as built").

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellDiscoverCard" => spell_discover_card,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqDiscoverCard" => req_discover_card,
        _ => return None,
    };
    Some(f)
}

/// `reqDiscoverCard(caster)`: the great person holds a card whose needs its team knows.
fn req_discover_card(g: &Game, caster: UnitId, _call: &PyCall) -> Option<SpellUnmet> {
    if g.unit_can_discover_card(caster) {
        return None;
    }
    // Which of `unit_can_discover_card`'s checks refused, for people.
    Some(match g.units[caster].card {
        None => SpellUnmet::NoCard,
        Some(card) if g.teams[g.unit_get_team(caster)].has_tech[card] => SpellUnmet::CardKnown(card),
        Some(card) => SpellUnmet::CardNeeds(card),
    })
}

/// `spellDiscoverCard(caster)`: its team learns the card (the spell uses the caster up).
fn spell_discover_card(g: &mut Game, caster: UnitId, _call: &PyCall) {
    g.unit_discover_card(caster);
}
