//! Raids (this game's own rules, not FFH2's; docs/RAIDS.md, the owner's design, 2026-11-21).
//!
//! An army parked on an enemy city's countryside either besieges it (docs/SIEGES.md) or holds the
//! **Raid** order on it (`Station::Raid`, [`Game::army_set_station`]): it takes a share of the
//! city's production and commerce as gold. The city's guard answers by rule, at its owner's raid
//! step ([`Game::player_do_raids`]), under the city's sally posture (`City::sally`).
//!
//! **Words** (section 2). The *countryside* of a city: the land plots whose `working_city` it is,
//! worked by a citizen or not, its own plot left out ([`Game::city_countryside`]). *Fields*: an
//! improvement a city's work crews build (a build with `city`) or one it grows into
//! (`improvement_upgrade`), on a plot of some city's countryside ([`Game::plot_is_fields`]).
//! The *guard*: the city owner's squads on its plot that belong to its garrison or to one of its
//! armies ([`Game::city_guard_squads`]). The *raiders*: the armies holding a valid Raid order on
//! it ([`Game::city_raiders`]).
//!
//! **The order** (section 3): refused by [`Game::army_raid_refusal`]; taken, the army's moves end
//! and its units sleep, and its supply counter is 0 (it lives off the land). It holds while
//! [`Game::army_raid_end_reason`] finds nothing: it ends with a move order or an attack
//! (`group_order_ends_station`, `group_attack_ends_raid`), when the army leaves the countryside,
//! the war ends, the city changes hands or is gone, or the countryside is stripped; and when the
//! guard drives the raiders off. An order that no longer holds is cleared at its owner's turn
//! end (`player_fleet_stations`, [`Game::army_raid_step`]) and at the city owner's raid step.
//!
//! **The raid step** (section 5, [`Game::player_do_raids`], every city of the player, in city
//! order): a stripped city counts down and takes nothing; otherwise the guard sallies against each
//! raiding plot in plot order (as many battles as enemy squads stood there, each one the posture
//! allows, the guard squad that fought fewest this turn then the strongest, an engagement in the
//! fields: [`crate::engagement::HitCtx::sally`]); the raiders that lose a battle are pushed back
//! and every Raid order on that plot ends. Then the state (contested: every raiding plot left was
//! sallied against; overrun: some raiders were not fought), the take (the city's production and
//! commerce modifier, `City::raid_percent`, and the gold to the raiders), and the strip counter.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use ffh_data::*;

use crate::army::Station;
use crate::engagement::engage::{EngageOrder, SallyForce};
use crate::engagement::formation::Side;
use crate::engagement::squads::FieldSquad;
use crate::engagement::{HitCtx, WarTest};
use crate::game::Game;
use crate::ids::*;

/// A city's sally posture (docs/RAIDS.md, section 4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sally {
    /// The guard never sallies.
    Never,
    /// The guard sallies when the forecast gives its squad at least `RAID_SALLY_ODDS` per mille
    /// to win the battle.
    #[default]
    Favoured,
    /// The guard sallies whenever it has a squad that can fight.
    Always,
}

impl Sally {
    pub const ALL: [Sally; 3] = [Sally::Never, Sally::Favoured, Sally::Always];

    /// Its name in the views and commands (`Never`, `Favoured`, `Always`).
    pub fn name(self) -> &'static str {
        match self {
            Sally::Never => "Never",
            Sally::Favoured => "Favoured",
            Sally::Always => "Always",
        }
    }

    pub fn from_name(s: &str) -> Option<Sally> {
        Sally::ALL.into_iter().find(|x| x.name().eq_ignore_ascii_case(s))
    }
}

/// A city's raid state as both sides see it (docs/RAIDS.md, section 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaidState {
    /// No raid on it (no take this turn).
    None,
    /// Every raiding plot was sallied against: `RAID_CONTESTED_PERCENT`.
    Contested,
    /// Some raiders were not fought: `RAID_TAKE_PERCENT`.
    Overrun,
    /// The countryside is stripped: turns left until it recovers.
    Stripped(i32),
}

