//! `ffh-play stage rituals|rituals-charged DIR`: the world rituals (docs/CARDS.md, "World
//! rituals") as the human sees them (docs/HUMAN.md: the rituals element, the alerts, the world
//! spell's button, the production dialog's ritual line).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ffh_core::ids::{CityId, PlayerId};
use ffh_data::*;

use crate::command::{CmdOk, Command};
use crate::human::AdvisorAnswerer;
use crate::json::Json;
use crate::session::{Session, SessionOptions, Stop};
use crate::setup::NewGame;

/// A saved game at a human control point (seed 21, a small map, five civilizations, no
/// barbarians) where the human plays the Hippus (world spell Warcry) with two cities, `capital`
/// and `second`. Without `charged`, the capital builds Ritual: Warcry at the head of its queue
/// with production in it; with `charged` (`rituals-charged`), the human holds a charge of it (in
/// its capital) and has no ritual queued. Of the four other civilizations, three are met:
/// `in_progress` builds its ritual in its capital, `charged` holds a charge, `idle` has completed
/// its ritual once (its cost doubled) and builds nothing; `unmet` (not met) builds its ritual. The
/// human's log holds the `began` messages.
pub struct StagedRituals {
    pub save: PathBuf,
    pub charged_variant: bool,
    pub capital: CityId,
    pub second: CityId,
    pub in_progress: PlayerId,
    pub charged: PlayerId,
    pub idle: PlayerId,
    pub unmet: PlayerId,
    pub turn: i32,
}

impl StagedRituals {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => if self.charged_variant { "rituals-charged" } else { "rituals" },
            "save" => self.save.display().to_string(),
            "capital" => self.capital, "second" => self.second,
            "in_progress" => self.in_progress, "charged" => self.charged, "idle" => self.idle, "unmet" => self.unmet,
            "turn" => self.turn,
        }
    }
}

/// Writes the `rituals` staged game into `dir` (a game directory).
pub fn rituals(dir: &Path) -> Result<StagedRituals, String> {
    staged(dir, false)
}

/// Writes the `rituals-charged` staged game into `dir`: as [`rituals`], with the human holding its
/// charge and no ritual queued.
pub fn rituals_charged(dir: &Path) -> Result<StagedRituals, String> {
    staged(dir, true)
}

/// Puts world ritual `project` at the head of city `c`'s queue with `percent` of its cost in it
/// (as the turn's production: the start is announced to every player).
fn begin(g: &mut ffh_core::game::Game, c: CityId, project: ProjectType, percent: i32) -> Result<(), String> {
    g.city_push_order(c, OrderType::ORDER_CREATE, project.i(), -1, false, false, false, true);
    if g.city_head_world_ritual(c) != Some(project) {
        return Err(format!("{} is not at the head of the queue", g.d.projects[project].base.type_));
    }
    let cost = g.city_production_needed_project(c, project);
    g.city_add_turn_production(c, (cost * percent / 100).max(1));
    Ok(())
}

/// A new game (seed 21, a small map, five civilizations, no barbarians) as `civ`, at the human's
/// control point once every AI has founded its capital (turn 8 at most), the human's capital
/// founded.
fn new_game_as(dir: &Path, civ: &str) -> Result<Session, String> {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, 21);
    ng.opponents = 4;
    ng.civ = Some(d.t::<CivilizationType>(civ));
    ng.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    advance(&mut s)?;
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| d.units[s.g.units[u].unit_type].found).ok_or("no settler")?;
    match s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false }) {
        Ok(CmdOk::Applied { .. }) => {}
        other => return Err(format!("found -> {other:?}")),
    }
    // A few turns: every AI founds its capital.
    while others(&s.g, h).iter().any(|&p| s.g.player_get_capital_city(p).is_none()) && s.g.game_turn() < 8 {
        match s.execute(&Command::EndTurn { force: true }) {
            Ok(CmdOk::Applied { .. }) => advance(&mut s)?,
            other => return Err(format!("end turn -> {other:?}")),
        }
    }
    Ok(s)
}

/// Plays on to the human's next control point.
fn advance(s: &mut Session) -> Result<(), String> {
    match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    }
}

/// The living civilizations other than `h`.
fn others(g: &ffh_core::game::Game, h: PlayerId) -> Vec<PlayerId> {
    g.player_ids().filter(|&p| p != h && g.players[p].alive && !g.player_is_barbarian(p)).collect()
}

