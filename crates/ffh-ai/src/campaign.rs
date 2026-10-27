//! Siege campaigns: coordinated multi-stack operations against enemy and barbarian cities.
//!
//! One operation per player at a time, recomputed every turn from the game state (nothing is
//! remembered outside `Game`, so a saved game resumed with a fresh controller plays on
//! identically):
//!
//! - **Target** (`choose_target`): at war (or preparing one), an enemy city; in peacetime, a
//!   barbarian city within 14 plots of ours; the best value (population, wonders, holy city,
//!   resources, a barbarian town near our cities) for its cost (the force it needs against what
//!   we see of its defenders, the distance). Every order of an operation carries its target as
//!   the group's mission AI (`MISSIONAI_ASSAULT`, saved with the game), and the city our stacks
//!   are committed to keeps its place while its force stays within reach: without that, stacks
//!   walked back and forth between two targets.
//! - **Commitment**: at war every stack on the landmass within 24 plots; against barbarians the
//!   nearest stacks until they hold twice the force the city needs.
//! - **Force** ([`simulate`]): the strike played out in expected values with the rules' combat
//!   odds (fortification, city defense, first strikes): collateral attackers first, then the
//!   strongest; every fight takes the defender's expected health down (a lost fight still
//!   wounds), collateral damage hits the other defenders. The force covers the city when the
//!   defenders fall with an attacker to spare; siege engines that can bring the walls down
//!   within a few turns count as having done so. With switch `SIM_FAITHFUL`
//!   ([`simulate_in_order`]) the strike is played as it will be fought -- the attackers in the
//!   strike's own order, each against the defender the rules put up against it, a defender
//!   fallen when it is 90% sure to be dead (70% once positioned) -- and the same simulation, in
//!   the rules' group-attack order, decides every field attack (`FIELD_SIM`,
//!   `military::field_go`).
//! - **Phases** ([`Phase`]): while the committed force falls short, stacks gather in the rally
//!   city (our city nearest the target; against barbarians they stay where they are) and
//!   production builds what is missing ([`army_choice`]); once it covers the city, stacks march
//!   to an assembly plot two plots out (the best defensive ground), one turn at a time, step
//!   next to the city together from there when the force there covers it, and strike when the
//!   force next to it does: every committed stack there attacks
//!   in the same turn (`strike`), siege engines bombarding first, then collateral attackers,
//!   then units summoned for good (skeletons, free to replace: siege fodder), then the best odds
//!   (or the cheapest unit to lose), while the remaining force still covers
//!   the remaining defenders. The victor does not stay alone in the city: up to three more
//!   units follow it in. Stacks past gathering neither turn back for our cities in peril (the
//!   threat usually comes from their target) nor stop to heal just before the strike.
//! - **No hopeless camping**: a stack next to the target whose operation cannot strike steps
//!   back to the assembly plot (or the rally city while the force falls short); stacks not in
//!   the operation never campaign on their own (`military.rs` hunts only cities it can take).
//! - **Production** ([`army_choice`], one hook in `production.rs`): attackers and siege engines
//!   up to [`needed_force`], only attackers that can hurt the target's garrison.

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::unit::CombatDetails;
use ffh_core::utils::{combat_odds, CombatOddsInput};
use ffh_data::*;

use crate::military::is_army_unit;
use crate::production::{best_unit, count_ai, Choice, Role};
use crate::units::{alive, skip};
use crate::view::{attack_power, View};

/// Operation phase (recomputed every turn).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The force falls short (or the war is not declared yet): gather in the rally city.
    Gather,
    /// March to the assembly plot two plots out and wait there for the others.
    March,
    /// The force at the assembly plot covers the city: step next to it together.
    StepIn,
    /// The force next to the city covers it: attack this turn.
    Strike,
}

/// One operation: a target city and the stacks committed to it.
#[derive(Clone, Debug)]
pub struct Op {
    pub city: CityId,
    pub plot: PlotIdx,
    /// A civ city (war) rather than a barbarian one.
    pub civ: bool,
    pub phase: Phase,
    /// Committed groups, nearest first.
    pub groups: Vec<GroupId>,
    /// Our city nearest the target (gathering point while the force falls short).
    pub rally: Option<PlotIdx>,
    /// Best defensive plot two plots from the target, on our side.
    pub assembly: Option<PlotIdx>,
    /// Plot next to the target the stacks step onto (next to the assembly plot).
    pub staging: Option<PlotIdx>,
    /// The strike of this turn has been made.
    pub struck: bool,
    /// At war with the target's owner (else we only prepare).
    pub at_war: bool,
    /// Units the target needs (estimate) and units committed.
    pub need: i32,
    pub have: i32,
}

/// This turn's operations (built on the first army decision of the turn, see [`plan`]).
#[derive(Default)]
pub struct Plan {
    pub built: bool,
    pub ops: Vec<Op>,
}

impl Plan {
    pub(crate) fn op_of(&self, grp: GroupId) -> Option<usize> {
        self.ops.iter().position(|o| o.groups.contains(&grp))
    }

    /// The city our war operation targets.
    pub fn war_target(&self) -> Option<CityId> {
        self.ops.iter().find(|o| o.civ).map(|o| o.city)
    }
}

// ----------------------------------------------------------------------------
// Fight estimates
// ----------------------------------------------------------------------------

/// Enemy units able to defend city plot `tp` that we know of (the garrison where
/// [`crate::view::sees_units`]: fog included with `FOG_ATTACK`, never stealthed units; the city
/// owner's units count before the war is declared too).
pub(crate) fn visible_defenders(g: &Game, team: TeamId, tp: PlotIdx) -> Vec<UnitId> {
    if !crate::view::sees_units(g, team, tp) {
        return Vec::new();
    }
    let owner = g.map.plots[tp].plot_city.map(|c| g.players[g.cities[c].owner].team);
    g.map.plots[tp]
        .units
        .iter()
        .copied()
        .filter(|&e| {
            let et = g.unit_get_team(e);
            et != team && (g.at_war(team, et) || Some(et) == owner) && g.unit_can_defend(e, Some(tp)) && !g.unit_is_invisible(e, team, false)
        })
        .collect()
}

/// `apply_combat_modifier` of the rules, inverted and reapplied: a strength computed with
/// modifier `from` recomputed with modifier `to`.
fn remodify(strength: i32, from: i32, to: i32) -> i32 {
    let base = if from > 0 { strength as i64 * 100 / (from as i64 + 100) } else { strength as i64 * (100 - from as i64) / 100 };
    let s = if to > 0 { base * (to as i64 + 100) / 100 } else { base * 100 / (100 - to as i64) };
    (s as i32).max(1)
}

/// One fight of the simulation: attacker `a` against defender `d` (on `dp`) at `hp` hit points
/// with `wall_cut` points of city defense bombarded away. Returns (odds per mille, expected
/// damage `a` deals when it loses). The odds are the rules' `getCombatOdds` with those two
/// changes.
fn fight(g: &Game, a: UnitId, d: UnitId, dp: PlotIdx, hp: i32, wall_cut: i32) -> (i32, i32) {
    pair(g, a, d, dp, wall_cut).fight(g, hp)
}

/// One attacker against one defender: what [`fight`] needs that does not change as the defender
/// loses health (the simulation computes it once per pair, then fights at any health).
#[derive(Copy, Clone, Debug)]
struct Pair {
    /// The defender's strength against this attacker at full health, walls cut.
    d_max: i32,
    d_max_hp: i32,
    a_str: i32,
    a_fp: i32,
    a_hp: i32,
    a_limit: i32,
    a_fs: i32,
    a_cfs: i32,
    a_immune: bool,
    d_fs: i32,
    d_cfs: i32,
    d_immune: bool,
    /// The defender is of a world unit class (the rules halve it when choosing a defender).
    d_world: bool,
    /// Switch `STRIKE_DAMAGE` for the attacker's owner.
    first_strikes: bool,
    /// The attacker's strength and firepower without regard to the defender, and its withdrawal
    /// chance: what the rules' own estimate uses ([`Pair::ai_odds`]).
    ai_str: i32,
    ai_fp: i32,
    withdraw: i32,
}

fn pair(g: &Game, a: UnitId, d: UnitId, dp: PlotIdx, wall_cut: i32) -> Pair {
    let mut det = CombatDetails::default();
    let mut d_max = g.unit_max_combat_str(d, Some(dp), Some(a), Some(&mut det));
    if wall_cut > 0 {
        d_max = remodify(d_max, det.modifier_total, det.modifier_total - wall_cut);
    }
    Pair {
        d_max,
        d_max_hp: g.unit_max_hit_points(d).max(1),
        a_str: g.unit_curr_combat_str(a, None, Some(d), None),
        a_fp: g.unit_curr_firepower(a, None, Some(d)),
        a_hp: g.unit_curr_hit_points(a),
        a_limit: g.units[a].combat_limit,
        a_fs: g.unit_first_strikes(a),
        a_cfs: g.unit_chance_first_strikes(a),
        a_immune: g.unit_immune_to_first_strikes(a),
        d_fs: g.unit_first_strikes(d),
        d_cfs: g.unit_chance_first_strikes(d),
        d_immune: g.unit_immune_to_first_strikes(d),
        d_world: ffh_core::utils::is_world_unit_class(g.d, g.unit_get_unit_class_type(d)),
        first_strikes: crate::switches::STRIKE_DAMAGE.on(g, g.units[a].owner),
        ai_str: g.unit_curr_combat_str(a, None, None, None),
        ai_fp: g.unit_curr_firepower(a, None, None),
        withdraw: g.unit_withdrawal_probability(a),
    }
}

impl Pair {
    /// [`fight`] at `hp` hit points.
    fn fight(&self, g: &Game, hp: i32) -> (i32, i32) {
        self.fight_capped(g, hp, false)
    }

    /// [`fight`] at `hp` hit points; with `cap`, a lost fight deals at most a round short of the
    /// defender's death (had it dealt more, it would have won).
    fn fight_capped(&self, g: &Game, hp: i32, cap: bool) -> (i32, i32) {
        let max_hp = self.d_max_hp;
        let hp = hp.clamp(1, max_hp);
        let d_max = self.d_max;
        let d_str = (d_max * hp / max_hp).max(1);
        let d_fp = (d_max + d_str + 1) / 2;
        let (a_str, a_fp, a_hp) = (self.a_str, self.a_fp, self.a_hp);
        if a_str <= 0 {
            return (0, 0);
        }
        let k = CombatOddsInput {
            attacker_strength: a_str,
            attacker_firepower: a_fp,
            defender_strength: d_str,
            defender_firepower: d_fp,
            attacker_hit_points: a_hp,
            defender_hit_points: hp,
            defender_max_hit_points: max_hp,
            attacker_combat_limit: self.a_limit,
            attacker_first_strikes: self.a_fs,
            attacker_chance_first_strikes: self.a_cfs,
            attacker_immune_to_first_strikes: self.a_immune,
            defender_first_strikes: self.d_fs,
            defender_chance_first_strikes: self.d_cfs,
            defender_immune_to_first_strikes: self.d_immune,
        };
        let defs = &g.d.defines;
        let odds = combat_odds(&k, defs.combat_die_sides, defs.combat_damage);
        // A lost fight: the rounds the attacker wins before it has lost all its hit points.
        let sf = (a_fp + d_fp + 1) / 2;
        let to_att = (defs.combat_damage * (d_fp + sf) / (a_fp + sf)).max(1);
        let to_def = (defs.combat_damage * (a_fp + sf) / (d_fp + sf)).max(1);
        let q = a_str as i64 * 1000 / (a_str + d_str) as i64; // per mille a round goes our way
        // First strikes (switch `STRIKE_DAMAGE`; `CvUnit::resolveCombat`): while a side has first
        // strikes left it cannot be hurt, and both count down each round, so the side with more
        // has that many rounds in which only the other is hurt (a chance first strike counts
        // half). Left out, a Moroi at strength 7 against Acheron (19, four first strikes) was
        // expected to take 12 hit points off him, and dealt none: it dies in the rounds he
        // cannot be hurt.
        let (mut a_hp_left, mut free_wound) = (a_hp as i64, 0i64);
        if self.first_strikes {
            let fs_d = if self.a_immune { 0 } else { self.d_fs as i64 * 2 + self.d_cfs as i64 };
            let fs_a = if self.d_immune { 0 } else { self.a_fs as i64 * 2 + self.a_cfs as i64 };
            let net2 = fs_d - fs_a; // twice the defender's free rounds (negative: ours)
            if net2 > 0 {
                a_hp_left -= net2 * (1000 - q) * to_att as i64 / 2000;
            } else {
                free_wound = -net2 * q * to_def as i64 / 2000;
            }
        }
        let losses = if a_hp_left > 0 { (a_hp_left + to_att as i64 - 1) / to_att as i64 } else { 0 };
        let won = losses * q / (1000 - q).max(50);
        let mut wound = (won * to_def as i64 + free_wound).min(hp as i64) as i32;
        if cap {
            wound = wound.min((hp + to_def - 1) / to_def * to_def - to_def).max(0);
        }
        // Combat limit (siege engines stop at a share of the defender's health).
        let limit = self.a_limit;
        if limit < max_hp {
            wound = wound.min((hp - (max_hp - limit)).max(0));
        }
        (odds, wound)
    }

