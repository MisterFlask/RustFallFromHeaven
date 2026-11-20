//! `ffh-play stage persons|persons-direct DIR`: Houses (docs/PERSONS.md, "UI", part K; "The
//! third build": Houses without Persons of Note, on "The second build": Houses as vassal
//! civilizations): one realm that shows every state of the system, for the tests and the
//! screenshots.
//!
//! Made with the rules' own calls wherever they reach: the first House is founded by a grant
//! (`player_grant_city` to a new House, its leader drawn), the others by the rules' founding
//! (`player_found_house`, which part 2's splits use, a leader drawn the same way: a realm with a
//! House grants only to its Houses), Houses are granted cities by `player_grant_city` (each a
//! vassal civilization of its own), the capitulated civilization is born (`world::birth`) and
//! capitulates (`team_assign_vassal`), the tributes are the realm's default at each founding
//! (`player_set_house_tribute_default`: in force at once), states are told by the Houses' turn.
//! Set by hand only what no call reaches: a request's term and a House's rest (so a refusal falls
//! due now, as a lapse would 40 turns later), and the human's treasury (at least [`STAGE_GOLD`]).
//! The third build's part 2A ([`stage_ultimatums`]) sets by hand what it lists in `hand_set`: the last
//! refusals that leave two Houses Furious with their ultimatums due at the human's next turn (the
//! rules issue one at once), and the demand of the ultimatum the capitulated House's contract came
//! from (the rules' own acceptance carries it out). The courtships (the fourth build:
//! [`stage_courtships`]) are the rules' Court channels; set by hand there: the neighbour's favour
//! at House `pleased` and the gold of the human's gift (each listed in `hand_set`).

use std::path::{Path, PathBuf};

use ffh_core::game::Game;
use ffh_core::ffh::cards::world::{self, Birth, NewCity};
use ffh_core::ids::{ArmyId, CityId, HouseId, PlayerId, PlotIdx, UnitId};
use ffh_core::persons::{GrantTo, LoyaltyState};
use ffh_data::*;

use crate::json::Json;

/// The human's treasury at least (see the module doc).
const STAGE_GOLD: i32 = 400;

/// One House of the scene.
pub struct StagedHouse {
    /// The human's: `friendly` (two cities, tribute 0%), `pleased` (Pleased or Cautious, tribute
    /// 10%, a claimant of the `taken` Crown city), `annoyed` (a civilization that capitulated, with
    /// its own leader; under a contract: no tribute above 10%), `furious` (Furious, its ultimatum
    /// due at the human's next turn; two cities: one held by an army of the human, its seat
    /// contested by a smaller one and lost), `large` (five cities, one grant from the split's
    /// threshold; tribute 10%, Furious: its ultimatum due at the human's next turn too); the
    /// neighbour's: `neighbour` (Cautious or worse; the human courts it when the rules allow).
    pub role: &'static str,
    pub id: HouseId,
    /// Its label (`House Arne`, `Hippus`).
    pub label: String,
    /// Its own player.
    pub player: PlayerId,
    pub seat: Option<CityId>,
    pub cities: Vec<CityId>,
    pub state: LoyaltyState,
    pub tribute: i32,
}

