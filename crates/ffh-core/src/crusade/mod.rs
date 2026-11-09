//! Crusades and allied loans: committed force for a called war (docs/CRUSADE.md).
//!
//! Not FFH2: a rules layer of this game, always on (parameters in `GameSetup::crusade`).
//!
//! A *crusade* is one called war plus its *Host*: a temporary AI player (a Host slot, added for
//! the crusade or freed by an earlier one, see [`host`]) that owns every unit lent to the war, is at war only with the *target* (and the
//! target's vassals) and is deleted at dissolution. Civs lend units ([`lend`]); a lent unit
//! counts toward its lender's *contribution* only once it has killed an enemy or died. At
//! dissolution ([`dissolve`]) the cities the Host still holds and the wealth it sacked are
//! shared by contribution in a draft (the top contributor picks first), survivors go home with
//! their promotions, and the Host slot is freed. Calls come from council war resolutions
//! ([`call`]: "dedicate or defy"); an *allied loan* ([`loan`]) is a small, time-limited
//! crusade made by agreement, its general serving the borrower's war.
//!
//! State lives in `Game` (`crusades`, `crusade_reports`, `Unit::lent`), so saves keep it; the
//! Host's own AI bookkeeping ([`HostAi`]) is plain state too, written only by its controller.

pub mod call;
pub mod dissolve;
pub mod host;
pub mod lend;
pub mod loan;

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// A crusade's id (never reused).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct CrusadeId(pub u32);

/// Ruleset parameters of crusades and loans.
/// Turn counts are at normal speed (scaled by the game speed's growth percent); strengths are
/// in [`Game::crusade_unit_strength`] units (combat strength x 100).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrusadeRules {
    /// Most turns a council crusade runs.
    pub term: i32,
    /// Turns a called civ has to deliver its pledge (answering later counts as refusing).
    pub pledge_deadline: i32,
    /// Obligation of a called civ, percent of its military strength (next to the target).
    pub obligation_percent: i32,
    /// Obligation at the farthest distance on the map, percent of the near obligation.
    pub far_obligation_percent: i32,
    /// Host AI: Muster ends when the mustered strength reaches this percent of the pledges.
    pub muster_percent: i32,
    /// Host AI: Recover when the army's average health falls below this percent.
    pub recover_health_percent: i32,
    /// Host AI: most turns spent in Recover.
    pub recover_turns: i32,
    /// Self-play measure: a lent unit that fights within this many turns of delivery "engaged".
    pub engage_turns: i32,
    /// Draft city value: gold per population point.
    pub city_pop_gold: i32,
    /// Draft city value: percent of the buildings' production cost counted as gold.
    pub city_building_percent: i32,
    /// Loans: longest duration.
    pub loan_max_turns: i32,
}

impl Default for CrusadeRules {
    fn default() -> Self {
        CrusadeRules {
            term: 40,
            pledge_deadline: 10,
            obligation_percent: 25,
            far_obligation_percent: 40,
            muster_percent: 60,
            recover_health_percent: 45,
            recover_turns: 5,
            engage_turns: 10,
            city_pop_gold: 30,
            city_building_percent: 100,
            loan_max_turns: 30,
        }
    }
}

/// What started the crusade (its *caller*).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CrusadeKind {
    /// A passed council war resolution of this vote source.
    Council(VoteSourceType),
    /// Allied loans to `borrower`'s war against the target: one general per borrower and target.
    Loan { borrower: PlayerId },
    /// Called directly (tests, future call sources); refusing costs nothing.
    Direct,
}

/// What ends the crusade early (besides its term).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Objective {
    /// Fight until the term ends.
    Term,
    /// Capture the cities on these plots (met when the target holds none of them).
    Capture(Vec<PlotIdx>),
    /// Recover the victim's cities lost to the target (met when the target holds none).
    Recover,
}

/// A called civ's answer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pledge {
    Refuse,
    /// Promised strength.
    Strength(i32),
}

/// One civ called to a crusade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub player: PlayerId,
    /// Strength it is expected to deliver.
    pub obligation: i32,
    /// `None` until it answers (no answer by the deadline = refusal).
    pub answer: Option<Pledge>,
    /// Standing charged (or the obligation met) at the deadline.
    pub settled: bool,
}

/// A civ that has handed units to the Host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lender {
    pub player: PlayerId,
    /// Handover strength of every unit lent.
    pub delivered: i32,
    /// Units lent.
    pub units: i32,
    /// Credited contribution (handover strength of the units that killed or died).
    pub credit: i32,
    /// Turn of the first credit (draft tie-break), -1 = none.
    pub first_credit_turn: i32,
    /// Turn of the first transfer.
    pub first_lend_turn: i32,
    /// The lender was not at war with the target before its first transfer (its war ends at
    /// dissolution).
    pub war_from_lending: bool,
    /// Loans: last turn of this lender's term (-1 for crusades).
    pub term_end: i32,
    /// Credits as (turn the unit was lent, strength): the bandwagon measure.
    pub credits: Vec<(i32, i32)>,
    /// Loans: the deals carrying this lender's ongoing payment (gold per turn, resources, ...),
    /// which run as long as the loan: they end with it, and a payment cancelled ends it.
    pub deals: Vec<DealId>,
}

