//! AI player for Cataclysm — plays 100% through `cataclysm_core::Command`, with zero AI-only rules.

mod diplomacy;
mod military;

use cataclysm_core::{
    Alignment, BuildOrder, Command, DistrictKind, Doctrine, Event, FactionId, Game, GreatPersonAction, HouseMood, PlayerId, Religion,
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
    diplomacy::manage_diplomacy(g, player, &mut events);
    manage_cities_and_caravans(g, player, &mut events);
    military::manage_squads(g, player, &mut events);
    military::manage_sallies(g, player, &mut events);
    military::manage_raiders(g, player, &mut events);

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