/// Founds the human's second city 4 to 6 plots from `capital` on its land, building a warrior.
fn second_city(g: &mut ffh_core::game::Game, h: PlayerId, capital: CityId) -> Result<CityId, String> {
    let d = g.d;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let area = g.map.plots[g.city_plot(capital)].area;
    let mut site = None;
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        let dist = g.plot_distance(x, y, cx, cy);
        if (4..=6).contains(&dist) && g.map.plots[q].area == area && g.player_can_found(h, x, y, false) && g.map.plots[q].units.is_empty() && site.map_or(true, |(b, _, _)| dist < b) {
            site = Some((dist, x, y));
        }
    }
    let (_, x, y) = site.ok_or("no city site 4 to 6 plots from the capital")?;
    let second = g.player_init_city(h, x, y, true, true);
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    if g.cities[second].order_queue.is_empty() {
        g.city_push_order(second, OrderType::ORDER_TRAIN, warrior.idx() as i32, -1, false, false, true, true);
    }
    Ok(second)
}

/// Research chosen: no popup asks for it on load.
fn choose_research(g: &mut ffh_core::game::Game, h: PlayerId) {
    if let Some(t) = g.d.techs.keys().find(|&t| g.player_can_research(h, t, false)) {
        g.player_push_research(h, t, true);
    }
}

fn staged(dir: &Path, charged_variant: bool) -> Result<StagedRituals, String> {
    let mut s = new_game_as(dir, "CIVILIZATION_HIPPUS")?;
    let h = s.human;
    let others = |g: &ffh_core::game::Game| others(g, h);
    let g = &mut s.g;
    let team = g.players[h].team;
    // The AIs with a capital and a world ritual, those the human has not met first last.
    let mut ais: Vec<PlayerId> = others(g).into_iter().filter(|&p| g.player_get_capital_city(p).is_some() && g.player_world_ritual(p).is_some()).collect();
    ais.sort_by_key(|&p| (!g.teams[team].has_met[g.players[p].team], p.0));
    let [in_progress, charged, idle, unmet]: [PlayerId; 4] = ais.get(..4).ok_or("fewer than four AIs with a capital and a world ritual")?.try_into().map_err(|_| "four AIs")?;
    if g.teams[team].has_met[g.players[unmet].team] {
        return Err("every AI met already".into());
    }
    for p in [in_progress, charged, idle] {
        let t = g.players[p].team;
        g.team_meet(team, t, false);
    }
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let second = second_city(g, h, capital)?;
    choose_research(g, h);
    let own = g.player_world_ritual(h).ok_or("the Hippus have no world ritual")?;
    if charged_variant {
        g.player_grant_world_ritual_charge(h).ok_or("no charge granted")?;
    } else {
        begin(g, capital, own, 37)?;
    }
    let ritual = |g: &ffh_core::game::Game, p: PlayerId| g.player_world_ritual(p).ok_or("no world ritual");
    let cap = |g: &ffh_core::game::Game, p: PlayerId| g.player_get_capital_city(p).ok_or("no capital");
    let (r, c) = (ritual(g, in_progress)?, cap(g, in_progress)?);
    begin(g, c, r, 52)?;
    g.player_grant_world_ritual_charge(charged).ok_or("no charge granted")?;
    g.players[idle].world_ritual_completions = 1;
    let (r, c) = (ritual(g, unmet)?, cap(g, unmet)?);
    begin(g, c, r, 18)?;
    // The news goes to the human's message log as at a turn's end.
    s.drain_log();
    let name = if charged_variant { "rituals-charged" } else { "rituals" };
    let paths = s.save(name).map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedRituals { save: paths.save, charged_variant, capital, second, in_progress, charged, idle, unmet, turn: s.g.game_turn() })
}

/// A saved game at a human control point (as [`rituals`]: seed 21, five civilizations, no
/// barbarians) where the human plays the Calabim (Name the Tithe, Ritual: The Tithe) with two
/// cities, `capital` and `second`, and two Vampires (`vampires`), and has named no rival. It has
/// met three of the four AIs: `peace` (two, at peace) and `war` (at war); `unmet` is not met.
pub struct StagedTithe {
    pub save: PathBuf,
    pub capital: CityId,
    pub second: CityId,
    pub vampires: Vec<ffh_core::ids::UnitId>,
    pub peace: Vec<PlayerId>,
    pub war: PlayerId,
    pub unmet: PlayerId,
    pub turn: i32,
}

