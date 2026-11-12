//! World rituals (docs/CARDS.md, "World rituals"; the owner's decisions, 2026-11-11 and
//! 2026-11-12): each civilization's world spell (`SpellInfo::global`) comes with a completion of
//! its ritual, a project with `world_ritual` (`PROJECT_RITUAL_<NAME>`, repeatable), naming it as
//! `ritual_charge`. FFH2 let a civilization cast its world spell once a game, free; here it is cast
//! once per completion of the ritual:
//! - at completion the spell becomes the owner's pending cast (`Player::world_ritual_pending`).
//!   A spell with no target takes effect at once ([`Game::player_fire_world_spell`]), through any
//!   unit of the owner as its nominal caster (one in or nearest the completing city); with no unit,
//!   or its requirement unmet, it stays pending and takes effect at the end of the first turn it
//!   can. A spell with a target (`target`, or placed by the player: `place_on_charge`) waits for
//!   the owner to cast it: a human cannot end a turn while it has a valid target
//!   ([`Game::player_world_cast_due`], checked by ffh-play's end turn); with none, the turn ends and
//!   the cast stays pending. The pending cast is the player's: losing a city changes nothing. The
//!   ritual is not started again while its cast is pending;
//! - the ritual is built in one city of a player at a time;
//! - as every ritual (`rituals.rs`; the owner's decision, 2026-11-12): taken off the head of its
//!   city's queue it loses its production, and it is not hurried with gold (population and a great
//!   person's hurry are allowed);
//! - each completion doubles the next cost (`repeat_cost_percent`), with no cap on the number of
//!   doublings; Birthright Regained undoes one (`Player::world_ritual_refunds`);
//! - every player is told when one starts, is abandoned (its production lost:
//!   [`Game::city_check_rituals`]) or completes;
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
    /// The world spell its completion brings (`None`: a ritual that acts at completion).
    pub spell: Option<SpellType>,
    /// The city whose queue holds it (in progress, or queued with no production yet).
    pub city: Option<CityId>,
    /// The production put into it in that city.
    pub progress: i32,
    /// Its current cost for the player (speed-scaled, as the production dialog shows it).
    pub cost: i32,
    /// The turns left to complete it, while a city has it queued.
    pub turns_left: Option<i32>,
    /// Is its world spell's cast pending (completed, not yet cast)?
    pub pending: bool,
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
    if g.players[p].world_ritual_pending == Some(project) || g.player_world_ritual_paused(p, project) {
        return true;
    }
    g.players[p].cities.iter().any(|&o| o != c && g.cities.contains(o) && g.cities[o].order_queue.iter().any(|q| creates(q, project)))
}

/// At the completion of a world ritual in city `c` (`ev_project_built`): the owner's count, the
/// news to every player, and its spell (if one names it) pending: an untargeted one takes effect
/// at once when it can.
pub fn project_built(g: &mut Game, c: CityId, project: ProjectType) {
    let d = g.d;
    if !d.projects[project].world_ritual {
        return;
    }
    let p = g.cities[c].owner;
    g.players[p].world_ritual_completions += 1;
    g.world_ritual_news(c, project, "completed");
    if let Some(spell) = g.world_ritual_spell(project) {
        g.players[p].world_ritual_pending = Some(project);
        if !g.world_spell_targeted(spell) {
            g.player_fire_world_spell(p, Some(c));
        }
    }
}

/// At the end of player `p`'s turn (`player_do_turn`): an untargeted world spell still pending
/// takes effect if it now can.
pub fn check_pending(g: &mut Game, p: PlayerId) {
    let Some(project) = g.players[p].world_ritual_pending else { return };
    if g.world_ritual_spell(project).is_some_and(|s| !g.world_spell_targeted(s)) {
        g.player_fire_world_spell(p, None);
    }
}

