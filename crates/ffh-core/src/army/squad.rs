//! Squads (the owner's design, 2026-11-21; docs/ENGAGEMENTS.md, "Squads"): the units a battle
//! commits. Every land fighting unit stands in a squad: a grid of 3 rows (front, middle, rear) by
//! 3 lanes holding up to `ENGAGEMENT_SQUAD_PLACES` (6) places, a large unit filling two cells side
//! by side in one row and counting 2. One battle is one squad against one squad.
//!
//! **Where squads live.** Each squad belongs to a body ([`SquadBody`]): an army, a city's garrison,
//! or none (`Loose`: heroes, mages, recon units, marching recruits, the barbarians' units, any
//! unit in no army and no garrison). An army unit stands only in its own army's or garrison's
//! squads; a unit of no body may stand in any squad of its owner on its plot (a hero in an army's
//! squad), and holds its cell only while it is on that plot.
//!
//! **Auto and hand-set.** A squad is Auto until the player moves a unit in or out of it: an Auto
//! squad is arranged by the rules' auto formation (its strength order by each unit's attack rating)
//! whenever it is read. A hand-set squad keeps its
//! units' cells (`Unit::squad_cell`); a unit that dies leaves its cell empty.
//!
//! **The layout** ([`Game::squad_layouts_at`]) is what the squads of a player on a plot are, read
//! from the units' `squad` and `squad_cell` and settled by one rule, so a battle or a view between
//! two syncs sees a consistent layout. Each unit keeps its squad and cell when they are valid
//! (the squad is its owner's, of a body that accepts it on this plot, the cell free and in the
//! grid, room left); every other unit **arrives**: it joins the first squad, in order (armies by
//! id, then the garrison, then the loose squads; within a body by squad id), that accepts it, has
//! room for its places and a free cell (two side by side for a large unit) in its best row (the
//! first of its rank order); with none, it forms a new squad of one. Deterministic, no RNG.
//!
//! **The sync** ([`Game::player_squad_sync`], run with the army sync after every command and in
//! each player's turn) writes the layouts back: new squads get ids, every unit its squad and cell,
//! and squads no unit holds are gone. A squad of two or more units, or a hand-set one, whose units
//! all left its body for one other body together (a whole squad split off into a new army, an
//! army merged into another) moves to that body; a lone unit of an Auto squad arrives instead. A squad that ends up on
//! two plots stays on the first (by plot index); the units on the other form a new squad there.
//!
//! **Commands** ([`Game::squad_move`], [`Game::squad_set_auto`], [`Game::squad_auto_sort`]): a
//! move of one unit to a cell of a squad (a unit there of the same size swaps with it) or to a new
//! squad makes both squads hand-set; Auto gives a squad back to the auto formation; auto-sort deals
//! the units of the chosen squads into as few squads as hold them (guards first, strongest first,
//! in snake order so each squad gets a front; then the other kinds in the auto formation's order),
//! each arranged by the auto formation.

use std::collections::{BTreeMap, BTreeSet};

use ffh_data::{ActionKind, ROWS};

use crate::engagement::formation::{kind_pick_order, Side};
use crate::engagement::HitCtx;
use crate::game::Game;
use crate::ids::*;

/// Lanes of a squad's grid (and of each side of a battle).
pub const SQUAD_LANES: u8 = 3;
/// Cells of a squad's grid: `row * SQUAD_LANES + lane`, row 0 the front.
pub const SQUAD_CELLS: usize = SQUAD_LANES as usize * ROWS;

/// The body a squad belongs to (module comment).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SquadBody {
    Army(ArmyId),
    Garrison(CityId),
    Loose,
}

impl SquadBody {
    /// The order of bodies on a plot: armies by id, the garrison, the loose squads.
    fn rank(self) -> (u8, u32) {
        match self {
            SquadBody::Army(a) => (0, a.0),
            SquadBody::Garrison(c) => (1, c.0),
            SquadBody::Loose => (2, 0),
        }
    }
}

