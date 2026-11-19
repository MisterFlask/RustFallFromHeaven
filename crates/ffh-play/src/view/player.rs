//! The empire screens (design §4.6): `techs` (the tech tree laid out by prerequisite depth),
//! `civics`, `religions` and `players` (the diplomacy overview of the players the human knows).

use std::collections::BTreeSet;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::persons::PersonsAct;
use ffh_data::*;

use crate::json::Json;
use crate::view::{players_brief, Viewer};

/// The prerequisite depth of every tech: 0 without prerequisites, else one more than its
/// deepest prerequisite (the tech tree's columns).
pub fn tech_depths(d: &GameData) -> Vec<i32> {
    fn depth(d: &GameData, t: TechType, memo: &mut Vec<i32>, stack: &mut Vec<TechType>) -> i32 {
        if memo[t.idx()] >= 0 {
            return memo[t.idx()];
        }
        if stack.contains(&t) {
            return 0; // a cycle in the data: cut it
        }
        stack.push(t);
        let info = &d.techs[t];
        let deepest = info.prereq_and_techs.iter().chain(&info.prereq_or_techs).map(|&p| depth(d, p, memo, stack) + 1).max().unwrap_or(0);
        stack.pop();
        memo[t.idx()] = deepest;
        deepest
    }
    let mut memo = vec![-1; d.techs.len()];
    let mut stack = Vec::new();
    for t in d.techs.keys() {
        depth(d, t, &mut memo, &mut stack);
    }
    memo
}

/// A list of type indices.
fn ids<I: Idx + Copy>(v: &[I]) -> Json {
    Json::Arr(v.iter().map(|&i| Json::idx(i)).collect())
}

/// `techs`: every tech with the human's state (has, can research, progress, cost, turns),
/// its prerequisites, its depth, its help (what it brings the human) and, unless the human has it,
/// what it unlocks as type indices (`unlocks`: the help's units, buildings, builds, civics and
/// religions; the screens draw them as icons).
pub fn techs(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let depths = tech_depths(d);
    let current = g.player_get_current_research(h);
    let out = d.techs.keys().map(|t| {
        let info = &d.techs[t];
        let has = g.teams[v.team].has_tech[t];
        let can = g.player_can_research(h, t, false);
        let un = crate::view::help::tech::unlocks(g, h, t);
        crate::jobj! {
            "tech" => Json::idx(t),
            "has" => has,
            "can_research" => can,
            "researching" => current == Some(t),
            "queued" => g.players[h].research_queue.iter().position(|&q| q == t),
            "progress" => g.teams[v.team].research_progress[t],
            "cost" => g.team_get_research_cost(v.team, t),
            "turns" => if can && !has { Json::from(g.player_get_research_turns_left(h, t, true)) } else { Json::Null },
            "and" => Json::Arr(info.prereq_and_techs.iter().map(|&p| Json::idx(p)).collect()),
            "or" => Json::Arr(info.prereq_or_techs.iter().map(|&p| Json::idx(p)).collect()),
            "depth" => depths[t.idx()],
            // The tech-specific research multipliers (`player_research_modifier_parts`, percent):
            // known by civilizations met and known alternative prerequisites (added to 100), a
            // held unique feature and the research path (each multiplying), and the total.
            "research_mods" => {
                let m = g.player_research_modifier_parts(h, t);
                crate::jobj! { "others" => m.known_by_others, "prereqs" => m.known_prereqs, "feature" => m.feature, "path" => m.path, "total" => m.total }
            },
            "disabled" => info.disable,
            // What it brings the human (C++ `setTechHelp`), for the research screens' tooltips.
            "help" => crate::view::help::tech::tech_help_with(g, h, t, &un),
            "unlocks" => if has {
                Json::Null
            } else {
                crate::jobj! {
                    "units" => ids(&un.units),
                    "buildings" => ids(&un.buildings),
                    "builds" => ids(&un.builds),
                    "civics" => ids(&un.civics),
                    "religions" => ids(&un.religions),
                }
            },
        }
    });
    crate::jobj! {
        "current" => Json::opt_idx(current),
        "queue" => Json::Arr(g.players[h].research_queue.iter().map(|&t| Json::idx(t)).collect()),
        "rate" => g.player_calculate_research_rate(h, None),
        "techs" => Json::Arr(out.collect()),
        "path" => Json::opt_idx(g.players[h].research_path),
        "path_percent" => g.d.defines.research_path_percent,
        "paths" => research_paths(v),
    }
}