/// A saved game at a human control point (the human the Bannor under Aristocracy): the capital,
/// three Crown cities (`taken`, claimed by House `pleased`, the one over the Crown's limit; `crown`
/// holds all three), the Houses of [`StagedHouse`], the armies by role (`armies`: `main`, an army of the human
/// for the levy; `holds_furious` in House `furious`'s second city; `contests_furious` in its seat,
/// outnumbered), and the neighbour civilization (`neighbour`, met, at peace) with two cities, one
/// (`neighbour_near`) within 12 plots of a House's city, the other its House's. `persons-direct` is
/// the same realm with Mysticism known: the civics screen can adopt God King at once (no anarchy,
/// no revolution timer).
pub struct StagedPersons {
    pub save: PathBuf,
    pub stage: &'static str,
    pub human: PlayerId,
    pub capital: CityId,
    pub crown: Vec<CityId>,
    pub taken: CityId,
    pub houses: Vec<StagedHouse>,
    pub armies: Vec<(&'static str, ArmyId)>,
    pub neighbour: PlayerId,
    pub neighbour_cities: Vec<CityId>,
    pub neighbour_near: CityId,
    /// What the stage set by hand beyond the list in the module's doc (empty so far).
    pub hand_set: Vec<String>,
    /// Display names, for the JSON.
    city_names: Vec<(CityId, String)>,
    army_names: Vec<(ArmyId, String)>,
    pub turn: i32,
}

impl StagedPersons {
    pub fn to_json(&self) -> Json {
        let city = |c: CityId| {
            let name = self.city_names.iter().find(|(id, _)| *id == c).map_or(String::new(), |(_, n)| n.clone());
            crate::jobj! { "id" => c, "name" => name }
        };
        let houses = Json::Obj(
            self.houses
                .iter()
                .map(|h| {
                    let j = crate::jobj! {
                        "id" => h.id, "label" => h.label.clone(), "player" => h.player, "seat" => h.seat.map(city).unwrap_or_default(),
                        "cities" => Json::Arr(h.cities.iter().map(|&c| city(c)).collect()), "state" => h.state.name(), "tribute" => h.tribute,
                    };
                    (h.role.to_string(), j)
                })
                .collect(),
        );
        let armies = Json::Obj(
            self.armies
                .iter()
                .map(|&(role, a)| {
                    let name = self.army_names.iter().find(|(id, _)| *id == a).map_or(String::new(), |(_, n)| n.clone());
                    (role.to_string(), crate::jobj! { "id" => a, "name" => name })
                })
                .collect(),
        );
        crate::jobj! {
            "stage" => self.stage, "save" => self.save.display().to_string(), "turn" => self.turn, "human" => self.human,
            "capital" => city(self.capital), "crown" => Json::Arr(self.crown.iter().map(|&c| city(c)).collect()), "taken" => city(self.taken),
            "houses" => houses, "armies" => armies,
            "neighbour" => crate::jobj! { "player" => self.neighbour, "cities" => Json::Arr(self.neighbour_cities.iter().map(|&c| city(c)).collect()), "near" => city(self.neighbour_near) },
            "hand_set" => self.hand_set.clone(),
        }
    }
}

/// The seeds tried, in order, until one's map has the room the scene needs.
const SEEDS: [u32; 8] = [11, 12, 13, 14, 15, 16, 17, 18];

/// Writes a [`StagedPersons`] into `dir` (a game directory): `persons`, or with `direct`
/// `persons-direct`.
pub fn persons(dir: &Path, direct: bool) -> Result<StagedPersons, String> {
    let mut last = String::new();
    for seed in SEEDS {
        match persons_seed(dir, seed, direct) {
            Ok(st) => return Ok(st),
            Err(e) => last = format!("seed {seed}: {e}"),
        }
    }
    Err(last)
}

/// A land plot on which a unit may be placed: not water or impassable, no unit, no city.
fn free(g: &Game, q: PlotIdx) -> bool {
    !g.plot_is_water(q) && !g.plot_is_impassable(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].plot_city.is_none()
}

/// The free land plot nearest (`x`, `y`) (rings out to 4; within a ring in a fixed order).
fn free_near(g: &Game, x: i32, y: i32) -> Result<PlotIdx, String> {
    for r in 1..=4i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                if let Some(q) = g.plot_xy(x, y, dx, dy) {
                    if free(g, q) {
                        return Ok(q);
                    }
                }
            }
        }
    }
    Err(format!("no free land plot near ({x}, {y})"))
}

/// `n` units of type `ty` of player `p` on plot `q`.
fn units_at(g: &mut Game, p: PlayerId, ty: UnitType, q: PlotIdx, n: usize) -> Vec<UnitId> {
    let (x, y) = g.map.xy(q);
    (0..n).map(|_| g.player_init_unit(p, ty, x, y, None, None)).collect()
}

/// Forces refusals of House `hh`'s requests until its state is `want`: its rest ends now (a
/// request is made by the Houses' turn), the request's term ends now, and the Houses' turn refuses
/// it (`Request refused: -3 (60 turns)`), as a lapse would.
fn refuse_until(g: &mut Game, h: PlayerId, hh: HouseId, want: LoyaltyState, who: &str) -> Result<(), String> {
    for _ in 0..6 {
        if g.house_state(hh) == want {
            return Ok(());
        }
        let turn = g.game_turn();
        if g.houses[hh].demand.is_none() {
            g.houses[hh].demand_rest_until = turn;
            g.player_persons_turn(h);
        }
        let Some(dm) = g.houses[hh].demand.as_mut() else { return Err(format!("{who}: no request to refuse")) };
        dm.deadline = turn;
        g.player_persons_turn(h);
    }
    if g.house_state(hh) == want {
        Ok(())
    } else {
        Err(format!("{who}: {} after the refusals, not {}", g.house_state(hh).name(), want.name()))
    }
}

