//! Batch B8's cards (docs/CARDS.md, section 5): when the AI casts the world spells of the second
//! batch of new civilizations, and at what. The civilizations they make are played by the normal
//! AI: a born one by its founder's controller, an Armageddon one (the Scaled Kingdom, the Brood, the
//! Infernals of the Infernal Gate) by the AI; their own units and rules need nothing here.
//!
//! - **Rite of the Necropolis, Wake the Hill Clans, Call the Aifon** are cast as soon as they can
//!   be: each makes a civilization on land that is free now (ruins, hills, a coast), and the land
//!   and the player slots go to whoever is first.
//! - **The Usurper's Crown** splits the rival an expert would weaken before a war on it: of the
//!   civilizations it may split, one we are at war with, else one we plan a war on, else one we are
//!   at most Cautious toward; the strongest of them. Never our vassal, our master or a defensive
//!   ally.
//! - **Oath of Fealty** ends a won war: the enemy it may take with the most cities.

use std::cmp::Reverse;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::view::View;

/// The world spells cast as soon as they can be.
const AT_ONCE: [&str; 3] = ["SPELL_RITE_OF_THE_NECROPOLIS", "SPELL_WAKE_THE_HILL_CLANS", "SPELL_CALL_THE_AIFON"];

/// The batch's turn (`cards::TURNS`).
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian {
        return;
    }
    let p = v.p;
    for name in AT_ONCE {
        if let Some(spell) = g.d.ty::<SpellType>(name) {
            if let Some(u) = caster(g, p, spell) {
                if g.unit_can_cast(u, spell, false) {
                    crate::magic::cast(g, u, spell, "card", 0);
                }
            }
        }
    }
    usurpers_crown(g, p);
    oath_of_fealty(g, p);
}

/// A unit of `p`'s that may cast its world spell `spell` (once a game, any unit): the first on the
/// map that has not cast this turn, while `p`'s team knows the spell's card and has not cast it.
fn caster(g: &Game, p: PlayerId, spell: SpellType) -> Option<UnitId> {
    let info = &g.d.spells[spell];
    let team = g.players[p].team;
    if info.tech_prereq.is_some_and(|t| !g.team_is_has_tech(team, Some(t))) || g.player_cast_once(p, spell) {
        return None;
    }
    g.players[p].units.iter().copied().find(|&u| !g.units[u].has_casted && g.unit_plot_opt(u).is_some())
}

/// Casts `spell` with `u` at `target` (the trace line and the telemetry reason `value`).
fn cast_at(g: &mut Game, u: UnitId, spell: SpellType, target: SpellTarget, what: &str, value: i64) {
    if g.unit_can_cast_at(u, spell, target) {
        let p = g.units[u].owner;
        crate::trace!(g, p, "{what}");
        g.tel_cast_reason(u, "card", value);
        g.unit_cast_at(u, spell, target);
    }
}

/// The Usurper's Crown: the rival to split (the module comment).
fn usurpers_crown(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_USURPERS_CROWN") else { return };
    let Some(u) = caster(g, p, spell) else { return };
    if !g.unit_can_cast_untargeted(u, spell, false) {
        return;
    }
    let team = g.players[p].team;
    let mut best: Option<((bool, bool, i32), PlayerId)> = None;
    for target in g.unit_spell_targets(u, spell) {
        let SpellTarget::Player(q) = target else { continue };
        let qt = g.players[q].team;
        if g.teams[qt].vassal[team] || g.teams[team].vassal[qt] || g.teams[team].defensive_pact[qt] {
            continue;
        }
        let war = g.at_war(team, qt);
        let plan = g.teams[team].ai_war_plan[qt].is_some();
        let rival = crate::diplomacy::attitude(g, p, q) <= AttitudeType::ATTITUDE_CAUTIOUS;
        if !(war || plan || rival) {
            continue;
        }
        let key = (war, plan, g.team_get_power(qt, false));
        if best.map_or(true, |(k, b)| (key, Reverse(q)) > (k, Reverse(b))) {
            best = Some((key, q));
        }
    }
    let Some((_, q)) = best else { return };
    let what = format!("crowns a usurper in {}", g.player_get_civilization_description(q));
    cast_at(g, u, spell, SpellTarget::Player(q), &what, g.players[q].cities.len() as i64);
}

/// Oath of Fealty: the enemy with the most cities becomes our vassal.
fn oath_of_fealty(g: &mut Game, p: PlayerId) {
    let Some(spell) = g.d.ty::<SpellType>("SPELL_OATH_OF_FEALTY") else { return };
    let Some(u) = caster(g, p, spell) else { return };
    let best = g
        .unit_spell_targets(u, spell)
        .into_iter()
        .filter_map(|t| match t {
            SpellTarget::Player(q) => Some(q),
            _ => None,
        })
        .max_by_key(|&q| (g.players[q].cities.len(), Reverse(q)));
    let Some(q) = best else { return };
    let what = format!("takes the fealty of {}", g.player_get_civilization_description(q));
    cast_at(g, u, spell, SpellTarget::Player(q), &what, g.players[q].cities.len() as i64);
}
