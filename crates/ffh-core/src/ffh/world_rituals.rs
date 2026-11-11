//! World rituals (docs/CARDS.md, "World rituals"; the owner's decision, 2026-11-11): each
//! civilization's world spell (`SpellInfo::global`) spends a charge of its ritual, a project with
//! `world_ritual` (`PROJECT_RITUAL_<NAME>`, repeatable). FFH2 let a civilization cast its world
//! spell once a game, free; here it is cast once per completion of the ritual:
//! - a completion leaves a charge in the city that built it; the charge is the empire's: any unit
//!   of the owner that meets the spell's other requirements casts the spell, which spends it. A
//!   captured or destroyed city loses its charges. A player holds at most one charge and does not
//!   start the ritual again while it holds one;
//! - the ritual is built in one city of a player at a time;
//! - once a city has put production into it, it stays that city's current production until it
//!   completes: removed from the queue or no longer first in it, its production is lost
//!   ([`Game::city_check_world_rituals`]); the armies' requisitions and the empire orders do not go
//!   in front of it;
//! - it cannot be hurried, nor take production in one step (gold, population, a Great Engineer,
//!   the Soul Forge): only the city's production of the turn and its overflow;
//! - each completion doubles the next cost (`repeat_cost_percent`), with no cap on the number of
//!   doublings; Birthright Regained undoes one (`Player::world_ritual_refunds`);
//! - every player is told when one starts, is abandoned or completes;
//! - the Elohim's Sanctuary ritual is not built while their Sanctuary holds
//!   (`Player::sanctuary_timer`); `GAMEOPTION_NO_WORLD_SPELLS` forbids every world ritual.
//!
//! A world ritual without a spell naming it (`ritual_charge`) simply completes and counts.

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// The largest cost a world ritual's doublings reach (before the game speed and handicap
/// percentages, which stay within `i32` from there).
pub const WORLD_RITUAL_MAX_COST: i32 = 1_000_000;

/// A player's world ritual as the UI and the AI read it ([`Game::world_rituals`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldRitual {
    pub player: PlayerId,
    pub project: ProjectType,
    /// The world spell its charge is for (`None`: a ritual that acts at completion).
    pub spell: Option<SpellType>,
    /// The city whose queue holds it (in progress, or queued with no production yet), else the
    /// city holding its charge.
    pub city: Option<CityId>,
    /// The production put into it in that city.
    pub progress: i32,
    /// Its current cost for the player (speed-scaled, as the production dialog shows it).
    pub cost: i32,
    /// The turns left to complete it, while a city has it queued.
    pub turns_left: Option<i32>,
    /// Does a city of the player hold its charge?
    pub charged: bool,
    /// The times the player completed it, net of the doublings undone (Birthright Regained).
    pub completed: i32,
    /// Its cost after one more completion.
    pub next_cost: i32,
}

/// `cost` grown by `percent` `times` times (`repeat_cost_percent`), at most
/// [`WORLD_RITUAL_MAX_COST`].
pub fn grow_cost(cost: i32, percent: i32, times: i32) -> i32 {
    if percent <= 0 {
        return cost;
    }
    let mut c = cost as i64;
    for _ in 0..times.max(0) {
        c = c * percent as i64 / 100;
        if c >= WORLD_RITUAL_MAX_COST as i64 {
            return WORLD_RITUAL_MAX_COST;
        }
    }
    c as i32
}

/// Is `order` the creation of `project`?
fn creates(order: &crate::structs::OrderData, project: ProjectType) -> bool {
    order.order_type == OrderType::ORDER_CREATE && order.data1 == project.i()
}