/// One raiding army's share of a take ([`RaidCalc`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaidShare {
    pub army: ArmyId,
    pub owner: PlayerId,
    /// Its equal part of the city's loss (the remainder to the first by army id).
    pub share: i32,
    /// Its owner's pillage gold modifier (percent more; `Player::pillaging_gold`, the Raiders
    /// trait).
    pub modifier: i32,
    /// The gold it gains: `share + share * modifier / 100`.
    pub gold: i32,
}

/// The parts of a raid's take (docs/RAIDS.md, section 5.3; the tooltips' calculation).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RaidCalc {
    /// The raid modifier (percent less of the production and commerce yield rates).
    pub percent: i32,
    /// The city's base production and commerce yield rates (before every modifier).
    pub production_base: i32,
    pub commerce_base: i32,
    /// The yield rates without the raid modifier and with it.
    pub production_without: i32,
    pub production_with: i32,
    pub commerce_without: i32,
    pub commerce_with: i32,
    /// What the city loses: without less with.
    pub production_lost: i32,
    pub commerce_lost: i32,
    /// The city's loss in all (production and commerce), split between the raiders.
    pub total: i32,
    /// Each raiding army's share, in army id order.
    pub shares: Vec<RaidShare>,
}

/// The improvements that are fields wherever they stand on a countryside: those a build with
/// `city` makes, and every improvement they grow into (`improvement_upgrade`).
fn fields_improvements(d: &GameData) -> &'static [bool] {
    static FIELDS: OnceLock<Vec<bool>> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let mut out = vec![false; d.improvements.len()];
        for b in d.builds.keys() {
            let info = &d.builds[b];
            if !info.city {
                continue;
            }
            let mut next = info.improvement;
            // (A chain of upgrades never loops in the data; the bound keeps a bad one finite.)
            let mut steps = 0;
            while let Some(i) = next {
                if out[i.idx()] || steps > d.improvements.len() {
                    break;
                }
                out[i.idx()] = true;
                next = d.improvements[i].improvement_upgrade;
                steps += 1;
            }
        }
        out
    })
}

impl Game {
    // ---------------------------------------------------------------------------------------
    // Words (section 2)
    // ---------------------------------------------------------------------------------------

    /// Whether `plot` is a plot of city `c`'s countryside: a land plot whose working city it is,
    /// not the city's own plot.
    pub fn plot_in_countryside(&self, plot: PlotIdx, c: CityId) -> bool {
        self.cities.contains(c) && self.map.plots[plot].working_city == Some(c) && plot != self.city_plot(c) && !self.plot_is_water(plot)
    }

    /// City `c`'s countryside (section 2), in the city's plot order.
    pub fn city_countryside(&self, c: CityId) -> Vec<PlotIdx> {
        if !self.cities.contains(c) {
            return Vec::new();
        }
        (0..self.city_num_city_plots(c) as usize).filter_map(|i| self.city_index_plot(c, i)).filter(|&p| self.plot_in_countryside(p, c)).collect()
    }

    /// Whether improvement `i` is a field improvement (crew-built, or grown from one).
    pub fn improvement_is_fields(&self, i: ImprovementType) -> bool {
        fields_improvements(self.d).get(i.idx()).copied().unwrap_or(false)
    }

    /// Whether `plot` holds fields (section 2): a field improvement on a plot of some city's
    /// countryside.
    pub fn plot_is_fields(&self, plot: PlotIdx) -> bool {
        let p = &self.map.plots[plot];
        p.improvement_type.is_some_and(|i| self.improvement_is_fields(i)) && p.working_city.is_some_and(|c| self.plot_in_countryside(plot, c))
    }

    /// City `c`'s guard (section 2): its owner's squads on its plot that belong to its garrison or
    /// to one of its owner's armies, each with its units that stand in squads, in the plot's squad
    /// order. Loose squads (heroes, mages, recon units on their own) are not in it.
    pub fn city_guard_squads(&self, c: CityId) -> Vec<FieldSquad> {
        if !self.cities.contains(c) {
            return Vec::new();
        }
        let owner = self.cities[c].owner;
        let plot = self.city_plot(c);
        let units: Vec<UnitId> = self.map.plots[plot].units.iter().copied().filter(|&u| self.units[u].owner == owner && !self.unit_is_dead(u) && self.unit_in_squads(u)).collect();
        self.engage_field_squads(plot, &units, Side::Attacker, &HitCtx::at(plot)).into_iter().filter(|s| self.raid_guard_body(c, s)).collect()
    }