/// The roles of the city sites around the capital.
struct Layout {
    taken: PlotIdx,
    crown: [PlotIdx; 2],
    a: [PlotIdx; 2],
    b: PlotIdx,
    c: [PlotIdx; 2],
    /// The capitulated civilization's capital.
    v: PlotIdx,
}

/// 11 city sites of player `h` (its capital's land first, nearest the capital first, 3 plots or
/// more from each other and from every city) and their roles: the taken Crown city within claim
/// range of a city of House `pleased` (b) and one of House `friendly` (a), and beyond claim range
/// of every city of House `furious` (c) and of the capitulated civilization (v), which are beyond
/// claim range of House `friendly`'s cities too (a grant to one claims nothing of the other).
fn layout(g: &Game, h: PlayerId) -> Result<Layout, String> {
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let area = g.map.plots[g.city_plot(capital)].area;
    let mut cand: Vec<((bool, i32, PlotIdx), PlotIdx)> = Vec::new();
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if g.plot_is_water(q) || g.plot_is_impassable(q) || g.map.plots[q].plot_city.is_some() || !g.player_can_found(h, x, y, false) {
            continue;
        }
        cand.push(((g.map.plots[q].area != area, g.plot_distance(x, y, cx, cy), q), q));
    }
    cand.sort();
    let cities: Vec<(i32, i32)> = g.cities.iter().map(|(_, c)| (c.x, c.y)).collect();
    let mut sites: Vec<PlotIdx> = Vec::new();
    for (_, q) in cand {
        let (x, y) = g.map.xy(q);
        let dist = |&(ox, oy): &(i32, i32)| g.plot_distance(x, y, ox, oy);
        if cities.iter().any(|c| dist(c) < 3) || sites.iter().any(|&s| dist(&g.map.xy(s)) < 3) {
            continue;
        }
        sites.push(q);
        if sites.len() == 14 {
            break;
        }
    }
    if sites.len() < 10 {
        return Err(format!("only {} city sites", sites.len()));
    }
    let range = g.d.defines.person_claim_range;
    let pd = |a: PlotIdx, b: PlotIdx| {
        let ((ax, ay), (bx, by)) = (g.map.xy(a), g.map.xy(b));
        g.plot_distance(ax, ay, bx, by)
    };
    for &taken in &sites {
        let mut near: Vec<PlotIdx> = sites.iter().copied().filter(|&s| s != taken && pd(s, taken) <= range).collect();
        if near.len() < 2 {
            continue;
        }
        near.sort_by_key(|&s| (pd(s, taken), s));
        let (b, a1) = (near[0], near[1]);
        let mut rest: Vec<PlotIdx> = sites.iter().copied().filter(|s| ![taken, a1, b].contains(s)).collect();
        rest.sort_by_key(|&s| (pd(s, a1), s));
        let Some(a2) = rest.first().copied() else { continue };
        let mut far: Vec<PlotIdx> = sites.iter().copied().filter(|&s| [taken, a1, a2, b].iter().all(|&o| pd(s, o) > range)).collect();
        if far.len() < 3 {
            continue;
        }
        far.sort_by_key(|&s| (pd(s, taken), s));
        let (c1, c2, v) = (far[0], far[1], far[2]);
        let crown: Vec<PlotIdx> = sites.iter().copied().filter(|s| ![taken, a1, a2, b, c1, c2, v].contains(s)).take(2).collect();
        if crown.len() < 2 {
            continue;
        }
        return Ok(Layout { taken, crown: [crown[0], crown[1]], a: [a1, a2], b, c: [c1, c2], v });
    }
    Err("no site for the taken city near two Houses' cities".into())
}