/// A squad (`Game::squads`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Squad {
    pub id: SquadId,
    pub owner: PlayerId,
    pub body: SquadBody,
    /// Hand-set: its units keep their cells. Otherwise Auto: the auto formation arranges it.
    pub hand_set: bool,
    /// The battles it fought (attacking or defending) in game turn `fought_turn`
    /// ([`Game::squad_fought_this_turn`]): the defender fields the squads that fought the fewest.
    pub fought_turn: i32,
    pub fought: i32,
}

/// A squad as it is now on a plot ([`Game::squad_layouts_at`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquadLayout {
    /// The squad (`None`: a squad the next sync creates).
    pub squad: Option<SquadId>,
    pub owner: PlayerId,
    pub body: SquadBody,
    pub hand_set: bool,
    /// Who stands in each cell (`row * 3 + lane`); a large unit in both of its cells.
    pub cells: [Option<UnitId>; SQUAD_CELLS],
}

impl SquadLayout {
    fn new(owner: PlayerId, squad: Option<SquadId>, body: SquadBody, hand_set: bool) -> SquadLayout {
        SquadLayout { squad, owner, body, hand_set, cells: [None; SQUAD_CELLS] }
    }

    /// Its units, each once with its cell (a large unit's first), in cell order.
    pub fn units(&self) -> Vec<(UnitId, u8)> {
        let mut out: Vec<(UnitId, u8)> = Vec::new();
        for (c, u) in self.cells.iter().enumerate() {
            if let Some(u) = *u {
                if !out.iter().any(|&(x, _)| x == u) {
                    out.push((u, c as u8));
                }
            }
        }
        out
    }

    /// The cell unit `u` stands in (a large unit's first).
    pub fn cell_of(&self, u: UnitId) -> Option<u8> {
        self.cells.iter().position(|&x| x == Some(u)).map(|c| c as u8)
    }

    fn contains(&self, u: UnitId) -> bool {
        self.cells.contains(&Some(u))
    }

    /// Whether `size` cells from `cell` (side by side in one row) are in the grid and free.
    fn fits(&self, cell: u8, size: i32) -> bool {
        let (lane, row) = (cell % SQUAD_LANES, cell / SQUAD_LANES);
        (row as usize) < ROWS && lane as i32 + size <= SQUAD_LANES as i32 && (0..size as u8).all(|k| self.cells[(cell + k) as usize].is_none())
    }

    fn put(&mut self, u: UnitId, cell: u8, size: i32) {
        for k in 0..size as u8 {
            self.cells[(cell + k) as usize] = Some(u);
        }
    }

    /// The first free cell of `row` for a unit of `size` cells: the centre lane first, then the
    /// lower (a large unit: lanes 0-1, then 1-2).
    fn free_in_row(&self, row: usize, size: i32) -> Option<u8> {
        let lanes: &[u8] = if size > 1 { &[0, 1] } else { &[1, 0, 2] };
        lanes.iter().map(|&l| row as u8 * SQUAD_LANES + l).find(|&c| self.fits(c, size))
    }
}

/// Where [`Game::squad_move`] puts a unit.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SquadDest {
    /// A cell of this squad.
    Squad(SquadId),
    /// A new squad of its own.
    New,
}

impl Game {
    /// The battles squad `s` fought this game turn.
    pub fn squad_fought_this_turn(&self, s: SquadId) -> i32 {
        match self.squads.get(s) {
            Some(sq) if sq.fought_turn == self.game_turn() => sq.fought,
            _ => 0,
        }
    }

    /// Counts a battle squad `s` fought (attacking or defending) this game turn.
    pub fn squad_note_battle(&mut self, s: SquadId) {
        let turn = self.game_turn();
        if let Some(sq) = self.squads.get_mut(s) {
            if sq.fought_turn != turn {
                sq.fought_turn = turn;
                sq.fought = 0;
            }
            sq.fought += 1;
        }
    }

    /// The places a squad holds (`ENGAGEMENT_SQUAD_PLACES`).
    pub fn squad_places(&self) -> i32 {
        self.d.defines.engagement_squad_places
    }

