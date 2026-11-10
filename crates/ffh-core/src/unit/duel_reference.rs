//! A frozen copy of `unit_max_combat_str` as it was before it was split into
//! `unit_combat_terms` (engagement combat, phase 6 step 4), and a test that the duel mode returns
//! exactly what it returned, details included, for many units, opponents and plots in every call
//! configuration. A guard for that refactor only: when the duel changes on purpose, delete this
//! file.

use super::combat::apply_combat_modifier;
use super::CombatDetails;
use crate::game::Game;
use crate::ids::*;
use crate::test_world::world;

fn old_max_combat_str(
    g: &Game,
    u: UnitId,
    plot: Option<PlotIdx>,
    attacker: Option<UnitId>,
    details: Option<&mut CombatDetails>,
) -> i32 {
    let d = g.d;
    let mut details = details;

    // C++ `if (pCombatDetails != NULL) pCombatDetails->iX = value;`
    macro_rules! set_detail {
        ($field:ident, $value:expr) => {
            if let Some(det) = details.as_deref_mut() {
                det.$field = $value;
            }
        };
    }

    let mut plot = plot;
    let mut attacker = attacker;

    // FfH Damage Types: Added by Kael 09/02/2007 (without a plot, the "attacker" argument
    // is the defender this unit attacks)
    let mut defender: Option<UnitId> = None;
    if plot.is_none() && attacker.is_some() {
        defender = attacker;
        attacker = None;
    }
    // FfH: End Add

    // handle our new special case
    let mut attacked_plot: Option<PlotIdx> = None;
    let mut attacking_unknown_defender = false;
    if attacker == Some(u) {
        attacking_unknown_defender = true;
        attacked_plot = plot;

        // reset these values, we will fiddle with them below
        plot = None;
        attacker = None;
    } else if attacker.is_some() {
        // otherwise, attack plot is the plot of us (the defender)
        attacked_plot = g.unit_plot_opt(u);
    }

    if let Some(det) = details.as_deref_mut() {
        *det = CombatDetails {
            owner: Some(g.units[u].owner),
            visual_owner: Some(g.unit_get_visual_owner(u, None)),
            unit_name: g.unit_get_name(u),
            ..CombatDetails::default()
        };
    }

    // FfH Defense Str: Modified by Kael 08/18/2007 (attacks use the attack strength,
    // everything else the defense strength)
    // C++ quirk: `pAttacker == this` cannot hold any more here (it was reset above).
    let mut str_ = if (attacker.is_none() && plot.is_none()) || attacker == Some(u) {
        g.unit_base_combat_str(u)
    } else {
        g.unit_base_combat_str_defense(u)
    };

    if str_ == 0 {
        return 0;
    }
    // FfH: End Modify

    let team = g.unit_get_team(u);
    let mut modifier = 0;

    let mut extra_modifier = g.unit_get_extra_combat_percent(u);
    modifier += extra_modifier;
    set_detail!(extra_combat_percent, extra_modifier);

    // do modifiers for animals and barbarians (leaving these out for bAttackingUnknownDefender case)
    if let Some(a) = attacker {
        let handicap = &d.handicaps[g.handicap];

        if g.unit_is_animal(u) {
            if g.unit_is_human(a) {
                extra_modifier = handicap.animal_combat_modifier;
                modifier += extra_modifier;
                set_detail!(animal_combat_modifier_ta, extra_modifier);
            } else {
                extra_modifier = handicap.ai_animal_combat_modifier;
                modifier += extra_modifier;
                set_detail!(ai_animal_combat_modifier_ta, extra_modifier);
            }
        }

        if g.unit_is_animal(a) {
            if g.unit_is_human(u) {
                extra_modifier = -handicap.animal_combat_modifier;
                modifier += extra_modifier;
                set_detail!(animal_combat_modifier_aa, extra_modifier);
            } else {
                extra_modifier = -handicap.ai_animal_combat_modifier;
                modifier += extra_modifier;
                set_detail!(ai_animal_combat_modifier_aa, extra_modifier);
            }
        }

        if g.unit_is_barbarian(u) {
            if g.unit_is_human(a) {
                extra_modifier = handicap.barbarian_combat_modifier;
                modifier += extra_modifier;
                set_detail!(barbarian_combat_modifier_tb, extra_modifier);
            } else {
                extra_modifier = handicap.ai_barbarian_combat_modifier;
                modifier += extra_modifier;
                set_detail!(ai_barbarian_combat_modifier_tb, extra_modifier);
            }
        }

        if g.unit_is_barbarian(a) {
            if g.unit_is_human(u) {
                extra_modifier = -handicap.barbarian_combat_modifier;
                modifier += extra_modifier;
                set_detail!(barbarian_combat_modifier_ab, extra_modifier);
            } else {
                extra_modifier = -handicap.ai_barbarian_combat_modifier;
                modifier += extra_modifier;
                // FfH: Modified by Kael 07/31/2008 (was stored in the TB field)
                set_detail!(ai_barbarian_combat_modifier_ab, extra_modifier);
            }
        }
    }

    // add defensive bonuses (leaving these out for bAttackingUnknownDefender case)
    if let Some(p) = plot {
        if !g.unit_no_defensive_bonus(u) {
            let ignore_building = match attacker {
                Some(a) => g.unit_ignore_building_defense(a),
                None => true,
            };
            extra_modifier = old_plot_defense_modifier(g, p, Some(team), ignore_building);
            modifier += extra_modifier;
            set_detail!(plot_defense_modifier, extra_modifier);
        }

        extra_modifier = g.unit_fortify_modifier(u);
        modifier += extra_modifier;
        set_detail!(fortify_modifier, extra_modifier);

        if g.plot_is_city(p, true, Some(team)) {
            extra_modifier = g.unit_city_defense_modifier(u);
            modifier += extra_modifier;
            set_detail!(city_defense_modifier, extra_modifier);
        }

        if g.plot_is_hills(p) {
            extra_modifier = g.unit_hills_defense_modifier(u);
            modifier += extra_modifier;
            set_detail!(hills_defense_modifier, extra_modifier);
        }

        if let Some(feature) = g.map.plots[p].feature_type {
            extra_modifier = g.unit_feature_defense_modifier(u, feature);
            modifier += extra_modifier;
            set_detail!(feature_defense_modifier, extra_modifier);
        } else {
            let terrain = g.map.plots[p].terrain_type.expect("CvUnit::maxCombatStr: plot without terrain");
            extra_modifier = g.unit_terrain_defense_modifier(u, terrain);
            modifier += extra_modifier;
            set_detail!(terrain_defense_modifier, extra_modifier);
        }
    }

    // if we are attacking to an plot with an unknown defender, the calc the modifier in reverse
    if attacking_unknown_defender {
        attacker = Some(u);
    }

    // calc attacker bonueses
    // UNOFFICIAL_PATCH 09/20/09 jdog5000: also requires the attacked plot.
    if let (Some(a), Some(ap)) = (attacker, attacked_plot) {
        let mut temp_modifier = 0;

        if g.plot_is_city(ap, true, Some(team)) {
            extra_modifier = -g.unit_city_attack_modifier(a);
            temp_modifier += extra_modifier;
            set_detail!(city_attack_modifier, extra_modifier);

            if g.unit_is_barbarian(a) {
                extra_modifier = d.defines.city_barbarian_defense_modifier;
                temp_modifier += extra_modifier;
                set_detail!(city_barbarian_defense_modifier, extra_modifier);
            }
        }

        if g.plot_is_hills(ap) {
            extra_modifier = -g.unit_hills_attack_modifier(a);
            temp_modifier += extra_modifier;
            set_detail!(hills_attack_modifier, extra_modifier);
        }

        if let Some(feature) = g.map.plots[ap].feature_type {
            extra_modifier = -g.unit_feature_attack_modifier(a, feature);
            temp_modifier += extra_modifier;
            set_detail!(feature_attack_modifier, extra_modifier);
        } else {
            let terrain = g.map.plots[ap].terrain_type.expect("CvUnit::maxCombatStr: plot without terrain");
            extra_modifier = -g.unit_terrain_attack_modifier(a, terrain);
            // C++ quirk: added to iModifier, not iTempModifier (never reversed for an unknown
            // defender).
            modifier += extra_modifier;
            set_detail!(terrain_attack_modifier, extra_modifier);
        }

        // only compute comparisions if we are the defender with a known attacker
        if !attacking_unknown_defender {
            // FfH Promotions: Added by Kael 08/13/2007
            // (C++ loops over every promotion; only those with `promotion_combat_mod > 0`
            // change the modifier, and `d.combat_mod_promotions` lists exactly those, in
            // the same order.)
            let our_promotions = &g.units[u].has_promotion;
            let their_promotions = &g.units[a].has_promotion;
            for &promotion in &d.combat_mod_promotions {
                let info = &d.promotions[promotion];
                // (C++ reads `isHasPromotion(NO_PROMOTION)` out of bounds for a missing
                // combat type; every promotion with a combat mod names one)
                if our_promotions[promotion] && info.promotion_combat_mod > 0 {
                    if info.promotion_combat_type.is_some_and(|t| their_promotions[t]) {
                        modifier += info.promotion_combat_mod;
                    }
                }
                if their_promotions[promotion] && info.promotion_combat_mod > 0 {
                    if info.promotion_combat_type.is_some_and(|t| our_promotions[t]) {
                        modifier -= info.promotion_combat_mod;
                    }
                }
            }
            let global_counter = g.get_global_counter();
            if global_counter * g.units[u].combat_percent_global_counter / 100 != 0 {
                modifier += global_counter * g.units[u].combat_percent_global_counter / 100;
            }
            if global_counter * g.units[a].combat_percent_global_counter / 100 != 0 {
                modifier -= global_counter * g.units[a].combat_percent_global_counter / 100;
            }
            // FfH: End Add

            extra_modifier = g.unit_unit_class_defense_modifier(u, g.unit_get_unit_class_type(a));
            temp_modifier += extra_modifier;
            set_detail!(class_defense_modifier, extra_modifier);

            extra_modifier = -g.unit_unit_class_attack_modifier(a, g.unit_get_unit_class_type(u));
            temp_modifier += extra_modifier;
            set_detail!(class_attack_modifier, extra_modifier);

            if let Some(unit_combat) = g.unit_get_unit_combat_type(a) {
                extra_modifier = g.unit_unit_combat_modifier(u, unit_combat);
                temp_modifier += extra_modifier;
                set_detail!(combat_modifier_a, extra_modifier);
            }
            if let Some(unit_combat) = g.unit_get_unit_combat_type(u) {
                extra_modifier = -g.unit_unit_combat_modifier(a, unit_combat);
                temp_modifier += extra_modifier;
                set_detail!(combat_modifier_t, extra_modifier);
            }

            extra_modifier = g.unit_domain_modifier(u, g.unit_get_domain_type(a));
            temp_modifier += extra_modifier;
            set_detail!(domain_modifier_a, extra_modifier);

            extra_modifier = -g.unit_domain_modifier(a, g.unit_get_domain_type(u));
            temp_modifier += extra_modifier;
            set_detail!(domain_modifier_t, extra_modifier);

            if g.unit_is_animal(a) {
                extra_modifier = g.unit_animal_combat_modifier(u);
                temp_modifier += extra_modifier;
                set_detail!(animal_combat_modifier_a, extra_modifier);
            }

            if g.unit_is_animal(u) {
                extra_modifier = -g.unit_animal_combat_modifier(a);
                temp_modifier += extra_modifier;
                set_detail!(animal_combat_modifier_t, extra_modifier);
            }
        }

        let attacker_plot = g.unit_plot(a);
        if !g.unit_is_river(a) {
            if g.plot_is_river_crossing(attacker_plot, g.direction_xy_plots(attacker_plot, ap)) {
                extra_modifier = -d.defines.river_attack_modifier;
                temp_modifier += extra_modifier;
                set_detail!(river_attack_modifier, extra_modifier);
            }
        }

        if !g.unit_is_amphib(a) {
            if !g.plot_is_water(ap) && g.plot_is_water(attacker_plot) {
                extra_modifier = -d.defines.amphib_attack_modifier;
                temp_modifier += extra_modifier;
                set_detail!(amphib_attack_modifier, extra_modifier);
            }
        }

        if g.units[a].kamikaze_percent != 0 {
            extra_modifier = g.units[a].kamikaze_percent;
            temp_modifier += extra_modifier;
            set_detail!(kamikaze_modifier, extra_modifier);
        }

        // if we are attacking an unknown defender, then use the reverse of the modifier
        if attacking_unknown_defender {
            modifier -= temp_modifier;
        } else {
            modifier += temp_modifier;
        }
    }

    // FfH Defense Str: Modified by Kael 08/18/2007 (strength in hundredths, damage types
    // reduced by the opponent's resistances)
    set_detail!(modifier_total, modifier);
    set_detail!(base_combat_str, str_);

    str_ *= 100;
    // C++ quirk: for an unknown defender `pAttacker` is this unit again, so its own
    // resistances reduce its damage-type strength.
    let our_damage_type_combat = &g.units[u].damage_type_combat;
    for opponent in [attacker, defender].into_iter().flatten() {
        for damage_type in d.damages.keys() {
            let damage_type_combat = our_damage_type_combat[damage_type];
            if damage_type_combat != 0 {
                let resist = g.unit_get_damage_type_resist(opponent, damage_type);
                if resist != 0 {
                    str_ -= damage_type_combat * 100;
                    str_ += damage_type_combat * (100 - resist);
                }
            }
        }
    }

    let combat = apply_combat_modifier(str_, modifier);
    // FfH: End Modify

    if let Some(det) = details.as_deref_mut() {
        det.combat = combat;
        det.max_combat_str = 1.max(combat);
        det.curr_hit_points = g.unit_curr_hit_points(u);
        det.max_hit_points = g.unit_max_hit_points(u);
        det.curr_combat_str = (det.max_combat_str * det.curr_hit_points) / det.max_hit_points;
    }

    1.max(combat)
}