/// The third build, part 2A (docs/PERSONS.md, rules 4 to 6), added at the end of the stage builder:
/// - House `furious` (Annoyed by the rules' refusals) takes one more `Request refused: -3 (60
///   turns)` by hand: Furious, with no contract, its ultimatum (its refused request again when it
///   can be met, else the rules' next demand) comes at the human's next turn, a blocking decision;
/// - House `annoyed` (the capitulated civilization) is under a contract, by the rules' acceptance
///   of an ultimatum whose demand is set by hand (`Tribute(10)`: no tribute above 10% for 20
///   turns; one ultimatum counted);
/// - a new House `large`: five cities the human founds, beyond claim range of the taken city, and
///   the rules' founding with all five (a split's call, `player_found_house_as`), at a realm's
///   default of 10%; one grant from the split's threshold of 6; two refusals by hand make it
///   Furious, so its ultimatum (not a refused request, no lower tribute below 20%, no memory: a
///   resource or gold a turn) is due at the human's next turn too.
fn stage_ultimatums(g: &mut Game, h: PlayerId, house_c: HouseId, house_v: HouseId, taken: CityId, hand_set: &mut Vec<String>) -> Result<HouseId, String> {
    use ffh_core::persons::{LoyaltyCause, LoyaltyMod, Ultimatum, UltimatumKind, DEMAND_REFUSED};
    let turn = g.game_turn();
    let refusal = |g: &mut Game, hh: HouseId| g.houses[hh].standing.mods.push(LoyaltyMod { cause: LoyaltyCause::DemandRefused, amount: DEMAND_REFUSED.0, until: Some(turn + DEMAND_REFUSED.1) });

    // `annoyed`: a contract, the rules' acceptance of a tribute ultimatum.
    let u = Ultimatum { house: house_v, kind: UltimatumKind::Tribute(10), request: false, until: turn + g.d.defines.person_ultimatum_turns };
    if g.house_ultimatum_accept(&u) != u.kind {
        return Err("House annoyed's contract is not the tribute's".into());
    }
    g.houses[house_v].ultimatums = 1;
    hand_set.push("House annoyed: the demand of the ultimatum its contract came from (its tribute to 10%), accepted by the rules".into());

    // `large`: five cities of the human's, then the founding with them.
    let range = g.d.defines.person_claim_range;
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let (tx, ty) = (g.cities[taken].x, g.cities[taken].y);
    let mut sites: Vec<(i32, PlotIdx)> = Vec::new();
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if g.plot_is_water(q) || g.plot_is_impassable(q) || g.plot_distance(x, y, tx, ty) <= range || !g.player_can_found(h, x, y, false) {
            continue;
        }
        sites.push((g.plot_distance(x, y, cx, cy), q));
    }
    sites.sort();
    // Founded one by one, each where the rules let the human found it with the others standing.
    let mut cities: Vec<CityId> = Vec::new();
    for (_, q) in sites {
        let (x, y) = g.map.xy(q);
        if cities.len() == 5 || !g.player_can_found(h, x, y, false) {
            continue;
        }
        cities.push(g.player_init_city(h, x, y, true, true));
    }
    if cities.len() < 5 {
        return Err(format!("only {} sites for House large", cities.len()));
    }
    let civ = g.player_get_civilization_type(h).ok_or("no civilization")?;
    let leader = g.house_leader_draw(civ, h).ok_or("no leader is free for House large")?;
    let default = g.players[h].house_tribute;
    g.player_set_house_tribute_default(h, 10).map_err(|e| format!("tribute: {e}"))?;
    let (house_l, _) = g.player_found_house_as(h, civ, leader, &cities, ffh_core::persons::HouseOrigin::Founded).ok_or("House large cannot be founded")?;
    g.player_set_house_tribute_default(h, default).map_err(|e| format!("tribute: {e}"))?;
    let all: Vec<CityId> = g.house_cities(house_l);
    for c in all {
        if g.city_is_governed(c) && g.cities[c].order_queue.is_empty() && !g.cities[c].settlement {
            g.city_choose_production(c, None, None, None, false, false);
        }
    }
    // Told as it is now (no change to tell at the next turn but the one set below).
    let state = g.house_state(house_l);
    g.houses[house_l].standing.state = state;

    // The last refusals: Furious, the ultimatums due at the human's next turn.
    refusal(g, house_c);
    hand_set.push("House furious: its last `Request refused` (Furious: its ultimatum at the next turn)".into());
    while g.house_state(house_l) != LoyaltyState::Furious {
        refusal(g, house_l);
    }
    hand_set.push("House large: two `Request refused` (Furious: its ultimatum at the next turn)".into());
    if g.house_great_cities(house_l) != 5 {
        return Err(format!("House large holds {} great cities", g.house_great_cities(house_l)));
    }
    if g.house_contract(house_v).is_none() || g.house_state(house_v) != LoyaltyState::Annoyed {
        return Err("House annoyed is not under a contract, Annoyed".into());
    }
    Ok(house_l)
}