/// The research paths for the human (docs/PORTING.md, "Research paths"): each path's techs (for
/// the human's alignment), which of them its team knows and which it can never research, the
/// research left on the others, whether every tech it can research is known (`complete`), and
/// whether the human may commit to it now (`refusal`: the reason it may not; once committed, every
/// path answers `Committed`).
pub fn research_paths(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let team = v.team;
    Json::Arr(
        d.research_paths
            .keys()
            .map(|path| {
                let techs = g.player_research_path_techs(h, path);
                let open = |t: TechType| !g.team_is_has_tech(team, Some(t)) && g.player_can_ever_research(h, t);
                let left: i64 = techs.iter().filter(|&&t| open(t)).map(|&t| g.team_get_research_left(team, t).max(0) as i64).sum();
                let refusal = g.player_research_path_refusal(h, path);
                crate::jobj! {
                    "path" => Json::idx(path),
                    "techs" => ids(&techs),
                    "known" => ids(&techs.iter().copied().filter(|&t| g.team_is_has_tech(team, Some(t))).collect::<Vec<_>>()),
                    // Techs of the path closed to the human (another state religion's, ...).
                    "closed" => ids(&techs.iter().copied().filter(|&t| !g.player_can_ever_research(h, t)).collect::<Vec<_>>()),
                    "capstones" => ids(&d.research_paths[path].capstones),
                    "left" => left,
                    "complete" => g.player_research_path_known(h, path),
                    "can_commit" => refusal.is_none(),
                    "refusal" => refusal.map_or(Json::Null, |r| Json::from(research_path_refusal_text(g, r))),
                }
            })
            .collect(),
    )
}

/// Why the human may not commit to a research path, as the screens and the command's refusal say
/// it.
pub fn research_path_refusal_text(g: &ffh_core::game::Game, r: ffh_core::player::research_path::ResearchPathRefusal) -> String {
    use ffh_core::player::research_path::ResearchPathRefusal as R;
    let d = g.d;
    match r {
        R::NotACiv => "Reason: no civilization.".to_string(),
        R::Committed(path) => format!("Reason: the {} doctrine is adopted for the rest of the game.", d.text.name(&d.research_paths[path].base.description)),
        R::Known => "Reason: every tech of the doctrine that this civilization can research is known.".to_string(),
        R::Closed => "Reason: the doctrine's late techs bring this civilization no unit.".to_string(),
    }
}

