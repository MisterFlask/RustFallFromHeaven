//! Research paths (docs/AI.md, "Research paths"; the rule: docs/PORTING.md, "Research paths").
//!
//! An expert commits at once: the commitment costs nothing but the freedom to switch before the
//! path is complete, and every turn uncommitted is research given away. It takes the path its
//! civilization is built for -- the one its own units, spells and heroes sit on (`CIV_PATHS`) --
//! and otherwise the path whose techs its leader cares for most, weighted by the research the
//! commitment saves. Once a path is complete it commits to the next. The path's late techs then
//! join the research plan (`tech_plan`, step `ResearchPath`), and the research score counts
//! their halved research time (`tech_plan::effective_left`).

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
    // Sea traders and pirates.
    ("CIVILIZATION_LANUN", &["RESEARCHPATH_MERCANTILE", "RESEARCHPATH_SHADOW"]),
    // Elven archers and the forest: the Flurry (Machinery), Druids.
    ("CIVILIZATION_LJOSALFAR", &["RESEARCHPATH_WILD", "RESEARCHPATH_ARTIFICE"]),
    // Golems: Clockwork (Machinery), Nullstone (Mithril Weapons).
    ("CIVILIZATION_LUCHUIRP", &["RESEARCHPATH_ARTIFICE", "RESEARCHPATH_STEEL"]),
    // The faith of the desert (FfH beelines Priesthood).
    ("CIVILIZATION_MALAKIM", &["RESEARCHPATH_FAITH"]),
    // The Eater of Dreams (Strength of Will), demons (Infernal Pact).
    ("CIVILIZATION_SHEAIM", &["RESEARCHPATH_ARCANE", "RESEARCHPATH_SHADOW"]),
    // Shadows and poisons (FfH beelines Poisons).
    ("CIVILIZATION_SIDAR", &["RESEARCHPATH_SHADOW", "RESEARCHPATH_ARCANE"]),
    ("CIVILIZATION_SVARTALFAR", &["RESEARCHPATH_SHADOW", "RESEARCHPATH_WILD"]),
];

/// The weight of a civilization's first and second path over a path it is not built for.
const FIRST_PATH_PCT: i64 = 400;
const SECOND_PATH_PCT: i64 = 200;

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
        let score = saved * (100 + interest_pct) / 100 * own_pct / 100;
        out.push((path, score));
    }
    out.sort_by_key(|&(path, score)| (std::cmp::Reverse(score), path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