/// A civilization in no one's hands and its first leader: the capitulated civilization's.
fn free_civ(g: &Game) -> Option<(CivilizationType, LeaderHeadType)> {
    let d = g.d;
    let taken: Vec<CivilizationType> = g.player_ids().filter_map(|p| g.player_get_civilization_type(p)).collect();
    d.civilizations.keys().filter(|&c| d.civilizations[c].playable && !taken.contains(&c)).find_map(|c| {
        d.leader_heads.keys().find(|&l| d.civilizations[c].leaders[l] && !ffh_core::ffh::cards::sundering::leader_in_play(g, l)).map(|l| (c, l))
    })
}

/// The fourth build (docs/PERSONS.md, rules 1 to 5): the courtships of the scene. The neighbour
/// `ai` courts the human's House `pleased` with its Court channel (the rules' own call), its
/// favour set by hand to two thirds of the requirement (a courtship that needs attention); the
/// human courts the neighbour's House `neighbour` with its own Court channel when the rules allow
/// it, and gives that House a gift of gold (an ordinary gift: its leader's attitude toward the
/// human). Set by hand, and listed in `hand_set`: the favour, and the gold of the gift.
fn stage_courtships(g: &mut Game, h: PlayerId, ai: PlayerId, pleased: HouseId, neighbour: HouseId, _direct: bool, hand_set: &mut Vec<String>) -> Result<(), String> {
    use ffh_core::culture::{ChannelOrder, ChannelTarget};
    use ffh_core::structs::TradeData;
    let court = g.culture_action(CultureEffect::Court).ok_or("no Court action in the data")?;
    let order = |h: HouseId| ChannelOrder { action: court, target: ChannelTarget::House(h), religion: None };
    // The neighbour courts House `pleased`.
    g.player_set_culture_channel(ai, Some(order(pleased))).map_err(|e| format!("the neighbour cannot court House pleased: {e}"))?;
    let need = 100 * g.house_requirement(pleased, ai).total;
    g.houses[pleased].favour = vec![ffh_core::persons::Favour { courter: ai, amount: need * 2 / 3 + 1, warned: false }];
    hand_set.push("House pleased: the neighbour's favour at two thirds of its requirement".into());
    // The human courts the neighbour's House, when the rules allow it, and gives it gold.
    if let Err(e) = g.player_set_culture_channel(h, Some(order(neighbour))) {
        hand_set.push(format!("the human's Court of the neighbour's House refused: {e}"));
    }
    let gold = 50;
    g.players[h].gold += gold;
    hand_set.push(format!("{gold} gold added to the human's treasury for a gift"));
    let hp = g.houses[neighbour].player;
    if !g.player_offer_house(h, hp, &[TradeData { item_type: TradeableItem::TRADE_GOLD, data: gold, offering: false, hidden: false }], &[]) {
        return Err("the human's gift to the neighbour's House refused".into());
    }
    Ok(())
}