    /// The rules' own odds estimate, out of 100, with the defender at `hp` hit points (C++
    /// `CvUnitAI::AI_attackOdds`, which `AI_getBestGroupAttacker` picks the attacker by: the
    /// attacker's strength without regard to the defender, first strikes as rounds, withdrawal
    /// counted as a win).
    fn ai_odds(&self, g: &Game, hp: i32) -> i32 {
        let mut ours = self.ai_str;
        if ours == 0 {
            return 1;
        }
        let hp = hp.clamp(0, self.d_max_hp);
        let mut theirs = self.d_max * hp / self.d_max_hp;
        let their_fp = (self.d_max + theirs + 1) / 2;
        let base = 100 * ours / (ours + theirs).max(1);
        if base == 0 {
            return 1;
        }
        let sf = (self.ai_fp + their_fp + 1) / 2;
        let damage = g.d.defines.combat_damage;
        let to_us = 1.max(damage * (their_fp + sf) / (self.ai_fp + sf).max(1));
        let to_them = 1.max(damage * (self.ai_fp + sf) / (their_fp + sf).max(1));
        let hit_limit = self.d_max_hp - self.a_limit;
        let mut rounds_us = (0.max(hp - hit_limit) + to_them - 1) / to_them;
        let mut rounds_them = (0.max(self.a_hp) + to_us - 1) / to_us;
        if !self.d_immune {
            rounds_us -= (base * self.a_fs + base * self.a_cfs / 2) / 100;
        }
        if !self.a_immune {
            rounds_them -= ((100 - base) * self.d_fs + (100 - base) * self.d_cfs / 2) / 100;
        }
        let diff = rounds_us.max(1) - rounds_them.max(1);
        if diff > 0 {
            theirs *= 1 + diff;
        } else {
            ours *= 1 - diff;
        }
        let mut odds = ours * 100 / (ours + theirs).max(1);
        odds += (100 - odds) * self.withdraw / 100;
        odds.clamp(1, 99)
    }

    /// How the rules rank this defender against this attacker at `hp` hit points
    /// (`CvUnit::isBetterDefenderThan`: its strength against the attacker at its health, first
    /// strikes weighed, a world unit halved; ties and the rarer tests left out).
    fn rank(&self, g: &Game, hp: i32) -> i64 {
        let fsd = (g.d.defines.combat_damage * 2 / 5) as i64;
        let mut s = self.d_max as i64 * hp.clamp(0, self.d_max_hp) as i64 / self.d_max_hp as i64;
        if !self.a_immune {
            s = s * ((self.d_fs as i64 * 2 + self.d_cfs as i64) * fsd + 100) / 100;
        }
        if self.d_immune {
            s = s * ((self.a_fs as i64 * 2 + self.a_cfs as i64) * fsd + 100) / 100;
        }
        if self.d_world {
            s /= 2;
        }
        s
    }
}

/// Expected outcome of a strike.
#[derive(Copy, Clone, Debug, Default)]
pub struct Sim {
    /// Attackers used until the defenders fell (`None`: they hold).
    pub used: Option<usize>,
    /// Defenders at the start.
    pub n_def: usize,
    /// Expected defender hit points left after all attacks.
    pub hp_left: i32,
    /// Best odds of one of the fights.
    pub best: i32,
    /// Attackers able to attack ([`simulate_in_order`] only, as are the fields below).
    pub n_att: usize,
    /// Expected defenders killed and attackers lost, per mille.
    pub kills: i32,
    pub losses: i32,
    /// The best the attack can gain: over the attacks in order, the most that what the kills are
    /// worth exceeds what the losses are worth ([`worth`], kills weighted by the caller), from
    /// some first attacks on; 0 when not even the first attack is worth it.
    pub gain: i64,
    /// The odds of the first attack (per mille; 0 when there is none).
    pub first: i32,
}

/// The order in which [`simulate_in_order`] sends the attackers: the order the attack itself
/// will follow.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Order {
    /// A siege operation's [`strike`]: collateral attackers while several defenders stand, units
    /// summoned for good, the best odds when 68%+, else the unit cheapest to lose for the damage
    /// it is expected to deal (none that would barely dent the defender); heroes and veterans
    /// below their odds last.
    Strike,
    /// A stack's group attack (`military::assault`): the attacker the rules pick
    /// (`AI_getBestGroupAttacker`: the best odds, heroes below 85% and units above level 4 below
    /// 95% after the others); a hero or veteran below its odds there ends it (`PRECIOUS`).
    Group,
}

/// What a unit is worth, in hammers, to the side that would lose it: its cost, or for units
/// whose cost says little (animals, summons, heroes such as Acheron at 25) fifteen hammers a
/// point of strength, whichever is more; experience adds a tenth per point.
pub(crate) fn worth(g: &Game, u: UnitId) -> i64 {
    let unit = &g.units[u];
    let info = &g.d.units[unit.unit_type];
    let base = (info.production_cost as i64).max(info.combat as i64 * 15).max(10);
    base * (10 + unit.experience.max(0) as i64) / 10
}

/// A civilization with the conquest drive (CLAUDE.md, "Conquest and domination are a flavor
/// drive"): the Clan of Embers, the Doviello, the Bannor, and every leader with the Aggressive
/// trait.
pub(crate) fn conqueror(g: &Game, p: PlayerId) -> bool {
    let civ = g.player_get_civilization_type(p).map(|c| g.d.civilizations[c].base.type_.as_str());
    matches!(civ, Some("CIVILIZATION_CLAN_OF_EMBERS" | "CIVILIZATION_DOVIELLO" | "CIVILIZATION_BANNOR"))
        || g.d.ty::<TraitType>("TRAIT_AGGRESSIVE").is_some_and(|t| g.players[p].traits[t])
}

/// Does `p` strike sieges at 60% sure (switch `BOLD_STRIKE`, conquerors only)? As one player's
/// edge the bolder bar won nothing and cost nothing measurable (seeds 40001-40016: share +0.6%,
/// win rate 14.1% -> 12.8%, 4 seeds lost only with it); across all players it took 8% more civ
/// cities. The conquest drive is flavor, so the conquerors take the bolder bar (docs/AI.md,
/// round 7h).
fn bold_striker(g: &Game, p: PlayerId) -> bool {
    crate::switches::BOLD_STRIKE.on(g, p) && conqueror(g, p)
}

/// Is switch `SIM_FAITHFUL` on for the owner of `atts`?
fn faithful(g: &Game, atts: &[UnitId]) -> bool {
    atts.first().is_some_and(|&a| crate::switches::SIM_FAITHFUL.on(g, g.units[a].owner))
}

/// Play the strike of `atts` against the defenders `defs` of plot `tp` in expected values
/// (see the module doc). `wall_cut`: city defense points bombarded away first. With switch
/// `SIM_FAITHFUL` the strike is played as [`strike`] will fight it ([`simulate_in_order`], each
/// defender 90% sure to fall).
pub fn simulate(g: &Game, tp: PlotIdx, defs: &[UnitId], atts: &[UnitId], wall_cut: i32) -> Sim {
    if faithful(g, atts) {
        return simulate_in_order(g, tp, defs, atts, wall_cut, Order::Strike, 100, 100);
    }
    let mut ds: Vec<(UnitId, i32, i32)> = defs.iter().map(|&d| (d, g.unit_curr_hit_points(d), g.unit_max_combat_str(d, Some(tp), None, None))).collect();
    let n_def = ds.len();
    if ds.is_empty() {
        return Sim { used: Some(0), n_def, hp_left: 0, best: 1000, ..Sim::default() };
    }
    let max_hp = g.d.defines.max_hit_points.max(1);
    let dead = max_hp / 8;
    let collateral = |u: UnitId| g.unit_collateral_damage(u) > 0;
    let mut order: Vec<UnitId> = atts.iter().copied().filter(|&u| fights(g, u)).collect();
    order.sort_by_key(|&u| (std::cmp::Reverse(collateral(u) && n_def >= 2), std::cmp::Reverse(attack_power(g, u)), u));
    let mut best = 0;
    for (i, &a) in order.iter().enumerate() {
        ds.retain(|d| d.1 > dead);
        if ds.is_empty() {
            return Sim { used: Some(i), n_def, hp_left: 0, best, ..Sim::default() };
        }
        // The defender the rules put up: the strongest (by its plot strength and health).
        let j = (0..ds.len()).max_by_key(|&j| (ds[j].2 as i64 * ds[j].1 as i64, std::cmp::Reverse(ds[j].0))).unwrap();
        let (odds, wound) = fight(g, a, ds[j].0, tp, ds[j].1, wall_cut);
        best = best.max(odds);
        let hp = ds[j].1;
        ds[j].1 = ((1000 - odds) as i64 * (hp - wound).max(0) as i64 / 1000) as i32;
        // Collateral damage on the others (CvUnit::collateralCombat).
        if collateral(a) && ds.len() > 1 {
            let cs = g.unit_base_combat_str(a) * g.unit_collateral_damage(a) / 100;
            let limit = g.unit_collateral_damage_limit(a);
            let mut n = (ds.len() as i32 - 1).min(g.unit_collateral_damage_max_units(a));
            let mut others: Vec<usize> = (0..ds.len()).filter(|&o| o != j).collect();
            others.sort_by_key(|&o| (std::cmp::Reverse(ds[o].1), o));
            for o in others {
                if n <= 0 {
                    break;
                }
                n -= 1;
                let ts = g.unit_base_combat_str(ds[o].0);
                let sf = (cs + ts + 1) / 2;
                if ts + sf <= 0 {
                    continue;
                }
                let dmg = g.d.defines.collateral_combat_damage * (cs + sf) / (ts + sf);
                let cap = limit.min(limit * (cs + sf) / (ts + sf));
                let damage = max_hp - ds[o].1;
                let new_damage = damage.max((damage + dmg).min(cap));
                ds[o].1 = max_hp - new_damage;
            }
        }
    }
    ds.retain(|d| d.1 > dead);
    let hp_left = ds.iter().map(|d| d.1).sum();
    Sim { used: if ds.is_empty() { Some(order.len()) } else { None }, n_def, hp_left, best, ..Sim::default() }
}

