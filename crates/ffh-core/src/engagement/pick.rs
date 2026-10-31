//! Who can fight an engagement, the stance each side fights with, and the pick of its force (plan
//! 5.2 to 5.4).
//!
//! **Who attacks** ([`Game::unit_can_engage`]): a land unit next to the land plot it attacks,
//! able to attack, with moves left, not marching, not a missile, that may still attack this turn
//! (one attack a turn, Blitz one more: [`Game::unit_has_attack_left`], counting engagements,
//! bombards and missile strikes) and that the move rules let attack the plot
//! (`unit_can_move_into` with its attack gates under the option: the attack count, and for a
//! unit with a combat limit some defender below the limit and no amphibious attack:
//! [`Game::unit_engage_limit_ok`]). The
//! attacker's pool ([`Game::engage_attackers`]) is every such unit of the player on the plot it
//! attacks from: its armies, its garrison, its heroes, mages and recon units.
//!
//! **Who defends** ([`Game::engage_defenders`]): every unit on the plot that can defend there,
//! is an enemy of the attacking side as `plot_get_best_defender` tests it (both ways, so hidden
//! nationality works as in the duel) and is not invisible to the attacking team (G5); a
//! declare-war preview also counts every team the declaration would bring into the war
//! ([`WarTest::DeclareWar`]). Ships never
//! defend land; marching units and immobile units with strength do. A unit defends any number of
//! times a turn. Under the option `plot_get_best_defender` (and its "better defender" ratings)
//! no longer picks the land defender; it stays for its other callers.
//!
//! **Stances** (5.3): an army's stance is its standing order; a city's garrison stance is its
//! garrison's. The attacker's stance ([`Game::engage_attack_stance`]) proposes its force; the
//! defender's ([`Game::engage_defense_stance`]) picks the defender's.
//!
//! **The pick** ([`Game::engage_auto_pick`], 5.4): deterministic, no RNG, the same for humans and
//! the AI. Candidates sort by (healthy first, job strength, HP, unit id); the stance's steps take
//! the best of their roles up to each step's target, then its fill takes the rest in its role
//! order, held units (Preserve Veterans: above the stance's level, heroes) last of all. The
//! attacker's proposal ([`Game::engage_propose_force`]) starts from that pick; from step 7 on it
//! also tries the pick's best single swaps by the estimate (plan 15, item 3:
//! [`Game::engage_swap_search`], which keeps the quota pick until then). A force the attacker
//! chooses itself is checked by [`Game::engage_validate_force`].

use ffh_data::*;

use super::battle::Side;
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;

/// Which war an engagement test assumes (plan 5.2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WarTest {
    /// The real fight: the defenders are the attacking side's enemies now (at war, or hostile:
    /// hidden nationality).
    AtWar,
    /// The preview of a move that declares war on `team`: the units of every team the
    /// declaration would bring into the war count as enemies too (`team`, its vassals and master,
    /// its defensive-pact partners and theirs: [`Game::team_wars_after_declaring`]), as the real
    /// fight after the declaration counts them.
    DeclareWar(TeamId),
}

/// The stance a side fights with and whose it is (plan 5.3; a report side's `stance`, `army` and
/// `garrison_of`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StanceChoice {
    pub stance: StanceType,
    /// The army whose stance it is.
    pub army: Option<ArmyId>,
    /// The city whose garrison's stance it is.
    pub garrison_of: Option<CityId>,
}

impl Game {
    /// Whether unit `u` may make another attack this turn under engagements (plan 5.2): one a
    /// turn, Blitz one more. Engagements, bombards and missile strikes count
    /// (`Unit::attacks_made`); any other attack this turn (a duel: boarding a ship) counts as one.
    pub fn unit_has_attack_left(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        let spent = unit.attacks_made.max(unit.made_attack as i32);
        spent < 1 + self.unit_is_blitz(u) as i32
    }

