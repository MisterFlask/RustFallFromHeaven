//! The cards of the decks' UI batch (docs/CARDS.md): three world spells that give the targeted
//! spells' interface one target of each kind it picks on the map or from a list.
//!
//! - **G17 Starmetal Meteor** (Sage, II, needs Astronomy): a plot. A land plot (not a peak) the
//!   caster's team sees, within [`METEOR_RANGE`] plots of its owner's borders, with no city, no
//!   improvement, no unit and no resource the team sees. Its feature is destroyed and it gains
//!   Mithril (docs/DECKS_ROUND8.md C.4: it destroys the improvement or feature only).
//! - **B18 Anointed Champion** (Bard, II, needs Arete): a unit. A combat unit of the caster's
//!   owner that is not a hero or a summon gains the Hero promotion and two levels. A hero's extra
//!   battle action does not stack with tier 4's (`GameData::engage_extra_actions` gives one either
//!   way).
//! - **B07 Legacy of Kings** (Bard, II, needs Philosophy): a choice. The leader gains one of
//!   [`LEGACY_TRAITS`] it lacks (Philosophical excluded, C.4: it feeds the deck), for good: the
//!   Adaptive and Insane trait swaps keep it (`Player::card_traits`).

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::SpellTarget;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellStarmetalMeteor" => spell_starmetal_meteor,
        "spellAnointedChampion" => spell_anointed_champion,
        "spellLegacyOfKings" => spell_legacy_of_kings,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsStarmetalMeteor" => targets_starmetal_meteor,
        "targetsAnointedChampion" => targets_anointed_champion,
        "targetsLegacyOfKings" => targets_legacy_of_kings,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// G17 Starmetal Meteor.

/// How far from its caster's borders the meteor may fall.
pub const METEOR_RANGE: i32 = 6;

/// Whether plot `t` is one player `p`'s meteor may strike, given whether it is within
/// [`METEOR_RANGE`] of `p`'s borders (`near`). A unit or a resource the team does not see is no
/// bar: a hidden unit is not harmed, and a hidden resource gives way to the Mithril (the target
/// list must not tell the player what it cannot see).
fn meteor_plot_ok(g: &Game, p: PlayerId, t: PlotIdx) -> bool {
    let team = g.players[p].team;
    let plot = &g.map.plots[t];
    !g.plot_is_water(t)
        && !g.plot_is_peak(t)
        && plot.plot_city.is_none()
        && plot.improvement_type.is_none()
        && g.plot_is_visible(t, team)
        && g.plot_get_bonus_type(t, Some(team)).is_none()
        && plot.units.iter().all(|&u| g.unit_is_invisible(u, team, false))
}

/// The plots player `p`'s Starmetal Meteor may strike, in plot order.
pub fn meteor_plots(g: &Game, p: PlayerId) -> Vec<PlotIdx> {
    let near = crate::ffh::cards::world::near_borders(g, p, METEOR_RANGE);
    g.map.plot_ids().filter(|&t| near[t.idx()] && meteor_plot_ok(g, p, t)).collect()
}

/// `targetsStarmetalMeteor(pCaster)`.
fn targets_starmetal_meteor(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    meteor_plots(g, g.units[caster].owner).into_iter().map(SpellTarget::Plot).collect()
}

/// `spellStarmetalMeteor(pCaster)`: the target plot loses its improvement and feature and gains
/// Mithril.
fn spell_starmetal_meteor(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if g.map.plots[t].improvement_type.is_some() {
        g.plot_set_improvement_type(t, None);
    }
    if g.map.plots[t].feature_type.is_some() {
        g.plot_set_feature_type(t, None);
    }
    let mithril = g.d.t::<BonusType>("BONUS_MITHRIL");
    g.plot_set_bonus_type(t, Some(mithril));
    let owner = g.units[caster].owner;
    let (x, y) = g.map.xy(t);
    let turn = g.game_turn();
    g.log.msg_at(turn, Some(owner), (x, y), || format!("Starmetal Meteor: Mithril at ({x}, {y})."));
}

// ---------------------------------------------------------------------------------------------
// B18 Anointed Champion.

/// How many levels the champion gains.
pub const CHAMPION_LEVELS: i32 = 2;