/// The attack of `atts` on the defenders `defs` of plot `tp`, played in expected values as it
/// will be fought: every attack goes to the attacker `order` picks next, against the defender the
/// rules put up against that attacker (its strength against it, at its expected health). Each
/// fight lowers the chance that defender is alive by the odds (a siege engine, which stops at its
/// combat limit, never kills) and its expected health to what winning and losing leave (nothing,
/// or the combat limit; the damage of a lost fight, at most a round short of a kill), and the
/// chance the attacker is lost (unless it withdraws). Collateral damage hits the others. A
/// defender has fallen when it is alive with a chance of `sure` per mille or less; `used` counts
/// the attackers until all have. What the kills and losses are worth ([`worth`], kills counting
/// `kill_pct` percent) gives `gain`. Before (switch `SIM_FAITHFUL` off) the strike was played
/// strongest first against the defender strongest without regard to the attacker while the strike
/// sent its sacrifices first, a defender at an eighth of its expected health counted as dead, and
/// a siege engine's win as a kill: the simulation saw the city fall where the strike stalled.
pub fn simulate_in_order(g: &Game, tp: PlotIdx, defs: &[UnitId], atts: &[UnitId], wall_cut: i32, order: Order, kill_pct: i64, sure: i64) -> Sim {
    let n_def = defs.len();
    let atts: Vec<UnitId> = atts.iter().copied().filter(|&u| fights(g, u)).collect();
    let n_att = atts.len();
    if defs.is_empty() {
        return Sim { used: Some(0), n_def, hp_left: 0, best: 1000, n_att, ..Sim::default() };
    }
    let Some(&first) = atts.first() else {
        return Sim { used: None, n_def, hp_left: defs.iter().map(|&d| g.unit_curr_hit_points(d)).sum(), best: 0, n_att, ..Sim::default() };
    };
    let owner = g.units[first].owner;
    let precious = crate::switches::PRECIOUS.on(g, owner);
    let dent_guard = crate::switches::STRIKE_DAMAGE.on(g, owner);
    let max_hp = g.d.defines.max_hit_points.max(1);
    // Per defender: unit, expected hit points, chance alive (per mille), worth. A defender has
    // fallen when the chance it is still alive is `sure` per mille or less. (Its expected health,
    // which blends the fights it lost and the ones it won, is no measure: after a 50% fight it
    // reads low while the defender is alive half the time and still has to be killed.)
    let mut ds: Vec<(UnitId, i32, i64, i64)> = defs.iter().map(|&d| (d, g.unit_curr_hit_points(d), 1000, worth(g, d))).collect();
    let standing = |d: &(UnitId, i32, i64, i64)| d.2 > sure;
    let pairs: Vec<Vec<Option<Pair>>> = atts
        .iter()
        .map(|&a| defs.iter().map(|&d| (g.unit_can_attack_unit(a, d) && g.unit_can_defend(d, Some(tp))).then(|| pair(g, a, d, tp, wall_cut))).collect())
        .collect();
    let raised = |u: UnitId| crate::magic::raised(g.d, g.units[u].unit_type);
    let mut left: Vec<usize> = (0..n_att).collect();
    let (mut best, mut kills, mut losses, mut gain, mut best_gain, mut first_odds) = (0, 0i64, 0i64, 0i64, 0i64, 0);
    let mut used = 0;
    loop {
        let alive: Vec<usize> = (0..ds.len()).filter(|&j| standing(&ds[j])).collect();
        if alive.is_empty() {
            return Sim { used: Some(used), n_def, hp_left: 0, best, n_att, kills: kills as i32, losses: losses as i32, gain: best_gain, first: first_odds };
        }
        // Every attacker left: the defender it would meet, the odds and the damage of a loss.
        let cands: Vec<(usize, usize, i32, i32)> = left
            .iter()
            .filter_map(|&i| {
                let j = alive.iter().copied().filter(|&j| pairs[i][j].is_some()).max_by_key(|&j| (pairs[i][j].unwrap().rank(g, ds[j].1), std::cmp::Reverse(ds[j].0)))?;
                let (odds, wound) = pairs[i][j].unwrap().fight_capped(g, ds[j].1, true);
                Some((i, j, odds, wound))
            })
            .collect();
        let pick = match order {
            Order::Strike => {
                // Heroes and veterans below their odds last (`PRECIOUS`).
                let held = |&&(i, _, o, _): &&(usize, usize, i32, i32)| precious && {
                    let bar = precious_bar(g, atts[i]);
                    bar > 0 && o < bar
                };
                let line: Vec<&(usize, usize, i32, i32)> = cands.iter().filter(|c| !held(c)).collect();
                let pool: Vec<&(usize, usize, i32, i32)> = if line.is_empty() { cands.iter().collect() } else { line };
                let by_odds = |v: &[&(usize, usize, i32, i32)]| v.iter().copied().max_by_key(|&&(i, _, o, _)| (o, std::cmp::Reverse(atts[i]))).copied();
                let collateral: Vec<&(usize, usize, i32, i32)> = pool.iter().copied().filter(|&&(i, ..)| g.unit_collateral_damage(atts[i]) > 0).collect();
                let summoned: Vec<&(usize, usize, i32, i32)> = pool.iter().copied().filter(|&&(i, ..)| raised(atts[i])).collect();
                let best_odds = pool.iter().map(|&&(_, _, o, _)| o).max().unwrap_or(0);
                if alive.len() >= 2 && !collateral.is_empty() {
                    by_odds(&collateral)
                } else if !summoned.is_empty() {
                    by_odds(&summoned)
                } else if best_odds >= 680 {
                    by_odds(&pool)
                } else {
                    let sacrifice = |&(i, _, _, w): &(usize, usize, i32, i32)| {
                        let u = atts[i];
                        let cost = g.d.units[g.units[u].unit_type].production_cost.max(1) as i64;
                        let strength = if dent_guard { w as i64 * 100 } else { attack_power(g, u) as i64 };
                        strength * (100 + g.unit_withdrawal_probability(u) as i64) * 100 / (cost * (10 + g.units[u].experience.max(0) as i64))
                    };
                    pool.iter().copied().max_by_key(|&c| (sacrifice(c), std::cmp::Reverse(atts[c.0]))).copied().filter(|&(_, _, _, w)| !dent_guard || w * 10 >= max_hp)
                }
            }
            Order::Group => {
                // `CvSelectionGroupAI::AI_getBestGroupAttacker` for an AI player.
                let mut pick: Option<(usize, usize, i32, i32)> = None;
                let mut best_value = 0;
                for &c in &cands {
                    let u = atts[c.0];
                    let unit = &g.units[u];
                    let mut value = pairs[c.0][c.1].unwrap().ai_odds(g, ds[c.1].1);
                    if unit.ai_type == Some(UnitAIType::UNITAI_WARWIZARD) && value < 95 || unit.ai_type == Some(UnitAIType::UNITAI_HERO) && value < 85 {
                        value = 0;
                    }
                    if unit.level > 4 && value < 95 {
                        value /= 2;
                    }
                    value = value.max(1);
                    let possible = (alive.len() as i32 - 1).min(g.unit_collateral_damage_max_units(u));
                    if g.unit_collateral_damage(u) > 0 && possible > 0 {
                        value = value * (100 + g.unit_collateral_damage(u) * possible / 5) / 100;
                    }
                    if value > best_value || value == best_value && value > 0 {
                        best_value = value;
                        pick = Some(c);
                    }
                }
                // The stack holds when the rules would send a hero or veteran below its odds.
                pick.filter(|&(i, _, o, _)| {
                    let bar = if precious { precious_bar(g, atts[i]) } else { 0 };
                    bar == 0 || o >= bar
                })
            }
        };
        let Some((i, j, odds, wound)) = pick else { break };
        let a = atts[i];
        let pr = pairs[i][j].unwrap();
        best = best.max(odds);
        if used == 0 {
            first_odds = odds;
        }
        // A siege engine that wins leaves the defender at its combat limit; anything else kills.
        let kills_on_win = pr.a_limit >= pr.d_max_hp;
        let hp = ds[j].1;
        let win_hp = if kills_on_win { 0 } else { hp.min(pr.d_max_hp - pr.a_limit) };
        ds[j].1 = ((odds as i64 * win_hp as i64 + (1000 - odds) as i64 * (hp - wound).max(0) as i64) / 1000) as i32;
        let alive_after = ds[j].2 * (1000 - if kills_on_win { odds } else { 0 }) as i64 / 1000;
        let killed = ds[j].2 - alive_after;
        ds[j].2 = alive_after;
        let lost = (1000 - odds) as i64 * (100 - pr.withdraw.clamp(0, 100)) as i64 / 100;
        kills += killed;
        losses += lost;
        gain += killed * ds[j].3 * kill_pct / 100_000 - lost * worth(g, a) / 1000;
        best_gain = best_gain.max(gain);
        // Collateral damage on the others (CvUnit::collateralCombat).
        if g.unit_collateral_damage(a) > 0 && alive.len() > 1 {
            let cs = g.unit_base_combat_str(a) * g.unit_collateral_damage(a) / 100;
            let limit = g.unit_collateral_damage_limit(a);
            let mut n = (alive.len() as i32 - 1).min(g.unit_collateral_damage_max_units(a));
            let mut others: Vec<usize> = alive.iter().copied().filter(|&o| o != j).collect();
            others.sort_by_key(|&o| (std::cmp::Reverse(ds[o].1), o));
            for o in others {
                if n <= 0 {
                    break;
                }
                n -= 1;
                let ts = g.unit_base_combat_str(ds[o].0);
                let sf = (cs + ts + 1) / 2;
                if ts + sf <= 0 {
                    continue;
                }
                let dmg = g.d.defines.collateral_combat_damage * (cs + sf) / (ts + sf);
                let cap = limit.min(limit * (cs + sf) / (ts + sf));
                let damage = max_hp - ds[o].1;
                let new_damage = damage.max((damage + dmg).min(cap));
                ds[o].1 = max_hp - new_damage;
            }
        }
        left.retain(|&x| x != i);
        used += 1;
    }
    let alive: Vec<usize> = (0..ds.len()).filter(|&j| standing(&ds[j])).collect();
    let hp_left = alive.iter().map(|&j| ds[j].1).sum();
    Sim { used: if alive.is_empty() { Some(used) } else { None }, n_def, hp_left, best, n_att, kills: kills as i32, losses: losses as i32, gain: best_gain, first: first_odds }
}

/// City defense our siege engines in `units` can bombard away within three turns (0 without
/// siege engines).
fn wall_cut(g: &Game, c: CityId, units: &[UnitId]) -> i32 {
    let rate: i32 = units.iter().map(|&u| g.unit_bombard_rate(u)).sum();
    if rate <= 0 {
        return 0;
    }
    let ignore = units.iter().any(|&u| g.unit_bombard_rate(u) > 0 && g.unit_ignore_building_defense(u));
    let def = g.city_defense_modifier(c, ignore);
    let per_turn = if ignore { rate } else { rate * (100 - g.cities[c].building_bombard_defense).max(0) / 100 };
    def.min(per_turn * 3).max(0)
}

/// Attackers to spare before a force counts as covering a city: a fifth of the force, at
/// least one (an expected-value estimate: fights go worse about half the time).
fn margin(n: usize) -> usize {
    (n / 5).max(1)
}

/// Does `units` take city `c` (plot `tp`, defenders `defs`) with `spare` attackers to spare?
/// With `spare`, a [`margin`] of attackers must be left over (only units able to attack count).
pub(crate) fn covers(g: &Game, c: CityId, tp: PlotIdx, defs: &[UnitId], units: &[UnitId], spare: bool, siege: bool) -> bool {
    let n_att = units.iter().filter(|&&u| fights(g, u)).count();
    if n_att == 0 {
        return false;
    }
    let cut = if siege { wall_cut(g, c, units) } else { 0 };
    // `SIM_FAITHFUL`: the confidence is the simulation's own -- every defender 90% sure to fall
    // with a reserve to spare, 70% sure without -- in place of attackers held in reserve against
    // an estimate that was optimistic. (At 95% and 80%, every civilization took 23% fewer
    // cities, at no gain: docs/AI.md, round 7g.)
    if faithful(g, units) {
        // `BOLD_STRIKE` (conquerors): without a reserve, 60% sure.
        let bold = bold_striker(g, g.units[units[0]].owner);
        let s = simulate_in_order(g, tp, defs, units, cut, Order::Strike, 100, if spare { 100 } else if bold { 400 } else { 300 });
        return s.used.is_some() || s.best >= 850 && defs.len() == 1;
    }
    let spare = if spare { margin(n_att) } else { 0 };
    let s = simulate(g, tp, defs, units, cut);
    s.used.is_some_and(|n| n + spare <= n_att) || s.best >= 850 && defs.len() == 1
}

// ----------------------------------------------------------------------------
// Targets
// ----------------------------------------------------------------------------

/// A candidate target city.
#[derive(Copy, Clone, Debug)]
struct Cand {
    city: CityId,
    plot: PlotIdx,
    civ: bool,
    /// Defenders seen (or assumed when the city is not visible).
    n_def: i32,
    visible: bool,
    value: i32,
    /// Plot distance from our nearest city on its landmass.
    dist: i32,
}

/// Units a city of `n_def` defenders needs before we have seen a fight (rough).
pub(crate) fn rough_need(n_def: i32) -> i32 {
    n_def * 2 + 1
}

/// What taking city `c` is worth to us (`dist`: from our nearest city).
fn city_value(g: &Game, p: PlayerId, c: CityId, dist: i32, plan: &crate::victory::Plan) -> i32 {
    let city = &g.cities[c];
    let team = g.players[p].team;
    let mut v = 400 + city.population * 100 + city.num_world_wonders * 300;
    v += crate::victory::rival_city_bonus(g, plan, c); // victory-pursuit
    if g.d.religions.keys().any(|r| g.get_holy_city(r) == Some(c)) {
        v += 400;
    }
    for i in 0..NUM_CITY_PLOTS {
        let Some(q) = g.city_index_plot(c, i) else { continue };
        let plot = &g.map.plots[q];
        if !plot.revealed[team] {
            continue;
        }
        if let Some(b) = plot.bonus_type {
            if g.d.bonuses[b].tech_reveal.map_or(true, |t| g.team_is_has_tech(team, Some(t))) {
                v += 60;
            }
        }
    }
    if g.player_is_barbarian(city.owner) {
        // A barbarian town near our cities keeps sending raiders.
        v += (14 - dist).max(0) * 40;
    }
    v
}