/// `civics`: each civic option with the current civic and every civic's availability, upkeep
/// and the anarchy a switch to it alone would cost. With `with` (civics to try, merged over the
/// current ones as `update_civics` merges them) also `proposed`: the merged civics, whether
/// `update_civics` would take them (every new civic `canDoCivics`, and `canRevolution` of the
/// set), and the set's anarchy and upkeep (the civics screen's totals, CvCivicsScreen.py).
pub fn civics(v: &Viewer, with: &[CivicType]) -> Json {
    let mut j = civics_screen(v);
    if !with.is_empty() {
        let g = v.g;
        let d = g.d;
        let h = v.h;
        let mut set = g.players[h].civics.clone();
        let mut valid = true;
        let mut seen = Vec::new();
        for &c in with {
            match d.civics[c].civic_option_type {
                Some(o) => {
                    if seen.contains(&o) || (set[o] != Some(c) && !g.player_can_do_civics(h, Some(c))) {
                        valid = false;
                    }
                    seen.push(o);
                    set[o] = Some(c);
                }
                None => valid = false,
            }
        }
        let changed = set != g.players[h].civics;
        valid = valid && changed && g.player_can_revolution(h, Some(&set));
        // This game's governors (docs/GOVERNORS.md): the government decides who runs the cities.
        let direct_after = set.iter().flatten().any(|&c| d.civics[c].direct_rule);
        let rule_change = match (g.player_rules_directly(h), direct_after) {
            (true, false) => Json::from("governed"),
            (false, true) => Json::from("direct"),
            _ => Json::Null,
        };
        let mut proposed = crate::jobj! {
            "civics" => Json::Arr(set.iter().map(|&c| Json::opt_idx(c)).collect()),
            "changed" => changed,
            "valid" => valid,
            "anarchy" => if changed { g.player_get_civic_anarchy_length(h, &set) } else { 0 },
            "upkeep" => g.player_get_civic_upkeep(h, Some(&set), false),
            // `governed`: every city will be handed to governors; `direct`: every city will be
            // ruled directly; null: no change of who runs them.
            "rule_change" => rule_change,
        };
        // Houses (docs/PERSONS.md, "UI", part I): what the set does to the Houses (`persons`),
        // and the warning of a direct-rule civic while Houses stand.
        if changed {
            let civics: Vec<Option<CivicType>> = set.iter().copied().collect();
            crate::view::house_marks::civics_preview(v, &civics, &mut proposed);
        }
        j.set("proposed", proposed);
    }
    j
}

fn civics_screen(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let pl = &g.players[h];
    let options = d.civic_options.keys().map(|o| {
        let civics = d.civics.keys().filter(|&c| d.civics[c].civic_option_type == Some(o)).map(|c| {
            let mut changed = pl.civics.clone();
            changed[o] = Some(c);
            let available = g.player_can_do_civics(h, Some(c));
            crate::jobj! {
                "civic" => Json::idx(c),
                "current" => pl.civics[o] == Some(c),
                "available" => available,
                // Why it cannot be adopted (the checks of C++ `canDoCivics`), as text.
                "requires" => if available { Json::Arr(Vec::new()) } else { Json::Arr(civic_requires(g, h, c).into_iter().map(Json::from).collect()) },
                // What it does, without the requirements `requires` says (the data's `effects`
                // has them all).
                "help" => crate::view::help::civic::effects_without_requirements(d, c),
                "upkeep" => g.player_get_single_civic_upkeep(h, Some(c), false),
                "anarchy" => if pl.civics[o] == Some(c) { 0 } else { g.player_get_civic_anarchy_length(h, &changed) },
            }
        });
        crate::jobj! { "option" => Json::idx(o), "current" => Json::opt_idx(pl.civics[o]), "civics" => Json::Arr(civics.collect()) }
    });
    crate::jobj! {
        "options" => Json::Arr(options.collect()),
        "can_revolution" => g.player_can_revolution(h, None),
        "revolution_timer" => pl.revolution_timer,
        "anarchy" => pl.anarchy_turns,
        "upkeep" => g.player_get_civic_upkeep(h, None, false),
    }
}