    /// Whether unit `u` stands in squads: a living land unit that fights, on land and not carried,
    /// not a summon called into a battle.
    pub fn unit_in_squads(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        if unit.battle_summon || unit.transport_unit.is_some() || !self.unit_engages_on_land(u) || !self.unit_can_fight(u) {
            return false;
        }
        self.unit_plot_opt(u).is_some_and(|p| !self.plot_is_water(p))
    }

    /// The body unit `u` belongs to: its army, its garrison, or none.
    pub fn unit_squad_body(&self, u: UnitId) -> SquadBody {
        if let Some(a) = self.units[u].army.filter(|&a| self.armies.contains(a)) {
            return SquadBody::Army(a);
        }
        match self.unit_garrison_city(u) {
            Some(c) => SquadBody::Garrison(c),
            None => SquadBody::Loose,
        }
    }

    /// Whether a squad of `body` on `plot` may hold unit `u` (module comment).
    fn squad_body_accepts(&self, body: SquadBody, u: UnitId, plot: PlotIdx) -> bool {
        let own = self.unit_squad_body(u);
        if own != SquadBody::Loose {
            return body == own;
        }
        let owner = self.units[u].owner;
        match body {
            SquadBody::Loose => true,
            SquadBody::Army(a) => self.armies.contains(a) && self.armies[a].owner == owner && self.army_plot(a) == Some(plot),
            SquadBody::Garrison(c) => self.cities.contains(c) && self.cities[c].owner == owner && self.city_plot(c) == plot,
        }
    }

    /// The cells unit `u` fills in a squad: 2 for a large unit.
    fn squad_size(&self, u: UnitId) -> i32 {
        1 + self.d.units[self.units[u].unit_type].large as i32
    }

    /// The places the units of `layout` fill.
    fn squad_layout_places(&self, layout: &SquadLayout) -> i32 {
        layout.units().iter().map(|&(u, _)| self.squad_size(u)).sum()
    }

    /// The auto formation of `units` as a squad (the rules' auto formation, as a defender, its
    /// strength order by each unit's attack rating: cheap, and the same on every plot), in cells.
    fn squad_auto_cells(&self, layout: &mut SquadLayout, units: &[UnitId], _plot: PlotIdx) {
        layout.cells = [None; SQUAD_CELLS];
        let slots = self.engage_squad_slots();
        let placed = self.engage_auto_formation_by(units, Side::Defender, &slots, |u| self.unit_attack_rating(u) as i64);
        for p in placed {
            let size = self.squad_size(p.unit);
            layout.put(p.unit, p.row * SQUAD_LANES + p.lane, size);
        }
        // A unit the formation left out (none fit: never with 9 cells for 6 places) takes any
        // free cell.
        for &u in units {
            if !layout.contains(u) {
                let size = self.squad_size(u);
                if let Some(c) = (0..ROWS).find_map(|r| layout.free_in_row(r, size)) {
                    layout.put(u, c, size);
                }
            }
        }
    }

    /// Player `p`'s squads on `plot` as they are now (module comment, "The layout"), in order:
    /// armies by id, the garrison, the loose squads, each body's squads by id, new squads last
    /// within their body. Only units that stand in squads ([`Game::unit_in_squads`]) are in them.
    pub fn squad_layouts_at(&self, p: PlayerId, plot: PlotIdx) -> Vec<SquadLayout> {
        let mut units: Vec<UnitId> = self.map.plots[plot].units.iter().copied().filter(|&u| self.units.contains(u) && self.units[u].owner == p && self.unit_in_squads(u)).collect();
        units.sort();
        let cap = self.squad_places();
        // The units each valid squad keeps.
        let mut by_squad: BTreeMap<SquadId, Vec<UnitId>> = BTreeMap::new();
        let mut arriving: Vec<UnitId> = Vec::new();
        for &u in &units {
            match self.units[u].squad {
                Some(s) if self.squads.contains(s) && self.squads[s].owner == p && self.squad_body_accepts(self.squads[s].body, u, plot) => by_squad.entry(s).or_default().push(u),
                _ => arriving.push(u),
            }
        }
        let mut layouts: Vec<SquadLayout> = Vec::new();
        let mut order: Vec<SquadId> = by_squad.keys().copied().collect();
        order.sort_by_key(|&s| (self.squads[s].body.rank(), s));
        for s in order {
            let sq = &self.squads[s];
            let mut layout = SquadLayout::new(p, Some(s), sq.body, sq.hand_set);
            let mut kept: Vec<UnitId> = Vec::new();
            let mut places = 0;
            for &u in &by_squad[&s] {
                let size = self.squad_size(u);
                if places + size > cap {
                    arriving.push(u);
                    continue;
                }
                if sq.hand_set {
                    let cell = self.units[u].squad_cell;
                    if !layout.fits(cell, size) {
                        arriving.push(u);
                        continue;
                    }
                    layout.put(u, cell, size);
                }
                places += size;
                kept.push(u);
            }
            if !sq.hand_set {
                self.squad_auto_cells(&mut layout, &kept, plot);
            }
            layouts.push(layout);
        }
        arriving.sort();
        for u in arriving {
            self.squad_arrive(&mut layouts, u, plot);
        }
        layouts
    }