/// Cities our army may campaign against: enemy civ cities at war (or the civ we prepare a war
/// against), else barbarian cities near our lands -- revealed, on a landmass where we have a
/// city. Returns the candidates and whether this is a war (or its preparation).
fn candidates(g: &Game, p: PlayerId) -> (Vec<Cand>, bool) {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    let war: Vec<TeamId> = g.teams.keys().filter(|&t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t)).collect();
    let preparing = if war.is_empty() { crate::diplomacy::preparing_target(g, team) } else { None };
    let civ = !war.is_empty() || preparing.is_some();
    let plan = if civ { crate::victory::plan(g, p) } else { crate::victory::Plan::default() }; // victory-pursuit
    let ours: Vec<(i32, i32, Option<AreaId>)> = g.players[p].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y, g.map.plots[g.city_plot(c)].area)).collect();
    let mut out = Vec::new();
    for pl in g.players.iter() {
        if !pl.alive || pl.cities.is_empty() {
            continue;
        }
        // (The Clan of Embers starts at peace with the barbarians.)
        let ok = if civ { war.contains(&pl.team) || preparing == Some(pl.team) } else { pl.team == barb && g.at_war(team, barb) };
        if !ok {
            continue;
        }
        for &c in &pl.cities {
            let cp = g.city_plot(c);
            if !g.map.plots[cp].revealed[team] {
                continue;
            }
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            let area = g.map.plots[cp].area;
            let Some(dist) = ours.iter().filter(|o| o.2 == area).map(|&(x, y, _)| g.plot_distance(x, y, cx, cy)).min() else { continue };
            if dist > if civ { 20 } else { 14 } {
                continue;
            }
            let visible = crate::view::sees_units(g, team, cp);
            // Unseen: what such a city usually holds. Barbarian towns arm over time (measured:
            // 0-1 defenders in their first 20 turns, 2 by turn 30, 3 by 60, 5 by 100).
            let n_def = if visible {
                visible_defenders(g, team, cp).len() as i32
            } else if civ {
                (2 + g.cities[c].population / 5).min(5)
            } else {
                (1 + (g.game_turn() - g.cities[c].game_turn_acquired) / 25).clamp(1, 6)
            };
            out.push(Cand { city: c, plot: cp, civ, n_def, visible, value: city_value(g, p, c, dist, &plan), dist });
        }
    }
    (out, civ)
}

/// Cheap score of a candidate (value for the force and the march it costs).
fn rough_score(c: &Cand) -> i64 {
    let need = rough_need(c.n_def) + i32::from(!c.visible) * 2;
    c.value as i64 * 1000 / (need as i64 * 100 + c.dist as i64 * 25 + 200)
}

/// A unit our operations count on to attack: one that can, other than a caster that marches with
/// the stack without fighting for it (`magic::spared_caster`).
pub(crate) fn fights(g: &Game, u: UnitId) -> bool {
    attack_power(g, u) > 0 && g.unit_can_attack(u) && !crate::magic::spared_caster(g, u)
}

/// Our army units on `area` able to attack (heroes included), strongest first.
fn force_on(g: &Game, p: PlayerId, area: Option<AreaId>) -> Vec<UnitId> {
    let mut out: Vec<UnitId> = Vec::new();
    for &grp in &g.players[p].groups {
        if !army_group(g, grp) {
            continue;
        }
        let Some(head) = g.group_get_head_unit(grp) else { continue };
        if g.map.plots[g.unit_plot(head)].area != area {
            continue;
        }
        out.extend(g.groups[grp].units.iter().copied().filter(|&u| fights(g, u)));
    }
    out.sort_by_key(|&u| (std::cmp::Reverse(attack_power(g, u)), u));
    out
}

/// A field army group (what `military::army` commands), on land, not aboard a ship.
fn army_group(g: &Game, grp: GroupId) -> bool {
    let Some(head) = g.group_get_head_unit(grp) else { return false };
    if g.unit_is_cargo(head) || g.unit_get_domain_type(head) != DomainType::DOMAIN_LAND {
        return false;
    }
    is_army_unit(g, head) || g.units[head].ai_type == Some(UnitAIType::UNITAI_HERO) && attack_power(g, head) > 0
}

/// Our units in groups whose last orders aim at city plot `tp` (see [`tagged`]).
fn committed_units(g: &Game, p: PlayerId, tp: PlotIdx) -> i32 {
    g.players[p].groups.iter().filter(|&&grp| tagged(g, grp) == Some(tp)).map(|&grp| g.groups[grp].units.len() as i32).sum()
}

/// Units the city of candidate `cand` needs against what we see of it, given our `force` on
/// its landmass (strongest first): the strike played out with the whole force when the city is
/// seen (what is missing, roughly, when it falls short), else a rough guess.
fn need_for(g: &Game, team: TeamId, cand: &Cand, force: &[UnitId]) -> i32 {
    if cand.visible && !force.is_empty() {
        let defs = visible_defenders(g, team, cand.plot);
        let all: Vec<UnitId> = force.iter().copied().take(24).collect();
        let s = simulate(g, cand.plot, &defs, &all, wall_cut(g, cand.city, &all));
        match s.used {
            // (`SIM_FAITHFUL`: already every defender 90% sure to fall.)
            Some(n) if faithful(g, &all) => n as i32,
            Some(n) => n as i32 + 1,
            // Short of the whole force: a defender's health per two more attackers.
            None => all.len() as i32 + 1 + s.hp_left * 2 / g.d.defines.max_hit_points.max(1),
        }
    } else if cand.visible {
        rough_need(cand.n_def)
    } else {
        // Enough to go and look.
        cand.n_def + 2
    }
}

/// The target of our operation and the number of units it needs. The city our stacks are
/// committed to (the most units whose last orders aim at it) keeps its place while it is within
/// reach of what we have or can build; otherwise the best value for its cost among the cities
/// we can reach (exact estimates for the few best, rough ones otherwise). Without that
/// stickiness the target changed whenever a stack came close enough to see a garrison, and the
/// stacks walked back and forth between two cities. Returns (candidate, units needed, feasible
/// with the force we have).
fn choose_target(g: &Game, p: PlayerId) -> Option<(Cand, i32, bool)> {
    let (cands, civ) = candidates(g, p);
    if cands.is_empty() {
        return None;
    }
    let team = g.players[p].team;
    let invested: Vec<i32> = cands.iter().map(|c| committed_units(g, p, c.plot)).collect();
    let kept = (0..cands.len()).filter(|&i| invested[i] > 0).max_by_key(|&i| (invested[i], std::cmp::Reverse(cands[i].city)));
    if let Some(i) = kept {
        let c = cands[i];
        let force = force_on(g, p, g.map.plots[c.plot].area);
        let need = need_for(g, team, &c, &force);
        // Within reach: twice the force we have at war; four more units than we have against
        // barbarians (production builds toward the target, but not without end).
        let reach = if civ { force.len() as i32 * 2 + 2 } else { force.len() as i32 + 4 };
        if need <= reach {
            return Some((c, need, need <= force.len() as i32));
        }
    }
    let mut order: Vec<usize> = (0..cands.len()).filter(|&i| Some(i) != kept).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(rough_score(&cands[i])), cands[i].city));
    let mut best: Option<(Cand, i32, bool, i64)> = None;
    for i in order.into_iter().take(4) {
        let c = cands[i];
        let force = force_on(g, p, g.map.plots[c.plot].area);
        let need = need_for(g, team, &c, &force);
        let feasible = need <= force.len() as i32;
        let mut score = c.value as i64 * 1000 / (need as i64 * 100 + c.dist as i64 * 25 + 200);
        if feasible {
            score += 1_000_000;
        }
        if best.map_or(true, |b| score > b.3) {
            best = Some((c, need, feasible, score));
        }
    }
    let (c, need, feasible, _) = best?;
    // A barbarian town far beyond the force we have and can build: leave it alone.
    if !civ && need > force_on(g, p, g.map.plots[c.plot].area).len() as i32 + 4 {
        return None;
    }
    Some((c, need, feasible))
}

// ----------------------------------------------------------------------------
// The plan of this turn
// ----------------------------------------------------------------------------

/// Build this turn's operation (once per `View`).
pub fn plan(g: &Game, v: &mut View) {
    if v.campaign.built {
        return;
    }
    v.campaign.built = true;
    if v.barbarian || g.players[v.p].cities.is_empty() {
        return;
    }
    let p = v.p;
    let Some((cand, need, feasible)) = choose_target(g, p) else { return };
    g.tel_intent(p, "campaign", || {
        let c = &g.cities[cand.city];
        format!("{} (player {}){}", c.name, c.owner.0, if feasible { "" } else { ", building up" })
    });
    let groups = op_groups(g, p, &cand);
    if groups.is_empty() {
        return;
    }
    if cand.civ && crate::switches::SECOND_OP.on(g, p) {
        if let Some(s) = split(g, p, &cand, need, &groups) {
            crate::trace!(g, p, "second operation: {} keeps {} units (need {}), {} takes {} (need {})", g.cities[cand.city].name, units_in(g, &s.keep), need, g.cities[s.second.city].name, units_in(g, &s.surplus), s.need);
            g.tel_decision(p, "military", "second_op", "surplus", units_in(g, &s.surplus) as i64, String::new);
            if let Some(op) = build_op(g, v, cand, need, s.keep) {
                v.campaign.ops.push(op);
            }
            if let Some(op) = build_op(g, v, s.second, s.need, s.surplus) {
                v.campaign.ops.push(op);
            }
            return;
        }
    }
    if let Some(op) = build_op(g, v, cand, need, groups) {
        v.campaign.ops.push(op);
    }
}

/// Our stacks on the landmass of target `cand` within reach of it, nearest first (plot
/// distance, group).
fn op_groups(g: &Game, p: PlayerId, cand: &Cand) -> Vec<(i32, GroupId)> {
    let tp = cand.plot;
    let (tx, ty) = g.map.xy(tp);
    let area = g.map.plots[tp].area;
    let mut groups: Vec<(i32, GroupId)> = Vec::new();
    for &grp in &g.players[p].groups {
        if !army_group(g, grp) {
            continue;
        }
        let head = g.group_get_head_unit(grp).unwrap();
        let hp = g.unit_plot(head);
        if g.map.plots[hp].area != area {
            continue;
        }
        let (x, y) = g.map.xy(hp);
        let d = g.plot_distance(x, y, tx, ty);
        if d > if cand.civ { 24 } else { 16.max(cand.dist + 6) } {
            continue;
        }
        groups.push((d, grp));
    }
    groups.sort();
    groups
}

fn units_in(g: &Game, groups: &[(i32, GroupId)]) -> i32 {
    groups.iter().map(|&(_, grp)| stack_units(g, grp)).sum()
}

/// The units of stack `grp` that count in an operation's force: all but the casters marching with
/// it (`magic::spared_caster`).
fn stack_units(g: &Game, grp: GroupId) -> i32 {
    g.groups[grp].units.iter().filter(|&&u| !crate::magic::spared_caster(g, u)).count() as i32
}

/// A war force split in two (see [`split`]).
struct Split {
    /// The stacks that stay on the first target.
    keep: Vec<(i32, GroupId)>,
    /// The second target, the units it needs, and the stacks that go for it (distance to it).
    second: Cand,
    need: i32,
    surplus: Vec<(i32, GroupId)>,
}

