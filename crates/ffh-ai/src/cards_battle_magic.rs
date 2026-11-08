//! The battle magic batch's AI (docs/CARDS.md): what the AI does with the cards that act in the
//! formation battles. The battles themselves need no AI (the auto formation places every unit,
//! a familiar in its caster's lane); this module trains the cards' units, values their
//! promotions and keeps familiars with their casters.
//!
//! - **Training** ([`turn`]): at war or planning one, a city trains the cards' national units up
//!   to their limits, one order a turn: the Whisperer, the Lutist and the Host Illusionist for the
//!   army, a Familiar for each caster we have, Homunculi once a city has the Vats (built in our
//!   most productive city when none has them).
//! - **Promotions** ([`promotion_value`], `units::best_promotion`'s hook): each card promotion by
//!   what it does for the unit that takes it (an Evoker only for a caster with a damage spell and
//!   no heal, a Spellsword's spell and a Homunculus's spell by their battle output, Contingency's
//!   recall for a caster that is hard to replace, its strike for the others).
//! - **Familiars** ([`turn`]): a familiar binds to the best caster in its stack, joins its group
//!   when they share a plot, and walks to it otherwise.

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::Choice;

/// The batch's types, looked up once (`None` without the cards' data).
struct Types {
    familiar: Option<UnitType>,
    bind: Option<SpellType>,
    homunculus: Option<UnitType>,
    vats: Option<BuildingType>,
    army_units: Vec<UnitType>,
}

fn types(d: &GameData) -> &'static Types {
    static T: OnceLock<Types> = OnceLock::new();
    T.get_or_init(|| Types {
        familiar: d.ty::<UnitType>("UNIT_FAMILIAR"),
        bind: d.ty::<SpellType>("SPELL_BIND_FAMILIAR"),
        homunculus: d.ty::<UnitType>("UNIT_HOMUNCULUS"),
        vats: d.ty::<BuildingType>("BUILDING_HOMUNCULUS_VATS"),
        army_units: ["UNIT_WHISPERER", "UNIT_LUTIST", "UNIT_HOST_ILLUSIONIST"].iter().filter_map(|n| d.ty::<UnitType>(n)).collect(),
    })
}

/// Once a turn (`cards::TURNS`): the familiars, then one training order.
pub fn turn(g: &mut Game, v: &crate::view::View) {
    let p = v.p;
    familiars(g, p);
    train(g, p);
}

/// Whether `p` is at war with a civilization or plans a war.
fn war_footing(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    crate::production::civ_enemies_at_war(g, p) || g.team_ids().any(|t| t != team && g.teams[team].ai_war_plan[t].is_some())
}

/// `p`'s casters (units with magic, not familiars), which a familiar may bind to.
fn casters(g: &Game, p: PlayerId) -> usize {
    g.players[p].units.iter().filter(|&&u| !g.unit_is_familiar(u) && g.unit_stat(u, StatType::Magic) > 0 && g.unit_engages_on_land(u)).count()
}

/// How many more units of type `t` `p` wants now (0 when it cannot train it).
fn wanted(g: &Game, p: PlayerId, t: UnitType, war: bool) -> i32 {
    let d = g.d;
    let tt = types(d);
    let Some(class) = d.units[t].unit_class_type else { return 0 };
    let limit = d.unit_classes[class].max_player_instances;
    let have = g.player_get_unit_class_count_plus_making(p, class);
    let want = if Some(t) == tt.familiar {
        (casters(g, p) as i32).min(limit)
    } else if war {
        limit
    } else {
        0
    };
    (want.min(limit) - have).max(0)
}

