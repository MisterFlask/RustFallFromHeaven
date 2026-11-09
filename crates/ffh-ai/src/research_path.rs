//! Research paths (docs/AI.md, "Research paths"; the rule: docs/PORTING.md, "Research paths").
//!
//! The commitment is for the rest of the game. An expert still commits at once: every turn
//! uncommitted is research given away, and the path a civilization is built for -- the one its own
//! units, spells and heroes sit on (`CIV_PATHS`) -- is its path whatever the map shows. Without one,
//! it takes the path whose techs its leader cares for most, weighted by the research the
//! commitment saves and by its access to the resources the path's late units need
//! (`resource_pct`: Knights without horses are no plan). The path's late techs then join the
//! research plan (`tech_plan`, step `ResearchPath`), and the research score counts their halved
//! research time (`tech_plan::effective_left`).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// Each civilization's paths, the one it is built for first: its unique units, heroes and
/// mechanics (`CIVILIZATION_*`, `RESEARCHPATH_*`).
const CIV_PATHS: &[(&str, &[&str])] = &[
    // Wizards, Archmages, the Arcane trait's leaders.
    ("CIVILIZATION_AMURITES", &["RESEARCHPATH_ARCANE"]),
    // Harlequins, Mimics and Perpentach's arcane bent; FfH beelines Iron Working.
    ("CIVILIZATION_BALSERAPHS", &["RESEARCHPATH_ARCANE", "RESEARCHPATH_STEEL"]),
    // Crusaders, Paladins, Flagbearers: the crusading faith (FfH beelines Fanaticism).
    ("CIVILIZATION_BANNOR", &["RESEARCHPATH_FAITH", "RESEARCHPATH_STEEL"]),
    // Vampire Lords (Divine Essence), Brujah (Rage).
    ("CIVILIZATION_CALABIM", &["RESEARCHPATH_FAITH", "RESEARCHPATH_STEEL"]),
    // Ogres and lizardmen: Stoneskin Ogres, Ogre Warchiefs, Lizardman Beastmasters.
    ("CIVILIZATION_CLAN_OF_EMBERS", &["RESEARCHPATH_STEEL", "RESEARCHPATH_WILD"]),
    // Circle of Urd (Rage), Verdandi (Mithril Weapons), Bison Riders; the wild north.
    ("CIVILIZATION_DOVIELLO", &["RESEARCHPATH_STEEL", "RESEARCHPATH_WILD"]),
    // The angels' faith (FfH beelines Priesthood).
    ("CIVILIZATION_ELOHIM", &["RESEARCHPATH_FAITH"]),
    // The Luonnotar (Strength of Will) and Medics; no gods.
    ("CIVILIZATION_GRIGORI", &["RESEARCHPATH_ARCANE", "RESEARCHPATH_MERCANTILE"]),
    // Horse lords (FfH beelines Stirrups).
    ("CIVILIZATION_HIPPUS", &["RESEARCHPATH_CAVALRY"]),
    // The High Priest of Winter (Theology).
    ("CIVILIZATION_ILLIANS", &["RESEARCHPATH_FAITH", "RESEARCHPATH_ARCANE"]),
    // Dwarven Cannons, Mithril; FfH beelines Iron Working.
    ("CIVILIZATION_KHAZAD", &["RESEARCHPATH_STEEL", "RESEARCHPATH_ARTIFICE"]),
    // Centaur Lancers (FfH beelines Stirrups).
    ("CIVILIZATION_KURIOTATES", &["RESEARCHPATH_CAVALRY"]),
    // Sea traders and pirates; Men-o'-War (Blasting Powder).
    ("CIVILIZATION_LANUN", &["RESEARCHPATH_MERCANTILE", "RESEARCHPATH_ARTIFICE"]),
    // Elven archers and the forest: the Flurry (Machinery), Druids.
    ("CIVILIZATION_LJOSALFAR", &["RESEARCHPATH_WILD", "RESEARCHPATH_ARTIFICE"]),
    // Golems: Clockwork (Machinery), Nullstone (Mithril Weapons).
    ("CIVILIZATION_LUCHUIRP", &["RESEARCHPATH_ARTIFICE", "RESEARCHPATH_STEEL"]),
    // The faith of the desert (FfH beelines Priesthood).
    ("CIVILIZATION_MALAKIM", &["RESEARCHPATH_FAITH"]),
    // The Eater of Dreams (Strength of Will); Eidolons (evil Faith).
    ("CIVILIZATION_SHEAIM", &["RESEARCHPATH_ARCANE", "RESEARCHPATH_FAITH"]),
    // Spirits and shades (FfH beelines Poisons, which is on no path).
    ("CIVILIZATION_SIDAR", &["RESEARCHPATH_ARCANE"]),
    // Illusionists (Sorcery) and the forest.
    ("CIVILIZATION_SVARTALFAR", &["RESEARCHPATH_ARCANE", "RESEARCHPATH_WILD"]),
];

