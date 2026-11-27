//! AI player for Cataclysm — plays 100% through `cataclysm_core::Command`, with zero AI-only rules.

use cataclysm_core::{
    Alignment, BuildOrder, Command, DistrictKind, Doctrine, Event, FactionId, Game, GreatPersonAction, HouseMood, PlayerId, PlotIdx, Religion,
    RitualKind, TechId, UnitClassId, WonderKind, TECHS,
};

/// How a player uses the rituals that move Worldseal Strain. `Standard` is the game's AI; the
/// others are measuring instruments for `cataclysm-tally` (they push one direction as hard as
/// they can, to show what the rules allow), not ways the game's AI plays.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Policy {
    #[default]
    Standard,
    /// Never casts a ritual that raises Strain.
    Quiet,
    /// Spends its Mana on the ritual that raises Strain most per Mana, every turn.
    Pusher,
}

/// Plays a complete turn for `player` (ending with `Command::EndTurn`) and returns all emitted events.
pub fn play_turn(g: &mut Game, player: PlayerId) -> Vec<Event> {
    play_turn_as(g, player, Policy::Standard)
}

/// [`play_turn`] under a ritual [`Policy`].
pub fn play_turn_as(g: &mut Game, player: PlayerId, policy: Policy) -> Vec<Event> {
    let mut events = Vec::new();
    if g.is_over() || !g.players[player.idx()].alive {
        return events;
    }

    if g.players[player.idx()].is_neutral {
        play_neutral_turn(g, player, &mut events);
        if let Ok(ev) = g.apply(player, Command::EndTurn) {
            events.extend(ev);
        }
        return events;
    }

    manage_doctrine_and_research(g, player, &mut events);
    manage_religion(g, player, &mut events);
    manage_court(g, player, &mut events);
    manage_rituals(g, player, policy, &mut events);
    manage_diplomacy_and_crusades(g, player, &mut events);
    manage_cities_and_caravans(g, player, &mut events);
    manage_squads(g, player, &mut events);

    if !g.is_over() {
        if let Ok(ev) = g.apply(player, Command::EndTurn) {
            events.extend(ev);
        }
    }
    events
}

fn try_cmd(g: &mut Game, p: PlayerId, cmd: Command, events: &mut Vec<Event>) -> bool {
    if g.is_over() {
        return false;
    }
    if let Ok(ev) = g.apply(p, cmd) {
        events.extend(ev);
        true
    } else {
        false
    }
}

fn manage_doctrine_and_research(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let pl = &g.players[p.idx()];
    if pl.primary_doctrine.is_none() && pl.techs.len() >= 2 {
        let doc = pl.faction.info().preferred_doctrine;
        try_cmd(g, p, Command::AdoptDoctrine { doctrine: doc, secondary: false }, events);
    } else if g.players[p.idx()].primary_doctrine.is_some() && g.players[p.idx()].secondary_doctrine.is_none() && g.players[p.idx()].techs.len() >= 6 {
        let prim = g.players[p.idx()].primary_doctrine.unwrap();
        let sec = match prim {
            Doctrine::Arcane | Doctrine::Faith => Doctrine::Artifice,
            Doctrine::Steel | Doctrine::Cavalry => Doctrine::Mercantile,
            _ => Doctrine::Steel,
        };
        if sec != prim {
            try_cmd(g, p, Command::AdoptDoctrine { doctrine: sec, secondary: true }, events);
        }
    }

    if g.players[p.idx()].current_research.is_none() {
        let prim = g.players[p.idx()].primary_doctrine;
        let mut best: Option<(i32, TechId)> = None;
        for t in &TECHS {
            if g.can_research(p, t.id).is_ok() {
                let mut score = 100 - t.cost / 4;
                if t.doctrine == prim && prim.is_some() {
                    score += 80 + t.tier as i32 * 15;
                } else if t.doctrine.is_none() {
                    score += 50;
                }
                if best.is_none_or(|(s, _)| score > s) {
                    best = Some((score, t.id));
                }
            }
        }
        if let Some((_, tech)) = best {
            try_cmd(g, p, Command::ChooseResearch { tech }, events);
        }
    }
}