/// Host AI state (the suggested state machine of docs/CRUSADE.md; written by the Host's
/// controller only, kept here so saved games resume it).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HostState {
    Muster,
    March,
    Engage,
    Recover,
}

/// The Host controller's bookkeeping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostAi {
    pub state: HostState,
    /// Turn the state was entered.
    pub since: i32,
    /// Muster rally point.
    pub rally: Option<PlotIdx>,
    /// Current objective plot (a target city).
    pub goal: Option<PlotIdx>,
}

/// One running crusade or loan general.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crusade {
    pub id: CrusadeId,
    pub kind: CrusadeKind,
    /// The Host player (a Host slot: `crusade_host_slot`).
    pub host: PlayerId,
    pub target: TeamId,
    /// League-defence victim, or a loan's borrower: its cities taken by the Host go back to it.
    pub victim: Option<PlayerId>,
    pub objective: Objective,
    pub start_turn: i32,
    /// Last turn: dissolution at the start of the next.
    pub end_turn: i32,
    /// Pledges are settled at the start of this turn.
    pub deadline_turn: i32,
    pub calls: Vec<Call>,
    pub lenders: Vec<Lender>,
    /// Plots of the victim's cities lost to the target (Recover; returned when retaken).
    pub victim_plots: Vec<PlotIdx>,
    /// The Host has owned a unit (it dissolves when it has none left).
    pub had_units: bool,
    /// Dissolution in progress (the Host may make peace).
    pub dissolving: bool,
    /// A loan general defending its borrower (a Defend loan serves under it).
    pub defend: bool,
    /// Strength of lent units that fought within `engage_turns` of delivery.
    pub engaged: i32,
    pub ai: HostAi,
}

/// On a unit the Host owns: who lent it and what it is worth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LentRecord {
    pub crusade: CrusadeId,
    pub lender: PlayerId,
    /// Combat strength at handover (the contribution value).
    pub strength: i32,
    /// Turn it was lent.
    pub turn: i32,
    /// Its value has been credited (first kill or death).
    pub credited: bool,
    /// Counted in `Crusade::engaged` (or past the window).
    pub engaged: bool,
}

/// Why a crusade dissolved.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dissolution {
    Objective,
    Term,
    NoUnits,
    TargetGone,
    /// A loan general whose borrower made peace or whose loans all ended.
    LoansEnded,
}

/// One lender's outcome (for the self-play measures and the log).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LenderReport {
    pub player: PlayerId,
    pub called: bool,
    pub obligation: i32,
    pub refused: bool,
    pub delivered: i32,
    pub credit: i32,
    /// Credit earned by units lent in the final quarter of the crusade.
    pub late_credit: i32,
    /// Spoils received: city values drafted (net of overdraw) plus gold paid out.
    pub spoils: i32,
    pub cities: i32,
}

/// A dissolved crusade (kept for reports).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrusadeReport {
    pub id: CrusadeId,
    pub kind: CrusadeKind,
    pub target: TeamId,
    pub start_turn: i32,
    pub end_turn: i32,
    pub reason: Dissolution,
    /// Draft pool value (city values + sacked wealth).
    pub pool: i32,
    pub wealth: i32,
    pub delivered: i32,
    pub engaged: i32,
    /// Called civs that refused or delivered nothing.
    pub refusals: i32,
    pub lenders: Vec<LenderReport>,
}

/// A strength for messages: the combat strength it stands for (strengths are x100).
pub fn strength_text(s: i32) -> String {
    let s = s.max(0);
    if s % 100 == 0 {
        format!("{}", s / 100)
    } else {
        format!("{}.{}", s / 100, (s % 100) / 10)
    }
}

/// Scale a normal-speed turn count to the game speed.
pub fn speed_turns(g: &Game, turns: i32) -> i32 {
    let percent = g.d.game_speeds[g.game_speed_type()].growth_percent;
    (turns * percent / 100).max(1)
}

impl Game {
    pub fn crusade_rules(&self) -> &CrusadeRules {
        &self.setup.crusade
    }

    /// Index of crusade `id` in `crusades`.
    pub fn crusade_index(&self, id: CrusadeId) -> Option<usize> {
        self.crusades.iter().position(|c| c.id == id)
    }

    pub fn crusade(&self, id: CrusadeId) -> Option<&Crusade> {
        self.crusades.iter().find(|c| c.id == id)
    }

    /// The crusade whose Host is `p`.
    #[inline]
    pub fn crusade_of_host(&self, p: PlayerId) -> Option<usize> {
        if self.crusades.is_empty() {
            return None;
        }
        self.crusades.iter().position(|c| c.host == p)
    }

    /// Is `p` a crusade Host (or loan general)?
    #[inline]
    pub fn player_is_crusade_host(&self, p: PlayerId) -> bool {
        self.crusade_of_host(p).is_some()
    }

    /// The crusade whose Host is on team `t`.
    #[inline]
    pub fn crusade_of_host_team(&self, t: TeamId) -> Option<usize> {
        if self.crusades.is_empty() {
            return None;
        }
        self.crusades.iter().position(|c| self.players[c.host].team == t)
    }