/// The units player `p`'s Anointed Champion may make a hero, in unit order: its combat units on
/// the map that are not heroes, summons (they last some turns) or dying.
pub fn champion_candidates(g: &Game, p: PlayerId) -> Vec<UnitId> {
    g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| {
            let unit = &g.units[u];
            g.unit_plot_opt(u).is_some() && unit.duration == 0 && !unit.death_delay && !g.unit_is_hero(u) && g.unit_can_fight(u)
        })
        .collect()
}

/// `targetsAnointedChampion(pCaster)`.
fn targets_anointed_champion(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    champion_candidates(g, g.units[caster].owner).into_iter().map(SpellTarget::Unit).collect()
}

/// `spellAnointedChampion(pCaster)`: the target becomes a hero and gains two levels (it leaves
/// its army: heroes move on their own, docs/ARMIES.md).
fn spell_anointed_champion(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Unit(u)) = g.spell_target else { return };
    if !g.units.contains(u) {
        return;
    }
    let hero = g.d.t::<PromotionType>("PROMOTION_HERO");
    g.unit_set_has_promotion(u, hero, true);
    crate::ffh::cards::world::unit_gain_levels(g, u, CHAMPION_LEVELS);
    let owner = g.units[u].owner;
    g.player_army_sync(owner);
    let name = g.spell_target_label(caster, g.d.t::<SpellType>("SPELL_ANOINTED_CHAMPION"), SpellTarget::Unit(u));
    let (x, y) = (g.units[u].x, g.units[u].y);
    let turn = g.game_turn();
    g.log.msg_at(turn, Some(owner), (x, y), || format!("Anointed Champion: {name} is a hero."));
}

// ---------------------------------------------------------------------------------------------
// B07 Legacy of Kings.

/// The traits Legacy of Kings offers: the selectable ones but Philosophical.
pub const LEGACY_TRAITS: [&str; 10] = [
    "TRAIT_AGGRESSIVE",
    "TRAIT_SPIRITUAL",
    "TRAIT_EXPANSIVE",
    "TRAIT_INDUSTRIOUS",
    "TRAIT_CREATIVE",
    "TRAIT_FINANCIAL",
    "TRAIT_ORGANIZED",
    "TRAIT_CHARISMATIC",
    "TRAIT_ARCANE",
    "TRAIT_RAIDERS",
];

/// The traits of [`LEGACY_TRAITS`] player `p`'s leader lacks, in that order.
pub fn legacy_traits(g: &Game, p: PlayerId) -> Vec<TraitType> {
    LEGACY_TRAITS.iter().map(|n| g.d.t::<TraitType>(n)).filter(|&t| !g.players[p].traits[t]).collect()
}

/// `targetsLegacyOfKings(pCaster)`: each trait the leader lacks, by its index
/// (`labelInfo(pCaster, iChoice, 'TraitType')` names it).
fn targets_legacy_of_kings(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    legacy_traits(g, g.units[caster].owner).into_iter().map(|t| SpellTarget::Choice(t.idx() as i32)).collect()
}