fn manage_religion(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    if g.players[p.idx()].religion.is_some() {
        return;
    }
    let align = g.players[p.idx()].alignment;
    let pref = match (g.players[p.idx()].faction, align) {
        (FactionId::Sheaim | FactionId::Infernals, _) => Religion::AshenVeil,
        (FactionId::Ljosalfar, _) => Religion::Leaves,
        (FactionId::Khazad, _) => Religion::Runes,
        (_, Alignment::Good) => Religion::Order,
        (_, Alignment::Evil) => Religion::AshenVeil,
        _ => Religion::Runes,
    };
    try_cmd(g, p, Command::FoundOrAdoptReligion { religion: pref }, events);
}

fn manage_court(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let gps: Vec<_> = g.court.values().filter(|gp| gp.owner == p).cloned().collect();
    for gp in gps {
        // Priority 1: fix a Furious/Restless House if we have one.
        let bad_house = g
            .houses
            .values()
            .filter(|h| h.owner == p)
            .find(|h| matches!(g.house_loyalty(h.id).1, HouseMood::Furious | HouseMood::Restless))
            .map(|h| h.id);
        if let Some(hid) = bad_house {
            if try_cmd(g, p, Command::UseGreatPerson { gp: gp.id, action: GreatPersonAction::CommissionLord(hid) }, events) {
                continue;
            }
        }
        // Priority 2: Discover unique Secret Tech if available!
        if gp.secret_tech.is_some() {
            if try_cmd(g, p, Command::UseGreatPerson { gp: gp.id, action: GreatPersonAction::DiscoverSecret }, events) {
                continue;
            }
        }
        // Priority 3: Commission as Squad Hero if we have a squad with space.
        let cap = g.players[p.idx()].squad_place_cap();
        if let Some(sid) = g.squads.values().find(|s| s.owner == p && s.places_used() + 1 <= cap).map(|s| s.id) {
            if try_cmd(g, p, Command::UseGreatPerson { gp: gp.id, action: GreatPersonAction::CommissionHero(sid) }, events) {
                continue;
            }
        }
        // Fallback: Tempo Burst.
        try_cmd(g, p, Command::UseGreatPerson { gp: gp.id, action: GreatPersonAction::TempoBurst }, events);
    }
}

fn manage_rituals(g: &mut Game, p: PlayerId, policy: Policy, events: &mut Vec<Event>) {
    if policy == Policy::Pusher {
        // The most Strain per Mana, for as long as the rules let it cast.
        for _ in 0..20 {
            let best = RitualKind::ALL
                .into_iter()
                .filter(|r| r.ac_delta() > 0 && g.check_ritual(p, *r).is_ok())
                .max_by_key(|r| (r.ac_delta() * 1000 / r.mana_cost().max(1), std::cmp::Reverse(r.mana_cost())));
            let Some(ritual) = best else { break };
            if !try_cmd(g, p, Command::CastRitual { ritual }, events) {
                break;
            }
        }
        return;
    }
    let fac = g.players[p.idx()].faction;
    let mana = g.players[p.idx()].mana;
    if mana < 25 {
        return;
    }
    let ritual = if fac == FactionId::Elohim && g.apocalypse.counter >= 10 {
        RitualKind::Sanctify
    } else if matches!(fac, FactionId::Sheaim | FactionId::Infernals) && !g.apocalypse.carcosa_risen && mana >= 35 {
        RitualKind::BlightStrike
    } else if mana >= 50 && g.player_squads(p).len() < g.squad_cap(p) {
        RitualKind::SummonHost
    } else if mana >= 35 {
        RitualKind::ArcaneSurge
    } else {
        return;
    };
    if policy == Policy::Quiet && ritual.ac_delta() > 0 {
        return;
    }
    try_cmd(g, p, Command::CastRitual { ritual }, events);
}

