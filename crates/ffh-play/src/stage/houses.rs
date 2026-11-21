//! `ffh-play stage houses|houses-foreign DIR`: this game's Houses (docs/HOUSES.md, rule 15) as the
//! human sees them, for the tests, the smoke and the screenshots: one realm that shows every state.
//!
//! The human plays the Bannor (Good) on a small map with three other civilizations, no
//! barbarians: `rival` (met, at peace; three cities and a House of its own, formed by the rules'
//! own pass and its AI's pick), `enemy` (met, at war with the human) and `unmet` (not met). The
//! human's Houses, each formed by the rules (`house_form`, the advisor's pick) and then given its
//! leader and leanings by hand (listed in `hand_set`), so that each state is shown whatever a
//! seed rolls:
//! - `pleased`: home, four cities (the capital its seat), a Good leader, religion The Order,
//!   sympathy for `rival` (no treaty): Pleased;
//! - `annoyed`: home, one city nine plots or more from the capital, a Neutral leader, religion the
//!   Runes of Kilmorph, sympathy for `enemy` (at war): Annoyed;
//! - `furious`: foreign, one city of `enemy`'s founding given to the human, an Evil leader,
//!   sympathy for `enemy`: Furious, its perk off;
//! - `outsider`: foreign, one city of `rival`'s founding given to the human, led by an outsider
//!   (a Good leader of the Bannor): Pleased, -3 happiness for the outsider.
//!
//! The human founds The Order in its capital (so the religion screen can preview adopting it).
//! `houses` also leaves one home city in no House, out of range of every home House that has room:
//! ending the turn forms its House, the human's pick (a home pick). `houses-foreign` instead leaves
//! a city of `enemy`'s founding in no House, out of range of House `furious`: ending the turn forms
//! a foreign House, whose pick has the outsider's card.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ffh_core::game::Game;
use ffh_core::houses::HouseState;
use ffh_core::ids::{CityId, HouseId, PlayerId};
use ffh_core::log::{LogMark, LogTag};
use ffh_data::*;

use crate::command::{CmdOk, Command};
use crate::human::AdvisorAnswerer;
use crate::json::Json;
use crate::session::{Session, SessionOptions, Stop};
use crate::setup::NewGame;

/// One House of the scene.
pub struct StagedHouse {
    /// `pleased`, `annoyed`, `furious`, `outsider` (the human's); `rival` (the rival's first).
    pub role: &'static str,
    pub id: HouseId,
    pub label: String,
    pub cities: Vec<CityId>,
    pub state: HouseState,
}

/// The staged game (see the module doc).
pub struct StagedHouses {
    pub save: PathBuf,
    /// `houses` or `houses-foreign`.
    pub stage: &'static str,
    pub human: PlayerId,
    pub rival: PlayerId,
    pub enemy: PlayerId,
    pub unmet: PlayerId,
    pub capital: CityId,
    pub houses: Vec<StagedHouse>,
    /// The city in no House, whose House forms when the turn ends.
    pub unhoused: CityId,
    /// What was set by hand beyond the rules' calls.
    pub hand_set: Vec<String>,
    pub turn: i32,
    names: Vec<(CityId, String)>,
}

impl StagedHouses {
    pub fn house(&self, role: &str) -> &StagedHouse {
        self.houses.iter().find(|h| h.role == role).unwrap_or_else(|| panic!("no House {role}"))
    }

    pub fn to_json(&self) -> Json {
        let city = |c: CityId| {
            let name = self.names.iter().find(|(id, _)| *id == c).map_or(String::new(), |(_, n)| n.clone());
            crate::jobj! { "id" => c, "name" => name }
        };
        let houses = Json::Obj(
            self.houses
                .iter()
                .map(|h| {
                    (h.role.to_string(), crate::jobj! { "id" => h.id, "label" => h.label.as_str(), "cities" => Json::Arr(h.cities.iter().map(|&c| city(c)).collect()), "state" => h.state.name() })
                })
                .collect(),
        );
        crate::jobj! {
            "stage" => self.stage, "save" => self.save.display().to_string(), "turn" => self.turn,
            "human" => self.human, "rival" => self.rival, "enemy" => self.enemy, "unmet" => self.unmet,
            "capital" => city(self.capital), "unhoused" => city(self.unhoused),
            "houses" => houses, "hand_set" => self.hand_set.clone(),
        }
    }
}

/// The seeds tried, in order, until one's map has the room the scene needs.
const SEEDS: [u32; 8] = [31, 32, 33, 34, 35, 36, 37, 38];