/// A second operation for a clear surplus (switch `SECOND_OP`; More Naval AI caps the units sent
/// at one city, `AI_pickTargetCity`: a city already targeted by more than max(6, 3 x its
/// defenders) is passed over for the next). When the stacks committed at war hold more than twice
/// what the first target (seen: its need is real) needs, the first target keeps the stacks already
/// ordered against it and the nearest others up to twice its need (at least six units), and the
/// rest go for a second enemy city on the landmass that they cover on their own with the march
/// reserve, the best value for its cost. An expert does not walk thirty units to a city twelve
/// take. Assignments stick: a stack ordered against the second city stays with it while the second
/// force still takes it (without the reserve) and the first keeps what it needs; otherwise
/// everyone goes back to the first. None when there is no second operation.
fn split(g: &Game, p: PlayerId, cand: &Cand, need: i32, groups: &[(i32, GroupId)]) -> Option<Split> {
    let team = g.players[p].team;
    let at_war = |c: &Cand| g.at_war(team, g.players[g.cities[c.city].owner].team);
    if !at_war(cand) {
        return None;
    }
    let area = g.map.plots[cand.plot].area;
    let (cands, _) = candidates(g, p);
    let others: Vec<Cand> = cands.into_iter().filter(|c| c.civ && c.city != cand.city && g.map.plots[c.plot].area == area && at_war(c)).collect();
    if others.is_empty() {
        return None;
    }
    let size = |grp: GroupId| g.groups[grp].units.len() as i32;
    let cap = (2 * need).max(6);
    let tagged_to = |c: &Cand| -> i32 { groups.iter().filter(|&&(_, grp)| tagged(g, grp) == Some(c.plot)).map(|&(_, grp)| size(grp)).sum() };
    // The second operation under way (the other city the most of our stacks are ordered against).
    let running = (0..others.len()).filter(|&i| tagged_to(&others[i]) > 0).max_by_key(|&i| (tagged_to(&others[i]), std::cmp::Reverse(others[i].city)));
    if running.is_none() && (!cand.visible || units_in(g, groups) < 2 * need + 2) {
        return None;
    }
    // Who goes where, given the second target `t2` (index in `others`).
    let assign = |t2: usize| -> (Vec<(i32, GroupId)>, Vec<(i32, GroupId)>) {
        let c2 = &others[t2];
        let (tx, ty) = g.map.xy(c2.plot);
        let d2 = |grp: GroupId| {
            g.group_get_head_unit(grp).map_or(i32::MAX, |h| {
                let (x, y) = g.map.xy(g.unit_plot(h));
                g.plot_distance(x, y, tx, ty)
            })
        };
        let mut order: Vec<(i32, GroupId)> = groups.iter().copied().filter(|&(_, grp)| tagged(g, grp) != Some(c2.plot)).collect();
        order.sort_by_key(|&(d, grp)| (tagged(g, grp) != Some(cand.plot), d, grp));
        let mut keep: Vec<(i32, GroupId)> = Vec::new();
        let mut second: Vec<(i32, GroupId)> = groups.iter().filter(|&&(_, grp)| tagged(g, grp) == Some(c2.plot)).map(|&(_, grp)| (d2(grp), grp)).collect();
        let mut n = 0;
        for (d, grp) in order {
            if n < cap || d2(grp) > 24 {
                n += size(grp);
                keep.push((d, grp));
            } else {
                second.push((d2(grp), grp));
            }
        }
        keep.sort();
        second.sort();
        (keep, second)
    };
    // The force against `t2` takes it (with the march reserve, or at acceptable odds once under
    // way), and the units it needs.
    let takes = |t2: usize, second: &[(i32, GroupId)], reserve: bool| -> Option<i32> {
        let c2 = &others[t2];
        let all: Vec<UnitId> = second.iter().flat_map(|&(_, grp)| g.groups[grp].units.iter().copied()).collect();
        let mut force: Vec<UnitId> = all.iter().copied().filter(|&u| fights(g, u)).collect();
        force.sort_by_key(|&u| (std::cmp::Reverse(attack_power(g, u)), u));
        let need2 = need_for(g, team, c2, &force);
        if force.is_empty() || need2 > force.len() as i32 || c2.visible && !covers(g, c2.city, c2.plot, &visible_defenders(g, team, c2.plot), &all, reserve, true) {
            return None;
        }
        Some(need2)
    };
    let chosen = match running {
        Some(t2) => {
            let (keep, second) = assign(t2);
            takes(t2, &second, false).filter(|_| units_in(g, &keep) >= need).map(|need2| (t2, need2, keep, second))
        }
        None => {
            let mut best: Option<(i64, usize, i32, Vec<(i32, GroupId)>, Vec<(i32, GroupId)>)> = None;
            for t2 in 0..others.len() {
                let (keep, second) = assign(t2);
                if second.is_empty() || units_in(g, &keep) < cap {
                    continue;
                }
                let Some(need2) = takes(t2, &second, true) else { continue };
                let c2 = &others[t2];
                let score = c2.value as i64 * 1000 / (need2 as i64 * 100 + c2.dist as i64 * 25 + 200);
                if best.as_ref().map_or(true, |b| score > b.0) {
                    best = Some((score, t2, need2, keep, second));
                }
            }
            best.map(|(_, t2, need2, keep, second)| (t2, need2, keep, second))
        }
    };
    let (t2, need2, keep, surplus) = chosen?;
    Some(Split { keep, second: others[t2], need: need2, surplus })
}

/// The operation against `cand` (needing `need` units) with our stacks `groups` (distance to
/// it, group; nearest first).
fn build_op(g: &Game, v: &mut View, cand: Cand, need: i32, groups: Vec<(i32, GroupId)>) -> Option<Op> {
    let p = v.p;
    let team = v.team;
    let tp = cand.plot;
    let (tx, ty) = g.map.xy(tp);
    let area = g.map.plots[tp].area;
    let owner_team = g.players[g.cities[cand.city].owner].team;
    let at_war = g.at_war(team, owner_team);
    if groups.is_empty() {
        return None;
    }
    // Commit: at war every stack; against barbarians the nearest until the force covers the
    // city twice over (stacks get delayed: healing, a city in peril), plus whatever already
    // stands near it or is on its way.
    let mut committed: Vec<GroupId> = Vec::new();
    let mut n_units = 0;
    let enough = need * 2 + 2;
    for &(d, grp) in &groups {
        if !cand.civ && n_units >= enough && d > 3 && tagged(g, grp) != Some(tp) {
            continue;
        }
        committed.push(grp);
        n_units += stack_units(g, grp);
    }
    let units_of = |pred: &dyn Fn(i32, GroupId) -> bool| -> Vec<UnitId> {
        let mut out = Vec::new();
        for &(d, grp) in &groups {
            if committed.contains(&grp) && pred(d, grp) {
                out.extend(g.groups[grp].units.iter().copied().filter(|&u| !crate::magic::spared_caster(g, u)));
            }
        }
        out
    };
    let all = units_of(&|_, _| true);
    let near = units_of(&|d, _| d <= 2);
    let adj: Vec<UnitId> = units_of(&|d, _| d <= 1).into_iter().filter(|&u| g.unit_moves_left(u) > 0).collect();
    // (Heroes and veterans count: below their odds they attack last in the strike, but they do.)
    let defs = visible_defenders(g, team, tp);
    // We commit to marching only with a confident force (a reserve to spare: `spare = true`), so
    // stacks do not advance understrength; but once a committed force is positioned next to or
    // beside the city it strikes at ACCEPTABLE odds (`spare = false`: it expects to clear the
    // defenders, without insisting on keeping a reserve) rather than retreating to muster an
    // overwhelming force against a target that only hardens while it waits. Taking territory at
    // acceptable risk is the point; a lone unit still never qualifies (it cannot clear a city).
    let (ok_all, ok_near, ok_adj) = if cand.visible {
        (
            covers(g, cand.city, tp, &defs, &all, true, true),
            covers(g, cand.city, tp, &defs, &near, false, true),
            !adj.is_empty() && covers(g, cand.city, tp, &defs, &adj, false, false),
        )
    } else {
        // Unseen: a scouting-sized force goes to look. Capped (not the assumed defender count,
        // which grows as a barb town arms with age -- that made the needed force climb out of
        // reach so the army mustered forever and never marched to reveal the town). Once the
        // stack reaches the assembly ring and the real defenders show, the visible branch above
        // decides: strike at acceptable odds, or step back if it turns out too strong.
        let n = ((cand.n_def + 2) as usize).min(4);
        (all.len() >= n, near.len() >= n, false)
    };
    // Concentrate before a MARGINAL strike. The adjacent ring may clear the city at acceptable
    // odds (`ok_adj`, no reserve) yet bounce; `strike()` only pools units a single plot away, so a
    // committed stack one plot out is left out of the fight (measured: a city struck by 8 while
    // another 8 stood one plot back). If that farther stack can reach an adjacent plot THIS turn
    // and the combined force would then take the city decisively (with a reserve), hold the
    // adjacent stacks and step the farther one in (StepIn) instead of striking piecemeal -- they
    // strike together next turn. Strictly one turn: a stack that cannot close this turn is not
    // counted, so a city that only hardens is still struck now at acceptable odds, and a ring that
    // already wins with a reserve (`adj_decisive`) strikes now regardless.
    let adj_decisive = ok_adj && covers(g, cand.city, tp, &defs, &adj, true, false);
    let concentrate = ok_adj
        && !adj_decisive
        && {
            let mut strikeable = adj.clone();
            for &(d, grp) in &groups {
                if d != 2 || !committed.contains(&grp) {
                    continue;
                }
                let Some(h) = g.group_get_head_unit(grp) else { continue };
                let closes = v.flood.run(g, h, 2, false).into_iter().any(|r| {
                    if r.turns != 0 || g.map.plots[r.plot].plot_city.is_some() {
                        return false;
                    }
                    let (rx, ry) = g.map.xy(r.plot);
                    g.plot_distance(rx, ry, tx, ty) == 1
                        && !g.plot_is_visible_enemy_unit_for_unit(r.plot, h)
                        && g.unit_can_move_into(h, r.plot, false, false, false)
                });
                if closes {
                    strikeable.extend(g.groups[grp].units.iter().copied());
                }
            }
            strikeable.len() > adj.len() && covers(g, cand.city, tp, &defs, &strikeable, true, true)
        };
    // Strike when the adjacent ring is decisive, or when no one-turn concentration would improve
    // it; otherwise defer one turn to concentrate. `concentrate` implies `ok_near` and `ok_all`
    // (strikeable is a subset of near and all), so a deferred strike lands on StepIn, never Gather.
    let strike_now = ok_adj && (adj_decisive || !concentrate);
    let phase = if !at_war {
        Phase::Gather
    } else if strike_now {
        Phase::Strike
    } else if !ok_all {
        Phase::Gather
    } else if ok_near {
        Phase::StepIn
    } else {
        Phase::March
    };
    let rally = g.players[p].cities.iter().copied().filter(|&c| g.map.plots[g.city_plot(c)].area == area).min_by_key(|&c| (g.plot_distance(tx, ty, g.cities[c].x, g.cities[c].y), c)).map(|c| g.city_plot(c));
    let reference = committed.iter().find_map(|&grp| g.group_get_head_unit(grp));
    let assembly = reference.and_then(|r| assembly_plot(g, v, r, tp, rally));
    let staging = reference.and_then(|r| staging_plot(g, r, tp, assembly));
    crate::trace!(
        g,
        p,
        "campaign vs {} ({}, {} defenders{}): {:?}, {} stacks / {} units (need {}), near {}, adjacent {} [war {} ok {}/{}/{} age {} dist {}]",
        g.cities[cand.city].name,
        if cand.civ { "civ" } else { "barbarian" },
        cand.n_def,
        if cand.visible { "" } else { " assumed" },
        phase,
        committed.len(),
        all.len(),
        need,
        near.len(),
        adj.len(),
        at_war,
        ok_all,
        ok_near,
        ok_adj,
        g.game_turn() - g.cities[cand.city].game_turn_acquired,
        cand.dist
    );
    Some(Op { city: cand.city, plot: tp, civ: cand.civ, phase, groups: committed, rally, assembly, staging, struck: false, at_war, need, have: all.len() as i32 })
}

/// The plot two plots from the target where the stacks gather: land we can enter, no enemy
/// unit on it; one from which the city can be seen (high ground: the stacks there see its
/// garrison and decide without stepping next to it), then the best defense, nearest our rally
/// city.
fn assembly_plot(g: &Game, v: &View, reference: UnitId, tp: PlotIdx, rally: Option<PlotIdx>) -> Option<PlotIdx> {
    let (tx, ty) = g.map.xy(tp);
    let sight = g.unit_visibility_range(reference);
    let mut best: Option<(PlotIdx, i32)> = None;
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(tx, ty, dx, dy) else { continue };
            let (qx, qy) = g.map.xy(q);
            if g.plot_distance(qx, qy, tx, ty) != 2 || !g.map.plots[q].revealed[v.team] || g.plot_is_water(q) || g.plot_is_impassable(q) {
                continue;
            }
            if g.map.plots[q].plot_city.is_some() || g.plot_is_visible_enemy_unit_for_unit(q, reference) || !g.unit_can_move_into(reference, q, false, false, false) {
                continue;
            }
            let from_rally = rally.map_or(0, |r| {
                let (rx, ry) = g.map.xy(r);
                g.plot_distance(qx, qy, rx, ry)
            });
            let sees = g.plot_can_see_plot(q, tp, v.team, sight, None);
            let score = i32::from(sees) * 1000 + g.plot_defense_modifier(q, Some(v.team), true) * 2 - from_rally * 10 - v.danger_at(q) / 200;
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((q, score));
            }
        }
    }
    best.map(|(q, _)| q)
}

/// The plot next to the target the stacks step onto: next to the assembly plot, best defense.
fn staging_plot(g: &Game, reference: UnitId, tp: PlotIdx, assembly: Option<PlotIdx>) -> Option<PlotIdx> {
    let (tx, ty) = g.map.xy(tp);
    let mut best: Option<(PlotIdx, i32)> = None;
    let team = g.unit_get_team(reference);
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(tx, ty, DirectionType::from_usize(dir)) else { continue };
        if g.plot_is_water(q) || g.plot_is_impassable(q) || g.map.plots[q].plot_city.is_some() || g.plot_is_visible_enemy_unit_for_unit(q, reference) {
            continue;
        }
        if !g.unit_can_move_into(reference, q, false, false, false) {
            continue;
        }
        let (qx, qy) = g.map.xy(q);
        let from_assembly = assembly.map_or(0, |a| {
            let (ax, ay) = g.map.xy(a);
            g.plot_distance(qx, qy, ax, ay)
        });
        let score = g.plot_defense_modifier(q, Some(team), true) - from_assembly * 100;
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((q, score));
        }
    }
    best.map(|(q, _)| q)
}

// ----------------------------------------------------------------------------
// Orders
// ----------------------------------------------------------------------------