fn manage_diplomacy_and_crusades(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    // Check if any enemy is raiding us heavily -> pay off if weak, else fight.
    let my_power: i32 = g.squads.values().filter(|s| s.owner == p).map(|s| s.power_rating()).sum();
    let fac = g.players[p.idx()].faction;

    // Opportunistic war declaration when strong or during Armageddon Lifeboat race!
    let at_war_count = g.players.iter().filter(|o| o.alive && !o.is_neutral && g.at_war(p, o.id)).count();
    if at_war_count == 0 && (g.turn >= 25 || g.apocalypse.carcosa_risen) && my_power >= 300 {
        let mut target = None;
        if g.apocalypse.carcosa_risen {
            // Target someone holding a Lifeboat!
            for lb in &g.apocalypse.lifeboats {
                if !lb.razed {
                    if let Some(c) = g.cities.get(&lb.city_id) {
                        if c.owner != p && !g.players[c.owner.idx()].is_neutral {
                            target = Some(c.owner);
                            break;
                        }
                    }
                }
            }
        }
        if target.is_none() && (matches!(fac, FactionId::Hippus | FactionId::Bannor | FactionId::Sheaim | FactionId::Infernals) || g.turn >= 45) {
            target = g
                .players
                .iter()
                .filter(|o| o.alive && !o.is_neutral && o.id != p && o.alignment != g.players[p.idx()].alignment)
                .min_by_key(|o| o.score)
                .map(|o| o.id);
        }
        if let Some(t) = target {
            try_cmd(g, p, Command::DeclareWar { target: t }, events);
            try_cmd(g, p, Command::CallCrusade { target: t }, events);
        }
    }

    // Lend a squad to an active Crusade against a rival if we have >= 3 squads.
    if g.player_squads(p).len() >= 3 {
        let cr = g
            .crusades
            .values()
            .find(|c| c.active && c.target != p && g.players[c.target.idx()].alignment != g.players[p.idx()].alignment)
            .map(|c| c.id);
        if let Some(cr_id) = cr {
            if let Some(sid) = g.squads.values().find(|s| s.owner == p && s.lent_by.is_none()).map(|s| s.id) {
                try_cmd(g, p, Command::LendSquadToCrusade { crusade: cr_id, squad: sid }, events);
            }
        }
    }
}

fn best_squad_recipe(g: &Game, p: PlayerId) -> Vec<UnitClassId> {
    let cap = g.players[p.idx()].squad_place_cap();
    let pick = |prefs: &[UnitClassId]| -> UnitClassId {
        prefs.iter().copied().find(|&c| g.can_recruit_class(p, c).is_ok()).unwrap_or(UnitClassId::Guard)
    };
    let front1 = pick(&[UnitClassId::Berserker, UnitClassId::Phalanx, UnitClassId::Paladin, UnitClassId::Eidolon, UnitClassId::Champion, UnitClassId::Pikeman, UnitClassId::Axeman, UnitClassId::Guard]);
    let front2 = pick(&[UnitClassId::Knight, UnitClassId::Horseman, UnitClassId::PyreZombie, UnitClassId::Pikeman, UnitClassId::Axeman, UnitClassId::Guard]);
    let back1 = pick(&[UnitClassId::Archmage, UnitClassId::Mage, UnitClassId::Longbowman, UnitClassId::Trebuchet, UnitClassId::Catapult, UnitClassId::Archer, UnitClassId::Adept, UnitClassId::Guard]);
    let back2 = pick(&[UnitClassId::Paladin, UnitClassId::Acolyte, UnitClassId::Assassin, UnitClassId::Ranger, UnitClassId::Archer, UnitClassId::Guard]);
    let mut recipe = vec![front1, front2, back1];
    if cap >= 4 {
        recipe.push(back2);
    }
    recipe
}