/// The valid targets of a placed world spell for a player, and its effect at one of them
/// (returns whether it took effect).
type PlaceHooks = (fn(&Game, PlayerId) -> Vec<crate::structs::SpellTarget>, fn(&mut Game, PlayerId, crate::structs::SpellTarget) -> bool);

/// The hooks of a placed world spell (`place_on_charge`), by its `py_targets` callback.
fn place_hooks(d: &GameData, spell: SpellType) -> Option<PlaceHooks> {
    use crate::ffh::world_ritual_spells::small;
    use crate::structs::SpellTarget;
    let call = crate::ffh::pycall::PyCall::parse(&d.spells[spell].py_targets)?;
    match call.name.as_str() {
        "targetsWellspring" => Some((small::wellspring_targets, |g, p, t| matches!(t, SpellTarget::Choice(c) if small::wellspring_place(g, p, c)))),
        _ => None,
    }
}

impl Game {
    /// May player `p` place world spell `spell` now (`place_on_charge`, docs/CARDS.md "World
    /// rituals": placed by the player, no unit casting it): its civilization's, and its cast is
    /// pending. Its targets aside.
    pub fn player_can_place(&self, p: PlayerId, spell: SpellType) -> bool {
        let info = &self.d.spells[spell];
        info.place_on_charge
            && self.players[p].alive
            && !self.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS)
            && info.ritual_charge.is_some_and(|r| self.player_world_ritual(p) == Some(r) && self.players[p].world_ritual_pending == Some(r))
            && place_hooks(self.d, spell).is_some()
    }

    /// Where player `p` may place world spell `spell` now (empty when it may not:
    /// [`Game::player_can_place`]).
    pub fn player_place_targets(&self, p: PlayerId, spell: SpellType) -> Vec<crate::structs::SpellTarget> {
        match place_hooks(self.d, spell) {
            Some((targets, _)) if self.player_can_place(p, spell) => targets(self, p),
            _ => Vec::new(),
        }
    }

    /// Player `p` places world spell `spell` at `target`, one of [`Game::player_place_targets`]:
    /// the pending cast is spent, counted and announced, and the spell takes effect. Returns
    /// whether it was placed.
    pub fn player_place(&mut self, p: PlayerId, spell: SpellType, target: crate::structs::SpellTarget) -> bool {
        let d = self.d;
        let Some((_, place)) = place_hooks(d, spell) else { return false };
        if !self.player_place_targets(p, spell).contains(&target) {
            return false;
        }
        let Some(ritual) = d.spells[spell].ritual_charge else { return false };
        if self.players[p].world_ritual_pending != Some(ritual) {
            return false;
        }
        self.players[p].world_ritual_pending = None;
        self.players[p].world_spell_casts += 1;
        let turn = self.game_turn();
        self.log.msg(turn, Some(p), || format!("World spell: {}", d.text.name(&d.spells[spell].base.description)));
        place(self, p, target)
    }
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

    /// Is the world spell of `project` pending for player `p` (completed, not yet cast)?
    pub fn player_world_ritual_pending(&self, p: PlayerId, project: ProjectType) -> bool {
        self.players[p].world_ritual_pending == Some(project)
    }

    /// Is world spell `spell` cast at a target the player picks (`target`, or placed:
    /// `place_on_charge`)? An untargeted one takes effect at its ritual's completion.
    pub fn world_spell_targeted(&self, spell: SpellType) -> bool {
        let info = &self.d.spells[spell];
        info.target.is_some() || info.place_on_charge
    }

    /// The pending targeted world spell player `p` must cast before its turn ends: one with at
    /// least one valid target now (a place of a placed spell, or a target of a unit that can cast
    /// it). `None`: nothing pending, an untargeted spell, or no valid target now.
    pub fn player_world_cast_due(&self, p: PlayerId) -> Option<SpellType> {
        let project = self.players[p].world_ritual_pending?;
        let spell = self.world_ritual_spell(project)?;
        if !self.world_spell_targeted(spell) || self.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS) {
            return None;
        }
        if self.d.spells[spell].place_on_charge {
            return (!self.player_place_targets(p, spell).is_empty()).then_some(spell);
        }
        let can = |u: UnitId| self.units.contains(u) && self.unit_plot_opt(u).is_some() && self.unit_can_cast_untargeted(u, spell, false) && !self.unit_spell_targets(u, spell).is_empty();
        self.players[p].units.iter().copied().any(can).then_some(spell)
    }

    /// Player `p`'s pending untargeted world spell takes effect now, through a nominal caster: a
    /// unit of `p` on the map, in `near` (the completing city) first, then the nearest to it (to
    /// the capital without one), one that loses no promotion by casting first. It stays pending
    /// while `p` has no unit or the spell's requirement is unmet. Returns whether it took effect.
    pub fn player_fire_world_spell(&mut self, p: PlayerId, near: Option<CityId>) -> bool {
        let d = self.d;
        let Some(project) = self.players[p].world_ritual_pending else { return false };
        let Some(spell) = self.world_ritual_spell(project) else { return false };
        if self.world_spell_targeted(spell) || self.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS) {
            return false;
        }
        let at = near.or(self.players[p].capital_city).filter(|&c| self.cities.contains(c)).map(|c| (self.cities[c].x, self.cities[c].y));
        let loses = |u: UnitId| d.promotions.keys().filter(|&pr| self.units[u].has_promotion[pr] && d.promotions[pr].removed_by_casting).count();
        let caster = self.players[p]
            .units
            .iter()
            .copied()
            .filter(|&u| self.units.contains(u) && self.unit_plot_opt(u).is_some() && !self.units[u].death_delay)
            .min_by_key(|&u| (at.map_or(0, |(x, y)| self.plot_distance(x, y, self.units[u].x, self.units[u].y)), loses(u), u.0));
        let Some(u) = caster else { return false };
        if !d.spells[spell].py_requirement.is_empty() && !self.py_can_cast(u, spell) {
            return false;
        }
        // (The cast spends the pending cast: `unit_spend_ritual_charge`.)
        self.unit_cast(u, spell);
        true
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

    /// City `c` has lost the production of world ritual `project` ([`Game::city_check_rituals`]).
    pub(crate) fn world_ritual_abandoned(&mut self, c: CityId, project: ProjectType) {
        self.world_ritual_news(c, project, "abandoned");
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
                // A player who has not met the ritual's owner is not told who it is.
                let met = self.teams[self.players[o].team].has_met[self.players[p].team];
                let who = if met { civ.as_str() } else { "Unknown civilization" };
                self.log.msg_at(turn, Some(o), at, || format!("{who} {verb} {ritual} in {city}."));
            }
        }
    }

    /// For tests and set-ups: makes player `p`'s world spell pending, without a completion (an
    /// untargeted one takes effect at the end of its turn). Returns the ritual, `None` when the
    /// player has none.
    pub fn player_grant_world_ritual_pending(&mut self, p: PlayerId) -> Option<ProjectType> {
        let project = self.player_world_ritual(p)?;
        self.players[p].world_ritual_pending = Some(project);
        Some(project)
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
            let progress = queued.map_or(0, |c| self.cities[c].project_production[project]);
            let turns_left = queued.and_then(|c| {
                let i = self.cities[c].order_queue.iter().position(|o| creates(o, project))?;
                Some(self.city_production_turns_left_project(c, project, i as i32))
            });
            out.push(WorldRitual {
                player: p,
                project,
                spell: self.world_ritual_spell(project),
                city: queued,
                progress,
                cost: self.player_get_production_needed_project(p, project),
                turns_left,
                pending: self.player_world_ritual_pending(p, project),
                completed: self.player_world_ritual_net_completions(p),
                next_cost: self.player_project_production_needed_at(p, project, 1),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests;