/// End the group's turn if it is still waiting for orders.
fn finish(g: &mut Game, grp: GroupId) {
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
}

/// Orders of an operation carry its target as the group's mission AI (C++
/// `AI_setMissionAI(MISSIONAI_ASSAULT, pTargetPlot)`), saved with the game: the next turn's plan
/// knows which stacks are already committed to which city, so the target stays put.
fn order(g: &mut Game, grp: GroupId, mission: MissionType, x: i32, y: i32, flags: i32, tp: PlotIdx) {
    g.group_push_mission(grp, mission, x, y, flags, false, false, Some(MissionAIType::MISSIONAI_ASSAULT), Some(tp), None);
}

/// [`order`] a move to `dest`, one turn of it: the rest of the path is dropped, so the stack
/// decides again next turn (an AI group on a multi-turn mission gets no orders until it ends,
/// C++ `CvSelectionGroup::doTurn`: stacks walked on to plots chosen for an earlier phase while
/// the force they were part of dwindled, or past cities of ours in peril while gathering).
fn march(g: &mut Game, grp: GroupId, dest: PlotIdx, tp: PlotIdx) {
    let (x, y) = g.map.xy(dest);
    order(g, grp, MissionType::MISSION_MOVE_TO, x, y, 0, tp);
    if alive(g, grp) && !g.groups[grp].mission_queue.is_empty() && !g.group_ready_to_move(grp, false) {
        g.group_clear_mission_queue(grp);
    }
}

/// [`order`] the group to hold its ground (fortify, else sleep, else skip).
fn hold_for(g: &mut Game, grp: GroupId, tp: PlotIdx) {
    for m in [MissionType::MISSION_FORTIFY, MissionType::MISSION_SLEEP, MissionType::MISSION_SKIP] {
        if m == MissionType::MISSION_SKIP || crate::units::can_start(g, grp, m, -1, -1) {
            order(g, grp, m, -1, -1, 0, tp);
            return;
        }
    }
}

/// The target city plot of the group's last campaign order (see `order`).
pub fn tagged(g: &Game, grp: GroupId) -> Option<PlotIdx> {
    if g.groups[grp].ai_mission_ai_type != Some(MissionAIType::MISSIONAI_ASSAULT) {
        return None;
    }
    g.group_ai_get_mission_ai_plot(grp)
}

/// Move the group to `dest`; when the rules find no path, to the reached plot nearest it.
fn go(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, dest: PlotIdx, tp: PlotIdx) {
    let here = g.unit_plot(u);
    if here == dest {
        hold_for(g, grp, tp);
        return;
    }
    march(g, grp, dest, tp);
    if alive(g, grp) && g.group_ready_to_move(grp, false) && g.units.contains(u) && g.unit_moves_left(u) > 0 {
        // No path: the reached plot nearest the destination.
        let (dx, dy) = g.map.xy(dest);
        let reach = v.flood.run(g, u, 8, false);
        let (hx, hy) = g.map.xy(here);
        let now = g.plot_distance(hx, hy, dx, dy);
        if let Some(r) = reach
            .iter()
            .filter(|r| {
                let (rx, ry) = g.map.xy(r.plot);
                let d = g.plot_distance(rx, ry, dx, dy);
                d < now && d >= 1
            })
            .min_by_key(|r| {
                let (rx, ry) = g.map.xy(r.plot);
                (g.plot_distance(rx, ry, dx, dy), r.turns, r.plot)
            })
            .copied()
        {
            march(g, grp, r.plot, tp);
        }
    }
    finish(g, grp);
}

/// Move toward the ring `ring` plots from the target: the reached plot on it the group gets to
/// first (the operation's assembly or staging plot on equal turns, so stacks meet; then the
/// best defense), else toward the operation's plot along the rules' path.
fn approach(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, op: &Op, ring: i32) {
    let here = g.unit_plot(u);
    let preferred = if ring == 1 { op.staging } else { op.assembly };
    let (tx, ty) = g.map.xy(op.plot);
    let team = v.team;
    let on_ring = |r: &crate::reach::Reached| {
        let (rx, ry) = g.map.xy(r.plot);
        g.plot_distance(rx, ry, tx, ty) == ring && g.map.plots[r.plot].plot_city.is_none()
    };
    // The flood stops past the turn it first arrives on the ring (the choice is the fewest turns).
    let reach = v.flood.run_nearest(g, u, 12, false, |r| on_ring(&r));
    let best = reach
        .iter()
        .filter(|r| on_ring(r))
        .min_by_key(|r| (r.turns, Some(r.plot) != preferred, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(team), true)), r.plot))
        .copied();
    crate::trace!(g, v.p, "  approach {:?} at {:?} -> ring {} of {:?}: {:?} (reach {}, preferred {:?})", grp, g.map.xy(here), ring, (tx, ty), best.map(|r| (g.map.xy(r.plot), r.turns)), reach.len(), preferred.map(|q| g.map.xy(q)));
    match best {
        Some(r) if r.plot == here => hold_for(g, grp, op.plot),
        Some(r) => {
            march(g, grp, r.plot, op.plot);
            finish(g, grp);
        }
        None => match preferred.or(op.assembly).or(op.staging) {
            Some(dest) => go(g, v, grp, u, dest, op.plot),
            None => go(g, v, grp, u, op.rally.unwrap_or(here), op.plot),
        },
    }
}

/// Siege engines in the group bombard the target's walls.
fn bombard(g: &mut Game, grp: GroupId, op: &Op) {
    if g.cities.contains(op.city) && g.cities[op.city].defense_damage < 100 && crate::units::can_start(g, grp, MissionType::MISSION_BOMBARD, -1, -1) {
        crate::units::push(g, grp, MissionType::MISSION_BOMBARD, -1, -1, 0, false);
    }
}

/// Is the group committed to an operation past gathering (marching, stepping in, striking)?
/// Such a stack leaves our cities in peril to their garrisons: the threat usually comes from
/// the city it marches on, and turning back each time it appears stalled the sieges (measured:
/// fewer cities taken, from civs and barbarians, and no fewer lost).
pub fn on_the_move(v: &View, grp: GroupId) -> bool {
    v.campaign.op_of(grp).is_some_and(|i| v.campaign.ops[i].phase != Phase::Gather)
}

/// Is the group next to the target of an operation that strikes this turn?
pub fn striking(g: &Game, v: &View, grp: GroupId) -> bool {
    let Some(op) = v.campaign.op_of(grp).map(|i| &v.campaign.ops[i]) else { return false };
    if op.phase != Phase::Strike {
        return false;
    }
    let (tx, ty) = g.map.xy(op.plot);
    g.group_get_head_unit(grp).is_some_and(|h| {
        let (x, y) = g.map.xy(g.unit_plot(h));
        g.plot_distance(x, y, tx, ty) <= 1
    })
}

/// Orders for a stack committed to an operation. `early`: called before hunting, when only
/// war operations and barbarian operations past gathering act. True if the group acted.
pub fn act(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId, early: bool) -> bool {
    plan(g, v);
    let Some(i) = v.campaign.op_of(grp) else { return false };
    let op = v.campaign.ops[i].clone();
    if !g.cities.contains(op.city) || g.cities[op.city].owner == v.p {
        return false;
    }
    let here = g.unit_plot(u);
    let (x, y) = g.map.xy(here);
    let (tx, ty) = g.map.xy(op.plot);
    let dist = g.plot_distance(x, y, tx, ty);
    // Against barbarians, a stack still gathering its force hunts first (unless it stands next
    // to the city: it steps back); once the force is there it marches (hunting would scatter
    // it again).
    if early && !op.civ && op.phase == Phase::Gather && dist > 1 {
        return false;
    }
    if dist <= 1 && crate::trace_on(v.p) {
        let a = crate::military::assess(g, grp, op.plot);
        let ours: Vec<String> = g.groups[grp].units.iter().map(|&o| g.d.units[g.units[o].unit_type].base.type_.trim_start_matches("UNIT_").to_string()).collect();
        let theirs: Vec<String> = g.map.plots[op.plot].units.iter().map(|&o| format!("{}({}%)", g.d.units[g.units[o].unit_type].base.type_.trim_start_matches("UNIT_"), 100 - g.units[o].damage)).collect();
        // (Our army units on the plots around the city: a single-unit stack with others around
        // is part of a joint strike, not a lone camp.)
        let around: usize = (0..DirectionType::NUM)
            .filter_map(|dir| g.plot_direction(tx, ty, DirectionType::from_usize(dir)))
            .map(|q| g.map.plots[q].units.iter().filter(|&&o| g.units[o].owner == v.p && is_army_unit(g, o)).count())
            .sum();
        crate::trace!(g, v.p, "stack {:?} ({} units) next to {} (defense {}%, {:?}, {} of ours around): {:?} ours {:?} theirs {:?}", grp, g.groups[grp].units.len(), g.cities[op.city].name, g.city_defense_modifier(op.city, false), op.phase, around, a, ours, theirs);
    }
    match op.phase {
        Phase::Strike if dist <= 1 => {
            if !op.struck {
                v.campaign.ops[i].struck = true;
                strike(g, v, &op);
            }
            if alive(g, grp) {
                finish(g, grp);
            }
        }
        Phase::Strike | Phase::StepIn => {
            if dist <= 1 {
                bombard(g, grp, &op);
                if alive(g, grp) && g.group_ready_to_move(grp, false) {
                    hold_for(g, grp, op.plot);
                }
            } else {
                // From the assembly ring only: a stack coming from afar would otherwise step
                // next to the city alone, ahead of the others.
                approach(g, v, grp, u, &op, if dist == 2 { 1 } else { 2 });
            }
        }
        Phase::March => {
            if dist <= 1 && g.groups[grp].units.len() >= 2 {
                // Here early: hold the ground (siege engines work on the walls).
                bombard(g, grp, &op);
                if alive(g, grp) && g.group_ready_to_move(grp, false) {
                    hold_for(g, grp, op.plot);
                }
            } else if dist == 2 {
                hold_for(g, grp, op.plot);
            } else {
                approach(g, v, grp, u, &op, 2);
            }
        }
        Phase::Gather => {
            // Too weak (or the war is not declared yet): gather in the rally city (at war; the
            // stacks committed against a barbarian town stay where they are, hunting raiders
            // and holding our cities, until the force is there: gathering them in one city bared
            // the others); a force almost strong enough that watches the city from the assembly
            // plot waits there for the rest (stepping back would lose sight of the garrison).
            let close = op.at_war && op.have * 10 >= op.need * 7;
            if dist == 2 && close && g.plot_is_visible(op.plot, v.team) {
                hold_for(g, grp, op.plot);
                return true;
            }
            if dist <= 1 {
                crate::trace!(g, v.p, "stack {:?} ({} units) steps back from {}", grp, g.groups[grp].units.len(), g.cities[op.city].name);
                if close {
                    if let Some(a) = op.assembly {
                        go(g, v, grp, u, a, op.plot);
                        return true;
                    }
                }
            }
            if !op.civ && dist > 1 {
                return false;
            }
            let Some(r) = op.rally else { return false };
            go(g, v, grp, u, r, op.plot);
        }
    }
    true
}

/// The odds (per mille) below which a precious unit attacks only last in a strike, and a stack it
/// would lead into an assault holds (0: any odds). More Naval AI's `AI_getBestGroupAttacker`
/// (Sephi's block): a hero at 85% plus its level, a unit above level 4 at 70% plus 3% a level (at
/// most 95%). A hero is a unique unit years of play went into; a high-level veteran is worth
/// several new units. Neither leads an attack it may well lose, nor is sacrificed to wear defenders
/// down; in a strike that takes the city, they finish what the others began.
pub(crate) fn precious_bar(g: &Game, u: UnitId) -> i32 {
    static HERO: std::sync::OnceLock<Option<PromotionType>> = std::sync::OnceLock::new();
    let unit = &g.units[u];
    let hero_promotion = *HERO.get_or_init(|| g.d.ty::<PromotionType>("PROMOTION_HERO"));
    let hero = unit.ai_type == Some(UnitAIType::UNITAI_HERO) || hero_promotion.is_some_and(|h| unit.has_promotion[h]);
    let level = unit.level.max(0);
    if hero {
        (850 + 10 * level).min(990)
    } else if level > 4 {
        (700 + 30 * level).min(950)
    } else {
        0
    }
}

