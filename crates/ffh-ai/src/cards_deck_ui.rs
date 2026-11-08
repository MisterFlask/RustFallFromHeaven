//! The AI's use of the decks' UI batch of cards (docs/CARDS.md; their rules are
//! `ffh_core::ffh::cards::deck_ui`), once a turn from `cards::turn`:
//!
//! - **Starmetal Meteor:** cast while the player has no Mithril, on a plot that is or will be its
//!   land: its own plots first, then the unowned plots in its cities' radius; plots without a
//!   feature first (the meteor clears it), then the lowest plot index.
//! - **Anointed Champion:** cast on its highest-level combat unit (the most experience, then the
//!   strongest, then the oldest) once that unit has reached level [`CHAMPION_MIN_LEVEL`].
//! - **Legacy of Kings:** cast at once, for the trait its leader's flavors favour most
//!   ([`trait_score`]).
//!
//! The caster of a world spell is any unit that can cast it: a city defender first (the army
//! keeps its turn), then one that loses no promotion by casting, then the oldest.

use ffh_core::ffh::cards::deck_ui::{champion_candidates, legacy_traits, meteor_plots};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

/// The level a unit must have reached to be anointed (a lesser one is not worth the card yet).
pub const CHAMPION_MIN_LEVEL: i32 = 3;

/// The AI's turn for these cards.
pub fn turn(g: &mut Game, v: &crate::view::View) {
    let p = v.p;
    if v.barbarian || !g.players[p].alive {
        return;
    }
    meteor(g, p);
    champion(g, p);
    legacy(g, p);
}

/// `spell` if `p` can cast it at all now: its card is known and it was not cast yet.
fn ready(g: &Game, p: PlayerId, name: &str) -> Option<SpellType> {
    let spell = g.d.ty::<SpellType>(name)?;
    let team = g.players[p].team;
    let info = &g.d.spells[spell];
    if !g.team_is_has_tech(team, info.tech_prereq) || g.players[p].once_spells.contains(&spell) {
        return None;
    }
    Some(spell)
}

/// The unit of `p` that casts world spell `spell` (see the module comment), if one can.
pub fn world_caster(g: &Game, p: PlayerId, spell: SpellType) -> Option<UnitId> {
    let d = g.d;
    let loses = |u: UnitId| d.promotions.keys().filter(|&pr| g.units[u].has_promotion[pr] && d.promotions[pr].removed_by_casting).count();
    g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| g.unit_march(u).is_none() && g.unit_can_cast_untargeted(u, spell, false))
        .min_by_key(|&u| (g.units[u].ai_type != Some(UnitAIType::UNITAI_CITY_DEFENSE), loses(u), u))
}

/// Casts `spell` at `target` with `p`'s world caster; true if cast.
fn cast_at(g: &mut Game, p: PlayerId, spell: SpellType, target: SpellTarget, why: &str) -> bool {
    let Some(u) = world_caster(g, p, spell) else { return false };
    if !g.unit_can_cast_at(u, spell, target) {
        return false;
    }
    crate::trace!(g, p, "card spell {} at {:?} ({})", g.d.spells[spell].base.type_, target, why);
    g.tel_cast_reason(u, "card", 0);
    g.unit_cast_at(u, spell, target);
    true
}

// ---------------------------------------------------------------------------------------------
// Starmetal Meteor.

/// Whether `p` has Mithril: connected to a city, or on a plot of its land it sees.
fn has_mithril(g: &Game, p: PlayerId, mithril: BonusType) -> bool {
    let team = g.players[p].team;
    g.player_get_num_available_bonuses(p, mithril) > 0
        || g.map.plot_ids().any(|t| g.map.plots[t].owner == Some(p) && g.plot_get_bonus_type(t, Some(team)) == Some(mithril))
}