/// Writes the `houses` (or with `foreign` the `houses-foreign`) staged game into `dir`.
pub fn houses(dir: &Path, foreign: bool) -> Result<StagedHouses, String> {
    let mut why = Vec::new();
    for seed in SEEDS {
        match staged(dir, seed, foreign) {
            Ok(st) => return Ok(st),
            Err(e) => why.push(format!("seed {seed}: {e}")),
        }
    }
    Err(why.join("; "))
}

/// The land plot nearest `near` where `p` can found a city, `lo` to `hi` plots from it and more
/// than `apart` plots from each of `away` (no unit on it; ties by plot index).
fn site(g: &Game, p: PlayerId, near: (i32, i32), lo: i32, hi: i32, away: &[(i32, i32)], apart: i32) -> Option<(i32, i32)> {
    let mut best: Option<(i32, usize, (i32, i32))> = None;
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if g.plot_is_water(q) || g.plot_is_impassable(q) || g.map.plots[q].plot_city.is_some() || !g.map.plots[q].units.is_empty() {
            continue;
        }
        let dist = g.plot_distance(x, y, near.0, near.1);
        if dist < lo || dist > hi || away.iter().any(|&(ax, ay)| g.plot_distance(x, y, ax, ay) <= apart) || !g.player_can_found(p, x, y, false) {
            continue;
        }
        if best.map_or(true, |(bd, bq, _)| (dist, q.idx()) < (bd, bq)) {
            best = Some((dist, q.idx(), (x, y)));
        }
    }
    best.map(|(_, _, xy)| xy)
}

fn found(g: &mut Game, p: PlayerId, near: (i32, i32), lo: i32, hi: i32, away: &[(i32, i32)], apart: i32, what: &str) -> Result<CityId, String> {
    let (x, y) = site(g, p, near, lo, hi, away, apart).ok_or_else(|| format!("no site for {what}"))?;
    Ok(g.player_init_city(p, x, y, true, true))
}

fn xy(g: &Game, c: CityId) -> (i32, i32) {
    (g.cities[c].x, g.cities[c].y)
}

/// A free minor leader of `civ` (any civilization when `None`) with alignment `a`, not in `taken`,
/// in leader order.
fn leader(g: &Game, civ: Option<CivilizationType>, a: &str, taken: &[LeaderHeadType]) -> Option<LeaderHeadType> {
    let d = g.d;
    let a = d.ty::<AlignmentType>(a);
    d.leader_heads.keys().find(|&l| {
        let info = &d.leader_heads[l];
        info.minor_leader_of.is_some() && civ.map_or(true, |c| info.minor_leader_of == Some(c)) && info.alignment == a && !taken.contains(&l) && g.house_leader_is_free(l)
    })
}

/// Plays on to the human's next control point.
fn advance(s: &mut Session) -> Result<(), String> {
    match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    }
}

/// What a House of the human is given by hand once the rules formed it.
struct Leaning {
    leader: LeaderHeadType,
    religion: Option<ReligionType>,
    sympathy: Option<PlayerId>,
    perk: &'static str,
    outsider: bool,
}

/// A House of `p` over `cities` (the seat first), formed by the rules (`house_form`, the
/// advisor's pick; its message left out) and then given `x` by hand; its forming told as rule 11
/// words it, the state it is in now taken as told.
fn house(g: &mut Game, p: PlayerId, cities: &[CityId], x: Leaning, hand_set: &mut Vec<String>, role: &'static str) -> Result<HouseId, String> {
    let d = g.d;
    let log = g.log.enabled;
    g.log.enabled = false;
    let h = g.house_form(p, cities.to_vec());
    g.log.enabled = log;
    let h = h.ok_or_else(|| format!("House {role} does not form"))?;
    let alignment = d.leader_heads[x.leader].alignment;
    let perk = d.t::<TraitType>(x.perk);
    {
        let house = &mut g.houses[h];
        house.leader = x.leader;
        house.alignment = alignment;
        house.religion = x.religion;
        house.sympathy = x.sympathy;
        house.perk = perk;
        house.outsider = x.outsider;
    }
    let state = g.house_state(h);
    g.houses[h].told_state = state;
    hand_set.push(format!("House {role}: its leader, religion, sympathy, perk{}", if x.outsider { " and outsider" } else { "" }));
    let label = g.house_label(h);
    let name = ffh_core::houses::leader_name(d, x.leader);
    let names: Vec<String> = g.houses[h].cities.iter().map(|&c| crate::view::city_name(g, c)).collect();
    let text = format!("{label} forms: {name} leads {}.", and_list(&names));
    let at = Some(xy(g, cities[0]));
    let turn = g.game_turn();
    g.log.msg_tagged(turn, Some(p), at, LogMark::House, LogTag::House(h), || text);
    Ok(h)
}

fn and_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