/// Why player `p` cannot adopt civic `c`: the checks of C++ `CvPlayer::canDoCivics` that fail,
/// in its order, as sentences ("Requires Code of Laws", "Only while at war", ...); a check the
/// rules' Python makes says "Not possible now".
pub fn civic_requires(g: &ffh_core::game::Game, p: PlayerId, c: CivicType) -> Vec<String> {
    use crate::names::display;
    let d = g.d;
    let info = &d.civics[c];
    let pl = &g.players[p];
    let mut out = Vec::new();
    if let Some(o) = info.civic_option_type {
        if g.player_is_full_member(p, VoteSourceType(0)) && g.is_force_civic_option(o) && !g.is_force_civic(c) {
            out.push("A resolution of the council forces another civic".to_string());
            return out;
        }
        if !g.player_is_has_civic_option(p, o) && !g.team_is_has_tech(pl.team, info.tech_prereq) {
            if let Some(t) = info.tech_prereq {
                out.push(format!("Requires {}", display(d, t)));
            }
        }
    }
    if info.prereq_war && g.team_get_at_war_count(pl.team, true) == 0 {
        out.push("Only while at war".to_string());
    }
    if let Some(a) = info.block_alignment.filter(|&a| pl.alignment == Some(a)) {
        out.push(format!("Not for {} civilizations", display(d, a)));
    }
    if let Some(a) = info.prereq_alignment.filter(|&a| pl.alignment != Some(a)) {
        out.push(format!("Only for {} civilizations", display(d, a)));
    }
    if let Some(civ) = info.prereq_civilization.filter(|&x| g.player_get_civilization_type(p) != Some(x)) {
        out.push(format!("Only for the {}", display(d, civ)));
    }
    if let Some(r) = info.prereq_religion.filter(|&r| g.player_get_state_religion(p) != Some(r)) {
        out.push(format!("Requires {} as state religion", display(d, r)));
    }
    if out.is_empty() {
        out.push("Not possible now".to_string());
    }
    out
}

/// `religions`: the state religion, conversion, and every religion's founding and holy city (as
/// far as the human has revealed it).
pub fn religions(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    let pl = &g.players[h];
    let out = d.religions.keys().map(|r| {
        let holy = g.get_holy_city(r).filter(|&c| v.city_revealed(c));
        crate::jobj! {
            "religion" => Json::idx(r),
            "founded" => g.is_religion_founded(r),
            "holy_city" => holy,
            "our_holy_city" => g.player_has_holy_city(h, r),
            "our_cities" => pl.cities.iter().filter(|&&c| g.cities[c].has_religion[r]).count(),
            "can_convert" => g.player_can_convert(h, Some(r)),
            // Houses (docs/PERSONS.md, "UI", part I): what converting does to the Houses.
            "persons" => if g.player_can_convert(h, Some(r)) { crate::view::house_marks::act_preview(v, &PersonsAct::Convert(Some(r))) } else { Json::Arr(Vec::new()) },
        }
    });
    crate::jobj! {
        "state_religion" => Json::opt_idx(g.player_get_state_religion(h)),
        "can_convert_none" => g.player_can_convert(h, None),
        "persons_none" => if g.player_can_convert(h, None) { crate::view::house_marks::act_preview(v, &PersonsAct::Convert(None)) } else { Json::Arr(Vec::new()) },
        "conversion_timer" => pl.conversion_timer,
        "anarchy" => g.player_get_religion_anarchy_length(h),
        "religions" => Json::Arr(out.collect()),
    }
}

/// The items of a deal as `[{item, data}]`.
pub fn trades_json(list: &[ffh_core::structs::TradeData]) -> Json {
    Json::Arr(list.iter().map(|t| crate::jobj! { "item" => Json::idx(t.item_type), "data" => t.data }).collect())
}