/// The plot `p`'s meteor falls on, if one suits: of [`meteor_plots`], those that are or will be
/// its land (its own plots, then unowned plots in the radius of one of its cities).
pub fn meteor_target(g: &Game, p: PlayerId) -> Option<PlotIdx> {
    let mut radius = vec![false; g.map.plots.len()];
    for &c in &g.players[p].cities {
        for i in 0..g.city_num_city_plots(c) as usize {
            if let Some(t) = g.city_index_plot(c, i) {
                radius[t.idx()] = true;
            }
        }
    }
    meteor_plots(g, p)
        .into_iter()
        .filter_map(|t| {
            let owner = g.map.plots[t].owner;
            let rank = match owner {
                Some(o) if o == p => 0,
                None if radius[t.idx()] => 1,
                _ => return None,
            };
            Some(((rank, g.map.plots[t].feature_type.is_some(), t), t))
        })
        .min_by_key(|&(key, _)| key)
        .map(|(_, t)| t)
}

fn meteor(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_STARMETAL_METEOR") else { return };
    let Some(mithril) = g.d.ty::<BonusType>("BONUS_MITHRIL") else { return };
    if has_mithril(g, p, mithril) {
        return;
    }
    if let Some(t) = meteor_target(g, p) {
        cast_at(g, p, spell, SpellTarget::Plot(t), "no Mithril");
    }
}

// ---------------------------------------------------------------------------------------------
// Anointed Champion.

/// The unit `p` anoints: its highest-level candidate, once that one has reached
/// [`CHAMPION_MIN_LEVEL`].
pub fn champion_target(g: &Game, p: PlayerId) -> Option<UnitId> {
    champion_candidates(g, p)
        .into_iter()
        .max_by_key(|&u| (g.units[u].level, g.units[u].experience, g.unit_base_combat_str(u), std::cmp::Reverse(u)))
        .filter(|&u| g.units[u].level >= CHAMPION_MIN_LEVEL)
}

fn champion(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_ANOINTED_CHAMPION") else { return };
    if let Some(u) = champion_target(g, p) {
        cast_at(g, p, spell, SpellTarget::Unit(u), "highest level");
    }
}

// ---------------------------------------------------------------------------------------------
// Legacy of Kings.

/// How much a leader with `flavor` (a flavor's value by name) wants trait `name`: the flavors
/// the trait serves, plus a small fixed preference that breaks ties toward the broadly useful
/// economic traits.
pub fn trait_score(name: &str, flavor: impl Fn(&str) -> i32) -> i32 {
    let (a, b, base) = match name {
        "TRAIT_AGGRESSIVE" => ("FLAVOR_MILITARY", "FLAVOR_MILITARY", 1),
        "TRAIT_RAIDERS" => ("FLAVOR_MILITARY", "FLAVOR_GOLD", 0),
        "TRAIT_CHARISMATIC" => ("FLAVOR_MILITARY", "FLAVOR_CULTURE", 1),
        "TRAIT_SPIRITUAL" => ("FLAVOR_RELIGION", "FLAVOR_RELIGION", 2),
        "TRAIT_EXPANSIVE" => ("FLAVOR_GROWTH", "FLAVOR_GROWTH", 2),
        "TRAIT_INDUSTRIOUS" => ("FLAVOR_PRODUCTION", "FLAVOR_PRODUCTION", 2),
        "TRAIT_CREATIVE" => ("FLAVOR_CULTURE", "FLAVOR_CULTURE", 1),
        "TRAIT_FINANCIAL" => ("FLAVOR_GOLD", "FLAVOR_GOLD", 3),
        "TRAIT_ORGANIZED" => ("FLAVOR_GOLD", "FLAVOR_PRODUCTION", 3),
        "TRAIT_ARCANE" => ("FLAVOR_SCIENCE", "FLAVOR_SCIENCE", 1),
        _ => return 0,
    };
    10 * (flavor(a) + flavor(b)) + base
}