/// The strike: the units of every committed stack next to the target attack this turn, one by
/// one, while the force left still covers the defenders left. Siege engines bombard first;
/// then collateral attackers while several defenders stand; then units summoned for good
/// (skeletons, `magic::raised`), the best odds first; then the best odds when they are good,
/// else the unit that is cheapest to lose for its strength (C++ `AI_getBestGroupSacrifice`
/// does the same in a stack attack). Each attacker is taken out of its stack first, as a human
/// selects a single unit: a stack whose other units have attacked cannot start a move.
fn strike(g: &mut Game, v: &mut View, op: &Op) {
    let p = v.p;
    let team = v.team;
    let tp = op.plot;
    let (tx, ty) = g.map.xy(tp);
    let next_to = |g: &Game, u: UnitId| {
        g.units.contains(u) && {
            let (x, y) = g.map.xy(g.unit_plot(u));
            g.plot_distance(x, y, tx, ty) == 1
        }
    };
    let groups: Vec<GroupId> = op.groups.iter().copied().filter(|&grp| alive(g, grp) && g.group_get_head_unit(grp).is_some_and(|h| next_to(g, h))).collect();
    for &grp in &groups {
        if alive(g, grp) {
            bombard(g, grp, op);
        }
    }
    let pool: Vec<UnitId> = groups.iter().filter(|&&grp| alive(g, grp)).flat_map(|&grp| g.groups[grp].units.clone()).collect();
    let name = g.cities[op.city].name.clone();
    // Our casters around the city soften its defenders and summon attackers first (magic.rs); the
    // strike below then weighs the defenders as they are left (a summon may even take or raze
    // the city), and blesses our attackers once it goes ahead.
    let ready: Vec<UnitId> = pool.iter().copied().filter(|&u| next_to(g, u) && g.unit_moves_left(u) > 0 && fights(g, u)).collect();
    crate::magic::trace_strike(g, p, tp);
    crate::magic::soften(g, v, tp, &ready);
    let mut done: Vec<UnitId> = Vec::new();
    let mut fights = 0;
    let precious = crate::switches::PRECIOUS.on(g, p);
    let dent_sacrifice = crate::switches::STRIKE_DAMAGE.on(g, p);
    let mut held_steps = 0;
    if precious && (crate::trace_on(p) || g.telemetry.on()) {
        let n = pool.iter().filter(|&&u| next_to(g, u) && precious_bar(g, u) > 0).count();
        if n > 0 {
            crate::trace!(g, p, "  {} heroes or veterans in the strike on {} attack last below their odds", n, name);
            g.tel_decision(p, "military", "precious_in_strike", "precious", n as i64, String::new);
        }
    }
    for _ in 0..48 {
        if !g.cities.contains(op.city) || g.cities[op.city].owner == p {
            break;
        }
        let defs = visible_defenders(g, team, tp);
        if defs.is_empty() {
            break;
        }
        // Siege engines with walls left to batter wait (their attack would only bombard).
        let walls = g.city_defense_modifier(op.city, false) > 0;
        let odds = |g: &Game, u: UnitId| g.plot_get_best_defender(tp, None, Some(p), Some(u), false, false, false).map_or(1000, |d| g.get_combat_odds(u, d));
        let able: Vec<UnitId> = pool
            .iter()
            .copied()
            .filter(|&u| next_to(g, u) && !done.contains(&u) && g.unit_moves_left(u) > 0 && !g.units[u].made_attack && attack_power(g, u) > 0 && g.unit_can_attack(u))
            .filter(|&u| !(walls && g.unit_bombard_rate(u) > 0))
            // A caster that marches with the stack attacks only at war wizard odds (CASTER_STACKS).
            .filter(|&u| !crate::magic::spared_caster(g, u) || odds(g, u) >= crate::magic::SPARED_CASTER_ODDS)
            .collect();
        // Heroes and veterans below their odds (switch `PRECIOUS`, [`precious_bar`]) fight last:
        // while another unit can attack, and never to wear the defenders down, only in a strike
        // that takes the city. (Held out of the strike altogether, they turned captures into
        // stalls: a fork of seed 26001 at turn 246 lost three units and the city where the
        // heroes' last attacks took it -- docs/AI.md, "Forks of the hero gate".)
        let line: Vec<UnitId> = able
            .iter()
            .copied()
            .filter(|&u| {
                let bar = if precious { precious_bar(g, u) } else { 0 };
                bar == 0 || odds(g, u) >= bar
            })
            .collect();
        // Start with a margin; once started, go on while the rest still takes the city, or
        // while the defenders are worn down to half their health and we outnumber them (they
        // heal if we stop). With switch `STRIKE_AT_PHASE` (or `BOLD_STRIKE`) the strike starts
        // at the odds that put the stacks in the strike phase (`plan`: the adjacent ring takes
        // the city without a reserve) instead of waiting next to the city for a reserve.
        let at_phase = crate::switches::STRIKE_AT_PHASE.on(g, p) || bold_striker(g, p);
        let takes = covers(g, op.city, tp, &defs, &able, fights == 0 && !at_phase, false);
        if fights == 0 && at_phase && crate::trace_on(p) {
            let reserve = covers(g, op.city, tp, &defs, &able, true, false);
            if takes != reserve {
                crate::trace!(g, p, "  strike on {} at the strike phase's odds: {} (with a reserve: {})", name, takes, reserve);
            }
        }
        let go = takes || fights > 0 && {
            let hp: i32 = defs.iter().map(|&d| g.unit_curr_hit_points(d)).sum();
            let max: i32 = defs.iter().map(|&d| g.unit_max_hit_points(d)).sum();
            hp * 2 <= max && line.len() >= defs.len()
        };
        let units = if takes && line.is_empty() { able.clone() } else { line };
        if units.len() < able.len() {
            held_steps += 1;
        }
        if crate::trace_on(p) {
            let s = simulate(g, tp, &defs, &units, 0);
            let hp: Vec<i32> = defs.iter().map(|&d| g.unit_curr_hit_points(d)).collect();
            crate::trace!(g, p, "  strike step {}: {} units, defenders hp {:?}, sim {:?}, go {}", fights, units.len(), hp, s, go);
        }
        if !go {
            break;
        }
        if fights == 0 {
            crate::magic::bless(g, v, tp, &units);
        }
        // The next attacker.
        let with_odds: Vec<(UnitId, i32)> = units.iter().map(|&u| (u, odds(g, u))).collect();
        let best_odds = with_odds.iter().map(|&(_, o)| o).max().unwrap_or(0);
        let collateral = defs.len() >= 2 && with_odds.iter().any(|&(u, _)| g.unit_collateral_damage(u) > 0);
        // The damage `u` is expected to deal the defender it meets when it loses.
        let dent = |u: UnitId| g.plot_get_best_defender(tp, None, Some(p), Some(u), false, false, false).map_or(0, |d| fight(g, u, d, tp, g.unit_curr_hit_points(d), 0).1);
        let sacrifice = |u: UnitId| {
            let cost = g.d.units[g.units[u].unit_type].production_cost.max(1) as i64;
            let strength = if dent_sacrifice { dent(u) as i64 * 100 } else { attack_power(g, u) as i64 };
            strength * (100 + g.unit_withdrawal_probability(u) as i64) * 100 / (cost * (10 + g.units[u].experience.max(0) as i64))
        };
        let raised = |u: UnitId| crate::magic::raised(g.d, g.units[u].unit_type);
        let pick = if collateral {
            with_odds.iter().filter(|&&(u, _)| g.unit_collateral_damage(u) > 0).max_by_key(|&&(u, o)| (o, std::cmp::Reverse(u))).map(|&(u, _)| u)
        } else if with_odds.iter().any(|&(u, _)| raised(u)) {
            // Units summoned for good (skeletons) cost nothing to replace: they wear the
            // defenders down for the units that follow, as siege fodder after the catapults.
            with_odds.iter().filter(|&&(u, _)| raised(u)).max_by_key(|&&(u, o)| (o, std::cmp::Reverse(u))).map(|&(u, _)| u)
        } else if best_odds >= 680 {
            with_odds.iter().max_by_key(|&&(u, o)| (o, std::cmp::Reverse(u))).map(|&(u, _)| u)
        } else {
            with_odds.iter().max_by_key(|&&(u, _)| (sacrifice(u), std::cmp::Reverse(u))).map(|&(u, _)| u)
        };
        let Some(u) = pick else { break };
        // A sacrifice that would not dent the defenders is no sacrifice (switch `STRIKE_DAMAGE`).
        if dent_sacrifice && best_odds < 680 && !collateral && !raised(u) && dent(u) * 10 < g.d.defines.max_hit_points {
            crate::trace!(g, p, "  no attacker would dent the defenders (best {} damage {})", g.d.units[g.units[u].unit_type].base.type_, dent(u));
            break;
        }
        crate::trace!(g, p, "  attacker {} (odds {}, damage if lost {})", g.d.units[g.units[u].unit_type].base.type_, with_odds.iter().find(|&&(x, _)| x == u).map_or(0, |&(_, o)| o), dent(u));
        // Out of its stack (a stack with spent units cannot start a move).
        if g.units[u].group.is_some_and(|grp| g.groups[grp].units.len() > 1) {
            g.unit_join_group(u, None, true);
        }
        let Some(grp) = g.units[u].group else { break };
        let moves = g.unit_moves_left(u);
        g.groups[grp].ai_group_attack = false;
        order(g, grp, MissionType::MISSION_MOVE_TO, tx, ty, MOVE_DIRECT_ATTACK, tp);
        fights += 1;
        if alive(g, grp) {
            g.groups[grp].ai_group_attack = false;
        }
        if g.units.contains(u) && g.unit_moves_left(u) == moves && !g.units[u].made_attack {
            // No attack happened.
            done.push(u);
        }
    }
    // The defenders are gone: move in.
    let taken = |g: &Game| !g.cities.contains(op.city) || g.cities[op.city].owner == p;
    if !taken(g) && g.cities.contains(op.city) && visible_defenders(g, team, tp).is_empty() {
        // (Workers left in the city are captured on the way in.) A fighter first: a caster that
        // marches with the stack goes in only when no other unit can.
        let first = |g: &Game, u: UnitId| next_to(g, u) && g.unit_moves_left(u) > 0 && g.unit_can_move_or_attack_into(u, tp, false);
        if let Some(u) = pool.iter().copied().filter(|&u| first(g, u)).min_by_key(|&u| crate::magic::spared_caster(g, u)) {
            if g.units[u].group.is_some_and(|grp| g.groups[grp].units.iter().any(|&o| g.unit_moves_left(o) <= 0)) {
                g.unit_join_group(u, None, true);
            }
            if let Some(grp) = g.units[u].group {
                order(g, grp, MissionType::MISSION_MOVE_TO, tx, ty, MOVE_DIRECT_ATTACK, tp);
            }
        }
    }
    // Taken: the other units still able to move follow into the city (a lone, wounded victor
    // would lose it to the next counterattack).
    if taken(g) && g.cities.contains(op.city) {
        let mut n = 0;
        // Fighters first (a caster that marches with the stack is no garrison).
        let mut pool = pool.clone();
        pool.sort_by_key(|&u| g.units.contains(u) && crate::magic::spared_caster(g, u));
        for &u in &pool {
            if n >= 3 {
                break;
            }
            if !next_to(g, u) || g.unit_moves_left(u) <= 0 || !g.unit_can_move_into(u, tp, false, false, false) {
                continue;
            }
            if g.units[u].group.is_some_and(|grp| g.groups[grp].units.len() > 1) {
                g.unit_join_group(u, None, true);
            }
            if let Some(grp) = g.units[u].group {
                march(g, grp, tp, tp);
                n += 1;
            }
        }
    }
    crate::trace!(g, p, "strike on {}: {} stacks, {} attacks, {}", name, groups.len(), fights, if taken(g) { "taken" } else { "holds" });
    if held_steps > 0 {
        g.tel_decision(p, "military", "precious_held", "precious", held_steps, String::new);
    }
    // End the turn of every stack involved (including the units taken out of them).
    for u in pool {
        if let Some(grp) = g.units.contains(u).then(|| g.units[u].group).flatten() {
            finish(g, grp);
        }
    }
}

// ----------------------------------------------------------------------------
// Production
// ----------------------------------------------------------------------------

/// The force our operation wants: attackers and siege engines.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Force {
    pub attackers: i32,
    pub siege: i32,
    /// Strength (x100) of the strongest defender seen at the target, in its city (fortified,
    /// city defense, terrain); 0 when nothing is seen.
    pub defense: i32,
    /// Team that owns the target city (barbarian or a rival civ): who our operation is fighting,
    /// so callers can weigh our strength against theirs.
    pub target: Option<TeamId>,
}

/// The force the operation of `p` wants (what production builds toward), from the cheap
/// estimate of its target (the city our stacks are committed to, else the best value for its
/// cost): about two attackers per defender, siege engines when the city has walls or several
/// defenders.
pub fn needed_force(g: &Game, p: PlayerId) -> Option<Force> {
    let (cands, civ) = candidates(g, p);
    let c = cands.iter().max_by_key(|c| (committed_units(g, p, c.plot) > 0, rough_score(c), std::cmp::Reverse(c.city)))?;
    let walls = g.city_defense_modifier(c.city, false);
    let mut attackers = rough_need(c.n_def);
    let siege = if c.n_def >= 3 || walls >= 25 { (c.n_def / 2).clamp(1, 3) } else { 0 };
    attackers -= siege;
    attackers = attackers.min(if civ { 16 } else { 10 });
    let team = g.players[p].team;
    let defense = visible_defenders(g, team, c.plot).iter().map(|&d| g.unit_max_combat_str(d, Some(c.plot), None, None)).max().unwrap_or(0);
    let target = Some(g.players[g.cities[c.city].owner].team);
    Some(Force { attackers, siege, defense, target })
}

