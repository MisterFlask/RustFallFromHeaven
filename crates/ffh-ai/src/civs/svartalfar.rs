//! Svartalfar. Their `MaintainFeatures` keeps the forest under every improvement they build on
//! their own land (`CvPlot::setImprovementType` with FfH's check), and workers only build there,
//! so they never clear forest. Veil of Night stays with the generic play (cast the first turn the
//! AI-only gate opens): never casting it measured worse (win 20% -> 10%, docs/AI_LOG.md). They keep
//! casters so their Illusionists (Mages upgraded from level-4 Adepts) come:
//! [`super::unique_casters`], the owner's rule that civilizations use their unique units.
//!
//! **Kidnap** (FfH kept it from its AI, `bAllowAI` 0, `unflagged.rs`): a recon unit inside a rival
//! city takes one of the great people settled there (success: its strength x 8 %, at most 95);
//! failing, it dies or is sent home and war breaks out with the city's owner. An expert risks it
//! only when that war is coming anyway: [`kidnap`] sends a lone recon unit of
//! strength 4 or more (a Hunter's 32% and up) into a city of the team we prepare a war against
//! that has great people settled, once the war is near (our army two thirds of what the
//! declaration waits for, `diplomacy::army_needed_for_war`, or 25 turns of preparation), and casts
//! there; [`kidnapper`] trains the strongest recon unit for it when we have none.

use ffh_core::ffh::spells_a::KIDNAP_TARGETS;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use super::CivPlay;
use crate::production::Choice;
use crate::units::{alive, hold, move_to};
use crate::view::View;
use crate::Ai;

pub struct Svartalfar;

impl CivPlay for Svartalfar {
    /// FfH's `AI_chooseTech` research plan (`tech_plan::goals`).
    fn tech_goals(&self, _g: &Game, _p: PlayerId) -> super::TechGoals {
        super::TechGoals { early_military: Some("TECH_HUNTING"), religion: Some("TECH_WAY_OF_THE_FORESTS"), beelines: vec!["TECH_POISONS", "TECH_ANIMAL_HANDLING"] }
    }

    fn production(&self, _ai: &Ai, g: &Game, c: CityId, threat: i32) -> Option<Choice> {
        super::unique_casters(g, c, threat).or_else(|| kidnapper(g, c, threat))
    }

    fn unit(&self, _ai: &Ai, g: &mut Game, v: &mut View, grp: GroupId, head: UnitId) -> bool {
        kidnap(g, v, grp, head)
    }
}

/// The great people settled in city `c` that a Kidnap could take.
fn settled_great_people(g: &Game, c: CityId) -> i32 {
    KIDNAP_TARGETS.iter().filter_map(|&(s, _)| g.d.ty::<SpecialistType>(s)).map(|s| g.cities[c].free_specialist_count[s].max(0)).sum()
}

/// Kidnappers need this much base strength (x 8 = the success chance in percent).
const MIN_STRENGTH: i32 = 4;

/// The team whose war is near enough for a Kidnap (see the module doc), if it has great people
/// settled in a city.
fn kidnap_target(g: &Game, p: PlayerId) -> Option<TeamId> {
    let team = g.players[p].team;
    let target = crate::diplomacy::preparing_target(g, team)?;
    let near = crate::diplomacy::army_size(g, p) * 3 >= crate::diplomacy::army_needed_for_war(g, p, target) * 2 || g.teams[team].ai_war_plan_state_counter[target] >= 25;
    let any = g.player_ids().any(|o| g.players[o].alive && g.players[o].team == target && g.players[o].cities.iter().any(|&c| settled_great_people(g, c) > 0));
    (near && any).then_some(target)
}

/// The strongest recon unit we can train, for a Kidnap, when the war is near, a target city has
/// great people and no recon unit of ours is strong enough; from our most productive city.
fn kidnapper(g: &Game, c: CityId, threat: i32) -> Option<Choice> {
    let d = g.d;
    let p = g.cities[c].owner;
    if threat > 0 || kidnap_target(g, p).is_none() {
        return None;
    }
    let recon = d.ty::<UnitCombatType>("UNITCOMBAT_RECON")?;
    if g.players[p].units.iter().any(|&u| g.unit_get_unit_combat_type(u) == Some(recon) && g.unit_base_combat_str(u) >= MIN_STRENGTH) {
        return None;
    }
    let best = g.players[p].cities.iter().copied().max_by_key(|&o| (g.city_yield_rate(o, YieldType::YIELD_PRODUCTION), std::cmp::Reverse(o)));
    if best != Some(c) {
        return None;
    }
    let u = d.units.keys().filter(|&u| d.units[u].unit_combat_type == Some(recon) && d.units[u].combat >= MIN_STRENGTH && g.city_can_train(c, u, false, false, false, false)).max_by_key(|&u| (d.units[u].combat, std::cmp::Reverse(u)))?;
    crate::trace!(g, p, "{}: {} to kidnap", g.cities[c].name, d.units[u].base.type_);
    Some(Choice::Train(u, UnitAIType::UNITAI_EXPLORE))
}