    #[inline]
    pub fn team_is_crusade_host(&self, t: TeamId) -> bool {
        self.crusade_of_host_team(t).is_some()
    }

    /// Is slot `p` a Host's (added for a crusade by `crusade_host_slot`, reused by later ones)?
    #[inline]
    pub fn player_is_host_slot(&self, p: PlayerId) -> bool {
        self.setup.players.get(p.idx()).is_some_and(|s| s.host)
    }

    /// A unit's contribution value: its combat strength (the better of attack and defense, with
    /// its general combat bonus) x 100, scaled by its health. Promotions earned later do not
    /// change what was recorded at handover.
    pub fn crusade_unit_strength(&self, u: UnitId) -> i32 {
        let base = self.unit_base_combat_str(u).max(self.unit_base_combat_str_defense(u));
        if base <= 0 {
            return 0;
        }
        let unit = &self.units[u];
        let max_hp = self.d.defines.max_hit_points.max(1);
        let hp = (max_hp - unit.damage).clamp(0, max_hp);
        base * (100 + unit.extra_combat_percent).max(10) * hp / max_hp
    }

    /// Can unit `u` be lent at all (a combat land unit of its own, not temporary, not aboard or
    /// carrying anything, not already serving a Host)?
    pub fn crusade_unit_lendable(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        unit.lent.is_none()
            && !unit.death_delay
            && unit.duration == 0
            && self.unit_can_fight(u)
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && unit.transport_unit.is_none()
            && unit.cargo == 0
            && self.crusade_unit_strength(u) > 0
            && !self.player_is_crusade_host(unit.owner)
            && self.unit_plot_opt(u).is_some()
    }

    /// Military strength of `p`: the strength of every unit it could lend.
    pub fn crusade_military_strength(&self, p: PlayerId) -> i32 {
        self.players[p].units.iter().filter(|&&u| self.units.contains(u) && self.crusade_unit_lendable(u)).map(|&u| self.crusade_unit_strength(u)).sum()
    }

    /// The teams the Host of crusade `idx` fights: the target and its vassals.
    pub fn crusade_enemy_teams(&self, idx: usize) -> Vec<TeamId> {
        let target = self.crusades[idx].target;
        let mut v = vec![target];
        for t in self.civ_team_ids() {
            if t != target && self.team_is_alive(t) && self.teams[t].vassal[target] {
                v.push(t);
            }
        }
        v
    }

    /// Is team `t` one the Host of crusade `idx` fights?
    pub fn crusade_is_enemy_team(&self, idx: usize, t: TeamId) -> bool {
        let target = self.crusades[idx].target;
        t == target || (self.team_is_alive(t) && self.teams[t].vassal[target])
    }

    /// The lender entry of `p` in crusade `idx`.
    pub fn crusade_lender(&self, idx: usize, p: PlayerId) -> Option<&Lender> {
        self.crusades[idx].lenders.iter().find(|l| l.player == p)
    }

    /// Units serving crusade `idx`'s Host lent by `p` (all of them for `None`).
    pub fn crusade_lent_units(&self, idx: usize, p: Option<PlayerId>) -> Vec<UnitId> {
        let host = self.crusades[idx].host;
        self.players[host]
            .units
            .iter()
            .copied()
            .filter(|&u| self.units.contains(u) && self.units[u].lent.as_ref().is_some_and(|l| p.is_none_or(|p| l.lender == p)))
            .collect()
    }

    /// Does `p` have units serving a Host at war with team `t` (no separate peace with `t`)?
    pub fn crusade_serving_against(&self, p: PlayerId, t: TeamId) -> bool {
        (0..self.crusades.len()).any(|i| {
            !self.crusades[i].dissolving && self.crusade_is_enemy_team(i, t) && !self.crusade_lent_units(i, Some(p)).is_empty()
        })
    }

    /// Value of city `c` in a draft (docs/CRUSADE.md, "City value"): population plus buildings,
    /// in gold.
    pub fn crusade_city_value(&self, c: CityId) -> i32 {
        let d = self.d;
        let rules = self.crusade_rules();
        let city = &self.cities[c];
        let mut value = city.population * rules.city_pop_gold;
        let construct = d.game_speeds[self.game_speed_type()].construct_percent;
        let mut buildings = 0i64;
        for b in d.buildings.keys() {
            let n = city.num_real_building[b];
            if n > 0 && d.buildings[b].production_cost > 0 {
                buildings += (d.buildings[b].production_cost as i64) * (n as i64) * (construct as i64) / 100;
            }
        }
        value += (buildings * rules.city_building_percent as i64 / 100) as i32;
        value.max(1)
    }

    /// Is `c` (now held by the Host of crusade `idx`) one of the victim's own cities?
    pub(crate) fn crusade_is_victim_city(&self, idx: usize, c: CityId) -> bool {
        let Some(victim) = self.crusades[idx].victim else { return false };
        let plot = self.city_plot(c);
        self.cities[c].original_owner == victim || self.crusades[idx].victim_plots.contains(&plot)
    }
}