/// Can a unit of type `u` hurt a defender of strength `defense` (x100) in a city? At least half
/// its strength (with the unit's city attack bonus): a stack of them grinds the garrison
/// down; weaker ones only feed it experience.
fn useful_attacker(g: &Game, u: UnitType, defense: i32) -> bool {
    let info = &g.d.units[u];
    let att = crate::view::type_attack_power(g.d, u) * (100 + info.city_attack_modifier.max(-50)) / 100;
    defense <= 0 || att * 2 >= defense || info.collateral_damage > 0 || info.bombard_rate > 0
}

/// Production hook (army composition): a siege engine, the building that trains them, or an
/// attacker when our operation wants more than we have. `attackers`/`siege`: what we have
/// (alive or in training).
pub fn army_choice(g: &Game, c: CityId, attackers: i32, siege: i32, afford: bool) -> Option<Choice> {
    let p = g.cities[c].owner;
    if !afford || g.player_is_barbarian(p) {
        return None;
    }
    let f = needed_force(g, p)?;
    let war = candidates_war(g, p);
    // In peacetime the army takes every other slot (buildings matter too).
    if !war && (g.game_turn() + c.0 as i32) % 2 != 0 {
        return None;
    }
    let mut choice = None;
    if siege < f.siege {
        choice = best_unit(g, c, Role::Siege).map(|u| Choice::Train(u, UnitAIType::UNITAI_ATTACK_CITY)).or_else(|| siege_building(g, c).map(Choice::Construct));
    }
    if choice.is_none() && attackers + siege < f.attackers + f.siege {
        let role = if war || f.defense > 0 { Role::CityAttack } else { Role::Attack };
        choice = best_unit(g, c, role).filter(|&u| useful_attacker(g, u, f.defense)).map(|u| Choice::Train(u, UnitAIType::UNITAI_ATTACK));
    }
    if let Some(ch) = choice {
        crate::trace!(g, p, "{}: the campaign wants {:?} (have {} + {} siege): {}", g.cities[c].name, f, attackers, siege, crate::production::choice_name(g, ch));
    }
    choice
}

/// At war with a civ (or preparing one).
fn candidates_war(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t)) || crate::diplomacy::preparing_target(g, team).is_some()
}

/// The building that lets city `c` train our best siege engine (FfH Siege Workshop), when no
/// city of ours can train one yet, none is building it, and `c` is our most productive city.
fn siege_building(g: &Game, c: CityId) -> Option<BuildingType> {
    let p = g.cities[c].owner;
    let d = g.d;
    let civ = g.player_get_civilization_type(p)?;
    let pl = &g.players[p];
    if pl.cities.iter().any(|&o| best_unit(g, o, Role::Siege).is_some()) {
        return None;
    }
    let prod = |o: CityId| g.city_yield_rate(o, YieldType::YIELD_PRODUCTION);
    if pl.cities.iter().any(|&o| o != c && !g.city_is_occupation(o) && (prod(o), std::cmp::Reverse(o)) > (prod(c), std::cmp::Reverse(c))) {
        return None;
    }
    for u in d.units.keys() {
        let info = &d.units[u];
        if info.domain_type != Some(DomainType::DOMAIN_LAND) || info.bombard_rate <= 0 && info.collateral_damage <= 0 {
            continue;
        }
        if !(info.unit_ai_type[UnitAIType::UNITAI_ATTACK_CITY] || info.unit_ai_type[UnitAIType::UNITAI_COLLATERAL]) {
            continue;
        }
        let Some(class) = info.unit_class_type else { continue };
        if d.civ_unit(civ, class) != Some(u) || !g.player_can_train(p, u, false, false, false) {
            continue;
        }
        // (Every siege engine needs a Siege Workshop, `PrereqBuilding`.)
        let Some(b) = crate::production::unit_building(d, civ, info) else { continue };
        let Some(bc) = d.buildings[b].building_class_type else { continue };
        if g.city_is_has_building_class(c, bc) || !g.city_can_construct(c, b, false, false, false) {
            continue;
        }
        if pl.cities.iter().any(|&o| g.city_production_building(o) == Some(b)) {
            return None;
        }
        return Some(b);
    }
    None
}

/// Siege engines and attackers we have (alive or in training), for traces.
pub fn army_counts(g: &Game, p: PlayerId) -> (i32, i32) {
    let att = count_ai(g, p, UnitAIType::UNITAI_ATTACK);
    let siege = count_ai(g, p, UnitAIType::UNITAI_ATTACK_CITY) + count_ai(g, p, UnitAIType::UNITAI_COLLATERAL);
    (att, siege)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use ffh_core::map::{Map, MapInitData};
    use ffh_core::mapscript::FractalMapScript;
    use ffh_core::setup::GameSetup;

    use super::*;

    /// One block of grassland (x 2..=17, y 3..=8) in an ocean, revealed to both players: a fresh
    /// two-player AI game whose starting units are gone (as `tests/magic.rs`).
    pub(crate) fn one_landmass_game(seed: u32) -> Game {
        one_landmass_game_with(seed, &[])
    }

    /// [`one_landmass_game`] with the first players' civilizations given (each with its first
    /// leader).
    pub(crate) fn one_landmass_game_with(seed: u32, civs: &[&str]) -> Game {
        let d = ffh_data::global();
        let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", seed);
        for (slot, name) in civs.iter().enumerate() {
            let civ = d.t::<CivilizationType>(name);
            setup.players[slot].civ = Some(civ);
            setup.players[slot].leader = d.leader_heads.keys().find(|&l| d.civilizations[civ].leaders[l]);
        }
        let mut g = Game::new(d, setup);
        g.setup.options[GameOptionType::GAMEOPTION_NO_BARBARIANS] = true;
        g.map_script = Arc::new(FractalMapScript::default());
        let ai = crate::ai_controller();
        for p in g.player_ids() {
            g.controllers[p] = ai.clone();
        }
        g.autorun = true;
        g.game_init();
        for u in g.players.iter().flat_map(|pl| pl.units.clone()).collect::<Vec<_>>() {
            if g.units.contains(u) {
                g.unit_kill(u, false, None);
            }
        }
        let (np, nt) = (g.num_players(), g.num_teams());
        g.map = Map::new(d, &MapInitData { grid_w: 20, grid_h: 12, ..Default::default() }, np, nt);
        let (grass, coast) = (d.t::<TerrainType>("TERRAIN_GRASS"), d.t::<TerrainType>("TERRAIN_COAST"));
        for p in g.map.plot_ids() {
            let (x, y) = g.map.xy(p);
            let land = (3..=8).contains(&y) && (2..=17).contains(&x);
            g.plot_set_plot_type(p, if land { PlotType::PLOT_LAND } else { PlotType::PLOT_OCEAN }, false);
            g.plot_set_terrain_type(p, if land { grass } else { coast }, false);
        }
        g.map_recalculate_areas();
        g.update_plot_groups();
        for t in g.team_ids() {
            for p in g.map.plot_ids() {
                g.plot_set_revealed(p, t, true, false, None, false);
            }
        }
        g.update_plot_groups();
        g
    }

    /// A skeleton in a strike attacks before the Champions, though they would take the city on
    /// their own: units summoned for good soften the defenders (the old order sent the best odds
    /// first, and the skeleton never fought).
    #[test]
    fn a_skeleton_softens_the_city_before_the_champions() {
        let d = ffh_data::global();
        let mut g = one_landmass_game(21);
        let (us, them) = (PlayerId(0), PlayerId(1));
        g.player_found(us, 4, 5);
        g.player_found(them, 12, 5);
        let city = g.players[them].capital_city.expect("the enemy founded a city");
        let tp = g.city_plot(city);
        g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        let (ut, tt) = (g.players[us].team, g.players[them].team);
        g.team_declare_war(ut, tt, true, None);
        let skeleton = g.player_init_unit(us, d.t::<UnitType>("UNIT_SKELETON"), 11, 5, Some(UnitAIType::UNITAI_ATTACK), None);
        assert!(crate::magic::raised(d, g.units[skeleton].unit_type));
        let champions: Vec<UnitId> = (0..2).map(|_| g.player_init_unit(us, d.t::<UnitType>("UNIT_CHAMPION"), 11, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        assert!(!crate::magic::raised(d, g.units[champions[0]].unit_type));
        let grp = g.units[champions[0]].group.expect("a group");
        for u in [skeleton, champions[1]] {
            g.unit_join_group(u, Some(grp), false);
        }
        let op = Op {
            city,
            plot: tp,
            civ: true,
            phase: Phase::Strike,
            groups: vec![grp],
            rally: None,
            assembly: None,
            staging: None,
            struck: false,
            at_war: true,
            need: 1,
            have: 3,
        };
        let mut v = View::new(&g, us);
        strike(&mut g, &mut v, &op);
        assert!(!g.units.contains(skeleton) || g.units[skeleton].made_attack, "the skeleton attacked");
    }

    /// A Mage that marches with a stack (CASTER_STACKS) is no attacker: the strike takes the city
    /// with the Champions and leaves the Mage out, and a stack whose only unit left to attack is
    /// the Mage holds rather than send it.
    #[test]
    fn a_mage_in_the_stack_does_not_attack() {
        let d = ffh_data::global();
        let mut g = one_landmass_game(22);
        let (us, them) = (PlayerId(0), PlayerId(1));
        if !crate::magic::school(&g, us) || !crate::switches::CASTER_STACKS.on(&g, us) {
            return;
        }
        g.player_found(us, 4, 5);
        g.player_found(them, 12, 5);
        let city = g.players[them].capital_city.expect("the enemy founded a city");
        let tp = g.city_plot(city);
        g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 12, 5, Some(UnitAIType::UNITAI_CITY_DEFENSE), None);
        let (ut, tt) = (g.players[us].team, g.players[them].team);
        g.team_declare_war(ut, tt, true, None);
        let champions: Vec<UnitId> = (0..4).map(|_| g.player_init_unit(us, d.t::<UnitType>("UNIT_CHAMPION"), 11, 5, Some(UnitAIType::UNITAI_ATTACK), None)).collect();
        let grp = g.units[champions[0]].group.expect("a group");
        for &u in &champions[1..] {
            g.unit_join_group(u, Some(grp), false);
        }
        let mage = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 11, 5, Some(UnitAIType::UNITAI_MAGE), None);
        assert!(crate::magic::spared_caster(&g, mage));
        let from = g.units[mage].group.expect("a group");
        assert!(crate::magic::join_stack(&mut g, from, grp), "the Mage joins the stack");
        assert_eq!(g.units[mage].ai_type, Some(UnitAIType::UNITAI_WARWIZARD));
        assert_eq!(g.group_get_head_unit(grp), Some(champions[0]), "a Champion heads the stack");
        assert!(!fights(&g, mage), "the Mage does not count in the force");
        let op = Op { city, plot: tp, civ: true, phase: Phase::Strike, groups: vec![grp], rally: None, assembly: None, staging: None, struck: false, at_war: true, need: 1, have: 3 };
        let mut v = View::new(&g, us);
        strike(&mut g, &mut v, &op);
        assert!(g.units.contains(mage) && !g.units[mage].made_attack, "the Mage did not attack");
        assert!(!g.cities.contains(city) || g.cities[city].owner == us, "the Champions took the city");
        // A Warrior of theirs in the open next to the Mage and one Champion that has attacked:
        // the Mage is the only unit left to attack, and the stack holds.
        let warrior = g.player_init_unit(them, d.t::<UnitType>("UNIT_WARRIOR"), 9, 6, Some(UnitAIType::UNITAI_ATTACK), None);
        let q = g.unit_plot(warrior);
        let champion = g.player_init_unit(us, d.t::<UnitType>("UNIT_CHAMPION"), 10, 6, Some(UnitAIType::UNITAI_ATTACK), None);
        let mage2 = g.player_init_unit(us, d.t::<UnitType>("UNIT_MAGE"), 10, 6, Some(UnitAIType::UNITAI_MAGE), None);
        let stack = g.units[champion].group.expect("a group");
        let from = g.units[mage2].group.expect("a group");
        assert!(crate::magic::join_stack(&mut g, from, stack));
        g.units[champion].made_attack = true;
        g.units[champion].moves = g.unit_max_moves(champion);
        let v = View::new(&g, us);
        crate::military::assault(&mut g, &v, stack, q);
        assert!(g.units.contains(mage2) && !g.units[mage2].made_attack, "the Mage was not sent at the Warrior");
    }
}