/// `players`: the view's player list, plus each player's deals with the human (with whether the
/// human may cancel each now, else in how many turns: `cancel_in`, 0 for never) and whether the
/// human can declare war on it.
pub fn players(v: &Viewer) -> Json {
    let g = v.g;
    let Json::Arr(mut list) = players_brief(v) else { unreachable!() };
    for j in &mut list {
        let p = PlayerId(j.get("id").and_then(Json::as_i64).unwrap_or(0) as u8);
        let t = g.players[p].team;
        let deals: Vec<Json> = g
            .deals
            .iter()
            .filter_map(|(id, deal)| {
                let (ours, theirs) = if (deal.first_player, deal.second_player) == (v.h, p) {
                    (&deal.first_trades, &deal.second_trades)
                } else if (deal.first_player, deal.second_player) == (p, v.h) {
                    (&deal.second_trades, &deal.first_trades)
                } else {
                    return None;
                };
                // When the human may cancel it (Civ4: after `PEACE_TREATY_LENGTH` turns); never for
                // a vassal deal the rules hold (a House's: docs/PERSONS.md, "The second build", rule
                // 9; a capitulation), so no count runs for it.
                let cancelable = g.deal_is_cancelable(id, Some(v.h));
                let cancel_in = if cancelable || g.deal_is_uncancelable_vassal_deal(id, Some(v.h)) { 0 } else { g.deal_turns_to_cancel(id, Some(v.h)).max(0) };
                Some(crate::jobj! { "deal" => id, "we_give" => trades_json(ours), "they_give" => trades_json(theirs), "turn" => deal.initial_game_turn, "cancelable" => cancelable, "cancel_in" => cancel_in })
            })
            .collect();
        j.set("deals", Json::Arr(deals));
        let can_war = t != v.team && g.team_can_declare_war(v.team, t);
        j.set("can_declare_war", can_war);
        // Houses (docs/PERSONS.md, "UI", part I): what declaring war on it does to the Houses
        // (the war dialog's lines; empty when nothing).
        j.set("war_preview", if can_war { crate::view::house_marks::act_preview(v, &PersonsAct::DeclareWar(t)) } else { Json::Arr(Vec::new()) });
        // Who the war brings in for certain, by the vassal rules (docs/PERSONS.md, "The second
        // build", rule 7; the war dialog's `At war with you as well: ..` and `Your Houses join:
        // ..`): its vassals (a House's: its liege and the liege's other vassals) and the human's,
        // labels in player order; empty when it cannot be declared.
        let (against, join) = if can_war { war_parties(g, v.team, t, None) } else { (Vec::new(), Vec::new()) };
        j.set("war_against", party_labels(g, &against));
        j.set("war_join", party_labels(g, &join));
        // What the declaration costs with them (the owner's decision, 2026-11-18): the war dialog's
        // `They remember it: attitude toward you -1 (fades).` (null when nothing).
        j.set("war_memory", if can_war { war_memory(g, v.h, t, &against) } else { None });
        // A House not the human's (rule 9; the diplomacy list's second line): its state toward
        // its own liege and its price to secede.
        if let Some(hh) = g.player_house(p).filter(|&hh| g.players[g.houses[hh].owner].team != v.team) {
            j.set("house_offer", house_offer(v, hh));
        }
        // A crusade's Host (docs/CRUSADE.md): an army, not a civilization to talk to.
        j.set("crusade_host", g.player_is_crusade_host(p));
    }
    crate::jobj! { "me" => v.h, "players" => Json::Arr(list) }
}

/// The teams a war between teams `t` and `other` brings in, as `team_declare_war` propagates it
/// by the vassal rules (docs/PERSONS.md, "The second build", rule 7: a vassal is at war with its
/// master's enemies, and a war on a vassal is a war on its master): `(against, join)`, the teams
/// newly at war with `t` other than `other` (the target's vassals), and those newly at war with
/// `other` other than `t` (`t`'s own vassals), by team id. As the rules: no war reaches a crusade's
/// Host nor crosses a truce. `gone`: a vassal team that has left its master before the war (a
/// House breaking away to become `t`'s: it is in neither list). Defensive pacts are left out:
/// the human does not see the others' pacts (the war dialog says they may bring others in).
pub fn war_parties(g: &Game, t: TeamId, other: TeamId, gone: Option<TeamId>) -> (Vec<TeamId>, Vec<TeamId>) {
    let is_vassal = |v: TeamId, m: TeamId| Some(v) != gone && g.teams[v].vassal[m];
    let pair = |a: TeamId, b: TeamId| (a.min(b), a.max(b));
    let mut new: BTreeSet<(TeamId, TeamId)> = BTreeSet::new();
    let mut todo = vec![(t, other)];
    while let Some((a, b)) = todo.pop() {
        if a == b || g.team_is_crusade_host(a) || g.team_is_crusade_host(b) || g.team_has_truce(a, b) || g.teams[a].at_war[b] || !new.insert(pair(a, b)) {
            continue;
        }
        for o in g.team_ids() {
            if o == a || o == b || !g.team_is_alive(o) {
                continue;
            }
            if is_vassal(o, b) || is_vassal(b, o) {
                todo.push((a, o));
            } else if is_vassal(o, a) || is_vassal(a, o) {
                todo.push((o, b));
            }
        }
    }
    let against = g.team_ids().filter(|&x| x != other && x != t && new.contains(&pair(t, x))).collect();
    let join = g.team_ids().filter(|&y| y != t && y != other && new.contains(&pair(y, other))).collect();
    (against, join)
}

