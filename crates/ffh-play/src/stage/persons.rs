//! `ffh-play stage persons|persons-direct DIR`: Persons of Note (docs/PERSONS.md, "UI", part K;
//! "The second build": Houses as vassal civilizations): one realm that shows every state of the
//! system, for the tests and the screenshots.
//!
//! Made with the rules' own calls wherever they reach: the cast enters by the persons turn
//! (`player_persons_turn`: heroes, then a veteran a turn), Houses are founded and granted cities by
//! `player_grant_city` (each a vassal civilization of its own), the capitulated civilization is
//! born (`world::birth`) and capitulates (`team_assign_vassal`), the tributes are set by
//! `player_set_house_tribute`, the commander's claim comes from a real capture (the army moves
//! into a barbarian city), states are told by the persons turn. Set by hand only what no call
//! reaches: a unit's level (the veterans' bar), a demand's term and a person's rest (so a refusal
//! falls due now, as a lapse would 40 turns later), the countdowns' turns left, and, only when
//! the draw gives no such leader, the ambition a role needs (said in the stage's JSON,
//! `hand_set`).

use std::path::{Path, PathBuf};

use ffh_core::game::Game;
use ffh_core::ffh::cards::world::{self, Birth, NewCity};
use ffh_core::ids::{ArmyId, CityId, HouseId, PersonId, PlayerId, PlotIdx, UnitId};
use ffh_core::persons::{Ambition, Claimant, GrantTo, LoyaltyState, Post};
use ffh_data::*;

use crate::command::{CmdOk, Command};
use crate::json::Json;

/// One House of the scene.
pub struct StagedHouse {
    /// The human's: `friendly` (two cities, tribute 0%), `pleased` (Pleased or Cautious, tribute
    /// 10%, a claimant of the taken city), `annoyed` (a civilization that capitulated, with its own
    /// leader), `furious` (countdown 8, two cities, one held by an army of the human); the
    /// neighbour's: `neighbour` (Cautious or worse: a bribe can be offered).
    pub role: &'static str,
    pub id: HouseId,
    /// Its label (`House Arne`, `Hippus`).
    pub label: String,
    /// Its own player.
    pub player: PlayerId,
    pub head: Option<PersonId>,
    pub seat: Option<CityId>,
    pub cities: Vec<CityId>,
    pub state: LoyaltyState,
    pub tribute: i32,
}