    /// Whether field squad `s` is of city `c`'s guard: its garrison's or an army of its owner's.
    fn raid_guard_body(&self, c: CityId, s: &FieldSquad) -> bool {
        match s.body {
            crate::army::SquadBody::Garrison(g) => g == c,
            crate::army::SquadBody::Army(a) => self.armies.get(a).is_some_and(|ar| ar.owner == self.cities[c].owner),
            crate::army::SquadBody::Loose => false,
        }
    }

    /// Whether unit `u` of city `c`'s guard can fight a sally against `to`: a living land unit
    /// that can attack (not a missile, not a summon called into a battle, not marching) within its
    /// combat limit there. Its moves and attacks this turn are not read (section 5.1).
    fn raid_unit_can_sally(&self, u: UnitId, to: PlotIdx) -> bool {
        let unit = &self.units[u];
        !self.unit_is_dead(u)
            && !unit.battle_summon
            && unit.march.is_none()
            && !self.unit_is_suicide(u)
            && self.unit_engages_on_land(u)
            && self.unit_can_attack(u)
            && self.engage_applies(u, to)
            && self.unit_engage_limit_ok(u, to, WarTest::AtWar)
    }

    /// The guard squads of city `c` that can sally against `to`, each with those of its units
    /// that can (the others leave their cells empty); squads with none are left out.
    pub fn raid_sally_squads(&self, c: CityId, to: PlotIdx) -> Vec<FieldSquad> {
        let mut out = self.city_guard_squads(c);
        for s in out.iter_mut() {
            s.places.retain(|p| self.raid_unit_can_sally(p.unit, to));
        }
        out.retain(|s| !s.places.is_empty());
        out
    }

    /// The guard squad of city `c` that would sally against `to` now (section 5.1): of those that
    /// can, the ones that fought the fewest battles this game turn, then the strongest. `None`:
    /// no squad that can fight, or nobody to fight there.
    pub fn raid_sally_pick(&self, c: CityId, to: PlotIdx) -> Option<FieldSquad> {
        let owner = self.cities.get(c)?.owner;
        let mut squads = self.raid_sally_squads(c, to);
        let force: Vec<UnitId> = squads.iter().flat_map(|s| s.units()).collect();
        if self.engage_defenders(to, owner, &force, WarTest::AtWar).is_empty() {
            return None;
        }
        let fought = |s: &FieldSquad| s.squad.map_or(0, |id| self.squad_fought_this_turn(id));
        let fewest = squads.iter().map(fought).min()?;
        squads.retain(|s| fought(s) == fewest);
        let ctx = HitCtx::sally(self.city_plot(c), to);
        let i = self.engage_strongest_squad(&squads, Side::Attacker, &ctx)?;
        Some(squads.swap_remove(i))
    }

    /// The forecast of the sally city `c`'s guard would fight against `to` now (sections 4 and
    /// 5.1): the `won_permille` of the squad [`Game::raid_sally_pick`] names, over
    /// `RAID_SALLY_FORECAST_RUNS` battles of the real sally's set-up on a private RNG (no draw of
    /// the game's). `None`: no squad that can fight, or nobody to fight. `to` must hold units the
    /// guard can fight (its enemies, as a battle meets them); they need not hold the Raid order (the
    /// AI asks before its army on `to` takes it). A sally with a summoner on either side forecasts
    /// in a copy of the game (as the attack preview does): deterministic, but not cheap.
    pub fn raid_sally_forecast(&self, c: CityId, to: PlotIdx) -> Option<i32> {
        let sq = self.raid_sally_pick(c, to)?;
        let runs = self.d.defines.raid_sally_forecast_runs.max(1);
        self.engage_sally_forecast(self.cities[c].owner, self.city_plot(c), to, sq.places, runs).ok().map(|f| f.won_permille)
    }