fn persons_seed(dir: &Path, seed: u32, direct: bool) -> Result<StagedPersons, String> {
    let d = ffh_data::global();
    let stage: &'static str = if direct { "persons-direct" } else { "persons" };
    let mut s = super::duel_with(dir, seed, 3, Some("CIVILIZATION_BANNOR"), &[])?;
    let h = s.human;
    let team = s.g.players[h].team;
    let mut hand_set: Vec<String> = Vec::new();
    let ai = s.g.civ_player_ids().find(|&p| p != h && s.g.players[p].alive && !s.g.player_is_barbarian(p)).ok_or("no neighbour")?;
    let ai_civ = s.g.player_get_civilization_type(ai).ok_or("the neighbour has no civilization")?;
    if s.g.house_leader_pool(ai_civ, ai).leaders.is_empty() {
        return Err("no leader is free for the neighbour's House".into());
    }

    // The government: Aristocracy (its tech known) for the human and the neighbour; God King's
    // tech too for persons-direct.
    {
        let g = &mut s.g;
        let aristocracy = d.t::<CivicType>("CIVIC_ARISTOCRACY");
        let option = d.civics[aristocracy].civic_option_type.ok_or("no civic option")?;
        for (p, techs) in [(h, if direct { vec!["TECH_CODE_OF_LAWS", "TECH_MYSTICISM"] } else { vec!["TECH_CODE_OF_LAWS"] }), (ai, vec!["TECH_CODE_OF_LAWS"])] {
            let t = g.players[p].team;
            for tech in techs {
                g.team_set_has_tech(t, d.t::<TechType>(tech), true, Some(p), false, false);
            }
            g.player_set_civics(p, option, Some(aristocracy));
            g.player_check_grants(p).map_err(|e| format!("grants: {e}"))?;
        }
        let their = g.players[ai].team;
        g.team_meet(team, their, false);
    }

    // The cities: the human's (Crown for now; `taken` last of the Crown's, so it is the one over
    // the limit).
    let lay = layout(&s.g, h)?;
    let capital = s.g.player_get_capital_city(h).ok_or("no capital")?;
    let found = |g: &mut Game, p: PlayerId, q: PlotIdx| {
        let (x, y) = g.map.xy(q);
        g.player_init_city(p, x, y, true, true)
    };
    let (taken, crown_y, crown_z, a_cities, b_city, c_cities) = {
        let g = &mut s.g;
        let y = found(g, h, lay.crown[0]);
        let z = found(g, h, lay.crown[1]);
        let taken = found(g, h, lay.taken);
        let a: Vec<CityId> = lay.a.iter().map(|&q| found(g, h, q)).collect();
        let b = found(g, h, lay.b);
        let c: Vec<CityId> = lay.c.iter().map(|&q| found(g, h, q)).collect();
        (taken, y, z, a, b, c)
    };

    // The neighbour's second city: within 12 plots of a House's city, 4 or more from the human's.
    let (neighbour_cities, neighbour_near) = {
        let g = &mut s.g;
        let house_cities: Vec<CityId> = a_cities.iter().chain(std::iter::once(&b_city)).chain(&c_cities).copied().collect();
        let human_cities: Vec<CityId> = g.players[h].cities.clone();
        let (vx, vy) = g.map.xy(lay.v);
        let mut best: Option<((i32, PlotIdx), PlotIdx)> = None;
        for q in g.map.plot_ids() {
            let (x, y) = g.map.xy(q);
            if g.plot_is_water(q) || g.plot_is_impassable(q) || !g.player_can_found(ai, x, y, false) {
                continue;
            }
            if human_cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) < 4) || g.plot_distance(x, y, vx, vy) < 4 {
                continue;
            }
            let near = house_cities.iter().map(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y)).min().unwrap_or(i32::MAX);
            if near <= ffh_core::persons::LAND_RANGE && best.is_none_or(|(k, _)| (near, q) < k) {
                best = Some(((near, q), q));
            }
        }
        let (_, q) = best.ok_or("no site for the neighbour's city within 12 plots of a House's city")?;
        let near = found(g, ai, q);
        let (x, y) = g.map.xy(q);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(r) = g.plot_xy(x, y, dx, dy) {
                    g.plot_set_revealed(r, team, true, false, None, true);
                }
            }
        }
        if g.players[ai].cities.len() < 2 {
            return Err("the neighbour has no other city".into());
        }
        (g.players[ai].cities.iter().copied().take(2).collect::<Vec<_>>(), near)
    };
    // (By plot: a city granted to a House changes id.)
    let neighbour_plots: Vec<PlotIdx> = neighbour_cities.iter().map(|&c| s.g.city_plot(c)).collect();
    let near_plot = s.g.city_plot(neighbour_near);

    // The human's army for the levy, by House A's first city.
    let ty = |n: &str| d.t::<UnitType>(n);
    let army_main = {
        let g = &mut s.g;
        let (x, y) = (g.cities[a_cities[0]].x, g.cities[a_cities[0]].y);
        let q = free_near(g, x, y)?;
        let units = units_at(g, h, ty("UNIT_WARRIOR"), q, 3);
        let a = g.army_form(h, &units)?;
        g.player_army_sync(h);
        a
    };

    // The Houses, each a vassal civilization: `friendly` founded by the realm's first grant (its
    // leader drawn), `furious` and `pleased` by the rules' founding (a leader drawn the same way),
    // granted in this order so that the claimant House `pleased` is passed over by no grant.
    let (house_a, house_b, house_c) = {
        let g = &mut s.g;
        let grant = |g: &mut Game, p: PlayerId, c: CityId, to: GrantTo| g.player_grant_city(p, c, to).map_err(|e| format!("grant of {}: {e}", g.cities[c].name));
        let civ = g.player_get_civilization_type(h).ok_or("no civilization")?;
        let found_house = |g: &mut Game, c: CityId| -> Result<HouseId, String> {
            let leader = g.house_leader_draw(civ, h).ok_or("no leader is free")?;
            g.player_found_house(h, leader, &[c]).map(|(hh, _)| hh).ok_or_else(|| "the House cannot be founded".to_string())
        };
        // The tributes: House `friendly` 0%, House `pleased` 10% (the others at the realm's 20%),
        // each the realm's default when it is founded, so in force at once (rule 4).
        let default = |g: &mut Game, pct: i32| g.player_set_house_tribute_default(h, pct).map_err(|e| format!("tribute: {e}"));
        default(g, 0)?;
        let ha = grant(g, h, a_cities[0], GrantTo::NewHouse)?;
        let a2 = g.map.plots[lay.a[1]].plot_city.ok_or("House A's second city")?;
        grant(g, h, a2, GrantTo::House(ha))?;
        default(g, ffh_core::persons::TRIBUTE_DEFAULT)?;
        let hc = found_house(g, c_cities[0])?;
        let c2 = g.map.plots[lay.c[1]].plot_city.ok_or("House C's second city")?;
        grant(g, h, c2, GrantTo::House(hc))?;
        default(g, 10)?;
        let hb = found_house(g, b_city)?;
        default(g, ffh_core::persons::TRIBUTE_DEFAULT)?;
        (ha, hb, hc)
    };

    // The capitulated civilization: born with a city of its own, then it capitulates to the human.
    let house_v = {
        let g = &mut s.g;
        let (civ, leader) = free_civ(g).ok_or("no civilization left for the capitulated one")?;
        let mut b = Birth::wild(civ, leader);
        b.found = vec![NewCity { plot: lay.v, buildings: Vec::new(), units: Vec::new() }];
        let vp = world::birth(g, &b);
        let vt = g.players[vp].team;
        g.team_meet(team, vt, false);
        g.team_assign_vassal(team, vt, true);
        g.player_house(vp).ok_or("the capitulated civilization is no House")?
    };

    // The neighbour's own House (its first grant founds it): Cautious or worse.
    let house_n = {
        let g = &mut s.g;
        let c = neighbour_cities.iter().copied().find(|&c| !g.city_is_capital(c)).ok_or("the neighbour's second city")?;
        g.player_grant_city(ai, c, GrantTo::NewHouse).map_err(|e| format!("the neighbour's grant: {e}"))?
    };

    // States: House `furious` Annoyed, by refused requests (the last refusal, which makes it
    // Furious, is set by `stage_ultimatums`: a Furious House issues its ultimatum at once).
    {
        let g = &mut s.g;
        g.player_persons_turn(h);
        refuse_until(g, h, house_c, LoyaltyState::Annoyed, "House C")?;
    }

    // The last Houses' turn (states told).
    {
        let g = &mut s.g;
        // The cities founded this turn choose their production now, as the end of the turn would
        // (a governed city never stands without one).
        let all: Vec<CityId> = g.cities.iter().map(|(c, _)| c).collect();
        for c in all {
            if g.city_is_governed(c) && g.cities[c].order_queue.is_empty() && !g.cities[c].settlement {
                g.city_choose_production(c, None, None, None, false, false);
            }
        }
        g.player_army_sync(h);
        g.player_persons_turn(h);
        g.player_persons_turn(ai);
    }

    // An army of the human in House C's second city (formed last: it holds the city).
    let army_c = {
        let g = &mut s.g;
        let c2 = g.house_cities(house_c).into_iter().find(|&c| g.city_plot(c) == lay.c[1]).ok_or("House C's second city")?;
        let (ccx, ccy) = (g.cities[c2].x, g.cities[c2].y);
        let uc: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(h, ty("UNIT_WARRIOR"), ccx, ccy, None, None)).collect();
        let a = g.army_form(h, &uc)?;
        g.player_army_sync(h);
        if g.army_units(a).len() != 2 {
            return Err("the army in House C's city is not whole".into());
        }
        a
    };

    // House C's seat contested and lost (rule 7: a tie or fewer goes to the House): two warriors
    // of the House against an army of one of the human's.
    let army_seat = {
        let g = &mut s.g;
        let c1 = g.house_cities(house_c).into_iter().find(|&c| g.city_plot(c) == lay.c[0]).ok_or("House C's seat")?;
        let (cx, cy) = (g.cities[c1].x, g.cities[c1].y);
        let hp = g.houses[house_c].player;
        for _ in 0..2 {
            g.player_init_unit(hp, ty("UNIT_WARRIOR"), cx, cy, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        }
        let u = g.player_init_unit(h, ty("UNIT_WARRIOR"), cx, cy, None, None);
        let a = g.army_form(h, &[u])?;
        g.player_army_sync(h);
        a
    };

    // The human's treasury: at least STAGE_GOLD, set by hand.
    if s.g.players[h].gold < STAGE_GOLD {
        s.g.players[h].gold = STAGE_GOLD;
    }

    // The third build, part 2A: the ultimatums due, a contract, a House one grant from a split.
    let house_l = stage_ultimatums(&mut s.g, h, house_c, house_v, taken, &mut hand_set)?;
    // The fourth build: the courtships (rules 1 to 5).
    stage_courtships(&mut s.g, h, ai, house_b, house_n, direct, &mut hand_set)?;

    // Checks: what the scene promises.
    {
        let g = &s.g;
        let want = |what: &str, got: LoyaltyState, ok: &[LoyaltyState]| if ok.contains(&got) { Ok(()) } else { Err(format!("{what} is {}, not {ok:?}", got.name())) };
        want("House friendly", g.house_state(house_a), &[LoyaltyState::Friendly])?;
        want("House pleased", g.house_state(house_b), &[LoyaltyState::Pleased, LoyaltyState::Cautious])?;
        want("House annoyed", g.house_state(house_v), &[LoyaltyState::Annoyed])?;
        want("House furious", g.house_state(house_c), &[LoyaltyState::Furious])?;
        want("House large", g.house_state(house_l), &[LoyaltyState::Furious])?;
        want("the neighbour's House", g.house_state(house_n), &[LoyaltyState::Cautious, LoyaltyState::Annoyed, LoyaltyState::Furious])?;
        if !g.city_claimants(taken).contains(&house_b) {
            return Err(format!("the taken city's claimants are {:?}", g.city_claimants(taken)));
        }
        if g.player_crown_over(h) != vec![taken] {
            return Err(format!("the Crown's city over its limit is not the taken one: {:?}", g.player_crown_over(h)));
        }
        if g.house_cities_held(house_c).1.len() != 1 {
            return Err("House C's city is not held by an army".into());
        }
        let seat_hold = g.house_cities(house_c).into_iter().find(|&c| g.city_plot(c) == lay.c[0]).and_then(|c| g.city_hold(c));
        if seat_hold.is_none_or(|hold| hold.held() || hold.liege == 0) {
            return Err(format!("House C's seat is not contested and lost: {seat_hold:?}"));
        }
        // The neighbour's courtship of House pleased, past two thirds.
        if !g.house_courtships(house_b).iter().any(|cv| cv.courter == ai && cv.attention) {
            return Err("the neighbour's courtship of House pleased is not past two thirds".into());
        }
        if direct {
            let god_king = d.t::<CivicType>("CIVIC_GOD_KING");
            if !g.player_can_do_civics(h, Some(god_king)) || !g.player_can_revolution(h, None) {
                return Err("God King cannot be adopted now".into());
            }
        }
    }

    let g = &s.g;
    let staged = |role: &'static str, id: HouseId| StagedHouse {
        role,
        id,
        label: g.house_label(id),
        player: g.houses[id].player,
        seat: g.house_seat(id),
        cities: g.house_cities(id),
        state: g.house_state(id),
        tribute: g.houses[id].tribute,
    };
    let mut houses = vec![staged("friendly", house_a), staged("pleased", house_b), staged("annoyed", house_v), staged("furious", house_c), staged("neighbour", house_n)];
    houses.push(staged("large", house_l));
    let armies: Vec<(&'static str, ArmyId)> = vec![("main", army_main), ("holds_furious", army_c), ("contests_furious", army_seat)];
    let mut named: Vec<CityId> = g.players[h].cities.iter().chain(&g.players[ai].cities).copied().collect();
    for hs in &houses {
        named.extend(hs.cities.iter().copied());
    }
    let city_names = named.iter().map(|&c| (c, d.text.name(&g.cities[c].name).to_string())).collect();
    let army_names = armies.iter().map(|&(_, a)| (a, g.armies[a].name.clone())).collect();
    let crown = vec![taken, crown_y, crown_z];
    let neighbour_cities: Vec<CityId> = neighbour_plots.iter().filter_map(|&q| g.map.plots[q].plot_city).collect();
    let neighbour_near = g.map.plots[near_plot].plot_city.ok_or("the neighbour's near city")?;
    let turn = g.game_turn();
    let paths = s.save(stage).map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedPersons {
        save: paths.save,
        stage,
        human: h,
        capital,
        crown,
        taken,
        houses,
        armies,
        neighbour: ai,
        neighbour_cities,
        neighbour_near,
        hand_set,
        city_names,
        army_names,
        turn,
    })
}