/// `plot_defense_modifier` as it was before it was split into `plot_defense`'s parts.
fn old_plot_defense_modifier(g: &Game, p: PlotIdx, defender: Option<TeamId>, ignore_building: bool) -> i32 {
    let d = g.d;
    let plot = &g.map.plots[p];

    let mut modifier = match plot.feature_type {
        None => d.terrains[plot.terrain_type.expect("terrain")].defense_modifier,
        Some(f) => d.features[f].defense_modifier,
    };

    if g.plot_is_hills(p) {
        modifier += d.defines.hills_extra_defense;
    }

    if let Some(improvement) = plot.improvement_type {
        if let Some(defender) = defender {
            let plot_team = g.plot_get_team(p);
            if plot_team.is_none() || g.team_is_friendly_territory(defender, plot_team) {
                modifier += d.improvements[improvement].defense_modifier;
            }
        }
    }

    if let Some(city) = plot.plot_city {
        modifier += g.city_defense_modifier(city, ignore_building);
    } else if let Some(defender) = defender {
        let plot_team = g.plot_get_team(p);
        if plot_team.is_none() || g.team_is_friendly_territory(defender, plot_team) {
            modifier += g.plot_get_range_defense(p, defender, 3, false, true);
        }
    }

    modifier
}

#[test]
fn the_duel_mode_returns_what_max_combat_str_returned() {
    let (g, units, plots) = world();
    let (mut old_det, mut new_det) = (CombatDetails::default(), CombatDetails::default());
    let mut checked = 0;
    let mut with_terms = 0;
    let mut same = |u: UnitId, plot: Option<PlotIdx>, opponent: Option<UnitId>| {
        // Not compared: a land unit in boats on water (strength 1, docs/NAVAL.md: a rules change
        // after this reference).
        if plot.or(g.unit_plot_opt(u)).is_some_and(|p| g.unit_in_boats_at(u, p)) {
            return;
        }
        let old = old_max_combat_str(&g, u, plot, opponent, Some(&mut old_det));
        let new = g.unit_max_combat_str(u, plot, opponent, Some(&mut new_det));
        assert_eq!((old, &old_det), (new, &new_det), "unit {u:?} plot {plot:?} opponent {opponent:?}");
        assert_eq!(old, g.unit_max_combat_str(u, plot, opponent, None));
        checked += 1;
        if new_det.modifier_total != 0 {
            with_terms += 1;
        }
    };
    for (i, &u) in units.iter().enumerate() {
        let own = g.unit_plot(u);
        same(u, None, None);
        for k in 0..10 {
            let p = plots[(i * 13 + k * 7) % plots.len()];
            same(u, Some(p), None);
            same(u, Some(p), Some(u));
        }
        for &v in &units {
            if v == u {
                continue;
            }
            same(u, None, Some(v));
            same(u, Some(own), Some(v));
            same(u, Some(g.unit_plot(v)), Some(v));
        }
    }
    // The world exercises the terms.
    assert!(checked > 30_000 && with_terms > checked / 2, "{checked} checked, {with_terms} with a modifier");
}

#[test]
fn plot_defense_parts_sum_to_the_old_modifier() {
    let (g, _, _) = world();
    for p in g.map.plot_ids() {
        for team in g.team_ids().map(Some).chain([None]) {
            for ignore_building in [false, true] {
                assert_eq!(g.plot_defense_modifier(p, team, ignore_building), old_plot_defense_modifier(&g, p, team, ignore_building));
            }
        }
    }
}