fn manage_cities_and_caravans(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let city_ids = g.player_cities(p);
    let sq_count = g.player_squads(p).len();
    let sq_cap = g.squad_cap(p);
    let car_count = g.caravans.values().filter(|c| c.owner == p).count();
    let car_cap = g.caravan_cap(p);

    // Launch a Caravan if we have slots and Gold!
    if car_count < car_cap && g.players[p.idx()].gold >= 90 && !city_ids.is_empty() {
        let origin = city_ids[0];
        let dest = g
            .cities
            .values()
            .filter(|c| c.id != origin && !g.at_war(p, c.owner) && !g.players[c.owner.idx()].is_neutral)
            .max_by_key(|c| (c.owner != p, c.size))
            .map(|c| c.id);
        if let Some(d) = dest {
            try_cmd(g, p, Command::LaunchCaravan { origin, destination: d, resource: cataclysm_core::Resource::None }, events);
        }
    }

    // A Wonder is built once in the world: order it in one city only, the one with the most
    // Industry, and not while another of our cities is already building it. The Keepers of the
    // Seal win by holding the Strain below 100 until turn 160; a Wonder adds Strain, so they
    // build one only once Strain is high anyway or the Blowout has come (then it is a Lifeboat).
    let wonder_city = city_ids.iter().copied().max_by_key(|&cid| (g.city_yields(cid).1.total, std::cmp::Reverse(cid)));
    let keepers_hold_off = g.players[p.idx()].faction == FactionId::Elohim && g.apocalypse.counter < 60 && !g.apocalypse.carcosa_risen;

    for cid in city_ids.clone() {
        // Demonic AIs raze captured Lifeboats immediately when unprotected!
        if g.players[p.idx()].faction == FactionId::Infernals && g.apocalypse.is_lifeboat(cid).is_some() && g.apocalypse.can_raze_city(cid) {
            try_cmd(g, p, Command::RazeCity { city: cid }, events);
            continue;
        }
        let Some(c) = g.cities.get(&cid).cloned() else { continue };
        if c.building.is_some() {
            continue;
        }
        // Priority 1: Lifeboat Wonder if unlocked (in the Wonder city only).
        let mut chosen = None;
        if Some(cid) == wonder_city && !keepers_hold_off {
            for w in WonderKind::ALL {
                let in_progress = city_ids.iter().any(|o| g.cities.get(o).is_some_and(|oc| oc.building == Some(BuildOrder::Wonder(w))));
                if !g.built_wonders.contains_key(&w) && g.players[p.idx()].has_tech(w.tech()) && !in_progress {
                    chosen = Some(BuildOrder::Wonder(w));
                    break;
                }
            }
        }
        // Priority 2: Raise Squads when below cap!
        if chosen.is_none() && sq_count < sq_cap.min(4) {
            chosen = Some(BuildOrder::Squad(best_squad_recipe(g, p)));
        }
        // Priority 3: Build Macro Districts while slots remain!
        if chosen.is_none() && c.districts.len() < c.max_district_slots(g.is_megacity(&c)) {
            let order = [
                DistrictKind::Barracks,
                DistrictKind::Bulwark,
                DistrictKind::Academy,
                DistrictKind::Market,
                DistrictKind::Forge,
                DistrictKind::Temple,
            ];
            for d in order {
                if !c.has_district(d) && d.info().tech.is_none_or(|t| g.players[p.idx()].has_tech(t)) {
                    chosen = Some(BuildOrder::District(d));
                    break;
                }
            }
        }
        // Priority 4: Train squads up to cap or reinforce.
        if chosen.is_none() {
            if g.player_squads(p).len() < sq_cap {
                chosen = Some(BuildOrder::Squad(best_squad_recipe(g, p)));
            } else {
                chosen = Some(BuildOrder::Reinforce(UnitClassId::Guard));
            }
        }
        if let Some(ord) = chosen {
            try_cmd(g, p, Command::CityBuild { city: cid, order: ord }, events);
        }
    }
}