/// World rituals that city `c` cannot build (`cannotCreate`): every one under
/// `GAMEOPTION_NO_WORLD_SPELLS`; outside a visibility test, while the owner holds its charge,
/// while another city of the owner has it queued, and the Sanctuary's while the Sanctuary holds.
/// Birthright Regained before the owner has completed its world ritual (net of refunds).
pub fn cannot_create(g: &Game, c: CityId, project: ProjectType, test_visible: bool) -> bool {
    let d = g.d;
    let p = g.cities[c].owner;
    if d.ty::<ProjectType>("PROJECT_BIRTHRIGHT_REGAINED") == Some(project) {
        return g.player_world_ritual_net_completions(p) < 1;
    }
    if !d.projects[project].world_ritual {
        return false;
    }
    if g.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS) {
        return true;
    }
    if test_visible {
        return false;
    }
    if g.player_world_ritual_charge_city(p, project).is_some() || g.player_world_ritual_paused(p, project) {
        return true;
    }
    g.players[p].cities.iter().any(|&o| o != c && g.cities.contains(o) && g.cities[o].order_queue.iter().any(|q| creates(q, project)))
}

/// At the completion of a world ritual in city `c` (`ev_project_built`): the owner's count, the
/// charge of its spell (if one spends it), and the news to every player.
pub fn project_built(g: &mut Game, c: CityId, project: ProjectType) {
    let d = g.d;
    if !d.projects[project].world_ritual {
        return;
    }
    let p = g.cities[c].owner;
    g.players[p].world_ritual_completions += 1;
    if g.world_ritual_spell(project).is_some() {
        g.cities[c].ritual_charges.push(project);
    }
    g.world_ritual_news(c, project, "completed");
}

impl Game {
    /// Is `project` a world ritual (`world_ritual` in its data)?
    pub fn project_is_world_ritual(&self, project: ProjectType) -> bool {
        self.d.projects[project].world_ritual
    }

    /// The world ritual of player `p`'s civilization, if it has one.
    pub fn player_world_ritual(&self, p: PlayerId) -> Option<ProjectType> {
        let civ = self.player_get_civilization_type(p)?;
        let d = self.d;
        d.projects.keys().find(|&pr| d.projects[pr].world_ritual && d.projects[pr].prereq_civilization == Some(civ))
    }

    /// The world spell (`global`) that spends charges of world ritual `project`, if one does.
    pub fn world_ritual_spell(&self, project: ProjectType) -> Option<SpellType> {
        let d = self.d;
        d.spells.keys().find(|&s| d.spells[s].global && d.spells[s].ritual_charge == Some(project))
    }

    /// Player `p`'s completions of its world ritual, net of the doublings undone: the number of
    /// times its cost doubles.
    pub fn player_world_ritual_net_completions(&self, p: PlayerId) -> i32 {
        let pl = &self.players[p];
        (pl.world_ritual_completions - pl.world_ritual_refunds).max(0)
    }

    /// The first city of player `p` (in its city order) holding a charge of `project`.
    pub fn player_world_ritual_charge_city(&self, p: PlayerId, project: ProjectType) -> Option<CityId> {
        self.players[p].cities.iter().copied().find(|&c| self.cities.contains(c) && self.cities[c].ritual_charges.contains(&project))
    }

    /// The first city of player `p` (in its city order) whose queue holds `project`.
    pub fn player_world_ritual_city(&self, p: PlayerId, project: ProjectType) -> Option<CityId> {
        self.players[p].cities.iter().copied().find(|&c| self.cities.contains(c) && self.cities[c].order_queue.iter().any(|o| creates(o, project)))
    }

    /// Is world ritual `project` paused for player `p`: the Sanctuary's while the player's
    /// Sanctuary holds (`sanctuary_timer` above 0)?
    pub fn player_world_ritual_paused(&self, p: PlayerId, project: ProjectType) -> bool {
        self.players[p].sanctuary_timer > 0 && self.world_ritual_spell(project).is_some_and(|s| self.d.spells[s].base.type_ == "SPELL_SANCTUARY")
    }