/// `spellLegacyOfKings(pCaster)`: the leader gains the chosen trait, for good.
fn spell_legacy_of_kings(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Choice(i)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    let Some(t) = legacy_traits(g, p).into_iter().find(|t| t.idx() as i32 == i) else { return };
    g.player_set_has_trait(p, t, true);
    g.players[p].card_traits.push(t);
    let name = g.d.text.name(&g.d.traits[t].base.description).to_string();
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Legacy of Kings: your leader is {name}."));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 7));
        g.game_init();
        g
    }

    /// Player 0 knows `card` (its needs too) and has a city; returns a unit of its to cast with.
    fn caster_with(g: &mut Game, card: &str) -> UnitId {
        let d = g.d;
        let p = PlayerId(0);
        let team = g.players[p].team;
        let card = d.t::<TechType>(card);
        for &t in d.techs[card].prereq_and_techs.iter() {
            g.team_set_has_tech(team, t, true, Some(p), false, false);
        }
        g.team_set_has_tech(team, card, true, Some(p), false, false);
        if g.players[p].cities.is_empty() {
            let settler = g.players[p].units.iter().copied().find(|&u| d.units[g.units[u].unit_type].found).expect("a settler");
            let (x, y) = (g.units[settler].x, g.units[settler].y);
            g.player_init_city(p, x, y, true, true);
        }
        let c = g.players[p].cities[0];
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None)
    }

    #[test]
    fn the_meteor_strikes_an_empty_plot_and_leaves_mithril() {
        let mut g = game();
        let d = g.d;
        let caster = caster_with(&mut g, "TECH_CARD_STARMETAL_METEOR");
        let spell = d.t::<SpellType>("SPELL_STARMETAL_METEOR");
        let p = PlayerId(0);
        let team = g.players[p].team;
        let targets = g.unit_spell_targets(caster, spell);
        assert!(!targets.is_empty(), "the land around a new city is a target");
        assert!(g.unit_can_cast(caster, spell, false));
        for &t in &targets {
            let SpellTarget::Plot(t) = t else { panic!("a plot target") };
            let plot = &g.map.plots[t];
            assert!(!g.plot_is_water(t) && plot.plot_city.is_none() && plot.improvement_type.is_none() && g.plot_is_visible(t, team));
        }
        // The city's plot is not a target; a plot with a unit of ours is not.
        let c = g.players[p].cities[0];
        assert!(!targets.contains(&SpellTarget::Plot(g.city_plot(c))));
        let SpellTarget::Plot(t) = targets[0] else { unreachable!() };
        let (x, y) = g.map.xy(t);
        let w = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        assert!(!g.unit_spell_targets(caster, spell).contains(&SpellTarget::Plot(t)), "a plot with a unit is not a target");
        assert!(!g.unit_can_cast_at(caster, spell, SpellTarget::Plot(t)));
        g.unit_kill(w, false, None);
        // A forest there: the meteor clears it and leaves Mithril.
        let forest = d.t::<FeatureType>("FEATURE_FOREST");
        g.plot_set_feature_type(t, Some(forest));
        assert!(g.unit_can_cast_at(caster, spell, SpellTarget::Plot(t)));
        g.unit_cast_at(caster, spell, SpellTarget::Plot(t));
        assert_eq!(g.map.plots[t].feature_type, None);
        assert_eq!(g.map.plots[t].bonus_type, Some(d.t::<BonusType>("BONUS_MITHRIL")));
        // Once a game.
        assert!(g.players[p].once_spells.contains(&spell));
        assert!(!g.unit_can_cast(caster, spell, false));
        assert!(!g.unit_can_cast_untargeted(caster, spell, true));
        // Once a game for the team: a teammate cannot cast it again (the card is the team's).
        let mate = g.player_ids().find(|&q| q != p && g.players[q].alive && !g.player_is_barbarian(q)).expect("another civilization");
        assert!(!g.player_cast_once(mate, spell));
        g.players[mate].team = g.players[p].team;
        assert!(g.player_cast_once(mate, spell));
    }

    #[test]
    fn the_meteor_reaches_six_plots_past_the_borders() {
        let mut g = game();
        let caster = caster_with(&mut g, "TECH_CARD_STARMETAL_METEOR");
        let p = g.units[caster].owner;
        let near = crate::ffh::cards::world::near_borders(&g, p, METEOR_RANGE);
        for t in meteor_plots(&g, p) {
            assert!(near[t.idx()]);
            let (x, y) = g.map.xy(t);
            let close = g.map.plot_ids().filter(|&o| g.map.plots[o].owner == Some(p)).any(|o| {
                let (ox, oy) = g.map.xy(o);
                g.step_distance(x, y, ox, oy) <= METEOR_RANGE
            });
            assert!(close, "({x}, {y}) is within {METEOR_RANGE} plots of the borders");
        }
    }

    #[test]
    fn the_champion_becomes_a_hero_with_two_levels() {
        let mut g = game();
        let d = g.d;
        let caster = caster_with(&mut g, "TECH_CARD_ANOINTED_CHAMPION");
        let spell = d.t::<SpellType>("SPELL_ANOINTED_CHAMPION");
        let p = PlayerId(0);
        let c = g.players[p].cities[0];
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let u = g.player_init_unit(p, d.t::<UnitType>("UNIT_AXEMAN"), x, y, None, None);
        g.units[u].level = 2;
        g.units[u].experience = 3;
        let target = SpellTarget::Unit(u);
        assert!(g.unit_spell_targets(caster, spell).contains(&target));
        assert!(g.unit_can_cast_at(caster, spell, target));
        g.unit_cast_at(caster, spell, target);
        assert!(g.unit_is_hero(u));
        // Level 2 with 3 XP could not promote (it needs 5); now it can twice: 10 XP, the threshold
        // of level 3.
        assert_eq!(g.units[u].experience, g.py_get_experience_needed(3, p));
        assert!(g.units[u].promotion_ready);
        // A hero is no target.
        assert!(!g.unit_spell_targets(caster, spell).contains(&target));
        assert!(!champion_candidates(&g, p).contains(&u));
    }

    #[test]
    fn levels_count_from_the_promotions_already_waiting() {
        let mut g = game();
        let d = g.d;
        let p = PlayerId(0);
        let (x, y) = (g.units[g.players[p].units[0]].x, g.units[g.players[p].units[0]].y);
        let u = g.player_init_unit(p, d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        g.units[u].level = 1;
        // 2 XP: one promotion waits (level 1 needs 2); two more levels need the threshold of level 3.
        g.units[u].experience = 2;
        crate::ffh::cards::world::unit_gain_levels(&mut g, u, 2);
        assert_eq!(g.units[u].experience, g.py_get_experience_needed(3, p));
    }

    #[test]
    fn a_hero_has_one_extra_battle_action_whatever_its_tier() {
        let d = ffh_data::global();
        let tier4 = d.units.keys().find(|&t| d.units[t].tier >= d.defines.engagement_extra_action_tier).expect("a tier-4 unit");
        assert_eq!(d.engage_extra_actions(tier4, true), d.engage_extra_actions(tier4, false), "the hero's extra action does not stack with tier 4's");
        let low = d.units.keys().find(|&t| d.units[t].tier > 0 && d.units[t].tier < d.defines.engagement_extra_action_tier).expect("a low-tier unit");
        assert_eq!(d.engage_extra_actions(low, true), d.engage_extra_actions(tier4, false));
    }

    #[test]
    fn the_legacy_trait_is_chosen_and_kept_through_a_trait_swap() {
        let mut g = game();
        let d = g.d;
        let caster = caster_with(&mut g, "TECH_CARD_LEGACY_OF_KINGS");
        let spell = d.t::<SpellType>("SPELL_LEGACY_OF_KINGS");
        let p = PlayerId(0);
        let targets = g.unit_spell_targets(caster, spell);
        let lacks = legacy_traits(&g, p);
        assert_eq!(targets.len(), lacks.len());
        let philosophical = d.t::<TraitType>("TRAIT_PHILOSOPHICAL");
        assert!(!targets.contains(&SpellTarget::Choice(philosophical.idx() as i32)), "Philosophical is not offered");
        let t = lacks[0];
        let choice = SpellTarget::Choice(t.idx() as i32);
        assert_eq!(g.spell_target_label(caster, spell, choice), d.text.name(&d.traits[t].base.description));
        assert!(g.unit_can_cast_at(caster, spell, choice));
        g.unit_cast_at(caster, spell, choice);
        assert!(g.players[p].traits[t]);
        assert_eq!(g.players[p].card_traits, vec![t]);
        assert!(!g.unit_spell_targets(caster, spell).contains(&choice));
        // An Adaptive swap (FFH2's doTraitX) drops the selectable traits but keeps the card's.
        let other = d.t::<TraitType>(LEGACY_TRAITS.iter().copied().find(|n| d.t::<TraitType>(n) != t && !g.players[p].traits[d.t::<TraitType>(n)]).unwrap());
        let trigger = d.t::<EventTriggerType>("EVENTTRIGGER_TRAIT_ADAPTIVE");
        let data = crate::structs::EventTriggeredData {
            id: TriggeredId(0),
            trigger,
            turn: g.game_turn(),
            player: p,
            city: None,
            plot_x: -1,
            plot_y: -1,
            unit: None,
            other_player: None,
            other_player_city: None,
            religion: None,
            corporation: None,
            building: None,
            text: String::new(),
            global_text: String::new(),
        };
        crate::ffh::random_events_a::do_trait(&mut g, &data, other);
        assert!(g.players[p].traits[other]);
        assert!(g.players[p].traits[t], "the card's trait survives the swap");
    }

    #[test]
    fn an_unknown_card_spell_is_not_offered() {
        let g = game();
        let d = g.d;
        let p = PlayerId(0);
        let u = g.players[p].units[0];
        for s in ["SPELL_STARMETAL_METEOR", "SPELL_ANOINTED_CHAMPION", "SPELL_LEGACY_OF_KINGS"] {
            let s = d.t::<SpellType>(s);
            assert!(d.spells[s].once && d.spells[s].target.is_some());
            assert!(!g.unit_can_cast_untargeted(u, s, true), "{}: needs its card", d.spells[s].base.type_);
        }
    }
}