    /// Whether unit `u` can attack plot `to` in an engagement (module comment; plan 5.2). Cheap:
    /// no best-defender search (a plot scan only for a unit with a combat limit).
    pub fn unit_can_engage(&self, u: UnitId, to: PlotIdx, war: WarTest) -> bool {
        if !self.engage_applies(u, to) {
            return false;
        }
        let Some(at) = self.unit_plot_opt(u) else { return false };
        let ((fx, fy), (tx, ty)) = (self.map.xy(at), self.map.xy(to));
        if self.step_distance(fx, fy, tx, ty) != 1 {
            return false;
        }
        let unit = &self.units[u];
        if unit.march.is_some() || self.unit_is_suicide(u) || !self.unit_can_attack(u) || !self.unit_can_move(u) {
            return false;
        }
        let declare_war = matches!(war, WarTest::DeclareWar(_));
        // `unit_can_move_into`'s attack branch tests the attack count and the combat limit
        // against the defenders at war (`unit_engage_limit_ok`); a declare-war preview also tests
        // the limit against the teams it would fight.
        if declare_war && !self.unit_engage_limit_ok(u, to, war) {
            return false;
        }
        self.unit_can_move_into(u, to, true, declare_war, false)
    }

    /// The combat-limit gate of an engagement (plan 5.2; C++ `canAttack(defender)`'s rules, on
    /// every defender instead of the best one), for a unit with a combat limit (siege): from the
    /// water it attacks no plot holding a unit it would meet, a non-combatant included (no
    /// amphibious artillery, as today: the duel's best defender may be a lone Worker); otherwise
    /// one of the units it would meet must be below its limit: a defender when the plot has any,
    /// else a non-combatant (as today). Units without a limit pass at once.
    pub(crate) fn unit_engage_limit_ok(&self, u: UnitId, to: PlotIdx, war: WarTest) -> bool {
        let limit = self.units[u].combat_limit;
        if limit >= 100 {
            return true;
        }
        let from_water = self.unit_plot_opt(u).is_some_and(|at| self.plot_is_water(at));
        let team = self.players[self.units[u].owner].team;
        let declared = self.engage_declared(team, war);
        let (mut defenders, mut others, mut other_below) = (false, false, false);
        for &d in &self.map.plots[to].units {
            if !self.engage_meets(d, to, team, std::slice::from_ref(&u), declared.as_ref()) {
                continue;
            }
            if from_water {
                return false;
            }
            let below = self.units[d].damage < limit;
            if self.engage_can_defend(d, to) {
                if below {
                    return true;
                }
                defenders = true;
            } else {
                others = true;
                other_below |= below;
            }
        }
        !defenders && (!others || other_below)
    }

    /// The attacker's pool (plan 5.2): every unit of `p` on `from` that can attack `to` in an
    /// engagement, in the plot's order.
    pub fn engage_attackers(&self, p: PlayerId, from: PlotIdx, to: PlotIdx, war: WarTest) -> Vec<UnitId> {
        self.map.plots[from].units.iter().copied().filter(|&u| self.units[u].owner == p && self.unit_can_engage(u, to, war)).collect()
    }

    /// The defender's candidates on `to` against `p`'s attack with `force` (plan 5.2; the module
    /// comment), in the plot's order. `force` decides hidden nationality: a force all of whose
    /// units are hostile to everyone fights any team's units, as such an attacker does in the
    /// duel; an empty force fights only `p`'s enemies.
    pub fn engage_defenders(&self, to: PlotIdx, p: PlayerId, force: &[UnitId], war: WarTest) -> Vec<UnitId> {
        let team = self.players[p].team;
        let declared = self.engage_declared(team, war);
        self.map.plots[to]
            .units
            .iter()
            .copied()
            .filter(|&d| self.engage_meets(d, to, team, force, declared.as_ref()) && self.engage_can_defend(d, to))
            .collect()
    }

    /// The teams a `war` test counts as enemies of `team` beyond its wars now: none for the real
    /// fight; for a declare-war preview every team the declaration brings in.
    fn engage_declared(&self, team: TeamId, war: WarTest) -> Option<IdxVec<TeamId, bool>> {
        match war {
            WarTest::AtWar => None,
            WarTest::DeclareWar(t) => Some(self.team_wars_after_declaring(team, t)),
        }
    }