    /// Whether city `c`'s guard sallies against `to` under its posture now (section 4); `Some`:
    /// the squad that does.
    fn raid_sally_decides(&self, c: CityId, to: PlotIdx) -> Option<FieldSquad> {
        let sq = self.raid_sally_pick(c, to)?;
        match self.cities[c].sally {
            Sally::Never => None,
            Sally::Always => Some(sq),
            Sally::Favoured => {
                let runs = self.d.defines.raid_sally_forecast_runs.max(1);
                let won = self.engage_sally_forecast(self.cities[c].owner, self.city_plot(c), to, sq.places.clone(), runs).ok()?.won_permille;
                (won >= self.d.defines.raid_sally_odds).then_some(sq)
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // The order (section 3)
    // ---------------------------------------------------------------------------------------

    /// Why army `a` may not take the Raid order on city `c` now (section 3); `None`: it may.
    pub fn army_raid_refusal(&self, a: ArmyId, c: CityId) -> Option<&'static str> {
        let Some(army) = self.armies.get(a) else { return Some("No such army.") };
        if army.domain != crate::army::ArmyDomain::Land {
            return Some("A fleet does not raid.");
        }
        if self.player_is_barbarian(army.owner) {
            return Some("The barbarians do not raid.");
        }
        if army.disband_to.is_some() {
            return Some("The army is disbanding.");
        }
        if !self.cities.contains(c) {
            return Some("No such city.");
        }
        let owner = self.cities[c].owner;
        if self.player_is_barbarian(owner) {
            return Some("Barbarian cities are not raided.");
        }
        let team = self.players[army.owner].team;
        if !self.at_war(team, self.city_team(c)) {
            return Some("Requires war with the city's owner.");
        }
        let Some(plot) = self.army_plot(a) else { return Some("The army is not on the map.") };
        if !self.plot_in_countryside(plot, c) {
            return Some("The army is not on the city's countryside.");
        }
        if self.cities[c].raid_stripped > 0 {
            return Some("The countryside is stripped.");
        }
        if army.station == Station::Raid(c) {
            return Some("The army raids this city already.");
        }
        None
    }

    /// Why army `a`'s Raid order on city `c` no longer holds (section 3); `None`: it holds.
    pub fn army_raid_end_reason(&self, a: ArmyId, c: CityId) -> Option<&'static str> {
        let Some(army) = self.armies.get(a) else { return Some("the army is gone") };
        if army.domain != crate::army::ArmyDomain::Land {
            return Some("a fleet does not raid");
        }
        if !self.cities.contains(c) {
            return Some("the city is gone");
        }
        let team = self.players[army.owner].team;
        let city_team = self.city_team(c);
        if city_team == team {
            return Some("the city changed hands");
        }
        if !self.at_war(team, city_team) {
            return Some("the war ended");
        }
        if !self.army_plot(a).is_some_and(|plot| self.plot_in_countryside(plot, c)) {
            return Some("the army left the countryside");
        }
        if self.cities[c].raid_stripped > 0 {
            return Some("the countryside is stripped");
        }
        None
    }

    /// The city army `a` raids, if its Raid order still holds (section 3).
    pub fn army_raid_city(&self, a: ArmyId) -> Option<CityId> {
        match self.armies.get(a)?.station {
            Station::Raid(c) if self.army_raid_end_reason(a, c).is_none() => Some(c),
            _ => None,
        }
    }

    /// The city the army of unit `u` raids (the order holding), if any.
    pub fn unit_raid_city(&self, u: UnitId) -> Option<CityId> {
        let a = self.units.get(u)?.army?;
        self.army_raid_city(a)
    }

    /// The raiders of city `c` (section 2): the armies holding a valid Raid order on it, in army
    /// id order.
    pub fn city_raiders(&self, c: CityId) -> Vec<ArmyId> {
        if !self.cities.contains(c) {
            return Vec::new();
        }
        let mut out: Vec<ArmyId> = Vec::new();
        for p in self.player_ids() {
            for &a in &self.players[p].armies {
                if self.armies.get(a).is_some_and(|ar| ar.station == Station::Raid(c)) && self.army_raid_city(a) == Some(c) {
                    out.push(a);
                }
            }
        }
        out.sort();
        out
    }

    /// Army `a` takes the Raid order on city `c` (checked by `army_set_station`): its moves end,
    /// its units sleep, its supply counter is 0; the messages to both sides.
    pub(crate) fn army_take_raid(&mut self, a: ArmyId, c: CityId) {
        self.armies[a].station = Station::Raid(c);
        self.armies[a].unsupplied_turns = 0;
        for u in self.army_units(a) {
            if self.units.contains(u) {
                self.unit_finish_moves(u);
            }
        }
        if let Some(grp) = self.armies[a].group.filter(|&g| self.groups.contains(g)) {
            self.group_clear_mission_queue(grp);
            self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
        }
        let p = self.armies[a].owner;
        let owner = self.cities[c].owner;
        let city = self.d.text.name(&self.cities[c].name).to_owned();
        let name = self.armies[a].name.clone();
        let adj = self.player_get_civilization_adjective(p);
        let turn = self.game_turn();
        let at = self.map.xy(self.city_plot(c));
        self.log.msg_at(turn, Some(p), at, || format!("The {name} raids {city}."));
        self.log.msg_at(turn, Some(owner), at, || format!("The {adj} {name} raids {city}."));
        if self.telemetry.on() {
            self.tel_raid_order(p, owner, c);
        }
    }

    /// Army `a`'s Raid order on city `c` ended (its station already cleared) because of `why`:
    /// the messages to both sides (a city gone: its owner's side has no city to name).
    pub(crate) fn raid_order_ended(&mut self, a: ArmyId, c: CityId, why: &'static str) {
        let Some(army) = self.armies.get(a) else { return };
        let p = army.owner;
        let name = army.name.clone();
        let turn = self.game_turn();
        if !self.cities.contains(c) {
            self.log.msg(turn, Some(p), || format!("The {name} stops raiding: {why}."));
            return;
        }
        let owner = self.cities[c].owner;
        let city = self.d.text.name(&self.cities[c].name).to_owned();
        let adj = self.player_get_civilization_adjective(p);
        let at = self.map.xy(self.city_plot(c));
        self.log.msg_at(turn, Some(p), at, || format!("The {name} stops raiding {city}: {why}."));
        if owner != p {
            self.log.msg_at(turn, Some(owner), at, || format!("The {adj} {name} stops raiding {city}: {why}."));
        }
        if self.telemetry.on() {
            self.tel_raid_end(p, why);
        }
    }

    /// Army `a`'s Raid order at its owner's turn end (`player_fleet_stations`): ended when it no
    /// longer holds; else its units keep sleeping (a group with no order of its own).
    pub fn army_raid_step(&mut self, a: ArmyId) {
        let Some(Station::Raid(c)) = self.armies.get(a).map(|ar| ar.station) else { return };
        if let Some(why) = self.army_raid_end_reason(a, c) {
            self.armies[a].station = crate::army::Station::None;
            self.raid_order_ended(a, c, why);
            return;
        }
        if let Some(grp) = self.armies[a].group.filter(|&g| self.groups.contains(g)) {
            if self.groups[grp].mission_queue.is_empty() && !self.group_is_busy(grp) && self.groups[grp].activity_type != ActivityType::ACTIVITY_SLEEP {
                self.group_set_activity_type(grp, ActivityType::ACTIVITY_SLEEP);
            }
        }
    }

    /// Ends every Raid order on city `c` (valid or not) because of `why`.
    fn raid_end_all(&mut self, c: CityId, why: &'static str) {
        for p in self.player_ids().collect::<Vec<_>>() {
            for a in self.players[p].armies.clone() {
                if self.armies.get(a).is_some_and(|ar| ar.station == Station::Raid(c)) {
                    self.armies[a].station = Station::None;
                    self.raid_order_ended(a, c, why);
                }
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // The take (section 5.3)
    // ---------------------------------------------------------------------------------------

    /// City `c`'s raid state now (section 8).
    pub fn city_raid_state(&self, c: CityId) -> RaidState {
        let city = &self.cities[c];
        let d = &self.d.defines;
        if city.raid_stripped > 0 {
            RaidState::Stripped(city.raid_stripped)
        } else if city.raid_percent == 0 {
            RaidState::None
        } else if city.raid_percent == d.raid_take_percent {
            RaidState::Overrun
        } else {
            RaidState::Contested
        }
    }

    /// The take of a raid on city `c` at `percent` shared by the armies `raiders` (section 5.3):
    /// what the city's production and commerce yield rates lose at that modifier in place of
    /// its raid modifier now, and each army's gold. Pure.
    pub fn raid_take_calc(&self, c: CityId, percent: i32, raiders: &[ArmyId]) -> RaidCalc {
        let city = &self.cities[c];
        let now = city.raid_percent;
        let rate = |y: YieldType, pct: i32| -> (i32, i32, i32) {
            let base = city.base_yield_rate[y.idx()];
            // Without any raid (the current modifier's percent given back), and with `pct`.
            let without = base * self.city_base_yield_rate_modifier(c, y, now) / 100;
            let with = base * self.city_base_yield_rate_modifier(c, y, now - pct) / 100;
            (base, without, with)
        };
        let (pb, pw, pr) = rate(YieldType::YIELD_PRODUCTION, percent);
        let (cb, cw, cr) = rate(YieldType::YIELD_COMMERCE, percent);
        let total = (pw - pr).max(0) + (cw - cr).max(0);
        let mut armies: Vec<ArmyId> = raiders.iter().copied().filter(|&a| self.armies.contains(a)).collect();
        armies.sort();
        armies.dedup();
        let n = armies.len() as i32;
        let shares = armies
            .iter()
            .enumerate()
            .map(|(i, &a)| {
                let owner = self.armies[a].owner;
                let share = total / n + if i == 0 { total % n } else { 0 };
                let modifier = self.players[owner].pillaging_gold;
                RaidShare { army: a, owner, share, modifier, gold: (share + share * modifier / 100).max(0) }
            })
            .collect();
        RaidCalc {
            percent,
            production_base: pb,
            commerce_base: cb,
            production_without: pw,
            production_with: pr,
            commerce_without: cw,
            commerce_with: cr,
            production_lost: (pw - pr).max(0),
            commerce_lost: (cw - cr).max(0),
            total,
            shares,
        }
    }

    /// The take of the raid on city `c` now (section 5.3; the tooltips): at its raid modifier,
    /// shared by its raiders.
    pub fn city_raid_calc(&self, c: CityId) -> RaidCalc {
        self.raid_take_calc(c, self.cities[c].raid_percent, &self.city_raiders(c))
    }

    /// What a raid on city `c` would take each turn with army `a` among its raiders, overrun
    /// (`RAID_TAKE_PERCENT`): the forecast of the Raid order's tooltip and the AI's. Pure.
    pub fn army_raid_forecast(&self, a: ArmyId, c: CityId) -> RaidCalc {
        let mut raiders = self.city_raiders(c);
        if !raiders.contains(&a) {
            raiders.push(a);
        }
        self.raid_take_calc(c, self.d.defines.raid_take_percent, &raiders)
    }

    /// Sets city `c`'s raid modifier, refreshing what reads it (as `city_change_yield_rate_modifier`
    /// does: the commerce caches, the citizens' assignment).
    fn city_set_raid_percent(&mut self, c: CityId, pct: i32) {
        if self.cities[c].raid_percent != pct {
            self.cities[c].raid_percent = pct;
            self.city_update_commerces(c);
            self.cities[c].ai_assign_work_dirty = true;
        }
    }

    /// Player `p` sets city `c`'s sally posture (section 4).
    pub fn city_set_sally(&mut self, p: PlayerId, c: CityId, posture: Sally) -> Result<(), &'static str> {
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("not your city");
        }
        self.cities[c].sally = posture;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------
    // The raid step (section 5)
    // ---------------------------------------------------------------------------------------

    /// Player `p`'s raid step (section 5; `player_do_turn`, after the fleets' standing orders):
    /// each of its cities, in city order.
    pub fn player_do_raids(&mut self, p: PlayerId) {
        for c in self.players[p].cities.clone() {
            if self.cities.contains(c) && self.cities[c].owner == p {
                self.city_do_raid(c);
            }
        }
    }

    /// City `c`'s raid step (section 5, module comment).
    fn city_do_raid(&mut self, c: CityId) {
        let d = &self.d.defines;
        let (take_pct, contested_pct, strip_turns, recover_turns) = (d.raid_take_percent, d.raid_contested_percent, d.raid_strip_turns.max(1), d.raid_recover_turns);
        // Stripped: it counts down and takes nothing.
        if self.cities[c].raid_stripped > 0 {
            self.cities[c].raid_stripped -= 1;
            self.cities[c].raid_turns = 0;
            self.city_set_raid_percent(c, 0);
            self.raid_end_all(c, "the countryside is stripped");
            return;
        }
        let owner = self.cities[c].owner;
        let city_plot = self.city_plot(c);
        // 1. Sallies, against each raiding plot in plot order.
        let plots: BTreeSet<PlotIdx> = self.city_raiders(c).into_iter().filter_map(|a| self.army_plot(a)).collect();
        let mut fought: BTreeSet<PlotIdx> = BTreeSet::new();
        for plot in plots {
            let candidates = self.engage_defenders(plot, owner, &[], WarTest::AtWar);
            let battles = self.engage_field_squads(plot, &candidates, Side::Defender, &HitCtx::sally(city_plot, plot)).len();
            for _ in 0..battles {
                // (With each army's owner: an army the battle destroys is gone before it is counted.)
                let here: Vec<(ArmyId, PlayerId)> = self.city_raiders(c).into_iter().filter(|&a| self.army_plot(a) == Some(plot)).map(|a| (a, self.armies[a].owner)).collect();
                if here.is_empty() {
                    break;
                }
                // The posture decides; a refusal ends the sallies against this plot this turn.
                let Some(sq) = self.raid_sally_decides(c, plot) else { break };
                let ord = EngageOrder { p: owner, from: city_plot, to: plot, launcher: None, squad: sq.squad, forced: None, log: None, surprise: false };
                let Ok(out) = self.engage_as(ord, Some(SallyForce { squad: sq.squad, places: sq.places })) else { break };
                fought.insert(plot);
                if self.telemetry.on() {
                    self.tel_raid_sally(owner, out.loser);
                }
                if !self.cities.contains(c) {
                    return;
                }
                // Driven off: every Raid order on that plot ends.
                if out.loser == Some(Side::Defender) {
                    for (a, raider) in here {
                        // (An army that fell to the last unit is driven off too: the battle report
                        // is its message.)
                        if self.telemetry.on() && self.armies.get(a).is_none_or(|ar| ar.station == Station::Raid(c)) {
                            self.tel_raid_driven_off(raider);
                        }
                        if self.armies.get(a).is_some_and(|ar| ar.station == Station::Raid(c)) {
                            self.armies[a].station = Station::None;
                            self.raid_order_ended(a, c, "driven off");
                        }
                    }
                    break;
                }
            }
        }
        // 2. The state.
        let raiders = self.city_raiders(c);
        if raiders.is_empty() {
            self.city_set_raid_percent(c, 0);
            self.cities[c].raid_turns = (self.cities[c].raid_turns - 1).max(0);
            return;
        }
        let contested = raiders.iter().all(|&a| self.army_plot(a).is_some_and(|q| fought.contains(&q)));
        // 3. The take.
        let pct = if contested { contested_pct } else { take_pct };
        self.city_set_raid_percent(c, pct);
        let calc = self.raid_take_calc(c, pct, &raiders);
        for s in &calc.shares {
            self.players[s.owner].gold += s.gold;
        }
        let turn = self.game_turn();
        let at = self.map.xy(city_plot);
        let city = self.d.text.name(&self.cities[c].name).to_owned();
        let state = if contested { "contested" } else { "overrun" };
        for s in &calc.shares {
            let name = self.armies[s.army].name.clone();
            let gold = s.gold;
            self.log.msg_at(turn, Some(s.owner), at, || format!("Raid on {city} ({state}): the {name} takes {gold} gold ({pct}% of production and commerce)."));
        }
        let total = calc.total;
        self.log.msg_at(turn, Some(owner), at, || format!("{city} is raided ({state}): -{pct}% production and commerce, {total} lost."));
        if self.telemetry.on() {
            self.tel_raid_take(owner, c, contested, &calc);
        }
        // Stripped (section 6).
        if !contested {
            self.cities[c].raid_turns += 1;
            if self.cities[c].raid_turns >= strip_turns {
                self.cities[c].raid_turns = 0;
                self.cities[c].raid_stripped = recover_turns.max(0);
                let mut told: Vec<PlayerId> = vec![owner];
                told.extend(raiders.iter().map(|&a| self.armies[a].owner));
                told.sort();
                told.dedup();
                for q in told {
                    self.log.msg_at(turn, Some(q), at, || format!("{city}: countryside stripped. No raid for {recover_turns} turns."));
                }
                if self.telemetry.on() {
                    self.tel_raid_stripped(owner);
                }
                self.raid_end_all(c, "the countryside is stripped");
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// The take's calculation, for the tooltips (CLAUDE.md, "a number the rules calculate shows its
// calculation"; docs/RAIDS.md, section 8)
// -------------------------------------------------------------------------------------------

impl Game {
    /// The lines of take `calc` of a raid on city `c` (made now, by [`Game::raid_take_calc`],
    /// [`Game::city_raid_calc`] or [`Game::army_raid_forecast`]): each yield's rate without the
    /// raid modifier less its rate with it (`Production: 12 at +0% less 6 at -50% = 6.`, the
    /// percents the city's modifier over 100), the loss (`Loss: 6 + 9 = 15.`), the split between
    /// several armies (`Split: 15 ÷ 2 = 7, the remainder 1 to the 1st Army.`), and each army's
    /// gold (`1st Army: 15 gold.`, `1st Army: 15 + 100% (Raiders) = 30 gold.`).
    pub fn raid_calc_lines(&self, c: CityId, calc: &RaidCalc) -> Vec<String> {
        if !self.cities.contains(c) {
            return Vec::new();
        }
        let now = self.cities[c].raid_percent;
        let line = |label: &str, y: YieldType, without: i32, with: i32, lost: i32| {
            let m = self.city_base_yield_rate_modifier(c, y, now) - 100;
            format!("{label}: {without} at {m:+}% less {with} at {:+}% = {lost}.", m - calc.percent)
        };
        let mut out = vec![
            line("Production", YieldType::YIELD_PRODUCTION, calc.production_without, calc.production_with, calc.production_lost),
            line("Commerce", YieldType::YIELD_COMMERCE, calc.commerce_without, calc.commerce_with, calc.commerce_lost),
            format!("Loss: {} + {} = {}.", calc.production_lost, calc.commerce_lost, calc.total),
        ];
        let name = |a: ArmyId| self.armies.get(a).map_or_else(|| "an army".to_string(), |ar| ar.name.clone());
        let n = calc.shares.len() as i32;
        if n > 1 {
            let rest = calc.total % n;
            let remainder = if rest > 0 { format!(", the remainder {rest} to the {}", name(calc.shares[0].army)) } else { String::new() };
            out.push(format!("Split: {} \u{f7} {n} = {}{remainder}.", calc.total, calc.total / n));
        }
        for s in &calc.shares {
            if s.modifier == 0 {
                out.push(format!("{}: {} gold.", name(s.army), s.gold));
            } else {
                let traits: Vec<&str> = self
                    .d
                    .traits
                    .keys()
                    .filter(|&t| self.players[s.owner].traits[t] && self.d.traits[t].pillaging_gold != 0)
                    .map(|t| self.d.text.name(&self.d.traits[t].base.description))
                    .collect();
                let from = if traits.is_empty() { String::new() } else { format!(" ({})", traits.join(", ")) };
                out.push(format!("{}: {} {} {}%{from} = {} gold.", name(s.army), s.share, if s.modifier < 0 { "\u{2212}" } else { "+" }, s.modifier.abs(), s.gold));
            }
        }
        out
    }
}