/// The weight of a civilization's first and second path over a path it is not built for.
const FIRST_PATH_PCT: i64 = 400;
const SECOND_PATH_PCT: i64 = 200;

/// How a path weighs (percent) by the resources its late units need (`resource_pct`): we have
/// one; one lies revealed in our land or within `RESOURCE_REACH` plots of a city of ours, unowned;
/// we cannot see that resource yet (its reveal tech unknown); we see it, and none lies in reach.
/// An input, not a veto: a civilization's own path (x4) still beats a generic one without its
/// resource -- the Amurites stay Arcane without Reagents for their Archmages, as their Wizards
/// and Mages need none (at 25% for none they left Arcane in 7 games of 8).
const RESOURCE_HAVE_PCT: i64 = 100;
const RESOURCE_IN_REACH_PCT: i64 = 90;
const RESOURCE_UNSEEN_PCT: i64 = 80;
const RESOURCE_NONE_PCT: i64 = 60;
const RESOURCE_REACH: i32 = 3;

/// Commits `p` to a research path when it has none (`switches::RESEARCH_PATH`).
pub fn turn(g: &mut Game, p: PlayerId) {
    if g.players[p].research_path.is_some() || g.players[p].cities.is_empty() || !crate::switches::RESEARCH_PATH.on(g, p) {
        return;
    }
    let scored = scores(g, p);
    let Some(&(path, score)) = scored.first() else {
        return;
    };
    if g.player_commit_research_path(p, path) {
        crate::trace!(g, p, "research path {} (score {})", g.d.research_paths[path].base.type_, score);
        if g.telemetry.on() {
            let alts = || ffh_core::telemetry::tel_alts(scored.iter().take(4).map(|&(q, s)| (g.d.research_paths[q].base.type_.as_str(), s)));
            g.tel_decision(p, "research_path", &g.d.research_paths[path].base.type_, "value", score, alts);
        }
    }
}

/// The paths `p` may commit to, best first, with their scores: the research the commitment
/// saves on the techs left (half of it, at `RESEARCH_PATH_PERCENT` 100), times the leader's
/// interest in those techs (its flavors against theirs), times the civilization's own paths.
pub fn scores(g: &Game, p: PlayerId) -> Vec<(ResearchPathType, i64)> {
    let d = g.d;
    let pl = &g.players[p];
    let team = pl.team;
    let leader = pl.personality_type.map(|l| &d.leader_heads[l]);
    let own: &[&str] = if crate::civs::enabled(g, p) {
        g.player_get_civilization_type(p)
            .and_then(|c| CIV_PATHS.iter().find(|(name, _)| d.civilizations[c].base.type_ == *name))
            .map_or(&[], |(_, paths)| *paths)
    } else {
        &[]
    };
    let mut out = Vec::new();
    for path in d.research_paths.keys() {
        if g.player_research_path_refusal(p, path).is_some() {
            continue;
        }
        let mut saved: i64 = 0;
        let mut interest: i64 = 0;
        for t in g.player_research_path_techs(p, path) {
            if g.team_is_has_tech(team, Some(t)) || !g.player_can_ever_research(p, t) {
                continue;
            }
            let left = g.team_get_research_left(team, t).max(0) as i64;
            saved += left * d.defines.research_path_percent as i64 / (100 + d.defines.research_path_percent as i64);
            // Leader flavors run 0-10, tech flavors 0-100: a perfect match of one flavor is 1000.
            let fit: i64 = leader.map_or(0, |l| d.techs[t].flavor_value.iter().zip(l.flavor_value.iter()).map(|(&a, &b)| a as i64 * b as i64).sum());
            interest += left * fit.min(1000);
        }
        if saved == 0 {
            continue;
        }
        // Interest as a percent of the research left: 0 (no matching flavor) to 100.
        let total_left = saved * (100 + d.defines.research_path_percent as i64) / d.defines.research_path_percent.max(1) as i64;
        let interest_pct = interest / 10 / total_left.max(1);
        let name = d.research_paths[path].base.type_.as_str();
        let own_pct = match own.iter().position(|&n| n == name) {
            Some(0) => FIRST_PATH_PCT,
            Some(_) => SECOND_PATH_PCT,
            None => 100,
        };
        let resource = if crate::switches::PATH_RESOURCES.on(g, p) { resource_pct(g, p, path) } else { 100 };
        let score = saved * (100 + interest_pct) / 100 * own_pct / 100 * resource / 100;
        out.push((path, score));
    }
    out.sort_by_key(|&(path, score)| (std::cmp::Reverse(score), path));
    out
}