    /// Unit `u` joins `layouts` by the arrival rule (module comment).
    fn squad_arrive(&self, layouts: &mut Vec<SquadLayout>, u: UnitId, plot: PlotIdx) {
        let size = self.squad_size(u);
        let best = self.unit_engage_row_order(u)[0].min(ROWS - 1);
        let cap = self.squad_places();
        for i in 0..layouts.len() {
            let l = &layouts[i];
            if !self.squad_body_accepts(l.body, u, plot) || self.squad_layout_places(l) + size > cap {
                continue;
            }
            let Some(cell) = l.free_in_row(best, size) else { continue };
            if l.hand_set {
                layouts[i].put(u, cell, size);
            } else {
                let mut members: Vec<UnitId> = layouts[i].units().into_iter().map(|(x, _)| x).collect();
                members.push(u);
                self.squad_auto_cells(&mut layouts[i], &members, plot);
            }
            return;
        }
        let body = self.unit_squad_body(u);
        let mut layout = SquadLayout::new(self.units[u].owner, None, body, false);
        self.squad_auto_cells(&mut layout, &[u], plot);
        // After the last squad of its body (or of a body before it).
        let at = layouts.iter().rposition(|l| l.body.rank() <= body.rank()).map_or(0, |i| i + 1);
        layouts.insert(at, layout);
    }

    /// The squad numbers within their body on `plot` (1-based, the order of
    /// [`Game::squad_layouts_at`]), for the views: `(squad, number)`.
    pub fn squad_numbers(layouts: &[SquadLayout]) -> Vec<u32> {
        let mut out = Vec::with_capacity(layouts.len());
        for (i, l) in layouts.iter().enumerate() {
            let n = layouts[..i].iter().filter(|x| x.body == l.body).count() as u32 + 1;
            out.push(n);
        }
        out
    }