/// Kidnap as the opening of a prepared war (see the module doc). True if the unit got orders.
fn kidnap(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let d = g.d;
    let p = v.p;
    let Some(spell) = d.ty::<SpellType>("SPELL_KIDNAP") else { return false };
    let recon = d.ty::<UnitCombatType>("UNITCOMBAT_RECON");
    if recon.is_none() || g.unit_get_unit_combat_type(u) != recon || g.groups[grp].units.len() != 1 || g.unit_base_combat_str(u) < MIN_STRENGTH {
        return false;
    }
    let Some(target) = kidnap_target(g, p) else { return false };
    let target_city = |g: &Game, q: PlotIdx| g.map.plots[q].plot_city.is_some_and(|c| g.players[g.cities[c].owner].team == target && settled_great_people(g, c) > 0);
    let here = g.unit_plot(u);
    if target_city(g, here) && g.unit_can_cast(u, spell, false) {
        crate::trace!(g, p, "kidnap in {} (team {:?}, the war's opening)", g.cities[g.map.plots[here].plot_city.unwrap()].name, target);
        g.unit_cast(u, spell);
        return true;
    }
    let reach = v.flood.run(g, u, 10, false);
    let Some(r) = reach.iter().find(|r| g.map.plots[r.plot].revealed[v.team] && target_city(g, r.plot)).copied() else { return false };
    crate::trace!(g, p, "kidnapper at {:?}: to {:?} ({} turns)", g.map.xy(here), g.map.xy(r.plot), r.turns);
    move_to(g, grp, r.plot, 0);
    if alive(g, grp) && g.units.contains(u) && g.unit_plot(u) == r.plot && g.unit_can_cast(u, spell, false) {
        crate::trace!(g, p, "kidnap on arrival (team {:?})", target);
        g.unit_cast(u, spell);
    }
    if alive(g, grp) && g.units.contains(u) && g.group_ready_to_move(grp, true) {
        hold(g, grp);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game_with;

    /// A Shadow standing in a city of the team we prepare a war against, with a great scientist
    /// settled there, kidnaps: the scientist is ours, or the attempt failed and the war is on.
    #[test]
    fn a_shadow_kidnaps_as_the_war_opens() {
        let d = ffh_data::global();
        let mut g = one_landmass_game_with(5, &["CIVILIZATION_SVARTALFAR", "CIVILIZATION_MALAKIM"]);
        let (us, them) = (PlayerId(0), PlayerId(1));
        g.player_found(us, 4, 5);
        g.player_found(them, 12, 5);
        let theirs = g.players[them].cities[0];
        g.city_change_free_specialist_count(theirs, d.t::<SpecialistType>("SPECIALIST_GREAT_SCIENTIST"), 1);
        let (t0, t1) = (g.players[us].team, g.players[them].team);
        g.team_ai_set_war_plan(t0, t1, Some(WarPlanType::WARPLAN_PREPARING_TOTAL), true);
        g.teams[t0].ai_war_plan_state_counter[t1] = 30;
        let shadow = g.player_init_unit(us, d.t::<UnitType>("UNIT_SHADOW"), 12, 5, Some(UnitAIType::UNITAI_EXPLORE), None);
        assert!(!g.at_war(t0, t1));
        let mut v = View::new(&g, us);
        let grp = g.units[shadow].group.unwrap();
        assert!(Svartalfar.unit(&Ai::default(), &mut g, &mut v, grp, shadow), "the shadow got its orders");
        let scientist = d.t::<UnitType>("UNIT_SCIENTIST");
        let taken = g.players[us].units.iter().any(|&u| g.units[u].unit_type == scientist);
        assert!(taken || g.at_war(t0, t1), "the kidnap was cast");
        assert_eq!(taken, g.cities[theirs].free_specialist_count[d.t::<SpecialistType>("SPECIALIST_GREAT_SCIENTIST")] == 0);
    }
}
