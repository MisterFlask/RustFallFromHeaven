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

fn staged(dir: &Path, charged_variant: bool) -> Result<StagedRituals, String> {
    let d = ffh_data::global();
    let mut ng = NewGame::new(d, 21);
    ng.opponents = 4;
    ng.civ = Some(d.t::<CivilizationType>("CIVILIZATION_HIPPUS"));
    ng.options.push(GameOptionType::GAMEOPTION_NO_BARBARIANS);
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::new_game(&ng, Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("new game: {e:?}"))?;
    let advance = |s: &mut Session| match s.advance(&mut |_| {}) {
        Stop::Human => Ok(()),
        other => Err(format!("the game stopped: {other:?}")),
    };
    advance(&mut s)?;
    let h = s.human;
    let settler = s.g.players[h].units.iter().copied().find(|&u| d.units[s.g.units[u].unit_type].found).ok_or("no settler")?;
    match s.execute(&Command::Mission { units: vec![settler], mission: MissionType::MISSION_FOUND, data1: -1, data2: -1, flags: 0, append: false }) {
        Ok(CmdOk::Applied { .. }) => {}
        other => return Err(format!("found -> {other:?}")),
    }
    // A few turns: every AI founds its capital.
    let others = |g: &ffh_core::game::Game| -> Vec<PlayerId> { g.player_ids().filter(|&p| p != h && g.players[p].alive && !g.player_is_barbarian(p)).collect() };
    while others(&s.g).iter().any(|&p| s.g.player_get_capital_city(p).is_none()) && s.g.game_turn() < 8 {
        match s.execute(&Command::EndTurn { force: true }) {
            Ok(CmdOk::Applied { .. }) => advance(&mut s)?,
            other => return Err(format!("end turn -> {other:?}")),
        }
    }
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
    // The human's second city, 4 to 6 plots from the capital on its land.
    let capital = g.player_get_capital_city(h).ok_or("no capital")?;
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
    // Research chosen: no popup asks for it on load.
    if let Some(t) = d.techs.keys().find(|&t| g.player_can_research(h, t, false)) {
        g.player_push_research(h, t, true);
    }
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