/// A saved game at a human control point (the human the Bannor under Aristocracy): the capital,
/// three Crown cities (`taken`, claimed by House `pleased` and by the commander whose army took it
/// this turn; `crown` holds all three), the Houses of [`StagedHouse`], the persons by role
/// (`persons`), the armies by role (`armies`), a Great Merchant (`great_person`) in the capital
/// that can found a House (a leader is left in the pool), and the neighbour civilization
/// (`neighbour`, met, at peace) with two cities, one (`neighbour_near`) within 12 plots of a
/// House's city, the other its House's. `persons-direct` is the same realm with Mysticism known:
/// the civics screen can adopt God King at once (no anarchy, no revolution timer).
pub struct StagedPersons {
    pub save: PathBuf,
    pub stage: &'static str,
    pub human: PlayerId,
    pub capital: CityId,
    pub crown: Vec<CityId>,
    pub taken: CityId,
    pub houses: Vec<StagedHouse>,
    pub persons: Vec<(&'static str, PersonId)>,
    pub armies: Vec<(&'static str, ArmyId)>,
    pub great_person: UnitId,
    pub neighbour: PlayerId,
    pub neighbour_cities: Vec<CityId>,
    pub neighbour_near: CityId,
    /// What the stage set by hand beyond the list in the module's doc (empty when the draw gave
    /// every role its ambition).
    pub hand_set: Vec<String>,
    /// Display names, for the JSON.
    city_names: Vec<(CityId, String)>,
    person_names: Vec<(PersonId, String, Option<UnitId>)>,
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
                        "id" => h.id, "label" => h.label.clone(), "player" => h.player, "head" => h.head, "seat" => h.seat.map(city).unwrap_or_default(),
                        "cities" => Json::Arr(h.cities.iter().map(|&c| city(c)).collect()), "state" => h.state.name(), "tribute" => h.tribute,
                    };
                    (h.role.to_string(), j)
                })
                .collect(),
        );
        let persons = Json::Obj(
            self.persons
                .iter()
                .map(|&(role, x)| {
                    let (name, unit) = self.person_names.iter().find(|(id, _, _)| *id == x).map_or((String::new(), None), |(_, n, u)| (n.clone(), *u));
                    (role.to_string(), crate::jobj! { "id" => x, "name" => name, "unit" => unit })
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
            "houses" => houses, "persons" => persons, "armies" => armies, "great_person" => self.great_person,
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

/// Forces refusals of person `x`'s demands until their state is `want`: their rest ends now (a
/// demand is made by the persons turn), the demand's term ends now, and the persons turn refuses
/// it (`Demand refused: -3 (60 turns)`), as a lapse would.
fn refuse_until(g: &mut Game, h: PlayerId, x: PersonId, want: LoyaltyState, who: &str) -> Result<(), String> {
    for _ in 0..6 {
        if g.person_state(x) == want {
            return Ok(());
        }
        let turn = g.game_turn();
        if g.persons[x].demand.is_none() {
            g.persons[x].demand_rest_until = turn;
            g.player_persons_turn(h);
        }
        let Some(dm) = g.persons[x].demand.as_mut() else { return Err(format!("{who}: no demand to refuse")) };
        dm.deadline = turn;
        g.player_persons_turn(h);
    }
    if g.person_state(x) == want {
        Ok(())
    } else {
        Err(format!("{who}: {} after the refusals, not {}", g.person_state(x).name(), want.name()))
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
/// range of a city of House `pleased` (b) and one of House `friendly` (a), and beyond it from every
/// city of House `furious` (c) and of the capitulated civilization (v).
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
        if sites.len() == 11 {
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
        let mut far: Vec<PlotIdx> = sites.iter().copied().filter(|&s| pd(s, taken) > range).collect();
        if near.len() < 2 || far.len() < 3 {
            continue;
        }
        near.sort_by_key(|&s| (pd(s, taken), s));
        far.sort_by_key(|&s| (pd(s, taken), s));
        let (b, a1) = (near[0], near[1]);
        let (c1, c2, v) = (far[0], far[1], far[2]);
        let mut rest: Vec<PlotIdx> = sites.iter().copied().filter(|s| ![taken, a1, b, c1, c2, v].contains(s)).collect();
        if rest.len() < 3 {
            continue;
        }
        rest.sort_by_key(|&s| (pd(s, a1), s));
        let a2 = rest.remove(0);
        return Ok(Layout { taken, crown: [rest[0], rest[1]], a: [a1, a2], b, c: [c1, c2], v });
    }
    Err("no site for the taken city near two Houses' cities".into())
}

/// A civilization in no one's hands and its first leader: the capitulated civilization's.
fn free_civ(g: &Game) -> Option<(CivilizationType, LeaderHeadType)> {
    let d = g.d;
    let taken: Vec<CivilizationType> = g.player_ids().filter_map(|p| g.player_get_civilization_type(p)).collect();
    d.civilizations.keys().filter(|&c| d.civilizations[c].playable && !taken.contains(&c)).find_map(|c| {
        d.leader_heads.keys().find(|&l| d.civilizations[c].leaders[l] && !ffh_core::ffh::cards::sundering::leader_in_play(g, l)).map(|l| (c, l))
    })
}

fn persons_seed(dir: &Path, seed: u32, direct: bool) -> Result<StagedPersons, String> {
    let d = ffh_data::global();
    let stage: &'static str = if direct { "persons-direct" } else { "persons" };
    let mut s = super::duel_with(dir, seed, 3, Some("CIVILIZATION_BANNOR"), &[])?;
    let h = s.human;
    let team = s.g.players[h].team;
    let mut hand_set: Vec<String> = Vec::new();
    let ai = s.g.civ_player_ids().find(|&p| p != h && s.g.players[p].alive && !s.g.player_is_barbarian(p)).ok_or("no neighbour")?;
    if s.g.persons_pool(ai).is_empty() {
        return Err("the neighbour's civilization has no persons of note".into());
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

    // The cities: the human's (Crown for now) and the barbarian city the army will take.
    let lay = layout(&s.g, h)?;
    let capital = s.g.player_get_capital_city(h).ok_or("no capital")?;
    let found = |g: &mut Game, p: PlayerId, q: PlotIdx| {
        let (x, y) = g.map.xy(q);
        g.player_init_city(p, x, y, true, true)
    };
    let (crown_y, crown_z, a_cities, b_city, c_cities, barb_city) = {
        let g = &mut s.g;
        let y = found(g, h, lay.crown[0]);
        let z = found(g, h, lay.crown[1]);
        let a: Vec<CityId> = lay.a.iter().map(|&q| found(g, h, q)).collect();
        let b = found(g, h, lay.b);
        let c: Vec<CityId> = lay.c.iter().map(|&q| found(g, h, q)).collect();
        let barb = g.barbarian_player();
        let bc = found(g, barb, lay.taken);
        g.city_set_population(bc, 3);
        for u in g.map.plots[lay.taken].units.clone() {
            g.unit_kill(u, false, None);
        }
        for k in 0..g.city_sentinel_count(bc) {
            g.city_set_sentinel_damage(bc, k, 100);
        }
        (y, z, a, b, c, bc)
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
            if human_cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) < 4)
                || g.plot_distance(x, y, g.map.xy(lay.taken).0, g.map.xy(lay.taken).1) < 4
                || g.plot_distance(x, y, vx, vy) < 4
            {
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

    // The units: two armies each led by a level 7 Axeman (the veterans to come), the army with
    // Donal Lugh next to the barbarian city, a hero in the capital (a person without a post).
    let ty = |n: &str| d.t::<UnitType>(n);
    let (vet1, vet2, army1, army2, army3, donal, rantine) = {
        let g = &mut s.g;
        let (kx, ky) = (g.cities[capital].x, g.cities[capital].y);
        let place = |g: &mut Game, c: CityId| {
            let (x, y) = (g.cities[c].x, g.cities[c].y);
            free_near(g, x, y)
        };
        let q1 = place(g, a_cities[0])?;
        let vet1 = g.player_init_unit(h, ty("UNIT_AXEMAN"), g.map.xy(q1).0, g.map.xy(q1).1, None, None);
        g.unit_set_level(vet1, 7);
        let mut u1 = vec![vet1];
        u1.extend(units_at(g, h, ty("UNIT_WARRIOR"), q1, 2));
        let army1 = g.army_form(h, &u1)?;
        let q2 = place(g, crown_y)?;
        let vet2 = g.player_init_unit(h, ty("UNIT_AXEMAN"), g.map.xy(q2).0, g.map.xy(q2).1, None, None);
        g.unit_set_level(vet2, 7);
        let mut u2 = vec![vet2];
        u2.extend(units_at(g, h, ty("UNIT_WARRIOR"), q2, 2));
        let army2 = g.army_form(h, &u2)?;
        // Next to the barbarian city, on a plot from which the army can move in.
        let (bx, by) = (g.cities[barb_city].x, g.cities[barb_city].y);
        let mut a3 = None;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
            let Some(q) = g.plot_xy(bx, by, dx, dy) else { continue };
            if !free(g, q) {
                continue;
            }
            let mut units = units_at(g, h, ty("UNIT_AXEMAN"), q, 2);
            units.extend(units_at(g, h, ty("UNIT_WARRIOR"), q, 1));
            if units.iter().all(|&u| g.unit_can_move_or_attack_into(u, lay.taken, false)) {
                a3 = Some((q, units));
                break;
            }
            for u in units {
                g.unit_kill(u, false, None);
            }
        }
        let (q3, u3) = a3.ok_or("no plot to take the barbarian city from")?;
        let army3 = g.army_form(h, &u3)?;
        let donal = g.player_init_unit(h, ty("UNIT_DONAL"), g.map.xy(q3).0, g.map.xy(q3).1, None, None);
        let grp = g.armies[army3].group;
        g.unit_join_group(donal, grp, false);
        let rantine = g.player_init_unit(h, ty("UNIT_RANTINE"), kx, ky, None, None);
        g.player_army_sync(h);
        (vet1, vet2, army1, army2, army3, donal, rantine)
    };

    // The cast enters: the heroes at once, a veteran a turn (two persons turns).
    {
        let g = &mut s.g;
        g.player_persons_turn(h);
        g.player_persons_turn(h);
    }
    let person_of = |g: &Game, u: UnitId, who: &str| g.unit_person(u).ok_or_else(|| format!("{who} is no person"));
    let (p_vet1, p_vet2, p_donal, p_rantine) = {
        let g = &s.g;
        (person_of(g, vet1, "the first veteran")?, person_of(g, vet2, "the second veteran")?, person_of(g, donal, "Donal Lugh")?, person_of(g, rantine, "Rantine")?)
    };
    if s.g.persons[p_donal].post != Post::Commander(army3) {
        return Err("Donal Lugh does not command his army".into());
    }

    // The army of Donal Lugh takes the barbarian city: the commander's claim (`City::taken_by`).
    {
        let mut units = s.g.army_units(army3);
        units.push(donal);
        let (x, y) = s.g.map.xy(lay.taken);
        match s.execute(&Command::Move { units, x, y, append: false, declare_war: None, confirm_end_siege: true }) {
            Ok(CmdOk::Applied { .. }) => {}
            other => return Err(format!("the army's move into the barbarian city: {other:?}")),
        }
    }
    let taken = s.g.map.plots[lay.taken].plot_city.filter(|&c| s.g.cities.contains(c) && s.g.cities[c].owner == h).ok_or("the barbarian city was not taken and kept")?;
    if s.g.cities[taken].taken_by.map(|(x, _)| x) != Some(p_donal) {
        return Err("the taken city does not record Donal Lugh".into());
    }

    // The Houses, each a vassal civilization: `furious` and `pleased` founded by a great person (a
    // person of note drawn), `friendly` by the first veteran. Granted in this order so that the
    // claimant House `pleased` is passed over by no grant before the taken city.
    let merchant = |g: &Game, p: PlayerId| -> Result<UnitType, String> {
        let civ = g.player_get_civilization_type(p).ok_or("no civilization")?;
        d.civ_unit(civ, d.t::<UnitClassType>("UNITCLASS_MERCHANT")).ok_or_else(|| "no Great Merchant".to_string())
    };
    let (house_a, house_b, house_c) = {
        let g = &mut s.g;
        let grant = |g: &mut Game, p: PlayerId, c: CityId, to: GrantTo| g.player_grant_city(p, c, to).map_err(|e| format!("grant of {}: {e}", g.cities[c].name));
        let (kx, ky) = (g.cities[capital].x, g.cities[capital].y);
        let gm = merchant(g, h)?;
        let gp = g.player_init_unit(h, gm, kx, ky, None, None);
        let hc = grant(g, h, c_cities[0], GrantTo::GreatPerson(gp))?;
        let c2 = g.map.plots[lay.c[1]].plot_city.ok_or("House C's second city")?;
        grant(g, h, c2, GrantTo::House(hc))?;
        let gp = g.player_init_unit(h, gm, kx, ky, None, None);
        let hb = grant(g, h, b_city, GrantTo::GreatPerson(gp))?;
        let a1 = g.map.plots[lay.a[0]].plot_city.ok_or("House A's first city")?;
        let ha = grant(g, h, a1, GrantTo::Person(p_vet1))?;
        let a2 = g.map.plots[lay.a[1]].plot_city.ok_or("House A's second city")?;
        grant(g, h, a2, GrantTo::House(ha))?;
        (ha, hb, hc)
    };
    let p_vet1 = s.g.houses[house_a].head.ok_or("House A has no head")?;
    let (p_b, p_c) = {
        let g = &s.g;
        (g.houses[house_b].head.ok_or("House B has no head")?, g.houses[house_c].head.ok_or("House C has no head")?)
    };

    // The tributes: House `friendly` 0%, House `pleased` 10% (the others at the realm's 20%).
    {
        let g = &mut s.g;
        g.player_set_house_tribute(h, house_a, 0).map_err(|e| format!("tribute: {e}"))?;
        g.player_set_house_tribute(h, house_b, 10).map_err(|e| format!("tribute: {e}"))?;
    }

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
    let p_v = s.g.houses[house_v].head.ok_or("the capitulated House has no head")?;

    // The neighbour's own House (a great person's founding): Cautious or worse.
    let house_n = {
        let g = &mut s.g;
        let (nx, ny) = {
            let cap = g.player_get_capital_city(ai).ok_or("the neighbour has no capital")?;
            (g.cities[cap].x, g.cities[cap].y)
        };
        let gm = merchant(g, ai)?;
        let gp = g.player_init_unit(ai, gm, nx, ny, None, None);
        let c = neighbour_cities.iter().copied().find(|&c| !g.city_is_capital(c)).ok_or("the neighbour's second city")?;
        g.player_grant_city(ai, c, GrantTo::GreatPerson(gp)).map_err(|e| format!("the neighbour's grant: {e}"))?
    };

    // States: House `furious` and the second veteran (who commands) Furious, by refused demands.
    {
        let g = &mut s.g;
        g.player_persons_turn(h);
        refuse_until(g, h, p_c, LoyaltyState::Furious, "House C")?;
        if g.persons[p_vet2].demand.is_none() && g.person_next_demand(p_vet2).is_none() {
            let name = g.persons[p_vet2].name.clone();
            g.persons[p_vet2].ambition = Ambition::War;
            hand_set.push(format!("{name}: ambition War (theirs makes no demand here)"));
        }
        refuse_until(g, h, p_vet2, LoyaltyState::Furious, "the Furious commander")?;
    }

    // The Great Merchant who can found a House, and the last persons turn (states told).
    let great_person = {
        let g = &mut s.g;
        let (kx, ky) = (g.cities[capital].x, g.cities[capital].y);
        let gm = merchant(g, h)?;
        let gp = g.player_init_unit(h, gm, kx, ky, None, None);
        // The cities founded and taken this turn choose their production now, as the end of the
        // turn would (a governed city never stands without one).
        let all: Vec<CityId> = g.cities.iter().map(|(c, _)| c).collect();
        for c in all {
            if g.city_is_governed(c) && g.cities[c].order_queue.is_empty() && !g.cities[c].settlement {
                g.city_choose_production(c, None, None, None, false, false);
            }
        }
        g.player_army_sync(h);
        g.player_persons_turn(h);
        g.player_persons_turn(ai);
        // The countdowns' turns left (set by the rules at Furious to 10).
        let turn = g.game_turn();
        g.houses[house_c].standing.countdown = Some(turn + 8);
        g.persons[p_vet2].standing.countdown = Some(turn + 6);
        gp
    };

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

    // Checks: what the scene promises.
    {
        let g = &s.g;
        let want = |what: &str, got: LoyaltyState, ok: &[LoyaltyState]| if ok.contains(&got) { Ok(()) } else { Err(format!("{what} is {}, not {ok:?}", got.name())) };
        want("House friendly", g.house_state(house_a), &[LoyaltyState::Friendly])?;
        want("House pleased", g.house_state(house_b), &[LoyaltyState::Pleased, LoyaltyState::Cautious])?;
        want("House annoyed", g.house_state(house_v), &[LoyaltyState::Annoyed])?;
        want("House furious", g.house_state(house_c), &[LoyaltyState::Furious])?;
        want("the neighbour's House", g.house_state(house_n), &[LoyaltyState::Cautious, LoyaltyState::Annoyed, LoyaltyState::Furious])?;
        want("the Furious commander", g.person_state(p_vet2), &[LoyaltyState::Furious])?;
        want("Donal Lugh", g.person_state(p_donal), &[LoyaltyState::Friendly, LoyaltyState::Pleased, LoyaltyState::Cautious])?;
        if g.persons[p_vet2].post != Post::Commander(army2) && g.persons[p_vet2].post != Post::Commander(army1) {
            return Err("the Furious commander has no command".into());
        }
        if g.persons[p_rantine].post != Post::None {
            return Err("Rantine holds a post".into());
        }
        if !g.city_claimants(taken).iter().any(|&c| c == Claimant::House(house_b)) || !g.city_claimants(taken).contains(&Claimant::Commander(p_donal)) {
            return Err(format!("the taken city's claimants are {:?}", g.city_claimants(taken)));
        }
        if g.house_cities_held(house_c).1.len() != 1 {
            return Err("House C's city is not held by an army".into());
        }
        if g.house_bribe_price(house_n).is_none() || g.house_bribe_refusal(house_n, h, ffh_core::persons::BribeKind::Secede).is_some() {
            return Err(format!("the neighbour's House takes no offer: {:?}", g.house_bribe_refusal(house_n, h, ffh_core::persons::BribeKind::Secede)));
        }
        g.player_check_grant(h, crown_y, GrantTo::GreatPerson(great_person)).map_err(|e| format!("the Great Merchant cannot found a House: {e}"))?;
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
        head: g.houses[id].head,
        seat: g.house_seat(id),
        cities: g.house_cities(id),
        state: g.house_state(id),
        tribute: g.houses[id].tribute,
    };
    let houses = vec![staged("friendly", house_a), staged("pleased", house_b), staged("annoyed", house_v), staged("furious", house_c), staged("neighbour", house_n)];
    let persons: Vec<(&'static str, PersonId)> = vec![
        ("head_friendly", p_vet1),
        ("head_pleased", p_b),
        ("head_annoyed", p_v),
        ("head_furious", p_c),
        ("commander_loyal", p_donal),
        ("commander_furious", p_vet2),
        ("no_post", p_rantine),
    ];
    let cmd_army = match g.persons[p_vet2].post {
        Post::Commander(a) => a,
        _ => unreachable!("checked"),
    };
    let armies: Vec<(&'static str, ArmyId)> = vec![("loyal", army3), ("furious", cmd_army), ("holds_furious", army_c)];
    let mut named: Vec<CityId> = g.players[h].cities.iter().chain(&g.players[ai].cities).copied().collect();
    for hs in &houses {
        named.extend(hs.cities.iter().copied());
    }
    let city_names = named.iter().map(|&c| (c, d.text.name(&g.cities[c].name).to_string())).collect();
    let person_names = persons.iter().map(|&(_, x)| (x, g.persons[x].name.clone(), g.person_unit(x))).collect();
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
        persons,
        armies,
        great_person,
        neighbour: ai,
        neighbour_cities,
        neighbour_near,
        hand_set,
        city_names,
        person_names,
        army_names,
        turn,
    })
}