/// One training order a turn for the cards' units (module comment), in `p`'s most productive city
/// that can train it and is not training a unit; the Vats first where Homunculi are wanted.
fn train(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let tt = types(d);
    let war = war_footing(g, p);
    let mut wish: Vec<UnitType> = tt.army_units.clone();
    wish.extend(tt.familiar);
    wish.extend(tt.homunculus);
    for t in wish {
        if !g.player_can_train(p, t, false, false, false) || wanted(g, p, t, war) == 0 {
            continue;
        }
        let ai = d.units[t].default_unit_ai_type.unwrap_or(UnitAIType::UNITAI_ATTACK);
        if let Some(c) = best_city(g, p, |c| g.city_can_train(c, t, false, false, false, false) && g.city_production_unit(c).is_none()) {
            crate::production::push(g, c, Choice::Train(t, ai));
            return;
        }
        // A Homunculus needs the Vats: built in our most productive city when no city of ours
        // has them or builds them.
        if let (true, Some(b)) = (Some(t) == tt.homunculus, tt.vats) {
            if g.players[p].cities.iter().all(|&c| g.city_num_building(c, b) == 0 && g.city_production_building(c) != Some(b)) {
                if let Some(c) = best_city(g, p, |c| g.city_can_construct(c, b, false, false, false)) {
                    crate::production::push(g, c, Choice::Construct(b));
                    return;
                }
            }
        }
    }
}

/// `p`'s city with the most production among those `ok` accepts.
fn best_city(g: &Game, p: PlayerId, ok: impl Fn(CityId) -> bool) -> Option<CityId> {
    g.players[p].cities.iter().copied().filter(|&c| ok(c)).max_by_key(|&c| (g.city_yield_rate(c, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(c)))
}

/// Each familiar of `p`: it binds to the best caster in its stack, joins its caster's group on a
/// shared plot, and walks to its caster elsewhere.
fn familiars(g: &mut Game, p: PlayerId) {
    let tt = types(g.d);
    let (Some(familiar), Some(bind)) = (tt.familiar, tt.bind) else { return };
    for f in g.players[p].units.clone() {
        if !g.units.contains(f) || g.units[f].unit_type != familiar {
            continue;
        }
        if g.unit_can_cast(f, bind, false) {
            crate::magic::cast(g, f, bind, "familiar", 0);
        }
        let Some(c) = g.units[f].summoner.filter(|&c| g.units.contains(c) && !g.unit_is_dead(c)) else { continue };
        let (Some(fp), Some(cp)) = (g.unit_plot_opt(f), g.unit_plot_opt(c)) else { continue };
        if fp == cp {
            let cg = g.units[c].group;
            if cg.is_some() && g.units[f].group != cg {
                g.unit_join_group(f, cg, false);
            }
        } else if let Some(fg) = g.units[f].group.filter(|&fg| g.groups.contains(fg) && g.groups[fg].units.len() == 1) {
            if g.group_ready_to_move(fg, false) {
                crate::units::move_to(g, fg, cp, 0);
            }
        }
    }
}

/// What card promotion `promo` is worth to unit `u` (module comment); `v` its generic value, kept
/// for every other promotion.
pub fn promotion_value(g: &Game, u: UnitId, promo: PromotionType, v: i32) -> i32 {
    let d = g.d;
    let m = d.promotions[promo].magic;
    if !m.any() {
        return v;
    }
    let spells = g.unit_battle_spells(u);
    let damage = spells.attack.is_some();
    let mut value = v;
    if m.spell_power > 0 || m.damage_only {
        value += if damage && spells.heal.is_none() { 150 } else { -1000 };
    }
    if m.hex_ward || m.end_charms {
        value += 100;
    }
    if m.presence.is_some() {
        value += 90;
    }
    if m.revive > 0 {
        value += 120;
    }
    if m.dispel || m.damage_vs_summons > 0 {
        value += 70;
    }
    if let Some(s) = m.strike_spell {
        // Half its attack against magic defence: about a strike's damage, on the back row.
        value += 20 + (d.battle_spell_output(&d.spells[s]) / 600) as i32;
    }
    if let Some(s) = m.spell {
        let si = &d.spells[s];
        value += if si.battle_heal > 0 {
            70
        } else if si.battle_charm.is_some() {
            60
        } else {
            (d.battle_spell_output(si) / 150) as i32
        };
    }
    if m.spell_pattern.is_some() || m.front_caster {
        value += if damage { 140 } else { 20 };
    }
    // Contingency: the recall for a caster that is hard to replace (tier 4, or high level), the
    // strike for the others.
    let precious = d.units[g.units[u].unit_type].tier >= 4 || g.units[u].level >= 6 || g.unit_is_hero(u);
    if m.death_recall {
        value += if precious { 110 } else { 40 };
    }
    if m.opening_cast {
        value += if precious { 50 } else { 90 };
    }
    value.max(1)
}