/// The trait `p`'s leader takes, if it lacks one of them.
pub fn legacy_choice(g: &Game, p: PlayerId) -> Option<TraitType> {
    let d = g.d;
    let leader = g.players[p].personality_type;
    let flavor = |name: &str| match (leader, d.ty::<FlavorType>(name)) {
        (Some(l), Some(f)) => d.leader_heads[l].flavor_value[f],
        _ => 0,
    };
    legacy_traits(g, p).into_iter().max_by_key(|&t| (trait_score(&d.traits[t].base.type_, flavor), std::cmp::Reverse(t)))
}

fn legacy(g: &mut Game, p: PlayerId) {
    let Some(spell) = ready(g, p, "SPELL_LEGACY_OF_KINGS") else { return };
    if let Some(t) = legacy_choice(g, p) {
        cast_at(g, p, spell, SpellTarget::Choice(t.idx() as i32), "flavors");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffh_core::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 11));
        g.game_init();
        g
    }

    /// Player 0 knows `card` and has a city; returns the city.
    fn knows(g: &mut Game, card: &str) -> CityId {
        let d = g.d;
        let p = PlayerId(0);
        let team = g.players[p].team;
        g.team_set_has_tech(team, d.t::<TechType>(card), true, Some(p), false, false);
        if g.players[p].cities.is_empty() {
            let settler = g.players[p].units.iter().copied().find(|&u| d.units[g.units[u].unit_type].found).expect("a settler");
            let (x, y) = (g.units[settler].x, g.units[settler].y);
            g.player_init_city(p, x, y, true, true);
        }
        g.players[p].cities[0]
    }

    #[test]
    fn the_meteor_falls_on_our_land_while_we_lack_mithril() {
        let mut g = game();
        knows(&mut g, "TECH_CARD_STARMETAL_METEOR");
        let p = PlayerId(0);
        let t = meteor_target(&g, p).expect("a plot of our new city's land");
        assert_eq!(g.map.plots[t].owner, Some(p));
        meteor(&mut g, p);
        let mithril = g.d.t::<BonusType>("BONUS_MITHRIL");
        assert_eq!(g.map.plots[t].bonus_type, Some(mithril));
        // Cast once: with Mithril now, nothing more (and the spell is spent anyway).
        assert!(g.players[p].once_spells.contains(&g.d.t::<SpellType>("SPELL_STARMETAL_METEOR")));
    }

    #[test]
    fn the_champion_is_the_highest_level_unit_once_it_is_level_three() {
        let mut g = game();
        let c = knows(&mut g, "TECH_CARD_ANOINTED_CHAMPION");
        let p = PlayerId(0);
        let d = g.d;
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let a = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        let b = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        g.units[a].level = 2;
        g.units[b].level = 2;
        assert_eq!(champion_target(&g, p), None, "no unit of level 3 yet");
        g.units[b].level = 3;
        assert_eq!(champion_target(&g, p), Some(b));
        champion(&mut g, p);
        assert!(g.unit_is_hero(b));
        assert!(!g.unit_is_hero(a));
    }

    #[test]
    fn the_legacy_trait_follows_the_leaders_flavors() {
        let military = |n: &str| if n == "FLAVOR_MILITARY" { 10 } else { 0 };
        let gold = |n: &str| if n == "FLAVOR_GOLD" { 10 } else { 0 };
        let best = |f: &dyn Fn(&str) -> i32| {
            ffh_core::ffh::cards::deck_ui::LEGACY_TRAITS.iter().copied().max_by_key(|n| trait_score(n, f)).unwrap()
        };
        assert_eq!(best(&military), "TRAIT_AGGRESSIVE");
        assert_eq!(best(&gold), "TRAIT_FINANCIAL");
        let mut g = game();
        knows(&mut g, "TECH_CARD_LEGACY_OF_KINGS");
        let p = PlayerId(0);
        let t = legacy_choice(&g, p).expect("a trait the leader lacks");
        legacy(&mut g, p);
        assert!(g.players[p].traits[t] && g.players[p].card_traits == vec![t]);
    }
}