/// The players of `teams` as the screens name a party (a House by its label, `House Arne`,
/// `Malakim`; another by its civilization, `the Svartalfar`: a House's liege, when war is declared
/// on the House), alive, in player order.
pub fn party_labels(g: &Game, teams: &[TeamId]) -> Json {
    Json::Arr(g.player_ids().into_iter().filter(|&p| g.players[p].alive && teams.contains(&g.players[p].team)).map(|p| Json::from(party_label(g, p))).collect())
}

/// Player `p` as the screens name a party: `House Arne`, `Malakim`, `the Svartalfar`.
fn party_label(g: &Game, p: PlayerId) -> String {
    match g.player_house(p) {
        Some(h) => g.house_label(h),
        None => {
            let name = g.player_house_name(p);
            if name == g.player_get_civilization_short_description(p) {
                format!("the {name}")
            } else {
                name
            }
        }
    }
}

/// The war dialog's line of what declaring war on team `t` costs with it (the owner's decision,
/// 2026-11-18): its leader remembers it (`MEMORY_DECLARED_WAR`, which fades) and its attitude
/// toward the human `h` moves by ffh-ai's `memory_added`: `They remember it: attitude toward you
/// -1 (fades).`; the parties the vassal rules bring in (`against`, [`war_parties`]) each remember
/// it too, and those it moves are named: ` So do House Arne and House Vel.` Null when it moves
/// nothing (a human leader, the cap).
pub fn war_memory(g: &Game, h: PlayerId, t: TeamId, against: &[TeamId]) -> Option<String> {
    use ffh_ai::diplomacy::memory_added;
    let war = MemoryType::MEMORY_DECLARED_WAR;
    let leader = g.team_get_leader_id(t)?;
    let n = memory_added(g, leader, h, war);
    if n == 0 {
        return None;
    }
    let also: Vec<String> = g.player_ids().filter(|&p| g.players[p].alive && against.contains(&g.players[p].team) && memory_added(g, p, h, war) != 0).map(|p| party_label(g, p)).collect();
    let signed = if n > 0 { format!("+{n}") } else { n.to_string() };
    let mut line = format!("They remember it: attitude toward you {signed} (fades).");
    if !also.is_empty() {
        line.push_str(&format!(" So do {}.", and_list(&also)));
    }
    Some(line)
}

/// "A", "A and B", "A, B and C".
fn and_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// House `hh` (not the human's) as the diplomacy list's second line states it (docs/PERSONS.md,
/// "The second build", rules 5 and 9): `{state, turns, price, refusal}`, its state toward its own
/// liege (`turns`: the Furious countdown's turns left, else null), its price to secede when it
/// would take the human's offer (else null), and the rules' reason it would not when that reason
/// is not its state (null otherwise: the state's mark says it).
fn house_offer(v: &Viewer, hh: HouseId) -> Json {
    use ffh_core::persons::{BribeKind, LoyaltyState};
    let g = v.g;
    let state = g.house_state(hh);
    let refusal = g.house_bribe_refusal(hh, v.h, BribeKind::Secede);
    crate::jobj! {
        "state" => state.name(),
        "turns" => g.house_view(hh).and_then(|x| x.countdown_turns),
        "price" => if refusal.is_none() { g.house_bribe_price(hh) } else { None },
        "refusal" => refusal.filter(|_| state >= LoyaltyState::Cautious),
    }
}