fn staged(dir: &Path, seed: u32, foreign: bool) -> Result<StagedHouses, String> {
    let d = ffh_data::global();
    let stage: &'static str = if foreign { "houses-foreign" } else { "houses" };
    let mut ng = NewGame::new(d, seed);
    ng.opponents = 3;
    ng.world = d.t("WORLDSIZE_SMALL");
    ng.civ = Some(d.t::<CivilizationType>("CIVILIZATION_BANNOR"));
    ng.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    advance(&mut s)?;
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| d.units[s.g.units[u].unit_type].found).ok_or("no settler")?;
    match s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false, confirm_end_siege: false }) {
        Ok(CmdOk::Applied { .. }) => {}
        other => return Err(format!("found -> {other:?}")),
    }
    let others = |g: &Game| -> Vec<PlayerId> { g.player_ids().filter(|&p| p != h && g.players[p].alive && !g.player_is_barbarian(p)).collect() };
    while others(&s.g).iter().any(|&p| s.g.player_get_capital_city(p).is_none()) && s.g.game_turn() < 8 {
        match s.execute(&Command::EndTurn { force: true }) {
            Ok(CmdOk::Applied { .. }) => advance(&mut s)?,
            other => return Err(format!("end turn -> {other:?}")),
        }
    }
    let mut hand_set: Vec<String> = Vec::new();
    let g = &mut s.g;
    let team = g.players[h].team;
    let list = others(g);
    if list.len() < 3 || list.iter().any(|&p| g.player_get_capital_city(p).is_none()) {
        return Err("three civilizations with a capital are needed".into());
    }
    // `unmet`: one the human has not met; `enemy`: not Good (so its war gives House `pleased` no
    // minus); `rival`: the other.
    let unmet = *list.iter().find(|&&p| !g.teams[team].has_met[g.players[p].team]).ok_or("every civilization is met")?;
    let rest: Vec<PlayerId> = list.iter().copied().filter(|&p| p != unmet).collect();
    let good = d.ty::<AlignmentType>("ALIGNMENT_GOOD");
    let enemy = *rest.iter().find(|&&p| g.players[p].alignment != good).ok_or("no civilization that is not Good")?;
    let rival = *rest.iter().find(|&&p| p != enemy).ok_or("no rival")?;
    for p in [rival, enemy] {
        let t = g.players[p].team;
        g.team_meet(team, t, false);
    }
    let et = g.players[enemy].team;
    g.team_declare_war(team, et, false, None);
    if !g.teams[team].at_war[et] {
        return Err("the war on the enemy is not declared".into());
    }

    // The cities.
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let cap = xy(g, capital);
    let mut pleased = vec![capital];
    for i in 0..3 {
        pleased.push(found(g, h, cap, 2, 6, &[], 0, &format!("House pleased's city {i}"))?);
    }
    let annoyed = found(g, h, cap, 9, 16, &[], 0, "House annoyed's seat")?;
    let rival_cap = xy(g, g.player_get_capital_city(rival).ok_or("the rival has no capital")?);
    let enemy_cap = xy(g, g.player_get_capital_city(enemy).ok_or("the enemy has no capital")?);
    // The rival's: two of its own (its first House) and one given to the human (House outsider).
    let mut rival_cities = vec![g.player_get_capital_city(rival).ok_or("no rival capital")?];
    for i in 0..2 {
        rival_cities.push(found(g, rival, rival_cap, 2, 6, &[], 0, &format!("the rival's city {i}"))?);
    }
    let given_rival = found(g, rival, rival_cap, 2, 8, &[], 0, "the rival's city given to the human")?;
    let given_enemy = found(g, enemy, enemy_cap, 2, 8, &[], 0, "the enemy's city given to the human")?;
    // The city left in no House.
    let unhoused = if foreign {
        found(g, enemy, enemy_cap, 2, 14, &[xy(g, given_enemy)], 8, "the enemy's second city given to the human")?
    } else {
        found(g, h, cap, 2, 10, &[xy(g, annoyed)], 8, "the human's city in no House")?
    };
    let mut given = vec![given_rival, given_enemy];
    if foreign {
        given.push(unhoused);
    }
    // (A city acquired is a new city record: each is found again by its plot.)
    let given_at: Vec<(i32, i32)> = given.iter().map(|&c| xy(g, c)).collect();
    for &c in &given {
        g.player_acquire_city(h, c, false, true, true);
    }
    let mut now = Vec::new();
    for &(x, y) in &given_at {
        let q = g.map.plot_xy(x, y).ok_or("a given city's plot is off the map")?;
        let c = g.map.plots[q].plot_city.filter(|&c| g.cities[c].owner == h).ok_or("a given city is not the human's")?;
        now.push(c);
    }
    hand_set.push("cities founded for the human, the rival and the enemy; one of the rival's and one of the enemy's (two in houses-foreign) given to the human".into());
    let (from_rival, seat_enemy) = (now[0], now[1]);
    let unhoused = if foreign { now[2] } else { unhoused };
    if g.cities[from_rival].original_owner != rival || g.cities[seat_enemy].original_owner != enemy {
        return Err("a given city keeps no record of its founder".into());
    }

    // The rival's House: the rules' own pass and its AI's pick.
    g.player_houses_turn(rival);
    let rival_house = *g.players[rival].houses.first().ok_or("the rival has no House")?;

    // The human's Houses.
    let bannor = d.ty::<CivilizationType>("CIVILIZATION_BANNOR");
    let mut taken: Vec<LeaderHeadType> = Vec::new();
    let pick = |g: &Game, civ: Option<CivilizationType>, a: &str, taken: &mut Vec<LeaderHeadType>| -> Result<LeaderHeadType, String> {
        let l = leader(g, civ, a, taken).or_else(|| leader(g, None, a, taken)).ok_or_else(|| format!("no free {a} minor leader"))?;
        taken.push(l);
        Ok(l)
    };
    let order = d.ty::<ReligionType>("RELIGION_THE_ORDER");
    let runes = d.ty::<ReligionType>("RELIGION_RUNES_OF_KILMORPH");
    let l = pick(g, bannor, "ALIGNMENT_GOOD", &mut taken)?;
    let house_pleased = house(g, h, &pleased, Leaning { leader: l, religion: order, sympathy: Some(rival), perk: "TRAIT_PERK_HORSEMASTERS_1", outsider: false }, &mut hand_set, "pleased")?;
    let l = pick(g, bannor, "ALIGNMENT_NEUTRAL", &mut taken)?;
    let house_annoyed = house(g, h, &[annoyed], Leaning { leader: l, religion: runes, sympathy: Some(enemy), perk: "TRAIT_PERK_SCHOLARS_1", outsider: false }, &mut hand_set, "annoyed")?;
    let enemy_civ = g.player_get_civilization_type(enemy);
    let l = pick(g, enemy_civ, "ALIGNMENT_EVIL", &mut taken)?;
    let house_furious = house(g, h, &[seat_enemy], Leaning { leader: l, religion: None, sympathy: Some(enemy), perk: "TRAIT_PERK_SMITHS_1", outsider: false }, &mut hand_set, "furious")?;
    let l = pick(g, bannor, "ALIGNMENT_GOOD", &mut taken)?;
    let house_outsider = house(g, h, &[from_rival], Leaning { leader: l, religion: None, sympathy: None, perk: "TRAIT_PERK_MERCHANTS_1", outsider: true }, &mut hand_set, "outsider")?;

    // The Order founded in the capital: the religion screen can adopt it.
    if let Some(r) = order {
        g.player_found_religion(h, Some(r), r, false);
        hand_set.push("The Order founded in the human's capital".into());
    }

    // The checks: each House in its state, the city in no House waiting.
    let want = [(house_pleased, HouseState::Pleased, "pleased"), (house_annoyed, HouseState::Annoyed, "annoyed"), (house_furious, HouseState::Furious, "furious"), (house_outsider, HouseState::Pleased, "outsider")];
    for (hh, st, role) in want {
        if g.house_state(hh) != st {
            return Err(format!("House {role} is {}, not {}", g.house_state(hh).name(), st.name()));
        }
    }
    if g.cities[unhoused].house.is_some() || g.city_house_to_join(unhoused).is_some() {
        return Err("the city meant to be in no House is in one, or would join one".into());
    }
    let others_unhoused: Vec<CityId> = g.player_unhoused_cities(h).iter().map(|u| u.city).filter(|&c| c != unhoused).collect();
    if !others_unhoused.is_empty() {
        return Err(format!("{} other cities in no House", others_unhoused.len()));
    }
    if g.teams[team].has_met[g.players[unmet].team] {
        return Err("the unmet civilization was met".into());
    }
    let roles: Vec<(&'static str, HouseId)> = vec![("pleased", house_pleased), ("annoyed", house_annoyed), ("furious", house_furious), ("outsider", house_outsider), ("rival", rival_house)];
    let houses: Vec<StagedHouse> = roles
        .into_iter()
        .map(|(role, id)| StagedHouse { role, id, label: g.house_label(id), cities: g.houses[id].cities.clone(), state: g.house_state(id) })
        .collect();
    let mut names: Vec<(CityId, String)> = g.players[h].cities.iter().map(|&c| (c, crate::view::city_name(g, c))).collect();
    names.extend(g.players[rival].cities.iter().map(|&c| (c, crate::view::city_name(g, c))));
    let paths = s.save(stage).map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedHouses { save: paths.save, stage, human: h, rival, enemy, unmet, capital, houses, unhoused, hand_set, turn: s.g.game_turn(), names })
}