fn manage_squads(g: &mut Game, p: PlayerId, events: &mut Vec<Event>) {
    let squad_ids = g.player_squads(p);
    let my_cities_count = g.player_cities(p).len();
    let cap = g.players[p.idx()].squad_place_cap();

    for sid in squad_ids {
        if !g.squads.contains_key(&sid) {
            continue;
        }
        // Upgrade or reinforce inside own borders when Gold allows.
        let in_home = g.world.plot(g.squads[&sid].plot).owner == Some(p);
        if in_home && g.players[p.idx()].gold >= 80 {
            let upgrade_target = g.squads[&sid]
                .members
                .iter()
                .find(|m| m.class.info().upgrades_to.is_some_and(|n| g.can_recruit_class(p, n).is_ok()))
                .map(|m| m.id);
            if let Some(mid) = upgrade_target {
                try_cmd(g, p, Command::UpgradeMember { squad: sid, member: mid }, events);
            } else if g.squads[&sid].places_used() < cap {
                let rec = best_squad_recipe(g, p);
                let cl = rec[g.squads[&sid].members.len() % rec.len()];
                try_cmd(g, p, Command::ReinforceSquad { squad: sid, class: cl }, events);
            }
            try_cmd(g, p, Command::AutoArrangeSquad { squad: sid }, events);
        }

        // Step movement/combat up to 4 times per turn.
        for _ in 0..4 {
            let Some(s) = g.squads.get(&sid).cloned() else { break };
            if s.moves_left <= 0 {
                break;
            }
            // Explore lair on current tile!
            if g.world.plot(s.plot).lair_unexplored {
                if try_cmd(g, p, Command::SquadExploreLair { squad: sid }, events) {
                    break;
                }
            }
            // Found a new city if < 4 cities, rich, and on good open land >= 4 tiles from any city!
            if my_cities_count < 4
                && g.players[p.idx()].gold >= 130
                && g.world.plot(s.plot).city.is_none()
                && g.cities.values().all(|c| g.world.distance(c.plot, s.plot) >= 4)
            {
                if try_cmd(g, p, Command::SquadFoundCity { squad: sid, name: None }, events) {
                    break;
                }
            }

            // Pick strategic destination:
            // 1. After AC 100: march to an unowned Lifeboat (Carcosa!) or defend own Lifeboat.
            // 2. Otherwise: closest enemy city / squad at war, or nearest unexplored lair.
            let target_plot = choose_squad_target(g, p, s.plot);
            let Some(goal) = target_plot else { break };
            if goal == s.plot {
                try_cmd(g, p, Command::SetSquadFortify { squad: sid }, events);
                break;
            }

            // If goal is adjacent and hostile -> attack!
            if g.world.distance(s.plot, goal) == 1 {
                let enemy_here = g.squads.values().any(|o| o.plot == goal && g.at_war(p, o.owner))
                    || g.world.plot(goal).city.and_then(|cid| g.cities.get(&cid)).is_some_and(|c| g.at_war(p, c.owner));
                if enemy_here {
                    try_cmd(g, p, Command::AttackPlot { squad: sid, to: goal }, events);
                    break;
                }
            }

            // Greedy step toward `goal` along passable land (avoiding peaceful rival cities).
            let mut nbs = g.world.neighbors(s.plot);
            nbs.retain(|&nb| {
                let pt = g.world.plot(nb);
                if !pt.kind.is_passable_land() {
                    return false;
                }
                if let Some(cid) = pt.city {
                    let c = &g.cities[&cid];
                    if c.owner != p && !g.at_war(p, c.owner) {
                        return false;
                    }
                }
                true
            });
            nbs.sort_by_key(|&nb| (g.world.distance(nb, goal), !g.world.plot(nb).road, nb));
            let Some(&step) = nbs.first() else { break };
            if g.world.distance(step, goal) >= g.world.distance(s.plot, goal) {
                break;
            }
            if !try_cmd(g, p, Command::MoveSquad { squad: sid, to: step }, events) {
                break;
            }
        }
    }
}