    /// The world ritual at the head of city `c`'s queue, whatever its progress.
    pub fn city_head_world_ritual(&self, c: CityId) -> Option<ProjectType> {
        let o = self.cities[c].head_order()?;
        if o.order_type != OrderType::ORDER_CREATE || o.data1 < 0 {
            return None;
        }
        let project = ProjectType::from_usize(o.data1 as usize);
        self.d.projects[project].world_ritual.then_some(project)
    }

    /// The world ritual city `c` is building: at the head of its queue with production put into
    /// it. Nothing goes in front of it without abandoning it (docs/CARDS.md).
    pub fn city_world_ritual_in_progress(&self, c: CityId) -> Option<ProjectType> {
        self.city_head_world_ritual(c).filter(|&pr| self.cities[c].project_production[pr] > 0)
    }

    /// After a change to city `c`'s queue: a world ritual with production that is no longer
    /// first in it (taken off, or put behind another order) is abandoned, its production lost.
    pub fn city_check_world_rituals(&mut self, c: CityId) {
        let d = self.d;
        let head = self.city_head_world_ritual(c);
        for project in d.projects.keys() {
            if d.projects[project].world_ritual && self.cities[c].project_production[project] > 0 && head != Some(project) {
                self.cities[c].project_production[project] = 0;
                self.world_ritual_news(c, project, "abandoned");
            }
        }
    }

    /// City `c` has put its first production into world ritual `project`.
    pub(crate) fn world_ritual_began(&mut self, c: CityId, project: ProjectType) {
        self.world_ritual_news(c, project, "began");
    }

    /// `<Civilization> <verb> <ritual> in <city>.` to every living player, at the city's plot.
    fn world_ritual_news(&mut self, c: CityId, project: ProjectType, verb: &str) {
        if !self.log.enabled {
            return;
        }
        let d = self.d;
        let p = self.cities[c].owner;
        let civ = self.player_get_civilization_short_description(p);
        let ritual = d.text.name(&d.projects[project].base.description).to_string();
        let city = d.text.name(&self.cities[c].name).to_string();
        let at = (self.cities[c].x, self.cities[c].y);
        let turn = self.game_turn();
        for o in self.player_ids().collect::<Vec<_>>() {
            if self.players[o].alive && !self.player_is_barbarian(o) {
                self.log.msg_at(turn, Some(o), at, || format!("{civ} {verb} {ritual} in {city}."));
            }
        }
    }

    /// For tests and set-ups: gives player `p` a charge of its world ritual, in its capital (else
    /// its first city), without a completion. Returns the city, `None` when the player has no
    /// world ritual or no city.
    pub fn player_grant_world_ritual_charge(&mut self, p: PlayerId) -> Option<CityId> {
        let project = self.player_world_ritual(p)?;
        let c = self.players[p].capital_city.filter(|&c| self.cities.contains(c)).or_else(|| self.players[p].cities.first().copied())?;
        self.cities[c].ritual_charges.push(project);
        Some(c)
    }

    /// Every living player's world ritual (docs/CARDS.md, "World rituals"), in player order.
    pub fn world_rituals(&self) -> Vec<WorldRitual> {
        let mut out = Vec::new();
        for p in self.player_ids() {
            if !self.players[p].alive || self.player_is_barbarian(p) {
                continue;
            }
            let Some(project) = self.player_world_ritual(p) else { continue };
            let queued = self.player_world_ritual_city(p, project);
            let charge = self.player_world_ritual_charge_city(p, project);
            let progress = queued.map_or(0, |c| self.cities[c].project_production[project]);
            let turns_left = queued.and_then(|c| {
                let i = self.cities[c].order_queue.iter().position(|o| creates(o, project))?;
                Some(self.city_production_turns_left_project(c, project, i as i32))
            });
            out.push(WorldRitual {
                player: p,
                project,
                spell: self.world_ritual_spell(project),
                city: queued.or(charge),
                progress,
                cost: self.player_get_production_needed_project(p, project),
                turns_left,
                charged: charge.is_some(),
                completed: self.player_world_ritual_net_completions(p),
                next_cost: self.player_project_production_needed_at(p, project, 1),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests;