    /// Settles player `p`'s squads (module comment, "The sync").
    pub fn player_squad_sync(&mut self, p: PlayerId) {
        // A squad whose units all left its body for one other body, on one plot, moves there.
        let mut members: BTreeMap<SquadId, Vec<UnitId>> = BTreeMap::new();
        for &u in &self.players[p].units {
            if let Some(s) = self.units[u].squad.filter(|&s| self.squads.contains(s) && self.squads[s].owner == p) {
                if self.unit_in_squads(u) {
                    members.entry(s).or_default().push(u);
                }
            }
        }
        for (&s, us) in &members {
            // A lone unit of an Auto squad arrives in its new body like any other unit.
            if us.len() < 2 && !self.squads[s].hand_set {
                continue;
            }
            let body = self.squads[s].body;
            let plot = self.unit_plot_opt(us[0]);
            if us.iter().any(|&u| plot.is_some_and(|pl| self.unit_plot_opt(u) == Some(pl) && self.squad_body_accepts(body, u, pl))) {
                continue;
            }
            let new = self.unit_squad_body(us[0]);
            if us.iter().all(|&u| self.unit_squad_body(u) == new && self.unit_plot_opt(u) == plot) {
                self.squads[s].body = new;
            }
        }
        let plots: BTreeSet<PlotIdx> = self.players[p].units.iter().filter(|&&u| self.unit_in_squads(u)).filter_map(|&u| self.unit_plot_opt(u)).collect();
        let mut used: BTreeSet<SquadId> = BTreeSet::new();
        for plot in plots {
            for l in self.squad_layouts_at(p, plot) {
                let id = match l.squad {
                    Some(s) if !used.contains(&s) => s,
                    _ => self.squads.insert_with(|id| Squad { id, owner: p, body: l.body, hand_set: l.hand_set, fought_turn: -1, fought: 0 }),
                };
                used.insert(id);
                for (u, cell) in l.units() {
                    self.units[u].squad = Some(id);
                    self.units[u].squad_cell = cell;
                }
            }
        }
        // Units that stand in no squad now (at sea, carried, lost their strength) keep a squad of
        // the plot they left only while it lives; units on land that cannot fight hold none.
        for u in self.players[p].units.clone() {
            if self.units[u].squad.is_some() && !self.unit_in_squads(u) && self.units[u].transport_unit.is_none() {
                self.units[u].squad = None;
            }
        }
        let held: BTreeSet<SquadId> = self.players[p].units.iter().filter_map(|&u| self.units[u].squad).collect();
        let gone: Vec<SquadId> = self.squads.iter().filter(|(s, sq)| sq.owner == p && !used.contains(s) && !held.contains(s)).map(|(s, _)| s).collect();
        for s in gone {
            self.squads.remove(s);
        }
    }