/// How well `p` can field the late units of `path` for want of resources (percent, the best of
/// its units; 100 when one needs none): an input of the choice, as a player sees it -- what it
/// has, what it can see in reach (`RESOURCE_*_PCT`).
pub fn resource_pct(g: &Game, p: PlayerId, path: ResearchPathType) -> i64 {
    let Some(civ) = g.player_get_civilization_type(p) else {
        return 100;
    };
    let units = g.research_path_units(civ, g.players[p].alignment, path);
    if units.is_empty() {
        return 100;
    }
    let mut cache: Vec<(Vec<BonusType>, i64)> = Vec::new();
    let mut best = 0;
    for u in units {
        let ui = &g.d.units[u];
        let mut needs: Vec<Vec<BonusType>> = Vec::new();
        if let Some(b) = ui.prereq_and_bonus {
            needs.push(vec![b]);
        }
        if !ui.prereq_or_bonuses.is_empty() {
            needs.push(ui.prereq_or_bonuses.clone());
        }
        let mut pct = RESOURCE_HAVE_PCT;
        for set in needs {
            let v = match cache.iter().find(|(s, _)| *s == set) {
                Some(&(_, v)) => v,
                None => {
                    let v = resource_set_pct(g, p, &set);
                    cache.push((set, v));
                    v
                }
            };
            pct = pct.min(v);
        }
        best = best.max(pct);
    }
    best
}

/// `resource_pct` for one requirement: any of the resources `set`.
fn resource_set_pct(g: &Game, p: PlayerId, set: &[BonusType]) -> i64 {
    if set.iter().any(|&b| g.player_get_num_available_bonuses(p, b) > 0) {
        return RESOURCE_HAVE_PCT;
    }
    let team = g.players[p].team;
    let cities: Vec<(i32, i32)> = g.players[p].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
    let in_reach = g.map.plot_ids().any(|q| {
        let plot = &g.map.plots[q];
        plot.revealed[team]
            && g.plot_get_bonus_type(q, Some(team)).is_some_and(|b| set.contains(&b))
            && (plot.owner == Some(p) || (plot.owner.is_none() && {
                let (x, y) = g.map.xy(q);
                cities.iter().any(|&(cx, cy)| g.plot_distance(x, y, cx, cy) <= RESOURCE_REACH)
            }))
    });
    if in_reach {
        RESOURCE_IN_REACH_PCT
    } else if set.iter().any(|&b| !g.team_is_has_tech(team, g.d.bonuses[b].tech_reveal) && !g.team_is_force_revealed_bonus(team, b)) {
        RESOURCE_UNSEEN_PCT
    } else {
        RESOURCE_NONE_PCT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path choice reads the resources the path's late units need as a player would: unseen,
    /// none in reach, one in reach, one held.
    #[test]
    fn the_choice_weighs_the_resources_of_the_paths_late_units() {
        // The Bannor (the Hippus palace supplies horses: they always have them).
        let mut g = crate::testkit::one_landmass_game_with(3, &["CIVILIZATION_BANNOR"]);
        let d = g.d;
        let p = PlayerId(0);
        let team = g.players[p].team;
        let cavalry = d.t::<ResearchPathType>("RESEARCHPATH_CAVALRY");
        let mercantile = d.t::<ResearchPathType>("RESEARCHPATH_MERCANTILE");
        let horse = d.t::<BonusType>("BONUS_HORSE");
        let husbandry = d.t::<TechType>("TECH_ANIMAL_HUSBANDRY");
        g.player_init_city(p, 4, 5, true, true);
        // Galleons and Men-o'-War need nothing: Mercantile is not held back.
        assert_eq!(resource_pct(&g, p, mercantile), RESOURCE_HAVE_PCT);
        // Knights need horses, which we cannot see yet.
        assert_eq!(resource_pct(&g, p, cavalry), RESOURCE_UNSEEN_PCT);
        g.team_set_has_tech(team, husbandry, true, Some(p), false, false);
        assert_eq!(resource_pct(&g, p, cavalry), RESOURCE_NONE_PCT);
        // Horses two plots from the city, unclaimed: in reach.
        let q = g.map.plot_xy(6, 5).expect("a plot");
        g.map.plots[q].bonus_type = Some(horse);
        assert_eq!(resource_pct(&g, p, cavalry), RESOURCE_IN_REACH_PCT);
    }

    #[test]
    fn every_civ_path_name_resolves() {
        let d = ffh_data::global();
        for (civ, paths) in CIV_PATHS {
            assert!(d.ty::<CivilizationType>(civ).is_some(), "{civ}");
            for p in *paths {
                assert!(d.ty::<ResearchPathType>(p).is_some(), "{p}");
            }
        }
    }
}