fn choose_squad_target(g: &Game, p: PlayerId, from: PlotIdx) -> Option<PlotIdx> {
    if g.apocalypse.carcosa_risen {
        let demonic = g.players[p.idx()].faction == FactionId::Infernals;
        // Prioritize any Lifeboat not owned by us (or if demonic, ANY Lifeboat to raze!).
        let mut best_lb = None;
        for lb in &g.apocalypse.lifeboats {
            if lb.razed {
                continue;
            }
            if let Some(c) = g.cities.get(&lb.city_id) {
                if demonic || c.owner != p {
                    let d = g.world.distance(from, c.plot);
                    if best_lb.is_none_or(|(bd, _)| d < bd) {
                        best_lb = Some((d, c.plot));
                    }
                }
            }
        }
        if let Some((_, plot)) = best_lb {
            return Some(plot);
        }
    }

    // Nearest enemy city at war.
    if let Some(c) = g
        .cities
        .values()
        .filter(|c| g.at_war(p, c.owner) && !g.players[c.owner.idx()].is_neutral)
        .min_by_key(|c| g.world.distance(from, c.plot))
    {
        return Some(c.plot);
    }

    // Nearest enemy squad at war within 6 tiles.
    if let Some(es) = g
        .squads
        .values()
        .filter(|s| g.at_war(p, s.owner) && g.world.distance(from, s.plot) <= 6)
        .min_by_key(|s| g.world.distance(from, s.plot))
    {
        return Some(es.plot);
    }

    // Nearest unexplored lair.
    let mut best_lair: Option<(i32, PlotIdx)> = None;
    for i in 0..g.world.len() {
        let pt = PlotIdx(i as u32);
        if g.world.plot(pt).lair_unexplored {
            let d = g.world.distance(from, pt);
            if best_lair.is_none_or(|(bd, _)| d < bd) {
                best_lair = Some((d, pt));
            }
        }
    }
    if let Some((_, p_lair)) = best_lair {
        return Some(p_lair);
    }

    // Otherwise patrol near capital.
    g.cities.values().find(|c| c.owner == p && c.capital).map(|c| c.plot)
}

fn play_neutral_turn(g: &mut Game, neutral: PlayerId, events: &mut Vec<Event>) {
    let squad_ids = g.player_squads(neutral);
    for sid in squad_ids {
        let Some(s) = g.squads.get(&sid).cloned() else { continue };
        // Carcosa guardians hold Carcosa!
        if g.apocalypse.carcosa_city.is_some_and(|cid| g.cities.get(&cid).is_some_and(|c| c.plot == s.plot)) {
            continue;
        }
        // Attack any adjacent non-neutral squad.
        let adj_enemy = g
            .world
            .neighbors(s.plot)
            .into_iter()
            .find(|&nb| g.squads.values().any(|o| o.plot == nb && o.owner != neutral));
        if let Some(target) = adj_enemy {
            try_cmd(g, neutral, Command::AttackPlot { squad: sid, to: target }, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cataclysm_core::GameSetup;

    #[test]
    fn ai_soak_plays_full_game_to_completion_without_invariant_errors() {
        for seed in [1u64, 42, 99] {
            let mut g = Game::new(GameSetup {
                seed,
                width: 22,
                height: 16,
                human_faction: FactionId::Sheaim,
                ai_count: 4,
            });
            let mut steps = 0;
            while !g.is_over() && steps < 1800 {
                let p = g.active_player;
                let _ = play_turn(&mut g, p);
                if steps % 25 == 0 {
                    g.check_invariants().expect("invariants must hold throughout AI soak");
                }
                steps += 1;
            }
            g.check_invariants().unwrap();
            assert!(g.is_over(), "seed {seed} should finish by Turn 160 / Armageddon (ended at turn {}, AC {})", g.turn, g.apocalypse.counter);
        }
    }
}