impl StagedTithe {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => "tithe",
            "save" => self.save.display().to_string(),
            "capital" => self.capital, "second" => self.second,
            "vampires" => self.vampires.clone(), "peace" => self.peace.clone(), "war" => self.war, "unmet" => self.unmet,
            "turn" => self.turn,
        }
    }
}

/// Writes the `tithe` staged game into `dir` (a game directory).
pub fn tithe(dir: &Path) -> Result<StagedTithe, String> {
    let d = ffh_data::global();
    let mut s = new_game_as(dir, "CIVILIZATION_CALABIM")?;
    let h = s.human;
    let g = &mut s.g;
    let team = g.players[h].team;
    let mut ais: Vec<PlayerId> = others(g, h).into_iter().filter(|&p| g.player_get_capital_city(p).is_some()).collect();
    ais.sort_by_key(|&p| (!g.teams[team].has_met[g.players[p].team], p.0));
    let [a, b, war, unmet]: [PlayerId; 4] = ais.get(..4).ok_or("fewer than four AIs with a capital")?.try_into().map_err(|_| "four AIs")?;
    if g.teams[team].has_met[g.players[unmet].team] {
        return Err("every AI met already".into());
    }
    for p in [a, b, war] {
        let t = g.players[p].team;
        g.team_meet(team, t, false);
    }
    let wt = g.players[war].team;
    g.team_declare_war(team, wt, false, None);
    if !g.teams[team].at_war[wt] {
        return Err("no war declared".into());
    }
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let second = second_city(g, h, capital)?;
    choose_research(g, h);
    let vampire = d.t::<UnitType>("UNIT_VAMPIRE");
    let promotion = d.t::<PromotionType>("PROMOTION_VAMPIRE");
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    let mut vampires = Vec::new();
    for _ in 0..2 {
        let u = g.player_init_unit(h, vampire, cx, cy, None, None);
        if !g.units[u].has_promotion[promotion] {
            g.unit_set_has_promotion(u, promotion, true);
        }
        vampires.push(u);
    }
    if g.players[h].tithe_target.is_some() {
        return Err("a rival is named already".into());
    }
    s.drain_log();
    let paths = s.save("tithe").map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedTithe { save: paths.save, capital, second, vampires, peace: vec![a, b], war, unmet, turn: s.g.game_turn() })
}

/// A saved game (a duel, seed 4) at a human control point where the human plays `civ` (a
/// `CIVILIZATION_*` type) with the land around its capital revealed, its world ritual's tech known
/// and a charge of its world ritual held (when the ritual leaves one), arranged so that its world rituals' actions are
/// offered: the Lanun's storm raised (Steer the Storm) and a second charge held; a Khazad tunnel
/// between two plots next to the capital (`unit`, a warrior, stands on its end: Enter the Delve); two Doviello
/// warriors in the capital (The Cull); for the Amurites (Wellspring is placed by the player, its
/// menu shown in its variants) culture for a second ring of borders, roads on every land plot of theirs, Body, Chaos and Earth nodes
/// (1, 2 and 3) far from the capital and an Earth Elemental in it (`unit`). With `due` (`ritual-due:<civ>`), no charge: the ritual instead stands at the head of
/// the capital's queue with all its production but 1, so that it completes at the end of the turn.
pub struct StagedRitualCiv {
    pub save: PathBuf,
    pub due: bool,
    pub civ: String,
    pub capital: CityId,
    pub unit: Option<ffh_core::ids::UnitId>,
    pub turn: i32,
}

impl StagedRitualCiv {
    pub fn to_json(&self) -> Json {
        crate::jobj! {
            "stage" => format!("{}:{}", if self.due { "ritual-due" } else { "ritual-civ" }, self.civ),
            "save" => self.save.display().to_string(),
            "capital" => self.capital, "unit" => self.unit, "turn" => self.turn,
        }
    }
}

/// Writes the `ritual-civ:<civ>` staged game into `dir` (a game directory).
pub fn ritual_civ(dir: &Path, civ: &str) -> Result<StagedRitualCiv, String> {
    ritual_civ_staged(dir, civ, false, true)
}