    /// The plot and layouts of player `p`'s squad `s` (after a sync), or an error the UI shows.
    fn squad_find(&self, p: PlayerId, s: SquadId) -> Result<(PlotIdx, Vec<SquadLayout>, usize), &'static str> {
        if !self.squads.contains(s) || self.squads[s].owner != p {
            return Err("no such squad");
        }
        let u = self.players[p].units.iter().copied().find(|&u| self.units[u].squad == Some(s) && self.unit_in_squads(u)).ok_or("the squad has no units")?;
        let plot = self.unit_plot_opt(u).ok_or("the squad has no units")?;
        let layouts = self.squad_layouts_at(p, plot);
        let i = layouts.iter().position(|l| l.squad == Some(s)).ok_or("the squad has no units")?;
        Ok((plot, layouts, i))
    }

    /// Moves player `p`'s unit `u` to `cell` (`row * 3 + lane`; a large unit's first) of `dest`
    /// (module comment, "Commands"); `None`: the first free cell of its best row there (the
    /// centre lane first), else of its other rows in its rank order. Both squads become hand-set.
    pub fn squad_move(&mut self, p: PlayerId, u: UnitId, dest: SquadDest, cell: Option<u8>) -> Result<(), &'static str> {
        if !self.units.contains(u) || self.units[u].owner != p || !self.unit_in_squads(u) {
            return Err("that unit stands in no squad");
        }
        let size = self.squad_size(u);
        let cell = match (cell, dest) {
            (Some(c), _) => c,
            (None, SquadDest::New) => {
                let best = self.unit_engage_row_order(u)[0].min(ROWS - 1);
                SquadLayout::new(p, None, SquadBody::Loose, true).free_in_row(best, size).unwrap_or(0)
            }
            (None, SquadDest::Squad(s)) => {
                let (_, layouts, i) = self.squad_find(p, s)?;
                let order = self.unit_engage_row_order(u);
                order.iter().find_map(|&r| layouts[i].free_in_row(r.min(ROWS - 1), size)).ok_or("that squad is full")?
            }
        };
        if cell as usize >= SQUAD_CELLS || (cell % SQUAD_LANES) as i32 + size > SQUAD_LANES as i32 {
            return Err("that cell is outside the squad");
        }
        self.player_squad_sync(p);
        let from = self.units[u].squad.ok_or("that unit stands in no squad")?;
        let from_cell = self.units[u].squad_cell;
        let plot = self.unit_plot_opt(u).ok_or("that unit stands in no squad")?;
        match dest {
            SquadDest::New => {
                let body = self.unit_squad_body(u);
                let alone = self.players[p].units.iter().filter(|&&x| self.units[x].squad == Some(from)).count() <= 1;
                if alone {
                    return Err("that unit is already a squad of its own");
                }
                let id = self.squads.insert_with(|id| Squad { id, owner: p, body, hand_set: true, fought_turn: -1, fought: 0 });
                self.units[u].squad = Some(id);
                self.units[u].squad_cell = cell;
                self.squads[from].hand_set = true;
            }
            SquadDest::Squad(s) => {
                let (at, layouts, i) = self.squad_find(p, s)?;
                if at != plot {
                    return Err("that squad is on another plot");
                }
                let target = &layouts[i];
                if !self.squad_body_accepts(target.body, u, plot) {
                    return Err("that unit cannot join that squad");
                }
                let mut occupants: Vec<UnitId> = (0..size as u8).filter_map(|k| target.cells[(cell + k) as usize]).filter(|&x| x != u).collect();
                occupants.dedup();
                match occupants.as_slice() {
                    [] => {
                        if s != from && self.squad_layout_places(target) + size > self.squad_places() {
                            return Err("that squad is full");
                        }
                    }
                    &[o] => {
                        let fits = self.squad_size(o) == size && target.cell_of(o) == Some(cell) && self.squad_body_accepts(self.squads[from].body, o, plot);
                        if !fits {
                            return Err("there is no room in that cell");
                        }
                        self.units[o].squad = Some(from);
                        self.units[o].squad_cell = from_cell;
                    }
                    _ => return Err("there is no room in that cell"),
                }
                self.units[u].squad = Some(s);
                self.units[u].squad_cell = cell;
                self.squads[from].hand_set = true;
                self.squads[s].hand_set = true;
            }
        }
        self.player_squad_sync(p);
        Ok(())
    }

    /// Gives player `p`'s squad `s` back to the auto formation.
    pub fn squad_set_auto(&mut self, p: PlayerId, s: SquadId) -> Result<(), &'static str> {
        if !self.squads.contains(s) || self.squads[s].owner != p {
            return Err("no such squad");
        }
        self.squads[s].hand_set = false;
        self.player_squad_sync(p);
        Ok(())
    }

    /// Auto-sort (module comment, "Commands"): the units of player `p`'s squads `list` (on one
    /// plot), each body's apart, dealt into as few squads as hold them, all Auto.
    pub fn squad_auto_sort(&mut self, p: PlayerId, list: &[SquadId]) -> Result<(), &'static str> {
        if list.is_empty() {
            return Err("no squads chosen");
        }
        self.player_squad_sync(p);
        let (plot, layouts, _) = self.squad_find(p, list[0])?;
        let mut chosen: Vec<&SquadLayout> = Vec::new();
        for &s in list {
            match layouts.iter().find(|l| l.squad == Some(s)) {
                Some(l) => {
                    if !chosen.iter().any(|c| c.squad == Some(s)) {
                        chosen.push(l);
                    }
                }
                None => return Err("the squads must stand on one plot"),
            }
        }
        let bodies: BTreeSet<SquadBody> = chosen.iter().map(|l| l.body).collect();
        let cap = self.squad_places();
        let ctx = HitCtx { to: plot, from: None, cover: false };
        let mut assign: Vec<(SquadId, Vec<UnitId>)> = Vec::new();
        for body in bodies {
            let mut ids: Vec<SquadId> = chosen.iter().filter(|l| l.body == body).filter_map(|l| l.squad).collect();
            ids.sort();
            let pool: Vec<UnitId> = chosen.iter().filter(|l| l.body == body).flat_map(|l| l.units().into_iter().map(|(u, _)| u)).collect();
            let places: i32 = pool.iter().map(|&u| self.squad_size(u)).sum();
            let k = ((places + cap - 1) / cap).max(1) as usize;
            let mut keyed: Vec<(bool, i32, i64, UnitId)> = pool
                .iter()
                .map(|&u| {
                    let kind = self.unit_engage_kind(u);
                    let guard = matches!(kind, ActionKind::Strike | ActionKind::Charge);
                    (!guard, kind_pick_order(kind, self.unit_engage_anti_flank(u)), -(self.engage_a_alone(u, Side::Defender, &ctx) as i64), u)
                })
                .collect();
            keyed.sort();
            let mut bins: Vec<Vec<UnitId>> = vec![Vec::new(); k];
            let mut fill = vec![0i32; k];
            let (mut i, mut dir) = (0usize, 1i32);
            for &(_, _, _, u) in &keyed {
                let size = self.squad_size(u);
                // The next bin in snake order with room (there is always one: k bins hold them).
                let mut tries = 0;
                while fill[i] + size > cap && tries < 2 * k {
                    (i, dir) = snake_step(i, dir, k);
                    tries += 1;
                }
                if fill[i] + size > cap {
                    // A large unit that fits nowhere whole: a squad of its own.
                    bins.push(vec![u]);
                    fill.push(size);
                    continue;
                }
                bins[i].push(u);
                fill[i] += size;
                (i, dir) = snake_step(i, dir, k);
            }
            for (j, bin) in bins.into_iter().enumerate() {
                let id = match ids.get(j) {
                    Some(&s) => s,
                    None => self.squads.insert_with(|id| Squad { id, owner: p, body, hand_set: false, fought_turn: -1, fought: 0 }),
                };
                assign.push((id, bin));
            }
        }
        for (id, bin) in assign {
            self.squads[id].hand_set = false;
            let mut layout = SquadLayout::new(p, Some(id), self.squads[id].body, false);
            self.squad_auto_cells(&mut layout, &bin, plot);
            for (u, cell) in layout.units() {
                self.units[u].squad = Some(id);
                self.units[u].squad_cell = cell;
            }
        }
        self.player_squad_sync(p);
        Ok(())
    }

    /// The bodies (each with its plot) of player `p` whose squads could be fewer: more squads than
    /// `ceil(places / ENGAGEMENT_SQUAD_PLACES)` (units that arrived formed squads of one; the AI
    /// auto-sorts those). Reads the layouts only.
    pub fn squad_bodies_to_sort(&self, p: PlayerId) -> Vec<(PlotIdx, SquadBody)> {
        let plots: BTreeSet<PlotIdx> = self.players[p].units.iter().filter(|&&u| self.unit_in_squads(u)).filter_map(|&u| self.unit_plot_opt(u)).collect();
        let cap = self.squad_places();
        let mut out = Vec::new();
        for plot in plots {
            let layouts = self.squad_layouts_at(p, plot);
            if layouts.len() < 2 {
                continue;
            }
            let bodies: BTreeSet<SquadBody> = layouts.iter().map(|l| l.body).collect();
            for body in bodies {
                let of: Vec<&SquadLayout> = layouts.iter().filter(|l| l.body == body).collect();
                let places: i32 = of.iter().map(|l| self.squad_layout_places(l)).sum();
                if of.len() as i32 > (places + cap - 1) / cap {
                    out.push((plot, body));
                }
            }
        }
        out
    }

    /// Auto-sorts every squad of body `body` of player `p` on `plot` (after a sync).
    pub fn squad_auto_sort_body(&mut self, p: PlayerId, plot: PlotIdx, body: SquadBody) -> Result<(), &'static str> {
        self.player_squad_sync(p);
        let ids: Vec<SquadId> = self.squad_layouts_at(p, plot).iter().filter(|l| l.body == body).filter_map(|l| l.squad).collect();
        self.squad_auto_sort(p, &ids)
    }
}

/// One step of a snake order over `k` bins (0, 1, .., k-1, k-1, .., 0, 0, ..).
fn snake_step(i: usize, dir: i32, k: usize) -> (usize, i32) {
    let next = i as i32 + dir;
    if next >= k as i32 {
        (k - 1, -1)
    } else if next < 0 {
        (0, 1)
    } else {
        (next as usize, dir)
    }
}