    /// Whether `team`'s attack with `force` on `to` meets unit `d` there, combatant or not: not
    /// its own, alive, not invisible to it (G5), and an enemy as `plot_get_best_defender` tests
    /// it (both ways: hidden nationality), or of a team in `declared` ([`Game::engage_declared`]).
    fn engage_meets(&self, d: UnitId, to: PlotIdx, team: TeamId, force: &[UnitId], declared: Option<&IdxVec<TeamId, bool>>) -> bool {
        let d_team = self.unit_get_team(d);
        if d_team == team || self.unit_is_dead(d) || self.unit_is_invisible(d, team, true) {
            return false;
        }
        self.unit_is_enemy(d, Some(team), Some(to))
            || (!force.is_empty() && force.iter().all(|&a| self.unit_is_enemy(a, Some(d_team), Some(to))))
            || declared.is_some_and(|w| w[d_team])
    }

    /// Whether a unit the attack meets on `to` is one of its defenders ([`Game::engage_defenders`]):
    /// it can defend there and has a role.
    fn engage_can_defend(&self, d: UnitId, to: PlotIdx) -> bool {
        self.unit_can_defend(d, Some(to)) && self.unit_combat_role(d).is_some()
    }

    /// The stance `p`'s attack from `from` proposes its force with (plan 5.3): the launching
    /// group's army's; a garrison unit's: its garrison's; a group in no army (heroes, mages,
    /// recon) or no launcher: the largest army of `p` on `from` (most units, then the lowest id),
    /// else the garrison's of `p`'s city on `from`, else the default. The barbarians fight with
    /// the default.
    pub fn engage_attack_stance(&self, p: PlayerId, from: PlotIdx, launcher: Option<GroupId>) -> StanceChoice {
        let default = StanceChoice { stance: self.d.default_stance, army: None, garrison_of: None };
        if self.player_is_barbarian(p) {
            return default;
        }
        if let Some(g) = launcher.filter(|&g| self.groups.contains(g)) {
            let units = &self.groups[g].units;
            if let Some(a) = units.iter().filter_map(|&u| self.units[u].army).find(|&a| self.armies.contains(a) && self.armies[a].owner == p) {
                return StanceChoice { stance: self.armies[a].stance, army: Some(a), garrison_of: None };
            }
            if let Some(c) = units.iter().find_map(|&u| self.unit_garrison_city(u)) {
                return StanceChoice { stance: self.cities[c].garrison_stance, army: None, garrison_of: Some(c) };
            }
        }
        if let Some(a) = self.engage_largest_army(self.map.plots[from].units.iter().copied().filter(|&u| self.units[u].owner == p)) {
            return StanceChoice { stance: self.armies[a].stance, army: Some(a), garrison_of: None };
        }
        match self.map.plots[from].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == p) {
            Some(c) => StanceChoice { stance: self.cities[c].garrison_stance, army: None, garrison_of: Some(c) },
            None => default,
        }
    }

    /// The stance the defenders `candidates` on `to` fight with (plan 5.3, the spec): on a city's
    /// plot its garrison's (even when field armies stand in it); otherwise the largest army among
    /// the candidates' (most of them, then the lowest id); else the default. Marching units,
    /// heroes and other units in no army count for no army.
    pub fn engage_defense_stance(&self, to: PlotIdx, candidates: &[UnitId]) -> StanceChoice {
        if let Some(c) = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c)) {
            return StanceChoice { stance: self.cities[c].garrison_stance, army: None, garrison_of: Some(c) };
        }
        match self.engage_largest_army(candidates.iter().copied()) {
            Some(a) => StanceChoice { stance: self.armies[a].stance, army: Some(a), garrison_of: None },
            None => StanceChoice { stance: self.d.default_stance, army: None, garrison_of: None },
        }
    }

    /// The army with the most of `units` (then the lowest id), if any of them is in one.
    fn engage_largest_army(&self, units: impl Iterator<Item = UnitId>) -> Option<ArmyId> {
        let mut counts: Vec<(ArmyId, i32)> = Vec::new();
        for u in units {
            let Some(a) = self.units[u].army.filter(|&a| self.armies.contains(a) && self.armies[a].owner == self.units[u].owner) else { continue };
            match counts.iter_mut().find(|(b, _)| *b == a) {
                Some((_, n)) => *n += 1,
                None => counts.push((a, 1)),
            }
        }
        counts.into_iter().max_by(|(a, n), (b, m)| n.cmp(m).then(b.cmp(a))).map(|(a, _)| a)
    }

    /// Whether `stance` holds unit `u` back (plan 2.2, Preserve Veterans): above its level, or a
    /// hero when it holds heroes. A held unit is committed only when the slots would otherwise
    /// go empty.
    pub fn engage_is_held(&self, u: UnitId, stance: StanceType) -> bool {
        let info = &self.d.stances[stance];
        (info.max_level >= 0 && self.units[u].level > info.max_level) || (info.hold_heroes && self.unit_is_hero(u))
    }

    /// The stance's pick of side `side`'s force from `pool` (distinct units: a pool or the
    /// defender's candidates; plan 5.4, the module comment), in the order picked: up to the side's slots (for the attacker, at most its ordinary slots'
    /// worth of units without `bRiver`). Deterministic, no RNG. `ctx` is the attack (the job
    /// strengths' plot).
    pub fn engage_auto_pick(&self, pool: &[UnitId], stance: StanceType, slots: &Slots, side: Side, ctx: &HitCtx) -> Vec<UnitId> {
        let d = self.d;
        let info = &d.stances[stance];
        let s = slots.max(side).max(0) as usize;
        let ordinary = slots.ordinary(side).max(0) as usize;
        let min_hp = d.defines.engagement_auto_min_hp_percent;

        struct Cand {
            u: UnitId,
            role: CombatRoleType,
            held: bool,
            river: bool,
            taken: bool,
        }
        // Sorted by (healthy desc, job desc, HP desc, id asc).
        // Phase 7: a defending Guardsman sorts first (its "better defender" rating, Q6).
        let mut keyed: Vec<((bool, i32, i32, UnitId), Cand)> = pool
            .iter()
            .filter_map(|&u| {
                let role = self.unit_combat_role(u)?;
                let damage = self.units[u].damage;
                let job = match side {
                    Side::Attacker => self.engage_job_attack(u, ctx),
                    Side::Defender => self.engage_job_defense(u, ctx),
                };
                let hp = super::hp_from_damage(damage, self.unit_engage_max_hp(u));
                let key = (100 - damage >= min_hp, job, hp, u);
                let river = side == Side::Attacker && self.unit_is_river(u);
                Some((key, Cand { u, role, held: self.engage_is_held(u, stance), river, taken: false }))
            })
            .collect();
        keyed.sort_by(|(a, _), (b, _)| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)).then(a.3.cmp(&b.3)));
        let mut cands: Vec<Cand> = keyed.into_iter().map(|(_, c)| c).collect();

        let mut pick: Vec<UnitId> = Vec::with_capacity(s);
        let mut non_river = 0usize;
        // Takes candidate `i` if the slots allow it.
        let mut take = |cands: &mut Vec<Cand>, pick: &mut Vec<UnitId>, i: usize| -> bool {
            let c = &mut cands[i];
            if c.taken || pick.len() >= s || (!c.river && non_river >= ordinary) {
                return false;
            }
            c.taken = true;
            non_river += !c.river as usize;
            pick.push(c.u);
            true
        };

        // The steps: each takes the best non-held units of its roles until the pick holds
        // max(count, ceil(S x percent / 100)) of them.
        for step in &info.steps {
            let target = (step.count.max(0) as usize).max((s * step.percent.max(0) as usize).div_ceil(100));
            let mut have = cands.iter().filter(|c| c.taken && step.roles.contains(&c.role)).count();
            for i in 0..cands.len() {
                if have >= target || pick.len() >= s {
                    break;
                }
                if !cands[i].held && step.roles.contains(&cands[i].role) && take(&mut cands, &mut pick, i) {
                    have += 1;
                }
            }
        }

        // The fill, in two tiers: the non-held units, then the held ones; each in the fill's role
        // order, then any role it does not list (so fewer candidates than slots are all taken).
        for held in [false, true] {
            for role in info.fill.iter().map(Some).chain(std::iter::once(None)) {
                for i in 0..cands.len() {
                    if pick.len() >= s {
                        return pick;
                    }
                    let c = &cands[i];
                    let of_role = match role {
                        Some(r) => c.role == *r,
                        None => !info.fill.contains(&c.role),
                    };
                    if c.held == held && of_role {
                        take(&mut cands, &mut pick, i);
                    }
                }
            }
        }
        pick
    }

    /// The force `p`'s attack from `ctx.from` on `ctx.to` proposes from `pool` under `stance`
    /// (plan 5.4 and 15, item 3): the stance's pick ([`Game::engage_auto_pick`]), improved by
    /// [`Game::engage_swap_search`]. The AI's attacks use it too.
    pub fn engage_propose_force(&self, pool: &[UnitId], stance: StanceType, slots: &Slots, ctx: &HitCtx) -> Vec<UnitId> {
        let pick = self.engage_auto_pick(pool, stance, slots, Side::Attacker, ctx);
        self.engage_swap_search(pool, stance, slots, ctx, pick)
    }

    /// **Hook for step 7** (plan 15, item 3: the attacker's auto-pick consults the estimate):
    /// estimate the quota `pick` and each of its single swaps ([`Game::engage_single_swaps`])
    /// against the defender's likely pick, and keep the best plan. Until the estimator
    /// (`engage_estimate`) lands it returns the quota pick unchanged. The defender's pick stays
    /// the blind stance pick (the spec); a held unit is swapped in only by the player.
    pub fn engage_swap_search(&self, _pool: &[UnitId], _stance: StanceType, _slots: &Slots, _ctx: &HitCtx, pick: Vec<UnitId>) -> Vec<UnitId> {
        pick
    }

    /// The single swaps of the attacker's `pick` (plan 15, item 3; the plan screen's "best
    /// single swaps"): `(out, in)`, a committed unit out and a unit of `pool` not in the pick in
    /// its place, every one that keeps the force valid, in pick order then pool order. Units
    /// `stance` holds are never swapped in (only the player commits them). A pick short of the
    /// slots has no swaps that add a unit: the pick already took every unit the slots allow.
    pub fn engage_single_swaps(&self, pool: &[UnitId], pick: &[UnitId], stance: StanceType, slots: &Slots) -> Vec<(UnitId, UnitId)> {
        let ordinary = slots.att().max(0) as usize;
        let non_river = pick.iter().filter(|&&u| !self.unit_is_river(u)).count();
        let mut swaps = Vec::new();
        for &out in pick {
            let out_river = self.unit_is_river(out);
            for &r in pool {
                if pick.contains(&r) || self.engage_is_held(r, stance) {
                    continue;
                }
                // Without bRiver it takes an ordinary slot: fine when the unit out had one.
                if !self.unit_is_river(r) && out_river && non_river >= ordinary {
                    continue;
                }
                swaps.push((out, r));
            }
        }
        swaps
    }

    /// Checks a force the attacker chose (the `engage` command's, an AI's; plan 5.4): every unit
    /// in its `pool` ([`Game::engage_attackers`]), none twice, at most the slots, at most the
    /// ordinary slots' worth of units without `bRiver`. The error is the reason the UI shows.
    pub fn engage_validate_force(&self, pool: &[UnitId], force: &[UnitId], slots: &Slots) -> Result<(), &'static str> {
        if force.is_empty() {
            return Err("no units to attack with");
        }
        for (i, &u) in force.iter().enumerate() {
            if force[..i].contains(&u) {
                return Err("a unit is in the force twice");
            }
            if !pool.contains(&u) {
                return Err("a unit in the force cannot attack this plot");
            }
        }
        if force.len() as i32 > slots.max(Side::Attacker) {
            return Err("more units than the battle has slots for");
        }
        if force.iter().filter(|&&u| !self.unit_is_river(u)).count() as i32 > slots.att() {
            return Err("the river slots are only for units that cross rivers freely");
        }
        Ok(())
    }
}