/// Writes the `ritual-notech:<civ>` staged game into `dir`: the human plays `civ` with the land
/// around its capital revealed, without its world ritual's tech and no charge (the production
/// list shows the ritual disabled: `Requires <tech>.`).
pub fn ritual_notech(dir: &Path, civ: &str) -> Result<StagedRitualCiv, String> {
    ritual_civ_staged(dir, civ, false, false)
}

/// Writes the `ritual-due:<civ>` staged game into `dir`: as [`ritual_civ`], the ritual completing
/// at the end of the turn instead of a charge held.
pub fn ritual_due(dir: &Path, civ: &str) -> Result<StagedRitualCiv, String> {
    ritual_civ_staged(dir, civ, true, true)
}

fn ritual_civ_staged(dir: &Path, civ: &str, due: bool, tech: bool) -> Result<StagedRitualCiv, String> {
    use ffh_core::structs::SpellTarget;
    let d = ffh_data::global();
    if d.ty::<CivilizationType>(civ).is_none() {
        return Err(format!("no civilization {civ}"));
    }
    let mut s = super::duel_as(dir, 4, 1, Some(civ))?;
    let h = s.human;
    let g = &mut s.g;
    let team = g.players[h].team;
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
    let (cx, cy) = (g.cities[capital].x, g.cities[capital].y);
    for q in g.map.plot_ids() {
        let (x, y) = g.map.xy(q);
        if g.plot_distance(x, y, cx, cy) <= 8 {
            g.plot_set_revealed(q, team, true, false, None, false);
        }
    }
    // The ritual's tech known (Ritual: Upgrade needs Masonry): the production list offers it.
    // Without `tech` (`ritual-notech`): not known, no charge, nothing more.
    let ritual_tech = g.player_world_ritual(h).and_then(|r| d.projects[r].tech_prereq);
    if !tech {
        let tech = ritual_tech.ok_or_else(|| format!("{civ}: its ritual needs no tech"))?;
        g.team_set_has_tech(team, tech, false, Some(h), false, false);
        choose_research(g, h);
        s.drain_log();
        let name = format!("ritual-notech-{}", civ.trim_start_matches("CIVILIZATION_").to_lowercase());
        let paths = s.save(&name).map_err(|e| format!("save: {e:?}"))?;
        return Ok(StagedRitualCiv { save: paths.save, due, civ: civ.to_string(), capital, unit: None, turn: s.g.game_turn() });
    }
    if let Some(tech) = ritual_tech {
        g.team_set_has_tech(team, tech, true, Some(h), false, false);
    }
    choose_research(g, h);
    // A charge, for a ritual whose spell spends one (Upgrade acts at its completion); with `due`,
    // the ritual 1 production short of completing instead.
    if due {
        let project = g.player_world_ritual(h).ok_or("no world ritual")?;
        g.city_clear_order_queue(capital);
        g.city_push_order(capital, OrderType::ORDER_CREATE, project.i(), -1, false, false, false, true);
        if g.city_head_world_ritual(capital) != Some(project) {
            return Err(format!("{} is not at the head of the queue", d.projects[project].base.type_));
        }
        let cost = g.city_production_needed_project(capital, project);
        g.city_add_turn_production(capital, cost - 1);
    } else if g.player_world_ritual(h).is_some_and(|r| g.world_ritual_spell(r).is_some()) {
        g.player_grant_world_ritual_charge(h);
    }
    let warrior = d.t::<UnitType>("UNIT_WARRIOR");
    let mut unit = None;
    match civ {
        "CIVILIZATION_LANUN" => {
            // The storm over the nearest water, then a charge again.
            let caster = g.player_init_unit(h, warrior, cx, cy, None, None);
            let spell = d.t::<SpellType>("SPELL_RAGING_SEAS");
            let near = |g: &ffh_core::game::Game, t: &SpellTarget| match *t {
                SpellTarget::Plot(q) => {
                    let (x, y) = g.map.xy(q);
                    (g.plot_distance(x, y, cx, cy), q)
                }
                _ => (i32::MAX, ffh_core::ids::PlotIdx(0)),
            };
            let target = g.unit_spell_targets(caster, spell).into_iter().min_by_key(|t| near(g, t)).ok_or("no water to raise a storm over")?;
            g.unit_cast_at(caster, spell, target);
            if g.player_storm(h).is_none() {
                return Err("Raging Seas raised no storm".into());
            }
            g.unit_kill(caster, false, None);
            g.player_grant_world_ritual_charge(h);
        }
        "CIVILIZATION_KHAZAD" => {
            // A tunnel between two of the human's land plots next to the capital, the farthest
            // apart; a warrior on its first end.
            let owned = |g: &ffh_core::game::Game, q: ffh_core::ids::PlotIdx| g.map.plots[q].owner == Some(h) && !g.plot_is_water(q) && g.map.plots[q].plot_city.is_none() && g.map.plots[q].units.is_empty();
            let near: Vec<_> = g.map.plot_ids().filter(|&q| owned(g, q) && { let (x, y) = g.map.xy(q); g.plot_distance(x, y, cx, cy) == 1 }).collect();
            let from = *near.first().ok_or("no plots of the Khazad for a tunnel")?;
            let (fx, fy) = g.map.xy(from);
            let to = near.iter().copied().max_by_key(|&q| { let (x, y) = g.map.xy(q); (g.plot_distance(x, y, fx, fy), std::cmp::Reverse(q)) }).filter(|&q| q != from).ok_or("one plot of the Khazad for a tunnel")?;
            g.delves.push((from, to));
            let (x, y) = g.map.xy(from);
            unit = Some(g.player_init_unit(h, warrior, x, y, None, None));
        }
        "CIVILIZATION_AMURITES" => {
            // Wellspring's menu in its variants: roads on every land plot of the Amurites (a node
            // there counts), nodes owned of Body (1), Chaos (2) and Earth (3), on the plots
            // farthest from the capital (the nearest stay bare for the placement), and an Earth
            // Elemental (an affinity for Earth Mana).
            // Culture for the borders' second ring: plots for the nodes and for the placement.
            g.city_change_culture(capital, h, 100, true, true);
            let road = d.t::<RouteType>("ROUTE_ROAD");
            let land: Vec<_> = g.map.plot_ids().filter(|&q| g.map.plots[q].owner == Some(h) && !g.plot_is_water(q) && !g.plot_is_peak(q)).collect();
            for &q in &land {
                g.plot_set_route_type(q, Some(road), true);
            }
            let manas = ffh_core::ffh::world_ritual_spells::small::wellspring_manas(d);
            let mut bare: Vec<_> = land.iter().copied().filter(|&q| ffh_core::ffh::world_ritual_spells::small::wellspring_plot_ok(g, h, q)).collect();
            bare.sort_by_key(|&q| {
                let (x, y) = g.map.xy(q);
                (std::cmp::Reverse(g.plot_distance(x, y, cx, cy)), q)
            });
            let mut plots = bare.into_iter();
            for (name, n) in [("BONUS_MANA_BODY", 1), ("BONUS_MANA_CHAOS", 2), ("BONUS_MANA_EARTH", 3)] {
                let b = d.t::<BonusType>(name);
                let &(_, node) = manas.iter().find(|&&(m, _)| m == b).ok_or("no node for a mana")?;
                // Up to `n` (the map may have given them one already).
                while g.player_get_num_available_bonuses(h, b) < n {
                    let q = plots.next().ok_or("too few bare plots of the Amurites for the nodes")?;
                    g.plot_set_bonus_type(q, Some(b));
                    g.plot_set_improvement_type(q, Some(node));
                }
                if g.player_get_num_available_bonuses(h, b) != n {
                    return Err(format!("{name}: {} owned, not {n}", g.player_get_num_available_bonuses(h, b)));
                }
            }
            unit = Some(g.player_init_unit(h, d.t::<UnitType>("UNIT_EARTH_ELEMENTAL"), cx, cy, None, None));
        }
        "CIVILIZATION_DOVIELLO" => {
            for _ in 0..2 {
                g.player_init_unit(h, warrior, cx, cy, None, None);
            }
        }
        _ => {}
    }
    s.drain_log();
    let name = format!("{}-{}", if due { "ritual-due" } else { "ritual-civ" }, civ.trim_start_matches("CIVILIZATION_").to_lowercase());
    let paths = s.save(&name).map_err(|e| format!("save: {e:?}"))?;
    Ok(StagedRitualCiv { save: paths.save, due, civ: civ.to_string(), capital, unit, turn: s.g.game_turn() })
}
